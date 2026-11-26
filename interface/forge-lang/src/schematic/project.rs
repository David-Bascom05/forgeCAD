//! Project-wide navigation, PCB handoff and electrical revision review.
use super::*;
use crate::{error::Op, props::quote};
use forge_agent::json::Json;

pub struct SearchHit {
    pub id: String,
    pub label: String,
    pub zone: Option<String>,
}

/// Zones divide a user-drawn frame into eight columns and four rows, read from the upper left.
pub fn zone(d: &Document, sheet: &str, at: Point) -> Option<String> {
    let symbol = &d.symbols[d.templates.get(sheet)?];
    let rect = symbol
        .graphics
        .iter()
        .filter_map(|g| match g.shape {
            Shape::Rect(a, b) => Some((a, b)),
            _ => None,
        })
        .max_by(|(a, b), (c, d)| {
            ((a[0] - b[0]) * (a[1] - b[1]))
                .abs()
                .total_cmp(&((c[0] - d[0]) * (c[1] - d[1])).abs())
        })?;
    let (a, b) = rect;
    let cell = |x: f64, l: f64, h: f64, n: f64| {
        (h > l && x >= l && x <= h).then(|| (((x - l) / (h - l) * n).floor() as usize).min(n as usize - 1))
    };
    let col = cell(at[0], a[0].min(b[0]), a[0].max(b[0]), 8.)?;
    let row = cell(at[1], a[1].min(b[1]), a[1].max(b[1]), 4.)?;
    Some(format!("{}{}", (b'D' - row as u8) as char, col + 1))
}

pub fn search(d: &Document, c: &Circuit, term: &str) -> Vec<SearchHit> {
    let words: Vec<_> = term.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    let mut add = |id: String, label: String, extra: &str, sheet: &str, at: Option<Point>| {
        let hay = format!("{id} {label} {extra}").to_lowercase();
        if words.iter().all(|w| hay.contains(w)) {
            hits.push(SearchHit {
                id,
                label,
                zone: at.and_then(|at| zone(d, sheet, at)),
            });
        }
    };
    for (scope, sheet) in &c.occurrences {
        add(scope.clone(), d.sheets[sheet].clone(), "sheet", sheet, None);
        for row in d.rows.iter().filter(|r| &r.sheet == sheet) {
            let id = format!("{scope}/{}", row.id);
            match &row.item {
                Item::Place {
                    symbol,
                    value,
                    fields,
                    reference,
                    at,
                    ..
                } => {
                    let sym = &d.symbols[symbol];
                    let extra = fields
                        .iter()
                        .chain(&sym.fields)
                        .map(|(k, v)| format!("{k} {v}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let display = d.reference(&id, reference.as_ref().unwrap_or(&row.id));
                    add(
                        id,
                        format!("{display} · {value} · {symbol}"),
                        &extra,
                        sheet,
                        Some(*at),
                    );
                }
                Item::Label { text, at, .. } | Item::Note { text, at, .. } => {
                    add(id, text.clone(), "", sheet, Some(*at))
                }
                Item::Port(at) => add(id, row.id.clone(), "port", sheet, Some(*at)),
                _ => {}
            }
        }
    }
    for (id, p) in &c.terminals {
        let scope = id.rsplit_once('/').unwrap().0;
        add(
            id.clone(),
            format!("{} · pin {}", p.pin, p.number),
            &d.symbols[&p.symbol].pins[&p.pin].description,
            &c.occurrences[scope],
            Some(p.at),
        );
    }
    hits
}

/// Suggestions bind existing terminals. Only an explicit diff_pair changes board intent.
pub fn pair_suggestions(d: &Document, sheet: &str) -> Vec<(String, String, String, String)> {
    let labels: Map<_, _> = d
        .rows
        .iter()
        .filter(|r| r.sheet == sheet)
        .filter_map(|r| match &r.item {
            Item::Label { text, target, .. } => Some((text, target)),
            _ => None,
        })
        .collect();
    let mut pairs = Vec::new();
    for (positive, pin) in &labels {
        for (suffix, mate) in [
            ("+", "-"),
            ("+", "−"),
            ("P", "N"),
            ("DP", "DM"),
            ("p", "n"),
            ("dp", "dm"),
        ] {
            if let Some(stem) = positive.strip_suffix(suffix) {
                let negative = format!("{stem}{mate}");
                if let Some(other) = labels.get(&negative) {
                    pairs.push(((*positive).clone(), negative, (*pin).clone(), (*other).clone()));
                }
            }
        }
    }
    pairs
}

pub(super) fn diff_pairs(c: &Circuit) -> Json {
    Json::Arr(
        c.diff_pairs
            .iter()
            .map(|(id, [p, n])| {
                Json::obj(vec![
                    ("id", id.clone().into()),
                    ("positive", p.clone().into()),
                    ("negative", n.clone().into()),
                    ("positive_net", c.net_of[p].clone().into()),
                    ("negative_net", c.net_of[n].clone().into()),
                ])
            })
            .collect(),
    )
}

pub fn board(d: &Document, c: &Circuit, variant: Option<&str>) -> Result<Json> {
    let overrides = variant
        .map(|v| {
            d.variants
                .get(v)
                .ok_or_else(|| Fault::new("UnknownVariant", Span::default(), v))
        })
        .transpose()?;
    let mut parts = Vec::new();
    for (scope, sheet) in &c.occurrences {
        for r in d.rows.iter().filter(|r| &r.sheet == sheet) {
            let Item::Place {
                symbol,
                reference,
                footprint,
                fields,
                datasheet,
                value,
                populated,
                ..
            } = &r.item
            else {
                continue;
            };
            let sym = &d.symbols[symbol];
            if sym.power_net.is_some() {
                continue;
            }
            let id = format!("{scope}/{}", r.id);
            let mut pads = Vec::new();
            for (pin, p) in &sym.pins {
                let terminal = format!("{id}.{pin}");
                for pad in &p.pads {
                    pads.push(Json::obj(vec![
                        ("pad", pad.clone().into()),
                        ("terminal", terminal.clone().into()),
                        (
                            "net",
                            c.net_of.get(&terminal).cloned().map_or(Json::Null, Into::into),
                        ),
                    ]));
                }
            }
            let mut metadata = sym.fields.clone();
            metadata.extend(fields.clone());
            parts.push(Json::obj(vec![
                ("fields", super::bom::field_json(&metadata)),
                (
                    "datasheet",
                    datasheet
                        .as_ref()
                        .or(sym.datasheet.as_ref())
                        .cloned()
                        .map_or(Json::Null, Into::into),
                ),
                ("id", id.clone().into()),
                (
                    "reference",
                    d.reference(&id, reference.as_ref().unwrap_or(&r.id)).into(),
                ),
                ("symbol", symbol.clone().into()),
                ("value", value.clone().into()),
                ("footprint", footprint.clone().map_or(Json::Null, Into::into)),
                (
                    "populated",
                    (*overrides.and_then(|v| v.get(&id)).unwrap_or(populated)).into(),
                ),
                ("pads", Json::Arr(pads)),
            ]));
        }
    }
    let links = c
        .links
        .iter()
        .map(|l| {
            Json::obj(vec![
                ("consumer", l.want.clone().into()),
                ("provider", l.give.clone().into()),
                ("role", l.role.0.clone().into()),
                (
                    "prefer_mm",
                    l.prefer
                        .or(d.roles[&l.role].prefer)
                        .map_or(Json::Null, Into::into),
                ),
            ])
        })
        .collect();
    Ok(Json::obj(vec![
        ("schema", "forgecad.pcb-handoff/1".into()),
        ("diff_pairs", diff_pairs(c)),
        ("components", Json::Arr(parts)),
        ("relationships", Json::Arr(links)),
    ]))
}

/// Compare stable occurrence identities, pad nets, role bindings, fitted parts and active rules.
pub fn electrical_diff(a: (&Document, &Circuit), b: (&Document, &Circuit)) -> Result<Json> {
    fn records((d, c): (&Document, &Circuit)) -> Result<Map<String, Json>> {
        let mut map = Map::new();
        if d.library {
            for s in d.symbols.keys() {
                let r = response(d, c, &["symbol".into(), s.clone()])?;
                map.insert(format!("symbol/{s}"), r.get("result").unwrap().clone());
            }
        } else {
            if let Some(Json::Arr(parts)) = board(d, c, None)?.get("components") {
                for part in parts {
                    map.insert(
                        format!("component/{}", part.get("id").unwrap().str().unwrap()),
                        part.clone(),
                    );
                }
            }
            for (id, t) in &c.terminals {
                map.insert(
                    format!("pin/{id}"),
                    Json::obj(vec![
                        ("electrical", t.electrical.word().into()),
                        ("function", t.function.clone().map_or(Json::Null, Into::into)),
                        (
                            "wants",
                            Json::Arr(t.wants.iter().map(|r| r.0.clone().into()).collect()),
                        ),
                        (
                            "gives",
                            Json::Arr(t.gives.iter().map(|r| r.0.clone().into()).collect()),
                        ),
                    ]),
                );
            }
        }
        for (name, changes) in &d.variants {
            map.insert(
                format!("variant/{name}"),
                Json::Obj(
                    changes
                        .iter()
                        .map(|(id, fit)| (id.clone(), (*fit).into()))
                        .collect(),
                ),
            );
        }
        for (rule, level) in &d.severities {
            map.insert(format!("policy/{rule}"), level.clone().into());
        }
        for ((rule, objects), reason) in &d.waivers {
            map.insert(
                format!("waiver/{rule}/{}", objects.join(",")),
                reason.clone().into(),
            );
        }
        for key in ["links", "erc", "diff-pairs"] {
            let r = response(d, c, &[key.into()])?;
            if let Some(Json::Arr(entries)) = r.get("result") {
                for item in entries {
                    let mut item = item.clone();
                    if let Json::Obj(values) = &mut item {
                        values.retain(|(k, _)| k != "locations");
                    }
                    let id = item.get("id").map(ToString::to_string).unwrap_or_else(|| {
                        format!("{} {}", item.get("kind").unwrap(), item.get("objects").unwrap())
                    });
                    map.insert(format!("{key}/{id}"), item);
                }
            }
        }
        if let Some(Json::Arr(roles)) = response(d, c, &[])?.get("result").and_then(|r| r.get("roles")) {
            for role in roles {
                map.insert(
                    format!("role/{}", role.get("id").and_then(Json::str).unwrap_or_default()),
                    role.clone(),
                );
            }
        }
        Ok(map)
    }
    let old = records(a)?;
    let new = records(b)?;
    Ok(Json::Arr(
        old.keys()
            .chain(new.keys())
            .collect::<Set<_>>()
            .into_iter()
            .filter(|k| old.get(*k) != new.get(*k))
            .map(|k| {
                Json::obj(vec![
                    ("id", k.clone().into()),
                    ("before", old.get(k).cloned().unwrap_or(Json::Null)),
                    ("after", new.get(k).cloned().unwrap_or(Json::Null)),
                ])
            })
            .collect(),
    ))
}

pub fn reannotate(d: &Document, sheet: &str) -> Vec<Op> {
    let mut counts = Map::new();
    d.rows
        .iter()
        .filter(|r| r.sheet == sheet)
        .filter_map(|r| {
            let Item::Place {
                symbol, reference, ..
            } = &r.item
            else {
                return None;
            };
            let def = &d.symbols[symbol];
            if def.power_net.is_some() {
                return None;
            }
            let count = counts.entry(&def.ref_prefix).or_insert(0);
            *count += 1;
            let next = format!("{}{count}", def.ref_prefix);
            (reference.as_ref().unwrap_or(&r.id) != &next).then(|| Op::SetArg {
                id: r.id.clone(),
                arg: "reference".into(),
                text: quote(&next),
            })
        })
        .collect()
}

/// Assign globally unique references in occurrence and then source order, reserving all locked names.
pub fn project_annotation(src: &str, d: &Document, c: &Circuit) -> Result<Vec<Op>> {
    let mut reserved = Set::new();
    for a in d.annotations.values().filter(|a| a.locked) {
        if !reserved.insert(a.reference.clone()) {
            return Err(Fault::new(
                "DuplicateLockedReference",
                Span::default(),
                &a.reference,
            ));
        }
    }
    let tree = crate::syntax::parse(src)
        .map_err(|e| Fault::new(e.kind(), e.span().unwrap_or_default(), e.text()))?;
    let mut names: Set<String> =
        tree.into_iter()
            .filter_map(|s| match s.kind {
                crate::syntax::StmtKind::Assign { name, .. }
                | crate::syntax::StmtKind::Shared { name, .. } => Some(name),
                _ => None,
            })
            .collect();
    let mut counts = Map::new();
    let mut serial = 0;
    let mut ops = Vec::new();
    for (scope, sheet) in &c.occurrences {
        for r in d.rows.iter().filter(|r| &r.sheet == sheet) {
            let Item::Place {
                symbol, reference, ..
            } = &r.item
            else {
                continue;
            };
            let sym = &d.symbols[symbol];
            if sym.power_net.is_some() {
                continue;
            }
            let path = format!("{scope}/{}", r.id);
            let annotation = d.annotations.get(&path);
            if annotation.is_some_and(|a| a.locked) {
                continue;
            }
            let count = counts.entry(&sym.ref_prefix).or_insert(0);
            let next = loop {
                *count += 1;
                let s = format!("{}{count}", sym.ref_prefix);
                if reserved.insert(s.clone()) {
                    break s;
                }
            };
            if d.reference(&path, reference.as_ref().unwrap_or(&r.id)) == next {
                continue;
            }
            if let Some(a) = annotation {
                ops.push(Op::SetArg {
                    id: a.id.clone(),
                    arg: "reference".into(),
                    text: quote(&next),
                });
            } else {
                let id = loop {
                    serial += 1;
                    let s = format!("refdes{serial}");
                    if names.insert(s.clone()) {
                        break s;
                    }
                };
                ops.push(Op::AddFeature {
                    before: None,
                    text: format!(
                        "{id} = annotation(occurrence: {}, reference: {})",
                        quote(&path),
                        quote(&next)
                    ),
                });
            }
        }
    }
    Ok(ops)
}

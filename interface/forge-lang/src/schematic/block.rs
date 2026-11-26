//! An atomic edit of a selection within a sheet definition.
use super::*;
use crate::{
    error::Op,
    props::quote,
    syntax::{self, Expr, ExprKind, StmtKind},
};
pub struct BlockPlan {
    pub ops: Vec<Op>,
    pub boundary: Set<String>,
}

fn dependencies(item: &Item) -> Vec<&str> {
    match item {
        Item::Wire { from, to, .. } | Item::Flow { from, to } => vec![from, to],
        Item::Gate { of, .. } => vec![of],
        Item::Label { target, .. } | Item::NoConnect(target) => vec![target],
        Item::Bus(m) => m.values().map(String::as_str).collect(),
        Item::Fulfill {
            want, give, returns, ..
        } => {
            let mut refs = vec![want.as_str(), give.as_str()];
            if let Some((a, b)) = returns {
                refs.extend([a.as_str(), b.as_str()]);
            }
            refs
        }
        _ => vec![],
    }
}
fn owner(endpoint: &str) -> &str {
    endpoint.split('.').next().unwrap_or(endpoint)
}

pub fn block_plan(
    src: &str,
    d: &Document,
    c: &Circuit,
    scope: &str,
    selected: &Set<String>,
    delta: Point,
    copy: bool,
) -> Result<BlockPlan> {
    let fail = |msg: &str| Fault::new("BlockSelection", Span::default(), msg);
    let sheet = c
        .occurrences
        .get(scope)
        .ok_or_else(|| fail("choose a sheet occurrence"))?;
    if selected.is_empty() || delta.iter().any(|v| !v.is_finite()) {
        return Err(fail("select objects and enter a finite offset"));
    }
    let rows: Vec<_> = d.rows.iter().filter(|r| &r.sheet == sheet).collect();
    if selected.iter().any(|s| !rows.iter().any(|r| &r.id == s)) {
        return Err(fail("selection contains an object outside this sheet"));
    }
    let mut include = selected.clone();
    let mut boundary = Set::new();
    // All electrical references point to placements, ports, junctions or sheet instances.
    for r in &rows {
        let deps = dependencies(&r.item);
        let count = deps.iter().filter(|k| selected.contains(owner(k))).count();
        if !deps.is_empty() && count == deps.len() {
            include.insert(r.id.clone());
        } else if count > 0 || !deps.is_empty() && selected.contains(&r.id) {
            boundary.insert(format!("{} · {}", r.id, deps.join(", ")));
            if copy {
                include.remove(&r.id);
            }
        }
    }
    if include.is_empty() {
        return Err(fail("include the endpoints of the connection to copy it"));
    }
    let tree = syntax::parse(src).map_err(|e| fail(&e.text()))?;
    let mut used: Set<_> = tree
        .iter()
        .filter_map(|s| match &s.kind {
            StmtKind::Assign { name, .. } | StmtKind::Shared { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    let mut rename = Map::new();
    if copy {
        for id in &include {
            let prefix = id.trim_end_matches(|c: char| c.is_ascii_digit());
            let name = (1..)
                .map(|i| format!("{prefix}{i}"))
                .find(|name| used.insert(name.clone()))
                .unwrap();
            rename.insert(id.clone(), name);
        }
    }
    let raw = |e: &Expr| &src[e.span.lo as usize..e.span.hi as usize];
    let moved = |e: &Expr| -> Result<String> {
        let ExprKind::Tuple(xy) = &e.kind else {
            return Err(fail("expected a coordinate pair"));
        };
        let coordinate = |i: usize| {
            if delta[i] == 0. {
                raw(&xy[i]).to_owned()
            } else {
                format!("({}) + ({}mm)", raw(&xy[i]), delta[i])
            }
        };
        Ok(format!("({}, {})", coordinate(0), coordinate(1)))
    };
    let renamed = |v: &str| {
        let (part, tail) = v.split_once('.').map_or((v, None), |(p, s)| (p, Some(s)));
        let next = rename.get(part).map(String::as_str).unwrap_or(part);
        tail.map_or_else(|| next.to_owned(), |s| format!("{next}.{s}"))
    };
    let mut ops = Vec::new();
    for stmt in &tree {
        let StmtKind::Assign { name, value } = &stmt.kind else {
            continue;
        };
        if !include.contains(name) {
            continue;
        }
        let row = rows.iter().find(|r| &r.id == name).unwrap();
        let ExprKind::Call(head, args) = &value.kind else {
            unreachable!()
        };
        let mut encoded = Vec::new();
        for arg in args {
            let key = &arg.name.as_ref().unwrap().0;
            let expr = &arg.value;
            let next = match (key.as_str(), &row.item) {
                ("at", _) | ("from" | "to" | "mid", Item::Drawing { .. }) => Some(moved(expr)?),
                ("via", Item::Wire { .. }) => {
                    let ExprKind::List(points) = &expr.kind else {
                        unreachable!()
                    };
                    Some(format!(
                        "[{}]",
                        points.iter().map(&moved).collect::<Result<Vec<_>>>()?.join(", ")
                    ))
                }
                ("from" | "to" | "on" | "want" | "give" | "return_from" | "return_to", _) if copy => {
                    Some(renamed(&raw(expr).split_whitespace().collect::<String>()))
                }
                ("of", Item::Gate { .. }) if copy => Some(renamed(raw(expr))),
                ("members", Item::Bus(m)) if copy => Some(format!(
                    "{{{}}}",
                    m.iter()
                        .map(|(k, v)| format!("{k}: {}", renamed(v)))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                ("reference" | "bindings", Item::Place { .. }) if copy => continue,
                _ => None,
            };
            if copy {
                encoded.push(format!("{key}: {}", next.as_deref().unwrap_or_else(|| raw(expr))));
            } else if let Some(text) = next {
                ops.push(Op::SetArg {
                    id: name.clone(),
                    arg: key.clone(),
                    text,
                });
            }
        }
        if copy {
            if let Item::Place { symbol, bindings, .. } = &row.item {
                let mut targets = bindings.clone();
                for (source, checks) in c.connections.range(format!("{scope}/{name}.")..) {
                    if !source.starts_with(&format!("{scope}/{name}.")) {
                        break;
                    }
                    for role in checks.keys() {
                        let want = d.roles[role].connection.as_ref().unwrap();
                        if let Some((original, _)) = want.to.split_once('.') {
                            let target = bindings.get(original).map(String::as_str).unwrap_or(original);
                            if let Some(new) = rename.get(target) {
                                targets.insert(original.to_owned(), new.clone());
                            } else {
                                boundary.insert(format!("{name} · want {} uses {target}", role.0));
                            }
                        }
                    }
                }
                if !targets.is_empty() {
                    encoded.push(format!(
                        "bindings: {{{}}}",
                        targets
                            .iter()
                            .map(|(a, b)| format!("{a}: {b}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                if d.symbols[symbol].power_net.is_some() {
                    boundary.insert(format!("{name} · shared power net"));
                }
            }
            if let Item::Label { text, .. } = &row.item {
                boundary.insert(format!("{name} · shared net label {}", quote(text)));
            }
            ops.push(Op::AddFeature {
                before: None,
                text: format!("{} = {}({})", rename[name], raw(head), encoded.join(", ")),
            });
        }
    }
    Ok(BlockPlan { ops, boundary })
}

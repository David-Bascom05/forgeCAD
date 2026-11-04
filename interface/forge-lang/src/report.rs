//! A run as JSON (the typed data) and as text (rendered from it).

use crate::eval::{ParamInfo, Status, Value};
use crate::values::{Dim, Src, fmt};
use crate::{Run, error::ParamKey};
use forge_agent::json::Json;
use std::collections::BTreeMap;

fn src_json(s: &Src) -> Json {
    match s {
        Src::Const => Json::obj(vec![("const", true.into())]),
        Src::Driving(p) => Json::obj(vec![("driving", p.to_string().into())]),
        Src::Driven(ps) => Json::obj(vec![(
            "driven",
            Json::Arr(ps.iter().map(|p| Json::Str(p.to_string())).collect()),
        )]),
    }
}

fn shown(v: f64, d: Dim) -> f64 {
    if d == Dim::Angle { v.to_degrees() } else { v }
}

fn param_value(p: &ParamInfo) -> Json {
    match &p.value {
        Value::Num(n) => Json::obj(vec![
            ("value", shown(n.v, n.dim).into()),
            ("dim", n.dim.name().into()),
        ]),
        Value::List(xs) => Json::Arr(
            xs.iter()
                .map(|x| match x {
                    Value::Num(n) => Json::Num(shown(n.v, n.dim)),
                    _ => Json::Null,
                })
                .collect(),
        ),
        _ => Json::Null,
    }
}

/// For each param, the features whose dimensions it drives or feeds.
pub fn uses(run: &Run) -> BTreeMap<ParamKey, Vec<String>> {
    let mut out: BTreeMap<ParamKey, Vec<String>> = BTreeMap::new();
    for f in &run.feats {
        for d in &f.dims {
            for p in d.num.src.params() {
                let v = out.entry(p).or_default();
                if !v.contains(&f.id) {
                    v.push(f.id.clone());
                }
            }
        }
    }
    out
}

pub fn json(run: &Run, src: &str) -> Json {
    let line = |lo: u32| src[..(lo as usize).min(src.len())].matches('\n').count() + 1;
    let uses = uses(run);
    let feats = run
        .feats
        .iter()
        .map(|f| {
            Json::obj(vec![
                ("id", f.id.as_str().into()),
                ("kind", f.kind.into()),
                ("line", line(f.span.lo).into()),
                (
                    "status",
                    match f.status {
                        Status::Ok => "ok",
                        Status::Failed => "error",
                        Status::NotBuilt => "not built",
                    }
                    .into(),
                ),
                (
                    "dims",
                    Json::Arr(
                        f.dims
                            .iter()
                            .map(|d| {
                                Json::obj(vec![
                                    ("what", d.what.into()),
                                    ("value", shown(d.num.v, d.num.dim).into()),
                                    ("dim", d.num.dim.name().into()),
                                    ("from", src_json(&d.num.src)),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ])
        })
        .collect();
    let params = run
        .shared
        .iter()
        .chain(&run.locals)
        .map(|p| {
            Json::obj(vec![
                ("param", p.key.to_string().into()),
                ("shared", p.key.is_shared().into()),
                ("structural", run.observed.contains(&p.key).into()),
                ("line", line(p.span.lo).into()),
                ("value", param_value(p)),
                (
                    "links",
                    Json::Arr(p.links.iter().map(|l| Json::Str(l.to_string())).collect()),
                ),
                (
                    "used_by",
                    Json::Arr(
                        uses.get(&p.key)
                            .into_iter()
                            .flatten()
                            .map(|s| Json::Str(s.clone()))
                            .collect(),
                    ),
                ),
            ])
        })
        .collect();
    let t = &run.timings;
    let mut f = vec![
        ("ok", run.ok().into()),
        (
            "timings_ms",
            Json::obj(vec![
                ("parse", t.parse_ms.into()),
                ("check", t.check_ms.into()),
                ("eval_and_build", t.eval_ms.into()),
                ("kernel", t.kernel_ms.into()),
                ("measure", t.measure_ms.into()),
                ("total", t.total_ms.into()),
            ]),
        ),
        ("rebuilt", run.rebuilt.into()),
        ("reused", run.reused.into()),
        ("remade", run.remade.into()),
        ("features", Json::Arr(feats)),
        ("params", Json::Arr(params)),
        (
            "errors",
            Json::Arr(run.errors.iter().map(|e| e.json(src)).collect()),
        ),
        (
            "warnings",
            Json::Arr(run.warnings.iter().map(|w| w.json(src)).collect()),
        ),
        (
            "expects",
            Json::Arr(
                run.expects
                    .iter()
                    .map(|e| {
                        Json::obj(vec![
                            ("metric", e.metric.name().into()),
                            ("measured", e.measured.into()),
                            ("ok", e.ok.into()),
                        ])
                    })
                    .collect(),
            ),
        ),
    ];
    if !run.sheets.is_empty() {
        f.push(("sheets", Json::Arr(run.sheets.iter().map(|s| s.json()).collect())));
    }
    if run.props.span.is_some() {
        f.push(("properties", run.props.json()));
    }
    if let Some(p) = &run.physical {
        f.push(("physical", p.json()));
    }
    if !run.thermos.is_empty() {
        f.push((
            "thermoforms",
            Json::Arr(run.thermos.iter().map(|s| s.json()).collect()),
        ));
    }
    if let Some(a) = &run.asm {
        f.push(("assembly", crate::asm::json(a)));
        f.push(("bom_parts", crate::props::bom_json(a)));
        if run.errors.is_empty() {
            let p = crate::props::asm_physical(a, None);
            f.push(("physical", p.map_or_else(Json::Str, |p| p.json())));
        }
    }
    if let Some(m) = &run.measure {
        f.push((
            "part",
            Json::obj(vec![
                ("volume", m.volume.into()),
                ("area", m.area.into()),
                (
                    "centroid",
                    Json::Arr(m.centroid.iter().map(|x| Json::Num(*x)).collect()),
                ),
                ("faces", m.faces.into()),
                ("edges", m.edges.into()),
                ("bodies", m.bodies.into()),
                ("valid", (m.problems == 0).into()),
                (
                    "problems",
                    Json::Arr(
                        m.invalid
                            .iter()
                            .map(|p| {
                                Json::obj(vec![
                                    ("kind", p.kind.into()),
                                    ("body", p.body.as_str().into()),
                                    ("reason", p.reason.as_str().into()),
                                    ("at", Json::Arr(p.at.iter().map(|a| a.as_str().into()).collect())),
                                ])
                            })
                            .collect(),
                    ),
                ),
                ("bbox", Json::Arr(m.bbox.iter().map(|x| Json::Num(*x)).collect())),
            ]),
        ));
    }
    Json::obj(f)
}

pub fn text(run: &Run, src: &str) -> String {
    let mut out = String::new();
    let line = |lo: u32| src[..(lo as usize).min(src.len())].matches('\n').count() + 1;
    for f in &run.feats {
        let st = match f.status {
            Status::Ok => "ok",
            Status::Failed => "ERROR",
            Status::NotBuilt => "not built",
        };
        let dims: Vec<String> = f
            .dims
            .iter()
            .map(|d| {
                let from = match &d.num.src {
                    Src::Const => String::new(),
                    Src::Driving(p) => format!("={p}"),
                    Src::Driven(ps) => {
                        format!(
                            "<-{}",
                            ps.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",")
                        )
                    }
                };
                format!("{} {}{from}", d.what, fmt(shown(d.num.v, d.num.dim)))
            })
            .collect();
        out.push_str(&format!(
            "{st:9} {:24} {:8} line {:<4} {}\n",
            f.id,
            f.kind,
            line(f.span.lo),
            dims.join("  ")
        ));
    }
    for e in &run.errors {
        let (l, c) = e.span().map_or((0, 0), |s| {
            let (l, c, ..) = s.lines(src);
            (l, c)
        });
        out.push_str(&format!("error {}:{} {}: {}\n", l, c, e.kind(), e.text()));
        for fx in e.fixes() {
            let mut s = String::new();
            fx.json().write(&mut s);
            out.push_str(&format!("  fix: {s}\n"));
        }
    }
    for w in &run.warnings {
        let mut s = String::new();
        w.json(src).write(&mut s);
        out.push_str(&format!("warning {s}\n"));
    }
    for e in &run.expects {
        out.push_str(&format!(
            "expect {} {} ({})\n",
            e.metric.name(),
            if e.ok { "ok" } else { "FAILED" },
            fmt(e.measured)
        ));
    }
    if let Some(m) = &run.measure {
        out.push_str(&format!(
            "part: volume {} mm3, area {} mm2, centroid {} {} {}, bodies {}, faces {}, edges {}, {}\n",
            fmt((m.volume * 1000.0).round() / 1000.0),
            fmt((m.area * 1000.0).round() / 1000.0),
            fmt((m.centroid[0] * 1000.0).round() / 1000.0),
            fmt((m.centroid[1] * 1000.0).round() / 1000.0),
            fmt((m.centroid[2] * 1000.0).round() / 1000.0),
            m.bodies,
            m.faces,
            m.edges,
            if m.problems == 0 { "valid" } else { "INVALID" }
        ));
        for p in &m.invalid {
            out.push_str(&format!("invalid {} {}: {}", p.body, p.kind, p.reason));
            if !p.at.is_empty() {
                out.push_str(&format!(" (at {})", p.at.join("; ")));
            }
            out.push('\n');
        }
    }
    if let Some(a) = &run.asm {
        let t = &a.timings;
        out.push_str(&format!(
            "assembly: {} instances, mobility {}, {} interferences, {} intended contacts; load {:.1} ms, anchors {:.2} ms, solve {:.2} ms{}, overlap {:.1} ms\n",
            a.poses.len(),
            a.mobility,
            a.interferences,
            a.intended,
            t.load_ms,
            t.anchors_ms,
            t.solve_ms,
            if t.incremental { " (incremental)" } else { "" },
            t.overlap_ms
        ));
        for j in &a.joints {
            let vals: Vec<String> = j
                .values
                .iter()
                .map(|(w, v, ang)| format!("{w} {}", fmt(if *ang { v.to_degrees() } else { *v })))
                .collect();
            out.push_str(&format!(
                "joint {:24} {:12} free {} {}\n",
                j.id,
                j.kind,
                j.free,
                vals.join("  ")
            ));
        }
        for e in &a.engagements {
            out.push_str(&format!(
                "engagement {} in {} ({}): {}{}\n",
                e.a,
                e.b,
                e.rule,
                fmt(e.engaged),
                match (e.min, e.short) {
                    (Some(m), true) => format!(", SHORT of {}", fmt(m)),
                    (Some(m), false) => format!(", min {}", fmt(m)),
                    (None, _) => String::new(),
                }
            ));
        }
        for e in &a.expects {
            out.push_str(&format!(
                "expect {} line {} {} ({})\n",
                e.metric,
                e.line,
                if e.ok { "ok" } else { "FAILED" },
                e.measured.map_or_else(|| "not measured".into(), fmt)
            ));
        }
    }
    let t = &run.timings;
    out.push_str(&format!(
        "{} in {:.1} ms (parse {:.2}, check {:.2}, eval+build {:.1} of which kernel {:.1}, measure {:.1}); rebuilt {}, reused {}\n",
        if run.ok() { "ok" } else { "FAILED" },
        t.total_ms,
        t.parse_ms,
        t.check_ms,
        t.eval_ms,
        t.kernel_ms,
        t.measure_ms,
        run.rebuilt,
        run.reused
    ));
    out
}

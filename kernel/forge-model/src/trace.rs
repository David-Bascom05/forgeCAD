//! Debug output after every feature of a sequential regen, off unless asked for:
//! FORGE_CHECK prints each feature whose result fails `forge_topo::check`;
//! FORGE_SVG=dir writes dir/NNN.svg per feature: the result's edges in black, the boolean's tool
//! in blue, edges named by a check problem in red (an isometric view).
use forge_geom::CurveGeom;
use forge_topo::{Body, EdgeId, Problem};
use std::fmt::Write;

struct Env {
    check: bool,
    svg: Option<String>,
}

fn env() -> &'static Env {
    static ON: std::sync::OnceLock<Env> = std::sync::OnceLock::new();
    ON.get_or_init(|| Env {
        check: std::env::var_os("FORGE_CHECK").is_some(),
        svg: std::env::var("FORGE_SVG").ok(),
    })
}

pub(crate) fn on() -> bool {
    let e = env();
    e.check || e.svg.is_some()
}

/// Feature `i` produced `body`, combining `tool` into it if a boolean.
pub(crate) fn step(i: usize, body: &Body, tool: Option<&Body>) {
    let body = body.compact();
    let problems = forge_topo::check(&body);
    if env().check && !problems.is_empty() {
        eprintln!(
            "FORGE_CHECK feature {i}: {} problems, first {:?}",
            problems.len(),
            &problems[..problems.len().min(3)]
        );
    }
    if let Some(dir) = &env().svg {
        let bad: Vec<EdgeId> = problems.iter().filter_map(problem_edge).collect();
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(format!("{dir}/{i:03}.svg"), svg(&body, tool, &bad));
    }
}

fn problem_edge(p: &Problem) -> Option<EdgeId> {
    match *p {
        Problem::EdgeUses { edge, .. }
        | Problem::EdgeUsesSameWay { edge }
        | Problem::VertexOffEdge { edge, .. }
        | Problem::EdgeOffFace { edge, .. }
        | Problem::EdgeDegenerate { edge } => Some(edge),
        _ => None,
    }
}

fn polylines(b: &Body) -> Vec<(EdgeId, Vec<[f64; 2]>)> {
    b.edge_ids()
        .map(|e| {
            let ed = b.edge(e);
            let n = 32;
            let pts = (0..=n)
                .map(|k| {
                    let p = ed
                        .curve
                        .eval(ed.range.lo + ed.range.width() * k as f64 / n as f64);
                    [(p.x - p.y) * 0.866_025_403_784_438_6, (p.x + p.y) * 0.5 - p.z]
                })
                .collect();
            (e, pts)
        })
        .collect()
}

fn svg(body: &Body, tool: Option<&Body>, bad: &[EdgeId]) -> String {
    let mine = polylines(body);
    let theirs = tool.map(polylines).unwrap_or_default();
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for p in mine.iter().chain(&theirs).flat_map(|(_, l)| l) {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let size = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-9);
    let pad = size * 0.05;
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"800\" height=\"800\">\n",
        lo[0] - pad,
        lo[1] - pad,
        size + 2.0 * pad,
        size + 2.0 * pad
    );
    let w = size / 400.0;
    let mut path = |l: &[[f64; 2]], style: &str| {
        let d: Vec<String> = l.iter().map(|p| format!("{:.5},{:.5}", p[0], p[1])).collect();
        let _ = writeln!(s, "<polyline fill=\"none\" {style} points=\"{}\"/>", d.join(" "));
    };
    for (_, l) in &theirs {
        path(
            l,
            &format!(
                "stroke=\"#36c\" stroke-width=\"{w}\" stroke-dasharray=\"{} {}\"",
                4.0 * w,
                3.0 * w
            ),
        );
    }
    for (e, l) in &mine {
        if bad.contains(e) {
            path(l, &format!("stroke=\"#d00\" stroke-width=\"{}\"", 3.0 * w));
        } else {
            path(l, &format!("stroke=\"#000\" stroke-width=\"{w}\""));
        }
    }
    s.push_str("</svg>\n");
    s
}

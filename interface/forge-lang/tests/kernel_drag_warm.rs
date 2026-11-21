//! What a drag remembers changes nothing. A Forge part after any sequence of values and edits,
//! run in one session (its stages kept), has, to the bit, what the part built alone has:
//! vertices, curves, surfaces, names, which faces and in what order. research/drag-speed.
//! DRAG_WARM_ALL=1 takes every number of every part, not the first few: a run of hours, by hand.
#![cfg(not(target_family = "wasm"))]
mod common;

use forge_geom::{Curve, CurveGeom, Surface};
use forge_lang::build::Cache;
use forge_lang::{Options, Run};
use forge_math::Point2;
use forge_math::hash::BitHasher;
use forge_topo::Body;
use std::panic::{AssertUnwindSafe, catch_unwind};

const BENCH: [&str; 15] = [
    "bracket", "clamp", "conrod", "crank", "frame", "gearbox", "guard", "heatsink", "hub", "insert", "jack",
    "manifold", "quadarm", "valve", "volute",
];

fn bench(name: &str) -> String {
    let path = format!("{}/parts/bench/{name}.fg", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).expect("part file")
}

fn surface(h: &mut BitHasher, s: &Surface) {
    match s {
        Surface::Procedural(p) => {
            let d = p.domain();
            let span = |i: forge_math::Interval| {
                let (lo, hi) = (i.lo.max(-1e3), i.hi.min(1e3));
                (0..=6).map(move |k| lo + (hi - lo) * k as f64 / 6.0)
            };
            for u in span(d.u.interval()) {
                for v in span(d.v.interval()) {
                    let q = p.deriv(Point2::new(u, v));
                    h.p3(q.p);
                    h.v3(q.du);
                    h.v3(q.dv);
                }
            }
        }
        // every field, every bit
        _ => match s.bits() {
            forge_geom::surface::Bits::Plane(w) => w.iter().for_each(|&x| h.u64(x)),
            forge_geom::surface::Bits::Cylinder(w) | forge_geom::surface::Bits::Sphere(w) => {
                w.iter().for_each(|&x| h.u64(x))
            }
            forge_geom::surface::Bits::Cone(w) => w.iter().for_each(|&x| h.u64(x)),
            forge_geom::surface::Bits::Torus(w) => w.iter().for_each(|&x| h.u64(x)),
            forge_geom::surface::Bits::Procedural(_) => unreachable!(),
        },
    }
}

fn curve(h: &mut BitHasher, c: &Curve, range: forge_math::Interval) {
    if let Curve::Intersection(ic) = c {
        ic.pts.iter().for_each(|&p| h.p3(p));
        ic.tans.iter().for_each(|&v| h.v3(v));
        h.f64(ic.sag);
        surface(h, &ic.a);
        surface(h, &ic.b);
    }
    for k in 0..=8 {
        let d = c.deriv(range.lo + range.width() * k as f64 / 8.0);
        h.p3(d.p);
        h.v3(d.d1);
    }
}

/// All of a body, in order.
fn digest(b: &Body) -> u64 {
    let b = b.compact();
    let mut h = BitHasher::new();
    for v in &b.vertices {
        h.p3(v.point);
        h.f64(v.tol);
        h.u64(v.name.0);
    }
    for e in &b.edges {
        h.u64(e.name.0);
        h.u64(e.start.0 as u64);
        h.u64(e.end.0 as u64);
        h.f64(e.range.lo);
        h.f64(e.range.hi);
        curve(&mut h, &e.curve, e.range);
    }
    for c in &b.coedges {
        h.u64(c.edge.0 as u64);
        h.u64(c.forward as u64);
    }
    for f in b.face_ids() {
        let face = b.face(f);
        h.u64(face.name.0);
        h.u64(face.same_sense as u64);
        h.u64(face.shell.0 as u64);
        for &l in &face.loops {
            h.u64(b.lp(l).coedges().len() as u64);
            b.lp(l).coedges().for_each(|c| h.u64(c.0 as u64));
        }
        surface(&mut h, &face.surface);
    }
    h.finish()
}

/// What a run holds: its errors, or all of its part.
fn outcome(r: &Run) -> String {
    match (&r.errors[..], &r.body) {
        ([], Some(b)) => format!("{} features {:x}", r.feats.len(), digest(b)),
        ([], None) => "nothing".to_string(),
        (es, _) => format!("{} features, errors {es:?}", r.feats.len()),
    }
}

/// `src` run in the session `cache` holds, with the bench font beside it.
fn run(src: &str, cache: Cache) -> (Run, Cache) {
    // the scratch folder common::run writes the font to
    let dir = std::env::temp_dir().join(format!("forge-lang-tests-{}", std::process::id()));
    if !dir.join(forge_agent::bench::FONT).exists() {
        common::run("");
    }
    let opts = Options {
        dir: Some(dir),
        ..Options::default()
    };
    forge_lang::run(src, opts, cache)
}

/// `src` built alone, on a thread with nothing remembered.
fn cold(src: &str) -> String {
    let src = src.to_string();
    std::thread::spawn(
        move || match catch_unwind(AssertUnwindSafe(|| run(&src, Cache::default()))) {
            Ok((r, _)) => outcome(&r),
            Err(_) => "panic".to_string(),
        },
    )
    .join()
    .unwrap()
}

/// Goes through `srcs` in one session, each held to the same source built cold.
fn holds(what: &str, srcs: &[String]) {
    let mut cache = Cache::default();
    // a walk comes back to its start: a source built cold once is that source's cold build
    let mut colds: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
    for (k, s) in srcs.iter().enumerate() {
        let kept = std::mem::take(&mut cache);
        let warm = match catch_unwind(AssertUnwindSafe(|| run(s, kept))) {
            Ok((r, c)) => {
                cache = c;
                outcome(&r)
            }
            // a session left mid-run is no test of the memory: start a fresh one
            Err(_) => "panic".to_string(),
        };
        let c = colds.entry(s).or_insert_with(|| cold(s)).clone();
        assert!(warm == c, "{what} step {k}: warm {warm} cold {c}\n{s}");
    }
    eprintln!("held {what}: {} steps", srcs.len());
}

/// The part's params with a number for a value (`shared n = 12`, `param f.n = 12.5mm`): each
/// one's line, and where in it the number stands.
fn numbers(src: &str) -> Vec<(usize, std::ops::Range<usize>)> {
    let mut out = vec![];
    for (i, l) in src.lines().enumerate() {
        if !(l.starts_with("shared ") || l.starts_with("param ")) {
            continue;
        }
        let Some(eq) = l.find(" = ") else { continue };
        let v = &l[eq + 3..];
        let v = v.split('#').next().unwrap_or_default().trim_end();
        let n = v.trim_end_matches("mm").trim_end_matches("deg");
        if n.parse::<f64>().is_ok() {
            out.push((i, eq + 3..eq + 3 + n.len()));
        }
    }
    out
}

/// `src` with line `at`'s number put to `v`.
fn with_number(src: &str, at: &(usize, std::ops::Range<usize>), v: f64) -> String {
    src.lines()
        .enumerate()
        .map(|(i, l)| {
            if i == at.0 {
                format!("{}{v:?}{}\n", &l[..at.1.start], &l[at.1.end..])
            } else {
                format!("{l}\n")
            }
        })
        .collect()
}

/// The part's top-level statements, each its lines (a block's to its closing brace).
fn statements(src: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut depth = 0i32;
    let mut cur = String::new();
    for l in src.lines() {
        cur.push_str(l);
        cur.push('\n');
        depth += l.matches(['{', '(', '[']).count() as i32 - l.matches(['}', ')', ']']).count() as i32;
        if depth == 0 {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The sources of a drag of the number at `at`, there and back, then a statement away and back.
fn walk(src: &str, at: &(usize, std::ops::Range<usize>)) -> Vec<String> {
    let mut out = Vec::new();
    let line = src.lines().nth(at.0).expect("the line");
    let v: f64 = line[at.1.clone()].parse().expect("a number");
    for k in [1.0, 1.01, 0.99, 1.005, 1.0 + 1e-15, 1.0] {
        out.push(with_number(src, at, v * k));
    }
    out.push(src.to_string());
    let stmts = statements(src);
    let without = |i: usize| -> String {
        stmts
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != i)
            .map(|(_, s)| s.as_str())
            .collect()
    };
    // undo after delete: the last statement, a middle one
    for i in [stmts.len().saturating_sub(1), stmts.len() / 2] {
        if i < stmts.len() {
            out.push(without(i));
            out.push(src.to_string());
        }
    }
    out
}

fn few() -> usize {
    if std::env::var_os("DRAG_WARM_ALL").is_some() {
        usize::MAX
    } else {
        2
    }
}

#[test]
fn bench_drags_leave_no_trace() {
    forge_model::suite::each(&BENCH, |name| {
        let src = bench(name);
        let dims = numbers(&src);
        // evenly over the part's numbers: early ones rebuild most
        let step = (dims.len() / few().min(dims.len()).max(1)).max(1);
        for d in dims.iter().step_by(step) {
            let line = src.lines().nth(d.0).unwrap_or_default();
            holds(&format!("{name} {line}"), &walk(&src, d));
        }
    });
}

/// With hardly anything kept, what is kept and forgotten changes no answer.
#[test]
fn forgetting_changes_nothing() {
    let src = bench("manifold");
    let d = numbers(&src).into_iter().next().expect("a number");
    let srcs = walk(&src, &d);
    for most in [0, 1, 2, 7, 64] {
        forge_bool::ssi::searched_keeps(most);
        forge_model::swept::tools_kept(most.min(3));
        holds(&format!("keeping {most}"), &srcs);
    }
    forge_bool::ssi::searched_keeps(4096);
    forge_model::swept::tools_kept(64);
}

/// Every number of a part's text moved by its last digit and back: a stale answer would be taken
/// here if anywhere.
#[test]
fn last_digits_are_not_taken_for_the_value() {
    for name in ["crank", "volute"] {
        let start = bench(name);
        let mut srcs = vec![start.clone()];
        let mut at = 0;
        let mut seen = 0;
        while let Some(i) = start[at..].find(|c: char| c.is_ascii_digit()) {
            let i = at + i;
            let end = i + start[i..]
                .find(|c: char| !c.is_ascii_digit() && c != '.')
                .unwrap_or(start.len() - i);
            at = end;
            // a number, not a name's digits
            let named = start[..i]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
            if !start[i..end].contains('.') || i == 0 || named {
                continue;
            }
            let Ok(v) = start[i..end].parse::<f64>() else {
                continue;
            };
            seen += 1;
            if seen % 5 != 0 {
                continue;
            }
            let up = f64::from_bits(v.to_bits() + 1);
            srcs.push(format!("{}{:?}{}", &start[..i], up, &start[end..]));
            srcs.push(start.clone());
        }
        assert!(srcs.len() > 2, "{name}");
        holds(&format!("{name} digits"), &srcs);
    }
}

/// A part built statement by statement, as an editor does: each prefix against itself built
/// alone. DRAG_WARM_FILE=path for a Forge file of your own.
#[test]
fn line_by_line_leaves_no_trace() {
    let srcs: Vec<String> = match std::env::var("DRAG_WARM_FILE") {
        Ok(p) => vec![std::fs::read_to_string(p).expect("the file")],
        Err(_) => ["hub", "clamp"].iter().map(|n| bench(n)).collect(),
    };
    forge_model::suite::each(&srcs, |src| {
        let stmts = statements(src);
        let prefixes: Vec<String> = (1..=stmts.len()).map(|n| stmts[..n].concat()).collect();
        holds("statement by statement", &prefixes);
    });
}

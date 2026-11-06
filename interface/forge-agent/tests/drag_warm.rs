//! What a drag remembers changes nothing. A part after any sequence of values and edits has, to the bit, what the part
//! built alone has: vertices, curves, surfaces, names, which faces and in what order. research/drag-speed.
//! DRAG_WARM_ALL=1 takes every number of every part, not the first few: a run of hours, by hand.
#![cfg(not(target_family = "wasm"))]
use forge_agent::bench;
use forge_agent::history::{SEEDS, run};
use forge_agent::model::Model;
use forge_agent::tools::Part;
use forge_geom::{Curve, CurveGeom, Surface};
use forge_math::Point2;
use forge_math::hash::BitHasher;
use forge_topo::Body;
use std::panic::{AssertUnwindSafe, catch_unwind};

type Fonts = Option<std::sync::Arc<forge_agent::model::Fonts>>;

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

/// What a model holds after an update: its error, or all of its part.
fn outcome(m: &Model) -> String {
    match (&m.error, m.body()) {
        (Some(e), _) => format!("error at {:?}: {}", m.features.len(), e.msg),
        (None, Some(b)) => format!("{} features {:x}", m.features.len(), digest(b)),
        (None, None) => "nothing".to_string(),
    }
}

/// `src` built alone, on a thread with nothing remembered.
fn cold(src: &str, fonts: &Fonts) -> String {
    let (src, fonts) = (src.to_string(), fonts.clone());
    std::thread::spawn(move || {
        let mut m = Model::default();
        m.fonts = fonts;
        match catch_unwind(AssertUnwindSafe(|| m.update(&src))) {
            Ok(_) => outcome(&m),
            Err(_) => "panic".to_string(),
        }
    })
    .join()
    .unwrap()
}

/// Goes through `srcs` on one model, each held to the same source built cold.
fn holds(what: &str, srcs: &[String], fonts: &Fonts) {
    let mut m = Model::default();
    m.fonts = fonts.clone();
    // a walk comes back to its start: a source built cold once is that source's cold build
    let mut colds: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
    for (k, s) in srcs.iter().enumerate() {
        let warm = match catch_unwind(AssertUnwindSafe(|| m.update(s))) {
            Ok(_) => outcome(&m),
            Err(_) => "panic".to_string(),
        };
        if warm == "panic" {
            // a model left mid-update is no test of the memory
            m = Model::default();
            m.fonts = fonts.clone();
        }
        let c = colds.entry(s).or_insert_with(|| cold(s, fonts)).clone();
        assert!(warm == c, "{what} step {k}: warm {warm} cold {c}\n{s}");
    }
    eprintln!("held {what}: {} steps", srcs.len());
}

/// The sources of a drag of number `d`, there and back, then a line away and back and a line moved.
fn walk(p: &mut Part, d: &forge_agent::tools::Dim) -> Vec<String> {
    let mut out = Vec::new();
    let start = p.model.src().to_string();
    for k in [1.0, 1.01, 0.99, 1.005, 1.0 + 1e-15, 1.0] {
        if p.model.edit(d.at.op(forge_agent::short(d.value * k))).is_ok() {
            out.push(p.model.src().to_string());
        }
    }
    out.push(start.clone());
    let lines: Vec<&str> = start.lines().collect();
    let without = |i: usize| {
        lines
            .iter()
            .enumerate()
            .filter(|&(k, _)| k != i)
            .map(|(_, l)| *l)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };
    // undo after delete: the last line, a middle one
    for i in [lines.len().saturating_sub(1), lines.len() / 2] {
        if i < lines.len() {
            out.push(without(i));
            out.push(start.clone());
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
    forge_model::suite::each(&bench::NAMES, |name| {
        let mut p = Part::of(bench::model(bench::src(name).expect("a bench part")));
        let fonts = p.model.fonts.clone();
        let dims = p.dims();
        // evenly over the part's numbers: early ones rebuild most
        let step = (dims.len() / few().min(dims.len()).max(1)).max(1);
        for d in dims.iter().step_by(step) {
            let srcs = walk(&mut p, d);
            holds(&format!("{name} {}", d.name), &srcs, &fonts);
            p = Part::of(bench::model(bench::src(name).unwrap()));
        }
    });
}

#[test]
fn history_drags_leave_no_trace() {
    // DRAG_WARM_SEEDS=first,count for fresh ones
    let fresh: Vec<u64> = std::env::var("DRAG_WARM_SEEDS").map_or(Vec::new(), |v| {
        let (a, n) = v.split_once(',').expect("first,count");
        let a: u64 = a.parse().expect("first");
        (a..a + n.parse::<u64>().expect("count")).collect()
    });
    let seeds: &[u64] = match () {
        _ if !fresh.is_empty() => &fresh,
        _ if few() == usize::MAX => &SEEDS,
        _ => &SEEDS[..12],
    };
    forge_model::suite::each(seeds, |&seed| {
        let src = run(seed, 12).src;
        let m = bench::model(&src);
        let mut p = Part::of(m);
        let fonts = p.model.fonts.clone();
        let dims = p.dims();
        let step = (dims.len() / few().min(dims.len()).max(1)).max(1);
        for d in dims.iter().step_by(step) {
            let srcs = walk(&mut p, d);
            holds(&format!("seed {seed} {}", d.name), &srcs, &fonts);
            let m = bench::model(&src);
            p = Part::of(m);
        }
    });
}

/// With hardly anything kept, what is kept and forgotten changes no answer.
#[test]
fn forgetting_changes_nothing() {
    let name = "manifold";
    let mut p = Part::of(bench::model(bench::src(name).unwrap()));
    let fonts = p.model.fonts.clone();
    let d = p.dims().into_iter().next().unwrap();
    let srcs = walk(&mut p, &d);
    for most in [0, 1, 2, 7, 64] {
        forge_bool::ssi::searched_keeps(most);
        forge_model::swept::tools_kept(most.min(3));
        holds(&format!("keeping {most}"), &srcs, &fonts);
    }
    forge_bool::ssi::searched_keeps(4096);
    forge_model::swept::tools_kept(64);
}

/// Every number of a part's lines moved by its last digit and back: a stale answer would be taken here if anywhere.
#[test]
fn last_digits_are_not_taken_for_the_value() {
    for name in ["crank", "volute"] {
        let src = bench::src(name).unwrap();
        let p = Part::of(bench::model(src));
        let fonts = p.model.fonts.clone();
        let start = p.model.src().to_string();
        let mut srcs = vec![start.clone()];
        let mut at = 0;
        let mut seen = 0;
        while let Some(i) = start[at..].find(|c: char| c.is_ascii_digit()) {
            let i = at + i;
            let end = i + start[i..]
                .find(|c: char| !c.is_ascii_digit() && c != '.')
                .unwrap_or(start.len() - i);
            at = end;
            if !start[i..end].contains('.') || i == 0 {
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
        holds(&format!("{name} digits"), &srcs, &fonts);
    }
}

/// A part built line by line, as an editor or the history generator does: each prefix against itself built alone, and valid
/// or not alike. DRAG_WARM_FILE=path for a file of your own.
#[test]
fn line_by_line_leaves_no_trace() {
    let srcs: Vec<String> = match std::env::var("DRAG_WARM_FILE") {
        Ok(p) => vec![std::fs::read_to_string(p).expect("the file")],
        Err(_) => SEEDS[..6].iter().map(|&s| run(s, 12).src).collect(),
    };
    forge_model::suite::each(&srcs, |src| {
        let lines: Vec<&str> = src.lines().collect();
        let fonts = bench::model(src).fonts.clone();
        let prefixes: Vec<String> = (1..=lines.len()).map(|n| lines[..n].join("\n") + "\n").collect();
        holds("line by line", &prefixes, &fonts);
        let mut m = Model::default();
        m.fonts = fonts;
        for p in &prefixes {
            m.update(p);
            if let Some(b) = m.body() {
                let found = forge_topo::check(&b.compact());
                eprintln!(
                    "{} lines: {} problems {:?}",
                    p.lines().count(),
                    found.len(),
                    found.first()
                );
            }
        }
    });
}

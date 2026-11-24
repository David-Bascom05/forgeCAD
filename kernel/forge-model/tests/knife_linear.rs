//! Knife edges one after another, each refused plain and built blunted. A retry used to rebuild
//! the part through retries of its own, doubling with each (16 took 30 s, 20 over 40). The work is
//! counted, not timed: a loaded machine moves any clock. Every feature stepped, on any path (one
//! body or bodies, plain or blunted, first try or retry), is a rung of the decision log, and the
//! steps grow linearly with the knife edges. Its own test binary, so no other test steps a feature
//! into the count. Run with `--features decisions` (CI: platforms.yml); without it there is
//! nothing to count, and nothing runs.
#![cfg(feature = "decisions")]

use forge_math::dmath;
use forge_model::Part;

/// One boss with a knife edge on a rod: the rod at x, the boss its round ends and a tangent side
/// (interface/forge-lang/tests/kernel_fusion_parts.rs, `knife_boss`).
fn knife_boss(x: f64) -> String {
    let at = |cx: f64, cy: f64, r: f64, deg: f64| {
        let a = deg.to_radians();
        (cx + r * dmath::cos(a), cy + r * dmath::sin(a))
    };
    let p = at(x, -25.0, 10.0, 163.0265);
    let t = at(x, 0.0, 18.0, -109.3809 - 45.6191);
    let a: f64 = (100.0 - 324.0 + 625.0) / 50.0;
    let q = (x - (100.0 - a * a).sqrt(), -25.0 + a);
    let s = q;
    format!(
        "  a = arc(center: ({x}, -25), from: ({}, {}), to: ({}, {}), cw: true)
  b = arc(center: ({x}, 0), from: ({}, {}), to: ({}, {}), cw: true)
  l = line(({}, {}), ({}, {}))",
        p.0, p.1, q.0, q.1, s.0, s.1, t.0, t.1, t.0, t.1, p.0, p.1
    )
}

fn source(n: usize, bodies: bool) -> String {
    let mut src = String::new();
    for k in 0..n {
        let x = 200.0 * k as f64 + 7.0;
        let new = if bodies && k > 0 { ", new: true" } else { "" };
        src += &format!(
            "sketch rod{k}(on: xy) {{\n  c = circle(({x}, 0), 18)\n}}\nr{k} = extrude(rod{k}, 20{new})\n\
             sketch boss{k}(on: xy) {{\n{}\n}}\nb{k} = extrude(boss{k}, 8)\n",
            knife_boss(x)
        );
    }
    src
}

/// The features stepped building n knife edges: the language's build, then the kernel's
/// sequential and n-ary regenerations.
fn steps(n: usize, bodies: bool) -> u64 {
    let _ = forge_bool::decisions::take();
    let src = source(n, bodies);
    let (run, cache) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
    assert!(run.errors.is_empty(), "{}", forge_lang::report::text(&run, &src));
    let (features, ids) = cache.history();
    let part = Part {
        features: features.to_vec(),
    };
    part.regenerate_solids_ids(Some(ids)).expect("sequential");
    part.regenerate_nary_solids_ids(1, Some(ids)).expect("n-ary");
    let stats = forge_bool::decisions::take();
    stats
        .rungs
        .get("feature stepped")
        .map_or(0, |r| r.outcomes.iter().sum())
}

#[test]
fn many_knife_edges_stay_linear() {
    for bodies in [false, true] {
        let (s10, s40) = (steps(10, bodies), steps(40, bodies));
        eprintln!("bodies {bodies}: features stepped, 10 knife edges {s10}, 40 {s40}");
        assert!(s10 > 0);
        // linear: four times the knife edges, about four times the steps (a doubling per knife
        // edge would be 2^30 times)
        assert!(s40 <= 5 * s10, "10 knife edges {s10} steps, 40 {s40}");
    }
}

//! Time to read and solve a typical sketch, and to evaluate parameters: a bracket's outline of
//! eight lines, four holes, a slot of two lines and two arcs, dimensioned by params.
//!   sketchbench [CASE] [N]   N: only that many calls of the one case named (for counting
//!                            instructions)
//! (The .fc sketch's horizontal and vertical distances have no word in the language: those
//! points are placed by their coordinates here.)
use std::time::Instant;

const PARAMS: &str = "shared w = 80\nshared h = 50\nshared wall = 2\nshared r = wall * 2 + 1\nshared d = sqrt(w ^ 2 + h ^ 2) / 4\n";

/// The bracket, `SLOT` the slot arcs' distance apart.
fn sketch(slot: f64) -> String {
    format!(
        "sketch s(on: xy) {{
  o = poly([(0, 0), near(80, -0.2), near(80, 25), near(60, 25.4), near(60, 50), near(20, 50.2), near(20, 25), near(0, 25)])
  horizontal(o[0])
  horizontal(o[2])
  horizontal(o[4])
  horizontal(o[6])
  vertical(o[1])
  vertical(o[3])
  vertical(o[5])
  vertical(o[7])
  distance(o[0], w)
  distance(o[1], h / 2)
  distance(o[3], h / 2)
  distance(o[2], w / 4)
  distance(o[6], w / 4)
  c1 = circle((r * 2, r * 2), 3)
  c2 = circle((w - r * 2, r * 2), 3)
  c3 = circle((w * 3 / 8, 40), 2.5)
  c4 = circle((w * 5 / 8, 40), 2.5)
  a1 = arc(center: near(50, 15), from: near(50, 12), to: near(50, 18))
  a2 = arc(center: near(30, 15), from: near(30, 18), to: near(30, 12))
  l9 = line(a1.b, a2.a)
  l10 = line(a2.b, a1.a)
  horizontal(l9)
  tangent(l9, a1)
  tangent(l10, a1)
  tangent(l9, a2)
  tangent(l10, a2)
  radius(a1, 3)
  equal(a1, a2)
  distance(a1.center, a2.center, {slot})
  fix(a2.center)
}}
"
    )
}

/// Microseconds per call of `f`, the best of 20 rounds (the machine may be busy). With an
/// argument, only that many calls of the one case named (for counting instructions).
fn best(n: usize, mut f: impl FnMut(usize)) -> f64 {
    if let Some(n) = std::env::args().nth(2).and_then(|a| a.parse().ok()) {
        (0..n).for_each(&mut f);
        return 0.0;
    }
    (0..20)
        .map(|_| {
            let t = Instant::now();
            (0..n).for_each(&mut f);
            t.elapsed().as_secs_f64() * 1e6 / n as f64
        })
        .fold(f64::MAX, f64::min)
}

fn case(what: &str) -> bool {
    std::env::args().nth(1).is_none_or(|a| what.starts_with(&a))
}

fn opts() -> forge_lang::Options {
    forge_lang::Options {
        build: false,
        checks: false,
        ..forge_lang::Options::default()
    }
}

fn main() {
    let src = format!("{PARAMS}{}", sketch(20.0));
    let (run, _) = forge_lang::run(&src, opts(), Default::default());
    assert!(run.errors.is_empty(), "{:?}", run.errors);
    for (what, s) in [
        ("params, 5 lines", PARAMS.to_string()),
        ("sketch read and solved, with the params", src.clone()),
    ] {
        if !case(what) {
            continue;
        }
        let us = best(200, |_| {
            std::hint::black_box(forge_lang::run(&s, opts(), Default::default()));
        });
        println!("{what}: {us:.2} us");
    }
    // one dimension changed, as a session re-reads the source: from scratch, and from the last run
    let moved = format!("{PARAMS}{}", sketch(21.0));
    for (what, keep) in [
        ("dimension changed, from scratch", false),
        ("dimension changed, from the last run", true),
    ] {
        if !case(what) {
            continue;
        }
        // each call reads the other source: from the last call's cache, or from none
        let mut cache = Some(forge_lang::run(&src, opts(), Default::default()).1);
        let us = best(200, |k| {
            let s = if k % 2 == 0 { &moved } else { &src };
            let c = if keep {
                cache.take().unwrap_or_default()
            } else {
                Default::default()
            };
            let (r, c) = forge_lang::run(s, opts(), c);
            std::hint::black_box(r);
            cache = Some(c);
        });
        println!("{what}: {us:.2} us");
    }
}

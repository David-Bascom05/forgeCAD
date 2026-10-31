//! Time to read and solve a typical sketch, and to evaluate parameters.

use forge_agent::text;
use std::time::Instant;

const PARAMS: &str = "w = 80\nh = 50\nwall = 2\nr = wall*2 + 1\nd = sqrt(w^2 + h^2)/4 mm\n";

/// A bracket: an outline of eight lines, four holes, a slot of two lines and two arcs.
const SKETCH: &str = "sketch
  poly 0.3 0 80 -0.2 80 25 60 25.4 60 50 20 50.2 20 25 0 25
  circle 10 10 3
  circle 70 10 3
  circle 30 40 2.5
  circle 50 40.3 2.5
  line 30 12 50 12.2
  line 50 18 30 18
  arc 50 15 l9.b l10.a
  arc 30 15 l10.b l9.a
  fix l1.a
  hor l1 l3 l5 l7
  ver l2 l4 l6 l8
  dist l1 w
  dist l2 h/2
  dist l4 h/2
  hdist l1.a l6.b w/4
  hdist l4.a l1.b w/4
  dia c1 6
  eq c1 c2
  dia c3 5
  eq c3 c4
  hdist l1.a c1 r*2
  vdist l1.a c1 r*2
  hdist c2 l1.b r*2
  vdist l1.a c2 r*2
  hdist l1.a c3 w*3/8
  hdist c4 l1.b w*3/8
  vdist l1.a c3 40
  hor c3 c4
  hor l9
  tan l9 a1
  tan l10 a1
  tan l9 a2
  tan l10 a2
  rad a1 3
  dist a1 a2 20
  hdist l1.a a2 30
  vdist l1.a a2 15
end
";

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

fn read(what: &str, src: &str) {
    if !case(what) {
        return;
    }
    let us = best(200, |_| {
        std::hint::black_box(text::parse(src).expect("parses"));
    });
    println!("{what}: {us:.2} us");
}

fn main() {
    let src = format!("{PARAMS}{SKETCH}");
    let part = text::parse(&src).expect("parses");
    let forge_model::Feature::Sketch(s) = &part.features[5] else {
        panic!("line 6 is the sketch")
    };
    let solved = &s.solved;
    println!(
        "{} entities, {} constraints",
        solved.entity_ids().count(),
        solved.constraint_ids().count()
    );
    read("params, 5 lines", PARAMS);
    read("sketch read and solved, with the params", &src);
    let what = "re-solve, nothing moved";
    let us = if !case(what) {
        0.0
    } else {
        best(200, |_| {
            let mut q = solved.clone();
            std::hint::black_box(q.solve());
        })
    };
    println!("{what}: {us:.2} us");
    let mut r = forge_math::rng::Rng::new(1);
    for d in [0.01, 0.3] {
        if !case(&format!("re-solve {d}")) {
            continue;
        }
        let jit: Vec<Vec<f64>> = (0..16)
            .map(|_| solved.params().iter().map(|v| v + r.range(-d, d)).collect())
            .collect();
        let mut its = 0;
        let us = best(200, |k| {
            let mut q = solved.clone();
            q.set_params(&jit[k % 16]).expect("same length");
            let rep = q.solve();
            its += rep.iterations;
            assert_eq!(rep.status, forge_sketch::Status::Solved);
        });
        println!(
            "re-solve, every coordinate moved up to {d} mm: {us:.2} us, {:.1} iterations",
            its as f64 / 4000.0
        );
    }
}

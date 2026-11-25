//! shape.rs's screen takes an arc within 1e-12 rad of a whole turn as whole (`on_sweep`, and the
//! ray's arc-end graze test skipped). Taken whole while another segment closes the profile, the
//! ray's parity counts both the arc across its gap and the closing segment: the disc's centre,
//! far from every boundary, reads Out. What keeps that case out of every part is not the model's
//! size but the profile's own refusal: `Segment::degenerate` (forge-ops profile.rs) refuses an arc
//! whose sweep is within `ANGULAR` = 1e-11 rad of a whole turn, at any radius, so the only whole
//! turns a shape meets are circles (sweep exactly 2pi, alone in their loop: `CircleNotAlone`).
//! Here the adversarial part: a 12 km and a 16 km arc (the second at `InputSize`'s 1.6e7, past
//! the 5e5 model half size) 1e-12 rad short of whole, closed by a line longer than `MIN_LENGTH`,
//! so the line is legitimate and only the near-whole arc can refuse it.

use forge_lang::{Options, Run};

fn run(src: &str) -> Run {
    forge_lang::run(src, Options::default(), Default::default()).0
}

/// A disc of radius `r` whose arc stops `ty` below its start, closed by a line, with a pocket
/// whose wall lies at `y` (inside the gap's band when `ty < y < 0`).
fn part(r: &str, ty: &str, y: &str) -> String {
    format!(
        "sketch plan(on: xy) {{
  a = arc(center: (0, 0), from: ({r}, 0), to: ({r}, {ty}))
  l = line(({r}, {ty}), ({r}, 0))
}}
base = extrude(plan, 10)
sketch top(on: base.end) {{
  p = rect(corner: (-5, {y}), size: (10, 5))
}}
pocket = cut(top.p, 3)
"
    )
}

#[test]
fn an_arc_a_hair_short_of_whole_is_refused_at_any_size() {
    // 12 km: 1.2e-5 below its start is 1e-12 rad; 16 km: 1.6e-5, 1e-12 rad again
    for (r, ty) in [("12000000", "-0.000012"), ("16000000", "-0.000016")] {
        let run = run(&part(r, ty, &format!("{}", ty.parse::<f64>().unwrap() / 2.0)));
        assert!(
            !run.ok(),
            "r {r}: a profile arc within 1e-12 rad of whole was taken"
        );
        let e = format!("{:?}", run.errors[0]);
        assert!(
            e.contains("SketchProfile") && e.contains("Degenerate"),
            "r {r}: {e}"
        );
    }
}

#[test]
fn an_arc_short_by_more_than_angular_is_answered_exactly() {
    // 12 km, 1e-3 below: 8.3e-11 rad short, past ANGULAR; the pocket's wall sits in the gap's
    // band (the ray through it crosses the closing line), and the pocket is cut
    let run = run(&part("12000000", "-0.001", "-0.0005"));
    assert!(run.ok(), "{:?}", run.errors);
    let b = run.body.as_ref().unwrap();
    assert!(forge_topo::check(b).is_empty());
    // the disc's arc side, its closing side, its two caps, and the pocket's four walls and floor
    assert_eq!(b.face_ids().count(), 9);
}

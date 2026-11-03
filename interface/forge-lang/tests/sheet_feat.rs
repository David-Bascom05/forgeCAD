//! The newer sheet features in Forge (contour, ...): parts in parts/sheet against forge-sheet's
//! closed forms, and their refusals typed.
use forge_lang::error::Error;
use forge_lang::{Options, Run};
use forge_math::dmath::PI;
use forge_topo::Body;

fn src(name: &str) -> String {
    let path = format!("{}/parts/sheet/{name}.fg", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).expect("part file")
}

fn run(src: &str) -> Run {
    forge_lang::run(src, Options::default(), Default::default()).0
}

fn clean(name: &str) -> Run {
    let s = src(name);
    let r = run(&s);
    assert!(
        r.errors.is_empty() && r.warnings.is_empty(),
        "{name}: {}",
        forge_lang::report::text(&r, &s)
    );
    assert!(r.expects.iter().all(|e| e.ok), "{name}");
    assert_eq!(r.measure.as_ref().expect("measured").problems, 0, "{name}");
    r
}

fn mass(b: &Body) -> (f64, f64) {
    let m = forge_topo::mass::mass(b).expect("mass");
    (m.volume, m.err[0])
}

fn near(what: &str, got: (f64, f64), want: f64) {
    assert!(
        (got.0 - want).abs() <= got.1 + 1e-9 * want.abs().max(1.0),
        "{what}: {} against {want}",
        got.0
    );
}

fn first_error(s: &str) -> Error {
    run(s)
        .errors
        .first()
        .cloned()
        .unwrap_or_else(|| panic!("no error in\n{s}"))
}

/// Folded and flat volumes: straights (tangent line to tangent line) and bends (angle, inside
/// radius), all `w` long.
fn closed(t: f64, k: f64, w: f64, straights: f64, bends: &[(f64, f64)]) -> (f64, f64) {
    let zones: f64 = bends.iter().map(|&(a, r)| a * (r * t + 0.5 * t * t)).sum();
    let ba: f64 = bends.iter().map(|&(a, r)| a * (r + k * t)).sum();
    (t * w * straights + w * zones, t * w * (straights + ba))
}

#[test]
fn a_hat_contour_matches_the_closed_form() {
    let r = clean("hat");
    let (t, k, w) = (1.5, 0.4, 25.0);
    let q = PI / 2.0;
    // the r 4 arc turns towards the material: inside radius 4 - t; the sharp corner the same way
    // at the sheet's radius 2, so both lines there lose (2 + t) to the reference setback
    let straights = 15.0 + 14.0 + (18.0 - 3.5) + (21.0 - 3.5);
    let (vf, vl) = closed(t, k, w, straights, &[(q, 3.0), (q, 4.0 - t), (q, 2.0)]);
    let s = &r.sheets[0];
    assert_eq!(s.walls.len(), 4);
    near("hat folded", mass(s.folded.as_ref().expect("built")), vf);
    near("hat flat", mass(s.unfolds[0].1.as_ref().expect("unfolded")), vl);
}

#[test]
fn a_contour_off_an_edge_matches_the_closed_form() {
    let r = clean("lip");
    let (t, k, w) = (2.0, 0.42, 30.0);
    let q = PI / 2.0;
    // the base to its tangent line (setback 2 at r 2), the wall up (17 less that setback), the
    // lip after the r 3 arc (12), the arc turning towards the material (inside radius 3 - t)
    let straights = (40.0 - 2.0) + (17.0 - 2.0) + 12.0;
    let (vf, vl) = closed(t, k, w, straights, &[(q, 2.0), (q, 3.0 - t)]);
    let s = &r.sheets[0];
    assert_eq!(s.walls.len(), 3);
    near("lip folded", mass(s.folded.as_ref().expect("built")), vf);
    near("lip flat", mass(s.unfolds[0].1.as_ref().expect("unfolded")), vl);
}

#[test]
fn contour_refusals_are_typed() {
    // an arc turning towards the material tighter than t
    let s = src("hat").replace("param sec.r2 = 4", "param sec.r2 = 1");
    assert_eq!(first_error(&s).kind(), "ContourRadius");
    // a last line shorter than the sharp corner's setback (2 + t)
    let s = src("hat").replace("line(to: (b, 0))", "line(to: (b, h + r2 - 3))");
    assert_eq!(first_error(&s).kind(), "ContourWallTooShort");
    // off an edge, the profile on a plane not square to it
    let s = src("lip").replace("about: plan.outline[0]", "about: plan.outline[1]");
    let e = first_error(&s);
    assert!(
        matches!(
            e.kind(),
            "ProfileOffSection" | "ContourStartOffEdge" | "ContourCoplanar"
        ),
        "{}",
        e.kind()
    );
    // a contour takes no width off an edge
    let s = src("lip").replace(
        "edge: base.edge(plan.outline[1]))",
        "edge: base.edge(plan.outline[1]), width: 3)",
    );
    assert_eq!(first_error(&s).kind(), "NotWith");
}

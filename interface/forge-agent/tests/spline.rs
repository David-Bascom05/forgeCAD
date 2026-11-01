//! Curves through the part file: loops, sketch rows, round trips.
use forge_agent::model::Model;
use forge_math::dmath;

fn built(src: &str) -> Model {
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error);
    m
}

fn volume(m: &Model) -> f64 {
    let b = m.body().expect("a body");
    assert!(forge_topo::check(b).is_empty());
    forge_topo::mass::mass(b).unwrap().volume
}

#[test]
fn loops_in_a_part_file() {
    let m = built("extrude 4 ellipse 0 0 7 3 20\n");
    assert!((volume(&m) - dmath::PI * 21.0 * 4.0).abs() < 1e-6);
    let m = built("extrude 5 loop thru 0 0 10 8 20 8 30 0 line 30 0 0 0\n");
    assert!(volume(&m) > 0.0);
    let m = built("extrude 5 loop spline 2 0 0 5 8 10 0 line 10 0 0 0\n");
    // a parabolic arch: 2/3 of the chord times half the control height, 5 high
    assert!(
        (volume(&m) - 5.0 * (2.0 / 3.0) * 10.0 * 4.0).abs() < 1e-9,
        "{}",
        volume(&m)
    );
    let m = built("extrude 3 loop ell 0 0 6 2 0 0 180 line -6 0 6 0\n");
    assert!((volume(&m) - 3.0 * 0.5 * dmath::PI * 12.0).abs() < 1e-6);
    let m = built("extrude 2 loop spline 1 0 0 4 0 4 4 0 4 0 0 knots 0 0 1 2 3 4 4\n");
    assert!((volume(&m) - 32.0).abs() < 1e-9, "{}", volume(&m));
}

#[test]
fn sketch_rows_solve_and_extrude() {
    let m = built(
        "s: sketch xy\n  spline 0 0 10 8 20 8 30 0 open\n  line s1.b s1.a\n  fix s1.a\n  hdist s1.a s1.b 40\nend\nextrude 5 s\n",
    );
    assert!(volume(&m) > 0.0);
    let m = built("s: sketch xy\n  ellipse o 8 0 3\n  rad e1 2\n  fix e1.m\nend\nextrude 1 s\n");
    assert!((volume(&m) - dmath::PI * 16.0).abs() < 1e-6, "{}", volume(&m));
    let m = built(
        "s: sketch xy\n  line 0 0 10 0\n  spline l1.b 15 5 10 10 open\n  line s1.b 0 10\n  line l2.b l1.a\n  tan l1 s1\nend\nextrude 1 s\n",
    );
    assert!(volume(&m) > 100.0);
    let m = built("s: sketch xy\n  bspline 3 0 0 5 8 10 8 15 0 open\n  line s1.b s1.a\nend\nextrude 1 s\n");
    assert!(volume(&m) > 0.0);
}

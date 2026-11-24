//! A move's arc gets the slab closed form only when its interval cross product proves it
//! strictly under a half turn in its sense (the closed form's precondition, DistArc.lean). A
//! vanishing arc (a near-tangent corner's, from an offset) is not provably so and goes to the
//! branch and bound; the manifold test part hit slab::dist's debug assertion with one.
use forge_cam_geom::check::{Move, Planar};

fn arc(t1: f64) -> Move {
    Move::Arc {
        c: [10.0, 5.0, -2.0],
        x: [1.0, 0.0, 0.0],
        y: [0.0, 1.0, 0.0],
        r: 7.0,
        t0: 0.25,
        t1,
    }
}

#[test]
fn a_vanishing_arc_has_no_planar_form() {
    assert!(matches!(arc(0.75).planar(), Some(Planar::Arc { .. })));
    assert!(matches!(arc(0.75).planar(), Some(Planar::Arc { ccw: true, .. })));
    // the next f64 after t0: its ends the same f64 points
    assert!(arc(0.25f64.next_up()).planar().is_none());
    assert!(arc(0.25).planar().is_none());
    // clockwise the same
    assert!(matches!(arc(-0.5).planar(), Some(Planar::Arc { ccw: false, .. })));
}

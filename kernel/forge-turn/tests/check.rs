//! The certified clearance on a plain shaft: proven when clear, a witness when it gouges.
use forge_cam_geom::region::Region;
use forge_turn::profile::rect;
use forge_turn::check::{Move, Obstacle, Verdict, clearance};
use forge_turn::insert::builtin;

fn shaft() -> Obstacle {
    // ρ ∈ [0, 10], z ∈ [−40, 0]
    Obstacle::of("part", &Region { loops: vec![rect(0.0, -40.0, 10.0, 0.0)] }, 1e-5).unwrap()
}

#[test]
fn library_decodes() {
    for id in ["od-rough-cnmg", "od-finish-dnmg", "od-profile-vbmt", "groove-3", "part-3", "thread-60", "drill-8"] {
        let t = builtin(id).unwrap();
        assert!(!t.bodies.is_empty(), "{id}");
    }
    let t = builtin("od-rough-cnmg").unwrap();
    assert_eq!(t.re, 0.8);
    assert_eq!(t.edges, [5.0, 85.0]);
}

#[test]
fn turning_along_the_shaft() {
    let t = builtin("od-rough-cnmg").unwrap();
    let mut part = shaft();
    let r = t.re;
    // the nose centre 0.3 off the finished radius plus the nose: clear by 0.3 (less mu)
    let mv = Move::Line { a: [10.0 + r + 0.3, 2.0], b: [10.0 + r + 0.3, -30.0] };
    for b in &t.bodies {
        let need = if b.cutter { 0.29 } else { 0.5 };
        let v = clearance(b, &mv, &mut part, need);
        assert!(matches!(v, Verdict::Clear { .. }), "{}: {v:?}", b.name);
    }
    // 0.2 into the part: a witness
    let mv = Move::Line { a: [10.0 + r - 0.2, 2.0], b: [10.0 + r - 0.2, -30.0] };
    let v = clearance(&t.bodies[0], &mv, &mut part, 0.0);
    assert!(matches!(v, Verdict::Hit { .. }), "{v:?}");
}

#[test]
fn facing_to_the_centre() {
    let t = builtin("od-rough-cnmg").unwrap();
    let mut part = shaft();
    let r = t.re;
    let mv = Move::Line { a: [14.0, r + 0.1], b: [0.0, r + 0.1] };
    let v = clearance(&t.bodies[0], &mv, &mut part, 0.09);
    assert!(matches!(v, Verdict::Clear { .. }), "{v:?}");
}

#[test]
fn an_arc_move_round_the_corner() {
    let t = builtin("od-finish-dnmg").unwrap();
    let mut part = shaft();
    let r = t.re + 0.05;
    // round the part's corner (10, 0) at distance r: the nose centre's arc, from above the face
    // to beside the cylinder
    let mv = Move::Arc { a: [10.0, r], b: [10.0 + r, 0.0], c: [10.0, 0.0], ccw: false };
    let v = clearance(&t.bodies[0], &mv, &mut part, 0.04);
    assert!(matches!(v, Verdict::Clear { .. }), "{v:?}");
}


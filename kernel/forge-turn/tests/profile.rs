//! Half-profiles of revolved parts, exact in kind.
mod common;
use common::part;
use forge_cam_geom::region::Seg;
use forge_turn::profile::half_profile;

#[test]
fn a_stepped_shaft_with_a_chamfer() {
    // ρ, z: a shaft of radius 10 from z = -40 to 0 with a 1 mm chamfer at the end, a shoulder
    // to radius 15 at z = -25
    let b = part(&[
        (0.0, -40.0),
        (15.0, -40.0),
        (15.0, -25.0),
        (10.0, -25.0),
        (10.0, -1.0),
        (9.0, 0.0),
        (0.0, 0.0),
    ]);
    let hp = half_profile(&b).unwrap();
    assert_eq!(hp.region.loops.len(), 1);
    let segs = &hp.region.loops[0].segs;
    assert_eq!(segs.len(), 7, "{segs:?}");
    assert!(segs.iter().all(|s| matches!(s, Seg::Line { .. })));
    assert_eq!(hp.bbox, [0.0, -40.0, 15.0, 0.0]);
    assert!(hp.mu < 1e-4, "{}", hp.mu);
}

#[test]
fn a_bored_bush_does_not_touch_the_axis() {
    let b = part(&[(5.0, 0.0), (5.0, 20.0), (12.0, 20.0), (12.0, 0.0)]);
    let hp = half_profile(&b).unwrap();
    assert_eq!(hp.region.loops.len(), 1);
    assert_eq!(hp.region.loops[0].segs.len(), 4);
    assert_eq!(hp.bbox, [5.0, 0.0, 12.0, 20.0]);
}

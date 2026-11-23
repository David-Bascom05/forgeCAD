//! Half-profiles of revolved parts, exact in kind.
use forge_cam_geom::region::Seg;
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_ops::profile::{Profile, ProfileLoop, Segment};
use forge_ops::revolve::{Axis2, revolve};
use forge_turn::profile::half_profile;

fn xz() -> Frame {
    Frame::from_axes(Point3::new(0.0, 0.0, 0.0), Vec3::X, Vec3::Z, -Vec3::Y).unwrap()
}

/// A revolved part from its (ρ, z) outline: lines, and arcs given by (centre, radius, start, sweep).
pub fn part(pts: &[(f64, f64)]) -> forge_topo::Body {
    let n = pts.len();
    let segs = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            (
                Segment::Line {
                    a: Point2::new(a.0, a.1),
                    b: Point2::new(b.0, b.1),
                },
                i as u64,
            )
        })
        .collect();
    let pr = Profile {
        plane: xz(),
        loops: vec![ProfileLoop::new(segs)],
    };
    revolve(
        &pr,
        Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        dmath::TAU,
        1,
    )
    .unwrap()
}

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

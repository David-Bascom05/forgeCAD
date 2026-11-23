//! The decide-once path on real bodies: the ledger filled from planar B-reps, the core's answer
//! written as a fresh body, which must be valid and of the right volume and topology.
#![cfg(feature = "decide_once")]

use forge_bool::Op;
use forge_bool::decide_once;
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, Vec3};
use forge_mesh::{Quality, tessellate};
use forge_ops::{Profile, extrude};
use forge_topo::{Body, check};

fn block(origin: Point3, w: f64, h: f64, d: f64, feature: u64) -> Body {
    let plane = Frame::WORLD.with_origin(origin);
    extrude(
        &Profile {
            plane,
            loops: vec![Profile::rectangle(Point2::ORIGIN, w, h, 0)],
        },
        d,
        feature,
    )
    .unwrap()
}

fn volume(b: &Body) -> f64 {
    let open = tessellate(b, &Quality::default()).unwrap().open_edges();
    assert!(open.is_empty(), "{} open mesh edges", open.len());
    forge_topo::mass::mass(b).unwrap().volume
}

fn counts(b: &Body) -> (usize, usize, usize) {
    (b.face_ids().count(), b.edge_ids().count(), b.vertex_ids().count())
}

fn run(a: &Body, b: &Body, op: Op) -> Result<Body, forge_bool::BoolError> {
    let r = decide_once::boolean(a, b, op)?.expect("flat and unambiguous");
    let p = check(&r);
    assert!(p.is_empty(), "{op:?}: {p:?}");
    Ok(r)
}

#[test]
fn overlapping_blocks_all_ops() {
    let a = block(Point3::ORIGIN, 10.0, 10.0, 10.0, 1);
    let b = block(Point3::new(4.0, 5.0, 6.0), 10.0, 10.0, 10.0, 2);
    let overlap = 6.0 * 5.0 * 4.0;
    for (op, want, top) in [
        (Op::Union, 2000.0 - overlap, (12, 30, 20)),
        (Op::Subtract, 1000.0 - overlap, (9, 21, 14)),
        (Op::Intersect, overlap, (6, 12, 8)),
    ] {
        let r = run(&a, &b, op).unwrap();
        assert!(
            (volume(&r) - want).abs() < 1e-9,
            "{op:?}: {} vs {want}",
            volume(&r)
        );
        assert_eq!(counts(&r), top, "{op:?}");
    }
}

#[test]
fn face_to_face_union_is_one_box() {
    let a = block(Point3::ORIGIN, 10.0, 10.0, 10.0, 1);
    let b = block(Point3::new(10.0, 0.0, 0.0), 10.0, 10.0, 10.0, 2);
    let r = run(&a, &b, Op::Union).unwrap();
    assert_eq!(counts(&r), (6, 12, 8));
    assert!((volume(&r) - 2000.0).abs() < 1e-9);
}

#[test]
fn touches_are_refused() {
    let a = block(Point3::ORIGIN, 10.0, 10.0, 10.0, 1);
    let corner = block(Point3::new(10.0, 10.0, 10.0), 5.0, 5.0, 5.0, 2);
    let edge = block(Point3::new(10.0, 10.0, 0.0), 5.0, 5.0, 10.0, 2);
    for b in [corner, edge] {
        assert!(matches!(
            run(&a, &b, Op::Union),
            Err(forge_bool::BoolError::ZeroThickness { .. })
        ));
    }
}

#[test]
fn through_slot_and_pocket() {
    let plate = block(Point3::ORIGIN, 30.0, 20.0, 6.0, 1);
    let slot = block(Point3::new(10.0, -1.0, -1.0), 5.0, 22.0, 8.0, 2);
    let r = run(&plate, &slot, Op::Subtract).unwrap();
    assert!((volume(&r) - (3600.0 - 5.0 * 20.0 * 6.0)).abs() < 1e-9);
    assert_eq!(r.shells.len(), 2, "two pieces");
    let pocket = block(Point3::new(10.0, 5.0, 3.0), 5.0, 5.0, 4.0, 3);
    let r = run(&plate, &pocket, Op::Subtract).unwrap();
    assert!((volume(&r) - (3600.0 - 75.0)).abs() < 1e-9);
    assert_eq!(counts(&r), (11, 24, 16));
}

#[test]
fn walls_within_linear_are_one_surface() {
    // B's end wall 0.6 LINEAR inside A's: snapped flush, the union an L with no sliver
    let e = 0.6 * forge_math::tol::LINEAR;
    let a = block(Point3::ORIGIN, 20.0, 10.0, 5.0, 1);
    let b = block(Point3::new(10.0, 0.0, 5.0), 10.0 - e, 10.0, 5.0, 2);
    let r = run(&a, &b, Op::Union).unwrap();
    assert_eq!(counts(&r), (8, 18, 12));
}

#[test]
fn random_rotated_blocks_keep_volume_identities() {
    let mut rng = Rng::new(1000);
    let mut ran = 0;
    for _ in 0..40 {
        let a = block(Point3::new(-5.0, -5.0, -5.0), 10.0, 10.0, 10.0, 1);
        let z = Vec3::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(-1.0, 1.0))
            .normalize()
            .unwrap();
        let o = Point3::new(rng.range(-6.0, 6.0), rng.range(-6.0, 6.0), rng.range(-6.0, 6.0));
        let b = extrude(
            &Profile {
                plane: Frame::from_z(o, z),
                loops: vec![Profile::rectangle(Point2::new(-3.0, -4.0), 6.0, 8.0, 0)],
            },
            7.0,
            2,
        )
        .unwrap();
        let (Ok(u), Ok(i), Ok(s)) = (
            run(&a, &b, Op::Union),
            run(&a, &b, Op::Intersect),
            run(&a, &b, Op::Subtract),
        ) else {
            continue;
        };
        let (vu, vi, vs) = (volume(&u), volume(&i), volume(&s));
        assert!((vu + vi - (1000.0 + 336.0)).abs() < 1e-8, "{vu} + {vi}");
        assert!((vs + vi - 1000.0).abs() < 1e-8, "{vs} + {vi}");
        ran += 1;
    }
    assert!(ran >= 36, "only {ran} of 40 ran");
}

/// The local ledger (spec §3a): a small tool on a big plate is decided in a region round it, and
/// the plate's faces outside pass through; the stitched body is valid, and the faces are whole.
#[test]
fn local_region_stitches_the_plate() {
    let plate = block(Point3::ORIGIN, 200.0, 100.0, 10.0, 1);
    // a pocket in the middle: the top keeps one face, with a hole
    let pocket = block(Point3::new(90.0, 40.0, 5.0), 20.0, 20.0, 10.0, 2);
    let r = run(&plate, &pocket, Op::Subtract).unwrap();
    assert_eq!(counts(&r), (11, 24, 16));
    assert!((volume(&r) - (200000.0 - 2000.0)).abs() < 1e-6);
    // a slot clean across: the top is two faces now, through the region's walls
    let slot = block(Point3::new(90.0, -1.0, 5.0), 20.0, 102.0, 10.0, 3);
    let r = run(&plate, &slot, Op::Subtract).unwrap();
    assert_eq!(counts(&r), (10, 24, 16));
    assert!((volume(&r) - (200000.0 - 20.0 * 100.0 * 5.0)).abs() < 1e-6);
    // bosses one by one, as a history: each boolean local, the result valid and exact
    let mut part = plate.clone();
    for k in 0..12 {
        let x = 5.0 + 15.0 * k as f64;
        let boss = block(Point3::new(x, 30.0, 10.0), 10.0, 10.0, 5.0, 10 + k);
        part = run(&part, &boss, Op::Union).unwrap();
    }
    assert!((volume(&part) - (200000.0 + 12.0 * 500.0)).abs() < 1e-6);
    assert_eq!(counts(&part).0, 6 + 12 * 5);
}

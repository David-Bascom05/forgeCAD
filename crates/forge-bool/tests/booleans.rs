use forge_bool::{Op, boolean};
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_mesh::{Quality, tessellate};
use forge_ops::{Axis2, Profile, ProfileLoop, Segment, extrude, revolve};
use forge_topo::{Body, check, genus};

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

fn rod(centre: Point3, r: f64, len: f64, feature: u64) -> Body {
    let plane = Frame::WORLD.with_origin(centre);
    extrude(
        &Profile {
            plane,
            loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
        },
        len,
        feature,
    )
    .unwrap()
}

fn ball(centre: Point3, r: f64, feature: u64) -> Body {
    let q = dmath::FRAC_PI_2;
    let half = ProfileLoop::new(vec![
        (
            Segment::Arc {
                center: Point2::ORIGIN,
                radius: r,
                start: -q,
                sweep: 2.0 * q,
            },
            0,
        ),
        (
            Segment::Line {
                a: Point2::new(0.0, r),
                b: Point2::new(0.0, -r),
            },
            1,
        ),
    ]);
    // revolve about the world x axis, so a cut across x meets the ball at a latitude, away from its poles
    let plane = Frame::from_zx(centre, Vec3::Z.normalize().unwrap(), Vec3::Y).unwrap();
    revolve(
        &Profile {
            plane,
            loops: vec![half],
        },
        Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::Y,
        },
        dmath::TAU,
        feature,
    )
    .unwrap()
}

fn volume(b: &Body) -> f64 {
    let m = tessellate(
        b,
        &Quality {
            chord: 0.001,
            max_angle: 0.05,
        },
    )
    .unwrap();
    let open = m.open_edges();
    assert!(open.is_empty(), "{} open mesh edges", open.len());
    m.volume()
}

fn valid(b: &Body) {
    let p = check(b);
    assert!(p.is_empty(), "{p:?}");
}

#[test]
fn drill_a_hole() {
    let plate = block(Point3::ORIGIN, 30.0, 20.0, 6.0, 1);
    let drill = rod(Point3::new(15.0, 10.0, -1.0), 4.0, 8.0, 2);
    let r = boolean(&plate, &drill, Op::Subtract, 3).unwrap();
    valid(&r);
    assert_eq!(genus(&r), 1);
    let want = 30.0 * 20.0 * 6.0 - dmath::PI * 16.0 * 6.0;
    assert!(
        (volume(&r) - want).abs() / want < 1e-4,
        "{} vs {want}",
        volume(&r)
    );
}

#[test]
fn blind_pocket_and_boss() {
    let plate = block(Point3::ORIGIN, 30.0, 20.0, 6.0, 1);
    let pocket = rod(Point3::new(15.0, 10.0, 3.0), 4.0, 5.0, 2);
    let r = boolean(&plate, &pocket, Op::Subtract, 3).unwrap();
    valid(&r);
    assert_eq!(genus(&r), 0);
    let want = 30.0 * 20.0 * 6.0 - dmath::PI * 16.0 * 3.0;
    assert!((volume(&r) - want).abs() / want < 1e-4);
    let boss = rod(Point3::new(8.0, 8.0, 4.0), 3.0, 6.0, 4);
    let r = boolean(&plate, &boss, Op::Union, 5).unwrap();
    valid(&r);
    let want = 30.0 * 20.0 * 6.0 + dmath::PI * 9.0 * 4.0;
    assert!((volume(&r) - want).abs() / want < 1e-4);
}

#[test]
fn overlapping_blocks_all_ops() {
    let a = block(Point3::ORIGIN, 10.0, 10.0, 10.0, 1);
    let b = block(Point3::new(4.0, 5.0, 6.0), 10.0, 10.0, 10.0, 2);
    let (va, vb) = (1000.0, 1000.0);
    let overlap = 6.0 * 5.0 * 4.0;
    for (op, want) in [
        (Op::Union, va + vb - overlap),
        (Op::Subtract, va - overlap),
        (Op::Intersect, overlap),
    ] {
        let r = boolean(&a, &b, op, 3).unwrap();
        valid(&r);
        assert!(
            (volume(&r) - want).abs() < 1e-6,
            "{op:?}: {} vs {want}",
            volume(&r)
        );
    }
}

#[test]
fn ball_in_a_block() {
    let a = block(Point3::ORIGIN, 10.0, 10.0, 10.0, 1);
    let b = ball(Point3::new(10.0, 5.0, 5.0), 3.0, 2);
    let half_ball = 2.0 / 3.0 * dmath::PI * 27.0;
    let r = boolean(&a, &b, Op::Subtract, 3).unwrap();
    valid(&r);
    assert!(
        (volume(&r) - (1000.0 - half_ball)).abs() / 1000.0 < 1e-4,
        "{}",
        volume(&r)
    );
    let r = boolean(&a, &b, Op::Union, 3).unwrap();
    valid(&r);
    assert!(
        (volume(&r) - (1000.0 + half_ball)).abs() / 1000.0 < 1e-4,
        "{}",
        volume(&r)
    );
}

#[test]
fn random_rotated_blocks_keep_volume_identities() {
    let mut rng = Rng::new(1000);
    let mut ran = 0;
    for _ in 0..60 {
        let a = block(Point3::new(-5.0, -5.0, -5.0), 10.0, 10.0, 10.0, 1);
        let z = Vec3::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(-1.0, 1.0))
            .normalize()
            .unwrap();
        let o = Point3::new(rng.range(-6.0, 6.0), rng.range(-6.0, 6.0), rng.range(-6.0, 6.0));
        let plane = Frame::from_z(o, z);
        let b = extrude(
            &Profile {
                plane,
                loops: vec![Profile::rectangle(Point2::new(-3.0, -4.0), 6.0, 8.0, 0)],
            },
            7.0,
            2,
        )
        .unwrap();
        let (Ok(u), Ok(i), Ok(s)) = (
            boolean(&a, &b, Op::Union, 3),
            boolean(&a, &b, Op::Intersect, 3),
            boolean(&a, &b, Op::Subtract, 3),
        ) else {
            continue;
        };
        valid(&u);
        valid(&i);
        valid(&s);
        let (vu, vi, vs) = (volume(&u), volume(&i), volume(&s));
        let vb = 6.0 * 8.0 * 7.0;
        assert!((vu + vi - (1000.0 + vb)).abs() < 1e-6, "{vu} + {vi}");
        assert!((vs + vi - 1000.0).abs() < 1e-6, "{vs} + {vi}");
        ran += 1;
    }
    assert!(ran >= 50, "only {ran} of 60 ran");
}

#[test]
fn coplanar_boss_bottoms_over_a_pocket() {
    let plate = block(Point3::ORIGIN, 40.0, 30.0, 10.0, 1);
    let pocket = rod(Point3::new(15.0, 15.0, 4.0), 6.0, 7.0, 2);
    let a = boolean(&plate, &pocket, Op::Subtract, 3).unwrap();
    valid(&a);
    let boss1 = rod(Point3::new(20.0, 15.0, 8.7), 5.0, 11.3, 4);
    let b = boolean(&a, &boss1, Op::Union, 5).unwrap();
    valid(&b);
    let boss2 = rod(Point3::new(12.0, 15.0, 8.7), 4.0, 9.3, 6);
    let c = boolean(&b, &boss2, Op::Union, 7).unwrap();
    valid(&c);
    let v = volume(&c);
    assert!(v > volume(&b));
}

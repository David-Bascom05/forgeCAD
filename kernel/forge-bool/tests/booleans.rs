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

/// The exact volume, from the surfaces; the mesh only shows the solid is closed.
fn volume(b: &Body) -> f64 {
    let open = tessellate(b, &Quality::default()).unwrap().open_edges();
    assert!(open.is_empty(), "{} open mesh edges", open.len());
    let m = forge_topo::mass::mass(b).unwrap();
    assert!(m.err[0] <= 1e-9 * m.volume.abs(), "error bound {}", m.err[0]);
    m.volume
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
        (volume(&r) - want).abs() / want < 1e-12,
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
    assert!((volume(&r) - want).abs() / want < 1e-12);
    let boss = rod(Point3::new(8.0, 8.0, 4.0), 3.0, 6.0, 4);
    let r = boolean(&plate, &boss, Op::Union, 5).unwrap();
    valid(&r);
    let want = 30.0 * 20.0 * 6.0 + dmath::PI * 9.0 * 4.0;
    assert!((volume(&r) - want).abs() / want < 1e-12);
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
            (volume(&r) - want).abs() < 1e-10,
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
        (volume(&r) - (1000.0 - half_ball)).abs() / 1000.0 < 1e-12,
        "{}",
        volume(&r)
    );
    let r = boolean(&a, &b, Op::Union, 3).unwrap();
    valid(&r);
    assert!(
        (volume(&r) - (1000.0 + half_ball)).abs() / 1000.0 < 1e-12,
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
        assert!((vu + vi - (1000.0 + vb)).abs() < 1e-9, "{vu} + {vi}");
        assert!((vs + vi - 1000.0).abs() < 1e-9, "{vs} + {vi}");
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

#[test]
fn check_honours_vertex_tolerance() {
    let good = block(Point3::ORIGIN, 10.0, 10.0, 10.0, 1);
    let mut moved = good.clone();
    moved.vertices[0].point.x += 3e-5;
    assert!(!check(&moved).is_empty());
    moved.vertices[0].tol = 4e-5;
    valid(&moved);
    // a ball reaching halfway along an edge is no longer a tolerance
    moved.vertices[0].tol = 6.0;
    assert!(
        check(&moved)
            .iter()
            .any(|p| matches!(p, forge_topo::Problem::VertexTolerance { .. }))
    );
}

#[test]
fn offsets_just_past_tolerance_are_kept() {
    // a wall two tolerances thin, and a cut two tolerances past a face: both real
    for e in [-2e-5, 2e-5] {
        let base = block(Point3::ORIGIN, 60.0, 40.0, 10.0, 1);
        let cut = block(Point3::new(50.0 + e, 5.0, -1.0), 10.0, 8.0, 12.0, 2);
        let r = boolean(&base, &cut, Op::Subtract, 3).unwrap();
        valid(&r);
        let want = 60.0 * 40.0 * 10.0 - (10.0 - e.max(0.0)) * 8.0 * 10.0;
        assert!((volume(&r) - want).abs() < 1e-9, "{e}: {} vs {want}", volume(&r));
    }
}

fn zero_thickness(r: Result<Body, forge_bool::BoolError>) -> bool {
    matches!(r, Err(forge_bool::BoolError::ZeroThickness { .. }))
}

#[test]
fn exact_touches_are_refused() {
    let plate = block(Point3::ORIGIN, 40.0, 30.0, 10.0, 1);
    // a ball resting on the top face, added: material meets at one point
    let resting = ball(Point3::new(20.0, 15.0, 13.0), 3.0, 2);
    assert!(zero_thickness(boolean(&plate, &resting, Op::Union, 3)));
    assert!(zero_thickness(forge_bool::nary(
        &[&plate, &resting],
        &[Op::Union],
        2
    )));
    // the same ball cut away touches only air: nothing changes
    let r = boolean(&plate, &resting, Op::Subtract, 3).unwrap();
    valid(&r);
    // a ball cut from inside, touching the top: a skin of zero thickness
    let inside = ball(Point3::new(20.0, 15.0, 7.0), 3.0, 2);
    assert!(zero_thickness(boolean(&plate, &inside, Op::Subtract, 3)));
    // two holes tangent to each other: a wall of zero thickness along a line
    let first = rod(Point3::new(10.0, 15.0, -1.0), 3.0, 12.0, 2);
    let holed = boolean(&plate, &first, Op::Subtract, 3).unwrap();
    let second = rod(Point3::new(16.0, 15.0, -1.0), 3.0, 12.0, 4);
    assert!(zero_thickness(boolean(&holed, &second, Op::Subtract, 5)));
    let r = forge_bool::nary(&[&plate, &first, &second], &[Op::Subtract, Op::Subtract], 2);
    assert!(zero_thickness(r.clone()), "{:?}", r.map(|b| check(&b)));
}

/// Both bodies' shells in one body, as a bad boolean might leave them.
fn side_by_side(a: &Body, b: &Body) -> Body {
    let mut out = a.clone();
    let v: Vec<_> = b
        .vertices
        .iter()
        .map(|x| out.add_tolerant_vertex(x.point, x.tol, x.name))
        .collect();
    let e: Vec<_> = b
        .edges
        .iter()
        .map(|x| {
            out.add_edge(
                x.curve.clone(),
                x.range,
                v[x.start.index()],
                v[x.end.index()],
                x.name,
            )
        })
        .collect();
    let shell = out.add_shell();
    for f in b.face_ids() {
        let face = b.face(f);
        let g = out.add_face(shell, face.surface.clone(), face.same_sense, face.name);
        for &l in &face.loops {
            let uses: Vec<_> = b
                .lp(l)
                .coedges()
                .map(|c| (e[b.coedge(c).edge.index()], b.coedge(c).forward))
                .collect();
            out.add_loop(g, &uses);
        }
    }
    out
}

#[test]
fn overlapping_shells_are_invalid() {
    let has = |b: &Body| {
        check(b)
            .iter()
            .any(|p| matches!(p, forge_topo::Problem::ShellsOverlap { .. }))
    };
    let plate = block(Point3::new(-20.0, -20.0, -20.0), 40.0, 40.0, 20.0, 1);
    // a rod through the plate and out of it, never joined to it
    let through = rod(Point3::new(0.0, 0.0, -30.0), 5.0, 60.0, 2);
    let both = side_by_side(&plate, &through);
    assert!(has(&both), "{:?}", check(&both));
    // a block wholly inside, as a second outer skin
    let inner = block(Point3::new(-5.0, -5.0, -15.0), 10.0, 10.0, 10.0, 2);
    assert!(has(&side_by_side(&plate, &inner)));
    // apart, or a void inside: fine
    let apart = block(Point3::new(30.0, 0.0, 0.0), 10.0, 10.0, 10.0, 2);
    valid(&side_by_side(&plate, &apart));
    let void = boolean(&plate, &inner, Op::Subtract, 3).unwrap();
    assert_eq!(void.shells.len(), 2);
    valid(&void);
}

/// Equal cylinders whose axes cross meet in two ellipses crossing where the cylinders touch.
/// Their common part is the Steinmetz solid, 16 r^3 / (3 sin a) for axes at angle a.
#[test]
fn equal_cylinders_crossing() {
    let (r, len) = (3.0, 40.0);
    let rod_along = |o: Point3, dir: Vec3, feature: u64| {
        let plane = Frame::from_zx(
            o - dir * (len / 2.0),
            dir.normalize().unwrap(),
            Vec3::new(0.3, -0.2, 0.9),
        )
        .unwrap();
        extrude(
            &Profile {
                plane,
                loops: vec![Profile::circle(Point2::ORIGIN, r, 0)],
            },
            len,
            feature,
        )
        .unwrap()
    };
    let v = dmath::PI * r * r * len;
    for deg in [90.0, 60.0, 35.0, 120.0] {
        let a = deg * dmath::PI / 180.0;
        let o = Point3::new(1.0, -2.0, 0.5);
        let first = rod_along(o, Vec3::new(1.0, 0.0, 0.0), 1);
        let (s, c) = dmath::sin_cos(a);
        let second = rod_along(o, Vec3::new(c, s * 0.6, s * 0.8), 2);
        let common = 16.0 * r * r * r / (3.0 * s);
        for (op, want) in [
            (Op::Union, 2.0 * v - common),
            (Op::Subtract, v - common),
            (Op::Intersect, common),
        ] {
            for nary in [false, true] {
                let b = if nary {
                    forge_bool::nary(&[&first, &second], &[op], 1)
                } else {
                    boolean(&first, &second, op, 3)
                }
                .unwrap_or_else(|e| panic!("{deg} {op:?} nary {nary}: {e:?}"));
                valid(&b);
                let got = volume(&b);
                assert!(
                    (got - want).abs() / want < 1e-3,
                    "{deg} {op:?} nary {nary}: {got} vs {want}"
                );
            }
        }
    }
}

/// Blocks touching at a corner: the touch point keeps a vertex per fan of faces, feature by
/// feature (where the corner is one the target already had, its faces left alone) and n-ary.
#[test]
fn blocks_touching_at_a_corner() {
    let a = block(Point3::ORIGIN, 10.0, 10.0, 10.0, 1);
    let b = block(Point3::new(10.0, 10.0, 10.0), 10.0, 10.0, 10.0, 2);
    for nary in [false, true] {
        let u = if nary {
            forge_bool::nary(&[&a, &b], &[Op::Union], 1)
        } else {
            boolean(&a, &b, Op::Union, 3)
        }
        .unwrap();
        valid(&u);
        assert!((volume(&u) - 2000.0).abs() < 1e-6, "{}", volume(&u));
    }
}

/// A hole of the concave round's own radius on its centre circle, so tangent to the wall above the
/// round: the same hole wherever round the part it goes, its two lobes on the torus both found.
#[test]
fn a_hole_on_a_rounds_centre_circle_at_any_angle() {
    let arc = Segment::Arc {
        center: Point2::new(55.0, -15.0),
        radius: 1.0,
        start: -dmath::FRAC_PI_2,
        sweep: -dmath::FRAC_PI_2,
    };
    let at = |x: f64, y: f64| Point2::new(x, y);
    let seg = |a: Point2, b: Point2| Segment::Line { a, b };
    let outline = vec![
        (seg(at(0.0, -22.0), at(60.0, -22.0)), 0),
        (seg(at(60.0, -22.0), at(60.0, -16.0)), 1),
        (seg(at(60.0, -16.0), at(55.0, -16.0)), 2),
        (arc, 3),
        (seg(at(54.0, -15.0), at(54.0, 10.0)), 4),
        (seg(at(54.0, 10.0), at(0.0, 10.0)), 5),
        (seg(at(0.0, 10.0), at(0.0, -22.0)), 6),
    ];
    let plane = Frame::from_axes(Point3::ORIGIN, Vec3::X, Vec3::Z, -Vec3::Y).unwrap();
    let axis = Axis2 {
        origin: Point2::ORIGIN,
        dir: Vec2::Y,
    };
    let part = revolve(
        &Profile {
            plane,
            loops: vec![ProfileLoop::new(outline)],
        },
        axis,
        dmath::TAU,
        1,
    )
    .unwrap();
    // what the hole takes, column by column over its disc: wall above 54, then the round, then the floor
    let n = 2000;
    let mut taken = 0.0;
    for i in 0..n {
        for j in 0..n {
            let h = 2.0 / n as f64;
            let (x, y) = (-1.0 + (i as f64 + 0.5) * h, -1.0 + (j as f64 + 0.5) * h);
            if x * x + y * y > 1.0 {
                continue;
            }
            let rho = ((55.0 + x) * (55.0 + x) + y * y).sqrt();
            let top = if rho <= 54.0 {
                10.0
            } else if rho < 55.0 {
                -15.0 - (1.0 - (55.0 - rho) * (55.0 - rho)).sqrt()
            } else {
                -16.0
            };
            taken += (top + 22.0) * h * h;
        }
    }
    let whole = volume(&part);
    for k in 0..36 {
        let a = dmath::TAU * k as f64 / 36.0 + 0.168;
        let (c, s) = (dmath::cos(a), dmath::sin(a));
        let x = Vec3::new(-s, c, 0.0);
        let frame = Frame::from_axes(
            Point3::new(55.0 * c, 55.0 * s, -30.0),
            x,
            Vec3::Z.cross(x),
            Vec3::Z,
        )
        .unwrap();
        let hole = extrude(
            &Profile {
                plane: frame,
                loops: vec![Profile::circle(Point2::ORIGIN, 1.0, 0)],
            },
            50.0,
            2,
        )
        .unwrap();
        let cut = boolean(&part, &hole, Op::Subtract, 3).unwrap_or_else(|e| panic!("at {k}: {e}"));
        valid(&cut);
        assert!(
            (whole - volume(&cut) - taken).abs() < 2e-2,
            "at {k}: {} not {taken}",
            whole - volume(&cut)
        );
    }
}

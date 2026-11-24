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
    moved.grow_vertex_tol(forge_topo::VertexId(0), 4e-5);
    valid(&moved);
    // a ball reaching halfway along an edge is no longer a tolerance
    moved.grow_vertex_tol(forge_topo::VertexId(0), 6.0);
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
/// Their common part is the Steinmetz solid, 16 r^3 / (3 sin a) for axes at angle a. One less
/// the other is two pieces of rod meeting only at those two points, the two wedges between the
/// tangent surfaces where the one rod's wall lies above the other's: not a manifold there, so
/// refused as a corner touch is.
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
                };
                if op == Op::Subtract {
                    // at one of the two points the rods touch, o ± r (axis × axis)
                    let n = Vec3::new(1.0, 0.0, 0.0).cross(Vec3::new(c, s * 0.6, s * 0.8));
                    let n = n * (r / n.length());
                    match b {
                        Err(forge_bool::BoolError::ZeroThickness { at }) => assert!(
                            [o + n, o - n].iter().any(|p| p.distance(at) < 1e-6),
                            "{deg} nary {nary}: refused at {at:?}"
                        ),
                        other => panic!("{deg} nary {nary}: {:?}", other.map(|b| volume(&b))),
                    }
                    continue;
                }
                let b = b.unwrap_or_else(|e| panic!("{deg} {op:?} nary {nary}: {e:?}"));
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

/// Blocks touching at a corner: the union is not a manifold at the corner (two fans of faces
/// round one point), refused there with ZeroThickness, feature by feature (where the corner is one
/// the target already had, its faces left alone) and n-ary, as a touch along an edge is.
#[test]
fn blocks_touching_at_a_corner() {
    let a = block(Point3::ORIGIN, 10.0, 10.0, 10.0, 1);
    let b = block(Point3::new(10.0, 10.0, 10.0), 10.0, 10.0, 10.0, 2);
    for nary in [false, true] {
        let u = if nary {
            forge_bool::nary(&[&a, &b], &[Op::Union], 1)
        } else {
            boolean(&a, &b, Op::Union, 3)
        };
        match u {
            Err(forge_bool::BoolError::ZeroThickness { at }) => {
                assert!(at.distance(Point3::new(10.0, 10.0, 10.0)) < 1e-9, "{at:?}")
            }
            other => panic!("nary {nary}: {:?}", other.map(|b| volume(&b))),
        }
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

/// A refusal says what failed and where, in words an agent can act on.
#[test]
fn a_refusal_names_its_faces() {
    let a = ball(Point3::ORIGIN, 5.0, 1);
    let face = a.face_ids().next().unwrap();
    let at = forge_bool::FaceRef {
        name: a.face(face).name,
        surface: a.face(face).surface.kind(),
        near: Point3::new(1.0, 2.0, 3.0),
    };
    let e =
        forge_bool::BoolError::Unsupported(Box::new(forge_bool::Unhandled::NoClosedForm { a: at, b: at }));
    let said = e.to_string();
    assert!(
        said.contains("without a closed form") && said.contains("sphere"),
        "{said}"
    );
    assert!(
        said.contains("(1.000, 2.000, 3.000)") && said.contains(&format!("{:?}", at.name)),
        "{said}"
    );
}

/// Two bent sheets flush on one flat, their bends tangent to it where they cross: the post's bend
/// (round z) dips under the flat by a rounding, so the bends overlap in a loop far under
/// tolerance. That is a touch at a point, not a knot of hair-thin curves (it was a "face split
/// crossed itself" refusal, or zero thickness, in the rack's rear hinge bracket on its post).
#[test]
fn bends_tangent_on_one_flat_touch() {
    let p = Point2::new;
    let h = dmath::FRAC_PI_2;
    for dip in [0.0, -5.7e-14, -1e-13, -1e-12] {
        let (r, xr, y0) = (4.62, 20.5, 21.0);
        let cx = xr - r;
        let post = vec![
            (
                Segment::Line {
                    a: p(-29.5, y0),
                    b: p(cx, y0 + dip),
                },
                1,
            ),
            (
                Segment::Arc {
                    center: p(cx, y0 + r + dip),
                    radius: r,
                    start: -h,
                    sweep: h,
                },
                2,
            ),
            (
                Segment::Line {
                    a: p(xr, y0 + r + dip),
                    b: p(xr, 71.0),
                },
                3,
            ),
            (
                Segment::Line {
                    a: p(xr, 71.0),
                    b: p(-29.5, 71.0),
                },
                4,
            ),
            (
                Segment::Line {
                    a: p(-29.5, 71.0),
                    b: p(-29.5, y0),
                },
                5,
            ),
        ];
        let post = extrude(
            &Profile {
                plane: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, -29.08)),
                loops: vec![ProfileLoop { segments: post }],
            },
            906.0,
            1,
        )
        .expect("post");
        // an L in (y, z), bent round x at (17.52, 1.6): outer radius 3.48, 1.88 thick, its upright
        // leg flush on the post's flat y = 21
        let (cy, cz) = (17.52, 1.6);
        let leg = vec![
            (
                Segment::Line {
                    a: p(-13.0, -1.88),
                    b: p(cy, -1.88),
                },
                1,
            ),
            (
                Segment::Arc {
                    center: p(cy, cz),
                    radius: 3.48,
                    start: -h,
                    sweep: h,
                },
                2,
            ),
            (
                Segment::Line {
                    a: p(21.0, cz),
                    b: p(21.0, 44.92),
                },
                3,
            ),
            (
                Segment::Line {
                    a: p(21.0, 44.92),
                    b: p(19.12, 44.92),
                },
                4,
            ),
            (
                Segment::Line {
                    a: p(19.12, 44.92),
                    b: p(19.12, cz),
                },
                5,
            ),
            (
                Segment::Arc {
                    center: p(cy, cz),
                    radius: 1.6,
                    start: 0.0,
                    sweep: -h,
                },
                6,
            ),
            (
                Segment::Line {
                    a: p(cy, 0.0),
                    b: p(-13.0, 0.0),
                },
                7,
            ),
            (
                Segment::Line {
                    a: p(-13.0, 0.0),
                    b: p(-13.0, -1.88),
                },
                8,
            ),
        ];
        let yz = Frame::from_axes(Point3::new(-17.0, 0.0, 0.0), Vec3::Y, Vec3::Z, Vec3::X).expect("frame");
        let bracket = extrude(
            &Profile {
                plane: yz,
                loops: vec![ProfileLoop { segments: leg }],
            },
            34.0,
            2,
        )
        .expect("bracket");
        let common =
            boolean(&bracket, &post, Op::Intersect, 3).unwrap_or_else(|e| panic!("dip {dip}: {e:?}"));
        assert!(volume(&common).abs() < 1e-6, "dip {dip}: {}", volume(&common));
    }
}

/// B2 (research/kernel-audit/tilt-wrong-ok.md): a box and a prism under its y = 0 wall whose top
/// wall runs from y = -1.2e-5 to 1.2e-5 over the length, crossing the box's wall at x = L/2 at
/// 2.4e-5/L rad. Between the walls lies a wedge 1.2e-5 thick at x = L (1.2·LINEAR): every op is a
/// valid solid of the exact volume (once an invalid body for intersect and false ZeroThickness
/// refusals for union and subtract: the prism's bottom edge was not imprinted into the box's
/// coplanar bottom, its samples all within LINEAR of the box's edge).
#[test]
fn tilted_wedge_thinner_than_two_tolerances() {
    let prism = |pts: &[(f64, f64)], feature: u64| {
        let n = pts.len();
        let segs = (0..n)
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                (
                    Segment::Line {
                        a: Point2::new(a.0, a.1),
                        b: Point2::new(b.0, b.1),
                    },
                    feature * 10 + i as u64,
                )
            })
            .collect();
        extrude(
            &Profile {
                plane: Frame::WORLD,
                loops: vec![ProfileLoop::new(segs)],
            },
            10.0,
            feature,
        )
        .unwrap()
    };
    for l in [1e3, 1e5, 1e6, 2e6] {
        let a = prism(&[(0.0, 0.0), (l, 0.0), (l, 10.0), (0.0, 10.0)], 1);
        let b = prism(&[(0.0, -10.0), (l, -10.0), (l, 1.2e-5), (0.0, -1.2e-5)], 2);
        // the wedge: a triangle L/2 long and 1.2e-5 wide at its end, 10 high
        let wedge = 3e-5 * l;
        for (op, want) in [
            (Op::Intersect, wedge),
            (Op::Union, 200.0 * l - wedge),
            (Op::Subtract, 100.0 * l - wedge),
        ] {
            let r = boolean(&a, &b, op, 9).unwrap_or_else(|e| panic!("L {l} {op:?}: {e:?}"));
            valid(&r);
            let open = tessellate(&r, &Quality::default()).unwrap().open_edges();
            assert!(open.is_empty(), "L {l} {op:?}: {} open mesh edges", open.len());
            // (the volume with its error bound: the wedge's own is tiny beside its coordinates)
            let m = forge_topo::mass::mass(&r).unwrap();
            let slack = m.err[0] + 1e-9 * want;
            assert!(
                (m.volume - want).abs() <= slack,
                "L {l} {op:?}: {} want {want}",
                m.volume
            );
        }
    }
}

/// A chain of booleans on one plate: tilted rectangular prisms (z tilted up to `tilt`, any angle
/// in plane), nine subtracts through it then a union below, `n` of them, as in turn from seed 7.
/// The tools with ids in `only` (all if empty). Each result must be valid (names included).
fn plate_chain(n: usize, tilt: f64, only: &[u64]) {
    let block = |f: Frame, w: f64, h: f64, d: f64, id: u64| {
        extrude(
            &Profile {
                plane: f,
                loops: vec![Profile::rectangle(Point2::ORIGIN, w, h, id * 10)],
            },
            d,
            id,
        )
    };
    let mut rng = Rng::new(7);
    let mut body = block(Frame::WORLD, 200.0, 200.0, 10.0, 1).unwrap();
    for i in 0..n {
        let z = Vec3::new(rng.range(-tilt, tilt), rng.range(-tilt, tilt), 1.0)
            .normalize()
            .unwrap();
        let (s, c) = dmath::sin_cos(rng.range(0.0, dmath::TAU));
        let o = Point3::new(
            rng.range(10.0, 180.0),
            rng.range(10.0, 180.0),
            rng.range(-5.0, 2.0),
        );
        let Some(f) = Frame::from_zx(o, z, Vec3::new(c, s, 0.0)) else {
            continue;
        };
        let (w, h) = (rng.range(3.0, 30.0), rng.range(3.0, 30.0));
        let op = if i % 10 == 9 { Op::Union } else { Op::Subtract };
        let d = if op == Op::Union {
            rng.range(1.0, 8.0)
        } else {
            20.0
        };
        let id = 100 + i as u64;
        if !only.is_empty() && !only.contains(&id) {
            continue;
        }
        let tool = block(f, w, h, d, id).unwrap();
        body = boolean(&body, &tool, op, id).unwrap_or_else(|e| panic!("op {i} {op:?}: {e:?}"));
        let p = check(&body);
        assert!(p.is_empty(), "after op {i} ({op:?}): {p:?}");
    }
}

/// A face split twice, the second time by its own boundary piece that named it the first time:
/// the new region once took the face's own name (`Name::piece` keeps the first whole's high
/// half), a duplicate name in a body handed out (the plate's bottom, at the tenth tool).
#[test]
fn a_region_split_again_by_its_naming_edge_gets_its_own_name() {
    plate_chain(95, 0.05, &[118, 119, 138, 139, 144, 145, 186, 187, 188, 194]);
}

/// The chain whole: 100 booleans of tilted tools on one plate, every result valid.
#[test]
fn a_hundred_tilted_booleans_on_one_plate() {
    plate_chain(100, 0.05, &[]);
}

/// `drift_adv top`'s op `i` from its parameters `q` (each in [-1, 1]): a 3 mm slab, tilted by up
/// to 1e-7, its bottom (a union) or top (a cut) within two tolerances of a plate's top at z = 10;
/// and the 100 x 100 x 10 plate itself.
fn top_slab(i: usize, q: [f64; 6]) -> (Body, Op) {
    const L: f64 = 1e-5;
    let id = 100 + i as u64;
    let op = if i.is_multiple_of(2) {
        Op::Subtract
    } else {
        Op::Union
    };
    let (x0, y0) = (50.0 + 40.0 * q[0], 50.0 + 40.0 * q[1]);
    let (w, h) = (5.0 + 20.0 * (q[2] + 1.0), 5.0 + 20.0 * (q[3] + 1.0));
    let z = 10.0 + 2.0 * L * q[4];
    let tilt = 1e-7 * q[5];
    let n = Vec3::new(tilt, -tilt, 1.0).normalize().unwrap();
    let at = Point3::new(x0, y0, if op == Op::Union { z } else { z - 3.0 });
    let f = Frame::from_zx(at, n, Vec3::new(1.0, 0.0, 0.0)).unwrap();
    (slab(f, w, h, 3.0, id), op)
}

fn slab(f: Frame, w: f64, h: f64, d: f64, id: u64) -> Body {
    let pts = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
    let segs = (0..4)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % 4]);
            (
                Segment::Line {
                    a: Point2::new(a.0, a.1),
                    b: Point2::new(b.0, b.1),
                },
                id * 10 + i as u64,
            )
        })
        .collect();
    extrude(
        &Profile {
            plane: f,
            loops: vec![ProfileLoop::new(segs)],
        },
        d,
        id,
    )
    .unwrap()
}

/// Slabs stacked within two tolerances of a plate's top and tilted by up to 1e-7, so each one's
/// face is found on the top's surface (coplanar) and the edges of one imprint on the other: an
/// imprinted edge strays 1.29e-5 from the face it now bounds, past its own tolerance of 1e-5.
/// Its tolerance must grow to hold that (research/booleans/drift.md): with it the result was an
/// Ok that `check` finds an edge off its face in (EdgeOffFace), a wrong Ok. Four ops of a
/// hill-climbed fifty (`drift_adv top`), by their original indices.
#[test]
fn an_imprinted_edge_off_its_new_face_grows_its_tolerance() {
    let mut body = slab(Frame::WORLD, 100.0, 100.0, 10.0, 1);
    let ops: [(usize, [f64; 6]); 4] = [
        (
            7,
            [
                0.5918829918885135,
                0.30239341812934617,
                -0.3509277137226333,
                0.0393398165925507,
                0.42821974697303966,
                0.4670581994319918,
            ],
        ),
        (
            26,
            [
                0.5088582386782867,
                0.8576087878475563,
                -0.1598695157072878,
                -0.015880172925941904,
                0.00011761254002262689,
                0.388185391233117,
            ],
        ),
        (
            31,
            [
                0.9556838675228327,
                0.020294401467336298,
                1.0,
                0.329820388187806,
                -0.9624960073434354,
                0.23750013520793,
            ],
        ),
        (
            32,
            [
                0.9348006431398126,
                0.41442383150441364,
                -0.8599947362654539,
                0.7072555274370649,
                -0.21964280055656504,
                0.22248465631838577,
            ],
        ),
    ];
    for (i, q) in ops {
        let (tool, op) = top_slab(i, q);
        // a refusal is an honest answer; an Ok must be a valid solid
        if let Ok(r) = boolean(&body, &tool, op, 100 + i as u64) {
            let p = check(&r);
            assert!(p.is_empty(), "op {i} ({op:?}): {p:?}");
            body = r;
        }
    }
}

/// The same stack: an edge's end vertex where it is imprinted on the other body's face lies
/// 1.51e-5 from that face's plane; its tolerance (the ball every surface through it meets,
/// `Vertex::tol`) must grow to hold it, not stay at 1e-5. Three ops of a hill-climbed fifty.
#[test]
fn an_imprinted_edges_end_vertex_holds_its_new_face() {
    let mut body = slab(Frame::WORLD, 100.0, 100.0, 10.0, 1);
    let ops: [(usize, [f64; 6]); 3] = [
        (
            9,
            [
                -0.26507043851392975,
                0.8406934451433343,
                0.13335405654850618,
                0.31614661880392536,
                0.60960096690831,
                1.0,
            ],
        ),
        (
            27,
            [
                -0.6739908258109015,
                0.8580906948089231,
                0.36771629220373514,
                0.35843675683996346,
                0.10504478019733154,
                0.3343261878770015,
            ],
        ),
        (
            34,
            [
                -0.543396694807474,
                0.736688722718039,
                0.881245991414807,
                0.4951744296767451,
                0.7269937185748625,
                -0.7096159813835494,
            ],
        ),
    ];
    for (i, q) in ops {
        let (tool, op) = top_slab(i, q);
        let Ok(r) = boolean(&body, &tool, op, 100 + i as u64) else {
            continue;
        };
        for e in r.edge_ids() {
            let edge = r.edge(e);
            for &c in &edge.coedges {
                let f = r.coedge_face(c);
                let forge_geom::Surface::Plane(pl) = &r.face(f).surface else {
                    continue;
                };
                for v in [edge.start, edge.end] {
                    let x = r.vertex(v);
                    let d = pl.signed_distance(x.point).abs();
                    assert!(
                        d <= x.tol,
                        "op {i} ({op:?}): vertex {v:?} {d:e} from face {f:?}, tol {:e}",
                        x.tol
                    );
                }
            }
        }
        body = r;
    }
}

/// `drift_adv edge`'s op `i` from its parameters `q` (each in [-1, 1]): a cut (even ops) or a wall
/// (odd ops) whose faces end within two tolerances of a 100 x 100 x 10 plate's y = 0 wall.
fn edge_wall(i: usize, q: [f64; 6]) -> (Body, Op) {
    const L: f64 = 1e-5;
    let id = 100 + i as u64;
    let op = if i.is_multiple_of(2) {
        Op::Subtract
    } else {
        Op::Union
    };
    let (x0, x1) = (50.0 + 45.0 * q[0], 50.0 + 45.0 * q[0] + 5.0 + 40.0 * (q[1] + 1.0));
    let (e0, e1) = (2.0 * L * q[2], 2.0 * L * q[3]);
    let z0 = -1.0 + 6.0 * (q[4] + 1.0);
    let f = Frame::WORLD.with_origin(Point3::new(0.0, 0.0, z0));
    let pts = if op == Op::Subtract {
        [(x0, e0 - 3.0), (x1, e1 - 3.0), (x1, e1), (x0, e0)]
    } else {
        [(x0, -3.0), (x1, -3.0), (x1, e1), (x0, e0)]
    };
    let segs = (0..4)
        .map(|k| {
            let (a, b) = (pts[k], pts[(k + 1) % 4]);
            (
                Segment::Line {
                    a: Point2::new(a.0, a.1),
                    b: Point2::new(b.0, b.1),
                },
                id * 10 + k as u64,
            )
        })
        .collect();
    let body = extrude(
        &Profile {
            plane: f,
            loops: vec![ProfileLoop::new(segs)],
        },
        4.0 + 2.0 * (q[5] + 1.0),
        id,
    )
    .unwrap();
    (body, op)
}

/// Walls and notches within two tolerances of a plate's wall: an imprint grows an edge's end
/// vertex to 1.65e-5, past the middle of a kept edge 3e-5 long, so the edge is shorter than the
/// balls at its ends (`check`: VertexTolerance). Such a boolean is refused (TolerancesMeet), not
/// handed out; on main the same ops give an open shell and an edge off its face. Five ops of a
/// hill-climbed fifty (`drift_adv edge`), by their original indices.
#[test]
fn an_imprint_growing_a_vertex_past_its_edge_is_refused() {
    let mut body = extrude(
        &Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::ORIGIN, 100.0, 100.0, 10)],
        },
        10.0,
        1,
    )
    .unwrap();
    let ops: [(usize, [f64; 6]); 5] = [
        (
            4,
            [
                0.24043599453866538,
                1.0,
                1.0,
                0.46278583338977464,
                0.13888022155812785,
                -0.25231714000559846,
            ],
        ),
        (
            5,
            [
                -0.34260200510255745,
                -0.5008453451283781,
                0.5439194898173249,
                0.8864538098387861,
                -0.3742697749892536,
                1.0,
            ],
        ),
        (
            6,
            [
                -0.6052129218507916,
                -0.14151901731041128,
                -0.03859426551685785,
                -0.328741331270186,
                0.06324335293438621,
                -0.7961567910602003,
            ],
        ),
        (
            12,
            [
                0.1285076930502913,
                1.0,
                -0.48202232932225164,
                0.16922547450692205,
                -0.27066331050627457,
                0.36967690966676126,
            ],
        ),
        (
            15,
            [
                -0.11662935493797899,
                -0.3211166218619659,
                -1.0,
                -0.6168664278629803,
                0.1091801553503916,
                0.343770057423318,
            ],
        ),
    ];
    for (i, q) in ops {
        let (tool, op) = edge_wall(i, q);
        // a refusal is an honest answer; an Ok must be a valid solid
        if let Ok(r) = boolean(&body, &tool, op, 100 + i as u64) {
            let p = check(&r);
            assert!(p.is_empty(), "op {i} ({op:?}): {p:?}");
            body = r;
        }
    }
}

/// A notch cut across a plate's wall, its walls ending within two tolerances of the plate's face,
/// then a wall unioned against that face, its own face within two tolerances of it: the union
/// came out an open shell (four edges used once), a wrong Ok. Two ops of a hill-climbed fifty
/// (`drift_adv edge`, research/booleans/drift.md §6), by their original indices.
#[test]
fn a_wall_unioned_flush_on_a_notched_face_stays_closed() {
    let plate = extrude(
        &Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::ORIGIN, 100.0, 100.0, 10)],
        },
        10.0,
        1,
    )
    .unwrap();
    let mut body = plate;
    let ops: [(usize, [f64; 6]); 2] = [
        (
            2,
            [
                0.8234008506443795,
                0.943091713370833,
                0.9816978534888866,
                -0.9265032466161587,
                -0.3739713735018809,
                0.8838505963032673,
            ],
        ),
        (
            41,
            [
                0.6199122167988423,
                0.8434506663474743,
                -0.6076188493258957,
                -0.9263658549015378,
                -0.323243568577226,
                -0.2806420579990162,
            ],
        ),
    ];
    for (i, q) in ops {
        let (tool, op) = edge_wall(i, q);
        // a refusal is an honest answer; an Ok must be a valid solid
        if let Ok(r) = boolean(&body, &tool, op, 100 + i as u64) {
            let p = check(&r);
            assert!(p.is_empty(), "op {i} ({op:?}): {p:?}");
            // the wall stands 1.2e-5 off the plate, past the tolerance: the union holds both
            if op == Op::Union {
                let (got, want) = (volume(&r), volume(&body) + volume(&tool));
                assert!(
                    (got - want).abs() <= 1e-9 * want,
                    "op {i}: volume {got} want {want}"
                );
            }
            body = r;
        }
    }
}

/// `an_imprinted_edges_end_vertex_holds_its_new_face` through the n-ary boolean, each op its own
/// two-tool call: its imprinted ends must grow as the sequential boolean's do (`imprint_tols`).
/// Three ops of a hill-climbed fifty (`drift_adv top`, `NARY=1`), by their original indices.
#[test]
fn an_nary_imprinted_edges_end_vertex_holds_its_new_face() {
    let mut body = slab(Frame::WORLD, 100.0, 100.0, 10.0, 1);
    let ops: [(usize, [f64; 6]); 3] = [
        (
            9,
            [
                -0.26507043851392975,
                0.8406934451433343,
                0.24718198994817364,
                0.31614661880392536,
                0.60960096690831,
                1.0,
            ],
        ),
        (
            27,
            [
                -0.6739908258109015,
                0.8580906948089231,
                0.36771629220373514,
                0.35843675683996346,
                0.10504478019733154,
                0.3343261878770015,
            ],
        ),
        (
            34,
            [
                -0.46367594489168495,
                0.736688722718039,
                0.881245991414807,
                0.4951744296767451,
                0.7269937185748625,
                -0.7096159813835494,
            ],
        ),
    ];
    for (i, q) in ops {
        let (tool, op) = top_slab(i, q);
        let Ok(r) = forge_bool::nary(&[&body, &tool], &[op], 1) else {
            continue;
        };
        for e in r.edge_ids() {
            let edge = r.edge(e);
            for &c in &edge.coedges {
                let f = r.coedge_face(c);
                let forge_geom::Surface::Plane(pl) = &r.face(f).surface else {
                    continue;
                };
                for v in [edge.start, edge.end] {
                    let x = r.vertex(v);
                    let d = pl.signed_distance(x.point).abs();
                    assert!(
                        d <= x.tol,
                        "op {i} ({op:?}): vertex {v:?} {d:e} from face {f:?}, tol {:e}",
                        x.tol
                    );
                }
            }
        }
        body = r;
    }
}

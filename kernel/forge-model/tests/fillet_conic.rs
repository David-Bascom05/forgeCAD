//! Conic fillets: a rational quadratic section between the ball's feet, middle weight
//! w = rho / (1 - rho), held to closed forms. On a straight edge between planes the section is the
//! same all along: the fill per unit length is T (1 - g(w)), T the area of the triangle (foot, apex,
//! foot) and g the conic segment's share of it.

use forge_bool::{Op, boolean};
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec3};
use forge_ops::{
    FilletEdge, FilletError, FilletOptions, Profile, ProfileLoop, Radius, Segment, extrude, fillet_with,
};
use forge_topo::{Body, EdgeId};

fn block(w: f64, d: f64, h: f64) -> Body {
    let line = |a: (f64, f64), b: (f64, f64), id: u64| {
        (
            Segment::Line {
                a: Point2::new(a.0, a.1),
                b: Point2::new(b.0, b.1),
            },
            id,
        )
    };
    let p = Profile {
        plane: Frame::from_zx(Point3::ORIGIN, UnitVec3::Z, Vec3::X).expect("frame"),
        loops: vec![ProfileLoop::new(vec![
            line((0.0, 0.0), (w, 0.0), 0),
            line((w, 0.0), (w, d), 1),
            line((w, d), (0.0, d), 2),
            line((0.0, d), (0.0, 0.0), 3),
        ])],
    };
    extrude(&p, h, 1).expect("block")
}

/// The share of the triangle (P0, P1, P2) the segment between a rational quadratic of middle
/// weight w and its chord takes.
fn segment_share(w: f64) -> f64 {
    if (w - 1.0).abs() < 1e-12 {
        2.0 / 3.0
    } else if w < 1.0 {
        let s = (1.0 - w * w).sqrt();
        w * (forge_math::dmath::acos(w) - w * s) / (s * s * s)
    } else {
        let s = (w * w - 1.0).sqrt();
        w * (w * s - w.acosh()) / (s * s * s)
    }
}

/// The edges of `b` along z at (x, y).
fn vertical_at(b: &Body, x: f64, y: f64) -> EdgeId {
    use forge_geom::CurveGeom;
    b.edge_ids()
        .find(|&e| {
            let k = b.edge(e);
            let (p, q) = (k.curve.eval(k.range.lo), k.curve.eval(k.range.hi));
            (p.x - x).abs() < 1e-9
                && (p.y - y).abs() < 1e-9
                && (q.x - x).abs() < 1e-9
                && (q.y - y).abs() < 1e-9
        })
        .expect("a vertical edge there")
}

fn conic(rho: f64) -> FilletOptions {
    FilletOptions {
        conic: Some(rho),
        ..FilletOptions::default()
    }
}

fn volume(b: &Body) -> f64 {
    forge_topo::mass::mass(b).expect("mass").volume
}

#[test]
fn straight_convex_edge_takes_the_closed_form() {
    let (w, d, h, r) = (40.0, 30.0, 20.0, 5.0);
    let b = block(w, d, h);
    let e = vertical_at(&b, w, d);
    for rho in [0.25, 0.5, 0.7, 0.9] {
        let pick = [FilletEdge {
            edge: e,
            radius: Radius::Constant(r),
        }];
        let done = fillet_with(&b, &pick, &conic(rho), 2).expect("conic round");
        assert!(
            forge_topo::check(&done).is_empty(),
            "rho {rho}: {:?}",
            forge_topo::check(&done)
        );
        // at a right angle the feet stand r from the edge: the triangle's area is r^2 / 2
        let wt = rho / (1.0 - rho);
        let taken = h * 0.5 * r * r * (1.0 - segment_share(wt));
        let got = w * d * h - volume(&done);
        eprintln!(
            "rho {rho}: took {got:.12}, closed form {taken:.12}, off {:.2e}",
            got - taken
        );
        assert!(
            (got - taken).abs() <= 1e-9,
            "rho {rho}: took {got}, closed form {taken}"
        );
    }
}

#[test]
fn rho_outside_the_open_interval_is_refused() {
    let b = block(40.0, 30.0, 20.0);
    let e = vertical_at(&b, 40.0, 30.0);
    let pick = [FilletEdge {
        edge: e,
        radius: Radius::Constant(5.0),
    }];
    for rho in [0.0, 1.0, -0.2, 1.5, f64::NAN] {
        assert!(matches!(
            fillet_with(&b, &pick, &conic(rho), 2),
            Err(FilletError::BadConic { .. })
        ));
    }
}

/// Area and first moment about x = 0 (the integral of x over it) of the region bounded by the
/// conic (fa, apex, fb; w) in an (x, z) half plane and the two legs from its feet to the apex, by
/// Green's theorem over the exact rational curve (Gauss-Legendre, far past convergence).
fn fill_moments(fa: (f64, f64), apex: (f64, f64), fb: (f64, f64), w: f64) -> (f64, f64) {
    // a path's point and derivative at t
    type Path<'a> = &'a dyn Fn(f64) -> ((f64, f64), (f64, f64));
    let conic = |t: f64| {
        let (b0, b1, b2) = ((1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t) * w, t * t);
        let (d0, d1, d2) = (-2.0 * (1.0 - t), 2.0 * (1.0 - 2.0 * t) * w, 2.0 * t);
        let (d, dd) = (b0 + b1 + b2, d0 + d1 + d2);
        let (nx, nz) = (
            b0 * fa.0 + b1 * apex.0 + b2 * fb.0,
            b0 * fa.1 + b1 * apex.1 + b2 * fb.1,
        );
        let (dnx, dnz) = (
            d0 * fa.0 + d1 * apex.0 + d2 * fb.0,
            d0 * fa.1 + d1 * apex.1 + d2 * fb.1,
        );
        (
            (nx / d, nz / d),
            ((dnx * d - nx * dd) / (d * d), (dnz * d - nz * dd) / (d * d)),
        )
    };
    let line = |p: (f64, f64), q: (f64, f64)| {
        move |t: f64| {
            (
                (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t),
                (q.0 - p.0, q.1 - p.1),
            )
        }
    };
    // 1/2 (x dz - z dx) and 1/2 x^2 dz, 5-point Gauss-Legendre on 200 pieces of each path
    let g = [
        (0.0, 128.0 / 225.0),
        (0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
        (-0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
        (0.906_179_845_938_664, 0.236_926_885_056_189_08),
        (-0.906_179_845_938_664, 0.236_926_885_056_189_08),
    ];
    let (mut area, mut mom) = (0.0, 0.0);
    let (l1, l2) = (line(fb, apex), line(apex, fa));
    let paths: [Path; 3] = [&conic, &l1, &l2];
    for p in paths {
        let n = 200;
        for i in 0..n {
            let (t0, t1) = (i as f64 / n as f64, (i + 1) as f64 / n as f64);
            for (gx, gw) in g {
                let ((x, z), (dx, dz)) = p(0.5 * (t0 + t1) + 0.5 * (t1 - t0) * gx);
                let k = 0.5 * (t1 - t0) * gw;
                area += k * 0.5 * (x * dz - z * dx);
                mom += k * 0.5 * x * x * dz;
            }
        }
    }
    (area.abs(), mom.abs())
}

/// A plate with a round boss standing on it.
fn boss(r_boss: f64, h_plate: f64, h_boss: f64) -> Body {
    let plate = block(60.0, 60.0, h_plate);
    let p = Profile {
        plane: Frame::from_zx(Point3::new(0.0, 0.0, h_plate), UnitVec3::Z, Vec3::X).expect("frame"),
        loops: vec![ProfileLoop::new(vec![(
            Segment::Circle {
                center: Point2::new(30.0, 30.0),
                radius: r_boss,
                ccw: true,
            },
            0,
        )])],
    };
    let b = extrude(&p, h_boss, 3).expect("boss");
    boolean(&plate, &b, Op::Union, 4).expect("union")
}

/// The circle edge of `b` at height z.
fn ring_at(b: &Body, z: f64) -> EdgeId {
    use forge_geom::CurveGeom;
    b.edge_ids()
        .find(|&e| {
            let k = b.edge(e);
            matches!(k.curve, forge_geom::Curve::Circle(_))
                && (k.curve.eval(k.range.lo).z - z).abs() < 1e-9
                && (k.curve.eval(k.range.mid()).z - z).abs() < 1e-9
        })
        .expect("a ring there")
}

#[test]
fn ring_rounds_take_pappus() {
    let (rb, hp, hb, r) = (10.0, 8.0, 12.0, 3.0);
    let b = boss(rb, hp, hb);
    let v0 = volume(&b);
    for rho in [0.3, 0.5, 0.8] {
        let wt = rho / (1.0 - rho);
        // the root, concave: the fill stands on the plate round the wall
        let root = ring_at(&b, hp);
        let pick = [FilletEdge {
            edge: root,
            radius: Radius::Constant(r),
        }];
        let done = fillet_with(&b, &pick, &conic(rho), 5).expect("root");
        assert!(
            forge_topo::check(&done).is_empty(),
            "{:?}",
            forge_topo::check(&done)
        );
        let (_, m) = fill_moments((rb + r, hp), (rb, hp), (rb, hp + r), wt);
        let added = 2.0 * std::f64::consts::PI * m;
        let got = volume(&done) - v0;
        eprintln!(
            "root rho {rho}: added {got:.10}, Pappus {added:.10}, off {:.2e}",
            got - added
        );
        assert!(
            (got - added).abs() <= 1e-9,
            "root rho {rho}: {got} against {added}"
        );
        // the top rim, convex: taken off the boss's corner
        let top = ring_at(&b, hp + hb);
        let pick = [FilletEdge {
            edge: top,
            radius: Radius::Constant(r),
        }];
        let done = fillet_with(&b, &pick, &conic(rho), 5).expect("rim");
        assert!(
            forge_topo::check(&done).is_empty(),
            "{:?}",
            forge_topo::check(&done)
        );
        let (_, m) = fill_moments((rb - r, hp + hb), (rb, hp + hb), (rb, hp + hb - r), wt);
        let taken = 2.0 * std::f64::consts::PI * m;
        let got = v0 - volume(&done);
        eprintln!(
            "rim rho {rho}: taken {got:.10}, Pappus {taken:.10}, off {:.2e}",
            got - taken
        );
        assert!(
            (got - taken).abs() <= 1e-9,
            "rim rho {rho}: {got} against {taken}"
        );
    }
}

/// Gauss-Legendre over [0, 1] in 200 pieces of 5 points, for the smooth rational integrands below.
fn integrate(f: &dyn Fn(f64) -> f64) -> f64 {
    integrate_n(f, 200)
}

/// `integrate` in n pieces.
fn integrate_n(f: &dyn Fn(f64) -> f64, n: usize) -> f64 {
    let g = [
        (0.0, 128.0 / 225.0),
        (0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
        (-0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
        (0.906_179_845_938_664, 0.236_926_885_056_189_08),
        (-0.906_179_845_938_664, 0.236_926_885_056_189_08),
    ];
    let mut s = 0.0;
    for i in 0..n {
        let (a, b) = (i as f64 / n as f64, (i + 1) as f64 / n as f64);
        for (x, w) in g {
            s += 0.5 * (b - a) * w * f(0.5 * (a + b) + 0.5 * (b - a) * x);
        }
    }
    s
}

#[test]
fn mitred_rim_takes_the_closed_form() {
    // the block's top rim: each blend's prism, run on past the corners, taken off; where two meet,
    // what both take is counted twice. At a corner the section's width a(z) from each side face is
    // the same, so that overlap is the integral of a(z)^2 over the section's height.
    let (l, d, h, r, rho) = (40.0, 30.0, 20.0, 6.0, 0.7);
    let b = block(l, d, h);
    let top: Vec<FilletEdge> = b
        .edge_ids()
        .filter(|&e| {
            use forge_geom::CurveGeom;
            let k = b.edge(e);
            (k.curve.eval(k.range.lo).z - h).abs() < 1e-9 && (k.curve.eval(k.range.hi).z - h).abs() < 1e-9
        })
        .map(|edge| FilletEdge {
            edge,
            radius: Radius::Constant(r),
        })
        .collect();
    assert_eq!(top.len(), 4);
    let done = fillet_with(&b, &top, &conic(rho), 2).expect("mitred rim");
    assert!(
        forge_topo::check(&done).is_empty(),
        "{:?}",
        forge_topo::check(&done)
    );
    let w = rho / (1.0 - rho);
    // the section in (a, depth): foot (r, 0) on the top, apex (0, 0), foot (0, r) on the side
    let at = |t: f64| {
        let (b0, b1, b2) = ((1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t) * w, t * t);
        let (d0, d1, d2) = (-2.0 * (1.0 - t), 2.0 * (1.0 - 2.0 * t) * w, 2.0 * t);
        let (q, dq) = (b0 + b1 + b2, d0 + d1 + d2);
        let (a, z) = (b0 * r / q, b2 * r / q);
        let (da, dz) = (
            (d0 * r * q - b0 * r * dq) / (q * q),
            (d2 * r * q - b2 * r * dq) / (q * q),
        );
        (a, z, da, dz)
    };
    let area = integrate(&|t| {
        let (a, z, da, dz) = at(t);
        0.5 * (a * dz - z * da)
    })
    .abs();
    let overlap = integrate(&|t| {
        let (a, _, _, dz) = at(t);
        a * a * dz
    })
    .abs();
    let taken = 2.0 * area * (l + d) - 4.0 * overlap;
    let got = l * d * h - volume(&done);
    eprintln!(
        "mitred rim: took {got:.12}, closed form {taken:.12}, off {:.2e}",
        got - taken
    );
    assert!((got - taken).abs() <= 1e-9, "took {got}, closed form {taken}");
}

#[test]
fn an_oblique_edge_takes_the_closed_form() {
    // a block with a 60 degree wedge cut off one top edge, rounded where the wedge meets the top:
    // the faces turn by 60 degrees there, each foot r tan(30deg) from the edge
    let (l, d, h, k, r) = (40.0, 30.0, 20.0, 10.0, 4.0);
    let line = |a: (f64, f64), b: (f64, f64), id: u64| {
        (
            Segment::Line {
                a: Point2::new(a.0, a.1),
                b: Point2::new(b.0, b.1),
            },
            id,
        )
    };
    let run = k / 3f64.sqrt();
    let p = Profile {
        plane: Frame::from_zx(Point3::ORIGIN, -UnitVec3::Y, Vec3::X).expect("frame"),
        loops: vec![ProfileLoop::new(vec![
            line((0.0, 0.0), (l, 0.0), 0),
            line((l, 0.0), (l, h - k), 1),
            line((l, h - k), (l - run, h), 2),
            line((l - run, h), (0.0, h), 3),
            line((0.0, h), (0.0, 0.0), 4),
        ])],
    };
    let b = extrude(&p, d, 1).expect("wedged block");
    let v0 = volume(&b);
    let edge = |x: f64, z: f64| {
        use forge_geom::CurveGeom;
        b.edge_ids()
            .find(|&e| {
                let c = b.edge(e);
                let m = c.curve.eval(c.range.mid());
                (m.x - x).abs() < 1e-9 && (m.z - z).abs() < 1e-9
            })
            .expect("edge")
    };
    for rho in [0.3, 0.6] {
        let e = edge(l - run, h);
        let done = fillet_with(
            &b,
            &[FilletEdge {
                edge: e,
                radius: Radius::Constant(r),
            }],
            &conic(rho),
            2,
        )
        .expect("oblique");
        assert!(forge_topo::check(&done).is_empty());
        // tan 30deg = 1 / sqrt 3
        let s = r / 3f64.sqrt();
        // the triangle (foot, apex, foot): two legs s at the apex, 120 degrees between them
        let tri = 0.5 * s * s * (0.5 * 3f64.sqrt());
        let taken = d * tri * (1.0 - segment_share(rho / (1.0 - rho)));
        let got = v0 - volume(&done);
        eprintln!(
            "oblique rho {rho}: took {got:.12}, closed form {taken:.12}, off {:.2e}",
            got - taken
        );
        assert!((got - taken).abs() <= 1e-9, "rho {rho}: {got} against {taken}");
    }
}

#[test]
fn concave_step_takes_the_closed_form() {
    let (l, d, t, s, r) = (40.0, 30.0, 8.0, 12.0, 5.0);
    let line = |a: (f64, f64), b: (f64, f64), id: u64| {
        (
            Segment::Line {
                a: Point2::new(a.0, a.1),
                b: Point2::new(b.0, b.1),
            },
            id,
        )
    };
    let p = Profile {
        plane: Frame::from_zx(Point3::ORIGIN, -UnitVec3::Y, Vec3::X).expect("frame"),
        loops: vec![ProfileLoop::new(vec![
            line((0.0, 0.0), (l, 0.0), 0),
            line((l, 0.0), (l, t), 1),
            line((l, t), (s, t), 2),
            line((s, t), (s, 3.0 * t), 3),
            line((s, 3.0 * t), (0.0, 3.0 * t), 4),
            line((0.0, 3.0 * t), (0.0, 0.0), 5),
        ])],
    };
    let b = extrude(&p, d, 1).expect("step");
    let v0 = volume(&b);
    let e = {
        use forge_geom::CurveGeom;
        b.edge_ids()
            .find(|&e| {
                let c = b.edge(e);
                let m = c.curve.eval(c.range.mid());
                (m.x - s).abs() < 1e-9 && (m.z - t).abs() < 1e-9
            })
            .expect("inside edge")
    };
    for rho in [0.3, 0.5, 0.85] {
        let done = fillet_with(
            &b,
            &[FilletEdge {
                edge: e,
                radius: Radius::Constant(r),
            }],
            &conic(rho),
            2,
        )
        .expect("concave");
        assert!(forge_topo::check(&done).is_empty());
        let added = d * 0.5 * r * r * (1.0 - segment_share(rho / (1.0 - rho)));
        let got = volume(&done) - v0;
        eprintln!(
            "concave rho {rho}: added {got:.12}, closed form {added:.12}, off {:.2e}",
            got - added
        );
        assert!((got - added).abs() <= 1e-9, "rho {rho}: {got} against {added}");
    }
}

#[test]
fn a_corner_only_a_ball_closes_is_refused_at_its_vertex() {
    // three edges of a block's corner picked: the circular fillet closes them with a sphere
    let b = block(40.0, 30.0, 20.0);
    let at = |p: Point3| {
        use forge_geom::CurveGeom;
        b.edge_ids()
            .find(|&e| {
                let c = b.edge(e);
                c.curve.eval(c.range.mid()).distance(p) < 1e-9
            })
            .expect("edge")
    };
    let pick: Vec<FilletEdge> = [
        Point3::new(20.0, 0.0, 20.0),
        Point3::new(0.0, 15.0, 20.0),
        Point3::new(0.0, 0.0, 10.0),
    ]
    .into_iter()
    .map(|p| FilletEdge {
        edge: at(p),
        radius: Radius::Constant(4.0),
    })
    .collect();
    assert!(fillet_with(&b, &pick, &FilletOptions::default(), 2).is_ok());
    let got = fillet_with(&b, &pick, &conic(0.5), 2);
    assert!(matches!(got, Err(FilletError::ConicVertex { .. })), "{got:?}");
}

#[test]
fn a_section_that_changes_along_its_edge_is_refused_at_the_edge() {
    // a boss standing on a round bar: its root is a curve of no closed form, so the section
    // turns and changes along it
    let bar = {
        let p = Profile {
            plane: Frame::from_zx(Point3::new(-30.0, 0.0, 0.0), UnitVec3::X, Vec3::Y).expect("frame"),
            loops: vec![ProfileLoop::new(vec![(
                Segment::Circle {
                    center: Point2::new(0.0, 0.0),
                    radius: 20.0,
                    ccw: true,
                },
                0,
            )])],
        };
        extrude(&p, 60.0, 1).expect("bar")
    };
    let post = {
        let p = Profile {
            plane: Frame::from_zx(Point3::ORIGIN, UnitVec3::Z, Vec3::X).expect("frame"),
            loops: vec![ProfileLoop::new(vec![(
                Segment::Circle {
                    center: Point2::new(0.0, 0.0),
                    radius: 8.0,
                    ccw: true,
                },
                0,
            )])],
        };
        extrude(&p, 30.0, 2).expect("post")
    };
    let b = boolean(&bar, &post, Op::Union, 3).expect("union");
    let root = b
        .edge_ids()
        .find(|&e| matches!(b.edge(e).curve, forge_geom::Curve::Intersection(_)))
        .expect("a root of no closed form");
    let pick = [FilletEdge {
        edge: root,
        radius: Radius::Constant(2.0),
    }];
    let got = fillet_with(&b, &pick, &conic(0.5), 4);
    assert!(matches!(got, Err(FilletError::ConicEdge { .. })), "{got:?}");
    // and a radius that varies, anywhere
    let b = block(40.0, 30.0, 20.0);
    let pick = [FilletEdge {
        edge: vertical_at(&b, 40.0, 30.0),
        radius: Radius::Linear(2.0, 4.0),
    }];
    let got = fillet_with(&b, &pick, &conic(0.5), 2);
    assert!(matches!(got, Err(FilletError::ConicEdge { .. })), "{got:?}");
}

#[test]
fn the_segment_share_is_its_integral_and_continuous_through_the_parabola() {
    // the closed forms of g(w) against the segment's area integrated straight off the curve:
    // triangle (0, 0), (1, 1), (2, 0), so the chord on y = 0 and the triangle's area 1
    for w in [0.05, 0.3, 0.7, 0.999, 1.0, 1.001, 1.5, 4.0, 20.0] {
        // (a weight of 20 bends the curve tightly at its middle: many pieces)
        let area = integrate_n(
            &|t| {
                let (b0, b1, b2) = ((1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t) * w, t * t);
                let (d0, d1, d2) = (-2.0 * (1.0 - t), 2.0 * (1.0 - 2.0 * t) * w, 2.0 * t);
                let (q, dq) = (b0 + b1 + b2, d0 + d1 + d2);
                let y = b1 / q;
                let dx = ((d1 + 2.0 * d2) * q - (b1 + 2.0 * b2) * dq) / (q * q);
                // the area under the arc, above the chord
                y * dx
            },
            4000,
        );
        let g = segment_share(w);
        // near w = 1 the forms divide by |1 - w^2|^(3/2): rounding grows as that shrinks
        let d = (1.0 - w * w).abs();
        let bound = 1e-13 * (1.0 + 1.0 / (d * d.sqrt()).max(1e-300)).min(1e3);
        assert!(
            (g - area).abs() <= bound.max(1e-13),
            "w {w}: closed form {g}, integral {area}"
        );
    }
    // through w = 1 from both sides: the ellipse's and the hyperbola's forms meet the parabola's
    for e in [1e-3, 1e-4, 1e-5] {
        for w in [1.0 - e, 1.0 + e] {
            let g = segment_share(w);
            // g = 2/3 + 4/15 (w - 1) - 6/35 (w - 1)^2 + ..: both forms run into the parabola's
            // value with the same slope, to their rounding (which grows as |1 - w^2|^(-3/2))
            let near = 2.0 / 3.0 + 4.0 / 15.0 * (w - 1.0) - 6.0 / 35.0 * (w - 1.0) * (w - 1.0);
            let rounding = 1e-14 / (2.0 * e * (2.0 * e).sqrt());
            assert!(
                (g - near).abs() <= e * e * e + rounding,
                "w {w}: {g} against {near}"
            );
        }
    }
}

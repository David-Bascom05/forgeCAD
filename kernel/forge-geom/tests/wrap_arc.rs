//! `wrap::Flat::Arc`: a circle drawn in (s, h) wrapped round a cylinder, and its closed-form meets.
//! Reference values: research/sheet-metal/arc/check_arc.py.

use forge_geom::curve::ProceduralCurve;
use forge_geom::intersect::{Contact, CurveSurface, SurfaceSurface};
use forge_geom::ruled::{Rail, Ruled, line_wall, meet_ruled, wrap_wall};
use forge_geom::testing::rand_frame;
use forge_geom::wrap::{Flat, Wrap, wrap_surface};
use forge_geom::{Curve, Line, Parabola, Plane, Surface, SurfaceGeom};
use forge_math::rng::Rng;
use forge_math::{Frame, Interval, Point2, Point3, Vec3, dmath};

const UNIT: Interval = Interval { lo: 0.0, hi: 1.0 };
const UNROLL: f64 = 6.0;

fn arc(cx: f64, cy: f64, radius: f64, start: f64, sweep: f64) -> Flat {
    Flat::Arc {
        center: Point2::new(cx, cy),
        radius,
        start,
        sweep,
    }
}

fn wrap(axis: Frame, radius: f64, flat: Flat) -> Wrap {
    Wrap {
        axis,
        radius,
        unroll: UNROLL,
        flat,
    }
}

/// The wall of `flat` between radii 5 and 7 about `axis`.
fn wall(axis: Frame, flat: Flat) -> Surface {
    Ruled::new(
        Rail::Wrap(wrap(axis, 5.0, flat)),
        Rail::Wrap(wrap(axis, 7.0, flat)),
    )
    .expect("a wall")
    .surface()
}

/// (s, h) of a point about `axis`, s taken in the turn nearest `near`.
fn unwrapped(axis: &Frame, p: Point3, unroll: f64, near: f64) -> Point2 {
    let l = axis.point_to_local(p);
    let a = dmath::atan2(l.y, l.x);
    let turn = dmath::TAU * unroll;
    let s = a * unroll;
    Point2::new(s + turn * ((near - s) / turn).round(), l.z)
}

#[test]
fn a_wrapped_circle_is_an_ellipse_and_its_wall_a_right_conoid() {
    let mut r = Rng::new(1601);
    for i in 0..300 {
        let (center, radius) = (
            Point2::new(r.range(-5.0, 5.0), r.range(-5.0, 5.0)),
            r.range(0.5, 4.0),
        );
        // whole turns too
        let sweep = if i % 10 == 0 {
            dmath::TAU
        } else {
            r.range(0.2, 6.0)
        } * if r.bool() { 1.0 } else { -1.0 };
        let flat = Flat::Arc {
            center,
            radius,
            start: r.range(-3.0, 3.0),
            sweep,
        };
        let (axis, unroll) = (rand_frame(&mut r), r.range(4.0, 12.0));
        let w = Wrap {
            axis,
            radius: unroll + r.range(-2.0, 2.0),
            unroll,
            flat,
        };
        let t = r.range(0.0, 1.0);
        let (p, d1, d2) = w.at(t);
        // in (phi, h) the ellipse with semi-axes radius / unroll and radius
        let q = unwrapped(&axis, p, unroll, center.x);
        let phi = q.x / unroll;
        let e = ((phi - center.x / unroll) * unroll / radius).powi(2) + ((q.y - center.y) / radius).powi(2);
        assert!((e - 1.0).abs() < 1e-12, "{e}");
        let l = axis.point_to_local(p);
        assert!(((l.x * l.x + l.y * l.y).sqrt() - w.radius).abs() < 1e-12);
        let h = 1e-6;
        let close = |a: Vec3, b: Vec3| assert!((a - b).length() < 1e-5 * (1.0 + b.length()), "{a:?} {b:?}");
        close((w.at(t + h).0 - w.at(t - h).0) / (2.0 * h), d1);
        close((w.at(t + h).1 - w.at(t - h).1) / (2.0 * h), d2);
        // the fraction back from the flat point; boxes hold the curve; carried on less than a turn
        let fq = flat.at(t).0;
        assert!((flat.param(fq) - t).abs() < 1e-12 || sweep.abs() == dmath::TAU);
        let (bs, bh) = flat.bbox(UNIT);
        assert!(bs.contains(fq.x) && bh.contains(fq.y));
        assert!(w.bbox(UNIT).contains(p));
        let d = w.domain();
        assert!((d.hi - d.lo) * sweep.abs() <= dmath::TAU + 1e-12, "{d:?}");
        // the wall: every point, at any radius, on the same ellipse (radial rulings square to the axis)
        let s = Ruled::new(Rail::Wrap(w), Rail::Wrap(w.at_radius(w.radius + 1.5)))
            .expect("a wall")
            .surface();
        let v = r.range(-0.5, 1.5);
        let pw = s.eval(Point2::new(t, v));
        let lw = axis.point_to_local(pw);
        assert!((lw.z - l.z).abs() < 1e-12);
        let dir = Vec3::new(lw.x, lw.y, 0.0)
            .normalize()
            .expect("off the axis")
            .get();
        assert!((dir - Vec3::new(l.x, l.y, 0.0) / w.radius).length() < 1e-12);
    }
}

fn line() -> Flat {
    Flat::Line {
        a: Point2::new(-3.0, -1.0),
        b: Point2::new(4.0, 2.0),
    }
}

fn parabola() -> Flat {
    let frame = Frame::from_axes(
        Point3::new(0.2, -0.5, 0.0),
        Vec3::new(0.6, 0.8, 0.0),
        Vec3::new(-0.8, 0.6, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
    )
    .expect("a frame");
    Flat::Parabola {
        curve: Parabola { frame, focal: 0.5 },
        range: Interval::new(-3.0, 3.0),
    }
}

/// The two circles of check_arc.py, each the arc over angles -3..3.
fn c1() -> Flat {
    arc(0.5, 0.25, 2.0, -3.0, 6.0)
}

fn c2() -> Flat {
    arc(2.5, 1.0, 1.5, -3.0, 6.0)
}

/// Fractions along the wrapped curve where it crosses the wall.
fn ts(c: CurveSurface) -> Vec<f64> {
    let CurveSurface::Hits(h) = c else { panic!("{c:?}") };
    assert!(h.iter().all(|h| h.contact == Contact::Crossing));
    h.iter().map(|h| h.t).collect()
}

fn near(got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "{got:?} {want:?}");
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() < 1e-13, "{got:?} {want:?}");
    }
}

/// Two flat curves and their crossings, (t on the first, t on the second).
type Case = (Flat, Flat, [(f64, f64); 2]);

#[test]
fn arcs_meet_lines_parabolas_and_arcs_as_sympy_finds() {
    // check_arc.txt
    let cases: [Case; 3] = [
        (
            line(),
            c1(),
            [
                (0.2261951594590688, 0.024691952896568947),
                (0.747942771575414, 0.5866732002668267),
            ],
        ),
        (
            parabola(),
            c1(),
            [
                (0.2191702871190343, 0.4470007611115635),
                (0.8187826533793707, 0.8243074455759417),
            ],
        ),
        (
            c1(),
            c2(),
            [
                (0.4365492248538991, 0.22226591043020733),
                (0.683040998569625, 0.8973243129933167),
            ],
        ),
    ];
    let mut r = Rng::new(1602);
    for (a, b, want) in cases {
        let axis = rand_frame(&mut r);
        let mut by_b = want.map(|w| w.1);
        by_b.sort_by(f64::total_cmp);
        // either curve against the other's wall, at a radius among the wall's
        near(
            &ts(wrap_wall(&wrap(axis, 6.0, a), UNIT, &wall(axis, b)).expect("closed form")),
            &want.map(|w| w.0),
        );
        near(
            &ts(wrap_wall(&wrap(axis, 6.0, b), UNIT, &wall(axis, a)).expect("closed form")),
            &by_b,
        );
        // two walls meet in the rulings through those points
        let Some(SurfaceSurface::Curves(cs)) = meet_ruled(&wall(axis, a), &wall(axis, b)) else {
            panic!("rulings")
        };
        assert_eq!(cs.len(), 2);
        for ((c, contact), w) in cs.iter().zip(want) {
            assert_eq!(*contact, Contact::Crossing);
            let Curve::Line(l) = c else { panic!("{c:?}") };
            let p = wrap(axis, 6.0, b).at(w.1).0;
            assert!((p - l.origin).cross(l.dir.get()).length() < 1e-12);
        }
    }
}

#[test]
fn arcs_meet_planes_square_to_and_along_the_axis_as_sympy_finds() {
    let mut r = Rng::new(1603);
    let axis = rand_frame(&mut r);
    let w = wrap(axis, 6.0, c1());
    // h = 1
    let level = Surface::Plane(Plane::new(
        axis.with_origin(axis.point_to_world(Point3::new(0.0, 0.0, 1.0))),
    ));
    near(
        &ts(wrap_surface(&w, UNIT, &level).expect("closed form")),
        &[0.5640661290826066, 0.9595326465156924],
    );
    // s = -1: the plane through the axis at that turn (its other half is a half turn away)
    let (sn, cs) = dmath::sin_cos(-1.0 / UNROLL);
    let along = Frame::from_zx(
        axis.origin,
        axis.vec_to_world(Vec3::new(-sn, cs, 0.0)).normalize().unwrap(),
        axis.z().get(),
    )
    .expect("a frame");
    near(
        &ts(wrap_surface(&w, UNIT, &Surface::Plane(Plane::new(along))).expect("closed form")),
        &[0.09685693237060373, 0.9031430676293962],
    );
    // a line along the axis there meets the arc's wall at those turns' heights
    let l = Line {
        origin: axis.point_to_world(Point3::new(6.0 * cs, 6.0 * sn, -10.0)),
        dir: axis.z(),
    };
    let heights = [0.09685693237060373, 0.9031430676293962].map(|t| c1().at(t).0.y + 10.0);
    let mut by_h = heights;
    by_h.sort_by(f64::total_cmp);
    near(
        &ts(line_wall(&l, Interval::new(0.0, 20.0), &wall(axis, c1())).expect("closed form")),
        &by_h,
    );
}

/// The implicit value of a flat curve at a point.
fn implicit(f: &Flat, q: Point2) -> f64 {
    f.meet_poly([[q.x, 0.0, 0.0], [q.y, 0.0, 0.0]])[0]
}

fn random_flat(r: &mut Rng) -> Flat {
    match r.below(3) {
        0 => Flat::Line {
            a: Point2::new(r.range(-4.0, 0.0), r.range(-3.0, 3.0)),
            b: Point2::new(r.range(0.0, 4.0), r.range(-3.0, 3.0)),
        },
        1 => {
            let lo = r.range(-2.0, 0.0);
            Flat::Parabola {
                curve: Parabola {
                    frame: Frame::WORLD
                        .rotated_about_z(r.range(0.0, 6.0))
                        .with_origin(Point3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), 0.0)),
                    focal: r.range(0.3, 3.0),
                },
                range: Interval::new(lo, lo + r.range(1.0, 3.0)),
            }
        }
        _ => arc(
            r.range(-2.0, 2.0),
            r.range(-2.0, 2.0),
            r.range(0.5, 3.0),
            r.range(-3.0, 3.0),
            r.range(0.5, 6.2) * if r.bool() { 1.0 } else { -1.0 },
        ),
    }
}

/// Random pairs with an arc on one side or both: every hit is on the wall, and there are as many as
/// the wall's flat implicit changes sign along the flat curve where the wall is.
#[test]
fn arc_meets_hold_against_sampling() {
    let mut r = Rng::new(1604);
    let mut seen = 0;
    for _ in 0..1000 {
        let (a, b) = (random_flat(&mut r), random_flat(&mut r));
        if a.poly().is_some() && b.poly().is_some() {
            continue;
        }
        let axis = rand_frame(&mut r);
        let (w, s) = (wrap(axis, 6.0, a), wall(axis, b));
        let hits = match wrap_wall(&w, UNIT, &s).expect("closed form") {
            CurveSurface::Hits(h) => h,
            o => panic!("{o:?}"),
        };
        for h in &hits {
            assert!(s.distance(h.point) < 1e-9, "{}", s.distance(h.point));
            assert!(w.at(h.t).0.distance(h.point) < 1e-12);
        }
        let n = 4000;
        let ext = Interval::new(-b.reach(), 1.0 + b.reach());
        // where on the wall's curve (at any turn) the curve's point at t is
        let us = |t: f64| {
            let u = b.param(a.at(t).0);
            let turn = match b {
                Flat::Arc { sweep, .. } => dmath::TAU / sweep.abs(),
                _ => f64::INFINITY,
            };
            [u, u - turn, u + turn]
        };
        let on = |t: f64| us(t).iter().any(|&u| ext.contains(u));
        let edge = |t: f64| {
            t < 1e-3
                || t > 1.0 - 1e-3
                || us(t)
                    .iter()
                    .any(|&u| (u - ext.lo).abs().min((u - ext.hi).abs()) < 1e-4)
        };
        let v: Vec<f64> = (0..=n)
            .map(|i| implicit(&b, a.at(i as f64 / n as f64).0))
            .collect();
        let changes: Vec<f64> = (0..n)
            .filter(|&i| v[i].signum() != v[i + 1].signum())
            .map(|i| (i as f64 + 0.5) / n as f64)
            .collect();
        // keep the pairs with nothing near a range's end, or touching
        if changes.iter().chain(hits.iter().map(|h| &h.t)).any(|&t| edge(t))
            || hits.iter().any(|h| h.contact != Contact::Crossing)
        {
            continue;
        }
        assert_eq!(
            changes.iter().filter(|&&t| on(t)).count(),
            hits.len(),
            "{a:?} {b:?} {hits:?}"
        );
        seen += hits.len();
    }
    assert!(seen > 200, "{seen}");
}

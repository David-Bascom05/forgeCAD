//! `wrap::Flat::Bezier`: a spline drawn in (s, h), cut into its rational Bezier pieces, each
//! wrapped round a cylinder (a spline cut across a sheet metal bend). Levels in closed form; the
//! wall meets that need an implicit refuse it; a coaxial cylinder still cuts its wall exactly.

use forge_geom::curve::ProceduralCurve;
use forge_geom::intersect::{Contact, CurveSurface, SurfaceSurface};
use forge_geom::ruled::{Rail, Ruled, line_wall, meet_ruled, wrap_wall};
use forge_geom::spline::Spline;
use forge_geom::testing::rand_frame;
use forge_geom::wrap::{Flat, Wrap, wrap_surface};
use forge_geom::{CurveGeom, Cylinder, Line, Plane, Surface, SurfaceGeom};
use forge_math::rng::Rng;
use forge_math::{Frame, Interval, Point2, Point3, Vec3, dmath};

const UNIT: Interval = Interval { lo: 0.0, hi: 1.0 };
const UNROLL: f64 = 6.0;

/// A cubic through five points in a 12 by 8 window of (s, h), or a rational ellipse arc.
fn random_spline(r: &mut Rng) -> Spline {
    if r.below(3) == 0 {
        let c = Point3::new(6.0, 0.0, 0.0);
        let (x, y) = (
            Vec3::new(r.range(3.0, 5.0), 0.0, 0.0),
            Vec3::new(0.0, r.range(1.5, 3.0), 0.0),
        );
        return Spline::ellipse(c, x, y, r.range(0.0, 3.0), r.range(1.0, 5.0)).expect("an arc");
    }
    let pts: Vec<Point3> = (0..5)
        .map(|i| Point3::new(2.5 * i as f64 + r.range(-0.8, 0.8), r.range(-4.0, 4.0), 0.0))
        .collect();
    Spline::through(&pts, None, None).expect("distinct points")
}

fn pieces(s: &Spline) -> Vec<Flat> {
    s.bezier_pieces(s.domain())
        .iter()
        .map(|h| Flat::bezier(h).expect("degree 3 at most"))
        .collect()
}

fn wrap(axis: Frame, radius: f64, flat: Flat) -> Wrap {
    Wrap {
        axis,
        radius,
        unroll: UNROLL,
        flat,
    }
}

fn wall(axis: Frame, flat: Flat) -> Surface {
    Ruled::new(
        Rail::Wrap(wrap(axis, 5.0, flat)),
        Rail::Wrap(wrap(axis, 7.0, flat)),
    )
    .expect("a wall")
    .surface()
}

#[test]
fn pieces_are_the_spline() {
    let mut r = Rng::new(7101);
    for _ in 0..60 {
        let s = random_spline(&mut r);
        let breaks: Vec<f64> = {
            let d = s.domain();
            let mut b = vec![d.lo];
            b.extend(s.knots_in(d));
            b.push(d.hi);
            b.dedup();
            b
        };
        let ps = pieces(&s);
        assert_eq!(ps.len(), breaks.len() - 1);
        for (k, f) in ps.iter().enumerate() {
            for i in 0..=10 {
                let u = i as f64 / 10.0;
                let t = breaks[k] + (breaks[k + 1] - breaks[k]) * u;
                let want = s.point(t);
                let got = f.at(u).0;
                assert!(
                    Point2::new(want.x, want.y).distance(got) < 1e-12,
                    "{}",
                    Point2::new(want.x, want.y).distance(got)
                );
            }
        }
    }
}

#[test]
fn lies_on_its_cylinder_with_true_derivatives_and_honest_bounds() {
    let mut r = Rng::new(7102);
    for _ in 0..40 {
        let s = random_spline(&mut r);
        let axis = rand_frame(&mut r);
        for f in pieces(&s) {
            let w = wrap(axis, r.range(3.0, 9.0), f);
            let b = w.bbox(UNIT);
            let speed = w.speed_bound(UNIT);
            for i in 0..=40 {
                let t = i as f64 / 40.0;
                let (p, d1, d2) = w.at(t);
                let l = axis.point_to_local(p);
                assert!(((l.x * l.x + l.y * l.y).sqrt() - w.radius).abs() < 1e-12 * (1.0 + w.radius));
                assert!(b.contains(p), "{t}");
                assert!(d1.length() <= speed, "{} > {speed}", d1.length());
                if (0.01..=0.99).contains(&t) {
                    let h = 1e-6;
                    let (pa, da, _) = w.at(t - h);
                    let (pb, db, _) = w.at(t + h);
                    assert!(((pb - pa) * (0.5 / h) - d1).length() < 1e-5 * (1.0 + d1.length()));
                    assert!(((db - da) * (0.5 / h) - d2).length() < 1e-4 * (1.0 + d2.length()));
                }
            }
        }
    }
}

#[test]
fn levels_are_where_the_coordinate_has_the_value() {
    let mut r = Rng::new(7103);
    let mut seen = 0;
    for _ in 0..150 {
        let s = random_spline(&mut r);
        for f in pieces(&s) {
            let c = r.below(2);
            let v = if c == 0 {
                r.range(0.5, 9.5)
            } else {
                r.range(-3.0, 3.0)
            };
            let w = wrap(Frame::WORLD, 6.0, f);
            // a plane square to the axis at height v, or through it at turn v / unroll
            let plane = if c == 1 {
                Plane::new(Frame::WORLD.with_origin(Point3::new(0.0, 0.0, v)))
            } else {
                let a = v / UNROLL;
                let n = Vec3::new(-dmath::sin(a), dmath::cos(a), 0.0);
                Plane::new(
                    Frame::from_zx(Point3::ORIGIN, n.normalize().expect("unit"), Vec3::Z).expect("a frame"),
                )
            };
            let Some(CurveSurface::Hits(hits)) = wrap_surface(&w, UNIT, &Surface::Plane(plane)) else {
                panic!("a plane square to the axis or along it");
            };
            let coord = |t: f64| {
                let q = f.at(t).0;
                if c == 0 { q.x } else { q.y }
            };
            for h in &hits {
                assert!((coord(h.t) - v).abs() < 1e-9, "{}", coord(h.t) - v);
                assert!((w.at(h.t).0 - h.point).length() < 1e-12);
            }
            let n = 4000;
            let changes = (0..n)
                .filter(|&i| {
                    ((coord(i as f64 / n as f64) - v) > 0.0) != ((coord((i + 1) as f64 / n as f64) - v) > 0.0)
                })
                .count();
            let crossing = hits.iter().filter(|h| h.contact == Contact::Crossing).count();
            assert_eq!(crossing, changes);
            seen += hits.len();
        }
    }
    assert!(seen > 100, "{seen}");
}

#[test]
fn its_wall_maps_points_back_to_their_parameters() {
    let mut r = Rng::new(7104);
    for _ in 0..30 {
        let s = random_spline(&mut r);
        let axis = rand_frame(&mut r);
        for f in pieces(&s) {
            let wall = wall(axis, f);
            for _ in 0..10 {
                let uv = Point2::new(r.range(0.05, 0.95), r.range(0.05, 0.95));
                let p = wall.eval(uv);
                let back = wall.eval(wall.project(p));
                assert!(back.distance(p) < 1e-9, "{}", back.distance(p));
            }
        }
    }
}

#[test]
fn wall_meets_refuse_it_but_a_coaxial_cylinder_cuts_it() {
    let mut r = Rng::new(7105);
    let s = random_spline(&mut r);
    let f = pieces(&s)[0];
    let line = Flat::Line {
        a: Point2::new(1.0, -3.0),
        b: Point2::new(9.0, 3.0),
    };
    let axis = Frame::WORLD;
    assert!(wrap_wall(&wrap(axis, 6.0, f), UNIT, &wall(axis, line)).is_none());
    assert!(wrap_wall(&wrap(axis, 6.0, line), UNIT, &wall(axis, f)).is_none());
    assert!(meet_ruled(&wall(axis, f), &wall(axis, line)).is_none());
    let along = Line::through(Point3::new(6.0, 0.0, -5.0), Point3::new(6.0, 0.0, 5.0)).expect("a line");
    assert!(line_wall(&along, Interval::new(0.0, 10.0), &wall(axis, f)).is_none());
    // the cylinder cuts the wall in the piece wrapped at its radius
    let cyl = Surface::Cylinder(Cylinder {
        frame: axis,
        radius: 6.0,
    });
    let Some(SurfaceSurface::Curves(cs)) = meet_ruled(&wall(axis, f), &cyl) else {
        panic!("the wrapped piece");
    };
    for i in 0..=20 {
        let p = cs[0].0.eval(i as f64 / 20.0);
        assert!(cyl.distance(p) < 1e-12 && wall(axis, f).distance(p) < 1e-9);
    }
    // degree 4 and a non-positive weight have no piece
    assert!(Flat::bezier(&[[0.0, 0.0, 0.0, 1.0]; 5]).is_none());
    assert!(Flat::bezier(&[[0.0, 0.0, 0.0, 1.0], [1.0, 0.0, 0.0, 0.0]]).is_none());
}

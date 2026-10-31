//! Where a ruled wall between two copies of one curve meets a surface in closed form: walls
//! between a parabola and its translate (a parabolic cylinder) cut by a plane of the parabola's
//! own direction, in the parabola moved along the rulings; walls between one wrapped curve at
//! two radii cut by a cylinder about the same axis, in the curve wrapped at that radius.

use super::{Rail, Ruled};
use crate::curve::CurveGeom;
use crate::curve::Parabola;
use crate::intersect::{Contact, CurveHit, CurveSurface, SurfaceSurface};
use crate::surface::Surface;
use forge_math::tol;
use forge_math::{Interval, poly};

/// The closed form for such a wall and a plane or cylinder, in either order; None otherwise.
pub fn meet_ruled(a: &Surface, b: &Surface) -> Option<SurfaceSurface> {
    match (ruled(a), ruled(b)) {
        (Some(r), None) => r.meet(b),
        (None, Some(r)) => r.meet(a),
        _ => None,
    }
}

fn ruled(s: &Surface) -> Option<&Ruled> {
    match s {
        Surface::Procedural(p) => p.as_any().and_then(|x| x.downcast_ref::<Ruled>()),
        _ => None,
    }
}

impl Ruled {
    fn meet(&self, other: &Surface) -> Option<SurfaceSurface> {
        let one = |c: crate::Curve| Some(SurfaceSurface::Curves(vec![(c, Contact::Crossing)]));
        match (self.a, self.b, other) {
            (
                Rail::Parabola { curve: ca, range: ra },
                Rail::Parabola { curve: cb, range: rb },
                Surface::Plane(p),
            ) => {
                let (fa, fb) = (&ca.frame, &cb.frame);
                let same = ra == rb
                    && ca.focal == cb.focal
                    && (fa.x().get() - fb.x().get()).length() <= tol::ANGULAR
                    && (fa.y().get() - fb.y().get()).length() <= tol::ANGULAR;
                let n = p.frame.z();
                if !same || n.cross(fa.z().get()).length() > tol::ANGULAR {
                    return None;
                }
                let d = fb.origin - fa.origin;
                let along = n.dot(d);
                if along.abs() <= tol::LINEAR {
                    return None;
                }
                let s = n.dot(p.frame.origin - fa.origin) / along;
                one(crate::Curve::Parabola(Parabola {
                    frame: fa.with_origin(fa.origin + d * s),
                    focal: ca.focal,
                }))
            }
            (Rail::Wrap(wa), Rail::Wrap(wb), Surface::Cylinder(c)) => {
                let (fa, fb) = (&wa.axis, &wb.axis);
                let z = fa.z();
                let off = c.frame.origin - fa.origin;
                let same = wa.flat == wb.flat
                    && wa.unroll == wb.unroll
                    && fa.origin.distance(fb.origin) <= tol::LINEAR
                    && (fa.x().get() - fb.x().get()).length() <= tol::ANGULAR
                    && (z.get() - fb.z().get()).length() <= tol::ANGULAR;
                if !same
                    || c.frame.z().cross(z.get()).length() > tol::ANGULAR
                    || (off - z * z.dot(off)).length() > tol::LINEAR
                {
                    return None;
                }
                one(wa.at_radius(c.radius).curve())
            }
            _ => None,
        }
    }
}

/// Where a parabola in a plane parallel to an extruded parabola wall's crosses the wall: in that
/// plane the wall is its parabola moved along the rulings, and two coplanar parabolas meet where
/// a quartic vanishes. None for any other curve and surface.
pub fn parabola_wall(par: &Parabola, range: Interval, s: &Surface) -> Option<CurveSurface> {
    let r = ruled(s).filter(|r| r.extruded)?;
    let (Rail::Parabola { curve: ca, range: ra }, Rail::Parabola { curve: cb, .. }) = (r.a, r.b) else {
        return None;
    };
    let fa = &ca.frame;
    let n = fa.z();
    if !(range.lo.is_finite() && range.hi.is_finite()) || n.cross(par.frame.z().get()).length() > 1e-12 {
        return None;
    }
    let d = cb.frame.origin - fa.origin;
    // the level along the rulings, which the carried-on wall must reach
    let level = n.dot(par.frame.origin - fa.origin) / n.dot(d);
    if !r.ext.v.contains(level) {
        return Some(CurveSurface::Hits(vec![]));
    }
    // the curve in the wall's parabola's frame, moved down to its plane: o + kp t^2 x + t y
    let o = fa.point_to_local(par.frame.origin - d * level);
    let (x, y) = (
        fa.vec_to_local(par.frame.x().get()),
        fa.vec_to_local(par.frame.y().get()),
    );
    let (ka, kp) = (1.0 / (4.0 * ca.focal), 1.0 / (4.0 * par.focal));
    // X(t) - ka Y(t)^2 with X = o.x + kp x.x t^2 + y.x t, Y = o.y + kp x.y t^2 + y.y t
    let (x0, x1, x2) = (o.x, y.x, kp * x.x);
    let (y0, y1, y2) = (o.y, y.y, kp * x.y);
    let quartic = [
        x0 - ka * y0 * y0,
        x1 - ka * 2.0 * y0 * y1,
        x2 - ka * (y1 * y1 + 2.0 * y0 * y2),
        -ka * 2.0 * y1 * y2,
        -ka * y2 * y2,
    ];
    let scale = quartic.iter().fold(0.0f64, |m, c| m.max(c.abs()));
    if scale <= 1e-12 * (o.x.abs() + o.y.abs() + 1.0) {
        return Some(CurveSurface::OnSurface);
    }
    let w = ra.width();
    let along = Interval::new(ra.lo + w * r.ext.u.lo, ra.lo + w * r.ext.u.hi);
    let curve = crate::Curve::Parabola(*par);
    let mut hits = Vec::new();
    for root in poly::real_roots(&quartic, range.lo, range.hi) {
        let t = root.x;
        if !along.contains(y0 + y1 * t + y2 * t * t) {
            continue;
        }
        let contact = if root.touching {
            Contact::Touching
        } else {
            Contact::Crossing
        };
        hits.push(CurveHit {
            t,
            point: curve.eval(t),
            contact,
        });
    }
    Some(CurveSurface::Hits(hits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Curve;
    use crate::intersect::curve_surface;
    use crate::surface::SurfaceGeom;
    use crate::testing::rand_frame;
    use forge_math::rng::Rng;
    use forge_math::{Frame, Point2, Point3};

    /// A parabolic cylinder (a parabola raised `h` square to its plane) and a parabola in a
    /// parallel plane through its middle.
    fn pair(r: &mut Rng) -> (Ruled, Parabola) {
        let f = rand_frame(r);
        let ca = Parabola {
            frame: f,
            focal: r.range(0.2, 3.0),
        };
        let h = r.range(0.5, 3.0);
        let cb = Parabola {
            frame: f.with_origin(f.origin + f.z() * h),
            ..ca
        };
        let range = Interval::new(r.range(-4.0, -1.0), r.range(1.0, 4.0));
        let wall = Ruled::new(
            Rail::Parabola { curve: ca, range },
            Rail::Parabola { curve: cb, range },
        )
        .expect("a wall");
        let g = f.compose(
            &Frame::WORLD
                .rotated_about_z(r.range(0.0, 6.0))
                .with_origin(Point3::new(
                    r.range(-2.0, 2.0),
                    r.range(-2.0, 2.0),
                    r.range(0.1, 0.9) * h,
                )),
        );
        (
            wall,
            Parabola {
                frame: g,
                focal: r.range(0.2, 3.0),
            },
        )
    }

    /// The quartic's crossings are on both curves, and as many as the sign changes along it.
    #[test]
    fn parabolas_meet_parabolic_walls_in_closed_form() {
        let mut r = Rng::new(919);
        let mut seen = 0;
        for _ in 0..200 {
            let (wall, par) = pair(&mut r);
            assert!(wall.extruded);
            let s = wall.clone().surface();
            let range = Interval::new(-3.0, 3.0);
            let Some(CurveSurface::Hits(hits)) = parabola_wall(&par, range, &s) else {
                panic!("a closed form")
            };
            for h in &hits {
                assert!(s.implicit(h.point).abs() < 1e-8, "{}", s.implicit(h.point));
                assert!(Curve::Parabola(par).eval(h.t).distance(h.point) < 1e-12);
            }
            // where the face is (u inside its rail's range), sign changes of the distance
            // count the crossings
            let n = 4000;
            let on_face = |t: f64| (0.02..=0.98).contains(&s.project(Curve::Parabola(par).eval(t)).x);
            let v: Vec<f64> = (0..=n)
                .map(|i| s.implicit(Curve::Parabola(par).eval(-3.0 + 6.0 * i as f64 / n as f64)))
                .collect();
            let changes = (0..n)
                .filter(|&i| {
                    v[i].signum() != v[i + 1].signum() && on_face(-3.0 + 6.0 * (i as f64 + 0.5) / n as f64)
                })
                .count();
            let inside = hits.iter().filter(|h| on_face(h.t)).count();
            assert_eq!(changes, inside);
            seen += inside;
        }
        assert!(seen > 50, "{seen}");
    }

    /// A parabola meets planes, cylinders and spheres where a quartic vanishes: every hit on both,
    /// as many as the sign changes along it.
    #[test]
    fn parabolas_meet_quadrics_in_closed_form() {
        let mut r = Rng::new(921);
        for _ in 0..300 {
            let par = Parabola {
                frame: rand_frame(&mut r),
                focal: r.range(0.2, 3.0),
            };
            let g = rand_frame(&mut r);
            let s = match r.below(3) {
                0 => Surface::Plane(crate::Plane::new(g)),
                1 => Surface::Cylinder(crate::Cylinder {
                    frame: g,
                    radius: r.range(1.0, 6.0),
                }),
                _ => Surface::Sphere(crate::Sphere {
                    frame: g,
                    radius: r.range(1.0, 6.0),
                }),
            };
            let range = Interval::new(-5.0, 5.0);
            let CurveSurface::Hits(hits) = curve_surface(&Curve::Parabola(par), range, &s) else {
                continue;
            };
            for h in &hits {
                assert!(s.distance(h.point) < 1e-8, "{}", s.distance(h.point));
            }
            let n = 4000;
            let v: Vec<f64> = (0..=n)
                .map(|i| s.implicit(Curve::Parabola(par).eval(-5.0 + 10.0 * i as f64 / n as f64)))
                .collect();
            let changes = (0..n).filter(|&i| v[i].signum() != v[i + 1].signum()).count();
            let crossing = hits.iter().filter(|h| h.contact == Contact::Crossing).count();
            assert_eq!(changes, crossing);
        }
    }

    /// The closed-form nearest point on a parabolic cylinder is as near as any.
    #[test]
    fn parabolic_walls_project_exactly() {
        let mut r = Rng::new(920);
        for _ in 0..200 {
            let (wall, _) = pair(&mut r);
            let s = wall.clone().surface();
            let uv = Point2::new(r.range(0.0, 1.0), r.range(0.0, 1.0));
            let d = wall.point(uv);
            let n = d.du.cross(d.dv).normalize().expect("regular");
            let q = d.p + n * r.range(-0.3, 0.3);
            let got = s.eval(s.project(q)).distance(q);
            let best = (0..=400)
                .flat_map(|i| (0..=20).map(move |j| (i, j)))
                .map(|(i, j)| {
                    wall.point(Point2::new(i as f64 / 400.0, j as f64 / 20.0))
                        .p
                        .distance(q)
                })
                .fold(f64::INFINITY, f64::min);
            assert!(got <= best + 1e-9, "{got} {best}");
        }
    }
}

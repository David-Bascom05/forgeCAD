//! Where a ruled wall between two copies of one curve meets a surface in closed form: walls
//! between a parabola and its translate (a parabolic cylinder) cut by a plane of the parabola's
//! own direction, in the parabola moved along the rulings; walls between one wrapped curve at
//! two radii cut by a cylinder about the same axis, in the curve wrapped at that radius.

use super::{Rail, Ruled};
use crate::curve::{CurveGeom, Line, Parabola};
use crate::intersect::{Contact, CurveHit, CurveSurface, SurfaceSurface};
use crate::surface::Surface;
use crate::wrap::{Flat, Wrap, roots};
use forge_math::tol;
use forge_math::{Interval, Vec2, poly};

/// The closed form for such a wall and a plane or cylinder, in either order, or two wrapped
/// walls about one axis; None otherwise.
pub fn meet_ruled(a: &Surface, b: &Surface) -> Option<SurfaceSurface> {
    match (ruled(a), ruled(b)) {
        (Some(r), None) => r.meet(b),
        (None, Some(r)) => r.meet(a),
        (Some(r), Some(q)) => r.meet_wall(q),
        _ => None,
    }
}

/// A wall between one wrapped curve at two radii: the curve, and the radii its carried-on
/// rulings span.
fn radial(r: &Ruled) -> Option<(Wrap, Interval)> {
    let (&Rail::Wrap(wa), &Rail::Wrap(wb)) = (&r.a, &r.b) else {
        return None;
    };
    // a spline piece has no implicit for `crossings`: its walls go to the general meet
    if matches!(wa.flat, Flat::Bezier { .. })
        || wa.flat != wb.flat
        || wa.unroll != wb.unroll
        || !wa.coaxial(&wb.axis)
    {
        return None;
    }
    let (ra, rb) = (wa.radius, wb.radius);
    let (lo, hi) = (ra + (rb - ra) * r.ext.v.lo, ra + (rb - ra) * r.ext.v.hi);
    Some((wa, Interval::new(lo.min(hi), lo.max(hi))))
}

/// Whether a polynomial is zero throughout, against the size of its points.
fn vanishes(p: &[f64], size: f64) -> bool {
    p.iter().fold(0.0f64, |m, c| m.max(c.abs())) <= 1e-12 * (size + 1.0)
}

/// Where flat curve `c` over `range` crosses flat curve `d` over `on`: c's fractions, and whether
/// the curves touch there; None where the two are one curve (against `size`). A polynomial curve's
/// coordinates go into the other's implicit, a quartic at most, and an arc's fraction comes from
/// the point; two circles meet on their radical line (research/sheet-metal/arc/).
fn crossings(c: &Flat, range: Interval, d: &Flat, on: Interval, size: f64) -> Option<Vec<(f64, bool)>> {
    if let Some(g) = c.poly() {
        let p = d.meet_poly(g);
        if vanishes(&p, size) {
            return None;
        }
        let found = roots(&p, range).into_iter();
        return Some(
            found
                .filter(|r| d.param_in(c.at(r.x).0, on).is_some())
                .map(|r| (r.x, r.touching))
                .collect(),
        );
    }
    let mut out = Vec::new();
    if let Some(g) = d.poly() {
        let p = c.meet_poly(g);
        if vanishes(&p, size) {
            return None;
        }
        for r in roots(&p, on) {
            out.extend(c.param_in(d.at(r.x).0, range).map(|t| (t, r.touching)));
        }
    } else if let (
        Flat::Arc {
            center: ca,
            radius: ra,
            ..
        },
        Flat::Arc {
            center: cb,
            radius: rb,
            ..
        },
    ) = (*c, *d)
    {
        let e = cb - ca;
        let dist = e.length();
        if dist <= 1e-12 * (size + 1.0) {
            return ((ra - rb).abs() > 1e-12 * (size + 1.0)).then_some(out);
        }
        // along the centres by x, across by -+y
        let (e, n) = (e / dist, Vec2::new(-e.y, e.x) / dist);
        let x = (dist * dist + ra * ra - rb * rb) / (2.0 * dist);
        let y2 = ra * ra - x * x;
        let touching = y2.abs() <= 1e-12 * ra * ra;
        let ys = match (touching, y2 > 0.0) {
            (true, _) => vec![0.0],
            (false, true) => vec![-y2.sqrt(), y2.sqrt()],
            (false, false) => vec![],
        };
        for y in ys {
            let q = ca + e * x + n * y;
            if d.param_in(q, on).is_some() {
                out.extend(c.param_in(q, range).map(|t| (t, touching)));
            }
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    Some(out)
}

fn contact(touching: bool) -> Contact {
    if touching {
        Contact::Touching
    } else {
        Contact::Crossing
    }
}

/// Where a wrapped curve meets a wrapped wall about the same axis: where their flat curves
/// cross, if the curve's radius is among the wall's.
pub fn wrap_wall(w: &Wrap, range: Interval, s: &Surface) -> Option<CurveSurface> {
    if matches!(w.flat, Flat::Bezier { .. }) {
        return None;
    }
    let r = ruled(s)?;
    let (wa, radii) = radial(r)?;
    if wa.unroll != w.unroll || !wa.coaxial(&w.axis) {
        return None;
    }
    let Some(found) = crossings(&w.flat, range, &wa.flat, r.ext.u, w.radius.abs()) else {
        return Some(if radii.contains(w.radius) {
            CurveSurface::OnSurface
        } else {
            CurveSurface::Hits(vec![])
        });
    };
    if !radii.contains(w.radius) {
        return Some(CurveSurface::Hits(vec![]));
    }
    let hits = found
        .into_iter()
        .map(|(t, touching)| CurveHit {
            t,
            point: w.at(t).0,
            contact: contact(touching),
        })
        .collect();
    Some(CurveSurface::Hits(hits))
}

/// Where a line along the axis meets a wrapped wall: at its turn, where the flat curve has it.
pub fn line_wall(l: &Line, range: Interval, s: &Surface) -> Option<CurveSurface> {
    let r = ruled(s)?;
    let (wa, radii) = radial(r)?;
    let f = &wa.axis;
    let dz = f.z().dot(l.dir.get());
    if dz.abs() < 1.0 - tol::ANGULAR {
        return None;
    }
    let o = f.point_to_local(l.origin);
    if !radii.contains((o.x * o.x + o.y * o.y).sqrt()) {
        return Some(CurveSurface::Hits(vec![]));
    }
    let s0 = forge_math::dmath::atan2(o.y, o.x) * wa.unroll;
    let turn = forge_math::dmath::TAU * wa.unroll;
    let (sr, _) = wa.flat.bbox(r.ext.u);
    let mut hits = Vec::new();
    for k in ((sr.lo - s0) / turn).floor() as i64..=((sr.hi - s0) / turn).ceil() as i64 {
        for root in wa.flat.level(0, s0 + turn * k as f64, r.ext.u) {
            let h = wa.flat.at(root.x).0.y;
            let t = (h - o.z) / dz;
            if range.contains(t) {
                hits.push(CurveHit {
                    t,
                    point: l.eval(t),
                    contact: contact(root.touching),
                });
            }
        }
    }
    hits.sort_by(|a, b| a.t.total_cmp(&b.t));
    Some(CurveSurface::Hits(hits))
}

fn ruled(s: &Surface) -> Option<&Ruled> {
    match s {
        Surface::Procedural(p) => p.as_any().and_then(|x| x.downcast_ref::<Ruled>()),
        _ => None,
    }
}

impl Ruled {
    /// Two wrapped walls about one axis meet in the rulings where their flat curves cross.
    fn meet_wall(&self, other: &Ruled) -> Option<SurfaceSurface> {
        let ((wa, _), (wb, _)) = (radial(self)?, radial(other)?);
        if wa.unroll != wb.unroll || !wa.coaxial(&wb.axis) {
            return None;
        }
        let Some(found) = crossings(&wb.flat, other.ext.u, &wa.flat, self.ext.u, wa.radius.abs()) else {
            return Some(SurfaceSurface::Coincident);
        };
        let f = &wa.axis;
        let mut out = Vec::new();
        for (t, touching) in found {
            let (q, _, _) = wb.flat.at(t);
            let (sn, cs) = forge_math::dmath::sin_cos(q.x / wa.unroll);
            let out_dir = f.x() * cs + f.y() * sn;
            let line = crate::Curve::Line(Line {
                origin: f.origin + f.z() * q.y,
                dir: out_dir.normalize()?,
            });
            out.push((line, contact(touching)));
        }
        Some(if out.is_empty() {
            SurfaceSurface::Empty
        } else {
            SurfaceSurface::Curves(out)
        })
    }

    fn meet(&self, other: &Surface) -> Option<SurfaceSurface> {
        let one = |c: crate::Curve| Some(SurfaceSurface::Curves(vec![(c, Contact::Crossing)]));
        match (&self.a, &self.b, other) {
            (
                &Rail::Parabola { curve: ca, range: ra },
                &Rail::Parabola { curve: cb, range: rb },
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
            (&Rail::Wrap(wa), &Rail::Wrap(wb), Surface::Cylinder(c)) => {
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
    let (&Rail::Parabola { curve: ca, range: ra }, &Rail::Parabola { curve: cb, .. }) = (&r.a, &r.b) else {
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
    // coefficients: research/closed-forms/
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

    use crate::intersect::curve_surface_subdivide;
    use crate::wrap::{Flat, wrap_surface};

    /// A flat curve in a 8 x 6 patch of (s, h).
    fn flat(r: &mut Rng) -> Flat {
        if r.bool() {
            Flat::Line {
                a: Point2::new(r.range(-4.0, 0.0), r.range(-3.0, 3.0)),
                b: Point2::new(r.range(0.0, 4.0), r.range(-3.0, 3.0)),
            }
        } else {
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
    }

    fn wrap(axis: Frame, radius: f64, flat: Flat) -> Wrap {
        Wrap {
            axis,
            radius,
            unroll: 6.0,
            flat,
        }
    }

    /// Hits that the subdivision finds clearly (crossings away from the range's ends), against
    /// the closed form's.
    fn agree(what: &str, closed: CurveSurface, numeric: CurveSurface) -> bool {
        let (CurveSurface::Hits(a), CurveSurface::Hits(b)) = (&closed, &numeric) else {
            return matches!(closed, CurveSurface::OnSurface)
                == matches!(numeric, CurveSurface::OnSurface | CurveSurface::Overlap);
        };
        let clear = |h: &[CurveHit]| {
            h.iter()
                .all(|h| h.contact == Contact::Crossing && h.t > 1e-3 && h.t < 1.0 - 1e-3)
        };
        if !(clear(a) && clear(b)) {
            return false;
        }
        assert_eq!(a.len(), b.len(), "{what}: {a:?} vs {b:?}");
        for (x, y) in a.iter().zip(b) {
            assert!(x.point.distance(y.point) < 1e-7, "{what}: {x:?} vs {y:?}");
        }
        true
    }

    /// Wrapped curves against planes square to and along the axis, coaxial cylinders and
    /// wrapped walls, and lines along the axis against walls: as the subdivision finds them.
    #[test]
    fn wrapped_closed_forms_match_subdivision() {
        let mut r = Rng::new(733);
        let unit = Interval::new(0.0, 1.0);
        let mut checked = [0; 5];
        for _ in 0..300 {
            let axis = rand_frame(&mut r);
            let w = wrap(axis, r.range(5.0, 8.0), flat(&mut r));
            let c = w.curve();
            let level = axis.with_origin(axis.point_to_world(Point3::new(0.0, 0.0, r.range(-2.0, 2.0))));
            // square to the axis's x, along its z, through x = -3..3
            let along = Frame::from_zx(
                axis.point_to_world(Point3::new(r.range(-3.0, 3.0), 0.0, 0.0)),
                axis.x(),
                axis.z().get(),
            )
            .expect("a frame");
            for (k, plane) in [level, along].into_iter().enumerate() {
                let s = Surface::Plane(crate::Plane { frame: plane });
                let closed = wrap_surface(&w, unit, &s).expect("a closed form");
                checked[k] += agree("plane", closed, curve_surface_subdivide(&c, unit, &s, None)) as usize;
            }
            let cyl = |radius| Surface::Cylinder(crate::Cylinder { frame: axis, radius });
            assert!(
                matches!(wrap_surface(&w, unit, &cyl(w.radius + 0.5)), Some(CurveSurface::Hits(h)) if h.is_empty())
            );
            assert_eq!(
                wrap_surface(&w, unit, &cyl(w.radius)),
                Some(CurveSurface::OnSurface)
            );
            // a wall through the curve's radius
            let fw = flat(&mut r);
            let Some(wall) = Ruled::new(
                Rail::Wrap(wrap(axis, w.radius - 1.0, fw)),
                Rail::Wrap(wrap(axis, w.radius + 1.0, fw)),
            ) else {
                continue;
            };
            let ws = wall.clone().surface();
            // the subdivision also finds the carried-on wall's clamped edge
            let inside = |h: CurveSurface| match h {
                CurveSurface::Hits(h) => CurveSurface::Hits(
                    h.into_iter()
                        .filter(|h| {
                            let t = wall.unwrapped(h.point, wall.ext).expect("a wrapped wall").0.x;
                            t > wall.ext.u.lo + 1e-6 && t < wall.ext.u.hi - 1e-6
                        })
                        .collect(),
                ),
                o => o,
            };
            let closed = wrap_wall(&w, unit, &ws).expect("a closed form");
            checked[2] += agree(
                "wall",
                closed,
                inside(curve_surface_subdivide(&c, unit, &ws, None)),
            ) as usize;
            let l = Line {
                origin: axis.point_to_world(Point3::new(w.radius * 0.8, w.radius * 0.6, -4.0)),
                dir: axis.z(),
            };
            let range = Interval::new(0.5, 7.5);
            let closed = line_wall(&l, range, &ws).expect("a closed form");
            let line = crate::Curve::Line(l);
            let numeric = curve_surface_subdivide(&line, range, &ws, None);
            let scale = |h: CurveSurface| match h {
                CurveSurface::Hits(h) => CurveSurface::Hits(
                    h.into_iter()
                        .map(|h| CurveHit {
                            t: (h.t - range.lo) / range.width(),
                            ..h
                        })
                        .collect(),
                ),
                o => o,
            };
            checked[3] += agree("line", scale(closed), scale(inside(numeric))) as usize;
            // two walls meet in rulings on both
            let other = Ruled::new(
                Rail::Wrap(wrap(axis, w.radius - 0.5, w.flat)),
                Rail::Wrap(wrap(axis, w.radius + 0.5, w.flat)),
            );
            if let Some(o) = other
                && let Some(SurfaceSurface::Curves(cs)) = wall.meet_wall(&o)
            {
                for (c, _) in cs {
                    let p = c.eval(w.radius);
                    let os = o.clone().surface();
                    assert!(
                        ws.implicit(p).abs() < 1e-9 && os.implicit(p).abs() < 1e-9,
                        "{p:?}"
                    );
                    checked[4] += 1;
                }
            }
        }
        assert!(checked.iter().all(|&n| n > 20), "{checked:?}");
    }
}

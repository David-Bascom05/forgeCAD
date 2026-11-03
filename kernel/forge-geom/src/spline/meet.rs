//! Where a spline wall meets a plane, a surface about the same axis or a parallel wall, in closed
//! form: the section against a line or circle in its own plane is a Bernstein polynomial of the
//! curve's degree or twice it, whose roots are the rulings or circles; an extruded section cut
//! askew is the same curve under an affine map, so an exact spline with the same weights.
//! Derivation: research/spline-loops/README.md.

use super::{Extruded, Spline, SplineCurve, Spun};
use crate::curve::{Circle, Curve, Line};
use crate::intersect::{Contact, CurveHit, CurveSurface, SurfaceSurface};
use crate::surface::Surface;
use forge_math::{Frame, Interval, Point3, Vec3, tol};
use std::sync::Arc;

pub fn extruded(s: &Surface) -> Option<&Extruded> {
    match s {
        Surface::Procedural(p) if p.spline_wall() => p.as_any().and_then(|x| x.downcast_ref::<Extruded>()),
        _ => None,
    }
}

pub fn spun(s: &Surface) -> Option<&Spun> {
    match s {
        Surface::Procedural(p) if p.spline_wall() => p.as_any().and_then(|x| x.downcast_ref::<Spun>()),
        _ => None,
    }
}

fn contact(touching: bool) -> Contact {
    if touching {
        Contact::Touching
    } else {
        Contact::Crossing
    }
}

fn curves(out: Vec<(Curve, Contact)>) -> SurfaceSurface {
    if out.is_empty() {
        SurfaceSurface::Empty
    } else {
        SurfaceSurface::Curves(out)
    }
}

/// The closed form for a spline wall against the other surface, in either order; None for any
/// other pair.
pub fn meet_spline(a: &Surface, b: &Surface) -> Option<SurfaceSurface> {
    let wall = |s: &Surface| matches!(s, Surface::Procedural(p) if p.spline_wall());
    if !wall(a) && !wall(b) {
        return None;
    }
    if let Some(e) = extruded(a) {
        return e.meet(b);
    }
    if let Some(e) = extruded(b) {
        return e.meet(a);
    }
    if let Some(s) = spun(a) {
        return s.meet(b);
    }
    spun(b)?.meet(a)
}

/// A line or circle in a section's plane: how a surface about the same axis (or swept along the
/// same z) reads there.
enum Trace {
    /// n.q + d = 0, n unit.
    Line(Vec3, f64),
    /// |q - o|^2 = r^2.
    Circle(Point3, f64),
}

impl Trace {
    /// Where the section meets it over `range`.
    fn roots(&self, c: &Spline, range: Interval) -> Vec<(f64, bool)> {
        match *self {
            Trace::Line(n, d) => c.plane_roots(n, d, range, tol::LINEAR),
            Trace::Circle(o, r) => {
                let a = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
                c.quadric_roots(o, &a, Vec3::ZERO, -r * r, range, 2.0 * r * tol::LINEAR)
            }
        }
    }
}

impl Extruded {
    fn meet(&self, other: &Surface) -> Option<SurfaceSurface> {
        let f = *self.frame();
        let z = f.z();
        let ruling = |t: f64| {
            let q = self.section().point(t);
            Line {
                origin: f.point_to_world(Point3::new(q.x, q.y, 0.0)),
                dir: z,
            }
        };
        let rulings = |tr: Trace| {
            curves(
                tr.roots(self.section(), self.reach())
                    .into_iter()
                    .map(|(t, touch)| (Curve::Line(ruling(t)), contact(touch)))
                    .collect(),
            )
        };
        match other {
            Surface::Plane(p) => {
                let n = f.vec_to_local(p.frame.z().get());
                let d = -f.point_to_local(p.frame.origin).to_vec().dot(n);
                if n.z.abs() <= tol::ANGULAR {
                    let n2 = Vec3::new(n.x, n.y, 0.0).normalize()?.get();
                    let s = self.section();
                    let dom = self.reach();
                    let flat = s.bbox(dom);
                    let on = [flat.x.lo, flat.x.hi].into_iter().all(|x| {
                        [flat.y.lo, flat.y.hi]
                            .into_iter()
                            .all(|y| (n2.x * x + n2.y * y + d).abs() <= tol::LINEAR)
                    });
                    if on {
                        return Some(SurfaceSurface::Coincident);
                    }
                    return Some(rulings(Trace::Line(n2, d)));
                }
                // a height for every point of the section: z = -(n.x x + n.y y + d) / n.z
                let (kx, ky, k0) = (-n.x / n.z, -n.y / n.z, -d / n.z);
                let cut = self.section().mapped(|q| {
                    let v = kx * q.x + ky * q.y + k0;
                    f.point_to_world(Point3::new(q.x, q.y, v))
                });
                Some(curves(vec![(
                    SplineCurve::curve(Arc::new(cut), self.reach()),
                    Contact::Crossing,
                )]))
            }
            Surface::Cylinder(c) if c.frame.z().cross(z.get()).length() <= tol::ANGULAR => {
                let o = f.point_to_local(c.frame.origin);
                Some(rulings(Trace::Circle(Point3::new(o.x, o.y, 0.0), c.radius)))
            }
            Surface::Procedural(_) => {
                let e = extruded(other)?;
                if e.frame().z().cross(z.get()).length() > tol::ANGULAR {
                    return None;
                }
                let same = same_section(self, e);
                if same {
                    return Some(SurfaceSurface::Coincident);
                }
                // both swept one way: where the sections cross, in this frame
                let g = e.frame();
                let to_here = e.section().mapped(|q| {
                    let w = g.point_to_world(q);
                    let l = f.point_to_local(w);
                    Point3::new(l.x, l.y, 0.0)
                });
                let hits = cross_sections(self.section(), self.reach(), &to_here, e.reach());
                Some(curves(
                    hits.into_iter()
                        .map(|(t, touch)| (Curve::Line(ruling(t)), contact(touch)))
                        .collect(),
                ))
            }
            _ => None,
        }
    }
}

fn same_section(a: &Extruded, b: &Extruded) -> bool {
    let (fa, fb) = (a.frame(), b.frame());
    let q = b.section().mapped(|p| {
        let l = fa.point_to_local(fb.point_to_world(p));
        Point3::new(l.x, l.y, 0.0)
    });
    same_curve(a.section(), &q)
}

/// Whether one curve lies along the other, by samples of each on the other.
pub fn same_curve(s: &Spline, q: &Spline) -> bool {
    let (d, e) = (s.domain(), q.domain());
    (0..=16).all(|i| {
        let p = s.point(d.lo + d.width() * i as f64 / 16.0);
        q.point(q.closest(p, e)).distance(p) <= tol::LINEAR
    }) || (0..=16).all(|i| {
        let p = q.point(e.lo + e.width() * i as f64 / 16.0);
        s.point(s.closest(p, d)).distance(p) <= tol::LINEAR
    })
}

/// Parameters on `a` where two planar splines cross or touch: their hulls split down until a
/// Newton step on the pair settles (Sederberg's clipping without the clip: boxes are enough for
/// the piece counts of sketches). Each with whether the crossing is tangent.
pub fn cross_sections(a: &Spline, ra: Interval, b: &Spline, rb: Interval) -> Vec<(f64, bool)> {
    let mut stack = vec![(ra, rb)];
    let mut out: Vec<(f64, bool)> = Vec::new();
    let mut steps = 0;
    while let Some((x, y)) = stack.pop() {
        steps += 1;
        if steps > 200_000 {
            break;
        }
        let (bx, by) = (a.bbox(x), b.bbox(y));
        if !bx.grow(tol::LINEAR).overlaps(&by.grow(tol::LINEAR)) {
            continue;
        }
        let size = |b: &forge_math::Box3| b.x.width().max(b.y.width());
        if size(&bx) <= 1e-4 && size(&by) <= 1e-4 {
            let (mut s, mut t) = (x.mid(), y.mid());
            let mut good = false;
            let mut touch = false;
            for _ in 0..12 {
                let (p, q) = (a.deriv(s), b.deriv(t));
                let e = q.p - p.p;
                let det = p.d1.x * (-q.d1.y) + q.d1.x * p.d1.y;
                let cross = p.d1.x * q.d1.y - p.d1.y * q.d1.x;
                touch = cross.abs() <= 1e-6 * p.d1.length() * q.d1.length();
                if touch {
                    // the common normal's foot: closest approach
                    let n = Vec3::new(-p.d1.y, p.d1.x, 0.0);
                    good = e.dot(n).abs() <= tol::LINEAR * n.length();
                    break;
                }
                let ds = (e.x * (-q.d1.y) + q.d1.x * e.y) / det;
                let dt = (p.d1.x * e.y - p.d1.y * e.x) / det;
                // settled once the points are apart by their own rounding, which grows with the
                // coordinates and the parameters; a fixed stop is circled forever far out
                // (research/spline-cross-stop)
                let (ls, lt) = (p.d1.length(), q.d1.length());
                let scale = 1.0 + p.p.to_vec().abs_max() + s.abs() * ls + t.abs() * lt;
                s += ds;
                t += dt;
                if e.length() <= 64.0 * f64::EPSILON * scale {
                    good = true;
                    break;
                }
            }
            if good && ra.contains(s) && rb.contains(t) && out.iter().all(|h| (h.0 - s).abs() > 1e-7) {
                out.push((s, touch));
            }
            continue;
        }
        if size(&bx) >= size(&by) {
            let m = x.mid();
            stack.push((Interval::new(x.lo, m), y));
            stack.push((Interval::new(m, x.hi), y));
        } else {
            let m = y.mid();
            stack.push((x, Interval::new(y.lo, m)));
            stack.push((x, Interval::new(m, y.hi)));
        }
    }
    out.sort_by(|p, q| p.0.total_cmp(&q.0));
    out
}

impl Spun {
    fn meet(&self, other: &Surface) -> Option<SurfaceSurface> {
        let f = *self.frame();
        let (z, o) = (f.z(), f.origin);
        let coaxial = |g: &Frame| {
            let off = g.origin - o;
            g.z().cross(z.get()).length() <= tol::ANGULAR && (off - z * z.dot(off)).length() <= tol::LINEAR
        };
        let height = |g: &Frame| z.dot(g.origin - o);
        let trace = match other {
            Surface::Plane(p) => {
                if p.frame.z().cross(z.get()).length() <= tol::ANGULAR {
                    Trace::Line(Vec3::Y, -height(&p.frame))
                } else {
                    let n = p.frame.z().get();
                    if n.dot(z.get()).abs() <= tol::ANGULAR && n.dot(p.frame.origin - o).abs() <= tol::LINEAR
                    {
                        return Some(self.through_axis(n));
                    }
                    return None;
                }
            }
            Surface::Cylinder(c) if coaxial(&c.frame) => Trace::Line(Vec3::X, -c.radius),
            Surface::Cone(c) if coaxial(&c.frame) => {
                // rho = r0 + k s (h - h0), s the cone's axis against ours
                let s = z.dot(c.frame.z().get()).signum();
                let k = c.tan_a();
                let n = Vec3::new(1.0, -k * s, 0.0);
                let l = n.length();
                Trace::Line(n / l, (-c.radius + k * s * height(&c.frame)) / l)
            }
            Surface::Sphere(c) if coaxial(&c.frame) => {
                Trace::Circle(Point3::new(0.0, height(&c.frame), 0.0), c.radius)
            }
            Surface::Torus(c) if coaxial(&c.frame) => {
                Trace::Circle(Point3::new(c.major, height(&c.frame), 0.0), c.minor)
            }
            _ => return None,
        };
        let s = self.section();
        Some(curves(
            trace
                .roots(s, self.reach())
                .into_iter()
                .filter_map(|(t, touch)| {
                    let q = s.point(t);
                    (q.x > tol::LINEAR).then(|| {
                        let frame = f.with_origin(o + z * q.y);
                        (Curve::Circle(Circle { frame, radius: q.x }), contact(touch))
                    })
                })
                .collect(),
        ))
    }

    /// The plane through the axis cuts the surface in the section, at its two angles.
    fn through_axis(&self, n: Vec3) -> SurfaceSurface {
        let f = *self.frame();
        let (ex, ez) = (f.z().cross(n), f.z().get());
        let out = [1.0, -1.0].map(|k| {
            let radial = ex * k;
            let c = self.section().mapped(|q| f.origin + radial * q.x + ez * q.y);
            (SplineCurve::curve(Arc::new(c), self.reach()), Contact::Crossing)
        });
        curves(out.to_vec())
    }
}

/// Where a line meets an extruded wall: the section against the line's trace, its parameter from
/// the crossing. A line along the rulings is on the wall or misses it.
pub fn line_extruded(l: &Line, range: Interval, s: &Surface) -> Option<CurveSurface> {
    let e = extruded(s)?;
    let f = e.frame();
    let d = f.vec_to_local(l.dir.get());
    let o = f.point_to_local(l.origin);
    let flat = (d.x * d.x + d.y * d.y).sqrt();
    if flat <= tol::ANGULAR {
        let q = Point3::new(o.x, o.y, 0.0);
        let c = e.section();
        let foot = c.point(c.closest(q, e.reach()));
        return Some(if foot.distance(q) <= tol::LINEAR {
            CurveSurface::OnSurface
        } else {
            CurveSurface::Hits(Vec::new())
        });
    }
    let (tx, ty) = (d.x / flat, d.y / flat);
    let n = Vec3::new(-ty, tx, 0.0);
    let k = -(n.x * o.x + n.y * o.y);
    let mut hits = Vec::new();
    let c = e.section();
    for (u, touch) in c.plane_roots(n, k, e.reach(), tol::LINEAR) {
        let q = c.point(u);
        let t = (tx * (q.x - o.x) + ty * (q.y - o.y)) / flat;
        if range.contains(t) {
            hits.push(CurveHit {
                t,
                point: l.origin + l.dir * t,
                contact: contact(touch),
            });
        }
    }
    hits.sort_by(|a, b| a.t.total_cmp(&b.t));
    Some(CurveSurface::Hits(hits))
}

/// Where a spline edge meets a plane or quadric, in closed form.
pub fn spline_surface(c: &SplineCurve, range: Interval, s: &Surface) -> Option<CurveSurface> {
    let roots: Vec<(f64, bool)> = match s {
        Surface::Plane(p) => {
            let n = p.frame.z().get();
            let d = -n.dot(p.frame.origin.to_vec());
            let (b, r) = (c.c.bbox(range), tol::LINEAR);
            if [b.x.lo, b.x.hi].into_iter().all(|x| {
                [b.y.lo, b.y.hi].into_iter().all(|y| {
                    [b.z.lo, b.z.hi]
                        .into_iter()
                        .all(|z| (n.x * x + n.y * y + n.z * z + d).abs() <= r)
                })
            }) {
                return Some(CurveSurface::OnSurface);
            }
            c.c.plane_roots(n, d, range, r)
        }
        Surface::Cylinder(_) | Surface::Cone(_) | Surface::Sphere(_) => {
            let crate::meet::Alg::Quadric { o, a, b, c: k } = crate::meet::Alg::of(s)? else {
                return None;
            };
            let rate = match s {
                Surface::Cylinder(q) => 2.0 * q.radius,
                Surface::Sphere(q) => 2.0 * q.radius,
                _ => return None,
            };
            c.c.quadric_roots(o, &a, b, k, range, rate * tol::LINEAR)
        }
        _ => return None,
    };
    Some(CurveSurface::Hits(
        roots
            .into_iter()
            .map(|(t, touch)| CurveHit {
                t,
                point: c.c.point(t),
                contact: contact(touch),
            })
            .collect(),
    ))
}

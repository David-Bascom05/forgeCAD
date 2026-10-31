//! What integrators need beyond evaluation: exact first derivatives, and a smooth parameter
//! where a curve's own is only once differentiable (a traced curve's chain joins).

use crate::curve::{Curve, CurveDeriv, CurveGeom, Domain1, ProcCurve, ProceduralCurve};
use crate::intcurve::IntCurve;
use crate::surface::Surface;
use forge_math::{Box3, Frame, Interval, Point3, Vec3, tol};
use std::sync::Arc;

impl Curve {
    /// Whether the first derivative is exact (an intersection curve's is its guiding cubic's).
    pub fn exact_d1(&self) -> bool {
        match self {
            Curve::Intersection(_) => false,
            Curve::Procedural(c) => c.exact_d1(),
            _ => true,
        }
    }

    /// `ProceduralCurve::smooth`.
    pub fn smooth(&self, range: Interval) -> Option<(&Curve, Interval)> {
        match self {
            Curve::Procedural(c) => c.smooth(range),
            _ => None,
        }
    }

    /// `eval` of the kinds without their own arm there (out of line, so the common kinds'
    /// dispatch stays a short chain of tests): a procedural curve's point alone.
    #[inline(never)]
    pub(crate) fn eval_rest(&self, t: f64) -> Point3 {
        match self {
            Curve::Ellipse(c) => c.eval(t),
            Curve::Parabola(c) => c.eval(t),
            Curve::Hyperbola(c) => c.eval(t),
            Curve::Procedural(c) => c.point(t),
            _ => self.deriv(t).p,
        }
    }

    /// The point and first derivative (a procedural curve may skip the second).
    pub fn deriv1(&self, t: f64) -> (Point3, Vec3) {
        match self {
            Curve::Procedural(c) => c.deriv1(t),
            _ => {
                let d = self.deriv(t);
                (d.p, d.d1)
            }
        }
    }
}

impl Surface {
    /// `Procedural::smooth`, or the surface itself.
    pub fn smooth(&self) -> &Surface {
        match self {
            Surface::Procedural(p) => p.smooth().unwrap_or(self),
            _ => self,
        }
    }

    /// The derivative of `gradient` at p along v (the unit normal field's turn, the implicit's
    /// Hessian times v); a difference of gradients on a procedural surface.
    pub fn gradient_deriv(&self, p: Point3, v: Vec3) -> Vec3 {
        // the turn of the unit vector from the axis, square to it, at distance rho
        let radial = |f: &Frame, l: Point3| {
            let rho = (l.x * l.x + l.y * l.y).sqrt();
            let r = if rho > 0.0 {
                f.x() * (l.x / rho) + f.y() * (l.y / rho)
            } else {
                f.x().get()
            };
            let w = v - f.z() * f.z().dot(v);
            (
                r,
                if rho > 0.0 {
                    (w - r * r.dot(w)) / rho
                } else {
                    Vec3::ZERO
                },
            )
        };
        let unit = |n: Vec3, w: Vec3, len: f64| {
            if len > 0.0 {
                (w - n * n.dot(w)) / len
            } else {
                Vec3::ZERO
            }
        };
        match self {
            Surface::Plane(_) => Vec3::ZERO,
            Surface::Cylinder(c) => radial(&c.frame, c.frame.point_to_local(p)).1,
            Surface::Cone(c) => radial(&c.frame, c.frame.point_to_local(p)).1 * c.cos_a(),
            Surface::Sphere(sp) => {
                let d = p - sp.frame.origin;
                unit(self.gradient(p), v, d.length())
            }
            Surface::Torus(t) => {
                let (r, dr) = radial(&t.frame, t.frame.point_to_local(p));
                let d = p - (t.frame.origin + r * t.major);
                unit(self.gradient(p), v - dr * t.major, d.length())
            }
            Surface::Procedural(s) => {
                let (len, h) = (v.length(), 1e-6 * (1.0 + (p - Point3::ORIGIN).abs_max()));
                if len == 0.0 {
                    return Vec3::ZERO;
                }
                let w = v * (h / len);
                (s.gradient(p + w) - s.gradient(p - w)) * (len / (2.0 * h))
            }
        }
    }
}

/// An intersection curve given a smooth parameter: its chain parameter only joins once
/// differentiably at the chain points. In stretches over which the curve turns less than 30
/// degrees, u is the distance along the stretch's chord, the point solved exactly onto both
/// surfaces and the plane square to the chord at that distance: as smooth as the surfaces
/// within a stretch, and only the speed jumps where stretches meet.
#[derive(Debug)]
pub struct ChordCurve {
    pub ic: IntCurve,
    stretches: Vec<Stretch>,
    /// u at the end (a closed curve's period).
    len: f64,
}

#[derive(Debug, Clone)]
struct Stretch {
    u0: f64,
    anchor: Point3,
    dir: Vec3,
    /// (t, u) at the chain points over the stretch, increasing.
    knots: Vec<(f64, f64)>,
    /// A bound on |dP/du| (1 / the least cosine between the curve and the chord).
    speed: f64,
}

impl ChordCurve {
    /// None where the chain doubles back within a stretch.
    pub fn new(ic: IntCurve) -> Option<ChordCurve> {
        let n = ic.pts.len();
        let hi = ic.domain_hi();
        let mut stretches = Vec::new();
        let (mut t0, mut u0) = (0.0, 0.0);
        while t0 < hi {
            let tan0 = ic.tans[t0 as usize % n];
            let mut t1 = t0 + 1.0;
            while t1 < hi && ic.tans[t1 as usize % n].dot(tan0) >= 0.866 {
                t1 += 1.0;
            }
            let (anchor, end) = (ic.pts[t0 as usize % n], ic.pts[t1 as usize % n]);
            let dir = (end - anchor).normalize()?.get();
            let knots: Vec<(f64, f64)> = (t0 as usize..=t1 as usize)
                .map(|k| (k as f64, u0 + dir.dot(ic.pts[k % n] - anchor)))
                .collect();
            if knots.windows(2).any(|w| w[1].1 <= w[0].1) {
                return None;
            }
            let cos = (t0 as usize..=t1 as usize)
                .map(|k| ic.tans[k % n].dot(dir))
                .fold(1.0, f64::min);
            if cos <= 0.1 {
                return None;
            }
            u0 = knots[knots.len() - 1].1;
            stretches.push(Stretch {
                u0: knots[0].1,
                anchor,
                dir,
                knots,
                speed: 1.25 / cos,
            });
            t0 = t1;
        }
        Some(ChordCurve {
            ic,
            stretches,
            len: u0,
        })
    }

    pub fn curve(self) -> Curve {
        Curve::Procedural(ProcCurve(Arc::new(self)))
    }

    /// u moved by periods into [0, len) on a closed curve.
    fn wrap(&self, u: f64) -> f64 {
        if self.ic.closed && !(0.0..self.len).contains(&u) {
            u.rem_euclid(self.len)
        } else {
            u
        }
    }

    fn stretch(&self, u: f64) -> &Stretch {
        let i = self
            .stretches
            .partition_point(|s| s.u0 <= u)
            .clamp(1, self.stretches.len());
        &self.stretches[i - 1]
    }

    /// The chain parameter about where u is (between the knots, linearly), whole turns of a
    /// closed curve carried over.
    fn t_of(&self, u: f64) -> f64 {
        let w = self.wrap(u);
        let k = &self.stretch(w).knots;
        let i = k.partition_point(|x| x.1 <= w).clamp(1, k.len() - 1);
        let ((ta, ua), (tb, ub)) = (k[i - 1], k[i]);
        let t = ta + (tb - ta) * (w - ua) / (ub - ua);
        if self.ic.closed {
            t + self.ic.domain_hi() * ((u - w) / self.len).round()
        } else {
            t
        }
    }

    /// u of the curve's point at chain parameter t.
    pub fn u_of(&self, t: f64) -> f64 {
        let hi = self.ic.domain_hi();
        let w = if self.ic.closed {
            t.rem_euclid(hi)
        } else {
            t.clamp(0.0, hi)
        };
        let i = self
            .stretches
            .partition_point(|s| s.knots[0].0 <= w)
            .clamp(1, self.stretches.len());
        let st = &self.stretches[i - 1];
        let u = st.u0 + st.dir.dot(self.ic.eval(w) - st.anchor);
        if self.ic.closed {
            u + self.len * ((t - w) / hi).round()
        } else {
            u
        }
    }

    /// The chain's cubic at t (Hermite, from the chain's directions), as `IntCurve::eval` starts.
    fn cubic(&self, t: f64) -> Point3 {
        let (ic, n, hi) = (&self.ic, self.ic.pts.len(), self.ic.domain_hi());
        let t = if (0.0..hi).contains(&t) {
            t
        } else if ic.closed {
            t.rem_euclid(hi)
        } else {
            t.clamp(0.0, hi)
        };
        let i = (t.floor() as usize).min(ic.segments() - 1);
        let (p0, p1) = (ic.pts[i], ic.pts[(i + 1) % n]);
        let chord = p1 - p0;
        let len = chord.length();
        let (m0, m1) = (ic.tans[i] * len, ic.tans[(i + 1) % n] * len);
        let f = t - i as f64;
        let (f2, f3) = (f * f, f * f * f);
        p0 + chord * (3.0 * f2 - 2.0 * f3) + m0 * (f3 - 2.0 * f2 + f) + m1 * (f3 - f2)
    }

    /// The point at u: from the chain's, Newton onto both surfaces and the stretch's plane.
    fn point(&self, u: f64) -> (Point3, &Stretch) {
        let w = self.wrap(u);
        let st = self.stretch(w);
        let s = w - st.u0;
        // the chain's cubic is within about 1e-6 of the curve: Newton doubles the digits per
        // step, so once a step is below 1e-8 of the size the next would be below rounding
        let mut p = self.cubic(self.t_of(w));
        let scale = 1.0 + (p - Point3::ORIGIN).abs_max();
        for _ in 0..8 {
            let ((fa, ga), (fb, gb)) = (self.ic.a.implicit_gradient(p), self.ic.b.implicit_gradient(p));
            let fs = st.dir.dot(p - st.anchor) - s;
            // Cramer on rows ga, gb, dir: the step solves ga.d = -fa, gb.d = -fb, dir.d = -fs
            let (bc, ca, ab) = (gb.cross(st.dir), st.dir.cross(ga), ga.cross(gb));
            let det = ga.dot(bc);
            if det.abs() <= 1e-14 {
                break;
            }
            let step = (bc * -fa + ca * -fb + ab * -fs) / det;
            p += step;
            if step.abs_max() <= 1e-8 * scale {
                break;
            }
        }
        (p, st)
    }
}

impl ProceduralCurve for ChordCurve {
    fn point(&self, u: f64) -> Point3 {
        ChordCurve::point(self, u).0
    }
    fn deriv1(&self, u: f64) -> (Point3, Vec3) {
        let (p, st) = ChordCurve::point(self, u);
        let t = self.ic.a.gradient(p).cross(self.ic.b.gradient(p));
        (p, t / t.dot(st.dir))
    }
    fn deriv(&self, u: f64) -> CurveDeriv {
        let (p, st) = self.point(u);
        let (a, b) = (&self.ic.a, &self.ic.b);
        let (ga, gb) = (a.gradient(p), b.gradient(p));
        // along the curve at unit speed along dir: d1 = T / (T . dir), T = ga x gb
        let t = ga.cross(gb);
        let k = t.dot(st.dir);
        let d1 = t / k;
        let dt = a.gradient_deriv(p, d1).cross(gb) + ga.cross(b.gradient_deriv(p, d1));
        CurveDeriv {
            p,
            d1,
            d2: (dt - d1 * dt.dot(st.dir)) / k,
        }
    }
    fn exact_d1(&self) -> bool {
        true
    }
    fn domain(&self) -> Domain1 {
        Domain1 {
            lo: 0.0,
            hi: self.len,
            period: self.ic.closed.then_some(self.len),
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let (tl, th) = (
            self.t_of(range.lo).floor() - 1.0,
            self.t_of(range.hi).ceil() + 1.0,
        );
        let tr = if self.ic.closed {
            Interval::new(tl, th.max(tl))
        } else {
            Interval::new(tl.max(0.0), th.min(self.ic.domain_hi()))
        };
        let t = self.ic.closest(p, tr);
        // the chain parameter's answer moved to u
        // (round a whole closed curve, free to run past the ends, then put back in the range)
        let whole = self.ic.closed && range.width() >= self.len;
        let fit = |u: f64| if whole { u } else { u.clamp(range.lo, range.hi) };
        let mut u = self.u_of(t);
        if self.ic.closed {
            u += self.len * ((range.mid() - u) / self.len).round();
        }
        let mut u = fit(u);
        // then Newton on (P - p) . P' = 0
        for _ in 0..8 {
            let d = self.deriv(u);
            let g = (d.p - p).dot(d.d1);
            let h = d.d1.dot(d.d1) + (d.p - p).dot(d.d2);
            if h <= 0.0 {
                break;
            }
            let next = fit(u - g / h);
            let done = (next - u).abs() <= 1e-14 * (1.0 + u.abs());
            u = next;
            if done {
                break;
            }
        }
        if whole {
            range.lo + (u - range.lo).rem_euclid(self.len)
        } else {
            u
        }
    }
    fn bbox(&self, range: Interval) -> Box3 {
        let (tl, th) = (self.t_of(range.lo), self.t_of(range.hi));
        let b = self.ic.bbox(Interval::new(tl, th.max(tl)));
        b.hull(&Box3::from_points([
            self.point(range.lo).0,
            self.point(range.hi).0,
        ]))
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(ChordCurve {
            ic: self.ic.transformed(f),
            stretches: self
                .stretches
                .iter()
                .map(|s| Stretch {
                    anchor: f.point_to_world(s.anchor),
                    dir: f.vec_to_world(s.dir),
                    ..s.clone()
                })
                .collect(),
            len: self.len,
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        if self.ic.closed && range.width() >= self.len {
            return self.stretches.iter().map(|s| s.speed).fold(0.0, f64::max);
        }
        let (lo, hi) = (self.wrap(range.lo), self.wrap(range.lo) + range.width());
        self.stretches
            .iter()
            .enumerate()
            .filter(|(i, s)| {
                let end = self.stretches.get(i + 1).map_or(self.len, |n| n.u0);
                (s.u0 <= hi && end >= lo) || (self.ic.closed && s.u0 + self.len <= hi)
            })
            .map(|(_, s)| s.speed)
            .fold(0.0, f64::max)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        // a chord of parameter length h sags at most h^2 max|P''| / 8
        let m = 16;
        let acc = (0..=m)
            .map(|i| {
                self.deriv(range.lo + range.width() * i as f64 / m as f64)
                    .d2
                    .length()
            })
            .fold(0.0, f64::max)
            * 1.5
            + f64::MIN_POSITIVE;
        ((range.width() / (8.0 * chord.max(tol::LINEAR) / acc).sqrt()).ceil() as usize).max(1)
    }
    fn kinks(&self, range: Interval) -> Vec<f64> {
        let mut out = Vec::new();
        let base = if self.ic.closed {
            self.len * (range.lo / self.len).floor()
        } else {
            0.0
        };
        let mut k = 0;
        loop {
            let s = &self.stretches[k % self.stretches.len()];
            let u = base + s.u0 + self.len * (k / self.stretches.len()) as f64;
            if u >= range.hi || (!self.ic.closed && k >= self.stretches.len()) {
                break;
            }
            if u > range.lo {
                out.push(u);
            }
            k += 1;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intcurve::trace;
    use crate::surface::{Cylinder, Sphere, SurfaceGeom};

    #[test]
    fn chord_curve_is_on_both_with_its_derivatives() {
        let cyl = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 2.0,
        });
        let sph = Surface::Sphere(Sphere {
            frame: Frame::WORLD.with_origin(Point3::new(0.5, 0.0, 1.0)),
            radius: 4.0,
        });
        let region = Box3::from_points([Point3::new(-6.0, -6.0, -6.0), Point3::new(6.0, 6.0, 8.0)]);
        for ic in trace(&cyl, &sph, region).unwrap() {
            let c = ChordCurve::new(ic).unwrap();
            let len = c.len;
            assert!(
                c.stretches.len() >= 8 && c.stretches.len() < c.ic.pts.len(),
                "{}",
                c.stretches.len()
            );
            for i in 0..=97 {
                let u = len * i as f64 / 97.0 + 0.123;
                let d = ProceduralCurve::deriv(&c, u);
                assert!(cyl.distance(d.p) <= 1e-12 && sph.distance(d.p) <= 1e-12, "{u}");
                let h = 1e-5;
                let (a, b) = (
                    ProceduralCurve::deriv(&c, u + h),
                    ProceduralCurve::deriv(&c, u - h),
                );
                if c.kinks(Interval::new(u - 2.0 * h, u + 2.0 * h)).is_empty() {
                    assert!(((a.p - b.p) / (2.0 * h) - d.d1).length() < 1e-8, "d1 at {u}");
                    assert!(((a.d1 - b.d1) / (2.0 * h) - d.d2).length() < 1e-5, "d2 at {u}");
                }
                let q = d.p + Vec3::new(0.01, -0.02, 0.015);
                let t = c.closest(q, Interval::new(0.0, len));
                let near = ProceduralCurve::deriv(&c, t);
                assert!(
                    (q - near.p).dot(near.d1).abs() < 1e-9,
                    "closest at {u}: t {t} res {} dist {} kinks {:?}",
                    (q - near.p).dot(near.d1),
                    near.p.distance(d.p),
                    c.kinks(Interval::new(u - 0.5, u + 0.5))
                );
                assert!(near.p.distance(d.p) < 0.1, "closest at {u}");
            }
        }
    }
}

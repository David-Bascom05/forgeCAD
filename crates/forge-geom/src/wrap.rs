//! Flat curves wrapped round a cylinder, exact by construction: a line or parabola drawn in
//! (s, h), s the length round and h the height along the axis, laid on a cylinder of radius
//! `unroll` (so s turns the angle s / unroll) and carried out to radius `radius` along the
//! same rays. Text wrapped round a cylinder has these for its edges and rails, and ruled walls
//! between two of them.

use crate::curve::{CurveDeriv, CurveGeom, Domain1, Parabola, ProcCurve, ProceduralCurve};
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec2, Vec3, dmath, tol};
use std::sync::Arc;

/// A curve in (s, h), by the fraction t along it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Flat {
    Line {
        a: Point2,
        b: Point2,
    },
    /// The parabola (in its frame's xy, read as (s, h)) over the parameter range.
    Parabola {
        curve: Parabola,
        range: Interval,
    },
}

impl Flat {
    /// Point and first two derivatives at fraction t.
    pub fn at(&self, t: f64) -> (Point2, Vec2, Vec2) {
        match *self {
            Flat::Line { a, b } => (a.lerp(b, t), b - a, Vec2::ZERO),
            Flat::Parabola { curve, range } => {
                let w = range.width();
                let d = curve.deriv(range.lo + w * t);
                (
                    Point2::new(d.p.x, d.p.y),
                    Vec2::new(d.d1.x, d.d1.y) * w,
                    Vec2::new(d.d2.x, d.d2.y) * (w * w),
                )
            }
        }
    }

    /// Boxes of s and h over fractions t.
    pub fn bbox(&self, t: Interval) -> (Interval, Interval) {
        match *self {
            Flat::Line { a, b } => (t * (b.x - a.x) + a.x, t * (b.y - a.y) + a.y),
            Flat::Parabola { curve, range } => {
                let w = range.width();
                let b = curve.bbox(Interval::new(range.lo + w * t.lo, range.lo + w * t.hi));
                (b.x, b.y)
            }
        }
    }

    /// Bounds on |d/dt| and |d2/dt2| over fractions t (the speed is linear in t).
    fn bounds(&self, t: Interval) -> (f64, f64) {
        let (_, v0, a) = self.at(t.lo);
        let (_, v1, _) = self.at(t.hi);
        (v0.length().max(v1.length()), a.length())
    }
}

/// A flat curve wrapped round the axis frame's z, its s = 0 on the frame's x.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wrap {
    pub axis: Frame,
    pub radius: f64,
    pub unroll: f64,
    pub flat: Flat,
}

impl Wrap {
    /// Point and first two derivatives at fraction t.
    pub fn at(&self, t: f64) -> (Point3, Vec3, Vec3) {
        let (q, d1, d2) = self.flat.at(t);
        let (sn, cs) = dmath::sin_cos(q.x / self.unroll);
        let (x, y, z) = (self.axis.x().get(), self.axis.y().get(), self.axis.z().get());
        let out = x * cs + y * sn;
        let round = y * cs - x * sn;
        let (r, w) = (self.radius, d1.x / self.unroll);
        (
            self.axis.origin + out * r + z * q.y,
            round * (r * w) + z * d1.y,
            out * (-r * w * w) + round * (r * d2.x / self.unroll) + z * d2.y,
        )
    }

    pub fn bbox(&self, t: Interval) -> Box3 {
        let (s, h) = self.flat.bbox(t);
        let a = s * (1.0 / self.unroll);
        let (cx, cy) = (a.cos() * self.radius, a.sin() * self.radius);
        let f = &self.axis;
        let (x, y, z, o) = (f.x().get(), f.y().get(), f.z().get(), f.origin);
        Box3 {
            x: cx * x.x + cy * y.x + h * z.x + o.x,
            y: cx * x.y + cy * y.y + h * z.y + o.y,
            z: cx * x.z + cy * y.z + h * z.z + o.z,
        }
        .grow(tol::LINEAR)
    }

    pub fn transformed(&self, f: &Frame) -> Wrap {
        Wrap {
            axis: f.compose(&self.axis),
            ..*self
        }
    }

    /// The same flat curve carried to another radius.
    pub fn at_radius(&self, radius: f64) -> Wrap {
        Wrap { radius, ..*self }
    }

    pub fn curve(self) -> crate::Curve {
        crate::Curve::Procedural(ProcCurve(Arc::new(self)))
    }

    fn bounds(&self, t: Interval) -> (f64, f64) {
        let (v, a) = self.flat.bounds(t);
        let k = self.radius / self.unroll;
        (v * (k + 1.0), k * v * v / self.unroll + a * (k + 1.0))
    }
}

impl ProceduralCurve for Wrap {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let (p, d1, d2) = self.at(t);
        CurveDeriv { p, d1, d2 }
    }
    fn domain(&self) -> Domain1 {
        // carried on a quarter past each end, so curves cut from walls cross their ends
        Domain1 {
            lo: -0.25,
            hi: 1.25,
            period: None,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        const N: usize = 32;
        let mut best = (f64::INFINITY, range.lo);
        for i in 0..=N {
            let t = range.lo + range.width() * i as f64 / N as f64;
            let d = self.at(t).0.distance_sq(p);
            if d < best.0 {
                best = (d, t);
            }
        }
        // Newton on (C - p).C' = 0, kept inside the range while it gets closer
        let (mut t, mut f) = (best.1, best.0);
        for _ in 0..16 {
            let (q, d1, d2) = self.at(t);
            let e = q - p;
            let (g, h) = (e.dot(d1), d1.length_sq() + e.dot(d2));
            let h = if h > 0.0 { h } else { d1.length_sq() };
            let next = (t - g / h).clamp(range.lo, range.hi);
            if next == t {
                break;
            }
            let fnext = self.at(next).0.distance_sq(p);
            if fnext > f {
                break;
            }
            (t, f) = (next, fnext);
        }
        t
    }
    fn bbox(&self, range: Interval) -> Box3 {
        Wrap::bbox(self, range)
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(Wrap::transformed(self, f)))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        self.bounds(range).0 * (1.0 + 1e-12)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        // equal steps h keep each chord within |C''| h^2 / 8 of the curve
        let n = range.width() * (self.bounds(range).1 / (8.0 * chord.max(tol::LINEAR))).sqrt();
        (n.ceil() as usize).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::rand_frame;
    use forge_math::rng::Rng;

    fn random_wrap(r: &mut Rng) -> Wrap {
        let flat = if r.bool() {
            Flat::Line {
                a: Point2::new(r.range(-5.0, 5.0), r.range(-5.0, 5.0)),
                b: Point2::new(r.range(-5.0, 5.0), r.range(-5.0, 5.0)),
            }
        } else {
            let lo = r.range(-3.0, 1.0);
            Flat::Parabola {
                curve: Parabola {
                    frame: Frame::WORLD
                        .rotated_about_z(r.range(0.0, 6.0))
                        .with_origin(Point3::new(r.range(-3.0, 3.0), r.range(-3.0, 3.0), 0.0)),
                    focal: r.range(0.2, 3.0),
                },
                range: Interval::new(lo, lo + r.range(0.5, 3.0)),
            }
        };
        let unroll = r.range(4.0, 12.0);
        Wrap {
            axis: rand_frame(r),
            radius: unroll + r.range(-2.0, 2.0),
            unroll,
            flat,
        }
    }

    #[test]
    fn derivatives_match_finite_differences() {
        let mut r = Rng::new(611);
        for _ in 0..300 {
            let w = random_wrap(&mut r);
            let t = r.range(0.0, 1.0);
            let h = 1e-6;
            let (_, d1, d2) = w.at(t);
            let close =
                |a: Vec3, b: Vec3| assert!((a - b).length() < 1e-5 * (1.0 + b.length()), "{a:?} {b:?}");
            close((w.at(t + h).0 - w.at(t - h).0) / (2.0 * h), d1);
            close((w.at(t + h).1 - w.at(t - h).1) / (2.0 * h), d2);
        }
    }

    #[test]
    fn lies_on_its_cylinder_and_boxes_hold_it() {
        let mut r = Rng::new(612);
        for _ in 0..200 {
            let w = random_wrap(&mut r);
            let (a, b) = (r.range(0.0, 1.0), r.range(0.0, 1.0));
            let range = Interval::new(a.min(b), a.max(b));
            let bx = w.bbox(range);
            let speed = w.speed_bound(range);
            for i in 0..=20 {
                let t = range.lo + range.width() * i as f64 / 20.0;
                let (p, d1, _) = w.at(t);
                let l = w.axis.point_to_local(p);
                assert!(((l.x * l.x + l.y * l.y).sqrt() - w.radius.abs()).abs() < 1e-9);
                assert!(bx.contains(p));
                assert!(d1.length() <= speed);
            }
            let t = r.range(0.0, 1.0);
            let p = w.at(t).0;
            assert!(w.at(w.closest(p, Interval::new(0.0, 1.0))).0.distance(p) < 1e-9);
        }
    }
}

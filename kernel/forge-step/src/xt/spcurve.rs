//! An SP-curve (XT Format Reference 5.2.1.8): a 2D B-curve in a surface's parameters, the curve
//! in space it maps to. Exact: points and derivatives come from the surface at the curve's
//! parameters, by the chain rule.

use super::geom::Param;
use forge_geom::CurveGeom;
use forge_geom::curve::{CurveDeriv, Domain1, ProcCurve, ProceduralCurve};
use forge_geom::surface::SurfDeriv;
use forge_geom::{Curve, Surface, SurfaceGeom};
use forge_math::Vec3;
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, Vec2, tol};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct OnSurface {
    s: Param,
    c: forge_geom::spline::Spline,
}

impl OnSurface {
    /// None when the curve's parameters leave the surface's domain anywhere sampled.
    pub(crate) fn curve(s: Param, c: forge_geom::spline::Spline) -> Option<Curve> {
        let me = OnSurface { s, c };
        let d = me.c.domain();
        if let Param::Map(s, _) = &me.s {
            let dom = s.domain();
            let inside = |x: f64, r: &forge_geom::Domain1| {
                r.period.is_some() || (x >= r.lo - 1e-6 && x <= r.hi + 1e-6)
            };
            for k in 0..=8 {
                let p = me.uv(d.lo + d.width() * f64::from(k) / 8.0);
                if !p.x.is_finite() || !p.y.is_finite() || !inside(p.x, &dom.u) || !inside(p.y, &dom.v) {
                    return None;
                }
            }
        }
        Some(Curve::Procedural(ProcCurve(Arc::new(me))))
    }

    /// The mapped parameters: the kernel surface's for `Map`, the file's otherwise.
    fn lin(&self, x: f64, y: f64) -> Vec2 {
        match &self.s {
            Param::Map(_, m) => Vec2::new(m[0][0] * x + m[0][1] * y, m[1][0] * x + m[1][1] * y),
            _ => Vec2::new(x, y),
        }
    }

    fn uv(&self, t: f64) -> Point2 {
        let p = self.c.deriv(t).p;
        let v = self.lin(p.x, p.y);
        Point2::new(v.x, v.y)
    }

    /// The surface's point and derivatives at (mapped) parameters.
    fn sd(&self, uv: Point2) -> SurfDeriv {
        param_deriv(&self.s, uv)
    }
}

/// A file surface's point and derivatives at its (mapped) parameters.
pub(crate) fn param_deriv(s: &Param, uv: Point2) -> SurfDeriv {
    {
        match s {
            Param::Map(s, _) => s.deriv(uv),
            Param::Swept { c, cs, d, k } => {
                let cd = c.deriv(cs * uv.x);
                SurfDeriv {
                    p: cd.p + *d * (k * uv.y),
                    du: cd.d1 * *cs,
                    dv: *d * *k,
                    duu: cd.d2 * (cs * cs),
                    duv: Vec3::ZERO,
                    dvv: Vec3::ZERO,
                }
            }
            Param::Spun { c, cs, p, a } => {
                let cd = c.deriv(cs * uv.x);
                let (c1, c2) = (cd.d1 * *cs, cd.d2 * (cs * cs));
                let r = cd.p - *p;
                let along = *a * a.dot(r);
                let w = r - along;
                let w1 = c1 - *a * a.dot(c1);
                let w2 = c2 - *a * a.dot(c2);
                let (sn, cn) = forge_math::dmath::sin_cos(uv.y);
                let turn = |x: Vec3| x * cn + a.cross(x) * sn;
                let turn_v = |x: Vec3| x * -sn + a.cross(x) * cn;
                SurfDeriv {
                    p: *p + along + turn(w),
                    du: *a * a.dot(c1) + turn(w1),
                    dv: turn_v(w),
                    duu: *a * a.dot(c2) + turn(w2),
                    duv: turn_v(w1),
                    dvv: -turn(w),
                }
            }
        }
    }
}

impl ProceduralCurve for OnSurface {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let c = self.c.deriv(t);
        let q = self.lin(c.p.x, c.p.y);
        let d1 = self.lin(c.d1.x, c.d1.y);
        let d2 = self.lin(c.d2.x, c.d2.y);
        let s = self.sd(Point2::new(q.x, q.y));
        CurveDeriv {
            p: s.p,
            d1: s.du * d1.x + s.dv * d1.y,
            d2: s.duu * (d1.x * d1.x)
                + s.duv * (2.0 * d1.x * d1.y)
                + s.dvv * (d1.y * d1.y)
                + s.du * d2.x
                + s.dv * d2.y,
        }
    }

    fn exact_d1(&self) -> bool {
        true
    }

    fn domain(&self) -> Domain1 {
        let d = self.c.domain();
        Domain1 {
            lo: d.lo,
            hi: d.hi,
            period: None,
        }
    }

    fn closest(&self, p: Point3, range: Interval) -> f64 {
        // the nearest of samples, then Newton on the squared distance
        let n = 64;
        let mut best = (f64::INFINITY, range.lo);
        for k in 0..=n {
            let t = range.lo + range.width() * f64::from(k) / f64::from(n);
            let d = self.point(t).distance_sq(p);
            if d < best.0 {
                best = (d, t);
            }
        }
        let mut t = best.1;
        for _ in 0..20 {
            let d = self.deriv(t);
            let r = d.p - p;
            let g = r.dot(d.d1);
            let h = d.d1.dot(d.d1) + r.dot(d.d2);
            if h.is_nan() || h <= 0.0 {
                break;
            }
            let next = (t - g / h).clamp(range.lo, range.hi);
            if (next - t).abs() <= 1e-15 * (1.0 + t.abs()) {
                t = next;
                break;
            }
            t = next;
        }
        if self.point(t).distance_sq(p) <= best.0 {
            t
        } else {
            best.1
        }
    }

    fn bbox(&self, range: Interval) -> Box3 {
        let bx = self.c.bbox(range);
        if let Param::Map(s, _) = &self.s {
            // the surface's box over the curve's parameter box: the spline's hull holds the curve
            let corners = [
                self.lin(bx.x.lo, bx.y.lo),
                self.lin(bx.x.hi, bx.y.hi),
                self.lin(bx.x.lo, bx.y.hi),
                self.lin(bx.x.hi, bx.y.lo),
            ];
            let (mut u, mut v) = (
                Interval::new(f64::INFINITY, f64::NEG_INFINITY),
                Interval::new(f64::INFINITY, f64::NEG_INFINITY),
            );
            for c in corners {
                u = Interval::new(u.lo.min(c.x), u.hi.max(c.x));
                v = Interval::new(v.lo.min(c.y), v.hi.max(c.y));
            }
            let b = s.bbox(Box2::new(u, v));
            // a surface's box can come back NaN (a cone's, over some spans): the samples' then
            if b.diagonal().is_finite() {
                return b;
            }
        }
        // samples, grown by how far the curve can stray between them at its greatest speed
        let n = 64;
        let w = range.width();
        let pts: Vec<Point3> = (0..=n)
            .map(|k| self.point(range.lo + w * f64::from(k) / f64::from(n)))
            .collect();
        let pad = self.speed_bound(range) * w / f64::from(n);
        Box3::from_points(pts.iter().copied()).grow(pad)
    }

    fn speed_bound(&self, range: Interval) -> f64 {
        let w = range.width();
        1.5 * (0..=32)
            .map(|k| self.deriv(range.lo + w * f64::from(k) / 32.0).d1.length())
            .fold(0.0, f64::max)
    }

    fn segments(&self, range: Interval, chord: f64) -> usize {
        // the turn between samples sets the chords: as many as a circle of the tightest bend needs
        let n = 64;
        let mut tight: f64 = f64::INFINITY;
        let w = range.width();
        for k in 0..=n {
            let d = self.deriv(range.lo + w * f64::from(k) / f64::from(n));
            let s = d.d1.length();
            let curv = d.d1.cross(d.d2).length() / (s * s * s).max(1e-300);
            if curv > 0.0 {
                tight = tight.min(1.0 / curv);
            }
        }
        let len = self.speed_bound(range) * w;
        let r = tight.max(tol::LINEAR);
        let step = 2.0 * (2.0 * r * chord.max(1e-12)).sqrt();
        ((len / step).ceil() as usize).clamp(1, 1 << 16)
    }

    fn on_surface(&self, t: f64) -> Option<(Surface, Point2, Vec2)> {
        let Param::Map(s, _) = &self.s else { return None };
        let c = self.c.deriv(t);
        let q = self.lin(c.p.x, c.p.y);
        Some((s.clone(), Point2::new(q.x, q.y), self.lin(c.d1.x, c.d1.y)))
    }

    fn transformed(&self, f: &Frame) -> ProcCurve {
        let s = match &self.s {
            Param::Map(s, m) => Param::Map(s.transformed(f), *m),
            Param::Swept { c, cs, d, k } => Param::Swept {
                c: c.transformed(f),
                cs: *cs,
                d: f.vec_to_world(*d),
                k: *k,
            },
            Param::Spun { c, cs, p, a } => Param::Spun {
                c: c.transformed(f),
                cs: *cs,
                p: f.point_to_world(*p),
                a: f.vec_to_world(*a),
            },
        };
        ProcCurve(Arc::new(OnSurface { s, c: self.c.clone() }))
    }
}

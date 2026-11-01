//! A spline turned about an axis: S(u, v) = F(r(v) cos u, r(v) sin u, h(v)), the section C = (r, h)
//! in its frame's xy, r >= 0. The implicit value is the signed distance to C in the half plane
//! through p and the axis.

use super::extruded::{carried, foot, section_radius, world_box};
use super::{Spline, SplineCurve};
use crate::curve::Domain1;
use crate::screw::{fresh_id, remembered};
use crate::surface::{Domain2, Proc, Procedural, ShapeBounds, SurfDeriv, Surface};
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, Vec2, Vec3, dmath, tol};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Spun {
    frame: Frame,
    c: Arc<Spline>,
    ext: Interval,
    id: u64,
    radius: f64,
    rmax: f64,
    speed: f64,
    /// Whether a point of the section (carried on) lies on the axis.
    poles: bool,
}

impl Spun {
    /// The surface of C about the frame's z; None where C is not regular or crosses the axis.
    pub fn new(frame: Frame, c: Arc<Spline>) -> Option<Spun> {
        let dom = c.domain();
        let mut ext = carried(&c)?;
        // carried on only where it stays off the axis' far side
        let hits = |e: Interval| c.bbox(e).x.lo < -tol::LINEAR;
        if c.plane_roots(Vec3::X, 0.0, Interval::new(dom.lo + 1e-9, dom.hi - 1e-9), 0.0)
            .iter()
            .any(|r| !r.1)
        {
            return None;
        }
        for k in [1.0, 0.5, 0.25, 0.0] {
            let e = Interval::new(dom.lo - k * (dom.lo - ext.lo), dom.hi + k * (ext.hi - dom.hi));
            ext = e;
            if !hits(e) {
                break;
            }
        }
        let (a, b) = c.ends();
        let poles = a.x.abs() <= tol::LINEAR || b.x.abs() <= tol::LINEAR;
        let rmax = c.bbox(dom).x.hi.max(0.0);
        let speed = c.derivative_bounds(dom)[0];
        let radius = {
            let mut k: f64 = 1.0 / section_radius(&c);
            let br = c.breaks();
            for i in 0..c.pieces() {
                for j in 0..=8 {
                    let v = br[i] + (br[i + 1] - br[i]) * j as f64 / 8.0;
                    let d = c.deriv(v);
                    if d.p.x > tol::LINEAR {
                        let l = d.d1.length().max(1e-300);
                        k = k.max(d.d1.y.abs() / (l * d.p.x));
                    }
                }
            }
            if k > 0.0 { 1.0 / k } else { f64::INFINITY }
        };
        Some(Spun {
            frame,
            c,
            ext,
            id: fresh_id(),
            radius,
            rmax,
            speed,
            poles,
        })
    }
    /// The surface reflected, `frame` being the mirror image of its frame with y negated: every
    /// angle runs the other way, the section and the sense of its normal stay.
    pub fn reflected(&self, frame: Frame) -> Spun {
        let mut s = self.clone();
        s.id = fresh_id();
        s.frame = frame;
        s
    }
    pub fn surface(self) -> Surface {
        Surface::Procedural(Proc(Arc::new(self)))
    }
    pub fn frame(&self) -> &Frame {
        &self.frame
    }
    pub fn section(&self) -> &Spline {
        &self.c
    }
    pub fn reach(&self) -> Interval {
        self.ext
    }
    fn at(&self, uv: Point2) -> SurfDeriv {
        let d = self.c.deriv(uv.y);
        let (s, c) = dmath::sin_cos(uv.x);
        let f = &self.frame;
        let (ex, ey, ez) = (f.x().get(), f.y().get(), f.z().get());
        let radial = ex * c + ey * s;
        let round = ey * c - ex * s;
        SurfDeriv {
            p: f.origin + radial * d.p.x + ez * d.p.y,
            du: round * d.p.x,
            dv: radial * d.d1.x + ez * d.d1.y,
            duu: radial * (-d.p.x),
            duv: round * d.d1.x,
            dvv: radial * d.d2.x + ez * d.d2.y,
        }
    }
    /// (rho, h) of p, and its angle.
    fn meridian(&self, p: Point3) -> (Point3, f64) {
        let l = self.frame.point_to_local(p);
        (
            Point3::new((l.x * l.x + l.y * l.y).sqrt(), l.z, 0.0),
            dmath::atan2(l.y, l.x),
        )
    }
    fn value_from(&self, p: Point3, near: Option<f64>) -> (f64, Vec3, Point2) {
        let (q, a) = self.meridian(p);
        let (t, f, n) = foot(&self.c, self.ext, q, near);
        let (s, c) = dmath::sin_cos(a);
        let (ex, ey, ez) = (self.frame.x().get(), self.frame.y().get(), self.frame.z().get());
        let g = (ex * c + ey * s) * n.x + ez * n.y;
        (f, g, Point2::new(a, t))
    }
    fn value(&self, p: Point3) -> (f64, Vec3) {
        remembered(self.id, p, || {
            let (f, g, _) = self.value_from(p, None);
            (f, g)
        })
    }
}

impl crate::loftmeet::Span for Spun {
    fn at(&self, uv: Point2) -> SurfDeriv {
        Spun::at(self, uv)
    }
    fn carried(&self) -> Box2 {
        Box2::new(Interval::new(-dmath::TAU, 2.0 * dmath::TAU), self.ext)
    }
    /// A quarter turn by each Bezier piece of the section.
    fn pieces(&self) -> Vec<Box2> {
        let spans = self.c.spans_over(self.ext);
        let mut out = Vec::with_capacity(4 * spans.len());
        for v in spans {
            for k in 0..4 {
                let q = dmath::FRAC_PI_2;
                out.push(Box2::new(Interval::new(k as f64 * q, (k + 1) as f64 * q), v));
            }
        }
        out
    }
    fn patch(&self, uv: Box2) -> Option<crate::loftmeet::Patch> {
        let row = self.c.control_over(uv.v);
        let f = &self.frame;
        let (o, ex, ey, ez) = (f.origin.to_vec(), f.x().get(), f.y().get(), f.z().get());
        let half = 0.5 * uv.u.width();
        let mid_w = dmath::cos(half);
        if mid_w <= 0.0 {
            return None;
        }
        let dir = |a: f64| {
            let (s, c) = dmath::sin_cos(a);
            (c, s)
        };
        // the arc's three homogeneous points: its ends, and the middle direction at weight cos(half)
        let arc = [(dir(uv.u.lo), 1.0), (dir(uv.u.mid()), mid_w), (dir(uv.u.hi), 1.0)];
        let mut pts = Vec::with_capacity(3 * row.len());
        for h in &row {
            for ((c, s), wa) in arc {
                // (r c, r s) stands for the point at weight wa only at the ends; in the middle
                // the unit direction over cos(half) at weight cos(half) has these coordinates
                let w = h[3] * wa;
                let q = o * w + (ex * c + ey * s) * h[0] + ez * (h[1] * wa);
                pts.push([q.x, q.y, q.z, w]);
            }
        }
        Some(crate::loftmeet::Patch::Curve(super::CurveHull {
            nu: 3,
            nv: row.len(),
            pts,
        }))
    }
}

impl Procedural for Spun {
    fn spline_wall(&self) -> bool {
        true
    }
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        self.at(uv)
    }
    fn exact_d1(&self) -> bool {
        true
    }
    fn regular(&self) -> bool {
        !self.poles
    }
    fn direct(&self) -> bool {
        true
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    fn normal(&self, uv: Point2) -> Option<forge_math::UnitVec3> {
        let d = self.at(uv);
        let n = d.du.cross(d.dv).normalize();
        if n.is_some() && self.c.point(uv.y).x > tol::LINEAR {
            return n;
        }
        None
    }
    fn domain(&self) -> Domain2 {
        let d = self.c.domain();
        Domain2 {
            u: Domain1::ANGLE,
            v: Domain1 {
                lo: d.lo,
                hi: d.hi,
                period: None,
            },
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        let (q, a) = self.meridian(p);
        Point2::new(a, self.c.closest(q, self.ext))
    }
    fn project_near(&self, p: Point3, near: Point2) -> Point2 {
        let (q, a) = self.meridian(p);
        let t = self
            .c
            .closest_near(q, near.y, self.ext)
            .unwrap_or_else(|| self.c.closest(q, self.ext));
        Point2::new(a, t)
    }
    fn value_near(&self, p: Point3, uv: Option<Point2>) -> Option<(f64, Vec3, Point2)> {
        let (f, g, q) = self.value_from(p, uv.map(|u| u.y));
        Some((f, g, q))
    }
    /// A section edge at one angle runs straight in v.
    fn curve_uv(&self, c: &crate::curve::ProcCurve) -> Option<(Point2, Vec2)> {
        let s = c.as_any()?.downcast_ref::<SplineCurve>()?;
        if s.c.breaks() != self.c.breaks() || self.poles {
            return None;
        }
        let (a, _) = s.c.ends();
        let (_, ang) = self.meridian(a);
        let (sn, cs) = dmath::sin_cos(ang);
        let f = &self.frame;
        let (ex, ey, ez) = (f.x().get(), f.y().get(), f.z().get());
        let radial = ex * cs + ey * sn;
        let want = self.c.mapped(|q| f.origin + radial * q.x + ez * q.y);
        (0..s.c.pieces())
            .all(|i| {
                let (x, y) = (want.row(i), s.c.row(i));
                x.iter()
                    .zip(y)
                    .all(|(p, q)| (0..4).all(|k| (p[k] - q[k]).abs() <= 1e-11 * (1.0 + p[k].abs())))
            })
            .then_some((Point2::new(ang, 0.0), Vec2::new(0.0, 1.0)))
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        let b = self.c.bbox(uv.v);
        let r = Interval::new(b.x.lo.max(0.0), b.x.hi.max(0.0));
        let (cu, su) = (uv.u.cos(), uv.u.sin());
        let times = |a: Interval, b: Interval| {
            let (p, q, s, t) = (a.lo * b.lo, a.lo * b.hi, a.hi * b.lo, a.hi * b.hi);
            Interval::new(p.min(q).min(s).min(t), p.max(q).max(s).max(t))
        };
        world_box(
            &self.frame,
            &Box3 {
                x: times(r, cu),
                y: times(r, su),
                z: b.y,
            },
        )
        .grow(tol::LINEAR)
    }
    fn implicit(&self, p: Point3) -> f64 {
        self.value(p).0
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        self.value(p).1
    }
    fn implicit_gradient(&self, p: Point3) -> (f64, Vec3) {
        self.value(p)
    }
    fn transformed(&self, f: &Frame) -> Proc {
        let mut s = self.clone();
        s.id = fresh_id();
        s.frame = f.compose(&self.frame);
        Proc(Arc::new(s))
    }
    fn uv_scale(&self) -> (f64, f64) {
        (self.rmax.max(tol::LINEAR), self.speed.max(tol::LINEAR))
    }
    fn mesh_radius(&self) -> f64 {
        self.radius
    }
    fn shape_bounds(&self, uv: Box2) -> Option<ShapeBounds> {
        let [c1, c2, c3] = self.c.derivative_bounds(uv.v);
        let b = self.c.bbox(uv.v);
        let r = b.x.lo.abs().max(b.x.hi.abs());
        Some(ShapeBounds::taylor(
            &self.at(Point2::new(uv.u.mid(), uv.v.mid())),
            (0.5 * uv.u.width(), 0.5 * uv.v.width()),
            [r, c1, c2],
            [r, c1, c2, c3],
        ))
    }
    fn u_kinks(&self, _u: Interval) -> Vec<f64> {
        Vec::new()
    }
}

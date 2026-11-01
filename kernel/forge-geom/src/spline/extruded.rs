//! A spline swept straight: S(u, v) = F(C(u) + v z), C a curve in the frame's xy plane. The implicit
//! value is the signed distance to C in that plane, whatever the height.

use super::{Spline, SplineCurve};
use crate::screw::{fresh_id, remembered};
use crate::surface::{Domain2, Proc, Procedural, ShapeBounds, SurfDeriv, Surface};
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, Vec2, Vec3, tol};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Extruded {
    frame: Frame,
    c: Arc<Spline>,
    /// C's parameter carried past its ends as far as it stays regular.
    ext: Interval,
    height: Interval,
    whole: Box3,
    id: u64,
    radius: f64,
    speed: f64,
}

/// The foot of q on section c over `ext`: the parameter, the signed distance (positive to the right
/// of the curve's direction), and the unit direction of that distance.
pub(super) fn foot(c: &Spline, ext: Interval, q: Point3, near: Option<f64>) -> (f64, f64, Vec2) {
    let t = near
        .and_then(|n| c.closest_near(q, n, ext))
        .unwrap_or_else(|| c.closest(q, ext));
    let d = c.deriv(t);
    let e = Vec2::new(q.x - d.p.x, q.y - d.p.y);
    let n = Vec2::new(d.d1.y, -d.d1.x).normalize().unwrap_or(Vec2::X);
    let along = e.dot(n);
    // past the carried-on end the distance is to that end, so no false zero runs on along it
    if (t <= ext.lo || t >= ext.hi) && e.length() > along.abs() + tol::LINEAR {
        let l = e.length();
        let s = if along < 0.0 { -1.0 } else { 1.0 };
        return (t, s * l, e / (s * l));
    }
    (t, along, n)
}

/// How far past each end a section may be carried and stay regular.
pub(super) fn carried(c: &Spline) -> Option<Interval> {
    let d = c.domain();
    if !c.regular_over(d) {
        return None;
    }
    let b = c.breaks();
    let n = c.pieces();
    let (h0, h1) = (b[1] - b[0], b[n] - b[n - 1]);
    for k in [0.25, 0.125, 0.0625, 0.03125] {
        let e = Interval::new(d.lo - k * h0, d.hi + k * h1);
        if c.regular_over(e) {
            return Some(e);
        }
    }
    Some(d)
}

/// The box in the frame's world round a box in its coordinates.
pub(super) fn world_box(f: &Frame, b: &Box3) -> Box3 {
    let (c, hx, hy, hz) = (
        f.point_to_world(b.center()),
        0.5 * b.x.width(),
        0.5 * b.y.width(),
        0.5 * b.z.width(),
    );
    let (x, y, z) = (f.x().get(), f.y().get(), f.z().get());
    let h = Vec3::new(
        hx * x.x.abs() + hy * y.x.abs() + hz * z.x.abs(),
        hx * x.y.abs() + hy * y.y.abs() + hz * z.y.abs(),
        hx * x.z.abs() + hy * y.z.abs() + hz * z.z.abs(),
    );
    Box3 {
        x: Interval::new(c.x - h.x, c.x + h.x),
        y: Interval::new(c.y - h.y, c.y + h.y),
        z: Interval::new(c.z - h.z, c.z + h.z),
    }
}

/// About the tightest radius of curvature of a section, by samples.
pub(super) fn section_radius(c: &Spline) -> f64 {
    let b = c.breaks();
    let mut k: f64 = 0.0;
    for i in 0..c.pieces() {
        for j in 0..=8 {
            let d = c.deriv(b[i] + (b[i + 1] - b[i]) * j as f64 / 8.0);
            let l = d.d1.length();
            k = k.max(d.d1.cross(d.d2).length() / (l * l * l).max(1e-300));
        }
    }
    if k > 0.0 { 1.0 / k } else { f64::INFINITY }
}

impl Extruded {
    /// The wall over C(u) between the heights in `height`, carried on a height past each end. None
    /// where C has a stationary point or a cusp.
    pub fn new(frame: Frame, c: Arc<Spline>, height: Interval) -> Option<Extruded> {
        let ext = carried(&c)?;
        let radius = section_radius(&c);
        let speed = c.derivative_bounds(c.domain())[0];
        let h = height.width();
        Some(Extruded {
            frame,
            whole: c.bbox(ext),
            c,
            ext,
            height: Interval::new(height.lo - h, height.hi + h),
            id: fresh_id(),
            radius,
            speed,
        })
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
    pub fn section_arc(&self) -> &Arc<Spline> {
        &self.c
    }
    pub fn reach(&self) -> Interval {
        self.ext
    }
    /// The bottom-up curve of the section at height v, in world coordinates.
    pub fn at_height(&self, v: f64) -> Spline {
        let f = self.frame.with_origin(self.frame.origin + self.frame.z() * v);
        self.c.mapped(|p| f.point_to_world(p))
    }
    fn at(&self, uv: Point2) -> SurfDeriv {
        let d = self.c.deriv(uv.x);
        let f = &self.frame;
        SurfDeriv {
            p: f.origin + f.x() * d.p.x + f.y() * d.p.y + f.z() * uv.y,
            du: f.x() * d.d1.x + f.y() * d.d1.y,
            dv: f.z().get(),
            duu: f.x() * d.d2.x + f.y() * d.d2.y,
            duv: Vec3::ZERO,
            dvv: Vec3::ZERO,
        }
    }
    fn local(&self, p: Point3) -> (Point3, f64) {
        let l = self.frame.point_to_local(p);
        (Point3::new(l.x, l.y, 0.0), l.z)
    }
    fn value(&self, p: Point3) -> (f64, Vec3) {
        remembered(self.id, p, || {
            let (q, _) = self.local(p);
            let (_, f, n) = foot(&self.c, self.ext, q, None);
            (f, self.frame.x() * n.x + self.frame.y() * n.y)
        })
    }
}

impl crate::loftmeet::Span for Extruded {
    fn at(&self, uv: Point2) -> SurfDeriv {
        Extruded::at(self, uv)
    }
    fn carried(&self) -> Box2 {
        Box2::new(self.ext, self.height)
    }
    fn pieces(&self) -> Vec<Box2> {
        self.c
            .spans_over(self.ext)
            .into_iter()
            .map(|u| Box2::new(u, self.height))
            .collect()
    }
    fn patch(&self, uv: Box2) -> Option<crate::loftmeet::Patch> {
        let row = self.c.control_over(uv.u);
        let f = &self.frame;
        let (o, x, y, z) = (f.origin.to_vec(), f.x().get(), f.y().get(), f.z().get());
        let mut pts = Vec::with_capacity(2 * row.len());
        for v in [uv.v.lo, uv.v.hi] {
            for h in &row {
                let q = o * h[3] + x * h[0] + y * h[1] + z * (v * h[3]);
                pts.push([q.x, q.y, q.z, h[3]]);
            }
        }
        Some(crate::loftmeet::Patch::Curve(super::CurveHull {
            nu: row.len(),
            nv: 2,
            pts,
        }))
    }
}

impl Procedural for Extruded {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        self.at(uv)
    }
    fn point(&self, uv: Point2) -> Point3 {
        let q = self.c.point(uv.x);
        let f = &self.frame;
        f.origin + f.x() * q.x + f.y() * q.y + f.z() * uv.y
    }
    fn exact_d1(&self) -> bool {
        true
    }
    fn regular(&self) -> bool {
        true
    }
    fn v_degree(&self) -> Option<usize> {
        Some(1)
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    fn domain(&self) -> Domain2 {
        let d = self.c.domain();
        Domain2 {
            u: crate::curve::Domain1 {
                lo: d.lo,
                hi: d.hi,
                period: None,
            },
            v: crate::curve::Domain1 {
                lo: self.height.lo,
                hi: self.height.hi,
                period: None,
            },
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        let (q, z) = self.local(p);
        Point2::new(self.c.closest(q, self.ext), z)
    }
    fn project_near(&self, p: Point3, near: Point2) -> Point2 {
        let (q, z) = self.local(p);
        let t = self
            .c
            .closest_near(q, near.x, self.ext)
            .unwrap_or_else(|| self.c.closest(q, self.ext));
        Point2::new(t, z)
    }
    fn value_near(&self, p: Point3, uv: Option<Point2>) -> Option<(f64, Vec3, Point2)> {
        let (q, z) = self.local(p);
        let (t, f, n) = foot(&self.c, self.ext, q, uv.map(|u| u.x));
        Some((f, self.frame.x() * n.x + self.frame.y() * n.y, Point2::new(t, z)))
    }
    /// A section edge at one height is drawn straight from its parameter.
    fn curve_uv(&self, c: &crate::curve::ProcCurve) -> Option<(Point2, Vec2)> {
        let s = c.as_any()?.downcast_ref::<SplineCurve>()?;
        let same = if Arc::ptr_eq(&s.c, &self.c) {
            Some(0.0)
        } else {
            let (a, _) = s.c.ends();
            let v = self.local(a).1;
            let want = self.at_height(v);
            (want.breaks() == s.c.breaks()
                && (0..s.c.pieces()).all(|i| {
                    let (x, y) = (want.row(i), s.c.row(i));
                    x.iter()
                        .zip(y)
                        .all(|(p, q)| (0..4).all(|k| (p[k] - q[k]).abs() <= 1e-11 * (1.0 + p[k].abs())))
                }))
            .then_some(v)
        }?;
        Some((Point2::new(0.0, same), Vec2::new(1.0, 0.0)))
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        let b = self.c.bbox(uv.u);
        world_box(
            &self.frame,
            &Box3 {
                x: b.x,
                y: b.y,
                z: uv.v,
            },
        )
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
    fn within(&self, p: Point3, r: f64) -> bool {
        let (q, _) = self.local(p);
        let g = |i: Interval, x: f64| (i.lo - x).max(x - i.hi).max(0.0);
        let (x, y) = (g(self.whole.x, q.x), g(self.whole.y, q.y));
        if x * x + y * y > r * r {
            return false;
        }
        self.implicit(p).abs() <= r
    }
    fn transformed(&self, f: &Frame) -> Proc {
        let mut s = self.clone();
        s.id = fresh_id();
        s.frame = f.compose(&self.frame);
        Proc(Arc::new(s))
    }
    fn uv_scale(&self) -> (f64, f64) {
        (self.speed.max(tol::LINEAR), 1.0)
    }
    fn mesh_radius(&self) -> f64 {
        self.radius
    }
    fn min_radius(&self) -> Option<f64> {
        None
    }
    fn shape_bounds(&self, uv: Box2) -> Option<ShapeBounds> {
        let [_, c2, c3] = self.c.derivative_bounds(uv.u);
        let mid = Point2::new(uv.u.mid(), uv.v.mid());
        Some(ShapeBounds::taylor(
            &self.at(mid),
            (0.5 * uv.u.width(), 0.5 * uv.v.width()),
            [c2, 0.0, 0.0],
            [c3, 0.0, 0.0, 0.0],
        ))
    }
    fn u_kinks(&self, u: Interval) -> Vec<f64> {
        self.c.knots_in(u)
    }
}

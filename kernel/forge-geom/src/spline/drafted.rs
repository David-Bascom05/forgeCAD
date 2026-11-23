//! A spline wall drafted about a neutral plane across its sweep, S(u, v) = F(C(u) + (v - v0) k
//! N(u) + v z): C a curve in the frame's xy plane, N its unit normal to the right of its direction, k the
//! lean (tan of the draft, signed). Each section is a parallel curve of C, so a point's foot on C is
//! its foot on the section at its height: the implicit value is the signed distance to C less the
//! section's offset, scaled to the surface's normal, exactly. Regular while |(v - v0) k| stays under
//! the radius of curvature on the side it moves to (the caller's to keep: a taper's cusp bound).

use super::extruded::{carried, foot, section_radius, world_box};
use super::{Spline, SplineCurve};
use crate::curve::{CurveDeriv, Domain1, ProcCurve, ProceduralCurve};
use crate::screw::{fresh_id, normal_turn, remembered};
use crate::surface::{Domain2, Proc, Procedural, SurfDeriv, Surface};
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, Vec2, Vec3, tol};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Drafted {
    frame: Frame,
    c: Arc<Spline>,
    ext: Interval,
    height: Interval,
    /// The neutral plane's height, where the wall is C's own.
    v0: f64,
    /// The section's offset per unit of height from v0, along N.
    lean: f64,
    whole: Box3,
    id: u64,
    radius: f64,
    speed: f64,
}

/// N and its first two derivatives in u (2-D, in the frame's xy), from C's first three.
fn normals(d1: Vec3, d2: Vec3, d3: Vec3) -> [Vec3; 3] {
    let r = |v: Vec3| Vec3::new(v.y, -v.x, 0.0);
    let l2 = d1.x * d1.x + d1.y * d1.y;
    let l = l2.sqrt();
    let (l3, l5) = (l2 * l, l2 * l2 * l);
    let a = d1.x * d2.x + d1.y * d2.y;
    let b = d2.x * d2.x + d2.y * d2.y + d1.x * d3.x + d1.y * d3.y;
    let n = r(d1) / l;
    let n1 = r(d2) / l - r(d1) * (a / l3);
    let n2 = r(d3) / l - r(d2) * (2.0 * a / l3) - r(d1) * (b / l3 - 3.0 * a * a / l5);
    [n, n1, n2]
}

impl Drafted {
    /// C swept along the frame's z over `height` (carried on a height past each end, as an
    /// extruded wall is), drafted about height `v0` with `lean`. None where C has a stationary
    /// point or a cusp.
    pub fn new(frame: Frame, c: Arc<Spline>, height: Interval, v0: f64, lean: f64) -> Option<Drafted> {
        // (not carried past its ends: a point moved in along the ruling at an end has its foot
        // there, and a carried tail curls away from the neighbour the section goes on to)
        carried(&c)?;
        let ext = c.domain();
        let h = height.width();
        let height = Interval::new(height.lo - h, height.hi + h);
        let reach = lean.abs() * (height.lo - v0).abs().max((height.hi - v0).abs());
        let b = c.bbox(ext);
        let whole = Box3 {
            x: b.x.grow(reach),
            y: b.y.grow(reach),
            z: b.z,
        };
        Some(Drafted {
            frame,
            radius: section_radius(&c),
            speed: c.derivative_bounds(c.domain())[0],
            whole,
            c,
            ext,
            height,
            v0,
            lean,
            id: fresh_id(),
        })
    }
    pub fn surface(self) -> Surface {
        Surface::Procedural(Proc(Arc::new(self)))
    }
    pub fn section(&self) -> &Spline {
        &self.c
    }
    pub fn lean(&self) -> f64 {
        self.lean
    }
    pub fn frame(&self) -> &Frame {
        &self.frame
    }
    /// The section at height `v` (the frame's z): C's parallel curve there, exactly, over C's
    /// parameter.
    pub fn section_at(&self, v: f64) -> crate::Curve {
        crate::Curve::Procedural(ProcCurve(Arc::new(Section {
            frame: self.frame,
            c: self.c.clone(),
            v,
            w: (v - self.v0) * self.lean,
            dom: self.c.domain(),
            radius: self.radius,
        })))
    }
    fn at(&self, uv: Point2) -> SurfDeriv {
        let (d, d3) = self.c.deriv3(uv.x);
        let [n, n1, n2] = normals(d.d1, d.d2, d3);
        let w = (uv.y - self.v0) * self.lean;
        let f = &self.frame;
        let world = |v: Vec3| f.x() * v.x + f.y() * v.y;
        SurfDeriv {
            p: f.origin + world(d.p.to_vec() + n * w) + f.z() * uv.y,
            du: world(d.d1 + n1 * w),
            dv: f.z().get() + world(n * self.lean),
            duu: world(d.d2 + n2 * w),
            duv: world(n1 * self.lean),
            dvv: Vec3::ZERO,
        }
    }
    fn local(&self, p: Point3) -> (Point3, f64) {
        let l = self.frame.point_to_local(p);
        (Point3::new(l.x, l.y, 0.0), l.z)
    }
    /// The signed distance to the section at p's height, along the surface's normal, and that
    /// normal; and the foot's parameter.
    fn value_at(&self, p: Point3, near: Option<f64>) -> (f64, Vec3, Point2) {
        let (q, z) = self.local(p);
        let (t, f, n) = foot(&self.c, self.ext, q, near);
        let k = 1.0 / (1.0 + self.lean * self.lean).sqrt();
        let g = (self.frame.x() * n.x + self.frame.y() * n.y - self.frame.z() * self.lean) * k;
        ((f - (z - self.v0) * self.lean) * k, g, Point2::new(t, z))
    }
    fn value(&self, p: Point3) -> (f64, Vec3) {
        remembered(self.id, p, || {
            let (f, g, _) = self.value_at(p, None);
            (f, g)
        })
    }
    /// How far the section moves from C over heights `v`.
    fn moved(&self, v: Interval) -> f64 {
        self.lean.abs() * (v.lo - self.v0).abs().max((v.hi - self.v0).abs())
    }
}

impl Procedural for Drafted {
    fn kind(&self) -> &'static str {
        "drafted spline"
    }
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        self.at(uv)
    }
    fn exact_d1(&self) -> bool {
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
    fn gradient_turn(&self, p: Point3, v: Vec3) -> Option<Vec3> {
        let q = self.project(p);
        (q.x > self.ext.lo && q.x < self.ext.hi)
            .then(|| normal_turn(&Procedural::deriv(self, q), p, v))
            .flatten()
    }
    fn project(&self, p: Point3) -> Point2 {
        self.value_at(p, None).2
    }
    fn project_near(&self, p: Point3, near: Point2) -> Point2 {
        self.value_at(p, Some(near.x)).2
    }
    fn value_near(&self, p: Point3, uv: Option<Point2>) -> Option<(f64, Vec3, Point2)> {
        Some(self.value_at(p, uv.map(|u| u.x)))
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        let b = self.c.bbox(uv.u);
        let m = self.moved(uv.v);
        world_box(
            &self.frame,
            &Box3 {
                x: b.x.grow(m),
                y: b.y.grow(m),
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
        (self.speed.max(tol::LINEAR), (1.0 + self.lean * self.lean).sqrt())
    }
    fn mesh_radius(&self) -> f64 {
        (self.radius - self.moved(self.height)).max(self.radius * 0.1)
    }
    fn min_radius(&self) -> Option<f64> {
        None
    }
    fn u_kinks(&self, u: Interval) -> Vec<f64> {
        self.c.knots_in(u)
    }
    /// A section edge (or the spline itself, at the neutral plane) is drawn straight from its
    /// parameter.
    fn curve_uv(&self, c: &ProcCurve) -> Option<(Point2, Vec2)> {
        let any = c.as_any()?;
        let v = if let Some(s) = any.downcast_ref::<Section>() {
            (Arc::ptr_eq(&s.c, &self.c) && s.frame == self.frame && s.w == (s.v - self.v0) * self.lean)
                .then_some(s.v)?
        } else {
            let s = any.downcast_ref::<SplineCurve>()?;
            let (a, _) = s.c.ends();
            let v = self.local(a).1;
            let same = s.c.breaks() == self.c.breaks()
                && (0..s.c.pieces()).all(|i| {
                    let (x, y) = (self.c.row(i), s.c.row(i));
                    x.iter().zip(y).all(|(p, q)| {
                        let w = self
                            .frame
                            .point_to_world(Point3::new(p[0] / p[3], p[1] / p[3], v));
                        let o = Point3::new(q[0] / q[3], q[1] / q[3], q[2] / q[3]);
                        w.distance(o) <= 1e-11 * (1.0 + w.to_vec().abs_max())
                    })
                });
            (same && (v - self.v0).abs() * self.lean.abs() <= 0.0).then_some(v)?
        };
        Some((Point2::new(0.0, v), Vec2::new(1.0, 0.0)))
    }
}

/// A drafted wall's section at one height: `F(C(t) + w N(t) + v z)`, exact, over C's parameter.
#[derive(Debug)]
struct Section {
    frame: Frame,
    c: Arc<Spline>,
    v: f64,
    w: f64,
    dom: Interval,
    radius: f64,
}

impl Section {
    fn world(&self, v: Vec3) -> Vec3 {
        self.frame.x() * v.x + self.frame.y() * v.y
    }
    /// How much faster than C it runs at most: |1 - w κ| with κ under 1 / (half the radius
    /// C's samples bend by).
    fn stretch(&self) -> f64 {
        1.0 + self.w.abs() / (0.5 * self.radius).max(1e-300)
    }
}

impl ProceduralCurve for Section {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let (d, d3) = self.c.deriv3(t);
        let [n, n1, n2] = normals(d.d1, d.d2, d3);
        CurveDeriv {
            p: self.frame.origin + self.world(d.p.to_vec() + n * self.w) + self.frame.z() * self.v,
            d1: self.world(d.d1 + n1 * self.w),
            d2: self.world(d.d2 + n2 * self.w),
        }
    }
    fn domain(&self) -> Domain1 {
        Domain1 {
            lo: self.dom.lo,
            hi: self.dom.hi,
            period: None,
        }
    }
    /// The foot on C is the foot on its parallel curve.
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let l = self.frame.point_to_local(p);
        self.c.closest(Point3::new(l.x, l.y, 0.0), range)
    }
    fn bbox(&self, range: Interval) -> Box3 {
        let b = self.c.bbox(range);
        let w = self.w.abs();
        world_box(
            &self.frame,
            &Box3 {
                x: b.x.grow(w),
                y: b.y.grow(w),
                z: Interval::new(self.v, self.v),
            },
        )
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(Section {
            frame: f.compose(&self.frame),
            c: self.c.clone(),
            v: self.v,
            w: self.w,
            dom: self.dom,
            radius: self.radius,
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        self.c.derivative_bounds(range)[0] * self.stretch()
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        let bounds = self.c.derivative_bounds(range);
        let m = (bounds[1] + bounds[0] * bounds[0] / (0.5 * self.radius).max(1e-300)) * self.stretch();
        let w = range.width();
        ((m * w * w / (8.0 * chord.max(1e-12))).sqrt().ceil() as usize).max(self.c.knots_in(range).len() + 1)
    }
    fn kinks(&self, range: Interval) -> Vec<f64> {
        self.c.knots_in(range)
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    fn exact_d1(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The derivatives against differences of the points, and the implicit value zero on the
    /// surface with its gradient along the normal.
    #[test]
    fn derivatives_and_value_agree() {
        let pts = [(0.0, 0.0), (1.0, 2.0), (3.0, 2.5), (4.0, 0.5), (6.0, 1.0)];
        let c = Spline::through(
            &pts.iter()
                .map(|&(x, y)| Point3::new(x, y, 0.0))
                .collect::<Vec<_>>(),
            None,
            None,
        )
        .expect("a spline");
        let s = Drafted::new(Frame::WORLD, Arc::new(c), Interval::new(0.0, 2.0), 0.0, -0.3).unwrap();
        let h = 1e-6;
        for &(u, v) in &[(0.3, 0.5), (1.1, 1.7), (1.9, 0.1)] {
            let d = s.deriv(Point2::new(u, v));
            let du = (s.deriv(Point2::new(u + h, v)).p - s.deriv(Point2::new(u - h, v)).p) / (2.0 * h);
            let dv = (s.deriv(Point2::new(u, v + h)).p - s.deriv(Point2::new(u, v - h)).p) / (2.0 * h);
            let duu = (s.deriv(Point2::new(u + h, v)).du - s.deriv(Point2::new(u - h, v)).du) / (2.0 * h);
            let duv = (s.deriv(Point2::new(u, v + h)).du - s.deriv(Point2::new(u, v - h)).du) / (2.0 * h);
            assert!((d.du - du).length() < 1e-6, "du");
            assert!((d.dv - dv).length() < 1e-6, "dv");
            assert!((d.duu - duu).length() < 1e-5, "duu");
            assert!((d.duv - duv).length() < 1e-5, "duv");
            assert!(s.implicit(d.p).abs() < 1e-12);
            let n = d.du.cross(d.dv).normalize().unwrap();
            assert!((s.gradient(d.p) - n.get()).length() < 1e-9);
            let q = s.project(d.p);
            assert!((q.x - u).abs() < 1e-9 && (q.y - v).abs() < 1e-12);
        }
    }
}

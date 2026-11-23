//! Uniform scaling, a similarity: p ↦ c + k (p − c), k > 0. It maps every kind of curve and surface
//! to one of its own kind with its lengths times k and its angles kept, so it is done per kind from
//! the kind's own data, as a rigid motion is (`transformed`), never by fitting:
//!
//! * a plane, cylinder, cone, sphere or torus keeps its frame's axes; the origin is scaled, and the
//!   radii (a cone's radius at v = 0, a torus's both) times k. A cone keeps its half-angle.
//! * a line keeps its direction; a circle, ellipse, parabola (focal length) or hyperbola (a, b) its
//!   frame's axes, the lengths times k. A parameter that is a length (along a line, a parabola's
//!   t, a plane's u and v, a height along a cylinder's or cone's axis) is times k: `curve` gives
//!   the edge's range with the curve.
//! * an intersection curve is the same chain on the scaled surfaces: points scaled, unit tangents
//!   kept, the chain's sag times k, its parameter (chain index) kept.
//! * a spline curve has its homogeneous control points scaled (`Spline::scaled_about`), and a
//!   spline wall (extruded or spun) its section's, in its frame, as a rigid motion maps them
//!   (`Extruded::scaled`, `Spun::scaled`); weights and knots kept: a rational Bezier piece of the
//!   scaled points is the scaled piece.
//! * any other procedural kind (a blend, a pipe, a patch, ..) has as yet no form of its own and is
//!   wrapped (`Grown`): its points are the original's scaled, exactly, and its own parameters
//!   are kept.
//!
//! A body (`scaled`) keeps the same faces,
//! edges, vertices, loops and shells, with the same names and the same senses, each on the image of
//! its surface or curve. No reflection, so no loop is turned.
//!
//! Tolerances. A vertex's or an edge's tolerance is a length (how far apart its curves or faces may
//! be), so a gap of g becomes k g. The rule: tol' = max(LINEAR, k tol) where tol > LINEAR, and LINEAR
//! stays LINEAR. LINEAR is the kernel's floor, the tolerance of a vertex settled to the precision of
//! its arithmetic, whose gaps are rounding and do not grow with k; a tolerance above it was measured
//! (a boolean that could not settle the vertex, a file's deviation) and scales with what it
//! measured, never below the floor. An intersection curve's sag scales.

use forge_geom::curve::{CurveDeriv, ProcCurve, ProceduralCurve};
use forge_geom::intcurve::IntCurve;
use forge_geom::spline::SplineCurve;
use forge_geom::spline::meet::{extruded, spun};
use forge_geom::surface::{Proc, Procedural, SurfDeriv};
use forge_geom::{
    Circle, Curve, Cylinder, Domain1, Domain2, Ellipse, Hyperbola, Line, Parabola, Plane, Sphere, Surface,
    Torus,
};
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, UnitVec3, Vec3, tol};
use std::sync::Arc;

/// Why a scale is refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BadScale {
    /// k is not a finite number greater than zero.
    Factor(f64),
    /// The centre is not a finite point.
    Centre,
}

/// k and c as a similarity takes them: k finite and > 0, c finite.
pub fn check(c: Point3, k: f64) -> Result<(), BadScale> {
    if !(k.is_finite() && k > 0.0) {
        return Err(BadScale::Factor(k));
    }
    if !(c.x.is_finite() && c.y.is_finite() && c.z.is_finite()) {
        return Err(BadScale::Centre);
    }
    Ok(())
}

/// The image of a point: c + k (p − c).
#[inline]
pub fn point(p: Point3, c: Point3, k: f64) -> Point3 {
    c + (p - c) * k
}

/// A frame's image: the same axes about the scaled origin.
#[inline]
pub fn frame(f: &Frame, c: Point3, k: f64) -> Frame {
    f.with_origin(point(f.origin, c, k))
}

/// A box's image (k > 0 keeps the order of the corners).
pub fn boxed(b: Box3, c: Point3, k: f64) -> Box3 {
    if b.is_empty() {
        return b;
    }
    let lo = point(Point3::new(b.x.lo, b.y.lo, b.z.lo), c, k);
    let hi = point(Point3::new(b.x.hi, b.y.hi, b.z.hi), c, k);
    Box3 {
        x: Interval::new(lo.x, hi.x),
        y: Interval::new(lo.y, hi.y),
        z: Interval::new(lo.z, hi.z),
    }
}

/// The surface scaled by `k` (> 0, `check`) about `c`: the same kind, its lengths times k.
pub fn surface(s: &Surface, c: Point3, k: f64) -> Surface {
    {
        match s {
            Surface::Plane(p) => Surface::Plane(Plane {
                frame: frame(&p.frame, c, k),
            }),
            Surface::Cylinder(y) => Surface::Cylinder(Cylinder {
                frame: frame(&y.frame, c, k),
                radius: y.radius * k,
            }),
            Surface::Cone(o) => Surface::Cone(o.similar(frame(&o.frame, c, k), k)),
            Surface::Sphere(s) => Surface::Sphere(Sphere {
                frame: frame(&s.frame, c, k),
                radius: s.radius * k,
            }),
            Surface::Torus(t) => Surface::Torus(Torus {
                frame: frame(&t.frame, c, k),
                major: t.major * k,
                minor: t.minor * k,
            }),
            Surface::Procedural(p) => {
                if let Some(x) = extruded(s) {
                    x.scaled(c, k).surface()
                } else if let Some(x) = spun(s) {
                    x.scaled(c, k).surface()
                } else {
                    Surface::Procedural(Proc(Arc::new(Grown {
                        inner: p.clone(),
                        c,
                        k,
                    })))
                }
            }
        }
    }
}

/// The curve scaled by `k` (> 0, `check`) about `c`, with the image of the parameter range `r`:
/// times k where the parameter is a length, else the same.
pub fn curve(e: &Curve, r: Interval, c: Point3, k: f64) -> (Curve, Interval) {
    let by_k = Interval::new(r.lo * k, r.hi * k);
    {
        match e {
            Curve::Line(l) => (
                Curve::Line(Line {
                    origin: point(l.origin, c, k),
                    dir: l.dir,
                }),
                by_k,
            ),
            Curve::Circle(o) => (
                Curve::Circle(Circle {
                    frame: frame(&o.frame, c, k),
                    radius: o.radius * k,
                }),
                r,
            ),
            Curve::Ellipse(o) => (
                Curve::Ellipse(Ellipse {
                    frame: frame(&o.frame, c, k),
                    major: o.major * k,
                    minor: o.minor * k,
                }),
                r,
            ),
            // k P(t) = (k t² / 4f) x + k t y = P'(k t) with focal length k f
            Curve::Parabola(p) => (
                Curve::Parabola(Parabola {
                    frame: frame(&p.frame, c, k),
                    focal: p.focal * k,
                }),
                by_k,
            ),
            Curve::Hyperbola(h) => (
                Curve::Hyperbola(Hyperbola {
                    frame: frame(&h.frame, c, k),
                    a: h.a * k,
                    b: h.b * k,
                }),
                r,
            ),
            Curve::Intersection(i) => (
                Curve::Intersection(Arc::new(IntCurve::new(
                    surface(&i.a, c, k),
                    surface(&i.b, c, k),
                    i.pts.iter().map(|&p| point(p, c, k)).collect(),
                    i.tans.clone(),
                    i.closed,
                    i.sag * k,
                ))),
                r,
            ),
            Curve::Procedural(p) => match forge_geom::spline::spline_of(e) {
                Some(sc) => (SplineCurve::curve(Arc::new(sc.c.scaled_about(c, k)), sc.dom), r),
                None => (
                    Curve::Procedural(ProcCurve(Arc::new(Grown {
                        inner: p.clone(),
                        c,
                        k,
                    }))),
                    r,
                ),
            },
        }
    }
}

/// A procedural surface or curve with no scaled form of its own, scaled by `k` about `c`: each of
/// its points is the image of the inner's at the same parameters, its derivatives times k. Exact:
/// nothing is fitted, every value is the inner's mapped.
#[derive(Debug)]
pub struct Grown<T> {
    pub inner: T,
    pub c: Point3,
    pub k: f64,
}

impl<T> Grown<T> {
    fn to(&self, p: Point3) -> Point3 {
        point(p, self.c, self.k)
    }
    /// The inverse image: c + (p − c) / k.
    fn from(&self, p: Point3) -> Point3 {
        self.c + (p - self.c) / self.k
    }
    fn deriv(&self, d: SurfDeriv) -> SurfDeriv {
        let k = self.k;
        SurfDeriv {
            p: self.to(d.p),
            du: d.du * k,
            dv: d.dv * k,
            duu: d.duu * k,
            duv: d.duv * k,
            dvv: d.dvv * k,
        }
    }
}

impl Procedural for Grown<Proc> {
    fn kind(&self) -> &'static str {
        self.inner.kind()
    }
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        Grown::deriv(self, self.inner.deriv(uv))
    }
    fn deriv1(&self, uv: Point2) -> SurfDeriv {
        Grown::deriv(self, self.inner.deriv1(uv))
    }
    fn point(&self, uv: Point2) -> Point3 {
        self.to(self.inner.point(uv))
    }
    fn normal(&self, uv: Point2) -> Option<UnitVec3> {
        // the same unit normal: du × dv times k²
        self.inner.normal(uv)
    }
    fn domain(&self) -> Domain2 {
        self.inner.domain()
    }
    fn project(&self, p: Point3) -> Point2 {
        self.inner.project(self.from(p))
    }
    fn project_near(&self, p: Point3, near: Point2) -> Point2 {
        self.inner.project_near(self.from(p), near)
    }
    fn value_near(&self, p: Point3, uv: Option<Point2>) -> Option<(f64, Vec3, Point2)> {
        self.inner
            .value_near(self.from(p), uv)
            .map(|(f, g, uv)| (self.k * f, g, uv))
    }
    fn foot_near(&self, p: Point3, uv: Point2) -> Option<Point2> {
        self.inner.foot_near(self.from(p), uv)
    }
    fn has_foot_near(&self) -> bool {
        self.inner.has_foot_near()
    }
    fn walks(&self) -> bool {
        self.inner.walks()
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        boxed(self.inner.bbox(uv), self.c, self.k)
    }
    fn whole_box(&self) -> bool {
        self.inner.whole_box()
    }
    // k f(from p) is 1-Lipschitz when f is, and a signed distance when f is one
    fn implicit(&self, p: Point3) -> f64 {
        self.k * self.inner.implicit(self.from(p))
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        self.inner.gradient(self.from(p))
    }
    fn implicit_gradient(&self, p: Point3) -> (f64, Vec3) {
        let (f, g) = self.inner.implicit_gradient(self.from(p));
        (self.k * f, g)
    }
    fn gradient_turn(&self, p: Point3, v: Vec3) -> Option<Vec3> {
        self.inner.gradient_turn(self.from(p), v).map(|t| t / self.k)
    }
    fn within(&self, p: Point3, r: f64) -> bool {
        self.inner.within(self.from(p), r / self.k)
    }
    fn transformed(&self, f: &Frame) -> Proc {
        // a rigid f after the scale: f(c + k (p − c)) = f(c) + k (f(p) − f(c))
        Proc(Arc::new(Grown {
            inner: self.inner.transformed(f),
            c: f.point_to_world(self.c),
            k: self.k,
        }))
    }
    fn uv_scale(&self) -> (f64, f64) {
        let (u, v) = self.inner.uv_scale();
        (u * self.k, v * self.k)
    }
    fn mesh_radius(&self) -> f64 {
        self.k * self.inner.mesh_radius()
    }
    fn offset_reach(&self, d: f64) -> f64 {
        self.k * self.inner.offset_reach(d / self.k)
    }
    fn min_radius(&self) -> Option<f64> {
        self.inner.min_radius().map(|r| r * self.k)
    }
    fn feature(&self) -> f64 {
        self.k * self.inner.feature()
    }
    fn mesh_radius_over(&self, uv: Box2) -> f64 {
        self.k * self.inner.mesh_radius_over(uv)
    }
    fn direct(&self) -> bool {
        self.inner.direct()
    }
    fn regular(&self) -> bool {
        self.inner.regular()
    }
    fn v_kinks(&self, v: Interval) -> Vec<f64> {
        self.inner.v_kinks(v)
    }
    fn u_kinks(&self, u: Interval) -> Vec<f64> {
        self.inner.u_kinks(u)
    }
    fn exact_d1(&self) -> bool {
        self.inner.exact_d1()
    }
    fn v_degree(&self) -> Option<usize> {
        self.inner.v_degree()
    }
}

impl ProceduralCurve for Grown<ProcCurve> {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let d = self.inner.deriv(t);
        CurveDeriv {
            p: self.to(d.p),
            d1: d.d1 * self.k,
            d2: d.d2 * self.k,
        }
    }
    fn point(&self, t: f64) -> Point3 {
        self.to(self.inner.point(t))
    }
    fn deriv1(&self, t: f64) -> (Point3, Vec3) {
        let (p, d) = self.inner.deriv1(t);
        (self.to(p), d * self.k)
    }
    fn domain(&self) -> Domain1 {
        self.inner.domain()
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        self.inner.closest(self.from(p), range)
    }
    fn bbox(&self, range: Interval) -> Box3 {
        boxed(self.inner.bbox(range), self.c, self.k)
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(Grown {
            inner: self.inner.transformed(f),
            c: f.point_to_world(self.c),
            k: self.k,
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        self.k * self.inner.speed_bound(range)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        self.inner.segments(range, chord / self.k)
    }
    fn across(&self) -> Option<Surface> {
        self.inner.across().map(|s| surface(&s, self.c, self.k))
    }
    fn kinks(&self, range: Interval) -> Vec<f64> {
        self.inner.kinks(range)
    }
    fn exact_d1(&self) -> bool {
        self.inner.exact_d1()
    }
}

use crate::{Body, Name};

/// Why a body cannot be scaled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScaleError {
    Bad(BadScale),
    /// The body was not compacted: a face or edge removed by an in-place edit is still held.
    Dead(Name),
}

/// A scaled tolerance, by the rule above.
pub fn scaled_tol(t: f64, k: f64) -> f64 {
    if t > tol::LINEAR {
        (t * k).max(tol::LINEAR)
    } else {
        t
    }
}

/// `body` (compacted) scaled by `k` (finite, > 0) about `c` (finite).
pub fn scaled(body: &Body, c: Point3, k: f64) -> Result<Body, ScaleError> {
    check(c, k).map_err(ScaleError::Bad)?;
    if let Some(f) = body.faces.iter().find(|f| !f.alive) {
        return Err(ScaleError::Dead(f.name));
    }
    if let Some(e) = body.edges.iter().find(|e| !e.alive) {
        return Err(ScaleError::Dead(e.name));
    }
    let mut out = body.clone();
    for f in &mut out.faces {
        f.surface = surface(&f.surface, c, k);
        f.bbox = Default::default();
        f.query = Default::default();
    }
    for e in &mut out.edges {
        (e.curve, e.range) = curve(&e.curve, e.range, c, k);
        e.tol = scaled_tol(e.tol, k);
        e.bbox = Default::default();
    }
    for v in &mut out.vertices {
        v.point = point(v.point, c, k);
        v.tol = scaled_tol(v.tol, k);
    }
    out.hulls = Default::default();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_geom::{Cone, CurveGeom, SurfaceGeom};

    fn frame0() -> Frame {
        Frame::from_zx(
            Point3::new(3.0, -2.0, 5.0),
            Vec3::new(0.2, -0.4, 1.0).normalize().expect("z"),
            Vec3::X,
        )
        .expect("frame")
    }

    /// Every analytic surface's scaled point at the image parameters is the scaled point.
    #[test]
    fn surfaces_map_points() {
        let f = frame0();
        let (c, k) = (Point3::new(1.0, 7.0, -3.0), 2.5);
        let all = [
            Surface::Plane(Plane { frame: f }),
            Surface::Cylinder(Cylinder {
                frame: f,
                radius: 4.0,
            }),
            Surface::Cone(Cone::new(f, 4.0, 0.3).expect("cone")),
            Surface::Sphere(Sphere {
                frame: f,
                radius: 4.0,
            }),
            Surface::Torus(Torus::new(f, 6.0, 2.0).expect("torus")),
        ];
        for s in &all {
            let t = surface(s, c, k);
            for &(u, v) in &[(0.3, 0.7), (1.9, -0.4), (4.0, 1.1)] {
                // a length parameter is times k
                let lengthy_u = matches!(s, Surface::Plane(_));
                let lengthy_v = matches!(s, Surface::Plane(_) | Surface::Cylinder(_) | Surface::Cone(_));
                let uv2 = Point2::new(
                    if lengthy_u { u * k } else { u },
                    if lengthy_v { v * k } else { v },
                );
                let want = point(s.eval(Point2::new(u, v)), c, k);
                let got = t.eval(uv2);
                assert!(
                    want.distance(got) < 1e-12 * (1.0 + want.to_vec().length()),
                    "{s:?}"
                );
                assert!(t.implicit(got).abs() < 1e-10, "{s:?}");
            }
        }
    }

    #[test]
    fn curves_map_points() {
        let f = frame0();
        let (c, k) = (Point3::new(-1.0, 2.0, 0.5), 0.375);
        let r = Interval::new(-0.5, 1.25);
        let all = [
            Curve::Line(Line {
                origin: f.origin,
                dir: f.x(),
            }),
            Curve::Circle(Circle {
                frame: f,
                radius: 3.0,
            }),
            Curve::Ellipse(Ellipse {
                frame: f,
                major: 5.0,
                minor: 2.0,
            }),
            Curve::Parabola(Parabola { frame: f, focal: 1.5 }),
            Curve::Hyperbola(Hyperbola {
                frame: f,
                a: 2.0,
                b: 1.0,
            }),
        ];
        for e in &all {
            let (s, r2) = curve(e, r, c, k);
            for j in 0..=4 {
                let a = j as f64 / 4.0;
                let want = point(e.eval(r.lo + a * r.width()), c, k);
                let got = s.eval(r2.lo + a * r2.width());
                assert!(
                    want.distance(got) < 1e-12 * (1.0 + want.to_vec().length()),
                    "{e:?}"
                );
            }
        }
    }

    #[test]
    fn refuses_bad_factors() {
        let o = Point3::ORIGIN;
        for k in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(check(o, k).is_err(), "{k}");
        }
        assert_eq!(check(Point3::new(f64::NAN, 0.0, 0.0), 2.0), Err(BadScale::Centre));
        assert!(check(o, 1e-3).is_ok());
    }
}

use crate::curve::Domain1;
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, UnitVec3, Vec2, Vec3, dmath, tol};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Domain2 {
    pub u: Domain1,
    pub v: Domain1,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfDeriv {
    pub p: Point3,
    pub du: Vec3,
    pub dv: Vec3,
    pub duu: Vec3,
    pub duv: Vec3,
    pub dvv: Vec3,
}

pub trait SurfaceGeom {
    fn deriv(&self, uv: Point2) -> SurfDeriv;
    fn eval(&self, uv: Point2) -> Point3 {
        self.deriv(uv).p
    }
    /// du x dv, normalised; None where the parameterisation is singular (cone
    /// apex, sphere poles). The direction is the surface's natural side:
    /// outward for closed shapes, the frame's z for a plane.
    fn normal(&self, uv: Point2) -> Option<UnitVec3> {
        let d = self.deriv(uv);
        d.du.cross(d.dv).normalize()
    }
    fn domain(&self) -> Domain2;
    /// Parameters of the point of the surface nearest `p` (over its whole domain).
    fn project(&self, p: Point3) -> Point2;
    /// A box guaranteed to contain the surface over the parameter box.
    fn bbox(&self, uv: Box2) -> Box3;
    /// Points of every branch of the surface's intersection with `other` in `region`, for the
    /// tracer to start from in place of its octree; None where it has no such way.
    fn seeds(&self, _other: &Surface, _region: Box3, _leaf: f64) -> Option<Vec<Point3>> {
        None
    }
    /// Whether a face's box may be `bbox` of the whole domain, no look at its loops.
    fn whole_box(&self) -> bool {
        false
    }
    fn transformed(&self, f: &Frame) -> Self
    where
        Self: Sized;
    fn distance(&self, p: Point3) -> f64 {
        self.eval(self.project(p)).distance(p)
    }
}

/// u along the frame's x, v along y; natural normal is z.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub frame: Frame,
}

/// Axis along the frame's z. u is the angle from x, v the height along z.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cylinder {
    pub frame: Frame,
    pub radius: f64,
}

/// Half-cone about the frame's z: radius `radius` at v = 0, growing by
/// tan(half_angle) per unit of v. u is the angle, v the height along z.
/// The apex is at v = -radius / tan(half_angle); the domain stops there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cone {
    pub frame: Frame,
    pub radius: f64,
    sin_a: f64,
    cos_a: f64,
}

/// u is longitude from x about z, v latitude in [-pi/2, pi/2].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    pub frame: Frame,
    pub radius: f64,
}

/// Ring torus about the frame's z: tube of radius `minor` around a circle of
/// radius `major` (major > minor). u goes around z, v around the tube.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Torus {
    pub frame: Frame,
    pub major: f64,
    pub minor: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Surface {
    Plane(Plane),
    Cylinder(Cylinder),
    Cone(Cone),
    Sphere(Sphere),
    Torus(Torus),
    /// Defined by construction (a blend, a pipe along an exact spine, a corner patch).
    Procedural(Proc),
}

/// A surface to the bit, or the procedural one it is by its address (whoever keeps these holds the surface too,
/// so the address stays its own). Equal only for surfaces that answer every question alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bits {
    Plane([u64; 12]),
    Cylinder([u64; 13]),
    Cone([u64; 15]),
    Sphere([u64; 13]),
    Torus([u64; 14]),
    Procedural(usize),
}

impl Surface {
    /// All this surface is. Whole structs below, so that a field added does not compile until it is here.
    pub fn bits(&self) -> Bits {
        fn with<const N: usize>(f: &Frame, more: &[f64]) -> [u64; N] {
            let (o, x, y, z) = (f.origin, f.x().get(), f.y().get(), f.z().get());
            let mut out = [0u64; N];
            let head = [o.x, o.y, o.z, x.x, x.y, x.z, y.x, y.y, y.z, z.x, z.y, z.z];
            for (w, v) in out.iter_mut().zip(head.iter().chain(more)) {
                *w = v.to_bits();
            }
            out
        }
        match self {
            Surface::Plane(Plane { frame }) => Bits::Plane(with(frame, &[])),
            Surface::Cylinder(Cylinder { frame, radius }) => Bits::Cylinder(with(frame, &[*radius])),
            Surface::Cone(Cone {
                frame,
                radius,
                sin_a,
                cos_a,
            }) => Bits::Cone(with(frame, &[*radius, *sin_a, *cos_a])),
            Surface::Sphere(Sphere { frame, radius }) => Bits::Sphere(with(frame, &[*radius])),
            Surface::Torus(Torus { frame, major, minor }) => Bits::Torus(with(frame, &[*major, *minor])),
            Surface::Procedural(p) => Bits::Procedural(std::sync::Arc::as_ptr(&p.0).cast::<()>() as usize),
        }
    }
}

/// A surface defined by construction rather than by a formula of its own: a rolling-ball blend,
/// a pipe along an exact spine, a corner patch spanning exact boundary curves. Exact: points are
/// solved for to full precision from the definition, never read off an approximation.
pub trait Procedural: Send + Sync + std::fmt::Debug {
    fn deriv(&self, uv: Point2) -> SurfDeriv;
    /// The point alone (meshing evaluates many more points than derivatives).
    fn point(&self, uv: Point2) -> Point3 {
        self.deriv(uv).p
    }
    /// Whether the surface is a section swept along u (a blend's arc), so `column` works the
    /// section out once for all its points (meshing then lays the face out in columns).
    fn swept(&self) -> bool {
        false
    }
    /// The points at u and each v of `vs`, onto `out`, each exactly `point`'s.
    fn column(&self, u: f64, vs: &[f64], out: &mut Vec<Point3>) {
        out.extend(vs.iter().map(|&v| self.point(Point2::new(u, v))));
    }
    /// The derivatives at u and each v of `vs`, onto `out`, each exactly `deriv`'s.
    fn column_derivs(&self, u: f64, vs: &[f64], out: &mut Vec<SurfDeriv>) {
        out.extend(vs.iter().map(|&v| self.deriv(Point2::new(u, v))));
    }
    /// `column_derivs` where only the point and first derivatives need be right (the second
    /// may be left out).
    fn column_d1(&self, u: f64, vs: &[f64], out: &mut Vec<SurfDeriv>) {
        self.column_derivs(u, vs, out)
    }
    fn normal(&self, uv: Point2) -> Option<UnitVec3> {
        let d = self.deriv(uv);
        d.du.cross(d.dv).normalize()
    }
    fn domain(&self) -> Domain2;
    /// Parameters of the point of the surface nearest `p`.
    fn project(&self, p: Point3) -> Point2;
    /// Where curve `c`, lying on the surface, is in its parameters, if in closed form: at the
    /// curve's t, `base + dir * t`. None where only projecting its points finds it.
    fn curve_uv(&self, c: &crate::curve::ProcCurve) -> Option<(Point2, Vec2)> {
        let _ = c;
        None
    }
    /// `project` for a point of the surface near the one at `near` (a loop's next sample), where
    /// the construction can start from there instead of searching.
    fn project_near(&self, p: Point3, near: Point2) -> Point2 {
        let _ = near;
        self.project(p)
    }
    /// `implicit`, `gradient` and the nearest point's parameters, found by Newton from `uv` where
    /// p lies on the surface near it (else as `project`); None where the surface has no faster
    /// way than `implicit` and `gradient`.
    fn value_near(&self, _p: Point3, _uv: Option<Point2>) -> Option<(f64, Vec3, Point2)> {
        None
    }
    /// A box guaranteed to contain the surface over the parameter box.
    fn bbox(&self, uv: Box2) -> Box3;
    /// Points of every branch of the surface's intersection with `other` in `region`, for the
    /// tracer to start from in place of its octree; None where it has no such way.
    fn seeds(&self, _other: &Surface, _region: Box3, _leaf: f64) -> Option<Vec<Point3>> {
        None
    }
    /// Whether a face's box may be `bbox` of the whole domain, no look at its loops.
    fn whole_box(&self) -> bool {
        false
    }
    /// Zero on the surface, positive on the natural normal's side, 1-Lipschitz (as
    /// `Surface::implicit`).
    fn implicit(&self, p: Point3) -> f64;
    /// Gradient of `implicit` at `p`.
    fn gradient(&self, p: Point3) -> Vec3;
    /// Both, for surfaces that get them from one solve.
    fn implicit_gradient(&self, p: Point3) -> (f64, Vec3) {
        (self.implicit(p), self.gradient(p))
    }
    /// Whether |implicit(p)| <= r, for surfaces that can often tell without the value.
    fn within(&self, p: Point3, r: f64) -> bool {
        self.implicit(p).abs() <= r
    }
    fn transformed(&self, f: &Frame) -> Proc;
    /// Lengths on the surface per unit of u and of v, roughly (for meshing).
    fn uv_scale(&self) -> (f64, f64) {
        (1.0, 1.0)
    }
    /// About the tightest radius of curvature, for meshing sizes and for callers that need a
    /// radius until a bound is proven (their results are checked for self-intersection): not
    /// a bound.
    fn mesh_radius(&self) -> f64;
    /// A radius of curvature no larger than the surface's tightest anywhere, proven; None when
    /// none is known.
    fn min_radius(&self) -> Option<f64> {
        None
    }
    /// The size of the smallest feature of the implicit's zero set off the face (a turn of a
    /// carried-on track, the gap between two sheets): tracing steps stay well below it.
    fn feature(&self) -> f64 {
        f64::INFINITY
    }
    /// `mesh_radius` over the parameter box only (a face on part of the surface).
    fn mesh_radius_over(&self, _uv: Box2) -> f64 {
        self.mesh_radius()
    }
    /// Bounds on the surface's bending over the parameter box, where the construction gives them
    /// (None where it does not): with them a chord's gap from the surface is bounded without
    /// evaluating it (meshing).
    fn shape_bounds(&self, uv: Box2) -> Option<ShapeBounds> {
        let _ = uv;
        None
    }
    /// The surface itself, for closed forms that know its kind.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
    /// Whether this is a spline wall, asked before the costlier `as_any` on hot paths.
    fn spline_wall(&self) -> bool {
        false
    }
    /// Whether `project` is exact and cheap, so a walk along a curve on the surface needs no Newton
    /// steps from the point before (which past a pole could land on the far side of the axis).
    fn direct(&self) -> bool {
        false
    }
    /// That no point of its domain is singular.
    fn regular(&self) -> bool {
        false
    }
    /// The u inside `u` where the surface is only once differentiable along u (a traced
    /// spine's chain points), for integrators that need smooth pieces.
    fn u_kinks(&self, _u: Interval) -> Vec<f64> {
        Vec::new()
    }
    /// Boxes that together hold every face this surface carries (None: unknown, the surface
    /// may be used anywhere it reaches).
    fn patches(&self) -> Option<&[Box3]> {
        None
    }
    /// Whether `deriv`'s first derivatives are exact (not differences), for integrators that
    /// take them as they are.
    fn exact_d1(&self) -> bool {
        false
    }
    /// Some(d) where the surface is a polynomial of degree d in v at each u over v in [0, 1] (a
    /// ruled surface: 1), so integrals along v have closed rules.
    fn v_degree(&self) -> Option<usize> {
        None
    }
    /// The same surface with a smooth parametrisation where this one's is only once
    /// differentiable (a pipe round a traced spine), for integrators that find their loops on it.
    fn smooth(&self) -> Option<&Surface> {
        None
    }
}

/// Bounds over a parameter box: `k` on |n.S_uu|, |n.S_uv| and |n.S_vv| (n the unit normal at the
/// same point), `m` on |S_uu|, |S_uv| and |S_vv|, `d` on |S_u|, |S_v| and |S_u.S_v|, and `area`
/// from below on |S_u x S_v|.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeBounds {
    pub k: [f64; 3],
    pub m: [f64; 3],
    pub d: [f64; 3],
    pub area: f64,
}

impl ShapeBounds {
    /// From the derivatives `d` at the middle of a box `half` wide each way, and bounds over it
    /// on the second derivatives' sizes (`m`) and the third's (`t`: S_uuu, S_uuv, S_uvv, S_vvv).
    /// Each first derivative strays from the middle's by at most m times the offset, the normal
    /// by at most twice the cross product's stray over its size, the second derivatives by t
    /// times the offset.
    pub fn taylor(d: &SurfDeriv, (hu, hv): (f64, f64), m: [f64; 3], t: [f64; 4]) -> ShapeBounds {
        let (eu, ev) = (m[0] * hu + m[1] * hv, m[1] * hu + m[2] * hv);
        let (lu, lv) = (d.du.length(), d.dv.length());
        let cross = d.du.cross(d.dv);
        let size = cross.length();
        // |a x b - a0 x b0| and |a.b - a0.b0| are both at most |a - a0| |b| + |a0| |b - b0|
        let stray = eu * (lv + ev) + lu * ev;
        let (n, turn) = if size > 0.0 {
            (cross / size, (2.0 * stray / size).min(2.0))
        } else {
            (Vec3::ZERO, 2.0)
        };
        let s = [d.duu, d.duv, d.dvv];
        let dt = [
            t[0] * hu + t[1] * hv,
            t[1] * hu + t[2] * hv,
            t[2] * hu + t[3] * hv,
        ];
        ShapeBounds {
            k: [0, 1, 2].map(|i| n.dot(s[i]).abs() + turn * s[i].length() + dt[i]),
            m: [0, 1, 2].map(|i| m[i].min(s[i].length() + dt[i])),
            d: [lu + eu, lv + ev, d.du.dot(d.dv).abs() + stray],
            area: (size - stray).max(0.0),
        }
    }

    /// From below, the least singular value of [S_u S_v]: its square times the largest's is the
    /// area's square, and the largest's square is at most |S_u|^2 + |S_v|^2.
    pub fn stretch(&self) -> f64 {
        self.area / (self.d[0] * self.d[0] + self.d[1] * self.d[1]).sqrt()
    }

    /// The most the unit normal turns per unit of u and of v. Its derivative n_u is tangent with
    /// n_u.S_u = -n.S_uu and n_u.S_v = -n.S_uv, so |n_u|^2 = (G a^2 - 2 F a b + E b^2) / (E G - F^2)
    /// for a = n.S_uu, b = n.S_uv (and n_v likewise from n.S_uv, n.S_vv).
    pub fn turn(&self) -> (f64, f64) {
        let [eu, ev, f] = [self.d[0] * self.d[0], self.d[1] * self.d[1], self.d[2]];
        let det = self.area * self.area;
        if det <= 0.0 {
            return (f64::INFINITY, f64::INFINITY);
        }
        let rate = |a: f64, b: f64| ((ev * a * a + 2.0 * f * a * b + eu * b * b) / det).sqrt();
        (rate(self.k[0], self.k[1]), rate(self.k[1], self.k[2]))
    }
}

/// A shared handle to a procedural surface; two handles are equal when they are the same surface.
#[derive(Clone, Debug)]
pub struct Proc(pub std::sync::Arc<dyn Procedural>);

impl std::ops::Deref for Proc {
    type Target = dyn Procedural;
    fn deref(&self) -> &(dyn Procedural + 'static) {
        &*self.0
    }
}

impl PartialEq for Proc {
    fn eq(&self, o: &Proc) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &o.0)
    }
}

fn angle_dom() -> Domain1 {
    Domain1::ANGLE
}

fn wrap_angle(y: f64, x: f64) -> f64 {
    if x == 0.0 && y == 0.0 {
        0.0
    } else {
        dmath::wrap_tau(dmath::atan2(y, x))
    }
}

/// Interval box of origin + a x + b y + c z, where a, b, c are frame-local intervals.
fn frame_box(f: &Frame, a: Interval, b: Interval, c: Interval) -> Box3 {
    let (x, y, z, o) = (f.x().get(), f.y().get(), f.z().get(), f.origin);
    Box3 {
        x: Interval::point(o.x) + a * x.x + b * y.x + c * z.x,
        y: Interval::point(o.y) + a * x.y + b * y.y + c * z.y,
        z: Interval::point(o.z) + a * x.z + b * y.z + c * z.z,
    }
}

/// The radial direction and its derivative at angle u, in world coordinates.
fn radial(f: &Frame, u: f64) -> (Vec3, Vec3) {
    let (s, c) = dmath::sin_cos(u);
    (f.x() * c + f.y() * s, f.x() * (-s) + f.y() * c)
}

impl Plane {
    pub fn new(frame: Frame) -> Self {
        Plane { frame }
    }
    pub fn normal_vec(&self) -> UnitVec3 {
        self.frame.z()
    }
    /// Signed distance along the normal.
    pub fn signed_distance(&self, p: Point3) -> f64 {
        self.frame.z().dot(p - self.frame.origin)
    }
}

impl SurfaceGeom for Plane {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        let f = &self.frame;
        SurfDeriv {
            p: f.plane_point(uv),
            du: f.x().get(),
            dv: f.y().get(),
            duu: Vec3::ZERO,
            duv: Vec3::ZERO,
            dvv: Vec3::ZERO,
        }
    }
    fn normal(&self, _: Point2) -> Option<UnitVec3> {
        Some(self.frame.z())
    }
    fn domain(&self) -> Domain2 {
        Domain2 {
            u: Domain1::LINE,
            v: Domain1::LINE,
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        let l = self.frame.point_to_local(p);
        Point2::new(l.x, l.y)
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        frame_box(&self.frame, uv.u, uv.v, Interval::point(0.0))
    }
    fn transformed(&self, f: &Frame) -> Self {
        Plane {
            frame: f.compose(&self.frame),
        }
    }
}

impl SurfaceGeom for Cylinder {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        let f = &self.frame;
        let r = self.radius;
        let (rad, drad) = radial(f, uv.x);
        SurfDeriv {
            p: f.origin + rad * r + f.z() * uv.y,
            du: drad * r,
            dv: f.z().get(),
            duu: rad * -r,
            duv: Vec3::ZERO,
            dvv: Vec3::ZERO,
        }
    }
    fn normal(&self, uv: Point2) -> Option<UnitVec3> {
        Some(UnitVec3::new_unchecked(radial(&self.frame, uv.x).0))
    }
    fn domain(&self) -> Domain2 {
        Domain2 {
            u: angle_dom(),
            v: Domain1::LINE,
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        let l = self.frame.point_to_local(p);
        Point2::new(wrap_angle(l.y, l.x), l.z)
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        frame_box(
            &self.frame,
            uv.u.cos() * self.radius,
            uv.u.sin() * self.radius,
            uv.v,
        )
    }
    fn transformed(&self, f: &Frame) -> Self {
        Cylinder {
            frame: f.compose(&self.frame),
            radius: self.radius,
        }
    }
}

impl Cone {
    /// half_angle strictly between 0 and pi/2.
    pub fn new(frame: Frame, radius: f64, half_angle: f64) -> Option<Self> {
        if !(half_angle > tol::ANGULAR && half_angle < dmath::FRAC_PI_2 - 1e-6) || radius < 0.0 {
            return None;
        }
        let (sin_a, cos_a) = dmath::sin_cos(half_angle);
        Some(Cone {
            frame,
            radius,
            sin_a,
            cos_a,
        })
    }
    pub fn tan_a(&self) -> f64 {
        self.sin_a / self.cos_a
    }
    pub fn sin_a(&self) -> f64 {
        self.sin_a
    }
    pub fn cos_a(&self) -> f64 {
        self.cos_a
    }
    pub fn apex_v(&self) -> f64 {
        -self.radius / self.tan_a()
    }
    pub fn apex(&self) -> Point3 {
        self.frame.origin + self.frame.z() * self.apex_v()
    }
    pub fn radius_at(&self, v: f64) -> f64 {
        self.radius + v * self.tan_a()
    }
}

impl SurfaceGeom for Cone {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        let f = &self.frame;
        let k = self.tan_a();
        let rho = self.radius_at(uv.y);
        let (rad, drad) = radial(f, uv.x);
        SurfDeriv {
            p: f.origin + rad * rho + f.z() * uv.y,
            du: drad * rho,
            dv: rad * k + f.z().get(),
            duu: rad * -rho,
            duv: drad * k,
            dvv: Vec3::ZERO,
        }
    }
    fn normal(&self, uv: Point2) -> Option<UnitVec3> {
        if self.radius_at(uv.y) <= tol::LINEAR {
            return None;
        }
        let (rad, _) = radial(&self.frame, uv.x);
        Some(UnitVec3::new_unchecked(
            rad * self.cos_a - self.frame.z() * self.sin_a,
        ))
    }
    fn domain(&self) -> Domain2 {
        Domain2 {
            u: angle_dom(),
            v: Domain1 {
                lo: self.apex_v(),
                hi: f64::INFINITY,
                period: None,
            },
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        let l = self.frame.point_to_local(p);
        let rho = (l.x * l.x + l.y * l.y).sqrt();
        let u = wrap_angle(l.y, l.x);
        // In the half-plane of angle u the generator runs from (radius, 0) along (sin a, cos a).
        let s = (rho - self.radius) * self.sin_a + l.z * self.cos_a;
        let v = (s * self.cos_a).max(self.apex_v());
        Point2::new(u, v)
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        let rho = Interval::point(self.radius) + uv.v * self.tan_a();
        frame_box(&self.frame, uv.u.cos() * rho, uv.u.sin() * rho, uv.v)
    }
    fn transformed(&self, f: &Frame) -> Self {
        Cone {
            frame: f.compose(&self.frame),
            ..*self
        }
    }
}

impl SurfaceGeom for Sphere {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        let f = &self.frame;
        let r = self.radius;
        let (rad, drad) = radial(f, uv.x);
        let (sv, cv) = dmath::sin_cos(uv.y);
        let z = f.z().get();
        SurfDeriv {
            p: f.origin + rad * (r * cv) + z * (r * sv),
            du: drad * (r * cv),
            dv: rad * (-r * sv) + z * (r * cv),
            duu: rad * (-r * cv),
            duv: drad * (-r * sv),
            dvv: rad * (-r * cv) + z * (-r * sv),
        }
    }
    fn normal(&self, uv: Point2) -> Option<UnitVec3> {
        let d = self.deriv(uv);
        if d.du.length() <= tol::LINEAR {
            return None;
        }
        (d.p - self.frame.origin).normalize()
    }
    fn domain(&self) -> Domain2 {
        Domain2 {
            u: angle_dom(),
            v: Domain1 {
                lo: -dmath::FRAC_PI_2,
                hi: dmath::FRAC_PI_2,
                period: None,
            },
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        let l = self.frame.point_to_local(p);
        let rho = (l.x * l.x + l.y * l.y).sqrt();
        Point2::new(wrap_angle(l.y, l.x), dmath::atan2(l.z, rho))
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        let r = self.radius;
        let cv = uv.v.cos() * r;
        frame_box(&self.frame, uv.u.cos() * cv, uv.u.sin() * cv, uv.v.sin() * r)
    }
    fn transformed(&self, f: &Frame) -> Self {
        Sphere {
            frame: f.compose(&self.frame),
            radius: self.radius,
        }
    }
}

impl Torus {
    pub fn new(frame: Frame, major: f64, minor: f64) -> Option<Self> {
        (minor > tol::MIN_LENGTH && major > minor + tol::MIN_LENGTH).then_some(Torus { frame, major, minor })
    }

    /// A torus whose tube reaches through its axis, for a revolved arc that ends on the axis: the
    /// centre on the arc's side (major > 0), or across the axis from it (major < 0). Only the part
    /// of the tube off the axis on the centre circle's frame is used, between the poles at `poles`.
    pub fn spindle(frame: Frame, major: f64, minor: f64) -> Option<Self> {
        (minor > tol::MIN_LENGTH && major.abs() > tol::MIN_LENGTH && major.abs() <= minor + tol::MIN_LENGTH)
            .then_some(Torus { frame, major, minor })
    }

    /// Whether the tube reaches the axis.
    pub fn is_spindle(&self) -> bool {
        self.major.abs() <= self.minor + tol::MIN_LENGTH
    }

    /// The v of the pole a spindle's used part ends at, positive; the other is its negative.
    pub fn pole_v(&self) -> f64 {
        dmath::acos((-self.major / self.minor).clamp(-1.0, 1.0))
    }
}

impl SurfaceGeom for Torus {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        let f = &self.frame;
        let (big, r) = (self.major, self.minor);
        let (rad, drad) = radial(f, uv.x);
        let (sv, cv) = dmath::sin_cos(uv.y);
        let z = f.z().get();
        let rho = big + r * cv;
        SurfDeriv {
            p: f.origin + rad * rho + z * (r * sv),
            du: drad * rho,
            dv: rad * (-r * sv) + z * (r * cv),
            duu: rad * -rho,
            duv: drad * (-r * sv),
            dvv: rad * (-r * cv) + z * (-r * sv),
        }
    }
    fn normal(&self, uv: Point2) -> Option<UnitVec3> {
        let (rad, _) = radial(&self.frame, uv.x);
        let (sv, cv) = dmath::sin_cos(uv.y);
        Some(UnitVec3::new_unchecked(rad * cv + self.frame.z() * sv))
    }
    fn domain(&self) -> Domain2 {
        Domain2 {
            u: angle_dom(),
            v: angle_dom(),
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        let l = self.frame.point_to_local(p);
        let rho = (l.x * l.x + l.y * l.y).sqrt();
        Point2::new(wrap_angle(l.y, l.x), wrap_angle(l.z, rho - self.major))
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        let rho = Interval::point(self.major) + uv.v.cos() * self.minor;
        frame_box(
            &self.frame,
            uv.u.cos() * rho,
            uv.u.sin() * rho,
            uv.v.sin() * self.minor,
        )
    }
    fn transformed(&self, f: &Frame) -> Self {
        Torus {
            frame: f.compose(&self.frame),
            ..*self
        }
    }
}

impl Surface {
    /// `project` for a point of the surface near the one at `near` (see `Procedural::project_near`).
    pub fn project_near(&self, p: Point3, near: Point2) -> Point2 {
        match self {
            Surface::Procedural(pr) => pr.project_near(p, near),
            _ => self.project(p),
        }
    }

    /// Whether the normal is undefined at `uv` (`normal(uv)` is None), without working it out
    /// where it never is.
    pub fn singular(&self, uv: Point2) -> bool {
        match self {
            Surface::Plane(_) | Surface::Cylinder(_) => false,
            // (a profile's arc ends on the axis to within tolerance)
            Surface::Torus(t) => {
                t.is_spindle() && (t.major + t.minor * dmath::cos(uv.y)).abs() <= 2.0 * tol::LINEAR
            }
            Surface::Cone(c) => c.radius_at(uv.y) <= tol::LINEAR,
            Surface::Procedural(p) if p.regular() => false,
            _ => self.normal(uv).is_none(),
        }
    }

    /// Boxes holding every face on the surface, where it knows them (`Procedural::patches`).
    pub fn patches(&self) -> Option<&[Box3]> {
        match self {
            Surface::Procedural(s) => s.patches(),
            _ => None,
        }
    }
}

impl Surface {
    /// Signed distance-like value: zero on the surface, positive on the side
    /// of its natural normal, and 1-Lipschitz (it changes no faster than the
    /// point moves), so a bound on it bounds where the surface can be.
    /// Gradient of `implicit` at p (unit length away from axes, poles and apexes).
    #[inline]
    pub fn gradient(&self, p: Point3) -> Vec3 {
        let radial = |f: &Frame, l: Point3| {
            let rho = (l.x * l.x + l.y * l.y).sqrt();
            if rho > 0.0 {
                f.x() * (l.x / rho) + f.y() * (l.y / rho)
            } else {
                f.x().get()
            }
        };
        match self {
            Surface::Plane(s) => s.frame.z().get(),
            Surface::Cylinder(c) => radial(&c.frame, c.frame.point_to_local(p)),
            Surface::Cone(c) => {
                let l = c.frame.point_to_local(p);
                radial(&c.frame, l) * c.cos_a() - c.frame.z() * c.sin_a()
            }
            Surface::Sphere(s) => (p - s.frame.origin).normalize().map_or(Vec3::Z, |u| u.get()),
            Surface::Torus(t) => {
                let l = t.frame.point_to_local(p);
                let r = radial(&t.frame, l);
                let centre = t.frame.origin + r * t.major;
                (p - centre).normalize().map_or(r, |u| u.get())
            }
            Surface::Procedural(s) => s.gradient(p),
        }
    }

    #[inline]
    pub fn implicit(&self, p: Point3) -> f64 {
        match self {
            Surface::Plane(s) => s.signed_distance(p),
            Surface::Cylinder(c) => {
                let l = c.frame.point_to_local(p);
                (l.x * l.x + l.y * l.y).sqrt() - c.radius
            }
            Surface::Cone(c) => {
                let l = c.frame.point_to_local(p);
                let rho = (l.x * l.x + l.y * l.y).sqrt();
                (rho - c.radius) * c.cos_a() - l.z * c.sin_a()
            }
            Surface::Sphere(s) => p.distance(s.frame.origin) - s.radius,
            Surface::Torus(t) => {
                let l = t.frame.point_to_local(p);
                let d = (l.x * l.x + l.y * l.y).sqrt() - t.major;
                (d * d + l.z * l.z).sqrt() - t.minor
            }
            Surface::Procedural(s) => s.implicit(p),
        }
    }
}

impl Surface {
    /// `implicit` and `gradient` at p together (one change of frame), equal to each.
    #[inline]
    pub fn implicit_gradient(&self, p: Point3) -> (f64, Vec3) {
        let radial = |f: &Frame, l: Point3, rho: f64| {
            if rho > 0.0 {
                f.x() * (l.x / rho) + f.y() * (l.y / rho)
            } else {
                f.x().get()
            }
        };
        match self {
            Surface::Cylinder(c) => {
                let l = c.frame.point_to_local(p);
                let rho = (l.x * l.x + l.y * l.y).sqrt();
                (rho - c.radius, radial(&c.frame, l, rho))
            }
            Surface::Cone(c) => {
                let l = c.frame.point_to_local(p);
                let rho = (l.x * l.x + l.y * l.y).sqrt();
                (
                    (rho - c.radius) * c.cos_a() - l.z * c.sin_a(),
                    radial(&c.frame, l, rho) * c.cos_a() - c.frame.z() * c.sin_a(),
                )
            }
            Surface::Torus(t) => {
                let l = t.frame.point_to_local(p);
                let rho = (l.x * l.x + l.y * l.y).sqrt();
                let d = rho - t.major;
                let r = radial(&t.frame, l, rho);
                let centre = t.frame.origin + r * t.major;
                (
                    (d * d + l.z * l.z).sqrt() - t.minor,
                    (p - centre).normalize().map_or(r, |u| u.get()),
                )
            }
            Surface::Procedural(s) => s.implicit_gradient(p),
            _ => (self.implicit(p), self.gradient(p)),
        }
    }

    /// Points at (u0 + k (u1 - u0) / m, v) for k in 1..m: the interior of a
    /// row at constant v, with one sin_cos for v and stepped angles for u.
    pub fn eval_row(
        &self,
        v: f64,
        u0: f64,
        u1: f64,
        m: usize,
        trig: &mut Vec<(f64, f64)>,
        out: &mut Vec<Point3>,
    ) {
        out.clear();
        if m < 2 {
            return;
        }
        let du = (u1 - u0) / m as f64;
        let radial = |f: &Frame, rho: f64, z: f64, trig: &[(f64, f64)], out: &mut Vec<Point3>| {
            let (x, y, zz) = (f.x().get(), f.y().get(), f.z().get());
            let base = f.origin + zz * z;
            for &(s, c) in trig {
                out.push(base + (x * c + y * s) * rho);
            }
        };
        match self {
            Surface::Plane(p) => {
                for k in 1..m {
                    out.push(p.frame.plane_point(Point2::new(u0 + du * k as f64, v)));
                }
                return;
            }
            Surface::Procedural(_) => {
                for k in 1..m {
                    out.push(self.eval(Point2::new(u0 + du * k as f64, v)));
                }
                return;
            }
            _ => dmath::sin_cos_steps(u0 + du, du, m - 1, trig),
        }
        match self {
            Surface::Plane(_) | Surface::Procedural(_) => {}
            Surface::Cylinder(c) => radial(&c.frame, c.radius, v, trig, out),
            Surface::Cone(c) => radial(&c.frame, c.radius_at(v), v, trig, out),
            Surface::Sphere(sp) => {
                let (sv, cv) = dmath::sin_cos(v);
                radial(&sp.frame, sp.radius * cv, sp.radius * sv, trig, out)
            }
            Surface::Torus(t) => {
                let (sv, cv) = dmath::sin_cos(v);
                radial(&t.frame, t.major + t.minor * cv, t.minor * sv, trig, out)
            }
        }
    }
}

macro_rules! dispatch {
    ($self:ident, $s:ident => $e:expr) => {
        match $self {
            Surface::Plane($s) => $e,
            Surface::Cylinder($s) => $e,
            Surface::Cone($s) => $e,
            Surface::Sphere($s) => $e,
            Surface::Torus($s) => $e,
            Surface::Procedural($s) => $e,
        }
    };
}

impl SurfaceGeom for Surface {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        dispatch!(self, s => s.deriv(uv))
    }
    fn eval(&self, uv: Point2) -> Point3 {
        match self {
            Surface::Procedural(s) => s.point(uv),
            _ => self.deriv(uv).p,
        }
    }
    fn normal(&self, uv: Point2) -> Option<UnitVec3> {
        dispatch!(self, s => s.normal(uv))
    }
    fn domain(&self) -> Domain2 {
        dispatch!(self, s => s.domain())
    }
    fn project(&self, p: Point3) -> Point2 {
        dispatch!(self, s => s.project(p))
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        dispatch!(self, s => s.bbox(uv))
    }
    fn transformed(&self, f: &Frame) -> Self {
        match self {
            Surface::Plane(s) => Surface::Plane(s.transformed(f)),
            Surface::Cylinder(s) => Surface::Cylinder(s.transformed(f)),
            Surface::Cone(s) => Surface::Cone(s.transformed(f)),
            Surface::Sphere(s) => Surface::Sphere(s.transformed(f)),
            Surface::Torus(s) => Surface::Torus(s.transformed(f)),
            Surface::Procedural(s) => Surface::Procedural(s.0.transformed(f)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::*;
    use forge_math::rng::Rng;

    #[test]
    fn derivatives_match_finite_differences() {
        let mut r = Rng::new(200);
        for _ in 0..500 {
            for s in rand_surfaces(&mut r) {
                let uv = rand_uv(&mut r, &s);
                let h = 1e-5;
                let d = s.deriv(uv);
                let du = |q: Point2| s.deriv(q).du;
                let dv = |q: Point2| s.deriv(q).dv;
                let pu = Point2::new(uv.x + h, uv.y);
                let mu = Point2::new(uv.x - h, uv.y);
                let pv = Point2::new(uv.x, uv.y + h);
                let mv = Point2::new(uv.x, uv.y - h);
                let close = |a: Vec3, b: Vec3, what: &str| {
                    assert!(
                        (a - b).length() < 1e-4 * (1.0 + b.length()),
                        "{what} {s:?} {uv:?}: {a:?} vs {b:?}"
                    );
                };
                close((s.eval(pu) - s.eval(mu)) / (2.0 * h), d.du, "du");
                close((s.eval(pv) - s.eval(mv)) / (2.0 * h), d.dv, "dv");
                close((du(pu) - du(mu)) / (2.0 * h), d.duu, "duu");
                close((du(pv) - du(mv)) / (2.0 * h), d.duv, "duv");
                close((dv(pv) - dv(mv)) / (2.0 * h), d.dvv, "dvv");
                if let Some(n) = s.normal(uv) {
                    let natural = d.du.cross(d.dv).normalize().unwrap();
                    assert!(n.dot(natural) > 1.0 - 1e-12, "normal {s:?}");
                }
            }
        }
    }

    #[test]
    fn project_inverts_eval_and_is_nearest() {
        let mut r = Rng::new(201);
        for _ in 0..2_000 {
            for s in rand_surfaces(&mut r) {
                let uv = rand_uv(&mut r, &s);
                let p = s.eval(uv);
                let back = s.eval(s.project(p));
                assert!(back.distance(p) < 1e-9, "{s:?} {uv:?}");
                // off-surface: moving along the normal and projecting returns to p
                if let Some(n) = s.normal(uv) {
                    let off = p + n * r.range(-0.05, 0.05) * curvature_safe(&s);
                    let q = s.eval(s.project(off));
                    assert!(q.distance(p) < 1e-8, "{s:?} {uv:?} {q:?} {p:?}");
                }
            }
        }
    }

    #[test]
    fn bbox_contains_samples() {
        let mut r = Rng::new(202);
        for _ in 0..300 {
            for s in rand_surfaces(&mut r) {
                let a = rand_uv(&mut r, &s);
                let b = rand_uv(&mut r, &s);
                let bx = Box2::new(Interval::new(a.x, b.x), Interval::new(a.y, b.y));
                let bb = s.bbox(bx);
                for i in 0..=10 {
                    for j in 0..=10 {
                        let q = Point2::new(
                            (bx.u.lo + bx.u.width() * i as f64 / 10.0).min(bx.u.hi),
                            (bx.v.lo + bx.v.width() * j as f64 / 10.0).min(bx.v.hi),
                        );
                        assert!(bb.contains(s.eval(q)), "{s:?} {bx:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn transform_moves_points() {
        let mut r = Rng::new(203);
        for _ in 0..200 {
            let f = rand_frame(&mut r);
            for s in rand_surfaces(&mut r) {
                let uv = rand_uv(&mut r, &s);
                assert!(s.transformed(&f).eval(uv).distance(f.point_to_world(s.eval(uv))) < 1e-9);
            }
        }
    }

    #[test]
    fn rows_match_pointwise_eval() {
        let mut r = Rng::new(204);
        let (mut trig, mut out) = (Vec::new(), Vec::new());
        for _ in 0..200 {
            for s in rand_surfaces(&mut r) {
                let a = rand_uv(&mut r, &s);
                let u1 = a.x + r.range(0.1, 3.0);
                let m = 2 + r.below(60);
                s.eval_row(a.y, a.x, u1, m, &mut trig, &mut out);
                assert_eq!(out.len(), m - 1);
                for (k, p) in out.iter().enumerate() {
                    let u = a.x + (u1 - a.x) * (k + 1) as f64 / m as f64;
                    assert!(p.distance(s.eval(Point2::new(u, a.y))) < 1e-9, "{s:?}");
                }
            }
        }
    }
}

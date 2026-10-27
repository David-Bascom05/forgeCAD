use crate::curve::Domain1;
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, UnitVec3, Vec3, dmath, tol};

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

/// A surface defined by construction rather than by a formula of its own: a rolling-ball blend,
/// a pipe along an exact spine, a corner patch spanning exact boundary curves. Exact: points are
/// solved for to full precision from the definition, never read off an approximation.
pub trait Procedural: Send + Sync + std::fmt::Debug {
    fn deriv(&self, uv: Point2) -> SurfDeriv;
    fn normal(&self, uv: Point2) -> Option<UnitVec3> {
        let d = self.deriv(uv);
        d.du.cross(d.dv).normalize()
    }
    fn domain(&self) -> Domain2;
    /// Parameters of the point of the surface nearest `p`.
    fn project(&self, p: Point3) -> Point2;
    /// A box guaranteed to contain the surface over the parameter box.
    fn bbox(&self, uv: Box2) -> Box3;
    /// Zero on the surface, positive on the natural normal's side, 1-Lipschitz (as
    /// `Surface::implicit`).
    fn implicit(&self, p: Point3) -> f64;
    /// Gradient of `implicit` at `p`.
    fn gradient(&self, p: Point3) -> Vec3;
    fn transformed(&self, f: &Frame) -> Proc;
    /// Lengths on the surface per unit of u and of v, roughly (for meshing).
    fn uv_scale(&self) -> (f64, f64) {
        (1.0, 1.0)
    }
    /// A radius of curvature no larger than the surface's tightest (for meshing).
    fn min_radius(&self) -> f64;
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
    /// Signed distance-like value: zero on the surface, positive on the side
    /// of its natural normal, and 1-Lipschitz (it changes no faster than the
    /// point moves), so a bound on it bounds where the surface can be.
    /// Gradient of `implicit` at p (unit length away from axes, poles and apexes).
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

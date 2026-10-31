//! Rolling-ball blends. A ball of radius r rolls touching two surfaces; its
//! centre runs along the spine, where the two surfaces offset by r meet, and
//! it touches each surface at the foot of the spine on it (the spring curves).
//! The swept surface is a pipe of radius r around the spine: a cylinder for a
//! straight spine, a torus for a circular one, otherwise a procedural pipe
//! that is exact by construction.

use crate::curve::{Circle, Curve, CurveDeriv, CurveGeom, Domain1, Line, ProcCurve, ProceduralCurve};
use crate::intersect::{SurfaceSurface, plane_surface};
use crate::surface::{Cylinder, Domain2, Plane, Proc, Procedural, SurfDeriv, Surface, SurfaceGeom, Torus};
use crate::varblend::VarBlend;
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, UnitVec3, Vec2, Vec3, dmath, tol};
use std::sync::Arc;

/// The surface moved by `d` along its natural normal, where that is the same
/// kind of surface (None where the offset collapses or self-intersects).
pub fn offset(s: &Surface, d: f64) -> Option<Surface> {
    Some(match s {
        Surface::Plane(p) => {
            Surface::Plane(Plane::new(p.frame.with_origin(p.frame.origin + p.frame.z() * d)))
        }
        Surface::Cylinder(c) => {
            if c.radius + d <= tol::LINEAR {
                return None;
            }
            Surface::Cylinder(Cylinder {
                frame: c.frame,
                radius: c.radius + d,
            })
        }
        Surface::Cone(c) => {
            // same axis and angle; radius at the frame origin grows by d / cos a
            let r = c.radius + d / c.cos_a();
            let (frame, r) = if r >= 0.0 {
                (c.frame, r)
            } else {
                // move the origin up the axis to where the radius is zero
                let h = -r / c.tan_a();
                (c.frame.with_origin(c.frame.origin + c.frame.z() * h), 0.0)
            };
            Surface::Cone(crate::surface::Cone::new(frame, r, dmath::atan(c.tan_a()))?)
        }
        Surface::Sphere(sp) => {
            if sp.radius + d <= tol::LINEAR {
                return None;
            }
            Surface::Sphere(crate::surface::Sphere {
                frame: sp.frame,
                radius: sp.radius + d,
            })
        }
        Surface::Torus(t) => {
            if t.minor + d <= tol::LINEAR {
                return None;
            }
            // a spindle torus past major: implicit and gradient still hold, it only feeds tracing
            Surface::Torus(Torus {
                frame: t.frame,
                major: t.major,
                minor: t.minor + d,
            })
        }
        Surface::Procedural(p) => return Offset::surface(p, d),
    })
}

/// A procedural surface moved `d` along its normal.
#[derive(Debug)]
pub struct Offset {
    base: Proc,
    d: f64,
}

impl Offset {
    /// `base` moved `d` along its normal, or None where that passes its tightest curvature.
    pub fn surface(base: &Proc, d: f64) -> Option<Surface> {
        if base.min_radius() <= -d + tol::MIN_LENGTH {
            return None;
        }
        Some(Surface::Procedural(Proc(Arc::new(Offset {
            base: base.clone(),
            d,
        }))))
    }

    /// The point, du and dv, from the base's derivatives and its normal's (Weingarten).
    fn first(&self, uv: Point2) -> (Point3, Vec3, Vec3) {
        let b = self.base.deriv(uv);
        let n = b.du.cross(b.dv);
        let len = n.length();
        if len == 0.0 {
            return (b.p, b.du, b.dv);
        }
        let unit = n / len;
        let dn = |m: Vec3| (m - unit * unit.dot(m)) / len;
        let nu = dn(b.duu.cross(b.dv) + b.du.cross(b.duv));
        let nv = dn(b.duv.cross(b.dv) + b.du.cross(b.dvv));
        (b.p + unit * self.d, b.du + nu * self.d, b.dv + nv * self.d)
    }
}

impl Procedural for Offset {
    fn point(&self, uv: Point2) -> Point3 {
        self.first(uv).0
    }
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        let (p, du, dv) = self.first(uv);
        let (su, sv) = self.base.uv_scale();
        let h = 1e-6;
        let (hu, hv) = (h / su.max(1e-9), h / sv.max(1e-9));
        let (_, u1, v1) = self.first(Point2::new(uv.x + hu, uv.y));
        let (_, u0, v0) = self.first(Point2::new(uv.x - hu, uv.y));
        let (_, _, v3) = self.first(Point2::new(uv.x, uv.y + hv));
        let (_, _, v2) = self.first(Point2::new(uv.x, uv.y - hv));
        SurfDeriv {
            p,
            du,
            dv,
            duu: (u1 - u0) / (2.0 * hu),
            duv: (v1 - v0) / (2.0 * hu),
            dvv: (v3 - v2) / (2.0 * hv),
        }
    }
    fn normal(&self, uv: Point2) -> Option<UnitVec3> {
        self.base.normal(uv)
    }
    fn domain(&self) -> Domain2 {
        self.base.domain()
    }
    fn project(&self, p: Point3) -> Point2 {
        self.base.project(p)
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        self.base.bbox(uv).grow(self.d.abs())
    }
    fn implicit(&self, p: Point3) -> f64 {
        let uv = self.project(p);
        match self.base.normal(uv) {
            Some(n) => n.dot(p - self.first(uv).0),
            None => self.base.implicit(p) - self.d,
        }
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        self.base
            .normal(self.project(p))
            .map_or_else(|| self.base.gradient(p), |n| n.get())
    }
    fn transformed(&self, f: &Frame) -> Proc {
        Proc(Arc::new(Offset {
            base: self.base.transformed(f),
            d: self.d,
        }))
    }
    fn uv_scale(&self) -> (f64, f64) {
        self.base.uv_scale()
    }
    fn min_radius(&self) -> f64 {
        (self.base.min_radius() + self.d).max(10.0 * tol::MIN_LENGTH)
    }
}

/// The point of `s` the ball centre `c` stands over, for a centre on `s` offset by `d`.
pub fn foot(s: &Surface, d: f64, c: Point3) -> Point3 {
    c - s.gradient(c) * d
}

/// A surface of revolution's profile in the (rho, z) half plane of an axis.
#[derive(Clone, Copy, Debug)]
enum Meridian {
    Line { p: Vec2, d: Vec2 },
    Circle { c: Vec2, r: f64 },
}

fn local_rz(f: &Frame, p: Point3) -> Vec2 {
    let l = f.point_to_local(p);
    Vec2::new((l.x * l.x + l.y * l.y).sqrt(), l.z)
}

fn on_axis(f: &Frame, p: Point3) -> bool {
    local_rz(f, p).x <= tol::LINEAR
}

fn parallel(a: UnitVec3, b: UnitVec3) -> bool {
    a.cross(b).length() <= tol::ANGULAR * 1e3
}

/// The surface's meridian about axis `f` (None when it is not a surface of
/// revolution about that axis).
fn meridian(s: &Surface, f: &Frame) -> Option<Meridian> {
    let z = f.z();
    let lz = |p: Point3| f.point_to_local(p).z;
    Some(match s {
        Surface::Plane(p) => {
            if !parallel(p.frame.z(), z) {
                return None;
            }
            Meridian::Line {
                p: Vec2::new(0.0, lz(p.frame.origin)),
                d: Vec2::X,
            }
        }
        Surface::Cylinder(c) => {
            if !parallel(c.frame.z(), z) || !on_axis(f, c.frame.origin) {
                return None;
            }
            Meridian::Line {
                p: Vec2::new(c.radius, 0.0),
                d: Vec2::Y,
            }
        }
        Surface::Cone(c) => {
            if !parallel(c.frame.z(), z) || !on_axis(f, c.frame.origin) {
                return None;
            }
            let s = c.frame.z().dot(z).signum();
            Meridian::Line {
                p: Vec2::new(c.radius, lz(c.frame.origin)),
                d: Vec2::new(s * c.sin_a(), c.cos_a()),
            }
        }
        Surface::Sphere(sp) => {
            if !on_axis(f, sp.frame.origin) {
                return None;
            }
            Meridian::Circle {
                c: Vec2::new(0.0, lz(sp.frame.origin)),
                r: sp.radius,
            }
        }
        Surface::Torus(t) => {
            if !parallel(t.frame.z(), z) || !on_axis(f, t.frame.origin) {
                return None;
            }
            Meridian::Circle {
                c: Vec2::new(t.major, lz(t.frame.origin)),
                r: t.minor,
            }
        }
        Surface::Procedural(_) => return None,
    })
}

fn meridian_hits(a: Meridian, b: Meridian) -> Vec<Vec2> {
    let line_circle = |p: Vec2, d: Vec2, c: Vec2, r: f64| {
        let d = d.normalize().unwrap_or(Vec2::X);
        let w = p - c;
        let (bq, cq) = (w.dot(d), w.dot(w) - r * r);
        let disc = bq * bq - cq;
        if disc < -tol::LINEAR * r {
            return vec![];
        }
        let s = disc.max(0.0).sqrt();
        vec![p + d * (-bq - s), p + d * (-bq + s)]
    };
    match (a, b) {
        (Meridian::Line { p, d }, Meridian::Line { p: q, d: e }) => {
            let den = d.cross(e);
            if den.abs() <= tol::ANGULAR {
                return vec![];
            }
            vec![p + d * ((q - p).cross(e) / den)]
        }
        (Meridian::Line { p, d }, Meridian::Circle { c, r })
        | (Meridian::Circle { c, r }, Meridian::Line { p, d }) => line_circle(p, d, c, r),
        (Meridian::Circle { c, r }, Meridian::Circle { c: k, r: s }) => {
            let off = k - c;
            let dist = off.length();
            if dist <= tol::LINEAR {
                return vec![];
            }
            let ex = off * (1.0 / dist);
            let x = (dist * dist + r * r - s * s) / (2.0 * dist);
            let h2 = r * r - x * x;
            if h2 < -tol::LINEAR * r {
                return vec![];
            }
            let h = h2.max(0.0).sqrt();
            vec![c + ex * x + ex.perp() * h, c + ex * x - ex.perp() * h]
        }
    }
}

/// An axis both surfaces may share: one of theirs, or the line of centres.
fn shared_axis(a: &Surface, b: &Surface) -> Option<Frame> {
    let axis = |s: &Surface| match s {
        Surface::Cylinder(c) => Some(c.frame),
        Surface::Cone(c) => Some(c.frame),
        Surface::Torus(t) => Some(t.frame),
        _ => None,
    };
    if let Some(f) = axis(a).or_else(|| axis(b)) {
        return Some(f);
    }
    match (a, b) {
        (Surface::Sphere(s), Surface::Plane(p)) | (Surface::Plane(p), Surface::Sphere(s)) => {
            Some(Frame::from_z(s.frame.origin, p.frame.z()))
        }
        (Surface::Sphere(s), Surface::Sphere(t)) => Some(Frame::from_z(
            s.frame.origin,
            (t.frame.origin - s.frame.origin).normalize()?,
        )),
        _ => None,
    }
}

/// Circles where two coaxial surfaces of revolution meet.
fn coaxial(a: &Surface, b: &Surface) -> Option<Vec<Curve>> {
    let f = shared_axis(a, b)?;
    let (ma, mb) = (meridian(a, &f)?, meridian(b, &f)?);
    Some(
        meridian_hits(ma, mb)
            .into_iter()
            .filter(|h| h.x > tol::LINEAR)
            .map(|h| {
                Curve::Circle(Circle {
                    frame: f.with_origin(f.origin + f.z() * h.y),
                    radius: h.x,
                })
            })
            .collect(),
    )
}

/// Lines where two cylinders with parallel axes meet.
fn parallel_cylinders(a: &Surface, b: &Surface) -> Option<Vec<Curve>> {
    let (Surface::Cylinder(c), Surface::Cylinder(d)) = (a, b) else {
        return None;
    };
    let z = c.frame.z();
    if !parallel(z, d.frame.z()) {
        return None;
    }
    let off = d.frame.origin - c.frame.origin;
    let off = off - z * z.dot(off);
    let dist = off.length();
    if dist <= tol::LINEAR {
        return Some(vec![]);
    }
    let ex = off / dist;
    let ey = z.cross(ex);
    let x = (dist * dist + c.radius * c.radius - d.radius * d.radius) / (2.0 * dist);
    let h2 = c.radius * c.radius - x * x;
    if h2 < -tol::LINEAR * c.radius {
        return Some(vec![]);
    }
    let h = h2.max(0.0).sqrt();
    Some(
        [-h, h]
            .into_iter()
            .map(|y| Curve::Line(Line::new(c.frame.origin + ex * x + ey * y, z)))
            .collect(),
    )
}

/// Whether the curve lies on both surfaces (sampled over `range`).
fn on_both(c: &Curve, range: Interval, a: &Surface, b: &Surface) -> bool {
    (0..=8).all(|i| {
        let p = c.eval(range.lo + range.width() * i as f64 / 8.0);
        a.implicit(p).abs() <= tol::LINEAR && b.implicit(p).abs() <= tol::LINEAR
    })
}

/// Where two surfaces meet, for the pairs with a closed form here: a plane
/// with anything analytic, parallel cylinders, coaxial surfaces of revolution.
/// None (or nothing found) leaves the pair to tracing.
pub fn closed_form(a: &Surface, b: &Surface) -> Option<Vec<Curve>> {
    let planar = match (a, b) {
        (Surface::Plane(p), s) | (s, Surface::Plane(p)) => match plane_surface(p, s) {
            SurfaceSurface::Curves(cs) => Some(cs.into_iter().map(|c| c.0).collect()),
            SurfaceSurface::Empty => Some(vec![]),
            _ => None,
        },
        _ => None,
    };
    planar
        .filter(|cs: &Vec<Curve>| !cs.is_empty())
        .or_else(|| parallel_cylinders(a, b))
        .or_else(|| coaxial(a, b))
        .filter(|cs| !cs.is_empty())
}

/// The branch of the intersection of `a` and `b` passing nearest `near`, in
/// closed form where the pair has one, traced within `region` otherwise.
pub fn spine(a: &Surface, b: &Surface, near: Point3, region: Box3) -> Option<Curve> {
    let closed = closed_form(a, b);
    let dist = |c: &Curve| {
        let r = c.full_range(1e6);
        c.eval(c.closest(near, r)).distance(near)
    };
    let pick = |cs: Vec<Curve>| cs.into_iter().min_by(|x, y| dist(x).total_cmp(&dist(y)));
    if let Some(c) = closed.and_then(pick)
        && on_both(&c, local_range(&c, near), a, b)
    {
        return Some(c);
    }
    let traced = crate::intcurve::trace(a, b, region)?;
    pick(
        traced
            .into_iter()
            .map(|c| Curve::Intersection(Arc::new(c)))
            .collect(),
    )
}

/// A short parameter range around the point of `c` nearest `p` (for checks).
fn local_range(c: &Curve, p: Point3) -> Interval {
    let t = c.closest(p, c.full_range(1e6));
    match c {
        Curve::Line(_) => Interval::new(t - 1.0, t + 1.0),
        Curve::Intersection(_) => {
            let d = c.domain();
            Interval::new((t - 1.0).max(d.lo), (t + 1.0).min(d.hi))
        }
        _ => Interval::new(t - 0.5, t + 0.5),
    }
}

/// The spine parameter nearest `p`. A traced spine's own search settles on
/// its guiding cubic; this finishes on the exact curve, stepping along its
/// exact direction until p - c(t) is square to it.
pub fn spine_closest(spine: &Curve, p: Point3) -> f64 {
    let mut t = spine.closest(p, spine.full_range(1e6));
    if let Curve::Intersection(ic) = spine {
        for _ in 0..12 {
            let d = spine.deriv(t);
            let Some(dir) = crate::intcurve::direction(&ic.a, &ic.b, d.p) else {
                break;
            };
            let dir = if dir.dot(d.d1) < 0.0 { -dir } else { dir };
            let speed = d.d1.length();
            if speed <= 0.0 {
                break;
            }
            let along = (p - d.p).dot(dir);
            t += along / speed;
            if along.abs() <= 1e-3 * tol::LINEAR {
                break;
            }
        }
    }
    t
}

/// A rolling-ball blend between surfaces `a` and `b`: the ball centre runs on
/// `a` offset by `da` and `b` offset by `db` (each +-radius along its natural normal).
#[derive(Clone, Debug)]
pub struct Blend {
    pub a: Surface,
    pub b: Surface,
    pub da: f64,
    pub db: f64,
    pub radius: f64,
    pub spine: Curve,
    /// The swept surface, its natural normal pointing away from the spine.
    pub surface: Surface,
    /// A variable radius: the blend is this canal surface and `radius` its largest radius.
    pub var: Option<Arc<VarBlend>>,
}

impl Blend {
    /// The blend whose spine passes nearest `near`, or None where the offsets
    /// do not meet there.
    pub fn new(a: &Surface, b: &Surface, da: f64, db: f64, near: Point3, region: Box3) -> Option<Blend> {
        let radius = da.abs();
        let (oa, ob) = (offset(a, da)?, offset(b, db)?);
        let spine = spine(&oa, &ob, near, region)?;
        let surface = match &spine {
            Curve::Line(l) => {
                let c = l.origin;
                let x = foot(a, da, c) - c;
                Surface::Cylinder(Cylinder {
                    frame: Frame::from_zx(c, l.dir, x).unwrap_or_else(|| Frame::from_z(c, l.dir)),
                    radius,
                })
            }
            Curve::Circle(k) if k.radius > radius + tol::LINEAR => Surface::Torus(Torus {
                frame: k.frame,
                major: k.radius,
                minor: radius,
            }),
            _ => Surface::Procedural(Pipe::build(spine.clone(), a.clone(), b.clone(), da, db, near)?),
        };
        Some(Blend {
            a: a.clone(),
            b: b.clone(),
            da,
            db,
            radius,
            spine,
            surface,
            var: None,
        })
    }

    /// The blend a variable-radius `vb` between `a` and `b` makes (its parameter is vb's s).
    pub fn variable(a: &Surface, b: &Surface, vb: Arc<VarBlend>) -> Blend {
        let radius = vb.radii().1;
        let (sa, sb) = vb.sides();
        Blend {
            a: a.clone(),
            b: b.clone(),
            da: sa * radius,
            db: sb * radius,
            radius,
            spine: vb.spine(),
            surface: vb.surface(),
            var: Some(vb),
        }
    }

    /// The points where the ball centred at spine parameter `t` touches `a` and `b`.
    pub fn feet(&self, t: f64) -> (Point3, Point3) {
        if let Some(v) = &self.var {
            return (v.spring(0).eval(t), v.spring(1).eval(t));
        }
        let c = self.spine.eval(t);
        (foot(&self.a, self.da, c), foot(&self.b, self.db, c))
    }

    /// The ball's cross-section circle at spine parameter `t`, with x towards
    /// the foot on `a`, and the angle at which it reaches the foot on `b`
    /// (in (0, pi), the short way round).
    pub fn section(&self, t: f64) -> Option<(Circle, f64)> {
        if let Some(v) = &self.var {
            return Some(v.section(t));
        }
        let c = self.spine.eval(t);
        let (fa, fb) = (foot(&self.a, self.da, c), foot(&self.b, self.db, c));
        let (wa, wb) = (fa - c, fb - c);
        let z = wa.cross(wb).normalize()?;
        let frame = Frame::from_zx(c, z, wa)?;
        let l = frame.point_to_local(fb);
        let angle = dmath::atan2(l.y, l.x);
        Some((
            Circle {
                frame,
                radius: self.radius,
            },
            angle,
        ))
    }

    /// The ball at t: its centre and radius. The section at t is its circle of contact, along
    /// which the blend's normal is the ball's.
    pub fn ball(&self, t: f64) -> (Point3, f64) {
        match &self.var {
            Some(v) => v.ball(t),
            None => (self.spine.eval(t), self.radius),
        }
    }

    /// The spring curve on `a` (`on_a`) or `b` in closed form, sharing the
    /// spine's parameter, or None where it has none.
    pub fn spring(&self, on_a: bool) -> Option<Curve> {
        if let Some(v) = &self.var {
            return Some(v.spring(usize::from(!on_a)));
        }
        let (s, d) = if on_a {
            (&self.a, self.da)
        } else {
            (&self.b, self.db)
        };
        let at = |t: f64| foot(s, d, self.spine.eval(t));
        let curve = match &self.spine {
            Curve::Line(l) => Curve::Line(Line::new(at(0.0), l.dir)),
            Curve::Circle(k) => {
                let f0 = at(0.0);
                let l = k.frame.point_to_local(f0);
                let rho = (l.x * l.x + l.y * l.y).sqrt();
                if rho <= tol::LINEAR || l.y.abs() > tol::LINEAR {
                    return None;
                }
                Curve::Circle(Circle {
                    frame: k.frame.with_origin(k.frame.origin + k.frame.z() * l.z),
                    radius: rho,
                })
            }
            _ => return Some(self.procedural_spring(on_a)),
        };
        let ok = [0.3, 1.1, 2.6, -1.7]
            .into_iter()
            .all(|t| curve.eval(t).distance(at(t)) <= tol::LINEAR);
        Some(if ok { curve } else { self.procedural_spring(on_a) })
    }

    fn procedural_spring(&self, on_a: bool) -> Curve {
        let (s, d) = if on_a {
            (&self.a, self.da)
        } else {
            (&self.b, self.db)
        };
        Curve::Procedural(ProcCurve(Arc::new(Spring {
            spine: self.spine.clone(),
            surface: s.clone(),
            d,
        })))
    }
}

/// An upper bound on the spine's speed over `range`.
fn curve_speed(c: &Curve, range: Interval) -> f64 {
    match c {
        Curve::Line(_) => 1.0,
        Curve::Circle(k) => k.radius,
        Curve::Ellipse(e) => e.major,
        Curve::Intersection(i) => i.speed_bound(),
        Curve::Procedural(p) => p.speed_bound(range),
        _ => {
            // parabola and hyperbola speeds grow away from the vertex: the ends bound them
            let a = c.deriv(range.lo).d1.length();
            let b = c.deriv(range.hi).d1.length();
            a.max(b).max(c.deriv(range.mid()).d1.length()) * (1.0 + 1e-9)
        }
    }
}

/// How fast the surface's unit gradient can turn per unit of travel, anywhere in `b`.
fn gradient_turn(s: &Surface, b: &Box3) -> f64 {
    let c = b.center();
    let half = 0.5 * b.diagonal();
    let from_axis = |f: &Frame| local_rz(f, c).x - half;
    let room = match s {
        Surface::Plane(_) => return 0.0,
        Surface::Cylinder(k) => from_axis(&k.frame),
        // the cone's gradient turns at most as fast as a cylinder's at the same distance
        Surface::Cone(k) => from_axis(&k.frame),
        Surface::Sphere(sp) => c.distance(sp.frame.origin) - half,
        Surface::Torus(t) => {
            let rz = local_rz(&t.frame, c);
            let core = Vec2::new(rz.x - t.major, rz.y).length();
            core.min(rz.x) - half
        }
        Surface::Procedural(p) => p.min_radius() * 0.5,
    };
    if room > 0.0 { 1.0 / room } else { f64::INFINITY }
}

/// Where the ball touches a surface as its centre runs along the spine: the
/// foot of the spine on the surface, a blend's spring curve. It shares the
/// spine's parameter.
#[derive(Debug)]
pub struct Spring {
    pub spine: Curve,
    pub surface: Surface,
    /// The spine lies on the surface offset by `d` along its natural normal.
    pub d: f64,
}

impl Spring {
    fn at(&self, t: f64) -> Point3 {
        foot(&self.surface, self.d, self.spine.eval(t))
    }
    fn step(&self) -> f64 {
        let d = self.spine.domain();
        let w = if d.lo.is_finite() && d.hi.is_finite() {
            d.hi - d.lo
        } else {
            1.0
        };
        1e-5 * w.clamp(1e-3, 1e3)
    }
}

impl ProceduralCurve for Spring {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let h = self.step();
        let (p, a, b) = (self.at(t), self.at(t + h), self.at(t - h));
        CurveDeriv {
            p,
            d1: (a - b) / (2.0 * h),
            d2: (a.to_vec() + b.to_vec() - p.to_vec() * 2.0) / (h * h),
        }
    }
    fn domain(&self) -> Domain1 {
        self.spine.domain()
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let n = self.segments(range, 1e-2).clamp(16, 256);
        let at = |i: usize| (range.lo + range.width() * i as f64 / n as f64).min(range.hi);
        let mut best = (f64::INFINITY, 0);
        for i in 0..=n {
            let d = self.at(at(i)).distance_sq(p);
            if d < best.0 {
                best = (d, i);
            }
        }
        let (lo, hi) = (at(best.1.saturating_sub(1)), at((best.1 + 1).min(n)));
        let mut t = at(best.1);
        for _ in 0..30 {
            let d = self.deriv(t);
            let g = (d.p - p).dot(d.d1);
            let h = d.d1.dot(d.d1) + (d.p - p).dot(d.d2);
            if h <= 0.0 {
                break;
            }
            let next = (t - g / h).clamp(lo, hi);
            let done = (next - t).abs() <= 1e-14 * (1.0 + t.abs());
            t = next;
            if done {
                break;
            }
        }
        if self.at(t).distance_sq(p) <= best.0 {
            t
        } else {
            at(best.1)
        }
    }
    fn bbox(&self, range: Interval) -> Box3 {
        self.spine.bbox(range).grow(self.d.abs() + tol::LINEAR)
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(Spring {
            spine: self.spine.transformed(f),
            surface: self.surface.transformed(f),
            d: self.d,
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        // foot = c - d grad(c): its speed is at most |c'| (1 + |d| L), L the gradient's turn rate,
        // bounded piece by piece: one box round a whole turn about an axis would hold the axis
        let n = 16;
        let w = range.width() / n as f64;
        (0..n)
            .map(|i| {
                let r = Interval::new(range.lo + w * i as f64, range.lo + w * (i + 1) as f64);
                let b = self.spine.bbox(r);
                curve_speed(&self.spine, r) * (1.0 + self.d.abs() * gradient_turn(&self.surface, &b))
            })
            .fold(0.0, f64::max)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        // a chord of parameter length h sags at most h^2 max|P''| / 8
        let m = 16;
        let mut acc = 0.0f64;
        for i in 0..=m {
            let t = (range.lo + range.width() * i as f64 / m as f64).min(range.hi);
            acc = acc.max(self.deriv(t).d2.length());
        }
        let acc = acc * 1.5 + f64::MIN_POSITIVE;
        let h = (8.0 * chord / acc).sqrt();
        ((range.width() / h).ceil() as usize).max(1)
    }
}

/// A pipe of radius r around an exact spine, spanning the arc between the
/// ball's contacts with two surfaces. u is the spine parameter; v runs over
/// the contact arc, 0 at the first contact and 1 at the second.
#[derive(Debug)]
pub struct Pipe {
    pub spine: Curve,
    pub first: Surface,
    pub second: Surface,
    pub d_first: f64,
    pub d_second: f64,
    pub radius: f64,
}

impl Pipe {
    /// Contact order is chosen so the natural normal points away from the spine.
    pub fn build(spine: Curve, a: Surface, b: Surface, da: f64, db: f64, near: Point3) -> Option<Proc> {
        let mut p = Pipe {
            spine,
            first: a,
            second: b,
            d_first: da,
            d_second: db,
            radius: da.abs(),
        };
        let t = p.spine.closest(near, p.spine.full_range(1e6));
        let (_, e1, e2, _) = p.cross(t)?;
        if e1.cross(e2).dot(p.spine.deriv(t).d1) > 0.0 {
            std::mem::swap(&mut p.first, &mut p.second);
            std::mem::swap(&mut p.d_first, &mut p.d_second);
        }
        Some(Proc(Arc::new(p)))
    }

    /// Centre, unit direction to the first contact, the unit perpendicular in
    /// the section towards the second, and the angle between the contacts.
    fn cross(&self, t: f64) -> Option<(Point3, Vec3, Vec3, f64)> {
        let c = self.spine.eval(t);
        let wa = (foot(&self.first, self.d_first, c) - c).normalize()?.get();
        let wb = (foot(&self.second, self.d_second, c) - c).normalize()?.get();
        let e2 = (wb - wa * wa.dot(wb)).normalize()?.get();
        let theta = dmath::atan2(wb.dot(e2), wb.dot(wa));
        Some((c, wa, e2, theta))
    }

    fn point(&self, uv: Point2) -> Point3 {
        match self.cross(uv.x) {
            Some((c, e1, e2, th)) => {
                let (s, k) = dmath::sin_cos(uv.y * th);
                c + (e1 * k + e2 * s) * self.radius
            }
            None => self.spine.eval(uv.x),
        }
    }
}

impl Procedural for Pipe {
    fn point(&self, uv: Point2) -> Point3 {
        Pipe::point(self, uv)
    }
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        let r = self.radius;
        let Some((c, e1, e2, th)) = self.cross(uv.x) else {
            let p = self.spine.eval(uv.x);
            return SurfDeriv {
                p,
                du: Vec3::ZERO,
                dv: Vec3::ZERO,
                duu: Vec3::ZERO,
                duv: Vec3::ZERO,
                dvv: Vec3::ZERO,
            };
        };
        let (s, k) = dmath::sin_cos(uv.y * th);
        let p = c + (e1 * k + e2 * s) * r;
        let dv = (e2 * k - e1 * s) * (r * th);
        let dvv = (e1 * k + e2 * s) * (-r * th * th);
        // along the spine: central differences of the exact surface
        let h = 1e-4 * self.spine.deriv(uv.x).d1.length().max(1e-9).recip().min(1.0);
        let (pp, pm) = (
            self.point(Point2::new(uv.x + h, uv.y)),
            self.point(Point2::new(uv.x - h, uv.y)),
        );
        let dvp = |u: f64| match self.cross(u) {
            Some((_, e1, e2, th)) => {
                let (s, k) = dmath::sin_cos(uv.y * th);
                (e2 * k - e1 * s) * (r * th)
            }
            None => Vec3::ZERO,
        };
        SurfDeriv {
            p,
            du: (pp - pm) / (2.0 * h),
            dv,
            duu: (pp + pm.to_vec() - p.to_vec() * 2.0).to_vec() / (h * h),
            duv: (dvp(uv.x + h) - dvp(uv.x - h)) / (2.0 * h),
            dvv,
        }
    }
    /// Exactly away from the spine (the contact order makes du x dv point that way).
    fn normal(&self, uv: Point2) -> Option<UnitVec3> {
        let (_, e1, e2, th) = self.cross(uv.x)?;
        let (s, k) = dmath::sin_cos(uv.y * th);
        (e1 * k + e2 * s).normalize()
    }
    fn domain(&self) -> Domain2 {
        Domain2 {
            u: self.spine.domain(),
            v: Domain1 {
                lo: -1.0,
                hi: 2.0,
                period: None,
            },
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        let u = spine_closest(&self.spine, p);
        match self.cross(u) {
            Some((c, e1, e2, th)) => {
                let q = p - c;
                Point2::new(u, dmath::atan2(q.dot(e2), q.dot(e1)) / th)
            }
            None => Point2::new(u, 0.0),
        }
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        self.spine.bbox(uv.u).grow(self.radius)
    }
    fn implicit(&self, p: Point3) -> f64 {
        let u = spine_closest(&self.spine, p);
        p.distance(self.spine.eval(u)) - self.radius
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        let u = spine_closest(&self.spine, p);
        (p - self.spine.eval(u)).normalize().map_or(Vec3::Z, |n| n.get())
    }
    fn transformed(&self, f: &Frame) -> Proc {
        Proc(Arc::new(Pipe {
            spine: self.spine.transformed(f),
            first: self.first.transformed(f),
            second: self.second.transformed(f),
            d_first: self.d_first,
            d_second: self.d_second,
            radius: self.radius,
        }))
    }
    fn uv_scale(&self) -> (f64, f64) {
        let d = self.spine.domain();
        let t = if d.lo.is_finite() {
            0.5 * (d.lo + d.hi)
        } else {
            0.0
        };
        let th = self.cross(t).map_or(1.0, |x| x.3.abs());
        (self.spine.deriv(t).d1.length().max(1e-9), self.radius * th)
    }
    fn min_radius(&self) -> f64 {
        self.radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::{Cone, Sphere};

    fn plane(o: Point3, n: Vec3) -> Surface {
        Surface::Plane(Plane::new(Frame::from_z(o, n.normalize().unwrap())))
    }

    #[test]
    fn plane_plane_gives_a_cylinder_tangent_to_both() {
        // a convex box edge along x at y = 0, z = 0, material at y < 0, z < 0
        let top = plane(Point3::ORIGIN, Vec3::Z);
        let side = plane(Point3::ORIGIN, Vec3::Y);
        let r = 2.0;
        let region = Box3::from_points([Point3::new(-10.0, -10.0, -10.0), Point3::new(10.0, 10.0, 10.0)]);
        let b = Blend::new(&top, &side, -r, -r, Point3::ORIGIN, region).unwrap();
        let Surface::Cylinder(c) = &b.surface else {
            panic!("{:?}", b.surface)
        };
        assert!((c.radius - r).abs() < 1e-12);
        let t = spine_closest(&b.spine, Point3::new(3.0, 0.0, 0.0));
        let (fa, fb) = b.feet(t);
        assert!(fa.distance(Point3::new(3.0, -2.0, 0.0)) < 1e-12, "{fa:?}");
        assert!(fb.distance(Point3::new(3.0, 0.0, -2.0)) < 1e-12, "{fb:?}");
        let (k, ang) = b.section(t).unwrap();
        assert!((ang - dmath::FRAC_PI_2).abs() < 1e-12);
        assert!(k.eval(ang).distance(fb) < 1e-12);
        let sa = b.spring(true).unwrap();
        assert!(sa.eval(t).distance(fa) < 1e-12);
    }

    #[test]
    fn plane_cylinder_gives_a_torus() {
        // a boss of radius 5 on a plate: concave edge, ball outside the material
        let boss = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 5.0,
        });
        let plate = plane(Point3::ORIGIN, Vec3::Z);
        let region = Box3::from_points([Point3::new(-10.0, -10.0, -10.0), Point3::new(10.0, 10.0, 10.0)]);
        let b = Blend::new(&boss, &plate, 1.0, 1.0, Point3::new(5.0, 0.0, 0.0), region).unwrap();
        let Surface::Torus(t) = &b.surface else {
            panic!("{:?}", b.surface)
        };
        assert!((t.major - 6.0).abs() < 1e-12 && (t.minor - 1.0).abs() < 1e-12);
        assert!((t.frame.origin.z - 1.0).abs() < 1e-12);
        for on_a in [true, false] {
            let s = b.spring(on_a).unwrap();
            for t in [0.0, 1.0, 4.0] {
                let (fa, fb) = b.feet(t);
                assert!(s.eval(t).distance(if on_a { fa } else { fb }) < 1e-12);
            }
        }
    }

    #[test]
    fn coaxial_cone_and_sphere_meet_in_circles() {
        let cone = Surface::Cone(Cone::new(Frame::WORLD, 3.0, 0.5).unwrap());
        let sphere = Surface::Sphere(Sphere {
            frame: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 2.0)),
            radius: 6.0,
        });
        let cs = coaxial(&cone, &sphere).unwrap();
        assert!(!cs.is_empty());
        for c in &cs {
            assert!(on_both(c, Interval::new(0.0, 6.0), &cone, &sphere));
        }
    }

    #[test]
    fn pipe_around_a_traced_spine_is_tangent_to_both_surfaces() {
        // a cylinder crossing a sphere: no closed-form spine, the pipe is procedural
        let cyl = Surface::Cylinder(Cylinder {
            frame: Frame::from_z(Point3::new(0.0, 0.0, 0.0), UnitVec3::X),
            radius: 3.0,
        });
        let sph = Surface::Sphere(Sphere {
            frame: Frame::WORLD.with_origin(Point3::new(0.0, 1.0, 0.0)),
            radius: 8.0,
        });
        let r = 1.0;
        let region = Box3::from_points([Point3::new(-12.0, -12.0, -12.0), Point3::new(12.0, 12.0, 12.0)]);
        // concave between the rod's outside and the ball's outside
        let near = Point3::new(7.5, 3.0, 0.0);
        let b = Blend::new(&cyl, &sph, r, r, near, region).unwrap();
        let Surface::Procedural(p) = &b.surface else {
            panic!("expected a pipe")
        };
        let dom = b.spine.domain();
        for i in 0..20 {
            let u = dom.lo + (dom.hi - dom.lo) * (i as f64 + 0.5) / 20.0;
            for v in [0.0, 0.3, 1.0] {
                let q = p.deriv(Point2::new(u, v)).p;
                assert!(p.implicit(q).abs() < 1e-9);
                let back = p.project(q);
                assert!(
                    p.deriv(back).p.distance(q) < 1e-8,
                    "{u} {v} {back:?} {} {:?}",
                    p.deriv(back).p.distance(q),
                    b.spine.domain()
                );
            }
            // at the contacts the pipe's normal is the surface's normal: G1 by construction
            let (fa, fb) = b.feet(u);
            assert!(cyl.distance(fa) < 1e-9 && sph.distance(fb) < 1e-9);
            for v in [0.0, 1.0] {
                let q = p.deriv(Point2::new(u, v)).p;
                let s = if cyl.distance(q) < sph.distance(q) {
                    &cyl
                } else {
                    &sph
                };
                assert!(s.distance(q) < 1e-9);
                let n = p.normal(Point2::new(u, v)).unwrap();
                assert!(n.cross(s.gradient(q)).length() < 1e-12, "{u}");
            }
            let d = p.deriv(Point2::new(u, 0.5));
            let natural = d.du.cross(d.dv).normalize().unwrap();
            assert!(natural.dot(p.normal(Point2::new(u, 0.5)).unwrap()) > 1.0 - 1e-9);
        }
        // the springs are procedural feet of the spine, exactly on their surfaces
        for (on_a, s) in [(true, &cyl), (false, &sph)] {
            let c = b.spring(on_a).unwrap();
            assert!(matches!(c, Curve::Procedural(_)));
            let range = Interval::new(dom.lo + 0.5, dom.hi - 0.5);
            let n = 50;
            for i in 0..=n {
                let t = range.lo + range.width() * i as f64 / n as f64;
                let q = c.eval(t);
                assert!(s.distance(q) < 1e-9);
                assert!((c.closest(q, range) - t).abs() < 1e-6, "{t}");
                assert!(c.bbox(range).contains(q));
            }
            let k = match &c {
                Curve::Procedural(k) => k.clone(),
                _ => unreachable!(),
            };
            let m = k.segments(range, 1e-3);
            for i in 0..m {
                let (t0, t1) = (
                    range.lo + range.width() * i as f64 / m as f64,
                    range.lo + range.width() * (i + 1) as f64 / m as f64,
                );
                let mid = c.eval(0.5 * (t0 + t1));
                assert!(mid.distance(c.eval(t0).lerp(c.eval(t1), 0.5)) <= 1e-3);
            }
            let v = k.speed_bound(range);
            for i in 0..n {
                let t = range.lo + range.width() * i as f64 / n as f64;
                assert!(c.deriv(t).d1.length() <= v);
            }
        }
    }
}

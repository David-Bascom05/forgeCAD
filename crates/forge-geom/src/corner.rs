//! Vertex blends: the surface that closes the hole where blends meet at a corner. When one
//! ball touches every side the hole lies on that ball's sphere. Otherwise it is an n-sided
//! transfinite patch: a formula over the exact boundary curves and the normals of the
//! surfaces beyond them, so it interpolates them exactly and meets them with G1.

use crate::curve::{Curve, CurveDeriv, CurveGeom, Domain1};
use crate::surface::{Domain2, Proc, Procedural, Sphere, SurfDeriv, Surface, SurfaceGeom};
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, Vec2, Vec3, dmath, linalg, tol};
use std::ops::{Add, Mul, Sub};
use std::sync::Arc;

mod rational;
pub use rational::{Proof as RationalProof, clear_proof_cache};

/// A boundary point, the outward normal of the surface beyond it, and their t derivatives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailJet {
    pub p: Point3,
    pub dp: Vec3,
    pub ddp: Vec3,
    pub dddp: Vec3,
    pub n: Vec3,
    pub dn: Vec3,
    pub ddn: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailBound {
    pub p: Box3,
    pub n: Box3,
    pub dp: Box3,
}

/// A boundary curve of a hole together with the outward normal of the neighbouring surface
/// along it: everything the patch needs from one side.
pub trait Rail: Send + Sync + std::fmt::Debug {
    fn jet(&self, t: f64) -> RailJet;
    /// Point, its t derivative and the outward normal.
    fn frame(&self, t: f64) -> (Point3, Vec3, Vec3) {
        let j = self.jet(t);
        (j.p, j.dp, j.n)
    }
    /// Point and outward normal only.
    fn point(&self, t: f64) -> (Point3, Vec3) {
        let j = self.jet(t);
        (j.p, j.n)
    }
    /// Boxes guaranteed to hold the points, the normals and the t derivatives over `t`.
    fn bound(&self, t: Interval) -> RailBound;
    fn transformed(&self, f: &Frame) -> Arc<dyn Rail>;
    fn surface_rail(&self) -> Option<&SurfaceRail> {
        None
    }
    fn spoke(&self) -> Option<&SpokeRail> {
        None
    }
}

/// A curve lying on a surface; the normal is the surface's, flipped when not `same_sense`.
#[derive(Clone, Debug)]
pub struct SurfaceRail {
    pub curve: Curve,
    pub surface: Surface,
    sign: f64,
}

impl SurfaceRail {
    pub fn new(curve: Curve, surface: Surface, same_sense: bool) -> Self {
        SurfaceRail {
            curve,
            surface,
            sign: if same_sense { 1.0 } else { -1.0 },
        }
    }

    /// Natural normal and its first two t derivatives along the curve (only n when !full).
    fn normal(&self, c: &CurveDeriv, t: f64, full: bool) -> (Vec3, Vec3, Vec3) {
        let z0 = Vec3::ZERO;
        match &self.surface {
            Surface::Plane(pl) => (pl.frame.z().get(), z0, z0),
            Surface::Sphere(s) => {
                let k = 1.0 / s.radius;
                ((c.p - s.frame.origin) * k, c.d1 * k, c.d2 * k)
            }
            Surface::Cylinder(cy) => {
                let z = cy.frame.z();
                let flat = |v: Vec3| v - z * z.dot(v);
                let k = 1.0 / cy.radius;
                (flat(c.p - cy.frame.origin) * k, flat(c.d1) * k, flat(c.d2) * k)
            }
            Surface::Cone(co) => {
                let z = co.frame.z();
                let flat = |v: Vec3| v - z * z.dot(v);
                let (u, u1, u2) = unit_jet(flat(c.p - co.frame.origin), flat(c.d1), flat(c.d2));
                (u * co.cos_a() - z * co.sin_a(), u1 * co.cos_a(), u2 * co.cos_a())
            }
            Surface::Torus(to) => {
                let z = to.frame.z();
                let flat = |v: Vec3| v - z * z.dot(v);
                let (u, u1, u2) = unit_jet(flat(c.p - to.frame.origin), flat(c.d1), flat(c.d2));
                let k = 1.0 / to.minor;
                (
                    (c.p - to.frame.origin - u * to.major) * k,
                    (c.d1 - u1 * to.major) * k,
                    (c.d2 - u2 * to.major) * k,
                )
            }
            Surface::Procedural(pr) => {
                let first = |c: &CurveDeriv| shape_dn(&pr.deriv(pr.project(c.p)), c.d1);
                let (n, n1) = first(c);
                if !full {
                    return (n, z0, z0);
                }
                // the third derivatives of the surface are not available: n'' from exact n'
                let h = 1e-5 * (1.0 + t.abs());
                let a = first(&self.curve.deriv(t + h)).1;
                let b = first(&self.curve.deriv(t - h)).1;
                (n, n1, (a - b) / (2.0 * h))
            }
        }
    }

    /// An upper bound on the normal curvature of the surface near the box.
    fn curvature_bound(&self, pb: &Box3) -> f64 {
        match &self.surface {
            Surface::Plane(_) => 0.0,
            Surface::Sphere(s) => 1.0 / s.radius,
            Surface::Cylinder(c) => 1.0 / c.radius,
            Surface::Torus(t) => (1.0 / t.minor).max(1.0 / (t.major - t.minor)),
            Surface::Cone(c) => {
                let l = flat_box(&c.frame, pb);
                let rho = (l[0].sqr() + l[1].sqr() + l[2].sqr()).sqrt();
                if rho.lo > 0.0 {
                    c.cos_a() / rho.lo
                } else {
                    f64::INFINITY
                }
            }
            Surface::Procedural(p) => 1.0 / p.mesh_radius(),
        }
    }
}

/// Natural unit normal of a surface and its derivative along the tangent `dp` (Weingarten).
fn shape_dn(d: &SurfDeriv, dp: Vec3) -> (Vec3, Vec3) {
    let Some(n) = d.du.cross(d.dv).normalize() else {
        return (Vec3::ZERO, Vec3::ZERO);
    };
    let n = n.get();
    let (e, f, g) = (d.du.dot(d.du), d.du.dot(d.dv), d.dv.dot(d.dv));
    let (l, m, nn) = (d.duu.dot(n), d.duv.dot(n), d.dvv.dot(n));
    let det = e * g - f * f;
    let Some((a, b)) = linalg::solve2(e, f, f, g, dp.dot(d.du), dp.dot(d.dv)) else {
        return (n, Vec3::ZERO);
    };
    let nu = (d.du * (m * f - l * g) + d.dv * (l * f - m * e)) / det;
    let nv = (d.du * (nn * f - m * g) + d.dv * (m * f - nn * e)) / det;
    (n, nu * a + nv * b)
}

/// The foot on `s` of a point `c` moving with velocity `dc` at a fixed distance from the
/// surface (a ball centre rolling along it), and the foot's velocity.
pub fn foot_velocity(s: &Surface, c: Point3, dc: Vec3) -> (Point3, Vec3) {
    let d = s.deriv(s.project(c));
    let Some(n) = d.du.cross(d.dv).normalize() else {
        return (d.p, dc);
    };
    let n = n.get();
    let h = (c - d.p).dot(n);
    let (e, f, g) = (d.du.dot(d.du), d.du.dot(d.dv), d.dv.dot(d.dv));
    let (l, m, nn) = (d.duu.dot(n), d.duv.dot(n), d.dvv.dot(n));
    let det = e * g - f * f;
    let nu = (d.du * (m * f - l * g) + d.dv * (l * f - m * e)) / det;
    let nv = (d.du * (nn * f - m * g) + d.dv * (m * f - nn * e)) / det;
    // c' = (S_u + h N_u) a + (S_v + h N_v) b, and the foot moves by S_u a + S_v b
    let (cu, cv) = (d.du + nu * h, d.dv + nv * h);
    let (a, b) = linalg::solve2(
        cu.dot(cu),
        cu.dot(cv),
        cu.dot(cv),
        cv.dot(cv),
        dc.dot(cu),
        dc.dot(cv),
    )
    .unwrap_or((0.0, 0.0));
    (d.p, d.du * a + d.dv * b)
}

/// w / |w| and its first two derivatives.
fn unit_jet(w: Vec3, w1: Vec3, w2: Vec3) -> (Vec3, Vec3, Vec3) {
    let rho = w.length();
    let u = w / rho;
    let r1 = u.dot(w1);
    let r2 = (w1.dot(w1) + w.dot(w2) - r1 * r1) / rho;
    let u1 = (w1 - u * r1) / rho;
    let u2 = (w2 - u1 * (2.0 * r1) - u * r2) / rho;
    (u, u1, u2)
}

type IVec = [Interval; 3];

fn ivec(b: &Box3) -> IVec {
    [b.x, b.y, b.z]
}

fn ibox(v: IVec) -> Box3 {
    Box3 {
        x: v[0],
        y: v[1],
        z: v[2],
    }
}

fn idot(a: IVec, b: IVec) -> Interval {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn meet(a: Interval, b: Interval) -> Interval {
    Interval {
        lo: a.lo.max(b.lo),
        hi: a.hi.min(b.hi),
    }
}

const UNIT: Interval = Interval { lo: -1.0, hi: 1.0 };

/// Box of (p - origin) with its component along the frame's z removed.
fn flat_box(f: &Frame, pb: &Box3) -> IVec {
    let o = f.origin;
    let v = [pb.x + -o.x, pb.y + -o.y, pb.z + -o.z];
    let z = f.z().get();
    let zi = [Interval::point(z.x), Interval::point(z.y), Interval::point(z.z)];
    let along = idot(v, zi);
    [v[0] - along * z.x, v[1] - along * z.y, v[2] - along * z.z]
}

/// Box of x a + y b for vectors x, y and intervals a, b.
fn comb(x: Vec3, a: Interval, y: Vec3, b: Interval) -> Box3 {
    Box3 {
        x: a * x.x + b * y.x,
        y: a * x.y + b * y.y,
        z: a * x.z + b * y.z,
    }
}

/// A box holding d curve / dt over `t`.
pub fn derivative_box(c: &Curve, t: Interval) -> Box3 {
    match c {
        Curve::Line(l) => Box3::from_point(Point3::ORIGIN + l.dir.get()),
        Curve::Circle(k) => comb(
            k.frame.x().get(),
            -t.sin() * k.radius,
            k.frame.y().get(),
            t.cos() * k.radius,
        ),
        Curve::Ellipse(e) => comb(
            e.frame.x().get(),
            -t.sin() * e.major,
            e.frame.y().get(),
            t.cos() * e.minor,
        ),
        Curve::Parabola(p) => comb(
            p.frame.x().get(),
            t * (1.0 / (2.0 * p.focal)),
            p.frame.y().get(),
            Interval::point(1.0),
        ),
        Curve::Hyperbola(h) => {
            // a t / sqrt(1 + t^2) rises with t
            let f = |t: f64| h.a * t / (1.0 + t * t).sqrt();
            let a = Interval::new(f(t.lo).next_down().next_down(), f(t.hi).next_up().next_up());
            comb(h.frame.x().get(), a, h.frame.y().get(), Interval::point(h.b))
        }
        Curve::Intersection(_) | Curve::Procedural(_) => {
            // no closed form: the middle value, widened by twice the largest sampled |P''|
            let m = (0..=4)
                .map(|k| c.deriv(t.lo + t.width() * k as f64 / 4.0).d2.length())
                .fold(0.0, f64::max);
            let d = c.deriv(t.mid()).d1;
            let e = 2.0 * m * 0.5 * t.width() + 8.0 * f64::EPSILON * d.length();
            Box3::from_point(Point3::ORIGIN + d).grow(e)
        }
    }
}

/// d^3 curve / dt^3: closed forms for the conics, else a central difference of the exact second.
fn third_derivative(c: &Curve, t: f64, d: &CurveDeriv) -> Vec3 {
    match c {
        Curve::Line(_) | Curve::Parabola(_) => Vec3::ZERO,
        // angle parameter: P''' = -P'
        Curve::Circle(_) | Curve::Ellipse(_) => -d.d1,
        Curve::Hyperbola(h) => {
            let f = (1.0 + t * t).sqrt();
            h.frame.x() * (-3.0 * h.a * t / (f * f * f * f * f))
        }
        Curve::Intersection(_) | Curve::Procedural(_) => {
            let h = 1e-5 * (1.0 + t.abs());
            (c.deriv(t + h).d2 - c.deriv(t - h).d2) / (2.0 * h)
        }
    }
}

/// A bound on |d curve / dt| over `t`.
pub fn speed_bound(c: &Curve, t: Interval) -> f64 {
    match c {
        Curve::Line(_) => 1.0,
        Curve::Circle(k) => k.radius,
        Curve::Ellipse(e) => e.major,
        Curve::Parabola(p) => {
            let m = t.lo.abs().max(t.hi.abs()) / (2.0 * p.focal);
            (1.0 + m * m).sqrt().next_up()
        }
        Curve::Hyperbola(h) => (h.a * h.a + h.b * h.b).sqrt().next_up(),
        Curve::Intersection(i) => i.speed_bound(),
        Curve::Procedural(p) => p.speed_bound(t),
    }
}

impl Rail for SurfaceRail {
    fn surface_rail(&self) -> Option<&SurfaceRail> {
        Some(self)
    }
    fn frame(&self, t: f64) -> (Point3, Vec3, Vec3) {
        let c = self.curve.deriv(t);
        (c.p, c.d1, self.normal(&c, t, false).0 * self.sign)
    }
    fn jet(&self, t: f64) -> RailJet {
        let c = self.curve.deriv(t);
        let (n, dn, ddn) = self.normal(&c, t, true);
        let s = self.sign;
        RailJet {
            p: c.p,
            dp: c.d1,
            ddp: c.d2,
            dddp: third_derivative(&self.curve, t, &c),
            n: n * s,
            dn: dn * s,
            ddn: ddn * s,
        }
    }
    fn point(&self, t: f64) -> (Point3, Vec3) {
        let c = match &self.surface {
            // only the position is needed
            Surface::Plane(_) | Surface::Sphere(_) | Surface::Cylinder(_) => CurveDeriv {
                p: self.curve.eval(t),
                d1: Vec3::ZERO,
                d2: Vec3::ZERO,
            },
            _ => self.curve.deriv(t),
        };
        (c.p, self.normal(&c, t, false).0 * self.sign)
    }
    fn bound(&self, t: Interval) -> RailBound {
        let pb = self.curve.bbox(t);
        let s = self.sign;
        let n = match &self.surface {
            Surface::Plane(pl) => {
                let z = pl.frame.z().get();
                [Interval::point(z.x), Interval::point(z.y), Interval::point(z.z)]
            }
            Surface::Sphere(sp) => {
                let (o, k) = (sp.frame.origin, 1.0 / sp.radius);
                [(pb.x + -o.x) * k, (pb.y + -o.y) * k, (pb.z + -o.z) * k]
            }
            Surface::Cylinder(c) => flat_box(&c.frame, &pb).map(|i| i * (1.0 / c.radius)),
            _ => {
                let tm = t.mid();
                let n0 = self.normal(&self.curve.deriv(tm), tm, false).0;
                // the normal turns by at most curvature x arc length from the middle
                let e = self.curvature_bound(&pb) * speed_bound(&self.curve, t) * 0.5 * t.width();
                let e = (e + 8.0 * f64::EPSILON).next_up();
                [n0.x, n0.y, n0.z].map(|c| Interval::new(c - e, c + e))
            }
        };
        let n = n.map(|i| meet(if s < 0.0 { -i } else { i }, UNIT));
        RailBound {
            p: pb,
            n: ibox(n),
            dp: derivative_box(&self.curve, t),
        }
    }
    fn transformed(&self, f: &Frame) -> Arc<dyn Rail> {
        Arc::new(SurfaceRail {
            curve: self.curve.transformed(f),
            surface: self.surface.transformed(f),
            sign: self.sign,
        })
    }
}

/// One side of a hole: a rail run from `t0` to `t1`, with the hole on its left seen from
/// outside (along the outward normal).
#[derive(Clone, Debug)]
pub struct Side {
    pub rail: Arc<dyn Rail>,
    pub t0: f64,
    pub t1: f64,
}

impl Side {
    pub fn new(rail: Arc<dyn Rail>, t0: f64, t1: f64) -> Self {
        Side { rail, t0, t1 }
    }
    fn at(&self, s: f64) -> f64 {
        self.t0 + s * (self.t1 - self.t0)
    }
    fn start(&self) -> Point3 {
        self.rail.point(self.t0).0
    }
    fn end(&self) -> Point3 {
        self.rail.point(self.t1).0
    }
    /// d^2/ds^2 of the point.
    fn second(&self, s: f64) -> Vec3 {
        let dt = self.t1 - self.t0;
        self.rail.jet(self.at(s)).ddp * (dt * dt)
    }
    /// d/ds of the point, s in [0, 1].
    fn tangent(&self, s: f64) -> Vec3 {
        self.rail.jet(self.at(s)).dp * (self.t1 - self.t0)
    }
    fn reversed(&self) -> Side {
        Side {
            rail: self.rail.clone(),
            t0: self.t1,
            t1: self.t0,
        }
    }
    /// Length of the polyline through 9 samples (a lower bound on the arc length).
    fn chord_length(&self) -> f64 {
        let mut l = 0.0;
        let mut q = self.start();
        for k in 1..=8 {
            let p = self.rail.point(self.at(k as f64 / 8.0)).0;
            l += p.distance(q);
            q = p;
        }
        l
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CornerError {
    /// Fewer than three sides of non-zero length.
    TooFewSides,
    TooManySides,
    /// Side i does not end where side i + 1 starts.
    Gap(usize),
    /// Along side i the patch would not turn into the hole: the sides are not in order around
    /// it, or two neighbouring sides meet tangentially.
    Orientation(usize),
    /// Every way of closing the hole folds.
    Folded,
}

/// Most sides a hole may have.
pub const MAX_SIDES: usize = 12;

/// Most sides a hole closed by rational quads may have.
pub const MAX_RATIONAL_SIDES: usize = 20;

/// How a hole closes.
#[derive(Clone, Debug)]
pub enum Closing {
    /// One surface; the flag is whether its natural normal is the outward one.
    One(Surface, bool),
    /// Patches round a centre point. `spokes` run from hole vertices (vertex k starts side k)
    /// to the centre; piece (surface, first, count) is bounded by sides first .. first + count
    /// (cyclic), the spoke from the vertex after them and the spoke back to vertex `first`.
    /// Each surface's natural normal is the outward one.
    Split {
        centre: Point3,
        spokes: Vec<(usize, Curve, Interval)>,
        pieces: Vec<(Surface, usize, usize)>,
    },
}

/// The surfaces for a hole: the sphere when a single ball fits it, else one `HolePatch`, or
/// when that would fold, `HolePatch`es round a centre point. All meet the sides and each
/// other with G1 by construction.
pub fn close_hole(sides: Vec<Side>) -> Result<Closing, CornerError> {
    if let Some((s, same)) = fit_sphere(&sides) {
        return Ok(Closing::One(Surface::Sphere(s), same));
    }
    let proc = |p: HolePatch| Surface::Procedural(Proc(Arc::new(p)));
    Ok(match patches(sides)? {
        Patches::One(p) => Closing::One(proc(p), true),
        Patches::Split(centre, spokes, pieces) => Closing::Split {
            centre,
            spokes,
            pieces: pieces.into_iter().map(|(p, a, b)| (proc(p), a, b)).collect(),
        },
    })
}

type Spoke = (usize, Curve, Interval);

enum Patches {
    One(HolePatch),
    Split(Point3, Vec<Spoke>, Vec<(HolePatch, usize, usize)>),
}

fn patches(sides: Vec<Side>) -> Result<Patches, CornerError> {
    let p = HolePatch::new(sides.clone())?;
    if !p.folded() {
        return Ok(Patches::One(p));
    }
    split(&sides)
}

/// The audit of every patch closing a hole (worst of each measure), and how many there are;
/// None for the sphere. `split` splits it whether or not one patch would do.
#[doc(hidden)]
pub fn audit_hole(
    sides: Vec<Side>,
    seed: u64,
    split_it: bool,
) -> Result<Option<(Audit, usize)>, CornerError> {
    if fit_sphere(&sides).is_some() {
        return Ok(None);
    }
    let found = if split_it { split(&sides)? } else { patches(sides)? };
    let list = match found {
        Patches::One(p) => vec![p],
        Patches::Split(_, _, pieces) => pieces.into_iter().map(|x| x.0).collect(),
    };
    let mut w = Audit {
        fold: f64::INFINITY,
        ..Audit::default()
    };
    for p in &list {
        let a = audit(p, seed);
        w.boundary = w.boundary.max(a.boundary);
        w.normal = w.normal.max(a.normal);
        w.derivs = w.derivs.max(a.derivs);
        w.project = w.project.max(a.project);
        w.bbox_misses += a.bbox_misses;
        w.fold = w.fold.min(a.fold);
    }
    Ok(Some((w, list.len())))
}

/// A spoke of a split hole: a curve from a hole vertex to the centre, with the outward normal
/// w / |w|. For a parabola (or a line), w = m (p'.p') - p' (p'.m), square to the curve and
/// turning with m, which runs linearly from the vertex's normal to the centre's. For a cubic
/// Bezier (`Cubic`), w = p' x f with f linear across the curve, in the vertex's tangent plane at
/// the start and the centre's at the end. p' = a + b t + c t^2, and w is a polynomial in t.
#[derive(Clone, Debug)]
pub struct SpokeRail {
    curve: Curve,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    m0: Vec3,
    m1: Vec3,
    /// The control points and f at the ends, for a cubic.
    cubic: Option<([Point3; 4], Vec3, Vec3)>,
}

impl SpokeRail {
    fn new(curve: Curve, range: Interval, n0: Vec3, n1: Vec3) -> SpokeRail {
        let d = curve.deriv(range.lo);
        let m1 = (n1 - n0) / range.width();
        SpokeRail {
            a: d.d1 - d.d2 * range.lo,
            b: d.d2,
            c: Vec3::ZERO,
            m0: n0 - m1 * range.lo,
            m1,
            curve,
            cubic: None,
        }
    }
    /// A cubic spoke over t in [0, 1].
    fn cubic(p: [Point3; 4], n0: Vec3, n1: Vec3) -> SpokeRail {
        let (d0, d1, d2) = (p[1] - p[0], p[2] - p[1], p[3] - p[2]);
        SpokeRail {
            curve: Curve::Procedural(crate::curve::ProcCurve(Arc::new(Cubic(p)))),
            a: d0 * 3.0,
            b: (d1 - d0) * 6.0,
            c: (d0 - d1 * 2.0 + d2) * 3.0,
            m0: n0,
            m1: n1 - n0,
            cubic: Some((p, n0.cross(d0 * 3.0), n1.cross(d2 * 3.0))),
        }
    }
    /// w and its first two t derivatives at t, given p' and its derivatives there.
    fn w(&self, t: f64, p1: Vec3, p2: Vec3, p3: Vec3) -> [Vec3; 3] {
        if let Some((_, f0, f1)) = self.cubic {
            let (f, df) = (f0 + (f1 - f0) * t, f1 - f0);
            return [
                p1.cross(f),
                p2.cross(f) + p1.cross(df),
                p3.cross(f) + p2.cross(df) * 2.0,
            ];
        }
        let m = self.m0 + self.m1 * t;
        let (s, s1, s2) = (p1.dot(p1), 2.0 * p1.dot(p2), 2.0 * (p2.dot(p2) + p1.dot(p3)));
        let (k, k1, k2) = (
            p1.dot(m),
            p2.dot(m) + p1.dot(self.m1),
            p3.dot(m) + 2.0 * p2.dot(self.m1),
        );
        [
            m * s - p1 * k,
            self.m1 * s + m * s1 - p2 * k - p1 * k1,
            self.m1 * (2.0 * s1) + m * s2 - p3 * k - p2 * (2.0 * k1) - p1 * k2,
        ]
    }
}

impl Rail for SpokeRail {
    fn spoke(&self) -> Option<&SpokeRail> {
        Some(self)
    }
    fn jet(&self, t: f64) -> RailJet {
        let c = self.curve.deriv(t);
        let (p1, p2, p3) = (c.d1, c.d2, self.c * 2.0);
        let [w, w1, w2] = self.w(t, p1, p2, p3);
        let (n, dn, ddn) = unit_jet(w, w1, w2);
        RailJet {
            p: c.p,
            dp: p1,
            ddp: p2,
            dddp: p3,
            n,
            dn,
            ddn,
        }
    }
    fn bound(&self, t: Interval) -> RailBound {
        let quad = |a: Vec3, b: Vec3, c: Vec3| {
            let t2 = t.sqr();
            [
                t2 * c.x + t * b.x + a.x,
                t2 * c.y + t * b.y + a.y,
                t2 * c.z + t * b.z + a.z,
            ]
        };
        let p1 = quad(self.a, self.b, self.c);
        let w: IVec = match self.cubic {
            Some((_, f0, f1)) => {
                let f = quad(f0, f1 - f0, Vec3::ZERO);
                [
                    p1[1] * f[2] - p1[2] * f[1],
                    p1[2] * f[0] - p1[0] * f[2],
                    p1[0] * f[1] - p1[1] * f[0],
                ]
            }
            None => {
                let m = quad(self.m0, self.m1, Vec3::ZERO);
                let (s, k) = (idot(p1, p1), idot(p1, m));
                std::array::from_fn(|i| m[i] * s - p1[i] * k)
            }
        };
        let len = (w[0].sqr() + w[1].sqr() + w[2].sqr()).sqrt();
        let n = if len.lo > 0.0 {
            w.map(|c| meet(c.div(&len), UNIT))
        } else {
            [UNIT; 3]
        };
        RailBound {
            p: self.curve.bbox(t),
            n: ibox(n),
            dp: ibox(p1),
        }
    }
    fn transformed(&self, f: &Frame) -> Arc<dyn Rail> {
        let v = |x: Vec3| f.vec_to_world(x);
        Arc::new(SpokeRail {
            curve: self.curve.transformed(f),
            a: v(self.a),
            b: v(self.b),
            c: v(self.c),
            m0: v(self.m0),
            m1: v(self.m1),
            cubic: self
                .cubic
                .map(|(p, f0, f1)| (p.map(|x| f.point_to_world(x)), v(f0), v(f1))),
        })
    }
}

/// A cubic Bezier curve, t in [0, 1].
#[derive(Clone, Debug)]
pub struct Cubic(pub [Point3; 4]);

impl Cubic {
    /// The control points of the piece over `r`.
    fn sub(&self, r: Interval) -> [Point3; 4] {
        let split = |p: [Point3; 4], t: f64| {
            let a = [p[0].lerp(p[1], t), p[1].lerp(p[2], t), p[2].lerp(p[3], t)];
            let b = [a[0].lerp(a[1], t), a[1].lerp(a[2], t)];
            let c = b[0].lerp(b[1], t);
            ([p[0], a[0], b[0], c], [c, b[1], a[2], p[3]])
        };
        let left = if r.hi < 1.0 { split(self.0, r.hi).0 } else { self.0 };
        if r.lo > 0.0 {
            split(left, r.lo / r.hi).1
        } else {
            left
        }
    }
}

impl crate::curve::ProceduralCurve for Cubic {
    fn exact_d1(&self) -> bool {
        true
    }
    fn deriv(&self, t: f64) -> CurveDeriv {
        let [p0, p1, p2, p3] = self.0;
        let (d0, d1, d2) = (p1 - p0, p2 - p1, p3 - p2);
        let s = 1.0 - t;
        let (b0, b1) = (
            p0.lerp(p1, t).lerp(p1.lerp(p2, t), t),
            p1.lerp(p2, t).lerp(p2.lerp(p3, t), t),
        );
        CurveDeriv {
            p: b0.lerp(b1, t),
            d1: (d0 * (s * s) + d1 * (2.0 * s * t) + d2 * (t * t)) * 3.0,
            d2: ((d1 - d0) * s + (d2 - d1) * t) * 6.0,
        }
    }
    fn domain(&self) -> Domain1 {
        Domain1 {
            lo: 0.0,
            hi: 1.0,
            period: None,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let mut best = (f64::INFINITY, range.lo);
        for k in 0..=32 {
            let t = (range.lo + range.width() * k as f64 / 32.0).min(range.hi);
            let d = self.deriv(t).p.distance_sq(p);
            if d < best.0 {
                best = (d, t);
            }
        }
        let mut t = best.1;
        for _ in 0..30 {
            let d = self.deriv(t);
            let r = d.p - p;
            let g = r.dot(d.d1);
            let h = d.d1.dot(d.d1) + r.dot(d.d2);
            let h = if h > 0.0 { h } else { d.d1.dot(d.d1) };
            if h.is_nan() || h <= 0.0 {
                break;
            }
            let next = (t - g / h).clamp(range.lo, range.hi);
            if self.deriv(next).p.distance_sq(p) > r.length_sq() {
                break;
            }
            let moved = (next - t).abs();
            t = next;
            if moved <= 1e-16 {
                break;
            }
        }
        t
    }
    fn bbox(&self, range: Interval) -> Box3 {
        let c = self.sub(range);
        let b = Box3::from_points(c);
        let m = [b.x, b.y, b.z]
            .iter()
            .fold(0.0f64, |a, i| a.max(i.lo.abs()).max(i.hi.abs()));
        b.grow(16.0 * f64::EPSILON * m)
    }
    fn transformed(&self, f: &Frame) -> crate::curve::ProcCurve {
        crate::curve::ProcCurve(Arc::new(Cubic(self.0.map(|p| f.point_to_world(p)))))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        let c = self.sub(range);
        let w = range.width().max(f64::MIN_POSITIVE);
        // the derivative's control points, per unit of t
        (0..3)
            .map(|i| (c[i + 1] - c[i]).length() * 3.0 / w)
            .fold(0.0, f64::max)
            * (1.0 + 1e-12)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        let mut acc: f64 = 0.0;
        for k in 0..=8 {
            acc = acc.max(self.deriv(range.lo + range.width() * k as f64 / 8.0).d2.length());
        }
        // a chord of parameter length h strays by about |P''| h^2 / 8
        let n = (range.width() * (acc / (8.0 * chord)).sqrt()).ceil();
        (n as usize).max(1)
    }
}

/// A centre for a hole and its outward normal: along the mean normal from the mean boundary
/// point, at the mean of each boundary sample's height and the height its tangent plane gives
/// there (exact for a paraboloid).
fn centre(sides: &[Side]) -> Option<(Point3, Vec3)> {
    let mut samples = Vec::with_capacity(16 * sides.len());
    for s in sides {
        let w = s.chord_length() / 16.0;
        for k in 0..16 {
            let (p, n) = s.rail.point(s.at((k as f64 + 0.5) / 16.0));
            samples.push((p, n, w));
        }
    }
    let total: f64 = samples.iter().map(|x| x.2).sum();
    let c0 = samples
        .iter()
        .fold(Point3::ORIGIN, |a, &(p, _, w)| a + p.to_vec() * (w / total));
    let nc = samples
        .iter()
        .fold(Vec3::ZERO, |a, &(_, n, w)| a + n * w)
        .normalize()?
        .get();
    let (mut h, mut wsum) = (0.0, 0.0);
    for &(p, n, w) in &samples {
        let cos = n.dot(nc);
        if cos > 0.2 {
            let (z, plane) = ((p - c0).dot(nc), (p - c0).dot(n) / cos);
            h += 0.5 * (z + plane) * w * cos;
            wsum += w * cos;
        }
    }
    (wsum > 0.0).then(|| (c0 + nc * (h / wsum), nc))
}

/// At hole vertex k (where side k starts): the point, the outward normal, and the direction
/// halving the hole's angle; None where the sides' normals differ.
fn vertex(sides: &[Side], k: usize) -> Option<(Point3, Vec3, Vec3)> {
    let m = sides.len();
    let s = &sides[k % m];
    let (j, dt) = (s.rail.jet(s.t0), s.t1 - s.t0);
    let prev = &sides[(k + m - 1) % m];
    let pj = prev.rail.jet(prev.t1);
    let n = j.n.normalize()?.get();
    if pj.n.normalize()?.get().dot(n) < 1.0 - 1e-9 {
        return None;
    }
    let ahead = (j.dp * dt).normalize()?.get();
    let back = (pj.dp * (prev.t0 - prev.t1)).normalize()?.get();
    let left = n.cross(ahead);
    // the hole's angle at the vertex, from `ahead` round to `back` about n
    let mut angle = dmath::atan2(back.dot(left), back.dot(ahead));
    if angle <= 0.0 {
        angle += dmath::TAU;
    }
    let (sn, cs) = dmath::sin_cos(0.5 * angle);
    Some((j.p, n, ahead * cs + left * sn))
}

/// Spokes from these (smooth) vertices to a centre: the centre, the spokes, and their rails.
/// With `like`, a patch over the hole, the centre is its middle. `cubic` spokes leave along the
/// halving directions; the others are parabolas.
#[allow(clippy::type_complexity)]
fn spokes(
    sides: &[Side],
    at: &[usize],
    cubic: bool,
    like: Option<&HolePatch>,
) -> Result<(Point3, Vec<Spoke>, Vec<(Arc<dyn Rail>, Interval)>), CornerError> {
    let m = sides.len();
    let (c, nc) = match like {
        Some(p) => {
            let x = Point2::ORIGIN + p.corners.iter().fold(Vec2::ZERO, |a, x| a + x.to_vec()) / m as f64;
            let d = p.deriv(x);
            (
                d.p,
                d.du.cross(d.dv).normalize().ok_or(CornerError::Folded)?.get(),
            )
        }
        _ => centre(sides).ok_or(CornerError::Folded)?,
    };
    let mut spokes = Vec::with_capacity(at.len());
    let mut rails: Vec<(Arc<dyn Rail>, Interval)> = Vec::with_capacity(at.len());
    let mut leave = Vec::with_capacity(at.len());
    for &k in at {
        let (v, nv, dir) = vertex(sides, k % m).ok_or(CornerError::Orientation(k % m))?;
        let l = v.distance(c);
        let toward = (v - c) - nc * (v - c).dot(nc);
        if cubic {
            // a third of the way along the halving direction, and straight toward the vertex
            // (fitted lengths fold: research/corner-patch)
            let e = toward.normalize().ok_or(CornerError::Folded)?.get();
            leave.push(e);
            let ctrl = [v, v + dir * (l / 3.0), c + e * (l / 3.0), c];
            let curve = Curve::Procedural(crate::curve::ProcCurve(Arc::new(Cubic(ctrl))));
            rails.push((Arc::new(SpokeRail::cubic(ctrl, nv, nc)), Interval::new(0.0, 1.0)));
            spokes.push((k % m, curve, Interval::new(0.0, 1.0)));
            continue;
        }
        // the control point in both tangent planes, nearest halfway along the halving direction
        // and leaving the centre straight toward the vertex
        let target = (v + dir * (l / 3.0)).lerp(c + toward * (1.0 / 3.0), 0.5);
        let (g, h) = (nv.dot(nc), 1.0 - nv.dot(nc) * nv.dot(nc));
        let (ra, rb) = ((target - v).dot(nv), (target - c).dot(nc));
        let q = if h > 1e-12 {
            // target - a nv - b nc, with [1 g; g 1] [a b] = [ra rb]
            let (a, b) = ((ra - g * rb) / h, (rb - g * ra) / h);
            target - nv * a - nc * b
        } else {
            target - nv * ra
        };
        let (curve, range) = bezier_parabola(v, q, c).ok_or(CornerError::Orientation(k % m))?;
        rails.push((Arc::new(SpokeRail::new(curve.clone(), range, nv, nc)), range));
        spokes.push((k % m, curve, range));
    }
    // cubic spokes leaving the centre within a quarter of an even share of the turn of each
    // other pinch the quads between them there (curvature 6x to 20x today's patch's, and false
    // projection minima: research/corner-patch)
    let share = dmath::TAU / leave.len().max(1) as f64;
    for i in 0..leave.len() {
        let (a, b) = (leave[i], leave[(i + 1) % leave.len()]);
        if dmath::atan2(a.cross(b).length(), a.dot(b)) < 0.25 * share {
            return Err(CornerError::Folded);
        }
    }
    Ok((c, spokes, rails))
}

/// The sides of piece i of a hole split at vertices `at`: sides at[i] .. at[i + 1], the spoke out
/// to the centre and the spoke back.
fn piece_sides(
    sides: &[Side],
    at: &[usize],
    rails: &[(Arc<dyn Rail>, Interval)],
    i: usize,
) -> (Vec<Side>, usize, usize) {
    let m = sides.len();
    let (first, next) = (at[i], if i + 1 < at.len() { at[i + 1] } else { at[0] + m });
    let mut ps: Vec<Side> = (first..next).map(|k| sides[k % m].clone()).collect();
    let (out, back) = (&rails[(i + 1) % at.len()], &rails[i]);
    ps.push(Side::new(out.0.clone(), out.1.lo, out.1.hi));
    ps.push(Side::new(back.0.clone(), back.1.hi, back.1.lo));
    (ps, first % m, next - first)
}

/// The hole split into patches round a centre point. Spokes leave smooth vertices only, splitting the angle there; pieces take two sides each
/// (one each for a hole of under six).
fn split(sides: &[Side]) -> Result<Patches, CornerError> {
    let m = sides.len();
    let step = if m >= 6 { 2 } else { 1 };
    // spoke vertices every `step` sides from some smooth start; a non-smooth one is skipped to
    // the next smooth vertex
    let smooth: Vec<bool> = (0..m).map(|k| vertex(sides, k).is_some()).collect();
    let start = (0..m).find(|&k| smooth[k]).ok_or(CornerError::Orientation(0))?;
    let mut at = vec![start];
    loop {
        let mut k = at[at.len() - 1] + step;
        while k < start + m && !smooth[k % m] {
            k += 1;
        }
        if k >= start + m {
            break;
        }
        at.push(k);
    }
    if at.len() < 3 {
        return Err(CornerError::Orientation(start));
    }
    let (c, spokes, rails) = spokes(sides, &at, false, None)?;
    let mut pieces = Vec::with_capacity(at.len());
    for i in 0..at.len() {
        let (ps, first, count) = piece_sides(sides, &at, &rails, i);
        // a split is the last resort: look four times as closely
        let p = HolePatch::new(ps)?;
        if p.folded() || p.scan(64, &mut |_| {}) {
            return Err(CornerError::Folded);
        }
        pieces.push((p, first, count));
    }
    Ok(Patches::Split(c, spokes, pieces))
}

/// A way to split a hole into quads round a centre: its sides (some halved), the hole side
/// each is part of, and the first of the points, every other one, that spokes leave.
type Plan = (Vec<Side>, Vec<usize>, usize);

/// The ways to split a hole into quads round a centre, best first. Spokes leave every other
/// vertex from some start, never one where the sides' normals differ; a side between two
/// vertices without spokes is halved and a spoke leaves its middle (one side of an odd hole,
/// or both sides round a kinked vertex). Last, every side halved. At least three spokes; fewer
/// halved sides first, then longer ones (measured: research/corner-patch/sharp.rs).
fn plans(sides: &[Side]) -> Vec<Plan> {
    let n = sides.len();
    let smooth: Vec<bool> = (0..n).map(|k| vertex(sides, k).is_some()).collect();
    let len: Vec<f64> = sides.iter().map(Side::chord_length).collect();
    let mut ways: Vec<Vec<bool>> = (0..n)
        .map(|s| {
            (0..n)
                .map(|k| {
                    let j = (k + n - s) % n;
                    j.is_multiple_of(2) && j + 1 < n && smooth[k]
                })
                .collect()
        })
        .collect();
    ways.push(vec![false; n]);
    let mut out: Vec<(usize, f64, Plan)> = Vec::new();
    for (w, spoke) in ways.iter().enumerate() {
        if ways[..w].contains(spoke) {
            continue;
        }
        let halved = |i: usize| !spoke[i] && !spoke[(i + 1) % n];
        let count = (0..n).filter(|&i| spoke[i] || halved(i)).count();
        if count < 3 {
            continue;
        }
        let (mut ex, mut from) = (Vec::new(), Vec::new());
        for (i, s) in sides.iter().enumerate() {
            if halved(i) {
                let tm = 0.5 * (s.t0 + s.t1);
                ex.extend([
                    Side::new(s.rail.clone(), s.t0, tm),
                    Side::new(s.rail.clone(), tm, s.t1),
                ]);
                from.extend([i, i]);
            } else {
                ex.push(s.clone());
                from.push(i);
            }
        }
        let cut: Vec<usize> = (0..n).filter(|&i| halved(i)).collect();
        let first = if spoke[0] { 0 } else { 1 };
        out.push((
            cut.len(),
            -cut.iter().map(|&i| len[i]).sum::<f64>(),
            (ex, from, first),
        ));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    out.into_iter().map(|x| x.2).collect()
}

/// The rational quads closing a hole by plan k of `plans` (for four sides, first one quad),
/// unproven, each with its four sides, and every hole side's pieces. None when there is no
/// such plan, a side is not a line or conic on a plane, cylinder or sphere, or the hole is the
/// wrong way round.
#[allow(clippy::type_complexity)]
fn rational_cells(sides: &[Side], k: usize) -> Option<(Vec<(rational::Quad, Vec<Side>)>, Vec<Vec<Piece>>)> {
    let n = sides.len();
    if !(3..=MAX_RATIONAL_SIDES).contains(&n) {
        return None;
    }
    if n == 4 && k == 0 {
        let q = rational::Quad::build(sides)?;
        let pieces = whole(sides);
        return Some((vec![(q, sides.to_vec())], pieces));
    }
    let (ex, from, first) = plans(sides).into_iter().nth(if n == 4 { k - 1 } else { k })?;
    let m = ex.len();
    let at: Vec<usize> = (0..m / 2).map(|i| 2 * i + first).collect();
    let like = (n <= MAX_SIDES).then(|| HolePatch::with_domain(sides.to_vec(), 1.0));
    let (_, _, rails) = spokes(&ex, &at, true, like.as_ref()).ok()?;
    let mut quads = Vec::with_capacity(at.len());
    let mut place = vec![(0, 0); m];
    for i in 0..at.len() {
        let (ps, f, _) = piece_sides(&ex, &at, &rails, i);
        quads.push((rational::Quad::build(&ps)?, ps));
        (place[f % m], place[(f + 1) % m]) = ((i, 0), (i, 1));
    }
    let mut pieces = vec![Vec::new(); n];
    for k in 0..m {
        pieces[from[k]].push((place[k].0, place[k].1, ex[k].t0, ex[k].t1));
    }
    Some((quads, pieces))
}

/// The hole closed by one patch of rational quads (see `rational_cells`), proven never to
/// fold, by the first plan that proves of the first two; None when neither does.
fn rational_patch(sides: &[Side]) -> Option<(HolePatch, RationalProof)> {
    let mut spent = RationalProof::default();
    for k in 0..2 {
        let Some((quads, pieces)) = rational_cells(sides, k) else {
            continue;
        };
        let p = HolePatch::from_quads(sides.to_vec(), quads.into_iter().map(|q| q.0).collect(), pieces);
        let mut proof = p.prove_rational()?;
        proof.regions += spent.regions;
        proof.tests += spent.tests;
        if proof.proved {
            return Some((p, proof));
        }
        spent = proof;
    }
    None
}

/// The rational closing of a hole (see `rational_patch`), and its proof.
pub fn close_hole_rational(sides: Vec<Side>) -> Option<(Closing, RationalProof)> {
    if let Some((s, same)) = fit_sphere(&sides) {
        return Some((Closing::One(Surface::Sphere(s), same), RationalProof::default()));
    }
    let (p, proof) = rational_patch(&sides)?;
    Some((Closing::One(Surface::Procedural(Proc(Arc::new(p))), true), proof))
}

/// The rational quads of a hole by plan k, each a patch alone, proven or not, for
/// measurements.
#[doc(hidden)]
pub fn rational_quads(sides: &[Side], k: usize) -> Option<Vec<HolePatch>> {
    let (quads, _) = rational_cells(sides, k)?;
    Some(
        quads
            .into_iter()
            .map(|(q, s)| {
                let pieces = whole(&s);
                HolePatch::from_quads(s, vec![q], pieces)
            })
            .collect(),
    )
}

/// The rational patch of a hole and its proof, for measurements.
#[doc(hidden)]
pub fn rational_pieces(sides: &[Side]) -> Option<(Vec<HolePatch>, RationalProof)> {
    let (p, proof) = rational_patch(sides)?;
    Some((vec![p], proof))
}

/// The sphere every side lies on with the sphere's normals, if there is one: the rolling
/// ball itself where equal blends meet. The flag is false for a concave ball (outward normal
/// toward the centre).
pub fn fit_sphere(sides: &[Side]) -> Option<(Sphere, bool)> {
    let mut samples = Vec::with_capacity(sides.len() * 5);
    for s in sides {
        for k in 0..5 {
            samples.push(s.rail.point(s.at(k as f64 / 4.0)));
        }
    }
    let (mut best, mut pair) = (0.0, (0, 0));
    for i in 0..samples.len() {
        for j in i + 1..samples.len() {
            let d = (samples[i].1 - samples[j].1).length_sq();
            if d > best {
                best = d;
                pair = (i, j);
            }
        }
    }
    let ((pa, na), (pb, nb)) = (samples[pair.0], samples[pair.1]);
    if best <= tol::ANGULAR * tol::ANGULAR {
        return None;
    }
    // p = c + rho n at every sample
    let dn = na - nb;
    let rho = (pa - pb).dot(dn) / dn.length_sq();
    if rho.abs() < tol::MIN_LENGTH {
        return None;
    }
    let c = pa - na * rho;
    if samples
        .iter()
        .any(|&(p, n)| (p - n * rho).distance(c) > tol::LINEAR)
    {
        return None;
    }
    let mut m = Vec3::ZERO;
    for &(_, n) in &samples {
        m += n;
    }
    let m = (m * rho.signum()).normalize()?;
    // the hole faces the sphere's longitude pi on its equator, away from the seam and the poles
    let frame = Frame::from_zx(c, m.any_perpendicular(), -m.get())?;
    Some((
        Sphere {
            frame,
            radius: rho.abs(),
        },
        rho > 0.0,
    ))
}

/// The n-sided side-based transfinite patch. The domain is a regular n-gon; at a point of it,
/// side i has a distance d_i and a position s_i = d_(i-1) / (d_(i-1) + d_(i+1)) along the
/// side. Its ribbon is R_i = C_i(s_i) + d_i T_i(s_i), where C_i is the exact boundary curve
/// and T_i lies in the neighbour's tangent plane, turning from the previous side's direction
/// at s = 0 to the next side's at s = 1. The patch is the blend sum_i R_i B_i with
/// B_i = prod_(j != i) d_j^2 / sum_k prod_(j != k) d_j^2. On side i every other B_j
/// vanishes to second order, so the patch passes through C_i with the tangent plane spanned
/// by C_i' and T_i: exactly the neighbour's.
#[derive(Debug)]
pub struct HolePatch {
    sides: Vec<Side>,
    /// Domain polygon, counter-clockwise; side i runs from corner i to corner i + 1 (for
    /// rational quads, the cells' outline).
    corners: Vec<Point2>,
    /// d_i(x) = g . x + h.
    lines: Vec<(Vec2, f64)>,
    /// The cross-tangent of each side is a e1 + b e2 in the frame e1 = C' / |C'|, e2 = n x e1,
    /// with (a, b) linear in s from [a0, b0] at the start to [a1, b1] at the end. It turns
    /// with the curve and stays in the neighbour's tangent plane.
    ends: Vec<[f64; 4]>,
    /// The ribbons bend by h^2 / 2 times a vector linear in s between these, matching the
    /// neighbouring curves' second derivatives at the corners (so the patch's curvature does
    /// not spike there).
    bends: Vec<(Vec3, Vec3)>,
    rect: Box2,
    /// Length of a side of the domain polygon.
    side: f64,
    /// The largest distance from each side within the polygon.
    heights: Vec<f64>,
    seeds: Vec<(Point2, Point3)>,
    scale: (f64, f64),
    /// The sampled tightest radius of curvature: it sizes the domain and the mesh.
    radius: f64,
    fold: bool,
    /// The rational quads, when they replace the ribbons.
    rational: Option<Arc<Cells>>,
}

/// A piece of a hole side: the cell and quad side it is, and its rail range.
type Piece = (usize, usize, f64, f64);

/// The pieces of a single quad's four sides: each whole.
fn whole(sides: &[Side]) -> Vec<Vec<Piece>> {
    (0..4).map(|i| vec![(0, i, sides[i].t0, sides[i].t1)]).collect()
}

/// Rational quads joined into one patch along internal seams, each over a parallelogram of the
/// domain, x = o + u a + v b for (u, v) in the unit square. Neighbouring cells map their shared
/// spoke alike, so the parametrisation is C0 across a seam (the surface G1).
#[derive(Debug)]
struct Cells {
    cells: Vec<(rational::Quad, Point2, Vec2, Vec2)>,
    /// Each hole side's pieces, in order.
    pieces: Vec<Vec<Piece>>,
}

impl Cells {
    /// Cell c's (u, v) at x and their gradients.
    fn local(&self, c: usize, x: Point2) -> (f64, f64, Vec2, Vec2) {
        let (_, o, a, b) = &self.cells[c];
        let det = a.cross(*b);
        let (gu, gv) = (Vec2::new(b.y, -b.x) / det, Vec2::new(-a.y, a.x) / det);
        let r = x - *o;
        (gu.dot(r), gv.dot(r), gu, gv)
    }
    /// The cell x is deepest in, and how deep (negative outside), in cell coordinates.
    fn cell_of(&self, x: Point2) -> (usize, f64) {
        let mut best = (0, f64::NEG_INFINITY);
        for c in 0..self.cells.len() {
            let (u, v, _, _) = self.local(c, x);
            let m = u.min(v).min(1.0 - u).min(1.0 - v);
            if m > best.1 {
                best = (c, m);
            }
        }
        best
    }
    fn eval<S: Scalar>(&self, x: Point2) -> S::V {
        let c = self.cell_of(x).0;
        let (u, v, gu, gv) = self.local(c, x);
        self.cells[c].0.eval::<S>(u, v, gu, gv)
    }
    /// The domain point nearest p's, from x in cell c: Newton in that cell, then into the
    /// neighbouring cell while the nearest point is on their seam and the distance falls across it.
    fn project(&self, p: Point3, c: usize, x: Point2) -> Point2 {
        const SQUARE: [(Vec2, f64); 4] = [
            (Vec2::Y, 0.0),
            (Vec2::new(-1.0, 0.0), 1.0),
            (Vec2::new(0.0, -1.0), 1.0),
            (Vec2::X, 0.0),
        ];
        let (m, mut c) = (self.cells.len(), c);
        let (u, v, _, _) = self.local(c, x);
        let mut y = Point2::new(u.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
        let deriv = |c: usize, y: Point2| surf_deriv(self.cells[c].0.eval::<J>(y.x, y.y, Vec2::X, Vec2::Y));
        for _ in 0..2 * m {
            let q = &self.cells[c].0;
            y = newton(
                p,
                y,
                1.0,
                &SQUARE,
                |z| Point2::new(z.x.clamp(0.0, 1.0), z.y.clamp(0.0, 1.0)),
                |z| deriv(c, z),
                |z| Point3::ORIGIN + q.eval::<f64>(z.x, z.y, Vec2::X, Vec2::Y),
            );
            if m < 3 {
                break;
            }
            let d = deriv(c, y);
            let r = d.p - p;
            // across v = 1 into cell c + 1, or u = 0 into c - 1
            if y.y >= 1.0 - 1e-12 && r.dot(d.dv) < 0.0 {
                (c, y) = ((c + 1) % m, Point2::new(0.0, 1.0 - y.x));
            } else if y.x <= 1e-12 && r.dot(d.du) > 0.0 {
                (c, y) = ((c + m - 1) % m, Point2::new(1.0 - y.y, 1.0));
            } else {
                break;
            }
        }
        let (_, o, a, b) = &self.cells[c];
        *o + *a * y.x + *b * y.y
    }
    /// The domain point of hole side i at rail parameter t.
    fn side_point(&self, i: usize, t: f64) -> Point2 {
        let off = |p: &&Piece| (p.2.min(p.3) - t).max(t - p.2.max(p.3)).max(0.0);
        let &(c, q, _, _) = self.pieces[i]
            .iter()
            .min_by(|a, b| off(a).total_cmp(&off(b)))
            .expect("a side has pieces");
        let s = self.cells[c].0.side_s(q, t);
        let (u, v) = [(s, 0.0), (1.0, s), (1.0 - s, 1.0), (0.0, 1.0 - s)][q];
        let (_, o, a, b) = &self.cells[c];
        *o + *a * u + *b * v
    }
}

impl HolePatch {
    /// The domain polygon's corners, counter-clockwise.
    pub fn domain_corners(&self) -> &[Point2] {
        &self.corners
    }

    /// Points and weights of a rule over the domain polygon, m Gauss-Legendre points each way
    /// on each piece of it. The rational quads' cells are smooth inside; the ribbons' blend turns
    /// with direction at each corner (smooth in angle and distance from it), so each side's
    /// triangle to the centre is halved at the side's middle and each half collapsed onto its
    /// corner (Duffy).
    pub fn domain_rule(&self, m: usize) -> Vec<(Point2, f64)> {
        let g: Vec<(f64, f64)> = forge_math::poly::gauss_legendre(m)
            .into_iter()
            .map(|(x, w)| (0.5 * (x + 1.0), 0.5 * w))
            .collect();
        let mut out = Vec::new();
        match &self.rational {
            Some(cells) => {
                for (_, o, a, b) in &cells.cells {
                    let j = a.cross(*b).abs();
                    for &(u, wu) in &g {
                        for &(v, wv) in &g {
                            out.push((*o + *a * u + *b * v, wu * wv * j));
                        }
                    }
                }
            }
            None => {
                let n = self.corners.len();
                let c = self
                    .corners
                    .iter()
                    .fold(Point2::ORIGIN, |a, p| a + (*p - Point2::ORIGIN) / n as f64);
                for i in 0..n {
                    let (p, q) = (self.corners[i], self.corners[(i + 1) % n]);
                    let mid = p.lerp(q, 0.5);
                    for (apex, e0, e1) in [(p, mid, c), (q, c, mid)] {
                        let (a, b) = (e0 - apex, e1 - apex);
                        let det = a.cross(b).abs();
                        for &(s, ws) in &g {
                            for &(t, wt) in &g {
                                out.push((apex + (a * (1.0 - t) + b * t) * s, ws * wt * s * det));
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// Sides in order around the hole. Zero-length sides (a zero setback) are dropped. Either
    /// orientation is accepted; the patch's natural normal is the outward one.
    pub fn new(sides: Vec<Side>) -> Result<HolePatch, CornerError> {
        let mut sides: Vec<Side> = sides
            .into_iter()
            .filter(|s| s.chord_length() > tol::LINEAR)
            .collect();
        let n = sides.len();
        if n < 3 {
            return Err(CornerError::TooFewSides);
        }
        if n > MAX_SIDES {
            return Err(CornerError::TooManySides);
        }
        for i in 0..n {
            if sides[i].end().distance(sides[(i + 1) % n].start()) > tol::LINEAR {
                return Err(CornerError::Gap(i));
            }
        }
        let mut p = HolePatch::with_domain(sides.clone(), 1.0);
        let signs: Vec<bool> = (0..n).map(|i| p.turns_in(i)).collect();
        if signs.iter().all(|&s| !s) {
            sides.reverse();
            sides = sides.iter().map(Side::reversed).collect();
            p = HolePatch::with_domain(sides.clone(), 1.0);
        }
        if let Some(i) = (0..n).find(|&i| !p.turns_in(i)) {
            return Err(CornerError::Orientation(i));
        }
        // size the domain so a unit of u or v is about one sampled radius on the surface: the
        // mesher steps the parameters by angles of that radius
        // (scaling the domain leaves the surface as it is)
        p.measure();
        let k = (0.5 * (p.scale.0 + p.scale.1) / p.radius).clamp(1e-3, 1e3);
        let mut q = HolePatch::with_domain(sides, k);
        (q.scale, q.radius, q.fold) = ((p.scale.0 / k, p.scale.1 / k), p.radius, p.fold);
        q.seed();
        Ok(q)
    }

    fn with_domain(sides: Vec<Side>, side_len: f64) -> HolePatch {
        let n = sides.len();
        // a domain side grows with the fourth root of the side's length: in proportion, a short
        // side squeezes the domain round it so much that the patch folds there
        let lengths: Vec<f64> = sides.iter().map(|s| s.chord_length().sqrt().sqrt()).collect();
        let mean = lengths.iter().sum::<f64>() / n as f64;
        let corners = cyclic_polygon(&lengths.iter().map(|l| l / mean * side_len).collect::<Vec<_>>());
        HolePatch::with_corners(sides, corners, side_len)
    }

    fn with_corners(sides: Vec<Side>, corners: Vec<Point2>, side_len: f64) -> HolePatch {
        let n = sides.len();
        let lines: Vec<(Vec2, f64)> = (0..n)
            .map(|i| {
                let (a, b) = (corners[i], corners[(i + 1) % n]);
                let g = (b - a).perp().normalize().expect("polygon sides have length");
                (g, -g.dot(a.to_vec()))
            })
            .collect();
        let dist = |k: usize, p: Point2| {
            let (g, h) = lines[k % n];
            g.dot(p.to_vec()) + h
        };
        // first-order agreement of neighbouring ribbons at each corner: near corner i + 1,
        // s_i = 1 - d_(i+1) / d_(i-1)(corner) and s_(i+1) = d_i / d_(i+2)(corner)
        let ends: Vec<[f64; 4]> = (0..n)
            .map(|i| {
                let prev = &sides[(i + n - 1) % n];
                let next = &sides[(i + 1) % n];
                let a = prev.tangent(1.0) * (-1.0 / dist(i + n - 2, corners[i]));
                let b = next.tangent(0.0) * (1.0 / dist(i + 2, corners[(i + 1) % n]));
                let (e1, e2) = frame_at(&sides[i], &sides[i].rail.jet(sides[i].t0));
                let (f1, f2) = frame_at(&sides[i], &sides[i].rail.jet(sides[i].t1));
                [a.dot(e1), a.dot(e2), b.dot(f1), b.dot(f2)]
            })
            .collect();
        let heights: Vec<f64> = (0..n)
            .map(|i| corners.iter().map(|&c| dist(i, c)).fold(0.0, f64::max))
            .collect();
        let bends = (0..n)
            .map(|i: usize| {
                let side = &sides[i];
                let prev = &sides[(i + n - 1) % n];
                let next = &sides[(i + 1) % n];
                let (a, b) = (dist(i + n - 2, corners[i]), dist(i + 2, corners[(i + 1) % n]));
                let [a0, b0, a1, b1]: [f64; 4] = ends[i];
                let (e1, e2) = frame_at(side, &side.rail.jet(side.t0));
                let (f1, f2) = frame_at(side, &side.rail.jet(side.t1));
                // the damping h = d - d^2 / height + ... bends the ribbon by -2 T / height
                let k = 2.0 / heights[i];
                (
                    prev.second(1.0) * (1.0 / (a * a)) + (e1 * a0 + e2 * b0) * k,
                    next.second(0.0) * (1.0 / (b * b)) + (f1 * a1 + f2 * b1) * k,
                )
            })
            .collect();
        let mut rect = Box2::new(Interval::EMPTY, Interval::EMPTY);
        for c in &corners {
            rect.u.include(c.x);
            rect.v.include(c.y);
        }
        HolePatch {
            sides,
            corners,
            lines,
            ends,
            bends,
            rect,
            side: side_len,
            heights,
            seeds: Vec::new(),
            scale: (1.0, 1.0),
            radius: 1.0,
            fold: false,
            rational: None,
        }
    }

    pub fn sides(&self) -> &[Side] {
        &self.sides
    }

    /// The domain point of side i at s in [0, 1] (where the patch is C_i(s)).
    pub fn side_uv(&self, i: usize, s: f64) -> Point2 {
        if self.rational.is_some() {
            return self.side_param_uv(i, self.sides[i].at(s));
        }
        let n = self.sides.len();
        let (a, b) = (self.corners[i], self.corners[(i + 1) % n]);
        let dist = |k: usize, p: Point2| {
            let (g, h) = self.lines[k];
            g.dot(p.to_vec()) + h
        };
        let (pk, nk) = ((i + n - 1) % n, (i + 1) % n);
        let (a0, a1) = (dist(pk, a), dist(pk, b) - dist(pk, a));
        let (b0, b1) = (dist(nk, a), dist(nk, b) - dist(nk, a));
        // s (a0 + a1 x + b0 + b1 x) = a0 + a1 x
        let sigma = (a0 - s * (a0 + b0)) / (s * (a1 + b1) - a1);
        if s <= 0.0 {
            a
        } else if s >= 1.0 {
            b
        } else {
            a.lerp(b, sigma.clamp(0.0, 1.0))
        }
    }

    /// The domain point of side i at rail parameter t.
    pub fn side_param_uv(&self, i: usize, t: f64) -> Point2 {
        if let Some(r) = &self.rational {
            return r.side_point(i, t);
        }
        let s = &self.sides[i];
        self.side_uv(i, (t - s.t0) / (s.t1 - s.t0))
    }

    /// A rational quad over four sides in order round it (lines and conics on planes,
    /// cylinders and spheres, and spokes), on a square domain; None when a side is of another
    /// kind or does not turn into the hole.
    #[cfg(test)]
    fn quad(sides: Vec<Side>) -> Option<HolePatch> {
        let q = rational::Quad::build(&sides)?;
        let pieces = whole(&sides);
        Some(HolePatch::from_quads(sides, vec![q], pieces))
    }

    /// One patch of rational quads (see `Cells`) over these hole sides. One quad is a square of
    /// the domain; m >= 3 are parallelograms round the origin, cell i with corners A_i,
    /// A_i + A_(i+1), A_(i+1) and the origin (A_i of length k at the angle 2 pi i / m): quad
    /// sides 0 and 1 on the domain's outline, 2 and 3 the spokes. The domain is sized so a unit
    /// is about the sampled radius, as `new` does.
    fn from_quads(sides: Vec<Side>, quads: Vec<rational::Quad>, pieces: Vec<Vec<Piece>>) -> HolePatch {
        let m = quads.len();
        let (mut scale, mut kmax) = (0.0, 0.0f64);
        for q in &quads {
            let (a, b) = q.net();
            scale += 0.5 * (a + b) / m as f64;
            for i in 1..6 {
                for j in 1..6 {
                    let x = (i as f64 / 6.0, j as f64 / 6.0);
                    kmax = kmax.max(max_curvature(&surf_deriv(q.eval::<J>(
                        x.0,
                        x.1,
                        Vec2::X,
                        Vec2::Y,
                    ))));
                }
            }
        }
        let rmin = sampled_radius(kmax, &sides);
        let k = (scale / rmin).clamp(1e-3, 1e3);
        let (cells, outline): (Vec<_>, Vec<Point2>) = if m == 1 {
            let (a, b) = (Vec2::new(k, 0.0), Vec2::new(0.0, k));
            let o = Point2::ORIGIN;
            (
                quads.into_iter().map(|q| (q, o, a, b)).collect(),
                vec![o, o + a, o + a + b, o + b],
            )
        } else {
            let at = |i: usize| {
                let (s, c) = dmath::sin_cos(dmath::TAU * (i % m) as f64 / m as f64);
                Vec2::new(c * k, s * k)
            };
            (
                quads
                    .into_iter()
                    .enumerate()
                    .map(|(i, q)| (q, Point2::ORIGIN + at(i), at(i + 1), -at(i)))
                    .collect(),
                (0..2 * m)
                    .map(|j| {
                        Point2::ORIGIN
                            + if j % 2 == 0 {
                                at(j / 2)
                            } else {
                                at(j / 2) + at(j / 2 + 1)
                            }
                    })
                    .collect(),
            )
        };
        let w = outline.len();
        let lines = (0..w)
            .map(|i| {
                let (a, b) = (outline[i], outline[(i + 1) % w]);
                let g = (b - a).perp().normalize().expect("domain sides have length");
                (g, -g.dot(a.to_vec()))
            })
            .collect();
        let mut rect = Box2::new(Interval::EMPTY, Interval::EMPTY);
        for c in &outline {
            rect.u.include(c.x);
            rect.v.include(c.y);
        }
        let mut p = HolePatch {
            sides,
            corners: outline,
            lines,
            ends: Vec::new(),
            bends: Vec::new(),
            rect,
            side: k,
            heights: Vec::new(),
            seeds: Vec::new(),
            scale: (scale / k, scale / k),
            radius: rmin,
            fold: false,
            rational: Some(Arc::new(Cells { cells, pieces })),
        };
        p.seed();
        p
    }

    /// The proof that the rational form never folds (None for the ribbon form).
    pub fn prove_rational(&self) -> Option<RationalProof> {
        let mut out = RationalProof {
            proved: true,
            ..RationalProof::default()
        };
        for (q, ..) in &self.rational.as_ref()?.cells {
            let p = q.prove();
            out.regions += p.regions;
            out.tests += p.tests;
            out.depth = out.depth.max(p.depth);
            out.cached |= p.cached;
            if !p.proved {
                out.proved = false;
                break;
            }
        }
        Some(out)
    }

    fn inside(&self, x: Point2) -> bool {
        if let Some(r) = &self.rational {
            // seams within rounding of both cells
            return r.cell_of(x).1 >= -1e-12;
        }
        self.lines.iter().all(|&(g, h)| g.dot(x.to_vec()) + h >= 0.0)
    }

    /// The nearest point of the domain polygon.
    fn clamp(&self, x: Point2) -> Point2 {
        if self.inside(x) {
            return x;
        }
        let n = self.corners.len();
        let mut best = (f64::INFINITY, x);
        for i in 0..n {
            let (a, b) = (self.corners[i], self.corners[(i + 1) % n]);
            let e = b - a;
            let t = ((x - a).dot(e) / e.length_sq()).clamp(0.0, 1.0);
            let q = a.lerp(b, t);
            let d = q.distance(x);
            if d < best.0 {
                best = (d, q);
            }
        }
        best.1
    }

    /// The cross-tangent of side i and its first two s derivatives, at s with the rail jet
    /// there.
    fn cross(&self, i: usize, j: &RailJet, s: f64) -> [Vec3; 3] {
        let dt = self.sides[i].t1 - self.sides[i].t0;
        let (c1, c2, c3) = (j.dp * dt, j.ddp * (dt * dt), j.dddp * (dt * dt * dt));
        let (n, n1, n2) = (j.n, j.dn * dt, j.ddn * (dt * dt));
        // e1 = c1 / l: e1 l = c1, so e1' l + e1 l' = c2 and e1'' l + 2 e1' l' + e1 l'' = c3
        let l = c1.length();
        let e1 = c1 / l;
        let l1 = e1.dot(c2);
        let e1d = (c2 - e1 * l1) / l;
        let l2 = e1d.dot(c2) + e1.dot(c3);
        let e1dd = (c3 - e1d * (2.0 * l1) - e1 * l2) / l;
        let e2 = n.cross(e1);
        let e2d = n1.cross(e1) + n.cross(e1d);
        let e2dd = n2.cross(e1) + n1.cross(e1d) * 2.0 + n.cross(e1dd);
        let [a0, b0, a1, b1] = self.ends[i];
        let (a, b) = (a0 + (a1 - a0) * s, b0 + (b1 - b0) * s);
        let (da, db) = (a1 - a0, b1 - b0);
        [
            e1 * a + e2 * b,
            e1 * da + e1d * a + e2 * db + e2d * b,
            e1d * (2.0 * da) + e1dd * a + e2d * (2.0 * db) + e2dd * b,
        ]
    }

    fn ribbon<S: Scalar>(&self, i: usize, s: S, d: S) -> S::V {
        let side = &self.sides[i];
        let dt = side.t1 - side.t0;
        let sv = s.value();
        if !S::JET {
            let (p, dp, n) = side.rail.frame(side.t0 + sv * dt);
            let e1 = dp * (dt / (dp.length() * dt.abs()));
            let [a0, b0, a1, b1] = self.ends[i];
            let tv = e1 * (a0 + (a1 - a0) * sv) + n.cross(e1) * (b0 + (b1 - b0) * sv);
            let (k0, k1) = self.bends[i];
            let k = (k0 + (k1 - k0) * sv) * 0.5;
            return S::along(s, p.to_vec(), Vec3::ZERO, Vec3::ZERO)
                + S::along(s, tv, Vec3::ZERO, Vec3::ZERO) * d
                + S::along(s, k, Vec3::ZERO, Vec3::ZERO) * (d * d);
        }
        let j = side.rail.jet(side.t0 + sv * dt);
        let [t0, t1, t2] = self.cross(i, &j, sv);
        let (c1, c2) = (j.dp * dt, j.ddp * (dt * dt));
        let (k0, k1) = self.bends[i];
        let k = S::along(s, (k0 + (k1 - k0) * sv) * 0.5, (k1 - k0) * 0.5, Vec3::ZERO);
        S::along(s, j.p.to_vec(), c1, c2) + S::along(s, t0, t1, t2) * d + k * (d * d)
    }

    fn eval<S: Scalar>(&self, x: Point2) -> S::V {
        let n = self.sides.len();
        let x = self.clamp(x);
        if let Some(r) = &self.rational {
            return r.eval::<S>(x);
        }
        let mut d = [S::konst(0.0); MAX_SIDES];
        let (mut k, mut second) = (0, f64::INFINITY);
        for i in 0..n {
            let (g, h) = self.lines[i];
            let v = (g.x * x.x + g.y * x.y + h).max(0.0);
            d[i] = S::affine(v, g.x, g.y);
            if i > 0 && v < d[k].value() {
                second = d[k].value();
                k = i;
            } else if i > 0 {
                second = second.min(v);
            }
        }
        let mut r = [S::zero_v(); MAX_SIDES];
        for i in 0..n {
            let (a, b) = (d[(i + n - 1) % n], d[(i + 1) % n]);
            let den = a + b;
            // only at the far corner of a triangle, where this ribbon's weight is zero
            let s = if den.value() > 0.0 {
                a * den.inv()
            } else {
                S::konst(0.5)
            };
            // the ribbon reaches across the domain damped: h(d) = d / (1 + d / height) leaves
            // value and slope at the side alone and halves the reach at the far corner
            let h = d[i] * (S::konst(1.0) + d[i] * S::konst(1.0 / self.heights[i])).inv();
            r[i] = self.ribbon(i, s, h);
        }
        // within 1e-8 of a corner, where the patch is only C1, the ribbons agree to first order
        // and the nearest one is the patch to rounding (the rest is that distance squared)
        if second <= 1e-8 * self.side {
            return r[k];
        }
        // B_i = d_i^-2 / sum_j d_j^-2, scaled by the nearest side's d_k^2 so every term is at
        // most 1: no underflow, and the ribbons are blended relative to the nearest one so the
        // large weight gradients near a corner multiply small differences
        let sk = d[k] * d[k];
        let mut w = [S::konst(0.0); MAX_SIDES];
        let mut sum = S::konst(1.0);
        for i in 0..n {
            if i != k {
                if d[i].value() == 0.0 {
                    // a corner: the nearest ribbon is the corner point
                    return r[k];
                }
                w[i] = sk * (d[i] * d[i]).inv();
                sum = sum + w[i];
            }
        }
        let inv = sum.inv();
        let mut out = r[k];
        for i in 0..n {
            if i != k {
                out = out + (r[i] - r[k]) * (w[i] * inv);
            }
        }
        out
    }

    pub fn point(&self, uv: Point2) -> Point3 {
        let v = self.eval::<f64>(uv);
        Point3::new(v.x, v.y, v.z)
    }

    /// Whether the cross-tangent points into the hole all along side i: b > 0 at both ends.
    fn turns_in(&self, i: usize) -> bool {
        let [_, b0, _, b1] = self.ends[i];
        b0 > 0.0 && b1 > 0.0
    }

    /// Parameter scales and the tightest curvature, from a grid over the domain.
    fn measure(&mut self) {
        let n = self.sides.len();
        let (mut su, mut sv, mut count) = (0.0, 0.0, 0usize);
        let mut kmax: f64 = 0.0;
        let mut take = |d: &SurfDeriv| {
            su += d.du.length();
            sv += d.dv.length();
            count += 1;
            kmax = kmax.max(max_curvature(d));
        };
        let fold = self.scan(16, &mut take);
        for i in 0..n {
            for k in 0..9 {
                take(&self.deriv(self.side_uv(i, k as f64 / 9.0)));
            }
        }
        self.scale = (su / count as f64, sv / count as f64);
        self.radius = sampled_radius(kmax, &self.sides);
        self.fold = fold;
    }

    /// Whether the patch folds over or creases: on an m x m grid over the domain, the normals
    /// of neighbouring points turn by more than 60 degrees, away from corners where the sides'
    /// normals differ (a cut edge's end), round which the normal turns by design. Each
    /// evaluation inside the domain is handed to `each`.
    fn scan(&self, m: usize, each: &mut impl FnMut(&SurfDeriv)) -> bool {
        let n = self.sides.len();
        let cell = self.rect.u.width().max(self.rect.v.width()) / m as f64;
        let kinks: Vec<Point2> = (0..n)
            .filter(|&i| {
                let a = self.sides[(i + n - 1) % n]
                    .rail
                    .point(self.sides[(i + n - 1) % n].t1)
                    .1;
                let b = self.sides[i].rail.point(self.sides[i].t0).1;
                a.dot(b) < (1.0 - 1e-9) * a.length() * b.length()
            })
            .map(|i| self.side_param_uv(i, self.sides[i].t0))
            .collect();
        let mut fold = false;
        let mut prev: Vec<Option<Vec3>> = vec![None; m + 1];
        for i in 0..=m {
            let mut left = None;
            for (j, above) in prev.iter_mut().enumerate() {
                let x = Point2::new(
                    self.rect.u.lo + self.rect.u.width() * i as f64 / m as f64,
                    self.rect.v.lo + self.rect.v.width() * j as f64 / m as f64,
                );
                let mut nrm = None;
                if self.inside(x) {
                    let d = self.deriv(x);
                    each(&d);
                    if kinks.iter().all(|k| k.distance(x) > 2.0 * cell) {
                        nrm = d.du.cross(d.dv).normalize().map(|u| u.get());
                    }
                }
                if let Some(a) = nrm {
                    fold |= [left, *above].into_iter().flatten().any(|b: Vec3| a.dot(b) < 0.5);
                }
                *above = nrm;
                left = nrm;
            }
        }
        fold
    }

    /// Points of the domain on an m x m grid over its rectangle, plus side samples.
    fn grid(&self, m: usize) -> Vec<Point2> {
        let mut out = Vec::new();
        for i in 0..=m {
            for j in 0..=m {
                let x = Point2::new(
                    self.rect.u.lo + self.rect.u.width() * i as f64 / m as f64,
                    self.rect.v.lo + self.rect.v.width() * j as f64 / m as f64,
                );
                if self.inside(x) {
                    out.push(x);
                }
            }
        }
        for i in 0..self.sides.len() {
            for k in 0..m {
                out.push(self.side_uv(i, k as f64 / m as f64));
            }
        }
        out
    }

    /// Whether the patch folds over or creases (from `measure`).
    fn folded(&self) -> bool {
        self.fold
    }

    fn seed(&mut self) {
        let xs = match &self.rational {
            // each cell its own grid (the outline's rectangle leaves cells of a star thin)
            Some(r) => {
                let g = 12;
                let mut xs = Vec::with_capacity(r.cells.len() * (g + 1) * (g + 1));
                for (_, o, a, b) in &r.cells {
                    for i in 0..=g {
                        for j in 0..=g {
                            xs.push(*o + *a * (i as f64 / g as f64) + *b * (j as f64 / g as f64));
                        }
                    }
                }
                xs
            }
            _ => self.grid(12),
        };
        self.seeds = xs.into_iter().map(|x| (x, self.point(x))).collect();
    }
}

/// A convex polygon with these side lengths, its corners on a circle, counter-clockwise.
fn cyclic_polygon(len: &[f64]) -> Vec<Point2> {
    let lmax = len.iter().copied().fold(0.0, f64::max);
    let big = len.iter().position(|&l| l == lmax).unwrap_or(0);
    // angle each side subtends at the centre for radius r; the longest may subtend more
    // than pi when the centre falls outside the polygon
    let angles = |r: f64, outside: bool| -> Vec<f64> {
        len.iter()
            .enumerate()
            .map(|(i, &l)| {
                let a = 2.0 * dmath::asin((l / (2.0 * r)).min(1.0));
                if outside && i == big { dmath::TAU - a } else { a }
            })
            .collect()
    };
    let total = |r: f64, outside: bool| angles(r, outside).iter().sum::<f64>() - dmath::TAU;
    let outside = total(0.5 * lmax, false) < 0.0;
    let (mut lo, mut hi) = (0.5 * lmax, lmax * len.len() as f64);
    for _ in 0..200 {
        let m = 0.5 * (lo + hi);
        if m <= lo || m >= hi {
            break;
        }
        // the sum falls with r inside; outside it rises (the long side's angle shrinks toward pi)
        let f = total(m, outside);
        if (f > 0.0) != outside {
            lo = m;
        } else {
            hi = m;
        }
    }
    let r = 0.5 * (lo + hi);
    let mut a = -dmath::FRAC_PI_2 - 0.5 * angles(r, outside)[0];
    let mut out = Vec::with_capacity(len.len());
    for t in angles(r, outside) {
        let (s, c) = dmath::sin_cos(a);
        out.push(Point2::new(r * c, r * s));
        a += t;
    }
    out
}

/// The unit tangent along side i's direction of travel and the inward e2 = n x e1.
fn frame_at(side: &Side, j: &RailJet) -> (Vec3, Vec3) {
    let e1 = (j.dp * (side.t1 - side.t0))
        .normalize()
        .map_or(Vec3::ZERO, |u| u.get());
    (e1, j.n.cross(e1))
}

/// The radius of the sharpest sampled curvature `kmax`, between a thousandth of the longest
/// side and the longest side: positive and finite even where a sample sits on a kinked corner.
fn sampled_radius(kmax: f64, sides: &[Side]) -> f64 {
    let size = sides.iter().map(|s| s.chord_length()).fold(0.0, f64::max);
    (1.0 / kmax).clamp(1e-3 * size, size)
}

fn surf_deriv(j: JV) -> SurfDeriv {
    SurfDeriv {
        p: Point3::new(j.v.x, j.v.y, j.v.z),
        du: j.u,
        dv: j.w,
        duu: j.uu,
        duv: j.uw,
        dvv: j.ww,
    }
}

/// Largest absolute principal curvature at a regular point.
fn max_curvature(d: &SurfDeriv) -> f64 {
    let Some(n) = d.du.cross(d.dv).normalize() else {
        return 0.0;
    };
    let (e, f, g) = (d.du.dot(d.du), d.du.dot(d.dv), d.dv.dot(d.dv));
    let (l, m, nn) = (d.duu.dot(n.get()), d.duv.dot(n.get()), d.dvv.dot(n.get()));
    let det = e * g - f * f;
    let h = (e * nn - 2.0 * f * m + g * l) / (2.0 * det);
    let k = (l * nn - m * m) / det;
    h.abs() + (h * h - k).max(0.0).sqrt()
}

impl HolePatch {
    /// A box holding the rational quads over b: each cell's over the part of b in it. Points
    /// outside the domain evaluate at their nearest domain point: within half b's diagonal of
    /// the centre's for a convex domain (four cells or fewer), else within the centre's
    /// distance from the domain plus the whole diagonal of the centre.
    fn cells_bbox(&self, r: &Cells, b: Box2) -> Box3 {
        let corners = [
            Point2::new(b.u.lo, b.v.lo),
            Point2::new(b.u.hi, b.v.lo),
            Point2::new(b.u.lo, b.v.hi),
            Point2::new(b.u.hi, b.v.hi),
        ];
        let convex = r.cells.len() <= 4;
        let w = self.corners.len();
        let within = corners.iter().all(|&c| self.inside(c))
            && (convex || (0..w).all(|k| !meets(&b, self.corners[k], self.corners[(k + 1) % w])));
        let b = if within {
            b
        } else {
            let half = 0.5 * (b.u.width() * b.u.width() + b.v.width() * b.v.width()).sqrt();
            let c = self.clamp(b.center());
            let r = if convex {
                half
            } else {
                2.0 * half + c.distance(b.center())
            };
            let r = (r * (1.0 + 1e-15)).next_up();
            Box2::new(Interval::new(c.x - r, c.x + r), Interval::new(c.y - r, c.y + r))
        };
        let corners = [
            Point2::new(b.u.lo, b.v.lo),
            Point2::new(b.u.hi, b.v.lo),
            Point2::new(b.u.lo, b.v.hi),
            Point2::new(b.u.hi, b.v.hi),
        ];
        let mut out = Box3::EMPTY;
        for (c, (q, ..)) in r.cells.iter().enumerate() {
            let (mut u, mut v) = (Interval::EMPTY, Interval::EMPTY);
            for &x in &corners {
                let (cu, cv, _, _) = r.local(c, x);
                u.include(cu);
                v.include(cv);
            }
            // the inverse map's rounding
            let (u, v) = (
                Interval::new((u.lo - 1e-12).max(0.0), (u.hi + 1e-12).min(1.0)),
                Interval::new((v.lo - 1e-12).max(0.0), (v.hi + 1e-12).min(1.0)),
            );
            if u.lo <= u.hi && v.lo <= v.hi {
                out = out.hull(&q.bbox(u, v));
            }
        }
        out
    }
}

/// Parameters of the point nearest p, by Newton from x (Gauss-Newton where the distance is not
/// convex) inside the convex polygon `lines` bound (d_i(x) = g . x + h >= 0), sliding along a
/// side the step would leave by; `size` is the domain's.
fn newton(
    p: Point3,
    x: Point2,
    size: f64,
    lines: &[(Vec2, f64)],
    clamp: impl Fn(Point2) -> Point2,
    deriv: impl Fn(Point2) -> SurfDeriv,
    point: impl Fn(Point2) -> Point3,
) -> Point2 {
    let mut x = x;
    let mut d = deriv(x);
    for _ in 0..50 {
        let r = d.p - p;
        let f = r.length_sq();
        let (gu, gv) = (r.dot(d.du), r.dot(d.dv));
        let (e, fm, g) = (d.du.dot(d.du), d.du.dot(d.dv), d.dv.dot(d.dv));
        let (h11, h12, h22) = (e + r.dot(d.duu), fm + r.dot(d.duv), g + r.dot(d.dvv));
        // Newton where the distance is convex, Gauss-Newton elsewhere
        let step = if h11 > 0.0 && h11 * h22 - h12 * h12 > 0.0 {
            linalg::solve2(h11, h12, h12, h22, -gu, -gv)
        } else {
            linalg::solve2(e, fm, fm, g, -gu, -gv)
        };
        let Some((mut a, mut b)) = step else { break };
        // on a side the step would leave by, Newton along that side instead
        let mut active = lines
            .iter()
            .filter(|&&(n, h)| n.dot(x.to_vec()) + h <= 1e-12 * size && n.x * a + n.y * b < 0.0);
        if let Some(&(n, _)) = active.next() {
            if active.next().is_some() {
                break;
            }
            let (tu, tv) = (-n.y, n.x);
            let (gt, ht) = (
                gu * tu + gv * tv,
                h11 * tu * tu + 2.0 * h12 * tu * tv + h22 * tv * tv,
            );
            let ht = if ht > 0.0 {
                ht
            } else {
                e * tu * tu + 2.0 * fm * tu * tv + g * tv * tv
            };
            let sigma = -gt / ht;
            (a, b) = (sigma * tu, sigma * tv);
        }
        let len = (a * a + b * b).sqrt();
        if f == 0.0 || len <= 1e-13 * size {
            break;
        }
        // the full step is evaluated with derivatives, as it is usually the next iterate
        let y = clamp(Point2::new(x.x + a, x.y + b));
        let dy = deriv(y);
        let next = if dy.p.distance_sq(p) <= f {
            Some((y, dy))
        } else if len <= 1e-9 * size {
            // a step this small only fails to rounding: converged
            None
        } else {
            let mut lam = 0.5;
            let mut found = None;
            for _ in 0..40 {
                let y = clamp(Point2::new(x.x + a * lam, x.y + b * lam));
                if point(y).distance_sq(p) <= f {
                    found = Some((y, deriv(y)));
                    break;
                }
                lam *= 0.5;
            }
            found
        };
        let Some((y, dy)) = next else { break };
        let moved = y.distance(x);
        (x, d) = (y, dy);
        if moved <= 1e-13 * size {
            break;
        }
    }
    x
}

/// Whether the segment p q meets the box (Liang-Barsky).
fn meets(b: &Box2, p: Point2, q: Point2) -> bool {
    let d = q - p;
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (pk, qk) in [
        (-d.x, p.x - b.u.lo),
        (d.x, b.u.hi - p.x),
        (-d.y, p.y - b.v.lo),
        (d.y, b.v.hi - p.y),
    ] {
        if pk == 0.0 {
            if qk < 0.0 {
                return false;
            }
        } else if pk < 0.0 {
            t0 = t0.max(qk / pk);
        } else {
            t1 = t1.min(qk / pk);
        }
    }
    t0 <= t1
}

impl Procedural for HolePatch {
    fn point(&self, uv: Point2) -> Point3 {
        HolePatch::point(self, uv)
    }
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        surf_deriv(self.eval::<J>(uv))
    }
    fn domain(&self) -> Domain2 {
        let d = |i: Interval| Domain1 {
            lo: i.lo,
            hi: i.hi,
            period: None,
        };
        Domain2 {
            u: d(self.rect.u),
            v: d(self.rect.v),
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        let (mut x, mut f) = (self.seeds[0].0, f64::INFINITY);
        for &(uv, q) in &self.seeds {
            let d = q.distance_sq(p);
            if d < f {
                (x, f) = (uv, d);
            }
        }
        if let Some(r) = &self.rational {
            // the nearest seed may be across a seam from p's cell, where a tight quad can hold
            // a false minimum: from every cell's nearest seed within twice the nearest's distance
            let m = r.cells.len();
            let per = self.seeds.len() / m;
            let mut best = (f64::INFINITY, x);
            for c in 0..m {
                let near = self.seeds[c * per..(c + 1) * per]
                    .iter()
                    .map(|&(uv, q)| (q.distance_sq(p), uv))
                    .min_by(|a, b| a.0.total_cmp(&b.0))
                    .expect("cells have seeds");
                if near.0 <= 4.0 * f {
                    // in the seed's own cell: a seed on a seam belongs to both
                    let y = r.project(p, c, near.1);
                    let e = self.point(y).distance_sq(p);
                    if e < best.0 {
                        best = (e, y);
                    }
                }
            }
            return best.1;
        }
        let size = self.rect.u.width().max(self.rect.v.width());
        newton(
            p,
            x,
            size,
            &self.lines,
            |y| self.clamp(y),
            |y| self.deriv(y),
            |y| self.point(y),
        )
    }
    fn bbox(&self, b: Box2) -> Box3 {
        if let Some(r) = &self.rational {
            return self.cells_bbox(r, b);
        }
        let n = self.sides.len();
        let corners_in = [
            Point2::new(b.u.lo, b.v.lo),
            Point2::new(b.u.hi, b.v.lo),
            Point2::new(b.u.lo, b.v.hi),
            Point2::new(b.u.hi, b.v.hi),
        ]
        .iter()
        .all(|&c| self.inside(c));
        let b = if corners_in {
            b
        } else {
            // points outside the polygon evaluate at their nearest polygon point, which moves
            // no further from the centre's than they are
            let c = self.clamp(b.center());
            let r = (0.5 * (b.u.width() * b.u.width() + b.v.width() * b.v.width()).sqrt() * (1.0 + 1e-15))
                .next_up();
            Box2::new(Interval::new(c.x - r, c.x + r), Interval::new(c.y - r, c.y + r))
        };
        let mut d = [Interval::point(0.0); MAX_SIDES];
        for (i, &(g, h)) in self.lines.iter().enumerate() {
            let di = b.u * g.x + b.v * g.y + h;
            d[i] = Interval::new(di.lo.max(0.0), di.hi.max(0.0));
        }
        let mut sq = [Interval::point(0.0); MAX_SIDES];
        for i in 0..n {
            sq[i] = d[i].sqr();
        }
        let mut w = [Interval::point(1.0); MAX_SIDES];
        for (i, wi) in w.iter_mut().enumerate().take(n) {
            for (j, s) in sq.iter().enumerate().take(n) {
                if j != i {
                    *wi = *wi * *s;
                }
            }
        }
        let mut total = Interval::point(0.0);
        for wi in &w[..n] {
            total = total + *wi;
        }
        let div = |a: f64, b: f64, empty: f64| if b > 0.0 { a / b } else { empty };
        let mut out = [Interval::point(0.0); 3];
        let mut hull = Box3::EMPTY;
        for i in 0..n {
            let (a, c) = (d[(i + n - 1) % n], d[(i + 1) % n]);
            let s = Interval::new(
                div(a.lo, a.lo + c.hi, 0.0).next_down().max(0.0),
                div(a.hi, a.hi + c.lo, 1.0).next_up().min(1.0),
            );
            let side = &self.sides[i];
            let t = s * (side.t1 - side.t0) + side.t0;
            let rb = side.rail.bound(t);
            let [a0, b0, a1, b1] = self.ends[i];
            let (ca, cb) = (s * (a1 - a0) + a0, s * (b1 - b0) + b0);
            let c1 = ivec(&rb.dp).map(|c| c * (side.t1 - side.t0));
            let len = idot(c1, c1).sqrt();
            let e1 = c1.map(|c| meet(c.div(&len), UNIT));
            let nv = ivec(&rb.n);
            let e2 = [
                nv[1] * e1[2] - nv[2] * e1[1],
                nv[2] * e1[0] - nv[0] * e1[2],
                nv[0] * e1[1] - nv[1] * e1[0],
            ];
            let pv = ivec(&rb.p);
            // h(d) = d / (1 + d / height) rises with d
            let hh = self.heights[i];
            let damp = |x: f64| x / (1.0 + x / hh);
            let h = Interval::new(
                damp(d[i].lo).next_down().next_down().max(0.0),
                damp(d[i].hi).next_up().next_up(),
            );
            let (k0, k1) = self.bends[i];
            let (k0, k1) = ([k0.x, k0.y, k0.z], [k1.x, k1.y, k1.z]);
            let hh2 = h.sqr() * 0.5;
            let rib: IVec = std::array::from_fn(|k| {
                pv[k] + h * (ca * e1[k] + cb * e2[k]) + hh2 * (s * (k1[k] - k0[k]) + k0[k])
            });
            hull = hull.hull(&ibox(rib));
            // B_i = P_i / (P_i + Q_i) rises with P_i and falls with Q_i
            let q = total - w[i];
            let q = Interval::new(q.lo.max(0.0), q.hi.max(0.0));
            let bw = Interval::new(
                div(w[i].lo, w[i].lo + q.hi, 0.0).next_down().max(0.0),
                div(w[i].hi, w[i].hi + q.lo, 1.0).next_up().min(1.0),
            );
            for k in 0..3 {
                out[k] = out[k] + bw * rib[k];
            }
        }
        let bx = ibox(std::array::from_fn(|k| meet(out[k], ivec(&hull)[k])));
        // room for the rounding of the pointwise evaluation
        let m =
            bx.x.lo
                .abs()
                .max(bx.x.hi.abs())
                .max(bx.y.lo.abs().max(bx.y.hi.abs()))
                .max(bx.z.lo.abs().max(bx.z.hi.abs()));
        bx.grow(64.0 * f64::EPSILON * m)
    }
    fn implicit(&self, p: Point3) -> f64 {
        let d = self.deriv(self.project(p));
        let r = p - d.p;
        let side = d.du.cross(d.dv).dot(r);
        if side < 0.0 { -r.length() } else { r.length() }
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        let d = self.deriv(self.project(p));
        let n = d.du.cross(d.dv).normalize().map_or(Vec3::Z, |u| u.get());
        let r = p - d.p;
        match r.normalize() {
            Some(u) if r.length() > tol::LINEAR => {
                if n.dot(r) < 0.0 {
                    -u.get()
                } else {
                    u.get()
                }
            }
            _ => n,
        }
    }
    fn transformed(&self, f: &Frame) -> Proc {
        let sides = self
            .sides
            .iter()
            .map(|s| Side::new(s.rail.transformed(f), s.t0, s.t1))
            .collect();
        Proc(Arc::new(HolePatch {
            sides,
            corners: self.corners.clone(),
            lines: self.lines.clone(),
            ends: self.ends.clone(),
            bends: self
                .bends
                .iter()
                .map(|&(a, b)| (f.vec_to_world(a), f.vec_to_world(b)))
                .collect(),
            rect: self.rect,
            side: self.side,
            heights: self.heights.clone(),
            seeds: self
                .seeds
                .iter()
                .map(|&(x, p)| (x, f.point_to_world(p)))
                .collect(),
            scale: self.scale,
            radius: self.radius,
            fold: self.fold,
            rational: self.rational.as_ref().map(|r| {
                Arc::new(Cells {
                    cells: r
                        .cells
                        .iter()
                        .map(|(q, o, a, b)| (q.transformed(f), *o, *a, *b))
                        .collect(),
                    pieces: r.pieces.clone(),
                })
            }),
        }))
    }
    fn uv_scale(&self) -> (f64, f64) {
        self.scale
    }
    /// The sample; min_radius is unknown. Round a corner where the sides' normals differ the
    /// normal turns in no distance (research/corner-patch/kmin.rs); elsewhere the ribbon form
    /// has no cheap bound and the rational quads' is not proven yet (a sample runs 25-40% over
    /// the dense one).
    fn mesh_radius(&self) -> f64 {
        self.radius
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    /// First derivatives by first-order jets (the second left zero).
    fn column_d1(&self, u: f64, vs: &[f64], out: &mut Vec<SurfDeriv>) {
        out.extend(vs.iter().map(|&v| {
            let j = self.eval::<J1>(Point2::new(u, v));
            SurfDeriv {
                p: Point3::ORIGIN + j.v,
                du: j.u,
                dv: j.w,
                duu: Vec3::ZERO,
                duv: Vec3::ZERO,
                dvv: Vec3::ZERO,
            }
        }))
    }
}

/// A curve on `surface` from `p0` leaving along `t0` to `p1` arriving along `t1`, tangent to
/// both: the quadratic Bezier whose middle control point is where the two tangent lines meet,
/// drawn in the surface's parameter plane (on a plane, exactly a parabola). None when the
/// tangent lines do not meet ahead of `p0` and behind `p1`.
pub fn trim_curve(
    surface: &Surface,
    p0: Point3,
    t0: Vec3,
    p1: Point3,
    t1: Vec3,
) -> Option<(Curve, Interval)> {
    if let Surface::Plane(_) = surface {
        let (mu, _) = meet_ahead(p1 - p0, t0, t1)?;
        if mu == 0.0 {
            let l = crate::curve::Line::through(p0, p1)?;
            return Some((Curve::Line(l), Interval::new(0.0, p0.distance(p1))));
        }
        return bezier_parabola(p0, p0 + t0 * mu, p1);
    }
    let a = surface.project(p0);
    let b = unwrap_near(surface, surface.project(p1), a);
    let (ta, tb) = (
        uv_tangent(&surface.deriv(a), t0)?,
        uv_tangent(&surface.deriv(b), t1)?,
    );
    let to3 = |v: Vec2| Vec3::new(v.x, v.y, 0.0);
    let (mu, _) = meet_ahead(to3(b - a), to3(ta), to3(tb))?;
    let q = if mu == 0.0 { a.lerp(b, 0.5) } else { a + ta * mu };
    let c = SurfaceBezier {
        surface: surface.clone(),
        ctrl: [a, q, b],
    };
    Some((
        Curve::Procedural(crate::curve::ProcCurve(Arc::new(c))),
        Interval::new(0.0, 1.0),
    ))
}

/// mu, nu > 0 with mu t0 + nu t1 = d; (0, 0) when d runs along both (a straight segment).
fn meet_ahead(d: Vec3, t0: Vec3, t1: Vec3) -> Option<(f64, f64)> {
    let (a, b, c) = (t0.dot(t0), t0.dot(t1), t1.dot(t1));
    let det = a * c - b * b;
    if det <= 1e-24 * a * c {
        let parallel =
            d.cross(t0).length() <= tol::LINEAR * t0.length() && d.dot(t0) > 0.0 && t0.dot(t1) > 0.0;
        return parallel.then_some((0.0, 0.0));
    }
    let (mu, nu) = linalg::solve2(a, b, b, c, d.dot(t0), d.dot(t1))?;
    // the tangent lines must actually meet (the three points span a plane with both tangents)
    if (t0 * mu + t1 * nu - d).length() > tol::LINEAR || mu <= 0.0 || nu <= 0.0 {
        return None;
    }
    Some((mu, nu))
}

fn uv_tangent(d: &SurfDeriv, t: Vec3) -> Option<Vec2> {
    let (e, f, g) = (d.du.dot(d.du), d.du.dot(d.dv), d.dv.dot(d.dv));
    let (a, b) = linalg::solve2(e, f, f, g, t.dot(d.du), t.dot(d.dv))?;
    Some(Vec2::new(a, b))
}

fn unwrap_near(s: &Surface, uv: Point2, r: Point2) -> Point2 {
    let d = s.domain();
    let fix = |x: f64, r: f64, period: Option<f64>| match period {
        Some(p) => x + p * ((r - x) / p).round(),
        None => x,
    };
    Point2::new(fix(uv.x, r.x, d.u.period), fix(uv.y, r.y, d.v.period))
}

/// The quadratic Bezier p0, q, p1 as the parabola it is.
pub fn bezier_parabola(p0: Point3, q: Point3, p1: Point3) -> Option<(Curve, Interval)> {
    let a = (p0 - q) + (p1 - q);
    let al = a.length();
    let d0 = (q - p0) * 2.0;
    if al <= 1e-12 * d0.length() {
        let l = crate::curve::Line::through(p0, p1)?;
        return Some((Curve::Line(l), Interval::new(0.0, p0.distance(p1))));
    }
    // B(s) = vertex + (s - s*) B'(s*) + (s - s*)^2 a, with B'(s*) perpendicular to a
    let s0 = -d0.dot(a) / (2.0 * al * al);
    let v = p0 + d0 * s0 + a * (s0 * s0);
    let tv = d0 + a * (2.0 * s0);
    let l = tv.length();
    let x = a.normalize()?;
    let frame = Frame::from_zx(v, x.cross(tv).normalize()?, a)?;
    let par = crate::curve::Parabola {
        frame,
        focal: l * l / (4.0 * al),
    };
    Some((Curve::Parabola(par), Interval::new(-s0 * l, (1.0 - s0) * l)))
}

/// The image on a surface of a quadratic Bezier in its parameter plane, t in [0, 1]: exactly
/// on the surface, leaving and arriving along the control polygon's legs.
#[derive(Clone, Debug)]
pub struct SurfaceBezier {
    pub surface: Surface,
    pub ctrl: [Point2; 3],
}

impl SurfaceBezier {
    fn uv(&self, t: f64) -> (Point2, Vec2, Vec2) {
        let [a, q, b] = self.ctrl;
        let acc = (a - q) + (b - q);
        let d0 = (q - a) * 2.0;
        (a + d0 * t + acc * (t * t), d0 + acc * (2.0 * t), acc * 2.0)
    }
    /// The control points of the piece over `r`.
    fn sub(&self, r: Interval) -> [Point2; 3] {
        let (a, da, _) = self.uv(r.lo);
        let (b, _, _) = self.uv(r.hi);
        [a, a + da * (0.5 * r.width()), b]
    }
    fn uv_box(&self, r: Interval) -> Box2 {
        let c = self.sub(r);
        let mut b = Box2::new(Interval::EMPTY, Interval::EMPTY);
        for p in c {
            b.u.include(p.x);
            b.v.include(p.y);
        }
        // the sub-control points are computed: one rounding step of room
        let e =
            4.0 * f64::EPSILON * (1.0 + b.u.lo.abs().max(b.u.hi.abs()).max(b.v.lo.abs().max(b.v.hi.abs())));
        Box2::new(b.u.grow(e), b.v.grow(e))
    }
}

impl crate::curve::ProceduralCurve for SurfaceBezier {
    fn exact_d1(&self) -> bool {
        true
    }
    fn deriv(&self, t: f64) -> CurveDeriv {
        let (x, x1, x2) = self.uv(t);
        let d = self.surface.deriv(x);
        CurveDeriv {
            p: d.p,
            d1: d.du * x1.x + d.dv * x1.y,
            d2: d.duu * (x1.x * x1.x)
                + d.duv * (2.0 * x1.x * x1.y)
                + d.dvv * (x1.y * x1.y)
                + d.du * x2.x
                + d.dv * x2.y,
        }
    }
    fn domain(&self) -> Domain1 {
        Domain1 {
            lo: 0.0,
            hi: 1.0,
            period: None,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let mut best = (f64::INFINITY, range.lo);
        for k in 0..=32 {
            let t = range.lo + range.width() * k as f64 / 32.0;
            let d = self.deriv(t.min(range.hi)).p.distance_sq(p);
            if d < best.0 {
                best = (d, t.min(range.hi));
            }
        }
        let mut t = best.1;
        for _ in 0..30 {
            let d = self.deriv(t);
            let r = d.p - p;
            let g = r.dot(d.d1);
            let h = d.d1.dot(d.d1) + r.dot(d.d2);
            let h = if h > 0.0 { h } else { d.d1.dot(d.d1) };
            if h.is_nan() || h <= 0.0 {
                break;
            }
            let next = (t - g / h).clamp(range.lo, range.hi);
            if self.deriv(next).p.distance_sq(p) > r.length_sq() {
                break;
            }
            let moved = (next - t).abs();
            t = next;
            if moved <= 1e-16 {
                break;
            }
        }
        t
    }
    fn bbox(&self, range: Interval) -> Box3 {
        self.surface.bbox(self.uv_box(range))
    }
    fn transformed(&self, f: &Frame) -> crate::curve::ProcCurve {
        crate::curve::ProcCurve(Arc::new(SurfaceBezier {
            surface: self.surface.transformed(f),
            ctrl: self.ctrl,
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        let b = self.uv_box(range);
        let (ju, jv) = jacobian_bound(&self.surface, b);
        let (_, a, _) = self.uv(range.lo);
        let (_, c, _) = self.uv(range.hi);
        // the parameter-plane velocity is linear in t, largest at an end
        let (mu, mv) = (a.x.abs().max(c.x.abs()), a.y.abs().max(c.y.abs()));
        (ju * mu + jv * mv) * (1.0 + 1e-12)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        let mut acc: f64 = 0.0;
        for k in 0..=8 {
            acc = acc.max(self.deriv(range.lo + range.width() * k as f64 / 8.0).d2.length());
        }
        // a chord of parameter length h strays by about |P''| h^2 / 8
        let n = (range.width() * (acc / (8.0 * chord)).sqrt()).ceil();
        (n as usize).max(1)
    }
}

/// Bounds on |S_u| and |S_v| over a parameter box.
fn jacobian_bound(s: &Surface, b: Box2) -> (f64, f64) {
    match s {
        Surface::Plane(_) => (1.0, 1.0),
        Surface::Cylinder(c) => (c.radius, 1.0),
        Surface::Cone(c) => {
            let r = (Interval::point(c.radius) + b.v * c.tan_a()).hi.abs();
            (r.max(c.radius_at(b.v.lo).abs()), 1.0 / c.cos_a())
        }
        Surface::Sphere(sp) => (sp.radius, sp.radius),
        Surface::Torus(t) => (t.major + t.minor, t.minor),
        Surface::Procedural(_) => {
            // no closed form: the largest sampled, doubled
            let (mut u, mut v): (f64, f64) = (0.0, 0.0);
            for i in 0..=4 {
                for j in 0..=4 {
                    let x = Point2::new(
                        b.u.lo + b.u.width() * i as f64 / 4.0,
                        b.v.lo + b.v.width() * j as f64 / 4.0,
                    );
                    let d = s.deriv(x);
                    u = u.max(d.du.length());
                    v = v.max(d.dv.length());
                }
            }
            (2.0 * u, 2.0 * v)
        }
    }
}

/// How far a patch is from its contract, measured on samples.
#[derive(Clone, Copy, Debug, Default)]
pub struct Audit {
    /// Largest distance from the patch to its boundary curves along the sides (mm).
    pub boundary: f64,
    /// Largest angle between the patch normal and the neighbour's along the sides, away
    /// from the corners (radians).
    pub normal: f64,
    /// Largest relative mismatch of deriv against central differences of itself.
    pub derivs: f64,
    /// Largest distance from a random point to the patch at its projection, for points on it.
    pub project: f64,
    /// Samples outside the bbox of their parameter box.
    pub bbox_misses: usize,
    /// Smallest cosine between the normals of neighbouring points of a 40 x 40 grid over the
    /// domain: negative where the patch folds over.
    pub fold: f64,
}

#[doc(hidden)]
pub fn audit(p: &HolePatch, seed: u64) -> Audit {
    let mut r = forge_math::rng::Rng::new(seed);
    let mut a = Audit {
        fold: f64::INFINITY,
        ..Audit::default()
    };
    let n = p.sides.len();
    for i in 0..n {
        let side = &p.sides[i];
        for k in 0..=64 {
            let s = k as f64 / 64.0;
            let x = p.side_param_uv(i, side.at(s));
            let (q, nb) = side.rail.point(side.at(s));
            a.boundary = a.boundary.max(p.point(x).distance(q));
            if (1..64).contains(&k) {
                let d = p.deriv(x);
                let np = d.du.cross(d.dv).normalize().unwrap();
                let nb = nb.normalize().unwrap();
                a.normal = a.normal.max(dmath::atan2(np.cross(nb).length(), np.dot(nb)));
            }
        }
    }
    let m = 40;
    let at = |i: f64, j: f64| {
        Point2::new(
            p.rect.u.lo + p.rect.u.width() * i / m as f64,
            p.rect.v.lo + p.rect.v.width() * j / m as f64,
        )
    };
    let unit = |i: usize, j: usize| {
        let x = at(i as f64, j as f64);
        let d = p.deriv(x);
        p.inside(x).then(|| d.du.cross(d.dv).normalize()).flatten()
    };
    let grid: Vec<Vec<Option<Vec3>>> = (0..=m)
        .map(|i| (0..=m).map(|j| unit(i, j).map(|u| u.get())).collect())
        .collect();
    for i in 0..=m {
        for j in 0..=m {
            let Some(a0) = grid[i][j] else { continue };
            // (not across a notch of the domain)
            let (fi, fj) = (i as f64, j as f64);
            for b in [
                grid.get(i + 1)
                    .and_then(|r| r[j])
                    .filter(|_| p.inside(at(fi + 0.5, fj))),
                grid[i]
                    .get(j + 1)
                    .copied()
                    .flatten()
                    .filter(|_| p.inside(at(fi, fj + 0.5))),
            ]
            .into_iter()
            .flatten()
            {
                a.fold = a.fold.min(a0.dot(b));
            }
        }
    }
    let rand_in = |r: &mut forge_math::rng::Rng| loop {
        let x = Point2::new(
            r.range(p.rect.u.lo, p.rect.u.hi),
            r.range(p.rect.v.lo, p.rect.v.hi),
        );
        if p.inside(x) {
            return x;
        }
    };
    let size = p.rect.u.width();
    for _ in 0..200 {
        let x = rand_in(&mut r);
        let d = p.deriv(x);
        let h = 1e-5 * size;
        let pu = Point2::new(x.x + h, x.y);
        let mu = Point2::new(x.x - h, x.y);
        let pv = Point2::new(x.x, x.y + h);
        let mv = Point2::new(x.x, x.y - h);
        // central differences within one cell (across a seam the parametrisation is only C0)
        let cell = |y: Point2| p.rational.as_ref().map(|r| r.cell_of(y).0);
        if [pu, mu, pv, mv]
            .iter()
            .all(|&y| p.inside(y) && cell(y) == cell(x))
        {
            let rel = |fd: Vec3, an: Vec3| (fd - an).length() / (1.0 + an.length());
            let e = [
                rel((p.point(pu) - p.point(mu)) / (2.0 * h), d.du),
                rel((p.point(pv) - p.point(mv)) / (2.0 * h), d.dv),
                rel((p.deriv(pu).du - p.deriv(mu).du) / (2.0 * h), d.duu),
                rel((p.deriv(pv).du - p.deriv(mv).du) / (2.0 * h), d.duv),
                rel((p.deriv(pv).dv - p.deriv(mv).dv) / (2.0 * h), d.dvv),
            ];
            a.derivs = e.iter().fold(a.derivs, |m, &v| m.max(v));
        }
        let q = d.p;
        let back = p.project(q);
        a.project = a.project.max(p.point(back).distance(q));
        let y = rand_in(&mut r);
        let bx = Box2::new(Interval::new(x.x, y.x), Interval::new(x.y, y.y));
        let bb = p.bbox(bx);
        for i in 0..=6 {
            for j in 0..=6 {
                let z = Point2::new(
                    (bx.u.lo + bx.u.width() * i as f64 / 6.0).min(bx.u.hi),
                    (bx.v.lo + bx.v.width() * j as f64 / 6.0).min(bx.v.hi),
                );
                if !bb.contains(p.point(z)) {
                    a.bbox_misses += 1;
                }
            }
        }
    }
    a
}

/// A number with its first and second partial derivatives in (u, v), or a plain f64.
trait Scalar: Copy + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> {
    type V: Copy + Add<Output = Self::V> + Sub<Output = Self::V> + Mul<Self, Output = Self::V>;
    const JET: bool;
    fn konst(v: f64) -> Self;
    fn affine(v: f64, gu: f64, gv: f64) -> Self;
    fn value(self) -> f64;
    fn inv(self) -> Self;
    fn zero_v() -> Self::V;
    /// c(self), given c and its first two derivatives at self's value.
    fn along(self, c: Vec3, c1: Vec3, c2: Vec3) -> Self::V;
}

impl Scalar for f64 {
    type V = Vec3;
    const JET: bool = false;
    fn konst(v: f64) -> f64 {
        v
    }
    fn affine(v: f64, _: f64, _: f64) -> f64 {
        v
    }
    fn value(self) -> f64 {
        self
    }
    fn inv(self) -> f64 {
        1.0 / self
    }
    fn zero_v() -> Vec3 {
        Vec3::ZERO
    }
    fn along(self, c: Vec3, _: Vec3, _: Vec3) -> Vec3 {
        c
    }
}

#[derive(Clone, Copy, Debug)]
struct J {
    v: f64,
    u: f64,
    w: f64,
    uu: f64,
    uw: f64,
    ww: f64,
}

#[derive(Clone, Copy, Debug)]
struct JV {
    v: Vec3,
    u: Vec3,
    w: Vec3,
    uu: Vec3,
    uw: Vec3,
    ww: Vec3,
}

impl Add for J {
    type Output = J;
    fn add(self, o: J) -> J {
        J {
            v: self.v + o.v,
            u: self.u + o.u,
            w: self.w + o.w,
            uu: self.uu + o.uu,
            uw: self.uw + o.uw,
            ww: self.ww + o.ww,
        }
    }
}

impl Sub for J {
    type Output = J;
    fn sub(self, o: J) -> J {
        J {
            v: self.v - o.v,
            u: self.u - o.u,
            w: self.w - o.w,
            uu: self.uu - o.uu,
            uw: self.uw - o.uw,
            ww: self.ww - o.ww,
        }
    }
}

impl Mul for J {
    type Output = J;
    fn mul(self, b: J) -> J {
        let a = self;
        J {
            v: a.v * b.v,
            u: a.u * b.v + a.v * b.u,
            w: a.w * b.v + a.v * b.w,
            uu: a.uu * b.v + 2.0 * a.u * b.u + a.v * b.uu,
            uw: a.uw * b.v + a.u * b.w + a.w * b.u + a.v * b.uw,
            ww: a.ww * b.v + 2.0 * a.w * b.w + a.v * b.ww,
        }
    }
}

impl Add for JV {
    type Output = JV;
    fn add(self, o: JV) -> JV {
        JV {
            v: self.v + o.v,
            u: self.u + o.u,
            w: self.w + o.w,
            uu: self.uu + o.uu,
            uw: self.uw + o.uw,
            ww: self.ww + o.ww,
        }
    }
}

impl Sub for JV {
    type Output = JV;
    fn sub(self, o: JV) -> JV {
        JV {
            v: self.v - o.v,
            u: self.u - o.u,
            w: self.w - o.w,
            uu: self.uu - o.uu,
            uw: self.uw - o.uw,
            ww: self.ww - o.ww,
        }
    }
}

impl Mul<J> for JV {
    type Output = JV;
    fn mul(self, b: J) -> JV {
        let a = self;
        JV {
            v: a.v * b.v,
            u: a.u * b.v + a.v * b.u,
            w: a.w * b.v + a.v * b.w,
            uu: a.uu * b.v + a.u * (2.0 * b.u) + a.v * b.uu,
            uw: a.uw * b.v + a.u * b.w + a.w * b.u + a.v * b.uw,
            ww: a.ww * b.v + a.w * (2.0 * b.w) + a.v * b.ww,
        }
    }
}

/// A first-order jet: value and its u and v derivatives.
#[derive(Clone, Copy, Debug)]
struct J1 {
    v: f64,
    u: f64,
    w: f64,
}

#[derive(Clone, Copy, Debug)]
struct JV1 {
    v: Vec3,
    u: Vec3,
    w: Vec3,
}

impl Add for J1 {
    type Output = J1;
    fn add(self, o: J1) -> J1 {
        J1 {
            v: self.v + o.v,
            u: self.u + o.u,
            w: self.w + o.w,
        }
    }
}

impl Sub for J1 {
    type Output = J1;
    fn sub(self, o: J1) -> J1 {
        J1 {
            v: self.v - o.v,
            u: self.u - o.u,
            w: self.w - o.w,
        }
    }
}

impl Mul for J1 {
    type Output = J1;
    fn mul(self, b: J1) -> J1 {
        J1 {
            v: self.v * b.v,
            u: self.u * b.v + self.v * b.u,
            w: self.w * b.v + self.v * b.w,
        }
    }
}

impl Add for JV1 {
    type Output = JV1;
    fn add(self, o: JV1) -> JV1 {
        JV1 {
            v: self.v + o.v,
            u: self.u + o.u,
            w: self.w + o.w,
        }
    }
}

impl Sub for JV1 {
    type Output = JV1;
    fn sub(self, o: JV1) -> JV1 {
        JV1 {
            v: self.v - o.v,
            u: self.u - o.u,
            w: self.w - o.w,
        }
    }
}

impl Mul<J1> for JV1 {
    type Output = JV1;
    fn mul(self, b: J1) -> JV1 {
        JV1 {
            v: self.v * b.v,
            u: self.u * b.v + self.v * b.u,
            w: self.w * b.v + self.v * b.w,
        }
    }
}

impl Scalar for J1 {
    type V = JV1;
    const JET: bool = true;
    fn konst(v: f64) -> J1 {
        J1 { v, u: 0.0, w: 0.0 }
    }
    fn affine(v: f64, gu: f64, gv: f64) -> J1 {
        J1 { v, u: gu, w: gv }
    }
    fn value(self) -> f64 {
        self.v
    }
    fn inv(self) -> J1 {
        let f = 1.0 / self.v;
        J1 {
            v: f,
            u: -f * f * self.u,
            w: -f * f * self.w,
        }
    }
    fn zero_v() -> JV1 {
        JV1 {
            v: Vec3::ZERO,
            u: Vec3::ZERO,
            w: Vec3::ZERO,
        }
    }
    fn along(self, c: Vec3, c1: Vec3, _: Vec3) -> JV1 {
        JV1 {
            v: c,
            u: c1 * self.u,
            w: c1 * self.w,
        }
    }
}

impl Scalar for J {
    type V = JV;
    const JET: bool = true;
    fn konst(v: f64) -> J {
        J {
            v,
            u: 0.0,
            w: 0.0,
            uu: 0.0,
            uw: 0.0,
            ww: 0.0,
        }
    }
    fn affine(v: f64, gu: f64, gv: f64) -> J {
        J {
            v,
            u: gu,
            w: gv,
            uu: 0.0,
            uw: 0.0,
            ww: 0.0,
        }
    }
    fn value(self) -> f64 {
        self.v
    }
    fn inv(self) -> J {
        let f = 1.0 / self.v;
        let (f1, f2) = (-f * f, 2.0 * f * f * f);
        J {
            v: f,
            u: f1 * self.u,
            w: f1 * self.w,
            uu: f2 * self.u * self.u + f1 * self.uu,
            uw: f2 * self.u * self.w + f1 * self.uw,
            ww: f2 * self.w * self.w + f1 * self.ww,
        }
    }
    fn zero_v() -> JV {
        let z = Vec3::ZERO;
        JV {
            v: z,
            u: z,
            w: z,
            uu: z,
            uw: z,
            ww: z,
        }
    }
    fn along(self, c: Vec3, c1: Vec3, c2: Vec3) -> JV {
        let s = self;
        JV {
            v: c,
            u: c1 * s.u,
            w: c1 * s.w,
            uu: c2 * (s.u * s.u) + c1 * s.uu,
            uw: c2 * (s.u * s.w) + c1 * s.uw,
            ww: c2 * (s.w * s.w) + c1 * s.ww,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Circle;
    use crate::testing::rand_frame;
    use forge_math::rng::Rng;

    /// The three quarter great circles bounding an octant of a sphere, counter-clockwise
    /// seen from outside.
    fn octant(f: &Frame, r: f64, outward: bool) -> Vec<Side> {
        let sphere = Surface::Sphere(Sphere { frame: *f, radius: r });
        let axes = [f.x(), f.y(), f.z()];
        (0..3)
            .map(|i| {
                let (a, b) = (axes[i], axes[(i + 1) % 3]);
                let frame = Frame::from_zx(f.origin, a.cross(b).normalize().unwrap(), a.get()).unwrap();
                let c = Curve::Circle(Circle { frame, radius: r });
                let rail = Arc::new(SurfaceRail::new(c, sphere.clone(), outward));
                Side::new(rail, 0.0, dmath::FRAC_PI_2)
            })
            .collect()
    }

    #[test]
    fn equal_ball_corner_is_the_sphere() {
        let mut rng = Rng::new(900);
        for _ in 0..50 {
            let f = rand_frame(&mut rng);
            let r = rng.range(0.5, 20.0);
            let Closing::One(Surface::Sphere(sp), same) = close_hole(octant(&f, r, true)).unwrap() else {
                panic!("not a sphere")
            };
            assert!(same && (sp.radius - r).abs() < 1e-9 && sp.frame.origin.distance(f.origin) < 1e-9);
            let c = close_hole(octant(&f, r, false)).unwrap();
            assert!(matches!(c, Closing::One(Surface::Sphere(_), false)));
        }
    }

    #[test]
    fn patch_on_sphere_octant_meets_contract() {
        let mut rng = Rng::new(901);
        for case in 0..20 {
            let f = rand_frame(&mut rng);
            let r = rng.range(0.5, 20.0);
            let mut sides = octant(&f, r, case % 2 == 0);
            if case % 3 == 0 {
                // the other orientation is accepted too
                sides.reverse();
                sides = sides.iter().map(Side::reversed).collect();
            }
            let p = HolePatch::new(sides).unwrap();
            let a = audit(&p, case);
            assert!(a.boundary <= 1e-9 * r.max(1.0), "{a:?}");
            assert!(a.normal <= 1e-9, "{a:?}");
            assert!(a.derivs <= 1e-4, "{a:?}");
            assert!(a.project <= 1e-9 * r.max(1.0), "{a:?}");
            assert_eq!(a.bbox_misses, 0);
            assert!(a.fold > 0.5, "{a:?}");
            // interior points stay near the sphere they interpolate
            for x in p.grid(8) {
                let d = p.point(x).distance(f.origin);
                assert!((d - r).abs() < 0.2 * r, "{d} {r}");
            }
        }
    }

    /// A square's sides in the plane z = 0, the top one the cubic `top` (from (1, 1) to
    /// (0, 1)) as a spoke.
    fn plane_quad(top: [Point3; 4]) -> Vec<Side> {
        let plane = Surface::Plane(crate::surface::Plane::new(Frame::WORLD));
        let line = |a: Point3, b: Point3| {
            let rail = Arc::new(SurfaceRail::new(
                Curve::Line(crate::curve::Line::through(a, b).unwrap()),
                plane.clone(),
                true,
            ));
            Side::new(rail, 0.0, a.distance(b))
        };
        let p = |x: f64, y: f64| Point3::new(x, y, 0.0);
        vec![
            line(p(0.0, 0.0), p(1.0, 0.0)),
            line(p(1.0, 0.0), p(1.0, 1.0)),
            Side::new(Arc::new(SpokeRail::cubic(top, Vec3::Z, Vec3::Z)), 0.0, 1.0),
            line(p(0.0, 1.0), p(0.0, 0.0)),
        ]
    }

    #[test]
    fn kinked_patch_has_a_distance_past_its_rim() {
        // the top spoke's normal tilts at its ends, so the corners there are kinked
        let p = |x: f64, y: f64| Point3::new(x, y, 0.0);
        let top = [p(1.0, 1.0), p(0.7, 1.0), p(0.3, 1.0), p(0.0, 1.0)];
        let mut sides = plane_quad(top);
        let n = Vec3::new(0.0, 0.3, 1.0);
        sides[2] = Side::new(Arc::new(SpokeRail::cubic(top, n, n)), 0.0, 1.0);
        let patch = HolePatch::new(sides).unwrap();
        assert!(patch.min_radius().is_none() && patch.radius > 0.0);
        for x in patch.grid(6) {
            assert!(patch.implicit(patch.point(x)).abs() < 1e-9);
        }
        // past every side and above and below the patch: finite, the gradient a unit vector
        for q in [p(0.5, -0.5), p(1.5, 0.5), p(0.5, 1.5), p(-0.5, 0.5), p(1.5, 1.5)] {
            for z in [-0.3, 0.0, 0.3] {
                let r = q + Vec3::new(0.0, 0.0, z);
                let (f, g) = (patch.implicit(r), patch.gradient(r));
                assert!(f.is_finite() && (g.length() - 1.0).abs() < 1e-12, "{r:?}");
            }
        }
    }

    #[test]
    fn rational_quad_proof_refuses_a_fold() {
        let p = |x: f64, y: f64| Point3::new(x, y, 0.0);
        // a straight top proves; one swinging down across the bottom side folds the patch over
        let flat = HolePatch::quad(plane_quad([p(1.0, 1.0), p(0.7, 1.0), p(0.3, 1.0), p(0.0, 1.0)])).unwrap();
        assert!(flat.prove_rational().unwrap().proved);
        let folded =
            HolePatch::quad(plane_quad([p(1.0, 1.0), p(0.2, 1.0), p(0.8, -1.5), p(0.0, 1.0)])).unwrap();
        assert!(!folded.prove_rational().unwrap().proved);
        // and it really does fold: the Jacobian takes both signs
        let k = folded.side;
        let (mut pos, mut neg) = (false, false);
        for i in 1..40 {
            for j in 1..40 {
                let d = folded.deriv(Point2::new(k * i as f64 / 40.0, k * j as f64 / 40.0));
                let z = d.du.cross(d.dv).z;
                (pos, neg) = (pos || z > 0.0, neg || z < 0.0);
            }
        }
        assert!(pos && neg);
        // a second proof of the same quad comes from the cache
        let again =
            HolePatch::quad(plane_quad([p(1.0, 1.0), p(0.7, 1.0), p(0.3, 1.0), p(0.0, 1.0)])).unwrap();
        let pr = again.prove_rational().unwrap();
        assert!(pr.proved && pr.cached);
    }
}

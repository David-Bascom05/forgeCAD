//! Variable-radius rolling-ball blends between two surfaces.
//!
//! A ball of radius r(s) rolls along the edge between surfaces A and B, touching both. Its
//! centre c(s) is the point at signed distance `side_a r(s)` from A and `side_b r(s)` from B
//! lying in the plane through the guide point g(s) perpendicular to g'(s). The blend is the
//! envelope of those balls (a canal surface): at each s the ball touches the envelope along its
//! characteristic circle, in the plane normal to c'(s), offset -r r'/|c'| from c, of radius
//! r sqrt(1 - r'^2/|c'|^2). The face is the arc of that circle between the two contact points.
//!
//! Parameters: u = s (the guide parameter), v in [0, 1] the fraction of the arc's angle. Points are
//! solved from the definition (Newton for c, then its Taylor coefficients order by order), never
//! read off a stored approximation; the cached rows only seed the solve and bound searches.

use crate::curve::{Circle, CurveDeriv, CurveGeom, Domain1, Line, ProcCurve, ProceduralCurve};
use crate::surface::{Domain2, Proc, Procedural, SurfDeriv, Surface, SurfaceGeom};
use forge_math::{Box2, Box3, Frame, Interval, Point2, Point3, UnitVec3, Vec3, dmath, tol};
use std::ops::{Add, Mul, Neg, Sub};
use std::sync::Arc;

/// Taylor coefficients of a function of s: f, f', f''/2, f'''/6, ...
#[derive(Clone, Copy, Debug, PartialEq)]
struct Jn<const N: usize>([f64; N]);

type J = Jn<4>;

impl<const N: usize> Default for Jn<N> {
    fn default() -> Self {
        Jn([0.0; N])
    }
}

impl<const N: usize> Jn<N> {
    fn k(v: f64) -> Self {
        let mut a = [0.0; N];
        a[0] = v;
        Jn(a)
    }
    /// The variable s itself, at s.
    fn var(s: f64) -> Self {
        let mut a = [0.0; N];
        a[0] = s;
        a[1] = 1.0;
        Jn(a)
    }
    fn d(self) -> Self {
        Jn(std::array::from_fn(|k| {
            if k + 1 < N {
                (k + 1) as f64 * self.0[k + 1]
            } else {
                0.0
            }
        }))
    }
    fn integral(self, c: f64) -> Self {
        Jn(std::array::from_fn(|k| {
            if k == 0 { c } else { self.0[k - 1] / k as f64 }
        }))
    }
    fn sqrt(self) -> Self {
        let a = self.0;
        let mut y = [0.0; N];
        y[0] = a[0].max(0.0).sqrt();
        let t = 0.5 / y[0];
        for k in 1..N {
            let mut acc = a[k];
            for i in 1..k {
                acc -= y[i] * y[k - i];
            }
            y[k] = acc * t;
        }
        Jn(y)
    }
    fn recip(self) -> Self {
        let b = self.0;
        let mut q = [0.0; N];
        q[0] = 1.0 / b[0];
        for k in 1..N {
            let mut acc = 0.0;
            for i in 1..=k {
                acc += b[i] * q[k - i];
            }
            q[k] = -acc * q[0];
        }
        Jn(q)
    }
    fn sin_cos(self) -> (Self, Self) {
        let a = self.0;
        let (mut s, mut c) = ([0.0; N], [0.0; N]);
        (s[0], c[0]) = dmath::sin_cos(a[0]);
        // k s_k = sum j a_j c_(k-j), k c_k = -sum j a_j s_(k-j)
        for k in 1..N {
            let (mut ss, mut cc) = (0.0, 0.0);
            for j in 1..=k {
                ss += j as f64 * a[j] * c[k - j];
                cc -= j as f64 * a[j] * s[k - j];
            }
            s[k] = ss / k as f64;
            c[k] = cc / k as f64;
        }
        (Jn(s), Jn(c))
    }
    fn atan2(y: Self, x: Self) -> Self {
        let w = (x * y.d() - y * x.d()) * (x * x + y * y).recip();
        w.integral(dmath::atan2(y.0[0], x.0[0]))
    }
}

impl<const N: usize> Add for Jn<N> {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Jn(std::array::from_fn(|k| self.0[k] + o.0[k]))
    }
}

impl<const N: usize> Sub for Jn<N> {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Jn(std::array::from_fn(|k| self.0[k] - o.0[k]))
    }
}

impl<const N: usize> Neg for Jn<N> {
    type Output = Self;
    fn neg(self) -> Self {
        self * -1.0
    }
}

impl<const N: usize> Mul for Jn<N> {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        let (a, b) = (self.0, o.0);
        Jn(std::array::from_fn(|k| (0..=k).map(|i| a[i] * b[k - i]).sum()))
    }
}

impl<const N: usize> Mul<f64> for Jn<N> {
    type Output = Self;
    fn mul(self, s: f64) -> Self {
        Jn(self.0.map(|x| x * s))
    }
}

/// A vector of jets.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Vn<const N: usize> {
    x: Jn<N>,
    y: Jn<N>,
    z: Jn<N>,
}

type V = Vn<4>;

impl<const N: usize> Default for Vn<N> {
    fn default() -> Self {
        Vn {
            x: Jn::default(),
            y: Jn::default(),
            z: Jn::default(),
        }
    }
}

impl<const N: usize> Vn<N> {
    fn k(v: Vec3) -> Self {
        Vn {
            x: Jn::k(v.x),
            y: Jn::k(v.y),
            z: Jn::k(v.z),
        }
    }
    /// The constant direction `v` scaled by the jet `j`.
    fn along(v: Vec3, j: Jn<N>) -> Self {
        Vn {
            x: j * v.x,
            y: j * v.y,
            z: j * v.z,
        }
    }
    fn from(c: [Vec3; N]) -> Self {
        Vn {
            x: Jn(c.map(|v| v.x)),
            y: Jn(c.map(|v| v.y)),
            z: Jn(c.map(|v| v.z)),
        }
    }
    fn at(&self, i: usize) -> Vec3 {
        Vec3::new(self.x.0[i], self.y.0[i], self.z.0[i])
    }
    fn set(&mut self, i: usize, v: Vec3) {
        self.x.0[i] = v.x;
        self.y.0[i] = v.y;
        self.z.0[i] = v.z;
    }
    fn coefs(&self) -> [Vec3; N] {
        std::array::from_fn(|i| self.at(i))
    }
    fn d(self) -> Self {
        Vn {
            x: self.x.d(),
            y: self.y.d(),
            z: self.z.d(),
        }
    }
    fn dot(self, o: Self) -> Jn<N> {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    fn dotc(self, v: Vec3) -> Jn<N> {
        self.x * v.x + self.y * v.y + self.z * v.z
    }
    fn norm(self) -> Jn<N> {
        self.dot(self).sqrt()
    }
}

impl<const N: usize> Add for Vn<N> {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Vn {
            x: self.x + o.x,
            y: self.y + o.y,
            z: self.z + o.z,
        }
    }
}

impl<const N: usize> Sub for Vn<N> {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Vn {
            x: self.x - o.x,
            y: self.y - o.y,
            z: self.z - o.z,
        }
    }
}

impl<const N: usize> Mul<Jn<N>> for Vn<N> {
    type Output = Self;
    fn mul(self, s: Jn<N>) -> Self {
        Vn {
            x: self.x * s,
            y: self.y * s,
            z: self.z * s,
        }
    }
}

impl<const N: usize> Mul<f64> for Vn<N> {
    type Output = Self;
    fn mul(self, s: f64) -> Self {
        Vn {
            x: self.x * s,
            y: self.y * s,
            z: self.z * s,
        }
    }
}

/// Signed distance to an analytic surface (its `implicit`, which is the true distance near
/// it) and, when asked, the distance's gradient, as jets of the point's jet.
fn dist_grad<const N: usize>(s: &Surface, x: Vn<N>, want_grad: bool) -> (Jn<N>, Vn<N>) {
    let local = |f: &Frame| {
        let d = x - Vn::k(f.origin.to_vec());
        (d.dotc(f.x().get()), d.dotc(f.y().get()), d.dotc(f.z().get()))
    };
    let radial = |f: &Frame, lx: Jn<N>, ly: Jn<N>| {
        let rho = (lx * lx + ly * ly).sqrt();
        if !want_grad {
            return (rho, Vn::default());
        }
        let inv = rho.recip();
        (
            rho,
            Vn::along(f.x().get(), lx * inv) + Vn::along(f.y().get(), ly * inv),
        )
    };
    match s {
        Surface::Plane(p) => (local(&p.frame).2, Vn::k(p.frame.z().get())),
        Surface::Cylinder(c) => {
            let (lx, ly, _) = local(&c.frame);
            let (rho, rad) = radial(&c.frame, lx, ly);
            (rho - Jn::k(c.radius), rad)
        }
        Surface::Cone(c) => {
            let (lx, ly, lz) = local(&c.frame);
            let (rho, rad) = radial(&c.frame, lx, ly);
            let (sa, ca) = (c.sin_a(), c.cos_a());
            let d = rho * ca - lz * sa - Jn::k(c.radius * ca);
            (d, rad * Jn::k(ca) - Vn::k(c.frame.z().get() * sa))
        }
        Surface::Sphere(sp) => {
            let d = x - Vn::k(sp.frame.origin.to_vec());
            let l = d.norm();
            let g = if want_grad { d * l.recip() } else { Vn::default() };
            (l - Jn::k(sp.radius), g)
        }
        Surface::Torus(t) => {
            let (lx, ly, lz) = local(&t.frame);
            let (rho, rad) = radial(&t.frame, lx, ly);
            let a = rho - Jn::k(t.major);
            let l = (a * a + lz * lz).sqrt();
            if !want_grad {
                return (l - Jn::k(t.minor), Vn::default());
            }
            let inv = l.recip();
            (
                l - Jn::k(t.minor),
                rad * (a * inv) + Vn::along(t.frame.z().get(), lz * inv),
            )
        }
        Surface::Procedural(_) => unreachable!("blend surfaces are checked to be analytic"),
    }
}

/// Solves the rows `a`, `b`, `c` times x = r; None when the rows are nearly dependent.
fn solve3(a: Vec3, b: Vec3, c: Vec3, r: [f64; 3]) -> Option<Vec3> {
    let (bc, ca, ab) = (b.cross(c), c.cross(a), a.cross(b));
    let det = a.dot(bc);
    if det.is_nan() || det.abs() <= 1e-12 * a.length() * b.length() * c.length() {
        return None;
    }
    Some((bc * r[0] + ca * r[1] + ab * r[2]) / det)
}

/// A C2 piecewise cubic through (x, y) points: natural ends, or the given end slopes.
/// Beyond the ends it carries on along the end tangent.
#[derive(Clone, Debug, PartialEq)]
struct Pieces {
    knots: Vec<f64>,
    /// Per segment: y = a + b h + c h^2 + d h^3, h measured from the segment's first knot.
    coef: Vec<[f64; 4]>,
}

impl Pieces {
    fn fit(x: &[f64], y: &[f64], ends: Option<(f64, f64)>) -> Pieces {
        let n = x.len() - 1;
        let h: Vec<f64> = (0..n).map(|i| x[i + 1] - x[i]).collect();
        let slope: Vec<f64> = (0..n).map(|i| (y[i + 1] - y[i]) / h[i]).collect();
        // tridiagonal system for the second derivatives m
        let (mut lo, mut di, mut up, mut rhs) = (
            vec![0.0; n + 1],
            vec![1.0; n + 1],
            vec![0.0; n + 1],
            vec![0.0; n + 1],
        );
        for i in 1..n {
            lo[i] = h[i - 1];
            di[i] = 2.0 * (h[i - 1] + h[i]);
            up[i] = h[i];
            rhs[i] = 6.0 * (slope[i] - slope[i - 1]);
        }
        if let Some((s0, s1)) = ends {
            di[0] = 2.0 * h[0];
            up[0] = h[0];
            rhs[0] = 6.0 * (slope[0] - s0);
            lo[n] = h[n - 1];
            di[n] = 2.0 * h[n - 1];
            rhs[n] = 6.0 * (s1 - slope[n - 1]);
        }
        for i in 1..=n {
            let w = lo[i] / di[i - 1];
            di[i] -= w * up[i - 1];
            rhs[i] -= w * rhs[i - 1];
        }
        let mut m = vec![0.0; n + 1];
        m[n] = rhs[n] / di[n];
        for i in (0..n).rev() {
            m[i] = (rhs[i] - up[i] * m[i + 1]) / di[i];
        }
        let coef = (0..n)
            .map(|i| {
                [
                    y[i],
                    slope[i] - h[i] * (2.0 * m[i] + m[i + 1]) / 6.0,
                    m[i] * 0.5,
                    (m[i + 1] - m[i]) / (6.0 * h[i]),
                ]
            })
            .collect();
        Pieces {
            knots: x.to_vec(),
            coef,
        }
    }

    /// Segment index and offset from its first knot (negative before the start, past its
    /// width after the end).
    fn seg(&self, s: f64) -> (usize, f64) {
        let n = self.coef.len();
        let i = self.knots.partition_point(|&k| k <= s).clamp(1, n) - 1;
        (i, s - self.knots[i])
    }

    /// Taylor coefficients at s.
    fn jet(&self, s: f64) -> [f64; 4] {
        let n = self.coef.len();
        let (i, h) = self.seg(s);
        let [a, b, c, d] = self.coef[i];
        let at = |h: f64| {
            [
                a + h * (b + h * (c + h * d)),
                b + h * (2.0 * c + 3.0 * h * d),
                c + 3.0 * h * d,
                d,
            ]
        };
        if h < 0.0 && i == 0 {
            [a + b * h, b, 0.0, 0.0]
        } else if i == n - 1 && s > self.knots[n] {
            let w = self.knots[n] - self.knots[n - 1];
            let e = at(w);
            let t = s - self.knots[n];
            [e[0] + e[1] * t, e[1], 0.0, 0.0]
        } else {
            at(h)
        }
    }

    /// Smallest and largest value over [a, b].
    fn bounds(&self, a: f64, b: f64) -> (f64, f64) {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut take = |s: f64| {
            let v = self.jet(s)[0];
            lo = lo.min(v);
            hi = hi.max(v);
        };
        take(a);
        take(b);
        for (i, &[_, c1, c2, c3]) in self.coef.iter().enumerate() {
            let (k0, k1) = (self.knots[i], self.knots[i + 1]);
            if k1 < a || k0 > b {
                continue;
            }
            if k0 > a {
                take(k0);
            }
            // roots of c1 + 2 c2 h + 3 c3 h^2
            let (qa, qb, qc) = (3.0 * c3, 2.0 * c2, c1);
            let roots = if qa.abs() < 1e-300 {
                if qb.abs() < 1e-300 { vec![] } else { vec![-qc / qb] }
            } else {
                let disc = qb * qb - 4.0 * qa * qc;
                if disc < 0.0 {
                    vec![]
                } else {
                    let sq = disc.sqrt();
                    vec![(-qb - sq) / (2.0 * qa), (-qb + sq) / (2.0 * qa)]
                }
            };
            for h in roots {
                let s = k0 + h;
                if s > a.max(k0) && s < b.min(k1) {
                    take(s);
                }
            }
        }
        (lo, hi)
    }
}

/// How the radius varies along the edge: a function of the guide parameter s. It is design
/// data (which ball sits where), so a cubic spline is a fine way to state it. Beyond its first and
/// last control points it continues along the end tangent.
#[derive(Clone, Debug, PartialEq)]
pub struct RadiusLaw {
    p: Pieces,
}

impl RadiusLaw {
    pub fn constant(r: f64) -> RadiusLaw {
        RadiusLaw::linear(0.0, r, 1.0, r)
    }
    /// r0 at s0 to r1 at s1, linear in s.
    pub fn linear(s0: f64, r0: f64, s1: f64, r1: f64) -> RadiusLaw {
        RadiusLaw {
            p: Pieces::fit(&[s0, s1], &[r0, r1], None),
        }
    }
    /// The C2 cubic spline through (s, r) control points (s increasing): natural ends (r'' = 0)
    /// unless end slopes dr/ds are given. Two points with natural ends give the linear law.
    pub fn smooth(points: &[(f64, f64)], end_slopes: Option<(f64, f64)>) -> Option<RadiusLaw> {
        if points.len() < 2
            || points
                .windows(2)
                .any(|w| w[1].0.partial_cmp(&w[0].0) != Some(std::cmp::Ordering::Greater))
        {
            return None;
        }
        let (x, y): (Vec<f64>, Vec<f64>) = points.iter().copied().unzip();
        Some(RadiusLaw {
            p: Pieces::fit(&x, &y, end_slopes),
        })
    }
    pub fn eval(&self, s: f64) -> f64 {
        self.p.jet(s)[0]
    }
    /// dr/ds.
    pub fn slope(&self, s: f64) -> f64 {
        self.p.jet(s)[1]
    }
    fn jet(&self, s: f64) -> J {
        Jn(self.p.jet(s))
    }
    /// Smallest and largest radius over [a, b].
    pub fn bounds(&self, a: f64, b: f64) -> (f64, f64) {
        self.p.bounds(a, b)
    }
}

/// The curve that fixes the parameter s: the ball centre for s lies in the plane through g(s)
/// perpendicular to g'(s). The guide is design data, like the radius law: it decides which ball
/// sits where and is never blend geometry. Whatever the guide, the blend touches both faces
/// exactly at r(s). Use the edge itself (Line, Circle) wherever it is one. The Hermite guide is
/// for edges with no closed form and for tangent chains, where sections must turn smoothly
/// across junctions so that neighbouring blends share their junction circles.
#[derive(Clone, Debug, PartialEq)]
pub enum Guide {
    Line(Line),
    /// s is the circle's angle.
    Circle(Circle),
    /// C4 degree-9 Hermite through exact edge points, matching the edges' derivatives by arc
    /// length up to the fourth there (so blends are curvature-continuous across its knots); s
    /// is the cumulative chord length between knots.
    Hermite(Arc<Hermite>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hermite {
    knots: Vec<f64>,
    /// Per segment: g = sum c[k] h^k, h measured from the segment's first knot.
    coef: Vec<[Vec3; 10]>,
}

/// Taylor coefficients by arc length (point, unit tangent, ...) of an edge at its parameter t,
/// running the way t increases. Exact: closed forms, or for an intersection edge solved order by
/// order from the two surfaces.
fn edge_taylor(c: &crate::Curve, t: f64) -> Option<[Vec3; 5]> {
    match c {
        crate::Curve::Line(l) => Some([
            l.eval(t).to_vec(),
            l.dir.get(),
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::ZERO,
        ]),
        crate::Curve::Circle(ci) => {
            let r = ci.radius;
            let mut th = Jn::<5>::var(t);
            th.0[1] = 1.0 / r;
            let (sn, cs) = th.sin_cos();
            let (x, y) = (ci.frame.x().get(), ci.frame.y().get());
            let e = Vn::k(ci.frame.origin.to_vec()) + Vn::along(x, cs * r) + Vn::along(y, sn * r);
            Some(e.coefs())
        }
        crate::Curve::Intersection(ic) => {
            if matches!(ic.a, Surface::Procedural(_)) || matches!(ic.b, Surface::Procedural(_)) {
                return None;
            }
            let p = ic.eval(t);
            let along = CurveGeom::deriv(&**ic, t).d1;
            let tan = crate::intcurve::direction(&ic.a, &ic.b, p)?;
            let tan = if tan.dot(along) < 0.0 { -tan } else { tan };
            let (na, nb) = (ic.a.gradient(p), ic.b.gradient(p));
            let mut e = Vn::<5>::k(p.to_vec());
            e.set(1, tan);
            for k in 2..5 {
                let fa = dist_grad(&ic.a, e, false).0.0[k];
                let fb = dist_grad(&ic.b, e, false).0.0[k];
                // unit speed: coefficient k-1 of e'.e' vanishes; e_k enters it as 2 k tan.e_k
                let ep = e.d();
                let w = ep.dot(ep).0[k - 1];
                e.set(k, solve3(na, nb, tan, [-fa, -fb, -w / (2.0 * k as f64)])?);
            }
            Some(e.coefs())
        }
        _ => None,
    }
}

impl Hermite {
    /// Through knots given as arc-length Taylor coefficients; s is the cumulative chord length.
    fn new(data: &[[Vec3; 5]]) -> Option<Hermite> {
        if data.len() < 2 {
            return None;
        }
        // y_j = c_j h^j; the right end's conditions sum_j y_j C(j, k) = T1_k h^k, k = 0..4,
        // fix y_5..y_9 through the constant matrix C(5 + i, k)
        let inv = invert5(std::array::from_fn(|k| std::array::from_fn(|i| binom(5 + i, k))))?;
        let mut knots = vec![0.0];
        let mut coef = Vec::with_capacity(data.len() - 1);
        for w in data.windows(2) {
            let (t0, t1) = (&w[0], &w[1]);
            let h = (t1[0] - t0[0]).length();
            if h <= tol::LINEAR {
                return None;
            }
            knots.push(knots.last().copied().unwrap_or(0.0) + h);
            let mut y = [Vec3::ZERO; 10];
            let mut hk = 1.0;
            for k in 0..5 {
                y[k] = t0[k] * hk;
                hk *= h;
            }
            let mut rhs = [Vec3::ZERO; 5];
            let mut hk = 1.0;
            for k in 0..5 {
                rhs[k] = t1[k] * hk;
                for (j, yj) in y.iter().enumerate().take(5) {
                    rhs[k] -= *yj * binom(j, k);
                }
                hk *= h;
            }
            for i in 0..5 {
                y[5 + i] = (0..5).fold(Vec3::ZERO, |acc, k| acc + rhs[k] * inv[i][k]);
            }
            let mut c = [Vec3::ZERO; 10];
            let mut hj = 1.0;
            for j in 0..10 {
                c[j] = y[j] / hj;
                hj *= h;
            }
            coef.push(c);
        }
        Some(Hermite { knots, coef })
    }

    /// Taylor coefficients (orders 0..3) of g and g' at s; straight along the end tangents
    /// beyond the ends.
    fn jets(&self, s: f64) -> ([Vec3; 4], [Vec3; 4]) {
        let n = self.coef.len();
        let i = self.knots.partition_point(|&k| k <= s).clamp(1, n) - 1;
        let (k0, k1) = (self.knots[i], self.knots[i + 1]);
        let at = |c: &[Vec3; 10], h: f64| -> [Vec3; 5] {
            std::array::from_fn(|k| {
                let mut acc = Vec3::ZERO;
                for j in (k..10).rev() {
                    acc = acc * h + c[j] * binom(j, k);
                }
                acc
            })
        };
        let straight = |h: f64, t: f64| {
            let e = at(&self.coef[i], h);
            (
                [e[0] + e[1] * t, e[1], Vec3::ZERO, Vec3::ZERO],
                [e[1], Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
            )
        };
        if s < k0 && i == 0 {
            return straight(0.0, s - k0);
        }
        if s > k1 && i == n - 1 {
            return straight(k1 - k0, s - k1);
        }
        let e = at(&self.coef[i], s - k0);
        (
            [e[0], e[1], e[2], e[3]],
            [e[1], e[2] * 2.0, e[3] * 3.0, e[4] * 4.0],
        )
    }
}

fn binom(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    (1..=k).fold(1.0, |acc, i| acc * (n + 1 - i) as f64 / i as f64)
}

/// Inverse of a 5x5 matrix by Gauss-Jordan with partial pivoting.
fn invert5(m: [[f64; 5]; 5]) -> Option<[[f64; 5]; 5]> {
    let mut a = m;
    let mut inv: [[f64; 5]; 5] =
        std::array::from_fn(|i| std::array::from_fn(|j| if i == j { 1.0 } else { 0.0 }));
    for col in 0..5 {
        let piv = (col..5).max_by(|&x, &y| a[x][col].abs().total_cmp(&a[y][col].abs()))?;
        if a[piv][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, piv);
        inv.swap(col, piv);
        let d = a[col][col];
        for j in 0..5 {
            a[col][j] /= d;
            inv[col][j] /= d;
        }
        for row in 0..5 {
            if row != col {
                let f = a[row][col];
                for j in 0..5 {
                    a[row][j] -= f * a[col][j];
                    inv[row][j] -= f * inv[col][j];
                }
            }
        }
    }
    Some(inv)
}

impl Guide {
    /// The Hermite guide through a chain of edges (lines, circles, intersection curves), each
    /// split into `per_edge` equal steps of its own parameter. Knots take each edge's exact
    /// point and arc-length derivatives; where two edges meet, the tangents must agree (a
    /// tangent chain) and the higher derivatives are averaged. Deterministic: knots depend only
    /// on the edges and their ranges. Returns the guide and each edge's s range.
    pub fn through_edges(
        edges: &[(crate::Curve, Interval)],
        per_edge: usize,
    ) -> Option<(Guide, Vec<Interval>)> {
        let per = per_edge.max(1);
        let mut data: Vec<[Vec3; 5]> = Vec::new();
        let mut ends = Vec::new();
        for (c, r) in edges {
            let first = data.len().saturating_sub(1);
            for k in 0..=per {
                let e = edge_taylor(c, r.lo + r.width() * k as f64 / per as f64)?;
                match data.last_mut() {
                    Some(last) if k == 0 => {
                        if (last[0] - e[0]).length() > tol::LINEAR || last[1].dot(e[1]) < 1.0 - 1e-9 {
                            return None;
                        }
                        for j in 2..5 {
                            last[j] = (last[j] + e[j]) * 0.5;
                        }
                    }
                    _ => data.push(e),
                }
            }
            ends.push((first, data.len() - 1));
        }
        let h = Hermite::new(&data)?;
        let ranges = ends
            .iter()
            .map(|&(a, b)| Interval::new(h.knots[a], h.knots[b]))
            .collect();
        Some((Guide::Hermite(Arc::new(h)), ranges))
    }

    /// Point and derivative at s.
    pub fn point_tan(&self, s: f64) -> (Point3, Vec3) {
        match self {
            Guide::Line(l) => (l.origin + l.dir * s, l.dir.get()),
            Guide::Circle(c) => {
                let d = c.deriv(s);
                (d.p, d.d1)
            }
            Guide::Hermite(h) => {
                let (g, gp) = h.jets(s);
                (Point3::ORIGIN + g[0], gp[0])
            }
        }
    }

    /// Jets of g and g' at s.
    fn jets(&self, s: f64) -> (V, V) {
        match self {
            Guide::Line(l) => {
                let d = l.dir.get();
                (V::from([l.origin.to_vec(), d, Vec3::ZERO, Vec3::ZERO]), V::k(d))
            }
            Guide::Circle(c) => {
                let (sn, cs) = J::var(s).sin_cos();
                let (x, y) = (c.frame.x().get(), c.frame.y().get());
                let r = c.radius;
                (
                    V::k(c.frame.origin.to_vec()) + V::along(x, cs * r) + V::along(y, sn * r),
                    V::along(x, sn * -r) + V::along(y, cs * r),
                )
            }
            Guide::Hermite(h) => {
                let (g, gp) = h.jets(s);
                (V::from(g), V::from(gp))
            }
        }
    }

    pub fn transformed(&self, f: &Frame) -> Guide {
        match self {
            Guide::Line(l) => Guide::Line(l.transformed(f)),
            Guide::Circle(c) => Guide::Circle(c.transformed(f)),
            Guide::Hermite(h) => Guide::Hermite(Arc::new(Hermite {
                knots: h.knots.clone(),
                coef: h
                    .coef
                    .iter()
                    .map(|c| {
                        let mut o = c.map(|v| f.vec_to_world(v));
                        o[0] = f.point_to_world(Point3::ORIGIN + c[0]).to_vec();
                        o
                    })
                    .collect(),
            })),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VarBlendError {
    /// Blends are defined between analytic surfaces (their implicits are true distances).
    NotAnalytic,
    BadRange,
    /// The radius must stay positive over the range.
    BadRadius,
    /// No ball touching both surfaces at the start of the range.
    NoStart,
    /// The surfaces are tangent along the ball's plane, so the centre is not fixed.
    Tangent {
        s: f64,
    },
    /// |r'| reaches |c'|: the radius changes as fast as the centre moves, and the balls no
    /// longer have an envelope.
    RadiusTooSteep {
        s: f64,
    },
    /// The two contact points coincide or sit opposite on the ball.
    Contacts {
        s: f64,
    },
    /// The centre could not be followed along s (it jumps or the solve fails).
    Lost {
        s: f64,
    },
}

impl std::fmt::Display for VarBlendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VarBlendError::NotAnalytic => write!(f, "variable blend needs two analytic surfaces"),
            VarBlendError::BadRange => write!(f, "variable blend range is empty"),
            VarBlendError::BadRadius => write!(f, "variable blend radius is not positive over the range"),
            VarBlendError::NoStart => write!(f, "no ball touches both surfaces at the start"),
            VarBlendError::Tangent { s } => write!(f, "surfaces tangent at s = {s}"),
            VarBlendError::RadiusTooSteep { s } => {
                write!(f, "radius changes as fast as the ball centre moves at s = {s}")
            }
            VarBlendError::Contacts { s } => write!(f, "blend contacts coincide or are opposite at s = {s}"),
            VarBlendError::Lost { s } => write!(f, "ball centre lost at s = {s}"),
        }
    }
}

impl std::error::Error for VarBlendError {}

/// One cross-section, as jets in s.
struct Section {
    c: V,
    k: V,
    rho: J,
    e1: V,
    e2: V,
    phi: J,
    /// Contacts at v = 0 and v = 1.
    q: [V; 2],
}

/// A solved cross-section, cached to seed solves and bound searches.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Row {
    s: f64,
    /// Taylor coefficients of the centre (the first as a vector from the origin).
    c: [Vec3; 4],
    r: f64,
    k: Point3,
    rho: f64,
    e1: Vec3,
    e2: Vec3,
    phi: f64,
    /// Points at v = 0, 1/2, 1.
    pts: [Point3; 3],
    /// Largest |B_u| and |B_v| seen over v, and the largest principal curvature.
    bu: f64,
    bv: f64,
    kmax: f64,
    /// |q'| and |q''| of the two spring curves.
    q1: [f64; 2],
    q2: [f64; 2],
}

/// Bounds over the stretch between two rows.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Seg {
    /// Bounds on |c'|, |B_u|, |q'| and |q''|.
    vc: f64,
    vb: f64,
    vq: [f64; 2],
    aq: [f64; 2],
    rmin: f64,
    rmax: f64,
}

/// A variable-radius rolling-ball blend between two analytic surfaces (see the module docs).
#[derive(Clone, Debug)]
pub struct VarBlend {
    a: Surface,
    b: Surface,
    side: [f64; 2],
    guide: Guide,
    law: Arc<RadiusLaw>,
    lo: f64,
    hi: f64,
    /// The contact with b is at v = 0, so du x dv points out of the balls.
    swap: bool,
    rows: Vec<Row>,
    segs: Vec<Seg>,
}

/// Distance at which a spine step is checked against its prediction (mm).
const PREDICT: f64 = 1e-6;

impl VarBlend {
    /// The blend between `a` and `b`, the balls on the `side_a` (+1 or -1, the sign of a's
    /// implicit) of a and the `side_b` of b, for s over `range` of the guide.
    pub fn new(
        a: &Surface,
        side_a: f64,
        b: &Surface,
        side_b: f64,
        guide: Guide,
        law: Arc<RadiusLaw>,
        range: Interval,
    ) -> Result<Arc<VarBlend>, VarBlendError> {
        if matches!(a, Surface::Procedural(_)) || matches!(b, Surface::Procedural(_)) {
            return Err(VarBlendError::NotAnalytic);
        }
        if range.width().is_nan() || range.width() <= 0.0 || !range.lo.is_finite() || !range.hi.is_finite() {
            return Err(VarBlendError::BadRange);
        }
        let rmin = law.bounds(range.lo, range.hi).0;
        if rmin.is_nan() || rmin <= tol::MIN_LENGTH {
            return Err(VarBlendError::BadRadius);
        }
        let mut vb = VarBlend {
            a: a.clone(),
            b: b.clone(),
            side: [side_a.signum(), side_b.signum()],
            guide,
            law,
            lo: range.lo,
            hi: range.hi,
            swap: false,
            rows: Vec::new(),
            segs: Vec::new(),
        };
        vb.build()?;
        Ok(Arc::new(vb))
    }

    pub fn surface(self: &Arc<Self>) -> Surface {
        Surface::Procedural(Proc(self.clone()))
    }

    /// v of the contact with surface `i` (0 for a, 1 for b).
    pub fn v_on(&self, i: usize) -> f64 {
        if (i == 1) != self.swap { 1.0 } else { 0.0 }
    }

    /// The spring curve where the blend touches surface `i` (0 for a, 1 for b); its parameter is s.
    pub fn spring(self: &Arc<Self>, i: usize) -> crate::Curve {
        crate::Curve::Procedural(ProcCurve(Arc::new(self.spring_curve(i))))
    }

    /// The spring curve on surface `i` as its concrete type.
    pub fn spring_curve(self: &Arc<Self>, i: usize) -> Spring {
        Spring {
            blend: self.clone(),
            v: self.v_on(i) as usize,
        }
    }

    /// The characteristic circle at s (exact) and the arc's angle: the blend's cross-section
    /// runs from angle 0 (v = 0) to that angle (v = 1).
    pub fn section_circle(&self, s: f64) -> (Circle, f64) {
        let sec = self.section_at(s);
        let (e1, e2) = (sec.e1.at(0), sec.e2.at(0));
        let z = e1.cross(e2).normalize().unwrap_or(UnitVec3::Z);
        let frame = Frame::from_zx(Point3::ORIGIN + sec.k.at(0), z, e1)
            .unwrap_or_else(|| Frame::from_z(Point3::ORIGIN + sec.k.at(0), z));
        (
            Circle {
                frame,
                radius: sec.rho.0[0],
            },
            sec.phi.0[0],
        )
    }

    pub fn range(&self) -> Interval {
        Interval::new(self.lo, self.hi)
    }

    pub fn radius(&self, s: f64) -> f64 {
        self.law.eval(s)
    }

    /// The ball centre at s.
    pub fn centre(&self, s: f64) -> Point3 {
        Point3::ORIGIN + self.section_at(s).c.at(0)
    }

    /// Newton for the centre at s with radius r: on both offsets and in the guide's plane.
    fn solve_centre(&self, s: f64, r: f64, seed: Point3) -> Option<Point3> {
        let (g, t) = self.guide.point_tan(s);
        let t = t.normalize()?.get();
        let res = |c: Point3| {
            [
                self.a.implicit(c) - self.side[0] * r,
                self.b.implicit(c) - self.side[1] * r,
                (c - g).dot(t),
            ]
        };
        let norm = |f: [f64; 3]| f[0].abs().max(f[1].abs()).max(f[2].abs());
        // NaN never counts as a decrease
        let better = |n: [f64; 3], o: [f64; 3]| norm(n) <= norm(o) && !n.iter().any(|x| x.is_nan());
        let scale = 1.0 + r + seed.to_vec().abs_max();
        let mut c = seed;
        let mut f = res(c);
        for _ in 0..40 {
            let dx = solve3(self.a.gradient(c), self.b.gradient(c), t, [-f[0], -f[1], -f[2]])?;
            // backtracking keeps every step a decrease, so the iteration cannot run away
            let mut lam = 1.0;
            let (mut cn, mut fnew) = (c + dx, res(c + dx));
            while !better(fnew, f) && lam > 1.0 / 64.0 && dx.length() > 1e-9 * scale {
                lam *= 0.5;
                cn = c + dx * lam;
                fnew = res(cn);
            }
            if !better(fnew, f) && dx.length() > 1e-9 * scale {
                return None;
            }
            (c, f) = (cn, fnew);
            if dx.length() * lam <= 1e-13 * scale {
                return Some(c);
            }
        }
        (norm(f) <= 1e-9 * scale).then_some(c)
    }

    /// The centre's Taylor coefficients at s from the centre c0, solved order by order.
    fn spine(&self, s: f64, c0: Point3) -> Option<(V, J)> {
        let r = self.law.jet(s);
        let (g, gp) = self.guide.jets(s);
        let (na, nb, t) = (self.a.gradient(c0), self.b.gradient(c0), gp.at(0));
        let mut c = V::k(c0.to_vec());
        for i in 1..4 {
            let fa = dist_grad(&self.a, c, false).0 - r * self.side[0];
            let fb = dist_grad(&self.b, c, false).0 - r * self.side[1];
            let fp = (c - g).dot(gp);
            c.set(i, solve3(na, nb, t, [-fa.0[i], -fb.0[i], -fp.0[i]])?);
        }
        Some((c, r))
    }

    fn section(&self, c: V, r: J) -> Section {
        let cp = c.d();
        let rp = r.d();
        let g = cp.dot(cp).recip();
        let k = c - cp * (r * rp * g);
        let rho = r * (J::k(1.0) - rp * rp * g).sqrt();
        let qa = c - dist_grad(&self.a, c, true).1 * (r * self.side[0]);
        let qb = c - dist_grad(&self.b, c, true).1 * (r * self.side[1]);
        let q = if self.swap { [qb, qa] } else { [qa, qb] };
        let a = q[0] - k;
        let e1 = a * a.norm().recip();
        let b = q[1] - k;
        let bp = b - e1 * b.dot(e1);
        let e2 = bp * bp.norm().recip();
        let phi = J::atan2(b.dot(e2), b.dot(e1));
        Section {
            c,
            k,
            rho,
            e1,
            e2,
            phi,
            q,
        }
    }

    fn point(sec: &Section, v: f64) -> SurfDeriv {
        let (sn, cs) = (sec.phi * v).sin_cos();
        let dir = sec.e1 * cs + sec.e2 * sn;
        let b = sec.k + dir * sec.rho;
        let bv = (sec.e2 * cs - sec.e1 * sn) * (sec.phi * sec.rho);
        let bvv = dir * (sec.phi * sec.phi * sec.rho);
        SurfDeriv {
            p: Point3::ORIGIN + b.at(0),
            du: b.at(1),
            dv: bv.at(0),
            duu: b.at(2) * 2.0,
            duv: bv.at(1),
            dvv: -bvv.at(0),
        }
    }

    /// Index of the row nearest s.
    fn nearest_row(&self, s: f64) -> usize {
        let i = self.rows.partition_point(|r| r.s <= s);
        if i == 0 {
            0
        } else if i == self.rows.len() || s - self.rows[i - 1].s <= self.rows[i].s - s {
            i - 1
        } else {
            i
        }
    }

    /// The cached model of the centre around row i, as Taylor coefficients at s.
    fn model(&self, i: usize, s: f64) -> [Vec3; 4] {
        let row = &self.rows[i];
        let h = s - row.s;
        let c = row.c;
        [
            c[0] + (c[1] + (c[2] + c[3] * h) * h) * h,
            c[1] + (c[2] * 2.0 + c[3] * (3.0 * h)) * h,
            c[2] + c[3] * (3.0 * h),
            c[3],
        ]
    }

    /// Centre jets at s, exact (solved), seeded from the nearest row; continuation from that
    /// row in small steps if the direct solve fails.
    fn spine_at(&self, s: f64) -> (V, J) {
        let i = self.nearest_row(s);
        let seed = Point3::ORIGIN + self.model(i, s)[0];
        if let Some(c) = self.solve_centre(s, self.law.eval(s), seed)
            && let Some(j) = self.spine(s, c)
        {
            return j;
        }
        let s0 = self.rows[i].s;
        let mut c = Point3::ORIGIN + self.rows[i].c[0];
        for k in 1..=16 {
            let t = s0 + (s - s0) * k as f64 / 16.0;
            match self.solve_centre(t, self.law.eval(t), c) {
                Some(n) => c = n,
                None => return (V::from(self.model(i, s)), self.law.jet(s)),
            }
        }
        self.spine(s, c)
            .unwrap_or_else(|| (V::from(self.model(i, s)), self.law.jet(s)))
    }

    fn section_at(&self, s: f64) -> Section {
        let (c, r) = self.spine_at(s);
        self.section(c, r)
    }

    fn check(sec: &Section, s: f64) -> Result<(), VarBlendError> {
        let (rho, phi) = (sec.rho.0[0], sec.phi.0[0]);
        if rho.is_nan() || rho <= tol::LINEAR {
            return Err(VarBlendError::RadiusTooSteep { s });
        }
        let along = (sec.q[1] - sec.k).dot(sec.e2).0[0];
        if (phi * rho).is_nan() || phi * rho <= tol::LINEAR || along.is_nan() || along <= tol::LINEAR {
            return Err(VarBlendError::Contacts { s });
        }
        Ok(())
    }

    fn make_row(&self, s: f64, c: Point3) -> Result<Row, VarBlendError> {
        let (cj, rj) = self.spine(s, c).ok_or(VarBlendError::Tangent { s })?;
        let sec = self.section(cj, rj);
        Self::check(&sec, s)?;
        let mut pts = [Point3::ORIGIN; 3];
        let (mut bu, mut bv, mut kmax) = (0.0f64, 0.0f64, 0.0f64);
        for (j, v) in [0.0, 0.25, 0.5, 0.75, 1.0].into_iter().enumerate() {
            let d = Self::point(&sec, v);
            if j % 2 == 0 {
                pts[j / 2] = d.p;
            }
            bu = bu.max(d.du.length());
            bv = bv.max(d.dv.length());
            kmax = kmax.max(principal_max(&d));
        }
        let sp = |q: &V| (q.at(1).length(), 2.0 * q.at(2).length());
        let (a, b) = (sp(&sec.q[0]), sp(&sec.q[1]));
        Ok(Row {
            s,
            c: cj.coefs(),
            r: rj.0[0],
            k: Point3::ORIGIN + sec.k.at(0),
            rho: sec.rho.0[0],
            e1: sec.e1.at(0),
            e2: sec.e2.at(0),
            phi: sec.phi.0[0],
            pts,
            bu,
            bv,
            kmax,
            q1: [a.0, b.0],
            q2: [a.1, b.1],
        })
    }

    fn build(&mut self) -> Result<(), VarBlendError> {
        let lo = self.lo;
        // the ball grows from nothing at the edge: radius continuation from the guide point
        let (g, _) = self.guide.point_tan(lo);
        let r = self.law.eval(lo);
        let mut c = g;
        for k in (0..=12).rev() {
            let lam = 1.0 / (1u32 << k) as f64;
            c = self.solve_centre(lo, r * lam, c).ok_or(VarBlendError::NoStart)?;
        }
        // orient: du x dv out of the ball
        let (cj, rj) = self.spine(lo, c).ok_or(VarBlendError::Tangent { s: lo })?;
        let sec = self.section(cj, rj);
        Self::check(&sec, lo)?;
        let d = Self::point(&sec, 0.5);
        if d.du.cross(d.dv).dot(d.p - (Point3::ORIGIN + cj.at(0))) < 0.0 {
            self.swap = true;
        }
        self.rows.push(self.make_row(lo, c)?);
        let span = self.hi - self.lo;
        let mut h = span / 8.0;
        while self.rows.last().is_some_and(|r| r.s < self.hi) {
            let last = *self.rows.last().expect("rows start non-empty");
            if self.rows.len() > 20_000 || h < span * 1e-10 {
                return Err(VarBlendError::Lost { s: last.s });
            }
            let mut s = last.s + h;
            if s > self.hi - 0.25 * h {
                s = self.hi;
            }
            let i = self.rows.len() - 1;
            let pred = Point3::ORIGIN + self.model(i, s)[0];
            let r = self.law.eval(s);
            let next = self.solve_centre(s, r, pred);
            let err = next.map_or(f64::INFINITY, |c| c.distance(pred));
            let moved = next.map_or(f64::INFINITY, |c| (c.to_vec() - last.c[0]).length());
            if err.is_nan() || err > PREDICT || moved > 0.5 * last.r.min(r) {
                h *= 0.5;
                continue;
            }
            let row = self.make_row(s, next.expect("checked above"))?;
            self.rows.push(row);
            if err < PREDICT / 16.0 {
                h = (h * 1.5).min(span / 4.0);
            }
        }
        self.segs = (0..self.rows.len() - 1)
            .map(|i| {
                let (r0, r1) = (&self.rows[i], &self.rows[i + 1]);
                let mid = 0.5 * (r0.s + r1.s);
                let vc = r0.c[1]
                    .length()
                    .max(r1.c[1].length())
                    .max(self.model(i, mid)[1].length())
                    .max(self.model(i + 1, mid)[1].length());
                let (rmin, rmax) = self.law.bounds(r0.s, r1.s);
                Seg {
                    vc: 1.25 * vc,
                    vb: 1.5 * r0.bu.max(r1.bu),
                    vq: [1.5 * r0.q1[0].max(r1.q1[0]), 1.5 * r0.q1[1].max(r1.q1[1])],
                    aq: [1.5 * r0.q2[0].max(r1.q2[0]), 1.5 * r0.q2[1].max(r1.q2[1])],
                    rmin,
                    rmax,
                }
            })
            .collect();
        Ok(())
    }

    /// h(s) = |p - c(s)| - r(s), its first two derivatives, from a centre jet and radius jet.
    fn tube(p: Point3, c: &[Vec3; 4], r: &[f64; 4]) -> (f64, f64, f64) {
        let d = p.to_vec() - c[0];
        let l = d.length().max(1e-300);
        let dc = d.dot(c[1]);
        let h1 = -dc / l - r[1];
        let h2 = (c[1].length_sq() - d.dot(c[2] * 2.0)) / l - dc * dc / (l * l * l) - 2.0 * r[2];
        (l - r[0], h1, h2)
    }

    /// Points in segment i where h'(s) = 0: bracketed on the cached model, then polished
    /// against the exact centre.
    fn stationary(&self, p: Point3, i: usize) -> Vec<(f64, V, J)> {
        let (s0, s1) = (self.rows[i].s, self.rows[i + 1].s);
        let hp = |s: f64| {
            let j = if s - s0 <= s1 - s { i } else { i + 1 };
            Self::tube(p, &self.model(j, s), &self.law.p.jet(s)).1
        };
        const N: usize = 4;
        let xs: Vec<f64> = (0..=N).map(|k| s0 + (s1 - s0) * k as f64 / N as f64).collect();
        let ys: Vec<f64> = xs.iter().map(|&s| hp(s)).collect();
        let mut out = Vec::new();
        for k in 0..N {
            let (mut a, mut b, mut fa, mut fb) = (xs[k], xs[k + 1], ys[k], ys[k + 1]);
            if fa == 0.0 && k > 0 {
                continue;
            }
            if fa.signum() == fb.signum() && fa != 0.0 && fb != 0.0 {
                continue;
            }
            // Illinois regula falsi: stays inside the bracket
            let mut side = 0;
            let mut s = a;
            for _ in 0..60 {
                s = if fb != fa {
                    (a * fb - b * fa) / (fb - fa)
                } else {
                    0.5 * (a + b)
                };
                if !(s > a && s < b) {
                    s = 0.5 * (a + b);
                }
                let fs = hp(s);
                if fs == 0.0 || (b - a) <= 1e-15 * (1.0 + s.abs()) {
                    break;
                }
                if fs.signum() == fb.signum() {
                    (b, fb) = (s, fs);
                    if side == 1 {
                        fa *= 0.5;
                    }
                    side = 1;
                } else {
                    (a, fa) = (s, fs);
                    if side == -1 {
                        fb *= 0.5;
                    }
                    side = -1;
                }
            }
            // exact polish: Newton on the solved centre, kept in the segment
            let mut j = self.spine_at(s);
            for _ in 0..3 {
                let (_, h1, h2) = Self::tube(p, &j.0.coefs(), &j.1.0);
                if h2.abs() < 1e-300 {
                    break;
                }
                let next = (s - h1 / h2).clamp(s0, s1);
                let done = (next - s).abs() <= 1e-14 * (1.0 + s.abs());
                if next == s {
                    break;
                }
                s = next;
                j = self.spine_at(s);
                if done {
                    break;
                }
            }
            out.push((s, j.0, j.1));
        }
        out
    }

    /// Lower bounds on |p - c(s)| from below and above, over segment i.
    fn seg_dist(&self, p: Point3, i: usize) -> (f64, f64) {
        let (r0, r1) = (&self.rows[i], &self.rows[i + 1]);
        let (d0, d1) = (
            p.distance(Point3::ORIGIN + r0.c[0]),
            p.distance(Point3::ORIGIN + r1.c[0]),
        );
        let w = self.segs[i].vc * (r1.s - r0.s);
        ((d0 + d1 - w) * 0.5, (d0 + d1 + w) * 0.5)
    }

    /// The smallest h(s) over the range, with its s and centre.
    fn tube_min(&self, p: Point3) -> (f64, f64, Point3) {
        let mut best = (f64::INFINITY, self.lo, Point3::ORIGIN);
        for r in &self.rows {
            let c = Point3::ORIGIN + r.c[0];
            let h = p.distance(c) - r.r;
            if h < best.0 {
                best = (h, r.s, c);
            }
        }
        for i in 0..self.segs.len() {
            if self.seg_dist(p, i).0 - self.segs[i].rmax >= best.0 {
                continue;
            }
            for (s, c, r) in self.stationary(p, i) {
                let cp = Point3::ORIGIN + c.at(0);
                let h = p.distance(cp) - r.0[0];
                if h < best.0 {
                    best = (h, s, cp);
                }
            }
        }
        best
    }

    /// v of the point of the section's circle nearest p (unclamped).
    fn circle_v(sec_k: Point3, e1: Vec3, e2: Vec3, phi: f64, p: Point3) -> f64 {
        let d = p - sec_k;
        dmath::atan2(d.dot(e2), d.dot(e1)) / phi
    }

    /// Parameter of the nearest point on the v = `v` spring over `range`, and its distance.
    fn spring_closest(&self, v: usize, p: Point3, range: Interval) -> (f64, f64) {
        let (lo, hi) = (
            range.lo.max(self.lo),
            range.hi.min(self.hi).max(range.lo.max(self.lo)),
        );
        let q = |s: f64| self.section_at(s).q[v];
        let mut best = (f64::INFINITY, lo);
        for s in [lo, hi] {
            let d = p.distance(Point3::ORIGIN + q(s).at(0));
            if d < best.0 {
                best = (d, s);
            }
        }
        let dists: Vec<f64> = self.rows.iter().map(|r| p.distance(r.pts[2 * v])).collect();
        for (r, &d) in self.rows.iter().zip(&dists) {
            if r.s >= lo && r.s <= hi && d < best.0 {
                best = (d, r.s);
            }
        }
        for i in 0..self.segs.len() {
            let (a, b) = (self.rows[i].s.max(lo), self.rows[i + 1].s.min(hi));
            if b <= a {
                continue;
            }
            let lb =
                (dists[i] + dists[i + 1] - self.segs[i].vq[v] * (self.rows[i + 1].s - self.rows[i].s)) * 0.5;
            if lb >= best.0 {
                continue;
            }
            // g(s) = (q - p) . q', a minimum where g goes from - to +
            let g = |s: f64| {
                let j = q(s);
                let (d, q1, q2) = (Point3::ORIGIN + j.at(0) - p, j.at(1), j.at(2) * 2.0);
                (d.dot(q1), q1.length_sq() + d.dot(q2), d.length())
            };
            let (mut a, mut b) = (a, b);
            let (ga, gb) = (g(a).0, g(b).0);
            if !(ga < 0.0 && gb > 0.0) {
                continue;
            }
            let mut s = 0.5 * (a + b);
            for _ in 0..60 {
                let (gs, dg, _) = g(s);
                if gs < 0.0 {
                    a = s;
                } else {
                    b = s;
                }
                let n = s - gs / dg;
                let next = if dg > 0.0 && n > a && n < b {
                    n
                } else {
                    0.5 * (a + b)
                };
                if (next - s).abs() <= 1e-14 * (1.0 + s.abs()) || b - a <= 1e-15 * (1.0 + s.abs()) {
                    s = next;
                    break;
                }
                s = next;
            }
            let d = g(s).2;
            if d < best.0 {
                best = (d, s);
            }
        }
        (best.1, best.0)
    }

    /// The parameters of the patch point nearest p.
    fn nearest(&self, p: Point3) -> Point2 {
        let mut best = (f64::INFINITY, Point2::new(self.lo, 0.0));
        let take = |d: f64, s: f64, v: f64, best: &mut (f64, Point2)| {
            if d < best.0 {
                *best = (d, Point2::new(s, v));
            }
        };
        for r in &self.rows {
            for (j, q) in r.pts.iter().enumerate() {
                take(p.distance(*q), r.s, 0.5 * j as f64, &mut best);
            }
        }
        // interior: stationary points of h whose circle point lies on the arc
        for i in 0..self.segs.len() {
            let (dlo, dhi) = self.seg_dist(p, i);
            let lb = (dlo - self.segs[i].rmax).max(self.segs[i].rmin - dhi);
            if lb >= best.0 {
                continue;
            }
            for (s, c, r) in self.stationary(p, i) {
                let sec = self.section(c, r);
                let v = Self::circle_v(
                    Point3::ORIGIN + sec.k.at(0),
                    sec.e1.at(0),
                    sec.e2.at(0),
                    sec.phi.0[0],
                    p,
                );
                if (0.0..=1.0).contains(&v) {
                    let d = (p.distance(Point3::ORIGIN + c.at(0)) - r.0[0]).abs();
                    take(d, s, v, &mut best);
                }
            }
        }
        // the end arcs
        for r in [&self.rows[0], self.rows.last().expect("rows are non-empty")] {
            let v = Self::circle_v(r.k, r.e1, r.e2, r.phi, p);
            if (0.0..=1.0).contains(&v) {
                let (sn, cs) = dmath::sin_cos(v * r.phi);
                let q = r.k + (r.e1 * cs + r.e2 * sn) * r.rho;
                take(p.distance(q), r.s, v, &mut best);
            }
        }
        // the springs
        for v in 0..2 {
            let (s, d) = self.spring_closest(v, p, Interval::new(self.lo, self.hi));
            take(d, s, v as f64, &mut best);
        }
        // polish: Newton on |B - p|^2 inside the domain, kept only while it gets closer
        let mut uv = best.1;
        let mut d0 = Procedural::deriv(self, uv).p.distance(p);
        for _ in 0..4 {
            let d = Procedural::deriv(self, uv);
            let e = d.p - p;
            let (g0, g1) = (d.du.dot(e), d.dv.dot(e));
            let (h00, h01, h11) = (
                d.du.length_sq() + d.duu.dot(e),
                d.du.dot(d.dv) + d.duv.dot(e),
                d.dv.length_sq() + d.dvv.dot(e),
            );
            let Some((x, y)) = forge_math::linalg::solve2(h00, h01, h01, h11, -g0, -g1) else {
                break;
            };
            let next = Point2::new((uv.x + x).clamp(self.lo, self.hi), (uv.y + y).clamp(0.0, 1.0));
            let dn = Procedural::deriv(self, next).p.distance(p);
            if dn.is_nan() || dn >= d0 {
                break;
            }
            (uv, d0) = (next, dn);
        }
        uv
    }

    fn transformed_blend(&self, f: &Frame) -> VarBlend {
        let pw = |p: Point3| f.point_to_world(p);
        let vw = |v: Vec3| f.vec_to_world(v);
        let rows = self
            .rows
            .iter()
            .map(|r| Row {
                c: [
                    pw(Point3::ORIGIN + r.c[0]).to_vec(),
                    vw(r.c[1]),
                    vw(r.c[2]),
                    vw(r.c[3]),
                ],
                k: pw(r.k),
                e1: vw(r.e1),
                e2: vw(r.e2),
                pts: r.pts.map(pw),
                ..*r
            })
            .collect();
        VarBlend {
            a: self.a.transformed(f),
            b: self.b.transformed(f),
            guide: self.guide.transformed(f),
            rows,
            ..self.clone()
        }
    }

    /// Rows bounding [s0, s1]: exact sections at the ends, cached rows between.
    fn rows_over(&self, s0: f64, s1: f64) -> Vec<(f64, Point3, f64, Vec3, Vec3, f64, usize)> {
        let at = |s: f64| {
            let sec = self.section_at(s);
            let i = self
                .segs
                .len()
                .min(self.rows.partition_point(|r| r.s <= s))
                .max(1)
                - 1;
            (
                s,
                Point3::ORIGIN + sec.k.at(0),
                sec.rho.0[0],
                sec.e1.at(0),
                sec.e2.at(0),
                sec.phi.0[0],
                i,
            )
        };
        let mut out = vec![at(s0)];
        for (i, r) in self.rows.iter().enumerate() {
            if r.s > s0 && r.s < s1 {
                out.push((r.s, r.k, r.rho, r.e1, r.e2, r.phi, i.min(self.segs.len() - 1)));
            }
        }
        if s1 > s0 {
            out.push(at(s1));
        }
        out
    }
}

/// Largest principal curvature magnitude from first and second derivatives.
fn principal_max(d: &SurfDeriv) -> f64 {
    let Some(n) = d.du.cross(d.dv).normalize().map(|u| u.get()) else {
        return 0.0;
    };
    let (e, f, g) = (d.du.length_sq(), d.du.dot(d.dv), d.dv.length_sq());
    let (l, m, nn) = (d.duu.dot(n), d.duv.dot(n), d.dvv.dot(n));
    let det = e * g - f * f;
    if det <= 0.0 {
        return 0.0;
    }
    let k = (l * nn - m * m) / det;
    let h = (e * nn - 2.0 * f * m + g * l) / (2.0 * det);
    h.abs() + (h * h - k).max(0.0).sqrt()
}

/// Box of the arc k + rho (cos t e1 + sin t e2), t in [t0, t1].
fn arc_box(k: Point3, rho: f64, e1: Vec3, e2: Vec3, t0: f64, t1: f64) -> Box3 {
    let mut b = Box3::EMPTY;
    let p = |t: f64| {
        let (s, c) = dmath::sin_cos(t);
        k + (e1 * c + e2 * s) * rho
    };
    b.include(p(t0));
    b.include(p(t1));
    for ax in 0..3 {
        let t = dmath::atan2(e2[ax], e1[ax]);
        // extremes of cos(t - t*) sit at t* + m pi
        let mut m = ((t0 - t) / dmath::PI).ceil();
        while t + m * dmath::PI <= t1 {
            b.include(p(t + m * dmath::PI));
            m += 1.0;
        }
    }
    b
}

impl Procedural for VarBlend {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        VarBlend::point(&self.section_at(uv.x), uv.y)
    }
    fn domain(&self) -> Domain2 {
        Domain2 {
            u: Domain1 {
                lo: self.lo,
                hi: self.hi,
                period: None,
            },
            v: Domain1 {
                lo: 0.0,
                hi: 1.0,
                period: None,
            },
        }
    }
    fn project(&self, p: Point3) -> Point2 {
        self.nearest(p)
    }
    fn bbox(&self, uv: Box2) -> Box3 {
        let s0 = uv.u.lo.clamp(self.lo, self.hi);
        let s1 = uv.u.hi.clamp(s0, self.hi);
        let (v0, v1) = (
            uv.v.lo.clamp(0.0, 1.0),
            uv.v.hi.clamp(0.0, 1.0).max(uv.v.lo.clamp(0.0, 1.0)),
        );
        let rows = self.rows_over(s0, s1);
        let arc =
            |r: &(f64, Point3, f64, Vec3, Vec3, f64, usize)| arc_box(r.1, r.2, r.3, r.4, r.5 * v0, r.5 * v1);
        let mut b = arc(&rows[0]);
        for w in rows.windows(2) {
            // a point between two rows is within half the gap times |B_u| of one of them
            let grow = self.segs[w[0].6.max(w[1].6.saturating_sub(1))]
                .vb
                .max(self.segs[w[0].6].vb)
                * (w[1].0 - w[0].0)
                * 0.5;
            b = b.hull(&arc(&w[0]).hull(&arc(&w[1])).grow(grow));
        }
        b.grow(tol::LINEAR)
    }
    fn implicit(&self, p: Point3) -> f64 {
        self.tube_min(p).0
    }
    fn gradient(&self, p: Point3) -> Vec3 {
        let (_, _, c) = self.tube_min(p);
        (p - c).normalize().map_or(Vec3::Z, |u| u.get())
    }
    fn transformed(&self, f: &Frame) -> Proc {
        Proc(Arc::new(self.transformed_blend(f)))
    }
    fn uv_scale(&self) -> (f64, f64) {
        let u = self.rows.iter().map(|r| r.bu).fold(0.0, f64::max);
        let v = self.rows.iter().map(|r| r.bv).fold(0.0, f64::max);
        (u.max(tol::LINEAR), v.max(tol::LINEAR))
    }
    fn min_radius(&self) -> f64 {
        let k = self.rows.iter().map(|r| r.kmax).fold(0.0, f64::max);
        let rmin = self.law.bounds(self.lo, self.hi).0;
        0.8 * rmin.min(if k > 0.0 { 1.0 / k } else { f64::INFINITY })
    }
}

/// Where a variable blend touches one of its surfaces: the contact point of the ball at s.
#[derive(Debug)]
pub struct Spring {
    blend: Arc<VarBlend>,
    /// 0: the v = 0 edge, 1: the v = 1 edge.
    v: usize,
}

impl Spring {
    fn segs_over(&self, range: Interval) -> impl Iterator<Item = (usize, f64)> + '_ {
        let rows = &self.blend.rows;
        (0..self.blend.segs.len()).filter_map(move |i| {
            let w = rows[i + 1].s.min(range.hi) - rows[i].s.max(range.lo);
            (w >= 0.0).then_some((i, w))
        })
    }
}

impl ProceduralCurve for Spring {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let q = self.blend.section_at(t).q[self.v];
        CurveDeriv {
            p: Point3::ORIGIN + q.at(0),
            d1: q.at(1),
            d2: q.at(2) * 2.0,
        }
    }
    fn domain(&self) -> Domain1 {
        Domain1 {
            lo: self.blend.lo,
            hi: self.blend.hi,
            period: None,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        self.blend.spring_closest(self.v, p, range).0
    }
    fn bbox(&self, range: Interval) -> Box3 {
        let b = &self.blend;
        let s0 = range.lo.clamp(b.lo, b.hi);
        let s1 = range.hi.clamp(s0, b.hi);
        let q = |s: f64| Point3::ORIGIN + b.section_at(s).q[self.v].at(0);
        let mut pts = vec![(s0, q(s0))];
        pts.extend(
            b.rows
                .iter()
                .filter(|r| r.s > s0 && r.s < s1)
                .map(|r| (r.s, r.pts[2 * self.v])),
        );
        if s1 > s0 {
            pts.push((s1, q(s1)));
        }
        let mut out = Box3::from_point(pts[0].1);
        for w in pts.windows(2) {
            let i = b
                .rows
                .partition_point(|r| r.s <= 0.5 * (w[0].0 + w[1].0))
                .clamp(1, b.segs.len())
                - 1;
            let grow = b.segs[i].vq[self.v] * (w[1].0 - w[0].0) * 0.5;
            out = out.hull(&Box3::from_points([w[0].1, w[1].1]).grow(grow));
        }
        out.grow(tol::LINEAR)
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(Spring {
            blend: Arc::new(self.blend.transformed_blend(f)),
            v: self.v,
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        self.segs_over(range)
            .map(|(i, _)| self.blend.segs[i].vq[self.v])
            .fold(0.0, f64::max)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        // equal steps h keep each chord within |q''| h^2 / 8 of the curve
        let a = self
            .segs_over(range)
            .map(|(i, _)| self.blend.segs[i].aq[self.v])
            .fold(0.0, f64::max);
        let n = range.width() * (a / (8.0 * chord.max(tol::LINEAR))).sqrt();
        (n.ceil() as usize).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;
    use crate::surface::{Cone, Cylinder, Plane, Sphere};
    use forge_math::rng::Rng;

    fn plane(o: Point3, n: Vec3) -> Surface {
        Surface::Plane(Plane::new(Frame::from_z(o, n.normalize().unwrap())))
    }

    fn line_y() -> Guide {
        Guide::Line(Line::new(Point3::ORIGIN, UnitVec3::Y))
    }

    fn circle(z: f64, r: f64) -> Guide {
        Guide::Circle(Circle {
            frame: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, z)),
            radius: r,
        })
    }

    struct Case {
        name: &'static str,
        blend: Arc<VarBlend>,
    }

    #[allow(clippy::too_many_arguments)]
    fn make(
        name: &'static str,
        a: Surface,
        sa: f64,
        b: Surface,
        sb: f64,
        g: Guide,
        law: RadiusLaw,
        lo: f64,
        hi: f64,
    ) -> Case {
        let blend = VarBlend::new(&a, sa, &b, sb, g, Arc::new(law), Interval::new(lo, hi))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        Case { name, blend }
    }

    fn cyl_cyl() -> Case {
        let a = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 10.0,
        });
        let b = Surface::Cylinder(Cylinder {
            frame: Frame::from_z(Point3::ORIGIN, UnitVec3::X),
            radius: 4.0,
        });
        let region = Box3::from_points([Point3::new(-12.0, -12.0, -12.0), Point3::new(12.0, 12.0, 12.0)]);
        let loops = crate::intcurve::trace(&a, &b, region).unwrap();
        let ic = loops.into_iter().find(|c| c.pts[0].x > 0.0).unwrap();
        let hi = ic.domain_hi();
        let edge = Curve::Intersection(Arc::new(ic));
        let (g, ranges) = Guide::through_edges(&[(edge, Interval::new(0.05 * hi, 0.3 * hi))], 8).unwrap();
        let r = ranges[0];
        let law = RadiusLaw::smooth(&[(r.lo, 0.8), (r.mid(), 1.6), (r.hi, 1.0)], None).unwrap();
        make("cylinder-cylinder", a, 1.0, b, 1.0, g, law, r.lo, r.hi)
    }

    fn cases() -> Vec<Case> {
        let floor = plane(Point3::ORIGIN, Vec3::Z);
        let wall = plane(Point3::ORIGIN, Vec3::X);
        let cyl = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD,
            radius: 10.0,
        });
        let cone = Surface::Cone(Cone::new(Frame::from_z(Point3::ORIGIN, -UnitVec3::Z), 6.0, 0.3).unwrap());
        let sphere = Surface::Sphere(Sphere {
            frame: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 3.0)),
            radius: 5.0,
        });
        let tilted = plane(Point3::ORIGIN, Vec3::new(1.0, 0.0, 0.6));
        let smooth = |p: &[(f64, f64)], e| RadiusLaw::smooth(p, e).unwrap();
        vec![
            make(
                "plane-plane const",
                floor.clone(),
                1.0,
                wall.clone(),
                1.0,
                line_y(),
                RadiusLaw::constant(2.0),
                -5.0,
                5.0,
            ),
            make(
                "plane-plane linear",
                floor.clone(),
                1.0,
                wall.clone(),
                1.0,
                line_y(),
                RadiusLaw::linear(-5.0, 1.0, 5.0, 3.0),
                -5.0,
                5.0,
            ),
            make(
                "plane-plane smooth obtuse",
                floor.clone(),
                1.0,
                tilted,
                1.0,
                line_y(),
                smooth(&[(-5.0, 1.0), (0.0, 2.5), (5.0, 1.5)], None),
                -5.0,
                5.0,
            ),
            make(
                "plane-plane convex",
                floor.clone(),
                -1.0,
                wall,
                -1.0,
                line_y(),
                RadiusLaw::linear(-5.0, 2.0, 5.0, 0.5),
                -5.0,
                5.0,
            ),
            make(
                "plane-cylinder const",
                floor.clone(),
                1.0,
                cyl.clone(),
                1.0,
                circle(0.0, 10.0),
                RadiusLaw::constant(1.5),
                0.0,
                3.0,
            ),
            make(
                "plane-cylinder outside",
                floor.clone(),
                1.0,
                cyl.clone(),
                1.0,
                circle(0.0, 10.0),
                RadiusLaw::linear(0.0, 1.0, 3.0, 3.0),
                0.0,
                3.0,
            ),
            make(
                "plane-cylinder hole",
                floor.clone(),
                1.0,
                cyl,
                -1.0,
                circle(0.0, 10.0),
                smooth(
                    &[(0.0, 1.0), (1.0, 4.0), (2.0, 2.0), (3.0, 3.0)],
                    Some((0.0, 0.0)),
                ),
                0.0,
                3.0,
            ),
            make(
                "plane-cone",
                floor.clone(),
                1.0,
                cone,
                1.0,
                circle(0.0, 6.0),
                RadiusLaw::linear(0.0, 0.5, 4.0, 2.0),
                0.0,
                4.0,
            ),
            make(
                "plane-sphere",
                floor,
                1.0,
                sphere,
                1.0,
                circle(0.0, 4.0),
                RadiusLaw::linear(0.0, 0.5, 2.0, 1.5),
                0.0,
                2.0,
            ),
            cyl_cyl(),
        ]
    }

    fn rand_uv(r: &mut Rng, b: &VarBlend) -> Point2 {
        Point2::new(r.range(b.lo, b.hi), r.unit())
    }

    #[test]
    fn plane_plane_matches_cylinder_and_cone() {
        let cs = cases();
        let mut r = Rng::new(1);
        for _ in 0..200 {
            let uv = rand_uv(&mut r, &cs[0].blend);
            let p = Procedural::deriv(&*cs[0].blend, uv).p;
            // axis x = z = 2 along y
            let d = ((p.x - 2.0).powi(2) + (p.z - 2.0).powi(2)).sqrt();
            assert!((d - 2.0).abs() < 1e-10, "{d}");
            // linear radius: a circular cone about the spine (r, s, r), r = 2 + s / 5
            let b = &cs[1].blend;
            let p = Procedural::deriv(&**b, uv).p;
            let m = 0.2;
            let apex = Point3::new(0.0, -10.0, 0.0);
            let axis = Vec3::new(m, 1.0, m).normalize().unwrap().get();
            let w = p - apex;
            let t = w.dot(axis);
            let radial = (w - axis * t).length();
            let tan = m / (1.0f64 + m * m).sqrt();
            assert!((radial - t * tan).abs() < 1e-10, "{radial} {}", t * tan);
        }
    }

    #[test]
    fn plane_cylinder_constant_is_a_torus() {
        let cs = cases();
        let b = &cs[4].blend;
        let mut r = Rng::new(2);
        for _ in 0..200 {
            let p = Procedural::deriv(&**b, rand_uv(&mut r, b)).p;
            let rho = (p.x * p.x + p.y * p.y).sqrt();
            let d = ((rho - 11.5).powi(2) + (p.z - 1.5).powi(2)).sqrt();
            assert!((d - 1.5).abs() < 1e-10, "{d}");
        }
    }

    #[test]
    fn balls_touch_both_surfaces_and_the_blend() {
        let mut r = Rng::new(3);
        for c in cases() {
            let b = &c.blend;
            for _ in 0..100 {
                let uv = rand_uv(&mut r, b);
                let s = uv.x;
                let rad = b.radius(s);
                let ctr = b.centre(s);
                assert!((b.a.implicit(ctr) - b.side[0] * rad).abs() < 1e-9, "{}", c.name);
                assert!((b.b.implicit(ctr) - b.side[1] * rad).abs() < 1e-9, "{}", c.name);
                let d = Procedural::deriv(&**b, uv);
                assert!((d.p.distance(ctr) - rad).abs() < 1e-9, "{} on ball", c.name);
                // envelope: the blend's normal points out of the ball through its centre
                let n = d.du.cross(d.dv).normalize().unwrap();
                assert!(n.dot((d.p - ctr) / rad) > 1.0 - 1e-9, "{} normal", c.name);
                // contacts: on each surface, and tangent to it there (G1 along the springs)
                for (i, surf) in [&b.a, &b.b].into_iter().enumerate() {
                    let q = Procedural::deriv(&**b, Point2::new(s, b.v_on(i)));
                    assert!(surf.implicit(q.p).abs() < 1e-9, "{} contact {i}", c.name);
                    let nb = q.du.cross(q.dv).normalize().unwrap();
                    assert!(nb.cross(surf.gradient(q.p)).length() < 1e-9, "{} G1 {i}", c.name);
                    let sp = b.spring(i);
                    assert!(sp.eval(s).distance(q.p) < 1e-12, "{} spring {i}", c.name);
                }
            }
        }
    }

    #[test]
    fn derivatives_match_finite_differences() {
        let mut r = Rng::new(4);
        for c in cases() {
            let b = &*c.blend;
            for _ in 0..100 {
                let uv = rand_uv(&mut r, b);
                let uv = Point2::new(uv.x.clamp(b.lo + 1e-3, b.hi - 1e-3), uv.y);
                let h = 1e-5;
                let d = Procedural::deriv(b, uv);
                let at = |u: f64, v: f64| Procedural::deriv(b, Point2::new(u, v));
                let close = |x: Vec3, y: Vec3, what: &str| {
                    assert!(
                        (x - y).length() < 1e-5 * (1.0 + y.length()),
                        "{} {what} {uv:?}: {x:?} vs {y:?}",
                        c.name
                    );
                };
                let (pu, mu) = (at(uv.x + h, uv.y), at(uv.x - h, uv.y));
                let (pv, mv) = (at(uv.x, uv.y + h), at(uv.x, uv.y - h));
                close((pu.p - mu.p) / (2.0 * h), d.du, "du");
                close((pv.p - mv.p) / (2.0 * h), d.dv, "dv");
                close((pu.du - mu.du) / (2.0 * h), d.duu, "duu");
                close((pv.du - mv.du) / (2.0 * h), d.duv, "duv");
                close((pv.dv - mv.dv) / (2.0 * h), d.dvv, "dvv");
                let sp = c.blend.spring_curve(0);
                let cd = sp.deriv(uv.x);
                close(
                    (sp.deriv(uv.x + h).p - sp.deriv(uv.x - h).p) / (2.0 * h),
                    cd.d1,
                    "spring d1",
                );
                close(
                    (sp.deriv(uv.x + h).d1 - sp.deriv(uv.x - h).d1) / (2.0 * h),
                    cd.d2,
                    "spring d2",
                );
            }
        }
    }

    #[test]
    fn bbox_contains_samples() {
        let mut r = Rng::new(5);
        for c in cases() {
            let b = &*c.blend;
            for k in 0..40 {
                let (p, q) = (rand_uv(&mut r, b), rand_uv(&mut r, b));
                let bx = if k == 0 {
                    Box2::new(Interval::new(b.lo, b.hi), Interval::new(0.0, 1.0))
                } else {
                    Box2::new(
                        Interval::new(p.x.min(q.x), p.x.max(q.x)),
                        Interval::new(p.y.min(q.y), p.y.max(q.y)),
                    )
                };
                let bb = Procedural::bbox(b, bx);
                for i in 0..=12 {
                    for j in 0..=12 {
                        let uv = Point2::new(
                            bx.u.lo + bx.u.width() * i as f64 / 12.0,
                            bx.v.lo + bx.v.width() * j as f64 / 12.0,
                        );
                        assert!(
                            bb.contains(Procedural::deriv(b, uv).p),
                            "{} {bx:?} {uv:?}",
                            c.name
                        );
                    }
                }
                let sp = c.blend.spring_curve(1);
                let cb = sp.bbox(bx.u);
                for i in 0..=20 {
                    let t = bx.u.lo + bx.u.width() * i as f64 / 20.0;
                    assert!(cb.contains(sp.deriv(t).p), "{} spring box", c.name);
                }
            }
        }
    }

    fn grid_nearest(b: &VarBlend, p: Point3, nu: usize, nv: usize) -> f64 {
        (0..=nu)
            .flat_map(|i| (0..=nv).map(move |j| (i, j)))
            .map(|(i, j)| {
                let g = Point2::new(b.lo + (b.hi - b.lo) * i as f64 / nu as f64, j as f64 / nv as f64);
                Procedural::deriv(b, g).p.distance(p)
            })
            .fold(f64::INFINITY, f64::min)
    }

    #[test]
    fn project_inverts_eval_and_is_nearest() {
        let mut r = Rng::new(6);
        for c in cases() {
            let b = &*c.blend;
            let off = 0.2 * b.min_radius();
            for _ in 0..60 {
                let uv = rand_uv(&mut r, b);
                let d = Procedural::deriv(b, uv);
                let back = Procedural::deriv(b, Procedural::project(b, d.p)).p;
                assert!(
                    back.distance(d.p) < 1e-9,
                    "{} {uv:?} {}",
                    c.name,
                    back.distance(d.p)
                );
                let n = d.du.cross(d.dv).normalize().unwrap();
                let p = d.p + n * r.range(-off, off);
                let q = Procedural::deriv(b, Procedural::project(b, p)).p;
                assert!(q.distance(p) <= d.p.distance(p) + 1e-9, "{} off {uv:?}", c.name);
            }
            for _ in 0..20 {
                let p = Point3::new(r.range(-15.0, 15.0), r.range(-15.0, 15.0), r.range(-5.0, 10.0));
                let q = Procedural::deriv(b, Procedural::project(b, p)).p;
                let best = grid_nearest(b, p, 60, 12);
                assert!(
                    q.distance(p) <= best + 1e-9,
                    "{} far {p:?}: {} vs {best}",
                    c.name,
                    q.distance(p)
                );
            }
        }
    }

    #[test]
    fn implicit_is_zero_on_the_blend_and_one_lipschitz() {
        let mut r = Rng::new(7);
        for c in cases() {
            let b = &*c.blend;
            for _ in 0..100 {
                let d = Procedural::deriv(b, rand_uv(&mut r, b));
                assert!(b.implicit(d.p).abs() < 1e-9, "{}", c.name);
                let n = d.du.cross(d.dv).normalize().unwrap();
                let e = 0.1 * b.min_radius();
                assert!(
                    b.implicit(d.p + n * e) > 0.0 && b.implicit(d.p - n * e) < 0.0,
                    "{} sign",
                    c.name
                );
                assert!(b.gradient(d.p).dot(n.get()) > 1.0 - 1e-9, "{} gradient", c.name);
                let p = d.p + Vec3::new(r.range(-3.0, 3.0), r.range(-3.0, 3.0), r.range(-3.0, 3.0));
                let q = p + Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
                assert!(
                    (b.implicit(p) - b.implicit(q)).abs() <= p.distance(q) + 1e-9,
                    "{} lipschitz",
                    c.name
                );
            }
        }
    }

    #[test]
    fn spring_curves_project_and_segment() {
        let mut r = Rng::new(8);
        for c in cases() {
            for i in 0..2 {
                let sp = c.blend.spring_curve(i);
                let dom = sp.domain().interval();
                for _ in 0..20 {
                    let t = r.range(dom.lo, dom.hi);
                    let p = sp.deriv(t).p;
                    let back = sp.closest(p, dom);
                    assert!(sp.deriv(back).p.distance(p) < 1e-9, "{} closest", c.name);
                }
                let chord = 0.01;
                let n = sp.segments(dom, chord);
                for k in 0..n {
                    let t0 = dom.lo + dom.width() * k as f64 / n as f64;
                    let t1 = dom.lo + dom.width() * (k + 1) as f64 / n as f64;
                    let (p0, p1) = (sp.deriv(t0).p, sp.deriv(t1).p);
                    let m = sp.deriv(0.5 * (t0 + t1)).p;
                    let l = Line::through(p0, p1).unwrap();
                    let foot = l.eval(l.param_of(m));
                    assert!(foot.distance(m) <= chord, "{} segments {n}", c.name);
                }
                assert!(
                    sp.speed_bound(dom) >= sp.deriv(dom.mid()).d1.length(),
                    "{}",
                    c.name
                );
            }
        }
    }

    #[test]
    fn transform_moves_points() {
        let mut r = Rng::new(9);
        for c in cases() {
            let f = crate::testing::rand_frame(&mut r);
            let s = c.blend.surface();
            let t = s.transformed(&f);
            for _ in 0..20 {
                let uv = rand_uv(&mut r, &c.blend);
                assert!(
                    t.eval(uv).distance(f.point_to_world(s.eval(uv))) < 1e-9,
                    "{}",
                    c.name
                );
            }
        }
    }

    #[test]
    fn tangent_chain_blends_meet_smoothly() {
        // a floor, and a wall running along x that turns into a cylinder
        let floor = plane(Point3::ORIGIN, Vec3::Z);
        let wall = plane(Point3::ORIGIN, Vec3::Y);
        let big = 20.0;
        let cyl = Surface::Cylinder(Cylinder {
            frame: Frame::WORLD.with_origin(Point3::new(0.0, big, 0.0)),
            radius: big,
        });
        let line = Curve::Line(Line::new(Point3::new(-10.0, 0.0, 0.0), UnitVec3::X));
        let arc = Curve::Circle(Circle {
            frame: Frame::WORLD.with_origin(Point3::new(0.0, big, 0.0)),
            radius: big,
        });
        let edges = [
            (line, Interval::new(0.0, 10.0)),
            (arc, Interval::new(-dmath::FRAC_PI_2, -dmath::FRAC_PI_2 + 1.0)),
        ];
        let (g, ranges) = Guide::through_edges(&edges, 6).unwrap();
        let (r0, r1) = (ranges[0], ranges[1]);
        let law = Arc::new(RadiusLaw::smooth(&[(r0.lo, 1.0), (r0.hi, 2.5), (r1.hi, 1.5)], None).unwrap());
        let a = VarBlend::new(&floor, 1.0, &wall, 1.0, g.clone(), law.clone(), r0).unwrap();
        let b = VarBlend::new(&floor, 1.0, &cyl, -1.0, g, law, r1).unwrap();
        let s = r0.hi;
        for k in 0..=8 {
            let v = k as f64 / 8.0;
            let (da, db) = (
                Procedural::deriv(&*a, Point2::new(s, v)),
                Procedural::deriv(&*b, Point2::new(s, v)),
            );
            assert!(da.p.distance(db.p) < 1e-9, "{v}: {:?} {:?}", da.p, db.p);
            let (na, nb) = (
                da.du.cross(da.dv).normalize().unwrap(),
                db.du.cross(db.dv).normalize().unwrap(),
            );
            assert!(na.dot(nb) > 1.0 - 1e-12, "{v}");
        }
        // the spine turns smoothly through the junction (its curvature follows the wall's, which
        // jumps, so the blends' du differ there; the shared circle and normals make them G1)
        let (ca, cb) = (a.spine_at(s).0, b.spine_at(s).0);
        assert!(
            (ca.at(1) - cb.at(1)).length() < 1e-9,
            "{:?} {:?}",
            ca.at(1),
            cb.at(1)
        );
    }

    #[test]
    fn bad_input_is_rejected() {
        let floor = plane(Point3::ORIGIN, Vec3::Z);
        let wall = plane(Point3::ORIGIN, Vec3::X);
        let ceiling = plane(Point3::new(0.0, 0.0, 4.0), -Vec3::Z);
        let law = |r| Arc::new(RadiusLaw::constant(r));
        let range = Interval::new(0.0, 1.0);
        let e = VarBlend::new(&floor, 1.0, &ceiling, 1.0, line_y(), law(2.0), range).unwrap_err();
        assert!(
            matches!(e, VarBlendError::NoStart | VarBlendError::Tangent { .. }),
            "{e}"
        );
        let e = VarBlend::new(&floor, 1.0, &wall, 1.0, line_y(), law(-1.0), range).unwrap_err();
        assert_eq!(e, VarBlendError::BadRadius);
        let crossing = Arc::new(RadiusLaw::linear(0.0, 1.0, 1.0, -1.0));
        assert_eq!(
            VarBlend::new(&floor, 1.0, &wall, 1.0, line_y(), crossing, range).unwrap_err(),
            VarBlendError::BadRadius
        );
        let ok = VarBlend::new(&floor, 1.0, &wall, 1.0, line_y(), law(1.0), range).unwrap();
        let e = VarBlend::new(&ok.surface(), 1.0, &wall, 1.0, line_y(), law(1.0), range).unwrap_err();
        assert_eq!(e, VarBlendError::NotAnalytic);
    }
}

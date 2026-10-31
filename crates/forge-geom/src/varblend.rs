//! Variable-radius rolling-ball blends between two surfaces.
//!
//! A ball of radius r(s) rolls along the edge between surfaces A and B, touching both. Its
//! centre c(s) is the point at signed distance `side_a r(s)` from A and `side_b r(s)` from B
//! lying in the plane through the guide point g(s) perpendicular to g'(s). The blend is the
//! envelope of those balls (a canal surface): at each s the ball touches the envelope along its
//! characteristic circle, in the plane normal to c'(s), offset -r r'/|c'| from c, of radius
//! r sqrt(1 - r'^2/|c'|^2). The face is the arc of that circle between the two contact points.
//!
//! Why the normal plane fixes s: it is the edge's own parameter for a line or circle, so a
//! law stated on the edge means what it says. It is well posed while the ball is nearer the
//! edge than the edge's radius of curvature: the plane then meets the offsets' intersection
//! once, transversally. It is also stable: a small change of edge or law moves c(s) smoothly.
//! Where the plane turns smoothly (C2 guide), c' is continuous, so the characteristic circles
//! are too, and blends of a tangent chain share their junction circles.
//!
//! A full round's balls also touch a third surface M (`VarBlend::round`): the radius is then
//! solved with the centre at each s (the law is only the rows' fit, for seeds and bounds), and
//! the arc runs from contact to contact over M, more than half a turn where M lies that way.
//!
//! Rejected: |r'| reaching |c'| (no envelope), a law bending faster than the balls can roll
//! (F_ss <= 0 on the arc, F = |x - c|^2 - r^2: the envelope folds), coinciding contacts.
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

/// Three rows `a`, `b`, `c` of a 3x3 system, factored once for several right-hand sides
/// (Cramer, by cross products); None when they are nearly dependent.
struct Rows3 {
    bc: Vec3,
    ca: Vec3,
    ab: Vec3,
    inv: f64,
}

impl Rows3 {
    fn new(a: Vec3, b: Vec3, c: Vec3) -> Option<Rows3> {
        let (bc, ca, ab) = (b.cross(c), c.cross(a), a.cross(b));
        let det = a.dot(bc);
        if det.is_nan() || det.abs() <= 1e-12 * (a.length_sq() * b.length_sq() * c.length_sq()).sqrt() {
            return None;
        }
        Some(Rows3 {
            bc,
            ca,
            ab,
            inv: 1.0 / det,
        })
    }
    fn solve(&self, r: [f64; 3]) -> Vec3 {
        (self.bc * r[0] + self.ca * r[1] + self.ab * r[2]) * self.inv
    }
}

/// A 4x4 system factored once (LU with partial pivoting) for several right-hand sides; None
/// when it is nearly singular.
struct Rows4 {
    lu: [[f64; 4]; 4],
    perm: [usize; 4],
}

impl Rows4 {
    fn new(mut lu: [[f64; 4]; 4]) -> Option<Rows4> {
        let scale = lu.iter().flatten().fold(0.0f64, |m, x| m.max(x.abs()));
        let mut perm = [0, 1, 2, 3];
        for k in 0..4 {
            let p = (k..4).max_by(|&i, &j| lu[i][k].abs().total_cmp(&lu[j][k].abs()))?;
            if lu[p][k].is_nan() || lu[p][k].abs() <= 1e-12 * scale {
                return None;
            }
            lu.swap(k, p);
            perm.swap(k, p);
            let pivot = lu[k];
            for row in &mut lu[k + 1..] {
                let f = row[k] / pivot[k];
                row[k] = f;
                for (x, p) in row.iter_mut().zip(pivot).skip(k + 1) {
                    *x -= f * p;
                }
            }
        }
        Some(Rows4 { lu, perm })
    }
    fn solve(&self, b: [f64; 4]) -> [f64; 4] {
        let mut x = self.perm.map(|i| b[i]);
        for i in 1..4 {
            for j in 0..i {
                x[i] -= self.lu[i][j] * x[j];
            }
        }
        for i in (0..4).rev() {
            for j in i + 1..4 {
                x[i] -= self.lu[i][j] * x[j];
            }
            x[i] /= self.lu[i][i];
        }
        x
    }
}

/// How the radius varies along the edge: a function of the guide parameter s. It is design
/// data (which ball sits where), so a C2 piecewise cubic is a fine way to state it. Past its
/// first and last control points the end pieces carry on.
#[derive(Clone, Debug, PartialEq)]
pub struct RadiusLaw {
    knots: Vec<f64>,
    /// Per segment: r = a + b h + c h^2 + d h^3, h measured from the segment's first knot.
    coef: Vec<[f64; 4]>,
    /// A closed edge's law repeats with this period (the span of its knots).
    period: Option<f64>,
}

impl RadiusLaw {
    pub fn constant(r: f64) -> RadiusLaw {
        RadiusLaw::linear(0.0, r, 1.0, r)
    }
    /// r0 at s0 to r1 at s1, linear in s.
    pub fn linear(s0: f64, r0: f64, s1: f64, r1: f64) -> RadiusLaw {
        RadiusLaw::fit(&[s0, s1], &[r0, r1], None)
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
        Some(RadiusLaw::fit(&x, &y, end_slopes))
    }

    /// A shape-preserving C1 cubic through (s, r) control points (s increasing): it never
    /// overshoots them (Fritsch-Carlson slopes), so it stays between neighbouring radii, and
    /// it holds the end radii past the ends.
    pub fn monotone(points: &[(f64, f64)]) -> Option<RadiusLaw> {
        let n = points.len();
        if n < 2
            || points
                .windows(2)
                .any(|w| w[1].0.partial_cmp(&w[0].0) != Some(std::cmp::Ordering::Greater))
        {
            return None;
        }
        let h: Vec<f64> = (0..n - 1).map(|i| points[i + 1].0 - points[i].0).collect();
        let d: Vec<f64> = (0..n - 1)
            .map(|i| (points[i + 1].1 - points[i].1) / h[i])
            .collect();
        // flat ends; inside, the weighted harmonic mean of the neighbouring slopes, zero at an extreme
        let mut m = vec![0.0; n];
        for i in 1..n - 1 {
            if d[i - 1] * d[i] > 0.0 {
                let (w1, w2) = (2.0 * h[i] + h[i - 1], h[i] + 2.0 * h[i - 1]);
                m[i] = (w1 + w2) / (w1 / d[i - 1] + w2 / d[i]);
            }
        }
        let mut knots = vec![points[0].0 - 1.0];
        let mut coef = vec![[points[0].1, 0.0, 0.0, 0.0]];
        for i in 0..n - 1 {
            let (y0, hi) = (points[i].1, h[i]);
            let c = (3.0 * d[i] - 2.0 * m[i] - m[i + 1]) / hi;
            let e = (m[i] + m[i + 1] - 2.0 * d[i]) / (hi * hi);
            knots.push(points[i].0);
            coef.push([y0, m[i], c, e]);
        }
        knots.push(points[n - 1].0);
        coef.push([points[n - 1].1, 0.0, 0.0, 0.0]);
        knots.push(points[n - 1].0 + 1.0);
        Some(RadiusLaw {
            knots,
            coef,
            period: None,
        })
    }

    /// `smooth` with each end's slope given or left natural.
    pub fn smooth_ends(points: &[(f64, f64)], start: Option<f64>, end: Option<f64>) -> Option<RadiusLaw> {
        if points.len() < 2
            || points
                .windows(2)
                .any(|w| w[1].0.partial_cmp(&w[0].0) != Some(std::cmp::Ordering::Greater))
        {
            return None;
        }
        let (x, y): (Vec<f64>, Vec<f64>) = points.iter().copied().unzip();
        Some(RadiusLaw::fit_ends(&x, &y, start, end))
    }

    /// One law made of others: law i from `parts[i].0` to the next part's start (the last to
    /// `end`), the first and last carried on past the ends; with `period`, repeating from the
    /// first start (then `end` is a period on).
    pub fn concat(parts: &[(f64, &RadiusLaw)], end: f64, period: Option<f64>) -> Option<RadiusLaw> {
        let mut knots = Vec::new();
        let mut coef = Vec::new();
        for (i, &(a, law)) in parts.iter().enumerate() {
            let b = parts.get(i + 1).map_or(end, |p| p.0);
            if b <= a {
                return None;
            }
            knots.push(a);
            coef.push(law.coefs(a));
            for &k in law.knots.iter().filter(|&&k| k > a && k < b) {
                knots.push(k);
                coef.push(law.coefs(k));
            }
        }
        knots.push(end);
        Some(RadiusLaw { knots, coef, period })
    }

    /// The C2 periodic cubic spline through (s, r) control points (s increasing, all within one
    /// period of the first), for a closed edge whose s runs over `period`.
    pub fn periodic(points: &[(f64, f64)], period: f64) -> Option<RadiusLaw> {
        let n = points.len();
        let ok = n >= 1
            && period > 0.0
            && points.windows(2).all(|w| w[1].0 > w[0].0)
            && points[n - 1].0 < points[0].0 + period;
        if !ok {
            return None;
        }
        let mut x: Vec<f64> = points.iter().map(|p| p.0).collect();
        let mut y: Vec<f64> = points.iter().map(|p| p.1).collect();
        x.push(x[0] + period);
        y.push(y[0]);
        let h: Vec<f64> = (0..n).map(|i| x[i + 1] - x[i]).collect();
        let slope: Vec<f64> = (0..n).map(|i| (y[i + 1] - y[i]) / h[i]).collect();
        // cyclic system for the second derivatives, diagonally dominant: plain elimination
        let mut a = vec![vec![0.0; n + 1]; n];
        for i in 0..n {
            let p = (i + n - 1) % n;
            a[i][p] += h[p];
            a[i][i] += 2.0 * (h[p] + h[i]);
            a[i][(i + 1) % n] += h[i];
            a[i][n] = 6.0 * (slope[i] - slope[p]);
        }
        for c in 0..n {
            for r in c + 1..n {
                let f = a[r][c] / a[c][c];
                let pivot = a[c].clone();
                for (x, p) in a[r].iter_mut().zip(&pivot).skip(c) {
                    *x -= f * p;
                }
            }
        }
        let mut m = vec![0.0; n + 1];
        for c in (0..n).rev() {
            let acc = (c + 1..n).fold(a[c][n], |acc, k| acc - a[c][k] * m[k]);
            m[c] = acc / a[c][c];
        }
        m[n] = m[0];
        let mut law = RadiusLaw::pieces(&x, &y, &h, &slope, &m);
        law.period = Some(period);
        Some(law)
    }

    fn fit(x: &[f64], y: &[f64], ends: Option<(f64, f64)>) -> RadiusLaw {
        RadiusLaw::fit_ends(x, y, ends.map(|e| e.0), ends.map(|e| e.1))
    }

    fn fit_ends(x: &[f64], y: &[f64], start: Option<f64>, end: Option<f64>) -> RadiusLaw {
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
        if let Some(s0) = start {
            di[0] = 2.0 * h[0];
            up[0] = h[0];
            rhs[0] = 6.0 * (slope[0] - s0);
        }
        if let Some(s1) = end {
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
        RadiusLaw::pieces(x, y, &h, &slope, &m)
    }

    /// The cubic pieces from the knots' values and second derivatives.
    fn pieces(x: &[f64], y: &[f64], h: &[f64], slope: &[f64], m: &[f64]) -> RadiusLaw {
        let n = h.len();
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
        RadiusLaw {
            knots: x.to_vec(),
            coef,
            period: None,
        }
    }

    /// The cubic Hermite through (s, r, dr/ds) points, s increasing.
    fn hermite(points: &[(f64, f64, f64)]) -> RadiusLaw {
        let coef = points
            .windows(2)
            .map(|w| {
                let ((s0, r0, m0), (s1, r1, m1)) = (w[0], w[1]);
                let h = s1 - s0;
                let d = (r1 - r0) / h;
                [
                    r0,
                    m0,
                    (3.0 * d - 2.0 * m0 - m1) / h,
                    (m0 + m1 - 2.0 * d) / (h * h),
                ]
            })
            .collect();
        RadiusLaw {
            knots: points.iter().map(|p| p.0).collect(),
            coef,
            period: None,
        }
    }

    /// Taylor coefficients at s (past the ends, of the end pieces).
    fn coefs(&self, s: f64) -> [f64; 4] {
        let s = self.wrap(s);
        let i = self.knots.partition_point(|&k| k <= s).clamp(1, self.coef.len()) - 1;
        let h = s - self.knots[i];
        let [a, b, c, d] = self.coef[i];
        [
            a + h * (b + h * (c + h * d)),
            b + h * (2.0 * c + 3.0 * h * d),
            c + 3.0 * h * d,
            d,
        ]
    }

    /// s moved by whole periods into the first one.
    fn wrap(&self, s: f64) -> f64 {
        match self.period {
            Some(p) => self.knots[0] + (s - self.knots[0]).rem_euclid(p),
            None => s,
        }
    }

    /// Smallest and largest radius over [a, b].
    pub fn bounds(&self, a: f64, b: f64) -> (f64, f64) {
        if let Some(p) = self.period {
            let (k0, k1) = (self.knots[0], self.knots[self.knots.len() - 1]);
            if b - a >= p {
                return self.span_bounds(k0, k1);
            }
            let w = b - a;
            let a = self.wrap(a);
            let b = a + w;
            if b <= k1 {
                return self.span_bounds(a, b);
            }
            let (x, y) = (self.span_bounds(a, k1), self.span_bounds(k0, b - p));
            return (x.0.min(y.0), x.1.max(y.1));
        }
        self.span_bounds(a, b)
    }

    fn span_bounds(&self, a: f64, b: f64) -> (f64, f64) {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        let mut take = |s: f64| {
            let v = self.eval(s);
            lo = lo.min(v);
            hi = hi.max(v);
        };
        take(a);
        take(b);
        let n = self.coef.len();
        for (i, &[_, c1, c2, c3]) in self.coef.iter().enumerate() {
            // the end segments carry on past the end knots
            let k0 = if i == 0 { f64::NEG_INFINITY } else { self.knots[i] };
            let k1 = if i == n - 1 {
                f64::INFINITY
            } else {
                self.knots[i + 1]
            };
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
                let s = self.knots[i] + h;
                if s > a.max(k0) && s < b.min(k1) {
                    take(s);
                }
            }
        }
        (lo, hi)
    }

    pub fn eval(&self, s: f64) -> f64 {
        self.coefs(s)[0]
    }
    /// dr/ds.
    pub fn slope(&self, s: f64) -> f64 {
        self.coefs(s)[1]
    }
    fn jet(&self, s: f64) -> J {
        Jn(self.coefs(s))
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
                e.set(
                    k,
                    Rows3::new(na, nb, tan)?.solve([-fa, -fb, -w / (2.0 * k as f64)]),
                );
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

    /// Taylor coefficients (orders 0..3) of g and g' at s. Past the ends the end segments
    /// carry on, so a parameter a rounding error outside the range sees the same smooth guide.
    fn jets(&self, s: f64) -> ([Vec3; 4], [Vec3; 4]) {
        let n = self.coef.len();
        let i = self.knots.partition_point(|&k| k <= s).clamp(1, n) - 1;
        let (c, h) = (&self.coef[i], s - self.knots[i]);
        let e: [Vec3; 5] = std::array::from_fn(|k| {
            let mut acc = Vec3::ZERO;
            for j in (k..10).rev() {
                acc = acc * h + c[j] * binom(j, k);
            }
            acc
        });
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
    /// The guide along a chain of edges (lines, circles, intersection curves), each walked
    /// forward (t increasing) or not, carried on for `reach` (a length) past both ends so a
    /// blend can run out past its vertices. Returns the guide, each edge's s range and the
    /// whole s range with the run-outs.
    ///
    /// A lone line or circle is its own guide, s its parameter. Otherwise the Hermite through
    /// the chain, each edge split into `per_edge` equal steps of its own parameter: knots take
    /// each edge's exact point and arc-length derivatives; where two edges meet the tangents
    /// must agree (a tangent chain) and the higher derivatives are averaged, also where a
    /// closed chain (no reach) comes back to its start. Deterministic: knots depend only on
    /// the edges and their ranges.
    pub fn through_edges(
        edges: &[(crate::Curve, Interval, bool)],
        per_edge: usize,
        reach: f64,
    ) -> Option<(Guide, Vec<Interval>, Interval)> {
        match edges {
            [(crate::Curve::Line(l), r, _)] => {
                return Some((Guide::Line(*l), vec![*r], r.grow(reach)));
            }
            [(crate::Curve::Circle(c), r, _)] => {
                // run-outs stop short of meeting round the back: a blend going round more than
                // once would lie over itself, and points on it would project to either sheet
                let grow = (reach / c.radius).min(0.25 * (dmath::TAU - r.width()).max(0.0));
                return Some((Guide::Circle(*c), vec![*r], r.grow(grow)));
            }
            _ => {}
        }
        // the run-outs: each end edge carried on by `reach` along its own curve
        let run_out = |(c, r, fwd): &(crate::Curve, Interval, bool), at_end: bool| {
            let t = if at_end == *fwd { r.hi } else { r.lo };
            let d = c.domain();
            // a periodic curve carries on round, but not back over the chain's own stretch of it
            // (a blend going round onto itself lies over itself, and points on it project to
            // either sheet): each run-out takes at most a quarter of the turn the chain leaves
            let dt = match d.period {
                Some(p) => {
                    let own: f64 = edges.iter().filter(|e| e.0 == *c).map(|e| e.1.width()).sum();
                    (reach / c.deriv(t).d1.length()).min(0.25 * (p - own).max(0.0))
                }
                None => reach / c.deriv(t).d1.length(),
            };
            // an edge's range may lie past its first period
            let far = match (at_end == *fwd, d.period) {
                (true, Some(_)) => t + dt,
                (false, Some(_)) => t - dt,
                (true, None) => (t + dt).min(d.hi),
                (false, None) => (t - dt).max(d.lo),
            };
            let piece = (c.clone(), Interval::new(t.min(far), t.max(far)), *fwd);
            (c.eval(far).distance(c.eval(t)) > tol::LINEAR * per_edge as f64).then_some(piece)
        };
        let head = run_out(edges.first()?, false);
        let tail = run_out(edges.last()?, true);
        let first = usize::from(head.is_some());
        let open = head.is_some() || tail.is_some();
        let chain: Vec<_> = head
            .into_iter()
            .chain(edges.iter().cloned())
            .chain(tail)
            .collect();
        let per = per_edge.max(1);
        let mut data: Vec<[Vec3; 5]> = Vec::new();
        let mut ends = Vec::new();
        for (c, r, fwd) in &chain {
            let start = data.len().saturating_sub(1);
            for k in 0..=per {
                let f = k as f64 / per as f64;
                let mut e = edge_taylor(
                    c,
                    if *fwd {
                        r.lo + r.width() * f
                    } else {
                        r.hi - r.width() * f
                    },
                )?;
                if !fwd {
                    // walking back: odd arc-length derivatives change sign
                    e[1] = -e[1];
                    e[3] = -e[3];
                }
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
            ends.push(Interval::new(start as f64, (data.len() - 1) as f64));
        }
        // a closed chain closes smoothly: its first and last knots become one
        let n = data.len() - 1;
        if !open
            && n > 1
            && (data[0][0] - data[n][0]).length() <= tol::LINEAR
            && data[0][1].dot(data[n][1]) >= 1.0 - 1e-9
        {
            let last = data[n];
            for (x, y) in data[0].iter_mut().zip(last).skip(2) {
                *x = (*x + y) * 0.5;
            }
            data[n] = data[0];
        }
        let h = Hermite::new(&data)?;
        let s = |i: Interval| Interval::new(h.knots[i.lo as usize], h.knots[i.hi as usize]);
        let whole = Interval::new(s(ends[0]).lo, s(ends[ends.len() - 1]).hi);
        let ranges = ends[first..first + edges.len()].iter().map(|&i| s(i)).collect();
        Some((Guide::Hermite(Arc::new(h)), ranges, whole))
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
    /// The radius bends faster than the balls can roll: neighbouring balls cut into the arc
    /// (the envelope folds over its edge of regression).
    Folds {
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
            VarBlendError::Folds { s } => write!(f, "radius bends too sharply, the blend folds at s = {s}"),
        }
    }
}

impl std::error::Error for VarBlendError {}

/// One cross-section, as jets in s.
struct Section {
    c: V,
    r: J,
    k: V,
    rho: J,
    e1: V,
    e2: V,
    phi: J,
    /// Contacts at v = 0 and v = 1.
    q: [V; 2],
}

/// A cross-section's arc: k + rho (cos t e1 + sin t e2), t = v phi for v in [0, 1].
#[derive(Clone, Copy, Debug, PartialEq)]
struct SecArc {
    k: Point3,
    rho: f64,
    e1: Vec3,
    e2: Vec3,
    phi: f64,
}

impl SecArc {
    fn point(&self, v: f64) -> Point3 {
        let (sn, cs) = dmath::sin_cos(v * self.phi);
        self.k + (self.e1 * cs + self.e2 * sn) * self.rho
    }

    /// v of the point of the circle nearest p (unclamped; past the far end of an arc of more
    /// than half a turn where that end is nearer in angle).
    fn v_of(&self, p: Point3) -> f64 {
        let d = p - self.k;
        let mut t = dmath::atan2(d.dot(self.e2), d.dot(self.e1));
        if self.phi > dmath::PI && t < 0.0 && t + dmath::TAU - self.phi < -t {
            t += dmath::TAU;
        }
        t / self.phi
    }

    /// Box of the arc over v in [v0, v1].
    fn bbox(&self, v0: f64, v1: f64) -> Box3 {
        let (t0, t1) = (v0 * self.phi, v1 * self.phi);
        let mut b = Box3::from_points([self.point(v0), self.point(v1)]);
        for ax in 0..3 {
            let t = dmath::atan2(self.e2[ax], self.e1[ax]);
            // extremes of cos(t - t*) sit at t* + m pi
            let mut m = ((t0 - t) / dmath::PI).ceil();
            while t + m * dmath::PI <= t1 {
                b.include(self.point((t + m * dmath::PI) / self.phi));
                m += 1.0;
            }
        }
        b
    }
}

/// A solved cross-section, cached to seed solves and bound searches.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Row {
    s: f64,
    /// Taylor coefficients of the centre (the first as a vector from the origin) and radius.
    c: [Vec3; 4],
    rj: [f64; 4],
    r: f64,
    arc: SecArc,
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

impl Section {
    fn arc(&self) -> SecArc {
        SecArc {
            k: Point3::ORIGIN + self.k.at(0),
            rho: self.rho.0[0],
            e1: self.e1.at(0),
            e2: self.e2.at(0),
            phi: self.phi.0[0],
        }
    }
}

impl Row {
    /// The point of track k (see `Seg`).
    fn on(&self, k: usize) -> Point3 {
        if k == 2 {
            Point3::ORIGIN + self.c[0]
        } else {
            self.pts[2 * k]
        }
    }
}

/// Bounds over the stretch between two rows.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Seg {
    /// Bounds on |B_u|, and on the speed and acceleration of each track (the contacts at
    /// v = 0 and 1, then the centre).
    vb: f64,
    vq: [f64; 3],
    aq: [f64; 3],
    rmin: f64,
    rmax: f64,
}

/// A run of segments inside one ball about the centre at its first row.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Block {
    lo: usize,
    hi: usize,
    c: Point3,
    /// Every centre in the run lies within this of `c`, every point of the blend within this
    /// plus `rmax`.
    reach: f64,
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
    /// A surface (and the balls' side of it) the arc passes over between its contacts, the
    /// long way round where that is the way to it (a full round's middle face).
    through: Option<(Surface, f64)>,
    rows: Vec<Row>,
    segs: Vec<Seg>,
    /// Runs of segments under one ball, so searches skip what is far.
    blocks: Vec<Block>,
    /// How far the cached rows' Taylor model may stray from the solved centre between rows.
    predict: f64,
}

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
        Self::make([a, b], [side_a, side_b], None, guide, law, range)
    }

    /// The full round between `a` and `b` over `m`, the balls on side `sides[i]` of each: the
    /// ball at s touches all three, its radius solved with its centre, and its arc passes over
    /// m, by more than half a turn where need be. `start` seeds one ball (centre, radius)
    /// anywhere along the range.
    pub fn round(
        [a, m, b]: [&Surface; 3],
        sides: [f64; 3],
        guide: Guide,
        range: Interval,
        start: (Point3, f64),
    ) -> Result<Arc<VarBlend>, VarBlendError> {
        let law = Arc::new(RadiusLaw::constant(start.1));
        Self::make(
            [a, b],
            [sides[0], sides[2]],
            Some((m, sides[1], start.0)),
            guide,
            law,
            range,
        )
    }

    /// With `through` (a full round's middle, its side and the start centre), the law only
    /// seeds the start: the radius is solved, and the law becomes its fit.
    fn make(
        [a, b]: [&Surface; 2],
        [side_a, side_b]: [f64; 2],
        through: Option<(&Surface, f64, Point3)>,
        guide: Guide,
        law: Arc<RadiusLaw>,
        range: Interval,
    ) -> Result<Arc<VarBlend>, VarBlendError> {
        if [Some(a), Some(b), through.map(|t| t.0)]
            .iter()
            .any(|s| matches!(s, Some(Surface::Procedural(_))))
        {
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
            through: through.map(|(m, side, _)| (m.clone(), side.signum())),
            rows: Vec::new(),
            segs: Vec::new(),
            blocks: Vec::new(),
            predict: 0.0,
        };
        vb.build(through.map(|t| t.2))?;
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
        self.track(self.v_on(i) as usize)
    }

    /// The ball centre's path; its parameter is s.
    pub fn spine(self: &Arc<Self>) -> crate::Curve {
        self.track(2)
    }

    fn track(self: &Arc<Self>, k: usize) -> crate::Curve {
        crate::Curve::Procedural(ProcCurve(Arc::new(Track {
            blend: self.clone(),
            k,
        })))
    }

    /// The cross-section at s: its characteristic circle (exact), x towards the contact with a,
    /// and the angle at which it reaches the contact with b.
    pub fn section(&self, s: f64) -> (Circle, f64) {
        let SecArc {
            k,
            rho,
            mut e1,
            mut e2,
            phi,
        } = self.section_at(s).arc();
        if self.swap {
            // the arc from b to a, walked back
            let (sn, cs) = dmath::sin_cos(phi);
            (e1, e2) = (e1 * cs + e2 * sn, e1 * sn - e2 * cs);
        }
        let z = e1.cross(e2).normalize().unwrap_or(UnitVec3::Z);
        let frame = Frame::from_zx(k, z, e1).unwrap_or_else(|| Frame::from_z(k, z));
        (Circle { frame, radius: rho }, phi)
    }

    /// The ball at s: its centre and radius. The section at s is its circle of contact, along
    /// which the blend's normal is the ball's.
    pub fn ball(&self, s: f64) -> (Point3, f64) {
        let sec = self.section_at(s);
        (Point3::ORIGIN + sec.c.at(0), sec.r.0[0])
    }

    /// The sides of a and b the balls are on (+1 or -1).
    pub fn sides(&self) -> (f64, f64) {
        (self.side[0], self.side[1])
    }

    pub fn range(&self) -> Interval {
        Interval::new(self.lo, self.hi)
    }

    pub fn radius(&self, s: f64) -> f64 {
        match self.through {
            Some(_) => self.spine_at(s).1.0[0],
            None => self.law.eval(s),
        }
    }

    /// Smallest and largest radius over the range.
    pub fn radii(&self) -> (f64, f64) {
        self.law.bounds(self.lo, self.hi)
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
            let dx = Rows3::new(self.a.gradient(c), self.b.gradient(c), t)?.solve([-f[0], -f[1], -f[2]]);
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

    /// The ball at s: the centre for radius r, or with a through surface the centre and radius
    /// touching all three (Newton from `seed` and r).
    fn solve_ball(&self, s: f64, r: f64, seed: Point3) -> Option<(Point3, f64)> {
        let Some((m, side_m)) = &self.through else {
            return self.solve_centre(s, r, seed).map(|c| (c, r));
        };
        let (g, t) = self.guide.point_tan(s);
        let t = t.normalize()?.get();
        let res = |c: Point3, r: f64| {
            [
                self.a.implicit(c) - self.side[0] * r,
                self.b.implicit(c) - self.side[1] * r,
                m.implicit(c) - side_m * r,
                (c - g).dot(t),
            ]
        };
        let norm = |f: [f64; 4]| f.iter().fold(0.0f64, |m, x| m.max(x.abs()));
        let better = |n: [f64; 4], o: [f64; 4]| norm(n) <= norm(o) && !n.iter().any(|x| x.is_nan());
        let scale = 1.0 + r + seed.to_vec().abs_max();
        let (mut c, mut r) = (seed, r);
        let mut f = res(c, r);
        for _ in 0..40 {
            let row = |g: Vec3, side: f64| [g.x, g.y, g.z, -side];
            let d = Rows4::new([
                row(self.a.gradient(c), self.side[0]),
                row(self.b.gradient(c), self.side[1]),
                row(m.gradient(c), *side_m),
                row(t, 0.0),
            ])?
            .solve(f.map(|x| -x));
            let (dc, dr) = (Vec3::new(d[0], d[1], d[2]), d[3]);
            let len = dc.length().max(dr.abs());
            let mut lam = 1.0;
            let (mut cn, mut rn) = (c + dc, r + dr);
            let mut fnew = res(cn, rn);
            while !better(fnew, f) && lam > 1.0 / 64.0 && len > 1e-9 * scale {
                lam *= 0.5;
                (cn, rn) = (c + dc * lam, r + dr * lam);
                fnew = res(cn, rn);
            }
            if !better(fnew, f) && len > 1e-9 * scale {
                return None;
            }
            (c, r, f) = (cn, rn, fnew);
            if len * lam <= 1e-13 * scale {
                break;
            }
        }
        (norm(f) <= 1e-9 * scale && r > tol::MIN_LENGTH).then_some((c, r))
    }

    /// The guide parameter in [lo, hi] whose point lies nearest p (sampled, then narrowed).
    fn guide_nearest(&self, p: Point3) -> f64 {
        let n = 64;
        let at = |k: usize| self.lo + (self.hi - self.lo) * k as f64 / n as f64;
        let d = |s: f64| self.guide.point_tan(s).0.distance(p);
        let k = (0..=n)
            .min_by(|&i, &j| d(at(i)).total_cmp(&d(at(j))))
            .unwrap_or(0);
        let (mut a, mut b) = (at(k.saturating_sub(1)), at((k + 1).min(n)));
        for _ in 0..60 {
            let (x, y) = (a + (b - a) / 3.0, b - (b - a) / 3.0);
            if d(x) <= d(y) {
                b = y;
            } else {
                a = x;
            }
        }
        0.5 * (a + b)
    }

    /// A first guess of the radius at s, from row i: its fitted model with a through surface,
    /// else the law.
    fn r_at(&self, i: usize, s: f64) -> f64 {
        match self.through {
            Some(_) => {
                let (row, h) = (&self.rows[i], s - self.rows[i].s);
                let r = row.rj;
                r[0] + h * (r[1] + h * (r[2] + h * r[3]))
            }
            None => self.law.eval(s),
        }
    }

    /// The centre's (and with a through surface the radius's) Taylor coefficients at s from the
    /// ball (c0, r0), solved order by order.
    fn centre_jets(&self, s: f64, c0: Point3, r0: f64) -> Option<(V, J)> {
        self.centre_jets_n(s, c0, r0)
    }

    /// `centre_jets` to order N - 1 only.
    fn centre_jets_n<const N: usize>(&self, s: f64, c0: Point3, r0: f64) -> Option<(Vn<N>, Jn<N>)> {
        let (g4, gp4) = self.guide.jets(s);
        let (g, gp) = (
            Vn::<N>::from(std::array::from_fn(|k| g4.at(k))),
            Vn::<N>::from(std::array::from_fn(|k| gp4.at(k))),
        );
        if let Some((m, side_m)) = &self.through {
            let row = |g: Vec3, side: f64| [g.x, g.y, g.z, -side];
            let rows = Rows4::new([
                row(self.a.gradient(c0), self.side[0]),
                row(self.b.gradient(c0), self.side[1]),
                row(m.gradient(c0), *side_m),
                row(gp.at(0), 0.0),
            ])?;
            let (mut c, mut r) = (Vn::<N>::k(c0.to_vec()), Jn::<N>::k(r0));
            for i in 1..N {
                let fa = dist_grad(&self.a, c, false).0 - r * self.side[0];
                let fb = dist_grad(&self.b, c, false).0 - r * self.side[1];
                let fm = dist_grad(m, c, false).0 - r * *side_m;
                let fp = (c - g).dot(gp);
                let x = rows.solve([-fa.0[i], -fb.0[i], -fm.0[i], -fp.0[i]]);
                c.set(i, Vec3::new(x[0], x[1], x[2]));
                r.0[i] = x[3];
            }
            return Some((c, r));
        }
        let rj = self.law.jet(s);
        let r = Jn::<N>(std::array::from_fn(|k| rj.0[k]));
        let rows = Rows3::new(self.a.gradient(c0), self.b.gradient(c0), gp.at(0))?;
        let mut c = Vn::<N>::k(c0.to_vec());
        for i in 1..N {
            let fa = dist_grad(&self.a, c, false).0 - r * self.side[0];
            let fb = dist_grad(&self.b, c, false).0 - r * self.side[1];
            let fp = (c - g).dot(gp);
            c.set(i, rows.solve([-fa.0[i], -fb.0[i], -fp.0[i]]));
        }
        Some((c, r))
    }

    /// The characteristic circle's centre and radius, its frame (e1 to the v = 0 contact), the
    /// arc's angle and the contacts, from centre and radius jets of any order past the first.
    #[allow(clippy::type_complexity)]
    fn frame_of<const N: usize>(
        &self,
        c: Vn<N>,
        r: Jn<N>,
    ) -> (Vn<N>, Jn<N>, Vn<N>, Vn<N>, Jn<N>, [Vn<N>; 2]) {
        let cp = c.d();
        let rp = r.d();
        let g = cp.dot(cp).recip();
        let k = c - cp * (r * rp * g);
        let rho = r * (Jn::<N>::k(1.0) - rp * rp * g).sqrt();
        let qa = c - dist_grad(&self.a, c, true).1 * (r * self.side[0]);
        let qb = c - dist_grad(&self.b, c, true).1 * (r * self.side[1]);
        let q = if self.swap { [qb, qa] } else { [qa, qb] };
        let a = q[0] - k;
        let e1 = a * a.norm().recip();
        let b = q[1] - k;
        // e2 toward the second contact, or toward the surface the arc passes over
        let w = match &self.through {
            None => b,
            Some((m, side)) => {
                let d = c - dist_grad(m, c, true).1 * (r * *side) - k;
                d - cp * (d.dot(cp) * g)
            }
        };
        let bp = w - e1 * w.dot(e1);
        let e2 = bp * bp.norm().recip();
        let mut phi = Jn::<N>::atan2(b.dot(e2), b.dot(e1));
        if self.through.is_some() && phi.0[0] < 0.0 {
            phi.0[0] += dmath::TAU;
        }
        (k, rho, e1, e2, phi, q)
    }

    /// The section's arc alone, from centre and radius jets of any order past the first.
    fn arc_of<const N: usize>(&self, c: Vn<N>, r: Jn<N>) -> SecArc {
        let (k, rho, e1, e2, phi, _) = self.frame_of(c, r);
        SecArc {
            k: Point3::ORIGIN + k.at(0),
            rho: rho.0[0],
            e1: e1.at(0),
            e2: e2.at(0),
            phi: phi.0[0],
        }
    }

    /// The section's arc at s, solved to first order only (a point of the blend needs no more).
    fn arc_at(&self, s: f64) -> SecArc {
        let i = self.nearest_row(s);
        if self.rows[i].s == s {
            let (c, r) = (self.rows[i].c, self.rows[i].rj);
            return self.arc_of(Vn::<2>::from([c[0], c[1]]), Jn::<2>([r[0], r[1]]));
        }
        let seed = Point3::ORIGIN + self.model(i, s)[0];
        match self
            .solve_ball(s, self.r_at(i, s), seed)
            .and_then(|(c, r)| self.centre_jets_n::<2>(s, c, r))
        {
            Some((c, r)) => self.arc_of(c, r),
            None => self.section_at(s).arc(),
        }
    }

    fn section_of(&self, c: V, r: J) -> Section {
        let (k, rho, e1, e2, phi, q) = self.frame_of(c, r);
        Section {
            c,
            r,
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
        // a row holds the solved jets at its own s
        if self.rows[i].s == s {
            return (V::from(self.rows[i].c), Jn(self.rows[i].rj));
        }
        let seed = Point3::ORIGIN + self.model(i, s)[0];
        if let Some((c, r)) = self.solve_ball(s, self.r_at(i, s), seed)
            && let Some(j) = self.centre_jets(s, c, r)
        {
            return j;
        }
        let s0 = self.rows[i].s;
        let (mut c, mut r) = (Point3::ORIGIN + self.rows[i].c[0], self.rows[i].r);
        for k in 1..=16 {
            let t = s0 + (s - s0) * k as f64 / 16.0;
            let guess = if self.through.is_some() {
                r
            } else {
                self.law.eval(t)
            };
            match self.solve_ball(t, guess, c) {
                Some(n) => (c, r) = n,
                None => return (V::from(self.model(i, s)), self.law.jet(s)),
            }
        }
        self.centre_jets(s, c, r)
            .unwrap_or_else(|| (V::from(self.model(i, s)), self.law.jet(s)))
    }

    /// Jets of track k at s: the contact at v = k (0 or 1), or the centre (2).
    fn track_at(&self, s: f64, k: usize) -> V {
        let (c, r) = self.spine_at(s);
        self.track_of(c, r, k)
    }

    /// Track k's jets from centre and radius jets.
    fn track_of(&self, c: V, r: J, k: usize) -> V {
        if k == 2 {
            return c;
        }
        let (surf, side) = if (k == 1) != self.swap {
            (&self.b, self.side[1])
        } else {
            (&self.a, self.side[0])
        };
        c - dist_grad(surf, c, true).1 * (r * side)
    }

    fn section_at(&self, s: f64) -> Section {
        let (c, r) = self.spine_at(s);
        self.section_of(c, r)
    }

    fn check(&self, sec: &Section, s: f64) -> Result<(), VarBlendError> {
        let (rho, phi) = (sec.rho.0[0], sec.phi.0[0]);
        if rho.is_nan() || rho <= tol::LINEAR {
            return Err(VarBlendError::RadiusTooSteep { s });
        }
        // the contacts apart, the second off the first's diameter unless the arc goes round over
        // its through surface
        let apart = match self.through {
            None => (sec.q[1] - sec.k).dot(sec.e2).0[0],
            Some(_) => (dmath::TAU - phi) * rho,
        };
        if (phi * rho).is_nan() || phi * rho <= tol::LINEAR || apart.is_nan() || apart <= tol::LINEAR {
            return Err(VarBlendError::Contacts { s });
        }
        // the envelope is regular where F_ss > 0, F = |x - c|^2 - r^2: on the arc that is
        // |c'|^2 - r'^2 - r r'' - (x - c).c'' > 0, smallest where x - c leans most along c''
        let (c1, c2) = (sec.c.at(1), sec.c.at(2) * 2.0);
        let (r0, r1, r2) = (sec.r.0[0], sec.r.0[1], 2.0 * sec.r.0[2]);
        let (k, e1, e2) = (sec.k.at(0) - sec.c.at(0), sec.e1.at(0), sec.e2.at(0));
        let base = c1.length_sq() - r1 * r1 - r0 * r2 - k.dot(c2);
        let (a, b) = (e1.dot(c2), e2.dot(c2));
        let (sp, cp) = dmath::sin_cos(phi);
        let mut lean = a.max(a * cp + b * sp);
        if dmath::wrap_tau(dmath::atan2(b, a)) <= phi {
            lean = (a * a + b * b).sqrt();
        }
        let g = base - rho * lean;
        if g.is_nan() || g <= 0.0 {
            return Err(VarBlendError::Folds { s });
        }
        Ok(())
    }

    fn make_row(&self, s: f64, c: Point3, r: f64) -> Result<Row, VarBlendError> {
        let (cj, rj) = self.centre_jets(s, c, r).ok_or(VarBlendError::Tangent { s })?;
        let sec = self.section_of(cj, rj);
        self.check(&sec, s)?;
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
            rj: rj.0,
            r: rj.0[0],
            arc: sec.arc(),
            pts,
            bu,
            bv,
            kmax,
            q1: [a.0, b.0],
            q2: [a.1, b.1],
        })
    }

    /// The rows from `lo` to `hi`, from the ball at `lo` followed from one near `start` (with
    /// a through surface) or grown from the guide point.
    fn build(&mut self, start: Option<Point3>) -> Result<(), VarBlendError> {
        let lo = self.lo;
        let mut r = self.law.eval(lo);
        let mut c = self.guide.point_tan(lo).0;
        match start {
            Some(seed) => {
                let s0 = self.guide_nearest(seed);
                (c, r) = self.solve_ball(s0, r, seed).ok_or(VarBlendError::NoStart)?;
                for k in 1..=16 {
                    let t = s0 + (lo - s0) * k as f64 / 16.0;
                    (c, r) = self.solve_ball(t, r, c).ok_or(VarBlendError::Lost { s: t })?;
                }
            }
            // the ball grows from nothing at the edge: radius continuation from the guide point
            None => {
                for k in (0..=12).rev() {
                    let lam = 1.0 / (1u32 << k) as f64;
                    c = self.solve_centre(lo, r * lam, c).ok_or(VarBlendError::NoStart)?;
                }
            }
        }
        // orient: du x dv out of the ball
        let (cj, rj) = self
            .centre_jets(lo, c, r)
            .ok_or(VarBlendError::Tangent { s: lo })?;
        let sec = self.section_of(cj, rj);
        self.check(&sec, lo)?;
        let d = Self::point(&sec, 0.5);
        if d.du.cross(d.dv).dot(d.p - (Point3::ORIGIN + cj.at(0))) < 0.0 {
            self.swap = true;
        }
        self.rows.push(self.make_row(lo, c, r)?);
        let span = self.hi - self.lo;
        // rows only seed solves and bound searches; a thousandth of the smallest radius
        self.predict = (1e-3 * self.radii().0).max(tol::LINEAR * 0.1);
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
            let guess = self.r_at(i, s);
            let next = self.solve_ball(s, guess, pred);
            let err = next.map_or(f64::INFINITY, |(c, r)| c.distance(pred) + (r - guess).abs());
            let moved = next.map_or(f64::INFINITY, |(c, _)| (c.to_vec() - last.c[0]).length());
            let r = next.map_or(guess, |b| b.1);
            if err.is_nan() || err > self.predict || moved > 2.0 * last.r.min(r) {
                h *= 0.5;
                continue;
            }
            let (c, r) = next.expect("checked above");
            let row = self.make_row(s, c, r)?;
            // the section checks between rows too, where a fold could hide
            let mid = 0.5 * (last.s + s);
            let mc = self.solve_ball(mid, self.r_at(i, mid), Point3::ORIGIN + self.model(i, mid)[0]);
            let (cj, rj) = mc
                .and_then(|(c, r)| self.centre_jets(mid, c, r))
                .ok_or(VarBlendError::Tangent { s: mid })?;
            self.check(&self.section_of(cj, rj), mid)?;
            self.rows.push(row);
            if err < self.predict / 16.0 {
                h = (h * 1.5).min(span / 4.0);
            }
        }
        if self.through.is_some() {
            // the solved radii, for bounds and seeds
            let pts: Vec<(f64, f64, f64)> = self.rows.iter().map(|r| (r.s, r.rj[0], r.rj[1])).collect();
            self.law = Arc::new(RadiusLaw::hermite(&pts));
        }
        let pad = if self.through.is_some() { self.predict } else { 0.0 };
        self.segs = (0..self.rows.len() - 1)
            .map(|i| {
                let (r0, r1) = (&self.rows[i], &self.rows[i + 1]);
                let mid = 0.5 * (r0.s + r1.s);
                let vc = r0.c[1]
                    .length()
                    .max(r1.c[1].length())
                    .max(self.model(i, mid)[1].length())
                    .max(self.model(i + 1, mid)[1].length());
                let ac = (r0.c[2].length() * 2.0)
                    .max(r1.c[2].length() * 2.0)
                    .max(self.model(i, mid)[2].length() * 2.0)
                    .max(self.model(i + 1, mid)[2].length() * 2.0);
                let (rmin, rmax) = self.law.bounds(r0.s, r1.s);
                let (rmin, rmax) = (rmin - pad, rmax + pad);
                Seg {
                    vb: 1.5 * r0.bu.max(r1.bu),
                    vq: [
                        1.5 * r0.q1[0].max(r1.q1[0]),
                        1.5 * r0.q1[1].max(r1.q1[1]),
                        1.25 * vc,
                    ],
                    aq: [
                        1.5 * r0.q2[0].max(r1.q2[0]),
                        1.5 * r0.q2[1].max(r1.q2[1]),
                        1.5 * ac,
                    ],
                    rmin,
                    rmax,
                }
            })
            .collect();
        self.blocks = self.make_blocks();
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

    /// Points in segment i where h'(s) = 0 on the cached model (bracketed), with the model's h
    /// there, appended to `out`.
    fn roots(&self, p: Point3, i: usize, out: &mut Vec<(f64, f64)>) {
        let (s0, s1) = (self.rows[i].s, self.rows[i + 1].s);
        let tube = |s: f64| {
            let j = if s - s0 <= s1 - s { i } else { i + 1 };
            Self::tube(p, &self.model(j, s), &self.law.coefs(s))
        };
        const N: usize = 4;
        let xs: [f64; N + 1] = std::array::from_fn(|k| s0 + (s1 - s0) * k as f64 / N as f64);
        let ys: [f64; N + 1] = std::array::from_fn(|k| tube(xs[k]).1);
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
                let fs = tube(s).1;
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
            out.push((s, tube(s).0));
        }
    }

    /// A model root polished against the exact centre: a solve, then Newton on h'(s) with the
    /// solved jets carried along, again from a fresh solve while the steps are not tiny (only
    /// then is the carried Taylor series exact to rounding); kept inside the range.
    fn polish(&self, p: Point3, mut s: f64) -> (f64, V, J) {
        for _ in 0..3 {
            let (c, mut r) = self.spine_at(s);
            let mut c = c.coefs();
            let s_at = s;
            for _ in 0..3 {
                let (_, h1, h2) = Self::tube(p, &c, &r.0);
                if h2.abs() < 1e-300 {
                    break;
                }
                let next = (s - h1 / h2).clamp(self.lo, self.hi);
                if next == s {
                    break;
                }
                let done = (next - s).abs() <= 1e-14 * (1.0 + s.abs());
                let h = next - s;
                c = [
                    c[0] + (c[1] + (c[2] + c[3] * h) * h) * h,
                    c[1] + (c[2] * 2.0 + c[3] * (3.0 * h)) * h,
                    c[2] + c[3] * (3.0 * h),
                    c[3],
                ];
                s = next;
                r = match self.through {
                    Some(_) => Jn([
                        r.0[0] + h * (r.0[1] + h * (r.0[2] + h * r.0[3])),
                        r.0[1] + h * (2.0 * r.0[2] + 3.0 * h * r.0[3]),
                        r.0[2] + 3.0 * h * r.0[3],
                        r.0[3],
                    ]),
                    None => self.law.jet(s),
                };
                if done {
                    break;
                }
            }
            if (s - s_at).abs() <= 1e-2 * self.predict {
                return (s, V::from(c), r);
            }
        }
        let (c, r) = self.spine_at(s);
        (s, c, r)
    }

    /// Lower bounds on |p - c(s)| from below and above, over segment i.
    fn seg_dist(&self, p: Point3, i: usize) -> (f64, f64) {
        let (r0, r1) = (&self.rows[i], &self.rows[i + 1]);
        let (a, b) = (Point3::ORIGIN + r0.c[0], Point3::ORIGIN + r1.c[0]);
        // the centre strays from its chord by at most |c''| h^2 / 8
        let h = r1.s - r0.s;
        let sag = self.segs[i].aq[2] * h * h * 0.125;
        (chord_dist(p, a, b) - sag, p.distance(a).max(p.distance(b)) + sag)
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
        let mut cands = Vec::new();
        for i in self.near_segs(p, best.0) {
            if self.seg_dist(p, i).0 - self.segs[i].rmax < best.0 {
                self.roots(p, i, &mut cands);
            }
        }
        // the model is within `predict` of the solved centre, so only roots it puts near the
        // best need solving
        cands.sort_by(|a, b| a.1.total_cmp(&b.1));
        for (s, hm) in cands {
            if hm - 2.0 * self.predict >= best.0 {
                break;
            }
            let (s, c, r) = self.polish(p, s);
            let cp = Point3::ORIGIN + c.at(0);
            let h = p.distance(cp) - r.0[0];
            if h < best.0 {
                best = (h, s, cp);
            }
        }
        best
    }

    /// Parameter of the nearest point on track k over `range`, and its distance; None when
    /// nothing there is nearer than `bound`.
    fn track_closest(&self, k: usize, p: Point3, range: Interval, bound: f64) -> Option<(f64, f64)> {
        let (lo, hi) = (
            range.lo.max(self.lo),
            range.hi.min(self.hi).max(range.lo.max(self.lo)),
        );
        let q = |s: f64| self.track_at(s, k);
        let mut best = (bound, f64::NAN);
        for s in [lo, hi] {
            // the range ends are usually rows already
            let i = self.nearest_row(s);
            let at = if self.rows[i].s == s {
                self.rows[i].on(k)
            } else {
                Point3::ORIGIN + q(s).at(0)
            };
            let d = p.distance(at);
            if d < best.0 {
                best = (d, s);
            }
        }
        for r in &self.rows {
            let d = p.distance(r.on(k));
            if r.s >= lo && r.s <= hi && d < best.0 {
                best = (d, r.s);
            }
        }
        // segments nearest first, so the best so far prunes the rest
        let mut order: Vec<(f64, usize)> = self
            .near_segs(p, best.0)
            .filter(|&i| self.rows[i + 1].s.min(hi) > self.rows[i].s.max(lo))
            .map(|i| {
                let h = self.rows[i + 1].s - self.rows[i].s;
                let lb = chord_dist(p, self.rows[i].on(k), self.rows[i + 1].on(k))
                    - self.segs[i].aq[k] * h * h * 0.125;
                (lb, i)
            })
            .filter(|x| x.0 < best.0)
            .collect();
        order.sort_by(|x, y| x.0.total_cmp(&y.0));
        for (lb, i) in order {
            let (a, b) = (self.rows[i].s.max(lo), self.rows[i + 1].s.min(hi));
            if lb >= best.0 {
                break;
            }
            // g(s) = (q - p) . q', a minimum where g goes from - to +; first on the rows'
            // model of the centre (within `predict` of it), then on the solved track
            let g_of = |j: V| {
                let (d, q1, q2) = (Point3::ORIGIN + j.at(0) - p, j.at(1), j.at(2) * 2.0);
                (d.dot(q1), q1.length_sq() + d.dot(q2), d.length())
            };
            let g = |s: f64| g_of(q(s));
            let gm = |s: f64| {
                let j = if s - self.rows[i].s <= self.rows[i + 1].s - s {
                    i
                } else {
                    i + 1
                };
                g_of(self.track_of(V::from(self.model(j, s)), self.law.jet(s), k))
            };
            let (mut a, mut b) = (a, b);
            let (ga, gb) = (g(a).0, g(b).0);
            if !(ga < 0.0 && gb > 0.0) {
                continue;
            }
            let mut s = 0.5 * (a + b);
            let (ma, mb) = (a, b);
            for _ in 0..60 {
                let (gs, dg, _) = gm(s);
                let n = s - gs / dg;
                if !(dg > 0.0 && n > ma && n < mb) {
                    break;
                }
                let done = (n - s).abs() <= 1e-12 * (1.0 + s.abs());
                s = n;
                if done {
                    break;
                }
            }
            for _ in 0..60 {
                let (gs, dg, _) = g(s);
                let n = s - gs / dg;
                // converged: a step at rounding (gs may be exactly zero)
                if dg > 0.0 && (n - s).abs() <= 1e-12 * (1.0 + s.abs()) {
                    s = n.clamp(a, b);
                    break;
                }
                if gs < 0.0 {
                    a = s;
                } else {
                    b = s;
                }
                let next = if dg > 0.0 && n > a && n < b {
                    n
                } else {
                    0.5 * (a + b)
                };
                if b - a <= 1e-15 * (1.0 + s.abs()) {
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
        (!best.1.is_nan()).then_some((best.1, best.0))
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
        let mut cands = Vec::new();
        for i in self.near_segs(p, best.0) {
            let (dlo, dhi) = self.seg_dist(p, i);
            let lb = (dlo - self.segs[i].rmax).max(self.segs[i].rmin - dhi);
            if lb < best.0 {
                self.roots(p, i, &mut cands);
            }
        }
        cands.sort_by(|a, b| a.1.abs().total_cmp(&b.1.abs()));
        let mut exact = false;
        for (s, hm) in cands {
            if hm.abs() - 2.0 * self.predict >= best.0 {
                break;
            }
            let (s, c, r) = self.polish(p, s);
            let v = self.section_of(c, r).arc().v_of(p);
            let d = (p.distance(Point3::ORIGIN + c.at(0)) - r.0[0]).abs();
            if (0.0..=1.0).contains(&v) && d < best.0 {
                take(d, s, v, &mut best);
                exact = true;
            }
        }
        // the end arcs
        for r in [&self.rows[0], self.rows.last().expect("rows are non-empty")] {
            let v = r.arc.v_of(p);
            if (0.0..=1.0).contains(&v) {
                let d = p.distance(r.arc.point(v));
                if d < best.0 {
                    take(d, r.s, v, &mut best);
                    exact = true;
                }
            }
        }
        // the springs
        for v in 0..2 {
            if let Some((s, d)) = self.track_closest(v, p, Interval::new(self.lo, self.hi), best.0) {
                take(d, s, v as f64, &mut best);
                exact = true;
            }
        }
        if exact {
            return best.1;
        }
        // only a grid point won: Newton on |B - p|^2 inside the domain, while it gets closer
        let mut uv = best.1;
        let mut d = Procedural::deriv(self, uv);
        let mut d0 = d.p.distance(p);
        for _ in 0..8 {
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
            let dn = Procedural::deriv(self, next);
            let l = dn.p.distance(p);
            if l.is_nan() || l >= d0 {
                break;
            }
            (uv, d, d0) = (next, dn, l);
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
                arc: SecArc {
                    k: pw(r.arc.k),
                    e1: vw(r.arc.e1),
                    e2: vw(r.arc.e2),
                    ..r.arc
                },
                pts: r.pts.map(pw),
                ..*r
            })
            .collect();
        let mut out = VarBlend {
            a: self.a.transformed(f),
            b: self.b.transformed(f),
            through: self.through.as_ref().map(|(m, side)| (m.transformed(f), *side)),
            guide: self.guide.transformed(f),
            rows,
            ..self.clone()
        };
        out.blocks = out.make_blocks();
        out
    }

    /// Blocks of eight segments, each a ball holding its centres and its largest radius.
    fn make_blocks(&self) -> Vec<Block> {
        (0..self.segs.len())
            .step_by(8)
            .map(|lo| {
                let hi = (lo + 8).min(self.segs.len());
                let c = Point3::ORIGIN + self.rows[lo].c[0];
                let mut reach = 0.0f64;
                let mut rmax = 0.0f64;
                for i in lo..hi {
                    let h = self.rows[i + 1].s - self.rows[i].s;
                    let sag = self.segs[i].aq[2] * h * h * 0.125;
                    let far = (Point3::ORIGIN + self.rows[i + 1].c[0]).distance(c);
                    reach = reach.max(far + sag);
                    rmax = rmax.max(self.segs[i].rmax);
                }
                Block {
                    lo,
                    hi,
                    c,
                    reach: reach * (1.0 + 1e-12) + tol::LINEAR,
                    rmax,
                }
            })
            .collect()
    }

    /// The segments that may hold a point of the blend, a spring or the centre within `bound`
    /// of p, block by block.
    fn near_segs(&self, p: Point3, bound: f64) -> impl Iterator<Item = usize> + '_ {
        self.blocks
            .iter()
            .filter(move |b| p.distance(b.c) - b.reach - b.rmax < bound)
            .flat_map(|b| b.lo..b.hi)
    }

    /// Arcs bounding [s0, s1], each with its s and segment: exact sections at the ends, cached
    /// rows between.
    fn arcs_over(&self, s0: f64, s1: f64) -> Vec<(f64, SecArc, usize)> {
        let last = self.segs.len() - 1;
        let at = |s: f64| {
            let i = self.rows.partition_point(|r| r.s <= s).clamp(1, last + 1) - 1;
            (s, self.section_at(s).arc(), i)
        };
        let mut out = vec![at(s0)];
        for (i, r) in self.rows.iter().enumerate() {
            if r.s > s0 && r.s < s1 {
                out.push((r.s, r.arc, i.min(last)));
            }
        }
        if s1 > s0 {
            out.push(at(s1));
        }
        out
    }
}

/// Distance from p to the segment ab.
fn chord_dist(p: Point3, a: Point3, b: Point3) -> f64 {
    let ab = b - a;
    let l = ab.length_sq();
    let t = if l > 0.0 {
        ((p - a).dot(ab) / l).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
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

impl Procedural for VarBlend {
    fn deriv(&self, uv: Point2) -> SurfDeriv {
        VarBlend::point(&self.section_at(uv.x), uv.y)
    }
    fn point(&self, uv: Point2) -> Point3 {
        self.arc_at(uv.x).point(uv.y)
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
        let arcs = self.arcs_over(s0, s1);
        let mut b = arcs[0].1.bbox(v0, v1);
        // the slack allowed between exact sections: a hundredth of the smallest radius
        let slack = 1e-2 * self.radii().0;
        for w in arcs.windows(2) {
            // a point between two sections is within half the gap times |B_u| of one of them;
            // where that is loose, more exact sections close it up
            let vb = self.segs[w[0].2.max(w[1].2.saturating_sub(1))]
                .vb
                .max(self.segs[w[0].2].vb);
            let gap = w[1].0 - w[0].0;
            let m = ((vb * gap * 0.5 / slack).ceil() as usize).clamp(1, 64);
            let mut prev = w[0].1;
            for k in 1..=m {
                let next = if k == m {
                    w[1].1
                } else {
                    self.arc_at(w[0].0 + gap * k as f64 / m as f64)
                };
                let grow = vb * gap / m as f64 * 0.5;
                b = b.hull(&prev.bbox(v0, v1).hull(&next.bbox(v0, v1)).grow(grow));
                prev = next;
            }
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
        let rmin = self.radii().0;
        0.8 * rmin.min(if k > 0.0 { 1.0 / k } else { f64::INFINITY })
    }
}

/// A curve the ball carries along s: a spring curve where it touches a surface, or its centre.
#[derive(Debug)]
struct Track {
    blend: Arc<VarBlend>,
    /// 0: the v = 0 edge, 1: the v = 1 edge, 2: the centre.
    k: usize,
}

impl Track {
    fn segs_over(&self, range: Interval) -> impl Iterator<Item = (usize, f64)> + '_ {
        let rows = &self.blend.rows;
        (0..self.blend.segs.len()).filter_map(move |i| {
            let w = rows[i + 1].s.min(range.hi) - rows[i].s.max(range.lo);
            (w >= 0.0).then_some((i, w))
        })
    }
}

impl ProceduralCurve for Track {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let q = self.blend.track_at(t, self.k);
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
        let b = &self.blend;
        b.track_closest(self.k, p, range, f64::INFINITY)
            .map_or(range.lo.clamp(b.lo, b.hi), |r| r.0)
    }
    fn bbox(&self, range: Interval) -> Box3 {
        let b = &self.blend;
        let s0 = range.lo.clamp(b.lo, b.hi);
        let s1 = range.hi.clamp(s0, b.hi);
        let q = |s: f64| Point3::ORIGIN + b.track_at(s, self.k).at(0);
        let mut pts = vec![(s0, q(s0))];
        pts.extend(
            b.rows
                .iter()
                .filter(|r| r.s > s0 && r.s < s1)
                .map(|r| (r.s, r.on(self.k))),
        );
        if s1 > s0 {
            pts.push((s1, q(s1)));
        }
        // between points, within half the gap times the speed bound; where that is loose,
        // more exact points close it up to a hundredth of the radius
        let slack = 1e-2 * b.radii().0;
        let mut out = Box3::from_point(pts[0].1);
        for w in pts.windows(2) {
            let i = b
                .rows
                .partition_point(|r| r.s <= 0.5 * (w[0].0 + w[1].0))
                .clamp(1, b.segs.len())
                - 1;
            let gap = w[1].0 - w[0].0;
            let v = b.segs[i].vq[self.k];
            let m = ((v * gap * 0.5 / slack).ceil() as usize).clamp(1, 64);
            let mut prev = w[0].1;
            for j in 1..=m {
                let next = if j == m {
                    w[1].1
                } else {
                    q(w[0].0 + gap * j as f64 / m as f64)
                };
                out = out.hull(&Box3::from_points([prev, next]).grow(v * gap / m as f64 * 0.5));
                prev = next;
            }
        }
        out.grow(tol::LINEAR)
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(Track {
            blend: Arc::new(self.blend.transformed_blend(f)),
            k: self.k,
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        self.segs_over(range)
            .map(|(i, _)| self.blend.segs[i].vq[self.k])
            .fold(0.0, f64::max)
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        // equal steps h keep each chord within |q''| h^2 / 8 of the curve
        let a = self
            .segs_over(range)
            .map(|(i, _)| self.blend.segs[i].aq[self.k])
            .fold(0.0, f64::max);
        let n = range.width() * (a / (8.0 * chord.max(tol::LINEAR))).sqrt();
        (n.ceil() as usize).max(1)
    }
}

/// Where a variable blend meets an analytic surface across its arc, as a curve of v (the
/// fraction of the arc): at each v the s putting B(s, v) on the surface is solved by Newton
/// along u, so points are exact. A cut that doubles back in v is not one of these.
#[derive(Debug)]
pub struct Cut {
    blend: Arc<VarBlend>,
    surface: Surface,
    /// (v, s) along the cut, increasing in v: seeds for the solves and bounds for boxes.
    knots: Vec<(f64, f64)>,
    /// Largest |ds/dv| seen between knots, padded.
    slope: f64,
}

impl VarBlend {
    /// The point of the blend at v on `surface` near s: (s, the blend's derivatives there).
    fn cut_at(&self, surface: &Surface, v: f64, mut s: f64) -> Option<(f64, SurfDeriv)> {
        let scale = 1.0 + self.radii().1;
        for _ in 0..40 {
            let d = Procedural::deriv(self, Point2::new(s, v));
            let g = surface.implicit(d.p);
            let gu = surface.gradient(d.p).dot(d.du);
            if g.abs() <= 1e-13 * scale {
                return Some((s, d));
            }
            if gu.abs() <= 1e-12 * d.du.length().max(1e-300) {
                return None;
            }
            let next = (s - g / gu).clamp(self.lo, self.hi);
            if next == s {
                return (g.abs() <= 1e-3 * tol::LINEAR).then_some((s, d));
            }
            s = next;
        }
        let d = Procedural::deriv(self, Point2::new(s, v));
        (surface.implicit(d.p).abs() <= 1e-3 * tol::LINEAR).then_some((s, d))
    }

    /// The cut of the blend by analytic `surface` from `a` to `b` (both on it), with the
    /// parameters (v) of a and b; None where it is not a function of v between them.
    pub fn cut(
        self: &Arc<Self>,
        surface: &Surface,
        a: Point3,
        b: Point3,
    ) -> Option<(crate::Curve, f64, f64)> {
        if matches!(surface, Surface::Procedural(_)) {
            return None;
        }
        let (ua, ub) = (self.nearest(a), self.nearest(b));
        let n = 16;
        let mut knots = Vec::with_capacity(n + 1);
        let mut s = ua.x;
        for i in 0..=n {
            let v = ua.y + (ub.y - ua.y) * i as f64 / n as f64;
            let (si, d) = self.cut_at(surface, v, s)?;
            // a function of v: the surface crosses the blend's u lines transversally
            let n_s = surface.gradient(d.p);
            if n_s.dot(d.du).abs() <= 1e-3 * d.du.length() {
                return None;
            }
            s = si;
            knots.push((v, s));
        }
        let ends = |p: Point3, k: (f64, f64)| {
            Procedural::deriv(&**self, Point2::new(k.1, k.0)).p.distance(p) <= tol::LINEAR
        };
        if !ends(a, knots[0]) || !ends(b, knots[n]) {
            return None;
        }
        if ub.y < ua.y {
            knots.reverse();
        }
        let slope = knots
            .windows(2)
            .map(|w| ((w[1].1 - w[0].1) / (w[1].0 - w[0].0)).abs())
            .fold(0.0, f64::max);
        let cut = Cut {
            blend: self.clone(),
            surface: surface.clone(),
            knots,
            slope: 2.0 * slope + 1e-9,
        };
        Some((crate::Curve::Procedural(ProcCurve(Arc::new(cut))), ua.y, ub.y))
    }
}

impl Cut {
    /// s at v, interpolated between the knots (a seed).
    fn seed(&self, v: f64) -> f64 {
        let k = &self.knots;
        let i = k.partition_point(|x| x.0 <= v).clamp(1, k.len() - 1);
        let (a, b) = (k[i - 1], k[i]);
        a.1 + (b.1 - a.1) * (v - a.0) / (b.0 - a.0)
    }
}

impl ProceduralCurve for Cut {
    fn deriv(&self, t: f64) -> CurveDeriv {
        let b = &self.blend;
        let Some((_, d)) = b.cut_at(&self.surface, t, self.seed(t)) else {
            let d = Procedural::deriv(&**b, Point2::new(self.seed(t), t));
            return CurveDeriv {
                p: d.p,
                d1: d.dv,
                d2: d.dvv,
            };
        };
        // g(B(s(v), v)) = 0: s' from the gradient; s'' leaving out the surface's own curvature
        let n = self.surface.gradient(d.p);
        let nu = n.dot(d.du);
        let sp = -n.dot(d.dv) / nu;
        let rest = d.duu * (sp * sp) + d.duv * (2.0 * sp) + d.dvv;
        let spp = -n.dot(rest) / nu;
        CurveDeriv {
            p: d.p,
            d1: d.du * sp + d.dv,
            d2: rest + d.du * spp,
        }
    }
    fn domain(&self) -> Domain1 {
        Domain1 {
            lo: self.knots[0].0,
            hi: self.knots[self.knots.len() - 1].0,
            period: None,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        let m = 16;
        let at = |i: usize| (range.lo + range.width() * i as f64 / m as f64).min(range.hi);
        let mut best = (f64::INFINITY, 0);
        for i in 0..=m {
            let d = self.deriv(at(i)).p.distance_sq(p);
            if d < best.0 {
                best = (d, i);
            }
        }
        let (lo, hi) = (at(best.1.saturating_sub(1)), at((best.1 + 1).min(m)));
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
        if self.deriv(t).p.distance_sq(p) <= best.0 {
            t
        } else {
            at(best.1)
        }
    }
    fn bbox(&self, range: Interval) -> Box3 {
        // exact points, close enough that the speed bound leaves a hundredth of the radius
        let v = self.speed_bound(range);
        let slack = 1e-2 * self.blend.radii().0;
        let n = ((v * range.width() / (2.0 * slack)).ceil() as usize).clamp(1, 256);
        let at = |i: usize| (range.lo + range.width() * i as f64 / n as f64).min(range.hi);
        let b = (0..=n).fold(Box3::EMPTY, |b, i| b.hull(&Box3::from_point(self.deriv(at(i)).p)));
        b.grow(v * range.width() / n as f64 * 0.5 + tol::LINEAR)
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(Cut {
            blend: Arc::new(self.blend.transformed_blend(f)),
            surface: self.surface.transformed(f),
            knots: self.knots.clone(),
            slope: self.slope,
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        // the rows' largest |B_u| and |B_v|, padded as the tracks pad theirs
        let _ = range;
        let (bu, bv) = Procedural::uv_scale(&*self.blend);
        1.5 * (bu * self.slope + bv)
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
        // as a fillet builds it: the law set on the edge, the blend running out 1.0 past its ends
        let (g, ranges, whole) =
            Guide::through_edges(&[(edge, Interval::new(0.05 * hi, 0.3 * hi), true)], 8, 1.0).unwrap();
        let r = ranges[0];
        assert!(whole.lo < r.lo - 0.9 && whole.hi > r.hi + 0.9, "{whole:?} {r:?}");
        let law = RadiusLaw::smooth(&[(r.lo, 0.8), (r.mid(), 1.6), (r.hi, 1.0)], None).unwrap();
        make("cylinder-cylinder", a, 1.0, b, 1.0, g, law, whole.lo, whole.hi)
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

    fn track(b: &Arc<VarBlend>, k: usize) -> Track {
        Track { blend: b.clone(), k }
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
        let cases = cases();
        // both contact orders are covered
        assert!(cases.iter().any(|c| c.blend.swap) && cases.iter().any(|c| !c.blend.swap));
        for c in cases {
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
                // what a fillet reads: the centre curve, and the section from a's contact to b's
                assert!(b.spine().eval(s).distance(ctr) < 1e-12, "{} spine", c.name);
                let (circle, angle) = b.section(s);
                assert!(angle > 0.0 && angle < dmath::PI, "{} angle", c.name);
                for (i, t) in [0.0, angle].into_iter().enumerate() {
                    assert!(
                        circle.eval(t).distance(b.spring(i).eval(s)) < 1e-9,
                        "{} section {i}",
                        c.name
                    );
                }
                let mid = circle.eval(0.5 * angle);
                assert!(mid.distance(Procedural::deriv(&**b, Point2::new(s, 0.5)).p) < 1e-9);
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
                let sp = track(&c.blend, 0);
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
                let sp = track(&c.blend, 1);
                let cb = sp.bbox(bx.u);
                for i in 0..=20 {
                    let t = bx.u.lo + bx.u.width() * i as f64 / 20.0;
                    assert!(cb.contains(sp.deriv(t).p), "{} spring box", c.name);
                }
            }
        }
    }

    fn scale_of(b: &VarBlend) -> f64 {
        1.0 + b.rows[0].c[0].abs_max()
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
    fn spring_and_centre_curves_project_and_segment() {
        let mut r = Rng::new(8);
        for c in cases() {
            for k in 0..3 {
                let sp = track(&c.blend, k);
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
            (line, Interval::new(0.0, 10.0), true),
            (
                arc.clone(),
                Interval::new(-dmath::FRAC_PI_2, -dmath::FRAC_PI_2 + 1.0),
                true,
            ),
        ];
        let (g, ranges, _) = Guide::through_edges(&edges, 6, 0.0).unwrap();
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
        // the same chain with the line walked against its parameter, carried on past both ends
        let back = Curve::Line(Line::new(Point3::ORIGIN, -UnitVec3::X));
        let edges = [
            (back, Interval::new(0.0, 10.0), false),
            (
                arc,
                Interval::new(-dmath::FRAC_PI_2, -dmath::FRAC_PI_2 + 1.0),
                true,
            ),
        ];
        let (g2, ranges2, whole) = Guide::through_edges(&edges, 6, 3.0).unwrap();
        for k in 0..=10 {
            let f = k as f64 / 10.0;
            let (p, q) = (
                a.guide.point_tan(r0.lo + f * (r1.hi - r0.lo)),
                g2.point_tan(ranges2[0].lo + f * (ranges2[1].hi - ranges2[0].lo)),
            );
            assert!(p.0.distance(q.0) < 1e-9 && (p.1 - q.1).length() < 1e-9, "{f}");
        }
        assert!((ranges2[0].lo - whole.lo - 3.0).abs() < 1e-9);
        assert!(g2.point_tan(whole.lo).0.distance(Point3::new(-13.0, 0.0, 0.0)) < 1e-9);
        let end = g2.point_tan(whole.hi).0 - Point3::new(0.0, big, 0.0);
        assert!((end.length() - big).abs() < 1e-6, "{}", end.length());
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

    /// A random blend in a random place: a plane against a plane, cylinder, sphere, cone or
    /// torus, or two crossing cylinders on a Hermite guide, with a random radius law. Picks
    /// with no blend must fail cleanly.
    fn random_blend(r: &mut Rng) -> (u64, Result<Arc<VarBlend>, VarBlendError>) {
        let f = crate::testing::rand_frame(r);
        let floor = plane(Point3::ORIGIN, Vec3::Z);
        let kind = r.below(6);
        let pm = |r: &mut Rng| if r.bool() { 1.0 } else { -1.0 };
        let arc = |r: &mut Rng| {
            let lo = r.range(0.0, 6.0);
            Interval::new(lo, lo + r.range(0.3, 4.0))
        };
        let (a, b, side, guide, size, range) = match kind {
            0 => {
                let t = r.range(-1.3, 1.3);
                let n = Vec3::new(dmath::cos(t), 0.0, dmath::sin(t));
                let range = Interval::new(r.range(-10.0, 0.0), r.range(1.0, 10.0));
                (floor, plane(Point3::ORIGIN, n), 1.0, line_y(), 6.0, range)
            }
            1 => {
                let rad = r.range(3.0, 20.0);
                let cyl = Surface::Cylinder(Cylinder {
                    frame: Frame::WORLD,
                    radius: rad,
                });
                (floor, cyl, pm(r), circle(0.0, rad), rad, arc(r))
            }
            2 => {
                let rho = r.range(3.0, 20.0);
                let zc = r.range(-0.7, 0.7) * rho;
                let sph = Surface::Sphere(Sphere {
                    frame: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, zc)),
                    radius: rho,
                });
                let g = circle(0.0, (rho * rho - zc * zc).sqrt());
                (floor, sph, pm(r), g, rho, arc(r))
            }
            3 => {
                let rad = r.range(3.0, 15.0);
                let up = r.bool();
                let z = if up { UnitVec3::Z } else { -UnitVec3::Z };
                let cone = Surface::Cone(
                    Cone::new(Frame::from_z(Point3::ORIGIN, z), rad, r.range(0.2, 1.0)).unwrap(),
                );
                (
                    floor,
                    cone,
                    if up { -1.0 } else { 1.0 },
                    circle(0.0, rad),
                    rad,
                    arc(r),
                )
            }
            4 => {
                let major = r.range(6.0, 20.0);
                let minor = r.range(1.5, 0.8 * major);
                let zt = r.range(-0.7, 0.7) * minor;
                let torus = Surface::Torus(
                    crate::surface::Torus::new(
                        Frame::WORLD.with_origin(Point3::new(0.0, 0.0, zt)),
                        major,
                        minor,
                    )
                    .unwrap(),
                );
                let g = circle(0.0, major + (minor * minor - zt * zt).sqrt());
                (floor, torus, 1.0, g, minor, arc(r))
            }
            _ => {
                let ra = r.range(6.0, 15.0);
                let rb = r.range(1.5, 0.7 * ra);
                let y0 = r.range(-0.3, 0.3) * (ra - rb);
                let main = Surface::Cylinder(Cylinder {
                    frame: Frame::WORLD,
                    radius: ra,
                });
                let branch = Surface::Cylinder(Cylinder {
                    frame: Frame::from_z(Point3::new(0.0, y0, 0.0), UnitVec3::X),
                    radius: rb,
                });
                let w = 2.0 * ra;
                let region = Box3::from_points([Point3::new(-w, -w, -w), Point3::new(w, w, w)]);
                let ic = crate::intcurve::trace(&main, &branch, region)
                    .unwrap()
                    .into_iter()
                    .find(|c| c.pts[0].x > 0.0)
                    .unwrap();
                let hi = ic.domain_hi();
                let t0 = r.range(0.0, 0.6) * hi;
                let edge = (
                    Curve::Intersection(Arc::new(ic)),
                    Interval::new(t0, t0 + r.range(0.1, 0.35) * hi),
                    true,
                );
                let (g, ranges, _) = Guide::through_edges(&[edge], 8, 0.0).unwrap();
                (main, branch, 1.0, g, rb, ranges[0])
            }
        };
        let (lo, hi) = (range.lo, range.hi);
        let top = 0.3 * size;
        let k = 2 + r.below(3);
        let pts: Vec<(f64, f64)> = (0..k)
            .map(|i| (lo + (hi - lo) * i as f64 / (k - 1) as f64, r.range(0.1, top)))
            .collect();
        let law = Arc::new(RadiusLaw::smooth(&pts, None).unwrap());
        let (a, b) = (a.transformed(&f), b.transformed(&f));
        let g = guide.transformed(&f);
        (kind as u64, VarBlend::new(&a, 1.0, &b, side, g, law, range))
    }

    #[test]
    fn random_blends_hold_their_definition() {
        let mut r = Rng::new(10);
        let (mut built, mut tried) = (0, 0);
        for _ in 0..300 {
            tried += 1;
            let (kind, res) = random_blend(&mut r);
            // refusals seen: radius dipping below zero, folds, no room for the ball
            let Ok(b) = res else {
                continue;
            };
            built += 1;
            for _ in 0..10 {
                let uv = rand_uv(&mut r, &b);
                let s = uv.x;
                let (rad, ctr) = (b.radius(s), b.centre(s));
                let scale = 1.0 + ctr.to_vec().abs_max();
                assert!(
                    (b.a.implicit(ctr) - b.side[0] * rad).abs() < 1e-9 * scale,
                    "{b:?}"
                );
                assert!((b.b.implicit(ctr) - b.side[1] * rad).abs() < 1e-9 * scale);
                let d = Procedural::deriv(&*b, uv);
                assert!((d.p.distance(ctr) - rad).abs() < 1e-9 * scale);
                for (i, surf) in [&b.a, &b.b].into_iter().enumerate() {
                    let q = Procedural::deriv(&*b, Point2::new(s, b.v_on(i)));
                    assert!(surf.implicit(q.p).abs() < 1e-9 * scale);
                    let nb = q.du.cross(q.dv).normalize().unwrap();
                    assert!(nb.cross(surf.gradient(q.p)).length() < 1e-8);
                }
                let back = Procedural::deriv(&*b, Procedural::project(&*b, d.p)).p;
                assert!(back.distance(d.p) < 1e-9 * scale, "kind {kind} {uv:?}");
                assert!(b.implicit(d.p).abs() < 1e-9 * scale);
                let n = d.du.cross(d.dv).normalize().unwrap();
                let off = r.range(-0.2, 0.2) * b.min_radius();
                let p = d.p + n * off;
                let q = Procedural::deriv(&*b, Procedural::project(&*b, p)).p;
                assert!(q.distance(p) <= off.abs() + 1e-9 * scale, "kind {kind} off {off}");
                assert!(
                    b.implicit(p) * off >= 0.0 || b.implicit(p).abs() < 1e-9 * scale,
                    "kind {kind}"
                );
            }
            let (p, q) = (rand_uv(&mut r, &b), rand_uv(&mut r, &b));
            for bx in [
                Box2::new(Interval::new(b.lo, b.hi), Interval::new(0.0, 1.0)),
                Box2::new(
                    Interval::new(p.x.min(q.x), p.x.max(q.x)),
                    Interval::new(p.y.min(q.y), p.y.max(q.y)),
                ),
            ] {
                let bb = Procedural::bbox(&*b, bx);
                for i in 0..=8 {
                    for j in 0..=8 {
                        let uv = Point2::new(
                            bx.u.lo + bx.u.width() * i as f64 / 8.0,
                            bx.v.lo + bx.v.width() * j as f64 / 8.0,
                        );
                        assert!(bb.contains(Procedural::deriv(&*b, uv).p), "kind {kind} box");
                    }
                }
            }
            let uv = Point2::new(0.5 * (b.lo + b.hi), r.unit());
            let h = 1e-6 * (b.hi - b.lo);
            let d = Procedural::deriv(&*b, uv);
            let du = (Procedural::deriv(&*b, Point2::new(uv.x + h, uv.y)).p
                - Procedural::deriv(&*b, Point2::new(uv.x - h, uv.y)).p)
                / (2.0 * h);
            assert!(
                (du - d.du).length() < 1e-5 * (1.0 + d.du.length()),
                "kind {kind} du"
            );
            let far = d.p
                + Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0))
                    * b.radius(uv.x)
                    * 3.0;
            let q = Procedural::deriv(&*b, Procedural::project(&*b, far)).p;
            let best = grid_nearest(&b, far, 80, 16);
            assert!(
                q.distance(far) <= best + 1e-9 * scale_of(&b),
                "kind {kind} far {} vs {best}",
                q.distance(far)
            );
        }
        assert!(built * 10 >= tried * 7, "{built} of {tried}");
    }

    #[test]
    fn periodic_and_monotone_laws() {
        let pts = [(0.5, 1.0), (2.0, 2.5), (4.0, 1.5)];
        let p = RadiusLaw::periodic(&pts, 6.0).unwrap();
        for &(s, r) in &pts {
            assert!((p.eval(s) - r).abs() < 1e-12 && (p.eval(s + 6.0) - r).abs() < 1e-12);
        }
        // C2 where the period closes
        let (a, b) = (p.coefs(6.5 - 1e-12), p.coefs(0.5));
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() < 1e-9, "{k}: {a:?} {b:?}");
        }
        let (lo, hi) = p.bounds(5.0, 9.0);
        for i in 0..=400 {
            let r = p.eval(5.0 + 4.0 * i as f64 / 400.0);
            assert!(r >= lo - 1e-12 && r <= hi + 1e-12);
        }
        // steep between flat stretches: the spline swings past, the monotone law does not
        let steep = [(0.0, 1.0), (10.0, 1.0), (10.5, 0.1), (20.0, 0.1)];
        assert!(RadiusLaw::smooth(&steep, None).unwrap().bounds(0.0, 20.0).0 < 0.0);
        let m = RadiusLaw::monotone(&steep).unwrap();
        let (lo, hi) = m.bounds(-5.0, 25.0);
        assert!((lo - 0.1).abs() < 1e-12 && (hi - 1.0).abs() < 1e-12, "{lo} {hi}");
        assert!((m.eval(10.25) - 0.55).abs() < 0.2 && m.slope(-3.0) == 0.0 && m.eval(30.0) == 0.1);
    }

    #[test]
    fn full_round_balls_touch_all_three() {
        // a rib along y narrowing from 4 wide to 2, its top at z = 8
        let a = plane(Point3::new(-2.0, 0.0, 0.0), Vec3::new(-1.0, 0.05, 0.0));
        let b = plane(Point3::new(2.0, 0.0, 0.0), Vec3::new(1.0, 0.05, 0.0));
        let m = plane(Point3::new(0.0, 0.0, 8.0), Vec3::Z);
        let k = (1.0f64 + 0.0025).sqrt();
        let start = (Point3::new(0.0, 10.0, 6.5), 1.5);
        let vb = VarBlend::round(
            [&a, &m, &b],
            [-1.0, -1.0, -1.0],
            line_y(),
            Interval::new(-2.0, 22.0),
            start,
        )
        .expect("round");
        for s in [-2.0, 0.0, 7.3, 20.0, 22.0] {
            let (c, r) = vb.ball(s);
            let want = (2.0 - s / 20.0) / k;
            assert!(
                (r - want).abs() < 1e-9 && (vb.radius(s) - want).abs() < 1e-9,
                "{s}: {r} {want}"
            );
            for (surf, side) in [(&a, -1.0), (&b, -1.0), (&m, -1.0)] {
                assert!((surf.implicit(c) - side * r).abs() < 1e-9);
            }
            // over the top, more than half a turn (the contacts sit just below the centre)
            let (_, phi) = vb.section(s);
            assert!(phi > dmath::PI && phi < dmath::PI + 0.1, "{phi}");
            let top = Procedural::point(&*vb, Point2::new(s, 0.5));
            assert!((top.z - 8.0).abs() < 1e-3, "{top:?}");
            assert!(Procedural::project(&*vb, top).distance(Point2::new(s, 0.5)) < 1e-9);
        }
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

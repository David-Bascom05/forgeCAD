//! Exact free-form curves: a clamped rational B-spline held as Bezier pieces in homogeneous
//! coordinates, the one curve behind B-splines, fit splines and ellipses in sketches. A wall made
//! from one is defined by construction (`Extruded`, `Spun`), never a surface spline.
//! Formulas and how they were found: research/spline-loops/README.md.

use crate::curve::{CurveDeriv, Domain1, ProcCurve, ProceduralCurve};
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec2, Vec3, dmath, tol};
use std::sync::Arc;

mod extruded;
pub mod meet;
mod spun;
pub mod swept;
pub use extruded::Extruded;
pub use spun::Spun;
pub use swept::{Spine, Swept};

/// The highest degree taken. Products of curves run to three times it.
pub const MAX_DEGREE: usize = 15;
const WORK: usize = 3 * MAX_DEGREE + 2;
type H = [f64; 4];

const fn binomials() -> [[f64; WORK]; WORK] {
    let mut t = [[0.0; WORK]; WORK];
    let mut n = 0;
    while n < WORK {
        t[n][0] = 1.0;
        let mut k = 1;
        while k <= n {
            t[n][k] = t[n - 1][k - 1] + t[n - 1][k];
            k += 1;
        }
        n += 1;
    }
    t
}
static BINOM: [[f64; WORK]; WORK] = binomials();

/// Bernstein coefficients of the product of two Bernstein polynomials, onto `out`.
fn bern_mul(a: &[f64], b: &[f64], out: &mut [f64]) {
    let (m, n) = (a.len() - 1, b.len() - 1);
    for x in out[..=m + n].iter_mut() {
        *x = 0.0;
    }
    for (i, &ai) in a.iter().enumerate() {
        let wi = ai * BINOM[m][i];
        for (j, &bj) in b.iter().enumerate() {
            out[i + j] += wi * BINOM[n][j] * bj;
        }
    }
    for (k, x) in out[..=m + n].iter_mut().enumerate() {
        *x /= BINOM[m + n][k];
    }
}

/// A Bernstein polynomial's value and slope at s (de Casteljau: stable, also outside [0, 1]).
fn bern_eval(c: &[f64], s: f64) -> (f64, f64) {
    let n = c.len() - 1;
    if n == 0 {
        return (c[0], 0.0);
    }
    let mut w = [0.0; WORK];
    w[..=n].copy_from_slice(c);
    for r in (2..=n).rev() {
        for j in 0..r {
            w[j] = w[j] + (w[j + 1] - w[j]) * s;
        }
    }
    (w[0] + (w[1] - w[0]) * s, n as f64 * (w[1] - w[0]))
}

/// The root of a Bernstein polynomial between a and b, where it changes sign.
fn bern_root(c: &[f64], mut a: f64, mut b: f64, fa: f64) -> f64 {
    let neg = fa < 0.0;
    let mut x = 0.5 * (a + b);
    for _ in 0..100 {
        let (f, d) = bern_eval(c, x);
        if f == 0.0 {
            return x;
        }
        if (f < 0.0) == neg {
            a = x;
        } else {
            b = x;
        }
        let nt = x - f / d;
        let next = if d != 0.0 && nt > a && nt < b {
            nt
        } else {
            0.5 * (a + b)
        };
        if next == x || b - a <= 4.0 * f64::EPSILON * a.abs().max(b.abs()).max(f64::MIN_POSITIVE) {
            return next;
        }
        x = next;
    }
    x
}

fn variations(c: &[f64]) -> usize {
    let mut last = 0.0;
    let mut n = 0;
    for &x in c {
        if x != 0.0 {
            if last * x < 0.0 {
                n += 1;
            }
            last = x;
        }
    }
    n
}

/// Where a Bernstein polynomial changes sign in (0, 1), ascending, onto `out`: its count. Fewer
/// sign changes among the coefficients than two need no cuts (variation diminishing); past that
/// the derivative's own changes cut [0, 1] into pieces where it is monotone.
fn crossings(c: &[f64], out: &mut [f64; WORK]) -> usize {
    match variations(c) {
        0 => 0,
        1 => {
            let (&a, &b) = (c.first().expect("not empty"), c.last().expect("not empty"));
            if a == 0.0 || b == 0.0 || a * b > 0.0 {
                return crossings_cut(c, out);
            }
            out[0] = bern_root(c, 0.0, 1.0, a);
            1
        }
        _ => crossings_cut(c, out),
    }
}

fn crossings_cut(c: &[f64], out: &mut [f64; WORK]) -> usize {
    let n = c.len() - 1;
    let mut d = [0.0; WORK];
    for i in 0..n {
        d[i] = n as f64 * (c[i + 1] - c[i]);
    }
    let mut cuts = [0.0; WORK];
    let m = if n > 1 { crossings(&d[..n], &mut cuts) } else { 0 };
    let mut found = 0;
    let (mut a, mut fa) = (0.0, c[0]);
    for &b in cuts[..m].iter().chain([&1.0]) {
        let fb = if b < 1.0 { bern_eval(c, b).0 } else { c[n] };
        if fa != 0.0 && fb != 0.0 && (fa < 0.0) != (fb < 0.0) {
            out[found] = bern_root(c, a, b, fa);
            found += 1;
        } else if fa == 0.0 && a > 0.0 && (fb < 0.0) != (bern_eval(c, 0.5 * a).0 < 0.0) {
            out[found] = a;
            found += 1;
        }
        (a, fa) = (b, fb);
    }
    found
}

/// The roots of a Bernstein polynomial in [0, 1], ascending, each with whether it only touches
/// zero: an extremum within `near` of zero counts as zero (the kernel's tolerance, in the value's
/// own scale, so a plane within tolerance of tangent touches a wall once and does not cut a
/// hair). Sign changes at such runs of extrema stay crossings.
pub fn bern_roots(c: &[f64], near: f64, out: &mut Vec<(f64, bool)>) {
    let n = c.len() - 1;
    if c.iter().all(|&x| x > near) || c.iter().all(|&x| x < -near) {
        return;
    }
    let mut d = [0.0; WORK];
    for i in 0..n {
        d[i] = n as f64 * (c[i + 1] - c[i]);
    }
    let mut cuts = [0.0; WORK];
    let m = if n > 1 { crossings(&d[..n], &mut cuts) } else { 0 };
    let val = |s: f64| bern_eval(c, s).0;
    if c[0] == 0.0 {
        out.push((0.0, false));
    }
    let (mut a, mut fa) = (0.0, c[0]);
    let mut k = 0;
    while k <= m {
        let (b, fb) = if k < m {
            (cuts[k], val(cuts[k]))
        } else {
            (1.0, c[n])
        };
        if k < m && fb.abs() <= near {
            // a run of extrema at zero: one root, a crossing if the sign is changed past it
            let first = k;
            while k + 1 < m && val(cuts[k + 1]).abs() <= near {
                k += 1;
            }
            let at = 0.5 * (cuts[first] + cuts[k]);
            let next = if k + 1 < m { val(cuts[k + 1]) } else { c[n] };
            out.push((at, (fa < 0.0) == (next < 0.0) || fa == 0.0 || next == 0.0));
            // past the run the value keeps the sign it leaves with
            a = cuts[k];
            fa = next;
            k += 1;
            continue;
        }
        if fa != 0.0 && fb != 0.0 && (fa < 0.0) != (fb < 0.0) {
            out.push((bern_root(c, a, b, fa), false));
        }
        (a, fa) = (b, fb);
        k += 1;
    }
    if c[n] == 0.0 {
        out.push((1.0, false));
    }
}

fn point_of(h: &H) -> Point3 {
    Point3::new(h[0] / h[3], h[1] / h[3], h[2] / h[3])
}

fn lerp(a: &H, b: &H, s: f64) -> H {
    [
        a[0] + (b[0] - a[0]) * s,
        a[1] + (b[1] - a[1]) * s,
        a[2] + (b[2] - a[2]) * s,
        a[3] + (b[3] - a[3]) * s,
    ]
}

fn sub(a: &H, b: &H) -> H {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]]
}

/// A chain of Bezier pieces of one degree in homogeneous coordinates: the curve in t, piece i over
/// [breaks[i], breaks[i + 1]] and carried on as its own polynomial past the two ends.
#[derive(Clone, Debug)]
pub struct Spline {
    degree: usize,
    breaks: Vec<f64>,
    /// (degree + 1) points per piece; (x w, y w, z w, w).
    pts: Vec<H>,
    rational: bool,
    /// Each piece's control hull.
    boxes: Vec<Box3>,
}

impl PartialEq for Spline {
    fn eq(&self, o: &Spline) -> bool {
        self.degree == o.degree && self.breaks == o.breaks && self.pts == o.pts
    }
}

impl Spline {
    /// From Bezier pieces, None where the data can't make a curve: too high a degree, no pieces,
    /// breaks that do not increase, a weight not positive, anything not finite.
    pub fn from_pieces(degree: usize, breaks: Vec<f64>, pts: Vec<H>) -> Option<Spline> {
        let n = breaks.len().checked_sub(1)?;
        if degree == 0 || degree > MAX_DEGREE || n == 0 || pts.len() != n * (degree + 1) {
            return None;
        }
        if breaks.windows(2).any(|w| w[1] <= w[0] || !w[1].is_finite()) || !breaks[0].is_finite() {
            return None;
        }
        if pts
            .iter()
            .any(|h| h.iter().any(|x| !x.is_finite()) || h[3] <= 0.0)
        {
            return None;
        }
        let rational = pts.iter().any(|h| h[3] != 1.0);
        let boxes = pts
            .chunks(degree + 1)
            .map(|r| Box3::from_points(r.iter().map(point_of)))
            .collect();
        Some(Spline {
            degree,
            breaks,
            pts,
            rational,
            boxes,
        })
    }

    /// From B-spline data: degree, knots (ctrl.len() + degree + 1 of them, not decreasing, each
    /// at most degree + 1 times), control points and weights. Knots not clamped at the ends are
    /// clamped by insertion; the curve is over [knots[degree], knots[ctrl.len()]].
    pub fn from_nurbs(
        degree: usize,
        knots: &[f64],
        ctrl: &[Point3],
        weights: Option<&[f64]>,
    ) -> Option<Spline> {
        if degree == 0
            || degree > MAX_DEGREE
            || ctrl.len() <= degree
            || knots.len() != ctrl.len() + degree + 1
            || knots.iter().any(|k| !k.is_finite())
            || knots.windows(2).any(|w| w[1] < w[0])
            || weights.is_some_and(|w| w.len() != ctrl.len())
        {
            return None;
        }
        let mut u = knots.to_vec();
        let mut p: Vec<H> = ctrl
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let w = weights.map_or(1.0, |w| w[i]);
                [c.x * w, c.y * w, c.z * w, w]
            })
            .collect();
        let (lo, hi) = (knots[degree], knots[ctrl.len()]);
        if hi <= lo {
            return None;
        }
        // Boehm: every knot in the domain up to degree times (the domain's ends degree + 1)
        let mut at = degree;
        while at + 1 < u.len() && u[at] < hi {
            let k = u[at];
            let mult = u.iter().filter(|&&x| x == k).count();
            let want = if k == lo { degree + 1 } else { degree };
            for _ in mult..want {
                insert_knot(&mut u, &mut p, degree, k);
            }
            at = u.iter().rposition(|&x| x == k).expect("just there") + 1;
        }
        for _ in u.iter().filter(|&&x| x == hi).count()..degree + 1 {
            insert_knot(&mut u, &mut p, degree, hi);
        }
        let first = u.iter().position(|&x| x == lo).expect("lo is a knot");
        let last = u.iter().rposition(|&x| x == hi).expect("hi is a knot");
        let (u, p) = (&u[first..=last], &p[first..last - degree]);
        let mut breaks: Vec<f64> = u.to_vec();
        breaks.dedup();
        let n = breaks.len() - 1;
        if p.len() != n * degree + 1 {
            return None;
        }
        let mut pts = Vec::with_capacity(n * (degree + 1));
        for i in 0..n {
            pts.extend_from_slice(&p[i * degree..=(i + 1) * degree]);
        }
        Spline::from_pieces(degree, breaks, pts)
    }

    /// The cubic through `pts`, curvature continuous, its knots spaced by the square root of the
    /// distance between points (centripetal) and scaled to run as long as those distances together:
    /// closed and periodic if first and last are the same point, else natural at an end with no
    /// slope given. A slope given is per that parameter.
    pub fn through(pts: &[Point3], from: Option<Vec3>, to: Option<Vec3>) -> Option<Spline> {
        let n = pts.len();
        if n < 2 {
            return None;
        }
        let mut t = vec![0.0; n];
        let mut length = 0.0;
        for i in 1..n {
            let h = pts[i].distance(pts[i - 1]);
            if h <= tol::LINEAR {
                return None;
            }
            length += h;
            t[i] = t[i - 1] + h.sqrt();
        }
        let scale = length / t[n - 1];
        t.iter_mut().for_each(|x| *x *= scale);
        let closed = n > 2 && pts[0].distance(pts[n - 1]) <= tol::LINEAR;
        let h = |i: usize| t[i + 1] - t[i];
        let dp = |i: usize| (pts[i + 1] - pts[i]) / h(i);
        // slopes m of the Hermite pieces: tridiagonal rows, C2 at the middle points
        let m: Vec<Vec3> = if closed {
            let k = n - 1;
            let row = |i: usize| {
                let (hl, hr) = (h((i + k - 1) % k), h(i % k));
                (
                    hr,
                    2.0 * (hl + hr),
                    hl,
                    (dp((i + k - 1) % k) * hr + dp(i % k) * hl) * 3.0,
                )
            };
            let rows: Vec<_> = (0..k).map(row).collect();
            let mut s = cyclic(&rows);
            s.push(s[0]);
            s
        } else {
            let mut rows = Vec::with_capacity(n);
            rows.push(match from {
                Some(d) => (0.0, 1.0, 0.0, d),
                None => (0.0, 2.0, 1.0, dp(0) * 3.0),
            });
            for i in 1..n - 1 {
                rows.push((
                    h(i),
                    2.0 * (h(i - 1) + h(i)),
                    h(i - 1),
                    (dp(i - 1) * h(i) + dp(i) * h(i - 1)) * 3.0,
                ));
            }
            rows.push(match to {
                Some(d) => (0.0, 1.0, 0.0, d),
                None => (1.0, 2.0, 0.0, dp(n - 2) * 3.0),
            });
            tridiagonal(&rows)
        };
        let mut out = Vec::with_capacity((n - 1) * 4);
        for i in 0..n - 1 {
            let (a, b) = (pts[i], pts[i + 1]);
            let (c1, c2) = (a + m[i] * (h(i) / 3.0), b - m[i + 1] * (h(i) / 3.0));
            for q in [a, c1, c2, b] {
                out.push([q.x, q.y, q.z, 1.0]);
            }
        }
        Spline::from_pieces(3, t, out)
    }

    /// The arc in the xy plane about `center`, semi-axes a along `x` and b across, from angle
    /// `start` through `sweep`: a rational quadratic per quarter turn or less, t the angle's
    /// distance along. None for a zero sweep or more than a whole turn.
    pub fn ellipse(center: Point3, x: Vec3, y: Vec3, start: f64, sweep: f64) -> Option<Spline> {
        if sweep == 0.0 || sweep.abs() > dmath::TAU * (1.0 + 1e-12) {
            return None;
        }
        let n = (sweep.abs() / dmath::FRAC_PI_2 - 1e-9).ceil().max(1.0) as usize;
        let d = sweep / n as f64;
        let w = dmath::cos(0.5 * d);
        let at = |a: f64| {
            let (s, c) = dmath::sin_cos(a);
            center + x * c + y * s
        };
        let mut pts = Vec::with_capacity(3 * n);
        let mut breaks = Vec::with_capacity(n + 1);
        breaks.push(0.0);
        for i in 0..n {
            let a0 = start + d * i as f64;
            let a1 = start + d * (i + 1) as f64;
            let (s, c) = dmath::sin_cos(a0 + 0.5 * d);
            let mid = center + x * (c / w) + y * (s / w);
            let (p0, p2) = (at(a0), at(a1));
            pts.push([p0.x, p0.y, p0.z, 1.0]);
            pts.push([mid.x * w, mid.y * w, mid.z * w, w]);
            pts.push([p2.x, p2.y, p2.z, 1.0]);
            breaks.push(d.abs() * (i + 1) as f64);
        }
        Spline::from_pieces(2, breaks, pts)
    }

    pub fn degree(&self) -> usize {
        self.degree
    }
    pub fn rational(&self) -> bool {
        self.rational
    }
    pub fn pieces(&self) -> usize {
        self.breaks.len() - 1
    }
    pub fn breaks(&self) -> &[f64] {
        &self.breaks
    }
    pub fn domain(&self) -> Interval {
        Interval::new(self.breaks[0], self.breaks[self.pieces()])
    }
    /// The curve as B-spline data: degree, knots (each break to the degree, so the pieces come
    /// back as they are), control points, weights where not all 1. Only where the pieces join.
    pub fn control_form(&self) -> (usize, Vec<f64>, Vec<Point3>, Option<Vec<f64>>) {
        let (p, n) = (self.degree, self.pieces());
        let mut pts = Vec::with_capacity(n * p + 1);
        let mut weights = Vec::with_capacity(n * p + 1);
        for i in 0..n {
            for (j, h) in self.row(i).iter().enumerate() {
                if j > 0 || i == 0 {
                    pts.push(point_of(h));
                    weights.push(h[3]);
                }
            }
        }
        let mut knots = vec![self.breaks[0]; p + 1];
        for b in &self.breaks[1..n] {
            knots.extend(std::iter::repeat_n(*b, p));
        }
        knots.extend(std::iter::repeat_n(self.breaks[n], p + 1));
        let any = weights.iter().any(|&w| w != 1.0);
        (p, knots, pts, any.then_some(weights))
    }
    /// Degree, breaks, and each piece's points in turn as x y z w (each its own, not times w).
    pub fn bezier_data(&self) -> (usize, Vec<f64>, Vec<H>) {
        let pts = self
            .pts
            .iter()
            .map(|h| [h[0] / h[3], h[1] / h[3], h[2] / h[3], h[3]])
            .collect();
        (self.degree, self.breaks.clone(), pts)
    }
    pub(crate) fn row(&self, i: usize) -> &[H] {
        &self.pts[i * (self.degree + 1)..(i + 1) * (self.degree + 1)]
    }
    /// The curve's first and last points.
    pub fn ends(&self) -> (Point3, Point3) {
        (
            point_of(&self.pts[0]),
            point_of(self.pts.last().expect("has points")),
        )
    }

    /// The piece t falls in (the end pieces carried on past the ends), and where along it.
    fn locate(&self, t: f64) -> (usize, f64, f64) {
        let n = self.pieces();
        let i = if n == 1 {
            0
        } else {
            let k = self.breaks[1..n].partition_point(|&b| b <= t);
            k.min(n - 1)
        };
        let h = self.breaks[i + 1] - self.breaks[i];
        (i, (t - self.breaks[i]) / h, h)
    }

    pub fn point(&self, t: f64) -> Point3 {
        let (i, s, _) = self.locate(t);
        let mut w = [[0.0; 4]; MAX_DEGREE + 1];
        w[..=self.degree].copy_from_slice(self.row(i));
        for r in (1..=self.degree).rev() {
            for j in 0..r {
                w[j] = lerp(&w[j], &w[j + 1], s);
            }
        }
        point_of(&w[0])
    }

    pub fn deriv(&self, t: f64) -> CurveDeriv {
        let (i, s, h) = self.locate(t);
        let p = self.degree;
        let mut w = [[0.0; 4]; MAX_DEGREE + 1];
        w[..=p].copy_from_slice(self.row(i));
        for r in (3..=p).rev() {
            for j in 0..r {
                w[j] = lerp(&w[j], &w[j + 1], s);
            }
        }
        let (a2, top) = if p >= 2 {
            let k = (p * (p - 1)) as f64 / (h * h);
            let dd = sub(&sub(&w[2], &w[1]), &sub(&w[1], &w[0]));
            let a2 = [dd[0] * k, dd[1] * k, dd[2] * k, dd[3] * k];
            (a2, [lerp(&w[0], &w[1], s), lerp(&w[1], &w[2], s)])
        } else {
            ([0.0; 4], [w[0], w[1]])
        };
        let k = p as f64 / h;
        let d = sub(&top[1], &top[0]);
        let a1 = [d[0] * k, d[1] * k, d[2] * k, d[3] * k];
        let a0 = lerp(&top[0], &top[1], s);
        let q = point_of(&a0);
        if !self.rational {
            return CurveDeriv {
                p: q,
                d1: Vec3::new(a1[0], a1[1], a1[2]),
                d2: Vec3::new(a2[0], a2[1], a2[2]),
            };
        }
        let iw = 1.0 / a0[3];
        let d1 = (Vec3::new(a1[0], a1[1], a1[2]) - q.to_vec() * a1[3]) * iw;
        let d2 = (Vec3::new(a2[0], a2[1], a2[2]) - d1 * (2.0 * a1[3]) - q.to_vec() * a2[3]) * iw;
        CurveDeriv { p: q, d1, d2 }
    }

    /// The pieces over `range`, each as its stretch of the parameter.
    pub(crate) fn spans_over(&self, range: Interval) -> Vec<Interval> {
        self.over(range)
            .filter(|p| p.2 > p.1)
            .map(|(i, s0, s1)| {
                let (a, h) = (self.breaks[i], self.breaks[i + 1] - self.breaks[i]);
                Interval::new(a + s0 * h, a + s1 * h)
            })
            .collect()
    }

    /// The control points over `range`, inside one piece.
    pub(crate) fn control_over(&self, range: Interval) -> Vec<H> {
        let (i, ..) = self.locate(range.mid());
        let (a, h) = (self.breaks[i], self.breaks[i + 1] - self.breaks[i]);
        let w = self.part(i, (range.lo - a) / h, (range.hi - a) / h);
        w[..=self.degree].to_vec()
    }

    /// `deriv`, and the third derivative.
    pub fn deriv3(&self, t: f64) -> (CurveDeriv, Vec3) {
        let d = self.deriv(t);
        if self.degree < 3 && !self.rational {
            return (d, Vec3::ZERO);
        }
        let (i, s, h) = self.locate(t);
        let p = self.degree;
        // the homogeneous derivatives by their Bernstein points' differences
        let mut w: [H; MAX_DEGREE + 1] = [[0.0; 4]; MAX_DEGREE + 1];
        w[..=p].copy_from_slice(self.row(i));
        let value = |row: &[H]| -> H {
            let mut q: [H; MAX_DEGREE + 1] = [[0.0; 4]; MAX_DEGREE + 1];
            q[..row.len()].copy_from_slice(row);
            for r in (1..row.len()).rev() {
                for j in 0..r {
                    q[j] = lerp(&q[j], &q[j + 1], s);
                }
            }
            q[0]
        };
        let a0 = value(&w[..=p]);
        let mut scale = 1.0;
        let mut ders = [[0.0; 4]; 4];
        ders[0] = a0;
        for o in 1..=3 {
            if o > p {
                break;
            }
            for j in 0..=p - o {
                w[j] = sub(&w[j + 1], &w[j]);
            }
            scale *= (p + 1 - o) as f64 / h;
            let x = value(&w[..=p - o]);
            ders[o] = [x[0] * scale, x[1] * scale, x[2] * scale, x[3] * scale];
        }
        let v = |h: &H| Vec3::new(h[0], h[1], h[2]);
        if !self.rational {
            return (d, v(&ders[3]));
        }
        let iw = 1.0 / a0[3];
        let d3 =
            (v(&ders[3]) - d.d2 * (3.0 * ders[1][3]) - d.d1 * (3.0 * ders[2][3]) - d.p.to_vec() * ders[3][3])
                * iw;
        (d, d3)
    }

    /// Piece i's control points over [s0, s1] of its own parameter.
    fn part(&self, i: usize, s0: f64, s1: f64) -> [H; MAX_DEGREE + 1] {
        let p = self.degree;
        let mut w = [[0.0; 4]; MAX_DEGREE + 1];
        w[..=p].copy_from_slice(self.row(i));
        // the end further from zero first, so the second split's fraction stays in [0, 1]
        let left = |w: &mut [H; MAX_DEGREE + 1], k: f64| {
            for r in 1..=p {
                for j in (r..=p).rev() {
                    w[j] = lerp(&w[j - 1], &w[j], k);
                }
            }
        };
        let right = |w: &mut [H; MAX_DEGREE + 1], k: f64| {
            for r in 1..=p {
                for j in 0..=p - r {
                    w[j] = lerp(&w[j], &w[j + 1], k);
                }
            }
        };
        if s1 != 0.0 {
            if s1 != 1.0 {
                left(&mut w, s1);
            }
            if s0 != 0.0 {
                right(&mut w, s0 / s1);
            }
        } else {
            right(&mut w, s0);
            left(&mut w, -s0 / (1.0 - s0));
        }
        w
    }

    /// The curve over `range` as rational Bezier pieces of its own degree, in order, each run
    /// with the curve's parameter: homogeneous points [x w, y w, z w, w].
    pub fn bezier_pieces(&self, range: Interval) -> Vec<Vec<[f64; 4]>> {
        self.over(range)
            .filter(|&(_, s0, s1)| s1 > s0)
            .map(|(i, s0, s1)| self.part(i, s0, s1)[..=self.degree].to_vec())
            .collect()
    }

    /// The pieces over `range`: each one's index and its stretch of its own parameter.
    fn over(&self, range: Interval) -> impl Iterator<Item = (usize, f64, f64)> + '_ {
        let n = self.pieces();
        (0..n).filter_map(move |i| {
            let (a, b) = (self.breaks[i], self.breaks[i + 1]);
            let lo = if i == 0 { range.lo } else { range.lo.max(a) };
            let hi = if i == n - 1 { range.hi } else { range.hi.min(b) };
            (lo < hi || (lo == hi && (a..=b).contains(&lo))).then(|| {
                let h = b - a;
                (i, (lo - a) / h, (hi - a) / h)
            })
        })
    }

    pub fn bbox(&self, range: Interval) -> Box3 {
        let mut b = Box3::EMPTY;
        for (i, s0, s1) in self.over(range) {
            if s0 == 0.0 && s1 == 1.0 {
                b = b.hull(&self.boxes[i]);
            } else {
                let w = self.part(i, s0, s1);
                for h in &w[..=self.degree] {
                    b.include(point_of(h));
                }
            }
        }
        b.grow(1e-13 * (1.0 + b.x.lo.abs().max(b.x.hi.abs()).max(b.y.lo.abs()).max(b.y.hi.abs())))
    }

    /// Bounds on |C'|, |C''| and |C'''| over `range`, from the Bernstein points of the homogeneous
    /// derivatives over each piece (quotient rule for weights).
    pub fn derivative_bounds(&self, range: Interval) -> [f64; 3] {
        let mut out = [0.0f64; 3];
        for (i, s0, s1) in self.over(range) {
            // (a range ending past a break by rounding: nothing there, and a scale of 1 / 0)
            if s1 <= s0 {
                continue;
            }
            let h = self.breaks[i + 1] - self.breaks[i];
            let k = s1 - s0;
            let p = self.degree;
            let w = self.part(i, s0, s1);
            // sizes of the point part and the weight part of A, A', A'', A''' over the stretch
            let mut d: [H; MAX_DEGREE + 1] = w;
            let mut sizes = [[0.0f64; 2]; 4];
            let mut wmin = f64::INFINITY;
            let mut cmax = 0.0f64;
            for q in &d[..=p] {
                wmin = wmin.min(q[3]);
                cmax = cmax.max(point_of(q).to_vec().length());
            }
            let mut scale = 1.0;
            for (o, size) in sizes.iter_mut().enumerate() {
                if o > p {
                    break;
                }
                if o > 0 {
                    for j in 0..=p - o {
                        d[j] = sub(&d[j + 1], &d[j]);
                    }
                    scale *= (p + 1 - o) as f64 / (h * k);
                }
                for q in &d[..=p - o] {
                    size[0] = size[0].max(scale * Vec3::new(q[0], q[1], q[2]).length());
                    size[1] = size[1].max(scale * q[3].abs());
                }
            }
            if !self.rational {
                for o in 0..3 {
                    out[o] = out[o].max(sizes[o + 1][0]);
                }
                continue;
            }
            let [_, a1, a2, a3] = sizes;
            let c1 = (a1[0] + cmax * a1[1]) / wmin;
            let c2 = (a2[0] + 2.0 * c1 * a1[1] + cmax * a2[1]) / wmin;
            let c3 = (a3[0] + 3.0 * c2 * a1[1] + 3.0 * c1 * a2[1] + cmax * a3[1]) / wmin;
            out[0] = out[0].max(c1);
            out[1] = out[1].max(c2);
            out[2] = out[2].max(c3);
        }
        out
    }

    /// Whether the curve has no cusp or stationary point.
    pub fn is_regular(&self) -> bool {
        self.regular_over(self.domain())
    }

    /// Whether the derivative stays away from zero over `range`: a cusp shows as the speed all but
    /// gone with the direction turned back (a sharp bend at speed is no cusp).
    pub(crate) fn regular_over(&self, range: Interval) -> bool {
        for (i, s0, s1) in self.over(range) {
            let n = 16;
            let (a, h) = (self.breaks[i], self.breaks[i + 1] - self.breaks[i]);
            let ds: Vec<Vec3> = (0..=n)
                .map(|k| self.deriv(a + h * (s0 + (s1 - s0) * k as f64 / n as f64)).d1)
                .collect();
            let size = ds.iter().fold(0.0f64, |m, d| m.max(d.length()));
            if ds.iter().any(|d| d.length() <= 1e-9 * size) || size == 0.0 {
                return false;
            }
            if ds
                .windows(2)
                .any(|w| w[0].dot(w[1]) < 0.0 && w[0].length().min(w[1].length()) < 1e-3 * size)
            {
                return false;
            }
            let w = self.part(i, s0, s1);
            if w[..=self.degree].iter().any(|q| q[3] <= 0.0) {
                return false;
            }
        }
        true
    }

    /// The t in `range` of the point nearest p. Every stationary point of a piece is a root of
    /// (A - p w).(A' w - A w'), a polynomial of degree 3 p - 1 (2 p - 1 with no weights), so the
    /// nearest is found, not hoped for; pieces whose box lies further than a point already seen
    /// take no solve.
    pub fn closest(&self, p: Point3, range: Interval) -> f64 {
        let mut best = (f64::INFINITY, range.lo);
        let take = |t: f64, best: &mut (f64, f64)| {
            let d = self.point(t).distance_sq(p);
            if d < best.0 {
                *best = (d, t);
            }
        };
        take(range.lo, &mut best);
        take(range.hi, &mut best);
        // the joints too: a foot exactly at one is no sign change inside either piece
        for (i, ..) in self.over(range) {
            let (a, b) = (self.breaks[i].max(range.lo), self.breaks[i + 1].min(range.hi));
            take(a, &mut best);
            take(0.5 * (a + b), &mut best);
            take(b, &mut best);
        }
        let (mut u, mut v) = ([0.0; WORK], [0.0; WORK]);
        let mut cuts = [0.0; WORK];
        for (i, s0, s1) in self.over(range) {
            let whole = s0 == 0.0 && s1 == 1.0;
            if whole && box_gap_sq(&self.boxes[i], p) >= best.0 {
                continue;
            }
            let w = self.part(i, s0, s1);
            let q = self.degree;
            let mut g = [0.0; WORK];
            let mut prod = [0.0; WORK];
            let n = if self.rational {
                // D = A' w - A w', of degree 2 q - 1 (q - 1 + q)
                let mut wc = [0.0; MAX_DEGREE + 1];
                let mut wd = [0.0; MAX_DEGREE + 1];
                for j in 0..=q {
                    wc[j] = w[j][3];
                }
                for j in 0..q {
                    wd[j] = w[j + 1][3] - w[j][3];
                }
                let m = 3 * q - 1;
                for c in 0..3 {
                    let mut a = [0.0; MAX_DEGREE + 1];
                    let mut ad = [0.0; MAX_DEGREE + 1];
                    for j in 0..=q {
                        a[j] = w[j][c];
                        u[j] = w[j][c] - [p.x, p.y, p.z][c] * w[j][3];
                    }
                    for j in 0..q {
                        ad[j] = w[j + 1][c] - w[j][c];
                    }
                    let (mut d1, mut d2) = ([0.0; WORK], [0.0; WORK]);
                    bern_mul(&ad[..q], &wc[..=q], &mut d1);
                    bern_mul(&wd[..q], &a[..=q], &mut d2);
                    for j in 0..2 * q {
                        v[j] = d1[j] - d2[j];
                    }
                    bern_mul(&u[..=q], &v[..2 * q], &mut prod);
                    for j in 0..=m {
                        g[j] += prod[j];
                    }
                }
                m
            } else {
                let m = 2 * q - 1;
                for c in 0..3 {
                    let at = [p.x, p.y, p.z][c];
                    for j in 0..=q {
                        u[j] = w[j][c] - at;
                    }
                    for j in 0..q {
                        v[j] = w[j + 1][c] - w[j][c];
                    }
                    bern_mul(&u[..=q], &v[..q], &mut prod);
                    for j in 0..=m {
                        g[j] += prod[j];
                    }
                }
                m
            };
            let k = crossings(&g[..=n], &mut cuts);
            let (a, h) = (self.breaks[i], self.breaks[i + 1] - self.breaks[i]);
            for &r in &cuts[..k] {
                take(a + (s0 + (s1 - s0) * r) * h, &mut best);
            }
        }
        best.1
    }

    /// The nearest point by Newton from `near`: for a point on or by the curve near that
    /// parameter, what `closest` would find; None where it does not settle or runs out of `range`.
    pub fn closest_near(&self, p: Point3, near: f64, range: Interval) -> Option<f64> {
        let mut t = near.clamp(range.lo, range.hi);
        for _ in 0..12 {
            let d = self.deriv(t);
            let e = d.p - p;
            let g = e.dot(d.d1);
            let h = d.d1.length_sq() + e.dot(d.d2);
            if h <= 0.0 {
                return None;
            }
            let step = -g / h;
            let next = t + step;
            if !range.contains(next) {
                return None;
            }
            if (d.d1 * step).length() <= 1e-13 * (1.0 + p.to_vec().abs_max()) {
                return Some(next);
            }
            t = next;
        }
        None
    }

    /// The same curve with every point sent through an affine map (of homogeneous points, the
    /// weights untouched).
    pub fn mapped(&self, f: impl Fn(Point3) -> Point3) -> Spline {
        let pts: Vec<H> = self
            .pts
            .iter()
            .map(|h| {
                let q = f(point_of(h));
                [q.x * h[3], q.y * h[3], q.z * h[3], h[3]]
            })
            .collect();
        Spline::from_pieces(self.degree, self.breaks.clone(), pts).expect("same pieces")
    }

    /// The same points run the other way, over the same range.
    pub fn reversed(&self) -> Spline {
        let (lo, hi) = (self.breaks[0], self.breaks[self.pieces()]);
        let breaks: Vec<f64> = self.breaks.iter().rev().map(|b| lo + hi - b).collect();
        let pts: Vec<H> = self.pts.iter().rev().copied().collect();
        Spline::from_pieces(self.degree, breaks, pts).expect("same pieces")
    }

    /// The stretch over `range` as a curve of its own, in the same parameter.
    pub fn stretch(&self, range: Interval) -> Option<Spline> {
        let mut breaks = vec![range.lo];
        let mut pts = Vec::new();
        for (i, s0, s1) in self.over(range) {
            if s1 <= s0 {
                continue;
            }
            let w = self.part(i, s0, s1);
            pts.extend_from_slice(&w[..=self.degree]);
            breaks.push(self.breaks[i] + s1 * (self.breaks[i + 1] - self.breaks[i]));
        }
        Spline::from_pieces(self.degree, breaks, pts)
    }

    /// The breaks where the tangent jumps: the curve is a chain of smooth arcs between them.
    pub fn corners(&self) -> Vec<f64> {
        (1..self.pieces())
            .filter(|&i| {
                let (a, b) = (self.row(i - 1), self.row(i));
                let p = self.degree;
                let tangent = |x: &H, y: &H| point_of(y) - point_of(x);
                let (t0, t1) = (tangent(&a[p - 1], &a[p]), tangent(&b[0], &b[1]));
                let sine = t0.cross(t1).length() / (t0.length() * t1.length()).max(1e-300);
                sine > tol::ANGULAR
                    || t0.dot(t1) < 0.0
                    || point_of(&a[p]).distance(point_of(&b[0])) > tol::LINEAR
            })
            .map(|i| self.breaks[i])
            .collect()
    }

    /// The Bernstein coefficients of a x + b y + c z + d w, a linear form of the homogeneous
    /// point, over piece i's stretch [s0, s1]: zero where the point lies in that plane.
    fn plane_coefs(&self, i: usize, s0: f64, s1: f64, f: [f64; 4], out: &mut [f64]) {
        let w = self.part(i, s0, s1);
        for (j, h) in w[..=self.degree].iter().enumerate() {
            out[j] = f[0] * h[0] + f[1] * h[1] + f[2] * h[2] + f[3] * h[3];
        }
    }

    /// The t in `range` where a point of the curve lies in the plane n.x + d = 0 (n unit): the
    /// roots of a Bernstein polynomial of the curve's own degree. A form within `near` of zero at
    /// an extremum touches.
    pub fn plane_roots(&self, n: Vec3, d: f64, range: Interval, near: f64) -> Vec<(f64, bool)> {
        let mut out: Vec<(f64, bool)> = Vec::new();
        let mut c = [0.0; MAX_DEGREE + 1];
        let mut found = Vec::new();
        for (i, s0, s1) in self.over(range) {
            if s1 <= s0 {
                continue;
            }
            let whole = s0 == 0.0 && s1 == 1.0;
            if whole && plane_clear(&self.boxes[i], n, d, near) {
                continue;
            }
            self.plane_coefs(i, s0, s1, [n.x, n.y, n.z, d], &mut c);
            found.clear();
            // `near` scales with the weight: the form is w times the distance
            bern_roots(&c[..=self.degree], near, &mut found);
            let (a, h) = (self.breaks[i], self.breaks[i + 1] - self.breaks[i]);
            for &(r, touch) in &found {
                push_root(&mut out, a + (s0 + (s1 - s0) * r) * h, touch);
            }
        }
        out
    }

    /// The t in `range` where the curve's point meets the quadric q.A.q + 2 b.q + c = 0, q = x - o:
    /// the roots of a Bernstein polynomial of twice the degree. `scale` is the form's rate per
    /// unit of distance, to read `near` in.
    pub fn quadric_roots(
        &self,
        o: Point3,
        a: &[[f64; 3]; 3],
        b: Vec3,
        c: f64,
        range: Interval,
        near: f64,
    ) -> Vec<(f64, bool)> {
        let mut out: Vec<(f64, bool)> = Vec::new();
        let p = self.degree;
        let mut found = Vec::new();
        for (i, s0, s1) in self.over(range) {
            if s1 <= s0 {
                continue;
            }
            let w = self.part(i, s0, s1);
            // a row per homogeneous coordinate of q, with weight
            let mut qs = [[0.0; MAX_DEGREE + 1]; 4];
            for j in 0..=p {
                qs[0][j] = w[j][0] - o.x * w[j][3];
                qs[1][j] = w[j][1] - o.y * w[j][3];
                qs[2][j] = w[j][2] - o.z * w[j][3];
                qs[3][j] = w[j][3];
            }
            let mut sum = [0.0; WORK];
            let mut prod = [0.0; WORK];
            let form = |x: usize, y: usize| match (x, y) {
                (3, 3) => c,
                (3, k) | (k, 3) => [b.x, b.y, b.z][k],
                (k, l) => a[k][l],
            };
            for x in 0..4 {
                for y in 0..4 {
                    let f = form(x, y);
                    if f == 0.0 {
                        continue;
                    }
                    bern_mul(&qs[x][..=p], &qs[y][..=p], &mut prod);
                    for j in 0..=2 * p {
                        sum[j] += f * prod[j];
                    }
                }
            }
            found.clear();
            bern_roots(&sum[..=2 * p], near, &mut found);
            let (a0, h) = (self.breaks[i], self.breaks[i + 1] - self.breaks[i]);
            for &(r, touch) in &found {
                push_root(&mut out, a0 + (s0 + (s1 - s0) * r) * h, touch);
            }
        }
        out
    }

    /// The interior breaks in `range`, where the curve's parameter is at most once differentiable.
    pub fn knots_in(&self, range: Interval) -> Vec<f64> {
        self.breaks[1..self.pieces()]
            .iter()
            .copied()
            .filter(|&b| b > range.lo && b < range.hi)
            .collect()
    }

    /// x dy - y dx over the curve, halved: the curve's share of a closed loop's area, by Gauss
    /// (exact without weights: the integrand has degree 2 p - 1).
    pub fn area_term(&self) -> f64 {
        let rule = forge_math::poly::gauss_legendre(if self.rational { 16 } else { self.degree + 1 });
        let mut sum = 0.0;
        for i in 0..self.pieces() {
            let (a, h) = (self.breaks[i], self.breaks[i + 1] - self.breaks[i]);
            for &(x, w) in &rule {
                let d = self.deriv(a + h * 0.5 * (x + 1.0));
                sum += 0.5 * h * w * 0.5 * (d.p.x * d.d1.y - d.p.y * d.d1.x);
            }
        }
        sum
    }

    /// `n` points in parameter order, the first at the start, the last short of the end.
    pub fn samples(&self, n: usize) -> Vec<Point3> {
        let per = n.div_ceil(self.pieces()).max(1);
        let mut out = Vec::with_capacity(per * self.pieces());
        for i in 0..self.pieces() {
            let (a, h) = (self.breaks[i], self.breaks[i + 1] - self.breaks[i]);
            for k in 0..per {
                out.push(self.point(a + h * k as f64 / per as f64));
            }
        }
        out
    }
}

fn push_root(out: &mut Vec<(f64, bool)>, t: f64, touch: bool) {
    if let Some(l) = out.last_mut()
        && (t - l.0).abs() <= 1e-12 * (1.0 + t.abs())
    {
        l.1 = l.1 && touch;
        return;
    }
    out.push((t, touch));
}

fn plane_clear(b: &Box3, n: Vec3, d: f64, near: f64) -> bool {
    let (c, e) = (
        b.center().to_vec(),
        Vec3::new(
            0.5 * b.x.width() * n.x.abs(),
            0.5 * b.y.width() * n.y.abs(),
            0.5 * b.z.width() * n.z.abs(),
        ),
    );
    (n.dot(c) + d).abs() > e.x + e.y + e.z + near
}

fn box_gap_sq(b: &Box3, p: Point3) -> f64 {
    let g = |i: Interval, x: f64| {
        if x < i.lo {
            i.lo - x
        } else if x > i.hi {
            x - i.hi
        } else {
            0.0
        }
    };
    let (x, y, z) = (g(b.x, p.x), g(b.y, p.y), g(b.z, p.z));
    x * x + y * y + z * z
}

fn insert_knot(u: &mut Vec<f64>, p: &mut Vec<H>, degree: usize, k: f64) {
    // k lies in the span [u[s], u[s + 1])
    let s = u.partition_point(|&x| x <= k) - 1;
    let s = s.min(p.len() - 1);
    let mult = u.iter().filter(|&&x| x == k).count();
    let mut q = Vec::with_capacity(p.len() + 1);
    q.extend_from_slice(&p[..=s - degree]);
    for i in (s - degree + 1)..=(s - mult) {
        let den = u[i + degree] - u[i];
        let a = if den > 0.0 { (k - u[i]) / den } else { 0.0 };
        q.push(lerp(&p[i - 1], &p[i], a));
    }
    q.extend_from_slice(&p[s - mult..]);
    *p = q;
    u.insert(s + 1, k);
}

fn tridiagonal(rows: &[(f64, f64, f64, Vec3)]) -> Vec<Vec3> {
    let n = rows.len();
    let mut c = vec![0.0; n];
    let mut d = vec![Vec3::ZERO; n];
    let (_, b0, c0, d0) = rows[0];
    c[0] = c0 / b0;
    d[0] = d0 / b0;
    for i in 1..n {
        let (a, b, cc, dd) = rows[i];
        let m = b - a * c[i - 1];
        c[i] = cc / m;
        d[i] = (dd - d[i - 1] * a) / m;
    }
    for i in (0..n - 1).rev() {
        d[i] = d[i] - d[i + 1] * c[i];
    }
    d
}

/// A cyclic tridiagonal system (Sherman-Morrison on the ordinary one).
fn cyclic(rows: &[(f64, f64, f64, Vec3)]) -> Vec<Vec3> {
    let n = rows.len();
    if n == 2 {
        // two unknowns each coupled twice
        let (a0, b0, c0, d0) = rows[0];
        let (a1, b1, c1, d1) = rows[1];
        let (p, q, r, s) = (b0, a0 + c0, a1 + c1, b1);
        let det = p * s - q * r;
        return vec![(d0 * s - d1 * q) / det, (d1 * p - d0 * r) / det];
    }
    let (alpha, beta) = (rows[n - 1].2, rows[0].0);
    let gamma = -rows[0].1;
    let mut plain: Vec<(f64, f64, f64, Vec3)> = rows.to_vec();
    plain[0].1 -= gamma;
    plain[n - 1].1 -= alpha * beta / gamma;
    plain[0].0 = 0.0;
    plain[n - 1].2 = 0.0;
    let x = tridiagonal(&plain);
    let unit: Vec<(f64, f64, f64, Vec3)> = plain
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let v = if i == 0 {
                gamma
            } else if i == n - 1 {
                alpha
            } else {
                0.0
            };
            (r.0, r.1, r.2, Vec3::new(v, 0.0, 0.0))
        })
        .collect();
    let z: Vec<f64> = tridiagonal(&unit).iter().map(|v| v.x).collect();
    let fact = (x[0] + x[n - 1] * (beta / gamma)) / (1.0 + z[0] + z[n - 1] * beta / gamma);
    (0..n).map(|i| x[i] - fact * z[i]).collect()
}

/// How a sketch's curve was given, as the part file writes it.
#[derive(Clone, Debug, PartialEq)]
pub enum Form {
    /// Degree, control points, weights where not all 1, knots where not uniform and clamped.
    Control {
        degree: usize,
        pts: Vec<Point2>,
        knots: Option<Vec<f64>>,
        weights: Option<Vec<f64>>,
    },
    /// The cubic through the points, with slopes at its ends where given.
    Through {
        pts: Vec<Point2>,
        from: Option<Vec2>,
        to: Option<Vec2>,
    },
    /// Semi-axes a along the angle `rot` and b across; from angle `start` through `sweep`
    /// (each measured in the ellipse's own frame).
    Ellipse {
        center: Point2,
        a: f64,
        b: f64,
        rot: f64,
        start: f64,
        sweep: f64,
    },
}

/// A curve in a sketch plane: the spline, z = 0, and how it was given.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaneSpline {
    pub c: Arc<Spline>,
    pub form: Form,
}

/// A uniform clamped knot vector for n control points of `degree`.
pub fn uniform_knots(degree: usize, n: usize) -> Vec<f64> {
    let inner = n.saturating_sub(degree + 1);
    let mut k = vec![0.0; degree + 1];
    k.extend((1..=inner).map(|i| i as f64));
    k.extend(std::iter::repeat_n((inner + 1) as f64, degree + 1));
    k
}

fn p3(p: Point2) -> Point3 {
    Point3::new(p.x, p.y, 0.0)
}

impl PlaneSpline {
    pub fn control(
        degree: usize,
        pts: Vec<Point2>,
        knots: Option<Vec<f64>>,
        weights: Option<Vec<f64>>,
    ) -> Option<PlaneSpline> {
        if weights
            .as_ref()
            .is_some_and(|w| w.iter().any(|&x| x <= 0.0 || x.is_nan()))
        {
            return None;
        }
        let k = knots.clone().unwrap_or_else(|| uniform_knots(degree, pts.len()));
        let ctrl: Vec<Point3> = pts.iter().map(|&p| p3(p)).collect();
        let c = Spline::from_nurbs(degree, &k, &ctrl, weights.as_deref())?;
        Some(PlaneSpline {
            c: Arc::new(c),
            form: Form::Control {
                degree,
                pts,
                knots,
                weights,
            },
        })
    }
    pub fn through(pts: Vec<Point2>, from: Option<Vec2>, to: Option<Vec2>) -> Option<PlaneSpline> {
        let ctrl: Vec<Point3> = pts.iter().map(|&p| p3(p)).collect();
        let v = |d: Vec2| Vec3::new(d.x, d.y, 0.0);
        let c = Spline::through(&ctrl, from.map(v), to.map(v))?;
        Some(PlaneSpline {
            c: Arc::new(c),
            form: Form::Through { pts, from, to },
        })
    }
    pub fn ellipse(center: Point2, a: f64, b: f64, rot: f64, start: f64, sweep: f64) -> Option<PlaneSpline> {
        if !(a > 0.0 && b > 0.0 && a.is_finite() && b.is_finite()) {
            return None;
        }
        let (s, c) = dmath::sin_cos(rot);
        let (x, y) = (Vec3::new(c, s, 0.0) * a, Vec3::new(-s, c, 0.0) * b);
        Some(PlaneSpline {
            c: Arc::new(Spline::ellipse(p3(center), x, y, start, sweep)?),
            form: Form::Ellipse {
                center,
                a,
                b,
                rot,
                start,
                sweep,
            },
        })
    }
    pub fn start(&self) -> Point2 {
        let (a, _) = self.c.ends();
        Point2::new(a.x, a.y)
    }
    pub fn end(&self) -> Point2 {
        let (_, b) = self.c.ends();
        Point2::new(b.x, b.y)
    }
    pub fn closed(&self) -> bool {
        self.start().distance(self.end()) <= tol::LINEAR
    }
    /// The same curve run the other way.
    pub fn reversed(&self) -> PlaneSpline {
        let form = match &self.form {
            Form::Control {
                degree,
                pts,
                knots,
                weights,
            } => Form::Control {
                degree: *degree,
                pts: pts.iter().rev().copied().collect(),
                knots: knots.as_ref().map(|k| {
                    let (lo, hi) = (k[0], k[k.len() - 1]);
                    k.iter().rev().map(|x| lo + hi - x).collect()
                }),
                weights: weights.as_ref().map(|w| w.iter().rev().copied().collect()),
            },
            Form::Through { pts, from, to } => Form::Through {
                pts: pts.iter().rev().copied().collect(),
                from: to.map(|d| -d),
                to: from.map(|d| -d),
            },
            &Form::Ellipse {
                center,
                a,
                b,
                rot,
                start,
                sweep,
            } => Form::Ellipse {
                center,
                a,
                b,
                rot,
                start: start + sweep,
                sweep: -sweep,
            },
        };
        PlaneSpline {
            c: Arc::new(self.c.reversed()),
            form,
        }
    }
    /// The curve mirrored in the sketch's x axis.
    pub fn mirrored(&self) -> PlaneSpline {
        let m = |p: &Point2| Point2::new(p.x, -p.y);
        let v = |d: &Vec2| Vec2::new(d.x, -d.y);
        let form = match &self.form {
            Form::Control {
                degree,
                pts,
                knots,
                weights,
            } => Form::Control {
                degree: *degree,
                pts: pts.iter().map(m).collect(),
                knots: knots.clone(),
                weights: weights.clone(),
            },
            Form::Through { pts, from, to } => Form::Through {
                pts: pts.iter().map(m).collect(),
                from: from.as_ref().map(v),
                to: to.as_ref().map(v),
            },
            &Form::Ellipse {
                center,
                a,
                b,
                rot,
                start,
                sweep,
            } => Form::Ellipse {
                center: m(&center),
                a,
                b,
                rot: -rot,
                start: -start,
                sweep: -sweep,
            },
        };
        PlaneSpline {
            c: Arc::new(self.c.mapped(|p| Point3::new(p.x, -p.y, p.z))),
            form,
        }
    }
    /// The curve grown k times about the origin.
    pub fn scaled(&self, k: f64) -> PlaneSpline {
        self.remade(&|p| Point2::new(p.x * k, p.y * k), &|r| r * k, &|a| a, false)
    }
    /// The curve made afresh from its data with points sent through `pt`, lengths through `len`,
    /// angles through `ang` (a reflection: `flip`, the arc's parameter runs the other way).
    pub fn remade(
        &self,
        pt: &dyn Fn(Point2) -> Point2,
        len: &dyn Fn(f64) -> f64,
        ang: &dyn Fn(f64) -> f64,
        flip: bool,
    ) -> PlaneSpline {
        let again = match &self.form {
            Form::Control {
                degree,
                pts,
                knots,
                weights,
            } => PlaneSpline::control(
                *degree,
                pts.iter().map(|&p| pt(p)).collect(),
                knots.clone(),
                weights.clone(),
            ),
            Form::Through { pts, from, to } => {
                let v = |d: &Vec2| pt(Point2::ORIGIN + *d) - pt(Point2::ORIGIN);
                PlaneSpline::through(
                    pts.iter().map(|&p| pt(p)).collect(),
                    from.as_ref().map(v),
                    to.as_ref().map(v),
                )
            }
            &Form::Ellipse {
                center,
                a,
                b,
                rot,
                start,
                sweep,
            } => {
                let f = if flip { -1.0 } else { 1.0 };
                PlaneSpline::ellipse(pt(center), len(a), len(b), ang(rot), f * start, f * sweep)
            }
        };
        again.expect("a curve moved is a curve")
    }
    /// The stretch of the curve over `range`, given by its own Bezier pieces.
    pub fn part(&self, range: Interval) -> Option<PlaneSpline> {
        let c = self.c.stretch(range)?;
        let (degree, knots, pts, weights) = c.control_form();
        Some(PlaneSpline {
            c: Arc::new(c),
            form: Form::Control {
                degree,
                pts: pts.iter().map(|p| Point2::new(p.x, p.y)).collect(),
                knots: Some(knots),
                weights,
            },
        })
    }
}

/// A piece of a spline wall as a rational Bezier patch: `nu` by `nv` homogeneous control points in
/// world coordinates, rows along u. It lies in the hull of its points (weights are positive).
pub struct CurveHull {
    pub(crate) nu: usize,
    pub(crate) nv: usize,
    pub(crate) pts: Vec<H>,
}

impl CurveHull {
    fn at(&self, i: usize, j: usize) -> Point3 {
        point_of(&self.pts[j * self.nu + i])
    }
    pub(crate) fn bbox(&self) -> Box3 {
        Box3::from_points(self.pts.iter().map(point_of))
    }
    pub(crate) fn centre(&self) -> Vec3 {
        let n = self.pts.len() as f64;
        self.pts.iter().fold(Vec3::ZERO, |s, h| s + point_of(h).to_vec()) / n
    }
    pub(crate) fn along_u(&self) -> f64 {
        let (u, v) = (self.nu - 1, self.nv - 1);
        self.at(u, 0)
            .distance(self.at(0, 0))
            .max(self.at(u, v).distance(self.at(0, v)))
    }
    pub(crate) fn along_v(&self) -> f64 {
        let (u, v) = (self.nu - 1, self.nv - 1);
        self.at(0, v)
            .distance(self.at(0, 0))
            .max(self.at(u, v).distance(self.at(u, 0)))
    }
    pub(crate) fn normal(&self) -> Vec3 {
        let (u, v) = (self.nu - 1, self.nv - 1);
        (self.at(u, v / 2) - self.at(0, v / 2)).cross(self.at(u / 2, v) - self.at(u / 2, 0))
    }
    pub(crate) fn extent(&self, u: Vec3) -> Interval {
        self.pts.iter().fold(Interval::EMPTY, |mut i, h| {
            i.include(point_of(h).to_vec().dot(u));
            i
        })
    }
    /// The quadric q.A.q + 2 b.q + c has one sign over the patch: its value times w^2 is a
    /// polynomial of bidegree (2 (nu - 1), 2 (nv - 1)), every Bernstein coefficient the same sign.
    pub(crate) fn misses(&self, q: &crate::meet::Alg) -> bool {
        let crate::meet::Alg::Quadric { o, a, b, c } = *q else {
            return false;
        };
        let (nu, nv) = (self.nu, self.nv);
        let rel: Vec<(Vec3, f64)> = self
            .pts
            .iter()
            .map(|h| {
                (
                    Vec3::new(h[0] - o.x * h[3], h[1] - o.y * h[3], h[2] - o.z * h[3]),
                    h[3],
                )
            })
            .collect();
        let value = |x: &(Vec3, f64), y: &(Vec3, f64)| {
            let ay = crate::meet::mul(&a, y.0);
            x.0.dot(ay) + b.dot(x.0) * y.1 + b.dot(y.0) * x.1 + c * x.1 * y.1
        };
        // at the corners the coefficients are the quadric's own values: opposite signs settle it
        let first = value(&rel[0], &rel[0]);
        for k in [nu - 1, (nv - 1) * nu, nv * nu - 1] {
            if value(&rel[k], &rel[k]) * first < 0.0 {
                return false;
            }
        }
        let (mu, mv) = (2 * nu - 1, 2 * nv - 1);
        let mut coef = vec![0.0; mu * mv];
        let (du, dv) = (nu - 1, nv - 1);
        for j in 0..nv {
            for i in 0..nu {
                let x = &rel[j * nu + i];
                for l in 0..nv {
                    for k in 0..nu {
                        coef[(j + l) * mu + i + k] += value(x, &rel[l * nu + k])
                            * BINOM[du][i]
                            * BINOM[du][k]
                            * BINOM[dv][j]
                            * BINOM[dv][l];
                    }
                }
            }
        }
        for l in 0..mv {
            for k in 0..mu {
                coef[l * mu + k] /= BINOM[2 * du][k] * BINOM[2 * dv][l];
            }
        }
        let eps = 1e-12 * coef.iter().fold(0.0f64, |m, x| m.max(x.abs()));
        coef.iter().all(|&x| x > eps) || coef.iter().all(|&x| x < -eps)
    }
}

/// A spline as an edge's curve, in world coordinates, over `dom` (the spline carried on where
/// that reaches past its ends).
#[derive(Clone, Debug)]
pub struct SplineCurve {
    pub c: Arc<Spline>,
    pub dom: Interval,
}

impl SplineCurve {
    pub fn curve(c: Arc<Spline>, dom: Interval) -> crate::Curve {
        crate::Curve::Procedural(ProcCurve(Arc::new(SplineCurve { c, dom })))
    }
}

/// The spline of a curve made of one.
pub fn spline_of(c: &crate::Curve) -> Option<&SplineCurve> {
    match c {
        crate::Curve::Procedural(p) if p.is_spline() => p.as_any()?.downcast_ref::<SplineCurve>(),
        _ => None,
    }
}

impl ProceduralCurve for SplineCurve {
    fn is_spline(&self) -> bool {
        true
    }
    fn deriv(&self, t: f64) -> CurveDeriv {
        self.c.deriv(t)
    }
    fn point(&self, t: f64) -> Point3 {
        self.c.point(t)
    }
    fn domain(&self) -> Domain1 {
        Domain1 {
            lo: self.dom.lo,
            hi: self.dom.hi,
            period: None,
        }
    }
    fn closest(&self, p: Point3, range: Interval) -> f64 {
        self.c.closest(p, range)
    }
    fn bbox(&self, range: Interval) -> Box3 {
        self.c.bbox(range)
    }
    fn transformed(&self, f: &Frame) -> ProcCurve {
        ProcCurve(Arc::new(SplineCurve {
            c: Arc::new(self.c.mapped(|p| f.point_to_world(p))),
            dom: self.dom,
        }))
    }
    fn speed_bound(&self, range: Interval) -> f64 {
        self.c.derivative_bounds(range)[0]
    }
    fn segments(&self, range: Interval, chord: f64) -> usize {
        let mut n = 0;
        for (i, s0, s1) in self.c.over(range) {
            if s1 <= s0 {
                continue;
            }
            let h = self.c.breaks[i + 1] - self.c.breaks[i];
            let w = (s1 - s0) * h;
            let a = self.c.breaks[i] + s0 * h;
            let m = self.c.derivative_bounds(Interval::new(a, a + w))[1];
            n += ((m * w * w / (8.0 * chord.max(1e-12))).sqrt().ceil() as usize).max(1);
        }
        n.max(1)
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
mod tests;

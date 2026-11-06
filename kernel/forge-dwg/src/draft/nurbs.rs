//! NURBS curves in the plane: evaluation, knot insertion, splitting, clamping, Bezier spans and
//! interpolation through fit points (after Piegl and Tiller's algorithms, written from the maths).

use super::geom::{Affine, Bbox, V2, v2};

/// A NURBS curve: `knots` has `ctrl.len() + degree + 1` values (clamped or not); the curve runs
/// over `[knots[degree], knots[ctrl.len()]]`. `weights`, when present, has one positive weight per
/// control point. `closed`: the curve's ends meet, and a piece of it may wrap from the domain's
/// end to its start (a periodic spline, or a clamped one that closes).
#[derive(Clone, Debug, PartialEq)]
pub struct Nurbs {
    pub degree: usize,
    pub knots: Vec<f64>,
    pub ctrl: Vec<V2>,
    pub weights: Option<Vec<f64>>,
    pub closed: bool,
}

/// A homogeneous control point: (w x, w y, w).
pub(crate) type H = [f64; 3];

fn hom(p: V2, w: f64) -> H {
    [p.x * w, p.y * w, w]
}
fn dehom(h: H) -> V2 {
    v2(h[0] / h[2], h[1] / h[2])
}
fn hlerp(a: H, b: H, t: f64) -> H {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// One rational Bezier piece of a curve: parameters `u0..u1` and homogeneous control points.
#[derive(Clone, Debug)]
pub(crate) struct Bez {
    pub u0: f64,
    pub u1: f64,
    pub h: Vec<H>,
}

impl Bez {
    pub fn pts(&self) -> impl Iterator<Item = V2> + '_ {
        self.h.iter().map(|&h| dehom(h))
    }
    pub fn bbox(&self) -> Bbox {
        Bbox::of(self.pts())
    }
    /// The two halves at the middle (de Casteljau).
    pub fn halve(&self) -> (Bez, Bez) {
        let n = self.h.len();
        let mut a = Vec::with_capacity(n);
        let mut b = vec![[0.0; 3]; n];
        let mut work = self.h.clone();
        a.push(work[0]);
        b[n - 1] = work[n - 1];
        for k in 1..n {
            for i in 0..n - k {
                work[i] = hlerp(work[i], work[i + 1], 0.5);
            }
            a.push(work[0]);
            b[n - 1 - k] = work[n - 1 - k];
        }
        let um = 0.5 * (self.u0 + self.u1);
        (
            Bez {
                u0: self.u0,
                u1: um,
                h: a,
            },
            Bez {
                u0: um,
                u1: self.u1,
                h: b,
            },
        )
    }
    /// How far the control polygon strays from its chord.
    pub fn flatness(&self) -> f64 {
        let p: Vec<V2> = self.pts().collect();
        let (a, b) = (p[0], p[p.len() - 1]);
        let d = b - a;
        let l = d.len();
        p.iter()
            .map(|&q| {
                if l > 0.0 {
                    (q - a).cross(d).abs() / l
                } else {
                    q.dist(a)
                }
            })
            .fold(0.0, f64::max)
    }
}

impl Nurbs {
    pub fn n(&self) -> usize {
        self.ctrl.len()
    }
    pub fn w(&self, i: usize) -> f64 {
        self.weights.as_ref().map_or(1.0, |w| w[i])
    }
    pub fn is_rational(&self) -> bool {
        self.weights
            .as_ref()
            .is_some_and(|w| w.iter().any(|&x| x != w[0]))
    }
    /// Whether the knots, control points and weights fit together.
    pub fn is_valid(&self) -> bool {
        let p = self.degree;
        p >= 1
            && self.n() > p
            && self.knots.len() == self.n() + p + 1
            && self.knots.windows(2).all(|k| k[0] <= k[1])
            && self.knots[p] < self.knots[self.n()]
            && self
                .weights
                .as_ref()
                .is_none_or(|w| w.len() == self.n() && w.iter().all(|&x| x > 0.0))
            && self.ctrl.iter().all(|c| c.is_finite())
            && self.knots.iter().all(|k| k.is_finite())
    }
    pub fn domain(&self) -> (f64, f64) {
        (self.knots[self.degree], self.knots[self.n()])
    }
    fn hpt(&self, i: usize) -> H {
        hom(self.ctrl[i], self.w(i))
    }

    /// The knot span holding `u` (clamped to the domain): k with knots[k] <= u < knots[k + 1].
    pub fn span(&self, u: f64) -> usize {
        let p = self.degree;
        let n = self.n();
        if u >= self.knots[n] {
            // the last non-empty span
            let mut k = n - 1;
            while k > p && self.knots[k] == self.knots[k + 1] {
                k -= 1;
            }
            return k;
        }
        if u <= self.knots[p] {
            let mut k = p;
            while k < n - 1 && self.knots[k] == self.knots[k + 1] {
                k += 1;
            }
            return k;
        }
        let (mut lo, mut hi) = (p, n);
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if u < self.knots[mid] {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        lo
    }

    /// The non-zero basis functions at `u` and their derivatives up to `nd`: ders[k][j] is the
    /// k-th derivative of N(span - p + j).
    fn basis_ders(&self, span: usize, u: f64, nd: usize) -> Vec<Vec<f64>> {
        let p = self.degree;
        let k = &self.knots;
        let mut ndu = vec![vec![0.0; p + 1]; p + 1];
        let mut left = vec![0.0; p + 1];
        let mut right = vec![0.0; p + 1];
        ndu[0][0] = 1.0;
        for j in 1..=p {
            left[j] = u - k[span + 1 - j];
            right[j] = k[span + j] - u;
            let mut saved = 0.0;
            for r in 0..j {
                ndu[j][r] = right[r + 1] + left[j - r];
                let temp = if ndu[j][r] != 0.0 {
                    ndu[r][j - 1] / ndu[j][r]
                } else {
                    0.0
                };
                ndu[r][j] = saved + right[r + 1] * temp;
                saved = left[j - r] * temp;
            }
            ndu[j][j] = saved;
        }
        let nd = nd.min(p);
        let mut ders = vec![vec![0.0; p + 1]; nd + 1];
        for j in 0..=p {
            ders[0][j] = ndu[j][p];
        }
        let mut a = vec![vec![0.0; p + 1]; 2];
        for r in 0..=p {
            let (mut s1, mut s2) = (0usize, 1usize);
            a[0][0] = 1.0;
            for kk in 1..=nd {
                let mut d = 0.0;
                let rk = r as isize - kk as isize;
                let pk = p - kk;
                if r >= kk {
                    let den = ndu[pk + 1][rk as usize];
                    a[s2][0] = if den != 0.0 { a[s1][0] / den } else { 0.0 };
                    d = a[s2][0] * ndu[rk as usize][pk];
                }
                let j1 = if rk >= -1 { 1 } else { (-rk) as usize };
                let j2 = if (r as isize - 1) <= pk as isize {
                    kk - 1
                } else {
                    p - r
                };
                for j in j1..=j2 {
                    let idx = (rk + j as isize) as usize;
                    let den = ndu[pk + 1][idx];
                    a[s2][j] = if den != 0.0 {
                        (a[s1][j] - a[s1][j - 1]) / den
                    } else {
                        0.0
                    };
                    d += a[s2][j] * ndu[idx][pk];
                }
                if r <= pk {
                    let den = ndu[pk + 1][r];
                    a[s2][kk] = if den != 0.0 { -a[s1][kk - 1] / den } else { 0.0 };
                    d += a[s2][kk] * ndu[r][pk];
                }
                ders[kk][r] = d;
                std::mem::swap(&mut s1, &mut s2);
            }
        }
        // row k times p! / (p - k)!
        let mut f = p as f64;
        for (kk, row) in ders.iter_mut().enumerate().skip(1) {
            for x in row.iter_mut() {
                *x *= f;
            }
            f *= (p - kk) as f64;
        }
        ders
    }

    /// The point and its first two derivatives at `u`.
    pub fn derivs(&self, u: f64) -> (V2, V2, V2) {
        let p = self.degree;
        let span = self.span(u);
        let d = self.basis_ders(span, u, 2);
        // homogeneous derivatives
        let mut a = [[0.0f64; 3]; 3];
        for (k, row) in d.iter().enumerate() {
            for j in 0..=p {
                let h = self.hpt(span - p + j);
                for c in 0..3 {
                    a[k][c] += row[j] * h[c];
                }
            }
        }
        let w = a[0][2];
        let pt = v2(a[0][0] / w, a[0][1] / w);
        let a1 = v2(a[1][0], a[1][1]);
        let w1 = a[1][2];
        let d1 = (a1 - pt * w1) / w;
        let (a2, w2) = (v2(a[2][0], a[2][1]), a[2][2]);
        let d2 = (a2 - d1 * (2.0 * w1) - pt * w2) / w;
        (pt, d1, d2)
    }

    pub fn point(&self, u: f64) -> V2 {
        // de Boor in homogeneous coordinates
        let p = self.degree;
        let span = self.span(u);
        let mut d: Vec<H> = (0..=p).map(|j| self.hpt(span - p + j)).collect();
        for r in 1..=p {
            for j in (r..=p).rev() {
                let i = span - p + j;
                let den = self.knots[i + p + 1 - r] - self.knots[i];
                let a = if den != 0.0 {
                    (u - self.knots[i]) / den
                } else {
                    0.0
                };
                d[j] = hlerp(d[j - 1], d[j], a);
            }
        }
        dehom(d[p])
    }

    /// The curve with `u` inserted `times` more times (Boehm), in homogeneous coordinates.
    pub fn insert(&self, u: f64, times: usize) -> Nurbs {
        let mut c = self.clone();
        for _ in 0..times {
            c = c.insert_once(u);
        }
        c
    }

    fn insert_once(&self, u: f64) -> Nurbs {
        let p = self.degree;
        let n = self.n();
        // the span with knots[k] <= u < knots[k+1], counting existing copies of u
        let mut k = p;
        while k + 1 < self.knots.len() && self.knots[k + 1] <= u {
            k += 1;
        }
        let k = k.min(n - 1).max(p);
        let mut h: Vec<H> = Vec::with_capacity(n + 1);
        for i in 0..=n {
            if i + p <= k {
                h.push(self.hpt(i));
            } else if i > k {
                h.push(self.hpt(i - 1));
            } else {
                let den = self.knots[i + p] - self.knots[i];
                let a = if den != 0.0 {
                    (u - self.knots[i]) / den
                } else {
                    0.0
                };
                h.push(hlerp(self.hpt(i - 1), self.hpt(i), a));
            }
        }
        let mut knots = self.knots.clone();
        knots.insert(k + 1, u);
        Nurbs::from_h(p, knots, &h, self.weights.is_some(), self.closed)
    }

    fn from_h(degree: usize, knots: Vec<f64>, h: &[H], weighted: bool, closed: bool) -> Nurbs {
        Nurbs {
            degree,
            knots,
            ctrl: h.iter().map(|&x| dehom(x)).collect(),
            weights: weighted.then(|| h.iter().map(|x| x[2]).collect()),
            closed,
        }
    }

    fn mult(&self, u: f64) -> usize {
        self.knots.iter().filter(|&&k| k == u).count()
    }

    /// The same curve over the same domain, with end knots of multiplicity degree + 1.
    pub fn clamped(&self) -> Nurbs {
        let (a, b) = self.domain();
        let mut c = self.clone();
        c.closed = false;
        c.cut_before(a);
        c.cut_after(b);
        c
    }

    /// Drops what comes before `u` (a parameter in the domain), clamping there.
    fn cut_before(&mut self, u: f64) {
        let p = self.degree;
        let m = self.mult(u).min(p);
        *self = self.insert(u, p - m);
        let t = self.knots.iter().rposition(|&k| k == u).unwrap_or(p);
        let mut knots = vec![u; p + 1];
        knots.extend_from_slice(&self.knots[t + 1..]);
        self.ctrl.drain(..t - p);
        if let Some(w) = &mut self.weights {
            w.drain(..t - p);
        }
        self.knots = knots;
    }

    /// Drops what comes after `u`, clamping there.
    fn cut_after(&mut self, u: f64) {
        let p = self.degree;
        let m = self.mult(u).min(p);
        *self = self.insert(u, p - m);
        let s = self.knots.iter().position(|&k| k == u).unwrap_or(self.n());
        let mut knots = self.knots[..s].to_vec();
        knots.extend(std::iter::repeat_n(u, p + 1));
        self.ctrl.truncate(s);
        if let Some(w) = &mut self.weights {
            w.truncate(s);
        }
        self.knots = knots;
    }

    /// The piece over `[u0, u1]` (inside the domain, u0 < u1), clamped.
    pub fn sub(&self, u0: f64, u1: f64) -> Nurbs {
        let (a, b) = self.domain();
        let (u0, u1) = (u0.max(a), u1.min(b));
        let mut c = self.clone();
        c.closed = false;
        c.cut_before(u0);
        c.cut_after(u1);
        c
    }

    pub fn reversed(&self) -> Nurbs {
        let (a, b) = (self.knots[0], self.knots[self.knots.len() - 1]);
        Nurbs {
            degree: self.degree,
            knots: self.knots.iter().rev().map(|&k| a + b - k).collect(),
            ctrl: self.ctrl.iter().rev().copied().collect(),
            weights: self.weights.as_ref().map(|w| w.iter().rev().copied().collect()),
            closed: self.closed,
        }
    }

    pub fn transformed(&self, m: &Affine) -> Nurbs {
        Nurbs {
            ctrl: self.ctrl.iter().map(|&p| m.apply(p)).collect(),
            ..self.clone()
        }
    }

    /// The curve's Bezier pieces, in order.
    pub(crate) fn beziers(&self) -> Vec<Bez> {
        let p = self.degree;
        let mut c = self.clamped();
        let (a, b) = c.domain();
        let inner: Vec<f64> = {
            let mut v: Vec<f64> = c.knots.iter().copied().filter(|&k| k > a && k < b).collect();
            v.dedup();
            v
        };
        for &u in &inner {
            let m = c.mult(u).min(p);
            c = c.insert(u, p - m);
        }
        let mut out = vec![];
        let mut i = 0;
        let mut u0 = a;
        for &u in inner.iter().chain(std::iter::once(&b)) {
            let h: Vec<H> = (i..=i + p).map(|j| c.hpt(j)).collect();
            out.push(Bez { u0, u1: u, h });
            // a knot of multiplicity p shares one control point between pieces
            i += p;
            u0 = u;
        }
        out
    }

    /// The control points' box: holds the curve (weights positive).
    pub fn hull_box(&self) -> Bbox {
        Bbox::of(self.ctrl.iter().copied())
    }

    /// The distinct knot values inside the domain, with its ends: the spans' breaks.
    pub fn breaks(&self) -> Vec<f64> {
        let (a, b) = self.domain();
        let mut v: Vec<f64> = vec![a];
        for &k in &self.knots {
            if k > a && k < b && v.last() != Some(&k) {
                v.push(k);
            }
        }
        v.push(b);
        v
    }

    /// Joins `o` on the end of `self` (its start at self's end), degrees equal, both clamped.
    pub fn join(&self, o: &Nurbs) -> Option<Nurbs> {
        if self.degree != o.degree {
            return None;
        }
        let p = self.degree;
        let a = self.clamped();
        let b = o.clamped();
        let (_, ae) = a.domain();
        let (bs, _) = b.domain();
        // the second's weights scaled so the shared point's weights agree
        let ws = a.w(a.n() - 1) / b.w(0);
        let mut knots = a.knots[..a.knots.len() - 1].to_vec();
        knots.extend(b.knots[p + 1..].iter().map(|&k| k - bs + ae));
        let mut ctrl = a.ctrl.clone();
        ctrl.extend_from_slice(&b.ctrl[1..]);
        let weights = if a.weights.is_some() || b.weights.is_some() {
            let mut w: Vec<f64> = (0..a.n()).map(|i| a.w(i)).collect();
            w.extend((1..b.n()).map(|i| b.w(i) * ws));
            Some(w)
        } else {
            None
        };
        let r = Nurbs {
            degree: p,
            knots,
            ctrl,
            weights,
            closed: false,
        };
        r.is_valid().then_some(r)
    }

    /// Global interpolation through `pts` (Piegl and Tiller's 9.1, with 9.2's end derivatives when
    /// given), chord-length parameters (`sqrt_chord` the square roots, `uniform` equal steps).
    pub fn interpolate(
        pts: &[V2],
        degree: usize,
        d0: Option<V2>,
        d1: Option<V2>,
        param: Param,
    ) -> Option<Nurbs> {
        let m = pts.len();
        if m < 2 {
            return None;
        }
        let ends = d0.is_some() as usize + d1.is_some() as usize;
        let p = degree.min(m + ends - 1).max(1);
        // parameters
        let mut u = vec![0.0; m];
        for k in 1..m {
            let d = pts[k].dist(pts[k - 1]);
            u[k] = u[k - 1]
                + match param {
                    Param::Chord => d,
                    Param::SqrtChord => d.sqrt(),
                    Param::Uniform => 1.0,
                };
        }
        let total = u[m - 1];
        if total <= 0.0 {
            return None;
        }
        // the end derivatives are unit tangents scaled to the parameter length
        let n = m + ends;
        // knots by averaging (9.8), with the extra end conditions spread in (9.22-like)
        let mut knots = vec![0.0; p + 1];
        if ends == 0 {
            for j in 1..m - p {
                let s: f64 = u[j..j + p].iter().sum();
                knots.push(s / p as f64);
            }
        } else {
            // n - p - 1 interior knots: average over parameters including the duplicated ends
            let mut uu = vec![];
            if d0.is_some() {
                uu.push(u[0]);
            }
            uu.extend_from_slice(&u);
            if d1.is_some() {
                uu.push(u[m - 1]);
            }
            let ni = n - p - 1;
            for j in 1..=ni {
                let s: f64 = uu[j..j + p].iter().sum();
                knots.push(s / p as f64);
            }
        }
        knots.extend(std::iter::repeat_n(total, p + 1));
        if knots.len() != n + p + 1 {
            return None;
        }
        let proto = Nurbs {
            degree: p,
            knots: knots.clone(),
            ctrl: vec![V2::ZERO; n],
            weights: None,
            closed: false,
        };
        // the dense system: rows are the interpolation and derivative conditions
        let mut a = vec![vec![0.0; n]; n];
        let mut rhs = vec![V2::ZERO; n];
        let mut row = 0;
        let mut put = |a: &mut Vec<Vec<f64>>, rhs: &mut Vec<V2>, t: f64, k: usize, val: V2| {
            let span = proto.span(t);
            let ders = proto.basis_ders(span, t, 1);
            for j in 0..=p {
                a[row][span - p + j] = ders[k][j];
            }
            rhs[row] = val;
            row += 1;
        };
        put(&mut a, &mut rhs, u[0], 0, pts[0]);
        if let Some(d) = d0 {
            put(
                &mut a,
                &mut rhs,
                u[0],
                1,
                d.unit_or_zero() * scale_for(param, total, m),
            );
        }
        for k in 1..m - 1 {
            put(&mut a, &mut rhs, u[k], 0, pts[k]);
        }
        if let Some(d) = d1 {
            put(
                &mut a,
                &mut rhs,
                u[m - 1],
                1,
                d.unit_or_zero() * scale_for(param, total, m),
            );
        }
        put(&mut a, &mut rhs, u[m - 1], 0, pts[m - 1]);
        let ctrl = solve_dense(a, rhs)?;
        let r = Nurbs { ctrl, ..proto };
        r.is_valid().then_some(r)
    }

    /// Bisects `[lo, hi]` for where `f` changes sign (f(lo), f(hi) of opposite signs), Illinois
    /// false position.
    pub(crate) fn root(f: &dyn Fn(f64) -> f64, mut lo: f64, mut hi: f64, eps: f64) -> f64 {
        let (mut flo, mut fhi) = (f(lo), f(hi));
        let mut side = 0i8;
        for _ in 0..200 {
            if (hi - lo).abs() <= eps {
                break;
            }
            let mut x = (lo * fhi - hi * flo) / (fhi - flo);
            if !(x > lo.min(hi) && x < lo.max(hi)) {
                x = 0.5 * (lo + hi);
            }
            let fx = f(x);
            if fx == 0.0 {
                return x;
            }
            if (fx < 0.0) == (flo < 0.0) {
                lo = x;
                flo = fx;
                if side == -1 {
                    fhi *= 0.5;
                }
                side = -1;
            } else {
                hi = x;
                fhi = fx;
                if side == 1 {
                    flo *= 0.5;
                }
                side = 1;
            }
        }
        if flo.abs() < fhi.abs() { lo } else { hi }
    }
}

/// How fit points are given parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Param {
    Chord,
    SqrtChord,
    Uniform,
}

fn scale_for(param: Param, total: f64, m: usize) -> f64 {
    match param {
        Param::Chord => 1.0,
        // the derivative scale for unit chord steps
        _ => total / (m - 1).max(1) as f64,
    }
}

/// Gaussian elimination with partial pivoting.
fn solve_dense(mut a: Vec<Vec<f64>>, mut b: Vec<V2>) -> Option<Vec<V2>> {
    let n = b.len();
    for col in 0..n {
        let piv = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[piv][col].abs() < 1e-300 {
            return None;
        }
        a.swap(col, piv);
        b.swap(col, piv);
        for r in col + 1..n {
            let f = a[r][col] / a[col][col];
            if f != 0.0 {
                for c in col..n {
                    a[r][c] -= f * a[col][c];
                }
                b[r] = b[r] - b[col] * f;
            }
        }
    }
    let mut x = vec![V2::ZERO; n];
    for r in (0..n).rev() {
        let mut s = b[r];
        for c in r + 1..n {
            s = s - x[c] * a[r][c];
        }
        x[r] = s / a[r][r];
    }
    x.iter().all(|p| p.is_finite()).then_some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad() -> Nurbs {
        Nurbs {
            degree: 3,
            knots: vec![0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 2.0, 2.0, 2.0],
            ctrl: vec![
                v2(0.0, 0.0),
                v2(1.0, 2.0),
                v2(3.0, 2.0),
                v2(4.0, 0.0),
                v2(6.0, 1.0),
            ],
            weights: Some(vec![1.0, 0.5, 2.0, 1.0, 1.0]),
            closed: false,
        }
    }

    #[test]
    fn derivatives_match_differences() {
        let c = quad();
        for &u in &[0.1, 0.7, 1.0, 1.3, 1.9] {
            let (p, d1, d2) = c.derivs(u);
            assert!(p.dist(c.point(u)) < 1e-12);
            let h = 1e-5;
            let fd = (c.point(u + h) - c.point(u - h)) / (2.0 * h);
            assert!(fd.dist(d1) < 1e-6 * d1.len().max(1.0), "{u}: {fd:?} {d1:?}");
            let fd2 = (c.point(u + h) - p * 2.0 + c.point(u - h)) / (h * h);
            assert!(fd2.dist(d2) < 1e-3 * d2.len().max(1.0), "{u}: {fd2:?} {d2:?}");
        }
    }

    #[test]
    fn insert_split_keep_the_shape() {
        let c = quad();
        let d = c.insert(0.4, 2);
        for k in 0..=20 {
            let u = k as f64 / 10.0;
            assert!(c.point(u).dist(d.point(u)) < 1e-12);
        }
        let s = c.sub(0.3, 1.6);
        assert_eq!(s.domain(), (0.3, 1.6));
        for k in 0..=13 {
            let u = 0.3 + k as f64 * 0.1;
            assert!(c.point(u).dist(s.point(u)) < 1e-12, "{u}");
        }
        let r = c.reversed();
        assert!(r.point(0.5).dist(c.point(1.5)) < 1e-12);
        let bz = c.beziers();
        assert_eq!(bz.len(), 2);
        let (h0, _) = bz[0].halve();
        assert!(h0.pts().next().unwrap().dist(c.point(0.0)) < 1e-12);
        assert!(h0.pts().last().unwrap().dist(c.point(0.5)) < 1e-12);
    }

    #[test]
    fn unclamped_clamps() {
        let c = Nurbs {
            degree: 2,
            knots: (0..8).map(|k| k as f64).collect(),
            ctrl: vec![
                v2(0.0, 0.0),
                v2(1.0, 1.0),
                v2(2.0, 0.0),
                v2(3.0, 1.0),
                v2(4.0, 0.0),
            ],
            weights: None,
            closed: false,
        };
        let k = c.clamped();
        assert!(k.is_valid());
        for t in [2.0, 2.5, 3.7, 5.0] {
            assert!(c.point(t).dist(k.point(t)) < 1e-12);
        }
    }

    #[test]
    fn interpolation_passes_through() {
        let pts = [
            v2(0.0, 0.0),
            v2(1.0, 1.0),
            v2(3.0, 0.5),
            v2(4.0, 2.0),
            v2(5.0, 0.0),
        ];
        for d in [None, Some(v2(1.0, 0.0))] {
            let c = Nurbs::interpolate(&pts, 3, d, d, Param::Chord).unwrap();
            for p in pts {
                let best = (0..=4000)
                    .map(|k| c.point(c.domain().1 * k as f64 / 4000.0).dist(p))
                    .fold(f64::MAX, f64::min);
                assert!(best < 1e-2, "{p:?} {best}");
            }
            assert!(c.point(c.domain().0).dist(pts[0]) < 1e-12);
            assert!(c.point(c.domain().1).dist(pts[4]) < 1e-12);
        }
    }

    #[test]
    fn joins() {
        let c = quad();
        let a = c.sub(0.0, 0.8);
        let b = c.sub(0.8, 2.0);
        let j = a.join(&b).unwrap();
        assert!(j.point(1.5).dist(c.point(1.5)) < 1e-12);
        assert!(j.point(0.4).dist(c.point(0.4)) < 1e-12);
    }
}

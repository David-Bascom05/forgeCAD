//! A float guide for the root-cover proposer (untrusted, like every proposer): approximate real
//! roots in f64, and short dyadic breakpoints between them. Whatever it proposes is checked by
//! the exact `cover_ok` before it is used; a wrong guess only falls back to bisection.

/// Horner in f64.
fn evf(p: &[f64], x: f64) -> f64 {
    let mut acc = 0.0;
    let mut i = p.len();
    while i > 0 {
        i -= 1;
        acc = acc * x + p[i];
    }
    acc
}

fn derivf(p: &[f64]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::with_capacity(p.len().saturating_sub(1));
    let mut i = 1;
    while i < p.len() {
        out.push(p[i] * i as f64);
        i += 1;
    }
    out
}

fn trimf(p: &[f64]) -> &[f64] {
    let mut n = p.len();
    while n > 0 && p[n - 1] == 0.0 {
        n -= 1;
    }
    &p[..n]
}

/// The real roots of `p` (coefficients low to high) in `(lo, hi)`, approximately, ascending:
/// between consecutive roots of `p'` (recursively) `p` is monotone, so a sign change there is
/// one root, found by bisection. Roots where `p` only touches zero are missed (the proposer then
/// falls back).
pub fn real_roots(p: &[f64], lo: f64, hi: f64) -> Vec<f64> {
    let p = trimf(p);
    let n = p.len();
    if n <= 1 {
        return Vec::new();
    }
    let mut pts: Vec<f64> = Vec::with_capacity(n + 1);
    pts.push(lo);
    let crit = real_roots(&derivf(p), lo, hi);
    let mut i = 0;
    while i < crit.len() {
        pts.push(crit[i]);
        i += 1;
    }
    pts.push(hi);
    let mut out: Vec<f64> = Vec::new();
    let mut k = 0;
    while k + 1 < pts.len() {
        let (mut a, mut b) = (pts[k], pts[k + 1]);
        let (fa, fb) = (evf(p, a), evf(p, b));
        if fa == 0.0 {
            if out.last() != Some(&a) {
                out.push(a);
            }
        } else if fa * fb < 0.0 {
            let mut it = 0;
            let sa = fa < 0.0;
            while it < 200 && b - a > 0.0 {
                let m = 0.5 * (a + b);
                if m <= a || m >= b {
                    break;
                }
                if (evf(p, m) < 0.0) == sa {
                    a = m;
                } else {
                    b = m;
                }
                it += 1;
            }
            out.push(0.5 * (a + b));
        }
        k += 1;
    }
    if n >= 2 && evf(p, hi) == 0.0 && out.last() != Some(&hi) {
        out.push(hi);
    }
    out
}

/// A short dyadic strictly inside `(a, b)`, away from both ends by a quarter of the gap.
pub fn short_between(a: f64, b: f64) -> Option<f64> {
    let g = b - a;
    if !(g > 0.0) || !g.is_finite() {
        return None;
    }
    let (l, h) = (a + 0.25 * g, b - 0.25 * g);
    let m = 0.5 * (a + b);
    let mut k = -64;
    while k <= 1074 {
        let s = (2.0f64).powi(k);
        let v = (m * s).round() / s;
        if v > l && v < h {
            return Some(v);
        }
        k += 1;
    }
    None
}

/// Fujiwara's bound on the roots' magnitude, `2 · max |a_{n−k} / a_n|^{1/k}`, in f64, rounded
/// up a little (a proposal: the rays are checked exactly).
pub fn fujiwara(p: &[f64]) -> Option<f64> {
    let p = trimf(p);
    let n = p.len();
    if n < 2 {
        return None;
    }
    let an = p[n - 1];
    let mut m = 0.0f64;
    let mut k = 1;
    while k < n {
        let c = (p[n - 1 - k] / an).abs();
        let r = if k == n - 1 {
            (c / 2.0).powf(1.0 / k as f64)
        } else {
            c.powf(1.0 / k as f64)
        };
        m = m.max(r);
        k += 1;
    }
    let b = 2.0 * m * 1.01 + 1.0;
    if b.is_finite() {
        short_between(b, 2.0 * b)
    } else {
        None
    }
}

/// Breakpoints for a cover of `(−bound, bound)`: a short dyadic between each pair of consecutive
/// approximate roots (each gap then holds about one root), and, apart, the critical points (the
/// roots of `p'`): where a complex pair near the real line makes a gap fail its Descartes check,
/// breaking it at a critical point usually mends it. `None` when the guide has nothing sound to
/// offer.
pub fn cover_points(p: &[f64], bound: f64) -> Option<(Vec<f64>, Vec<f64>)> {
    if !bound.is_finite() || p.iter().any(|c| !c.is_finite()) {
        return None;
    }
    let rs = real_roots(p, -bound, bound);
    let mut out: Vec<f64> = Vec::with_capacity(rs.len());
    let mut i = 0;
    while i + 1 < rs.len() {
        out.push(short_between(rs[i], rs[i + 1])?);
        i += 1;
    }
    if out.windows(2).any(|w| w[0] >= w[1]) || out.iter().any(|x| *x <= -bound || *x >= bound) {
        return None;
    }
    let mut cs = real_roots(&derivf(trimf(p)), -bound, bound);
    cs.retain(|c| *c > -bound && *c < bound && !rs.contains(c));
    Some((out, cs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roots_of_a_cubic() {
        // (x − 1)(x + 2)(x − 3) = x³ − 2x² − 5x + 6
        let r = real_roots(&[6.0, -5.0, -2.0, 1.0], -10.0, 10.0);
        assert_eq!(r.len(), 3);
        assert!((r[0] + 2.0).abs() < 1e-12 && (r[1] - 1.0).abs() < 1e-12 && (r[2] - 3.0).abs() < 1e-12);
        // breakpoints separate the roots, one root per gap
        let (c, _) = cover_points(&[6.0, -5.0, -2.0, 1.0], 10.0).unwrap();
        let mut ends = vec![-10.0];
        ends.extend(c.iter().copied());
        ends.push(10.0);
        for w in ends.windows(2) {
            let n = r.iter().filter(|x| **x > w[0] && **x < w[1]).count();
            assert!(n <= 1, "{ends:?}");
        }
        assert!(fujiwara(&[6.0, -5.0, -2.0, 1.0]).unwrap() >= 3.0);
    }
}

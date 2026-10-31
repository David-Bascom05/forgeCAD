//! Real roots of polynomials. Coefficients are in ascending order:
//! `c[0] + c[1] x + c[2] x^2 + ...`.

/// A real root and whether the polynomial touches zero there without crossing
/// (an even-multiplicity root: a tangency in geometric terms).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Root {
    pub x: f64,
    pub touching: bool,
}

pub fn eval(c: &[f64], x: f64) -> f64 {
    c.iter().rev().fold(0.0, |acc, &a| acc * x + a)
}

/// Horner value and a bound on its rounding error.
pub fn eval_with_bound(c: &[f64], x: f64) -> (f64, f64) {
    let v = eval(c, x);
    let ax = x.abs();
    let mag = c.iter().rev().fold(0.0, |acc, &a| acc * ax + a.abs());
    (v, mag * 4.0 * c.len() as f64 * f64::EPSILON)
}

pub fn derivative(c: &[f64]) -> Vec<f64> {
    c.iter().enumerate().skip(1).map(|(i, &a)| a * i as f64).collect()
}

fn trim(c: &[f64]) -> &[f64] {
    let scale = c.iter().fold(0.0f64, |m, a| m.max(a.abs()));
    let mut n = c.len();
    while n > 0 && c[n - 1].abs() <= scale * 1e-14 {
        n -= 1;
    }
    &c[..n]
}

/// Numerically stable roots of `a x^2 + b x + c`, ascending. A double root is
/// returned once, flagged touching.
pub fn quadratic(a: f64, b: f64, c: f64) -> Vec<Root> {
    let mut out = Vec::with_capacity(2);
    quadratic_into(a, b, c, &mut |r| out.push(r));
    out
}

fn quadratic_into(a: f64, b: f64, c: f64, out: &mut impl FnMut(Root)) {
    let scale = a.abs().max(b.abs()).max(c.abs());
    if scale == 0.0 {
        return;
    }
    if a.abs() <= scale * 1e-14 {
        if b.abs() > scale * 1e-14 {
            out(Root {
                x: -c / b,
                touching: false,
            });
        }
        return;
    }
    let disc = b * b - 4.0 * a * c;
    let disc_err = 8.0 * f64::EPSILON * (b * b).max((4.0 * a * c).abs());
    if disc < -disc_err {
        return;
    }
    if disc <= disc_err {
        out(Root {
            x: -b / (2.0 * a),
            touching: true,
        });
        return;
    }
    let s = disc.sqrt();
    let q = -0.5 * (b + if b >= 0.0 { s } else { -s });
    let (x1, x2) = (q / a, c / q);
    let (lo, hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    out(Root {
        x: lo,
        touching: false,
    });
    out(Root {
        x: hi,
        touching: false,
    });
}

/// All real roots of `c` in `[lo, hi]`, ascending. Works for any degree: the
/// roots of the derivative cut the range into pieces where the polynomial is
/// monotonic, and each piece holds at most one root, found by bisection
/// polished with Newton. Roots where the value only touches zero (within its
/// rounding error) come from the derivative's roots and are flagged.
pub fn real_roots(c: &[f64], lo: f64, hi: f64) -> Vec<Root> {
    let mut out = Vec::with_capacity(c.len());
    roots_into(c, lo, hi, &mut out);
    out
}

/// `real_roots` appended to `out` (which also holds the derivative's roots for a while):
/// small degrees allocate nothing more.
fn roots_into(c: &[f64], lo: f64, hi: f64, out: &mut Vec<Root>) {
    let c = trim(c);
    match c.len() {
        0 | 1 => return,
        2 => {
            let x = -c[0] / c[1];
            if lo <= x && x <= hi {
                out.push(Root { x, touching: false });
            }
            return;
        }
        3 => {
            quadratic_into(c[2], c[1], c[0], &mut |r| {
                if lo <= r.x && r.x <= hi {
                    out.push(r)
                }
            });
            return;
        }
        _ => {}
    }
    const SMALL: usize = 16;
    let n = c.len() - 1;
    let (mut ds, dh);
    let d: &[f64] = if n <= SMALL {
        ds = [0.0; SMALL];
        for (i, &a) in c.iter().enumerate().skip(1) {
            ds[i - 1] = a * i as f64;
        }
        &ds[..n]
    } else {
        dh = derivative(c);
        &dh
    };
    // the derivative's roots, borrowed from the end of `out` as cuts
    let mark = out.len();
    roots_into(d, lo, hi, out);
    let (mut cs, ch);
    let cuts: &[f64] = if out.len() - mark + 2 <= SMALL + 2 {
        cs = [0.0; SMALL + 2];
        cs[0] = lo;
        for (k, r) in out[mark..].iter().enumerate() {
            cs[k + 1] = r.x;
        }
        let m = out.len() - mark + 2;
        cs[m - 1] = hi;
        &cs[..m]
    } else {
        let mut v = Vec::with_capacity(out.len() - mark + 2);
        v.push(lo);
        v.extend(out[mark..].iter().map(|r| r.x));
        v.push(hi);
        ch = v;
        &ch
    };
    out.truncate(mark);
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (fa, ea) = eval_with_bound(c, a);
        let (fb, eb) = eval_with_bound(c, b);
        let za = fa.abs() <= ea;
        let zb = fb.abs() <= eb;
        if za {
            push_root(out, mark, c, d, a);
        }
        if !za && !zb && (fa < 0.0) != (fb < 0.0) {
            push_root(out, mark, c, d, bisect(c, d, a, b, fa));
        }
        if zb {
            push_root(out, mark, c, d, b);
        }
    }
}

fn push_root(out: &mut Vec<Root>, mark: usize, c: &[f64], d: &[f64], x: f64) {
    if out.len() > mark
        && let Some(last) = out.last()
        && (x - last.x).abs() <= 1e-12 * (1.0 + x.abs())
    {
        return;
    }
    let (dv, de) = eval_with_bound(d, x);
    let scale = 1e-7 * c.iter().fold(0.0f64, |m, a| m.max(a.abs())).max(1e-300);
    out.push(Root {
        x,
        touching: dv.abs() <= de.max(scale * (1.0 + x.abs())),
    });
}

fn bisect(c: &[f64], d: &[f64], mut a: f64, mut b: f64, fa: f64) -> f64 {
    let neg_at_a = fa < 0.0;
    let mut x = 0.5 * (a + b);
    for _ in 0..200 {
        let f = eval(c, x);
        if f == 0.0 {
            return x;
        }
        if (f < 0.0) == neg_at_a {
            a = x;
        } else {
            b = x;
        }
        let df = eval(d, x);
        let newton = x - f / df;
        let next = if df != 0.0 && newton > a && newton < b {
            newton
        } else {
            0.5 * (a + b)
        };
        if next == x || b - a <= f64::EPSILON * (a.abs().max(b.abs())) {
            return next;
        }
        x = next;
    }
    x
}

/// A bound on the magnitude of every real root (Cauchy).
pub fn root_bound(c: &[f64]) -> f64 {
    let c = trim(c);
    let Some(&lead) = c.last() else { return 0.0 };
    1.0 + c[..c.len() - 1]
        .iter()
        .fold(0.0f64, |m, a| m.max((a / lead).abs()))
}

pub fn all_real_roots(c: &[f64]) -> Vec<Root> {
    let b = root_bound(c);
    real_roots(c, -b, b)
}

/// The n-point Gauss-Legendre rule on [-1, 1]: nodes (increasing) and weights, each node by
/// Newton on the Legendre polynomial P_n from the asymptotic guess cos(pi (i - 1/4) / (n + 1/2)).
pub fn gauss_legendre(n: usize) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = (1..=n)
        .map(|i| {
            let mut x = crate::dmath::cos(crate::dmath::PI * (i as f64 - 0.25) / (n as f64 + 0.5));
            let mut dp = 1.0;
            for _ in 0..100 {
                // P_n(x) and P_n'(x) by the three-term recurrence
                let (mut p0, mut p1) = (1.0, x);
                for k in 2..=n {
                    (p0, p1) = (p1, ((2 * k - 1) as f64 * x * p1 - (k - 1) as f64 * p0) / k as f64);
                }
                let p = if n == 0 { 1.0 } else { p1 };
                dp = n as f64 * (x * p - p0) / (x * x - 1.0);
                let dx = p / dp;
                x -= dx;
                if dx.abs() <= 1e-16 {
                    break;
                }
            }
            (-x, 2.0 / ((1.0 - x * x) * dp * dp))
        })
        .collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gauss_legendre_integrates_to_its_degree() {
        for n in [1, 3, 8, 16, 32] {
            let r = gauss_legendre(n);
            for d in 0..2 * n {
                let got: f64 = r.iter().map(|&(x, w)| w * x.powi(d as i32)).sum();
                let want = if d % 2 == 0 { 2.0 / (d + 1) as f64 } else { 0.0 };
                assert!((got - want).abs() < 1e-14, "{n} {d} {got}");
            }
        }
    }
    use crate::rng::Rng;

    fn from_roots(rs: &[f64]) -> Vec<f64> {
        let mut c = vec![1.0];
        for &r in rs {
            let mut n = vec![0.0; c.len() + 1];
            for (i, &a) in c.iter().enumerate() {
                n[i] -= a * r;
                n[i + 1] += a;
            }
            c = n;
        }
        c
    }

    #[test]
    fn quadratic_cases() {
        let r = quadratic(1.0, -3.0, 2.0);
        assert_eq!(r.len(), 2);
        assert!((r[0].x - 1.0).abs() < 1e-15 && (r[1].x - 2.0).abs() < 1e-15);
        let r = quadratic(1.0, -2.0, 1.0);
        assert_eq!(r.len(), 1);
        assert!(r[0].touching);
        assert!(quadratic(1.0, 0.0, 1.0).is_empty());
        let r = quadratic(1.0, -1e8, 1.0);
        assert!((r[0].x - 1e-8).abs() < 1e-22);
    }

    #[test]
    fn random_distinct_roots() {
        let mut rng = Rng::new(30);
        for _ in 0..5_000 {
            let n = 3 + rng.below(4);
            let mut rs: Vec<f64> = (0..n).map(|_| rng.range(-10.0, 10.0)).collect();
            rs.sort_by(f64::total_cmp);
            if rs.windows(2).any(|w| w[1] - w[0] < 0.05) {
                continue;
            }
            let found = all_real_roots(&from_roots(&rs));
            assert_eq!(found.len(), rs.len(), "{rs:?} -> {found:?}");
            for (f, r) in found.iter().zip(&rs) {
                assert!((f.x - r).abs() < 1e-8, "{rs:?} -> {found:?}");
                assert!(!f.touching);
            }
        }
    }

    #[test]
    fn double_roots_are_touching() {
        let found = all_real_roots(&from_roots(&[-2.0, 1.0, 1.0, 3.0]));
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(found[1].touching && (found[1].x - 1.0).abs() < 1e-6);
        assert!(!found[0].touching && !found[2].touching);
        // (x^2 + 1)(x - 1)^2: one touching root only
        let c = [1.0, -2.0, 2.0, -2.0, 1.0];
        let found = all_real_roots(&c);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].touching);
    }
}

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
    let scale = a.abs().max(b.abs()).max(c.abs());
    if scale == 0.0 {
        return vec![];
    }
    if a.abs() <= scale * 1e-14 {
        if b.abs() <= scale * 1e-14 {
            return vec![];
        }
        return vec![Root {
            x: -c / b,
            touching: false,
        }];
    }
    let disc = b * b - 4.0 * a * c;
    let disc_err = 8.0 * f64::EPSILON * (b * b).max((4.0 * a * c).abs());
    if disc < -disc_err {
        return vec![];
    }
    if disc <= disc_err {
        return vec![Root {
            x: -b / (2.0 * a),
            touching: true,
        }];
    }
    let s = disc.sqrt();
    let q = -0.5 * (b + if b >= 0.0 { s } else { -s });
    let (x1, x2) = (q / a, c / q);
    let (lo, hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    vec![
        Root {
            x: lo,
            touching: false,
        },
        Root {
            x: hi,
            touching: false,
        },
    ]
}

/// All real roots of `c` in `[lo, hi]`, ascending. Works for any degree: the
/// roots of the derivative cut the range into pieces where the polynomial is
/// monotonic, and each piece holds at most one root, found by bisection
/// polished with Newton. Roots where the value only touches zero (within its
/// rounding error) come from the derivative's roots and are flagged.
pub fn real_roots(c: &[f64], lo: f64, hi: f64) -> Vec<Root> {
    let c = trim(c);
    match c.len() {
        0 | 1 => return vec![],
        2 => {
            let x = -c[0] / c[1];
            return if lo <= x && x <= hi {
                vec![Root { x, touching: false }]
            } else {
                vec![]
            };
        }
        3 => {
            return quadratic(c[2], c[1], c[0])
                .into_iter()
                .filter(|r| lo <= r.x && r.x <= hi)
                .collect();
        }
        _ => {}
    }
    let d = derivative(c);
    let crit = real_roots(&d, lo, hi);
    let mut cuts = Vec::with_capacity(crit.len() + 2);
    cuts.push(lo);
    cuts.extend(crit.iter().map(|r| r.x));
    cuts.push(hi);

    let mut out: Vec<Root> = Vec::new();
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (fa, ea) = eval_with_bound(c, a);
        let (fb, eb) = eval_with_bound(c, b);
        let za = fa.abs() <= ea;
        let zb = fb.abs() <= eb;
        if za {
            push_root(&mut out, c, a);
        }
        if !za && !zb && (fa < 0.0) != (fb < 0.0) {
            push_root(&mut out, c, bisect(c, a, b, fa));
        }
        if zb {
            push_root(&mut out, c, b);
        }
    }
    out
}

fn push_root(out: &mut Vec<Root>, c: &[f64], x: f64) {
    if let Some(last) = out.last()
        && (x - last.x).abs() <= 1e-12 * (1.0 + x.abs())
    {
        return;
    }
    let d = derivative(c);
    let (dv, de) = eval_with_bound(&d, x);
    let scale = 1e-7 * c.iter().fold(0.0f64, |m, a| m.max(a.abs())).max(1e-300);
    out.push(Root {
        x,
        touching: dv.abs() <= de.max(scale * (1.0 + x.abs())),
    });
}

fn bisect(c: &[f64], mut a: f64, mut b: f64, fa: f64) -> f64 {
    let neg_at_a = fa < 0.0;
    let d = derivative(c);
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
        let df = eval(&d, x);
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

#[cfg(test)]
mod tests {
    use super::*;
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

//! Small dense linear algebra for Newton steps.

/// Solves the 2x2 system `[a b; c d] x = r`; None when singular relative to its scale.
pub fn solve2(a: f64, b: f64, c: f64, d: f64, r0: f64, r1: f64) -> Option<(f64, f64)> {
    let det = a * d - b * c;
    let scale = (a.abs() + b.abs()) * (c.abs() + d.abs());
    if scale == 0.0 || det.abs() <= scale * 1e-14 {
        return None;
    }
    Some(((r0 * d - b * r1) / det, (a * r1 - c * r0) / det))
}

/// Least-squares / square solve of a dense row-major `m x n` system by
/// Householder QR with column pivoting. Returns None if rank deficient beyond `n`.
/// Rows beyond the rank are satisfied in the least-squares sense; for an
/// under-determined system the minimum-change solution along the pivoted
/// columns is returned (free columns set to zero).
pub fn solve_least_squares(a: &[f64], m: usize, n: usize, b: &[f64]) -> Option<Vec<f64>> {
    assert_eq!(a.len(), m * n);
    assert_eq!(b.len(), m);
    let mut a = a.to_vec();
    let mut b = b.to_vec();
    let mut perm: Vec<usize> = (0..n).collect();
    let mut norms: Vec<f64> = (0..n).map(|j| col_norm_sq(&a, m, n, j, 0)).collect();
    let max0 = norms.iter().cloned().fold(0.0, f64::max).sqrt();
    let tol = max0 * 1e-12 * (m.max(n) as f64);
    let steps = m.min(n);
    let mut rank = 0;
    for k in 0..steps {
        let (mut best, mut bj) = (-1.0, k);
        for (j, &nj) in norms.iter().enumerate().skip(k) {
            if nj > best {
                best = nj;
                bj = j;
            }
        }
        if bj != k {
            for i in 0..m {
                a.swap(i * n + k, i * n + bj);
            }
            perm.swap(k, bj);
            norms.swap(k, bj);
        }
        let alpha_sq = col_norm_sq(&a, m, n, k, k);
        let alpha = alpha_sq.sqrt();
        if alpha <= tol {
            break;
        }
        rank = k + 1;
        let akk = a[k * n + k];
        let s = if akk >= 0.0 { -alpha } else { alpha };
        let mut v: Vec<f64> = (k..m).map(|i| a[i * n + k]).collect();
        v[0] -= s;
        let vnorm_sq: f64 = v.iter().map(|x| x * x).sum();
        if vnorm_sq > 0.0 {
            for j in k..n {
                let dot: f64 = (k..m).map(|i| v[i - k] * a[i * n + j]).sum();
                let f = 2.0 * dot / vnorm_sq;
                for i in k..m {
                    a[i * n + j] -= f * v[i - k];
                }
            }
            let dot: f64 = (k..m).map(|i| v[i - k] * b[i]).sum();
            let f = 2.0 * dot / vnorm_sq;
            for i in k..m {
                b[i] -= f * v[i - k];
            }
        }
        for (j, nj) in norms.iter_mut().enumerate().skip(k + 1) {
            *nj = col_norm_sq(&a, m, n, j, k + 1);
        }
    }
    let mut y = vec![0.0; n];
    for k in (0..rank).rev() {
        let mut s = b[k];
        for j in k + 1..rank {
            s -= a[k * n + j] * y[j];
        }
        y[k] = s / a[k * n + k];
    }
    let mut x = vec![0.0; n];
    for k in 0..n {
        x[perm[k]] = y[k];
    }
    Some(x)
}

fn col_norm_sq(a: &[f64], m: usize, n: usize, j: usize, from: usize) -> f64 {
    (from..m).map(|i| a[i * n + j] * a[i * n + j]).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    #[test]
    fn square_systems() {
        let mut r = Rng::new(40);
        for _ in 0..2_000 {
            let n = 1 + r.below(8);
            let a: Vec<f64> = (0..n * n).map(|_| r.range(-1.0, 1.0)).collect();
            let x: Vec<f64> = (0..n).map(|_| r.range(-5.0, 5.0)).collect();
            let b: Vec<f64> = (0..n)
                .map(|i| (0..n).map(|j| a[i * n + j] * x[j]).sum())
                .collect();
            let got = solve_least_squares(&a, n, n, &b).unwrap();
            let res: f64 = (0..n)
                .map(|i| ((0..n).map(|j| a[i * n + j] * got[j]).sum::<f64>() - b[i]).abs())
                .fold(0.0, f64::max);
            assert!(res < 1e-8, "residual {res}");
        }
        assert_eq!(solve2(1.0, 2.0, 3.0, 4.0, 5.0, 6.0), Some((-4.0, 4.5)));
        assert_eq!(solve2(1.0, 2.0, 2.0, 4.0, 1.0, 1.0), None);
    }

    #[test]
    fn underdetermined_is_consistent() {
        // one equation, two unknowns: x + y = 2
        let x = solve_least_squares(&[1.0, 1.0], 1, 2, &[2.0]).unwrap();
        assert!((x[0] + x[1] - 2.0).abs() < 1e-14);
    }
}

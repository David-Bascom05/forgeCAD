//! The one rank decision of a loop block (audit 2 §3.1): every rank question (the Newton step's,
//! the reported rank and mobility, the redundant rows and what implies them, each joint's free
//! motions) is answered from a singular value decomposition, by one rule, in the block's own
//! frame; and capped by the block's generic rank, which the constraint graph gives exactly.
//!
//! The block's Jacobian is taken in its own frame (lengths about the block's centroid, angles
//! times its extent: `solve::Block`), so a rigid motion of the whole assembly moves its rows and
//! columns by rotations only, and its singular values not at all. The rule is then about singular
//! values alone:
//! - a singular value at or under the floor `noise` (rounding: `ε (m + n)` of the largest, and the
//!   rows' own residual, which J was evaluated off the exact solution by) is zero;
//! - one at or over `GAP` times the floor is not;
//! - with any between, the rank is not decided by the numbers (a configuration near a singular
//!   one): `Verdict::clear` is false, and the rank given is the count over the floor.
//!
//! Each singular value of the computed J is within the error's norm of the exact one (Weyl), so a
//! clear verdict is the rank of every matrix within the floor of the one computed.
//!
//! The decomposition is one-sided Jacobi (Hestenes): plane rotations of column pairs in a fixed
//! cyclic order, IEEE arithmetic and square roots only, so native and wasm agree bit for bit.

/// How far over the floor a singular value must be to count, so a rank is clear.
pub(crate) const GAP: f64 = 1e3;

/// `A = U Σ Vᵀ` for a row-major `m x n` matrix: singular values largest first, ties by column.
pub(crate) struct Svd {
    pub m: usize,
    pub n: usize,
    /// The `n` singular values, largest first.
    pub sigma: Vec<f64>,
    /// Row-major `n x n`: column `j` is the right singular vector of `sigma[j]`.
    pub v: Vec<f64>,
    /// Row-major `m x n`: column `j` is `A v_j` (that is `σ_j u_j`).
    pub av: Vec<f64>,
}

impl Svd {
    pub fn new(a: &[f64], m: usize, n: usize) -> Svd {
        let mut b = a.to_vec();
        let mut v = vec![0.0; n * n];
        for i in 0..n {
            v[i * n + i] = 1.0;
        }
        let col = |b: &[f64], p: usize, q: usize| {
            let (mut pp, mut qq, mut pq) = (0.0, 0.0, 0.0);
            for i in 0..m {
                let (x, y) = (b[i * n + p], b[i * n + q]);
                pp += x * x;
                qq += y * y;
                pq += x * y;
            }
            (pp, qq, pq)
        };
        // sweeps until no pair is further from orthogonal than rounding (quadratic convergence:
        // a handful; the cap only bounds a pathological input)
        for _ in 0..64 {
            let mut turned = false;
            for p in 0..n {
                for q in p + 1..n {
                    let (alpha, beta, gamma) = col(&b, p, q);
                    if gamma == 0.0 || gamma.abs() <= f64::EPSILON * (alpha * beta).sqrt() {
                        continue;
                    }
                    turned = true;
                    let zeta = (beta - alpha) / (2.0 * gamma);
                    let t = zeta.signum() / (zeta.abs() + (1.0 + zeta * zeta).sqrt());
                    let c = 1.0 / (1.0 + t * t).sqrt();
                    let s = c * t;
                    for i in 0..m {
                        let (x, y) = (b[i * n + p], b[i * n + q]);
                        b[i * n + p] = c * x - s * y;
                        b[i * n + q] = s * x + c * y;
                    }
                    for i in 0..n {
                        let (x, y) = (v[i * n + p], v[i * n + q]);
                        v[i * n + p] = c * x - s * y;
                        v[i * n + q] = s * x + c * y;
                    }
                }
            }
            if !turned {
                break;
            }
        }
        let norms: Vec<f64> = (0..n)
            .map(|j| (0..m).map(|i| b[i * n + j] * b[i * n + j]).sum::<f64>().sqrt())
            .collect();
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&x, &y| norms[y].total_cmp(&norms[x]).then(x.cmp(&y)));
        let pick = |src: &[f64], rows: usize| {
            let mut out = vec![0.0; rows * n];
            for i in 0..rows {
                for (k, &j) in order.iter().enumerate() {
                    out[i * n + k] = src[i * n + j];
                }
            }
            out
        };
        Svd {
            m,
            n,
            sigma: order.iter().map(|&j| norms[j]).collect(),
            v: pick(&v, n),
            av: pick(&b, m),
        }
    }

    /// The rank by the one rule, `residual` being how far (in the rows' own units) the rows were
    /// from being met where J was evaluated.
    pub fn verdict(&self, residual: f64, generic: usize) -> Verdict {
        decide(&self.sigma, self.m, self.n, residual, generic)
    }

    /// The minimum-norm `δ` with `J δ = b` over the rank `r` part: `Σᵢ<r vᵢ (uᵢ · b) / σᵢ`.
    pub fn min_norm(&self, b: &[f64], r: usize) -> Vec<f64> {
        let (m, n) = (self.m, self.n);
        let mut d = vec![0.0; n];
        for j in 0..r {
            let s2 = self.sigma[j] * self.sigma[j];
            let ub: f64 = (0..m).map(|i| self.av[i * n + j] * b[i]).sum::<f64>() / s2;
            for (t, x) in d.iter_mut().enumerate() {
                *x += self.v[t * n + j] * ub;
            }
        }
        d
    }

    /// An orthonormal basis of the null space of J past rank `r`: the right singular vectors
    /// `v_r .. v_n`.
    pub fn null_space(&self, r: usize) -> Vec<Vec<f64>> {
        let n = self.n;
        (r..n)
            .map(|j| (0..n).map(|t| self.v[t * n + j]).collect())
            .collect()
    }

    /// Row `i` of J in the coordinates of its row space of rank `r`: `(J V)ᵢ` over the first `r`.
    pub fn row_coords(&self, i: usize, r: usize) -> Vec<f64> {
        (0..r).map(|j| self.av[i * self.n + j]).collect()
    }
}

/// A rank, and whether the numbers decide it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Verdict {
    pub rank: usize,
    /// No singular value lies between the floor and `GAP` times it.
    pub clear: bool,
    /// The floor a singular value is zero under.
    pub floor_bits: u64,
}

impl Verdict {
    pub fn floor(&self) -> f64 {
        f64::from_bits(self.floor_bits)
    }
}

/// The floor under which a singular value of an `m x n` matrix of largest singular value `top`,
/// evaluated `residual` off its exact point, is zero: rounding in forming and factoring it, and the
/// residual. The rows are in the block's frame, a unit motion moving them by about one, so the floor
/// is absolute: noise in a matrix of noise is noise (`rank([1e-17, −3e-17])` is 0).
pub(crate) fn floor(top: f64, m: usize, n: usize, residual: f64) -> f64 {
    64.0 * f64::EPSILON * (m + n) as f64 * top.max(1.0) + residual.abs()
}

/// The one rule (the module's doc), on singular values largest first, capped at `generic`.
pub(crate) fn decide(sigma: &[f64], m: usize, n: usize, residual: f64, generic: usize) -> Verdict {
    let top = sigma.first().copied().unwrap_or(0.0);
    let eta = floor(top, m, n, residual);
    let rank = sigma.iter().take_while(|&&s| s > eta).count().min(generic);
    let clear = sigma.iter().all(|&s| s <= eta || s >= GAP * eta);
    Verdict {
        rank,
        clear,
        floor_bits: eta.to_bits(),
    }
}

/// The rank of a row-major `m x n` matrix by the one rule, with no cap.
pub(crate) fn rank(a: &[f64], m: usize, n: usize, residual: f64) -> usize {
    Svd::new(a, m, n).verdict(residual, usize::MAX).rank
}

/// The generic rank of rows over columns, exactly: the largest matching of rows to the columns
/// each may depend on (`cols[i]`), by augmenting paths in index order. No configuration can give
/// J a larger rank, whatever its numbers (Dulmage–Mendelsohn's structural rank).
pub(crate) fn generic_rank(cols: &[Vec<usize>], n: usize) -> usize {
    let mut owner: Vec<usize> = vec![usize::MAX; n];
    fn augment(i: usize, cols: &[Vec<usize>], owner: &mut [usize], seen: &mut [bool]) -> bool {
        for &c in &cols[i] {
            if seen[c] {
                continue;
            }
            seen[c] = true;
            if owner[c] == usize::MAX || augment(owner[c], cols, owner, seen) {
                owner[c] = i;
                return true;
            }
        }
        false
    }
    let mut seen = vec![false; n];
    let mut matched = 0;
    for i in 0..cols.len() {
        seen.iter_mut().for_each(|s| *s = false);
        if augment(i, cols, &mut owner, &mut seen) {
            matched += 1;
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::rng::Rng;

    fn mul(a: &[f64], m: usize, n: usize, x: &[f64]) -> Vec<f64> {
        (0..m)
            .map(|i| (0..n).map(|j| a[i * n + j] * x[j]).sum())
            .collect()
    }

    /// The audit's repro (qr_noise_rank.rs): rounding noise was given full rank by a threshold
    /// relative to the matrix's own largest column.
    #[test]
    fn noise_has_no_rank() {
        assert_eq!(rank(&[1e-17, -3e-17], 1, 2, 0.0), 0);
        assert_eq!(rank(&[1e-17, -3e-17], 2, 1, 0.0), 0);
        assert_eq!(rank(&[1.0, 2.0, 2.0, 4.0], 2, 2, 0.0), 1);
        assert_eq!(rank(&[1.0, 0.0, 0.0, 1.0], 2, 2, 0.0), 2);
        assert_eq!(rank(&[0.0; 6], 2, 3, 0.0), 0);
        // a value a millionth of the rows' scale is a value; one the size of the residual is not
        assert_eq!(rank(&[1.0, 0.0, 0.0, 1e-6], 2, 2, 1e-12), 2);
        assert_eq!(rank(&[1.0, 0.0, 0.0, 1e-12], 2, 2, 1e-12), 1);
        // and between the floor and GAP times it, no clear rank
        let s = Svd::new(&[1.0, 0.0, 0.0, 1e-11], 2, 2);
        assert!(!s.verdict(1e-13, usize::MAX).clear);
    }

    #[test]
    fn decomposes_solves_and_spans_the_null_space() {
        let mut r = Rng::new(7);
        for _ in 0..300 {
            let m = 1 + r.below(7);
            let n = 1 + r.below(9);
            let mut a: Vec<f64> = (0..m * n).map(|_| r.range(-1.0, 1.0)).collect();
            if m > 1 && r.bool() {
                for c in 0..n {
                    a[(m - 1) * n + c] = 2.0 * a[c];
                }
            }
            let s = Svd::new(&a, m, n);
            assert!(s.sigma.windows(2).all(|w| w[0] >= w[1]));
            let v = s.verdict(0.0, usize::MAX);
            assert!(v.clear, "{:?}", s.sigma);
            // V orthonormal, A V = the stored columns
            for j in 0..n {
                let vj: Vec<f64> = (0..n).map(|t| s.v[t * n + j]).collect();
                let avj = mul(&a, m, n, &vj);
                for (i, x) in avj.iter().enumerate() {
                    assert!((x - s.av[i * n + j]).abs() < 1e-12);
                }
                for k in 0..n {
                    let d: f64 = (0..n).map(|t| s.v[t * n + j] * s.v[t * n + k]).sum();
                    assert!((d - if j == k { 1.0 } else { 0.0 }).abs() < 1e-12);
                }
            }
            for z in s.null_space(v.rank) {
                assert!(mul(&a, m, n, &z).iter().all(|x| x.abs() < 1e-12));
            }
            // a consistent right side is met by the minimum-norm step
            let x: Vec<f64> = (0..n).map(|_| r.range(-1.0, 1.0)).collect();
            let b = mul(&a, m, n, &x);
            let d = s.min_norm(&b, v.rank);
            let got = mul(&a, m, n, &d);
            assert!(got.iter().zip(&b).all(|(g, w)| (g - w).abs() < 1e-9));
            assert!(d.iter().map(|t| t * t).sum::<f64>() <= x.iter().map(|t| t * t).sum::<f64>() + 1e-9);
            assert_eq!(v.rank, s.null_space(v.rank).len().abs_diff(n));
        }
    }

    #[test]
    fn generic_rank_is_a_matching() {
        // three rows on two columns, two rows on one column, an empty row
        assert_eq!(generic_rank(&[vec![0, 1], vec![0, 1], vec![0, 1]], 2), 2);
        assert_eq!(generic_rank(&[vec![0], vec![0], vec![1, 2]], 3), 2);
        assert_eq!(generic_rank(&[vec![], vec![2]], 3), 1);
    }
}

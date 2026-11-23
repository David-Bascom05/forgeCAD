#![allow(dead_code)]
struct Qr {
    /// rows x cols, row-major; R above the diagonal after factoring, reflectors below.
    a: Vec<f64>,
    m: usize,
    n: usize,
    /// Each reflector's leading entry (the rest are stored below R's diagonal).
    head: Vec<f64>,
    /// Column k of the factored matrix is original column `perm[k]`.
    pub perm: Vec<usize>,
    pub rank: usize,
}

impl Qr {
    /// Factors the row-major `m x n` matrix `a`. Columns whose remaining norm falls to `rel` of
    /// the largest column norm count as dependent.
    pub fn new(a: Vec<f64>, m: usize, n: usize, rel: f64) -> Qr {
        let mut q = Qr {
            a,
            m,
            n,
            head: Vec::new(),
            perm: (0..n).collect(),
            rank: 0,
        };
        let col =
            |a: &[f64], j: usize, from: usize| (from..m).map(|i| a[i * n + j] * a[i * n + j]).sum::<f64>();
        let mut norms: Vec<f64> = (0..n).map(|j| col(&q.a, j, 0)).collect();
        let max0 = norms.iter().cloned().fold(0.0, f64::max).sqrt();
        let tol = max0 * rel;
        for k in 0..m.min(n) {
            let (mut best, mut bj) = (-1.0, k);
            for (j, &nj) in norms.iter().enumerate().skip(k) {
                if nj > best {
                    best = nj;
                    bj = j;
                }
            }
            if bj != k {
                for i in 0..m {
                    q.a.swap(i * n + k, i * n + bj);
                }
                q.perm.swap(k, bj);
                norms.swap(k, bj);
            }
            let alpha = col(&q.a, k, k).sqrt();
            if alpha <= tol || alpha == 0.0 {
                break;
            }
            q.rank = k + 1;
            let akk = q.a[k * n + k];
            let s = if akk >= 0.0 { -alpha } else { alpha };
            // v = x - s e1, stored with v[0] in head and the rest in place
            let v0 = akk - s;
            let vn2 = v0 * v0 + (k + 1..m).map(|i| q.a[i * n + k] * q.a[i * n + k]).sum::<f64>();
            q.head.push(v0);
            q.a[k * n + k] = s;
            if vn2 > 0.0 {
                for j in k + 1..n {
                    let mut dot = v0 * q.a[k * n + j];
                    for i in k + 1..m {
                        dot += q.a[i * n + k] * q.a[i * n + j];
                    }
                    let f = 2.0 * dot / vn2;
                    q.a[k * n + j] -= f * v0;
                    for i in k + 1..m {
                        q.a[i * n + j] -= f * q.a[i * n + k];
                    }
                }
            }
            for (j, nj) in norms.iter_mut().enumerate().skip(k + 1) {
                *nj = col(&q.a, j, k + 1);
            }
        }
        q
    }

    /// Applies reflector k to `x` (length m).
    fn reflect(&self, k: usize, x: &mut [f64]) {
        let n = self.n;
        let v0 = self.head[k];
        let vn2 = v0 * v0
            + (k + 1..self.m)
                .map(|i| self.a[i * n + k] * self.a[i * n + k])
                .sum::<f64>();
        if vn2 == 0.0 {
            return;
        }
        let mut dot = v0 * x[k];
        for (i, xi) in x.iter().enumerate().skip(k + 1) {
            dot += self.a[i * n + k] * xi;
        }
        let f = 2.0 * dot / vn2;
        x[k] -= f * v0;
        for (i, xi) in x.iter_mut().enumerate().skip(k + 1) {
            *xi -= f * self.a[i * n + k];
        }
    }

    /// `Q y` for y of length m.
    pub fn q_times(&self, mut y: Vec<f64>) -> Vec<f64> {
        for k in (0..self.rank).rev() {
            self.reflect(k, &mut y);
        }
        y
    }

    /// For the factored matrix `Jᵀ` (n unknowns as rows, m equations as columns): the
    /// minimum-norm `δ` with `J δ = b` over the independent equations (`perm[..rank]`).
    pub fn min_norm(&self, b: &[f64]) -> Vec<f64> {
        let n = self.n;
        // Rᵀ y = Pᵀ b on the first rank rows; R is upper triangular in the factored matrix
        let mut y = vec![0.0; self.m];
        for k in 0..self.rank {
            let mut s = b[self.perm[k]];
            for (i, yi) in y.iter().enumerate().take(k) {
                s -= self.a[i * n + k] * yi;
            }
            y[k] = s / self.a[k * n + k];
        }
        self.q_times(y)
    }

    /// An orthonormal basis of the null space of `J` (the factored matrix being `Jᵀ`): the last
    /// `m − rank` columns of Q.
    pub fn null_space(&self) -> Vec<Vec<f64>> {
        (self.rank..self.m)
            .map(|k| {
                let mut e = vec![0.0; self.m];
                e[k] = 1.0;
                self.q_times(e)
            })
            .collect()
    }
}

/// Numerical rank of a row-major `m x n` matrix.
fn rank(a: &[f64], m: usize, n: usize) -> usize {
    Qr::new(a.to_vec(), m, n, 1e-9).rank
}

fn main() {
    // a joint's 1 gradient row projected on a 2-dim null space: pure rounding noise
    println!("noise 1x2 rank = {}", rank(&[1e-17, -3e-17], 1, 2));
    println!("noise 3x2 rank = {}", rank(&[1e-17, -3e-17, 2e-17, 0.0, 0.0, 5e-18], 3, 2));
    println!("exact zero rank = {}", rank(&[0.0, 0.0], 1, 2));
    // a near-toggle: row 2 = row 1 + 1e-10 perturbation
    println!("near-dep rank 1e-9 = {}", rank(&[1.0, 0.0, 1.0, 1e-10], 2, 2));
    println!("near-dep rank 1e-8 = {}", rank(&[1.0, 0.0, 1.0, 5e-9], 2, 2));
}

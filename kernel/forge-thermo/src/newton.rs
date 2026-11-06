//! The membrane's Newton step. Each triangle's Hessian in its principal frame splits into a
//! stretch block, a shear block and the pair out of its plane, each clamped to be no less than
//! zero (Teran, Sifakis, Irving, Fedkiw 2005; Smith, de Goes, Kim 2019), so the sum is positive
//! semi-definite; it is summed on 3x3 blocks over the nodes with the pressure's, a hinge's and the
//! plug's spring's, and solved by a sparse Cholesky on the moving coordinates (multifrontal, by supernodes; Liu 1992), in
//! the order of a nested dissection of the sheet's layout. Derived and checked:
//! research/thermoform/newton/.
#![allow(clippy::needless_range_loop)]
use crate::stretch::Law;
use forge_math::{Vec3, dmath};

/// A symmetric matrix over the nodes: a 3x3 block (row-major) for each ordered pair of nodes that
/// share a triangle or a hinge.
pub(crate) struct Blocks {
    start: Vec<usize>,
    col: Vec<u32>,
    pub(crate) val: Vec<[f64; 9]>,
}

impl Blocks {
    pub(crate) fn new(nodes: usize, groups: &[&[usize]]) -> Blocks {
        let mut adj: Vec<Vec<u32>> = (0..nodes).map(|k| vec![k as u32]).collect();
        for g in groups {
            for &u in *g {
                adj[u].extend(g.iter().map(|&v| v as u32));
            }
        }
        let mut start = vec![0];
        let mut col = vec![];
        for mut a in adj {
            a.sort_unstable();
            a.dedup();
            col.extend(a);
            start.push(col.len());
        }
        let val = vec![[0.0; 9]; col.len()];
        Blocks { start, col, val }
    }

    /// The block of nodes (u, v).
    pub(crate) fn at(&self, u: usize, v: usize) -> usize {
        let r = self.start[u]..self.start[u + 1];
        r.start
            + self.col[r]
                .binary_search(&(v as u32))
                .expect("nodes sharing a triangle")
    }

    fn row(&self, u: usize) -> impl Iterator<Item = (usize, usize)> + '_ {
        (self.start[u]..self.start[u + 1]).map(|k| (k, self.col[k] as usize))
    }
}

/// The nodes in the order of a nested dissection of their places on the reference plane: each
/// half by the wider side's middle, the half's nodes next to the other half last.
pub(crate) fn dissect(at: &[[f64; 2]], adj: &Blocks) -> Vec<u32> {
    let mut out = Vec::with_capacity(at.len());
    let mut mark = vec![0u32; at.len()];
    let mut stamp = 0;
    split(
        (0..at.len() as u32).collect(),
        at,
        adj,
        &mut mark,
        &mut stamp,
        &mut out,
    );
    out
}

fn split(
    nodes: Vec<u32>,
    at: &[[f64; 2]],
    adj: &Blocks,
    mark: &mut [u32],
    stamp: &mut u32,
    out: &mut Vec<u32>,
) {
    if nodes.len() <= 32 {
        out.extend(nodes);
        return;
    }
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for &k in &nodes {
        for c in 0..2 {
            lo[c] = lo[c].min(at[k as usize][c]);
            hi[c] = hi[c].max(at[k as usize][c]);
        }
    }
    let axis = usize::from(hi[1] - lo[1] > hi[0] - lo[0]);
    let mut by = nodes;
    by.sort_by(|&a, &b| {
        at[a as usize][axis]
            .total_cmp(&at[b as usize][axis])
            .then(a.cmp(&b))
    });
    // the cut between whole rows where it can be
    let cut = at[by[by.len() / 2] as usize][axis];
    let mut half = by.partition_point(|&k| at[k as usize][axis] < cut);
    if half == 0 {
        half = by.len() / 2;
    }
    let right = by.split_off(half);
    *stamp += 1;
    for &k in &right {
        mark[k as usize] = *stamp;
    }
    let s = *stamp;
    let (sep, left): (Vec<u32>, Vec<u32>) = by
        .into_iter()
        .partition(|&k| adj.row(k as usize).any(|(_, v)| mark[v] == s));
    split(left, at, adj, mark, stamp, out);
    split(right, at, adj, mark, stamp, out);
    out.extend(sep);
}

const NONE: u32 = u32::MAX;

/// The work (multiply-adds) a factor must hold before its subtrees go on threads: about a sheet
/// of 100 nodes across. Below it the threads' start and their waits for the slowest cost more
/// than they save on a loaded machine (a 56-node cover's test 8 s on one thread, 26 s on all).
const SHARED: f64 = 3e8;

/// A subtree to factor: its supernodes lo..=hi and their panels.
type Job<'a> = (usize, usize, &'a mut [f64]);

/// A sparse Cholesky of a `Blocks` matrix over the coordinates that move: multifrontal, by
/// supernodes (runs of columns with one pattern), each a dense panel.
#[derive(Default)]
pub(crate) struct Chol {
    /// The pattern's key: each node's moving coordinates (bit c for coordinate c).
    mask: Vec<u8>,
    at: Vec<(u32, u8)>,
    /// The matrix's lower triangle by columns, each entry's block times 9 plus its place in it.
    ap: Vec<usize>,
    ai: Vec<u32>,
    src: Vec<u32>,
    sup: Vec<Super>,
    rows: Vec<u32>,
    panel: Vec<f64>,
    map: Vec<u32>,
    kids: Vec<Vec<u32>>,
    /// Subtrees (supernodes lo..=hi, each its own descendants) factored apart, and the
    /// supernodes over them, in order.
    tasks: Vec<(usize, usize)>,
    top: Vec<u32>,
}

/// A supernode: its first column and width, its rows (its own columns first) in `rows`, its
/// panel's start in `panel` (rows by width, by rows), the supernode it updates.
#[derive(Clone, Copy)]
struct Super {
    f: usize,
    k: usize,
    rows: (usize, usize),
    panel: usize,
    parent: u32,
}

impl Super {
    fn size(&self) -> usize {
        (self.rows.1 - self.rows.0) * self.k
    }
}

/// What a front reads: the supernodes, the matrix and the values.
#[derive(Clone, Copy)]
struct Fronts<'a> {
    sup: &'a [Super],
    rows: &'a [u32],
    ap: &'a [usize],
    ai: &'a [u32],
    src: &'a [u32],
    kids: &'a [Vec<u32>],
    a: &'a [[f64; 9]],
    b: Option<&'a [[f64; 9]]>,
    shift: f64,
}

impl Fronts<'_> {
    /// Supernodes lo..=hi (a subtree, `panel` their panels), its root's update (None: not
    /// positive definite).
    fn subtree(&self, lo: usize, hi: usize, panel: &mut [f64], n: usize) -> Option<Vec<f64>> {
        let mut map = vec![0u32; n];
        let mut update: Vec<Vec<f64>> = vec![vec![]; hi + 1 - lo];
        let base = self.sup[lo].panel;
        for s in lo..=hi {
            let (from, len) = (self.sup[s].panel - base, self.sup[s].size());
            let u = self.front(
                s,
                &mut panel[from..from + len],
                &mut |c| std::mem::take(&mut update[c as usize - lo]),
                &mut map,
            )?;
            update[s - lo] = u;
        }
        Some(std::mem::take(&mut update[hi - lo]))
    }

    /// Supernode `s`'s front: its matrix entries and its children's updates (`take`), its panel
    /// factored into `p`, and the update it leaves its parent (None: not positive definite).
    fn front(
        &self,
        s: usize,
        p: &mut [f64],
        take: &mut dyn FnMut(u32) -> Vec<f64>,
        map: &mut [u32],
    ) -> Option<Vec<f64>> {
        let Super { f, k, rows, .. } = self.sup[s];
        let rs = &self.rows[rows.0..rows.1];
        let r = rs.len();
        let m = r - k;
        for (q, &i) in rs.iter().enumerate() {
            map[i as usize] = q as u32;
        }
        p.iter_mut().for_each(|v| *v = 0.0);
        let mut w = vec![0.0; m * m];
        let mut diag = vec![0.0; k];
        for c in 0..k {
            for q in self.ap[f + c]..self.ap[f + c + 1] {
                let i = self.ai[q] as usize;
                let (kb, e) = (self.src[q] as usize / 9, self.src[q] as usize % 9);
                let v = self.a[kb][e] + self.b.map_or(0.0, |b| b[kb][e]);
                p[map[i] as usize * k + c] += v;
                if i == f + c {
                    p[c * k + c] += self.shift;
                    diag[c] = (v + self.shift).abs();
                }
            }
        }
        for &c in &self.kids[s] {
            let u = take(c);
            let sc = self.sup[c as usize];
            let below = &self.rows[sc.rows.0 + sc.k..sc.rows.1];
            let mc = below.len();
            for (x, &gi) in below.iter().enumerate() {
                let li = map[gi as usize] as usize;
                for (y, &gj) in below[..=x].iter().enumerate() {
                    let lj = map[gj as usize] as usize;
                    let v = u[x * mc + y];
                    if lj < k {
                        p[li * k + lj] += v;
                    } else {
                        w[(li - k) * m + lj - k] += v;
                    }
                }
            }
        }
        // the panel, column by column (left-looking in it)
        for c in 0..k {
            for i in c..r {
                let (head, tail) = p.split_at_mut(i * k);
                let row_i = &mut tail[..k];
                let s = if i == c {
                    dot(&row_i[..c], &row_i[..c])
                } else {
                    dot(&row_i[..c], &head[c * k..c * k + c])
                };
                row_i[c] -= s;
            }
            let d = p[c * k + c];
            if d.is_nan() || d <= 1e-13 * diag[c] {
                return None;
            }
            let d = d.sqrt();
            p[c * k + c] = d;
            for i in c + 1..r {
                p[i * k + c] /= d;
            }
        }
        // what it leaves its parent: its rows below less their panel rows' products
        let low = &p[k * k..];
        for x in 0..m {
            let rx = &low[x * k..x * k + k];
            for y in 0..=x {
                w[x * m + y] -= dot(rx, &low[y * k..y * k + k]);
            }
        }
        Some(w)
    }
}

/// a . b, in four running sums (the same order everywhere).
#[inline]
fn dot(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    let (a, b) = (&a[..n], &b[..n]);
    let mut s = [0.0; 4];
    let ((ca, ra), (cb, rb)) = (a.as_chunks::<4>(), b.as_chunks::<4>());
    for (x, y) in ca.iter().zip(cb) {
        s[0] += x[0] * y[0];
        s[1] += x[1] * y[1];
        s[2] += x[2] * y[2];
        s[3] += x[3] * y[3];
    }
    let mut t = (s[0] + s[2]) + (s[1] + s[3]);
    for (x, y) in ra.iter().zip(rb) {
        t += x * y;
    }
    t
}

impl Chol {
    /// The coordinates `mask` moves, in `order`'s order of the nodes: the pattern, the
    /// elimination tree and the supernodes, kept while the mask is.
    pub(crate) fn pattern(&mut self, mask: &[u8], order: &[u32], m: &Blocks) {
        if self.mask == mask && !self.ap.is_empty() {
            return;
        }
        self.mask = mask.to_vec();
        let mut dof = vec![[NONE; 3]; mask.len()];
        self.at.clear();
        for &u in order {
            for c in 0..3 {
                if mask[u as usize] >> c & 1 == 1 {
                    dof[u as usize][c] = self.at.len() as u32;
                    self.at.push((u, c as u8));
                }
            }
        }
        let n = self.at.len();
        self.ap.clear();
        self.ai.clear();
        self.src.clear();
        self.ap.push(0);
        // the upper triangle's rows of each column too, for the elimination tree
        let (mut up, mut upi) = (vec![0usize], vec![]);
        let mut col: Vec<(u32, u32)> = vec![];
        for j in 0..n {
            let (v, cv) = self.at[j];
            col.clear();
            for (_, u) in m.row(v as usize) {
                // A[i][j] is block (u, v)'s entry (cu, cv)
                let kb = m.at(u, v as usize);
                for cu in 0..3 {
                    let i = dof[u][cu];
                    if i != NONE {
                        col.push((i, (kb * 9 + cu * 3 + cv as usize) as u32));
                    }
                }
            }
            col.sort_unstable();
            for &(i, s) in &col {
                if i as usize >= j {
                    self.ai.push(i);
                    self.src.push(s);
                } else {
                    upi.push(i);
                }
            }
            self.ap.push(self.ai.len());
            up.push(upi.len());
        }
        // the elimination tree and each column's count below its diagonal (Liu; Davis, LDL)
        let mut parent = vec![NONE; n];
        let mut flag = vec![NONE; n];
        let mut lnz = vec![0usize; n];
        for k in 0..n {
            flag[k] = k as u32;
            for &i in &upi[up[k]..up[k + 1]] {
                let mut i = i as usize;
                while flag[i] != k as u32 {
                    if parent[i] == NONE {
                        parent[i] = k as u32;
                    }
                    lnz[i] += 1;
                    flag[i] = k as u32;
                    i = parent[i] as usize;
                }
            }
        }
        // supernodes: a column joins the last when it is its only parent with one row fewer
        self.sup.clear();
        let mut of = vec![0u32; n];
        let mut j = 0;
        while j < n {
            let f = j;
            while j + 1 < n && parent[j] == (j + 1) as u32 && lnz[j] == lnz[j + 1] + 1 {
                j += 1;
            }
            j += 1;
            of[f..j].iter_mut().for_each(|o| *o = self.sup.len() as u32);
            self.sup.push(Super {
                f,
                k: j - f,
                rows: (0, 0),
                panel: 0,
                parent: NONE,
            });
        }
        for s in 0..self.sup.len() {
            let last = self.sup[s].f + self.sup[s].k - 1;
            self.sup[s].parent = if parent[last] == NONE {
                NONE
            } else {
                of[parent[last] as usize]
            };
        }
        // each supernode's rows: its columns', its matrix entries' and its children's below it
        let mut kids: Vec<Vec<u32>> = vec![vec![]; self.sup.len()];
        for (s, p) in self.sup.iter().enumerate() {
            if p.parent != NONE {
                kids[p.parent as usize].push(s as u32);
            }
        }
        self.rows.clear();
        let mut seen = vec![NONE; n];
        let mut panel = 0;
        for s in 0..self.sup.len() {
            let Super { f, k, .. } = self.sup[s];
            let start = self.rows.len();
            for c in f..f + k {
                self.rows.push(c as u32);
                seen[c] = s as u32;
            }
            for c in f..f + k {
                for &i in &self.ai[self.ap[c]..self.ap[c + 1]] {
                    if seen[i as usize] != s as u32 {
                        seen[i as usize] = s as u32;
                        self.rows.push(i);
                    }
                }
            }
            for &c in &kids[s] {
                let (a, b) = self.sup[c as usize].rows;
                let kc = self.sup[c as usize].k;
                for q in a + kc..b {
                    let i = self.rows[q];
                    if seen[i as usize] != s as u32 {
                        seen[i as usize] = s as u32;
                        self.rows.push(i);
                    }
                }
            }
            self.rows[start + k..].sort_unstable();
            let r = self.rows.len() - start;
            debug_assert_eq!(r, k + lnz[f + k - 1]);
            self.sup[s].rows = (start, self.rows.len());
            self.sup[s].panel = panel;
            panel += r * k;
        }
        self.panel = vec![0.0; panel];
        self.map = vec![0; n];
        self.split(kids);
    }

    /// Shares the elimination tree out: the heaviest subtree split into its children's till
    /// there are a few for each thread; one whose supernodes are not a run of its own, and the
    /// supernodes split, are factored after.
    fn split(&mut self, kids: Vec<Vec<u32>>) {
        let ns = self.sup.len();
        let mut first: Vec<usize> = (0..ns).collect();
        let mut count = vec![1usize; ns];
        let mut work = vec![0.0f64; ns];
        for s in 0..ns {
            let r = (self.sup[s].rows.1 - self.sup[s].rows.0) as f64;
            work[s] += self.sup[s].k as f64 * r * r;
            let p = self.sup[s].parent;
            if p != NONE {
                let p = p as usize;
                first[p] = first[p].min(first[s]);
                count[p] += count[s];
                work[p] += work[s];
            }
        }
        let threads = forge_math::par::threads().min(8);
        let total: f64 = (0..ns)
            .filter(|&s| self.sup[s].parent == NONE)
            .map(|s| work[s])
            .sum();
        let mut open: Vec<usize> = (0..ns).filter(|&s| self.sup[s].parent == NONE).collect();
        if threads > 1 && total > SHARED {
            while open.len() < 4 * threads {
                let Some((i, &s)) = open
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| !kids[**s].is_empty())
                    .max_by(|a, b| work[*a.1].total_cmp(&work[*b.1]).then(b.1.cmp(a.1)))
                else {
                    break;
                };
                if work[s] < total / (8 * threads) as f64 {
                    break;
                }
                open.swap_remove(i);
                open.extend(kids[s].iter().map(|&c| c as usize));
            }
        }
        open.sort_unstable();
        let mut mine = vec![false; ns];
        self.tasks.clear();
        // one thread (the browser's), or a factor too small to share: all in order
        if threads <= 1 || total <= SHARED {
            open.clear();
        }
        for &s in &open {
            if count[s] == s + 1 - first[s] {
                self.tasks.push((first[s], s));
                mine[first[s]..=s].iter_mut().for_each(|m| *m = true);
            }
        }
        self.top = (0..ns).filter(|&s| !mine[s]).map(|s| s as u32).collect();
        self.kids = kids;
    }

    /// Whether the pattern holds every coordinate `mask` moves.
    pub(crate) fn covers(&self, mask: &[u8]) -> bool {
        self.mask.len() == mask.len() && self.mask.iter().zip(mask).all(|(a, b)| b & !a == 0)
    }

    /// The coordinates moving, in order.
    pub(crate) fn dofs(&self) -> &[(u32, u8)] {
        &self.at
    }

    /// Factors `a` (plus `b` where given, plus `shift` on the diagonal); false where it is not
    /// positive definite. The subtrees of `tasks` on their own threads, the rest after: each
    /// supernode's arithmetic the same whichever thread does it.
    pub(crate) fn factor(&mut self, a: &[[f64; 9]], b: Option<&[[f64; 9]]>, shift: f64) -> bool {
        let n = self.at.len();
        let cx = Fronts {
            sup: &self.sup,
            rows: &self.rows,
            ap: &self.ap,
            ai: &self.ai,
            src: &self.src,
            kids: &self.kids,
            a,
            b,
            shift,
        };
        let mut update: Vec<Vec<f64>> = vec![vec![]; self.sup.len()];
        let mut rest: &mut [f64] = &mut self.panel;
        let mut at = 0;
        let mut jobs = vec![];
        for &(lo, hi) in &self.tasks {
            let (from, to) = (self.sup[lo].panel, self.sup[hi].panel + self.sup[hi].size());
            let (_, tail) = std::mem::take(&mut rest).split_at_mut(from - at);
            let (mine, tail) = tail.split_at_mut(to - from);
            rest = tail;
            at = to;
            jobs.push((lo, hi, mine));
        }
        // the tasks dealt round to the threads, each thread's in turn
        let threads = forge_math::par::threads().min(8).min(jobs.len()).max(1);
        let mut hands: Vec<Vec<(usize, Job)>> = (0..threads).map(|_| vec![]).collect();
        for (i, job) in jobs.into_iter().enumerate() {
            hands[i % threads].push((i, job));
        }
        let mut roots: Vec<Option<Vec<f64>>> = vec![None; self.tasks.len()];
        let run = |hand: Vec<(usize, Job)>| {
            hand.into_iter()
                .map(|(i, (lo, hi, mine))| (i, cx.subtree(lo, hi, mine, n)))
                .collect::<Vec<_>>()
        };
        let done: Vec<(usize, Option<Vec<f64>>)> = if threads > 1 {
            std::thread::scope(|sc| {
                let hs: Vec<_> = hands.into_iter().map(|h| sc.spawn(move || run(h))).collect();
                hs.into_iter()
                    .flat_map(|h| h.join().unwrap_or_default())
                    .collect()
            })
        } else {
            hands.into_iter().flat_map(run).collect()
        };
        for (i, u) in done {
            roots[i] = u;
        }
        for (&(_, hi), u) in self.tasks.iter().zip(roots) {
            match u {
                Some(u) => update[hi] = u,
                None => return false,
            }
        }
        for &s in &self.top {
            let s = s as usize;
            let (from, len) = (self.sup[s].panel, self.sup[s].size());
            let p = &mut self.panel[from..from + len];
            match cx.front(
                s,
                p,
                &mut |c| std::mem::take(&mut update[c as usize]),
                &mut self.map,
            ) {
                Some(u) => update[s] = u,
                None => return false,
            }
        }
        true
    }

    /// Solves the factored system in place (`x` by the coordinates' order).
    pub(crate) fn solve(&self, x: &mut [f64]) {
        for s in &self.sup {
            let rs = &self.rows[s.rows.0..s.rows.1];
            let p = &self.panel[s.panel..s.panel + rs.len() * s.k];
            let k = s.k;
            for c in 0..k {
                let v = (x[s.f + c] - dot(&p[c * k..c * k + c], &x[s.f..s.f + c])) / p[c * k + c];
                x[s.f + c] = v;
            }
            for (q, &i) in rs.iter().enumerate().skip(k) {
                x[i as usize] -= dot(&p[q * k..q * k + k], &x[s.f..s.f + k]);
            }
        }
        for s in self.sup.iter().rev() {
            let rs = &self.rows[s.rows.0..s.rows.1];
            let p = &self.panel[s.panel..s.panel + rs.len() * s.k];
            let k = s.k;
            for (q, &i) in rs.iter().enumerate().skip(k) {
                let xi = x[i as usize];
                for c in 0..k {
                    x[s.f + c] -= p[q * k + c] * xi;
                }
            }
            for c in (0..k).rev() {
                let mut v = x[s.f + c];
                for q in c + 1..k {
                    v -= p[q * k + c] * x[s.f + q];
                }
                x[s.f + c] = v / p[c * k + c];
            }
        }
    }
}

/// psi's derivatives in the squared principal stretches (psi(s1, s2) = W, s = l^2): psi_1, psi_2,
/// psi_11, psi_12, psi_22, and kappa = (psi_1 - psi_2) / (s1 - s2), its limit psi_11 - psi_12 at s1 = s2.
struct Principal {
    p1: f64,
    p2: f64,
    p11: f64,
    p12: f64,
    p22: f64,
    k: f64,
}

/// s^q, exactly for the common exponents.
fn spow(s: f64, q: f64, ln: f64) -> f64 {
    if q == 0.0 {
        1.0
    } else if q == -1.0 {
        1.0 / s
    } else if q == -2.0 {
        1.0 / (s * s)
    } else if q == 1.0 {
        s
    } else {
        dmath::exp(q * ln)
    }
}

/// (s1^q - s2^q) / (s1 - s2), s1 >= s2, by its series where they are within a thousandth.
fn divided(s1: f64, s2: f64, q: f64, s1q: f64, s2q: f64) -> f64 {
    if q == 0.0 {
        return 0.0;
    }
    let e = s1 / s2 - 1.0;
    if e < 1e-3 {
        let (q1, q2, q3) = (q - 1.0, q - 2.0, q - 3.0);
        s2q / s2 * (q + e * (q * q1 / 2.0 + e * (q * q1 * q2 / 6.0 + e * q * q1 * q2 * q3 / 24.0)))
    } else {
        (s1q - s2q) / (s1 - s2)
    }
}

fn principal(law: &Law, s1: f64, s2: f64) -> Principal {
    let logs = law
        .terms
        .iter()
        .any(|t| !matches!(t.1, 2.0 | -2.0))
        .then(|| (dmath::ln(s1), dmath::ln(s2)));
    let (l1, l2) = logs.unwrap_or((0.0, 0.0));
    let mut o = Principal {
        p1: 0.0,
        p2: 0.0,
        p11: 0.0,
        p12: 0.0,
        p22: 0.0,
        k: 0.0,
    };
    for &(mu, al) in &law.terms {
        let q = 0.5 * al - 1.0;
        let (s1q, s2q) = (spow(s1, q, l1), spow(s2, q, l2));
        // l3^alpha = (s1 s2)^(-alpha / 2)
        let t = if al == 2.0 {
            1.0 / (s1 * s2)
        } else if al == -2.0 {
            s1 * s2
        } else {
            dmath::exp(-0.5 * al * (l1 + l2))
        };
        let h = 0.5 * mu;
        o.p1 += h * (s1q - t / s1);
        o.p2 += h * (s2q - t / s2);
        o.p11 += h * (q * s1q / s1 + (1.0 + 0.5 * al) * t / (s1 * s1));
        o.p22 += h * (q * s2q / s2 + (1.0 + 0.5 * al) * t / (s2 * s2));
        o.p12 += h * 0.5 * al * t / (s1 * s2);
        o.k += h * (divided(s1, s2, q, s1q, s2q) + t / (s1 * s2));
    }
    o
}

/// A symmetric 2x2 [[p, q], [q, r]] with its negative eigenvalue set to nothing.
fn clamp2(p: f64, q: f64, r: f64) -> (f64, f64, f64) {
    if p >= 0.0 && r >= 0.0 && p * r >= q * q {
        return (p, q, r);
    }
    let (m, h) = (0.5 * (p + r), 0.5 * (p - r));
    let w = m + (h * h + q * q).sqrt();
    if w <= 0.0 {
        return (0.0, 0.0, 0.0);
    }
    let (ex, ey) = if (w - p).abs() > (w - r).abs() {
        (q, w - p)
    } else {
        (w - r, q)
    };
    let n2 = ex * ex + ey * ey;
    if n2 <= 0.0 {
        return (p.max(0.0), 0.0, r.max(0.0));
    }
    let s = w / n2;
    (s * ex * ex, s * ex * ey, s * ey * ey)
}

/// One triangle's Hessian, projected, at its corners' places `x`: the blocks (0,0) (0,1) (0,2)
/// (1,1) (1,2) (2,2) (row-major; (w, v) is (v, w)'s transpose). None: turned inside out.
pub(crate) fn element(
    law: &Law,
    x: [Vec3; 3],
    dinv: [[f64; 2]; 2],
    vol: f64,
    wrinkle: bool,
) -> Option<[[f64; 9]; 6]> {
    let (e1, e2) = (x[1] - x[0], x[2] - x[0]);
    let f0 = e1 * dinv[0][0] + e2 * dinv[1][0];
    let f1 = e1 * dinv[0][1] + e2 * dinv[1][1];
    let (a, b, c) = (f0.dot(f0), f0.dot(f1), f1.dot(f1));
    let det = a * c - b * b;
    if det.is_nan() || det <= 1e-12 {
        return None;
    }
    let half = 0.5 * (a - c);
    let disc = (half * half + b * b).sqrt();
    let s1 = 0.5 * (a + c) + disc;
    let s2 = det / s1;
    let (vx, vy) = if disc <= 1e-12 * (a + c) {
        (1.0, 0.0)
    } else {
        let (vx, vy) = if (s1 - a).abs() > (s1 - c).abs() {
            (b, s1 - a)
        } else {
            (s1 - c, b)
        };
        let n = (vx * vx + vy * vy).sqrt();
        (vx / n, vy / n)
    };
    let (l1, l2) = (s1.sqrt(), s2.sqrt());
    let u1 = (f0 * vx + f1 * vy) * (1.0 / l1);
    let u2 = (f0 * -vy + f1 * vx) * (1.0 / l2);
    let n = u1.cross(u2);
    let mut pr = if wrinkle && l1 <= 1.0 {
        return Some([[0.0; 9]; 6]);
    } else {
        principal(law, s1, s2)
    };
    if wrinkle && l2 < 1.0 / l1.sqrt() {
        // wrinkled: psi~(s1) = psi(s1, 1 / l1), psi~' its psi_1, psi~'' = psi_11 - psi_12^2 / psi_22
        let w = principal(law, s1, 1.0 / l1);
        let p11 = if w.p22 > 0.0 {
            w.p11 - w.p12 * w.p12 / w.p22
        } else {
            w.p11
        };
        pr = Principal {
            p1: w.p1,
            p2: 0.0,
            p11,
            p12: 0.0,
            p22: 0.0,
            k: w.p1 / (s1 - s2).max(1e-300),
        };
    }
    let (sa, sb) = (2.0 * pr.p1, 2.0 * pr.p2);
    let st = clamp2(
        vol * (4.0 * s1 * pr.p11 + sa),
        vol * 4.0 * l1 * l2 * pr.p12,
        vol * (4.0 * s2 * pr.p22 + sb),
    );
    let sh = clamp2(
        vol * (2.0 * pr.k * s1 + sb),
        vol * 2.0 * pr.k * l1 * l2,
        vol * (2.0 * pr.k * s2 + sa),
    );
    let (r0, r1) = ((vol * sa).max(0.0), (vol * sb).max(0.0));
    // the corners' weights on F's frame columns: beta = B V, B the corners' on F's columns
    let bc = [
        [-(dinv[0][0] + dinv[1][0]), -(dinv[0][1] + dinv[1][1])],
        dinv[0],
        dinv[1],
    ];
    let beta = bc.map(|r| [r[0] * vx + r[1] * vy, -r[0] * vy + r[1] * vx]);
    let outer = |p: Vec3, q: Vec3| {
        [
            p.x * q.x,
            p.x * q.y,
            p.x * q.z,
            p.y * q.x,
            p.y * q.y,
            p.y * q.z,
            p.z * q.x,
            p.z * q.y,
            p.z * q.z,
        ]
    };
    let (o11, o12, o22, onn) = (outer(u1, u1), outer(u1, u2), outer(u2, u2), outer(n, n));
    let mut out = [[0.0; 9]; 6];
    let mut k = 0;
    for v in 0..3 {
        for w in v..3 {
            let (bv, bw) = (beta[v], beta[w]);
            let ca = st.0 * bv[0] * bw[0] + sh.0 * bv[1] * bw[1];
            let cb = st.1 * bv[0] * bw[1] + sh.1 * bv[1] * bw[0];
            let cc = st.1 * bv[1] * bw[0] + sh.1 * bv[0] * bw[1];
            let cd = st.2 * bv[1] * bw[1] + sh.2 * bv[0] * bw[0];
            let ce = r0 * bv[0] * bw[0] + r1 * bv[1] * bw[1];
            for i in 0..3 {
                for j in 0..3 {
                    // u2 u1^T is u1 u2^T's transpose
                    out[k][3 * i + j] = ca * o11[3 * i + j]
                        + cb * o12[3 * i + j]
                        + cc * o12[3 * j + i]
                        + cd * o22[3 * i + j]
                        + ce * onn[3 * i + j];
                }
            }
            k += 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy(law: &Law, x: [Vec3; 3], dinv: [[f64; 2]; 2], vol: f64, wrinkle: bool) -> f64 {
        let (e1, e2) = (x[1] - x[0], x[2] - x[0]);
        let f0 = e1 * dinv[0][0] + e2 * dinv[1][0];
        let f1 = e1 * dinv[0][1] + e2 * dinv[1][1];
        let (a, b, c) = (f0.dot(f0), f0.dot(f1), f1.dot(f1));
        let half = 0.5 * (a - c);
        let disc = (half * half + b * b).sqrt();
        let s1 = 0.5 * (a + c) + disc;
        let s2 = (a * c - b * b) / s1;
        let (l1, mut l2) = (s1.sqrt(), s2.sqrt());
        if wrinkle {
            if l1 <= 1.0 {
                return 0.0;
            }
            l2 = l2.max(1.0 / l1.sqrt());
        }
        let l3 = 1.0 / (l1 * l2);
        vol * law
            .terms
            .iter()
            .map(|&(mu, al)| {
                mu / al * (dmath::powf(l1, al) + dmath::powf(l2, al) + dmath::powf(l3, al) - 3.0)
            })
            .sum::<f64>()
    }

    /// The exact (unprojected) Hessian in tension, where no block clamps, is the energy's by
    /// central differences; and the series of the divided difference meets its direct form.
    #[test]
    fn a_triangles_hessian_is_its_energys() {
        let laws = [
            Law::neo_hookean(0.5),
            Law {
                terms: vec![(0.30, 2.0), (-0.05, -2.0)],
            },
            Law {
                terms: vec![(0.33, 1.6), (0.02, 5.1), (-0.01, -3.2)],
            },
        ];
        let dinv = {
            let (a, b, c, d) = (1.3, 0.2, 0.1, 1.1);
            let det = a * d - b * c;
            [[d / det, -b / det], [-c / det, a / det]]
        };
        let base = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.3, 0.3, 0.4),
            Vec3::new(0.5, 1.9, -0.2),
        ];
        for law in &laws {
            for wrinkle in [false, true] {
                for squash in [1.0, 0.45] {
                    let mut x = base;
                    x[2] = x[2] * squash;
                    let h = element(law, x, dinv, 0.7, wrinkle).expect("upright");
                    let e = |y: [Vec3; 3]| energy(law, y, dinv, 0.7, wrinkle);
                    let d = 1e-5;
                    let mut worst: f64 = 0.0;
                    let mut scale: f64 = 0.0;
                    let mut k = 0;
                    for v in 0..3 {
                        for w in v..3 {
                            for i in 0..3 {
                                for j in 0..3 {
                                    let at = |sv: f64, sw: f64| {
                                        let mut y = x;
                                        let mut bump = |n: usize, c: usize, s: f64| {
                                            let mut p = [y[n].x, y[n].y, y[n].z];
                                            p[c] += s;
                                            y[n] = Vec3::new(p[0], p[1], p[2]);
                                        };
                                        bump(v, i, sv);
                                        bump(w, j, sw);
                                        e(y)
                                    };
                                    let fd = (at(d, d) - at(d, -d) - at(-d, d) + at(-d, -d)) / (4.0 * d * d);
                                    worst = worst.max((fd - h[k][3 * i + j]).abs());
                                    scale = scale.max(fd.abs());
                                }
                            }
                            k += 1;
                        }
                    }
                    assert!(
                        worst < 1e-4 * scale,
                        "{law:?} wrinkle {wrinkle} squash {squash}: {worst} of {scale}"
                    );
                }
            }
        }
        let q = -0.2;
        let a = divided(
            2.0 * (1.0 + 0.999e-3),
            2.0,
            q,
            dmath::powf(2.0 * (1.0 + 0.999e-3), q),
            dmath::powf(2.0, q),
        );
        let b = divided(
            2.0 * (1.0 + 1.001e-3),
            2.0,
            q,
            dmath::powf(2.0 * (1.0 + 1.001e-3), q),
            dmath::powf(2.0, q),
        );
        // apart by their arguments' 2e-6 only
        assert!((a - b).abs() < 5e-6 * a.abs(), "{a} {b}");
    }

    /// A projected Hessian has no negative direction, compressed or turned however.
    #[test]
    fn a_projected_hessian_is_positive_semi_definite() {
        let law = Law {
            terms: vec![(0.33, 1.6), (0.02, 5.1), (-0.01, -3.2)],
        };
        let dinv = [[1.0, 0.0], [0.0, 1.0]];
        let mut s = 0x9e37_79b9_7f4a_7c15_u64;
        let mut r = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s >> 11) as f64 / (1u64 << 53) as f64 - 0.5
        };
        for _ in 0..200 {
            let x = [
                Vec3::new(r(), r(), r()) * 0.3,
                Vec3::new(1.0 + r(), r(), r()),
                Vec3::new(r(), 1.0 + r(), r()),
            ];
            let Some(h) = element(&law, x, dinv, 1.0, false) else {
                continue;
            };
            // the 9x9 from its blocks; its least Rayleigh quotient over random directions and corners
            let full = |a: usize, b: usize| {
                let (v, w) = (a / 3, b / 3);
                let (v, w, i, j) = if v <= w {
                    (v, w, a % 3, b % 3)
                } else {
                    (w, v, b % 3, a % 3)
                };
                let k = [[0, 1, 2], [1, 3, 4], [2, 4, 5]][v][w];
                h[k][3 * i + j]
            };
            let norm = (0..9).map(|i| full(i, i).abs()).fold(0.0, f64::max);
            for _ in 0..40 {
                let d: Vec<f64> = (0..9).map(|_| r()).collect();
                let q: f64 = (0..9)
                    .flat_map(|a| (0..9).map(move |b| (a, b)))
                    .map(|(a, b)| d[a] * full(a, b) * d[b])
                    .sum();
                assert!(q >= -1e-10 * norm, "{q}");
            }
        }
    }

    /// The factor solves a sheet-shaped system: a grid's Laplacian on its moving coordinates,
    /// large enough to be shared out over threads.
    #[test]
    fn the_factor_solves_a_grids_system() {
        let n = 120;
        let id = |i: usize, j: usize| j * (n + 1) + i;
        let mut tris = vec![];
        for j in 0..n {
            for i in 0..n {
                tris.push([id(i, j), id(i + 1, j), id(i + 1, j + 1)]);
                tris.push([id(i, j), id(i + 1, j + 1), id(i, j + 1)]);
            }
        }
        let nodes = (n + 1) * (n + 1);
        let groups: Vec<&[usize]> = tris.iter().map(|t| &t[..]).collect();
        let mut m = Blocks::new(nodes, &groups);
        let at: Vec<[f64; 2]> = (0..nodes)
            .map(|k| [(k % (n + 1)) as f64, (k / (n + 1)) as f64])
            .collect();
        let order = dissect(&at, &m);
        let mut seen = order.clone();
        seen.sort_unstable();
        assert_eq!(seen, (0..nodes as u32).collect::<Vec<_>>());
        for t in &tris {
            for &u in t {
                for &v in t {
                    let k = m.at(u, v);
                    let w = if u == v { 2.0 } else { -1.0 };
                    for c in 0..3 {
                        m.val[k][4 * c] += w;
                    }
                    // a coupling between the coordinates
                    m.val[k][1] += 0.1 * w;
                    m.val[k][3] += 0.1 * w;
                }
            }
        }
        let mask: Vec<u8> = (0..nodes)
            .map(|k| {
                let (i, j) = (k % (n + 1), k / (n + 1));
                if i == 0 || j == 0 || i == n || j == n {
                    0
                } else if i == 5 {
                    0b011
                } else {
                    0b111
                }
            })
            .collect();
        let mut f = Chol::default();
        f.pattern(&mask, &order, &m);
        assert!(f.factor(&m.val, None, 0.0));
        assert!(forge_math::par::threads() == 1 || f.tasks.len() > 1);
        let dofs = f.dofs().to_vec();
        let want: Vec<f64> = (0..dofs.len()).map(|k| ((k * 7919) % 13) as f64 - 6.0).collect();
        // b = A want
        let mut index = vec![[usize::MAX; 3]; nodes];
        for (k, &(u, c)) in dofs.iter().enumerate() {
            index[u as usize][c as usize] = k;
        }
        let mut b = vec![0.0; dofs.len()];
        for (r, &(u, cu)) in dofs.iter().enumerate() {
            for (k, v) in m.row(u as usize) {
                for cv in 0..3 {
                    let c = index[v][cv];
                    if c != usize::MAX {
                        b[r] += m.val[k][3 * cu as usize + cv] * want[c];
                    }
                }
            }
        }
        f.solve(&mut b);
        let err = b
            .iter()
            .zip(&want)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(err < 1e-9, "{err}");
        // not definite: refused
        let neg: Vec<[f64; 9]> = m.val.iter().map(|b| b.map(|v| -v)).collect();
        assert!(!f.factor(&neg, None, 0.0));
    }
}

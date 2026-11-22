//! `place_cuts`' clustering (kernel/forge-bool/src/boolean.rs:4561-4620 at 888fc195), made
//! float-free by passing every distance test in as a table indexed by whose tolerance it uses:
//!
//!   le1[i][j][a] = cuts[i].point.distance(cuts[j].point) <= cuts[a].tol
//!   le2[i][j][a] = cuts[i].point.distance(cuts[j].point) <= 2.0 * cuts[a].tol
//!
//! Every test in `place_cuts` has the form `d(i, j) <= c * max(tol_a, tol_b)` with c in {1, 2},
//! and for non-NaN floats `d <= max(x, y)` iff `d <= x || d <= y` (`max` returns one of its
//! arguments), and `2.0 * max(x, y) == max(2.0 * x, 2.0 * y)` (doubling is exact and monotone).
//! So the tables decide each test exactly as the kernel does. Note the merge (4598-4606) measures
//! a cut against both heads with the *heads'* reach `max(tol_o, tol_k)`, not the cut's own.
//!
//! `place` returns each cut's cluster (an index into the surviving heads).

/// d(i, j) <= max(tol_a, tol_b)
fn near(le1: &Vec<Vec<Vec<bool>>>, i: usize, j: usize, a: usize, b: usize) -> bool {
    le1[i][j][a] || le1[i][j][b]
}

/// d(i, j) <= 2 * max(tol_a, tol_b)
fn near2(le2: &Vec<Vec<Vec<bool>>>, i: usize, j: usize, a: usize, b: usize) -> bool {
    le2[i][j][a] || le2[i][j][b]
}

/// The greedy pass (4562-4583): each cut joins the first head within reach, or becomes a head;
/// `chained` records a head passed over within two reaches.
pub fn greedy(
    n: usize,
    le1: &Vec<Vec<Vec<bool>>>,
    le2: &Vec<Vec<Vec<bool>>>,
) -> (Vec<usize>, Vec<usize>, bool) {
    let mut cluster: Vec<usize> = Vec::new();
    let mut heads: Vec<usize> = Vec::new();
    let mut chained = false;
    let mut i = 0;
    while i < n {
        let mut found = false;
        let mut at = 0;
        let mut k = 0;
        while k < heads.len() {
            let h = heads[k];
            if near(le1, h, i, h, i) {
                found = true;
                at = k;
                break;
            }
            if near2(le2, h, i, h, i) {
                chained = true;
            }
            k += 1;
        }
        if found {
            cluster.push(at);
        } else {
            cluster.push(heads.len());
            heads.push(i);
        }
        i += 1;
    }
    (cluster, heads, chained)
}

/// Some cut of cluster k or o within the heads' reach of both heads (4600-4604).
fn shared(
    n: usize,
    le1: &Vec<Vec<Vec<bool>>>,
    cluster: &Vec<usize>,
    h: usize,
    hk: usize,
    k: usize,
    o: usize,
) -> bool {
    let mut any = false;
    let mut i = 0;
    while i < n {
        if (cluster[i] == k || cluster[i] == o) && near(le1, i, h, h, hk) && near(le1, i, hk, h, hk) {
            any = true;
        }
        i += 1;
    }
    any
}

/// Relabel cluster o as k and close the gap (4607-4613).
fn fold(n: usize, cluster: &mut Vec<usize>, k: usize, o: usize) {
    let mut x = 0;
    while x < n {
        if cluster[x] == o {
            cluster[x] = k;
        } else if cluster[x] > o {
            cluster[x] -= 1;
        }
        x += 1;
    }
}

/// `heads.remove(o)`, written out.
fn remove_at(heads: &Vec<usize>, o: usize) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    let mut j = 0;
    while j < heads.len() {
        if j != o {
            out.push(heads[j]);
        }
        j += 1;
    }
    out
}

/// Merging heads joined by a chain (4596-4620): from the last head down, a head within two of
/// the heads' reach of an earlier head, with a cut of either within that reach of both, merges
/// into the first such earlier head.
pub fn merge(
    n: usize,
    le1: &Vec<Vec<Vec<bool>>>,
    le2: &Vec<Vec<Vec<bool>>>,
    cluster: &mut Vec<usize>,
    heads: &mut Vec<usize>,
    chained: bool,
) {
    let top = if chained { heads.len() } else { 1 };
    let mut o = top;
    while o > 1 {
        o -= 1;
        let h = heads[o];
        let mut k = 0;
        while k < o {
            let hk = heads[k];
            if near2(le2, h, hk, h, hk) && shared(n, le1, cluster, h, hk, k, o) {
                fold(n, cluster, k, o);
                *heads = remove_at(heads, o);
                break;
            }
            k += 1;
        }
    }
}

pub fn place(n: usize, le1: &Vec<Vec<Vec<bool>>>, le2: &Vec<Vec<Vec<bool>>>) -> Vec<usize> {
    let (mut cluster, mut heads, chained) = greedy(n, le1, le2);
    merge(n, le1, le2, &mut cluster, &mut heads, chained);
    cluster
}

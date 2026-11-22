//! `place_cuts`' clustering (kernel/forge-bool/src/boolean.rs:4561-4620 at 888fc195), with its
//! distance tests given as tables so the function is float-free:
//! near[i][j]  = cuts[i].point.distance(cuts[j].point) <= cuts[i].tol.max(cuts[j].tol)   (4568, 4605-4606)
//! within2[i][j] = the same distance <= 2.0 * that reach                                   (4572, 4603)
//! Returns each cut's cluster (head index into the surviving heads).

/// The greedy pass (4562-4583): each cut joins the first head within reach, or becomes a head.
pub fn greedy(n: usize, near: &Vec<Vec<bool>>, within2: &Vec<Vec<bool>>) -> (Vec<usize>, Vec<usize>, bool) {
    let mut cluster: Vec<usize> = Vec::new();
    let mut heads: Vec<usize> = Vec::new();
    let mut chained = false;
    let mut i = 0;
    while i < n {
        let mut found = n; // n: none
        let mut k = 0;
        while k < heads.len() {
            let h = heads[k];
            if near[h][i] {
                found = k;
                break;
            }
            if within2[h][i] {
                chained = true;
            }
            k += 1;
        }
        if found < n {
            cluster.push(found);
        } else {
            cluster.push(heads.len());
            heads.push(i);
        }
        i += 1;
    }
    (cluster, heads, chained)
}

/// Merging heads joined by a chain (4596-4620): a later head within two reaches of an earlier one,
/// with a cut of either within reach of both, merges into the earlier.
pub fn merge(
    n: usize,
    near: &Vec<Vec<bool>>,
    within2: &Vec<Vec<bool>>,
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
            let mut any = false;
            let mut i = 0;
            while i < n {
                if (cluster[i] == k || cluster[i] == o) && near[i][h] && near[i][hk] {
                    any = true;
                }
                i += 1;
            }
            if within2[h][hk] && any {
                let mut x = 0;
                while x < n {
                    if cluster[x] == o {
                        cluster[x] = k;
                    } else if cluster[x] > o {
                        cluster[x] -= 1;
                    }
                    x += 1;
                }
                heads.remove(o);
                break;
            }
            k += 1;
        }
    }
}

pub fn place(n: usize, near: &Vec<Vec<bool>>, within2: &Vec<Vec<bool>>) -> Vec<usize> {
    let (mut cluster, mut heads, chained) = greedy(n, near, within2);
    merge(n, near, within2, &mut cluster, &mut heads, chained);
    cluster
}

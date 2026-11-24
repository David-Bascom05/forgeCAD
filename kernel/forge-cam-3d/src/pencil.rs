//! Pencil finishing: one pass along each concave corner the tool fits into tightly.
//!
//! Method (plain, published: the double-contact locus. In a concave corner sharper than the tool,
//! the tool touches two faces at once, and the CL surface, the upper envelope of the faces'
//! offsets, has a crease there: a valley with a jump in slope). On the grid of CL heights a node
//! is on a crease when, along one of four directions (x, y and the diagonals), it is a local
//! minimum and the second difference exceeds `kink` (for a crease it grows like 1/g as the grid
//! shrinks; on a smooth concave fillet of radius ρ > R it stays near 1/(ρ − R)). Crease nodes
//! are traced into polylines through their 8-neighbours, and draped (drape.rs), which makes
//! every move safe; the crease's own position matters only for where the pass runs.

use crate::contain::Containment;
use crate::drape::{self, Drape, DrapeError, Pass};
use crate::drop::DropCutter;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub grid: f64,
    /// The least second difference (1/mm) that marks a crease: 1/(the largest corner radius,
    /// less the tool radius, that still counts as a pencil corner).
    pub kink: f64,
    /// Polylines shorter than this are dropped.
    pub min_len: f64,
}

/// The crease nodes' xy, traced into polylines.
pub fn creases(dc: &dyn DropCutter, p: Params, c: &Containment) -> Vec<Vec<[f64; 2]>> {
    let [lo, hi] = dc.extent();
    let g = p.grid;
    let nx = (((hi[0] - lo[0]) / g).ceil() as usize) + 1;
    let ny = (((hi[1] - lo[1]) / g).ceil() as usize) + 1;
    let mut z = vec![f64::NAN; nx * ny];
    let mut k = 0;
    while k < z.len() {
        let x = lo[0] + g * (k % nx) as f64;
        let y = lo[1] + g * (k / nx) as f64;
        if let Some(b) = dc.drop(x, y) {
            z[k] = 0.5 * (b.lo + b.hi);
        }
        k += 1;
    }
    let mut on = vec![false; nx * ny];
    const DIRS: [(i64, i64); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];
    let mut j = 1;
    while j + 1 < ny {
        let mut i = 1;
        while i + 1 < nx {
            let k = j * nx + i;
            if !z[k].is_nan() {
                for (di, dj) in DIRS {
                    let a = ((j as i64 - dj) as usize) * nx + (i as i64 - di) as usize;
                    let b = ((j as i64 + dj) as usize) * nx + (i as i64 + di) as usize;
                    if z[a].is_nan() || z[b].is_nan() {
                        continue;
                    }
                    let h2 = g * g * ((di * di + dj * dj) as f64);
                    let d2 = (z[a] + z[b] - 2.0 * z[k]) / h2;
                    if d2 >= p.kink && z[k] <= z[a] && z[k] <= z[b] {
                        on[k] = true;
                    }
                }
            }
            i += 1;
        }
        j += 1;
    }
    // trace: from ends (one neighbour) first, then what is left (rings)
    let nbrs = |k: usize, on: &Vec<bool>| -> Vec<usize> {
        let (i, j) = ((k % nx) as i64, (k / nx) as i64);
        let mut v = Vec::new();
        for dj in -1i64..=1 {
            for di in -1i64..=1 {
                if di == 0 && dj == 0 {
                    continue;
                }
                let (a, b) = (i + di, j + dj);
                if a >= 0 && b >= 0 && a < nx as i64 && b < ny as i64 {
                    let q = (b as usize) * nx + a as usize;
                    if on[q] {
                        v.push(q);
                    }
                }
            }
        }
        v
    };
    let xy = |k: usize| [lo[0] + g * (k % nx) as f64, lo[1] + g * (k / nx) as f64];
    let mut used = vec![false; nx * ny];
    let mut out = Vec::new();
    let mut pass = 0;
    while pass < 2 {
        let mut k = 0;
        while k < on.len() {
            if on[k] && !used[k] {
                let deg = nbrs(k, &on).iter().filter(|q| !used[**q]).count();
                if pass == 1 || deg <= 1 {
                    let mut line = Vec::new();
                    let mut cur = k;
                    loop {
                        used[cur] = true;
                        line.push(xy(cur));
                        // the next unused neighbour, 4-neighbours first (straighter lines)
                        let ns = nbrs(cur, &on);
                        let mut nxt = None;
                        for q in &ns {
                            let d = (q % nx).abs_diff(cur % nx) + (q / nx).abs_diff(cur / nx);
                            if !used[*q] && d == 1 {
                                nxt = Some(*q);
                                break;
                            }
                        }
                        if nxt.is_none() {
                            for q in &ns {
                                if !used[*q] {
                                    nxt = Some(*q);
                                    break;
                                }
                            }
                        }
                        match nxt {
                            Some(q) => cur = q,
                            None => break,
                        }
                    }
                    let mut len = 0.0;
                    let mut m = 1;
                    while m < line.len() {
                        let (a, b) = (line[m - 1], line[m]);
                        len += ((a[0] - b[0]) * (a[0] - b[0]) + (a[1] - b[1]) * (a[1] - b[1])).sqrt();
                        m += 1;
                    }
                    let inside = match &c.boundary {
                        Some(b) => line.iter().any(|q| crate::contain::inside(b, *q)),
                        None => true,
                    };
                    if len >= p.min_len && inside {
                        out.push(line);
                    }
                }
            }
            k += 1;
        }
        pass += 1;
    }
    out
}

/// Pencil passes: the creases, draped and clipped to the containment.
pub fn pencil(dc: &dyn DropCutter, p: Params, d: Drape, c: &Containment) -> Result<Vec<Pass>, DrapeError> {
    let mut out = Vec::new();
    for passes in drape::drape_all(dc, &creases(dc, p, c), d)? {
        out.extend(passes);
    }
    Ok(c.clip(out, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop::Analytic;

    #[test]
    fn the_foot_of_a_dome_is_a_pencil_ring() {
        // a ball r = 3 at the foot of a sphere r = 30 on the floor: a concave corner (the
        // sphere meets the floor at about 70.5°): the CL crease is the circle of radius
        // √(33² − 13²) ≈ 30.33
        let dc = Analytic {
            r: 3.0,
            planes: vec![(0.0, [-40.0, -40.0], [40.0, 40.0])],
            spheres: vec![([0.0, 0.0, -10.0], 30.0)],
        };
        let p = Params {
            grid: 0.5,
            kink: 1.0,
            min_len: 5.0,
        };
        let cs = creases(&dc, p, &Containment::default());
        assert!(!cs.is_empty());
        let want = (33.0f64 * 33.0 - 13.0 * 13.0).sqrt();
        let mut n = 0;
        for l in &cs {
            for q in l {
                let r = (q[0] * q[0] + q[1] * q[1]).sqrt();
                assert!((r - want).abs() < 1.0, "r={r}");
                n += 1;
            }
        }
        assert!(n > 100);
        let d = Drape {
            step: 1.0,
            slack: 0.005,
            leave: 0.01,
            zmin: 0.0,
        };
        let ps = pencil(&dc, p, d, &Containment::default()).unwrap();
        assert!(!ps.is_empty());
    }

    #[test]
    fn no_pencil_on_a_smooth_dome_top() {
        let dc = Analytic {
            r: 3.0,
            planes: vec![],
            spheres: vec![([0.0, 0.0, -10.0], 30.0)],
        };
        let c = Containment {
            boundary: Some(vec![[-15.0, -15.0], [15.0, -15.0], [15.0, 15.0], [-15.0, 15.0]]),
            ..Default::default()
        };
        let cs = creases(
            &dc,
            Params {
                grid: 0.5,
                kink: 1.0,
                min_len: 2.0,
            },
            &c,
        );
        assert!(cs.is_empty());
    }
}

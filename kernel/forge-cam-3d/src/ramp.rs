//! Ramp finishing: waterline-like passes that descend continuously from one level to the next
//! instead of stepping down at one place (no step-down mark).
//!
//! Method (plain, published: a contour ramp between z-levels). For each closed waterline loop at
//! a level, the loop at the level above that it lies under (nearest centroid) is its partner.
//! The ramp's xy runs once round the lower loop, starting at its vertex nearest the partner's
//! start, while blending from the partner's nearest points towards the lower loop by arc length
//! (s = 0 on the upper loop, s = 1 on the lower). The xy curve is then draped (drape.rs): the
//! heights are the CL heights there, so the ramp descends as the surface does and every move is
//! proven like any draped pass. A loop with no partner (the top of a boss) is draped as is.

use crate::drape::{self, Drape, DrapeError, Pass};
use crate::drop::DropCutter;
use crate::waterline::Loop;

fn centroid(p: &[[f64; 2]]) -> [f64; 2] {
    let mut s = [0.0, 0.0];
    for q in p {
        s[0] += q[0];
        s[1] += q[1];
    }
    let n = p.len().max(1) as f64;
    [s[0] / n, s[1] / n]
}

fn d2(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]) * (a[0] - b[0]) + (a[1] - b[1]) * (a[1] - b[1])
}

/// The point of the closed polyline `p` nearest q.
fn nearest_on(p: &[[f64; 2]], q: [f64; 2]) -> [f64; 2] {
    let n = p.len();
    let mut best = p[0];
    let mut bd = f64::INFINITY;
    let mut k = 0;
    while k < n {
        let a = p[k];
        let b = p[(k + 1) % n];
        let ab = [b[0] - a[0], b[1] - a[1]];
        let l2 = ab[0] * ab[0] + ab[1] * ab[1];
        let t = if l2 > 0.0 {
            (((q[0] - a[0]) * ab[0] + (q[1] - a[1]) * ab[1]) / l2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let c = [a[0] + ab[0] * t, a[1] + ab[1] * t];
        let d = d2(c, q);
        if d < bd {
            bd = d;
            best = c;
        }
        k += 1;
    }
    best
}

/// Ramp passes from waterline loops grouped by level, top level first.
pub fn ramps(dc: &dyn DropCutter, levels: &[Vec<Loop>], d: Drape) -> Result<Vec<Pass>, DrapeError> {
    let mut out = Vec::new();
    let mut li = 0;
    while li < levels.len() {
        for lower in &levels[li] {
            if lower.pts.len() < 3 {
                continue;
            }
            let lc = centroid(&lower.pts);
            let partner = if li == 0 {
                None
            } else {
                let mut best: Option<&Loop> = None;
                let mut bd = f64::INFINITY;
                for up in &levels[li - 1] {
                    let dd = d2(centroid(&up.pts), lc);
                    if up.pts.len() >= 3 && dd < bd {
                        bd = dd;
                        best = Some(up);
                    }
                }
                best
            };
            let n = lower.pts.len();
            let xy: Vec<[f64; 2]> = match partner {
                None => {
                    let mut v = lower.pts.clone();
                    v.push(lower.pts[0]);
                    v
                }
                Some(up) => {
                    // start at the lower loop's vertex nearest the upper loop's start
                    let mut s0 = 0;
                    let mut bd = f64::INFINITY;
                    let mut k = 0;
                    while k < n {
                        let dd = d2(lower.pts[k], up.pts[0]);
                        if dd < bd {
                            bd = dd;
                            s0 = k;
                        }
                        k += 1;
                    }
                    let mut total = 0.0;
                    let mut k = 0;
                    while k < n {
                        total += d2(lower.pts[(s0 + k) % n], lower.pts[(s0 + k + 1) % n]).sqrt();
                        k += 1;
                    }
                    let mut v = Vec::with_capacity(n + 1);
                    let mut acc = 0.0;
                    let mut k = 0;
                    while k <= n {
                        let q = lower.pts[(s0 + k) % n];
                        let s = if total > 0.0 { acc / total } else { 1.0 };
                        let u = nearest_on(&up.pts, q);
                        v.push([u[0] + (q[0] - u[0]) * s, u[1] + (q[1] - u[1]) * s]);
                        if k < n {
                            acc += d2(q, lower.pts[(s0 + k + 1) % n]).sqrt();
                        }
                        k += 1;
                    }
                    v
                }
            };
            out.extend(drape::drape(dc, &xy, d)?);
        }
        li += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop::Analytic;
    use crate::waterline::{self, Params};

    #[test]
    fn a_ramp_descends_one_level_per_turn() {
        let dc = Analytic {
            r: 3.0,
            planes: vec![(0.0, [-50.0, -50.0], [50.0, 50.0])],
            spheres: vec![([0.0, 0.0, -10.0], 30.0)],
        };
        let lv: Vec<Vec<Loop>> = [9.0, 8.0, 7.0]
            .iter()
            .map(|&z| {
                waterline::level(
                    &dc,
                    z,
                    Params {
                        grid: 2.0,
                        tol_xy: 0.01,
                    },
                )
                .unwrap()
            })
            .collect();
        let d = Drape {
            step: 1.0,
            slack: 0.005,
            leave: 0.01,
            zmin: 0.0,
        };
        let ps = ramps(&dc, &lv, d).unwrap();
        assert_eq!(ps.len(), 3);
        // the second and third descend from about one level to the next
        for (k, p) in ps.iter().enumerate().skip(1) {
            let z_hi = [9.0, 8.0, 7.0][k - 1];
            let z_lo = [9.0, 8.0, 7.0][k];
            assert!((p[0][2] - z_hi).abs() < 0.05, "start {}", p[0][2]);
            assert!(
                (p[p.len() - 1][2] - z_lo).abs() < 0.05,
                "end {}",
                p[p.len() - 1][2]
            );
            // monotone down, within the drape's tolerance
            let mut m = 1;
            while m < p.len() {
                assert!(p[m][2] <= p[m - 1][2] + 0.02);
                m += 1;
            }
        }
    }
}

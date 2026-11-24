//! Draping: an xy polyline dropped onto the cutter-location surface, as 3D passes whose every
//! chord is proven free of gouges (beyond `slack`) by the drop-cutter.
//!
//! Parallel, radial, spiral, scallop and flat-area finishing differ only in the xy pattern they
//! drape (pattern.rs). This is generation, not a later fix-up stage: each point is a drop-cutter
//! CL point (the upper end of its certified band, so never below the true CL height), and the
//! samples are refined until every chord is proven within tolerance before it is emitted.
//!
//! - A sample with nothing under the tool ends the pass (the pattern left the part).
//! - A chord is accepted when (1) the CL height is proven ≤ the chord + `slack` all along it
//!   (`seg3_below`: it gouges by at most slack, slack ≤ the op's inner tolerance), and (2) at its
//!   midpoint the chord is at most `leave` above the CL height (it does not air-cut the surface
//!   by more than the outer tolerance there). Otherwise it is split at the xy midpoint.
//! - `zmin` raises every point to at least that height (a floor the op must not go below);
//!   raising a point never makes a move gouge.

use crate::drop::DropCutter;

/// A 3D pass: tool-tip points, joined by straight cutting moves.
pub type Pass = Vec<[f64; 3]>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drape {
    /// The largest xy spacing of the first samples.
    pub step: f64,
    /// How far a chord may dip below the CL surface (≤ the inner tolerance).
    pub slack: f64,
    /// How far a chord's midpoint may lie above the CL surface (the outer tolerance).
    pub leave: f64,
    /// No point below this height.
    pub zmin: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DrapeError {
    Bad(&'static str),
    /// A chord could not be proven within the tolerance: where (xy).
    Unproven {
        at: [f64; 2],
    },
}

fn cl(dc: &dyn DropCutter, p: [f64; 2], zmin: f64) -> Option<[f64; 3]> {
    dc.drop(p[0], p[1]).map(|b| [p[0], p[1], b.hi.max(zmin)])
}

/// Drapes the polyline `xy` (open) onto the CL surface.
pub fn drape(dc: &dyn DropCutter, xy: &[[f64; 2]], d: Drape) -> Result<Vec<Pass>, DrapeError> {
    if !(d.step > 0.0 && d.slack >= 0.0 && d.leave > 0.0) {
        return Err(DrapeError::Bad("step/slack/leave"));
    }
    let mut out: Vec<Pass> = Vec::new();
    let mut cur: Pass = Vec::new();
    let mut k = 0;
    while k + 1 < xy.len() || (k == 0 && xy.len() == 1) {
        let a = xy[k];
        let b = if xy.len() == 1 { a } else { xy[k + 1] };
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = (dx * dx + dy * dy).sqrt();
        let n = ((len / d.step).ceil() as usize).max(1);
        // the first piece of the first segment includes its start
        let first = if k == 0 { 0 } else { 1 };
        let mut i = first;
        while i <= n {
            let t = (i as f64) / (n as f64);
            let p = [a[0] + dx * t, a[1] + dy * t];
            match cl(dc, p, d.zmin) {
                None => {
                    if cur.len() > 1 {
                        out.push(core::mem::take(&mut cur));
                    } else {
                        cur.clear();
                    }
                }
                Some(q) => {
                    // a chord that leaves the part between samples ends the pass there
                    if let Some(&prev) = cur.last()
                        && !refine(dc, prev, q, d, 0, &mut cur)?
                    {
                        if cur.len() > 1 {
                            out.push(core::mem::take(&mut cur));
                        } else {
                            cur.clear();
                        }
                    }
                    cur.push(q);
                }
            }
            i += 1;
        }
        k += 1;
        if xy.len() == 1 {
            break;
        }
    }
    if cur.len() > 1 {
        out.push(cur);
    }
    Ok(out)
}

/// Appends the points strictly between a and b that make every chord acceptable. False when a
/// midpoint has nothing under the tool (the caller breaks the pass).
fn refine(
    dc: &dyn DropCutter,
    a: [f64; 3],
    b: [f64; 3],
    d: Drape,
    depth: u32,
    out: &mut Pass,
) -> Result<bool, DrapeError> {
    let m = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
    let mz = 0.5 * (a[2] + b[2]);
    let below = dc.seg3_below(a, b, d.slack);
    let close = match dc.drop(m[0], m[1]) {
        None => return Ok(false),
        Some(band) => mz - band.lo.max(d.zmin) <= d.leave,
    };
    if below && close {
        return Ok(true);
    }
    if depth > 40 {
        return Err(DrapeError::Unproven { at: m });
    }
    let Some(q) = cl(dc, m, d.zmin) else {
        return Ok(false);
    };
    if !refine(dc, a, q, d, depth + 1, out)? {
        return Ok(false);
    }
    out.push(q);
    refine(dc, q, b, d, depth + 1, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop::Analytic;

    fn dome() -> Analytic {
        Analytic {
            r: 3.0,
            planes: vec![(0.0, [-50.0, -50.0], [50.0, 50.0])],
            spheres: vec![([0.0, 0.0, -10.0], 30.0)],
        }
    }

    #[test]
    fn a_line_over_the_dome() {
        let dc = dome();
        let d = Drape {
            step: 2.0,
            slack: 0.005,
            leave: 0.005,
            zmin: f64::NEG_INFINITY,
        };
        let ps = drape(&dc, &[[-49.0, 0.0], [49.0, 0.0]], d).unwrap();
        assert_eq!(ps.len(), 1);
        let p = &ps[0];
        // every chord within tolerance, every point on or above the CL surface
        let mut k = 0;
        while k + 1 < p.len() {
            assert!(dc.seg3_below(p[k], p[k + 1], 0.005));
            let b = dc.drop(p[k][0], p[k][1]).unwrap();
            assert!(p[k][2] >= b.lo && p[k][2] - b.hi <= 1e-12);
            k += 1;
        }
        // the top
        let top = p.iter().map(|q| q[2]).fold(f64::NEG_INFINITY, f64::max);
        assert!((top - 20.0).abs() < 0.01, "{top}");
        // fewer points on the flat floor than on the dome
        assert!(p.len() < 400, "{}", p.len());
    }

    #[test]
    fn leaving_the_part_splits_the_pass() {
        let dc = Analytic {
            r: 3.0,
            planes: vec![
                (0.0, [-10.0, -10.0], [10.0, 10.0]),
                (0.0, [30.0, -10.0], [40.0, 10.0]),
            ],
            spheres: vec![],
        };
        let d = Drape {
            step: 1.0,
            slack: 0.005,
            leave: 0.005,
            zmin: f64::NEG_INFINITY,
        };
        let ps = drape(&dc, &[[-20.0, 0.0], [50.0, 0.0]], d).unwrap();
        assert_eq!(ps.len(), 2);
    }
}

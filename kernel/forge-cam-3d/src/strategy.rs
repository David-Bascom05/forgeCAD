//! The draped strategies (parallel, radial, spiral) and flat-area finishing, with containment.
//!
//! Each returns passes (tool-tip polylines) whose every chord is proven free of gouges beyond
//! the drape's slack where it was generated; link.rs orders and links them; forge-cam's checker
//! certifies the result again against the exact faces.
//!
//! Methods (plain, published):
//! - Parallel: raster lines dropped onto the CL surface (Huang & Oliver 1994; surface-finish.md
//!   §2.1). CAM-3AXIS owns the primary Parallel op; this is the same drape, used by
//!   steep-and-shallow and available for containment-limited passes.
//! - Radial: spokes about a user point; Spiral: an Archimedean spiral about a user point
//!   (pattern.rs). Neither grows from a medial axis (US 10,108,172 avoided).
//! - Flat area: the heights where the CL surface is flat (a flat face under a flat or bull
//!   tool), each finished by raster passes at exactly that height, kept where the tool stands on
//!   the face (its CL band within [z − tol, z]) and each chord proven at that height within half
//!   the tolerance (the certifier allows the tolerance; exactly 0 at a face's own height is not
//!   provable against a face only known to its lowering's μ).

use crate::contain::Containment;
use crate::drape::{self, Drape, DrapeError, Pass};
use crate::drop::DropCutter;
use crate::pattern;
use crate::steep::{SlopeMap, slope_map};

fn finish(
    dc: &dyn DropCutter,
    lines: Vec<Vec<[f64; 2]>>,
    d: Drape,
    c: &Containment,
    sm: Option<&SlopeMap>,
) -> Result<Vec<Pass>, DrapeError> {
    let mut ps = Vec::new();
    for passes in drape::drape_all(dc, &lines, d)? {
        ps.extend(passes);
    }
    let owned;
    let sm = match sm {
        Some(s) => Some(s),
        None => {
            if c.tan_slope.0 > 0.0 || c.tan_slope.1 < f64::INFINITY {
                owned = slope_map(dc, d.step);
                Some(&owned)
            } else {
                None
            }
        }
    };
    let m = sm.and_then(|s| c.mask(s));
    Ok(c.clip(ps, m.as_ref()))
}

/// Parallel passes at `angle_deg`, `stepover` apart (horizontal), over the part's extent.
pub fn parallel(
    dc: &dyn DropCutter,
    angle_deg: f64,
    stepover: f64,
    d: Drape,
    c: &Containment,
) -> Result<Vec<Pass>, DrapeError> {
    let [lo, hi] = dc.extent();
    finish(dc, pattern::raster(lo, hi, angle_deg, stepover, true), d, c, None)
}

/// Radial spokes about `centre` between radii r0 and r1, at most `step` apart at r1.
pub fn radial(
    dc: &dyn DropCutter,
    centre: [f64; 2],
    r0: f64,
    r1: f64,
    step: f64,
    d: Drape,
    c: &Containment,
) -> Result<Vec<Pass>, DrapeError> {
    finish(dc, pattern::radial(centre, r0, r1, step, true), d, c, None)
}

/// An Archimedean spiral about `centre`, `pitch` apart, from r0 to r1 (inward when asked).
#[allow(clippy::too_many_arguments)]
pub fn spiral(
    dc: &dyn DropCutter,
    centre: [f64; 2],
    r0: f64,
    r1: f64,
    pitch: f64,
    inward: bool,
    d: Drape,
    c: &Containment,
) -> Result<Vec<Pass>, DrapeError> {
    let s = pattern::spiral(centre, r0, r1, pitch, d.leave, inward);
    finish(dc, vec![s], d, c, None)
}

/// The heights at which the CL surface is flat over at least `min_nodes` nodes of the slope
/// map (slope ≤ `tan_flat`), lowest first; heights within `tol` are one level.
pub fn flat_levels(
    dc: &dyn DropCutter,
    sm: &SlopeMap,
    tan_flat: f64,
    tol: f64,
    min_nodes: usize,
) -> Vec<f64> {
    let mut zs: Vec<f64> = Vec::new();
    let mut j = 0;
    while j < sm.ny {
        let mut i = 0;
        while i < sm.nx {
            let t = sm.tan[j * sm.nx + i];
            if !t.is_nan() && t <= tan_flat {
                let x = sm.x0 + sm.g * i as f64;
                let y = sm.y0 + sm.g * j as f64;
                if let Some(b) = dc.drop(x, y) {
                    zs.push(b.hi);
                }
            }
            i += 1;
        }
        j += 1;
    }
    zs.sort_by(|a, b| a.total_cmp(b));
    let mut out = Vec::new();
    let mut k = 0;
    while k < zs.len() {
        let mut e = k;
        while e + 1 < zs.len() && zs[e + 1] - zs[k] <= tol {
            e += 1;
        }
        if e + 1 - k >= min_nodes {
            // the highest in the cluster: the tool at that height is above every CL in it
            out.push(zs[e]);
        }
        k = e + 1;
    }
    out
}

fn len2(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]) * (a[0] - b[0]) + (a[1] - b[1]) * (a[1] - b[1])).sqrt()
}

fn on_flat(dc: &dyn DropCutter, p: [f64; 2], z: f64, tol: f64) -> bool {
    match dc.drop(p[0], p[1]) {
        None => false,
        Some(b) => b.hi <= z && b.lo >= z - tol,
    }
}

/// Bisects between a point on the flat `a` and one off it `b` to within `tol`; the on-flat end.
fn edge(dc: &dyn DropCutter, mut a: [f64; 2], mut b: [f64; 2], z: f64, tol: f64) -> [f64; 2] {
    let mut k = 0;
    while k < 60 {
        if len2(a, b) <= tol {
            break;
        }
        let m = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
        if on_flat(dc, m, z, tol) {
            a = m;
        } else {
            b = m;
        }
        k += 1;
    }
    a
}

/// Flat-area passes: raster lines at each flat level, `stepover` apart at `angle_deg`, kept
/// where the tool stands on the flat; every chord proven free at the level's height.
pub fn flat(
    dc: &dyn DropCutter,
    levels: &[f64],
    angle_deg: f64,
    stepover: f64,
    step: f64,
    tol: f64,
    c: &Containment,
) -> Vec<Pass> {
    let [lo, hi] = dc.extent();
    let mut out = Vec::new();
    for &z in levels {
        for l in pattern::raster(lo, hi, angle_deg, stepover, true) {
            let (a, b) = (l[0], l[1]);
            let len = len2(a, b);
            let n = ((len / step).ceil() as usize).max(1);
            let mut cur: Pass = Vec::new();
            let mut prev: Option<([f64; 2], bool)> = None;
            let mut i = 0;
            while i <= n {
                let t = i as f64 / n as f64;
                let p = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                let on = on_flat(dc, p, z, tol);
                match prev {
                    Some((q, was)) if was != on => {
                        let e = if on {
                            edge(dc, p, q, z, tol)
                        } else {
                            edge(dc, q, p, z, tol)
                        };
                        if on {
                            cur.clear();
                            cur.push([e[0], e[1], z]);
                        } else {
                            let last = cur.last().copied();
                            if let Some(lq) = last
                                && dc.seg3_below(lq, [e[0], e[1], z], 0.5 * tol)
                            {
                                cur.push([e[0], e[1], z]);
                            }
                            if cur.len() > 1 {
                                out.push(core::mem::take(&mut cur));
                            }
                            cur.clear();
                        }
                    }
                    _ => {}
                }
                if on {
                    match cur.last().copied() {
                        Some(lq) if !dc.seg3_below(lq, [p[0], p[1], z], 0.5 * tol) => {
                            // something stands up between: end the run, start anew here
                            if cur.len() > 1 {
                                out.push(core::mem::take(&mut cur));
                            }
                            cur.clear();
                            cur.push([p[0], p[1], z]);
                        }
                        _ => cur.push([p[0], p[1], z]),
                    }
                }
                prev = Some((p, on));
                i += 1;
            }
            if cur.len() > 1 {
                out.push(cur);
            }
        }
    }
    c.clip(out, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop::Analytic;

    fn stepped() -> Analytic {
        // a 2 mm ball over a floor at z = 0 and a raised pad at z = 5 (the ball over a pad's
        // edge rolls round it), no curved faces
        Analytic {
            r: 2.0,
            planes: vec![
                (0.0, [-30.0, -30.0], [30.0, 30.0]),
                (5.0, [-10.0, -10.0], [10.0, 10.0]),
            ],
            spheres: vec![],
        }
    }

    #[test]
    fn flat_finds_both_levels_and_stays_on_them() {
        let dc = stepped();
        let sm = slope_map(&dc, 1.0);
        let lv = flat_levels(&dc, &sm, 1e-6, 1e-6, 4);
        assert_eq!(lv.len(), 2, "{lv:?}");
        assert!((lv[0] - 0.0).abs() < 1e-9 && (lv[1] - 5.0).abs() < 1e-9);
        let ps = flat(&dc, &lv, 0.0, 2.0, 0.5, 0.01, &Containment::default());
        assert!(!ps.is_empty());
        for p in &ps {
            let mut k = 0;
            while k + 1 < p.len() {
                assert!(dc.seg3_below(p[k], p[k + 1], 0.005));
                k += 1;
            }
            // passes at the pad's height stay over the pad (the tool's centre within the pad)
            if p[0][2] == 5.0 {
                for q in p {
                    assert!(q[0].abs() <= 10.0 + 1e-6 && q[1].abs() <= 10.0 + 1e-6, "{q:?}");
                }
            }
        }
    }

    #[test]
    fn spiral_and_radial_drape_on_the_dome() {
        let dc = Analytic {
            r: 3.0,
            planes: vec![(0.0, [-40.0, -40.0], [40.0, 40.0])],
            spheres: vec![([0.0, 0.0, -10.0], 30.0)],
        };
        let d = Drape {
            step: 2.0,
            slack: 0.005,
            leave: 0.01,
            zmin: 0.0,
        };
        // containment: the cap above z = 15 only
        let c = Containment {
            zmin: 15.0,
            ..Default::default()
        };
        let s = spiral(&dc, [0.0, 0.0], 0.0, 20.0, 1.0, false, d, &c).unwrap();
        assert!(!s.is_empty());
        for p in &s {
            for q in p {
                assert!(q[2] >= 15.0);
            }
        }
        let r = radial(&dc, [0.0, 0.0], 1.0, 20.0, 2.0, d, &c).unwrap();
        assert!(r.len() > 10);
    }
}

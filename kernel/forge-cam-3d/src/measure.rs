//! Measuring a finishing path: the cusp it leaves, how much of the region it covers, and its
//! length and time at feed. For the quality report and the benchmark harness
//! (verify/forge-cam-quality); a measurement on a grid, not a certificate (S3, the certified
//! maximum cusp from the swept tools, is the certificate to come).
//!
//! For a ball of radius r, the surface a set of tool positions leaves at (x, y) is the lower
//! envelope min over positions p of  p.z + r − √(r² − |(x, y) − p.xy|²). The *ideal* surface is
//! that envelope over every grid CL position (what the tool could leave with no stepover at all);
//! the path's is the envelope over its moves, sampled every g/4. Their difference at a node,
//! times cos θ of the ideal surface's slope there, is the cusp (measured normal to the surface,
//! as the scallop formulas are).

use crate::drape::Pass;
use crate::drop::DropCutter;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    /// Largest and 95th-percentile cusp over covered nodes (mm).
    pub max: f64,
    pub p95: f64,
    /// Covered nodes / nodes in the region (a node is covered when some tool position of the
    /// path reaches it).
    pub covered: f64,
    /// Cutting length (mm).
    pub length: f64,
}

/// The cusp of `passes` (tool-tip polylines of a ball of radius `r`) on the grid of spacing `g`
/// over the box [lo, hi], against the ideal surface from the drop-cutter `dc`.
pub fn cusp(dc: &dyn DropCutter, r: f64, passes: &[Pass], lo: [f64; 2], hi: [f64; 2], g: f64) -> Stats {
    let nx = (((hi[0] - lo[0]) / g).ceil() as usize) + 1;
    let ny = (((hi[1] - lo[1]) / g).ceil() as usize) + 1;
    let k = (r / g).floor() as i64;
    let splat = |s: &mut Vec<f64>, p: [f64; 3]| {
        let ci = ((p[0] - lo[0]) / g).round() as i64;
        let cj = ((p[1] - lo[1]) / g).round() as i64;
        let mut dj = -k - 1;
        while dj <= k + 1 {
            let mut di = -k - 1;
            while di <= k + 1 {
                let (a, b) = (ci + di, cj + dj);
                if a >= 0 && b >= 0 && a < nx as i64 && b < ny as i64 {
                    let x = lo[0] + g * a as f64;
                    let y = lo[1] + g * b as f64;
                    let d2 = (x - p[0]) * (x - p[0]) + (y - p[1]) * (y - p[1]);
                    if d2 <= r * r {
                        let h = p[2] + r - (r * r - d2).sqrt();
                        let q = (b as usize) * nx + a as usize;
                        if h < s[q] {
                            s[q] = h;
                        }
                    }
                }
                di += 1;
            }
            dj += 1;
        }
    };
    // ideal: every grid position
    let mut ideal = vec![f64::INFINITY; nx * ny];
    let mut q = 0;
    while q < nx * ny {
        let x = lo[0] + g * (q % nx) as f64;
        let y = lo[1] + g * (q / nx) as f64;
        if let Some(b) = dc.drop(x, y) {
            splat(&mut ideal, [x, y, b.hi]);
        }
        q += 1;
    }
    // the path, sampled every g/4
    let mut got = vec![f64::INFINITY; nx * ny];
    let mut length = 0.0;
    for p in passes {
        let mut m = 0;
        while m + 1 < p.len() {
            let (a, b) = (p[m], p[m + 1]);
            let l = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
            length += l;
            let n = ((l / (0.25 * g)).ceil() as usize).max(1);
            let mut i = 0;
            while i <= n {
                let t = i as f64 / n as f64;
                splat(
                    &mut got,
                    [
                        a[0] + (b[0] - a[0]) * t,
                        a[1] + (b[1] - a[1]) * t,
                        a[2] + (b[2] - a[2]) * t,
                    ],
                );
                i += 1;
            }
            m += 1;
        }
    }
    // the cusp normal to the surface: the vertical difference times cos θ of the ideal
    // surface's slope there (central differences; one-sided at the region's edge)
    let slope_cos = |q: usize| -> f64 {
        let (i, j) = (q % nx, q / nx);
        let at = |a: usize, b: usize| ideal[b * nx + a];
        let dx = if i > 0 && i + 1 < nx && at(i - 1, j).is_finite() && at(i + 1, j).is_finite() {
            (at(i + 1, j) - at(i - 1, j)) / (2.0 * g)
        } else {
            0.0
        };
        let dy = if j > 0 && j + 1 < ny && at(i, j - 1).is_finite() && at(i, j + 1).is_finite() {
            (at(i, j + 1) - at(i, j - 1)) / (2.0 * g)
        } else {
            0.0
        };
        1.0 / (1.0 + dx * dx + dy * dy).sqrt()
    };
    let mut cusps = Vec::new();
    let mut region = 0usize;
    let mut q = 0;
    while q < nx * ny {
        if ideal[q].is_finite() {
            region += 1;
            if got[q].is_finite() {
                cusps.push((got[q] - ideal[q]).max(0.0) * slope_cos(q));
            }
        }
        q += 1;
    }
    cusps.sort_by(|a, b| a.total_cmp(b));
    let max = cusps.last().copied().unwrap_or(0.0);
    let p95 = if cusps.is_empty() {
        0.0
    } else {
        cusps[((cusps.len() - 1) as f64 * 0.95) as usize]
    };
    Stats {
        max,
        p95,
        covered: if region == 0 {
            0.0
        } else {
            cusps.len() as f64 / region as f64
        },
        length,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drape::{Drape, drape};
    use crate::drop::Analytic;
    use crate::pattern;
    use crate::scallop::{Contact, cusp as cusp_of, step_for};
    use forge_cam_geom::ival::Ival;

    #[test]
    fn parallel_on_a_floor_meets_the_formula() {
        // a 6 mm ball on a flat floor, raster at the stepover for 0.01 mm cusp: the measured
        // cusp is the formula's (the grid's sampling below it)
        let dc = Analytic {
            r: 3.0,
            planes: vec![(0.0, [-20.0, -20.0], [20.0, 20.0])],
            spheres: vec![],
        };
        let s = step_for(Contact::Ball { r: 3.0 }, 0.01, 6.0).unwrap();
        let want = cusp_of(Contact::Ball { r: 3.0 }, Ival::pt(s)).hi;
        let d = Drape {
            step: 2.0,
            slack: 0.001,
            leave: 0.001,
            zmin: f64::NEG_INFINITY,
        };
        let mut ps = Vec::new();
        for l in pattern::raster([-10.0, -10.0], [10.0, 10.0], 0.0, s, true) {
            ps.extend(drape(&dc, &l, d).unwrap());
        }
        let st = cusp(&dc, 3.0, &ps, [-3.0, -3.0], [3.0, 3.0], 0.04);
        assert!(st.max <= want + 1e-6, "{st:?} want {want}");
        assert!(st.max >= 0.8 * want, "{st:?} want {want}");
        assert!(st.covered > 0.99);
    }

    #[test]
    fn steep_and_shallow_holds_the_cusp_target_across_the_threshold() {
        use crate::scallop::{raster_step, waterline_step};
        use crate::steep;
        let dc = Analytic {
            r: 3.0,
            planes: vec![(0.0, [-40.0, -40.0], [40.0, 40.0])],
            spheres: vec![([0.0, 0.0, -10.0], 30.0)],
        };
        let h = 0.02;
        let d = step_for(Contact::Ball { r: 3.0 }, h, 6.0).unwrap();
        let s45 = std::f64::consts::FRAC_1_SQRT_2;
        let e = Ival::new(s45.next_down(), s45.next_up());
        let p = steep::Params {
            sin0: s45,
            cos0: s45,
            overlap: 1.0,
            dz: waterline_step(d, e),
            stepover: raster_step(d, e),
            angle_deg: 0.0,
            top: 20.0,
            bottom: 0.5,
            grid: 1.0,
            tol: 0.002,
        };
        let (st, sh) = steep::steep_and_shallow(&dc, p).unwrap();
        let mut all = st;
        all.extend(sh);
        // a strip across the 45° circle (xy radius 23.3) on the dome's flank
        let m = cusp(&dc, 3.0, &all, [19.0, -2.0], [27.0, 2.0], 0.05);
        // convex section radius 30 against the tool's 3: the cusp grows by about (30 + 3)/30
        assert!(m.max <= 1.1 * h + 0.003, "{m:?}");
        assert!(m.covered > 0.99, "{m:?}");
    }
}

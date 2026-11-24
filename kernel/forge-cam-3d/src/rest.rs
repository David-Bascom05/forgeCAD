//! Rest finishing: finish only where an earlier, larger tool (the reference) left material.
//!
//! Method (plain, published: reference-tool rest machining). For a ball of radius R whose CL
//! height on the grid is c(u, v), the surface it leaves at (x, y) is the lower envelope of its
//! positions:  S(x, y) = min over (u, v) within R of  c(u, v) + R − √(R² − |(x, y) − (u, v)|²).
//! The rest region is where S_ref − S_cur exceeds `tol` over the part's footprint: material the reference tool could not
//! reach and the current one can. It is grown by `margin` and finished with the current tool by
//! parallel passes (draped, so proven like any other) clipped to it.
//!
//! The envelope is a measurement on the grid (the minimum over grid positions only, so it may
//! overstate S, more on steep slopes), used to choose *where* to cut, never for safety. Give a
//! `tol` well above that error (e.g. 0.05 mm at g = 0.25 mm with a 2 mm ball: at tol 0.01 and
//! g = 0.5 the dome's flank shows false rest).

use crate::contain::Containment;
use crate::drape::{Drape, DrapeError, Pass};
use crate::drop::DropCutter;
use crate::pattern;
use crate::steep::Mask;

fn cl_grid(dc: &dyn DropCutter, x0: f64, y0: f64, g: f64, nx: usize, ny: usize) -> Vec<f64> {
    let mut z = vec![f64::NAN; nx * ny];
    let mut k = 0;
    while k < z.len() {
        if let Some(b) = dc.drop(x0 + g * (k % nx) as f64, y0 + g * (k / nx) as f64) {
            z[k] = b.hi;
        }
        k += 1;
    }
    z
}

/// The lower envelope of a ball of radius r at every grid CL height.
fn envelope(c: &[f64], nx: usize, ny: usize, g: f64, r: f64) -> Vec<f64> {
    let k = (r / g).floor() as i64;
    let mut s = vec![f64::INFINITY; nx * ny];
    let mut j = 0i64;
    while j < ny as i64 {
        let mut i = 0i64;
        while i < nx as i64 {
            let cz = c[(j as usize) * nx + i as usize];
            if !cz.is_nan() {
                let mut dj = -k;
                while dj <= k {
                    let mut di = -k;
                    while di <= k {
                        let (a, b) = (i + di, j + dj);
                        let d2 = g * g * ((di * di + dj * dj) as f64);
                        if a >= 0 && b >= 0 && a < nx as i64 && b < ny as i64 && d2 <= r * r {
                            let h = cz + r - (r * r - d2).sqrt();
                            let q = (b as usize) * nx + a as usize;
                            if h < s[q] {
                                s[q] = h;
                            }
                        }
                        di += 1;
                    }
                    dj += 1;
                }
            }
            i += 1;
        }
        j += 1;
    }
    s
}

/// The rest region on the grid of `g` over the current tool's extent: where the reference ball
/// (radius r_ref, drop-cutter `refc`) leaves more than `tol` above what the current ball
/// (radius r_cur, `cur`) leaves.
pub fn region(cur: &dyn DropCutter, r_cur: f64, refc: &dyn DropCutter, r_ref: f64, g: f64, tol: f64) -> Mask {
    let [lo, hi] = cur.extent();
    let nx = (((hi[0] - lo[0]) / g).ceil() as usize) + 1;
    let ny = (((hi[1] - lo[1]) / g).ceil() as usize) + 1;
    let s_cur = envelope(&cl_grid(cur, lo[0], lo[1], g, nx, ny), nx, ny, g, r_cur);
    let s_ref = envelope(&cl_grid(refc, lo[0], lo[1], g, nx, ny), nx, ny, g, r_ref);
    let mut on = vec![false; nx * ny];
    let mut k = 0;
    while k < on.len() {
        // only over the part's own footprint (the extent less the tool's radius): beyond its
        // silhouette both balls roll over the outer edge into air, which is not rest material
        let x = lo[0] + g * (k % nx) as f64;
        let y = lo[1] + g * (k / nx) as f64;
        let foot = x >= lo[0] + r_cur && x <= hi[0] - r_cur && y >= lo[1] + r_cur && y <= hi[1] - r_cur;
        on[k] = foot && s_cur[k].is_finite() && s_ref[k].is_finite() && s_ref[k] - s_cur[k] > tol;
        k += 1;
    }
    Mask {
        x0: lo[0],
        y0: lo[1],
        g,
        nx,
        ny,
        on,
    }
}

/// Rest passes: parallel passes of the current tool, clipped to the rest region grown by
/// `margin` and to the containment.
#[allow(clippy::too_many_arguments)]
pub fn rest(
    cur: &dyn DropCutter,
    rest: &Mask,
    margin: f64,
    angle_deg: f64,
    stepover: f64,
    d: Drape,
    c: &Containment,
) -> Result<Vec<Pass>, DrapeError> {
    let m = rest.dilate(margin);
    let [lo, hi] = cur.extent();
    let mut ps = Vec::new();
    for l in pattern::raster(lo, hi, angle_deg, stepover, true) {
        for p in crate::drape::drape(cur, &l, d)? {
            ps.extend(crate::steep::clip_open(&p, &m));
        }
    }
    Ok(c.clip(ps, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop::Analytic;

    fn part(r: f64) -> Analytic {
        Analytic {
            r,
            planes: vec![(0.0, [-40.0, -40.0], [40.0, 40.0])],
            spheres: vec![([0.0, 0.0, -10.0], 30.0)],
        }
    }

    #[test]
    fn rest_is_the_foot_of_the_dome() {
        // a 12 mm ball before a 4 mm one: material is left in the concave corner at the dome's
        // foot (between about r = 26.9 on the sphere and 32.25 on the floor), nowhere else
        let (cur, refc) = (part(2.0), part(6.0));
        let m = region(&cur, 2.0, &refc, 6.0, 0.25, 0.05);
        let mut n = 0;
        let mut j = 0;
        while j < m.ny {
            let mut i = 0;
            while i < m.nx {
                if m.on[j * m.nx + i] {
                    let x = m.x0 + m.g * i as f64;
                    let y = m.y0 + m.g * j as f64;
                    let r = (x * x + y * y).sqrt();
                    // the 6 mm ball touches the sphere at r ≈ 26.9 and the floor beyond √(36² − 16²) ≈ 32.25
                    assert!(r > 26.0 && r < 33.0, "rest at r={r}");
                    n += 1;
                }
                i += 1;
            }
            j += 1;
        }
        assert!(n > 50, "{n}");
        let d = Drape {
            step: 1.0,
            slack: 0.005,
            leave: 0.01,
            zmin: 0.0,
        };
        let ps = rest(&cur, &m, 1.0, 0.0, 0.5, d, &Containment::default()).unwrap();
        assert!(!ps.is_empty());
    }
}

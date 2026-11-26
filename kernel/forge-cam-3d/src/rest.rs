//! Rest finishing: finish only where an earlier, larger tool (the reference) left material.
//!
//! Method (plain, published: reference-tool rest machining). For a cutter of radius R and lower
//! profile b (`Profile`: a ball's b(ρ) = R − √(R² − ρ²), any cutter's from CAM-3AXIS's column)
//! whose CL height on the grid is c(u, v), the surface it leaves at (x, y) is the lower envelope
//! of its positions:  S(x, y) = min over (u, v) within R of  c(u, v) + b(|(x, y) − (u, v)|).
//! The rest region is where S_ref − S_cur exceeds `tol` over the part's footprint: material the
//! reference tool could not reach and the current one can. It is grown by `margin` and finished with the current tool by
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

/// A cutter's lower profile: b(ρ), the height of its lowest point at radius ρ above its tip, for
/// ρ in [0, radius].
pub trait Profile {
    fn radius(&self) -> f64;
    fn b(&self, rho: f64) -> f64;
}

/// A ball of radius r: b(ρ) = r − √(r² − ρ²).
pub struct Ball(pub f64);

impl Profile for Ball {
    fn radius(&self) -> f64 {
        self.0
    }
    fn b(&self, rho: f64) -> f64 {
        let r = self.0;
        r - (r * r - rho * rho).max(0.0).sqrt()
    }
}

/// Any cutter by CAM-3AXIS's column (flat, ball, bull, chamfer, drill ends).
impl Profile for forge_cam_geom::drop::Column {
    fn radius(&self) -> f64 {
        self.r
    }
    fn b(&self, rho: f64) -> f64 {
        self.b_at(rho.min(self.r)).mid()
    }
}

fn cl_grid(dc: &dyn DropCutter, x0: f64, y0: f64, g: f64, nx: usize, ny: usize) -> Vec<f64> {
    // the rows on all cores (forge_math::par), joined in order: each height is its node's drop
    let rows = forge_math::par::map(ny, |j| {
        let mut z = vec![f64::NAN; nx];
        let mut i = 0;
        while i < nx {
            let k = j * nx + i;
            if let Some(b) = dc.drop(x0 + g * (k % nx) as f64, y0 + g * (k / nx) as f64) {
                z[i] = b.hi;
            }
            i += 1;
        }
        z
    });
    rows.concat()
}

/// The lower envelope of the cutter (profile `pr`) at every grid CL height.
fn envelope(c: &[f64], nx: usize, ny: usize, g: f64, pr: &dyn Profile) -> Vec<f64> {
    let r = pr.radius();
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
                            let h = cz + pr.b(d2.sqrt());
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
pub fn region(
    cur: &dyn DropCutter,
    p_cur: &dyn Profile,
    refc: &dyn DropCutter,
    p_ref: &dyn Profile,
    g: f64,
    tol: f64,
) -> Mask {
    region_in(cur, p_cur, refc, p_ref, g, tol, None)
}

/// `region`, kept also to where the stock as the earlier ops left it (`stock(x, y)`: its top at
/// the grid node (x, y), as a model swept by their paths makes it) stands more than `tol` above
/// what the current tool leaves: material the earlier tools left in fact, not only in
/// principle. A choice of where to cut, like the envelopes: never for safety.
#[allow(clippy::too_many_arguments)]
pub fn region_in(
    cur: &dyn DropCutter,
    p_cur: &dyn Profile,
    refc: &dyn DropCutter,
    p_ref: &dyn Profile,
    g: f64,
    tol: f64,
    stock: Option<&dyn Fn(f64, f64) -> f64>,
) -> Mask {
    let r_cur = p_cur.radius();
    let [lo, hi] = cur.extent();
    let nx = (((hi[0] - lo[0]) / g).ceil() as usize) + 1;
    let ny = (((hi[1] - lo[1]) / g).ceil() as usize) + 1;
    let s_cur = envelope(&cl_grid(cur, lo[0], lo[1], g, nx, ny), nx, ny, g, p_cur);
    let s_ref = envelope(&cl_grid(refc, lo[0], lo[1], g, nx, ny), nx, ny, g, p_ref);
    let mut on = vec![false; nx * ny];
    let mut k = 0;
    while k < on.len() {
        // only over the part's own footprint (the extent less the tool's radius): beyond its
        // silhouette both balls roll over the outer edge into air, which is not rest material
        let x = lo[0] + g * (k % nx) as f64;
        let y = lo[1] + g * (k / nx) as f64;
        let foot = x >= lo[0] + r_cur && x <= hi[0] - r_cur && y >= lo[1] + r_cur && y <= hi[1] - r_cur;
        on[k] = foot
            && s_cur[k].is_finite()
            && s_ref[k].is_finite()
            && s_ref[k] - s_cur[k] > tol
            && stock.is_none_or(|st| st(x, y) - s_cur[k] > tol);
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
    let lines = pattern::raster(lo, hi, angle_deg, stepover, true);
    for passes in crate::drape::drape_all_in(cur, &lines, d, Some(&m))? {
        for p in passes {
            ps.extend(crate::steep::clip_open(&p, &m));
        }
    }
    Ok(c.clip(ps, None))
}

/// Each pass in order cut down to its run from the first to the last of its segments that
/// `meets` (does the tool, from a to b, meet material?), a pass that meets none dropped: the
/// draped points themselves, so each kept segment is one the pass had. `meets` is asked of
/// every segment of every pass in turn (a model of the stock may take each as cut).
pub fn trim(ps: Vec<Pass>, meets: &mut dyn FnMut([f64; 3], [f64; 3]) -> bool) -> Vec<Pass> {
    let mut out = Vec::with_capacity(ps.len());
    for p in ps {
        let hit: Vec<bool> = p.windows(2).map(|w| meets(w[0], w[1])).collect();
        if let (Some(i), Some(j)) = (hit.iter().position(|h| *h), hit.iter().rposition(|h| *h)) {
            out.push(p[i..=j + 1].to_vec());
        }
    }
    out
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
        let m = region(&cur, &Ball(2.0), &refc, &Ball(6.0), 0.25, 0.05);
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

    #[test]
    fn rest_is_only_where_the_stock_still_stands() {
        // the same corner, but the earlier ops left stock only on the half x > 0 (finished,
        // 0.01 over, elsewhere): the region is the foot's half on that side
        let (cur, refc) = (part(2.0), part(6.0));
        let all = region(&cur, &Ball(2.0), &refc, &Ball(6.0), 0.25, 0.05);
        let stock = |x: f64, _y: f64| if x > 0.0 { 100.0 } else { f64::NEG_INFINITY };
        let m = region_in(&cur, &Ball(2.0), &refc, &Ball(6.0), 0.25, 0.05, Some(&stock));
        let (mut n, mut n_all) = (0, 0);
        for k in 0..m.on.len() {
            let x = m.x0 + m.g * (k % m.nx) as f64;
            n_all += all.on[k] as usize;
            if m.on[k] {
                assert!(all.on[k] && x > 0.0, "rest at x={x}");
                n += 1;
            }
            if all.on[k] && x > 0.0 {
                assert!(m.on[k]);
            }
        }
        assert!(n > 20 && n < n_all, "{n} of {n_all}");
        // no stock above the finish anywhere: no region
        let none = region_in(
            &cur,
            &Ball(2.0),
            &refc,
            &Ball(6.0),
            0.25,
            0.05,
            Some(&|_, _| -1.0),
        );
        assert!(!none.on.iter().any(|x| *x));
    }

    #[test]
    fn a_pass_is_trimmed_to_where_it_meets_material() {
        let p: Pass = (0..10).map(|i| [i as f64, 0.0, 0.0]).collect();
        let q: Pass = (0..4).map(|i| [i as f64, 5.0, 0.0]).collect();
        // material from x 3 to 6 on the first pass only
        let mut asked = 0;
        let out = trim(vec![p.clone(), q], &mut |a, b| {
            asked += 1;
            a[1] == 0.0 && b[0] > 3.0 && a[0] < 6.0
        });
        assert_eq!(asked, 9 + 3);
        assert_eq!(out, vec![p[3..=6].to_vec()]);
    }
}

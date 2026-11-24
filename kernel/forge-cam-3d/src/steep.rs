//! Steep and shallow finishing: waterline passes where the surface is steep, parallel passes
//! where it is shallow, each extended over the other by an overlap distance.
//!
//! Method (plain, published; Fusion's Steep and Shallow is the reference behaviour, research
//! surface-finish.md §2.1). The threshold θ0 splits the part by the CL surface's slope. With
//! target cusp h the spacing along the surface is d = `step_for(h)`, and
//!   waterline step-down d_z = d·sin θ0  (on slopes ≥ θ0, spacing d_z/sin θ ≤ d),
//!   raster stepover   s_xy = d·cos θ0  (on slopes ≤ θ0, spacing s_xy/cos θ ≤ d),
//! so on planar pieces the cusp is at most h on both sides of the threshold (the scallop
//! module's formulas; curvature changes it to second order, and S3, the cusp measured on the
//! swept tools, is the certificate).
//!
//! The slope here is the CL surface's, by central differences of the drop-cutter on a grid
//! (for a ball it is the part's slope at the contact). It decides only *which* strategy covers a
//! place, never safety: every pass is already proven free of gouges where it was generated, and
//! clipping a pass at its vertices keeps that.

use crate::drape::{self, Drape, Pass};
use crate::drop::DropCutter;
use crate::pattern;
use crate::waterline::{self, Loop};

/// The CL slope (tan θ) on a grid; NaN where nothing is under the tool.
#[derive(Clone, Debug)]
pub struct SlopeMap {
    pub x0: f64,
    pub y0: f64,
    pub g: f64,
    pub nx: usize,
    pub ny: usize,
    pub tan: Vec<f64>,
}

/// A boolean grid with the same layout.
#[derive(Clone, Debug)]
pub struct Mask {
    pub x0: f64,
    pub y0: f64,
    pub g: f64,
    pub nx: usize,
    pub ny: usize,
    pub on: Vec<bool>,
}

pub fn slope_map(dc: &dyn DropCutter, g: f64) -> SlopeMap {
    let [lo, hi] = dc.extent();
    let nx = (((hi[0] - lo[0]) / g).ceil() as usize) + 1;
    let ny = (((hi[1] - lo[1]) / g).ceil() as usize) + 1;
    let h = 0.25 * g;
    let mut tan = vec![f64::NAN; nx * ny];
    let mut j = 0;
    while j < ny {
        let mut i = 0;
        while i < nx {
            let x = lo[0] + g * i as f64;
            let y = lo[1] + g * j as f64;
            let z = |px: f64, py: f64| dc.drop(px, py).map(|b| 0.5 * (b.lo + b.hi));
            if let (Some(a), Some(b), Some(c), Some(d)) = (z(x - h, y), z(x + h, y), z(x, y - h), z(x, y + h))
            {
                let gx = (b - a) / (2.0 * h);
                let gy = (d - c) / (2.0 * h);
                tan[j * nx + i] = (gx * gx + gy * gy).sqrt();
            }
            i += 1;
        }
        j += 1;
    }
    SlopeMap {
        x0: lo[0],
        y0: lo[1],
        g,
        nx,
        ny,
        tan,
    }
}

impl SlopeMap {
    /// Nodes at least as steep as tan θ0 (`steep`) or no steeper (`!steep`); nodes with nothing
    /// under the tool are in neither.
    pub fn mask(&self, tan0: f64, steep: bool) -> Mask {
        let mut on = vec![false; self.tan.len()];
        let mut k = 0;
        while k < on.len() {
            let t = self.tan[k];
            if !t.is_nan() {
                on[k] = if steep { t >= tan0 } else { t <= tan0 };
            }
            k += 1;
        }
        Mask {
            x0: self.x0,
            y0: self.y0,
            g: self.g,
            nx: self.nx,
            ny: self.ny,
            on,
        }
    }
}

impl Mask {
    /// Grown by distance `r` (a disc of nodes).
    pub fn dilate(&self, r: f64) -> Mask {
        let k = (r / self.g).ceil() as i64;
        let mut on = vec![false; self.on.len()];
        let (nx, ny) = (self.nx as i64, self.ny as i64);
        let mut j = 0;
        while j < ny {
            let mut i = 0;
            while i < nx {
                if self.on[(j * nx + i) as usize] {
                    let mut dj = -k;
                    while dj <= k {
                        let mut di = -k;
                        while di <= k {
                            let (a, b) = (i + di, j + dj);
                            let dd = ((di * di + dj * dj) as f64).sqrt() * self.g;
                            if a >= 0 && b >= 0 && a < nx && b < ny && dd <= r + 1e-12 {
                                on[(b * nx + a) as usize] = true;
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
        Mask { on, ..self.clone() }
    }

    /// The value at the node nearest p (false outside the grid).
    pub fn at(&self, p: [f64; 2]) -> bool {
        let i = ((p[0] - self.x0) / self.g).round();
        let j = ((p[1] - self.y0) / self.g).round();
        if i < 0.0 || j < 0.0 || i as usize >= self.nx || j as usize >= self.ny {
            return false;
        }
        self.on[j as usize * self.nx + i as usize]
    }
}

/// The runs of an open pass whose vertices are in the mask (cut at vertices: each kept chord
/// was proven where it was generated).
pub fn clip_open(p: &Pass, m: &Mask) -> Vec<Pass> {
    let mut out = Vec::new();
    let mut cur: Pass = Vec::new();
    for q in p {
        if m.at([q[0], q[1]]) {
            cur.push(*q);
        } else {
            if cur.len() > 1 {
                out.push(core::mem::take(&mut cur));
            }
            cur.clear();
        }
    }
    if cur.len() > 1 {
        out.push(cur);
    }
    out
}

/// The runs of a closed waterline loop in the mask, as open passes (a loop wholly inside comes
/// back closed: its first point repeated at the end).
pub fn clip_loop(l: &Loop, m: &Mask) -> Vec<Pass> {
    let n = l.pts.len();
    let inside: Vec<bool> = l.pts.iter().map(|q| m.at(*q)).collect();
    let pt = |k: usize| [l.pts[k % n][0], l.pts[k % n][1], l.z];
    if n == 0 {
        return Vec::new();
    }
    let mut start = None;
    let mut k = 0;
    while k < n {
        if !inside[k] {
            start = Some(k);
            break;
        }
        k += 1;
    }
    let Some(s) = start else {
        let mut v: Pass = (0..=n).map(pt).collect();
        v.truncate(n + 1);
        return vec![v];
    };
    // walk once round from an outside vertex
    let mut out = Vec::new();
    let mut cur: Pass = Vec::new();
    let mut i = 1;
    while i <= n {
        let k = (s + i) % n;
        if inside[k] {
            cur.push(pt(k));
        } else {
            if cur.len() > 1 {
                out.push(core::mem::take(&mut cur));
            }
            cur.clear();
        }
        i += 1;
    }
    if cur.len() > 1 {
        out.push(cur);
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// The threshold slope θ0, as (sin θ0, cos θ0) (exact reals given by the caller).
    pub sin0: f64,
    pub cos0: f64,
    /// How far each side extends over the other.
    pub overlap: f64,
    /// Waterline step-down and raster stepover (from the scallop module, see the module doc).
    pub dz: f64,
    pub stepover: f64,
    /// Raster direction, degrees from +x.
    pub angle_deg: f64,
    /// Heights the waterlines span.
    pub top: f64,
    pub bottom: f64,
    /// Grid for the slope map and the waterline sampling.
    pub grid: f64,
    /// Tolerances: waterline xy, drape slack and leave.
    pub tol: f64,
    /// The xy box the parallel passes span (None: the part's extent).
    pub area: Option<[[f64; 2]; 2]>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SteepError {
    Bad(&'static str),
    Waterline(waterline::WaterlineError),
    Drape(drape::DrapeError),
}

/// The steep passes (waterline, top down) and the shallow passes (parallel), in that order.
pub fn steep_and_shallow(dc: &dyn DropCutter, p: Params) -> Result<(Vec<Pass>, Vec<Pass>), SteepError> {
    if !(p.cos0 > 0.0
        && p.sin0 >= 0.0
        && p.overlap >= 0.0
        && p.dz > 0.0
        && p.stepover > 0.0
        && p.grid > 0.0
        && p.tol > 0.0)
    {
        return Err(SteepError::Bad("params"));
    }
    let sm = slope_map(dc, p.grid);
    let tan0 = p.sin0 / p.cos0;
    let steep = sm.mask(tan0, true).dilate(p.overlap);
    let shallow = sm.mask(tan0, false).dilate(p.overlap);
    // one CL grid for every level, the levels on all cores (in order)
    let zs = waterline::levels(p.top, p.bottom, p.dz);
    let all = waterline::levels_on(
        dc,
        &zs,
        waterline::Params {
            grid: p.grid,
            tol_xy: p.tol,
        },
    )
    .map_err(SteepError::Waterline)?;
    let mut sp = Vec::new();
    for ls in &all {
        for l in ls {
            sp.extend(clip_loop(l, &steep));
        }
    }
    let [lo, hi] = p.area.unwrap_or(dc.extent());
    let d = Drape {
        step: p.grid,
        slack: p.tol,
        leave: p.tol,
        zmin: p.bottom,
    };
    let mut hp = Vec::new();
    let lines = pattern::raster(lo, hi, p.angle_deg, p.stepover, true);
    for passes in drape::drape_all(dc, &lines, d).map_err(SteepError::Drape)? {
        for pass in passes {
            hp.extend(clip_open(&pass, &shallow));
        }
    }
    Ok((sp, hp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drop::Analytic;

    #[test]
    fn dome_splits_at_the_threshold() {
        let dc = Analytic {
            r: 3.0,
            planes: vec![(0.0, [-40.0, -40.0], [40.0, 40.0])],
            spheres: vec![([0.0, 0.0, -10.0], 30.0)],
        };
        let h = std::f64::consts::FRAC_1_SQRT_2;
        let p = Params {
            sin0: h,
            cos0: h,
            overlap: 1.0,
            dz: 2.0,
            stepover: 2.0,
            angle_deg: 0.0,
            top: 20.0,
            bottom: 0.0,
            grid: 1.0,
            tol: 0.01,
            area: None,
        };
        let (steep, shallow) = steep_and_shallow(&dc, p).unwrap();
        assert!(!steep.is_empty() && !shallow.is_empty());
        // the CL sphere (radius 33 about z = −10) is 45° steep where the xy radius is 33/√2 ≈ 23.3
        let r45 = 33.0 * h;
        for s in &steep {
            for q in s {
                let r = (q[0] * q[0] + q[1] * q[1]).sqrt();
                assert!(r >= r45 - 1.0 - 1.5, "steep at r={r}");
            }
        }
        for s in &shallow {
            for q in s {
                let r = (q[0] * q[0] + q[1] * q[1]).sqrt();
                // shallow: the cap inside r45, or the floor outside the dome's foot (r ≥ ~30.3)
                assert!(r <= r45 + 1.0 + 1.5 || r >= 30.33 - 1.0 - 1.5, "shallow at r={r}");
            }
        }
    }
}

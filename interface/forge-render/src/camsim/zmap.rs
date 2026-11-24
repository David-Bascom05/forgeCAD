//! The stock for display: a grid of vertical dexels, one interval of material [lo, hi] per
//! column, cut by the tool's lower envelope swept along each piece of motion. Display only: it
//! never decides anything (the verdicts are the certified checks'), and its error is the cell
//! size, which the panel states.
//!
//! A piece at one height (lines and arcs in a plane, which is most of 2.5D) is swept exactly per
//! column: the column's distance to the piece in xy, then the tool's envelope at that distance.
//! A piece whose height changes is stamped at steps of half a cell. One interval per column lets
//! a setup turned over continue from the last one's stock (through holes stay holes).

use super::timeline::Seg;
use forge_cam_geom::tool::{PSeg, Part, Shape};
use forge_math::dmath;

/// The cutter's lower envelope: the lowest height of its cutting body above the tip at each
/// radius, sampled every `dr`, up to `r`.
#[derive(Clone, Debug, PartialEq)]
pub struct Cutter {
    pub r: f64,
    dr: f64,
    h: Vec<f32>,
}

impl Cutter {
    /// A flat end mill of radius r.
    pub fn flat(r: f64) -> Cutter {
        Cutter {
            r,
            dr: r,
            h: vec![0.0, 0.0],
        }
    }

    /// A ball end mill of radius r.
    pub fn ball(r: f64) -> Cutter {
        let n = 64;
        let dr = r / n as f64;
        let h = (0..=n)
            .map(|k| {
                let p = k as f64 * dr;
                (r - (r * r - p * p).max(0.0).sqrt()) as f32
            })
            .collect();
        Cutter { r, dr, h }
    }

    /// From the tool's revolved profile: its cutting bodies' lowest point at each radius.
    pub fn of(shape: &Shape) -> Option<Cutter> {
        Cutter::of_part(shape, Part::Cutter)
    }

    /// The lowest height above the tip, at each radius, of the tool's bodies of one kind (its
    /// shank and holder: `Part::NonCutting`).
    pub fn of_part(shape: &Shape, part: Part) -> Option<Cutter> {
        let mut pts: Vec<([f64; 2], [f64; 2])> = vec![];
        let mut r = 0.0f64;
        for b in shape.bodies.iter().filter(|b| b.part == part) {
            for s in &b.chain {
                match *s {
                    PSeg::Line { a, b } => pts.push((a, b)),
                    PSeg::Arc { a, b, c, r: rr, ccw } => {
                        // as a fine polyline
                        let a0 = dmath::atan2(a[1] - c[1], a[0] - c[0]);
                        let mut a1 = dmath::atan2(b[1] - c[1], b[0] - c[0]);
                        let tau = std::f64::consts::TAU;
                        if ccw {
                            while a1 < a0 {
                                a1 += tau;
                            }
                        } else {
                            while a1 > a0 {
                                a1 -= tau;
                            }
                        }
                        let n = 24;
                        let mut prev = a;
                        for k in 1..=n {
                            let t = a0 + (a1 - a0) * k as f64 / n as f64;
                            let q = if k == n {
                                b
                            } else {
                                [c[0] + rr * dmath::cos(t), c[1] + rr * dmath::sin(t)]
                            };
                            pts.push((prev, q));
                            prev = q;
                        }
                    }
                }
            }
            r = r.max(b.rmax);
        }
        if pts.is_empty() || r <= 0.0 {
            return None;
        }
        let n = 128;
        let dr = r / n as f64;
        let mut h = vec![f32::INFINITY; n + 1];
        for (k, hk) in h.iter_mut().enumerate() {
            let p = (k as f64 * dr).min(r * (1.0 - 1e-9));
            for &(a, b) in &pts {
                let (lo, hi) = (a[0].min(b[0]), a[0].max(b[0]));
                if p < lo || p > hi {
                    continue;
                }
                let z = if (b[0] - a[0]).abs() < 1e-12 {
                    a[1].min(b[1])
                } else {
                    a[1] + (b[1] - a[1]) * (p - a[0]) / (b[0] - a[0])
                };
                *hk = hk.min(z as f32);
            }
        }
        // a radius the profile skipped (rounding at a vertical piece): its neighbour's height
        for k in 1..h.len() {
            if !h[k].is_finite() {
                h[k] = h[k - 1];
            }
        }
        if !h[0].is_finite() {
            h[0] = h.iter().copied().find(|x| x.is_finite()).unwrap_or(0.0);
        }
        Some(Cutter { r, dr, h })
    }

    /// The envelope's height above the tip at radius d (None beyond the cutter).
    pub fn at(&self, d: f64) -> Option<f32> {
        if d > self.r {
            return None;
        }
        let x = d / self.dr;
        let k = (x as usize).min(self.h.len() - 1);
        let k1 = (k + 1).min(self.h.len() - 1);
        let f = (x - k as f64).clamp(0.0, 1.0) as f32;
        Some(self.h[k] * (1.0 - f) + self.h[k1] * f)
    }
}

/// The dexel grid.
#[derive(Clone, Debug, PartialEq)]
pub struct ZMap {
    pub x0: f64,
    pub y0: f64,
    pub cell: f64,
    pub nx: usize,
    pub ny: usize,
    /// Each column's material, bottom and top; empty where hi ≤ lo.
    pub lo: Vec<f32>,
    pub hi: Vec<f32>,
    /// No column's top is above this (cutting only lowers them).
    pub top: f32,
}

impl ZMap {
    /// A box of stock, its cells square, at most about `cells` of them.
    pub fn new(min: [f64; 3], max: [f64; 3], cells: usize) -> ZMap {
        let (w, d) = ((max[0] - min[0]).max(1e-6), (max[1] - min[1]).max(1e-6));
        let cell = (w * d / cells.max(1) as f64).sqrt();
        let (nx, ny) = (
            ((w / cell).ceil() as usize).max(1),
            ((d / cell).ceil() as usize).max(1),
        );
        // centred on the box
        let (x0, y0) = (
            min[0] + 0.5 * (w - nx as f64 * cell),
            min[1] + 0.5 * (d - ny as f64 * cell),
        );
        ZMap {
            x0,
            y0,
            cell,
            nx,
            ny,
            lo: vec![min[2] as f32; nx * ny],
            hi: vec![max[2] as f32; nx * ny],
            top: max[2] as f32,
        }
    }

    /// A column's centre.
    pub fn centre(&self, i: usize, j: usize) -> [f64; 2] {
        [
            self.x0 + (i as f64 + 0.5) * self.cell,
            self.y0 + (j as f64 + 0.5) * self.cell,
        ]
    }

    /// The column holding point p, if any.
    pub fn cell_of(&self, p: [f64; 2]) -> Option<(usize, usize)> {
        let (fi, fj) = ((p[0] - self.x0) / self.cell, (p[1] - self.y0) / self.cell);
        if fi < 0.0 || fj < 0.0 {
            return None;
        }
        let (i, j) = (fi as usize, fj as usize);
        (i < self.nx && j < self.ny).then_some((i, j))
    }

    /// Columns whose centres fall in [x0, x1] × [y0, y1].
    fn range(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> (usize, usize, usize, usize) {
        let c = self.cell;
        let i0 = ((x0 - self.x0) / c - 0.5).ceil().max(0.0) as usize;
        let j0 = ((y0 - self.y0) / c - 0.5).ceil().max(0.0) as usize;
        let i1 = (((x1 - self.x0) / c - 0.5).floor() + 1.0).clamp(0.0, self.nx as f64) as usize;
        let j1 = (((y1 - self.y0) / c - 0.5).floor() + 1.0).clamp(0.0, self.ny as f64) as usize;
        (i0, j0, i1, j1)
    }

    /// Lowers a column's top to z (a cut from above); true if it changed.
    #[inline]
    fn lower(&mut self, k: usize, z: f32) -> bool {
        if z < self.hi[k] && self.hi[k] > self.lo[k] {
            self.hi[k] = z.max(self.lo[k]);
            true
        } else {
            false
        }
    }

    /// The tool stamped with its tip at p; whether anything changed.
    pub fn stamp(&mut self, p: [f64; 3], tool: &Cutter) -> bool {
        self.stamp_with(p, tool, &mut |g, k, z| g.lower(k, z))
    }

    /// Sweeps the tool along `seg` from fraction u0 to u1; whether anything changed.
    pub fn cut(&mut self, seg: &Seg, u0: f64, u1: f64, tool: &Cutter) -> bool {
        self.cut_with(seg, u0, u1, tool, &mut |g, k, z| g.lower(k, z))
    }

    /// How deep an envelope swept along `seg` would go into the material there is (the most
    /// any column's top stands above it), and where; None if it stays clear. Nothing is cut.
    pub fn depth(&mut self, seg: &Seg, env: &Cutter) -> Option<(f32, [f64; 3])> {
        // nothing stands higher than the tallest column: an envelope above it meets nothing
        let low = seg.a[2].min(seg.b[2]) as f32 + env.h.iter().copied().fold(f32::INFINITY, f32::min);
        if low >= self.top {
            return None;
        }
        let mut worst: Option<(f32, usize, f32)> = None;
        self.cut_with(seg, 0.0, 1.0, env, &mut |g, k, z| {
            let d = g.hi[k].min(g.hi[k].max(g.lo[k])) - z.max(g.lo[k]);
            if g.hi[k] > g.lo[k] && d > 1e-4 && worst.is_none_or(|w| d > w.0) {
                worst = Some((d, k, z));
            }
            false
        });
        worst.map(|(d, k, z)| {
            let c = self.centre(k % self.nx, k / self.nx);
            (d, [c[0], c[1], z as f64])
        })
    }

    /// The tool stamped with its tip at p; whether anything changed.
    fn stamp_with(
        &mut self,
        p: [f64; 3],
        tool: &Cutter,
        act: &mut dyn FnMut(&mut ZMap, usize, f32) -> bool,
    ) -> bool {
        let r = tool.r;
        let (i0, j0, i1, j1) = self.range(p[0] - r, p[1] - r, p[0] + r, p[1] + r);
        let mut any = false;
        for j in j0..j1 {
            for i in i0..i1 {
                let c = self.centre(i, j);
                let d = ((c[0] - p[0]).powi(2) + (c[1] - p[1]).powi(2)).sqrt();
                if let Some(h) = tool.at(d) {
                    any |= act(self, j * self.nx + i, p[2] as f32 + h);
                }
            }
        }
        any
    }

    /// Sweeps the tool along `seg` from fraction u0 to u1; whether anything changed.
    fn cut_with(
        &mut self,
        seg: &Seg,
        u0: f64,
        u1: f64,
        tool: &Cutter,
        act: &mut dyn FnMut(&mut ZMap, usize, f32) -> bool,
    ) -> bool {
        if u1 <= u0 {
            return false;
        }
        let (a, b) = (seg.at(u0), seg.at(u1));
        let flat = (a[2] - b[2]).abs() < 1e-9;
        if !flat {
            if seg.arc.is_none() && (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12 {
                // straight down or up: its lowest place
                return self.stamp_with(if a[2] < b[2] { a } else { b }, tool, act);
            }
            let len = seg.len * (u1 - u0);
            let n = ((len / (0.5 * self.cell)).ceil() as usize).clamp(1, 100_000);
            let mut any = false;
            for k in 0..=n {
                any |= self.stamp_with(seg.at(u0 + (u1 - u0) * k as f64 / n as f64), tool, act);
            }
            return any;
        }
        let z = a[2] as f32;
        let r = tool.r;
        match seg.arc {
            None => {
                let (i0, j0, i1, j1) = self.range(
                    a[0].min(b[0]) - r,
                    a[1].min(b[1]) - r,
                    a[0].max(b[0]) + r,
                    a[1].max(b[1]) + r,
                );
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let ll = dx * dx + dy * dy;
                let mut any = false;
                for j in j0..j1 {
                    for i in i0..i1 {
                        let c = self.centre(i, j);
                        let t = if ll > 0.0 {
                            (((c[0] - a[0]) * dx + (c[1] - a[1]) * dy) / ll).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let d = ((c[0] - a[0] - t * dx).powi(2) + (c[1] - a[1] - t * dy).powi(2)).sqrt();
                        if let Some(h) = tool.at(d) {
                            any |= act(self, j * self.nx + i, z + h);
                        }
                    }
                }
                any
            }
            Some(ar) => {
                // the circle of the start's radius: a posted arc's end radius differs from it by
                // the control's rounding only (`ArcOf::dr`, microns), below the grid's cell
                let (s0, s1) = (ar.a0 + ar.sweep * u0, ar.a0 + ar.sweep * u1);
                let (lo_a, sw) = if s1 >= s0 { (s0, s1 - s0) } else { (s1, s0 - s1) };
                let rr = ar.r + r;
                let (i0, j0, i1, j1) = self.range(ar.c[0] - rr, ar.c[1] - rr, ar.c[0] + rr, ar.c[1] + rr);
                let tau = std::f64::consts::TAU;
                let mut any = false;
                for j in j0..j1 {
                    for i in i0..i1 {
                        let c = self.centre(i, j);
                        let (vx, vy) = (c[0] - ar.c[0], c[1] - ar.c[1]);
                        let rho = (vx * vx + vy * vy).sqrt();
                        if rho > rr || rho < ar.r - r {
                            continue;
                        }
                        let th = dmath::atan2(vy, vx);
                        let off = (th - lo_a).rem_euclid(tau);
                        let d = if off <= sw {
                            (rho - ar.r).abs()
                        } else {
                            let e = |t: f64| {
                                ((c[0] - ar.c[0] - ar.r * dmath::cos(t)).powi(2)
                                    + (c[1] - ar.c[1] - ar.r * dmath::sin(t)).powi(2))
                                .sqrt()
                            };
                            e(lo_a).min(e(lo_a + sw))
                        };
                        if let Some(h) = tool.at(d) {
                            any |= act(self, j * self.nx + i, z + h);
                        }
                    }
                }
                any
            }
        }
    }

    /// The stock turned over about x as a setup turned over sees it: (x, y, z) → (x, −y, h − z),
    /// resampled onto a grid over `min`..`max` (the new setup's stock box) of about `cells`.
    pub fn turned(&self, h: f64, min: [f64; 3], max: [f64; 3], cells: usize) -> ZMap {
        let mut out = ZMap::new(min, max, cells);
        for j in 0..out.ny {
            for i in 0..out.nx {
                let c = out.centre(i, j);
                let k = j * out.nx + i;
                match self.cell_of([c[0], -c[1]]) {
                    Some((si, sj)) => {
                        let sk = sj * self.nx + si;
                        if self.hi[sk] > self.lo[sk] {
                            out.lo[k] = (h - self.hi[sk] as f64) as f32;
                            out.hi[k] = (h - self.lo[sk] as f64) as f32;
                        } else {
                            out.lo[k] = 0.0;
                            out.hi[k] = 0.0;
                        }
                    }
                    None => {
                        out.lo[k] = 0.0;
                        out.hi[k] = 0.0;
                    }
                }
            }
        }
        out.top = out.hi.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        out
    }

    /// Material left, mm³ (the columns' lengths times a cell's area).
    pub fn volume(&self) -> f64 {
        let a = self.cell * self.cell;
        self.lo
            .iter()
            .zip(&self.hi)
            .map(|(l, h)| ((h - l).max(0.0)) as f64 * a)
            .sum()
    }
}

/// The part as columns too, for comparison: its highest and lowest surface over each column
/// (None where it isn't), from its triangles in the setup's frame.
#[derive(Clone, Debug)]
pub struct PartMap {
    pub top: Vec<f32>,
    pub bottom: Vec<f32>,
    /// The lowest top over the column and its eight neighbours (−∞ if any of them has no part):
    /// a column within a cell of a wall is not called a gouge (the grid cannot tell there).
    pub floor: Vec<f32>,
}

impl PartMap {
    pub fn of(grid: &ZMap, tris: &[[[f64; 3]; 3]]) -> PartMap {
        let n = grid.nx * grid.ny;
        let mut top = vec![f32::NEG_INFINITY; n];
        let mut bottom = vec![f32::INFINITY; n];
        for t in tris {
            let [a, b, c] = *t;
            let den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
            if den.abs() < 1e-14 {
                continue; // vertical: seen edge-on from above
            }
            let (i0, j0, i1, j1) = grid.range(
                a[0].min(b[0]).min(c[0]),
                a[1].min(b[1]).min(c[1]),
                a[0].max(b[0]).max(c[0]),
                a[1].max(b[1]).max(c[1]),
            );
            for j in j0..j1 {
                for i in i0..i1 {
                    let p = grid.centre(i, j);
                    let l0 = ((b[1] - c[1]) * (p[0] - c[0]) + (c[0] - b[0]) * (p[1] - c[1])) / den;
                    let l1 = ((c[1] - a[1]) * (p[0] - c[0]) + (a[0] - c[0]) * (p[1] - c[1])) / den;
                    let l2 = 1.0 - l0 - l1;
                    let e = -1e-9;
                    if l0 < e || l1 < e || l2 < e {
                        continue;
                    }
                    let z = (l0 * a[2] + l1 * b[2] + l2 * c[2]) as f32;
                    let k = j * grid.nx + i;
                    top[k] = top[k].max(z);
                    bottom[k] = bottom[k].min(z);
                }
            }
        }
        let (nx, ny) = (grid.nx, grid.ny);
        let mut floor = vec![f32::NEG_INFINITY; n];
        for j in 0..ny {
            for i in 0..nx {
                let mut m = f32::INFINITY;
                for (di, dj) in [
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                    (-1, 0),
                    (0, 0),
                    (1, 0),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                ] {
                    let (a, b) = (i as i64 + di, j as i64 + dj);
                    let t = if a < 0 || b < 0 || a >= nx as i64 || b >= ny as i64 {
                        f32::NEG_INFINITY
                    } else {
                        top[b as usize * nx + a as usize]
                    };
                    m = m.min(t);
                }
                floor[j * nx + i] = m;
            }
        }
        PartMap { top, bottom, floor }
    }

    pub fn has(&self, k: usize) -> bool {
        self.top[k].is_finite()
    }
}

/// How a column compares with the part (display classes; the verdict is the checker's).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Compare {
    /// Material is still there above the part (or where the part isn't) by more than tol.
    Under,
    /// Within ±tol of the part's top.
    InTol,
    /// The top is below the part's by more than tol.
    Gouge,
}

/// A column's class against the part with tolerance `tol` (an empty column over no part: None).
pub fn compare(g: &ZMap, part: &PartMap, k: usize, tol: f32) -> Option<(Compare, f32)> {
    let (lo, hi) = (g.lo[k], g.hi[k]);
    if part.has(k) {
        let wall = !part.floor[k].is_finite();
        if hi <= lo {
            // cut through where the part is: below its bottom (at a wall, the grid can't tell)
            let d = -(part.top[k] - part.bottom[k]).max(tol * 2.0);
            return (!wall).then_some((Compare::Gouge, d));
        }
        let d = hi - part.top[k];
        Some(if d < -tol && !wall && hi - part.floor[k] < -tol {
            (Compare::Gouge, d)
        } else if d <= tol {
            (Compare::InTol, d)
        } else {
            (Compare::Under, d)
        })
    } else if hi > lo + tol {
        Some((Compare::Under, hi - lo))
    } else {
        None
    }
}

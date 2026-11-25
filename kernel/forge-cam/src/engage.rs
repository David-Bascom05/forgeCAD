//! Feeds from engagement: the stock a path meets, modelled as a height map, and the feed of each
//! piece of the path set so that the chip the edge cuts (after radial thinning) is the cutting
//! data's feed per tooth, never more.
//!
//! This is offline feed optimisation: the swept tool is taken from a model of the stock, the
//! engagement of each move measured, and the largest feed within the limit chosen, so the tool
//! runs at full chip load in light cuts and is never overloaded in heavy ones. The path's
//! geometry is unchanged: a move is only split where its feed changes (lines into collinear
//! pieces, arcs and helices into pieces of the same circle), and what is posted is certified as
//! before. Feeds are no part of G1–G4; G5 checks every posted feed against the machine.
//!
//! The model: a z-up dexel per cell (its centre) over the setup's stock box, lowered by every
//! void the in-process stock proves cleared and by the tool's body along the path. It holds at
//! least the true stock (the voids are what is provably cleared; undercuts are not taken), so
//! engagement read from it is at least the true one up to a cell, and the feed it gives is not
//! above what the true engagement allows (up to the cell's error and the quantisation).
use crate::ops::{OpPath, PMove, Phase};
use crate::stock::InProcess;
use forge_cam_geom::region::{Region, Seg};
use forge_math::dmath;

/// The tool's lower envelope: the height of its body above the tip at radial distance d, out
/// to its radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Profile {
    /// A flat end (re 0) or a bull nose (corner radius re).
    Bull {
        r: f64,
        re: f64,
    },
    Ball {
        r: f64,
    },
}

impl Profile {
    pub fn of(t: &forge_tooling::Tool) -> Option<Profile> {
        use forge_tooling::Kind;
        let r = 0.5 * t.dc;
        match t.kind {
            Kind::Flat | Kind::Slot => Some(Profile::Bull { r, re: 0.0 }),
            Kind::Bull => Some(Profile::Bull {
                r,
                re: t.re.clamp(0.0, r),
            }),
            Kind::Ball => Some(Profile::Ball { r }),
            _ => None,
        }
    }

    pub fn r(&self) -> f64 {
        match *self {
            Profile::Bull { r, .. } | Profile::Ball { r } => r,
        }
    }

    /// Height above the tip at radial distance d ≤ r.
    pub fn at(&self, d: f64) -> f64 {
        match *self {
            Profile::Bull { r, re } => {
                let flat = r - re;
                if d <= flat {
                    0.0
                } else {
                    let u = (d - flat).min(re);
                    re - (re * re - u * u).max(0.0).sqrt()
                }
            }
            Profile::Ball { r } => {
                let u = d.min(r);
                r - (r * r - u * u).max(0.0).sqrt()
            }
        }
    }
}

/// The stock as heights over a grid of cells (the cell's centre).
#[derive(Clone, Debug)]
pub struct HeightMap {
    pub x0: f64,
    pub y0: f64,
    pub h: f64,
    pub nx: usize,
    pub ny: usize,
    pub bottom: f64,
    pub z: Vec<f64>,
}

/// A closed loop as a polygon (arcs as chords, 32 a turn), for filling cells only.
fn polygon(segs: &[Seg]) -> Vec<(f64, f64)> {
    let mut pts = Vec::new();
    for sg in segs {
        match *sg {
            Seg::Line { a, .. } => pts.push((a.x, a.y)),
            Seg::Arc { a, b, c, ccw } => {
                let r = ((a.x - c.x) * (a.x - c.x) + (a.y - c.y) * (a.y - c.y)).sqrt();
                let a0 = dmath::atan2(a.y - c.y, a.x - c.x);
                let a1 = dmath::atan2(b.y - c.y, b.x - c.x);
                let sw = sweep(a0, a1, ccw);
                let n = ((sw.abs() / std::f64::consts::TAU * 32.0).ceil() as usize).max(1);
                for k in 0..n {
                    let (sn, cs) = dmath::sin_cos(a0 + sw * k as f64 / n as f64);
                    pts.push((c.x + r * cs, c.y + r * sn));
                }
            }
        }
    }
    pts
}

/// A closed loop as a polygon whose chords stray from its arcs by at most `sag` (and no fewer
/// than `polygon`'s 32 a turn).
fn polygon_fine(segs: &[Seg], sag: f64) -> Vec<(f64, f64)> {
    let mut pts = Vec::new();
    for sg in segs {
        match *sg {
            Seg::Line { a, .. } => pts.push((a.x, a.y)),
            Seg::Arc { a, b, c, ccw } => {
                let r = ((a.x - c.x) * (a.x - c.x) + (a.y - c.y) * (a.y - c.y)).sqrt();
                let a0 = dmath::atan2(a.y - c.y, a.x - c.x);
                let a1 = dmath::atan2(b.y - c.y, b.x - c.x);
                let sw = sweep(a0, a1, ccw);
                // a chord of length l strays l²/8r from its arc
                let step = (8.0 * r * sag).sqrt();
                let n = ((sw.abs() / std::f64::consts::TAU * 32.0).ceil() as usize)
                    .max((sw.abs() * r / step).ceil() as usize)
                    .max(1);
                for k in 0..n {
                    let (sn, cs) = dmath::sin_cos(a0 + sw * k as f64 / n as f64);
                    pts.push((c.x + r * cs, c.y + r * sn));
                }
            }
        }
    }
    pts
}

/// The signed sweep from angle a0 to a1, counter-clockwise if ccw, in (0, 2π] or [−2π, 0) (the
/// same angle: a whole turn).
fn sweep(a0: f64, a1: f64, ccw: bool) -> f64 {
    let tau = std::f64::consts::TAU;
    let mut sw = a1 - a0;
    if ccw {
        while sw <= 0.0 {
            sw += tau;
        }
        while sw > tau {
            sw -= tau;
        }
    } else {
        while sw >= 0.0 {
            sw -= tau;
        }
        while sw < -tau {
            sw += tau;
        }
    }
    sw
}

impl HeightMap {
    /// The box [min, max] in cells of size h.
    pub fn new(min: [f64; 3], max: [f64; 3], h: f64) -> HeightMap {
        let nx = (((max[0] - min[0]) / h).ceil() as usize).max(1);
        let ny = (((max[1] - min[1]) / h).ceil() as usize).max(1);
        HeightMap {
            x0: min[0],
            y0: min[1],
            h,
            nx,
            ny,
            bottom: min[2],
            z: vec![max[2]; nx * ny],
        }
    }

    /// The in-process stock: its box less each void (a cell is lowered to a void's floor when
    /// its centre is inside the void's region, even–odd over its loops).
    pub fn of_stock(ip: &InProcess, h: f64) -> HeightMap {
        let mut m = HeightMap::new(ip.min, ip.max, h);
        m.lower_voids(ip);
        m
    }

    /// Every cell lowered to the in-process stock's voids (the floor of each void whose region
    /// holds the cell's centre), where it is higher: a map of the stock as earlier moves left it
    /// brought down to voids recorded since.
    pub fn lower_voids(&mut self, ip: &InProcess) {
        for (rg, floor) in &ip.voids {
            self.lower_region(rg, *floor);
        }
    }

    fn lower_region(&mut self, rg: &Region, floor: f64) {
        let polys: Vec<Vec<(f64, f64)>> = rg.loops.iter().map(|l| polygon(&l.segs)).collect();
        let mut xs: Vec<f64> = Vec::new();
        for j in 0..self.ny {
            let y = self.y0 + (j as f64 + 0.5) * self.h;
            xs.clear();
            for p in &polys {
                let n = p.len();
                for i in 0..n {
                    let (x0, y0) = p[i];
                    let (x1, y1) = p[(i + 1) % n];
                    if (y0 > y) != (y1 > y) {
                        xs.push(x0 + (y - y0) * (x1 - x0) / (y1 - y0));
                    }
                }
            }
            xs.sort_by(f64::total_cmp);
            for pair in xs.as_chunks::<2>().0 {
                // cells whose centre lies in [pair[0], pair[1]]
                let i0 = ((pair[0] - self.x0) / self.h - 0.5).ceil().max(0.0) as usize;
                let i1 = ((pair[1] - self.x0) / self.h - 0.5).floor();
                if i1 < 0.0 {
                    continue;
                }
                let i1 = (i1 as usize).min(self.nx.saturating_sub(1));
                for i in i0..=i1 {
                    let k = j * self.nx + i;
                    if self.z[k] > floor {
                        self.z[k] = floor.max(self.bottom);
                    }
                }
            }
        }
    }

    /// The in-process stock, never below it: `of_stock` with each void lowering only the cells
    /// wholly inside its region (`lower_voids_under`).
    pub fn of_stock_under(ip: &InProcess, h: f64) -> HeightMap {
        let mut m = HeightMap::new(ip.min, ip.max, h);
        m.lower_voids_under(ip);
        m
    }

    /// `lower_voids`, but a cell is lowered only where the whole cell lies inside the void's
    /// region (its centre inside and at least h/√2 from every edge, the edges' arcs taken as
    /// chords within h/100 of them, which the margin covers): no cell is ever brought below
    /// material it still holds, however thin. For tests of clearance (hops), not for engagement.
    pub fn lower_voids_under(&mut self, ip: &InProcess) {
        for (rg, floor) in &ip.voids {
            self.lower_region_under(rg, *floor);
        }
    }

    fn lower_region_under(&mut self, rg: &Region, floor: f64) {
        let sag = 0.01 * self.h;
        let polys: Vec<Vec<(f64, f64)>> = rg.loops.iter().map(|l| polygon_fine(&l.segs, sag)).collect();
        let margin = self.h * std::f64::consts::FRAC_1_SQRT_2 + sag;
        let mut inside = vec![false; self.nx * self.ny];
        let mut xs: Vec<f64> = Vec::new();
        for j in 0..self.ny {
            let y = self.y0 + (j as f64 + 0.5) * self.h;
            xs.clear();
            for p in &polys {
                let n = p.len();
                for i in 0..n {
                    let (x0, y0) = p[i];
                    let (x1, y1) = p[(i + 1) % n];
                    if (y0 > y) != (y1 > y) {
                        xs.push(x0 + (y - y0) * (x1 - x0) / (y1 - y0));
                    }
                }
            }
            xs.sort_by(f64::total_cmp);
            for pair in xs.as_chunks::<2>().0 {
                let i0 = ((pair[0] - self.x0) / self.h - 0.5).ceil().max(0.0) as usize;
                let i1 = ((pair[1] - self.x0) / self.h - 0.5).floor();
                if i1 < 0.0 {
                    continue;
                }
                let i1 = (i1 as usize).min(self.nx.saturating_sub(1));
                for i in i0..=i1 {
                    inside[j * self.nx + i] = true;
                }
            }
        }
        // the cells whose centre lies within the margin of an edge: not wholly inside
        let cell = |v: f64, o: f64| ((v - o) / self.h - 0.5).floor();
        for p in &polys {
            let n = p.len();
            for k in 0..n {
                let (a, b) = (p[k], p[(k + 1) % n]);
                let i0 = cell(a.0.min(b.0) - margin, self.x0).max(0.0) as usize;
                let j0 = cell(a.1.min(b.1) - margin, self.y0).max(0.0) as usize;
                let i1 = ((cell(a.0.max(b.0) + margin, self.x0) + 2.0).max(0.0) as usize).min(self.nx);
                let j1 = ((cell(a.1.max(b.1) + margin, self.y0) + 2.0).max(0.0) as usize).min(self.ny);
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let ll = dx * dx + dy * dy;
                for j in j0..j1 {
                    let y = self.y0 + (j as f64 + 0.5) * self.h;
                    for i in i0..i1 {
                        let x = self.x0 + (i as f64 + 0.5) * self.h;
                        let t = if ll > 0.0 {
                            (((x - a.0) * dx + (y - a.1) * dy) / ll).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let (px, py) = (a.0 + t * dx, a.1 + t * dy);
                        if (x - px) * (x - px) + (y - py) * (y - py) < margin * margin {
                            inside[j * self.nx + i] = false;
                        }
                    }
                }
            }
        }
        for (k, &ins) in inside.iter().enumerate() {
            if ins && self.z[k] > floor {
                self.z[k] = floor.max(self.bottom);
            }
        }
    }

    /// `sweep`, never below the material: a cell is lowered only where one position of the tool
    /// along a → b covers the whole cell (its centre within r − h/√2 of the tip's axis there),
    /// and only to the body's highest point over the cell at that position. Thin material the
    /// tool's edge left inside a cell keeps the cell up. For tests of clearance (hops).
    pub fn sweep_under(&mut self, a: [f64; 3], b: [f64; 3], p: &Profile) -> f64 {
        let r = p.r();
        let hh = self.h * std::f64::consts::FRAC_1_SQRT_2;
        let cell = |v: f64, o: f64| ((v - o) / self.h - 0.5).floor();
        let i0 = cell(a[0].min(b[0]) - r, self.x0).max(0.0) as usize;
        let j0 = cell(a[1].min(b[1]) - r, self.y0).max(0.0) as usize;
        let i1 = ((cell(a[0].max(b[0]) + r, self.x0) + 2.0).max(0.0) as usize).min(self.nx);
        let j1 = ((cell(a[1].max(b[1]) + r, self.y0) + 2.0).max(0.0) as usize).min(self.ny);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let ll = dx * dx + dy * dy;
        let zmin = a[2].min(b[2]);
        let mut removed = 0.0;
        for j in j0..j1 {
            let y = self.y0 + (j as f64 + 0.5) * self.h;
            for i in i0..i1 {
                let k = j * self.nx + i;
                let old = self.z[k];
                if old <= zmin || old <= self.bottom {
                    continue;
                }
                let x = self.x0 + (i as f64 + 0.5) * self.h;
                let t = if ll > 0.0 {
                    (((x - a[0]) * dx + (y - a[1]) * dy) / ll).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (px, py) = (a[0] + t * dx, a[1] + t * dy);
                let d = ((x - px) * (x - px) + (y - py) * (y - py)).sqrt();
                if d + hh > r {
                    continue;
                }
                let mut zc = a[2] + t * (b[2] - a[2]) + p.at(d + hh);
                for e in [a, b] {
                    let de = ((x - e[0]) * (x - e[0]) + (y - e[1]) * (y - e[1])).sqrt();
                    if de + hh <= r {
                        zc = zc.min(e[2] + p.at(de + hh));
                    }
                }
                let new = zc.max(self.bottom);
                if new < old {
                    removed += old - new;
                    self.z[k] = new;
                }
            }
        }
        removed * self.h * self.h
    }

    /// The top at a point; outside the box, no material (−∞).
    pub fn top(&self, x: f64, y: f64) -> f64 {
        let i = ((x - self.x0) / self.h).floor();
        let j = ((y - self.y0) / self.h).floor();
        if i < 0.0 || j < 0.0 || i >= self.nx as f64 || j >= self.ny as f64 {
            return f64::NEG_INFINITY;
        }
        self.z[j as usize * self.nx + i as usize]
    }

    /// Sweeps the tool's body with its tip along a → b (height linear): cells whose centre is
    /// within r of the segment are lowered to the body. The volume removed (mm³).
    pub fn sweep(&mut self, a: [f64; 3], b: [f64; 3], p: &Profile) -> f64 {
        self.sweep_in(a, b, p, None)
    }

    /// Every move of a path from `start` swept (`sweep`), an arc or helix as chords no longer
    /// than a cell; a probe or a cycle is passed over (its end taken as the next start). For a
    /// model of what the path leaves, as `sweep` makes it: at each cell's centre, not under it.
    pub fn sweep_path(&mut self, start: [f64; 3], moves: &[PMove], p: &Profile) {
        let mut at = start;
        for m in moves {
            match *m {
                PMove::Rapid { to } | PMove::Line { to, .. } => {
                    self.sweep(at, to, p);
                    at = to;
                }
                PMove::Arc { to, c, cw, .. } => {
                    let a0 = dmath::atan2(at[1] - c[1], at[0] - c[0]);
                    let sw = sweep(a0, dmath::atan2(to[1] - c[1], to[0] - c[0]), !cw);
                    let rad = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt();
                    let k = ((sw.abs() * rad / self.h).ceil() as usize).max(1);
                    let mut q0 = at;
                    for i in 1..=k {
                        let t = i as f64 / k as f64;
                        let (sn, cs) = dmath::sin_cos(a0 + t * sw);
                        let q = [c[0] + rad * cs, c[1] + rad * sn, at[2] + t * (to[2] - at[2])];
                        self.sweep(q0, q, p);
                        q0 = q;
                    }
                    at = to;
                }
                PMove::Probe { to, .. } => at = to,
                PMove::Cycle { .. } => {}
            }
        }
    }

    /// `sweep`, passing over the runs of a row that lie in a tile of `tiles` whose highest cell
    /// is at or below both the move's lowest tip and the bottom: `sweep` leaves every such cell
    /// as it is (its first test). The cells are visited in the same order: the same map and the
    /// same volume, to the bit. `tiles` must be `Tiles::of` this map, changed by `sweep_in`
    /// alone since (it keeps each tile's maximum at or above its cells).
    pub fn sweep_in(&mut self, a: [f64; 3], b: [f64; 3], p: &Profile, mut tiles: Option<&mut Tiles>) -> f64 {
        let r = p.r();
        let cell = |v: f64, o: f64| ((v - o) / self.h - 0.5).floor();
        let i0 = cell(a[0].min(b[0]) - r, self.x0).max(0.0) as usize;
        let j0 = cell(a[1].min(b[1]) - r, self.y0).max(0.0) as usize;
        let i1 = ((cell(a[0].max(b[0]) + r, self.x0) + 2.0).max(0.0) as usize).min(self.nx);
        let j1 = ((cell(a[1].max(b[1]) + r, self.y0) + 2.0).max(0.0) as usize).min(self.ny);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let ll = dx * dx + dy * dy;
        let zmin = a[2].min(b[2]);
        let mut removed = 0.0;
        let floor = zmin.max(self.bottom);
        // the column runs of each tile row holding a tile the move may lower (all of [i0, i1)
        // without tiles), found once per tile row
        let mut runs: Vec<(usize, usize)> = Vec::new();
        let mut runs_row = usize::MAX;
        for j in j0..j1 {
            let y = self.y0 + (j as f64 + 0.5) * self.h;
            if runs_row != j / TILE {
                runs_row = j / TILE;
                runs.clear();
                match tiles.as_deref() {
                    None => runs.push((i0, i1)),
                    Some(t) => {
                        let mut i = i0;
                        while i < i1 {
                            let ti = i / TILE;
                            let end = ((ti + 1) * TILE).min(i1);
                            // a tile at or below the tip's lowest and the bottom: every cell of
                            // it is left as it is (the test below), its run passed over
                            if t.max[runs_row * t.ntx + ti] > floor {
                                match runs.last_mut() {
                                    Some(r) if r.1 == i => r.1 = end,
                                    _ => runs.push((i, end)),
                                }
                            }
                            i = end;
                        }
                    }
                }
            }
            for &(r0, r1) in &runs {
                for i in r0..r1 {
                    let k = j * self.nx + i;
                    let old = self.z[k];
                    if old <= zmin || old <= self.bottom {
                        continue;
                    }
                    let x = self.x0 + (i as f64 + 0.5) * self.h;
                    let t = if ll > 0.0 {
                        (((x - a[0]) * dx + (y - a[1]) * dy) / ll).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let (px, py) = (a[0] + t * dx, a[1] + t * dy);
                    let d = ((x - px) * (x - px) + (y - py) * (y - py)).sqrt();
                    if d > r {
                        continue;
                    }
                    let mut zc = a[2] + t * (b[2] - a[2]) + p.at(d);
                    for e in [a, b] {
                        let de = ((x - e[0]) * (x - e[0]) + (y - e[1]) * (y - e[1])).sqrt();
                        if de <= r {
                            zc = zc.min(e[2] + p.at(de));
                        }
                    }
                    let new = zc.max(self.bottom);
                    if new < old {
                        removed += old - new;
                        self.z[k] = new;
                        if let Some(t) = tiles.as_deref_mut() {
                            let q = (j / TILE) * t.ntx + i / TILE;
                            if !t.dirty[q] {
                                t.dirty[q] = true;
                                t.touched.push(q);
                            }
                        }
                    }
                }
            }
        }
        if let Some(t) = tiles {
            for q in std::mem::take(&mut t.touched) {
                t.dirty[q] = false;
                t.max[q] = tile_max(self, q % t.ntx, q / t.ntx);
            }
        }
        removed * self.h * self.h
    }

    /// What the side edge's front half meets at tip c moving along u (θ from the direction of
    /// travel, + left): the stock standing above the tip by more than `eps`, the edge read at
    /// r + h/2, a little outside the tool, so the cells' error errs toward more engagement.
    /// None: the edge meets nothing.
    pub fn engage(&self, c: [f64; 3], u: [f64; 2], r: f64, eps: f64) -> Option<Engage> {
        self.engage_in(c, u, r, eps, None)
    }

    /// `engage`, first asking `tiles` (`Tiles::of` this map, kept by `sweep_in`): where every
    /// tile the edge can read is at or below c[2] + eps, no point of it stands above the tip by
    /// more than eps (each read is a cell of one of those tiles, or −∞ off the map), so the
    /// answer is None without reading the edge. Otherwise the edge is read as `engage` does.
    pub fn engage_in(
        &self,
        c: [f64; 3],
        u: [f64; 2],
        r: f64,
        eps: f64,
        tiles: Option<&Tiles>,
    ) -> Option<Engage> {
        if let Some(t) = tiles {
            let rr = r + 0.5 * self.h;
            let thr = c[2] + eps;
            // the cells `top` can read for points within rr of c: their indices, a cell wider
            let i0 = ((c[0] - rr - self.x0) / self.h).floor() - 1.0;
            let i1 = ((c[0] + rr - self.x0) / self.h).floor() + 1.0;
            let j0 = ((c[1] - rr - self.y0) / self.h).floor() - 1.0;
            let j1 = ((c[1] + rr - self.y0) / self.h).floor() + 1.0;
            if i0.is_finite() && i1.is_finite() && j0.is_finite() && j1.is_finite() {
                let clamp = |v: f64, n: usize| v.max(0.0).min(n.saturating_sub(1) as f64) as usize;
                let (a0, a1) = (clamp(i0, self.nx) / TILE, clamp(i1, self.nx) / TILE);
                let (b0, b1) = (clamp(j0, self.ny) / TILE, clamp(j1, self.ny) / TILE);
                let mut low = true;
                'tiles: for tj in b0..=b1 {
                    for ti in a0..=a1 {
                        // at or below (a NaN is neither: the edge is read)
                        let below = matches!(
                            t.max[tj * t.ntx + ti].partial_cmp(&thr),
                            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                        );
                        if !below {
                            low = false;
                            break 'tiles;
                        }
                    }
                }
                if low {
                    return None;
                }
            }
        }
        let rr = r + 0.5 * self.h;
        let n = EDGE_N;
        let step = std::f64::consts::PI / (n - 1) as f64;
        let mut e: Option<Engage> = None;
        for (k, &(th, s, co)) in edge().iter().enumerate() {
            let dx = u[0] * co - u[1] * s;
            let dy = u[0] * s + u[1] * co;
            let t = self.top(c[0] + rr * dx, c[1] + rr * dy);
            if t > c[2] + eps {
                let g = e.get_or_insert(Engage {
                    cos_max: co,
                    th_lo: th,
                    th_hi: th,
                    phi: 0.0,
                    ap: 0.0,
                    ae: 0.0,
                });
                g.cos_max = g.cos_max.max(co);
                g.th_lo = g.th_lo.min(th);
                g.th_hi = g.th_hi.max(th);
                g.phi += if k == 0 || k == n - 1 { 0.5 * step } else { step };
                g.ap = g.ap.max(t - c[2]);
            }
        }
        if let Some(g) = &mut e {
            // the width of material across the travel the engaged span covers
            g.ae = r * (dmath::sin(g.th_hi) - dmath::sin(g.th_lo)).max(0.0);
        }
        e
    }

    /// The largest cos θ over the engaged front edge: the chip there is fz · that (`engage`).
    pub fn chip_factor(&self, c: [f64; 3], u: [f64; 2], r: f64, eps: f64) -> Option<f64> {
        self.engage(c, u, r, eps).map(|e| e.cos_max)
    }
}

/// The side of a tile of `Tiles`, in cells.
const TILE: usize = 8;

/// The highest cell of each TILE × TILE tile of a height map (+∞ where a cell is NaN), for
/// `HeightMap::sweep_in` to pass over tiles a move cannot lower.
#[derive(Clone, Debug)]
pub struct Tiles {
    ntx: usize,
    max: Vec<f64>,
    dirty: Vec<bool>,
    touched: Vec<usize>,
}

impl Tiles {
    pub fn of(m: &HeightMap) -> Tiles {
        let (ntx, nty) = (m.nx.div_ceil(TILE), m.ny.div_ceil(TILE));
        let mut max = vec![f64::NEG_INFINITY; ntx * nty];
        for tj in 0..nty {
            for ti in 0..ntx {
                max[tj * ntx + ti] = tile_max(m, ti, tj);
            }
        }
        Tiles {
            ntx,
            dirty: vec![false; max.len()],
            max,
            touched: Vec::new(),
        }
    }
}

fn tile_max(m: &HeightMap, ti: usize, tj: usize) -> f64 {
    let mut hi = f64::NEG_INFINITY;
    for j in tj * TILE..((tj + 1) * TILE).min(m.ny) {
        for &z in &m.z[j * m.nx + ti * TILE..j * m.nx + ((ti + 1) * TILE).min(m.nx)] {
            if z.is_nan() {
                return f64::INFINITY;
            }
            hi = hi.max(z);
        }
    }
    hi
}

/// Points round the front half of the edge, 1° apart (`HeightMap::engage`).
const EDGE_N: usize = 181;

/// θ and its sine and cosine at each of `engage`'s edge points, computed once: the same
/// expressions `engage` evaluated at every call, so the same numbers.
fn edge() -> &'static [(f64, f64, f64); EDGE_N] {
    static EDGE: std::sync::OnceLock<[(f64, f64, f64); EDGE_N]> = std::sync::OnceLock::new();
    EDGE.get_or_init(|| {
        let step = std::f64::consts::PI / (EDGE_N - 1) as f64;
        let mut t = [(0.0, 0.0, 0.0); EDGE_N];
        for (k, x) in t.iter_mut().enumerate() {
            let th = -std::f64::consts::FRAC_PI_2 + k as f64 * step;
            let (s, co) = dmath::sin_cos(th);
            *x = (th, s, co);
        }
        t
    })
}

/// What the side edge meets at one place (HeightMap::engage).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Engage {
    /// max cos θ over the engaged edge: the chip is fz · this.
    pub cos_max: f64,
    /// The engaged span's ends, radians from the travel (+ left), and the angle engaged in it
    /// (radians; the span may have gaps).
    pub th_lo: f64,
    pub th_hi: f64,
    pub phi: f64,
    /// Axial depth (the highest stock above the tip on the engaged edge) and the radial width
    /// across the travel, mm.
    pub ap: f64,
    pub ae: f64,
}

/// One fixed-length sample of a scheduled path (`schedule_samples`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    /// The move it belongs to and its end along the path.
    pub mv: usize,
    pub end: [f64; 3],
    /// Arc length (mm) from the path's start to the sample's end.
    pub s: f64,
    /// The heaviest engagement met in the sample (None: air).
    pub engage: Option<Engage>,
}

/// What the feeds are set from.
///
/// The feed is set per FIXED-LENGTH sample from chip thinning and the chip load: block lengths
/// stay predictable for the controller's lookahead, and the path is never split where the
/// engagement changes. There is no abrasion term, and every sample is its own move: samples are
/// never joined, whatever their feeds (equal F words are simply not repeated when posted).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sched {
    /// The chip to cut (feed per tooth), mm.
    pub fz: f64,
    pub flutes: f64,
    pub rpm: f64,
    /// The largest and least feed, mm/min (the largest kept under the machine's limit by the
    /// caller; see `schedule`).
    pub f_max: f64,
    pub f_min: f64,
    /// The most chip thinning may raise the feed per tooth by (forge_tooling::feeds::THIN_CAP).
    pub cap: f64,
    /// Cell size of the model, mm.
    pub cell: f64,
    /// Sample length, mm: each move is divided into equal samples no longer than this.
    pub sample: f64,
}

/// Stock this far above the tip counts as material (mm).
const EPS_Z: f64 = 1e-3;

/// A move divided into n equal pieces: their ends (the last the move's end).
fn pieces_n(from: [f64; 3], m: &PMove, n: usize) -> Vec<[f64; 3]> {
    match *m {
        PMove::Rapid { to } | PMove::Line { to, .. } => (1..=n)
            .map(|k| {
                if k == n {
                    return to;
                }
                let s = k as f64 / n as f64;
                [
                    from[0] + s * (to[0] - from[0]),
                    from[1] + s * (to[1] - from[1]),
                    from[2] + s * (to[2] - from[2]),
                ]
            })
            .collect(),
        PMove::Arc { to, c, cw, .. } => {
            let r = ((from[0] - c[0]) * (from[0] - c[0]) + (from[1] - c[1]) * (from[1] - c[1])).sqrt();
            let a0 = dmath::atan2(from[1] - c[1], from[0] - c[0]);
            let a1 = dmath::atan2(to[1] - c[1], to[0] - c[0]);
            let sw = sweep(a0, a1, !cw);
            (1..=n)
                .map(|k| {
                    if k == n {
                        return to;
                    }
                    let s = k as f64 / n as f64;
                    let (sn, cs) = dmath::sin_cos(a0 + s * sw);
                    [c[0] + r * cs, c[1] + r * sn, from[2] + s * (to[2] - from[2])]
                })
                .collect()
        }
        PMove::Cycle { .. } => vec![],
        // a probe cuts nothing
        PMove::Probe { .. } => vec![],
    }
}

/// A move's length (arcs along the helix).
pub(crate) fn move_len(from: [f64; 3], m: &PMove) -> f64 {
    let d3 = |a: [f64; 3], b: [f64; 3]| {
        ((a[0] - b[0]) * (a[0] - b[0]) + (a[1] - b[1]) * (a[1] - b[1]) + (a[2] - b[2]) * (a[2] - b[2])).sqrt()
    };
    match *m {
        PMove::Rapid { to } | PMove::Line { to, .. } => d3(from, to),
        PMove::Arc { to, c, cw, .. } => {
            let r = ((from[0] - c[0]) * (from[0] - c[0]) + (from[1] - c[1]) * (from[1] - c[1])).sqrt();
            let a0 = dmath::atan2(from[1] - c[1], from[0] - c[0]);
            let a1 = dmath::atan2(to[1] - c[1], to[0] - c[0]);
            let xy = sweep(a0, a1, !cw).abs() * r;
            (xy * xy + (to[2] - from[2]) * (to[2] - from[2])).sqrt()
        }
        PMove::Cycle { .. } => 0.0,
        // a probe cuts nothing: no engagement length
        PMove::Probe { .. } => 0.0,
    }
}

/// Whether a move is fed by engagement: a feed move with xy motion, not climbing.
fn scheduled(at: [f64; 3], m: &PMove) -> bool {
    match *m {
        PMove::Line { to, phase, .. } => {
            phase != Phase::Rapid && (to[0] != at[0] || to[1] != at[1]) && to[2] <= at[2]
        }
        PMove::Arc { to, .. } => to[2] <= at[2],
        _ => false,
    }
}

/// The path's samples: each scheduled move divided into equal samples no longer than
/// `s.sample` (boundaries fixed by length alone), each measured in pieces no longer than
/// min(R/4, 0.5) mm against `map` before the piece is swept; `map` is left as the path cut it.
pub fn schedule_samples(path: &OpPath, map: &mut HeightMap, prof: &Profile, s: &Sched) -> Vec<Sample> {
    let r = prof.r();
    let l = s.cell.max((0.25 * r).min(0.5));
    let mut tiles = Tiles::of(map);
    let mut out = Vec::new();
    let mut at = path.start;
    let mut arc = 0.0;
    for (mi, mv) in path.moves.iter().enumerate() {
        let to = match *mv {
            PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => to,
            PMove::Cycle { .. } => continue,
            // a probe cuts nothing; the path goes on from its start when it backs off
            PMove::Probe { to, back, .. } => {
                if !back {
                    at = to;
                }
                continue;
            }
        };
        let len = move_len(at, mv);
        let sched = scheduled(at, mv);
        // a hop (crate::hop) is read whole as one sample: one block, its feed the chip rule's for
        // the worst piece if any meets material, else its own
        let hop = matches!(
            mv,
            PMove::Line {
                phase: Phase::Hop,
                ..
            }
        );
        let ns = if sched && !hop {
            (len / s.sample.max(1e-9)).ceil().max(1.0) as usize
        } else {
            1
        };
        let per = ((len / ns as f64) / l).ceil().max(1.0) as usize;
        let ends = pieces_n(at, mv, ns * per);
        let mut a = at;
        for k in 0..ns {
            let mut worst: Option<Engage> = None;
            for &b in &ends[k * per..(k + 1) * per] {
                let dxy = ((b[0] - a[0]) * (b[0] - a[0]) + (b[1] - a[1]) * (b[1] - a[1])).sqrt();
                if sched && dxy > 0.0 {
                    let u = [(b[0] - a[0]) / dxy, (b[1] - a[1]) / dxy];
                    // the piece's own cut is what lies ahead of its start (read before it is
                    // swept); its end reads what lies ahead of it: both count, so a sample never
                    // takes the air feed for a piece that cuts (a short move into material)
                    for c in [a, b] {
                        if let Some(e) = map.engage_in(c, u, r, EPS_Z, Some(&tiles)) {
                            worst = Some(match worst {
                                Some(w) if w.cos_max > e.cos_max => Engage {
                                    ap: w.ap.max(e.ap),
                                    ae: w.ae.max(e.ae),
                                    ..w
                                },
                                Some(w) => Engage {
                                    ap: w.ap.max(e.ap),
                                    ae: w.ae.max(e.ae),
                                    ..e
                                },
                                None => e,
                            });
                        }
                    }
                }
                map.sweep_in(a, b, prof, Some(&mut tiles));
                a = b;
            }
            if sched {
                out.push(Sample {
                    mv: mi,
                    end: ends[(k + 1) * per - 1],
                    s: arc + len * (k + 1) as f64 / ns as f64,
                    engage: worst,
                });
            }
        }
        arc += len;
        at = to;
    }
    out
}

/// The path with each scheduled move's feed set per fixed-length sample from the engagement
/// it meets in `map` (left as the path cut it): fz / max cos θ over the sample and the start of
/// the next (the chip never above fz), at most cap · fz, the feed within [f_min, f_max]. A move
/// is split at its sample boundaries, each sample its own move (never joined). Straight plunges, moves up and rapids keep their feeds,
/// and a sample that meets no material keeps its move's own feed where that is the higher (a
/// link through cleared air set to run fast). The caller keeps f_max
/// under the machine's limit (G5 checks the posted F soundly, so the limit itself may fail).
/// The feed (mm/min) `schedule` gives each sample: the chip rule per sample (fz over the
/// sample's max cos θ, at most `cap` fz), no faster than the next sample, within the machine's
/// range. One place for it: `schedule` posts these, and whatever integrates over the samples
/// (tool life) reads them here.
pub fn sample_feeds(samples: &[Sample], s: &Sched) -> Vec<f64> {
    // the chip rule per sample
    let want = |e: &Option<Engage>| -> f64 {
        match e {
            Some(g) if g.cos_max * s.cap > 1.0 => s.fz / g.cos_max,
            _ => s.cap * s.fz,
        }
    };
    let n = samples.len();
    let mut feed: Vec<f64> = Vec::with_capacity(n);
    for k in 0..n {
        let mut fz = want(&samples[k].engage);
        // no faster than the next sample of the same run of feed moves
        if let Some(nx) = samples.get(k + 1) {
            fz = fz.min(want(&nx.engage));
        }
        feed.push((fz * s.flutes * s.rpm).clamp(s.f_min, s.f_max.max(s.f_min)));
    }
    feed
}

pub fn schedule(path: &OpPath, map: &mut HeightMap, prof: &Profile, s: &Sched) -> OpPath {
    let samples = schedule_samples(path, map, prof, s);
    let feed = sample_feeds(&samples, s);
    let n = samples.len();
    let mut out = Vec::with_capacity(path.moves.len() + n);
    let mut k = 0;
    for (mi, mv) in path.moves.iter().enumerate() {
        let start = k;
        while k < n && samples[k].mv == mi {
            k += 1;
        }
        if start == k {
            out.push(*mv);
            continue;
        }
        // the move's own feed, where it was set above the chip rule's (a link through cleared
        // air: nothing to cut, so no chip to hold)
        let own = match *mv {
            PMove::Line { f, .. } | PMove::Arc { f, .. } => f,
            _ => 0.0,
        };
        for i in start..k {
            let (end, mut f) = (samples[i].end, feed[i]);
            // (any contact at all keeps the chip rule: a link that grazes stock outside the
            // region it was tested against must not touch it at the link feed)
            if samples[i].engage.is_none() && own > f {
                f = own.min(s.f_max.max(s.f_min));
            }
            out.push(match *mv {
                PMove::Line { phase, .. } => PMove::Line { to: end, f, phase },
                PMove::Arc { c, cw, .. } => PMove::Arc { to: end, c, cw, f },
                other => other,
            });
        }
    }
    OpPath {
        tool: path.tool,
        start: path.start,
        moves: out,
        rpm: path.rpm,
        coolant: path.coolant,
    }
}

/// As `schedule`, but each move keeps its own geometry whole and takes ONE feed: the lowest its
/// samples call for (so the chip stays within fz over all of it). A move that meets no material
/// in any sample keeps its own feed where that is higher. For paths built of long arcs and lines
/// (adaptive): no block is split, and F changes only between moves, so the control is not asked
/// to slow for an F change inside a curve.
pub fn schedule_per_move(path: &OpPath, map: &mut HeightMap, prof: &Profile, s: &Sched) -> OpPath {
    let samples = schedule_samples(path, map, prof, s);
    let feed = sample_feeds(&samples, s);
    let n = samples.len();
    let mut out = Vec::with_capacity(path.moves.len());
    let mut k = 0;
    for (mi, mv) in path.moves.iter().enumerate() {
        let start = k;
        while k < n && samples[k].mv == mi {
            k += 1;
        }
        if start == k {
            out.push(*mv);
            continue;
        }
        let own = match *mv {
            PMove::Line { f, .. } | PMove::Arc { f, .. } => f,
            _ => 0.0,
        };
        let air = samples[start..k].iter().all(|sm| sm.engage.is_none());
        let lo = feed[start..k].iter().copied().fold(f64::INFINITY, f64::min);
        let f = if air && own > lo {
            own.min(s.f_max.max(s.f_min))
        } else {
            lo
        };
        out.push(match *mv {
            PMove::Line { to, phase, .. } => PMove::Line { to, f, phase },
            PMove::Arc { to, c, cw, .. } => PMove::Arc { to, c, cw, f },
            other => other,
        });
    }
    OpPath {
        tool: path.tool,
        start: path.start,
        moves: out,
        rpm: path.rpm,
        coolant: path.coolant,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sched(sample: f64) -> Sched {
        Sched {
            fz: 0.05,
            flutes: 3.0,
            rpm: 10_000.0,
            f_max: 10_000.0,
            f_min: 100.0,
            cap: 3.0,
            cell: 0.05,
            sample,
        }
    }

    fn slot_then_side() -> OpPath {
        let line = |to: [f64; 3]| PMove::Line {
            to,
            f: 1000.0,
            phase: Phase::Cut,
        };
        OpPath {
            tool: 0,
            start: [-6.0, 10.0, 8.0],
            moves: vec![
                line([46.0, 10.0, 8.0]),
                line([46.0, 9.0, 8.0]),
                line([-6.0, 9.0, 8.0]),
            ],
            rpm: 10_000.0,
            coolant: forge_post::Coolant::Flood,
        }
    }

    fn feeds(p: &OpPath) -> Vec<(f64, [f64; 3])> {
        p.moves
            .iter()
            .filter_map(|m| {
                if let PMove::Line { f, to, .. } = m {
                    Some((*f, *to))
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn a_slot_then_a_side_cut() {
        // a 40 x 20 block 10 high, a 10 mm flat end mill: a slot along y = 10 then a pass 1 mm
        // into its wall at y = 9 (ae/D = 0.1)
        let ip = InProcess::new([0.0, 0.0, 0.0], [40.0, 20.0, 10.0]);
        let mut m = HeightMap::of_stock(&ip, 0.05);
        let out = schedule(
            &slot_then_side(),
            &mut m,
            &Profile::Bull { r: 5.0, re: 0.0 },
            &sched(2.0),
        );
        let f = feeds(&out);
        // in the slot: fz exactly (1500 mm/min); the side cut: sin φ = 0.6, fz / 0.6 ≈ 2500
        // (the edge read a little outside the tool: a little less); in air: 3 fz = 4500
        let slot = 0.05 * 3.0 * 10_000.0;
        assert!(
            f.iter()
                .all(|(v, _)| *v >= slot - 1e-9 && *v <= 3.0 * slot + 1e-9),
            "{f:?}"
        );
        assert!(f.iter().any(|(v, _)| *v == slot), "{f:?}");
        assert!(
            f.iter()
                .any(|(v, _)| *v > 2.2 * slot / 1.5 && *v < 2.6 * slot / 1.5),
            "{f:?}"
        );
    }

    /// Sample boundaries are fixed by length alone,
    /// whatever the engagement. Every engaged sample of the 52 mm moves ends at a multiple of
    /// 52 / 26 = 2 mm along it, and a run of engagement does not change where they fall.
    #[test]
    fn samples_are_fixed_length_whatever_the_engagement() {
        let p = slot_then_side();
        let prof = Profile::Bull { r: 5.0, re: 0.0 };
        let ip = InProcess::new([0.0, 0.0, 0.0], [40.0, 20.0, 10.0]);
        let mut m = HeightMap::of_stock(&ip, 0.05);
        let air = InProcess::new([100.0, 100.0, 0.0], [101.0, 101.0, 1.0]);
        let mut m_air = HeightMap::of_stock(&air, 0.05);
        let a = schedule_samples(&p, &mut m, &prof, &sched(2.0));
        let b = schedule_samples(&p, &mut m_air, &prof, &sched(2.0));
        let ends = |v: &[Sample]| v.iter().map(|x| (x.mv, x.end)).collect::<Vec<_>>();
        assert_eq!(ends(&a), ends(&b));
        for x in a.iter().filter(|x| x.mv == 0) {
            let along = x.end[0] + 6.0;
            assert!((along / 2.0 - (along / 2.0).round()).abs() < 1e-9, "{x:?}");
        }
        // every sample its own move, whatever the feeds (all at the cap in air): never joined
        let out = schedule(&p, &mut HeightMap::of_stock(&air, 0.05), &prof, &sched(2.0));
        assert_eq!(out.moves.len(), b.len());
    }

    /// `sweep_in` with tiles is `sweep`, to the bit (map and volume), move after move: flat,
    /// bull and ball ends, level, ramped and plunging moves, over a stock cut unevenly. Fails if
    /// a tile's maximum is ever below one of its cells (e.g. `tile_max` taking the least).
    #[test]
    fn tiles_change_nothing() {
        let mut seed = 11u64;
        let mut rnd = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 11) as f64) / ((1u64 << 53) as f64)
        };
        let profs = [
            Profile::Bull { r: 5.0, re: 0.0 },
            Profile::Bull { r: 3.0, re: 1.0 },
            Profile::Ball { r: 2.0 },
            Profile::Bull { r: 12.0, re: 0.0 },
        ];
        let mut a = HeightMap::new([0.0, 0.0, 0.0], [37.3, 29.1, 10.0], 0.1);
        let mut b = HeightMap::new([0.0, 0.0, 0.0], [37.3, 29.1, 10.0], 0.1);
        let mut tiles = Tiles::of(&b);
        let mut skipped = 0usize;
        let mut nones = 0usize;
        for k in 0..400 {
            let p = &profs[k % profs.len()];
            let z0 = 10.5 - 11.0 * rnd();
            let z1 = if k % 3 == 0 { z0 } else { 10.5 - 11.0 * rnd() };
            let s = [rnd() * 45.0 - 4.0, rnd() * 37.0 - 4.0, z0];
            let e = if k % 7 == 0 {
                [s[0], s[1], z1]
            } else {
                [s[0] + rnd() * 12.0 - 6.0, s[1] + rnd() * 12.0 - 6.0, z1]
            };
            skipped += tiles
                .max
                .iter()
                .filter(|m| **m <= s[2].min(e[2]).max(b.bottom))
                .count();
            let ra = a.sweep(s, e, p);
            let rb = b.sweep_in(s, e, p, Some(&mut tiles));
            assert_eq!(ra.to_bits(), rb.to_bits(), "move {k}: {ra} vs {rb}");
            assert!(
                a.z.iter().zip(&b.z).all(|(x, y)| x.to_bits() == y.to_bits()),
                "move {k}: the maps differ"
            );
            // the edge read through the tiles is the edge read cell by cell, at the move's end
            // and at random places and heights, inside the map and over its edges
            for q in 0..6 {
                let c = if q == 0 {
                    e
                } else {
                    [rnd() * 49.0 - 6.0, rnd() * 41.0 - 6.0, 11.0 - 12.0 * rnd()]
                };
                let th = rnd() * std::f64::consts::TAU;
                let (sn, cs) = dmath::sin_cos(th);
                let r = p.r();
                let want = b.engage(c, [cs, sn], r, 1e-3);
                let got = b.engage_in(c, [cs, sn], r, 1e-3, Some(&tiles));
                if want.is_none() {
                    nones += 1;
                }
                assert_eq!(format!("{want:?}"), format!("{got:?}"), "move {k}, place {q}");
            }
        }
        assert!(nones > 50, "too few places with nothing engaged: {nones}");
        assert!(
            skipped > 0,
            "no tile was ever passed over: the test shows nothing"
        );
    }

    /// The edge table is what `engage` computed at each call: θ_k = −π/2 + k·π/180 and its
    /// sine and cosine (dmath), bit for bit.
    #[test]
    fn the_edge_table_is_the_edge() {
        let step = std::f64::consts::PI / 180.0;
        for (k, &(th, s, co)) in edge().iter().enumerate() {
            let t = -std::f64::consts::FRAC_PI_2 + k as f64 * step;
            let (ws, wc) = dmath::sin_cos(t);
            assert_eq!(
                (th.to_bits(), s.to_bits(), co.to_bits()),
                (t.to_bits(), ws.to_bits(), wc.to_bits())
            );
        }
    }

    /// The body's true height at (x, y) after a → b (the tool's tip at fine positions along it),
    /// or `old` where the tool never covers the point.
    fn true_height(old: f64, a: [f64; 3], b: [f64; 3], p: &Profile, x: f64, y: f64) -> f64 {
        let mut z = old;
        // (positions 0.05 mm apart at most along these moves: finer than the cells' h/√2 margin)
        for k in 0..=400 {
            let t = k as f64 / 400.0;
            let c = [
                a[0] + t * (b[0] - a[0]),
                a[1] + t * (b[1] - a[1]),
                a[2] + t * (b[2] - a[2]),
            ];
            let d = ((x - c[0]).powi(2) + (y - c[1]).powi(2)).sqrt();
            if d <= p.r() {
                z = z.min(c[2] + p.at(d));
            }
        }
        z
    }

    #[test]
    fn sweep_under_never_brings_a_cell_below_what_it_still_holds() {
        let profs = [
            Profile::Bull { r: 3.0, re: 0.0 },
            Profile::Bull { r: 3.0, re: 1.0 },
            Profile::Ball { r: 3.0 },
        ];
        let moves = [
            ([2.0, 5.13, 4.0], [17.7, 5.13, 4.0]),
            ([3.1, 2.9, 6.0], [15.2, 11.4, 2.5]),
            ([9.03, 7.71, 5.0], [9.03, 7.71, 5.0]),
        ];
        for p in &profs {
            for &(a, b) in &moves {
                let mut under = HeightMap::new([0.0, 0.0, 0.0], [20.0, 15.0, 10.0], 0.25);
                let mut plain = under.clone();
                under.sweep_under(a, b, p);
                plain.sweep(a, b, p);
                let mut lowered = 0;
                for j in 0..under.ny {
                    for i in 0..under.nx {
                        let k = j * under.nx + i;
                        assert!(under.z[k] >= plain.z[k] - 1e-12, "below sweep at {i} {j}");
                        if under.z[k] >= 10.0 {
                            continue;
                        }
                        lowered += 1;
                        // every point of the cell: still at or below the cell's height
                        for v in 0..=3 {
                            for u in 0..=3 {
                                let x = under.x0 + (i as f64 + u as f64 / 3.0) * under.h;
                                let y = under.y0 + (j as f64 + v as f64 / 3.0) * under.h;
                                let t = true_height(10.0, a, b, p, x, y);
                                assert!(
                                    t <= under.z[k] + 1e-9,
                                    "{p:?} {a:?}->{b:?}: cell {i} {j} at {} holds {t}",
                                    under.z[k]
                                );
                            }
                        }
                    }
                }
                assert!(lowered > 100, "{p:?} {a:?}->{b:?}: only {lowered} cells lowered");
            }
        }
    }

    #[test]
    fn a_path_swept_is_its_moves_swept() {
        // a line then a half circle (ccw, centre (10, 0), radius 5) with a 2 mm ball 1 deep
        let p = Profile::Ball { r: 2.0 };
        let mut m = HeightMap::new([-10.0, -10.0, -5.0], [30.0, 10.0, 0.0], 0.1);
        let moves = [
            PMove::Line {
                to: [5.0, 0.0, -1.0],
                f: 100.0,
                phase: Phase::Cut,
            },
            PMove::Arc {
                to: [15.0, 0.0, -1.0],
                c: [10.0, 0.0],
                cw: false,
                f: 100.0,
            },
        ];
        m.sweep_path([0.0, 0.0, -1.0], &moves, &p);
        let at = |x: f64, y: f64| m.top(x, y);
        // under the line and the arc's lowest point (10, −5): the tip's depth, within a cell
        assert!((at(2.05, 0.05) + 1.0).abs() < 0.01, "{}", at(2.05, 0.05));
        assert!((at(10.05, -4.95) + 1.0).abs() < 0.01, "{}", at(10.05, -4.95));
        // the arc's far side (10, 5) and the circle's centre: untouched
        assert_eq!(at(10.05, 5.05), 0.0);
        assert_eq!(at(10.05, 0.05), 0.0);
    }

    #[test]
    fn lower_voids_under_lowers_only_cells_wholly_inside() {
        let c = forge_cam_geom::region::P2 { x: 8.37, y: 7.11 };
        let a = forge_cam_geom::region::P2 { x: c.x + 5.3, y: c.y };
        let circle = forge_cam_geom::region::Loop {
            segs: vec![Seg::Arc {
                a,
                b: a,
                c,
                ccw: true,
            }],
        };
        let ip = InProcess {
            min: [0.0, 0.0, 0.0],
            max: [20.0, 15.0, 10.0],
            voids: vec![(Region { loops: vec![circle] }, 4.0)],
            under: Vec::new(),
        };
        let m = HeightMap::of_stock_under(&ip, 0.25);
        let plain = HeightMap::of_stock(&ip, 0.25);
        let (mut n, mut n_plain) = (0, 0);
        for j in 0..m.ny {
            for i in 0..m.nx {
                let k = j * m.nx + i;
                n_plain += (plain.z[k] < 10.0) as usize;
                if m.z[k] >= 10.0 {
                    continue;
                }
                n += 1;
                for v in 0..=4 {
                    for u in 0..=4 {
                        let x = m.x0 + (i as f64 + u as f64 / 4.0) * m.h;
                        let y = m.y0 + (j as f64 + v as f64 / 4.0) * m.h;
                        assert!(
                            ((x - c.x).powi(2) + (y - c.y).powi(2)).sqrt() <= 5.3 + 1e-9,
                            "cell {i} {j} pokes out"
                        );
                    }
                }
            }
        }
        // all but the rim: the disk of radius 5.3 − h/√2 − h holds every centre lowered by plain
        // whose cell is wholly inside, near enough
        assert!(
            n > 0 && n < n_plain && (n as f64) > 0.85 * n_plain as f64,
            "{n} of {n_plain}"
        );
    }
}

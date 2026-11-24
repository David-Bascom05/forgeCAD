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
        for (rg, floor) in &ip.voids {
            m.lower_region(rg, *floor);
        }
        m
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
                }
            }
        }
        removed * self.h * self.h
    }

    /// What the side edge's front half meets at tip c moving along u (θ from the direction of
    /// travel, + left): the stock standing above the tip by more than `eps`, the edge read at
    /// r + h/2, a little outside the tool, so the cells' error errs toward more engagement.
    /// None: the edge meets nothing.
    pub fn engage(&self, c: [f64; 3], u: [f64; 2], r: f64, eps: f64) -> Option<Engage> {
        let rr = r + 0.5 * self.h;
        let n = 181;
        let step = std::f64::consts::PI / (n - 1) as f64;
        let mut e: Option<Engage> = None;
        for k in 0..n {
            let th = -std::f64::consts::FRAC_PI_2 + k as f64 * step;
            let (s, co) = dmath::sin_cos(th);
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
    let mut out = Vec::new();
    let mut at = path.start;
    let mut arc = 0.0;
    for (mi, mv) in path.moves.iter().enumerate() {
        let to = match *mv {
            PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => to,
            PMove::Cycle { .. } => continue,
        };
        let len = move_len(at, mv);
        let sched = scheduled(at, mv);
        let ns = if sched {
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
                    if let Some(e) = map.engage(b, u, r, EPS_Z) {
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
                map.sweep(a, b, prof);
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
            // (contact narrower than a cell of the model is its rounding: a link along a wall it
            // just cut touches it at exactly r)
            if samples[i].engage.is_none_or(|e| e.ae < s.cell) && own > f {
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
}

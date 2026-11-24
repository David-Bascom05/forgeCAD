//! Feeds from engagement: the stock a path meets, modelled as a height map, and the feed of each
//! piece of the path set so that the chip the edge cuts (after radial thinning) is the cutting
//! data's feed per tooth, never more.
//!
//! This is the offline feed optimisation of GE's US 5,828,574 (1997, expired 2017-05-19): the
//! swept tool is taken from a model of the stock, the engagement of each move measured, and the
//! largest feed within the limit chosen (research/cam/patents.md §2.8). The path's geometry is
//! unchanged: a move is only split where its feed changes (lines into collinear pieces, arcs and
//! helices into pieces of the same circle), and what is posted is certified as before. Feeds are
//! no part of G1–G4; G5 checks every posted feed against the machine.
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

    /// The largest cos θ over the side edge's front half (θ from the direction of travel u)
    /// where the stock stands above the tip c by more than `eps`: the chip there is fz · that.
    /// None: the edge meets nothing. The edge is read at r + h/2, a little outside the tool: the
    /// cells' error then errs toward more engagement (a lower feed).
    pub fn chip_factor(&self, c: [f64; 3], u: [f64; 2], r: f64, eps: f64) -> Option<f64> {
        let rr = r + 0.5 * self.h;
        let n = 181;
        let step = std::f64::consts::PI / (n - 1) as f64;
        let mut best: Option<f64> = None;
        for k in 0..n {
            let th = -std::f64::consts::FRAC_PI_2 + k as f64 * step;
            let (s, co) = dmath::sin_cos(th);
            let dx = u[0] * co - u[1] * s;
            let dy = u[0] * s + u[1] * co;
            if self.top(c[0] + rr * dx, c[1] + rr * dy) > c[2] + eps {
                best = Some(best.map_or(co, |b: f64| b.max(co)));
            }
        }
        best
    }
}

/// What the feeds are set from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sched {
    /// The cutting data's feed per tooth for the tool (the chip to cut), mm.
    pub fz: f64,
    pub flutes: f64,
    pub rpm: f64,
    /// The machine's largest feed, mm/min.
    pub f_max: f64,
    /// The most chip thinning may raise the feed per tooth by (forge_tooling::feeds::THIN_CAP).
    pub cap: f64,
    /// Cell size of the model, mm.
    pub cell: f64,
}

/// Steps the feed per tooth is quantised to: fz · STEP^k, k = 0, 1, .. (rounded down, so the
/// chip is never above fz).
const STEP: f64 = 1.15;
/// Stock this far above the tip counts as material (mm).
const EPS_Z: f64 = 1e-3;

/// A move's pieces: their ends (the last the move's end).
fn pieces(from: [f64; 3], m: &PMove, l: f64) -> Vec<[f64; 3]> {
    let d3 = |a: [f64; 3], b: [f64; 3]| {
        ((a[0] - b[0]) * (a[0] - b[0]) + (a[1] - b[1]) * (a[1] - b[1]) + (a[2] - b[2]) * (a[2] - b[2])).sqrt()
    };
    match *m {
        PMove::Rapid { to } | PMove::Line { to, .. } => {
            let n = (d3(from, to) / l).ceil().max(1.0) as usize;
            (1..=n)
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
                .collect()
        }
        PMove::Arc { to, c, cw, .. } => {
            let r = ((from[0] - c[0]) * (from[0] - c[0]) + (from[1] - c[1]) * (from[1] - c[1])).sqrt();
            let a0 = dmath::atan2(from[1] - c[1], from[0] - c[0]);
            let a1 = dmath::atan2(to[1] - c[1], to[0] - c[0]);
            let sw = sweep(a0, a1, !cw);
            let n = ((sw.abs() * r).max(d3(from, to)) / l).ceil().max(1.0) as usize;
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

/// A move's piece ends and each piece's largest allowed feed per tooth (None: not scheduled).
type Plan = (Vec<[f64; 3]>, Vec<Option<f64>>);

/// The path with each feed move's feed set from the engagement it meets in `map` (which it
/// leaves as the path cut it). Straight plunges, moves up and rapids keep their feeds.
pub fn schedule(path: &OpPath, map: &mut HeightMap, prof: &Profile, s: &Sched) -> OpPath {
    let r = prof.r();
    let l = s.cell.max((0.25 * r).min(0.5));
    // pass 1: per move, its piece ends and each piece's largest allowed feed (None: not
    // scheduled)
    let mut at = path.start;
    let mut plan: Vec<Plan> = Vec::with_capacity(path.moves.len());
    for mv in &path.moves {
        let to = match *mv {
            PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => to,
            PMove::Cycle { .. } => {
                plan.push((vec![], vec![]));
                continue;
            }
        };
        let ends = pieces(at, mv, l);
        let scheduled = match *mv {
            PMove::Line { phase, .. } => {
                phase != Phase::Rapid && (to[0] != at[0] || to[1] != at[1]) && to[2] <= at[2]
            }
            PMove::Arc { .. } => to[2] <= at[2],
            _ => false,
        };
        let mut req = Vec::with_capacity(ends.len());
        let mut a = at;
        for &b in &ends {
            let dxy = ((b[0] - a[0]) * (b[0] - a[0]) + (b[1] - a[1]) * (b[1] - a[1])).sqrt();
            if scheduled && dxy > 0.0 {
                let u = [(b[0] - a[0]) / dxy, (b[1] - a[1]) / dxy];
                let fz = match map.chip_factor(b, u, r, EPS_Z) {
                    // hex = fz cos θ ≤ the data's fz
                    Some(c) if c * s.cap > 1.0 => s.fz / c,
                    _ => s.cap * s.fz,
                };
                req.push(Some(fz));
            } else {
                req.push(None);
            }
            map.sweep(a, b, prof);
            a = b;
        }
        plan.push((ends, req));
        at = to;
    }
    // pass 2: each piece no faster than the next one (the engagement it runs into), quantised
    // down; then pieces of one move at the same feed merged
    let flat: Vec<Option<f64>> = plan.iter().flat_map(|(_, r)| r.iter().copied()).collect();
    let mut fz_of = vec![None; flat.len()];
    for k in 0..flat.len() {
        if let Some(f) = flat[k] {
            let next = flat.get(k + 1).copied().flatten().unwrap_or(f);
            let f = f.min(next);
            let steps = (dmath::ln(f / s.fz) / dmath::ln(STEP) + 1e-9).floor().max(0.0);
            fz_of[k] = Some(s.fz * dmath::powf(STEP, steps));
        }
    }
    let mut out = Vec::with_capacity(path.moves.len());
    let mut k = 0;
    for (mv, (ends, _)) in path.moves.iter().zip(&plan) {
        let n = ends.len();
        if n == 0 || fz_of[k..k + n].iter().all(|f| f.is_none()) {
            out.push(*mv);
            k += n;
            continue;
        }
        // just under the machine's limit: G5 checks the posted F against it soundly, so a feed
        // at the limit itself may not pass
        let feed = |fz: f64| (fz * s.flutes * s.rpm).min(0.999 * s.f_max);
        let mut i = 0;
        while i < n {
            let fz = fz_of[k + i].unwrap_or(s.fz);
            let mut j = i;
            while j + 1 < n && fz_of[k + j + 1].unwrap_or(s.fz) == fz {
                j += 1;
            }
            let end = ends[j];
            out.push(match *mv {
                PMove::Line { phase, .. } => PMove::Line {
                    to: end,
                    f: feed(fz),
                    phase,
                },
                PMove::Arc { c, cw, .. } => PMove::Arc {
                    to: end,
                    c,
                    cw,
                    f: feed(fz),
                },
                other => other,
            });
            i = j + 1;
        }
        k += n;
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

    #[test]
    fn a_slot_then_a_side_cut() {
        // a 40 x 20 block 10 high, a 10 mm flat end mill: a slot along y = 10 then a pass 1 mm
        // into its wall at y = 9 (ae/D = 0.1)
        let ip = InProcess::new([0.0, 0.0, 0.0], [40.0, 20.0, 10.0]);
        let mut m = HeightMap::of_stock(&ip, 0.05);
        let prof = Profile::Bull { r: 5.0, re: 0.0 };
        let s = Sched {
            fz: 0.05,
            flutes: 3.0,
            rpm: 10_000.0,
            f_max: 10_000.0,
            cap: 3.0,
            cell: 0.05,
        };
        let path = OpPath {
            tool: 0,
            start: [-6.0, 10.0, 8.0],
            moves: vec![
                PMove::Line {
                    to: [46.0, 10.0, 8.0],
                    f: 1000.0,
                    phase: Phase::Cut,
                },
                PMove::Line {
                    to: [46.0, 9.0, 8.0],
                    f: 1000.0,
                    phase: Phase::Cut,
                },
                PMove::Line {
                    to: [-6.0, 9.0, 8.0],
                    f: 1000.0,
                    phase: Phase::Cut,
                },
            ],
            rpm: 10_000.0,
            coolant: forge_post::Coolant::Flood,
        };
        let out = schedule(&path, &mut m, &prof, &s);
        let feeds: Vec<f64> = out
            .moves
            .iter()
            .filter_map(|m| {
                if let PMove::Line { f, .. } = m {
                    Some(*f)
                } else {
                    None
                }
            })
            .collect();
        // in the slot: fz exactly (1500 mm/min); the side cut: sin φ = 0.6, so fz / 0.6 = 1.67 fz,
        // quantised down to 1.15^3 = 1.52 fz (2281 mm/min); in air: 3 fz, 1.15^7 = 2.66 fz
        let slot = 0.05 * 3.0 * 10_000.0;
        assert!(
            feeds.iter().all(|f| *f >= slot - 1e-9 && *f <= 3.0 * slot + 1e-9),
            "{feeds:?}"
        );
        assert!(feeds.contains(&slot), "{feeds:?}");
        let side = slot * dmath::powf(STEP, 3.0);
        assert!(feeds.iter().any(|f| (f - side).abs() < 1e-6), "{feeds:?}");
    }
}

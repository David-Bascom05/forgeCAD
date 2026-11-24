//! Adaptive clearing: a 2.5D roughing path whose radial engagement is held under ONE capped
//! target by simulating the cleared area step by step.
//!
//! **Patent design-around: read before changing anything here.** This module is built strictly
//! to research/cam/toolpath-quality/fto-adaptive.md §3 (candidates A + C + D + E, B only plain),
//! per the coordinator's rules of 2026-11-24 (engineering reading, not legal advice):
//! 1. A cleared-area simulation steers each step to ONE capped engagement target (`Adaptive::
//!    phi_max`): the largest turn into material whose engagement does not exceed the cap. There is
//!    NO minimum engagement, NO band, and no designed variation. The output is OPEN passes joined by
//!    links, never spiral loops built to a cyclic engagement pattern (SolidCAM US 8,489,224 /
//!    9,823,645 / 10,895,861 / 10,324,445). A generated step is never revised afterwards
//!    (Celeritive US 10,108,176; Delcam/Autodesk US 9,489,339).
//! 2. Links by a FIXED rule: stay down on a straight move that lies in the cleared area, else
//!    retract. No transition toolpaths or areas, no trimming of cuts against them (Celeritive
//!    US 8,295,972 / 8,880,212), no choice between link kinds or between trochoid and spiral by
//!    estimated time (SolidCAM US 12,248,293, US 2025/0284264).
//! 3. Feed: not set here; forge_cam::engage sets it per fixed-length sample (MERL US 9,892,215).
//! 4. Islands are met by the same simulation as they come: NO moat pass around an island
//!    (SolidCAM '224 cl.45).
//! 5. Only plain offsets (the final wall pass) and no Voronoi diagram (Celeritive US 10,579,040;
//!    Autodesk US 10,108,172).
//! 6. No corner-radius rule from predicted engagement (DP Technology US 9,778,649); paths are
//!    the simulation's steps. Never the two-tangent-arc pass into a three-sided region (Truemill
//!    US 7,577,490).
//!
//! Since fto-adaptive.md §8 (TOOLPATH-RESEARCH, 2026-11-24): the step rule alone, in a closed
//! region, makes an outward spiral, which SolidCAM's '645/'861/'445 machine claims may read on
//! however it was computed. So every pass is a one-direction "peel" (§8 (b)): its heading stays
//! within `PEEL` of one sweep direction, it never winds round a centre or an island, and it ends
//! at the boundary or the cap; the next starts by the fixed link rule.
//!
//! The tests `single_capped_target` and `passes_peel_one_way` assert rule 1 and the peel; keep
//! them passing.
//!
//! Certification: the path is posted and certified like every other (G1–G3 on the posted
//! path). What the op claims cleared (its voids, for the in-process stock and G4) is not taken
//! from the simulation: a cell is claimed only when every point of it lies within the flat of
//! the tool's end from a move at that level (its centre within r_flat − h/√2 − margin of the
//! move, by an f64 distance whose rounding error is far below the margin), so the claim is
//! inside what the tool sweeps.
use crate::engage::{HeightMap, Profile};
use crate::ops::{Cut, OpError, OpPath, PMove, Phase, Span, levels};
use crate::stock::InProcess;
use forge_cam_geom::offset::offset;
use forge_cam_geom::region::{Loop, Region, Seg, p2};
use forge_math::dmath;

/// What the user sets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Adaptive {
    /// The cap on the radial engagement angle, degrees (the single target).
    pub phi_max: f64,
    /// Step length along a pass, mm (0: a quarter of the tool's radius).
    pub step: f64,
    /// Stock left on the walls, mm.
    pub leave: f64,
    /// A plain offset pass along the walls at the end of each level.
    pub wall_pass: bool,
    /// The sweep direction, degrees from +x: every pass heads within `PEEL` of it (one-direction
    /// "peel" passes, fto-adaptive.md §8 (b): a pass never winds round a centre).
    pub sweep: f64,
}

/// The most a pass's heading may turn from the sweep direction (either way).
pub const PEEL: f64 = 75.0;

/// The op's result: the path, and per level the cells it provably cleared (floor z).
pub struct AdaptiveOut {
    pub path: OpPath,
    pub voids: Vec<(Region, f64)>,
}

/// A closed loop as a polygon, arcs as chords whose sagitta is at most `sag`.
fn polygon(segs: &[Seg], sag: f64) -> Vec<(f64, f64)> {
    let mut pts = Vec::new();
    for sg in segs {
        match *sg {
            Seg::Line { a, .. } => pts.push((a.x, a.y)),
            Seg::Arc { a, b, c, ccw } => {
                let r = ((a.x - c.x) * (a.x - c.x) + (a.y - c.y) * (a.y - c.y)).sqrt();
                let a0 = dmath::atan2(a.y - c.y, a.x - c.x);
                let a1 = dmath::atan2(b.y - c.y, b.x - c.x);
                let tau = std::f64::consts::TAU;
                let mut sw = a1 - a0;
                if ccw {
                    while sw <= 0.0 {
                        sw += tau;
                    }
                } else {
                    while sw >= 0.0 {
                        sw -= tau;
                    }
                }
                // sagitta r (1 − cos(θ/2)) ≤ sag: θ ≤ 2 acos(1 − sag/r)
                let th = 2.0 * dmath::acos_clamped(1.0 - (sag / r.max(1e-9)).min(1.0));
                let n = ((sw.abs() / th.max(1e-6)).ceil() as usize).clamp(1, 100_000);
                for k in 0..n {
                    let (sn, cs) = dmath::sin_cos(a0 + sw * k as f64 / n as f64);
                    pts.push((c.x + r * cs, c.y + r * sn));
                }
            }
        }
    }
    pts
}

/// Even–odd inside over a set of polygons.
fn inside(polys: &[Vec<(f64, f64)>], x: f64, y: f64) -> bool {
    let mut c = false;
    for p in polys {
        let n = p.len();
        for i in 0..n {
            let (x0, y0) = p[i];
            let (x1, y1) = p[(i + 1) % n];
            if (y0 > y) != (y1 > y) && x < x0 + (y - y0) * (x1 - x0) / (y1 - y0) {
                c = !c;
            }
        }
    }
    c
}

/// Distance from (x, y) to the polygons' edges.
fn edge_dist(polys: &[Vec<(f64, f64)>], x: f64, y: f64) -> f64 {
    let mut best = f64::INFINITY;
    for p in polys {
        let n = p.len();
        for i in 0..n {
            let (x0, y0) = p[i];
            let (x1, y1) = p[(i + 1) % n];
            let (dx, dy) = (x1 - x0, y1 - y0);
            let ll = dx * dx + dy * dy;
            let t = if ll > 0.0 {
                (((x - x0) * dx + (y - y0) * dy) / ll).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let (px, py) = (x0 + t * dx, y0 + t * dy);
            best = best.min(((x - px) * (x - px) + (y - py) * (y - py)).sqrt());
        }
    }
    best
}

fn rot(u: [f64; 2], a: f64) -> [f64; 2] {
    let (s, c) = dmath::sin_cos(a);
    [u[0] * c - u[1] * s, u[0] * s + u[1] * c]
}

/// The tool-centre region (allowed), the reachable target (work) and the stock map.
struct Field {
    allowed: Vec<Vec<(f64, f64)>>,
    work: Vec<Vec<(f64, f64)>>,
    map: HeightMap,
    prof: Profile,
    r: f64,
    eps: f64,
}

impl Field {
    fn ok(&self, x: f64, y: f64) -> bool {
        inside(&self.allowed, x, y)
    }

    /// Material at the level to be cut (inside the reachable target, above z).
    fn material(&self, x: f64, y: f64, z: f64) -> bool {
        self.map.top(x, y) > z + self.eps && inside(&self.work, x, y)
    }

    /// The engagement angle (radians) at c moving along u, the edge reading only material to be
    /// cut; ∞ where c is not an allowed centre.
    fn phi(&self, c: [f64; 3], u: [f64; 2]) -> f64 {
        if !self.ok(c[0], c[1]) {
            return f64::INFINITY;
        }
        let rr = self.r + 0.5 * self.map.h;
        let n = 91;
        let step = std::f64::consts::PI / (n - 1) as f64;
        let mut phi = 0.0;
        for k in 0..n {
            let th = -std::f64::consts::FRAC_PI_2 + k as f64 * step;
            let d = rot(u, th);
            if self.material(c[0] + rr * d[0], c[1] + rr * d[1], c[2]) {
                phi += if k == 0 || k == n - 1 { 0.5 * step } else { step };
            }
        }
        phi
    }

    /// Whether the tool's disk about (x, y) at z meets material to be cut (within r + h).
    fn disk_has_material(&self, x: f64, y: f64, z: f64) -> bool {
        let h = self.map.h;
        let rr = self.r + h;
        let n = (rr / h).ceil() as i64;
        for j in -n..=n {
            for i in -n..=n {
                let (dx, dy) = (i as f64 * h, j as f64 * h);
                if dx * dx + dy * dy <= rr * rr && self.material(x + dx, y + dy, z) {
                    return true;
                }
            }
        }
        false
    }

    /// The fixed link rule: the straight move a → b at z lies in the cleared area (no material
    /// within r + h of it, sampled at h) and in the allowed centres.
    fn link_clear(&self, a: [f64; 2], b: [f64; 2], z: f64) -> bool {
        let l = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let n = (l / self.map.h).ceil().max(1.0) as usize;
        (0..=n).all(|k| {
            let s = k as f64 / n as f64;
            let (x, y) = (a[0] + s * (b[0] - a[0]), a[1] + s * (b[1] - a[1]));
            self.ok(x, y) && !self.disk_has_material(x, y, z)
        })
    }
}

/// Adaptive clearing of `rg` (the region the tool may cut in, islands as holes) between the
/// span's heights, from the in-process stock `ip`.
pub fn adaptive(
    rg: &Region,
    c: Cut,
    sp: Span,
    ip: &InProcess,
    prm: &Adaptive,
) -> Result<AdaptiveOut, OpError> {
    let Cut { tool, tidx, h, feed } = c;
    let Some(prof) = Profile::of(&tool.tool) else {
        return Err(OpError::NeedTool {
            what: "an end mill (flat, bull or ball) for adaptive clearing".into(),
        });
    };
    let r = 0.5 * tool.tool.dc + prm.leave;
    let r_flat = match prof {
        Profile::Bull { r, re } => r - re,
        Profile::Ball { .. } => 0.0,
    };
    // the step length, and the margin the tool centres keep from the region's offset: a
    // straight step between two allowed centres dips across a rounded corner of the allowed
    // region (radius ≥ r) by at most ds²/(8r)
    let ds = if prm.step > 0.0 {
        prm.step
    } else {
        0.125 * tool.tool.dc
    };
    // (an arc fitted through the steps, radius ≥ r and within 0.005 of them, bulges past a
    // chord by as much again)
    let margin = 0.01 + 0.005 + 2.0 * ds * ds / (8.0 * (0.5 * tool.tool.dc));
    // allowed centres: the region shrunk by the tool radius and a margin; the reachable target:
    // that grown back by the radius (the opening)
    let (cl, _) = offset(rg, r + margin).map_err(|e| OpError::Region(format!("{e:?}")))?;
    if cl.is_empty() {
        return Err(OpError::ToolTooBig {
            tool_d: tool.tool.dc,
            why: "the tool does not fit the region".into(),
        });
    }
    let creg = forge_cam_geom::boolean::region_of(&cl);
    let (wl, _) = offset(&creg, -(r + margin)).map_err(|e| OpError::Region(format!("{e:?}")))?;
    let sag = 1e-3;
    let allowed: Vec<_> = cl.iter().map(|l| polygon(&l.segs, sag)).collect();
    let work: Vec<_> = wl.iter().map(|l| polygon(&l.segs, sag)).collect();
    let cell = (tool.tool.dc / 40.0).clamp(0.05, 0.25);
    let mut f = Field {
        allowed,
        work,
        map: HeightMap::of_stock(ip, cell),
        prof,
        r: 0.5 * tool.tool.dc,
        eps: 1e-3,
    };
    let cap = prm.phi_max.clamp(5.0, 180.0).to_radians();
    let sweep_dir = {
        let (sn, cs) = dmath::sin_cos(prm.sweep.to_radians());
        [cs, sn]
    };
    let peel_cos = dmath::cos(PEEL.to_radians());
    let mut mv: Vec<PMove> = Vec::new();
    let mut voids = Vec::new();
    // where the tool is: (x, y, z)
    let mut at: Option<[f64; 3]> = None;
    let mut start: Option<[f64; 3]> = None;
    let mut above = sp.top;
    for z in levels(sp.top, sp.bottom, sp.step) {
        let mut moves_at_level: Vec<([f64; 3], [f64; 3])> = Vec::new();
        // candidate starts: allowed centres on a grid of r/4 (fine enough to find the band beside
        // the stock that is clear of it yet inside the region)
        let g = 0.25 * f.r;
        let (lo, hi) = (ip.min, ip.max);
        let mut cands: Vec<[f64; 2]> = Vec::new();
        // out to 2r past the stock (clear's region reaches D past it): starts in the air beside it
        let mut y = lo[1] - 2.0 * f.r;
        while y <= hi[1] + 2.0 * f.r {
            let mut x = lo[0] - 2.0 * f.r;
            while x <= hi[0] + 2.0 * f.r {
                if f.ok(x, y) {
                    cands.push([x, y]);
                }
                x += g;
            }
            y += g;
        }
        let mut entered = false;
        // first, a start from the open side (cleared air next to material ahead), no entry
        let mut open_try = true;
        let mut guard = 0usize;
        let mut small = 0usize;
        // entries at this level: a new one where the passes stop with material left
        let mut entries = 0usize;
        loop {
            guard += 1;
            if guard > 10_000 {
                break;
            }
            // the next pass's start: a parked centre (its disk clear of material) next to
            // material, nearest the tool; before any entry at this level, the deepest allowed
            // point gets a helix
            let here = at
                .map(|p| [p[0], p[1]])
                .unwrap_or([0.5 * (lo[0] + hi[0]), 0.5 * (lo[1] + hi[1])]);
            let mut best: Option<([f64; 2], f64)> = None;
            if entered || open_try {
                for &p in &cands {
                    if f.disk_has_material(p[0], p[1], z) {
                        continue;
                    }
                    // material just beyond the disk
                    let near = (0..16).any(|k| {
                        let d = rot([1.0, 0.0], std::f64::consts::TAU * k as f64 / 16.0);
                        // past the disk by a grid spacing: the next candidate out may already
                        // overlap material
                        let rr = f.r + g + 2.0 * f.map.h;
                        // ahead along the sweep: a pass from here meets it going forward
                        d[0] * sweep_dir[0] + d[1] * sweep_dir[1] > 0.0
                            && f.material(p[0] + rr * d[0], p[1] + rr * d[1], z)
                    });
                    if !near {
                        continue;
                    }
                    // across the sweep from one side to the other (fto-adaptive.md §9 (2)): the
                    // lowest band of r/2 across the sweep first, the nearest within it
                    let across = (sweep_dir[0] * p[1] - sweep_dir[1] * p[0]) / (0.5 * f.r);
                    let d = across.floor() * 1e9 + (p[0] - here[0]).powi(2) + (p[1] - here[1]).powi(2);
                    if best.is_none_or(|(_, bd)| d < bd) {
                        best = Some((p, d));
                    }
                }
            } else {
                // the deepest allowed candidate with material under it
                let mut deep: Option<([f64; 2], f64)> = None;
                for &p in &cands {
                    // the first entry: anywhere with material in reach; a later one only where
                    // material stands under the centre itself (not for crumbs at a wall)
                    let here = f.material(p[0], p[1], z);
                    if if entries > 0 {
                        !here
                    } else {
                        !here && !f.disk_has_material(p[0], p[1], z)
                    } {
                        continue;
                    }
                    let d = edge_dist(&f.allowed, p[0], p[1]);
                    if deep.is_none_or(|(_, bd)| d > bd) {
                        deep = Some((p, d));
                    }
                }
                let Some((p, depth)) = deep else { break };
                // a helix of radius up to r where it fits (a hole of 2r: room to start passes
                // off its centre), else a plunge
                let rh = f.r.min((depth - 1e-3).max(0.0));
                // the helix starts just above the stock it meets (the highest in its swept disk),
                // not at the level's top: a later entry comes down through cleared air
                let top_z = {
                    let rr = rh + f.r + f.map.h;
                    let n = (rr / f.map.h).ceil() as i64;
                    let mut t = f64::NEG_INFINITY;
                    for j in -n..=n {
                        for i in -n..=n {
                            let (dx, dy) = (i as f64 * f.map.h, j as f64 * f.map.h);
                            if dx * dx + dy * dy <= rr * rr {
                                t = t.max(f.map.top(p[0] + dx, p[1] + dy));
                            }
                        }
                    }
                    (t + 0.5).clamp(z, above)
                };
                // a later entry: up from where the tool is first (a rapid only above the work)
                if let Some(c) = at
                    && c[2] < h.retract
                {
                    mv.push(PMove::Line {
                        to: [c[0], c[1], h.retract],
                        f: feed.0,
                        phase: Phase::Link,
                    });
                }
                mv.push(PMove::Rapid {
                    to: [p[0] + rh, p[1], h.retract],
                });
                mv.push(PMove::Line {
                    to: [p[0] + rh, p[1], top_z],
                    f: feed.1,
                    phase: Phase::Plunge,
                });
                let mut cur = [p[0] + rh, p[1], top_z];
                if rh > 0.25 * f.r {
                    let per_turn = std::f64::consts::TAU * rh * dmath::sin_cos(3f64.to_radians()).0
                        / dmath::sin_cos(3f64.to_radians()).1;
                    let turns = ((top_z - z) / per_turn - 1e-9).ceil().max(1.0) as usize;
                    let quarters = 4 * turns;
                    for k in 1..=quarters {
                        let (sn, cs) = dmath::sin_cos(std::f64::consts::FRAC_PI_2 * k as f64);
                        let (x, y) = if k % 4 == 0 {
                            (p[0] + rh, p[1])
                        } else {
                            (p[0] + rh * cs, p[1] + rh * sn)
                        };
                        let zk = if k == quarters {
                            z
                        } else {
                            top_z + (z - top_z) * k as f64 / quarters as f64
                        };
                        let to = [x, y, zk];
                        // the helix as chords for the model, posted as arcs
                        for q in 1..=4 {
                            let a0 = std::f64::consts::FRAC_PI_2 * (k - 1) as f64;
                            let (s2, c2) = dmath::sin_cos(a0 + std::f64::consts::FRAC_PI_2 * q as f64 / 4.0);
                            let zz = cur[2] + (zk - cur[2]) * q as f64 / 4.0;
                            let b = [p[0] + rh * c2, p[1] + rh * s2, zz];
                            let a = [
                                p[0] + rh
                                    * dmath::sin_cos(a0 + std::f64::consts::FRAC_PI_2 * (q - 1) as f64 / 4.0)
                                        .1,
                                p[1] + rh
                                    * dmath::sin_cos(a0 + std::f64::consts::FRAC_PI_2 * (q - 1) as f64 / 4.0)
                                        .0,
                                cur[2] + (zk - cur[2]) * (q - 1) as f64 / 4.0,
                            ];
                            f.map.sweep(a, b, &f.prof.clone());
                            moves_at_level.push((a, b));
                        }
                        mv.push(PMove::Arc {
                            to,
                            c: p,
                            cw: false,
                            f: feed.1,
                        });
                        cur = to;
                    }
                    // one whole turn at depth: the helix leaves a ramped floor
                    for k in 1..=4 {
                        let (sn, cs) = dmath::sin_cos(std::f64::consts::FRAC_PI_2 * k as f64);
                        let (x, y) = if k == 4 {
                            (p[0] + rh, p[1])
                        } else {
                            (p[0] + rh * cs, p[1] + rh * sn)
                        };
                        for q in 1..=4 {
                            let a0 = std::f64::consts::FRAC_PI_2 * (k - 1) as f64;
                            let (s1, c1) =
                                dmath::sin_cos(a0 + std::f64::consts::FRAC_PI_2 * (q - 1) as f64 / 4.0);
                            let (s2, c2) = dmath::sin_cos(a0 + std::f64::consts::FRAC_PI_2 * q as f64 / 4.0);
                            let a = [p[0] + rh * c1, p[1] + rh * s1, z];
                            let b = [p[0] + rh * c2, p[1] + rh * s2, z];
                            f.map.sweep(a, b, &f.prof.clone());
                            moves_at_level.push((a, b));
                        }
                        mv.push(PMove::Arc {
                            to: [x, y, z],
                            c: p,
                            cw: false,
                            f: feed.0,
                        });
                    }
                    cur = [p[0] + rh, p[1], z];
                } else {
                    let b = [cur[0], cur[1], z];
                    f.map.sweep(cur, b, &f.prof.clone());
                    mv.push(PMove::Line {
                        to: b,
                        f: feed.1,
                        phase: Phase::Plunge,
                    });
                    moves_at_level.push((b, b));
                    cur = b;
                }
                if start.is_none() {
                    start = Some([p[0] + rh, p[1], h.clear]);
                }
                at = Some(cur);
                entered = true;
                continue;
            }
            let Some((p, _)) = best else {
                if open_try && !entered {
                    // no open side here: a helix entry
                    open_try = false;
                    continue;
                }
                // no start next to material: a new entry if material is left (at most 20)
                entries += 1;
                if entries >= 20 {
                    break;
                }
                entered = false;
                small = 0;
                continue;
            };
            // the link, by the fixed rule
            entered = true;
            if start.is_none() {
                start = Some([p[0], p[1], h.clear]);
            }
            let cur = at.unwrap_or([p[0], p[1], h.clear]);
            let target = [p[0], p[1], z];
            if cur[2] == z && f.link_clear([cur[0], cur[1]], p, z) {
                mv.push(PMove::Line {
                    to: target,
                    f: feed.0,
                    phase: Phase::Link,
                });
            } else {
                mv.push(PMove::Line {
                    to: [cur[0], cur[1], h.retract],
                    f: feed.0,
                    phase: Phase::Link,
                });
                mv.push(PMove::Rapid {
                    to: [p[0], p[1], h.retract],
                });
                mv.push(PMove::Line {
                    to: target,
                    f: feed.1,
                    phase: Phase::Plunge,
                });
            }
            moves_at_level.push((target, target));
            // the heading: the sweep direction (every pass the same way)
            let dir = sweep_dir;
            let mut u = dir;
            let mut touched = false;
            let mut removed = 0.0;
            let mut pos = target;
            let mut air_run = 0usize;
            let mut pass: Vec<[f64; 3]> = Vec::new();
            for _ in 0..20_000 {
                // the largest turn to the right (into material) whose engagement is within the
                // cap: bisection on the turn δ ∈ [−90°, 90°], φ falling as δ rises; where no turn
                // keeps the cap at this step length, shorter steps (down to an eighth)
                let mut len = ds;
                let mut found = None;
                for _ in 0..4 {
                    let eval = |d: f64| {
                        let w = rot(u, d);
                        let q = [pos[0] + len * w[0], pos[1] + len * w[1], z];
                        // a heading past PEEL from the sweep direction is not taken
                        if w[0] * dir[0] + w[1] * dir[1] < peel_cos {
                            return (f64::INFINITY, w, q);
                        }
                        (f.phi(q, w), w, q)
                    };
                    // turns within ±90° of the heading and within PEEL of the sweep direction
                    let th_u = dmath::atan2(dir[0] * u[1] - dir[1] * u[0], dir[0] * u[0] + dir[1] * u[1]);
                    let peel = PEEL.to_radians() - 1e-9;
                    let lo_d = (-std::f64::consts::FRAC_PI_2).max(-peel - th_u);
                    let hi_d = std::f64::consts::FRAC_PI_2.min(peel - th_u);
                    let (p_hi, ..) = eval(hi_d);
                    let (p_lo, ..) = eval(lo_d);
                    // turn toward the side the material is on (the right, climb, once a pass has
                    // found its wall), as far as the cap allows: bisection between the full
                    // turn that way and the full turn the other way
                    let (toward, away) = if p_lo >= p_hi { (lo_d, hi_d) } else { (hi_d, lo_d) };
                    let p_away = if toward == lo_d { p_hi } else { p_lo };
                    if p_away > cap {
                        len *= 0.5;
                        continue;
                    }
                    let d = if p_hi <= 0.0 && p_lo <= 0.0 {
                        // air every way: turn toward the side material lies on (within the
                        // tool's reach and two steps), as far as the peel allows; none: straight
                        let reach = f.r + 2.0 * len + 2.0 * f.map.h;
                        let mut side = 0.0;
                        for k in 0..24 {
                            let dd = rot([1.0, 0.0], std::f64::consts::TAU * k as f64 / 24.0);
                            if f.material(pos[0] + reach * dd[0], pos[1] + reach * dd[1], z) {
                                side += u[0] * dd[1] - u[1] * dd[0];
                            }
                        }
                        // gently (10°): the strip a pass cuts follows the edge the last one
                        // left, so a steep approach would set every strip's slope
                        let gentle = 10f64.to_radians();
                        if side > 1e-9 {
                            gentle.clamp(lo_d, hi_d)
                        } else if side < -1e-9 {
                            (-gentle).clamp(lo_d, hi_d)
                        } else {
                            0.0f64.clamp(lo_d, hi_d)
                        }
                    } else if eval(toward).0 <= cap {
                        toward
                    } else {
                        let (mut a, mut b) = (toward, away);
                        for _ in 0..14 {
                            let mid = 0.5 * (a + b);
                            if eval(mid).0 <= cap {
                                b = mid;
                            } else {
                                a = mid;
                            }
                        }
                        b
                    };
                    // the chosen step itself must keep the cap (∞ off the allowed centres)
                    let e = eval(d);
                    if e.0 > cap {
                        len *= 0.5;
                        continue;
                    }
                    found = Some(e);
                    break;
                }
                let Some((phi, w, q)) = found else { break };
                let got = f.map.sweep(pos, q, &f.prof.clone());
                removed += got;
                // a step that removes less than a 0.01 mm strip counts as air: a pass grazing
                // slivers would otherwise orbit
                let step_len = ((q[0] - pos[0]).powi(2) + (q[1] - pos[1]).powi(2)).sqrt();
                if phi <= 0.0 || got < 0.01 * step_len * (above - z).max(f.map.h) {
                    air_run += 1;
                    // in air two radii after cutting, or four before meeting material: done
                    let limit = if touched { 8 } else { 16 };
                    if air_run > limit {
                        moves_at_level.push((pos, q));
                        pass.push(q);
                        break;
                    }
                } else {
                    air_run = 0;
                    touched = true;
                }
                moves_at_level.push((pos, q));
                pass.push(q);
                pos = q;
                u = w;
            }
            // drop the trailing air steps already swept (they cut nothing) from the path
            let keep = pass.len().saturating_sub(air_run);
            let mut pts = vec![target];
            pts.extend_from_slice(&pass[..keep]);
            fit_arcs(&pts, f.r, feed.0, &mut mv);
            at = Some(if keep > 0 { pass[keep - 1] } else { target });
            // each start is used once (crumbs the cap keeps a pass from reaching are left to
            // the wall pass, not retried)
            cands.retain(|c| *c != p);
            // three passes in a row that each removed next to nothing: the rest is crumbs the
            // cap keeps the passes from (corners), left to the rings below
            if removed < 0.25 * f.r * f.r * (above - z).max(f.map.h) {
                small += 1;
                if small >= 3 {
                    // the passes stall: a new entry where material is left (at most 20)
                    entries += 1;
                    if entries >= 20 {
                        break;
                    }
                    entered = false;
                    small = 0;
                }
            } else {
                small = 0;
            }
        }
        // a plain offset pass along the walls (rule 5), linked by the same rule
        // and plain offset rings at r + k·r inside it (inner first) where material is left
        // between the passes and the wall band: each ring cut only if its disk meets material,
        // entered at a vertex whose disk is clear (else left), the wall ring always, last
        if prm.wall_pass {
            let mut rings: Vec<(usize, Vec<Seg>)> = Vec::new();
            for k in 0..64 {
                let Ok((ring, _)) = offset(rg, r + k as f64 * f.r) else {
                    break;
                };
                if ring.is_empty() {
                    break;
                }
                for l in ring {
                    rings.push((k, l.segs));
                }
            }
            rings.sort_by_key(|x| std::cmp::Reverse(x.0));
            for (k, segs) in rings {
                if segs.is_empty() {
                    continue;
                }
                // points along it no further apart than a cell (arcs' chords and lines alike)
                let mut pts = polygon(&segs, 0.01);
                pts.push(pts[0]);
                let dense: Vec<(f64, f64)> = pts
                    .windows(2)
                    .flat_map(|w| {
                        let n = (((w[1].0 - w[0].0) * (w[1].0 - w[0].0)
                            + (w[1].1 - w[0].1) * (w[1].1 - w[0].1))
                            .sqrt()
                            / f.map.h)
                            .ceil()
                            .max(1.0) as usize;
                        (0..n).map(move |m| {
                            let t = m as f64 / n as f64;
                            (w[0].0 + t * (w[1].0 - w[0].0), w[0].1 + t * (w[1].1 - w[0].1))
                        })
                    })
                    .collect();
                if k > 0 && !dense.iter().any(|q| f.disk_has_material(q.0, q.1, z)) {
                    continue;
                }
                // a vertex to enter at whose disk is clear, the loop turned to start there
                // a point to enter at whose disk is clear: a vertex, else a point along a segment
                // (the segment split there), the loop turned to start there
                let mut entry: Option<(usize, Option<forge_cam_geom::region::P2>)> = (0..segs.len())
                    .find(|&i| {
                        let q = segs[i].start();
                        !f.disk_has_material(q.x, q.y, z)
                    })
                    .map(|i| (i, None));
                if entry.is_none() {
                    'seek: for (i, sg) in segs.iter().enumerate() {
                        for q in polygon(std::slice::from_ref(sg), 0.01).iter().skip(1) {
                            if !f.disk_has_material(q.0, q.1, z) {
                                entry = Some((i, Some(p2(q.0, q.1))));
                                break 'seek;
                            }
                        }
                        // lines give only their start: sample them too
                        if let Seg::Line { a, b } = *sg {
                            let n = (((b.x - a.x) * (b.x - a.x) + (b.y - a.y) * (b.y - a.y)).sqrt() / f.map.h)
                                .ceil() as usize;
                            for m in 1..n {
                                let t = m as f64 / n as f64;
                                let (x, y) = (a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
                                if !f.disk_has_material(x, y, z) {
                                    entry = Some((i, Some(p2(x, y))));
                                    break 'seek;
                                }
                            }
                        }
                    }
                }
                // an inner ring with no clear point is left; the wall ring is always cut
                let (i0, split) = match entry {
                    Some(e) => e,
                    None if k == 0 => (0, None),
                    None => continue,
                };
                let mut l: Vec<Seg> = Vec::with_capacity(segs.len() + 1);
                match split {
                    None => {
                        l.extend_from_slice(&segs[i0..]);
                        l.extend_from_slice(&segs[..i0]);
                    }
                    Some(q) => {
                        let (first, last) = match segs[i0] {
                            Seg::Line { a, b } => (Seg::Line { a: q, b }, Seg::Line { a, b: q }),
                            Seg::Arc { a, b, c, ccw } => {
                                (Seg::Arc { a: q, b, c, ccw }, Seg::Arc { a, b: q, c, ccw })
                            }
                        };
                        l.push(first);
                        l.extend_from_slice(&segs[i0 + 1..]);
                        l.extend_from_slice(&segs[..i0]);
                        l.push(last);
                    }
                }
                let l = forge_cam_geom::region::Loop { segs: l };
                {
                    let s0 = l.segs[0].start();
                    let target = [s0.x, s0.y, z];
                    let cur = at.unwrap_or(target);
                    if cur[2] == z && f.link_clear([cur[0], cur[1]], [s0.x, s0.y], z) {
                        mv.push(PMove::Line {
                            to: target,
                            f: feed.0,
                            phase: Phase::Link,
                        });
                    } else {
                        mv.push(PMove::Line {
                            to: [cur[0], cur[1], h.retract],
                            f: feed.0,
                            phase: Phase::Link,
                        });
                        mv.push(PMove::Rapid {
                            to: [s0.x, s0.y, h.retract],
                        });
                        mv.push(PMove::Line {
                            to: target,
                            f: feed.1,
                            phase: Phase::Plunge,
                        });
                    }
                    let mut a = target;
                    for sg in &l.segs {
                        let b = [sg.end().x, sg.end().y, z];
                        // the model sweeps the chord; the post takes the arc
                        let pts = polygon(std::slice::from_ref(sg), 0.01);
                        for p in pts.iter().skip(1).chain(std::iter::once(&(b[0], b[1]))) {
                            let q = [p.0, p.1, z];
                            f.map.sweep(a, q, &f.prof.clone());
                            moves_at_level.push((a, q));
                            a = q;
                        }
                        match *sg {
                            Seg::Line { .. } => mv.push(PMove::Line {
                                to: b,
                                f: feed.0,
                                phase: Phase::Cut,
                            }),
                            Seg::Arc { c, ccw, .. } => mv.push(PMove::Arc {
                                to: b,
                                c: [c.x, c.y],
                                cw: !ccw,
                                f: feed.0,
                            }),
                        }
                        a = b;
                    }
                    at = Some(target);
                }
            }
        }
        // what this level provably cleared
        if let Some(v) = claimed(&moves_at_level, z, r_flat, ip, f.r) {
            voids.push((v, z));
        }
        if let Some(p) = at {
            mv.push(PMove::Line {
                to: [p[0], p[1], h.retract],
                f: feed.0,
                phase: Phase::Link,
            });
            at = Some([p[0], p[1], h.retract]);
        }
        above = z;
    }
    let Some(start) = start else {
        return Err(OpError::Region("adaptive: nothing to cut".into()));
    };
    if let Some(p) = at {
        mv.push(PMove::Rapid {
            to: [p[0], p[1], h.clear],
        });
    }
    Ok(AdaptiveOut {
        path: OpPath {
            tool: tidx,
            start,
            moves: mv,
            rpm: 0.0,
            coolant: forge_post::Coolant::Flood,
        },
        voids,
    })
}

/// The pass through `pts` (one height) as moves: greedy arcs through runs of points, each
/// within `TOL_FIT` of every point it replaces, of radius at least r (so a step's dip across it
/// stays within the margin the centres keep) and at most 10 r (G5 bounds an arc by its circle's
/// box), under 170° and turning one way; else lines.
fn fit_arcs(pts: &[[f64; 3]], r: f64, f: f64, mv: &mut Vec<PMove>) {
    const TOL_FIT: f64 = 0.005;
    let circle = |a: [f64; 3], b: [f64; 3], c: [f64; 3]| -> Option<([f64; 2], f64)> {
        let (ax, ay, bx, by, cx, cy) = (a[0], a[1], b[0], b[1], c[0], c[1]);
        let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
        if d.abs() < 1e-12 {
            return None;
        }
        let a2 = ax * ax + ay * ay;
        let b2 = bx * bx + by * by;
        let c2 = cx * cx + cy * cy;
        let ux = (a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by)) / d;
        let uy = (a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax)) / d;
        let rr = ((ax - ux) * (ax - ux) + (ay - uy) * (ay - uy)).sqrt();
        Some(([ux, uy], rr))
    };
    // signed turn of the polyline a → b → c (+ counter-clockwise)
    let turn =
        |a: [f64; 3], b: [f64; 3], c: [f64; 3]| (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
    let ok = |i: usize, j: usize| -> Option<([f64; 2], bool)> {
        let m = (i + j) / 2;
        let (c, rr) = circle(pts[i], pts[m], pts[j])?;
        // at most 10 r: G5 bounds an arc by its whole circle's box against the travel
        if rr < r || rr > 10.0 * r {
            return None;
        }
        let ccw = turn(pts[i], pts[m], pts[j]) > 0.0;
        let mut total = 0.0;
        for k in i..j {
            let (a, b) = (pts[k], pts[k + 1]);
            let db = ((b[0] - c[0]).powi(2) + (b[1] - c[1]).powi(2)).sqrt();
            if (db - rr).abs() > TOL_FIT {
                return None;
            }
            let a0 = dmath::atan2(a[1] - c[1], a[0] - c[0]);
            let a1 = dmath::atan2(b[1] - c[1], b[0] - c[0]);
            let mut sw = a1 - a0;
            while sw > std::f64::consts::PI {
                sw -= std::f64::consts::TAU;
            }
            while sw < -std::f64::consts::PI {
                sw += std::f64::consts::TAU;
            }
            if (sw > 0.0) != ccw || sw == 0.0 {
                return None;
            }
            total += sw.abs();
        }
        if total > 170f64.to_radians() {
            return None;
        }
        Some((c, ccw))
    };
    let n = pts.len();
    let mut i = 0;
    while i + 1 < n {
        let mut best: Option<(usize, [f64; 2], bool)> = None;
        let mut j = i + 2;
        while j < n {
            match ok(i, j) {
                Some((c, ccw)) => best = Some((j, c, ccw)),
                None => break,
            }
            j += 1;
        }
        match best {
            Some((j, c, ccw)) => {
                mv.push(PMove::Arc {
                    to: pts[j],
                    c,
                    cw: !ccw,
                    f,
                });
                i = j;
            }
            None => {
                mv.push(PMove::Line {
                    to: pts[i + 1],
                    f,
                    phase: Phase::Cut,
                });
                i += 1;
            }
        }
    }
}

/// The cells of side H (r/2) whose every point lies within the tool's flat end (r_flat) of a
/// move at this level: centre within r_flat − H/√2 − 1e-6 of some move (f64 distances on
/// coordinates under 10⁴ mm err by under 1e-11 mm, far below the margin). As rectangles, each
/// a row's run of such cells; None if none.
fn claimed(moves: &[([f64; 3], [f64; 3])], z: f64, r_flat: f64, ip: &InProcess, r: f64) -> Option<Region> {
    let hh = 0.5 * r;
    let need = r_flat - hh * std::f64::consts::FRAC_1_SQRT_2 - 1e-6;
    if need <= 0.0 {
        return None;
    }
    let nx = (((ip.max[0] - ip.min[0]) / hh).ceil() as usize).max(1);
    let ny = (((ip.max[1] - ip.min[1]) / hh).ceil() as usize).max(1);
    let mut cov = vec![false; nx * ny];
    for &(a, b) in moves {
        if a[2] != z || b[2] != z {
            continue;
        }
        let (x0, x1) = (a[0].min(b[0]) - need, a[0].max(b[0]) + need);
        let (y0, y1) = (a[1].min(b[1]) - need, a[1].max(b[1]) + need);
        let i0 = (((x0 - ip.min[0]) / hh - 0.5).floor().max(0.0)) as usize;
        let j0 = (((y0 - ip.min[1]) / hh - 0.5).floor().max(0.0)) as usize;
        let i1 = ((((x1 - ip.min[0]) / hh - 0.5).ceil() + 1.0).max(0.0) as usize).min(nx);
        let j1 = ((((y1 - ip.min[1]) / hh - 0.5).ceil() + 1.0).max(0.0) as usize).min(ny);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let ll = dx * dx + dy * dy;
        for j in j0..j1 {
            for i in i0..i1 {
                let k = j * nx + i;
                if cov[k] {
                    continue;
                }
                let x = ip.min[0] + (i as f64 + 0.5) * hh;
                let y = ip.min[1] + (j as f64 + 0.5) * hh;
                let t = if ll > 0.0 {
                    (((x - a[0]) * dx + (y - a[1]) * dy) / ll).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (px, py) = (a[0] + t * dx, a[1] + t * dy);
                if ((x - px) * (x - px) + (y - py) * (y - py)).sqrt() <= need {
                    cov[k] = true;
                }
            }
        }
    }
    // rows' runs, merged with the run of the same span in the row below
    let mut rects: Vec<[usize; 4]> = Vec::new(); // i0, i1 (excl), j0, j1 (excl)
    let mut open: Vec<usize> = Vec::new();
    for j in 0..ny {
        let mut runs = Vec::new();
        let mut i = 0;
        while i < nx {
            if cov[j * nx + i] {
                let s = i;
                while i < nx && cov[j * nx + i] {
                    i += 1;
                }
                runs.push((s, i));
            } else {
                i += 1;
            }
        }
        let mut next_open = Vec::new();
        for (s, e) in runs {
            if let Some(&k) = open
                .iter()
                .find(|&&k| rects[k][0] == s && rects[k][1] == e && rects[k][3] == j)
            {
                rects[k][3] = j + 1;
                next_open.push(k);
            } else {
                rects.push([s, e, j, j + 1]);
                next_open.push(rects.len() - 1);
            }
        }
        open = next_open;
    }
    if rects.is_empty() {
        return None;
    }
    // disjoint rectangles shrunk by a hair so no two touch (a region's loops may not)
    let e = 1e-6;
    let loops = rects
        .iter()
        .map(|q| {
            let x0 = ip.min[0] + q[0] as f64 * hh + e;
            let x1 = ip.min[0] + q[1] as f64 * hh - e;
            let y0 = ip.min[1] + q[2] as f64 * hh + e;
            let y1 = ip.min[1] + q[3] as f64 * hh - e;
            let c = [p2(x0, y0), p2(x1, y0), p2(x1, y1), p2(x0, y1)];
            Loop {
                segs: (0..4)
                    .map(|i| Seg::Line {
                        a: c[i],
                        b: c[(i + 1) % 4],
                    })
                    .collect(),
            }
        })
        .collect();
    Some(Region { loops })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{Heights, ToolUse};

    fn tool(id: &str) -> ToolUse {
        let lib = forge_tooling::Library::builtin();
        let t = lib.tool(id).unwrap().clone();
        let shape = t.shape(30.0, None).unwrap();
        ToolUse {
            t: 1,
            tool: t,
            holder: None,
            stickout: 30.0,
            shape,
        }
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Loop {
        let c = [p2(x0, y0), p2(x1, y0), p2(x1, y1), p2(x0, y1)];
        Loop {
            segs: (0..4)
                .map(|i| Seg::Line {
                    a: c[i],
                    b: c[(i + 1) % 4],
                })
                .collect(),
        }
    }

    /// The winding number of a path's xy projection (every move, links and rapids too, closed
    /// by the chord from its end to its start) about q.
    fn winding(path: &OpPath, q: [f64; 2]) -> i64 {
        let mut pts: Vec<[f64; 2]> = vec![[path.start[0], path.start[1]]];
        let mut at = path.start;
        for mv in &path.moves {
            match *mv {
                PMove::Rapid { to } | PMove::Line { to, .. } => {
                    pts.push([to[0], to[1]]);
                    at = to;
                }
                PMove::Arc { to, c, cw, .. } => {
                    // the arc by points (its sweep under a half turn each way of the chord's side)
                    let r = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt();
                    let a0 = dmath::atan2(at[1] - c[1], at[0] - c[0]);
                    let a1 = dmath::atan2(to[1] - c[1], to[0] - c[0]);
                    let mut sw = a1 - a0;
                    let tau = std::f64::consts::TAU;
                    if cw {
                        while sw >= 0.0 {
                            sw -= tau;
                        }
                    } else {
                        while sw <= 0.0 {
                            sw += tau;
                        }
                    }
                    for k in 1..=16 {
                        let (sn, cs) = dmath::sin_cos(a0 + sw * k as f64 / 16.0);
                        pts.push([c[0] + r * cs, c[1] + r * sn]);
                    }
                    at = to;
                }
                PMove::Cycle { .. } => {}
            }
        }
        pts.push(pts[0]);
        let mut total = 0.0;
        for w in pts.windows(2) {
            let a = dmath::atan2(w[0][1] - q[1], w[0][0] - q[0]);
            let b = dmath::atan2(w[1][1] - q[1], w[1][0] - q[0]);
            let mut d = b - a;
            while d > std::f64::consts::PI {
                d -= std::f64::consts::TAU;
            }
            while d < -std::f64::consts::PI {
                d += std::f64::consts::TAU;
            }
            total += d;
        }
        (total / std::f64::consts::TAU).round() as i64
    }

    /// fto-adaptive.md §9 condition (2): the whole path, passes and links and rapids, closed by
    /// its end–start chord, winds 0 times round the island, and at most once round any point of
    /// a grid (a pass and its way back enclose a strip of cleared air once; a spiral winds n
    /// times round its core).
    #[test]
    fn path_winds_round_nothing() {
        let t = tool("flat-6");
        // the stock 40 x 30 with a 10 x 8 island (a boss) in the middle
        let rg = Region {
            loops: vec![
                rect(-7.0, -7.0, 47.0, 37.0),
                rect(15.0, 11.0, 25.0, 19.0).reversed(),
            ],
        };
        let ip = InProcess::new([0.0, 0.0, -5.0], [40.0, 30.0, 0.0]);
        for (sweep, wall) in [(0.0, false), (90.0, false), (0.0, true)] {
            let prm = Adaptive {
                phi_max: 60.0,
                step: 0.0,
                leave: 0.0,
                wall_pass: wall,
                sweep,
            };
            let out = adaptive(
                &rg,
                Cut {
                    tool: &t,
                    tidx: 0,
                    h: Heights {
                        clear: 20.0,
                        retract: 3.0,
                    },
                    feed: (1000.0, 300.0),
                },
                Span {
                    top: 0.0,
                    bottom: -3.0,
                    step: 3.0,
                },
                &ip,
                &prm,
            )
            .unwrap();
            assert_eq!(
                winding(&out.path, [20.0, 15.0]),
                0,
                "sweep {sweep}, wall {wall}: round the island"
            );
            let mut y = -6.0;
            while y <= 36.0 {
                let mut x = -6.0;
                while x <= 46.0 {
                    // a pass and the link or rapid back enclose the strip of cleared air between
                    // them once; a spiral of n loops winds n times round its core
                    let w = winding(&out.path, [x + 0.013, y + 0.017]);
                    assert!(
                        w.abs() <= 1,
                        "sweep {sweep}, wall {wall}: {w} times round ({x}, {y})"
                    );
                    x += 2.0;
                }
                y += 2.0;
            }
        }
    }

    /// The peel (fto-adaptive.md §8 (b)): every cutting step at depth, wall pass off, heads
    /// within PEEL of the sweep direction, so no pass winds round a centre.
    #[test]
    fn passes_peel_one_way() {
        let t = tool("flat-6");
        // an open region: the stock box grown by the tool's diameter and 1 mm (as forge-cam-lang
        // makes it: room for the tool beside the stock)
        let rg = Region {
            loops: vec![rect(-7.0, -7.0, 47.0, 37.0)],
        };
        let ip = InProcess::new([0.0, 0.0, -5.0], [40.0, 30.0, 0.0]);
        for sweep in [0.0, 90.0, 30.0] {
            let prm = Adaptive {
                phi_max: 60.0,
                step: 0.0,
                leave: 0.0,
                wall_pass: false,
                sweep,
            };
            let out = adaptive(
                &rg,
                Cut {
                    tool: &t,
                    tidx: 0,
                    h: Heights {
                        clear: 20.0,
                        retract: 3.0,
                    },
                    feed: (1000.0, 300.0),
                },
                Span {
                    top: 0.0,
                    bottom: -3.0,
                    step: 3.0,
                },
                &ip,
                &prm,
            )
            .unwrap();
            let (sn, cs) = dmath::sin_cos(sweep.to_radians());
            let lim = dmath::cos((PEEL + 1.0).to_radians());
            let mut at = out.path.start;
            let mut n = 0;
            for mv in &out.path.moves {
                let to = match *mv {
                    PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => to,
                    PMove::Cycle { .. } => continue,
                };
                let cutting = matches!(
                    mv,
                    PMove::Line {
                        phase: Phase::Cut,
                        ..
                    } | PMove::Arc { .. }
                );
                if cutting && to[2] == -3.0 && at[2] == -3.0 {
                    let (dx, dy) = (to[0] - at[0], to[1] - at[1]);
                    let l = (dx * dx + dy * dy).sqrt();
                    // lines and fitted arcs (by their chord) alike
                    if l > 1e-9 {
                        n += 1;
                        assert!(
                            (dx * cs + dy * sn) / l >= lim,
                            "sweep {sweep}: step {at:?} -> {to:?}"
                        );
                    }
                }
                at = to;
            }
            assert!(n > 20, "sweep {sweep}: {n} steps");
        }
    }

    /// Rule 1 of the design-around: one capped target. Every step of every pass keeps the
    /// engagement the simulation measured within the cap (and nothing in the API sets a minimum).
    #[test]
    fn single_capped_target() {
        let t = tool("flat-6");
        // an open region: the stock box grown by the tool's diameter and 1 mm (as forge-cam-lang
        // makes it: room for the tool beside the stock)
        let rg = Region {
            loops: vec![rect(-7.0, -7.0, 47.0, 37.0)],
        };
        let ip = InProcess::new([0.0, 0.0, -5.0], [40.0, 30.0, 0.0]);
        let prm = Adaptive {
            phi_max: 60.0,
            step: 0.0,
            leave: 0.0,
            wall_pass: false,
            sweep: 0.0,
        };
        let out = adaptive(
            &rg,
            Cut {
                tool: &t,
                tidx: 0,
                h: Heights {
                    clear: 20.0,
                    retract: 3.0,
                },
                feed: (1000.0, 300.0),
            },
            Span {
                top: 0.0,
                bottom: -3.0,
                step: 3.0,
            },
            &ip,
            &prm,
        )
        .unwrap();
        // replay the cuts on a fresh model: each step's engagement within the cap (+ a cell's
        // worth of slack for the model's resolution)
        let prof = Profile::of(&t.tool).unwrap();
        let mut m = HeightMap::of_stock(&ip, 0.15);
        let mut at = out.path.start;
        let mut steps = 0;
        let mut over = 0;
        for mv in &out.path.moves {
            match *mv {
                PMove::Line {
                    to,
                    phase: Phase::Cut,
                    ..
                } if to[2] == at[2] && to[2] < -1.0 => {
                    let d = ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2)).sqrt();
                    if d > 0.0 {
                        let u = [(to[0] - at[0]) / d, (to[1] - at[1]) / d];
                        if let Some(e) = m.engage(to, u, 3.0, 1e-3) {
                            steps += 1;
                            if e.phi.to_degrees() > 60.0 + 25.0 {
                                over += 1;
                            }
                        }
                    }
                    m.sweep(at, to, &prof);
                }
                PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => {
                    m.sweep(at, to, &prof);
                }
                PMove::Cycle { .. } => {}
            }
            at = match *mv {
                PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => to,
                PMove::Cycle { .. } => at,
            };
        }
        assert!(steps > 50, "{steps}");
        assert!(over * 50 <= steps, "{over} of {steps} steps over the cap");
        // and it cleared most of the pocket's floor area (claimed voids)
        assert!(!out.voids.is_empty());
    }
}

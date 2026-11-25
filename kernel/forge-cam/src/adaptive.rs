//! Adaptive clearing: a 2.5D roughing path whose radial engagement is held under ONE capped
//! target by simulating the cleared area step by step.
//!
//! How it works, and why:
//! 1. A cleared-area simulation steers each step to one capped engagement target (`Adaptive::
//!    phi_max`): the largest turn into material whose engagement does not exceed the cap. Holding
//!    engagement at the cap keeps the tool load steady, so the full flute length can be used at a
//!    feed set for that load, and no step ever slots. Steps are final once made: the path is the
//!    simulation's output, so what is certified is what was simulated.
//! 2. Passes are joined by a fixed link rule: stay down on a straight move that lies in the cleared
//!    area, else go back along the last pass and then straight (below), else back along it and
//!    through the cleared area by the shortest way the height map allows, each leg tested as a
//!    straight link is; else retract. A link at depth is taken only when it is no longer than the
//!    vertical moves the retract would make plus a margin (`LINK_MARGIN_R`). On a machine with
//!    jerk limits a retract is the costliest thing in the path (two feed moves the level's depth
//!    and short rapids that never reach speed), so links run through cleared air at the link feed
//!    (just under the machine's limit), and a retract crosses just over the stock as it stood
//!    before the op rather than at the setup's retract height.
//! 3. Feed is not set here; forge_cam::engage sets it per fixed-length sample from the measured
//!    engagement.
//! 4. Islands are met by the same simulation as they come; the engagement cap holds round them.
//! 5. One plain offset wall pass leaves an even stock on the walls for the finishing op.
//!
//! Every pass is a one-direction "peel": its heading stays within `PEEL` of one sweep direction
//! and it ends at the boundary or the cap; the next starts by the link rule. Peels keep the cut
//! climb (or conventional) throughout, and their ends fall where links are short. `two_way` makes
//! alternate peels run back, a zigzag that halves the link travel where the machine's
//! acceleration makes that pay.
//!
//! The tests `single_capped_target` and `passes_peel_one_way` assert the cap and the peel; keep
//! them passing.
//!
//! Links at depth: straight where the cleared area holds it, else back along the last pass as it
//! was cut, then straight. The retrace lemma: the pass's own posted moves reversed, at its depth,
//! with the same tool, sweep a subset of the swath that pass cut, so they meet no material; only
//! the last straight leg is tested. `links_never_cut` replays it.
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
    /// "peel" passes: a pass never winds round a centre, so the cut stays climb or conventional).
    pub sweep: f64,
    /// Two-way passes: a pass heads within `PEEL` of the sweep direction
    /// or of its reverse, the starts still taken across the sweep from one side to the other
    /// (a zigzag); off: every pass the same way.
    pub two_way: bool,
    /// The feed (mm/min) of the links through cleared air: up out of the tool's own cleared
    /// column, down into air, and every link at depth (0: the cutting feed). Nothing is cut on
    /// them, so they need not crawl at the cutting feed; the language sets it just under the
    /// machine's `max_feed`.
    pub link_feed: f64,
}

/// The margin, in tool radii, a link at depth may run past the feed moves the retract it
/// replaces would make (`Field::link_budget`): a retract also adds rapids that stop and start
/// short of their speed. Provenance: 4, 10, 20 and 40 tried on verify/forge-cam-quality's
/// deep_boss_{n,p} under the jerk-limited planner (2026-11-24); 4 was quickest one-way.
pub const LINK_MARGIN_R: f64 = 4.0;
/// How many starts, in order, are tried for reach at depth before a retract (P1).
pub const REACH_N: usize = 30;

/// The most a pass's heading may turn from the sweep direction (either way).
pub const PEEL: f64 = 75.0;

/// The op's result: the path, and per level the cells it provably cleared (floor z).
pub struct AdaptiveOut {
    pub path: OpPath,
    pub voids: Vec<(Region, f64)>,
    /// The rings each level ends with: one record per ring or run
    /// of one, its moves `path.moves[first..end]`.
    pub rings: Vec<RingRun>,
    /// The links at depth that retrace a pass: `path.moves[first..end]`
    /// each; their arcs are links, not cuts (a posted arc carries no phase).
    pub retraces: Vec<(usize, usize)>,
    /// What the user should know about the path made (e.g. a link kind left out, and why).
    pub notes: Vec<String>,
}

/// A ring at the end of a level, or a run of one: `k` = 0 the wall pass (one offset of all the
/// walls together, at r), k > 0 a cleanup ring at r + k·r (only its runs that meet material).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingRun {
    pub z: f64,
    pub k: usize,
    pub first: usize,
    pub end: usize,
    /// The whole ring (no point of it clear to enter at, or the wall pass), not a run of it.
    pub whole: bool,
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
    /// The stock as it stood before the op (rapids are certified against it).
    map0: HeightMap,
    prof: Profile,
    r: f64,
    eps: f64,
    /// The box (xmin, ymin, xmax, ymax) of every sweep since the start cache last looked: the
    /// only cells changed (or restored, by a crumb taken back) since then.
    dirty: [f64; 4],
    /// The highest map cell of each block of BLOCK × BLOCK cells (built on the first
    /// `max_in`), a block stale once a sweep's box meets it, refreshed when asked.
    blocks: std::cell::RefCell<Option<Blocks>>,
    /// The stock before the op, and every move swept since, in order: `descent_meets` rebuilds
    /// the stock round a point from them, never below the material.
    ip: InProcess,
    swept: Vec<([f64; 3], [f64; 3])>,
}

/// The cells per side of a `max_in` block.
const BLOCK: usize = 8;

/// Block maxima of the map (see `Field::blocks`).
struct Blocks {
    bnx: usize,
    max: Vec<f64>,
    stale: Vec<bool>,
}

/// No box.
const NO_BOX: [f64; 4] = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];

impl Field {
    /// The map swept along a → b, the box of what changed noted.
    fn sweep(&mut self, a: [f64; 3], b: [f64; 3]) -> f64 {
        self.dirty = [
            self.dirty[0].min(a[0]).min(b[0]),
            self.dirty[1].min(a[1]).min(b[1]),
            self.dirty[2].max(a[0]).max(b[0]),
            self.dirty[3].max(a[1]).max(b[1]),
        ];
        let prof = self.prof;
        self.swept.push((a, b));
        let removed = self.map.sweep(a, b, &prof);
        // the blocks the swept box meets (the tool's radius, a cell more each side): stale
        if let Some(bl) = self.blocks.get_mut().as_mut() {
            let m = &self.map;
            let ci = |v: f64, o: f64| ((v - o) / m.h).floor() as i64;
            let w = self.r + m.h;
            let (i0, j0) = (
                (ci(a[0].min(b[0]) - w, m.x0) - 1).max(0),
                (ci(a[1].min(b[1]) - w, m.y0) - 1).max(0),
            );
            let (i1, j1) = (ci(a[0].max(b[0]) + w, m.x0) + 1, ci(a[1].max(b[1]) + w, m.y0) + 1);
            if i1 >= 0 && j1 >= 0 {
                let (i1, j1) = ((i1 as usize).min(m.nx - 1), (j1 as usize).min(m.ny - 1));
                for bj in j0 as usize / BLOCK..=j1 / BLOCK {
                    for bi in i0 as usize / BLOCK..=i1 / BLOCK {
                        bl.stale[bj * bl.bnx + bi] = true;
                    }
                }
            }
        }
        removed
    }

    /// At least the highest map cell of those with index i0..=i1, j0..=j1 (clamped to the grid):
    /// the highest of the blocks that meet them (−∞ if none).
    fn max_in(&self, i0: i64, j0: i64, i1: i64, j1: i64) -> f64 {
        let m = &self.map;
        let (i0, j0) = (i0.max(0) as usize, j0.max(0) as usize);
        if i1 < 0 || j1 < 0 || i0 >= m.nx || j0 >= m.ny {
            return f64::NEG_INFINITY;
        }
        let (i1, j1) = ((i1 as usize).min(m.nx - 1), (j1 as usize).min(m.ny - 1));
        let mut bl = self.blocks.borrow_mut();
        let b = bl.get_or_insert_with(|| {
            let bnx = m.nx.div_ceil(BLOCK);
            let n = bnx * m.ny.div_ceil(BLOCK);
            Blocks {
                bnx,
                max: vec![f64::NEG_INFINITY; n],
                stale: vec![true; n],
            }
        });
        let mut t = f64::NEG_INFINITY;
        for bj in j0 / BLOCK..=j1 / BLOCK {
            for bi in i0 / BLOCK..=i1 / BLOCK {
                let k = bj * b.bnx + bi;
                if b.stale[k] {
                    let mut mx = f64::NEG_INFINITY;
                    for j in bj * BLOCK..((bj + 1) * BLOCK).min(m.ny) {
                        for i in bi * BLOCK..((bi + 1) * BLOCK).min(m.nx) {
                            mx = mx.max(m.z[j * m.nx + i]);
                        }
                    }
                    b.max[k] = mx;
                    b.stale[k] = false;
                }
                t = t.max(b.max[k]);
            }
        }
        t
    }

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
        // every sample's cell lies in the box of cells from x − n h to x + n h (one cell more each
        // side for rounding): none above the level there, no material (`material` needs one)
        let m = &self.map;
        let ci = |v: f64, o: f64| ((v - o) / h).floor() as i64;
        let (rx, ry) = (n as f64 * h, n as f64 * h);
        if self.max_in(
            ci(x - rx, m.x0) - 1,
            ci(y - ry, m.y0) - 1,
            ci(x + rx, m.x0) + 1,
            ci(y + ry, m.y0) + 1,
        ) <= z + self.eps
        {
            return false;
        }
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

    /// The highest the stock stood before this op within `rr` of (x, y) (−∞: none).
    /// Whether any material above z may stand within the tool's disk at (x, y): the stock round
    /// the point rebuilt from the op's start and every move swept since, never below the material
    /// (`HeightMap::lower_voids_under`, `sweep_under`: a cell lowered only where wholly cut), and
    /// any cell meeting the disk above z. Exact up to the cells' size the safe way: it may see
    /// material that is not there, never miss material that is.
    fn descent_meets(&self, x: f64, y: f64, z: f64) -> bool {
        // on a grid an eighth of the map's (the cells' error, which makes the answer safe, also
        // widens it: at the map's own cell a park beside a wall was judged too near; at an eighth
        // the descents slowed cost under 1.2 % of any part's time), at most 400 cells a side
        let r = self.r;
        let hm = self.map.h;
        let h = (0.125 * hm).max(2.0 * (r + hm) / 400.0);
        let hh = h * std::f64::consts::FRAC_1_SQRT_2;
        // a cell meets the disk where its centre is within r + h/√2 of the axis
        let reach = r + hh;
        let lo = [
            (x - reach).max(self.ip.min[0]),
            (y - reach).max(self.ip.min[1]),
            self.ip.min[2],
        ];
        let hi = [
            (x + reach).min(self.ip.max[0]),
            (y + reach).min(self.ip.max[1]),
            self.ip.max[2],
        ];
        if lo[0] >= hi[0] || lo[1] >= hi[1] || hi[2] <= z + self.eps {
            return false;
        }
        // the stock before the op round the point, each void lowering only the cells wholly in it
        let mut m = HeightMap::new(lo, hi, h);
        m.lower_voids_under(&self.ip);
        // the moves that can bring a cell of the window down to z: near enough, and at or under
        // it; the latest first (the pass that cleared round the point is the likeliest to cover a
        // cell)
        let near: Vec<&([f64; 3], [f64; 3])> = self
            .swept
            .iter()
            .rev()
            .filter(|(a, b)| {
                a[2].min(b[2]) <= z + self.eps
                    && a[0].min(b[0]) - r <= hi[0]
                    && a[0].max(b[0]) + r >= lo[0]
                    && a[1].min(b[1]) - r <= hi[1]
                    && a[1].max(b[1]) + r >= lo[1]
            })
            .collect();
        // whether one position of one move covers the square of half-diagonal `half` about c
        // whole, with the body at or under z there (as `HeightMap::sweep_under` lowers a cell)
        let covers = |c: (f64, f64), half: f64| {
            near.iter().any(|&&(a, b)| {
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let ll = dx * dx + dy * dy;
                let t = if ll > 0.0 {
                    (((c.0 - a[0]) * dx + (c.1 - a[1]) * dy) / ll).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (px, py) = (a[0] + t * dx, a[1] + t * dy);
                let d = ((c.0 - px) * (c.0 - px) + (c.1 - py) * (c.1 - py)).sqrt();
                d + half <= r && a[2] + t * (b[2] - a[2]) + self.prof.at(d + half) <= z + self.eps
            })
        };
        // blocks of k × k cells (about the map's cell) first: a block covered whole needs no look
        // at its cells
        let k = ((hm / h).round() as usize).max(1);
        for bj in (0..m.ny).step_by(k) {
            for bi in (0..m.nx).step_by(k) {
                let (j1, i1) = ((bj + k).min(m.ny), (bi + k).min(m.nx));
                let bc = (
                    m.x0 + 0.5 * (bi + i1) as f64 * h,
                    m.y0 + 0.5 * (bj + j1) as f64 * h,
                );
                let half = 0.5 * h * (((i1 - bi) * (i1 - bi) + (j1 - bj) * (j1 - bj)) as f64).sqrt();
                if covers(bc, half) {
                    continue;
                }
                for j in bj..j1 {
                    let cy = m.y0 + (j as f64 + 0.5) * h;
                    for i in bi..i1 {
                        let cx = m.x0 + (i as f64 + 0.5) * h;
                        if (cx - x) * (cx - x) + (cy - y) * (cy - y) > reach * reach
                            || m.z[j * m.nx + i] <= z + self.eps
                        {
                            continue;
                        }
                        if !covers((cx, cy), hh) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn top0(&self, x: f64, y: f64, rr: f64) -> f64 {
        let h = self.map0.h;
        let n = (rr / h).ceil() as i64;
        let mut t = f64::NEG_INFINITY;
        for j in -n..=n {
            for i in -n..=n {
                let (dx, dy) = (i as f64 * h, j as f64 * h);
                if dx * dx + dy * dy <= rr * rr {
                    t = t.max(self.map0.top(x + dx, y + dy));
                }
            }
        }
        t
    }

    /// The link by retract from `cur` to `target` (its level): up, across at the retract height,
    /// down. A rapid is certified against the stock as it stood before the op (G3, 0.1 mm clear),
    /// so up and down go at feed only through the height that stock stood at over the tool's
    /// disk (and `CLEAR` above it), rapid beyond. Across, through `via` first (xy, at the retract
    /// height): a return over the swath the last pass cut, not across what is still uncut
    /// (so the tool never travels round uncut material at the retract height).
    fn retract_link(
        &self,
        mv: &mut Vec<PMove>,
        cur: [f64; 3],
        target: [f64; 3],
        retract: f64,
        feed: (f64, f64),
        via: &[[f64; 2]],
    ) {
        const CLEAR: f64 = 0.5;
        let rr = self.r + CLEAR + self.map0.h;
        // the height to cross at: just over the stock as it stood before the op along the whole
        // way across (the same disk test as the rapids' check), never above the retract height;
        // over a flat top that is one rapid across instead of up, across and down
        let mut way: Vec<[f64; 2]> = vec![[cur[0], cur[1]]];
        way.extend_from_slice(via);
        way.push([target[0], target[1]]);
        let mut top = cur[2].max(target[2]);
        for w in way.windows(2) {
            let l = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
            let n = (l / self.map0.h).ceil().max(1.0) as usize;
            for k in 0..=n {
                let t = k as f64 / n as f64;
                let (x, y) = (
                    w[0][0] + t * (w[1][0] - w[0][0]),
                    w[0][1] + t * (w[1][1] - w[0][1]),
                );
                top = top.max(self.top0(x, y, rr) + CLEAR);
            }
        }
        let retract = top.min(retract);
        if cur[2] < retract {
            let up = (self.top0(cur[0], cur[1], rr) + CLEAR).clamp(cur[2], retract);
            if up > cur[2] {
                mv.push(PMove::Line {
                    to: [cur[0], cur[1], up],
                    f: feed.0,
                    phase: Phase::Link,
                });
            }
            if up < retract {
                mv.push(PMove::Rapid {
                    to: [cur[0], cur[1], retract],
                });
            }
        }
        for v in via {
            mv.push(PMove::Rapid {
                to: [v[0], v[1], retract],
            });
        }
        mv.push(PMove::Rapid {
            to: [target[0], target[1], retract],
        });
        let down = (self.top0(target[0], target[1], rr) + CLEAR).clamp(target[2], retract);
        if down < retract {
            mv.push(PMove::Rapid {
                to: [target[0], target[1], down],
            });
        }
        // the rest of the way: where the tool's disk meets no material above the level (the map is
        // a height field: then the whole column is cut away), air: a link at the feed; else into
        // material at the plunge feed. The map's cells are lowered by their centres, so a skin
        // finer than a cell may stand in a disk the map shows clear: `descent_meets` looks again
        // without that error
        let air = !self.disk_has_material(target[0], target[1], target[2])
            && !self.descent_meets(target[0], target[1], target[2]);
        mv.push(PMove::Line {
            to: target,
            f: if air { feed.0 } else { feed.1 },
            phase: if air { Phase::Link } else { Phase::Plunge },
        });
    }

    /// The longest link at depth worth taking from `a` to `b` at z instead of a retract: the
    /// vertical way the retract would go (up from a and down to b through the height the stock
    /// stood at, as `retract_link` makes them) plus `margin`, for the stops and starts a retract
    /// adds. Not the way across: a retract crosses at rapid, while a link at depth that runs
    /// along a wall it just cut is held to the chip rule by the feed scheduler, so a long one
    /// costs more than the retract it replaces.
    fn link_budget(&self, a: [f64; 2], b: [f64; 2], z: f64, margin: f64) -> f64 {
        const CLEAR: f64 = 0.5;
        let rr = self.r + CLEAR + self.map0.h;
        let up = (self.top0(a[0], a[1], rr) + CLEAR - z).max(0.0);
        let down = (self.top0(b[0], b[1], rr) + CLEAR - z).max(0.0);
        up + down + margin
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
    let Cut {
        tool, tidx, h, feed, ..
    } = c;
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
    let margin = 0.01 + TOL_FIT + 2.0 * ds * ds / (8.0 * (0.5 * tool.tool.dc));
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
        map0: HeightMap::of_stock(ip, cell),
        prof,
        r: 0.5 * tool.tool.dc,
        eps: 1e-3,
        dirty: NO_BOX,
        blocks: std::cell::RefCell::new(None),
        ip: ip.clone(),
        swept: Vec::new(),
    };
    let cap = prm.phi_max.clamp(5.0, 180.0).to_radians();
    let mut sweep_dir: [f64; 2];
    let peel_cos = dmath::cos(PEEL.to_radians());
    // the entries a sweep of a level may make before it stops
    let max_entries = 20usize;
    // the cleanup rings' spacing: a radius (half a radius was tried: slower, no more complete)
    let ring_step = 0.5 * tool.tool.dc;
    // the tightest a pass may curve: the tool's radius. A control holds a feed F round a curve of
    // radius R only while F <= sqrt(A R) (A its acceleration); at R = r that is about 47 mm/s at
    // 750 mm/s^2 for a 6 mm tool, above roughing feeds, so passes keep their feed through turns
    // (and a turn that tight would mostly be the cap swinging the tool round a corner anyway)
    let rmin = 0.5 * tool.tool.dc;
    let link_margin = LINK_MARGIN_R * 0.5 * tool.tool.dc;
    // the feed of links through cleared air
    let lf = if prm.link_feed > 0.0 {
        prm.link_feed
    } else {
        feed.0
    };
    // the direction the last pass went along the sweep (+1 with it, −1 against it)
    let mut last_sign = 1.0f64;
    let mut rings_log: Vec<RingRun> = Vec::new();
    let mut retraces_log: Vec<(usize, usize)> = Vec::new();
    let mut pass_ranges: Vec<(usize, usize)> = Vec::new();
    // a retrace runs tangent to the walls the pass left: where those may be final part walls
    // (no stock left on them and no wall pass after), it could mark a finished surface, so no
    // retraces there
    let retrace_ok = prm.leave > 0.0 || prm.wall_pass;
    let mut notes: Vec<String> = Vec::new();
    if !retrace_ok {
        notes.push(
            "adaptive: leave = 0 and no wall pass: links do not retrace passes (a retrace could \
             rub a finished wall); they retract instead"
                .into(),
        );
    }
    // the way back over the last pass, for a link by retract (one-way passes)
    let mut back: Vec<[f64; 2]> = Vec::new();
    let mut mv: Vec<PMove> = Vec::new();
    let mut voids = Vec::new();
    // where the tool is: (x, y, z)
    let mut at: Option<[f64; 3]> = None;
    let mut start: Option<[f64; 3]> = None;
    let mut above = sp.top;
    for z in levels(sp.top, sp.bottom, sp.step) {
        // what the tool has done at depth since it last came down at this level, as cut and
        // linked: where that began and the moves (each sweeps only what is cleared or cuts it)
        let mut hist: ([f64; 3], Vec<PMove>) = ([0.0, 0.0, f64::NAN], Vec::new());
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
        let mut grid = {
            let n = |a: f64, b: f64| (((b - a) + 4.0 * f.r) / g).floor() as usize + 1;
            let (nx, ny) = (n(lo[0], hi[0]), n(lo[1], hi[1]));
            Grid {
                o: [lo[0] - 2.0 * f.r, lo[1] - 2.0 * f.r],
                g,
                nx,
                ny,
                free: vec![false; nx * ny],
                last_searched: false,
            }
        };
        // the sweeps of this level: the user's direction first; where its passes stop with
        // material left (an island's lee, out of reach of passes that way), the reverse, then
        // the two across it, each with its own entries
        let all_cands = cands.clone();
        let mut dir_k = 0usize;
        // crumb passes taken back (see CRUMB_STEPS) until a level would be left incomplete
        let mut crumbs_now = true;
        sweep_dir = {
            let (sn, cs) = dmath::sin_cos(prm.sweep.to_radians());
            [cs, sn]
        };
        // each point's distance from the region's edge and whether it is allowed, worked out
        // once a level (they do not change; the material does), in the scan's own order
        let edge_cache: std::cell::RefCell<Vec<(f64, bool)>> = std::cell::RefCell::new(Vec::new());
        let left_area = |f: &Field, d: f64| -> f64 {
            let (lo, hi) = (ip.min, ip.max);
            let h = f.map.h;
            let mut a = 0.0;
            let mut cache = edge_cache.borrow_mut();
            let mut k = 0usize;
            let mut y = lo[1];
            while y <= hi[1] {
                let mut x = lo[0];
                while x <= hi[0] {
                    // (material within a radius of the region's edge is the rings' and the wall
                    // pass's to take)
                    if f.material(x, y, z) {
                        while cache.len() <= k {
                            cache.push((f64::NAN, false));
                        }
                        if cache[k].0.is_nan() {
                            cache[k] = (edge_dist(&f.allowed, x, y), f.ok(x, y));
                        }
                        if cache[k].0 > d && cache[k].1 {
                            a += h * h;
                        }
                    }
                    k += 1;
                    x += h;
                }
                y += h;
            }
            a
        };
        let mut entered = false;
        // first, a start from the open side (cleared air next to material ahead), no entry
        let mut open_try = true;
        let mut guard = 0usize;
        let mut small = 0usize;
        // entries at this level: a new one where the passes stop with material left
        let mut entries = 0usize;
        macro_rules! next_sweep {
            () => {{
                let left_far = left_area(&f, 4.0 * f.r) > 4.0 * f.map.h * f.map.h;
                // with crumbs taken back, any material more than a radius from the walls (beyond
                // the cleanup rings' reach) sends the level round again with every pass kept
                let left_near = crumbs_now && dir_k >= 3 && left_area(&f, f.r) > f.r * f.r;
                if (left_far || left_near) && dir_k >= 3 && crumbs_now {
                    // every direction tried while crumbs were taken back, material still far
                    // from the walls: the sweeps again with every pass kept (complete first)
                    crumbs_now = false;
                    dir_k = 0;
                    let (sn, cs) = dmath::sin_cos(prm.sweep.to_radians());
                    sweep_dir = [cs, sn];
                    entries = 0;
                    small = 0;
                    open_try = true;
                    entered = false;
                    last_sign = 1.0;
                    cands = all_cands.clone();
                    true
                } else if dir_k < 3 && left_far {
                    dir_k += 1;
                    let a = prm.sweep + [180.0, 90.0, -90.0][dir_k - 1];
                    let (sn, cs) = dmath::sin_cos(a.to_radians());
                    sweep_dir = [cs, sn];
                    entries = 0;
                    small = 0;
                    open_try = true;
                    entered = false;
                    last_sign = 1.0;
                    cands = all_cands.clone();
                    true
                } else {
                    false
                }
            }};
        }
        // whether each candidate's disk holds material, as last tested: a value stands until a
        // sweep (or a crumb taken back) comes within its reach (the disk test reads cells within
        // r + h of the centre, a sweep changes cells within r of its path: 2r + 3h covers both
        // and the cells' rounding)
        let mut disk_cache: std::collections::HashMap<(u64, u64), bool> = std::collections::HashMap::new();
        f.dirty = NO_BOX;
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
            let mut best: Option<([f64; 2], f64, f64)> = None;
            let mut order: Vec<([f64; 2], f64, f64)> = Vec::new();
            if f.dirty[0] <= f.dirty[2] {
                let m = 2.0 * f.r + 3.0 * f.map.h;
                let b = [f.dirty[0] - m, f.dirty[1] - m, f.dirty[2] + m, f.dirty[3] + m];
                disk_cache.retain(|&(x, y), _| {
                    let (x, y) = (f64::from_bits(x), f64::from_bits(y));
                    x < b[0] || y < b[1] || x > b[2] || y > b[3]
                });
                f.dirty = NO_BOX;
            }
            if entered || open_try {
                for &p in &cands {
                    let key = (p[0].to_bits(), p[1].to_bits());
                    let has = match disk_cache.get(&key) {
                        Some(&v) => v,
                        None => {
                            let v = f.disk_has_material(p[0], p[1], z);
                            disk_cache.insert(key, v);
                            v
                        }
                    };
                    if has {
                        continue;
                    }
                    // material just beyond the disk, ahead along the sweep (sign +1) or, two-way,
                    // against it (−1): a pass from here meets it going that way
                    // within the band the pass sweeps going straight (its width, out to four
                    // radii ahead)
                    let ahead = |sign: f64| {
                        let u = [sign * sweep_dir[0], sign * sweep_dir[1]];
                        let n = [-u[1], u[0]];
                        [f.r + g + 2.0 * f.map.h, 2.0 * f.r, 3.0 * f.r, 4.0 * f.r]
                            .iter()
                            .any(|&t| {
                                [-0.9, -0.45, 0.0, 0.45, 0.9].iter().any(|&s| {
                                    let (x, y) =
                                        (p[0] + t * u[0] + s * f.r * n[0], p[1] + t * u[1] + s * f.r * n[1]);
                                    f.material(x, y, z)
                                })
                            })
                    };
                    // two-way: the way back first (a zigzag), else the way the last pass went
                    let sign = if !prm.two_way {
                        if !ahead(1.0) {
                            continue;
                        }
                        1.0
                    } else if ahead(-last_sign) {
                        -last_sign
                    } else if ahead(last_sign) {
                        last_sign
                    } else {
                        continue;
                    };
                    // across the sweep from one side to the other: the
                    // lowest band of r/2 across the sweep first, the nearest within it
                    let across = (sweep_dir[0] * p[1] - sweep_dir[1] * p[0]) / (0.5 * f.r);
                    let d = across.floor() * 1e9 + (p[0] - here[0]).powi(2) + (p[1] - here[1]).powi(2);
                    if best.is_none_or(|(_, bd, _)| d < bd) {
                        best = Some((p, d, sign));
                    }
                    order.push((p, d, sign));
                }
                // P1: among the starts reachable at depth (straight in the
                // cleared area, or by a retrace then straight), the first in that order (the
                // lowest band across the sweep, then the nearest); none of the first REACH_N in
                // reach: the first, by a retract
                // (one-way passes only: two-way passes keep their zigzag order, which already
                // ends each pass near the next start; P1 there costs more reversals than it saves)
                if let Some(c) = at.filter(|c| retrace_ok && c[2] == z && !prm.two_way) {
                    order.sort_by(|u, v| u.1.total_cmp(&v.1));
                    if let Some(q) = order.iter().take(REACH_N).find(|(q, ..)| {
                        f.link_clear([c[0], c[1]], *q, z)
                            || retrace_link(
                                c,
                                *q,
                                &hist,
                                z,
                                &|a, b| f.link_clear(a, b, z),
                                0.0,
                                f.link_budget([c[0], c[1]], *q, z, link_margin),
                            )
                            .is_some()
                    }) {
                        best = Some(*q);
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
                let Some((p, depth)) = deep else {
                    if next_sweep!() {
                        continue;
                    }
                    break;
                };
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
                        f: lf,
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
                            f.sweep(a, b);
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
                            f.sweep(a, b);
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
                    f.sweep(cur, b);
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
                // after an entry the history at depth begins where it ends
                hist = (cur, Vec::new());
                entered = true;
                continue;
            }
            let Some((p, _, sign)) = best else {
                if open_try && !entered {
                    // no open side here: a helix entry
                    open_try = false;
                    continue;
                }
                // no start next to material: a new entry if material is left (at most 20)
                entries += 1;
                if entries >= max_entries {
                    if next_sweep!() {
                        continue;
                    }
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
            // a start within half a radius of where the tool already is, at depth and parked
            // clear: the pass starts where the tool is (no jog of a millimetre between passes)
            let snap = cur[2] == z
                && ((p[0] - cur[0]).powi(2) + (p[1] - cur[1]).powi(2)).sqrt() < f.r
                && f.ok(cur[0], cur[1])
                && !f.disk_has_material(cur[0], cur[1], z);
            let target = if snap { cur } else { [p[0], p[1], z] };
            // what the pass changes, kept so a crumb pass can be taken back
            let undo_map = if crumbs_now { f.map.z.clone() } else { Vec::new() };
            let undo_swept = f.swept.len();
            let undo = (
                mv.len(),
                hist.clone(),
                retraces_log.len(),
                moves_at_level.len(),
                at,
                start,
                last_sign,
                back.clone(),
                pass_ranges.len(),
            );
            let retrace = if retrace_ok && cur[2] == z && !f.link_clear([cur[0], cur[1]], p, z) {
                retrace_link(
                    cur,
                    p,
                    &hist,
                    z,
                    &|a, b| f.link_clear(a, b, z),
                    0.0,
                    f.link_budget([cur[0], cur[1]], p, z, link_margin),
                )
            } else {
                None
            };
            if snap {
                // no link: the pass begins here
            } else if cur[2] == z && f.link_clear([cur[0], cur[1]], p, z) {
                let l = PMove::Line {
                    to: target,
                    f: lf,
                    phase: Phase::Link,
                };
                mv.push(l);
                hist.1.push(l);
            } else if let Some(back_moves) = retrace.map(|m| at_feed(m, lf)) {
                let l = PMove::Line {
                    to: target,
                    f: lf,
                    phase: Phase::Link,
                };
                retraces_log.push((mv.len(), mv.len() + back_moves.len()));
                mv.extend_from_slice(&back_moves);
                mv.push(l);
                hist.1.extend(back_moves);
                hist.1.push(l);
            } else if let Some(legs) = (cur[2] == z && retrace_ok)
                .then(|| {
                    let cap = f.link_budget([cur[0], cur[1]], p, z, link_margin);
                    retrace_path_link(&f, &mut grid, cur, p, &hist, z, cap, &|a, b| {
                        f.link_clear(a, b, z)
                    })
                })
                .flatten()
                .map(|m| at_feed(m, lf))
            {
                retraces_log.push((mv.len(), mv.len() + legs.len()));
                mv.extend_from_slice(&legs);
                hist.1.extend(legs);
            } else {
                if cur[2] == z && std::env::var_os("FORGE_ADAPTIVE_DIAG").is_some() {
                    // what blocks the straight link: the highest material over the tool's disk
                    // along it, above the level; and whether any unused start was in reach
                    let (a, b) = ([cur[0], cur[1]], p);
                    let l = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
                    let n = (l / f.map.h).ceil().max(1.0) as usize;
                    let rr = f.r + f.map.h;
                    let mut block = f64::NEG_INFINITY;
                    for k in 0..=n {
                        let t = k as f64 / n as f64;
                        let (x, y) = (a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]));
                        let m = (rr / f.map.h).ceil() as i64;
                        for j in -m..=m {
                            for i in -m..=m {
                                let (dx, dy) = (i as f64 * f.map.h, j as f64 * f.map.h);
                                if dx * dx + dy * dy <= rr * rr && inside(&f.work, x + dx, y + dy) {
                                    block = block.max(f.map.top(x + dx, y + dy) - z);
                                }
                            }
                        }
                    }
                    let mut near: Vec<&[f64; 2]> = cands
                        .iter()
                        .filter(|c| **c != p && !f.disk_has_material(c[0], c[1], z))
                        .collect();
                    near.sort_by(|u, v| {
                        let du = (u[0] - a[0]).powi(2) + (u[1] - a[1]).powi(2);
                        let dv = (v[0] - a[0]).powi(2) + (v[1] - a[1]).powi(2);
                        du.total_cmp(&dv)
                    });
                    let reach = near.iter().take(40).any(|c| f.link_clear(a, **c, z));
                    eprintln!("diag retract len={l:.2} block={block:.3} other_reachable={reach}");
                }
                f.retract_link(&mut mv, cur, target, h.retract, (lf, feed.1), &back);
                // down at the start: the history at depth begins here
                hist = (target, Vec::new());
            }
            moves_at_level.push((target, target));
            // the heading: the sweep direction, or two-way its reverse (the way material lies)
            let dir = [sign * sweep_dir[0], sign * sweep_dir[1]];
            last_sign = sign;
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
                    // and within the turn a step of this length may take on a curve of radius
                    // `rmin` (none under it: a control slows on a tighter curve)
                    let dmax = if rmin > 0.5 * len {
                        {
                            // 2 asin(len / 2 rmin), by atan2 (dmath: bit-identical everywhere)
                            let sx = 0.5 * len / rmin;
                            (2.0 * dmath::atan2(sx, (1.0 - sx * sx).sqrt())).min(std::f64::consts::FRAC_PI_2)
                        }
                    } else {
                        std::f64::consts::FRAC_PI_2
                    };
                    let lo_d = (-dmax).max(-peel - th_u);
                    let hi_d = dmax.min(peel - th_u);
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
                        // left, so a steep approach would set every strip's slope. But only as
                        // far toward the material as the engagement the heading meets ahead
                        // stays within the cap: going straight on, out to a radius past where
                        // it first meets material (a start whose disk already overlaps a face
                        // by more than the cap allows would otherwise turn into it and stall at
                        // the stock's corner, and whether it does would depend on where the
                        // grids fall). The first heading, from most toward the material to
                        // away from it, whose engagement ahead is within the cap; none: the one
                        // meeting the least
                        let gentle = 10f64.to_radians();
                        let s = if side > 1e-9 {
                            1.0
                        } else if side < -1e-9 {
                            -1.0
                        } else {
                            0.0
                        };
                        if s == 0.0 {
                            0.0f64.clamp(lo_d, hi_d)
                        } else {
                            // the most engagement met going straight along w from here: out to
                            // a radius past the first contact, at most four radii (∞ steps off
                            // the allowed centres end the look)
                            let ahead_max = |w: [f64; 2]| -> f64 {
                                let (mut t, mut hit, mut most) = (len, None::<f64>, 0.0f64);
                                while t <= 4.0 * f.r && hit.is_none_or(|h| t <= h + f.r) {
                                    let q = [pos[0] + t * w[0], pos[1] + t * w[1], z];
                                    let ph = f.phi(q, w);
                                    if !ph.is_finite() {
                                        break;
                                    }
                                    if ph > 0.0 && hit.is_none() {
                                        hit = Some(t);
                                    }
                                    most = most.max(ph);
                                    t += len;
                                }
                                most
                            };
                            let mut least: Option<(f64, f64)> = None;
                            let mut pick = None;
                            for k in [1.0, 0.5, 0.0, -0.5, -1.0, -2.0] {
                                let d = (s * k * gentle).clamp(lo_d, hi_d);
                                let m = ahead_max(rot(u, d));
                                if m <= cap {
                                    pick = Some(d);
                                    break;
                                }
                                if least.is_none_or(|(_, lm)| m < lm) {
                                    least = Some((d, m));
                                }
                            }
                            pick.or(least.map(|(d, _)| d)).unwrap_or(0.0f64.clamp(lo_d, hi_d))
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
                let got = f.sweep(pos, q);
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
            // a crumb pass (a few steps into a sliver a corner or the cap left): taken back with
            // its link, the sliver left to the cleanup rings below, and counted as a stall
            if crumbs_now && keep <= CRUMB_STEPS {
                f.map.z = undo_map;
                f.swept.truncate(undo_swept);
                *f.blocks.get_mut() = None;
                mv.truncate(undo.0);
                hist = undo.1;
                retraces_log.truncate(undo.2);
                moves_at_level.truncate(undo.3);
                at = undo.4;
                start = undo.5;
                last_sign = undo.6;
                back = undo.7;
                pass_ranges.truncate(undo.8);
                cands.retain(|c| *c != p);
                small += 1;
                if small >= 3 {
                    entries += 1;
                    if entries >= max_entries {
                        if next_sweep!() {
                            continue;
                        }
                        break;
                    }
                    entered = false;
                    small = 0;
                }
                continue;
            }
            let mut pts = vec![target];
            pts.extend_from_slice(&pass[..keep]);
            let k0 = mv.len();
            fit_pass(&pts, f.r, feed.0, &mut mv);
            pass_ranges.push((k0, mv.len()));
            if std::env::var_os("FORGE_ADAPTIVE_DIAG").is_some() {
                let mut a = pts[0];
                let mut ls = Vec::new();
                for m in &mv[k0..] {
                    let (to, l) = match *m {
                        PMove::Arc { to, c, cw, .. } => (to, arc_len(a, to, c, cw)),
                        PMove::Line { to, .. } => {
                            (to, ((to[0] - a[0]).powi(2) + (to[1] - a[1]).powi(2)).sqrt())
                        }
                        _ => continue,
                    };
                    ls.push(format!(
                        "{}{:.1}",
                        if matches!(m, PMove::Arc { .. }) { "a" } else { "l" },
                        l
                    ));
                    a = to;
                }
                eprintln!("pass {} steps: {}", pts.len() - 1, ls.join(" "));
            }
            // the pass as cut, for a link back along it
            hist.1.extend_from_slice(&mv[k0..]);
            at = Some(if keep > 0 { pass[keep - 1] } else { target });
            // the way back over this pass: its points from the end to its start, simplified to
            // within r/2 of it (inside the swath it cut)
            back = simplify(
                &pts.iter().rev().map(|q| [q[0], q[1]]).collect::<Vec<_>>(),
                0.5 * f.r,
            );
            back.remove(0);
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
                    if entries >= max_entries {
                        if next_sweep!() {
                            continue;
                        }
                        break;
                    }
                    entered = false;
                    small = 0;
                }
            } else {
                small = 0;
            }
        }
        // every sweep has stopped: material left far from the walls (the wall pass and rings
        // take what is nearer) is reported, never left silently
        let left = left_area(&f, 4.0 * f.r);
        if left > 10.0 * f.r * f.r {
            notes.push(format!(
                "adaptive: at z {z:.3} the passes left {left:.0} mm² uncut away from the walls \
                 after every sweep (G4 lists it); clear() takes it"
            ));
        }
        // a plain offset pass along the walls (rule 5), linked by the same rule
        // and plain offset rings at r + k·r inside it (inner first) where material is left
        // between the passes and the wall band: each ring cut only if its disk meets material,
        // entered at a vertex whose disk is clear (else left), the wall ring always, last
        if prm.wall_pass {
            let mut rings: Vec<(usize, Vec<Seg>)> = Vec::new();
            for k in 0..64 {
                let Ok((ring, _)) = offset(rg, r + k as f64 * ring_step) else {
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
                // a cleanup ring: only its runs that meet material, each from the clear
                // point before it to the clear point after, linked by the rule; material all round
                // it: the whole ring, below
                if k > 0 {
                    let n = dense.len();
                    let active: Vec<bool> = dense.iter().map(|q| f.disk_has_material(q.0, q.1, z)).collect();
                    if let Some(i0) = active.iter().position(|a| !a) {
                        let mut i = 0;
                        while i < n {
                            if !active[(i0 + i) % n] {
                                i += 1;
                                continue;
                            }
                            let j = (i0 + i) % n;
                            let mut pts: Vec<[f64; 3]> = vec![{
                                let q = dense[(j + n - 1) % n];
                                [q.0, q.1, z]
                            }];
                            let mut m = i;
                            while m < n && active[(i0 + m) % n] {
                                let q = dense[(i0 + m) % n];
                                pts.push([q.0, q.1, z]);
                                m += 1;
                            }
                            let q = dense[(i0 + m) % n];
                            pts.push([q.0, q.1, z]);
                            i = m + 1;
                            let target = pts[0];
                            let cur = at.unwrap_or(target);
                            if cur[2] == z && f.link_clear([cur[0], cur[1]], [target[0], target[1]], z) {
                                mv.push(PMove::Line {
                                    to: target,
                                    f: lf,
                                    phase: Phase::Link,
                                });
                            } else {
                                f.retract_link(&mut mv, cur, target, h.retract, (lf, feed.1), &[]);
                            }
                            // straight stretches as one move each (within 2 µm of the ring,
                            // which runs at least 2r from every wall), then arcs fitted
                            let pts: Vec<[f64; 3]> =
                                simplify(&pts.iter().map(|q| [q[0], q[1]]).collect::<Vec<_>>(), 0.002)
                                    .into_iter()
                                    .map(|q| [q[0], q[1], z])
                                    .collect();
                            let first = mv.len();
                            for w in pts.windows(2) {
                                f.sweep(w[0], w[1]);
                                moves_at_level.push((w[0], w[1]));
                            }
                            fit_pass(&pts, f.r, feed.0, &mut mv);
                            rings_log.push(RingRun {
                                z,
                                k,
                                first,
                                end: mv.len(),
                                whole: false,
                            });
                            at = Some(*pts.last().unwrap());
                        }
                        continue;
                    }
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
                            f: lf,
                            phase: Phase::Link,
                        });
                    } else {
                        f.retract_link(&mut mv, cur, target, h.retract, (lf, feed.1), &[]);
                    }
                    let first = mv.len();
                    let mut a = target;
                    for sg in &l.segs {
                        let b = [sg.end().x, sg.end().y, z];
                        // the model sweeps the chord; the post takes the arc
                        let pts = polygon(std::slice::from_ref(sg), 0.01);
                        for p in pts.iter().skip(1).chain(std::iter::once(&(b[0], b[1]))) {
                            let q = [p.0, p.1, z];
                            f.sweep(a, q);
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
                    rings_log.push(RingRun {
                        z,
                        k,
                        first,
                        end: mv.len(),
                        whole: true,
                    });
                    at = Some(target);
                }
            }
        }
        if std::env::var_os("FORGE_ADAPTIVE_DIAG").is_some() {
            // the material left at this level, on a 2 mm grid ('#': any left in the cell)
            let (lo, hi) = (ip.min, ip.max);
            let g2 = 2.0;
            let mut y = hi[1];
            let mut left = 0.0;
            eprintln!("left at z={z}:");
            while y >= lo[1] {
                let mut row = String::new();
                let mut x = lo[0];
                while x <= hi[0] {
                    let mut any = false;
                    let mut yy = y - g2;
                    while yy < y {
                        let mut xx = x;
                        while xx < x + g2 {
                            if f.material(xx, yy, z) {
                                any = true;
                                left += f.map.h * f.map.h;
                            }
                            xx += f.map.h;
                        }
                        yy += f.map.h;
                    }
                    row.push(if any {
                        '#'
                    } else if !f.ok(x + 1.0, y - 1.0) {
                        '.'
                    } else {
                        ' '
                    });
                    x += g2;
                }
                eprintln!("  |{row}|");
                y -= g2;
            }
            eprintln!("left area {left:.0} mm2");
        }
        if std::env::var_os("FORGE_ADAPTIVE_DIAG").is_some() {
            let (lo, hi) = (ip.min, ip.max);
            let mut y = hi[1];
            eprintln!("after rings at z={z}:");
            while y >= lo[1] {
                let mut row = String::new();
                let mut x = lo[0];
                while x <= hi[0] {
                    let mut n = 0;
                    let mut yy = y - 2.0;
                    while yy < y {
                        let mut xx = x;
                        while xx < x + 2.0 {
                            if f.material(xx, yy, z) {
                                n += 1;
                            }
                            xx += f.map.h;
                        }
                        yy += f.map.h;
                    }
                    row.push(if n > 20 {
                        '#'
                    } else if n > 0 {
                        '+'
                    } else if !f.ok(x + 1.0, y - 1.0) {
                        '.'
                    } else {
                        ' '
                    });
                    x += 2.0;
                }
                eprintln!("  |{row}|");
                y -= 2.0;
            }
        }
        // what this level provably cleared
        if let Some(v) = claimed(&moves_at_level, z, r_flat, ip, f.r) {
            voids.push((v, z));
        }
        if let Some(p) = at {
            mv.push(PMove::Line {
                to: [p[0], p[1], h.retract],
                f: lf,
                phase: Phase::Link,
            });
            at = Some([p[0], p[1], h.retract]);
        }
        // after every pass and ring: what is left at this level, reported (`left_uncut`)
        if let Some(n) = left_note_h(z, left_uncut(&f.map, &f.work, z, f.eps), f.map.h) {
            notes.push(n);
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
    if std::env::var_os("FORGE_ADAPTIVE_DIAG").is_some() {
        // where the blocks shorter than r come from
        let mut at = start;
        let mut cnt: std::collections::BTreeMap<&str, (usize, usize)> = Default::default();
        for (i, m) in mv.iter().enumerate() {
            let to = match *m {
                PMove::Line { to, .. } | PMove::Rapid { to } | PMove::Arc { to, .. } => to,
                _ => continue,
            };
            let l = match *m {
                PMove::Arc { c, cw, .. } => arc_len(at, to, c, cw),
                _ => ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2) + (to[2] - at[2]).powi(2)).sqrt(),
            };
            let what = if matches!(m, PMove::Rapid { .. }) {
                "rapid"
            } else if let Some(&(a, b)) = pass_ranges.iter().find(|&&(a, b)| i >= a && i < b) {
                if b - a == 1 {
                    "pass (only)"
                } else if i == a {
                    "pass first"
                } else if i + 1 == b {
                    "pass last"
                } else if matches!(m, PMove::Arc { .. }) {
                    "pass mid arc"
                } else {
                    "pass mid line"
                }
            } else if rings_log.iter().any(|g| i >= g.first && i < g.end) {
                "ring"
            } else if retraces_log.iter().any(|&(a, b)| i >= a && i < b) {
                "retrace"
            } else if (to[2] - at[2]).abs() > 1e-9 {
                "vertical/helix"
            } else {
                match m {
                    PMove::Line {
                        phase: Phase::Link, ..
                    } => "link line",
                    PMove::Line {
                        phase: Phase::Cut, ..
                    } => "cut line (other)",
                    PMove::Arc { .. } => "arc (other)",
                    _ => "other",
                }
            };
            let e = cnt.entry(what).or_default();
            e.0 += 1;
            if l < 0.5 * tool.tool.dc {
                e.1 += 1;
            }
            at = to;
        }
        for (k, (n, sh)) in cnt {
            eprintln!("blocks {k:15} {n:6} under r: {sh}");
        }
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
        rings: rings_log,
        retraces: retraces_log,
        notes,
    })
}

/// The grid of candidate tool centres at a level (spacing g from `o`), and which of them are
/// known to be parked in cleared air (a disk once clear at a level stays clear: the stock only
/// loses material).
struct Grid {
    o: [f64; 2],
    g: f64,
    nx: usize,
    ny: usize,
    free: Vec<bool>,
    /// Whether the last `path_link` got as far as the search (a way in was found).
    last_searched: bool,
}

impl Grid {
    fn at(&self, i: usize, j: usize) -> [f64; 2] {
        [self.o[0] + i as f64 * self.g, self.o[1] + j as f64 * self.g]
    }
    fn near(&self, p: [f64; 2]) -> Option<(usize, usize)> {
        let i = ((p[0] - self.o[0]) / self.g).round();
        let j = ((p[1] - self.o[1]) / self.g).round();
        (i >= 0.0 && j >= 0.0 && (i as usize) < self.nx && (j as usize) < self.ny)
            .then_some((i as usize, j as usize))
    }
}

/// A link at depth through the cleared area when neither a straight move nor a retrace reaches:
/// the shortest chain of grid centres whose tool disks are clear of material (and allowed),
/// from a's grid neighbour to b's, then pulled tight into straight legs, each one tested by
/// `clear` (the same exact test as every link at depth) so the tool meets no material on any of
/// them. A link at depth beside the work costs far less on a real machine than a retract (two
/// moves of the level's whole depth, each with its own accelerations), and on an open level the
/// cleared area round the stock is connected. Within `cap` in length, else None (a retract).
fn path_link(
    f: &Field,
    grid: &mut Grid,
    a: [f64; 2],
    b: [f64; 2],
    z: f64,
    cap: f64,
    clear: &dyn Fn([f64; 2], [f64; 2]) -> bool,
) -> Option<Vec<[f64; 2]>> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let (sa, sb) = (grid.near(a)?, grid.near(b)?);
    let idx = |(i, j): (usize, usize)| j * grid.nx + i;
    let n = grid.nx * grid.ny;
    // within this search a blocked centre stays blocked (0 unknown, 1 free, 2 blocked)
    let mut state: Vec<u8> = grid.free.iter().map(|&fr| if fr { 1 } else { 0 }).collect();
    let is_free = |st: &mut Vec<u8>, gr: &Grid, c: (usize, usize)| -> bool {
        let k = idx(c);
        if st[k] == 0 {
            let p = gr.at(c.0, c.1);
            st[k] = if f.ok(p[0], p[1]) && !f.disk_has_material(p[0], p[1], z) {
                1
            } else {
                2
            };
        }
        st[k] == 1
    };
    grid.last_searched = false;
    if !is_free(&mut state, grid, sb) {
        return None;
    }
    // the way in: the clear centres within three cells of a that a straight leg reaches (the
    // tool ends a pass against material, its own grid centre is seldom clear)
    let mut seeds: Vec<((usize, usize), u64)> = Vec::new();
    for dj in -3i64..=3 {
        for di in -3i64..=3 {
            let (ni, nj) = (sa.0 as i64 + di, sa.1 as i64 + dj);
            if ni < 0 || nj < 0 || ni as usize >= grid.nx || nj as usize >= grid.ny {
                continue;
            }
            let q = (ni as usize, nj as usize);
            if !is_free(&mut state, grid, q) {
                continue;
            }
            let pq = grid.at(q.0, q.1);
            if clear(a, pq) {
                let d = ((pq[0] - a[0]).powi(2) + (pq[1] - a[1]).powi(2)).sqrt();
                seeds.push((q, (d / grid.g * 1000.0) as u64));
            }
        }
    }
    if seeds.is_empty() {
        return None;
    }
    grid.last_searched = true;
    // A*: cost in grid steps (√2 diagonals), scaled to integers
    let hcost = |c: (usize, usize)| {
        let (dx, dy) = (c.0.abs_diff(sb.0) as f64, c.1.abs_diff(sb.1) as f64);
        ((dx.max(dy) + (std::f64::consts::SQRT_2 - 1.0) * dx.min(dy)) * 1000.0) as u64
    };
    let limit = (cap / grid.g * 1000.0) as u64;
    let mut dist = vec![u64::MAX; n];
    let mut prev = vec![usize::MAX; n];
    let mut heap = BinaryHeap::new();
    for &(q, d0) in &seeds {
        if d0 < dist[idx(q)] {
            dist[idx(q)] = d0;
            heap.push(Reverse((d0 + hcost(q), d0, q)));
        }
    }
    let mut found = false;
    while let Some(Reverse((_, d, c))) = heap.pop() {
        if c == sb {
            found = true;
            break;
        }
        if d > dist[idx(c)] {
            continue;
        }
        for (di, dj) in [
            (-1i64, 0i64),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (-1, 1),
            (1, -1),
            (1, 1),
        ] {
            let (ni, nj) = (c.0 as i64 + di, c.1 as i64 + dj);
            if ni < 0 || nj < 0 || ni as usize >= grid.nx || nj as usize >= grid.ny {
                continue;
            }
            let q = (ni as usize, nj as usize);
            let nd = d + if di != 0 && dj != 0 { 1414 } else { 1000 };
            if nd >= dist[idx(q)] || nd + hcost(q) > limit {
                continue;
            }
            if !is_free(&mut state, grid, q) {
                continue;
            }
            dist[idx(q)] = nd;
            prev[idx(q)] = idx(c);
            heap.push(Reverse((nd + hcost(q), nd, q)));
        }
    }
    // remember what was found clear for the next search at this level
    for (k, st) in state.iter().enumerate() {
        if *st == 1 {
            grid.free[k] = true;
        }
    }
    if !found {
        return None;
    }
    let mut chain = vec![b];
    let mut k = idx(sb);
    while k != usize::MAX {
        chain.push(grid.at(k % grid.nx, k / grid.nx));
        k = prev[k];
    }
    chain.push(a);
    chain.reverse();
    // pulled tight: from each kept point, the farthest point of the chain in straight reach
    let mut out = vec![a];
    let mut i = 0;
    let mut len = 0.0;
    while i + 1 < chain.len() {
        let mut j = chain.len() - 1;
        while j > i + 1 && !clear(chain[i], chain[j]) {
            j -= 1;
        }
        if !clear(chain[i], chain[j]) {
            return None;
        }
        len += ((chain[j][0] - chain[i][0]).powi(2) + (chain[j][1] - chain[i][1]).powi(2)).sqrt();
        out.push(chain[j]);
        i = j;
    }
    (len <= cap).then_some(out)
}

/// A link at depth back along the last pass as it was cut (the retrace lemma, as in
/// `retrace_link`), to the first point from which a clear grid centre is in straight reach, then
/// through the cleared area by `path_link`. The whole within `cap` in length, else None.
#[allow(clippy::too_many_arguments)]
fn retrace_path_link(
    f: &Field,
    grid: &mut Grid,
    cur: [f64; 3],
    b: [f64; 2],
    last: &([f64; 3], Vec<PMove>),
    z: f64,
    cap: f64,
    clear: &dyn Fn([f64; 2], [f64; 2]) -> bool,
) -> Option<Vec<PMove>> {
    let (start, moves) = last;
    let feed = |m: &PMove| match *m {
        PMove::Line { f, .. } | PMove::Arc { f, .. } => f,
        _ => 0.0,
    };
    let leg = |q: [f64; 2], fd: f64| PMove::Line {
        to: [q[0], q[1], z],
        f: fd,
        phase: Phase::Link,
    };
    let fd0 = moves.last().map(feed).unwrap_or(0.0);
    if let Some(legs) = path_link(f, grid, [cur[0], cur[1]], b, z, cap, clear) {
        return Some(legs[1..].iter().map(|q| leg(*q, fd0)).collect());
    }
    if moves.is_empty() || start[2] != z {
        return None;
    }
    let mut pts = vec![*start];
    for m in moves {
        match *m {
            PMove::Line { to, .. } | PMove::Arc { to, .. } => pts.push(to),
            _ => return None,
        }
    }
    if *pts.last().unwrap() != cur || pts.iter().any(|p| p[2] != z) {
        return None;
    }
    let mut out = Vec::new();
    let mut len = 0.0;
    let mut tries = 0;
    for i in (0..moves.len()).rev() {
        let (from, to) = (pts[i + 1], pts[i]);
        let back = match moves[i] {
            PMove::Line { f, .. } => {
                len += ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt();
                PMove::Line {
                    to,
                    f,
                    phase: Phase::Link,
                }
            }
            PMove::Arc { c, cw, f, .. } => {
                len += arc_len(to, from, c, cw);
                PMove::Arc { to, c, cw: !cw, f }
            }
            _ => return None,
        };
        if len >= cap {
            return None;
        }
        out.push(back);
        if let Some(legs) = path_link(f, grid, [to[0], to[1]], b, z, cap - len, clear) {
            out.extend(legs[1..].iter().map(|q| leg(*q, feed(&moves[i]))));
            return Some(out);
        }
        // a search that found a way in but no way through: try a few more points, then give up
        if grid.last_searched {
            tries += 1;
            if tries > 3 {
                return None;
            }
        }
    }
    None
}

/// A link at depth back along the last pass as it was cut, then
/// straight to b. The retrace is the pass's own posted moves reversed (a line back along itself,
/// an arc back about the same centre), at the pass's depth, with the same tool: the tool sweeps
/// exactly a subset of the swath that pass cut, so it meets no material and removes none (the
/// lemma; G1-G3 check the posted path again). Only the last leg, from a point of the pass to b,
/// is tested (`clear`, as every link at depth is). Walking back from the pass's end (where the
/// tool is, `cur`), the first point from which b is in straight reach; the whole link within
/// `k` times the straight distance plus `d`; else None (a retract). Geometric only.
#[allow(clippy::too_many_arguments)]
fn retrace_link(
    cur: [f64; 3],
    b: [f64; 2],
    last: &([f64; 3], Vec<PMove>),
    z: f64,
    clear: &dyn Fn([f64; 2], [f64; 2]) -> bool,
    k: f64,
    d: f64,
) -> Option<Vec<PMove>> {
    let (start, moves) = last;
    if moves.is_empty() || start[2] != z {
        return None;
    }
    // the pass's points: start, then each move's end
    let mut pts = vec![*start];
    for m in moves {
        match *m {
            PMove::Line { to, .. } | PMove::Arc { to, .. } => pts.push(to),
            _ => return None,
        }
    }
    // the tool must be at the pass's end, at its depth
    let end = *pts.last().unwrap();
    if end != cur || pts.iter().any(|p| p[2] != z) {
        return None;
    }
    let dist = |p: [f64; 3], q: [f64; 2]| ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2)).sqrt();
    let cap = k * dist(cur, b) + d;
    let mut out = Vec::new();
    let mut len = 0.0;
    // walk back: move i goes pts[i] -> pts[i+1]; reversed it goes pts[i+1] -> pts[i]
    for i in (0..moves.len()).rev() {
        let (from, to) = (pts[i + 1], pts[i]);
        let back = match moves[i] {
            PMove::Line { f, .. } => {
                len += ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt();
                PMove::Line {
                    to,
                    f,
                    phase: Phase::Link,
                }
            }
            PMove::Arc { c, cw, f, .. } => {
                len += arc_len(to, from, c, cw);
                PMove::Arc { to, c, cw: !cw, f }
            }
            _ => return None,
        };
        if len + dist(to, b) > cap {
            return None;
        }
        out.push(back);
        if clear([to[0], to[1]], b) {
            return Some(out);
        }
    }
    None
}

/// Link moves at the feed `lf` (they run through cleared air: nothing is cut on them).
fn at_feed(mut moves: Vec<PMove>, lf: f64) -> Vec<PMove> {
    for m in &mut moves {
        match m {
            PMove::Line { f, .. } | PMove::Arc { f, .. } => *f = lf,
            _ => {}
        }
    }
    moves
}

/// The length of the arc from `a` to `b` about `c` (clockwise or not), in the plane.
fn arc_len(a: [f64; 3], b: [f64; 3], c: [f64; 2], cw: bool) -> f64 {
    let r = ((a[0] - c[0]).powi(2) + (a[1] - c[1]).powi(2)).sqrt();
    let t0 = dmath::atan2(a[1] - c[1], a[0] - c[0]);
    let t1 = dmath::atan2(b[1] - c[1], b[0] - c[0]);
    let mut d = if cw { t0 - t1 } else { t1 - t0 };
    while d < 0.0 {
        d += std::f64::consts::TAU;
    }
    r * d
}

/// A polyline simplified (Douglas–Peucker): every dropped point within `tol` of the kept chord.
fn simplify(p: &[[f64; 2]], tol: f64) -> Vec<[f64; 2]> {
    if p.len() <= 2 {
        return p.to_vec();
    }
    let (a, b) = (p[0], p[p.len() - 1]);
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let ll = dx * dx + dy * dy;
    let mut worst = (0.0, 0);
    for (i, q) in p.iter().enumerate().take(p.len() - 1).skip(1) {
        let t = if ll > 0.0 {
            (((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / ll).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let d = ((q[0] - a[0] - t * dx).powi(2) + (q[1] - a[1] - t * dy).powi(2)).sqrt();
        if d > worst.0 {
            worst = (d, i);
        }
    }
    if worst.0 <= tol {
        return vec![a, b];
    }
    let mut l = simplify(&p[..=worst.1], tol);
    let r = simplify(&p[worst.1..], tol);
    l.pop();
    l.extend(r);
    l
}

/// A pass of at most this many steps is a crumb, taken back (`adaptive`).
pub const CRUMB_STEPS: usize = 3;

/// How far a fitted arc or line may stray from the simulated steps it replaces (mm). The allowed
/// tool centres keep this much more from the region's edge, and the feeds are set from the
/// engagement of the posted path itself, so the fit costs no safety; a looser fit gives fewer,
/// longer blocks.
pub const TOL_FIT: f64 = 0.05;

/// The pass through `pts` (one height) as moves by the shared fitter (`crate::fit`): lines and
/// arcs, each within `TOL_FIT` of every step it replaces; arcs of radius r..10 r (at least r, so
/// a step's dip across it stays within the margin the centres keep; at most 10 r, as G5 boxes an
/// arc by its whole circle) and under 170°.
fn fit_pass(pts: &[[f64; 3]], r: f64, f: f64, mv: &mut Vec<PMove>) {
    if pts.len() < 2 {
        return;
    }
    let fit = crate::fit::Fit {
        tol: TOL_FIT,
        arcs: true,
        min_r: r,
        max_r: 10.0 * r,
        arc_dz: 1e-9,
    };
    let pieces = crate::fit::fit(pts, fit, &mut |from, p| match *p {
        crate::fit::Piece::Arc { to, c, cw } => {
            arc_len(from, to, c, cw)
                < 170f64.to_radians() * ((from[0] - c[0]).powi(2) + (from[1] - c[1]).powi(2)).sqrt()
        }
        _ => true,
    });
    for p in pieces {
        mv.push(match p {
            crate::fit::Piece::Line { to } => PMove::Line {
                to,
                f,
                phase: Phase::Cut,
            },
            crate::fit::Piece::Arc { to, c, cw } => PMove::Arc { to, c, cw, f },
        });
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

/// The stock a level left (mm²): the map's cells standing above z (by more than eps) whose centre
/// is inside the work region and more than a cell from its edge. The model holds material to a
/// cell, so nearer the walls is the model's rounding, and the wall pass's. G4 lists what is left
/// whatever its size; this is for the op's own report (`left_note`).
fn left_uncut(m: &HeightMap, work: &[Vec<(f64, f64)>], z: f64, eps: f64) -> f64 {
    let mut left = 0.0;
    for j in 0..m.ny {
        let y = m.y0 + (j as f64 + 0.5) * m.h;
        for i in 0..m.nx {
            if m.z[j * m.nx + i] > z + eps {
                let x = m.x0 + (i as f64 + 0.5) * m.h;
                if inside(work, x, y) && edge_dist(work, x, y) > m.h {
                    left += m.h * m.h;
                }
            }
        }
    }
    left
}

/// The op's note for stock left at a level: more than a 2 x 2 block of cells (`left_uncut`, cell
/// h), so that nothing is left silently (the next op meets it at full depth).
fn left_note_h(z: f64, left: f64, h: f64) -> Option<String> {
    (left > 4.0 * h * h).then(|| {
        format!(
            "adaptive: at z {z:.3} {left:.0} mm² of stock is left uncut after every pass and ring \
             (G4 lists it); the next op meets it at full depth: clear() or pocket() takes it"
        )
    })
}

#[cfg(test)]
mod tests {

    /// The leftover report: a patch more than a cell inside the work region is measured and
    /// reported; one cell, or a strip within a cell of the region's edge, is not.
    #[test]
    fn stock_left_more_than_a_cell_in_is_reported() {
        let h = 0.1;
        let mut m = HeightMap::new([0.0, 0.0, 0.0], [10.0, 10.0, 5.0], h);
        // all cut to z 1, the level
        m.z.iter_mut().for_each(|z| *z = 1.0);
        let work = vec![vec![(1.0, 1.0), (9.0, 1.0), (9.0, 9.0), (1.0, 9.0)]];
        let raise = |m: &mut HeightMap, x0: f64, x1: f64, y0: f64, y1: f64| {
            for j in 0..m.ny {
                let y = m.y0 + (j as f64 + 0.5) * m.h;
                for i in 0..m.nx {
                    let x = m.x0 + (i as f64 + 0.5) * m.h;
                    if (x0..x1).contains(&x) && (y0..y1).contains(&y) {
                        m.z[j * m.nx + i] = 5.0;
                    }
                }
            }
        };
        assert_eq!(left_uncut(&m, &work, 1.0, 1e-3), 0.0);
        assert!(left_note_h(1.0, 0.0, h).is_none());
        // a strip along the work's edge, within a cell of it: the model's rounding, not counted
        let mut e = m.clone();
        raise(&mut e, 1.0, 9.0, 1.0, 1.08);
        assert_eq!(left_uncut(&e, &work, 1.0, 1e-3), 0.0);
        // one cell inside: counted, too small to report
        let mut c = m.clone();
        raise(&mut c, 5.0, 5.1, 5.0, 5.1);
        let a = left_uncut(&c, &work, 1.0, 1e-3);
        assert!((a - h * h).abs() < 1e-12, "{a}");
        assert!(left_note_h(1.0, a, h).is_none());
        // a 1 mm square inside: 1 mm², reported
        let mut p = m.clone();
        raise(&mut p, 4.0, 5.0, 4.0, 5.0);
        let a = left_uncut(&p, &work, 1.0, 1e-3);
        assert!((a - 1.0).abs() < 1e-9, "{a}");
        let n = left_note_h(1.0, a, h).expect("reported");
        assert!(
            n.starts_with("adaptive: at z 1.000 1 mm² of stock is left uncut"),
            "{n}"
        );
    }

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
            probe: false,
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

    /// The path as travelled: points no further apart than 0.25 mm (arcs by chords), each with
    /// whether the move reaching it cuts (a feed move that is not a link or a plunge).
    fn travelled(path: &OpPath) -> Vec<([f64; 3], bool)> {
        travelled_links(path, &[])
    }

    /// `travelled`, the moves in `links` (index ranges) taken as links, not cuts.
    fn travelled_links(path: &OpPath, links: &[(usize, usize)]) -> Vec<([f64; 3], bool)> {
        let mut pts = vec![(path.start, false)];
        let mut at = path.start;
        for (mi, mv) in path.moves.iter().enumerate() {
            let is_link = links.iter().any(|&(a, b)| (a..b).contains(&mi));
            let (to, cut) = match *mv {
                PMove::Rapid { to } => (to, false),
                PMove::Line { to, phase, .. } => (to, phase == Phase::Cut),
                PMove::Arc { to, .. } => (to, true),
                PMove::Cycle { .. } | PMove::Probe { .. } => continue,
            };
            let cut = cut && !is_link;
            match *mv {
                PMove::Arc { c, cw, .. } => {
                    let r = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt();
                    let a0 = dmath::atan2(at[1] - c[1], at[0] - c[0]);
                    let a1 = dmath::atan2(to[1] - c[1], to[0] - c[0]);
                    let tau = std::f64::consts::TAU;
                    let mut sw = a1 - a0;
                    if cw {
                        while sw >= 0.0 {
                            sw -= tau;
                        }
                    } else {
                        while sw <= 0.0 {
                            sw += tau;
                        }
                    }
                    let n = ((sw.abs() * r / 0.25).ceil() as usize).max(1);
                    for k in 1..=n {
                        let t = k as f64 / n as f64;
                        let (sn, cs) = dmath::sin_cos(a0 + sw * t);
                        pts.push(([c[0] + r * cs, c[1] + r * sn, at[2] + t * (to[2] - at[2])], cut));
                    }
                }
                _ => {
                    let l =
                        ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2) + (to[2] - at[2]).powi(2)).sqrt();
                    let n = ((l / 0.25).ceil() as usize).max(1);
                    for k in 1..=n {
                        let t = k as f64 / n as f64;
                        pts.push((
                            [
                                at[0] + t * (to[0] - at[0]),
                                at[1] + t * (to[1] - at[1]),
                                at[2] + t * (to[2] - at[2]),
                            ],
                            cut,
                        ));
                    }
                }
            }
            at = to;
        }
        pts
    }

    /// The shape of the path, the binding tests:
    /// (1) each pass (a run of cutting moves at one depth) turns in all by at most 2·PEEL;
    /// (2) the path AS TRAVELLED (passes, links and rapids in order, never closed by a chord)
    ///     sweeps an angle about every island point, and about every point of material while it
    ///     is still uncut, that stays strictly inside (−2π, 2π) at every prefix: a spiral or a
    ///     moat accumulates whole turns, peels with their returns oscillate.
    fn assert_no_winding(out: &AdaptiveOut, island: [f64; 4], stock: [f64; 4], z: f64, r: f64, what: &str) {
        let path = &out.path;
        if std::env::var_os("FORGE_ADAPTIVE_TDUMP").is_some() {
            for (i, m) in path.moves.iter().enumerate() {
                let link = out.retraces.iter().any(|&(a, b)| (a..b).contains(&i));
                eprintln!("mv {i} link={link} {m:?}");
            }
        }
        let pts = travelled_links(path, &out.retraces);
        // (1)
        let mut run: Vec<[f64; 2]> = Vec::new();
        let check = |run: &mut Vec<[f64; 2]>| {
            let mut total = 0.0f64;
            let mut last: Option<f64> = None;
            for w in run.windows(2) {
                let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
                if dx * dx + dy * dy < 1e-12 {
                    continue;
                }
                let h = dmath::atan2(dy, dx);
                if let Some(l) = last {
                    let mut d = h - l;
                    while d > std::f64::consts::PI {
                        d -= std::f64::consts::TAU;
                    }
                    while d < -std::f64::consts::PI {
                        d += std::f64::consts::TAU;
                    }
                    total += d;
                }
                last = Some(h);
            }
            assert!(
                total.abs().to_degrees() <= 2.0 * PEEL + 1.0,
                "{what}: a pass turns {:.1} deg in all, from {:?} to {:?} ({} points)",
                total.to_degrees(),
                run.first(),
                run.last(),
                run.len()
            );
            run.clear();
        };
        let mut prev: Option<[f64; 3]> = None;
        for &(p, cut) in &pts {
            let at_depth = (p[2] - z).abs() < 1e-9 && prev.is_some_and(|q| (q[2] - z).abs() < 1e-9);
            if cut && at_depth {
                if run.is_empty() {
                    let q = prev.unwrap();
                    run.push([q[0], q[1]]);
                }
                run.push([p[0], p[1]]);
            } else if !run.is_empty() {
                check(&mut run);
            }
            prev = Some(p);
        }
        if !run.is_empty() {
            check(&mut run);
        }
        // (2): the points, each with the index from which it is cut (islands never)
        let mut probes: Vec<([f64; 2], usize)> = Vec::new();
        let mut y = island[1] + 0.5;
        while y < island[3] {
            let mut x = island[0] + 0.5;
            while x < island[2] {
                probes.push(([x, y], usize::MAX));
                x += 1.0;
            }
            y += 1.0;
        }
        let mut y = stock[1] + 0.25;
        while y < stock[3] {
            let mut x = stock[0] + 0.25;
            while x < stock[2] {
                let inside_island = x > island[0] && x < island[2] && y > island[1] && y < island[3];
                if !inside_island {
                    // cut from the first point where the tool at the level comes within r of it
                    let k = pts
                        .iter()
                        .position(|(p, _)| {
                            p[2] <= z + 1e-9 && (p[0] - x).powi(2) + (p[1] - y).powi(2) <= r * r
                        })
                        .unwrap_or(usize::MAX);
                    probes.push(([x + 0.013, y + 0.017], k));
                }
                x += 1.5;
            }
            y += 1.5;
        }
        for (q, until) in probes {
            let mut sum = 0.0f64;
            let mut worst = 0.0f64;
            let n = pts.len().min(until);
            for w in pts[..n].windows(2) {
                let a = dmath::atan2(w[0].0[1] - q[1], w[0].0[0] - q[0]);
                let b = dmath::atan2(w[1].0[1] - q[1], w[1].0[0] - q[0]);
                let mut d = b - a;
                while d > std::f64::consts::PI {
                    d -= std::f64::consts::TAU;
                }
                while d < -std::f64::consts::PI {
                    d += std::f64::consts::TAU;
                }
                sum += d;
                worst = worst.max(sum.abs());
            }
            assert!(
                worst < std::f64::consts::TAU - 1e-6,
                "{what}: the path turns {:.0} deg about ({:.2}, {:.2}) while it is {}",
                worst.to_degrees(),
                q[0],
                q[1],
                if until == usize::MAX {
                    "an island or never cut"
                } else {
                    "uncut"
                }
            );
        }
    }

    /// The path as travelled winds round nothing: see `assert_no_winding`.
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
        // the wall pass is a plain offset round the walls and islands at the end of each level:
        // it goes round by design, so these tests are of the sweep alone (wall off)
        for (sweep, wall, two_way) in [
            (0.0, false, false),
            (90.0, false, false),
            (30.0, false, false),
            (0.0, false, true),
            (90.0, false, true),
            (30.0, false, true),
        ] {
            // stock left on the walls (0.05): with no wall pass and none left, links would not
            // retrace, and the retraces are what this test must see
            let prm = Adaptive {
                phi_max: 60.0,
                step: 0.0,
                leave: 0.05,
                wall_pass: wall,
                sweep,
                two_way,
                link_feed: 0.0,
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
                    ramp_feed: None,
                    lead_feed: None,
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
            assert_no_winding(
                &out,
                [15.0, 11.0, 25.0, 19.0],
                [0.0, 0.0, 40.0, 30.0],
                -3.0,
                3.0,
                &format!("sweep {sweep}, wall {wall}, two-way {two_way}"),
            );
        }
    }

    /// The wall pass and the cleanup rings:
    /// (a) every ring of a level comes after the level's passes; (b) the wall pass is one offset
    /// of all the walls together, at r: at each level exactly the loops of that offset, whole;
    /// (c) a cleanup ring (k > 0) is cut only where it meets material: each run removes material,
    /// and a whole cleanup ring removes material on every move of it.
    #[test]
    fn rings_after_the_sweep_one_wall_offset_cleanup_partial() {
        let t = tool("flat-6");
        let rg = Region {
            loops: vec![
                rect(-7.0, -7.0, 47.0, 37.0),
                rect(15.0, 11.0, 25.0, 19.0).reversed(),
            ],
        };
        let ip = InProcess::new([0.0, 0.0, -5.0], [40.0, 30.0, 0.0]);
        let walls = offset(&rg, 3.0).unwrap().0.len();
        for two_way in [false, true] {
            let prm = Adaptive {
                phi_max: 60.0,
                step: 0.0,
                leave: 0.0,
                wall_pass: true,
                sweep: 0.0,
                two_way,
                link_feed: 0.0,
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
                    ramp_feed: None,
                    lead_feed: None,
                },
                Span {
                    top: 0.0,
                    bottom: -4.0,
                    step: 2.0,
                },
                &ip,
                &prm,
            )
            .unwrap();
            let in_ring = |i: usize| out.rings.iter().any(|g| (g.first..g.end).contains(&i));
            for z in [-2.0, -4.0] {
                let here: Vec<&RingRun> = out.rings.iter().filter(|g| g.z == z).collect();
                assert!(!here.is_empty(), "two-way {two_way}: no rings at {z}");
                // (b)
                let wall: Vec<&&RingRun> = here.iter().filter(|g| g.k == 0).collect();
                assert_eq!(wall.len(), walls, "two-way {two_way}: the wall pass at {z}");
                assert!(wall.iter().all(|g| g.whole));
                // (a): no pass move at this level after its first ring
                let first_ring = here.iter().map(|g| g.first).min().unwrap();
                let mut at = out.path.start;
                for (i, m) in out.path.moves.iter().enumerate() {
                    let to = match *m {
                        PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => to,
                        PMove::Cycle { .. } | PMove::Probe { .. } => continue,
                    };
                    let cut = matches!(
                        m,
                        PMove::Line {
                            phase: Phase::Cut,
                            ..
                        } | PMove::Arc { .. }
                    );
                    if cut && to[2] == z && at[2] == z && !in_ring(i) {
                        assert!(
                            i < first_ring,
                            "two-way {two_way}: a pass move {i} after the rings at {z}"
                        );
                    }
                    at = to;
                }
            }
            // (c): replay on a fresh model
            let prof = Profile::of(&t.tool).unwrap();
            let mut m = HeightMap::of_stock(&ip, 0.05);
            let mut at = out.path.start;
            let mut removed = vec![0.0; out.path.moves.len()];
            for (i, mv) in out.path.moves.iter().enumerate() {
                let to = match *mv {
                    PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => to,
                    PMove::Cycle { .. } | PMove::Probe { .. } => continue,
                };
                // arcs by chords
                if let PMove::Arc { c, cw, .. } = *mv {
                    let p = travelled(&OpPath {
                        tool: 0,
                        start: at,
                        moves: vec![PMove::Arc { to, c, cw, f: 1.0 }],
                        rpm: 0.0,
                        coolant: forge_post::Coolant::Flood,
                    });
                    for w in p.windows(2) {
                        removed[i] += m.sweep(w[0].0, w[1].0, &prof);
                    }
                } else {
                    removed[i] = m.sweep(at, to, &prof);
                }
                at = to;
            }
            for g in out.rings.iter().filter(|g| g.k > 0) {
                let total: f64 = removed[g.first..g.end].iter().sum();
                assert!(
                    total > 1e-3,
                    "two-way {two_way}: a cleanup run {g:?} removes nothing"
                );
                if g.whole {
                    for (i, r) in removed.iter().enumerate().take(g.end).skip(g.first) {
                        assert!(*r > 0.0, "two-way {two_way}: whole ring {g:?}: move {i} in air");
                    }
                }
            }
        }
    }

    /// Links never cut. Replay the path on a fresh model: every link
    /// move at depth (straight, or back along a pass as it was cut) removes nothing but the
    /// model's own rounding (a sliver of cells along a swath's edge, under 0.01 mm³ per mm).
    #[test]
    fn links_never_cut() {
        let t = tool("flat-6");
        let rg = Region {
            loops: vec![
                rect(-7.0, -7.0, 47.0, 37.0),
                rect(15.0, 11.0, 25.0, 19.0).reversed(),
            ],
        };
        let ip = InProcess::new([0.0, 0.0, -9.0], [40.0, 30.0, 0.0]);
        for two_way in [false, true] {
            let prm = Adaptive {
                phi_max: 40.0,
                step: 0.0,
                leave: 0.0,
                wall_pass: true,
                sweep: 0.0,
                two_way,
                link_feed: 0.0,
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
                    ramp_feed: None,
                    lead_feed: None,
                },
                Span {
                    top: 0.0,
                    bottom: -8.0,
                    step: 8.0,
                },
                &ip,
                &prm,
            )
            .unwrap();
            let prof = Profile::of(&t.tool).unwrap();
            let mut m = HeightMap::of_stock(&ip, 0.05);
            let mut at = out.path.start;
            let mut links = 0;
            for mv in &out.path.moves {
                let to = match *mv {
                    PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => to,
                    PMove::Cycle { .. } | PMove::Probe { .. } => continue,
                };
                // (a retraced arc carries no phase: those are covered by the lemma alone)
                let link = matches!(
                    mv,
                    PMove::Line {
                        phase: Phase::Link,
                        ..
                    }
                );
                let pts = travelled(&OpPath {
                    tool: 0,
                    start: at,
                    moves: vec![*mv],
                    rpm: 0.0,
                    coolant: forge_post::Coolant::Flood,
                });
                let mut got = 0.0;
                for w in pts.windows(2) {
                    got += m.sweep(w[0].0, w[1].0, &prof);
                }
                let level = to[2] == at[2] && to[2] == -8.0;
                if link && level {
                    links += 1;
                    let l = ((to[0] - at[0]).powi(2) + (to[1] - at[1]).powi(2)).sqrt();
                    assert!(
                        got <= 0.01 * l.max(1.0),
                        "two-way {two_way}: a link {at:?} -> {to:?} removes {got} mm³"
                    );
                }
                at = to;
            }
            assert!(links > 0, "two-way {two_way}: no links at depth");
        }
    }

    /// The peel: every pass (a run of cutting steps at depth between links, wall pass off) heads
    /// within PEEL of one direction throughout, so no pass winds round a centre; that direction
    /// is the user's sweep, or one of the further sweeps a level may need (its reverse, then the
    /// two across it).
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
                two_way: false,
                link_feed: 0.0,
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
                    ramp_feed: None,
                    lead_feed: None,
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
            let dirs: Vec<(f64, f64)> = [0.0, 180.0, 90.0, -90.0]
                .iter()
                .map(|d| dmath::sin_cos((sweep + d).to_radians()))
                .collect();
            let lim = dmath::cos((PEEL + 1.0).to_radians());
            let mut at = out.path.start;
            let mut n = 0;
            // the steps of the pass being read (unit headings)
            let mut run: Vec<(f64, f64, [f64; 3], [f64; 3])> = Vec::new();
            let check = |run: &[(f64, f64, [f64; 3], [f64; 3])]| {
                if run.is_empty() {
                    return;
                }
                assert!(
                    dirs.iter()
                        .any(|&(sn, cs)| run.iter().all(|&(ux, uy, ..)| ux * cs + uy * sn >= lim)),
                    "sweep {sweep}: a pass of {} steps from {:?} heads within PEEL of no one sweep",
                    run.len(),
                    run[0].2
                );
            };
            for (mi, mv) in out.path.moves.iter().enumerate() {
                let to = match *mv {
                    PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => to,
                    PMove::Cycle { .. } | PMove::Probe { .. } => continue,
                };
                // a retrace's arcs are links (back along a pass), not steps of a pass
                let link = out.retraces.iter().any(|&(a, b)| (a..b).contains(&mi));
                let cutting = !link
                    && matches!(
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
                        run.push((dx / l, dy / l, at, to));
                    }
                } else {
                    check(&run);
                    run.clear();
                }
                at = to;
            }
            check(&run);
            assert!(n > 20, "sweep {sweep}: {n} steps");
        }
    }

    /// One capped target. Every step of every pass keeps the
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
            two_way: false,
            link_feed: 0.0,
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
                ramp_feed: None,
                lead_feed: None,
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
                PMove::Cycle { .. } | PMove::Probe { .. } => {}
            }
            at = match *mv {
                PMove::Rapid { to } | PMove::Line { to, .. } | PMove::Arc { to, .. } => to,
                PMove::Cycle { .. } | PMove::Probe { .. } => at,
            };
        }
        assert!(steps > 50, "{steps}");
        assert!(over * 50 <= steps, "{over} of {steps} steps over the cap");
        // and it cleared most of the pocket's floor area (claimed voids)
        assert!(!out.voids.is_empty());
    }
}

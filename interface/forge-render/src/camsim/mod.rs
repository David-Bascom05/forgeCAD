//! The toolpath and stock simulation's state, free of the GPU and of egui (so it is tested
//! headless and is the same on both targets): the run's timeline, its tools, each setup's part
//! and stock, the display stock played to any time (checkpoints make scrubbing back cheap), and
//! the certified checks as markers on the timeline.
//!
//! The rule it keeps: what it draws is a preview; what it says about the program (pass, fail,
//! what is left) comes from the run's certified checks and its exact G4 leftover, never from the
//! dexels.

use forge_math::ipow::Powi as _;
pub mod draw;
pub mod machine;
pub mod timeline;
pub mod tools;
pub mod zmap;

use forge_cam_lang::Cam;
use forge_cam_lang::sim::SimSetup;
use timeline::Timeline;
use zmap::{Cutter, PartMap, ZMap};

/// A certified check that did not pass, or a leftover region, placed on the timeline.
#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub op: Option<usize>,
    pub setup: usize,
    /// The check's rule ("gouge", "holder_collision", .., or "leftover").
    pub rule: String,
    pub verdict: String,
    /// Where (setup frame).
    pub at: [f64; 3],
    /// When on the timeline.
    pub t: f64,
    pub text: String,
    /// Its row under its kind's heading (UX: the heading says what, the row when, where and how
    /// much): "move 0 · 0.35 mm into the stock".
    pub row: String,
    /// The part of the tool the check names ("cutter", "shank/holder"); empty where none is
    /// said (leftover, a refused op, the machine's contacts).
    pub part: String,
    /// The tool whose move it is, in words, in an op of several tools (else empty): said once
    /// in its kind's heading when the kind's places share it, else on each row.
    pub tool: String,
    /// A leftover region's box (xmin ymin xmax ymax, z0 z1).
    pub region: Option<([f64; 4], f64, f64)>,
    /// Material left by the plan, not missed: a leftover region wholly under the part's bottom
    /// in its setup (the stock left below the part for a later setup or a part-off). Shown
    /// neutral, never as an issue.
    pub planned: bool,
}

impl Marker {
    /// Collision-like (red) or leftover-like (amber).
    pub fn severe(&self) -> bool {
        self.rule != "leftover"
    }
}

/// The detail window's stock (`Sim::detail_at`).
struct Detail {
    setup: usize,
    /// The op it was started for (from the whole stock as that op began).
    op: u32,
    centre: [f64; 2],
    half: f64,
    /// The whole grid's columns it stands for: [i0, j0, i1, j1) (the drawing leaves them out).
    cols: [usize; 4],
    /// After `done` pieces.
    z: ZMap,
    done: usize,
    shown: Option<ZMap>,
}

/// The display stock played along the timeline.
struct Play {
    cells: usize,
    /// (pieces done, the stock then): every so often, and at each setup's start.
    checkpoints: Vec<(usize, ZMap)>,
    /// The stock after `done` pieces.
    cur: ZMap,
    done: usize,
    /// Each setup's stock once its last piece is done (for a setup turned over from it).
    finals: Vec<Option<ZMap>>,
    every: usize,
}

/// The simulation of one machining document.
pub struct Sim {
    pub cam: Cam,
    pub tl: Timeline,
    pub setups: Vec<SimSetup>,
    /// Per tool (`Cam::sim_tools` index): its cutting envelope, if its profile has one.
    pub cutters: Vec<Option<Cutter>>,
    /// Each tool as drawn (fluted cutter, neck, shank, holder), meshed once: display only.
    pub looks: Vec<tools::ToolLook>,
    /// Per tool: its shank and holder's lowest envelope (the non-cutting bodies).
    pub holders: Vec<Option<Cutter>>,
    /// Per setup: the part's triangles in that setup's frame (empty if it could not be built).
    pub part_tris: Vec<Vec<[[f64; 3]; 3]>>,
    /// Per setup: the part as columns on that setup's grid.
    pub part_maps: Vec<Option<PartMap>>,
    pub markers: Vec<Marker>,
    /// Each posted program run on its machine (by setup), or why it could not be.
    pub machines: Vec<Result<machine::MachineView, String>>,
    /// The machine run playing (an index into `machines`), or the toolpath (None).
    pub source: Option<usize>,
    /// The toolpath's own timeline and markers, while a machine's play.
    own: Option<(Timeline, Vec<Marker>)>,
    play: Option<Play>,
    /// The stock shown and the time it was made for.
    shown: Option<(f64, usize, ZMap)>,
    /// The detail window: a finer grid about the tool (`detail_at`).
    detail: Option<Detail>,
    /// The whole stock as an op began (its index): the detail window's start, kept while that op
    /// plays.
    op_start: Option<(u32, ZMap)>,
}

/// The default grid: about this many columns per setup.
pub const CELLS: usize = 160_000;

impl Sim {
    /// Runs the document (its part files beside `dir`), meshes each setup's part for display.
    pub fn open(src: &str, dir: &std::path::Path) -> Sim {
        let cam = forge_cam_lang::run(src, dir);
        let setups = cam.sim_setups();
        let tris = setups.iter().map(part_triangles).collect();
        Sim::new(cam, tris)
    }

    /// From a run and each setup's part triangles (in the setup's frame).
    pub fn new(cam: Cam, part_tris: Vec<Vec<[[f64; 3]; 3]>>) -> Sim {
        let tl = Timeline::of(&cam);
        let setups = cam.sim_setups();
        let cutters = cam.sim_tools().iter().map(|t| Cutter::of(&t.shape)).collect();
        let helix = tools::helices();
        let looks = cam
            .sim_tools()
            .iter()
            .map(|t| tools::build(t, helix.get(&t.tool.id).copied()))
            .collect();
        let holders = cam
            .sim_tools()
            .iter()
            .map(|t| Cutter::of_part(&t.shape, forge_cam_geom::tool::Part::NonCutting))
            .collect();
        let mut sim = Sim {
            cam,
            tl,
            setups,
            cutters,
            looks,
            holders,
            part_tris,
            part_maps: vec![],
            markers: vec![],
            machines: vec![],
            source: None,
            own: None,
            play: None,
            shown: None,
            detail: None,
            op_start: None,
        };
        sim.markers = sim.markers_of();
        // each setup's posted program on its machine
        sim.machines = (0..sim.setups.len())
            .map(|s| {
                let r = sim.cam.machine_run(s)?;
                Ok(machine::MachineView::of(&sim.cam, r, s))
            })
            .collect();
        sim.set_cells(CELLS);
        sim
    }

    /// The grid's size (the panel's accuracy): starts the stock again.
    pub fn set_cells(&mut self, cells: usize) {
        let cells = cells.clamp(1_000, 4_000_000);
        self.part_maps = (0..self.setups.len())
            .map(|s| {
                let g = self.blank(s, cells);
                let tris = self.part_tris.get(s)?;
                (!tris.is_empty()).then(|| PartMap::of(&g, tris))
            })
            .collect();
        let first = self
            .tl
            .segs
            .first()
            .map_or(0, |g| self.tl.ops[g.op as usize].setup);
        let cur = self.blank(first, cells);
        self.play = Some(Play {
            cells,
            checkpoints: vec![(0, cur.clone())],
            cur,
            done: 0,
            finals: vec![None; self.setups.len()],
            every: (self.tl.segs.len() / 48).max(16),
        });
        self.shown = None;
    }

    /// Plays machine run `m` (the posted text on its machine), or the toolpath (None).
    pub fn set_source(&mut self, m: Option<usize>) -> Result<(), String> {
        if m == self.source {
            return Ok(());
        }
        // back to the toolpath's own first
        if let Some((tl, mk)) = self.own.take() {
            self.tl = tl;
            self.markers = mk;
        }
        self.source = None;
        if let Some(i) = m {
            let mv = match self.machines.get(i) {
                Some(Ok(mv)) => mv,
                Some(Err(e)) => return Err(e.clone()),
                None => return Err("no such machine run".into()),
            };
            let tl = mv.tl.clone();
            let mut mk = mv.markers.clone();
            // the certified checks and leftover stay, placed on the machine's time by their op
            for m in &self.markers {
                let mut m = m.clone();
                if let Some(o) = m.op
                    && let (Some(a), Some(b)) = (self.tl.ops.get(o), tl.ops.get(o))
                {
                    let f = if a.t1 > a.t0 {
                        (m.t - a.t0) / (a.t1 - a.t0)
                    } else {
                        0.0
                    };
                    m.t = b.t0 + f * (b.t1 - b.t0);
                } else if m.op.is_none() {
                    m.t = tl
                        .ops
                        .iter()
                        .filter(|o| o.setup == m.setup)
                        .map(|o| o.t1)
                        .fold(0.0, f64::max);
                }
                mk.push(m);
            }
            let own_tl = std::mem::replace(&mut self.tl, tl);
            let own_mk = std::mem::replace(&mut self.markers, mk);
            self.own = Some((own_tl, own_mk));
            self.source = Some(i);
            // the posted code's rapids and holders against the stock as it cuts it
            let issues = self.stock_issues();
            self.markers.extend(issues);
            self.markers.sort_by(|a, b| a.t.total_cmp(&b.t));
        }
        let cells = self.cells();
        self.set_cells(cells);
        Ok(())
    }

    /// The stock as the playing timeline cuts it, checked on the grid (approximate, never a
    /// proven check: their rules are `rapid_into_stock_grid` and `holder_into_stock_grid`,
    /// apart from the certified checker's `rapid_into_stock`; their words say so): each rapid's cutter
    /// before it cuts, and each move's shank and holder after, against the material there is
    /// (a display-grid check, as fine as the stock's cells: the toolpath's own are the certified
    /// checker's; this is for the posted code on a machine). Markers, worst per move.
    pub fn stock_issues(&self) -> Vec<Marker> {
        let cells = self.cells();
        let mut out = vec![];
        let mut cur: Option<(usize, ZMap)> = None;
        for g in &self.tl.segs {
            let s = self.tl.ops[g.op as usize].setup;
            if cur.as_ref().is_none_or(|c| c.0 != s) {
                cur = Some((s, self.blank(s, cells)));
            }
            let Some((_, z)) = cur.as_mut() else { continue };
            let tool = g.tool as usize;
            let (Some(Some(c)), h) = (
                self.cutters.get(tool),
                self.holders.get(tool).and_then(|h| h.as_ref()),
            ) else {
                continue;
            };
            let name = &self.tl.ops[g.op as usize].name;
            if g.kind == timeline::Kind::Rapid
                && let Some((d, at)) = z.depth(g, c)
            {
                out.push(Marker {
                    op: Some(g.op as usize),
                    setup: s,
                    rule: "rapid_into_stock_grid".into(),
                    verdict: "fail".into(),
                    at,
                    t: g.t0,
                    text: format!(
                        "{name}: a rapid goes about {d:.2} mm into the stock as cut (move {}). {GRID_NOTE}",
                        g.mv
                    ),
                    row: format!("move {} \u{b7} about {d:.2} mm into the stock as cut", g.mv),
                    tool: String::new(),
                    part: "cutter".into(),
                    region: None,
                    planned: false,
                });
            }
            z.cut(g, 0.0, 1.0, c);
            if let Some(h) = h
                && let Some((d, at)) = z.depth(g, h)
            {
                out.push(Marker {
                    op: Some(g.op as usize),
                    setup: s,
                    rule: "holder_into_stock_grid".into(),
                    verdict: "fail".into(),
                    at,
                    t: g.t0,
                    text: format!(
                        "{name}: the shank or holder goes about {d:.2} mm into the stock as cut (move {}). {GRID_NOTE}",
                        g.mv
                    ),
                    row: format!("move {} \u{b7} about {d:.2} mm into the stock as cut", g.mv),
                    tool: String::new(),
                    part: "shank/holder".into(),
                    region: None,
                    planned: false,
                });
            }
        }
        out
    }

    /// The machine run playing, if one is.
    pub fn machine(&self) -> Option<&machine::MachineView> {
        self.machines.get(self.source?)?.as_ref().ok()
    }

    pub fn cells(&self) -> usize {
        self.play.as_ref().map_or(CELLS, |p| p.cells)
    }

    /// A setup's stock box as a fresh grid.
    fn blank(&self, s: usize, cells: usize) -> ZMap {
        match self.setups.get(s) {
            Some(su) => ZMap::new(su.stock.min, su.stock.max, cells),
            // a run with no setup (it stopped at an error): nothing to cut
            None => ZMap::new([0.0; 3], [1.0, 1.0, 0.0], 1),
        }
    }

    fn setup_of_seg(&self, i: usize) -> usize {
        self.tl.ops[self.tl.segs[i].op as usize].setup
    }

    /// The stock a setup starts from: its box, or the setup it continues turned over.
    fn start_of(&self, s: usize, finals: &[Option<ZMap>], cells: usize) -> ZMap {
        let su = &self.setups[s];
        match (su.from, su.turned) {
            (Some(p), Some(h)) => match finals.get(p).and_then(|f| f.as_ref()) {
                Some(f) => f.turned(h, su.stock.min, su.stock.max, cells),
                None => self.blank(s, cells),
            },
            _ => self.blank(s, cells),
        }
    }

    /// Plays the stock to `k` pieces done.
    fn advance(&mut self, k: usize) {
        let mut play = self.play.take().expect("set_cells made it");
        if k < play.done {
            // back to the latest checkpoint at or before k
            let (d, z) = play
                .checkpoints
                .iter()
                .rev()
                .find(|(d, _)| *d <= k)
                .cloned()
                .expect("one at 0");
            play.done = d;
            play.cur = z;
        }
        while play.done < k {
            let i = play.done;
            let s = self.setup_of_seg(i);
            if i > 0 {
                let prev = self.setup_of_seg(i - 1);
                if prev != s {
                    play.finals[prev] = Some(play.cur.clone());
                    play.cur = self.start_of(s, &play.finals, play.cells);
                    if !play.checkpoints.iter().any(|(d, _)| *d == i) {
                        play.checkpoints.push((i, play.cur.clone()));
                        play.checkpoints.sort_by_key(|c| c.0);
                    }
                }
            }
            let g = self.tl.segs[i];
            if let Some(Some(c)) = self.cutters.get(g.tool as usize) {
                play.cur.cut(&g, 0.0, 1.0, c);
            }
            play.done += 1;
            if play.done.is_multiple_of(play.every) && !play.checkpoints.iter().any(|(d, _)| *d == play.done)
            {
                play.checkpoints.push((play.done, play.cur.clone()));
                play.checkpoints.sort_by_key(|c| c.0);
            }
        }
        self.play = Some(play);
    }

    /// The stock at time t, and its setup: every piece before t's piece swept whole, t's piece
    /// up to t.
    pub fn stock_at(&mut self, t: f64) -> Option<(&ZMap, usize)> {
        let (_, i) = self.tl.pos(t)?;
        if self.shown.as_ref().is_some_and(|(st, _, _)| *st == t) {
            let (_, s, z) = self.shown.as_ref()?;
            return Some((z, *s));
        }
        // a setup's first piece: the stock it starts from
        self.advance(i);
        let s = self.setup_of_seg(i);
        let play = self.play.as_ref()?;
        let mut z = if i > 0 && self.setup_of_seg(i - 1) != s {
            // the setup before has just finished: its stock is the one played so far
            let mut finals = play.finals.clone();
            finals[self.setup_of_seg(i - 1)] = Some(play.cur.clone());
            self.start_of(s, &finals, play.cells)
        } else {
            play.cur.clone()
        };
        let g = self.tl.segs[i];
        let u = if g.dt > 0.0 {
            ((t - g.t0) / g.dt).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if let Some(Some(c)) = self.cutters.get(g.tool as usize) {
            z.cut(&g, 0.0, u, c);
        }
        self.shown = Some((t, s, z));
        let (_, s, z) = self.shown.as_ref()?;
        Some((z, *s))
    }

    /// The stock about the tool at time t, finer than the whole stock's grid (about 0.03 mm), so
    /// a ball's cusps and a stepover's marks are in it: a square window snapped to the whole
    /// grid's columns (which the drawing leaves out under it), its side about five tool radii,
    /// played from its setup's start through every piece that reaches it. It moves when the
    /// tool leaves its middle half (and is played again); time going back plays it again too.
    /// None in a setup turned over from another (its start is that setup's stock), or with no
    /// cutter. Display only, like the whole stock.
    pub fn detail_at(&mut self, t: f64) -> Option<(&ZMap, [usize; 4])> {
        self.detail_near(t, true)
    }

    /// `detail_at`, moving the window (a fresh start, some hundreds of ms) only if `may_move`
    /// (else None where it would have to move): playback keeps its pace and the window follows
    /// at the next pause.
    pub fn detail_near(&mut self, t: f64, may_move: bool) -> Option<(&ZMap, [usize; 4])> {
        let (p, i) = self.tl.pos(t)?;
        let s = self.setup_of_seg(i);
        let su = self.setups.get(s)?;
        if su.from.is_some() {
            return None;
        }
        let g = self.tl.segs[i];
        let r = self.cutters.get(g.tool as usize)?.as_ref()?.r;
        let half = (2.5 * r).clamp(4.0, 12.0);
        let coarse = self.blank(s, self.cells());
        let keep = self.detail.as_ref().is_some_and(|d| {
            d.setup == s
                && d.op == g.op
                && d.half == half
                && i + 1 >= d.done
                && (p[0] - d.centre[0]).abs() < 0.6 * half
                && (p[1] - d.centre[1]).abs() < 0.6 * half
        });
        if !keep && !may_move {
            return None;
        }
        if !keep {
            // the window's columns of the whole grid, about the tool
            let c = coarse.cell;
            let col = |x: f64, x0: f64, n: usize| (((x - x0) / c).round().max(0.0) as usize).min(n);
            let (i0, i1) = (
                col(p[0] - half, coarse.x0, coarse.nx),
                col(p[0] + half, coarse.x0, coarse.nx),
            );
            let (j0, j1) = (
                col(p[1] - half, coarse.y0, coarse.ny),
                col(p[1] + half, coarse.y0, coarse.ny),
            );
            if i1 <= i0 || j1 <= j0 {
                self.detail = None;
                return None;
            }
            let (x0, x1) = (coarse.x0 + i0 as f64 * c, coarse.x0 + i1 as f64 * c);
            let (y0, y1) = (coarse.y0 + j0 as f64 * c, coarse.y0 + j1 as f64 * c);
            let fine = 0.03f64.max((x1 - x0).max(y1 - y0) / 700.0);
            let n = ((x1 - x0) * (y1 - y0) / (fine * fine)).round() as usize;
            let (lo, hi) = (su.stock.min, su.stock.max);
            let mut z = ZMap::new([x0, y0, lo[2]], [x1, y1, hi[2]], n.max(1));
            // stamped no closer than the chord the smallest cutter leaves no ridge of over a
            // twentieth of a cell (s²/8r): a ball's 0.03 mm cell, r 3 → 0.19 mm
            let rmin = self
                .cutters
                .iter()
                .flatten()
                .map(|c| c.r)
                .fold(f64::INFINITY, f64::min);
            if rmin.is_finite() {
                z.step = Some((0.5 * z.cell).max((8.0 * rmin * z.cell / 20.0).sqrt()));
            }
            // from the whole stock as this op began (its columns interpolated onto the fine
            // grid), then the op's own pieces on the fine grid: what made the surface being
            // looked at is the op cutting it, and what earlier ops left shows as finely as the
            // whole grid has it (cutting only lowers columns: this is the whole stock's
            // surface wherever the op has not been yet)
            let first = self.tl.ops[g.op as usize].first;
            if self.op_start.as_ref().is_none_or(|(o, _)| *o != g.op) {
                let before = if first == 0 || self.setup_of_seg(first - 1) != s {
                    self.blank(s, self.cells())
                } else {
                    self.advance(first);
                    self.play.as_ref()?.cur.clone()
                };
                self.op_start = Some((g.op, before));
            }
            z.resample(&self.op_start.as_ref()?.1);
            self.detail = Some(Detail {
                setup: s,
                op: g.op,
                centre: [p[0], p[1]],
                half,
                cols: [i0, j0, i1, j1],
                z,
                done: first,
                shown: None,
            });
        }
        let mut d = self.detail.take()?;
        // every whole piece before t's that reaches the window
        let (wx0, wy0) = (d.z.x0, d.z.y0);
        let (wx1, wy1) = (wx0 + d.z.nx as f64 * d.z.cell, wy0 + d.z.ny as f64 * d.z.cell);
        let reaches = |g: &timeline::Seg, r: f64| {
            let (mut lo, mut hi) = (
                [g.a[0].min(g.b[0]), g.a[1].min(g.b[1])],
                [g.a[0].max(g.b[0]), g.a[1].max(g.b[1])],
            );
            if let Some(a) = &g.arc {
                let rr = a.r + a.dr.abs();
                lo = [a.c[0] - rr, a.c[1] - rr];
                hi = [a.c[0] + rr, a.c[1] + rr];
            }
            !(hi[0] + r < wx0 || lo[0] - r > wx1 || hi[1] + r < wy0 || lo[1] - r > wy1)
        };
        if d.done > i {
            // time went back: from the op's start again
            self.detail = None;
            return self.detail_near(t, may_move);
        }
        while d.done < i {
            let g = self.tl.segs[d.done];
            if let Some(Some(c)) = self.cutters.get(g.tool as usize)
                && reaches(&g, c.r)
            {
                d.z.cut(&g, 0.0, 1.0, c);
            }
            d.done += 1;
        }
        // t's piece up to t
        let mut z = d.z.clone();
        let u = if g.dt > 0.0 {
            ((t - g.t0) / g.dt).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if let Some(Some(c)) = self.cutters.get(g.tool as usize)
            && reaches(&g, c.r)
        {
            z.cut(&g, 0.0, u, c);
        }
        d.shown = Some(z);
        self.detail = Some(d);
        let d = self.detail.as_ref()?;
        Some((d.shown.as_ref()?, d.cols))
    }

    /// The stock last made by `stock_at`, and its setup.
    pub fn stock_shown(&self) -> Option<(&ZMap, usize)> {
        self.shown.as_ref().map(|(_, s, z)| (z, *s))
    }

    /// The certified checks that did not pass, and G4's leftover regions, as markers.
    fn markers_of(&self) -> Vec<Marker> {
        let mut out: Vec<Marker> = vec![];
        for (oi, op) in self.cam.ops.iter().enumerate() {
            let paths = self.cam.sim_paths(oi);
            // an op refused before it had moves (planning said no): one marker at its place on
            // the timeline, over its setup's stock, saying why, so it is never silently absent
            let had = out.len();
            for c in &op.checks {
                if c.verdict == "pass" {
                    continue;
                }
                // the path whose move `move_index` is nearest the check's place
                let mut best: Option<(f64, f64)> = None;
                for (pi, p) in paths.iter().enumerate() {
                    if c.move_index >= p.moves.len() {
                        continue;
                    }
                    if let Some(t) = self.tl.locate(oi, pi, c.move_index, 0.5) {
                        let (q, _) = self.tl.pos(t).unwrap_or(([f64::INFINITY; 3], 0));
                        let d = (0..3).map(|k| (q[k] - c.at[k]).ipow(2)).sum::<f64>();
                        if best.is_none_or(|b| d < b.0) {
                            best = Some((d, t));
                        }
                    }
                }
                // the check's own path where it names one with that move, else the nearest
                let t = (c.move_index < paths.get(c.path).map_or(0, |p| p.moves.len()))
                    .then(|| self.tl.locate(oi, c.path, c.move_index, 0.5))
                    .flatten()
                    .or(best.map(|b| b.1))
                    .or_else(|| self.tl.nearest(oi, c.at))
                    .unwrap_or(self.tl.ops[oi].t0);
                // an op of several tools (a spot drill, then the drill): whose move it is
                let tool = match self.tl.pos(t) {
                    Some((_, seg)) if self.tl.ops[oi].tools.len() > 1 => self
                        .cam
                        .sim_tools()
                        .get(self.tl.segs[seg].tool as usize)
                        .map(tools::tool_words)
                        .unwrap_or_default(),
                    _ => String::new(),
                };
                out.push(Marker {
                    op: Some(oi),
                    setup: op.setup,
                    rule: check_kind(c),
                    verdict: c.verdict.to_string(),
                    at: c.at,
                    t,
                    text: format!("{}: {}", op.name, check_sentence(c)),
                    row: check_row(c),
                    tool,
                    part: c.tool_part.to_string(),
                    region: None,
                    planned: false,
                });
            }
            if op.status == "refused" && out.len() == had {
                let at = self.setups.get(op.setup).map_or([0.0; 3], |su| {
                    let (a, b) = (su.stock.min, su.stock.max);
                    [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1]), b[2]]
                });
                out.push(Marker {
                    op: Some(oi),
                    setup: op.setup,
                    rule: "refused".into(),
                    verdict: "fail".into(),
                    at,
                    t: self.tl.ops.get(oi).map_or(0.0, |sp| sp.t0),
                    text: format!(
                        "{} refused: {}",
                        op.name,
                        op.refusal.as_deref().unwrap_or("no reason given")
                    ),
                    row: tidy(op.refusal.as_deref().unwrap_or("no reason given")),
                    tool: String::new(),
                    part: String::new(),
                    region: None,
                    planned: false,
                });
            }
        }
        for l in &self.cam.leftover {
            // shown once the setup's last op is done
            let t = self
                .tl
                .ops
                .iter()
                .filter(|o| o.setup == l.setup)
                .map(|o| o.t1)
                .fold(0.0, f64::max);
            // the part's bottom in this setup: material wholly under it is left by the plan
            let bottom = self
                .setups
                .get(l.setup)
                .map_or(f64::NEG_INFINITY, |s| s.part_box.0[2]);
            let planned = l.z1 <= bottom + 1e-6;
            for b in &l.boxes {
                let (w, d) = (b[2] - b[0], b[3] - b[1]);
                let place = if planned {
                    "Under the part"
                } else {
                    "Left on the part"
                };
                out.push(Marker {
                    op: None,
                    setup: l.setup,
                    rule: "leftover".into(),
                    verdict: l.why.into(),
                    at: [0.5 * (b[0] + b[2]), 0.5 * (b[1] + b[3]), l.z1],
                    t,
                    text: format!(
                        "{place}, z {:.2} to {:.2} mm, {w:.1} \u{d7} {d:.1} mm ({})",
                        l.z0, l.z1, l.why
                    ),
                    row: format!(
                        "z {:.2} to {:.2} mm \u{b7} {w:.1} \u{d7} {d:.1} mm \u{b7} {}",
                        l.z0, l.z1, l.why
                    ),
                    tool: String::new(),
                    part: String::new(),
                    region: Some((*b, l.z0, l.z1)),
                    planned,
                });
            }
        }
        out.sort_by(|a, b| a.t.total_cmp(&b.t));
        out
    }
}

/// The part of a setup as triangles in that setup's frame (built and meshed for display; empty
/// if it does not build).
pub fn part_triangles(s: &SimSetup) -> Vec<[[f64; 3]; 3]> {
    let opts = forge_lang::Options {
        build: true,
        dir: Some(s.part_dir.clone()),
        ..Default::default()
    };
    let (run, _) = forge_lang::run(&s.part_src, opts, Default::default());
    let Some(body) = run.body else { return vec![] };
    let q = forge_mesh::Quality {
        chord: 0.02,
        ..Default::default()
    };
    let Ok(m) = forge_mesh::tessellate(&body, &q) else {
        return vec![];
    };
    let map = |p: forge_math::Point3| -> [f64; 3] {
        match s.turned {
            Some(h) => [p.x, -p.y, h - p.z],
            None => [p.x, p.y, p.z],
        }
    };
    m.triangles
        .iter()
        .map(|t| {
            let v = t.map(|i| map(m.positions[i as usize]));
            // turning over mirrors y and z: the winding is kept (two reflections)
            v
        })
        .collect()
}

/// A failed check in words: who meets what, on which motion, how far (rounded, with units).
/// "Holder hits the stock on the plunge at move 3 (2.75 mm into it)".
pub fn check_sentence(c: &forge_cam::ops::CheckOut) -> String {
    let who = match c.tool_part {
        "" => "The tool".to_string(),
        p => {
            let mut w = p.replace('_', " ");
            if let Some(f) = w.get(0..1) {
                w = f.to_uppercase() + &w[1..];
            }
            w
        }
    };
    let what = {
        let o = c.other.to_ascii_lowercase();
        if o.starts_with("stock") {
            "the stock".to_string()
        } else if o.starts_with("part") {
            "the part".to_string()
        } else if o.contains("fixture")
            || o.contains("vise")
            || o.contains("clamp")
            || o.contains("workholding")
        {
            "the workholding".to_string()
        } else if o.is_empty() {
            "the part".to_string()
        } else {
            c.other.clone()
        }
    };
    let phase = match c.phase {
        "" => String::new(),
        p => format!(" on the {}", p.replace('_', " ")),
    };
    let mm = |v: f64| format!("{:.2} mm", v.abs());
    if c.value < -0.005 {
        format!(
            "{who} hits {what}{phase} at move {} ({} into it)",
            c.move_index,
            mm(c.value)
        )
    } else if c.value < 0.005 {
        format!(
            "{who} touches {what}{phase} at move {} (needs {} clear)",
            c.move_index,
            mm(c.limit)
        )
    } else {
        format!(
            "{who} comes {} from {what}{phase} at move {} (needs {})",
            mm(c.value),
            c.move_index,
            mm(c.limit)
        )
    }
}

/// What a stock marker from the display grid says of itself: it is approximate, not proven.
pub const GRID_NOTE: &str = "Approximate: measured on the simulation's display grid, not a proven check.";

/// A failed check's kind, by what happened: a rapid that goes into the stock, the part or the
/// workholding is a crash, not a near miss, and is a kind of its own ("rapid_into_stock", ..);
/// otherwise the checker's rule.
pub fn check_kind(c: &forge_cam::ops::CheckOut) -> String {
    if c.rule == "rapid_clear" && c.value < -0.005 {
        return format!("rapid_into_{}", what_word(&c.other));
    }
    c.rule.to_string()
}

/// What a check's `other` names, in a word: stock, part or workholding (else as given).
fn what_word(other: &str) -> String {
    let o = other.to_ascii_lowercase();
    if o.starts_with("stock") {
        "stock".into()
    } else if o.starts_with("part") || o.is_empty() {
        "part".into()
    } else if ["fixture", "vise", "clamp", "workholding"]
        .iter()
        .any(|w| o.contains(w))
    {
        "workholding".into()
    } else {
        other.to_string()
    }
}

/// A failed check's row under its kind: when (the move), where and how much, not what (the
/// heading says that): "move 0 · 0.35 mm into the stock".
pub fn check_row(c: &forge_cam::ops::CheckOut) -> String {
    let what = format!("the {}", what_word(&c.other));
    let mm = |v: f64| format!("{:.2} mm", v.abs());
    let how = if c.value < -0.005 {
        format!("{} into {what}", mm(c.value))
    } else if c.value < 0.005 {
        format!("touches {what}, needs {} clear", mm(c.limit))
    } else {
        format!("{} from {what}, needs {}", mm(c.value), mm(c.limit))
    };
    format!("move {} \u{b7} {how}", c.move_index)
}

/// A run's message tidied for a person: a parser's `NotPrism { face: 38 }` said in words and
/// numbers with long tails rounded to 3 decimals. The message itself stays on hover.
pub fn tidy(msg: &str) -> String {
    // a leading op key ("thread_mill: ...") is the run's, not a sentence's
    let msg = match msg.split_once(": ") {
        Some((k, rest)) if !k.is_empty() && k.chars().all(|c| c.is_ascii_lowercase() || c == '_') => {
            let mut r = rest.to_string();
            if let Some(f) = r.get(0..1) {
                r = f.to_uppercase() + &r[1..];
            }
            r
        }
        _ => msg.to_string(),
    };
    let s = msg
        .replace("NotPrism { face: ", "isn't a prism (face ")
        .replace(" }", ")");
    let mut out = String::with_capacity(s.len());
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let start = i;
        let neg = cs[i] == '-' && i + 1 < cs.len() && cs[i + 1].is_ascii_digit();
        if cs[i].is_ascii_digit() || neg {
            i += 1;
            while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') {
                i += 1;
            }
            let tok: String = cs[start..i].iter().collect();
            let decimals = tok.split_once('.').map_or(0, |(_, d)| d.len());
            match tok.parse::<f64>() {
                Ok(v) if decimals > 3 => out.push_str(&format!("{v:.3}")),
                _ => out.push_str(&tok),
            }
        } else {
            out.push(cs[i]);
            i += 1;
        }
    }
    out
}

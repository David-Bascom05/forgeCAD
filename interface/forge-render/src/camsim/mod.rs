//! The toolpath and stock simulation's state, free of the GPU and of egui (so it is tested
//! headless and is the same on both targets): the run's timeline, its tools, each setup's part
//! and stock, the display stock played to any time (checkpoints make scrubbing back cheap), and
//! the certified checks as markers on the timeline.
//!
//! The rule it keeps: what it draws is a preview; what it says about the program (pass, fail,
//! what is left) comes from the run's certified checks and its exact G4 leftover, never from the
//! dexels.

pub mod draw;
pub mod machine;
pub mod timeline;
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
    /// A leftover region's box (xmin ymin xmax ymax, z0 z1).
    pub region: Option<([f64; 4], f64, f64)>,
}

impl Marker {
    /// Collision-like (red) or leftover-like (amber).
    pub fn severe(&self) -> bool {
        self.rule != "leftover"
    }
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
            holders,
            part_tris,
            part_maps: vec![],
            markers: vec![],
            machines: vec![],
            source: None,
            own: None,
            play: None,
            shown: None,
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

    /// The stock as the playing timeline cuts it, checked on the grid: each rapid's cutter
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
                    rule: "rapid_into_stock".into(),
                    verdict: "fail".into(),
                    at,
                    t: g.t0,
                    text: format!(
                        "{name}: a rapid goes {d:.3} mm into the stock as cut (move {})",
                        g.mv
                    ),
                    region: None,
                });
            }
            z.cut(g, 0.0, 1.0, c);
            if let Some(h) = h
                && let Some((d, at)) = z.depth(g, h)
            {
                out.push(Marker {
                    op: Some(g.op as usize),
                    setup: s,
                    rule: "holder_into_stock".into(),
                    verdict: "fail".into(),
                    at,
                    t: g.t0,
                    text: format!(
                        "{name}: the shank or holder goes {d:.3} mm into the stock as cut (move {})",
                        g.mv
                    ),
                    region: None,
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

    /// The stock last made by `stock_at`, and its setup.
    pub fn stock_shown(&self) -> Option<(&ZMap, usize)> {
        self.shown.as_ref().map(|(_, s, z)| (z, *s))
    }

    /// The certified checks that did not pass, and G4's leftover regions, as markers.
    fn markers_of(&self) -> Vec<Marker> {
        let mut out = vec![];
        for (oi, op) in self.cam.ops.iter().enumerate() {
            let paths = self.cam.sim_paths(oi);
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
                        let d = (0..3).map(|k| (q[k] - c.at[k]).powi(2)).sum::<f64>();
                        if best.is_none_or(|b| d < b.0) {
                            best = Some((d, t));
                        }
                    }
                }
                let t = best
                    .map(|b| b.1)
                    .or_else(|| self.tl.nearest(oi, c.at))
                    .unwrap_or(self.tl.ops[oi].t0);
                out.push(Marker {
                    op: Some(oi),
                    setup: op.setup,
                    rule: c.rule.to_string(),
                    verdict: c.verdict.to_string(),
                    at: c.at,
                    t,
                    text: format!(
                        "{} {} — {} {} at move {} ({}): {:.4} vs {:.4} mm",
                        op.name, c.rule, c.tool_part, c.verdict, c.move_index, c.phase, c.value, c.limit
                    ) + &if c.other.is_empty() {
                        String::new()
                    } else {
                        format!(" on {}", c.other)
                    },
                    region: None,
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
            for b in &l.boxes {
                out.push(Marker {
                    op: None,
                    setup: l.setup,
                    rule: "leftover".into(),
                    verdict: l.why.into(),
                    at: [0.5 * (b[0] + b[2]), 0.5 * (b[1] + b[3]), l.z1],
                    t,
                    text: format!(
                        "leftover ({}) z {:.3}..{:.3}, x {:.2}..{:.2} y {:.2}..{:.2}",
                        l.why, l.z0, l.z1, b[0], b[2], b[1], b[3]
                    ),
                    region: Some((*b, l.z0, l.z1)),
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

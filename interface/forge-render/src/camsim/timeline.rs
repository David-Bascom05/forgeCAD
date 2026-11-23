//! The program as a timeline: every move of every op, in order, each with its start time and
//! duration, so a time finds the tool's place and a check's move finds its time. Built from the
//! real run (`forge_cam_lang::run`): the paths as posted where a program posted them (what the
//! certified checks ran on), else as planned. Times: feeds as programmed, rapids at the run's
//! rate (`RAPID`, the figure forge-cam-lang's own `time_s` uses), canned cycles as their motion.

use forge_cam::ops::{OpPath, PMove, Phase};
use forge_cam_lang::Cam;

/// mm/min a rapid is timed at (forge-cam-lang's `time_s` takes the same).
pub const RAPID: f64 = 10_000.0;

/// What kind of motion a piece is: how it is drawn and coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Rapid,
    Cut,
    Plunge,
    Link,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Rapid => "rapid",
            Kind::Cut => "cut",
            Kind::Plunge => "plunge",
            Kind::Link => "link",
        }
    }

    fn of(p: Phase) -> Kind {
        match p {
            Phase::Cut => Kind::Cut,
            Phase::Plunge => Kind::Plunge,
            Phase::Link => Kind::Link,
            Phase::Rapid => Kind::Rapid,
        }
    }
}

/// An arc in xy about `c`, the height linear in the angle (a helix when the ends' z differ).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcOf {
    pub c: [f64; 2],
    pub r: f64,
    /// Start angle and signed sweep, radians.
    pub a0: f64,
    pub sweep: f64,
}

/// One piece of motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg {
    pub op: u32,
    pub path: u32,
    /// The move's index in the played path (`OpPath::moves`): what a check's `move_index` names.
    pub mv: u32,
    /// The tool (`Cam::sim_tools` index).
    pub tool: u32,
    pub kind: Kind,
    pub a: [f64; 3],
    pub b: [f64; 3],
    pub arc: Option<ArcOf>,
    /// mm/min.
    pub f: f64,
    /// Start time and duration, seconds.
    pub t0: f64,
    pub dt: f64,
    pub len: f64,
}

impl Seg {
    /// The tool tip at fraction u ∈ [0, 1] of the piece.
    pub fn at(&self, u: f64) -> [f64; 3] {
        let u = u.clamp(0.0, 1.0);
        let z = self.a[2] + (self.b[2] - self.a[2]) * u;
        match self.arc {
            None => [
                self.a[0] + (self.b[0] - self.a[0]) * u,
                self.a[1] + (self.b[1] - self.a[1]) * u,
                z,
            ],
            Some(ar) => {
                let t = ar.a0 + ar.sweep * u;
                [ar.c[0] + ar.r * t.cos(), ar.c[1] + ar.r * t.sin(), z]
            }
        }
    }

    /// Points along it for drawing: ends included, arcs every `step` radians at most.
    pub fn points(&self, step: f64, out: &mut Vec<[f64; 3]>) {
        out.push(self.a);
        if let Some(ar) = self.arc {
            let n = ((ar.sweep.abs() / step).ceil() as usize).clamp(1, 720);
            for k in 1..n {
                out.push(self.at(k as f64 / n as f64));
            }
        }
        out.push(self.b);
    }

    pub fn t1(&self) -> f64 {
        self.t0 + self.dt
    }
}

/// One op's place on the timeline.
#[derive(Clone, Debug)]
pub struct OpSpan {
    pub name: String,
    pub kind: &'static str,
    pub setup: usize,
    pub status: &'static str,
    /// Its pieces, `segs[first..end]`.
    pub first: usize,
    pub end: usize,
    pub t0: f64,
    pub t1: f64,
    /// Each path's tool.
    pub tools: Vec<u32>,
}

/// The whole program in time.
#[derive(Clone, Debug, Default)]
pub struct Timeline {
    pub segs: Vec<Seg>,
    pub ops: Vec<OpSpan>,
    pub total: f64,
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
}

/// Pushes the pieces of one path, from time `t`; returns the time after it.
pub fn push_path(op: u32, path: u32, p: &OpPath, segs: &mut Vec<Seg>, mut t: f64) -> f64 {
    let tool = p.tool as u32;
    let mut at = p.start;
    let put = |segs: &mut Vec<Seg>,
               mv: usize,
               kind: Kind,
               a: [f64; 3],
               b: [f64; 3],
               arc: Option<ArcOf>,
               f: f64,
               t: &mut f64| {
        let len = match arc {
            None => dist(a, b),
            Some(ar) => ((ar.r * ar.sweep).powi(2) + (b[2] - a[2]).powi(2)).sqrt(),
        };
        let rate = if kind == Kind::Rapid || f <= 0.0 { RAPID } else { f };
        let dt = len / rate * 60.0;
        segs.push(Seg {
            op,
            path,
            mv: mv as u32,
            tool,
            kind,
            a,
            b,
            arc,
            f: rate,
            t0: *t,
            dt,
            len,
        });
        *t += dt;
    };
    for (i, m) in p.moves.iter().enumerate() {
        match *m {
            PMove::Rapid { to } => {
                put(segs, i, Kind::Rapid, at, to, None, RAPID, &mut t);
                at = to;
            }
            PMove::Line { to, f, phase } => {
                put(segs, i, Kind::of(phase), at, to, None, f, &mut t);
                at = to;
            }
            PMove::Arc { to, c, cw, f } => {
                let r = ((at[0] - c[0]).powi(2) + (at[1] - c[1]).powi(2)).sqrt();
                let a0 = (at[1] - c[1]).atan2(at[0] - c[0]);
                let mut a1 = (to[1] - c[1]).atan2(to[0] - c[0]);
                let tau = std::f64::consts::TAU;
                // as the checker reads it: the same end is a whole turn
                if cw {
                    while a1 >= a0 {
                        a1 -= tau;
                    }
                } else {
                    while a1 <= a0 {
                        a1 += tau;
                    }
                }
                let arc = ArcOf {
                    c,
                    r,
                    a0,
                    sweep: a1 - a0,
                };
                put(segs, i, Kind::Cut, at, to, Some(arc), f, &mut t);
                at = to;
            }
            PMove::Cycle {
                kind,
                holes,
                z,
                r,
                clear,
                q,
                f,
                ..
            } => {
                let (x, y) = (holes[0], holes[1]);
                put(segs, i, Kind::Rapid, at, [x, y, clear], None, RAPID, &mut t);
                put(
                    segs,
                    i,
                    Kind::Rapid,
                    [x, y, clear],
                    [x, y, r],
                    None,
                    RAPID,
                    &mut t,
                );
                // pecks: down by q, back out (to R for a peck, a little for a chip break)
                let pecking = matches!(
                    kind,
                    forge_post::CycleKind::Peck | forge_post::CycleKind::ChipBreak
                );
                let mut top = r;
                if pecking && q > 0.0 {
                    let mut d = r;
                    while d - q > z {
                        let nd = d - q;
                        put(segs, i, Kind::Plunge, [x, y, top], [x, y, nd], None, f, &mut t);
                        let back = if kind == forge_post::CycleKind::Peck {
                            r
                        } else {
                            nd + 0.5
                        };
                        put(segs, i, Kind::Link, [x, y, nd], [x, y, back], None, RAPID, &mut t);
                        top = back;
                        d = nd;
                    }
                }
                put(segs, i, Kind::Plunge, [x, y, top], [x, y, z], None, f, &mut t);
                put(segs, i, Kind::Link, [x, y, z], [x, y, clear], None, RAPID, &mut t);
                at = [x, y, clear];
            }
        }
    }
    t
}

impl Timeline {
    /// The run's ops in order (ops without paths take no time).
    pub fn of(cam: &Cam) -> Timeline {
        let mut tl = Timeline::default();
        let mut t = 0.0;
        for (oi, op) in cam.ops.iter().enumerate() {
            let first = tl.segs.len();
            let t0 = t;
            let paths = cam.sim_paths(oi);
            for (pi, p) in paths.iter().enumerate() {
                t = push_path(oi as u32, pi as u32, p, &mut tl.segs, t);
            }
            tl.ops.push(OpSpan {
                name: op.name.clone(),
                kind: op.kind,
                setup: op.setup,
                status: op.status,
                first,
                end: tl.segs.len(),
                t0,
                t1: t,
                tools: paths.iter().map(|p| p.tool as u32).collect(),
            });
        }
        tl.total = t;
        tl
    }

    /// The piece playing at time t (the last one at or past the end).
    pub fn seg_at(&self, t: f64) -> Option<usize> {
        if self.segs.is_empty() {
            return None;
        }
        let i = self.segs.partition_point(|s| s.t1() <= t);
        Some(i.min(self.segs.len() - 1))
    }

    /// The tool tip at time t, and the piece.
    pub fn pos(&self, t: f64) -> Option<([f64; 3], usize)> {
        let i = self.seg_at(t)?;
        let s = &self.segs[i];
        let u = if s.dt > 0.0 { (t - s.t0) / s.dt } else { 1.0 };
        Some((s.at(u), i))
    }

    /// The op playing at time t.
    pub fn op_at(&self, t: f64) -> Option<usize> {
        let i = self.seg_at(t)?;
        Some(self.segs[i].op as usize)
    }

    /// The time of move `mv` of op `op`'s path `path` (where it starts), at fraction s of it.
    pub fn locate(&self, op: usize, path: usize, mv: usize, s: f64) -> Option<f64> {
        let sp = self.ops.get(op)?;
        let pieces = &self.segs[sp.first..sp.end];
        let k = pieces
            .iter()
            .position(|g| g.path as usize == path && g.mv as usize == mv)?;
        let n = pieces[k..]
            .iter()
            .take_while(|g| g.path as usize == path && g.mv as usize == mv)
            .count();
        let (a, b) = (pieces[k].t0, pieces[k + n - 1].t1());
        Some(a + (b - a) * s.clamp(0.0, 1.0))
    }

    /// The time the tool is nearest `p` within op `op` (a check's `at` when its move is unknown).
    pub fn nearest(&self, op: usize, p: [f64; 3]) -> Option<f64> {
        let sp = self.ops.get(op)?;
        let mut best = (f64::INFINITY, None);
        for g in &self.segs[sp.first..sp.end] {
            let n = if g.arc.is_some() { 32 } else { 8 };
            for k in 0..=n {
                let u = k as f64 / n as f64;
                let d = dist(g.at(u), p);
                if d < best.0 {
                    best = (d, Some(g.t0 + g.dt * u));
                }
            }
        }
        best.1
    }

    /// The start of the piece after (or before, `back`) the one at t.
    pub fn step(&self, t: f64, back: bool) -> f64 {
        let Some(i) = self.seg_at(t) else { return 0.0 };
        if back {
            let s = &self.segs[i];
            if t > s.t0 + 1e-9 {
                s.t0
            } else if i > 0 {
                self.segs[i - 1].t0
            } else {
                0.0
            }
        } else if i + 1 < self.segs.len() {
            self.segs[i + 1].t0
        } else {
            self.total
        }
    }
}

/// m:ss or h:mm:ss.
pub fn clock(t: f64) -> String {
    let s = t.max(0.0);
    let (h, m, sec) = ((s / 3600.0) as u64, ((s % 3600.0) / 60.0) as u64, s % 60.0);
    if h > 0 {
        format!("{h}:{m:02}:{sec:04.1}")
    } else {
        format!("{m}:{sec:04.1}")
    }
}

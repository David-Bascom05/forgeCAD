//! Stage bending: a whole program's tools set out as stations along the bed, every one on the
//! press for every step, each bend made at the station holding its punch and die.
//!
//! The planner's programs ([`crate::programs`]) say which punch and die pairs a bend order needs.
//! Here each pair becomes a station: its stacks as long as its longest bend needs, set left to
//! right in the order the program first uses them, far enough apart that the part, wherever it
//! reaches along the bed at its own station, stays off the next station's tools. The layout is
//! then simulated whole: every step at its station with every other station's tools on the
//! press, through every check, certified as any step is. A program whose layout does not fit
//! the bed, or fails a check with the neighbours there, gives way to a tighter layout, then to
//! the next program.
use crate::catalog::Speeds;
use crate::model::Model;
use crate::plan::{Options, Plan, Program, programs, simulate};
use crate::sim::{Check, Rule, Setup, StepCtx};
use crate::tooling::{PunchKind, Station, Tooling};

/// Stations are set on this grid along the bed (mm): what a setter measures to.
pub const GRID: f64 = 5.0;

/// Ram travel per step, mm: from top dead centre to the sheet (approach), through the bend
/// (work), and back up (return).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Travel {
    pub approach: f64,
    pub work: f64,
    pub ret: f64,
}

impl Travel {
    /// Seconds at the press's speeds, when every one is known.
    pub fn seconds(&self, sp: &Speeds) -> Option<f64> {
        Some(self.approach / sp.approach? + self.work / sp.work? + self.ret / sp.ret?)
    }
}

/// One station of the layout and why it holds what it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct StationUse {
    pub punch: String,
    pub die: String,
    pub kind: PunchKind,
    /// Its centre along the bed from the bed's centre, and its stacks' lengths.
    pub y: f64,
    pub punch_length: f64,
    pub die_length: f64,
    /// The steps made here (indices into the plan's steps).
    pub steps: Vec<usize>,
    /// Where the part reaches along the bed while it is here, from the station's centre.
    pub reach: (f64, f64),
    /// The least distance of the part to this station's punch over its steps (the certified
    /// check: its value, its rule, the part and the instant), when the punch is checked by
    /// distance.
    pub punch_clear: Option<Check>,
    /// Punches tried before this one for its first bend, and the rule each failed (from the
    /// search: the reason a straight punch was not used).
    pub instead_of: Vec<(String, Rule)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Staged {
    /// The tooling with the stations set.
    pub tooling: Tooling,
    /// The program simulated with every station on the press.
    pub plan: Plan,
    /// Per step, its station (index into `stations`).
    pub station_of: Vec<usize>,
    pub stations: Vec<StationUse>,
    pub flips: usize,
    pub turns: usize,
    /// Steps whose station is not the one before: the part carried along the bed.
    pub shifts: usize,
    /// Per step, the ram's travel.
    pub travel: Vec<Travel>,
    /// Programs and layouts tried before this one passed.
    pub programs_tried: usize,
    pub layouts_tried: usize,
    /// The layout packed tight (stations' stacks closer than the part's reach): the neighbours
    /// were checked against the part and passed.
    pub tight: bool,
}

impl Staged {
    /// The press's motion over the whole program, seconds, at its published speeds (None: a
    /// speed is not known). Handling, station shifts and tool setting are not counted.
    pub fn ram_seconds(&self, sp: &Speeds) -> Option<f64> {
        self.travel.iter().map(|t| t.seconds(sp)).sum()
    }
}

/// Why no staged program: none feasible at all (the planner's reply says why), or none whose
/// stations fit and pass together (the best attempt, simulated with its stations).
#[derive(Clone, Debug, PartialEq)]
pub enum StageFail {
    NoProgram(Plan),
    NoLayout { why: String, attempt: Option<Box<Plan>> },
}

/// The bends done before each step of a program.
fn done_before(m: &Model, seq: &[usize]) -> Vec<Vec<bool>> {
    let mut done = vec![false; m.bends.len()];
    let mut out = Vec::new();
    for &j in seq {
        out.push(done.clone());
        done[j] = true;
    }
    out
}

/// A program's stations, in the order it first uses them, and each step's index into them. A
/// station is a punch and a die and their stacks' lengths: a step works at one holding its tools
/// whose stacks cover the bend less its allowed ends and stand inside it (clear of flanges
/// standing at its ends), else a new one is set with the step's own lengths. So a box's last
/// sides, which want shorter stacks than its first, get a station of their own with the same tools.
fn pairs(m: &Model, tl: &Tooling, prog: &Program) -> Option<(Vec<Station>, Vec<usize>)> {
    let mut ps: Vec<Station> = Vec::new();
    let mut of = Vec::new();
    for st in &prog.plan.steps {
        let (pl, dl) = (st.punch_length?, st.die_length?);
        let b = &m.bends[st.bend];
        let need = (b.len - 2.0 * tl.short_end.unwrap_or(m.t.max(3.0))).max(0.0);
        let room = b.len - 2.0 * tl.clearance;
        let fits = |x: f64| x >= need && x <= room + 1e-9;
        let i = match ps.iter().position(|s| {
            (s.punch, s.die) == (st.setup.punch, st.setup.die) && fits(s.punch_length) && fits(s.die_length)
        }) {
            Some(i) => i,
            None => {
                ps.push(Station {
                    punch: st.setup.punch,
                    die: st.setup.die,
                    y: 0.0,
                    punch_length: pl,
                    die_length: dl,
                });
                ps.len() - 1
            }
        };
        of.push(i);
    }
    Some((ps, of))
}

/// A station layout for a program: None when a stack's length is not known or the stations
/// will not fit the bed. `tight` packs the stacks a margin apart whatever the part's reach.
fn layout(m: &Model, tl: &Tooling, prog: &Program, tight: bool) -> Option<(Vec<Station>, Vec<(f64, f64)>)> {
    let (ps, of) = pairs(m, tl, prog)?;
    let steps = &prog.plan.steps;
    let dones = done_before(m, &prog.plan.sequence);
    let bare = Tooling {
        stations: Vec::new(),
        ..tl.clone()
    };
    let len: Vec<(f64, f64)> = ps.iter().map(|s| (s.punch_length, s.die_length)).collect();
    let mut reach = vec![(f64::INFINITY, f64::NEG_INFINITY); ps.len()];
    for (i, st) in steps.iter().enumerate() {
        let k = of[i];
        let ctx = StepCtx::new(m, &bare, &dones[i], st.bend, st.setup, st.overbend);
        let (lo, hi) = ctx.part_span_y();
        reach[k] = (reach[k].0.min(lo), reach[k].1.max(hi));
    }
    let width = |k: usize| len[k].0.max(len[k].1);
    // a margin past the clearance, and the grid's rounding taken back
    let margin = 10.0 + GRID;
    let mut ys = vec![0.0f64];
    for k in 1..ps.len() {
        let packed = 0.5 * (width(k - 1) + width(k));
        let gap = if tight {
            packed
        } else {
            // the part at k-1 off station k's stacks, and the part at k off k-1's
            packed
                .max(reach[k - 1].1 + 0.5 * width(k))
                .max(-reach[k].0 + 0.5 * width(k - 1))
        };
        ys.push(ys[k - 1] + gap + margin);
    }
    let lo = ys[0] - 0.5 * width(0);
    let hi = ys[ps.len() - 1] + 0.5 * width(ps.len() - 1);
    if hi - lo > tl.press.bed_length {
        return None;
    }
    let shift = -0.5 * (lo + hi);
    let half = 0.5 * tl.press.bed_length;
    let stations: Vec<Station> = ps
        .iter()
        .enumerate()
        .map(|(k, st)| {
            let h = 0.5 * width(k);
            let y = ((ys[k] + shift) / GRID).round() * GRID;
            // the grid may push the end stacks past the bed: pulled back inside it
            let y = y.clamp(-half + h, half - h);
            Station { y, ..*st }
        })
        .collect();
    Some((stations, reach))
}

/// The ram's travel through a step: approach to the sheet, the bend, the return.
fn travel(m: &Model, tl: &Tooling, done: &[bool], st: &crate::sim::Step) -> Travel {
    let ctx = StepCtx::new(m, tl, done, st.bend, st.setup, st.overbend);
    let pinch = ctx.ztip(0.0);
    let bottom = ctx.ztip(ctx.phimax);
    Travel {
        approach: (ctx.z_tdc - pinch).max(0.0),
        work: (pinch - bottom).max(0.0),
        ret: (ctx.z_tdc - bottom).max(0.0),
    }
}

/// A staged program: the best of the planner's ranked programs (up to `max`) whose stations fit
/// the bed and pass every check with all of them on the press.
pub fn stage(m: &Model, tl: &Tooling, opts: &Options, max: usize) -> Result<Staged, StageFail> {
    let bare = Tooling {
        stations: Vec::new(),
        ..tl.clone()
    };
    let progs = programs(m, &bare, opts, max.max(1));
    if progs.is_empty() {
        return Err(StageFail::NoProgram(crate::plan(m, &bare, opts)));
    }
    let mut layouts = 0;
    let mut attempt: Option<Plan> = None;
    for (pi, prog) in progs.iter().enumerate() {
        for tight in [false, true] {
            let Some((stations, reach)) = layout(m, &bare, prog, tight) else {
                continue;
            };
            layouts += 1;
            let tl2 = Tooling {
                stations: stations.clone(),
                ..bare.clone()
            };
            if tl2.validate().is_err() {
                continue;
            }
            let seq = prog.plan.sequence.clone();
            let setups: Vec<Option<Setup>> = prog.plan.steps.iter().map(|s| Some(s.setup)).collect();
            let o = Options {
                sequence: Some(seq.clone()),
                setups: Some(setups),
                ..opts.clone()
            };
            let mut plan = simulate(m, &tl2, &seq, &o);
            // a stated setup is simulated as stated: the search's reasons for it are the program's
            for (st, from) in plan.steps.iter_mut().zip(&prog.plan.steps) {
                st.rejected = from.rejected.clone();
            }
            if !plan.feasible {
                if attempt.is_none() {
                    attempt = Some(plan);
                }
                continue;
            }
            let Some((_, of)) = pairs(m, &bare, prog) else {
                continue;
            };
            let dones = done_before(m, &seq);
            let travel: Vec<Travel> = plan
                .steps
                .iter()
                .zip(&dones)
                .map(|(st, d)| travel(m, &tl2, d, st))
                .collect();
            let uses: Vec<StationUse> = stations
                .iter()
                .enumerate()
                .map(|(k, s)| {
                    let here: Vec<usize> = (0..of.len()).filter(|i| of[*i] == k).collect();
                    let p = &tl2.punches[s.punch];
                    let pid = format!("punch:{}/", p.id);
                    let punch_clear = here
                        .iter()
                        .flat_map(|i| plan.steps[*i].checks.iter())
                        .filter(|c| {
                            matches!(c.rule, Rule::ToolCollision | Rule::ClosesOnPunch)
                                && c.tool.as_deref().is_some_and(|t| t.starts_with(&pid))
                        })
                        .min_by(|a, b| a.value.total_cmp(&b.value))
                        .cloned();
                    // why this punch: the search's rejections for its first bend, by punch
                    let mut instead_of: Vec<(String, Rule)> = Vec::new();
                    if let Some(first) = here.first() {
                        for r in &prog.plan.steps[*first].rejected {
                            if r.0 != p.id && !instead_of.iter().any(|x| x.0 == r.0) {
                                instead_of.push((r.0.clone(), r.3));
                            }
                        }
                    }
                    StationUse {
                        punch: p.id.clone(),
                        die: tl2.dies[s.die].id.clone(),
                        kind: p.kind,
                        y: s.y,
                        punch_length: s.punch_length,
                        die_length: s.die_length,
                        steps: here,
                        reach: reach[k],
                        punch_clear,
                        instead_of,
                    }
                })
                .collect();
            let shifts = of.windows(2).filter(|w| w[0] != w[1]).count();
            return Ok(Staged {
                tooling: tl2,
                plan,
                station_of: of,
                stations: uses,
                flips: prog.flips,
                turns: prog.turns,
                shifts,
                travel,
                programs_tried: pi + 1,
                layouts_tried: layouts,
                tight,
            });
        }
    }
    Err(StageFail::NoLayout {
        why: if layouts == 0 {
            "no program's stations fit the bed together".into()
        } else {
            "every layout that fits the bed fails a check with its neighbours on the press".into()
        },
        attempt: attempt.map(Box::new),
    })
}

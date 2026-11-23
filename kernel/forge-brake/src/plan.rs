//! Bend order and tooling: a deterministic depth-first search over the set of bends done.
//!
//! A step's checks depend only on the bends already done, the bend and its setup, so each result
//! is kept and every state found dead is not entered again. Bends are tried outermost first (the
//! deepest in the wall tree), then in the part's order; for each, both orientations, the punches
//! (straight, gooseneck, acute, then by id) and the dies (V nearest the set's rule, then by id).
//! The closed-form rules are tried before any distance is: a setup that fails one costs nothing.
use crate::model::Model;
use crate::sim::{Check, Rule, Setup, Severity, Step, StepCtx};
use crate::tooling::{PunchKind, Tooling};
use forge_sheet::{JointId, Strip, WallId};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Options {
    /// Bends in this order (indices into `Model::bends`), not searched.
    pub sequence: Option<Vec<usize>>,
    /// The part's material, for springback rows.
    pub material: Option<String>,
    /// Its tensile strength (MPa), for the force check.
    pub uts: Option<f64>,
    /// Only these punches / dies (indices into the tooling's lists).
    pub punches: Option<Vec<usize>>,
    pub dies: Option<Vec<usize>>,
    /// The most setups the search may simulate (0: the default, 4000).
    pub budget: usize,
    /// Only this orientation (rotated end for end or not).
    pub rotate: Option<bool>,
}

/// What stops a bend: its best attempt's failing checks.
#[derive(Clone, Debug, PartialEq)]
pub struct Blocking {
    pub bend: usize,
    pub joint: JointId,
    /// How many bends were done when it was tried.
    pub after: Vec<usize>,
    pub punch: String,
    pub die: String,
    pub checks: Vec<Check>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Suggestion {
    /// A leg is short of the die's shoulder by `by`: lengthen the wall's flange.
    LengthenFlange { bend: usize, wall: WallId, by: f64 },
    /// No tool in the set will do: one with this property would.
    NeedTool { bend: usize, rule: Rule, what: String },
    /// The bend passes with this tooling in another place in the order.
    UseTool { bend: usize, punch: String, die: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub feasible: bool,
    pub sequence: Vec<usize>,
    pub steps: Vec<Step>,
    pub blocking: Vec<Blocking>,
    pub suggestions: Vec<Suggestion>,
    /// Setups simulated.
    pub explored: usize,
    /// The search stopped at its budget.
    pub exhausted: bool,
}

struct Search<'a> {
    m: &'a Model,
    tl: &'a Tooling,
    opts: &'a Options,
    setups: Vec<Setup>,
    memo: HashMap<(u64, usize), Option<Step>>,
    selfs: HashMap<(u64, usize), Check>,
    dead: HashSet<u64>,
    /// Per bend: (bends done, failures, the attempt).
    best: Vec<Option<(usize, usize, Step, u64)>>,
    explored: usize,
    budget: usize,
    exhausted: bool,
}

fn mask(done: &[bool]) -> u64 {
    done.iter()
        .enumerate()
        .fold(0u64, |m, (k, d)| if *d { m | (1 << k) } else { m })
}

impl<'a> Search<'a> {
    fn new(m: &'a Model, tl: &'a Tooling, opts: &'a Options) -> Search<'a> {
        let t = m.t;
        let mut punches: Vec<usize> = opts
            .punches
            .clone()
            .unwrap_or_else(|| (0..tl.punches.len()).collect());
        let rank = |k: PunchKind| match k {
            PunchKind::Straight => 0,
            PunchKind::Gooseneck => 1,
            PunchKind::Acute => 2,
        };
        punches.sort_by(|a, b| {
            let (p, q) = (&tl.punches[*a], &tl.punches[*b]);
            rank(p.kind).cmp(&rank(q.kind)).then(p.id.cmp(&q.id))
        });
        let mut dies: Vec<usize> = opts.dies.clone().unwrap_or_else(|| (0..tl.dies.len()).collect());
        let want = tl.v_rule * t;
        dies.sort_by(|a, b| {
            let (p, q) = (&tl.dies[*a], &tl.dies[*b]);
            (p.v - want)
                .abs()
                .total_cmp(&(q.v - want).abs())
                .then(p.id.cmp(&q.id))
        });
        let mut setups = Vec::new();
        for rotate in [false, true] {
            if opts.rotate.is_some_and(|r| r != rotate) {
                continue;
            }
            for &punch in &punches {
                for &die in &dies {
                    setups.push(Setup { punch, die, rotate });
                }
            }
        }
        Search {
            m,
            tl,
            opts,
            setups,
            memo: HashMap::new(),
            selfs: HashMap::new(),
            dead: HashSet::new(),
            best: vec![None; m.bends.len()],
            explored: 0,
            budget: if opts.budget == 0 { 4000 } else { opts.budget },
            exhausted: false,
        }
    }

    fn overbend(&self, j: usize) -> f64 {
        self.tl
            .overbend(self.opts.material.as_deref(), self.m.bends[j].theta)
    }

    /// One setup simulated: quick stops at the first failure.
    fn run(&mut self, done: &[bool], j: usize, setup: Setup, quick: bool) -> Step {
        self.explored += 1;
        let ob = self.overbend(j);
        let ctx = StepCtx::new(self.m, self.tl, done, j, setup, ob);
        let (mut checks, travel, force) = ctx.rules(self.opts.uts);
        let (pl, dl) = ctx.tool_lengths();
        let mut step = Step {
            bend: j,
            joint: self.m.bends[j].joint,
            angle: self.m.bends[j].theta,
            overbend: ob,
            setup,
            punch: ctx.punch.id.clone(),
            die: ctx.die.id.clone(),
            punch_length: pl,
            die_length: dl,
            flip: self.m.bends[j].sigma < 0.0,
            gauge: None,
            travel,
            force,
            checks: Vec::new(),
        };
        let failed = |c: &[Check]| c.iter().any(|c| !c.pass && c.severity == Severity::Error);
        if quick && failed(&checks) {
            step.checks = checks;
            return step;
        }
        let (gc, gauge) = ctx.gauge();
        checks.push(gc);
        step.gauge = gauge.clone();
        if quick && failed(&checks) {
            step.checks = checks;
            return step;
        }
        let key = (mask(done), j);
        let sc = match self.selfs.get(&key) {
            Some(c) => c.clone(),
            None => {
                let c = ctx.self_check(quick);
                self.selfs.insert(key, c.clone());
                c
            }
        };
        checks.push(sc);
        if quick && failed(&checks) {
            step.checks = checks;
            return step;
        }
        use crate::sim::Phase;
        for phase in [Phase::Static, Phase::Approach, Phase::Bend, Phase::Return] {
            let g = if phase == Phase::Static {
                None
            } else {
                gauge.as_ref()
            };
            ctx.collisions(phase, g, &mut checks, quick);
            if quick && failed(&checks) {
                break;
            }
        }
        step.checks = checks;
        step
    }

    /// The first setup that passes bend j after `done`, kept.
    fn try_bend(&mut self, done: &[bool], j: usize) -> Option<Step> {
        let key = (mask(done), j);
        if let Some(r) = self.memo.get(&key) {
            return r.clone();
        }
        let n_done = done.iter().filter(|d| **d).count();
        let mut found = None;
        for s in self.setups.clone() {
            if self.explored >= self.budget {
                self.exhausted = true;
                break;
            }
            let step = self.run(done, j, s, true);
            if std::env::var_os("BENDSIM_TRACE").is_some() {
                eprintln!(
                    "setup {j} {} {} rot {} -> {:?}",
                    step.punch,
                    step.die,
                    s.rotate,
                    step.failures()
                        .iter()
                        .map(|c| (c.rule, c.value, c.tool.clone(), c.phase))
                        .collect::<Vec<_>>()
                );
            }
            if step.pass() {
                found = Some(self.run(done, j, s, false));
                break;
            }
            let nf = step.failures().len();
            let better = match &self.best[j] {
                None => true,
                Some((d, f, _, _)) => n_done > *d || (n_done == *d && nf < *f),
            };
            if better {
                self.best[j] = Some((n_done, nf, step, mask(done)));
            }
        }
        self.memo.insert(key, found.clone());
        found
    }

    fn order(&self) -> Vec<usize> {
        let depth = |k: usize| {
            let mut d = 0;
            let mut w = self.m.bends[k].a;
            while let Some(p) = self.m.parent[w] {
                d += 1;
                w = self.m.bends[p].a;
            }
            d
        };
        let mut o: Vec<usize> = (0..self.m.bends.len()).collect();
        o.sort_by(|a, b| depth(*b).cmp(&depth(*a)).then(a.cmp(b)));
        o
    }

    fn dfs(&mut self, done: &mut Vec<bool>, steps: &mut Vec<Step>) -> bool {
        if done.iter().all(|d| *d) {
            return true;
        }
        let key = mask(done);
        if self.dead.contains(&key) || self.exhausted {
            return false;
        }
        for j in self.order() {
            if done[j] {
                continue;
            }
            if let Some(step) = self.try_bend(done, j) {
                done[j] = true;
                steps.push(step);
                if self.dfs(done, steps) {
                    return true;
                }
                steps.pop();
                done[j] = false;
            }
            if self.exhausted {
                return false;
            }
        }
        self.dead.insert(key);
        false
    }
}

/// A bend order and tooling that pass every check, or why none does.
pub fn plan(m: &Model, tl: &Tooling, opts: &Options) -> Plan {
    if let Some(seq) = &opts.sequence {
        return simulate(m, tl, seq, opts);
    }
    let mut s = Search::new(m, tl, opts);
    let mut done = vec![false; m.bends.len()];
    let mut steps = Vec::new();
    let feasible = s.dfs(&mut done, &mut steps);
    let sequence = steps.iter().map(|st| st.bend).collect();
    let blocking = if feasible { Vec::new() } else { blocking(&s) };
    let suggestions = suggest(m, &blocking);
    Plan {
        feasible,
        sequence,
        steps,
        blocking,
        suggestions,
        explored: s.explored,
        exhausted: s.exhausted,
    }
}

fn blocking(s: &Search) -> Vec<Blocking> {
    s.best
        .iter()
        .enumerate()
        .filter_map(|(k, b)| {
            let (_, _, step, m) = b.as_ref()?;
            Some(Blocking {
                bend: k,
                joint: s.m.bends[k].joint,
                after: (0..s.m.bends.len()).filter(|i| m & (1 << i) != 0).collect(),
                punch: step.punch.clone(),
                die: step.die.clone(),
                checks: step.failures().into_iter().cloned().collect(),
            })
        })
        .collect()
}

fn suggest(m: &Model, blocking: &[Blocking]) -> Vec<Suggestion> {
    let mut out = Vec::new();
    for b in blocking {
        for c in &b.checks {
            match c.rule {
                Rule::MinFlange => {
                    if let Some(crate::sim::PartRef {
                        strip: Strip::Wall(w),
                        ..
                    }) = c.part
                    {
                        let by = ((c.limit - c.value) * 10.0).ceil() / 10.0 + 0.1;
                        out.push(Suggestion::LengthenFlange {
                            bend: b.bend,
                            wall: w,
                            by,
                        });
                    }
                }
                Rule::VTooNarrow => out.push(Suggestion::NeedTool {
                    bend: b.bend,
                    rule: c.rule,
                    what: format!("a die with a wider V than {}", b.die),
                }),
                Rule::ClosesOnPunch => out.push(Suggestion::NeedTool {
                    bend: b.bend,
                    rule: c.rule,
                    what: "a gooseneck punch with a deeper throat, or a shorter flange".into(),
                }),
                Rule::DieAngle | Rule::PunchAngle => out.push(Suggestion::NeedTool {
                    bend: b.bend,
                    rule: c.rule,
                    what: "acute tooling".into(),
                }),
                Rule::PunchRadius => out.push(Suggestion::NeedTool {
                    bend: b.bend,
                    rule: c.rule,
                    what: format!("a punch with a tip radius at most {:.2}", c.limit),
                }),
                _ => {}
            }
        }
    }
    let _ = m;
    out.dedup();
    out
}

/// The bends in the order given, each with the first setup that passes it, or its best attempt.
pub fn simulate(m: &Model, tl: &Tooling, seq: &[usize], opts: &Options) -> Plan {
    let mut s = Search::new(m, tl, opts);
    let mut done = vec![false; m.bends.len()];
    let mut steps = Vec::new();
    let mut feasible = true;
    let mut blocking = Vec::new();
    for &j in seq {
        match s.try_bend(&done, j) {
            Some(st) => steps.push(st),
            None => {
                feasible = false;
                // its best attempt, simulated in full, stands in the plan
                let attempt = s.best[j].clone();
                if let Some((_, _, st, _)) = attempt {
                    let full = s.run(&done, j, st.setup, false);
                    blocking.push(Blocking {
                        bend: j,
                        joint: m.bends[j].joint,
                        after: (0..m.bends.len()).filter(|i| done[*i]).collect(),
                        punch: full.punch.clone(),
                        die: full.die.clone(),
                        checks: full.failures().into_iter().cloned().collect(),
                    });
                    steps.push(full);
                }
            }
        }
        done[j] = true;
    }
    let suggestions = suggest(m, &blocking);
    Plan {
        feasible,
        sequence: seq.to_vec(),
        steps,
        blocking,
        suggestions,
        explored: s.explored,
        exhausted: s.exhausted,
    }
}

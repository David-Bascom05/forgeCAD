//! `query F bendsim`: the part's sheet bent on a press brake (kernel/forge-brake), answered as
//! typed JSON, schema `forgecad.bendsim/1` (docs/bendsim.md).
//!
//! Words: `bendsim [press P [punches A,B] [dies C,D]] [tooling T] [vendor V] [sequence S]
//! [sheet NAME] [uts MPA]`.
//! - P: a press brake from the library (kernel/forge-brake data, then the part's folder's
//!   `bend/tooling/*.toml`, then the user's): real makers' records, every value cited; the
//!   tools are the library's for its clamping system, or those named (by id).
//! - T: a stand-in set (`std-eu`, `std-us`: forgecad's own numbers, not a maker's) or a
//!   tooling file (JSON, beside the part).
//! - V: a vendor file (JSON) whose `tooling` field is a set id, a tooling object, or
//!   `{"base": ID, ..}` (the set with tools added or replaced by id, rules overridden).
//! - S: bends in order, comma separated, by name (the flange's feature) or by number (1 = the
//!   sheet's first bend); left out, the order is searched.
//!
//! Tooling files hold lengths in mm and angles in degrees.
use crate::query::Answer;
use crate::sheet::SheetOut;
use crate::{Options, run};
use forge_agent::json::Json;
use forge_brake as fb;
use forge_brake::sim::{Check, PartRef, Rule, Severity, Surface};
use forge_math::{Point2, dmath};
use forge_sheet::{Strip, WallId};
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "forgecad.bendsim/1";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ask {
    pub press: Option<String>,
    pub punches: Option<Vec<String>>,
    pub dies: Option<Vec<String>>,
    pub tooling: Option<String>,
    pub vendor: Option<String>,
    pub sequence: Option<String>,
    pub sheet: Option<String>,
    pub uts: Option<f64>,
    /// Keep the last step's tools where they pass (a program applied from `programs`).
    pub keep_tools: bool,
    /// Also answer up to this many ranked feasible programs (`programs`).
    pub programs: Option<usize>,
    /// Stations along the bed as a program states them (a `bending` document's `station`s).
    pub stations: Vec<StationAsk>,
    /// Per step of `sequence`, what the program states of its setup.
    pub steps: Vec<StepAsk>,
    /// Lay the program's tools out as stations along the bed (`stations auto`): stage bending.
    pub stage: bool,
    /// Bends locked to a station (`lock rear=2,tab=1`: a bend's name or number, a station's
    /// number from 1, into `stations`): each made there; the order and the rest are searched.
    pub locks: Vec<(String, usize)>,
    /// Answer only where the part and the tools collide (`collisions`), and of one step
    /// (`step N`, from 1).
    pub collisions: bool,
    pub step: Option<usize>,
}

/// A station: its punch and die (library ids), its centre along the bed from the bed's centre,
/// and its stacks' lengths.
#[derive(Clone, Debug, PartialEq)]
pub struct StationAsk {
    pub punch: String,
    pub die: String,
    pub y: f64,
    pub punch_length: f64,
    pub die_length: f64,
}

/// A step's stated setup: its station (an index into `Ask::stations`) and orientation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepAsk {
    pub station: Option<usize>,
    pub rotate: Option<bool>,
}

/// The words after `bendsim`.
pub fn parse(words: &[String]) -> Result<Ask, String> {
    let mut a = Ask::default();
    let mut it = words.iter();
    while let Some(w) = it.next() {
        let mut val = |what: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("`{what}` takes a value"))
        };
        match w.trim_start_matches("--") {
            "tooling" => a.tooling = Some(val("tooling")?),
            "press" => a.press = Some(val("press")?),
            "punches" => a.punches = Some(val("punches")?.split(',').map(|s| s.trim().to_string()).collect()),
            "dies" => a.dies = Some(val("dies")?.split(',').map(|s| s.trim().to_string()).collect()),
            "vendor" => a.vendor = Some(val("vendor")?),
            "sequence" => a.sequence = Some(val("sequence")?),
            "keep_tools" => a.keep_tools = true,
            "lock" => {
                for item in val("lock")?.split(',').map(str::trim).filter(|x| !x.is_empty()) {
                    let (b, k) = item
                        .split_once('=')
                        .ok_or_else(|| format!("`lock` takes bend=station pairs, not `{item}`"))?;
                    let k = k
                        .trim()
                        .parse::<usize>()
                        .ok()
                        .filter(|k| *k >= 1)
                        .ok_or_else(|| format!("`lock`: `{k}` is not a station number (1, 2, ..)"))?;
                    a.locks.push((b.trim().to_string(), k - 1));
                }
            }
            "collisions" => a.collisions = true,
            "step" => {
                a.step = Some(
                    val("step")?
                        .parse::<usize>()
                        .ok()
                        .filter(|n| *n >= 1)
                        .ok_or("step takes a step's number, from 1")?,
                )
            }
            "stations" => match val("stations")?.as_str() {
                "auto" => a.stage = true,
                other => {
                    return Err(format!(
                        "`stations` takes `auto` (lay the tools out along the bed), not `{other}`"
                    ));
                }
            },
            "programs" => {
                a.programs = Some(
                    val("programs")?
                        .parse::<usize>()
                        .ok()
                        .filter(|n| (1..=50).contains(n))
                        .ok_or("programs takes a count, 1 to 50")?,
                )
            }
            "sheet" => a.sheet = Some(val("sheet")?),
            "uts" => {
                let v = val("uts")?;
                a.uts = Some(
                    v.parse::<f64>()
                        .ok()
                        .filter(|x| x.is_finite() && *x > 0.0)
                        .ok_or("uts takes a strength in MPa")?,
                );
            }
            other => {
                return Err(format!(
                    "bendsim takes press P, punches A,B, dies C,D, tooling T, vendor V, sequence S, stations auto, lock B=N, collisions, step N, sheet NAME, uts MPA; not `{other}`"
                ));
            }
        }
    }
    Ok(a)
}

/// A typed error: its kind, its words, and what would do instead.
fn err(kind: &str, text: impl Into<String>, extra: Vec<(&str, Json)>) -> Answer {
    let text = text.into();
    let mut e = vec![("kind", Json::from(kind)), ("text", Json::from(text.as_str()))];
    e.extend(extra);
    Answer {
        ok: false,
        text: format!("error {kind}: {text}"),
        json: Json::obj(vec![
            ("ok", false.into()),
            ("schema", SCHEMA.into()),
            ("error", Json::obj(e)),
        ]),
    }
}

fn strs(v: &[&str]) -> Json {
    Json::Arr(v.iter().map(|s| Json::from(*s)).collect())
}

/// Runs the part at `path` and answers.
pub fn ask_file(path: &Path, root: Option<PathBuf>, ask: &Ask) -> Answer {
    match std::fs::read_to_string(path) {
        Ok(src) => ask_src(&src, path.parent().map(Path::to_path_buf), root, ask),
        Err(e) => err("File", format!("{}: {e}", path.display()), vec![]),
    }
}

pub fn ask_src(src: &str, dir: Option<PathBuf>, root: Option<PathBuf>, ask: &Ask) -> Answer {
    match simulate(src, dir, root, ask) {
        Ok(s) if ask.collisions => collisions(&s.answer, ask.step),
        Ok(s) => s.answer,
        Err(a) => a,
    }
}

/// `bendsim collisions [step N]`: where the part and the tools collide, step by step (or one
/// step's), schema `forgecad.bendsim.collisions/1`: every contact's faces on both bodies, its
/// region (points, box), depth, when, and a line of words.
pub fn collisions(full: &Answer, step: Option<usize>) -> Answer {
    let j = &full.json;
    let all = match j.get("contacts") {
        Some(Json::Arr(v)) => v.clone(),
        _ => Vec::new(),
    };
    let picked: Vec<Json> = all
        .into_iter()
        .filter(|c| step.is_none_or(|n| c.get("step").and_then(Json::num) == Some(n as f64)))
        .collect();
    let mut text = String::new();
    for c in &picked {
        if let Some(t) = c.get("text").and_then(Json::str) {
            text.push_str(t);
            text.push('\n');
        }
    }
    if picked.is_empty() {
        text.push_str("no collisions: every step's part is proven clear of the tools\n");
    }
    let keep = |k: &str| j.get(k).cloned().unwrap_or(Json::Null);
    Answer {
        ok: true,
        text,
        json: Json::obj(vec![
            ("ok", true.into()),
            ("schema", "forgecad.bendsim.collisions/1".into()),
            ("sheet", keep("sheet")),
            ("tooling", keep("tooling")),
            ("feasible", keep("feasible")),
            ("sequence", keep("sequence")),
            ("step", step.map_or(Json::Null, |n| n.into())),
            ("contacts", Json::Arr(picked)),
        ]),
    }
}

/// A simulation as the app shows it: the model, the tooling, the plan, the bends' names, and the
/// reply the query gives.
pub struct Sim {
    pub model: fb::Model,
    pub tooling: fb::Tooling,
    pub plan: fb::Plan,
    pub names: Vec<String>,
    pub answer: Answer,
    /// Ranked feasible programs, when asked for (`Ask::programs`).
    pub programs: Vec<fb::Program>,
    /// The staged program (`stations auto`): its stations and why; `tooling` and `plan` are its.
    pub stage: Option<fb::Staged>,
}

/// A made plan, kept by its whole question: (tooling as laid out, plan, ranked programs, staged
/// program, the reply's stage field).
type Made = (
    fb::Tooling,
    fb::Plan,
    Option<Vec<fb::Program>>,
    Option<fb::Staged>,
    Option<Json>,
);

/// The last plans made in this process, each by its whole question (the model, tooling, options
/// and what was asked, written out exactly): a reopened or re-simulated unchanged part is answered
/// at once.
static PLANS: std::sync::Mutex<Vec<(String, Made)>> = std::sync::Mutex::new(Vec::new());
const PLANS_KEPT: usize = 8;

/// Forget every kept plan (tests: a plan made afresh).
#[doc(hidden)]
pub fn forget_plans() {
    if let Ok(mut v) = PLANS.lock() {
        v.clear();
    }
}

fn memo_get(key: &str) -> Option<Made> {
    let v = PLANS.lock().ok()?;
    v.iter().find(|(k, _)| k == key).map(|(_, m)| m.clone())
}

fn memo_put(key: String, made: Made) {
    if let Ok(mut v) = PLANS.lock() {
        v.retain(|(k, _)| *k != key);
        v.push((key, made));
        while v.len() > PLANS_KEPT {
            v.remove(0);
        }
    }
}

/// `BENDSIM_TIME=1`: each phase's wall time on stderr (native only: no clock on wasm32).
struct Phases {
    #[cfg(not(target_arch = "wasm32"))]
    at: Option<std::time::Instant>,
}

impl Phases {
    fn start() -> Phases {
        Phases {
            #[cfg(not(target_arch = "wasm32"))]
            at: std::env::var_os("BENDSIM_TIME").map(|_| std::time::Instant::now()),
        }
    }

    fn lap(&mut self, _what: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(t) = &mut self.at {
            eprintln!(
                "bendsim time: {_what:<10} {:>8.1} ms",
                t.elapsed().as_secs_f64() * 1e3
            );
            *t = std::time::Instant::now();
        }
    }
}

/// The part's sheet simulated: Err is the typed error reply.
pub fn simulate(src: &str, dir: Option<PathBuf>, root: Option<PathBuf>, ask: &Ask) -> Result<Sim, Answer> {
    let sh = built(src, dir.clone(), root, ask)?;
    simulate_sheet(&sh, dir.as_deref(), ask)
}

/// The part built and the sheet asked for (its first, unless `ask.sheet` names one).
fn built(src: &str, dir: Option<PathBuf>, root: Option<PathBuf>, ask: &Ask) -> Result<SheetOut, Answer> {
    let opts = Options {
        dir: dir.clone(),
        root,
        ..Options::default()
    };
    let mut ph = Phases::start();
    let (r, _) = run(src, opts, Default::default());
    ph.lap("build");
    if r.sheets.is_empty() {
        return Err(match r.errors.first() {
            Some(e) => {
                // the build's own typed error, with its span and fixes
                let j = e.json(src);
                Answer {
                    ok: false,
                    text: format!("error NotBuilt: {}: {}", e.kind(), e.text()),
                    json: Json::obj(vec![
                        ("ok", false.into()),
                        ("schema", SCHEMA.into()),
                        (
                            "error",
                            Json::obj(vec![
                                ("kind", "NotBuilt".into()),
                                ("text", "the part did not build".into()),
                                ("cause", j),
                            ]),
                        ),
                    ]),
                }
            }
            None => err("NoSheet", "the part has no sheet metal", vec![]),
        });
    }
    let names: Vec<&str> = r.sheets.iter().map(|s| s.id.as_str()).collect();
    let sh = match &ask.sheet {
        Some(n) => match r.sheets.iter().find(|s| &s.id == n) {
            Some(s) => s,
            None => {
                return Err(err(
                    "NoSuchSheet",
                    format!("no sheet `{n}`"),
                    vec![("sheets", strs(&names))],
                ));
            }
        },
        None => &r.sheets[0],
    };
    Ok(sh.clone())
}

/// One built sheet simulated (for a caller that has run the part already, as `query F dfm`
/// does): its vendor's tooling unless `ask` names other.
pub fn simulate_sheet(sh: &SheetOut, dir: Option<&Path>, ask: &Ask) -> Result<Sim, Answer> {
    let p = prepare(sh, dir, ask)?;
    let made = match memo_get(&p.key) {
        Some(hit) => hit,
        None => {
            let m = make(sh, dir, &p);
            memo_put(p.key.clone(), m.clone());
            m
        }
    };
    Ok(assemble(sh, p, made))
}

/// A simulation made a little at a time, for an app that must not wait on it: `begin` builds the
/// part and the tooling (milliseconds), so the press, its tools and the flat part can be drawn at
/// once (`model`, `tooling`); `step(n)` plans until about `n` more setups are simulated and says how
/// far it has got; `finish` is the very `Sim` that `simulate` gives. A kept plan (an unchanged
/// part) is done at `begin`. A staged program or ranked programs are made whole by the first step.
/// It holds what it reads (no references) but is not `Send` (the search's verdict memo is shared
/// within it): a worker makes it on its own thread and sends back `Progress` and the `Sim`.
pub struct Planning {
    sh: SheetOut,
    dir: Option<PathBuf>,
    prep: Prep,
    job: Option<fb::PlanJob>,
    /// The plan found, its handling checked one step at a time: (plan, next step, checks).
    handling: Option<Handling>,
    made: Option<Made>,
}

/// A found plan whose handling is being checked: the plan, the next step, the checks so far.
type Handling = (fb::Plan, usize, Vec<(usize, fb::Check)>);

/// `simulate`, begun: the part built, the tooling and the model made, nothing planned yet.
pub fn begin(src: &str, dir: Option<PathBuf>, root: Option<PathBuf>, ask: &Ask) -> Result<Planning, Answer> {
    let sh = built(src, dir.clone(), root, ask)?;
    let prep = prepare(&sh, dir.as_deref(), ask)?;
    let made = memo_get(&prep.key);
    Ok(Planning {
        sh,
        dir,
        prep,
        job: None,
        handling: None,
        made,
    })
}

impl Planning {
    pub fn model(&self) -> &fb::Model {
        &self.prep.model
    }

    /// The tooling as asked (a staged program's stations come with the plan).
    pub fn tooling(&self) -> &fb::Tooling {
        &self.prep.tooling
    }

    pub fn names(&self) -> &[String] {
        &self.prep.bend_names
    }

    pub fn progress(&self) -> fb::Progress {
        let bends = self.prep.model.bends.len();
        match (&self.made, &self.job) {
            (Some(m), _) => fb::Progress {
                bends_planned: m.1.steps.len(),
                bends,
                setups_tried: m.1.explored,
                done: true,
            },
            (None, Some(j)) => j.progress(&self.prep.model),
            (None, None) if self.handling.is_some() => {
                let h = self.handling.as_ref().unwrap();
                fb::Progress {
                    bends_planned: h.0.steps.len(),
                    bends,
                    setups_tried: h.0.explored,
                    done: false,
                }
            }
            (None, None) => fb::Progress {
                bends_planned: 0,
                bends,
                setups_tried: 0,
                done: false,
            },
        }
    }

    /// Plans on until about `n` more setups are simulated (a frame's slice, a worker's loop).
    pub fn step(&mut self, n: usize) -> fb::Progress {
        if self.made.is_some() {
            return self.progress();
        }
        let p = &self.prep;
        // a plain plan goes a slice at a time; a staged program or ranked programs whole
        if p.ask.stage || p.ask.programs.is_some() {
            let m = make(&self.sh, self.dir.as_deref(), p);
            memo_put(p.key.clone(), m.clone());
            self.made = Some(m);
            return self.progress();
        }
        // the operator's move into each step after the first, checked one step a call (as
        // fb::scene::annotate_handling does all at once)
        if let Some((plan, k, checks)) = &mut self.handling {
            if *k < plan.steps.len() {
                let mut sc = fb::scene::Scene::new(&p.model, &p.tooling, plan);
                if let Some(c) = sc.handling_check(*k) {
                    checks.push((*k, c));
                }
                *k += 1;
            }
            if *k >= plan.steps.len() {
                let (mut plan, _, checks) = self.handling.take().unwrap();
                for (k, c) in checks {
                    plan.steps[k].checks.push(c);
                }
                let m = (p.tooling.clone(), plan, None, None, None);
                memo_put(p.key.clone(), m.clone());
                self.made = Some(m);
            }
            return self.progress();
        }
        let job = self
            .job
            .get_or_insert_with(|| fb::PlanJob::new(&p.model, &p.tooling, &p.opts));
        let pr = job.step(&p.model, &p.tooling, &p.opts, n);
        if pr.done
            && let Some(job) = self.job.take()
            && let Some(plan) = job.plan(&p.model, &p.tooling, &p.opts)
        {
            self.handling = Some((plan, 1, Vec::new()));
        }
        self.progress()
    }

    /// The simulation (planned to the end first, if it is not yet).
    pub fn finish(mut self) -> Sim {
        while !self.step(usize::MAX / 2).done {}
        let made = self.made.take().expect("planned");
        assemble(&self.sh, self.prep, made)
    }
}

/// A simulation's question, answered up to the plan: the tooling (stations set), the model, the
/// bends' names, the options, and the memo's key.
struct Prep {
    ask: Ask,
    tooling: fb::Tooling,
    tsource: Json,
    model: fb::Model,
    bend_names: Vec<String>,
    opts: fb::Options,
    key: String,
}

fn prepare(sh: &SheetOut, dir: Option<&Path>, ask: &Ask) -> Result<Prep, Answer> {
    // a program's stations: their tools are the ones on the press
    let mut ask = ask.clone();
    if !ask.stations.is_empty() {
        let mut ps: Vec<String> = ask.stations.iter().map(|s| s.punch.clone()).collect();
        let mut ds: Vec<String> = ask.stations.iter().map(|s| s.die.clone()).collect();
        ps.dedup();
        ds.dedup();
        ask.punches.get_or_insert(ps);
        ask.dies.get_or_insert(ds);
    }
    let mut ph = Phases::start();
    let (mut tooling, tsource) = tooling(&ask, dir, sh.vendor.as_ref().map(|v| v.id.as_str()))?;
    ph.lap("tooling");
    for st in &ask.stations {
        let (Some(p), Some(d)) = (
            tooling.punches.iter().position(|x| x.id == st.punch),
            tooling.dies.iter().position(|x| x.id == st.die),
        ) else {
            return Err(err(
                "BadStation",
                format!(
                    "a station's punch {} or die {} is not on the press",
                    st.punch, st.die
                ),
                vec![],
            ));
        };
        tooling.stations.push(fb::tooling::Station {
            punch: p,
            die: d,
            y: st.y,
            punch_length: st.punch_length,
            die_length: st.die_length,
        });
    }
    if let Err(e) = tooling.validate() {
        return Err(err("BadStation", format!("{e:?}"), vec![]));
    }
    let model = match fb::Model::new(&sh.sheet) {
        Ok(m) => m,
        Err(e) => return Err(model_err(sh, &e)),
    };
    let bend_names: Vec<String> = model.bends.iter().map(|b| joint_name(sh, b.joint)).collect();
    ph.lap("model");
    let sequence = match &ask.sequence {
        None => None,
        Some(s) => Some(sequence(s, &bend_names)?),
    };
    let uts = ask.uts.or_else(|| {
        sh.material
            .as_deref()
            .and_then(forge_material::find)
            .and_then(|m| m.uts)
    });
    let mut locks = vec![None; model.bends.len()];
    for (b, k) in &ask.locks {
        let j = sequence_one(b, &bend_names)?;
        if *k >= tooling.stations.len() {
            return Err(err(
                "BadStation",
                format!(
                    "`lock {b}={}`: the press has {} stations",
                    k + 1,
                    tooling.stations.len()
                ),
                vec![],
            ));
        }
        locks[j] = Some(*k);
    }
    let opts = fb::Options {
        locks,
        sequence,
        material: sh.material.clone(),
        uts,
        keep_tools: ask.keep_tools,
        setups: (!ask.steps.is_empty()).then(|| {
            ask.steps
                .iter()
                .map(|st| {
                    st.station
                        .and_then(|k| tooling.stations.get(k))
                        .map(|s| fb::Setup {
                            punch: s.punch,
                            die: s.die,
                            rotate: st.rotate.unwrap_or(false),
                        })
                })
                .collect()
        }),
        ..fb::Options::default()
    };
    // a plan already made for this very model, tooling and options (a reopening, Simulate Again
    // on an unchanged part): the planner is deterministic, so it is the same plan
    let key = format!(
        "{model:?}|{tooling:?}|{opts:?}|{:?}|{}|{bend_names:?}|{:?}",
        ask.programs, ask.stage, ask.press
    );
    Ok(Prep {
        ask,
        tooling,
        tsource,
        model,
        bend_names,
        opts,
        key,
    })
}

/// The plan made (not kept): staged, or with its ranked programs, or alone; the handling checked.
fn make(sh: &SheetOut, dir: Option<&Path>, p: &Prep) -> Made {
    let (ask, model, opts, bend_names) = (&p.ask, &p.model, &p.opts, &p.bend_names);
    let mut ph = Phases::start();
    // stage bending: the program's tools laid out along the bed, simulated all on the press
    let mut tooling = p.tooling.clone();
    let mut staged: Option<fb::Staged> = None;
    let mut stage_json: Option<Json> = None;
    let mut shared: Option<Vec<fb::Program>> = None;
    let mut plan = if ask.stage && tooling.stations.is_empty() && opts.sequence.is_none() {
        match fb::stage(model, &tooling, opts, 8) {
            Ok(s) => {
                let speeds = ask
                    .press
                    .as_deref()
                    .and_then(|p| catalog(dir).ok().and_then(|c| c.press(p).map(|r| r.speeds)));
                stage_json = Some(staged_json(sh, &s, bend_names, speeds.as_ref()));
                tooling = s.tooling.clone();
                let p = s.plan.clone();
                staged = Some(s);
                p
            }
            Err(fb::StageFail::NoProgram(p)) => {
                stage_json = Some(Json::obj(vec![
                    ("ok", false.into()),
                    (
                        "why",
                        "no feasible program to lay out: `blocking` says why".into(),
                    ),
                ]));
                p
            }
            Err(fb::StageFail::NoLayout { why, .. }) => {
                stage_json = Some(Json::obj(vec![
                    ("ok", false.into()),
                    ("why", why.as_str().into()),
                ]));
                fb::plan(model, &tooling, opts)
            }
        }
    } else if let Some(n) = ask.programs {
        // the plan and the ranked programs from one search's learning
        let (p, r) = fb::plan_and_programs(model, &tooling, opts, n);
        shared = Some(r);
        p
    } else {
        fb::plan(model, &tooling, opts)
    };
    ph.lap(if ask.stage { "stage" } else { "plan" });
    let ranked = match shared {
        Some(r) => Some(r),
        None => ask.programs.map(|n| fb::programs(model, &tooling, opts, n)),
    };
    ph.lap("programs");
    // the operator's move into each step after the first, checked (a warning if not proven clear)
    fb::scene::annotate_handling(model, &tooling, &mut plan);
    ph.lap("handling");
    (tooling, plan, ranked, staged, stage_json)
}

/// The simulation from its question and its plan: the reply, the tooling as laid out.
fn assemble(sh: &SheetOut, p: Prep, made: Made) -> Sim {
    let (tooling, plan, ranked, staged, stage_json) = made;
    let Prep {
        tsource,
        model,
        bend_names,
        ..
    } = p;
    let mut ph = Phases::start();
    let mut answer = answer(sh, &model, &tooling, &plan, &bend_names);
    ph.lap("reply");
    if let Json::Obj(f) = &mut answer.json {
        f.insert(4, ("tooling_source".into(), tsource));
        if let Some(r) = &ranked {
            f.push(("programs".into(), programs_json(&tooling, r, &bend_names)));
        }
        if let Some(j) = stage_json {
            f.push(("stage".into(), j));
        }
        f.push((
            "contacts".into(),
            Json::Arr(contacts_json(sh, &model, &tooling, &plan, &bend_names)),
        ));
    }
    Sim {
        model,
        tooling,
        plan,
        names: bend_names,
        answer,
        programs: ranked.unwrap_or_default(),
        stage: staged,
    }
}

/// The simulation's findings in `query F dfm`'s Item shape: one per failing check of each step
/// (severity "violation", or "warning" for withdraw), and one "refused" item per blocked bend;
/// `rule` is the simulator's rule word.
pub fn items(sh: &SheetOut, sim: &Sim) -> Vec<Json> {
    let mut out = Vec::new();
    let mut item = |c: &Check, step: Option<(usize, &fb::Step)>, bend: usize, severity: &str| {
        let a = angular(c.rule);
        let v = |x: f64| if a { rdeg(x) } else { r6(x) };
        let anchor = c.part.as_ref().map(|p| part(sh, p));
        let mut data = vec![("tool", c.tool.as_deref().map_or(Json::Null, Json::from))];
        if let Some((n, st)) = step {
            data.push(("step", ((n + 1) as f64).into()));
            data.push(("punch", st.punch.as_str().into()));
            data.push(("die", st.die.as_str().into()));
        }
        data.push(("phase", c.phase.map_or(Json::Null, |p| p.word().into())));
        data.push(("phi_deg", c.phi.map_or(Json::Null, rdeg)));
        // the place in the press frame (the folded part's is not named: location.at is null)
        data.push((
            "at_press",
            c.at.map_or(Json::Null, |a| Json::Arr(a.iter().map(|x| r6(*x)).collect())),
        ));
        let mut fixes = Vec::new();
        if c.rule == Rule::MinFlange
            && let Some(PartRef {
                strip: Strip::Wall(w),
                ..
            }) = c.part
        {
            let by = ((c.limit - c.value) * 10.0).ceil() / 10.0 + 0.1;
            if let Some(op) = sh.lengthen(w, by) {
                fixes.push(op.json());
            }
        }
        out.push(Json::obj(vec![
            ("rule", c.rule.word().into()),
            ("severity", severity.into()),
            (
                "text",
                format!(
                    "{}: {} {} {} {}",
                    sim.names[bend],
                    c.rule.word(),
                    r6s(if a { c.value.to_degrees() } else { c.value }),
                    if c.at_most { "over" } else { "under" },
                    r6s(if a { c.limit.to_degrees() } else { c.limit })
                )
                .as_str()
                .into(),
            ),
            ("feature", sim.names[bend].as_str().into()),
            ("line", Json::Null),
            ("measured", v(c.value)),
            ("limit", v(c.limit)),
            ("sense", (if c.at_most { "max" } else { "min" }).into()),
            ("unit", (if a { "deg" } else { "mm" }).into()),
            (
                "location",
                Json::obj(vec![
                    (
                        "anchor",
                        anchor
                            .as_ref()
                            .and_then(|j| j.get("anchor").cloned())
                            .unwrap_or(Json::Null),
                    ),
                    ("faces", Json::Arr(vec![])),
                    ("flat", Json::Null),
                    (
                        "at",
                        c.at.map_or(Json::Null, |a| Json::Arr(a.iter().map(|x| r6(*x)).collect())),
                    ),
                ]),
            ),
            ("data", Json::obj(data)),
            ("fixes", Json::Arr(fixes)),
        ]));
    };
    for (n, st) in sim.plan.steps.iter().enumerate() {
        for c in st.checks.iter().filter(|c| !c.pass) {
            let sev = if c.severity == Severity::Warning {
                "warning"
            } else {
                "violation"
            };
            item(c, Some((n, st)), st.bend, sev);
        }
    }
    for b in &sim.plan.blocking {
        for c in &b.checks {
            item(c, None, b.bend, "refused");
        }
    }
    out
}

fn model_err(sh: &SheetOut, e: &fb::ModelError) -> Answer {
    match e {
        fb::ModelError::Hem { joint } => err(
            "HemNotSimulated",
            format!(
                "`{}` is a hem: it needs a pre-bend and a flattening die, which the simulator does not model",
                joint_name(sh, *joint)
            ),
            vec![("bend", joint_name(sh, *joint).as_str().into())],
        ),
        fb::ModelError::NotBend { joint } => err(
            "NotBend",
            format!(
                "`{}` is not a bend (a rip or a tangent joint)",
                joint_name(sh, *joint)
            ),
            vec![],
        ),
        fb::ModelError::Spline { wall } => err(
            "SplineOutline",
            format!(
                "wall `{}` has a spline in its outline: its clearances are not in closed form",
                wall_name(sh, *wall)
            ),
            vec![],
        ),
        fb::ModelError::PatternSpline => err(
            "SplineOutline",
            "the flat pattern has a spline (a cut wrapped across a bend): its clearances are not in closed form",
            vec![],
        ),
        fb::ModelError::Build(e) => err(
            "SheetNotBuilt",
            format!("the sheet does not build: {e:?}"),
            vec![],
        ),
    }
}

fn joint_name(sh: &SheetOut, j: forge_sheet::JointId) -> String {
    sh.joints
        .iter()
        .find(|x| x.0 == j)
        .map_or_else(|| format!("bend{}", j.0), |x| x.1.clone())
}

fn wall_name(sh: &SheetOut, w: WallId) -> String {
    sh.walls
        .iter()
        .find(|x| x.0 == w)
        .map_or_else(|| format!("wall{}", w.0), |x| x.1.clone())
}

/// One bend by name or number (as in a sequence).
fn sequence_one(item: &str, names: &[String]) -> Result<usize, Answer> {
    Ok(sequence(item, names)?[0])
}

fn sequence(s: &str, names: &[String]) -> Result<Vec<usize>, Answer> {
    let mut out = Vec::new();
    for item in s.split(',').map(str::trim).filter(|x| !x.is_empty()) {
        let k = match item.parse::<usize>() {
            Ok(n) if n >= 1 && n <= names.len() => n - 1,
            Ok(n) => {
                return Err(err(
                    "BadSequence",
                    format!("bend {n} is out of range: the sheet has {} bends", names.len()),
                    vec![
                        ("item", item.into()),
                        (
                            "bends",
                            Json::Arr(names.iter().map(|x| Json::from(x.as_str())).collect()),
                        ),
                    ],
                ));
            }
            Err(_) => match names.iter().position(|x| x == item) {
                Some(k) => k,
                None => {
                    return Err(err(
                        "NoSuchBend",
                        format!("no bend `{item}`"),
                        vec![
                            ("item", item.into()),
                            (
                                "bends",
                                Json::Arr(names.iter().map(|x| Json::from(x.as_str())).collect()),
                            ),
                        ],
                    ));
                }
            },
        };
        if out.contains(&k) {
            return Err(err(
                "BadSequence",
                format!("`{item}` is in the sequence twice"),
                vec![("item", item.into())],
            ));
        }
        out.push(k);
    }
    if out.is_empty() {
        return Err(err("BadSequence", "the sequence is empty", vec![]));
    }
    Ok(out)
}

/// The tooling asked for: a file, a vendor's, or a standard set (std-eu by default).
/// A vendor profile's `[tooling]` (TOML, mm and degrees) as the JSON a tooling file holds.
fn toml_json(i: &forge_vendor::toml::Item) -> Json {
    use forge_vendor::toml::Item;
    match i {
        Item::Str(s) => Json::from(s.as_str()),
        Item::Num(n) => Json::from(*n),
        Item::Bool(b) => Json::Bool(*b),
        Item::Arr(v) => Json::Arr(v.iter().map(toml_json).collect()),
        Item::Table(t) => table_json(t),
        Item::Tables(ts) => Json::Arr(ts.iter().map(table_json).collect()),
    }
}

fn table_json(t: &forge_vendor::toml::Table) -> Json {
    Json::Obj(t.0.iter().map(|(k, v)| (k.clone(), toml_json(v))).collect())
}

/// The keys whose numbers are lengths (a vendor file's lengths are in its `units`); angles,
/// counts, loads and the V rule (in thicknesses) are not.
const LENGTHS: [&str; 22] = [
    "height",
    "tip_radius",
    "tip_height",
    "width",
    "depth",
    "z0",
    "z1",
    "v",
    "shoulder_radius",
    "length",
    "open_height",
    "stroke",
    "bed_length",
    "housing_gap",
    "front",
    "back",
    "x",
    "r",
    "retract",
    "clearance",
    "self_clearance",
    "short_end",
];

/// `j` with every length scaled by `s` (inches to mm: 25.4).
fn scaled(j: &Json, s: f64, key: Option<&str>) -> Json {
    let length = key.is_some_and(|k| LENGTHS.contains(&k));
    match j {
        Json::Num(n) if length => Json::from(n * s),
        Json::Arr(v) => Json::Arr(v.iter().map(|x| scaled(x, s, key)).collect()),
        Json::Obj(f) => Json::Obj(
            f.iter()
                .map(|(k, v)| (k.clone(), scaled(v, s, Some(k))))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Where the tooling came from, for the reply: a standard set, a vendor's profile, or a file.
fn source(kind: &str, id: &str, extra: Vec<(&str, Json)>) -> Json {
    let mut f = vec![("kind", Json::from(kind)), ("id", Json::from(id))];
    f.extend(extra);
    Json::obj(f)
}

/// The tooling asked for, and where it came from: `--vendor` (a vendor id, or a JSON file with a
/// `tooling` field), `--tooling` (a set id or a JSON file), else the sheet's own vendor's, else
/// the standard set std-eu.
fn tooling(ask: &Ask, dir: Option<&Path>, sheet_vendor: Option<&str>) -> Result<(fb::Tooling, Json), Answer> {
    let read = |p: &str| -> Result<Json, Answer> {
        let path = dir.map_or_else(|| PathBuf::from(p), |d| d.join(p));
        let text = std::fs::read_to_string(&path).map_err(|e| {
            err(
                "NoSuchTooling",
                format!("{}: {e}", path.display()),
                vec![("ids", strs(&fb::library::IDS))],
            )
        })?;
        Json::parse(&text).map_err(|e| err("BadTooling", format!("{}: {e}", path.display()), vec![]))
    };
    let standard = |why: &str| {
        (
            fb::library::std_eu(),
            source(
                "standard",
                "std-eu",
                vec![
                    ("why", why.into()),
                    ("real", false.into()),
                    ("note", STANDIN.into()),
                ],
            ),
        )
    };
    if let Some(p) = &ask.press {
        return library_tooling(ask, p, dir);
    }
    let vendor = ask
        .vendor
        .as_deref()
        .or(if ask.tooling.is_none() { sheet_vendor } else { None });
    if let Some(v) = vendor {
        if v.ends_with(".json") {
            let j = read(v)?;
            let t = j
                .get("tooling")
                .ok_or_else(|| err("BadTooling", format!("vendor file {v} has no `tooling`"), vec![]))?;
            return Ok((tooling_of(t)?, source("file", v, vec![])));
        }
        let profiles = forge_vendor::Profiles::cached(dir);
        let Some(e) = profiles.find(v) else {
            return Err(err("NoSuchVendor", format!("no vendor profile `{v}`"), vec![]));
        };
        let ven = &e.vendor;
        let mut extra = vec![
            ("vendor", Json::from(ven.name.as_str())),
            ("source", ven.source.as_deref().map_or(Json::Null, Json::from)),
            ("taken", ven.taken.as_deref().map_or(Json::Null, Json::from)),
            (
                "verify",
                "restated from public guidelines, not official: verify with the vendor before ordering"
                    .into(),
            ),
        ];
        let t = match &ven.tooling {
            None => {
                // nothing published: the standard set, said so
                let (t, _) = standard("the vendor publishes no press-brake tooling");
                extra.push((
                    "why",
                    "the vendor publishes no press-brake tooling: the standard set std-eu".into(),
                ));
                return Ok((t, source("vendor", &ven.id, extra)));
            }
            Some(forge_vendor::Tooling::Set(id)) => fb::library::set(id).ok_or_else(|| {
                err(
                    "NoSuchTooling",
                    format!("vendor {v} names tooling set `{id}`"),
                    vec![("ids", strs(&fb::library::IDS))],
                )
            })?,
            Some(forge_vendor::Tooling::Table(t)) => {
                // lengths in the file's units, angles in degrees
                let j = match ven.units {
                    forge_vendor::Units::In => scaled(&table_json(t), 25.4, None),
                    forge_vendor::Units::Mm => table_json(t),
                };
                if let Some(n) = j.get("note").and_then(Json::str) {
                    extra.push(("note", n.into()));
                }
                tooling_of(&j)?
            }
        };
        return Ok((t, source("vendor", &ven.id, extra)));
    }
    match &ask.tooling {
        None => Ok(standard("no vendor or tooling given: the standard set")),
        Some(t) => match fb::library::set(t) {
            Some(s) => Ok((
                s,
                source(
                    "standard",
                    t,
                    vec![
                        ("why", "asked for".into()),
                        ("real", false.into()),
                        ("note", STANDIN.into()),
                    ],
                ),
            )),
            None if t.ends_with(".json") => Ok((tooling_of(&read(t)?)?, source("file", t, vec![]))),
            None => Err(err(
                "NoSuchTooling",
                format!("no tooling set `{t}`: a standard set or a .json file"),
                vec![("ids", strs(&fb::library::IDS))],
            )),
        },
    }
}

/// Ranked programs: each its order, what it asks of the operator and the setter, and its steps'
/// tooling and orientation.
/// The staged program: its stations along the bed (each with its tools, place, lengths, the
/// steps made there and why its punch), every step's station, the moves between them, and the
/// ram's time where the press's speeds are published.
fn staged_json(
    sh: &SheetOut,
    s: &fb::Staged,
    names: &[String],
    speeds: Option<&fb::catalog::Speeds>,
) -> Json {
    let tl = &s.tooling;
    let stations: Vec<Json> = s
        .stations
        .iter()
        .enumerate()
        .map(|(k, u)| {
            let bends: Vec<Json> = u
                .steps
                .iter()
                .map(|i| names[s.plan.steps[*i].bend].as_str().into())
                .collect();
            let mut why = match u.kind {
                fb::PunchKind::Straight => "a straight punch: the first choice".to_string(),
                kind => {
                    let mut rules: Vec<&str> = u
                        .instead_of
                        .iter()
                        .filter(|r| tl.punch(&r.0).is_some_and(|p| p.kind == fb::PunchKind::Straight))
                        .map(|r| r.1.word())
                        .collect();
                    rules.sort_unstable();
                    rules.dedup();
                    let article = if kind == fb::PunchKind::Acute { "an" } else { "a" };
                    if rules.is_empty() {
                        format!("{article} {} punch: it makes these bends", kind.word())
                    } else {
                        format!(
                            "{article} {} punch: every straight punch failed ({})",
                            kind.word(),
                            rules.join(", ")
                        )
                    }
                }
            };
            if let Some(c) = &u.punch_clear {
                let at = c.part.as_ref().map(|p| part(sh, p));
                let what = at
                    .as_ref()
                    .and_then(|a| a.get("anchor"))
                    .and_then(Json::str)
                    .map(str::to_string)
                    .unwrap_or_else(|| "the part".into());
                let at = c.bound.unwrap_or(c.value);
                why.push_str(&if c.near.is_some() {
                    format!(
                        "; {what} comes within {} mm of it: near the {} mm limit, within the check's resolution (a warning)",
                        r6s(at),
                        r6s(c.limit)
                    )
                } else {
                    format!("; {what} clears it by {} mm (limit {} mm)", r6s(at), r6s(c.limit))
                });
            }
            Json::obj(vec![
                ("n", (k + 1).into()),
                ("punch", u.punch.as_str().into()),
                ("die", u.die.as_str().into()),
                ("kind", u.kind.word().into()),
                ("y", r6(u.y)),
                ("punch_length", r6(u.punch_length)),
                ("die_length", r6(u.die_length)),
                ("reach", Json::Arr(vec![r6(u.reach.0), r6(u.reach.1)])),
                ("bends", Json::Arr(bends)),
                ("why", why.as_str().into()),
                (
                    "punch_clear",
                    u.punch_clear.as_ref().map_or(Json::Null, |c| check(sh, c)),
                ),
            ])
        })
        .collect();
    let steps: Vec<Json> = s
        .plan
        .steps
        .iter()
        .zip(&s.station_of)
        .zip(&s.travel)
        .map(|((st, k), t)| {
            Json::obj(vec![
                ("bend", names[st.bend].as_str().into()),
                ("station", (k + 1).into()),
                ("travel", Json::Arr(vec![r6(t.approach), r6(t.work), r6(t.ret)])),
                (
                    "seconds",
                    speeds.and_then(|sp| t.seconds(sp)).map_or(Json::Null, r6),
                ),
            ])
        })
        .collect();
    let ram = speeds.and_then(|sp| s.ram_seconds(sp));
    Json::obj(vec![
        ("ok", true.into()),
        ("stations", Json::Arr(stations)),
        ("steps", Json::Arr(steps)),
        ("flips", s.flips.into()),
        ("turns", s.turns.into()),
        ("shifts", s.shifts.into()),
        ("tool_changes", 0.0.into()),
        ("tight", s.tight.into()),
        ("programs_tried", s.programs_tried.into()),
        ("layouts_tried", s.layouts_tried.into()),
        (
            "time",
            Json::obj(vec![
                ("ram_seconds", ram.map_or(Json::Null, r6)),
                (
                    "counted",
                    "the ram's approach, bend and return at the press's published speeds".into(),
                ),
                (
                    "not_counted",
                    "handling, the part's moves between stations, gauging (no published times)".into(),
                ),
            ]),
        ),
    ])
}

/// A tool body's words: `punch:trumpf/OW300-S/body` the punch trumpf/OW300-S's body, ...
fn tool_words(name: &str) -> String {
    let (station, rest) = match name.strip_prefix("station").and_then(|r| r.split_once(':')) {
        Some((k, r)) => (Some(k.to_string()), r),
        None => (None, name),
    };
    let what = if let Some(r) = rest.strip_prefix("punch:") {
        match r.rsplit_once('/') {
            Some((id, part)) if part == "body" || part == "tip" => format!("the punch {id}'s {part}"),
            _ => format!("the punch {r}"),
        }
    } else if let Some(id) = rest.strip_prefix("die:") {
        format!("the die {id}")
    } else if let Some(f) = rest.strip_prefix("backgauge:") {
        format!("the backgauge's {f}")
    } else if let Some(side) = rest.strip_prefix("frame:") {
        format!("the press frame's {side} housing")
    } else {
        format!("the press's {rest}")
    };
    match station {
        Some(k) => format!("{what} (station {k})"),
        None => what,
    }
}

/// Where a step's part and the tools come within the clearance, on both bodies: `contacts`.
/// Only a step (or a blocked bend's best attempt) whose tool check failed or is Near pays for it.
fn contacts_json(
    sh: &SheetOut,
    m: &fb::Model,
    tl: &fb::Tooling,
    plan: &fb::Plan,
    names: &[String],
) -> Vec<Json> {
    let mut out = Vec::new();
    let mut done = vec![false; m.bends.len()];
    let mut items: Vec<(Option<usize>, Vec<bool>, &fb::Step)> = Vec::new();
    for (i, st) in plan.steps.iter().enumerate() {
        items.push((Some(i + 1), done.clone(), st));
        done[st.bend] = true;
    }
    for b in &plan.blocking {
        // a blocked bend not among the steps: its best attempt, after the bends it followed
        if plan.steps.iter().any(|st| st.bend == b.bend) {
            continue;
        }
        let mut d = vec![false; m.bends.len()];
        for k in &b.after {
            d[*k] = true;
        }
        items.push((None, d, &b.attempt));
    }
    // each step's contacts are its own: on all cores, in step order
    let found = forge_math::par::map(items.len(), |i| {
        let (_, done, st) = &items[i];
        fb::contacts_of(m, tl, done, st)
    });
    for ((n, _, st), cs) in items.iter().zip(found) {
        for c in cs {
            out.push(contact_json(sh, m, &c, *n, st, names));
        }
    }
    out
}

fn contact_json(
    sh: &SheetOut,
    m: &fb::Model,
    c: &fb::Contact,
    n: Option<usize>,
    st: &fb::Step,
    names: &[String],
) -> Json {
    let res = (c.limit / 4.0).max(0.01);
    let verdict = if c.depth.is_some() {
        "hit"
    } else if c.near.is_some() {
        "near"
    } else {
        "undecided"
    };
    let bend = &names[st.bend];
    let half = 0.5 * m.bends[st.bend].len;
    let (ylo, yhi) = c.points.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| {
        (a.0.min(p[1]), a.1.max(p[1]))
    });
    let faces: Vec<Json> = c
        .faces
        .iter()
        .map(|f| {
            let mut j = part(sh, &f.part);
            if let Json::Obj(v) = &mut j {
                v.push(("cells".into(), f.cells.len().into()));
                v.push(("proven".into(), f.proven.into()));
            }
            j
        })
        .collect();
    let anchor = c
        .faces
        .iter()
        .max_by_key(|f| (f.proven, f.cells.len()))
        .and_then(|f| {
            part(sh, &f.part)
                .get("anchor")
                .and_then(Json::str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "the part".into());
    let how = match (verdict, c.depth) {
        (_, Some(d)) if d < 0.0 => format!("runs {} mm into", r6s(-d)),
        (_, Some(d)) => format!(
            "comes within {} mm (under the {} mm clearance) of",
            r6s(d),
            r6s(c.limit)
        ),
        ("near", _) => format!(
            "comes within the {} mm resolution of the {} mm clearance to",
            r6s(res),
            r6s(c.limit)
        ),
        _ => format!("is not proven {} mm clear of", r6s(c.limit)),
    };
    let deg = |x: f64| (x.to_degrees() * 10.0).round() / 10.0;
    let (when, span) = match c.phase {
        fb::Phase::Bend => {
            let (a, b) = (deg(c.span.0), deg(c.span.1));
            let at = if (b - a).abs() < 0.05 {
                format!("{a}°")
            } else {
                format!("{a}°–{b}°")
            };
            (
                format!("at {at} of"),
                Json::obj(vec![("angle_deg", Json::Arr(vec![a.into(), b.into()]))]),
            )
        }
        fb::Phase::Approach | fb::Phase::Return => (
            if c.phase == fb::Phase::Approach {
                "as the ram comes down for".to_string()
            } else {
                "as the ram goes back up after".to_string()
            },
            Json::obj(vec![("ram_mm", Json::Arr(vec![r6(c.span.0), r6(c.span.1)]))]),
        ),
        fb::Phase::Static => ("lying on the die for".to_string(), Json::obj(vec![])),
    };
    let step = match n {
        Some(n) => format!("step {n} (bend {bend})"),
        None => format!("bend {bend}'s best attempt"),
    };
    // part against part: the other faces it meets, by name
    let mut others: Vec<String> = Vec::new();
    for o in &c.others {
        if let Some(a) = part(sh, o).get("anchor").and_then(Json::str)
            && !others.iter().any(|x| x == a)
        {
            others.push(a.to_string());
        }
    }
    let target = if c.tool == "part" {
        if others.is_empty() {
            "the part itself".to_string()
        } else {
            others.join(" and ")
        }
    } else {
        tool_words(&c.tool)
    };
    let text = format!(
        "{anchor} {how} {} {}–{} mm from the bend's left end, {when} {step}",
        target,
        r6s(ylo + half),
        r6s(yhi + half)
    );
    // at most 64 points each way, spread over the region
    let pick = |v: &[[f64; 3]]| -> Json {
        let k = v.len().div_ceil(64).max(1);
        Json::Arr(
            v.iter()
                .step_by(k)
                .map(|p| Json::Arr(p.iter().map(|x| r6(*x)).collect()))
                .collect(),
        )
    };
    let cells: usize = c.faces.iter().map(|f| f.cells.len()).sum();
    let proven: usize = c.faces.iter().map(|f| f.proven).sum();
    Json::obj(vec![
        ("step", n.map_or(Json::Null, |n| n.into())),
        ("bend", bend.as_str().into()),
        ("rule", c.rule.word().into()),
        ("severity", if c.severity == Severity::Error { "error" } else { "warning" }.into()),
        ("verdict", verdict.into()),
        ("phase", c.phase.word().into()),
        ("tool", c.tool.as_str().into()),
        ("estimate", c.estimate.into()),
        ("faces", Json::Arr(faces)),
        ("others", Json::Arr(c.others.iter().map(|o| part(sh, o)).collect())),
        ("depth", c.depth.map_or(Json::Null, r6)),
        ("lower", r6(c.lower)),
        ("limit", r6(c.limit)),
        ("when", span),
        (
            "region",
            Json::obj(vec![
                ("points", pick(&c.points)),
                ("tool_points", pick(&c.tool_points)),
                (
                    "bbox",
                    Json::Arr(vec![
                        Json::Arr(c.bbox.0.iter().map(|x| r6(*x)).collect()),
                        Json::Arr(c.bbox.1.iter().map(|x| r6(*x)).collect()),
                    ]),
                ),
                ("cells", cells.into()),
                ("proven", proven.into()),
                ("complete", c.complete.into()),
                (
                    "exact",
                    "proven cells hold a real point under the limit; the rest are bounded: the region is never smaller than the true contact"
                        .into(),
                ),
            ]),
        ),
        ("text", text.as_str().into()),
    ])
}

fn programs_json(tl: &fb::Tooling, ps: &[fb::Program], names: &[String]) -> Json {
    Json::Arr(
        ps.iter()
            .enumerate()
            .map(|(i, p)| {
                Json::obj(vec![
                    ("rank", (i + 1).into()),
                    (
                        "sequence",
                        Json::Arr(
                            p.plan
                                .sequence
                                .iter()
                                .map(|b| names[*b].as_str().into())
                                .collect(),
                        ),
                    ),
                    ("flips", p.flips.into()),
                    ("turns", p.turns.into()),
                    ("stations", p.setups.into()),
                    ("tool_changes", p.changes.into()),
                    (
                        "steps",
                        Json::Arr(
                            p.plan
                                .steps
                                .iter()
                                .map(|s| {
                                    Json::obj(vec![
                                        ("bend", names[s.bend].as_str().into()),
                                        ("punch", tl.punches[s.setup.punch].id.as_str().into()),
                                        ("die", tl.dies[s.setup.die].id.as_str().into()),
                                        ("rotate", s.setup.rotate.into()),
                                        ("flip", s.flip.into()),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                ])
            })
            .collect(),
    )
}

const STANDIN: &str = "a stand-in set: forgecad's own round numbers, not a maker's catalogue";

/// The library: built in, then the part's folder's `bend/tooling/*.toml`, then the user's
/// (`<config>/forgecad/bend/tooling`); a file that does not read is an error.
pub fn catalog(dir: Option<&Path>) -> Result<fb::catalog::Catalog, Answer> {
    let mut levels: Vec<PathBuf> = Vec::new();
    if let Some(d) = dir {
        levels.push(d.join("bend").join("tooling"));
    }
    if let Some(c) = forge_dirs::config() {
        levels.push(c.join("forgecad").join("bend").join("tooling"));
    }
    let mut files: Vec<(String, String)> = Vec::new();
    for l in levels {
        let Ok(rd) = std::fs::read_dir(&l) else { continue };
        let mut ps: Vec<PathBuf> = rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        ps.sort();
        for p in ps {
            let text = std::fs::read_to_string(&p).map_err(|e| bad(format!("{}: {e}", p.display())))?;
            files.push((p.display().to_string(), text));
        }
    }
    let mut srcs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    srcs.extend(fb::catalog::BUILTIN.iter().copied());
    fb::catalog::Catalog::from_sources(&srcs).map_err(|e| err("BadLibrary", e.to_string(), vec![]))
}

fn unknown_err(e: fb::ToolingError) -> Answer {
    match e {
        fb::ToolingError::Unknown { id, fields } => err(
            "UnknownGeometry",
            format!("{id}: not known: {}; it cannot be simulated", fields.join(", ")),
            vec![
                ("id", id.as_str().into()),
                (
                    "fields",
                    Json::Arr(fields.iter().map(|f| f.as_str().into()).collect()),
                ),
            ],
        ),
        fb::ToolingError::NoPunches | fb::ToolingError::NoDies => err(
            "NoDefaultTooling",
            "the library holds none of this press maker's tools and its clamping system is not published: name the punches and dies"
                .to_string(),
            vec![],
        ),
        other => err("BadTooling", format!("{other:?}"), vec![]),
    }
}

/// A press from the library and its tools (named, else every known one of its clamping system).
fn library_tooling(ask: &Ask, press: &str, dir: Option<&Path>) -> Result<(fb::Tooling, Json), Answer> {
    let cat = catalog(dir)?;
    let Some(rec) = cat.press(press) else {
        let ids: Vec<&str> = cat.presses.iter().map(|p| p.id.as_str()).collect();
        return Err(err(
            "NoSuchPress",
            format!("no press `{press}` in the library"),
            vec![("ids", strs(&ids))],
        ));
    };
    let t = match (&ask.punches, &ask.dies) {
        (None, None) => cat.tooling_for(press),
        (ps, ds) => {
            let all_p: Vec<String> = cat
                .default_tools(rec, true)
                .into_iter()
                .map(String::from)
                .collect();
            let all_d: Vec<String> = cat
                .default_tools(rec, false)
                .into_iter()
                .map(String::from)
                .collect();
            let ps: Vec<&str> = ps.as_ref().unwrap_or(&all_p).iter().map(String::as_str).collect();
            let ds: Vec<&str> = ds.as_ref().unwrap_or(&all_d).iter().map(String::as_str).collect();
            cat.tooling(press, &ps, &ds)
        }
    }
    .map_err(unknown_err)?;
    let mut extra = vec![
        ("maker", rec.prov.maker.as_str().into()),
        ("model", rec.model.as_str().into()),
        ("real", rec.prov.real.into()),
        ("clamp", rec.clamp.as_str().into()),
        ("file", rec.prov.file.as_str().into()),
        (
            "verify",
            "restated from the makers' public specs and catalogues: verify against your machine and tools"
                .into(),
        ),
    ];
    if !t.press.unchecked.is_empty() {
        extra.push((
            "unknown",
            Json::Arr(t.press.unchecked.iter().map(|u| u.as_str().into()).collect()),
        ));
    }
    Ok((t, source("library", press, extra)))
}

fn bad(text: impl Into<String>) -> Answer {
    err("BadTooling", text, vec![])
}

fn num(j: &Json, k: &str) -> Option<f64> {
    j.get(k).and_then(Json::num)
}

fn need(j: &Json, k: &str, what: &str) -> Result<f64, Answer> {
    num(j, k).ok_or_else(|| bad(format!("{what} needs `{k}`")))
}

fn deg(d: f64) -> f64 {
    d * dmath::PI / 180.0
}

fn segments(j: &Json) -> Vec<fb::Segment> {
    match j.get("segments") {
        Some(Json::Arr(v)) => v
            .iter()
            .filter_map(|s| {
                Some(fb::Segment {
                    length: num(s, "length")?,
                    count: num(s, "count").unwrap_or(1.0) as u32,
                })
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn style(j: &Json) -> fb::Style {
    if j.get("style").and_then(Json::str) == Some("us") {
        fb::Style::Us
    } else {
        fb::Style::Eu
    }
}

fn profile(j: &Json) -> Result<Option<Vec<fb::geo::Seg>>, Answer> {
    let Some(Json::Arr(pts)) = j.get("profile") else {
        return Ok(None);
    };
    // a polygon: [[x, z], ..]
    let mut v = Vec::new();
    for p in pts {
        match p {
            Json::Arr(xy) if xy.len() == 2 => {
                v.push((xy[0].num().unwrap_or(f64::NAN), xy[1].num().unwrap_or(f64::NAN)))
            }
            _ => return Err(bad("a profile is a list of [x, z] points")),
        }
    }
    let n = v.len();
    Ok(Some(
        (0..n)
            .map(|i| fb::geo::Seg::Line {
                a: Point2::new(v[i].0, v[i].1),
                b: Point2::new(v[(i + 1) % n].0, v[(i + 1) % n].1),
            })
            .collect(),
    ))
}

fn punch_of(j: &Json) -> Result<fb::Punch, Answer> {
    let id = j
        .get("id")
        .and_then(Json::str)
        .ok_or_else(|| bad("a punch needs `id`"))?
        .to_string();
    let kind = match j.get("kind").and_then(Json::str).unwrap_or("straight") {
        "straight" => fb::PunchKind::Straight,
        "gooseneck" => fb::PunchKind::Gooseneck,
        "acute" => fb::PunchKind::Acute,
        k => {
            return Err(bad(format!(
                "punch {id}: kind is straight, gooseneck or acute, not {k}"
            )));
        }
    };
    let w = format!("punch {id}");
    Ok(fb::Punch {
        style: style(j),
        kind,
        height: need(j, "height", &w)?,
        tip_radius: need(j, "tip_radius", &w)?,
        tip_angle: deg(need(j, "tip_angle", &w)?),
        tip_height: num(j, "tip_height").unwrap_or(12.0),
        width: need(j, "width", &w)?,
        relief: match j.get("relief") {
            Some(r @ Json::Obj(_)) => Some(fb::tooling::Relief {
                depth: need(r, "depth", &w)?,
                z0: need(r, "z0", &w)?,
                z1: need(r, "z1", &w)?,
            }),
            _ => None,
        },
        profile: profile(j)?,
        segments: segments(j),
        max_load: num(j, "max_load"),
        drawn_width: None,
        id,
    })
}

fn die_of(j: &Json) -> Result<fb::Die, Answer> {
    let id = j
        .get("id")
        .and_then(Json::str)
        .ok_or_else(|| bad("a die needs `id`"))?
        .to_string();
    let w = format!("die {id}");
    Ok(fb::Die {
        style: style(j),
        v: need(j, "v", &w)?,
        angle: deg(need(j, "angle", &w)?),
        shoulder_radius: need(j, "shoulder_radius", &w)?,
        height: need(j, "height", &w)?,
        width: need(j, "width", &w)?,
        floor: num(j, "floor"),
        profile: profile(j)?,
        segments: segments(j),
        max_load: num(j, "max_load"),
        id,
    })
}

fn press_into(j: &Json, p: &mut fb::Press) {
    if let Some(id) = j.get("id").and_then(Json::str) {
        p.id = id.into();
    }
    let set = |k: &str, v: &mut f64| {
        if let Some(x) = num(j, k) {
            *v = x;
        }
    };
    set("open_height", &mut p.open_height);
    set("stroke", &mut p.stroke);
    set("bed_length", &mut p.bed_length);
    set("housing_gap", &mut p.housing_gap);
    if let Some(f) = num(j, "max_force") {
        p.max_force = Some(f);
    }
    for (k, beam) in [("ram", &mut p.ram), ("bed", &mut p.bed)] {
        if let Some(b) = j.get(k) {
            if let Some(x) = num(b, "front") {
                beam.front = x;
            }
            if let Some(x) = num(b, "back") {
                beam.back = x;
            }
        }
    }
    if let Some(b) = j.get("backgauge") {
        let g = &mut p.backgauge;
        if let Some(n) = num(b, "fingers") {
            g.fingers = n as u32;
        }
        let pair = |k: &str| match b.get(k) {
            Some(Json::Arr(v)) if v.len() == 2 => Some((v[0].num()?, v[1].num()?)),
            _ => None,
        };
        if let Some(x) = pair("x") {
            g.x = x;
        }
        if let Some(r) = pair("r") {
            g.r = r;
        }
        if let Some(x) = num(b, "retract") {
            g.retract = x;
        }
        if let Some(f) = b.get("finger") {
            if let Some(x) = num(f, "depth") {
                g.finger.depth = x;
            }
            if let Some(x) = num(f, "height") {
                g.finger.height = x;
            }
            if let Some(x) = num(f, "width") {
                g.finger.width = x;
            }
        }
    }
}

/// A tooling section: a set id, a whole set, or a set with tools added or replaced by id.
pub fn tooling_of(j: &Json) -> Result<fb::Tooling, Answer> {
    if let Some(id) = j.str() {
        return fb::library::set(id).ok_or_else(|| {
            err(
                "NoSuchTooling",
                format!("no tooling set `{id}`"),
                vec![("ids", strs(&fb::library::IDS))],
            )
        });
    }
    let base = j.get("base").and_then(Json::str).unwrap_or("std-eu");
    let mut t = fb::library::set(base).ok_or_else(|| {
        err(
            "NoSuchTooling",
            format!("no tooling set `{base}`"),
            vec![("ids", strs(&fb::library::IDS))],
        )
    })?;
    // a list given is all there is (a vendor's tooling narrows the set); one left out is the
    // base set's
    if j.get("punches").is_some() {
        t.punches.clear();
    }
    if j.get("dies").is_some() {
        t.dies.clear();
    }
    if let Some(id) = j.get("id").and_then(Json::str) {
        t.id = id.into();
    }
    if let Some(Json::Arr(ps)) = j.get("punches") {
        for p in ps {
            let p = punch_of(p)?;
            match t.punches.iter_mut().find(|q| q.id == p.id) {
                Some(q) => *q = p,
                None => t.punches.push(p),
            }
        }
    }
    if let Some(Json::Arr(ds)) = j.get("dies") {
        for d in ds {
            let d = die_of(d)?;
            match t.dies.iter_mut().find(|q| q.id == d.id) {
                Some(q) => *q = d,
                None => t.dies.push(d),
            }
        }
    }
    if let Some(p) = j.get("press") {
        press_into(p, &mut t.press);
    }
    if let Some(x) = num(j, "clearance") {
        t.clearance = x;
    }
    if let Some(x) = num(j, "self_clearance") {
        t.self_clearance = x;
    }
    if let Some(x) = num(j, "short_end") {
        t.short_end = Some(x);
    }
    if let Some(x) = num(j, "v_rule") {
        t.v_rule = x;
    }
    if let Some(Json::Arr(rows)) = j.get("springback") {
        t.springback = rows
            .iter()
            .filter_map(|r| {
                Some(fb::Springback {
                    material: r.get("material").and_then(Json::str).unwrap_or("*").into(),
                    angle: deg(num(r, "angle")?),
                    overbend: deg(num(r, "overbend")?),
                })
            })
            .collect();
    }
    t.validate().map_err(|e| bad(format!("{e:?}")))?;
    Ok(t)
}

fn r6(x: f64) -> Json {
    if x.is_finite() {
        Json::from((x * 1e6).round() / 1e6)
    } else {
        Json::Null
    }
}

fn rdeg(x: f64) -> Json {
    r6(x.to_degrees())
}

/// Rules whose value and limit are angles.
fn angular(r: Rule) -> bool {
    matches!(r, Rule::DieAngle | Rule::PunchAngle)
}

/// A part place as the part's anchors name it: a wall's skins `W.top`, `W.bottom`; a flange's
/// edge faces `F.tip.face`, `F.start_side.face`, `F.end_side.face`; a bend zone's skins
/// `F.bend_top`, `F.bend_bottom`.
fn part(sh: &SheetOut, p: &PartRef) -> Json {
    let (strip, name) = match p.strip {
        Strip::Wall(w) => ("wall", wall_name(sh, w)),
        Strip::Bend(j) => ("bend", joint_name(sh, j)),
    };
    let flange = matches!(p.strip, Strip::Wall(w) if sh.heights.iter().any(|h| h.0 == w));
    let anchor = match (p.strip, p.surface, p.segment) {
        (Strip::Wall(_), Surface::Side, Some(s)) if flange && s <= 2 => {
            format!("{name}.{}.face", ["start_side", "tip", "end_side"][s as usize])
        }
        (Strip::Wall(_), Surface::Bottom, _) => format!("{name}.bottom"),
        (Strip::Wall(_), _, _) => format!("{name}.top"),
        (Strip::Bend(_), Surface::Bottom, _) => format!("{name}.bend_bottom"),
        (Strip::Bend(_), _, _) => format!("{name}.bend_top"),
    };
    let surface = match p.surface {
        Surface::Top => "top",
        Surface::Bottom => "bottom",
        Surface::Side => "side",
    };
    let mut f = vec![
        ("anchor", Json::from(anchor.as_str())),
        ("strip", strip.into()),
        ("name", name.as_str().into()),
        ("surface", surface.into()),
    ];
    if let Some(s) = p.segment {
        f.push(("segment", (s as f64).into()));
    }
    Json::obj(f)
}

fn check(sh: &SheetOut, c: &Check) -> Json {
    let a = angular(c.rule);
    let v = |x: f64| if a { rdeg(x) } else { r6(x) };
    let mut f = vec![
        ("rule", Json::from(c.rule.word())),
        ("pass", c.pass.into()),
        (
            "severity",
            (if c.severity == Severity::Error {
                "error"
            } else {
                "warning"
            })
            .into(),
        ),
        ("value", v(c.value)),
        ("limit", v(c.limit)),
        ("compare", (if c.at_most { "<=" } else { ">=" }).into()),
        (
            "verdict",
            (if c.pass {
                "pass"
            } else if c.unchecked.is_some() {
                "not_checked"
            } else if c.near.is_some() {
                "near"
            } else if c.undecided {
                "undecided"
            } else {
                "fail"
            })
            .into(),
        ),
        ("certified", c.certified.into()),
        ("unit", (if a { "deg" } else { "mm" }).into()),
    ];
    if let Some(u) = &c.unchecked {
        f.push(("unknown", u.as_str().into()));
    }
    if let Some(r) = c.near {
        // proven at least `value`, within this of the limit: not proven at it
        f.push(("resolution", v(r)));
    }
    if c.estimate {
        f.push((
            "against",
            "estimated envelope (no published geometry; errs large)".into(),
        ));
    }
    if c.rule == Rule::Handling {
        f.push((
            "method",
            "the planned operator's move (lift, pull out, turn, push in, set down; the ram at top dead centre) against \
             every body of the press: proven clear by the part's grown boxes and sphere when they clear, else the part \
             itself carried through the move by the same certified search the bends use; a fail is a real point of the \
             part proven to meet the body (value: its distance, negative inside)"
                .into(),
        ));
    }
    if let Some(lo) = c.lo {
        f.push(("limit_lo", v(lo)));
    }
    if let Some(b) = c.bound {
        f.push(("bound", r6(b)));
    }
    if let Some(p) = &c.part {
        f.push(("part", part(sh, p)));
    }
    if let Some(p) = &c.other {
        f.push(("other", part(sh, p)));
    }
    if let Some(t) = &c.tool {
        f.push(("tool", t.as_str().into()));
    }
    if let Some(at) = c.at {
        f.push(("at", Json::Arr(at.iter().map(|x| r6(*x)).collect())));
    }
    if let Some(p) = c.phi {
        f.push(("phi", rdeg(p)));
    }
    if let Some(p) = c.phase {
        f.push(("phase", p.word().into()));
    }
    Json::obj(f)
}

/// Why the step's tools: the die by the set's V rule, the punch the first in order (straight,
/// gooseneck, acute) that passes, and the setups tried before it with the rule each failed.
fn choice(m: &fb::Model, tl: &fb::Tooling, st: &fb::Step) -> Json {
    let t = m.t;
    let die = tl.die(&st.die);
    let punch = tl.punch(&st.punch);
    let want = tl.v_rule * t;
    let die_why = match die {
        Some(d) if st.rejected.is_empty() => format!(
            "V {} nearest the rule V = {} t = {}; the first die tried",
            r6s(d.v),
            r6s(tl.v_rule),
            r6s(want)
        ),
        Some(d) => format!(
            "V {} (the rule asks V = {} t = {}): the first that passes",
            r6s(d.v),
            r6s(tl.v_rule),
            r6s(want)
        ),
        None => String::new(),
    };
    let straight_failed: Vec<&(String, String, bool, Rule)> = st
        .rejected
        .iter()
        .filter(|r| tl.punch(&r.0).is_some_and(|p| p.kind == fb::PunchKind::Straight))
        .collect();
    let punch_why = match punch.map(|p| p.kind) {
        Some(fb::PunchKind::Straight) => "a straight punch: the first choice".to_string(),
        // no setup tried before it: the last step's tools, kept (no tool change)
        Some(k) if st.rejected.is_empty() => format!(
            "{} {} punch: the step before's, kept (no tool change)",
            if k == fb::PunchKind::Acute { "an" } else { "a" },
            k.word()
        ),
        Some(k) => format!(
            "{} {} punch: every straight punch failed ({})",
            if k == fb::PunchKind::Acute { "an" } else { "a" },
            k.word(),
            {
                let mut rules: Vec<&str> = straight_failed.iter().map(|r| r.3.word()).collect();
                rules.sort_unstable();
                rules.dedup();
                rules.join(", ")
            }
        ),
        None => String::new(),
    };
    let rejected: Vec<Json> = st
        .rejected
        .iter()
        .take(24)
        .map(|r| {
            Json::obj(vec![
                ("punch", r.0.as_str().into()),
                ("die", r.1.as_str().into()),
                ("rotate", r.2.into()),
                ("rule", r.3.word().into()),
            ])
        })
        .collect();
    Json::obj(vec![
        ("die", die_why.as_str().into()),
        ("punch", punch_why.as_str().into()),
        ("v_rule", r6(tl.v_rule)),
        ("tried", ((st.rejected.len() + 1) as f64).into()),
        ("rejected", Json::Arr(rejected)),
    ])
}

fn r6s(x: f64) -> String {
    let v = (x * 1e3).round() / 1e3;
    format!("{v}")
}

fn answer(sh: &SheetOut, m: &fb::Model, tl: &fb::Tooling, plan: &fb::Plan, names: &[String]) -> Answer {
    let name = |k: usize| Json::from(names[k].as_str());
    let bends: Vec<Json> = m
        .bends
        .iter()
        .enumerate()
        .map(|(k, b)| {
            Json::obj(vec![
                ("n", ((k + 1) as f64).into()),
                ("bend", name(k)),
                ("angle", rdeg(b.theta)),
                ("direction", (if b.sigma > 0.0 { "up" } else { "down" }).into()),
                ("radius", r6(b.r)),
                ("length", r6(b.len)),
            ])
        })
        .collect();
    let mut text = String::new();
    text.push_str(&format!(
        "{}: {} on {} ({}): {}\n",
        sh.id,
        if plan.feasible {
            "feasible"
        } else {
            "no feasible sequence"
        },
        tl.id,
        tl.press.id,
        plan.sequence
            .iter()
            .map(|k| names[*k].as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    let steps: Vec<Json> = plan
        .steps
        .iter()
        .enumerate()
        .map(|(i, st)| {
            text.push_str(&format!(
                "  {}. {} {:.1} deg (+{:.1}): {} on {}, {}{}{}\n",
                i + 1,
                names[st.bend],
                st.angle.to_degrees(),
                st.overbend.to_degrees(),
                st.punch,
                st.die,
                if st.flip { "flipped, " } else { "" },
                if st.setup.rotate { "rotated, " } else { "" },
                if st.pass() {
                    "pass".to_string()
                } else {
                    format!(
                        "FAIL {}",
                        st.failures()
                            .iter()
                            .map(|c| c.rule.word())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                }
            ));
            let gauge = st.gauge.as_ref().map_or(Json::Null, |g| {
                Json::obj(vec![
                    ("x", r6(g.x)),
                    ("r", r6(g.r)),
                    ("fingers", Json::Arr(g.fingers.iter().map(|y| r6(*y)).collect())),
                    ("stop", part(sh, &g.part)),
                ])
            });
            Json::obj(vec![
                ("n", ((i + 1) as f64).into()),
                ("bend", name(st.bend)),
                ("angle", rdeg(st.angle)),
                ("overbend", rdeg(st.overbend)),
                ("punch", st.punch.as_str().into()),
                ("die", st.die.as_str().into()),
                ("punch_length", st.punch_length.map_or(Json::Null, r6)),
                ("die_length", st.die_length.map_or(Json::Null, r6)),
                (
                    "orientation",
                    Json::obj(vec![
                        ("flip", st.flip.into()),
                        ("rotate", st.setup.rotate.into()),
                        ("y", r6(0.0)),
                    ]),
                ),
                ("backgauge", gauge),
                ("travel", r6(st.travel)),
                ("force", st.force.map_or(Json::Null, r6)),
                ("pass", st.pass().into()),
                ("choice", choice(m, tl, st)),
                (
                    "checks",
                    Json::Arr(st.checks.iter().map(|c| check(sh, c)).collect()),
                ),
            ])
        })
        .collect();
    let blocking: Vec<Json> = plan
        .blocking
        .iter()
        .map(|b| {
            text.push_str(&format!(
                "  blocked: {} after [{}]: {}\n",
                names[b.bend],
                b.after
                    .iter()
                    .map(|k| names[*k].as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                b.checks
                    .iter()
                    .map(|c| c.rule.word())
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
            Json::obj(vec![
                ("bend", name(b.bend)),
                ("after", Json::Arr(b.after.iter().map(|k| name(*k)).collect())),
                ("punch", b.punch.as_str().into()),
                ("die", b.die.as_str().into()),
                (
                    "checks",
                    Json::Arr(b.checks.iter().map(|c| check(sh, c)).collect()),
                ),
            ])
        })
        .collect();
    let mut fixes = Vec::new();
    let suggestions: Vec<Json> = plan
        .suggestions
        .iter()
        .map(|s| match s {
            fb::Suggestion::LengthenFlange { bend, wall, by } => {
                if let Some(op) = sh.lengthen(*wall, *by) {
                    fixes.push(op.json());
                }
                Json::obj(vec![
                    ("kind", "LengthenFlange".into()),
                    ("bend", name(*bend)),
                    ("wall", wall_name(sh, *wall).as_str().into()),
                    ("by", r6(*by)),
                ])
            }
            fb::Suggestion::NeedTool { bend, rule, what } => Json::obj(vec![
                ("kind", "NeedTool".into()),
                ("bend", name(*bend)),
                ("rule", rule.word().into()),
                ("what", what.as_str().into()),
            ]),
            fb::Suggestion::UseTool { bend, punch, die } => Json::obj(vec![
                ("kind", "UseTool".into()),
                ("bend", name(*bend)),
                ("punch", punch.as_str().into()),
                ("die", die.as_str().into()),
            ]),
        })
        .collect();
    let json = Json::obj(vec![
        ("ok", true.into()),
        ("schema", SCHEMA.into()),
        ("sheet", sh.id.as_str().into()),
        ("tooling", tl.id.as_str().into()),
        ("press", tl.press.id.as_str().into()),
        ("material", sh.material.as_deref().map_or(Json::Null, Json::from)),
        (
            "springback",
            if tl.springback.is_empty() {
                Json::obj(vec![
                    ("known", false.into()),
                    (
                        "why",
                        "no cited springback for this material in forge-material: bent to the turn, no overbend".into(),
                    ),
                ])
            } else {
                Json::obj(vec![
                    ("known", true.into()),
                    (
                        "source",
                        "the stand-in set's rows (forgecad's own round numbers)".into(),
                    ),
                ])
            },
        ),
        ("clearance", r6(tl.clearance)),
        ("feasible", plan.feasible.into()),
        (
            "sequence",
            Json::Arr(plan.sequence.iter().map(|k| name(*k)).collect()),
        ),
        ("bends", Json::Arr(bends)),
        ("steps", Json::Arr(steps)),
        ("blocking", Json::Arr(blocking)),
        ("fixes", Json::Arr(fixes)),
        ("suggestions", Json::Arr(suggestions)),
        ("explored", (plan.explored as f64).into()),
        ("exhausted", plan.exhausted.into()),
    ]);
    Answer { ok: true, json, text }
}

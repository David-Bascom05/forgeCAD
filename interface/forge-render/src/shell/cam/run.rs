//! The document generated: forge-cam-lang's run of the text, and what the workspace shows of it
//! per setup and per operation. Read from the reply `forgecad.cam/1` (the data contract the
//! CLI and the agents get) and from the run's public paths, never re-derived.

use super::doc::{self, Role, Stmt};
use forge_agent::json::Json;
use std::path::Path;

/// A check as the reply gives it (a failure or an undecided one).
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub rule: String,
    pub verdict: String,
    pub value: f64,
    pub limit: f64,
    /// The witness, in the setup's WCS.
    pub at: [f64; 3],
    pub tool_part: String,
    pub other: String,
    pub phase: String,
    /// The move of the op's path it is on.
    pub mov: usize,
}

/// How an op's path is known to be safe (David, 2026-11-23: the generators are to be proven,
/// the checker kept as an oracle and an on-demand command).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Assurance {
    /// Proven by construction: its generator is proven (Lean, Aeneas L4); nothing checked.
    Proven,
    /// Checked on demand: the certified checker ran on it when asked (Check Now).
    CheckedOnDemand,
    /// Interim: the certified checker runs on every generate (forge-cam-lang today).
    #[default]
    CheckedAtGenerate,
}

impl Assurance {
    pub fn words(self) -> &'static str {
        match self {
            Assurance::Proven => "proven by construction",
            Assurance::CheckedOnDemand => "checked on demand",
            Assurance::CheckedAtGenerate => "checked at generate (interim)",
        }
    }
}

/// One op as generated.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OpRes {
    pub name: String,
    pub kind: String,
    pub setup: String,
    /// built | refused | unposted
    pub status: String,
    /// How its path is known safe.
    pub assurance: Assurance,
    pub tool: Option<String>,
    pub tool_t: Option<u32>,
    pub tool_d: Option<f64>,
    pub refusal: Option<String>,
    pub fix: Option<String>,
    pub checks: Vec<Check>,
    pub plan: Vec<String>,
    pub removals: Vec<String>,
    pub moves: usize,
    pub length_cut: f64,
    pub length_rapid: f64,
    pub time_s: f64,
    /// The least cutter distance the certificate found.
    pub least: Option<f64>,
    pub pairs: usize,
    pub rpm: Option<f64>,
    pub feed: Option<f64>,
    pub fz: Option<f64>,
    pub hex: Option<f64>,
    pub vc: Option<f64>,
    pub power_kw: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SetupRes {
    pub name: String,
    pub part: String,
    pub machine: String,
    pub origin: [f64; 3],
    pub offset: u32,
    pub stock_min: [f64; 3],
    pub stock_max: [f64; 3],
    pub tol: f64,
    pub clearance: f64,
    pub at: Option<[f64; 3]>,
    pub clear_h: f64,
    pub retract_h: f64,
    pub fixture: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProgRes {
    pub name: String,
    pub post: String,
    pub lines: usize,
    pub round_trip: bool,
    pub certified: String,
    pub error: Option<String>,
    pub text: Option<String>,
    /// A split program's part: the setup it posts (its name is `PROGRAM/SETUP`).
    pub setup: Option<String>,
}

/// A left-over region (G4).
#[derive(Clone, Debug, PartialEq)]
pub struct LeftRes {
    pub setup: String,
    pub z: [f64; 2],
    pub boxes: Vec<[f64; 4]>,
    pub why: String,
}

/// A whole run.
pub struct Run {
    /// The text it ran on.
    pub text: String,
    /// The statements of that text (what is stale is what changed since).
    pub stmts: Vec<Stmt>,
    pub ok: bool,
    pub certification: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub setups: Vec<SetupRes>,
    pub ops: Vec<OpRes>,
    pub programs: Vec<ProgRes>,
    pub leftover: Vec<LeftRes>,
    pub leftover_done: bool,
    /// The reply, as `forgecad cam --json` prints it.
    pub json: String,
    /// The setup sheet (HTML).
    pub sheet: String,
    /// The run itself: the ops' paths for the views (milling).
    pub cam: Option<forge_cam_lang::Cam>,
    /// A turning document's run (forge-turn-lang).
    pub turn: Option<forge_turn_lang::TurnCam>,
    /// "milling" or "turning".
    pub process: &'static str,
    /// The toolpaths' quality (forge-cam-quality: cycle time on the machine, engagement, chip,
    /// air cutting), measured on the run; milling only for now.
    pub quality: Option<forge_cam_quality::JobQ>,
}

fn s(j: &Json, k: &str) -> String {
    j.get(k).and_then(Json::str).unwrap_or_default().to_string()
}
fn n(j: &Json, k: &str) -> Option<f64> {
    j.get(k).and_then(Json::num)
}
fn v3(j: Option<&Json>) -> Option<[f64; 3]> {
    match j? {
        Json::Arr(a) if a.len() == 3 => Some([a[0].num()?, a[1].num()?, a[2].num()?]),
        _ => None,
    }
}
fn arr(j: &Json, k: &str) -> Vec<Json> {
    match j.get(k) {
        Some(Json::Arr(a)) => a.clone(),
        _ => vec![],
    }
}
fn strs(j: &Json, k: &str) -> Vec<String> {
    arr(j, k)
        .iter()
        .filter_map(|x| x.str().map(str::to_string))
        .collect()
}

/// Generates `text` (its files beside it in `dir`).
pub fn run(text: &str, dir: &Path) -> Run {
    if forge_turn_lang::is_turning(text) {
        return run_turning(text, dir);
    }
    let cam = forge_cam_lang::run(text, dir);
    let j = cam.json();
    let mut json = String::new();
    j.write(&mut json);
    let setups = arr(&j, "setups")
        .iter()
        .map(|x| {
            let wcs = x.get("wcs");
            let st = x.get("stock");
            let h = x.get("heights");
            SetupRes {
                name: s(x, "name"),
                part: s(x, "part"),
                machine: s(x, "machine"),
                origin: v3(wcs.and_then(|w| w.get("origin"))).unwrap_or_default(),
                offset: wcs.and_then(|w| n(w, "offset")).unwrap_or(0.0) as u32,
                stock_min: v3(st.and_then(|w| w.get("min"))).unwrap_or_default(),
                stock_max: v3(st.and_then(|w| w.get("max"))).unwrap_or_default(),
                tol: n(x, "tol").unwrap_or(0.0),
                clearance: n(x, "clearance").unwrap_or(0.0),
                at: v3(x.get("at")),
                clear_h: h.and_then(|h| n(h, "clear")).unwrap_or(0.0),
                retract_h: h.and_then(|h| n(h, "retract")).unwrap_or(0.0),
                fixture: x
                    .get("fixture")
                    .and_then(|f| f.get("name"))
                    .and_then(Json::str)
                    .map(str::to_string),
            }
        })
        .collect();
    let ops = arr(&j, "ops")
        .iter()
        .zip(&cam.ops)
        .map(|(x, o)| {
            let t = x.get("tool");
            let f = x.get("feeds");
            let p = x.get("path");
            let c = x.get("certificate");
            OpRes {
                name: s(x, "name"),
                kind: s(x, "kind"),
                setup: s(x, "setup"),
                status: s(x, "status"),
                // every op of forge-cam-lang is certified on the posted path when generated
                assurance: Assurance::CheckedAtGenerate,
                tool: t
                    .and_then(|t| t.get("id"))
                    .and_then(Json::str)
                    .map(str::to_string),
                tool_t: t.and_then(|t| n(t, "T")).map(|v| v as u32),
                tool_d: t.and_then(|t| n(t, "d")),
                refusal: o.refusal.clone(),
                fix: o.fix.clone(),
                checks: arr(x, "checks")
                    .iter()
                    .map(|c| Check {
                        rule: s(c, "rule"),
                        verdict: s(c, "verdict"),
                        value: n(c, "value").unwrap_or(f64::NAN),
                        limit: n(c, "limit").unwrap_or(f64::NAN),
                        at: v3(c.get("at")).unwrap_or_default(),
                        tool_part: s(c, "tool_part"),
                        other: s(c, "other"),
                        phase: s(c, "phase"),
                        mov: n(c, "move").unwrap_or(0.0) as usize,
                    })
                    .collect(),
                plan: strs(x, "plan"),
                removals: strs(x, "removals"),
                moves: p.and_then(|p| n(p, "moves")).unwrap_or(0.0) as usize,
                length_cut: p.and_then(|p| n(p, "length_cut")).unwrap_or(0.0),
                length_rapid: p.and_then(|p| n(p, "length_rapid")).unwrap_or(0.0),
                time_s: p.and_then(|p| n(p, "time_s")).unwrap_or(0.0),
                least: c.and_then(|c| n(c, "least_cutter_distance")),
                pairs: c.and_then(|c| n(c, "pairs")).unwrap_or(0.0) as usize,
                rpm: f.and_then(|f| n(f, "rpm")),
                feed: f.and_then(|f| n(f, "f")),
                fz: f.and_then(|f| n(f, "fz")),
                hex: f.and_then(|f| n(f, "hex")),
                vc: f.and_then(|f| n(f, "vc")),
                power_kw: f.and_then(|f| n(f, "power_kw")),
            }
        })
        .collect();
    let programs = arr(&j, "programs")
        .iter()
        .zip(&cam.programs)
        .map(|(x, p)| ProgRes {
            name: s(x, "name"),
            post: s(x, "post"),
            lines: n(x, "lines").unwrap_or(0.0) as usize,
            round_trip: matches!(x.get("round_trip"), Some(Json::Bool(true))),
            certified: s(x, "certified"),
            error: p.error.clone(),
            text: p.text.clone(),
            setup: p.setup.clone(),
        })
        .collect();
    let lo = j.get("leftover");
    let leftover = lo
        .map(|l| arr(l, "regions"))
        .unwrap_or_default()
        .iter()
        .map(|r| LeftRes {
            setup: s(r, "setup"),
            z: match r.get("z") {
                Some(Json::Arr(a)) if a.len() == 2 => [a[0].num().unwrap_or(0.0), a[1].num().unwrap_or(0.0)],
                _ => [0.0, 0.0],
            },
            boxes: arr(r, "boxes")
                .iter()
                .filter_map(|b| match b {
                    Json::Arr(a) if a.len() == 4 => {
                        Some([a[0].num()?, a[1].num()?, a[2].num()?, a[3].num()?])
                    }
                    _ => None,
                })
                .collect(),
            why: s(r, "why"),
        })
        .collect();
    Run {
        text: text.to_string(),
        stmts: doc::read(text).unwrap_or_default(),
        ok: cam.ok,
        certification: cam.certification().to_string(),
        errors: cam.errors.clone(),
        warnings: cam.warnings.clone(),
        setups,
        ops,
        programs,
        leftover,
        leftover_done: matches!(lo.and_then(|l| l.get("computed")), Some(Json::Bool(true))),
        json,
        sheet: cam.sheet(),
        quality: Some(forge_cam_quality::measure_cam(
            &cam,
            String::new(),
            &forge_cam_quality::Config::default(),
        )),
        cam: Some(cam),
        turn: None,
        process: "milling",
    }
}

/// A turning document generated (forge-turn-lang): its ops, programs and checks as the
/// workspace shows a milling run's. Checks are the report's refusals, at (ρ, z) in the part's
/// frame (`at` = [ρ, 0, z]).
fn run_turning(text: &str, dir: &Path) -> Run {
    let tc = forge_turn_lang::run(text, dir);
    let j = tc.json();
    let mut json = String::new();
    j.write(&mut json);
    let stmts = doc::read(text).unwrap_or_default();
    // each op's tool, from the document: `tool: NAME` → `NAME = lathe_tool("ID", t: N)`
    let tool_of = |op: &str| -> (Option<String>, Option<u32>) {
        let Some(name) = stmts
            .iter()
            .find(|s| s.name == op)
            .and_then(|s| s.arg("tool").map(str::to_string))
        else {
            return (None, None);
        };
        let Some(t) = stmts.iter().find(|s| s.name == name) else {
            return (None, None);
        };
        (
            t.pos(0).map(doc::unquote),
            t.arg("t").and_then(|x| x.trim().parse().ok()),
        )
    };
    let tools = forge_turn::insert::library(forge_turn::insert::BUILTIN).unwrap_or_default();
    let refusals = tc.report.as_ref().map(|r| r.refusals.clone()).unwrap_or_default();
    let ops = tc
        .ops
        .iter()
        .enumerate()
        .map(|(k, o)| {
            let (tool, t) = tool_of(&o.name);
            let rec = tool.as_ref().and_then(|id| tools.iter().find(|x| &x.id == id));
            OpRes {
                name: o.name.clone(),
                kind: o.kind.clone(),
                // forge-turn-lang says "planned" for an op no lathe_program posted
                status: if o.status == "planned" {
                    "unposted".into()
                } else {
                    o.status.clone()
                },
                assurance: Assurance::CheckedAtGenerate,
                tool,
                tool_t: t,
                // a lathe tool has no cutting diameter: its nose radius is in its record
                tool_d: None,
                refusal: o.refusal.clone(),
                fix: None,
                checks: refusals
                    .iter()
                    .filter(|r| r.op == k)
                    .map(|r| Check {
                        rule: r.rule.clone(),
                        verdict: r.verdict.clone(),
                        value: f64::NAN,
                        limit: f64::NAN,
                        at: [r.at[0], 0.0, r.at[1]],
                        tool_part: r.body.clone(),
                        other: r.rule.clone(),
                        phase: String::new(),
                        mov: r.step,
                    })
                    .collect(),
                moves: o.moves,
                vc: rec.map(|r| r.vc),
                fz: rec.map(|r| r.f),
                ..Default::default()
            }
        })
        .collect();
    let programs = tc
        .programs
        .iter()
        .map(|p| ProgRes {
            name: p.name.clone(),
            post: p.post.clone(),
            lines: p.text.as_ref().map_or(0, |t| t.lines().count()),
            round_trip: p.text.is_some(),
            certified: match (p.error.is_some(), tc.certification()) {
                (true, _) => "no".into(),
                (false, "full") => "full".into(),
                (false, _) => "except G5 (unchecked: no machine or not placed)".into(),
            },
            error: p.error.clone(),
            text: p.text.clone(),
            setup: None,
        })
        .collect();
    let setups = stmts
        .iter()
        .filter(|s| s.func == "lathe_setup" && !s.suppressed)
        .map(|s| {
            let bar: f64 = s
                .arg("bar")
                .and_then(|x| x.trim().parse().ok())
                .unwrap_or(f64::NAN);
            let front: f64 = s
                .arg("front")
                .and_then(|x| x.trim().parse().ok())
                .unwrap_or(f64::NAN);
            SetupRes {
                name: s.name.clone(),
                part: stmts
                    .iter()
                    .find(|p| Some(p.name.as_str()) == s.pos(0))
                    .and_then(|p| p.pos(0).map(doc::unquote))
                    .unwrap_or_default(),
                machine: s.arg_str("machine").unwrap_or_else(|| "none".into()),
                stock_min: [-bar / 2.0, -bar / 2.0, f64::NAN],
                stock_max: [bar / 2.0, bar / 2.0, front],
                tol: s.arg("tol").and_then(|x| x.trim().parse().ok()).unwrap_or(0.002),
                clearance: s
                    .arg("clearance")
                    .and_then(|x| x.trim().parse().ok())
                    .unwrap_or(1.0),
                at: s
                    .arg("at")
                    .and_then(|x| x.trim().parse().ok())
                    .map(|z: f64| [0.0, 0.0, z]),
                fixture: s.arg("chuck").map(str::to_string),
                ..Default::default()
            }
        })
        .collect();
    let leftover = match j.get("leftover").map(|l| arr(l, "regions")) {
        Some(rs) => rs
            .iter()
            .map(|r| {
                let pair = |k: &str| match r.get(k) {
                    Some(Json::Arr(a)) if a.len() == 2 => {
                        [a[0].num().unwrap_or(0.0), a[1].num().unwrap_or(0.0)]
                    }
                    _ => [0.0, 0.0],
                };
                let (rho, z) = (pair("rho"), pair("z"));
                LeftRes {
                    setup: String::new(),
                    z,
                    boxes: vec![[rho[0], z[0], rho[1], z[1]]],
                    why: s(r, "reason"),
                }
            })
            .collect(),
        None => vec![],
    };
    let sheet = turning_sheet(text, &tc);
    Run {
        text: text.to_string(),
        stmts,
        ok: tc.ok,
        certification: tc.certification().to_string(),
        errors: tc.errors.clone(),
        warnings: tc.warnings.clone(),
        setups,
        ops,
        programs,
        leftover,
        leftover_done: true,
        json,
        sheet,
        cam: None,
        turn: Some(tc),
        process: "turning",
        quality: None,
    }
}

/// A turning setup sheet: forge-turn-lang has none yet, so its ops and programs as a table.
fn turning_sheet(text: &str, tc: &forge_turn_lang::TurnCam) -> String {
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let mut h = String::from(
        "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>Setup sheet (turning)</title><style>body{font-family:sans-serif;margin:2em}table{border-collapse:collapse}td,th{border:1px solid #999;padding:4px 8px}</style></head><body>\n<h1>Setup sheet: turning</h1>\n<table><tr><th>op</th><th>kind</th><th>status</th><th>moves</th></tr>\n",
    );
    for o in &tc.ops {
        h.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            esc(&o.name),
            esc(&o.kind),
            esc(&o.status),
            o.moves
        ));
    }
    h.push_str("</table>\n<h2>Programs</h2>\n<ul>\n");
    for p in &tc.programs {
        h.push_str(&format!(
            "<li>{} ({}, O{}): {}</li>\n",
            esc(&p.name),
            esc(&p.post),
            p.number,
            p.text.as_ref().map_or_else(
                || p.error.clone().unwrap_or_default(),
                |t| format!("{} lines", t.lines().count())
            )
        ));
    }
    h.push_str(&format!(
        "</ul>\n<h2>Document</h2>\n<pre>{}</pre>\n</body></html>\n",
        esc(text)
    ));
    h
}

/// `r` checked again over its posted paths (forge-cam-lang's `Cam::check`), nothing
/// regenerated: each op's checks and status replaced by the new verdicts, its assurance "checked
/// on demand". A turning run is returned as it is (forge-turn-lang has no on-demand check yet).
pub fn recheck(mut r: Run) -> Run {
    let Some(cam) = &r.cam else { return r };
    for oc in cam.check(None) {
        let Some(o) = r.ops.iter_mut().find(|o| o.name == oc.op) else {
            continue;
        };
        o.checks = oc
            .checks
            .iter()
            .map(|c| Check {
                rule: c.rule.to_string(),
                verdict: c.verdict.to_string(),
                value: c.value,
                limit: c.limit,
                at: c.at,
                tool_part: c.tool_part.to_string(),
                other: c.other.clone(),
                phase: c.phase.to_string(),
                mov: c.move_index,
            })
            .collect();
        o.pairs = oc.pairs;
        o.assurance = Assurance::CheckedOnDemand;
        if o.status == "built" || o.status == "refused" {
            o.status = if oc.clear { "built" } else { "refused" }.into();
        }
    }
    r
}

impl Run {
    /// Program statement `name` as run: itself, or its parts (`NAME/SETUP`, one per setup when
    /// split) as one: lines summed, the least certified part's words, the texts one after another.
    pub fn program(&self, name: &str) -> Option<ProgRes> {
        let parts: Vec<&ProgRes> = self
            .programs
            .iter()
            .filter(|x| x.name == name || x.name.split_once('/').is_some_and(|(n, _)| n == name))
            .collect();
        match parts.as_slice() {
            [] => None,
            [one] => Some((*one).clone()),
            many => Some(ProgRes {
                name: name.to_string(),
                post: many[0].post.clone(),
                lines: many.iter().map(|x| x.lines).sum(),
                round_trip: many.iter().all(|x| x.round_trip),
                certified: many
                    .iter()
                    .find(|x| x.certified != "full")
                    .map_or_else(|| "full".to_string(), |x| x.certified.clone()),
                error: many.iter().find_map(|x| x.error.clone()),
                text: many
                    .iter()
                    .map(|x| x.text.clone())
                    .collect::<Option<Vec<String>>>()
                    .map(|t| t.join("\n")),
                setup: None,
            }),
        }
    }

    /// An op's quality, if measured.
    pub fn quality(&self, op: &str) -> Option<&forge_cam_quality::OpQ> {
        self.quality.as_ref()?.ops.iter().find(|q| q.name == op)
    }

    pub fn op(&self, name: &str) -> Option<&OpRes> {
        self.ops.iter().find(|o| o.name == name)
    }
    pub fn setup(&self, name: &str) -> Option<&SetupRes> {
        self.setups.iter().find(|o| o.name == name)
    }
}

/// The ops of `now` whose result is out of date against `ran` (the statements a run used):
/// an op is stale when it, any statement it is built on (part, machine, tool, setup, fixture,
/// shared values), or an earlier op of its setup changed. A new op is stale until generated.
pub fn stale(ran: &[Stmt], now: &[Stmt]) -> Vec<String> {
    let same = |s: &Stmt| ran.iter().any(|r| r == s);
    let mut out = vec![];
    let mut base_changed = false;
    let mut setup_changed: Vec<String> = vec![];
    for s in now {
        if s.suppressed {
            continue;
        }
        match &s.role {
            Role::Op(_) => {
                let setup = s.setup.clone().unwrap_or_default();
                if base_changed || setup_changed.contains(&setup) || !same(s) {
                    setup_changed.push(setup);
                    out.push(s.name.clone());
                }
            }
            Role::Setup => {
                if !same(s) {
                    setup_changed.push(s.name.clone());
                }
            }
            Role::Program => {}
            _ => {
                if !same(s) {
                    base_changed = true;
                }
            }
        }
    }
    // a statement taken out (or suppressed) changes what every later op starts from; one
    // already suppressed in the run changed nothing
    for r in ran {
        if !r.suppressed && !now.iter().any(|s| s.name == r.name && !s.suppressed) && r.is_op() {
            let su = r.setup.clone().unwrap_or_default();
            let mut after = false;
            for s in now {
                if s.name == r.name {
                    after = true;
                }
                if s.is_op() && s.setup.as_deref() == Some(su.as_str()) && after && !out.contains(&s.name) {
                    out.push(s.name.clone());
                }
            }
            // where it was: every op of its setup after its position in the run
            let pos = ran.iter().position(|x| x.name == r.name).unwrap_or(0);
            for x in &ran[pos..] {
                if x.is_op()
                    && x.setup == r.setup
                    && now.iter().any(|s| s.name == x.name && !s.suppressed)
                    && !out.contains(&x.name)
                {
                    out.push(x.name.clone());
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flange() -> (String, std::path::PathBuf) {
        let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
        (
            std::fs::read_to_string(dir.join("flange_cam.fg")).expect("the example"),
            dir,
        )
    }

    #[test]
    fn the_flange_generates_and_every_op_is_certified() {
        let (text, dir) = flange();
        let r = run(&text, &dir);
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        let names: Vec<&str> = r.ops.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, ["skim", "top", "prof", "bore", "bolts"]);
        assert!(
            r.ops.iter().all(|o| o.status == "built"),
            "{:?}",
            r.ops
                .iter()
                .map(|o| (&o.name, &o.status, &o.refusal))
                .collect::<Vec<_>>()
        );
        assert_eq!(r.setups.len(), 1);
        assert!(r.setups[0].stock_max[2] > r.setups[0].stock_min[2]);
        let p = &r.programs[0];
        assert!(
            p.round_trip && p.text.as_ref().is_some_and(|t| t.lines().count() == p.lines),
            "{p:?}"
        );
        assert!(r.ops[0].tool_t == Some(1) && r.ops[0].rpm.is_some());
        assert!(r.ops.iter().all(|o| o.moves > 0 && o.time_s > 0.0));
        assert!(r.sheet.contains("<html>"));
        // nothing is stale against the text it ran on
        assert!(stale(&r.stmts, &r.stmts).is_empty());
    }

    #[test]
    fn an_edit_makes_that_op_and_later_ones_of_its_setup_stale() {
        let (text, _) = flange();
        let was = doc::read(&text).unwrap();
        let t = doc::apply(
            &text,
            &doc::Edit::Set {
                id: "prof".into(),
                arg: "step".into(),
                text: "3".into(),
            },
        )
        .unwrap();
        assert_eq!(stale(&was, &doc::read(&t).unwrap()), ["prof", "bore", "bolts"]);
        // a tool changed: every op after it
        let t = doc::apply(
            &text,
            &doc::Edit::Set {
                id: "t1".into(),
                arg: "stickout".into(),
                text: "40".into(),
            },
        )
        .unwrap();
        assert_eq!(
            stale(&was, &doc::read(&t).unwrap()),
            ["skim", "top", "prof", "bore", "bolts"]
        );
    }

    #[test]
    fn check_now_checks_the_posted_paths_again() {
        let (text, dir) = flange();
        let r = recheck(run(&text, &dir));
        assert!(
            r.ops
                .iter()
                .all(|o| o.status == "built" && o.assurance == Assurance::CheckedOnDemand),
            "{:?}",
            r.ops
                .iter()
                .map(|o| (&o.name, &o.status, o.assurance))
                .collect::<Vec<_>>()
        );
        assert!(r.ops.iter().all(|o| o.pairs > 0 || o.kind == "drill"));
    }

    #[test]
    fn the_shaft_turns_and_every_op_is_certified() {
        let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
        let text = std::fs::read_to_string(dir.join("shaft_turn.fg")).expect("the example");
        let r = run(&text, &dir);
        assert_eq!(r.process, "turning");
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        let names: Vec<(&str, &str)> = r
            .ops
            .iter()
            .map(|o| (o.name.as_str(), o.status.as_str()))
            .collect();
        assert_eq!(
            names,
            [
                ("face", "built"),
                ("rough", "built"),
                ("fin", "built"),
                ("thr", "built"),
                ("cut", "built")
            ]
        );
        assert_eq!(r.certification, "full");
        assert!(
            r.programs[0]
                .text
                .as_ref()
                .is_some_and(|t| t.lines().count() > 20)
        );
        assert_eq!(r.ops[0].tool.as_deref(), Some("od-rough-cnmg"));
        assert_eq!(r.ops[0].tool_t, Some(1));
        assert_eq!(r.setups[0].name, "op1");
        assert!(stale(&r.stmts, &r.stmts).is_empty());
    }

    #[test]
    fn a_refusal_comes_back_with_its_check_and_fix() {
        let (text, dir) = flange();
        // a tool too short for the bore: its shank goes into the stock
        let t = text.replace(
            "t1   = tool(\"flat-10\", t: 1, stickout: 32",
            "t1   = tool(\"flat-10\", t: 1, stickout: 12",
        );
        assert_ne!(t, text);
        let r = run(&t, &dir);
        let bad: Vec<&OpRes> = r.ops.iter().filter(|o| o.status == "refused").collect();
        assert!(
            !bad.is_empty(),
            "{:?}",
            r.ops.iter().map(|o| (&o.name, &o.status)).collect::<Vec<_>>()
        );
        assert!(bad.iter().any(|o| o.refusal.is_some()));
        assert_eq!(r.certification, "refused");
    }
}

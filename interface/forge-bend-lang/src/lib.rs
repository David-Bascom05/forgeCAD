//! The bending document (research/bendsim/environment.md §8): a press-brake program as text, the
//! Bend Simulation environment's source of truth. Its statements, a closed namespace as a
//! machining document's:
//!
//! ```text
//! bending
//! part   = part("channel.fg", sheet: "base")
//! press  = press("trumpf-trubend-5130")         # or tooling("std-eu"), a stand-in set
//! s1     = station(punch: "trumpf/OW200-S", die: "trumpf/EV_W16-84deg", y: 0, length: 500)
//! b1     = bend("right", at: s1)
//! b2     = bend("left", at: s1, rotate: true)
//! sheet  = setup_sheet()
//! prog   = program(format: "delem")             # typed ProgramFormatUnavailable
//! ```
//!
//! Steps run in statement order: reordering is moving `bend` lines (forge-lang's MoveFeature);
//! every other edit is SetArg, AddFeature, DeleteFeature or ReplaceFeature on these statements,
//! so the file's own undo log covers them. A `bend` without `at:` has its tooling chosen by the
//! planner; one with it is simulated as stated, failing or not.
use forge_agent::json::Json;

use forge_lang::bendsim::{Ask, Sim, StationAsk, StepAsk};
use forge_lang::syntax::{self, Expr, ExprKind, StmtKind};
use std::collections::BTreeMap;
use std::path::Path;

pub const SCHEMA: &str = "forgecad.bending/1";

/// The document's built-ins (a closed namespace: looked up among these only).
pub const NAMES: &[&str] = &[
    "part",
    "press",
    "tooling",
    "station",
    "bend",
    "setup_sheet",
    "program",
];

/// Whether a source is a bending document: its first statement the bare name `bending`.
pub fn is_bending(src: &str) -> bool {
    match syntax::parse(src) {
        Ok(p) => matches!(
            p.first().map(|s| &s.kind),
            Some(StmtKind::Expr(Expr { kind: ExprKind::Name(n), .. })) if n == "bending"
        ),
        Err(_) => false,
    }
}

/// A `bend` statement as read: its name in the document, the bend, its station, orientation.
#[derive(Clone, Debug, PartialEq)]
pub struct BendStmt {
    pub id: String,
    pub bend: String,
    pub station: Option<String>,
    pub rotate: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StationStmt {
    pub id: String,
    pub ask: StationAsk,
}

/// The document read: what it asks, before simulating.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Doc {
    pub part: Option<(String, Option<String>)>,
    pub press: Option<String>,
    pub tooling: Option<String>,
    pub stations: Vec<StationStmt>,
    pub bends: Vec<BendStmt>,
    pub setup_sheet: bool,
    /// `program(format: ..)` asked for, with the format.
    pub program: Option<String>,
}

/// The document's answer.
pub struct Bending {
    pub ok: bool,
    pub errors: Vec<(String, String)>,
    pub doc: Doc,
    pub sim: Option<Sim>,
}

#[derive(Clone, Debug)]
enum V {
    Num(f64),
    Str(String),
    Bool(bool),
    Station(String),
}

fn eval(e: &Expr, env: &BTreeMap<String, V>) -> Result<V, String> {
    Ok(match &e.kind {
        ExprKind::Num(x, _) => V::Num(*x),
        ExprKind::Str(s) => V::Str(s.clone()),
        ExprKind::Bool(b) => V::Bool(*b),
        ExprKind::Name(n) => env.get(n).cloned().ok_or_else(|| format!("unknown name `{n}`"))?,
        ExprKind::Neg(x) => match eval(x, env)? {
            V::Num(v) => V::Num(-v),
            _ => return Err("`-` of a non-number".into()),
        },
        ExprKind::Bin(op, a, b) => {
            let (V::Num(a), V::Num(b)) = (eval(a, env)?, eval(b, env)?) else {
                return Err("arithmetic on a non-number".into());
            };
            V::Num(match op {
                syntax::BinOp::Add => a + b,
                syntax::BinOp::Sub => a - b,
                syntax::BinOp::Mul => a * b,
                syntax::BinOp::Div => a / b,
                _ => return Err("only + - * / in a bending document".into()),
            })
        }
        _ => return Err("a value this document does not take".into()),
    })
}

/// Reads a bending document's statements (no file is read, nothing simulated).
pub fn read(src: &str) -> Result<Doc, Vec<(String, String)>> {
    let prog = syntax::parse(src).map_err(|e| vec![("document".to_string(), format!("{e:?}"))])?;
    if !is_bending(src) {
        return Err(vec![(
            "document".into(),
            "not a bending document: its first statement must be `bending`".into(),
        )]);
    }
    let mut doc = Doc::default();
    let mut errors = Vec::new();
    let mut env: BTreeMap<String, V> = BTreeMap::new();
    for st in prog.iter().skip(1) {
        let r: Result<(), (String, String)> = (|| match &st.kind {
            StmtKind::Shared { name, value } => {
                let v = eval(value, &env).map_err(|e| (name.clone(), e))?;
                env.insert(name.clone(), v);
                Ok(())
            }
            StmtKind::Assign { name, value } => {
                let fail = |e: String| (name.clone(), e);
                let ExprKind::Call(f, args) = &value.kind else {
                    let v = eval(value, &env).map_err(fail)?;
                    env.insert(name.clone(), v);
                    return Ok(());
                };
                let ExprKind::Name(fname) = &f.kind else {
                    return Err(fail("a call of a non-built-in".into()));
                };
                let mut pos = Vec::new();
                let mut named: BTreeMap<String, V> = BTreeMap::new();
                for a in args {
                    let v = eval(&a.value, &env).map_err(fail)?;
                    match &a.name {
                        Some((k, _)) => {
                            named.insert(k.clone(), v);
                        }
                        None => pos.push(v),
                    }
                }
                let text = |v: Option<&V>, what: &str| match v {
                    Some(V::Str(s)) => Ok(s.clone()),
                    _ => Err(fail(format!("{fname}: {what} takes a string"))),
                };
                let num = |v: Option<&V>, what: &str| match v {
                    Some(V::Num(x)) if x.is_finite() => Ok(*x),
                    _ => Err(fail(format!("{fname}: {what} takes a number (mm)"))),
                };
                let known = |allowed: &[&str]| -> Result<(), (String, String)> {
                    match named.keys().find(|k| !allowed.contains(&k.as_str())) {
                        Some(k) => Err(fail(format!(
                            "{fname} takes no `{k}:` (it takes {})",
                            allowed.join(", ")
                        ))),
                        None => Ok(()),
                    }
                };
                match fname.as_str() {
                    "part" => {
                        known(&["sheet"])?;
                        let p = text(pos.first(), "the part's file")?;
                        let sheet = named.get("sheet").map(|v| text(Some(v), "sheet")).transpose()?;
                        doc.part = Some((p, sheet));
                    }
                    "press" => {
                        known(&[])?;
                        doc.press = Some(text(pos.first(), "the press's library id")?);
                    }
                    "tooling" => {
                        known(&[])?;
                        doc.tooling = Some(text(pos.first(), "a stand-in set's id")?);
                    }
                    "station" => {
                        known(&["punch", "die", "y", "length", "punch_length", "die_length"])?;
                        let length = named.get("length").map(|v| num(Some(v), "length")).transpose()?;
                        let pl = match named.get("punch_length") {
                            Some(v) => num(Some(v), "punch_length")?,
                            None => length.ok_or_else(|| {
                                fail("station: length (or punch_length and die_length)".into())
                            })?,
                        };
                        let dl = match named.get("die_length") {
                            Some(v) => num(Some(v), "die_length")?,
                            None => length.ok_or_else(|| {
                                fail("station: length (or punch_length and die_length)".into())
                            })?,
                        };
                        doc.stations.push(StationStmt {
                            id: name.clone(),
                            ask: StationAsk {
                                punch: text(named.get("punch"), "punch")?,
                                die: text(named.get("die"), "die")?,
                                y: named
                                    .get("y")
                                    .map(|v| num(Some(v), "y"))
                                    .transpose()?
                                    .unwrap_or(0.0),
                                punch_length: pl,
                                die_length: dl,
                            },
                        });
                        env.insert(name.clone(), V::Station(name.clone()));
                    }
                    "bend" => {
                        known(&["at", "rotate"])?;
                        let bend = match pos.first() {
                            Some(V::Str(s)) => s.clone(),
                            _ => return Err(fail("bend: the bend's name (a string)".into())),
                        };
                        if pos.len() > 1 {
                            return Err(fail(
                                "bend: one bend per step (two in one stroke are not simulated)".into(),
                            ));
                        }
                        let station = match named.get("at") {
                            None => None,
                            Some(V::Station(s)) => Some(s.clone()),
                            Some(_) => return Err(fail("bend: at: takes a station".into())),
                        };
                        let rotate = match named.get("rotate") {
                            None => None,
                            Some(V::Bool(b)) => Some(*b),
                            Some(_) => return Err(fail("bend: rotate: takes true or false".into())),
                        };
                        doc.bends.push(BendStmt {
                            id: name.clone(),
                            bend,
                            station,
                            rotate,
                        });
                    }
                    "setup_sheet" => {
                        known(&[])?;
                        doc.setup_sheet = true;
                    }
                    "program" => {
                        known(&["format"])?;
                        doc.program = Some(text(named.get("format"), "format")?);
                    }
                    other => {
                        return Err(fail(format!(
                            "`{other}` is not a bending document's built-in ({})",
                            NAMES.join(", ")
                        )));
                    }
                }
                Ok(())
            }
            StmtKind::Expr(_) => Ok(()),
            _ => Err((
                "document".into(),
                "a statement this document does not take".into(),
            )),
        })();
        if let Err(e) = r {
            errors.push(e);
        }
    }
    if doc.part.is_none() {
        errors.push(("document".into(), "no part(\"FILE.fg\")".into()));
    }
    if errors.is_empty() { Ok(doc) } else { Err(errors) }
}

impl Doc {
    /// What the document asks of the simulation.
    pub fn ask(&self) -> Ask {
        let index = |id: &str| self.stations.iter().position(|s| s.id == id);
        Ask {
            press: self.press.clone(),
            tooling: self.tooling.clone(),
            sheet: self.part.as_ref().and_then(|p| p.1.clone()),
            sequence: (!self.bends.is_empty()).then(|| {
                self.bends
                    .iter()
                    .map(|b| b.bend.clone())
                    .collect::<Vec<_>>()
                    .join(",")
            }),
            stations: self.stations.iter().map(|s| s.ask.clone()).collect(),
            steps: self
                .bends
                .iter()
                .map(|b| StepAsk {
                    station: b.station.as_deref().and_then(index),
                    rotate: b.rotate,
                })
                .collect(),
            keep_tools: true,
            ..Ask::default()
        }
    }
}

/// The document read and simulated; the part's file beside it.
pub fn run(src: &str, dir: &Path) -> Bending {
    let doc = match read(src) {
        Ok(d) => d,
        Err(errors) => {
            return Bending {
                ok: false,
                errors,
                doc: Doc::default(),
                sim: None,
            };
        }
    };
    let (file, _) = doc.part.clone().unwrap();
    let path = dir.join(&file);
    let part = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            return Bending {
                ok: false,
                errors: vec![("part".into(), format!("{}: {e}", path.display()))],
                doc,
                sim: None,
            };
        }
    };
    let pdir = path.parent().map(Path::to_path_buf);
    let mut errors = Vec::new();
    if let Some(f) = &doc.program {
        errors.push((
            "program".into(),
            format!(
                "ProgramFormatUnavailable: no openly published specification of the `{f}` press-brake \
                 program format was found (research/bendsim/environment.md §8); the setup sheet is the output"
            ),
        ));
    }
    match forge_lang::bendsim::simulate(&part, pdir, None, &doc.ask()) {
        Ok(sim) => Bending {
            ok: errors.is_empty(),
            errors,
            doc,
            sim: Some(sim),
        },
        Err(a) => {
            errors.push(("simulation".into(), a.text));
            Bending {
                ok: false,
                errors,
                doc,
                sim: None,
            }
        }
    }
}

impl Bending {
    /// `forgecad.bending/1`: the document's statements, then the simulation's reply
    /// (`forgecad.bendsim/1`) whole, each step tied to its `bend` statement.
    pub fn json(&self) -> Json {
        let errs = Json::Arr(
            self.errors
                .iter()
                .map(|(at, text)| {
                    let kind = if text.starts_with("ProgramFormatUnavailable") {
                        "ProgramFormatUnavailable"
                    } else {
                        "BendingError"
                    };
                    Json::obj(vec![
                        ("at", at.as_str().into()),
                        ("kind", kind.into()),
                        ("text", text.as_str().into()),
                    ])
                })
                .collect(),
        );
        let stmts = Json::Arr(
            self.doc
                .bends
                .iter()
                .enumerate()
                .map(|(i, b)| {
                    Json::obj(vec![
                        ("n", (i + 1).into()),
                        ("id", b.id.as_str().into()),
                        ("bend", b.bend.as_str().into()),
                        ("station", b.station.as_deref().map_or(Json::Null, Json::from)),
                        ("rotate", b.rotate.map_or(Json::Null, Json::from)),
                    ])
                })
                .collect(),
        );
        Json::obj(vec![
            ("ok", self.ok.into()),
            ("schema", SCHEMA.into()),
            ("errors", errs),
            ("steps", stmts),
            (
                "simulation",
                self.sim.as_ref().map_or(Json::Null, |s| s.answer.json.clone()),
            ),
        ])
    }

    /// The setup sheet: the press, the tool layout along the bed, and the steps (HTML).
    pub fn sheet(&self) -> String {
        let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let mut h = String::from(
            "<!doctype html><meta charset=utf-8><title>Bending setup sheet</title>\
             <style>body{font:14px system-ui;margin:24px}table{border-collapse:collapse;margin:8px 0}\
             td,th{border:1px solid #999;padding:4px 8px;text-align:left}.fail{color:#b00}</style>",
        );
        let Some(sim) = &self.sim else {
            h.push_str("<p class=fail>not simulated</p>");
            for (a, t) in &self.errors {
                h.push_str(&format!("<p class=fail>{}: {}</p>", esc(a), esc(t)));
            }
            return h;
        };
        let tl = &sim.tooling;
        h.push_str(&format!(
            "<h1>Bending setup sheet</h1><p>Press: {} — bed {:.0} mm, between housings {:.0} mm</p>",
            esc(&tl.press.id),
            tl.press.bed_length,
            tl.press.housing_gap
        ));
        h.push_str("<h2>Tool layout</h2><table><tr><th>station</th><th>punch</th><th>die</th><th>centre y (mm)</th><th>punch stack (mm)</th><th>die stack (mm)</th></tr>");
        for (i, s) in tl.stations.iter().enumerate() {
            h.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.1}</td><td>{:.1}</td><td>{:.1}</td></tr>",
                self.doc
                    .stations
                    .get(i)
                    .map_or(format!("{}", i + 1), |x| esc(&x.id)),
                esc(&tl.punches[s.punch].id),
                esc(&tl.dies[s.die].id),
                s.y,
                s.punch_length,
                s.die_length
            ));
        }
        h.push_str("</table><h2>Steps</h2><table><tr><th>#</th><th>bend</th><th>angle</th><th>punch</th><th>die</th><th>orientation</th><th>backgauge X / R</th><th>checks</th></tr>");
        for (i, st) in sim.plan.steps.iter().enumerate() {
            let fails: Vec<String> = st.failures().iter().map(|c| c.rule.word().to_string()).collect();
            h.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{:.1}°</td><td>{}</td><td>{}</td><td>{}{}</td><td>{}</td><td{}>{}</td></tr>",
                i + 1,
                esc(&sim.names[st.bend]),
                st.angle.to_degrees(),
                esc(&st.punch),
                esc(&st.die),
                if st.flip { "flipped" } else { "as is" },
                if st.setup.rotate { ", end for end" } else { "" },
                st.gauge
                    .as_ref()
                    .map_or("not checked".to_string(), |g| format!("{:.1} / {:.1}", g.x, g.r)),
                if fails.is_empty() { "" } else { " class=fail" },
                if fails.is_empty() { "pass".to_string() } else { fails.join(", ") }
            ));
        }
        h.push_str("</table>");
        for (a, t) in &self.errors {
            h.push_str(&format!("<p class=fail>{}: {}</p>", esc(a), esc(t)));
        }
        h
    }
}

/// A bending document for a simulated program: the part, the press (or stand-in set), one
/// station per distinct punch and die (side by side from the bed's centre), and each step's
/// bend at its station with its orientation. What the environment writes on the first edit.
pub fn document(part: &str, sheet: Option<&str>, press: Option<&str>, standin: &str, sim: &Sim) -> String {
    let tl = &sim.tooling;
    let mut out = String::from("bending\n");
    out.push_str(&match sheet {
        Some(s) => format!("part = part({part:?}, sheet: {s:?})\n"),
        None => format!("part = part({part:?})\n"),
    });
    out.push_str(&match press {
        Some(p) => format!("press = press({p:?})\n"),
        None => format!("press = tooling({standin:?})\n"),
    });
    let mut pairs: Vec<(usize, usize, f64, f64)> = Vec::new();
    for st in &sim.plan.steps {
        let (pl, dl) = (st.punch_length.unwrap_or(0.0), st.die_length.unwrap_or(0.0));
        match pairs
            .iter_mut()
            .find(|p| (p.0, p.1) == (st.setup.punch, st.setup.die))
        {
            Some(p) => {
                p.2 = p.2.max(pl);
                p.3 = p.3.max(dl);
            }
            None => pairs.push((st.setup.punch, st.setup.die, pl, dl)),
        }
    }
    // side by side, each its stack plus 50 mm apart, centred on the bed
    let widths: Vec<f64> = pairs.iter().map(|p| p.2.max(p.3).max(1.0)).collect();
    let total: f64 = widths.iter().sum::<f64>() + 50.0 * (widths.len().saturating_sub(1)) as f64;
    let mut at = -0.5 * total;
    for (i, (p, d, pl, dl)) in pairs.iter().enumerate() {
        let y = at + 0.5 * widths[i];
        at += widths[i] + 50.0;
        out.push_str(&format!(
            "s{} = station(punch: {:?}, die: {:?}, y: {}, punch_length: {}, die_length: {})\n",
            i + 1,
            tl.punches[*p].id,
            tl.dies[*d].id,
            num(y),
            num(pl.max(1.0)),
            num(dl.max(1.0))
        ));
    }
    for (i, st) in sim.plan.steps.iter().enumerate() {
        let k = pairs
            .iter()
            .position(|p| (p.0, p.1) == (st.setup.punch, st.setup.die))
            .unwrap();
        out.push_str(&format!(
            "b{} = bend({:?}, at: s{}{})\n",
            i + 1,
            sim.names[st.bend],
            k + 1,
            if st.setup.rotate { ", rotate: true" } else { "" }
        ));
    }
    out.push_str("sheet = setup_sheet()\n");
    out
}

fn num(x: f64) -> String {
    let r = (x * 10.0).round() / 10.0;
    if r == r.trunc() {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

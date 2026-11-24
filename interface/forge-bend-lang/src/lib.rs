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

/// Press-brake program formats written: those whose specification is openly published
/// (research/bendsim/env/data/program-formats.md). `cincinnati`: Cincinnati Incorporated's XML
/// program (`*.pgm`), EM-503 (R-09/09) "XML FILE FORMAT" (research/bendsim/env/data/
/// cincinnati-xml.md).
pub const FORMATS: &[&str] = &["cincinnati"];

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

/// Each built-in's arguments: its keywords, and its positionals as `#0`, `#1`, ..; a call giving
/// any other is refused (`UnknownArg`, with the nearest listed).
pub const ARGS: &[(&str, &[&str])] = &[
    ("part", &["#0", "sheet"]),
    ("press", &["#0"]),
    ("tooling", &["#0"]),
    (
        "station",
        &["punch", "die", "y", "length", "punch_length", "die_length"],
    ),
    ("bend", &["#0", "at", "rotate"]),
    ("setup_sheet", &[]),
    ("program", &["format"]),
];

fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

/// The arguments a call gives against its built-in's list: `UnknownArg` for one not listed.
fn check_args(f: &str, pos: usize, named: &[String]) -> Result<(), String> {
    let allowed = ARGS.iter().find(|a| a.0 == f).map_or(&[][..], |a| a.1);
    let given = (0..pos).map(|k| format!("#{k}")).chain(named.iter().cloned());
    for g in given {
        if !allowed.contains(&g.as_str()) {
            let near = allowed
                .iter()
                .filter(|a| !a.starts_with('#'))
                .min_by_key(|a| edit_distance(a, &g))
                .map_or(String::new(), |a| format!("; nearest `{a}`"));
            let what = if g.starts_with('#') {
                format!("a positional argument {g}")
            } else {
                format!("`{g}:`")
            };
            return Err(format!(
                "UnknownArg: {f} takes no {what} (it takes {}){near}",
                allowed.join(", ")
            ));
        }
    }
    Ok(())
}

/// The statements read so far, and the names they bind.
struct Reader {
    doc: Doc,
    env: BTreeMap<String, V>,
}

impl Reader {
    /// One statement's call of a built-in (a closed namespace: the head is looked up among the
    /// built-ins only; its arguments are read among the bindings only).
    fn call(&mut self, name: &str, f: &str, pos: Vec<V>, named: BTreeMap<String, V>) -> Result<(), String> {
        let keys: Vec<String> = named.keys().cloned().collect();
        let text = |v: Option<&V>, what: &str| match v {
            Some(V::Str(s)) => Ok(s.clone()),
            _ => Err(format!("{f}: {what} takes a string")),
        };
        let num = |v: Option<&V>, what: &str| match v {
            Some(V::Num(x)) if x.is_finite() => Ok(*x),
            _ => Err(format!("{f}: {what} takes a number (mm)")),
        };
        if NAMES.contains(&f) {
            check_args(f, pos.len(), &keys)?;
        }
        match f {
            "part" => {
                let p = text(pos.first(), "the part's file")?;
                let sheet = named.get("sheet").map(|v| text(Some(v), "sheet")).transpose()?;
                self.doc.part = Some((p, sheet));
            }
            "press" => self.doc.press = Some(text(pos.first(), "the press's library id")?),
            "tooling" => self.doc.tooling = Some(text(pos.first(), "a stand-in set's id")?),
            "station" => {
                let length = named.get("length").map(|v| num(Some(v), "length")).transpose()?;
                let need = || "station: length (or punch_length and die_length)".to_string();
                let pl = match named.get("punch_length") {
                    Some(v) => num(Some(v), "punch_length")?,
                    None => length.ok_or_else(need)?,
                };
                let dl = match named.get("die_length") {
                    Some(v) => num(Some(v), "die_length")?,
                    None => length.ok_or_else(need)?,
                };
                self.doc.stations.push(StationStmt {
                    id: name.into(),
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
                self.env.insert(name.into(), V::Station(name.into()));
            }
            "bend" => {
                let bend = match pos.first() {
                    Some(V::Str(s)) => s.clone(),
                    _ => return Err("bend: the bend's name (a string)".into()),
                };
                let station = match named.get("at") {
                    None => None,
                    Some(V::Station(s)) => Some(s.clone()),
                    Some(_) => return Err("bend: at: takes a station".into()),
                };
                let rotate = match named.get("rotate") {
                    None => None,
                    Some(V::Bool(b)) => Some(*b),
                    Some(_) => return Err("bend: rotate: takes true or false".into()),
                };
                self.doc.bends.push(BendStmt {
                    id: name.into(),
                    bend,
                    station,
                    rotate,
                });
            }
            "setup_sheet" => self.doc.setup_sheet = true,
            "program" => self.doc.program = Some(text(named.get("format"), "format")?),
            other => {
                return Err(format!(
                    "`{other}` is not a bending document's built-in ({})",
                    NAMES.join(", ")
                ));
            }
        }
        Ok(())
    }
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
    let mut r = Reader {
        doc: Doc::default(),
        env: BTreeMap::new(),
    };
    let mut errors = Vec::new();
    for st in prog.iter().skip(1) {
        let res: Result<(), (String, String)> = match &st.kind {
            StmtKind::Shared { name, value } => match eval(value, &r.env) {
                Ok(v) => {
                    r.env.insert(name.clone(), v);
                    Ok(())
                }
                Err(e) => Err((name.clone(), e)),
            },
            StmtKind::Assign { name, value } => (|| {
                let fail = |e: String| (name.clone(), e);
                let ExprKind::Call(f, args) = &value.kind else {
                    let v = eval(value, &r.env).map_err(fail)?;
                    r.env.insert(name.clone(), v);
                    return Ok(());
                };
                let ExprKind::Name(fname) = &f.kind else {
                    return Err(fail("a call of a non-built-in".into()));
                };
                let mut pos = Vec::new();
                let mut named: BTreeMap<String, V> = BTreeMap::new();
                for a in args {
                    let v = eval(&a.value, &r.env).map_err(fail)?;
                    match &a.name {
                        Some((k, _)) => {
                            named.insert(k.clone(), v);
                        }
                        None => pos.push(v),
                    }
                }
                r.call(name, fname, pos, named).map_err(fail)
            })(),
            StmtKind::Expr(_) => Ok(()),
            _ => Err((
                "document".into(),
                "a statement this document does not take".into(),
            )),
        };
        if let Err(e) = res {
            errors.push(e);
        }
    }
    let doc = r.doc;
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

/// The document read and simulated; the part's file beside it (read through forge-dirs: the
/// browser's files too).
pub fn run(src: &str, dir: &Path) -> Bending {
    let file = match read(src) {
        Ok(d) => d.part.map(|p| p.0),
        Err(errors) => {
            return Bending {
                ok: false,
                errors,
                doc: Doc::default(),
                sim: None,
            };
        }
    };
    let path = dir.join(file.unwrap_or_default());
    match forge_dirs::fs::read_to_string(&path) {
        Ok(part) => run_with(src, &part, path.parent()),
        Err(e) => Bending {
            ok: false,
            errors: vec![("part".into(), format!("{}: {e}", path.display()))],
            doc: read(src).unwrap_or_default(),
            sim: None,
        },
    }
}

/// The document simulated on the part's text already in hand (its own files beside `pdir`).
pub fn run_with(src: &str, part: &str, pdir: Option<&Path>) -> Bending {
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
    let mut errors = Vec::new();
    if let Some(f) = doc.program.as_deref().filter(|f| !FORMATS.contains(f)) {
        errors.push((
            "program".into(),
            format!(
                "ProgramFormatUnavailable: no openly published specification of the `{f}` press-brake \
                 program format was found (research/bendsim/env/data/program-formats.md); the formats \
                 written are: {}",
                FORMATS.join(", ")
            ),
        ));
    }
    match forge_lang::bendsim::simulate(part, pdir.map(Path::to_path_buf), None, &doc.ask()) {
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

/// The companion document's path for a part: `<stem>_bend.fg` beside it.
pub fn companion(part: &Path) -> std::path::PathBuf {
    let stem = part
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    part.with_file_name(format!("{stem}_bend.fg"))
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

    /// The press-brake program in the format the document asks for (`program(format: ..)`):
    /// the file's text and extension, and what it leaves to the control (said, not guessed).
    pub fn program_text(&self) -> Option<Result<Program, String>> {
        let f = self.doc.program.as_deref()?;
        let sim = match &self.sim {
            Some(s) => s,
            None => return Some(Err("not simulated".into())),
        };
        Some(match f {
            "cincinnati" => Ok(cincinnati(sim, &self.doc)).map(|(t, n)| (t, "pgm", n)),
            other => Err(format!("ProgramFormatUnavailable: `{other}`")),
        })
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

/// A written program: its text, its file extension, and what it leaves to the control.
pub type Program = (String, &'static str, Vec<String>);

/// A number in inches as the format stores it: no trailing zeros (EM-503 p. 27), 4 decimals.
fn inch(mm: f64) -> String {
    let v = (mm / 25.4 * 10_000.0).round() / 10_000.0;
    let mut t = format!("{v:.4}");
    while t.contains('.') && (t.ends_with('0') || t.ends_with('.')) {
        t.pop();
    }
    if t == "-0" { "0".into() } else { t }
}

fn deg(x: f64) -> String {
    let v = (x * 10_000.0).round() / 10_000.0;
    let mut t = format!("{v:.4}");
    while t.contains('.') && (t.ends_with('0') || t.ends_with('.')) {
        t.pop();
    }
    t
}

fn attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Cincinnati's material code (EM-503 pp. 29-30) for a forge-material name: only where the
/// family is plain (mild steel 0, stainless 2, aluminium: soft 3 / hard 4 by temper, copper 5);
/// anything else is left to the control (said in the notes).
fn mat_type(material: Option<&str>) -> Option<u32> {
    let m = material?.to_ascii_lowercase();
    if m.contains("stainless") || m.starts_with("30") || m.starts_with("31") {
        Some(2)
    } else if m.starts_with("50")
        || m.starts_with("60")
        || m.starts_with("1100")
        || m.starts_with("3003")
        || m.contains("alumin")
    {
        // soft: O and H1x/H3x of 1xxx/3xxx/5xxx; hard: T tempers
        Some(if m.contains("-t") { 4 } else { 3 })
    } else if m.contains("copper") || m.starts_with("c1") {
        Some(5)
    } else if m.contains("mild")
        || m.contains("1008")
        || m.contains("1010")
        || m.contains("a36")
        || m == "steel"
    {
        Some(0)
    } else {
        None
    }
}

/// The program as Cincinnati's XML (`<Document><PressBrakeProgram>`), from the simulated steps:
/// each station a ToolSetup upper and lower (ID by station, Location from the bed's centre,
/// plus to the right seen from the front; its segment stack where the tool's lengths make it);
/// each step in angle mode at its included angle, its bend length, its tools, the X gauge (as the
/// flange dimension, EM-503 p. 26) where the backgauge was checked. Everything else is left
/// out, so the control's defaults hold (p. 26); the notes say what was left and why.
pub fn cincinnati(sim: &Sim, doc: &Doc) -> (String, Vec<String>) {
    let tl = &sim.tooling;
    let mut notes = vec![
        "values in inches, angles in degrees (EM-503 p. 27)".to_string(),
        "ToolSetup Name is forgecad's tool id: the control's Tool Library must hold tools of these names"
            .to_string(),
    ];
    // the stations: the document's, else one per distinct punch and die of the steps, centred
    let stations: Vec<(usize, usize, f64, f64, f64)> = if tl.stations.is_empty() {
        let mut v: Vec<(usize, usize, f64, f64, f64)> = Vec::new();
        for st in &sim.plan.steps {
            if !v.iter().any(|x| (x.0, x.1) == (st.setup.punch, st.setup.die)) {
                v.push((
                    st.setup.punch,
                    st.setup.die,
                    0.0,
                    st.punch_length.unwrap_or(0.0),
                    st.die_length.unwrap_or(0.0),
                ));
            }
        }
        if v.len() > 1 {
            notes.push("no stations stated: every tool pair at the bed's centre (one at a time)".into());
        }
        v
    } else {
        tl.stations
            .iter()
            .map(|s| (s.punch, s.die, s.y, s.punch_length, s.die_length))
            .collect()
    };
    let mut x = String::from("<?xml version=\"1.0\" ?>\n<Document>");
    let t = sim.model.t;
    x.push_str(&format!(
        "<PressBrakeProgram Version=\"9\" MatThick=\"{}\"",
        inch(t)
    ));
    let material = sim
        .answer
        .json
        .get("material")
        .and_then(|m| m.str().map(str::to_string));
    match mat_type(material.as_deref()) {
        Some(m) => x.push_str(&format!(" MatType=\"{m}\"")),
        None => notes.push(format!(
            "MatType left to the control (default 0, mild steel): {} has no plain Cincinnati code",
            material.as_deref().unwrap_or("the material")
        )),
    }
    x.push_str(&format!(" NumSteps=\"{}\">", sim.plan.steps.len()));
    for (upper, list) in [(true, "UpperToolSets"), (false, "LowerToolSets")] {
        x.push_str(&format!("<{list}><ToolList Count=\"{}\">", stations.len()));
        for (k, (p, d, y, pl, dl)) in stations.iter().enumerate() {
            let (name, len, segs) = if upper {
                (&tl.punches[*p].id, *pl, &tl.punches[*p].segments)
            } else {
                (&tl.dies[*d].id, *dl, &tl.dies[*d].segments)
            };
            // Location: + right seen from the front; the press's y runs to the operator's left
            x.push_str(&format!(
                "<ToolSetup Name=\"{}\" ID=\"{}\" Length=\"{}\" Location=\"{}\"",
                attr(name),
                k + 1,
                inch(len),
                inch(-y)
            ));
            match forge_brake::tooling::cover_within(segs, len, len).filter(|c| (c.0 - len).abs() < 0.05) {
                Some((_, pieces)) => {
                    let all: Vec<f64> = pieces
                        .iter()
                        .flat_map(|(l, n)| std::iter::repeat_n(*l, *n as usize))
                        .collect();
                    x.push_str(&format!(" NumSegs=\"{}\"><SegStackup>", all.len()));
                    for l in all {
                        x.push_str(&format!("<SegEntry SegValue=\"{}\" />", inch(l)));
                    }
                    x.push_str("</SegStackup></ToolSetup>");
                }
                None => x.push_str(" />"),
            }
        }
        x.push_str(&format!("</ToolList></{list}>"));
    }
    x.push_str("<StepData>");
    let mut no_gauge = Vec::new();
    for (i, st) in sim.plan.steps.iter().enumerate() {
        let k = stations
            .iter()
            .position(|s| (s.0, s.1) == (st.setup.punch, st.setup.die))
            .unwrap_or(0)
            + 1;
        let included = 180.0 - st.angle.to_degrees();
        let len = sim.model.bends[st.bend].len;
        x.push_str(&format!(
            "<Step RevMode=\"0\" RevAngle=\"{}\" BendLen=\"{}\" UpperID=\"{k}\" LowerID=\"{k}\"",
            deg(included),
            inch(len)
        ));
        match &st.gauge {
            Some(g) => {
                x.push_str(&format!(" XLeft=\"{0}\" XRight=\"{0}\"", inch(g.x)));
            }
            None => no_gauge.push(format!("{}", i + 1)),
        }
        x.push_str(&format!(
            "><UIDSelectList Size=\"1\"><UID IntValue=\"{k}\" /></UIDSelectList><LIDSelectList Size=\"1\"><LID IntValue=\"{k}\" /></LIDSelectList><ProgOutputs>"
        ));
        for _ in 0..8 {
            x.push_str("<ProgOutData />");
        }
        x.push_str("</ProgOutputs></Step>");
    }
    x.push_str("</StepData></PressBrakeProgram></Document>\n");
    if !no_gauge.is_empty() {
        notes.push(format!(
            "X gauge left to the control (default: max travel - 1.0 in) at step(s) {}: the press's backgauge ranges are not published, so no stop was checked",
            no_gauge.join(", ")
        ));
    }
    notes.push("orientation (PartOrient), R/Z gauge, speeds and outputs left to the control: their bit meanings and datums are not stated in EM-503".into());
    let _ = doc;
    (x, notes)
}

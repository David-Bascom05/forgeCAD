//! The machining document for lathes: turning in .fg (kernel/forge-turn).
//!
//! A turning document is a machining document (first statement `machining`) whose setup is a
//! `lathe_setup(..)`. `forgecad cam` sends it here (forge-cli: one branch on `is_turning`); the
//! milling document stays in forge-cam-lang. The statements are the language's own, read by the
//! same parser; the calls are turning's built-ins:
//!
//! ```text
//! machining
//! shared bar = 32
//! f     = part("shaft.fg")
//! t1    = lathe_tool("od-rough-cnmg", t: 1)
//! t2    = lathe_tool("od-finish-dnmg", t: 2)
//! jaws  = chuck(face: -70, grip: 30, jaw: 15)
//! op1   = lathe_setup(f, bar: bar, front: 2, chuck: jaws, machine: "generic-lathe",
//!                     at: 100, offsets: ((1, 0, 0), (2, 0, 0)))
//! face  = turn_face(op1, tool: t1, to: 0, step: 1)
//! rough = turn_rough(op1, tool: t1, leave: 0.3, step: 2, to: -56)
//! fin   = turn_finish(op1, tool: t2, to: -55)
//! prog  = lathe_program(op1, post: "fanuc-lathe", number: 1)
//! ```
//!
//! Also `bore_rough(op, tool:, leave:, step:, to:, from: D)` (D the drilled hole's diameter; then
//! `turn_finish` with a boring tool finishes the bore), `turn_groove(op, tool:, from:, to:, r:)`
//! (with an ID grooving bar, also `bore:` the bore's diameter on the way in), `turn_thread(op,
//! tool:, d:, pitch:, from:, to:, passes:)` (an ID thread insert cuts an internal thread),
//! `part_off(op, tool:, at:)`, `turn_drill(op, tool:, to:, peck:)`; `turn_finish` with a
//! left-hand tool finishes up the profile, `from:` Z0 `to:` Z1. Every op is
//! posted, read back (the round trip), and the path the text means is what is certified
//! (forge_turn::ops::certify). The reply is `forgecad.cam/1` with `"process": "turning"`.
use forge_agent::json::Json;
use forge_lang::syntax::{self, Arg, BinOp, Expr, ExprKind, StmtKind};
use forge_turn::insert::{self, TurnTool};
use forge_turn::machine::{self, Place};
use forge_turn::ops::{self, Chuck, OpPath, Report, Setup};
use forge_turn::post;
use std::collections::BTreeMap;
use std::path::Path;

/// Whether a source is a turning document: a machining document with a `lathe_setup` call.
pub fn is_turning(src: &str) -> bool {
    let Ok(p) = syntax::parse(src) else { return false };
    let machining = matches!(p.first().map(|s| &s.kind), Some(StmtKind::Expr(Expr { kind: ExprKind::Name(n), .. })) if n == "machining");
    machining
        && p.iter().any(|s| {
            matches!(&s.kind, StmtKind::Assign { value: Expr { kind: ExprKind::Call(f, _), .. }, .. }
                if matches!(&f.kind, ExprKind::Name(n) if n == "lathe_setup"))
        })
}

#[derive(Clone, Debug)]
enum V {
    Num(f64),
    Str(String),
    Tuple(Vec<V>),
    Part(usize),
    Tool(usize),
    Chuck(Chuck),
    Setup(usize),
    #[allow(dead_code)]
    Op(usize),
}

/// An op of the document, as built.
#[derive(Clone, Debug)]
pub struct OpOut {
    pub name: String,
    pub kind: String,
    /// "built" (certified) or "refused".
    pub status: String,
    pub moves: usize,
    pub refusal: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ProgramOut {
    pub name: String,
    pub post: String,
    pub number: u32,
    pub text: Option<String>,
    pub error: Option<String>,
}

/// The document's result.
pub struct TurnCam {
    pub ok: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub ops: Vec<OpOut>,
    pub programs: Vec<ProgramOut>,
    pub report: Option<Report>,
    /// The exact in-process stock and the G4 leftover (`cam F check`); otherwise the normal
    /// path: the stock as columns, no leftover.
    pub exact: bool,
    parts: Vec<forge_turn::profile::HalfProfile>,
    tools: Vec<(TurnTool, u32)>,
    setups: Vec<(String, Setup)>,
    paths: Vec<(String, usize, OpPath)>,
}

fn named<'a>(a: &'a BTreeMap<String, V>, k: &str) -> Result<&'a V, String> {
    a.get(k).ok_or_else(|| format!("missing `{k}:`"))
}

fn num(a: &BTreeMap<String, V>, k: &str, default: Option<f64>) -> Result<f64, String> {
    match a.get(k) {
        Some(V::Num(x)) => Ok(*x),
        Some(v) => Err(format!("`{k}:` is not a number ({v:?})")),
        None => default.ok_or_else(|| format!("missing `{k}:`")),
    }
}

fn text(a: &BTreeMap<String, V>, k: &str, default: Option<&str>) -> Result<String, String> {
    match a.get(k) {
        Some(V::Str(s)) => Ok(s.clone()),
        Some(v) => Err(format!("`{k}:` is not text ({v:?})")),
        None => default
            .map(str::to_string)
            .ok_or_else(|| format!("missing `{k}:`")),
    }
}

impl TurnCam {
    fn eval(&self, e: &Expr, env: &BTreeMap<String, V>) -> Result<V, String> {
        Ok(match &e.kind {
            ExprKind::Num(x, _) => V::Num(*x),
            ExprKind::Str(s) => V::Str(s.clone()),
            ExprKind::Name(n) => env.get(n).cloned().ok_or_else(|| format!("unknown name `{n}`"))?,
            ExprKind::Neg(x) => match self.eval(x, env)? {
                V::Num(v) => V::Num(-v),
                v => return Err(format!("minus of {v:?}")),
            },
            ExprKind::Tuple(xs) | ExprKind::List(xs) => {
                V::Tuple(xs.iter().map(|x| self.eval(x, env)).collect::<Result<_, _>>()?)
            }
            ExprKind::Bin(op, l, r) => match (self.eval(l, env)?, self.eval(r, env)?) {
                (V::Num(a), V::Num(b)) => V::Num(match op {
                    BinOp::Add => a + b,
                    BinOp::Sub => a - b,
                    BinOp::Mul => a * b,
                    BinOp::Div => a / b,
                    _ => return Err(format!("operator {op:?} in a turning document")),
                }),
                _ => return Err("arithmetic on a non-number".into()),
            },
            ExprKind::Call(f, args) => {
                let ExprKind::Name(fname) = &f.kind else {
                    return Err("a call of a non-built-in".into());
                };
                let a = self.named_args(args, env)?;
                match fname.as_str() {
                    "chuck" => V::Chuck(Chuck {
                        z_face: num(&a, "face", None)?,
                        grip: num(&a, "grip", None)?,
                        jaw_height: num(&a, "jaw", None)?,
                        body_r: num(&a, "body_r", Some(80.0))?,
                        body_len: num(&a, "body_len", Some(60.0))?,
                    }),
                    x => return Err(format!("`{x}(..)` inside an argument: only chuck(..) is")),
                }
            }
            _ => return Err("an expression a turning document does not take".into()),
        })
    }

    fn named_args(&self, args: &[Arg], env: &BTreeMap<String, V>) -> Result<BTreeMap<String, V>, String> {
        let mut out = BTreeMap::new();
        let mut pos = 0;
        for a in args {
            let v = self.eval(&a.value, env)?;
            match &a.name {
                Some((n, _)) => {
                    out.insert(n.clone(), v);
                }
                None => {
                    out.insert(format!("#{pos}"), v);
                    pos += 1;
                }
            }
        }
        Ok(out)
    }

    fn setup_of(&self, a: &BTreeMap<String, V>) -> Result<usize, String> {
        match named(a, "#0")? {
            V::Setup(i) => Ok(*i),
            v => Err(format!("the first argument is a lathe_setup, not {v:?}")),
        }
    }

    fn tool_of(&self, a: &BTreeMap<String, V>) -> Result<(TurnTool, u32), String> {
        match named(a, "tool")? {
            V::Tool(i) => Ok(self.tools[*i].clone()),
            v => Err(format!("`tool:` is a lathe_tool, not {v:?}")),
        }
    }

    fn call(&mut self, name: &str, f: &str, a: &BTreeMap<String, V>, dir: &Path) -> Result<V, String> {
        let op =
            |me: &mut TurnCam, kind: &str, r: Result<OpPath, ops::OpError>, s: usize| -> Result<V, String> {
                let p = r.map_err(|e| format!("{kind}: {e:?}"))?;
                me.paths.push((name.to_string(), s, p));
                me.ops.push(OpOut {
                    name: name.into(),
                    kind: kind.into(),
                    status: "planned".into(),
                    moves: me.paths.last().map_or(0, |p| p.2.steps.len()),
                    refusal: None,
                });
                Ok(V::Op(me.paths.len() - 1))
            };
        match f {
            "part" => {
                let file = text(a, "#0", None)?;
                let path = dir.join(&file);
                let src = forge_dirs::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                let (run, _) = forge_lang::run(&src, forge_lang::Options::default(), Default::default());
                if !run.ok() {
                    return Err(format!(
                        "the part `{file}` does not build: {:?}",
                        run.errors.first()
                    ));
                }
                let body = run.body.ok_or("the part has no body")?;
                let hp = forge_turn::profile::half_profile(&body)
                    .map_err(|e| format!("the part is not a turned part about its z axis: {e:?}"))?;
                self.parts.push(hp);
                Ok(V::Part(self.parts.len() - 1))
            }
            "lathe_tool" => {
                let id = text(a, "#0", None)?;
                let t = insert::builtin(&id).map_err(|e| e.to_string())?;
                let st = num(a, "t", None)? as u32;
                self.tools.push((t, st));
                Ok(V::Tool(self.tools.len() - 1))
            }
            "lathe_setup" => {
                let V::Part(pi) = named(a, "#0")? else {
                    return Err("lathe_setup's first argument is a part(..)".into());
                };
                let V::Chuck(chuck) = named(a, "chuck")?.clone() else {
                    return Err("`chuck:` is a chuck(..)".into());
                };
                let machine = match a.get("machine") {
                    Some(V::Str(m)) => {
                        Some(machine::builtin(m).map_err(|e| format!("{e:?}"))?)
                    }
                    None => None,
                    Some(v) => return Err(format!("`machine:` is a name, not {v:?}")),
                };
                let place = match (a.get("at"), a.get("offsets")) {
                    (Some(V::Num(z0)), Some(V::Tuple(offs))) => {
                        let mut tools = Vec::new();
                        for o in offs {
                            match o {
                                V::Tuple(t) if t.len() == 3 => match (&t[0], &t[1], &t[2]) {
                                    (V::Num(n), V::Num(x), V::Num(z)) => tools.push((*n as u32, [*x, *z])),
                                    _ => return Err("an offset is (station, x, z)".into()),
                                },
                                _ => return Err("an offset is (station, x, z)".into()),
                            }
                        }
                        Some(Place { z0: *z0, tools })
                    }
                    (None, None) => None,
                    _ => {
                        return Err(
                            "`at:` (the machine Z of the part's Z0) and `offsets:` go together".into(),
                        );
                    }
                };
                let s = Setup {
                    part: self.parts[*pi].clone(),
                    bar_r: 0.5 * num(a, "bar", None)?,
                    bar_front: num(a, "front", None)?,
                    chuck,
                    fixture_clearance: num(a, "clearance", Some(1.0))?,
                    holder_clearance: num(a, "holder_clearance", Some(0.25))?,
                    rapid_clearance: num(a, "rapid_clearance", Some(0.2))?,
                    tol: num(a, "tol", Some(0.002))?,
                    machine,
                    place,
                    fixtures: Vec::new(),
                };
                s.check().map_err(|e| format!("{e:?}"))?;
                self.setups.push((name.to_string(), s));
                Ok(V::Setup(self.setups.len() - 1))
            }
            "turn_face" => {
                let si = self.setup_of(a)?;
                let (t, st) = self.tool_of(a)?;
                let r = ops::face(
                    &self.setups[si].1,
                    &t,
                    st,
                    num(a, "to", None)?,
                    num(a, "step", Some(t.ap))?,
                );
                op(self, "face", r, si)
            }
            "turn_rough" => {
                let si = self.setup_of(a)?;
                let (t, st) = self.tool_of(a)?;
                let s = &self.setups[si].1;
                let top = num(a, "top", Some(s.part.bbox[3]))?;
                let r = ops::rough(
                    s,
                    &t,
                    st,
                    num(a, "leave", Some(0.3))?,
                    num(a, "step", Some(t.ap))?,
                    num(a, "to", None)?,
                    top,
                );
                op(self, "rough", r, si)
            }
            "bore_rough" => {
                let si = self.setup_of(a)?;
                let (t, st) = self.tool_of(a)?;
                let s = &self.setups[si].1;
                let top = num(a, "top", Some(s.part.bbox[3]))?;
                let r = ops::bore_rough(
                    s,
                    &t,
                    st,
                    num(a, "leave", Some(0.3))?,
                    num(a, "step", Some(t.ap))?,
                    num(a, "to", None)?,
                    top,
                    0.5 * num(a, "from", None)?,
                );
                op(self, "bore rough", r, si)
            }
            "turn_finish" => {
                let si = self.setup_of(a)?;
                let (t, st) = self.tool_of(a)?;
                let (s, leave, to) = (
                    &self.setups[si].1,
                    num(a, "leave", Some(0.0))?,
                    num(a, "to", None)?,
                );
                // a left-hand tool finishes up the profile, from `from:` to `to:`
                let r = if t.left {
                    ops::finish_up(s, &t, st, leave, num(a, "from", None)?, to)
                } else {
                    ops::finish(s, &t, st, leave, to)
                };
                op(self, "finish", r, si)
            }
            "turn_groove" => {
                let si = self.setup_of(a)?;
                let (t, st) = self.tool_of(a)?;
                let (s, z0, z1, rho) = (
                    &self.setups[si].1,
                    num(a, "from", None)?,
                    num(a, "to", None)?,
                    num(a, "r", None)?,
                );
                let r = if t.kind == insert::Kind::IdGroove {
                    ops::id_groove(s, &t, st, z0, z1, rho, 0.5 * num(a, "bore", None)?)
                } else {
                    ops::groove(s, &t, st, z0, z1, rho)
                };
                op(self, "groove", r, si)
            }
            "turn_thread" => {
                let si = self.setup_of(a)?;
                let (t, st) = self.tool_of(a)?;
                let thread = if t.kind == insert::Kind::IdThread {
                    ops::id_thread
                } else {
                    ops::thread
                };
                let r = thread(
                    &self.setups[si].1,
                    &t,
                    st,
                    num(a, "d", None)?,
                    num(a, "pitch", None)?,
                    num(a, "from", None)?,
                    num(a, "to", None)?,
                    num(a, "passes", Some(8.0))? as usize,
                );
                op(self, "thread", r, si)
            }
            "part_off" => {
                let si = self.setup_of(a)?;
                let (t, st) = self.tool_of(a)?;
                let r = ops::part_off(&self.setups[si].1, &t, st, num(a, "at", None)?);
                op(self, "part-off", r, si)
            }
            "turn_drill" => {
                let si = self.setup_of(a)?;
                let (t, st) = self.tool_of(a)?;
                let r = ops::drill(
                    &self.setups[si].1,
                    &t,
                    st,
                    num(a, "to", None)?,
                    num(a, "peck", Some(0.0))?,
                );
                op(self, "drill", r, si)
            }
            "lathe_program" => {
                let si = self.setup_of(a)?;
                let post_name = text(a, "post", None)?;
                let number = num(a, "number", Some(1.0))? as u32;
                self.program(name, si, &post_name, number);
                Ok(V::Num(0.0))
            }
            x => Err(format!("`{x}` is not a turning built-in")),
        }
    }

    /// Posts the setup's ops, reads the text back, certifies the path it means.
    fn program(&mut self, name: &str, si: usize, post_name: &str, number: u32) {
        let mut out = ProgramOut {
            name: name.into(),
            post: post_name.into(),
            number,
            text: None,
            error: None,
        };
        let idx: Vec<usize> = (0..self.paths.len()).filter(|&i| self.paths[i].1 == si).collect();
        let job: Vec<OpPath> = idx.iter().map(|&i| self.paths[i].2.clone()).collect();
        let s = &self.setups[si].1;
        let r = post::Dialect::get(post_name)
            .map_err(|e| format!("{e:?}"))
            .and_then(|d| {
                if let Some(m) = &s.machine
                    && m.control != post_name
                {
                    self.warnings.push(format!(
                        "the machine `{}` speaks `{}`, the program is posted as `{post_name}`",
                        m.name, m.control
                    ));
                }
                let q = post::quantize(&job, number, name, &d).map_err(|e| format!("{e:?}"))?;
                let (text, motions) = post::round_trip(&q, &d)?;
                let posted = post::posted_moves(&motions, &job, &d);
                let rep = if self.exact {
                    ops::certify(s, &job, Some(&posted))?
                } else {
                    ops::certify_fast(s, &job, Some(&posted))?
                };
                Ok((text, rep))
            });
        match r {
            Ok((text, rep)) => {
                for (k, &i) in idx.iter().enumerate() {
                    let refs: Vec<_> = rep.refusals.iter().filter(|x| x.op == k).collect();
                    let o = &mut self.ops[i];
                    if refs.is_empty() {
                        o.status = "built".into();
                    } else {
                        o.status = "refused".into();
                        let r = refs[0];
                        o.refusal = Some(format!(
                            "{} ({}) at step {}, near ({:.4}, {:.4}): {} (and {} more)",
                            r.rule,
                            r.body,
                            r.step,
                            r.at[0],
                            r.at[1],
                            r.verdict,
                            refs.len() - 1
                        ));
                    }
                }
                if rep.ok() {
                    out.text = Some(text);
                } else {
                    out.error = Some(format!("{} refusals: no program is written", rep.refusals.len()));
                    self.ok = false;
                }
                self.report = Some(rep);
            }
            Err(e) => {
                out.error = Some(e.clone());
                self.errors.push(e);
                self.ok = false;
            }
        }
        self.programs.push(out);
    }

    /// "full", "except G5" (no machine, or a machine but no placement: the spans only), or
    /// "refused".
    pub fn certification(&self) -> &str {
        if !self.ok {
            return "refused";
        }
        match &self.report {
            Some(r) if r.g5 == "full" => "full",
            Some(_) => "except G5",
            None => "refused",
        }
    }

    /// The program's text, when every op is certified.
    pub fn program_text(&self) -> Option<&str> {
        self.programs.iter().find_map(|p| p.text.as_deref())
    }

    /// The reply, `forgecad.cam/1`.
    pub fn json(&self) -> Json {
        let ops = self
            .ops
            .iter()
            .map(|o| {
                Json::obj(vec![
                    ("name", Json::Str(o.name.clone())),
                    ("kind", Json::Str(o.kind.clone())),
                    ("status", Json::Str(o.status.clone())),
                    ("moves", Json::Num(o.moves as f64)),
                    ("refusal", o.refusal.clone().map(Json::Str).unwrap_or(Json::Null)),
                ])
            })
            .collect();
        let programs = self
            .programs
            .iter()
            .map(|p| {
                Json::obj(vec![
                    ("name", Json::Str(p.name.clone())),
                    ("post", Json::Str(p.post.clone())),
                    ("number", Json::Num(p.number as f64)),
                    (
                        "lines",
                        Json::Num(p.text.as_ref().map_or(0, |t| t.lines().count()) as f64),
                    ),
                    ("round_trip", Json::Bool(p.text.is_some())),
                    ("error", p.error.clone().map(Json::Str).unwrap_or(Json::Null)),
                ])
            })
            .collect();
        let (checks, leftover) = match &self.report {
            Some(r) => (
                r.checks,
                r.leftover
                    .iter()
                    .zip(&r.leftover_reasons)
                    .map(|(g, why)| {
                        let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
                        for l in &g.loops {
                            for sg in &l.segs {
                                for p in [sg.start(), sg.end()] {
                                    b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
                                }
                            }
                        }
                        Json::obj(vec![
                            ("reason", Json::Str(why.clone())),
                            ("rho", Json::Arr(vec![Json::Num(b[0]), Json::Num(b[2])])),
                            ("z", Json::Arr(vec![Json::Num(b[1]), Json::Num(b[3])])),
                        ])
                    })
                    .collect(),
            ),
            None => (0, vec![]),
        };
        Json::obj(vec![
            ("ok", self.ok.into()),
            ("schema", Json::Str("forgecad.cam/1".into())),
            ("process", Json::Str("turning".into())),
            ("certification", Json::Str(self.certification().into())),
            ("errors", Json::Arr(self.errors.iter().map(|e| Json::Str(e.clone())).collect())),
            ("warnings", Json::Arr(self.warnings.iter().map(|e| Json::Str(e.clone())).collect())),
            ("ops", Json::Arr(ops)),
            ("programs", Json::Arr(programs)),
            ("checks", Json::Num(checks as f64)),
            (
                "stock_model",
                Json::Str(self.report.as_ref().map_or(String::new(), |r| r.stock_model.clone())),
            ),
            (
                "leftover",
                Json::obj(vec![
                    ("computed", self.report.as_ref().is_some_and(|r| r.leftover_done).into()),
                    ("rule", Json::Str("G4: final stock ⊆ (part ⊕ B_tol) ∪ these regions ∪ the declared removals, in (ρ, z); computed by `cam F check`".into())),
                    ("regions", Json::Arr(leftover)),
                ]),
            ),
        ])
    }
}

/// Runs a turning document (its file's directory finds the part): the normal path.
pub fn run(src: &str, dir: &Path) -> TurnCam {
    run_with(src, dir, false)
}

/// `run` with the exact in-process stock and the G4 leftover (`forgecad cam F check`).
pub fn run_checked(src: &str, dir: &Path) -> TurnCam {
    run_with(src, dir, true)
}

fn run_with(src: &str, dir: &Path, exact: bool) -> TurnCam {
    let mut cam = TurnCam {
        exact,
        ok: true,
        errors: vec![],
        warnings: vec![],
        ops: vec![],
        programs: vec![],
        report: None,
        parts: vec![],
        tools: vec![],
        setups: vec![],
        paths: vec![],
    };
    let prog = match syntax::parse(src) {
        Ok(p) => p,
        Err(e) => {
            cam.ok = false;
            cam.errors.push(format!("{e:?}"));
            return cam;
        }
    };
    if !is_turning(src) {
        cam.ok = false;
        cam.errors
            .push("not a turning document: `machining` first, and a lathe_setup(..)".into());
        return cam;
    }
    let mut env: BTreeMap<String, V> = BTreeMap::new();
    for st in prog.iter().skip(1) {
        let r: Result<(), String> = (|| {
            match &st.kind {
                StmtKind::Shared { name, value } => {
                    let v = cam.eval(value, &env)?;
                    env.insert(name.clone(), v);
                }
                StmtKind::Assign { name, value } => {
                    let v = match &value.kind {
                        ExprKind::Call(f, args) if matches!(&f.kind, ExprKind::Name(n) if n != "chuck") => {
                            let ExprKind::Name(fname) = &f.kind else {
                                unreachable!()
                            };
                            let a = cam.named_args(args, &env)?;
                            cam.call(name, fname, &a, dir)?
                        }
                        _ => cam.eval(value, &env)?,
                    };
                    env.insert(name.clone(), v);
                }
                StmtKind::Expr(_) => {}
                _ => return Err("a statement a turning document does not take".into()),
            }
            Ok(())
        })();
        if let Err(e) = r {
            cam.ok = false;
            cam.errors.push(e);
        }
    }
    if cam.programs.is_empty() && cam.ok {
        cam.warnings
            .push("no lathe_program(..): nothing is posted or certified".into());
    }
    cam
}

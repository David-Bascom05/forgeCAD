//! `query F bendsim`: the part's sheet bent on a press brake (kernel/forge-brake), answered as
//! typed JSON, schema `forgecad.bendsim/1` (docs/bendsim.md).
//!
//! Words: `bendsim [tooling T] [vendor V] [sequence S] [sheet NAME] [uts MPA]`.
//! - T: a standard set (`std-eu`, `std-us`) or a tooling file (JSON, beside the part).
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
    pub tooling: Option<String>,
    pub vendor: Option<String>,
    pub sequence: Option<String>,
    pub sheet: Option<String>,
    pub uts: Option<f64>,
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
            "vendor" => a.vendor = Some(val("vendor")?),
            "sequence" => a.sequence = Some(val("sequence")?),
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
                    "bendsim takes tooling T, vendor V, sequence S, sheet NAME, uts MPA; not `{other}`"
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
        Ok(s) => s.answer,
        Err(a) => a,
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
}

/// The part's sheet simulated: Err is the typed error reply.
pub fn simulate(src: &str, dir: Option<PathBuf>, root: Option<PathBuf>, ask: &Ask) -> Result<Sim, Answer> {
    let opts = Options {
        dir: dir.clone(),
        root,
        ..Options::default()
    };
    let (r, _) = run(src, opts, Default::default());
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
    let tooling = tooling(ask, dir.as_deref())?;
    let model = match fb::Model::new(&sh.sheet) {
        Ok(m) => m,
        Err(e) => return Err(model_err(sh, &e)),
    };
    let bend_names: Vec<String> = model.bends.iter().map(|b| joint_name(sh, b.joint)).collect();
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
    let opts = fb::Options {
        sequence,
        material: sh.material.clone(),
        uts,
        ..fb::Options::default()
    };
    let plan = fb::plan(&model, &tooling, &opts);
    let answer = answer(sh, &model, &tooling, &plan, &bend_names);
    Ok(Sim {
        model,
        tooling,
        plan,
        names: bend_names,
        answer,
    })
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
fn tooling(ask: &Ask, dir: Option<&Path>) -> Result<fb::Tooling, Answer> {
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
    if let Some(v) = &ask.vendor {
        let j = read(v)?;
        let t = j
            .get("tooling")
            .ok_or_else(|| err("BadTooling", format!("vendor file {v} has no `tooling`"), vec![]))?;
        return tooling_of(t);
    }
    match &ask.tooling {
        None => Ok(fb::library::std_eu()),
        Some(t) => match fb::library::set(t) {
            Some(s) => Ok(s),
            None if t.ends_with(".json") => tooling_of(&read(t)?),
            None => Err(err(
                "NoSuchTooling",
                format!("no tooling set `{t}`: a standard set or a .json file"),
                vec![("ids", strs(&fb::library::IDS))],
            )),
        },
    }
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
    let whole = j.get("base").is_none() && j.get("punches").is_some() && j.get("dies").is_some();
    if whole {
        t.punches.clear();
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
        ("certified", c.certified.into()),
        ("unit", (if a { "deg" } else { "mm" }).into()),
    ];
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

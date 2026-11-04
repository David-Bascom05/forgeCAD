//! Inventor's assembly authoring: Place, Joint, Constrain, Pattern, Free Move and Free Rotate,
//! and what a component's or a relationship's menu does (ground, suppress, delete, edit). Every
//! change is typed forge-lang ops on the .fg. A dialog previews its ops as they stand (the solve
//! and its typed problems show before OK, as Inventor's do); OK applies them as one undo step.

use super::Shell;
use super::fg::{FgAct, Severity, short};
use super::fgasm::{AsmPanel, dialog_head};
use super::icons;
use super::theme::{Theme, medium, mono, sans, semi};
use crate::snaps::SnapKind;
use egui::{Align2, Color32, Rect, Sense, Stroke, Ui, pos2, vec2};
use forge_lang::error::Op;
use forge_lang::syntax::{Expr, ExprKind, StmtKind};
use forge_math::Frame;
use std::path::PathBuf;

/// The Joint dialog's types: its word, the joint it writes (Automatic chooses), its icon.
pub const JOINTS: [(&str, &str, &str); 7] = [
    ("Automatic", "", "jrevolute"),
    ("Rigid", "fastened", "jfastened"),
    ("Rotational", "revolute", "jrevolute"),
    ("Slider", "slider", "jslider"),
    ("Cylindrical", "cylindrical", "jcylindrical"),
    ("Planar", "planar", "plane"),
    ("Ball", "ball", "ball"),
];

/// The Place Constraint dialog's types; None: no Forge constraint is it yet.
pub const CONSTRAINTS: [(&str, Option<&str>, &str); 5] = [
    ("Mate", Some("mate"), "jmate"),
    ("Angle", Some("angle"), "jangle"),
    ("Tangent", None, "tangent"),
    ("Insert", Some("insert"), "jinsert"),
    ("Symmetry", None, "symmetric"),
];

/// A joint's values, each with the param name the dialog writes for it (`x`, `y`: built-ins).
fn freedoms(joint: &str) -> &'static [(&'static str, &'static str, bool)] {
    match joint {
        "revolute" => &[("turn", "turn", true)],
        "slider" => &[("along", "along", false)],
        "cylindrical" => &[("along", "along", false), ("turn", "turn", true)],
        "planar" => &[("x", "dx", false), ("y", "dy", false), ("turn", "turn", true)],
        _ => &[],
    }
}

/// One origin picked: on which occurrence, its text in the part, what it is, where it was.
#[derive(Clone, Debug, PartialEq)]
pub struct Pick {
    pub occ: String,
    /// The reference as the assembly writes it: `peg.pin.end.center`.
    pub reference: String,
    pub kind: Option<SnapKind>,
    pub world: Option<Frame>,
    /// In the occurrence's part: the world follows the occurrence as the preview moves it.
    pub local: Option<Frame>,
}

/// What's under the pointer while an origin is wanted: the occurrence, its snaps in the world,
/// and (from the dialog) the one nearest the pointer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hover {
    pub occ: String,
    /// Each snap: what it is, its text, where it is in the world and in the part.
    pub snaps: Vec<(SnapKind, String, Frame, Frame)>,
    /// Why nothing snaps here.
    pub why: Option<String>,
    pub near: Option<Pick>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Motion {
    /// 0 rotation (gear), 1 rotation-translation (rack and pinion).
    pub kind: usize,
    pub a: String,
    pub b: String,
    pub value: String,
    pub reverse: bool,
}

/// The Joint or Place Constraint dialog, new or editing a statement.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinkForm {
    pub joint: bool,
    pub kind: usize,
    /// Mate: 0 mate, 1 flush.
    pub solution: usize,
    pub picks: [Option<Pick>; 2],
    /// The origin the view's clicks go to.
    pub waiting: Option<usize>,
    pub flip: bool,
    /// Per freedom (joint) or the offset/angle (constraint).
    pub values: Vec<String>,
    /// Per freedom: limited, low, high.
    pub limits: Vec<(bool, String, String)>,
    /// Joint: 0 Joint, 1 Limits. Constraint: 0 Assembly, 1 Motion.
    pub tab: usize,
    pub motion: Motion,
    pub name: String,
    pub editing: Option<String>,
    /// The ops last previewed, and why forge-lang refuses them if it does.
    pub sent: Option<Vec<Op>>,
    pub refused: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlaceForm {
    /// Part and assembly files near the assembly, by their path from it, and which are assemblies.
    pub files: Vec<(String, bool)>,
    pub filter: String,
    pub chosen: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PatternForm {
    pub inst: String,
    pub count: String,
    pub spacing: String,
    /// 0 x, 1 y, 2 z.
    pub dir: usize,
}

/// Inventor's Mirror Components and Copy Components: a new part file (a mirrored one, or a copy)
/// placed, or the same file reused.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MirrorForm {
    pub inst: String,
    /// Mirror (else Copy).
    pub mirror: bool,
    /// The assembly's plane mirrored in: 0 XY, 1 XZ, 2 YZ.
    pub plane: usize,
    /// The same part file again (a symmetric part, or a plain copy), not a new file.
    pub reuse: bool,
    /// The new file's name, beside the part's.
    pub file: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AsmTool {
    Place(PlaceForm),
    Link(Box<LinkForm>),
    Pattern(PatternForm),
    MirrorCopy(MirrorForm),
    /// Free Move (false) or Free Rotate (true).
    Free(bool),
}

impl AsmTool {
    /// An origin is wanted from the view.
    pub fn waiting(&self) -> bool {
        matches!(self, AsmTool::Link(f) if f.waiting.is_some())
    }
}

/// A number as typed, with its unit word or none: the value.
fn number(s: &str) -> Option<f64> {
    let t = s.trim();
    let t = t
        .strip_suffix("deg")
        .or_else(|| t.strip_suffix('°'))
        .or_else(|| t.strip_suffix("mm"))
        .unwrap_or(t);
    t.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

fn literal(v: f64, angle: bool) -> String {
    if angle {
        format!("{}deg", short(v))
    } else {
        short(v)
    }
}

/// A number field's text: Err with what's wrong.
fn value_of(s: &str, what: &str, angle: bool) -> Result<Option<String>, String> {
    if s.trim().is_empty() {
        return Ok(None);
    }
    number(s)
        .map(|v| Some(literal(v, angle)))
        .ok_or(format!("{what}: not a number"))
}

/// A field's text: a number (written as the statement's own param) or an expression of the
/// assembly's params (`hinge_off`, `door_open`), written as it is.
enum Val {
    Lit(String),
    Expr(String),
}

fn val_of(s: &str, what: &str, angle: bool) -> Result<Option<Val>, String> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    if let Some(v) = number(t) {
        return Ok(Some(Val::Lit(literal(v, angle))));
    }
    let ok = t
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || " _.+-*/()".contains(c))
        && forge_lang::syntax::parse(&format!("probe_ = {t}\n")).is_ok();
    if ok {
        Ok(Some(Val::Expr(t.to_string())))
    } else {
        Err(format!("{what}: a number or a param"))
    }
}

/// A value into the statement: a number as `param NAME.pname = v` read by name, else as written.
fn put(v: Val, name: &str, pname: &str, params: &mut Vec<String>) -> String {
    match v {
        Val::Lit(l) => {
            params.push(format!("param {name}.{pname} = {l}"));
            pname.to_string()
        }
        Val::Expr(e) => e,
    }
}

fn names_taken(src: &str) -> std::collections::BTreeSet<String> {
    forge_lang::syntax::parse(src)
        .map(|p| forge_lang::edit::bound(&p))
        .unwrap_or_default()
}

/// A new statement's name: `base_1`, `base_2`, .. whichever is free.
pub fn free_name(src: &str, base: &str) -> String {
    match forge_lang::syntax::parse(src) {
        Ok(p) => forge_lang::edit_ops::free_name(&p, base),
        Err(_) => format!("{base}_1"),
    }
}

impl LinkForm {
    pub fn new_joint(src: &str) -> LinkForm {
        let mut f = LinkForm {
            joint: true,
            waiting: Some(0),
            ..LinkForm::default()
        };
        f.name = free_name(src, "joint");
        f.fit_values();
        f
    }

    pub fn new_constraint(src: &str) -> LinkForm {
        LinkForm {
            joint: false,
            waiting: Some(0),
            values: vec![String::new()],
            name: free_name(src, "mate"),
            ..LinkForm::default()
        }
    }

    /// The joint this writes: Automatic's choice from what was picked.
    pub fn joint_kind(&self) -> &'static str {
        match JOINTS[self.kind].1 {
            "" => {
                let axial = self
                    .picks
                    .iter()
                    .all(|p| p.as_ref().and_then(|p| p.kind).is_some_and(SnapKind::axial));
                if axial { "revolute" } else { "fastened" }
            }
            k => k,
        }
    }

    /// One value field per freedom of the kind chosen.
    fn fit_values(&mut self) {
        if !self.joint {
            return;
        }
        let n = freedoms(self.joint_kind()).len();
        self.values.resize(n, String::new());
        self.limits.resize(n, (false, String::new(), String::new()));
    }

    /// The constraint this writes, from its type and what was picked.
    pub fn constraint_kind(&self) -> Result<&'static str, String> {
        let k = |i: usize| self.picks[i].as_ref().and_then(|p| p.kind);
        let point = |s: Option<SnapKind>| matches!(s, Some(SnapKind::Mid | SnapKind::End));
        let axial = |s: Option<SnapKind>| s.is_some_and(SnapKind::axial);
        match CONSTRAINTS[self.kind].1 {
            None => Err(format!("{}: no Forge constraint yet", CONSTRAINTS[self.kind].0)),
            Some("mate") => Ok(match (k(0), k(1)) {
                (a, b) if axial(a) && axial(b) => "concentric",
                (a, b) if point(a) && point(b) => "coincident",
                (a, b) if point(a) && axial(b) => "on_line",
                (a, _) if point(a) => "on_plane",
                _ if self.solution == 1 => "flush",
                _ => "mate",
            }),
            Some(other) => Ok(other),
        }
    }

    /// The statement (with its params) this dialog writes, and its name.
    pub fn statement(&self) -> Result<String, String> {
        let name = self.name.trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err("Name: letters, digits and _".into());
        }
        let mut params: Vec<String> = vec![];
        let mut args: Vec<String> = vec![];
        let f = if !self.joint && self.tab == 1 {
            let m = &self.motion;
            if m.a.is_empty() || m.b.is_empty() {
                return Err("Pick both joints".into());
            }
            let (fname, word, angle) = if m.kind == 0 {
                ("gear", "ratio", false)
            } else {
                ("rack_pinion", "radius", false)
            };
            let v = value_of(&m.value, if m.kind == 0 { "Ratio" } else { "Distance" }, angle)?.ok_or(
                if m.kind == 0 {
                    "Ratio: a number"
                } else {
                    "Distance: a number"
                },
            )?;
            let v = if m.reverse { format!("-{v}") } else { v };
            if m.kind == 0 {
                params.push(format!("param {name}.k = ratio({v})"));
            } else {
                params.push(format!("param {name}.r = {v}"));
            }
            args.extend([m.a.clone(), m.b.clone()]);
            args.push(format!("{word}: {}", if m.kind == 0 { "k" } else { "r" }));
            fname
        } else {
            let [Some(a), Some(b)] = &self.picks else {
                return Err("Pick both origins".into());
            };
            args.extend([a.reference.clone(), b.reference.clone()]);
            if self.joint {
                let kind = self.joint_kind();
                let fr = freedoms(kind);
                for (k, &(word, pname, angle)) in fr.iter().enumerate() {
                    if let Some(v) = val_of(self.values.get(k).map_or("", |s| s.as_str()), word, angle)? {
                        let v = put(v, name, pname, &mut params);
                        args.push(format!("{word}: {v}"));
                    }
                    if let Some((true, lo, hi)) = self.limits.get(k) {
                        let lo = val_of(lo, "Limit start", angle)?.ok_or("Limit start: a number")?;
                        let hi = val_of(hi, "Limit end", angle)?.ok_or("Limit end: a number")?;
                        let lo = put(lo, name, &format!("{pname}_lo"), &mut params);
                        let hi = put(hi, name, &format!("{pname}_hi"), &mut params);
                        let key = if fr.len() == 1 {
                            "limit".to_string()
                        } else {
                            format!("{word}_limit")
                        };
                        args.push(format!("{key}: ({lo}, {hi})"));
                    }
                }
                if self.flip {
                    args.push("flip: true".into());
                }
                kind
            } else {
                let kind = self.constraint_kind()?;
                let v = self.values.first().map_or("", |s| s.as_str());
                match kind {
                    "angle" => {
                        let v = val_of(v, "Angle", true)?.unwrap_or(Val::Lit("0deg".into()));
                        args.push(put(v, name, "theta", &mut params));
                    }
                    "mate" | "flush" | "insert" | "on_plane" => match val_of(v, "Offset", false)? {
                        Some(Val::Lit(l)) if l == "0" => {}
                        Some(v) => {
                            let v = put(v, name, "gap", &mut params);
                            args.push(format!("offset: {v}"));
                        }
                        None => {}
                    },
                    _ => {}
                }
                kind
            }
        };
        params.push(format!("{name} = {f}({})", args.join(", ")));
        Ok(params.join("\n"))
    }

    /// The ops OK applies: the statement added, or put in the edited one's place.
    pub fn ops(&self, src: &str) -> Result<Vec<Op>, String> {
        let text = self.statement()?;
        if self.editing.as_deref() != Some(self.name.trim()) && names_taken(src).contains(self.name.trim()) {
            return Err(format!("Name: {} is taken", self.name.trim()));
        }
        // a param the statement already has (anywhere in the file) is set where it is
        // (those just above the edited statement go with it: ReplaceFeature takes them)
        let prog = forge_lang::syntax::parse(src).unwrap_or_default();
        let (mut with, mut at) = (prog.len(), prog.len());
        if let Some(id) = &self.editing
            && let Some(k) = prog
                .iter()
                .position(|s| matches!(&s.kind, StmtKind::Assign { name, .. } if name == id))
        {
            (with, at) = (k, k);
            while with > 0
                && matches!(&prog[with - 1].kind, StmtKind::Param { owner, .. } if owner.first().is_some_and(|o| o.name == *id))
            {
                with -= 1;
            }
        }
        let have: Vec<(String, String)> = prog
            .iter()
            .enumerate()
            .filter(|(k, _)| *k < with || *k >= at)
            .filter_map(|(_, s)| match &s.kind {
                StmtKind::Param { owner, name, .. } if owner.len() == 1 => {
                    Some((owner[0].name.clone(), name.clone()))
                }
                _ => None,
            })
            .collect();
        let mut ops = vec![];
        let mut lines = vec![];
        for l in text.lines() {
            let set = l.strip_prefix("param ").and_then(|r| {
                let (key, v) = r.split_once(" = ")?;
                let (owner, name) = key.split_once('.')?;
                have.iter().any(|h| h.0 == owner && h.1 == name).then(|| {
                    let (value, dim) = match v.strip_suffix("deg") {
                        Some(d) => (number(d).unwrap_or(0.0), forge_lang::values::Dim::Angle),
                        None => match v.strip_prefix("ratio(").and_then(|x| x.strip_suffix(')')) {
                            Some(r) => (number(r).unwrap_or(0.0), forge_lang::values::Dim::Ratio),
                            None => (number(v).unwrap_or(0.0), forge_lang::values::Dim::Length),
                        },
                    };
                    Op::SetParam {
                        param: forge_lang::error::ParamKey {
                            owner: vec![forge_lang::edit::seg(owner)],
                            name: name.to_string(),
                        },
                        value: forge_lang::error::Literal { value, dim },
                    }
                })
            });
            match set {
                Some(op) => ops.push(op),
                None => lines.push(l),
            }
        }
        let text = lines.join("\n");
        match &self.editing {
            Some(id) => ops.push(Op::ReplaceFeature { id: id.clone(), text }),
            None => ops.push(Op::AddFeature { before: None, text }),
        }
        Ok(ops)
    }

    /// The dialog for an existing joint, constraint or relation, from its statement.
    pub fn edit(src: &str, id: &str) -> Option<LinkForm> {
        let prog = forge_lang::syntax::parse(src).ok()?;
        let (fname, args) = prog.iter().find_map(|s| match &s.kind {
            StmtKind::Assign { name, value } if name == id => match &value.kind {
                ExprKind::Call(f, args) => match &f.kind {
                    ExprKind::Name(n) => Some((n.clone(), args.clone())),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })?;
        let text = |e: &Expr| src[e.span.lo as usize..e.span.hi as usize].to_string();
        let arg = |name: &str, pos: usize| -> Option<Expr> {
            args.iter()
                .find(|a| a.name.as_ref().is_some_and(|n| n.0 == name))
                .or_else(|| args.iter().filter(|a| a.name.is_none()).nth(pos))
                .map(|a| a.value.clone())
        };
        // a value: its param's right side when it names one of the statement's own
        let param = |e: &Expr| -> String {
            if let ExprKind::Name(n) = &e.kind {
                for s in &prog {
                    if let StmtKind::Param { owner, name, value } = &s.kind
                        && owner.len() == 1
                        && owner[0].name == id
                        && name == n
                    {
                        return text(value);
                    }
                }
            }
            text(e)
        };
        let unratio = |s: String| {
            s.strip_prefix("ratio(")
                .and_then(|x| x.strip_suffix(')'))
                .map_or(s.clone(), |x| x.to_string())
        };
        let mut f = LinkForm {
            name: id.to_string(),
            editing: Some(id.to_string()),
            ..LinkForm::default()
        };
        let pick = |e: Option<Expr>| {
            e.map(|e| Pick {
                occ: String::new(),
                reference: text(&e),
                kind: None,
                world: None,
                local: None,
            })
        };
        if let Some(k) = JOINTS.iter().position(|j| j.1 == fname && !j.1.is_empty()) {
            f.joint = true;
            f.kind = k;
            f.picks = [pick(arg("a", 0)), pick(arg("b", 1))];
            f.flip = arg("flip", 9).is_some_and(|e| matches!(e.kind, ExprKind::Bool(true)));
            f.fit_values();
            let fr = freedoms(&fname);
            for (k, &(word, _, _)) in fr.iter().enumerate() {
                if let Some(e) = arg(word, 9) {
                    f.values[k] = param(&e);
                }
                let key = if fr.len() == 1 {
                    "limit".to_string()
                } else {
                    format!("{word}_limit")
                };
                if let Some(Expr {
                    kind: ExprKind::Tuple(t),
                    ..
                }) = arg(&key, 9)
                    && t.len() == 2
                {
                    f.limits[k] = (true, param(&t[0]), param(&t[1]));
                }
            }
            return Some(f);
        }
        let solved: Option<(usize, usize)> = match fname.as_str() {
            "mate" | "concentric" | "coincident" | "on_line" | "on_plane" => Some((0, 0)),
            "flush" => Some((0, 1)),
            "angle" => Some((1, 0)),
            "insert" => Some((3, 0)),
            _ => None,
        };
        if let Some((k, solution)) = solved {
            f.kind = k;
            f.solution = solution;
            f.picks = [pick(arg("a", 0)), pick(arg("b", 1))];
            let v = if fname == "angle" {
                arg("value", 2)
            } else {
                arg("offset", 9)
            };
            f.values = vec![v.map(|e| param(&e)).unwrap_or_default()];
            return Some(f);
        }
        if fname == "gear" || fname == "rack_pinion" {
            f.tab = 1;
            let v = arg(if fname == "gear" { "ratio" } else { "radius" }, 2)
                .map(|e| unratio(param(&e)))
                .unwrap_or_default();
            let (reverse, v) = match v.strip_prefix('-') {
                Some(x) => (true, x.to_string()),
                None => (false, v),
            };
            f.motion = Motion {
                kind: usize::from(fname == "rack_pinion"),
                a: arg("a", 0).map(|e| text(&e)).unwrap_or_default(),
                b: arg("b", 1).map(|e| text(&e)).unwrap_or_default(),
                value: v,
                reverse,
            };
            f.values = vec![String::new()];
            return Some(f);
        }
        None
    }
}

/// The .fg files in a folder, and in its folders a level down, by their path from it.
fn fg_files(base: &std::path::Path, out: &mut Vec<String>) {
    let list = |d: &std::path::Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(d)
            .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default();
        v.sort();
        v
    };
    let name = |p: &std::path::Path| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    };
    let fg = |p: &PathBuf| p.extension().is_some_and(|x| x == "fg");
    for p in list(base) {
        if p.is_dir() && !name(&p).starts_with('.') {
            out.extend(
                list(&p)
                    .iter()
                    .filter(|k| fg(k))
                    .map(|k| format!("{}/{}", name(&p), name(k))),
            );
        } else if fg(&p) {
            out.push(name(&p));
        }
    }
}

/// The .fg files a Place dialog offers: the assembly's folder's, then its parent's (from `..`).
pub fn place_files(dir: &std::path::Path) -> Vec<String> {
    let mut out = vec![];
    fg_files(dir, &mut out);
    if let Some(up) = dir.parent() {
        let here = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut more = vec![];
        fg_files(up, &mut more);
        out.extend(
            more.into_iter()
                .filter(|s| !s.starts_with(&format!("{here}/")))
                .map(|s| format!("../{s}")),
        );
    }
    out
}

/// A component's ops: grounded or not, suppressed or not, deleted with every relationship that
/// names it.
pub fn ground_op(id: &str, on: bool) -> Op {
    Op::SetArg {
        id: id.into(),
        arg: "ground".into(),
        text: on.to_string(),
    }
}

pub fn suppress_op(id: &str, on: bool) -> Op {
    Op::SetArg {
        id: id.into(),
        arg: "suppressed".into(),
        text: on.to_string(),
    }
}

/// A component suppressed with the joints and constraints that name it, as Inventor does;
/// unsuppressed with them.
pub fn suppress_ops(a: &AsmPanel, id: &str, on: bool) -> Vec<Op> {
    let mut ops = vec![suppress_op(id, on)];
    ops.extend(
        a.links
            .iter()
            .filter(|l| l.owner.is_empty() && l.suppressed != on)
            .filter(|l| {
                matches!(
                    l.family,
                    forge_lang::asm::LinkFamily::Joint | forge_lang::asm::LinkFamily::Constraint
                )
            })
            .filter(|l| [&l.a.0, &l.b.0].iter().any(|p| super::fgasm::under(p, id)))
            .map(|l| suppress_op(&l.id, on)),
    );
    ops
}

pub fn delete_ops(a: &AsmPanel, id: &str) -> Vec<Op> {
    let mut gone: Vec<String> = a
        .links
        .iter()
        .filter(|l| l.owner.is_empty())
        .filter(|l| [&l.a.0, &l.b.0].iter().any(|p| super::fgasm::under(p, id)))
        .map(|l| l.id.clone())
        .collect();
    // relations of the joints going
    let more: Vec<String> = a
        .links
        .iter()
        .filter(|l| l.owner.is_empty() && l.family == forge_lang::asm::LinkFamily::Relation)
        .filter(|l| gone.contains(&l.a.0) || gone.contains(&l.b.0))
        .map(|l| l.id.clone())
        .collect();
    gone.extend(more);
    gone.push(id.to_string());
    gone.sort();
    gone.dedup();
    gone.into_iter().map(|id| Op::DeleteFeature { id }).collect()
}

/// A pose as the file writes one.
pub fn pose_text(f: &Frame) -> String {
    let v = |x: f64| short((x * 1e6).round() / 1e6);
    let (o, z, x) = (f.origin, f.z().get(), f.x().get());
    format!(
        "pose(at: ({}, {}, {}), z: ({}, {}, {}), x: ({}, {}, {}))",
        v(o.x),
        v(o.y),
        v(o.z),
        v(z.x),
        v(z.y),
        v(z.z),
        v(x.x),
        v(x.y),
        v(x.z)
    )
}

const W: f32 = 340.0;

fn kind_button(
    ui: &mut Ui,
    t: &Theme,
    icon: &str,
    tip: &str,
    (on, enabled): (bool, bool),
    name: String,
    places: &mut Vec<(String, Rect)>,
) -> bool {
    let (r, resp) = ui.allocate_exact_size(
        vec2(34.0, 30.0),
        if enabled { Sense::click() } else { Sense::hover() },
    );
    let p = ui.painter();
    if on {
        p.rect_filled(r, 4.0, t.on);
        p.rect_stroke(r, 4.0, Stroke::new(1.0, t.focus), egui::StrokeKind::Inside);
    } else if resp.hovered() && enabled {
        p.rect_filled(r, 4.0, t.hover);
    }
    let ink = if !enabled {
        t.ink2.gamma_multiply(0.4)
    } else if on {
        t.on_ink
    } else {
        t.ink
    };
    icons::draw(p, r.center() - vec2(10.0, 10.0), 20.0, icon, 1.4, ink);
    places.push((name, r));
    let clicked = resp.clicked();
    resp.on_hover_text(tip);
    clicked && enabled
}

/// An origin's row: its number, what's picked, and whether the view's clicks go to it.
fn origin_row(
    ui: &mut Ui,
    t: &Theme,
    k: usize,
    p: &Option<Pick>,
    waiting: bool,
    places: &mut Vec<(String, Rect)>,
) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(W, 32.0), Sense::click());
    let pa = ui.painter();
    let edge = if waiting {
        t.focus
    } else if p.is_some() {
        t.accent
    } else {
        t.line
    };
    pa.rect_filled(r, 4.0, t.raised);
    pa.rect_stroke(
        r,
        4.0,
        Stroke::new(if waiting { 2.0 } else { 1.0 }, edge),
        egui::StrokeKind::Inside,
    );
    pa.circle_filled(pos2(r.left() + 16.0, r.center().y), 9.0, edge);
    pa.text(
        pos2(r.left() + 16.0, r.center().y),
        Align2::CENTER_CENTER,
        (k + 1).to_string(),
        semi(11.0),
        Color32::WHITE,
    );
    let (label, ink) = match p {
        Some(Pick {
            occ, kind: Some(k), ..
        }) if *k != SnapKind::Anchor => (format!("{occ} · {}", k.word()), t.ink),
        Some(Pick { reference, .. }) if !reference.contains('(') => (reference.clone(), t.ink),
        Some(Pick { occ, reference, .. }) if !occ.is_empty() => {
            let call = reference[occ.len() + 1..].split('(').next().unwrap_or("");
            (format!("{occ} · {call}(…)"), t.ink)
        }
        Some(p) => (p.reference.clone(), t.ink),
        None if waiting => ("Pick an origin on a component".to_string(), t.ink2),
        None => ("Origin".to_string(), t.ink2),
    };
    let room = r.width() - 40.0;
    let g = ui.fonts_mut(|f| f.layout_no_wrap(label.clone(), mono(11.0), ink));
    let shown = if g.size().x > room {
        let keep = (label.chars().count() as f32 * room / g.size().x) as usize;
        format!(
            "…{}",
            label
                .chars()
                .skip(label.chars().count().saturating_sub(keep.saturating_sub(1)))
                .collect::<String>()
        )
    } else {
        label.clone()
    };
    pa.text(
        pos2(r.left() + 32.0, r.center().y),
        Align2::LEFT_CENTER,
        shown,
        mono(11.0),
        ink,
    );
    places.push((format!("asm:origin:{}", k + 1), r));
    let c = resp.clicked();
    resp.on_hover_text(p.as_ref().map_or(label, |p| p.reference.clone()));
    c
}

fn field(
    ui: &mut Ui,
    label: &str,
    text: &mut String,
    unit: &str,
    name: String,
    places: &mut Vec<(String, Rect)>,
    t: &Theme,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_sized(
            vec2(96.0, 24.0),
            egui::Label::new(egui::RichText::new(label).font(sans(12.0)).color(t.ink)),
        );
        let r = ui.add(
            egui::TextEdit::singleline(text)
                .id(egui::Id::unique(("asm-field", &name)))
                .font(mono(12.0))
                .desired_width(150.0),
        );
        changed = r.changed();
        places.push((name, r.rect));
        ui.label(egui::RichText::new(unit).font(sans(11.0)).color(t.ink2));
    });
    changed
}

/// A dialog's footer: Apply (+, new only), Cancel, OK. What was pressed.
fn footer(
    ui: &mut Ui,
    t: &Theme,
    can_ok: bool,
    again: bool,
    places: &mut Vec<(String, Rect)>,
) -> Option<&'static str> {
    let mut out = None;
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if again {
            let r = ui.add_enabled(
                can_ok,
                egui::Button::new(egui::RichText::new("+").font(semi(15.0))).min_size(vec2(30.0, 28.0)),
            );
            places.push(("asm:apply".into(), r.rect));
            if r.on_hover_text("Apply").clicked() {
                out = Some("apply");
            }
        }
        let gap = W - if again { 38.0 } else { 0.0 } - 64.0 - 8.0 - 64.0;
        ui.add_space(gap.max(0.0));
        let c = ui.add_sized(vec2(64.0, 28.0), egui::Button::new("Cancel"));
        places.push(("asm:cancel".into(), c.rect));
        if c.clicked() {
            out = Some("cancel");
        }
        let label = egui::RichText::new("OK").font(semi(13.0)).color(Color32::WHITE);
        let ok = ui.add_enabled(
            can_ok,
            egui::Button::new(label).fill(t.accent).min_size(vec2(64.0, 28.0)),
        );
        places.push(("asm:ok".into(), ok.rect));
        if ok.clicked() {
            out = Some("ok");
        }
    });
    out
}

/// The solve's problems a dialog's statement has, as the preview gives them, and the DOF left.
fn problems(
    ui: &mut Ui,
    t: &Theme,
    issues: &[super::fg::Issue],
    name: &str,
    mobility: usize,
    places: &mut Vec<(String, Rect)>,
) -> usize {
    let mine: Vec<&super::fg::Issue> = issues
        .iter()
        .filter(|i| {
            i.feature.as_deref() == Some(name)
                || i.asm
                    .as_ref()
                    .is_some_and(|a| super::fgasm::named(a).1.iter().any(|x| x == name))
                || i.text
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .any(|w| w == name)
        })
        .collect();
    for i in &mine {
        let c = match i.severity {
            Severity::Error => t.error,
            _ => Color32::from_rgb(0xE0, 0xA0, 0x30),
        };
        let r = ui.add(
            egui::Label::new(
                egui::RichText::new(format!("{}: {}", i.kind, i.text))
                    .font(sans(11.0))
                    .color(c),
            )
            .wrap(),
        );
        places.push((format!("asm:problem:{}", i.kind), r.rect));
    }
    let r = ui.label(
        egui::RichText::new(format!("Assembly DOF {mobility}"))
            .font(sans(11.0))
            .color(t.ink2),
    );
    places.push(("asm:dof".into(), r.rect));
    mine.len()
}

impl Shell {
    /// The authoring dialog up, if any, and its glyphs in the view.
    pub(super) fn asm_tool_window(&mut self, ctx: &egui::Context) {
        let Some(mut fg) = self.fg.take() else { return };
        let t = self.theme;
        let v = self.view_rect;
        let mut places: Vec<(String, Rect)> = Vec::new();
        let mut out: Vec<FgAct> = Vec::new();
        let src = fg.source.clone();
        let issues = fg.issues.clone();
        if let Some(a) = &mut fg.asm
            && let Some(mut tool) = a.tool.take()
        {
            let at = pos2(v.left() + 16.0, v.top() + 56.0);
            let mut keep = true;
            let shown = Self::window("Assemble", "asm-tool", &t, at, W).show(ctx, |ui| match &mut tool {
                AsmTool::Place(p) => keep = Self::place_ui(ui, &t, a, p, &mut out, &mut places),
                AsmTool::Pattern(p) => keep = Self::pattern_ui(ui, &t, p, &mut out, &mut places),
                AsmTool::MirrorCopy(m) => keep = Self::mirror_ui(ui, &t, m, &mut out, &mut places),
                AsmTool::Free(rotate) => keep = Self::free_ui(ui, &t, a, *rotate, &mut out, &mut places),
                AsmTool::Link(f) => keep = Self::link_ui(ui, &t, a, f, &src, &issues, &mut out, &mut places),
            });
            if let Some(s) = shown {
                self.view_blocks.push(s.response.rect);
            }
            if let (true, AsmTool::Link(f)) = (keep, &mut tool) {
                self.asm_glyphs(ctx, a, f);
            }
            if keep {
                a.tool = Some(tool);
            } else {
                a.hover = None;
            }
        }
        self.places.extend(places);
        fg.out.extend(out);
        self.fg = Some(fg);
    }

    fn place_ui(
        ui: &mut Ui,
        t: &Theme,
        a: &AsmPanel,
        p: &mut PlaceForm,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) -> bool {
        if dialog_head(ui, t, "part", "Place Component", "asm:place", places) {
            return false;
        }
        let r = ui.add(
            egui::TextEdit::singleline(&mut p.filter)
                .id(egui::Id::unique("asm-place-filter"))
                .hint_text("Filter")
                .font(sans(12.0))
                .desired_width(W),
        );
        places.push(("asm:placefilter".into(), r.rect));
        let f = p.filter.to_lowercase();
        let shown: Vec<usize> = (0..p.files.len())
            .filter(|&k| f.is_empty() || p.files[k].0.to_lowercase().contains(&f))
            .collect();
        egui::ScrollArea::vertical()
            .id_salt("asm-place-files")
            .max_height(300.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for k in shown {
                    let on = p.chosen == Some(k);
                    let (r, resp) = ui.allocate_exact_size(vec2(W - 12.0, 22.0), Sense::click());
                    if on {
                        ui.painter().rect_filled(r, 3.0, t.on);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(r, 3.0, t.hover);
                    }
                    let (file, sub) = &p.files[k];
                    let ink = if on { t.on_ink } else { t.ink };
                    icons::draw(
                        ui.painter(),
                        r.left_top() + vec2(4.0, 3.0),
                        16.0,
                        if *sub { "subasm" } else { "part" },
                        1.25,
                        ink,
                    );
                    ui.painter().text(
                        pos2(r.left() + 26.0, r.center().y),
                        Align2::LEFT_CENTER,
                        file,
                        mono(11.0),
                        ink,
                    );
                    places.push((format!("asm:file:{file}"), r));
                    if resp.clicked() {
                        p.chosen = Some(k);
                    }
                    if resp.double_clicked() {
                        out.push(FgAct::AsmPlace {
                            file: file.clone(),
                            like: None,
                        });
                        p.chosen = None;
                    }
                }
            });
        let first = a.insts.iter().all(|i| !i.path.contains('.')) && a.insts.is_empty();
        if first {
            ui.label(
                egui::RichText::new("The first component placed is grounded")
                    .font(sans(11.0))
                    .color(t.ink2),
            );
        }
        let mut keep = true;
        ui.horizontal(|ui| {
            if Self::fg_button(ui, "Browse…", "asm:browse".into(), places) {
                out.push(FgAct::AsmBrowse);
            }
            if Self::fg_button(ui, "Standard part…", "asm:stdpart".into(), places) {
                out.push(FgAct::AsmStandardPart);
            }
        });
        match footer(ui, t, p.chosen.is_some(), true, places) {
            Some("cancel") => keep = false,
            Some(w) => {
                if let Some(k) = p.chosen {
                    out.push(FgAct::AsmPlace {
                        file: p.files[k].0.clone(),
                        like: None,
                    });
                }
                keep = w == "apply";
            }
            None => {}
        }
        keep
    }

    fn pattern_ui(
        ui: &mut Ui,
        t: &Theme,
        p: &mut PatternForm,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) -> bool {
        if dialog_head(ui, t, "rect", "Pattern Component", "asm:pattern", places) {
            return false;
        }
        ui.label(
            egui::RichText::new(format!("Component  {}", p.inst))
                .font(mono(12.0))
                .color(t.ink),
        );
        field(ui, "Count", &mut p.count, "", "asm:count".into(), places, t);
        field(
            ui,
            "Spacing",
            &mut p.spacing,
            "mm",
            "asm:spacing".into(),
            places,
            t,
        );
        ui.horizontal(|ui| {
            ui.add_sized(
                vec2(96.0, 24.0),
                egui::Label::new(egui::RichText::new("Direction").font(sans(12.0)).color(t.ink)),
            );
            for (k, w) in ["X", "Y", "Z"].iter().enumerate() {
                let r = ui.selectable_label(p.dir == k, *w);
                places.push((format!("asm:dir:{w}"), r.rect));
                if r.clicked() {
                    p.dir = k;
                }
            }
        });
        let n = p.count.trim().parse::<usize>().ok().filter(|&n| n >= 2);
        let d = number(&p.spacing);
        if n.is_none() {
            ui.label(
                egui::RichText::new("Count: 2 or more")
                    .font(sans(11.0))
                    .color(t.error),
            );
        }
        match footer(ui, t, n.is_some() && d.is_some(), false, places) {
            Some("ok") => {
                let mut step = [0.0; 3];
                step[p.dir] = d.unwrap_or(0.0);
                out.push(FgAct::AsmPattern {
                    inst: p.inst.clone(),
                    count: n.unwrap_or(2),
                    step,
                });
                false
            }
            Some(_) => false,
            None => true,
        }
    }

    fn mirror_ui(
        ui: &mut Ui,
        t: &Theme,
        m: &mut MirrorForm,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) -> bool {
        let (icon, title) = if m.mirror {
            ("mirror", "Mirror Components")
        } else {
            ("subasm", "Copy Components")
        };
        if dialog_head(ui, t, icon, title, "asm:mirror", places) {
            return false;
        }
        ui.label(
            egui::RichText::new(format!("Component  {}", m.inst))
                .font(mono(12.0))
                .color(t.ink),
        );
        if m.mirror {
            ui.horizontal(|ui| {
                ui.add_sized(
                    vec2(96.0, 24.0),
                    egui::Label::new(egui::RichText::new("Mirror plane").font(sans(12.0)).color(t.ink)),
                );
                for (k, w) in ["XY", "XZ", "YZ"].iter().enumerate() {
                    let r = ui.selectable_label(m.plane == k, *w);
                    places.push((format!("asm:plane:{w}"), r.rect));
                    if r.clicked() {
                        m.plane = k;
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            ui.add_sized(
                vec2(96.0, 24.0),
                egui::Label::new(egui::RichText::new("Status").font(sans(12.0)).color(t.ink)),
            );
            let new = if m.mirror {
                "Mirror (new part)"
            } else {
                "Copy (new part)"
            };
            for (k, w) in [new, "Reuse"].iter().enumerate() {
                let r = ui.selectable_label(m.reuse == (k == 1), *w);
                places.push((
                    format!("asm:status:{}", if k == 1 { "reuse" } else { "new" }),
                    r.rect,
                ));
                if r.clicked() {
                    m.reuse = k == 1;
                }
            }
        });
        let ok_name = m.reuse
            || (!m.file.trim().is_empty() && m.file.trim().ends_with(".fg") && !m.file.contains(['/', '\\']));
        if !m.reuse {
            field(ui, "New file", &mut m.file, "", "asm:newfile".into(), places, t);
        }
        let say = match (m.mirror, m.reuse) {
            (true, false) => "A new part file: this one reflected, placed reflected in the plane",
            (true, true) => "The same part placed reflected: for a part symmetric itself",
            (false, false) => "A new part file, a copy of this one, placed beside it",
            (false, true) => "The same part placed again beside it",
        };
        ui.label(egui::RichText::new(say).font(sans(11.0)).color(t.ink2));
        match footer(ui, t, ok_name, false, places) {
            Some("ok") => {
                out.push(FgAct::AsmMirror {
                    inst: m.inst.clone(),
                    plane: m.mirror.then_some(m.plane),
                    file: (!m.reuse).then(|| m.file.trim().to_string()),
                });
                false
            }
            Some(_) => false,
            None => true,
        }
    }

    fn free_ui(
        ui: &mut Ui,
        t: &Theme,
        a: &mut AsmPanel,
        rotate: bool,
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) -> bool {
        let title = if rotate { "Free Rotate" } else { "Free Move" };
        if dialog_head(
            ui,
            t,
            if rotate { "revolve" } else { "move" },
            title,
            "asm:free",
            places,
        ) {
            a.moved.clear();
            out.push(FgAct::AsmShow);
            return false;
        }
        ui.label(
            egui::RichText::new(if rotate {
                "Drag a component to turn it."
            } else {
                "Drag a component to move it."
            })
            .font(sans(12.0))
            .color(t.ink),
        );
        if let Some(h) = &a.hover {
            let dof = a.dof(&h.occ);
            let r = ui.label(
                egui::RichText::new(format!("{}  ·  DOF {dof}", h.occ))
                    .font(mono(11.0))
                    .color(t.ink2),
            );
            places.push(("asm:freedof".into(), r.rect));
        }
        let moved: Vec<String> = a.moved.keys().cloned().collect();
        if !moved.is_empty() {
            ui.label(
                egui::RichText::new(format!("Moved: {}", moved.join(", ")))
                    .font(sans(11.0))
                    .color(t.ink2),
            );
        }
        let mut keep = true;
        ui.horizontal(|ui| {
            let r = ui.add_enabled(!moved.is_empty(), egui::Button::new("Apply pose"));
            places.push(("asm:applypose".into(), r.rect));
            if r.on_hover_text("Write the moved components' poses into the file")
                .clicked()
            {
                out.push(FgAct::AsmApplyMoves);
            }
            let r = ui.add_enabled(!moved.is_empty(), egui::Button::new("Reset"));
            places.push(("asm:reset".into(), r.rect));
            if r.clicked() {
                a.moved.clear();
                out.push(FgAct::AsmShow);
            }
            let r = ui.button("Done");
            places.push(("asm:done".into(), r.rect));
            if r.clicked() {
                a.moved.clear();
                out.push(FgAct::AsmShow);
                keep = false;
            }
        });
        keep
    }

    #[allow(clippy::too_many_arguments)]
    fn link_ui(
        ui: &mut Ui,
        t: &Theme,
        a: &mut AsmPanel,
        f: &mut LinkForm,
        src: &str,
        issues: &[super::fg::Issue],
        out: &mut Vec<FgAct>,
        places: &mut Vec<(String, Rect)>,
    ) -> bool {
        let (icon, title) = match (f.joint, f.editing.is_some()) {
            (true, false) => ("jrevolute", "Place Joint"),
            (true, true) => ("jrevolute", "Edit Joint"),
            (false, false) => ("jmate", "Place Constraint"),
            (false, true) => ("jmate", "Edit Constraint"),
        };
        if dialog_head(ui, t, icon, title, "asm:link", places) {
            out.push(FgAct::AsmPreview(None));
            return false;
        }
        // Automatic's choice follows the picks: its fields with it
        f.fit_values();
        // its tabs
        ui.horizontal(|ui| {
            let tabs: [&str; 2] = if f.joint {
                ["Joint", "Limits"]
            } else {
                ["Assembly", "Motion"]
            };
            for (k, w) in tabs.iter().enumerate() {
                let r = ui.selectable_label(f.tab == k, egui::RichText::new(*w).font(medium(12.0)));
                places.push((format!("asm:tab:{w}"), r.rect));
                if r.clicked() && !(f.editing.is_some() && !f.joint && k != f.tab) {
                    f.tab = k;
                }
            }
        });
        ui.separator();
        if f.joint && f.tab == 1 {
            let fr = freedoms(f.joint_kind());
            if fr.is_empty() {
                ui.label(
                    egui::RichText::new("A rigid or ball joint has no values to limit.")
                        .font(sans(12.0))
                        .color(t.ink2),
                );
            }
            for (k, &(word, _, angle)) in fr.iter().enumerate() {
                let unit = if angle { "deg" } else { "mm" };
                let (on, lo, hi) = &mut f.limits[k];
                let r = ui.checkbox(on, egui::RichText::new(format!("Limit {word}")).font(sans(12.0)));
                places.push((format!("asm:limit:{word}"), r.rect));
                if *on {
                    field(ui, "Start", lo, unit, format!("asm:lo:{word}"), places, t);
                    field(ui, "End", hi, unit, format!("asm:hi:{word}"), places, t);
                }
            }
        } else if f.joint || f.tab == 0 {
            // the type
            ui.horizontal(|ui| {
                if f.joint {
                    for (k, j) in JOINTS.iter().enumerate() {
                        if kind_button(
                            ui,
                            t,
                            j.2,
                            j.0,
                            (f.kind == k, true),
                            format!("asm:type:{}", j.0),
                            places,
                        ) {
                            f.kind = k;
                            f.fit_values();
                        }
                    }
                } else {
                    for (k, c) in CONSTRAINTS.iter().enumerate() {
                        let tip = match c.1 {
                            Some(_) => c.0.to_string(),
                            None => format!("{}: no Forge constraint yet", c.0),
                        };
                        if kind_button(
                            ui,
                            t,
                            c.2,
                            &tip,
                            (f.kind == k, c.1.is_some()),
                            format!("asm:type:{}", c.0),
                            places,
                        ) {
                            f.kind = k;
                            f.values = vec![String::new()];
                        }
                    }
                }
            });
            let kind_word = if f.joint {
                format!("{} · {}", JOINTS[f.kind].0, f.joint_kind())
            } else {
                match f.constraint_kind() {
                    Ok(k) => format!("{} · {k}", CONSTRAINTS[f.kind].0),
                    Err(e) => e,
                }
            };
            let r = ui.label(egui::RichText::new(kind_word).font(sans(11.0)).color(t.ink2));
            places.push(("asm:kindword".into(), r.rect));
            ui.add_space(4.0);
            for k in 0..2 {
                if origin_row(ui, t, k, &f.picks[k], f.waiting == Some(k), places) {
                    f.waiting = Some(k);
                }
            }
            if let Some(h) = a.hover.as_ref().filter(|_| f.waiting.is_some())
                && let Some(why) = &h.why
            {
                ui.label(egui::RichText::new(why).font(sans(11.0)).color(t.ink2));
            }
            ui.add_space(4.0);
            if f.joint {
                let r = ui.checkbox(
                    &mut f.flip,
                    egui::RichText::new("Flip component 1").font(sans(12.0)),
                );
                places.push(("asm:flip".into(), r.rect));
                let fr = freedoms(f.joint_kind());
                for (k, &(word, _, angle)) in fr.iter().enumerate() {
                    let label = match word {
                        "turn" => "Angle",
                        "along" => "Offset",
                        "x" => "Offset x",
                        _ => "Offset y",
                    };
                    field(
                        ui,
                        label,
                        &mut f.values[k],
                        if angle { "deg" } else { "mm" },
                        format!("asm:value:{word}"),
                        places,
                        t,
                    );
                }
            } else {
                if CONSTRAINTS[f.kind].1 == Some("mate") {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            vec2(96.0, 24.0),
                            egui::Label::new(egui::RichText::new("Solution").font(sans(12.0)).color(t.ink)),
                        );
                        for (k, w) in ["Mate", "Flush"].iter().enumerate() {
                            let r = ui.selectable_label(f.solution == k, *w);
                            places.push((format!("asm:solution:{w}"), r.rect));
                            if r.clicked() {
                                f.solution = k;
                            }
                        }
                    });
                }
                let (label, unit) = if CONSTRAINTS[f.kind].1 == Some("angle") {
                    ("Angle", "deg")
                } else {
                    ("Offset", "mm")
                };
                if f.values.is_empty() {
                    f.values.push(String::new());
                }
                field(ui, label, &mut f.values[0], unit, "asm:value:0".into(), places, t);
            }
        } else {
            // Motion: a joint's value drives another's
            ui.horizontal(|ui| {
                for (k, (w, icon)) in [
                    ("Rotation", "gear"),
                    ("Rotation-Translation", "rackpinion"),
                    ("Transitional", "tangent"),
                ]
                .iter()
                .enumerate()
                {
                    let tip = if k == 2 {
                        "Transitional: no Forge relation yet".to_string()
                    } else {
                        w.to_string()
                    };
                    if kind_button(
                        ui,
                        t,
                        icon,
                        &tip,
                        (f.motion.kind == k && k < 2, k < 2),
                        format!("asm:motion:{w}"),
                        places,
                    ) {
                        f.motion.kind = k;
                    }
                }
            });
            let joints: Vec<String> = a
                .links
                .iter()
                .filter(|l| l.owner.is_empty() && l.family == forge_lang::asm::LinkFamily::Joint)
                .map(|l| l.id.clone())
                .collect();
            for (k, label) in ["Joint 1", "Joint 2"].iter().enumerate() {
                let cur = if k == 0 { &mut f.motion.a } else { &mut f.motion.b };
                ui.horizontal(|ui| {
                    ui.add_sized(
                        vec2(96.0, 24.0),
                        egui::Label::new(egui::RichText::new(*label).font(sans(12.0)).color(t.ink)),
                    );
                    let r = egui::ComboBox::from_id_salt(("asm-motion", k))
                        .selected_text(if cur.is_empty() {
                            "Pick a joint".to_string()
                        } else {
                            cur.clone()
                        })
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            for j in &joints {
                                let r = ui.selectable_value(cur, j.clone(), j);
                                places.push((format!("asm:joint{}:{j}", k + 1), r.rect));
                            }
                        });
                    places.push((format!("asm:motionjoint:{}", k + 1), r.response.rect));
                });
            }
            let label = if f.motion.kind == 0 { "Ratio" } else { "Distance" };
            field(
                ui,
                label,
                &mut f.motion.value,
                if f.motion.kind == 0 { "" } else { "mm/turn·2π" },
                "asm:motionvalue".into(),
                places,
                t,
            );
            let r = ui.checkbox(&mut f.motion.reverse, "Reverse");
            places.push(("asm:reverse".into(), r.rect));
        }
        ui.add_space(4.0);
        field(ui, "Name", &mut f.name, "", "asm:name".into(), places, t);
        // the preview: the ops as they stand, sent when they change
        let ops = f.ops(src);
        match &ops {
            Ok(o) if f.sent.as_ref() != Some(o) => {
                f.sent = Some(o.clone());
                // what forge-lang would refuse, said here before OK
                f.refused = forge_lang::edit::apply_all_checked(src, o)
                    .err()
                    .map(|e| format!("{}: {}", e.kind(), e.text()));
                out.push(FgAct::AsmPreview(Some(o.clone())));
            }
            Err(_) if f.sent.is_some() => {
                f.sent = None;
                out.push(FgAct::AsmPreview(None));
            }
            _ => {}
        }
        if let Err(e) = &ops {
            let r = ui.label(egui::RichText::new(e).font(sans(11.0)).color(t.ink2));
            places.push(("asm:needs".into(), r.rect));
        } else if let Some(e) = &f.refused {
            let r = ui.add(egui::Label::new(egui::RichText::new(e).font(sans(11.0)).color(t.error)).wrap());
            places.push(("asm:refused".into(), r.rect));
        } else {
            problems(ui, t, issues, f.name.trim(), a.mobility, places);
        }
        let can_ok = ops.is_ok() && f.refused.is_none();
        match footer(ui, t, can_ok, f.editing.is_none(), places) {
            Some("cancel") => {
                out.push(FgAct::AsmPreview(None));
                false
            }
            Some(w) => {
                if let Ok(o) = ops {
                    out.push(FgAct::AsmOps(o));
                }
                if w == "apply" {
                    // another of the same kind
                    let next = format!("{}x", f.name.trim());
                    *f = LinkForm {
                        joint: f.joint,
                        kind: f.kind,
                        tab: f.tab,
                        waiting: Some(0),
                        values: vec![String::new(); f.values.len()],
                        limits: vec![(false, String::new(), String::new()); f.limits.len()],
                        name: free_name(
                            &format!("{src}\n{next} = 0\n{} = 0\n", f.name.trim()),
                            if f.joint { "joint" } else { "mate" },
                        ),
                        ..LinkForm::default()
                    };
                    true
                } else {
                    false
                }
            }
            None => true,
        }
    }

    /// The origins in the view: what's under the pointer to snap to (the nearest lit, its words
    /// beside it), and the two picked, each a triad with its number.
    fn asm_glyphs(&mut self, ctx: &egui::Context, a: &mut AsmPanel, f: &LinkForm) {
        let Some((camera, size)) = self.camera else { return };
        let v = self.view_rect;
        let ppp = ctx.pixels_per_point();
        let t = self.theme;
        let screen = |p: forge_math::Point3| {
            camera
                .pixel_of([p.x as f32, p.y as f32, p.z as f32], size.0, size.1)
                .map(|q| pos2(v.left() + q.0 / ppp, v.top() + q.1 / ppp))
        };
        let p = ctx
            .layer_painter(egui::LayerId::new(
                egui::Order::Middle,
                egui::Id::unique("asm-glyphs"),
            ))
            .with_clip_rect(v);
        let orange = Color32::from_rgb(0xF0, 0x8C, 0x28);
        let triad = |fr: &Frame, label: &str, ink: Color32| {
            let Some(o) = screen(fr.origin) else { return };
            // 40 points long however far away
            let r = camera.axes().1;
            let per_mm = screen(fr.origin + forge_math::Vec3::new(r[0] as f64, r[1] as f64, r[2] as f64))
                .map_or(1.0, |q| q.distance(o).max(1e-3));
            let reach = (40.0 / per_mm) as f64;
            for (d, c) in [
                (fr.x().get(), Color32::from_rgb(0xE0, 0x40, 0x40)),
                (fr.y().get(), Color32::from_rgb(0x40, 0xC0, 0x40)),
                (fr.z().get(), Color32::from_rgb(0x40, 0x80, 0xF0)),
            ] {
                if let Some(e) = screen(fr.origin + d * reach) {
                    p.line_segment([o, e], Stroke::new(2.0, c));
                }
            }
            p.circle_filled(o, 7.0, ink);
            p.text(o, Align2::CENTER_CENTER, label, semi(10.0), Color32::WHITE);
        };
        for (k, pk) in f.picks.iter().enumerate() {
            if let Some(fr) = pk.as_ref().and_then(|x| x.world) {
                triad(&fr, &(k + 1).to_string(), t.accent);
            }
        }
        let Some(h) = &mut a.hover else { return };
        h.near = None;
        if f.waiting.is_none() {
            return;
        }
        let cursor = ctx.pointer_hover_pos().filter(|c| v.contains(*c));
        let mut best: Option<(f32, usize, egui::Pos2)> = None;
        // the nearest on the screen; a rim whose face looks away (a hole's far end) only if
        // nothing else is there
        let eye = camera.eye();
        let eye = forge_math::Point3::new(eye[0] as f64, eye[1] as f64, eye[2] as f64);
        for (k, (kind, _, fr, _)) in h.snaps.iter().enumerate() {
            let Some(q) = screen(fr.origin) else { continue };
            let away =
                !matches!(kind, SnapKind::Axis | SnapKind::Anchor) && fr.z().get().dot(eye - fr.origin) < 0.0;
            let d = cursor.map_or(f32::MAX, |c| c.distance(q)) + if away { 1000.0 } else { 0.0 };
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, k, q));
            }
        }
        for (k, (kind, _, fr, _)) in h.snaps.iter().enumerate() {
            let Some(q) = screen(fr.origin) else { continue };
            let lit = best.is_some_and(|b| b.1 == k);
            let ink = if lit { orange } else { t.ink2 };
            let s = Stroke::new(if lit { 2.0 } else { 1.25 }, ink);
            match kind {
                SnapKind::Face => {
                    p.circle_stroke(q, 6.0, s);
                    p.circle_filled(q, 2.0, ink);
                }
                SnapKind::Circle => {
                    p.circle_stroke(q, 6.0, s);
                    p.line_segment([q - vec2(4.0, 0.0), q + vec2(4.0, 0.0)], s);
                    p.line_segment([q - vec2(0.0, 4.0), q + vec2(0.0, 4.0)], s);
                }
                SnapKind::Axis => {
                    let d = [
                        q + vec2(0.0, -6.0),
                        q + vec2(6.0, 0.0),
                        q + vec2(0.0, 6.0),
                        q + vec2(-6.0, 0.0),
                    ];
                    p.add(egui::Shape::closed_line(d.to_vec(), s));
                }
                SnapKind::Mid => {
                    let d = [q + vec2(0.0, -6.0), q + vec2(5.5, 4.0), q + vec2(-5.5, 4.0)];
                    p.add(egui::Shape::closed_line(d.to_vec(), s));
                }
                SnapKind::End => {
                    p.rect_stroke(
                        Rect::from_center_size(q, vec2(9.0, 9.0)),
                        0.0,
                        s,
                        egui::StrokeKind::Middle,
                    );
                }
                SnapKind::Anchor => {
                    p.circle_stroke(q, 4.0, s);
                    p.line_segment([q, q + vec2(9.0, -9.0)], s);
                }
            }
        }
        if let Some((_, k, q)) = best {
            let (kind, text, fr, local) = &h.snaps[k];
            h.near = Some(Pick {
                occ: h.occ.clone(),
                reference: format!("{}.{text}", h.occ),
                kind: Some(*kind),
                world: Some(*fr),
                local: Some(*local),
            });
            triad(fr, "", orange);
            let what = if *kind == SnapKind::Anchor {
                text.as_str()
            } else {
                kind.word()
            };
            let label = format!("{what} · {}", h.occ);
            let g = ctx.fonts_mut(|fo| fo.layout_no_wrap(label, sans(11.0), Color32::WHITE));
            let bg = Rect::from_min_size(q + vec2(12.0, 8.0), g.size() + vec2(8.0, 4.0));
            p.rect_filled(bg, 3.0, Color32::from_black_alpha(190));
            p.galley(bg.min + vec2(4.0, 2.0), g, Color32::WHITE);
        }
    }
}

/// A ribbon command: its tool up (again: down), and what the status bar says.
pub fn command(fg: &mut super::fg::FgPanel, c: super::FgCmd) -> String {
    use super::FgCmd as C;
    let src = fg.source.clone();
    let Some(a) = &mut fg.asm else {
        return "The assembly is still solving".into();
    };
    let same = match (&a.tool, c) {
        (Some(AsmTool::Place(_)), C::Place) | (Some(AsmTool::Pattern(_)), C::Pattern) => true,
        (Some(AsmTool::MirrorCopy(m)), C::Mirror | C::Copy) => m.mirror == (c == C::Mirror),
        (Some(AsmTool::Free(r)), C::FreeMove | C::FreeRotate) => *r == (c == C::FreeRotate),
        (Some(AsmTool::Link(f)), C::Joint | C::Constrain) => f.joint == (c == C::Joint),
        _ => false,
    };
    if matches!(a.tool, Some(AsmTool::Link(_))) {
        fg.out.push(FgAct::AsmPreview(None));
    }
    a.moved.clear();
    a.hover = None;
    if same {
        a.tool = None;
        fg.out.push(FgAct::AsmShow);
        return String::new();
    }
    a.drive = None;
    let (tool, say) = match c {
        C::Place => {
            let own = fg.file.clone();
            let files = a
                .dir
                .as_deref()
                .map(|d| {
                    place_files(d)
                        .into_iter()
                        .filter(|f| !own.ends_with(f.as_str()) || f.contains('/'))
                        .map(|f| {
                            let sub =
                                std::fs::read_to_string(d.join(&f)).is_ok_and(|t| crate::fg::is_assembly(&t));
                            (f, sub)
                        })
                        .collect()
                })
                .unwrap_or_default();
            (
                AsmTool::Place(PlaceForm {
                    files,
                    ..PlaceForm::default()
                }),
                "Place: choose a part or assembly file",
            )
        }
        C::Joint => (
            AsmTool::Link(Box::new(LinkForm::new_joint(&src))),
            "Joint: pick an origin on the first component",
        ),
        C::Constrain => (
            AsmTool::Link(Box::new(LinkForm::new_constraint(&src))),
            "Constrain: pick on the first component",
        ),
        C::FreeMove => (AsmTool::Free(false), "Free Move: drag a component"),
        C::FreeRotate => (AsmTool::Free(true), "Free Rotate: drag a component"),
        C::Mirror | C::Copy => {
            let Some(i) = a.selected.clone().and_then(|k| {
                a.insts
                    .iter()
                    .find(|i| i.path == k && !i.path.contains('.') && !i.sub)
            }) else {
                return "Choose a top-level part in the browser first".into();
            };
            let mirror = c == C::Mirror;
            let stem = i
                .file
                .rsplit('/')
                .next()
                .unwrap_or(&i.file)
                .trim_end_matches(".fg");
            (
                AsmTool::MirrorCopy(MirrorForm {
                    inst: i.path.clone(),
                    mirror,
                    plane: 2,
                    reuse: false,
                    file: format!("{stem}_{}.fg", if mirror { "mirror" } else { "copy" }),
                }),
                if mirror {
                    "Mirror: the plane, and a new part or reuse"
                } else {
                    "Copy: a new part or reuse"
                },
            )
        }
        C::Pattern => {
            let Some(inst) = a.selected.clone().filter(|k| {
                a.insts
                    .iter()
                    .any(|i| i.path == *k && !i.path.contains('.') && !i.sub)
            }) else {
                return "Pattern: choose a top-level part in the browser first".into();
            };
            (
                AsmTool::Pattern(PatternForm {
                    inst,
                    count: "3".into(),
                    spacing: "50".into(),
                    dir: 0,
                }),
                "Pattern: how many, how far apart",
            )
        }
        _ => return String::new(),
    };
    a.tool = Some(tool);
    fg.out.push(FgAct::AsmShow);
    say.into()
}

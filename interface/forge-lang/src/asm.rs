//! Assembly documents: a `.fg` file whose first line is `assembly`. Its statements make instances
//! of part files and subassemblies, joints and loose constraints between their references
//! (anchors and role names, `bolt[3].shank.axis`), relations, fits and fit rules; ids follow
//! the language's id rules (name, call path, range index, labels). Drives are params, so motion
//! is a `SetParam`. The document is solved by forge-asm and every result comes back typed.
//!
//! In a part file, `anchor(ref, kind:, size:, depth:)` declares an anchor with a fit feature, and
//! `name = anchor_kind()` declares a kind of its own for fit rules.

use crate::builtins::{ANG, Args, LEN, axis_of, num_of, plane_of, point_of};
use crate::error::{Error, Literal, Op, ParamKey, Warning, path_text, span_json};
use crate::eval::{Dimension, Eval, R, Value};
use crate::syntax::{Arg, Cmp, Expr, ExprKind, Key, Seg, Span, StmtKind, Unit};
use crate::values::{Dim, Num, Src, Ty};
use forge_agent::json::Json;
use forge_asm as fa;
use forge_asm::check::{BrepExact, Checker, Exact, PartMesh};
use forge_asm::{ConstraintKind, Freedom, JointKind};
use forge_math::{Frame, Point3, UnitVec3, Vec3};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

/// Built-ins of an assembly document.
pub const NAMES: &[&str] = &[
    "part",
    "subassembly",
    "pose",
    "fastened",
    "revolute",
    "slider",
    "cylindrical",
    "pin_slot",
    "planar",
    "ball",
    "coincident",
    "on_plane",
    "on_line",
    "mate",
    "flush",
    "concentric",
    "insert",
    "parallel",
    "perpendicular",
    "angle",
    "distance",
    "lock",
    "gear",
    "rack_pinion",
    "screw",
    "linear",
    "fit",
    "fit_rule",
    "kind",
];

/// Built-ins of a part file, for assemblies to use.
pub const PART_NAMES: &[&str] = &["anchor", "anchor_kind"];

/// The four built-in anchor kinds, constants in both kinds of document.
pub const KINDS: &[&str] = &["external_thread", "internal_thread", "shaft", "bore"];

/// Expect metrics of an assembly.
pub const METRICS: &[&str] = &[
    "interferences",
    "interference",
    "clearance",
    "mobility",
    "x",
    "y",
    "z",
    "value",
    "distance",
];

// ---- values -------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct RefDef {
    pub inst: String,
    pub segs: Vec<Seg>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum AsmVal {
    /// An instance, by its id.
    Inst(String),
    /// A reference on an instance: an anchor or a role name, through subassemblies.
    Ref(RefDef),
    /// A joint or loose constraint, by its id.
    Link {
        id: String,
        joint: Option<JointKind>,
    },
    /// One value of a joint (`pin.angle`).
    JointValue {
        joint: String,
        freedom: Freedom,
    },
    /// A relation, fit or fit rule.
    Item(String),
    /// A part file's anchor: its reference and its fit feature.
    Anchor {
        reference: Value,
        feature: Option<fa::Feature>,
    },
    Kind(fa::FeatureKind),
    Pose(Frame),
    /// The word `full` (`min_engagement: full`).
    Full,
}

impl AsmVal {
    pub fn ty(&self) -> Ty {
        match self {
            AsmVal::Inst(_) => Ty::Instance,
            AsmVal::Ref(_) => Ty::Reference,
            AsmVal::Link { .. } => Ty::Joint,
            AsmVal::JointValue { .. } => Ty::JointValue,
            AsmVal::Item(_) => Ty::Check,
            AsmVal::Anchor { .. } => Ty::Anchor,
            AsmVal::Kind(_) => Ty::AnchorKind,
            AsmVal::Pose(_) => Ty::Pose,
            AsmVal::Full => Ty::Word,
        }
    }
}

fn asm(v: AsmVal) -> Value {
    Value::Asm(Rc::new(v))
}

#[derive(Clone, Debug)]
pub struct InstDef {
    pub id: String,
    pub file: String,
    pub body: Option<String>,
    pub pose: Frame,
    pub ground: bool,
    pub reference: bool,
    pub suppressed: bool,
    pub sub: bool,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LinkKind {
    Joint(JointKind),
    Constraint(ConstraintKind),
}

#[derive(Clone, Debug)]
pub struct LinkDef {
    pub id: String,
    pub kind: LinkKind,
    pub a: RefDef,
    pub b: RefDef,
    pub flip: bool,
    pub drive: Vec<(Freedom, Num)>,
    pub limits: Vec<(Freedom, f64, f64)>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct RelDef {
    pub id: String,
    pub kind: fa::RelationKind,
    pub leader: (String, Freedom),
    pub follower: (String, Freedom),
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FitDef {
    pub id: String,
    pub a: String,
    pub b: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Matcher {
    Kind(fa::FeatureKind),
    File(String),
}

#[derive(Clone, Debug)]
pub struct RuleDef {
    pub id: String,
    pub a: Matcher,
    pub b: Matcher,
    pub size: Option<f64>,
    pub min_engagement: Option<fa::MinEngagement>,
    pub span: Span,
}

/// What an assembly document's evaluation made.
#[derive(Clone, Debug, Default)]
pub struct Doc {
    pub insts: Vec<InstDef>,
    pub links: Vec<LinkDef>,
    pub relations: Vec<RelDef>,
    pub fits: Vec<FitDef>,
    pub rules: Vec<RuleDef>,
    /// References expects measure (`expect x(rail.eia[0])`), resolved as joint ends are.
    pub probes: Vec<RefDef>,
}

/// Whether a program declares itself an assembly: its first statement is the word `assembly`.
pub fn declared(prog: &[crate::syntax::Stmt]) -> bool {
    matches!(prog.first().map(|s| &s.kind), Some(StmtKind::Expr(Expr { kind: ExprKind::Name(n), .. })) if n == "assembly")
}

pub fn constant(name: &str) -> Option<Value> {
    if name == "assembly" {
        return Some(Value::Builtin("assembly"));
    }
    if name == "full" {
        return Some(asm(AsmVal::Full));
    }
    if let Some(k) = KINDS.iter().position(|k| *k == name) {
        let kind = [
            fa::FeatureKind::ExternalThread,
            fa::FeatureKind::InternalThread,
            fa::FeatureKind::Shaft,
            fa::FeatureKind::Bore,
        ][k]
            .clone();
        return Some(asm(AsmVal::Kind(kind)));
    }
    NAMES
        .iter()
        .chain(PART_NAMES)
        .find(|n| **n == name)
        .map(|n| Value::Builtin(n))
}

/// A part feature called in an assembly document.
pub fn not_here(func: &'static str, span: Span) -> Error {
    note(
        span,
        Issue::WrongDocument {
            func,
            assembly: false,
        },
    )
}

/// `pose(..)`: its numbers are a written pose.
pub fn written_pose(f: &Expr) -> bool {
    matches!(&f.kind, ExprKind::Name(n) if n == "pose")
}

/// `part(.., at: ..)` and `subassembly(.., at: ..)`: their `at:` is a written pose.
pub fn places(f: &Expr) -> bool {
    matches!(&f.kind, ExprKind::Name(n) if n == "part" || n == "subassembly")
}

fn wrong(span: Span, what: &'static str, want: Ty, found: &Value) -> Error {
    Error::WrongType {
        span,
        what,
        want,
        found: found.ty(),
    }
}

fn freedom_words(kind: JointKind) -> &'static [(&'static str, Freedom)] {
    use Freedom::*;
    match kind {
        JointKind::Revolute => &[("turn", Rz)],
        JointKind::Slider => &[("along", Tz)],
        JointKind::Cylindrical => &[("along", Tz), ("turn", Rz)],
        JointKind::PinSlot => &[("along", Tx), ("turn", Rz)],
        JointKind::Planar => &[("x", Tx), ("y", Ty), ("turn", Rz)],
        JointKind::Fastened | JointKind::Ball => &[],
    }
}

/// The word a joint's freedom goes by.
pub fn freedom_word(kind: JointKind, f: Freedom) -> &'static str {
    freedom_words(kind)
        .iter()
        .find(|w| w.1 == f)
        .map_or(f.name(), |w| w.0)
}

fn joint_args(kind: JointKind) -> &'static [&'static str] {
    match kind {
        JointKind::Revolute => &["a", "b", "turn", "limit", "flip"],
        JointKind::Slider => &["a", "b", "along", "limit", "flip"],
        JointKind::Cylindrical | JointKind::PinSlot => {
            &["a", "b", "along", "turn", "along_limit", "turn_limit", "flip"]
        }
        JointKind::Planar => &[
            "a",
            "b",
            "x",
            "y",
            "turn",
            "x_limit",
            "y_limit",
            "turn_limit",
            "flip",
        ],
        JointKind::Fastened | JointKind::Ball => &["a", "b", "flip"],
    }
}

fn limit_key(word: &str) -> &'static str {
    match word {
        "turn" => "turn_limit",
        "along" => "along_limit",
        "x" => "x_limit",
        _ => "y_limit",
    }
}

fn joint_kind(name: &str) -> Option<JointKind> {
    Some(match name {
        "fastened" => JointKind::Fastened,
        "revolute" => JointKind::Revolute,
        "slider" => JointKind::Slider,
        "cylindrical" => JointKind::Cylindrical,
        "pin_slot" => JointKind::PinSlot,
        "planar" => JointKind::Planar,
        "ball" => JointKind::Ball,
        _ => return None,
    })
}

fn reference(v: &Value, span: Span) -> R<RefDef> {
    match v {
        Value::Asm(a) => match &**a {
            AsmVal::Inst(id) => Ok(RefDef {
                inst: id.clone(),
                segs: vec![],
                span,
            }),
            AsmVal::Ref(r) => Ok(RefDef { span, ..r.clone() }),
            _ => Err(wrong(span, "a joint's end", Ty::Reference, v)),
        },
        o => Err(wrong(span, "a joint's end", Ty::Reference, o)),
    }
}

fn inst_of(v: &Value, span: Span, what: &'static str) -> R<String> {
    match v {
        Value::Asm(a) => match &**a {
            AsmVal::Inst(id) => Ok(id.clone()),
            _ => Err(wrong(span, what, Ty::Instance, v)),
        },
        o => Err(wrong(span, what, Ty::Instance, o)),
    }
}

fn vec3(v: &Value, span: Span, what: &'static str) -> R<Vec3> {
    match v {
        Value::Tuple(t) if t.len() == 3 => {
            let mut xyz = [0.0; 3];
            for (k, x) in t.iter().enumerate() {
                xyz[k] = num_of(x.clone(), &[Dim::Length, Dim::Ratio, Dim::Any], span, what)?.v;
            }
            Ok(Vec3::new(xyz[0], xyz[1], xyz[2]))
        }
        Value::Dir(d) | Value::WorldDir(d) => Ok(*d),
        Value::Point(p) => Ok(p.to_vec()),
        o => Err(wrong(span, what, Ty::Tuple, o)),
    }
}

fn pose_of(ev: &mut Eval, v: Value, span: Span) -> R<Frame> {
    match &v {
        Value::Asm(a) => match &**a {
            AsmVal::Pose(f) => Ok(*f),
            _ => Err(wrong(span, "a pose", Ty::Pose, &v)),
        },
        Value::Tuple(t) if t.len() == 3 => {
            Ok(Frame::WORLD.with_origin(Point3::ORIGIN + vec3(&v, span, "a pose")?))
        }
        Value::Point(_) => Ok(Frame::WORLD.with_origin(point_of(ev, v, span)?)),
        _ => plane_of(ev, v, span, "a pose"),
    }
}

fn pair(v: &Value, span: Span, dims: &[Dim], what: &'static str) -> R<(f64, f64)> {
    match v {
        Value::Tuple(t) if t.len() == 2 => {
            let lo = num_of(t[0].clone(), dims, span, what)?.v;
            let hi = num_of(t[1].clone(), dims, span, what)?.v;
            Ok((lo.min(hi), lo.max(hi)))
        }
        o => Err(wrong(span, what, Ty::Tuple, o)),
    }
}

// ---- calls --------------------------------------------------------------------------------------

/// The assembly and anchor built-ins; None for any other name, or for a name an assembly shares
/// with a part built-in, called in a part.
pub fn call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> Option<R<Value>> {
    let in_asm = ev.asm.is_some();
    if NAMES.contains(&name) {
        if in_asm {
            return Some(asm_call(ev, name, args, span));
        }
        if crate::builtins::builtin(name).is_some() || crate::builtins::constant(name).is_some() {
            return None;
        }
        return Some(Err(note(
            span,
            Issue::WrongDocument {
                func: name,
                assembly: true,
            },
        )));
    }
    if PART_NAMES.contains(&name) {
        if in_asm {
            return Some(Err(note(
                span,
                Issue::WrongDocument {
                    func: name,
                    assembly: false,
                },
            )));
        }
        return Some(part_call(ev, name, args, span));
    }
    None
}

fn doc<'a>(ev: &'a mut Eval<'_>) -> &'a mut Doc {
    ev.asm.as_mut().expect("an assembly document")
}

fn asm_call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    if let Some(kind) = joint_kind(name) {
        return joint(ev, name, kind, args, span);
    }
    match name {
        "part" | "subassembly" => {
            let a = Args::bind(
                ev,
                name,
                &["file", "body", "at", "ground", "reference", "suppressed"],
                args,
                span,
            )?;
            let file = match a.req("file")? {
                Value::Str(s) => s.to_string(),
                o => return Err(wrong(a.span("file"), "a part file", Ty::Str, &o)),
            };
            let body = match a.opt("body") {
                None => None,
                Some(Value::Str(s)) => Some(s.to_string()),
                Some(o) => return Err(wrong(a.span("body"), "a body", Ty::Str, &o)),
            };
            let pose = match a.opt("at") {
                None => Frame::WORLD,
                Some(v) => pose_of(ev, v, a.span("at"))?,
            };
            let (ground, reference, suppressed) =
                (a.flag("ground")?, a.flag("reference")?, a.flag("suppressed")?);
            let id = ev.take_id(name, span)?;
            ev.record(
                &id,
                if name == "part" { "instance" } else { "subassembly" },
                span,
                vec![],
            );
            doc(ev).insts.push(InstDef {
                id: id.clone(),
                file,
                body,
                pose,
                ground,
                reference,
                suppressed,
                sub: name == "subassembly",
                span,
            });
            Ok(asm(AsmVal::Inst(id)))
        }
        "pose" => {
            let a = Args::bind(ev, name, &["at", "z", "x", "rz"], args, span)?;
            let o = match a.opt("at") {
                None => Vec3::ZERO,
                Some(v) => vec3(&v, a.span("at"), "a pose's origin")?,
            };
            let z = match a.opt("z") {
                None => Vec3::Z,
                Some(v) => vec3(&v, a.span("z"), "a pose's z")?,
            };
            let x = match a.opt("x") {
                None if z.cross(Vec3::Z).length() >= 1e-9 => Vec3::Z.cross(z),
                None => Vec3::X,
                Some(v) => vec3(&v, a.span("x"), "a pose's x")?,
            };
            let zu = z.normalize().ok_or(Error::BadValue {
                span: a.span("z"),
                what: "a pose's z",
                value: 0.0,
            })?;
            let mut f = Frame::from_zx(Point3::ORIGIN + o, zu, x).ok_or(Error::BadValue {
                span: a.span("x"),
                what: "a pose's x",
                value: 0.0,
            })?;
            if let Some(r) = a.num("rz", ANG)? {
                f = f.rotated_about_z(r.v);
            }
            Ok(asm(AsmVal::Pose(f)))
        }
        "coincident" | "on_plane" | "on_line" | "mate" | "flush" | "concentric" | "insert" | "parallel"
        | "perpendicular" | "angle" | "distance" | "lock" => constraint(ev, name, args, span),
        "gear" | "rack_pinion" | "screw" | "linear" => relation(ev, name, args, span),
        "fit" => {
            let a = Args::bind(ev, name, &["a", "b"], args, span)?;
            let x = inst_of(&a.req("a")?, a.span("a"), "a fit's part")?;
            let y = inst_of(&a.req("b")?, a.span("b"), "a fit's part")?;
            let id = ev.take_id(name, span)?;
            ev.record(&id, "fit", span, vec![]);
            doc(ev).fits.push(FitDef {
                id: id.clone(),
                a: x,
                b: y,
                span,
            });
            Ok(asm(AsmVal::Item(id)))
        }
        "fit_rule" => {
            let a = Args::bind(ev, name, &["a", "b", "size", "min_engagement"], args, span)?;
            let m = |v: Value, sp: Span| -> R<Matcher> {
                match &v {
                    Value::Str(s) => Ok(Matcher::File(s.to_string())),
                    Value::Asm(x) => match &**x {
                        AsmVal::Kind(k) => Ok(Matcher::Kind(k.clone())),
                        _ => Err(wrong(sp, "a fit rule's side", Ty::AnchorKind, &v)),
                    },
                    _ => Err(wrong(sp, "a fit rule's side", Ty::AnchorKind, &v)),
                }
            };
            let x = m(a.req("a")?, a.span("a"))?;
            let y = m(a.req("b")?, a.span("b"))?;
            let size = a.num("size", LEN)?;
            // a length, or `full`: the screw fills the internal thread's whole depth
            let (min, full) = match a.opt("min_engagement") {
                Some(Value::Asm(x)) if matches!(&*x, AsmVal::Full) => (None, true),
                _ => (a.num("min_engagement", LEN)?, false),
            };
            let id = ev.take_id(name, span)?;
            let mut dims = vec![];
            if let Some(n) = &size {
                dims.push(Dimension {
                    what: "size",
                    num: n.clone(),
                });
            }
            if let Some(n) = &min {
                dims.push(Dimension {
                    what: "min_engagement",
                    num: n.clone(),
                });
            }
            ev.record(&id, "fit_rule", span, dims);
            doc(ev).rules.push(RuleDef {
                id: id.clone(),
                a: x,
                b: y,
                size: size.map(|n| n.v),
                min_engagement: if full {
                    Some(fa::MinEngagement::Full)
                } else {
                    min.map(|n| fa::MinEngagement::Length(n.v))
                },
                span,
            });
            Ok(asm(AsmVal::Item(id)))
        }
        "kind" => {
            let a = Args::bind(ev, name, &["name"], args, span)?;
            match a.req("name")? {
                Value::Str(s) => Ok(asm(AsmVal::Kind(fa::FeatureKind::Declared(s.to_string())))),
                o => Err(wrong(a.span("name"), "an anchor kind", Ty::Str, &o)),
            }
        }
        _ => unreachable!("every assembly built-in is called"),
    }
}

fn joint(ev: &mut Eval, name: &'static str, kind: JointKind, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(ev, name, joint_args(kind), args, span)?;
    let ra = reference(&a.req("a")?, a.span("a"))?;
    let rb = reference(&a.req("b")?, a.span("b"))?;
    let flip = a.flag("flip")?;
    let words = freedom_words(kind);
    let mut drive = vec![];
    let mut limits = vec![];
    let mut dims = vec![];
    for &(w, f) in words {
        let d: &[Dim] = if f.is_angle() { ANG } else { LEN };
        if let Some(n) = a.num(w, d)? {
            dims.push(Dimension {
                what: w,
                num: n.clone(),
            });
            drive.push((f, n));
        }
        let key = if words.len() == 1 { "limit" } else { limit_key(w) };
        if let Some(v) = a.opt(key) {
            let (lo, hi) = pair(&v, a.span(key), d, "a limit")?;
            limits.push((f, lo, hi));
        }
    }
    let id = ev.take_id(name, span)?;
    ev.record(&id, name, span, dims);
    doc(ev).links.push(LinkDef {
        id: id.clone(),
        kind: LinkKind::Joint(kind),
        a: ra,
        b: rb,
        flip,
        drive,
        limits,
        span,
    });
    Ok(asm(AsmVal::Link {
        id,
        joint: Some(kind),
    }))
}

fn constraint(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    let names: &[&'static str] = match name {
        "on_plane" | "mate" | "flush" | "insert" => &["a", "b", "offset"],
        "concentric" | "parallel" => &["a", "b", "opposed"],
        "angle" | "distance" => &["a", "b", "value"],
        _ => &["a", "b"],
    };
    let a = Args::bind(ev, name, names, args, span)?;
    let ra = reference(&a.req("a")?, a.span("a"))?;
    let rb = reference(&a.req("b")?, a.span("b"))?;
    let offset = if names.contains(&"offset") {
        a.num("offset", LEN)?
    } else {
        None
    };
    let o = offset.as_ref().map_or(0.0, |n| n.v);
    let mut dims = vec![];
    if let Some(n) = &offset {
        dims.push(Dimension {
            what: "offset",
            num: n.clone(),
        });
    }
    let (kind, flip) = match name {
        "coincident" => (ConstraintKind::Coincident, false),
        "on_plane" => (ConstraintKind::PointOnPlane { offset: o }, false),
        "on_line" => (ConstraintKind::PointOnLine, false),
        "mate" => (ConstraintKind::PlanePlane { offset: o }, true),
        "flush" => (ConstraintKind::PlanePlane { offset: o }, false),
        "concentric" => (ConstraintKind::Concentric, a.flag("opposed")?),
        "insert" => (ConstraintKind::Insert { offset: o }, true),
        "parallel" => (ConstraintKind::Parallel, a.flag("opposed")?),
        "perpendicular" => (ConstraintKind::Perpendicular, false),
        "angle" => {
            let n = a.req_num("value", ANG)?;
            dims.push(Dimension {
                what: "angle",
                num: n.clone(),
            });
            (ConstraintKind::Angle { angle: n.v }, false)
        }
        "distance" => {
            let n = a.req_num("value", LEN)?;
            dims.push(Dimension {
                what: "distance",
                num: n.clone(),
            });
            (ConstraintKind::Distance { distance: n.v }, false)
        }
        _ => (ConstraintKind::Lock, false),
    };
    let id = ev.take_id(name, span)?;
    ev.record(&id, name, span, dims);
    doc(ev).links.push(LinkDef {
        id: id.clone(),
        kind: LinkKind::Constraint(kind),
        a: ra,
        b: rb,
        flip,
        drive: vec![],
        limits: vec![],
        span,
    });
    Ok(asm(AsmVal::Link { id, joint: None }))
}

fn relation(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    let (names, val, d): (&[&'static str], &'static str, &[Dim]) = match name {
        "gear" => (&["a", "b", "ratio"], "ratio", &[Dim::Ratio, Dim::Any]),
        "rack_pinion" => (&["a", "b", "radius"], "radius", LEN),
        "screw" => (&["a", "b", "lead"], "lead", LEN),
        _ => (&["a", "b", "ratio"], "ratio", &[Dim::Ratio, Dim::Any]),
    };
    let a = Args::bind(ev, name, names, args, span)?;
    let n = a.req_num(val, d)?;
    let (la, fa_) = match name {
        "gear" => (true, true),
        "rack_pinion" | "screw" => (true, false),
        _ => (false, false),
    };
    let value_of = |v: Value, sp: Span, angle: bool| -> R<(String, Freedom)> {
        match &v {
            Value::Asm(x) => match &**x {
                AsmVal::JointValue { joint, freedom } => Ok((joint.clone(), *freedom)),
                AsmVal::Link { id, joint: Some(k) } => k
                    .freedoms()
                    .iter()
                    .find(|f| f.is_angle() == angle)
                    .map(|f| (id.clone(), *f))
                    .ok_or_else(|| wrong(sp, "a joint value", Ty::JointValue, &v)),
                _ => Err(wrong(sp, "a joint value", Ty::JointValue, &v)),
            },
            _ => Err(wrong(sp, "a joint value", Ty::JointValue, &v)),
        }
    };
    let leader = value_of(a.req("a")?, a.span("a"), la)?;
    let follower = value_of(a.req("b")?, a.span("b"), fa_)?;
    let kind = match name {
        "gear" => fa::RelationKind::Gear { ratio: n.v },
        "rack_pinion" => fa::RelationKind::RackPinion { radius: n.v },
        "screw" => fa::RelationKind::Screw { lead: n.v },
        _ => fa::RelationKind::Linear { ratio: n.v },
    };
    let id = ev.take_id(name, span)?;
    ev.record(&id, name, span, vec![Dimension { what: val, num: n }]);
    doc(ev).relations.push(RelDef {
        id: id.clone(),
        kind,
        leader,
        follower,
        span,
    });
    Ok(asm(AsmVal::Item(id)))
}

fn part_call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    match name {
        "anchor" => {
            let a = Args::bind(ev, name, &["ref", "kind", "size", "depth"], args, span)?;
            let r = a.req("ref")?;
            let feature = match a.opt("kind") {
                None => None,
                Some(Value::Asm(k)) if matches!(&*k, AsmVal::Kind(_)) => {
                    let AsmVal::Kind(kind) = &*k else {
                        unreachable!("matched")
                    };
                    let size = a.req_num("size", LEN)?.v;
                    let depth = a.num("depth", LEN)?.map(|n| n.v);
                    Some(fa::Feature {
                        kind: kind.clone(),
                        size,
                        depth,
                    })
                }
                Some(o) => return Err(wrong(a.span("kind"), "an anchor kind", Ty::AnchorKind, &o)),
            };
            Ok(asm(AsmVal::Anchor {
                reference: r,
                feature,
            }))
        }
        "anchor_kind" => {
            Args::bind(ev, name, &[], args, span)?;
            let id = ev.take_id(name, span)?;
            Ok(asm(AsmVal::Kind(fa::FeatureKind::Declared(id))))
        }
        _ => unreachable!("every part built-in is called"),
    }
}

pub fn member(a: &Rc<AsmVal>, name: &str, span: Span) -> R<Value> {
    let seg = Seg {
        name: name.into(),
        index: vec![],
    };
    match &**a {
        AsmVal::Inst(id) => Ok(asm(AsmVal::Ref(RefDef {
            inst: id.clone(),
            segs: vec![seg],
            span,
        }))),
        AsmVal::Ref(r) => {
            let mut r = r.clone();
            r.segs.push(seg);
            r.span = span;
            Ok(asm(AsmVal::Ref(r)))
        }
        AsmVal::Link { id, joint: Some(k) } => freedom_words(*k)
            .iter()
            .find(|w| w.0 == name)
            .map(|w| {
                asm(AsmVal::JointValue {
                    joint: id.clone(),
                    freedom: w.1,
                })
            })
            .ok_or(Error::UnknownMember {
                span,
                ty: Ty::Joint,
                member: name.into(),
            }),
        other => Err(Error::UnknownMember {
            span,
            ty: other.ty(),
            member: name.into(),
        }),
    }
}

pub fn index(a: &Rc<AsmVal>, k: i64, span: Span) -> R<Value> {
    match &**a {
        AsmVal::Ref(r) => {
            let mut r = r.clone();
            if let Some(last) = r.segs.last_mut() {
                last.index.push(Key::Ix(k));
            }
            Ok(asm(AsmVal::Ref(r)))
        }
        other => Err(Error::WrongType {
            span,
            what: "indexing",
            want: Ty::List,
            found: other.ty(),
        }),
    }
}

// ---- typed results ------------------------------------------------------------------------------

/// A constraint, joint part, relation or ground, named as the document names it.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemName {
    pub id: String,
    /// The primitive or value within a joint (`coincident`, `drive angle`), or None for a whole
    /// loose constraint, relation or instance.
    pub part: Option<String>,
}

impl ItemName {
    fn text(&self) -> String {
        match &self.part {
            Some(p) => format!("{} ({p})", self.id),
            None => self.id.clone(),
        }
    }
    fn json(&self) -> Json {
        Json::obj(vec![
            ("id", self.id.as_str().into()),
            ("part", self.part.clone().map_or(Json::Null, Json::Str)),
        ])
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FileProblem {
    Missing,
    /// The file has errors of its own: how many, and the first's kind.
    Errors {
        count: usize,
        first: &'static str,
    },
    /// A subassembly that is a part file, or a part that is an assembly.
    WrongKind {
        assembly: bool,
    },
    Cyclic,
    NoBody {
        body: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Issue {
    /// A built-in of the other kind of document.
    WrongDocument {
        func: &'static str,
        assembly: bool,
    },
    PartFile {
        file: String,
        problem: FileProblem,
    },
    BrokenAnchor {
        item: String,
        side: fa::Side,
        anchor: String,
        reason: fa::AnchorReason,
    },
    WrongReference {
        item: String,
        side: fa::Side,
        anchor: String,
        want: Vec<&'static str>,
        found: &'static str,
    },
    Redundant {
        item: ItemName,
        rows: usize,
        implied_by: Vec<ItemName>,
    },
    Conflict {
        items: Vec<ItemName>,
        residuals: Vec<f64>,
        /// The statement the solve left out, which a fix deletes.
        culprit: String,
    },
    NoGround,
    NotConverged {
        items: Vec<ItemName>,
        residual: f64,
    },
    LimitViolated {
        joint: String,
        freedom: &'static str,
        value: f64,
        range: (f64, f64),
        angle: bool,
        /// The param driving the joint, set by the fix to the nearer limit.
        fix: Option<(ParamKey, f64)>,
    },
    BadFreedom {
        joint: String,
        freedom: &'static str,
    },
    BadRelation {
        relation: String,
    },
    DuplicateItem {
        id: String,
    },
    SelfLink {
        item: String,
    },
    CyclicAssembly {
        file: String,
    },
    PartChanged {
        file: String,
    },
    Interference {
        a: String,
        b: String,
        volume: Option<f64>,
        confirmed: bool,
        /// When unconfirmed because the kernel's intersection failed: its reason.
        boolean_failed: Option<String>,
    },
    IntendedContact {
        a: String,
        b: String,
        by: String,
    },
    ShallowEngagement {
        a: String,
        b: String,
        rule: String,
        engaged: f64,
        min: f64,
    },
    /// A fit rule names a kind no part declares.
    UnknownKind {
        kind: String,
        declared: Vec<String>,
    },
    ExpectFailed {
        metric: String,
        measured: f64,
        cmp: Cmp,
        limit: f64,
    },
    UnknownMetric {
        metric: String,
    },
    /// An expect that cannot be measured: never skipped.
    ExpectUnevaluable {
        metric: String,
        why: Unevaluable,
    },
    /// Inside a subassembly occurrence.
    Inside {
        path: String,
        issue: Box<Issue>,
    },
}

/// Why an expect could not be measured.
#[derive(Clone, Debug, PartialEq)]
pub enum Unevaluable {
    /// Its `index`-th argument is not given.
    MissingArg { index: usize },
    /// An argument that is no instance or reference.
    NotAPlace { found: Ty },
    /// A reference the part does not give.
    NoReference { reference: String },
    /// `value(..)` of something that is no joint value.
    NotAJointValue { found: Ty },
    /// A joint value the solve did not report (the joint failed).
    NoJointValue { joint: String },
    /// No solid to measure there (a reference to nothing built, a part with no body).
    NoGeometry { reference: String },
    /// The kernel's exact intersection of two parts failed: the pair and its reason.
    BooleanFailed { a: String, b: String, reason: String },
    /// The limit is not a number.
    NoLimit,
    /// The document could not be run again with its `at` params.
    AtFailed,
}

impl Unevaluable {
    fn text(&self) -> String {
        match self {
            Unevaluable::MissingArg { index } => format!("argument {} is missing", index + 1),
            Unevaluable::NotAPlace { found } => {
                format!("an argument is a {found:?}, not an instance or a reference")
            }
            Unevaluable::NoReference { reference } => format!("{reference} is not a reference of its part"),
            Unevaluable::NotAJointValue { found } => {
                format!("value() takes a joint value (j.turn), not a {found:?}")
            }
            Unevaluable::NoJointValue { joint } => {
                format!("{joint} has no value: it was left out of the solve")
            }
            Unevaluable::NoGeometry { reference } => format!("{reference} has no solid to measure"),
            Unevaluable::BooleanFailed { a, b, reason } => {
                format!("the exact intersection of {a} and {b} failed: {reason}")
            }
            Unevaluable::NoLimit => "the limit is not a number".into(),
            Unevaluable::AtFailed => "the document does not run with its at params".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

/// One typed assembly result, where in the document it belongs.
#[derive(Clone, Debug, PartialEq)]
pub struct AsmNote {
    pub span: Span,
    pub issue: Issue,
}

fn note(span: Span, issue: Issue) -> Error {
    Error::Asm(Box::new(AsmNote { span, issue }))
}

impl Issue {
    pub fn level(&self) -> Level {
        match self {
            Issue::Redundant { .. }
            | Issue::Interference { .. }
            | Issue::ShallowEngagement { .. }
            | Issue::PartChanged { .. } => Level::Warning,
            Issue::IntendedContact { .. } => Level::Info,
            Issue::Inside { issue, .. } => issue.level(),
            _ => Level::Error,
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Issue::WrongDocument { .. } => "WrongDocument",
            Issue::PartFile { .. } => "PartFile",
            Issue::BrokenAnchor { .. } => "BrokenAnchor",
            Issue::WrongReference { .. } => "WrongReference",
            Issue::Redundant { .. } => "Redundant",
            Issue::Conflict { .. } => "Conflict",
            Issue::NoGround => "NoGround",
            Issue::NotConverged { .. } => "NotConverged",
            Issue::LimitViolated { .. } => "LimitViolated",
            Issue::BadFreedom { .. } => "BadFreedom",
            Issue::BadRelation { .. } => "BadRelation",
            Issue::DuplicateItem { .. } => "DuplicateItem",
            Issue::SelfLink { .. } => "SelfLink",
            Issue::CyclicAssembly { .. } => "CyclicAssembly",
            Issue::PartChanged { .. } => "PartChanged",
            Issue::Interference { .. } => "Interference",
            Issue::IntendedContact { .. } => "IntendedContact",
            Issue::ShallowEngagement { .. } => "ShallowEngagement",
            Issue::UnknownKind { .. } => "UnknownKind",
            Issue::ExpectFailed { .. } => "ExpectFailed",
            Issue::UnknownMetric { .. } => "UnknownMetric",
            Issue::ExpectUnevaluable { .. } => "ExpectUnevaluable",
            Issue::Inside { issue, .. } => issue.kind(),
        }
    }
}

fn valid_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .replace("__", "_")
}

impl AsmNote {
    pub fn kind(&self) -> &'static str {
        self.issue.kind()
    }

    /// Typed edits that remove it.
    pub fn fixes(&self) -> Vec<Op> {
        match &self.issue {
            Issue::Conflict { culprit, .. } => vec![Op::DeleteFeature { id: culprit.clone() }],
            Issue::LimitViolated {
                fix: Some((param, v)),
                angle,
                ..
            } => vec![Op::SetParam {
                param: param.clone(),
                value: Literal {
                    value: if *angle { v.to_degrees() } else { *v },
                    dim: if *angle { Dim::Angle } else { Dim::Length },
                },
            }],
            // declaring the contact intended
            Issue::Interference { a, b, .. } => vec![Op::AddFeature {
                before: None,
                text: format!("fit_{} = fit({a}, {b})", valid_name(&format!("{a}_{b}"))),
            }],
            _ => vec![],
        }
    }

    pub fn text(&self) -> String {
        issue_text(&self.issue)
    }

    pub fn data(&self) -> Vec<(&'static str, Json)> {
        issue_data(&self.issue)
    }

    /// As a warning or information line.
    pub fn json(&self, src: &str) -> Json {
        let mut f: Vec<(&str, Json)> = vec![
            ("kind", self.kind().into()),
            (
                "severity",
                match self.issue.level() {
                    Level::Info => "info",
                    Level::Warning => "warning",
                    Level::Error => "error",
                }
                .into(),
            ),
        ];
        let (l, c, ..) = self.span.lines(src);
        f.push(("span", span_json(self.span)));
        f.push(("line", (l as f64).into()));
        f.push(("col", (c as f64).into()));
        f.extend(self.data());
        f.push(("text", self.text().into()));
        f.push(("fixes", Json::Arr(self.fixes().iter().map(Op::json).collect())));
        Json::obj(f)
    }
}

fn side(s: fa::Side) -> &'static str {
    match s {
        fa::Side::A => "a",
        fa::Side::B => "b",
    }
}

fn reason_text(r: &fa::AnchorReason) -> String {
    match r {
        fa::AnchorReason::NoInstance { at } => format!("no instance at step {at} of its path"),
        fa::AnchorReason::Suppressed { at } => format!("the instance at step {at} is suppressed"),
        fa::AnchorReason::BadPath { at } => format!("its path cannot go on at step {at}"),
        fa::AnchorReason::NoDocument { doc } => format!("{} is not loaded", doc.0),
        fa::AnchorReason::NoAnchor => "the part has no such reference".into(),
        fa::AnchorReason::Part(f) => match f {
            fa::AnchorFault::EntityGone { entity } => format!("its entity {entity} is gone"),
            fa::AnchorFault::Degenerate { entity } => format!("{entity} gives no reference of that kind"),
        },
    }
}

fn names(xs: &[ItemName]) -> String {
    xs.iter().map(ItemName::text).collect::<Vec<_>>().join(", ")
}

fn issue_text(i: &Issue) -> String {
    match i {
        Issue::WrongDocument { func, assembly: true } => {
            format!("{func} is for assembly documents (first line `assembly`)")
        }
        Issue::WrongDocument {
            func,
            assembly: false,
        } => format!("{func} is for part files, not assemblies"),
        Issue::PartFile { file, problem } => match problem {
            FileProblem::Missing => format!("{file}: no such file"),
            FileProblem::Errors { count, first } => format!("{file}: {count} errors, the first {first}"),
            FileProblem::WrongKind { assembly: true } => format!("{file} is an assembly: use subassembly()"),
            FileProblem::WrongKind { assembly: false } => format!("{file} is a part: use part()"),
            FileProblem::Cyclic => format!("{file} contains itself"),
            FileProblem::NoBody { body } => format!("{file} has no body {body}"),
        },
        Issue::BrokenAnchor {
            item,
            side: s,
            anchor,
            reason,
        } => {
            format!("{item}'s end {} ({anchor}): {}", side(*s), reason_text(reason))
        }
        Issue::WrongReference {
            item,
            side: s,
            anchor,
            want,
            found,
        } => format!(
            "{item}'s end {} ({anchor}) is a {found}; it takes a {}",
            side(*s),
            want.join(" or ")
        ),
        Issue::Redundant {
            item,
            rows,
            implied_by,
        } => format!(
            "{} is implied{} by {}",
            item.text(),
            if *rows > 0 {
                format!(" ({rows} rows)")
            } else {
                String::new()
            },
            names(implied_by)
        ),
        Issue::Conflict { items, culprit, .. } => {
            format!("no pose meets all of {}; {culprit} is left out", names(items))
        }
        Issue::NoGround => "nothing is grounded: each part keeps where it is written".into(),
        Issue::NotConverged { items, residual } => {
            format!("{} did not close (residual {residual:.3e})", names(items))
        }
        Issue::LimitViolated {
            joint,
            freedom,
            value,
            range,
            angle,
            ..
        } => {
            let s = |x: f64| {
                if *angle {
                    format!("{:.6}deg", x.to_degrees())
                } else {
                    format!("{x:.6}")
                }
            };
            format!(
                "{joint}.{freedom} = {} is outside {}..{}",
                s(*value),
                s(range.0),
                s(range.1)
            )
        }
        Issue::BadFreedom { joint, freedom } => format!("{joint} has no {freedom}"),
        Issue::BadRelation { relation } => format!("{relation} relates values of the wrong kinds"),
        Issue::DuplicateItem { id } => format!("{id} is made twice"),
        Issue::SelfLink { item } => format!("{item} joins a reference to itself"),
        Issue::CyclicAssembly { file } => format!("{file} contains itself"),
        Issue::PartChanged { file } => format!("{file} is not the version pinned"),
        Issue::Interference {
            a,
            b,
            volume,
            boolean_failed,
            ..
        } => match (volume, boolean_failed) {
            (Some(v), _) => format!("{a} and {b} share {v:.4} mm3 (declare a fit if meant)"),
            (None, Some(why)) => format!(
                "{a} and {b} overlap by their meshes; the exact intersection failed ({why}), so the volume is unconfirmed"
            ),
            (None, None) => format!("{a} and {b} overlap (unconfirmed: no exact solids)"),
        },
        Issue::IntendedContact { a, b, by } => format!("{a} and {b} touch as {by} says"),
        Issue::ShallowEngagement {
            a,
            b,
            rule,
            engaged,
            min,
        } => format!("{a} and {b}: threads engage {engaged:.3}, under {rule}'s {min:.3}"),
        Issue::UnknownKind { kind, declared } => format!(
            "no part declares the anchor kind {kind} (declared: {})",
            declared.join(", ")
        ),
        Issue::ExpectFailed {
            metric,
            measured,
            cmp,
            limit,
        } => format!("expect {metric}: {measured} is not {cmp:?} {limit}"),
        Issue::ExpectUnevaluable { metric, why } => {
            format!("expect {metric} cannot be measured: {}", why.text())
        }
        Issue::UnknownMetric { metric } => {
            format!("{metric} is no assembly measure ({})", METRICS.join(", "))
        }
        Issue::Inside { path, issue } => format!("in {path}: {}", issue_text(issue)),
    }
}

fn issue_data(i: &Issue) -> Vec<(&'static str, Json)> {
    let s = |x: &str| Json::Str(x.into());
    let list = |xs: &[ItemName]| Json::Arr(xs.iter().map(ItemName::json).collect());
    match i {
        Issue::WrongDocument { func, assembly } => vec![("func", s(func)), ("assembly", (*assembly).into())],
        Issue::PartFile { file, problem } => vec![("file", s(file)), ("problem", s(&format!("{problem:?}")))],
        Issue::BrokenAnchor {
            item,
            side: sd,
            anchor,
            reason,
        } => vec![
            ("item", s(item)),
            ("side", s(side(*sd))),
            ("anchor", s(anchor)),
            ("reason", s(&format!("{reason:?}"))),
        ],
        Issue::WrongReference {
            item,
            side: sd,
            anchor,
            want,
            found,
        } => vec![
            ("item", s(item)),
            ("side", s(side(*sd))),
            ("anchor", s(anchor)),
            ("want", Json::Arr(want.iter().map(|w| s(w)).collect())),
            ("found", s(found)),
        ],
        Issue::Redundant {
            item,
            rows,
            implied_by,
        } => vec![
            ("item", item.json()),
            ("rows", (*rows).into()),
            ("implied_by", list(implied_by)),
        ],
        Issue::Conflict {
            items,
            residuals,
            culprit,
        } => vec![
            ("items", list(items)),
            (
                "residuals",
                Json::Arr(residuals.iter().map(|r| Json::Num(*r)).collect()),
            ),
            ("left_out", s(culprit)),
        ],
        Issue::NoGround => vec![],
        Issue::NotConverged { items, residual } => {
            vec![("items", list(items)), ("residual", (*residual).into())]
        }
        Issue::LimitViolated {
            joint,
            freedom,
            value,
            range,
            angle,
            ..
        } => {
            let f = |x: f64| Json::Num(if *angle { x.to_degrees() } else { x });
            vec![
                ("joint", s(joint)),
                ("freedom", s(freedom)),
                ("value", f(*value)),
                ("range", Json::Arr(vec![f(range.0), f(range.1)])),
            ]
        }
        Issue::BadFreedom { joint, freedom } => vec![("joint", s(joint)), ("freedom", s(freedom))],
        Issue::BadRelation { relation } => vec![("relation", s(relation))],
        Issue::DuplicateItem { id } => vec![("id", s(id))],
        Issue::SelfLink { item } => vec![("item", s(item))],
        Issue::CyclicAssembly { file } | Issue::PartChanged { file } => vec![("file", s(file))],
        Issue::Interference {
            a,
            b,
            volume,
            confirmed,
            boolean_failed,
        } => {
            let mut v = vec![
                ("a", s(a)),
                ("b", s(b)),
                ("volume", volume.map_or(Json::Null, Json::Num)),
                ("confirmed", (*confirmed).into()),
            ];
            if let Some(why) = boolean_failed {
                v.push(("boolean_failed", s(why)));
            }
            v
        }
        Issue::IntendedContact { a, b, by } => vec![("a", s(a)), ("b", s(b)), ("by", s(by))],
        Issue::ShallowEngagement {
            a,
            b,
            rule,
            engaged,
            min,
        } => vec![
            ("a", s(a)),
            ("b", s(b)),
            ("rule", s(rule)),
            ("engaged", (*engaged).into()),
            ("min", (*min).into()),
        ],
        Issue::UnknownKind { kind, declared } => vec![
            ("anchor_kind", s(kind)),
            ("declared", Json::Arr(declared.iter().map(|d| s(d)).collect())),
        ],
        Issue::ExpectFailed {
            metric,
            measured,
            cmp,
            limit,
        } => vec![
            ("metric", s(metric)),
            ("measured", (*measured).into()),
            ("cmp", s(&format!("{cmp:?}"))),
            ("limit", (*limit).into()),
        ],
        Issue::UnknownMetric { metric } => vec![("metric", s(metric))],
        Issue::ExpectUnevaluable { metric, why } => {
            vec![("metric", s(metric)), ("why", s(&format!("{why:?}")))]
        }
        Issue::Inside { path, issue } => {
            let mut d = vec![("path", s(path))];
            d.extend(issue_data(issue));
            d
        }
    }
}

// ---- loading parts and subassemblies -------------------------------------------------------------

/// A part file, built, with the references assemblies asked of it.
pub struct PartLoad {
    pub hash: fa::ContentHash,
    pub body: Option<forge_topo::Body>,
    pub refs: BTreeMap<String, Result<fa::Reference, fa::AnchorFault>>,
    pub features: Vec<(String, fa::Feature)>,
    pub kinds: Vec<String>,
    pub errors: Vec<Error>,
    pub mesh: Option<Arc<PartMesh>>,
    /// Bodies by name (`body:`), when the part has several.
    pub bodies: BTreeMap<String, forge_topo::Body>,
}

/// A part file's text hash and the references and bodies asked of it.
type PartKey = (String, u64, Vec<String>, Vec<String>, Option<std::path::PathBuf>);
/// A subassembly document evaluated, and its errors.
type SubLoad = (Doc, Vec<Error>);

/// Loaded documents and the last solve, kept across runs of one assembly.
#[derive(Default)]
pub struct AsmCache {
    parts: BTreeMap<PartKey, Arc<PartLoad>>,
    subs: BTreeMap<(String, u64, Option<std::path::PathBuf>), Arc<SubLoad>>,
    last: Option<Live>,
}

/// A solve kept for the next run: when only drive values change, the solver re-solves only the
/// partitions they reach and the checker re-tests only what moved.
struct Live {
    key: Vec<usize>,
    asm: fa::Assembly,
    solver: fa::Solver<Arc<fa::Catalog>>,
    checker: Checker<Arc<fa::Catalog>, Arc<BrepExact>>,
}

fn text_hash(text: &str) -> u64 {
    crate::build::history_id(text)
}

fn content_hash(text: &str) -> fa::ContentHash {
    let mut out = [0u8; 32];
    for k in 0..4 {
        let h = crate::build::history_id(&format!("{k}:{text}")).to_le_bytes();
        out[k * 8..k * 8 + 8].copy_from_slice(&h);
    }
    fa::ContentHash(out)
}

/// The most a document an assembly reads may be (as for any file a part reads).
pub const LIMIT: u64 = 64 << 20;

/// A part or subassembly file an assembly names, as media.rs resolves files: relative to the
/// naming document's own directory, its real path a `.fg` file of at most `LIMIT` bytes under
/// `fence` (the session's root) when there is one. Every refusal is the same None, so a client
/// learns nothing of what exists outside.
pub(crate) fn locate(
    dir: &Option<std::path::PathBuf>,
    fence: &Option<std::path::PathBuf>,
    file: &str,
) -> Option<std::path::PathBuf> {
    let p = std::fs::canonicalize(dir.as_ref()?.join(file)).ok()?;
    if p.extension().is_none_or(|x| x != "fg") {
        return None;
    }
    let meta = std::fs::metadata(&p).ok()?;
    if !meta.is_file() || meta.len() > LIMIT {
        return None;
    }
    if let Some(f) = fence
        && !p.starts_with(f)
    {
        return None;
    }
    Some(p)
}

/// A located file's text, read no further than `LIMIT`.
pub(crate) fn read_capped(p: &std::path::Path) -> Option<String> {
    use std::io::Read;
    let mut s = String::new();
    std::fs::File::open(p)
        .ok()?
        .take(LIMIT)
        .read_to_string(&mut s)
        .ok()?;
    Some(s)
}

/// The key of a file that could not be located: it reads as missing everywhere.
fn unlocated(file: &str) -> String {
    format!("?{file}")
}

fn located(dir: &Option<std::path::PathBuf>, fence: &Option<std::path::PathBuf>, file: &str) -> String {
    locate(dir, fence, file).map_or_else(|| unlocated(file), |p| p.to_string_lossy().to_string())
}

/// The expression a reference path is in the part: `shank.axis`, `bolt[2].axis`, `vtap.lf`.
fn ref_expr(segs: &[Seg]) -> Expr {
    let sp = Span { lo: 0, hi: 0 };
    let mut e: Option<Expr> = None;
    for s in segs {
        let base = match e.take() {
            None => Expr {
                kind: ExprKind::Name(s.name.clone()),
                span: sp,
            },
            Some(o) => Expr {
                kind: ExprKind::Attr(Box::new(o), s.name.clone()),
                span: sp,
            },
        };
        let mut cur = base;
        for k in &s.index {
            cur = match k {
                Key::Ix(i) => Expr {
                    kind: ExprKind::Index(
                        Box::new(cur),
                        Box::new(Expr {
                            kind: ExprKind::Num(*i as f64, Unit::None),
                            span: sp,
                        }),
                    ),
                    span: sp,
                },
                Key::Label(l) => Expr {
                    kind: ExprKind::Attr(Box::new(cur), l.clone()),
                    span: sp,
                },
            };
        }
        e = Some(cur);
    }
    e.expect("a reference has a name")
}

/// A value a part's reference evaluated to, as a reference.
fn as_reference(ev: &mut Eval, v: Value) -> Option<fa::Reference> {
    let span = Span { lo: 0, hi: 0 };
    match &v {
        Value::Asm(a) => match &**a {
            AsmVal::Anchor { reference, .. } => as_reference(ev, reference.clone()),
            _ => None,
        },
        Value::Plane(p) => Some(fa::Reference::frame(p.frame)),
        // a feature's cap or face by role (`bar.end`): its plane
        Value::Faces(f) => f.plane.map(fa::Reference::frame),
        Value::Axis(..) | Value::Line(..) => {
            let (o, d) = axis_of(ev, v, span).ok()?;
            Some(fa::Reference::axis(o, d))
        }
        Value::Point(_) | Value::Ent(_) => Some(fa::Reference::point(point_of(ev, v, span).ok()?)),
        Value::Wired(w) => {
            if let Some(f) = w.plane() {
                Some(fa::Reference::frame(f))
            } else if let Some((o, d)) = w.axis() {
                Some(fa::Reference::axis(o, d))
            } else {
                w.point().map(fa::Reference::point)
            }
        }
        _ => None,
    }
}

/// The body an assembly places of a part by default: the part as made, less any flat pattern an
/// `unfold` made of a sheet (that is the drawing's, not the assembly's; `body:` still picks it).
fn placed_body(ev: &Eval) -> Option<forge_topo::Body> {
    let flats: Vec<usize> = ev
        .sheets
        .list
        .iter()
        .flat_map(|st| &st.unfolds)
        .filter_map(|u| ev.feats.iter().find(|f| f.id == u.id).map(|f| f.idx))
        .collect();
    let st = ev.b.stages.last()?;
    if flats.is_empty() || !st.solids.iter().any(|s| flats.contains(&s.id)) {
        return ev.b.body().cloned();
    }
    let kept: Vec<&forge_model::Solid> = st.solids.iter().filter(|s| !flats.contains(&s.id)).collect();
    match &kept[..] {
        [] => None,
        [one] => Some(forge_model::finish(one.body.clone())),
        many => {
            let mut all = forge_topo::Body::new();
            for s in many {
                all.append(&forge_model::finish(s.body.clone()));
            }
            Some(all)
        }
    }
}

fn load_part(
    text: &str,
    dir: Option<std::path::PathBuf>,
    fence: Option<std::path::PathBuf>,
    refs: &[String],
    bodies: &[String],
) -> PartLoad {
    let hash = content_hash(text);
    let mut out = PartLoad {
        hash,
        body: None,
        refs: BTreeMap::new(),
        features: vec![],
        kinds: vec![],
        errors: vec![],
        mesh: None,
        bodies: BTreeMap::new(),
    };
    let prog = match crate::syntax::parse(text) {
        Ok(p) => p,
        Err(e) => {
            out.errors.push(e);
            return out;
        }
    };
    if declared(&prog) {
        return out;
    }
    let mut ev = Eval::new(text, crate::eval::Rules::default(), true, Default::default());
    ev.dir = dir;
    ev.root = fence;
    ev.check_static(&prog);
    if !ev.errors.is_empty() {
        ev.b.building = false;
    }
    ev.run(&prog);
    crate::sheet::finish(&mut ev);
    out.body = placed_body(&ev);
    if let Some(st) = ev.b.stages.last() {
        for name in bodies {
            if let Some(f) = ev.feats.iter().find(|f| &f.id == name)
                && let Some(s) = st.solids.iter().find(|s| s.id == f.idx)
            {
                out.bodies.insert(name.clone(), s.body.clone());
            }
        }
    }
    // declared anchors and kinds, made in loops too (`seat[3]`, `tap.lf`)
    let mut todo: Vec<(String, Value)> = ev.globals();
    while let Some((name, v)) = todo.pop() {
        match &v {
            Value::Map(m) => {
                for (k, x) in m.iter() {
                    let n = match k {
                        Key::Ix(i) => format!("{name}[{i}]"),
                        Key::Label(l) => format!("{name}[{l}]"),
                    };
                    todo.push((n, x.clone()));
                }
            }
            Value::Asm(a) => match &**a {
                AsmVal::Anchor { reference, feature } => {
                    let r = as_reference(&mut ev, reference.clone());
                    out.refs.insert(
                        name.clone(),
                        r.ok_or(fa::AnchorFault::Degenerate { entity: name.clone() }),
                    );
                    if let Some(f) = feature {
                        out.features.push((name.clone(), f.clone()));
                    }
                }
                AsmVal::Kind(fa::FeatureKind::Declared(k)) => out.kinds.push(k.clone()),
                _ => {}
            },
            _ => {}
        }
    }
    out.features.sort_by(|a, b| a.0.cmp(&b.0));
    for r in refs {
        if out.refs.contains_key(r) {
            continue;
        }
        let Some(segs) = crate::error::parse_path(r) else {
            continue;
        };
        let e = ref_expr(&segs);
        if let Ok(v) = ev.expr(&e) {
            match as_reference(&mut ev, v) {
                Some(x) => {
                    out.refs.insert(r.clone(), Ok(x));
                }
                None => {
                    out.refs
                        .insert(r.clone(), Err(fa::AnchorFault::Degenerate { entity: r.clone() }));
                }
            }
        }
    }
    out.errors = std::mem::take(&mut ev.errors);
    if let Some(b) = &out.body {
        let q = forge_mesh::Quality {
            chord: MESH_CHORD,
            max_angle: forge_math::dmath::PI / 12.0,
        };
        if let Ok(m) = forge_mesh::tessellate(b, &q) {
            out.mesh = Some(Arc::new(PartMesh::from_trimesh(&m, MESH_CHORD)));
        }
    }
    out
}

/// The interference meshes' chord tolerance (mm): overlaps shallower than about twice this may
/// go unseen, and the report says so per pair.
pub const MESH_CHORD: f64 = 0.01;

fn load_sub(
    text: &str,
    dir: Option<std::path::PathBuf>,
    fence: Option<std::path::PathBuf>,
) -> (Doc, Vec<Error>) {
    let prog = match crate::syntax::parse(text) {
        Ok(p) => p,
        Err(e) => return (Doc::default(), vec![e]),
    };
    let mut ev = Eval::new(text, crate::eval::Rules::default(), false, Default::default());
    ev.dir = dir;
    ev.root = fence;
    ev.asm = Some(Doc::default());
    ev.check_static(&prog);
    ev.run(&prog);
    (ev.asm.take().unwrap_or_default(), std::mem::take(&mut ev.errors))
}

// ---- solving ------------------------------------------------------------------------------------

/// Times of the assembly's own passes (ms).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AsmTimings {
    pub load_ms: f64,
    pub anchors_ms: f64,
    pub solve_ms: f64,
    pub overlap_ms: f64,
    pub expects_ms: f64,
    /// Whether the solve and the overlap check reused the last run's (an edit of drives only).
    pub incremental: bool,
    /// Runs made for `expect .. at`: one per distinct set of params.
    pub at_runs: usize,
}

/// One joint's values after the solve.
#[derive(Clone, Debug, PartialEq)]
pub struct JointOut {
    pub id: String,
    pub kind: &'static str,
    pub values: Vec<(&'static str, f64, bool)>,
    pub free: usize,
    /// Its limits as written (freedom, low, high; angles in radians): a drive's range.
    pub limits: Vec<(&'static str, f64, f64)>,
}

/// What an assembly run gives besides its typed notes.
#[derive(Clone, Debug, Default)]
pub struct AsmOut {
    /// Each top-level instance's solved pose, by id.
    pub poses: Vec<(String, Frame)>,
    /// Each part occurrence (through subassemblies), its file and pose.
    pub occurrences: Vec<(String, String, Frame)>,
    pub joints: Vec<JointOut>,
    pub mobility: usize,
    pub interferences: usize,
    pub intended: usize,
    pub rules: Vec<(String, usize)>,
    /// Parts by file, multiplied out through subassemblies.
    pub bom: Vec<(String, usize)>,
    pub hashes: Vec<(String, String)>,
    pub timings: AsmTimings,
    pub stats: fa::Stats,
    pub overlap: Option<OverlapStats>,
    /// The parts to draw: each occurrence's body and pose.
    pub draw: Vec<(Arc<forge_topo::Body>, Frame)>,
    /// With `Options::render`: each part's body once, and each occurrence's (by `occurrences`'
    /// order) index into it, so a picture meshes a part once and draws it at every occurrence.
    pub parts: Vec<Arc<forge_topo::Body>>,
    pub occ_part: Vec<Option<usize>>,
    /// The probed expects' values, by position (`Options::probe`).
    pub probes: Vec<(usize, Option<f64>)>,
    /// Every expect line: measured, against its limit, passed or not.
    pub expects: Vec<ExpectOut>,
    /// Every rule-covered thread pair's engagement, short or not.
    pub engagements: Vec<EngagementOut>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExpectOut {
    pub line: usize,
    pub metric: String,
    /// None when it could not be measured (then an `ExpectUnevaluable` error says why).
    pub measured: Option<f64>,
    pub limit: Option<f64>,
    pub ok: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EngagementOut {
    pub a: String,
    pub b: String,
    pub rule: String,
    pub engaged: f64,
    pub min: Option<f64>,
    pub short: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OverlapStats {
    pub candidates: usize,
    pub narrow_tests: usize,
    pub cache_hits: usize,
    pub exact_runs: usize,
}

/// Ids to names, and names to the statements that made them.
#[derive(Default)]
struct Names {
    inst: BTreeMap<u64, (String, Span)>,
    link: BTreeMap<u64, (String, Span, Option<JointKind>)>,
    rel: BTreeMap<u64, (String, Span)>,
    rule: BTreeMap<u64, (String, Span)>,
    fits: Vec<(String, Span)>,
    /// Each probed reference's path and anchor.
    probe: Vec<(RefDef, fa::End)>,
    /// Instances inside subassemblies, by id hash: occurrence paths read as names.
    sub_inst: BTreeMap<u64, String>,
}

fn hid(id: &str) -> u64 {
    crate::build::history_id(id)
}

/// A document's id: its path below the top document's directory, where it is below it.
fn short(root: &Option<std::path::PathBuf>, path: &str) -> String {
    root.as_ref()
        .and_then(|r| std::path::Path::new(path).strip_prefix(r).ok())
        .map_or_else(|| path.to_string(), |p| p.to_string_lossy().to_string())
}

/// The document's assembly for forge-asm, and the references it asks of each part file.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn build_doc(
    d: &Doc,
    root: &Option<std::path::PathBuf>,
    fence: &Option<std::path::PathBuf>,
    dir: &Option<std::path::PathBuf>,
    subs: &BTreeMap<String, Arc<SubLoad>>,
    want: &mut BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)>,
    names: &mut Names,
) -> fa::Assembly {
    let mut out = fa::Assembly::default();
    let key = |f: &str| located(dir, fence, f);
    let doc_id = |f: &str| fa::DocId(short(root, &key(f)).into());
    for i in &d.insts {
        let file = key(&i.file);
        names.inst.insert(hid(&i.id), (i.id.clone(), i.span));
        let of = if i.sub {
            fa::Component::Assembly(doc_id(&i.file))
        } else {
            let e = want.entry(file.clone()).or_default();
            if let Some(b) = &i.body {
                e.1.insert(b.clone());
            }
            fa::Component::Part(fa::PartRef {
                doc: doc_id(&i.file),
                body: i.body.as_deref().map_or(0, hid),
                pin: None,
            })
        };
        out.instances.push(fa::Instance {
            id: fa::InstanceId(hid(&i.id)),
            of,
            pose: i.pose,
            ground: i.ground,
            suppressed: i.suppressed,
            reference: i.reference,
        });
    }
    // a reference's path through subassemblies to a part, and its name in that part
    let end = |r: &RefDef, want: &mut BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)>| -> fa::End {
        let mut path = vec![fa::InstanceId(hid(&r.inst))];
        let mut cur = d.insts.iter().find(|x| x.id == r.inst);
        let mut cur_dir = dir.clone();
        let mut segs: &[Seg] = &r.segs;
        while let Some(c) = cur.filter(|c| c.sub) {
            let Some(file) = locate(&cur_dir, fence, &c.file) else {
                break;
            };
            let Some(sub) = subs.get(&file.to_string_lossy().to_string()) else {
                break;
            };
            let Some(first) = segs.first() else { break };
            let text = path_text(std::slice::from_ref(first));
            match sub.0.insts.iter().find(|x| x.id == text) {
                Some(x) => {
                    path.push(fa::InstanceId(hid(&x.id)));
                    cur = Some(x);
                    cur_dir = file.parent().map(|p| p.to_path_buf());
                    segs = &segs[1..];
                }
                None => break,
            }
        }
        let anchor = if segs.is_empty() {
            fa::AnchorRef::Local(fa::Reference::frame(Frame::WORLD))
        } else {
            let text = path_text(segs);
            if let Some(c) = cur.filter(|c| !c.sub) {
                want.entry(located(&cur_dir, fence, &c.file))
                    .or_default()
                    .0
                    .insert(text.clone());
            }
            fa::AnchorRef::Named(text)
        };
        fa::End { path, anchor }
    };
    for r in &d.probes {
        let e = end(r, want);
        names.probe.push((r.clone(), e));
    }
    for (n, l) in d.links.iter().enumerate() {
        let a = end(&l.a, want);
        let b = end(&l.b, want);
        let id = hid(&l.id);
        // blame in document order: of a conflict the later statement is left out
        out.rank.insert(id, n as u64);
        match l.kind {
            LinkKind::Joint(kind) => {
                names.link.insert(id, (l.id.clone(), l.span, Some(kind)));
                out.joints.push(fa::Joint {
                    id: fa::JointId(id),
                    kind,
                    a,
                    b,
                    flip: l.flip,
                    drive: l.drive.iter().map(|(f, n)| (*f, n.v)).collect(),
                    limits: l.limits.clone(),
                    suppressed: false,
                });
            }
            LinkKind::Constraint(kind) => {
                names.link.insert(id, (l.id.clone(), l.span, None));
                out.constraints.push(fa::Constraint {
                    id,
                    kind,
                    a,
                    b,
                    flip: l.flip,
                    suppressed: false,
                });
            }
        }
    }
    for (n, r) in d.relations.iter().enumerate() {
        let id = hid(&r.id);
        out.rank.insert(id, (d.links.len() + n) as u64);
        names.rel.insert(id, (r.id.clone(), r.span));
        out.relations.push(fa::Relation {
            id: fa::RelationId(id),
            kind: r.kind,
            leader: fa::ValueRef {
                joint: fa::JointId(hid(&r.leader.0)),
                freedom: r.leader.1,
            },
            follower: fa::ValueRef {
                joint: fa::JointId(hid(&r.follower.0)),
                freedom: r.follower.1,
            },
            offset: None,
            suppressed: false,
        });
    }
    for f in &d.fits {
        names.fits.push((f.id.clone(), f.span));
        out.fits.push(fa::Fit {
            a: vec![fa::InstanceId(hid(&f.a))],
            b: vec![fa::InstanceId(hid(&f.b))],
        });
    }
    for r in &d.rules {
        let id = hid(&r.id);
        names.rule.insert(id, (r.id.clone(), r.span));
        let m = |m: &Matcher| match m {
            Matcher::Kind(k) => fa::FitMatch::Feature {
                kind: k.clone(),
                size: r.size,
            },
            Matcher::File(f) => fa::FitMatch::Part {
                doc: doc_id(f),
                body: None,
            },
        };
        out.fit_rules.push(fa::FitRule {
            id,
            a: m(&r.a),
            b: m(&r.b),
            min_engagement: r.min_engagement,
        });
    }
    out
}

/// Solves an evaluated assembly document: loads its parts and subassemblies (reusing `cache`),
/// solves, checks interference and the document's expects, and turns every result into typed
/// errors and warnings on `ev`.
pub fn finish(
    ev: &mut Eval,
    opts: &crate::Options,
    cache: Option<Box<AsmCache>>,
) -> (Option<AsmOut>, Option<Box<AsmCache>>) {
    let Some(d) = ev.asm.take() else {
        return (None, cache);
    };
    let mut cache = cache.unwrap_or_default();
    if !opts.build || !ev.errors.is_empty() {
        ev.asm = Some(d);
        return (None, Some(cache));
    }
    let mut t = AsmTimings::default();
    let t0 = Instant::now();
    let dir = opts.dir.clone();
    // the session's root: nothing outside it is read
    let fence = opts.root.as_ref().and_then(|r| std::fs::canonicalize(r).ok());
    let mut names = Names::default();

    // subassemblies, depth first, each document once; a cycle is refused
    let mut subs: BTreeMap<String, Arc<SubLoad>> = BTreeMap::new();
    let mut stack: Vec<(Doc, Option<std::path::PathBuf>, Vec<String>)> =
        vec![(d.clone(), dir.clone(), vec![])];
    while let Some((doc_, here, chain)) = stack.pop() {
        for i in doc_.insts.iter().filter(|i| i.sub) {
            let Some(p) = locate(&here, &fence, &i.file) else {
                ev.errors.push(note(
                    i.span,
                    Issue::PartFile {
                        file: i.file.clone(),
                        problem: FileProblem::Missing,
                    },
                ));
                continue;
            };
            let key = p.to_string_lossy().to_string();
            if chain.contains(&key) {
                ev.errors.push(note(
                    i.span,
                    Issue::PartFile {
                        file: i.file.clone(),
                        problem: FileProblem::Cyclic,
                    },
                ));
                continue;
            }
            if subs.contains_key(&key) {
                continue;
            }
            let Some(text) = read_capped(&p) else {
                ev.errors.push(note(
                    i.span,
                    Issue::PartFile {
                        file: i.file.clone(),
                        problem: FileProblem::Missing,
                    },
                ));
                continue;
            };
            let parent = p.parent().map(|x| x.to_path_buf());
            let ck = (key.clone(), text_hash(&text), fence.clone());
            let loaded = match cache.subs.get(&ck) {
                Some(x) => x.clone(),
                None => {
                    let is_asm = crate::syntax::parse(&text).is_ok_and(|p| declared(&p));
                    if !is_asm {
                        ev.errors.push(note(
                            i.span,
                            Issue::PartFile {
                                file: i.file.clone(),
                                problem: FileProblem::WrongKind { assembly: false },
                            },
                        ));
                        continue;
                    }
                    let x = Arc::new(load_sub(&text, parent.clone(), fence.clone()));
                    cache.subs.insert(ck, x.clone());
                    x
                }
            };
            if let Some(e) = loaded.1.first() {
                ev.errors.push(note(
                    i.span,
                    Issue::PartFile {
                        file: i.file.clone(),
                        problem: FileProblem::Errors {
                            count: loaded.1.len(),
                            first: e.kind(),
                        },
                    },
                ));
            }
            subs.insert(key.clone(), loaded.clone());
            let mut c = chain.clone();
            c.push(key);
            stack.push((loaded.0.clone(), parent, c));
        }
    }

    for (doc, _) in subs.values().map(|x| &**x) {
        for i in &doc.insts {
            names.sub_inst.insert(hid(&i.id), i.id.clone());
        }
    }
    // the documents for forge-asm, and what each part file is asked for
    let mut want: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    // the references expects measure, so the parts are asked for them
    let mut d = d;
    for (k, s) in ev.expects.clone().iter().enumerate() {
        if let StmtKind::Expect { args, .. } = &s.kind {
            let mark = ev.enter_expect(k);
            let refs: Vec<Value> = args.iter().filter_map(|e| ev.expr(e).ok()).collect();
            ev.leave_expect(mark);
            for v in refs {
                if let Value::Asm(a) = v
                    && let AsmVal::Ref(r) = &*a
                {
                    d.probes.push(r.clone());
                }
            }
        }
    }
    let top = build_doc(&d, &dir, &fence, &dir, &subs, &mut want, &mut names);
    let mut catalog = fa::Catalog::default();
    for (key, sub) in &subs {
        let sdir = std::path::Path::new(key).parent().map(|p| p.to_path_buf());
        let mut sn = Names::default();
        let a = build_doc(&sub.0, &dir, &fence, &sdir, &subs, &mut want, &mut sn);
        catalog.assemblies.insert(fa::DocId(short(&dir, key).into()), a);
    }

    // parts: built once per text and question, by file
    let mut exact = BrepExact::default();
    let mut loads: Vec<usize> = vec![];
    let mut declared_kinds: BTreeSet<String> = BTreeSet::new();
    for (file, (refs, bodies)) in &want {
        // named as the document wrote it, never by its real path
        let named = d
            .insts
            .iter()
            .find(|i| !i.sub && located(&dir, &fence, &i.file) == *file);
        let span = named.map_or(Span { lo: 0, hi: 0 }, |i| i.span);
        let shown = named.map_or_else(
            || {
                std::path::Path::new(file.trim_start_matches('?'))
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().to_string())
            },
            |i| i.file.clone(),
        );
        let text = if file.starts_with('?') {
            None
        } else {
            read_capped(std::path::Path::new(file))
        };
        let Some(text) = text else {
            ev.errors.push(note(
                span,
                Issue::PartFile {
                    file: shown,
                    problem: FileProblem::Missing,
                },
            ));
            continue;
        };
        let refs: Vec<String> = refs.iter().cloned().collect();
        let bodies: Vec<String> = bodies.iter().cloned().collect();
        let ck = (
            file.clone(),
            text_hash(&text),
            refs.clone(),
            bodies.clone(),
            fence.clone(),
        );
        let load = match cache.parts.get(&ck) {
            Some(l) => l.clone(),
            None => {
                let parent = std::path::Path::new(file).parent().map(|p| p.to_path_buf());
                let l = Arc::new(load_part(&text, parent, fence.clone(), &refs, &bodies));
                cache.parts.insert(ck, l.clone());
                l
            }
        };
        loads.push(Arc::as_ptr(&load) as usize);
        if crate::syntax::parse(&text).is_ok_and(|p| declared(&p)) {
            ev.errors.push(note(
                span,
                Issue::PartFile {
                    file: shown.clone(),
                    problem: FileProblem::WrongKind { assembly: true },
                },
            ));
            continue;
        }
        if let Some(e) = load.errors.first() {
            ev.errors.push(note(
                span,
                Issue::PartFile {
                    file: shown.clone(),
                    problem: FileProblem::Errors {
                        count: load.errors.len(),
                        first: e.kind(),
                    },
                },
            ));
        }
        declared_kinds.extend(load.kinds.iter().cloned());
        let doc = fa::DocId(short(&dir, file).into());
        let mut entries: Vec<(u64, Option<forge_topo::Body>)> = vec![(0, load.body.clone())];
        for (name, b) in &load.bodies {
            entries.push((hid(name), Some(b.clone())));
        }
        for b in bodies.iter().filter(|b| !load.bodies.contains_key(*b)) {
            ev.errors.push(note(
                span,
                Issue::PartFile {
                    file: shown.clone(),
                    problem: FileProblem::NoBody { body: b.clone() },
                },
            ));
        }
        for (k, body) in entries {
            let mesh = if k == 0 {
                load.mesh.clone()
            } else {
                body.as_ref().and_then(|b| {
                    let q = forge_mesh::Quality {
                        chord: MESH_CHORD,
                        max_angle: forge_math::dmath::PI / 12.0,
                    };
                    forge_mesh::tessellate(b, &q)
                        .ok()
                        .map(|m| Arc::new(PartMesh::from_trimesh(&m, MESH_CHORD)))
                })
            };
            catalog.parts.insert(
                (doc.clone(), k),
                fa::PartInfo {
                    hash: Some(load.hash),
                    refs: load.refs.clone(),
                    features: load.features.clone(),
                    mesh,
                },
            );
            if let Some(b) = body {
                exact.bodies.insert((doc.clone(), k), b);
            }
        }
    }
    for r in &d.rules {
        for m in [&r.a, &r.b] {
            if let Matcher::Kind(fa::FeatureKind::Declared(k)) = m
                && !declared_kinds.contains(k)
            {
                ev.errors.push(note(
                    r.span,
                    Issue::UnknownKind {
                        kind: k.clone(),
                        declared: declared_kinds.iter().cloned().collect(),
                    },
                ));
            }
        }
    }
    t.load_ms = t0.elapsed().as_secs_f64() * 1e3;
    if !ev.errors.is_empty() {
        return (None, Some(cache));
    }

    // the anchor pass, then the solve: incremental when only drives changed
    let t1 = Instant::now();
    let broken = fa::check_anchors(&top, &catalog);
    t.anchors_ms = t1.elapsed().as_secs_f64() * 1e3;
    let t2 = Instant::now();
    // the same document but for drive values: compared in place, nothing cloned
    let same_drives_only = |old: &fa::Assembly, new: &fa::Assembly| {
        old.instances == new.instances
            && old.constraints == new.constraints
            && old.relations == new.relations
            && old.fits == new.fits
            && old.fit_rules == new.fit_rules
            && old.rank == new.rank
            && old.joints.len() == new.joints.len()
            && old.joints.iter().zip(&new.joints).all(|(a, b)| {
                a.id == b.id
                    && a.kind == b.kind
                    && a.a == b.a
                    && a.b == b.b
                    && a.flip == b.flip
                    && a.limits == b.limits
                    && a.suppressed == b.suppressed
                    && a.drive.len() == b.drive.len()
                    && a.drive.iter().zip(&b.drive).all(|(x, y)| x.0 == y.0)
            })
    };
    let live = cache
        .last
        .take()
        .filter(|l| l.key == loads && same_drives_only(&l.asm, &top));
    let mut live = match live {
        Some(mut l) => {
            let mut changed = vec![];
            for (jo, jn) in l.asm.joints.iter().zip(&top.joints) {
                for (a, b) in jo.drive.iter().zip(&jn.drive) {
                    if a.1.to_bits() != b.1.to_bits() {
                        changed.push((jn.id, b.0, b.1));
                    }
                }
            }
            for (j, f, v) in changed {
                l.solver.set_drive(j, f, v);
            }
            l.asm = top;
            t.incremental = true;
            l
        }
        None => {
            let catalog = Arc::new(catalog);
            let exact = Arc::new(exact);
            Live {
                key: loads,
                solver: fa::Solver::new(top.clone(), catalog.clone(), fa::Options::default()),
                asm: top,
                checker: Checker::new(catalog, exact),
            }
        }
    };
    let Live {
        solver,
        checker,
        asm: live_asm,
        ..
    } = &mut live;
    let sol = solver.solution();
    t.solve_ms = t2.elapsed().as_secs_f64() * 1e3;
    let t3 = Instant::now();
    // the overlap check can wait (`Options::overlap`): a UI shows the posed parts first
    let rep = if opts.overlap {
        checker.interference(sol, &live_asm.fits, &live_asm.fit_rules)
    } else {
        fa::check::InterferenceReport::default()
    };
    t.overlap_ms = t3.elapsed().as_secs_f64() * 1e3;

    // typed results
    let _ = broken;
    for dg in &sol.diagnostics {
        push_diag(ev, &names, dg, &d);
    }
    for dg in rep.diagnostics() {
        push_diag(ev, &names, &dg, &d);
    }

    // each item's status: built, or failed where an error names it
    let failed: BTreeSet<String> = ev
        .errors
        .iter()
        .filter_map(|e| match e {
            Error::Asm(n) => match &n.issue {
                Issue::Conflict { culprit, .. } => Some(culprit.clone()),
                Issue::BrokenAnchor { item, .. }
                | Issue::WrongReference { item, .. }
                | Issue::SelfLink { item } => Some(item.clone()),
                Issue::LimitViolated { joint, .. } => Some(joint.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    for f in ev.feats.iter_mut().filter(|f| f.idx == usize::MAX) {
        f.status = if failed.contains(&f.id) {
            crate::eval::Status::Failed
        } else {
            crate::eval::Status::Ok
        };
    }
    let mut out = AsmOut {
        mobility: sol.mobility,
        interferences: rep.interferences.len(),
        intended: rep.intended.len(),
        rules: rep
            .rules
            .iter()
            .map(|r| {
                (
                    names
                        .rule
                        .get(&r.rule)
                        .map_or_else(|| r.rule.to_string(), |x| x.0.clone()),
                    r.pairs.len(),
                )
            })
            .collect(),
        stats: sol.stats,
        overlap: opts.overlap.then_some(OverlapStats {
            candidates: rep.candidates,
            narrow_tests: rep.narrow_tests,
            cache_hits: rep.cache_hits,
            exact_runs: rep.exact_runs,
        }),
        ..AsmOut::default()
    };
    for e in &rep.engagements {
        out.engagements.push(EngagementOut {
            a: path_name(&names, &e.a),
            b: path_name(&names, &e.b),
            rule: names
                .rule
                .get(&e.rule)
                .map_or_else(|| e.rule.to_string(), |x| x.0.clone()),
            engaged: e.engaged,
            min: e.min,
            short: e.short,
        });
    }
    for (id, f) in &sol.poses {
        if let Some((n, _)) = names.inst.get(&id.0) {
            out.poses.push((n.clone(), *f));
        }
    }
    out.poses.sort_by(|a, b| a.0.cmp(&b.0));
    let occ_name = |p: &fa::Path| -> String {
        // the top-level instance, then the subassemblies' instances, by their names
        path_name(&names, p)
    };
    let mut bom: BTreeMap<String, usize> = BTreeMap::new();
    let mut unique = std::collections::HashMap::new();
    for o in &sol.occurrences {
        out.occurrences
            .push((occ_name(&o.path), o.part.doc.0.to_string(), o.pose));
        if !o.reference {
            *bom.entry(o.part.doc.0.to_string()).or_default() += 1;
        }
        if opts.render
            && let Some(b) = checker.exact().bodies.get(&(o.part.doc.clone(), o.part.body))
        {
            // each part's body once, shared by its occurrences
            let key = (o.part.doc.clone(), o.part.body);
            let k = match unique.get(&key) {
                Some(&k) => k,
                None => {
                    out.parts.push(Arc::new(b.clone()));
                    unique.insert(key, out.parts.len() - 1);
                    out.parts.len() - 1
                }
            };
            out.draw.push((out.parts[k].clone(), o.pose));
            out.occ_part.push(Some(k));
        } else {
            out.occ_part.push(None);
        }
    }
    out.bom = bom.into_iter().collect();
    out.hashes = sol
        .hashes
        .iter()
        .map(|(p, h)| {
            (
                p.doc.0.to_string(),
                h.map_or_else(String::new, |h| h.0.iter().map(|b| format!("{b:02x}")).collect()),
            )
        })
        .collect();
    for j in &sol.joints {
        let Some((n, _, Some(kind))) = names.link.get(&j.joint.0) else {
            continue;
        };
        let limits = d
            .links
            .iter()
            .find(|l| l.id == *n)
            .map(|l| {
                l.limits
                    .iter()
                    .map(|(f, lo, hi)| (freedom_word(*kind, *f), *lo, *hi))
                    .collect()
            })
            .unwrap_or_default();
        out.joints.push(JointOut {
            limits,
            id: n.clone(),
            kind: kind.name(),
            values: j
                .values
                .iter()
                .map(|(f, v)| (freedom_word(*kind, *f), *v, f.is_angle()))
                .collect(),
            free: j.free,
        });
    }

    // expects
    let t4 = Instant::now();
    let catalog = checker.parts().clone();
    // a UI may leave them for later (`Options::expects`): the `at` ones run the document again
    let expects = if opts.expects || opts.probe.is_some() {
        ev.expects.clone()
    } else {
        vec![]
    };
    // the expects measured on this solve (all of them when probing for an `at` run)
    let mut direct: BTreeMap<usize, Result<f64, Issue>> = BTreeMap::new();
    for (k, s) in expects.iter().enumerate() {
        let StmtKind::Expect { metric, args, at, .. } = &s.kind else {
            continue;
        };
        let wanted = match &opts.probe {
            Some(ps) => ps.contains(&k),
            None => at.is_empty(),
        };
        if !wanted {
            continue;
        }
        let mut m = Measure {
            names: &names,
            sol,
            catalog: &catalog,
            checker,
            asm: live_asm,
        };
        let mark = ev.enter_expect(k);
        let v = m.measure(ev, metric, args);
        ev.leave_expect(mark);
        match &opts.probe {
            Some(_) => out.probes.push((k, v.ok())),
            None => {
                direct.insert(k, v);
            }
        }
    }
    // this solve is where the `at` runs start: each re-solves and re-checks only what its
    // drives move
    let mut live = Some(live);
    // expects `at` the same params share one run: the document with those values, the parts
    // from this run's cache (only the drives differ), every expect of the set measured at once
    let mut at_values: BTreeMap<usize, Option<f64>> = BTreeMap::new();
    if opts.probe.is_none() {
        let mut sets: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (k, s) in expects.iter().enumerate() {
            let StmtKind::Expect { at, .. } = &s.kind else {
                continue;
            };
            if at.is_empty() {
                continue;
            }
            let mark = ev.enter_expect(k);
            let text = at_text(ev, at);
            ev.leave_expect(mark);
            match text {
                Some(text) => sets.entry(text).or_default().push(k),
                None => {
                    at_values.insert(k, None);
                }
            }
        }
        for (text, ks) in sets {
            let o = crate::Options {
                probe: Some(ks.clone()),
                ..opts.clone()
            };
            if let Some(l) = live.take() {
                cache.last = Some(l);
            }
            let mut kept = crate::build::Cache::default();
            kept.asm = Some(std::mem::take(&mut cache));
            let (run, back) = crate::run(&text, o, kept);
            cache = back.asm.unwrap_or_default();
            t.at_runs += 1;
            let probed = run.asm.map(|a| a.probes).unwrap_or_default();
            for k in ks {
                let v = probed.iter().find(|p| p.0 == k).and_then(|p| p.1);
                at_values.insert(k, v);
            }
        }
    }
    for (k, s) in expects.iter().enumerate() {
        let mark = ev.enter_expect(k);
        'one: {
            let StmtKind::Expect {
                metric,
                args: _,
                cmp,
                at,
            } = &s.kind
            else {
                break 'one;
            };
            if opts.probe.is_some() {
                break 'one;
            }
            let line = s.span.lines(ev.src).0;
            let unevaluable = |why: Unevaluable| Issue::ExpectUnevaluable {
                metric: metric.clone(),
                why,
            };
            let measured = if at.is_empty() {
                direct
                    .remove(&k)
                    .unwrap_or(Err(unevaluable(Unevaluable::AtFailed)))
            } else {
                at_values
                    .get(&k)
                    .copied()
                    .flatten()
                    .ok_or_else(|| unevaluable(Unevaluable::AtFailed))
            };
            let measured = match measured {
                Ok(v) => v,
                Err(issue) => {
                    ev.errors.push(note(s.span, issue));
                    out.expects.push(ExpectOut {
                        line,
                        metric: metric.clone(),
                        measured: None,
                        limit: None,
                        ok: false,
                    });
                    break 'one;
                }
            };
            let Some((c, v, tol)) = cmp else {
                out.expects.push(ExpectOut {
                    line,
                    metric: metric.clone(),
                    measured: Some(measured),
                    limit: None,
                    ok: true,
                });
                break 'one;
            };
            let limit = match ev.expr(v) {
                Ok(Value::Num(n)) => n.v,
                _ => {
                    ev.errors.push(note(s.span, unevaluable(Unevaluable::NoLimit)));
                    out.expects.push(ExpectOut {
                        line,
                        metric: metric.clone(),
                        measured: Some(measured),
                        limit: None,
                        ok: false,
                    });
                    break 'one;
                }
            };
            let tol = match tol {
                Some((x, pct)) => match ev.expr(x) {
                    Ok(Value::Num(n)) => {
                        if *pct {
                            limit.abs() * n.v / 100.0
                        } else {
                            n.v
                        }
                    }
                    _ => 0.0,
                },
                None => 0.0,
            };
            let ok = match c {
                Cmp::Eq => (measured - limit).abs() <= tol.max(1e-9 * limit.abs().max(1.0)),
                Cmp::Lt => measured < limit + tol,
                Cmp::Le => measured <= limit + tol,
                Cmp::Gt => measured > limit - tol,
                Cmp::Ge => measured >= limit - tol,
            };
            out.expects.push(ExpectOut {
                line,
                metric: metric.clone(),
                measured: Some(measured),
                limit: Some(limit),
                ok,
            });
            if !ok {
                ev.errors.push(note(
                    s.span,
                    Issue::ExpectFailed {
                        metric: metric.clone(),
                        measured,
                        cmp: *c,
                        limit,
                    },
                ));
            }
        }
        ev.leave_expect(mark);
    }
    t.expects_ms = t4.elapsed().as_secs_f64() * 1e3;
    out.timings = t;
    if let Some(l) = live {
        cache.last = Some(l);
    }
    (Some(out), Some(cache))
}

/// The document with an `expect`'s `at P = v` params set: what it is measured on. None when a
/// value is not a number here, or a param is not the document's.
fn at_text(ev: &mut Eval, at: &[(Vec<Seg>, Expr)]) -> Option<String> {
    let mut text = ev.src.to_string();
    for (path, e) in at {
        let (last, owner) = path.split_last()?;
        let param = ParamKey {
            owner: owner.to_vec(),
            name: last.name.clone(),
        };
        let n = match ev.expr(e) {
            Ok(Value::Num(n)) => n,
            _ => return None,
        };
        let value = Literal {
            value: if n.dim == Dim::Angle {
                n.v.to_degrees()
            } else {
                n.v
            },
            dim: if n.dim == Dim::Any { Dim::Length } else { n.dim },
        };
        text = crate::edit::apply(&text, &Op::SetParam { param, value }).ok()?;
    }
    Some(text)
}

/// What an expect measures with.
struct Measure<'a> {
    names: &'a Names,
    sol: &'a fa::Solution,
    catalog: &'a fa::Catalog,
    checker: &'a mut Checker<Arc<fa::Catalog>, Arc<BrepExact>>,
    asm: &'a fa::Assembly,
}

impl Measure<'_> {
    /// An argument as a place: an instance, or a reference on one (an anchor, a role name),
    /// with the occurrence it is on and its world frame.
    /// The part occurrences at or under a path: one part, or every part of a subassembly.
    fn parts_under(&self, p: &fa::Path) -> Vec<&fa::Occurrence> {
        self.sol
            .occurrences
            .iter()
            .filter(|o| o.path.starts_with(p))
            .collect()
    }

    fn place(&self, ev: &mut Eval, args: &[Expr], index: usize) -> Result<(fa::Path, Frame), Unevaluable> {
        let e = args.get(index).ok_or(Unevaluable::MissingArg { index })?;
        let v = match ev.expr(e) {
            Ok(v) => v,
            Err(err) => {
                ev.errors.push(err);
                return Err(Unevaluable::NotAPlace {
                    found: Ty::Unresolved,
                });
            }
        };
        let Value::Asm(a) = &v else {
            return Err(Unevaluable::NotAPlace { found: v.ty() });
        };
        match &**a {
            AsmVal::Inst(id) => {
                let path = vec![fa::InstanceId(hid(id))];
                let pose = self.sol.pose(path[0]).ok_or(Unevaluable::NoReference {
                    reference: id.clone(),
                })?;
                Ok((path, pose))
            }
            AsmVal::Ref(r) => {
                let text = || {
                    let mut t = r.inst.clone();
                    if !r.segs.is_empty() {
                        t.push('.');
                        t.push_str(&path_text(&r.segs));
                    }
                    t
                };
                let missing = || Unevaluable::NoReference { reference: text() };
                let (_, end) = self
                    .names
                    .probe
                    .iter()
                    .find(|(p, _)| p.inst == r.inst && p.segs == r.segs)
                    .ok_or_else(missing)?;
                let occ = self.sol.occurrences.iter().find(|o| o.path == end.path);
                let (path, pose, part) = match occ {
                    Some(o) => (o.path.clone(), o.pose, Some(&o.part)),
                    // a whole subassembly instance: its own frame
                    None if end.path.len() == 1 => {
                        let pose = self.sol.pose(end.path[0]).ok_or_else(missing)?;
                        (end.path.clone(), pose, None)
                    }
                    None => return Err(missing()),
                };
                let frame = match &end.anchor {
                    fa::AnchorRef::Local(l) => pose.compose(&l.frame),
                    fa::AnchorRef::Named(n) => {
                        let info = part
                            .and_then(|p| fa::Parts::part(self.catalog, p))
                            .ok_or_else(missing)?;
                        match info.refs.get(n) {
                            Some(Ok(rf)) => pose.compose(&rf.frame),
                            _ => return Err(missing()),
                        }
                    }
                };
                Ok((path, frame))
            }
            _ => Err(Unevaluable::NotAPlace { found: v.ty() }),
        }
    }

    fn measure(&mut self, ev: &mut Eval, metric: &str, args: &[Expr]) -> Result<f64, Issue> {
        let why = |w: Unevaluable| Issue::ExpectUnevaluable {
            metric: metric.into(),
            why: w,
        };
        match metric {
            "mobility" => Ok(self.sol.mobility as f64),
            "interferences" => {
                let rep = self
                    .checker
                    .interference(self.sol, &self.asm.fits, &self.asm.fit_rules);
                Ok(rep.interferences.len() as f64)
            }
            "interference" | "clearance" | "distance" => {
                let (pa, fa_) = self.place(ev, args, 0).map_err(why)?;
                let (pb, fb) = self.place(ev, args, 1).map_err(why)?;
                if metric == "distance" {
                    return Ok(fa_.origin.distance(fb.origin));
                }
                // the part occurrences each side is (a subassembly is all of its parts)
                let (oa, ob) = (self.parts_under(&pa), self.parts_under(&pb));
                let name = |p: &fa::Path| path_name(self.names, p);
                for (o, p) in [(&oa, &pa), (&ob, &pb)] {
                    if o.is_empty() {
                        return Err(why(Unevaluable::NoGeometry { reference: name(p) }));
                    }
                }
                if metric == "clearance" {
                    let mut best = f64::INFINITY;
                    for x in &oa {
                        for y in &ob {
                            let c = self
                                .checker
                                .clearance(self.sol, &x.path, &y.path, 1e9)
                                .ok_or_else(|| {
                                    why(Unevaluable::NoGeometry {
                                        reference: name(&x.path),
                                    })
                                })?;
                            best = best.min(c.distance);
                        }
                    }
                    return Ok(best);
                }
                // the solids' shared volume, exactly, whatever a fit says of it
                let mut vol = 0.0;
                for x in &oa {
                    for y in &ob {
                        let rel = x.pose.inverse().compose(&y.pose);
                        vol += self
                            .checker
                            .exact()
                            .overlap_volume(&x.part, &y.part, &rel)
                            .map_err(|e| {
                                why(match e {
                                    fa::check::ExactFail::NoSolid => Unevaluable::NoGeometry {
                                        reference: name(&x.path),
                                    },
                                    fa::check::ExactFail::Boolean(reason) => Unevaluable::BooleanFailed {
                                        a: name(&x.path),
                                        b: name(&y.path),
                                        reason,
                                    },
                                })
                            })?;
                    }
                }
                Ok(vol)
            }
            "x" | "y" | "z" => {
                let (_, f) = self.place(ev, args, 0).map_err(why)?;
                Ok(match metric {
                    "x" => f.origin.x,
                    "y" => f.origin.y,
                    _ => f.origin.z,
                })
            }
            "value" => {
                let e = args.first().ok_or(why(Unevaluable::MissingArg { index: 0 }))?;
                let v = match ev.expr(e) {
                    Ok(v) => v,
                    Err(err) => {
                        ev.errors.push(err);
                        return Err(why(Unevaluable::NotAJointValue {
                            found: Ty::Unresolved,
                        }));
                    }
                };
                let Value::Asm(a) = &v else {
                    return Err(why(Unevaluable::NotAJointValue { found: v.ty() }));
                };
                let AsmVal::JointValue { joint, freedom } = &**a else {
                    return Err(why(Unevaluable::NotAJointValue { found: v.ty() }));
                };
                self.sol
                    .joints
                    .iter()
                    .find(|j| j.joint.0 == hid(joint))
                    .and_then(|j| j.values.iter().find(|x| x.0 == *freedom).map(|x| x.1))
                    .ok_or(why(Unevaluable::NoJointValue { joint: joint.clone() }))
            }
            _ => Err(Issue::UnknownMetric {
                metric: metric.into(),
            }),
        }
    }
}

fn item_name(names: &Names, it: &fa::Item) -> ItemName {
    match it {
        fa::Item::Constraint { id, .. } => match id {
            fa::ConstraintId::Loose(c) => ItemName {
                id: names.link.get(c).map_or_else(|| c.to_string(), |x| x.0.clone()),
                part: None,
            },
            fa::ConstraintId::Joint { joint, part } => {
                let (n, kind) = names
                    .link
                    .get(&joint.0)
                    .map_or((joint.0.to_string(), None), |x| (x.0.clone(), x.2));
                let p = match (part, kind) {
                    (fa::JointPart::Prim(i), Some(k)) => {
                        k.bundle().get(*i as usize).map_or("?", |c| c.name()).to_string()
                    }
                    (fa::JointPart::Drive(f), Some(k)) => format!("drive {}", freedom_word(k, *f)),
                    (fa::JointPart::Limit(f), Some(k)) => format!("limit {}", freedom_word(k, *f)),
                    _ => "?".into(),
                };
                ItemName { id: n, part: Some(p) }
            }
        },
        fa::Item::Relation(r) => ItemName {
            id: names
                .rel
                .get(&r.0)
                .map_or_else(|| r.0.to_string(), |x| x.0.clone()),
            part: None,
        },
        fa::Item::Ground(i) => ItemName {
            id: names
                .inst
                .get(&i.0)
                .map_or_else(|| i.0.to_string(), |x| x.0.clone()),
            part: Some("ground".into()),
        },
    }
}

/// The statement an item belongs to.
fn item_span(names: &Names, it: &fa::Item) -> Option<Span> {
    match it {
        fa::Item::Constraint { id, .. } => {
            let k = match id {
                fa::ConstraintId::Loose(c) => *c,
                fa::ConstraintId::Joint { joint, .. } => joint.0,
            };
            names.link.get(&k).map(|x| x.1)
        }
        fa::Item::Relation(r) => names.rel.get(&r.0).map(|x| x.1),
        fa::Item::Ground(i) => names.inst.get(&i.0).map(|x| x.1),
    }
}

fn path_name(names: &Names, p: &fa::Path) -> String {
    let mut s = names
        .inst
        .get(&p[0].0)
        .map_or_else(|| p[0].0.to_string(), |x| x.0.clone());
    for x in &p[1..] {
        match names.sub_inst.get(&x.0) {
            Some(n) => s.push_str(&format!(".{n}")),
            None => s.push_str(&format!(".#{:x}", x.0)),
        }
    }
    s
}

fn push_diag(ev: &mut Eval, names: &Names, dg: &fa::Diagnostic, d: &Doc) {
    let first = Span { lo: 0, hi: 0 };
    let link = |id: Option<fa::ConstraintId>, joint: Option<fa::JointId>| -> (String, Span) {
        let k = joint.map(|j| j.0).or(id.map(|c| match c {
            fa::ConstraintId::Loose(c) => c,
            fa::ConstraintId::Joint { joint, .. } => joint.0,
        }));
        k.and_then(|k| names.link.get(&k))
            .map_or((String::new(), first), |x| (x.0.clone(), x.1))
    };
    let (span, issue) = match dg {
        fa::Diagnostic::BrokenAnchor {
            constraint,
            joint,
            side,
            anchor,
            reason,
        } => {
            let (item, span) = link(*constraint, *joint);
            (
                span,
                Issue::BrokenAnchor {
                    item,
                    side: *side,
                    anchor: anchor.clone(),
                    reason: reason.clone(),
                },
            )
        }
        fa::Diagnostic::WrongReference {
            constraint,
            joint,
            side,
            anchor,
            want,
            found,
        } => {
            let (item, span) = link(*constraint, *joint);
            (
                span,
                Issue::WrongReference {
                    item,
                    side: *side,
                    anchor: anchor.clone(),
                    want: want.iter().map(|k| k.name()).collect(),
                    found: found.name(),
                },
            )
        }
        fa::Diagnostic::Redundant {
            item,
            rows,
            implied_by,
        } => (
            item_span(names, item).unwrap_or(first),
            Issue::Redundant {
                item: item_name(names, item),
                rows: *rows,
                implied_by: implied_by.iter().map(|x| item_name(names, x)).collect(),
            },
        ),
        fa::Diagnostic::Conflict { items, residuals } => {
            let last = items.last();
            let culprit = last.map_or_else(String::new, |x| item_name(names, x).id);
            (
                last.and_then(|x| item_span(names, x)).unwrap_or(first),
                Issue::Conflict {
                    items: items.iter().map(|x| item_name(names, x)).collect(),
                    residuals: residuals.clone(),
                    culprit,
                },
            )
        }
        fa::Diagnostic::NoGround => (d.insts.first().map_or(first, |i| i.span), Issue::NoGround),
        fa::Diagnostic::NotConverged { items, residual, .. } => (
            items.first().and_then(|x| item_span(names, x)).unwrap_or(first),
            Issue::NotConverged {
                items: items.iter().map(|x| item_name(names, x)).collect(),
                residual: *residual,
            },
        ),
        fa::Diagnostic::LimitViolated {
            joint,
            freedom,
            value,
            range,
            fix,
        } => {
            let (n, span, kind) = names
                .link
                .get(&joint.0)
                .map_or((String::new(), first, None), |x| (x.0.clone(), x.1, x.2));
            let word = kind.map_or(freedom.name(), |k| freedom_word(k, *freedom));
            // the param driving the value, which the fix sets
            let param = d
                .links
                .iter()
                .find(|l| l.id == n)
                .and_then(|l| l.drive.iter().find(|x| x.0 == *freedom))
                .and_then(|x| match &x.1.src {
                    Src::Driving(p) => Some(p.clone()),
                    _ => None,
                });
            let to = fix.as_ref().map(|f| match f {
                fa::Fix::SetDrive { value, .. } => *value,
            });
            (
                span,
                Issue::LimitViolated {
                    joint: n,
                    freedom: word,
                    value: *value,
                    range: *range,
                    angle: freedom.is_angle(),
                    fix: param.zip(to),
                },
            )
        }
        fa::Diagnostic::BadFreedom { joint, freedom } => {
            let (n, span) = link(None, Some(*joint));
            (
                span,
                Issue::BadFreedom {
                    joint: n,
                    freedom: freedom.name(),
                },
            )
        }
        fa::Diagnostic::BadRelation { relation } => {
            let (n, span) = names
                .rel
                .get(&relation.0)
                .map_or((String::new(), first), |x| (x.0.clone(), x.1));
            (span, Issue::BadRelation { relation: n })
        }
        fa::Diagnostic::DuplicateInstance { id } => {
            let (n, span) = names
                .inst
                .get(&id.0)
                .map_or((String::new(), first), |x| x.clone());
            (span, Issue::DuplicateItem { id: n })
        }
        fa::Diagnostic::DuplicateJoint { id } => {
            let (n, span) = link(None, Some(*id));
            (span, Issue::DuplicateItem { id: n })
        }
        fa::Diagnostic::DuplicateConstraint { id } => {
            let (n, span) = link(Some(fa::ConstraintId::Loose(*id)), None);
            (span, Issue::DuplicateItem { id: n })
        }
        fa::Diagnostic::SelfJoint { constraint, joint } => {
            let (n, span) = link(*constraint, *joint);
            (span, Issue::SelfLink { item: n })
        }
        fa::Diagnostic::CyclicAssembly { doc } => (
            first,
            Issue::CyclicAssembly {
                file: doc.0.to_string(),
            },
        ),
        fa::Diagnostic::PartChanged { part, .. } => (
            first,
            Issue::PartChanged {
                file: part.doc.0.to_string(),
            },
        ),
        fa::Diagnostic::Interference {
            a,
            b,
            volume,
            confirmed,
            boolean_failed,
        } => (
            names.inst.get(&a[0].0).map_or(first, |x| x.1),
            Issue::Interference {
                a: path_name(names, a),
                b: path_name(names, b),
                volume: *volume,
                confirmed: *confirmed,
                boolean_failed: boolean_failed.clone(),
            },
        ),
        fa::Diagnostic::IntendedContact { a, b, by } => {
            let (by, span) = match by {
                fa::Intent::Fit { index } => {
                    names.fits.get(*index).cloned().unwrap_or((String::new(), first))
                }
                fa::Intent::Rule { rule } => names.rule.get(rule).cloned().unwrap_or((String::new(), first)),
            };
            (
                span,
                Issue::IntendedContact {
                    a: path_name(names, a),
                    b: path_name(names, b),
                    by,
                },
            )
        }
        fa::Diagnostic::ShallowEngagement {
            a,
            b,
            rule,
            engaged,
            min,
        } => {
            let (rn, span) = names.rule.get(rule).cloned().unwrap_or((String::new(), first));
            (
                span,
                Issue::ShallowEngagement {
                    a: path_name(names, a),
                    b: path_name(names, b),
                    rule: rn,
                    engaged: *engaged,
                    min: *min,
                },
            )
        }
        fa::Diagnostic::Inside { path, diagnostic } => {
            let span = names.inst.get(&path[0].0).map_or(first, |x| x.1);
            let mut sub = vec![];
            let mut tmp = Eval::new("", crate::eval::Rules::default(), false, Default::default());
            push_diag(&mut tmp, &Names::default(), diagnostic, &Doc::default());
            sub.extend(tmp.errors.into_iter().filter_map(|e| match e {
                Error::Asm(n) => Some(n.issue),
                _ => None,
            }));
            sub.extend(tmp.warnings.into_iter().filter_map(|w| match w {
                Warning::Asm(n) => Some(n.issue),
                _ => None,
            }));
            let Some(inner) = sub.pop() else { return };
            (
                span,
                Issue::Inside {
                    path: path_name(names, path),
                    issue: Box::new(inner),
                },
            )
        }
    };
    let n = AsmNote { span, issue };
    match n.issue.level() {
        Level::Error => ev.errors.push(Error::Asm(Box::new(n))),
        _ => ev.warnings.push(Warning::Asm(Box::new(n))),
    }
}

/// The placed parts as one body, for a picture.
pub fn compound(out: &AsmOut) -> Option<forge_topo::Body> {
    let mut all = forge_topo::Body::new();
    for (b, f) in &out.draw {
        let moved = forge_asm::check::rigid_copy(b, f);
        append(&mut all, &moved);
    }
    let any = all.face_ids().next().is_some();
    any.then_some(all)
}

fn append(out: &mut forge_topo::Body, b: &forge_topo::Body) {
    use forge_topo::body::{EdgeId, ShellId, VertexId};
    let mut vmap: Vec<Option<VertexId>> = vec![None; b.vertices.len()];
    let mut emap: Vec<Option<EdgeId>> = vec![None; b.edges.len()];
    let mut smap: Vec<Option<ShellId>> = vec![None; b.shells.len()];
    for fid in b.face_ids() {
        let face = b.face(fid);
        let shell = *smap[face.shell.index()].get_or_insert_with(|| out.add_shell());
        let nf = out.add_face(shell, face.surface.clone(), face.same_sense, face.name);
        for &l in &face.loops {
            let mut uses = Vec::new();
            for c in b.lp(l).coedges() {
                let co = b.coedge(c);
                let e = match emap[co.edge.index()] {
                    Some(e) => e,
                    None => {
                        let edge = b.edge(co.edge);
                        let mut vert = |v: VertexId| {
                            *vmap[v.index()].get_or_insert_with(|| {
                                let x = b.vertex(v);
                                out.add_tolerant_vertex(x.point, x.tol, x.name)
                            })
                        };
                        let (s, t) = (vert(edge.start), vert(edge.end));
                        let e = out.add_edge(edge.curve.clone(), edge.range, s, t, edge.name);
                        emap[co.edge.index()] = Some(e);
                        e
                    }
                };
                uses.push((e, co.forward));
            }
            out.add_loop(nf, &uses);
        }
    }
}

/// The assembly section of a run's JSON.
pub fn json(out: &AsmOut) -> Json {
    let frame = |f: &Frame| {
        let (o, x, z) = (f.origin, f.x().get(), f.z().get());
        Json::obj(vec![
            ("origin", Json::Arr(vec![o.x.into(), o.y.into(), o.z.into()])),
            ("x", Json::Arr(vec![x.x.into(), x.y.into(), x.z.into()])),
            ("z", Json::Arr(vec![z.x.into(), z.y.into(), z.z.into()])),
        ])
    };
    let t = &out.timings;
    let mut f = vec![
        (
            "instances",
            Json::Arr(
                out.poses
                    .iter()
                    .map(|(id, p)| Json::obj(vec![("id", id.as_str().into()), ("pose", frame(p))]))
                    .collect(),
            ),
        ),
        (
            "joints",
            Json::Arr(
                out.joints
                    .iter()
                    .map(|j| {
                        Json::obj(vec![
                            ("id", j.id.as_str().into()),
                            ("kind", j.kind.into()),
                            (
                                "values",
                                Json::Arr(
                                    j.values
                                        .iter()
                                        .map(|(w, v, a)| {
                                            Json::obj(vec![
                                                ("freedom", (*w).into()),
                                                ("value", (if *a { v.to_degrees() } else { *v }).into()),
                                            ])
                                        })
                                        .collect(),
                                ),
                            ),
                            ("free", j.free.into()),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("mobility", out.mobility.into()),
        ("interferences", out.interferences.into()),
        ("intended_contacts", out.intended.into()),
        (
            "fit_rules",
            Json::Arr(
                out.rules
                    .iter()
                    .map(|(r, n)| Json::obj(vec![("rule", r.as_str().into()), ("pairs", (*n).into())]))
                    .collect(),
            ),
        ),
        (
            "bom",
            Json::Arr(
                out.bom
                    .iter()
                    .map(|(f, n)| Json::obj(vec![("file", f.as_str().into()), ("qty", (*n).into())]))
                    .collect(),
            ),
        ),
        (
            "hashes",
            Json::Arr(
                out.hashes
                    .iter()
                    .map(|(f, h)| Json::obj(vec![("file", f.as_str().into()), ("hash", h.as_str().into())]))
                    .collect(),
            ),
        ),
        (
            "timings_ms",
            Json::obj(vec![
                ("load", t.load_ms.into()),
                ("anchors", t.anchors_ms.into()),
                ("solve", t.solve_ms.into()),
                ("overlap", t.overlap_ms.into()),
                ("expects", t.expects_ms.into()),
                ("incremental", t.incremental.into()),
                ("at_runs", t.at_runs.into()),
            ]),
        ),
    ];
    let opt = |x: Option<f64>| x.map_or(Json::Null, Json::Num);
    f.push((
        "expects",
        Json::Arr(
            out.expects
                .iter()
                .map(|e| {
                    Json::obj(vec![
                        ("line", e.line.into()),
                        ("metric", e.metric.as_str().into()),
                        ("measured", opt(e.measured)),
                        ("limit", opt(e.limit)),
                        ("ok", e.ok.into()),
                    ])
                })
                .collect(),
        ),
    ));
    f.push((
        "engagements",
        Json::Arr(
            out.engagements
                .iter()
                .map(|e| {
                    Json::obj(vec![
                        ("a", e.a.as_str().into()),
                        ("b", e.b.as_str().into()),
                        ("rule", e.rule.as_str().into()),
                        ("engaged", e.engaged.into()),
                        ("min", opt(e.min)),
                        ("short", e.short.into()),
                    ])
                })
                .collect(),
        ),
    ));
    if let Some(o) = &out.overlap {
        f.push((
            "overlap",
            Json::obj(vec![
                ("candidates", o.candidates.into()),
                ("narrow_tests", o.narrow_tests.into()),
                ("cache_hits", o.cache_hits.into()),
                ("exact_runs", o.exact_runs.into()),
            ]),
        ));
    }
    Json::obj(f)
}

#[allow(dead_code)]
fn unit_dir(v: Vec3) -> Option<UnitVec3> {
    v.normalize()
}

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
use std::collections::{BTreeMap, BTreeSet, HashMap};
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
    "std",
    "ground",
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
    "gap",
    "axis_offset",
    "axis_angle",
];

// ---- values -------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct RefDef {
    pub inst: String,
    pub segs: Vec<Seg>,
    pub span: Span,
    /// An end on a body of this document, worked out where it was written: in the document's
    /// model space, the body's own (`segs` is then only its text).
    pub local: Option<fa::Reference>,
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
    /// A component definition of this document.
    Def(Rc<CompDef>),
    /// A call of one: the part it makes (its key) and the evaluation it is (`Comp::eval_key`).
    Call {
        key: String,
        eval: String,
    },
    /// A component of a used file (`t.cup`), placed from that file.
    Foreign {
        name: String,
        file: String,
    },
    /// A call of one (`t.cup(d: 9)`): its params' values, by name; and as given, with their
    /// provenance and text (what the call reads of this document).
    ForeignCall {
        name: String,
        file: String,
        args: Vec<(String, Literal)>,
        given: Vec<(String, Value, String)>,
    },
    /// A harness or one of its items (`harness.rs`).
    Harness(crate::harness::HVal),
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
            AsmVal::Harness(_) => Ty::Harness,
            AsmVal::Full => Ty::Word,
            AsmVal::Def(_) | AsmVal::Call { .. } | AsmVal::Foreign { .. } | AsmVal::ForeignCall { .. } => {
                Ty::Function
            }
        }
    }
}

fn asm(v: AsmVal) -> Value {
    Value::Asm(Rc::new(v))
}

#[derive(Clone, Debug)]
pub struct InstDef {
    pub id: String,
    /// A part file, or `#NAME` for a component of this document (`source` says which).
    pub file: String,
    pub source: Source,
    /// A call's evaluation (values and where each came from), for an inline component.
    pub eval: Option<String>,
    pub body: Option<String>,
    pub pose: Frame,
    pub ground: bool,
    pub reference: bool,
    pub suppressed: bool,
    pub sub: bool,
    pub span: Span,
}

/// Where a component comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// A part file, a subassembly or a standard part.
    File,
    /// A `component NAME(..) { .. }` block of this document.
    Inline,
    /// A body of this document a joint or constraint names.
    Body,
    /// The bodies of this document no joint names, held where they are made.
    Root,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::File => "file",
            Source::Inline => "inline",
            Source::Body => "body",
            Source::Root => "root",
        }
    }
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
    pub suppressed: bool,
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
    /// In-file components whose `ground:` was left to the rule: grounded when nothing moves them.
    pub auto_ground: BTreeSet<String>,
    /// Occurrences `ground(X)` holds where they are written.
    pub grounds: BTreeSet<String>,
    /// The first line says `assembly`: an assembly before it holds anything.
    pub declared: bool,
    /// Part files placed another way than as they are, by their key (`mug.fg|with:d=70`):
    /// their params set, or one of their components called.
    pub variants: BTreeMap<String, Variant>,
    /// The document's wire harnesses, as written (`harness.rs`).
    pub harnesses: Vec<crate::harness::HarnessDef>,
}

/// A part file placed as something other than its default part.
#[derive(Clone, Debug, PartialEq)]
pub enum Variant {
    /// Its shared params set (`part(FILE, with: {d: 70})`).
    With(Vec<(String, Literal)>),
    /// One of its components called (`t.cup(d: 9)` of `t = use(FILE)`).
    Component {
        name: String,
        args: Vec<(String, Literal)>,
    },
}

impl Variant {
    /// What it adds to the file's key: what makes it a part of its own in the BOM.
    pub fn key(&self) -> String {
        let args = |xs: &[(String, Literal)]| {
            xs.iter()
                .map(|(n, l)| format!("{n}={}", l.text()))
                .collect::<Vec<_>>()
                .join(",")
        };
        match self {
            Variant::With(xs) => format!("with:{}", args(xs)),
            Variant::Component { name, args: xs } => format!("{name}({})", args(xs)),
        }
    }
}

/// A number given across files (a `with:` value, a used file's component's argument), as it is
/// written into that file.
fn literal_of(v: &Value, span: Span) -> R<Literal> {
    match v {
        Value::Num(n) => Ok(Literal {
            value: if n.dim == Dim::Angle {
                n.v.to_degrees()
            } else {
                n.v
            },
            dim: if n.dim == Dim::Any { Dim::Length } else { n.dim },
        }),
        o => Err(wrong(span, "a number for another file's param", Ty::Num, o)),
    }
}

/// The `with:` argument of a call, as written.
fn with_arg(args: &[Arg]) -> Option<&Expr> {
    args.iter()
        .find(|a| a.name.as_ref().is_some_and(|n| n.0 == "with"))
        .map(|a| &a.value)
}

fn with_text(src: &str, w: &Expr, name: &str) -> String {
    match &w.kind {
        ExprKind::Map(xs) => xs
            .iter()
            .find(|x| x.0 == name)
            .map(|x| one_line(&src[x.1.span.lo as usize..x.1.span.hi as usize]))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// A file's text kept for the run's evaluations of it (each text once, for the process: an
/// evaluation borrows its text as long as the document's). Found by the text itself, never by a
/// hash of it: two texts are never one.
pub(crate) fn kept_text(text: String) -> &'static str {
    use std::sync::{Mutex, OnceLock};
    static TEXTS: OnceLock<Mutex<std::collections::BTreeSet<&'static str>>> = OnceLock::new();
    let mut m = TEXTS.get_or_init(Default::default).lock().expect("texts");
    if let Some(t) = m.get(text.as_str()) {
        return t;
    }
    let t: &'static str = Box::leak(text.into_boxed_str());
    m.insert(t);
    t
}

/// `part(FILE, with: {..})` evaluated as a call of the file's default part: its shared params
/// its params (given, or their defaults, a literal one its own handle `STEM.p`), its other
/// statements the body, over the file's own text. Its features, the params it reads and observes
/// are this document's report (the solve still loads the file). The evaluation's key, or None
/// when the file is not one this can read (the loader then says why).
fn file_call(ev: &mut Eval, file: &str, key: &str, given: &[(String, Value, String)]) -> Option<String> {
    let path = locate(&ev.dir, &ev.root, file)?;
    let text = kept_text(read_capped(&path)?);
    let prog = crate::syntax::parse(text).ok()?;
    let stem = std::path::Path::new(file)
        .file_stem()
        .map_or_else(|| file.to_string(), |s| s.to_string_lossy().to_string());
    let traces: Vec<String> = given.iter().map(|g| format!("{}<-{}", g.0, g.2)).collect();
    let eval_key = format!("{key}|{}", traces.join(","));
    if ev.comps.iter().any(|c| c.eval_key == eval_key) {
        return Some(eval_key);
    }
    let mut sub = Eval::new(
        text,
        ev.rules,
        ev.b.building,
        ev.b.cache.comps.remove(&eval_key).unwrap_or_default(),
    );
    sub.dir = path.parent().map(|p| p.to_path_buf());
    sub.root = ev.root.clone();
    sub.use_chain = ev.use_chain.clone();
    sub.asm = Some(Doc::default());
    sub.comp_path = format!("{}{stem}?/", ev.comp_path);
    sub.total = ev.total.clone();
    let mut body = vec![];
    for st in &prog {
        match &st.kind {
            StmtKind::Shared { name, value } => {
                let v = match given.iter().find(|g| &g.0 == name) {
                    Some(g) => g.1.clone(),
                    None => {
                        let literal = matches!(value.kind, ExprKind::Num(..));
                        match sub.expr(value) {
                            Ok(Value::Num(x)) if literal => Value::Num(Num {
                                src: Src::Driving(ParamKey {
                                    owner: vec![Seg {
                                        name: stem.clone(),
                                        index: vec![],
                                    }],
                                    name: name.clone(),
                                }),
                                ..x
                            }),
                            Ok(v) => v,
                            Err(_) => continue,
                        }
                    }
                };
                let _ = sub.bind_name(name, v, st.span);
            }
            StmtKind::Expr(e) if matches!(&e.kind, ExprKind::Name(n) if n == "assembly") => {}
            _ => body.push(st.clone()),
        }
    }
    sub.check_static(&body);
    if !sub.errors.is_empty() {
        sub.b.building = false;
    }
    sub.run(&body);
    // its warnings are cleared below (the file's own): its shop rules go unasked
    crate::sheet::finish_with(&mut sub, false);
    // what it observes of this document's params is this document's (its own are its own)
    let shared: BTreeSet<ParamKey> = ev.shared.iter().map(|p| p.key.clone()).collect();
    for k in std::mem::take(&mut sub.observed) {
        if shared.contains(&k) {
            ev.observed.insert(k);
        }
    }
    // its own errors are the file's (the loader reports them on the placement)
    sub.errors.clear();
    sub.warnings.clear();
    ev.comps.push(Comp {
        id: key.trim_start_matches('#').to_string(),
        eval_key: eval_key.clone(),
        def: stem,
        ev: sub,
        span: Span::default(),
        sub: false,
    });
    Some(eval_key)
}

/// What a used file's component sees: the file's shared params, functions and uses, and its
/// own definition.
fn component_scope(prog: &[crate::syntax::Stmt], name: &str) -> Vec<crate::syntax::Stmt> {
    prog.iter()
        .filter(|s| match &s.kind {
            StmtKind::Shared { .. } | StmtKind::Fn { .. } => true,
            StmtKind::Assign { value, .. } => {
                matches!(&value.kind, ExprKind::Call(f, _) if matches!(&f.kind, ExprKind::Name(n) if n == "use"))
            }
            StmtKind::Block {
                kind: crate::syntax::BlockKind::Component,
                name: n,
                ..
            } => n == name,
            _ => false,
        })
        .cloned()
        .collect()
}

/// `part(t.cup(d: x))` evaluated here, over the used file's text: the values given keep their
/// provenance (this document's params), the file's shared params are its own (`STEM.p`). Its features and what it reads are this document's report (the solve still
/// loads it from the file). The evaluation's key, or None when the file is not one this reads
/// (the loader then says why).
fn foreign_call(
    ev: &mut Eval,
    file: &str,
    name: &str,
    key: &str,
    given: &[(String, Value, String)],
) -> Option<String> {
    let path = locate(&ev.dir, &ev.root, file)?;
    let text = kept_text(read_capped(&path)?);
    let prog = crate::syntax::parse(text).ok()?;
    let stem = std::path::Path::new(file)
        .file_stem()
        .map_or_else(|| file.to_string(), |s| s.to_string_lossy().to_string());
    let traces: Vec<String> = given.iter().map(|g| format!("{}<-{}", g.0, g.2)).collect();
    let eval_key = format!("{key}|{}", traces.join(","));
    if ev.comps.iter().any(|c| c.eval_key == eval_key) {
        return Some(eval_key);
    }
    let (calls, comps) = ev.total.get();
    if comps + 1 > crate::eval::TOTAL_COMPONENTS {
        return None;
    }
    ev.total.set((calls, comps + 1));
    let mut scope = Eval::new(text, ev.rules, false, Default::default());
    scope.dir = path.parent().map(|p| p.to_path_buf());
    scope.root = ev.root.clone();
    scope.use_chain = ev.use_chain.clone();
    scope.asm = Some(Doc::default());
    scope.total = ev.total.clone();
    scope.run(&component_scope(&prog, name));
    // the file's shared params are its own (`STEM.W`), never this document's of the same name
    for p in scope.shared.iter_mut() {
        p.key.owner = vec![Seg {
            name: stem.clone(),
            index: vec![],
        }];
    }
    let def = scope.top_level().into_iter().find_map(|(n, v)| match v {
        Value::Asm(a) if n == name => match &*a {
            AsmVal::Def(d) => Some(d.clone()),
            _ => None,
        },
        _ => None,
    })?;
    let mut values = call_values(&mut scope, &def, &[], def.span).ok()?;
    for (n, v, _) in given {
        values.iter_mut().find(|x| x.0 == *n)?.1 = v.clone();
    }
    let cache = ev.b.cache.comps.remove(&eval_key).unwrap_or_default();
    let path = format!("{}{stem}.{name}?/", ev.comp_path);
    // its warnings are cleared below (the file's own): its shop rules go unasked
    let mut sub = sub_eval_at(&scope, &def, &values, ev.b.building, cache, path, false);
    // what it observes of this document's params is this document's (its own are its own)
    let shared: BTreeSet<ParamKey> = ev.shared.iter().map(|p| p.key.clone()).collect();
    for k in std::mem::take(&mut sub.observed) {
        if shared.contains(&k) {
            ev.observed.insert(k);
        }
    }
    // its own errors are the file's (the loader reports them on the placement)
    sub.errors.clear();
    sub.warnings.clear();
    ev.comps.push(Comp {
        id: key.trim_start_matches('#').to_string(),
        eval_key: eval_key.clone(),
        def: format!("{stem}.{name}"),
        ev: sub,
        span: Span::default(),
        sub: false,
    });
    Some(eval_key)
}

/// `t.cup(p: v, ..)` of a used file's component: the call, placed by `part(..)`.
pub fn call_foreign(ev: &mut Eval, name: &str, file: &str, args: &[Arg], span: Span) -> R<Value> {
    let (mut out, mut given) = (vec![], vec![]);
    for a in args {
        let Some((n, _)) = &a.name else {
            return Err(Error::MissingArg {
                span: a.value.span,
                func: "component",
                arg: "a param by name (`d: 80`)",
            });
        };
        let v = ev.expr(&a.value)?;
        out.push((n.clone(), literal_of(&v, a.value.span)?));
        let t = one_line(
            ev.src
                .get(a.value.span.lo as usize..a.value.span.hi as usize)
                .unwrap_or(""),
        );
        given.push((n.clone(), v, t));
    }
    let _ = span;
    Ok(asm(AsmVal::ForeignCall {
        name: name.into(),
        file: file.into(),
        args: out,
        given,
    }))
}

impl Doc {
    /// No components or links: the document is a part.
    pub fn is_empty(&self) -> bool {
        !self.declared
            && self.insts.is_empty()
            && self.links.is_empty()
            && self.relations.is_empty()
            && self.fits.is_empty()
            && self.rules.is_empty()
            && self.harnesses.is_empty()
    }
}

/// Whether a program declares itself an assembly: its first statement is the word `assembly`.
pub fn declared(prog: &[crate::syntax::Stmt]) -> bool {
    matches!(prog.first().map(|s| &s.kind), Some(StmtKind::Expr(Expr { kind: ExprKind::Name(n), .. })) if n == "assembly")
}

/// Whether a program holds components or links (read from its text, not run): it is placed
/// with `subassembly()`, not `part()`.
pub fn has_components(prog: &[crate::syntax::Stmt]) -> bool {
    const PLACES: &[&str] = &[
        "part",
        "subassembly",
        "std",
        "fastened",
        "revolute",
        "slider",
        "cylindrical",
        "pin_slot",
        "planar",
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
    ];
    fn any(stmts: &[crate::syntax::Stmt]) -> bool {
        // a component's definition places nothing: only placing it, or a joint, does
        stmts.iter().any(|s| match &s.kind {
            StmtKind::Assign { value: e, .. } | StmtKind::Expr(e) => {
                matches!(&e.kind, ExprKind::Call(f, _) if matches!(&f.kind, ExprKind::Name(n) if PLACES.contains(&n.as_str())))
            }
            StmtKind::For { body, .. } => any(body),
            StmtKind::Block {
                kind: crate::syntax::BlockKind::Harness,
                ..
            } => true,
            StmtKind::If { then, other, .. } => any(then) || other.as_deref().is_some_and(any),
            _ => false,
        })
    }
    declared(prog) || any(prog)
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
    matches!(&f.kind, ExprKind::Name(n) if n == "part" || n == "subassembly" || n == "std")
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
        JointKind::Revolute => &["a", "b", "turn", "limit", "flip", "suppressed"],
        JointKind::Slider => &["a", "b", "along", "limit", "flip", "suppressed"],
        JointKind::Cylindrical | JointKind::PinSlot => &[
            "a",
            "b",
            "along",
            "turn",
            "along_limit",
            "turn_limit",
            "flip",
            "suppressed",
        ],
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
            "suppressed",
        ],
        JointKind::Fastened | JointKind::Ball => &["a", "b", "flip", "suppressed"],
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
                local: None,
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
/// with a part built-in where the part's is meant (a sketch's constraint, the solid `ball`).
pub fn call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> Option<R<Value>> {
    if PART_NAMES.contains(&name) {
        return Some(part_call(ev, name, args, span));
    }
    if !NAMES.contains(&name) {
        return None;
    }
    let shared = crate::builtins::builtin(name).is_some() || crate::builtins::constant(name).is_some();
    if shared && (ev.in_sketch() || (name == "ball" && !ball_joint(ev, args))) {
        return None;
    }
    if ev.asm.is_some() {
        return Some(asm_call(ev, name, args, span));
    }
    if shared {
        return None;
    }
    // a component's own statements (or a part file a part() reads): no components or joints
    Some(Err(note(
        span,
        Issue::Component(CompProblem::Nested { func: name }),
    )))
}

/// `ball(a, b)` joins two references; `ball(POINT, r)` is the solid.
fn ball_joint(ev: &mut Eval, args: &[Arg]) -> bool {
    let named = |n: &str| args.iter().any(|a| a.name.as_ref().is_some_and(|x| x.0 == n));
    if ["r", "cut", "keep", "new"].iter().any(|n| named(n)) {
        return false;
    }
    if named("b") || named("flip") || named("suppressed") {
        return true;
    }
    let Some(second) = args.get(1).filter(|a| a.name.is_none()) else {
        return false;
    };
    let (e, w) = (ev.errors.len(), ev.warnings.len());
    let v = ev.expr(&second.value);
    ev.errors.truncate(e);
    ev.warnings.truncate(w);
    !matches!(v, Ok(Value::Num(_)))
}

fn doc<'a>(ev: &'a mut Eval<'_>) -> &'a mut Doc {
    ev.asm.as_mut().expect("an assembly document")
}

fn asm_call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    if let Some(kind) = joint_kind(name) {
        return joint(ev, name, kind, args, span);
    }
    match name {
        "subassembly" if let Some(r) = crate::harness::place(ev, args, span) => r,
        "part" | "subassembly" => {
            let a = Args::bind(
                ev,
                name,
                &[
                    "file",
                    "body",
                    "at",
                    "ground",
                    "reference",
                    "suppressed",
                    "with",
                    "version",
                ],
                args,
                span,
            )?;
            // a part file pinned to one of its named versions: `part("bracket.fg", version: 3)`
            let pinned = match a.opt("version") {
                None => None,
                Some(Value::Num(n)) if n.v >= 1.0 && n.v.fract() == 0.0 && n.v < 1e9 => Some(n.v as u32),
                Some(o) => return Err(wrong(a.span("version"), "a named version's number", Ty::Num, &o)),
            };
            // a file's shared params set for this occurrence: `part("mug.fg", with: {d: 70})`
            let with = match a.opt("with") {
                None => vec![],
                Some(Value::Map(m)) => {
                    let mut out = vec![];
                    for (k, v) in m.iter() {
                        let Key::Label(n) = k else {
                            return Err(wrong(
                                a.span("with"),
                                "a file's params by name",
                                Ty::Map,
                                &Value::Map(m.clone()),
                            ));
                        };
                        out.push((n.clone(), literal_of(v, a.span("with"))?));
                    }
                    out
                }
                Some(o) => return Err(wrong(a.span("with"), "a file's params by name", Ty::Map, &o)),
            };
            // a file, a used file's component, or a call of a component of this document
            let file_arg = match (a.req("file")?, pinned) {
                (Value::Str(s), Some(n)) if !s.starts_with('#') => {
                    let dir = ev.dir.clone().unwrap_or_default();
                    match crate::pins::pin(&dir, &s, n) {
                        Ok(p) => Value::Str(p.to_string_lossy().to_string().into()),
                        Err(e) => {
                            return Err(note(
                                a.span("version"),
                                Issue::PartFile {
                                    file: s.to_string(),
                                    problem: match e {
                                        crate::pins::PinError::NoHistory => FileProblem::NoHistory,
                                        crate::pins::PinError::Outside => FileProblem::Outside,
                                        crate::pins::PinError::NoVersion { newest } => {
                                            FileProblem::NoVersion { version: n, newest }
                                        }
                                    },
                                },
                            ));
                        }
                    }
                }
                (v, Some(_)) => return Err(wrong(a.span("version"), "a part file to pin", Ty::Str, &v)),
                (v, None) => v,
            };
            let (file, source, eval) = match file_arg {
                // `#` starts a component's key: no file is named so
                Value::Str(s) if s.starts_with('#') => {
                    return Err(wrong(
                        a.span("file"),
                        "a part file (a name not starting with #)",
                        Ty::Str,
                        &Value::Str(s),
                    ));
                }
                Value::Str(s) if with.is_empty() => (s.to_string(), Source::File, None),
                Value::Str(s) => {
                    let v = Variant::With(with.clone());
                    let k = format!("{s}|{}", v.key());
                    doc(ev).variants.insert(k.clone(), v);
                    // the file as a call: its values' provenance and what it observes are this
                    // document's, as an inline call's are (proof X1, X2)
                    let given: Vec<(String, Value, String)> = match (a.opt("with"), with_arg(args)) {
                        (Some(Value::Map(m)), Some(w)) => m
                            .iter()
                            .filter_map(|(key, v)| match key {
                                Key::Label(n) => {
                                    let t = with_text(ev.src, w, n);
                                    Some((n.clone(), v.clone(), t))
                                }
                                _ => None,
                            })
                            .collect(),
                        _ => vec![],
                    };
                    let eval = file_call(ev, &s, &k, &given);
                    (k, Source::File, eval)
                }
                Value::Asm(x) if matches!(&*x, AsmVal::Foreign { .. } | AsmVal::ForeignCall { .. }) => {
                    let (name, file, args, given) = match &*x {
                        AsmVal::Foreign { name, file } => (name.clone(), file.clone(), vec![], vec![]),
                        AsmVal::ForeignCall {
                            name,
                            file,
                            args,
                            given,
                        } => (name.clone(), file.clone(), args.clone(), given.clone()),
                        _ => unreachable!("matched"),
                    };
                    let v = Variant::Component {
                        name: name.clone(),
                        args,
                    };
                    let k = format!("{file}|{}", v.key());
                    doc(ev).variants.insert(k.clone(), v);
                    // the call evaluated here: its values' provenance and what it observes are
                    // this document's, as a `with:` file's are
                    let eval = foreign_call(ev, &file, &name, &k, &given);
                    (k, Source::File, eval)
                }
                Value::Asm(x) if name == "part" && matches!(&*x, AsmVal::Def(_) | AsmVal::Call { .. }) => {
                    let call = match &*x {
                        AsmVal::Def(d) => call_def(ev, d, &[], a.span("file"))?,
                        _ => Value::Asm(x.clone()),
                    };
                    let Value::Asm(c) = &call else {
                        unreachable!("a call")
                    };
                    let AsmVal::Call { key, eval } = &**c else {
                        unreachable!("a call")
                    };
                    (
                        format!("#{}{key}", ev.comp_path),
                        Source::Inline,
                        Some(eval.clone()),
                    )
                }
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
            let auto = source == Source::Inline && a.opt("ground").is_none();
            let id = ev.take_id(name, span)?;
            ev.record(
                &id,
                if name == "part" { "instance" } else { "subassembly" },
                span,
                vec![],
            );
            let is_sub = eval
                .as_ref()
                .is_some_and(|e| ev.comps.iter().any(|c| &c.eval_key == e && c.sub));
            doc(ev).insts.push(InstDef {
                id: id.clone(),
                file,
                source,
                eval,
                body,
                pose,
                ground,
                reference,
                suppressed,
                sub: name == "subassembly" || is_sub,
                span,
            });
            if auto {
                doc(ev).auto_ground.insert(id.clone());
            }
            Ok(asm(AsmVal::Inst(id)))
        }
        "std" => std_part(ev, args, span),
        // `ground(X)`: an occurrence, or a body of this document, held where it is written
        "ground" => {
            let (args, ends) = with_body_ends(ev, args)?;
            let a = ev.with_bound(ends, |ev| Args::bind(ev, name, &["a"], &args, span))?;
            let v = a.req("a")?;
            let id = match &v {
                Value::Asm(x) => match &**x {
                    AsmVal::Inst(id) => id.clone(),
                    AsmVal::Ref(r) if r.segs.is_empty() || r.local.is_some() => r.inst.clone(),
                    _ => return Err(wrong(a.span("a"), "an occurrence or body", Ty::Instance, &v)),
                },
                o => return Err(wrong(a.span("a"), "an occurrence or body", Ty::Instance, o)),
            };
            doc(ev).grounds.insert(id);
            Ok(Value::Bool(true))
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
    let (args, ends) = with_body_ends(ev, args)?;
    ev.with_bound(ends, |ev| joint_with(ev, name, kind, &args, span))
}

fn joint_with(ev: &mut Eval, name: &'static str, kind: JointKind, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(ev, name, joint_args(kind), args, span)?;
    let ra = reference(&a.req("a")?, a.span("a"))?;
    let rb = reference(&a.req("b")?, a.span("b"))?;
    let flip = a.flag("flip")?;
    let suppressed = a.flag("suppressed")?;
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
        suppressed,
        span,
    });
    Ok(asm(AsmVal::Link {
        id,
        joint: Some(kind),
    }))
}

fn constraint(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    let (args, ends) = with_body_ends(ev, args)?;
    ev.with_bound(ends, |ev| constraint_with(ev, name, &args, span))
}

fn constraint_with(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    let names: &[&'static str] = match name {
        "on_plane" | "mate" | "flush" | "insert" => &["a", "b", "offset", "suppressed"],
        "concentric" | "parallel" => &["a", "b", "opposed", "suppressed"],
        "angle" | "distance" => &["a", "b", "value", "suppressed"],
        _ => &["a", "b", "suppressed"],
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
    let suppressed = a.flag("suppressed")?;
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
        suppressed,
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

/// `std(FAMILY, size:, length:, thread:, grade:, sheet:, at:, ..)`: an instance of a standard
/// part, its file the key the tables generate it from (`stdparts`).
fn std_part(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    use crate::stdparts::{Located, Spans, Spec};
    let a = Args::bind(
        ev,
        "std",
        &[
            "family",
            "size",
            "length",
            "thread",
            "grade",
            "sheet",
            "at",
            "ground",
            "reference",
            "suppressed",
        ],
        args,
        span,
    )?;
    let text = |k: &'static str| -> R<Option<String>> {
        match a.opt(k) {
            None => Ok(None),
            Some(Value::Str(s)) => Ok(Some(s.to_string())),
            Some(o) => Err(wrong(a.span(k), k, Ty::Str, &o)),
        }
    };
    let family = match a.req("family")? {
        Value::Str(s) => s.to_string(),
        o => return Err(wrong(a.span("family"), "a standard", Ty::Str, &o)),
    };
    let size = match a.req("size")? {
        Value::Str(s) => s.to_string(),
        o => return Err(wrong(a.span("size"), "a size", Ty::Str, &o)),
    };
    let mut options = BTreeMap::new();
    for k in ["thread", "grade"] {
        if let Some(v) = text(k)? {
            options.insert(k.to_string(), v);
        }
    }
    if let Some(n) = a.num("sheet", LEN)? {
        options.insert("sheet".into(), crate::values::fmt(n.v));
    }
    let spec = Spec {
        family,
        size,
        length: a.num("length", LEN)?.map(|n| n.v),
        options,
    };
    let at = Spans {
        family: a.span("family"),
        size: a.span("size"),
        length: a.span("length"),
        options: a.span(
            ["thread", "grade", "sheet"]
                .into_iter()
                .find(|k| a.opt(k).is_some())
                .unwrap_or("size"),
        ),
    };
    let pose = match a.opt("at") {
        None => Frame::WORLD,
        Some(v) => pose_of(ev, v, a.span("at"))?,
    };
    let (ground, reference, suppressed) = (a.flag("ground")?, a.flag("reference")?, a.flag("suppressed")?);
    let id = ev.take_id("part", span)?;
    crate::stdparts::check(&spec, at).map_err(|err| Located {
        err,
        id: Some(id.clone()),
    })?;
    ev.record(&id, "instance", span, vec![]);
    doc(ev).insts.push(InstDef {
        id: id.clone(),
        file: crate::stdparts::key(&spec),
        source: Source::File,
        eval: None,
        body: None,
        pose,
        ground,
        reference,
        suppressed,
        sub: false,
        span,
    });
    Ok(asm(AsmVal::Inst(id)))
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
            local: None,
        }))),
        AsmVal::Ref(r) => {
            let mut r = r.clone();
            r.segs.push(seg);
            r.span = span;
            Ok(asm(AsmVal::Ref(r)))
        }
        AsmVal::Harness(crate::harness::HVal::Connector { h, k, pins }) => {
            Ok(crate::harness::pin_of(*h, *k, pins, Key::Label(name.into())))
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

/// A call on an instance or a reference, as its text (`plane(at: boss.end.center, normal: ..)`):
/// a reference whose last step the part file evaluates.
pub fn part_call_on(a: &Rc<AsmVal>, text: String, span: Span) -> R<Value> {
    let seg = Seg {
        name: text,
        index: vec![],
    };
    match &**a {
        AsmVal::Inst(id) => Ok(asm(AsmVal::Ref(RefDef {
            inst: id.clone(),
            segs: vec![seg],
            span,
            local: None,
        }))),
        AsmVal::Ref(r) => {
            let mut r = r.clone();
            r.segs.push(seg);
            r.span = span;
            Ok(asm(AsmVal::Ref(r)))
        }
        other => Err(Error::NotCallable {
            span,
            found: other.ty(),
        }),
    }
}

pub fn index(a: &Rc<AsmVal>, k: i64, span: Span) -> R<Value> {
    match &**a {
        AsmVal::Harness(crate::harness::HVal::Connector { h, k: c, pins }) => {
            Ok(crate::harness::pin_of(*h, *c, pins, Key::Ix(k)))
        }
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
    /// There, but no Forge file: not a plain `.fg`, too big, not text, or it does not parse.
    /// Nothing of it is said, nor anything worked out from it (a line, a length).
    NotForge,
    /// There, but outside the project (or the session's root) once links are resolved.
    Outside,
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
    /// `with:` names a param the file has no shared param for.
    NoParam {
        param: String,
    },
    /// A used file has no component of this name.
    NoComponent {
        name: String,
    },
    /// `version:` given, but the file's folder keeps no history.
    NoHistory,
    /// `version:` names no named version of the file; the newest there is.
    NoVersion {
        version: u32,
        newest: Option<u32>,
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
    /// An `expect .. at` whose params put a joint past its limit: the check is taken at a pose
    /// the joint's range forbids (or, where the solve holds the joint at the limit, not at the
    /// params at all). `limit` says which joint, the value and the range.
    AtPastLimit {
        limit: Box<Issue>,
    },
    /// A component of this document written wrong.
    Component(CompProblem),
    /// A wire harness's problem (`harness.rs`).
    Harness(crate::harness::HarnessProblem),
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
    /// An `at` param that is not this document's: a subassembly's drives are its own file's
    /// params, which a parent cannot set.
    AtUnknownParam { param: String },
    /// An `at` value that is not a number.
    AtNotANumber { param: String },
    /// `gap(a, b)` of planes that are not parallel, at this angle (degrees).
    NotParallel { angle: f64 },
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
            Unevaluable::AtUnknownParam { param } => format!(
                "at {param}: this document has no param {param} (a subassembly's drives are its own file's params, set in that file)"
            ),
            Unevaluable::AtNotANumber { param } => format!("at {param}: the value is not a number"),
            Unevaluable::NotParallel { angle } => {
                format!(
                    "gap() takes two parallel planes; these are {} deg apart",
                    forge_agent::short(*angle)
                )
            }
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
            Issue::Harness(p) => p.level(),
            Issue::AtPastLimit { .. } => Level::Warning,
            // nothing grounded: each partition held at its least occurrence, its freedoms free
            Issue::NoGround => Level::Warning,
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
            Issue::AtPastLimit { .. } => "AtPastLimit",
            Issue::Component(p) => p.kind(),
            Issue::Harness(p) => p.kind(),
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
            Issue::Harness(p) => p.fixes(),
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
        Issue::Component(p) => p.text(),
        Issue::Harness(p) => p.text(),
        Issue::WrongDocument { func, assembly: true } => {
            format!("{func} is for assembly documents (first line `assembly`)")
        }
        Issue::WrongDocument {
            func,
            assembly: false,
        } => format!("{func} is for part files, not assemblies"),
        Issue::PartFile { file, problem } => match problem {
            FileProblem::Missing => format!("{file}: no such file"),
            FileProblem::NotForge => {
                format!("{file} is not a Forge file this reads (a plain .fg of text that parses, at most 64 MB)")
            }
            FileProblem::Outside => format!("{file} is outside the session's root, not read"),
            FileProblem::Errors { count, first } => format!("{file}: {count} errors, the first {first}"),
            FileProblem::WrongKind { assembly: true } => format!("{file} is an assembly: use subassembly()"),
            FileProblem::WrongKind { assembly: false } => format!("{file} is a part: use part()"),
            FileProblem::Cyclic => format!("{file} contains itself"),
            FileProblem::NoBody { body } => format!("{file} has no body {body}"),
            FileProblem::NoParam { param } => format!("{file} has no shared param {param} for with:"),
            FileProblem::NoComponent { name } => format!("{file} has no component {name}"),
            FileProblem::NoHistory => format!("{file} has no versions: its folder keeps no history"),
            FileProblem::NoVersion {
                version,
                newest: Some(m),
            } => format!("{file} has no version {version} (the newest is {m})"),
            FileProblem::NoVersion { version, newest: None } => {
                format!("{file} has no version {version} (it has no named versions)")
            }
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
        Issue::NoGround => {
            "nothing is grounded: each part starts where it is written and its freedoms are free (ground one: `ground: true`, `ground(X)`)".into()
        }
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
        Issue::AtPastLimit { limit } => format!(
            "at these params {}: the check is taken outside the joint's range",
            issue_text(limit)
        ),
    }
}

fn issue_data(i: &Issue) -> Vec<(&'static str, Json)> {
    let s = |x: &str| Json::Str(x.into());
    let list = |xs: &[ItemName]| Json::Arr(xs.iter().map(ItemName::json).collect());
    match i {
        Issue::Harness(p) => p.data(),
        Issue::Component(p) => match p {
            CompProblem::Nested { func } => vec![("func", s(func))],
            CompProblem::BodyGone { id } => vec![("id", s(id))],
            CompProblem::NotYet { what } => vec![("what", s(what))],
            CompProblem::ReadsOutside { name, reader } => vec![("name", s(name)), ("reader", s(reader))],
            CompProblem::StillReadBy { name, readers } => vec![
                ("name", s(name)),
                ("readers", Json::Arr(readers.iter().map(|r| s(r)).collect())),
            ],
            CompProblem::ReadOtherwise { component, reader } => {
                vec![("component", s(component)), ("reader", s(reader))]
            }
            CompProblem::FileExists { file }
            | CompProblem::TargetIsLink { file }
            | CompProblem::BadFileName { file }
            | CompProblem::Outside { file } => vec![("file", s(file))],
            CompProblem::NotInlinable { file, why } => vec![("file", s(file)), ("why", s(why))],
            CompProblem::Cycle { a, b } => vec![("a", s(a)), ("b", s(b))],
            _ => vec![],
        },
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
        Issue::AtPastLimit { limit } => issue_data(limit),
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
    /// Its iProperties and solids with their materials, for the BOM and the assembly's mass.
    pub summary: Arc<crate::props::PartSummary>,
    /// The file is an assembly document, not a part.
    pub declared: bool,
    /// The interference meshes of `bodies`, by name.
    pub body_meshes: BTreeMap<String, Arc<PartMesh>>,
    /// Its text and everything it read, as one hash: the same ident, the same solids.
    pub ident: u128,
    /// Its threads left cosmetic, in its frame.
    pub threads: Vec<crate::holes::Cosmetic>,
    /// Its thermoforms' moulds: tooling, drawn only when asked.
    pub tooling: Option<Arc<forge_topo::Body>>,
    /// Its anchors, when asked for (`Options::comp_anchors`): its faces named as it names them.
    pub anchors: Option<Arc<crate::anchors::Anchors>>,
}

/// A call's params' values and the trace of each.
type Inputs = (Vec<(String, Value)>, Vec<String>);
/// An in-file component's load, with the stage, text hash and questions it was made from.
type InFile = (Rc<crate::build::Stage>, u64, Vec<String>, bool, Arc<PartLoad>);
/// A link's arguments with its body ends put in by name, and those names' values.
type Ends = (Vec<Arg>, Vec<(String, Value)>);
/// A part file's text hash and the references and bodies asked of it.
/// A part file, its text hash, the references and bodies asked of it, the fence, and whether
/// its threads are left cosmetic (a drawing's) and it is meshed (`Options::meshes`).
type PartKey = (
    String,
    u64,
    Vec<String>,
    Vec<String>,
    Option<std::path::PathBuf>,
    (bool, bool, bool),
);
/// A subassembly document evaluated, and its errors.
type SubLoad = (Doc, Vec<Error>);

/// A subassembly file holding components of its own: its evaluation, built, and each call's
/// document by its path, where its in-file parts are made (as this document's are here).
struct FileEnv {
    ev: Eval<'static>,
    prepared: BTreeMap<String, Doc>,
}

/// A component of another file, placed through that file as a subassembly: `FILE|#KEY`, the
/// file's own place and its key there (`#lid(w=40)`).
fn nested_key(file: &str) -> Option<(&str, &str)> {
    let k = file.find("|#")?;
    Some((&file[..k], &file[k + 1..]))
}

/// A component of this document or of a subassembly file: made by an evaluation, never read.
fn is_infile(file: &str) -> bool {
    file.starts_with('#') || nested_key(file).is_some()
}

/// `text` evaluated as a document of its own and built: its document ready to solve, its
/// in-file parts' evaluations, and its errors.
fn file_env(
    text: &str,
    dir: Option<std::path::PathBuf>,
    fence: Option<std::path::PathBuf>,
) -> (FileEnv, Doc, Vec<Error>) {
    let text = kept_text(text.to_string());
    let mut ev = Eval::new(text, crate::eval::Rules::default(), true, Default::default());
    ev.dir = dir;
    ev.root = fence;
    let prog = match crate::syntax::parse(text) {
        Ok(p) => p,
        Err(e) => {
            let env = FileEnv {
                ev,
                prepared: BTreeMap::new(),
            };
            return (env, Doc::default(), vec![e]);
        }
    };
    ev.asm = Some(Doc {
        declared: declared(&prog),
        ..Default::default()
    });
    ev.check_static(&prog);
    if !ev.errors.is_empty() {
        ev.b.building = false;
    }
    ev.run(&prog);
    // a subassembly file loaded into an assembly keeps no warnings of its own
    crate::sheet::finish_with(&mut ev, false);
    let d = ev.asm.take().unwrap_or_default();
    let d = in_file(&mut ev, d);
    let mut prepared = BTreeMap::new();
    prepare_all(&mut ev, &d, &mut prepared);
    let errors = std::mem::take(&mut ev.errors);
    (FileEnv { ev, prepared }, d, errors)
}

/// A subassembly file's document as placed from here: its own components named through it.
fn namespaced(d: &Doc, file: &str) -> Doc {
    let mut d = d.clone();
    for i in d.insts.iter_mut().filter(|i| i.file.starts_with('#')) {
        i.file = format!("{file}|{}", i.file);
    }
    d
}

/// Loaded documents and the last solve, kept across runs of one assembly.
#[derive(Default)]
pub struct AsmCache {
    parts: BTreeMap<PartKey, Arc<PartLoad>>,
    /// This document's own components, by file key: the stage they were read from (kept, so
    /// equal only while the build is), the text and questions, the load.
    infile: BTreeMap<String, InFile>,
    subs: BTreeMap<(String, u64, Option<std::path::PathBuf>), Arc<SubLoad>>,
    last: Option<Live>,
    /// Parts runs on other threads build at once (`at` runs): each built by one of them only.
    pool: Option<Arc<PartPool>>,
    /// What runs sharing a pool ask of each part file (`Wants`): collected by a first pass that
    /// builds nothing, or taken by the real run, which asks each file it uses for all of it.
    wants: Option<(Arc<Wants>, bool)>,
}

/// Each part file's references and bodies, over several documents' runs.
/// The file of each part load `pool` holds, in key order: one for each load made through it.
pub(crate) fn pool_files(pool: &PartPool) -> Vec<String> {
    pool.lock()
        .unwrap_or_else(|e| e.into_inner())
        .keys()
        .map(|k| k.0.clone())
        .collect()
}

pub(crate) type Wants = std::sync::Mutex<BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)>>;

pub(crate) type PartPool = std::sync::Mutex<BTreeMap<PartKey, Arc<std::sync::OnceLock<Arc<PartLoad>>>>>;

impl AsmCache {
    /// Nothing loaded yet, its parts built through `pool`: runs on other threads sharing it
    /// build each part (by its whole key) once between them.
    /// Nothing loaded yet, its parts built through `pool` (runs on other threads sharing it build
    /// each part, by its whole key, once between them), and what the runs sharing it want of each
    /// part file: with `collect`, the run
    /// only adds what it would ask to `wants` and builds nothing; without, it asks each part file
    /// it uses for all `wants` holds of it, so runs asking one file for different references
    /// make one load of it between them.
    pub(crate) fn with_wants(pool: Arc<PartPool>, wants: Arc<Wants>, collect: bool) -> AsmCache {
        AsmCache {
            pool: Some(pool),
            wants: Some((wants, collect)),
            ..AsmCache::default()
        }
    }
}

impl AsmCache {
    /// Joint values set straight in the kept solve, as a drag in the app sets them: `(joint id,
    /// freedom word, value)` with angles in radians. Only the partitions they reach are solved
    /// again; nothing is evaluated or checked. Each part occurrence's pose comes back in the order
    /// of the last run's `AsmOut::occurrences`. None: no solve kept, or a value isn't a drive
    /// of a top-level joint (that needs a run).
    pub fn drive(&mut self, drives: &[(String, String, f64)]) -> Option<Vec<Frame>> {
        let l = self.last.as_mut()?;
        for (joint, word, v) in drives {
            let id = fa::JointId(hid(joint));
            let j = l.asm.joints.iter_mut().find(|j| j.id == id)?;
            let f = freedom_words(j.kind).iter().find(|w| w.0 == word)?.1;
            // the kept document follows, so the next run sets back what it doesn't say
            j.drive.iter_mut().find(|d| d.0 == f)?.1 = *v;
            l.solver.set_drive(id, f, *v);
        }
        Some(l.solver.solution().occurrences.iter().map(|o| o.pose).collect())
    }
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
/// A file placed another way (`mug.fg|with:d=70`): its own name and the variant; a standard
/// part's key (`std:ISO 4762|M6|20`) is none.
fn variant_split(file: &str) -> Option<(&str, &str)> {
    if file.starts_with(crate::stdparts::KEY) {
        return None;
    }
    file.split_once('|')
}

/// What a placement names: a component of this file by its key (`#box(w=40)`, which the solve
/// finds among the document's evaluations, never on disk), else a part file as `locate` finds it.
fn locate_part(
    dir: &Option<std::path::PathBuf>,
    fence: &Option<std::path::PathBuf>,
    file: &str,
) -> Option<std::path::PathBuf> {
    if is_infile(file) {
        return Some(std::path::PathBuf::from(file));
    }
    locate(dir, fence, file)
}

pub(crate) fn locate(
    dir: &Option<std::path::PathBuf>,
    fence: &Option<std::path::PathBuf>,
    file: &str,
) -> Option<std::path::PathBuf> {
    // a component's key is no file: never read, never located as one
    if is_infile(file) {
        return None;
    }
    // a file placed another way (`mug.fg|with:..`): the file's own place
    if let Some((path, _)) = variant_split(file) {
        return locate(dir, fence, path);
    }
    // a pinned file, or a file a pinned document names: read from its save, never the disk
    if crate::pins::is_pinned(file)
        || dir
            .as_ref()
            .is_some_and(|d| crate::pins::is_pinned(&d.to_string_lossy()))
    {
        let p = match dir {
            Some(d) => d.join(file),
            None => std::path::PathBuf::from(file),
        };
        if p.extension().is_none_or(|x| x != "fg") {
            return None;
        }
        return crate::pins::locate(&p);
    }
    let key = (dir.clone(), fence.clone(), file.to_string());
    let hit = LOCATED.with(|m| m.borrow().as_ref().and_then(|m| m.get(&key).cloned()));
    let found = hit.unwrap_or_else(|| {
        let found = locate_now(dir, fence, file);
        LOCATED.with(|m| {
            if let Some(m) = m.borrow_mut().as_mut() {
                m.insert(key, found.clone());
            }
        });
        found
    });
    // read only once the fence, kind and size let it through; else only whether it is there
    if !file.starts_with(crate::stdparts::KEY) {
        match (&found, dir) {
            (Some(p), _) => crate::deps::file(p),
            (None, Some(d)) => crate::deps::looked(&d.join(file)),
            (None, None) => {}
        }
    }
    found
}

type LocateKey = (Option<std::path::PathBuf>, Option<std::path::PathBuf>, String);

thread_local! {
    /// Files located during one assembly's finish (its `at` runs too): each path resolved once.
    static LOCATED: std::cell::RefCell<Option<HashMap<LocateKey, Option<std::path::PathBuf>>>> =
        const { std::cell::RefCell::new(None) };
}

/// Keeps `locate`'s answers while alive; the outermost one clears them.
struct LocateScope(bool);

impl LocateScope {
    fn enter() -> LocateScope {
        LocateScope(LOCATED.with(|m| {
            let mut m = m.borrow_mut();
            let outer = m.is_none();
            if outer {
                *m = Some(HashMap::new());
            }
            outer
        }))
    }
}

impl Drop for LocateScope {
    fn drop(&mut self) {
        if self.0 {
            LOCATED.with(|m| *m.borrow_mut() = None);
        }
    }
}

fn locate_now(
    dir: &Option<std::path::PathBuf>,
    fence: &Option<std::path::PathBuf>,
    file: &str,
) -> Option<std::path::PathBuf> {
    // a standard part's key stands for its generated text, wherever the document is
    if file.starts_with(crate::stdparts::KEY) {
        return crate::stdparts::text_of(file).map(|_| std::path::PathBuf::from(file));
    }
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

/// Why a file did not locate: outside the fence (links resolved), or not there to read.
fn not_located(
    dir: &Option<std::path::PathBuf>,
    fence: &Option<std::path::PathBuf>,
    file: &str,
) -> FileProblem {
    let file = variant_split(file).map_or(file, |v| v.0);
    if crate::outside(dir.as_deref(), fence.as_deref(), file) {
        return FileProblem::Outside;
    }
    // something there the loader refused: not a plain .fg, or too big; read no further
    match dir.as_ref().map(|d| d.join(file)) {
        Some(p) if std::fs::symlink_metadata(&p).is_ok() => FileProblem::NotForge,
        _ => FileProblem::Missing,
    }
}

/// A located file's text, read no further than `LIMIT`: only a plain file that is UTF-8 and
/// parses as Forge. Anything else is no Forge file: nothing of it goes anywhere, nor anything
/// worked out from it (where it stops parsing, how long it is).
pub fn read_capped(p: &std::path::Path) -> Option<String> {
    // a component's key is never a file to read
    if p.to_str().is_none_or(is_infile) {
        return None;
    }
    if let Some(k) = p.to_str().filter(|k| k.starts_with(crate::stdparts::KEY)) {
        return crate::stdparts::text_of(k);
    }
    if p.to_str().is_some_and(crate::pins::is_pinned) {
        let s = crate::pins::read(p).filter(|s| s.len() as u64 <= LIMIT)?;
        crate::syntax::parse(&s).ok()?;
        return Some(s);
    }
    // a link inside the fence is the file it names (the loaders' fence looked through it):
    // read as it is, a regular file, capped (deps::read_plain)
    let bytes = std::fs::canonicalize(p)
        .ok()
        .and_then(|real| crate::deps::read_plain(&real, LIMIT).ok());
    crate::deps::read(p, bytes.as_deref());
    if let Some(d) = p.parent() {
        crate::deps::materials(Some(d));
    }
    let s = String::from_utf8(bytes?).ok()?;
    crate::syntax::parse(&s).ok()?;
    Some(s)
}

/// Why a file located but did not read: not there, or there and no Forge file.
fn read_problem(p: &std::path::Path) -> FileProblem {
    if std::fs::symlink_metadata(p).is_err() {
        return FileProblem::Missing;
    }
    FileProblem::NotForge
}

/// The key of a file that could not be located: it reads as missing everywhere.
fn unlocated(file: &str) -> String {
    format!("?{file}")
}

fn located(dir: &Option<std::path::PathBuf>, fence: &Option<std::path::PathBuf>, file: &str) -> String {
    // a component of this document, or of a subassembly file
    if is_infile(file) {
        return file.to_string();
    }
    // a file placed another way: its own place, the variant kept
    if let Some((path, variant)) = variant_split(file) {
        return format!("{}|{variant}", located(dir, fence, path));
    }
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
    // a thermoform's mould is tooling: not drawn with the part it formed (`tooling_body` is it)
    let moulds: Vec<usize> = ev.thermos.iter().map(|t| t.mould_solid).collect();
    let st = ev.b.stages.last()?;
    let out = |s: &forge_model::Solid| flats.contains(&s.id) || moulds.contains(&s.id);
    if !st.solids.iter().any(out) {
        return ev.b.body().cloned();
    }
    let kept: Vec<&forge_model::Solid> = st.solids.iter().filter(|s| !out(s)).collect();
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

/// Each in-file component call's anchors, found on its own evaluation, by the occurrence file an
/// assembly names its parts with (`#lid`, `#box(w=60)/lid(w=60)`), calls inside calls too.
pub fn comp_anchors(comps: &mut [Comp]) -> Vec<(String, crate::anchors::Anchors)> {
    let mut out = Vec::new();
    for c in comps {
        let file = format!("#{}", c.ev.comp_path.trim_end_matches('/'));
        let body = placed_body(&c.ev);
        if body.is_some() {
            out.push((file, crate::anchors::find(&mut c.ev, body.as_ref())));
        }
        out.extend(comp_anchors(&mut c.ev.comps));
    }
    out
}

/// A part's thermoform moulds, the tooling `placed_body` leaves out.
fn tooling_body(ev: &Eval) -> Option<forge_topo::Body> {
    let st = ev.b.stages.last()?;
    let mut all: Option<forge_topo::Body> = None;
    for t in &ev.thermos {
        if let Some(s) = st.solids.iter().find(|s| s.id == t.mould_solid) {
            let b = forge_model::finish(s.body.clone());
            match &mut all {
                Some(a) => a.append(&b),
                None => all = Some(b),
            }
        }
    }
    all
}

/// A part file's declared anchors (loops' too: `stud[2]`) as frames in the part's own space: a
/// plane's frame, an axis's along its z, a point's at its origin. What the app draws them by.
pub fn part_anchors(text: &str, dir: Option<std::path::PathBuf>) -> Vec<(String, Frame)> {
    let p = load_part_with(text, dir, None, &[], &[], false, (false, false));
    p.refs
        .into_iter()
        .filter_map(|(n, r)| r.ok().map(|r| (n, r.frame)))
        .collect()
}

fn load_part_with(
    text: &str,
    dir: Option<std::path::PathBuf>,
    fence: Option<std::path::PathBuf>,
    refs: &[String],
    bodies: &[String],
    mesh: bool,
    // its threads left cosmetic; its anchors found
    (cosmetic, anchors): (bool, bool),
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
        summary: Default::default(),
        declared: false,
        body_meshes: BTreeMap::new(),
        ident: 0,
        threads: vec![],
        tooling: None,
        anchors: None,
    };
    let watch = crate::deps::watch();
    let prog = match crate::syntax::parse(text) {
        Ok(p) => p,
        Err(e) => {
            out.errors.push(e);
            return out;
        }
    };
    if has_components(&prog) {
        out.declared = true;
        drop(watch);
        return out;
    }
    let mut ev = Eval::new(text, crate::eval::Rules::default(), true, Default::default());
    ev.dir = dir;
    ev.root = fence;
    ev.cosmetic_threads = cosmetic;
    // its component definitions are allowed (they place nothing)
    ev.asm = Some(Doc::default());
    ev.check_static(&prog);
    if !ev.errors.is_empty() {
        ev.b.building = false;
    }
    ev.run(&prog);
    out.threads = std::mem::take(&mut ev.threads);
    // a part's warnings stay with it: an assembly keeps none of them, so its shop rules go unasked
    crate::sheet::finish_with(&mut ev, false);
    finish_load(&mut ev, &mut out, refs, bodies, mesh);
    if anchors {
        out.anchors = Some(Arc::new(crate::anchors::find(&mut ev, out.body.as_ref())));
    }
    out.errors = std::mem::take(&mut ev.errors);
    let read = watch.done();
    out.ident = crate::store::key("part", |h| {
        h.str(text);
        h.u128(read.digest());
    });
    out
}

/// A part evaluated (a file's, or an inline component's): its body, summary, bodies asked for,
/// declared anchors and kinds, the references asked for, and its interference mesh.
fn finish_load(ev: &mut Eval, out: &mut PartLoad, refs: &[String], bodies: &[String], mesh: bool) {
    out.body = placed_body(ev);
    out.tooling = tooling_body(ev).map(Arc::new);
    out.summary = Arc::new(crate::props::PartSummary::of(ev));
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
                    let r = as_reference(ev, reference.clone());
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
        // a path (`shank.axis`, `stud[2]`), or a call's text: the part's own expression
        let e = if r.contains('(') {
            let Some(e) = crate::syntax::parse(&format!("ref_ = {r}\n"))
                .ok()
                .and_then(|p| p.into_iter().next())
                .and_then(|s| match s.kind {
                    crate::syntax::StmtKind::Assign { value, .. } => Some(value),
                    _ => None,
                })
            else {
                continue;
            };
            e
        } else {
            let Some(segs) = crate::error::parse_path(r) else {
                continue;
            };
            ref_expr(&segs)
        };
        let (errors, warnings) = (ev.errors.len(), ev.warnings.len());
        let got = ev.expr(&e);
        // a call's probe leaves nothing behind in the part's own results
        if r.contains('(') {
            ev.errors.truncate(errors);
            ev.warnings.truncate(warnings);
        }
        if let Ok(v) = got {
            match as_reference(ev, v) {
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
    if mesh {
        out.mesh = out.body.as_ref().and_then(interference_mesh);
        out.body_meshes = out
            .bodies
            .iter()
            .filter_map(|(n, b)| Some((n.clone(), interference_mesh(b)?)))
            .collect();
    }
}

/// A part file's text as a placement asks for it: its own place (`mug.fg|with:..` is
/// `mug.fg`), the variant the document wrote, and the text with `with:`'s params set (Err: a param
/// the file has not). The text is None where the file can't be read.
fn variant_text<'d>(
    d: &'d Doc,
    dir: &Option<std::path::PathBuf>,
    fence: &Option<std::path::PathBuf>,
    file: &str,
) -> Result<(String, Option<&'d Variant>, Option<String>), String> {
    let path = variant_split(file).map_or(file, |x| x.0).to_string();
    let variant = d
        .variants
        .iter()
        .find(|(k, _)| located(dir, fence, k) == *file)
        .map(|x| x.1);
    let text = if path.starts_with('?') {
        None
    } else {
        read_capped(std::path::Path::new(&path))
    };
    let text = match (text, variant) {
        (Some(mut t), Some(Variant::With(xs))) => {
            for (n, l) in xs {
                t = crate::edit::apply(
                    &t,
                    &Op::SetParam {
                        param: ParamKey::shared(n),
                        value: *l,
                    },
                )
                .map_err(|_| n.clone())?;
            }
            Some(t)
        }
        (t, _) => t,
    };
    Ok((path, variant, text))
}

/// A part file loaded as its placement asks: its default part, or one of its components called.
fn make_load(
    text: &str,
    path: &str,
    variant: Option<&Variant>,
    fence: &Option<std::path::PathBuf>,
    refs: &[String],
    bodies: &[String],
    // its threads left cosmetic, its interference meshes made, its anchors found: as the part
    // key has them
    (cosmetic, mesh, anchors): (bool, bool, bool),
) -> PartLoad {
    let parent = std::path::Path::new(path).parent().map(|p| p.to_path_buf());
    match variant {
        Some(Variant::Component { name, args }) => {
            load_component(text, parent, fence.clone(), name, args, refs, (mesh, anchors))
        }
        _ => load_part_with(
            text,
            parent,
            fence.clone(),
            refs,
            bodies,
            mesh,
            (cosmetic, anchors),
        ),
    }
}

/// One component of a part file, called with `args` (the rest its defaults), as a part: its
/// own evaluation over that file's text, the file's shared params and functions its scope.
fn load_component(
    text: &str,
    dir: Option<std::path::PathBuf>,
    fence: Option<std::path::PathBuf>,
    name: &str,
    args: &[(String, Literal)],
    refs: &[String],
    (mesh, anchors): (bool, bool),
) -> PartLoad {
    let mut out = empty_load(&format!("{name}@{text}"));
    out.hash = content_hash(&format!("{name}({args:?})\n{text}"));
    let prog = match crate::syntax::parse(text) {
        Ok(p) => p,
        Err(e) => {
            out.errors.push(e);
            return out;
        }
    };
    let mut ev = Eval::new(text, crate::eval::Rules::default(), false, Default::default());
    ev.dir = dir;
    ev.root = fence;
    ev.asm = Some(Doc::default());
    // its scope: the file's shared params, functions and uses, and the definition itself
    ev.run(&component_scope(&prog, name));
    let def = ev.top_level().into_iter().find_map(|(n, v)| match v {
        Value::Asm(a) if n == name => match &*a {
            AsmVal::Def(d) => Some(d.clone()),
            _ => None,
        },
        _ => None,
    });
    let Some(def) = def else {
        out.errors.push(note(
            Span::default(),
            Issue::PartFile {
                file: name.into(),
                problem: FileProblem::NoComponent { name: name.into() },
            },
        ));
        return out;
    };
    let span = def.span;
    let mut values = match call_values(&mut ev, &def, &[], span) {
        Ok(v) => v,
        Err(e) => {
            out.errors.push(e);
            return out;
        }
    };
    for (n, l) in args {
        match values.iter_mut().find(|v| v.0 == *n) {
            Some(v) => {
                let x = if l.dim == Dim::Angle {
                    l.value.to_radians()
                } else {
                    l.value
                };
                v.1 = Value::Num(Num::konst(x, l.dim));
            }
            None => {
                out.errors.push(Error::UnknownArg {
                    span,
                    func: "component",
                    arg: n.clone(),
                });
                return out;
            }
        }
    }
    // a part loaded into an assembly: none of its warnings are kept
    let mut sub = sub_eval(&ev, &def, &values, true, Default::default(), false);
    finish_load(&mut sub, &mut out, refs, &[], mesh);
    if anchors {
        out.anchors = Some(Arc::new(crate::anchors::find(&mut sub, out.body.as_ref())));
    }
    out.errors = std::mem::take(&mut sub.errors);
    out.errors.extend(std::mem::take(&mut ev.errors));
    out
}

fn interference_mesh(b: &forge_topo::Body) -> Option<Arc<PartMesh>> {
    let q = forge_mesh::Quality {
        chord: MESH_CHORD,
        max_angle: forge_math::dmath::PI / 12.0,
    };
    let m = forge_mesh::tessellate(b, &q).ok()?;
    Some(Arc::new(PartMesh::from_trimesh(&m, MESH_CHORD)))
}

/// The interference meshes' chord tolerance (mm): overlaps shallower than about twice this may
/// go unseen, and the report says so per pair.
pub const MESH_CHORD: f64 = 0.01;

/// How far a `clearance` expect looks: everywhere.
const CLEARANCE_REACH: f64 = 1e9;

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
    /// Part files built, these runs together: one per file or standard part key, however many
    /// occurrences place it; none for one the last run built.
    pub built: usize,
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
    /// The top-level instances that are grounded (`ground: true`), by id.
    pub grounded: Vec<String>,
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
    /// Each of `parts`' identity (its document, all it read, its body): 0 where unknown.
    pub part_idents: Vec<u128>,
    /// Each of `parts`' threads left cosmetic (`Options::cosmetic_threads`), in its frame.
    pub part_threads: Vec<Arc<Vec<crate::holes::Cosmetic>>>,
    pub occ_part: Vec<Option<usize>>,
    /// The probed expects' values, by position (`Options::probe`).
    pub probes: Vec<(usize, Option<f64>)>,
    /// The structure as STEP AP214, when asked for (`Options::step`).
    pub step: Option<Result<String, String>>,
    /// Every expect line: measured, against its limit, passed or not.
    pub expects: Vec<ExpectOut>,
    /// Every rule-covered thread pair's engagement, short or not.
    pub engagements: Vec<EngagementOut>,
    /// Every part and subassembly occurrence, in the order the documents make them.
    pub insts: Vec<InstOut>,
    /// The top document's components, where each comes from (`insts`' top-level ones).
    pub sources: Vec<(String, Source)>,
    /// Each occurrence of a call of a component of this file, and the evaluation it is.
    pub evals: Vec<(String, String)>,
    /// Every joint, constraint, relation, fit and fit rule, through subassemblies.
    pub links: Vec<LinkOut>,
    /// Where each of `links` is in the world, solved: a joint's or constraint's two ends' frames
    /// (the anchors it ties); None for an end that is not an anchor on a placed part.
    pub link_at: Vec<[Option<Frame>; 2]>,
    /// Each part file's iProperties and solids, by the file as `occurrences` names it.
    pub part_docs: BTreeMap<String, Arc<crate::props::PartSummary>>,
    /// Each occurrence's body (0: the part as made, else a `body:` by its name's hash) and
    /// whether it is a reference (out of the BOM and the mass).
    pub occ_body: Vec<(u64, bool)>,
    /// Each occurrence's part's thermoform moulds (tooling, not in `draw`), if it has any.
    pub occ_tooling: Vec<Option<Arc<forge_topo::Body>>>,
    /// With `Options::comp_anchors`: each occurrence's part file's anchors (an in-file component's
    /// are `Run::comp_anchors`).
    pub occ_anchors: Vec<Option<Arc<crate::anchors::Anchors>>>,
    /// The document's wire harnesses, routed on the solved assembly.
    pub harnesses: Vec<crate::harness::HarnessOut>,
}

/// A part or subassembly occurrence, as a browser lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct InstOut {
    /// Its path through subassemblies (`post_fl.stud_a[0]`).
    pub path: String,
    /// The file as its document wrote it.
    pub file: String,
    pub sub: bool,
    /// Grounded in its own document.
    pub ground: bool,
    pub suppressed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LinkFamily {
    Joint,
    Constraint,
    Relation,
    Fit,
    FitRule,
}

/// A joint, loose constraint, relation, fit or fit rule, and what it ties.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkOut {
    /// The subassembly occurrence whose document writes it; empty for the top document.
    pub owner: String,
    pub id: String,
    pub family: LinkFamily,
    pub kind: String,
    /// Its two sides: an occurrence path and its anchor (a joint end or constraint), a joint and
    /// its value (a relation), an occurrence (a fit), or what a fit rule matches.
    pub a: (String, String),
    pub b: (String, String),
    pub suppressed: bool,
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

pub(crate) fn hid(id: &str) -> u64 {
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
        // on a body of this document: worked out where it was written
        if let Some(l) = &r.local {
            return fa::End {
                path: vec![fa::InstanceId(hid(&r.inst))],
                anchor: fa::AnchorRef::Local(*l),
            };
        }
        let mut path = vec![fa::InstanceId(hid(&r.inst))];
        let mut cur = d.insts.iter().find(|x| x.id == r.inst);
        let mut cur_dir = dir.clone();
        let mut segs: &[Seg] = &r.segs;
        while let Some(c) = cur.filter(|c| c.sub) {
            let Some(file) = locate_part(&cur_dir, fence, &c.file) else {
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
                    // an in-file subassembly's parts are found where the document's are
                    if !is_infile(&c.file) {
                        cur_dir = file.parent().map(|p| p.to_path_buf());
                    }
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
    // a suppressed joint or constraint is listed, not solved
    for (n, l) in d.links.iter().enumerate().filter(|(_, l)| !l.suppressed) {
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
                    suppressed: l.suppressed,
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
                    suppressed: l.suppressed,
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
            within: vec![],
            a: m(&r.a),
            b: m(&r.b),
            min_engagement: r.min_engagement,
        });
    }
    out
}

/// A document's occurrences and links, and its subassemblies' under their occurrences' paths.
#[allow(clippy::too_many_arguments)]
fn structure(
    d: &Doc,
    owner: &str,
    dir: &Option<std::path::PathBuf>,
    fence: &Option<std::path::PathBuf>,
    subs: &BTreeMap<String, Arc<SubLoad>>,
    depth: usize,
    insts: &mut Vec<InstOut>,
    links: &mut Vec<LinkOut>,
) {
    let join = |id: &str| {
        if owner.is_empty() {
            id.to_string()
        } else {
            format!("{owner}.{id}")
        }
    };
    let sub_of = |dir: &Option<std::path::PathBuf>, file: &str| {
        let p = locate_part(dir, fence, file)?;
        let s = subs.get(&p.to_string_lossy().to_string())?;
        if is_infile(file) {
            return Some((s, dir.clone()));
        }
        Some((s, p.parent().map(|x| x.to_path_buf())))
    };
    for i in &d.insts {
        let path = join(&i.id);
        insts.push(InstOut {
            path: path.clone(),
            file: i.file.clone(),
            sub: i.sub,
            ground: i.ground,
            suppressed: i.suppressed,
        });
        // a cycle is refused elsewhere; a bound keeps this from following one
        if i.sub
            && depth < 16
            && let Some((s, sdir)) = sub_of(dir, &i.file)
        {
            structure(&s.0, &path, &sdir, fence, subs, depth + 1, insts, links);
        }
    }
    // an end's occurrence path through subassemblies, and the anchor left on it
    let end = |r: &RefDef| -> (String, String) {
        let mut names = vec![r.inst.clone()];
        let mut cur = d.insts.iter().find(|x| x.id == r.inst);
        let mut cur_dir = dir.clone();
        let mut segs: &[Seg] = &r.segs;
        while let Some(c) = cur.filter(|c| c.sub) {
            let Some((s, sdir)) = sub_of(&cur_dir, &c.file) else {
                break;
            };
            let Some(first) = segs.first() else { break };
            let text = path_text(std::slice::from_ref(first));
            let Some(x) = s.0.insts.iter().find(|x| x.id == text) else {
                break;
            };
            names.push(x.id.clone());
            cur = Some(x);
            cur_dir = sdir;
            segs = &segs[1..];
        }
        (join(&names.join(".")), path_text(segs))
    };
    for l in &d.links {
        let (family, kind) = match l.kind {
            LinkKind::Joint(k) => (LinkFamily::Joint, k.name()),
            LinkKind::Constraint(k) => (LinkFamily::Constraint, k.name()),
        };
        links.push(LinkOut {
            owner: owner.to_string(),
            id: l.id.clone(),
            family,
            kind: kind.to_string(),
            a: end(&l.a),
            b: end(&l.b),
            suppressed: l.suppressed,
        });
    }
    let value = |(joint, f): &(String, Freedom)| {
        let word = d
            .links
            .iter()
            .find(|l| l.id == *joint)
            .and_then(|l| match l.kind {
                LinkKind::Joint(k) => Some(freedom_word(k, *f)),
                LinkKind::Constraint(_) => None,
            })
            .unwrap_or(f.name());
        (join(joint), word.to_string())
    };
    for r in &d.relations {
        links.push(LinkOut {
            owner: owner.to_string(),
            id: r.id.clone(),
            family: LinkFamily::Relation,
            kind: r.kind.name().to_string(),
            a: value(&r.leader),
            b: value(&r.follower),
            suppressed: false,
        });
    }
    for f in &d.fits {
        links.push(LinkOut {
            owner: owner.to_string(),
            id: f.id.clone(),
            family: LinkFamily::Fit,
            kind: "fit".into(),
            a: (join(&f.a), String::new()),
            b: (join(&f.b), String::new()),
            suppressed: false,
        });
    }
    let matcher = |m: &Matcher| match m {
        Matcher::Kind(k) => k.name().to_string(),
        Matcher::File(f) => f.clone(),
    };
    for r in &d.rules {
        links.push(LinkOut {
            owner: owner.to_string(),
            id: r.id.clone(),
            family: LinkFamily::FitRule,
            kind: "fit_rule".into(),
            a: (matcher(&r.a), String::new()),
            b: (matcher(&r.b), String::new()),
            suppressed: false,
        });
    }
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
    // a part: nothing to place or solve
    if d.is_empty() {
        return (None, cache);
    }
    let mut cache = cache.unwrap_or_default();
    if !opts.build || !ev.errors.is_empty() {
        ev.asm = Some(d);
        return (None, Some(cache));
    }
    let _located = LocateScope::enter();
    crate::geomemo::recall();
    let mut d = in_file(ev, d);
    // each call's own components, nested: their documents by their evaluations' paths
    let mut prepared: BTreeMap<String, Doc> = BTreeMap::new();
    prepare_all(ev, &d, &mut prepared);
    if !ev.errors.is_empty() {
        ev.asm = Some(d);
        return (None, Some(cache));
    }
    let mut t = AsmTimings::default();
    let t0 = Instant::now();
    let dir = opts.dir.clone();
    // the session's root: nothing outside it is read
    let fence = crate::fence_of(opts);
    let mut names = Names::default();

    // subassemblies, depth first, each document once; a cycle is refused
    let mut subs: BTreeMap<String, Arc<SubLoad>> = BTreeMap::new();
    let mut envs: BTreeMap<String, FileEnv> = BTreeMap::new();
    let mut stack: Vec<(Doc, Option<std::path::PathBuf>, Vec<String>)> =
        vec![(d.clone(), dir.clone(), vec![])];
    while let Some((doc_, here, chain)) = stack.pop() {
        for i in doc_.insts.iter().filter(|i| i.sub) {
            // a call of a component of this file that places or joins its own: its evaluation's
            // document, ready to solve
            if is_infile(&i.file) {
                let key = i.file.clone();
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
                // of this document, or of the subassembly file it is placed through
                let (rest, owner) = match nested_key(&key) {
                    Some((f, r)) => (r, Some(f)),
                    None => (key.as_str(), None),
                };
                let path = format!("{}/", rest.trim_start_matches('#'));
                let found = match owner {
                    Some(f) => envs
                        .get(f)
                        .and_then(|e| e.prepared.get(&path))
                        .map(|x| namespaced(x, f)),
                    None => prepared.get(&path).cloned(),
                };
                if !subs.contains_key(&key)
                    && let Some(doc) = found
                {
                    let loaded = Arc::new((doc, vec![]));
                    subs.insert(key.clone(), loaded.clone());
                    let mut c = chain.clone();
                    c.push(key);
                    stack.push((loaded.0.clone(), here.clone(), c));
                }
                continue;
            }
            let Some(p) = locate(&here, &fence, &i.file) else {
                ev.errors.push(note(
                    i.span,
                    Issue::PartFile {
                        file: i.file.clone(),
                        problem: not_located(&here, &fence, &i.file),
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
                        problem: read_problem(&p),
                    },
                ));
                continue;
            };
            let parent = p.parent().map(|x| x.to_path_buf());
            let ck = (key.clone(), text_hash(&text), fence.clone());
            let loaded = match cache.subs.get(&ck) {
                Some(x) => x.clone(),
                None => {
                    let is_asm = crate::syntax::parse(&text).is_ok_and(|p| has_components(&p));
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
            // components of its own: the file evaluated and built as a document, its parts made
            // there, named through it
            let own = |r: &RefDef| loaded.0.insts.iter().any(|x| x.id == r.inst);
            let loaded = if loaded.0.insts.iter().any(|x| x.source != Source::File)
                || loaded.0.links.iter().any(|l| !own(&l.a) || !own(&l.b))
            {
                let (env, doc, errors) = file_env(&text, parent.clone(), fence.clone());
                envs.insert(key.clone(), env);
                Arc::new((namespaced(&doc, &key), errors))
            } else {
                loaded
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
    // each subassembly's fit and rule names, for what its fits say when used from here
    let mut sub_fit_names: BTreeMap<fa::DocId, (Vec<String>, BTreeMap<u64, String>)> = BTreeMap::new();
    for (key, sub) in &subs {
        let sdir = match nested_key(key) {
            _ if key.starts_with('#') => dir.clone(),
            Some((file, _)) => std::path::Path::new(file).parent().map(|p| p.to_path_buf()),
            None => std::path::Path::new(key).parent().map(|p| p.to_path_buf()),
        };
        let mut sn = Names::default();
        let a = build_doc(&sub.0, &dir, &fence, &sdir, &subs, &mut want, &mut sn);
        let doc = fa::DocId(short(&dir, key).into());
        sub_fit_names.insert(
            doc.clone(),
            (
                sn.fits.iter().map(|f| f.0.clone()).collect(),
                sn.rule.iter().map(|(k, v)| (*k, v.0.clone())).collect(),
            ),
        );
        catalog.assemblies.insert(doc, a);
    }

    // what other runs sharing the pool ask of the same files: a first pass only says what it
    // would ask; a run then asks each of its files for everything any of them asks
    match &cache.wants {
        Some((all, true)) => {
            let mut all = all.lock().unwrap_or_else(|e| e.into_inner());
            // a file's own components (`#lid`) are named by the document they are in, and never
            // loaded through the pool: no other document's are the same
            for (file, (refs, bodies)) in want.iter().filter(|(f, _)| !f.starts_with('#')) {
                let e = all.entry(file.clone()).or_default();
                e.0.extend(refs.iter().cloned());
                e.1.extend(bodies.iter().cloned());
            }
            drop(all);
            ev.asm = Some(d);
            return (None, Some(cache));
        }
        Some((all, false)) => {
            let all = all.lock().unwrap_or_else(|e| e.into_inner());
            for (file, (refs, bodies)) in want.iter_mut().filter(|(f, _)| !f.starts_with('#')) {
                if let Some((r, b)) = all.get(file) {
                    refs.extend(r.iter().cloned());
                    bodies.extend(b.iter().cloned());
                }
            }
        }
        None => {}
    }

    // parts: built once per text and question, by file
    let mut exact = BrepExact::default();
    let mut loads: Vec<usize> = vec![];
    let mut part_docs: BTreeMap<String, Arc<crate::props::PartSummary>> = BTreeMap::new();
    let mut tooling: BTreeMap<String, Arc<forge_topo::Body>> = BTreeMap::new();
    let mut anchors_of: BTreeMap<String, Arc<crate::anchors::Anchors>> = BTreeMap::new();
    let mut declared_kinds: BTreeSet<String> = BTreeSet::new();
    let mut threads_of: BTreeMap<(fa::DocId, u64), Arc<Vec<crate::holes::Cosmetic>>> = BTreeMap::new();
    // the parts not yet built, built on all cores first
    let mut missing: Vec<(PartKey, String)> = vec![];
    for (file, (refs, bodies)) in &want {
        // this file's own components are its evaluations', never read
        if is_infile(file) {
            continue;
        }
        let Ok((_, _, Some(text))) = variant_text(&d, &dir, &fence, file) else {
            continue;
        };
        let ck = (
            file.clone(),
            text_hash(&text),
            refs.iter().cloned().collect(),
            bodies.iter().cloned().collect(),
            fence.clone(),
            (opts.cosmetic_threads, opts.meshes, opts.comp_anchors),
        );
        if !cache.parts.contains_key(&ck) && !missing.iter().any(|m| m.0 == ck) {
            missing.push((ck, text));
        }
    }
    let pool = cache.pool.clone();
    let built = crate::par::map(missing.len(), |i| {
        let (ck, text) = &missing[i];
        let (file, _, refs, bodies, fence, (cosmetic, meshed, named)) = ck;
        let make = || {
            let path = variant_split(file).map_or(file.as_str(), |x| x.0);
            let variant = d
                .variants
                .iter()
                .find(|(k, _)| located(&dir, fence, k) == *file)
                .map(|x| x.1);
            Arc::new(make_load(
                text,
                path,
                variant,
                fence,
                refs,
                bodies,
                (*cosmetic, *meshed, *named),
            ))
        };
        let Some(pool) = &pool else { return (make(), true) };
        let cell = pool
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(ck.clone())
            .or_default()
            .clone();
        let mut mine = false;
        let l = cell.get_or_init(|| {
            mine = true;
            make()
        });
        (l.clone(), mine)
    });
    for ((ck, _), (l, mine)) in missing.into_iter().zip(built) {
        t.built += mine as usize;
        cache.parts.insert(ck, l);
    }
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
        let refs: Vec<String> = refs.iter().cloned().collect();
        let bodies: Vec<String> = bodies.iter().cloned().collect();
        let load = if file.starts_with('#') {
            infile_load(ev, &prepared, file, &refs, &mut cache, opts.meshes)
        } else if let Some((owner, key)) = nested_key(file) {
            // a subassembly file's own component: made by that file's evaluation
            match envs.get_mut(owner) {
                Some(FileEnv { ev: fe, prepared: fp }) => {
                    infile_load(fe, fp, key, &refs, &mut cache, opts.meshes)
                }
                None => Arc::new(empty_load(file)),
            }
        } else {
            let (path, variant, text) = match variant_text(&d, &dir, &fence, file) {
                Ok(x) => x,
                Err(n) => {
                    ev.errors.push(note(
                        span,
                        Issue::PartFile {
                            file: shown.clone(),
                            problem: FileProblem::NoParam { param: n },
                        },
                    ));
                    continue;
                }
            };
            let Some(text) = text else {
                // located and read but no Forge file, or never located
                let problem = match std::path::Path::new(&path) {
                    p if p.is_absolute() && std::fs::symlink_metadata(p).is_ok() => read_problem(p),
                    _ => named.map_or(FileProblem::Missing, |i| not_located(&dir, &fence, &i.file)),
                };
                ev.errors
                    .push(note(span, Issue::PartFile { file: shown, problem }));
                continue;
            };
            let ck = (
                file.clone(),
                text_hash(&text),
                refs.clone(),
                bodies.clone(),
                fence.clone(),
                (opts.cosmetic_threads, opts.meshes, opts.comp_anchors),
            );
            match cache.parts.get(&ck) {
                Some(l) => l.clone(),
                None => {
                    let l = Arc::new(make_load(
                        &text,
                        &path,
                        variant,
                        &fence,
                        &refs,
                        &bodies,
                        (opts.cosmetic_threads, opts.meshes, opts.comp_anchors),
                    ));
                    cache.parts.insert(ck, l.clone());
                    t.built += 1;
                    l
                }
            }
        };
        loads.push(Arc::as_ptr(&load) as usize);
        if load.declared {
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
        part_docs.insert(doc.0.to_string(), load.summary.clone());
        // and by its identity, what the BOM's rows name
        part_docs
            .entry(bom_key(&doc.0))
            .or_insert_with(|| load.summary.clone());
        // a part's threads are on its own body, not on a named body it also has
        threads_of.insert((doc.clone(), 0), Arc::new(load.threads.clone()));
        if let Some(t) = &load.tooling {
            tooling.insert(doc.0.to_string(), t.clone());
        }
        if let Some(a) = &load.anchors {
            anchors_of.insert(doc.0.to_string(), a.clone());
        }
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
                load.body_meshes
                    .iter()
                    .find(|(n, _)| hid(n) == k)
                    .map(|(_, m)| m.clone())
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
                exact.idents.insert(
                    (doc.clone(), k),
                    crate::store::key("body", |h| {
                        h.u128(load.ident);
                        h.word(k);
                    }),
                );
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
    // STEP of the structure, from the documents as built (forge-asm solves for the poses)
    let step = opts
        .step
        .as_ref()
        .map(|name| fa::step::export(&top, name, &catalog, &exact.bodies).map_err(|e| format!("{e:?}")));
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
    let every_fit = all_fits(live_asm, checker.parts());
    // the subassemblies' fits and rules by their names under their use (`c.f_ub`), shown where
    // the subassembly is used
    for (p, doc, fit, rule) in &every_fit.2 {
        let Some((fits, rules)) = sub_fit_names.get(doc) else {
            continue;
        };
        let at = names.inst.get(&p[0].0).map_or(Span::default(), |x| x.1);
        let under = |n: &str| format!("{}.{n}", path_name(&names, p));
        match (fit, rule) {
            (Some(k), _) => names
                .fits
                .push((under(fits.get(*k).map_or("?", |x| x.as_str())), at)),
            (_, Some(id)) => {
                let n = under(rules.get(id).map_or("?", |x| x.as_str()));
                names.rule.entry(*id).or_insert((n, at));
            }
            _ => {}
        }
    }
    let t3 = Instant::now();
    // the overlap check can wait (`Options::overlap`): a UI shows the posed parts first
    let rep = if opts.overlap {
        checker.interference(sol, &every_fit.0, &every_fit.1)
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
        step,
        part_docs,
        ..AsmOut::default()
    };
    structure(&d, "", &dir, &fence, &subs, 0, &mut out.insts, &mut out.links);
    out.sources = d.insts.iter().map(|i| (i.id.clone(), i.source)).collect();
    out.evals = d
        .insts
        .iter()
        .filter_map(|i| Some((i.id.clone(), i.eval.clone()?)))
        .collect();
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
    out.grounded = d
        .insts
        .iter()
        .filter(|i| i.ground)
        .map(|i| i.id.clone())
        .collect();
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
        out.occ_body.push((o.part.body, o.reference));
        out.occ_tooling
            .push(tooling.get(&o.part.doc.0.to_string()).cloned());
        out.occ_anchors
            .push(anchors_of.get(&o.part.doc.0.to_string()).cloned());
        if !o.reference {
            *bom.entry(bom_key(&o.part.doc.0)).or_default() += 1;
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
                    out.part_idents
                        .push(checker.exact().idents.get(&key).copied().unwrap_or(0));
                    out.part_threads
                        .push(threads_of.get(&key).cloned().unwrap_or_default());
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
    let placed: BTreeMap<String, (Frame, &fa::PartRef)> = sol
        .occurrences
        .iter()
        .map(|o| (occ_name(&o.path), (o.pose, &o.part)))
        .collect();
    let world = |end: &(String, String)| -> Option<Frame> {
        let (pose, part) = placed.get(&end.0)?;
        if end.1.is_empty() {
            return Some(*pose);
        }
        let r = fa::Parts::part(&**checker.parts(), part)?
            .refs
            .get(&end.1)?
            .as_ref()
            .ok()?;
        Some(pose.compose(&r.frame))
    };
    out.link_at = out
        .links
        .iter()
        .map(|l| match l.family {
            LinkFamily::Joint | LinkFamily::Constraint => [world(&l.a), world(&l.b)],
            _ => [None, None],
        })
        .collect();
    if !d.harnesses.is_empty() {
        let parts = checker.parts().clone();
        let place = |r: &RefDef| -> Option<(Frame, Option<&fa::PartInfo>, fa::AnchorRef)> {
            let (_, end) = names
                .probe
                .iter()
                .find(|(p, _)| p.inst == r.inst && p.segs == r.segs && p.local == r.local)?;
            match sol.occurrences.iter().find(|o| o.path == end.path) {
                Some(o) => Some((o.pose, fa::Parts::part(&*parts, &o.part), end.anchor.clone())),
                None if end.path.len() == 1 => Some((sol.pose(end.path[0])?, None, end.anchor.clone())),
                None => None,
            }
        };
        let at = |r: &RefDef| -> Option<Frame> {
            let (pose, info, anchor) = place(r)?;
            match anchor {
                fa::AnchorRef::Local(l) => Some(pose.compose(&l.frame)),
                fa::AnchorRef::Named(n) => match info?.refs.get(&n) {
                    Some(Ok(rf)) => Some(pose.compose(&rf.frame)),
                    _ => None,
                },
            }
        };
        // the names the part does give, of those this run asked of it
        let refs_of = |r: &RefDef| -> Vec<String> {
            place(r)
                .and_then(|(_, info, _)| info)
                .map(|i| {
                    i.refs
                        .iter()
                        .filter(|x| x.1.is_ok())
                        .map(|x| x.0.clone())
                        .collect()
                })
                .unwrap_or_default()
        };
        // the parts the wires may run into, where they are
        let placed: Vec<crate::harness::Placed> = sol
            .occurrences
            .iter()
            .filter(|o| !o.reference)
            .filter_map(|o| {
                let body = checker.exact().bodies.get(&(o.part.doc.clone(), o.part.body))?;
                Some(crate::harness::Placed {
                    name: path_name(&names, &o.path),
                    body: Arc::new(body.clone()),
                    pose: o.pose,
                })
            })
            .collect();
        out.harnesses = crate::harness::finish(ev, &d.harnesses, &at, &refs_of, &placed, opts.render);
    }
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
    // the clearances asked for, worked out on all threads before they are read one by one
    let mut pairs: Vec<(fa::Path, fa::Path)> = Vec::new();
    for (k, s) in expects.iter().enumerate() {
        let StmtKind::Expect { metric, args, at, .. } = &s.kind else {
            continue;
        };
        let wanted = match &opts.probe {
            Some(ps) => ps.contains(&k),
            None => at.is_empty(),
        };
        if !wanted || metric != "clearance" {
            continue;
        }
        let m = Measure {
            names: &names,
            sol,
            catalog: &catalog,
            checker,
            fits: &every_fit,
        };
        let mark = ev.enter_expect(k);
        let (errors, warnings) = (ev.errors.len(), ev.warnings.len());
        if let (Ok((pa, _)), Ok((pb, _))) = (m.place(ev, args, 0), m.place(ev, args, 1)) {
            for x in m.parts_under(&pa) {
                for y in m.parts_under(&pb) {
                    pairs.push((x.path.clone(), y.path.clone()));
                }
            }
        }
        ev.errors.truncate(errors);
        ev.warnings.truncate(warnings);
        ev.leave_expect(mark);
    }
    checker.prefetch_clearances(sol, &pairs, CLEARANCE_REACH);
    // a document with a model of its own measures its part metrics there (`lib.rs`)
    let own_model = ev.b.body().is_some();
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
        // a part measure (volume, valid, ..) is the document's model's (`lib.rs`)
        if own_model && !METRICS.contains(&metric.as_str()) {
            continue;
        }
        if !wanted {
            continue;
        }
        let mut m = Measure {
            names: &names,
            sol,
            catalog: &catalog,
            checker,
            fits: &every_fit,
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
    // expects `at` the same params share one run: the document with those values, the parts
    // from this run's cache, every expect of the set measured at once
    let mut at_values: BTreeMap<usize, Option<f64>> = BTreeMap::new();
    let mut at_why: BTreeMap<usize, Unevaluable> = BTreeMap::new();
    if opts.probe.is_none() {
        let mut sets: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut texts = AtTexts::default();
        for (k, s) in expects.iter().enumerate() {
            let StmtKind::Expect { at, .. } = &s.kind else {
                continue;
            };
            if at.is_empty() || (own_model && !is_metric(s)) {
                continue;
            }
            let mark = ev.enter_expect(k);
            let text = at_text(ev, at, &mut texts);
            ev.leave_expect(mark);
            match text {
                Ok((text, set)) => {
                    // a joint's drive set past its limit: the check would measure elsewhere
                    for (key, v) in &set {
                        for l in &d.links {
                            let LinkKind::Joint(kind) = l.kind else { continue };
                            for (f, n) in &l.drive {
                                let driven = matches!(&n.src, Src::Driving(p) if p == key);
                                let Some(&(_, lo, hi)) = l.limits.iter().find(|x| x.0 == *f) else {
                                    continue;
                                };
                                // past the range by more than the value's own rounding
                                let slack = 1e-9 * lo.abs().max(hi.abs()).max(1.0);
                                if driven && !(lo - slack..=hi + slack).contains(v) {
                                    ev.warnings.push(Warning::Asm(Box::new(AsmNote {
                                        span: s.span,
                                        issue: Issue::AtPastLimit {
                                            limit: Box::new(Issue::LimitViolated {
                                                joint: l.id.clone(),
                                                freedom: freedom_word(kind, *f),
                                                value: *v,
                                                range: (lo, hi),
                                                angle: f.is_angle(),
                                                fix: None,
                                            }),
                                        },
                                    })));
                                }
                            }
                        }
                    }
                    sets.entry(text).or_default().push(k)
                }
                Err(why) => {
                    at_why.insert(k, why);
                }
            }
        }
        // each set's document from a fresh solve, on all cores: the parts come from this run
        let sets: Vec<(String, Vec<usize>)> = sets.into_iter().collect();
        let (parts, subs) = (cache.parts.clone(), cache.subs.clone());
        let pool: Arc<PartPool> = Arc::default();
        let runs = crate::par::map(sets.len(), |j| {
            let (text, ks) = &sets[j];
            // a probe measures its own interferences when it asks for them
            let o = crate::Options {
                probe: Some(ks.clone()),
                overlap: false,
                ..opts.clone()
            };
            let mut kept = crate::build::Cache::default();
            kept.asm = Some(Box::new(AsmCache {
                infile: Default::default(),
                parts: parts.clone(),
                subs: subs.clone(),
                last: None,
                pool: Some(pool.clone()),
                wants: None,
            }));
            let (run, _) = crate::run(text, o, kept);
            let built = run.asm.as_ref().map_or(0, |a| a.timings.built);
            // a joint past its limit at these params: every check of the set measured at the limit
            let past: Vec<Issue> = run
                .errors
                .iter()
                .filter_map(|e| match e {
                    Error::Asm(n) if matches!(n.issue, Issue::LimitViolated { .. }) => Some(n.issue.clone()),
                    _ => None,
                })
                .collect();
            (past, run.asm.map(|a| a.probes).unwrap_or_default(), built)
        });
        for ((_, ks), (past, probed, built)) in sets.into_iter().zip(runs) {
            t.at_runs += 1;
            t.built += built;
            for k in &ks {
                for limit in &past {
                    let w = Warning::Asm(Box::new(AsmNote {
                        span: expects[*k].span,
                        issue: Issue::AtPastLimit {
                            limit: Box::new(limit.clone()),
                        },
                    }));
                    let same = |x: &Warning| match (x, &w) {
                        (Warning::Asm(a), Warning::Asm(b)) => {
                            a.span == b.span
                                && a.kind() == b.kind()
                                && issue_text(&a.issue) == issue_text(&b.issue)
                        }
                        _ => false,
                    };
                    if !ev.warnings.iter().any(same) {
                        ev.warnings.push(w);
                    }
                }
            }
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
            if opts.probe.is_some() || (own_model && !is_metric(s)) {
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
                    .ok_or_else(|| unevaluable(at_why.get(&k).cloned().unwrap_or(Unevaluable::AtFailed)))
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
    cache.last = Some(live);
    if opts.probe.is_none() {
        crate::geomemo::keep();
    }
    (Some(out), Some(cache))
}

/// The document with an `expect`'s `at P = v` params set: what it is measured on. None when a
/// value is not a number here, or a param is not the document's.
fn at_text(
    ev: &mut Eval,
    at: &[(Vec<Seg>, Expr)],
    done: &mut AtTexts,
) -> Result<(String, Vec<(ParamKey, f64)>), Unevaluable> {
    let mut edits: Vec<(ParamKey, Literal, f64)> = vec![];
    // the first value that is not one stops the edits there, as an edit failing first would
    let mut bad: Option<Unevaluable> = None;
    for (path, e) in at {
        let Some((last, owner)) = path.split_last() else {
            bad = Some(Unevaluable::AtFailed);
            break;
        };
        let param = ParamKey {
            owner: owner.to_vec(),
            name: last.name.clone(),
        };
        let n = match ev.expr(e) {
            Ok(Value::Num(n)) => n,
            _ => {
                bad = Some(Unevaluable::AtNotANumber {
                    param: param.to_string(),
                });
                break;
            }
        };
        let value = Literal {
            value: if n.dim == Dim::Angle {
                n.v.to_degrees()
            } else {
                n.v
            },
            dim: if n.dim == Dim::Any { Dim::Length } else { n.dim },
        };
        edits.push((param, value, n.v));
    }
    // expects at the same values share one text
    let key: Vec<(ParamKey, u64, Dim)> = edits
        .iter()
        .map(|(p, l, _)| (p.clone(), l.value.to_bits(), l.dim))
        .collect();
    let set: Vec<(ParamKey, f64)> = edits.iter().map(|(p, _, v)| (p.clone(), *v)).collect();
    if bad.is_none()
        && let Some(r) = done.texts.get(&key)
    {
        return r.clone().map(|t| (t, set));
    }
    if done.dims.is_none() {
        done.dims = Some(
            ev.shared
                .iter()
                .chain(&ev.locals)
                .map(|p| {
                    let d = match &p.value {
                        Value::Num(n) => Some(n.dim),
                        _ => None,
                    };
                    (p.key.clone(), d)
                })
                .collect(),
        );
    }
    let mut text = ev.src.to_string();
    // the document's dimensions hold while each value set keeps its param's dimension; after a
    // param of no fixed dimension is set they are worked out again from the text
    let mut fresh = false;
    let r = (|| {
        for (param, value, _) in &edits {
            let base = done.dims.as_ref().expect("made above");
            let known = |k: &ParamKey| -> Option<Dim> {
                if fresh {
                    return crate::edit::param_dim(&text, k);
                }
                base.get(k).copied().flatten()
            };
            let was = known(param);
            text = match crate::edit::apply_with(
                &text,
                &Op::SetParam {
                    param: param.clone(),
                    value: *value,
                },
                &known,
            ) {
                Ok(t) => t,
                Err(Error::UnknownParam { .. }) => {
                    return Err(Unevaluable::AtUnknownParam {
                        param: param.to_string(),
                    });
                }
                Err(_) => return Err(Unevaluable::AtFailed),
            };
            if was.is_none_or(|d| d == Dim::Any) {
                fresh = true;
            }
        }
        Ok(text.clone())
    })();
    if let Some(why) = bad {
        r?;
        return Err(why);
    }
    done.texts.insert(key, r.clone());
    r.map(|t| (t, set))
}

/// `at_text`'s work so far in one assembly's run: the document's param dimensions, and the text
/// made for each set of values.
#[derive(Default)]
struct AtTexts {
    dims: Option<HashMap<ParamKey, Option<Dim>>>,
    #[allow(clippy::type_complexity)]
    texts: HashMap<Vec<(ParamKey, u64, Dim)>, Result<String, Unevaluable>>,
}

/// The fits and fit rules of an assembly and of every subassembly used in it, each under its
/// use: a subassembly's fits name its own occurrences, and its rules cover pairs inside it only.
/// The third list says, for each fit and rule added, the use it is under, its document and its
/// place there (a fit's index, a rule's id), in the order they were added.
type FitSource = (fa::Path, fa::DocId, Option<usize>, Option<u64>);

fn all_fits(asm: &fa::Assembly, catalog: &fa::Catalog) -> (Vec<fa::Fit>, Vec<fa::FitRule>, Vec<FitSource>) {
    let mut fits = asm.fits.clone();
    let mut rules = asm.fit_rules.clone();
    let mut from = vec![];
    let mut todo: Vec<(fa::Path, &fa::Assembly, usize)> = vec![(vec![], asm, 0)];
    while let Some((at, a, depth)) = todo.pop() {
        // a cycle cannot reach here (refused on loading), but stay finite anyway
        if depth > 64 {
            continue;
        }
        for i in a.instances.iter().filter(|i| !i.suppressed) {
            let fa::Component::Assembly(d) = &i.of else {
                continue;
            };
            let Some(sub) = catalog.assemblies.get(d) else {
                continue;
            };
            let mut p = at.clone();
            p.push(i.id);
            let under = |q: &fa::Path| {
                let mut x = p.clone();
                x.extend(q.iter().copied());
                x
            };
            for (k, f) in sub.fits.iter().enumerate() {
                fits.push(fa::Fit {
                    a: under(&f.a),
                    b: under(&f.b),
                });
                from.push((p.clone(), d.clone(), Some(k), None));
            }
            for r in &sub.fit_rules {
                rules.push(fa::FitRule {
                    within: under(&r.within),
                    ..r.clone()
                });
                from.push((p.clone(), d.clone(), None, Some(r.id)));
            }
            todo.push((p, sub, depth + 1));
        }
    }
    (fits, rules, from)
}

/// What an expect measures with.
struct Measure<'a> {
    names: &'a Names,
    sol: &'a fa::Solution,
    catalog: &'a fa::Catalog,
    checker: &'a mut Checker<Arc<fa::Catalog>, Arc<BrepExact>>,
    /// The assembly's fits and fit rules with its subassemblies' (`all_fits`).
    fits: &'a (Vec<fa::Fit>, Vec<fa::FitRule>, Vec<FitSource>),
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
        // a body of this document a joint made a component: its occurrence
        if let Value::Feature(f) = &v
            && self.names.inst.contains_key(&hid(&f.id))
        {
            let path = vec![fa::InstanceId(hid(&f.id))];
            let pose = self.sol.pose(path[0]).ok_or(Unevaluable::NoReference {
                reference: f.id.clone(),
            })?;
            return Ok((path, pose));
        }
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
                let rep = self.checker.interference(self.sol, &self.fits.0, &self.fits.1);
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
                                .clearance(self.sol, &x.path, &y.path, CLEARANCE_REACH)
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
            // two references' planes (z their normals) or axes (z along them), in the world
            "gap" | "axis_offset" | "axis_angle" => {
                let (_, fa_) = self.place(ev, args, 0).map_err(why)?;
                let (_, fb) = self.place(ev, args, 1).map_err(why)?;
                let (za, zb) = (fa_.z().get(), fb.z().get());
                let angle =
                    forge_math::dmath::acos(za.dot(zb).abs().clamp(0.0, 1.0)) * 180.0 / forge_math::dmath::PI;
                let w = fb.origin - fa_.origin;
                match metric {
                    "axis_angle" => Ok(angle),
                    "axis_offset" => Ok((w - za * w.dot(za)).length()),
                    _ if za.cross(zb).length() > 1e-9 => Err(why(Unevaluable::NotParallel { angle })),
                    _ => Ok(w.dot(za).abs()),
                }
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
            "components",
            Json::Arr(
                out.insts
                    .iter()
                    .filter_map(|i| Some((i, out.sources.iter().find(|s| s.0 == i.path)?.1)))
                    .map(|(i, src)| {
                        Json::obj(vec![
                            ("id", i.path.as_str().into()),
                            ("source", src.name().into()),
                            ("file", i.file.as_str().into()),
                            ("subassembly", i.sub.into()),
                            ("ground", i.ground.into()),
                            ("suppressed", i.suppressed.into()),
                        ])
                    })
                    .collect(),
            ),
        ),
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
                ("built", t.built.into()),
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
    if !out.harnesses.is_empty() {
        f.push((
            "harnesses",
            Json::Arr(out.harnesses.iter().map(crate::harness::json).collect()),
        ));
    }
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

// ---- components of this document ----------------------------------------------------------------

/// What goes wrong with a component of this document.
#[derive(Clone, Debug, PartialEq)]
pub enum CompProblem {
    /// `component` inside a loop, function, sketch or another component.
    NotTopLevel,
    /// `shared` inside a component: the document's params are written at its top.
    Shared,
    /// A component, instance or joint inside a component (a subassembly is a file).
    Nested { func: &'static str },
    /// A joint end of this document no body or component holds.
    NoComponent,
    /// A body a joint names is gone by the end of the document (a later boolean took it).
    BodyGone { id: String },
    /// Designed, not built yet (the design's first merge leaves it out): said, typed.
    NotYet { what: &'static str },
    /// A refactor would leave a name unbound: what the moved statements read that stays behind.
    ReadsOutside { name: String, reader: String },
    /// A refactor would take away what statements left behind read: each reader.
    StillReadBy { name: String, readers: Vec<String> },
    /// A component's name read otherwise than placing it (`part(cup(..))`): not movable to a file.
    ReadOtherwise { component: String, reader: String },
    /// The file to write already exists.
    FileExists { file: String },
    /// The file to write is a link (dangling or not): never written through.
    TargetIsLink { file: String },
    /// A file a refactor makes is a plain `.fg` name beside the document, and this is not.
    BadFileName { file: String },
    /// The document has no file yet: nowhere beside it to write.
    UntitledDocument,
    /// The file is outside the project (or the session's root): never read.
    Outside { file: String },
    /// A file not inlinable as written: why.
    NotInlinable { file: String, why: String },
    /// Components placing each other round (a placing b placing a, or a itself): no end (R1).
    Cycle { a: String, b: String },
}

/// A component problem as an error, where it is.
pub fn comp_error(span: Span, p: CompProblem) -> Error {
    note(span, Issue::Component(p))
}

/// A construct the language will have but does not yet, refused typed.
pub fn not_yet(what: &'static str, span: Span) -> Error {
    note(span, Issue::Component(CompProblem::NotYet { what }))
}

impl CompProblem {
    fn kind(&self) -> &'static str {
        match self {
            CompProblem::NotTopLevel => "ComponentNotTopLevel",
            CompProblem::Shared => "SharedInComponent",
            CompProblem::Nested { .. } => "InComponent",
            CompProblem::NoComponent => "NoComponent",
            CompProblem::BodyGone { .. } => "BodyGone",
            CompProblem::NotYet { .. } => "NotYet",
            CompProblem::ReadsOutside { .. } => "ReadsOutside",
            CompProblem::StillReadBy { .. } => "StillReadBy",
            CompProblem::ReadOtherwise { .. } => "ReadOtherwise",
            CompProblem::FileExists { .. } => "FileExists",
            CompProblem::TargetIsLink { .. } => "TargetIsLink",
            CompProblem::BadFileName { .. } => "BadFileName",
            CompProblem::UntitledDocument => "UntitledDocument",
            CompProblem::Outside { .. } => "Outside",
            CompProblem::NotInlinable { .. } => "NotInlinable",
            CompProblem::Cycle { .. } => "ComponentCycle",
        }
    }
    fn text(&self) -> String {
        match self {
            CompProblem::NotTopLevel => "a component is a statement of the document's top level".into(),
            CompProblem::Shared => "shared params are the document's: write it above the component".into(),
            CompProblem::Nested { func } => {
                format!("{func} places or joins components: write it in the document, outside the component")
            }
            CompProblem::NoComponent => {
                "this end lies on no body or component: start it with its component (`lid.NAME`)".into()
            }
            CompProblem::BodyGone { id } => format!("{id} is no body at the end of the document"),
            CompProblem::ReadsOutside { name, reader } => {
                format!("{reader} reads {name}, which would stay behind: move it too, or make it shared")
            }
            CompProblem::StillReadBy { name, readers } => {
                format!("{name} is still read by {}", readers.join(", "))
            }
            CompProblem::ReadOtherwise { component, reader } => {
                format!("{reader} reads {component} other than placing it (part({component}(..)))")
            }
            CompProblem::FileExists { file } => format!("{file} exists already: pick another name"),
            CompProblem::TargetIsLink { file } => format!("{file} is a link: pick another name"),
            CompProblem::BadFileName { file } => {
                format!("{file}: a new file is a plain name ending .fg, beside this one (no folder)")
            }
            CompProblem::UntitledDocument => "save this file first: what moves out goes beside it".into(),
            CompProblem::Outside { file } => format!("{file} is outside the session's root, not read"),
            CompProblem::NotInlinable { file, why } => format!("{file} can't be brought in: {why}"),
            CompProblem::Cycle { a, b } => {
                format!("{a} places {b}, which places {a}: a component may not place itself")
            }
            CompProblem::NotYet { what } => {
                format!(
                    "{what} is designed but not built yet: place a file (`part(FILE)`) or a component of this file"
                )
            }
        }
    }
}

/// An inline component's own evaluation: a part of its own over this document's text.
pub struct Comp<'s> {
    /// The part it makes (`cup(d=80,h=95)`): equal values, the same part.
    pub id: String,
    /// The part and where each value came from: evaluations are shared only when both agree,
    /// so a param driving one call never stands for another's (proof-findings 3).
    pub eval_key: String,
    /// The definition's name.
    pub def: String,
    pub ev: Eval<'s>,
    /// The `component` statement.
    pub span: Span,
    /// It places or joins components of its own: a subassembly.
    pub sub: bool,
}

/// How deep components may place components (a cycle the static check misses stops here).
pub const DEPTH: usize = 16;

/// A part's identity, what its BOM row is by: a call of a component of this file is its
/// definition and values (`#lid(w=40)`), whatever calls it is under; a body of a call is that
/// call's (`#box(w=40)/base`); a file is itself.
pub fn bom_key(doc: &str) -> String {
    // a subassembly file's own component: that file's, by its identity there
    if let Some((file, key)) = nested_key(doc) {
        return format!("{file}|{}", bom_key(key));
    }
    let Some(rest) = doc.strip_prefix('#') else {
        return doc.to_string();
    };
    let segs: Vec<&str> = rest.split('/').collect();
    let last = segs.last().copied().unwrap_or(rest);
    if last.contains('(') {
        return format!("#{last}");
    }
    match segs.iter().rev().skip(1).find(|s| s.contains('(')) {
        Some(call) => format!("#{call}/{last}"),
        None => doc.to_string(),
    }
}

/// The key of the bodies no joint names, in an evaluation under `path`.
pub fn root_key(path: &str) -> String {
    if path.is_empty() {
        ROOT.to_string()
    } else {
        format!("#{path}#root")
    }
}

/// An inline component as a run reports it: its features and params named from outside
/// (`lid.plate`, `lid.plate.d`), its iProperties.
#[derive(Clone, Debug)]
pub struct CompOut {
    /// The part it makes (`peg(d=6,h=30)`).
    pub id: String,
    /// And where its values came from: one evaluation per part and inputs.
    pub eval_key: String,
    pub def: String,
    pub span: Span,
    pub feats: Vec<crate::eval::FeatInfo>,
    pub locals: Vec<crate::eval::ParamInfo>,
    pub props: crate::props::Props,
}

fn prefixed(id: &str, inner: &str) -> String {
    format!("{id}.{inner}")
}

/// The inline components of a run, as `Run::components`.
pub fn components(ev: &Eval) -> Vec<CompOut> {
    let mut out = own_components(&ev.comps);
    for c in &ev.comps {
        nested_components(&c.ev, &mut out);
    }
    out
}

/// The calls inside a call's evaluation, at any depth (each evaluation once): a definition only
/// called inside another's lists its features too.
fn nested_components(ev: &Eval, out: &mut Vec<CompOut>) {
    for c in &ev.comps {
        if !out.iter().any(|o| o.eval_key == c.eval_key) {
            out.extend(own_components(std::slice::from_ref(c)));
        }
        nested_components(&c.ev, out);
    }
}

fn own_components(comps: &[Comp<'_>]) -> Vec<CompOut> {
    comps
        .iter()
        .map(|c| CompOut {
            id: c.id.clone(),
            eval_key: c.eval_key.clone(),
            def: c.def.clone(),
            span: c.span,
            feats: c
                .ev
                .feats
                .iter()
                // definition ids: what edit ops address (`peg.shank`), the same for every call
                .map(|f| crate::eval::FeatInfo {
                    id: prefixed(&c.def, &f.id),
                    ..f.clone()
                })
                .collect(),
            locals: c
                .ev
                .locals
                .iter()
                .map(|p| {
                    let mut p = p.clone();
                    p.key.owner.insert(
                        0,
                        Seg {
                            name: c.def.clone(),
                            index: vec![],
                        },
                    );
                    p
                })
                .collect(),
            props: c.ev.props.clone(),
        })
        .collect()
}

/// Puts each inline component's kernel history back in the document's cache, for the next run.
pub fn keep_comp_caches(ev: &mut Eval) {
    for c in &mut ev.comps {
        let b = std::mem::replace(&mut c.ev.b, crate::build::Builder::new(false, Default::default()));
        ev.b.cache.comps.insert(c.eval_key.clone(), b.into_cache());
    }
}

/// A `component NAME(p: DEFAULT, ..) { STATEMENTS }` definition: a part as a function of its
/// params. It places nothing; `part(NAME(p: v))` does.
#[derive(Clone, Debug)]
pub struct CompDef {
    pub name: String,
    /// Where it is written: its name under the definitions it is inside (`box.lid`), what a
    /// part's identity and its features' ids are by, wherever it is called from.
    pub qual: String,
    pub params: Vec<Arg>,
    pub body: Rc<Vec<crate::syntax::Stmt>>,
    pub span: Span,
}

/// `component NAME(p: DEFAULT, ..) { .. }`: defined here, its statements checked once with the
/// defaults (built only where it is placed).
pub fn component_block(
    ev: &mut Eval,
    s: &crate::syntax::Stmt,
    name: &str,
    args: &[Arg],
    body: &[crate::syntax::Stmt],
) -> R<()> {
    if !ev.at_top() || ev.asm.is_none() {
        return Err(note(s.span, Issue::Component(CompProblem::NotTopLevel)));
    }
    for a in args {
        if a.name.is_none() {
            return Err(Error::MissingArg {
                span: a.value.span,
                func: "component",
                arg: "a param's name and default (`d: 80`)",
            });
        }
    }
    // a param named as a shared param would stand for it inside the functions the body calls
    // (proof-findings 5): one name, one meaning
    for a in args {
        let (n, sp) = a.name.as_ref().expect("checked: named");
        if let Some(p) = ev.shared.iter().find(|p| p.key.name == *n) {
            return Err(Error::Rebind {
                span: *sp,
                name: n.clone(),
                first: p.span,
            });
        }
    }
    for st in body {
        if matches!(st.kind, StmtKind::Shared { .. }) {
            ev.errors
                .push(note(st.span, Issue::Component(CompProblem::Shared)));
        }
    }
    // the definitions this one is written inside, from the calls this evaluation is under
    let outer: Vec<&str> = ev
        .comp_path
        .split('/')
        .filter(|x| !x.is_empty())
        .map(|x| x.split(['(', '?']).next().unwrap_or(x))
        .collect();
    let qual = match outer.last() {
        Some(o) => format!("{o}.{name}"),
        None => name.to_string(),
    };
    let def = Rc::new(CompDef {
        qual,
        name: name.to_string(),
        params: args.to_vec(),
        body: Rc::new(
            body.iter()
                .filter(|st| !matches!(st.kind, StmtKind::Shared { .. }))
                .cloned()
                .collect(),
        ),
        span: s.span,
    });
    ev.ctx_pending(name, s.span);
    let id = ev.take_id("component", s.span)?;
    ev.record(&id, "component", s.span, vec![]);
    // its statements' errors even where it is never placed: a check with the defaults
    let values = call_values(ev, &def, &[], s.span)?;
    // each default a param of the document's (`cup.d`): listed with its value, its handle (a
    // literal is SetParam'd as SetArg on the definition) and whether it is structural
    for ((n, v), a) in values.iter().zip(&def.params) {
        ev.locals.push(crate::eval::ParamInfo {
            key: ParamKey {
                owner: vec![Seg {
                    name: def.name.clone(),
                    index: vec![],
                }],
                name: n.clone(),
            },
            span: a.name.as_ref().map_or(a.value.span, |x| x.1),
            rhs: a.value.span,
            value: match v {
                Value::Num(x) => Value::Num(Num {
                    dim: if x.dim == Dim::Any { Dim::Length } else { x.dim },
                    ..x.clone()
                }),
                o => o.clone(),
            },
            links: {
                let mut ls = BTreeSet::new();
                if let Value::Num(x) = v
                    && !matches!(x.src, Src::Driving(ref p) if p.owner.len() == 1 && p.owner[0].name == def.name)
                {
                    ls.extend(x.src.params());
                }
                ls
            },
            used: true,
        });
    }
    let mut probe = sub_eval(ev, &def, &values, false, crate::build::Cache::default(), true);
    absorb(ev, &mut probe, &def, None);
    ev.bind_name(name, asm(AsmVal::Def(def)), s.span)
}

/// Component `name` of the document evaluated with its defaults, as its definition checks it.
pub fn default_eval<'s>(ev: &Eval<'s>, name: &str) -> Option<Eval<'s>> {
    let def = ev.top_level().into_iter().find_map(|(n, v)| match v {
        Value::Asm(a) if n == name => match &*a {
            AsmVal::Def(d) => Some(d.clone()),
            _ => None,
        },
        _ => None,
    })?;
    let mut host = ev.child(ev.b.building, Default::default());
    let values = call_values(&mut host, &def, &[], def.span).ok()?;
    Some(sub_eval(
        ev,
        &def,
        &values,
        ev.b.building,
        crate::build::Cache::default(),
        true,
    ))
}

/// A definition's params for a call: each argument given (evaluated here), else its default.
fn call_values(ev: &mut Eval, def: &CompDef, args: &[Arg], span: Span) -> R<Vec<(String, Value)>> {
    Ok(call_inputs(ev, def, args, span)?.0)
}

/// The params' values and traces: an argument as written here; a default as the definition
/// writes it, evaluated with only the document's shared params and functions (proof E10: a
/// default never sees a caller's names). A literal default is its own handle: the param
/// `NAME.p`, set by SetArg on the definition (proof E6).
fn call_inputs(ev: &mut Eval, def: &CompDef, args: &[Arg], span: Span) -> R<Inputs> {
    for a in args {
        let Some((n, sp)) = &a.name else {
            return Err(Error::MissingArg {
                span: a.value.span,
                func: "component",
                arg: "a param by name (`d: 80`)",
            });
        };
        if !def
            .params
            .iter()
            .any(|p| p.name.as_ref().is_some_and(|x| &x.0 == n))
        {
            return Err(Error::UnknownArg {
                span: *sp,
                func: "component",
                arg: n.clone(),
            });
        }
    }
    let _ = span;
    let text = |e: &Expr| one_line(ev.src.get(e.span.lo as usize..e.span.hi as usize).unwrap_or(""));
    let (mut out, mut traces) = (vec![], vec![]);
    for p in &def.params {
        let n = &p.name.as_ref().expect("checked: named").0;
        match args.iter().find(|a| a.name.as_ref().is_some_and(|x| &x.0 == n)) {
            Some(a) => {
                traces.push(text(&a.value));
                let v = ev.expr(&a.value)?;
                // another component's geometry, made in context: its frame where that occurrence
                // is written (never where the solve puts it), an input like a number
                let v = match &v {
                    Value::Asm(x) if matches!(&**x, AsmVal::Ref(_) | AsmVal::Inst(_)) => {
                        context_value(ev, x, a.value.span)?
                    }
                    _ => v,
                };
                out.push((n.clone(), v));
            }
            None => {
                traces.push(format!("default {}", text(&p.value)));
                let mut d = ev.child(false, Default::default());
                let v = d.expr(&p.value)?;
                let literal = matches!(p.value.kind, ExprKind::Num(..))
                    || matches!(&p.value.kind, ExprKind::Neg(x) if matches!(x.kind, ExprKind::Num(..)));
                let v = match v {
                    Value::Num(x) if literal => Value::Num(Num {
                        src: Src::Driving(ParamKey {
                            owner: vec![Seg {
                                name: def.name.clone(),
                                index: vec![],
                            }],
                            name: n.clone(),
                        }),
                        ..x
                    }),
                    v => v,
                };
                out.push((n.clone(), v));
            }
        }
    }
    Ok((out, traces))
}

fn r9(x: f64) -> String {
    crate::values::fmt((x * 1e9).round() / 1e9)
}

fn frame_text(f: &Frame) -> String {
    let (o, x, z) = (f.origin, f.x().get(), f.z().get());
    format!(
        "frame({},{},{};{},{},{};{},{},{})",
        r9(o.x),
        r9(o.y),
        r9(o.z),
        r9(x.x),
        r9(x.y),
        r9(x.z),
        r9(z.x),
        r9(z.y),
        r9(z.z)
    )
}

/// A reference to another occurrence's geometry as a value here: an inline component's
/// reference evaluated in its call, at its written pose; a file's not yet (typed).
pub(crate) fn context_value(ev: &mut Eval, x: &Rc<AsmVal>, span: Span) -> R<Value> {
    let (inst, segs) = match &**x {
        AsmVal::Inst(i) => (i.clone(), vec![]),
        AsmVal::Ref(r) => (r.inst.clone(), r.segs.clone()),
        _ => unreachable!("a reference"),
    };
    let d = ev.asm.as_ref().ok_or_else(|| not_yet("a reference here", span))?;
    let Some(i) = d.insts.iter().find(|i| i.id == inst).cloned() else {
        return Err(not_yet("a reference to what is no occurrence here", span));
    };
    let Some(eval) = i.eval.clone() else {
        return Err(not_yet(
            "a reference into a part file's geometry, made in context",
            span,
        ));
    };
    let pose = i.pose;
    let Some(c) = ev.comps.iter_mut().find(|c| c.eval_key == eval) else {
        return Err(not_yet("a reference to a call not made", span));
    };
    let r = if segs.is_empty() {
        fa::Reference::frame(Frame::WORLD)
    } else {
        let e = ref_expr(&segs);
        let v = c.ev.expr(&e)?;
        as_reference(&mut c.ev, v).ok_or(Error::WrongType {
            span,
            what: "a reference made in context",
            want: Ty::Reference,
            found: Ty::Unresolved,
        })?
    };
    let f = pose.compose(&r.frame);
    Ok(match r.kind {
        fa::RefKind::Point => Value::Point(f.origin),
        fa::RefKind::Axis => Value::Axis(f.origin, f.z()),
        _ => Value::Plane(Rc::new(crate::eval::Plane {
            frame: f,
            world: None,
        })),
    })
}

/// A call's key: the definition and its params' values (`cup(d=80,h=95)`), what makes two
/// calls the same part.
fn call_key(def: &CompDef, values: &[(String, Value)]) -> String {
    let show = |v: &Value| match v {
        Value::Num(n) if n.dim == Dim::Angle => format!("{}deg", crate::values::fmt(n.v.to_degrees())),
        Value::Num(n) => crate::values::fmt(n.v),
        Value::Str(s) => format!("{s:?}"),
        Value::Bool(b) => b.to_string(),
        // a place given in context: to the kernel's tolerance, so equal places are one part
        Value::Plane(p) => frame_text(&p.frame),
        Value::Faces(f) if f.plane.is_some() => frame_text(f.plane.as_ref().expect("checked")),
        Value::Axis(o, d) => {
            let d = d.get();
            format!(
                "axis({},{},{};{},{},{})",
                r9(o.x),
                r9(o.y),
                r9(o.z),
                r9(d.x),
                r9(d.y),
                r9(d.z)
            )
        }
        Value::Point(p) => format!("pt({},{},{})", r9(p.x), r9(p.y), r9(p.z)),
        o => format!("{:?}", o.ty()),
    };
    let args: Vec<String> = values.iter().map(|(n, v)| format!("{n}={}", show(v))).collect();
    format!("{}({})", def.qual, args.join(","))
}

fn sub_eval<'s>(
    ev: &Eval<'s>,
    def: &CompDef,
    values: &[(String, Value)],
    building: bool,
    cache: crate::build::Cache,
    shop: bool,
) -> Eval<'s> {
    sub_eval_at(
        ev,
        def,
        values,
        building,
        cache,
        format!("{}{}?/", ev.comp_path, def.name),
        shop,
    )
}

/// A component's evaluation under `path` (the calls it is under, its own key last): it may
/// hold all a document does, placements and joints too (a subassembly then).
fn sub_eval_at<'s>(
    ev: &Eval<'s>,
    def: &CompDef,
    values: &[(String, Value)],
    building: bool,
    cache: crate::build::Cache,
    path: String,
    shop: bool,
) -> Eval<'s> {
    let mut sub = ev.child(building, cache);
    sub.comp_path = path;
    sub.asm = Some(Doc::default());
    for (n, v) in values {
        if let Err(e) = sub.bind_name(n, v.clone(), def.span) {
            sub.errors.push(e);
        }
    }
    sub.check_static(&def.body);
    if !sub.errors.is_empty() {
        sub.b.building = false;
    }
    sub.run(&def.body);
    // a call of this document keeps its shop warnings; another file's part loaded here, none
    crate::sheet::finish_with(&mut sub, shop);
    sub
}

/// What a component's evaluation says that is the document's: params it read, errors and
/// warnings of its own statements.
fn absorb(ev: &mut Eval, sub: &mut Eval, def: &CompDef, key: Option<&str>) {
    for (k, p) in sub.shared.iter().enumerate() {
        if p.used
            && let Some(h) = ev.shared.get_mut(k)
        {
            h.used = true;
        }
    }
    let shared: BTreeSet<ParamKey> = ev.shared.iter().map(|p| p.key.clone()).collect();
    // a definition checked with its defaults places nothing: what it observed is no one's
    // (proof E7); a call's is the document's
    let observed = std::mem::take(&mut sub.observed);
    // a component's own params (`cup.d`, a default's handle) are named as they are
    let own = |k: &ParamKey| matches!(&k.owner[..], [o] if o.name == def.name);
    for k in observed.into_iter().filter(|_| key.is_some()) {
        if shared.contains(&k) || own(&k) {
            ev.observed.insert(k);
        } else if let Some(key) = key {
            let mut k = k;
            k.owner.insert(
                0,
                Seg {
                    name: key.to_string(),
                    index: vec![],
                },
            );
            ev.observed.insert(k);
        }
    }
    let span = def.span;
    let inside = |sp: Option<Span>| sp.is_none_or(|x| x.lo >= span.lo && x.hi <= span.hi);
    for e in std::mem::take(&mut sub.errors)
        .into_iter()
        .filter(|e| inside(e.span()))
    {
        if !ev
            .errors
            .iter()
            .any(|x| x.kind() == e.kind() && x.span() == e.span())
        {
            ev.errors.push(e);
        }
    }
    for w in std::mem::take(&mut sub.warnings) {
        if matches!(&w, Warning::UnusedParam { span, .. } if !inside(Some(*span))) {
            continue;
        }
        if !ev.warnings.contains(&w) {
            ev.warnings.push(w);
        }
    }
}

/// `NAME(p: v, ..)` on a definition: the call, built once per key.
pub fn call_def(ev: &mut Eval, def: &Rc<CompDef>, args: &[Arg], span: Span) -> R<Value> {
    let (values, traces) = call_inputs(ev, def, args, span)?;
    let key = call_key(def, &values);
    // shared only by calls whose every input is the same trace (written the same, read from the
    // same params the same way): a handle in one is then a handle in the other (proof E1)
    let inputs: Vec<String> = values
        .iter()
        .zip(&traces)
        .map(|((n, v), t)| {
            let src = match v {
                Value::Num(x) => match &x.src {
                    Src::Const => String::new(),
                    Src::Driving(p) => format!("={p}"),
                    Src::Driven(ps) => format!(
                        "~{}",
                        ps.iter().map(|p| p.to_string()).collect::<Vec<_>>().join("+")
                    ),
                },
                _ => String::new(),
            };
            format!("{n}<-{t}{src}")
        })
        .collect();
    let eval_key = format!("{key}|{}", inputs.join(","));
    if !ev.comps.iter().any(|c| c.eval_key == eval_key) {
        let cache = ev.b.cache.comps.remove(&eval_key).unwrap_or_default();
        // a placement cycle the static check could not see (through a function): refused, typed,
        // never the evaluator's fuel (R1)
        if ev.comp_path.split('/').filter(|x| !x.is_empty()).count() >= DEPTH {
            return Err(comp_error(
                span,
                CompProblem::Cycle {
                    a: def.name.clone(),
                    b: def.name.clone(),
                },
            ));
        }
        // one budget for the run: a wide tree of calls ends in Fuel, not in the machine's memory
        let (calls, comps) = ev.total.get();
        if comps + 1 > crate::eval::TOTAL_COMPONENTS {
            return Err(Error::Fuel {
                span,
                func: def.name.clone(),
                budget: crate::eval::TOTAL_COMPONENTS,
            });
        }
        ev.total.set((calls, comps + 1));
        let path = format!("{}{key}/", ev.comp_path);
        let mut sub = sub_eval_at(ev, def, &values, ev.b.building, cache, path, true);
        absorb(ev, &mut sub, def, Some(&def.name));
        let is_sub = sub
            .asm
            .as_ref()
            .is_some_and(|d| !d.insts.is_empty() || !d.links.is_empty());
        ev.comps.push(Comp {
            id: key.clone(),
            eval_key: eval_key.clone(),
            def: def.qual.clone(),
            ev: sub,
            span: def.span,
            sub: is_sub,
        });
    }
    Ok(asm(AsmVal::Call { key, eval: eval_key }))
}

/// The expression a joint end starts from: its first name, with that name's indices
/// (`slat[2]` of `slat[2].side(p.o[1])`).
fn root_expr(e: &Expr) -> &Expr {
    match &e.kind {
        ExprKind::Attr(o, _) => root_expr(o),
        ExprKind::Call(f, _) => match &f.kind {
            ExprKind::Attr(o, _) => root_expr(o),
            _ => e,
        },
        ExprKind::Index(o, _) if matches!(o.kind, ExprKind::Name(_)) => e,
        ExprKind::Index(o, _) => root_expr(o),
        _ => e,
    }
}

/// The end with its root taken off: `lid.hax` is `hax`, `lid.plane(at: ..)` is `plane(at: ..)`.
fn strip_root(e: &Expr, root: &Expr) -> Option<Expr> {
    let kind = match &e.kind {
        ExprKind::Attr(o, n) if std::ptr::eq(&**o, root) => ExprKind::Name(n.clone()),
        ExprKind::Attr(o, n) => ExprKind::Attr(Box::new(strip_root(o, root)?), n.clone()),
        ExprKind::Call(f, args) => ExprKind::Call(Box::new(strip_root(f, root)?), args.clone()),
        ExprKind::Index(o, i) => ExprKind::Index(Box::new(strip_root(o, root)?), i.clone()),
        _ => return None,
    };
    Some(Expr { kind, span: e.span })
}

fn one_line(t: &str) -> String {
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The body of this document a feature made, if it is one as the document stands.
fn makes_body(ev: &Eval, idx: usize) -> bool {
    ev.b.stages
        .last()
        .is_some_and(|st| st.solids.iter().any(|s| s.id == idx))
}

/// The body holding a face set's or edge set's first entity, by the feature that made it.
fn owning_body(ev: &Eval, v: &Value) -> Option<String> {
    let st = ev.b.stages.last()?;
    let idx = match v {
        Value::Faces(f) => {
            let n = *f.names.first()?;
            st.solids
                .iter()
                .find(|s| !crate::build::role_faces(&s.body, n).is_empty())?
                .id
        }
        Value::Edges(es) => {
            let n = *es.first()?;
            st.solids
                .iter()
                .find(|s| s.body.edge_ids().any(|x| s.body.edge(x).name == n))?
                .id
        }
        _ => return None,
    };
    ev.feats.iter().find(|f| f.idx == idx).map(|f| f.id.clone())
}

/// A joint end on a body of this document: the body becomes a component (where it was made),
/// the end worked out here, in the document. None for an end of an instance or component.
fn body_end(ev: &mut Eval, e: &Expr) -> R<Option<RefDef>> {
    let root = root_expr(e);
    let mark = (ev.errors.len(), ev.warnings.len());
    let restore = |ev: &mut Eval| {
        ev.errors.truncate(mark.0);
        ev.warnings.truncate(mark.1);
    };
    let rv = ev.expr(root);
    restore(ev);
    if matches!(rv, Ok(Value::Asm(_))) {
        return Ok(None);
    }
    let feat = match &rv {
        Ok(Value::Feature(f)) => Some(f.clone()),
        _ => None,
    };
    let root_body = feat
        .as_ref()
        .filter(|f| makes_body(ev, f.idx))
        .map(|f| f.id.clone());
    let tail = one_line(
        ev.src[root.span.hi as usize..e.span.hi as usize]
            .trim_start_matches(|c: char| c == '.' || c.is_whitespace()),
    );
    let value = match ev.expr(e) {
        Ok(v) => v,
        Err(err) => {
            restore(ev);
            let stripped = (root_body.is_some() || (!ev.b.building && feat.is_some()))
                .then(|| strip_root(e, root))
                .flatten();
            match stripped.map(|x| ev.expr(&x)) {
                Some(Ok(v)) => v,
                _ => return Err(err),
            }
        }
    };
    // nothing built to say whose it is (`check`, or after a failure): still an end
    if !ev.b.building && (feat.is_some() || matches!(value, Value::Unresolved)) {
        return Ok(Some(RefDef {
            inst: feat.map_or_else(String::new, |f| f.id.clone()),
            segs: vec![],
            span: e.span,
            local: None,
        }));
    }
    let (owner, text) = match root_body {
        Some(b) => (b, tail),
        None => match owning_body(ev, &value) {
            Some(b) => (b, one_line(&ev.src[e.span.lo as usize..e.span.hi as usize])),
            None => {
                let geometric = matches!(
                    value,
                    Value::Plane(_)
                        | Value::Axis(..)
                        | Value::Point(_)
                        | Value::Wired(_)
                        | Value::Faces(_)
                        | Value::Edges(_)
                        | Value::Line(..)
                        | Value::Ent(_)
                );
                if geometric {
                    return Err(note(e.span, Issue::Component(CompProblem::NoComponent)));
                }
                return Ok(None);
            }
        },
    };
    // the body itself (`ground(base)`, `fastened(lid, base)`): its own frame, where it was made
    let whole = std::ptr::eq(root, e) && matches!(value, Value::Feature(_));
    let local = match whole {
        true => fa::Reference::frame(Frame::WORLD),
        false => as_reference(ev, value.clone())
            .ok_or_else(|| wrong(e.span, "a joint's end", Ty::Reference, &value))?,
    };
    if !doc(ev).insts.iter().any(|i| i.id == owner) {
        let span = ev.feats.iter().find(|f| f.id == owner).map_or(e.span, |f| f.span);
        let file = format!("#{}{owner}", ev.comp_path);
        doc(ev).insts.push(InstDef {
            id: owner.clone(),
            file,
            source: Source::Body,
            eval: None,
            body: None,
            pose: Frame::WORLD,
            ground: false,
            reference: false,
            suppressed: false,
            sub: false,
            span,
        });
        doc(ev).auto_ground.insert(owner.clone());
    }
    Ok(Some(RefDef {
        inst: owner,
        segs: if text.is_empty() {
            vec![]
        } else {
            vec![Seg {
                name: text,
                index: vec![],
            }]
        },
        span: e.span,
        local: Some(local),
    }))
}

/// A link's arguments with its two ends on bodies of this document worked out (bound to names
/// no source can write), so the rest reads them as any reference.
fn with_body_ends(ev: &mut Eval, args: &[Arg]) -> R<Ends> {
    let mut out = args.to_vec();
    let mut bound = vec![];
    let named = |n: &str| {
        args.iter()
            .position(|a| a.name.as_ref().is_some_and(|x| x.0 == n))
    };
    let positional: Vec<usize> = (0..args.len()).filter(|&k| args[k].name.is_none()).collect();
    let a = named("a").or(positional.first().copied());
    let b = named("b").or(positional.get(if named("a").is_some() { 0 } else { 1 }).copied());
    for k in [a, b].into_iter().flatten() {
        if let Some(r) = body_end(ev, &args[k].value)? {
            let n = format!("end {k}");
            out[k].value = Expr {
                kind: ExprKind::Name(n.clone()),
                span: args[k].value.span,
            };
            bound.push((n, asm(AsmVal::Ref(r))));
        }
    }
    Ok((out, bound))
}

/// Whether an expect line measures the assembly (its solve, overlaps, poses).
pub fn is_metric(s: &crate::syntax::Stmt) -> bool {
    matches!(&s.kind, StmtKind::Expect { metric, .. } if METRICS.contains(&metric.as_str()))
}

/// The document's own components made ready to solve: each body a joint names still a body,
/// the rule's grounds, and the bodies no joint names as one grounded occurrence, `#root`.
fn in_file(ev: &mut Eval, mut d: Doc) -> Doc {
    let solids = crate::props::solids(ev);
    for i in d.insts.iter().filter(|i| i.source == Source::Body) {
        if !solids.iter().any(|s| s.0 == i.id) {
            ev.errors.push(note(
                i.span,
                Issue::Component(CompProblem::BodyGone { id: i.id.clone() }),
            ));
        }
    }
    // grounded only as written (`ground: true`, `ground(X)`): nothing grounded is a warning
    for i in d.insts.iter_mut().filter(|i| d.grounds.contains(&i.id)) {
        i.ground = true;
    }
    let loose = solids
        .iter()
        .any(|s| !d.insts.iter().any(|i| i.source == Source::Body && i.id == s.0));
    if loose {
        d.insts.push(InstDef {
            id: ROOT.into(),
            file: root_key(&ev.comp_path),
            source: Source::Root,
            eval: None,
            body: None,
            pose: Frame::WORLD,
            ground: true,
            reference: false,
            suppressed: false,
            sub: false,
            span: Span { lo: 0, hi: 0 },
        });
    }
    d
}

/// The occurrence of a document's bodies no joint names.
pub const ROOT: &str = "#root";

fn mesh_of(b: &forge_topo::Body) -> Option<Arc<PartMesh>> {
    let q = forge_mesh::Quality {
        chord: MESH_CHORD,
        max_angle: forge_math::dmath::PI / 12.0,
    };
    forge_mesh::tessellate(b, &q)
        .ok()
        .map(|m| Arc::new(PartMesh::from_trimesh(&m, MESH_CHORD)))
}

/// A component of this document as a part: an inline one from its own evaluation, a body or the
/// root's bodies from the document's. Kept while the build they are read from is.
fn infile_load(
    ev: &mut Eval,
    prepared: &BTreeMap<String, Doc>,
    key: &str,
    refs: &[String],
    cache: &mut AsmCache,
    mesh: bool,
) -> Arc<PartLoad> {
    // the evaluation it is in: the calls its key is under (`#box(d=1)/lid(d=2)`)
    let rest = key.trim_start_matches('#');
    let path = match rest.rfind('/') {
        Some(k) => &rest[..=k],
        None => "",
    };
    let (Some(d), Some(owner)) = (prepared.get(path), owner_mut(ev, path)) else {
        return Arc::new(empty_load(key));
    };
    infile_load_in(owner, d, key, refs, cache, mesh)
}

/// The evaluation under `path` (`box(d=1)/lid(d=2)/`), found through its calls.
fn owner_mut<'a, 's>(ev: &'a mut Eval<'s>, path: &str) -> Option<&'a mut Eval<'s>> {
    let Some((first, rest)) = path.split_once('/') else {
        return Some(ev);
    };
    if first.is_empty() {
        return Some(ev);
    }
    let c = ev.comps.iter_mut().find(|c| c.id == first)?;
    owner_mut(&mut c.ev, rest)
}

/// Every evaluation with components of its own (the document's, and each call's that places
/// or joins anything), by its path, its document made ready to solve.
fn prepare_all(ev: &mut Eval, top: &Doc, out: &mut BTreeMap<String, Doc>) {
    out.insert(ev.comp_path.clone(), top.clone());
    let mut comps = std::mem::take(&mut ev.comps);
    for c in comps.iter_mut().filter(|c| c.sub) {
        let doc = c.ev.asm.clone().unwrap_or_default();
        let doc = in_file(&mut c.ev, doc);
        let errs = std::mem::take(&mut c.ev.errors);
        ev.errors.extend(errs);
        if !out.contains_key(&c.ev.comp_path) {
            prepare_all(&mut c.ev, &doc, out);
        }
    }
    ev.comps = comps;
}

/// `mesh`: its interference meshes made (`Options::meshes`; a drawing's loads ask none).
fn infile_load_in(
    ev: &mut Eval,
    d: &Doc,
    key: &str,
    refs: &[String],
    cache: &mut AsmCache,
    mesh: bool,
) -> Arc<PartLoad> {
    let Some(inst) = d.insts.iter().find(|i| i.file == key) else {
        return Arc::new(empty_load(key));
    };
    let rest = key.trim_start_matches('#');
    let name = rest.rsplit('/').next().unwrap_or(rest).to_string();
    let comp = ev.comps.iter().position(|c| c.id == name);
    let stage = match (inst.source, comp) {
        (Source::Inline, Some(k)) => ev.comps[k].ev.b.stages.last().cloned(),
        _ => ev.b.stages.last().cloned(),
    };
    let text = match (inst.source, comp) {
        (Source::Inline, Some(k)) => {
            let sp = ev.comps[k].span;
            text_hash(&format!("{}@{}", &ev.src[sp.lo as usize..sp.hi as usize], sp.lo))
        }
        _ => text_hash(ev.src),
    };
    if let (Some(st), Some((kst, kt, kr, km, l))) = (&stage, cache.infile.get(key))
        && Rc::ptr_eq(st, kst)
        && *kt == text
        && kr == refs
        && *km == mesh
    {
        return l.clone();
    }
    let mut out = empty_load(key);
    out.hash = content_hash(&format!("{key}\n{}", ev.src));
    match (inst.source, comp) {
        (Source::Inline, Some(k)) => {
            let mut comps = std::mem::take(&mut ev.comps);
            finish_load(&mut comps[k].ev, &mut out, refs, &[], mesh);
            ev.comps = comps;
        }
        (Source::Body | Source::Root, _) => {
            let all = crate::props::solids(ev);
            let body_ids: BTreeSet<&str> = d
                .insts
                .iter()
                .filter(|i| i.source == Source::Body)
                .map(|i| i.id.as_str())
                .collect();
            let mine: Vec<_> = all
                .into_iter()
                .filter(|s| {
                    if inst.source == Source::Body {
                        s.0 == name
                    } else {
                        !body_ids.contains(s.0.as_str())
                    }
                })
                .collect();
            let body = match &mine[..] {
                [] => None,
                [one] => Some(one.2.clone()),
                many => {
                    let mut b = forge_topo::Body::new();
                    for s in many {
                        b.append(&s.2);
                    }
                    Some(b)
                }
            };
            // a body is a part of the document: its own material, not the document's numbers
            let props = if inst.source == Source::Root {
                ev.props.clone()
            } else {
                crate::props::Props {
                    material: mine.first().and_then(|s| s.1.as_ref().map(|m| m.name.clone())),
                    ..Default::default()
                }
            };
            out.summary = Arc::new(crate::props::PartSummary::new(
                props,
                mine.into_iter().map(|(id, m, b)| (id, m, Arc::new(b))).collect(),
            ));
            if mesh {
                out.mesh = body.as_ref().and_then(mesh_of);
            }
            out.body = body;
        }
        _ => {}
    }
    let l = Arc::new(out);
    if let Some(st) = stage {
        cache
            .infile
            .insert(key.to_string(), (st, text, refs.to_vec(), mesh, l.clone()));
    }
    l
}

fn empty_load(key: &str) -> PartLoad {
    PartLoad {
        hash: content_hash(key),
        body: None,
        refs: BTreeMap::new(),
        features: vec![],
        kinds: vec![],
        errors: vec![],
        mesh: None,
        bodies: BTreeMap::new(),
        summary: Default::default(),
        declared: false,
        body_meshes: BTreeMap::new(),
        ident: 0,
        threads: vec![],
        tooling: None,
        anchors: None,
    }
}

/// Each body of the document and the features that made it, in the order made: those whose faces
/// it holds (lineage through every boolean), and the sketches they were drawn from. What a body
/// component's tree lists.
pub fn body_chains(ev: &mut Eval) -> Vec<(String, Vec<String>)> {
    let n = ev.b.built();
    let Some(st) = ev.b.stages.last().cloned() else {
        return vec![];
    };
    let origin = ev.b.origin(n);
    let profiles: Vec<(usize, String)> = ev
        .top_level()
        .into_iter()
        .filter_map(|(_, v)| match v {
            Value::Feature(f) => f.prof.as_ref().map(|p| (f.idx, p.borrow().id.clone())),
            _ => None,
        })
        .collect();
    let mut out = vec![];
    for s in &st.solids {
        let Some(own) = ev.feats.iter().find(|f| f.idx == s.id) else {
            continue;
        };
        let body = forge_model::finish(s.body.clone());
        let mut made: BTreeSet<usize> = [s.id].into();
        for f in body.face_ids() {
            if let Some(ks) = origin.get(&body.face(f).name) {
                made.extend(ks.iter().copied());
            }
        }
        // another body's own making is that body's (a face it touches is joined across bodies)
        for o in st.solids.iter().filter(|o| o.id != s.id) {
            made.remove(&o.id);
        }
        let mut ids: BTreeSet<String> = ev
            .feats
            .iter()
            .filter(|f| made.contains(&f.idx))
            .map(|f| f.id.clone())
            .collect();
        for (k, sk) in &profiles {
            if made.contains(k) {
                ids.insert(sk.clone());
            }
        }
        let ordered: Vec<String> = ev
            .feats
            .iter()
            .filter(|f| ids.contains(&f.id))
            .map(|f| f.id.clone())
            .collect();
        out.push((own.id.clone(), ordered));
    }
    out
}

/// Components placing each other round, found before anything runs (R1): each component block
/// at any depth, the components its statements place or call; a cycle is `ComponentCycle`.
pub fn cycle_check(prog: &[crate::syntax::Stmt], errors: &mut Vec<Error>) {
    fn blocks<'a>(stmts: &'a [crate::syntax::Stmt], out: &mut Vec<&'a crate::syntax::Stmt>) {
        for s in stmts {
            if let StmtKind::Block {
                kind: crate::syntax::BlockKind::Component,
                body,
                ..
            } = &s.kind
            {
                out.push(s);
                blocks(body, out);
            }
        }
    }
    let mut all = vec![];
    blocks(prog, &mut all);
    let names: BTreeSet<String> = all
        .iter()
        .filter_map(|s| match &s.kind {
            StmtKind::Block { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    let mut edges: BTreeMap<String, (Span, BTreeSet<String>)> = BTreeMap::new();
    for s in &all {
        let StmtKind::Block { name, body, .. } = &s.kind else {
            continue;
        };
        let reads: BTreeSet<String> = body
            .iter()
            .filter(|b| {
                !matches!(
                    b.kind,
                    StmtKind::Block {
                        kind: crate::syntax::BlockKind::Component,
                        ..
                    }
                )
            })
            .flat_map(crate::edit::mentions)
            .filter(|n| names.contains(n))
            .collect();
        edges.insert(name.clone(), (s.span, reads));
    }
    let mut said = BTreeSet::new();
    for (a, (span, _)) in &edges {
        // depth first from a: a path back to a is a cycle
        let mut todo: Vec<(String, String)> = edges[a].1.iter().map(|b| (b.clone(), b.clone())).collect();
        let mut seen = BTreeSet::new();
        while let Some((x, first)) = todo.pop() {
            if &x == a {
                if said.insert(a.clone()) {
                    errors.push(comp_error(
                        *span,
                        CompProblem::Cycle {
                            a: a.clone(),
                            b: first,
                        },
                    ));
                }
                break;
            }
            if seen.insert(x.clone())
                && let Some((_, next)) = edges.get(&x)
            {
                todo.extend(next.iter().map(|n| (n.clone(), first.clone())));
            }
        }
    }
}

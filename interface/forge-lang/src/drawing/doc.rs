//! A drawing document read into typed items: the models it draws, its sheets, its views (each
//! a child of the view it is projected, cut or enlarged from), and the annotations on them. The
//! values are the document's own small language over Forge's syntax: numbers (mm, deg), strings,
//! tuples and lists, `shared` params and arithmetic on them, references `e(..)`/`f(..)` to the
//! model's edges and faces by persistent name.

use super::ops::DrawOp;
use crate::syntax::{self, Arg, BinOp, Expr, ExprKind, Span, StmtKind, Unit};
use forge_agent::json::Json;
use forge_math::{Point2, Vec3};
use forge_topo::Name;
use std::collections::HashMap;

/// A model entity by its persistent name, in an occurrence (empty for a part).
#[derive(Clone, Debug, PartialEq)]
pub struct Ref {
    pub occ: String,
    pub name: Name,
    pub face: bool,
    pub at: Option<At>,
    pub span: Span,
}

/// A point of an edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Start,
    End,
    Mid,
    Center,
}

impl Ref {
    /// The reference as the document writes it.
    pub fn text(&self) -> String {
        let f = if self.face { "f" } else { "e" };
        let base = if self.occ.is_empty() {
            format!("{f}(\"{:016x}\")", self.name.0)
        } else {
            format!("{f}(\"{}\", \"{:016x}\")", self.occ, self.name.0)
        };
        match self.at {
            None => base,
            Some(a) => format!(
                "{base}.{}",
                match a {
                    At::Start => "start",
                    At::End => "end",
                    At::Mid => "mid",
                    At::Center => "center",
                }
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Num(f64),
    Str(String),
    Bool(bool),
    List(Vec<Val>),
    /// Another item of the document, by id.
    Item(String),
    Ref(Ref),
}

impl Val {
    fn ty(&self) -> &'static str {
        match self {
            Val::Num(_) => "number",
            Val::Str(_) => "string",
            Val::Bool(_) => "bool",
            Val::List(_) => "list",
            Val::Item(_) => "item",
            Val::Ref(_) => "reference",
        }
    }
}

/// What can be wrong with a drawing: each with the data a fix needs.
#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    Syntax {
        message: String,
    },
    NotADrawing,
    UnknownName {
        name: String,
    },
    UnknownFunction {
        name: String,
    },
    MissingArg {
        func: &'static str,
        arg: &'static str,
    },
    UnknownArg {
        func: &'static str,
        arg: String,
    },
    WrongType {
        func: &'static str,
        arg: String,
        want: &'static str,
        got: &'static str,
    },
    BadValue {
        func: &'static str,
        arg: String,
        why: String,
    },
    Duplicate {
        id: String,
    },
    /// The model file could not be read or did not build.
    Model {
        file: String,
        why: String,
    },
    /// A reference no longer names anything in the model: Inventor's sick annotation.
    Sick {
        item: String,
        reference: String,
    },
    /// The reference exists but is not drawn in the view (hidden, cropped, turned edge-on).
    NotInView {
        item: String,
        reference: String,
    },
    /// The reference is drawn only as a hidden line: Inventor dimensions those, drafters don't.
    HiddenOnly {
        item: String,
        reference: String,
    },
    /// The thing asked for does not apply here (a balloon on a part drawing).
    NotHere {
        item: String,
        why: String,
    },
    /// A GD&T rule of the sheet's standard broken.
    Gdt {
        item: String,
        rule: GdtRule,
        detail: String,
    },
}

/// The GD&T rules a drawing is checked against (ASME Y14.5-2018, ISO 1101/5459).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GdtRule {
    /// A frame cites a datum letter no datum feature symbol or target on the drawing defines.
    DatumUndefined,
    /// Two datum feature symbols with one letter.
    DatumDuplicate,
    /// I, O and Q are not datum letters (nor anything but capitals).
    DatumLetter,
    /// Form tolerances (straightness, flatness, circularity, cylindricity) take no datums.
    NoDatumsAllowed,
    /// Orientation and runout tolerances need a datum.
    DatumsNeeded,
    /// The standard does not have the characteristic (Y14.5-2018 dropped concentricity, symmetry).
    NotInStandard,
    /// A diameter zone on a characteristic that cannot have one.
    DiameterNotAllowed,
    /// MMC or LMC on a feature that has no size (a plane, a line).
    ModifierNeedsSize,
    /// Runout is measured about a datum axis: its datum should be a feature of size.
    RunoutNeedsAxis,
    /// Position with no datum reference frame.
    PositionNeedsFrame,
    /// Position located by toleranced, not basic, dimensions.
    PositionNotBasic,
    /// Position with no basic dimension locating it one way (or either).
    PositionNotLocated,
}

impl GdtRule {
    pub fn word(self) -> &'static str {
        match self {
            GdtRule::DatumUndefined => "DatumUndefined",
            GdtRule::DatumDuplicate => "DatumDuplicate",
            GdtRule::DatumLetter => "DatumLetter",
            GdtRule::NoDatumsAllowed => "NoDatumsAllowed",
            GdtRule::DatumsNeeded => "DatumsNeeded",
            GdtRule::NotInStandard => "NotInStandard",
            GdtRule::DiameterNotAllowed => "DiameterNotAllowed",
            GdtRule::ModifierNeedsSize => "ModifierNeedsSize",
            GdtRule::RunoutNeedsAxis => "RunoutNeedsAxis",
            GdtRule::PositionNeedsFrame => "PositionNeedsFrame",
            GdtRule::PositionNotBasic => "PositionNotBasic",
            GdtRule::PositionNotLocated => "PositionNotLocated",
        }
    }
}

impl Problem {
    pub fn kind(&self) -> &'static str {
        match self {
            Problem::Syntax { .. } => "Syntax",
            Problem::NotADrawing => "NotADrawing",
            Problem::UnknownName { .. } => "UnknownName",
            Problem::UnknownFunction { .. } => "UnknownFunction",
            Problem::MissingArg { .. } => "MissingArg",
            Problem::UnknownArg { .. } => "UnknownArg",
            Problem::WrongType { .. } => "WrongType",
            Problem::BadValue { .. } => "BadValue",
            Problem::Duplicate { .. } => "Duplicate",
            Problem::Model { .. } => "Model",
            Problem::Sick { .. } => "Sick",
            Problem::NotInView { .. } => "NotInView",
            Problem::NotHere { .. } => "NotHere",
            Problem::HiddenOnly { .. } => "HiddenOnly",
            Problem::Gdt { .. } => "Gdt",
        }
    }

    pub fn text(&self) -> String {
        match self {
            Problem::Syntax { message } => message.clone(),
            Problem::NotADrawing => "a drawing's first line is the word `drawing`".into(),
            Problem::UnknownName { name } => format!("no `{name}` above this line"),
            Problem::UnknownFunction { name } => format!("`{name}` is not a drawing function"),
            Problem::MissingArg { func, arg } => format!("{func} needs `{arg}`"),
            Problem::UnknownArg { func, arg } => format!("{func} has no argument `{arg}`"),
            Problem::WrongType { func, arg, want, got } => {
                format!("{func}'s `{arg}` is a {want}, not a {got}")
            }
            Problem::BadValue { func, arg, why } => format!("{func}'s `{arg}`: {why}"),
            Problem::Duplicate { id } => format!("`{id}` is already an item"),
            Problem::Model { file, why } => format!("{file}: {why}"),
            Problem::Sick { item, reference } => {
                format!("{item}: {reference} is no longer in the model; the annotation is sick")
            }
            Problem::NotInView { item, reference } => format!("{item}: {reference} is not drawn in its view"),
            Problem::NotHere { item, why } => format!("{item}: {why}"),
            Problem::HiddenOnly { item, reference } => {
                format!(
                    "{item}: {reference} is only a hidden line in this view; dimension it where it is seen"
                )
            }
            Problem::Gdt { item, rule, detail } => format!("{item}: {} ({detail})", rule.word()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawError {
    pub problem: Problem,
    pub span: Span,
    /// A warning: the drawing is still made (a sick dimension is drawn sick).
    pub warning: bool,
    pub fixes: Vec<DrawOp>,
}

impl DrawError {
    pub fn json(&self, src: &str) -> Json {
        let (line, col) = line_col(src, self.span.lo as usize);
        Json::obj(vec![
            ("kind", self.problem.kind().into()),
            ("severity", if self.warning { "warning" } else { "error" }.into()),
            ("message", self.problem.text().into()),
            ("line", line.into()),
            ("col", col.into()),
            ("span", crate::error::span_json(self.span)),
            ("fixes", Json::Arr(self.fixes.iter().map(DrawOp::json).collect())),
        ])
    }
}

pub fn line_col(src: &str, at: usize) -> (usize, usize) {
    let before = &src[..at.min(src.len())];
    let line = before.matches('\n').count() + 1;
    let col = before.len() - before.rfind('\n').map_or(0, |k| k + 1) + 1;
    (line, col)
}

#[derive(Clone, Debug, PartialEq)]
pub struct SheetDef {
    pub standard: forge_draw::gdt::Standard,
    pub size: String,
    pub w: f64,
    pub h: f64,
    pub third_angle: bool,
    pub fields: forge_draw::sheet::Fields,
}

/// How a view is made from its model or parent.
#[derive(Clone, Debug, PartialEq)]
pub enum ViewKind {
    Base {
        model: String,
        /// Toward the eye and up the sheet, in model space.
        toward: Vec3,
        up: Vec3,
        /// How far apart an assembly is drawn: 0 together, 1 fully exploded.
        explode: f64,
    },
    Projected {
        parent: String,
    },
    Section {
        parent: String,
        /// The cutting line on the parent, in its view (model mm).
        line: Vec<Point2>,
    },
    Detail {
        parent: String,
        centre: Point2,
        /// A circle's radius, or a rectangle's half sizes (model mm on the parent).
        r: f64,
        rect: Option<(f64, f64)>,
    },
    /// A sheet metal part's flat pattern.
    Flat {
        model: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewDef {
    pub kind: ViewKind,
    pub sheet: Option<String>,
    /// The view's centre on the sheet (mm).
    pub at: Point2,
    pub scale: Option<f64>,
    pub hidden: Option<bool>,
    pub tangent: Option<bool>,
    pub label: Option<String>,
    /// Occurrences left out, by path or path prefix.
    pub hide: Vec<String>,
}

/// An annotation's kind and its references.
#[derive(Clone, Debug, PartialEq)]
pub enum AnnotKind {
    /// Between two references, or along one; `dir` is horizontal, vertical, aligned or None
    /// (chosen by where it is placed).
    Linear {
        a: Ref,
        b: Option<Ref>,
        dir: Option<String>,
    },
    Angle {
        a: Ref,
        b: Ref,
    },
    Radius {
        e: Ref,
    },
    Diameter {
        e: Ref,
    },
    HoleNote {
        e: Ref,
    },
    /// Distances from a base reference to each of the others along one direction: an ordinate
    /// set (values at each), a baseline set (stacked dimensions) or a chain (each to the next).
    Set {
        base: Ref,
        refs: Vec<Ref>,
        dir: String,
        style: SetStyle,
    },
    CenterMark {
        e: Ref,
    },
    CenterMarks,
    Centerlines,
    Centerline {
        a: Ref,
        b: Ref,
    },
    Leader {
        r: Ref,
    },
    Balloon {
        occ: String,
    },
    AutoBalloons,
    /// A feature control frame: its rows (more than one stacked, or one composite frame).
    Fcf {
        to: Attach,
        rows: Vec<forge_draw::gdt::Row>,
        composite: bool,
        all_around: bool,
        between: Option<(String, String)>,
    },
    Datum {
        to: Attach,
        letter: String,
    },
    DatumTarget {
        r: Ref,
        label: String,
        size: String,
        kind: forge_draw::gdt::Target,
        area: f64,
    },
    Surface {
        r: Ref,
        value: String,
        process: forge_draw::gdt::Process,
        lay: String,
    },
    Weld {
        r: Ref,
        weld: forge_draw::gdt::Weld,
    },
    Edge {
        r: Ref,
        upper: String,
        lower: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetStyle {
    Ordinate,
    Baseline,
    Chain,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnnotDef {
    pub view: String,
    pub kind: AnnotKind,
    /// Where it is placed, in its view (model mm from the view's origin).
    pub at: Option<Point2>,
    /// Text written in place of the measured value, or added to a note.
    pub text: Option<String>,
    pub prefix: String,
    pub suffix: String,
    /// The dimension's tolerance, as written.
    pub tol: Option<TolSpec>,
    /// Decimal places of the value (2 when not given).
    pub places: Option<usize>,
}

/// A dimension's tolerance.
#[derive(Clone, Debug, PartialEq)]
pub enum TolSpec {
    /// `25 ±0.1`
    Symmetric(f64),
    /// `25` with `+0.1` over `-0.05` beside it
    Deviation(f64, f64),
    /// The limits themselves, upper over lower.
    Limits(f64, f64),
    Max,
    Min,
    /// A theoretically exact value, boxed.
    Basic,
    /// For reference only, in parentheses.
    Reference,
    /// An ISO 286 class (`H7`, `g6`) and its deviations.
    Fit(String),
}

/// What a GD&T symbol is attached to: a model reference by leader, or another annotation (a
/// dimension, so the feature of size it measures).
#[derive(Clone, Debug, PartialEq)]
pub enum Attach {
    Ref(Ref),
    Item(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemKind {
    Model {
        path: String,
        asm: bool,
        /// g/cm3, for the title block's mass (until the part carries its material's density).
        density: Option<f64>,
    },
    Sheet(SheetDef),
    View(ViewDef),
    Annot(AnnotDef),
    /// Text on a sheet at a sheet point.
    Note {
        sheet: Option<String>,
        at: Point2,
        text: String,
    },
    PartsList {
        sheet: Option<String>,
        view: Option<String>,
        at: Option<Point2>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: String,
    pub span: Span,
    pub kind: ItemKind,
}

/// The document, read: items in order, and what was wrong.
#[derive(Clone, Debug, Default)]
pub struct Doc {
    pub items: Vec<Item>,
    pub errors: Vec<DrawError>,
    pub params: Vec<(String, f64)>,
}

impl Doc {
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }
    pub fn view(&self, id: &str) -> Option<&ViewDef> {
        match &self.item(id)?.kind {
            ItemKind::View(v) => Some(v),
            _ => None,
        }
    }
}

/// Whether a program is a drawing: its first statement is the word `drawing`.
pub fn declared(prog: &[syntax::Stmt]) -> bool {
    matches!(prog.first().map(|s| &s.kind), Some(StmtKind::Expr(Expr { kind: ExprKind::Name(n), .. })) if n == "drawing")
}

/// The drawing's built-ins: what agents and the app may write.
pub const FUNCTIONS: &[&str] = &[
    "part",
    "assembly",
    "sheet",
    "base_view",
    "projected",
    "section",
    "detail",
    "flat_view",
    "dim",
    "angle",
    "radius",
    "diameter",
    "hole_note",
    "ordinate",
    "baseline",
    "chain",
    "center_mark",
    "center_marks",
    "centerlines",
    "centerline",
    "leader",
    "note",
    "balloon",
    "auto_balloons",
    "fcf",
    "datum",
    "datum_target",
    "surface",
    "weld",
    "edge",
    "parts_list",
    "e",
    "f",
];

struct Reader<'a> {
    env: HashMap<String, Val>,
    doc: Doc,
    src: &'a str,
}

fn err(problem: Problem, span: Span) -> DrawError {
    DrawError {
        problem,
        span,
        warning: false,
        fixes: vec![],
    }
}

type R<T> = Result<T, DrawError>;

/// Arguments of one call: positional, then keyword.
struct Args {
    func: &'static str,
    pos: Vec<(Val, Span)>,
    kw: Vec<(String, Val, Span)>,
    used: Vec<bool>,
    span: Span,
}

impl Args {
    fn get(&mut self, name: &str, k: Option<usize>) -> Option<(Val, Span)> {
        if let Some(i) = self.kw.iter().position(|x| x.0 == name) {
            self.used[i] = true;
            return Some((self.kw[i].1.clone(), self.kw[i].2));
        }
        k.and_then(|k| self.pos.get(k).cloned())
    }
    fn need(&mut self, name: &'static str, k: Option<usize>) -> R<(Val, Span)> {
        self.get(name, k).ok_or_else(|| {
            err(
                Problem::MissingArg {
                    func: self.func,
                    arg: name,
                },
                self.span,
            )
        })
    }
    fn wrong(&self, arg: &str, want: &'static str, v: &Val, span: Span) -> DrawError {
        err(
            Problem::WrongType {
                func: self.func,
                arg: arg.to_string(),
                want,
                got: v.ty(),
            },
            span,
        )
    }
    fn num(&mut self, name: &'static str, k: Option<usize>) -> R<Option<f64>> {
        match self.get(name, k) {
            None => Ok(None),
            Some((Val::Num(x), _)) => Ok(Some(x)),
            Some((v, s)) => Err(self.wrong(name, "number", &v, s)),
        }
    }
    fn string(&mut self, name: &'static str, k: Option<usize>) -> R<Option<String>> {
        match self.get(name, k) {
            None => Ok(None),
            Some((Val::Str(x), _)) => Ok(Some(x)),
            Some((v, s)) => Err(self.wrong(name, "string", &v, s)),
        }
    }
    fn boolean(&mut self, name: &'static str) -> R<Option<bool>> {
        match self.get(name, None) {
            None => Ok(None),
            Some((Val::Bool(x), _)) => Ok(Some(x)),
            Some((v, s)) => Err(self.wrong(name, "bool", &v, s)),
        }
    }
    fn point(&mut self, name: &'static str, k: Option<usize>) -> R<Option<Point2>> {
        match self.get(name, k) {
            None => Ok(None),
            Some((v, s)) => as_point(&v)
                .map(Some)
                .ok_or_else(|| self.wrong(name, "point (x, y)", &v, s)),
        }
    }
    fn item(&mut self, name: &'static str, k: Option<usize>) -> R<Option<String>> {
        match self.get(name, k) {
            None => Ok(None),
            Some((Val::Item(x), _)) => Ok(Some(x)),
            Some((v, s)) => Err(self.wrong(name, "item", &v, s)),
        }
    }
    fn reference(&mut self, name: &'static str, k: Option<usize>) -> R<Option<Ref>> {
        match self.get(name, k) {
            None => Ok(None),
            Some((Val::Ref(x), _)) => Ok(Some(x)),
            Some((v, s)) => Err(self.wrong(name, "reference e(..) or f(..)", &v, s)),
        }
    }
    fn need_ref(&mut self, name: &'static str, k: Option<usize>) -> R<Ref> {
        let (v, s) = self.need(name, k)?;
        match v {
            Val::Ref(r) => Ok(r),
            v => Err(self.wrong(name, "reference e(..) or f(..)", &v, s)),
        }
    }
    fn refs(&mut self, name: &'static str, k: Option<usize>) -> R<Vec<Ref>> {
        let (v, s) = self.need(name, k)?;
        match v {
            Val::List(xs) => xs
                .into_iter()
                .map(|x| match x {
                    Val::Ref(r) => Ok(r),
                    v => Err(self.wrong(name, "list of references", &v, s)),
                })
                .collect(),
            v => Err(self.wrong(name, "list of references", &v, s)),
        }
    }
    /// Every keyword given was one the function reads.
    fn done(&self) -> R<()> {
        match self.kw.iter().zip(&self.used).find(|(_, u)| !**u) {
            Some((k, _)) => Err(err(
                Problem::UnknownArg {
                    func: self.func,
                    arg: k.0.clone(),
                },
                k.2,
            )),
            None => Ok(()),
        }
    }
}

fn as_point(v: &Val) -> Option<Point2> {
    match v {
        Val::List(xs) => match xs[..] {
            [Val::Num(x), Val::Num(y)] => Some(Point2::new(x, y)),
            _ => None,
        },
        _ => None,
    }
}

fn as_vec3(v: &Val) -> Option<Vec3> {
    match v {
        Val::List(xs) => match xs[..] {
            [Val::Num(x), Val::Num(y), Val::Num(z)] => Some(Vec3::new(x, y, z)),
            _ => None,
        },
        _ => None,
    }
}

impl Reader<'_> {
    fn expr(&mut self, e: &Expr) -> R<Val> {
        Ok(match &e.kind {
            ExprKind::Num(x, unit) => {
                let _ = matches!(unit, Unit::Deg | Unit::Mm | Unit::None);
                Val::Num(*x)
            }
            ExprKind::Str(s) => Val::Str(s.replace("\\n", "\n")),
            ExprKind::Bool(b) => Val::Bool(*b),
            ExprKind::Name(n) => match self.env.get(n) {
                Some(v) => v.clone(),
                None if FUNCTIONS.contains(&n.as_str()) => {
                    return Err(err(Problem::UnknownName { name: n.clone() }, e.span));
                }
                None => return Err(err(Problem::UnknownName { name: n.clone() }, e.span)),
            },
            ExprKind::List(xs) | ExprKind::Tuple(xs) => {
                Val::List(xs.iter().map(|x| self.expr(x)).collect::<R<_>>()?)
            }
            ExprKind::Neg(a) => match self.expr(a)? {
                Val::Num(x) => Val::Num(-x),
                v => return Err(self.bad_op(&v, e.span)),
            },
            ExprKind::Bin(op, a, b) => {
                let (x, y) = (self.expr(a)?, self.expr(b)?);
                match (op, &x, &y) {
                    (BinOp::Add, Val::Num(p), Val::Num(q)) => Val::Num(p + q),
                    (BinOp::Sub, Val::Num(p), Val::Num(q)) => Val::Num(p - q),
                    (BinOp::Mul, Val::Num(p), Val::Num(q)) => Val::Num(p * q),
                    (BinOp::Div, Val::Num(p), Val::Num(q)) if *q != 0.0 => Val::Num(p / q),
                    (BinOp::Add, Val::List(p), Val::List(q)) if p.len() == q.len() => Val::List(
                        p.iter()
                            .zip(q)
                            .map(|(a, b)| match (a, b) {
                                (Val::Num(a), Val::Num(b)) => Ok(Val::Num(a + b)),
                                _ => Err(self.bad_op(a, e.span)),
                            })
                            .collect::<R<_>>()?,
                    ),
                    (BinOp::Add, Val::Str(p), Val::Str(q)) => Val::Str(format!("{p}{q}")),
                    _ => return Err(self.bad_op(&x, e.span)),
                }
            }
            ExprKind::Attr(base, member) => {
                let v = self.expr(base)?;
                match (v, member.as_str()) {
                    (Val::Ref(mut r), m @ ("start" | "end" | "mid" | "center")) => {
                        r.at = Some(match m {
                            "start" => At::Start,
                            "end" => At::End,
                            "mid" => At::Mid,
                            _ => At::Center,
                        });
                        r.span = e.span;
                        Val::Ref(r)
                    }
                    (v, _) => return Err(self.bad_op(&v, e.span)),
                }
            }
            ExprKind::Call(f, args) => {
                let ExprKind::Name(name) = &f.kind else {
                    return Err(err(
                        Problem::UnknownFunction {
                            name: self.src[f.span.lo as usize..f.span.hi as usize].to_string(),
                        },
                        f.span,
                    ));
                };
                return self.call(name, args, e.span, None);
            }
            _ => {
                return Err(err(
                    Problem::Syntax {
                        message: "not a drawing value".into(),
                    },
                    e.span,
                ));
            }
        })
    }

    fn bad_op(&self, v: &Val, span: Span) -> DrawError {
        err(
            Problem::WrongType {
                func: "an expression",
                arg: self.src[span.lo as usize..span.hi as usize].to_string(),
                want: "number",
                got: v.ty(),
            },
            span,
        )
    }

    fn args(&mut self, func: &'static str, args: &[Arg], span: Span) -> R<Args> {
        let mut pos = Vec::new();
        let mut kw = Vec::new();
        for a in args {
            let v = self.expr(&a.value)?;
            match &a.name {
                Some((n, _)) => kw.push((n.clone(), v, a.value.span)),
                None => pos.push((v, a.value.span)),
            }
        }
        let used = vec![false; kw.len()];
        Ok(Args {
            func,
            pos,
            kw,
            used,
            span,
        })
    }

    /// What a GD&T symbol hangs on: a reference, or an annotation item above it.
    fn attach(&self, a: &mut Args, func: &'static str) -> R<Attach> {
        match a.need("to", Some(1))? {
            (Val::Ref(r), _) => Ok(Attach::Ref(r)),
            (Val::Item(id), s) => match self.doc.item(&id).map(|i| &i.kind) {
                Some(ItemKind::Annot(_)) => Ok(Attach::Item(id)),
                _ => Err(err(
                    Problem::BadValue {
                        func,
                        arg: "to".into(),
                        why: format!(
                            "`{id}` is not an annotation: a symbol hangs on a reference or a dimension"
                        ),
                    },
                    s,
                )),
            },
            (v, s) => Err(a.wrong("to", "reference or dimension", &v, s)),
        }
    }

    /// A dimension's tolerance from its arguments: tol, limits, fit, basic, reference, bound.
    fn tolerance(&self, a: &mut Args, func: &'static str, span: Span) -> R<Option<TolSpec>> {
        let mut found: Vec<TolSpec> = Vec::new();
        match a.get("tol", None) {
            None => {}
            Some((Val::Num(x), _)) => found.push(TolSpec::Symmetric(x.abs())),
            Some((Val::List(xs), s)) => match xs[..] {
                [Val::Num(u), Val::Num(l)] if u >= l => found.push(TolSpec::Deviation(u, l)),
                _ => {
                    return Err(err(
                        Problem::BadValue {
                            func,
                            arg: "tol".into(),
                            why: "(upper, lower) deviations, upper first".into(),
                        },
                        s,
                    ));
                }
            },
            Some((v, s)) => return Err(a.wrong("tol", "number or (upper, lower)", &v, s)),
        }
        match a.get("limits", None) {
            None => {}
            Some((Val::List(xs), s)) => match xs[..] {
                [Val::Num(u), Val::Num(l)] if u >= l => found.push(TolSpec::Limits(u, l)),
                _ => {
                    return Err(err(
                        Problem::BadValue {
                            func,
                            arg: "limits".into(),
                            why: "(upper, lower) deviations, upper first".into(),
                        },
                        s,
                    ));
                }
            },
            Some((v, s)) => return Err(a.wrong("limits", "(upper, lower)", &v, s)),
        }
        if let Some(f) = a.string("fit", None)? {
            found.push(TolSpec::Fit(f));
        }
        if a.boolean("basic")? == Some(true) {
            found.push(TolSpec::Basic);
        }
        if a.boolean("reference")? == Some(true) {
            found.push(TolSpec::Reference);
        }
        match a.string("bound", None)?.as_deref() {
            None => {}
            Some("max") => found.push(TolSpec::Max),
            Some("min") => found.push(TolSpec::Min),
            Some(o) => {
                return Err(err(
                    Problem::BadValue {
                        func,
                        arg: "bound".into(),
                        why: format!("`{o}`: max or min"),
                    },
                    span,
                ));
            }
        }
        if found.len() > 1 {
            return Err(err(
                Problem::BadValue {
                    func,
                    arg: "tol".into(),
                    why: "one tolerance a dimension: tol, limits, fit, basic, reference or bound".into(),
                },
                span,
            ));
        }
        Ok(found.pop())
    }

    fn view_of(&self, a: &mut Args, k: Option<usize>) -> R<String> {
        let (v, s) = a.need("view", k)?;
        match v {
            Val::Item(id) if self.doc.view(&id).is_some() => Ok(id),
            v => Err(a.wrong("view", "view", &v, s)),
        }
    }

    /// A call: a reference, or (with `id`) an item of the document.
    fn call(&mut self, name: &str, args: &[Arg], span: Span, id: Option<&str>) -> R<Val> {
        let Some(&func) = FUNCTIONS.iter().find(|f| **f == name) else {
            return Err(err(
                Problem::UnknownFunction {
                    name: name.to_string(),
                },
                span,
            ));
        };
        if func == "e" || func == "f" {
            let a = self.args(func, args, span)?;
            let (occ, hex, hs) = match (a.pos.first(), a.pos.get(1)) {
                (Some((Val::Str(o), _)), Some((Val::Str(h), s))) => (o.clone(), h.clone(), *s),
                (Some((Val::Str(h), s)), None) => (String::new(), h.clone(), *s),
                _ => return Err(err(Problem::MissingArg { func, arg: "name" }, span)),
            };
            a.done()?;
            let name = u64::from_str_radix(hex.trim_start_matches('@'), 16).map_err(|_| {
                err(
                    Problem::BadValue {
                        func,
                        arg: "name".into(),
                        why: "a persistent name is 16 hex digits".into(),
                    },
                    hs,
                )
            })?;
            return Ok(Val::Ref(Ref {
                occ,
                name: Name(name),
                face: func == "f",
                at: None,
                span,
            }));
        }
        let Some(id) = id else {
            return Err(err(
                Problem::NotHere {
                    item: name.to_string(),
                    why: "an item is made on its own line: `name = ..`".into(),
                },
                span,
            ));
        };
        let mut a = self.args(func, args, span)?;
        let kind = match func {
            "part" | "assembly" => {
                let (v, s) = a.need("file", Some(0))?;
                let Val::Str(path) = v else {
                    return Err(a.wrong("file", "string", &v, s));
                };
                let density = a.num("density", None)?;
                if density.is_some_and(|d| d <= 0.0) {
                    return Err(err(
                        Problem::BadValue {
                            func,
                            arg: "density".into(),
                            why: "a density is more than 0 (g/cm3)".into(),
                        },
                        span,
                    ));
                }
                ItemKind::Model {
                    path,
                    asm: func == "assembly",
                    density,
                }
            }
            "sheet" => {
                let size = a.string("size", Some(0))?.unwrap_or_else(|| "B".into());
                let Some((w, h)) = forge_draw::sheet::size(&size) else {
                    return Err(err(
                        Problem::BadValue {
                            func,
                            arg: "size".into(),
                            why: format!(
                                "sizes are {}",
                                forge_draw::sheet::SIZES
                                    .iter()
                                    .map(|s| s.0)
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            ),
                        },
                        span,
                    ));
                };
                let third_angle = match a.string("projection", None)?.as_deref() {
                    None | Some("third") => true,
                    Some("first") => false,
                    Some(o) => {
                        return Err(err(
                            Problem::BadValue {
                                func,
                                arg: "projection".into(),
                                why: format!("`{o}`: third or first"),
                            },
                            span,
                        ));
                    }
                };
                let mut f = forge_draw::sheet::Fields {
                    size: size.clone(),
                    ..Default::default()
                };
                for (k, slot) in [
                    ("title", &mut f.title),
                    ("number", &mut f.number),
                    ("rev", &mut f.rev),
                    ("material", &mut f.material),
                    ("drawn", &mut f.drawn),
                    ("date", &mut f.date),
                    ("mass", &mut f.mass),
                    ("company", &mut f.company),
                    ("scale", &mut f.scale),
                    ("tolerance", &mut f.tolerance),
                ] {
                    if let Some(v) = a.get(k, None) {
                        *slot = match v.0 {
                            Val::Str(s) => s,
                            v => return Err(a.wrong(k, "string", &v, v_span(&a, k))),
                        };
                    }
                }
                let standard = match a.string("standard", None)?.as_deref() {
                    None | Some("ASME") => forge_draw::gdt::Standard::Asme,
                    Some("ISO") => forge_draw::gdt::Standard::Iso,
                    Some(o) => {
                        return Err(err(
                            Problem::BadValue {
                                func,
                                arg: "standard".into(),
                                why: format!("`{o}`: ASME (Y14.5-2018) or ISO (1101)"),
                            },
                            span,
                        ));
                    }
                };
                ItemKind::Sheet(SheetDef {
                    standard,
                    size,
                    w,
                    h,
                    third_angle,
                    fields: f,
                })
            }
            "base_view" | "projected" | "section" | "detail" | "flat_view" => {
                let kind = match func {
                    "base_view" | "flat_view" => {
                        let model = match a.need("model", Some(0))? {
                            (Val::Item(m), _)
                                if matches!(
                                    self.doc.item(&m).map(|i| &i.kind),
                                    Some(ItemKind::Model { .. })
                                ) =>
                            {
                                m
                            }
                            (v, s) => return Err(a.wrong("model", "model", &v, s)),
                        };
                        if func == "flat_view" {
                            ViewKind::Flat { model }
                        } else {
                            let (toward, up) = match a.get("dir", None) {
                                None => (Vec3::new(0.0, -1.0, 0.0), Vec3::Z),
                                Some((Val::Str(n), s)) => match forge_draw::View::named(&n) {
                                    Some(v) => (v.toward.get(), v.up.get()),
                                    None => {
                                        return Err(err(
                                            Problem::BadValue {
                                                func,
                                                arg: "dir".into(),
                                                why: format!(
                                                    "views are {}",
                                                    forge_draw::view::NAMED.join(" ")
                                                ),
                                            },
                                            s,
                                        ));
                                    }
                                },
                                Some((v, s)) => {
                                    let t = as_vec3(&v)
                                        .ok_or_else(|| a.wrong("dir", "view name or (x, y, z)", &v, s))?;
                                    (t, Vec3::Z)
                                }
                            };
                            let up = match a.get("up", None) {
                                None => up,
                                Some((v, s)) => {
                                    as_vec3(&v).ok_or_else(|| a.wrong("up", "(x, y, z)", &v, s))?
                                }
                            };
                            if forge_draw::View::new(toward, up).is_none() {
                                return Err(err(
                                    Problem::BadValue {
                                        func,
                                        arg: "up".into(),
                                        why: "up runs along the view direction".into(),
                                    },
                                    span,
                                ));
                            }
                            ViewKind::Base {
                                model,
                                toward,
                                up,
                                explode: a.num("explode", None)?.unwrap_or(0.0),
                            }
                        }
                    }
                    _ => {
                        let parent = self.view_of(&mut a, Some(0))?;
                        match func {
                            "projected" => ViewKind::Projected { parent },
                            "section" => {
                                let (v, s) = a.need("line", Some(1))?;
                                let pts: Option<Vec<Point2>> = match &v {
                                    Val::List(xs) => xs.iter().map(as_point).collect(),
                                    _ => None,
                                };
                                let line = pts
                                    .filter(|p| p.len() >= 2)
                                    .ok_or_else(|| a.wrong("line", "list of two or more points", &v, s))?;
                                ViewKind::Section { parent, line }
                            }
                            _ => {
                                let centre = a
                                    .point("center", Some(1))?
                                    .ok_or_else(|| err(Problem::MissingArg { func, arg: "center" }, span))?;
                                let rect = a.point("size", None)?.map(|p| (0.5 * p.x, 0.5 * p.y));
                                let r = a
                                    .num("r", None)?
                                    .or(rect.map(|r| r.0.max(r.1)))
                                    .ok_or_else(|| err(Problem::MissingArg { func, arg: "r" }, span))?;
                                ViewKind::Detail {
                                    parent,
                                    centre,
                                    r,
                                    rect,
                                }
                            }
                        }
                    }
                };
                let at = a
                    .point("at", None)?
                    .ok_or_else(|| err(Problem::MissingArg { func, arg: "at" }, span))?;
                let hide = match a.get("hide", None) {
                    None => vec![],
                    Some((Val::List(xs), s)) => xs
                        .into_iter()
                        .map(|x| match x {
                            Val::Str(p) => Ok(p),
                            v => Err(a.wrong("hide", "list of occurrence paths", &v, s)),
                        })
                        .collect::<R<_>>()?,
                    Some((v, s)) => return Err(a.wrong("hide", "list of occurrence paths", &v, s)),
                };
                let scale = a.num("scale", None)?;
                if scale.is_some_and(|s| s <= 0.0) {
                    return Err(err(
                        Problem::BadValue {
                            func,
                            arg: "scale".into(),
                            why: "a scale is more than 0".into(),
                        },
                        span,
                    ));
                }
                ItemKind::View(ViewDef {
                    kind,
                    sheet: a.item("sheet", None)?,
                    at,
                    scale,
                    hidden: a.boolean("hidden")?,
                    tangent: a.boolean("tangent")?,
                    label: a.string("label", None)?,
                    hide,
                })
            }
            "note" => {
                let text = a
                    .string("text", Some(0))?
                    .ok_or_else(|| err(Problem::MissingArg { func, arg: "text" }, span))?;
                let at = a
                    .point("at", None)?
                    .ok_or_else(|| err(Problem::MissingArg { func, arg: "at" }, span))?;
                ItemKind::Note {
                    sheet: a.item("sheet", None)?,
                    at,
                    text,
                }
            }
            "parts_list" => {
                let view = match a.get("view", Some(0)) {
                    Some((Val::Item(v), _)) if self.doc.view(&v).is_some() => Some(v),
                    Some((v, s)) => return Err(a.wrong("view", "view", &v, s)),
                    None => None,
                };
                ItemKind::PartsList {
                    sheet: a.item("sheet", None)?,
                    view,
                    at: a.point("at", None)?,
                }
            }
            _ => {
                let view = self.view_of(&mut a, Some(0))?;
                let kind = match func {
                    "dim" => AnnotKind::Linear {
                        a: a.need_ref("a", Some(1))?,
                        b: a.reference("b", Some(2))?,
                        dir: a.string("dir", None)?,
                    },
                    "angle" => AnnotKind::Angle {
                        a: a.need_ref("a", Some(1))?,
                        b: a.need_ref("b", Some(2))?,
                    },
                    "radius" => AnnotKind::Radius {
                        e: a.need_ref("edge", Some(1))?,
                    },
                    "diameter" => AnnotKind::Diameter {
                        e: a.need_ref("edge", Some(1))?,
                    },
                    "hole_note" => AnnotKind::HoleNote {
                        e: a.need_ref("edge", Some(1))?,
                    },
                    "ordinate" | "baseline" | "chain" => {
                        let base = a.need_ref("base", Some(1))?;
                        let refs = a.refs("refs", Some(2))?;
                        let dir = a.string("dir", None)?.unwrap_or_else(|| "horizontal".into());
                        if dir != "horizontal" && dir != "vertical" {
                            return Err(err(
                                Problem::BadValue {
                                    func,
                                    arg: "dir".into(),
                                    why: "horizontal or vertical".into(),
                                },
                                span,
                            ));
                        }
                        AnnotKind::Set {
                            base,
                            refs,
                            dir,
                            style: match func {
                                "ordinate" => SetStyle::Ordinate,
                                "baseline" => SetStyle::Baseline,
                                _ => SetStyle::Chain,
                            },
                        }
                    }
                    "center_mark" => AnnotKind::CenterMark {
                        e: a.need_ref("edge", Some(1))?,
                    },
                    "center_marks" => AnnotKind::CenterMarks,
                    "centerlines" => AnnotKind::Centerlines,
                    "centerline" => AnnotKind::Centerline {
                        a: a.need_ref("a", Some(1))?,
                        b: a.need_ref("b", Some(2))?,
                    },
                    "leader" => AnnotKind::Leader {
                        r: a.need_ref("to", Some(1))?,
                    },
                    "balloon" => AnnotKind::Balloon {
                        occ: a.string("occurrence", Some(1))?.ok_or_else(|| {
                            err(
                                Problem::MissingArg {
                                    func,
                                    arg: "occurrence",
                                },
                                span,
                            )
                        })?,
                    },
                    "fcf" => {
                        let to = self.attach(&mut a, func)?;
                        let row = |a: &Args,
                                   ch: &Val,
                                   tol: &Val,
                                   datums: Option<&Val>,
                                   s: Span|
                         -> R<forge_draw::gdt::Row> {
                            let bad = |why: String| {
                                err(
                                    Problem::BadValue {
                                        func,
                                        arg: "rows".into(),
                                        why,
                                    },
                                    s,
                                )
                            };
                            let Val::Str(ch) = ch else {
                                return Err(a.wrong("characteristic", "string", ch, s));
                            };
                            let ch = forge_draw::gdt::Char::from_word(ch).ok_or_else(|| {
                                bad(format!(
                                    "`{ch}`: the characteristics are {}",
                                    forge_draw::gdt::CHARS
                                        .iter()
                                        .map(|c| c.0)
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                ))
                            })?;
                            let tol = match tol {
                                Val::Str(t) => forge_draw::gdt::parse_tol(t).map_err(bad)?,
                                Val::Num(x) => forge_draw::gdt::parse_tol(&x.to_string()).map_err(bad)?,
                                v => return Err(a.wrong("tolerance", "string", v, s)),
                            };
                            let datums = match datums {
                                None => vec![],
                                Some(Val::List(xs)) => xs
                                    .iter()
                                    .map(|x| match x {
                                        Val::Str(d) => forge_draw::gdt::parse_datum(d).map_err(bad),
                                        v => Err(a.wrong("datums", "list of strings", v, s)),
                                    })
                                    .collect::<R<_>>()?,
                                Some(Val::Str(d)) => vec![forge_draw::gdt::parse_datum(d).map_err(bad)?],
                                Some(v) => return Err(a.wrong("datums", "list of strings", v, s)),
                            };
                            if datums.len() > 3 {
                                return Err(bad("a frame holds at most three datums".into()));
                            }
                            Ok(forge_draw::gdt::Row { ch, tol, datums })
                        };
                        let rows = match a.get("rows", None) {
                            Some((Val::List(rs), s)) => rs
                                .iter()
                                .map(|r| match r {
                                    Val::List(cells) if cells.len() >= 2 => {
                                        row(&a, &cells[0], &cells[1], cells.get(2), s)
                                    }
                                    v => Err(a.wrong("rows", "(characteristic, tolerance, [datums])", v, s)),
                                })
                                .collect::<R<Vec<_>>>()?,
                            Some((v, s)) => return Err(a.wrong("rows", "list of rows", &v, s)),
                            None => {
                                let (ch, s) = a.need("characteristic", Some(2))?;
                                let (tol, _) = a.need("tolerance", Some(3))?;
                                let datums = a.get("datums", Some(4)).map(|d| d.0);
                                vec![row(&a, &ch, &tol, datums.as_ref(), s)?]
                            }
                        };
                        if rows.is_empty() {
                            return Err(err(Problem::MissingArg { func, arg: "rows" }, span));
                        }
                        let between = match a.get("between", None) {
                            None => None,
                            Some((Val::List(xs), s)) => match &xs[..] {
                                [Val::Str(p), Val::Str(q)] => Some((p.clone(), q.clone())),
                                _ => {
                                    return Err(a.wrong(
                                        "between",
                                        "two point labels",
                                        &Val::List(xs.clone()),
                                        s,
                                    ));
                                }
                            },
                            Some((v, s)) => return Err(a.wrong("between", "two point labels", &v, s)),
                        };
                        AnnotKind::Fcf {
                            to,
                            rows,
                            composite: a.boolean("composite")?.unwrap_or(false),
                            all_around: a.boolean("all_around")?.unwrap_or(false),
                            between,
                        }
                    }
                    "datum" => AnnotKind::Datum {
                        to: self.attach(&mut a, func)?,
                        letter: a
                            .string("letter", Some(2))?
                            .ok_or_else(|| err(Problem::MissingArg { func, arg: "letter" }, span))?,
                    },
                    "datum_target" => {
                        let r = a.need_ref("to", Some(1))?;
                        let label = a
                            .string("label", Some(2))?
                            .ok_or_else(|| err(Problem::MissingArg { func, arg: "label" }, span))?;
                        let kind = match a.string("kind", None)?.as_deref() {
                            None | Some("point") => forge_draw::gdt::Target::Point,
                            Some("line") => forge_draw::gdt::Target::Line,
                            Some("area") => forge_draw::gdt::Target::Area,
                            Some(o) => {
                                return Err(err(
                                    Problem::BadValue {
                                        func,
                                        arg: "kind".into(),
                                        why: format!("`{o}`: point, line or area"),
                                    },
                                    span,
                                ));
                            }
                        };
                        AnnotKind::DatumTarget {
                            r,
                            label,
                            size: a.string("size", None)?.unwrap_or_default(),
                            kind,
                            area: a.num("area", None)?.unwrap_or(0.0),
                        }
                    }
                    "surface" => {
                        let r = a.need_ref("to", Some(1))?;
                        let process = match a.string("process", None)?.as_deref() {
                            None | Some("any") => forge_draw::gdt::Process::Any,
                            Some("required") => forge_draw::gdt::Process::Required,
                            Some("forbidden") => forge_draw::gdt::Process::Forbidden,
                            Some(o) => {
                                return Err(err(
                                    Problem::BadValue {
                                        func,
                                        arg: "process".into(),
                                        why: format!("`{o}`: any, required or forbidden"),
                                    },
                                    span,
                                ));
                            }
                        };
                        let value = match (a.num("ra", None)?, a.num("rz", None)?) {
                            (Some(x), _) => format!("Ra {}", forge_draw::annot::number(x, 3)),
                            (_, Some(x)) => format!("Rz {}", forge_draw::annot::number(x, 3)),
                            _ => String::new(),
                        };
                        AnnotKind::Surface {
                            r,
                            value,
                            process,
                            lay: a.string("lay", None)?.unwrap_or_default(),
                        }
                    }
                    "weld" => {
                        use forge_draw::gdt::{Weld, WeldKind, WeldSide};
                        let r = a.need_ref("to", Some(1))?;
                        let kind = match a.string("kind", Some(2))?.as_deref() {
                            None | Some("fillet") => WeldKind::Fillet,
                            Some("square") => WeldKind::Square,
                            Some("v") => WeldKind::V,
                            Some("bevel") => WeldKind::Bevel,
                            Some("plug") => WeldKind::Plug,
                            Some(o) => {
                                return Err(err(
                                    Problem::BadValue {
                                        func,
                                        arg: "kind".into(),
                                        why: format!("`{o}`: fillet, square, v, bevel or plug"),
                                    },
                                    span,
                                ));
                            }
                        };
                        let side = match a.string("side", None)?.as_deref() {
                            None | Some("arrow") => WeldSide::Arrow,
                            Some("other") => WeldSide::Other,
                            Some("both") => WeldSide::Both,
                            Some(o) => {
                                return Err(err(
                                    Problem::BadValue {
                                        func,
                                        arg: "side".into(),
                                        why: format!("`{o}`: arrow, other or both"),
                                    },
                                    span,
                                ));
                            }
                        };
                        let size = match a.get("size", None) {
                            None => String::new(),
                            Some((Val::Num(x), _)) => forge_draw::annot::number(x, 2),
                            Some((Val::Str(x), _)) => x,
                            Some((v, s)) => return Err(a.wrong("size", "number or string", &v, s)),
                        };
                        AnnotKind::Weld {
                            r,
                            weld: Weld {
                                kind,
                                side,
                                size,
                                length: a.string("length", None)?.unwrap_or_default(),
                                all_around: a.boolean("all_around")?.unwrap_or(false),
                                field: a.boolean("field")?.unwrap_or(false),
                                tail: a.string("tail", None)?.unwrap_or_default(),
                            },
                        }
                    }
                    "edge" => AnnotKind::Edge {
                        r: a.need_ref("to", Some(1))?,
                        upper: a.string("upper", Some(2))?.unwrap_or_else(|| "+0.3".into()),
                        lower: a.string("lower", None)?.unwrap_or_default(),
                    },
                    _ => AnnotKind::AutoBalloons,
                };
                let dir_ok = |d: &Option<String>| {
                    d.as_deref()
                        .is_none_or(|d| ["horizontal", "vertical", "aligned"].contains(&d))
                };
                if let AnnotKind::Linear { dir, .. } = &kind
                    && !dir_ok(dir)
                {
                    return Err(err(
                        Problem::BadValue {
                            func,
                            arg: "dir".into(),
                            why: "horizontal, vertical or aligned".into(),
                        },
                        span,
                    ));
                }
                let tol = self.tolerance(&mut a, func, span)?;
                let places = match a.num("places", None)? {
                    None => None,
                    Some(p) if (0.0..=6.0).contains(&p) && p.fract() == 0.0 => Some(p as usize),
                    Some(_) => {
                        return Err(err(
                            Problem::BadValue {
                                func,
                                arg: "places".into(),
                                why: "0 to 6 decimal places".into(),
                            },
                            span,
                        ));
                    }
                };
                ItemKind::Annot(AnnotDef {
                    view,
                    kind,
                    at: a.point("at", None)?,
                    text: a.string("text", None)?,
                    prefix: a.string("prefix", None)?.unwrap_or_default(),
                    suffix: a.string("suffix", None)?.unwrap_or_default(),
                    tol,
                    places,
                })
            }
        };
        a.done()?;
        self.doc.items.push(Item {
            id: id.to_string(),
            span,
            kind,
        });
        Ok(Val::Item(id.to_string()))
    }
}

fn v_span(a: &Args, k: &str) -> Span {
    a.kw.iter().find(|x| x.0 == k).map_or(a.span, |x| x.2)
}

/// Reads a drawing's source: every item it can, every problem typed.
pub fn read(src: &str) -> Doc {
    let prog = match syntax::parse(src) {
        Ok(p) => p,
        Err(e) => {
            let span = e.span().unwrap_or(Span::new(0, 0));
            return Doc {
                errors: vec![err(Problem::Syntax { message: e.text() }, span)],
                ..Doc::default()
            };
        }
    };
    let mut r = Reader {
        env: HashMap::new(),
        doc: Doc::default(),
        src,
    };
    if !declared(&prog) {
        r.doc.errors.push(err(
            Problem::NotADrawing,
            prog.first().map_or(Span::new(0, 0), |s| s.span),
        ));
        return r.doc;
    }
    for s in &prog[1..] {
        let res = match &s.kind {
            StmtKind::Shared { name, value } => r.expr(value).map(|v| {
                if let Val::Num(x) = v {
                    r.doc.params.push((name.clone(), x));
                }
                r.env.insert(name.clone(), v);
            }),
            StmtKind::Assign { name, value } => {
                if r.env.contains_key(name) {
                    Err(err(Problem::Duplicate { id: name.clone() }, s.span))
                } else {
                    match &value.kind {
                        ExprKind::Call(f, args) => match &f.kind {
                            ExprKind::Name(func) => r.call(func, args, value.span, Some(name)).map(|v| {
                                r.env.insert(name.clone(), v);
                            }),
                            _ => Err(err(
                                Problem::UnknownFunction {
                                    name: src[f.span.lo as usize..f.span.hi as usize].to_string(),
                                },
                                f.span,
                            )),
                        },
                        _ => r.expr(value).map(|v| {
                            r.env.insert(name.clone(), v);
                        }),
                    }
                }
            }
            _ => Err(err(
                Problem::Syntax {
                    message: "a drawing holds `name = item(..)` lines and `shared` params".into(),
                },
                s.span,
            )),
        };
        if let Err(e) = res {
            r.doc.errors.push(e);
        }
    }
    r.doc
}

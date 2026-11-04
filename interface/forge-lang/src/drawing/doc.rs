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
    /// The thing asked for does not apply here (a balloon on a part drawing).
    NotHere {
        item: String,
        why: String,
    },
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
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemKind {
    Model {
        path: String,
        asm: bool,
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
                ItemKind::Model {
                    path,
                    asm: func == "assembly",
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
                ] {
                    if let Some(v) = a.get(k, None) {
                        *slot = match v.0 {
                            Val::Str(s) => s,
                            v => return Err(a.wrong(k, "string", &v, v_span(&a, k))),
                        };
                    }
                }
                ItemKind::Sheet(SheetDef {
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
                ItemKind::Annot(AnnotDef {
                    view,
                    kind,
                    at: a.point("at", None)?,
                    text: a.string("text", None)?,
                    prefix: a.string("prefix", None)?.unwrap_or_default(),
                    suffix: a.string("suffix", None)?.unwrap_or_default(),
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

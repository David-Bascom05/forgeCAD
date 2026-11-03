//! The registration hook for the wired features: holes and threads (holes.rs), patterns
//! (patterns.rs), work features (work.rs), bodies (bodies.rs), text and imports (media.rs) and a
//! sketch's projections (project.rs). builtins.rs asks here for the names it does not know.

use crate::builtins::{Args, num_of};
use crate::eval::{Eval, Feat, R, Value};
use crate::feat_error::{Because, FeatError};
use crate::syntax::{Arg, Span};
use crate::values::{Dim, Num, Ty};
use forge_math::{Frame, Point3, UnitVec3};
use forge_model::{Datum, Layout, Ref, Step};
use std::rc::Rc;

/// The wired built-ins, called by name.
pub const NAMES: &[&str] = &[
    "hole",
    "thread",
    "pattern",
    "lin",
    "circ",
    "mirror",
    "along",
    "places",
    "workplane",
    "workaxis",
    "workpoint",
    "ucs",
    "union",
    "subtract",
    "combine",
    "split",
    "splitface",
    "copybody",
    "movebody",
    "slide",
    "turn",
    "place",
    "text",
    "import",
    "project",
    "entity",
    "delete",
    "replace",
    "moveface",
];

/// Words: `full` (a thread's whole length), `all` (every body).
pub const WORDS: &[&str] = &["full", "all"];

/// A wired value.
#[derive(Clone, Debug)]
pub enum W {
    /// A work feature: what it came to, and its place in the history.
    Work {
        id: String,
        idx: usize,
        datum: Datum,
    },
    /// A pattern's layout, its references resolved, and the dimensions it took.
    Layout(Layout, Vec<crate::eval::Dimension>),
    /// One motion of a body.
    Motion(Step),
    Word(&'static str),
}

impl W {
    pub fn ty(&self) -> Ty {
        match self {
            W::Work { datum, .. } => match datum {
                Datum::Point(_) => Ty::Point,
                Datum::Axis(_) => Ty::Axis,
                Datum::Plane(_) | Datum::Frame(_) => Ty::Plane,
            },
            W::Layout(..) => Ty::Layout,
            W::Motion(_) => Ty::Motion,
            W::Word(_) => Ty::Word,
        }
    }

    pub fn point(&self) -> Option<Point3> {
        match self {
            W::Work {
                datum: Datum::Point(p),
                ..
            } => Some(*p),
            W::Work {
                datum: Datum::Frame(f),
                ..
            } => Some(f.origin),
            _ => None,
        }
    }

    pub fn axis(&self) -> Option<(Point3, UnitVec3)> {
        match self {
            W::Work {
                datum: Datum::Axis(l),
                ..
            } => Some((l.origin, l.dir)),
            _ => None,
        }
    }

    pub fn plane(&self) -> Option<Frame> {
        match self {
            W::Work {
                datum: Datum::Plane(f) | Datum::Frame(f),
                ..
            } => Some(*f),
            _ => None,
        }
    }

    /// The reference a work feature is.
    pub fn work_ref(&self) -> Option<Ref> {
        match self {
            W::Work { idx, .. } => Some(Ref {
                feature: *idx,
                name: None,
            }),
            _ => None,
        }
    }
}

pub fn wired(w: W) -> Value {
    Value::Wired(Rc::new(w))
}

pub fn constant(name: &str) -> Option<Value> {
    if let Some(n) = NAMES.iter().find(|n| **n == name) {
        return Some(Value::Builtin(n));
    }
    WORDS.iter().find(|n| **n == name).map(|n| wired(W::Word(n)))
}

/// Calls that take a built-in's name with another meaning: `intersect(target, tools: [..])`
/// joins bodies (with `tools`), where `intersect(profile, extent)` extrudes.
pub fn pre_call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> Option<R<Value>> {
    let tools = args
        .iter()
        .any(|a| a.name.as_ref().is_some_and(|n| n.0 == "tools"));
    (name == "intersect" && tools).then(|| crate::bodies::combine(ev, "intersect", args, span))
}

pub fn call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    match name {
        "hole" => crate::holes::hole(ev, args, span),
        "thread" => crate::holes::thread(ev, args, span),
        "pattern" => crate::patterns::pattern(ev, args, span),
        "lin" | "circ" | "mirror" | "along" | "places" => crate::patterns::layout(ev, name, args, span),
        "workplane" | "workaxis" | "workpoint" | "ucs" => crate::work::work(ev, name, args, span),
        "union" | "subtract" | "combine" => crate::bodies::combine(ev, name, args, span),
        "split" | "splitface" => crate::bodies::split(ev, name, args, span),
        "copybody" | "movebody" => crate::bodies::move_body(ev, name, args, span),
        "slide" | "turn" | "place" => crate::bodies::motion(ev, name, args, span),
        "text" => crate::media::text(ev, args, span),
        "import" => crate::media::import(ev, args, span),
        "project" => crate::project::project(ev, args, span),
        "entity" => crate::media::entity(ev, args, span),
        "delete" | "replace" | "moveface" => crate::direct::direct(ev, name, args, span),
        _ => Err(crate::error::Error::UnknownName {
            span,
            name: name.into(),
        }),
    }
}

/// A member of a feature value that the wired features add (a hole's `wall`, a thread's
/// `flanks`, an import's entities ..); None: not one of theirs.
pub fn feature_member(ev: &mut Eval, f: &Rc<Feat>, name: &str, span: Span) -> Option<R<Value>> {
    match f.kind {
        "hole" => crate::holes::hole_member(ev, f, name, span),
        "thread" => crate::holes::thread_member(ev, f, name, span),
        "text" => crate::media::text_member(ev, f, name, span),
        _ => None,
    }
}

/// A member of a wired value: a work feature's plane, axis, origin, normal.
pub fn member(_ev: &mut Eval, w: &Rc<W>, name: &str, _span: Span) -> Option<R<Value>> {
    let plane = |f: Frame| {
        Value::Plane(Rc::new(crate::eval::Plane {
            frame: f,
            world: None,
        }))
    };
    Some(Ok(match (name, &**w) {
        ("plane", _) if w.plane().is_some() => plane(w.plane()?),
        ("normal", _) if w.plane().is_some() => Value::Dir(w.plane()?.z().get()),
        ("origin", _) if w.plane().is_some() => Value::Point(w.plane()?.origin),
        ("axis", _) if w.axis().is_some() => {
            let (o, d) = w.axis()?;
            Value::Axis(o, d)
        }
        ("dir", _) if w.axis().is_some() => Value::Dir(w.axis()?.1.get()),
        ("point" | "origin", _) if w.point().is_some() => Value::Point(w.point()?),
        _ => return None,
    }))
}

// ---- shared helpers ---------------------------------------------------------------------------

/// A word argument among the allowed ones.
pub fn word(a: &Args, k: &'static str, allowed: &'static [&'static str]) -> R<Option<&'static str>> {
    match a.opt(k) {
        None => Ok(None),
        Some(Value::Str(s)) => match allowed.iter().find(|w| ***w == *s) {
            Some(w) => Ok(Some(w)),
            None => Err(FeatError::UnknownWord {
                span: a.span(k),
                arg: k,
                word: s.to_string(),
                allowed,
            }
            .into()),
        },
        Some(o) => Err(crate::error::Error::WrongType {
            span: a.span(k),
            what: k,
            want: Ty::Str,
            found: o.ty(),
        }),
    }
}

/// Exactly one of `forms` given: its name.
pub fn one_form(a: &Args, func: &'static str, forms: &'static [&'static str], span: Span) -> R<&'static str> {
    let given: Vec<&'static str> = forms.iter().copied().filter(|f| a.opt(f).is_some()).collect();
    match given[..] {
        [one] => Ok(one),
        _ => Err(FeatError::Forms {
            span,
            func,
            forms,
            given: given.len(),
        }
        .into()),
    }
}

/// An option that only applies to another form, if given: an error, never dropped.
pub fn only_with(a: &Args, func: &'static str, arg: &'static str, ok: bool, because: Because) -> R<()> {
    if a.opt(arg).is_some() && !ok {
        return Err(FeatError::NotApplicable {
            span: a.span(arg),
            func,
            arg,
            because,
        }
        .into());
    }
    Ok(())
}

pub fn conflict(a: &Args, func: &'static str, x: &'static str, y: &'static str) -> R<()> {
    if a.opt(x).is_some() && a.opt(y).is_some() {
        return Err(FeatError::Conflict {
            span: a.span(y),
            func,
            a: x,
            b: y,
        }
        .into());
    }
    Ok(())
}

/// A list's items, a map's values in key order, or one value.
pub fn items(v: Value) -> Vec<Value> {
    match v {
        Value::List(xs) => xs.to_vec(),
        Value::Map(m) => m.iter().map(|x| x.1.clone()).collect(),
        v => vec![v],
    }
}

/// A Count argument as a whole number.
pub fn count(a: &Args, k: &'static str) -> R<Option<(usize, Num)>> {
    match a.opt(k) {
        None => Ok(None),
        Some(v) => {
            let n = num_of(v, &[Dim::Count], a.span(k), k)?;
            if n.v < 1.0 {
                return Err(crate::error::Error::BadValue {
                    span: a.span(k),
                    what: k,
                    value: n.v,
                });
            }
            Ok(Some((n.v.round() as usize, n)))
        }
    }
}

/// The feature value a statement made, without the profile-specific parts.
pub fn feat(id: String, kind: &'static str, idx: usize, hid: u64, axis: Option<(Point3, UnitVec3)>) -> Value {
    Value::Feature(Rc::new(Feat {
        id,
        kind,
        idx,
        hid,
        shape: None,
        axis,
        prof: None,
    }))
}

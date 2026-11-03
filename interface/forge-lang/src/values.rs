//! Values: numbers with a dimension and where they came from (a param that drives them, or the
//! params an expression derived them from), and the geometric values anchors resolve to.

use crate::error::{Error, ParamKey};
use crate::syntax::Span;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dim {
    Length,
    Angle,
    Count,
    Ratio,
    Area,
    Volume,
    /// The structural constants 0, 1 and 2: they take the other operand's dimension.
    Any,
}

impl Dim {
    pub fn name(self) -> &'static str {
        match self {
            Dim::Length => "Length",
            Dim::Angle => "Angle",
            Dim::Count => "Count",
            Dim::Ratio => "Ratio",
            Dim::Area => "Area",
            Dim::Volume => "Volume",
            Dim::Any => "Any",
        }
    }
    pub fn from_name(s: &str) -> Option<Dim> {
        Some(match s {
            "Length" => Dim::Length,
            "Angle" => Dim::Angle,
            "Count" => Dim::Count,
            "Ratio" => Dim::Ratio,
            "Area" => Dim::Area,
            "Volume" => Dim::Volume,
            "Any" => Dim::Any,
            _ => return None,
        })
    }
    fn pow(self) -> Option<i32> {
        match self {
            Dim::Length => Some(1),
            Dim::Area => Some(2),
            Dim::Volume => Some(3),
            _ => None,
        }
    }
    fn of_pow(p: i32) -> Option<Dim> {
        match p {
            1 => Some(Dim::Length),
            2 => Some(Dim::Area),
            3 => Some(Dim::Volume),
            _ => None,
        }
    }
    fn scalar(self) -> bool {
        matches!(self, Dim::Count | Dim::Ratio)
    }
}

/// Where a number came from. Driving and driven are disjoint: a number is exactly one param
/// (an editable handle, edited by `SetParam` on it), or derived from params by an expression
/// (no handle: edit its inputs), or a structural constant.
#[derive(Clone, Debug, PartialEq)]
pub enum Src {
    Const,
    Driving(ParamKey),
    Driven(BTreeSet<ParamKey>),
}

impl Src {
    pub fn params(&self) -> BTreeSet<ParamKey> {
        match self {
            Src::Const => BTreeSet::new(),
            Src::Driving(p) => [p.clone()].into(),
            Src::Driven(s) => s.clone(),
        }
    }
    fn join(a: &Src, b: &Src) -> Src {
        let mut s = a.params();
        s.extend(b.params());
        if s.is_empty() { Src::Const } else { Src::Driven(s) }
    }
    fn derived(a: &Src) -> Src {
        let s = a.params();
        if s.is_empty() { Src::Const } else { Src::Driven(s) }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Num {
    pub v: f64,
    pub dim: Dim,
    pub src: Src,
}

impl Num {
    pub fn konst(v: f64, dim: Dim) -> Num {
        Num {
            v,
            dim,
            src: Src::Const,
        }
    }
    pub fn is_whole(&self) -> bool {
        (self.v - self.v.round()).abs() < 1e-9
    }
}

type R<T> = Result<T, Error>;

fn mismatch(op: &'static str, a: &Num, b: &Num, span: Span) -> Error {
    Error::DimMismatch {
        span,
        op,
        left: a.dim,
        right: b.dim,
    }
}

pub fn add(a: &Num, b: &Num, sub: bool, span: Span) -> R<Num> {
    let op = if sub { "-" } else { "+" };
    let dim = match (a.dim, b.dim) {
        (Dim::Any, d) | (d, Dim::Any) => d,
        (x, y) if x == y => x,
        _ => return Err(mismatch(op, a, b, span)),
    };
    let v = if sub { a.v - b.v } else { a.v + b.v };
    Ok(Num {
        v,
        dim,
        src: Src::join(&a.src, &b.src),
    })
}

pub fn mul(a: &Num, b: &Num, span: Span) -> R<Num> {
    let dim = match (a.dim, b.dim) {
        (Dim::Any, d) | (d, Dim::Any) => d,
        (Dim::Count, Dim::Count) => Dim::Count,
        (x, y) if x.scalar() && y.scalar() => Dim::Ratio,
        (x, d) | (d, x) if x.scalar() => d,
        (x, y) => match (x.pow(), y.pow()) {
            (Some(p), Some(q)) if p + q <= 3 => Dim::of_pow(p + q).expect("1..3"),
            _ => return Err(mismatch("*", a, b, span)),
        },
    };
    Ok(Num {
        v: a.v * b.v,
        dim,
        src: Src::join(&a.src, &b.src),
    })
}

pub fn div(a: &Num, b: &Num, span: Span) -> R<Num> {
    if b.v == 0.0 {
        return Err(Error::BadValue {
            span,
            what: "a divisor",
            value: 0.0,
        });
    }
    let v = a.v / b.v;
    let dim = match (a.dim, b.dim) {
        (Dim::Count, Dim::Any | Dim::Count) if (v - v.round()).abs() < 1e-9 && b.dim == Dim::Any => {
            Dim::Count
        }
        (Dim::Count, y) if y.scalar() || y == Dim::Any => Dim::Ratio,
        (Dim::Count, _) => return Err(mismatch("/", a, b, span)),
        (d, Dim::Any) => d,
        (d, y) if y.scalar() => d,
        // a bare number over a count or ratio is a ratio; over a size, nothing this language has
        (Dim::Any, y) if y.scalar() => Dim::Ratio,
        (Dim::Any, _) => return Err(mismatch("/", a, b, span)),
        (x, y) if x == y => Dim::Ratio,
        (x, y) => match (x.pow(), y.pow()) {
            (Some(p), Some(q)) if p > q => Dim::of_pow(p - q).expect("1..2"),
            _ => return Err(mismatch("/", a, b, span)),
        },
    };
    Ok(Num {
        v,
        dim,
        src: Src::join(&a.src, &b.src),
    })
}

pub fn pow(a: &Num, b: &Num, span: Span) -> R<Num> {
    if !matches!(b.dim, Dim::Any | Dim::Count) || !b.is_whole() {
        return Err(Error::NotWhole { span, value: b.v });
    }
    let n = b.v.round() as i32;
    let dim = match a.dim {
        d @ (Dim::Any | Dim::Count | Dim::Ratio) => d,
        d => match d.pow().and_then(|p| Dim::of_pow(p * n)) {
            Some(x) => x,
            None => return Err(mismatch("^", a, b, span)),
        },
    };
    Ok(Num {
        v: a.v.powi(n),
        dim,
        src: Src::join(&a.src, &b.src),
    })
}

pub fn neg(a: &Num) -> Num {
    Num {
        v: -a.v,
        dim: a.dim,
        src: Src::derived(&a.src),
    }
}

pub fn sqrt(a: &Num, span: Span) -> R<Num> {
    if a.v < 0.0 {
        return Err(Error::BadValue {
            span,
            what: "the argument of sqrt",
            value: a.v,
        });
    }
    let dim = match a.dim {
        Dim::Area => Dim::Length,
        Dim::Any => Dim::Any,
        Dim::Ratio | Dim::Count => Dim::Ratio,
        d => {
            return Err(Error::WrongDim {
                span,
                what: "sqrt",
                want: Dim::Area,
                found: d,
            });
        }
    };
    Ok(Num {
        v: a.v.sqrt(),
        dim,
        src: Src::derived(&a.src),
    })
}

/// A number derived from `a` by a function (sin, abs ..).
pub fn derive(a: &Num, v: f64, dim: Dim) -> Num {
    Num {
        v,
        dim,
        src: Src::derived(&a.src),
    }
}

pub fn join_src(a: &Src, b: &Src) -> Src {
    Src::join(a, b)
}

/// Value types, for type errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    Num,
    Bool,
    Str,
    List,
    Map,
    Tuple,
    Vec2,
    Function,
    Word,
    Dir,
    Line,
    Plane,
    Axis,
    Point,
    Sketch,
    Entity,
    Disc,
    Feature,
    Faces,
    Edges,
    Extent,
    Layout,
    Motion,
    Unresolved,
    Sheet,
    SheetEdge,
    Unfold,
    BendRule,
    Scaled,
    Stock,
    Shop,
    Relief,
    CornerKind,
}

pub fn fmt(x: f64) -> String {
    if x.abs() < 1e-12 {
        return "0".into();
    }
    let s = format!("{x:.12}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

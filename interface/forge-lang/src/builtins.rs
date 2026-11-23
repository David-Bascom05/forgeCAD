//! The built-in functions: math, sketch entities and constraints, features, anchors and
//! selections. Each checks its arguments' types and dimensions and records the dimensions a
//! feature takes, with where each came from.

use crate::error::{EntKind, Error, Role};
use crate::eval::{Dimension, Ent, Eval, ExtentV, Faces, Feat, Plane, R, Shape, Value};
use crate::sketch::{Item, P2, Sk};
use crate::syntax::{Arg, Span};
use crate::values::{self, Dim, Num, Src, Ty};
use forge_agent::select;
use forge_geom::{Curve, Surface};
use forge_math::{Frame, Point2, Point3, UnitVec3, Vec2, Vec3, dmath};
use forge_model::{Combine, Extent, Feature, Target, Toward, Turn};
use forge_ops::{Axis2, ChamferCorner, ChamferSize};
use forge_sketch::{Constraint, EntityId};
use forge_topo::{Body, EdgeId, FaceId, Name};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub const BUILTINS: &[&str] = &[
    "use",
    "sqrt",
    "div",
    "mod",
    "sin",
    "cos",
    "tan",
    "asin",
    "acos",
    "atan",
    "atan2",
    "abs",
    "min",
    "max",
    "count",
    "ratio",
    "len",
    "polar",
    "pt",
    "near",
    "line",
    "arc",
    "circle",
    "ellipse",
    "earc",
    "rect",
    "slot",
    "hull",
    "disc",
    "poly",
    "spline",
    "tarc",
    "close",
    "tangent",
    "coincident",
    "horizontal",
    "vertical",
    "parallel",
    "perpendicular",
    "equal",
    "midpoint",
    "distance",
    "radius",
    "angle",
    "fix",
    "concentric",
    "collinear",
    "symmetric",
    "hdistance",
    "vdistance",
    "extrude",
    "cut",
    "revolve",
    "fillet",
    "chamfer",
    "shell",
    "loft",
    "offset",
    "rotate",
    "edges",
    "faces",
    "shift",
    "hline",
    "vline",
    "meet",
    "mid",
    "plane",
    "properties",
];

/// A built-in function's name, this module's or another's (shapes).
pub fn builtin(name: &str) -> Option<&'static str> {
    names_get(name).and_then(|n| n.1)
}

/// One place of `NAMES`: a name ("" for none), its built-in (Some, `builtin`'s) and whether
/// `constant` knows it.
type NameSlot = (&'static str, Option<&'static str>, bool);

/// Places in `NAMES`: a power of two, at least twice the names.
const NAME_SLOTS: usize = 1024;

/// FNV-1a of a name, as a place in `NAMES`.
const fn name_slot(s: &[u8]) -> usize {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut i = 0;
    while i < s.len() {
        h = (h ^ s[i] as u64).wrapping_mul(0x0100_0000_01b3);
        i += 1;
    }
    (h ^ (h >> 32)) as usize & (NAME_SLOTS - 1)
}

const fn same(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Every built-in's name and every name `constant` knows, in one open-addressed table worked
/// out when the crate compiles: every name looked up or bound asks, so a table, not a walk of
/// the lists, and nothing to build at run time.
#[allow(clippy::large_const_arrays)]
const NAMES: [NameSlot; NAME_SLOTS] = {
    let mut t: [NameSlot; NAME_SLOTS] = [("", None, false); NAME_SLOTS];
    let builtins: [&[&str]; 2] = [BUILTINS, crate::shapes::NAMES];
    let constants: [&[&str]; 7] = [
        &CONSTANTS,
        &MORE,
        crate::sheet::BUILTINS,
        crate::thermo::BUILTINS,
        crate::weld::BUILTINS,
        crate::wired::NAMES,
        crate::sheet::CONSTANTS,
    ];
    let mut used = 0;
    let mut pass = 0;
    while pass < 2 {
        let lists: &[&[&str]] = if pass == 0 { &builtins } else { &constants };
        let mut l = 0;
        while l < lists.len() {
            let mut k = 0;
            while k < lists[l].len() {
                let n = lists[l][k];
                let mut i = name_slot(n.as_bytes());
                // the first entry of a name stands (a built-in before a constant)
                while !t[i].0.is_empty() && !same(t[i].0.as_bytes(), n.as_bytes()) {
                    i = (i + 1) & (NAME_SLOTS - 1);
                }
                if t[i].0.is_empty() {
                    used += 1;
                    t[i] = (n, if pass == 0 { Some(n) } else { None }, pass == 1);
                } else if pass == 1 {
                    t[i].2 = true;
                }
                k += 1;
            }
            l += 1;
        }
        pass += 1;
    }
    assert!(2 * used <= NAME_SLOTS);
    t
};

/// `name`'s place in `NAMES`, if it has one.
#[inline]
fn names_get(name: &str) -> Option<&'static NameSlot> {
    let mut i = name_slot(name.as_bytes());
    loop {
        let e = &NAMES[i];
        if e.0.is_empty() {
            return None;
        }
        if e.0 == name {
            return Some(e);
        }
        i = (i + 1) & (NAME_SLOTS - 1);
    }
}

/// `constant`'s own names (the lists it also answers for are named in `names`).
const CONSTANTS: [&str; 10] = ["pi", "X", "Y", "Z", "xy", "xz", "yz", "thru", "thru_all", "next"];

const MORE: [&str; 5] = ["axis", "sym", "two", "upto", "intersect"];

pub fn constant(name: &str) -> Option<Value> {
    if !names_get(name).is_some_and(|n| n.2) {
        return None;
    }
    let world = |n: &'static str, f: Frame| {
        Some(Value::Plane(Rc::new(Plane {
            frame: f,
            world: Some(n),
        })))
    };
    match name {
        "pi" => Some(Value::Num(Num::konst(std::f64::consts::PI, Dim::Ratio))),
        "X" => Some(Value::WorldDir(Vec3::X)),
        "Y" => Some(Value::WorldDir(Vec3::Y)),
        "Z" => Some(Value::WorldDir(Vec3::Z)),
        "xy" => world("xy", Frame::WORLD),
        "xz" => world(
            "xz",
            Frame::from_axes(Point3::ORIGIN, Vec3::X, Vec3::Z, -Vec3::Y).expect("orthonormal"),
        ),
        "yz" => world(
            "yz",
            Frame::from_axes(Point3::ORIGIN, Vec3::Y, Vec3::Z, Vec3::X).expect("orthonormal"),
        ),
        "thru" => Some(Value::Extent(ExtentV::Thru)),
        "thru_all" => Some(Value::Extent(ExtentV::ThruAll)),
        "next" => Some(Value::Extent(ExtentV::Next)),
        n if MORE.contains(&n) => MORE.iter().find(|m| **m == n).map(|m| Value::Builtin(m)),
        n if crate::sheet::BUILTINS.contains(&n) => crate::sheet::BUILTINS
            .iter()
            .find(|m| **m == n)
            .map(|m| Value::Builtin(m)),
        n if crate::thermo::BUILTINS.contains(&n) => crate::thermo::BUILTINS
            .iter()
            .find(|m| **m == n)
            .map(|m| Value::Builtin(m)),
        n if crate::weld::BUILTINS.contains(&n) => crate::weld::BUILTINS
            .iter()
            .find(|m| **m == n)
            .map(|m| Value::Builtin(m)),
        n => crate::wired::constant(n).or_else(|| crate::sheet::constant(n)),
    }
}

/// A call's arguments bound to names: positional ones in order, then keywords.
pub struct Args {
    func: &'static str,
    call: Span,
    /// Each argument given, by name (a handful: a list is quicker than a map).
    vals: Vec<(&'static str, (Value, Span))>,
}

impl Args {
    pub fn bind(
        ev: &mut Eval,
        func: &'static str,
        names: &[&'static str],
        args: &[Arg],
        call: Span,
    ) -> R<Args> {
        let mut vals: Vec<(&'static str, (Value, Span))> = Vec::with_capacity(args.len());
        for (k, a) in args.iter().enumerate() {
            let key = match &a.name {
                Some((n, sp)) => *names.iter().find(|x| *x == n).ok_or_else(|| Error::UnknownArg {
                    span: *sp,
                    func,
                    arg: n.clone(),
                })?,
                None => *names.get(k).ok_or_else(|| Error::TooManyArgs {
                    span: a.value.span,
                    func: func.into(),
                    max: names.len(),
                })?,
            };
            let v = ev.expr(&a.value)?;
            if let Some(sk) = ev.sketch_ref() {
                crate::eval::collect_keys(&v, &mut sk.borrow_mut().params);
            }
            crate::eval::collect_keys(&v, &mut ev.reads);
            // an empty face set is unknown only after an earlier failure (or in `check`); on a
            // part built so far without one, it is a selection that found nothing
            let known_empty = ev.b.building && ev.errors.is_empty() && !unresolved_in(&v);
            ev.poisoned |= unknown_in(&v) && !known_empty;
            // (given twice, the last is the one)
            match vals.iter_mut().find(|x| x.0 == key) {
                Some(x) => x.1 = (v, a.value.span),
                None => vals.push((key, (v, a.value.span))),
            }
        }
        Ok(Args { func, call, vals })
    }
    /// Whether an argument stands for something an earlier error left unknown.
    pub fn poisoned(&self) -> bool {
        self.vals.iter().any(|v| matches!(v.1.0, Value::Unresolved))
    }
    fn get(&self, k: &str) -> Option<&(Value, Span)> {
        self.vals.iter().find(|x| x.0 == k).map(|x| &x.1)
    }
    pub fn opt(&self, k: &str) -> Option<Value> {
        self.get(k).map(|v| v.0.clone())
    }
    pub fn req(&self, k: &'static str) -> R<Value> {
        self.opt(k).ok_or(Error::MissingArg {
            span: self.call,
            func: self.func,
            arg: k,
        })
    }
    pub fn span(&self, k: &str) -> Span {
        self.get(k).map_or(self.call, |v| v.1)
    }
    pub(crate) fn num(&self, k: &'static str, dims: &[Dim]) -> R<Option<Num>> {
        match self.opt(k) {
            None => Ok(None),
            Some(v) => num_of(v, dims, self.span(k), k).map(Some),
        }
    }
    pub(crate) fn req_num(&self, k: &'static str, dims: &[Dim]) -> R<Num> {
        num_of(self.req(k)?, dims, self.span(k), k)
    }
    pub(crate) fn flag(&self, k: &'static str) -> R<bool> {
        match self.opt(k) {
            None => Ok(false),
            Some(Value::Bool(b)) => Ok(b),
            Some(o) => Err(Error::WrongType {
                span: self.span(k),
                what: k,
                want: Ty::Bool,
                found: o.ty(),
            }),
        }
    }
}

/// Whether a value is, or holds, `Unresolved`.
fn unresolved_in(v: &Value) -> bool {
    match v {
        Value::Unresolved => true,
        Value::List(xs) | Value::Tuple(xs) => xs.iter().any(unresolved_in),
        Value::Map(m) => m.iter().any(|x| unresolved_in(&x.1)),
        _ => false,
    }
}

/// Whether a value is, or holds, something unknown.
pub fn unknown_in(v: &Value) -> bool {
    match v {
        Value::Unresolved => true,
        Value::Faces(f) => f.names.is_empty() && f.plane.is_none(),
        Value::List(xs) | Value::Tuple(xs) => xs.iter().any(unknown_in),
        Value::Map(m) => m.iter().any(|x| unknown_in(&x.1)),
        _ => false,
    }
}

pub fn num_of(v: Value, dims: &[Dim], span: Span, what: &'static str) -> R<Num> {
    let Value::Num(n) = v else {
        return Err(Error::WrongType {
            span,
            what,
            want: Ty::Num,
            found: v.ty(),
        });
    };
    if n.dim != Dim::Any && !dims.contains(&n.dim) {
        return Err(Error::WrongDim {
            span,
            what,
            want: dims[0],
            found: n.dim,
        });
    }
    if dims == [Dim::Count] && !n.is_whole() {
        return Err(Error::NotWhole { span, value: n.v });
    }
    Ok(n)
}

pub(crate) fn positive(n: &Num, span: Span, what: &'static str) -> R<f64> {
    if n.v <= 0.0 || !n.v.is_finite() {
        return Err(Error::BadValue {
            span,
            what,
            value: n.v,
        });
    }
    Ok(n.v)
}

pub(crate) fn dim(what: &'static str, n: &Num) -> Dimension {
    Dimension { what, num: n.clone() }
}

pub(crate) const LEN: &[Dim] = &[Dim::Length];
pub(crate) const ANG: &[Dim] = &[Dim::Angle];
/// A length (an extrude's extent) or an angle (a revolve's).
pub(crate) const LEN_ANG: &[Dim] = &[Dim::Length, Dim::Angle];

/// What makes a spline one on control vertices of a set degree, knots and weights.
const CONTROL: [&str; 3] = ["degree", "knots", "weights"];

/// A through-spline's end slopes as given.
const SLOPES: [&str; 2] = ["start_slope", "end_slope"];

/// A slope `(dx, dy)`: the curve's rate of change at its end, per unit along it.
fn slope_arg(a: &Args, k: &'static str) -> R<Option<Vec2>> {
    let Some(v) = a.opt(k) else {
        return Ok(None);
    };
    let xy = match v {
        Value::Tuple(xy) if xy.len() == 2 => xy,
        o => {
            return Err(Error::WrongType {
                span: a.span(k),
                what: k,
                want: Ty::Vec2,
                found: o.ty(),
            });
        }
    };
    let n = |x: &Value| num_of(x.clone(), &[Dim::Ratio, Dim::Count, Dim::Length], a.span(k), k).map(|n| n.v);
    let d = Vec2::new(n(&xy[0])?, n(&xy[1])?);
    if d.length() == 0.0 || !d.x.is_finite() || !d.y.is_finite() {
        return Err(Error::BadValue {
            span: a.span(k),
            what: "a spline's end slope (0, 0)",
            value: 0.0,
        });
    }
    Ok(Some(d))
}

/// A control spline's weights, if given.
type Weights = Option<Vec<f64>>;

/// A control-vertex spline's `degree` (cubic, or less with too few points), `knots` (as many as
/// the points and the degree and one more, never falling) and `weights` (one a point, positive).
fn control_args(a: &Args, n: usize) -> R<(usize, Option<Vec<f64>>, Weights)> {
    let degree = match a.num("degree", &[Dim::Count])? {
        Some(d) => {
            if d.v < 1.0 || d.v > forge_geom::spline::MAX_DEGREE as f64 || d.v >= n as f64 {
                return Err(Error::BadValue {
                    span: a.span("degree"),
                    what: "a spline's degree (1 up to one less than its points)",
                    value: d.v,
                });
            }
            d.v as usize
        }
        None => 3.min(n.saturating_sub(1)).max(1),
    };
    let list = |k: &'static str| -> R<Option<Vec<f64>>> {
        let Some(v) = a.opt(k) else {
            return Ok(None);
        };
        let Value::List(xs) = v else {
            return Err(Error::WrongType {
                span: a.span(k),
                what: k,
                want: Ty::List,
                found: v.ty(),
            });
        };
        xs.iter()
            .map(|x| num_of(x.clone(), &[Dim::Ratio, Dim::Count], a.span(k), k).map(|n| n.v))
            .collect::<R<Vec<f64>>>()
            .map(Some)
    };
    let knots = list("knots")?;
    if let Some(k) = &knots {
        if k.len() != n + degree + 1 {
            return Err(Error::BadValue {
                span: a.span("knots"),
                what: "the number of knots (points + degree + 1)",
                value: k.len() as f64,
            });
        }
        if let Some(w) = k.windows(2).find(|w| w[1] < w[0]) {
            return Err(Error::BadValue {
                span: a.span("knots"),
                what: "a knot below the one before it",
                value: w[1],
            });
        }
        if k[degree] >= k[n] {
            return Err(Error::BadValue {
                span: a.span("knots"),
                what: "the span of the knots the curve runs over",
                value: k[n] - k[degree],
            });
        }
    }
    let weights = list("weights")?;
    if let Some(w) = &weights {
        if w.len() != n {
            return Err(Error::BadValue {
                span: a.span("weights"),
                what: "the number of weights (one a point)",
                value: w.len() as f64,
            });
        }
        if let Some(&x) = w.iter().find(|x| !(**x > 0.0 && x.is_finite())) {
            return Err(Error::BadValue {
                span: a.span("weights"),
                what: "a weight",
                value: x,
            });
        }
    }
    Ok((degree, knots, weights))
}

/// The sketch the call is inside, or a WrongPlace error.
pub(crate) fn in_sketch(ev: &Eval, func: &'static str, span: Span) -> R<Rc<RefCell<Sk>>> {
    ev.sketch_ref().ok_or(Error::WrongPlace {
        span,
        func,
        needs_sketch: true,
    })
}

pub(crate) fn not_in_sketch(ev: &Eval, func: &'static str, span: Span) -> R<()> {
    if ev.sketch_ref().is_some() {
        return Err(Error::WrongPlace {
            span,
            func,
            needs_sketch: false,
        });
    }
    Ok(())
}

/// A 2D point: a pair of Lengths, or a point of this sketch.
pub fn p2_of(ev: &Eval, v: Value, span: Span) -> R<P2> {
    match v {
        Value::Tuple(t) if t.len() == 2 => {
            let x = num_of(t[0].clone(), LEN, span, "x")?;
            let y = num_of(t[1].clone(), LEN, span, "y")?;
            Ok(P2::At(Point2::new(x.v, y.v)))
        }
        Value::Ent(e) => {
            let here = ev.sketch_ref();
            if let Some(h) = &here
                && !Rc::ptr_eq(h, &e.sk)
            {
                return Err(Error::OtherSketch {
                    span,
                    sketch: e.sk.borrow().id.clone(),
                    here: h.borrow().id.clone(),
                });
            }
            let id = point_ent(&e, span)?;
            Ok(P2::Ent(id))
        }
        o => Err(Error::WrongType {
            span,
            what: "a 2D point",
            want: Ty::Vec2,
            found: o.ty(),
        }),
    }
}

/// The point an entity stands for: a point itself, a circle's or arc's centre.
pub(crate) fn point_ent(e: &Ent, span: Span) -> R<EntityId> {
    let sk = e.sk.borrow();
    match (e.kind, sk.s.entity(e.id)) {
        (EntKind::Point, _) => Ok(e.id),
        (_, Some(forge_sketch::Entity::Circle { center, .. } | forge_sketch::Entity::Arc { center, .. })) => {
            Ok(center)
        }
        _ => Err(Error::WrongEntity {
            span,
            want: EntKind::Point,
            found: e.kind,
        }),
    }
}

pub(crate) fn pos2(ev: &Eval, p: P2) -> Point2 {
    match p {
        P2::At(x) => x,
        P2::Ent(e) => ev
            .sketch_ref()
            .map_or(Point2::new(0.0, 0.0), |s| s.borrow().pos(e)),
    }
}

/// A 3D point an anchor gives: a sketch's point (or a circle's centre), or a point value.
pub fn point_of(_ev: &Eval, v: Value, span: Span) -> R<Point3> {
    match v {
        Value::Point(p) => Ok(p),
        Value::Verts(vs) => {
            let n = crate::verts::one(&vs, span, "a vertex (one)")?;
            crate::verts::point(_ev, n).ok_or(Error::EmptySelection { span })
        }
        Value::Wired(ref w) if w.point().is_some() => Ok(w.point().expect("checked")),
        Value::Ent(e) => {
            let id = point_ent(&e, span)?;
            let sk = e.sk.borrow();
            Ok(sk.world(sk.pos(id)))
        }
        Value::Axis(o, _) => Ok(o),
        Value::Plane(p) => Ok(p.frame.origin),
        o => Err(Error::WrongType {
            span,
            what: "a point",
            want: Ty::Point,
            found: o.ty(),
        }),
    }
}

/// A plane an anchor gives: a world plane (first feature only), a plane value, a sketch's
/// plane, a planar face set.
pub fn plane_of(ev: &mut Eval, v: Value, span: Span, _what: &'static str) -> R<Frame> {
    // an occurrence's face (`l1.plate.end`): where that occurrence is written, made in context
    if let Value::Asm(a) = &v
        && matches!(&**a, crate::asm::AsmVal::Ref(_) | crate::asm::AsmVal::Inst(_))
    {
        let here = crate::asm::context_value(ev, a, span)?;
        return plane_of(ev, here, span, _what);
    }
    match v {
        Value::Plane(p) => {
            ev.world_check(p.world.is_some(), span)?;
            Ok(p.frame)
        }
        Value::Sketch(s) => Ok(s.borrow().frame),
        Value::Wired(ref w) if w.plane().is_some() => Ok(w.plane().expect("checked")),
        Value::Faces(f) => match f.plane {
            Some(fr) => Ok(fr),
            // not building (a check, or after a failure): a face's plane is not known, as with
            // any other geometric anchor
            None if f.names.is_empty() && !ev.b.building => Ok(Frame::WORLD),
            None if f.names.is_empty() => Err(Error::NoFace {
                span,
                id: f.owner.clone(),
                role: f.role,
            }),
            None => Err(Error::NotPlanar {
                span,
                id: f.owner.clone(),
                role: f.role,
            }),
        },
        Value::Unresolved => Ok(Frame::WORLD),
        o => Err(Error::WrongType {
            span,
            what: "a plane",
            want: Ty::Plane,
            found: o.ty(),
        }),
    }
}

pub fn axis_of(_ev: &Eval, v: Value, span: Span) -> R<(Point3, UnitVec3)> {
    match v {
        Value::Axis(o, d) => Ok((o, d)),
        Value::Wired(ref w) if w.axis().is_some() => Ok(w.axis().expect("checked")),
        Value::Ent(e) if e.kind == EntKind::Line => {
            let sk = e.sk.borrow();
            let Some(forge_sketch::Entity::Line { a, b }) = sk.s.entity(e.id) else {
                unreachable!("a line")
            };
            let (pa, pb) = (sk.world(sk.pos(a)), sk.world(sk.pos(b)));
            Ok((pa, crate::eval::unit_or_z(pb - pa)))
        }
        Value::Feature(f) if f.axis.is_some() => Ok(f.axis.expect("checked")),
        o => Err(Error::WrongType {
            span,
            what: "an axis",
            want: Ty::Axis,
            found: o.ty(),
        }),
    }
}

pub fn dir_of(v: Value, span: Span) -> R<Vec3> {
    match v {
        Value::Dir(d) | Value::WorldDir(d) => Ok(d),
        Value::Wired(ref w) if w.axis().is_some() => Ok(w.axis().expect("checked").1.get()),
        Value::Axis(_, d) => Ok(d.get()),
        Value::Line(_, d, _) => Ok(d.get()),
        Value::Faces(f) if f.plane.is_some() => Ok(f.plane.expect("checked").z().get()),
        o => Err(Error::WrongType {
            span,
            what: "a direction",
            want: Ty::Dir,
            found: o.ty(),
        }),
    }
}

pub(crate) fn ent_of(v: Value, span: Span, want: EntKind) -> R<Ent> {
    match v {
        Value::Ent(e) => Ok(e),
        o => Err(Error::WrongType {
            span,
            what: "a sketch entity",
            want: Ty::Entity,
            found: {
                let _ = want;
                o.ty()
            },
        }),
    }
}

pub(crate) fn mk_ent(sk: &Rc<RefCell<Sk>>, id: EntityId) -> Value {
    let kind = sk.borrow().kind(id);
    Value::Ent(Ent {
        sk: sk.clone(),
        id,
        kind,
        loop_segs: Rc::new(vec![]),
    })
}

fn mk_loop(sk: &Rc<RefCell<Sk>>, segs: Vec<EntityId>) -> Value {
    Value::Ent(Ent {
        sk: sk.clone(),
        id: segs[0],
        kind: EntKind::Loop,
        loop_segs: Rc::new(segs),
    })
}

/// The name the entity being made takes, if the call is an assignment's whole right side.
pub(crate) fn pending_name(ev: &Eval, span: Span) -> Option<String> {
    ev.ent_name
        .as_ref()
        .filter(|(_, sp)| *sp == span)
        .map(|(n, _)| n.clone())
}

fn math1(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(ev, name, &["x"], args, span)?;
    let x = a.req_num(
        "x",
        &[
            Dim::Any,
            Dim::Angle,
            Dim::Ratio,
            Dim::Count,
            Dim::Length,
            Dim::Area,
        ],
    )?;
    let sp = a.span("x");
    let n = match name {
        "sqrt" => values::sqrt(&x, sp)?,
        "abs" => values::derive(&x, x.v.abs(), x.dim),
        "sin" | "cos" | "tan" => {
            num_of(Value::Num(x.clone()), ANG, sp, name)?;
            let v = match name {
                "sin" => dmath::sin(x.v),
                "cos" => dmath::cos(x.v),
                _ => {
                    let (s, c) = dmath::sin_cos(x.v);
                    s / c
                }
            };
            values::derive(&x, v, Dim::Ratio)
        }
        "asin" | "acos" | "atan" => {
            num_of(Value::Num(x.clone()), &[Dim::Ratio], sp, name)?;
            let v = match name {
                "asin" => dmath::asin(x.v),
                "acos" => dmath::acos(x.v),
                _ => dmath::atan(x.v),
            };
            values::derive(&x, v, Dim::Angle)
        }
        "count" => {
            if !x.is_whole() || x.v < 0.0 {
                return Err(Error::NotWhole { span: sp, value: x.v });
            }
            if !matches!(x.dim, Dim::Length | Dim::Any | Dim::Count) || !matches!(x.src, Src::Const) {
                return Err(Error::WrongDim {
                    span: sp,
                    what: "count(..) takes a number as written",
                    want: Dim::Count,
                    found: x.dim,
                });
            }
            values::derive(&x, x.v, Dim::Count)
        }
        _ => {
            if !matches!(x.dim, Dim::Length | Dim::Any | Dim::Ratio) || !matches!(x.src, Src::Const) {
                return Err(Error::WrongDim {
                    span: sp,
                    what: "ratio(..) takes a number as written",
                    want: Dim::Ratio,
                    found: x.dim,
                });
            }
            values::derive(&x, x.v, Dim::Ratio)
        }
    };
    Ok(Value::Num(n))
}

pub fn call(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    if let Some(r) = crate::asm::call(ev, name, args, span) {
        return r;
    }
    if let Some(r) = crate::wired::pre_call(ev, name, args, span) {
        return r;
    }
    match name {
        "properties" => crate::props::call(ev, args, span),
        "sqrt" | "abs" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "count" | "ratio" => {
            math1(ev, name, args, span)
        }
        // index arithmetic: the whole quotient and the remainder of two counts (`div(k, 5)` is
        // k's row of five, `mod(k, 5)` its column), whatever k is
        "div" | "mod" => {
            let a = Args::bind(ev, name, &["a", "b"], args, span)?;
            let x = a.req_num("a", &[Dim::Count])?;
            let y = a.req_num("b", &[Dim::Count])?;
            let d = y.v.round() as i64;
            if d <= 0 {
                return Err(Error::BadValue {
                    span: a.span("b"),
                    what: "a divisor",
                    value: y.v,
                });
            }
            let n = x.v.round() as i64;
            let v = if name == "div" {
                n.div_euclid(d)
            } else {
                n.rem_euclid(d)
            };
            Ok(Value::Num(Num {
                v: v as f64,
                dim: Dim::Count,
                src: values::join_src(&x.src, &y.src),
            }))
        }
        "atan2" => {
            let a = Args::bind(ev, name, &["y", "x"], args, span)?;
            let y = a.req_num("y", &[Dim::Length, Dim::Ratio])?;
            let x = a.req_num("x", &[Dim::Length, Dim::Ratio])?;
            values::add(&y, &x, false, span)?;
            let src = values::join_src(&y.src, &x.src);
            Ok(Value::Num(Num {
                v: dmath::atan2(y.v, x.v),
                dim: Dim::Angle,
                src: match src {
                    Src::Driving(p) => Src::Driven(std::sync::Arc::new([p.clone()].into())),
                    s => s,
                },
            }))
        }
        "min" | "max" => {
            let a = Args::bind(ev, name, &["a", "b"], args, span)?;
            let x = a.req_num("a", &[Dim::Length, Dim::Angle, Dim::Count, Dim::Ratio, Dim::Area])?;
            let y = a.req_num("b", &[Dim::Length, Dim::Angle, Dim::Count, Dim::Ratio, Dim::Area])?;
            let s = values::add(&x, &y, false, span)?;
            let v = if name == "min" { x.v.min(y.v) } else { x.v.max(y.v) };
            Ok(Value::Num(Num { v, ..s }))
        }
        "len" => {
            let a = Args::bind(ev, name, &["list"], args, span)?;
            match a.req("list")? {
                Value::List(xs) => Ok(Value::Num(Num::konst(xs.len() as f64, Dim::Count))),
                o => Err(Error::WrongType {
                    span: a.span("list"),
                    what: "len",
                    want: Ty::List,
                    found: o.ty(),
                }),
            }
        }
        "polar" => {
            let a = Args::bind(ev, name, &["n", "r", "start"], args, span)?;
            let n = a.req_num("n", &[Dim::Count])?;
            let r = a.req_num("r", LEN)?;
            let st = a.num("start", ANG)?.unwrap_or(Num::konst(0.0, Dim::Angle));
            let src = values::join_src(&values::join_src(&n.src, &r.src), &st.src);
            let k = n.v.round() as usize;
            let pts = (0..k)
                .map(|i| {
                    let t = st.v + std::f64::consts::TAU * i as f64 / k as f64;
                    let mk = |v: f64| {
                        Value::Num(Num {
                            v,
                            dim: Dim::Length,
                            src: match &src {
                                Src::Const => Src::Const,
                                s => Src::Driven(std::sync::Arc::new(s.keys())),
                            },
                        })
                    };
                    let (s, c) = dmath::sin_cos(t);
                    Value::Tuple(Rc::new(vec![mk(r.v * c), mk(r.v * s)]))
                })
                .collect();
            Ok(Value::List(Rc::new(pts)))
        }
        "sym" => {
            let a = Args::bind(ev, name, &["d"], args, span)?;
            let d = a.req_num("d", LEN_ANG)?;
            positive(&d, a.span("d"), "a symmetric extent")?;
            Ok(Value::Extent(ExtentV::Sym(d)))
        }
        "two" => {
            let a = Args::bind(ev, name, &["forward", "back"], args, span)?;
            let f = a.req_num("forward", LEN_ANG)?;
            let b = a.req_num("back", LEN_ANG)?;
            if f.dim != b.dim && f.dim != Dim::Any && b.dim != Dim::Any {
                return Err(Error::DimMismatch {
                    span,
                    op: "two",
                    left: f.dim,
                    right: b.dim,
                });
            }
            Ok(Value::Extent(ExtentV::Two(f, b)))
        }
        "upto" => {
            let a = Args::bind(ev, name, &["face", "offset"], args, span)?;
            let off = a
                .num("offset", LEN_ANG)?
                .unwrap_or_else(|| Num::konst(0.0, Dim::Any));
            match a.req("face")? {
                Value::Faces(f) => Ok(Value::Extent(ExtentV::UpTo(f, off))),
                Value::Wired(w) if w.plane().is_some() && w.work_ref().is_some() => Ok(Value::Extent(
                    ExtentV::UpToPlane(w.work_ref().expect("checked").feature, off),
                )),
                // a sketch's plane
                Value::Sketch(s) => Ok(Value::Extent(ExtentV::UpToPlane(s.borrow().idx, off))),
                Value::Unresolved => Ok(Value::Unresolved),
                o => Err(Error::WrongType {
                    span: a.span("face"),
                    what: "upto",
                    want: Ty::Faces,
                    found: o.ty(),
                }),
            }
        }
        "pt" | "near" => {
            let sk = in_sketch(ev, name, span)?;
            let a = Args::bind(ev, name, &["x", "y"], args, span)?;
            let x = a.req_num("x", LEN)?;
            let y = a.req_num("y", LEN)?;
            let id = sk.borrow_mut().point(
                Point2::new(x.v, y.v),
                name == "pt",
                pending_name(ev, span).as_deref(),
                span,
            )?;
            sk.borrow_mut().points.push(id);
            sk.borrow_mut().nums.extend([(id, "x", x), (id, "y", y)]);
            Ok(mk_ent(&sk, id))
        }
        "disc" => {
            let a = Args::bind(ev, name, &["center", "r"], args, span)?;
            let c = a.req("center")?;
            let r = a.req_num("r", LEN)?;
            positive(&r, a.span("r"), "a disc's radius")?;
            Ok(Value::Disc(Rc::new((c, r))))
        }
        "line" => line(ev, args, span),
        "arc" => {
            let sk = in_sketch(ev, name, span)?;
            let a = Args::bind(ev, name, &["center", "from", "to", "cw"], args, span)?;
            let c = p2_of(ev, a.req("center")?, a.span("center"))?;
            let f = p2_of(ev, a.req("from")?, a.span("from"))?;
            let t = p2_of(ev, a.req("to")?, a.span("to"))?;
            let cw = a.flag("cw")?;
            let mut s = sk.borrow_mut();
            let (c, f, t) = (s.p2(c, span)?, s.p2(f, span)?, s.p2(t, span)?);
            let nm = pending_name(ev, span);
            let id = s.arc(c, f, t, !cw, nm.as_deref(), span)?;
            drop(s);
            Ok(mk_ent(&sk, id))
        }
        "circle" => {
            let sk = in_sketch(ev, name, span)?;
            let a = Args::bind(ev, name, &["center", "r"], args, span)?;
            let cv = a.req("center")?;
            let c = p2_of(ev, cv.clone(), a.span("center"))?;
            let r = a.req_num("r", LEN)?;
            positive(&r, a.span("r"), "a radius")?;
            let mut s = sk.borrow_mut();
            let c = s.p2(c, span)?;
            let nm = pending_name(ev, span);
            let id = s.circle(c, r.v, nm.as_deref(), span)?;
            s.nums.push((id, "r", r.clone()));
            if let Value::Tuple(xy) = &cv
                && let [Value::Num(x), Value::Num(y)] = &xy[..]
            {
                s.nums.extend([(id, "x", x.clone()), (id, "y", y.clone())]);
            }
            drop(s);
            Ok(mk_ent(&sk, id))
        }
        "ellipse" => {
            let sk = in_sketch(ev, name, span)?;
            let a = Args::bind(ev, name, &["center", "major", "b"], args, span)?;
            let c = p2_of(ev, a.req("center")?, a.span("center"))?;
            let m = p2_of(ev, a.req("major")?, a.span("major"))?;
            let b = a.req_num("b", LEN)?;
            positive(&b, a.span("b"), "a semi-minor axis")?;
            let mut s = sk.borrow_mut();
            let (c, m) = (s.p2(c, span)?, s.p2(m, span)?);
            let nm = pending_name(ev, span);
            let id = s.ellipse(c, m, b.v, nm.as_deref(), span)?;
            s.nums.push((id, "b", b.clone()));
            drop(s);
            Ok(mk_ent(&sk, id))
        }
        "earc" => {
            let sk = in_sketch(ev, name, span)?;
            let a = Args::bind(
                ev,
                name,
                &["center", "major", "b", "from", "to", "cw"],
                args,
                span,
            )?;
            let c = p2_of(ev, a.req("center")?, a.span("center"))?;
            let m = p2_of(ev, a.req("major")?, a.span("major"))?;
            let b = a.req_num("b", LEN)?;
            positive(&b, a.span("b"), "a semi-minor axis")?;
            let f = p2_of(ev, a.req("from")?, a.span("from"))?;
            let t = p2_of(ev, a.req("to")?, a.span("to"))?;
            let cw = a.flag("cw")?;
            let mut s = sk.borrow_mut();
            let (c, m, f, t) = (s.p2(c, span)?, s.p2(m, span)?, s.p2(f, span)?, s.p2(t, span)?);
            let nm = pending_name(ev, span);
            let id = s.ellipse_arc(c, m, b.v, f, t, !cw, nm.as_deref(), span)?;
            s.nums.push((id, "b", b.clone()));
            drop(s);
            Ok(mk_ent(&sk, id))
        }
        // outside a sketch, rect is a bend relief
        "rect" if ev.sketch_ref().is_none() => crate::sheet::call(ev, name, args, span),
        "rect" => {
            let sk = in_sketch(ev, name, span)?;
            let a = Args::bind(ev, name, &["center", "size", "corner"], args, span)?;
            let size = a.req("size")?;
            let (w, h) = match &size {
                Value::Tuple(t) if t.len() == 2 => (
                    num_of(t[0].clone(), LEN, a.span("size"), "width")?,
                    num_of(t[1].clone(), LEN, a.span("size"), "height")?,
                ),
                o => {
                    return Err(Error::WrongType {
                        span: a.span("size"),
                        what: "a rect's size",
                        want: Ty::Vec2,
                        found: o.ty(),
                    });
                }
            };
            positive(&w, a.span("size"), "a width")?;
            positive(&h, a.span("size"), "a height")?;
            let corner = match (a.opt("center"), a.opt("corner")) {
                (Some(c), None) => {
                    let p = pos2(ev, p2_of(ev, c, a.span("center"))?);
                    Point2::new(p.x - w.v / 2.0, p.y - h.v / 2.0)
                }
                (None, Some(c)) => pos2(ev, p2_of(ev, c, a.span("corner"))?),
                _ => {
                    return Err(Error::MissingArg {
                        span,
                        func: "rect",
                        arg: "center or corner (one of them)",
                    });
                }
            };
            let nm = pending_name(ev, span);
            let segs = sk.borrow_mut().rect(corner, w.v, h.v, nm.as_deref(), span)?;
            Ok(mk_loop(&sk, segs))
        }
        "slot" | "hull" => {
            let sk = in_sketch(ev, name, span)?;
            let items: Vec<(Value, Span, Option<Num>)> = if name == "slot" {
                let a = Args::bind(ev, name, &["a", "b", "r"], args, span)?;
                let r = a.req_num("r", LEN)?;
                positive(&r, a.span("r"), "a slot's radius")?;
                vec![
                    (a.req("a")?, a.span("a"), Some(r.clone())),
                    (a.req("b")?, a.span("b"), Some(r)),
                ]
            } else {
                let mut v = vec![];
                for x in args {
                    let val = ev.expr(&x.value)?;
                    v.push((val, x.value.span, None));
                }
                v
            };
            let mut hull = vec![];
            for (v, sp, r) in items {
                let (c, r) = match (v, r) {
                    (Value::Disc(d), _) => (d.0.clone(), d.1.v),
                    (c, Some(r)) => (c, r.v),
                    (c, None) => (c, 0.0),
                };
                let p = p2_of(ev, c, sp)?;
                let (center, ent) = match p {
                    P2::At(x) => (x, None),
                    P2::Ent(e) => (sk.borrow().pos(e), Some(e)),
                };
                hull.push(Item { center, r, ent });
            }
            let nm = pending_name(ev, span);
            let segs = sk.borrow_mut().hull(&hull, nm.as_deref(), span)?;
            Ok(mk_loop(&sk, segs))
        }
        "poly" | "spline" if ev.path_mut().is_none() || name == "poly" => {
            let sk = in_sketch(ev, name, span)?;
            let keys: &[&str] = if name == "spline" {
                &[
                    "points",
                    "closed",
                    "cv",
                    "degree",
                    "knots",
                    "weights",
                    "start_slope",
                    "end_slope",
                ]
            } else {
                &["points", "closed"]
            };
            let a = Args::bind(ev, name, keys, args, span)?;
            // control vertices: a cubic B-spline the points pull, not one through them; a degree,
            // knots or weights make it one, open. End slopes make one through them open.
            let control = name == "spline" && CONTROL.iter().any(|k| a.opt(k).is_some());
            let sloped = name == "spline" && SLOPES.iter().any(|k| a.opt(k).is_some());
            let given = || {
                CONTROL
                    .into_iter()
                    .chain(SLOPES)
                    .find(|k| a.opt(k).is_some())
                    .expect("one")
            };
            let cv = match a.opt("cv") {
                None => control,
                Some(Value::Bool(true)) if sloped => {
                    let k = SLOPES.into_iter().find(|k| a.opt(k).is_some()).expect("one");
                    return Err(Error::NotWith {
                        span: a.span(k),
                        func: "spline",
                        arg: k,
                        with: "cv: true",
                    });
                }
                Some(Value::Bool(false)) if control => {
                    return Err(Error::NotWith {
                        span: a.span(given()),
                        func: "spline",
                        arg: given(),
                        with: "cv: false",
                    });
                }
                Some(Value::Bool(b)) => b,
                Some(o) => {
                    return Err(Error::WrongType {
                        span: a.span("cv"),
                        what: "cv",
                        want: Ty::Bool,
                        found: o.ty(),
                    });
                }
            };
            if control && sloped {
                let k = SLOPES.into_iter().find(|k| a.opt(k).is_some()).expect("one");
                return Err(Error::NotWith {
                    span: a.span(k),
                    func: "spline",
                    arg: k,
                    with: CONTROL.into_iter().find(|k| a.opt(k).is_some()).expect("one"),
                });
            }
            let closed = match a.opt("closed") {
                None => !control && !sloped,
                Some(Value::Bool(true)) if control || sloped => {
                    return Err(Error::NotWith {
                        span: a.span(given()),
                        func: "spline",
                        arg: given(),
                        with: "closed: true",
                    });
                }
                Some(Value::Bool(b)) => b,
                Some(o) => {
                    return Err(Error::WrongType {
                        span: a.span("closed"),
                        what: "closed",
                        want: Ty::Bool,
                        found: o.ty(),
                    });
                }
            };
            let pts = match a.req("points")? {
                Value::List(xs) => xs
                    .iter()
                    .map(|x| p2_of(ev, x.clone(), a.span("points")))
                    .collect::<R<Vec<_>>>()?,
                o => {
                    return Err(Error::WrongType {
                        span: a.span("points"),
                        what: "points",
                        want: Ty::List,
                        found: o.ty(),
                    });
                }
            };
            if pts.len() < 2 {
                return Err(Error::BadValue {
                    span: a.span("points"),
                    what: "the number of points",
                    value: pts.len() as f64,
                });
            }
            if name == "poly" {
                let nm = pending_name(ev, span);
                let segs = sk.borrow_mut().polygon(&pts, closed, nm.as_deref(), span)?;
                Ok(mk_loop(&sk, segs))
            } else {
                let mut s = sk.borrow_mut();
                let ids = pts.iter().map(|&p| s.p2(p, span)).collect::<R<Vec<_>>>()?;
                let nm = pending_name(ev, span);
                let id = if control {
                    let (degree, knots, weights) = control_args(&a, ids.len())?;
                    s.bspline_on(&ids, degree, knots, weights, nm.as_deref(), span)?
                } else if cv {
                    s.bspline(&ids, closed, nm.as_deref(), span)?
                } else {
                    let id = s.spline(&ids, closed, nm.as_deref(), span)?;
                    if sloped {
                        let from = slope_arg(&a, "start_slope")?;
                        let to = slope_arg(&a, "end_slope")?;
                        s.slopes(id, from, to, span)?;
                    }
                    id
                };
                drop(s);
                Ok(mk_ent(&sk, id))
            }
        }
        "spline" | "tarc" | "close" => path_seg(ev, name, args, span),
        "tangent" | "coincident" | "horizontal" | "vertical" | "parallel" | "perpendicular" | "equal"
        | "midpoint" | "distance" | "radius" | "angle" | "fix" | "concentric" | "collinear" | "symmetric"
        | "hdistance" | "vdistance" => constraint(ev, name, args, span),
        "extrude" | "cut" | "intersect" => extrude(ev, name, args, span),
        "use" => crate::uses::call(ev, args, span),
        n if crate::sheet::BUILTINS.contains(&n) => crate::sheet::call(ev, name, args, span),
        n if crate::thermo::BUILTINS.contains(&n) => crate::thermo::call(ev, name, args, span),
        n if crate::weld::BUILTINS.contains(&n) => crate::weld::call(ev, name, args, span),
        n if crate::harness::BUILTINS.contains(&n) => crate::harness::call(ev, name, args, span),
        "revolve" => revolve(ev, args, span),
        "fillet" => crate::fillets::fillet(ev, args, span),
        "chamfer" => chamfer(ev, args, span),
        "shell" => shell(ev, args, span),
        "loft" => crate::shapes::loft(ev, args, span),
        "offset" => {
            not_in_sketch(ev, name, span)?;
            let a = Args::bind(ev, name, &["plane", "d"], args, span)?;
            let f = plane_of(ev, a.req("plane")?, a.span("plane"), "plane")?;
            let d = a.req_num("d", LEN)?;
            Ok(Value::Plane(Rc::new(Plane {
                frame: f.with_origin(f.origin + f.z() * d.v),
                world: None,
            })))
        }
        "rotate" => {
            not_in_sketch(ev, name, span)?;
            let a = Args::bind(ev, name, &["plane", "about", "by"], args, span)?;
            let f = plane_of(ev, a.req("plane")?, a.span("plane"), "plane")?;
            let (o, d) = axis_of(ev, a.req("about")?, a.span("about"))?;
            let by = a.req_num("by", ANG)?;
            // as forge-model's angled work plane: the plane's normal turned about the axis
            let base = Frame::from_zx(o, d, f.z().get()).ok_or(Error::BadValue {
                span: a.span("about"),
                what: "an axis along the plane's normal",
                value: 0.0,
            })?;
            let n = base.rotated_about_z(by.v).x();
            let frame = Frame::from_zx(o, n, d.get()).ok_or(Error::BadValue {
                span,
                what: "the turned plane",
                value: by.v,
            })?;
            Ok(Value::Plane(Rc::new(Plane { frame, world: None })))
        }
        "axis" => {
            let a = Args::bind(ev, name, &["a", "b"], args, span)?;
            let p = point_of(ev, a.req("a")?, a.span("a"))?;
            let q = point_of(ev, a.req("b")?, a.span("b"))?;
            let d = (q - p).normalize().ok_or(Error::BadValue {
                span,
                what: "the distance between an axis's points",
                value: 0.0,
            })?;
            Ok(Value::Axis(p, d))
        }
        "edges" | "faces" => selection(ev, name, args, span),
        "shift" | "hline" | "vline" | "meet" => lines(ev, name, args, span),
        "mid" => {
            let a = Args::bind(ev, name, &["of"], args, span)?;
            let e = ent_of(a.req("of")?, a.span("of"), EntKind::Line)?;
            let s = e.sk.borrow();
            let Some(forge_sketch::Entity::Line { a: p, b: q }) = s.s.entity(e.id) else {
                return Err(Error::WrongEntity {
                    span: a.span("of"),
                    want: EntKind::Line,
                    found: e.kind,
                });
            };
            Ok(Value::Point(s.world(s.pos(p).lerp(s.pos(q), 0.5))))
        }
        "plane" => {
            not_in_sketch(ev, name, span)?;
            let a = Args::bind(ev, name, &["at", "normal", "x"], args, span)?;
            let o = point_of(ev, a.req("at")?, a.span("at"))?;
            let mut anchored = |k: &'static str| -> R<Vec3> {
                let v = a.req(k)?;
                ev.world_check(matches!(v, Value::WorldDir(_)), a.span(k))?;
                dir_of(v, a.span(k))
            };
            let z = anchored("normal")?;
            let z = z.normalize().ok_or(Error::BadValue {
                span: a.span("normal"),
                what: "a zero normal",
                value: 0.0,
            })?;
            // no x: the frame's own (forge_math's choice from the normal)
            let x = match a.opt("x") {
                None => Frame::from_z(o, z).x().get(),
                Some(_) => anchored("x")?,
            };
            let frame = Frame::from_zx(o, z, x).ok_or(Error::BadValue {
                span: a.span("x"),
                what: "an x along the normal",
                value: 0.0,
            })?;
            Ok(Value::Plane(Rc::new(Plane { frame, world: None })))
        }
        n if crate::shapes::NAMES.contains(&n) => crate::shapes::call(ev, n, args, span),
        _ => crate::wired::call(ev, name, args, span),
    }
}

pub fn ent_call(_ev: &mut Eval, e: Ent, _args: &[Arg], span: Span) -> R<Value> {
    Err(Error::NotCallable {
        span,
        found: Value::Ent(e).ty(),
    })
}

fn line(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    if ev.path_mut().is_some() {
        return path_seg(ev, "line", args, span);
    }
    let sk = in_sketch(ev, "line", span)?;
    let a = Args::bind(ev, "line", &["a", "b", "construction"], args, span)?;
    let p = p2_of(ev, a.req("a")?, a.span("a"))?;
    let q = p2_of(ev, a.req("b")?, a.span("b"))?;
    let cons = a.flag("construction")?;
    let mut s = sk.borrow_mut();
    let (p, q) = (s.p2(p, span)?, s.p2(q, span)?);
    let nm = pending_name(ev, span);
    let id = s.line(p, q, cons, nm.as_deref(), span)?;
    drop(s);
    Ok(mk_ent(&sk, id))
}

/// A segment of a path block: `line(to:)`, `tarc(to:)` (tangent arc), `spline(through:, to:)`,
/// `close()`.
fn path_seg(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    let sk = in_sketch(ev, name, span)?;
    if ev.path_mut().is_none() {
        return Err(Error::WrongPlace {
            span,
            func: name,
            needs_sketch: true,
        });
    }
    let a = Args::bind(ev, name, &["to", "through", "tangent"], args, span)?;
    let st = ev.path_mut().expect("in a path").clone();
    let seg_name = format!("{}.{}", st.name, st.segs.len());
    let end = match name {
        "close" => st.start,
        _ => {
            let p = p2_of(ev, a.req("to")?, a.span("to"))?;
            sk.borrow_mut().p2(p, span)?
        }
    };
    let tangent = match a.opt("tangent") {
        Some(Value::Bool(b)) => b,
        Some(o) => {
            return Err(Error::WrongType {
                span: a.span("tangent"),
                what: "tangent",
                want: Ty::Bool,
                found: o.ty(),
            });
        }
        None => name != "line" && name != "close",
    };
    let id = match name {
        "tarc" => {
            let prev = st.last.ok_or(Error::WrongPlace {
                span,
                func: "tarc after a first segment",
                needs_sketch: true,
            })?;
            sk.borrow_mut()
                .tangent_arc(prev, st.at, end, Some(&seg_name), span)?
        }
        "spline" => {
            let mut pts = vec![st.at];
            if let Some(v) = a.opt("through") {
                let Value::List(xs) = v else {
                    return Err(Error::WrongType {
                        span: a.span("through"),
                        what: "through",
                        want: Ty::List,
                        found: v.ty(),
                    });
                };
                for x in xs.iter() {
                    let p = p2_of(ev, x.clone(), a.span("through"))?;
                    pts.push(sk.borrow_mut().p2(p, span)?);
                }
            }
            pts.push(end);
            let id = sk.borrow_mut().spline(&pts, false, Some(&seg_name), span)?;
            if tangent && let Some(prev) = st.last {
                sk.borrow_mut().tangent(id, prev, span)?;
            }
            id
        }
        _ => {
            let id = sk.borrow_mut().line(st.at, end, false, Some(&seg_name), span)?;
            if tangent && let Some(prev) = st.last {
                sk.borrow_mut().tangent(prev, id, span)?;
            }
            id
        }
    };
    let ps = ev.path_mut().expect("in a path");
    ps.at = end;
    ps.last = Some(id);
    ps.segs.push(id);
    Ok(mk_ent(&sk, id))
}

fn constraint(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    let sk = in_sketch(ev, name, span)?;
    let a = Args::bind(ev, name, &["a", "b", "c", "value"], args, span)?;
    let ent = |k: &'static str| -> R<Ent> {
        let v = a.req(k)?;
        let e = ent_of(v, a.span(k), EntKind::Point)?;
        if !Rc::ptr_eq(&e.sk, &sk) {
            return Err(Error::OtherSketch {
                span: a.span(k),
                sketch: e.sk.borrow().id.clone(),
                here: sk.borrow().id.clone(),
            });
        }
        Ok(e)
    };
    let want = |e: &Ent, k: EntKind, sp: Span| -> R<()> {
        if e.kind != k {
            return Err(Error::WrongEntity {
                span: sp,
                want: k,
                found: e.kind,
            });
        }
        Ok(())
    };
    let c = match name {
        "tangent" => {
            let (x, y) = (ent("a")?, ent("b")?);
            sk.borrow_mut().tangent(x.id, y.id, span)?;
            return Ok(Value::Unresolved);
        }
        "fix" => Constraint::Fixed(ent("a")?.id),
        "horizontal" | "vertical" => {
            let x = ent("a")?;
            want(&x, EntKind::Line, a.span("a"))?;
            if name == "horizontal" {
                Constraint::Horizontal(x.id)
            } else {
                Constraint::Vertical(x.id)
            }
        }
        "coincident" => {
            let (x, y) = (ent("a")?, ent("b")?);
            match (x.kind, y.kind) {
                (EntKind::Point, EntKind::Point) => Constraint::Coincident(x.id, y.id),
                (EntKind::Point, EntKind::Line) => Constraint::PointOnLine(x.id, y.id),
                (EntKind::Point, EntKind::Circle | EntKind::Arc) => Constraint::PointOnCircle(x.id, y.id),
                (_, k) => {
                    return Err(Error::WrongEntity {
                        span: a.span("a"),
                        want: EntKind::Point,
                        found: if x.kind == EntKind::Point { k } else { x.kind },
                    });
                }
            }
        }
        "parallel" | "perpendicular" => {
            let (x, y) = (ent("a")?, ent("b")?);
            want(&x, EntKind::Line, a.span("a"))?;
            want(&y, EntKind::Line, a.span("b"))?;
            if name == "parallel" {
                Constraint::Parallel(x.id, y.id)
            } else {
                Constraint::Perpendicular(x.id, y.id)
            }
        }
        "equal" => Constraint::Equal(ent("a")?.id, ent("b")?.id),
        "concentric" => {
            let (x, y) = (ent("a")?, ent("b")?);
            for (e, k) in [(&x, "a"), (&y, "b")] {
                if !matches!(e.kind, EntKind::Circle | EntKind::Arc) {
                    want(e, EntKind::Circle, a.span(k))?;
                }
            }
            Constraint::Concentric(x.id, y.id)
        }
        "collinear" => {
            // parallel, and the second's start on the first
            let (x, y) = (ent("a")?, ent("b")?);
            want(&x, EntKind::Line, a.span("a"))?;
            want(&y, EntKind::Line, a.span("b"))?;
            let start = match sk.borrow().s.entity(y.id) {
                Some(forge_sketch::Entity::Line { a, .. }) => a,
                _ => unreachable!("a line"),
            };
            sk.borrow_mut()
                .constrain(Constraint::Parallel(x.id, y.id), span)?;
            Constraint::PointOnLine(start, x.id)
        }
        "symmetric" => {
            // two points mirrored about a line; two lines end for end, two circles or arcs by
            // their centres (and an arc's ends)
            let (x, y, l) = (ent("a")?, ent("b")?, ent("c")?);
            want(&l, EntKind::Line, a.span("c"))?;
            if x.kind != y.kind {
                want(&y, x.kind, a.span("b"))?;
            }
            let ends = |e: &Ent| -> Vec<EntityId> {
                match sk.borrow().s.entity(e.id) {
                    Some(forge_sketch::Entity::Line { a, b }) => vec![a, b],
                    Some(forge_sketch::Entity::Circle { center, .. }) => vec![center],
                    Some(forge_sketch::Entity::Arc {
                        center, start, end, ..
                    }) => vec![center, start, end],
                    _ => vec![e.id],
                }
            };
            let (ps, qs) = (ends(&x), ends(&y));
            if !matches!(
                x.kind,
                EntKind::Point | EntKind::Line | EntKind::Circle | EntKind::Arc
            ) {
                want(&x, EntKind::Point, a.span("a"))?;
            }
            // an arc mirrored runs the other way: its start is the other's end
            let qs = if x.kind == EntKind::Arc {
                vec![qs[0], qs[2], qs[1]]
            } else {
                qs
            };
            for (p, q) in ps.iter().zip(&qs).skip(1) {
                sk.borrow_mut()
                    .constrain(Constraint::Symmetric(*p, *q, l.id), span)?;
            }
            Constraint::Symmetric(ps[0], qs[0], l.id)
        }
        "midpoint" => {
            let (x, y) = (ent("a")?, ent("b")?);
            want(&x, EntKind::Point, a.span("a"))?;
            want(&y, EntKind::Line, a.span("b"))?;
            Constraint::Midpoint(x.id, y.id)
        }
        "radius" => {
            let x = ent("a")?;
            let v = a.req_num("b", LEN)?;
            positive(&v, a.span("b"), "a radius")?;
            Constraint::Radius(x.id, v.v)
        }
        "angle" => {
            let (x, y) = (ent("a")?, ent("b")?);
            let v = a.req_num("c", ANG)?;
            Constraint::Angle(x.id, y.id, v.v)
        }
        "hdistance" | "vdistance" => {
            // b.x - a.x (b.y - a.y) is d, in the sketch's axes
            let (x, y) = (ent("a")?, ent("b")?);
            want(&x, EntKind::Point, a.span("a"))?;
            want(&y, EntKind::Point, a.span("b"))?;
            let v = a.req_num("c", LEN)?;
            if name == "hdistance" {
                Constraint::HorizontalDistance(x.id, y.id, v.v)
            } else {
                Constraint::VerticalDistance(x.id, y.id, v.v)
            }
        }
        _ => {
            // distance(a, b, d) between points, a point and a line; distance(line, d): its length
            let x = ent("a")?;
            match a.opt("c") {
                Some(_) => {
                    let y = ent("b")?;
                    let v = a.req_num("c", LEN)?;
                    match (x.kind, y.kind) {
                        (EntKind::Point, EntKind::Point) => Constraint::Distance(x.id, y.id, v.v),
                        (EntKind::Point, EntKind::Line) => Constraint::DistancePointLine(x.id, y.id, v.v),
                        (_, k) => {
                            return Err(Error::WrongEntity {
                                span: a.span("b"),
                                want: EntKind::Point,
                                found: k,
                            });
                        }
                    }
                }
                None => {
                    want(&x, EntKind::Line, a.span("a"))?;
                    let v = a.req_num("b", LEN)?;
                    Constraint::Length(x.id, v.v)
                }
            }
        }
    };
    sk.borrow_mut().constrain(c, span)?;
    Ok(Value::Unresolved)
}

// ---- features ---------------------------------------------------------------------------

pub(crate) fn profile_of(
    ev: &Eval,
    v: Value,
    span: Span,
) -> R<(Rc<RefCell<Sk>>, forge_ops::Profile, Vec<EntityId>)> {
    let _ = ev;
    match v {
        Value::Sketch(s) => {
            let p = s.borrow().profile(None, span)?;
            let segs = s.borrow().ents.iter().map(|e| e.0).collect();
            Ok((s, p, segs))
        }
        Value::Ent(e) => {
            let ids: Vec<EntityId> = if e.loop_segs.is_empty() {
                vec![e.id]
            } else {
                e.loop_segs.to_vec()
            };
            let p = e.sk.borrow().profile(Some(&ids), span)?;
            Ok((e.sk.clone(), p, ids))
        }
        o => Err(Error::WrongType {
            span,
            what: "a profile (a sketch or one of its loops)",
            want: Ty::Sketch,
            found: o.ty(),
        }),
    }
}

/// A profile argument's sketch, its regions, and the entities they were taken from.
type Regions = (Rc<RefCell<Sk>>, Vec<forge_ops::Profile>, Vec<EntityId>);

/// The regions a list of loops of one sketch (a sketch text, a loop per character) bound, only
/// they: each element's regions, the union of its parts' (a composite glyph's components may
/// overlap); different elements never meet.
pub(crate) fn profiles_of(xs: &[Value], span: Span) -> R<Regions> {
    let mut sk: Option<Rc<RefCell<Sk>>> = None;
    let mut ids = Vec::new();
    let mut elements: Vec<&[EntityId]> = Vec::new();
    for x in xs.iter() {
        match x {
            Value::Ent(e) if !e.loop_segs.is_empty() => {
                if let Some(s) = &sk
                    && !Rc::ptr_eq(s, &e.sk)
                {
                    return Err(Error::OtherSketch {
                        span,
                        sketch: e.sk.borrow().id.clone(),
                        here: s.borrow().id.clone(),
                    });
                }
                sk = Some(e.sk.clone());
                ids.extend(e.loop_segs.iter().copied());
                elements.push(&e.loop_segs);
            }
            o => {
                return Err(Error::WrongType {
                    span,
                    what: "a profile (a sketch, one of its loops, or a text)",
                    want: Ty::Sketch,
                    found: o.ty(),
                });
            }
        }
    }
    let sk = sk.ok_or(Error::WrongType {
        span,
        what: "a profile (a sketch, one of its loops, or a text)",
        want: Ty::Sketch,
        found: Ty::List,
    })?;
    let ps = sk.borrow().profiles_grouped(&elements, span)?;
    Ok((sk, ps, ids))
}

pub(crate) fn has_solid(ev: &Eval) -> bool {
    ev.feats.iter().any(|f| {
        (matches!(f.kind, "extrude" | "revolve" | "loft" | "import")
            || crate::shapes::SOLID.contains(&f.kind))
            && f.status != crate::eval::Status::Failed
    })
}

fn extrude(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    use crate::feat_error::{Because, FeatError};
    not_in_sketch(ev, name, span)?;
    let id = ev.take_id(if name == "extrude" { "extrude" } else { "cut" }, span)?;
    let a = Args::bind(
        ev,
        name,
        &[
            "profile", "d", "taper", "new", "along", "flip", "bodies", "surface", "thin", "side",
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    if a.flag("surface")? || a.opt("thin").is_some() {
        return extrude_sheet(ev, name, &a, id, span);
    }
    if a.opt("side").is_some() {
        return Err(Error::NotWith {
            span: a.span("side"),
            func: name,
            arg: "side",
            with: "no thin (a side is the side a thin wall is thickened to)",
        });
    }
    // one region (a sketch's, a loop's), or every region of a list of loops (a text's)
    let (sk, profile, many, segs) = match a.req("profile")? {
        Value::List(xs) => {
            let (sk, mut ps, segs) = profiles_of(&xs, a.span("profile"))?;
            if ps.is_empty() {
                return Err(Error::Regions {
                    span: a.span("profile"),
                    sketch: sk.borrow().id.clone(),
                    count: 0,
                });
            }
            let one = ps.remove(0);
            (sk, one, ps, segs)
        }
        v => {
            let (sk, p, segs) = profile_of(ev, v, a.span("profile"))?;
            (sk, p, Vec::new(), segs)
        }
    };
    if a.opt("taper").is_some() {
        if !matches!(a.req("d")?, Value::Num(_)) {
            return Err(Error::NotWith {
                span: a.span("taper"),
                func: name,
                arg: "taper",
                with: "an extent other than a depth (a taper runs a set distance)",
            });
        }
        if a.opt("along").is_some() {
            return Err(FeatError::Conflict {
                span: a.span("along"),
                func: name,
                a: "taper",
                b: "along",
            }
            .into());
        }
        let curved = std::iter::once(&profile)
            .chain(&many)
            .flat_map(|p| &p.loops)
            .any(|l| {
                l.segments
                    .iter()
                    .any(|g| matches!(g.0, forge_ops::Segment::Spline(_)))
            });
        if curved {
            return Err(Error::TaperOnCurve {
                span: a.span("taper"),
                sketch: sk.borrow().id.clone(),
            });
        }
    }
    let cut = name == "cut";
    let mut dims = vec![];
    let sign = if cut { -1.0 } else { 1.0 };
    // along: the walls run that way, the ends square to the sketch's normal
    let along = a.opt("along").map(|v| dir_of(v, a.span("along"))).transpose()?;
    // flip: thru and next run the other way (a depth runs the other way when negative)
    let flip = a.flag("flip")?;
    let d_val = a.req("d")?;
    if flip && !matches!(d_val, Value::Extent(ExtentV::Thru | ExtentV::Next)) {
        return Err(FeatError::NotApplicable {
            span: a.span("flip"),
            func: name,
            arg: "flip",
            because: Because::OtherForm,
        }
        .into());
    }
    if along.is_some()
        && !matches!(
            d_val,
            Value::Num(_)
                | Value::Extent(ExtentV::Sym(_) | ExtentV::Two(..) | ExtentV::Thru | ExtentV::ThruAll)
        )
    {
        return Err(Error::NotWith {
            span: a.span("along"),
            func: name,
            arg: "along",
            with: "next or upto (a slanted extrude runs a set distance, or through)",
        });
    }
    let len = |n: &Num, sp: Span, what: &'static str| num_of(Value::Num(n.clone()), LEN, sp, what);
    // the kernel's slanted extent: forward along the normal, back against it (a cut's into the
    // material, against the normal)
    let slant = |f: f64, b: f64, dir: Vec3| {
        if cut {
            Extent::Along {
                forward: b,
                back: f,
                dir,
            }
        } else {
            Extent::Along {
                forward: f,
                back: b,
                dir,
            }
        }
    };
    let (extent, near, far) = match d_val {
        Value::Num(n) => {
            let d = num_of(Value::Num(n.clone()), LEN, a.span("d"), "a depth")?;
            if d.v == 0.0 {
                return Err(Error::BadValue {
                    span: a.span("d"),
                    what: "a depth",
                    value: 0.0,
                });
            }
            dims.push(dim("depth", &d));
            let dist = sign * d.v;
            match (a.num("taper", ANG)?, along) {
                (Some(t), _) => {
                    dims.push(dim("taper", &t));
                    (
                        Extent::Tapered {
                            distance: dist,
                            angle: t.v,
                        },
                        0.0,
                        Some(dist),
                    )
                }
                (None, Some(dir)) => {
                    let (f, b) = if d.v > 0.0 { (d.v, 0.0) } else { (0.0, -d.v) };
                    (slant(f, b, dir), dist.min(0.0), Some(dist.max(0.0)))
                }
                (None, None) => (Extent::Distance(dist), 0.0, Some(dist)),
            }
        }
        Value::Extent(x) => match x {
            ExtentV::Sym(d) => {
                let d = len(&d, a.span("d"), "a symmetric extent")?;
                dims.push(dim("depth", &d));
                let d = d.v;
                match along {
                    Some(dir) => (slant(d / 2.0, d / 2.0, dir), -d / 2.0, Some(d / 2.0)),
                    None => (Extent::Symmetric(d), -d / 2.0, Some(d / 2.0)),
                }
            }
            ExtentV::Two(f, b) => {
                let f = len(&f, a.span("d"), "a forward extent")?;
                let b = len(&b, a.span("d"), "a back extent")?;
                dims.push(dim("forward", &f));
                dims.push(dim("back", &b));
                let (f, b) = (f.v, b.v);
                match along {
                    Some(dir) => (slant(f, b, dir), -sign * b, Some(sign * f)),
                    // a cut's forward runs into the material, against the normal: it spans -f to
                    // b along the normal, either way round (negative sizes, as written before,
                    // run it the other way)
                    None => (
                        if cut {
                            Extent::TwoSided {
                                forward: (-f).max(b),
                                back: -((-f).min(b)),
                            }
                        } else {
                            Extent::TwoSided { forward: f, back: b }
                        },
                        -sign * b,
                        Some(sign * f),
                    ),
                }
            }
            ExtentV::Thru | ExtentV::ThruAll => {
                let toward = match (&x, cut != flip) {
                    (ExtentV::ThruAll, _) => Toward::Both,
                    (_, true) => Toward::Back,
                    _ => Toward::Forward,
                };
                match along {
                    Some(dir) => (Extent::ThroughAlong { toward, dir }, 0.0, None),
                    None => (Extent::Through(toward), 0.0, None),
                }
            }
            ExtentV::Next => (Extent::ToNext { reverse: cut != flip }, 0.0, None),
            ExtentV::UpTo(f, off) => {
                let off = len(&off, a.span("d"), "an offset")?;
                let feature = ev.feats.iter().find(|x| x.id == f.owner).map_or(0, |x| x.idx);
                let name = f.names.first().copied().unwrap_or(Name::NONE);
                (
                    Extent::To {
                        target: Target::Face { feature, name },
                        offset: off.v,
                    },
                    0.0,
                    None,
                )
            }
            ExtentV::UpToPlane(k, off) => {
                let off = len(&off, a.span("d"), "an offset")?;
                (
                    Extent::To {
                        target: Target::Plane(k),
                        offset: off.v,
                    },
                    0.0,
                    None,
                )
            }
        },
        Value::Unresolved => (Extent::Distance(1.0), 0.0, None),
        o => {
            return Err(Error::WrongType {
                span: a.span("d"),
                what: "a depth or extent",
                want: Ty::Num,
                found: o.ty(),
            });
        }
    };
    let op = match name {
        "cut" => Combine::Cut,
        "intersect" => Combine::Keep,
        _ if a.flag("new")? || !has_solid(ev) => Combine::New,
        _ => Combine::Add,
    };
    let f = if many.is_empty() {
        Feature::Extrude { profile, extent, op }
    } else {
        // several regions (a text's glyphs): each extruded alike, none reaching a face
        if matches!(extent, Extent::To { .. } | Extent::ToNext { .. }) {
            return Err(Error::NotWith {
                span: a.span("d"),
                func: name,
                arg: "d",
                with: "several regions (a text): next and upto end each region on its own face",
            });
        }
        // (in order: the first region was taken out above)
        let mut profiles = many;
        profiles.insert(0, profile);
        Feature::Regions(Box::new(forge_model::regions::Regions { profiles, extent, op }))
    };
    let f = crate::bodies::wrap_in(ev, &a, f)?;
    let (idx, hid) = ev.push_feature(&id, "extrude", f, span, dims)?;
    let axis = single_circle_axis(&sk, &segs);
    Ok(Value::Feature(Rc::new(Feat {
        id,
        kind: if cut { "cut" } else { "extrude" },
        idx,
        hid,
        shape: Some(Shape {
            sketch: sk.clone(),
            far,
            near,
            cut,
            segs,
        }),
        axis,
        prof: Some(sk),
    })))
}

/// `extrude(PATH | PROFILE, d | sym(d) | two(f, b), surface: true)`: a sheet, a body of its own.
/// With `thin: t, side:` instead, that sheet thickened (as `thicken` does) into a wall t thick,
/// which adds (or cuts) as an extrude does.
/// A sheet's chains: a whole sketch gives the loops of its region; anything else is read end to
/// end, a chain ending where it closes (a loop, a circle, each contour of a text's glyph), each a
/// tube with its front outside.
fn sheet_chains_of(ev: &Eval, v: Value, span: Span) -> R<(Rc<RefCell<Sk>>, Vec<crate::shapes::Chain>)> {
    if let Value::Sketch(_) = v {
        let (sk, p, _) = profile_of(ev, v, span)?;
        return Ok((sk, p.loops.into_iter().map(|l| l.segments).collect()));
    }
    crate::shapes::chains_of(v, span)
}

fn extrude_sheet(ev: &mut Eval, name: &'static str, a: &Args, id: String, span: Span) -> R<Value> {
    let thin = a.opt("thin").is_some();
    if thin && a.flag("surface")? {
        return Err(Error::NotWith {
            span: a.span("thin"),
            func: name,
            arg: "thin",
            with: "surface (a thin wall is a solid)",
        });
    }
    let only: &[&'static str] = if thin {
        &["taper", "along", "flip"]
    } else {
        &["taper", "new", "along", "flip", "bodies", "side"]
    };
    for &arg in only {
        if a.opt(arg).is_some() {
            return Err(Error::NotWith {
                span: a.span(arg),
                func: name,
                arg,
                with: if thin {
                    "thin (a wall runs along the normal, a set distance)"
                } else {
                    "surface (a sheet is a body of its own, made along the normal)"
                },
            });
        }
    }
    if name != "extrude" && !thin {
        return Err(Error::NotWith {
            span: a.span("surface"),
            func: name,
            arg: "surface",
            with: "a cut or intersect (a sheet holds no material to take away)",
        });
    }
    let (sk, chains) = sheet_chains_of(ev, a.req("profile")?, a.span("profile"))?;
    let len = |n: &Num, what: &'static str| num_of(Value::Num(n.clone()), LEN, a.span("d"), what);
    let mut dims = vec![];
    let (lo, hi) = match a.req("d")? {
        Value::Num(n) => {
            let d = len(&n, "a depth")?;
            if d.v == 0.0 {
                return Err(Error::BadValue {
                    span: a.span("d"),
                    what: "a depth",
                    value: 0.0,
                });
            }
            dims.push(dim("depth", &d));
            (d.v.min(0.0), d.v.max(0.0))
        }
        Value::Extent(ExtentV::Sym(d)) => {
            let d = len(&d, "a symmetric extent")?;
            dims.push(dim("depth", &d));
            (-d.v / 2.0, d.v / 2.0)
        }
        Value::Extent(ExtentV::Two(f, b)) => {
            let (f, b) = (len(&f, "a forward extent")?, len(&b, "a back extent")?);
            dims.push(dim("forward", &f));
            dims.push(dim("back", &b));
            (-b.v, f.v)
        }
        Value::Unresolved => (0.0, 1.0),
        Value::Extent(_) => {
            return Err(Error::NotWith {
                span: a.span("d"),
                func: name,
                arg: "surface",
                with: "an extent other than a depth, sym or two (a sheet runs a set distance)",
            });
        }
        o => {
            return Err(Error::WrongType {
                span: a.span("d"),
                what: "a depth or extent",
                want: Ty::Num,
                found: o.ty(),
            });
        }
    };
    let plane = sk.borrow().frame;
    let thin = match a.num("thin", LEN)? {
        None => None,
        Some(t) => {
            positive(&t, a.span("thin"), "a wall's thickness")?;
            dims.push(dim("thin", &t));
            let (lo, hi) = match crate::shapes::side_word(a)? {
                forge_model::Side::Pos => (0.0, t.v),
                forge_model::Side::Neg => (-t.v, 0.0),
                forge_model::Side::Both => (-t.v / 2.0, t.v / 2.0),
            };
            let op = match name {
                "cut" => Combine::Cut,
                "intersect" => Combine::Keep,
                _ if a.flag("new")? || !has_solid(ev) => Combine::New,
                _ => Combine::Add,
            };
            Some(forge_model::sheets::Thin { lo, hi, op })
        }
    };
    // a cut runs into the material, against the normal
    let (lo, hi) = if name == "cut" { (-hi, -lo) } else { (lo, hi) };
    let f = Feature::Sheet(Box::new(forge_model::sheets::Sheet {
        plane,
        chains,
        how: forge_model::sheets::SheetHow::Extrude { lo, hi },
        thin,
    }));
    let kind = match (thin.is_some(), name) {
        (false, _) => "sheet",
        (true, "cut") => "cut",
        (true, _) => "extrude",
    };
    let f = if thin.is_some() {
        crate::bodies::wrap_in(ev, a, f)?
    } else {
        f
    };
    let (idx, hid) = ev.push_feature(&id, kind, f, span, dims)?;
    Ok(Value::Feature(Rc::new(Feat {
        id,
        kind,
        idx,
        hid,
        shape: None,
        axis: None,
        prof: Some(sk),
    })))
}

fn single_circle_axis(sk: &Rc<RefCell<Sk>>, segs: &[EntityId]) -> Option<(Point3, UnitVec3)> {
    let s = sk.borrow();
    let circles: Vec<EntityId> = segs
        .iter()
        .copied()
        .filter(|e| matches!(s.s.entity(*e), Some(forge_sketch::Entity::Circle { .. })))
        .collect();
    let [c] = circles[..] else { return None };
    let Some(forge_sketch::Entity::Circle { center, .. }) = s.s.entity(c) else {
        return None;
    };
    Some((s.world(s.pos(center)), s.frame.z()))
}

fn revolve(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "revolve", span)?;
    let id = ev.take_id("revolve", span)?;
    let a = Args::bind(
        ev,
        "revolve",
        &[
            "profile", "axis", "angle", "cut", "keep", "new", "bodies", "surface",
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    if a.flag("surface")? {
        return revolve_sheet(ev, &a, id, span);
    }
    // an axis of the part or the origin (a work axis, `axis(P, Q)`, `E.axis`, X Y Z), not a line
    // of the profile's sketch: where it lies in that sketch's plane
    let axv = a.req("axis")?;
    if !matches!(axv, Value::Ent(_)) {
        let (o, d) = match axv {
            Value::WorldDir(d) => (Point3::ORIGIN, crate::eval::unit_or_z(d)),
            v => axis_of(ev, v, a.span("axis"))?,
        };
        let (sk, profile, _) = profile_of(ev, a.req("profile")?, a.span("profile"))?;
        let (pa, pb) = {
            let s = sk.borrow();
            let (p, q) = (s.frame.point_to_local(o), s.frame.point_to_local(o + d.get()));
            let off = p.z.abs().max(q.z.abs());
            if off > 1e-6 {
                return Err(Error::BadValue {
                    span: a.span("axis"),
                    what: "an axis out of the profile's plane (its distance)",
                    value: off,
                });
            }
            (Point2::new(p.x, p.y), Point2::new(q.x, q.y))
        };
        return revolve_on(ev, &a, id, span, sk, profile, (pa, pb));
    }
    let ax = ent_of(axv, a.span("axis"), EntKind::Line)?;
    let mut profiled = profile_of(ev, a.req("profile")?, a.span("profile"));
    // an axis drawn as a plain line (not construction) that is no side of the profile: the
    // profile without it
    if profiled.is_err() && ax.kind == EntKind::Line && !ax.sk.borrow().s.is_construction(ax.id) {
        let _ = ax.sk.borrow_mut().s.set_construction(ax.id, true);
        let again = profile_of(ev, a.req("profile")?, a.span("profile"));
        let _ = ax.sk.borrow_mut().s.set_construction(ax.id, false);
        if again.is_ok() {
            profiled = again;
        }
    }
    let (sk, profile, _) = profiled?;
    if !Rc::ptr_eq(&ax.sk, &sk) || ax.kind != EntKind::Line {
        return Err(Error::WrongEntity {
            span: a.span("axis"),
            want: EntKind::Line,
            found: ax.kind,
        });
    }
    let (pa, pb) = {
        let s = sk.borrow();
        let Some(forge_sketch::Entity::Line { a, b }) = s.s.entity(ax.id) else {
            unreachable!("a line")
        };
        (s.pos(a), s.pos(b))
    };
    revolve_on(ev, &a, id, span, sk, profile, (pa, pb))
}

/// `revolve(PATH | PROFILE, axis: LINE | AXIS, angle: A | sym(A) | two(A, B), surface: true)`: a
/// sheet, a body of its own.
fn revolve_sheet(ev: &mut Eval, a: &Args, id: String, span: Span) -> R<Value> {
    for arg in ["cut", "keep", "new", "bodies"] {
        if a.opt(arg).is_some() {
            return Err(Error::NotWith {
                span: a.span(arg),
                func: "revolve",
                arg,
                with: "surface (a sheet is a body of its own)",
            });
        }
    }
    let (sk, chains) = sheet_chains_of(ev, a.req("profile")?, a.span("profile"))?;
    let (pa, pb) = match a.req("axis")? {
        Value::Ent(ax) => {
            if !Rc::ptr_eq(&ax.sk, &sk) || ax.kind != EntKind::Line {
                return Err(Error::WrongEntity {
                    span: a.span("axis"),
                    want: EntKind::Line,
                    found: ax.kind,
                });
            }
            let s = sk.borrow();
            let Some(forge_sketch::Entity::Line { a, b }) = s.s.entity(ax.id) else {
                unreachable!("a line")
            };
            (s.pos(a), s.pos(b))
        }
        axv => {
            let (o, d) = match axv {
                Value::WorldDir(d) => (Point3::ORIGIN, crate::eval::unit_or_z(d)),
                v => axis_of(ev, v, a.span("axis"))?,
            };
            let s = sk.borrow();
            let (p, q) = (s.frame.point_to_local(o), s.frame.point_to_local(o + d.get()));
            let off = p.z.abs().max(q.z.abs());
            if off > 1e-6 {
                return Err(Error::BadValue {
                    span: a.span("axis"),
                    what: "an axis out of the profile's plane (its distance)",
                    value: off,
                });
            }
            (Point2::new(p.x, p.y), Point2::new(q.x, q.y))
        }
    };
    let mut dims = vec![];
    let ang = |n: &Num, what: &'static str| num_of(Value::Num(n.clone()), ANG, a.span("angle"), what);
    let (from, angle) = match a.opt("angle") {
        None => (0.0, std::f64::consts::TAU),
        Some(Value::Extent(ExtentV::Sym(t))) => {
            let t = ang(&t, "a symmetric turn")?;
            dims.push(dim("angle", &t));
            (-t.v / 2.0, t.v)
        }
        Some(Value::Extent(ExtentV::Two(f, b))) => {
            let (f, b) = (ang(&f, "a forward turn")?, ang(&b, "a back turn")?);
            dims.push(dim("forward", &f));
            dims.push(dim("back", &b));
            (-b.v, f.v + b.v)
        }
        Some(Value::Extent(_)) => {
            return Err(Error::NotWith {
                span: a.span("angle"),
                func: "revolve",
                arg: "surface",
                with: "a turn up to a face (a sheet turns a set angle)",
            });
        }
        Some(_) => {
            let t = a.req_num("angle", ANG)?;
            dims.push(dim("angle", &t));
            (t.v.min(0.0), t.v.abs())
        }
    };
    let plane = sk.borrow().frame;
    let f = Feature::Sheet(Box::new(forge_model::sheets::Sheet {
        plane,
        chains,
        how: forge_model::sheets::SheetHow::Revolve {
            axis: Axis2 {
                origin: pa,
                dir: pb - pa,
            },
            from,
            angle,
        },
        thin: None,
    }));
    let (idx, hid) = ev.push_feature(&id, "sheet", f, span, dims)?;
    let (wa, wb) = {
        let s = sk.borrow();
        (s.world(pa), s.world(pb))
    };
    Ok(Value::Feature(Rc::new(Feat {
        id,
        kind: "sheet",
        idx,
        hid,
        shape: None,
        axis: Some((wa, crate::eval::unit_or_z(wb - wa))),
        prof: Some(sk),
    })))
}

/// A revolve of `profile` (of sketch `sk`) about the line through `pa`, `pb` in its plane.
fn revolve_on(
    ev: &mut Eval,
    a: &Args,
    id: String,
    span: Span,
    sk: Rc<RefCell<Sk>>,
    profile: forge_ops::Profile,
    (pa, pb): (Point2, Point2),
) -> R<Value> {
    let mut dims = vec![];
    let ang = |n: &Num, what: &'static str| num_of(Value::Num(n.clone()), ANG, a.span("angle"), what);
    // angle: A | sym(A) | two(A, B) | upto(FACES | WORKPLANE, offset: A)
    let turn = match a.opt("angle") {
        None => Turn::Angle(std::f64::consts::TAU),
        Some(Value::Extent(ExtentV::Sym(t))) => {
            let t = ang(&t, "a symmetric turn")?;
            dims.push(dim("angle", &t));
            Turn::Symmetric(t.v)
        }
        Some(Value::Extent(ExtentV::Two(f, b))) => {
            let (f, b) = (ang(&f, "a forward turn")?, ang(&b, "a back turn")?);
            dims.push(dim("forward", &f));
            dims.push(dim("back", &b));
            Turn::TwoSided {
                forward: f.v,
                back: b.v,
            }
        }
        Some(Value::Extent(x @ (ExtentV::UpTo(..) | ExtentV::UpToPlane(..)))) => {
            let off = match &x {
                ExtentV::UpTo(_, o) | ExtentV::UpToPlane(_, o) => ang(o, "an offset turn")?,
                _ => unreachable!("upto"),
            };
            let target = match x {
                ExtentV::UpToPlane(k, _) => Target::Plane(k),
                ExtentV::UpTo(f, _) => Target::Face {
                    feature: ev.feats.iter().find(|y| y.id == f.owner).map_or(0, |y| y.idx),
                    name: f.names.first().copied().unwrap_or(Name::NONE),
                },
                _ => unreachable!("upto"),
            };
            Turn::To {
                target,
                offset: off.v,
            }
        }
        Some(Value::Extent(_)) => {
            return Err(Error::WrongType {
                span: a.span("angle"),
                what: "a turn (an angle, sym(A), two(A, B) or upto(..))",
                want: Ty::Num,
                found: Ty::Extent,
            });
        }
        Some(_) => {
            let t = a.req_num("angle", ANG)?;
            dims.push(dim("angle", &t));
            Turn::Angle(t.v)
        }
    };
    let op = crate::shapes::combine(ev, a, "revolve")?;
    let axis2 = Axis2 {
        origin: pa,
        dir: pb - pa,
    };
    let f = crate::bodies::wrap_in(
        ev,
        a,
        Feature::Revolve {
            profile,
            axis: axis2,
            extent: turn,
            op,
        },
    )?;
    let (idx, hid) = ev.push_feature(&id, "revolve", f, span, dims)?;
    let (wa, wb) = {
        let s = sk.borrow();
        (s.world(pa), s.world(pb))
    };
    Ok(Value::Feature(Rc::new(Feat {
        id,
        kind: "revolve",
        idx,
        hid,
        shape: None,
        axis: Some((wa, crate::eval::unit_or_z(wb - wa))),
        prof: Some(sk.clone()),
    })))
}

pub(crate) fn edges_value(v: Value, span: Span) -> R<Option<Vec<Name>>> {
    match v {
        Value::Map(m) => edges_value(
            Value::List(Rc::new(m.iter().map(|x| x.1.clone()).collect())),
            span,
        ),
        Value::Edges(e) => Ok(Some(e.to_vec())),
        Value::Unresolved => Ok(None),
        Value::List(xs) => {
            let mut out = vec![];
            for x in xs.iter() {
                match edges_value(x.clone(), span)? {
                    Some(e) => out.extend(e),
                    None => return Ok(None),
                }
            }
            Ok(Some(out))
        }
        o => Err(Error::WrongType {
            span,
            what: "edges",
            want: Ty::Edges,
            found: o.ty(),
        }),
    }
}

/// `chamfer(EDGES, d)`, or a two-sided size measured from one face of each edge:
/// `chamfer(EDGES, d, d2: D, face: FACE)` (d along FACE, D along the other face) or
/// `chamfer(EDGES, d, angle: A, face: FACE)` (d along FACE, the cut leaving it at A);
/// `corner: "patch"|"point"` how three chamfers meet at a corner.
fn chamfer(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    use crate::feat_error::{Because, FeatError};
    const NAME: &str = "chamfer";
    not_in_sketch(ev, NAME, span)?;
    let id = ev.take_id(NAME, span)?;
    // `r` and `groups`: what the call took when it shared fillet's arguments
    let a = Args::bind(
        ev,
        NAME,
        &["edges", "d", "d2", "angle", "face", "corner", "r", "groups"],
        args,
        span,
    )?;
    let mut groups: Vec<(Value, Span, Num)> = vec![];
    if let Some(e) = a.opt("edges") {
        let d = match (a.opt("d"), a.opt("r")) {
            (Some(_), Some(_)) => {
                return Err(FeatError::Conflict {
                    span: a.span("r"),
                    func: NAME,
                    a: "d",
                    b: "r",
                }
                .into());
            }
            (None, Some(_)) => a.req_num("r", LEN)?,
            _ => a.req_num("d", LEN)?,
        };
        groups.push((e, a.span("edges"), d));
    }
    if let Some(g) = a.opt("groups") {
        let Value::List(xs) = g else {
            return Err(Error::WrongType {
                span: a.span("groups"),
                what: "groups",
                want: Ty::List,
                found: g.ty(),
            });
        };
        for x in xs.iter() {
            match x {
                Value::Tuple(t) if t.len() == 2 => {
                    let r = num_of(t[1].clone(), LEN, a.span("groups"), "a group's size")?;
                    groups.push((t[0].clone(), a.span("groups"), r));
                }
                o => {
                    return Err(Error::WrongType {
                        span: a.span("groups"),
                        what: "a group (edges, size)",
                        want: Ty::Tuple,
                        found: o.ty(),
                    });
                }
            }
        }
    }
    let Some(first) = groups.first().map(|g| g.2.clone()) else {
        return Err(Error::MissingArg {
            span,
            func: NAME,
            arg: "edges",
        });
    };
    let mut dims = vec![];
    let mut edges: Vec<Name> = vec![];
    for (k, (v, sp, d)) in groups.iter().enumerate() {
        positive(d, *sp, "a chamfer size")?;
        // one size per chamfer feature in forge-model: groups of other sizes are two chamfers
        if k > 0 && d.v != first.v {
            return Err(FeatError::Conflict {
                span: *sp,
                func: NAME,
                a: "groups",
                b: "sizes",
            }
            .into());
        }
        dims.push(dim("size", d));
        match edges_value(v.clone(), *sp)? {
            Some(es) => {
                if es.is_empty() && ev.b.building {
                    return Err(Error::EmptySelection { span: *sp });
                }
                edges.extend(es);
            }
            // what an earlier error left unknown: nothing to build; with no error before, the
            // selection found nothing on the part
            None if ev.b.building && ev.errors.is_empty() => return Err(Error::EmptySelection { span: *sp }),
            None if ev.b.building => return Ok(Value::Unresolved),
            None => {}
        }
    }
    let (d2, angle) = (a.num("d2", LEN)?, a.num("angle", ANG)?);
    if d2.is_some() && angle.is_some() {
        return Err(FeatError::Conflict {
            span: a.span("angle"),
            func: NAME,
            a: "d2",
            b: "angle",
        }
        .into());
    }
    if d2.is_some() || angle.is_some() {
        if groups.len() > 1 || a.opt("groups").is_some() {
            return Err(FeatError::NotApplicable {
                span: a.span("groups"),
                func: NAME,
                arg: "groups",
                because: Because::OtherForm,
            }
            .into());
        }
    } else if a.opt("face").is_some() {
        // a reference face measures a two-sided size only
        return Err(FeatError::NotApplicable {
            span: a.span("face"),
            func: NAME,
            arg: "face",
            because: Because::OtherForm,
        }
        .into());
    }
    let face = || -> R<Name> {
        let names = match a.opt("face") {
            None => {
                return Err(Error::MissingArg {
                    span,
                    func: NAME,
                    arg: "face",
                });
            }
            Some(Value::Faces(f)) => f.names.clone(),
            Some(Value::Unresolved) => vec![],
            Some(o) => {
                return Err(Error::WrongType {
                    span: a.span("face"),
                    what: "face",
                    want: Ty::Faces,
                    found: o.ty(),
                });
            }
        };
        match names[..] {
            [n] => Ok(n),
            _ if !ev.b.building || a.poisoned() => Ok(Name::NONE),
            _ => Err(Error::OneFace {
                span: a.span("face"),
                what: "face",
                count: names.len(),
            }),
        }
    };
    let size = match (d2, angle) {
        (Some(d2), _) => {
            positive(&d2, a.span("d2"), "a chamfer size")?;
            dims.push(dim("size2", &d2));
            ChamferSize::Distances {
                face: face()?,
                d1: first.v,
                d2: d2.v,
            }
        }
        (_, Some(angle)) => {
            if !(angle.v > 0.0 && angle.v < std::f64::consts::PI) {
                return Err(Error::BadValue {
                    span: a.span("angle"),
                    what: "a chamfer angle (in deg, between 0 and 180)",
                    value: angle.v.to_degrees(),
                });
            }
            dims.push(dim("angle", &angle));
            ChamferSize::DistanceAngle {
                face: face()?,
                d: first.v,
                angle: angle.v,
            }
        }
        _ => ChamferSize::Equal(first.v),
    };
    let corner = match crate::wired::word(&a, "corner", &["patch", "point"])? {
        Some("point") => ChamferCorner::Point,
        _ => ChamferCorner::Patch,
    };
    let f = Feature::Chamfer { edges, size, corner };
    let (idx, hid) = ev.push_feature(&id, NAME, f, span, dims)?;
    Ok(Value::Feature(Rc::new(Feat {
        id,
        kind: NAME,
        idx,
        hid,
        shape: None,
        axis: None,
        prof: None,
    })))
}

fn shell(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "shell", span)?;
    let id = ev.take_id("shell", span)?;
    let a = Args::bind(ev, "shell", &["t", "open", "out"], args, span)?;
    let t = a.req_num("t", LEN)?;
    positive(&t, a.span("t"), "a wall")?;
    let open = match a.opt("open") {
        None => vec![],
        Some(Value::Faces(f)) => f.names.clone(),
        Some(Value::Unresolved) => vec![],
        Some(o) => {
            return Err(Error::WrongType {
                span: a.span("open"),
                what: "open faces",
                want: Ty::Faces,
                found: o.ty(),
            });
        }
    };
    let f = Feature::Shell {
        thickness: t.v,
        outward: a.flag("out")?,
        open,
    };
    let (idx, hid) = ev.push_feature(&id, "shell", f, span, vec![dim("wall", &t)])?;
    Ok(Value::Feature(Rc::new(Feat {
        id,
        kind: "shell",
        idx,
        hid,
        shape: None,
        axis: None,
        prof: None,
    })))
}

// ---- anchors ----------------------------------------------------------------------------

/// `P[k]`: copy `k` of pattern `P` as the part stands (0 the original: the copied features'
/// faces), or a hole's place `k` (in the order of its points). Copies are named from the
/// pattern's id and their place, so `edges(of: P[2], ..)` names one copy's edges apart from the
/// others' however alike they are.
pub(crate) fn instance(ev: &mut Eval, f: &Rc<Feat>, k: i64, span: Span) -> R<Value> {
    let wrong = || Error::WrongType {
        span,
        what: "indexing (a pattern's copies, a hole's places)",
        want: Ty::List,
        found: Ty::Feature,
    };
    if !matches!(f.kind, "pattern" | "hole") {
        return Err(wrong());
    }
    if !ev.b.building || ev.b.built() <= f.idx {
        return Ok(Value::Unresolved);
    }
    let hist = ev.b.history();
    let (places, src) = match &ev.b.features[f.idx] {
        Feature::Pattern { of, layout } => {
            let n = forge_model::placements(hist, f.idx, layout).map_or(0, |p| p.len());
            (n + 1, Some(of.clone()))
        }
        Feature::Hole(h) => (
            match &h.places {
                Some(forge_model::At::Sketch { at, .. }) => at.len().max(1),
                _ => 1,
            },
            None,
        ),
        // a pattern or hole on other bodies (`bodies:`)
        _ => return Err(wrong()),
    };
    let out = || Error::IndexOut {
        span,
        index: k,
        len: places,
    };
    let k = usize::try_from(k).map_err(|_| out())?;
    if k >= places {
        return Err(out());
    }
    let n = ev.b.built();
    let names: Vec<Name> = match (&src, k) {
        // the original: what the copied features made
        (Some(forge_model::Source::Features(src)), 0) => {
            let origin = ev.b.origin(n);
            let body = ev.b.body().expect("built");
            body.face_ids()
                .map(|g| body.face(g).name)
                .filter(|m| origin.get(m).is_some_and(|o| src.iter().any(|s| o.contains(s))))
                .collect()
        }
        (Some(forge_model::Source::Body), 0) => {
            let origin = ev.b.origin(n);
            let body = ev.b.body().expect("built");
            body.face_ids()
                .map(|g| body.face(g).name)
                .filter(|m| origin.get(m).is_some_and(|o| o.iter().any(|&i| i < f.idx)))
                .collect()
        }
        _ => {
            let key = format!("instances {} {}", f.hid, ev.b.stages[f.idx].id);
            let (fi, hid) = (f.idx, f.hid);
            let before = f.idx.checked_sub(1).and_then(|j| ev.b.stages.get(j)).cloned();
            let made = match ev.b.memo_kept::<Rc<Vec<Vec<Name>>>>(&key) {
                Some(m) => m,
                None => {
                    let hist = ev.b.history();
                    let m = Rc::new(instance_names(
                        hist,
                        fi,
                        hid,
                        src.as_ref(),
                        places,
                        before.as_deref(),
                    ));
                    ev.b.memo_keep(key, m.clone());
                    m
                }
            };
            let wholes: std::collections::HashSet<Name> =
                made.get(k).into_iter().flatten().map(|m| m.whole()).collect();
            let body = ev.b.body().expect("built");
            let mut out: Vec<Name> = body
                .face_ids()
                .map(|g| body.face(g).name)
                .filter(|m| wholes.contains(&m.whole()))
                .collect();
            // faces of the copy joined into one with others' (one plane across them)
            for &(piece, whole) in &body.joins.aliases {
                if wholes.contains(&piece.whole()) && !out.contains(&whole) {
                    out.push(whole);
                }
            }
            out.retain(|m| body.face_ids().any(|g| body.face(g).name == *m));
            out
        }
    };
    let body = ev.b.body().expect("built");
    let ids: Vec<FaceId> = body
        .face_ids()
        .filter(|&g| names.contains(&body.face(g).name))
        .collect();
    let plane = match &ids[..] {
        [one] => face_plane(body, *one),
        _ => None,
    };
    Ok(Value::Faces(Rc::new(Faces {
        names,
        owner: format!("{}[{k}]", f.id),
        role: Role::Faces,
        plane,
    })))
}

/// `P[k].NAME`: on a copy or a place, a member of the faces it holds (`edges`, `plane`, ..);
/// else on a pattern's copy the copied feature's anchor (`anchor` of it) carried onto the copy
/// (`pegs[2].end`, `pegs[2].side(c)`), and on a hole's place the hole's anchor at that place
/// alone (`bolt[1].rim`).
pub(crate) fn copy_anchor(
    ev: &mut Eval,
    pf: &Rc<Feat>,
    k: i64,
    name: &str,
    span: Span,
    anchor: &dyn Fn(&mut Eval, &Rc<Feat>) -> R<Value>,
) -> R<Value> {
    let inst = instance(ev, pf, k, span)?;
    let Value::Faces(fs) = &inst else {
        return Ok(inst);
    };
    if name == "faces" {
        return Ok(inst);
    }
    if ["axis", "plane", "center", "normal", "edges"].contains(&name) {
        return member(ev, inst, name, span);
    }
    let k = k as usize;
    let fs = fs.clone();
    let only = |ev: &mut Eval, v: Value| -> R<Value> {
        let Some(body) = ev.b.body() else {
            return Ok(Value::Unresolved);
        };
        Ok(match v {
            Value::Faces(a) => Value::Faces(Rc::new(Faces {
                names: a.names.iter().filter(|n| fs.names.contains(n)).copied().collect(),
                owner: format!("{}[{k}]", pf.id),
                role: a.role,
                plane: a.plane,
            })),
            Value::Edges(es) => {
                let ids: HashMap<Name, EdgeId> = body.edge_ids().map(|e| (body.edge(e).name, e)).collect();
                Value::Edges(Rc::new(
                    es.iter()
                        .filter(|n| {
                            ids.get(n).is_some_and(|&e| {
                                select::faces_of_edge(body, e)
                                    .iter()
                                    .any(|&f| fs.names.contains(&body.face(f).name))
                            })
                        })
                        .copied()
                        .collect(),
                ))
            }
            o => o,
        })
    };
    if pf.kind == "hole" {
        if name == "rim"
            && let Some((o, d)) = pf.axis
        {
            return rim(ev, pf, o, d, span, Some(&fs.names));
        }
        let v = anchor(ev, pf)?;
        return only(ev, v);
    }
    // the copied features' anchor: of each that has it (a side of an entity of its profile,
    // one feature's; a role, every feature's that has one), together
    let src = ev.copied.get(&pf.hid).cloned().unwrap_or_default();
    let mut got: Option<(Vec<Name>, Role)> = None;
    let mut first_err = None;
    for s in &src {
        match anchor(ev, s) {
            Ok(Value::Faces(a)) => {
                let g = got.get_or_insert_with(|| (vec![], a.role));
                g.0.extend(a.names.iter().copied());
            }
            Ok(Value::Unresolved) => return Ok(Value::Unresolved),
            Ok(o) if src.len() == 1 && k == 0 => return Ok(o),
            Ok(o) => {
                return Err(Error::WrongType {
                    span,
                    what: "an anchor on a pattern's copy (faces: edges as FACES.edges)",
                    want: Ty::Faces,
                    found: o.ty(),
                });
            }
            Err(e) => {
                first_err.get_or_insert(e);
            }
        }
    }
    let Some((names, role)) = got else {
        return Err(first_err.unwrap_or(Error::UnknownMember {
            span,
            ty: Ty::Faces,
            member: name.into(),
        }));
    };
    let names = if k == 0 {
        names
    } else {
        copy_names(ev, pf, k, &names)
    };
    let plane = ev.b.body().and_then(|body| {
        let ids: Vec<FaceId> = body
            .face_ids()
            .filter(|&g| names.contains(&body.face(g).name))
            .collect();
        match ids[..] {
            [one] => face_plane(body, one),
            _ => None,
        }
    });
    Ok(Value::Faces(Rc::new(Faces {
        names,
        owner: format!("{}[{k}]", pf.id),
        role,
        plane,
    })))
}

/// The faces of pattern `pf`'s copy `k` (from 1) that are the copies of the faces `names` of
/// the original: each named as the kernel names a copy (`forge_model::instance_name`), pieces and
/// joins followed.
fn copy_names(ev: &Eval, pf: &Feat, k: usize, names: &[Name]) -> Vec<Name> {
    let Some(body) = ev.b.body() else {
        return vec![];
    };
    let key = match &ev.b.features[pf.idx] {
        Feature::Pattern {
            layout: forge_model::Layout::Linear { first, .. },
            ..
        } => {
            let rows = first.count.max(1);
            [(k % rows) as u64, (k / rows) as u64]
        }
        _ => [k as u64, 0],
    };
    let mut from: std::collections::HashSet<Name> = names.iter().map(|n| n.whole()).collect();
    for &(piece, whole) in &body.joins.aliases {
        if names.contains(&whole) {
            from.insert(piece.whole());
        }
    }
    let to: std::collections::HashSet<Name> = from
        .iter()
        .map(|&n| forge_model::instance_name(pf.hid, key, n).whole())
        .collect();
    let mut out: Vec<Name> = body
        .face_ids()
        .map(|g| body.face(g).name)
        .filter(|m| to.contains(&m.whole()))
        .collect();
    for &(piece, whole) in &body.joins.aliases {
        if to.contains(&piece.whole())
            && !out.contains(&whole)
            && body.face_ids().any(|g| body.face(g).name == whole)
        {
            out.push(whole);
        }
    }
    out
}

/// Each place's face names as the kernel names them (place 0 the original), from the tools it
/// moves into place: a hole's places, or a pattern's copies of its features (each copy of the
/// whole part named from the part before it, `before`).
fn instance_names(
    hist: forge_model::History<'_>,
    i: usize,
    hid: u64,
    src: Option<&forge_model::Source>,
    places: usize,
    before: Option<&crate::build::Stage>,
) -> Vec<Vec<Name>> {
    let faces = |b: &Body| -> Vec<Name> { b.face_ids().map(|g| b.face(g).name).collect() };
    match src {
        Some(forge_model::Source::Body) => {
            let names: Vec<Name> = before
                .and_then(|b| b.work())
                .map(|w| w.faces().map(|(b, g)| b.face(g).name).collect())
                .unwrap_or_default();
            let Feature::Pattern { layout, .. } = &hist[i] else {
                return vec![];
            };
            let rows = match layout {
                forge_model::Layout::Linear { first, .. } => first.count.max(1),
                _ => usize::MAX,
            };
            let mut out = vec![names.clone()];
            for k in 1..places {
                let key = if rows == usize::MAX {
                    [k as u64, 0]
                } else {
                    [(k % rows) as u64, (k / rows) as u64]
                };
                out.push(
                    names
                        .iter()
                        .map(|&m| forge_model::instance_name(hid, key, m))
                        .collect(),
                );
            }
            out
        }
        _ => {
            let Ok(tools) = forge_model::feature_tools(hist, i) else {
                return vec![];
            };
            let mut out: Vec<Vec<Name>> = match src {
                // the copies alone, each the copied features' tools in turn
                Some(_) => {
                    let per = tools.len() / (places - 1).max(1);
                    let mut o = vec![vec![]];
                    for c in tools.chunks(per.max(1)) {
                        o.push(c.iter().flat_map(|t| faces(&t.0)).collect());
                    }
                    o
                }
                None => tools.iter().map(|t| faces(&t.0)).collect(),
            };
            out.resize(places, vec![]);
            out
        }
    }
}

pub(crate) fn face_plane(body: &Body, f: FaceId) -> Option<Frame> {
    let face = body.face(f);
    match &face.surface {
        Surface::Plane(p) if face.same_sense => Some(p.frame),
        Surface::Plane(p) => Some(p.frame.flipped()),
        _ => None,
    }
}

/// A frame on face plane `fp`: z its outward normal, x the reference's x laid in the plane,
/// origin the reference origin dropped onto it.
pub(crate) fn carried(fp: Frame, reference: Frame) -> Frame {
    let z = fp.z();
    let o = reference.origin - z.get() * (reference.origin - fp.origin).dot(z.get());
    let x = reference.x().get();
    let x = if x.cross(z.get()).length() < 1e-9 {
        reference.y().get()
    } else {
        x
    };
    Frame::from_zx(o, z, x).unwrap_or(fp)
}

pub(crate) fn role_set(ev: &mut Eval, f: &Feat, role: Role, names: &[Name], frame: Option<Frame>) -> Value {
    if !ev.b.building || ev.b.built() <= f.idx {
        return Value::Faces(Rc::new(Faces {
            names: vec![],
            owner: f.id.clone(),
            role,
            plane: frame,
        }));
    }
    let body = ev.b.body().expect("built");
    let mut faces = vec![];
    for &n in names {
        faces.extend(crate::build::role_faces(body, n));
    }
    let plane = frame.or_else(|| {
        let reference = f.shape.as_ref().map_or(Frame::WORLD, |s| s.sketch.borrow().frame);
        faces
            .first()
            .and_then(|&x| face_plane(body, x))
            .map(|fp| carried(fp, reference))
    });
    Value::Faces(Rc::new(Faces {
        names: faces.iter().map(|&x| body.face(x).name).collect(),
        owner: f.id.clone(),
        role,
        plane,
    }))
}

fn cap(ev: &mut Eval, f: &Feat, end: bool) -> Value {
    let sh = f.shape.clone().expect("an extrude");
    let sf = sh.sketch.borrow().frame;
    let n = sf.z();
    let at = if end { sh.far } else { Some(sh.near) };
    let toward = sh
        .far
        .map_or(if sh.cut { -1.0 } else { 1.0 }, |x| (x - sh.near).signum());
    // start: out of the part, back past the sketch; end: on, for an add; back, for a cut's floor
    let z_sign = if !end || sh.cut { -toward } else { toward };
    let frame = at.map(|d| {
        let o = sf.origin + n * d;
        let fr = sf.with_origin(o);
        if z_sign < 0.0 { fr.flipped() } else { fr }
    });
    let root = Name::feature(f.hid);
    // which of the tool's caps lies where: the one the kernel names "start" sits at the sketch
    // unless the extrude ran backwards. Asked of the part as the feature left it, so a cap a
    // later edit moved (an offset, a draft, a slab) is still found by its name.
    let own_caps = [forge_ops::roles::start(root), forge_ops::roles::end(root)];
    // a several-region extrude's (a text's): each region's
    let region_caps: Vec<Name> = region_roots(ev, f).map_or(Vec::new(), |rs| {
        rs.iter()
            .flat_map(|r| [forge_ops::roles::start(r.0), forge_ops::roles::end(r.0)])
            .collect()
    });
    let names: &[Name] = if region_caps.is_empty() {
        &own_caps
    } else {
        &region_caps
    };
    let at_height = |body: &Body, fr: Frame, nm: Name| {
        crate::build::role_faces(body, nm).into_iter().any(|x| {
            face_plane(body, x).is_some_and(|fp| (fp.origin - fr.origin).dot(fr.z().get()).abs() < 1e-6)
        })
    };
    let own = ev.b.stages.get(f.idx).cloned();
    let picked: Vec<Name> = match (frame, own.as_ref().and_then(|s| s.view())) {
        (Some(fr), Some(body)) => names
            .iter()
            .copied()
            .filter(|&nm| at_height(body, fr, nm))
            .collect(),
        _ => names.to_vec(),
    };
    let v = role_set(ev, f, if end { Role::End } else { Role::Start }, &picked, frame);
    // its plane where the face is now
    match (&v, frame, ev.b.body()) {
        (Value::Faces(fs), Some(fr), Some(body)) => {
            let now = fs
                .names
                .first()
                .and_then(|n| body.face_ids().find(|&x| body.face(x).name == *n))
                .and_then(|x| face_plane(body, x));
            match now {
                Some(fp) if (fp.origin - fr.origin).dot(fr.z().get()).abs() >= 1e-6 => {
                    Value::Faces(Rc::new(Faces {
                        plane: Some(carried(fp, fr)),
                        ..(**fs).clone()
                    }))
                }
                _ => v,
            }
        }
        _ => v,
    }
}

/// A local param of feature `id` read from outside it.
pub(crate) fn feature_local(ev: &mut Eval, id: &str, name: &str, span: Span) -> Option<R<Value>> {
    let p = ev
        .locals
        .iter()
        .find(|p| p.key.name == name && crate::error::path_text(&p.key.owner) == id)?;
    let key = p.key.clone();
    let k = ev.locals.iter().position(|q| q.key == key).expect("found");
    if let Err(e) = ev.out_of_scope_rule(key, span) {
        return Some(Err(e));
    }
    ev.locals[k].used = true;
    Some(Ok(match &ev.locals[k].value {
        Value::Num(n) => Value::Num(Num {
            v: n.v,
            dim: n.dim,
            src: Src::driving(ev.locals[k].key.clone()),
        }),
        o => o.clone(),
    }))
}

pub fn member(ev: &mut Eval, v: Value, name: &str, span: Span) -> R<Value> {
    let unknown = |v: &Value| Error::UnknownMember {
        span,
        ty: v.ty(),
        member: name.into(),
    };
    match &v {
        Value::Sheet(s) => crate::sheet::member(ev, &s.clone(), name, span),
        Value::Feature(f) => {
            if let Some(v) = feature_local(ev, &f.id, name, span) {
                return v;
            }
            if let Some(v) = crate::shapes::member(ev, f, name) {
                return Ok(v);
            }
            if let Some(v) = crate::weld::member(ev, f, name) {
                return Ok(v);
            }
            match name {
                "start" | "end" | "floor" | "mouth" if f.shape.is_some() => {
                    Ok(cap(ev, f, name == "end" || name == "floor"))
                }
                "axis" if f.axis.is_some() => {
                    let (o, d) = f.axis.expect("checked");
                    Ok(Value::Axis(o, d))
                }
                "faces" => Ok(feature_faces(ev, f)),
                "edges" => Ok(feature_edges(ev, &[f.idx])),
                "rim" if f.axis.is_some() => {
                    let (o, d) = f.axis.expect("checked");
                    rim(ev, f, o, d, span, None)
                }
                _ => crate::wired::feature_member(ev, f, name, span).unwrap_or_else(|| Err(unknown(&v))),
            }
        }
        Value::Sketch(s) => {
            let sk = s.borrow();
            match name {
                "plane" => Ok(Value::Plane(Rc::new(Plane {
                    frame: sk.frame,
                    world: None,
                }))),
                "origin" => Ok(Value::Point(sk.frame.origin)),
                "points" => Ok(Value::List(Rc::new(
                    sk.points.iter().map(|&p| mk_ent(s, p)).collect(),
                ))),
                _ => sk
                    .named
                    .iter()
                    .find(|x| x.0 == name)
                    .map(|x| x.1.clone())
                    .ok_or_else(|| unknown(&v)),
            }
        }
        Value::Ent(e) => {
            let sk = e.sk.borrow();
            let ent = sk.s.entity(e.id);
            let pt = |id: EntityId| {
                Value::Ent(Ent {
                    sk: e.sk.clone(),
                    id,
                    kind: EntKind::Point,
                    loop_segs: Rc::new(vec![]),
                })
            };
            use forge_sketch::Entity as E;
            match (name, ent) {
                ("segs", _) if !e.loop_segs.is_empty() => Ok(Value::List(Rc::new(
                    e.loop_segs.iter().map(|&x| mk_ent(&e.sk, x)).collect(),
                ))),
                ("x" | "y", Some(E::Point { .. })) => {
                    let p = sk.pos(e.id);
                    Ok(Value::Num(Num {
                        v: if name == "x" { p.x } else { p.y },
                        dim: Dim::Length,
                        src: Src::Const,
                    }))
                }
                ("a", Some(E::Line { a, .. } | E::Arc { start: a, .. })) => Ok(pt(a)),
                ("b", Some(E::Line { b, .. } | E::Arc { end: b, .. })) => Ok(pt(b)),
                ("center", Some(E::Circle { center, .. } | E::Arc { center, .. })) => Ok(pt(center)),
                // through the centre, square to the sketch: a hole's axis, for an assembly's checks
                ("axis", Some(E::Circle { center, .. } | E::Arc { center, .. })) => {
                    Ok(Value::Axis(sk.frame.plane_point(sk.pos(center)), sk.frame.z()))
                }
                ("dir", Some(E::Line { a, b })) => {
                    let d = sk.pos(b) - sk.pos(a);
                    Ok(Value::Dir(sk.frame.vec_to_world(Vec3::new(d.x, d.y, 0.0))))
                }
                _ => Err(unknown(&v)),
            }
        }
        Value::Faces(fs) => match name {
            "axis" => {
                let Some(body) = ev.b.body().filter(|_| !fs.names.is_empty()) else {
                    return Ok(Value::Unresolved);
                };
                let f = body
                    .face_ids()
                    .find(|&x| body.face(x).name == fs.names[0])
                    .expect("named");
                match &body.face(f).surface {
                    Surface::Cylinder(c) => Ok(Value::Axis(c.frame.origin, c.frame.z())),
                    Surface::Cone(c) => Ok(Value::Axis(c.frame.origin, c.frame.z())),
                    _ => Err(Error::NotPlanar {
                        span,
                        id: fs.owner.clone(),
                        role: fs.role,
                    }),
                }
            }
            "plane" => Ok(Value::Plane(Rc::new(Plane {
                frame: plane_of(ev, v.clone(), span, "plane")?,
                world: None,
            }))),
            "center" => face_center(ev, fs, span),
            "normal" => Ok(Value::Dir(plane_of(ev, v.clone(), span, "normal")?.z().get())),
            "edges" => {
                if !ev.b.building || (fs.names.is_empty() && !ev.errors.is_empty()) {
                    return Ok(Value::Unresolved);
                }
                let body = ev.b.body().expect("built");
                let mut out: Vec<Name> = vec![];
                for f in body.face_ids().filter(|&f| fs.names.contains(&body.face(f).name)) {
                    for e in select::edges_of_face(body, f) {
                        let n = body.edge(e).name;
                        if !out.contains(&n) {
                            out.push(n);
                        }
                    }
                }
                Ok(Value::Edges(Rc::new(out)))
            }
            _ => Err(unknown(&v)),
        },
        Value::Edges(es) if matches!(name, "center" | "start" | "end") => edge_point(ev, es, name, span),
        Value::Tuple(t) if t.len() == 2 => match name {
            "x" => Ok(t[0].clone()),
            "y" => Ok(t[1].clone()),
            _ => Err(unknown(&v)),
        },
        Value::Plane(p) => match name {
            "normal" => Ok(Value::Dir(p.frame.z().get())),
            "origin" => Ok(Value::Point(p.frame.origin)),
            _ => Err(unknown(&v)),
        },
        Value::Map(m) => m
            .iter()
            .find(|(k, _)| *k == crate::syntax::Key::Label(name.into()))
            .map(|x| x.1.clone())
            .ok_or_else(|| unknown(&v)),
        Value::Wired(w) => crate::wired::member(ev, w, name, span).unwrap_or_else(|| Err(unknown(&v))),
        Value::Asm(a) => crate::asm::member(a, name, span),
        Value::Unresolved => Ok(Value::Unresolved),
        _ => Err(unknown(&v)),
    }
}

/// Each loop of a face as points in its walking order (lines by their ends, curves sampled).
fn loop_points(body: &Body, f: FaceId) -> Vec<Vec<Point3>> {
    let mut out = vec![];
    for &lp in body.face(f).loops.iter() {
        let mut pts: Vec<Point3> = vec![];
        for c in body.lp(lp).coedges() {
            let ce = body.coedge(c);
            let n = if matches!(body.edge(ce.edge).curve, Curve::Line(_)) {
                1
            } else {
                64
            };
            let mut p = body.edge_points(ce.edge, n);
            if !ce.forward {
                p.reverse();
            }
            pts.extend(p.into_iter().skip(usize::from(!pts.is_empty())));
        }
        out.push(pts);
    }
    out
}

/// Where `F.center` is: planar faces' area centroid in plane `fr`, as `fr` moved there.
pub fn faces_centroid(body: &Body, faces: &[FaceId], fr: &Frame) -> Option<Frame> {
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for &f in faces {
        for lp in loop_points(body, f) {
            for w in lp.windows(2) {
                let (p, q) = (fr.point_to_local(w[0]), fr.point_to_local(w[1]));
                let c = p.x * q.y - q.x * p.y;
                a += c;
                cx += (p.x + q.x) * c;
                cy += (p.y + q.y) * c;
            }
        }
    }
    (a.abs() >= 1e-12)
        .then(|| fr.with_origin(fr.point_to_world(Point3::new(cx / (3.0 * a), cy / (3.0 * a), 0.0))))
}

/// Where `F.center` is on cylindrical or conical faces: the axis, its origin halfway along them.
pub fn faces_axis_middle(body: &Body, faces: &[FaceId]) -> Option<(Point3, UnitVec3)> {
    let ax = match &body.face(*faces.first()?).surface {
        Surface::Cylinder(c) => c.frame,
        Surface::Cone(c) => c.frame,
        _ => return None,
    };
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for &f in faces {
        for p in loop_points(body, f).into_iter().flatten() {
            let t = (p - ax.origin).dot(ax.z().get());
            lo = lo.min(t);
            hi = hi.max(t);
        }
    }
    (lo <= hi).then(|| (ax.origin + ax.z().get() * ((lo + hi) * 0.5), ax.z()))
}

/// `F.center`: a planar face set's plane with its origin at the faces' area centroid (x as the
/// plane's); a cylinder's or cone's axis with its origin halfway along the face.
fn face_center(ev: &mut Eval, fs: &Faces, span: Span) -> R<Value> {
    let Some(body) = ev.b.body().filter(|_| !fs.names.is_empty()) else {
        return Ok(Value::Unresolved);
    };
    let faces: Vec<FaceId> = body
        .face_ids()
        .filter(|&f| fs.names.contains(&body.face(f).name))
        .collect();
    let got = match &fs.plane {
        Some(fr) => {
            faces_centroid(body, &faces, fr).map(|frame| Value::Plane(Rc::new(Plane { frame, world: None })))
        }
        None => faces_axis_middle(body, &faces).map(|(o, d)| Value::Axis(o, d)),
    };
    got.ok_or_else(|| Error::NotPlanar {
        span,
        id: fs.owner.clone(),
        role: fs.role,
    })
}

/// `EDGES.center` (a circle's or arc's centre on its axis, a line's midpoint, a curve's middle),
/// `EDGES.start`, `EDGES.end`: of one edge.
fn edge_point(ev: &mut Eval, es: &[Name], name: &str, span: Span) -> R<Value> {
    let Some(body) = ev.b.body() else {
        return Ok(Value::Unresolved);
    };
    let [n] = es else {
        return Err(Error::BadValue {
            span,
            what: "edges in a set a point is taken of (one)",
            value: es.len() as f64,
        });
    };
    let Some(e) = body.edge_by_name(*n) else {
        return Ok(Value::Unresolved);
    };
    Ok(match edge_snap(body, e, name) {
        Ok(p) => Value::Point(p),
        Err((o, d)) => Value::Axis(o, d),
    })
}

/// What `EDGES.center|start|end` gives for one edge: a point, or a circle's axis (Err).
pub fn edge_snap(body: &Body, e: EdgeId, name: &str) -> Result<Point3, (Point3, UnitVec3)> {
    let edge = body.edge(e);
    match (name, &edge.curve) {
        ("start", _) => Ok(body.vertex(edge.start).point),
        ("end", _) => Ok(body.vertex(edge.end).point),
        (_, Curve::Circle(c)) => Err((c.frame.origin, c.frame.z())),
        (_, Curve::Line(_)) => {
            let (a, b) = (body.vertex(edge.start).point, body.vertex(edge.end).point);
            Ok(a + (b - a) * 0.5)
        }
        _ => Ok(forge_geom::CurveGeom::eval(
            &edge.curve,
            edge.range.lo + edge.range.width() * 0.5,
        )),
    }
}

/// `E.side(entity)`: the face an entity of the profile made.
/// A several-region extrude's (a text's) regions: each one's root name and the segment ids of
/// its loops. None for any other feature.
pub(crate) fn region_roots(ev: &Eval, f: &Feat) -> Option<Vec<(Name, Vec<u64>)>> {
    let Some(Feature::Regions(r)) = crate::holes::kernel_feature(ev, f.idx) else {
        return None;
    };
    const PIECE: u64 = 0xffff << 48;
    Some(
        r.profiles
            .iter()
            .filter_map(|p| {
                let key = forge_model::regions::key(p)?;
                let segs = p
                    .loops
                    .iter()
                    .flat_map(|l| l.segments.iter().map(|s| s.1 & !PIECE))
                    .collect();
                Some((Name::feature(forge_model::regions::region_id(f.hid, key)), segs))
            })
            .collect(),
    )
}

/// `E.side(X)` of an entity or loop, or of a list of them (a text): every face they made.
pub fn side_of(ev: &mut Eval, f: &Feat, v: Value, span: Span) -> R<Value> {
    let Value::List(xs) = v else {
        let Value::Ent(e) = v else {
            return Err(Error::WrongType {
                span,
                what: "side(entity)",
                want: Ty::Entity,
                found: v.ty(),
            });
        };
        return side(ev, f, &e, span);
    };
    let mut out: Option<Faces> = None;
    for x in xs.iter() {
        let Value::Ent(e) = x else {
            return Err(Error::WrongType {
                span,
                what: "side(entity)",
                want: Ty::Entity,
                found: x.ty(),
            });
        };
        match side(ev, f, e, span)? {
            Value::Faces(fs) => match &mut out {
                None => out = Some((*fs).clone()),
                Some(o) => {
                    o.names.extend(fs.names.iter().copied());
                    o.plane = None;
                }
            },
            other => return Ok(other),
        }
    }
    Ok(out.map_or(Value::Unresolved, |o| Value::Faces(Rc::new(o))))
}

pub fn side(ev: &mut Eval, f: &Feat, e: &Ent, span: Span) -> R<Value> {
    let sk = f.prof.clone().expect("a profile feature");
    if !Rc::ptr_eq(&sk, &e.sk) {
        return Err(Error::OtherSketch {
            span,
            sketch: e.sk.borrow().id.clone(),
            here: sk.borrow().id.clone(),
        });
    }
    let ids: Vec<EntityId> = if e.loop_segs.is_empty() {
        vec![e.id]
    } else {
        e.loop_segs.to_vec()
    };
    let root = Name::feature(f.hid);
    let segs: Vec<u64> = ids.iter().map(|&i| sk.borrow().seg_id(i)).collect();
    let regions = region_roots(ev, f);
    let names: Vec<Name> = match crate::shapes::side_names(ev, f, &segs) {
        Some(n) => n,
        None => ids
            .iter()
            .flat_map(|&i| {
                let seg = sk.borrow().seg_id(i);
                // a several-region extrude's wall is named under its region's root
                let root = regions
                    .as_ref()
                    .and_then(|rs| rs.iter().find(|r| r.1.contains(&seg)))
                    .map_or(root, |r| r.0);
                // a revolve's faces are named per part of the turn as well
                if f.kind == "revolve" {
                    vec![
                        forge_ops::roles::revolve_side(root, seg, 0),
                        forge_ops::roles::revolve_side(root, seg, 1),
                    ]
                } else {
                    vec![forge_ops::roles::side(root, seg)]
                }
            })
            .collect(),
    };
    // the frame: origin at the entity's start, x along it
    let reference = {
        let s = sk.borrow();
        match s.s.entity(ids[0]) {
            Some(forge_sketch::Entity::Line { a, b }) => {
                let (pa, pb) = (s.world(s.pos(a)), s.world(s.pos(b)));
                Frame::from_zx(pa, s.frame.z(), pb - pa).unwrap_or(s.frame)
            }
            _ => s.frame,
        }
    };
    let v = role_set(ev, f, Role::Side, &names, None);
    Ok(match v {
        Value::Faces(fs) => {
            let plane = match (ev.b.body(), fs.names.first()) {
                (Some(body), Some(n)) => body
                    .face_ids()
                    .find(|&x| body.face(x).name == *n)
                    .and_then(|x| face_plane(body, x))
                    .map(|fp| {
                        let z = fp.z();
                        let o = reference.origin - z.get() * (reference.origin - fp.origin).dot(z.get());
                        Frame::from_zx(o, z, reference.x().get()).unwrap_or(fp)
                    }),
                _ => None,
            };
            Value::Faces(Rc::new(Faces {
                plane,
                ..(*fs).clone()
            }))
        }
        o => o,
    })
}

pub(crate) fn feature_faces(ev: &mut Eval, f: &Feat) -> Value {
    if !ev.b.building || ev.b.built() <= f.idx {
        return Value::Unresolved;
    }
    let n = ev.b.built();
    let origin = ev.b.origin(n);
    let body = ev.b.body().expect("built");
    let names: Vec<Name> = body
        .face_ids()
        .map(|x| body.face(x).name)
        .filter(|nm| origin.get(nm).is_some_and(|o| o.contains(&f.idx)))
        .collect();
    Value::Faces(Rc::new(Faces {
        names,
        owner: f.id.clone(),
        role: Role::Faces,
        plane: None,
    }))
}

pub(crate) fn feature_edges(ev: &mut Eval, idx: &[usize]) -> Value {
    if !ev.b.building || idx.iter().any(|&i| ev.b.built() <= i) {
        return Value::Unresolved;
    }
    let n = ev.b.built();
    let origin = ev.b.origin(n);
    let body = ev.b.body().expect("built");
    let mut out = vec![];
    for e in body.edge_ids() {
        let by = select::faces_of_edge(body, e).into_iter().any(|f| {
            origin
                .get(&body.face(f).name)
                .is_some_and(|o| o.iter().any(|x| idx.contains(x)))
        });
        if by {
            out.push(body.edge(e).name);
        }
    }
    Value::Edges(Rc::new(out))
}

/// A hole's or bore's rim: the loop of edges nearest its mouth where its faces meet faces it did
/// not make (any curve: a hole drilled into a blend has a mouth that is no circle). A mouth a
/// later fillet rounded has only smooth edges there: chamfering it is the typed `MouthRounded`.
/// `only`: the mouth among these faces of the feature's (one place of a hole: `bolt[1].rim`).
fn rim(ev: &mut Eval, f: &Feat, o: Point3, d: UnitVec3, span: Span, only: Option<&[Name]>) -> R<Value> {
    if !ev.b.building || ev.b.built() <= f.idx {
        return Ok(Value::Unresolved);
    }
    let n = ev.b.built();
    let origin = ev.b.origin(n);
    let body = ev.b.body().expect("built");
    let made = |x: FaceId| {
        let n = body.face(x).name;
        origin.get(&n).is_some_and(|o| o.contains(&f.idx)) && only.is_none_or(|ns| ns.contains(&n))
    };
    // the boundary: one face the feature's, the other not
    let mut bound: Vec<(EdgeId, f64)> = vec![];
    for e in body.edge_ids() {
        let fs = select::faces_of_edge(body, e);
        if fs.iter().any(|&x| made(x)) && fs.iter().any(|&x| !made(x)) {
            let edge = body.edge(e);
            let mid = forge_geom::CurveGeom::eval(&edge.curve, edge.range.mid());
            bound.push((e, (mid - o).dot(d.get())));
        }
    }
    let Some(&(first, _)) = bound.iter().min_by(|a, b| a.1.total_cmp(&b.1)) else {
        return Ok(Value::Edges(Rc::new(vec![])));
    };
    // the loop through the nearest edge: boundary edges joined at their ends
    let ends = |e: EdgeId| [body.edge(e).start, body.edge(e).end];
    let mut mouth = vec![first];
    let mut grew = true;
    while grew {
        grew = false;
        for &(e, _) in &bound {
            if !mouth.contains(&e) && mouth.iter().any(|&m| ends(m).iter().any(|v| ends(e).contains(v))) {
                mouth.push(e);
                grew = true;
            }
        }
    }
    if mouth.iter().all(|&e| select::crease(body, e) == Some("smooth")) {
        return Err(Error::MouthRounded {
            span,
            id: f.id.clone(),
        });
    }
    Ok(Value::Edges(Rc::new(
        mouth.iter().map(|&e| body.edge(e).name).collect(),
    )))
}

// ---- selections -------------------------------------------------------------------------

pub(crate) fn word(a: &Args, k: &'static str, allowed: &[&'static str]) -> R<Option<&'static str>> {
    match a.opt(k) {
        None => Ok(None),
        Some(Value::Str(s)) => {
            allowed
                .iter()
                .find(|w| ***w == *s)
                .copied()
                .map(Some)
                .ok_or(Error::BadValue {
                    span: a.span(k),
                    what: k,
                    value: f64::NAN,
                })
        }
        Some(o) => Err(Error::WrongType {
            span: a.span(k),
            what: k,
            want: Ty::Str,
            found: o.ty(),
        }),
    }
}

pub(crate) fn feats_of(v: Value, span: Span) -> R<Vec<Rc<Feat>>> {
    match v {
        Value::Feature(f) => Ok(vec![f]),
        Value::Map(m) => {
            let mut out = vec![];
            for x in m.iter() {
                out.extend(feats_of(x.1.clone(), span)?);
            }
            Ok(out)
        }
        Value::List(xs) => {
            let mut out = vec![];
            for x in xs.iter() {
                out.extend(feats_of(x.clone(), span)?);
            }
            Ok(out)
        }
        o => Err(Error::WrongType {
            span,
            what: "features",
            want: Ty::Feature,
            found: o.ty(),
        }),
    }
}

fn selection(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, name, span)?;
    let a = Args::bind(
        ev,
        name,
        &[
            "of", "between", "crease", "shape", "along", "normal", "r", "longer", "shorter", "extreme",
            "r_below", "r_above", "touching", "body",
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    // faces given (an anchor, a pattern's copy): the faces among them, or their edges
    let mut given: Option<Vec<Name>> = None;
    let of: Option<Vec<Rc<Feat>>> = match a.opt("of") {
        None => None,
        Some(Value::Faces(fs)) if name == "edges" => {
            let v = member(ev, Value::Faces(fs), "edges", span)?;
            return filter_edges(ev, &a, v);
        }
        Some(Value::Faces(fs)) => {
            given = Some(fs.names.clone());
            None
        }
        Some(v) => Some(feats_of(v, a.span("of"))?),
    };
    let side_of = |v: Value| -> R<By> {
        match v {
            Value::Faces(fs) => Ok(By::Faces(fs.names.clone())),
            v => Ok(By::Feats(feats_of(v, a.span("between"))?)),
        }
    };
    let between: Option<(By, By)> = match a.opt("between") {
        None => None,
        Some(Value::Tuple(t)) | Some(Value::List(t)) if t.len() == 2 => {
            Some((side_of(t[0].clone())?, side_of(t[1].clone())?))
        }
        Some(o) => {
            return Err(Error::WrongType {
                span: a.span("between"),
                what: "between (two features)",
                want: Ty::Tuple,
                found: o.ty(),
            });
        }
    };
    let crease = word(&a, "crease", &["concave", "convex", "smooth"])?;
    let shape = word(
        &a,
        "shape",
        &[
            "line", "circle", "curve", "plane", "cyl", "cone", "sphere", "torus", "blend", "spline",
        ],
    )?;
    let along = a.opt("along").map(|v| dir_of(v, a.span("along"))).transpose()?;
    let normal = a.opt("normal").map(|v| dir_of(v, a.span("normal"))).transpose()?;
    let r = a.num("r", LEN)?;
    let longer = a.num("longer", LEN)?;
    let shorter = a.num("shorter", LEN)?;
    let extreme = a
        .opt("extreme")
        .map(|v| dir_of(v, a.span("extreme")))
        .transpose()?;
    if extreme.is_some() {
        let err = Error::PositionalSelection {
            span: a.span("extreme"),
            rule: crate::error::Rule::NoPositionalSelection,
        };
        ev.rule(ev.rules.no_positional_selection, err)?;
    }
    for k in ["r", "longer", "shorter", "r_below", "r_above"] {
        if let Some(Value::Num(n)) = a.opt(k) {
            ev.observe(&n);
        }
    }
    if !ev.b.building || ev.b.built() == 0 {
        return Ok(Value::Unresolved);
    }
    let unbuilt = |b: &By| matches!(b, By::Feats(fs) if fs.iter().any(|f| ev.b.built() <= f.idx));
    if of.iter().flatten().any(|f| ev.b.built() <= f.idx)
        || between.as_ref().is_some_and(|(x, y)| unbuilt(x) || unbuilt(y))
    {
        return Ok(Value::Unresolved);
    }
    let n = ev.b.built();
    let origin = ev.b.origin(n);
    let body = ev.b.body().expect("built");
    let made = |f: FaceId, set: &[Rc<Feat>]| {
        origin
            .get(&body.face(f).name)
            .is_some_and(|o| set.iter().any(|x| o.contains(&x.idx)))
    };
    let centre = |e: select::Ent| select::centre(body, e);
    if name == "faces" {
        let mut fs: Vec<FaceId> = body.face_ids().collect();
        if let Some(of) = &of {
            fs.retain(|&f| made(f, of));
        }
        if let Some(g) = &given {
            fs.retain(|&f| g.contains(&body.face(f).name));
        }
        if let Some(s) = shape {
            fs.retain(|&f| select::surface_kind(&body.face(f).surface) == s);
        }
        if let Some(nv) = normal {
            let nv = nv.normalize().map_or(Vec3::Z, |u| u.get());
            fs.retain(|&f| select::plane_normal(body, f).is_some_and(|m| m.dot(nv) > 1.0 - 1e-9));
        }
        if let Some(r) = &r {
            fs.retain(|&f| {
                select::radius(body, select::Ent::Face(f))
                    .is_some_and(|x| (x - r.v).abs() <= 1e-6 * r.v.max(1.0))
            });
        }
        let more = more_filters(ev, &a)?;
        fs.retain(|&f| more.keeps(body, select::Ent::Face(f)));
        if let Some(d) = extreme {
            let at = |f: FaceId| (centre(select::Ent::Face(f)) - Point3::ORIGIN).dot(d);
            let top = fs.iter().map(|&f| at(f)).fold(f64::NEG_INFINITY, f64::max);
            fs.retain(|&f| at(f) >= top - 1e-6 * top.abs().max(1.0));
        }
        let names: Vec<Name> = fs.iter().map(|&f| body.face(f).name).collect();
        let plane = match &fs[..] {
            [one] => face_plane(body, *one),
            _ => None,
        };
        return Ok(Value::Faces(Rc::new(Faces {
            names,
            owner: of
                .as_ref()
                .and_then(|o| o.first())
                .map_or("part".into(), |f| f.id.clone()),
            role: Role::Faces,
            plane,
        })));
    }
    let mut es: Vec<EdgeId> = body.edge_ids().collect();
    if let Some(of) = &of {
        es.retain(|&e| select::faces_of_edge(body, e).into_iter().any(|f| made(f, of)));
    }
    if let Some((x, y)) = &between {
        let by = |f: FaceId, b: &By| match b {
            By::Feats(fs) => made(f, fs),
            By::Faces(ns) => ns.contains(&body.face(f).name),
        };
        es.retain(|&e| match &select::faces_of_edge(body, e)[..] {
            [f, g] => (by(*f, x) && by(*g, y)) || (by(*f, y) && by(*g, x)),
            _ => false,
        });
    }
    let v = Value::Edges(Rc::new(es.iter().map(|&e| body.edge(e).name).collect()));
    let _ = (crease, shape, along, r, longer, shorter, extreme);
    filter_edges(ev, &a, v)
}

/// The selection filters faces and edges share beyond the first set: radius under or over a
/// value (`r_below`, `r_above`), touching given faces or edges (`touching`: the faces and edges
/// that share an edge or a vertex with them), and in a body (`body: F`).
struct More {
    below: Option<f64>,
    above: Option<f64>,
    touching: Option<std::collections::HashSet<Name>>,
    /// The chosen bodies' faces and edges.
    body: Option<std::collections::HashSet<Name>>,
}

impl More {
    fn keeps(&self, body: &Body, e: select::Ent) -> bool {
        if let Some(keep) = &self.body {
            let n = match e {
                select::Ent::Face(f) => body.face(f).name,
                select::Ent::Edge(g) => body.edge(g).name,
                select::Ent::Vert(_) => return false,
            };
            if !keep.contains(&n) {
                return false;
            }
        }
        let r = || select::radius(body, e);
        if self.below.is_some_and(|b| !r().is_some_and(|x| x < b)) {
            return false;
        }
        if self.above.is_some_and(|b| !r().is_some_and(|x| x > b)) {
            return false;
        }
        if let Some(t) = &self.touching {
            let near = |x: select::Ent| match x {
                select::Ent::Face(f) => t.contains(&body.face(f).name),
                select::Ent::Edge(g) => t.contains(&body.edge(g).name),
                select::Ent::Vert(_) => false,
            };
            // what the entity's own boundary and neighbours are
            let verts = |g: EdgeId| [body.edge(g).start, body.edge(g).end];
            let hit = match e {
                select::Ent::Face(f) => select::edges_of_face(body, f).into_iter().any(|g| {
                    near(select::Ent::Edge(g))
                        || select::faces_of_edge(body, g)
                            .into_iter()
                            .any(|h| h != f && near(select::Ent::Face(h)))
                }),
                select::Ent::Edge(g) => {
                    select::faces_of_edge(body, g)
                        .into_iter()
                        .any(|h| near(select::Ent::Face(h)))
                        || body.edge_ids().any(|h| {
                            h != g
                                && near(select::Ent::Edge(h))
                                && verts(h).iter().any(|v| verts(g).contains(v))
                        })
                }
                select::Ent::Vert(_) => false,
            };
            if !hit {
                return false;
            }
        }
        true
    }
}

fn more_filters(ev: &Eval, a: &Args) -> R<More> {
    let names = |v: Value, sp: Span| -> R<Vec<Name>> {
        match v {
            Value::Faces(f) => Ok(f.names.clone()),
            Value::Edges(e) => Ok(e.to_vec()),
            o => Err(Error::WrongType {
                span: sp,
                what: "touching (faces or edges)",
                want: Ty::Faces,
                found: o.ty(),
            }),
        }
    };
    let mut more = More {
        below: a.num("r_below", LEN)?.map(|n| n.v),
        above: a.num("r_above", LEN)?.map(|n| n.v),
        touching: a
            .opt("touching")
            .map(|v| names(v, a.span("touching")))
            .transpose()?
            .map(|v| v.into_iter().collect()),
        body: None,
    };
    if let Some(v) = a.opt("body") {
        let ids: Vec<usize> = feats_of(v, a.span("body"))?.iter().map(|f| f.idx).collect();
        // the faces and edges in each chosen body's span of the joined body
        let mut keep = std::collections::HashSet::new();
        if let Some(stage) = ev.b.stages.last()
            && let Some(b) = stage.view()
        {
            for s in stage.spans().iter().filter(|s| ids.contains(&s.solid)) {
                keep.extend(s.faces.clone().map(|f| b.faces[f as usize].name));
                keep.extend(s.edges.clone().map(|e| b.edges[e as usize].name));
            }
        }
        more.body = Some(keep);
    }
    Ok(more)
}

fn filter_edges(ev: &mut Eval, a: &Args, v: Value) -> R<Value> {
    let Value::Edges(names) = v else { return Ok(v) };
    let crease = word(a, "crease", &["concave", "convex", "smooth"])?;
    let shape = word(a, "shape", &["line", "circle", "curve"])?;
    let along = a.opt("along").map(|v| dir_of(v, a.span("along"))).transpose()?;
    let r = a.num("r", LEN)?;
    let longer = a.num("longer", LEN)?;
    let shorter = a.num("shorter", LEN)?;
    let extreme = a
        .opt("extreme")
        .map(|v| dir_of(v, a.span("extreme")))
        .transpose()?;
    let Some(body) = ev.b.body() else {
        return Ok(Value::Unresolved);
    };
    let mut es: Vec<EdgeId> = body
        .edge_ids()
        .filter(|&e| names.contains(&body.edge(e).name))
        .collect();
    if let Some(c) = crease {
        es.retain(|&e| select::crease(body, e) == Some(c));
    }
    if let Some(s) = shape {
        es.retain(|&e| select::curve_kind(&body.edge(e).curve) == s);
    }
    if let Some(d) = along {
        let d = d.normalize().map_or(Vec3::Z, |u| u.get());
        es.retain(|&e| match &body.edge(e).curve {
            Curve::Line(l) => l.dir.get().dot(d).abs() > 1.0 - 1e-9,
            _ => false,
        });
    }
    if let Some(r) = &r {
        es.retain(|&e| {
            select::radius(body, select::Ent::Edge(e)).is_some_and(|x| (x - r.v).abs() <= 1e-6 * r.v.max(1.0))
        });
    }
    if let Some(l) = &longer {
        es.retain(|&e| select::length(body, e) > l.v);
    }
    if let Some(l) = &shorter {
        es.retain(|&e| select::length(body, e) < l.v);
    }
    let more = more_filters(ev, a)?;
    es.retain(|&e| more.keeps(body, select::Ent::Edge(e)));
    if let Some(d) = extreme {
        let at = |e: EdgeId| (select::centre(body, select::Ent::Edge(e)) - Point3::ORIGIN).dot(d);
        let top = es.iter().map(|&e| at(e)).fold(f64::NEG_INFINITY, f64::max);
        es.retain(|&e| at(e) >= top - 1e-6 * top.abs().max(1.0));
    }
    let _ = Vec2::new(0.0, 0.0);
    Ok(Value::Edges(Rc::new(
        es.iter().map(|&e| body.edge(e).name).collect(),
    )))
}

/// Lines in space, for constructing points: `shift(line, d)` (the entity moved d to its left,
/// into a counter-clockwise loop), `hline(y)` and `vline(x)` in the sketch being drawn, and
/// `meet(a, b)`: where two lines cross, in the sketch being drawn.
fn lines(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    let line_of = |v: Value, sp: Span| -> R<(Point3, UnitVec3, Src)> {
        match v {
            Value::Line(p, d, s) => Ok((p, d, s)),
            Value::Ent(e) if e.kind == EntKind::Line => {
                let s = e.sk.borrow();
                let Some(forge_sketch::Entity::Line { a, b }) = s.s.entity(e.id) else {
                    unreachable!("a line")
                };
                let (pa, pb) = (s.world(s.pos(a)), s.world(s.pos(b)));
                Ok((pa, crate::eval::unit_or_z(pb - pa), Src::Const))
            }
            o => Err(Error::WrongType {
                span: sp,
                what: "a line",
                want: Ty::Line,
                found: o.ty(),
            }),
        }
    };
    match name {
        "shift" => {
            let a = Args::bind(ev, name, &["line", "d"], args, span)?;
            let lv = a.req("line")?;
            let normal = match &lv {
                Value::Ent(e) => e.sk.borrow().frame.z(),
                _ => ev.sketch_ref().map_or(UnitVec3::Z, |s| s.borrow().frame.z()),
            };
            let (p, d, src) = line_of(lv, a.span("line"))?;
            let by = a.req_num("d", LEN)?;
            let left = normal.cross(d.get());
            Ok(Value::Line(p + left * by.v, d, values::join_src(&src, &by.src)))
        }
        "hline" | "vline" => {
            let sk = in_sketch(ev, name, span)?;
            let a = Args::bind(ev, name, &["at"], args, span)?;
            let at = a.req_num("at", LEN)?;
            let s = sk.borrow();
            let (p, d) = if name == "hline" {
                (Point2::new(0.0, at.v), s.frame.x())
            } else {
                (Point2::new(at.v, 0.0), s.frame.y())
            };
            Ok(Value::Line(s.world(p), d, at.src.clone()))
        }
        _ => {
            let sk = in_sketch(ev, name, span)?;
            let a = Args::bind(ev, name, &["a", "b"], args, span)?;
            let (p, d, s1) = line_of(a.req("a")?, a.span("a"))?;
            let (q, e, s2) = line_of(a.req("b")?, a.span("b"))?;
            let f = sk.borrow().frame;
            let to2 = |x: Point3| {
                let l = f.point_to_local(x);
                Point2::new(l.x, l.y)
            };
            let dir2 = |v: Vec3| {
                let l = f.vec_to_local(v);
                Vec2::new(l.x, l.y)
            };
            let (p2, d2, q2, e2) = (to2(p), dir2(d.get()), to2(q), dir2(e.get()));
            let den = d2.cross(e2);
            if den.abs() < 1e-12 {
                return Err(Error::BadValue {
                    span,
                    what: "the angle between lines that meet (they are parallel)",
                    value: 0.0,
                });
            }
            let t = (q2 - p2).cross(e2) / den;
            let x = p2 + d2 * t;
            let src = values::join_src(&s1, &s2);
            let src = match src {
                Src::Const => Src::Const,
                s => Src::Driven(std::sync::Arc::new(s.keys())),
            };
            let n = |v: f64| {
                Value::Num(Num {
                    v,
                    dim: Dim::Length,
                    src: src.clone(),
                })
            };
            Ok(Value::Tuple(Rc::new(vec![n(x.x), n(x.y)])))
        }
    }
}

/// One side of `between`: what features made, or these faces.
enum By {
    Feats(Vec<Rc<Feat>>),
    Faces(Vec<Name>),
}

#[cfg(test)]
mod names_tests {
    /// Each name the map says `constant` knows, it does.
    #[test]
    fn every_constant_name_is_a_constant() {
        for &(n, _, c) in super::NAMES.iter().filter(|e| !e.0.is_empty()) {
            assert!(!c || super::constant(n).is_some(), "{n}");
        }
    }
}

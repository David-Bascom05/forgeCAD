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
use forge_ops::{Axis2, ChamferCorner, ChamferSize, Radius};
use forge_sketch::{Constraint, EntityId};
use forge_topo::{Body, EdgeId, FaceId, Name};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub const BUILTINS: &[&str] = &[
    "sqrt",
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
];

/// A built-in function's name, this module's or another's (shapes).
pub fn builtin(name: &str) -> Option<&'static str> {
    BUILTINS
        .iter()
        .chain(crate::shapes::NAMES)
        .find(|b| **b == name)
        .copied()
}

const MORE: [&str; 5] = ["axis", "sym", "two", "upto", "intersect"];

pub fn constant(name: &str) -> Option<Value> {
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
        n => crate::wired::constant(n),
    }
}

/// A call's arguments bound to names: positional ones in order, then keywords.
pub struct Args {
    func: &'static str,
    call: Span,
    vals: HashMap<&'static str, (Value, Span)>,
}

impl Args {
    pub fn bind(
        ev: &mut Eval,
        func: &'static str,
        names: &[&'static str],
        args: &[Arg],
        call: Span,
    ) -> R<Args> {
        let mut vals = HashMap::new();
        for (k, a) in args.iter().enumerate() {
            let key = match &a.name {
                Some((n, sp)) => *names.iter().find(|x| *x == n).ok_or(Error::UnknownArg {
                    span: *sp,
                    func,
                    arg: n.clone(),
                })?,
                None => *names.get(k).ok_or(Error::TooManyArgs {
                    span: a.value.span,
                    func: func.into(),
                    max: names.len(),
                })?,
            };
            let v = ev.expr(&a.value)?;
            if let Some(sk) = ev.sketch_ref() {
                crate::eval::collect_params(&v, &mut sk.borrow_mut().params);
            }
            crate::eval::collect_params(&v, &mut ev.reads);
            // an empty face set is unknown only after an earlier failure (or in `check`); on a
            // part built so far without one, it is a selection that found nothing
            let known_empty = ev.b.building && ev.errors.is_empty() && !unresolved_in(&v);
            ev.poisoned |= unknown_in(&v) && !known_empty;
            vals.insert(key, (v, a.value.span));
        }
        Ok(Args { func, call, vals })
    }
    /// Whether an argument stands for something an earlier error left unknown.
    pub fn poisoned(&self) -> bool {
        self.vals.values().any(|v| matches!(v.0, Value::Unresolved))
    }
    pub fn opt(&self, k: &str) -> Option<Value> {
        self.vals.get(k).map(|v| v.0.clone())
    }
    pub fn req(&self, k: &'static str) -> R<Value> {
        self.opt(k).ok_or(Error::MissingArg {
            span: self.call,
            func: self.func,
            arg: k,
        })
    }
    pub fn span(&self, k: &str) -> Span {
        self.vals.get(k).map_or(self.call, |v| v.1)
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
        Value::Wired(ref w) if w.point().is_some() => Ok(w.point().expect("checked")),
        Value::Ent(e) => {
            let id = point_ent(&e, span)?;
            let sk = e.sk.borrow();
            Ok(sk.world(sk.pos(id)))
        }
        Value::Axis(o, _) => Ok(o),
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
    match v {
        Value::Plane(p) => {
            ev.world_check(p.world.is_some(), span)?;
            Ok(p.frame)
        }
        Value::Sketch(s) => Ok(s.borrow().frame),
        Value::Wired(ref w) if w.plane().is_some() => Ok(w.plane().expect("checked")),
        Value::Faces(f) => match f.plane {
            Some(fr) => Ok(fr),
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
    if let Some(r) = crate::wired::pre_call(ev, name, args, span) {
        return r;
    }
    match name {
        "sqrt" | "abs" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "count" | "ratio" => {
            math1(ev, name, args, span)
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
                    Src::Driving(p) => Src::Driven([p].into()),
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
                                s => Src::Driven(s.params()),
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
            let d = a.req_num("d", LEN)?;
            Ok(Value::Extent(ExtentV::Sym(positive(
                &d,
                a.span("d"),
                "a symmetric extent",
            )?)))
        }
        "two" => {
            let a = Args::bind(ev, name, &["forward", "back"], args, span)?;
            let f = a.req_num("forward", LEN)?;
            let b = a.req_num("back", LEN)?;
            Ok(Value::Extent(ExtentV::Two(f.v, b.v)))
        }
        "upto" => {
            let a = Args::bind(ev, name, &["face", "offset"], args, span)?;
            let off = a.num("offset", LEN)?.map_or(0.0, |n| n.v);
            match a.req("face")? {
                Value::Faces(f) => Ok(Value::Extent(ExtentV::UpTo(f, off))),
                Value::Wired(w) if w.plane().is_some() && w.work_ref().is_some() => Ok(Value::Extent(
                    ExtentV::UpToPlane(w.work_ref().expect("checked").feature, off),
                )),
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
            let c = p2_of(ev, a.req("center")?, a.span("center"))?;
            let r = a.req_num("r", LEN)?;
            positive(&r, a.span("r"), "a radius")?;
            let mut s = sk.borrow_mut();
            let c = s.p2(c, span)?;
            let nm = pending_name(ev, span);
            let id = s.circle(c, r.v, nm.as_deref(), span)?;
            drop(s);
            Ok(mk_ent(&sk, id))
        }
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
            let a = Args::bind(ev, name, &["points", "closed"], args, span)?;
            let closed = match a.opt("closed") {
                None => true,
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
                let id = s.spline(&ids, closed, nm.as_deref(), span)?;
                drop(s);
                Ok(mk_ent(&sk, id))
            }
        }
        "spline" | "tarc" | "close" => path_seg(ev, name, args, span),
        "tangent" | "coincident" | "horizontal" | "vertical" | "parallel" | "perpendicular" | "equal"
        | "midpoint" | "distance" | "radius" | "angle" | "fix" => constraint(ev, name, args, span),
        "extrude" | "cut" | "intersect" => extrude(ev, name, args, span),
        n if crate::sheet::BUILTINS.contains(&n) => crate::sheet::call(ev, name, args, span),
        "revolve" => revolve(ev, args, span),
        "fillet" | "chamfer" => finish(ev, name, args, span),
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
            let x = anchored("x")?;
            let z = z.normalize().ok_or(Error::BadValue {
                span: a.span("normal"),
                what: "a zero normal",
                value: 0.0,
            })?;
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

pub(crate) fn has_solid(ev: &Eval) -> bool {
    ev.feats.iter().any(|f| {
        (matches!(f.kind, "extrude" | "revolve" | "loft" | "import")
            || crate::shapes::SOLID.contains(&f.kind))
            && f.status != crate::eval::Status::Failed
    })
}

fn extrude(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, name, span)?;
    let id = ev.take_id(if name == "extrude" { "extrude" } else { "cut" }, span)?;
    let a = Args::bind(ev, name, &["profile", "d", "taper", "new"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let (sk, profile, segs) = profile_of(ev, a.req("profile")?, a.span("profile"))?;
    if a.opt("taper").is_some() {
        if !matches!(a.req("d")?, Value::Num(_)) {
            return Err(Error::NotWith {
                span: a.span("taper"),
                func: name,
                arg: "taper",
                with: "an extent other than a depth (a taper runs a set distance)",
            });
        }
        let curved = profile.loops.iter().any(|l| {
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
    let (extent, near, far) = match a.req("d")? {
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
            match a.num("taper", ANG)? {
                Some(t) => {
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
                None => (Extent::Distance(dist), 0.0, Some(dist)),
            }
        }
        Value::Extent(x) => match x {
            ExtentV::Sym(d) => (Extent::Symmetric(d), -d / 2.0, Some(d / 2.0)),
            ExtentV::Two(f, b) => (
                Extent::TwoSided {
                    forward: sign * f,
                    back: sign * b,
                },
                -sign * b,
                Some(sign * f),
            ),
            ExtentV::Thru => (
                Extent::Through(if cut { Toward::Back } else { Toward::Forward }),
                0.0,
                None,
            ),
            ExtentV::ThruAll => (Extent::Through(Toward::Both), 0.0, None),
            ExtentV::Next => (Extent::ToNext { reverse: cut }, 0.0, None),
            ExtentV::UpTo(f, off) => {
                let feature = ev.feats.iter().find(|x| x.id == f.owner).map_or(0, |x| x.idx);
                let name = f.names.first().copied().unwrap_or(Name::NONE);
                (
                    Extent::To {
                        target: Target::Face { feature, name },
                        offset: off,
                    },
                    0.0,
                    None,
                )
            }
            ExtentV::UpToPlane(k, off) => (
                Extent::To {
                    target: Target::Plane(k),
                    offset: off,
                },
                0.0,
                None,
            ),
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
    let f = Feature::Extrude { profile, extent, op };
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
        &["profile", "axis", "angle", "cut", "new"],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let (sk, profile, _) = profile_of(ev, a.req("profile")?, a.span("profile"))?;
    let ax = ent_of(a.req("axis")?, a.span("axis"), EntKind::Line)?;
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
    let mut dims = vec![];
    let turn = match a.num("angle", ANG)? {
        Some(t) => {
            dims.push(dim("angle", &t));
            Turn::Angle(t.v)
        }
        None => Turn::Angle(std::f64::consts::TAU),
    };
    let op = if a.flag("cut")? {
        Combine::Cut
    } else if a.flag("new")? || !has_solid(ev) {
        Combine::New
    } else {
        Combine::Add
    };
    let axis2 = Axis2 {
        origin: pa,
        dir: pb - pa,
    };
    let f = Feature::Revolve {
        profile,
        axis: axis2,
        extent: turn,
        op,
    };
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

fn finish(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, name, span)?;
    let id = ev.take_id(name, span)?;
    let a = Args::bind(ev, name, &["edges", "r", "groups"], args, span)?;
    let mut groups: Vec<(Value, Span, Num)> = vec![];
    if let Some(e) = a.opt("edges") {
        groups.push((e, a.span("edges"), a.req_num("r", LEN)?));
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
    let mut dims = vec![];
    let mut edges: Vec<(Name, f64)> = vec![];
    for (v, sp, r) in &groups {
        positive(
            r,
            *sp,
            if name == "fillet" {
                "a fillet radius"
            } else {
                "a chamfer size"
            },
        )?;
        dims.push(dim(if name == "fillet" { "radius" } else { "size" }, r));
        match edges_value(v.clone(), *sp)? {
            Some(es) => {
                if es.is_empty() && ev.b.building {
                    return Err(Error::EmptySelection { span: *sp });
                }
                edges.extend(es.into_iter().map(|e| (e, r.v)));
            }
            // what an earlier error left unknown: nothing to build; with no error before, the
            // selection found nothing on the part
            None if ev.b.building && ev.errors.is_empty() => return Err(Error::EmptySelection { span: *sp }),
            None if ev.b.building => return Ok(Value::Unresolved),
            None => {}
        }
    }
    let f = if name == "fillet" {
        Feature::Fillet {
            edges: edges.iter().map(|(e, r)| (*e, Radius::Constant(*r))).collect(),
            starts: vec![],
            setbacks: vec![],
            only: false,
        }
    } else {
        // one size per chamfer feature in forge-model: the first group's
        let size = edges.first().map_or(1.0, |e| e.1);
        Feature::Chamfer {
            edges: edges.iter().map(|e| e.0).collect(),
            size: ChamferSize::Equal(size),
            corner: ChamferCorner::Patch,
        }
    };
    let (idx, hid) = ev.push_feature(&id, name, f, span, dims)?;
    Ok(Value::Feature(Rc::new(Feat {
        id,
        kind: name,
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
    let names = [forge_ops::roles::start(root), forge_ops::roles::end(root)];
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
            src: Src::Driving(ev.locals[k].key.clone()),
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
                "rim" if f.axis.is_some() && f.kind != "hole" => {
                    let (o, d) = f.axis.expect("checked");
                    rim(ev, f, o, d, span)
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
        Value::Unresolved => Ok(Value::Unresolved),
        _ => Err(unknown(&v)),
    }
}

/// `E.side(entity)`: the face an entity of the profile made.
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
    let names: Vec<Name> = match crate::shapes::side_names(ev, f, &segs) {
        Some(n) => n,
        None => ids
            .iter()
            .flat_map(|&i| {
                let seg = sk.borrow().seg_id(i);
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
fn rim(ev: &mut Eval, f: &Feat, o: Point3, d: UnitVec3, span: Span) -> R<Value> {
    if !ev.b.building || ev.b.built() <= f.idx {
        return Ok(Value::Unresolved);
    }
    let n = ev.b.built();
    let origin = ev.b.origin(n);
    let body = ev.b.body().expect("built");
    let made = |x: FaceId| origin.get(&body.face(x).name).is_some_and(|o| o.contains(&f.idx));
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
        ],
        args,
        span,
    )?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let of: Option<Vec<Rc<Feat>>> = match a.opt("of") {
        None => None,
        Some(Value::Faces(fs)) if name == "edges" => {
            let v = member(ev, Value::Faces(fs), "edges", span)?;
            return filter_edges(ev, &a, v);
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
            "line", "circle", "curve", "plane", "cyl", "cone", "sphere", "torus", "blend",
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
    for k in ["r", "longer", "shorter"] {
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
                s => Src::Driven(s.params()),
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

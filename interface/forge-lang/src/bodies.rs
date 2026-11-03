//! Parts of several bodies: a feature on chosen bodies (`bodies:` on any wired feature), and
//! features that combine (`union`, `subtract`, `intersect(.., tools:)`, `combine`), split,
//! split faces, move and copy bodies. A body is named by the feature that made it.

use crate::builtins::{ANG, Args, LEN, axis_of, dir_of, not_in_sketch, plane_of};
use crate::eval::{Eval, R, Value};
use crate::feat_error::FeatError;
use crate::syntax::{Arg, Span};
use crate::values::Ty;
use crate::wired::{W, feat, wired, word};
use forge_model::{BodyFeature, Combine, Cutter, Feature, Keep, Pick, Step, Target};
use forge_ops::Segment;
use forge_sketch::Entity;

/// The bodies a feature value names (the features that made them).
fn body_ids(v: Value, span: Span) -> R<Vec<usize>> {
    match v {
        Value::Feature(f) => Ok(vec![f.idx]),
        Value::List(xs) => {
            let mut out = vec![];
            for x in xs.iter() {
                out.extend(body_ids(x.clone(), span)?);
            }
            Ok(out)
        }
        Value::Map(m) => {
            let mut out = vec![];
            for x in m.iter() {
                out.extend(body_ids(x.1.clone(), span)?);
            }
            Ok(out)
        }
        o => Err(crate::error::Error::WrongType {
            span,
            what: "bodies (the features that made them)",
            want: Ty::Feature,
            found: o.ty(),
        }),
    }
}

/// `bodies: F | [F..] | all` on a feature: it acts on those bodies, not the last made.
pub fn wrap_in(_ev: &mut Eval, a: &Args, f: Feature) -> R<Feature> {
    let bodies = match a.opt("bodies") {
        None => return Ok(f),
        Some(Value::Str(w)) if &*w == "all" => Pick::All,
        Some(v) => Pick::Some(body_ids(v, a.span("bodies"))?),
    };
    Ok(Feature::Body(Box::new(BodyFeature::In { bodies, feature: f })))
}

pub fn combine(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, name, span)?;
    let id = ev.take_id(name, span)?;
    let a = Args::bind(ev, name, &["target", "tools", "op", "keep"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    crate::wired::only_with(
        &a,
        name,
        "op",
        name == "combine",
        crate::feat_error::Because::OtherForm,
    )?;
    let op = match name {
        "union" => Combine::Add,
        "subtract" => Combine::Cut,
        "intersect" => Combine::Keep,
        _ => match word(&a, "op", &["add", "cut", "keep"])? {
            Some("add") => Combine::Add,
            Some("cut") => Combine::Cut,
            Some(_) => Combine::Keep,
            None => {
                return Err(crate::error::Error::MissingArg {
                    span,
                    func: name,
                    arg: "op",
                });
            }
        },
    };
    let [target] = body_ids(a.req("target")?, a.span("target"))?[..] else {
        return Err(FeatError::NotOne {
            span: a.span("target"),
            what: "a target body",
            count: 0,
        }
        .into());
    };
    let tools = body_ids(a.req("tools")?, a.span("tools"))?;
    if tools.is_empty() {
        return Err(FeatError::NotOne {
            span: a.span("tools"),
            what: "tool bodies",
            count: 0,
        }
        .into());
    }
    let f = Feature::Body(Box::new(BodyFeature::Combine {
        op,
        target,
        tools,
        keep: a.flag("keep")?,
    }));
    let (idx, hid) = ev.push_feature(&id, "combine", f, span, vec![])?;
    Ok(feat(id, "combine", idx, hid, None))
}

/// What cuts: a work plane or one face (followed through edits), a plane worked out here, or a
/// line, arc or circle of a sketch swept along its plane's normal.
fn cutter(ev: &mut Eval, v: Value, span: Span) -> R<Cutter> {
    match v {
        Value::Wired(ref w) if w.plane().is_some() && w.work_ref().is_some() => Ok(Cutter::Target(
            Target::Plane(w.work_ref().expect("checked").feature),
        )),
        Value::Faces(_) => {
            let r = crate::work::to_ref(ev, v, span)?;
            Ok(Cutter::Target(Target::Face {
                feature: r.feature,
                name: r.name.unwrap_or(forge_topo::Name::NONE),
            }))
        }
        Value::Ent(ref e) => {
            let s = e.sk.borrow();
            let p = |x| s.pos(x);
            let seg = match s.s.entity(e.id) {
                Some(Entity::Line { a, b }) => Segment::Line { a: p(a), b: p(b) },
                Some(Entity::Circle { center, .. }) => Segment::Circle {
                    center: p(center),
                    radius: s.s.radius(e.id),
                    ccw: true,
                },
                Some(Entity::Arc {
                    center,
                    start,
                    end,
                    ccw,
                }) => {
                    let (c, a0, b0) = (p(center), p(start), p(end));
                    let r = c.distance(a0);
                    let t0 = forge_math::dmath::atan2(a0.y - c.y, a0.x - c.x);
                    let t1 = forge_math::dmath::atan2(b0.y - c.y, b0.x - c.x);
                    let tau = std::f64::consts::TAU;
                    let mut sweep = (t1 - t0).rem_euclid(tau);
                    if !ccw {
                        sweep -= tau;
                    }
                    Segment::Arc {
                        center: c,
                        radius: r,
                        start: t0,
                        sweep,
                    }
                }
                _ => {
                    return Err(crate::error::Error::WrongEntity {
                        span,
                        want: crate::error::EntKind::Line,
                        found: e.kind,
                    });
                }
            };
            Ok(Cutter::Curve(s.frame, seg))
        }
        v => Ok(Cutter::Plane(plane_of(ev, v, span, "by")?)),
    }
}

pub fn split(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, name, span)?;
    let id = ev.take_id(name, span)?;
    let a = Args::bind(ev, name, &["by", "keep", "faces", "bodies"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let face = name == "splitface";
    crate::wired::only_with(&a, name, "keep", !face, crate::feat_error::Because::OtherForm)?;
    crate::wired::only_with(&a, name, "faces", face, crate::feat_error::Because::OtherForm)?;
    let by = cutter(ev, a.req("by")?, a.span("by"))?;
    let bf = if face {
        let faces = match a.req("faces")? {
            Value::Faces(f) => f.names.clone(),
            o => {
                return Err(crate::error::Error::WrongType {
                    span: a.span("faces"),
                    what: "faces to split",
                    want: Ty::Faces,
                    found: o.ty(),
                });
            }
        };
        BodyFeature::SplitFace { faces, by }
    } else {
        let keep = match word(&a, "keep", &["both", "plus", "minus"])? {
            None | Some("both") => Keep::Both,
            Some("plus") => Keep::Plus,
            Some(_) => Keep::Minus,
        };
        BodyFeature::Split { by, keep }
    };
    let f = wrap_in(ev, &a, Feature::Body(Box::new(bf)))?;
    let (idx, hid) = ev.push_feature(&id, name, f, span, vec![])?;
    Ok(feat(id, name, idx, hid, None))
}

pub fn move_body(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, name, span)?;
    let id = ev.take_id(name, span)?;
    let a = Args::bind(ev, name, &["of", "moves"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let of = body_ids(a.req("of")?, a.span("of"))?;
    let copy = name == "copybody";
    if copy && of.len() != 1 {
        return Err(FeatError::NotOne {
            span: a.span("of"),
            what: "the body copied",
            count: of.len(),
        }
        .into());
    }
    let steps = match a.req("moves")? {
        Value::List(xs) => xs
            .iter()
            .map(|x| match x {
                Value::Wired(w) => match **w {
                    W::Motion(s) => Ok(s),
                    _ => Err(w.ty()),
                },
                o => Err(o.ty()),
            })
            .collect::<Result<Vec<Step>, Ty>>()
            .map_err(|found| crate::error::Error::WrongType {
                span: a.span("moves"),
                what: "motions (slide, spin, place)",
                want: Ty::Motion,
                found,
            })?,
        Value::Wired(w) if matches!(*w, W::Motion(_)) => {
            let W::Motion(s) = *w else { unreachable!("checked") };
            vec![s]
        }
        o => {
            return Err(crate::error::Error::WrongType {
                span: a.span("moves"),
                what: "motions (slide, spin, place)",
                want: Ty::Motion,
                found: o.ty(),
            });
        }
    };
    let f = Feature::Body(Box::new(BodyFeature::In {
        bodies: Pick::Some(of),
        feature: Feature::Body(Box::new(BodyFeature::Move { steps, copy })),
    }));
    let (idx, hid) = ev.push_feature(&id, name, f, span, vec![])?;
    Ok(feat(id, name, idx, hid, None))
}

/// `slide(along: DIR, by: L)`, `spin(about: AXIS, by: A)`, `place(PLANE)` (world to its frame).
pub fn motion(ev: &mut Eval, name: &'static str, args: &[Arg], span: Span) -> R<Value> {
    let names: &[&'static str] = match name {
        "slide" => &["along", "by"],
        "spin" => &["about", "by"],
        _ => &["on"],
    };
    let a = Args::bind(ev, name, names, args, span)?;
    let step = match name {
        "slide" => {
            let v = a.req("along")?;
            ev.world_check(matches!(v, Value::WorldDir(_)), a.span("along"))?;
            let d = dir_of(v, a.span("along"))?;
            let d = d.normalize().ok_or(crate::error::Error::BadValue {
                span: a.span("along"),
                what: "a zero direction",
                value: 0.0,
            })?;
            let by = a.req_num("by", LEN)?;
            Step::Translate(d.get() * by.v)
        }
        "spin" => {
            let (o, d) = axis_of(ev, a.req("about")?, a.span("about"))?;
            let by = a.req_num("by", ANG)?;
            Step::Rotate {
                origin: o,
                axis: d.get(),
                angle: by.v,
            }
        }
        _ => Step::Frame(plane_of(ev, a.req("on")?, a.span("on"), "on")?),
    };
    Ok(wired(W::Motion(step)))
}

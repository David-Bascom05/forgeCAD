//! Work features (`workplane`, `workaxis`, `workpoint`, `ucs`): named features that make no
//! solid, defined by references the kernel follows through every edit above them, and the
//! references themselves: a value (a work feature, a sketch, a sketch's point, one face or one
//! edge) as an earlier feature and its own name for the entity.

use crate::builtins::{ANG, Args, LEN, dim, not_in_sketch, plane_of};
use crate::eval::{Eval, R, Value};
use crate::feat_error::FeatError;
use crate::syntax::Span;
use crate::wired::{W, one_form, wired};
use forge_agent::select;
use forge_model::{AxisBy, Feature, PlaneBy, PointBy, Ref, UcsBy, Work};
use forge_sketch::{Entity, EntityId};
use forge_topo::Name;

const PLANE_FORMS: &[&str] = &["on", "offset", "turn", "tangent", "through", "mid", "normal_to"];
const AXIS_FORMS: &[&str] = &["along", "through", "meet", "normal_to"];
const POINT_FORMS: &[&str] = &["at", "mid", "meet"];
const UCS_FORMS: &[&str] = &["on", "through"];
const KEYS: &[&str] = &[
    "on",
    "offset",
    "d",
    "turn",
    "about",
    "by",
    "tangent",
    "parallel",
    "through",
    "mid",
    "normal_to",
    "at",
    "along",
    "meet",
];

pub fn work(ev: &mut Eval, name: &'static str, args: &[crate::syntax::Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, name, span)?;
    let id = ev.take_id(name, span)?;
    let a = Args::bind(ev, name, KEYS, args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let forms = match name {
        "workplane" => PLANE_FORMS,
        "workaxis" => AXIS_FORMS,
        "workpoint" => POINT_FORMS,
        _ => UCS_FORMS,
    };
    let form = one_form(&a, name, forms, span)?;
    // every argument belongs to the form given
    let partners: &[&str] = match (name, form) {
        ("workplane", "offset") => &["d"],
        ("workplane", "turn") => &["about", "by"],
        ("workplane", "tangent") => &["parallel"],
        ("workplane" | "workaxis", "normal_to") => &["at"],
        _ => &[],
    };
    for k in KEYS {
        let ok = *k == form || partners.contains(k);
        crate::wired::only_with(&a, name, k, ok, crate::feat_error::Because::OtherForm)?;
        if partners.contains(k) && a.opt(k).is_none() {
            return Err(crate::error::Error::MissingArg {
                span,
                func: name,
                arg: k,
            });
        }
    }
    let r = |ev: &mut Eval, k: &'static str| -> R<Ref> { to_ref(ev, a.req(k)?, a.span(k)) };
    let two = |ev: &mut Eval, k: &'static str| -> R<[Ref; 2]> {
        match a.req(k)? {
            Value::Tuple(t) | Value::List(t) if t.len() == 2 => Ok([
                to_ref(ev, t[0].clone(), a.span(k))?,
                to_ref(ev, t[1].clone(), a.span(k))?,
            ]),
            o => Err(crate::error::Error::WrongType {
                span: a.span(k),
                what: "two references",
                want: crate::values::Ty::Tuple,
                found: o.ty(),
            }),
        }
    };
    let three = |ev: &mut Eval, k: &'static str| -> R<[Ref; 3]> {
        match a.req(k)? {
            Value::Tuple(t) | Value::List(t) if t.len() == 3 => Ok([
                to_ref(ev, t[0].clone(), a.span(k))?,
                to_ref(ev, t[1].clone(), a.span(k))?,
                to_ref(ev, t[2].clone(), a.span(k))?,
            ]),
            o => Err(crate::error::Error::WrongType {
                span: a.span(k),
                what: "three points",
                want: crate::values::Ty::List,
                found: o.ty(),
            }),
        }
    };
    let mut dims = vec![];
    let f = match (name, form) {
        ("workplane", "on") => Feature::Plane(plane_of(ev, a.req("on")?, a.span("on"), "on")?),
        ("ucs", "on") => Feature::Work(Work::Ucs(UcsBy::Frame(plane_of(
            ev,
            a.req("on")?,
            a.span("on"),
            "on",
        )?))),
        ("workplane", "offset") => {
            let d = a.req_num("d", LEN)?;
            dims.push(dim("offset", &d));
            Feature::Work(Work::Plane(PlaneBy::Offset(r(ev, "offset")?, d.v)))
        }
        ("workplane", "turn") => {
            let by = a.req_num("by", ANG)?;
            dims.push(dim("angle", &by));
            Feature::Work(Work::Plane(PlaneBy::Angle {
                plane: r(ev, "turn")?,
                axis: r(ev, "about")?,
                angle: by.v,
            }))
        }
        ("workplane", "tangent") => Feature::Work(Work::Plane(PlaneBy::Tangent {
            face: r(ev, "tangent")?,
            plane: r(ev, "parallel")?,
        })),
        ("workplane", "through") => Feature::Work(Work::Plane(PlaneBy::Points(three(ev, "through")?))),
        ("ucs", "through") => Feature::Work(Work::Ucs(UcsBy::Points(three(ev, "through")?))),
        ("workplane", "mid") => {
            let [p, q] = two(ev, "mid")?;
            Feature::Work(Work::Plane(PlaneBy::Mid(p, q)))
        }
        ("workplane", "normal_to") => Feature::Work(Work::Plane(PlaneBy::Normal {
            curve: r(ev, "normal_to")?,
            at: r(ev, "at")?,
        })),
        ("workaxis", "along") => Feature::Work(Work::Axis(AxisBy::Along(r(ev, "along")?))),
        ("workaxis", "through") => {
            let [p, q] = two(ev, "through")?;
            Feature::Work(Work::Axis(AxisBy::Points(p, q)))
        }
        ("workaxis", "meet") => {
            let [p, q] = two(ev, "meet")?;
            Feature::Work(Work::Axis(AxisBy::Meet(p, q)))
        }
        ("workaxis", "normal_to") => Feature::Work(Work::Axis(AxisBy::Normal {
            plane: r(ev, "normal_to")?,
            at: r(ev, "at")?,
        })),
        ("workpoint", "at") => Feature::Work(Work::Point(PointBy::At(r(ev, "at")?))),
        ("workpoint", "mid") => Feature::Work(Work::Point(PointBy::Mid(r(ev, "mid")?))),
        _ => {
            let [p, q] = two(ev, "meet")?;
            Feature::Work(Work::Point(PointBy::Meet(p, q)))
        }
    };
    let (idx, _) = ev.push_feature(&id, "work", f, span, dims)?;
    let datum = if ev.b.building {
        forge_model::work::datum(ev.b.history(), idx).map_err(|error| crate::error::Error::Build {
            span,
            id: id.clone(),
            error,
        })?
    } else {
        return Ok(Value::Unresolved);
    };
    Ok(wired(W::Work { id, idx, datum }))
}

/// A value as a reference the kernel follows: a work feature or a sketch (its plane), a named
/// sketch point (or a line's end, a circle's centre), exactly one face or one edge of the part
/// (as the earlier feature whose own solid holds it, by its name there).
pub fn to_ref(ev: &mut Eval, v: Value, span: Span) -> R<Ref> {
    // `check` builds nothing: a face or edge is not known yet
    if !ev.b.building && matches!(v, Value::Faces(_) | Value::Edges(_)) {
        return Ok(Ref {
            feature: 0,
            name: None,
        });
    }
    match v {
        Value::Wired(ref w) if w.work_ref().is_some() => Ok(w.work_ref().expect("checked")),
        Value::Sketch(s) => Ok(Ref {
            feature: s.borrow().idx,
            name: None,
        }),
        Value::Ent(e) => {
            let s = e.sk.borrow();
            let t = point_name(&s, e.id).ok_or_else(|| FeatError::Unnamed {
                span,
                sketch: s.id.clone(),
            })?;
            Ok(Ref {
                feature: s.idx,
                name: Some(forge_model::params::point_name(&t)),
            })
        }
        Value::Faces(fs) => match &fs.names[..] {
            [one] => owner(ev, *one, true, span),
            // the pieces of one face (a revolve's halves, a face a cut split) are one reference
            [first, ..] if one_surface(ev, &fs.names) => owner(ev, *first, true, span),
            ns => Err(FeatError::NotOne {
                span,
                what: "a face reference",
                count: ns.len(),
            }
            .into()),
        },
        Value::Edges(es) => match &es[..] {
            [one] => owner(ev, *one, false, span),
            ns => Err(FeatError::NotOne {
                span,
                what: "an edge reference",
                count: ns.len(),
            }
            .into()),
        },
        Value::Unresolved => Ok(Ref {
            feature: 0,
            name: None,
        }),
        o => Err(FeatError::NotARef { span, found: o.ty() }.into()),
    }
}

/// Whether the part's faces of these names all lie on one surface.
fn one_surface(ev: &Eval, names: &[Name]) -> bool {
    let Some(b) = ev.b.body() else { return false };
    let mut surfaces = b
        .face_ids()
        .filter(|&f| names.contains(&b.face(f).name))
        .map(|f| &b.face(f).surface);
    let Some(first) = surfaces.next() else {
        return false;
    };
    surfaces.all(|s| s == first)
}

/// The sketch's name for a point entity: its own, or a named line's or arc's end (`l.a`, `l.b`),
/// a named circle's or arc's centre (`c.c`).
pub fn point_name(s: &crate::sketch::Sk, p: EntityId) -> Option<String> {
    let named = |e: EntityId| {
        s.ents
            .iter()
            .find(|x| x.0 == e && !x.1.starts_with('~'))
            .map(|x| x.1.clone())
    };
    if let Some(n) = named(p) {
        return Some(n);
    }
    for (e, n, _) in &s.ents {
        if n.starts_with('~') {
            continue;
        }
        let part = match s.s.entity(*e) {
            Some(Entity::Line { a, .. } | Entity::Arc { start: a, .. }) if a == p => "a",
            Some(Entity::Line { b, .. } | Entity::Arc { end: b, .. }) if b == p => "b",
            Some(Entity::Circle { center, .. } | Entity::Arc { center, .. }) if center == p => "c",
            _ => continue,
        };
        return Some(format!("{n}.{part}"));
    }
    None
}

/// The earlier feature whose own solid holds the part's face (or edge) `name`, and its name
/// there: among the features the part's lineage says made it, one that has the very name, else
/// one whose entity lies on the same surface (or curve).
pub fn owner(ev: &mut Eval, name: Name, face: bool, span: Span) -> R<Ref> {
    let n = ev.b.built();
    if !ev.b.building || n == 0 {
        return Ok(Ref {
            feature: 0,
            name: Some(name),
        });
    }
    let origin = ev.b.origin(n);
    let body = ev.b.body().expect("built").clone();
    let mut cands: Vec<usize> = vec![];
    let faces: Vec<Name> = if face {
        vec![name]
    } else {
        body.edge_by_name(name)
            .map(|e| {
                select::faces_of_edge(&body, e)
                    .into_iter()
                    .map(|f| body.face(f).name)
                    .collect()
            })
            .unwrap_or_default()
    };
    for f in &faces {
        for &k in origin.get(f).into_iter().flatten() {
            if !cands.contains(&k) {
                cands.push(k);
            }
        }
    }
    let h = ev.b.history();
    let mut near = None;
    for &k in &cands {
        match forge_model::work::own_name(h, k, &body, name) {
            Some(m) if m == name => {
                return Ok(Ref {
                    feature: k,
                    name: Some(m),
                });
            }
            Some(m) => {
                near = near.or(Some(Ref {
                    feature: k,
                    name: Some(m),
                }))
            }
            None => {}
        }
    }
    near.ok_or_else(|| FeatError::NoOwner { span }.into())
}

//! Fillets with every option the kernel's takes: a radius per group, constant or varying along
//! each edge (`vary(a, b)`: linear; `vary([(t, r), ..])`: through radii at fractions of the
//! way), the vertex an edge's law starts from (`from:`), setbacks at corners, and `only` (the
//! edges named, not the tangent chains they lie on).

use crate::builtins::{Args, LEN, dim, edges_value, not_in_sketch, num_of, point_of, positive};
use crate::error::Error;
use crate::eval::{Dimension, Eval, R, Value};
use crate::feat_error::FeatError;
use crate::syntax::{Arg, Span};
use crate::values::{Dim, Ty};
use crate::wired::{W, feat, wired};
use forge_model::Feature;
use forge_ops::Radius;
use forge_topo::Name;

/// `vary(a, b)` or `vary([(t, r), ..])`: a radius that changes along an edge.
pub fn vary(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let a = Args::bind(ev, "vary", &["from", "to"], args, span)?;
    let mut dims = vec![];
    let r = match (a.req("from")?, a.opt("to")) {
        (Value::List(pts), None) => {
            let mut out = vec![];
            for p in pts.iter() {
                let Value::Tuple(t) = p else {
                    return Err(Error::WrongType {
                        span: a.span("from"),
                        what: "a (fraction, radius) point",
                        want: Ty::Tuple,
                        found: p.ty(),
                    });
                };
                if t.len() != 2 {
                    return Err(Error::WrongType {
                        span: a.span("from"),
                        what: "a (fraction, radius) point",
                        want: Ty::Tuple,
                        found: Ty::Tuple,
                    });
                }
                let f = num_of(
                    t[0].clone(),
                    &[Dim::Ratio],
                    a.span("from"),
                    "a fraction along the edge",
                )?;
                let r = num_of(t[1].clone(), LEN, a.span("from"), "a radius")?;
                positive(&r, a.span("from"), "a fillet radius")?;
                let last = out.last().map_or(-1.0, |x: &(f64, f64)| x.0);
                if !(0.0..=1.0).contains(&f.v) || f.v <= last {
                    return Err(Error::BadValue {
                        span: a.span("from"),
                        what: "fractions along the edge (0 to 1, increasing)",
                        value: f.v,
                    });
                }
                dims.push(dim("fraction", &f));
                dims.push(dim("radius", &r));
                out.push((f.v, r.v));
            }
            if out.len() < 2 {
                return Err(FeatError::NotOne {
                    span: a.span("from"),
                    what: "a varying radius's points (two or more)",
                    count: out.len(),
                }
                .into());
            }
            Radius::Points(out)
        }
        (v, Some(w)) => {
            let x = num_of(v, LEN, a.span("from"), "a start radius")?;
            let y = num_of(w, LEN, a.span("to"), "an end radius")?;
            positive(&x, a.span("from"), "a fillet radius")?;
            positive(&y, a.span("to"), "a fillet radius")?;
            dims.push(dim("start radius", &x));
            dims.push(dim("end radius", &y));
            Radius::Linear(x.v, y.v)
        }
        (v, None) => {
            return Err(Error::WrongType {
                span: a.span("from"),
                what: "vary(a, b) or vary([(t, r), ..])",
                want: Ty::List,
                found: v.ty(),
            });
        }
    };
    Ok(wired(W::Radius(r, dims)))
}

fn radius_of(v: Value, span: Span, dims: &mut Vec<Dimension>) -> R<Radius> {
    match v {
        Value::Wired(w) => match &*w {
            W::Radius(r, d) => {
                dims.extend(d.iter().cloned());
                Ok(r.clone())
            }
            o => Err(Error::WrongType {
                span,
                what: "a radius (a length or vary(..))",
                want: Ty::Num,
                found: o.ty(),
            }),
        },
        v => {
            let r = num_of(v, LEN, span, "a fillet radius")?;
            positive(&r, span, "a fillet radius")?;
            dims.push(dim("radius", &r));
            Ok(Radius::Constant(r.v))
        }
    }
}

/// The part's vertex at a point (an anchor), by its name.
fn vertex_at(ev: &mut Eval, v: Value, span: Span) -> R<Name> {
    let p = point_of(ev, v, span)?;
    let Some(b) = ev.b.body() else {
        return Ok(Name::NONE);
    };
    let at: Vec<Name> = b
        .vertex_ids()
        .filter(|&x| b.vertex(x).point.distance(p) <= 1e-6 * (1.0 + p.distance(forge_math::Point3::ORIGIN)))
        .map(|x| b.vertex(x).name)
        .collect();
    match at[..] {
        [one] => Ok(one),
        _ => Err(FeatError::NotOne {
            span,
            what: "the part's vertex at the point",
            count: at.len(),
        }
        .into()),
    }
}

pub fn fillet(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "fillet", span)?;
    let id = ev.take_id("fillet", span)?;
    let a = Args::bind(
        ev,
        "fillet",
        &["edges", "r", "groups", "from", "setbacks", "only"],
        args,
        span,
    )?;
    // (edges, radius, start vertex) per group
    let mut groups: Vec<(Value, Span, Value, Option<Value>)> = vec![];
    if let Some(e) = a.opt("edges") {
        groups.push((e, a.span("edges"), a.req("r")?, a.opt("from")));
    } else if a.opt("from").is_some() {
        return Err(FeatError::NotApplicable {
            span: a.span("from"),
            func: "fillet",
            arg: "from",
            because: crate::feat_error::Because::OtherForm,
        }
        .into());
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
                Value::Tuple(t) if t.len() == 2 || t.len() == 3 => {
                    groups.push((t[0].clone(), a.span("groups"), t[1].clone(), t.get(2).cloned()))
                }
                o => {
                    return Err(Error::WrongType {
                        span: a.span("groups"),
                        what: "a group (edges, radius) or (edges, radius, from)",
                        want: Ty::Tuple,
                        found: o.ty(),
                    });
                }
            }
        }
    }
    if groups.is_empty() {
        return Err(Error::MissingArg {
            span,
            func: "fillet",
            arg: "edges and r, or groups",
        });
    }
    let mut dims = vec![];
    let (mut edges, mut starts) = (vec![], vec![]);
    for (v, sp, r, from) in groups {
        let r = radius_of(r, sp, &mut dims)?;
        let from = match from {
            Some(f) if ev.b.building => Some(vertex_at(ev, f, sp)?),
            _ => None,
        };
        match edges_value(v, sp)? {
            Some(es) => {
                if es.is_empty() && ev.b.building {
                    return Err(Error::EmptySelection { span: sp });
                }
                for e in es {
                    edges.push((e, r.clone()));
                    if let Some(f) = from {
                        starts.push((e, f));
                    }
                }
            }
            None if ev.b.building && ev.errors.is_empty() => return Err(Error::EmptySelection { span: sp }),
            None if ev.b.building => return Ok(Value::Unresolved),
            None => {}
        }
    }
    let mut setbacks = vec![];
    if let Some(s) = a.opt("setbacks") {
        for x in crate::wired::items(s) {
            let Value::Tuple(t) = &x else {
                return Err(Error::WrongType {
                    span: a.span("setbacks"),
                    what: "a setback (point, length) or (point, edges, length)",
                    want: Ty::Tuple,
                    found: x.ty(),
                });
            };
            let sp = a.span("setbacks");
            let (p, e, d) = match &t[..] {
                [p, d] => (p.clone(), None, d.clone()),
                [p, e, d] => (p.clone(), Some(e.clone()), d.clone()),
                _ => {
                    return Err(Error::WrongType {
                        span: sp,
                        what: "a setback (point, length) or (point, edges, length)",
                        want: Ty::Tuple,
                        found: Ty::Tuple,
                    });
                }
            };
            let d = num_of(d, LEN, sp, "a setback")?;
            positive(&d, sp, "a setback")?;
            dims.push(dim("setback", &d));
            if !ev.b.building {
                continue;
            }
            let v = vertex_at(ev, p, sp)?;
            let e = match e {
                None => None,
                Some(ev_) => match edges_value(ev_, sp)? {
                    Some(es) if es.len() == 1 => Some(es[0]),
                    es => {
                        return Err(FeatError::NotOne {
                            span: sp,
                            what: "a setback's edge",
                            count: es.map_or(0, |x| x.len()),
                        }
                        .into());
                    }
                },
            };
            setbacks.push((v, e, d.v));
        }
    }
    let f = Feature::Fillet {
        edges,
        starts,
        setbacks,
        only: a.flag("only")?,
    };
    let (idx, hid) = ev.push_feature(&id, "fillet", f, span, dims)?;
    Ok(feat(id, "fillet", idx, hid, None))
}

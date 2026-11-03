//! `project(..)` in a sketch: edges, faces' boundaries, points and axes of the part as it stands
//! laid on the sketch's plane and held there (fixed), read afresh on every run. Exact images
//! (lines, circles, arcs, ellipses), or with `approx: TOL` a fitted spline for a general curve.

use crate::builtins::{Args, LEN, in_sketch, mk_ent, pending_name, positive};
use crate::error::EntKind;
use crate::eval::{Ent, Eval, R, Value};
use crate::feat_error::FeatError;
use crate::sketch::Sk;
use crate::syntax::{Arg, Span};
use forge_agent::select;
use forge_doc::sketch::{Flat, Source, flat};
use forge_geom::Line;
use forge_math::Point2;
use forge_sketch::{Constraint, EntityId};
use forge_topo::{Body, EdgeId, Name};
use std::cell::RefCell;
use std::rc::Rc;

pub fn project(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    let sk = in_sketch(ev, "project", span)?;
    let a = Args::bind(ev, "project", &["of", "approx"], args, span)?;
    if a.poisoned() || !ev.b.building {
        return Ok(Value::Unresolved);
    }
    let approx = match a.num("approx", LEN)? {
        Some(t) => Some(positive(&t, a.span("approx"), "an approx tolerance")?),
        None => None,
    };
    let sources = sources(ev, a.req("of")?, a.span("of"))?;
    let name = pending_name(ev, span);
    let plane = sk.borrow().frame;
    let mut made = vec![];
    for (k, s) in sources.iter().enumerate() {
        let f = flat(&plane, s, approx).map_err(|why| -> crate::error::Error {
            if approx.is_none() && matches!(s, Source::Edge { .. }) && forge_doc::sketch::needs_fit(&plane, s)
            {
                FeatError::NeedsApprox { span: a.span("of") }.into()
            } else {
                FeatError::NoImage {
                    span: a.span("of"),
                    why,
                }
                .into()
            }
        })?;
        let nm = name.as_ref().map(|n| {
            if sources.len() == 1 {
                n.clone()
            } else {
                format!("{n}.{k}")
            }
        });
        made.push(laid(&sk, f, nm.as_deref(), span)?);
    }
    Ok(match made[..] {
        [one] => mk_ent(&sk, one),
        _ => Value::Ent(Ent {
            sk: sk.clone(),
            id: made[0],
            kind: EntKind::Loop,
            loop_segs: Rc::new(made),
        }),
    })
}

/// The 3D geometry a value stands for.
fn sources(ev: &mut Eval, v: Value, span: Span) -> R<Vec<Source>> {
    let edge = |b: &Body, e: EdgeId| {
        let x = b.edge(e);
        Source::Edge {
            curve: x.curve.clone(),
            range: x.range,
            start: b.vertex(x.start).point,
            end: b.vertex(x.end).point,
            closed: x.start == x.end,
        }
    };
    Ok(match v {
        Value::Edges(es) => {
            let b = ev.b.body().expect("built");
            b.edge_ids()
                .filter(|&e| es.contains(&b.edge(e).name))
                .map(|e| edge(b, e))
                .collect()
        }
        Value::Faces(fs) => {
            let b = ev.b.body().expect("built");
            let mut out: Vec<Name> = vec![];
            let mut srcs = vec![];
            for f in b.face_ids().filter(|&f| fs.names.contains(&b.face(f).name)) {
                for e in select::edges_of_face(b, f) {
                    if !out.contains(&b.edge(e).name) {
                        out.push(b.edge(e).name);
                        srcs.push(edge(b, e));
                    }
                }
            }
            srcs
        }
        Value::Point(p) => vec![Source::Point(p)],
        Value::Axis(o, d) => vec![Source::Axis(Line { origin: o, dir: d })],
        Value::Wired(ref w) if w.point().is_some() => vec![Source::Point(w.point().expect("checked"))],
        Value::Wired(ref w) if w.axis().is_some() => {
            let (o, d) = w.axis().expect("checked");
            vec![Source::Axis(Line { origin: o, dir: d })]
        }
        o => {
            return Err(crate::error::Error::WrongType {
                span,
                what: "edges, faces, a point or an axis to project",
                want: crate::values::Ty::Edges,
                found: o.ty(),
            });
        }
    })
}

/// The flat image as sketch entities, every point fixed where it lies.
fn laid(sk: &Rc<RefCell<Sk>>, f: Flat, name: Option<&str>, span: Span) -> R<EntityId> {
    let mut s = sk.borrow_mut();
    let pin = |s: &mut Sk, p: Point2| s.point(p, true, None, span);
    Ok(match f {
        Flat::Point(p) => s.point(p, true, name, span)?,
        Flat::Line(a, b) => {
            let (a, b) = (pin(&mut s, a)?, pin(&mut s, b)?);
            s.line(a, b, false, name, span)?
        }
        Flat::Circle(c, r) => {
            let c = pin(&mut s, c)?;
            s.circle(c, r, name, span)?
        }
        Flat::Arc {
            center,
            start,
            end,
            ccw,
        } => {
            let (c, a, b) = (pin(&mut s, center)?, pin(&mut s, start)?, pin(&mut s, end)?);
            s.arc(c, a, b, ccw, name, span)?
        }
        Flat::Ellipse {
            center,
            major,
            b,
            ends,
        } => {
            let (c, m) = (pin(&mut s, center)?, pin(&mut s, major)?);
            let kerr = |error| crate::error::Error::SketchKernel {
                span,
                sketch: String::new(),
                error,
            };
            let e = match ends {
                Some((p, q, ccw)) => {
                    let (p, q) = (pin(&mut s, p)?, pin(&mut s, q)?);
                    s.s.add_ellipse_arc(c, m, b, p, q, ccw).map_err(kerr)?
                }
                None => {
                    let e = s.s.add_ellipse(c, m, b).map_err(kerr)?;
                    s.constrain(Constraint::EllipseMinor(e, b), span)?;
                    e
                }
            };
            s.name(e, name, span);
            e
        }
        Flat::Spline(pts, closed) => {
            let ids = pts.iter().map(|&p| pin(&mut s, p)).collect::<R<Vec<_>>>()?;
            s.spline(&ids, closed, name, span)?
        }
    })
}

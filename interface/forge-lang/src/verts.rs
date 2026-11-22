//! Vertex selections: `verts(of:, between:, body:, extreme:)`, the part's vertices found as its
//! edges and faces are (by what made the faces round them, through later booleans), so a work
//! feature, a fillet's start or setback, or a pattern's place names a vertex the kernel follows
//! through every edit above it (`.fc`'s `(verts ..)`), never a point frozen where it once was.

use crate::builtins::{Args, dir_of, feats_of, not_in_sketch};
use crate::error::Error;
use crate::eval::{Eval, Feat, R, Value};
use crate::syntax::{Arg, Span};
use crate::values::Ty;
use forge_agent::select;
use forge_math::Point3;
use forge_topo::{Body, FaceId, Name, VertexId};
use std::collections::HashSet;
use std::rc::Rc;

/// One side of `of` or `between`: the faces features made, or these faces or edges.
enum By {
    Feats(Vec<Rc<Feat>>),
    Names(Vec<Name>),
}

fn side(v: Value, span: Span) -> R<By> {
    match v {
        Value::Faces(fs) => Ok(By::Names(fs.names.clone())),
        Value::Edges(es) => Ok(By::Names(es.to_vec())),
        v => Ok(By::Feats(feats_of(v, span)?)),
    }
}

/// The faces and edges round each live vertex of the part.
fn around(body: &Body) -> Vec<(VertexId, Vec<FaceId>, Vec<Name>)> {
    let live = select::verts(body);
    let mut out: Vec<(VertexId, Vec<FaceId>, Vec<Name>)> =
        live.iter().map(|&v| (v, vec![], vec![])).collect();
    let mut at = vec![usize::MAX; body.vertices.len()];
    for (k, &v) in live.iter().enumerate() {
        at[v.index()] = k;
    }
    for e in body.edge_ids() {
        let edge = body.edge(e);
        let fs = select::faces_of_edge(body, e);
        for v in [edge.start, edge.end] {
            let k = at[v.index()];
            if k == usize::MAX {
                continue;
            }
            let slot = &mut out[k];
            slot.2.push(edge.name);
            for &f in &fs {
                if !slot.1.contains(&f) {
                    slot.1.push(f);
                }
            }
        }
    }
    out
}

pub fn verts(ev: &mut Eval, args: &[Arg], span: Span) -> R<Value> {
    not_in_sketch(ev, "verts", span)?;
    let a = Args::bind(ev, "verts", &["of", "between", "body", "extreme"], args, span)?;
    if a.poisoned() {
        return Ok(Value::Unresolved);
    }
    let of = a.opt("of").map(|v| side(v, a.span("of"))).transpose()?;
    let between = match a.opt("between") {
        None => None,
        Some(Value::Tuple(t)) | Some(Value::List(t)) if t.len() == 2 => Some((
            side(t[0].clone(), a.span("between"))?,
            side(t[1].clone(), a.span("between"))?,
        )),
        Some(o) => {
            return Err(Error::WrongType {
                span: a.span("between"),
                what: "between (two features or face sets)",
                want: Ty::Tuple,
                found: o.ty(),
            });
        }
    };
    let bodies = a.opt("body").map(|v| feats_of(v, a.span("body"))).transpose()?;
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
    if !ev.b.building || ev.b.built() == 0 {
        return Ok(Value::Unresolved);
    }
    let n = ev.b.built();
    let unbuilt = |b: &By| matches!(b, By::Feats(fs) if fs.iter().any(|f| n <= f.idx));
    if of.as_ref().is_some_and(unbuilt)
        || between.as_ref().is_some_and(|(x, y)| unbuilt(x) || unbuilt(y))
        || bodies.iter().flatten().any(|f| n <= f.idx)
    {
        return Ok(Value::Unresolved);
    }
    // the chosen bodies' edges (a vertex is in a body when an edge of it ends there)
    let in_bodies: Option<HashSet<Name>> = bodies.map(|fs| {
        let ids: Vec<usize> = fs.iter().map(|f| f.idx).collect();
        let mut keep = HashSet::new();
        if let Some(stage) = ev.b.stages.last()
            && let Some(b) = stage.view()
        {
            for s in stage.spans().iter().filter(|s| ids.contains(&s.solid)) {
                keep.extend(s.edges.clone().map(|e| b.edges[e as usize].name));
            }
        }
        keep
    });
    let origin = ev.b.origin(n);
    let body = ev.b.body().expect("built");
    let touches = |fs: &[FaceId], es: &[Name], by: &By| match by {
        By::Feats(set) => fs.iter().any(|&f| {
            origin
                .get(&body.face(f).name)
                .is_some_and(|o| set.iter().any(|x| o.contains(&x.idx)))
        }),
        By::Names(ns) => {
            fs.iter().any(|&f| ns.contains(&body.face(f).name)) || es.iter().any(|e| ns.contains(e))
        }
    };
    let mut vs: Vec<(VertexId, Point3)> = around(body)
        .into_iter()
        .filter(|(_, fs, es)| {
            of.as_ref().is_none_or(|by| touches(fs, es, by))
                && between
                    .as_ref()
                    .is_none_or(|(x, y)| touches(fs, es, x) && touches(fs, es, y))
                && in_bodies
                    .as_ref()
                    .is_none_or(|keep| es.iter().any(|e| keep.contains(e)))
        })
        .map(|(v, ..)| (v, body.vertex(v).point))
        .collect();
    if let Some(d) = extreme {
        let at = |p: Point3| (p - Point3::ORIGIN).dot(d);
        let top = vs.iter().map(|x| at(x.1)).fold(f64::NEG_INFINITY, f64::max);
        vs.retain(|x| at(x.1) >= top - 1e-6 * top.abs().max(1.0));
    }
    Ok(Value::Verts(Rc::new(
        vs.iter().map(|x| body.vertex(x.0).name).collect(),
    )))
}

/// The one vertex a selection found, by name: none or several is `NotOne`.
pub fn one(v: &[Name], span: Span, what: &'static str) -> R<Name> {
    match v {
        [one] => Ok(*one),
        ns => Err(crate::feat_error::FeatError::NotOne {
            span,
            what,
            count: ns.len(),
        }
        .into()),
    }
}

/// Where the part's vertex `name` is.
pub fn point(ev: &Eval, name: Name) -> Option<Point3> {
    let b = ev.b.body()?;
    b.vertex_ids()
        .find(|&v| b.vertex(v).name == name)
        .map(|v| b.vertex(v).point)
}

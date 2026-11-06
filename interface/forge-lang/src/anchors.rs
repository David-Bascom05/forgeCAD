//! What a pick on the finished part is in Forge: for each face and edge an anchor names, the
//! anchor's text (`base.end`, `boss.side(plan.outline[1])`, `edges(between: (a, b))`), found by
//! evaluating candidate anchors on the part as it stands, so the text is what the evaluator itself
//! resolves. A UI writes these into the source it generates; a face or edge no anchor names has
//! no stable Forge name, and the UI says so rather than writing a coordinate or a hash.

use crate::build::role_faces;
use crate::error::EntKind;
use crate::eval::{Eval, Value};
use crate::syntax::{self, StmtKind};
use forge_topo::{Body, Name};
use std::collections::HashMap;

/// A sketch's references: its plane and its named entities, as text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SketchRefs {
    pub id: String,
    /// Lines (and the line segments of loops): axes, directions, paths.
    pub lines: Vec<String>,
    /// Closed entities and loops: profiles.
    pub closed: Vec<String>,
    /// Named points, and `S.points` when it has any.
    pub points: Vec<String>,
    /// Other curves (arcs, splines): paths.
    pub curves: Vec<String>,
}

/// A sheet metal edge a flange can stand on: its text (`base.edge(plan.outline[1])`, `up.tip`)
/// and the edges of the part it is.
#[derive(Clone, Debug, PartialEq)]
pub struct SheetEdge {
    pub text: String,
    pub names: Vec<Name>,
}

#[derive(Clone, Debug, Default)]
pub struct Anchors {
    /// Each face name of the finished part an anchor finds, and that anchor's text: the one
    /// naming the fewest faces.
    pub faces: HashMap<Name, String>,
    /// How many faces of the part each face anchor names (more than one: a set, not the face).
    pub sizes: HashMap<String, usize>,
    /// Each edge between two anchored faces: `edges(between: (A, B))`, checked to hold it.
    pub edges: HashMap<Name, String>,
    pub sketches: Vec<SketchRefs>,
    /// The plane of each planar face anchor and each sketch (`S.plane`), as the evaluator
    /// carries it (x and origin from the sketch): where a sketch on it is drawn.
    pub planes: HashMap<String, forge_math::Frame>,
    pub sheet_edges: Vec<SheetEdge>,
    /// The features (and loop instances) anchors were looked for on, by id text.
    pub features: Vec<String>,
}

/// The members a feature's faces are found by (each tried; one a kind lacks is skipped).
const FACE_ROLES: [&str; 18] = [
    "faces", "start", "end", "floor", "mouth", "wall", "bore", "seat", "sink", "inner", "rim", "top",
    "bottom", "flanks", "root", "chamfer", "bend", "bend_top",
];

fn eval_text(ev: &mut Eval, text: &str) -> Option<Value> {
    let prog = syntax::parse(&format!("anchor_probe_ = {text}\n")).ok()?;
    let StmtKind::Assign { value, .. } = &prog.first()?.kind else {
        return None;
    };
    let (errors, warnings) = (ev.errors.len(), ev.warnings.len());
    let v = ev.expr(value).ok();
    // a probe leaves nothing behind
    ev.errors.truncate(errors);
    ev.warnings.truncate(warnings);
    v
}

/// What an expression asked of the finished part (`Options::asks`) evaluated to.
#[derive(Clone, Debug, PartialEq)]
pub enum Asked {
    Plane(forge_math::Frame),
    /// Faces by name, and their plane when they lie in one.
    Faces(Vec<Name>, Option<forge_math::Frame>),
    Edges(Vec<Name>),
    Point(forge_math::Point3),
    Axis(forge_math::Point3, forge_math::Vec3),
    /// Something else: its type's name.
    Other(String),
    /// It did not evaluate.
    Nothing,
}

pub fn ask(ev: &mut Eval, text: &str) -> Asked {
    match eval_text(ev, text) {
        None => Asked::Nothing,
        Some(Value::Plane(p)) => Asked::Plane(p.frame),
        Some(Value::Faces(f)) => Asked::Faces(f.names.clone(), f.plane),
        Some(Value::Edges(e)) => Asked::Edges(e.to_vec()),
        Some(Value::Point(p)) => Asked::Point(p),
        Some(Value::Axis(o, d)) | Some(Value::Line(o, d, _)) => Asked::Axis(o, d.get()),
        Some(Value::Sketch(s)) => Asked::Plane(s.borrow().frame),
        Some(v) => Asked::Other(format!("{:?}", v.ty())),
    }
}

fn face_names(v: &Value) -> Option<Vec<Name>> {
    match v {
        Value::Faces(fs) if !fs.names.is_empty() => Some(fs.names.clone()),
        _ => None,
    }
}

fn sketch_refs(id: &str, v: &Value) -> Option<SketchRefs> {
    let Value::Sketch(s) = v else { return None };
    let sk = s.borrow();
    let mut r = SketchRefs {
        id: id.to_string(),
        ..SketchRefs::default()
    };
    if !sk.points.is_empty() {
        r.points.push(format!("{id}.points"));
    }
    for (name, val) in &sk.named {
        let Value::Ent(e) = val else { continue };
        let text = format!("{id}.{name}");
        match e.kind {
            EntKind::Point => r.points.push(text),
            EntKind::Line => r.lines.push(text),
            EntKind::Circle => r.closed.push(text),
            EntKind::Spline if sk.closed_curve(e.id) => r.closed.push(text),
            EntKind::Arc | EntKind::Spline => r.curves.push(text),
            EntKind::Loop => {
                for k in 0..e.loop_segs.len() {
                    r.lines.push(format!("{text}[{k}]"));
                }
                r.closed.push(text);
            }
        }
    }
    Some(r)
}

/// A loop's instances: `bolt[2]`, and a nested loop's `bpsk[0][1]`.
fn instances(text: &str, v: &Value, out: &mut Vec<(String, Value)>) {
    match v {
        Value::Map(m) => {
            for (k, x) in m.iter() {
                instances(&format!("{text}[{k}]"), x, out);
            }
        }
        Value::Feature(_) | Value::Sketch(_) | Value::Sheet(_) => out.push((text.to_string(), v.clone())),
        _ => {}
    }
}

/// The anchors of the part `ev` finished, for its joined `body`.
pub fn find(ev: &mut Eval, body: Option<&Body>) -> Anchors {
    let mut out = Anchors::default();
    // what the top level bound: features, loop maps of them, sketches, sheets
    let mut bases: Vec<(String, Value)> = Vec::new();
    for (name, v) in ev.top_level() {
        match &v {
            Value::Map(_) => instances(&name, &v, &mut bases),
            Value::Feature(_) | Value::Sketch(_) | Value::Sheet(_) => bases.push((name, v)),
            _ => {}
        }
    }
    bases.sort_by(|a, b| a.0.cmp(&b.0));
    let sketches: Vec<SketchRefs> = bases.iter().filter_map(|(n, v)| sketch_refs(n, v)).collect();
    // every role, and every side a sketch entity made
    let mut sets: Vec<(String, Vec<Name>)> = Vec::new();
    for (base, v) in &bases {
        if matches!(v, Value::Sketch(_)) {
            continue;
        }
        out.features.push(base.clone());
        let mut texts: Vec<String> = FACE_ROLES.iter().map(|m| format!("{base}.{m}")).collect();
        if let Value::Feature(f) = v {
            let sk = f
                .prof
                .clone()
                .or_else(|| f.shape.as_ref().map(|s| s.sketch.clone()));
            if let Some(sk) = sk {
                let sk = sk.borrow();
                if let Some(refs) = sketches.iter().find(|r| r.id == sk.id) {
                    for e in refs.lines.iter().chain(&refs.closed).chain(&refs.curves) {
                        texts.push(format!("{base}.side({e})"));
                    }
                }
            }
        }
        for t in texts {
            if let Some(names) = eval_text(ev, &t).as_ref().and_then(face_names) {
                sets.push((t, names));
            }
        }
        // a sheet's edges a flange can stand on
        if matches!(v, Value::Sheet(_)) {
            let mut cands: Vec<String> = ["tip", "start_side", "end_side"]
                .iter()
                .map(|m| format!("{base}.{m}"))
                .collect();
            for r in &sketches {
                for e in r.lines.iter().chain(&r.curves) {
                    cands.push(format!("{base}.edge({e})"));
                }
            }
            for t in cands {
                if let Some(Value::Edges(names)) = eval_text(ev, &format!("{t}.edges"))
                    && !names.is_empty()
                {
                    out.sheet_edges.push(SheetEdge {
                        text: t,
                        names: names.to_vec(),
                    });
                }
            }
        }
    }
    for r in &sketches {
        let t = format!("{}.plane", r.id);
        if let Some(Value::Plane(p)) = eval_text(ev, &t) {
            out.planes.insert(t, p.frame);
        }
    }
    out.sketches = sketches;
    let Some(body) = body else { return out };
    // each face: the anchor naming it among the fewest faces
    let mut best: HashMap<Name, (usize, String)> = HashMap::new();
    for (t, names) in &sets {
        let faces: Vec<_> = names.iter().flat_map(|&n| role_faces(body, n)).collect();
        for f in &faces {
            let n = body.face(*f).name;
            let better = best.get(&n).is_none_or(|b| faces.len() < b.0);
            if better {
                best.insert(n, (faces.len(), t.clone()));
            }
        }
    }
    for (k, t) in best.values() {
        out.sizes.insert(t.clone(), *k);
    }
    out.faces = best.into_iter().map(|(n, (_, t))| (n, t)).collect();
    let mut texts: Vec<String> = out.faces.values().cloned().collect();
    texts.sort();
    texts.dedup();
    for t in texts {
        let pt = format!("{t}.plane");
        if let Some(Value::Plane(p)) = eval_text(ev, &pt) {
            out.planes.insert(pt, p.frame);
        }
    }
    // each edge between two differently anchored faces, if the selection holds it
    let mut tried: HashMap<(String, String), Vec<Name>> = HashMap::new();
    for e in body.edge_ids() {
        let fs = forge_agent::select::faces_of_edge(body, e);
        let [a, b] = fs[..] else { continue };
        let (Some(ta), Some(tb)) = (
            out.faces.get(&body.face(a).name).cloned(),
            out.faces.get(&body.face(b).name).cloned(),
        ) else {
            continue;
        };
        if ta == tb {
            continue;
        }
        let key = if ta < tb { (ta, tb) } else { (tb, ta) };
        let text = format!("edges(between: ({}, {}))", key.0, key.1);
        let names = tried.entry(key).or_insert_with(|| match eval_text(ev, &text) {
            Some(Value::Edges(n)) => n.to_vec(),
            _ => vec![],
        });
        let name = body.edge(e).name;
        if names.contains(&name) {
            out.edges.insert(name, text);
        }
    }
    out
}

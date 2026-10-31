//! Finding entities without knowing their names: selectors, short handles, and which feature
//! made each face.

use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::{Point3, Vec3};
use forge_topo::{Body, EdgeId, FaceId, Name, VertexId};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ent {
    Face(FaceId),
    Edge(EdgeId),
    Vert(VertexId),
}

/// A body ready to be asked about: compacted, with every face's making feature.
pub struct View<'a> {
    pub body: &'a Body,
    /// Each face's making feature and the patterns that copied it there.
    pub origin: &'a Lineage,
    /// Resolves a feature reference (label, `hole2`, `fN`) to its index.
    pub feat: &'a dyn Fn(&str) -> Result<usize, String>,
    /// The body (the feature that made it) an entity belongs to.
    pub body_of: &'a dyn Fn(Ent) -> Option<usize>,
}

/// Each face's making feature after feature `i` built `body`, given those before (`prev`, `None`
/// for the first). A face keeps its origin while its name lives, and a renamed piece of a face
/// (split by a later boolean) keeps the origin of the face it overlaps on the same surface.
/// `copied` names the faces a pattern at `i` copied: their source's lineage, then `i`.
pub fn lineage(body: &Body, prev: Option<(&Body, &Lineage)>, i: usize, copied: &Lineage) -> Lineage {
    let mut out = HashMap::with_capacity(body.faces.len());
    for f in body.face_ids() {
        let face = body.face(f);
        let o = prev.and_then(|(pb, po)| {
            po.get(&face.name)
                .cloned()
                .or_else(|| copied.get(&face.name).cloned())
                .or_else(|| {
                    // a new face on a far face's surface (a second pad's top) is not that face
                    pb.face_ids()
                        .find(|&g| {
                            pb.face(g).surface == face.surface && pb.face_box(g).overlaps(&body.face_box(f))
                        })
                        .and_then(|g| po.get(&pb.face(g).name).cloned())
                })
        });
        out.insert(face.name, o.unwrap_or_else(|| vec![i]));
    }
    out
}

/// The faces a pattern at `k` copied, from the part before it (`prev`, `po`): each new face is
/// carried back by every copy's motion onto the surface of a source face `from` accepts, and
/// gets that face's lineage and `k`. Geometric, so faces split by the copy still trace back.
pub fn copies(
    body: &Body,
    prev: &Body,
    po: &Lineage,
    k: usize,
    features: forge_model::History<'_>,
    layout: &forge_model::Layout,
    from: &dyn Fn(&Vec<usize>) -> bool,
) -> Lineage {
    let sources: Vec<FaceId> = prev
        .face_ids()
        .filter(|&g| po.get(&prev.face(g).name).is_some_and(from))
        .collect();
    let mut out = HashMap::new();
    for f in body.face_ids() {
        let face = body.face(f);
        if po.contains_key(&face.name) {
            continue;
        }
        // a point on the face's surface, near its middle
        let c = centre(body, Ent::Face(f));
        let p = face.surface.eval(face.surface.project(c));
        let back = forge_model::copy_sources(features, k, layout, p).unwrap_or_default();
        let hit = sources.iter().find(|&&g| {
            let s = &prev.face(g).surface;
            surface_kind(s) == surface_kind(&face.surface)
                && radius(prev, Ent::Face(g)) == radius(body, Ent::Face(f))
                && back.iter().any(|q| s.eval(s.project(*q)).distance(*q) < 1e-6)
        });
        if let Some(g) = hit {
            let mut c = po[&prev.face(*g).name].clone();
            c.push(k);
            out.insert(face.name, c);
        }
    }
    out
}

/// Each face's making feature, then the patterns that copied it (by feature index).
pub type Lineage = HashMap<Name, Vec<usize>>;

pub fn name(body: &Body, e: Ent) -> Name {
    match e {
        Ent::Face(f) => body.face(f).name,
        Ent::Edge(x) => body.edge(x).name,
        Ent::Vert(v) => body.vertex(v).name,
    }
}

/// The vertices live edges end at (a part mid-history may hold dead ones).
pub fn verts(body: &Body) -> Vec<VertexId> {
    let mut used = vec![false; body.vertices.len()];
    for e in body.edge_ids() {
        used[body.edge(e).start.index()] = true;
        used[body.edge(e).end.index()] = true;
    }
    body.vertex_ids().filter(|x| used[x.index()]).collect()
}

pub fn all(body: &Body) -> Vec<Ent> {
    let mut v: Vec<Ent> = body.face_ids().map(Ent::Face).collect();
    v.extend(body.edge_ids().map(Ent::Edge));
    v.extend(verts(body).into_iter().map(Ent::Vert));
    v
}

/// Short handles: the fewest hex digits of each name (at least 4) that no other entity shares.
pub fn handles(body: &Body) -> HashMap<Name, String> {
    let mut names: Vec<u64> = all(body).into_iter().map(|e| name(body, e).0).collect();
    names.sort_unstable();
    names.dedup();
    let common = |a: u64, b: u64| ((a ^ b).leading_zeros() / 4) as usize;
    let mut out = HashMap::with_capacity(names.len());
    for (k, &n) in names.iter().enumerate() {
        let mut d = 4;
        if k > 0 {
            d = d.max(common(n, names[k - 1]) + 1);
        }
        if k + 1 < names.len() {
            d = d.max(common(n, names[k + 1]) + 1);
        }
        out.insert(Name(n), format!("@{}", &format!("{n:016x}")[..d.min(16)]));
    }
    out
}

/// The entity a handle (`@` and a unique prefix of its name's hex) names.
pub fn by_handle(body: &Body, h: &str) -> Result<Ent, String> {
    let p = h.strip_prefix('@').unwrap_or(h).to_ascii_lowercase();
    let hits: Vec<Ent> = all(body)
        .into_iter()
        .filter(|&e| format!("{:016x}", name(body, e).0).starts_with(&p))
        .collect();
    match hits[..] {
        [e] => Ok(e),
        [] => Err(format!("no entity {h} on the part here")),
        _ => Err(format!(
            "{h} is ambiguous ({} entities); use more digits",
            hits.len()
        )),
    }
}

pub fn faces_of_edge(body: &Body, e: EdgeId) -> Vec<FaceId> {
    let mut f: Vec<FaceId> = body
        .edge(e)
        .coedges
        .iter()
        .map(|&c| body.coedge_face(c))
        .collect();
    f.dedup();
    f
}

pub fn edges_of_face(body: &Body, f: FaceId) -> Vec<EdgeId> {
    let mut out = Vec::new();
    for &l in &body.face(f).loops {
        for &c in &body.lp(l).coedges {
            out.push(body.coedge(c).edge);
        }
    }
    out
}

pub fn surface_kind(s: &Surface) -> &'static str {
    match s {
        Surface::Plane(_) => "plane",
        Surface::Cylinder(_) => "cyl",
        Surface::Cone(_) => "cone",
        Surface::Sphere(_) => "sphere",
        Surface::Torus(_) => "torus",
        Surface::Procedural(_) => "blend",
    }
}

pub fn curve_kind(c: &Curve) -> &'static str {
    match c {
        Curve::Line(_) => "line",
        Curve::Circle(_) => "circle",
        _ => "curve",
    }
}

/// The outward normal of a planar face.
pub fn plane_normal(body: &Body, f: FaceId) -> Option<Vec3> {
    let face = body.face(f);
    match &face.surface {
        Surface::Plane(p) => {
            let z = p.frame.z().get();
            Some(if face.same_sense { z } else { -z })
        }
        _ => None,
    }
}

pub fn radius(body: &Body, e: Ent) -> Option<f64> {
    match e {
        Ent::Face(f) => match &body.face(f).surface {
            Surface::Cylinder(c) => Some(c.radius),
            Surface::Sphere(s) => Some(s.radius),
            Surface::Torus(t) => Some(t.minor),
            _ => None,
        },
        Ent::Edge(x) => match &body.edge(x).curve {
            Curve::Circle(c) => Some(c.radius),
            _ => None,
        },
        Ent::Vert(_) => None,
    }
}

pub fn length(body: &Body, e: EdgeId) -> f64 {
    let edge = body.edge(e);
    match &edge.curve {
        Curve::Line(_) => edge.range.width(),
        Curve::Circle(c) => c.radius * edge.range.width(),
        _ => body
            .edge_points(e, 64)
            .windows(2)
            .map(|w| w[0].distance(w[1]))
            .sum(),
    }
}

/// How an edge's two faces meet at its middle: "convex", "concave" or "smooth" (tangent or
/// coplanar), from the first face's inward direction against the second face's normal.
pub fn crease(body: &Body, e: EdgeId) -> Option<&'static str> {
    let edge = body.edge(e);
    let [c1, c2] = edge.coedges[..] else { return None };
    let t = edge.range.lo + edge.range.width() / 2.0;
    let d = edge.curve.deriv(t);
    let normal = |c: forge_topo::CoedgeId| {
        let f = body.coedge_face(c);
        body.face_normal(f, body.face(f).surface.project(d.p))
            .map(|n| n.get())
    };
    let (n1, n2) = (normal(c1)?, normal(c2)?);
    if n1.cross(n2).length() < 1e-6 && n1.dot(n2) > 0.0 {
        return Some("smooth");
    }
    // a face lies to the left of its coedges seen from outside
    let along = if body.coedge(c1).forward { d.d1 } else { -d.d1 };
    let inward = n1.cross(along);
    Some(if inward.dot(n2) > 0.0 { "concave" } else { "convex" })
}

/// A round entity's axis: a point on it level with the entity's middle, and its direction
/// (none for a sphere, whose point is its centre).
pub fn axis(body: &Body, e: Ent) -> Option<(Point3, Option<Vec3>)> {
    let frame = match e {
        Ent::Face(f) => match &body.face(f).surface {
            Surface::Cylinder(c) => c.frame,
            Surface::Cone(c) => c.frame,
            Surface::Torus(t) => t.frame,
            Surface::Sphere(s) => return Some((s.frame.origin, None)),
            _ => return None,
        },
        Ent::Edge(x) => match &body.edge(x).curve {
            Curve::Circle(c) => return Some((c.frame.origin, Some(c.frame.z().get()))),
            _ => return None,
        },
        Ent::Vert(_) => return None,
    };
    let z = frame.z().get();
    let along = (centre(body, e) - frame.origin).dot(z);
    Some((frame.origin + z * along, Some(z)))
}

/// A point that stands for the entity: a vertex, an edge's middle, a face's box centre.
pub fn centre(body: &Body, e: Ent) -> Point3 {
    match e {
        Ent::Vert(v) => body.vertex(v).point,
        Ent::Edge(x) => {
            let edge = body.edge(x);
            edge.curve.eval(edge.range.lo + edge.range.width() / 2.0)
        }
        Ent::Face(f) => {
            let b = body.face_box(f);
            Point3::new(
                (b.x.lo + b.x.hi) / 2.0,
                (b.y.lo + b.y.hi) / 2.0,
                (b.z.lo + b.z.hi) / 2.0,
            )
        }
    }
}

/// Points spread over the entity, for nearest-point questions.
pub fn samples(body: &Body, e: Ent) -> Vec<Point3> {
    match e {
        Ent::Vert(v) => vec![body.vertex(v).point],
        Ent::Edge(x) => body.edge_points(x, 32),
        Ent::Face(f) => {
            let mut p: Vec<Point3> = edges_of_face(body, f)
                .into_iter()
                .flat_map(|x| body.edge_points(x, 16))
                .collect();
            p.push(centre(body, e));
            p
        }
    }
}

/// `+z`, `-x` and the like, or none.
pub fn axis_word(w: &str) -> Option<Vec3> {
    let s = if w.starts_with('-') { -1.0 } else { 1.0 };
    let v = match w.trim_start_matches(['+', '-']) {
        "x" => Vec3::X,
        "y" => Vec3::Y,
        "z" => Vec3::Z,
        _ => return None,
    };
    Some(v * s)
}

/// The axis word for a direction along a world axis, or its rounded components.
pub fn dir_text(v: Vec3) -> String {
    for (w, a) in [("x", Vec3::X), ("y", Vec3::Y), ("z", Vec3::Z)] {
        let d = v.dot(a);
        if d.abs() > 1.0 - 1e-9 {
            return format!("{}{w}", if d > 0.0 { '+' } else { '-' });
        }
    }
    format!(
        "{},{},{}",
        crate::short(v.x),
        crate::short(v.y),
        crate::short(v.z)
    )
}

const EXTREMES: [(&str, Vec3); 6] = [
    ("top", Vec3::Z),
    ("bottom", Vec3::new(0.0, 0.0, -1.0)),
    ("right", Vec3::X),
    ("left", Vec3::new(-1.0, 0.0, 0.0)),
    ("back", Vec3::Y),
    ("front", Vec3::new(0.0, -1.0, 0.0)),
];

/// Runs a selector (without its parentheses) over the view.
pub fn select(v: &View, sel: &str) -> Result<Vec<Ent>, String> {
    let body = v.body;
    let toks: Vec<&str> = sel
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split_whitespace()
        .collect();
    let mut k = 0;
    let mut set: Vec<Ent> = match toks.first().copied() {
        Some("faces") => body.face_ids().map(Ent::Face).collect(),
        Some("edges") => body.edge_ids().map(Ent::Edge).collect(),
        Some("verts") => verts(body).into_iter().map(Ent::Vert).collect(),
        t => {
            return Err(format!(
                "a selector starts with faces, edges or verts, not '{}'",
                t.unwrap_or("")
            ));
        }
    };
    k += 1;
    let num = |k: usize| -> Result<f64, String> {
        let t = toks.get(k).copied().unwrap_or("");
        t.parse()
            .map_err(|_| format!("expected a number after '{}', found '{t}'", toks[k - 1]))
    };
    let origin = |f: FaceId| v.origin.get(&body.face(f).name).cloned();
    let face_origins = |e: Ent| -> Vec<Vec<usize>> {
        match e {
            Ent::Face(f) => origin(f).into_iter().collect(),
            Ent::Edge(x) => faces_of_edge(body, x).into_iter().filter_map(origin).collect(),
            Ent::Vert(_) => vec![],
        }
    };
    while k < toks.len() {
        let t = toks[k];
        k += 1;
        let kind = |e: &Ent| match *e {
            Ent::Face(f) => surface_kind(&body.face(f).surface),
            Ent::Edge(x) => curve_kind(&body.edge(x).curve),
            Ent::Vert(_) => "vert",
        };
        let keep: Box<dyn Fn(&Ent) -> bool> = match t {
            "plane" | "planar" | "cyl" | "cylindrical" | "cone" | "conical" | "sphere" | "spherical"
            | "torus" | "blend" | "line" | "linear" | "straight" | "circle" | "circular" | "curve" => {
                let want = match t {
                    "planar" => "plane",
                    "cylindrical" => "cyl",
                    "conical" => "cone",
                    "spherical" => "sphere",
                    "linear" | "straight" => "line",
                    "circular" => "circle",
                    w => w,
                };
                Box::new(move |e| kind(e) == want)
            }
            "convex" | "concave" | "smooth" => Box::new(move |e| match *e {
                Ent::Edge(x) => crease(body, x) == Some(t),
                _ => false,
            }),
            "normal" | "along" => {
                let d = match toks.get(k).and_then(|w| axis_word(w)) {
                    Some(d) => {
                        k += 1;
                        d
                    }
                    None => {
                        let d = Vec3::new(num(k)?, num(k + 1)?, num(k + 2)?);
                        k += 3;
                        d.normalize().ok_or("a zero direction")?.get()
                    }
                };
                if t == "normal" {
                    Box::new(move |e| match *e {
                        Ent::Face(f) => plane_normal(body, f).is_some_and(|n| n.dot(d) > 1.0 - 1e-9),
                        _ => false,
                    })
                } else {
                    Box::new(move |e| match *e {
                        Ent::Edge(x) => match &body.edge(x).curve {
                            Curve::Line(l) => l.dir.dot(d).abs() > 1.0 - 1e-9,
                            _ => false,
                        },
                        _ => false,
                    })
                }
            }
            _ if t.starts_with("r=") || t.starts_with("r<") || t.starts_with("r>") => {
                let r: f64 = t[2..].parse().map_err(|_| format!("bad radius in '{t}'"))?;
                let op = t.as_bytes()[1];
                Box::new(move |e| {
                    radius(body, *e).is_some_and(|x| match op {
                        b'=' => (x - r).abs() <= 1e-6 * r.abs().max(1.0),
                        b'<' => x < r,
                        _ => x > r,
                    })
                })
            }
            "longer" | "shorter" => {
                let n = num(k)?;
                k += 1;
                let longer = t == "longer";
                Box::new(move |e| match *e {
                    Ent::Edge(x) => (length(body, x) > n) == longer,
                    _ => false,
                })
            }
            "of" => {
                // one feature or more: `of boss`, `of feature 4`, `of boss hole1`
                let mut set = Vec::new();
                loop {
                    let mut name = toks.get(k).copied().unwrap_or("").to_string();
                    let mut used = 1;
                    if name == "feature" {
                        name = format!("f{}", toks.get(k + 1).copied().unwrap_or(""));
                        used = 2;
                    }
                    match (v.feat)(&name) {
                        Ok(i) => {
                            set.push(i);
                            k += used;
                        }
                        Err(e) if set.is_empty() => return Err(e),
                        Err(_) => break,
                    }
                }
                Box::new(move |e| face_origins(*e).iter().any(|c| c.iter().any(|o| set.contains(o))))
            }
            "body" => {
                let i = (v.feat)(toks.get(k).copied().unwrap_or(""))?;
                k += 1;
                Box::new(move |e| (v.body_of)(*e) == Some(i))
            }
            "between" => {
                let a = (v.feat)(toks.get(k).copied().unwrap_or(""))?;
                let b = (v.feat)(toks.get(k + 1).copied().unwrap_or(""))?;
                k += 2;
                Box::new(move |e| match &face_origins(*e)[..] {
                    [x, y] if matches!(e, Ent::Edge(_)) => {
                        (x.contains(&a) && y.contains(&b)) || (x.contains(&b) && y.contains(&a))
                    }
                    _ => false,
                })
            }
            "on" => {
                let at = by_handle(body, toks.get(k).copied().unwrap_or(""))?;
                k += 1;
                let touching: Vec<Ent> = match at {
                    Ent::Face(f) => {
                        let es = edges_of_face(body, f);
                        let mut out: Vec<Ent> = es.iter().map(|&x| Ent::Edge(x)).collect();
                        for &x in &es {
                            out.extend(
                                faces_of_edge(body, x)
                                    .into_iter()
                                    .filter(|&g| g != f)
                                    .map(Ent::Face),
                            );
                            out.extend([Ent::Vert(body.edge(x).start), Ent::Vert(body.edge(x).end)]);
                        }
                        out
                    }
                    Ent::Edge(x) => {
                        let mut out: Vec<Ent> = faces_of_edge(body, x).into_iter().map(Ent::Face).collect();
                        out.extend([Ent::Vert(body.edge(x).start), Ent::Vert(body.edge(x).end)]);
                        out
                    }
                    Ent::Vert(p) => body
                        .edge_ids()
                        .filter(|&x| body.edge(x).start == p || body.edge(x).end == p)
                        .flat_map(|x| {
                            let mut o = vec![Ent::Edge(x)];
                            o.extend(faces_of_edge(body, x).into_iter().map(Ent::Face));
                            o
                        })
                        .collect(),
                };
                Box::new(move |e| touching.contains(e))
            }
            "near" => {
                let p = Point3::new(num(k)?, num(k + 1)?, num(k + 2)?);
                k += 3;
                let d = |e: &Ent| {
                    samples(body, *e)
                        .iter()
                        .map(|q| q.distance(p))
                        .fold(f64::INFINITY, f64::min)
                };
                if let Some(best) = set.iter().min_by(|a, b| d(a).total_cmp(&d(b))).copied() {
                    set = vec![best];
                }
                continue;
            }
            _ => match EXTREMES.iter().find(|x| x.0 == t) {
                Some(&(_, dir)) => {
                    let at = move |e: &Ent| (centre(body, *e) - Point3::ORIGIN).dot(dir);
                    let top = set.iter().map(at).fold(f64::NEG_INFINITY, f64::max);
                    Box::new(move |e| at(e) >= top - 1e-6 * top.abs().max(1.0))
                }
                None => return Err(format!("unknown selector word '{t}'")),
            },
        };
        set.retain(|e| keep(e));
    }
    Ok(set)
}

/// One line about an entity: its kind and the numbers that tell it apart.
pub fn describe(body: &Body, e: Ent) -> String {
    // a round thing is placed by its axis, not by the middle of the (half) face
    let (c, ax) = match axis(body, e) {
        Some((p, Some(d))) => (p, format!("axis {} ", dir_text(d))),
        Some((p, None)) => (p, String::new()),
        None => (centre(body, e), String::new()),
    };
    let at = format!(
        "at {},{},{}",
        crate::short(c.x),
        crate::short(c.y),
        crate::short(c.z)
    );
    match e {
        Ent::Face(f) => {
            let s = &body.face(f).surface;
            let k = surface_kind(s);
            match (plane_normal(body, f), radius(body, e)) {
                (Some(n), _) => format!("{k} {} {at}", dir_text(n)),
                (_, Some(r)) => format!("{k} r={} {ax}{at}", crate::short(r)),
                _ => format!("{k} {at}"),
            }
        }
        Ent::Edge(x) => {
            let k = curve_kind(&body.edge(x).curve);
            let len = crate::short(length(body, x));
            match &body.edge(x).curve {
                Curve::Line(l) => format!("{k} {} len={len} {at}", dir_text(l.dir.get())),
                Curve::Circle(ci) => format!("{k} r={} len={len} {ax}{at}", crate::short(ci.radius)),
                _ => format!("{k} len={len} {at}"),
            }
        }
        Ent::Vert(_) => format!("vert {at}"),
    }
}

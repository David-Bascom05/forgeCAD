//! Rolling-ball fillets, all edges of one operation solved together.
//!
//! Each filleted edge becomes a blend face (`forge_geom::blend`) and its two
//! faces are cut back to the blend's spring curves. A tangent chain whose
//! radius varies (picked with a law, or edges picked with different radii)
//! gets one guide and one radius law (`forge_geom::varblend`), one canal
//! surface per edge, so its blends share their junction circles. What happens
//! at each end is decided per vertex, from the cycle of faces around it: a lone
//! blend runs out into the faces across its end, two blends of a tangent chain
//! meet in a shared cross-section, three equal blends meet in a sphere. Every
//! face corner the fillet touches is closed by a path of new edges, so a
//! corner may end in any trim curve.

use forge_geom::blend::{Blend, closed_form, spine_closest};
use forge_geom::intersect::{CurveSurface, curve_surface};
use forge_geom::varblend::{Guide, RadiusLaw, VarBlend, VarBlendError};
use forge_geom::{Curve, CurveGeom, Sphere, Surface, SurfaceGeom};
use forge_math::{Box3, Frame, Interval, Point3, UnitVec3, Vec3, dmath, tol};
use forge_topo::{Body, CoedgeId, EdgeId, FaceId, Name, VertexId};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// How the radius runs along a picked edge. Along means from the edge's start (`law_starts`)
/// to its other end, whichever way the edge itself runs; a closed edge runs from its vertex the
/// way it runs.
#[derive(Clone, Debug, PartialEq)]
pub enum Radius {
    Constant(f64),
    /// From the start to the other end, linear along the edge.
    Linear(f64, f64),
    /// Radii at fractions of the way along the edge from its start (0 to 1,
    /// increasing), with a smooth law through them.
    Points(Vec<(f64, f64)>),
}

impl From<f64> for Radius {
    fn from(r: f64) -> Radius {
        Radius::Constant(r)
    }
}

impl Radius {
    /// The same law with the edge walked the other way.
    pub fn reversed(&self) -> Radius {
        match self {
            Radius::Constant(r) => Radius::Constant(*r),
            Radius::Linear(a, b) => Radius::Linear(*b, *a),
            Radius::Points(p) => Radius::Points(p.iter().rev().map(|&(f, r)| (1.0 - f, r)).collect()),
        }
    }
    /// The (fraction along the edge, radius) points the pick fixes.
    fn points(&self) -> Vec<(f64, f64)> {
        match self {
            Radius::Constant(r) => vec![(0.0, *r), (1.0, *r)],
            Radius::Linear(a, b) => vec![(0.0, *a), (1.0, *b)],
            Radius::Points(p) => p.clone(),
        }
    }
    /// The law over each half of the edge, as fractions of that half.
    fn halves(&self) -> (Option<Radius>, Option<Radius>) {
        let p = self.points();
        let mut out = (Vec::new(), Vec::new());
        for &(f, r) in &p {
            if f <= 0.5 {
                out.0.push((2.0 * f, r));
            }
            if f >= 0.5 {
                out.1.push((2.0 * f - 1.0, r));
            }
        }
        let law = |pts: Vec<(f64, f64)>| (!pts.is_empty()).then_some(Radius::Points(pts));
        (law(out.0), law(out.1))
    }

    /// The radius, where it is the same all along.
    pub fn constant(&self) -> Option<f64> {
        let p = self.points();
        p.iter().all(|q| q.1 == p[0].1).then_some(p[0].1)
    }
    fn max(&self) -> f64 {
        self.points().iter().fold(0.0f64, |m, q| m.max(q.1))
    }
    fn valid(&self) -> bool {
        let p = self.points();
        !p.is_empty()
            && p.iter()
                .all(|q| q.1 > tol::MIN_LENGTH && q.1.is_finite() && (0.0..=1.0).contains(&q.0))
            && p.windows(2).all(|w| w[1].0 > w[0].0)
    }
}

/// One edge to round, with its radius.
#[derive(Clone, Debug, PartialEq)]
pub struct FilletEdge {
    pub edge: EdgeId,
    pub radius: Radius,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilletError {
    /// The edge is not a live edge between two faces.
    NotAnEdge { edge: EdgeId },
    /// The radius is not a usable length, or its law makes no blend (it changes or bends
    /// faster than the ball can roll).
    BadRadius { edge: EdgeId },
    /// The two faces are tangent along the edge: there is nothing to round.
    Smooth { edge: EdgeId },
    /// The two faces offset by the radius do not meet near the edge.
    NoBlend { edge: EdgeId },
    /// The blend runs past the edge or face it should end on.
    Overflow { edge: EdgeId },
    /// The blend ends exactly on a vertex or edge: zero-length or zero-thickness result.
    Tangency { edge: EdgeId },
    /// Several blends meet at this vertex in a way that needs a corner patch.
    Corner { vertex: VertexId },
    /// An edge is listed twice.
    Duplicate { edge: EdgeId },
    /// A ring's round whose ball is wider than the circle its centre rolls round; `max` is the
    /// largest radius that rounds it.
    TooLarge { edge: EdgeId, max: f64 },
}

type R<T> = Result<T, FilletError>;

/// A filleted edge's blend and the curves it touches its faces along.
struct Rolled {
    edge: EdgeId,
    convex: bool,
    blend: Blend,
    /// Spring curves on the surface of the face using `edge.coedges[i]`; they share the spine's parameter.
    springs: [Curve; 2],
    /// The spine runs the way the edge's curve does.
    aligned: bool,
    /// The faces on each side, and whether each walks the edge forward.
    faces: [FaceId; 2],
    coedges: [CoedgeId; 2],
    forward: [bool; 2],
    name: Name,
    var: Option<VarEdge>,
    /// Per side, where the ball rests not on the edge's face but on one tangent to it beyond a
    /// smooth edge of it: that face and the smooth edge. The strip of the edge's face between is
    /// swallowed.
    over: Over,
    /// Where on its edge a spring crosses a smooth edge of its face: the ball moves onto the face
    /// beyond there, so the edge is split there and each piece rolled by itself.
    split: Option<f64>,
}

/// Per blend side, the face rested on beyond a smooth edge, and that edge.
type Over = [Option<(FaceId, EdgeId)>; 2];

/// An edge an end face gets over a face rested on: it, its vertex on the smooth edge, its vertex
/// on the spring.
type Cap = (EdgeId, VertexId, VertexId);

/// How a switched blend's smooth edge ends where the blend does: cut at a parameter, or swallowed
/// up to its own vertex.
#[derive(Clone, Copy, Debug)]
enum GEnd {
    Cut(f64, VertexId),
    Gone(VertexId),
}

/// Where a variable blend's edge sits in its chain.
#[derive(Clone, Copy, Debug)]
struct VarEdge {
    chain: usize,
    index: usize,
    len: usize,
    closed: bool,
    /// The edge's own s range (the blend's runs on past the chain's ends).
    own: Interval,
}

/// Filleted edges that continue each other through their vertices, in order, each
/// walked forward or not.
struct Chain {
    members: Vec<(EdgeId, bool)>,
    closed: bool,
}

/// How a filleted edge ends at one of its vertices: per side, the vertex and
/// spring parameter where the spring stops, and the path closing the blend
/// face there (from the side whose face leaves the vertex along the edge to
/// the side whose face arrives).
struct SpringEnd {
    v: [VertexId; 2],
    t: [f64; 2],
    path: Vec<(EdgeId, bool)>,
}

/// Choices beyond the edges and their radii.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FilletOptions {
    /// The vertex an edge's law runs from: one of its ends, or the end of its run (`law_starts`).
    pub starts: Vec<(EdgeId, VertexId)>,
    /// How far from a corner vertex its blends stop (the length along each edge from the vertex
    /// to where the blend's section crosses it): for one edge there, or all of them.
    pub setbacks: Vec<(VertexId, Option<EdgeId>, f64)>,
}

/// The vertex each varying law of `edges` runs from. A run is the edges one varying radius was
/// given to, side by side in `edges`; it is walked from its start, each edge from the end it is
/// reached by. An edge given one of its ends starts there; the walk starts at the vertex given
/// for one of the run's edges, else the end of its first edge not shared with the second, else
/// (a lone edge) the end lowest in x, then y, then z.
/// Geometric, never by name: renaming cannot turn a law round. An edge the walk misses starts
/// at its own lowest end, or is refused where its start was given.
pub fn law_starts(
    body: &Body,
    edges: &[FilletEdge],
    given: &[(EdgeId, VertexId)],
) -> R<BTreeMap<EdgeId, VertexId>> {
    let ends = |e: EdgeId| (body.edge(e).start, body.edge(e).end);
    let lowest = |e: EdgeId| {
        let (a, b) = ends(e);
        let (p, q) = (body.vertex(a).point, body.vertex(b).point);
        let key = |p: Point3, q: Point3| {
            [(p.x, q.x), (p.y, q.y), (p.z, q.z)]
                .into_iter()
                .find(|(u, w)| (u - w).abs() > tol::LINEAR)
                .is_none_or(|(u, w)| u < w)
        };
        if key(p, q) { a } else { b }
    };
    let mut out = BTreeMap::new();
    let mut i = 0;
    while i < edges.len() {
        let r = &edges[i].radius;
        let mut j = i + 1;
        while j < edges.len() && r.constant().is_none() && edges[j].radius == *r {
            j += 1;
        }
        let run: Vec<EdgeId> = edges[i..j].iter().map(|f| f.edge).collect();
        i = j;
        if r.constant().is_some() {
            continue;
        }
        let from = run
            .iter()
            .find_map(|e| given.iter().find(|g| g.0 == *e).map(|g| g.1));
        let start = from.or_else(|| match run[..] {
            [a, b, ..] => {
                let (s, t) = ends(a);
                let shared = |v: VertexId| ends(b).0 == v || ends(b).1 == v;
                match (shared(s), shared(t)) {
                    (false, true) => Some(s),
                    (true, false) => Some(t),
                    _ => None,
                }
            }
            _ => None,
        });
        let mut reached: Vec<VertexId> = start.into_iter().collect();
        let mut left = run.clone();
        // an edge given one of its own ends starts there, and the walk goes on past it
        left.retain(|&e| {
            let (a, b) = ends(e);
            match given.iter().find(|g| g.0 == e && (g.1 == a || g.1 == b)) {
                Some(&(_, v)) => {
                    out.insert(e, v);
                    reached.extend([a, b]);
                    false
                }
                None => true,
            }
        });
        while let Some(k) = left.iter().position(|&e| {
            let (a, b) = ends(e);
            reached.contains(&a) || reached.contains(&b)
        }) {
            let e = left.remove(k);
            let (a, b) = ends(e);
            let (v, w) = if reached.contains(&a) { (a, b) } else { (b, a) };
            out.insert(e, v);
            reached.push(w);
        }
        for e in left {
            if from.is_some() {
                return Err(FilletError::BadRadius { edge: e });
            }
            out.insert(e, lowest(e));
        }
    }
    Ok(out)
}

/// Rounds `edges` of `body`, each with its own radius, in one operation.
/// `feature` is the history id the new faces, edges and vertices are named from.
pub fn fillet(body: &Body, edges: &[FilletEdge], feature: u64) -> R<Body> {
    fillet_with(body, edges, &FilletOptions::default(), feature)
}

/// `fillet` with laws started at given vertices and corners set back by given distances.
pub fn fillet_with(body: &Body, edges: &[FilletEdge], opts: &FilletOptions, feature: u64) -> R<Body> {
    if body.dead > 0 && (!opts.starts.is_empty() || !opts.setbacks.is_empty()) {
        // the options name this body's entities
        return Err(FilletError::NotAnEdge {
            edge: edges.first().map_or(EdgeId(0), |f| f.edge),
        });
    }
    let mut body = if body.dead > 0 {
        body.compact()
    } else {
        body.clone()
    };
    let mut seen = BTreeSet::new();
    for fe in edges {
        if !seen.insert(fe.edge) {
            return Err(FilletError::Duplicate { edge: fe.edge });
        }
        if fe.edge.index() >= body.edges.len() || !body.edge(fe.edge).alive {
            return Err(FilletError::NotAnEdge { edge: fe.edge });
        }
        if !fe.radius.valid() {
            return Err(FilletError::BadRadius { edge: fe.edge });
        }
    }
    // a closed edge with a varying radius is split in two: the halves' blends then meet
    // in sections at both vertices, like any chain
    let mut picked: Vec<FilletEdge> = Vec::with_capacity(edges.len());
    // a half with no point of the law on it still rounds, the law running across it
    let mut unpicked: Vec<(EdgeId, f64)> = Vec::new();
    let starts = law_starts(&body, edges, &opts.starts)?;
    for fe in edges {
        let e = fe.edge;
        let (v0, v1) = (body.edge(e).start, body.edge(e).end);
        if v0 != v1 || fe.radius.constant().is_some() {
            let mut fe = fe.clone();
            // internally a law runs from the edge's start
            if starts.get(&e).is_some_and(|&v| v != v0) {
                fe.radius = fe.radius.reversed();
            }
            picked.push(fe);
            continue;
        }
        let mid = body.edge(e).range.mid();
        let other = split_edge(&mut body, e, mid);
        let (first, second) = fe.radius.halves();
        for (edge, radius) in [(e, first), (other, second)] {
            match radius {
                Some(radius) => picked.push(FilletEdge { edge, radius }),
                None => unpicked.push((edge, fe.radius.max())),
            }
        }
    }
    let edges = &picked[..];
    merge_flush(&mut body, &seen);
    let all = tangent_chains(&body, edges, &unpicked);
    let seen: BTreeSet<EdgeId> = all.iter().map(|f| f.0).collect();
    merge_flush(&mut body, &seen);
    let root = Name::feature(feature);
    let mut all = all;
    let mut picks: BTreeMap<EdgeId, &Radius> = edges.iter().map(|f| (f.edge, &f.radius)).collect();
    // an edge whose ball moves onto a face beyond a smooth edge partway along is split there
    // and everything rolled again (rolling knew the old loops' coedges)
    for pass in 0.. {
        // chains whose radius varies roll as a whole; every other edge rolls by itself
        let chains: Vec<Chain> = chains(&body, &all)
            .into_iter()
            .filter(|c| {
                let r: Vec<&Radius> = c
                    .members
                    .iter()
                    .filter_map(|m| picks.get(&m.0).copied())
                    .collect();
                r.iter().any(|x| x.constant().is_none())
                    || r.iter().any(|x| (x.max() - r[0].max()).abs() > tol::LINEAR)
            })
            .collect();
        let in_var: BTreeMap<EdgeId, (usize, usize)> = chains
            .iter()
            .enumerate()
            .flat_map(|(c, ch)| ch.members.iter().enumerate().map(move |(i, m)| (m.0, (c, i))))
            .collect();
        let lone: Vec<(EdgeId, f64)> = all
            .iter()
            .filter(|f| !in_var.contains_key(&f.0))
            .map(|&(e, scale)| (e, picks.get(&e).map_or(scale, |r| r.max())))
            .collect();
        let jobs = lone.len() + chains.len();
        let rolled: Vec<R<Vec<Rolled>>> = par_map(jobs, |i| match lone.get(i) {
            Some(&(e, r)) => roll(&body, e, r, root).map(|x| vec![x]),
            None => roll_chain(&body, &chains[i - lone.len()], i - lone.len(), &picks, root),
        });
        let mut by_edge: BTreeMap<EdgeId, Rolled> = BTreeMap::new();
        for r in rolled {
            for x in r? {
                by_edge.insert(x.edge, x);
            }
        }
        let splits: Vec<(EdgeId, f64)> = by_edge
            .values()
            .filter_map(|r| r.split.map(|u| (r.edge, u)))
            .collect();
        if let (Some(&(edge, _)), 2) = (splits.first(), pass) {
            return Err(FilletError::Overflow { edge });
        }
        if splits.is_empty() {
            let rolled = all
                .iter()
                .map(|f| by_edge.remove(&f.0).expect("every edge rolled"))
                .collect();
            let mut b = Builder::new(body, rolled, root);
            b.setbacks = opts.setbacks.clone();
            return b.build();
        }
        for (e, u) in splits {
            let other = split_edge(&mut body, e, u);
            let at = all.iter().position(|f| f.0 == e).expect("rolled");
            all.insert(at + 1, (other, all[at].1));
            if let Some(&r) = picks.get(&e) {
                picks.insert(other, r);
            }
        }
    }
    unreachable!("the passes return")
}

/// Runs `f` over 0..n on the machine's threads, results in order.
fn par_map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let threads = std::thread::available_parallelism().map_or(1, |t| t.get()).min(n);
    if threads <= 1 {
        return (0..n).map(f).collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let parts: Vec<Vec<(usize, T)>> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut got = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if i >= n {
                            break got;
                        }
                        got.push((i, f(i)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().expect("fillet worker"))
            .collect()
    });
    let mut out: Vec<Option<T>> = (0..n).map(|_| None).collect();
    for (i, t) in parts.into_iter().flatten() {
        out[i] = Some(t);
    }
    out.into_iter().map(|t| t.expect("every edge done")).collect()
}

/// The edges with every tangent continuation of each added, each with the largest
/// radius of the pick that reached it: a chain rounds as a whole, as a ball rolling
/// along it would.
fn tangent_chains(body: &Body, edges: &[FilletEdge], unpicked: &[(EdgeId, f64)]) -> Vec<(EdgeId, f64)> {
    let at = edges_at(body);
    let mut out: Vec<(EdgeId, f64)> = edges.iter().map(|f| (f.edge, f.radius.max())).collect();
    out.extend_from_slice(unpicked);
    let mut have: BTreeSet<EdgeId> = out.iter().map(|f| f.0).collect();
    let mut i = 0;
    while i < out.len() {
        let (edge, radius) = out[i];
        i += 1;
        let x = body.edge(edge);
        for v in [x.start, x.end] {
            let mine = out_of(body, edge, v);
            let next: Vec<EdgeId> = at[&v]
                .iter()
                .copied()
                .filter(|&o| o != edge && !have.contains(&o) && body.edge(o).coedges.len() == 2)
                .filter(|&o| {
                    let f = &body.edge(o).coedges;
                    body.coedge_face(f[0]) != body.coedge_face(f[1])
                })
                .filter(|&o| continues(&mine, &out_of(body, o, v), radius))
                .collect();
            if let [o] = next[..]
                && !is_smooth(body, o, radius)
            {
                have.insert(o);
                out.push((o, radius));
            }
        }
    }
    out
}

/// The edges at each vertex.
fn edges_at(body: &Body) -> BTreeMap<VertexId, Vec<EdgeId>> {
    let mut at: BTreeMap<VertexId, Vec<EdgeId>> = BTreeMap::new();
    for e in body.edge_ids() {
        let x = body.edge(e);
        at.entry(x.start).or_default().push(e);
        if x.end != x.start {
            at.entry(x.end).or_default().push(e);
        }
    }
    at
}

/// Unit tangents of e at its end v, pointing out of v.
fn out_of(body: &Body, e: EdgeId, v: VertexId) -> Vec<Vec3> {
    let x = body.edge(e);
    let mut t = Vec::new();
    if x.start == v {
        t.push(x.curve.deriv(x.range.lo).d1);
    }
    if x.end == v {
        t.push(-x.curve.deriv(x.range.hi).d1);
    }
    t.into_iter()
        .filter_map(|d| d.normalize())
        .map(|u| u.get())
        .collect()
}

/// Whether two edges leave a vertex the opposite way, within LINEAR over the radius.
fn continues(a: &[Vec3], b: &[Vec3], radius: f64) -> bool {
    b.iter()
        .any(|t| a.iter().any(|m| (*m + *t).length() * radius <= tol::LINEAR))
}

/// The filleted edges as tangent chains: two edges are linked at a vertex where they are
/// the only filleted ends there, continue each other, and every other edge is smooth.
fn chains(body: &Body, all: &[(EdgeId, f64)]) -> Vec<Chain> {
    let scale: BTreeMap<EdgeId, f64> = all.iter().copied().collect();
    let at = edges_at(body);
    // (edge, at its end) -> the end it links to
    let mut link: BTreeMap<(EdgeId, bool), (EdgeId, bool)> = BTreeMap::new();
    let mut verts = BTreeSet::new();
    for &(e, _) in all {
        verts.insert(body.edge(e).start);
        verts.insert(body.edge(e).end);
    }
    for v in verts {
        let mut ends = Vec::new();
        for &e in &at[&v] {
            if scale.contains_key(&e) {
                let x = body.edge(e);
                if x.start == v {
                    ends.push((e, false));
                }
                if x.end == v {
                    ends.push((e, true));
                }
            }
        }
        let [a, b] = ends[..] else { continue };
        let r = scale[&a.0].max(scale[&b.0]);
        let tan = |(e, end): (EdgeId, bool)| {
            let x = body.edge(e);
            let d = if end {
                -x.curve.deriv(x.range.hi).d1
            } else {
                x.curve.deriv(x.range.lo).d1
            };
            // a traced curve's end tangent is only as good as its trace: the surfaces' normals
            // there give the exact one
            if let (Curve::Intersection(_), [c0, c1]) = (&x.curve, &x.coedges[..]) {
                let p = body.vertex(v).point;
                let n = |c: CoedgeId| body.face(body.coedge_face(c)).surface.gradient(p);
                if let Some(u) = n(*c0).cross(n(*c1)).normalize() {
                    let u = u.get();
                    return Some(if u.dot(d) < 0.0 { -u } else { u });
                }
            }
            d.normalize().map(|u| u.get())
        };
        let (Some(ta), Some(tb)) = (tan(a), tan(b)) else {
            continue;
        };
        let others_smooth = at[&v].iter().filter(|&&o| o != a.0 && o != b.0).all(|&o| {
            let y = body.edge(o);
            y.coedges.len() == 2
                && (body.coedge_face(y.coedges[0]) == body.coedge_face(y.coedges[1]) || is_smooth(body, o, r))
        });
        if continues(&[ta], &[tb], r) && others_smooth {
            link.insert(a, b);
            link.insert(b, a);
        }
    }
    let mut done = BTreeSet::new();
    let mut out = Vec::new();
    // open chains from their free ends first, then what is left is closed
    for pass in 0..2 {
        for &(e, _) in all {
            if done.contains(&e) {
                continue;
            }
            let fwd = match (link.contains_key(&(e, false)), link.contains_key(&(e, true))) {
                (false, _) => true,
                (true, false) => false,
                (true, true) if pass == 1 => true,
                _ => continue,
            };
            let mut members = vec![(e, fwd)];
            done.insert(e);
            let mut closed = false;
            let (mut cur, mut f) = (e, fwd);
            while let Some(&(n, n_end)) = link.get(&(cur, f)) {
                if (n, n_end) == (e, !fwd) {
                    closed = true;
                    break;
                }
                if !done.insert(n) {
                    break;
                }
                (cur, f) = (n, !n_end);
                members.push((cur, f));
            }
            if closed {
                members = canonical_loop(body, members);
            }
            out.push(Chain { members, closed });
        }
    }
    out
}

/// Whether the faces either side of `e` meet tangentially at its middle (to within
/// LINEAR over the radius).
fn is_smooth(body: &Body, e: EdgeId, radius: f64) -> bool {
    let x = body.edge(e);
    let p = x.curve.eval(x.range.mid());
    let n: Vec<_> = x
        .coedges
        .iter()
        .filter_map(|&c| {
            let f = body.coedge_face(c);
            body.face_normal(f, body.face(f).surface.project(p))
        })
        .collect();
    n.len() == 2 && n[0].cross(n[1]).length() * radius <= tol::LINEAR && n[0].dot(n[1]) > 0.0
}

/// A closed chain started and walked the same way whatever the body's numbering: from
/// the member whose middle is least (by x, then y, then z), the way that runs up there.
fn canonical_loop(body: &Body, members: Vec<(EdgeId, bool)>) -> Vec<(EdgeId, bool)> {
    let key = |&(e, fwd): &(EdgeId, bool)| {
        let x = body.edge(e);
        let d = x.curve.deriv(x.range.mid());
        let t = if fwd { d.d1 } else { -d.d1 };
        ([d.p.x, d.p.y, d.p.z], [t.x, t.y, t.z])
    };
    let less = |a: [f64; 3], b: [f64; 3]| a.iter().zip(&b).find(|p| p.0 != p.1).is_some_and(|p| p.0 < p.1);
    let i = (0..members.len())
        .reduce(|a, b| {
            if less(key(&members[b]).0, key(&members[a]).0) {
                b
            } else {
                a
            }
        })
        .unwrap_or(0);
    let mut out: Vec<(EdgeId, bool)> = members[i..].iter().chain(&members[..i]).copied().collect();
    if less(key(&out[0]).1, [0.0; 3]) {
        // walked back: the same loop from the same member, the other way round
        out[1..].reverse();
        for m in &mut out {
            m.1 = !m.1;
        }
    }
    out
}

/// Splits edge `e` at parameter `mid` (a closed edge at its middle): `e` keeps the first part,
/// the returned edge is the second. Every loop through it walks both.
fn split_edge(body: &mut Body, e: EdgeId, mid: f64) -> EdgeId {
    let x = body.edge(e).clone();
    let m = body.add_vertex(x.curve.eval(mid), x.name.with_str("half"));
    let other = body.add_edge(
        x.curve.clone(),
        Interval::new(mid, x.range.hi),
        m,
        x.end,
        x.name.with_str("half"),
    );
    let edge = &mut body.edges[e.index()];
    edge.range.hi = mid;
    edge.end = m;
    edge.bbox.clear();
    let mut loops: Vec<(FaceId, forge_topo::LoopId)> = x
        .coedges
        .iter()
        .map(|&c| (body.coedge_face(c), body.coedge(c).lp))
        .collect();
    loops.dedup();
    for (f, l) in loops {
        let mut uses: Vec<(EdgeId, bool)> = Vec::new();
        for d in body.lp(l).coedges() {
            let co = body.coedge(d);
            match (co.edge == e, co.forward) {
                (true, true) => uses.extend([(e, true), (other, true)]),
                (true, false) => uses.extend([(other, false), (e, false)]),
                _ => uses.push((co.edge, co.forward)),
            }
        }
        body.remove_loop(f, l);
        body.add_loop(f, &uses);
    }
    other
}

/// Faces on one surface that meet along an edge at a vertex of a filleted edge
/// become one face: a boolean leaves such seams, and a blend must run out
/// across the surface as a whole.
fn merge_flush(body: &mut Body, filleted: &BTreeSet<EdgeId>) {
    let mut verts = BTreeSet::new();
    for &e in filleted {
        verts.insert(body.edge(e).start);
        verts.insert(body.edge(e).end);
    }
    let candidates: Vec<EdgeId> = body
        .edge_ids()
        .filter(|&e| {
            let x = body.edge(e);
            !filleted.contains(&e) && (verts.contains(&x.start) || verts.contains(&x.end))
        })
        .collect();
    for e in candidates {
        let edge = body.edge(e);
        if !edge.alive || edge.coedges.len() != 2 {
            continue;
        }
        let (a, b) = (
            body.coedge_face(edge.coedges[0]),
            body.coedge_face(edge.coedges[1]),
        );
        if a == b || !same_surface(body, a, b) {
            continue;
        }
        merge_faces(body, e);
    }
}

/// Whether two faces lie on one plane with the same outward side. (Curved
/// faces on one surface are left split: their seams keep a periodic face's
/// boundary drawable.)
fn same_surface(body: &Body, a: FaceId, b: FaceId) -> bool {
    let (fa, fb) = (body.face(a), body.face(b));
    match (&fa.surface, &fb.surface) {
        (Surface::Plane(p), Surface::Plane(q)) => {
            let sign = if fa.same_sense == fb.same_sense { 1.0 } else { -1.0 };
            p.frame.z().cross(q.frame.z()).length() <= tol::ANGULAR
                && p.frame.z().dot(q.frame.z()) * sign > 0.0
                && p.signed_distance(q.frame.origin).abs() <= tol::LINEAR
        }
        _ => false,
    }
}

/// Joins the two faces either side of `e` into the first: their loops through
/// `e` become one, the second face's other loops move over, and edges the
/// joined loop walks there and straight back are dropped.
fn merge_faces(body: &mut Body, e: EdgeId) {
    let (ca, cb) = (body.edge(e).coedges[0], body.edge(e).coedges[1]);
    let (a, b) = (body.coedge_face(ca), body.coedge_face(cb));
    let (la, lb) = (body.coedge(ca).lp, body.coedge(cb).lp);
    let uses = |body: &Body, l: forge_topo::LoopId| -> Vec<(EdgeId, bool)> {
        body.lp(l)
            .coedges()
            .map(|c| (body.coedge(c).edge, body.coedge(c).forward))
            .collect()
    };
    let (ua, ub) = (uses(body, la), uses(body, lb));
    let ia = body.lp(la).coedges().position(|c| c == ca).expect("in its loop");
    let ib = body.lp(lb).coedges().position(|c| c == cb).expect("in its loop");
    let mut joined: Vec<(EdgeId, bool)> = ua[..ia].to_vec();
    joined.extend_from_slice(&ub[ib + 1..]);
    joined.extend_from_slice(&ub[..ib]);
    joined.extend_from_slice(&ua[ia + 1..]);
    // there-and-back pairs (the two faces shared more than one edge in a row)
    let mut changed = true;
    while changed && joined.len() > 1 {
        changed = false;
        let n = joined.len();
        for i in 0..n {
            let j = (i + 1) % n;
            if joined[i].0 == joined[j].0 && joined[i].1 != joined[j].1 {
                let (hi, lo) = (i.max(j), i.min(j));
                joined.remove(hi);
                joined.remove(lo);
                changed = true;
                break;
            }
        }
    }
    // an edge walked both ways further apart (the other seam of a revolve's two halves) cuts
    // the loop in two: the outer and the hole it ran into
    let mut joined = vec![joined];
    while let Some((k, i, j)) = joined.iter().enumerate().find_map(|(k, l)| {
        (0..l.len()).find_map(|i| (i + 1..l.len()).find(|&j| l[j].0 == l[i].0).map(|j| (k, i, j)))
    }) {
        let (mut hole, mut rest) = (
            joined[k][i + 1..j].to_vec(),
            [&joined[k][..i], &joined[k][j + 1..]].concat(),
        );
        // the outer one turns counter-clockwise about the outward normal, and stays first
        let n = body
            .face(a)
            .surface
            .gradient(body.vertex(body.edge(e).start).point);
        let n = if body.face(a).same_sense { n } else { -n };
        if turn(body, &hole).dot(n) > 0.0 {
            std::mem::swap(&mut hole, &mut rest);
        }
        joined[k] = rest;
        joined.push(hole);
    }
    let others: Vec<Vec<(EdgeId, bool)>> = body
        .face(b)
        .loops
        .iter()
        .filter(|&&l| l != lb)
        .map(|&l| uses(body, l))
        .collect();
    body.remove_loop(a, la);
    let lbs = body.face(b).loops.clone();
    for &l in &lbs {
        body.remove_loop(b, l);
    }
    body.kill_face(b);
    for l in joined.into_iter().filter(|l| !l.is_empty()) {
        body.add_loop(a, &l);
    }
    for o in others {
        body.add_loop(a, &o);
    }
    let face = &mut body.faces[a.index()];
    face.bbox.clear();
    face.query.clear();
}

/// The area vector of a loop: along the normal it turns counter-clockwise about.
fn turn(body: &Body, l: &[(EdgeId, bool)]) -> Vec3 {
    let mut pts: Vec<Point3> = Vec::new();
    for &(e, fw) in l {
        let mut p = body.edge_points(e, 8);
        if !fw {
            p.reverse();
        }
        pts.extend(p);
    }
    let o = pts.first().copied().unwrap_or(Point3::ORIGIN);
    pts.windows(2)
        .fold(Vec3::ZERO, |s, w| s + (w[0] - o).cross(w[1] - o))
}

/// An edge's faces at one of its points, and where a ball between them sits.
struct Local {
    faces: [FaceId; 2],
    coedges: [CoedgeId; 2],
    forward: [bool; 2],
    convex: bool,
    /// The side of each face's surface (along its natural normal) the ball centre is on.
    sign: [f64; 2],
    p: Point3,
    tangent: UnitVec3,
    /// From p towards the centre of a ball of radius 1.
    toward: Vec3,
}

/// The faces either side of edge `e` at its parameter t, for a ball of radius about r.
fn local(body: &Body, e: EdgeId, r: f64, t: f64) -> R<Local> {
    if e.index() >= body.edges.len() || !body.edge(e).alive || body.edge(e).coedges.len() != 2 {
        return Err(FilletError::NotAnEdge { edge: e });
    }
    if !(r > tol::MIN_LENGTH && r.is_finite()) {
        return Err(FilletError::BadRadius { edge: e });
    }
    let edge = body.edge(e);
    let (c0, c1) = (edge.coedges[0], edge.coedges[1]);
    let (f0, f1) = (body.coedge_face(c0), body.coedge_face(c1));
    if f0 == f1 {
        return Err(FilletError::Smooth { edge: e });
    }
    let d = edge.curve.deriv(t);
    let tangent = d.d1.normalize().ok_or(FilletError::NotAnEdge { edge: e })?;
    let normal = |f: FaceId| {
        let s = &body.face(f).surface;
        body.face_normal(f, s.project(d.p))
    };
    let (n0, n1) = match (normal(f0), normal(f1)) {
        (Some(a), Some(b)) => (a, b),
        _ => return Err(FilletError::NoBlend { edge: e }),
    };
    // the faces turn by less than LINEAR over one radius: tangent
    if n0.cross(n1).length() * r <= tol::LINEAR && n0.dot(n1) > 0.0 {
        return Err(FilletError::Smooth { edge: e });
    }
    // face 1 runs away from the edge along n1 x (its walking direction); convex when that dips below face 0
    let walk1 = if body.coedge(c1).forward {
        tangent.get()
    } else {
        -tangent.get()
    };
    let into1 = n1.cross(walk1);
    let convex = n0.dot(into1) < 0.0;
    let side = if convex { -1.0 } else { 1.0 };
    let sense = |f: FaceId| if body.face(f).same_sense { 1.0 } else { -1.0 };
    let bis = (n0.get() + n1.get()).normalize().map_or(Vec3::ZERO, |u| u.get());
    let half = ((1.0 + n0.dot(n1)) * 0.5).max(1e-6).sqrt();
    Ok(Local {
        faces: [f0, f1],
        coedges: [c0, c1],
        forward: [body.coedge(c0).forward, body.coedge(c1).forward],
        convex,
        sign: [side * sense(f0), side * sense(f1)],
        p: d.p,
        tangent,
        toward: bis * (side / half),
    })
}

/// The blend for one edge of constant radius r, from the faces on either side at its middle.
fn roll(body: &Body, e: EdgeId, r: f64, root: Name) -> R<Rolled> {
    if e.index() >= body.edges.len() {
        return Err(FilletError::NotAnEdge { edge: e });
    }
    let l = local(body, e, r, body.edge(e).range.mid())?;
    let near = l.p + l.toward * r;
    let region = body.edge_box(e).grow(4.0 * r + tol::LINEAR);
    let blend = blend_of(body, l.faces, [l.sign[0] * r, l.sign[1] * r], l.p, near, region)
        .ok_or(FilletError::NoBlend { edge: e })?;
    let tm = spine_closest(&blend.spine, l.p);
    if blend.spine.eval(tm).distance(near) > r {
        return Err(FilletError::NoBlend { edge: e });
    }
    let springs = [
        blend.spring(true).ok_or(FilletError::NoBlend { edge: e })?,
        blend.spring(false).ok_or(FilletError::NoBlend { edge: e })?,
    ];
    let (blend, springs, over) = switch_support(body, &l, r, blend, springs, region);
    let split = switch_split(body, e, &l, &blend, &springs, &over);
    let tm = spine_closest(&blend.spine, l.p);
    let aligned = blend.spine.deriv(tm).d1.dot(l.tangent.get()) > 0.0;
    Ok(Rolled {
        edge: e,
        convex: l.convex,
        blend,
        springs,
        aligned,
        faces: l.faces,
        coedges: l.coedges,
        forward: l.forward,
        name: root.with_str("fillet").with_name(body.edge(e).name),
        var: None,
        over,
        split,
    })
}

/// The blend between faces `f` (offset by `d`), its spine passing nearest `near`: built with the
/// faces in an order the geometry fixes (their normals at `p`), so its spine's direction and its
/// surface's frame do not depend on which way round the body stores the edge; `a` stays `f[0]`.
fn blend_of(
    body: &Body,
    f: [FaceId; 2],
    d: [f64; 2],
    p: Point3,
    near: Point3,
    region: Box3,
) -> Option<Blend> {
    let n = |f: FaceId| {
        body.face_normal(f, body.face(f).surface.project(p))
            .map_or([0.0; 3], |n| [n.get().x, n.get().y, n.get().z])
    };
    let (n0, n1) = (n(f[0]), n(f[1]));
    let swap = (0..3)
        .find(|&k| (n0[k] - n1[k]).abs() > 1e-9)
        .is_some_and(|k| n0[k] > n1[k]);
    let s = |i: usize| &body.face(f[i]).surface;
    if !swap {
        return Blend::new(s(0), s(1), d[0], d[1], near, region);
    }
    let mut b = Blend::new(s(1), s(0), d[1], d[0], near, region)?;
    std::mem::swap(&mut b.a, &mut b.b);
    std::mem::swap(&mut b.da, &mut b.db);
    Some(b)
}

/// Where a spring of the blend between the edge's faces lies off its face, across a smooth
/// edge of it, the ball rests on the face beyond: the blend rolled on that face instead.
fn switch_support(
    body: &Body,
    l: &Local,
    r: f64,
    blend: Blend,
    springs: [Curve; 2],
    region: Box3,
) -> (Blend, [Curve; 2], Over) {
    let mid = |b: &Blend, c: &Curve| c.eval(spine_closest(&b.spine, l.p));
    let sense = |f: FaceId| if body.face(f).same_sense { 1.0 } else { -1.0 };
    for s in 0..2 {
        let b = l.faces[s];
        if on_face(body, b, mid(&blend, &springs[s])) {
            continue;
        }
        let around: Vec<CoedgeId> = body
            .face(b)
            .loops
            .iter()
            .flat_map(|&lp| body.lp(lp).coedges())
            .collect();
        for c in around {
            let g = body.coedge(c).edge;
            let Some(cf) = body.partner(c).map(|p| body.coedge_face(p)) else {
                continue;
            };
            if cf == b || cf == l.faces[1 - s] {
                continue;
            }
            // tangent along g: the faces' normals agree at its middle
            let x = body.edge(g);
            let q = x.curve.eval(x.range.mid());
            let n = |f: FaceId| body.face_normal(f, body.face(f).surface.project(q));
            let (Some(nb), Some(nc)) = (n(b), n(cf)) else {
                continue;
            };
            if nb.dot(nc.get()) < 1.0 - 1e-9 {
                continue;
            }
            let mut faces = l.faces;
            faces[s] = cf;
            let mut sign = l.sign;
            sign[s] *= sense(b) * sense(cf);
            let near = l.p + l.toward * r;
            let Some(nb) = blend_of(body, faces, [sign[0] * r, sign[1] * r], l.p, near, region) else {
                continue;
            };
            let (Some(s0), Some(s1)) = (nb.spring(true), nb.spring(false)) else {
                continue;
            };
            let ns = [s0, s1];
            if (0..2).all(|k| on_face(body, faces[k], mid(&nb, &ns[k]))) {
                let mut over = [None, None];
                over[s] = Some((cf, g));
                return (nb, ns, over);
            }
        }
    }
    (blend, springs, [None, None])
}

/// Where a spring crosses a smooth edge of the face it lies on (that face and the one beyond
/// tangent along it) between the edge's ends, the edge's parameter at that section: the ball
/// rests on one face before it and on the other after.
fn switch_split(
    body: &Body,
    e: EdgeId,
    l: &Local,
    blend: &Blend,
    springs: &[Curve; 2],
    over: &Over,
) -> Option<f64> {
    let x = body.edge(e);
    let spine = &blend.spine;
    let t0 = spine_closest(spine, x.curve.eval(x.range.lo));
    let t1 = t0
        + along(
            spine_closest(spine, x.curve.eval(x.range.hi)) - t0,
            spine.domain().period,
        );
    for s in 0..2 {
        let face = over[s].map_or(l.faces[s], |o| o.0);
        let fx = body.face(face);
        // only an edge passing by the spring can cross it: those round the face's loop from
        // the edge's own coedge, up to the first to leave the spring's reach
        let around: Vec<EdgeId> = match over[s] {
            Some((_, g)) => vec![g],
            None => {
                let reach = body.edge_box(e).grow(2.0 * blend.radius);
                let lp = body.lp(body.coedge(l.coedges[s]).lp);
                let n = lp.coedges().len();
                let k = lp.coedges().position(|c| c == l.coedges[s]).unwrap_or(0);
                let mut out = Vec::new();
                for (step, far) in [(1, true), (n - 1, false)] {
                    for i in 1..n {
                        let c = lp.coedge((k + step * i) % n);
                        let ends = (body.coedge_start(c), body.coedge_end(c));
                        out.push(body.coedge(c).edge);
                        if !reach.contains(body.vertex(if far { ends.1 } else { ends.0 }).point) {
                            break;
                        }
                    }
                }
                out
            }
        };
        let ends = [springs[s].eval(t0), springs[s].eval(t1)];
        for g in around {
            let gx = body.edge(g);
            // which side of g a spring point is on, across the face's tangent plane
            let side = |p: Point3| {
                let u = gx.curve.closest(p, gx.range);
                let d = gx.curve.deriv(u);
                let n = body.face_normal(face, fx.surface.project(d.p))?;
                Some((p - d.p).dot(n.get().cross(d.d1)))
            };
            // a crossing at an end's section is where a piece split there already ends
            let (Some(h0), Some(h1)) = (side(ends[0]), side(ends[1])) else {
                continue;
            };
            if h0 * h1 >= 0.0 || h0.abs().min(h1.abs()) <= 1e3 * tol::LINEAR {
                continue;
            }
            // tangent along g: its faces' normals agree at its middle
            let [c0, c1] = gx.coedges[..] else { continue };
            let (f0, f1) = (body.coedge_face(c0), body.coedge_face(c1));
            let q = gx.curve.eval(gx.range.mid());
            let n = |f: FaceId| body.face_normal(f, body.face(f).surface.project(q));
            if f0 == f1
                || !n(f0)
                    .zip(n(f1))
                    .is_some_and(|(a, b)| a.dot(b.get()) >= 1.0 - 1e-9)
            {
                continue;
            }
            let side = |t: f64| side(springs[s].eval(t));
            // regula falsi (Illinois) on the side
            let ((mut a, mut ha), (mut b, mut hb)) = ((t0, h0), (t1, h1));
            let mut t = a;
            for _ in 0..60 {
                t = (a * hb - b * ha) / (hb - ha);
                let Some(h) = side(t) else { break };
                if h.abs() <= 1e-3 * tol::LINEAR {
                    break;
                }
                if h * hb < 0.0 {
                    (a, ha) = (b, hb);
                } else {
                    ha *= 0.5;
                }
                (b, hb) = (t, h);
            }
            let p = springs[s].eval(t);
            if gx.curve.eval(gx.curve.closest(p, gx.range)).distance(p) > tol::LINEAR {
                continue;
            }
            // the edge's point in the section's plane
            let c = spine.deriv(t);
            let mut u = x.curve.closest(c.p, x.range);
            for _ in 0..20 {
                let d = x.curve.deriv(u);
                let step = (d.p - c.p).dot(c.d1) / d.d1.dot(c.d1);
                u -= step;
                if step.abs() <= 1e-15 * (1.0 + u.abs()) {
                    break;
                }
            }
            if u > x.range.lo && u < x.range.hi {
                return Some(u);
            }
        }
    }
    None
}

/// Whether point `p` (on or near face `f`'s surface) lies within the face.
fn on_face(body: &Body, f: FaceId, p: Point3) -> bool {
    let loops = forge_topo::uv::face_uv_loops(body, f);
    let surface = &body.face(f).surface;
    let d = surface.domain();
    let uv = surface.project(p);
    if surface.eval(uv).distance(p) > tol::LINEAR {
        return false;
    }
    let shifts = |p: Option<f64>| match p {
        Some(p) => vec![0.0, -p, p],
        None => vec![0.0],
    };
    shifts(d.u.period).iter().any(|&du| {
        shifts(d.v.period)
            .iter()
            .any(|&dv| forge_topo::uv::point_in_loops(forge_math::Point2::new(uv.x + du, uv.y + dv), &loops))
    })
}

/// The blends of a chain whose radius varies: one guide and one law along the whole
/// chain, one canal surface per edge on the edge's stretch of it (the end ones carried
/// on past the chain's ends, so they can run out).
fn roll_chain(
    body: &Body,
    chain: &Chain,
    id: usize,
    picks: &BTreeMap<EdgeId, &Radius>,
    root: Name,
) -> R<Vec<Rolled>> {
    let first = chain.members[0].0;
    let picked: Vec<&Radius> = chain
        .members
        .iter()
        .filter_map(|m| picks.get(&m.0).copied())
        .collect();
    let rmax = picked.iter().fold(0.0f64, |m, r| m.max(r.max()));
    let edges: Vec<(Curve, Interval, bool)> = chain
        .members
        .iter()
        .map(|&(e, fwd)| (body.edge(e).curve.clone(), body.edge(e).range, fwd))
        .collect();
    // knots on lines and circles only smooth the joints: two steps, or one per quarter turn, so the
    // guide turns gently there (closer knots bend it harder and fold large balls)
    let exact = edges
        .iter()
        .all(|e| matches!(e.0, Curve::Line(_) | Curve::Circle(_)));
    let quarters = |e: &(Curve, Interval, bool)| match e.0 {
        Curve::Circle(_) => (e.1.width() / dmath::FRAC_PI_2 - 1e-9).ceil().max(1.0) as usize,
        _ => 1,
    };
    let per = if exact {
        edges.iter().map(quarters).max().unwrap_or(1).max(2)
    } else {
        8
    };
    let mut err = FilletError::NoBlend { edge: first };
    // run-outs reach past the chain's ends far enough for the ball to cross oblique faces
    // there; shorter where the law, carried on, would leave sensible radii or the blend fails
    let full = chain
        .members
        .iter()
        .fold(Box3::EMPTY, |b, m| b.hull(&body.edge_box(m.0)))
        .diagonal()
        + 4.0 * rmax;
    let reaches = if chain.closed {
        vec![0.0]
    } else {
        vec![full, 0.25 * full, rmax]
    };
    for reach in reaches {
        let (guide, ranges, whole) =
            Guide::through_edges(&edges, per, reach).ok_or(FilletError::NoBlend { edge: first })?;
        let n = ranges.len();
        let inner = Interval::new(ranges[0].lo, ranges[n - 1].hi);
        let law = chain_law(chain, &ranges, whole, picks).ok_or(FilletError::BadRadius { edge: first })?;
        let (lo, hi) = law.bounds(inner.lo, inner.hi);
        if lo <= tol::MIN_LENGTH {
            return Err(FilletError::BadRadius { edge: first });
        }
        let (wlo, whi) = law.bounds(whole.lo, whole.hi);
        if (wlo < 0.5 * lo || whi > 2.0 * hi) && reach > rmax {
            continue;
        }
        let law = Arc::new(law);
        let mut out = Vec::with_capacity(n);
        for (i, &(e, fwd)) in chain.members.iter().enumerate() {
            let own = ranges[i];
            let range = Interval::new(
                if i == 0 { whole.lo } else { own.lo },
                if i == n - 1 { whole.hi } else { own.hi },
            );
            let edge = body.edge(e);
            let sm = own.mid();
            let g = guide.point_tan(sm).0;
            let l = local(body, e, rmax, edge.curve.closest(g, edge.range))?;
            let (s0, s1) = (&body.face(l.faces[0]).surface, &body.face(l.faces[1]).surface);
            let vb = match VarBlend::new(s0, l.sign[0], s1, l.sign[1], guide.clone(), law.clone(), range) {
                Ok(vb) => vb,
                Err(x) => {
                    let at = match x {
                        VarBlendError::Tangent { s }
                        | VarBlendError::RadiusTooSteep { s }
                        | VarBlendError::Contacts { s }
                        | VarBlendError::Lost { s }
                        | VarBlendError::Folds { s } => Some(s),
                        _ => None,
                    };
                    err = match x {
                        VarBlendError::RadiusTooSteep { .. } | VarBlendError::Folds { .. } => {
                            FilletError::BadRadius { edge: e }
                        }
                        _ => FilletError::NoBlend { edge: e },
                    };
                    // trouble on the edges themselves is not the run-out's
                    match at {
                        Some(s) if s >= inner.lo && s <= inner.hi => return Err(err),
                        _ => break,
                    }
                }
            };
            let rm = law.eval(sm);
            if vb.centre(sm).distance(l.p + l.toward * rm) > rm {
                return Err(FilletError::NoBlend { edge: e });
            }
            let blend = Blend::variable(s0, s1, vb);
            let springs = [
                blend.spring(true).ok_or(FilletError::NoBlend { edge: e })?,
                blend.spring(false).ok_or(FilletError::NoBlend { edge: e })?,
            ];
            out.push(Rolled {
                edge: e,
                convex: l.convex,
                blend,
                springs,
                aligned: fwd,
                faces: l.faces,
                coedges: l.coedges,
                forward: l.forward,
                name: root.with_str("fillet").with_name(edge.name),
                var: Some(VarEdge {
                    chain: id,
                    index: i,
                    len: n,
                    closed: chain.closed,
                    own,
                }),
                over: [None, None],
                split: None,
            });
        }
        if out.len() == n {
            return Ok(out);
        }
    }
    Err(err)
}

/// The chain's radius law. A constant pick holds its radius exactly along its own edge.
/// Between constant edges the radius changes over the variable and unpicked edges, by a
/// smooth spline through their points held flat where it meets a constant edge; two
/// constant edges of different radii meeting directly change over a short zone centred
/// on their joint. Points at one place (a joint between laws) take their mean. Without
/// constant picks a closed chain's law is periodic.
fn chain_law(
    chain: &Chain,
    ranges: &[Interval],
    whole: Interval,
    picks: &BTreeMap<EdgeId, &Radius>,
) -> Option<RadiusLaw> {
    let n = chain.members.len();
    let period = whole.width();
    let eps = 1e-9 * period.max(1.0);
    // each member's own points, in s, and its constant radius if it has one
    let mut own: Vec<(Points, Option<f64>)> = Vec::with_capacity(n);
    for (i, &(e, fwd)) in chain.members.iter().enumerate() {
        let r = ranges[i];
        let at = |f: f64| {
            if fwd {
                r.lo + f * r.width()
            } else {
                r.hi - f * r.width()
            }
        };
        let Some(pick) = picks.get(&e) else {
            own.push((Vec::new(), None));
            continue;
        };
        let mut pts: Vec<(f64, f64)> = pick.points().iter().map(|&(f, x)| (at(f), x)).collect();
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        own.push((pts, pick.constant()));
    }
    let Some(first) = (0..n).find(|&i| own[i].1.is_some()) else {
        return free_law(chain, ranges, whole, own.into_iter().flat_map(|o| o.0).collect());
    };
    // walk from the first constant edge's middle (closed) or the chain's start, s unrolled
    let order: Vec<usize> = if chain.closed {
        (0..=n).map(|k| (first + k) % n).collect()
    } else {
        (0..n).collect()
    };
    let shift = |k: usize, i: usize| {
        if chain.closed && (i < first || k == n) {
            period
        } else {
            0.0
        }
    };
    let start = if chain.closed {
        ranges[first].mid()
    } else {
        ranges[0].lo
    };
    let end = if chain.closed {
        start + period
    } else {
        ranges[n - 1].hi
    };
    let mut parts: Vec<(f64, RadiusLaw)> = Vec::new();
    // the run of points since the last constant edge, from its end (and radius) or the start
    let mut run: Vec<(f64, f64)> = Vec::new();
    let mut left: Option<(f64, f64)> = None;
    let mut prev: Option<(f64, Interval)> = None;
    for (k, &i) in order.iter().enumerate() {
        let d = shift(k, i);
        let sp = Interval::new(ranges[i].lo + d, ranges[i].hi + d);
        let Some(c) = own[i].1 else {
            run.extend(own[i].0.iter().map(|&(s, r)| (s + d, r)));
            prev = None;
            continue;
        };
        let a = sp.lo;
        let from = match prev {
            Some((r0, p)) if (r0 - c).abs() > tol::LINEAR => {
                // two constants meet: a short zone about their joint
                let w = (0.5 * p.width().min(sp.width())).min(4.0 * c.max(r0));
                parts.push((a - w, RadiusLaw::monotone(&[(a - w, r0), (a + w, c)])?));
                Some(a + w)
            }
            Some(_) => None,
            None => {
                let from = left.map_or(start, |l| l.0);
                if a > from + eps {
                    let mut pts: Vec<(f64, f64)> = left.into_iter().collect();
                    pts.extend(run.iter().filter(|p| p.0 > from + eps && p.0 < a - eps));
                    pts.push((a, c));
                    parts.push((from, run_law(&pts, left.is_some(), true)?));
                }
                Some(a)
            }
        };
        if let Some(f) = from {
            let f = if chain.closed && k == 0 { start } else { f };
            parts.push((f, RadiusLaw::constant(c)));
        }
        if chain.closed && k == n {
            break;
        }
        run.clear();
        left = Some((sp.hi, c));
        prev = Some((c, sp));
    }
    if !chain.closed && prev.is_none() {
        // a run to the chain's end, free there
        let from = left.map_or(start, |l| l.0);
        let mut pts: Vec<(f64, f64)> = left.into_iter().collect();
        pts.extend(run.iter().filter(|p| p.0 > from + eps));
        parts.push((from, run_law(&pts, left.is_some(), false)?));
    }
    let refs: Vec<(f64, &RadiusLaw)> = parts.iter().map(|(s, l)| (*s, l)).collect();
    RadiusLaw::concat(&refs, end, chain.closed.then_some(period))
}

/// (s, radius) points of a law.
type Points = Vec<(f64, f64)>;

/// A smooth law through a run's points (merged where they coincide), held flat at its
/// first and last points where they are a constant edge's; one that never overshoots
/// where the spline swings past the radii.
fn run_law(pts: &[(f64, f64)], held_start: bool, held_end: bool) -> Option<RadiusLaw> {
    let pts = merge_points(pts.to_vec());
    if pts.len() == 1 {
        return Some(RadiusLaw::constant(pts[0].1));
    }
    let law = RadiusLaw::smooth_ends(&pts, held_start.then_some(0.0), held_end.then_some(0.0))?;
    within(&law, &pts).or_else(|| RadiusLaw::monotone(&pts))
}

/// The law when no edge of the chain is constant: a spline through all points, periodic
/// round a closed chain.
fn free_law(
    chain: &Chain,
    ranges: &[Interval],
    whole: Interval,
    mut pts: Vec<(f64, f64)>,
) -> Option<RadiusLaw> {
    let period = whole.width();
    if chain.closed {
        for p in &mut pts {
            p.0 = whole.lo + (p.0 - whole.lo).rem_euclid(period);
        }
    }
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut pts = merge_points(pts);
    if chain.closed && pts.len() > 1 {
        let last = pts[pts.len() - 1];
        if last.0 >= whole.lo + period - 1e-9 * period.max(1.0) {
            pts[0].1 = 0.5 * (pts[0].1 + last.1);
            pts.pop();
        }
    }
    if pts.len() == 1 {
        return Some(RadiusLaw::constant(pts[0].1));
    }
    let law = if chain.closed {
        RadiusLaw::periodic(&pts, period)?
    } else {
        RadiusLaw::smooth(&pts, None)?
    };
    let inner = Interval::new(ranges[0].lo, ranges[ranges.len() - 1].hi);
    let (lo, hi) = pts
        .iter()
        .fold((f64::INFINITY, 0.0f64), |(a, b), p| (a.min(p.1), b.max(p.1)));
    let (rlo, rhi) = law.bounds(inner.lo, inner.hi);
    if rlo >= 0.5 * lo && rhi <= 2.0 * hi {
        return Some(law);
    }
    if chain.closed {
        // one turn of the points with the first repeated past the period, held flat beyond
        let mut ring = pts.clone();
        ring.push((pts[0].0 + period, pts[0].1));
        return RadiusLaw::monotone(&ring);
    }
    RadiusLaw::monotone(&pts)
}

/// Points sorted by s, those at one place (within rounding) merged to their mean.
fn merge_points(mut pts: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, f64, f64)> = Vec::new();
    for (s, r) in pts {
        match merged.last_mut() {
            Some(m) if s - m.0 <= 1e-9 * (1.0 + s.abs()) => {
                m.1 += r;
                m.2 += 1.0;
            }
            _ => merged.push((s, r, 1.0)),
        }
    }
    merged.iter().map(|m| (m.0, m.1 / m.2)).collect()
}

/// The law, if it stays within half and twice the radii it was given.
fn within(law: &RadiusLaw, pts: &[(f64, f64)]) -> Option<RadiusLaw> {
    let (lo, hi) = pts
        .iter()
        .fold((f64::INFINITY, 0.0f64), |(a, b), p| (a.min(p.1), b.max(p.1)));
    let (rlo, rhi) = law.bounds(pts[0].0, pts[pts.len() - 1].0);
    (rlo >= 0.5 * lo && rhi <= 2.0 * hi).then(|| law.clone())
}

/// One face corner at a vertex: the face, the coedge arriving and the one leaving.
#[derive(Clone, Copy, Debug)]
struct Corner {
    face: FaceId,
    cin: CoedgeId,
    cout: CoedgeId,
}

/// A new face closing a corner: surface, same sense, name, its loop.
type Patch = (Surface, bool, Name, Vec<(EdgeId, bool)>);

struct Builder {
    body: Body,
    rolled: Vec<Rolled>,
    root: Name,
    /// Rolled index of each filleted edge.
    which: BTreeMap<EdgeId, usize>,
    /// Position of each coedge in its loop.
    pos: Vec<u32>,
    ends: BTreeMap<(EdgeId, bool), SpringEnd>,
    /// New end (vertex, parameter) of an unfilleted edge cut back at one end (true: its end).
    trims: BTreeMap<(EdgeId, bool), (VertexId, f64)>,
    /// Edges to insert after an arriving coedge, closing that face corner.
    paths: BTreeMap<CoedgeId, Vec<(EdgeId, bool)>>,
    touched: BTreeSet<FaceId>,
    /// Coedges a run-out passes wholly over: their faces' loops drop them.
    dropped: BTreeSet<CoedgeId>,
    /// Faces whose loops are joined up again from their pieces (a spring left them and came back).
    rechain: BTreeSet<FaceId>,
    /// Asked-for setbacks (vertex, edge or all there, distance), and those a corner took.
    setbacks: Vec<(VertexId, Option<EdgeId>, f64)>,
    set_back: BTreeSet<VertexId>,
    /// New faces other than the blends (corner patches).
    patches: Vec<Patch>,
    /// Per switched blend side (rolled index, side): the edges where an end face meets the face
    /// the ball rests on, each with its vertex on the smooth edge and its vertex on the spring.
    caps: BTreeMap<(usize, usize), Vec<Cap>>,
    /// Per switched blend side, how the smooth edge ends at each of the blend's ends: cut at a
    /// parameter (a vertex there), or gone up to its own vertex.
    g_ends: BTreeMap<(usize, usize), Vec<GEnd>>,
    /// Strip faces a switched blend's smooth edge was cut twice on: the piece of it past the
    /// swallowed stretch, whose loop may be a region of its own.
    slivers: Vec<(FaceId, EdgeId, EdgeId)>,
    /// Uses added to a face's loops, joined up with them (`chain_loops`).
    extra: BTreeMap<FaceId, Vec<(EdgeId, bool)>>,
}

impl Builder {
    fn new(body: Body, rolled: Vec<Rolled>, root: Name) -> Builder {
        let which = rolled.iter().enumerate().map(|(i, r)| (r.edge, i)).collect();
        let mut pos = vec![0u32; body.coedges.len()];
        for l in body.loop_ids() {
            for (k, c) in body.lp(l).coedges().enumerate() {
                pos[c.index()] = k as u32;
            }
        }
        Builder {
            body,
            rolled,
            root,
            which,
            pos,
            ends: BTreeMap::new(),
            trims: BTreeMap::new(),
            paths: BTreeMap::new(),
            touched: BTreeSet::new(),
            dropped: BTreeSet::new(),
            rechain: BTreeSet::new(),
            setbacks: Vec::new(),
            set_back: BTreeSet::new(),
            patches: Vec::new(),
            caps: BTreeMap::new(),
            g_ends: BTreeMap::new(),
            slivers: Vec::new(),
            extra: BTreeMap::new(),
        }
    }

    fn build(mut self) -> R<Body> {
        // each vertex with a coedge arriving there
        let mut verts: BTreeMap<VertexId, CoedgeId> = BTreeMap::new();
        for r in &self.rolled {
            let e = self.body.edge(r.edge);
            let c = e.coedges[0];
            let back = self.body.edge(r.edge).coedges[1];
            verts.entry(self.body.coedge_end(c)).or_insert(c);
            verts.entry(self.body.coedge_end(back)).or_insert(back);
        }
        for (v, c) in verts {
            self.resolve(v, c)?;
        }
        self.cut_smooth_edges()?;
        let springs = self.make_springs()?;
        self.trim_edges()?;
        // two blends trimming one face: their springs on it may cross (a hole's rim rounded
        // into a rounded edge beside it), which no single blend's own checks see
        // (a variable chain's blends meet in shared sections, so count as one)
        let mut by_face: BTreeMap<FaceId, Vec<EdgeId>> = BTreeMap::new();
        let mut groups: BTreeMap<FaceId, BTreeSet<(bool, usize)>> = BTreeMap::new();
        for (i, r) in self.rolled.iter().enumerate() {
            let group = r.var.map_or((false, i), |x| (true, x.chain));
            for &c in &self.body.edge(r.edge).coedges {
                let f = self.body.coedge_face(c);
                by_face.entry(f).or_default().push(r.edge);
                groups.entry(f).or_default().insert(group);
            }
        }
        by_face.retain(|f, _| groups[f].len() > 1);
        self.rebuild_faces(&springs);
        self.split_slivers();
        for (f, edges) in by_face {
            if edges.len() > 1 && crate::selfx::face_loops_cross(&self.body, f) {
                return Err(FilletError::Overflow { edge: edges[0] });
            }
        }
        self.add_blends(&springs);
        for (surface, same_sense, name, uses) in std::mem::take(&mut self.patches) {
            let shell = self.body.faces[0].shell;
            let f = self.body.add_face(shell, surface, same_sense, name);
            self.body.add_loop(f, &uses);
        }
        if let Some(s) = self.setbacks.iter().find(|s| !self.set_back.contains(&s.0)) {
            return Err(FilletError::Corner { vertex: s.0 });
        }
        Ok(self.body.compact())
    }

    fn next_in_loop(&self, c: CoedgeId) -> CoedgeId {
        let l = self.body.lp(self.body.coedge(c).lp);
        l.coedge((self.pos[c.index()] as usize + 1) % l.coedges().len())
    }

    fn prev_in_loop(&self, c: CoedgeId) -> CoedgeId {
        let l = self.body.lp(self.body.coedge(c).lp);
        let n = l.coedges().len();
        l.coedge((self.pos[c.index()] as usize + n - 1) % n)
    }

    /// The face corners around `v`, in order: each one's leaving edge is the next one's arriving edge.
    fn corners(&self, start: CoedgeId) -> Option<Vec<Corner>> {
        let b = &self.body;
        let mut out = Vec::new();
        let mut cin = start;
        loop {
            let cout = self.next_in_loop(cin);
            out.push(Corner {
                face: b.coedge_face(cin),
                cin,
                cout,
            });
            cin = b.partner(cout)?;
            if cin == start {
                return Some(out);
            }
            if out.len() > 64 {
                return None;
            }
        }
    }

    fn filleted(&self, c: CoedgeId) -> Option<usize> {
        self.which.get(&self.body.coedge(c).edge).copied()
    }

    /// Which side (index into the edge's coedges) a coedge is.
    fn side(&self, c: CoedgeId) -> usize {
        let e = self.body.edge(self.body.coedge(c).edge);
        usize::from(e.coedges[0] != c)
    }

    fn resolve(&mut self, v: VertexId, start: CoedgeId) -> R<()> {
        let ring = self.corners(start).ok_or(FilletError::Corner { vertex: v })?;
        for k in &ring {
            self.touched.insert(k.face);
        }
        let hot: Vec<usize> = (0..ring.len())
            .filter(|&i| self.filleted(ring[i].cin).is_some())
            .collect();
        match hot[..] {
            [h] if self.rolled[self.filleted(ring[h].cin).expect("hot")]
                .over
                .iter()
                .any(Option::is_some) =>
            {
                self.switched_out(v, &ring, h)
            }
            [h] => {
                // the ball may run off a short or narrow face there: then it rolls on over the next
                let (trims, paths) = (self.trims.clone(), self.paths.clone());
                match self.run_out(v, &ring, h) {
                    Err(FilletError::Overflow { .. }) => {
                        (self.trims, self.paths) = (trims, paths);
                        self.roll_over(v, &ring, h)
                    }
                    other => other,
                }
            }
            [a, b] if self.is_joint(&ring, a, b) => self.joint(v, &ring, a, b),
            _ => {
                let ring = self.rest_ring(v, ring)?;
                self.corner(v, &ring)
            }
        }
    }

    /// The ring with, beside each blend resting on a face beyond its strip face's smooth edge,
    /// that face put in between the two, the smooth edge as the sharp edge between it and the
    /// strip face. Its corner's cin is the smooth edge's coedge arriving at its end along the
    /// swallowed strip, or the blend's own coedge when the face rested on comes first.
    fn rest_ring(&self, v: VertexId, ring: Vec<Corner>) -> R<Vec<Corner>> {
        let k = ring.len();
        // the smooth edge's coedge arriving at its end along blend ri's edge from v
        let arriving = |ri: usize, g: EdgeId| {
            let edge = self.rolled[ri].edge;
            let x = self.body.edge(edge);
            let into = if x.start == v {
                x.curve.deriv(x.range.lo).d1
            } else {
                -x.curve.deriv(x.range.hi).d1
            };
            let gx = self.body.edge(g);
            let (s, e) = (self.body.vertex(gx.start).point, self.body.vertex(gx.end).point);
            let w = if (e - s).dot(into) > 0.0 { gx.end } else { gx.start };
            gx.coedges
                .iter()
                .copied()
                .find(|&c| self.body.coedge_end(c) == w)
                .ok_or(FilletError::Corner { vertex: v })
        };
        let mut out = Vec::with_capacity(k + 2);
        for j in 0..k {
            let c = ring[j];
            match self
                .filleted(c.cin)
                .map(|ri| (ri, self.rolled[ri].over[self.side(c.cin)]))
            {
                Some((ri, Some((cf, g)))) => {
                    let gc = arriving(ri, g)?;
                    if self.body.coedge_face(gc) != c.face {
                        return Err(FilletError::Corner { vertex: v });
                    }
                    out.push(Corner {
                        face: cf,
                        cin: c.cin,
                        cout: c.cin,
                    });
                    out.push(Corner {
                        face: c.face,
                        cin: gc,
                        cout: c.cout,
                    });
                }
                _ => out.push(c),
            }
            let next = ring[(j + 1) % k].cin;
            if let Some(ri) = self.filleted(next)
                && let Some((cf, g)) = self.rolled[ri].over[1 - self.side(next)]
            {
                let gc = arriving(ri, g)?;
                if self.body.coedge_face(gc) != cf {
                    return Err(FilletError::Corner { vertex: v });
                }
                out.push(Corner {
                    face: cf,
                    cin: gc,
                    cout: gc,
                });
            }
        }
        Ok(out)
    }

    /// The blend and side resting on a face beyond smooth edge `g`.
    fn resting_beyond(&self, g: EdgeId) -> Option<(usize, usize)> {
        (0..self.rolled.len())
            .flat_map(|i| [(i, 0), (i, 1)])
            .find(|&(i, s)| self.rolled[i].over[s].is_some_and(|o| o.1 == g))
    }

    /// Each switched side's smooth edge loses the stretch the ball swallowed, between where the
    /// blend's two ends cut it (or up to its own vertex where it ends with the strip): it keeps
    /// the piece before, and a new edge is the piece after, both faces rechained with it.
    fn cut_smooth_edges(&mut self) -> R<()> {
        for ((ri, s), list) in std::mem::take(&mut self.g_ends) {
            let err = FilletError::Overflow {
                edge: self.rolled[ri].edge,
            };
            let (cf, g) = self.rolled[ri].over[s].ok_or(err.clone())?;
            let gx = self.body.edge(g).clone();
            let t_of = |x: &GEnd| match *x {
                GEnd::Cut(t, _) => t,
                GEnd::Gone(w) if w == gx.start => gx.range.lo,
                GEnd::Gone(_) => gx.range.hi,
            };
            let [mut a, mut b] = list[..] else { return Err(err) };
            if t_of(&a) > t_of(&b) {
                std::mem::swap(&mut a, &mut b);
            }
            match (a, b) {
                (GEnd::Cut(ta, va), GEnd::Cut(tb, vb)) => {
                    self.trims.insert((g, true), (va, ta));
                    let name = gx.name.with_name(self.rolled[ri].name);
                    let piece = self.body.add_edge(
                        gx.curve.clone(),
                        Interval::new(tb, gx.range.hi),
                        vb,
                        gx.end,
                        name,
                    );
                    for &c in &gx.coedges {
                        let f = self.body.coedge_face(c);
                        let use_ = (piece, self.body.coedge(c).forward);
                        self.extra.entry(f).or_default().push(use_);
                        self.rechain.insert(f);
                        self.touched.insert(f);
                        if f != cf {
                            self.slivers.push((f, g, piece));
                        }
                    }
                }
                (GEnd::Cut(t, v), GEnd::Gone(_)) => {
                    self.trims.insert((g, true), (v, t));
                }
                (GEnd::Gone(_), GEnd::Cut(t, v)) => {
                    self.trims.insert((g, false), (v, t));
                }
                _ => return Err(err),
            }
        }
        Ok(())
    }

    /// A strip face whose smooth edge was cut twice may fall in two: the stretch of strip between
    /// the piece past the swallowed stretch and the blend's spring is a face of its own.
    fn split_slivers(&mut self) {
        for (f, g, piece) in std::mem::take(&mut self.slivers) {
            let loop_of = |b: &Body, e: EdgeId| {
                b.face(f)
                    .loops
                    .iter()
                    .copied()
                    .find(|&l| b.lp(l).coedges().any(|c| b.coedge(c).edge == e))
            };
            let (Some(lg), Some(lp)) = (loop_of(&self.body, g), loop_of(&self.body, piece)) else {
                continue;
            };
            if lg == lp {
                continue;
            }
            let uses: Vec<(EdgeId, bool)> = self
                .body
                .lp(lp)
                .coedges()
                .map(|c| (self.body.coedge(c).edge, self.body.coedge(c).forward))
                .collect();
            let face = self.body.face(f);
            let (shell, surface, same_sense) = (face.shell, face.surface.clone(), face.same_sense);
            let name = face.name.with_name(self.body.edge(piece).name);
            self.body.remove_loop(f, lp);
            let nf = self.body.add_face(shell, surface, same_sense, name);
            self.body.add_loop(nf, &uses);
        }
    }

    /// The blend and side resting on ring corner j's face, beyond the strip face's smooth edge.
    fn rests(&self, ring: &[Corner], j: usize) -> Option<(usize, usize)> {
        let k = ring.len();
        let on = |c: CoedgeId, other: bool| {
            let ri = self.filleted(c)?;
            let s = usize::from(other) ^ self.side(c);
            self.rolled[ri].over[s]
                .is_some_and(|o| o.0 == ring[j].face)
                .then_some((ri, s))
        };
        on(ring[j].cin, false).or_else(|| on(ring[(j + 1) % k].cin, true))
    }

    /// Whether the blends arriving at ring corners `a` and `b` continue each other
    /// through the vertex: same radius, and every other edge there smooth.
    fn is_joint(&self, ring: &[Corner], a: usize, b: usize) -> bool {
        let k = ring.len();
        let (ra, rb) = (
            &self.rolled[self.filleted(ring[a].cin).expect("hot")],
            &self.rolled[self.filleted(ring[b].cin).expect("hot")],
        );
        let linked = match (ra.var, rb.var) {
            (None, None) => (ra.blend.radius - rb.blend.radius).abs() <= tol::LINEAR,
            (Some(x), Some(y)) => {
                let step = (y.index + x.len - x.index) % x.len;
                x.chain == y.chain && (step == 1 || step == x.len - 1 || (x.len == 1 && x.closed))
            }
            _ => false,
        };
        if !linked || ra.convex != rb.convex {
            return false;
        }
        (0..k).filter(|&j| j != a && j != b).all(|j| {
            let c = ring[j].cin;
            let e = self.body.coedge(c).edge;
            let x = self.body.edge(e);
            self.body.coedge_face(x.coedges[0]) == self.body.coedge_face(x.coedges[1])
                || is_smooth(&self.body, e, ra.blend.radius)
        })
    }

    /// The spine parameter where blend `ri` meets the vertex coedge `c` arrives at: a
    /// variable blend's own end there, else the spine's point nearest the vertex.
    fn end_t(&self, ri: usize, c: CoedgeId, vp: Point3) -> f64 {
        let ro = &self.rolled[ri];
        match ro.var {
            Some(x) if self.arrives_at_end(c) == ro.aligned => x.own.hi,
            Some(x) => x.own.lo,
            None => spine_closest(&ro.blend.spine, vp),
        }
    }

    fn vertex_name(&self, v: VertexId, faces: &[Name]) -> Name {
        Name::NONE
            .with_set(&mut faces.to_vec())
            .with_name(self.body.vertex(v).name)
    }

    fn face_name(&self, f: FaceId) -> Name {
        self.body.face(f).name
    }

    /// An edge on `curve` from `a` (at parameter ta) to `b` (at tb), and whether walking
    /// a to b runs with it.
    fn edge_between(
        &mut self,
        curve: Curve,
        ta: f64,
        tb: f64,
        a: VertexId,
        b: VertexId,
        name: Name,
    ) -> (EdgeId, bool) {
        if ta <= tb {
            (self.body.add_edge(curve, Interval::new(ta, tb), a, b, name), true)
        } else {
            (
                self.body.add_edge(curve, Interval::new(tb, ta), b, a, name),
                false,
            )
        }
    }

    /// Where an unfilleted edge, arriving at `v` through coedge `c`, is cut at `p`.
    /// With `extend`, p may lie on the edge carried on past v, over face `extend`:
    /// a blend's end cut by a face it runs out across, beyond that face's corner.
    fn trim(
        &mut self,
        c: CoedgeId,
        p: Point3,
        x: VertexId,
        blend_edge: EdgeId,
        extend: Option<FaceId>,
    ) -> R<()> {
        let co = self.body.coedge(c).clone();
        let e = self.body.edge(co.edge);
        let at_end = co.forward;
        let mid = e.range.mid();
        let window = if at_end {
            Interval::new(mid, e.range.hi)
        } else {
            Interval::new(e.range.lo, mid)
        };
        let mut t = e.curve.closest(p, window);
        if e.curve.eval(t).distance(p) > tol::LINEAR {
            // a short edge the blend crosses past its middle
            t = e.curve.closest(p, e.range);
        }
        if e.curve.eval(t).distance(p) > tol::LINEAR
            && let Some(f) = extend
        {
            let v = if at_end { e.range.hi } else { e.range.lo };
            // as far past v as p is from it: the edge may be shorter than the blend's radius
            let speed = e.curve.deriv(v).d1.length().max(1e-12);
            let mut w = window.width().max(2.0 * e.curve.eval(v).distance(p) / speed);
            if let Some(period) = e.curve.domain().period {
                w = w.min(0.5 * period);
            }
            let past = if at_end {
                Interval::new(v, v + w)
            } else {
                Interval::new(v - w, v)
            };
            let u = e.curve.closest(p, past);
            let ext = if at_end {
                Interval::new(v, u)
            } else {
                Interval::new(u, v)
            };
            if e.curve.eval(u).distance(p) <= tol::LINEAR
                && ext.width() > 0.0
                && inside_face(&self.body, f, &e.curve, ext)
            {
                t = u;
            }
        }
        if e.curve.eval(t).distance(p) > tol::LINEAR {
            return Err(FilletError::Overflow { edge: blend_edge });
        }
        let far = if at_end { e.start } else { e.end };
        if self.body.vertex(far).point.distance(p) <= tol::LINEAR {
            return Err(FilletError::Tangency { edge: blend_edge });
        }
        self.trims.insert((co.edge, at_end), (x, t));
        Ok(())
    }

    /// A lone blend at `v`: it runs out across the faces between its two sides.
    fn run_out(&mut self, v: VertexId, ring: &[Corner], hot: usize) -> R<()> {
        let k = ring.len();
        // corner j of the rotated ring: corner 0 has the blend's edge arriving
        let at = |j: usize| ring[(hot + j) % k];
        let ri = self.filleted(at(0).cin).expect("hot corner");
        let edge = self.rolled[ri].edge;
        let vp = self.body.vertex(v).point;
        let t_ref = self.end_t(ri, at(0).cin, vp);
        let radius = self.rolled[ri].blend.radius;
        let reach = self.body.edge_box(edge).diagonal() + 4.0 * radius;
        // points X[1..k-1]: X[j] on the edge arriving at corner j
        let mut xs: Vec<(Point3, VertexId)> = vec![(vp, v); k];
        let mut tside = [0.0; 2];
        for (j, (spring_c, other)) in [
            (1, (at(0).cin, at(1).face)),
            (k - 1, (at(k - 1).cout, at(k - 2).face)),
        ] {
            let ro = &self.rolled[ri];
            let s = self.side(spring_c);
            let spring = &ro.springs[s];
            let surf = &self.body.face(other).surface;
            let (t, p) = nearest_hit(spring, search_window(spring, t_ref, reach), surf, vp)
                .ok_or(FilletError::Overflow { edge })?;
            tside[s] = t;
            let names = [
                self.face_name(at(j).face),
                self.face_name(at(j - 1).face),
                ro.name,
            ];
            let name = self.vertex_name(v, &names);
            xs[j] = (p, self.body.add_vertex(p, name));
        }
        for (j, slot) in xs.iter_mut().enumerate().take(k - 1).skip(2) {
            let e = self.body.edge(self.body.coedge(at(j).cin).edge);
            let (curve, range) = (e.curve.clone(), e.range);
            let surf = self.rolled[ri].blend.surface.clone();
            let (_, p) = nearest_hit(&curve, range, &surf, vp).ok_or(FilletError::Overflow { edge })?;
            let names = [
                self.face_name(at(j).face),
                self.face_name(at(j - 1).face),
                self.rolled[ri].name,
            ];
            let name = self.vertex_name(v, &names);
            *slot = (p, self.body.add_vertex(p, name));
        }
        for (j, &(p, x)) in xs.clone().iter().enumerate().skip(1) {
            // the edges either side of the blend's own faces may carry on past v
            let extend = match j {
                1 => Some(at(0).face),
                _ if j == k - 1 => Some(at(k - 1).face),
                _ => None,
            };
            self.trim(at(j).cin, p, x, edge, extend)?;
        }
        // cross curves on the faces in between, each from X[j] to X[j+1]
        let mut back: Vec<(EdgeId, bool)> = Vec::new();
        for j in 1..k - 1 {
            let f = at(j).face;
            let ro = &self.rolled[ri];
            let (a, b) = (xs[j], xs[j + 1]);
            let (curve, ta, tb) = cross_curve(&ro.blend, &self.body.face(f).surface, a.0, b.0)
                .ok_or(FilletError::Overflow { edge })?;
            let name = Name::NONE
                .with_set(&mut [ro.name, self.face_name(f)])
                .with_name(self.body.vertex(v).name);
            let (eid, fwd) = self.edge_between(curve, ta, tb, a.1, b.1, name);
            self.paths.insert(at(j).cin, vec![(eid, fwd)]);
            back.push((eid, !fwd));
        }
        back.reverse();
        let s_in = self.side(at(0).cin);
        let mut end = SpringEnd {
            v: [v; 2],
            t: tside,
            path: back,
        };
        end.v[s_in] = xs[1].1;
        end.v[1 - s_in] = xs[k - 1].1;
        let at_end = self.arrives_at_end(at(0).cin);
        self.ends.insert((edge, at_end), end);
        Ok(())
    }

    /// A lone blend resting on a face beyond a smooth edge of one of its edge's faces (`over`),
    /// run out across the one face at `v` between its sides. On that side the cut ends on the face
    /// it rests on: where the smooth edge ends on the end face, the swallowed strip's end edge goes
    /// and the next edge is cut back; where it carries on past, the strip's end edge is carried on
    /// to the smooth edge, which is cut there, and a new edge runs from it over the face rested on.
    fn switched_out(&mut self, v: VertexId, ring: &[Corner], hot: usize) -> R<()> {
        let k = ring.len();
        let at = |j: usize| ring[(hot + j) % k];
        let ri = self.filleted(at(0).cin).expect("hot corner");
        let edge = self.rolled[ri].edge;
        let err = FilletError::Overflow { edge };
        if k != 3 {
            return Err(err);
        }
        let vp = self.body.vertex(v).point;
        let t_ref = self.end_t(ri, at(0).cin, vp);
        let reach = self.body.edge_box(edge).diagonal() + 4.0 * self.rolled[ri].blend.radius;
        let ef = at(1).face;
        let es = self.body.face(ef).surface.clone();
        let mut xs = [(vp, v); 2];
        let mut tside = [0.0; 2];
        // the end face's uses before the cut (first side) and after it (last side)
        let (mut before, mut after) = (Vec::new(), Vec::new());
        let mut key = at(1).cin;
        for (n, (j, spring_c)) in [(1, at(0).cin), (k - 1, at(k - 1).cout)].into_iter().enumerate() {
            let s = self.side(spring_c);
            let spring = self.rolled[ri].springs[s].clone();
            let (t, p) =
                nearest_hit(&spring, search_window(&spring, t_ref, reach), &es, vp).ok_or(err.clone())?;
            tside[s] = t;
            let names = [
                self.face_name(at(j).face),
                self.face_name(at(j - 1).face),
                self.rolled[ri].name,
            ];
            let xv = self.body.add_vertex(p, self.vertex_name(v, &names));
            xs[n] = (p, xv);
            let own = if n == 0 { at(0).face } else { at(k - 1).face };
            let Some((cf, g)) = self.rolled[ri].over[s] else {
                self.trim(at(j).cin, p, xv, edge, Some(own))?;
                continue;
            };
            let gx = self.body.edge(g).clone();
            let (_, gp) = nearest_hit(&gx.curve, gx.range, &es, vp).ok_or(err.clone())?;
            // the strip's end edge: on the end face, from v (last side) or to v (first side)
            let strip_end = if n == 0 { at(1).cin } else { at(1).cout };
            let w = [gx.start, gx.end]
                .into_iter()
                .find(|&w| self.body.vertex(w).point.distance(gp) <= tol::LINEAR);
            if let Some(w) = w {
                // the smooth edge ends on the end face: the strip's end edge goes, C's is cut back
                let x = self.body.edge(self.body.coedge(strip_end).edge);
                if x.start != w && x.end != w {
                    return Err(err);
                }
                let other = self.body.partner(strip_end).ok_or(err.clone())?;
                self.drop_all(&[strip_end, other], edge)?;
                self.g_ends.entry((ri, s)).or_default().push(GEnd::Gone(w));
                let next = if n == 0 {
                    self.prev_in_loop(strip_end)
                } else {
                    self.next_in_loop(strip_end)
                };
                let across = self.body.partner(next).ok_or(err.clone())?;
                if self.body.coedge_face(across) != cf {
                    return Err(err);
                }
                let arriving = if self.body.coedge_end(next) == w {
                    next
                } else {
                    across
                };
                self.trim(arriving, p, xv, edge, None)?;
                if n == 0 {
                    key = next;
                }
            } else {
                // it carries on: the strip's end edge reaches it, it is cut there, a new edge on C
                let gname = Name::NONE.with_set(&mut [self.rolled[ri].name, gx.name]);
                let xg = self.body.add_vertex(gp, gname);
                self.trim(at(j).cin, gp, xg, edge, Some(own))?;
                let tg = gx.curve.closest(gp, gx.range);
                self.g_ends.entry((ri, s)).or_default().push(GEnd::Cut(tg, xg));
                let (curve, ta, tb) = meet(&self.body.face(cf).surface, &es, gp, p).ok_or(err.clone())?;
                let cname = Name::NONE
                    .with_set(&mut [self.rolled[ri].name, self.face_name(cf), self.face_name(ef)])
                    .with_name(self.body.vertex(v).name);
                let ce = self.body.add_edge(curve, Interval::new(ta, tb), xg, xv, cname);
                self.caps.entry((ri, s)).or_default().push((ce, xg, xv));
                if n == 0 {
                    before.push((ce, true));
                } else {
                    after.push((ce, false));
                }
            }
            self.touched.insert(cf);
        }
        let ro = &self.rolled[ri];
        let ((a, va), (b, vb)) = (xs[0], xs[1]);
        let (curve, ta, tb) = cross_curve(&ro.blend, &es, a, b).ok_or(err.clone())?;
        let name = Name::NONE
            .with_set(&mut [ro.name, self.face_name(ef)])
            .with_name(self.body.vertex(v).name);
        let (eid, fwd) = self.edge_between(curve, ta, tb, va, vb, name);
        let mut path = before;
        path.push((eid, fwd));
        path.extend(after);
        self.paths.insert(key, path);
        self.touched.insert(ef);
        let s_in = self.side(at(0).cin);
        let mut end = SpringEnd {
            v: [v; 2],
            t: tside,
            path: vec![(eid, !fwd)],
        };
        end.v[s_in] = va;
        end.v[1 - s_in] = vb;
        let at_end = self.arrives_at_end(at(0).cin);
        self.ends.insert((edge, at_end), end);
        Ok(())
    }

    /// A lone blend whose run-out leaves the faces round its vertex: the ball runs off a
    /// short or narrow face onto the faces beyond. The blend's end is walked face by face
    /// from one spring to the other: each edge where it crosses a face's boundary is cut,
    /// and the corners and edges it passes wholly over are dropped.
    fn roll_over(&mut self, v: VertexId, ring: &[Corner], hot: usize) -> R<()> {
        let k = ring.len();
        let at = |j: usize| ring[(hot + j) % k];
        let ri = self.filleted(at(0).cin).expect("hot corner");
        let edge = self.rolled[ri].edge;
        let err = FilletError::Overflow { edge };
        let vp = self.body.vertex(v).point;
        let t_ref = self.end_t(ri, at(0).cin, vp);
        let reach = self.body.edge_box(edge).diagonal() + 4.0 * self.rolled[ri].blend.radius;
        let (s_in, s_out) = (self.side(at(0).cin), self.side(at(k - 1).cout));
        let (a0, b0) = (at(0).face, at(k - 1).face);
        let blend = self.rolled[ri].blend.surface.clone();
        let name = self.rolled[ri].name;
        let on_edge = |b: &Builder, c: CoedgeId, p: Point3| {
            let e = b.body.edge(b.body.coedge(c).edge);
            e.curve.eval(e.curve.closest(p, e.range)).distance(p) <= tol::LINEAR
        };
        // where spring s, near the vertex, crosses coedge c's edge (meeting the surface of face f)
        let spring_hit = |b: &Builder, s: usize, c: CoedgeId, f: FaceId| -> Option<(f64, Point3)> {
            let spring = &b.rolled[ri].springs[s];
            let across = &b.body.face(f).surface;
            let hits = match curve_surface(spring, search_window(spring, t_ref, reach), across) {
                CurveSurface::Hits(h) => h,
                _ => return None,
            };
            hits.into_iter()
                .filter(|h| on_edge(b, c, h.point))
                .min_by(|x, y| (x.t - t_ref).abs().total_cmp(&(y.t - t_ref).abs()))
                .map(|h| (h.t, h.point))
        };
        let mut dropped: Vec<CoedgeId> = Vec::new();
        let names = |b: &Builder, c: CoedgeId| {
            let e = b.body.coedge(c).edge;
            Name::NONE
                .with_set(&mut [name, b.body.edge(e).name])
                .with_name(b.body.vertex(v).name)
        };
        // off face a0: the spring's first crossing with an edge of a0 after the vertex
        let mut c = at(0).cout;
        let (t_in, x_in) = loop {
            if c == at(0).cin || dropped.len() > 64 {
                return Err(err);
            }
            let across = self.body.coedge_face(self.body.partner(c).ok_or(err.clone())?);
            if let Some(hit) = spring_hit(self, s_in, c, across) {
                break hit;
            }
            dropped.push(c);
            c = self.next_in_loop(c);
        };
        let xv_in = self.body.add_vertex(x_in, names(self, c));
        let mut cur = self.body.partner(c).ok_or(err.clone())?;
        self.trim(cur, x_in, xv_in, edge, None)?;
        // across the faces beyond, each from its entry to its exit, to face b0
        let (mut entry, mut back) = ((x_in, xv_in), Vec::new());
        let (t_out, xv_out) = loop {
            let f = self.body.coedge_face(cur);
            if f == a0 || f == b0 || back.len() > 16 {
                return Err(err);
            }
            self.touched.insert(f);
            let mut d = self.next_in_loop(cur);
            let (exit, t_out) = loop {
                if d == cur {
                    return Err(err);
                }
                let g = self.body.coedge_face(self.body.partner(d).ok_or(err.clone())?);
                let hit = if g == b0 {
                    spring_hit(self, s_out, d, f).map(|(t, p)| (p, Some(t)))
                } else {
                    let e = self.body.edge(self.body.coedge(d).edge);
                    let (curve, range) = (e.curve.clone(), e.range);
                    nearest_hit(&curve, range, &blend, vp)
                        .filter(|h| h.1.distance(entry.0) > tol::LINEAR)
                        .map(|h| (h.1, None))
                };
                if let Some(h) = hit {
                    break h;
                }
                dropped.push(d);
                d = self.next_in_loop(d);
            };
            let xv = self.body.add_vertex(exit, names(self, d));
            let (curve, ta, tb) =
                cross_curve(&self.rolled[ri].blend, &self.body.face(f).surface, entry.0, exit)
                    .ok_or(err.clone())?;
            let ename = Name::NONE
                .with_set(&mut [name, self.face_name(f)])
                .with_name(self.body.vertex(v).name)
                .with(back.len() as u64);
            let (eid, fwd) = self.edge_between(curve, ta, tb, entry.1, xv, ename);
            self.paths.insert(cur, vec![(eid, fwd)]);
            back.push((eid, !fwd));
            cur = self.body.partner(d).ok_or(err.clone())?;
            self.trim(cur, exit, xv, edge, None)?;
            entry = (exit, xv);
            if let Some(t) = t_out {
                break (t, xv);
            }
        };
        // on face b0, from the last cut to the blend's edge
        let mut d = self.next_in_loop(cur);
        while d != at(k - 1).cout {
            if dropped.len() > 64 {
                return Err(err);
            }
            dropped.push(d);
            d = self.next_in_loop(d);
        }
        // what is dropped goes from both its faces, and no other blend ends there
        let gone: BTreeSet<CoedgeId> = dropped.iter().copied().collect();
        for &d in &dropped {
            let e = self.body.coedge(d).edge;
            let x = self.body.edge(e);
            if !self.body.partner(d).is_some_and(|p| gone.contains(&p))
                || self.which.contains_key(&e)
                || self.rolled.iter().any(|r| {
                    let y = self.body.edge(r.edge);
                    [y.start, y.end]
                        .iter()
                        .any(|&w| w != v && (w == x.start || w == x.end))
                })
            {
                return Err(err);
            }
            self.touched.insert(self.body.coedge_face(d));
        }
        self.dropped.extend(gone);
        back.reverse();
        let mut end = SpringEnd {
            v: [v; 2],
            t: [0.0; 2],
            path: back,
        };
        (end.v[s_in], end.t[s_in]) = (xv_in, t_in);
        (end.v[s_out], end.t[s_out]) = (xv_out, t_out);
        let at_end = self.arrives_at_end(at(0).cin);
        self.ends.insert((edge, at_end), end);
        Ok(())
    }

    /// Whether coedge `c` (arriving at a vertex) arrives at its edge's end rather than its start.
    fn arrives_at_end(&self, c: CoedgeId) -> bool {
        self.body.coedge(c).forward
    }

    /// Two blends of a tangent chain meet at `v` (arriving at ring corners `ia` and
    /// `ib`), every other edge there smooth: they share the ball's cross-section.
    /// It lies where a spring crosses an edge between faces on different surfaces;
    /// other smooth edges (seams) are cut where the springs cross them, and the
    /// springs between are walked by both the blend and the face corner.
    fn joint(&mut self, v: VertexId, ring: &[Corner], ia: usize, ib: usize) -> R<()> {
        let k = ring.len();
        let err = FilletError::Corner { vertex: v };
        let (ra, rb) = (
            self.filleted(ring[ia].cin).expect("hot"),
            self.filleted(ring[ib].cin).expect("hot"),
        );
        let vp = self.body.vertex(v).point;
        let ea = self.rolled[ra].edge;
        // blend a's and b's spring sides on the P faces (ring ia..ib); the X faces are the rest
        let (sa, sb) = (self.side(ring[ia].cin), self.side(ring[(ib + k - 1) % k].cout));
        let span = (ib + k - ia) % k;
        // the smooth edges on each side, from a's end to b's
        let p_edges: Vec<CoedgeId> = (1..span).map(|d| ring[(ia + d) % k].cin).collect();
        let x_edges: Vec<CoedgeId> = (span + 1..k).rev().map(|d| ring[(ia + d) % k].cin).collect();
        // the section: where a spring crosses an edge between two surfaces, else any smooth edge
        let var = self.rolled[ra].var.is_some();
        let t0 = self.end_t(ra, ring[ia].cin, vp);
        let mut ta = None;
        let mut any = None;
        // a variable chain's section is its blends' shared end; edges between surfaces pass through its feet
        for (edges, s) in [(&p_edges, sa), (&x_edges, 1 - sa)] {
            if var {
                let foot = self.rolled[ra].springs[s].eval(t0);
                for &c in edges.iter() {
                    let e = self.body.edge(self.body.coedge(c).edge);
                    let (f0, f1) = (
                        self.body.coedge_face(e.coedges[0]),
                        self.body.coedge_face(e.coedges[1]),
                    );
                    let change = f0 != f1 && self.body.face(f0).surface != self.body.face(f1).surface;
                    if change && e.curve.eval(e.curve.closest(foot, e.range)).distance(foot) > tol::LINEAR {
                        return Err(err);
                    }
                }
                ta = Some(t0);
                continue;
            }
            for &c in edges.iter() {
                let e = self.body.edge(self.body.coedge(c).edge);
                let (f0, f1) = (
                    self.body.coedge_face(e.coedges[0]),
                    self.body.coedge_face(e.coedges[1]),
                );
                let change = f0 != f1 && self.body.face(f0).surface != self.body.face(f1).surface;
                if !change && any.is_some() {
                    continue;
                }
                let t = crossing(&self.rolled[ra].springs[s], t0, &e.curve, e.range).ok_or(err.clone())?;
                if !change {
                    any = Some(t);
                    continue;
                }
                let spine = &self.rolled[ra].blend.spine;
                match ta {
                    Some(u) if spine.eval(u).distance(spine.eval(t)) > tol::LINEAR => return Err(err),
                    Some(_) => {}
                    None => ta = Some(t),
                }
            }
        }
        let ta = ta.or(any).unwrap_or(t0);
        let ca = self.rolled[ra].blend.spine.eval(ta);
        let tb = if var {
            self.end_t(rb, ring[ib].cin, vp)
        } else {
            spine_closest(&self.rolled[rb].blend.spine, ca)
        };
        if ca.distance(self.rolled[rb].blend.spine.eval(tb)) > tol::LINEAR {
            return Err(err);
        }
        let p = self.rolled[ra].springs[sa].eval(ta);
        let x = self.rolled[ra].springs[1 - sa].eval(ta);
        if p.distance(self.rolled[rb].springs[sb].eval(tb)) > tol::LINEAR
            || x.distance(self.rolled[rb].springs[1 - sb].eval(tb)) > tol::LINEAR
        {
            return Err(err);
        }
        let (na, nb) = (self.rolled[ra].name, self.rolled[rb].name);
        let fname = |j: usize| self.face_name(ring[j % k].face);
        let vp_name = self.vertex_name(v, &[fname(ia), fname(ib + k - 1), na, nb]);
        let vx_name = self.vertex_name(v, &[fname(ib), fname(ia + k - 1), na, nb]);
        let pv = self.body.add_vertex(p, vp_name);
        let xv = self.body.add_vertex(x, vx_name);
        // a blend resting beyond a smooth edge on one side meets the other there: cut it there
        for (ri, sp) in [(ra, sa), (rb, sb)] {
            for (side, q, qv) in [(sp, p, pv), (1 - sp, x, xv)] {
                if let Some((_, g)) = self.rolled[ri].over[side] {
                    let gx = self.body.edge(g);
                    let t = gx.curve.closest(q, gx.range);
                    if gx.curve.eval(t).distance(q) > tol::LINEAR {
                        return Err(err);
                    }
                    self.g_ends.entry((ri, side)).or_default().push(GEnd::Cut(t, qv));
                }
            }
        }
        let mid = |b: &Builder, ri: usize| match b.rolled[ri].var {
            Some(x) => x.own.mid(),
            None => {
                let e = b.body.edge(b.rolled[ri].edge);
                spine_closest(&b.rolled[ri].blend.spine, e.curve.eval(e.range.mid()))
            }
        };
        let (mid_a, mid_b) = (mid(self, ra), mid(self, rb));
        let ahead = |b: &Builder, ri: usize, t: f64, from: f64, mid: f64| {
            let period = b.rolled[ri].blend.spine.domain().period;
            along(t - from, period) * along(mid - from, period) > 0.0
        };
        // per side: the spring pieces from a's end to b's, the section foot's place among
        // them, and each smooth edge's
        let mut sides = Vec::with_capacity(2);
        for (edges, s_a, s_b, foot, fv) in [(&p_edges, sa, sb, p, pv), (&x_edges, 1 - sa, 1 - sb, x, xv)] {
            // landings: (blend, side, parameter, vertex); the foot is (None, ..)
            let mut on_a: Vec<(f64, VertexId, usize)> = Vec::new();
            let mut on_b: Vec<(f64, VertexId, usize)> = Vec::new();
            let mut at_foot: Vec<usize> = Vec::new();
            for (m, &c) in edges.iter().enumerate() {
                let e = self.body.edge(self.body.coedge(c).edge);
                let (curve, range, ename) = (e.curve.clone(), e.range, e.name);
                // a variable blend's spring stops at the section: the crossing may be b's
                let hit = crossing(&self.rolled[ra].springs[s_a], ta, &curve, range);
                if hit.is_none() && !var {
                    return Err(err);
                }
                let name = Name::NONE.with_set(&mut [na, nb]).with_name(ename);
                let q = hit.map(|t| (t, self.rolled[ra].springs[s_a].eval(t)));
                if let Some((_, q)) = q
                    && q.distance(foot) <= tol::LINEAR
                {
                    self.trim(c, foot, fv, ea, None)?;
                    at_foot.push(m);
                } else if let Some((t, q)) = q
                    && ahead(self, ra, t, ta, mid_a)
                {
                    let qv = self.body.add_vertex(q, name);
                    self.trim(c, q, qv, ea, None)?;
                    on_a.push((t, qv, m));
                } else {
                    let u = crossing(&self.rolled[rb].springs[s_b], tb, &curve, range).ok_or(err.clone())?;
                    if !ahead(self, rb, u, tb, mid_b) {
                        return Err(err);
                    }
                    let q = self.rolled[rb].springs[s_b].eval(u);
                    let qv = self.body.add_vertex(q, name);
                    self.trim(c, q, qv, ea, None)?;
                    on_b.push((u, qv, m));
                }
            }
            // in ring order the landings must run from a's far end to b's
            let pa = self.rolled[ra].blend.spine.domain().period;
            let pb = self.rolled[rb].blend.spine.domain().period;
            let order: Vec<usize> = on_a.iter().chain(on_b.iter()).map(|l| l.2).collect();
            let dist_a: Vec<f64> = on_a.iter().map(|l| along(l.0 - ta, pa).abs()).collect();
            let dist_b: Vec<f64> = on_b.iter().map(|l| along(l.0 - tb, pb).abs()).collect();
            let a_end = on_a.len();
            let mut placed = order.clone();
            placed.splice(a_end..a_end, at_foot.iter().copied());
            if at_foot.len() > 1
                || !placed.windows(2).all(|w| w[0] < w[1])
                || !dist_a.windows(2).all(|w| w[0] > w[1])
                || !dist_b.windows(2).all(|w| w[0] < w[1])
            {
                return Err(err);
            }
            // nodes from a's end to b's: a's landings, the foot, b's landings
            let mut nodes: Vec<(VertexId, Option<(usize, f64)>)> =
                on_a.iter().map(|l| (l.1, Some((ra, l.0)))).collect();
            nodes.push((fv, None));
            nodes.extend(on_b.iter().map(|l| (l.1, Some((rb, l.0)))));
            let foot_at = a_end;
            let mut segs = Vec::with_capacity(nodes.len() - 1);
            for w in 0..nodes.len() - 1 {
                let (ri, s, tf) = if w < foot_at { (ra, s_a, ta) } else { (rb, s_b, tb) };
                let t_of = |n: &(VertexId, Option<(usize, f64)>)| n.1.map_or(tf, |x| x.1);
                let (t0, t1) = (t_of(&nodes[w]), t_of(&nodes[w + 1]));
                let curve = self.rolled[ri].springs[s].clone();
                let name = Name::NONE
                    .with_set(&mut [self.rolled[ri].name])
                    .with_name(self.body.vertex(nodes[w + 1].0).name)
                    .with(w as u64);
                segs.push(self.edge_between(curve, t0, t1, nodes[w].0, nodes[w + 1].0, name));
            }
            // node of each boundary in a-to-b order: a's end, the smooth edges, b's end
            let mut bounds = vec![0usize; edges.len() + 2];
            for (n, l) in on_a.iter().enumerate() {
                bounds[l.2 + 1] = n;
            }
            for &m in &at_foot {
                bounds[m + 1] = foot_at;
            }
            for (n, l) in on_b.iter().enumerate() {
                bounds[l.2 + 1] = foot_at + 1 + n;
            }
            bounds[edges.len() + 1] = nodes.len() - 1;
            let ends = (nodes[0], nodes[nodes.len() - 1]);
            sides.push((segs, bounds, foot_at, ends));
        }
        let rev = |segs: &[(EdgeId, bool)]| -> Vec<(EdgeId, bool)> {
            segs.iter().rev().map(|&(e, f)| (e, !f)).collect()
        };
        // face corners: P ones walk from a to b, X ones from b to a
        for d in 0..span {
            let (segs, bounds, _, _) = &sides[0];
            let path = segs[bounds[d]..bounds[d + 1]].to_vec();
            if !path.is_empty() {
                self.paths.insert(ring[(ia + d) % k].cin, path);
            }
        }
        for d in 0..k - span {
            let (segs, bounds, _, _) = &sides[1];
            let n = bounds.len();
            let path = rev(&segs[bounds[n - 2 - d]..bounds[n - 1 - d]]);
            if !path.is_empty() {
                self.paths.insert(ring[(ib + d) % k].cin, path);
            }
        }
        let (circle, angle) = self.rolled[ra].blend.section(ta).ok_or(err.clone())?;
        // the section runs from the foot on the blend's first surface (its side 0)
        let (from_v, to_v) = if sa == 0 { (pv, xv) } else { (xv, pv) };
        let name = Name::NONE
            .with_set(&mut [na, nb])
            .with_str("section")
            .with_name(self.body.vertex(v).name);
        let s = self.body.add_edge(
            Curve::Circle(circle),
            Interval::new(0.0, angle),
            from_v,
            to_v,
            name,
        );
        let x_to_p = from_v == xv;
        let ((p_segs, _, p_foot, p_ends), (x_segs, _, x_foot, x_ends)) = (&sides[0], &sides[1]);
        let t_end = |n: &(VertexId, Option<(usize, f64)>), t: f64| n.1.map_or(t, |x| x.1);
        // blend a, from its X end round through the section to its P end
        let mut path = x_segs[..*x_foot].to_vec();
        path.push((s, x_to_p));
        path.extend(rev(&p_segs[..*p_foot]));
        let mut end_a = SpringEnd {
            v: [v; 2],
            t: [ta; 2],
            path,
        };
        (end_a.v[sa], end_a.t[sa]) = (p_ends.0.0, t_end(&p_ends.0, ta));
        (end_a.v[1 - sa], end_a.t[1 - sa]) = (x_ends.0.0, t_end(&x_ends.0, ta));
        // blend b, from its P end round to its X end
        let mut path = rev(&p_segs[*p_foot..]);
        path.push((s, !x_to_p));
        path.extend(x_segs[*x_foot..].iter().copied());
        let mut end_b = SpringEnd {
            v: [v; 2],
            t: [tb; 2],
            path,
        };
        (end_b.v[sb], end_b.t[sb]) = (p_ends.1.0, t_end(&p_ends.1, tb));
        (end_b.v[1 - sb], end_b.t[1 - sb]) = (x_ends.1.0, t_end(&x_ends.1, tb));
        let eb = self.rolled[rb].edge;
        self.ends.insert((ea, self.arrives_at_end(ring[ia].cin)), end_a);
        self.ends.insert((eb, self.arrives_at_end(ring[ib].cin)), end_b);
        Ok(())
    }

    /// The ball's centre halfway from `v` to the far end of the filleted tangent chain that
    /// leaves it along `e`, or None when the chain comes back to `v`.
    fn chain_middle(&self, v: VertexId, e: EdgeId, spine: &Curve) -> Option<Point3> {
        let blended: BTreeSet<EdgeId> = self.rolled.iter().map(|r| r.edge).collect();
        let other = |e: EdgeId, w: VertexId| {
            let x = self.body.edge(e);
            if x.start == w { x.end } else { x.start }
        };
        // unit tangent of e leaving its end w
        let leaving = |e: EdgeId, w: VertexId| {
            let x = self.body.edge(e);
            let d = if x.start == w {
                x.curve.deriv(x.range.lo).d1
            } else {
                -x.curve.deriv(x.range.hi).d1
            };
            d.normalize().map_or(Vec3::ZERO, |u| u.get())
        };
        // the chain's edges, each with the vertex it is walked from
        let mut walk = vec![(e, v)];
        let (mut e, mut w) = (e, other(e, v));
        for _ in 0..self.rolled.len() {
            if w == v {
                return None;
            }
            let back = leaving(e, w);
            let next = blended.iter().copied().find(|&o| {
                let x = self.body.edge(o);
                o != e && (x.start == w || x.end == w) && leaving(o, w).dot(back) < -1.0 + 1e-9
            });
            let Some(o) = next else { break };
            walk.push((o, w));
            (e, w) = (o, other(o, w));
        }
        if w == v {
            return None;
        }
        // halfway along it (not across: a chain round a tip comes back beside where it left)
        let span = |e: EdgeId, from: VertexId| {
            let x = self.body.edge(e);
            if x.start == from {
                (x.range.lo, x.range.hi)
            } else {
                (x.range.hi, x.range.lo)
            }
        };
        let lengths: Vec<f64> = walk
            .iter()
            .map(|&(e, from)| {
                let (a, b) = span(e, from);
                arc_length(&self.body.edge(e).curve, a.min(b), a.max(b))
            })
            .collect();
        let mut left = 0.5 * lengths.iter().sum::<f64>();
        for (&(e, from), &l) in walk.iter().zip(&lengths) {
            if left <= l {
                let (a, b) = span(e, from);
                let c = &self.body.edge(e).curve;
                let t = if left > 0.0 {
                    at_length(c, a, b, left).unwrap_or(b)
                } else {
                    a
                };
                return Some(spine.eval(spine_closest(spine, c.eval(t))));
            }
            left -= l;
        }
        None
    }

    /// The corner at `v` as the corner solver sees it: faces counter-clockwise
    /// seen from outside (the ring reversed), the edge between each and the next.
    /// Also the ring corner of each blend.
    fn corner_input(&self, v: VertexId, ring: &[Corner]) -> (CornerInput, Vec<usize>) {
        let k = ring.len();
        let vp = self.body.vertex(v).point;
        let mut faces = Vec::with_capacity(k);
        let mut face_surfaces = Vec::with_capacity(k);
        let mut edges = Vec::with_capacity(k);
        let mut blends = Vec::new();
        let mut blend_ring = Vec::new();
        for i in 0..k {
            let j = (k - i) % k;
            let f = ring[j].face;
            faces.push(f);
            face_surfaces.push((self.body.face(f).surface.clone(), self.body.face(f).same_sense));
            // between faces[i] (ring j) and faces[i + 1] (ring j - 1) lies the edge arriving at ring j
            let c = ring[j].cin;
            // its parameter grows away from v unless the coedge arriving here runs with it
            let away = if self.body.coedge(c).forward { -1.0 } else { 1.0 };
            match self.filleted(c) {
                Some(ri) => {
                    let ro = &self.rolled[ri];
                    let middle = self.chain_middle(v, self.body.coedge(c).edge, &ro.blend.spine);
                    edges.push(CornerEdge::Blend(blends.len()));
                    blends.push(CornerBlend {
                        surface: ro.blend.surface.clone(),
                        same_sense: ro.convex,
                        radius: ro.blend.radius,
                        convex: ro.convex,
                        spine: ro.blend.spine.clone(),
                        out: if ro.aligned { away } else { -away },
                        setback: self.end_t(ri, c, vp),
                        middle,
                    });
                    blend_ring.push(j);
                }
                None => edges.push(CornerEdge::Sharp {
                    curve: self.body.edge(self.body.coedge(c).edge).curve.clone(),
                    out: away,
                }),
            }
        }
        (
            CornerInput {
                vertex: v,
                point: vp,
                blends,
                faces,
                face_surfaces,
                edges,
            },
            blend_ring,
        )
    }

    /// The spine parameter each blend at corner `v` (in corner input order) was asked to stop
    /// at: where its section crosses its edge the asked length along it from the vertex.
    fn asked_setbacks(&mut self, v: VertexId, ring: &[Corner], blend_ring: &[usize]) -> R<Vec<Option<f64>>> {
        let mut out = vec![None; blend_ring.len()];
        for (b, &j) in blend_ring.iter().enumerate() {
            let ri = self.filleted(ring[j].cin).expect("blend");
            let e = self.rolled[ri].edge;
            let Some(&(_, _, d)) = self
                .setbacks
                .iter()
                .find(|s| s.0 == v && s.1 == Some(e))
                .or_else(|| self.setbacks.iter().find(|s| s.0 == v && s.1.is_none()))
            else {
                continue;
            };
            self.set_back.insert(v);
            let x = self.body.edge(e);
            let at_end = self.arrives_at_end(ring[j].cin);
            let (from, mid) = if at_end {
                (x.range.hi, x.range.mid())
            } else {
                (x.range.lo, x.range.mid())
            };
            let Some(t) = at_length(&x.curve, from, mid, d) else {
                return Err(FilletError::Overflow { edge: e });
            };
            let p = x.curve.eval(t);
            out[b] = Some(spine_closest(&self.rolled[ri].blend.spine, p));
        }
        Ok(out)
    }

    /// Where each blend stops (in corner input order) when one ball touches all
    /// three faces of a corner (equal radii, one kind), and the ball's centre.
    fn ball_setbacks(&self, v: VertexId, ring: &[Corner]) -> Option<(Vec<f64>, Point3)> {
        if ring.len() != 3 {
            return None;
        }
        let ri: Vec<usize> = ring.iter().map(|k| self.filleted(k.cin)).collect::<Option<_>>()?;
        if ri.iter().any(|&i| self.rolled[i].var.is_some()) {
            return None;
        }
        let r0 = &self.rolled[ri[0]];
        let (radius, convex) = (r0.blend.radius, r0.convex);
        if ri.iter().any(|&i| {
            (self.rolled[i].blend.radius - radius).abs() > tol::LINEAR || self.rolled[i].convex != convex
        }) {
            return None;
        }
        // each face's offset, from the blend arriving at its corner
        let offs: Vec<(Surface, f64)> = ring
            .iter()
            .zip(&ri)
            .map(|(k, &i)| {
                let b = &self.rolled[i].blend;
                if self.side(k.cin) == 0 {
                    (b.a.clone(), b.da)
                } else {
                    (b.b.clone(), b.db)
                }
            })
            .collect();
        let vp = self.body.vertex(v).point;
        let guess = ri
            .iter()
            .map(|&i| {
                let s = &self.rolled[i].blend.spine;
                s.eval(spine_closest(s, vp)).to_vec()
            })
            .fold(Vec3::ZERO, |a, b| a + b)
            / 3.0;
        let centre = three_offsets(&offs, Point3::new(guess.x, guess.y, guess.z))?;
        // corner input order is ring 0, 2, 1
        let mut ts = Vec::new();
        for j in [0, 2, 1] {
            let s = &self.rolled[ri[j]].blend.spine;
            let t = spine_closest(s, centre);
            if s.eval(t).distance(centre) > tol::LINEAR {
                return None;
            }
            ts.push(t);
        }
        Some((ts, centre))
    }

    /// A vertex in `pool` within LINEAR of `p`, or a new one.
    fn pooled(&mut self, pool: &mut Vec<(Point3, VertexId)>, p: Point3, name: Name) -> VertexId {
        if let Some(&(_, v)) = pool.iter().find(|q| q.0.distance(p) <= tol::LINEAR) {
            return v;
        }
        let v = self.body.add_vertex(p, name);
        pool.push((p, v));
        v
    }

    /// Several blends meet at `v`, perhaps with sharp edges between them. The
    /// corner solver says where each blend stops and how each face is trimmed,
    /// then patches the hole they leave; without one, a single ball closes a
    /// three-blend corner where it fits.
    fn corner(&mut self, v: VertexId, ring: &[Corner]) -> R<()> {
        let k = ring.len();
        let err = FilletError::Corner { vertex: v };
        let (input, blend_ring) = self.corner_input(v, ring);
        let asked = self.asked_setbacks(v, ring, &blend_ring)?;
        let (setbacks, trims, centre) = match crate::corner::setbacks(&input) {
            Some(mut s) => {
                for (b, t) in asked.iter().enumerate() {
                    if let Some(t) = *t {
                        s[b] = t;
                    }
                }
                let t = crate::corner::face_curves(&input, &s).map_err(|_| err.clone())?;
                (s, t, None)
            }
            None if asked.iter().any(|t| t.is_some()) => return Err(err),
            None => {
                let (s, c) = self.ball_setbacks(v, ring).ok_or(err.clone())?;
                (s, Vec::new(), Some(c))
            }
        };
        if setbacks.len() != input.blends.len() {
            return Err(err);
        }
        let ccw = |j: usize| (k - j) % k;
        let vname = self.body.vertex(v).name;
        let patch_name = self.root.with_str("corner").with_name(vname);
        let mut pool: Vec<(Point3, VertexId)> = Vec::new();
        // per ring corner j: where its blend stops (parameter, contact on face j, contact on face j - 1)
        let mut stops: Vec<Option<(f64, VertexId, VertexId)>> = vec![None; k];
        for (b, &j) in blend_ring.iter().enumerate() {
            let ri = self.filleted(ring[j].cin).expect("blend");
            let s_in = self.side(ring[j].cin);
            let t = setbacks[b];
            let (pa, pb) = (
                self.rolled[ri].springs[s_in].eval(t),
                self.rolled[ri].springs[1 - s_in].eval(t),
            );
            let fa = self.face_name(ring[j].face);
            let fb = self.face_name(ring[(j + k - 1) % k].face);
            let rn = self.rolled[ri].name;
            let va = self.pooled(&mut pool, pa, Name::NONE.with_set(&mut [fa, rn]).with_name(vname));
            let vb = self.pooled(&mut pool, pb, Name::NONE.with_set(&mut [fb, rn]).with_name(vname));
            stops[j] = Some((t, va, vb));
        }
        // sharp edges are cut where the face trims beside them end
        let any_blend = self.rolled[self.filleted(ring[blend_ring[0]].cin).expect("blend")].edge;
        let mut cuts: Vec<Option<VertexId>> = vec![None; k];
        for j in 0..k {
            if stops[j].is_some() {
                continue;
            }
            let e = self.body.edge(self.body.coedge(ring[j].cin).edge);
            let (curve, range) = (e.curve.clone(), e.range);
            let on_edge = |p: Point3| curve.eval(curve.closest(p, range)).distance(p) <= tol::LINEAR;
            let m = trims
                .iter()
                .filter(|t| t.face == ccw(j) || t.face == ccw((j + k - 1) % k))
                .flat_map(|t| [t.start, t.end])
                .find(|&p| on_edge(p) && p.distance(input.point) > tol::LINEAR)
                .ok_or(err.clone())?;
            // a smooth edge a blend rests beyond does not reach v: named for it too
            let mut name = Name::NONE.with_str("cut").with_name(vname);
            if e.start != v && e.end != v {
                name = name.with_name(e.name);
            }
            let vm = self.pooled(&mut pool, m, name);
            let g = self.body.coedge(ring[j].cin).edge;
            match self.resting_beyond(g) {
                Some(rs) => {
                    let gx = self.body.edge(g);
                    let t = gx.curve.closest(m, gx.range);
                    self.g_ends.entry(rs).or_default().push(GEnd::Cut(t, vm));
                }
                None => self.trim(ring[j].cin, m, vm, any_blend, None)?,
            }
            cuts[j] = Some(vm);
        }
        // face corners: from the arriving side's stop to the leaving side's, along the face's trim
        let mut face_edges: Vec<Option<(EdgeId, bool)>> = vec![None; k];
        for j in 0..k {
            let jn = (j + 1) % k;
            let a = stops[j].map_or(cuts[j], |s| Some(s.1)).ok_or(err.clone())?;
            let b = stops[jn].map_or(cuts[jn], |s| Some(s.2)).ok_or(err.clone())?;
            match trims.iter().find(|t| t.face == ccw(j)) {
                Some(t) => {
                    let (pa, pb) = (self.body.vertex(a).point, self.body.vertex(b).point);
                    let (ta, tb) = (t.curve.closest(pa, t.range), t.curve.closest(pb, t.range));
                    if t.curve.eval(ta).distance(pa) > tol::LINEAR
                        || t.curve.eval(tb).distance(pb) > tol::LINEAR
                    {
                        return Err(err);
                    }
                    let name = Name::NONE.with_set(&mut [self.face_name(ring[j].face), patch_name]);
                    let walk = self.edge_between(t.curve.clone(), ta, tb, a, b, name);
                    match self.rests(ring, j) {
                        // on a face rested on: from the cut smooth edge to the spring, joined
                        // into the face with the spring
                        Some(rs) => {
                            let (xg, xv) = if cuts[j].is_some() { (a, b) } else { (b, a) };
                            self.caps.entry(rs).or_default().push((walk.0, xg, xv));
                            self.touched.insert(ring[j].face);
                        }
                        None => {
                            self.paths.insert(ring[j].cin, vec![walk]);
                        }
                    }
                    face_edges[j] = Some(walk);
                }
                None if a == b => {}
                None => return Err(err),
            }
        }
        // blend sections, walked by the blend from its leaving side to its arriving side
        let mut sections: Vec<Option<(EdgeId, bool)>> = vec![None; k];
        for j in 0..k {
            let Some((t, va, vb)) = stops[j] else { continue };
            let ri = self.filleted(ring[j].cin).expect("blend");
            let s_in = self.side(ring[j].cin);
            let (circle, angle) = self.rolled[ri].blend.section(t).ok_or(err.clone())?;
            // the section runs from the foot on the blend's side 0
            let (v0, v1) = if s_in == 0 { (va, vb) } else { (vb, va) };
            let name = Name::NONE.with_set(&mut [self.rolled[ri].name, patch_name]);
            let e = self
                .body
                .add_edge(Curve::Circle(circle), Interval::new(0.0, angle), v0, v1, name);
            let walk = (e, v0 == vb);
            sections[j] = Some(walk);
            let mut end = SpringEnd {
                v: [v; 2],
                t: [t; 2],
                path: vec![walk],
            };
            end.v[s_in] = va;
            end.v[1 - s_in] = vb;
            let edge = self.rolled[ri].edge;
            self.ends.insert((edge, self.arrives_at_end(ring[j].cin)), end);
        }
        // the hole, walked with the patch on its left: each side the other way to its neighbour
        let mut hole: Vec<HoleSide> = Vec::new();
        let mut hole_edges: Vec<(EdgeId, bool)> = Vec::new();
        for j in std::iter::once(0).chain((1..k).rev()) {
            let jp = (j + k - 1) % k;
            let mut sides = Vec::new();
            if let Some((e, fwd)) = sections[j] {
                let ro = &self.rolled[self.filleted(ring[j].cin).expect("blend")];
                let blend = blend_ring.iter().position(|&x| x == j).expect("blend");
                // along a section a swept blend's normal is its ball's: the sphere answers it
                // without projecting onto the blend
                let neighbour = match (&ro.blend.surface, stops[j]) {
                    (Surface::Procedural(_), Some((t, _, _))) => {
                        let (c, r) = ro.blend.ball(t);
                        Surface::Sphere(Sphere {
                            frame: Frame::from_z(c, UnitVec3::Z),
                            radius: r,
                        })
                    }
                    _ => ro.blend.surface.clone(),
                };
                sides.push((e, !fwd, neighbour, ro.convex, SideKind::Section { blend }));
            }
            if let Some((e, fwd)) = face_edges[jp] {
                let face = self.body.face(ring[jp].face);
                let kind = SideKind::FaceCurve { face: ccw(jp) };
                sides.push((e, !fwd, face.surface.clone(), face.same_sense, kind));
            }
            for (e, fwd, neighbour, neighbour_same_sense, kind) in sides {
                let edge = self.body.edge(e);
                let (s, t) = (
                    self.body.vertex(edge.start).point,
                    self.body.vertex(edge.end).point,
                );
                hole.push(HoleSide {
                    curve: edge.curve.clone(),
                    range: edge.range,
                    forward: fwd,
                    start: if fwd { s } else { t },
                    end: if fwd { t } else { s },
                    neighbour,
                    neighbour_same_sense,
                    kind,
                });
                hole_edges.push((e, fwd));
            }
        }
        let patch = match centre {
            None => crate::corner::patch(&input, &hole)
                .ok_or(err.clone())?
                .map_err(|_| err.clone())?,
            Some(c) => ball_patch(&input, &hole, c).ok_or(err.clone())?,
        };
        let mut inner = Vec::with_capacity(patch.inner.len());
        for (n, (curve, range, a, b)) in patch.inner.iter().enumerate() {
            let name = patch_name.with_str("inner").with(n as u64);
            let va = self.pooled(&mut pool, *a, name.with_str("start"));
            let vb = self.pooled(&mut pool, *b, name.with_str("end"));
            inner.push(self.body.add_edge(curve.clone(), *range, va, vb, name));
        }
        for (n, pf) in patch.faces.into_iter().enumerate() {
            let uses = pf
                .uses
                .iter()
                .map(|u| match *u {
                    PatchUse::Side(i) => hole_edges[i],
                    PatchUse::Inner { edge, forward } => (inner[edge], forward),
                })
                .collect();
            let name = if n == 0 {
                patch_name
            } else {
                patch_name.with(n as u64)
            };
            self.patches.push((pf.surface, pf.same_sense, name, uses));
        }
        Ok(())
    }

    /// The spring edges of every blend, indexed like `rolled`.
    fn make_springs(&mut self) -> R<Vec<[Springs; 2]>> {
        let mut out = Vec::with_capacity(self.rolled.len());
        for i in 0..self.rolled.len() {
            let e = self.rolled[i].edge;
            let closed = self.body.edge(e).start == self.body.edge(e).end;
            let mut pair = Vec::with_capacity(2);
            for side in 0..2 {
                let (s, t) = (&self.ends[&(e, false)], &self.ends[&(e, true)]);
                let ro = &self.rolled[i];
                let curve = ro.springs[side].clone();
                // the spring runs with the spine: from the edge's start when aligned
                let ((v0, t0), (v1, t1)) = if ro.aligned {
                    ((s.v[side], s.t[side]), (t.v[side], t.t[side]))
                } else {
                    ((t.v[side], t.t[side]), (s.v[side], s.t[side]))
                };
                let t1 = match curve.domain().period {
                    Some(p) => {
                        let w = (t1 - t0).rem_euclid(p);
                        if closed && w * curve.deriv(t0).d1.length() <= tol::LINEAR {
                            t0 + p
                        } else {
                            t0 + w
                        }
                    }
                    None => t1,
                };
                if (!closed && curve.eval(t0).distance(curve.eval(t1)) <= tol::LINEAR) || t1 <= t0 {
                    return Err(FilletError::Overflow { edge: e });
                }
                let over = ro.over[side];
                let face = over.map_or(ro.faces[side], |o| o.0);
                let name = Name::NONE.with_set(&mut [ro.name, self.body.face(face).name]);
                // where the spring leaves its face and comes back, the ball rolls over what lies between
                let trips = if inside_face(&self.body, face, &curve, Interval::new(t0, t1)) {
                    Vec::new()
                } else if over.is_some() {
                    return Err(FilletError::Overflow { edge: e });
                } else {
                    self.excursions(i, side, Interval::new(t0, t1))?
                };
                let mut stops = vec![(v0, t0)];
                for x in &trips {
                    stops.push((x.v_out, x.t_out));
                    stops.push((x.v_in, x.t_in));
                }
                stops.push((v1, t1));
                let mut pieces = Vec::with_capacity(trips.len() + 1);
                for (k, w) in stops.chunks(2).enumerate() {
                    let ((va, ta), (vb, tb)) = (w[0], w[1]);
                    let range = Interval::new(ta, tb);
                    if tb <= ta || (!trips.is_empty() && !inside_face(&self.body, face, &curve, range)) {
                        return Err(FilletError::Overflow { edge: e });
                    }
                    let name = if k == 0 { name } else { name.with(k as u64) };
                    pieces.push(self.body.add_edge(curve.clone(), range, va, vb, name));
                }
                if !trips.is_empty() {
                    self.rechain.insert(face);
                }
                if over.is_some() {
                    // on the face rested on: the spring as e's coedge on the strip walked it, and
                    // the edges the ends cut over that face, from and back to the smooth edge
                    let ro = &self.rolled[i];
                    let mut uses = walk(&pieces, ro.forward[side] == ro.aligned);
                    let ends = |b: &Body, u: &(EdgeId, bool)| {
                        let x = b.edge(u.0);
                        if u.1 { (x.start, x.end) } else { (x.end, x.start) }
                    };
                    let (first, last) = (
                        ends(&self.body, &uses[0]).0,
                        ends(&self.body, &uses[uses.len() - 1]).1,
                    );
                    for &(ce, xg, xv) in self.caps.get(&(i, side)).into_iter().flatten() {
                        let from_g = self.body.edge(ce).start == xg;
                        if xv == first {
                            uses.insert(0, (ce, from_g));
                        } else if xv == last {
                            uses.push((ce, !from_g));
                        }
                    }
                    self.extra.entry(face).or_default().extend(uses);
                    self.rechain.insert(face);
                }
                pair.push(Springs {
                    pieces,
                    gaps: trips.into_iter().map(|x| x.gap).collect(),
                });
            }
            let second = pair.pop().expect("two sides");
            out.push([pair.pop().expect("two sides"), second]);
        }
        Ok(out)
    }

    /// Where spring `side` of blend `ri` leaves its face over `range` and comes back: each
    /// time, the cut of the blend across the faces between, those faces trimmed and what the
    /// cut passes wholly over dropped (as a run-out rolling over does).
    fn excursions(&mut self, ri: usize, side: usize, range: Interval) -> R<Vec<Trip>> {
        let edge = self.rolled[ri].edge;
        let err = FilletError::Overflow { edge };
        let home = self.rolled[ri].faces[side];
        let spring = self.rolled[ri].springs[side].clone();
        let slack = 1e-6 * range.width();
        // every crossing of the spring with an edge of its face
        let mut hits: Vec<(f64, CoedgeId, Point3)> = Vec::new();
        for &l in &self.body.face(home).loops {
            for c in self.body.lp(l).coedges() {
                let x = self.body.coedge(c).edge;
                if x == edge || self.dropped.contains(&c) {
                    continue;
                }
                let Some(across) = self.body.partner(c) else {
                    continue;
                };
                let surf = self.body.face(self.body.coedge_face(across)).surface.clone();
                let e = self.body.edge(x);
                let (curve, er) = (e.curve.clone(), e.range);
                if let CurveSurface::Hits(h) = curve_surface(&spring, range, &surf) {
                    for h in h {
                        let on = curve.eval(curve.closest(h.point, er)).distance(h.point) <= tol::LINEAR;
                        if on && h.t > range.lo + slack && h.t < range.hi - slack {
                            if self.which.contains_key(&x) {
                                return Err(err);
                            }
                            hits.push((h.t, c, h.point));
                        }
                    }
                }
            }
        }
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        hits.dedup_by(|a, b| (a.0 - b.0).abs() <= slack);
        if hits.is_empty() || hits.len() % 2 == 1 {
            return Err(err);
        }
        let ro = &self.rolled[ri];
        // the blend walks this spring with its parameter or against it
        let up = ro.forward[side] != ro.aligned;
        let mut trips = Vec::new();
        for w in hits.chunks(2) {
            let ((t_out, c_out, p_out), (t_in, c_in, p_in)) = (w[0], w[1]);
            let name = |b: &Builder, c: CoedgeId| {
                Name::NONE
                    .with_set(&mut [b.rolled[ri].name, b.body.edge(b.body.coedge(c).edge).name])
                    .with(t_out.to_bits())
            };
            let v_out = self.body.add_vertex(p_out, name(self, c_out));
            let v_in = self.body.add_vertex(p_in, name(self, c_in));
            // the faces walk the cut against the blend: from where the blend's walk ends
            let ((from_c, from), (to_c, to)) = if up {
                ((c_in, (p_in, v_in)), (c_out, (p_out, v_out)))
            } else {
                ((c_out, (p_out, v_out)), (c_in, (p_in, v_in)))
            };
            let cur = self.body.partner(from_c).ok_or(err.clone())?;
            self.trim(cur, from.0, from.1, edge, None)?;
            let (path, last) = self.walk_cut(ri, cur, from, home, &to)?;
            if last != to_c {
                return Err(err);
            }
            // on the home face, what lies from the last cut to the first is passed over
            let mut gone = Vec::new();
            let mut d = self.next_in_loop(to_c);
            while d != from_c {
                if gone.len() > 64 || self.which.contains_key(&self.body.coedge(d).edge) {
                    return Err(err);
                }
                gone.push(d);
                d = self.next_in_loop(d);
            }
            self.drop_all(&gone, edge)?;
            let mut gap: Vec<(EdgeId, bool)> = path.iter().rev().map(|&(e, f)| (e, !f)).collect();
            if !up {
                gap.reverse();
                for g in &mut gap {
                    g.1 = !g.1;
                }
            }
            trips.push(Trip {
                t_out,
                v_out,
                t_in,
                v_in,
                gap,
            });
        }
        Ok(trips)
    }

    /// The blend's cut walked across the faces from `entry`, on the edge of `cur` (which
    /// runs into what the fillet takes away), until it reaches `home` at `target` on the
    /// edge it crosses back: the edges the faces walk, and the coedge it ends on.
    fn walk_cut(
        &mut self,
        ri: usize,
        mut cur: CoedgeId,
        mut entry: (Point3, VertexId),
        home: FaceId,
        target: &(Point3, VertexId),
    ) -> R<(Vec<(EdgeId, bool)>, CoedgeId)> {
        let edge = self.rolled[ri].edge;
        let err = FilletError::Overflow { edge };
        let blend = self.rolled[ri].blend.surface.clone();
        let name = self.rolled[ri].name;
        let mut path = Vec::new();
        let mut gone = Vec::new();
        loop {
            let f = self.body.coedge_face(cur);
            if f == home || path.len() > 16 {
                return Err(err);
            }
            self.touched.insert(f);
            let mut d = self.next_in_loop(cur);
            let (exit, done) = loop {
                if d == cur || gone.len() > 64 {
                    return Err(err);
                }
                let g = self.body.coedge_face(self.body.partner(d).ok_or(err.clone())?);
                let e = self.body.edge(self.body.coedge(d).edge);
                let (curve, range) = (e.curve.clone(), e.range);
                let on = |p: Point3| curve.eval(curve.closest(p, range)).distance(p) <= tol::LINEAR;
                if g == home && on(target.0) {
                    break (target.0, true);
                }
                if g != home
                    && let Some((_, p)) = nearest_hit(&curve, range, &blend, entry.0)
                    && p.distance(entry.0) > tol::LINEAR
                {
                    break (p, false);
                }
                if self.which.contains_key(&self.body.coedge(d).edge) {
                    return Err(err);
                }
                gone.push(d);
                d = self.next_in_loop(d);
            };
            let xv = if done {
                target.1
            } else {
                let vn = Name::NONE
                    .with_set(&mut [name, self.body.edge(self.body.coedge(d).edge).name])
                    .with(path.len() as u64);
                self.body.add_vertex(exit, vn)
            };
            let (curve, ta, tb) =
                cross_curve(&self.rolled[ri].blend, &self.body.face(f).surface, entry.0, exit)
                    .ok_or(err.clone())?;
            let ename = Name::NONE
                .with_set(&mut [name, self.face_name(f)])
                .with(path.len() as u64)
                .with(ta.to_bits());
            let (eid, fwd) = self.edge_between(curve, ta, tb, entry.1, xv, ename);
            self.paths.insert(cur, vec![(eid, fwd)]);
            path.push((eid, fwd));
            cur = self.body.partner(d).ok_or(err.clone())?;
            self.trim(cur, exit, xv, edge, None)?;
            entry = (exit, xv);
            if done {
                self.drop_all(&gone, edge)?;
                return Ok((path, cur));
            }
        }
    }

    /// Drops coedges a cut passes wholly over: each with its partner, no blend's edge among them.
    fn drop_all(&mut self, gone: &[CoedgeId], edge: EdgeId) -> R<()> {
        for &d in gone {
            let e = self.body.coedge(d).edge;
            if self.which.contains_key(&e) {
                return Err(FilletError::Overflow { edge });
            }
            self.dropped.insert(d);
            self.touched.insert(self.body.coedge_face(d));
        }
        Ok(())
    }

    fn trim_edges(&mut self) -> R<()> {
        let trims: Vec<((EdgeId, bool), (VertexId, f64))> =
            self.trims.iter().map(|(k, v)| (*k, *v)).collect();
        for ((e, at_end), (v, t)) in trims {
            let edge = &mut self.body.edges[e.index()];
            if at_end {
                edge.end = v;
                edge.range.hi = t;
            } else {
                edge.start = v;
                edge.range.lo = t;
            }
            edge.bbox.clear();
        }
        for &(e, _) in self.trims.keys() {
            let edge = self.body.edge(e);
            // cut back from both ends to one point: two blends meet exactly there
            if self
                .body
                .vertex(edge.start)
                .point
                .distance(self.body.vertex(edge.end).point)
                <= tol::LINEAR
            {
                return Err(FilletError::Tangency { edge: e });
            }
            if edge.range.hi <= edge.range.lo {
                return Err(FilletError::Overflow { edge: e });
            }
        }
        Ok(())
    }

    fn rebuild_faces(&mut self, springs: &[[Springs; 2]]) {
        for r in &self.rolled {
            for &c in &self.body.edge(r.edge).coedges {
                self.touched.insert(self.body.coedge_face(c));
            }
        }
        let faces: Vec<FaceId> = self.touched.iter().copied().collect();
        for f in faces {
            let loops = self.body.face(f).loops.to_vec();
            let mut pending: Vec<(EdgeId, bool)> = Vec::new();
            for l in loops {
                let mut uses = Vec::new();
                for c in self.body.lp(l).coedges() {
                    if self.dropped.contains(&c) {
                        continue;
                    }
                    let co = self.body.coedge(c).clone();
                    match self.which.get(&co.edge) {
                        Some(&i) => {
                            let s = usize::from(self.rolled[i].coedges[0] != c);
                            // a strip the ball rolled past goes; its spring is on the face beyond
                            if self.rolled[i].over[s].is_none() {
                                uses.extend(walk(
                                    &springs[i][s].pieces,
                                    co.forward == self.rolled[i].aligned,
                                ));
                            }
                        }
                        None => uses.push((co.edge, co.forward)),
                    }
                    if let Some(p) = self.paths.get(&c) {
                        uses.extend(p.iter().copied());
                    }
                }
                self.body.remove_loop(f, l);
                if self.rechain.contains(&f) {
                    pending.extend(uses);
                } else {
                    self.body.add_loop(f, &uses);
                }
            }
            if let Some(x) = self.extra.get(&f) {
                pending.extend(x.iter().copied());
            }
            for lp in chain_loops(&self.body, &pending) {
                self.body.add_loop(f, &lp);
            }
            let face = &mut self.body.faces[f.index()];
            face.bbox.clear();
            face.query.clear();
        }
    }

    fn add_blends(&mut self, springs: &[[Springs; 2]]) {
        for (i, ro) in self.rolled.iter().enumerate() {
            // side 0 walks the edge from a to b; the blend walks its spring back, crosses at a,
            // walks side 1's spring forward and crosses back at b
            let a_is_end = !ro.forward[0];
            let at_a = &self.ends[&(ro.edge, a_is_end)];
            let at_b = &self.ends[&(ro.edge, !a_is_end)];
            let mut uses = springs[i][0].walk(ro.forward[0] != ro.aligned);
            uses.extend(at_a.path.iter().copied());
            uses.extend(springs[i][1].walk(ro.forward[1] != ro.aligned));
            uses.extend(at_b.path.iter().copied());
            let shell = self.body.face(ro.faces[0]).shell;
            let f = self
                .body
                .add_face(shell, ro.blend.surface.clone(), ro.convex, ro.name);
            self.body.add_loop(f, &uses);
        }
    }
}

/// A spring's edges in its parameter's order, and between each two the path the blend
/// takes (walking the spring with its parameter) where the spring was off its face.
struct Springs {
    pieces: Vec<EdgeId>,
    gaps: Vec<Vec<(EdgeId, bool)>>,
}

impl Springs {
    /// The blend's walk along the spring, with its parameter (`up`) or against it.
    fn walk(&self, up: bool) -> Vec<(EdgeId, bool)> {
        let mut out = Vec::new();
        for (k, &p) in self.pieces.iter().enumerate() {
            out.push((p, true));
            if let Some(g) = self.gaps.get(k) {
                out.extend(g.iter().copied());
            }
        }
        if !up {
            out.reverse();
            for u in &mut out {
                u.1 = !u.1;
            }
        }
        out
    }
}

/// Where a spring left its face and came back, and the blend's path between.
struct Trip {
    t_out: f64,
    v_out: VertexId,
    t_in: f64,
    v_in: VertexId,
    gap: Vec<(EdgeId, bool)>,
}

/// Edges laid end to end along their curve, walked with it or back.
fn walk(edges: &[EdgeId], forward: bool) -> Vec<(EdgeId, bool)> {
    if forward {
        edges.iter().map(|&e| (e, true)).collect()
    } else {
        edges.iter().rev().map(|&e| (e, false)).collect()
    }
}

/// Uses joined into loops end to start, each next taken in the order given from where the
/// last one was.
pub(crate) fn chain_loops(body: &Body, uses: &[(EdgeId, bool)]) -> Vec<Vec<(EdgeId, bool)>> {
    let ends = |&(e, f): &(EdgeId, bool)| {
        let x = body.edge(e);
        if f { (x.start, x.end) } else { (x.end, x.start) }
    };
    let n = uses.len();
    let mut used = vec![false; n];
    let mut out = Vec::new();
    while let Some(first) = (0..n).find(|&i| !used[i]) {
        used[first] = true;
        let mut lp = vec![uses[first]];
        let (start, mut at) = ends(&uses[first]);
        let mut i = first;
        while at != start {
            let Some(next) = (1..n)
                .map(|k| (i + k) % n)
                .find(|&j| !used[j] && ends(&uses[j]).0 == at)
            else {
                break;
            };
            used[next] = true;
            lp.push(uses[next]);
            at = ends(&uses[next]).1;
            i = next;
        }
        out.push(lp);
    }
    out
}

/// A parameter step, the short way round when the curve is periodic.
fn along(d: f64, period: Option<f64>) -> f64 {
    match period {
        Some(p) => {
            let w = d.rem_euclid(p);
            if w > 0.5 * p { w - p } else { w }
        }
        None => d,
    }
}

/// Whether the spring stays on its face (sampled strictly between its ends,
/// against the face as it was): a ball running off the face overflows.
pub(crate) fn inside_face(body: &Body, f: FaceId, c: &Curve, range: Interval) -> bool {
    let loops = forge_topo::uv::face_uv_loops(body, f);
    let surface = &body.face(f).surface;
    let d = surface.domain();
    let shifts = |p: Option<f64>| match p {
        Some(p) => vec![0.0, -p, p],
        None => vec![0.0],
    };
    let n = 16;
    (1..n).all(|i| {
        let uv = surface.project(c.eval(range.lo + range.width() * i as f64 / n as f64));
        shifts(d.u.period).iter().any(|&du| {
            shifts(d.v.period).iter().any(|&dv| {
                forge_topo::uv::point_in_loops(forge_math::Point2::new(uv.x + du, uv.y + dv), &loops)
            })
        })
    })
}

/// Where surfaces `a` and `b` meet, from `p` to `q` (one of them a plane): the curve and the
/// parameters of p and q, q past p (the short way round a closed curve).
fn meet(a: &Surface, b: &Surface, p: Point3, q: Point3) -> Option<(Curve, f64, f64)> {
    let curves = match (a, b) {
        (Surface::Plane(pl), s) | (s, Surface::Plane(pl)) => {
            match forge_geom::intersect::plane_surface(pl, s) {
                forge_geom::intersect::SurfaceSurface::Curves(cs) => cs,
                _ => return None,
            }
        }
        _ => return None,
    };
    for (c, _) in curves {
        if let Curve::Line(_) = c {
            let l = forge_geom::Line::through(p, q)?;
            return Some((Curve::Line(l), 0.0, p.distance(q)));
        }
        // a circle runs the short way from p to q: turned over where it would go the long way
        let c = match c {
            Curve::Circle(k) => {
                let turn = Interval::new(0.0, dmath::TAU);
                let (tp, tq) = (k.closest(p, turn), k.closest(q, turn));
                if (tq - tp).rem_euclid(dmath::TAU) > dmath::PI {
                    let z = (-k.frame.z().get()).normalize()?;
                    let f = Frame::from_zx(k.frame.origin, z, k.frame.x().get())?;
                    Curve::Circle(forge_geom::Circle { frame: f, ..k })
                } else {
                    Curve::Circle(k)
                }
            }
            Curve::Ellipse(k) => {
                let turn = Interval::new(0.0, dmath::TAU);
                let (tp, tq) = (k.closest(p, turn), k.closest(q, turn));
                if (tq - tp).rem_euclid(dmath::TAU) > dmath::PI {
                    let z = (-k.frame.z().get()).normalize()?;
                    let f = Frame::from_zx(k.frame.origin, z, k.frame.x().get())?;
                    Curve::Ellipse(forge_geom::Ellipse { frame: f, ..k })
                } else {
                    Curve::Ellipse(k)
                }
            }
            c => c,
        };
        let range = c.full_range(1e6);
        let (tp, tq) = (c.closest(p, range), c.closest(q, range));
        if c.eval(tp).distance(p) > tol::LINEAR || c.eval(tq).distance(q) > tol::LINEAR {
            continue;
        }
        let tq = match c.domain().period {
            Some(per) => tp + (tq - tp).rem_euclid(per),
            None if tq > tp => tq,
            None => continue,
        };
        return Some((c, tp, tq));
    }
    None
}

/// A parameter range of `c` around `t` reaching about `reach` along it.
fn search_window(c: &Curve, t: f64, reach: f64) -> Interval {
    let d = c.domain();
    let speed = c.deriv(t).d1.length().max(1e-12);
    let w = reach / speed;
    if let Some(p) = d.period {
        // round a closed curve too only as far as reach (a spring far round a closed spine
        // may not be a curve there at all), at most half a turn either way
        let w = w.min(0.5 * p);
        return Interval::new(t - w, t + w);
    }
    Interval::new((t - w).max(d.lo), (t + w).min(d.hi))
}

/// The point where `c` (over `range`) meets `s` nearest `near`, with its parameter.
pub(crate) fn nearest_hit(c: &Curve, range: Interval, s: &Surface, near: Point3) -> Option<(f64, Point3)> {
    match curve_surface(c, range, s) {
        CurveSurface::Hits(h) => h
            .into_iter()
            .min_by(|a, b| a.point.distance(near).total_cmp(&b.point.distance(near)))
            .map(|h| (h.t, h.point)),
        _ => None,
    }
}

/// Whether `p` (on the blend's surface) lies on the arc the ball spans between its contacts.
fn on_contact_arc(blend: &Blend, p: Point3) -> bool {
    let t = spine_closest(&blend.spine, p);
    let Some((circle, angle)) = blend.section(t) else {
        return false;
    };
    let l = circle.frame.point_to_local(p);
    let phi = dmath::atan2(l.y, l.x);
    let slack = tol::LINEAR / blend.radius;
    phi >= -slack && phi <= angle + slack
}

/// The curve where the blend meets surface `s` between `a` and `b` (both on
/// it), with the parameters of a and b; across a period, the arc whose
/// middle lies on the blend's contact arc.
fn cross_curve(blend: &Blend, s: &Surface, a: Point3, b: Point3) -> Option<(Curve, f64, f64)> {
    // a variable blend's cut is solved along its arc, not traced through its distance
    if let Some(cut) = blend.var.as_ref().and_then(|v| v.cut(s, a, b)) {
        return Some(cut);
    }
    let region = Box3::from_points([a, b]).grow(2.0 * blend.radius + tol::LINEAR);
    let curves = closed_form(&blend.surface, s).or_else(|| {
        forge_geom::intcurve::trace(&blend.surface, s, region).map(|cs| {
            cs.into_iter()
                .map(|c| Curve::Intersection(std::sync::Arc::new(c)))
                .collect()
        })
    })?;
    for c in curves {
        let full = c.full_range(1e6);
        let (ta, tb) = (c.closest(a, full), c.closest(b, full));
        if c.eval(ta).distance(a) > tol::LINEAR || c.eval(tb).distance(b) > tol::LINEAR {
            continue;
        }
        match c.domain().period {
            Some(p) => {
                let up = (tb - ta).rem_euclid(p);
                // a to b going up, or going down
                let mid = c.eval(ta + 0.5 * up);
                if on_contact_arc(blend, mid) {
                    return Some((c, ta, ta + up));
                }
                return Some((c, ta, ta - (p - up)));
            }
            None => return Some((c, ta, tb)),
        }
    }
    None
}

/// One blend arriving at a corner.
#[derive(Clone, Debug)]
pub struct CornerBlend {
    pub surface: Surface,
    pub same_sense: bool,
    pub radius: f64,
    pub convex: bool,
    /// The ball-centre curve.
    pub spine: Curve,
    /// +1 when the spine parameter grows going away from the vertex, else -1.
    pub out: f64,
    /// Where the blend would stop by itself (the section through the vertex).
    pub setback: f64,
    /// The ball's centre halfway along the edge's tangent chain: a stop goes no farther, so
    /// the stops at its two ends never cross. None for a closed chain.
    pub middle: Option<Point3>,
}

/// What lies between two neighbouring faces of a corner.
#[derive(Clone, Debug)]
pub enum CornerEdge {
    /// A blend (index into `CornerInput::blends`).
    Blend(usize),
    /// An edge left sharp; `out` is +1 when its parameter grows away from the vertex.
    Sharp { curve: Curve, out: f64 },
}

/// A vertex where several blends meet, as the corner solver sees it.
#[derive(Clone, Debug)]
pub struct CornerInput {
    pub vertex: VertexId,
    pub point: Point3,
    pub blends: Vec<CornerBlend>,
    /// Counter-clockwise seen from outside.
    pub faces: Vec<FaceId>,
    /// Parallel to `faces`: (surface, same_sense).
    pub face_surfaces: Vec<(Surface, bool)>,
    /// `edges[i]` lies between `faces[i]` and `faces[i + 1]`.
    pub edges: Vec<CornerEdge>,
}

/// Where a face ends at a corner: from one neighbour's stop to the next's.
#[derive(Clone, Debug)]
pub struct FaceTrim {
    /// Index into `CornerInput::faces`.
    pub face: usize,
    pub curve: Curve,
    pub range: Interval,
    pub start: Point3,
    pub end: Point3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SideKind {
    /// A blend's cross-section at its setback.
    Section { blend: usize },
    /// A face's trim (index into `CornerInput::faces`).
    FaceCurve { face: usize },
    /// A trimmed unfilleted edge.
    Edge { edge: EdgeId },
}

/// One side of a corner's hole, in order round it, walked with the patch on
/// its left seen from outside (the other way to its neighbour).
#[derive(Clone, Debug)]
pub struct HoleSide {
    pub curve: Curve,
    pub range: Interval,
    /// The patch walks the curve with its parameter.
    pub forward: bool,
    pub start: Point3,
    pub end: Point3,
    /// G1: along the side the patch's outward normal is the neighbour's.
    pub neighbour: Surface,
    pub neighbour_same_sense: bool,
    pub kind: SideKind,
}

/// The direction square to the side, tangent to the neighbour, pointing into the hole at `t`.
pub fn cross_tangent(side: &HoleSide, t: f64) -> Vec3 {
    let d = side.curve.deriv(t);
    let walk = if side.forward { d.d1 } else { -d.d1 };
    let n = side
        .neighbour
        .normal(side.neighbour.project(d.p))
        .map_or(Vec3::ZERO, |n| {
            if side.neighbour_same_sense {
                n.get()
            } else {
                -n.get()
            }
        });
    n.cross(walk).normalize().map_or(Vec3::ZERO, |u| u.get())
}

/// One face of a corner patch.
#[derive(Clone, Debug)]
pub struct PatchFace {
    pub surface: Surface,
    pub same_sense: bool,
    pub uses: Vec<PatchUse>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PatchUse {
    /// Hole side i, walked as the hole lists it.
    Side(usize),
    /// An edge of the patch's own (index into `CornerPatch::inner`).
    Inner { edge: usize, forward: bool },
}

/// The faces closing a corner's hole and the edges between them.
#[derive(Clone, Debug)]
pub struct CornerPatch {
    /// (curve, range, start point, end point).
    pub inner: Vec<(Curve, Interval, Point3, Point3)>,
    pub faces: Vec<PatchFace>,
}

/// The ball centred at `centre` closing a hole made only of its own sections.
fn ball_patch(input: &CornerInput, hole: &[HoleSide], centre: Point3) -> Option<CornerPatch> {
    let b = input.blends.first()?;
    // poles a quarter turn from the patch's middle, which faces the old vertex
    let mid = (input.point - centre).normalize()?;
    let frame = Frame::from_zx(centre, mid.any_perpendicular(), mid.get())?;
    Some(CornerPatch {
        inner: Vec::new(),
        faces: vec![PatchFace {
            surface: Surface::Sphere(Sphere {
                frame,
                radius: b.radius,
            }),
            same_sense: b.convex,
            uses: (0..hole.len()).map(PatchUse::Side).collect(),
        }],
    })
}

/// The parameter near `t` where curve `a` crosses curve `b` (over `range`),
/// both lying on one surface: alternate nearest points until they meet.
fn crossing(a: &Curve, mut t: f64, b: &Curve, range: Interval) -> Option<f64> {
    let window = search_window(a, t, 1e3);
    for _ in 0..100 {
        let p = a.eval(t);
        let q = b.eval(b.closest(p, range));
        if p.distance(q) <= 1e-3 * tol::LINEAR {
            return Some(t);
        }
        t = a.closest(q, window);
    }
    let p = a.eval(t);
    (b.eval(b.closest(p, range)).distance(p) <= tol::LINEAR).then_some(t)
}

/// The point on all three offset surfaces, by Newton from `p`.
fn three_offsets(offs: &[(Surface, f64)], mut p: Point3) -> Option<Point3> {
    for _ in 0..50 {
        let g: Vec<f64> = offs.iter().map(|(s, d)| s.implicit(p) - d).collect();
        if g.iter().all(|x| x.abs() <= 1e-3 * tol::LINEAR) {
            return Some(p);
        }
        let n: Vec<Vec3> = offs.iter().map(|(s, _)| s.gradient(p)).collect();
        let det = n[0].dot(n[1].cross(n[2]));
        if det.abs() <= 1e-12 {
            return None;
        }
        // Cramer: dp = -J^-1 g with J's rows the gradients
        let dp = (n[1].cross(n[2]) * g[0] + n[2].cross(n[0]) * g[1] + n[0].cross(n[1]) * g[2]) / det;
        p = p - dp;
    }
    let ok = offs.iter().all(|(s, d)| (s.implicit(p) - d).abs() <= tol::LINEAR);
    ok.then_some(p)
}

/// The parameter `d` along `c` from `from` toward `to` (arc length), or None past `to`.
fn at_length(c: &Curve, from: f64, to: f64, d: f64) -> Option<f64> {
    let sign = if to < from { -1.0 } else { 1.0 };
    let speed = match c {
        Curve::Line(_) => Some(1.0),
        Curve::Circle(k) => Some(k.radius),
        _ => None,
    };
    if d.is_nan() || d <= 0.0 {
        return None;
    }
    if let Some(v) = speed {
        return ((to - from).abs() * v >= d).then_some(from + sign * d / v);
    }
    if arc_length(c, from, to) < d {
        return None;
    }
    // safeguarded Newton on the length from `from`
    let (mut lo, mut hi) = (from, to);
    let mut t = from + (to - from) * 0.5;
    for _ in 0..60 {
        let f = arc_length(c, from, t) - d;
        if f.abs() <= 1e-13 * d {
            break;
        }
        if f < 0.0 {
            lo = t;
        } else {
            hi = t;
        }
        let v = c.deriv(t).d1.length();
        let n = t - sign * f / v;
        t = if v > 0.0 && (n - lo) * (n - hi) < 0.0 {
            n
        } else {
            0.5 * (lo + hi)
        };
    }
    Some(t)
}

/// The length of `c` between parameters `a` and `b`: 8-point Gauss-Legendre on panels doubled
/// until two passes agree.
fn arc_length(c: &Curve, a: f64, b: f64) -> f64 {
    match c {
        Curve::Line(_) => return (b - a).abs(),
        Curve::Circle(k) => return k.radius * (b - a).abs(),
        _ => {}
    }
    const X: [f64; 4] = [
        0.1834346424956498,
        0.525532409916329,
        0.7966664774136267,
        0.9602898564975363,
    ];
    const W: [f64; 4] = [
        0.362683783378362,
        0.3137066458778873,
        0.2223810344533745,
        0.1012285362903763,
    ];
    let pass = |n: usize| {
        let h = (b - a) / n as f64;
        let mut s = 0.0;
        for i in 0..n {
            let m = a + h * (i as f64 + 0.5);
            for (x, w) in X.iter().zip(W) {
                let o = 0.5 * h * x;
                s += w * (c.deriv(m - o).d1.length() + c.deriv(m + o).d1.length());
            }
        }
        0.5 * h.abs() * s
    };
    let mut n = 2;
    let mut l = pass(n);
    while n < 512 {
        n *= 2;
        let next = pass(n);
        let done = (next - l).abs() <= 1e-13 * next.max(tol::LINEAR);
        l = next;
        if done {
            break;
        }
    }
    l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_along_curves() {
        let e = Curve::Ellipse(forge_geom::curve::Ellipse {
            frame: Frame::WORLD,
            major: 2.0,
            minor: 1.0,
        });
        // a quarter of the ellipse: 2 E(sqrt(3)/2)
        let q = 2.422112055136919;
        assert!((arc_length(&e, 0.0, dmath::FRAC_PI_2) - q).abs() < 1e-12);
        let t = at_length(&e, dmath::FRAC_PI_2, 0.0, q).expect("on the edge");
        assert!(t.abs() < 1e-9, "{t}");
        let t = at_length(&e, 0.0, 1.0, 0.7).expect("on the edge");
        assert!((arc_length(&e, 0.0, t) - 0.7).abs() < 1e-12);
        assert!(at_length(&e, 0.0, 1.0, 5.0).is_none());
    }

    fn law(
        members: &[(u32, bool)],
        closed: bool,
        ranges: &[(f64, f64)],
        picks: &[(u32, Radius)],
    ) -> RadiusLaw {
        let chain = Chain {
            members: members.iter().map(|&(e, f)| (EdgeId(e), f)).collect(),
            closed,
        };
        let ranges: Vec<Interval> = ranges.iter().map(|&(a, b)| Interval::new(a, b)).collect();
        let whole = Interval::new(ranges[0].lo, ranges[ranges.len() - 1].hi);
        let picks: BTreeMap<EdgeId, &Radius> = picks.iter().map(|(e, r)| (EdgeId(*e), r)).collect();
        chain_law(&chain, &ranges, whole, &picks).expect("a law")
    }

    #[test]
    fn constant_picks_hold_along_their_edges() {
        // constant, unpicked, constant: the change happens on the unpicked edge only
        let l = law(
            &[(0, true), (1, true), (2, false)],
            false,
            &[(0.0, 10.0), (10.0, 14.0), (14.0, 24.0)],
            &[(0, Radius::Constant(1.0)), (2, Radius::Constant(2.0))],
        );
        for i in 0..=40 {
            let s = 24.0 * i as f64 / 40.0;
            let r = l.eval(s);
            match s {
                s if s <= 10.0 => assert!((r - 1.0).abs() < 1e-12, "{s} {r}"),
                s if s >= 14.0 => assert!((r - 2.0).abs() < 1e-12, "{s} {r}"),
                _ => assert!((1.0..=2.0).contains(&r) && l.slope(s) >= 0.0, "{s} {r}"),
            }
        }
        assert!(l.slope(10.0).abs() < 1e-12 && l.slope(14.0).abs() < 1e-12);
        // two constants meeting: a zone centred on the joint, flat outside it
        let l = law(
            &[(0, true), (1, true)],
            false,
            &[(0.0, 10.0), (10.0, 14.0)],
            &[(0, Radius::Constant(1.0)), (1, Radius::Constant(1.5))],
        );
        assert!((l.eval(7.9) - 1.0).abs() < 1e-12 && (l.eval(12.1) - 1.5).abs() < 1e-12);
        assert!((l.eval(10.0) - 1.25).abs() < 1e-12);
        // a linear edge between constants of other radii reaches them flat at its ends
        let l = law(
            &[(0, true), (1, true), (2, true), (3, true)],
            true,
            &[(0.0, 10.0), (10.0, 20.0), (20.0, 30.0), (30.0, 40.0)],
            &[
                (0, Radius::Constant(1.0)),
                (1, Radius::Linear(1.0, 2.0)),
                (2, Radius::Constant(2.0)),
            ],
        );
        for s in [0.0, 5.0, 9.99, 40.0, 45.0] {
            assert!((l.eval(s) - 1.0).abs() < 1e-12, "{s} {}", l.eval(s));
        }
        for s in [20.0, 25.0, 29.99, 65.0] {
            assert!((l.eval(s) - 2.0).abs() < 1e-12, "{s} {}", l.eval(s));
        }
        assert!((l.eval(15.0) - 1.5).abs() < 1e-9);
    }
}

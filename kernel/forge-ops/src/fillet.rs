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
    /// Two blends beside a sharp edge at this vertex should cut each other, and their cut cannot
    /// be followed: a patch there would be a shape nobody asked for.
    Mitre { vertex: VertexId },
    /// The blend of this edge ends where its tangent chain runs on unpicked, and has nothing there
    /// to run out into: the chain's next edge needs picking too.
    FreeEnd { edge: EdgeId },
    /// This edge's blend and another cross on the face between them, and the one blend of the faces
    /// beyond, which should take that face whole, does not stand.
    Crossed { edge: EdgeId },
    /// An edge is listed twice.
    Duplicate { edge: EdgeId },
    /// A ring's round whose ball is wider than the circle its centre rolls round; `max` is the
    /// largest radius that rounds it.
    TooLarge { edge: EdgeId, max: f64 },
    /// (Inside `fillet_with` only.) A spring leaves its face over this edge and comes back over
    /// it: the edge's middle goes, so it is split at `at` and the fillet tried again.
    SplitFirst { edge: EdgeId, at: f64 },
    /// The round was built and the result fails `forge_topo::check` here: no body rather than a
    /// wrong one. Mass and every later boolean on such a body are not to be trusted.
    Invalid(Vec<forge_topo::Problem>),
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
    /// The name from the edge as a whole: what the blend makes at a vertex is named from this
    /// and the vertex, so a split elsewhere on the edge renames nothing there.
    base: Name,
    var: Option<VarEdge>,
    /// Per side, where the ball rests not on the edge's face but on one tangent to it beyond a
    /// smooth edge of it: that face and the smooth edge. The strip of the edge's face between is
    /// swallowed.
    over: Over,
    /// Where on its edge a spring crosses a smooth edge of its face: the ball moves onto the face
    /// beyond there, so the edge is split there and each piece rolled by itself.
    split: Option<f64>,
    /// The side whose spring runs on a far edge of its face (`cliff`).
    cliff: Option<usize>,
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
    /// Edges a tangent chain does not run on along: the blend ends where it reaches one.
    pub stops: Vec<EdgeId>,
    /// Only the edges given: no tangent chain runs on from them at all.
    pub only: bool,
    /// Where two blends' springs cross on the face between them, one blend of the faces beyond
    /// takes that face whole. Asked only after the two have refused to stand side by side.
    pub across: bool,
    /// Springs are tested against seams of their faces and may cross them. Asked only after a refusal:
    /// it costs a walk along every spring beside a seam, and few reach one.
    pub seams: bool,
}

/// The vertex each varying law of `edges` runs from. A run is the edges one varying radius was
/// given to, side by side in `edges`; it is walked from its start, each edge from the end it is
/// reached by. An edge given one of its ends starts there; the walk starts at the vertex given
/// for one of the run's edges, else the run's free end lowest in x, then y, then z (a lone edge's
/// lower end). A chain apart from the rest starts likewise, or is refused where a start was given.
/// Geometric, never by name or the order given: renaming cannot turn a law round.
pub fn law_starts(
    body: &Body,
    edges: &[FilletEdge],
    given: &[(EdgeId, VertexId)],
) -> R<BTreeMap<EdgeId, VertexId>> {
    let ends = |e: EdgeId| (body.edge(e).start, body.edge(e).end);
    // lower in x, then y, then z, what a tolerance cannot tell apart counting as level
    let below = |p: Point3, q: Point3| {
        [(p.x, q.x), (p.y, q.y), (p.z, q.z)]
            .into_iter()
            .find(|(u, w)| (u - w).abs() > tol::LINEAR)
            .map_or(std::cmp::Ordering::Equal, |(u, w)| u.total_cmp(&w))
    };
    let lowest = |e: EdgeId| {
        let (a, b) = ends(e);
        if below(body.vertex(a).point, body.vertex(b).point).is_le() {
            a
        } else {
            b
        }
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
        // the run's free ends: used by one edge of it, whatever order it was given in
        let free_end = |left: &[EdgeId]| {
            let uses = |v: VertexId| left.iter().filter(|&&e| ends(e).0 == v || ends(e).1 == v).count();
            left.iter()
                .flat_map(|&e| [ends(e).0, ends(e).1])
                .filter(|&v| uses(v) == 1)
                .min_by(|&v, &w| below(body.vertex(v).point, body.vertex(w).point))
        };
        let start = from.or_else(|| free_end(&run));
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
        loop {
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
            let Some(&e) = left.first() else { break };
            if from.is_some() {
                return Err(FilletError::BadRadius { edge: e });
            }
            // a chain apart from the rest, or a ring, whose free end or lowest vertex starts it
            reached.push(free_end(&left).unwrap_or_else(|| lowest(e)));
        }
    }
    Ok(out)
}

/// Whether edge `e` is convex: true, or concave: false (None where it bounds no two faces).
pub fn fillet_kind(body: &Body, e: EdgeId) -> Option<bool> {
    let x = body.edge(e);
    local(body, e, 1.0, x.range.mid()).ok().map(|l| l.convex)
}

/// Rounds `edges` of `body`, each with its own radius, in one operation.
/// `feature` is the history id the new faces, edges and vertices are named from.
pub fn fillet(body: &Body, edges: &[FilletEdge], feature: u64) -> R<Body> {
    let done = fillet_with(body, edges, &FilletOptions::default(), feature)?;
    let problems = forge_topo::check_against(&done, body);
    // pieces split apart can share a name until they are joined, as a boolean's can
    if problems
        .iter()
        .all(|p| matches!(p, forge_topo::Problem::DuplicateName { .. }))
    {
        return Ok(done);
    }
    // a spring over a seam between samples comes out as loops that cross
    let seams = FilletOptions {
        seams: true,
        ..FilletOptions::default()
    };
    if let Ok(b) = fillet_with(body, edges, &seams, feature)
        && forge_topo::check_against(&b, body)
            .iter()
            .all(|p| matches!(p, forge_topo::Problem::DuplicateName { .. }))
    {
        return Ok(b);
    }
    Err(FilletError::Invalid(problems))
}

/// `fillet` with laws started at given vertices and corners set back by given distances.
pub fn fillet_with(body: &Body, edges: &[FilletEdge], opts: &FilletOptions, feature: u64) -> R<Body> {
    let got = fillet_picked(body, edges, opts, feature);
    if matches!(got, Err(FilletError::Overflow { .. })) && !opts.across {
        let across = FilletOptions {
            across: true,
            ..opts.clone()
        };
        match fillet_picked(body, edges, &across, feature) {
            Ok(b) if forge_topo::check(&b).is_empty() => return Ok(b),
            Ok(_) => {
                return Err(FilletError::Crossed {
                    edge: edges.first().map_or(EdgeId(0), |f| f.edge),
                });
            }
            Err(e @ FilletError::Crossed { .. }) => return Err(e),
            Err(_) => {}
        }
        if !opts.seams {
            let seams = FilletOptions {
                seams: true,
                ..opts.clone()
            };
            if let Ok(b) = fillet_picked(body, edges, &seams, feature)
                && forge_topo::check(&b).is_empty()
            {
                return Ok(b);
            }
        }
    }
    if let Err(FilletError::Overflow { edge } | FilletError::NoBlend { edge }) = got
        && opts.only
    {
        // what stops it is the end of the pick, if the whole chain builds
        let chain = FilletOptions {
            only: false,
            ..opts.clone()
        };
        if fillet_picked(body, edges, &chain, feature).is_ok() {
            return Err(FilletError::FreeEnd { edge });
        }
    }
    got
}

fn fillet_picked(body: &Body, edges: &[FilletEdge], opts: &FilletOptions, feature: u64) -> R<Body> {
    if opts.setbacks.is_empty()
        && let Some((first, then)) = minority_first(body, edges)
    {
        let done = fillet_with(body, &first, opts, feature)?;
        let named = |f: &FilletEdge| {
            let name = body.edge(f.edge).name;
            let edge = done.edge_ids().find(|&e| done.edge(e).name == name)?;
            Some(FilletEdge {
                edge,
                radius: f.radius.clone(),
            })
        };
        let then: Vec<FilletEdge> = then.iter().filter_map(named).collect();
        // the rounds run on over the first ones' new faces, not along edges nobody picked
        let picked: BTreeSet<Name> = edges.iter().map(|f| body.edge(f.edge).name).collect();
        let was: BTreeSet<Name> = body.edge_ids().map(|e| body.edge(e).name).collect();
        let stops = done
            .edge_ids()
            .filter(|&e| {
                let n = done.edge(e).name;
                was.contains(&n) && !picked.contains(&n)
            })
            .collect();
        let rest = FilletOptions {
            stops,
            only: opts.only,
            seams: opts.seams,
            ..FilletOptions::default()
        };
        return fillet_with(&done, &then, &rest, feature);
    }
    let mut work: Option<Body> = None;
    let mut first = None;
    for _ in 0..4 {
        let on = work.as_ref().unwrap_or(body);
        match fillet_on(on, edges, opts, feature) {
            // the edge split in two, ids before it standing, its middle lies between the pieces
            Err(FilletError::SplitFirst { edge, at }) => {
                let mut b = if on.dead > 0 { on.compact() } else { on.clone() };
                split_edge(&mut b, edge, at);
                work = Some(b);
            }
            Err(e) => {
                first = Some(first.unwrap_or(e));
                break;
            }
            ok => return ok,
        }
    }
    let e = first.unwrap_or(FilletError::Overflow {
        edge: edges.first().map_or(EdgeId(0), |f| f.edge),
    });
    // a spring crossing a round face's seam meets a line that is no boundary at all: with the seam
    // moved round to where nothing is, the blend never sees it
    let seams = matches!(e, FilletError::Overflow { .. } | FilletError::Corner { .. })
        .then(|| reseamed(body, edges))
        .flatten();
    if let Some(b) = &seams
        && let Ok(done) = fillet_on(b, edges, opts, feature)
    {
        return Ok(done);
    }
    // an edge a vertex splits for no reason (a boolean's, with nothing else there) stops a spring
    // that crosses its other piece: whole, the corner sees the edge the spring crosses
    if matches!(
        e,
        FilletError::Corner { .. } | FilletError::Mitre { .. } | FilletError::Overflow { .. }
    ) && opts.setbacks.is_empty()
        && opts.starts.is_empty()
        && let Some(b) = joined_pieces(seams.as_ref().unwrap_or(body), edges)
        && let Ok(done) = fillet_on(&b, edges, opts, feature)
        && forge_topo::check(&done).is_empty()
        && forge_topo::mass::mass(&done).is_ok()
    {
        return Ok(done);
    }
    Err(e)
}

/// The body with each unpicked edge whole where only its two pieces meet at a vertex, one curve
/// and the same two faces either side. None if there is no such vertex.
fn joined_pieces(body: &Body, edges: &[FilletEdge]) -> Option<Body> {
    let mut b = if body.dead > 0 {
        body.compact()
    } else {
        body.clone()
    };
    let picked: BTreeSet<EdgeId> = edges.iter().map(|f| f.edge).collect();
    let at = edges_at(&b);
    let mut moved = false;
    // an edge gone into another: the vertex at its far end now has that one
    let mut into: BTreeMap<EdgeId, EdgeId> = BTreeMap::new();
    for pair in at.values() {
        let &[p, q] = &pair[..] else { continue };
        let now = |mut e: EdgeId| {
            while let Some(&n) = into.get(&e) {
                e = n;
            }
            e
        };
        let (p, q) = (now(p), now(q));
        if p == q || picked.contains(&p) || picked.contains(&q) {
            continue;
        }
        // kept whatever order the edges are stored in
        let (keep, go) = if b.edge(p).name < b.edge(q).name {
            (p, q)
        } else {
            (q, p)
        };
        if join_edge(&mut b, keep, go).is_some() {
            into.insert(go, keep);
            moved = true;
        }
    }
    moved.then_some(b)
}

/// Runs `keep` on over `go`, which goes: only if both lie on one line or circle, meet at one vertex
/// and are walked by the same loops.
fn join_edge(b: &mut Body, keep: EdgeId, go: EdgeId) -> Option<()> {
    let (x, y) = (b.edge(keep).clone(), b.edge(go).clone());
    if x.start == x.end || y.start == y.end {
        return None;
    }
    let lps = |e: &forge_topo::Edge| {
        let mut l: Vec<_> = e.coedges.iter().map(|&c| b.coedge(c).lp).collect();
        l.sort_unstable();
        l
    };
    if lps(&x) != lps(&y) {
        return None;
    }
    let (w, far) = if y.start == x.end || y.start == x.start {
        (y.start, y.end)
    } else {
        (y.end, y.start)
    };
    if far == x.start || far == x.end {
        return None;
    }
    let at_end = w == x.end;
    let t_far = if matches!(x.curve, Curve::Line(_) | Curve::Circle(_)) {
        let on = |p: Point3| {
            let wide = match x.curve.domain().period {
                Some(period) => Interval::new(x.range.lo, x.range.lo + period),
                None => x.range.grow(1e3 + y.range.width()),
            };
            let t = x.curve.closest(p, wide);
            (x.curve.eval(t).distance(p) <= 1e-3 * tol::LINEAR).then_some(t)
        };
        on(y.curve.eval(y.range.mid()))?;
        on(b.vertex(far).point)?
    } else {
        // pieces of one traced curve: the second takes up where the first stops
        let (near_t, far_t) = if y.start == w {
            (y.range.lo, y.range.hi)
        } else {
            (y.range.hi, y.range.lo)
        };
        let own = if at_end { x.range.hi } else { x.range.lo };
        if x.curve != y.curve || (near_t - own).abs() > 1e-9 * (1.0 + own.abs()) {
            return None;
        }
        far_t
    };
    // a circle's piece runs on past the kept one's end, whichever way the parameter wraps
    let t_far = match x.curve.domain().period {
        Some(period) if at_end => x.range.hi + (t_far - x.range.hi).rem_euclid(period),
        Some(period) => x.range.lo - (x.range.lo - t_far).rem_euclid(period),
        None => t_far,
    };
    if (at_end && t_far <= x.range.hi) || (!at_end && t_far >= x.range.lo) {
        return None;
    }
    let mut loops: Vec<_> = y
        .coedges
        .iter()
        .map(|&c| (b.coedge_face(c), b.coedge(c).lp))
        .collect();
    loops.sort_unstable();
    loops.dedup();
    let edge = &mut b.edges[keep.index()];
    if at_end {
        edge.end = far;
        edge.range.hi = t_far;
    } else {
        edge.start = far;
        edge.range.lo = t_far;
    }
    edge.bbox.clear();
    for (f, l) in loops {
        let uses: Vec<(EdgeId, bool)> = b
            .lp(l)
            .coedges()
            .map(|c| (b.coedge(c).edge, b.coedge(c).forward))
            .filter(|u| u.0 != go)
            .collect();
        b.replace_loop(f, l, &uses);
    }
    Some(())
}

/// The body with the seam of each round face beside a filleted edge moved to the middle of the
/// widest stretch round the face that its other loops leave free. Only a seam running between two
/// closed rims, alone in its loop with them. None if no seam moved.
fn reseamed(body: &Body, edges: &[FilletEdge]) -> Option<Body> {
    let mut b = if body.dead > 0 {
        body.compact()
    } else {
        body.clone()
    };
    let mut faces: Vec<FaceId> = edges
        .iter()
        .filter(|fe| fe.edge.index() < b.edges.len() && b.edge(fe.edge).alive)
        .flat_map(|fe| {
            b.edge(fe.edge)
                .coedges
                .iter()
                .map(|&c| b.coedge_face(c))
                .collect::<Vec<_>>()
        })
        .collect();
    faces.sort_unstable();
    faces.dedup();
    let mut moved = false;
    for f in faces {
        moved |= move_seam(&mut b, f).is_some();
    }
    moved.then_some(b)
}

fn move_seam(b: &mut Body, f: FaceId) -> Option<()> {
    let face = b.face(f);
    let surface = face.surface.clone();
    if !matches!(surface, Surface::Cylinder(_) | Surface::Cone(_)) || face.loops.len() < 2 {
        return None;
    }
    // the loop of a seam and two rims, the others' extent round the face
    let mut seam = None;
    let mut taken: Vec<f64> = Vec::new();
    for &l in &face.loops {
        let uses: Vec<CoedgeId> = b.lp(l).coedges().collect();
        let seams: Vec<EdgeId> = uses
            .iter()
            .map(|&c| b.coedge(c).edge)
            .filter(|&e| b.edge(e).coedges.iter().all(|&c| b.coedge_face(c) == f))
            .collect();
        if uses.len() == 4 && seams.len() == 2 && seams[0] == seams[1] {
            let rims: Vec<EdgeId> = uses
                .iter()
                .map(|&c| b.coedge(c).edge)
                .filter(|&e| e != seams[0])
                .collect();
            seam = Some((seams[0], rims));
        } else {
            for &c in &uses {
                let e = b.edge(b.coedge(c).edge);
                for k in 0..=32 {
                    let p = e.curve.eval(e.range.lo + e.range.width() * k as f64 / 32.0);
                    taken.push(surface.project(p).x.rem_euclid(dmath::TAU));
                }
            }
        }
    }
    let (e, rims) = seam?;
    if taken.is_empty() || rims.len() != 2 {
        return None;
    }
    taken.sort_by(f64::total_cmp);
    let (mut gap, mut at) = (
        taken[0] + dmath::TAU - taken[taken.len() - 1],
        taken[taken.len() - 1],
    );
    for w in taken.windows(2) {
        if w[1] - w[0] > gap {
            (gap, at) = (w[1] - w[0], w[0]);
        }
    }
    let u = at + 0.5 * gap;
    let (vs, ve) = (b.edge(e).start, b.edge(e).end);
    let new_at = |b: &Body, v: VertexId| {
        let uv = surface.project(b.vertex(v).point);
        surface.eval(forge_math::Point2::new(u, uv.y))
    };
    let (ps, pe) = (new_at(b, vs), new_at(b, ve));
    let dir = (pe - ps).normalize()?;
    // each rim runs round from the new point on, one turn as before
    let mut ranges = Vec::new();
    for &r in &rims {
        let x = b.edge(r);
        if x.start != x.end || !matches!(x.curve, Curve::Circle(_)) {
            return None;
        }
        let q = if x.start == vs {
            ps
        } else if x.start == ve {
            pe
        } else {
            return None;
        };
        let t = x.curve.closest(q, x.range);
        ranges.push((r, Interval::new(t, t + x.range.width())));
    }
    for (r, range) in ranges {
        let x = &mut b.edges[r.index()];
        x.range = range;
        x.bbox.clear();
    }
    b.vertices[vs.index()].point = ps;
    b.vertices[ve.index()].point = pe;
    let len = (pe - ps).length();
    let x = &mut b.edges[e.index()];
    x.curve = Curve::Line(forge_geom::Line::new(ps, dir));
    x.range = Interval::new(0.0, len);
    x.bbox.clear();
    Some(())
}

fn fillet_on(body: &Body, edges: &[FilletEdge], opts: &FilletOptions, feature: u64) -> R<Body> {
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
    // an edge between two faces on one plane (a seam a boolean left) has nothing to round:
    // merging the faces removes it
    let flush = |e: EdgeId| {
        let c = &body.edge(e).coedges;
        c.len() == 2 && {
            let (a, b) = (body.coedge_face(c[0]), body.coedge_face(c[1]));
            a != b && same_surface(&body, a, b)
        }
    };
    // nor has one its faces meet tangent along (a revolve's seam, picked with all the edges)
    let tangent = |fe: &FilletEdge| {
        let x = body.edge(fe.edge);
        [0.1, 0.5, 0.9].iter().all(|&u| {
            let t = x.range.lo + u * x.range.width();
            matches!(
                local(&body, fe.edge, fe.radius.max(), t),
                Err(FilletError::Smooth { .. })
            )
        })
    };
    let kept: Vec<FilletEdge> = edges
        .iter()
        .filter(|fe| !flush(fe.edge) && !tangent(fe))
        .cloned()
        .collect();
    if let (Some(fe), true) = (edges.first(), kept.is_empty()) {
        return Err(FilletError::Smooth { edge: fe.edge });
    }
    let edges = &kept[..];
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
        let (mut first, mut second) = fe.radius.halves();
        // a law that ends at another radius than it began cannot run round: each half holds its end's
        let p = fe.radius.points();
        if let [(0.0, a), .., (1.0, b)] = p[..]
            && a != b
        {
            for half in [&mut first, &mut second] {
                if let Some(Radius::Points(q)) = half
                    && let [(_, r)] = q[..]
                {
                    *half = Some(Radius::Constant(r));
                }
            }
        }
        for (edge, radius) in [(e, first), (other, second)] {
            match radius {
                Some(radius) => picked.push(FilletEdge { edge, radius }),
                None => unpicked.push((edge, fe.radius.max())),
            }
        }
    }
    let edges = &picked[..];
    merge_flush(&mut body, &seen);
    let all = if opts.only {
        edges.iter().map(|f| (f.edge, f.radius.max())).collect()
    } else {
        tangent_chains(&body, edges, &unpicked, &opts.stops)
    };
    let seen: BTreeSet<EdgeId> = all.iter().map(|f| f.0).collect();
    merge_flush(&mut body, &seen);
    let root = Name::feature(feature);
    let mut all = all;
    // an arc whose radius the ball's is: no blend rolls there
    let mut pivots = Vec::new();
    all.retain(|&(e, r)| match pivot_of(&body, e, r, root) {
        Some(p) => {
            pivots.push(p);
            false
        }
        None => true,
    });
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
        if opts.across {
            let pairs = crossing_pairs(&body, &by_edge);
            let Some(&(first, ..)) = pairs.first() else {
                return Err(FilletError::Overflow {
                    edge: edges.first().map_or(EdgeId(0), |f| f.edge),
                });
            };
            if pass > 0 {
                return Err(FilletError::Overflow { edge: first });
            }
            for (e, other, near) in pairs {
                let r = by_edge[&e].blend.radius;
                by_edge.insert(e, roll_across(&body, e, other, r, near, root)?);
                by_edge.remove(&other);
                all.retain(|f| f.0 != other);
                picks.remove(&other);
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
            b.free_ends = opts.only || !opts.stops.is_empty();
            b.cross_seam = opts.seams;
            b.setbacks = opts.setbacks.clone();
            b.pivots = pivots;
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

/// Where blends of both kinds meet at a vertex no one ball or mitre serves: the kind fewer of them
/// there are rounds first, and the rest then roll on as a tangent chain over its blend (the
/// corner is the torus they sweep, as Parasolid and OCCT make it). The edges of each round.
fn minority_first(body: &Body, edges: &[FilletEdge]) -> Option<(Vec<FilletEdge>, Vec<FilletEdge>)> {
    let picked: BTreeMap<EdgeId, bool> = edges
        .iter()
        .filter(|f| f.edge.index() < body.edges.len() && body.edge(f.edge).alive)
        .filter_map(|f| {
            let x = body.edge(f.edge);
            let l = local(body, f.edge, f.radius.max(), x.range.mid()).ok()?;
            Some((f.edge, l.convex))
        })
        .collect();
    if picked.values().all(|&c| c) || picked.values().all(|&c| !c) {
        return None;
    }
    let mut at: BTreeMap<VertexId, Vec<EdgeId>> = BTreeMap::new();
    for &e in picked.keys() {
        let x = body.edge(e);
        at.entry(x.start).or_default().push(e);
        at.entry(x.end).or_default().push(e);
    }
    let mut first: BTreeSet<EdgeId> = BTreeSet::new();
    for es in at.values() {
        let concave = es.iter().filter(|e| !picked[e]).count();
        let convex = es.len() - concave;
        if concave == 0 || convex == 0 {
            continue;
        }
        // concave wins a tie, the order Parasolid names first
        let minority = convex < concave;
        // those rolling on over it must be of one radius, or the chain's law runs across the corner
        let radius = |e: &EdgeId| {
            edges
                .iter()
                .find(|f| f.edge == *e)
                .and_then(|f| f.radius.constant())
        };
        let mut rest = es.iter().filter(|e| picked[e] != minority).map(radius);
        let one = rest.next().flatten();
        if one.is_none() || rest.any(|r| r != one) {
            continue;
        }
        first.extend(es.iter().filter(|e| picked[e] == minority));
    }
    if first.is_empty() || first.len() == edges.len() {
        return None;
    }
    let (a, b) = edges.iter().cloned().partition(|f| first.contains(&f.edge));
    Some((a, b))
}

/// The pivot edge `e` of radius r makes, where it is an arc on a cylinder whose offset by the ball's
/// radius collapses onto the axis, the plane it rims square to that axis.
fn pivot_of(body: &Body, e: EdgeId, r: f64, root: Name) -> Option<Pivot> {
    let x = body.edge(e);
    let Curve::Circle(k) = &x.curve else { return None };
    if (k.radius - r).abs() > tol::LINEAR {
        return None;
    }
    let l = local(body, e, r, x.range.mid()).ok()?;
    let (top, wall, cyl, plane) = match (&body.face(l.faces[0]).surface, &body.face(l.faces[1]).surface) {
        (Surface::Plane(p), Surface::Cylinder(c)) => (l.faces[0], l.faces[1], c, p),
        (Surface::Cylinder(c), Surface::Plane(p)) => (l.faces[1], l.faces[0], c, p),
        _ => return None,
    };
    if cyl.frame.z().cross(plane.frame.z()).length() > 1e3 * tol::ANGULAR
        || (cyl.radius - r).abs() > tol::LINEAR
    {
        return None;
    }
    // the ball rolls inside the wall: its centre is on the axis, a radius off the plane on the
    // ball's side, and its foot on the plane stands over that
    let axis = k.frame.origin;
    let n = if l.toward.dot(plane.frame.z().get()) > 0.0 {
        plane.frame.z().get()
    } else {
        -plane.frame.z().get()
    };
    if (l.p + l.toward * r).distance(axis + n * r) > 2.0 * tol::LINEAR {
        return None;
    }
    Some(Pivot {
        edge: e,
        top,
        wall,
        centre: axis + n * r,
        radius: r,
        convex: l.convex,
        name: root.with_str("fillet").with_name(x.name),
        foot: axis,
        pole: None,
        low: [None, None],
        sections: [None, None],
    })
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
fn tangent_chains(
    body: &Body,
    edges: &[FilletEdge],
    unpicked: &[(EdgeId, f64)],
    stops: &[EdgeId],
) -> Vec<(EdgeId, f64)> {
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
                .filter(|&o| {
                    o != edge && !have.contains(&o) && !stops.contains(&o) && body.edge(o).coedges.len() == 2
                })
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
        t.extend(end_tangent(body, e, v, false));
    }
    if x.end == v {
        t.extend(end_tangent(body, e, v, true));
    }
    t
}

/// The unit tangent of `e` out of `v`, at its end or its start. A traced curve's end tangent is only
/// as good as its trace: the surfaces' normals there give the exact one.
fn end_tangent(body: &Body, e: EdgeId, v: VertexId, end: bool) -> Option<Vec3> {
    let x = body.edge(e);
    let d = if end {
        -x.curve.deriv(x.range.hi).d1
    } else {
        x.curve.deriv(x.range.lo).d1
    };
    if let (Curve::Intersection(_), [c0, c1]) = (&x.curve, &x.coedges[..]) {
        let p = body.vertex(v).point;
        let n = |c: CoedgeId| body.face(body.coedge_face(c)).surface.gradient(p);
        if let Some(u) = n(*c0).cross(n(*c1)).normalize() {
            let u = u.get();
            return Some(if u.dot(d) < 0.0 { -u } else { u });
        }
    }
    d.normalize().map(|u| u.get())
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
        let tan = |(e, end): (EdgeId, bool)| end_tangent(body, e, v, end);
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
        body.replace_loop(f, l, &uses);
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

/// Joins the two faces either side of `e` into the one of the lesser name (so the order the
/// boolean left them in names nothing): their loops through `e` become one, the other face's
/// loops move over, and edges the joined loop walks there and straight back are dropped.
fn merge_faces(body: &mut Body, e: EdgeId) {
    let (mut ca, mut cb) = (body.edge(e).coedges[0], body.edge(e).coedges[1]);
    if body.face(body.coedge_face(cb)).name < body.face(body.coedge_face(ca)).name {
        (ca, cb) = (cb, ca);
    }
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
    // the joined loop is the outer one where a's was
    let outer = body.face(a).loops.first() == Some(&la);
    body.remove_loop(a, la);
    let lbs = body.face(b).loops.clone();
    for &l in &lbs {
        body.remove_loop(b, l);
    }
    body.kill_face(b);
    let mut first = None;
    for l in joined.into_iter().filter(|l| !l.is_empty()) {
        first.get_or_insert(body.add_loop(a, &l));
    }
    for o in others {
        body.add_loop(a, &o);
    }
    if outer && let Some(l) = first {
        let ls = &mut body.faces[a.index()].loops;
        if let Some(k) = ls.iter().position(|&x| x == l) {
            ls[..=k].rotate_right(1);
        }
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
    let flat = l.p + l.toward * r;
    let far = flat.distance(l.p);
    // in a narrow wedge the centre stands far from the edge, and the faces' curving over that
    // distance moves it by more than the radius
    let near = if far > 1.5 * r {
        rolled_centre(body, &l, r).unwrap_or(flat)
    } else {
        flat
    };
    let region = body.edge_box(e).grow((4.0 * r).max(1.5 * far) + tol::LINEAR);
    let of = |near: Point3| blend_of(body, l.faces, [l.sign[0] * r, l.sign[1] * r], l.p, near, region);
    let blend = of(near)
        .filter(|b| b.spine.eval(spine_closest(&b.spine, l.p)).distance(near) <= r)
        .or_else(|| {
            // the faces curve enough to move the centre off the normals' estimate: it is where the
            // ball grows to, which the spine must pass through
            let c = rolled_centre(body, &l, r)?;
            of(c).filter(|b| b.spine.eval(spine_closest(&b.spine, c)).distance(c) <= tol::LINEAR)
        })
        .ok_or(FilletError::NoBlend { edge: e })?;
    let springs = [
        blend.spring(true).ok_or(FilletError::NoBlend { edge: e })?,
        blend.spring(false).ok_or(FilletError::NoBlend { edge: e })?,
    ];
    let (blend, springs, over) = switch_support(body, &l, blend, springs, |faces, sign| {
        blend_of(body, faces, [sign[0] * r, sign[1] * r], l.p, near, region)
    });
    let (blend, springs, on_rim) = match over
        .iter()
        .all(Option::is_none)
        .then(|| cliff(body, &l, &blend, &springs, r, near, region))
    {
        Some(Some((b, sp, s))) => (b, sp, Some(s)),
        _ => (blend, springs, None),
    };
    Ok(rolled_from(body, e, &l, blend, springs, (over, on_rim), root))
}

fn rolled_from(
    body: &Body,
    e: EdgeId,
    l: &Local,
    blend: Blend,
    springs: [Curve; 2],
    (over, on_rim): (Over, Option<usize>),
    root: Name,
) -> Rolled {
    let split = switch_split(body, e, l, &blend, &springs, &over);
    let tm = spine_closest(&blend.spine, l.p);
    let aligned = blend.spine.deriv(tm).d1.dot(l.tangent.get()) > 0.0;
    Rolled {
        edge: e,
        convex: l.convex,
        blend,
        springs,
        aligned,
        faces: l.faces,
        coedges: l.coedges,
        forward: l.forward,
        name: root.with_str("fillet").with_name(body.edge(e).name),
        base: root.with_str("fillet").with_name(body.edge(e).name.whole()),
        var: None,
        over,
        split,
        cliff: on_rim,
    }
}

/// Pairs of picked edges of one radius and kind along a face between them, whose blends' springs
/// on it have crossed all along: the first edge, the second, and near where the ball over both rolls.
fn crossing_pairs(body: &Body, by_edge: &BTreeMap<EdgeId, Rolled>) -> Vec<(EdgeId, EdgeId, Point3)> {
    let plain = |r: &Rolled| r.var.is_none() && r.cliff.is_none() && r.over.iter().all(Option::is_none);
    let mut used = BTreeSet::new();
    let mut out = Vec::new();
    for (&a, ra) in by_edge {
        for (&b, rb) in by_edge.range(a..).skip(1) {
            if used.contains(&a)
                || used.contains(&b)
                || !plain(ra)
                || !plain(rb)
                || ra.convex != rb.convex
                || (ra.blend.radius - rb.blend.radius).abs() > tol::LINEAR
            {
                continue;
            }
            let shared = |s: usize| (0..2).find(|&k| ra.faces[s] == rb.faces[k]).map(|k| (s, k));
            let [Some((sa, sb))] = [shared(0).or_else(|| shared(1))] else {
                continue;
            };
            let f = ra.faces[sa];
            let surface = &body.face(f).surface;
            let domain = surface.domain();
            let (xa, xb) = (body.edge(a), body.edge(b));
            let crossed = |frac: f64| {
                let ea = xa.curve.eval(xa.range.lo + frac * xa.range.width());
                let eb = xb.curve.eval(xb.curve.closest(ea, xb.range));
                let pa = ra.springs[sa].eval(spine_closest(&ra.blend.spine, ea));
                let pb = rb.springs[sb].eval(spine_closest(&rb.blend.spine, eb));
                let o = surface.project(ea);
                let uv = |p: Point3| {
                    let q = surface.project(p);
                    let wrap = |d: f64, per: Option<f64>| per.map_or(d, |w| d - w * (d / w).round());
                    (wrap(q.x - o.x, domain.u.period), wrap(q.y - o.y, domain.v.period))
                };
                let (d, ua, ub) = (uv(eb), uv(pa), uv(pb));
                let dd = d.0 * d.0 + d.1 * d.1;
                dd > 0.0 && (ua.0 * d.0 + ua.1 * d.1) > (ub.0 * d.0 + ub.1 * d.1)
            };
            if [0.02, 0.5, 0.98].iter().all(|&s| crossed(s)) {
                let mid = xa.curve.eval(xa.range.mid());
                let ca = ra.blend.spine.eval(spine_closest(&ra.blend.spine, mid));
                let cb = rb.blend.spine.eval(spine_closest(&rb.blend.spine, mid));
                out.push((a, b, ca.lerp(cb, 0.5)));
                used.insert(a);
                used.insert(b);
            }
        }
    }
    out
}

/// One blend for two edges of the face between them: resting on the faces beyond each, with the
/// face itself gone. The ball must rest on both within them, and stay behind the face it passes.
fn roll_across(body: &Body, e: EdgeId, other: EdgeId, r: f64, near: Point3, root: Name) -> R<Rolled> {
    let err = FilletError::Crossed { edge: e };
    let l = local(body, e, r, body.edge(e).range.mid())?;
    let sense = |f: FaceId| if body.face(f).same_sense { 1.0 } else { -1.0 };
    let shared = |f: FaceId| body.edge(other).coedges.iter().any(|&c| body.coedge_face(c) == f);
    let s = (0..2).find(|&s| shared(l.faces[s])).ok_or(err.clone())?;
    let f = l.faces[s];
    let beyond = body
        .edge(other)
        .coedges
        .iter()
        .map(|&c| body.coedge_face(c))
        .find(|&x| x != f)
        .ok_or(err.clone())?;
    let mut faces = l.faces;
    faces[s] = beyond;
    let mut sign = l.sign;
    sign[s] *= sense(f) * sense(beyond);
    let region = body
        .edge_box(e)
        .hull(&body.edge_box(other))
        .grow(4.0 * r + tol::LINEAR);
    // no ball rests on both (walls parallel, too far apart): the two rounds are all there is
    let blend = blend_of(body, faces, [sign[0] * r, sign[1] * r], l.p, near, region)
        .ok_or(FilletError::Overflow { edge: e })?;
    let springs = [
        blend.spring(true).ok_or(err.clone())?,
        blend.spring(false).ok_or(err.clone())?,
    ];
    let t = spine_closest(&blend.spine, l.p);
    let centre = blend.spine.eval(t);
    if body.face(f).surface.implicit(centre).abs() < r - tol::LINEAR
        || !(0..2).all(|k| on_face(body, faces[k], springs[k].eval(t)))
    {
        return Err(err);
    }
    let mut over = [None, None];
    over[s] = Some((beyond, other));
    Ok(rolled_from(body, e, &l, blend, springs, (over, None), root))
}

/// Where the ball is wider than a face it should rest on, so its foot there lies past that face's
/// far edge: it rests on that edge instead, rolling along it (a cliff edge). The edge's curve is a
/// surface of no thickness whose offset is the tube round it, so the spine and the springs are
/// those of any blend, the spring on that side being the edge itself.
fn cliff(
    body: &Body,
    l: &Local,
    blend: &Blend,
    springs: &[Curve; 2],
    r: f64,
    near: Point3,
    region: Box3,
) -> Option<(Blend, [Curve; 2], usize)> {
    let foot = |b: &Blend, c: &Curve| c.eval(spine_closest(&b.spine, l.p));
    for s in 0..2 {
        let f = l.faces[s];
        let mine = body.coedge(l.coedges[s]).edge;
        if foot_on(body, l, s, foot(blend, &springs[s]))
            || !off_all_along(body, l, mine, blend, &springs[s], s)
        {
            continue;
        }
        let keep = l.faces[1 - s];
        let other = &body.face(keep).surface;
        for &lp in &body.face(f).loops {
            for c in body.lp(lp).coedges() {
                let g = body.coedge(c).edge;
                let x = body.edge(g);
                if g == mine {
                    continue;
                }
                let rail = match &x.curve {
                    Curve::Line(k) => Surface::Cylinder(forge_geom::Cylinder {
                        frame: Frame::from_z(k.origin, k.dir),
                        radius: 0.0,
                    }),
                    Curve::Circle(k) => Surface::Torus(forge_geom::Torus {
                        frame: k.frame,
                        major: k.radius,
                        minor: 0.0,
                    }),
                    _ => continue,
                };
                let Some(mut b) = Blend::new(other, &rail, l.sign[1 - s] * r, r, near, region) else {
                    continue;
                };
                let (Some(on_keep), Some(on_rail)) = (b.spring(true), b.spring(false)) else {
                    continue;
                };
                // the ball must touch the far edge within it, and rest on the other face
                let t = spine_closest(&b.spine, l.p);
                let q = on_rail.eval(t);
                if x.curve.eval(x.curve.closest(q, x.range)).distance(q) > tol::LINEAR
                    || !on_face(body, keep, on_keep.eval(t))
                    || blocked_beyond(body, f, c, q, b.spine.eval(t), r, l.convex)
                {
                    continue;
                }
                let springs = if s == 1 {
                    [on_keep, on_rail]
                } else {
                    std::mem::swap(&mut b.a, &mut b.b);
                    std::mem::swap(&mut b.da, &mut b.db);
                    [on_rail, on_keep]
                };
                return Some((b, springs, s));
            }
        }
    }
    None
}

/// Whether the face across coedge `c` from `f` stands in the way of a ball touching the far edge
/// at `q`: the centre lies on the ball's own side of it (in the material for a convex edge, out of
/// it for a concave one), nearer than the radius, and the way from `q` to the foot of the centre
/// leads onto the face. The face beyond a true cliff falls away; this one the ball would pass through.
fn blocked_beyond(
    body: &Body,
    f: FaceId,
    c: CoedgeId,
    q: Point3,
    centre: Point3,
    r: f64,
    convex: bool,
) -> bool {
    let Some(p) = body.partner(c) else {
        return false;
    };
    let n = body.coedge_face(p);
    if n == f {
        return false;
    }
    let s = &body.face(n).surface;
    let uv = s.project(centre);
    let foot = s.eval(uv);
    let Some(out) = body.face_normal(n, uv) else {
        return false;
    };
    let above = (centre - foot).dot(out.get());
    let Some(towards) = (foot - q).normalize() else {
        return false;
    };
    let step = q + towards.get() * (1e-3 * r);
    (if convex { above < 0.0 } else { above > 0.0 })
        && foot.distance(centre) < r - tol::LINEAR
        && on_face(body, n, s.eval(s.project(step)))
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
/// edge of it, the ball rests on the face beyond: the blend rolled on that face instead
/// (`make`, from the faces and the sides of their surfaces the ball is on).
fn switch_support(
    body: &Body,
    l: &Local,
    blend: Blend,
    springs: [Curve; 2],
    make: impl Fn([FaceId; 2], [f64; 2]) -> Option<Blend>,
) -> (Blend, [Curve; 2], Over) {
    let mid = |b: &Blend, c: &Curve| c.eval(spine_closest(&b.spine, l.p));
    let sense = |f: FaceId| if body.face(f).same_sense { 1.0 } else { -1.0 };
    let plane = |f: FaceId| matches!(body.face(f).surface, Surface::Plane(_));
    for s in 0..2 {
        let b = l.faces[s];
        if foot_on(body, l, s, mid(&blend, &springs[s])) {
            continue;
        }
        // the faces beyond b's edges, where the ball's foot lies past it: first those tangent to it
        // along the edge, their normals agreeing at its middle (two planes never are); then any
        // face the ball can rest on as well, the strip of b swallowed, so long as the ball stays
        // behind b
        let mut beyond: Vec<(FaceId, EdgeId, bool)> = body
            .face(b)
            .loops
            .iter()
            .flat_map(|&lp| body.lp(lp).coedges())
            .filter_map(|c| {
                let g = body.coedge(c).edge;
                let cf = body.coedge_face(body.partner(c)?);
                // a seam between two pieces of one surface is no other face to rest on
                let seam = body.face(b).surface == body.face(cf).surface;
                if cf == b || cf == l.faces[1 - s] || seam {
                    return None;
                }
                let x = body.edge(g);
                let q = x.curve.eval(x.range.mid());
                let n = |f: FaceId| body.face_normal(f, body.face(f).surface.project(q));
                let smooth = !(plane(b) && plane(cf))
                    && n(b)
                        .zip(n(cf))
                        .is_some_and(|(nb, nc)| nb.dot(nc.get()) >= 1.0 - 1e-9);
                Some((cf, g, smooth))
            })
            .collect();
        beyond.sort_by_key(|x| !x.2);
        for (cf, g, smooth) in beyond {
            let mut faces = l.faces;
            faces[s] = cf;
            let mut sign = l.sign;
            sign[s] *= sense(b) * sense(cf);
            let Some(nb) = make(faces, sign) else {
                continue;
            };
            let (Some(s0), Some(s1)) = (nb.spring(true), nb.spring(false)) else {
                continue;
            };
            let ns = [s0, s1];
            // past a sharp edge the ball stays behind the strip it passes over, all of it
            let clear = smooth || {
                let c = nb.spine.eval(spine_closest(&nb.spine, l.p));
                (body.face(b).surface.implicit(c).abs() - nb.radius) >= -tol::LINEAR
                    && off_all_along(body, l, body.coedge(l.coedges[s]).edge, &blend, &springs[s], s)
            };
            if clear && (0..2).all(|k| on_face(body, faces[k], mid(&nb, &ns[k]))) {
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
            // one leaving the edge's own vertex: the joint there finds where the spring crosses it
            if [gx.start, gx.end].iter().any(|&w| w == x.start || w == x.end) {
                continue;
            }
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
            if h0 * h1 >= 0.0 || h0.abs().min(h1.abs()) <= 4.0 * tol::LINEAR {
                continue;
            }
            // tangent along g: its faces' normals agree at its middle
            let [c0, c1] = gx.coedges[..] else { continue };
            let (f0, f1) = (body.coedge_face(c0), body.coedge_face(c1));
            let q = gx.curve.eval(gx.range.mid());
            let n = |f: FaceId| body.face_normal(f, body.face(f).surface.project(q));
            if f0 == f1
                || body.face(f0).surface == body.face(f1).surface
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

/// Whether side `s`'s foot is off its face near both ends of the edge as well: the ball passes the
/// whole strip, and has not merely come to a pocket partway along.
fn off_all_along(body: &Body, l: &Local, e: EdgeId, blend: &Blend, spring: &Curve, s: usize) -> bool {
    let x = body.edge(e);
    [0.02, 0.98].iter().all(|&f| {
        let p = x.curve.eval(x.range.lo + f * x.range.width());
        !on_face(body, l.faces[s], spring.eval(spine_closest(&blend.spine, p)))
    })
}

/// Whether the ball's foot `q` lies on the blend's face on side `s`. The foot is reached from the
/// edge's middle across the face: with no other edge of the face in the box they span (grown for
/// a surface's bulge), it cannot have left, and no point need be tested.
fn foot_on(body: &Body, l: &Local, s: usize, q: Point3) -> bool {
    let f = l.faces[s];
    let own = body.coedge(l.coedges[s]).edge;
    let reach = Box3::from_points([l.p, q]).grow(0.5 * l.p.distance(q));
    let clear = body.face(f).loops.iter().all(|&lp| {
        body.lp(lp).coedges().all(|c| {
            let g = body.coedge(c).edge;
            g == own || !body.edge_box(g).overlaps(&reach)
        })
    });
    if clear {
        return true;
    }
    // a foot the surface cannot place (a spring on a loft within tolerance) has not run off it
    let surface = &body.face(f).surface;
    surface.eval(surface.project(q)).distance(q) > tol::LINEAR || on_face(body, f, q)
}

/// Whether point `p` lies within face `f`, seen from its surface: a foot just off a face is on the
/// face beyond a smooth edge, whichever surface it was worked out on.
fn in_face(body: &Body, f: FaceId, p: Point3) -> bool {
    let surface = &body.face(f).surface;
    let loops = forge_topo::uv::face_uv_loops(body, f);
    let d = surface.domain();
    let uv = surface.project(p);
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
            let (blend, springs, over) = switch_support(body, &l, blend, springs, |faces, sign| {
                let (a, b) = (&body.face(faces[0]).surface, &body.face(faces[1]).surface);
                let vb = VarBlend::new(a, sign[0], b, sign[1], guide.clone(), law.clone(), range).ok()?;
                Some(Blend::variable(a, b, vb))
            });
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
                base: root.with_str("fillet").with_name(edge.name.whole()),
                var: Some(VarEdge {
                    chain: id,
                    index: i,
                    len: n,
                    closed: chain.closed,
                    own,
                }),
                over,
                split: None,
                cliff: None,
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
        // (one point of a law, on a half of a closed edge, is no radius held along that half)
        let held = if pts.len() > 1 { pick.constant() } else { None };
        own.push((pts, held));
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
    /// Whether springs are tested against their faces' seams (`FilletOptions::seams`).
    cross_seam: bool,
    /// Trims (as in `trims`) reaching past where the edge's traced curve stops: it is walked on to them.
    carried: BTreeSet<(EdgeId, bool)>,
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
    /// Rims whose ball stands still: filled in as their ends are reached.
    pivots: Vec<Pivot>,
    /// Faces a whole-rim pivot took away.
    gone_faces: Vec<FaceId>,
    /// Vertices at one place, a cut landing on an edge's far end or two cuts meeting: the first
    /// becomes the second, and the edge between goes.
    merged: Vec<(VertexId, VertexId)>,
    collapsing: bool,
    /// A corner was closed in a way that only the result can vouch for: the body must pass its
    /// check and have a mass.
    unsure: bool,
    /// An edge whose blend ran out beside a tangent chain going on unpicked.
    free_end: Option<EdgeId>,
    /// Whether a chain may go on unpicked here: `only`, or a second round's stops.
    free_ends: bool,
    /// Blend sides whose spring a mitre carries on past its face's corner, with the spring's parameter
    /// at the corner: the face grows over its surface carried on to meet the other blend.
    overrun: BTreeMap<(usize, usize), Vec<f64>>,
}

/// An arc of a face's rim whose radius the ball's is: its centre stays put while it turns about
/// the arc's axis, so the blend there is that ball, a sphere (a torus of no major radius).
#[derive(Clone, Debug)]
struct Pivot {
    edge: EdgeId,
    /// The flat or round face the arc rims, and the cylinder it runs on.
    top: FaceId,
    wall: FaceId,
    centre: Point3,
    radius: f64,
    convex: bool,
    name: Name,
    /// The ball's foot on the top; its vertex there, the cut edges' vertices on the wall at the
    /// arc's start and end, and the sections from the blends beside it, once made.
    foot: Point3,
    pole: Option<VertexId>,
    low: [Option<VertexId>; 2],
    sections: [Option<EdgeId>; 2],
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
            cross_seam: false,
            carried: BTreeSet::new(),
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
            pivots: Vec::new(),
            gone_faces: Vec::new(),
            merged: Vec::new(),
            collapsing: false,
            unsure: false,
            free_end: None,
            free_ends: false,
            overrun: BTreeMap::new(),
        }
    }

    fn build(mut self) -> R<Body> {
        let was: BTreeSet<Name> = self.body.vertices.iter().map(|v| v.name).collect();
        let edges_from = self.body.edges.len();
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
        self.close_pivots()?;
        self.cut_smooth_edges()?;
        let springs = self.make_springs()?;
        self.trim_edges()?;
        self.apply_merges();
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
        let touched: Vec<(FaceId, EdgeId)> = by_face.iter().map(|(&f, e)| (f, e[0])).collect();
        by_face.retain(|f, _| groups[f].len() > 1);
        self.rebuild_faces(&springs);
        self.split_slivers();
        self.align_rings();
        let no_width = self.no_width();
        for (f, edges) in by_face {
            // two blends' springs may have passed each other with no crossing (round a ring)
            if edges.len() > 1
                && !no_width.iter().any(|n| n.0 == f)
                && (crate::selfx::face_loops_cross(&self.body, f)
                    || loops_touch(&self.body, f)
                    || (loops_near(&self.body, f) && !forge_topo::sides_of(&self.body, &[f]).is_empty()))
            {
                return Err(FilletError::Overflow { edge: edges[0] });
            }
        }
        // a spring run across another loop of its face (a hole's rim just past the round's edge)
        for (f, edge) in touched {
            // on flats only: on a curved face, its curves cost more than the crossing is worth asking
            if self.body.face(f).loops.len() > 1
                && matches!(self.body.face(f).surface, Surface::Plane(_))
                && !no_width.iter().any(|n| n.0 == f)
                && spring_crosses_loop(&self.body, f, edges_from)
            {
                return Err(FilletError::Overflow { edge });
            }
        }
        self.add_blends(&springs);
        for f in std::mem::take(&mut self.gone_faces) {
            self.body.kill_face(f);
        }
        for (surface, same_sense, name, uses) in std::mem::take(&mut self.patches) {
            let shell = self.body.faces[0].shell;
            let f = self.body.add_face(shell, surface, same_sense, name);
            self.body.add_loop(f, &uses);
        }
        if let Some(s) = self.setbacks.iter().find(|s| !self.set_back.contains(&s.0)) {
            return Err(FilletError::Corner { vertex: s.0 });
        }
        let closed_up = self.collapsing;
        let unsure = self.unsure;
        let free_end = self.free_end;
        let first = self.rolled.first().map(|r| r.edge);
        self.collapse(no_width);
        let mut out = self.body.into_compact();
        // new vertices sharing a name: a corner reached two ways, from pieces of one face. Which
        // should keep it nothing here can say stably, so such a corner refuses.
        let mut new: Vec<Name> = out
            .vertices
            .iter()
            .map(|v| v.name)
            .filter(|n| !was.contains(n))
            .collect();
        new.sort_unstable();
        // new vertices sharing a name: told apart in order along x, y, z
        if new.windows(2).any(|w| w[0] == w[1]) {
            for &name in new
                .windows(2)
                .filter(|w| w[0] == w[1])
                .map(|w| &w[0])
                .collect::<BTreeSet<_>>()
            {
                let mut same: Vec<usize> = (0..out.vertices.len())
                    .filter(|&i| out.vertices[i].name == name)
                    .collect();
                let key = |i: &usize| {
                    let p = out.vertices[*i].point;
                    [p.x, p.y, p.z]
                };
                same.sort_by(|a, b| key(a).partial_cmp(&key(b)).unwrap_or(std::cmp::Ordering::Equal));
                for (k, i) in same.into_iter().enumerate().skip(1) {
                    out.vertices[i].name = name.with(k as u64);
                }
            }
            // and the edges named from them
            let mut seen: BTreeMap<Name, Vec<usize>> = BTreeMap::new();
            for (i, e) in out.edges.iter().enumerate() {
                seen.entry(e.name).or_default().push(i);
            }
            for mut same in seen.into_values().filter(|v| v.len() > 1) {
                let key = |i: &usize| {
                    let e = &out.edges[*i];
                    let p = e.curve.eval(e.range.mid());
                    [p.x, p.y, p.z]
                };
                same.sort_by(|a, b| key(a).partial_cmp(&key(b)).unwrap_or(std::cmp::Ordering::Equal));
                for (k, i) in same.into_iter().enumerate().skip(1) {
                    out.edges[i].name = out.edges[i].name.with(k as u64);
                }
            }
        }
        // no edge so short should stand
        if let Some(e) = first
            && out.edge_ids().any(|x| {
                let y = out.edge(x);
                y.start != y.end
                    && out.vertex(y.start).point.distance(out.vertex(y.end).point) <= tol::LINEAR
                    && y.curve.eval(y.range.mid()).distance(out.vertex(y.start).point) <= tol::LINEAR
            })
        {
            return Err(FilletError::Overflow { edge: e });
        }
        if let Some(edge) = free_end
            && !forge_topo::check(&out).is_empty()
        {
            return Err(FilletError::FreeEnd { edge });
        }
        // these paths are rare, and a face closed up wrongly must not come back as a part
        if closed_up && !forge_topo::check(&out).is_empty() {
            return Err(FilletError::Overflow {
                edge: first.unwrap_or(EdgeId(0)),
            });
        }
        if unsure && (!forge_topo::check(&out).is_empty() || forge_topo::mass::mass(&out).is_err()) {
            return Err(FilletError::Overflow {
                edge: first.unwrap_or(EdgeId(0)),
            });
        }
        Ok(out)
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
        let mut ring = self.corners(start).ok_or(FilletError::Corner { vertex: v })?;
        // the ring starts where the geometry says, not where storage does: at the corner whose
        // arriving edge leaves v first in (x, y, z), then whose face's normal there does
        let key = |k: &Corner| {
            let b = &self.body;
            let (co, p) = (b.coedge(k.cin), b.vertex(v).point);
            let e = b.edge(co.edge);
            let d = e.curve.deriv(if co.forward { e.range.hi } else { e.range.lo }).d1;
            let d = (if co.forward { -d } else { d })
                .normalize()
                .map_or(Vec3::ZERO, |u| u.get());
            let n = b
                .face_normal(k.face, b.face(k.face).surface.project(p))
                .map_or(Vec3::ZERO, |u| u.get());
            [d.x, d.y, d.z, n.x, n.y, n.z]
        };
        let first = |a: &[f64; 6], b: &[f64; 6]| {
            (0..6)
                .find(|&i| (a[i] - b[i]).abs() > 1e-9)
                .is_some_and(|i| a[i] < b[i])
        };
        let keys: Vec<[f64; 6]> = ring.iter().map(key).collect();
        let lead = (1..ring.len()).fold(0, |m, i| if first(&keys[i], &keys[m]) { i } else { m });
        ring.rotate_left(lead);
        for k in &ring {
            self.touched.insert(k.face);
        }
        let hot: Vec<usize> = (0..ring.len())
            .filter(|&i| self.filleted(ring[i].cin).is_some())
            .collect();
        let pivot = ring.iter().find_map(|k| {
            let e = self.body.coedge(k.cin).edge;
            self.pivots.iter().position(|p| p.edge == e)
        });
        if let Some(p) = pivot {
            return match hot[..] {
                [h] => self.pivot_end(v, &ring, p, h),
                _ => Err(FilletError::Corner { vertex: v }),
            };
        }
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
                let (trims, paths, carried) = (self.trims.clone(), self.paths.clone(), self.carried.clone());
                match self.run_out(v, &ring, h) {
                    Err(FilletError::Overflow { .. }) => {
                        (self.trims, self.paths, self.carried) = (trims, paths, carried);
                        self.roll_over(v, &ring, h)
                    }
                    other => other,
                }
            }
            [a, b] if self.is_joint(&ring, a, b) => {
                // the section may lie where only the other blend's spring crosses a smooth edge
                let saved = (self.trims.clone(), self.paths.clone(), self.g_ends.clone());
                self.joint(v, &ring, a, b).or_else(|e| {
                    (self.trims, self.paths, self.g_ends) = saved.clone();
                    self.unsure = true;
                    self.joint(v, &ring, b, a).or_else(|_| {
                        (self.trims, self.paths, self.g_ends) = saved.clone();
                        let more = (
                            self.dropped.clone(),
                            self.extra.clone(),
                            self.rechain.clone(),
                            self.patches.len(),
                        );
                        self.walk_joint(v, &ring, a, b).or_else(|_| {
                            (self.trims, self.paths, self.g_ends) = saved;
                            (self.dropped, self.extra, self.rechain) = (more.0, more.1, more.2);
                            self.patches.truncate(more.3);
                            self.walk_joint(v, &ring, b, a).map_err(|_| e)
                        })
                    })
                })
            }
            [a, b] if self.mitres(v, &ring, a, b) => {
                let saved = (self.trims.clone(), self.paths.clone(), self.g_ends.clone());
                self.mitre(v, &ring, a, b).or_else(|_| {
                    // an edge tangent here though sharp further on leaves the blends continuing
                    // each other at this vertex: no mitre, a joint
                    (self.trims, self.paths, self.g_ends) = saved;
                    if !self.continues_here(v, &ring, a, b) {
                        return Err(FilletError::Mitre { vertex: v });
                    }
                    self.unsure = true;
                    self.joint(v, &ring, a, b)
                        .map_err(|_| FilletError::Mitre { vertex: v })
                })
            }
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
                // the ball passes over the whole strip: it goes, and the edge with it
                (GEnd::Gone(_), GEnd::Gone(_)) => {
                    let strip = self.rolled[ri].faces[s];
                    self.drop_whole(g);
                    self.rechain.insert(cf);
                    self.gone_faces.push(strip);
                }
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

    /// Whether two blends of one radius and kind run straight on through `v`, every other edge's
    /// faces tangent at `v` itself.
    fn continues_here(&self, v: VertexId, ring: &[Corner], a: usize, b: usize) -> bool {
        let (ra, rb) = (
            &self.rolled[self.filleted(ring[a].cin).expect("hot")],
            &self.rolled[self.filleted(ring[b].cin).expect("hot")],
        );
        let r = ra.blend.radius;
        if (rb.blend.radius - r).abs() > tol::LINEAR || ra.convex != rb.convex {
            return false;
        }
        let p = self.body.vertex(v).point;
        let (ta, tb) = (out_of(&self.body, ra.edge, v), out_of(&self.body, rb.edge, v));
        // traced edges' end tangents are only as good as their trace
        if !ta.iter().any(|x| tb.iter().any(|y| (*x + *y).length() <= 1e-4)) {
            return false;
        }
        (0..ring.len()).filter(|&j| j != a && j != b).all(|j| {
            let e = self.body.edge(self.body.coedge(ring[j].cin).edge);
            let n: Vec<_> = e
                .coedges
                .iter()
                .filter_map(|&c| {
                    let f = self.body.coedge_face(c);
                    self.body.face_normal(f, self.body.face(f).surface.project(p))
                })
                .collect();
            n.len() == 2 && n[0].cross(n[1]).length() * r <= tol::LINEAR && n[0].dot(n[1]) > 0.0
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
        Name::NONE.with_set(faces).with_name(self.body.vertex(v).name)
    }

    fn face_name(&self, f: FaceId) -> Name {
        self.body.face(f).name.whole()
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
            // a traced curve stops where its trace did, but the surfaces go on
            if e.curve.eval(t).distance(p) > tol::LINEAR
                && let Curve::Intersection(ic) = &e.curve
                && let Some((on, before)) = ic.carried_to(p, at_end)
            {
                let shift = before as f64;
                let on = Curve::Intersection(Arc::new(on));
                let full = on.domain();
                let hi = if at_end { full.hi } else { 0.0 };
                let ext = if at_end {
                    Interval::new(v, hi)
                } else {
                    Interval::new(0.0, v + shift)
                };
                if inside_face(&self.body, f, &on, ext) {
                    self.carried.insert((co.edge, at_end));
                    self.trims.insert((co.edge, at_end), (x, hi));
                    return Ok(());
                }
            }
        }
        if e.curve.eval(t).distance(p) > tol::LINEAR {
            return Err(FilletError::Overflow { edge: blend_edge });
        }
        let far = if at_end { e.start } else { e.end };
        if self.body.vertex(far).point.distance(p) <= tol::LINEAR {
            // the cut lands on the edge's far end: the edge goes, the cut's vertex is that end
            self.drop_whole(co.edge);
            self.merged.push((x, far));
            return Ok(());
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
        let mut free = false;
        for (j, (spring_c, other)) in [
            (1, (at(0).cin, at(1).face)),
            (k - 1, (at(k - 1).cout, at(k - 2).face)),
        ] {
            let ro = &self.rolled[ri];
            let s = self.side(spring_c);
            let spring = &ro.springs[s];
            let surf = &self.body.face(other).surface;
            // the face it would meet tangent to its own along their edge: the spring ends on that edge
            let e = self.body.edge(self.body.coedge(at(j).cin).edge);
            let tangent = self.free_ends && is_smooth(&self.body, self.body.coedge(at(j).cin).edge, radius);
            let found = if tangent {
                crossing(spring, t_ref, &e.curve, e.range).map(|t| (t, spring.eval(t)))
            } else {
                nearest_hit(spring, search_window(spring, t_ref, reach), surf, vp)
            };
            free |= tangent;
            if tangent {
                self.free_end = Some(edge);
            }
            let (t, p) = found.ok_or(if free {
                FilletError::FreeEnd { edge }
            } else {
                FilletError::Overflow { edge }
            })?;
            tside[s] = t;
            let names = [
                self.face_name(at(j).face),
                self.face_name(at(j - 1).face),
                ro.base,
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
                self.rolled[ri].base,
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
                .or_else(|| {
                    // one end where the blend touches the face: walked into it
                    let walk = |x, p| {
                        forge_geom::intcurve::towards_touch(
                            &ro.blend.surface,
                            &self.body.face(f).surface,
                            x,
                            p,
                            0.05 * ro.blend.radius,
                        )
                    };
                    let curve = |c| Curve::Intersection(Arc::new(c));
                    walk(a.0, b.0)
                        .map(|c| {
                            let hi = c.domain_hi();
                            (curve(c), 0.0, hi)
                        })
                        .or_else(|| {
                            let c = walk(b.0, a.0)?;
                            let hi = c.domain_hi();
                            Some((curve(c), hi, 0.0))
                        })
                })
                .ok_or(if free {
                    FilletError::FreeEnd { edge }
                } else {
                    FilletError::Overflow { edge }
                })?;
            let name = Name::NONE
                .with_set(&[ro.base, self.face_name(f)])
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
                let gname = Name::NONE.with_set(&[self.rolled[ri].name, gx.name]);
                let xg = self.body.add_vertex(gp, gname);
                self.trim(at(j).cin, gp, xg, edge, Some(own))?;
                let tg = gx.curve.closest(gp, gx.range);
                self.g_ends.entry((ri, s)).or_default().push(GEnd::Cut(tg, xg));
                let (curve, ta, tb) = meet(&self.body.face(cf).surface, &es, gp, p).ok_or(err.clone())?;
                let cname = Name::NONE
                    .with_set(&[self.rolled[ri].name, self.face_name(cf), self.face_name(ef)])
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
            .with_set(&[ro.name, self.face_name(ef)])
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
        let name = self.rolled[ri].base;
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
        // named as the stop on that edge is where the blend ends square (`vertex_name`)
        let names = |b: &Builder, c: CoedgeId| {
            let mut faces: Vec<Name> = b
                .body
                .edge(b.body.coedge(c).edge)
                .coedges
                .iter()
                .map(|&x| b.face_name(b.body.coedge_face(x)))
                .collect();
            faces.push(name);
            b.vertex_name(v, &faces)
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
                .with_set(&[name, self.face_name(f)])
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
        // a smooth edge a blend rests beyond, which its spring does not cross here: the ball passes
        // it by, its strip narrowing away to nothing at v
        let passed = |b: &Self, c: CoedgeId, s: usize, sb: usize| {
            let g = b.body.coedge(c).edge;
            let x = b.body.edge(g);
            let rests = |ri: usize, s: usize| b.rolled[ri].over[s].is_some_and(|o| o.1 == g);
            (rests(ra, s) || rests(rb, sb))
                && crossing(&b.rolled[ra].springs[s], t0, &x.curve, x.range).is_none()
        };
        // a variable chain's section is its blends' shared end; edges between surfaces pass through its feet
        for (edges, s, s_b) in [(&p_edges, sa, sb), (&x_edges, 1 - sa, 1 - sb)] {
            if var {
                let foot = self.rolled[ra].springs[s].eval(t0);
                for &c in edges.iter() {
                    let e = self.body.edge(self.body.coedge(c).edge);
                    let (f0, f1) = (
                        self.body.coedge_face(e.coedges[0]),
                        self.body.coedge_face(e.coedges[1]),
                    );
                    let change =
                        f0 != f1 && !self.body.face(f0).surface.one_with(&self.body.face(f1).surface);
                    if change && e.curve.eval(e.curve.closest(foot, e.range)).distance(foot) > tol::LINEAR {
                        return Err(err);
                    }
                }
                ta = Some(t0);
                continue;
            }
            for &c in edges.iter() {
                if passed(self, c, s, s_b) {
                    continue;
                }
                let e = self.body.edge(self.body.coedge(c).edge);
                let (f0, f1) = (
                    self.body.coedge_face(e.coedges[0]),
                    self.body.coedge_face(e.coedges[1]),
                );
                let change = f0 != f1 && !self.body.face(f0).surface.one_with(&self.body.face(f1).surface);
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
        let (na, nb) = (self.rolled[ra].base, self.rolled[rb].base);
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
                    if gx.curve.eval(t).distance(q) <= tol::LINEAR {
                        self.g_ends.entry((ri, side)).or_default().push(GEnd::Cut(t, qv));
                    } else if gx.start == v || gx.end == v {
                        self.g_ends.entry((ri, side)).or_default().push(GEnd::Gone(v));
                    } else {
                        return Err(err);
                    }
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
                if passed(self, c, s_a, s_b) {
                    self.unsure = true;
                    at_foot.push(m);
                    continue;
                }
                let e = self.body.edge(self.body.coedge(c).edge);
                let (curve, range, ename) = (e.curve.clone(), e.range, e.name.whole());
                // a variable blend's spring stops at the section: the crossing may be b's
                let hit = crossing(&self.rolled[ra].springs[s_a], ta, &curve, range);
                if hit.is_none() && !var {
                    return Err(err);
                }
                let name = Name::NONE.with_set(&[na, nb]).with_name(ename);
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
                    .with_set(&[self.rolled[ri].name])
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
            .with_set(&[na, nb])
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

    /// Two blends of a chain at `v` whose balls differ there: what the ball rests on changes along
    /// the chain at sections with no relation to its vertices. It is walked from the middle of `ia`'s
    /// blend towards `ib`'s: wherever a foot leaves its face over a smooth edge, the ball goes on over
    /// the face beyond, until it rests on what `ib`'s blend rests on. Each stretch between two of
    /// these sections that no edge runs under is a face of its own. What lies under them goes: from `v`
    /// outwards, each edge is cut where a foot crossed it, or goes whole with its far end looked at
    /// in turn.
    fn walk_joint(&mut self, v: VertexId, ring: &[Corner], ia: usize, ib: usize) -> R<()> {
        let err = FilletError::Corner { vertex: v };
        let k = ring.len();
        let (ra, rb) = (
            self.filleted(ring[ia].cin).ok_or(err.clone())?,
            self.filleted(ring[ib].cin).ok_or(err.clone())?,
        );
        let sa = self.side(ring[ia].cin);
        let sb = self.side(ring[(ib + k - 1) % k].cout);
        let plain = |r: &Rolled| r.var.is_none() && r.cliff.is_none();
        if !plain(&self.rolled[ra]) || !plain(&self.rolled[rb]) {
            return Err(err);
        }
        let r = self.rolled[ra].blend.radius;
        if (self.rolled[rb].blend.radius - r).abs() > tol::LINEAR {
            return Err(err);
        }
        let (ea, eb) = (self.rolled[ra].edge, self.rolled[rb].edge);
        let vp = self.body.vertex(v).point;
        let body = &self.body;
        let sense = |f: FaceId| if body.face(f).same_sense { 1.0 } else { -1.0 };
        // per side of a's blend: the face under the foot, and which way the surface's offset runs
        let on = |ro: &Rolled, s: usize| ro.over[s].map_or(ro.faces[s], |o| o.0);
        let mut faces = [on(&self.rolled[ra], 0), on(&self.rolled[ra], 1)];
        let mut off = [self.rolled[ra].blend.da, self.rolled[ra].blend.db];
        // a's side i against b's
        let b_side = |i: usize| if i == sa { sb } else { 1 - sb };
        let want = [on(&self.rolled[rb], b_side(0)), on(&self.rolled[rb], b_side(1))];
        let reach = 8.0 * r + body.edge_box(ea).hull(&body.edge_box(eb)).diagonal();
        let region = body.edge_box(ea).hull(&body.edge_box(eb)).grow(reach);
        let mut blend = self.rolled[ra].blend.clone();
        let mut springs = self.rolled[ra].springs.clone();
        let x = body.edge(ea);
        let mut t = spine_closest(&blend.spine, x.curve.eval(x.range.mid()));
        let t_v = spine_closest(&blend.spine, vp);
        let mut up = along(t_v - t, blend.spine.domain().period) >= 0.0;
        // the stretches: each its blend, springs, and parameters from its start section to its end
        let mut stretches: Vec<(Blend, [Curve; 2], f64, f64)> = Vec::new();
        // the sections' (t on the stretch before, and the crossing it made: side, coedge, foot)
        let mut crossed: Vec<(EdgeId, CoedgeId, usize, Point3)> = Vec::new();
        let mut t_start = t;
        let mut walked = 0.0;
        for _ in 0..16 {
            if faces == want {
                break;
            }
            // first foot off its face, going on from t
            let speed = blend.spine.deriv(t).d1.length().max(1e-12);
            let dt = (0.125 * r / speed) * if up { 1.0 } else { -1.0 };
            let foot_off = |t: f64| (0..2).find(|&i| !in_face(body, faces[i], springs[i].eval(t)));
            // (a foot that has just crossed lies on the edge between)
            if crossed.is_empty() && foot_off(t).is_some() {
                return Err(err);
            }
            let mut hit = None;
            let (mut lo, mut u) = (t, t);
            while walked < reach {
                let n = u + dt;
                if foot_off(n).is_some() {
                    hit = Some(n);
                    break;
                }
                walked += 0.125 * r;
                (lo, u) = (n, n);
            }
            let mut hi = hit.ok_or(err.clone())?;
            for _ in 0..48 {
                let m = 0.5 * (lo + hi);
                if foot_off(m).is_some() {
                    hi = m;
                } else {
                    lo = m;
                }
            }
            let i = foot_off(hi).ok_or(err.clone())?;
            let q = springs[i].eval(hi);
            // the edge it left over, and exactly where: of the edges there, the one with the face
            // the foot goes on over (near a vertex of several faces, not always the nearest)
            let ahead = springs[i].eval(hi + 0.25 * dt);
            let mut edges: Vec<(f64, CoedgeId)> = body
                .face(faces[i])
                .loops
                .iter()
                .flat_map(|&l| body.lp(l).coedges())
                .map(|c| {
                    let e = body.edge(body.coedge(c).edge);
                    (e.curve.eval(e.curve.closest(q, e.range)).distance(q), c)
                })
                .filter(|x| x.0 <= 1e-3 * r)
                .collect();
            edges.sort_by(|a, b| a.0.total_cmp(&b.0));
            let (_, c) = edges
                .iter()
                .copied()
                .find(|x| {
                    body.partner(x.1)
                        .is_some_and(|p| in_face(body, body.coedge_face(p), ahead))
                })
                .ok_or(err.clone())?;
            let g = body.coedge(c).edge;
            let gx = body.edge(g);
            let at = crossing(&springs[i], hi, &gx.curve, gx.range).unwrap_or(hi);
            let foot = springs[i].eval(at);
            if gx.curve.eval(gx.curve.closest(foot, gx.range)).distance(foot) > tol::LINEAR {
                return Err(err);
            }
            let across = body.coedge_face(body.partner(c).ok_or(err.clone())?);
            let normal = |f: FaceId| body.face_normal(f, body.face(f).surface.project(foot));
            let (n0, n1) = normal(faces[i]).zip(normal(across)).ok_or(err.clone())?;
            if n0.cross(n1).length() * r > tol::LINEAR || n0.dot(n1.get()) <= 0.0 {
                return Err(err);
            }
            let centre = blend.spine.eval(at);
            let tangent = blend.spine.deriv(at).d1;
            stretches.push((blend.clone(), springs.clone(), t_start, at));
            crossed.push((g, c, i, foot));
            off[i] *= sense(faces[i]) * sense(across);
            faces[i] = across;
            let s = |f: FaceId| &body.face(f).surface;
            blend =
                Blend::new(s(faces[0]), s(faces[1]), off[0], off[1], centre, region).ok_or(err.clone())?;
            springs = [
                blend.spring(true).ok_or(err.clone())?,
                blend.spring(false).ok_or(err.clone())?,
            ];
            t = spine_closest(&blend.spine, centre);
            if blend.spine.eval(t).distance(centre) > tol::LINEAR {
                return Err(err);
            }
            up = blend.spine.deriv(t).d1.dot(tangent) > 0.0;
            t_start = t;
        }
        if faces != want {
            return Err(err);
        }
        // the edgeless stretches traced again, each over its own length: a trace's accuracy goes
        // with the size of the region it was asked about
        for s in stretches.iter_mut().skip(1) {
            let (c0, c1) = (s.0.spine.eval(s.2), s.0.spine.eval(s.3));
            let tight = Box3::from_points([c0, c1]).grow(4.0 * r + tol::LINEAR);
            let b = Blend::new(&s.0.a, &s.0.b, s.0.da, s.0.db, c0, tight).ok_or(err.clone())?;
            let t0 = spine_closest(&b.spine, c0);
            let t1 = t0 + along(spine_closest(&b.spine, c1) - t0, b.spine.domain().period);
            if b.spine.eval(t0).distance(c0) > tol::LINEAR || b.spine.eval(t1).distance(c1) > tol::LINEAR {
                return Err(err);
            }
            let sp = [
                b.spring(true).ok_or(err.clone())?,
                b.spring(false).ok_or(err.clone())?,
            ];
            *s = (b, sp, t0, t1);
        }
        // where the last stretch hands over to b's blend; with no crossing, at the vertex
        let last = crossed.len();
        let at_b = if last == 0 {
            blend.spine.eval(t_v)
        } else {
            blend.spine.eval(t)
        };
        let tb = spine_closest(&self.rolled[rb].blend.spine, at_b);
        if self.rolled[rb].blend.spine.eval(tb).distance(at_b) > tol::LINEAR {
            return Err(err);
        }
        // the sections, in order: (blend on its start side, t on it, its two feet by a's side)
        let n_sec = last.max(1);
        let sec_of = |j: usize| -> (&Blend, f64, &[Curve; 2]) {
            if last == 0 {
                (&self.rolled[ra].blend, t_v, &self.rolled[ra].springs)
            } else {
                let s = &stretches[j];
                (&s.0, s.3, &s.1)
            }
        };
        let name = self
            .root
            .with_str("walk")
            .with_name(body.vertex(v).name)
            .with_name(self.rolled[ra].base)
            .with_name(self.rolled[rb].base);
        let mut plan: Vec<([Point3; 2], (forge_geom::Circle, f64))> = Vec::with_capacity(n_sec);
        for j in 0..n_sec {
            let (b, u, sp) = sec_of(j);
            // the section as the stretch with no edge of its own traces it, so that it lies on that
            // stretch's face, which has only its own trace to answer to
            let own = if j >= 1 {
                Some((&stretches[j].0, stretches[j].3))
            } else if last > 1 {
                Some((&stretches[1].0, stretches[1].2))
            } else {
                None
            };
            let (from, at) = own.unwrap_or((b, u));
            plan.push((
                [sp[0].eval(u), sp[1].eval(u)],
                from.section(at).ok_or(err.clone())?,
            ));
        }
        let mut nodes: Vec<[VertexId; 2]> = Vec::with_capacity(n_sec);
        let mut secs = Vec::with_capacity(n_sec);
        for (j, (feet, (circle, angle))) in plan.into_iter().enumerate() {
            let n = name.with(j as u64);
            let v0 = self.body.add_vertex(feet[0], n.with_str("a"));
            let v1 = self.body.add_vertex(feet[1], n.with_str("b"));
            let e = self.body.add_edge(
                Curve::Circle(circle),
                Interval::new(0.0, angle),
                v0,
                v1,
                n.with_str("s"),
            );
            nodes.push([v0, v1]);
            secs.push(e);
        }
        let px = sa;
        let p_to_x = sa == 0;
        // the stretches between sections: faces walk their springs, a patch takes the blend
        for j in 1..last {
            let (b, sp, _, t1) = &stretches[j];
            let t0 = stretches[j].2;
            let (fp, fx) = (
                self.stretch_face(&crossed, j, px, ra),
                self.stretch_face(&crossed, j, 1 - px, ra),
            );
            let n = name.with(j as u64);
            let (sc, surface) = (sp.clone(), b.surface.clone());
            let wp = self.edge_between(
                sc[px].clone(),
                t0,
                *t1,
                nodes[j - 1][px],
                nodes[j][px],
                n.with_str("p"),
            );
            let wx = self.edge_between(
                sc[1 - px].clone(),
                t0,
                *t1,
                nodes[j - 1][1 - px],
                nodes[j][1 - px],
                n.with_str("x"),
            );
            self.extra.entry(fp).or_default().push(wp);
            self.extra.entry(fx).or_default().push((wx.0, !wx.1));
            self.rechain.insert(fp);
            self.rechain.insert(fx);
            self.touched.insert(fp);
            self.touched.insert(fx);
            let uses = vec![(wp.0, !wp.1), (secs[j - 1], p_to_x), wx, (secs[j], !p_to_x)];
            let convex = self.rolled[ra].convex;
            self.patches.push((surface, convex, n.with_str("patch"), uses));
        }
        // the blends' ends
        let t_a = if last == 0 { t_v } else { stretches[0].3 };
        let mut end_a = SpringEnd {
            v: [v; 2],
            t: [t_a; 2],
            path: vec![(secs[0], !p_to_x)],
        };
        end_a.v[sa] = nodes[0][sa];
        end_a.v[1 - sa] = nodes[0][1 - sa];
        let mut end_b = SpringEnd {
            v: [v; 2],
            t: [tb; 2],
            path: vec![(secs[n_sec - 1], p_to_x)],
        };
        end_b.v[sb] = nodes[n_sec - 1][sa];
        end_b.v[1 - sb] = nodes[n_sec - 1][1 - sa];
        self.ends.insert((ea, self.arrives_at_end(ring[ia].cin)), end_a);
        self.ends.insert((eb, self.arrives_at_end(ring[ib].cin)), end_b);
        let rests: Vec<((usize, usize), EdgeId)> = [(ra, 0), (ra, 1), (rb, 0), (rb, 1)]
            .into_iter()
            .filter_map(|(ri, s)| self.rolled[ri].over[s].map(|o| ((ri, s), o.1)))
            .collect();
        let mut ended: BTreeSet<(usize, usize)> = BTreeSet::new();
        let spines: Vec<Curve> = stretches
            .iter()
            .map(|x| x.0.spine.clone())
            .chain([
                self.rolled[ra].blend.spine.clone(),
                self.rolled[rb].blend.spine.clone(),
            ])
            .collect();
        // a point the ball passes over
        let under = |q: Point3| {
            spines
                .iter()
                .any(|sp| sp.eval(spine_closest(sp, q)).distance(q) <= 2.0 * r)
        };
        // each edge a foot crossed is cut there, what lies behind the foot going: a face walks the
        // springs on its P side from a to b, and on its X side from b to a
        let mut queue = vec![ring[0].cin];
        let mut seen = BTreeSet::from([v]);
        let mut cut = BTreeSet::new();
        for (j, &(g, c, i, foot)) in crossed.iter().enumerate() {
            let node = nodes[j][i];
            let behind = if i == px {
                self.body.partner(c).ok_or(err.clone())?
            } else {
                c
            };
            cut.insert(g);
            for &(rs, _) in rests.iter().filter(|x| x.1 == g) {
                let x = self.body.edge(g);
                let t = x.curve.closest(foot, x.range);
                self.g_ends.entry(rs).or_default().push(GEnd::Cut(t, node));
                ended.insert(rs);
            }
            if rests.iter().all(|x| x.1 != g) {
                self.trim(behind, foot, node, ea, None)?;
            }
            // (an edge cut twice runs on behind this foot to the other cut, not to its own end)
            if under(self.body.vertex(self.body.coedge_end(behind)).point)
                && seen.insert(self.body.coedge_end(behind))
            {
                queue.push(behind);
            }
        }
        // what lay under them: from v outwards, any edge not cut goes whole
        while let Some(arrive) = queue.pop() {
            if seen.len() > 32 {
                return Err(err);
            }
            let around = self.corners(arrive).ok_or(err.clone())?;
            for corner in around {
                let c = corner.cin;
                let e = self.body.coedge(c).edge;
                self.rechain.insert(corner.face);
                self.touched.insert(corner.face);
                if e == ea || e == eb || cut.contains(&e) {
                    continue;
                }
                if self.which.contains_key(&e) {
                    return Err(err);
                }
                let w = self.body.coedge_end(c);
                if let Some(&(rs, _)) = rests.iter().find(|x| x.1 == e) {
                    self.g_ends.entry(rs).or_default().push(GEnd::Gone(w));
                    ended.insert(rs);
                    continue;
                }
                let far = self.body.partner(c).ok_or(err.clone())?;
                // running on past the ball: cut by the walk at its other end, or the check refuses
                if !under(self.body.vertex(self.body.coedge_end(far)).point) {
                    continue;
                }
                self.drop_whole(e);
                if seen.insert(self.body.coedge_end(far)) {
                    queue.push(far);
                }
            }
        }
        // an edge rested beyond that this end did not reach: its strip ends at its nearer vertex
        for &(rs, g) in &rests {
            if !ended.contains(&rs) {
                let x = self.body.edge(g);
                let w = [x.start, x.end]
                    .into_iter()
                    .min_by(|&p, &q| {
                        let d = |u: VertexId| self.body.vertex(u).point.distance(vp);
                        d(p).total_cmp(&d(q))
                    })
                    .ok_or(err.clone())?;
                self.g_ends.entry(rs).or_default().push(GEnd::Gone(w));
            }
        }
        self.unsure = true;
        Ok(())
    }

    /// The face under side `side`'s foot along stretch `j`: the blend's own, past each crossing.
    fn stretch_face(
        &self,
        crossed: &[(EdgeId, CoedgeId, usize, Point3)],
        j: usize,
        side: usize,
        ra: usize,
    ) -> FaceId {
        let ro = &self.rolled[ra];
        let mut f = ro.over[side].map_or(ro.faces[side], |o| o.0);
        for x in &crossed[..j] {
            if x.2 == side {
                f = self.body.partner(x.1).map_or(f, |p| self.body.coedge_face(p));
            }
        }
        f
    }

    /// The blend beside pivot `p` ends at `v`, an end of the arc: at the section through the
    /// pivot's ball, which is the sphere's edge. Its foot on the wall cuts the smooth edge there.
    fn pivot_end(&mut self, v: VertexId, ring: &[Corner], p: usize, hot: usize) -> R<()> {
        let err = FilletError::Corner { vertex: v };
        let k = ring.len();
        let (arc, centre, foot, top) = {
            let x = &self.pivots[p];
            (x.edge, x.centre, x.foot, x.top)
        };
        if k != 3 {
            return Err(err);
        }
        let ri = self.filleted(ring[hot].cin).expect("hot");
        let edge = self.rolled[ri].edge;
        let t = spine_closest(&self.rolled[ri].blend.spine, centre);
        if self.rolled[ri].blend.spine.eval(t).distance(centre) > tol::LINEAR {
            return Err(err);
        }
        // the third edge at v, smooth, between the wall and the blend's other face
        let third = (0..k)
            .map(|j| ring[j].cin)
            .find(|&c| {
                let e = self.body.coedge(c).edge;
                e != arc && e != edge
            })
            .ok_or(err.clone())?;
        let s_in = self.side(ring[hot].cin);
        let (fa, fb) = self.rolled[ri].blend.feet(t);
        let (on_0, on_1) = (fa, fb);
        // which foot is on the top
        let top_side = usize::from(self.rolled[ri].faces[0] != top);
        if self.rolled[ri].faces[top_side] != top {
            return Err(err);
        }
        let (low_p, foot_p) = if top_side == 0 { (on_1, on_0) } else { (on_0, on_1) };
        if foot_p.distance(foot) > tol::LINEAR {
            return Err(err);
        }
        let vname = self.body.vertex(v).name;
        let base = self.rolled[ri].base;
        let pole = match self.pivots[p].pole {
            Some(w) => w,
            None => {
                let n = self.pivots[p].name;
                let w = self.body.add_vertex(foot, n.with_str("pole"));
                self.pivots[p].pole = Some(w);
                w
            }
        };
        let low = self.body.add_vertex(
            low_p,
            Name::NONE.with_set(&[base, self.pivots[p].name]).with_name(vname),
        );
        self.trim(third, low_p, low, edge, None)?;
        let x = self.body.edge(arc);
        let at_arc_end = x.end == v;
        self.pivots[p].low[usize::from(at_arc_end)] = Some(low);
        let (circle, angle) = self.rolled[ri].blend.section(t).ok_or(err.clone())?;
        // the section runs from the foot on the blend's first surface
        let (from, to) = if top_side == 0 { (pole, low) } else { (low, pole) };
        let name = Name::NONE
            .with_set(&[base, self.pivots[p].name])
            .with_str("section")
            .with_name(vname);
        let sec = self
            .body
            .add_edge(Curve::Circle(circle), Interval::new(0.0, angle), from, to, name);
        self.pivots[p].sections[usize::from(at_arc_end)] = Some(sec);
        // the blend walks it from its leaving side to its arriving side
        let mut end = SpringEnd {
            v: [low; 2],
            t: [t; 2],
            path: Vec::new(),
        };
        end.v[top_side] = pole;
        let arrive = end.v[s_in];
        end.path = vec![(sec, self.body.edge(sec).end == arrive)];
        self.ends.insert((edge, self.arrives_at_end(ring[hot].cin)), end);
        Ok(())
    }

    /// Takes each pivot's arc out and closes it: on the wall a circle a radius below, on the ball
    /// a sphere bounded by that and the sections beside it. A whole rim takes its plane away.
    fn close_pivots(&mut self) -> R<()> {
        for x in std::mem::take(&mut self.pivots) {
            let err = FilletError::Overflow { edge: x.edge };
            let a = self.body.edge(x.edge).clone();
            let [c0, c1] = a.coedges[..] else { return Err(err) };
            let (ct, cw) = if self.body.coedge_face(c0) == x.top {
                (c0, c1)
            } else {
                (c1, c0)
            };
            let Curve::Circle(k) = &a.curve else {
                return Err(err);
            };
            let drop = x.centre - x.foot;
            let circle = forge_geom::Circle {
                frame: k.frame.with_origin(k.frame.origin + drop),
                radius: k.radius,
            };
            let closed = a.start == a.end;
            let mut low = x.low;
            if closed {
                // the seam is cut where the rim's new circle crosses it
                let lp = self.body.lp(self.body.coedge(cw).lp);
                let seam = lp
                    .coedges()
                    .find(|&c| self.body.coedge(c).edge != x.edge && self.body.coedge_end(c) == a.start)
                    .ok_or(err.clone())?;
                let q = circle.eval(a.range.lo);
                let w = self.body.add_vertex(q, x.name.with_str("cut"));
                self.trim(seam, q, w, x.edge, None)?;
                low = [Some(w), Some(w)];
                // nothing else may stand on the plane the ball takes away
                if self.body.face(x.top).loops.len() != 1
                    || self.body.lp(self.body.coedge(ct).lp).coedges().count() != 1
                {
                    return Err(err);
                }
                self.gone_faces.push(x.top);
            }
            let (Some(l0), Some(l1)) = (low[0], low[1]) else {
                return Err(err);
            };
            let spring =
                self.body
                    .add_edge(Curve::Circle(circle), a.range, l0, l1, x.name.with_str("spring"));
            let fw = self.body.coedge(cw).forward;
            let before = self.prev_in_loop(cw);
            self.paths.entry(before).or_default().push((spring, fw));
            self.dropped.insert(ct);
            self.dropped.insert(cw);
            self.touched.insert(x.top);
            self.touched.insert(x.wall);
            let mut uses = vec![(spring, !fw)];
            if !closed {
                let (Some(s0), Some(s1), Some(pole)) = (x.sections[0], x.sections[1], x.pole) else {
                    return Err(err);
                };
                // the walk on the sphere: against the wall's, then up one section and down the other
                let (into, out) = if fw { (s0, s1) } else { (s1, s0) };
                uses.push((into, self.body.edge(into).end == pole));
                uses.push((out, self.body.edge(out).start == pole));
            }
            let frame = Frame::from_z(x.centre, k.frame.z());
            let mut frame = frame;
            if closed {
                // the cap is bounded in its parameters by a seam from the rim to its pole
                let q = circle.eval(a.range.lo);
                let (out, up) = (q - x.centre, x.foot - x.centre);
                let z = out.cross(up).normalize().ok_or(err.clone())?;
                let pole = self.body.add_vertex(x.foot, x.name.with_str("pole"));
                let seam = Frame::from_zx(x.centre, z, out).ok_or(err.clone())?;
                let meridian = forge_geom::Circle {
                    frame: seam,
                    radius: x.radius,
                };
                let edge = self.body.add_edge(
                    Curve::Circle(meridian),
                    Interval::new(0.0, dmath::FRAC_PI_2),
                    low[0].ok_or(err.clone())?,
                    pole,
                    x.name.with_str("seam"),
                );
                uses.extend([(edge, true), (edge, false)]);
                frame = Frame::from_zx(x.centre, k.frame.z(), out).ok_or(err.clone())?;
            }
            self.patches.push((
                Surface::Sphere(Sphere {
                    frame,
                    radius: x.radius,
                }),
                x.convex,
                x.name,
                uses,
            ));
        }
        Ok(())
    }

    /// Whether two blends at `v` simply cut each other: three faces there, the third edge left
    /// sharp, both blends of one kind and rolling on their own faces. A corner set back is patched.
    fn mitres(&self, v: VertexId, ring: &[Corner], a: usize, b: usize) -> bool {
        let (ra, rb) = (
            &self.rolled[self.filleted(ring[a].cin).expect("hot")],
            &self.rolled[self.filleted(ring[b].cin).expect("hot")],
        );
        let plain = |r: &Rolled| r.var.is_none() && r.over.iter().all(Option::is_none);
        ring.len() == 3
            && ra.convex == rb.convex
            && plain(ra)
            && plain(rb)
            && !self.setbacks.iter().any(|s| s.0 == v)
    }

    /// Two blends at `v` beside a sharp edge: each is carried on until it meets the other. The
    /// springs on the face between them meet at P, and from there the blends cut each other. Their
    /// other springs reach the sharp edge at Q; where those two differ, the blend reaching
    /// further cuts the other's face from its Q to X, the other's spring there, and the mitre runs
    /// from X to P. Where they agree X is Q.
    fn mitre(&mut self, v: VertexId, ring: &[Corner], a: usize, b: usize) -> R<()> {
        let err = FilletError::Corner { vertex: v };
        // ring b follows ring a: face a lies between their edges
        let (a, b) = if (a + 1) % 3 == b { (a, b) } else { (b, a) };
        let c = (b + 1) % 3;
        let vp = self.body.vertex(v).point;
        let (ra, rb) = (
            self.filleted(ring[a].cin).expect("hot"),
            self.filleted(ring[b].cin).expect("hot"),
        );
        let (ea, eb) = (self.rolled[ra].edge, self.rolled[rb].edge);
        let (sa, sb) = (self.side(ring[a].cin), self.side(ring[b].cin));
        let reach = self.body.edge_box(ea).hull(&self.body.edge_box(eb)).diagonal()
            + 4.0 * self.rolled[ra].blend.radius.max(self.rolled[rb].blend.radius);
        let (ta0, tb0) = (self.end_t(ra, ring[a].cin, vp), self.end_t(rb, ring[b].cin, vp));
        let (ta, tb, p) = springs_meet(
            &self.rolled[ra].springs[sa],
            ta0,
            &self.rolled[rb].springs[1 - sb],
            tb0,
        )
        .ok_or(err.clone())?;
        // the far ends: blend a's on face c, blend b's on face b, each at the other's face
        let hit = |ri: usize, s: usize, across: FaceId, t: f64| {
            let spring = &self.rolled[ri].springs[s];
            nearest_hit(
                spring,
                search_window(spring, t, reach),
                &self.body.face(across).surface,
                vp,
            )
        };
        let (qa_t, qa) = hit(ra, 1 - sa, ring[b].face, ta0).ok_or(err.clone())?;
        let (qb_t, qb) = hit(rb, sb, ring[c].face, tb0).ok_or(err.clone())?;
        let names = |me: &Self, j: usize, w: usize, base: Name| {
            me.vertex_name(v, &[me.face_name(ring[j].face), me.face_name(ring[w].face), base])
        };
        let (na, nb) = (self.rolled[ra].base, self.rolled[rb].base);
        let vp_name = self.vertex_name(
            v,
            &[self.face_name(ring[a].face), self.face_name(ring[b].face), na, nb],
        );
        let pv = self.body.add_vertex(p, vp_name);
        let on_sharp = |me: &Self, q: Point3| {
            let e = me.body.edge(me.body.coedge(ring[c].cin).edge);
            e.curve.eval(e.curve.closest(q, e.range)).distance(vp) + vp.distance(q)
        };
        // which blend reaches further down the sharp edge; X on the other's spring
        let same = qa.distance(qb) <= tol::LINEAR;
        let a_far = on_sharp(self, qa) > on_sharp(self, qb);
        let q = if a_far { qa } else { qb };
        let qv = self
            .body
            .add_vertex(q, names(self, b, c, if a_far { na } else { nb }));
        self.trim(ring[c].cin, q, qv, ea, None)?;
        let (x, xv, ta_far, tb_far) = if same {
            (q, qv, qa_t, qb_t)
        } else if a_far {
            // on blend b's spring on face b
            let surf = self.rolled[ra].blend.surface.clone();
            let spring = &self.rolled[rb].springs[sb];
            let (t, x) =
                nearest_hit(spring, search_window(spring, tb0, reach), &surf, q).ok_or(err.clone())?;
            let xv = self.body.add_vertex(x, names(self, b, a, na));
            (x, xv, qa_t, t)
        } else {
            let surf = self.rolled[rb].blend.surface.clone();
            let spring = &self.rolled[ra].springs[1 - sa];
            let (t, x) =
                nearest_hit(spring, search_window(spring, ta0, reach), &surf, q).ok_or(err.clone())?;
            let xv = self.body.add_vertex(x, names(self, c, a, nb));
            (x, xv, t, qb_t)
        };
        // the mitre, from X to P
        let (curve, t0, t1) =
            blends_meet(&self.rolled[ra].blend, &self.rolled[rb].blend, x, p).ok_or(err.clone())?;
        let mname = Name::NONE
            .with_set(&[na, nb])
            .with_str("mitre")
            .with_name(self.body.vertex(v).name);
        let (m, m_fwd) = self.edge_between(curve, t0, t1, xv, pv, mname);
        let mut path_a = Vec::new();
        let mut path_b = vec![(m, !m_fwd)];
        if !same {
            // the far blend's cut of the near blend's face, from Q to X
            let (ri, face, base) = if a_far {
                (ra, ring[b].face, na)
            } else {
                (rb, ring[c].face, nb)
            };
            let (curve, t0, t1) = cross_curve(&self.rolled[ri].blend, &self.body.face(face).surface, q, x)
                .or_else(|| {
                    let b = &self.rolled[ri].blend;
                    let c = forge_geom::intcurve::towards_touch(
                        &b.surface,
                        &self.body.face(face).surface,
                        q,
                        x,
                        0.05 * b.radius,
                    )?;
                    let hi = c.domain_hi();
                    Some((Curve::Intersection(Arc::new(c)), 0.0, hi))
                })
                .ok_or(err.clone())?;
            let name = Name::NONE
                .with_set(&[base, self.face_name(face)])
                .with_name(self.body.vertex(v).name);
            let (cut, fwd) = self.edge_between(curve, t0, t1, qv, xv, name);
            self.touched.insert(face);
            if a_far {
                // face b comes down its spring to X and on to Q; blend a walks Q to X
                self.paths.insert(ring[b].cin, vec![(cut, !fwd)]);
                path_a.push((cut, fwd));
            } else {
                // face c comes up the sharp edge to Q and on to X; blend b walks X to Q
                self.paths.insert(ring[c].cin, vec![(cut, fwd)]);
                path_b.push((cut, !fwd));
            }
        }
        if !same {
            let (key, corner) = if a_far {
                ((rb, sb), qb_t)
            } else {
                ((ra, 1 - sa), qa_t)
            };
            self.overrun.entry(key).or_default().push(corner);
        }
        path_a.push((m, m_fwd));
        // blend a: its side on face a ends at P, its other at Q or X
        let mut end_a = SpringEnd {
            v: [pv; 2],
            t: [ta; 2],
            path: path_a,
        };
        (end_a.v[1 - sa], end_a.t[1 - sa]) = (if same || a_far { qv } else { xv }, ta_far);
        // blend b: its side on face b ends at Q or X, its other at P
        let mut end_b = SpringEnd {
            v: [pv; 2],
            t: [tb; 2],
            path: path_b,
        };
        (end_b.v[sb], end_b.t[sb]) = (if same || !a_far { qv } else { xv }, tb_far);
        self.ends.insert((ea, self.arrives_at_end(ring[a].cin)), end_a);
        self.ends.insert((eb, self.arrives_at_end(ring[b].cin)), end_b);
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
        // blends named as their edges' wholes, unless two here are pieces of one
        let mut bases: Vec<Name> = blend_ring
            .iter()
            .filter_map(|&j| self.filleted(ring[j].cin).map(|ri| self.rolled[ri].base))
            .collect();
        bases.sort_unstable();
        let whole = bases.windows(2).all(|w| w[0] != w[1]);
        // and faces likewise
        let mut lines: Vec<Name> = ring.iter().map(|c| self.face_name(c.face)).collect();
        lines.sort_unstable();
        let whole_faces = lines.windows(2).all(|w| w[0] != w[1]);
        let fname = |b: &Self, f: FaceId| {
            if whole_faces {
                b.face_name(f)
            } else {
                b.body.face(f).name
            }
        };
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
            let fa = fname(self, ring[j].face);
            let fb = fname(self, ring[(j + k - 1) % k].face);
            let rn = if whole {
                self.rolled[ri].base
            } else {
                self.rolled[ri].name
            };
            let va = self.pooled(&mut pool, pa, Name::NONE.with_set(&[fa, rn]).with_name(vname));
            let vb = self.pooled(&mut pool, pb, Name::NONE.with_set(&[fb, rn]).with_name(vname));
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
                    let name = Name::NONE.with_set(&[fname(self, ring[j].face), patch_name]);
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
            // the section runs from the foot on the blend's side 0, or the other way round from
            // the other foot when that one comes first in (x, y, z): storage does not choose
            let (v0, v1) = if s_in == 0 { (va, vb) } else { (vb, va) };
            let (q0, q1) = (self.body.vertex(v0).point, self.body.vertex(v1).point);
            let (circle, v0, v1) = match (0..3).find(|&i| (q0[i] - q1[i]).abs() > 1e-9) {
                Some(i) if q1[i] < q0[i] => {
                    let f = &circle.frame;
                    let x = q1 - f.origin;
                    let frame = Frame::from_zx(f.origin, -f.z(), x).ok_or(err.clone())?;
                    (forge_geom::Circle { frame, ..circle }, v1, v0)
                }
                _ => (circle, v0, v1),
            };
            let name = Name::NONE.with_set(&[self.rolled[ri].name, patch_name]);
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
        let seams = if self.cross_seam {
            self.cross_seams()?
        } else {
            SeamCrossings::new()
        };
        let mut out = Vec::with_capacity(self.rolled.len());
        for i in 0..self.rolled.len() {
            let e = self.rolled[i].edge;
            let closed = self.body.edge(e).start == self.body.edge(e).end;
            let mut pair = Vec::with_capacity(2);
            for side in 0..2 {
                // a vertex no corner rule closed leaves the edge without an end
                let Some(((v0, t0), (v1, t1))) = self.spring_span(i, side) else {
                    return Err(FilletError::Overflow { edge: e });
                };
                let ro = &self.rolled[i];
                let curve = ro.springs[side].clone();
                if (!closed && curve.eval(t0).distance(curve.eval(t1)) <= tol::LINEAR) || t1 <= t0 {
                    return Err(FilletError::Overflow { edge: e });
                }
                let over = ro.over[side];
                let face = over.map_or(ro.faces[side], |o| o.0);
                let name = Name::NONE.with_set(&[ro.name, self.body.face(face).name]);
                // where the spring leaves its face and comes back, the ball rolls over what lies between
                let across = seams.get(&(i, side));
                // what a mitre carries on past the corner is off the face as it was, and meant to be
                let mut kept = Interval::new(t0, t1);
                for &q in self.overrun.get(&(i, side)).into_iter().flatten() {
                    if q > kept.lo && q < kept.hi {
                        if q - t0 < t1 - q {
                            kept.lo = q;
                        } else {
                            kept.hi = q;
                        }
                    }
                }
                let inside = across.is_some() || inside_face(&self.body, face, &curve, kept);
                let rim = (ro.cliff == Some(side) || !inside)
                    && along_rim(&self.body, face, &curve, Interval::new(t0, t1));
                self.collapsing |= rim;
                // the spring is off the face wherever the far edge it runs on stops short of its ends
                if ro.cliff == Some(side) && !rim {
                    return Err(FilletError::Overflow { edge: e });
                }
                let trips = if rim || inside {
                    Vec::new()
                } else if over.is_some() {
                    return Err(FilletError::Overflow { edge: e });
                } else {
                    self.excursions(i, side, Interval::new(t0, t1))?
                };
                if let Some(list) = across {
                    // its stretches, turn about on the face and on the one across each seam
                    let mut stops = vec![(v0, t0)];
                    stops.extend(list.iter().map(|&(t, v, _)| (v, t)));
                    stops.push((v1, t1));
                    let stretch = |k: usize| Interval::new(stops[k].1, stops[k + 1].1);
                    let starts_home = inside_face(&self.body, face, &curve, stretch(0));
                    let mut pieces = Vec::with_capacity(stops.len() - 1);
                    let mut mine = Vec::new();
                    let dir = self.rolled[i].forward[side] == self.rolled[i].aligned;
                    for k in 0..stops.len() - 1 {
                        let on_home = starts_home == (k % 2 == 0);
                        // the far face of the crossing bounding this stretch
                        let far = list[if k < list.len() { k } else { k - 1 }].2;
                        let on = if on_home { face } else { far };
                        if stretch(k).hi <= stretch(k).lo || !inside_face(&self.body, on, &curve, stretch(k))
                        {
                            return Err(FilletError::Overflow { edge: e });
                        }
                        let name = if k == 0 { name } else { name.with(k as u64) };
                        let piece =
                            self.body
                                .add_edge(curve.clone(), stretch(k), stops[k].0, stops[k + 1].0, name);
                        pieces.push(piece);
                        if on_home {
                            mine.push(piece);
                        } else {
                            self.extra.entry(far).or_default().extend(walk(&[piece], dir));
                            self.rechain.insert(far);
                            self.touched.insert(far);
                        }
                    }
                    self.rechain.insert(face);
                    pair.push(Springs {
                        pieces,
                        home: mine,
                        gaps: Vec::new(),
                    });
                    continue;
                }
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
                    home: pieces.clone(),
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
            let over = self.body.coedge(c_out).edge;
            if over == self.body.coedge(c_in).edge {
                let e = self.body.edge(over);
                let q = spring.eval(0.5 * (t_out + t_in));
                return Err(FilletError::SplitFirst {
                    edge: over,
                    at: e.curve.closest(q, e.range),
                });
            }
            let name = |b: &Builder, c: CoedgeId| {
                Name::NONE
                    .with_set(&[b.rolled[ri].name, b.body.edge(b.body.coedge(c).edge).name])
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

    /// Where a spring crosses an edge of its face with the same surface across it (a seam) it stays on
    /// that surface, and no surface says where. Those edges crossed an even number of times, by all the
    /// springs together, have what lies between each two crossings taken out: the faces either side
    /// open into each other there, each spring's stretches lying on the face they are on. Per spring,
    /// its crossings in order with their vertices. (An edge crossed once is left alone: a spring running
    /// a little past its face at a mitre, on the same function.)
    fn cross_seams(&mut self) -> R<SeamCrossings> {
        // per seam edge: parameter on it, then the spring, its parameter and the point
        let mut found: BTreeMap<EdgeId, Vec<SeamHit>> = BTreeMap::new();
        for i in 0..self.rolled.len() {
            for side in 0..2 {
                let ro = &self.rolled[i];
                if ro.over[side].is_some() || ro.cliff == Some(side) {
                    continue;
                }
                let Some(((_, t0), (_, t1))) = self.spring_span(i, side) else {
                    continue;
                };
                let (range, home) = (Interval::new(t0, t1), ro.faces[side]);
                let (fx, spring) = (self.body.face(home), &ro.springs[side]);
                let slack = 1e-6 * range.width();
                let reach = spring.bbox(range).grow(tol::LINEAR);
                for &l in &fx.loops {
                    for c in self.body.lp(l).coedges() {
                        let g = self.body.coedge(c).edge;
                        if !self.body.edge_box(g).overlaps(&reach) {
                            continue;
                        }
                        let Some(across) = self.body.partner(c).map(|p| self.body.coedge_face(p)) else {
                            continue;
                        };
                        if across == home
                            || self.dropped.contains(&c)
                            || self.which.contains_key(&g)
                            || !self.body.face(across).surface.one_with(&fx.surface)
                        {
                            continue;
                        }
                        let gx = self.body.edge(g);
                        // which side of g, in the face's tangent plane, where its nearest point is within it
                        let side_of = |t: f64| {
                            let p = spring.eval(t);
                            let u = gx.curve.closest(p, gx.range);
                            let d = gx.curve.deriv(u);
                            let n = self.body.face_normal(home, fx.surface.project(d.p))?;
                            Some((
                                (p - d.p).dot(n.get().cross(d.d1)),
                                u > gx.range.lo && u < gx.range.hi,
                            ))
                        };
                        let n = 128;
                        let at = |k: usize| range.lo + range.width() * k as f64 / n as f64;
                        let list = found.entry(g).or_default();
                        let mut last = side_of(at(0));
                        for k in 1..=n {
                            let now = side_of(at(k));
                            if let (Some((a, ia)), Some((b, ib))) = (last, now)
                                && a * b < 0.0
                                && (ia || ib)
                                && let Some(t) =
                                    crossing(spring, 0.5 * (at(k - 1) + at(k)), &gx.curve, gx.range)
                                && t > range.lo + slack
                                && t < range.hi - slack
                            {
                                let p = spring.eval(t);
                                list.push((gx.curve.closest(p, gx.range), i, side, t, p));
                            }
                            last = now;
                        }
                    }
                }
            }
        }
        let mut out: SeamCrossings = BTreeMap::new();
        for (g, mut list) in found {
            if list.is_empty()
                || list.len() % 2 == 1
                || self.trims.contains_key(&(g, true))
                || self.trims.contains_key(&(g, false))
            {
                continue;
            }
            list.sort_by(|a, b| a.0.total_cmp(&b.0));
            let gx = self.body.edge(g).clone();
            let mut verts = Vec::with_capacity(list.len());
            for (k, &(u, i, side, t, p)) in list.iter().enumerate() {
                let name = Name::NONE
                    .with_set(&[self.rolled[i].name, gx.name])
                    .with(k as u64);
                let v = self.body.add_vertex(p, name);
                verts.push((u, v));
                let across = gx
                    .coedges
                    .iter()
                    .map(|&c| self.body.coedge_face(c))
                    .find(|&f| f != self.rolled[i].faces[side])
                    .ok_or(FilletError::Overflow { edge: g })?;
                out.entry((i, side)).or_default().push((t, v, across));
            }
            // the seam keeps its first piece; each later one is a new edge, in both its faces
            self.trims.insert((g, true), (verts[0].1, verts[0].0));
            for (k, w) in verts[1..].chunks(2).enumerate() {
                let hi = w.get(1).map_or(gx.range.hi, |x| x.0);
                let end = w.get(1).map_or(gx.end, |x| x.1);
                let name = gx.name.with_str("seam").with(k as u64);
                let piece =
                    self.body
                        .add_edge(gx.curve.clone(), Interval::new(w[0].0, hi), w[0].1, end, name);
                for &c in &gx.coedges {
                    let f = self.body.coedge_face(c);
                    let use_ = (piece, self.body.coedge(c).forward);
                    self.extra.entry(f).or_default().push(use_);
                    self.rechain.insert(f);
                    self.touched.insert(f);
                }
            }
        }
        for list in out.values_mut() {
            list.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
        Ok(out)
    }

    /// Where spring `side` of blend `i` starts and stops, in its own direction.
    fn spring_span(&self, i: usize, side: usize) -> Option<((VertexId, f64), (VertexId, f64))> {
        let ro = &self.rolled[i];
        let (s, t) = (
            self.ends.get(&(ro.edge, false))?,
            self.ends.get(&(ro.edge, true))?,
        );
        let ((v0, t0), (v1, t1)) = if ro.aligned {
            ((s.v[side], s.t[side]), (t.v[side], t.t[side]))
        } else {
            ((t.v[side], t.t[side]), (s.v[side], s.t[side]))
        };
        let closed = self.body.edge(ro.edge).start == self.body.edge(ro.edge).end;
        let t1 = match ro.springs[side].domain().period {
            Some(p) => {
                let w = (t1 - t0).rem_euclid(p);
                if closed && w * ro.springs[side].deriv(t0).d1.length() <= tol::LINEAR {
                    t0 + p
                } else {
                    t0 + w
                }
            }
            None => t1,
        };
        Some(((v0, t0), (v1, t1)))
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
                    .with_set(&[name, self.body.edge(self.body.coedge(d).edge).name])
                    .with(path.len() as u64);
                self.body.add_vertex(exit, vn)
            };
            let (curve, ta, tb) =
                cross_curve(&self.rolled[ri].blend, &self.body.face(f).surface, entry.0, exit)
                    .ok_or(err.clone())?;
            let ename = Name::NONE
                .with_set(&[name, self.face_name(f)])
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

    /// Takes edge `e` out of both its faces' loops.
    fn drop_whole(&mut self, e: EdgeId) {
        let cs: Vec<CoedgeId> = self.body.edge(e).coedges.iter().copied().collect();
        for c in cs {
            self.dropped.insert(c);
            self.touched.insert(self.body.coedge_face(c));
        }
    }

    /// Two vertices at one place become the one of the lesser name: which way round they came is
    /// the order of the edges, which names nothing.
    fn merge_by_name(&mut self, a: VertexId, b: VertexId) {
        let (na, nb) = (self.body.vertex(a).name, self.body.vertex(b).name);
        self.merged.push(if na < nb { (b, a) } else { (a, b) });
    }

    /// Makes each merged vertex the one it landed on.
    fn apply_merges(&mut self) {
        if self.merged.is_empty() {
            return;
        }
        let roots = merge_roots(&std::mem::take(&mut self.merged));
        for e in &mut self.body.edges {
            e.start = *roots.get(&e.start).unwrap_or(&e.start);
            e.end = *roots.get(&e.end).unwrap_or(&e.end);
        }
        self.collapsing = true;
    }

    /// Closed edges of one face lying along each other with their vertices in different places:
    /// each is cut at the other's, so that they run between the same two.
    fn align_rings(&mut self) {
        if !self.collapsing {
            return;
        }
        let faces: Vec<FaceId> = self.touched.iter().copied().collect();
        for f in faces {
            let all: Vec<EdgeId> = self
                .body
                .face(f)
                .loops
                .iter()
                .flat_map(|&l| self.body.lp(l).coedges())
                .map(|c| self.body.coedge(c).edge)
                .collect();
            // open edges along each other: their ends at one place are one vertex
            for (i, &a) in all.iter().enumerate() {
                for &b in &all[i + 1..] {
                    let (x, y) = (self.body.edge(a).clone(), self.body.edge(b).clone());
                    if x.start == x.end || y.start == y.end {
                        continue;
                    }
                    let p = x.curve.eval(x.range.mid());
                    if y.curve.eval(y.curve.closest(p, y.range)).distance(p) > tol::LINEAR {
                        continue;
                    }
                    let at = |b: &Body, v: VertexId| b.vertex(v).point;
                    for (u, w) in [
                        (x.start, y.start),
                        (x.start, y.end),
                        (x.end, y.start),
                        (x.end, y.end),
                    ] {
                        if u != w && at(&self.body, u).distance(at(&self.body, w)) <= tol::LINEAR {
                            self.merge_by_name(u, w);
                        }
                    }
                    self.apply_merges();
                }
            }
            let es: Vec<EdgeId> = all
                .iter()
                .copied()
                .filter(|&e| self.body.edge(e).start == self.body.edge(e).end)
                .collect();
            for (i, &a) in es.iter().enumerate() {
                for &b in &es[i + 1..] {
                    let (x, y) = (self.body.edge(a).clone(), self.body.edge(b).clone());
                    let p = x.curve.eval(x.range.mid());
                    if y.curve.eval(y.curve.closest(p, y.range)).distance(p) > tol::LINEAR
                        || x.start == y.start
                    {
                        continue;
                    }
                    let (va, vb) = (x.start, y.start);
                    let (pa, pb) = (self.body.vertex(va).point, self.body.vertex(vb).point);
                    if pa.distance(pb) <= tol::LINEAR {
                        self.merge_by_name(va, vb);
                        self.apply_merges();
                        continue;
                    }
                    let (ta, tb) = (y.curve.closest(pa, y.range), x.curve.closest(pb, x.range));
                    let other_a = split_edge(&mut self.body, a, tb);
                    let other_b = split_edge(&mut self.body, b, ta);
                    // the cut's new vertices are the ones already there
                    let (ma, mb) = (self.body.edge(other_a).start, self.body.edge(other_b).start);
                    self.merged.push((ma, vb));
                    self.merged.push((mb, va));
                    self.apply_merges();
                }
            }
        }
    }

    /// Faces left with no width, each edge of their loops lying along another of them (a ball as
    /// wide as the face, or two blends meeting along its middle), and those pairs.
    fn no_width(&self) -> Vec<(FaceId, Vec<(EdgeId, EdgeId)>)> {
        let mut out = Vec::new();
        if !self.collapsing {
            return out;
        }
        for &f in &self.touched {
            if !self.body.face(f).alive || self.gone_faces.contains(&f) {
                continue;
            }
            let es: Vec<EdgeId> = self
                .body
                .face(f)
                .loops
                .iter()
                .flat_map(|&l| self.body.lp(l).coedges())
                .map(|c| self.body.coedge(c).edge)
                .collect();
            let along = |a: EdgeId, b: EdgeId| {
                let (x, y) = (self.body.edge(a), self.body.edge(b));
                let same = (x.start == y.start && x.end == y.end) || (x.start == y.end && x.end == y.start);
                same && [0.25, 0.5, 0.75].iter().all(|&u| {
                    let p = x.curve.eval(x.range.lo + u * x.range.width());
                    y.curve.eval(y.curve.closest(p, y.range)).distance(p) <= tol::LINEAR
                })
            };
            let mut left = es.clone();
            let mut pairs = Vec::new();
            while let Some(a) = left.pop() {
                let Some(k) = left.iter().position(|&b| b != a && along(a, b)) else {
                    pairs.clear();
                    left.clear();
                    break;
                };
                let b = left.remove(k);
                pairs.push((a, b));
            }
            if !pairs.is_empty() {
                out.push((f, pairs));
            }
        }
        out
    }

    /// Takes each face of no width away: the other face along each pair of its edges takes the
    /// edge kept instead.
    fn collapse(&mut self, faces: Vec<(FaceId, Vec<(EdgeId, EdgeId)>)>) {
        for (f, pairs) in faces {
            // how face f walks each edge: the other face along the one going walks the other the same way
            let walks: BTreeMap<EdgeId, bool> = self
                .body
                .face(f)
                .loops
                .iter()
                .flat_map(|&l| self.body.lp(l).coedges())
                .map(|c| (self.body.coedge(c).edge, self.body.coedge(c).forward))
                .collect();
            self.body.kill_face(f);
            for (a, b) in pairs {
                let (keep, go) = if a.index() < b.index() { (a, b) } else { (b, a) };
                let way = walks[&keep];
                let owners: Vec<FaceId> = self
                    .body
                    .edge(go)
                    .coedges
                    .iter()
                    .map(|&c| self.body.coedge_face(c))
                    .collect();
                for g in owners {
                    let loops = self.body.face(g).loops.to_vec();
                    for l in loops {
                        let uses: Vec<(EdgeId, bool)> = self
                            .body
                            .lp(l)
                            .coedges()
                            .map(|c| {
                                let co = self.body.coedge(c);
                                if co.edge == go {
                                    (keep, way)
                                } else {
                                    (co.edge, co.forward)
                                }
                            })
                            .collect();
                        if uses.iter().any(|u| u.0 == keep) {
                            self.body.replace_loop(g, l, &uses);
                        }
                    }
                    self.body.face_changed(g);
                }
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
        for (e, at_end) in std::mem::take(&mut self.carried) {
            let &(v, _) = self
                .trims
                .get(&(e, at_end))
                .ok_or(FilletError::Overflow { edge: e })?;
            let p = self.body.vertex(v).point;
            let Curve::Intersection(ic) = &self.body.edge(e).curve else {
                return Err(FilletError::Overflow { edge: e });
            };
            let (on, before) = ic
                .carried_to(p, at_end)
                .ok_or(FilletError::Overflow { edge: e })?;
            let shift = before as f64;
            let edge = &mut self.body.edges[e.index()];
            edge.range = Interval::new(edge.range.lo + shift, edge.range.hi + shift);
            let hi = on.domain_hi();
            edge.curve = Curve::Intersection(Arc::new(on));
            if let Some(other) = self.trims.get_mut(&(e, !at_end)) {
                other.1 += shift;
            }
            self.trims.insert((e, at_end), (v, if at_end { hi } else { 0.0 }));
        }
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
        let cut: Vec<EdgeId> = self.trims.keys().map(|k| k.0).collect();
        for e in cut {
            let edge = self.body.edge(e);
            // cut back from both ends to one point: two blends meet exactly there, and the edge goes
            if self
                .body
                .vertex(edge.start)
                .point
                .distance(self.body.vertex(edge.end).point)
                <= tol::LINEAR
            {
                let (from, to) = (edge.end, edge.start);
                self.merge_by_name(from, to);
                self.drop_whole(e);
                continue;
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
                        // an edge a cut landed on the far end of: the cut still leaves from there
                        if let Some(p) = self.paths.get(&c) {
                            uses.extend(p.iter().copied());
                        }
                        continue;
                    }
                    let co = self.body.coedge(c).clone();
                    match self.which.get(&co.edge) {
                        Some(&i) => {
                            let s = usize::from(self.rolled[i].coedges[0] != c);
                            // a strip the ball rolled past goes; its spring is on the face beyond
                            if self.rolled[i].over[s].is_none() {
                                uses.extend(walk(&springs[i][s].home, co.forward == self.rolled[i].aligned));
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
                } else if !uses.is_empty() {
                    self.body.add_loop(f, &uses);
                }
            }
            if let Some(x) = self.extra.get(&f) {
                pending.extend(x.iter().copied());
            }
            let chained = chain_loops(&self.body, &pending);
            if self.rechain.contains(&f) && chained.is_empty() {
                self.gone_faces.push(f);
            }
            for lp in chained {
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

/// A spring over a seam: the seam's parameter, the blend, its side, the spring's parameter, the point.
type SeamHit = (f64, usize, usize, f64, Point3);

/// Per spring (blend, side), where it crosses a seam: its parameter, the new vertex, the face across.
type SeamCrossings = BTreeMap<(usize, usize), Vec<(f64, VertexId, FaceId)>>;

/// A spring's edges in its parameter's order, and between each two the path the blend
/// takes (walking the spring with its parameter) where the spring was off its face.
struct Springs {
    pieces: Vec<EdgeId>,
    /// Those on the blend's own face (the others lie across a seam of it).
    home: Vec<EdgeId>,
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
        // back at the start with a way on still unused (a closed rim, and the seam leaving its
        // vertex): the loop goes on
        while at != start || (0..n).any(|j| !used[j] && ends(&uses[j]).0 == at) {
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

/// Each vertex of `merges` (a pair: the first becomes the second) and the one it ends as. Union-find,
/// so pairs naming each other, however they chain, cannot go round for ever.
fn merge_roots(merges: &[(VertexId, VertexId)]) -> BTreeMap<VertexId, VertexId> {
    let mut parent: BTreeMap<VertexId, VertexId> = BTreeMap::new();
    let root = |parent: &BTreeMap<VertexId, VertexId>, mut v: VertexId| {
        while let Some(&p) = parent.get(&v) {
            v = p;
        }
        v
    };
    for &(a, b) in merges {
        let (ra, rb) = (root(&parent, a), root(&parent, b));
        if ra != rb {
            parent.insert(ra, rb);
        }
    }
    let all: Vec<VertexId> = parent.keys().copied().collect();
    all.into_iter().map(|v| (v, root(&parent, v))).collect()
}

/// Whether an edge of another of a face's loops passes from one side of a new edge to the other, by
/// more than the tolerance.
fn spring_crosses_loop(body: &Body, f: FaceId, from: usize) -> bool {
    let loops = &body.face(f).loops;
    let edges_of = |l| body.lp(l).coedges().map(|c| body.coedge(c).edge);
    let surface = &body.face(f).surface;
    loops.iter().any(|&l| {
        edges_of(l).filter(|e| e.index() >= from).any(|a| {
            let ax = body.edge(a);
            // a rough box for a traced curve: an exact one is dear, and gone with the rebuild
            let ab = match ax.curve {
                Curve::Line(_) | Curve::Circle(_) => body.edge_box(a).grow(2.0 * tol::LINEAR),
                _ => {
                    let pts = (0..=8).map(|i| ax.curve.eval(ax.range.lo + ax.range.width() * i as f64 / 8.0));
                    let rough = Box3::from_points(pts);
                    rough.grow(0.25 * rough.diagonal() + 2.0 * tol::LINEAR)
                }
            };
            loops.iter().filter(|&&o| o != l).any(|&o| {
                edges_of(o).any(|b| {
                    if !ab.overlaps(&body.edge_box(b)) {
                        return false;
                    }
                    let bx = body.edge(b);
                    // on a flat a line's or circle's side of a point is a sum or two
                    let plane = match surface {
                        Surface::Plane(_) => body.face_normal(f, forge_math::Point2::ORIGIN).map(|n| n.get()),
                        _ => None,
                    };
                    let height = |p: Point3| {
                        if let (Some(n), Curve::Line(k)) = (plane, &ax.curve) {
                            let o = ax.curve.eval(ax.range.lo);
                            let along = (p - o).dot(k.dir.get());
                            let long = ax.range.width();
                            return (along > 1e-9 && along < long - 1e-9)
                                .then(|| (p - o).dot(n.cross(k.dir.get())));
                        }
                        let u = ax.curve.closest(p, ax.range);
                        if u <= ax.range.lo + 1e-9 || u >= ax.range.hi - 1e-9 {
                            return None;
                        }
                        let d = ax.curve.deriv(u);
                        let n = match plane {
                            Some(n) => n,
                            None => body.face_normal(f, surface.project(d.p))?.get(),
                        };
                        let across = n.cross(d.d1).normalize()?;
                        Some((p - d.p).dot(across.get()))
                    };
                    let at = |i: usize, n: usize| {
                        bx.curve
                            .eval(bx.range.lo + bx.range.width() * i as f64 / n as f64)
                    };
                    // from one side of the new edge to the other between neighbouring points, near it
                    let scan = |n: usize| {
                        let (mut crossed, mut least, mut last) = (false, f64::INFINITY, None);
                        for i in 0..=n {
                            let p = at(i, n);
                            let h = height(p);
                            if let (Some((q, was)), Some(now)) = (last, h) {
                                let q: Point3 = q;
                                let was: f64 = was;
                                crossed |= was * now < 0.0
                                    && was.abs().min(now.abs()) > tol::LINEAR
                                    && was.abs() + now.abs() <= 1.5 * q.distance(p);
                            }
                            if let Some(now) = h {
                                least = least.min(now.abs());
                            }
                            last = h.map(|now| (p, now));
                        }
                        (crossed, least)
                    };
                    // the height moves no faster than the curve: far off at a coarse row, it never comes
                    let (crossed, least) = scan(8);
                    let step = at(0, 8).distance(at(1, 8)).max(at(3, 8).distance(at(4, 8)));
                    crossed || (least <= 2.0 * step && scan(64).0)
                })
            })
        })
    })
}

/// Whether two edges of a face's loops that share no vertex stand in each other's boxes: only
/// then can they have touched or passed.
fn loops_near(body: &Body, f: FaceId) -> bool {
    let es: Vec<EdgeId> = body
        .face(f)
        .loops
        .iter()
        .flat_map(|&l| body.lp(l).coedges())
        .map(|c| body.coedge(c).edge)
        .collect();
    es.iter().enumerate().any(|(i, &a)| {
        let (ax, ab) = (body.edge(a), body.edge_box(a).grow(2.0 * tol::LINEAR));
        es[i + 1..].iter().any(|&b| {
            let bx = body.edge(b);
            a != b
                && ab.overlaps(&body.edge_box(b))
                && ![bx.start, bx.end].iter().any(|&w| w == ax.start || w == ax.end)
        })
    })
}

/// Whether two edges of a face's loops that share no vertex come within tolerance of each other:
/// springs just touching leave a face pinched at a point, which no chord test sees.
fn loops_touch(body: &Body, f: FaceId) -> bool {
    let es: Vec<EdgeId> = body
        .face(f)
        .loops
        .iter()
        .flat_map(|&l| body.lp(l).coedges())
        .map(|c| body.coedge(c).edge)
        .collect();
    let near = |a: EdgeId, b: EdgeId| {
        let (x, y) = (body.edge(a), body.edge(b));
        let gap = |t: f64| {
            let p = x.curve.eval(t);
            y.curve.eval(y.curve.closest(p, y.range)).distance(p)
        };
        // the least along a, from the best of a coarse row by golden section
        let n = 32;
        let w = x.range.width() / n as f64;
        let best = (0..=n)
            .map(|i| {
                let t = x.range.lo + i as f64 * w;
                (t, gap(t))
            })
            .min_by(|p, q| p.1.total_cmp(&q.1))
            .map_or(x.range.lo, |b| b.0);
        let (mut lo, mut hi) = ((best - w).max(x.range.lo), (best + w).min(x.range.hi));
        let phi = 0.618_033_988_749_894_8;
        for _ in 0..32 {
            let (c, d) = (hi - phi * (hi - lo), lo + phi * (hi - lo));
            if gap(c) < gap(d) {
                hi = d;
            } else {
                lo = c;
            }
        }
        gap(0.5 * (lo + hi)) <= 2.0 * tol::LINEAR
    };
    es.iter().enumerate().any(|(i, &a)| {
        let (ax, ab) = (body.edge(a), body.edge_box(a).grow(2.0 * tol::LINEAR));
        es[i + 1..].iter().any(|&b| {
            let bx = body.edge(b);
            a != b
                && ab.overlaps(&body.edge_box(b))
                && ![bx.start, bx.end].iter().any(|&w| w == ax.start || w == ax.end)
                && near(a, b)
        })
    })
}

/// Whether the spring runs along edges of its face (a ball as wide as the face rests on its rim).
fn along_rim(body: &Body, f: FaceId, c: &Curve, range: Interval) -> bool {
    let near = |p: Point3| {
        body.face(f)
            .loops
            .iter()
            .flat_map(|&l| body.lp(l).coedges())
            .any(|k| {
                let x = body.edge(body.coedge(k).edge);
                x.curve.eval(x.curve.closest(p, x.range)).distance(p) <= tol::LINEAR
            })
    };
    [0.2, 0.5, 0.8]
        .iter()
        .all(|&u| near(c.eval(range.lo + u * range.width())))
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
            None if on_contact_arc(blend, c.eval(0.5 * (ta + tb))) => return Some((c, ta, tb)),
            // a chain through a touch point, gone round the far way: walked directly
            None => {
                let walk =
                    |x, p| forge_geom::intcurve::towards_touch(&blend.surface, s, x, p, 0.05 * blend.radius);
                let curve = |c| Curve::Intersection(std::sync::Arc::new(c));
                return walk(a, b)
                    .map(|c| {
                        let hi = c.domain_hi();
                        (curve(c), 0.0, hi)
                    })
                    .or_else(|| {
                        let c = walk(b, a)?;
                        let hi = c.domain_hi();
                        Some((curve(c), hi, 0.0))
                    });
            }
        }
    }
    None
}

/// Where two curves on one surface cross, from parameters near `ta` and `tb`: both parameters and
/// the point. Newton on their distance.
fn springs_meet(a: &Curve, mut ta: f64, b: &Curve, mut tb: f64) -> Option<(f64, f64, Point3)> {
    for _ in 0..32 {
        let (da, db) = (a.deriv(ta), b.deriv(tb));
        let w = da.p - db.p;
        if w.length() <= 1e-3 * tol::LINEAR {
            return Some((ta, tb, da.p));
        }
        // least squares of da.d1 x - db.d1 y = -w
        let (aa, ab, bb) = (da.d1.dot(da.d1), da.d1.dot(db.d1), db.d1.dot(db.d1));
        let det = aa * bb - ab * ab;
        if det <= 1e-12 * aa * bb {
            return None;
        }
        let (ra, rb) = (-w.dot(da.d1), w.dot(db.d1));
        ta += (ra * bb + rb * ab) / det;
        tb += (ra * ab + rb * aa) / det;
    }
    None
}

/// The curve where two blends cut each other, from `x` to `p` (their springs' meeting point, where
/// they are tangent). Two cylinders of one radius with crossing axes meet in a plane through both.
fn blends_meet(a: &Blend, b: &Blend, x: Point3, p: Point3) -> Option<(Curve, f64, f64)> {
    if let (Surface::Cylinder(ca), Surface::Cylinder(cb)) = (&a.surface, &b.surface)
        && (ca.radius - cb.radius).abs() <= tol::LINEAR
    {
        let (u, w) = (ca.frame.z().get(), cb.frame.z().get());
        let off = cb.frame.origin - ca.frame.origin;
        let cross = u.cross(w);
        if off.dot(cross).abs() <= tol::LINEAR * cross.length() {
            // the axes' crossing, and the bisecting plane through x and p
            let t = off.cross(w).dot(cross) / cross.length_sq();
            let c = ca.frame.origin + u * t;
            for n in [u + w, u - w] {
                let Some(n) = n.normalize() else { continue };
                if (x - c).dot(n.get()).abs() <= tol::LINEAR && (p - c).dot(n.get()).abs() <= tol::LINEAR {
                    let plane = Surface::Plane(forge_geom::Plane::new(Frame::from_z(c, n)));
                    if let Some(m) = meet(&plane, &a.surface, x, p) {
                        return Some(m);
                    }
                }
            }
        }
    }
    // otherwise walked into the touch
    let step = 0.05 * a.radius.min(b.radius);
    let c = forge_geom::intcurve::towards_touch(&a.surface, &b.surface, x, p, step)?;
    let hi = c.domain_hi();
    Some((Curve::Intersection(Arc::new(c)), 0.0, hi))
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
    let mut s = b.closest(p, range);
    if b.eval(s).distance(p) <= tol::LINEAR {
        return Some(t);
    }
    // curves crossing at a shallow angle: projecting back and forth creeps there at the angle's cos^2
    // a step, Newton on the two parameters does not
    for _ in 0..30 {
        let (da, db) = (a.deriv(t), b.deriv(s));
        let r = da.p - db.p;
        if r.length() <= 1e-3 * tol::LINEAR {
            return Some(t);
        }
        let (aa, ab, bb) = (da.d1.dot(da.d1), da.d1.dot(db.d1), db.d1.dot(db.d1));
        let det = aa * bb - ab * ab;
        if det <= 1e-12 * aa * bb {
            return None;
        }
        let (ra, rb) = (r.dot(da.d1), r.dot(db.d1));
        t = (t + (rb * ab - ra * bb) / det).clamp(window.lo, window.hi);
        s = (s + (rb * aa - ra * ab) / det).clamp(range.lo, range.hi);
    }
    (a.eval(t).distance(b.eval(s)) <= tol::LINEAR).then_some(t)
}

/// The centre of a ball of radius `r` on the two faces, in the plane across the edge at `l.p`: the
/// ball grown from nothing at the edge, so it stays on this branch of the offsets' meet.
fn rolled_centre(body: &Body, l: &Local, r: f64) -> Option<Point3> {
    let across = Surface::Plane(forge_geom::Plane {
        frame: Frame::from_z(l.p, l.tangent),
    });
    let mut c = l.p + l.toward * (r / 16.0);
    for k in (0..5).rev() {
        let rho = r / f64::from(1 << k);
        let offs = [
            (body.face(l.faces[0]).surface.clone(), l.sign[0] * rho),
            (body.face(l.faces[1]).surface.clone(), l.sign[1] * rho),
            (across.clone(), 0.0),
        ];
        let at = three_offsets(&offs, c)?;
        c = l.p + (at - l.p) * 2.0;
        if k == 0 {
            return Some(at);
        }
    }
    None
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
    fn merges_naming_each_other_end() {
        let v = VertexId;
        let m = merge_roots(&[
            (v(1), v(2)),
            (v(2), v(3)),
            (v(3), v(1)),
            (v(2), v(1)),
            (v(4), v(4)),
            (v(5), v(1)),
        ]);
        let root = m[&v(1)];
        assert_eq!(m[&v(2)], root);
        assert_eq!(m[&v(5)], root);
        assert!(!m.contains_key(&root));
        assert!(!m.contains_key(&v(4)));
    }

    #[test]
    fn curves_crossing_at_a_shallow_angle_meet() {
        use forge_geom::Line;
        let a = Curve::Line(Line::new(Point3::ORIGIN, UnitVec3::X));
        let d = Vec3::new(1.0, 0.002, 0.0).normalize().expect("unit");
        let b = Curve::Line(Line::new(Point3::new(10.0, 0.0, 0.0), d));
        let t = crossing(&a, 3.0, &b, Interval::new(-100.0, 100.0)).expect("they cross");
        assert!((t - 10.0).abs() < 1e-6, "crossed at {t}");
    }

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

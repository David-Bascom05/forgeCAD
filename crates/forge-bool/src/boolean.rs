//! Union, subtraction and intersection of two solids.
//!
//! 1. Candidate face pairs by bounding box.
//! 2. Each pair's surfaces are intersected in closed form; each curve is cut
//!    where it crosses either face's boundary and the pieces lying inside
//!    both faces become new edges.
//! 3. Every boundary edge a new edge ends on is split there, once, for both
//!    faces that share it.
//! 4. Each face crossed by new edges is split by tracing the regions they
//!    and its boundary enclose, in its (oriented) parameter plane.
//! 5. A piece next to a new edge is inside or outside the other solid by
//!    which side of the other face it leans to; that answer floods across
//!    unsplit edges. Only regions no new edge touches need a ray test.
//! 6. The pieces the operation keeps are sewn into a new body.

use crate::classify::{Index, Where};
use crate::ssi;
use forge_geom::intersect::{Contact, CurveHit, CurveSurface, SurfaceSurface, curve_surface};
use forge_geom::{Curve, CurveGeom, Cylinder, Plane, Surface, SurfaceGeom};
use forge_math::predicates::angle_cmp;
use forge_math::{Box3, Frame, Interval, Point2, Point3, Vec2, Vec3, dmath, tol};
use forge_topo::{Body, EdgeId, FaceId, Name, VertexId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Union,
    Subtract,
    Intersect,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BoolError {
    /// A configuration the kernel does not handle yet.
    Unsupported(&'static str),
    /// Something that should not happen with valid inputs.
    Internal(&'static str),
    /// The result would have material of zero thickness at `at`: the solids touch there
    /// exactly (a ball resting on a face, rods tangent along a line, blocks meeting at an
    /// edge) and both sides of the touch are kept, so the result is not a manifold solid.
    ZeroThickness { at: Point3 },
}

impl std::fmt::Display for BoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoolError::Unsupported(w) => write!(f, "not supported yet: {w}"),
            BoolError::Internal(w) => write!(f, "internal error: {w}"),
            BoolError::ZeroThickness { at } => write!(
                f,
                "the solids touch exactly at ({:.6}, {:.6}, {:.6}), leaving material of zero \
                 thickness there; move one apart or overlap them",
                at.x, at.y, at.z
            ),
        }
    }
}

pub(crate) type R<T> = Result<T, BoolError>;

/// A closed walk of edge pieces: (piece, forward).
pub(crate) type Walk = Vec<(usize, bool)>;
/// A face region: outer walk first, then holes.
pub(crate) type Region = Vec<Walk>;

/// Which input body: 0 is A, 1 is B.
pub(crate) type Side = usize;

/// Splits an edge into its pieces once (later calls give the same range).
type Ensure<'x> = dyn FnMut(Side, EdgeId, &mut Vec<Piece>) -> std::ops::Range<usize> + 'x;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum VKey {
    Old(Side, VertexId),
    New(u32),
}

/// What a cut lies on: a boundary edge of one side, or no edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CutOn {
    Edge(Side, EdgeId),
    /// On no edge: where an intersection curve ends inside both faces (its branches meet there,
    /// where the surfaces touch), a vertex the branches share; or half way round a curve's loop.
    ///
    /// Touch cuts and free cuts follow OpenCascade's general fuse: a curve that meets an edge,
    /// crossing or touching it, within tolerance leaves a vertex on both (its vertex/edge and
    /// edge/face interferences give "a vertex lying on an edge"), and a section curve is split at
    /// every such vertex before faces are split. Tolerances, not exact predicates: exact pipelines
    /// (Lévy 2024, CGAL's Nef polyhedra, Barki et al. 2015) need planar facets, and these curves are
    /// traced approximations of curved surfaces, so a touch is decided within tolerance as in
    /// Jackson's tolerant booleans (1995). Hoffmann (1989) is why the decision is made once, here,
    /// and every later step reads it rather than re-deciding from geometry.
    /// - OCCT Boolean Operations user guide: https://occt3d.com/dev/doc/overview/html/occt_user_guides__boolean_operations.html
    /// - B. Lévy, Exact predicates, exact constructions and combinatorics for mesh CSG, arXiv:2405.12949
    /// - D. Jackson, Boundary representation modelling with local tolerances, SMA 1995
    /// - C. M. Hoffmann, The problems of accuracy and robustness in geometric computation, Computer 22(3), 1989
    Free,
}

impl CutOn {
    pub(crate) fn edge(self) -> Option<(Side, EdgeId)> {
        match self {
            CutOn::Edge(side, e) => Some((side, e)),
            CutOn::Free => None,
        }
    }
}

/// A cut of a boundary edge by an intersection curve.
pub(crate) struct Cut {
    pub(crate) on: CutOn,
    pub(crate) s: f64,
    pub(crate) point: Point3,
    /// How far the point may be from the edge: LINEAR, or the distance to the end vertex it
    /// was found beside (within that vertex's tolerance).
    pub(crate) tol: f64,
    /// Found where the curve meets the edge without crossing it, or a free cut: ways out of its
    /// vertex may leave along one tangent.
    pub(crate) touch: bool,
}

/// Where point `p` lies on edge `e`: its parameter and the cut's tolerance, if it is within
/// LINEAR of the edge, or past an end but within that end vertex's tolerance.
pub(crate) fn on_edge(body: &Body, e: EdgeId, p: Point3) -> Option<(f64, f64)> {
    let edge = body.edge(e);
    let s = edge.curve.closest(p, edge.range);
    let d = edge.curve.eval(s).distance(p);
    if d <= tol::LINEAR {
        return Some((s, tol::LINEAR));
    }
    let end = if s <= edge.range.lo {
        edge.start
    } else if s >= edge.range.hi {
        edge.end
    } else {
        return None;
    };
    (d <= body.vertex(end).tol).then_some((s, d))
}

pub(crate) struct Seg {
    pub(crate) curve: Curve,
    pub(crate) range: Interval,
    /// Indices into the cuts list; None for a closed curve lying wholly inside both faces.
    pub(crate) ends: Option<(usize, usize)>,
    pub(crate) faces: [FaceId; 2],
    /// The bodies the two faces belong to.
    pub(crate) sides: [Side; 2],
    pub(crate) name: Name,
}

/// A piece of an edge in the result: part of an original edge, or a new edge.
#[derive(Clone)]
pub(crate) struct Piece {
    pub(crate) curve: Curve,
    pub(crate) range: Interval,
    pub(crate) start: VKey,
    pub(crate) end: VKey,
    pub(crate) name: Name,
    /// The new edge this piece is, if any.
    pub(crate) seg: Option<usize>,
    /// The original edge this piece is all of, if it was not split.
    pub(crate) whole: Option<(Side, EdgeId)>,
    /// The body whose edge this piece is part of (None for a new edge).
    pub(crate) side: Option<Side>,
    /// The edge this piece is part of, when it was split.
    pub(crate) of: Option<EdgeId>,
}

/// A face of the result before it is built: loops of (piece, forward).
struct Spec {
    side: Side,
    from: FaceId,
    loops: Vec<Vec<(usize, bool)>>,
    /// NONE for a piece of a split face: named only if kept (`sole_pieces_keep_names`).
    name: Name,
    /// Pieces of the other body's edges imprinted on this face (a coplanar
    /// face's boundary); classification never floods across them.
    foreign: Vec<usize>,
    /// The face comes through whole, every edge intact: its cached data carries over.
    whole: bool,
    /// The face keeps its loops (but `remove`) and gains these: kept, it is extended in
    /// place, not rebuilt (a new loop inside it, or a local split leaving its outer loop).
    extend: bool,
    /// Loops of `from` an extending spec takes off the face (they were split, or moved into
    /// a new region).
    remove: Vec<forge_topo::LoopId>,
    /// A class known before classification (a face's untouched remainder, decided by one point).
    preset: Option<Class>,
}

/// Part of an edge of one body lying inside a coplanar face of the other.
pub(crate) struct Imprint {
    pub(crate) side: Side,
    pub(crate) edge: EdgeId,
    pub(crate) range: Interval,
    pub(crate) into: FaceId,
    /// The body `into` belongs to.
    pub(crate) into_side: Side,
}

/// Where a face piece lies relative to the other body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    In,
    Out,
    /// On a face of the other body, both facing the same way.
    OnSame,
    /// On a face of the other body, facing opposite ways.
    OnOpposite,
}

pub fn boolean(a: &Body, b: &Body, op: Op, feature: u64) -> R<Body> {
    let mut out = a.clone();
    boolean_into(&mut out, b, op, feature)?;
    Ok(out.compact())
}

/// The boolean done in place on `target`: only faces near `b` are examined,
/// and only the faces it actually changes are rebuilt. Removed entities stay
/// as tombstones until `Body::compact`. Names do not depend on `_feature`: what the boolean makes
/// is named by what meets there, as the n-ary boolean names it.
pub fn boolean_into(target: &mut Body, b: &Body, op: Op, _feature: u64) -> R<()> {
    let plan = plan(target, b, op)?;
    let mut clock = phases_on().then(std::time::Instant::now);
    apply(target, b, &plan);
    lap(&mut clock, 6);
    Ok(())
}

/// Everything a boolean decides, worked out without touching A.
struct Plan {
    pieces: Vec<Piece>,
    specs: Vec<Spec>,
    kept: Vec<(usize, bool)>,
    points: Vec<Point3>,
    /// Each new point's name.
    vnames: Vec<Name>,
    /// Each new point's tolerance.
    tols: Vec<f64>,
    /// Existing vertices whose tolerance grows to hold the curves now ending on them.
    grown: Vec<(VKey, f64)>,
    alias: Vec<(VKey, VKey)>,
    /// A's faces that might change; every other face of A comes through as it is.
    involved: Vec<FaceId>,
    /// Involved faces of A nothing touched, found wholly on the kept side of B.
    untouched_kept: Vec<FaceId>,
}

/// The pieces an edge splits into at its cuts (one piece, the whole edge, if none), pushed
/// consecutively: their indices are the returned range.
#[allow(clippy::too_many_arguments)]
pub(crate) fn edge_split(
    body: &Body,
    side: Side,
    e: EdgeId,
    cuts: &[Cut],
    by_edge: &[(Side, EdgeId, usize)],
    cut_vertex: &[VKey],
    alias: &[(VKey, VKey)],
    pieces: &mut Vec<Piece>,
) -> std::ops::Range<usize> {
    let resolve = |v: VKey| alias.iter().find(|(f, _)| *f == v).map_or(v, |&(_, t)| t);
    let edge = body.edge(e);
    let from = by_edge.partition_point(|&(sd, ed, _)| (sd, ed) < (side, e));
    let mut splits: Vec<(f64, VKey)> = by_edge[from..]
        .iter()
        .take_while(|&&(sd, ed, _)| sd == side && ed == e)
        .map(|&(_, _, i)| (cuts[i].s, cut_vertex[i]))
        .collect();
    // on a periodic curve a cut may come in another period than the edge's range
    if let Some(period) = edge.curve.domain().period {
        for sp in &mut splits {
            while sp.0 < edge.range.lo {
                sp.0 += period;
            }
            while sp.0 >= edge.range.lo + period {
                sp.0 -= period;
            }
        }
    }
    splits.sort_by(|x, y| x.0.total_cmp(&y.0));
    splits.dedup_by(|x, y| x.1 == y.1);
    let (vs, ve) = (
        resolve(VKey::Old(side, edge.start)),
        resolve(VKey::Old(side, edge.end)),
    );
    splits.retain(|&(_, v)| v != vs && v != ve);
    let whole = splits.is_empty().then_some((side, e));
    let first = pieces.len();
    let mut at = (edge.range.lo, vs);
    for &(s, v) in splits.iter().chain([(edge.range.hi, ve)].iter()) {
        pieces.push(Piece {
            curve: edge.curve.clone(),
            range: Interval::new(at.0, s),
            start: at.1,
            end: v,
            // a piece is named once kept (`piece_name`)
            name: if whole.is_some() { edge.name } else { Name::NONE },
            seg: None,
            whole,
            side: Some(side),
            of: whole.is_none().then_some(e),
        });
        at = (s, v);
    }
    first..pieces.len()
}

const FAST_EXTEND: bool = true;

/// Time spent per phase of the booleans so far (ns), when FORGE_PHASES is set:
/// intersect, imprint, place cuts and make pieces, split, classify, keep, apply.
pub fn phase_times() -> [u64; 7] {
    PHASES.with(|p| *p.borrow())
}

thread_local! {
    static PHASES: std::cell::RefCell<[u64; 7]> = const { std::cell::RefCell::new([0; 7]) };
}

fn phases_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("FORGE_PHASES").is_some())
}

/// FORGE_DEBUG is set (read once: hot paths ask).
pub(crate) fn debug_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("FORGE_DEBUG").is_some())
}

/// FORGE_SPECS is set.
fn specs_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("FORGE_SPECS").is_some())
}

/// Adds the time since `*t` to phase `k` and restarts the clock.
fn lap(t: &mut Option<std::time::Instant>, k: usize) {
    if let Some(t0) = t {
        let now = std::time::Instant::now();
        PHASES.with(|p| p.borrow_mut()[k] += (now - *t0).as_nanos() as u64);
        *t = Some(now);
    }
}

fn plan(a: &Body, b: &Body, op: Op) -> R<Plan> {
    let mut clock = phases_on().then(std::time::Instant::now);
    let bodies = [a, b];
    let index = [Index::new(a), Index::new(b)];
    // new edges and vertices are named by the faces and edges meeting there, never by the
    // feature: the n-ary boolean, meeting them all at once, names them alike
    let root = Name::feature(0);

    // 1-2: intersect overlapping face pairs, keep pieces inside both faces
    let mut cuts: Vec<Cut> = Vec::new();
    let mut segs: Vec<Seg> = Vec::new();
    let mut coplanar: Vec<(FaceId, FaceId)> = Vec::new();
    let mut imprints: Vec<Imprint> = Vec::new();
    let mut touching: Vec<Touch> = Vec::new();
    let b_box = index[1].bbox;
    let near_b: Vec<FaceId> = a.faces_where(|x| x.overlaps(&b_box)).collect();
    let b_boxes: Vec<Box3> = (0..b.faces.len() as u32)
        .map(|f| index[1].face_box(FaceId(f)))
        .collect();
    for &fa in &near_b {
        let ba = index[0].face_box(fa);
        for fb in b.face_ids() {
            if !ba.overlaps(&b_boxes[fb.index()]) {
                continue;
            }
            let (sa, sb) = (&a.face(fa).surface, &b.face(fb).surface);
            let region = overlap(&ba, &b_boxes[fb.index()]).grow(tol::LINEAR);
            let found = ssi::surfaces_in(sa, sb, region);
            if debug_on() {
                let what = match &found {
                    SurfaceSurface::Curves(cs) => format!("{} curves", cs.len()),
                    other => format!("{other:?}").chars().take(40).collect(),
                };
                eprintln!("  pair {fa:?} {fb:?}: {what}");
            }
            touching.extend(touches(&found, [&index[0], &index[1]], [0, 1], [fa, fb], region));
            let curves = match found {
                SurfaceSurface::Empty | SurfaceSurface::Points(_) => continue,
                SurfaceSurface::Coincident => {
                    coplanar.push((fa, fb));
                    continue;
                }
                SurfaceSurface::NotAnalytic => {
                    return Err(BoolError::Unsupported("surface pair without a closed form"));
                }
                SurfaceSurface::Curves(cs) => cs,
            };
            // branches of one intersection may meet where the surfaces touch
            let branches = [
                curves.iter().filter(|c| c.1 == Contact::Crossing).count() > 1,
                branches_meet(&curves),
            ];
            for (ci, (curve, contact)) in curves.into_iter().enumerate() {
                // surfaces that only touch along a curve cross nowhere: no face splits there
                if contact == Contact::Touching {
                    continue;
                }
                let pair_name = root
                    .with_str("cut")
                    .with_set(vec![a.face(fa).name, b.face(fb).name])
                    .with(ci as u64);
                trim(
                    bodies,
                    [&index[0], &index[1]],
                    [0, 1],
                    [fa, fb],
                    curve,
                    pair_name,
                    branches,
                    &mut cuts,
                    &mut segs,
                    &mut imprints,
                )?;
            }
        }
    }

    // where the solids touch, both sides kept would be material meeting itself
    for t in &touching {
        let near = |side: Side| {
            let (body, f) = (bodies[side], t.faces[side]);
            local_class(body, &index[side], f, t.at, t.dir, |q| {
                match index[1 - side].classify(q) {
                    Where::On => None,
                    w => Some(w),
                }
            })
        };
        if debug_on() {
            eprintln!(
                "touch {:?} at {:?} dir {:?}: {:?} {:?}",
                t.faces,
                t.at,
                t.dir,
                near(0),
                near(1)
            );
        }
        let (Some(ca), Some(cb)) = (near(0), near(1)) else {
            continue;
        };
        let kept_a = matches!(
            (op, ca),
            (Op::Union | Op::Subtract, Where::Outside) | (Op::Intersect, Where::Inside)
        );
        let kept_b = matches!(
            (op, cb),
            (Op::Union, Where::Outside) | (Op::Subtract | Op::Intersect, Where::Inside)
        );
        if kept_a && kept_b {
            return Err(BoolError::ZeroThickness { at: t.at });
        }
    }

    lap(&mut clock, 0);
    // faces on one surface: each imprints the other's boundary
    for &(fa, fb) in &coplanar {
        let pair = [&index[0], &index[1]];
        imprint(bodies, pair, [0, 1], 1, fb, fa, &mut cuts, &mut imprints)?;
        imprint(bodies, pair, [0, 1], 0, fa, fb, &mut cuts, &mut imprints)?;
    }

    // 3: vertices for the cuts, then edge pieces
    lap(&mut clock, 1);
    let (cut_vertex, mut new_points, alias) = place_cuts(&bodies, &cuts)?;
    let mut marks = curve_ends(&segs, &cut_vertex);
    let mut ended = vec![false; new_points.len()];
    for m in &marks {
        ended[m.0 as usize] = true;
    }
    for (c, &k) in cuts.iter().zip(&cut_vertex) {
        if let VKey::New(i) = k
            && !ended[i as usize]
            && let Some((sd, e)) = c.on.edge()
        {
            marks.push((i, bodies[sd].edge(e).name));
        }
    }
    let mut vnames = vertex_names(&new_points, marks, root);
    // closed new edges' own vertices (added after the cuts' below)
    vnames.extend(segs.iter().map(|s| s.name.with_str("seam")));
    // cut points of one vertex differ by up to their tolerance: a new vertex settles on the point
    // nearest every curve through it (alternating projections), and every curve parameter is
    // then taken from that point, so edge and curve ends land on their vertices
    let mut through: Vec<Vec<(&Curve, Interval)>> = vec![Vec::new(); new_points.len()];
    for (c, &k) in cuts.iter().zip(&cut_vertex) {
        if let VKey::New(i) = k
            && let Some((sd, e)) = c.on.edge()
        {
            let edge = bodies[sd].edge(e);
            through[i as usize].push((&edge.curve, edge.range));
        }
    }
    for sg in &segs {
        let Some((i0, i1)) = sg.ends else { continue };
        let w = 1e-3 * sg.range.width();
        for (t, k) in [(sg.range.lo, cut_vertex[i0]), (sg.range.hi, cut_vertex[i1])] {
            if let VKey::New(i) = k {
                through[i as usize].push((&sg.curve, Interval::new(t - w, t + w)));
            }
        }
    }
    let mut new_tols = Vec::with_capacity(new_points.len());
    for (p, curves) in new_points.iter_mut().zip(&through) {
        let (q, tol) = settle(*p, curves)?;
        *p = q;
        new_tols.push(tol);
    }
    let key_point = |k: VKey| match k {
        VKey::Old(sd, v) => bodies[sd].vertex(v).point,
        VKey::New(i) => new_points[i as usize],
    };
    for (c, &k) in cuts.iter_mut().zip(&cut_vertex) {
        c.point = key_point(k);
        if let Some((sd, e)) = c.on.edge() {
            let edge = bodies[sd].edge(e);
            c.s = edge.curve.closest(c.point, edge.range);
        }
    }
    for sg in &mut segs {
        let Some((i0, i1)) = sg.ends else { continue };
        let w = 1e-3 * sg.range.width();
        let near = |t: f64, k: VKey| sg.curve.closest(key_point(k), Interval::new(t - w, t + w));
        let (lo, hi) = (
            near(sg.range.lo, cut_vertex[i0]),
            near(sg.range.hi, cut_vertex[i1]),
        );
        sg.range = Interval::new(lo, hi);
    }
    let grown = grown_tols(&bodies, &cuts, &cut_vertex, &segs, &alias, key_point);
    if debug_on() {
        for (i, sg) in segs.iter().enumerate() {
            let ends = sg.ends.map(|(x, y)| (cut_vertex[x], cut_vertex[y]));
            eprintln!(
                "seg {i}: faces {:?} ends {:?} range {:?} mid {:?}",
                sg.faces,
                ends,
                sg.range,
                sg.curve.eval(sg.range.mid())
            );
        }
        for (i, c) in cuts.iter().enumerate() {
            eprintln!(
                "cut {i}: on {:?} s {:.4} -> {:?} at {:?}",
                c.on, c.s, cut_vertex[i], c.point
            );
        }
    }
    let mut pieces: Vec<Piece> = Vec::new();
    // ways out of a vertex share a tangent only where a curve met an edge without crossing it
    let mut ties: Vec<VKey> = cuts
        .iter()
        .zip(&cut_vertex)
        .filter(|(c, _)| c.touch)
        .map(|(_, &k)| k)
        .collect();
    if !ties.is_empty() {
        ties.sort_unstable();
        let aliased: Vec<VKey> = alias
            .iter()
            .filter(|(_, t)| ties.binary_search(t).is_ok())
            .map(|&(f, _)| f)
            .collect();
        ties.extend(aliased);
        ties.sort_unstable();
    }
    let mut by_edge: Vec<(Side, EdgeId, usize)> = cuts
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.on.edge().map(|(sd, e)| (sd, e, i)))
        .collect();
    by_edge.sort_unstable();
    // pieces per edge, made only for edges a spec reaches
    // looked up, never iterated, so the map's order cannot reach the result
    // each edge's pieces, split once: (first piece, count) by edge index, count 0 if not yet
    let mut edge_pieces: [Vec<(u32, u32)>; 2] = [vec![(0, 0); a.edges.len()], vec![(0, 0); b.edges.len()]];
    // the edges split into more than one piece, and their pieces
    let mut split_runs: Vec<(Side, EdgeId, std::ops::Range<usize>)> = Vec::new();
    let mut ensure = |side: Side, e: EdgeId, pieces: &mut Vec<Piece>| -> std::ops::Range<usize> {
        let (first, n) = edge_pieces[side][e.index()];
        if n > 0 {
            return first as usize..(first + n) as usize;
        }
        let r = edge_split(
            bodies[side],
            side,
            e,
            &cuts,
            &by_edge,
            &cut_vertex,
            &alias,
            pieces,
        );
        edge_pieces[side][e.index()] = (r.start as u32, r.len() as u32);
        if r.len() > 1 {
            split_runs.push((side, e, r.clone()));
        }
        r
    };
    let mut seg_piece = Vec::with_capacity(segs.len());
    for (si, s) in segs.iter().enumerate() {
        let (start, end) = match s.ends {
            Some((i, j)) => (cut_vertex[i], cut_vertex[j]),
            None => {
                let v = VKey::New((new_points.len() + si) as u32);
                (v, v)
            }
        };
        seg_piece.push(pieces.len());
        pieces.push(Piece {
            curve: s.curve.clone(),
            range: s.range,
            start,
            end,
            name: s.name,
            seg: Some(si),
            whole: None,
            side: None,
            of: None,
        });
    }
    // closed new edges carry their own vertex at the curve start
    let mut points = new_points;
    let mut tols = new_tols;
    for s in &segs {
        points.push(s.curve.eval(s.range.lo));
        tols.push(tol::LINEAR);
    }

    // A's faces that can change: crossed by new edges, using a split edge, or within B's reach
    let mut involved: Vec<FaceId> = segs.iter().map(|s| s.faces[0]).collect();
    involved.extend(imprints.iter().filter(|im| im.side == 1).map(|im| im.into));
    for &(sd, e, _) in &by_edge {
        if sd == 0 {
            involved.extend(a.edge(e).coedges.iter().map(|&c| a.coedge_face(c)));
        }
    }
    let mut split_faces: Vec<FaceId> = Vec::new();
    for &(sd, e, _) in &by_edge {
        if sd == 0 {
            split_faces.extend(a.edge(e).coedges.iter().map(|&c| a.coedge_face(c)));
        }
    }
    split_faces.sort_unstable();
    split_faces.dedup();
    involved.extend(near_b);
    involved.sort_unstable();
    involved.dedup();

    lap(&mut clock, 2);
    // 4: split faces
    let mut specs: Vec<Spec> = Vec::new();
    let mut untouched_kept: Vec<FaceId> = Vec::new();
    for (side, &body) in bodies.iter().enumerate() {
        let faces: Vec<FaceId> = if side == 0 {
            involved.clone()
        } else {
            body.face_ids().collect()
        };
        for f in faces {
            let mine: Vec<usize> = (0..segs.len()).filter(|&i| segs[i].faces[side] == f).collect();
            // a face of A that no new edge, imprint or split edge touches lies wholly inside or
            // outside B: one point decides it, and it needs no pieces (a plate face with hundreds
            // of holes lies under every tool's box)
            if side == 0
                && mine.is_empty()
                && split_faces.binary_search(&f).is_err()
                && !imprints.iter().any(|im| im.side == 1 && im.into == f)
                && !coplanar.iter().any(|&(fa, _)| fa == f)
                && let Some(p) = face_point(body, f)
            {
                let kept = match (op, index[1].classify(p)) {
                    (Op::Intersect, Where::Inside) | (Op::Union | Op::Subtract, Where::Outside) => Some(true),
                    (_, Where::Inside | Where::Outside) => Some(false),
                    (_, Where::On) => None,
                };
                match kept {
                    Some(true) => {
                        untouched_kept.push(f);
                        continue;
                    }
                    Some(false) => continue,
                    None => {}
                }
            }
            // new edges inside a planar face of A closing into one loop that touches neither its
            // boundary nor encloses any of its loops: the face just gains a hole (and a disk fills
            // it), no need to piece its whole boundary (a plate face with hundreds of holes)
            if FAST_EXTEND
                && side == 0
                && !mine.is_empty()
                && matches!(body.face(f).surface, Surface::Plane(_))
                && split_faces.binary_search(&f).is_err()
                && !imprints.iter().any(|im| im.side == 1 && im.into == f)
                && let Some((cycle, poly)) = lone_cycle(
                    body,
                    f,
                    &mine.iter().map(|&i| seg_piece[i]).collect::<Vec<_>>(),
                    &pieces,
                )
                && !encloses_a_loop(body, f, &poly)
            {
                let back: Vec<(usize, bool)> = cycle.iter().rev().map(|&(p, d)| (p, !d)).collect();
                let left = (forge_topo::uv::signed_area(&poly) > 0.0) == body.face(f).same_sense;
                let (disk, hole) = if left { (cycle, back) } else { (back, cycle) };
                let name = body.face(f).name;
                specs.push(Spec {
                    side,
                    from: f,
                    loops: vec![hole],
                    name,
                    foreign: Vec::new(),
                    whole: false,
                    extend: true,
                    remove: Vec::new(),
                    preset: None,
                });
                // the face keeps its name in place
                let disk = vec![disk];
                specs.push(Spec {
                    side,
                    from: f,
                    name: Name::NONE,
                    loops: disk,
                    foreign: Vec::new(),
                    whole: false,
                    extend: false,
                    remove: Vec::new(),
                    preset: None,
                });
                continue;
            }
            let mut foreign: Vec<usize> = Vec::new();
            for im in imprints.iter().filter(|im| im.side != side && im.into == f) {
                for p in ensure(im.side, im.edge, &mut pieces) {
                    let m = pieces[p].range.mid();
                    if im.range.contains(m) && !foreign.contains(&p) {
                        foreign.push(p);
                    }
                }
            }
            // a big plane face whose outer loop the change leaves alone: only the loops it touches
            // are split with the new edges; the face keeps its id, its other loops and its index,
            // and the rest of it is decided by one point near that outer loop
            if side == 0
                && (!mine.is_empty() || !foreign.is_empty())
                && body.face(f).loops.len() >= 8
                && matches!(body.face(f).surface, Surface::Plane(_))
                && !coplanar.iter().any(|&(fa, _)| fa == f)
                && let Some(local) = local_split(LocalSplit {
                    body,
                    f,
                    op,
                    index: &index[1],
                    by_edge: &by_edge,
                    alias: &alias,
                    new_edges: &mine
                        .iter()
                        .map(|&i| seg_piece[i])
                        .chain(foreign.iter().copied())
                        .collect::<Vec<_>>(),
                    pieces: &pieces,
                })
            {
                let walk = |l: forge_topo::LoopId, pieces: &mut Vec<Piece>, ensure: &mut Ensure| {
                    let mut w: Walk = Vec::new();
                    for &c in &body.lp(l).coedges {
                        let co = body.coedge(c);
                        let ps = ensure(0, co.edge, pieces);
                        if co.forward {
                            w.extend(ps.map(|p| (p, true)));
                        } else {
                            w.extend(ps.rev().map(|p| (p, false)));
                        }
                    }
                    w
                };
                let boundary: Vec<Walk> = local
                    .touched
                    .iter()
                    .map(|&l| walk(l, &mut pieces, &mut ensure))
                    .collect();
                let mut new_edges: Vec<usize> = mine.iter().map(|&i| seg_piece[i]).collect();
                new_edges.extend(foreign.iter().copied());
                let split = split_face(body, f, &pieces, &boundary, &new_edges, false, &ties)?;
                // an untouched hole inside a new region moves into it
                let face = body.face(f);
                let mut moved: Vec<(usize, forge_topo::LoopId)> = Vec::new();
                let drawn = std::cell::OnceCell::<Vec<Option<Vec<Point2>>>>::new();
                let region_boxes = std::cell::OnceCell::<Vec<(Box3, Vec<Box3>)>>::new();
                let outlines = std::cell::OnceCell::<Vec<Outline>>::new();
                for &l in face.loops.iter().skip(1) {
                    if local.touched.contains(&l) {
                        continue;
                    }
                    let cands = body.lp(l).coedges.iter().map(|&c| {
                        let q = face.surface.project(body.vertex(body.coedge_start(c)).point);
                        if face.same_sense {
                            q
                        } else {
                            Point2::new(-q.x, q.y)
                        }
                    });
                    // exactly, on a plane split exactly: by one of the hole's vertices, as it
                    // touches no new edge
                    if split.outlines.is_none() {
                        let q = body.vertex(body.coedge_start(body.lp(l).coedges[0])).point;
                        let boxes = region_boxes.get_or_init(|| {
                            split.regions.iter().map(|r| walk_boxes(&r[0], &pieces)).collect()
                        });
                        let held: Option<Vec<bool>> = split
                            .regions
                            .iter()
                            .zip(boxes)
                            .map(|(r, (hull, each))| match hull.contains(q) {
                                true => region_holds(body, f, r, &pieces, q, each),
                                false => Some(false),
                            })
                            .collect();
                        if let Some(held) = held {
                            if let Some(r) = held.iter().position(|&h| h) {
                                moved.push((r, l));
                            }
                            continue;
                        }
                    }
                    let drawn = drawn.get_or_init(|| match &split.outlines {
                        Some(o) => o.clone(),
                        None => plane_outlines(body, f, &split.regions, &pieces),
                    });
                    let outlines = outlines.get_or_init(|| {
                        drawn
                            .iter()
                            .flatten()
                            .map(|o| {
                                let b = o.iter().fold(
                                    (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY),
                                    |a, q| (a.0.min(q.x), a.1.max(q.x), a.2.min(q.y), a.3.max(q.y)),
                                );
                                (o.as_slice(), b)
                            })
                            .collect()
                    });
                    let Some(q) = clear_point(cands, outlines) else {
                        continue;
                    };
                    if let Some(r) = drawn
                        .iter()
                        .position(|o| o.as_ref().is_some_and(|o| inside_poly(q, o)))
                    {
                        moved.push((r, l));
                    }
                }
                let used = |loops: &[Walk]| -> Vec<usize> {
                    foreign
                        .iter()
                        .copied()
                        .filter(|p| loops.iter().flatten().any(|h| h.0 == *p))
                        .collect()
                };
                let mut remove = local.touched.clone();
                remove.extend(moved.iter().map(|m| m.1));
                specs.push(Spec {
                    side,
                    from: f,
                    foreign: used(&split.open),
                    loops: split.open,
                    name: face.name,
                    whole: false,
                    extend: true,
                    remove,
                    preset: Some(local.class),
                });
                for (k, mut loops) in split.regions.into_iter().enumerate() {
                    for &(r, l) in &moved {
                        if r == k {
                            loops.push(walk(l, &mut pieces, &mut ensure));
                        }
                    }
                    specs.push(Spec {
                        side,
                        from: f,
                        foreign: used(&loops),
                        name: Name::NONE,
                        loops,
                        whole: false,
                        extend: false,
                        remove: Vec::new(),
                        preset: None,
                    });
                }
                continue;
            }
            let mut whole = true;
            let mut boundary: Vec<Vec<(usize, bool)>> = Vec::with_capacity(body.face(f).loops.len());
            for &l in &body.face(f).loops {
                let mut walk = Vec::with_capacity(body.lp(l).coedges.len());
                for &c in &body.lp(l).coedges {
                    let co = body.coedge(c);
                    let ps = ensure(side, co.edge, &mut pieces);
                    whole &= ps.len() == 1;
                    if co.forward {
                        walk.extend(ps.clone().map(|p| (p, true)));
                    } else {
                        walk.extend(ps.clone().rev().map(|p| (p, false)));
                    }
                }
                boundary.push(walk);
            }
            if mine.is_empty() && foreign.is_empty() {
                specs.push(Spec {
                    side,
                    from: f,
                    loops: boundary,
                    name: body.face(f).name,
                    foreign,
                    whole,
                    extend: false,
                    remove: Vec::new(),
                    preset: None,
                });
                continue;
            }
            let mut new_edges: Vec<usize> = mine.iter().map(|&i| seg_piece[i]).collect();
            new_edges.extend(foreign.iter().copied());
            let regions = split_face(body, f, &pieces, &boundary, &new_edges, true, &ties)?.regions;
            for loops in regions {
                let used: Vec<usize> = foreign
                    .iter()
                    .copied()
                    .filter(|p| loops.iter().flatten().any(|h| h.0 == *p))
                    .collect();
                specs.push(Spec {
                    side,
                    from: f,
                    name: Name::NONE,
                    loops,
                    foreign: used,
                    whole: false,
                    extend: false,
                    remove: Vec::new(),
                    preset: None,
                });
            }
        }
    }

    // pieces of the two bodies' edges with the same ends along the same curve (coincident walls
    // meeting a face at one line) are one edge: B's copy gives way to A's
    let mut by_ends: Vec<((VKey, VKey), usize)> = pieces
        .iter()
        .enumerate()
        .filter(|(_, pc)| pc.side == Some(0))
        .map(|(i, pc)| ((pc.start.min(pc.end), pc.start.max(pc.end)), i))
        .collect();
    by_ends.sort();
    let mut canon: Vec<(usize, bool)> = (0..pieces.len()).map(|p| (p, false)).collect();
    for (p, pc) in pieces.iter().enumerate().filter(|(_, pc)| pc.side == Some(1)) {
        let key = (pc.start.min(pc.end), pc.start.max(pc.end));
        let from = by_ends.partition_point(|x| x.0 < key);
        let mid = pc.curve.eval(pc.range.mid());
        for &(_, a) in by_ends[from..].iter().take_while(|x| x.0 == key) {
            let other = &pieces[a];
            if other
                .curve
                .eval(other.curve.closest(mid, other.range))
                .distance(mid)
                <= tol::LINEAR
            {
                canon[p] = (a, pc.start != other.start);
                break;
            }
        }
    }
    if canon.iter().enumerate().any(|(p, c)| c.0 != p) {
        for sp in &mut specs {
            for l in &mut sp.loops {
                for h in l.iter_mut() {
                    let (c, flip) = canon[h.0];
                    *h = (c, h.1 != flip);
                }
            }
            for f in &mut sp.foreign {
                *f = canon[*f].0;
            }
        }
    }

    lap(&mut clock, 3);
    // 5: classify
    let class = classify_specs(&bodies, &index, &specs, &pieces, &segs, &coplanar)?;
    if specs_on() {
        for (i, sp) in specs.iter().enumerate() {
            let ls: Vec<Vec<usize>> = sp.loops.iter().map(|l| l.iter().map(|h| h.0).collect()).collect();
            eprintln!(
                "  SPEC {i}: side {} face {:?} class {:?} loops {ls:?}",
                sp.side, sp.from, class[i]
            );
        }
    }

    lap(&mut clock, 4);
    // 6: keep, flip. A shared same-facing region keeps A's copy only.
    let keep = |side: Side, c: Class| match (op, side, c) {
        (Op::Union, _, Class::Out) | (Op::Union, 0, Class::OnSame) => Some(false),
        (Op::Subtract, 0, Class::Out) | (Op::Subtract, 0, Class::OnOpposite) => Some(false),
        (Op::Subtract, 1, Class::In) => Some(true),
        (Op::Intersect, _, Class::In) | (Op::Intersect, 0, Class::OnSame) => Some(false),
        _ => None,
    };
    let kept: Vec<(usize, bool)> = (0..specs.len())
        .filter_map(|i| keep(specs[i].side, class[i]).map(|flip| (i, flip)))
        .collect();
    // an edge four kept faces meet at is where the two solids' material touches along a line
    let mut uses = vec![0u8; pieces.len()];
    for &(i, _) in &kept {
        for &(p, _) in specs[i].loops.iter().flatten() {
            uses[p] = uses[p].saturating_add(1);
            if uses[p] > 2 {
                let pc = &pieces[p];
                return Err(BoolError::ZeroThickness {
                    at: pc.curve.eval(pc.range.mid()),
                });
            }
        }
    }
    // faces of A beyond B's reach are outside B: kept by union and subtract, dropped by intersect
    if op == Op::Intersect {
        involved = a.face_ids().collect();
    }
    lap(&mut clock, 5);
    let vname = |v: VKey| match v {
        VKey::Old(sd, id) => bodies[sd].vertex(id).name,
        VKey::New(i) => vnames[i as usize],
    };
    sole_pieces_keep_names(
        &bodies,
        &mut specs,
        &kept,
        &mut pieces,
        &uses,
        &split_runs,
        &vname,
    );
    Ok(Plan {
        pieces,
        specs,
        kept,
        points,
        vnames,
        tols,
        grown,
        alias,
        involved,
        untouched_kept,
    })
}

/// Makes the planned changes to A: drops the faces that go, rebuilds the
/// faces that change, adds the faces that come from B, and regroups shells.
fn apply(a: &mut Body, b: &Body, plan: &Plan) {
    let Plan {
        pieces,
        specs,
        kept,
        points,
        vnames,
        tols,
        grown,
        alias,
        involved,
        untouched_kept,
    } = plan;
    let mut as_is = vec![false; specs.len()];
    for &(si, flip) in kept {
        if specs[si].side == 0 && specs[si].whole && !flip {
            as_is[si] = true;
        }
    }
    let mut staying: Vec<FaceId> = specs
        .iter()
        .zip(&as_is)
        .filter(|(_, k)| **k)
        .map(|(s, _)| s.from)
        .collect();
    staying.extend(
        kept.iter()
            .filter(|&&(si, flip)| specs[si].extend && !flip)
            .map(|&(si, _)| specs[si].from),
    );
    staying.extend(untouched_kept.iter().copied());
    staying.sort_unstable();
    staying.dedup();
    for &f in involved {
        if staying.binary_search(&f).is_err() {
            a.kill_face(f);
        }
    }
    // a face split locally gives up the loops it replaces before it gains the new ones
    for &(si, flip) in kept {
        let s = &specs[si];
        if s.extend && !flip && !s.remove.is_empty() {
            crate::classify::drop_query_loops(a, s.from, &s.remove);
            for &l in &s.remove {
                a.remove_loop(s.from, l);
            }
        }
    }
    let mut b_vertex: Vec<Option<VertexId>> = vec![None; b.vertices.len()];
    let mut new_vertex: Vec<Option<VertexId>> = vec![None; points.len()];
    let mut emap: Vec<Option<EdgeId>> = vec![None; pieces.len()];
    let shell = a
        .shells
        .first()
        .map(|_| forge_topo::ShellId(0))
        .unwrap_or_else(|| a.add_shell());
    let mut vertex = |a: &mut Body, v: VKey| -> VertexId {
        let v = alias.iter().find(|(f, _)| *f == v).map_or(v, |&(_, t)| t);
        match v {
            VKey::Old(0, id) => id,
            VKey::Old(_, id) => *b_vertex[id.index()].get_or_insert_with(|| {
                let x = b.vertex(id);
                a.add_tolerant_vertex(x.point, grown_tol(grown, v, x.tol), x.name)
            }),
            VKey::New(i) => *new_vertex[i as usize].get_or_insert_with(|| {
                let i = i as usize;
                a.add_tolerant_vertex(points[i], tols[i], vnames[i])
            }),
        }
    };
    for &(si, flip) in kept {
        if as_is[si] {
            continue;
        }
        let s = &specs[si];
        // (read by reference: a clone of the face would share its point-in-face index, which
        // the next boolean then finds shared and rebuilds)
        let src = if s.side == 0 {
            a.face(s.from)
        } else {
            b.face(s.from)
        };
        let (sense, bx) = (src.same_sense, src.bbox.get().copied());
        // an extended (planar) face only gains holes inside it: its box stays
        let face = if s.extend && !flip {
            s.from
        } else {
            let surface = src.surface.clone();
            a.add_face(shell, surface, sense != flip, s.name)
        };
        if s.whole
            && !flip
            && let Some(bx) = bx
        {
            a.faces[face.index()].bbox.set(bx);
        }
        for lp in &s.loops {
            let mut uses: Vec<(EdgeId, bool)> = Vec::with_capacity(lp.len());
            for &(p, fwd) in lp {
                let e = match emap[p] {
                    Some(e) => e,
                    None => {
                        let pc = &pieces[p];
                        let e = match pc.whole {
                            // an untouched edge of A keeps its identity
                            Some((0, e)) => e,
                            _ => {
                                let (v0, v1) = (vertex(a, pc.start), vertex(a, pc.end));
                                a.add_edge(pc.curve.clone(), pc.range, v0, v1, pc.name)
                            }
                        };
                        emap[p] = Some(e);
                        e
                    }
                };
                uses.push((e, fwd));
            }
            if flip {
                uses = uses.into_iter().rev().map(|(e, f)| (e, !f)).collect();
            }
            let lp = a.add_loop(face, &uses);
            if s.extend && !flip {
                crate::classify::extend_query(a, face, lp);
            }
        }
    }
    for &(k, t) in grown {
        if let VKey::Old(0, v) = k {
            let x = &mut a.vertices[v.index()];
            x.tol = x.tol.max(t);
        }
    }
    // B kept where no new edge joins it to A: a lump apart (a union of disjoint solids) or a
    // void, a shell of its own (other shells are regrouped when the body is compacted)
    if pieces.iter().all(|p| p.seg.is_none()) && kept.iter().any(|&(si, _)| specs[si].side == 1) {
        a.rebuild_shells();
    }
}

/// `tol` grown to what `grown` asks of vertex `k`.
fn grown_tol(grown: &[(VKey, f64)], k: VKey, tol: f64) -> f64 {
    grown.iter().filter(|g| g.0 == k).fold(tol, |t, g| t.max(g.1))
}

/// Existing vertices that settled cuts and new edges end on, with the distance from each to
/// the farthest curve now ending there where that is beyond the vertex's tolerance (and, for
/// a vertex of B merged into one of A, the reach of B's vertex too).
pub(crate) fn grown_tols(
    bodies: &[&Body],
    cuts: &[Cut],
    cut_vertex: &[VKey],
    segs: &[Seg],
    alias: &[(VKey, VKey)],
    key_point: impl Fn(VKey) -> Point3,
) -> Vec<(VKey, f64)> {
    let mut grown: Vec<(VKey, f64)> = Vec::new();
    let mut grow = |k: VKey, d: f64| {
        if let VKey::Old(sd, v) = k
            && d > bodies[sd].vertex(v).tol
        {
            grown.push((k, d));
        }
    };
    for (c, &k) in cuts.iter().zip(cut_vertex) {
        let Some((sd, e)) = c.on.edge() else { continue };
        let edge = bodies[sd].edge(e);
        // an edge already ends on its own vertex; only a split at another body's vertex is new
        if k == VKey::Old(sd, edge.start) || k == VKey::Old(sd, edge.end) {
            continue;
        }
        grow(k, edge.curve.eval(c.s).distance(key_point(k)));
    }
    for sg in segs {
        let Some((i0, i1)) = sg.ends else { continue };
        for (t, k) in [(sg.range.lo, cut_vertex[i0]), (sg.range.hi, cut_vertex[i1])] {
            grow(k, sg.curve.eval(t).distance(key_point(k)));
        }
    }
    for &(from, to) in alias {
        if let VKey::Old(sd, v) = from {
            let x = bodies[sd].vertex(v);
            grow(to, x.point.distance(key_point(to)) + x.tol);
        }
    }
    grown
}

/// Two faces touching without crossing: a point inside both where their surfaces are tangent,
/// and the touching curve's direction there (None for an isolated point).
pub(crate) struct Touch {
    pub(crate) sides: [Side; 2],
    pub(crate) faces: [FaceId; 2],
    pub(crate) at: Point3,
    pub(crate) dir: Option<Vec3>,
}

/// Where faces `faces` (of bodies `sides`) touch: an isolated tangent point or points along a
/// tangent curve, lying in both faces (a ball's pole is on its face's boundary).
pub(crate) fn touches(
    found: &SurfaceSurface,
    index: [&Index; 2],
    sides: [Side; 2],
    faces: [FaceId; 2],
    region: forge_math::Box3,
) -> Vec<Touch> {
    let inside = |p: Point3| (0..2).all(|k| index[k].point_in_face(faces[k], p) != Where::Outside);
    let mut out = Vec::new();
    match found {
        SurfaceSurface::Points(ps) => {
            for &at in ps.iter().filter(|&&p| region.contains(p) && inside(p)) {
                out.push(Touch {
                    sides,
                    faces,
                    at,
                    dir: None,
                });
            }
        }
        SurfaceSurface::Curves(cs) => {
            for (c, contact) in cs {
                if *contact != Contact::Touching {
                    continue;
                }
                let d = c.domain();
                let range = match (c, d.period) {
                    (_, Some(per)) => Interval::new(d.lo, d.lo + per),
                    (Curve::Line(l), _) => match line_box(l, &region) {
                        Some(r) => r,
                        None => continue,
                    },
                    _ if d.lo.is_finite() && d.hi.is_finite() => Interval::new(d.lo, d.hi),
                    _ => continue,
                };
                // points along the curve: other solids decide what is kept at each
                for k in 0..9 {
                    let t = range.lo + range.width() * (k as f64 + 0.5) / 9.0;
                    let dv = c.deriv(t);
                    if inside(dv.p) {
                        out.push(Touch {
                            sides,
                            faces,
                            at: dv.p,
                            dir: Some(dv.d1),
                        });
                    }
                }
            }
        }
        _ => {}
    }
    out
}

/// How face `f` lies around a point `p` of it where it touches another solid: `class` of
/// points of the face a growing step away from `p` across the touch, the first step at which
/// every direction landing on the face gives an answer (None from `class` is too close to
/// call). None if the directions disagree (the face crosses there) or no step decides.
pub(crate) fn local_class<T: PartialEq>(
    body: &Body,
    own: &Index,
    f: FaceId,
    p: Point3,
    dir: Option<Vec3>,
    class: impl Fn(Point3) -> Option<T>,
) -> Option<T> {
    let face = body.face(f);
    let n = body.face_normal(f, face.surface.project(p))?;
    let dirs: Vec<Vec3> = match dir {
        Some(d) => {
            let w = n.cross(d).normalize()?.get();
            vec![w, -w]
        }
        None => {
            // turned off the frame's axes, so no probe runs along a seam to a pole
            let fr = Frame::from_z(p, n);
            let (sn, cs) = dmath::sin_cos(0.3719);
            let x = fr.x().get() * cs + fr.y().get() * sn;
            let y = n.cross(x);
            vec![x, -x, y, -y]
        }
    };
    let mut step = 1e-3;
    while step < 0.3 {
        let mut got: Option<T> = None;
        let mut undecided = false;
        for &w in &dirs {
            let q = face.surface.eval(face.surface.project(p + w * step));
            // the face, or one beside it on the same surface (a ball's other half)
            let on_face = own.point_in_face(f, q) == Where::Inside
                || body.faces_where(|b| b.contains(q)).any(|g| {
                    g != f
                        && body.face(g).surface == face.surface
                        && body.face(g).same_sense == face.same_sense
                        && own.point_in_face(g, q) == Where::Inside
                });
            // at a touch on the face's boundary only the directions into it say how it lies
            if !on_face {
                continue;
            }
            match class(q) {
                None => {
                    undecided = true;
                    break;
                }
                Some(c) => {
                    if got.as_ref().is_some_and(|g| *g != c) {
                        return None;
                    }
                    got = Some(c);
                }
            }
        }
        if !undecided {
            return got;
        }
        step *= 4.0;
    }
    None
}

/// Cuts `curve` (on both faces' surfaces) where it crosses either face's
/// boundary and records the pieces lying inside both faces.
#[allow(clippy::too_many_arguments)]
pub(crate) fn trim(
    bodies: [&Body; 2],
    index: [&Index; 2],
    sides: [Side; 2],
    faces: [FaceId; 2],
    curve: Curve,
    name: Name,
    // whether the intersection has other branches; and whether they may meet this one (traced)
    branches: [bool; 2],
    cuts: &mut Vec<Cut>,
    segs: &mut Vec<Seg>,
    imprints: &mut Vec<Imprint>,
) -> R<()> {
    let domain = curve.domain();
    let closed = domain.period.is_some();
    let period = domain.period.unwrap_or(0.0);
    let range = if closed {
        Interval::new(domain.lo, domain.lo + period)
    } else if domain.lo.is_finite() && domain.hi.is_finite() {
        Interval::new(domain.lo, domain.hi)
    } else {
        let both = index[0]
            .face_box(faces[0])
            .hull(&index[1].face_box(faces[1]))
            .grow(1.0);
        // an open conic's parameter is bounded by its distance from its frame's origin:
        // the y coordinate (over b for a hyperbola)
        let reach = |f: &forge_math::Frame| {
            let c = both.center();
            c.distance(f.origin) + both.diagonal() / 2.0
        };
        match &curve {
            Curve::Line(line) => match line_box(line, &both) {
                Some(r) => r,
                None => return Ok(()),
            },
            Curve::Parabola(p) => Interval::new(-reach(&p.frame), reach(&p.frame)),
            Curve::Hyperbola(h) => Interval::new(-reach(&h.frame) / h.b, reach(&h.frame) / h.b),
            _ => return Err(BoolError::Unsupported("open curve without a closed form")),
        }
    };
    let first = cuts.len();
    let first_seg = segs.len();
    let mut ts: Vec<(f64, usize)> = Vec::new();
    // edges the curve runs along: (side, edge, is a seam)
    let mut along: Vec<(Side, EdgeId, bool)> = Vec::new();
    // an edge whose box misses the curve's cannot cut it (big faces have hundreds of edges)
    let reach = curve.bbox(range).grow(tol::LINEAR);
    for k in 0..2 {
        let (body, f, side) = (bodies[k], faces[k], sides[k]);
        for &l in &body.face(f).loops {
            for &c in &body.lp(l).coedges {
                let e = body.coedge(c).edge;
                let edge = body.edge(e);
                if !body.edge_box(e).overlaps(&reach) {
                    continue;
                }
                if cuts[first..].iter().any(|x| x.on == CutOn::Edge(side, e)) {
                    continue; // a seam is visited twice
                }
                let across = across(body, f, c)?;
                let hits = match curve_surface(&curve, range, &across) {
                    CurveSurface::Hits(h) => h,
                    // the curve runs along this edge: cut it at the edge's ends; the stretch along
                    // the edge is boundary, not a new edge (the faces either side of it meet the
                    // other body there, and their own pairs decide what happens)
                    CurveSurface::OnSurface | CurveSurface::Overlap => {
                        let g = body.partner(c).map(|o| body.coedge_face(o));
                        let seam = g == Some(f);
                        // beside a face on the other face's surface, coplanar imprinting does it
                        let other = &bodies[1 - k].face(faces[1 - k]).surface;
                        let coplanar =
                            g.is_some_and(|g| g != f && ssi::coincident(&body.face(g).surface, other));
                        if !coplanar && !along.iter().any(|a| a.0 == k && a.1 == e) {
                            along.push((k, e, seam));
                        }
                        for (s, v) in [(edge.range.lo, edge.start), (edge.range.hi, edge.end)] {
                            let vx = body.vertex(v);
                            let t = curve.closest(vx.point, range);
                            if curve.eval(t).distance(vx.point) <= vx.tol {
                                ts.push((t, cuts.len()));
                                cuts.push(Cut {
                                    on: CutOn::Edge(side, e),
                                    s,
                                    point: vx.point,
                                    tol: vx.tol,
                                    touch: branches[1],
                                });
                            }
                        }
                        continue;
                    }
                };
                // a "touch" may be two crossings a hair apart (near-tangent circles), so look; a
                // curve that only touches the boundary from outside the face is not cut there, but
                // one touching it from inside meets the edge there and goes on: a vertex on both
                // (a step out from the touch until it leaves the boundary's tolerance: tangent,
                // it parts from the boundary only quadratically, so the steps grow to a traced
                // curve's chain spacing, one unit of its parameter)
                // (a step out of the face on either side ends it: the curve crosses out there)
                let touch_inside = |h: &CurveHit| {
                    let side = |t: f64| {
                        if range.contains(t) || closed {
                            index[k].point_in_face(f, curve.eval(t))
                        } else {
                            Where::Outside
                        }
                    };
                    // the side of the surface across the edge the curve keeps to: the face's
                    // outside there, it touches from outside (cheaper than asking the face)
                    let inner = |s: f64| {
                        let d = edge.curve.deriv(s).d1;
                        let along = if body.coedge(c).forward { d } else { -d };
                        let Some(n) = body.face_normal(f, body.face(f).surface.project(h.point)) else {
                            return true;
                        };
                        let inward = across.gradient(h.point).dot(n.cross(along));
                        let dt = 5e-2 * range.width().min(1.0);
                        let g = across.implicit(curve.eval(h.t - dt)) + across.implicit(curve.eval(h.t + dt));
                        g * inward >= 0.0 || g.abs() <= 1e-7
                    };
                    index[1 - k]
                        .face_box(faces[1 - k])
                        .grow(tol::LINEAR)
                        .contains(h.point)
                        && on_edge(body, e, h.point).is_some_and(|(s, _)| inner(s))
                        && [1e-3, 1e-2, 5e-2, 0.2, 0.5].iter().find_map(|&w| {
                            let dt = w * range.width().min(1.0);
                            match side(h.t - dt) {
                                Where::Outside => Some(false),
                                a => match side(h.t + dt) {
                                    Where::Outside => Some(false),
                                    b => (a == Where::Inside && b == Where::Inside).then_some(true),
                                },
                            }
                        }) == Some(true)
                };
                for (h, grazing) in hits.into_iter().flat_map(|h| {
                    let grazing = h.contact != Contact::Crossing;
                    let near = crossings_near(&curve, range, &across, h);
                    let touch = (grazing && near.is_empty() && touch_inside(&h)).then_some(h);
                    near.into_iter().chain(touch).map(move |x| (x, grazing))
                }) {
                    // off the other face no piece of the curve starts here: cutting the edge would
                    // only split it where nothing meets it (and not its coincident twin). Its box
                    // says so cheaply; a grazing crossing, or one within two tolerances of the
                    // box's side (where a face's own boundary lies), is checked exactly
                    let other = index[1 - k].face_box(faces[1 - k]);
                    if let Some((s, tol)) = on_edge(body, e, h.point)
                        && other.grow(tol::LINEAR).contains(h.point)
                        && (!(grazing || !other.contains(h.point))
                            || index[1 - k].point_in_face(faces[1 - k], h.point) != Where::Outside)
                    {
                        ts.push((h.t, cuts.len()));
                        cuts.push(Cut {
                            on: CutOn::Edge(side, e),
                            s,
                            point: h.point,
                            tol,
                            touch: grazing || branches[1],
                        });
                    }
                }
            }
        }
    }
    // where the curve runs along an existing edge it crosses the other face there: the edge is
    // cut where it leaves that face and imprinted into it (splitting that face, and stopping the
    // classification flood across it); along a seam the curve adds no edge at all
    if !along.is_empty() {
        for &(k, e, _) in &along {
            let (body, side) = (bodies[k], sides[k]);
            let edge = body.edge(e);
            let mut marks = vec![edge.range.lo, edge.range.hi];
            for ci in first..cuts.len() {
                if cuts[ci].on.edge().is_some_and(|(sd, _)| sd == side) {
                    continue;
                }
                let p = cuts[ci].point;
                if let Some((s, tol)) = on_edge(body, e, p) {
                    cuts.push(Cut {
                        on: CutOn::Edge(side, e),
                        s,
                        point: p,
                        tol: tol.max(cuts[ci].tol),
                        touch: cuts[ci].touch,
                    });
                    marks.push(s);
                }
            }
            marks.sort_by(f64::total_cmp);
            for w in marks.windows(2) {
                if w[1] - w[0] <= 1e-12 * (1.0 + edge.range.width()) {
                    continue;
                }
                let into = faces[1 - k];
                let m = edge.curve.eval(0.5 * (w[0] + w[1]));
                if index[1 - k].point_in_face(into, m) == Where::Inside {
                    imprints.push(Imprint {
                        side,
                        edge: e,
                        range: Interval::new(w[0], w[1]),
                        into,
                        into_side: sides[1 - k],
                    });
                }
            }
        }
        if along.iter().any(|a| a.2) {
            return Ok(());
        }
    }
    // where the curve passes a point at which the surfaces touch, branches of the intersection
    // cross: a vertex there; and an open curve ending inside both
    // faces ends where its branches meet: a vertex too, unless a boundary already cuts it there
    let mut free: Vec<f64> = if !branches[1] {
        Vec::new()
    } else {
        touch_params(
            &curve,
            range,
            [
                &bodies[0].face(faces[0]).surface,
                &bodies[1].face(faces[1]).surface,
            ],
        )
    };
    if branches[0] && !closed && !matches!(curve, Curve::Line(_)) {
        free.extend([range.lo, range.hi]);
    }
    {
        for t in free {
            let p = curve.eval(t);
            let cut = cuts[first..]
                .iter()
                .any(|c| c.point.distance(p) <= c.tol.max(tol::LINEAR));
            if !cut
                && (0..2).all(|k| index[k].face_box(faces[k]).grow(tol::LINEAR).contains(p))
                && (0..2).all(|k| index[k].point_in_face(faces[k], p) != Where::Outside)
            {
                ts.push((t, cuts.len()));
                cuts.push(Cut {
                    on: CutOn::Free,
                    s: t,
                    point: p,
                    tol: tol::LINEAR,
                    touch: true,
                });
            }
        }
    }
    ts.sort_by(|x, y| x.0.total_cmp(&y.0));
    if debug_on() {
        let tsp: Vec<_> = ts.iter().map(|&(t, k)| (t, cuts[k].point)).collect();
        eprintln!("  curve {faces:?} range {range:?} cuts {tsp:?}");
    }
    // the same point found on two edges (a vertex, or both faces' boundaries) is one cut; a
    // curve coming back to a point it passed (a branch crossing itself where the surfaces touch)
    // meets it again, a cut each time
    ts.dedup_by(|x, y| {
        let (a, b) = (&cuts[x.1], &cuts[y.1]);
        let reach = a.tol.max(b.tol);
        a.point.distance(b.point) <= reach && curve.eval(0.5 * (x.0 + y.0)).distance(a.point) <= reach
    });
    // and round the end of a closed curve's period
    if closed && ts.len() > 1 {
        let ((t0, i0), (t1, i1)) = (ts[0], ts[ts.len() - 1]);
        let (a, b) = (&cuts[i0], &cuts[i1]);
        let reach = a.tol.max(b.tol);
        if a.point.distance(b.point) <= reach
            && curve.eval(0.5 * (t1 + t0 + period)).distance(a.point) <= reach
        {
            ts.pop();
        }
    }
    // such a loop from a point back to it gets a vertex half way: an edge ending where it starts
    // is a whole period of its curve or nothing
    let loops: Vec<f64> = (0..ts.len())
        .filter_map(|k| {
            let (t0, i0) = ts[k];
            let (t1, i1) = match ts.get(k + 1) {
                Some(&n) => n,
                None if closed => (ts[0].0 + period, ts[0].1),
                None => return None,
            };
            // (cut points of one vertex lie within its tolerance, so up to twice it apart)
            let (a, b) = (&cuts[i0], &cuts[i1]);
            let reach = 2.0 * a.tol.max(b.tol);
            let mid = 0.5 * (t0 + t1);
            let whole = closed && ts.len() == 1;
            (!whole && a.point.distance(b.point) <= reach && curve.eval(mid).distance(a.point) > reach)
                .then_some(mid)
        })
        .collect();
    for t in loops {
        let t = if closed && t >= range.hi { t - period } else { t };
        ts.push((t, cuts.len()));
        cuts.push(Cut {
            on: CutOn::Free,
            s: t,
            point: curve.eval(t),
            tol: tol::LINEAR,
            touch: true,
        });
    }
    ts.sort_by(|x, y| x.0.total_cmp(&y.0));
    // a stretch on a boundary is an existing edge, never a new one; a stretch may still touch
    // a boundary at a point (circles tangent there), so a sample landing on one is tried again,
    // away from the ends (a cut is placed only to within tolerance). The edges a stretch crosses
    // at its ends (inside them, not running along them) do not put it on the boundary.
    let crossed = |k: usize, ends: &[usize]| -> Vec<EdgeId> {
        cuts[first..]
            .iter()
            .filter_map(|c| match c.on {
                CutOn::Edge(sd, e) if sd == sides[k] => Some((c, e)),
                _ => None,
            })
            .filter(|&(c, e)| {
                let r = bodies[k].edge(e).range;
                let w = 1e-9 * (1.0 + r.width());
                c.s > r.lo + w
                    && c.s < r.hi - w
                    && !along.iter().any(|a| a.0 == k && a.1 == e)
                    && ends
                        .iter()
                        .any(|&i| cuts[i].point.distance(c.point) <= cuts[i].tol.max(c.tol))
            })
            .map(|(_, e)| e)
            .collect()
    };
    // a line may run on past where a procedural surface holds it (a screw face's section line
    // carried through the axis, into the opposite half plane): a stretch there is on no face
    let off = |p: Point3| {
        matches!(curve, Curve::Line(_))
            && (0..2).any(|k| {
                let s = &bodies[k].face(faces[k]).surface;
                matches!(s, Surface::Procedural(_)) && s.distance(p) > 10.0 * tol::LINEAR
            })
    };
    let inside = |t0: f64, t1: f64, ends: &[usize]| -> R<bool> {
        let at = |k: usize, p: Point3| match index[k].point_in_face(faces[k], p) {
            Where::On => {
                let past = crossed(k, ends);
                if past.is_empty() {
                    Where::On
                } else {
                    index[k].point_in_face_past(faces[k], p, &past)
                }
            }
            w => w,
        };
        // no boundary of either face crosses the stretch: a face it is inside of somewhere it is
        // inside of everywhere, and only the one it is on needs another sample
        let mut w = [Where::On; 2];
        for f in [0.5, 0.381_966_011_250_105_1, 0.618_033_988_749_894_9] {
            let t = t0 + (t1 - t0) * f;
            let p = curve.eval(t);
            if f != 0.5 && curve.eval(t0).distance(p).min(curve.eval(t1).distance(p)) <= 4.0 * tol::LINEAR {
                break;
            }
            // a stretch between two points of the boundary it is on runs along it: sampled again
            // only where it touches a boundary that does not end it. A whole closed curve (no
            // cut, or one) touching a boundary would need a vertex there, which no sample gives
            if f != 0.5
                && (ends.is_empty()
                    || ends[0] == ends[1]
                    || (0..2).any(|k| {
                        w[k] == Where::On
                            && cuts[first..].iter().any(|c| {
                                c.on.edge().is_some_and(|(sd, _)| sd == sides[k])
                                    && ends
                                        .iter()
                                        .any(|&i| cuts[i].point.distance(c.point) <= cuts[i].tol.max(c.tol))
                            })
                    }))
            {
                break;
            }
            if off(p) {
                return Ok(false);
            }
            for (k, wk) in w.iter_mut().enumerate() {
                if *wk == Where::On {
                    *wk = at(k, p);
                    // off either face the stretch is no new edge, whatever the other says
                    if *wk == Where::Outside {
                        return Ok(false);
                    }
                }
            }
            if debug_on() && matches!(curve, Curve::Circle(_)) {
                eprintln!("  trim {faces:?} t {t:.4} p {p:?}: {:?} {:?}", w[0], w[1]);
            }
            if w == [Where::Inside; 2] {
                return Ok(true);
            }
        }
        Ok(false)
    };
    if ts.is_empty() {
        if closed && inside(range.lo, range.hi, &[])? {
            segs.push(Seg {
                curve: curve.clone(),
                range,
                ends: None,
                faces,
                sides,
                name,
            });
        }
        return Ok(());
    }
    let n = ts.len();
    let count = if closed { n } else { n - 1 };
    for k in 0..count {
        let (t0, i0) = ts[k];
        let (mut t1, i1) = ts[(k + 1) % n];
        if k + 1 == n {
            t1 += period;
        }
        if t1 - t0 <= 1e-12 {
            continue;
        }
        if inside(t0, t1, &[i0, i1])? {
            segs.push(Seg {
                curve: curve.clone(),
                range: Interval::new(t0, t1),
                ends: Some((i0, i1)),
                faces,
                sides,
                name,
            });
        }
    }
    // a piece of the curve is named by the edges it runs between, not its place along the curve;
    // pieces between the same two edges by where they start along the first
    // a free cut has no edge: it is named by its rank among the curve's free cuts
    let free: Vec<usize> = ts
        .iter()
        .map(|x| x.1)
        .filter(|&i| cuts[i].on == CutOn::Free)
        .collect();
    let edge_name = |i: usize| match cuts[i].on {
        CutOn::Edge(sd, e) => {
            let k = if sides[0] == sd { 0 } else { 1 };
            bodies[k].edge(e).name
        }
        CutOn::Free => {
            let rank = free.iter().position(|&j| j == i).unwrap_or(free.len());
            name.with_str("free").with(rank as u64)
        }
    };
    let keys: Vec<(Name, Name, f64)> = segs[first_seg..]
        .iter()
        .map(|sg| {
            let (i0, i1) = sg.ends.expect("an open piece");
            (edge_name(i0), edge_name(i1), cuts[i0].s)
        })
        .collect();
    for (j, sg) in segs[first_seg..].iter_mut().enumerate() {
        let (a, b, s) = keys[j];
        let rank = keys.iter().filter(|k| k.0 == a && k.1 == b && k.2 < s).count();
        sg.name = name.with_name(a).with_name(b).with(rank as u64);
    }
    Ok(())
}

/// The new curve pieces ending at each new vertex, each with which end: (point, name).
pub(crate) fn curve_ends(segs: &[Seg], cut_vertex: &[VKey]) -> Vec<(u32, Name)> {
    let mut out = Vec::with_capacity(2 * segs.len());
    for sg in segs {
        let Some((i0, i1)) = sg.ends else { continue };
        for (k, end) in [(cut_vertex[i0], 0), (cut_vertex[i1], 1)] {
            if let VKey::New(i) = k {
                out.push((i, Name(sg.name.0 ^ end)));
            }
        }
    }
    out
}

/// Names for new vertices from what meets there (`marks`: point and name: the curve pieces
/// ending there, else the edges cut), not from the order they were found in. Points where the
/// same ones meet are told apart by where they lie.
pub(crate) fn vertex_names(points: &[Point3], marks: Vec<(u32, Name)>, root: Name) -> Vec<Name> {
    let n = points.len();
    // grouped by point (a counting sort), each group's names in order
    let mut start = vec![0usize; n + 1];
    for m in &marks {
        start[m.0 as usize + 1] += 1;
    }
    for i in 0..n {
        start[i + 1] += start[i];
    }
    let mut fill = start.clone();
    let mut by_point = vec![Name::NONE; marks.len()];
    for m in marks {
        by_point[fill[m.0 as usize]] = m.1;
        fill[m.0 as usize] += 1;
    }
    let root = root.with_str("vertex");
    let keys: Vec<Name> = (0..n)
        .map(|i| {
            let group = &mut by_point[start[i]..start[i + 1]];
            group.sort_unstable();
            // a cheap mix: there are many of these, and the names mixed are hashes already
            let mut key = root.0;
            for k in 0..group.len() {
                if k == 0 || group[k] != group[k - 1] {
                    key = (key ^ group[k].0).wrapping_mul(0x9e37_79b9_7f4a_7c15);
                    key ^= key >> 29;
                }
            }
            Name(key)
        })
        .collect();
    let at = |i: usize| (points[i].x, points[i].y, points[i].z);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| {
        keys[i]
            .cmp(&keys[j])
            .then(at(i).partial_cmp(&at(j)).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut out = vec![Name::NONE; n];
    let mut rank = 0;
    for (w, &i) in order.iter().enumerate() {
        rank = if w > 0 && keys[order[w - 1]] == keys[i] {
            rank + 1
        } else {
            0
        };
        out[i] = keys[i].with(rank);
    }
    out
}

/// A name for a piece of a split face from its boundary, not its place in the split's order:
/// the least of its boundary pieces' names, each with the way it runs (a piece runs each way
/// round at most one region of a face).
pub(crate) fn region_name(face: Name, loops: &[Vec<(usize, bool)>], pieces: &[Piece]) -> Name {
    let least = loops
        .iter()
        .flatten()
        .map(|&(p, fwd)| (pieces[p].name, fwd))
        .min();
    face.piece(least.map_or(Name::NONE, |(n, fwd)| n.with(fwd as u64)))
}

/// For each key, whether it is the only one of its value.
pub(crate) fn alone<K: Ord + Copy>(keys: &[K]) -> Vec<bool> {
    let mut sorted = keys.to_vec();
    sorted.sort_unstable();
    keys.iter()
        .map(|k| sorted.get(sorted.partition_point(|x| x < k) + 1) != Some(k))
        .collect()
}

/// Names the kept pieces of split faces. A face or edge split by the boolean of which only one
/// piece is kept keeps its own name: it was only trimmed.
fn sole_pieces_keep_names(
    bodies: &[&Body; 2],
    specs: &mut [Spec],
    kept: &[(usize, bool)],
    pieces: &mut [Piece],
    uses: &[u8],
    split_runs: &[(Side, EdgeId, std::ops::Range<usize>)],
    vname: &dyn Fn(VKey) -> Name,
) {
    for (side, e, run) in split_runs {
        let edge = bodies[*side].edge(*e).name;
        let used: Vec<usize> = run.clone().filter(|&p| uses[p] > 0).collect();
        for &p in &used {
            pieces[p].name = if used.len() == 1 {
                edge
            } else {
                piece_name(edge, &pieces[p], vname)
            };
        }
    }
    // a face's specs are pushed together, and kept in order: its kept pieces are one run
    let mut at = 0;
    while at < kept.len() {
        let key = |k: usize| (specs[kept[k].0].side, specs[kept[k].0].from);
        let mut end = at + 1;
        while end < kept.len() && key(end) == key(at) {
            end += 1;
        }
        for k in at..end {
            let s = &mut specs[kept[k].0];
            if end == at + 1 && !s.whole {
                s.name = bodies[s.side].face(s.from).name;
            } else if s.name == Name::NONE {
                s.name = region_name(bodies[s.side].face(s.from).name, &s.loops, pieces);
            }
        }
        at = end;
    }
}

/// A piece of edge `edge` named by its ends, not its place in the order along the edge.
pub(crate) fn piece_name(edge: Name, pc: &Piece, vname: &dyn Fn(VKey) -> Name) -> Name {
    edge.piece(vname(pc.start).with_name(vname(pc.end)))
}

/// The surface that meets face `f`'s surface along the edge of coedge `c`:
/// the neighbouring face's, or for a seam (or an edge between two faces on
/// one surface, like revolve halves) the plane through the edge, across it.
pub(crate) fn across(body: &Body, f: FaceId, c: forge_topo::CoedgeId) -> R<Surface> {
    let edge = body.edge(body.coedge(c).edge);
    if let Curve::Procedural(pc) = &edge.curve
        && let Some(s) = pc.across()
    {
        return Ok(s);
    }
    let t = edge.range.mid();
    let d = edge.curve.deriv(t);
    let s = &body.face(f).surface;
    let n = s
        .normal(s.project(d.p))
        .ok_or(BoolError::Internal("edge at a singular point"))?;
    // a surface through the edge, across this one: the edge's own plane for a conic (the
    // cylinder on a circle lying in a planar face or touching it), or the plane holding a line
    // and the normal
    let edge_plane = || -> R<Surface> {
        Ok(match &edge.curve {
            // in a planar face, or where its plane touches the face (a swept tube's side): the
            // cylinder through it stands across the face
            Curve::Circle(k) if matches!(s, Surface::Plane(_)) || n.dot(k.frame.z()).abs() > 1.0 - 1e-6 => {
                Surface::Cylinder(Cylinder {
                    frame: k.frame,
                    radius: k.radius,
                })
            }
            Curve::Circle(k) => Surface::Plane(Plane::new(k.frame)),
            // in a planar face, the round cylinder of the minor radius whose axis leans over the
            // major one holds it (an oblique section of that cylinder) and stands across the face
            Curve::Ellipse(e) if matches!(s, Surface::Plane(_)) || n.dot(e.frame.z()).abs() > 1.0 - 1e-6 => {
                let lean = (1.0 - (e.minor / e.major) * (e.minor / e.major)).max(0.0).sqrt();
                let axis = (e.frame.z() * (e.minor / e.major) + e.frame.x() * lean)
                    .normalize()
                    .ok_or(BoolError::Internal("degenerate edge"))?;
                Surface::Cylinder(Cylinder {
                    frame: Frame::from_z(e.frame.origin, axis),
                    radius: e.minor,
                })
            }
            Curve::Ellipse(e) => Surface::Plane(Plane::new(e.frame)),
            // likewise the parabolic wall square to a parabola's plane (a vertex blend's rim)
            Curve::Parabola(pa) if n.dot(pa.frame.z()).abs() > 1.0 - 1e-6 => {
                let h = pa.frame.z() * edge.range.width().max(1.0);
                let rail = |o: Vec3| forge_geom::ruled::Rail::Parabola {
                    curve: forge_geom::Parabola {
                        frame: pa.frame.with_origin(pa.frame.origin + o),
                        focal: pa.focal,
                    },
                    range: edge.range,
                };
                match forge_geom::ruled::Ruled::new(rail(-h), rail(h)) {
                    Some(w) => w.surface(),
                    None => return Err(BoolError::Internal("degenerate edge")),
                }
            }
            _ => {
                let z =
                    d.d1.cross(n.get())
                        .normalize()
                        .ok_or(BoolError::Internal("degenerate edge"))?;
                Surface::Plane(Plane::new(Frame::from_z(d.p, z)))
            }
        })
    };
    match body.partner(c).map(|o| body.coedge_face(o)) {
        // a neighbour on this face's surface (coplanar faces framed apart) is not across it
        Some(g) if g != f && body.face(g).surface != *s && !ssi::coincident(&body.face(g).surface, s) => {
            let other = &body.face(g).surface;
            // a smooth edge (the faces meet tangentially, as a rounded corner meets its
            // walls): the neighbour's surface would only touch a crossing curve there
            let smooth = other
                .normal(other.project(d.p))
                .is_some_and(|m| m.cross(n).length() <= 1e-6);
            if smooth && !matches!(edge.curve, Curve::Intersection(_)) {
                edge_plane()
            } else {
                Ok(other.clone())
            }
        }
        _ => edge_plane(),
    }
}

/// Imprints the boundary of `src_face` (of body `src`) onto the coplanar
/// face `dst_face` of the other body: cuts where its edges cross the
/// destination's boundary (on both edges), and records the parts lying inside.
#[allow(clippy::too_many_arguments)]
pub(crate) fn imprint(
    bodies: [&Body; 2],
    index: [&Index; 2],
    sides: [Side; 2],
    src: usize,
    src_face: FaceId,
    dst_face: FaceId,
    cuts: &mut Vec<Cut>,
    imprints: &mut Vec<Imprint>,
) -> R<()> {
    let dst = 1 - src;
    let (sb, db) = (bodies[src], bodies[dst]);
    // how far a point an edge reaches may be from its box: its ends' tolerance
    let slack = |body: &Body, e: EdgeId| {
        let edge = body.edge(e);
        body.vertex(edge.start).tol.max(body.vertex(edge.end).tol)
    };
    for &l in &sb.face(src_face).loops {
        for &c in &sb.lp(l).coedges {
            let e = sb.coedge(c).edge;
            // a seam is used twice by the face: once is enough
            if sb
                .partner(c)
                .is_some_and(|o| o < c && sb.coedge_face(o) == src_face)
            {
                continue;
            }
            let edge = sb.edge(e);
            let (reach, grow) = (sb.edge_box(e), slack(sb, e));
            let mut ts: Vec<f64> = Vec::new();
            for &l2 in &db.face(dst_face).loops {
                for &c2 in &db.lp(l2).coedges {
                    let e2 = db.coedge(c2).edge;
                    // an edge clear of this one's box can neither cross it nor share its carrier
                    if !db.edge_box(e2).grow(grow + slack(db, e2)).overlaps(&reach) {
                        continue;
                    }
                    let hits = match curve_surface(&edge.curve, edge.range, &across(db, dst_face, c2)?) {
                        CurveSurface::Hits(h) => h,
                        // lying in both the face's plane and the neighbour's surface, the edge runs
                        // along the same carrier as e2: cut each at the other's ends inside it
                        _ => {
                            coincide(sb, sides[src], e, db, sides[dst], e2, cuts);
                            continue;
                        }
                    };
                    for h in hits {
                        if let Some((s2, tol)) = on_edge(db, e2, h.point) {
                            cuts.push(Cut {
                                on: CutOn::Edge(sides[dst], e2),
                                s: s2,
                                point: h.point,
                                tol,
                                touch: false,
                            });
                            cuts.push(Cut {
                                on: CutOn::Edge(sides[src], e),
                                s: h.t,
                                point: h.point,
                                tol,
                                touch: false,
                            });
                            ts.push(h.t);
                        }
                    }
                }
            }
            ts.sort_by(f64::total_cmp);
            let mut marks = vec![edge.range.lo];
            marks.extend(
                ts.iter()
                    .copied()
                    .filter(|&t| t > edge.range.lo && t < edge.range.hi),
            );
            marks.push(edge.range.hi);
            for w in marks.windows(2) {
                if w[1] - w[0] <= 1e-12 * (1.0 + edge.range.width()) {
                    continue;
                }
                match index[dst].point_in_face(dst_face, edge.curve.eval(0.5 * (w[0] + w[1]))) {
                    Where::Inside => imprints.push(Imprint {
                        side: sides[src],
                        edge: e,
                        range: Interval::new(w[0], w[1]),
                        into: dst_face,
                        into_side: sides[dst],
                    }),
                    // along the destination's boundary: the shared part of coincident edges
                    Where::On | Where::Outside => {}
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn overlap(a: &forge_math::Box3, b: &forge_math::Box3) -> forge_math::Box3 {
    let cut = |x: Interval, y: Interval| Interval::new(x.lo.max(y.lo), x.hi.min(y.hi));
    forge_math::Box3 {
        x: cut(a.x, b.x),
        y: cut(a.y, b.y),
        z: cut(a.z, b.z),
    }
}

/// Whether two traced branches of one intersection may cross (where the surfaces touch): their
/// chains' boxes overlap.
pub(crate) fn branches_meet(curves: &[(Curve, Contact)]) -> bool {
    let boxes: Vec<Box3> = curves
        .iter()
        .filter_map(|c| match c {
            (Curve::Intersection(k), Contact::Crossing) => {
                Some(Box3::from_points(k.pts.iter().copied()).grow(k.sag + tol::LINEAR))
            }
            (e @ Curve::Ellipse(_), Contact::Crossing) => {
                Some(e.bbox(Interval::new(0.0, dmath::TAU)).grow(tol::LINEAR))
            }
            _ => None,
        })
        .collect();
    (0..boxes.len()).any(|i| boxes[i + 1..].iter().any(|b| boxes[i].overlaps(b)))
}

/// Parameters of curve `c` (on both surfaces) where the surfaces touch: the normals there are
/// parallel. Only a traced curve between two analytic surfaces passes such a point (a conic
/// meets its surfaces across; a blend's curves are left alone): its chain points are scanned,
/// and each dip of the angle between the normals narrowed down along the curve.
///
/// These are the singular ("characteristic") points of surface-surface intersection in
/// Patrikalakis, Maekawa and Cho's Shape Interrogation (ch. 5, intersection problems): where the normals are collinear,
/// and where branches cross (Ye and Maekawa's branch points). A marching tracer is unreliable
/// there, so the point is found from the normals, not from the curve's own tangent; a crossing
/// is told from a smooth dip in the angle by the angle's falling off linearly into it.
/// - https://web.mit.edu/hyperbook/Patrikalakis-Maekawa-Cho/
fn touch_params(c: &Curve, range: Interval, s: [&Surface; 2]) -> Vec<f64> {
    if s.iter()
        .any(|x| matches!(x, Surface::Plane(_) | Surface::Procedural(_)))
    {
        return Vec::new();
    }
    // (the gradients of the analytic surfaces other than a plane are unit normals)
    let sin_at = |p: Point3| s[0].gradient(p).cross(s[1].gradient(p)).length();
    let k = match c {
        Curve::Intersection(k) => k,
        // two equal cylinders' ellipses cross at the ends of their minor axes
        Curve::Ellipse(_) => {
            return (0..4)
                .map(|i| range.lo + 0.5 * dmath::PI * i as f64)
                .filter(|&t| sin_at(c.eval(t)) <= 1e-9)
                .collect();
        }
        _ => return Vec::new(),
    };
    let sin = |t: f64| sin_at(c.eval(t));
    // chain point i sits at t = i
    let n = k.pts.len();
    let stride = (n / 12).max(1);
    let v: Vec<f64> = k.pts.iter().step_by(stride).map(|&p| sin_at(p)).collect();
    let mut out = Vec::new();
    for (m, i) in (0..n).step_by(stride).enumerate() {
        let (l, r) = (
            if m == 0 { f64::INFINITY } else { v[m - 1] },
            v.get(m + 1).copied().unwrap_or(f64::INFINITY),
        );
        if !(v[m] <= l && v[m] < r && v[m] < 0.3) {
            continue;
        }
        // the lowest chain point near the dip, and its neighbours
        let near = i.saturating_sub(stride)..(i + stride + 1).min(n);
        let w: Vec<f64> = near.clone().map(|j| sin_at(k.pts[j])).collect();
        let Some(j) = (0..w.len()).min_by(|&x, &y| w[x].total_cmp(&w[y])) else {
            continue;
        };
        let side = |d: isize| {
            let q = (near.start + j) as isize + d;
            let q = if k.closed {
                q.rem_euclid(n as isize) as usize
            } else {
                q as usize
            };
            let x = q.checked_sub(near.start).and_then(|x| w.get(x).copied());
            (q < n).then(|| {
                (
                    x.unwrap_or_else(|| sin_at(k.pts[q])),
                    k.pts[q].distance(k.pts[near.start + j]),
                )
            })
        };
        // the angle falls off linearly into a touch: the steeper side's slope, carried past the
        // lowest point, meets the other side (a smooth dip that stays open does not)
        let x = w[j];
        if x > 1e-3 {
            let (Some(a), Some(b)) = (side(-1), side(1)) else {
                continue;
            };
            let ((vs, ds), (vb, db)) = if a.0 < b.0 { (a, b) } else { (b, a) };
            let slope = (vb - x) / db;
            if !(slope > 0.0 && x < slope * ds && (vs - (slope * ds - x)).abs() <= 0.25 * vs) {
                continue;
            }
        }
        let t = (near.start + j) as f64;
        let (mut lo, mut hi) = ((t - 1.0).max(range.lo), (t + 1.0).min(range.hi));
        for _ in 0..50 {
            let (m1, m2) = (
                hi - 0.618_033_988_749_895 * (hi - lo),
                lo + 0.618_033_988_749_895 * (hi - lo),
            );
            if sin(m1) < sin(m2) {
                hi = m2;
            } else {
                lo = m1;
            }
        }
        let t = 0.5 * (lo + hi);
        if sin(t) <= 1e-5 {
            out.push(t);
        }
    }
    out
}

/// A crossing hit as it is; a touching one replaced by the crossings found where the surface's
/// distance changes sign within a small window around it (none if it truly only touches).
pub(crate) fn crossings_near(curve: &Curve, range: Interval, s: &Surface, h: CurveHit) -> Vec<CurveHit> {
    if h.contact == Contact::Crossing {
        return vec![h];
    }
    let w = 1e-2 * range.width().min(1.0);
    let n = 64;
    let g = |t: f64| s.implicit(curve.eval(t));
    let at = |k: usize| (h.t - w + 2.0 * w * k as f64 / n as f64).clamp(range.lo, range.hi);
    // samples within a hair of the surface carry no sign: a true touch flickers around zero
    let dead = 1e-9;
    let mut out = Vec::new();
    let mut last: Option<(f64, f64)> = None;
    for k in 0..=n {
        let t1 = at(k);
        let g1 = g(t1);
        if g1.abs() <= dead {
            continue;
        }
        if let Some((t0, g0)) = last
            && (g0 < 0.0) != (g1 < 0.0)
        {
            let (mut lo, mut hi) = (t0, t1);
            for _ in 0..60 {
                let mid = 0.5 * (lo + hi);
                if (g(mid) < 0.0) == (g0 < 0.0) {
                    lo = mid
                } else {
                    hi = mid
                }
            }
            let t = 0.5 * (lo + hi);
            out.push(CurveHit {
                t,
                point: curve.eval(t),
                contact: Contact::Crossing,
            });
        }
        last = Some((t1, g1));
    }
    out
}

/// Two edges on the same carrier curve: each is cut where the other's ends lie inside it.
fn coincide(sb: &Body, src: Side, e: EdgeId, db: &Body, dst: Side, e2: EdgeId, cuts: &mut Vec<Cut>) {
    let mut ends_into = |from: &Body, fe: EdgeId, to: &Body, te: EdgeId, side: Side| {
        let (f, t) = (from.edge(fe), to.edge(te));
        for v in [f.start, f.end] {
            let vx = from.vertex(v);
            let s = t.curve.closest(vx.point, t.range);
            if t.curve.eval(s).distance(vx.point) <= vx.tol {
                cuts.push(Cut {
                    on: CutOn::Edge(side, te),
                    s,
                    point: vx.point,
                    tol: vx.tol,
                    touch: false,
                });
            }
        }
    };
    ends_into(sb, e, db, e2, dst);
    ends_into(db, e2, sb, e, src);
}

fn line_box(line: &forge_geom::Line, b: &forge_math::Box3) -> Option<Interval> {
    let (mut t0, mut t1) = (f64::NEG_INFINITY, f64::INFINITY);
    let d = line.dir.get();
    let o = line.origin;
    for (oi, di, iv) in [(o.x, d.x, b.x), (o.y, d.y, b.y), (o.z, d.z, b.z)] {
        if di.abs() < 1e-300 {
            if oi < iv.lo || oi > iv.hi {
                return None;
            }
            continue;
        }
        let (a, c) = ((iv.lo - oi) / di, (iv.hi - oi) / di);
        t0 = t0.max(a.min(c));
        t1 = t1.min(a.max(c));
    }
    (t0 < t1).then(|| Interval::new(t0, t1))
}

/// Vertices for the cuts (one per distinct point: cuts at one place on edges
/// of both bodies share it, and it is an existing vertex when any of them
/// sits at an edge's end), the new points, and aliases from B's existing
/// vertices to A's where the two coincide.
type Placed = (Vec<VKey>, Vec<Point3>, Vec<(VKey, VKey)>);

fn place_cuts(bodies: &[&Body; 2], cuts: &[Cut]) -> R<Placed> {
    let mut cluster: Vec<usize> = Vec::with_capacity(cuts.len());
    let mut heads: Vec<usize> = Vec::new();
    for (i, c) in cuts.iter().enumerate() {
        match heads
            .iter()
            .position(|&h| cuts[h].point.distance(c.point) <= cuts[h].tol.max(c.tol))
        {
            Some(k) => cluster.push(k),
            None => {
                cluster.push(heads.len());
                heads.push(i);
            }
        }
    }
    let end_vertex = |c: &Cut| {
        let (sd, e) = c.on.edge()?;
        let edge = bodies[sd].edge(e);
        [edge.start, edge.end]
            .into_iter()
            .find(|&v| {
                let vx = bodies[sd].vertex(v);
                vx.point.distance(c.point) <= vx.tol.max(c.tol)
            })
            .map(|v| VKey::Old(sd, v))
    };
    let mut points: Vec<Point3> = Vec::new();
    let mut alias: Vec<(VKey, VKey)> = Vec::new();
    let mut key_of: Vec<VKey> = Vec::with_capacity(heads.len());
    for k in 0..heads.len() {
        let mut olds: Vec<VKey> = (0..cuts.len())
            .filter(|&i| cluster[i] == k)
            .filter_map(|i| end_vertex(&cuts[i]))
            .collect();
        olds.sort();
        olds.dedup();
        let key = match olds.first() {
            Some(&v) => v,
            None => {
                points.push(cuts[heads[k]].point);
                VKey::New(points.len() as u32 - 1)
            }
        };
        for &o in olds.iter().skip(1) {
            match (key, o) {
                (VKey::Old(0, _), VKey::Old(1, _)) => alias.push((o, key)),
                _ => return Err(BoolError::Unsupported("two vertices of one body meet at a cut")),
            }
        }
        key_of.push(key);
    }
    Ok((cluster.iter().map(|&k| key_of[k]).collect(), points, alias))
}

/// The point nearest every curve through a new vertex (alternating projections from `p`), and
/// the tolerance it needs: the farthest curve, at least LINEAR.
pub(crate) fn settle(mut p: Point3, curves: &[(&Curve, Interval)]) -> R<(Point3, f64)> {
    if curves.is_empty() {
        return Ok((p, tol::LINEAR));
    }
    if let [(c, r)] = curves {
        return Ok((c.eval(c.closest(p, *r)), tol::LINEAR));
    }
    // the last round's projections bound how far the point is from each curve
    let mut near = [Point3::ORIGIN; 8];
    for _ in 0..8 {
        let mut sum = Vec3::ZERO;
        for (k, &(c, r)) in curves.iter().enumerate() {
            let q = c.eval(c.closest(p, r));
            if k < near.len() {
                near[k] = q;
            }
            sum += q - Point3::ORIGIN;
        }
        let was = p;
        p = Point3::ORIGIN + sum / curves.len() as f64;
        // a fixed point: every later round would give these same bits
        if p == was {
            break;
        }
    }
    let mut worst = tol::LINEAR;
    for (k, &(c, r)) in curves.iter().enumerate() {
        let d = match near.get(k) {
            Some(q) => q.distance(p),
            None => c.eval(c.closest(p, r)).distance(p),
        };
        worst = worst.max(d);
    }
    Ok((p, worst))
}

/// Direction of a half-edge in the face's oriented parameter plane, at its
/// start (`at_end` false) or pointing back along it from its end. Directions are ordered by
/// `predicates::angle_cmp`, exactly.
/// The parameter-plane chord from a piece's end at a vertex (as `uv_dir`) to a point the same
/// short length (0.01, or a tenth of the piece) along it: it tells apart curves leaving a vertex
/// along one tangent (an intersection curve touching a boundary edge there) by how each bends.
/// None at a singular point (a pole or an apex), where the parameters say nothing of direction.
fn uv_chord(body: &Body, f: FaceId, p: &Piece, forward: bool, at_end: bool) -> Option<Vec2> {
    let from_lo = forward != at_end;
    let t = if from_lo { p.range.lo } else { p.range.hi };
    let speed = p.curve.deriv(t).d1.length().max(f64::MIN_POSITIVE);
    let dt = (0.01 / speed).min(0.1 * p.range.width());
    let face = body.face(f);
    let a = face.surface.project(p.curve.eval(t));
    face.surface.normal(a)?;
    let b = forge_topo::uv::unwrap_near(
        body,
        f,
        face.surface
            .project(p.curve.eval(if from_lo { t + dt } else { t - dt })),
        a,
    );
    let d = b - a;
    Some(if face.same_sense { d } else { Vec2::new(-d.x, d.y) })
}

/// The way on from half-edge `i` at a vertex where ways may leave along one tangent (a curve
/// touching an edge, or a traced curve's tangent where the surfaces touch and its branches
/// cross, which is unreliable): where the choice rests on two such, again by chords a short
/// length along each, told apart by how each bends (at a pole or apex, where there is no
/// chord, by the tangent). Ordering by the first direction that differs is what a
/// second-order contact needs (the curves agree to first order); chords stand in for curvature,
/// which Ye and Maekawa's formulas give but which are numerically unreliable near a singular
/// point (Patrikalakis, Maekawa and Cho, ch. 5).
#[cold]
fn untie(
    body: &Body,
    f: FaceId,
    pieces: &[Piece],
    half: &[(usize, bool)],
    i: usize,
    out: &[(VKey, usize)],
) -> R<usize> {
    let dir = |k: usize| uv_dir(body, f, &pieces[half[k].0], half[k].1, false);
    let mut theta = uv_dir(body, f, &pieces[half[i].0], half[i].1, true)?;
    let mut ways: Vec<(Vec2, usize)> = out.iter().map(|&(_, k)| Ok((dir(k)?, k))).collect::<R<_>>()?;
    let (c, j) = clockwise(theta, ways.iter().map(|&w| Ok(w)))?.unwrap_or((theta, i));
    // (ways of one body's own edges along one tangent are its own smooth joins, as they always
    // were; only a way of a new edge, or of the other body's, makes a tie; nor is the way
    // straight back along the same piece)
    let near = |a: Vec2, b: Vec2| {
        let (d, x) = (a.dot(b), a.x * b.y - a.y * b.x);
        d > 0.0 && x * x < 0.002 * d * d
    };
    let mixed = |x: usize, y: usize| {
        let (p, q) = (&pieces[half[x].0], &pieces[half[y].0]);
        half[x].0 != half[y].0 && (p.side.is_none() || q.side.is_none() || p.side != q.side)
    };
    let tie = ways
        .iter()
        .any(|&(a, k)| (near(a, theta) && mixed(k, i)) || (k != j && near(a, c) && mixed(k, j)));
    if !tie {
        return Ok(j);
    }
    let chord = |h: (usize, bool), at_end: bool| {
        uv_chord(body, f, &pieces[h.0], h.1, at_end).and_then(|c| c.normalize())
    };
    if let Some(t) = chord(half[i], true) {
        theta = t;
    }
    for w in &mut ways {
        if let Some(c) = chord(half[w.1], false) {
            w.0 = c;
        }
    }
    Ok(clockwise(theta, ways.into_iter().map(Ok))?.map_or(i, |w| w.1))
}

/// Of the ways out of a vertex (uv direction, half-edge), the one just clockwise of the way
/// back `theta`.
fn clockwise(theta: Vec2, ways: impl Iterator<Item = R<(Vec2, usize)>>) -> R<Option<(Vec2, usize)>> {
    let after = |x: Vec2, y: Vec2| angle_cmp(x, y) == std::cmp::Ordering::Greater;
    let mut best: Option<(Vec2, usize)> = None;
    let mut top: Option<(Vec2, usize)> = None;
    for w in ways {
        let (a, j) = w?;
        if after(theta, a) && best.is_none_or(|b| after(a, b.0)) {
            best = Some((a, j));
        }
        if top.is_none_or(|b| after(a, b.0)) {
            top = Some((a, j));
        }
    }
    Ok(best.or(top))
}

pub(crate) fn uv_dir(body: &Body, f: FaceId, p: &Piece, forward: bool, at_end: bool) -> R<Vec2> {
    let t = if forward != at_end { p.range.lo } else { p.range.hi };
    let d = p.curve.deriv(t);
    let dir = if forward != at_end { d.d1 } else { -d.d1 };
    let face = body.face(f);
    let uv = face.surface.project(d.p);
    let sd = face.surface.deriv(uv);
    // least squares for (a, b) with a du + b dv = dir
    let (uu, uv2, vv) = (sd.du.dot(sd.du), sd.du.dot(sd.dv), sd.dv.dot(sd.dv));
    let (ru, rv) = (sd.du.dot(dir), sd.dv.dot(dir));
    let (a, b) = forge_math::linalg::solve2(uu, uv2, uv2, vv, ru, rv)
        .ok_or(BoolError::Unsupported("split through a singular point"))?;
    let a = if face.same_sense { a } else { -a };
    Ok(Vec2::new(a, b))
}

struct LocalSplit<'a> {
    body: &'a Body,
    f: FaceId,
    op: Op,
    /// B's index.
    index: &'a Index<'a>,
    by_edge: &'a [(Side, EdgeId, usize)],
    alias: &'a [(VKey, VKey)],
    /// The new edges in the face (pieces).
    new_edges: &'a [usize],
    pieces: &'a [Piece],
}

struct Local {
    /// Loops of the face the change touches (an edge cut, or a new edge ending on a vertex).
    touched: Vec<forge_topo::LoopId>,
    /// The class of the rest of the face.
    class: Class,
}

/// Whether a split of face `f` can stay local: its outer loop untouched and the face near it
/// kept as it is; then which loops the change touches.
fn local_split(c: LocalSplit) -> Option<Local> {
    let resolve = |v: VKey| c.alias.iter().find(|(x, _)| *x == v).map_or(v, |&(_, t)| t);
    let mut at_a: Vec<VertexId> = c
        .new_edges
        .iter()
        .flat_map(|&p| [c.pieces[p].start, c.pieces[p].end])
        .filter_map(|v| match resolve(v) {
            VKey::Old(0, v) => Some(v),
            _ => None,
        })
        .collect();
    at_a.sort_unstable();
    at_a.dedup();
    let cut = |e: EdgeId| {
        let i = c.by_edge.partition_point(|&(sd, ed, _)| (sd, ed) < (0, e));
        c.by_edge.get(i).is_some_and(|&(sd, ed, _)| sd == 0 && ed == e)
    };
    let body = c.body;
    let hit = |l: forge_topo::LoopId| {
        body.lp(l)
            .coedges
            .iter()
            .any(|&co| cut(body.coedge(co).edge) || at_a.binary_search(&body.coedge_start(co)).is_ok())
    };
    let loops = &body.face(c.f).loops;
    if hit(loops[0]) {
        return None;
    }
    let class = match c.index.classify(face_point(body, c.f)?) {
        Where::Inside => Class::In,
        Where::Outside => Class::Out,
        Where::On => return None,
    };
    let kept_as_is = match c.op {
        Op::Union | Op::Subtract => class == Class::Out,
        Op::Intersect => class == Class::In,
    };
    kept_as_is.then(|| Local {
        touched: loops[1..].iter().copied().filter(|&l| hit(l)).collect(),
        class,
    })
}

/// A point on face `f` a little inside its boundary, beside the middle of its first edge.
pub(crate) fn face_point(body: &Body, f: FaceId) -> Option<Point3> {
    let face = body.face(f);
    let c = *body.lp(*face.loops.first()?).coedges.first()?;
    let co = body.coedge(c);
    let edge = body.edge(co.edge);
    let d = edge.curve.deriv(edge.range.mid());
    let along = if co.forward { d.d1 } else { -d.d1 };
    let n = body.face_normal(f, face.surface.project(d.p))?;
    let inward = n.cross(along).normalize()?;
    let len = edge
        .curve
        .eval(edge.range.lo)
        .distance(d.p)
        .max(d.p.distance(edge.curve.eval(edge.range.hi)));
    let q = d.p + inward * (0.01 * len.max(1e-3)).min(0.05);
    Some(face.surface.eval(face.surface.project(q)))
}

/// The pieces chained into one closed loop (in walking order) and its outline in the face's
/// parameter plane; None if they don't close into exactly one loop, or a vertex of A is on it.
fn lone_cycle(body: &Body, f: FaceId, ps: &[usize], pieces: &[Piece]) -> Option<(Walk, Vec<Point2>)> {
    if ps
        .iter()
        .any(|&p| matches!(pieces[p].start, VKey::Old(0, _)) || matches!(pieces[p].end, VKey::Old(0, _)))
    {
        return None;
    }
    let mut cycle = vec![(ps[0], true)];
    let mut used = vec![false; ps.len()];
    used[0] = true;
    let start = pieces[ps[0]].start;
    let mut at = pieces[ps[0]].end;
    while at != start {
        let k =
            (0..ps.len()).find(|&k| !used[k] && (pieces[ps[k]].start == at || pieces[ps[k]].end == at))?;
        used[k] = true;
        let fwd = pieces[ps[k]].start == at;
        at = if fwd {
            pieces[ps[k]].end
        } else {
            pieces[ps[k]].start
        };
        cycle.push((ps[k], fwd));
    }
    if used.iter().any(|u| !u) {
        return None;
    }
    let mut samples = Vec::with_capacity(16 * cycle.len());
    for &(p, fwd) in &cycle {
        piece_samples(&pieces[p], fwd, &mut samples);
    }
    let (poly, _) = forge_topo::uv::draw_loop(body, f, &samples);
    Some((cycle, poly))
}

/// Whether a loop of face `f` might lie inside the outline (in `f`'s parameter plane): any
/// loop with a vertex in the outline's box, grown a little. Conservative on purpose: the
/// outline's chords cut inside the true curve, and a loop crossing it would have split edges.
fn encloses_a_loop(body: &Body, f: FaceId, poly: &[Point2]) -> bool {
    let (mut lo, mut hi) = (
        Point2::new(f64::INFINITY, f64::INFINITY),
        Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for q in poly {
        lo = Point2::new(lo.x.min(q.x), lo.y.min(q.y));
        hi = Point2::new(hi.x.max(q.x), hi.y.max(q.y));
    }
    let g = 1e-3 * (hi.x - lo.x + hi.y - lo.y);
    let face = body.face(f);
    face.loops.iter().any(|&l| {
        let v = body.edge(body.coedge(body.lp(l).coedges[0]).edge).start;
        let q = face.surface.project(body.vertex(v).point);
        q.x >= lo.x - g && q.x <= hi.x + g && q.y >= lo.y - g && q.y <= hi.y + g
    })
}

/// Points along a half-edge in walking order, without its last point.
pub(crate) fn piece_samples(p: &Piece, forward: bool, out: &mut Vec<Point3>) {
    let n = match p.curve {
        Curve::Line(_) => 2,
        _ => 16,
    };
    for i in 0..n {
        let k = if forward { i } else { n - i };
        out.push(
            p.curve
                .eval((p.range.lo + p.range.width() * (k as f64 / n as f64)).min(p.range.hi)),
        );
    }
}

/// Regions of face `f` cut by `new_edges` (each usable both ways) together
/// with its boundary (one way). Returns each region's loops, outer first.
/// What splitting a face gives: its regions (each region's loops, outer first), and when
/// the face's outer loop was left out of the split, the hole loops lying in no new region
/// (they stay with the face itself) and each region's outline (mirrored parameter plane).
pub(crate) struct Split {
    pub(crate) regions: Vec<Region>,
    pub(crate) open: Vec<Walk>,
    /// None when the face is planar and was split exactly: `region_holds` decides then, and the
    /// outlines are drawn only where it cannot (`plane_outlines`).
    pub(crate) outlines: Option<Vec<Option<Vec<Point2>>>>,
}

pub(crate) fn split_face(
    body: &Body,
    f: FaceId,
    pieces: &[Piece],
    boundary: &[Vec<(usize, bool)>],
    new_edges: &[usize],
    outer: bool,
    // the vertices where a cut met its edge without crossing it: two ways out may share a
    // tangent there
    ties: &[VKey],
) -> R<Split> {
    let mut half: Vec<(usize, bool)> = boundary.iter().flatten().copied().collect();
    for &p in new_edges {
        half.push((p, true));
        half.push((p, false));
    }
    let start_of = |h: (usize, bool)| if h.1 { pieces[h.0].start } else { pieces[h.0].end };
    let end_of = |h: (usize, bool)| if h.1 { pieces[h.0].end } else { pieces[h.0].start };
    // half-edges by start vertex, for finding the ways on from a vertex
    let mut by_start: Vec<(VKey, usize)> = (0..half.len()).map(|j| (start_of(half[j]), j)).collect();
    by_start.sort();
    let next = |i: usize| -> R<usize> {
        let v = end_of(half[i]);
        let from = by_start.partition_point(|&(k, _)| k < v);
        let out = &by_start[from..from + by_start[from..].partition_point(|&(k, _)| k == v)];
        // one way on (always so at a pole, where u is undefined): no angles needed
        if let [(_, j)] = out {
            return Ok(*j);
        }
        // each way by its tangent (where two may leave along one, by chords: see untie)
        if ties.binary_search(&v).is_ok() {
            return untie(body, f, pieces, &half, i, out);
        }
        let theta = uv_dir(body, f, &pieces[half[i].0], half[i].1, true)?;
        let ways = out
            .iter()
            .map(|&(_, k)| Ok((uv_dir(body, f, &pieces[half[k].0], half[k].1, false)?, k)));
        Ok(clockwise(theta, ways)?.map_or(i, |w| w.1))
    };
    let mut used = vec![false; half.len()];
    let mut cycles: Vec<Vec<usize>> = Vec::new();
    for s in 0..half.len() {
        if used[s] {
            continue;
        }
        let mut cyc = Vec::new();
        let mut i = s;
        while !used[i] {
            used[i] = true;
            cyc.push(i);
            i = next(i)?;
            if cyc.len() > half.len() {
                return Err(BoolError::Internal("face split did not close"));
            }
        }
        if i != s {
            if debug_on() {
                eprintln!(
                    "crossed itself: face {f:?} {:?} sense {}",
                    body.face(f).surface,
                    body.face(f).same_sense
                );
                for (k, &(p, fwd)) in half.iter().enumerate() {
                    let pc = &pieces[p];
                    let (a, b) = if fwd {
                        (pc.start, pc.end)
                    } else {
                        (pc.end, pc.start)
                    };
                    let mark = if cyc.contains(&k) { "*" } else { " " };
                    eprintln!(
                        "  {mark}half {k}: piece {p} {a:?} -> {b:?} seg {:?} mid {:?}",
                        pc.seg,
                        pc.curve.eval(pc.range.mid())
                    );
                }
            }
            return Err(BoolError::Internal("face split crossed itself"));
        }
        cycles.push(cyc);
    }
    let face = body.face(f);
    let nb = boundary.iter().map(|l| l.len()).sum::<usize>();
    let outer_len = if outer {
        boundary.first().map_or(0, |l| l.len())
    } else {
        0
    };
    // on a plane, a cycle made only of original boundary is an original loop, untouched:
    // its orientation is known (the first loop is the outer one) and it needs no outline
    let planar = matches!(face.surface, Surface::Plane(_));
    let untouched = |cyc: &Vec<usize>| planar && cyc.iter().all(|&i| i < nb);
    if let Surface::Plane(pl) = &face.surface {
        let walks: Vec<Walk> = cycles
            .iter()
            .map(|cyc| cyc.iter().map(|&i| half[i]).collect())
            .collect();
        let kind = |k: usize| match untouched(&cycles[k]) {
            true if cycles[k][0] < outer_len => Some(Cycle::Outer),
            true => Some(Cycle::Hole),
            false => None,
        };
        if let Some((regions, open)) = plane_split(pl, face.same_sense, &walks, kind, pieces, outer)? {
            return Ok(Split {
                regions,
                open,
                outlines: None,
            });
        }
    }
    let mirror = |p: Point2| {
        if face.same_sense {
            p
        } else {
            Point2::new(-p.x, p.y)
        }
    };
    let mut polys: Vec<Option<Vec<Point2>>> = cycles
        .iter()
        .map(|cyc| {
            if untouched(cyc) {
                return None;
            }
            let walk = cyc.iter().map(|&i| half[i]);
            let pts = match &face.surface {
                Surface::Cylinder(c) => cylinder_outline(c, walk, pieces),
                _ => None,
            };
            let pts = pts.unwrap_or_else(|| {
                let mut samples = Vec::new();
                for &i in cyc {
                    piece_samples(&pieces[half[i].0], half[i].1, &mut samples);
                }
                // pole-aware drawing
                forge_topo::uv::draw_loop(body, f, &samples).0
            });
            // mirrored so every face's loops run counter-clockwise
            Some(pts.into_iter().map(mirror).collect())
        })
        .collect();
    // each cycle was unwrapped on its own; shift them all into the window of the
    // widest one, so containment compares like with like on periodic surfaces
    let dom = face.surface.domain();
    if dom.u.period.is_some() || dom.v.period.is_some() {
        let span = |p: &Vec<Point2>| {
            p.iter().fold(
                (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY),
                |a, q| (a.0.min(q.x), a.1.max(q.x), a.2.min(q.y), a.3.max(q.y)),
            )
        };
        let reference = polys
            .iter()
            .flatten()
            .map(span)
            .max_by(|a, b| ((a.1 - a.0) * (a.3 - a.2)).total_cmp(&((b.1 - b.0) * (b.3 - b.2))));
        if let Some(r) = reference {
            let fit = |c: f64, lo: f64, hi: f64, period: Option<f64>| match period {
                Some(per) => [0.0, -per, per, -2.0 * per, 2.0 * per]
                    .into_iter()
                    .find(|k| c + k >= lo - 1e-9 && c + k <= hi + 1e-9)
                    .unwrap_or_else(|| per * ((0.5 * (lo + hi) - c) / per).round()),
                None => 0.0,
            };
            for p in polys.iter_mut().flatten() {
                let sp = span(p);
                let du = fit(0.5 * (sp.0 + sp.1), r.0, r.1, dom.u.period);
                let dv = fit(0.5 * (sp.2 + sp.3), r.2, r.3, dom.v.period);
                if du != 0.0 || dv != 0.0 {
                    for q in p.iter_mut() {
                        *q = Point2::new(q.x + du, q.y + dv);
                    }
                }
            }
        }
    }
    // each outline's box, a quick rejection before the full containment test
    let boxes: Vec<Option<(f64, f64, f64, f64)>> = polys
        .iter()
        .map(|p| {
            p.as_ref().map(|p| {
                p.iter().fold(
                    (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY),
                    |a, q| (a.0.min(q.x), a.1.max(q.x), a.2.min(q.y), a.3.max(q.y)),
                )
            })
        })
        .collect();
    let areas: Vec<f64> = cycles
        .iter()
        .zip(&polys)
        .map(|(cyc, poly)| match poly {
            Some(p) => forge_topo::uv::signed_area(p),
            None if cyc[0] < outer_len => f64::INFINITY,
            None => -1.0,
        })
        .collect();
    let mut regions: Vec<Region> = Vec::new();
    let mut open: Vec<Walk> = Vec::new();
    let mut outer_of: Vec<usize> = Vec::new();
    for (k, cyc) in cycles.iter().enumerate() {
        if areas[k] > 0.0 {
            outer_of.push(k);
            regions.push(vec![cyc.iter().map(|&i| half[i]).collect()]);
        }
    }
    let outlines = std::cell::OnceCell::<Vec<Outline>>::new();
    for (k, cyc) in cycles.iter().enumerate() {
        if areas[k] > 0.0 {
            continue;
        }
        let probe = match &polys[k] {
            // a point just off the hole's longest segment, on its material side (the left);
            // a short first segment put the probe a hair from the outline, on either side
            Some(poly) => {
                let k = (0..poly.len())
                    .max_by(|&a, &b| {
                        let len = |i: usize| (poly[(i + 1) % poly.len()] - poly[i]).length();
                        len(a).total_cmp(&len(b))
                    })
                    .unwrap_or(0);
                let (p0, p1) = (poly[k], poly[(k + 1) % poly.len()]);
                p0.lerp(p1, 0.5) + (p1 - p0).perp() * 1e-3
            }
            // an untouched hole touches no new edge, so it lies wholly on one side of every outline
            None => {
                let cands = cyc.iter().map(|&i| {
                    let pc = &pieces[half[i].0];
                    mirror(face.surface.project(pc.curve.eval(if half[i].1 {
                        pc.range.lo
                    } else {
                        pc.range.hi
                    })))
                });
                let outlines = outlines.get_or_init(|| {
                    outer_of
                        .iter()
                        .filter_map(|&o| Some((polys[o].as_deref()?, boxes[o]?)))
                        .collect()
                });
                clear_point(cands, outlines).ok_or(BoolError::Internal("empty cycle"))?
            }
        };
        // the smallest outline holding it; the untouched outer loop (no outline) holds every hole.
        // Outlines of a face spanning more than a period can't all share one window, so the
        // probe is tried a period either way too
        let shifts = |period: Option<f64>| match period {
            Some(p) => vec![0.0, -p, p],
            None => vec![0.0],
        };
        let (su, sv) = (shifts(dom.u.period), shifts(dom.v.period));
        let holds = |o: usize| {
            let (Some(poly), Some(b)) = (&polys[o], &boxes[o]) else {
                return true;
            };
            su.iter().any(|&du| {
                sv.iter().any(|&dv| {
                    let q = Point2::new(probe.x + du, probe.y + dv);
                    q.x >= b.0 && q.x <= b.1 && q.y >= b.2 && q.y <= b.3 && inside_poly(q, poly)
                })
            })
        };
        let host = outer_of
            .iter()
            .enumerate()
            .filter(|&(_, &o)| holds(o))
            .min_by(|x, y| areas[*x.1].total_cmp(&areas[*y.1]))
            .map(|(r, _)| r);
        let Some(host) = host else {
            if !outer {
                open.push(cyc.iter().map(|&i| half[i]).collect());
                continue;
            }
            if debug_on() {
                eprintln!(
                    "hole outside: face {f:?} {:?} sense {} probe {probe:?}",
                    face.surface, face.same_sense
                );
                for (q, cyc) in cycles.iter().enumerate() {
                    let pts = polys[q].as_ref().map(|p| {
                        p.iter()
                            .map(|v| format!("({:.3},{:.3})", v.x, v.y))
                            .collect::<Vec<_>>()
                            .join(" ")
                    });
                    eprintln!(
                        "  cycle {q} area {:.4} halves {:?}\n    {:?}",
                        areas[q],
                        cyc.iter().map(|&i| half[i]).collect::<Vec<_>>(),
                        pts
                    );
                }
            }
            return Err(BoolError::Internal("hole outside every region"));
        };
        regions[host].push(cyc.iter().map(|&i| half[i]).collect());
    }
    let outlines = outer_of.iter().map(|&o| polys[o].clone()).collect();
    Ok(Split {
        regions,
        open,
        outlines: Some(outlines),
    })
}

/// A walk on a cylinder made of rulings and circles about its axis, drawn exactly in (u, v): a
/// ruling keeps u, a circle keeps v and turns u by its own angle, so its corners are the outline.
/// None for any other piece.
fn cylinder_outline(
    c: &Cylinder,
    walk: impl Iterator<Item = (usize, bool)>,
    pieces: &[Piece],
) -> Option<Vec<Point2>> {
    let z = c.frame.z();
    let mut pts: Vec<Point2> = Vec::new();
    let mut at: Option<Point2> = None;
    for (p, fwd) in walk {
        let pc = &pieces[p];
        let a = match at {
            Some(a) => a,
            None => c.project(piece_end(pc, fwd)),
        };
        pts.push(a);
        let b = match &pc.curve {
            Curve::Line(l) if l.dir.cross(z).length() <= 1e-12 => {
                Point2::new(a.x, c.project(piece_end(pc, !fwd)).y)
            }
            Curve::Circle(k)
                if k.frame.z().cross(z).length() <= 1e-12 && (k.radius - c.radius).abs() <= tol::LINEAR =>
            {
                let turn = pc.range.width() * k.frame.z().dot(z).signum();
                Point2::new(a.x + if fwd { turn } else { -turn }, a.y)
            }
            _ => return None,
        };
        at = Some(b);
    }
    Some(pts)
}

/// A cycle of original boundary only, untouched: the face's outer loop or one of its holes.
#[derive(Clone, Copy, PartialEq)]
enum Cycle {
    Outer,
    Hole,
}

/// `split_face` on a plane, exactly: each cycle's signed area and where each hole lies from the
/// pieces' own lines and conics, no outline drawn. None when a piece is another curve or a test
/// is undecided (a ray through a vertex at every angle tried), for the drawn split to do.
#[allow(clippy::type_complexity)]
fn plane_split(
    pl: &Plane,
    same_sense: bool,
    walks: &[Walk],
    kind: impl Fn(usize) -> Option<Cycle>,
    pieces: &[Piece],
    outer: bool,
) -> R<Option<(Vec<Region>, Vec<Walk>)>> {
    let n = pl.frame.z().get() * if same_sense { 1.0 } else { -1.0 };
    let mut areas: Vec<f64> = Vec::with_capacity(walks.len());
    for (k, w) in walks.iter().enumerate() {
        areas.push(match kind(k) {
            Some(Cycle::Outer) => f64::INFINITY,
            Some(Cycle::Hole) => -1.0,
            None => match plane_area(n, w, pieces) {
                Some(a) => a,
                None => return Ok(None),
            },
        });
    }
    let outer_of: Vec<usize> = (0..walks.len()).filter(|&k| areas[k] > 0.0).collect();
    let mut regions: Vec<Region> = outer_of.iter().map(|&k| vec![walks[k].clone()]).collect();
    let boxes = std::cell::OnceCell::<Vec<(Box3, Vec<Box3>)>>::new();
    let mut open: Vec<Walk> = Vec::new();
    for (k, w) in walks.iter().enumerate() {
        if areas[k] > 0.0 {
            continue;
        }
        // a point of the hole: just off its longest piece on its material side (the left), or an
        // untouched hole's own vertex, as it touches no new edge
        let probe = match kind(k) {
            Some(_) => piece_end(&pieces[w[0].0], w[0].1),
            None => {
                let size = |h: &(usize, bool)| {
                    let pc = &pieces[h.0];
                    let m = pc.curve.eval(pc.range.mid());
                    m.distance(pc.curve.eval(pc.range.lo)) + m.distance(pc.curve.eval(pc.range.hi))
                };
                let h = w
                    .iter()
                    .copied()
                    .max_by(|a, b| size(a).total_cmp(&size(b)))
                    .ok_or(BoolError::Internal("empty cycle"))?;
                let pc = &pieces[h.0];
                let d = pc.curve.deriv(pc.range.mid());
                let along = if h.1 { d.d1 } else { -d.d1 };
                let Some(left) = n.cross(along).normalize() else {
                    return Ok(None);
                };
                d.p + left * (1e-3 * size(&h))
            }
        };
        // the smallest outer cycle holding it; the untouched outer loop holds every hole
        let mut host: Option<usize> = None;
        for (r, &o) in outer_of.iter().enumerate() {
            let holds = match kind(o) {
                Some(Cycle::Outer) => true,
                _ => {
                    let (hull, each) = &boxes
                        .get_or_init(|| outer_of.iter().map(|&o| walk_boxes(&walks[o], pieces)).collect())[r];
                    hull.contains(probe)
                        && match plane_inside(pl, probe, &walks[o], pieces, each) {
                            Some(b) => b,
                            None => return Ok(None),
                        }
                }
            };
            if holds && host.is_none_or(|h| areas[o] < areas[outer_of[h]]) {
                host = Some(r);
            }
        }
        match host {
            Some(r) => regions[r].push(w.clone()),
            None if !outer => open.push(w.clone()),
            None => return Err(BoolError::Internal("hole outside every region")),
        }
    }
    Ok(Some((regions, open)))
}

/// Twice the area a closed walk on a plane bounds about normal `n` (positive counter-clockwise),
/// exactly: half the integral of p x dp, in closed form for lines and conics. None for another curve.
fn plane_area(n: Vec3, walk: &[(usize, bool)], pieces: &[Piece]) -> Option<f64> {
    let r = piece_end(&pieces[walk.first()?.0], walk[0].1);
    let mut a = Vec3::new(0.0, 0.0, 0.0);
    for &(p, fwd) in walk {
        let pc = &pieces[p];
        let (t0, t1) = (pc.range.lo, pc.range.hi);
        let (p0, p1) = (pc.curve.eval(t0) - r, pc.curve.eval(t1) - r);
        let c = match &pc.curve {
            Curve::Line(_) => p0.cross(p1),
            // o + a cos t x + b sin t y: (o - r) x (p1 - p0) + a b (t1 - t0) x X y
            Curve::Circle(k) => {
                (k.frame.origin - r).cross(p1 - p0) + k.frame.z().get() * (k.radius * k.radius * (t1 - t0))
            }
            Curve::Ellipse(e) => {
                (e.frame.origin - r).cross(p1 - p0) + e.frame.z().get() * (e.major * e.minor * (t1 - t0))
            }
            _ => return None,
        };
        a = if fwd { a + c } else { a - c };
    }
    Some(a.dot(n))
}

/// Where a walk of pieces starts along half-edge (p, forward).
fn piece_end(p: &Piece, forward: bool) -> Point3 {
    p.curve.eval(if forward { p.range.lo } else { p.range.hi })
}

/// Boxes holding a walk of pieces: all of it, and each piece (grown by a tolerance).
fn walk_boxes(walk: &[(usize, bool)], pieces: &[Piece]) -> (Box3, Vec<Box3>) {
    let each: Vec<Box3> = walk
        .iter()
        .map(|&(p, _)| pieces[p].curve.bbox(pieces[p].range).grow(tol::LINEAR))
        .collect();
    (each.iter().fold(Box3::EMPTY, |a, b| a.hull(b)), each)
}

/// Whether point `q` of plane `pl` lies inside the closed walk, by the parity of a ray's crossings
/// in closed form, at up to three angles. None when every ray meets a vertex or runs along a piece.
fn plane_inside(
    pl: &Plane,
    q: Point3,
    walk: &[(usize, bool)],
    pieces: &[Piece],
    boxes: &[Box3],
) -> Option<bool> {
    let n = pl.frame.z().get();
    'ray: for angle in [0.1234, 1.9876, 4.0123] {
        let (sn, cs) = dmath::sin_cos(angle);
        let d = pl.frame.x() * cs + pl.frame.y() * sn;
        let mut crossings = 0;
        for (&(p, _), b) in walk.iter().zip(boxes) {
            if !crate::classify::ray_hits_box(q, d, b) {
                continue;
            }
            match crate::classify::plane_ray_crossings(n, q, d, &pieces[p].curve, pieces[p].range) {
                Some(k) => crossings += k,
                None => continue 'ray,
            }
        }
        return Some(crossings % 2 == 1);
    }
    None
}

/// Whether point `q` of planar face `f` lies inside a region's outer walk (of a split made by
/// `plane_split`); None when undecided.
fn region_holds(
    body: &Body,
    f: FaceId,
    region: &Region,
    pieces: &[Piece],
    q: Point3,
    boxes: &[Box3],
) -> Option<bool> {
    let Surface::Plane(pl) = &body.face(f).surface else {
        return None;
    };
    plane_inside(pl, q, region.first()?, pieces, boxes)
}

/// The outlines `split_face` draws for a planar face's regions (mirrored parameter plane).
fn plane_outlines(body: &Body, f: FaceId, regions: &[Region], pieces: &[Piece]) -> Vec<Option<Vec<Point2>>> {
    let face = body.face(f);
    regions
        .iter()
        .map(|r| {
            let mut samples = Vec::new();
            for &(p, fwd) in &r[0] {
                piece_samples(&pieces[p], fwd, &mut samples);
            }
            let (pts, _) = forge_topo::uv::draw_loop(body, f, &samples);
            Some(
                pts.into_iter()
                    .map(|p| {
                        if face.same_sense {
                            p
                        } else {
                            Point2::new(-p.x, p.y)
                        }
                    })
                    .collect(),
            )
        })
        .collect()
}

pub(crate) fn inside_poly(p: Point2, poly: &[Point2]) -> bool {
    forge_math::predicates::inside_polygon(p, poly)
}

/// A drawn outline and its box (u lo, u hi, v lo, v hi).
type Outline<'a> = (&'a [Point2], (f64, f64, f64, f64));

/// Of points of a loop that crosses none of `outlines` (so lies wholly on one side of each), the
/// first clear of every outline by a fiftieth of its size, else the one farthest from them: an
/// outline drawn with chords can pass on the wrong side of a point nearer its curve than that.
fn clear_point(cands: impl Iterator<Item = Point2>, outlines: &[Outline]) -> Option<Point2> {
    let clearance = |q: Point2| -> f64 {
        outlines
            .iter()
            .map(|&(o, b)| {
                let gap = 0.02 * Vec2::new(b.1 - b.0, b.3 - b.2).length();
                let off = Vec2::new(
                    (b.0 - q.x).max(q.x - b.1).max(0.0),
                    (b.2 - q.y).max(q.y - b.3).max(0.0),
                )
                .length();
                if off >= gap {
                    return f64::INFINITY;
                }
                let d = (0..o.len())
                    .map(|i| {
                        let (a, b) = (o[i], o[(i + 1) % o.len()]);
                        let ab = b - a;
                        let t = ((q - a).dot(ab) / ab.length_sq().max(f64::MIN_POSITIVE)).clamp(0.0, 1.0);
                        q.distance(a + ab * t)
                    })
                    .fold(f64::INFINITY, f64::min);
                d / gap.max(f64::MIN_POSITIVE)
            })
            .fold(f64::INFINITY, f64::min)
    };
    let mut best: Option<(f64, Point2)> = None;
    let mut cands = cands.peekable();
    while let Some(q) = cands.next() {
        // a lone point needs no measuring
        if best.is_none() && cands.peek().is_none() {
            return Some(q);
        }
        let c = clearance(q);
        if c >= 1.0 {
            return Some(q);
        }
        if best.is_none_or(|b| c > b.0) {
            best = Some((c, q));
        }
    }
    best.map(|b| b.1)
}

/// Inside, outside, or on the other body, for every spec.
fn classify_specs(
    bodies: &[&Body; 2],
    index: &[Index; 2],
    specs: &[Spec],
    pieces: &[Piece],
    segs: &[Seg],
    coplanar: &[(FaceId, FaceId)],
) -> R<Vec<Class>> {
    let n = specs.len();
    let mut class: Vec<Option<Class>> = specs.iter().map(|s| s.preset).collect();
    let mut fixed: Vec<bool> = specs.iter().map(|s| s.preset.is_some()).collect();
    // pieces lying on a coplanar face of the other body
    for (i, s) in specs.iter().enumerate() {
        let partners: Vec<FaceId> = coplanar
            .iter()
            .filter_map(|&(fa, fb)| match s.side {
                0 if fa == s.from => Some(fb),
                1 if fb == s.from => Some(fa),
                _ => None,
            })
            .collect();
        if partners.is_empty() {
            continue;
        }
        let body = bodies[s.side];
        let other = 1 - s.side;
        let Some(p) = interior_point(body, s, pieces) else {
            continue;
        };
        if specs_on() {
            let w: Vec<_> = partners
                .iter()
                .map(|&pf| (pf, index[other].point_in_face(pf, p)))
                .collect();
            eprintln!("  COPLANAR spec {i} face {:?} probe {p:?} partners {w:?}", s.from);
        }
        for pf in partners {
            if index[other].point_in_face(pf, p) == Where::Inside {
                let ob = bodies[other];
                let here = body.face_normal(s.from, body.face(s.from).surface.project(p));
                let there = ob.face_normal(pf, ob.face(pf).surface.project(p));
                let (Some(h), Some(t)) = (here, there) else {
                    continue;
                };
                class[i] = Some(if h.dot(t) > 0.0 {
                    Class::OnSame
                } else {
                    Class::OnOpposite
                });
                fixed[i] = true;
                break;
            }
        }
    }
    // direct: lean of the piece at a new edge
    for (i, s) in specs.iter().enumerate() {
        if fixed[i] {
            continue;
        }
        let body = bodies[s.side];
        let other = 1 - s.side;
        for &(p, fwd) in s.loops.iter().flatten() {
            let Some(si) = pieces[p].seg else { continue };
            let piece = &pieces[p];
            let d = piece.curve.deriv(piece.range.mid());
            let along = if fwd { d.d1 } else { -d.d1 };
            let face = body.face(s.from);
            let Some(n_here) = body.face_normal(s.from, face.surface.project(d.p)) else {
                continue;
            };
            let inward = n_here.cross(along);
            let of = segs[si].faces[other];
            let ob = bodies[other];
            let Some(n_other) = ob.face_normal(of, ob.face(of).surface.project(d.p)) else {
                continue;
            };
            let lean = inward.dot(n_other.get());
            if lean.abs() <= 1e-9 * inward.length() {
                continue;
            }
            class[i] = Some(if lean < 0.0 { Class::In } else { Class::Out });
            break;
        }
    }
    // flood across shared native edge pieces between pieces of one body
    // an edge imprinted into the other body's face lies on that body: no flood across it
    let mut users: Vec<Vec<usize>> = vec![Vec::new(); pieces.len()];
    let mut imprinted = vec![false; pieces.len()];
    for &p in specs.iter().flat_map(|s| &s.foreign) {
        imprinted[p] = true;
    }
    for (i, s) in specs.iter().enumerate() {
        for &(p, _) in s.loops.iter().flatten() {
            if pieces[p].seg.is_none() && !imprinted[p] {
                users[p].push(i);
            }
        }
    }
    let spread = |class: &mut Vec<Option<Class>>, start: usize| -> R<()> {
        let mut queue = vec![start];
        while let Some(i) = queue.pop() {
            let c = class[i].expect("queued specs are classified");
            for &(p, _) in specs[i].loops.iter().flatten() {
                for &j in &users[p] {
                    if specs[j].side != specs[i].side || fixed[j] {
                        continue;
                    }
                    match class[j] {
                        None => {
                            if specs_on() {
                                eprintln!(
                                    "  FLOOD spec {j} (face {:?}) <- spec {i} (face {:?} {:?}) ({c:?}) across piece {p}",
                                    specs[j].from,
                                    specs[i].from,
                                    bodies[specs[i].side].face(specs[i].from).surface
                                );
                            }
                            class[j] = Some(c);
                            queue.push(j);
                        }
                        Some(cj) if cj != c => {
                            if debug_on() {
                                debug_conflict(bodies, specs, pieces, class, i, j, p);
                            }
                            return Err(BoolError::Internal("inconsistent classification"));
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    };
    for i in 0..n {
        if class[i].is_some() && !fixed[i] {
            spread(&mut class, i)?;
        }
    }
    // regions no new edge reaches: one ray test each
    for i in 0..n {
        if class[i].is_some() {
            continue;
        }
        let s = &specs[i];
        let probe =
            interior_point(bodies[s.side], s, pieces).ok_or(BoolError::Internal("no interior point"))?;
        if specs_on() {
            eprintln!(
                "  RAY spec {i} probe {probe:?} -> {:?}",
                index[1 - s.side].classify(probe)
            );
        }
        class[i] = Some(match index[1 - s.side].classify(probe) {
            Where::Inside => Class::In,
            Where::Outside => Class::Out,
            Where::On => {
                if debug_on() {
                    eprintln!(
                        "untouched spec {i}: side {} face {:?} {:?} probe {probe:?} coplanar {coplanar:?}",
                        s.side,
                        s.from,
                        bodies[s.side].face(s.from).surface
                    );
                }
                return Err(BoolError::Unsupported("untouched face lies on the other body"));
            }
        });
        spread(&mut class, i)?;
    }
    Ok(class
        .into_iter()
        .map(|c| c.expect("every spec classified"))
        .collect())
}

/// Error-path diagnostics for a classification conflict (FORGE_DEBUG set).
fn debug_conflict(
    bodies: &[&Body; 2],
    specs: &[Spec],
    pieces: &[Piece],
    class: &[Option<Class>],
    i: usize,
    j: usize,
    p: usize,
) {
    let show = |k: usize| {
        let s = &specs[k];
        let f = bodies[s.side].face(s.from);
        eprintln!(
            "  spec {k}: side {} face {:?} {:?} sense {} class {:?} loops {:?}",
            s.side, s.from, f.surface, f.same_sense, class[k], s.loops
        );
    };
    eprintln!(
        "conflict across piece {p} ({:?} -> {:?}, seg {:?})",
        pieces[p].start, pieces[p].end, pieces[p].seg
    );
    show(i);
    show(j);
    for (k, pc) in pieces.iter().enumerate() {
        if specs[i]
            .loops
            .iter()
            .flatten()
            .chain(specs[j].loops.iter().flatten())
            .any(|h| h.0 == k)
        {
            eprintln!(
                "  piece {k}: {:?} -> {:?} seg {:?} at {:?}",
                pc.start,
                pc.end,
                pc.seg,
                pc.curve.eval(pc.range.mid())
            );
        }
    }
}

/// A point inside a spec's face: a small step inward from its first edge's midpoint.
fn interior_point(body: &Body, s: &Spec, pieces: &[Piece]) -> Option<Point3> {
    let h = *s.loops.first()?.first()?;
    // a step no other piece of the region comes nearer than: the way there crosses none of them
    // (a sliver narrower than the usual step would put the probe past its far side)
    let mut step = (0.01 * half_chord(&pieces[h.0])).min(0.05);
    while step >= 10.0 * tol::LINEAR {
        let q = probe_at(body, s.from, h, pieces, Some(step))?;
        let clear = s.loops.iter().flatten().all(|&(o, _)| {
            o == h.0 || {
                let pc = &pieces[o];
                // a circle's arc is no nearer than the whole circle
                matches!(&pc.curve, Curve::Circle(c) if (q.distance(c.frame.origin) - c.radius).abs() >= step)
                    || pc.curve.eval(pc.curve.closest(q, pc.range)).distance(q) >= step
            }
        });
        if clear {
            return Some(q);
        }
        step *= 0.25;
    }
    probe(body, s.from, h, pieces)
}

/// A point inside face `f` a small step off the middle of half-edge `(p, fwd)` of one of its
/// loops, on the face's side.
pub(crate) fn probe(body: &Body, f: FaceId, h: (usize, bool), pieces: &[Piece]) -> Option<Point3> {
    probe_at(body, f, h, pieces, None)
}

/// `probe`, a given distance off the half-edge (by default 1% of its half chord, at most 0.05).
pub(crate) fn probe_at(
    body: &Body,
    f: FaceId,
    (p, fwd): (usize, bool),
    pieces: &[Piece],
    step: Option<f64>,
) -> Option<Point3> {
    let face = body.face(f);
    let piece = &pieces[p];
    let t = piece.range.mid();
    let d = piece.curve.deriv(t);
    let along = if fwd { d.d1 } else { -d.d1 };
    let n = body.face_normal(f, face.surface.project(d.p))?;
    let inward = n.cross(along).normalize()?;
    let step = step.unwrap_or_else(|| (0.01 * half_chord(piece)).min(0.05));
    let q = d.p + inward * step;
    Some(face.surface.eval(face.surface.project(q)))
}

/// The longer chord from a piece's middle to an end (a closed piece, a whole circle, has no
/// chord end to end), at least 1e-3.
pub(crate) fn half_chord(piece: &Piece) -> f64 {
    let at = |t: f64| piece.curve.eval(t);
    let m = at(piece.range.mid());
    at(piece.range.lo)
        .distance(m)
        .max(m.distance(at(piece.range.hi)))
        .max(1e-3)
}

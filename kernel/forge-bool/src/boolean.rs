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
use forge_geom::intersect::{
    Contact, CurveHit, CurveSurface, SurfaceSurface, curve_surface, curve_surface_in,
};
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

impl VKey {
    /// The key as one number, in the same order (sorting by it is cheaper).
    pub(crate) fn packed(self) -> u64 {
        match self {
            VKey::Old(s, v) => (s as u64) << 32 | v.0 as u64,
            VKey::New(n) => 1 << 63 | n as u64,
        }
    }
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
    /// The face pair whose curve found it (`trim`), for naming a vertex nothing ends at.
    pub(crate) pair: Name,
}

/// How near a traced branch may stop to the rim edge it ends on (where it meets that edge's
/// face tangentially, a step along the rim changes its distance from both surfaces only
/// quadratically).
const RIM: f64 = 10.0 * tol::LINEAR;

/// Where point `p` lies on edge `e`: its parameter and the cut's tolerance, if it is within
/// LINEAR of the edge, or past an end but within that end vertex's tolerance.
pub(crate) fn on_edge(body: &Body, e: EdgeId, p: Point3) -> Option<(f64, f64)> {
    let edge = body.edge(e);
    let s = edge.curve.closest(p, edge.range);
    let d = edge.curve.eval(s).distance(p);
    // (an edge read from a file lies within its measured deviation of its faces)
    if d <= edge.tol {
        return Some((s, edge.tol));
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
    /// The faces' pair and branch, as the name has them, and that with what each end cuts:
    /// what new vertices are named from, whatever lies at the curve's other end.
    pub(crate) pair: Name,
    pub(crate) end_names: [Name; 2],
    /// What each end cuts, as the name has it (the n-ary renames spans of the curve by these).
    pub(crate) end_keys: [Name; 2],
}

/// A piece of an edge in the result: part of an original edge, or a new edge.
#[derive(Clone)]
pub(crate) struct Piece {
    pub(crate) curve: Curve,
    pub(crate) range: Interval,
    /// A piece of an edge read from a file keeps the edge's tolerance.
    pub(crate) tol: f64,
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
pub(crate) struct Spec {
    pub(crate) side: Side,
    pub(crate) from: FaceId,
    pub(crate) loops: Vec<Vec<(usize, bool)>>,
    /// NONE for a piece of a split face: named only if kept (`sole_pieces_keep_names`).
    pub(crate) name: Name,
    /// Pieces of the other body's edges imprinted on this face (a coplanar
    /// face's boundary); classification never floods across them.
    pub(crate) foreign: Vec<usize>,
    /// The face comes through whole, every edge intact: its cached data carries over.
    pub(crate) whole: bool,
    /// The face keeps its loops (but `remove`) and gains these: kept, it is extended in
    /// place, not rebuilt (a new loop inside it, or a local split leaving its outer loop).
    pub(crate) extend: bool,
    /// Loops of `from` an extending spec takes off the face (they were split, or moved into
    /// a new region).
    pub(crate) remove: Vec<forge_topo::LoopId>,
    /// A class known before classification (a face's untouched remainder, decided by one point).
    pub(crate) preset: Option<Class>,
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
pub(crate) enum Class {
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
    Ok(out.into_compact())
}

/// The boolean done in place on `target`: only faces near `b` are examined,
/// and only the faces it actually changes are rebuilt. Removed entities stay
/// as tombstones until `Body::compact`. Names do not depend on `_feature`: what the boolean makes
/// is named by what meets there, as the n-ary boolean names it.
pub fn boolean_into(target: &mut Body, b: &Body, op: Op, _feature: u64) -> R<()> {
    if imprinted(target, b, || std::borrow::Cow::Borrowed(b), op) {
        return Ok(());
    }
    general(target, b, op)
}

/// `boolean_into` with the tool as its entities (`forge_ops::extrude_unlinked`): the Body is
/// built only if the general boolean has to run.
pub fn boolean_unlinked(target: &mut Body, b: forge_topo::Unlinked, op: Op) -> R<()> {
    if imprinted(target, &b, || std::borrow::Cow::Owned(b.clone().into_body()), op) {
        return Ok(());
    }
    general(target, &b.into_body(), op)
}

/// Whether the local-imprint path did the boolean (the mode lets it run and it covers the tool);
/// `body`, the tool's Body, only for the check.
fn imprinted<'b, S: crate::imprint::Src>(
    target: &mut Body,
    src: &S,
    body: impl FnOnce() -> std::borrow::Cow<'b, Body>,
    op: Op,
) -> bool {
    match crate::imprint::mode() {
        crate::imprint::Mode::Off => false,
        crate::imprint::Mode::On => crate::imprint::column(target, src, op),
        crate::imprint::Mode::Check => {
            // the general boolean on a copy; the part goes on from this path's result, so the
            // check follows what `on` would make
            let mut slow = target.clone();
            let valid_before = forge_topo::check(target).is_empty();
            if !crate::imprint::column(target, src, op) {
                return false;
            }
            if let Err(e) = general(&mut slow, &body(), op) {
                panic!("imprint made a body where the general boolean failed: {e}");
            }
            if let Some(d) = crate::imprint::diff(target, &slow) {
                panic!("imprint differs from the general boolean: {d}");
            }
            let problems = forge_topo::check(target);
            if valid_before && !problems.is_empty() {
                panic!(
                    "imprint made an invalid body (the general's {}): {:?}",
                    if forge_topo::check(&slow).is_empty() {
                        "valid"
                    } else {
                        "invalid too"
                    },
                    &problems[..problems.len().min(3)]
                );
            }
            true
        }
    }
}

fn general(target: &mut Body, b: &Body, op: Op) -> R<()> {
    UNDECIDED.with(|u| u.set(None));
    let mut scratch = SCRATCH.with(|s| s.take());
    let plan = plan(target, b, op, &mut scratch)?;
    let mut clock = phases_on().then(std::time::Instant::now);
    let (loops, verts) = (target.loops.len(), target.vertices.len());
    let touched = touched_old(&plan);
    apply(target, b, &plan);
    split_new_pinches(target, loops, verts);
    split_old_pinches(target, &touched);
    target.joins.extend(&b.joins);
    target.joins.pieces.extend_from_slice(&plan.wholes);
    for &w in b.split.iter().chain(&plan.split) {
        target.note_split(w);
    }
    if !plan.survivors.is_empty() {
        target.sole_survivors(&plan.survivors);
    }
    lap(&mut clock, 6);
    scratch.keep(plan);
    SCRATCH.with(|s| s.replace(scratch));
    undecided_touch(target)
}

/// A touch nothing could judge that left a face on its wrong side (ulps off the sound case): refused
/// where it lies, as the closed-solid count sees no fault in it.
pub(crate) fn undecided_touch(body: &Body) -> R<()> {
    match UNDECIDED.with(|u| u.take()) {
        Some(at)
            if forge_topo::check(body)
                .iter()
                .any(|p| matches!(p, forge_topo::Problem::FaceWrongSide { .. })) =>
        {
            Err(BoolError::ZeroThickness { at })
        }
        _ => Ok(()),
    }
}

/// The target's vertices where something of this boolean meets them: a new curve's end, or a
/// vertex of the tool lying on one (two blocks touching at a corner). Only there can a boolean
/// pinch a vertex the target already had.
fn touched_old(plan: &Plan) -> Vec<VertexId> {
    let mut out: Vec<VertexId> = plan
        .alias
        .iter()
        .map(|a| a.1)
        .chain(
            plan.pieces
                .iter()
                .filter(|p| p.seg.is_some())
                .flat_map(|p| [p.start, p.end]),
        )
        .filter_map(|k| match k {
            VKey::Old(0, v) => Some(v),
            _ => None,
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// `split_new_pinches` at the target's own vertices `at`: the faces the boolean left alone may
/// make a fan of their own there, so every use leaving the vertex is gathered.
fn split_old_pinches(target: &mut Body, at: &[VertexId]) {
    if at.is_empty() {
        return;
    }
    // (each use of each loop is dear on a big part: only faces about those vertices)
    let reach = forge_math::Box3::from_points(at.iter().map(|&v| target.vertex(v).point)).grow(tol::LINEAR);
    let mut runs: Vec<(VertexId, forge_topo::CoedgeId)> = target
        .face_ids()
        .filter(|&f| target.face_box(f).overlaps(&reach))
        .flat_map(|f| target.face(f).loops.iter().copied())
        .flat_map(|l| target.lp(l).coedges())
        .map(|c| (target.coedge_start(c), c))
        .filter(|(v, _)| at.binary_search(v).is_ok())
        .collect();
    runs.sort_unstable();
    let mut prev = Vec::new();
    for run in runs.chunk_by(|x, y| x.0 == y.0).filter(|run| run.len() > 1) {
        let cs: Vec<_> = run.iter().map(|x| x.1).collect();
        target.split_pinched(run[0].0, &cs, &mut prev);
    }
}

/// Pieces left touching at a point keep a vertex each: looked for at the vertices a boolean
/// made (from `verts` on, where its curves meet), from its new loops (from `loops` on).
fn split_new_pinches(target: &mut Body, loops: usize, verts: usize) {
    if target.vertices.len() == verts {
        return;
    }
    let t: &Body = target;
    let mut from: Vec<(VertexId, forge_topo::CoedgeId)> = (loops..t.loops.len())
        .filter(|&l| {
            t.face(t.loops[l].face)
                .loops
                .contains(&forge_topo::LoopId(l as u32))
        })
        .flat_map(|l| t.loops[l].coedges().map(|c| (t.coedge_start(c), c)))
        .filter(|(v, _)| v.index() >= verts)
        .collect();
    from.sort_unstable();
    // round the vertex from the first (back along its loop, across to the partner): one fan
    // when that comes back having passed all the others
    let one_fan = |t: &Body, run: &[(VertexId, forge_topo::CoedgeId)]| {
        let c0 = run[0].1;
        let (mut c, mut seen) = (c0, 1);
        for _ in 0..t.coedges.len() {
            let l = t.lp(t.coedge(c).lp);
            let (first, n) = (l.coedge(0).0, l.coedges().len() as u32);
            let Some(next) = t.partner(forge_topo::CoedgeId(first + (c.0 - first + n - 1) % n)) else {
                return false;
            };
            if next == c0 {
                return seen == run.len();
            }
            seen += run[1..].iter().filter(|x| x.1 == next).count();
            c = next;
        }
        false
    };
    let mut prev = Vec::new();
    for run in from.chunk_by(|x, y| x.0 == y.0).filter(|run| run.len() > 1) {
        if one_fan(target, run) {
            continue;
        }
        let cs: Vec<_> = run.iter().map(|x| x.1).collect();
        target.split_pinched(run[0].0, &cs, &mut prev);
    }
}

/// Lists a boolean fills, kept per thread so the next boolean reuses their room.
#[derive(Default)]
struct Scratch {
    cuts: Vec<Cut>,
    segs: Vec<Seg>,
    pieces: Vec<Piece>,
    specs: Vec<Spec>,
    /// Each edge's pieces: (boolean, first piece, count) by side and edge index, current only
    /// where the boolean is this one's.
    edge_pieces: [Vec<(u32, u32, u32)>; 2],
    boolean: u32,
}

impl Scratch {
    fn keep(&mut self, plan: Plan) {
        let (mut pieces, mut specs) = (plan.pieces, plan.specs);
        pieces.clear();
        specs.clear();
        (self.pieces, self.specs) = (pieces, specs);
    }
}

thread_local! {
    static SCRATCH: std::cell::RefCell<Scratch> = std::cell::RefCell::default();
}

/// Everything a boolean decides, worked out without touching A.
pub(crate) struct Plan {
    pub(crate) pieces: Vec<Piece>,
    /// Edges split into several kept pieces, as wholes.
    pub(crate) split: Vec<Name>,
    /// Wholes of edges split before that this boolean trimmed to one piece: that piece may be
    /// all that is left of them (sorted).
    pub(crate) survivors: Vec<Name>,
    pub(crate) specs: Vec<Spec>,
    pub(crate) kept: Vec<(usize, bool)>,
    pub(crate) points: Vec<Point3>,
    /// Each new point's name.
    pub(crate) vnames: Vec<Name>,
    /// Existing vertices renamed (`flush_edges_as_cuts`).
    pub(crate) flush: Vec<(VKey, Name)>,
    /// Each new point's tolerance.
    pub(crate) tols: Vec<f64>,
    /// Existing vertices whose tolerance grows to hold the curves now ending on them.
    pub(crate) grown: Vec<(VKey, f64)>,
    pub(crate) alias: Vec<(VKey, VKey)>,
    /// Each piece of a face or edge split into several kept ones, and its whole's name.
    pub(crate) wholes: Vec<(Name, Name)>,
    /// A's faces that might change; every other face of A comes through as it is.
    pub(crate) involved: Vec<FaceId>,
    /// Involved faces of A nothing touched, found wholly on the kept side of B.
    pub(crate) untouched_kept: Vec<FaceId>,
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
            tol: edge.tol,
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

thread_local! {
    /// Springs a closed curve crossed an odd number of times are looked at again by their fences.
    static FENCES: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// A point touch in this boolean that nothing could judge.
    pub(crate) static UNDECIDED: std::cell::Cell<Option<Point3>> = const { std::cell::Cell::new(None) };
    /// A closed curve crossed springs an odd number of times in this plan.
    static ODD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `plan_once`; lost its way, once more with the slow search for shallow crossings of springs.
fn plan(a: &Body, b: &Body, op: Op, scratch: &mut Scratch) -> R<Plan> {
    ODD.with(|o| o.set(false));
    match plan_once(a, b, op, scratch) {
        Err(BoolError::Internal(w)) if ODD.with(|o| o.get()) => {
            FENCES.with(|f| f.set(true));
            let r = plan_once(a, b, op, scratch);
            FENCES.with(|f| f.set(false));
            r.map_err(|e| match e {
                BoolError::Internal(_) => BoolError::Internal(w),
                e => e,
            })
        }
        r => r,
    }
}

fn plan_once(a: &Body, b: &Body, op: Op, scratch: &mut Scratch) -> R<Plan> {
    let mut clock = phases_on().then(std::time::Instant::now);
    let bodies = [a, b];
    let index = [Index::new(a), Index::new(b)];
    // new edges and vertices are named by the faces and edges meeting there, never by the
    // feature: the n-ary boolean, meeting them all at once, names them alike
    let root = Name::feature(0);

    // 1-2: intersect overlapping face pairs, keep pieces inside both faces
    let (mut cuts, mut segs) = (
        std::mem::take(&mut scratch.cuts),
        std::mem::take(&mut scratch.segs),
    );
    cuts.clear();
    segs.clear();
    let mut coplanar: Vec<(FaceId, FaceId)> = Vec::new();
    let mut imprints: Vec<Imprint> = Vec::new();
    let mut touching: Vec<Touch> = Vec::new();
    let b_box = index[1].bbox;
    let mut near_b: Vec<FaceId> = a.faces_where(|x| x.overlaps(&b_box)).collect();
    // what a boolean makes must not follow the order in the arenas, to the bit
    near_b.sort_unstable_by_key(|&f| a.face(f).name);
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
            if rolls_off(a, fa, sb) || rolls_off(b, fb, sa) {
                continue;
            }
            let region = overlap(&ba, &b_boxes[fb.index()]).grow(tol::LINEAR);
            let found = ssi::surfaces_in(sa, sb, meet_region(&ba, &b_boxes[fb.index()], sa, sb));
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
            for (curve, contact) in curves {
                // surfaces that only touch along a curve cross nowhere: no face splits there
                if contact == Contact::Touching {
                    continue;
                }
                let pair_name = root
                    .with_str("cut")
                    .with_set(&[a.face(fa).name.whole(), b.face(fb).name.whole()]);
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
    rank_pairs(&mut segs, &bodies);

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
            // cut at the point by a third surface, so round it nothing says: mostly a sound vertex
            // (two balls on a top meet in a cycle of four faces), checked once built
            if t.dir.is_none() {
                UNDECIDED.with(|u| u.set(Some(t.at)));
            }
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
        if kept_a
            && kept_b
            && !flows_on([a, b], t.faces, t.at, op == Op::Subtract)
            && !crack_tip([a, b], [&index[0], &index[1]], t, op == Op::Union)
        {
            return Err(BoolError::ZeroThickness { at: t.at });
        }
    }

    lap(&mut clock, 0);
    // faces on one surface: each imprints the other's boundary
    let mut along = Vec::new();
    for &(fa, fb) in &coplanar {
        let pair = [&index[0], &index[1]];
        imprint(
            bodies,
            pair,
            [0, 1],
            1,
            fb,
            fa,
            &mut cuts,
            &mut imprints,
            &mut along,
        )?;
        imprint(
            bodies,
            pair,
            [0, 1],
            0,
            fa,
            fb,
            &mut cuts,
            &mut imprints,
            &mut along,
        )?;
    }
    share_cuts(&bodies, &mut along, &mut cuts);

    // 3: vertices for the cuts, then edge pieces
    lap(&mut clock, 1);
    let (cut_vertex, mut new_points, alias) = place_cuts(&bodies, &cuts)?;
    let mut marks = curve_ends(&segs, &cut_vertex);
    let mut ended = vec![false; new_points.len()];
    for m in &marks {
        ended[m.0 as usize] = true;
    }
    cut_marks(
        &cuts,
        &cut_vertex,
        &ended,
        &|sd, e| bodies[sd].edge(e).name,
        &mut marks,
    );
    let mut vnames = vertex_names(&new_points, marks, root, || part_frame(a), None);
    // a piece of a cut edge keeps the whole edge's name, so an earlier boolean that cut it may
    // have named the vertex it made from the same marks: then a name here passes over the ends
    // of the edges cut (rare: looked for only at the new vertices' own edges)
    let end_names = |sd: Side, e: EdgeId| {
        let x = bodies[sd].edge(e);
        [bodies[sd].vertex(x.start).name, bodies[sd].vertex(x.end).name]
    };
    let clash = cuts
        .iter()
        .zip(&cut_vertex)
        .any(|(c, &k)| match (k, c.on.edge()) {
            (VKey::New(i), Some((sd, e))) => end_names(sd, e).contains(&vnames[i as usize]),
            _ => false,
        });
    if clash {
        let mut old: Vec<Name> = cuts
            .iter()
            .filter_map(|c| c.on.edge())
            .flat_map(|(sd, e)| end_names(sd, e))
            .collect();
        old.sort_unstable();
        for n in &mut vnames {
            while old.binary_search(n).is_ok() {
                *n = n.with(1);
            }
        }
    }
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
    let mut pieces = std::mem::take(&mut scratch.pieces);
    let mut by_edge: Vec<(Side, EdgeId, usize)> = cuts
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.on.edge().map(|(sd, e)| (sd, e, i)))
        .collect();
    by_edge.sort_unstable();
    // pieces per edge, made only for edges a spec reaches
    // looked up, never iterated, so the map's order cannot reach the result
    // each edge's pieces, split once: (first piece, count) by edge index, count 0 if not yet
    scratch.boolean = scratch.boolean.wrapping_add(1);
    if scratch.boolean == 0 {
        scratch.edge_pieces = Default::default();
        scratch.boolean = 1;
    }
    let now = scratch.boolean;
    let edge_pieces = &mut scratch.edge_pieces;
    edge_pieces[0].resize(edge_pieces[0].len().max(a.edges.len()), (0, 0, 0));
    edge_pieces[1].resize(edge_pieces[1].len().max(b.edges.len()), (0, 0, 0));
    // the edges split into more than one piece, and their pieces
    let mut split_runs: Vec<(Side, EdgeId, std::ops::Range<usize>)> = Vec::new();
    let mut ensure = |side: Side, e: EdgeId, pieces: &mut Vec<Piece>| -> std::ops::Range<usize> {
        let (at, first, n) = edge_pieces[side][e.index()];
        if at == now {
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
        edge_pieces[side][e.index()] = (now, r.start as u32, r.len() as u32);
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
            tol: tol::LINEAR,
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
    let mut specs = std::mem::take(&mut scratch.specs);
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
                    index: [&index[0], &index[1]],
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
                    for c in body.lp(l).coedges() {
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
                let split = split_face(body, f, &pieces, &boundary, &new_edges, false, &|_| None)?;
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
                    let cands = body.lp(l).coedges().map(|c| {
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
                        let q = body.vertex(body.coedge_start(body.lp(l).coedge(0))).point;
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
                            if let Some(r) = innermost(&held, boxes) {
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
                let mut walk = Vec::with_capacity(body.lp(l).coedges().len());
                for c in body.lp(l).coedges() {
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
            // an imprint a hair from the boundary, between the same two vertices, is the boundary
            // (a tool's side all but tangent to a round face's edge)
            // (a chord that near a curve is short)
            if foreign.iter().any(|&p| half_chord(&pieces[p]) <= 0.5) {
                let key = |p: &Piece| (p.start.min(p.end), p.start.max(p.end));
                let mut ends: Vec<_> = boundary
                    .iter()
                    .flatten()
                    .filter(|h| !matches!(pieces[h.0].curve, Curve::Line(_)))
                    .map(|h| (key(&pieces[h.0]), h.0))
                    .collect();
                ends.sort_unstable();
                if !ends.is_empty() {
                    foreign.retain(|&p| {
                        let pc = &pieces[p];
                        let k = key(pc);
                        let from = ends.partition_point(|x| x.0 < k);
                        !ends[from..].iter().take_while(|x| x.0 == k).any(|&(_, b)| {
                            let o = &pieces[b];
                            let mid = pc.curve.eval(pc.range.mid());
                            b != p && o.curve.eval(o.curve.closest(mid, o.range)).distance(mid) <= tol::LINEAR
                        })
                    });
                }
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
            // a cycle a new edge shows to be dropped is not drawn (`split_face`)
            let gone = |h: (usize, bool)| {
                let (_, l) = lean(&bodies, &segs, &pieces, side, f, h)?;
                let class = if l < 0.0 { Class::In } else { Class::Out };
                (l.abs() > 1e-3).then(|| keep_of(op, side, class).is_none())
            };
            let regions = split_face(body, f, &pieces, &boundary, &new_edges, true, &gone)?.regions;
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
    let mut shared: Vec<usize> = Vec::new();
    for (p, pc) in pieces.iter().enumerate().filter(|(_, pc)| pc.side == Some(1)) {
        let key = (pc.start.min(pc.end), pc.start.max(pc.end));
        let from = by_ends.partition_point(|x| x.0 < key);
        let mid = pc.curve.eval(pc.range.mid());
        for &(_, a) in by_ends[from..].iter().take_while(|x| x.0 == key) {
            let other = &pieces[a];
            let t = other.curve.closest(mid, other.range);
            if other.curve.eval(t).distance(mid) <= tol::LINEAR {
                // a closed edge's ends say nothing of which way it runs: its tangent does
                let flip = if pc.start == pc.end {
                    pc.curve.deriv(pc.range.mid()).d1.dot(other.curve.deriv(t).d1) < 0.0
                } else {
                    pc.start != other.start
                };
                canon[p] = (a, flip);
                shared.push(a);
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
    let class = classify_specs(&bodies, &index, &specs, &pieces, &segs, &coplanar, &shared)?;
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
    let kept: Vec<(usize, bool)> = (0..specs.len())
        .filter_map(|i| keep_of(op, specs[i].side, class[i]).map(|flip| (i, flip)))
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
    let (split, wholes) = sole_pieces_keep_names(
        &bodies,
        &mut specs,
        &kept,
        &mut pieces,
        &uses,
        &split_runs,
        &vname,
    );
    let flush = if coplanar.is_empty() {
        Vec::new()
    } else {
        let kept_faces = kept
            .iter()
            .map(|&(i, _)| (specs[i].side, specs[i].from))
            .collect();
        let pairs: Vec<[(Side, FaceId); 2]> = coplanar.iter().map(|&(fa, fb)| [(0, fa), (1, fb)]).collect();
        let whole = |sd: Side, e: EdgeId| match edge_pieces[sd][e.index()] {
            (at, first, 1) if at == now => Some(first as usize),
            _ => None,
        };
        flush_edges_as_cuts(&bodies, kept_faces, &mut pieces, &uses, &pairs, &whole, root)
    };
    // an edge of A split before and trimmed here to one piece may be its whole's last piece
    let mut survivors: Vec<Name> = Vec::new();
    if !a.split.is_empty() {
        for (side, e, run) in &split_runs {
            if *side == 0
                && run.clone().filter(|&p| uses[p] > 0).count() == 1
                && let Some(w) = a.split_whole(a.edge(*e).name)
            {
                survivors.push(w);
            }
        }
        survivors.sort_unstable();
        survivors.dedup();
    }
    (scratch.cuts, scratch.segs) = (cuts, segs);
    Ok(Plan {
        pieces,
        split,
        survivors,
        specs,
        kept,
        points,
        vnames,
        flush,
        tols,
        grown,
        alias,
        wholes,
        involved,
        untouched_kept,
    })
}

/// Makes the planned changes to A: drops the faces that go, rebuilds the
/// faces that change, adds the faces that come from B, and regroups shells.
pub(crate) fn apply<S: crate::imprint::Src>(a: &mut Body, b: &S, plan: &Plan) {
    let Plan {
        pieces,
        split: _,
        survivors: _,
        specs,
        kept,
        points,
        vnames,
        flush,
        tols,
        grown,
        alias,
        involved,
        untouched_kept,
        ..
    } = plan;
    // sorted by vertex (`flush_edges_as_cuts`)
    let renamed = |v: VKey, n: Name| flush.binary_search_by_key(&v, |f| f.0).map_or(n, |k| flush[k].1);
    for &(v, n) in flush {
        if let VKey::Old(0, id) = v {
            a.vertices[id.index()].name = n;
        }
    }
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
    // a face split locally gives up the loops it replaces before it gains the new ones; where
    // its outer loop goes, the spec's first loop (its new outer one) takes its place
    let mut new_outer: Vec<usize> = Vec::new();
    for &(si, flip) in kept {
        let s = &specs[si];
        if s.extend && !flip && !s.remove.is_empty() {
            if s.remove.contains(&a.face(s.from).loops[0]) {
                new_outer.push(si);
            }
            crate::classify::drop_query_loops(a, s.from, &s.remove);
            for &l in &s.remove {
                a.remove_loop(s.from, l);
            }
        }
    }
    let mut b_vertex: Vec<Option<VertexId>> = vec![None; b.vertex_count()];
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
                let (point, tol, name) = b.vertex(id);
                a.add_tolerant_vertex(point, grown_tol(grown, v, tol), renamed(v, name))
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
        let (sense, bx) = if s.side == 0 {
            (a.face(s.from).same_sense, a.face(s.from).bbox.get().copied())
        } else {
            (b.face(s.from).1, b.bbox(s.from))
        };
        // an extended (planar) face only gains holes inside it: its box stays
        let face = if s.extend && !flip {
            s.from
        } else {
            let surface = if s.side == 0 {
                a.face(s.from).surface.clone()
            } else {
                b.face(s.from).0.clone()
            };
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
                                let e = a.add_edge(pc.curve.clone(), pc.range, v0, v1, pc.name);
                                a.edges[e.index()].tol = pc.tol;
                                e
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
        if !flip && new_outer.contains(&si) {
            let loops = &mut a.faces[face.index()].loops;
            let at = loops.len() - s.loops.len();
            loops[..=at].rotate_right(1);
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

/// Whether two faces kept where they touch face the same way there (`flip`: one of them turned
/// over in the result): each ends where the other starts and one skin runs on tangent across the
/// touch, with material all on its one side. Only faces facing apart have material meeting itself.
pub(crate) fn flows_on(bodies: [&Body; 2], faces: [FaceId; 2], at: Point3, flip: bool) -> bool {
    let normal = |k: usize| {
        let s = &bodies[k].face(faces[k]).surface;
        bodies[k].face_normal(faces[k], s.project(at))
    };
    match (normal(0), normal(1)) {
        (Some(x), Some(y)) => (x.dot(y.get()) > 0.0) != flip,
        _ => false,
    }
}

/// Whether solids touching here, facing apart, have material on one side of the touch: faces flush
/// (or one solid in the other) up to the line, parting as a groove of no width at its root beyond it.
/// One skin turns back along the line, no material meets itself. Only with a gap on every side do they
/// meet there and nowhere near. `any`: a union, where either solid's inside is material.
#[cold]
pub(crate) fn crack_tip(bodies: [&Body; 2], index: [&Index; 2], t: &Touch, any: bool) -> bool {
    let s = &bodies[0].face(t.faces[0]).surface;
    let Some(n) = bodies[0].face_normal(t.faces[0], s.project(t.at)) else {
        return false;
    };
    let dirs: Vec<Vec3> = match t.dir {
        Some(d) => match n.cross(d).normalize() {
            Some(w) => vec![w.get(), -w.get()],
            None => return false,
        },
        None => {
            let fr = Frame::from_z(t.at, n);
            vec![fr.x().get(), -fr.x().get(), fr.y().get(), -fr.y().get()]
        }
    };
    // far enough that a gap opening with the square of the distance shows past tolerance
    [0.05, 0.2].iter().all(|&d| {
        dirs.iter().any(|&w| {
            let q = t.at + w * d;
            let (x, y) = (index[0].classify(q), index[1].classify(q));
            let inside = if any {
                x == Where::Inside || y == Where::Inside
            } else {
                x == Where::Inside && y == Where::Inside
            };
            inside || (x == Where::On && y == Where::On)
        })
    })
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

/// Whether `curve` passes the middle of edge `e`, not just an end.
#[cold]
fn runs_along(body: &Body, e: EdgeId, curve: &Curve, range: Interval) -> bool {
    let edge = body.edge(e);
    let m = edge.curve.eval(edge.range.mid());
    curve.eval(curve.closest(m, range)).distance(m) <= tol::LINEAR.max(body.vertex(edge.start).tol)
}

/// Where a tolerant edge passes through the other face, as (on the curve, on the edge, point).
/// Out of line: no edge outside an imported body comes here, and `trim` is hot.
#[cold]
#[inline(never)]
fn tolerant_cuts(
    edge: &forge_topo::Edge,
    curve: &Curve,
    range: Interval,
    (body, index, face): (&Body, &Index, FaceId),
) -> Vec<(f64, f64, Point3)> {
    let mut out = Vec::new();
    let band = edge.tol;
    let CurveSurface::Hits(hits) = curve_surface(&edge.curve, edge.range, &body.face(face).surface) else {
        return out;
    };
    for h in hits {
        let t = curve.closest(h.point, range);
        if curve.eval(t).distance(h.point) <= band
            && index.face_box(face).grow(band).contains(h.point)
            && index.point_in_face(face, h.point) != Where::Outside
        {
            out.push((t, h.t, h.point));
        }
    }
    out
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
    // springs crossed by the spring against the other surface
    let mut fenced: Vec<(usize, EdgeId)> = Vec::new();
    // a traced branch's ends, where it may stop on an edge
    let ends = (!closed && domain.lo.is_finite() && domain.hi.is_finite())
        .then(|| [range.lo, range.hi].map(|t| (t, curve.eval(t))));
    // an edge whose box misses the curve's cannot cut it (big faces have hundreds of edges)
    let reach = curve.bbox(range).grow(tol::LINEAR);
    for k in 0..2 {
        let (body, f, side) = (bodies[k], faces[k], sides[k]);
        for &l in &body.face(f).loops {
            for c in body.lp(l).coedges() {
                let e = body.coedge(c).edge;
                let edge = body.edge(e);
                if !body.edge_box(e).overlaps(&reach) {
                    continue;
                }
                if cuts[first..].iter().any(|x| x.on == CutOn::Edge(side, e)) {
                    continue; // a seam is visited twice
                }
                // on a plane the curve meets the edge where it meets the edge's own line or circle
                if let Surface::Plane(pl) = &body.face(f).surface
                    && let Some(hits) = plane_meet(pl, &curve, range, &edge.curve)
                {
                    for h in hits {
                        if let Some((s, tol)) = on_edge(body, e, h.point) {
                            let other = index[1 - k].face_box(faces[1 - k]);
                            if other.grow(tol::LINEAR).contains(h.point)
                                && (other.contains(h.point)
                                    || index[1 - k].point_in_face(faces[1 - k], h.point) != Where::Outside)
                            {
                                ts.push((h.t, cuts.len()));
                                cuts.push(Cut {
                                    pair: Name::NONE,
                                    on: CutOn::Edge(side, e),
                                    s,
                                    point: h.point,
                                    tol,
                                    touch: branches[1],
                                });
                            }
                        }
                    }
                    continue;
                }
                // an edge read from a file may lie off its faces, and two faces touching along it
                // need not meet at all: where it passes through the other surface, the curve ends
                // there too, for both of its faces alike
                if edge.tol > tol::LINEAR {
                    let other = (bodies[1 - k], index[1 - k], faces[1 - k]);
                    for (t, s, point) in tolerant_cuts(edge, &curve, range, other) {
                        ts.push((t, cuts.len()));
                        cuts.push(Cut {
                            pair: Name::NONE,
                            on: CutOn::Edge(side, e),
                            s,
                            point,
                            tol: edge.tol,
                            touch: branches[1],
                        });
                    }
                    continue;
                }
                let across = across(body, f, c)?;
                // only hits on the edge count: within its ends' tolerance of its box
                let near = || {
                    let slack = body
                        .vertex(edge.start)
                        .tol
                        .max(body.vertex(edge.end).tol)
                        .max(edge.tol);
                    Some(body.edge_box(e).grow(2.0 * slack))
                };
                // a fence stands only near its spring: where the other face's surface is analytic,
                // the spring itself against that surface finds the same points, and shallow ones
                let other_surface = &bodies[1 - k].face(faces[1 - k]).surface;
                let on_spring = matches!(&edge.curve, Curve::Procedural(pc)
                        if pc.0.as_any().is_some_and(|a| a.is::<forge_geom::blend::Spring>()))
                    && !matches!(other_surface, Surface::Procedural(_));
                let found = if on_spring {
                    fenced.push((k, e));
                    // (a crossing lies on the other face too: in its box)
                    let other_box = index[1 - k].face_box(faces[1 - k]).grow(tol::LINEAR);
                    if !other_box.overlaps(&reach) {
                        continue;
                    }
                    let within = overlap(&other_box, &reach);
                    let spring = match &edge.curve {
                        Curve::Procedural(pc) => {
                            pc.0.as_any()
                                .and_then(|a| a.downcast_ref::<forge_geom::blend::Spring>())
                        }
                        _ => None,
                    };
                    let quick = spring.and_then(|sp| sp.crossing_points(edge.range, other_surface, &within));
                    let on_curve = |t_on: f64, point: Point3| {
                        let t = curve.closest(point, range);
                        let _ = t_on;
                        (curve.eval(t).distance(point) <= tol::LINEAR).then_some(CurveHit {
                            t,
                            point,
                            contact: Contact::Crossing,
                        })
                    };
                    match quick {
                        Some(ps) => {
                            CurveSurface::Hits(ps.into_iter().filter_map(|(t, p)| on_curve(t, p)).collect())
                        }
                        None => {
                            match curve_surface_in(&edge.curve, edge.range, other_surface, || Some(within)) {
                                CurveSurface::Hits(more) => CurveSurface::Hits(
                                    more.into_iter().filter_map(|h| on_curve(h.t, h.point)).collect(),
                                ),
                                o => o,
                            }
                        }
                    }
                } else {
                    curve_surface_in(&curve, range, &across, near)
                };
                // a line and a straight edge with both its ends on it run together, however their
                // surfaces round off: a hair of tilt puts the crossing anywhere along them
                let hits = match found {
                    CurveSurface::Hits(h)
                        if !matches!((&curve, &edge.curve), (Curve::Line(a), Curve::Line(b))
                                if a.dir.get().cross(b.dir.get()).length() <= 1e-6)
                            || ![edge.start, edge.end].into_iter().all(|v| {
                                let vx = body.vertex(v);
                                curve.eval(curve.closest(vx.point, range)).distance(vx.point) <= vx.tol
                            }) =>
                    {
                        h
                    }
                    // the curve runs along this edge: cut it at the edge's ends; the stretch along
                    // the edge is boundary, not a new edge (the faces either side of it meet the
                    // other body there, and their own pairs decide what happens)
                    _ => {
                        let g = body.partner(c).map(|o| body.coedge_face(o));
                        let seam = g == Some(f);
                        // beside a face on the other face's surface, coplanar imprinting does it
                        let other = &bodies[1 - k].face(faces[1 - k]).surface;
                        let coplanar =
                            g.is_some_and(|g| g != f && ssi::coincident(&body.face(g).surface, other));
                        // (on the surface across a seam the curve may only pass an end: a cone's
                        // seam and another of its lines in one plane through its axis)
                        let runs = !seam || runs_along(body, e, &curve, range);
                        if runs && !coplanar && !along.iter().any(|a| a.0 == k && a.1 == e) {
                            along.push((k, e, seam));
                        }
                        for (s, v) in [(edge.range.lo, edge.start), (edge.range.hi, edge.end)] {
                            let vx = body.vertex(v);
                            let t = curve.closest(vx.point, range);
                            if curve.eval(t).distance(vx.point) <= vx.tol {
                                ts.push((t, cuts.len()));
                                cuts.push(Cut {
                                    pair: Name::NONE,
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
                    // faces tangent along the edge: any curve reaching it grazes what lies across
                    let smooth = || {
                        let (x, y) = (body.face(f).surface.gradient(h.point), across.gradient(h.point));
                        let c = x.cross(y);
                        c.dot(c) <= 1e-8 * x.dot(x) * y.dot(y)
                    };
                    index[1 - k]
                        .face_box(faces[1 - k])
                        .grow(tol::LINEAR)
                        .contains(h.point)
                        && on_edge(body, e, h.point).is_some_and(|(s, _)| inner(s) || smooth())
                        && [1e-3, 1e-2, 5e-2, 0.2, 0.5].iter().find_map(|&w| {
                            let dt = w * range.width().min(1.0);
                            match side(h.t - dt) {
                                // leaving the face at this edge, tangent to what lies across it
                                Where::Outside if !smooth() => Some(false),
                                a => match (a, side(h.t + dt)) {
                                    (Where::Outside, Where::Inside) | (Where::Inside, Where::Outside) => {
                                        Some(smooth())
                                    }
                                    (Where::Outside, _) | (_, Where::Outside) => Some(false),
                                    (Where::Inside, Where::Inside) => Some(true),
                                    _ => None,
                                },
                            }
                        }) == Some(true)
                };
                // a tangent-looking root a hair along from this edge, whose neighbour leaves the face
                // nearly tangent: the curve crosses the edge itself, where it passes closest
                let slid = |h: CurveHit| {
                    // (near this edge's box only, the common case passed over cheaply)
                    if !body.edge_box(e).grow(1e-3).contains(h.point) || on_edge(body, e, h.point).is_some() {
                        return h;
                    }
                    let p = edge.curve.eval(edge.curve.closest(h.point, edge.range));
                    let normal = |g: FaceId| body.face_normal(g, body.face(g).surface.project(p));
                    let smooth = body.partner(c).map(|o| body.coedge_face(o)).is_some_and(|g| {
                        matches!((normal(f), normal(g)), (Some(a), Some(b)) if a.cross(b).length() <= 1e-4)
                    });
                    let t = curve.closest(p, range);
                    if smooth && p.distance(h.point) <= 1e-3 && curve.eval(t).distance(p) <= tol::LINEAR {
                        CurveHit { t, point: p, ..h }
                    } else {
                        h
                    }
                };
                for (h, grazing) in hits.into_iter().flat_map(|h| {
                    let grazing = h.contact != Contact::Crossing;
                    let near = crossings_near(&curve, range, &across, h);
                    let h = if grazing && near.is_empty() { slid(h) } else { h };
                    // (at the edge's end the curve meets the face's corner, whichever way it goes on)
                    let corner = || {
                        [edge.start, edge.end].iter().any(|&v| {
                            body.vertex(v).point.distance(h.point) <= body.vertex(v).tol.max(tol::LINEAR)
                        })
                    };
                    let touch = (grazing && near.is_empty() && (corner() || touch_inside(&h))).then_some(h);
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
                            pair: Name::NONE,
                            on: CutOn::Edge(side, e),
                            s,
                            point: h.point,
                            tol,
                            touch: grazing || branches[1],
                        });
                    }
                }
                // a traced branch stopping on the edge (on the rim of a face it runs over, past
                // which that surface's implicit no longer vanishes) crosses nothing there: it is
                // cut where it ends
                for &(t, p) in ends.iter().flatten() {
                    if !body.edge_box(e).grow(RIM).contains(p)
                        || cuts[first..].iter().any(|x| {
                            x.on == CutOn::Edge(side, e) && x.point.distance(p) <= x.tol.max(tol::LINEAR)
                        })
                    {
                        continue;
                    }
                    // one running onto the rim tangentially (a blend on its plane) is fixed along it
                    // only loosely and stops near it: it ends where the edge meets the other surface
                    let at = on_edge(body, e, p).map(|(s, tol)| (s, p, tol)).or_else(|| {
                        // (the box is no test on a diagonal edge: asking the other surface is dear)
                        if edge.curve.eval(edge.curve.closest(p, edge.range)).distance(p) > RIM {
                            return None;
                        }
                        let other = &bodies[1 - k].face(faces[1 - k]).surface;
                        let CurveSurface::Hits(hits) = curve_surface(&edge.curve, edge.range, other) else {
                            return None;
                        };
                        let h = hits
                            .into_iter()
                            .min_by(|a, b| a.point.distance(p).total_cmp(&b.point.distance(p)))?;
                        let d = h.point.distance(p);
                        (d <= RIM).then_some((h.t, h.point, d.max(tol::LINEAR)))
                    });
                    if let Some((s, q, tol)) = at
                        && index[1 - k].face_box(faces[1 - k]).grow(tol::LINEAR).contains(q)
                        && index[1 - k].point_in_face(faces[1 - k], q) != Where::Outside
                    {
                        ts.push((t, cuts.len()));
                        cuts.push(Cut {
                            pair: Name::NONE,
                            on: CutOn::Edge(side, e),
                            s,
                            point: q,
                            tol,
                            touch: branches[1],
                        });
                    }
                }
            }
        }
    }
    // a closed curve crosses a face's boundary an even number of times: one found odd has missed a
    // shallow crossing of a spring, which the fence across it can still find
    // (seams are no boundary: crossing one leaves nothing)
    let odd = |k: usize| {
        let mut at: Vec<Point3> = Vec::with_capacity(ts.len());
        for &(_, i) in &ts {
            let Some((sd, e)) = cuts[i].on.edge() else {
                continue;
            };
            let seam = || {
                let cs = &bodies[k].edge(e).coedges;
                cs.iter().all(|&c| bodies[k].coedge_face(c) == faces[k])
            };
            if sd == sides[k] && !seam() && at.iter().all(|p| p.distance(cuts[i].point) > 10.0 * tol::LINEAR)
            {
                at.push(cuts[i].point);
            }
        }
        at.len() % 2 == 1
    };
    if closed && !fenced.is_empty() && !FENCES.with(|f| f.get()) && fenced.iter().any(|x| odd(x.0)) {
        ODD.with(|o| o.set(true));
    }
    if closed && FENCES.with(|f| f.get()) && fenced.iter().any(|x| odd(x.0)) {
        for &(k, e) in &fenced {
            let (body, side) = (bodies[k], sides[k]);
            let Curve::Procedural(pc) = &body.edge(e).curve else {
                continue;
            };
            let Some(spring) =
                pc.0.as_any()
                    .and_then(|a| a.downcast_ref::<forge_geom::blend::Spring>())
            else {
                continue;
            };
            let across = spring.fence();
            let box_e = Some(body.edge_box(e).grow(2.0 * tol::LINEAR));
            if let CurveSurface::Hits(hs) = curve_surface_in(&curve, range, &across, || box_e) {
                for h in hs {
                    if let Some((s, tol)) = on_edge(body, e, h.point)
                        && !cuts[first..].iter().any(|x| x.point.distance(h.point) <= 1e-2)
                        && index[1 - k].point_in_face(faces[1 - k], h.point) != Where::Outside
                    {
                        ts.push((h.t, cuts.len()));
                        cuts.push(Cut {
                            pair: Name::NONE,
                            on: CutOn::Edge(side, e),
                            s,
                            point: h.point,
                            tol,
                            touch: branches[1],
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
                        pair: Name::NONE,
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
                // the whole stretch on the other face, not just its middle where a curve touches it
                let on_it = |x: f64| {
                    let q = edge.curve.eval(w[0] + x * (w[1] - w[0]));
                    bodies[1 - k].face(into).surface.distance(q) <= tol::LINEAR
                };
                let straight = matches!(edge.curve, Curve::Line(_));
                if index[1 - k].point_in_face(into, m) == Where::Inside
                    && (straight || on_it(0.25) && on_it(0.75))
                {
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
                    pair: Name::NONE,
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
    // a curve leaving an edge at a shallow angle stays within tolerance of it for a while, so where
    // it "crosses" is anywhere along that stretch. With another edge's cut in the stretch, it
    // crosses there; out and back over one edge, in the stretch's middle: two vertices a hair apart
    // would need an edge along the first edge
    let pairs = match curve {
        Curve::Line(_) => 0,
        _ if closed => ts.len(),
        _ => ts.len().saturating_sub(1),
    };
    for w in 0..pairs {
        let next = (w + 1) % ts.len();
        let (i, j) = (ts[w].1, ts[next].1);
        if i == j {
            continue;
        }
        let gap = cuts[i].point.distance(cuts[j].point);
        // (a curve radius R stays within t of a tangent for sqrt(8 R t): under a third of a mm unless
        // R is over a metre)
        if gap <= 1.2 * cuts[i].tol.max(cuts[j].tol) || gap > 0.3 {
            continue;
        }
        let wrap = if next == 0 { period } else { 0.0 };
        let mid = curve.eval(0.5 * (ts[w].0 + ts[next].0 + wrap));
        let onto = |c: &Cut, q: Point3| match c.on {
            CutOn::Edge(sd, e) => on_edge(bodies[(sides[0] != sd) as usize], e, q),
            CutOn::Free => None,
        };
        if onto(&cuts[i], mid).is_none() && onto(&cuts[j], mid).is_none() {
            continue;
        }
        if cuts[i].on == cuts[j].on {
            if let Some((s, tol)) = onto(&cuts[i], mid) {
                for k in [i, j] {
                    (cuts[k].s, cuts[k].point, cuts[k].tol) = (s, mid, tol);
                }
            }
            continue;
        }
        for (from, to) in [(j, i), (i, j)] {
            if onto(&cuts[from], mid).is_some()
                && let Some((s, tol)) = onto(&cuts[from], cuts[to].point)
            {
                let point = cuts[to].point;
                (cuts[from].s, cuts[from].point, cuts[from].tol) = (s, point, tol);
                break;
            }
        }
    }
    // the same point found on two edges (a vertex, or both faces' boundaries) is one cut; a
    // curve coming back to a point it passed (a branch crossing itself where the surfaces touch)
    // meets it again, a cut each time
    let found = ts.len();
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
    let merged = ts.len() < found;
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
    // a closed curve with no second cut whose far point touches a boundary (an ellipse's top on the
    // rim its plane passes through), inside both faces elsewhere: a vertex there, or no sample of
    // the whole loop says it is a new edge and the cut is lost
    let mut loops = loops;
    // (what the middle says of the whole loop needs no asking again: inside both, the usual hole;
    // off either; on a boundary there and further round too, so along it)
    let mut whole = None;
    if closed && ts.len() <= 1 {
        let (t0, t1) = ts.first().map_or((range.lo, range.hi), |&(t, _)| (t, t + period));
        let mid = 0.5 * (t0 + t1);
        let p = curve.eval(mid);
        let w0 = index[0].point_in_face(faces[0], p);
        let w1 = if w0 == Where::Outside {
            Where::Outside
        } else {
            index[1].point_in_face(faces[1], p)
        };
        whole = match [w0, w1] {
            [Where::Inside, Where::Inside] => Some(true),
            w if w.contains(&Where::Outside) => Some(false),
            w => {
                let q = curve.eval(t0 + 0.381_966_011_250_105_1 * (t1 - t0));
                let on = usize::from(w[0] != Where::On);
                let there = index[on].point_in_face(faces[on], q);
                if there == Where::Inside && index[1 - on].point_in_face(faces[1 - on], q) == Where::Inside {
                    loops.push(mid);
                    None
                } else {
                    (there == Where::On).then_some(false)
                }
            }
        };
    }
    for t in loops {
        let t = if closed && t >= range.hi { t - period } else { t };
        ts.push((t, cuts.len()));
        cuts.push(Cut {
            pair: Name::NONE,
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
        if let Some(w) = whole {
            return Ok(w);
        }
        let at = |k: usize, p: Point3| match index[k].point_in_face(faces[k], p) {
            Where::On => {
                let past = crossed(k, ends);
                // an edge it crosses only to run within tolerance of it (a chord of a hair) is
                // one it lies along
                let hair = || {
                    curve.eval(t0).distance(p).min(curve.eval(t1).distance(p)) > 4.0 * tol::LINEAR
                        && past.iter().any(|&e| on_edge(bodies[k], e, p).is_some())
                };
                if past.is_empty() || hair() {
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
            // a closed curve whole (no cut, or one) that touches a boundary has a vertex there by now
            // (above). Between two cuts, a stretch from one edge back to the
            // same edge, with its middle on that edge, runs along it; one whose middle is on
            // another edge only passes it within tolerance (a tool's edge a hair clear of the
            // surface makes no cut there) and is a new edge if it is inside elsewhere
            if f != 0.5
                && (ends.is_empty()
                    || ends[0] == ends[1]
                    || (0..2).any(|k| {
                        w[k] == Where::On && {
                            let body = bodies[k];
                            let (m, a, b) = (curve.eval(0.5 * (t0 + t1)), curve.eval(t0), curve.eval(t1));
                            // both ends cut on one edge and the middle on it too
                            let holds =
                                |c: &Cut, i: usize| cuts[i].point.distance(c.point) <= cuts[i].tol.max(c.tol);
                            let cut_along = cuts[first..].iter().any(|c| match c.on.edge() {
                                Some((sd, e)) if sd == sides[k] && holds(c, ends[0]) => {
                                    cuts[first..]
                                        .iter()
                                        .any(|d| d.on.edge() == Some((sd, e)) && holds(d, ends[1]))
                                        && on_edge(body, e, m).is_some()
                                }
                                _ => false,
                            });
                            cut_along
                                || body.face(faces[k]).loops.iter().any(|&l| {
                                    body.lp(l).coedges().any(|c| {
                                        let e = body.coedge(c).edge;
                                        let eb = body.edge_box(e).grow(tol::LINEAR);
                                        eb.contains(m)
                                            && eb.contains(a)
                                            && eb.contains(b)
                                            && on_edge(body, e, m).is_some()
                                            && on_edge(body, e, a).is_some()
                                            && on_edge(body, e, b).is_some()
                                    })
                                })
                        }
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
                pair: name,
                end_names: [Name::NONE; 2],
                end_keys: [Name::NONE; 2],
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
                pair: name,
                end_names: [Name::NONE; 2],
                end_keys: [Name::NONE; 2],
            });
        }
    }
    // a piece of the curve is named by the edges it runs between (either way round), not its
    // place along the curve; pieces of the pair alike are ranked later (`rank_pairs`)
    let free = name.with_str("free");
    let one = |i: usize| match cuts[i].on {
        CutOn::Edge(sd, e) => bodies[if sides[0] == sd { 0 } else { 1 }].edge(e).name.whole(),
        CutOn::Free => free,
    };
    // where the curve crosses several edges at once (a vertex, or edges of both faces), by all
    // of them: which of the cuts there ends the piece is chance
    let edge_name = |i: usize| {
        let near = |j: usize| j != i && cuts[j].point.distance(cuts[i].point) <= cuts[i].tol.max(cuts[j].tol);
        // (only where cuts were found to be one)
        if !merged || !(first..cuts.len()).any(near) {
            return one(i);
        }
        Name::NONE.with_set(
            &(first..cuts.len())
                .filter(|&j| j == i || near(j))
                .map(one)
                .collect::<Vec<_>>(),
        )
    };
    for sg in &mut segs[first_seg..] {
        let (i0, i1) = sg.ends.expect("an open piece");
        let (a, b) = (edge_name(i0), edge_name(i1));
        sg.name = name.with(0x5e7).with_name(a.min(b)).with_name(a.max(b));
        sg.end_names = [name.with_name(a), name.with_name(b)];
        sg.end_keys = [a, b];
    }
    for c in &mut cuts[first..] {
        c.pair = name;
    }
    Ok(())
}

/// The pieces of one pair of whole faces' intersection (all its branches, on any pieces of the
/// faces) that the edges they run between do not tell apart, told apart by where they lie in the
/// first whole's surface frame: the part's own, so turning or moving the part, or rounding, keeps
/// the order, and a face split earlier names its curves as the whole face would.
pub(crate) fn rank_pairs(segs: &mut [Seg], bodies: &[&Body]) {
    // pieces alike: the same pair and name, by one key (most booleans have none; few pieces are
    // looked at pairwise)
    let key = |s: &Seg| s.pair.0 ^ s.name.0.rotate_left(29);
    let n = segs.len();
    if n <= 32 && !(0..n).any(|i| (i + 1..n).any(|j| key(&segs[i]) == key(&segs[j]))) {
        return;
    }
    let mut by: Vec<(u64, u32)> = segs.iter().enumerate().map(|(i, s)| (key(s), i as u32)).collect();
    by.sort_unstable();
    for g in by.chunk_by(|a, b| a.0 == b.0).filter(|g| g.len() > 1) {
        // (a key two pairs share: each pair apart)
        let mut at: Vec<(Name, Name, usize)> = g
            .iter()
            .map(|x| x.1 as usize)
            .map(|i| (segs[i].pair, segs[i].name, i))
            .collect();
        at.sort_unstable();
        for at in at.chunk_by(|a, b| (a.0, a.1) == (b.0, b.1)) {
            let at: Vec<usize> = at.iter().map(|x| x.2).collect();
            if at.len() < 2 {
                continue;
            }
            let s0 = &segs[at[0]];
            let (fa, fb) = (
                bodies[s0.sides[0]].face(s0.faces[0]),
                bodies[s0.sides[1]].face(s0.faces[1]),
            );
            let frame = surface_frame(if fa.name.whole() <= fb.name.whole() {
                &fa.surface
            } else {
                &fb.surface
            });
            let mut names: Vec<Name> = at.iter().map(|&i| segs[i].name).collect();
            rank_names(&mut names, &|k| {
                let sg = &segs[at[k]];
                frame.point_to_local(curve_at(&sg.curve, sg.range, sg.ends.is_some()))
            });
            for (&i, n) in at.iter().zip(names) {
                segs[i].name = n;
            }
        }
    }
}

/// Where a curve piece lies, for ranking: an open one's middle, a closed one's samples' mean
/// (wherever its parameter starts).
pub(crate) fn curve_at(curve: &Curve, range: Interval, open: bool) -> Point3 {
    if open {
        return curve.eval(range.mid());
    }
    let sum = (0..8).fold(forge_math::Vec3::ZERO, |s, k| {
        s + curve.eval(range.lo + range.width() * k as f64 / 8.0).to_vec()
    });
    Point3::ORIGIN + sum * 0.125
}

/// Names alike told apart by where their pieces lie (`at`, in the pair's frame): `rank_pairs`.
pub(crate) fn rank_names(names: &mut [Name], at: &dyn Fn(usize) -> Point3) {
    let n = names.len();
    if !(0..n).any(|i| (i + 1..n).any(|j| names[i] == names[j])) {
        return;
    }
    let mut by_name: Vec<(Name, usize)> = names.iter().enumerate().map(|(i, &n)| (n, i)).collect();
    by_name.sort_unstable();
    for group in by_name.chunk_by(|a, b| a.0 == b.0).filter(|g| g.len() > 1) {
        if let [(_, i), (_, j)] = *group {
            let later = if ranked_pair(at(i), at(j)) { j } else { i };
            names[later] = names[later].with(1);
            continue;
        }
        let pts: Vec<Point3> = group.iter().map(|x| at(x.1)).collect();
        for (x, r) in group.iter().zip(ranked(&pts)) {
            if r > 0 {
                names[x.1] = names[x.1].with(r);
            }
        }
    }
}

/// Whether `p` comes before `q` as `ranked` orders the two.
fn ranked_pair(p: Point3, q: Point3) -> bool {
    let d = [q.x - p.x, q.y - p.y, q.z - p.z];
    let k = (0..3)
        .max_by(|&a, &b| d[a].abs().total_cmp(&d[b].abs()))
        .unwrap_or(0);
    d[k] > 0.0
}

/// Each point's rank among `pts`, along the axis they spread furthest on (then the next):
/// points that sit nearly level on one axis are not ordered by noise on it.
pub fn ranked(pts: &[Point3]) -> Vec<u64> {
    if pts.len() < 2 {
        return vec![0; pts.len()];
    }
    let coord = |p: Point3, k: usize| [p.x, p.y, p.z][k];
    let spread = |k: usize| {
        let (lo, hi) = pts
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &p| {
                (lo.min(coord(p, k)), hi.max(coord(p, k)))
            });
        hi - lo
    };
    let mut axes = [0usize, 1, 2];
    axes.sort_by(|&a, &b| spread(b).total_cmp(&spread(a)));
    let near = |a: f64, b: f64| (a - b).abs() <= 1e-6 * (1.0 + a.abs().max(b.abs()));
    let less = |p: Point3, q: Point3| {
        for &k in &axes {
            let (a, b) = (coord(p, k), coord(q, k));
            if !near(a, b) {
                return a < b;
            }
        }
        false
    };
    pts.iter()
        .map(|&p| pts.iter().filter(|&&q| less(q, p)).count() as u64)
        .collect()
}

/// Marks for the new vertices nothing ends at (`ended`): each edge cut there, as a whole, with
/// the faces whose curve cut it. Where a face on another's surface imprints edges there (a flush
/// crossing, which the sequential regen meets a tool at a time and the n-ary all at once), one
/// mark of the imprinted edges together: crossings of one edge by several features' imprints,
/// made by different booleans, are told apart by it.
pub(crate) fn cut_marks(
    cuts: &[Cut],
    cut_vertex: &[VKey],
    ended: &[bool],
    edge_name: &dyn Fn(Side, EdgeId) -> Name,
    marks: &mut Vec<(u32, Name, Name)>,
) {
    let mut flush: Vec<(u32, Name)> = Vec::new();
    for (c, &k) in cuts.iter().zip(cut_vertex).filter(|x| x.0.pair == Name::NONE) {
        if let VKey::New(i) = k
            && !ended[i as usize]
            && c.pair == Name::NONE
            && let Some((sd, e)) = c.on.edge()
        {
            flush.push((i, edge_name(sd, e).whole()));
        }
    }
    flush.sort_unstable();
    flush.dedup();
    let mut at = vec![false; if flush.is_empty() { 0 } else { ended.len() }];
    for g in flush.chunk_by(|a, b| a.0 == b.0) {
        let names: Vec<Name> = g.iter().map(|x| x.1).collect();
        let n = Name::NONE.with_str("flush").with_set(&names);
        marks.push((g[0].0, n, n));
        at[g[0].0 as usize] = true;
    }
    for (c, &k) in cuts.iter().zip(cut_vertex) {
        if let VKey::New(i) = k
            && !ended[i as usize]
            && !at.get(i as usize).is_some_and(|&x| x)
            && let Some((sd, e)) = c.on.edge()
        {
            let n = c.pair.with_name(edge_name(sd, e).whole());
            marks.push((i, n, n));
        }
    }
}

/// The part's own frame, where ties among new vertices are broken: its least-named face's surface's,
/// so it turns and moves with the part and does not follow the order in the arenas.
pub(crate) fn part_frame(b: &Body) -> forge_math::Frame {
    b.face_ids()
        .min_by_key(|&f| b.face(f).name)
        .map_or(forge_math::Frame::WORLD, |f| surface_frame(&b.face(f).surface))
}

/// The surface's own frame: where ties among what it holds are broken.
pub fn surface_frame(s: &Surface) -> forge_math::Frame {
    match s {
        Surface::Plane(p) => p.frame,
        Surface::Cylinder(c) => c.frame,
        Surface::Cone(c) => c.frame,
        Surface::Sphere(c) => c.frame,
        Surface::Torus(c) => c.frame,
        Surface::Procedural(_) => forge_math::Frame::WORLD,
    }
}

/// What meets at each new vertex where a curve piece ends: the piece's faces and what it cuts
/// there. (point, name)
/// And for vertices those leave alike, the pieces themselves, with which end.
pub(crate) fn curve_ends(segs: &[Seg], cut_vertex: &[VKey]) -> Vec<(u32, Name, Name)> {
    let mut out = Vec::with_capacity(2 * segs.len());
    for sg in segs {
        let Some((i0, i1)) = sg.ends else { continue };
        for (k, end) in [(cut_vertex[i0], 0), (cut_vertex[i1], 1)] {
            if let VKey::New(i) = k {
                out.push((i, sg.end_names[end], sg.name.with(end as u64)));
            }
        }
    }
    out
}

/// Names for the new vertices the result keeps (`held`, if known) from what meets there (`marks`: point and
/// name: the curve pieces ending there, else the edges cut), not from the order they were found
/// in. Points where the same ones meet are told apart by where they lie.
pub(crate) fn vertex_names(
    points: &[Point3],
    marks: Vec<(u32, Name, Name)>,
    root: Name,
    frame: impl Fn() -> forge_math::Frame,
    held: Option<&[bool]>,
) -> Vec<Name> {
    let held = |i: usize| held.is_none_or(|h| h[i]);
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
    let mut by_point = vec![(Name::NONE, Name::NONE); marks.len()];
    for m in marks {
        by_point[fill[m.0 as usize]] = (m.1, m.2);
        fill[m.0 as usize] += 1;
    }
    let root = root.with_str("vertex");
    // a cheap mix: there are many of these, and the names mixed are hashes already
    let mix = |names: &mut dyn Iterator<Item = Name>| {
        let (mut key, mut last) = (root.0, None);
        for x in names {
            if last != Some(x) {
                key = (key ^ x.0).wrapping_mul(0x9e37_79b9_7f4a_7c15);
                key ^= key >> 29;
            }
            last = Some(x);
        }
        Name(key)
    };
    let keys: Vec<Name> = (0..n)
        .map(|i| {
            if !held(i) {
                return Name::NONE;
            }
            let group = &mut by_point[start[i]..start[i + 1]];
            // the least of what meets there: a curve piece that rounding leaves out at the
            // point (where coplanar faces touch) does not change it; vertices it leaves alike
            // are told apart below
            group.sort_unstable_by_key(|g| g.0);
            mix(&mut group.iter().map(|g| g.0).take(1))
        })
        .collect();
    // (vertices the result drops need no name, nor tell others apart)
    let mut order: Vec<usize> = (0..n).filter(|&i| held(i)).collect();
    order.sort_unstable_by_key(|&i| (keys[i], i));
    let mut out = vec![Name::NONE; n];
    for group in order.chunk_by(|&i, &j| keys[i] == keys[j]) {
        if let [i] = group {
            out[*i] = keys[*i].with(0);
            continue;
        }
        // vertices alike are told apart by their marks' second names, then by where they lie,
        // along the way they spread most
        let mut second = |i: usize| {
            let marks = &mut by_point[start[i]..start[i + 1]];
            marks.sort_unstable_by_key(|g| g.1);
            mix(&mut marks.iter().map(|g| g.1))
        };
        if let [i, j] = *group {
            let (a, b) = (second(i), second(j));
            let first = if a != b {
                a < b
            } else {
                let f = frame();
                !ranked_pair(f.point_to_local(points[j]), f.point_to_local(points[i]))
            };
            let (lo, hi) = if first { (i, j) } else { (j, i) };
            out[lo] = keys[lo].with(0);
            out[hi] = keys[hi].with(1);
            continue;
        }
        let mut by_second: Vec<(Name, usize)> = group.iter().map(|&i| (second(i), i)).collect();
        by_second.sort_unstable();
        let mut rank = 0u64;
        for sub in by_second.chunk_by(|a, b| a.0 == b.0) {
            let f = frame();
            let pts: Vec<Point3> = sub.iter().map(|x| f.point_to_local(points[x.1])).collect();
            for (x, r) in sub.iter().zip(ranked(&pts)) {
                out[x.1] = keys[x.1].with(rank + r);
            }
            rank += sub.len() as u64;
        }
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
/// piece is kept keeps its own name: it was only trimmed. Returns the edges left in several
/// pieces, as wholes, and each new piece with what it was split from.
fn sole_pieces_keep_names(
    bodies: &[&Body; 2],
    specs: &mut [Spec],
    kept: &[(usize, bool)],
    pieces: &mut [Piece],
    uses: &[u8],
    split_runs: &[(Side, EdgeId, std::ops::Range<usize>)],
    vname: &dyn Fn(VKey) -> Name,
) -> (Vec<Name>, Vec<(Name, Name)>) {
    let (mut split, mut wholes) = (Vec::new(), Vec::new());
    for (side, e, run) in split_runs {
        let edge = bodies[*side].edge(*e).name;
        let used: Vec<usize> = run.clone().filter(|&p| uses[p] > 0).collect();
        if used.len() > 1 {
            split.push(bodies[*side].split_whole(edge).unwrap_or(edge));
        }
        for &p in &used {
            pieces[p].name = if used.len() == 1 {
                edge
            } else {
                let n = piece_name(edge, &pieces[p], vname);
                wholes.push((n, edge));
                n
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
                let whole = bodies[s.side].face(s.from).name;
                s.name = region_name(whole, &s.loops, pieces);
                wholes.push((s.name, whole));
            }
        }
        at = end;
    }
    (split, wholes)
}

/// An edge of one body kept whole along a face of the other it lay flush on (a block standing on
/// a plate) is named as the cut it becomes when the bodies overlap a little: by the face it
/// lies on and its own other face, and the edges at its ends; its ends' vertices likewise. So
/// sinking or lifting a feature by a hair keeps the names. Returns the vertices renamed, sorted.
pub(crate) fn flush_edges_as_cuts(
    bodies: &[&Body],
    mut kept_faces: Vec<(Side, FaceId)>,
    pieces: &mut [Piece],
    uses: &[u8],
    coplanar: &[[(Side, FaceId); 2]],
    whole_piece: &dyn Fn(Side, EdgeId) -> Option<usize>,
    root: Name,
) -> Vec<(VKey, Name)> {
    if coplanar.is_empty() {
        return Vec::new();
    }
    kept_faces.sort_unstable();
    let is_kept = |x: (Side, FaceId)| kept_faces.binary_search(&x).is_ok();
    // each coplanar face and the face it lay on, in order
    let mut partners: Vec<((Side, FaceId), (Side, FaceId))> =
        coplanar.iter().flat_map(|&[x, y]| [(x, y), (y, x)]).collect();
    partners.sort_unstable();
    let cut = root.with_str("cut");
    let mut at_vertex: Vec<(VKey, u64)> = Vec::new();
    // the faces that went, each with the other body's face that took its place
    let mut gone_faces: Vec<((Side, FaceId), (Side, FaceId))> = Vec::new();
    for &(x, y) in &partners {
        if !is_kept(x) && is_kept(y) && gone_faces.last().is_none_or(|g| g.0 != x) {
            gone_faces.push((x, y));
        }
    }
    let mut done: Vec<usize> = Vec::new();
    // (name, piece, its ends and their marks, the face it now lies on)
    type Named = (Name, usize, [(VKey, Name); 2], (Side, FaceId));
    let mut named: Vec<Named> = Vec::new();
    for ((sd, gone), onto) in gone_faces {
        let body = bodies[sd];
        for &l in &body.face(gone).loops {
            for gc in body.lp(l).coedges() {
                let e = body.coedge(gc).edge;
                let Some(p) = whole_piece(sd, e) else { continue };
                if uses[p] == 0 || done.contains(&p) {
                    continue;
                }
                done.push(p);
                let Some(other) = body
                    .edge(e)
                    .coedges
                    .iter()
                    .map(|&c| body.coedge_face(c))
                    .find(|&f| f != gone)
                else {
                    continue;
                };
                if let Some((name, ends)) = flush_name(body, sd, e, gone, other, onto, bodies, cut) {
                    named.push((name, p, ends, onto));
                }
            }
        }
    }
    // flush edges their neighbours do not tell apart (a revolve's seams on a face), by where they lie
    named.sort_unstable_by_key(|x| (x.0, x.1));
    for g in named.chunk_by_mut(|a, b| a.0 == b.0).filter(|g| g.len() > 1) {
        let (sd, f) = g[0].3;
        let frame = surface_frame(&bodies[sd].face(f).surface);
        let mut names: Vec<Name> = g.iter().map(|x| x.0).collect();
        rank_names(&mut names, &|k| {
            let pc = &pieces[g[k].1];
            frame.point_to_local(pc.curve.eval(pc.range.mid()))
        });
        for (x, n) in g.iter_mut().zip(names) {
            x.0 = n;
        }
    }
    for &(name, p, ends, _) in &named {
        pieces[p].name = name;
        at_vertex.extend(ends.map(|(v, m)| (v, m.0)));
    }
    at_vertex.sort_unstable_by_key(|x| x.0);
    let vertex = root.with_str("vertex");
    let mut out: Vec<(VKey, Name)> = Vec::new();
    for x in at_vertex {
        match out.last_mut() {
            Some(last) if last.0 == x.0 => last.1 = Name(last.1.0.wrapping_add(x.1)),
            _ => out.push((x.0, Name(x.1))),
        }
    }
    for o in &mut out {
        o.1 = vertex.with(o.1.0).with(0);
    }
    // vertices their marks do not tell apart (a flat's two ends on a rod), by where they lie
    let mut sorted: Vec<Name> = out.iter().map(|x| x.1).collect();
    sorted.sort_unstable();
    if sorted.windows(2).any(|w| w[0] == w[1])
        && let Some(&(_, _, _, (sd, f))) = named.first()
    {
        let frame = surface_frame(&bodies[sd].face(f).surface);
        let lie = |v: VKey| {
            named.iter().find_map(|&(_, p, ends, _)| {
                let pc = &pieces[p];
                ends.iter().any(|e| e.0 == v).then(|| {
                    let t = if v == pc.start { pc.range.lo } else { pc.range.hi };
                    pc.curve.eval(t)
                })
            })
        };
        let mut names: Vec<Name> = out.iter().map(|x| x.1).collect();
        rank_names(&mut names, &|k| {
            frame.point_to_local(lie(out[k].0).unwrap_or(Point3::ORIGIN))
        });
        for (o, n) in out.iter_mut().zip(names) {
            o.1 = n;
        }
    }
    out
}

/// The name a flush edge `e` (of body `sd`, between face `gone`, which went, and `other`) takes
/// as a cut between `other` and the face `onto` that took `gone`'s place, and its ends with what
/// each adds to its vertex's key.
#[allow(clippy::too_many_arguments)]
fn flush_name(
    body: &Body,
    sd: Side,
    e: EdgeId,
    gone: FaceId,
    other: FaceId,
    onto: (Side, FaceId),
    bodies: &[&Body],
    cut: Name,
) -> Option<(Name, [(VKey, Name); 2])> {
    // at each end, the edge of its other face that meets it there: its neighbours in that
    // face's loop (not on the face it lay on)
    let edge = body.edge(e);
    let &c = edge.coedges.iter().find(|&&c| body.coedge_face(c) == other)?;
    let lp = body.lp(body.coedge(c).lp);
    let k = lp.coedges().position(|x| x == c).expect("in its loop");
    let n = lp.coedges().len();
    let beside = |x: forge_topo::CoedgeId| {
        let x = body.coedge(x).edge;
        let on_gone = body.edge(x).coedges.iter().any(|&c| body.coedge_face(c) == gone);
        (x != e && !on_gone).then(|| body.edge(x).name.whole())
    };
    let (before, after) = (beside(lp.coedge((k + n - 1) % n)), beside(lp.coedge((k + 1) % n)));
    // the one before meets it where the coedge starts
    let at_start = body.coedge_start(c) == edge.start;
    let (Some(n0), Some(n1)) = (if at_start {
        (before, after)
    } else {
        (after, before)
    }) else {
        return None;
    };
    // as the boolean names the cut it is (`trim`): the faces as wholes and the edges it runs
    // between, either way round; each end by the edge there (and, where that is the other end's
    // too, by its vertex)
    let pair = cut.with_set(&[
        body.face(other).name.whole(),
        bodies[onto.0].face(onto.1).name.whole(),
    ]);
    let name = pair.with(0x5e7).with_name(n0.min(n1)).with_name(n0.max(n1));
    let end = |v: VertexId, n: Name| {
        let m = pair.with_name(n);
        (
            VKey::Old(sd, v),
            if n0 == n1 {
                m.with_name(body.vertex(v).name)
            } else {
                m
            },
        )
    };
    Some((name, [end(edge.start, n0), end(edge.end, n1)]))
}

/// A piece of edge `edge` named by its ends, not its place in the order along the edge.
pub(crate) fn piece_name(edge: Name, pc: &Piece, vname: &dyn Fn(VKey) -> Name) -> Name {
    edge.piece(vname(pc.start).with_name(vname(pc.end)))
}

/// Where a line or circle of plane `pl` crosses a circle of the plane, or a circle a line, over the
/// curve's `range`, in closed form. None when they come near touching, run together, or are
/// other curves (two lines among them: a line meets the neighbouring plane as cheaply): the
/// edge's neighbouring surface decides then.
fn plane_meet(pl: &Plane, curve: &Curve, range: Interval, edge: &Curve) -> Option<Vec<CurveHit>> {
    let n = pl.frame.z();
    // the points where two of these meet, if they cross cleanly
    let line_circle = |o: Point3, d: Vec3, c: Point3, r: f64| -> Option<Vec<Point3>> {
        let w = o - c;
        let (b, cc) = (d.dot(w), w.dot(w) - r * r);
        let disc = b * b - cc;
        if disc.abs() <= 1e-6 * r * r {
            return None;
        }
        if disc < 0.0 {
            return Some(Vec::new());
        }
        let q = disc.sqrt();
        Some(vec![o + d * (-b - q), o + d * (-b + q)])
    };
    let pts: Vec<Point3> = match (curve, edge) {
        (Curve::Line(a), Curve::Circle(k)) | (Curve::Circle(k), Curve::Line(a)) => {
            if k.frame.z().dot(n).abs() < 1.0 - 1e-12 {
                return None;
            }
            line_circle(a.origin, a.dir.get(), k.frame.origin, k.radius)?
        }
        (Curve::Circle(k1), Curve::Circle(k2)) => {
            if k1.frame.z().dot(n).abs() < 1.0 - 1e-12 || k2.frame.z().dot(n).abs() < 1.0 - 1e-12 {
                return None;
            }
            let (c1, c2, r1, r2) = (k1.frame.origin, k2.frame.origin, k1.radius, k2.radius);
            let d = c1.distance(c2);
            let r = r1.min(r2);
            // concentric, or near touching inside or out
            if d <= 1e-3 * r || (d - (r1 + r2)).abs() <= 1e-3 * r || (d - (r1 - r2).abs()).abs() <= 1e-3 * r {
                return if d <= 1e-3 * r && (r1 - r2).abs() > 1e-3 * r {
                    Some(Vec::new())
                } else {
                    None
                };
            }
            if d > r1 + r2 || d < (r1 - r2).abs() {
                return Some(Vec::new());
            }
            let u = (c2 - c1) * (1.0 / d);
            let a = (d * d + r1 * r1 - r2 * r2) / (2.0 * d);
            let h = (r1 * r1 - a * a).max(0.0).sqrt();
            let v = n.cross(u);
            vec![c1 + u * a + v * h, c1 + u * a - v * h]
        }
        _ => return None,
    };
    let mut hits: Vec<CurveHit> = Vec::with_capacity(pts.len());
    for p in pts {
        let t = match curve {
            Curve::Line(l) => {
                let t = (p - l.origin).dot(l.dir.get());
                if !range.contains(t) {
                    continue;
                }
                t
            }
            Curve::Circle(k) => {
                let q = p - k.frame.origin;
                let t0 = dmath::atan2(q.dot(k.frame.y().get()), q.dot(k.frame.x().get()));
                let t = range.lo + dmath::wrap_tau(t0 - range.lo);
                if t > range.hi + 1e-12 {
                    continue;
                }
                t.min(range.hi)
            }
            _ => return None,
        };
        // polished on the edge: Newton steps on its distance while they bring the point nearer
        let gap = |t: f64| {
            let d = curve.deriv(t);
            match edge {
                Curve::Line(l) => {
                    let m = n.cross(l.dir);
                    ((d.p - l.origin).dot(m), d.d1.dot(m))
                }
                Curve::Circle(k) => {
                    let q = d.p - k.frame.origin;
                    let len = q.length();
                    (len - k.radius, q.dot(d.d1) / len)
                }
                _ => (0.0, 1.0),
            }
        };
        let mut t = t;
        let mut g = gap(t);
        for _ in 0..3 {
            if g.0 == 0.0 || g.1 == 0.0 {
                break;
            }
            let t1 = t - g.0 / g.1;
            let g1 = gap(t1);
            let nearer = g1.0.abs() < g.0.abs();
            if !nearer || !range.contains(t1) {
                break;
            }
            (t, g) = (t1, g1);
        }
        hits.push(CurveHit {
            t,
            point: curve.eval(t),
            contact: Contact::Crossing,
        });
    }
    hits.sort_by(|x, y| x.t.total_cmp(&y.t));
    hits.dedup_by(|x, y| x.point.distance(y.point) <= tol::LINEAR);
    Some(hits)
}

/// Whether face `f` rolls off plane `other` along one of its edges: a blend meeting a face on
/// that plane tangentially. The plane only touches the blend there (an arc stays to one side of
/// its tangent), and that touch is the edge; traced, it would come out a hair off it.
pub(crate) fn rolls_off(body: &Body, f: FaceId, other: &Surface) -> bool {
    if !matches!(other, Surface::Plane(_)) || matches!(body.face(f).surface, Surface::Plane(_)) {
        return false;
    }
    body.face(f)
        .loops
        .iter()
        .flat_map(|&l| body.lp(l).coedges())
        .any(|c| {
            let Some(g) = body.partner(c).map(|o| body.coedge_face(o)) else {
                return false;
            };
            let s = &body.face(g).surface;
            if g == f || !matches!(s, Surface::Plane(_)) || !ssi::coincident(s, other) {
                return false;
            }
            let e = body.edge(body.coedge(c).edge);
            let p = e.curve.eval(e.range.mid());
            let normal = |h: FaceId| body.face_normal(h, body.face(h).surface.project(p));
            matches!((normal(f), normal(g)), (Some(x), Some(y)) if x.cross(y).length() <= 1e-6)
        })
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
            // (for a straight edge also where they all but do: the neighbour crosses at so shallow
            // an angle that a hair of rounding moves the point along the curve past the edge)
            let flat = if matches!(edge.curve, Curve::Line(_)) {
                1e-3
            } else {
                1e-6
            };
            let smooth = other
                .normal(other.project(d.p))
                .is_some_and(|m| m.cross(n).length() <= flat);
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
    along: &mut Vec<Along>,
) -> R<()> {
    let dst = 1 - src;
    let (sb, db) = (bodies[src], bodies[dst]);
    // how far a point an edge reaches may be from its box: its ends' tolerance
    let slack = |body: &Body, e: EdgeId| {
        let edge = body.edge(e);
        body.vertex(edge.start).tol.max(body.vertex(edge.end).tol)
    };
    for &l in &sb.face(src_face).loops {
        for c in sb.lp(l).coedges() {
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
                for c2 in db.lp(l2).coedges() {
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
                            along.push(if src == 0 {
                                (sides[src], e, sides[dst], e2)
                            } else {
                                (sides[dst], e2, sides[src], e)
                            });
                            continue;
                        }
                    };
                    // this edge's end on e2 beside a crossing but not at one (a corner on a rim it all
                    // but runs along): e2 ends there too, or the two come out in different pieces
                    let straight = matches!(
                        (&edge.curve, &db.edge(e2).curve),
                        (Curve::Line(_), Curve::Line(_))
                    );
                    for v in [edge.start, edge.end].into_iter().filter(|_| !straight) {
                        let p = sb.vertex(v).point;
                        let end =
                            |w: VertexId| db.vertex(w).point.distance(p) <= db.vertex(w).tol.max(tol::LINEAR);
                        let e2_ends = db.edge(e2);
                        if db.edge_box(e2).grow(grow + slack(db, e2)).contains(p)
                            && hits.iter().any(|h| h.point.distance(p) <= 0.3)
                            && !hits.iter().any(|h| h.point.distance(p) <= tol::LINEAR)
                            && !end(e2_ends.start)
                            && !end(e2_ends.end)
                            && let Some((s2, tol)) = on_edge(db, e2, p)
                        {
                            cuts.push(Cut {
                                pair: Name::NONE,
                                on: CutOn::Edge(sides[dst], e2),
                                s: s2,
                                point: p,
                                tol,
                                touch: false,
                            });
                        }
                    }
                    for h in hits {
                        if let Some((s2, tol)) = on_edge(db, e2, h.point) {
                            cuts.push(Cut {
                                pair: Name::NONE,
                                on: CutOn::Edge(sides[dst], e2),
                                s: s2,
                                point: h.point,
                                tol,
                                touch: false,
                            });
                            cuts.push(Cut {
                                pair: Name::NONE,
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
                // a sample on the boundary may only touch it there (a circle tangent to an edge), so
                // the stretch runs along it only if it is on at others too
                let at =
                    |f: f64| index[dst].point_in_face(dst_face, edge.curve.eval(w[0] + (w[1] - w[0]) * f));
                // (an edge already found along one of the face's is on it)
                let lies_along = |a: &Along| {
                    if src == 0 {
                        a.0 == sides[src] && a.1 == e
                    } else {
                        a.2 == sides[src] && a.3 == e
                    }
                };
                let place = match at(0.5) {
                    Where::On if !along.iter().any(lies_along) => match at(0.381_966_011_250_105_1) {
                        Where::On => at(0.618_033_988_749_894_9),
                        o => o,
                    },
                    o => o,
                };
                match place {
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
    let mut out: Vec<CurveHit> = Vec::new();
    let mut last: Option<(f64, f64)> = None;
    // how far the curve has gone from the surface since the last crossing
    let mut deep = 0.0_f64;
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
            // in and out again by less than a tolerance: still a touch
            if !out.is_empty() && deep <= tol::LINEAR {
                out.pop();
            } else {
                out.push(CurveHit {
                    t,
                    point: curve.eval(t),
                    contact: Contact::Crossing,
                });
            }
            deep = 0.0;
        }
        deep = deep.max(g1.abs());
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
                    pair: Name::NONE,
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

/// An edge and another body's edge on one carrier.
pub(crate) type Along = (Side, EdgeId, Side, EdgeId);

/// A cut in the middle of an edge is a cut of the edge lying along it too, or the two come out in
/// different pieces and neither finds its partner. (A curve in the faces' common surface can cut
/// one of them alone, on a neighbour's box where the neighbour itself does not reach.)
fn share_cuts(bodies: &[&Body; 2], along: &mut Vec<Along>, cuts: &mut Vec<Cut>) {
    if along.is_empty() {
        return;
    }
    along.sort_unstable();
    along.dedup();
    for i in 0..cuts.len() {
        let Some((sd, e)) = cuts[i].on.edge() else {
            continue;
        };
        for &(a, ea, b, eb) in along.iter() {
            let (to, e2) = if (a, ea) == (sd, e) {
                (b, eb)
            } else if (b, eb) == (sd, e) {
                (a, ea)
            } else {
                continue;
            };
            let c = &cuts[i];
            // at an end of either there is nothing to split: `coincide` has that cut already
            let at_end = |b: &Body, e: EdgeId| {
                let edge = b.edge(e);
                [edge.start, edge.end].into_iter().any(|v| {
                    let vx = b.vertex(v);
                    vx.point.distance(c.point) <= vx.tol.max(c.tol)
                })
            };
            if at_end(bodies[sd], e) || at_end(bodies[to], e2) {
                continue;
            }
            if let Some((s, tol)) = on_edge(bodies[to], e2, c.point) {
                let cut = Cut {
                    on: CutOn::Edge(to, e2),
                    s,
                    tol,
                    point: c.point,
                    touch: c.touch,
                    pair: c.pair,
                };
                cuts.push(cut);
            }
        }
    }
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
    let mut chained = false;
    for (i, c) in cuts.iter().enumerate() {
        let mut found = None;
        for (k, &h) in heads.iter().enumerate() {
            let (d, reach) = (cuts[h].point.distance(c.point), cuts[h].tol.max(c.tol));
            if d <= reach {
                found = Some(k);
                break;
            }
            chained |= d <= 2.0 * reach;
        }
        match found {
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
    // "within tolerance" chains: a cut near two heads is one place with them, or a vertex near both
    // would end up in two. Only heads within two tolerances can have one
    for o in (1..if chained { heads.len() } else { 1 }).rev() {
        let (h, t) = (cuts[heads[o]].point, cuts[heads[o]].tol);
        for k in 0..o {
            let reach = t.max(cuts[heads[k]].tol);
            if h.distance(cuts[heads[k]].point) <= 2.0 * reach
                && (0..cuts.len()).any(|i| {
                    (cluster[i] == k || cluster[i] == o)
                        && cuts[i].point.distance(h) <= reach
                        && cuts[i].point.distance(cuts[heads[k]].point) <= reach
                })
            {
                for x in cluster.iter_mut() {
                    if *x == o {
                        *x = k;
                    } else if *x > o {
                        *x -= 1;
                    }
                }
                heads.remove(o);
                break;
            }
        }
    }
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
            alias.push((o, key));
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

/// Whether the way on from half-edge `i` (`(c, j)`, the way just clockwise of `theta`) rests
/// on ways leaving a vertex along one tangent (a curve touching an edge, or a traced curve's tangent where the surfaces touch and its branches
/// cross, which is unreliable): those are told apart again by chords a short
/// length along each, told apart by how each bends (at a pole or apex, where there is no
/// chord, by the tangent). Ordering by the first direction that differs is what a
/// second-order contact needs (the curves agree to first order); chords stand in for curvature,
/// which Ye and Maekawa's formulas give but which are numerically unreliable near a singular
/// point (Patrikalakis, Maekawa and Cho, ch. 5).
fn tied(
    pieces: &[Piece],
    half: &[(usize, bool)],
    out: &[(VKey, usize)],
    i: usize,
    theta: Vec2,
    (c, j): (Vec2, usize),
    ways: impl Iterator<Item = R<(Vec2, usize)>>,
) -> R<bool> {
    // (ways of one body's own edges along one tangent are its own smooth joins, as they always
    // were; only a way of a new edge, or of the other body's, makes a tie; nor is the way
    // straight back along the same piece. Where every way is one body's own edge, the vertex is a
    // pinch where two of its circles touch: the way on and the way through differ by how they bend)
    let near = |a: Vec2, b: Vec2| {
        let (d, x) = (a.dot(b), a.x * b.y - a.y * b.x);
        d > 0.0 && x * x < 0.002 * d * d
    };
    let side = |x: usize| pieces[half[x].0].side;
    let pinch = || out.iter().all(|&(_, k)| side(k).is_some() && side(k) == side(i));
    let mixed = |x: usize, y: usize| {
        half[x].0 != half[y].0 && (side(x).is_none() || side(y).is_none() || side(x) != side(y) || pinch())
    };
    for w in ways {
        let (a, k) = w?;
        if (near(a, theta) && mixed(k, i)) || (k != j && near(a, c) && mixed(k, j)) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The ways `tied` found tied, told apart by chords.
#[cold]
fn by_chords(
    body: &Body,
    f: FaceId,
    pieces: &[Piece],
    half: &[(usize, bool)],
    i: usize,
    mut theta: Vec2,
    ways: &mut [(Vec2, usize)],
) -> R<usize> {
    let chord = |h: (usize, bool), at_end: bool| {
        uv_chord(body, f, &pieces[h.0], h.1, at_end).and_then(|c| c.normalize())
    };
    if let Some(t) = chord(half[i], true) {
        theta = t;
    }
    for w in ways.iter_mut() {
        if let Some(c) = chord(half[w.1], false) {
            w.0 = c;
        }
    }
    Ok(clockwise(theta, ways.iter().map(|&w| Ok(w)))?.map_or(i, |w| w.1))
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
    if let Surface::Cone(_) = face.surface
        && sd.du.dot(sd.du) <= 1e-18 * sd.dv.dot(sd.dv)
    {
        // an apex has no tangent plane: its rulings are told by their u
        return round_point(face, p, t, uv, forward != at_end);
    }
    if sd.du.dot(sd.du) <= 1e-18 * sd.dv.dot(sd.dv) {
        // at a pole u says nothing: the ways are told apart in the tangent plane, turned as the
        // parameter plane is (its normal du x dv, taken a hair off the pole)
        let v = face.surface.domain().v;
        let step = if uv.y > 0.5 * (v.lo + v.hi) { -1e-6 } else { 1e-6 } * (v.hi - v.lo);
        let n = face
            .surface
            .normal(Point2::new(uv.x, uv.y + step))
            .ok_or(BoolError::Unsupported("split through a singular point"))?
            .get();
        let helper = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let e1 = (helper - n * helper.dot(n))
            .normalize()
            .ok_or(BoolError::Unsupported("split through a singular point"))?
            .get();
        let (a, b) = (dir.dot(e1), dir.dot(n.cross(e1)));
        return Ok(Vec2::new(if face.same_sense { a } else { -a }, b));
    }
    // least squares for (a, b) with a du + b dv = dir
    let (uu, uv2, vv) = (sd.du.dot(sd.du), sd.du.dot(sd.dv), sd.dv.dot(sd.dv));
    let (ru, rv) = (sd.du.dot(dir), sd.dv.dot(dir));
    let Some((a, b)) = forge_math::linalg::solve2(uu, uv2, uv2, vv, ru, rv) else {
        return round_point(face, p, t, uv, forward != at_end);
    };
    let a = if face.same_sense { a } else { -a };
    Ok(Vec2::new(a, b))
}

/// `uv_dir` at a cone's apex or a sphere's pole, where every way leaves along v: a way is told
/// by its u, the angle round the point, in the chart r (cos u, sin u) where r is the distance
/// from it (turned over where v grows away from it, as (u, v) runs the other way round).
#[cold]
fn round_point(face: &forge_topo::Face, p: &Piece, t: f64, uv: Point2, from_lo: bool) -> R<Vec2> {
    if !matches!(face.surface, Surface::Cone(_) | Surface::Sphere(_)) {
        return Err(BoolError::Unsupported("split through a singular point"));
    }
    let step = 1e-3 * p.range.width();
    let near = face
        .surface
        .project(p.curve.eval(if from_lo { t + step } else { t - step }));
    let (sin, cos) = forge_math::dmath::sin_cos(near.x);
    let flip = (near.y > uv.y) == face.same_sense;
    Ok(Vec2::new(cos, if flip { -sin } else { sin }))
}

struct LocalSplit<'a> {
    body: &'a Body,
    f: FaceId,
    op: Op,
    /// A's index and B's.
    index: [&'a Index<'a>; 2],
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
            .coedges()
            .any(|co| cut(body.coedge(co).edge) || at_a.binary_search(&body.coedge_start(co)).is_ok())
    };
    let loops = &body.face(c.f).loops;
    if hit(loops[0]) {
        return None;
    }
    let class = match c.index[1].classify(face_point(body, c.f)?) {
        Where::Inside => Class::In,
        Where::Outside => Class::Out,
        Where::On => return None,
    };
    let kept_as_is = match c.op {
        Op::Union | Op::Subtract => class == Class::Out,
        Op::Intersect => class == Class::In,
    };
    // a touched loop has a cut edge, or a vertex a new edge ends on: within B's box either way
    // (the vertex within its tolerance)
    let reach = at_a.iter().fold(tol::LINEAR, |r, &v| r.max(body.vertex(v).tol));
    let mut near: Vec<forge_topo::LoopId> = c.index[0]
        .coedges_meeting(c.f, &c.index[1].bbox.grow(reach + tol::LINEAR))
        .into_iter()
        .map(|co| body.coedge(co).lp)
        .collect();
    near.sort_unstable();
    near.dedup();
    near.retain(|&l| hit(l));
    kept_as_is.then(|| Local {
        touched: loops[1..]
            .iter()
            .copied()
            .filter(|l| near.binary_search(l).is_ok())
            .collect(),
        class,
    })
}

/// A point on face `f` a little inside its boundary, beside the middle of its first edge.
pub(crate) fn face_point(body: &Body, f: FaceId) -> Option<Point3> {
    let face = body.face(f);
    let c = body.lp(*face.loops.first()?).coedges().next()?;
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
        let v = body.edge(body.coedge(body.lp(l).coedge(0)).edge).start;
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn split_face(
    body: &Body,
    f: FaceId,
    pieces: &[Piece],
    boundary: &[Vec<(usize, bool)>],
    new_edges: &[usize],
    outer: bool,
    // of a half-edge, whether the region on its left is surely dropped (Some(true)) or kept
    // (Some(false)): a cycle dropped by its first answer is not drawn but made a region alone
    gone: &dyn Fn((usize, bool)) -> Option<bool>,
) -> R<Split> {
    let mut half: Vec<(usize, bool)> =
        Vec::with_capacity(boundary.iter().map(|l| l.len()).sum::<usize>() + 2 * new_edges.len());
    half.extend(boundary.iter().flatten().copied());
    for &p in new_edges {
        half.push((p, true));
        half.push((p, false));
    }
    let start_of = |h: (usize, bool)| if h.1 { pieces[h.0].start } else { pieces[h.0].end };
    let end_of = |h: (usize, bool)| if h.1 { pieces[h.0].end } else { pieces[h.0].start };
    // half-edges by start vertex, for finding the ways on from a vertex
    let mut by_start: Vec<(VKey, usize)> = (0..half.len()).map(|j| (start_of(half[j]), j)).collect();
    by_start.sort();
    // each half-edge's direction at its start and at its end, worked out once (a vertex where m
    // ways meet is passed m times)
    let dirs: Vec<[std::cell::Cell<Option<Vec2>>; 2]> = (0..half.len()).map(|_| Default::default()).collect();
    let dir = |i: usize, at_end: bool| -> R<Vec2> {
        let c = &dirs[i][at_end as usize];
        if let Some(d) = c.get() {
            return Ok(d);
        }
        let d = uv_dir(body, f, &pieces[half[i].0], half[i].1, at_end)?;
        c.set(Some(d));
        Ok(d)
    };
    let next = |i: usize| -> R<usize> {
        let v = end_of(half[i]);
        let from = by_start.partition_point(|&(k, _)| k < v);
        let out = &by_start[from..from + by_start[from..].partition_point(|&(k, _)| k == v)];
        // one way on (always so at a pole, where u is undefined): no angles needed
        if let [(_, j)] = out {
            return Ok(*j);
        }
        // each way by its tangent; where two leave along one, by chords (see tied): tied looks
        // for that at every vertex (a new edge may leave an old vertex along the old arc beside
        // it), not only where a cut touched an edge
        let theta = dir(i, true)?;
        let ways = || out.iter().map(|&(_, k)| Ok((dir(k, false)?, k)));
        let (c, j) = clockwise(theta, ways())?.unwrap_or((theta, i));
        if !tied(pieces, &half, out, i, theta, (c, j), ways())? {
            return Ok(j);
        }
        let mut ways: Vec<(Vec2, usize)> = ways().collect::<R<_>>()?;
        by_chords(body, f, pieces, &half, i, theta, &mut ways)
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
        if let Some((regions, open)) = plane_split(pl, face.same_sense, walks, kind, pieces, outer)? {
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
    // a cycle not drawn exactly is sampled, unless it is surely dropped: then it is made a region
    // alone, undrawn (only where every region is wanted whole, `outer`: a local split's outlines
    // must all be drawn)
    let mut skip = vec![false; cycles.len()];
    let mut polys: Vec<Option<Vec<Point2>>> = Vec::with_capacity(cycles.len());
    // the cycles drawn exactly (their outlines have no sag)
    let mut exact: Vec<usize> = Vec::new();
    for (k, cyc) in cycles.iter().enumerate() {
        if untouched(cyc) {
            polys.push(None);
            continue;
        }
        let walk = cyc.iter().map(|&i| half[i]);
        let pts = match &face.surface {
            Surface::Cylinder(c) => cylinder_outline(c, walk, pieces),
            _ => None,
        };
        if pts.is_some() {
            exact.push(k);
        }
        // (a cycle using the face's outer loop is no hole: a hole needs drawing to find its region)
        if pts.is_none()
            && cyc.iter().any(|&i| i < outer_len)
            && cyc.iter().find_map(|&i| gone(half[i])) == Some(true)
        {
            skip[k] = true;
            polys.push(None);
            continue;
        }
        let mut pts = pts.unwrap_or_else(|| {
            let mut samples = Vec::new();
            for &i in cyc {
                piece_samples(&pieces[half[i].0], half[i].1, &mut samples);
            }
            // pole-aware drawing
            forge_topo::uv::draw_loop(body, f, &samples).0
        });
        // mirrored so every face's loops run counter-clockwise
        for p in &mut pts {
            *p = mirror(*p);
        }
        polys.push(Some(pts));
    }
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
    // each outer cycle and its region (an undrawn cycle is a region alone, in its place)
    let mut outer_of: Vec<usize> = Vec::new();
    let mut region_of: Vec<usize> = Vec::new();
    let mut drawn: Vec<Option<Vec<Point2>>> = Vec::new();
    for (k, cyc) in cycles.iter().enumerate() {
        if areas[k] > 0.0 && !skip[k] {
            outer_of.push(k);
            region_of.push(regions.len());
        }
        if areas[k] > 0.0 || skip[k] {
            regions.push(vec![cyc.iter().map(|&i| half[i]).collect()]);
            drawn.push(polys[k].clone());
        }
    }
    let outlines = std::cell::OnceCell::<Vec<Outline>>::new();
    let shifts = |period: Option<f64>| match period {
        Some(p) => ([0.0, -p, p], 3),
        None => ([0.0; 3], 1),
    };
    let ((us, nu), (vs, nv)) = (shifts(dom.u.period), shifts(dom.v.period));
    let (su, sv) = (&us[..nu], &vs[..nv]);
    // each outline's chords by 16, each run's box grown by the most its chords may sag
    const RUN: usize = 16;
    let runs = std::cell::OnceCell::<Vec<std::cell::OnceCell<Vec<[f64; 4]>>>>::new();
    let run_boxes = |p: &[Point2]| -> Vec<[f64; 4]> {
        (0..p.len())
            .step_by(RUN)
            .map(|s| {
                let mut r = [f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY];
                let mut far = 0.0f64;
                for k in s..(s + RUN).min(p.len()) {
                    let (a, c) = (p[k], p[(k + 1) % p.len()]);
                    far = far.max(0.06 * ((c.x - a.x).abs() + (c.y - a.y).abs()));
                    r = [
                        r[0].min(a.x.min(c.x)),
                        r[1].max(a.x.max(c.x)),
                        r[2].min(a.y.min(c.y)),
                        r[3].max(a.y.max(c.y)),
                    ];
                }
                [r[0] - far, r[1] + far, r[2] - far, r[3] + far]
            })
            .collect()
    };
    // how far point q lies from the chords of the outlines with a curve in them, not drawn
    // exactly, that share no piece with hole cycle `cyc`, in sags (a chord of 16 to a piece
    // bulges from its curve by under 0.06 of its length) plus `slack`; at least 1 where no such
    // chord can be drawn past it
    let off_chords = |q: Point2, cyc: &[usize], slack: f64| -> f64 {
        let mut worst = f64::INFINITY;
        let line = |j: &usize| matches!(pieces[half[*j].0].curve, Curve::Line(_));
        let shares = |j: &usize| cyc.iter().any(|&i| half[i].0 == half[*j].0);
        for &o in &outer_of {
            let (Some(poly), Some(b)) = (&polys[o], boxes[o]) else {
                continue;
            };
            let g = 0.06 * Vec2::new(b.1 - b.0, b.3 - b.2).length() + slack;
            let near = |&(du, dv): &(f64, f64)| {
                let q = Point2::new(q.x + du, q.y + dv);
                q.x >= b.0 - g && q.x <= b.1 + g && q.y >= b.2 - g && q.y <= b.3 + g
            };
            let mut at = su
                .iter()
                .flat_map(|&du| sv.iter().map(move |&dv| (du, dv)))
                .filter(near)
                .peekable();
            if at.peek().is_none()
                || exact.contains(&o)
                || cycles[o].iter().all(line)
                || cycles[o].iter().any(shares)
            {
                continue;
            }
            for (du, dv) in at {
                let q = Point2::new(q.x + du, q.y + dv);
                let ks = runs.get_or_init(|| vec![std::cell::OnceCell::new(); cycles.len()])[o]
                    .get_or_init(|| run_boxes(poly))
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| {
                        q.x >= r[0] - slack
                            && q.x <= r[1] + slack
                            && q.y >= r[2] - slack
                            && q.y <= r[3] + slack
                    });
                for k in ks.flat_map(|(j, _)| j * RUN..(j * RUN + RUN).min(poly.len())) {
                    let (a, c) = (poly[k], poly[(k + 1) % poly.len()]);
                    let ac = c - a;
                    // (a chord is no longer than its run in x and y together)
                    let far = 0.06 * (ac.x.abs() + ac.y.abs()) + slack;
                    if q.x < a.x.min(c.x) - far
                        || q.x > a.x.max(c.x) + far
                        || q.y < a.y.min(c.y) - far
                        || q.y > a.y.max(c.y) + far
                    {
                        continue;
                    }
                    let sag = 0.06 * ac.length() + slack;
                    let t = ((q - a).dot(ac) / ac.length_sq().max(f64::MIN_POSITIVE)).clamp(0.0, 1.0);
                    worst = worst.min(q.distance(a + ac * t) / sag.max(f64::MIN_POSITIVE));
                }
            }
        }
        worst
    };
    for (c, cyc) in cycles.iter().enumerate() {
        if areas[c] > 0.0 || skip[c] {
            continue;
        }
        let probe = match &polys[c] {
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
                // (no further across than a thousandth of the hole's own width: u and v scale apart
                // on a curved face, and a long segment's step in v may be a wide one in u)
                let across = (p1 - p0).perp();
                let width = if matches!(face.surface, Surface::Plane(_)) {
                    1.0
                } else {
                    let at = |p: Point2| (p - p0).dot(across);
                    let (lo, hi) = poly
                        .iter()
                        .fold((0.0f64, 0.0f64), |r, &p| (r.0.min(at(p)), r.1.max(at(p))));
                    (hi - lo) / across.length_sq().max(f64::MIN_POSITIVE)
                };
                let chord = p0.lerp(p1, 0.5) + across * (1e-3 * width.min(1.0));
                // the chord's middle may lie up to its sag off the hole's curve, and a curved
                // outline may be drawn either side of a point nearer it than its own chords' sag:
                // with such an outline that near (concentric or near-tangent loops), a point a
                // step in off one of the hole's drawn points (each on its curve) where it runs
                // smoothly, outside its outline and clearest of the others
                let curved = !exact.contains(&c)
                    && cyc
                        .iter()
                        .any(|&i| !matches!(pieces[half[i].0].curve, Curve::Line(_)));
                let own = if curved { 0.06 * (p1 - p0).length() } else { 0.0 };
                let mut best = (off_chords(chord, cyc, own), chord);
                let n = poly.len();
                let step = (n / 12).max(1);
                for i in (step / 2..n).step_by(step) {
                    if best.0 >= 1.0 {
                        break;
                    }
                    let (a, b) = (poly[i] - poly[(i + n - 1) % n], poly[(i + 1) % n] - poly[i]);
                    if a.dot(b) <= 0.9 * a.length() * b.length() {
                        continue;
                    }
                    let q = poly[i] + (a + b).perp() * 0.5e-3;
                    let s = off_chords(q, cyc, 0.0);
                    let outside = |&du: &f64| {
                        sv.iter()
                            .all(|&dv| !inside_poly(Point2::new(q.x + du, q.y + dv), poly))
                    };
                    if s > best.0 && su.iter().all(outside) {
                        best = (s, q);
                    }
                }
                best.1
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
            // its host may be a cycle not drawn: draw them all
            if skip.contains(&true) {
                return split_face(body, f, pieces, boundary, new_edges, outer, &|_| None);
            }
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
        regions[region_of[host]].push(cyc.iter().map(|&i| half[i]).collect());
    }
    Ok(Split {
        regions,
        open,
        outlines: Some(drawn),
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
    mut walks: Vec<Walk>,
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
    let mut regions: Vec<Region> = outer_of
        .iter()
        .map(|&k| vec![std::mem::take(&mut walks[k])])
        .collect();
    let boxes = std::cell::OnceCell::<Vec<(Box3, Vec<Box3>)>>::new();
    let mut open: Vec<Walk> = Vec::new();
    for k in 0..walks.len() {
        if areas[k] > 0.0 {
            continue;
        }
        let w = std::mem::take(&mut walks[k]);
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
                    let (hull, each) =
                        &boxes.get_or_init(|| regions.iter().map(|g| walk_boxes(&g[0], pieces)).collect())[r];
                    hull.contains(probe)
                        && match plane_inside(pl, probe, &regions[r][0], pieces, each) {
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
            Some(r) => regions[r].push(w),
            None if !outer => open.push(w),
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

/// Of the regions holding a point (they may nest: a ring round a disc), the innermost: the one
/// with the smallest box.
#[inline(never)]
fn innermost(held: &[bool], boxes: &[(Box3, Vec<Box3>)]) -> Option<usize> {
    let size = |r: usize| boxes[r].0.diagonal();
    (0..held.len())
        .filter(|&r| held[r])
        .min_by(|&a, &b| size(a).total_cmp(&size(b)))
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
    shared: &[usize],
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
            // a hair of the face, every point of it on the partner's boundary, lies on it too
            let w = index[other].point_in_face(pf, p);
            let on = w == Where::Inside
                || (w == Where::On
                    && spare_probes(body, s, pieces).all(|q| index[other].classify(q) == Where::On));
            if on {
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
        for &(p, fwd) in s.loops.iter().flatten() {
            let Some((_, lean)) = lean(bodies, segs, pieces, s.side, s.from, (p, fwd)) else {
                continue;
            };
            if lean.abs() <= 1e-9 {
                continue;
            }
            class[i] = Some(if lean < 0.0 { Class::In } else { Class::Out });
            break;
        }
    }
    // flood across shared native edge pieces between pieces of one body
    // an edge imprinted into the other body's face lies on that body: no flood across it
    let mut imprinted = vec![false; pieces.len()];
    for &p in specs.iter().flat_map(|s| &s.foreign) {
        imprinted[p] = true;
    }
    // nor across an edge the two bodies share: the other's face may end there, so being in it
    // can change across (a tool's edge lying on a part's edge)
    for &p in shared {
        imprinted[p] = true;
    }
    // the specs using each piece, in spec order: piece p's are users[from[p]..from[p + 1]]
    let floods = |p: usize| pieces[p].seg.is_none() && !imprinted[p];
    let mut from = vec![0u32; pieces.len() + 1];
    for &(p, _) in specs.iter().flat_map(|s| s.loops.iter().flatten()) {
        if floods(p) {
            from[p + 1] += 1;
        }
    }
    for p in 0..pieces.len() {
        from[p + 1] += from[p];
    }
    let mut users = vec![0u32; from[pieces.len()] as usize];
    let mut next = from.clone();
    for (i, s) in specs.iter().enumerate() {
        for &(p, _) in s.loops.iter().flatten() {
            if floods(p) {
                users[next[p] as usize] = i as u32;
                next[p] += 1;
            }
        }
    }
    let users = |p: usize| {
        users[from[p] as usize..from[p + 1] as usize]
            .iter()
            .map(|&j| j as usize)
    };
    let spread = |class: &mut Vec<Option<Class>>, start: usize| -> R<()> {
        let mut queue = vec![start];
        while let Some(i) = queue.pop() {
            let c = class[i].expect("queued specs are classified");
            for &(p, _) in specs[i].loops.iter().flatten() {
                for j in users(p) {
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
        let mut at = index[1 - s.side].classify(probe);
        // a probe on a line or circle where the bodies only touch says nothing: others decide
        if at == Where::On
            && let Some(w) = spare_probes(bodies[s.side], s, pieces)
                .map(|q| index[1 - s.side].classify(q))
                .find(|w| *w != Where::On)
        {
            at = w;
        }
        class[i] = Some(match at {
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

/// How the region on the left of half-edge `h` of face `f` (of body `side`) leans into the other
/// body at a new edge: that body, and the sine of the angle between the way into the region and
/// the other face's outward normal, negative inside the other body. None for an old edge or at a
/// singular point.
pub(crate) fn lean(
    bodies: &[&Body],
    segs: &[Seg],
    pieces: &[Piece],
    side: Side,
    f: FaceId,
    h: (usize, bool),
) -> Option<(Side, f64)> {
    let (p, fwd) = h;
    let sg = &segs[pieces[p].seg?];
    let k = (sg.sides[0] == side) as usize;
    let (other, of) = (sg.sides[k], sg.faces[k]);
    let (body, ob) = (bodies[side], bodies[other]);
    let piece = &pieces[p];
    let d = piece.curve.deriv(piece.range.mid());
    let along = if fwd { d.d1 } else { -d.d1 };
    let n_here = body.face_normal(f, body.face(f).surface.project(d.p))?;
    let inward = n_here.cross(along);
    let n_other = ob.face_normal(of, ob.face(of).surface.project(d.p))?;
    Some((other, inward.dot(n_other.get()) / inward.length()))
}

/// Whether the boolean keeps a piece of `side` of class `c`, and if so whether it flips.
fn keep_of(op: Op, side: Side, c: Class) -> Option<bool> {
    match (op, side, c) {
        (Op::Union, _, Class::Out) | (Op::Union, 0, Class::OnSame) => Some(false),
        (Op::Subtract, 0, Class::Out) | (Op::Subtract, 0, Class::OnOpposite) => Some(false),
        (Op::Subtract, 1, Class::In) => Some(true),
        (Op::Intersect, _, Class::In) | (Op::Intersect, 0, Class::OnSame) => Some(false),
        _ => None,
    }
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
    // (off a hair of an edge a step is too small to leave the boundary: its longest piece then)
    let first = s.loops.first()?;
    let mut h = *first.first()?;
    let mut chord = half_chord(&pieces[h.0]);
    if chord < 0.01
        && let Some((o, c)) = first
            .iter()
            .map(|o| (*o, half_chord(&pieces[o.0])))
            .find(|o| o.1 >= 0.01)
    {
        (h, chord) = (o, c);
    }
    if let Some(q) = clear_step(body, s, pieces, h, chord)? {
        return Some(q);
    }
    // (a hole a hair inside that piece, along all of it: off another)
    for &o in s.loops.iter().flatten() {
        if o != h
            && let Some(q) = clear_step(body, s, pieces, o, half_chord(&pieces[o.0]))?
        {
            return Some(q);
        }
    }
    probe(body, s.from, h, pieces)
}

/// A step off half-edge `h` no other piece of the region comes nearer than: the way there crosses none
/// of them (a sliver narrower than the usual step would put the probe past its far side).
fn clear_step(
    body: &Body,
    s: &Spec,
    pieces: &[Piece],
    h: (usize, bool),
    chord: f64,
) -> Option<Option<Point3>> {
    let mut step = (0.01 * chord).min(0.05);
    // (two tolerances clear of everything still tells inside from on)
    while step >= 2.0 * tol::LINEAR {
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
            return Some(Some(q));
        }
        step *= 0.5;
    }
    Some(None)
}

/// More points inside a region: a small step off each of its half-edges, a third and two thirds
/// along it (where no other piece of the region comes nearer than the step).
#[cold]
fn spare_probes<'a>(body: &'a Body, s: &'a Spec, pieces: &'a [Piece]) -> impl Iterator<Item = Point3> + 'a {
    s.loops.iter().flatten().flat_map(move |&h| {
        [1.0 / 3.0, 2.0 / 3.0].into_iter().filter_map(move |frac| {
            let pc = &pieces[h.0];
            let step = (0.01 * half_chord(pc)).min(0.05);
            let d = pc.curve.deriv(pc.range.lo + frac * pc.range.width());
            let face = body.face(s.from);
            let along = if h.1 { d.d1 } else { -d.d1 };
            let inward = body
                .face_normal(s.from, face.surface.project(d.p))?
                .cross(along)
                .normalize()?;
            let q = face.surface.eval(face.surface.project(d.p + inward * step));
            s.loops
                .iter()
                .flatten()
                .all(|&(o, _)| {
                    o == h.0 || {
                        let c = &pieces[o];
                        c.curve.eval(c.curve.closest(q, c.range)).distance(q) >= step
                    }
                })
                .then_some(q)
        })
    })
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

/// The box a pair's meet is sought in: the face boxes' overlap, but a torus's whole box where a
/// face is a piece of one (its tight box only says whether the pair can meet; marching a meet
/// clipped to it costs more pieces).
pub(crate) fn meet_region(ba: &Box3, bb: &Box3, sa: &Surface, sb: &Surface) -> Box3 {
    let whole = |s: &Surface, b: &Box3| match s {
        Surface::Torus(t) => Box3::from_point(t.frame.origin).grow(t.major + t.minor),
        _ => *b,
    };
    let r = overlap(&whole(sa, ba), &whole(sb, bb));
    // a straight-across face's box is its edges' hull, so its boundary reaches the box's sides:
    // a traced curve must run on past them to cross them
    let ruled = |s: &Surface| matches!(s, Surface::Procedural(p) if p.v_degree() == Some(1));
    r.grow(if ruled(sa) || ruled(sb) {
        (0.002 * r.diagonal()).max(tol::LINEAR)
    } else {
        tol::LINEAR
    })
}

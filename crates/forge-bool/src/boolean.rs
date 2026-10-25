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
use forge_math::{Frame, Interval, Point2, Point3, Vec3, dmath, tol};
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
}

type R<T> = Result<T, BoolError>;

/// A closed walk of edge pieces: (piece, forward).
type Walk = Vec<(usize, bool)>;
/// A face region: outer walk first, then holes.
type Region = Vec<Walk>;

/// Which input body: 0 is A, 1 is B.
type Side = usize;
/// Splits an edge into its pieces once (later calls give the same range).
type Ensure<'x> = dyn FnMut(Side, EdgeId, &mut Vec<Piece>) -> std::ops::Range<usize> + 'x;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum VKey {
    Old(Side, VertexId),
    New(u32),
}

/// A cut of a boundary edge by an intersection curve.
struct Cut {
    side: Side,
    edge: EdgeId,
    s: f64,
    point: Point3,
}

struct Seg {
    curve: Curve,
    range: Interval,
    /// Indices into the cuts list; None for a closed curve lying wholly inside both faces.
    ends: Option<(usize, usize)>,
    faces: [FaceId; 2],
    name: Name,
}

/// A piece of an edge in the result: part of an original edge, or a new edge.
#[derive(Clone)]
struct Piece {
    curve: Curve,
    range: Interval,
    start: VKey,
    end: VKey,
    name: Name,
    /// The new edge this piece is, if any.
    seg: Option<usize>,
    /// The original edge this piece is all of, if it was not split.
    whole: Option<(Side, EdgeId)>,
    /// The body whose edge this piece is part of (None for a new edge).
    side: Option<Side>,
}

/// A face of the result before it is built: loops of (piece, forward).
struct Spec {
    side: Side,
    from: FaceId,
    loops: Vec<Vec<(usize, bool)>>,
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
struct Imprint {
    side: Side,
    edge: EdgeId,
    range: Interval,
    into: FaceId,
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
/// as tombstones until `Body::compact`.
pub fn boolean_into(target: &mut Body, b: &Body, op: Op, feature: u64) -> R<()> {
    let plan = plan(target, b, op, feature)?;
    let mut clock = phases_on().then(std::time::Instant::now);
    apply(target, b, &plan, Name::feature(feature));
    lap(&mut clock, 6);
    Ok(())
}

/// Everything a boolean decides, worked out without touching A.
struct Plan {
    pieces: Vec<Piece>,
    specs: Vec<Spec>,
    kept: Vec<(usize, bool)>,
    points: Vec<Point3>,
    alias: Vec<(VKey, VKey)>,
    /// A's faces that might change; every other face of A comes through as it is.
    involved: Vec<FaceId>,
    /// Involved faces of A nothing touched, found wholly on the kept side of B.
    untouched_kept: Vec<FaceId>,
}

/// The pieces an edge splits into at its cuts (one piece, the whole edge, if none), pushed
/// consecutively: their indices are the returned range.
#[allow(clippy::too_many_arguments)]
fn edge_split(
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
    for (k, &(s, v)) in splits.iter().chain([(edge.range.hi, ve)].iter()).enumerate() {
        let name = if whole.is_some() {
            edge.name
        } else {
            edge.name.with_str("split").with(k as u64)
        };
        pieces.push(Piece {
            curve: edge.curve.clone(),
            range: Interval::new(at.0, s),
            start: at.1,
            end: v,
            name,
            seg: None,
            whole,
            side: Some(side),
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

/// Adds the time since `*t` to phase `k` and restarts the clock.
fn lap(t: &mut Option<std::time::Instant>, k: usize) {
    if let Some(t0) = t {
        let now = std::time::Instant::now();
        PHASES.with(|p| p.borrow_mut()[k] += (now - *t0).as_nanos() as u64);
        *t = Some(now);
    }
}

fn plan(a: &Body, b: &Body, op: Op, feature: u64) -> R<Plan> {
    let mut clock = phases_on().then(std::time::Instant::now);
    let bodies = [a, b];
    let index = [Index::new(a), Index::new(b)];
    let root = Name::feature(feature);

    // 1-2: intersect overlapping face pairs, keep pieces inside both faces
    let mut cuts: Vec<Cut> = Vec::new();
    let mut segs: Vec<Seg> = Vec::new();
    let mut coplanar: Vec<(FaceId, FaceId)> = Vec::new();
    let mut imprints: Vec<Imprint> = Vec::new();
    for fa in a.face_ids() {
        for fb in b.face_ids() {
            if !index[0].boxes[fa.index()].overlaps(&index[1].boxes[fb.index()]) {
                continue;
            }
            let (sa, sb) = (&a.face(fa).surface, &b.face(fb).surface);
            let region =
                overlap(&index[0].boxes[fa.index()], &index[1].boxes[fb.index()]).grow(10.0 * tol::LINEAR);
            let found = ssi::surfaces_in(sa, sb, region);
            if std::env::var_os("FORGE_DEBUG").is_some() {
                let what = match &found {
                    SurfaceSurface::Curves(cs) => format!("{} curves", cs.len()),
                    other => format!("{other:?}").chars().take(40).collect(),
                };
                eprintln!("  pair {fa:?} {fb:?}: {what}");
            }
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
                    &bodies,
                    &index,
                    [fa, fb],
                    curve,
                    pair_name,
                    &mut cuts,
                    &mut segs,
                    &mut imprints,
                )?;
            }
        }
    }

    lap(&mut clock, 0);
    // faces on one surface: each imprints the other's boundary
    for &(fa, fb) in &coplanar {
        imprint(&bodies, &index, 1, fb, fa, &mut cuts, &mut imprints)?;
        imprint(&bodies, &index, 0, fa, fb, &mut cuts, &mut imprints)?;
    }

    // 3: vertices for the cuts, then edge pieces
    lap(&mut clock, 1);
    let (cut_vertex, mut new_points, alias) = place_cuts(&bodies, &cuts)?;
    // cut points of one vertex differ by up to 10 LINEAR: a new vertex settles on the point
    // nearest every curve through it (alternating projections), and every curve parameter is
    // then taken from that point, so edge and curve ends land on their vertices
    let mut through: Vec<Vec<(&Curve, Interval)>> = vec![Vec::new(); new_points.len()];
    for (c, &k) in cuts.iter().zip(&cut_vertex) {
        if let VKey::New(i) = k {
            let edge = bodies[c.side].edge(c.edge);
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
    for (p, curves) in new_points.iter_mut().zip(&through) {
        if curves.is_empty() {
            continue;
        }
        for _ in 0..8 {
            let mut sum = Vec3::ZERO;
            for &(c, r) in curves {
                sum += c.eval(c.closest(*p, r)) - Point3::ORIGIN;
            }
            *p = Point3::ORIGIN + sum / curves.len() as f64;
        }
    }
    let key_point = |k: VKey| match k {
        VKey::Old(sd, v) => bodies[sd].vertex(v).point,
        VKey::New(i) => new_points[i as usize],
    };
    for (c, &k) in cuts.iter_mut().zip(&cut_vertex) {
        let edge = bodies[c.side].edge(c.edge);
        c.point = key_point(k);
        c.s = edge.curve.closest(c.point, edge.range);
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
    if std::env::var_os("FORGE_DEBUG").is_some() {
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
                "cut {i}: side {} edge {:?} s {:.4} -> {:?} at {:?}",
                c.side, c.edge, c.s, cut_vertex[i], c.point
            );
        }
    }
    let mut pieces: Vec<Piece> = Vec::new();
    let mut by_edge: Vec<(Side, EdgeId, usize)> = cuts
        .iter()
        .enumerate()
        .map(|(i, c)| (c.side, c.edge, i))
        .collect();
    by_edge.sort_unstable();
    // pieces per edge, made only for edges a spec reaches
    // looked up, never iterated, so the map's order cannot reach the result
    // each edge's pieces, split once: (first piece, count) by edge index, count 0 if not yet
    let mut edge_pieces: [Vec<(u32, u32)>; 2] = [vec![(0, 0); a.edges.len()], vec![(0, 0); b.edges.len()]];
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
        });
    }
    // closed new edges carry their own vertex at the curve start
    let mut points = new_points;
    for s in &segs {
        points.push(s.curve.eval(s.range.lo));
    }

    // A's faces that can change: crossed by new edges, using a split edge, or within B's reach
    let b_box = index[1].bbox;
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
    involved.extend(
        a.face_ids()
            .filter(|&f| index[0].boxes[f.index()].overlaps(&b_box)),
    );
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
                specs.push(Spec {
                    side,
                    from: f,
                    loops: vec![disk],
                    // the face keeps its name in place: its pieces carry the feature's too
                    name: name.with_name(root).with_str("split").with(1),
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
                let split = split_face(body, f, &pieces, &boundary, &new_edges, false)?;
                // an untouched hole inside a new region moves into it
                let face = body.face(f);
                let mut moved: Vec<(usize, forge_topo::LoopId)> = Vec::new();
                for &l in face.loops.iter().skip(1) {
                    if local.touched.contains(&l) {
                        continue;
                    }
                    let v = body.coedge_start(body.lp(l).coedges[0]);
                    let q = face.surface.project(body.vertex(v).point);
                    let q = if face.same_sense {
                        q
                    } else {
                        Point2::new(-q.x, q.y)
                    };
                    if let Some(r) = split
                        .outlines
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
                        loops,
                        name: face.name.with_name(root).with_str("split").with(k as u64),
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
            let regions = split_face(body, f, &pieces, &boundary, &new_edges, true)?.regions;
            for (k, loops) in regions.into_iter().enumerate() {
                let used: Vec<usize> = foreign
                    .iter()
                    .copied()
                    .filter(|p| loops.iter().flatten().any(|h| h.0 == *p))
                    .collect();
                specs.push(Spec {
                    side,
                    from: f,
                    loops,
                    name: body.face(f).name.with_str("split").with(k as u64),
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
                <= 10.0 * tol::LINEAR
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
    if std::env::var_os("FORGE_SPECS").is_some() {
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
    // faces of A beyond B's reach are outside B: kept by union and subtract, dropped by intersect
    if op == Op::Intersect {
        involved = a.face_ids().collect();
    }
    lap(&mut clock, 5);
    Ok(Plan {
        pieces,
        specs,
        kept,
        points,
        alias,
        involved,
        untouched_kept,
    })
}

/// Makes the planned changes to A: drops the faces that go, rebuilds the
/// faces that change, adds the faces that come from B, and regroups shells.
fn apply(a: &mut Body, b: &Body, plan: &Plan, root: Name) {
    let Plan {
        pieces,
        specs,
        kept,
        points,
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
            VKey::Old(_, id) => *b_vertex[id.index()]
                .get_or_insert_with(|| a.add_vertex(b.vertex(id).point, b.vertex(id).name)),
            VKey::New(i) => *new_vertex[i as usize].get_or_insert_with(|| {
                a.add_vertex(points[i as usize], root.with_str("vertex").with(i as u64))
            }),
        }
    };
    for &(si, flip) in kept {
        if as_is[si] {
            continue;
        }
        let s = &specs[si];
        let src = if s.side == 0 {
            a.face(s.from).clone()
        } else {
            b.face(s.from).clone()
        };
        // an extended (planar) face only gains holes inside it: its box stays
        let face = if s.extend && !flip {
            s.from
        } else {
            a.add_face(shell, src.surface, src.same_sense != flip, s.name)
        };
        if s.whole
            && !flip
            && let Some(bx) = src.bbox.get()
        {
            a.faces[face.index()].bbox.set(*bx);
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
    // shells are regrouped when the body is compacted
}

/// Cuts `curve` (on both faces' surfaces) where it crosses either face's
/// boundary and records the pieces lying inside both faces.
#[allow(clippy::too_many_arguments)]
fn trim(
    bodies: &[&Body; 2],
    index: &[Index; 2],
    faces: [FaceId; 2],
    curve: Curve,
    name: Name,
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
        let Curve::Line(line) = &curve else {
            return Err(BoolError::Unsupported("open conic intersection curve"));
        };
        let both = index[0].boxes[faces[0].index()]
            .hull(&index[1].boxes[faces[1].index()])
            .grow(1.0);
        match line_box(line, &both) {
            Some(r) => r,
            None => return Ok(()),
        }
    };
    let first = cuts.len();
    let mut ts: Vec<(f64, usize)> = Vec::new();
    // edges the curve runs along: (side, edge, is a seam)
    let mut along: Vec<(Side, EdgeId, bool)> = Vec::new();
    // an edge whose box misses the curve's cannot cut it (big faces have hundreds of edges)
    let reach = curve.bbox(range).grow(10.0 * tol::LINEAR);
    for side in 0..2 {
        let body = bodies[side];
        let f = faces[side];
        for &l in &body.face(f).loops {
            for &c in &body.lp(l).coedges {
                let e = body.coedge(c).edge;
                let edge = body.edge(e);
                if !body.edge_box(e).overlaps(&reach) {
                    continue;
                }
                if cuts[first..].iter().any(|k| k.side == side && k.edge == e) {
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
                        let other = &bodies[1 - side].face(faces[1 - side]).surface;
                        let coplanar = g.is_some_and(|g| {
                            g != f
                                && ssi::surfaces(&body.face(g).surface, other) == SurfaceSurface::Coincident
                        });
                        if !coplanar && !along.iter().any(|a| a.0 == side && a.1 == e) {
                            along.push((side, e, seam));
                        }
                        for (s, v) in [(edge.range.lo, edge.start), (edge.range.hi, edge.end)] {
                            let p = body.vertex(v).point;
                            let t = curve.closest(p, range);
                            if curve.eval(t).distance(p) <= 10.0 * tol::LINEAR {
                                ts.push((t, cuts.len()));
                                cuts.push(Cut {
                                    side,
                                    edge: e,
                                    s,
                                    point: p,
                                });
                            }
                        }
                        continue;
                    }
                };
                // a curve that touches the boundary without crossing it is not cut there; a
                // "touch" may be two crossings a hair apart (near-tangent circles), so look
                for h in hits
                    .into_iter()
                    .flat_map(|h| crossings_near(&curve, range, &across, h))
                {
                    let s = edge.curve.closest(h.point, edge.range);
                    if edge.curve.eval(s).distance(h.point) <= 10.0 * tol::LINEAR {
                        ts.push((h.t, cuts.len()));
                        cuts.push(Cut {
                            side,
                            edge: e,
                            s,
                            point: h.point,
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
        for &(side, e, _) in &along {
            let body = bodies[side];
            let edge = body.edge(e);
            let mut marks = vec![edge.range.lo, edge.range.hi];
            for k in first..cuts.len() {
                if cuts[k].side == side {
                    continue;
                }
                let p = cuts[k].point;
                let s = edge.curve.closest(p, edge.range);
                if edge.curve.eval(s).distance(p) <= 10.0 * tol::LINEAR {
                    cuts.push(Cut {
                        side,
                        edge: e,
                        s,
                        point: p,
                    });
                    marks.push(s);
                }
            }
            marks.sort_by(f64::total_cmp);
            for w in marks.windows(2) {
                if w[1] - w[0] <= 1e-12 * (1.0 + edge.range.width()) {
                    continue;
                }
                let into = faces[1 - side];
                let m = edge.curve.eval(0.5 * (w[0] + w[1]));
                if index[1 - side].point_in_face(into, m) == Where::Inside {
                    imprints.push(Imprint {
                        side,
                        edge: e,
                        range: Interval::new(w[0], w[1]),
                        into,
                    });
                }
            }
        }
        if along.iter().any(|a| a.2) {
            return Ok(());
        }
    }
    ts.sort_by(|x, y| x.0.total_cmp(&y.0));
    if std::env::var_os("FORGE_DEBUG").is_some() {
        let tsp: Vec<_> = ts.iter().map(|&(t, k)| (t, cuts[k].point)).collect();
        eprintln!("  curve {faces:?} range {range:?} cuts {tsp:?}");
    }
    // the same point found on two edges (a vertex, or both faces' boundaries) is one cut
    ts.dedup_by(|x, y| cuts[x.1].point.distance(cuts[y.1].point) <= 10.0 * tol::LINEAR);
    let inside = |t: f64| -> R<bool> {
        let p = curve.eval(t);
        let wa = index[0].point_in_face(faces[0], p);
        let wb = index[1].point_in_face(faces[1], p);
        if std::env::var_os("FORGE_DEBUG").is_some() && matches!(curve, Curve::Circle(_)) {
            eprintln!("  trim {:?} t {t:.4} p {p:?}: {wa:?} {wb:?}", faces);
        }
        // a stretch on a boundary is an existing edge, never a new one
        Ok(wa == Where::Inside && wb == Where::Inside)
    };
    if ts.is_empty() {
        if closed && inside(range.mid())? {
            segs.push(Seg {
                curve: curve.clone(),
                range,
                ends: None,
                faces,
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
        if inside(0.5 * (t0 + t1))? {
            segs.push(Seg {
                curve: curve.clone(),
                range: Interval::new(t0, t1),
                ends: Some((i0, i1)),
                faces,
                name: name.with(k as u64),
            });
        }
    }
    Ok(())
}

/// The surface that meets face `f`'s surface along the edge of coedge `c`:
/// the neighbouring face's, or for a seam (or an edge between two faces on
/// one surface, like revolve halves) the plane through the edge, across it.
fn across(body: &Body, f: FaceId, c: forge_topo::CoedgeId) -> R<Surface> {
    let edge = body.edge(body.coedge(c).edge);
    let t = edge.range.mid();
    let d = edge.curve.deriv(t);
    let s = &body.face(f).surface;
    let n = s
        .normal(s.project(d.p))
        .ok_or(BoolError::Internal("edge at a singular point"))?;
    // a surface through the edge, across this one: the edge's own plane for a conic (the
    // cylinder on a circle lying in a planar face), or the plane holding a line and the normal
    let edge_plane = || -> R<Surface> {
        Ok(match &edge.curve {
            Curve::Circle(k) if matches!(s, Surface::Plane(_)) => Surface::Cylinder(Cylinder {
                frame: k.frame,
                radius: k.radius,
            }),
            Curve::Circle(k) => Surface::Plane(Plane::new(k.frame)),
            Curve::Ellipse(e) => Surface::Plane(Plane::new(e.frame)),
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
        Some(g)
            if g != f
                && body.face(g).surface != *s
                && ssi::surfaces(&body.face(g).surface, s) != SurfaceSurface::Coincident =>
        {
            let other = &body.face(g).surface;
            // a smooth edge (the faces meet tangentially, as a rounded corner meets its
            // walls): the neighbour's surface would only touch a crossing curve there
            let smooth = other
                .normal(other.project(d.p))
                .is_some_and(|m| m.cross(n).length() <= 1e-6);
            if smooth && !matches!(edge.curve, Curve::Intersection(_)) {
                edge_plane()
            } else {
                Ok(*other)
            }
        }
        _ => edge_plane(),
    }
}

/// Imprints the boundary of `src_face` (of body `src`) onto the coplanar
/// face `dst_face` of the other body: cuts where its edges cross the
/// destination's boundary (on both edges), and records the parts lying inside.
fn imprint(
    bodies: &[&Body; 2],
    index: &[Index; 2],
    src: Side,
    src_face: FaceId,
    dst_face: FaceId,
    cuts: &mut Vec<Cut>,
    imprints: &mut Vec<Imprint>,
) -> R<()> {
    let dst = 1 - src;
    let (sb, db) = (bodies[src], bodies[dst]);
    let mut seen: Vec<EdgeId> = Vec::new();
    for &l in &sb.face(src_face).loops {
        for &c in &sb.lp(l).coedges {
            let e = sb.coedge(c).edge;
            if seen.contains(&e) {
                continue;
            }
            seen.push(e);
            let edge = sb.edge(e);
            let mut ts: Vec<f64> = Vec::new();
            for &l2 in &db.face(dst_face).loops {
                for &c2 in &db.lp(l2).coedges {
                    let e2 = db.coedge(c2).edge;
                    let edge2 = db.edge(e2);
                    let hits = match curve_surface(&edge.curve, edge.range, &across(db, dst_face, c2)?) {
                        CurveSurface::Hits(h) => h,
                        // lying in both the face's plane and the neighbour's surface, the edge runs
                        // along the same carrier as e2: cut each at the other's ends inside it
                        _ => {
                            coincide(sb, src, e, db, e2, cuts);
                            continue;
                        }
                    };
                    for h in hits {
                        let s2 = edge2.curve.closest(h.point, edge2.range);
                        if edge2.curve.eval(s2).distance(h.point) <= 10.0 * tol::LINEAR {
                            cuts.push(Cut {
                                side: dst,
                                edge: e2,
                                s: s2,
                                point: h.point,
                            });
                            cuts.push(Cut {
                                side: src,
                                edge: e,
                                s: h.t,
                                point: h.point,
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
                        side: src,
                        edge: e,
                        range: Interval::new(w[0], w[1]),
                        into: dst_face,
                    }),
                    // along the destination's boundary: the shared part of coincident edges
                    Where::On | Where::Outside => {}
                }
            }
        }
    }
    Ok(())
}

fn overlap(a: &forge_math::Box3, b: &forge_math::Box3) -> forge_math::Box3 {
    let cut = |x: Interval, y: Interval| Interval::new(x.lo.max(y.lo), x.hi.min(y.hi));
    forge_math::Box3 {
        x: cut(a.x, b.x),
        y: cut(a.y, b.y),
        z: cut(a.z, b.z),
    }
}

/// A crossing hit as it is; a touching one replaced by the crossings found where the surface's
/// distance changes sign within a small window around it (none if it truly only touches).
fn crossings_near(curve: &Curve, range: Interval, s: &Surface, h: CurveHit) -> Vec<CurveHit> {
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
fn coincide(sb: &Body, src: Side, e: EdgeId, db: &Body, e2: EdgeId, cuts: &mut Vec<Cut>) {
    let near = 10.0 * tol::LINEAR;
    let mut ends_into = |from: &Body, fe: EdgeId, to: &Body, te: EdgeId, side: Side| {
        let (f, t) = (from.edge(fe), to.edge(te));
        for v in [f.start, f.end] {
            let p = from.vertex(v).point;
            let s = t.curve.closest(p, t.range);
            if t.curve.eval(s).distance(p) <= near {
                cuts.push(Cut {
                    side,
                    edge: te,
                    s,
                    point: p,
                });
            }
        }
    };
    ends_into(sb, e, db, e2, 1 - src);
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
    let near = 10.0 * tol::LINEAR;
    let mut cluster: Vec<usize> = Vec::with_capacity(cuts.len());
    let mut heads: Vec<usize> = Vec::new();
    for (i, c) in cuts.iter().enumerate() {
        match heads
            .iter()
            .position(|&h| cuts[h].point.distance(c.point) <= near)
        {
            Some(k) => cluster.push(k),
            None => {
                cluster.push(heads.len());
                heads.push(i);
            }
        }
    }
    let end_vertex = |c: &Cut| {
        let edge = bodies[c.side].edge(c.edge);
        [edge.start, edge.end]
            .into_iter()
            .find(|&v| bodies[c.side].vertex(v).point.distance(c.point) <= near)
            .map(|v| VKey::Old(c.side, v))
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

/// Direction of a half-edge in the face's oriented parameter plane, at its
/// start (`at_end` false) or pointing back along it from its end.
fn uv_angle(body: &Body, f: FaceId, p: &Piece, forward: bool, at_end: bool) -> R<f64> {
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
    Ok(dmath::atan2(b, a))
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
fn face_point(body: &Body, f: FaceId) -> Option<Point3> {
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
fn piece_samples(p: &Piece, forward: bool, out: &mut Vec<Point3>) {
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
struct Split {
    regions: Vec<Region>,
    open: Vec<Walk>,
    outlines: Vec<Option<Vec<Point2>>>,
}

fn split_face(
    body: &Body,
    f: FaceId,
    pieces: &[Piece],
    boundary: &[Vec<(usize, bool)>],
    new_edges: &[usize],
    outer: bool,
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
        let out: Vec<usize> = by_start[from..]
            .iter()
            .take_while(|&&(k, _)| k == v)
            .map(|&(_, j)| j)
            .collect();
        // one way on (always so at a pole, where u is undefined): no angles needed
        if out.len() == 1 {
            return Ok(out[0]);
        }
        let theta = uv_angle(body, f, &pieces[half[i].0], half[i].1, true)?;
        // the outgoing half-edge just clockwise of the way back
        let mut best: Option<(f64, usize)> = None;
        let mut top: Option<(f64, usize)> = None;
        for j in out {
            let a = uv_angle(body, f, &pieces[half[j].0], half[j].1, false)?;
            if a < theta && best.is_none_or(|b| a > b.0) {
                best = Some((a, j));
            }
            if top.is_none_or(|b| a > b.0) {
                top = Some((a, j));
            }
        }
        Ok(best.or(top).map_or(i, |b| b.1))
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
            if std::env::var_os("FORGE_DEBUG").is_some() {
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
            let mut samples = Vec::new();
            for &i in cyc {
                piece_samples(&pieces[half[i].0], half[i].1, &mut samples);
            }
            // pole-aware drawing, then mirrored so every face's loops run counter-clockwise
            let (pts, _) = forge_topo::uv::draw_loop(body, f, &samples);
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
            // an untouched hole touches no new edge, so its first vertex is clear of every new outline
            None => {
                let pc = &pieces[half[cyc[0]].0];
                mirror(
                    face.surface.project(pc.curve.eval(if half[cyc[0]].1 {
                        pc.range.lo
                    } else {
                        pc.range.hi
                    })),
                )
            }
        };
        // the smallest outline holding it; the untouched outer loop (no outline) holds every hole.
        // Outlines of a face spanning more than a period can't all share one window, so the
        // probe is tried a period either way too
        let shifts = |period: Option<f64>| match period {
            Some(p) => [0.0, -p, p],
            None => [0.0; 3],
        };
        let holds = |poly: &Vec<Point2>| {
            shifts(dom.u.period).iter().any(|&du| {
                shifts(dom.v.period)
                    .iter()
                    .any(|&dv| inside_poly(Point2::new(probe.x + du, probe.y + dv), poly))
            })
        };
        let host = outer_of
            .iter()
            .enumerate()
            .filter(|&(_, &o)| polys[o].as_ref().is_none_or(holds))
            .min_by(|x, y| areas[*x.1].total_cmp(&areas[*y.1]))
            .map(|(r, _)| r);
        let Some(host) = host else {
            if !outer {
                open.push(cyc.iter().map(|&i| half[i]).collect());
                continue;
            }
            if std::env::var_os("FORGE_DEBUG").is_some() {
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
        outlines,
    })
}

fn inside_poly(p: Point2, poly: &[Point2]) -> bool {
    let mut c = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            c = !c;
        }
    }
    c
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
        if std::env::var_os("FORGE_SPECS").is_some() {
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
                            if std::env::var_os("FORGE_SPECS").is_some() {
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
                            if std::env::var_os("FORGE_DEBUG").is_some() {
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
        if std::env::var_os("FORGE_SPECS").is_some() {
            eprintln!(
                "  RAY spec {i} probe {probe:?} -> {:?}",
                index[1 - s.side].classify(probe)
            );
        }
        class[i] = Some(match index[1 - s.side].classify(probe) {
            Where::Inside => Class::In,
            Where::Outside => Class::Out,
            Where::On => {
                if std::env::var_os("FORGE_DEBUG").is_some() {
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
    let face = body.face(s.from);
    let &(p, fwd) = s.loops.first()?.first()?;
    let piece = &pieces[p];
    let t = piece.range.mid();
    let d = piece.curve.deriv(t);
    let along = if fwd { d.d1 } else { -d.d1 };
    let n = body.face_normal(s.from, face.surface.project(d.p))?;
    let inward = n.cross(along).normalize()?;
    // half chords: a closed piece (a whole circle) has no chord end to end
    let at = |t: f64| piece.curve.eval(t);
    let len = at(piece.range.lo)
        .distance(d.p)
        .max(d.p.distance(at(piece.range.hi)))
        .max(1e-3);
    let q = d.p + inward * (0.01 * len).min(0.05);
    Some(face.surface.eval(face.surface.project(q)))
}

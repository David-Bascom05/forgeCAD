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
use forge_geom::intersect::{Contact, CurveSurface, SurfaceSurface, curve_surface};
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
    /// The face keeps all its loops and gains these (one closed new edge crossed
    /// it and touched nothing else): kept, it is extended in place, not rebuilt.
    extend: bool,
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
    apply(target, b, &plan, Name::feature(feature));
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
}

/// The pieces an edge splits into at its cuts (one piece, the whole edge, if none).
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
) -> Vec<usize> {
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
    let mut out = Vec::with_capacity(splits.len() + 1);
    let mut at = (edge.range.lo, vs);
    for (k, &(s, v)) in splits.iter().chain([(edge.range.hi, ve)].iter()).enumerate() {
        let name = if whole.is_some() {
            edge.name
        } else {
            edge.name.with_str("split").with(k as u64)
        };
        out.push(pieces.len());
        pieces.push(Piece {
            curve: edge.curve.clone(),
            range: Interval::new(at.0, s),
            start: at.1,
            end: v,
            name,
            seg: None,
            whole,
        });
        at = (s, v);
    }
    out
}

const FAST_EXTEND: bool = false;

fn plan(a: &Body, b: &Body, op: Op, feature: u64) -> R<Plan> {
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

    // faces on one surface: each imprints the other's boundary
    for &(fa, fb) in &coplanar {
        imprint(&bodies, &index, 1, fb, fa, &mut cuts, &mut imprints)?;
        imprint(&bodies, &index, 0, fa, fb, &mut cuts, &mut imprints)?;
    }

    // 3: vertices for the cuts, then edge pieces
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
    let mut edge_pieces: std::collections::HashMap<(Side, EdgeId), Vec<usize>> =
        std::collections::HashMap::new();
    let mut ensure = |side: Side, e: EdgeId, pieces: &mut Vec<Piece>| -> Vec<usize> {
        edge_pieces
            .entry((side, e))
            .or_insert_with(|| {
                edge_split(
                    bodies[side],
                    side,
                    e,
                    &cuts,
                    &by_edge,
                    &cut_vertex,
                    &alias,
                    pieces,
                )
            })
            .clone()
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

    // 4: split faces
    let mut specs: Vec<Spec> = Vec::new();
    for (side, &body) in bodies.iter().enumerate() {
        let faces: Vec<FaceId> = if side == 0 {
            involved.clone()
        } else {
            body.face_ids().collect()
        };
        for f in faces {
            let mine: Vec<usize> = (0..segs.len()).filter(|&i| segs[i].faces[side] == f).collect();
            // one closed new edge inside a face of A, its boundary untouched: the face just
            // gains a hole loop (and a disk inside it), no need to piece its whole boundary
            if FAST_EXTEND
                && side == 0
                && mine.len() == 1
                && segs[mine[0]].ends.is_none()
                && split_faces.binary_search(&f).is_err()
                && !imprints.iter().any(|im| im.side == 1 && im.into == f)
            {
                let sp = seg_piece[mine[0]];
                let (disk, hole) = if encloses_left(body, f, &pieces[sp]) {
                    ((sp, true), (sp, false))
                } else {
                    ((sp, false), (sp, true))
                };
                let name = body.face(f).name;
                specs.push(Spec {
                    side,
                    from: f,
                    loops: vec![vec![hole]],
                    name,
                    foreign: Vec::new(),
                    whole: false,
                    extend: true,
                });
                specs.push(Spec {
                    side,
                    from: f,
                    loops: vec![vec![disk]],
                    name: name.with_str("split").with(1),
                    foreign: Vec::new(),
                    whole: false,
                    extend: false,
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
            let mut whole = true;
            let mut boundary: Vec<Vec<(usize, bool)>> = Vec::with_capacity(body.face(f).loops.len());
            for &l in &body.face(f).loops {
                let mut walk = Vec::with_capacity(body.lp(l).coedges.len());
                for &c in &body.lp(l).coedges {
                    let co = body.coedge(c);
                    let ps = ensure(side, co.edge, &mut pieces);
                    whole &= ps.len() == 1;
                    if co.forward {
                        walk.extend(ps.iter().map(|&p| (p, true)));
                    } else {
                        walk.extend(ps.iter().rev().map(|&p| (p, false)));
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
                });
                continue;
            }
            let mut new_edges: Vec<usize> = mine.iter().map(|&i| seg_piece[i]).collect();
            new_edges.extend(foreign.iter().copied());
            let regions = split_face(body, f, &pieces, &boundary, &new_edges)?;
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
                });
            }
        }
    }

    // 5: classify
    let class = classify_specs(&bodies, &index, &specs, &pieces, &segs, &coplanar)?;

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
    Ok(Plan {
        pieces,
        specs,
        kept,
        points,
        alias,
        involved,
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
    staying.sort_unstable();
    staying.dedup();
    for &f in involved {
        if staying.binary_search(&f).is_err() {
            a.kill_face(f);
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
        let face = if s.extend && !flip {
            a.faces[s.from.index()].bbox = Default::default();
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
            a.add_loop(face, &uses);
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
    let mut seams: Vec<(Side, EdgeId)> = Vec::new();
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
                        if body.partner(c).map(|o| body.coedge_face(o)) == Some(f) {
                            seams.push((side, e));
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
                // a curve that touches the boundary without crossing it is not cut there
                for h in hits.into_iter().filter(|h| h.contact == Contact::Crossing) {
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
    // along a seam the curve crosses the other face on an edge that already exists: the seam
    // is cut where it leaves that face and imprinted into it, and the curve adds no edge
    if !seams.is_empty() {
        for (side, e) in seams {
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
        return Ok(());
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

/// Whether walking a closed piece forward goes counter-clockwise around the
/// region it encloses (on its left, seen from the face's outside).
fn encloses_left(body: &Body, f: FaceId, p: &Piece) -> bool {
    let mut samples = Vec::with_capacity(16);
    piece_samples(p, true, &mut samples);
    let (pts, _) = forge_topo::uv::draw_loop(body, f, &samples);
    let area = forge_topo::uv::signed_area(&pts);
    (area > 0.0) == body.face(f).same_sense
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
fn split_face(
    body: &Body,
    f: FaceId,
    pieces: &[Piece],
    boundary: &[Vec<(usize, bool)>],
    new_edges: &[usize],
) -> R<Vec<Region>> {
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
            return Err(BoolError::Internal("face split crossed itself"));
        }
        cycles.push(cyc);
    }
    let face = body.face(f);
    let nb = boundary.iter().map(|l| l.len()).sum::<usize>();
    let outer_len = boundary.first().map_or(0, |l| l.len());
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
            // a point just off the hole's first segment, on its material side (the left)
            Some(poly) => {
                let (p0, p1) = (poly[0], poly[1]);
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
        // the smallest outline holding it; the untouched outer loop (no outline) holds every hole
        let host = outer_of
            .iter()
            .enumerate()
            .filter(|&(_, &o)| polys[o].as_ref().is_none_or(|poly| inside_poly(probe, poly)))
            .min_by(|x, y| areas[*x.1].total_cmp(&areas[*y.1]))
            .map(|(r, _)| r);
        let Some(host) = host else {
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
    Ok(regions)
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
    let mut class: Vec<Option<Class>> = vec![None; n];
    let mut fixed = vec![false; n];
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

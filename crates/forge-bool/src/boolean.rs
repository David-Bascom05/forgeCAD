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
use forge_geom::intersect::{CurveSurface, SurfaceSurface, curve_surface};
use forge_geom::{Curve, CurveGeom, Plane, Surface, SurfaceGeom};
use forge_math::{Frame, Interval, Point2, Point3, dmath, tol};
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
    let bodies = [a, b];
    let index = [Index::new(a), Index::new(b)];
    let root = Name::feature(feature);

    // 1-2: intersect overlapping face pairs, keep pieces inside both faces
    let mut cuts: Vec<Cut> = Vec::new();
    let mut segs: Vec<Seg> = Vec::new();
    let mut coplanar: Vec<(FaceId, FaceId)> = Vec::new();
    for fa in a.face_ids() {
        for fb in b.face_ids() {
            if !index[0].boxes[fa.index()].overlaps(&index[1].boxes[fb.index()]) {
                continue;
            }
            let (sa, sb) = (&a.face(fa).surface, &b.face(fb).surface);
            // traced pairs get margin around the box overlap: tracing in exactly the
            // overlap lost branch pieces in the torture part (cause not pinned down yet)
            let region = overlap(&index[0].boxes[fa.index()], &index[1].boxes[fb.index()]).grow(1.0);
            let curves = match ssi::surfaces_in(sa, sb, region) {
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
            for (ci, (curve, _)) in curves.into_iter().enumerate() {
                let pair_name = root
                    .with_str("cut")
                    .with_set(vec![a.face(fa).name, b.face(fb).name])
                    .with(ci as u64);
                trim(&bodies, &index, [fa, fb], curve, pair_name, &mut cuts, &mut segs)?;
            }
        }
    }

    // faces on one surface: each imprints the other's boundary
    let mut imprints: Vec<Imprint> = Vec::new();
    for &(fa, fb) in &coplanar {
        imprint(&bodies, &index, 1, fb, fa, &mut cuts, &mut imprints)?;
        imprint(&bodies, &index, 0, fa, fb, &mut cuts, &mut imprints)?;
    }

    // 3: vertices for the cuts, then edge pieces
    let (cut_vertex, new_points, alias) = place_cuts(&bodies, &cuts)?;
    let resolve = |v: VKey| alias.iter().find(|(f, _)| *f == v).map_or(v, |&(_, t)| t);
    let mut pieces: Vec<Piece> = Vec::new();
    let mut edge_pieces: [Vec<Vec<usize>>; 2] =
        [vec![Vec::new(); a.edges.len()], vec![Vec::new(); b.edges.len()]];
    // cuts grouped by edge
    let mut by_edge: Vec<(Side, EdgeId, usize)> = cuts
        .iter()
        .enumerate()
        .map(|(i, c)| (c.side, c.edge, i))
        .collect();
    by_edge.sort_unstable();
    for side in 0..2 {
        let body = bodies[side];
        for e in body.edge_ids() {
            let edge = body.edge(e);
            let from = by_edge.partition_point(|&(sd, ed, _)| (sd, ed) < (side, e));
            let mut splits: Vec<(f64, VKey)> = by_edge[from..]
                .iter()
                .take_while(|&&(sd, ed, _)| sd == side && ed == e)
                .map(|&(_, _, i)| (cuts[i].s, cut_vertex[i]))
                .collect();
            splits.sort_by(|x, y| x.0.total_cmp(&y.0));
            splits.dedup_by(|x, y| x.1 == y.1);
            let (vs, ve) = (
                resolve(VKey::Old(side, edge.start)),
                resolve(VKey::Old(side, edge.end)),
            );
            splits.retain(|&(_, v)| v != vs && v != ve);
            let mut at = (edge.range.lo, vs);
            for (k, &(s, v)) in splits.iter().chain([(edge.range.hi, ve)].iter()).enumerate() {
                let name = if splits.is_empty() {
                    edge.name
                } else {
                    edge.name.with_str("split").with(k as u64)
                };
                edge_pieces[side][e.index()].push(pieces.len());
                pieces.push(Piece {
                    curve: edge.curve.clone(),
                    range: Interval::new(at.0, s),
                    start: at.1,
                    end: v,
                    name,
                    seg: None,
                });
                at = (s, v);
            }
        }
    }
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
        });
    }
    // closed new edges carry their own vertex at the curve start
    let mut points = new_points;
    for s in &segs {
        points.push(s.curve.eval(s.range.lo));
    }

    // 4: split faces
    let mut specs: Vec<Spec> = Vec::new();
    for side in 0..2 {
        let body = bodies[side];
        for f in body.face_ids() {
            let mine: Vec<usize> = (0..segs.len()).filter(|&i| segs[i].faces[side] == f).collect();
            let mut foreign: Vec<usize> = Vec::new();
            for im in imprints.iter().filter(|im| im.side != side && im.into == f) {
                for &p in &edge_pieces[im.side][im.edge.index()] {
                    let m = pieces[p].range.mid();
                    if im.range.contains(m) && !foreign.contains(&p) {
                        foreign.push(p);
                    }
                }
            }
            let boundary: Vec<Vec<(usize, bool)>> = body
                .face(f)
                .loops
                .iter()
                .map(|&l| {
                    body.lp(l)
                        .coedges
                        .iter()
                        .flat_map(|&c| {
                            let co = body.coedge(c);
                            let ps = &edge_pieces[side][co.edge.index()];
                            let ordered: Vec<(usize, bool)> = if co.forward {
                                ps.iter().map(|&p| (p, true)).collect()
                            } else {
                                ps.iter().rev().map(|&p| (p, false)).collect()
                            };
                            ordered
                        })
                        .collect()
                })
                .collect();
            if mine.is_empty() && foreign.is_empty() {
                let whole = body
                    .face(f)
                    .loops
                    .iter()
                    .flat_map(|&l| body.lp(l).coedges.iter())
                    .all(|&c| edge_pieces[side][body.coedge(c).edge.index()].len() == 1);
                specs.push(Spec {
                    side,
                    from: f,
                    loops: boundary,
                    name: body.face(f).name,
                    foreign,
                    whole,
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
                });
            }
        }
    }

    // 5: classify
    let class = classify_specs(&bodies, &index, &specs, &pieces, &segs, &coplanar)?;

    // 6: keep, flip, sew. A shared same-facing region keeps A's copy only.
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
    assemble(&bodies, &specs, &kept, &pieces, &points, &alias, root)
}

/// Cuts `curve` (on both faces' surfaces) where it crosses either face's
/// boundary and records the pieces lying inside both faces.
fn trim(
    bodies: &[&Body; 2],
    index: &[Index; 2],
    faces: [FaceId; 2],
    curve: Curve,
    name: Name,
    cuts: &mut Vec<Cut>,
    segs: &mut Vec<Seg>,
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
    // an edge whose box misses the curve's cannot cut it (big faces have hundreds of edges)
    let reach = curve.bbox(range).grow(10.0 * tol::LINEAR);
    for side in 0..2 {
        let body = bodies[side];
        let f = faces[side];
        for &l in &body.face(f).loops {
            for &c in &body.lp(l).coedges {
                let e = body.coedge(c).edge;
                let edge = body.edge(e);
                if !edge.curve.bbox(edge.range).overlaps(&reach) {
                    continue;
                }
                if cuts[first..].iter().any(|k| k.side == side && k.edge == e) {
                    continue; // a seam is visited twice
                }
                let across = across(body, f, c)?;
                let hits = match curve_surface(&curve, range, &across) {
                    CurveSurface::Hits(h) => h,
                    // the curve is this edge, between f and a face on the other face's surface:
                    // that coplanar contact is the imprinting's job, and the curve adds nothing
                    // the curve runs along this edge, which borders a face on the other face's
                    // surface (a coplanar contact the imprinting handles): cut the curve at the
                    // edge's ends; the stretch along the edge is boundary, not a new edge
                    CurveSurface::OnSurface
                        if ssi::surfaces(&across, &bodies[1 - side].face(faces[1 - side]).surface)
                            == SurfaceSurface::Coincident =>
                    {
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
                    CurveSurface::OnSurface | CurveSurface::Overlap => {
                        return Err(BoolError::Unsupported("intersection curve runs along an edge"));
                    }
                };
                for h in hits {
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
    ts.sort_by(|x, y| x.0.total_cmp(&y.0));
    // the same point found on two edges (a vertex, or both faces' boundaries) is one cut
    ts.dedup_by(|x, y| cuts[x.1].point.distance(cuts[y.1].point) <= 10.0 * tol::LINEAR);
    let inside = |t: f64| -> R<bool> {
        let p = curve.eval(t);
        let wa = index[0].point_in_face(faces[0], p);
        let wb = index[1].point_in_face(faces[1], p);
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
    Ok(match body.partner(c).map(|o| body.coedge_face(o)) {
        Some(g) if g != f && body.face(g).surface != body.face(f).surface => body.face(g).surface,
        _ => {
            let t = edge.range.mid();
            let d = edge.curve.deriv(t);
            let s = &body.face(f).surface;
            let n = s
                .normal(s.project(d.p))
                .ok_or(BoolError::Internal("seam at a singular point"))?;
            let z =
                d.d1.cross(n.get())
                    .normalize()
                    .ok_or(BoolError::Internal("degenerate seam"))?;
            Surface::Plane(Plane::new(Frame::from_z(d.p, z)))
        }
    })
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
                        _ => return Err(BoolError::Unsupported("coplanar faces share an edge")),
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
                    Where::On => return Err(BoolError::Unsupported("coplanar faces share an edge")),
                    Where::Outside => {}
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
    let polys: Vec<Option<Vec<Point2>>> = cycles
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
            .map(|(r, _)| r)
            .ok_or(BoolError::Internal("hole outside every region"))?;
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
    let mut users: Vec<Vec<usize>> = vec![Vec::new(); pieces.len()];
    for (i, s) in specs.iter().enumerate() {
        for &(p, _) in s.loops.iter().flatten() {
            if pieces[p].seg.is_none() && !s.foreign.contains(&p) {
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
            Where::On => return Err(BoolError::Unsupported("untouched face lies on the other body")),
        });
        spread(&mut class, i)?;
    }
    Ok(class
        .into_iter()
        .map(|c| c.expect("every spec classified"))
        .collect())
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
    let len = piece
        .curve
        .eval(piece.range.lo)
        .distance(piece.curve.eval(piece.range.hi))
        .max(1e-3);
    let q = d.p + inward * (0.01 * len).min(0.05);
    Some(face.surface.eval(face.surface.project(q)))
}

#[allow(clippy::too_many_arguments)]
fn assemble(
    bodies: &[&Body; 2],
    specs: &[Spec],
    kept: &[(usize, bool)],
    pieces: &[Piece],
    points: &[Point3],
    alias: &[(VKey, VKey)],
    root: Name,
) -> R<Body> {
    let mut out = Body::new();
    // shells: kept specs connected through shared pieces
    let mut parent: Vec<usize> = (0..kept.len()).collect();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let mut owner: Vec<Option<usize>> = vec![None; pieces.len()];
    let mut uses = vec![0u32; pieces.len()];
    for (k, &(si, _)) in kept.iter().enumerate() {
        for &(p, _) in specs[si].loops.iter().flatten() {
            uses[p] += 1;
            match owner[p] {
                None => owner[p] = Some(k),
                Some(o) => {
                    let (x, y) = (find(&mut parent, o), find(&mut parent, k));
                    parent[x.max(y)] = x.min(y);
                }
            }
        }
    }
    if uses.iter().any(|&u| u != 0 && u != 2) {
        return Err(BoolError::Internal("result is not closed"));
    }
    let mut shell_of = vec![usize::MAX; kept.len()];
    let mut shells = Vec::new();
    for k in 0..kept.len() {
        let r = find(&mut parent, k);
        if shell_of[r] == usize::MAX {
            shell_of[r] = shells.len();
            shells.push(out.add_shell());
        }
        shell_of[k] = shell_of[r];
    }
    // vertex of the result for each input vertex and each new point, filled as used
    let mut vmap: [Vec<Option<VertexId>>; 2] = [
        vec![None; bodies[0].vertices.len()],
        vec![None; bodies[1].vertices.len()],
    ];
    let mut new_map: Vec<Option<VertexId>> = vec![None; points.len()];
    let mut emap: Vec<Option<EdgeId>> = vec![None; pieces.len()];
    let mut vertex = |out: &mut Body, v: VKey| -> VertexId {
        let v = alias.iter().find(|(f, _)| *f == v).map_or(v, |&(_, t)| t);
        let slot = match v {
            VKey::Old(side, id) => &mut vmap[side][id.index()],
            VKey::New(i) => &mut new_map[i as usize],
        };
        if let Some(id) = *slot {
            return id;
        }
        let (p, name) = match v {
            VKey::Old(side, id) => (bodies[side].vertex(id).point, bodies[side].vertex(id).name),
            VKey::New(i) => (points[i as usize], root.with_str("vertex").with(i as u64)),
        };
        let id = out.add_vertex(p, name);
        *match v {
            VKey::Old(side, i) => &mut vmap[side][i.index()],
            VKey::New(i) => &mut new_map[i as usize],
        } = Some(id);
        id
    };
    for (k, &(si, flip)) in kept.iter().enumerate() {
        let s = &specs[si];
        let src = bodies[s.side].face(s.from);
        let face = out.add_face(shells[shell_of[k]], src.surface, src.same_sense != flip, s.name);
        if s.whole
            && let Some(b) = src.bbox.get()
        {
            out.faces[face.index()].bbox.set(*b);
        }
        for lp in &s.loops {
            let mut uses: Vec<(EdgeId, bool)> = Vec::with_capacity(lp.len());
            for &(p, fwd) in lp {
                let e = match emap[p] {
                    Some(e) => e,
                    None => {
                        let pc = &pieces[p];
                        let (a, b) = (vertex(&mut out, pc.start), vertex(&mut out, pc.end));
                        let e = out.add_edge(pc.curve.clone(), pc.range, a, b, pc.name);
                        emap[p] = Some(e);
                        e
                    }
                };
                uses.push((e, fwd));
            }
            if flip {
                uses = uses.into_iter().rev().map(|(e, f)| (e, !f)).collect();
            }
            out.add_loop(face, &uses);
        }
    }
    Ok(out)
}

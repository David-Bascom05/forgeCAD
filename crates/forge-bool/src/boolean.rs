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
}

pub fn boolean(a: &Body, b: &Body, op: Op, feature: u64) -> R<Body> {
    let bodies = [a, b];
    let index = [Index::new(a), Index::new(b)];
    let root = Name::feature(feature);

    // 1-2: intersect overlapping face pairs, keep pieces inside both faces
    let mut cuts: Vec<Cut> = Vec::new();
    let mut segs: Vec<Seg> = Vec::new();
    for fa in a.face_ids() {
        for fb in b.face_ids() {
            if !index[0].faces[fa.index()]
                .bbox
                .overlaps(&index[1].faces[fb.index()].bbox)
            {
                continue;
            }
            let (sa, sb) = (&a.face(fa).surface, &b.face(fb).surface);
            let curves = match ssi::surfaces(sa, sb) {
                SurfaceSurface::Empty | SurfaceSurface::Points(_) => continue,
                SurfaceSurface::Coincident => {
                    return Err(BoolError::Unsupported("faces on the same surface"));
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

    // 3: vertices for the cuts, then edge pieces
    let (cut_vertex, new_points) = place_cuts(&bodies, &cuts)?;
    let mut pieces: Vec<Piece> = Vec::new();
    let mut edge_pieces: [Vec<Vec<usize>>; 2] =
        [vec![Vec::new(); a.edges.len()], vec![Vec::new(); b.edges.len()]];
    for side in 0..2 {
        let body = bodies[side];
        for e in body.edge_ids() {
            let edge = body.edge(e);
            let mut splits: Vec<(f64, VKey)> = (0..cuts.len())
                .filter(|&i| cuts[i].side == side && cuts[i].edge == e)
                .map(|i| (cuts[i].s, cut_vertex[i]))
                .collect();
            splits.sort_by(|x, y| x.0.total_cmp(&y.0));
            splits.dedup_by(|x, y| x.1 == y.1);
            splits.retain(|&(_, v)| v != VKey::Old(side, edge.start) && v != VKey::Old(side, edge.end));
            let mut at = (edge.range.lo, VKey::Old(side, edge.start));
            for (k, &(s, v)) in splits
                .iter()
                .chain([(edge.range.hi, VKey::Old(side, edge.end))].iter())
                .enumerate()
            {
                let name = if splits.is_empty() {
                    edge.name
                } else {
                    edge.name.with_str("split").with(k as u64)
                };
                edge_pieces[side][e.index()].push(pieces.len());
                pieces.push(Piece {
                    curve: edge.curve,
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
            curve: s.curve,
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
            if mine.is_empty() {
                specs.push(Spec {
                    side,
                    from: f,
                    loops: boundary,
                    name: body.face(f).name,
                });
                continue;
            }
            let new_edges: Vec<usize> = mine.iter().map(|&i| seg_piece[i]).collect();
            let regions = split_face(body, f, &pieces, &boundary, &new_edges)?;
            for (k, loops) in regions.into_iter().enumerate() {
                specs.push(Spec {
                    side,
                    from: f,
                    loops,
                    name: body.face(f).name.with_str("split").with(k as u64),
                });
            }
        }
    }

    // 5: classify
    let class = classify_specs(&bodies, &index, &specs, &pieces, &segs)?;

    // 6: keep, flip, sew
    let keep = |side: Side, w: Where| match (op, side, w) {
        (Op::Union, _, Where::Outside) => Some(false),
        (Op::Subtract, 0, Where::Outside) => Some(false),
        (Op::Subtract, 1, Where::Inside) => Some(true),
        (Op::Intersect, _, Where::Inside) => Some(false),
        _ => None,
    };
    let kept: Vec<(usize, bool)> = (0..specs.len())
        .filter_map(|i| keep(specs[i].side, class[i]).map(|flip| (i, flip)))
        .collect();
    assemble(&bodies, &specs, &kept, &pieces, &points, root)
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
    let closed = curve.domain().period.is_some();
    let range = if closed {
        Interval::new(0.0, dmath::TAU)
    } else {
        let Curve::Line(line) = curve else {
            return Err(BoolError::Unsupported("open conic intersection curve"));
        };
        let both = index[0].faces[faces[0].index()]
            .bbox
            .hull(&index[1].faces[faces[1].index()].bbox)
            .grow(1.0);
        match line_box(&line, &both) {
            Some(r) => r,
            None => return Ok(()),
        }
    };
    let first = cuts.len();
    let mut ts: Vec<(f64, usize)> = Vec::new();
    for side in 0..2 {
        let body = bodies[side];
        let f = faces[side];
        for &l in &body.face(f).loops {
            for &c in &body.lp(l).coedges {
                let e = body.coedge(c).edge;
                if cuts[first..].iter().any(|k| k.side == side && k.edge == e) {
                    continue; // a seam is visited twice
                }
                let edge = body.edge(e);
                let across = match body.partner(c).map(|o| body.coedge_face(o)) {
                    Some(g) if g != f && body.face(g).surface != body.face(f).surface => body.face(g).surface,
                    // a seam, or an edge between two faces of one surface (revolve halves):
                    // the plane through the edge, across the surface
                    _ => {
                        let t = edge.range.mid();
                        let d = edge.curve.deriv(t);
                        let n = body
                            .face(f)
                            .surface
                            .normal(body.face(f).surface.project(d.p))
                            .ok_or(BoolError::Internal("seam at a singular point"))?;
                        let z =
                            d.d1.cross(n.get())
                                .normalize()
                                .ok_or(BoolError::Internal("degenerate seam"))?;
                        Surface::Plane(Plane::new(Frame::from_z(d.p, z)))
                    }
                };
                let hits = match curve_surface(&curve, range, &across) {
                    CurveSurface::Hits(h) => h,
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
        if wa == Where::On || wb == Where::On {
            return Err(BoolError::Unsupported("intersection curve runs along a boundary"));
        }
        Ok(wa == Where::Inside && wb == Where::Inside)
    };
    if ts.is_empty() {
        if closed && inside(range.mid())? {
            segs.push(Seg {
                curve,
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
            t1 += dmath::TAU;
        }
        if t1 - t0 <= 1e-12 {
            continue;
        }
        if inside(0.5 * (t0 + t1))? {
            segs.push(Seg {
                curve,
                range: Interval::new(t0, t1),
                ends: Some((i0, i1)),
                faces,
                name: name.with(k as u64),
            });
        }
    }
    Ok(())
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

/// One vertex per distinct cut point: an existing vertex when the cut is at
/// an edge's end, otherwise a new one shared by every cut at that point.
fn place_cuts(bodies: &[&Body; 2], cuts: &[Cut]) -> R<(Vec<VKey>, Vec<Point3>)> {
    let near = 10.0 * tol::LINEAR;
    let mut keys = Vec::with_capacity(cuts.len());
    let mut points: Vec<Point3> = Vec::new();
    let mut placed: Vec<(Point3, VKey)> = Vec::new();
    for c in cuts {
        let body = bodies[c.side];
        let edge = body.edge(c.edge);
        let key = if body.vertex(edge.start).point.distance(c.point) <= near {
            VKey::Old(c.side, edge.start)
        } else if body.vertex(edge.end).point.distance(c.point) <= near {
            VKey::Old(c.side, edge.end)
        } else if let Some(&(_, k)) = placed.iter().find(|(p, _)| p.distance(c.point) <= near) {
            k
        } else {
            points.push(c.point);
            VKey::New(points.len() as u32 - 1)
        };
        if placed
            .iter()
            .any(|(p, k)| p.distance(c.point) <= near && *k != key)
        {
            return Err(BoolError::Unsupported(
                "intersection passes through a vertex of the other body",
            ));
        }
        placed.push((c.point, key));
        keys.push(key);
    }
    Ok((keys, points))
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
    let next = |i: usize| -> R<usize> {
        let v = end_of(half[i]);
        let out: Vec<usize> = (0..half.len()).filter(|&j| start_of(half[j]) == v).collect();
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
    let polys: Vec<Vec<Point2>> = cycles
        .iter()
        .map(|cyc| {
            let mut samples = Vec::new();
            for &i in cyc {
                piece_samples(&pieces[half[i].0], half[i].1, &mut samples);
            }
            // pole-aware drawing, then mirrored so every face's loops run counter-clockwise
            let (pts, _) = forge_topo::uv::draw_loop(body, f, &samples);
            pts.into_iter()
                .map(|p| {
                    if face.same_sense {
                        p
                    } else {
                        Point2::new(-p.x, p.y)
                    }
                })
                .collect()
        })
        .collect();
    let areas: Vec<f64> = polys.iter().map(|p| forge_topo::uv::signed_area(p)).collect();
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
        // a point just off the hole's first segment, on its material side (the left)
        let (p0, p1) = (polys[k][0], polys[k][1]);
        let d = p1 - p0;
        let probe = p0.lerp(p1, 0.5) + d.perp() * 1e-3;
        let host = outer_of
            .iter()
            .enumerate()
            .filter(|&(_, &o)| inside_poly(probe, &polys[o]))
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

/// Inside or outside the other body for every spec.
fn classify_specs(
    bodies: &[&Body; 2],
    index: &[Index; 2],
    specs: &[Spec],
    pieces: &[Piece],
    segs: &[Seg],
) -> R<Vec<Where>> {
    let n = specs.len();
    let mut class: Vec<Option<Where>> = vec![None; n];
    // direct: lean of the piece at a new edge
    for (i, s) in specs.iter().enumerate() {
        let body = bodies[s.side];
        let other = 1 - s.side;
        for &(p, fwd) in s.loops.iter().flatten() {
            let Some(si) = pieces[p].seg else { continue };
            let piece = &pieces[p];
            let t = piece.range.mid();
            let d = piece.curve.deriv(t);
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
            class[i] = Some(if lean < 0.0 { Where::Inside } else { Where::Outside });
            break;
        }
    }
    // flood across shared original edge pieces within each side
    let mut users: Vec<Vec<usize>> = vec![Vec::new(); pieces.len()];
    for (i, s) in specs.iter().enumerate() {
        for &(p, _) in s.loops.iter().flatten() {
            if pieces[p].seg.is_none() {
                users[p].push(i);
            }
        }
    }
    let mut queue: Vec<usize> = (0..n).filter(|&i| class[i].is_some()).collect();
    while let Some(i) = queue.pop() {
        let c = class[i].expect("queued specs are classified");
        for &(p, _) in specs[i].loops.iter().flatten() {
            for &j in &users[p] {
                match class[j] {
                    None => {
                        class[j] = Some(c);
                        queue.push(j);
                    }
                    Some(cj) if cj != c => return Err(BoolError::Internal("inconsistent classification")),
                    _ => {}
                }
            }
        }
    }
    // regions no new edge reaches: one ray test each, flooded the same way
    for i in 0..n {
        if class[i].is_some() {
            continue;
        }
        let s = &specs[i];
        let probe =
            interior_point(bodies[s.side], s, pieces).ok_or(BoolError::Internal("no interior point"))?;
        let w = index[1 - s.side].classify(probe);
        if w == Where::On {
            return Err(BoolError::Unsupported("untouched face lies on the other body"));
        }
        class[i] = Some(w);
        let mut q = vec![i];
        while let Some(k) = q.pop() {
            for &(p, _) in specs[k].loops.iter().flatten() {
                for &j in &users[p] {
                    if class[j].is_none() {
                        class[j] = Some(w);
                        q.push(j);
                    }
                }
            }
        }
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

fn assemble(
    bodies: &[&Body; 2],
    specs: &[Spec],
    kept: &[(usize, bool)],
    pieces: &[Piece],
    points: &[Point3],
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
    let mut vmap: Vec<(VKey, VertexId)> = Vec::new();
    let mut emap: Vec<Option<EdgeId>> = vec![None; pieces.len()];
    let mut vertex = |out: &mut Body, v: VKey| -> VertexId {
        if let Some(&(_, id)) = vmap.iter().find(|(k, _)| *k == v) {
            return id;
        }
        let (p, name) = match v {
            VKey::Old(side, id) => (bodies[side].vertex(id).point, bodies[side].vertex(id).name),
            VKey::New(i) => (points[i as usize], root.with_str("vertex").with(i as u64)),
        };
        let id = out.add_vertex(p, name);
        vmap.push((v, id));
        id
    };
    for (k, &(si, flip)) in kept.iter().enumerate() {
        let s = &specs[si];
        let src = bodies[s.side].face(s.from);
        let face = out.add_face(shells[shell_of[k]], src.surface, src.same_sense != flip, s.name);
        for lp in &s.loops {
            let mut uses: Vec<(EdgeId, bool)> = Vec::with_capacity(lp.len());
            for &(p, fwd) in lp {
                let e = match emap[p] {
                    Some(e) => e,
                    None => {
                        let pc = &pieces[p];
                        let (a, b) = (vertex(&mut out, pc.start), vertex(&mut out, pc.end));
                        let e = out.add_edge(pc.curve, pc.range, a, b, pc.name);
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

//! Splitting faces along a surface: each face the surface crosses becomes pieces bounded by new
//! edges where it crosses; edges it crosses are cut in two; the solid is unchanged.

use crate::BodyError;
use forge_bool::{Index, Where};
use forge_geom::intersect::{Contact, CurveSurface, SurfaceSurface, curve_surface};
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::{Interval, Point2, Point3, Vec2, tol};
use forge_topo::uv::{default_segments, signed_area};
use forge_topo::{Body, EdgeId, FaceId, Name, VertexId};
use std::collections::HashMap;

/// `body` with each face named in `faces` split along `by` (the feature of id `id`).
pub(crate) fn split(body: &Body, faces: &[Name], by: &Surface, id: u64) -> Result<Body, BodyError> {
    let mut b = body.clone();
    let root = Name::feature(id).with_str("splitface");
    for (k, &name) in faces.iter().enumerate() {
        let f = b.face_by_name(name).ok_or(BodyError::NoFace { name })?;
        split_one(&mut b, f, by, root.with(k as u64))?;
    }
    Ok(b.compact())
}

/// Where the cutter crosses the face's boundary: at a vertex, or inside an edge.
#[derive(Clone, Copy, PartialEq)]
enum At {
    Vertex(VertexId),
    Edge(EdgeId, f64),
}

type Use = (EdgeId, bool);

/// A stretch of a cutter curve inside the face: the curve, its range, and the crossings it
/// runs between (none for a closed curve wholly inside).
type Chord = (usize, Interval, Option<(usize, usize)>);

fn split_one(b: &mut Body, f: FaceId, by: &Surface, root: Name) -> Result<(), BodyError> {
    let name = b.face(f).name;
    let unsupported = |w| Err(BodyError::Unsupported(w));
    let curves = match forge_bool::ssi::surfaces_in(&b.face(f).surface, by, b.face_box(f).grow(1.0)) {
        SurfaceSurface::Curves(cs) => cs.into_iter().map(|c| c.0).collect::<Vec<Curve>>(),
        SurfaceSurface::NotAnalytic => return unsupported("a cutter meeting this face"),
        _ => return Err(BodyError::NotCrossed { name }),
    };
    // a seam: an edge the face runs along both ways
    let mut seams: Vec<EdgeId> = Vec::new();
    {
        let mut seen = std::collections::HashSet::new();
        for &l in &b.face(f).loops {
            for &c in &b.lp(l).coedges {
                if !seen.insert(b.coedge(c).edge) {
                    seams.push(b.coedge(c).edge);
                }
            }
        }
    }
    // where the cutter crosses the boundary
    let mut cross: Vec<(Point3, At)> = Vec::new();
    for &l in &b.face(f).loops {
        for &c in &b.lp(l).coedges {
            let e = b.coedge(c).edge;
            let edge = b.edge(e);
            let hits = match curve_surface(&edge.curve, edge.range, by) {
                CurveSurface::Hits(h) => h,
                _ => return unsupported("an edge lying on the cutter"),
            };
            for h in hits.iter().filter(|h| h.contact == Contact::Crossing) {
                let near = |v: VertexId| h.point.distance(b.vertex(v).point) <= b.vertex(v).tol + tol::LINEAR;
                let at = if near(edge.start) {
                    At::Vertex(edge.start)
                } else if near(edge.end) {
                    At::Vertex(edge.end)
                } else {
                    At::Edge(e, h.t)
                };
                let same = |x: &(Point3, At)| match (x.1, at) {
                    (At::Vertex(u), At::Vertex(v)) => u == v,
                    (At::Edge(u, s), At::Edge(v, t)) => u == v && (s - t).abs() <= 1e-9,
                    _ => false,
                };
                if !cross.iter().any(same) {
                    cross.push((h.point, at));
                }
            }
        }
    }
    // the stretches of each curve inside the face: (curve, range, start crossing, end crossing)
    let index = Index::new(b);
    let inside = |p: Point3| index.point_in_face(f, p) == Where::Inside;
    let fb = b.face_box(f).grow(1.0);
    let mut chords: Vec<Chord> = Vec::new();
    for (ci, c) in curves.iter().enumerate() {
        let d = c.domain();
        let span = match d.period {
            Some(p) => Interval::new(d.lo, d.lo + p),
            None if d.lo.is_finite() && d.hi.is_finite() => Interval::new(d.lo, d.hi),
            None => {
                let wide = Interval::new(-1e12, 1e12);
                let ts = crate::pattern::corners(&fb).map(|q| c.closest(q, wide));
                Interval::new(
                    ts.iter().copied().fold(f64::INFINITY, f64::min),
                    ts.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                )
            }
        };
        let mut on: Vec<(f64, usize)> = cross
            .iter()
            .enumerate()
            .filter_map(|(x, (p, _))| {
                let t = c.closest(*p, span);
                (c.eval(t).distance(*p) <= 10.0 * tol::LINEAR).then_some((t, x))
            })
            .collect();
        on.sort_by(|a, b| a.0.total_cmp(&b.0));
        match (d.period, on.len()) {
            (Some(_), 0) => {
                if inside(c.eval(span.mid())) {
                    chords.push((ci, span, None));
                }
            }
            // round a periodic face, through its seam
            (Some(_), 1) => match cross[on[0].1].1 {
                At::Edge(e, _) if seams.contains(&e) => {
                    chords.push((
                        ci,
                        Interval::new(on[0].0, on[0].0 + span.width()),
                        Some((on[0].1, on[0].1)),
                    ));
                }
                _ => return unsupported("a closed cut touching the boundary once"),
            },
            _ => {
                let n = on.len();
                let pairs = if d.period.is_some() {
                    n
                } else {
                    n.saturating_sub(1)
                };
                for j in 0..pairs {
                    let (a, z) = (on[j], on[(j + 1) % n]);
                    let hi = if j + 1 == n { z.0 + span.width() } else { z.0 };
                    if hi - a.0 > 1e-9 && inside(c.eval(0.5 * (a.0 + hi))) {
                        chords.push((ci, Interval::new(a.0, hi), Some((a.1, z.1))));
                    }
                }
            }
        }
    }
    if chords.is_empty() {
        return Err(BodyError::NotCrossed { name });
    }

    // a vertex at each crossing a chord ends at
    let mut vert: HashMap<usize, VertexId> = HashMap::new();
    for &(ci, range, ends) in &chords {
        let Some((s, t)) = ends else { continue };
        for (x, at) in [(s, range.lo), (t, range.hi)] {
            let gap = curves[ci].eval(at).distance(cross[x].0);
            let v = match cross[x].1 {
                At::Vertex(v) => v,
                At::Edge(e, u) => *vert.entry(x).or_insert_with(|| {
                    let p = b.edge(e).curve.eval(u);
                    b.add_tolerant_vertex(p, gap + tol::LINEAR, root.with_str("vertex").with(x as u64))
                }),
            };
            let tv = &mut b.vertices[v.index()].tol;
            *tv = tv.max(gap + tol::LINEAR);
            vert.insert(x, v);
        }
    }
    // cut the boundary edges at the new vertices, in every loop using them
    let mut cuts: HashMap<EdgeId, Vec<(f64, VertexId)>> = HashMap::new();
    for (x, &(_, at)) in cross.iter().enumerate() {
        if let (At::Edge(e, t), Some(&v)) = (at, vert.get(&x)) {
            cuts.entry(e).or_default().push((t, v));
        }
    }
    let mut cut_edges: Vec<EdgeId> = cuts.keys().copied().collect();
    cut_edges.sort();
    let mut pieces: HashMap<EdgeId, Vec<EdgeId>> = HashMap::new();
    for e in &cut_edges {
        let mut at = cuts[e].clone();
        at.sort_by(|a, b| a.0.total_cmp(&b.0));
        let edge = b.edge(*e).clone();
        let mut from = (edge.range.lo, edge.start);
        let mut out = Vec::new();
        for (k, &(t, v)) in at.iter().chain([(edge.range.hi, edge.end)].iter()).enumerate() {
            let n = edge.name.with_name(root).with_str("split").with(k as u64);
            out.push(b.add_edge(edge.curve.clone(), Interval::new(from.0, t), from.1, v, n));
            from = (t, v);
        }
        pieces.insert(*e, out);
    }
    let mut loops: Vec<forge_topo::LoopId> = cut_edges
        .iter()
        .flat_map(|e| b.edge(*e).coedges.iter().map(|&c| b.coedge(c).lp))
        .collect();
    loops.sort();
    loops.dedup();
    for l in loops {
        let face = b.lp(l).face;
        let mut uses: Vec<Use> = Vec::new();
        for &c in &b.lp(l).coedges {
            let co = b.coedge(c);
            match pieces.get(&co.edge) {
                Some(ps) if co.forward => uses.extend(ps.iter().map(|&p| (p, true))),
                Some(ps) => uses.extend(ps.iter().rev().map(|&p| (p, false))),
                None => uses.push((co.edge, co.forward)),
            }
        }
        let at = b
            .face(face)
            .loops
            .iter()
            .position(|&x| x == l)
            .expect("its face's loop");
        b.remove_loop(face, l);
        let nl = b.add_loop(face, &uses);
        let ls = &mut b.faces[face.index()].loops;
        ls.pop();
        ls.insert(at, nl);
        debug_assert_eq!(ls[at], nl);
    }
    // the new edges
    let mut leave: HashMap<VertexId, Use> = HashMap::new();
    let mut closed: Vec<EdgeId> = Vec::new();
    let mut ring: Option<EdgeId> = None;
    for (k, &(ci, range, ends)) in chords.iter().enumerate() {
        let n = root.with_str("edge").with(k as u64);
        match ends {
            Some((s, t)) => {
                let (vs, vt) = (vert[&s], vert[&t]);
                let e = b.add_edge(curves[ci].clone(), range, vs, vt, n);
                if vs == vt {
                    ring = Some(e);
                    continue;
                }
                for (v, u) in [(vs, (e, true)), (vt, (e, false))] {
                    if leave.insert(v, u).is_some() {
                        return unsupported("the cutter meeting the boundary twice at one point");
                    }
                }
            }
            None => {
                let v = b.add_vertex(
                    curves[ci].eval(range.lo),
                    root.with_str("vertex").with_str("closed").with(k as u64),
                );
                closed.push(b.add_edge(curves[ci].clone(), range, v, v, n));
            }
        }
    }
    // walk the face's boundary, turning onto a new edge wherever one leaves
    let mut boundary: Vec<Use> = Vec::new();
    // each use's successor round its loop
    let mut succ: Vec<usize> = Vec::new();
    for &l in &b.face(f).loops {
        let (first, n) = (boundary.len(), b.lp(l).coedges.len());
        for (j, &c) in b.lp(l).coedges.iter().enumerate() {
            boundary.push((b.coedge(c).edge, b.coedge(c).forward));
            succ.push(first + (j + 1) % n);
        }
    }
    let mut cycles: Vec<Vec<Use>> = match ring {
        Some(e) => round_seam(b, f, &boundary, &succ, e, &pieces, &seams)?,
        None => {
            let mut seam: Vec<EdgeId> = seams.clone();
            seam.extend(seams.iter().filter_map(|e| pieces.get(e)).flatten());
            walk(b, &boundary, &succ, &leave, &seam)?
        }
    };
    for e in closed {
        cycles.push(vec![(e, true)]);
        cycles.push(vec![(e, false)]);
    }
    let groups = group(b, f, &cycles)?;
    if groups.len() < 2 {
        return Err(BodyError::NotCrossed { name });
    }
    let (surface, same_sense, shell) = (b.face(f).surface.clone(), b.face(f).same_sense, b.face(f).shell);
    b.kill_face(f);
    for (k, g) in groups.iter().enumerate() {
        let nf = b.add_face(
            shell,
            surface.clone(),
            same_sense,
            name.with_name(root).with_str("split").with(k as u64),
        );
        for &c in g {
            b.add_loop(nf, &cycles[c]);
        }
    }
    Ok(())
}

fn ends(b: &Body, (e, fwd): Use) -> (VertexId, VertexId) {
    let edge = b.edge(e);
    if fwd {
        (edge.start, edge.end)
    } else {
        (edge.end, edge.start)
    }
}

/// The face's boundary walked into cycles, turning onto a new edge wherever one leaves; cycles
/// that meet only across the face's seam are joined, the seam dropped.
fn walk(
    b: &Body,
    boundary: &[Use],
    succ: &[usize],
    leave: &HashMap<VertexId, Use>,
    seam: &[EdgeId],
) -> Result<Vec<Vec<Use>>, BodyError> {
    let mut from: HashMap<VertexId, Vec<usize>> = HashMap::new();
    for (k, &u) in boundary.iter().enumerate() {
        from.entry(ends(b, u).0).or_default().push(k);
    }
    let mut used = vec![false; boundary.len()];
    let mut cycles: Vec<Vec<Use>> = Vec::new();
    for first in 0..boundary.len() {
        if used[first] {
            continue;
        }
        let mut cyc = Vec::new();
        let mut k = first;
        while !used[k] {
            used[k] = true;
            cyc.push(boundary[k]);
            k = match leave.get(&ends(b, boundary[k]).1) {
                None => succ[k],
                Some(&u) => {
                    cyc.push(u);
                    match from.get(&ends(b, u).1).map(|v| &v[..]) {
                        Some(&[next]) => next,
                        _ => {
                            return Err(BodyError::Unsupported(
                                "a cut ending where the face's boundary meets itself",
                            ));
                        }
                    }
                }
            };
        }
        if k != first {
            return Err(BodyError::Unsupported(
                "a cut ending where the face's boundary meets itself",
            ));
        }
        cycles.push(cyc);
    }
    // join the two cycles each seam edge (or piece of one) borders, at that edge
    'join: loop {
        for i in 0..cycles.len() {
            for a in 0..cycles[i].len() {
                let (e, fwd) = cycles[i][a];
                if !seam.contains(&e) {
                    continue;
                }
                let Some(j) = (0..cycles.len()).find(|&j| j != i && cycles[j].contains(&(e, !fwd))) else {
                    continue;
                };
                let other = cycles.remove(j);
                let i = if j < i { i - 1 } else { i };
                let at = other.iter().position(|&u| u == (e, !fwd)).expect("found");
                let rest: Vec<Use> = (1..other.len()).map(|k| other[(at + k) % other.len()]).collect();
                cycles[i].splice(a..=a, rest);
                continue 'join;
            }
        }
        return Ok(cycles);
    }
}

/// A closed cut round a periodic face through its seam (split there in two pieces): the bands
/// either side, each the seam's piece up and back and the cut. Other loops stay whole.
fn round_seam(
    b: &Body,
    f: FaceId,
    boundary: &[Use],
    succ: &[usize],
    cut: EdgeId,
    pieces: &HashMap<EdgeId, Vec<EdgeId>>,
    seams: &[EdgeId],
) -> Result<Vec<Vec<Use>>, BodyError> {
    let bad = BodyError::Unsupported("a cut round a face through its seam");
    let (lo, hi) = seams
        .iter()
        .find_map(|s| match pieces.get(s).map(|p| &p[..]) {
            Some(&[lo, hi]) => Some((lo, hi)),
            _ => None,
        })
        .ok_or(bad.clone())?;
    let p = boundary
        .iter()
        .position(|&u| u == (lo, true))
        .ok_or(bad.clone())?;
    let q = boundary.iter().position(|&u| u == (hi, false)).ok_or(bad)?;
    // a loop's uses from one index round to another
    let run = |from: usize, to: usize| {
        let mut out = vec![boundary[from]];
        let mut k = from;
        while k != to {
            k = succ[k];
            out.push(boundary[k]);
        }
        out
    };
    let mut cycles = vec![run(succ[p], q), run(succ[q], p)];
    let s = &b.face(f).surface;
    // the cut goes the way that leaves the one band a plain outer loop
    let mut up = cycles[0].clone();
    up.push((cut, true));
    let pts = draw(b, s, &up);
    let fwd = winds(&pts, &s.domain()).is_none() && (signed_area(&pts) > 0.0) == b.face(f).same_sense;
    cycles[0].push((cut, fwd));
    cycles[1].push((cut, !fwd));
    let mut seen: std::collections::HashSet<Use> = cycles.iter().flatten().copied().collect();
    for (k, &u) in boundary.iter().enumerate() {
        if seen.insert(u) {
            let mut c = vec![u];
            let mut j = succ[k];
            while j != k {
                seen.insert(boundary[j]);
                c.push(boundary[j]);
                j = succ[j];
            }
            cycles.push(c);
        }
    }
    Ok(cycles)
}

/// A cycle drawn in the face's parameter plane, unwrapped as it goes.
fn draw(b: &Body, s: &Surface, cyc: &[Use]) -> Vec<Point2> {
    let d = s.domain();
    let mut out: Vec<Point2> = Vec::new();
    for &(e, fwd) in cyc {
        let mut pts = b.edge_points(e, default_segments(b.edge(e)));
        if !fwd {
            pts.reverse();
        }
        pts.pop();
        for p in pts {
            let uv = s.project(p);
            out.push(match out.last() {
                Some(&r) => near(uv, r, &d),
                None => uv,
            });
        }
    }
    out
}

/// Whether a drawn cycle winds round a period: the axis (0 u, 1 v) and whether forward.
fn winds(pts: &[Point2], d: &forge_geom::Domain2) -> Option<(usize, bool)> {
    let (first, last) = (pts[0], *pts.last().expect("a cycle has points"));
    let back = near(first, last, d) - first;
    if back.x.abs() > 1e-9 {
        Some((0, back.x > 0.0))
    } else if back.y.abs() > 1e-9 {
        Some((1, back.y > 0.0))
    } else {
        None
    }
}

fn near(uv: Point2, r: Point2, d: &forge_geom::Domain2) -> Point2 {
    let fix = |x: f64, r: f64, period: Option<f64>| match period {
        Some(p) => x + p * ((r - x) / p).round(),
        None => x,
    };
    Point2::new(fix(uv.x, r.x, d.u.period), fix(uv.y, r.y, d.v.period))
}

/// The cycles (indices) of each new face: each outer boundary with the holes inside it, and
/// on a periodic face the bands between loops that wrap round.
fn group(b: &Body, f: FaceId, cycles: &[Vec<Use>]) -> Result<Vec<Vec<usize>>, BodyError> {
    let s = &b.face(f).surface;
    let d = s.domain();
    let same = b.face(f).same_sense;
    let drawn: Vec<Vec<Point2>> = cycles.iter().map(|c| draw(b, s, c)).collect();
    // each cycle's wrap: the axis it winds round (0 u, 1 v) and which way
    let wrap: Vec<Option<(usize, bool)>> = drawn.iter().map(|pts| winds(pts, &d)).collect();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut outer: Vec<(usize, f64)> = Vec::new();
    let mut holes: Vec<usize> = Vec::new();
    for (k, pts) in drawn.iter().enumerate() {
        if wrap[k].is_none() {
            let a = signed_area(pts);
            if (a > 0.0) == same {
                outer.push((groups.len(), a.abs()));
                groups.push(vec![k]);
            } else {
                holes.push(k);
            }
        }
    }
    // bands: sorted across the wrap, each loop with material past it paired with the next
    let mut wrapped: Vec<(f64, usize, bool)> = Vec::new();
    let mut axis = None;
    for (k, w) in wrap.iter().enumerate() {
        let Some((a, up)) = *w else { continue };
        if axis.is_some_and(|x| x != a) {
            return Err(BodyError::Unsupported("loops wrapping both ways round a face"));
        }
        axis = Some(a);
        let across = |p: &Point2| if a == 0 { p.y } else { p.x };
        let mean = drawn[k].iter().map(across).sum::<f64>() / drawn[k].len() as f64;
        // the material lies on the travel's left (right, the face against its surface)
        let above = if a == 0 { up == same } else { up != same };
        wrapped.push((mean, k, above));
    }
    wrapped.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut bands: Vec<(usize, f64, f64)> = Vec::new();
    let mut j = 0;
    while j < wrapped.len() {
        let (lo, k, above) = wrapped[j];
        if above && j + 1 < wrapped.len() && !wrapped[j + 1].2 {
            bands.push((groups.len(), lo, wrapped[j + 1].0));
            groups.push(vec![k, wrapped[j + 1].1]);
            j += 2;
        } else {
            let (lo, hi) = if above {
                (lo, f64::INFINITY)
            } else {
                (f64::NEG_INFINITY, lo)
            };
            bands.push((groups.len(), lo, hi));
            groups.push(vec![k]);
            j += 1;
        }
    }
    // a hole goes in the innermost outer loop round a point just inside its face
    for h in holes {
        let pts = &drawn[h];
        let (p, q) = (pts[0], pts[1 % pts.len()]);
        let along = q - p;
        let len = along.length();
        if len == 0.0 {
            return Err(BodyError::Unsupported("a degenerate loop"));
        }
        let left = Vec2::new(-along.y, along.x) * (1e-3 * len.min(1.0) / len);
        let probe = p + along * 0.5 + if same { left } else { -left };
        let holder = outer
            .iter()
            .filter(|(g, _)| {
                let o = &drawn[groups[*g][0]];
                forge_math::predicates::inside_polygon(near(probe, o[0], &d), o)
            })
            .min_by(|x, y| x.1.total_cmp(&y.1))
            .map(|x| x.0)
            .or_else(|| {
                let a = axis?;
                let across = if a == 0 { probe.y } else { probe.x };
                bands
                    .iter()
                    .find(|bd| across > bd.1 && across < bd.2)
                    .map(|bd| bd.0)
            })
            .ok_or(BodyError::Unsupported("a hole in no piece"))?;
        groups[holder].push(h);
    }
    Ok(groups)
}

//! What a body is made of, read off its B-rep alone (a part, an import): its round holes and its
//! section by a plane. Nothing here needs the history that made the body.

use crate::select::edges_of_face;
use forge_geom::{Curve, CurveGeom, Surface, SurfaceGeom};
use forge_math::{Frame, Point3, Vec3, dmath};
use forge_mesh::Quality;
use forge_mesh::tessellate::tessellate_tolerant;
use forge_topo::{Body, FaceId};
use std::collections::HashMap;

/// A round hole: the concave cylinder faces round one axis at one radius, closed all the way round.
#[derive(Clone, Debug, PartialEq)]
pub struct Hole {
    pub faces: Vec<FaceId>,
    /// On the axis, halfway along the hole.
    pub centre: Point3,
    /// Out of the floor for a blind hole; else the sign that makes its largest part positive.
    pub axis: Vec3,
    pub radius: f64,
    /// Its length along the axis.
    pub depth: f64,
    pub thru: bool,
}

const BINS: usize = 72;

fn canonical(d: Vec3) -> Vec3 {
    let [x, y, z] = [d.x, d.y, d.z];
    let big = if x.abs() >= y.abs() && x.abs() >= z.abs() {
        x
    } else if y.abs() >= z.abs() {
        y
    } else {
        z
    };
    if big < 0.0 { -d } else { d }
}

/// The body's round holes, in a fixed order (radius, then centre x, y, z).
pub fn holes(body: &Body) -> Vec<Hole> {
    // concave cylinders: the face's normal points at the axis
    let mut cyl: Vec<(FaceId, Frame, f64)> = Vec::new();
    for f in body.face_ids() {
        let face = body.face(f);
        let Surface::Cylinder(c) = &face.surface else {
            continue;
        };
        let Some(&e) = edges_of_face(body, f).first() else {
            continue;
        };
        let pts = body.edge_points(e, 3);
        let p = pts[1];
        let Some(n) = body.face_normal(f, face.surface.project(p)) else {
            continue;
        };
        let z = c.frame.z().get();
        let foot = c.frame.origin + z * (p - c.frame.origin).dot(z);
        if n.get().dot(p - foot) < 0.0 {
            cyl.push((f, c.frame, c.radius));
        }
    }
    // one axis and radius: one group
    let mut groups: Vec<(Vec3, Point3, f64, Vec<FaceId>)> = Vec::new();
    for (f, fr, r) in cyl {
        let d = canonical(fr.z().get());
        let o = fr.origin - d * (fr.origin - Point3::ORIGIN).dot(d);
        let tol = 1e-6 * (1.0 + r);
        let same = groups
            .iter_mut()
            .find(|g| g.0.cross(d).length() < 1e-9 && g.1.distance(o) < tol && (g.2 - r).abs() < tol);
        match same {
            Some(g) => g.3.push(f),
            None => groups.push((d, o, r, vec![f])),
        }
    }
    // one axis may run through several holes (a wall's and the wall's across): faces that touch
    let mut split = Vec::new();
    for (d, o, r, faces) in groups {
        let mut comp: Vec<Vec<FaceId>> = Vec::new();
        for f in faces {
            let near: Vec<usize> = (0..comp.len())
                .filter(|&k| {
                    comp[k].iter().any(|&g| {
                        edges_of_face(body, f)
                            .iter()
                            .any(|e| crate::select::faces_of_edge(body, *e).contains(&g))
                    })
                })
                .collect();
            let mut joined = vec![f];
            for &k in near.iter().rev() {
                joined.extend(comp.remove(k));
            }
            comp.push(joined);
        }
        split.extend(comp.into_iter().map(|c| (d, o, r, c)));
    }
    let mut out = Vec::new();
    for (d, o, r, faces) in split {
        let fr = Frame::from_z(o, d.normalize().expect("a cylinder's axis"));
        let (x, y) = (fr.x().get(), fr.y().get());
        let mut bins = [false; BINS];
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        let bin = |a: f64| ((dmath::wrap_tau(a) / dmath::TAU * BINS as f64) as usize).min(BINS - 1);
        for &f in &faces {
            for e in edges_of_face(body, f) {
                let pts = body.edge_points(e, 24);
                let ang: Vec<f64> = pts
                    .iter()
                    .map(|p| {
                        let v = *p - o;
                        let t = v.dot(d);
                        lo = lo.min(t);
                        hi = hi.max(t);
                        dmath::atan2(v.dot(y), v.dot(x))
                    })
                    .collect();
                for w in ang.windows(2) {
                    let mut step = dmath::wrap_tau(w[1] - w[0]);
                    if step > dmath::PI {
                        step -= dmath::TAU;
                    }
                    let n = (step.abs() / (dmath::TAU / BINS as f64)).ceil() as usize * 2 + 1;
                    for k in 0..=n {
                        bins[bin(w[0] + step * k as f64 / n as f64)] = true;
                    }
                }
            }
        }
        if !bins.iter().all(|b| *b) {
            continue;
        }
        // an end closed by a floor: a plane square to the axis whose outer loop holds the rim
        let mut floor: Option<f64> = None;
        for &f in &faces {
            for e in edges_of_face(body, f) {
                if !matches!(body.edge(e).curve, Curve::Circle(_)) {
                    continue;
                }
                for g in crate::select::faces_of_edge(body, e) {
                    if faces.contains(&g) {
                        continue;
                    }
                    let gf = body.face(g);
                    let Surface::Plane(pl) = &gf.surface else { continue };
                    if pl.frame.z().get().cross(d).length() > 1e-9 {
                        continue;
                    }
                    let outer = body.lp(gf.loops[0]).coedges().any(|c| body.coedge(c).edge == e);
                    if outer {
                        floor = Some((pl.frame.origin - o).dot(d));
                    }
                }
            }
        }
        let axis = match floor {
            Some(t) if (t - lo).abs() > (t - hi).abs() => -d,
            _ => d,
        };
        let mut faces = faces;
        faces.sort();
        out.push(Hole {
            faces,
            centre: o + d * ((lo + hi) / 2.0),
            axis,
            radius: r,
            depth: hi - lo,
            thru: floor.is_none(),
        });
    }
    let key = |h: &Hole| [h.radius, h.centre.x, h.centre.y, h.centre.z].map(|v| (v * 1e6).round() as i64);
    out.sort_by_key(key);
    out
}

/// One run of a section loop over one face: a line on a plane, an arc on a cylinder standing
/// square to the cut, else the points it passes.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    Line(Point3, Point3),
    /// Centre, radius, and its points end to end (the first and last are its ends).
    Arc(Point3, f64, Vec<Point3>),
    Curve(Vec<Point3>),
}

impl Piece {
    pub fn points(&self) -> Vec<Point3> {
        match self {
            Piece::Line(a, b) => vec![*a, *b],
            Piece::Arc(_, _, p) | Piece::Curve(p) => p.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Loop {
    /// Each with the face it runs over.
    pub pieces: Vec<(FaceId, Piece)>,
    /// Signed about the plane's normal: material on the left, so outlines count up and holes down.
    pub area: f64,
    pub length: f64,
    pub closed: bool,
}

fn refine_on_face(body: &Body, f: FaceId, plane: &Frame, mut p: Point3) -> Point3 {
    let s = &body.face(f).surface;
    if matches!(s, Surface::Plane(_)) {
        return p;
    }
    let (o, n) = (plane.origin, plane.z().get());
    for _ in 0..40 {
        let q = s.eval(s.project(p));
        let next = q - n * (q - o).dot(n);
        if next.distance(p) < 1e-13 {
            return next;
        }
        p = next;
    }
    p
}

/// Where the edge crosses the plane, near `p`: Newton along the curve.
fn refine_on_edge(body: &Body, e: forge_topo::EdgeId, plane: &Frame, p: Point3) -> Point3 {
    let edge = body.edge(e);
    let (o, n) = (plane.origin, plane.z().get());
    let c = &edge.curve;
    if let Curve::Line(_) = c {
        let (a, b) = (c.eval(edge.range.lo), c.eval(edge.range.hi));
        let (da, db) = ((a - o).dot(n), (b - o).dot(n));
        if (da - db).abs() > 1e-15 {
            return a + (b - a) * (da / (da - db));
        }
        return p;
    }
    let mut t = c.closest(p, edge.range);
    for _ in 0..30 {
        let dv = c.deriv(t);
        let g = (dv.p - o).dot(n);
        let dg = dv.d1.dot(n);
        if dg.abs() < 1e-15 {
            break;
        }
        let nt = (t - g / dg).clamp(edge.range.lo, edge.range.hi);
        if (nt - t).abs() < 1e-15 {
            break;
        }
        t = nt;
    }
    let q = c.eval(t);
    if q.distance(p) < 0.1 { q } else { p }
}

fn nearest_edge(body: &Body, f: FaceId, p: Point3) -> Option<forge_topo::EdgeId> {
    edges_of_face(body, f)
        .into_iter()
        .map(|e| {
            let edge = body.edge(e);
            let q = edge.curve.eval(edge.curve.closest(p, edge.range));
            (q.distance(p), e)
        })
        .filter(|x| x.0 < 0.1)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|x| x.1)
}

fn common_edge(body: &Body, a: FaceId, b: FaceId) -> Option<forge_topo::EdgeId> {
    edges_of_face(body, a)
        .into_iter()
        .find(|e| crate::select::faces_of_edge(body, *e).contains(&b))
}

/// A crossing: the mesh edge it is on, by its two vertices.
type Key = (u32, u32);

/// The body cut by the plane (`plane`'s z the normal): its loops, each in runs over the faces it
/// crosses, the points on the true surfaces and edges, not the triangles'.
pub fn section(body: &Body, plane: &Frame) -> Result<Vec<Loop>, String> {
    let q = Quality {
        chord: 0.01,
        ..Quality::default()
    };
    let mesh = tessellate_tolerant(body, &q).map_err(|e| format!("mesh: {e:?}"))?;
    let (o, n) = (plane.origin, plane.z().get());
    // equal positions are one vertex, whatever the mesher shared
    let mut ids: HashMap<[u64; 3], u32> = HashMap::new();
    let vid: Vec<u32> = mesh
        .positions
        .iter()
        .map(|p| {
            let k = [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()];
            let l = ids.len() as u32;
            *ids.entry(k).or_insert(l)
        })
        .collect();
    let side: Vec<bool> = mesh.positions.iter().map(|p| (*p - o).dot(n) >= 0.0).collect();
    // each crossing (a mesh edge by its two vertices): its point and the faces beside it
    let mut cross: HashMap<(u32, u32), (Point3, Vec<FaceId>, usize)> = HashMap::new();
    let mut segs: Vec<(Key, Key, FaceId)> = Vec::new();
    for (k, t) in mesh.triangles.iter().enumerate() {
        let f = FaceId(mesh.face_of[k]);
        let v = t.map(|i| i as usize);
        let s = v.map(|i| side[i]);
        if s[0] == s[1] && s[1] == s[2] {
            continue;
        }
        let mut hits = Vec::new();
        for (i, j) in [(0, 1), (1, 2), (2, 0)] {
            if s[i] == s[j] {
                continue;
            }
            let (a, b) = (mesh.positions[v[i]], mesh.positions[v[j]]);
            let (da, db) = ((a - o).dot(n), (b - o).dot(n));
            let key = {
                let (x, y) = (vid[v[i]], vid[v[j]]);
                if x < y { (x, y) } else { (y, x) }
            };
            let e = cross
                .entry(key)
                .or_insert_with(|| (a + (b - a) * (da / (da - db)), vec![], 0));
            if !e.1.contains(&f) {
                e.1.push(f);
            }
            e.2 += 1;
            hits.push(key);
        }
        let [p0, p1, p2] = v.map(|i| mesh.positions[i]);
        let tn = (p1 - p0).cross(p2 - p0);
        let dir = n.cross(tn);
        let (a, b) = (hits[0], hits[1]);
        let along = cross[&b].0 - cross[&a].0;
        if along.dot(dir) >= 0.0 {
            segs.push((a, b, f));
        } else {
            segs.push((b, a, f));
        }
    }
    // the true points: on the body's edge where a crossing is on a face's rim, else on the surface
    let mut exact: HashMap<(u32, u32), Point3> = HashMap::new();
    for (k, (p, faces, uses)) in &cross {
        let q = match faces[..] {
            [a, b, ..] if a != b => match common_edge(body, a, b) {
                Some(e) => refine_on_edge(body, e, plane, *p),
                None => refine_on_face(body, a, plane, *p),
            },
            [a] if *uses == 1 => match nearest_edge(body, a, *p) {
                Some(e) => refine_on_edge(body, e, plane, *p),
                None => refine_on_face(body, a, plane, *p),
            },
            [a, ..] => refine_on_face(body, a, plane, *p),
            [] => *p,
        };
        exact.insert(*k, q);
    }
    // crossings at one point are one node: faces meshed apart, or the plane through a vertex
    let mut keys: Vec<(u32, u32)> = exact.keys().copied().collect();
    keys.sort();
    let mut grid: HashMap<[i64; 3], Vec<u32>> = HashMap::new();
    let mut nodes: Vec<Point3> = Vec::new();
    let mut node: HashMap<(u32, u32), u32> = HashMap::new();
    const SNAP: f64 = 1e-6;
    for k in keys {
        let p = exact[&k];
        let c = [p.x, p.y, p.z].map(|v| (v / SNAP).floor() as i64);
        let mut found = None;
        'look: for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(l) = grid.get(&[c[0] + dx, c[1] + dy, c[2] + dz])
                        && let Some(&i) = l.iter().find(|&&i| nodes[i as usize].distance(p) < SNAP)
                    {
                        found = Some(i);
                        break 'look;
                    }
                }
            }
        }
        let i = found.unwrap_or_else(|| {
            nodes.push(p);
            let i = nodes.len() as u32 - 1;
            grid.entry(c).or_default().push(i);
            i
        });
        node.insert(k, i);
    }
    let segs: Vec<(u32, u32, FaceId)> = segs
        .into_iter()
        .map(|(a, b, f)| (node[&a], node[&b], f))
        .filter(|s| s.0 != s.1)
        .collect();
    let mut next: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, s) in segs.iter().enumerate() {
        next.entry(s.0).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let mut loops = Vec::new();
    // a fixed start for each loop: the least node not yet used
    let mut order: Vec<usize> = (0..segs.len()).collect();
    order.sort_by_key(|&i| (segs[i].0, segs[i].1));
    for &start in &order {
        if used[start] {
            continue;
        }
        let mut chain: Vec<usize> = vec![start];
        used[start] = true;
        let mut closed = false;
        loop {
            let end = segs[*chain.last().expect("a chain")].1;
            if end == segs[start].0 {
                closed = true;
                break;
            }
            let Some(n) = next.get(&end).and_then(|c| c.iter().copied().find(|&i| !used[i])) else {
                break;
            };
            used[n] = true;
            chain.push(n);
        }
        loops.push(pieces(body, plane, &segs, &chain, &nodes, closed));
    }
    loops.sort_by(|a, b| b.area.abs().total_cmp(&a.area.abs()));
    Ok(loops)
}

fn pieces(
    body: &Body,
    plane: &Frame,
    segs: &[(u32, u32, FaceId)],
    chain: &[usize],
    nodes: &[Point3],
    closed: bool,
) -> Loop {
    let n = plane.z().get();
    // runs of one face
    let mut runs: Vec<(FaceId, Vec<Point3>)> = Vec::new();
    for &i in chain {
        let (a, b, f) = segs[i];
        match runs.last_mut() {
            Some((g, pts)) if *g == f => pts.push(nodes[b as usize]),
            _ => runs.push((f, vec![nodes[a as usize], nodes[b as usize]])),
        }
    }
    if closed && runs.len() > 1 && runs[0].0 == runs[runs.len() - 1].0 {
        let (_, last) = runs.pop().expect("two runs");
        let mut pts = last;
        pts.extend(runs[0].1.drain(1..));
        runs[0].1 = pts;
    }
    let mut area = 0.0;
    let mut length = 0.0;
    let mut out = Vec::new();
    let (x, y) = (plane.x().get(), plane.y().get());
    let uv = |p: Point3| {
        let v = p - plane.origin;
        (v.dot(x), v.dot(y))
    };
    for (f, pts) in runs {
        let piece = match &body.face(f).surface {
            Surface::Plane(_) => Piece::Line(pts[0], pts[pts.len() - 1]),
            // a cylinder cut along its axis: straight
            Surface::Cylinder(c) if c.frame.z().get().dot(n).abs() < 1e-12 => {
                Piece::Line(pts[0], pts[pts.len() - 1])
            }
            Surface::Cylinder(c) if c.frame.z().get().cross(n).length() < 1e-9 => {
                let z = c.frame.z().get();
                let centre = c.frame.origin + z * (plane.origin - c.frame.origin).dot(n) / z.dot(n);
                let pts = pts
                    .iter()
                    .map(|p| match (*p - centre).normalize() {
                        Some(u) => centre + u.get() * c.radius,
                        None => *p,
                    })
                    .collect();
                Piece::Arc(centre, c.radius, pts)
            }
            _ => Piece::Curve(pts),
        };
        match &piece {
            Piece::Arc(c, r, pts) => {
                let (cx, cy) = uv(*c);
                let mut sweep = 0.0;
                for w in pts.windows(2) {
                    let (a, b) = (uv(w[0]), uv(w[1]));
                    let (ta, tb) = (dmath::atan2(a.1 - cy, a.0 - cx), dmath::atan2(b.1 - cy, b.0 - cx));
                    let mut d = dmath::wrap_tau(tb - ta);
                    if d > dmath::PI {
                        d -= dmath::TAU;
                    }
                    sweep += d;
                }
                let (a, _) = (uv(pts[0]), 0);
                let t0 = dmath::atan2(a.1 - cy, a.0 - cx);
                let t1 = t0 + sweep;
                area += 0.5
                    * (r * cx * (dmath::sin(t1) - dmath::sin(t0))
                        - r * cy * (dmath::cos(t1) - dmath::cos(t0))
                        + r * r * sweep);
                length += r * sweep.abs();
            }
            p => {
                for w in p.points().windows(2) {
                    let (a, b) = (uv(w[0]), uv(w[1]));
                    area += 0.5 * (a.0 * b.1 - b.0 * a.1);
                    length += w[0].distance(w[1]);
                }
            }
        }
        out.push((f, piece));
    }
    Loop {
        pieces: out,
        area,
        length,
        closed,
    }
}

/// Where two bodies (in one frame) share volume: how much, where, and which faces of each bound it.
#[derive(Clone, Debug, PartialEq)]
pub struct Clash {
    pub volume: f64,
    pub centroid: Point3,
    pub lo: Point3,
    pub hi: Point3,
    pub faces_a: Vec<FaceId>,
    pub faces_b: Vec<FaceId>,
}

/// The shared volume of `a` and `b` (none if they only touch or miss), by the kernel's
/// intersection.
pub fn clash(a: &Body, b: &Body) -> Result<Option<Clash>, String> {
    let c = forge_bool::boolean::boolean(a, b, forge_bool::boolean::Op::Intersect, 0)
        .map_err(|e| format!("the intersection failed: {e:?}"))?;
    if c.face_ids().next().is_none() {
        return Ok(None);
    }
    let m = forge_topo::mass::mass(&c).map_err(|e| format!("mass of the intersection: {e:?}"))?;
    if m.volume.abs() < 1e-9 {
        return Ok(None);
    }
    // tight: the edges' own points (a face's box is padded)
    let mut bb = forge_math::Box3::EMPTY;
    for e in c.edge_ids() {
        for p in c.edge_points(e, 16) {
            bb = bb.hull(&forge_math::Box3::from_point(p));
        }
    }
    let of = |src: &Body| {
        let mut out: Vec<FaceId> = Vec::new();
        for f in c.face_ids() {
            let n = c.face(f).name;
            let hit = src
                .face_ids()
                .find(|&g| src.face(g).name == n)
                .or_else(|| src.face_ids().find(|&g| n.is_piece_of(src.face(g).name)));
            if let Some(g) = hit
                && !out.contains(&g)
            {
                out.push(g);
            }
        }
        out.sort();
        out
    };
    Ok(Some(Clash {
        volume: m.volume.abs(),
        centroid: m.centroid,
        lo: Point3::new(bb.x.lo, bb.y.lo, bb.z.lo),
        hi: Point3::new(bb.x.hi, bb.y.hi, bb.z.hi),
        faces_a: of(a),
        faces_b: of(b),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::{Point2, UnitVec3};
    use forge_ops::{Profile, extrude};

    fn plate_with_holes() -> Body {
        let p = Profile {
            plane: Frame::WORLD,
            loops: vec![
                Profile::rectangle(Point2::new(0.0, 0.0), 60.0, 40.0, 0),
                Profile::circle(Point2::new(15.0, 20.0), 2.5, 100),
                Profile::circle(Point2::new(45.0, 20.0), 4.0, 200),
            ],
        };
        extrude(&p, 6.0, 1).expect("plate")
    }

    #[test]
    fn a_plate_has_its_two_round_holes_and_no_others() {
        let b = plate_with_holes();
        let h = holes(&b);
        assert_eq!(h.len(), 2, "{h:?}");
        assert!((h[0].radius - 2.5).abs() < 1e-12);
        assert!(
            h[0].centre.distance(Point3::new(15.0, 20.0, 3.0)) < 1e-9,
            "{:?}",
            h[0].centre
        );
        assert!((h[0].depth - 6.0).abs() < 1e-9);
        assert!(h[0].thru);
        assert!((h[0].axis - Vec3::Z).length() < 1e-12);
        assert!((h[1].radius - 4.0).abs() < 1e-12);
    }

    #[test]
    fn a_section_of_the_plate_is_its_outline_less_its_holes_exactly() {
        let b = plate_with_holes();
        let at = Frame::from_z(Point3::new(0.0, 0.0, 2.0), UnitVec3::Z);
        let l = section(&b, &at).unwrap();
        assert_eq!(l.len(), 3);
        assert!(l.iter().all(|x| x.closed));
        assert!((l[0].area - 2400.0).abs() < 1e-6, "{}", l[0].area);
        let pi = dmath::PI;
        assert!((l[1].area + pi * 16.0).abs() < 1e-6, "{}", l[1].area);
        assert!((l[2].area + pi * 6.25).abs() < 1e-6, "{}", l[2].area);
        let arc = l[1].pieces.iter().find_map(|p| match &p.1 {
            Piece::Arc(c, r, _) => Some((*c, *r)),
            _ => None,
        });
        let (c, r) = arc.expect("an arc");
        assert!(c.distance(Point3::new(45.0, 20.0, 2.0)) < 1e-9 && (r - 4.0).abs() < 1e-12);
        // across: the plate's side, and two lines either side of each hole
        let across = Frame::from_z(Point3::new(0.0, 20.0, 0.0), UnitVec3::Y);
        let l = section(&b, &across).unwrap();
        assert_eq!(l.len(), 3, "{l:?}");
        let total: f64 = l.iter().map(|x| x.area.abs()).sum();
        assert!((total - 6.0 * (60.0 - 5.0 - 8.0)).abs() < 1e-6, "{total}");
    }
}

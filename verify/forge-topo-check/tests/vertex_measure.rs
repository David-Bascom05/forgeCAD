//! Measurements for the Lean statements (not a gate; `--ignored`): at each vertex of the suite's
//! and fuzz parts' bodies,
//! - coplanar incident faces: two distinct faces using the vertex, on one plane (ledger D8.cells:
//!   `J` then holds a plane);
//! - vertex isolation: does the vertex's ball (radius its `tol`, and fixed radii) meet a planar
//!   face that does not use the vertex (ledger D8.iso)?

use forge_geom::curve::Curve;
use forge_geom::surface::Surface;
use forge_math::Point3;
use forge_model::suite;
use forge_topo::body::{Body, FaceId, VertexId};

fn bodies(nfuzz: u64) -> Vec<(String, Body)> {
    let mut parts: Vec<(String, forge_model::Part)> = suite::parts()
        .into_iter()
        .filter(|(n, _)| !matches!(*n, "torture-2" | "text" | "thread"))
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    parts.extend((0..nfuzz).map(|s| (format!("fuzz {s}"), suite::fuzz(s, 4))));
    suite::each(&parts, |(n, p)| (n.clone(), p.regenerate().ok()))
        .into_iter()
        .filter_map(|(n, b)| b.map(|b| (n, b)))
        .collect()
}

/// The live faces using each vertex (through their loops' edges' ends).
fn incident(b: &Body) -> Vec<Vec<FaceId>> {
    let mut inc: Vec<Vec<FaceId>> = vec![Vec::new(); b.vertices.len()];
    for (fi, f) in b.faces.iter().enumerate() {
        if !f.alive {
            continue;
        }
        for &l in f.loops.iter() {
            for c in b.loops[l.index()].coedges() {
                let e = &b.edges[b.coedges[c.index()].edge.index()];
                for v in [e.start, e.end] {
                    let s = &mut inc[v.index()];
                    if !s.contains(&FaceId(fi as u32)) {
                        s.push(FaceId(fi as u32));
                    }
                }
            }
        }
    }
    inc
}

fn plane_of(b: &Body, f: FaceId) -> Option<(forge_math::Vec3, f64)> {
    match &b.faces[f.index()].surface {
        Surface::Plane(p) => {
            let n = p.frame.z().get();
            let o = p.frame.origin;
            Some((n, n.dot(o - Point3::ORIGIN)))
        }
        _ => None,
    }
}

fn same_plane(a: (forge_math::Vec3, f64), b: (forge_math::Vec3, f64), tol: f64) -> bool {
    let c = a.0.dot(b.0);
    (c >= 1.0 - tol && (a.1 - b.1).abs() <= tol) || (c <= -1.0 + tol && (a.1 + b.1).abs() <= tol)
}

/// Normals exactly parallel (each cross component exactly zero) and offsets equal as computed.
fn same_plane_exact(a: (forge_math::Vec3, f64), b: (forge_math::Vec3, f64)) -> bool {
    use forge_math::predicates::cross_sign;
    let (n, m) = (a.0, b.0);
    cross_sign(n.y, m.z, n.z, m.y) == 0.0
        && cross_sign(n.z, m.x, n.x, m.z) == 0.0
        && cross_sign(n.x, m.y, n.y, m.x) == 0.0
        && (if n.dot(m) > 0.0 { a.1 == b.1 } else { a.1 == -b.1 })
}

/// The distance from `p` to planar face `f`'s region, for faces bounded by lines: the plane
/// distance when the projection is inside the outer loop's polygon, else the distance to its
/// segments. `None` for a face with a curved edge.
fn dist_planar(b: &Body, f: FaceId, p: Point3) -> Option<f64> {
    let Surface::Plane(pl) = &b.faces[f.index()].surface else { return None };
    let (x, y, z) = (pl.frame.x().get(), pl.frame.y().get(), pl.frame.z().get());
    let o = pl.frame.origin;
    let to2 = |q: Point3| ((q - o).dot(x), (q - o).dot(y));
    let (px, py) = to2(p);
    let h = (p - o).dot(z).abs();
    let mut segs: Vec<((f64, f64), (f64, f64))> = Vec::new();
    let mut inside = false;
    for (k, &l) in b.faces[f.index()].loops.iter().enumerate() {
        for c in b.loops[l.index()].coedges() {
            let e = &b.edges[b.coedges[c.index()].edge.index()];
            if !matches!(e.curve, Curve::Line(_)) {
                return None;
            }
            let a = to2(b.vertices[e.start.index()].point);
            let bb = to2(b.vertices[e.end.index()].point);
            segs.push((a, bb));
            // crossing count (all loops: holes flip it back)
            let _ = k;
            if (a.1 > py) != (bb.1 > py) {
                let t = (py - a.1) / (bb.1 - a.1);
                if a.0 + t * (bb.0 - a.0) > px {
                    inside = !inside;
                }
            }
        }
    }
    if inside {
        return Some(h);
    }
    let mut best = f64::INFINITY;
    for (a, bb) in segs {
        let (dx, dy) = (bb.0 - a.0, bb.1 - a.1);
        let len2 = dx * dx + dy * dy;
        let t = if len2 > 0.0 { (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
        let (qx, qy) = (a.0 + t * dx - px, a.1 + t * dy - py);
        best = best.min((qx * qx + qy * qy + h * h).sqrt());
    }
    Some(best)
}

#[test]
#[ignore]
fn measure_vertices() {
    let nfuzz: u64 = std::env::var("FUZZ").ok().and_then(|v| v.parse().ok()).unwrap_or(200);
    let bs = bodies(nfuzz);
    let radii = [1e-9, 1e-7, 1e-6, 1e-5];
    let (mut nv, mut cop_exact, mut cop_near, mut skipped) = (0usize, 0usize, 0usize, 0usize);
    let mut iso_tol = 0usize;
    let mut iso_r = [0usize; 4];
    let mut min_gap = f64::INFINITY;
    let mut examples: Vec<String> = Vec::new();
    for (name, b) in &bs {
        let inc = incident(b);
        for (vi, v) in b.vertices.iter().enumerate() {
            let fs = &inc[vi];
            if fs.is_empty() {
                continue;
            }
            nv += 1;
            // coplanar incident faces
            let planes: Vec<(FaceId, (forge_math::Vec3, f64))> =
                fs.iter().filter_map(|&f| plane_of(b, f).map(|p| (f, p))).collect();
            let mut ex = false;
            let mut near = false;
            for i in 0..planes.len() {
                for j in i + 1..planes.len() {
                    if same_plane_exact(planes[i].1, planes[j].1) {
                        ex = true;
                    }
                    if same_plane(planes[i].1, planes[j].1, 1e-9) {
                        near = true;
                    }
                }
            }
            if ex {
                cop_exact += 1;
            }
            if near {
                cop_near += 1;
                if examples.len() < 12 {
                    examples.push(format!("{name}: vertex {vi} at {:?}, faces {:?}", v.point, fs));
                }
            }
            // isolation: the nearest planar face not using the vertex
            let mut gap = f64::INFINITY;
            for (fi, f) in b.faces.iter().enumerate() {
                let fid = FaceId(fi as u32);
                if !f.alive || fs.contains(&fid) {
                    continue;
                }
                match dist_planar(b, fid, v.point) {
                    Some(d) => gap = gap.min(d),
                    None => skipped += 1,
                }
            }
            min_gap = min_gap.min(gap);
            if gap <= v.tol {
                iso_tol += 1;
            }
            for (k, r) in radii.iter().enumerate() {
                if gap <= *r {
                    iso_r[k] += 1;
                }
            }
            let _ = VertexId(vi as u32);
        }
    }
    println!("bodies {}, vertices {nv}", bs.len());
    println!("coplanar incident faces: exactly parallel normals and equal offsets {cop_exact}, within 1e-9 {cop_near}");
    for e in &examples {
        println!("  {e}");
    }
    println!("vertex isolation: a non-incident planar face within the vertex's tol: {iso_tol}");
    for (k, r) in radii.iter().enumerate() {
        println!("  within {r:e}: {}", iso_r[k]);
    }
    println!("smallest gap to a non-incident planar face: {min_gap:e}; curved faces skipped {skipped}");
}


/// The suite's `coplanar` part has vertices where two incident faces lie on exactly one plane (18
/// when written: the z = 20 face and small faces on it). So the Lean model may not assume distinct
/// planes at a vertex: D8.formula's `J` counts only carriers with different zero sets, plus the
/// zone's edge points (verify/forge-bool-proof LEDGER D8.cells).
#[test]
fn coplanar_part_has_coplanar_incident_faces() {
    let b = suite::part("coplanar").expect("the coplanar part").regenerate().expect("it builds");
    let inc = incident(&b);
    let exact = inc
        .iter()
        .filter(|fs| {
            let ps: Vec<(forge_math::Vec3, f64)> = fs.iter().filter_map(|&f| plane_of(&b, f)).collect();
            (0..ps.len()).any(|i| (i + 1..ps.len()).any(|j| same_plane_exact(ps[i], ps[j])))
        })
        .count();
    assert!(exact > 0, "no coplanar incident faces left in the coplanar part: J's refinement can be revisited");
}

/// For the coplanar pairs at a vertex: do the two faces share an edge (which `unify` should have
/// merged), or meet only at vertices?
#[test]
#[ignore]
fn coplanar_pairs_share_an_edge() {
    let b = suite::part("coplanar").expect("the coplanar part").regenerate().expect("it builds");
    let inc = incident(&b);
    let faces_of_edge = |e: usize| -> Vec<FaceId> {
        b.edges[e].coedges.iter().map(|&c| b.coedge_face(c)).collect()
    };
    let mut pairs: Vec<(usize, FaceId, FaceId)> = Vec::new();
    for (vi, fs) in inc.iter().enumerate() {
        for i in 0..fs.len() {
            for j in i + 1..fs.len() {
                if let (Some(p), Some(q)) = (plane_of(&b, fs[i]), plane_of(&b, fs[j]))
                    && same_plane_exact(p, q)
                {
                    pairs.push((vi, fs[i], fs[j]));
                }
            }
        }
    }
    for (vi, f, g) in &pairs {
        let shared: Vec<usize> = (0..b.edges.len())
            .filter(|&e| b.edges[e].alive)
            .filter(|&e| {
                let fs = faces_of_edge(e);
                fs.contains(f) && fs.contains(g)
            })
            .collect();
        let same_sense = b.faces[f.index()].same_sense == b.faces[g.index()].same_sense;
        let n = plane_of(&b, *f).unwrap().0.dot(plane_of(&b, *g).unwrap().0);
        println!("vertex {vi}: faces {f:?} {g:?}, normals dot {n}, same_sense equal {same_sense}, shared edges {shared:?}");
    }
}

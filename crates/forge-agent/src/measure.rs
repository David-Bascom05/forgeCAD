//! Distances, angles, mass properties and wall thickness.

use crate::select::{self, Ent};
use forge_geom::Curve;
use forge_math::{Point3, Vec3, dmath};
use forge_mesh::TriMesh;
use forge_topo::Body;

pub struct Mass {
    pub volume: f64,
    pub area: f64,
    pub centroid: Point3,
    /// Second moments about the centroid (unit density): Ixx, Iyy, Izz.
    pub inertia: [f64; 3],
}

pub fn mass(m: &TriMesh) -> Mass {
    let (mut vol, mut c) = (0.0, Vec3::ZERO);
    // integrals of x^2, y^2, z^2 over the solid, from tetrahedra on the origin
    let mut sq = [0.0f64; 3];
    for t in &m.triangles {
        let [a, b, d] = t.map(|i| m.positions[i as usize] - Point3::ORIGIN);
        let v = a.dot(b.cross(d)) / 6.0;
        vol += v;
        c += (a + b + d) * (v / 4.0);
        for (k, s) in sq.iter_mut().enumerate() {
            let (x, y, z) = (a[k], b[k], d[k]);
            *s += v / 10.0 * (x * x + y * y + z * z + x * y + y * z + z * x);
        }
    }
    let centroid = Point3::ORIGIN + c * (1.0 / vol);
    let g = centroid - Point3::ORIGIN;
    let about = |k: usize| sq[k] - vol * g[k] * g[k];
    let (xx, yy, zz) = (about(0), about(1), about(2));
    Mass {
        volume: vol,
        area: m.area(),
        centroid,
        inertia: [yy + zz, xx + zz, xx + yy],
    }
}

/// The thinnest wall: from each triangle, straight in against its normal to the next surface
/// the ray leaves the solid through. The thickness and where it is.
pub fn wall(body: &Body, m: &TriMesh) -> Option<(f64, Point3)> {
    // faces meeting at an edge make a wedge there, not a wall
    let mut touching = std::collections::HashSet::new();
    for e in body.edge_ids() {
        let fs = select::faces_of_edge(body, e);
        for &a in &fs {
            for &b in &fs {
                touching.insert((a.0, b.0));
            }
        }
    }
    let tri = |t: &[u32; 3]| t.map(|i| m.positions[i as usize]);
    let normals: Vec<Option<Vec3>> = m
        .triangles
        .iter()
        .map(|t| m.triangle_normal(*t).normalize().map(|n| n.get()))
        .collect();
    let mut best: Option<(f64, Point3)> = None;
    for (i, t) in m.triangles.iter().enumerate() {
        let Some(n) = normals[i] else { continue };
        let [a, b, c] = tri(t);
        let o = a + ((b - a) + (c - a)) * (1.0 / 3.0);
        let d = -n;
        for (j, u) in m.triangles.iter().enumerate() {
            // only the far side: a face the ray leaves through looks along it
            if j == i
                || normals[j].is_none_or(|nj| nj.dot(d) <= 1e-9)
                || touching.contains(&(m.face_of[i], m.face_of[j]))
            {
                continue;
            }
            if let Some(s) = hit(o, d, tri(u))
                && s > 1e-7
                && best.is_none_or(|b| s < b.0)
            {
                best = Some((s, o + d * (s / 2.0)));
            }
        }
    }
    best
}

/// Where the ray o + s d meets the triangle, if it does.
fn hit(o: Point3, d: Vec3, [a, b, c]: [Point3; 3]) -> Option<f64> {
    let (e1, e2) = (b - a, c - a);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-14 {
        return None;
    }
    let tv = o - a;
    let u = tv.dot(p) / det;
    let q = tv.cross(e1);
    let v = d.dot(q) / det;
    if u < -1e-9 || v < -1e-9 || u + v > 1.0 + 1e-9 {
        return None;
    }
    Some(e2.dot(q) / det)
}

/// Points over an entity: a face's mesh vertices, an edge's samples, a vertex.
fn points(body: &Body, mesh: &TriMesh, e: Ent) -> Vec<Point3> {
    match e {
        Ent::Face(f) => {
            let mut p: Vec<Point3> = mesh
                .triangles
                .iter()
                .zip(&mesh.face_of)
                .filter(|x| *x.1 == f.0)
                .flat_map(|x| x.0.map(|i| mesh.positions[i as usize]))
                .collect();
            p.extend(select::samples(body, e));
            p
        }
        _ => select::samples(body, e),
    }
}

/// A direction that stands for the entity: a plane's normal or a line's direction.
fn direction(body: &Body, e: Ent) -> Option<(Vec3, bool)> {
    match e {
        Ent::Face(f) => select::plane_normal(body, f).map(|n| (n, true)),
        Ent::Edge(x) => match &body.edge(x).curve {
            Curve::Line(l) => Some((l.dir.get(), false)),
            _ => None,
        },
        Ent::Vert(_) => None,
    }
}

/// What can be said about one entity, as (name, value) pairs.
pub fn one(body: &Body, mesh: &TriMesh, e: Ent) -> Vec<(&'static str, f64)> {
    let mut out = Vec::new();
    match e {
        Ent::Face(f) => {
            let area: f64 = mesh
                .triangles
                .iter()
                .zip(&mesh.face_of)
                .filter(|x| *x.1 == f.0)
                .map(|x| {
                    let [a, b, c] = x.0.map(|i| mesh.positions[i as usize]);
                    (b - a).cross(c - a).length() / 2.0
                })
                .sum();
            out.push(("area", area));
        }
        Ent::Edge(x) => out.push(("length", select::length(body, x))),
        Ent::Vert(_) => {}
    }
    if let Some(r) = select::radius(body, e) {
        out.push(("radius", r));
    }
    // a round thing's centre is on its axis, not in the middle of a half face
    let c = select::axis(body, e).map_or_else(|| select::centre(body, e), |a| a.0);
    out.extend([("x", c.x), ("y", c.y), ("z", c.z)]);
    out
}

/// The exact gap from a planar face to a round thing standing square to it: a cylinder lying
/// along it, a circle parallel to it, a sphere.
fn plane_round(body: &Body, a: Ent, b: Ent) -> Option<f64> {
    let Ent::Face(f) = a else { return None };
    let n = select::plane_normal(body, f)?;
    let (p, dir) = select::axis(body, b)?;
    let off = (p - select::centre(body, a)).dot(n).abs();
    // a plane through a round thing (a keyway floor in a shaft) is its depth in it
    let r = select::radius(body, b).unwrap_or(0.0);
    match (b, dir) {
        (Ent::Edge(_), Some(u)) if u.cross(n).length() < 1e-9 => Some(off),
        (Ent::Face(g), Some(u))
            if u.dot(n).abs() < 1e-9 && matches!(body.face(g).surface, forge_geom::Surface::Cylinder(_)) =>
        {
            Some((off - r).abs())
        }
        (Ent::Face(_), None) => Some((off - r).abs()),
        _ => None,
    }
}

/// Distance between two entities (exact for parallel planes, else the closest of dense
/// samples), and the angle between them when both have a direction.
pub fn between(body: &Body, mesh: &TriMesh, a: Ent, b: Ent) -> Vec<(&'static str, f64)> {
    let mut out = Vec::new();
    let (da, db) = (direction(body, a), direction(body, b));
    let parallel_planes = match (da, db) {
        (Some((na, true)), Some((nb, true))) => na.cross(nb).length() < 1e-9,
        _ => false,
    };
    if parallel_planes {
        let n = da.expect("planar").0;
        out.push((
            "distance",
            (select::centre(body, b) - select::centre(body, a)).dot(n).abs(),
        ));
    } else if let Some(d) = plane_round(body, a, b).or_else(|| plane_round(body, b, a)) {
        out.push(("distance", d));
    } else {
        let (pa, pb) = (points(body, mesh, a), points(body, mesh, b));
        let mut d = f64::INFINITY;
        for p in &pa {
            for q in &pb {
                d = d.min(p.distance(*q));
            }
        }
        out.push(("distance", d));
    }
    // two holes or bosses: how far apart their axes are, or their centres
    match (select::axis(body, a), select::axis(body, b)) {
        (Some((p, Some(u))), Some((q, Some(w)))) if u.cross(w).length() < 1e-9 => {
            let d = q - p;
            out.push(("axes", (d - u * d.dot(u)).length()));
        }
        (Some((p, None)), Some((q, None))) => out.push(("centres", p.distance(q))),
        _ => {}
    }
    if let (Some((u, pu)), Some((w, pw))) = (da, db) {
        let c = u.dot(w).clamp(-1.0, 1.0);
        let deg = |x: f64| x * 180.0 / dmath::PI;
        let ang = if pu == pw {
            // two planes: the angle between them; two lines: between their directions
            let a = deg(dmath::acos(c));
            a.min(180.0 - a)
        } else {
            90.0 - deg(dmath::acos(c.abs()))
        };
        out.push(("angle", ang));
    }
    out
}

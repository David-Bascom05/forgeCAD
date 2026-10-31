//! Certificates a solid must carry beyond `forge_topo::certify`: every face's normal points out
//! of the material (by ray parity), and no two faces that share no vertex meet.

use forge_bool::{Index, Where};
use forge_geom::SurfaceGeom;
use forge_math::{Box3, Point3, Vec3};
use forge_mesh::{Quality, TriMesh, tessellate};
use forge_topo::{Body, FaceId};

#[derive(Clone, Debug, PartialEq)]
pub enum Flaw {
    /// A topological or geometric problem `forge_topo::certify` found.
    Topo(String),
    /// Just outside the face (along its normal) is material and just inside is not.
    Inverted { face: u32, at: Point3 },
    /// Both sides of the face are material, or neither is: it does not bound the solid there.
    NotBounding { face: u32, at: Point3, inside: bool },
    /// The B-rep classifier puts a point near a face on the other side from where the
    /// winding number of a fine mesh of the solid puts it.
    Misclassified { face: u32, at: Point3, index: Where },
    /// Two faces that share no vertex meet at this point (to within tolerance on both).
    Crossing { a: u32, b: u32, at: Point3 },
    /// The body does not mesh.
    Mesh(String),
}

impl Flaw {
    /// A short kind for grouping, without ids or coordinates.
    pub fn kind(&self) -> String {
        match self {
            Flaw::Topo(s) => s.split([' ', '{', '(']).next().unwrap_or("").to_string(),
            Flaw::Inverted { .. } => "Inverted".to_string(),
            Flaw::NotBounding { .. } => "NotBounding".to_string(),
            Flaw::Misclassified { .. } => "Misclassified".to_string(),
            Flaw::Crossing { .. } => "Crossing".to_string(),
            Flaw::Mesh(_) => "Mesh".to_string(),
        }
    }
}

/// Everything wrong with a body: `forge_topo::certify`, then, on a body that passes it, face
/// orientation and crossings.
pub fn certify(body: &Body, chord: f64) -> Vec<Flaw> {
    let body = body.compact();
    let out: Vec<Flaw> = forge_topo::certify(&body)
        .into_iter()
        .map(|p| Flaw::Topo(format!("{p:?}")))
        .collect();
    if !out.is_empty() {
        return out;
    }
    geometric(&body, chord)
}

/// Orientation and crossings alone, on a body whatever `forge_topo::certify` says of it.
pub fn geometric(body: &Body, chord: f64) -> Vec<Flaw> {
    let body = body.compact();
    let mut out = Vec::new();
    let mesh = match tessellate(
        &body,
        &Quality {
            chord,
            max_angle: 0.5,
        },
    ) {
        Ok(m) => m,
        Err(e) => return vec![Flaw::Mesh(format!("{e:?}"))],
    };
    let index = Index::new(&body);
    orientation(&body, &index, &mesh, &mut out);
    crossings(&body, &index, &mesh, chord, &mut out);
    out
}

/// At each face's largest triangle, a point on the surface under its centroid, stepped a little
/// out and in along the face's normal: out must be outside the solid, in inside. A face that
/// fails is looked at again further off, against a fine mesh's winding number too, to tell a
/// wrong face from a wrong classifier.
fn orientation(body: &Body, index: &Index, mesh: &TriMesh, out: &mut Vec<Flaw>) {
    // each face's triangles, largest first
    let mut by_face: Vec<Vec<(f64, usize)>> = vec![Vec::new(); body.faces.len()];
    for (t, tri) in mesh.triangles.iter().enumerate() {
        by_face[mesh.face_of[t] as usize].push((mesh.triangle_normal(*tri).length(), t));
    }
    let mut fine: Option<TriMesh> = None;
    for f in body.face_ids() {
        let tris = &mut by_face[f.index()];
        tris.sort_by(|a, b| b.0.total_cmp(&a.0));
        let surface = &body.face(f).surface;
        // a point on the surface under a triangle's centroid, inside the face
        let on_face = tris.iter().take(8).find_map(|&(_, t)| {
            let [a, b, c] = mesh.triangles[t].map(|i| mesh.positions[i as usize]);
            let centroid = Point3::new(
                (a.x + b.x + c.x) / 3.0,
                (a.y + b.y + c.y) / 3.0,
                (a.z + b.z + c.z) / 3.0,
            );
            let uv = surface.project(centroid);
            let p = surface.eval(uv);
            (index.point_in_face(f, p) == Where::Inside).then_some((p, uv))
        });
        let Some((p, uv)) = on_face else { continue };
        let Some(n) = body.face_normal(f, uv) else {
            continue;
        };
        let sides = |step: f64| (index.classify(p + n * step), index.classify(p + n * -step));
        // a verdict must hold at two steps, so a thin wall or a nearby edge is not taken for one
        let (near, far) = (sides(1e-4), sides(4e-4));
        let right = (Where::Outside, Where::Inside);
        if near == right || far == right || near != far || near.0 == Where::On || near.1 == Where::On {
            continue;
        }
        // another face within the far step: a wall thinner than the steps, nothing to say
        if crowded(body, index, f, p, 0.05) {
            continue;
        }
        // the winding number of a fine mesh at a step well past its chord says which it is
        let fine = fine.get_or_insert_with(|| {
            tessellate(
                body,
                &Quality {
                    chord: 0.002,
                    max_angle: 0.2,
                },
            )
            .unwrap_or_default()
        });
        let step = 0.02;
        let (o, i) = (p + n * step, p + n * -step);
        // material wherever the mesh winds round the point at all (an inside-out solid winds -1)
        let mesh_side = (winding(fine, o).abs() > 0.5, winding(fine, i).abs() > 0.5);
        let index_side = (index.classify(o), index.classify(i));
        let face = f.0;
        if mesh_side == (false, true) {
            let (at, w) = if index_side.0 != Where::Outside {
                (o, index_side.0)
            } else {
                (i, index_side.1)
            };
            if w != Where::On {
                out.push(Flaw::Misclassified { face, at, index: w });
            }
        } else if mesh_side == (true, false) {
            out.push(Flaw::Inverted { face, at: p });
        } else {
            out.push(Flaw::NotBounding {
                face,
                at: p,
                inside: mesh_side.0,
            });
        }
    }
}

/// Whether a face other than `f` passes within `r` of `p`.
fn crowded(body: &Body, index: &Index, f: FaceId, p: Point3, r: f64) -> bool {
    body.face_ids().filter(|&g| g != f).any(|g| {
        let s = &body.face(g).surface;
        if !body.face_box(g).grow(r).contains(p) || s.distance(p) > r {
            return false;
        }
        index.point_in_face(g, s.eval(s.project(p))) != Where::Outside
    })
}

/// The generalised winding number of a closed mesh about `p` (1 inside, 0 outside), from the
/// solid angles its triangles subtend (Van Oosterom and Strackee).
fn winding(m: &TriMesh, p: Point3) -> f64 {
    let mut sum = 0.0;
    for t in &m.triangles {
        let [a, b, c] = t.map(|i| m.positions[i as usize] - p);
        let (la, lb, lc) = (a.length(), b.length(), c.length());
        let num = a.dot(b.cross(c));
        let den = la * lb * lc + a.dot(b) * lc + a.dot(c) * lb + b.dot(c) * la;
        sum += 2.0 * forge_math::dmath::atan2(num, den);
    }
    sum / (4.0 * forge_math::dmath::PI)
}

/// Mesh triangles of faces that share no vertex, tested pairwise where their boxes meet; a
/// crossing found there is kept only if the exact surfaces meet nearby inside both faces.
fn crossings(body: &Body, index: &Index, mesh: &TriMesh, chord: f64, out: &mut Vec<Flaw>) {
    let n = body.faces.len();
    let mut verts: Vec<Vec<u32>> = vec![Vec::new(); n];
    for f in body.face_ids() {
        for &l in &body.face(f).loops {
            for c in body.lp(l).coedges() {
                verts[f.index()].push(body.coedge_start(c).0);
            }
        }
        verts[f.index()].sort_unstable();
        verts[f.index()].dedup();
    }
    let mut tris: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut boxes = vec![Box3::from_points([]); n];
    for (t, tri) in mesh.triangles.iter().enumerate() {
        let f = mesh.face_of[t] as usize;
        tris[f].push(t);
        for &i in tri {
            boxes[f].include(mesh.positions[i as usize]);
        }
    }
    let corners = |t: usize| mesh.triangles[t].map(|i| mesh.positions[i as usize]);
    let tri_box = |t: usize| Box3::from_points(corners(t));
    for fa in 0..n {
        for fb in fa + 1..n {
            if tris[fa].is_empty() || tris[fb].is_empty() || !boxes[fa].overlaps(&boxes[fb]) {
                continue;
            }
            if verts[fa].iter().any(|v| verts[fb].binary_search(v).is_ok()) {
                continue;
            }
            let bb: Vec<(usize, Box3)> = tris[fb]
                .iter()
                .map(|&t| (t, tri_box(t)))
                .filter(|(_, b)| b.overlaps(&boxes[fa]))
                .collect();
            'pair: for &ta in &tris[fa] {
                let ba = tri_box(ta);
                if !ba.overlaps(&boxes[fb]) {
                    continue;
                }
                for (tb, b) in &bb {
                    if !ba.overlaps(b) {
                        continue;
                    }
                    if let Some(q) = tri_tri(corners(ta), corners(*tb)) {
                        let (a, b) = (FaceId(fa as u32), FaceId(fb as u32));
                        if let Some(at) = confirm(body, index, a, b, q, chord) {
                            out.push(Flaw::Crossing {
                                a: fa as u32,
                                b: fb as u32,
                                at,
                            });
                            break 'pair;
                        }
                    }
                }
            }
        }
    }
}

/// A point near `q` on both faces' surfaces (by alternating projection), inside both faces.
fn confirm(body: &Body, index: &Index, a: FaceId, b: FaceId, q: Point3, chord: f64) -> Option<Point3> {
    let (sa, sb) = (&body.face(a).surface, &body.face(b).surface);
    let mut x = q;
    for _ in 0..64 {
        x = sa.eval(sa.project(x));
        x = sb.eval(sb.project(x));
    }
    let tol = forge_math::tol::LINEAR;
    let on = sa.distance(x) <= tol && sb.distance(x) <= tol && x.distance(q) <= 4.0 * chord;
    (on && index.point_in_face(a, x) == Where::Inside && index.point_in_face(b, x) == Where::Inside)
        .then_some(x)
}

/// Where two triangles cross: an edge of one through the other (coplanar contact is ignored).
fn tri_tri(a: [Point3; 3], b: [Point3; 3]) -> Option<Point3> {
    for (s, t) in [(a, b), (b, a)] {
        for i in 0..3 {
            if let Some(p) = seg_tri(s[i], s[(i + 1) % 3], t) {
                return Some(p);
            }
        }
    }
    None
}

fn seg_tri(p: Point3, q: Point3, t: [Point3; 3]) -> Option<Point3> {
    let (e1, e2) = (t[1] - t[0], t[2] - t[0]);
    let d: Vec3 = q - p;
    let h = d.cross(e2);
    let det = e1.dot(h);
    if det.abs() <= 1e-14 * d.length() * e1.length() * e2.length() {
        return None;
    }
    let s = p - t[0];
    let u = s.dot(h) / det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let qv = s.cross(e1);
    let v = d.dot(qv) / det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let w = e2.dot(qv) / det;
    (0.0..=1.0).contains(&w).then(|| p + d * w)
}

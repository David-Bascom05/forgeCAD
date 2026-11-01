//! Triangles per face, kept across edits: a face whose surface and boundary did not change
//! keeps its triangles, so after an edit only the faces it touched are meshed again.

use forge_geom::{Surface, SurfaceGeom};
use forge_math::{Box3, Point2, Point3};
use forge_mesh::tessellate::tessellate_tolerant;
use forge_mesh::{Quality, TriMesh};
use forge_topo::{Body, FaceId, Name};
use std::collections::HashMap;
use std::sync::Arc;

pub struct FaceMesh {
    pub mesh: TriMesh,
    /// The face's share of the solid's volume (divergence theorem) and its area.
    pub volume: f64,
    pub area: f64,
    /// Tight, from the triangles (a face's own box may be loose).
    pub bbox: Box3,
}

#[derive(Default)]
pub struct Cache {
    faces: HashMap<Name, Option<Arc<FaceMesh>>>,
}

/// What a face's triangles depend on: its name, sense, surface (when it has a formula), and
/// every boundary edge's name, direction, range and ends.
pub fn key(body: &Body, f: FaceId) -> Name {
    let face = body.face(f);
    let mut k = face.name.with(face.same_sense as u64);
    if !matches!(face.surface, Surface::Procedural(_)) {
        // a surface with a formula is pinned down by a few of its points
        for uv in [
            Point2::new(0.0, 0.0),
            Point2::new(0.7, 1.3),
            Point2::new(2.1, -0.4),
        ] {
            let p = face.surface.eval(uv);
            k = k.with(p.x.to_bits()).with(p.y.to_bits()).with(p.z.to_bits());
        }
    }
    for &l in &face.loops {
        k = k.with(0x100);
        for c in body.lp(l).coedges() {
            let co = body.coedge(c);
            let e = body.edge(co.edge);
            k = k
                .with_name(e.name)
                .with(co.forward as u64)
                .with(e.range.lo.to_bits())
                .with(e.range.hi.to_bits());
            for v in [e.start, e.end] {
                let p = body.vertex(v).point;
                k = k.with(p.x.to_bits()).with(p.y.to_bits()).with(p.z.to_bits());
            }
        }
    }
    k
}

/// The face alone, as a body of its own (its edges and vertices copied), to be meshed.
pub fn alone(body: &Body, f: FaceId) -> Body {
    let face = body.face(f);
    let mut b = Body::new();
    b.from_file = body.from_file;
    let shell = b.add_shell();
    let nf = b.add_face(shell, face.surface.clone(), face.same_sense, face.name);
    let (mut vs, mut es) = (HashMap::new(), HashMap::new());
    for &l in &face.loops {
        let mut uses = Vec::new();
        for c in body.lp(l).coedges() {
            let co = body.coedge(c);
            let e = body.edge(co.edge);
            let mut vert = |v: forge_topo::VertexId, b: &mut Body| {
                *vs.entry(v).or_insert_with(|| {
                    let x = body.vertex(v);
                    b.add_tolerant_vertex(x.point, x.tol, x.name)
                })
            };
            let (s, t) = (vert(e.start, &mut b), vert(e.end, &mut b));
            let ne = *es
                .entry(co.edge)
                .or_insert_with(|| b.add_edge(e.curve.clone(), e.range, s, t, e.name));
            uses.push((ne, co.forward));
        }
        b.add_loop(nf, &uses);
    }
    b
}

fn mesh_face(body: &Body, f: FaceId) -> Option<Arc<FaceMesh>> {
    let q = Quality {
        chord: 0.05,
        ..Quality::default()
    };
    let mut mesh = tessellate_tolerant(&alone(body, f), &q).ok()?;
    let (mut volume, mut area) = (0.0, 0.0);
    for t in &mesh.triangles {
        let [a, b, c] = t.map(|i| mesh.positions[i as usize] - Point3::ORIGIN);
        volume += a.dot(b.cross(c)) / 6.0;
        area += (b - a).cross(c - a).length() / 2.0;
    }
    mesh.face_of.iter_mut().for_each(|x| *x = f.0);
    let bbox = mesh.bbox();
    Some(Arc::new(FaceMesh {
        mesh,
        volume,
        area,
        bbox,
    }))
}

impl Cache {
    /// Every live face's triangles (None where meshing failed), meshing only faces not seen
    /// before; faces the body no longer has are forgotten.
    pub fn faces(&mut self, body: &Body) -> Vec<(FaceId, Option<Arc<FaceMesh>>)> {
        let mut next = HashMap::with_capacity(self.faces.len());
        let out = body
            .face_ids()
            .map(|f| {
                let k = key(body, f);
                let m = match self.faces.remove(&k).or_else(|| next.get(&k).cloned()) {
                    Some(m) => m,
                    None => mesh_face(body, f),
                };
                next.insert(k, m.clone());
                (
                    f,
                    m.map(|m| {
                        if m.mesh.face_of.first() == Some(&f.0) {
                            m
                        } else {
                            // the same face under another id
                            let mut mesh = m.mesh.clone();
                            mesh.face_of.iter_mut().for_each(|x| *x = f.0);
                            Arc::new(FaceMesh {
                                mesh,
                                volume: m.volume,
                                area: m.area,
                                bbox: m.bbox,
                            })
                        }
                    }),
                )
            })
            .collect();
        self.faces = next;
        out
    }
}

/// All the faces' triangles as one mesh (face seams not shared).
pub fn join(faces: &[(FaceId, Option<Arc<FaceMesh>>)]) -> TriMesh {
    let mut out = TriMesh::default();
    for m in faces.iter().filter_map(|x| x.1.as_ref()) {
        let base = out.positions.len() as u32;
        out.positions.extend_from_slice(&m.mesh.positions);
        out.triangles
            .extend(m.mesh.triangles.iter().map(|t| t.map(|i| i + base)));
        out.face_of.extend_from_slice(&m.mesh.face_of);
    }
    out
}

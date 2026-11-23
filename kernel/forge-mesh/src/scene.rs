//! What a mesh file holds: named objects, each a closed triangle mesh with shared vertices, their
//! materials, and the build: which object goes where. Made from a body (one object per solid) or
//! from an assembly's parts and poses; written as STL, OBJ or 3MF (`stl`, `obj`, `threemf`).

use crate::mesh::TriMesh;
use crate::tessellate::{Quality, TessError, tessellate_tolerant};
use forge_geom::SurfaceGeom;
use forge_math::{Frame, Point3, Vec3};
use forge_topo::{Body, FaceId};
use std::collections::BTreeMap;
use std::ops::Range;

/// A display colour and the name a printer maps to its own material.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub name: String,
    pub rgb: [u8; 3],
}

/// One closed mesh: vertices shared between the triangles that meet at them, triangles
/// counter-clockwise seen from outside.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Object {
    pub name: String,
    pub positions: Vec<Point3>,
    pub triangles: Vec<[u32; 3]>,
    /// The body face each triangle came from (empty when not known).
    pub face_of: Vec<u32>,
    /// Unit normals of the exact surface, and each triangle's corners' indices into them (both
    /// empty when not asked for).
    pub normals: Vec<Vec3>,
    pub corner_normals: Vec<[u32; 3]>,
    /// Index into the scene's materials.
    pub material: Option<usize>,
}

/// A placement of an object: a 3MF build item. `transform` is 3MF's row-major affine matrix
/// "m00 m01 m02 m10 m11 m12 m20 m21 m22 m30 m31 m32": a point p goes to p * M.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub object: usize,
    pub transform: [f64; 12],
    pub name: String,
}

pub const IDENTITY: [f64; 12] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    /// The model's title (3MF metadata, STL header).
    pub title: String,
    pub objects: Vec<Object>,
    pub materials: Vec<Material>,
    pub items: Vec<Item>,
}

/// A frame's placement as a 3MF matrix: its axes as the first three rows, its origin the last.
pub fn frame_matrix(f: &Frame) -> [f64; 12] {
    let (x, y, z, o) = (f.x().get(), f.y().get(), f.z().get(), f.origin);
    [x.x, x.y, x.z, y.x, y.y, y.z, z.x, z.y, z.z, o.x, o.y, o.z]
}

pub fn apply(m: &[f64; 12], p: Point3) -> Point3 {
    Point3::new(
        p.x * m[0] + p.y * m[3] + p.z * m[6] + m[9],
        p.x * m[1] + p.y * m[4] + p.z * m[7] + m[10],
        p.x * m[2] + p.y * m[5] + p.z * m[8] + m[11],
    )
}

fn apply_vec(m: &[f64; 12], v: Vec3) -> Vec3 {
    Vec3::new(
        v.x * m[0] + v.y * m[3] + v.z * m[6],
        v.x * m[1] + v.y * m[4] + v.z * m[7],
        v.x * m[2] + v.y * m[5] + v.z * m[8],
    )
}

fn det(m: &[f64; 12]) -> f64 {
    m[0] * (m[4] * m[8] - m[5] * m[7]) - m[1] * (m[3] * m[8] - m[5] * m[6])
        + m[2] * (m[3] * m[7] - m[4] * m[6])
}

/// What a mesh is, checked: closed and consistently oriented when `open_edges` is 0, a manifold
/// surface when `pinched` is 0 as well, facing out when `volume` is positive.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MeshCheck {
    pub triangles: usize,
    pub vertices: usize,
    /// Directed edges not matched by exactly one opposite edge.
    pub open_edges: usize,
    /// Vertices where the surface pinches (more than one fan of triangles round them).
    pub pinched: usize,
    pub degenerate: usize,
    pub volume: f64,
    pub area: f64,
}

impl MeshCheck {
    /// Closed, consistently oriented and facing out: no open edge, at least a tetrahedron's
    /// triangles, positive volume. Says nothing of pinches (`manifold`).
    pub fn watertight(&self) -> bool {
        self.open_edges == 0 && self.triangles >= 4 && self.volume > 0.0
    }

    /// A manifold surface: every edge between exactly two triangles (`open_edges`) and one fan of
    /// triangles round every vertex (`pinched`).
    pub fn manifold(&self) -> bool {
        self.open_edges == 0 && self.pinched == 0
    }
}

impl Object {
    pub fn trimesh(&self) -> TriMesh {
        TriMesh {
            positions: self.positions.clone(),
            triangles: self.triangles.clone(),
            face_of: self.face_of.clone(),
            ..TriMesh::default()
        }
    }

    pub fn check(&self) -> MeshCheck {
        let m = self.trimesh();
        MeshCheck {
            triangles: m.triangles.len(),
            vertices: m.positions.len(),
            open_edges: m.open_edges().len(),
            pinched: m.pinched_vertices().len(),
            degenerate: m.degenerate_triangles(0.0),
            volume: m.volume(),
            area: m.area(),
        }
    }

    /// The object moved by `m` (a rigid placement or any matrix; a mirroring one has its
    /// triangles turned so they still face out).
    pub fn transformed(&self, m: &[f64; 12]) -> Object {
        let flip = det(m) < 0.0;
        let mut o = self.clone();
        o.positions = self.positions.iter().map(|&p| apply(m, p)).collect();
        o.normals = self
            .normals
            .iter()
            .map(|&n| {
                let v = apply_vec(m, n);
                v.normalize().map_or(v, |u| u.get())
            })
            .collect();
        if flip {
            for t in &mut o.triangles {
                t.swap(1, 2);
            }
            for t in &mut o.corner_normals {
                t.swap(1, 2);
            }
        }
        o
    }
}

impl Scene {
    /// Every item as an object of its own where it is placed, each placed where it is (identity):
    /// what STL and OBJ, which have no placements, write.
    pub fn flatten(&self) -> Scene {
        let mut objects = Vec::with_capacity(self.items.len());
        let mut items = Vec::with_capacity(self.items.len());
        for it in &self.items {
            let mut o = if it.transform == IDENTITY {
                self.objects[it.object].clone()
            } else {
                self.objects[it.object].transformed(&it.transform)
            };
            if !it.name.is_empty() {
                o.name = it.name.clone();
            }
            items.push(Item {
                object: objects.len(),
                transform: IDENTITY,
                name: it.name.clone(),
            });
            objects.push(o);
        }
        Scene {
            title: self.title.clone(),
            objects,
            materials: self.materials.clone(),
            items,
        }
    }

    /// Each item's check, as placed.
    pub fn checks(&self) -> Vec<MeshCheck> {
        self.items
            .iter()
            .map(|it| {
                let mut c = self.objects[it.object].check();
                c.volume *= det(&it.transform).abs();
                c
            })
            .collect()
    }

    /// A scene of objects each placed once, where it is.
    pub fn of_objects(title: &str, objects: Vec<Object>, materials: Vec<Material>) -> Scene {
        let items = objects
            .iter()
            .enumerate()
            .map(|(k, _)| Item {
                object: k,
                transform: IDENTITY,
                name: String::new(),
            })
            .collect();
        Scene {
            title: title.to_string(),
            objects,
            materials,
            items,
        }
    }
}

/// How a body is cut into objects: one per solid (a name and its faces, as a part's solids lie in
/// its joined body), or with none given one per connected piece of the mesh, named `name`,
/// `name.2`, `name.3`, ...
pub struct Solids<'a> {
    pub name: &'a str,
    pub solids: &'a [(String, Range<u32>)],
}

/// A body's mesh within `q`, cut into its solids: each object's own vertices (shared by its
/// triangles; neighbouring faces meet on the very same edge points, which the tessellator cuts
/// once per edge), and with `normals` the exact surface's normal at each corner.
pub fn body_objects(body: &Body, q: &Quality, by: &Solids, normals: bool) -> Result<Vec<Object>, TessError> {
    // the faces the solids name are the body's own: a body with dead entities is compacted,
    // renumbering them, and then its pieces are found from the mesh instead
    let compacted;
    let (b, solids) = if body.dead > 0 {
        compacted = body.compact();
        (&compacted, &[][..])
    } else {
        (body, by.solids)
    };
    let mesh = tessellate_tolerant(b, q)?;
    let nf = b.faces.len();
    let mut group = vec![usize::MAX; nf];
    for (k, (_, r)) in solids.iter().enumerate() {
        for f in r.clone() {
            if let Some(g) = group.get_mut(f as usize) {
                *g = k;
            }
        }
    }
    let covered = !solids.is_empty() && mesh.face_of.iter().all(|&f| group[f as usize] != usize::MAX);
    let (of_tri, names): (Vec<usize>, Vec<String>) = if covered {
        (
            mesh.face_of.iter().map(|&f| group[f as usize]).collect(),
            solids.iter().map(|s| s.0.clone()).collect(),
        )
    } else {
        let (of, n) = pieces(&mesh);
        let names = (0..n)
            .map(|k| {
                if k == 0 {
                    by.name.to_string()
                } else {
                    format!("{}.{}", by.name, k + 1)
                }
            })
            .collect();
        (of, names)
    };
    let mut objects: Vec<Object> = names
        .into_iter()
        .map(|name| Object {
            name,
            ..Object::default()
        })
        .collect();
    // each object's vertices numbered in the order its triangles first use them
    let mut local: BTreeMap<(usize, u32), u32> = BTreeMap::new();
    let mut nkeys: Vec<BTreeMap<[u64; 3], u32>> = vec![BTreeMap::new(); objects.len()];
    for (t, (&tri, &f)) in mesh.triangles.iter().zip(&mesh.face_of).enumerate() {
        let o = &mut objects[of_tri[t]];
        let mut out = [0u32; 3];
        for (k, &v) in tri.iter().enumerate() {
            out[k] = *local.entry((of_tri[t], v)).or_insert_with(|| {
                o.positions.push(mesh.positions[v as usize]);
                o.positions.len() as u32 - 1
            });
        }
        o.triangles.push(out);
        o.face_of.push(f);
        if normals {
            let flat = mesh.triangle_normal(tri).normalize().map(|u| u.get());
            let face = b.face(FaceId(f));
            let mut cn = [0u32; 3];
            for (k, &v) in tri.iter().enumerate() {
                let p = mesh.positions[v as usize];
                let n = face
                    .surface
                    .normal(face.surface.project(p))
                    .map(|n| if face.same_sense { n.get() } else { -n.get() })
                    // a surface's normal turned against its triangle (a pole, a fold) is not
                    // what a viewer should shade with
                    .filter(|n| flat.is_none_or(|t| n.dot(t) > 0.0))
                    .or(flat)
                    .unwrap_or(Vec3::ZERO);
                let key = [n.x.to_bits(), n.y.to_bits(), n.z.to_bits()];
                let nk = &mut nkeys[of_tri[t]];
                cn[k] = *nk.entry(key).or_insert_with(|| {
                    o.normals.push(n);
                    o.normals.len() as u32 - 1
                });
            }
            o.corner_normals.push(cn);
        }
    }
    objects.retain(|o| !o.triangles.is_empty());
    Ok(objects)
}

/// Each triangle's connected piece (joined through shared vertices), pieces numbered in the order
/// their first triangle comes; and how many.
fn pieces(m: &TriMesh) -> (Vec<usize>, usize) {
    let mut parent: Vec<u32> = (0..m.positions.len() as u32).collect();
    fn find(p: &mut [u32], mut x: u32) -> u32 {
        while p[x as usize] != x {
            p[x as usize] = p[p[x as usize] as usize];
            x = p[x as usize];
        }
        x
    }
    for &[a, b, c] in &m.triangles {
        for (x, y) in [(a, b), (b, c)] {
            let (rx, ry) = (find(&mut parent, x), find(&mut parent, y));
            if rx != ry {
                parent[rx.max(ry) as usize] = rx.min(ry);
            }
        }
    }
    let mut id: BTreeMap<u32, usize> = BTreeMap::new();
    let mut of = Vec::with_capacity(m.triangles.len());
    for t in &m.triangles {
        let r = find(&mut parent, t[0]);
        let n = id.len();
        of.push(*id.entry(r).or_insert(n));
    }
    (of, id.len())
}

/// A number as the text formats write it: to 1e-6 mm (a nanometre), trailing zeros dropped, never
/// "-0". Rust's own formatting, so the same on every platform.
pub fn num(x: f64) -> String {
    let mut s = format!("{x:.6}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".into();
    }
    s
}

/// Objects from triangles given by their corners (an STL's), corners at the very same point made
/// one vertex.
pub fn weld(name: &str, corners: &[[Point3; 3]]) -> Object {
    let mut at: BTreeMap<[u64; 3], u32> = BTreeMap::new();
    let mut o = Object {
        name: name.to_string(),
        ..Object::default()
    };
    for tri in corners {
        let mut t = [0u32; 3];
        for (k, p) in tri.iter().enumerate() {
            let key = [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()];
            t[k] = *at.entry(key).or_insert_with(|| {
                o.positions.push(*p);
                o.positions.len() as u32 - 1
            });
        }
        o.triangles.push(t);
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tetra(o: Point3) -> Object {
        let p = |x: f64, y: f64, z: f64| Point3::new(o.x + x, o.y + y, o.z + z);
        Object {
            positions: vec![
                p(0.0, 0.0, 0.0),
                p(1.0, 0.0, 0.0),
                p(0.0, 1.0, 0.0),
                p(0.0, 0.0, 1.0),
            ],
            triangles: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2]],
            ..Object::default()
        }
    }

    #[test]
    fn a_pinch_is_closed_but_not_manifold() {
        let one = tetra(Point3::ORIGIN).check();
        assert!(one.watertight() && one.manifold(), "{one:?}");
        // a second tetrahedron below the first, sharing only its vertex 0
        let mut two = tetra(Point3::ORIGIN);
        let b = tetra(Point3::ORIGIN)
            .transformed(&[-1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0]);
        two.positions.extend(&b.positions[1..]);
        let map = |i: u32| if i == 0 { 0 } else { i + 3 };
        two.triangles.extend(b.triangles.iter().map(|t| t.map(map)));
        let c = two.check();
        assert_eq!(c.open_edges, 0, "no open edge sees it");
        assert!(c.watertight());
        assert_eq!(c.pinched, 1);
        assert!(!c.manifold());
    }

    #[test]
    fn numbers() {
        assert_eq!(num(1.0), "1");
        assert_eq!(num(-0.0000001), "0");
        assert_eq!(num(2.5), "2.5");
        assert_eq!(num(-12.3456789), "-12.345679");
        assert_eq!(num(100.0), "100");
    }

    #[test]
    fn frames_as_matrices() {
        let f = Frame::from_z(
            Point3::new(1.0, 2.0, 3.0),
            forge_math::Vec3::X.normalize().unwrap(),
        );
        let m = frame_matrix(&f);
        let p = Point3::new(0.5, -1.0, 2.0);
        let a = apply(&m, p);
        let b = f.point_to_world(p);
        assert!((a - b).length() < 1e-12);
    }
}

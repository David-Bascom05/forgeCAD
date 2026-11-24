//! A scene as the tracer holds it: each part's finest triangles under its own hierarchy, the
//! placements (each a part, a pose, an appearance) under one more, all flattened into the arrays
//! the GPU reads. The CPU mirror traces the same arrays.

use super::bvh::{self, Aabb, Bvh, Node};
use super::v3::{V3, v3};
use crate::mesh::{PartMesh, VERTEX};

/// Rows of a pose: local to world, 3x4.
pub type Rows = [[f32; 4]; 3];

/// A placement for the tracer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instance {
    pub part: u32,
    pub rows: Rows,
    pub material: u32,
}

/// A triangle as the GPU reads it: its first corner, two edges, the corners' normals
/// (octahedral, snorm16x2) in the three w's.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tri {
    pub p0: V3,
    pub e1: V3,
    pub e2: V3,
    pub n: [u32; 3],
}

/// A placement as the GPU reads it: world to local rows, its hierarchy's root, its appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuInstance {
    pub to_local: Rows,
    pub root: u32,
    pub material: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Built {
    /// Every part's nodes, then the placements' (from `top`).
    pub nodes: Vec<Node>,
    pub top: u32,
    pub tris: Vec<Tri>,
    pub instances: Vec<GpuInstance>,
    pub bounds: Option<Aabb>,
}

/// An octahedral normal, as the mesh stores it, back to a unit vector.
pub fn unoct(w: u32) -> V3 {
    let x = (w & 0xffff) as u16 as i16 as f32 / 32767.0;
    let y = (w >> 16) as u16 as i16 as f32 / 32767.0;
    let z = 1.0 - x.abs() - y.abs();
    let t = (-z).max(0.0);
    let x = x + if x >= 0.0 { -t } else { t };
    let y = y + if y >= 0.0 { -t } else { t };
    v3(x, y, z).norm()
}

fn place(r: &Rows, p: V3) -> V3 {
    v3(
        r[0][0] * p.x + r[0][1] * p.y + r[0][2] * p.z + r[0][3],
        r[1][0] * p.x + r[1][1] * p.y + r[1][2] * p.z + r[1][3],
        r[2][0] * p.x + r[2][1] * p.y + r[2][2] * p.z + r[2][3],
    )
}

/// The inverse of an affine 3x4.
pub fn invert(r: &Rows) -> Rows {
    let m = |i: usize, j: usize| r[i][j] as f64;
    let det = m(0, 0) * (m(1, 1) * m(2, 2) - m(1, 2) * m(2, 1)) - m(0, 1) * (m(1, 0) * m(2, 2) - m(1, 2) * m(2, 0))
        + m(0, 2) * (m(1, 0) * m(2, 1) - m(1, 1) * m(2, 0));
    let inv_det = if det.abs() < 1e-30 { 0.0 } else { 1.0 / det };
    let mut a = [[0.0f64; 3]; 3];
    for (i, row) in a.iter_mut().enumerate() {
        for (j, x) in row.iter_mut().enumerate() {
            let (i1, i2) = ((j + 1) % 3, (j + 2) % 3);
            let (j1, j2) = ((i + 1) % 3, (i + 2) % 3);
            *x = (m(i1, j1) * m(i2, j2) - m(i1, j2) * m(i2, j1)) * inv_det;
        }
    }
    let t = [m(0, 3), m(1, 3), m(2, 3)];
    let mut out = [[0.0f32; 4]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = a[i][j] as f32;
        }
        out[i][3] = -(a[i][0] * t[0] + a[i][1] * t[1] + a[i][2] * t[2]) as f32;
    }
    out
}

/// A part's finest triangles: corners and the normals' words.
pub fn part_triangles(mesh: &PartMesh) -> Vec<([V3; 3], [u32; 3])> {
    let Some(lod) = mesh.lods.first() else {
        return Vec::new();
    };
    let vert = |i: u32| {
        let b = &lod.vertices[i as usize * VERTEX..][..VERTEX];
        let f = |k: usize| f32::from_le_bytes(b[k * 4..k * 4 + 4].try_into().unwrap());
        (v3(f(0), f(1), f(2)), u32::from_le_bytes(b[12..16].try_into().unwrap()))
    };
    lod.indices
        .as_chunks::<12>()
        .0
        .iter()
        .map(|c| {
            let i = |k: usize| u32::from_le_bytes(c[k * 4..k * 4 + 4].try_into().unwrap());
            let (a, b, d) = (vert(i(0)), vert(i(1)), vert(i(2)));
            ([a.0, b.0, d.0], [a.1, b.1, d.1])
        })
        .collect()
}

impl Built {
    /// The scene of these parts at these placements.
    pub fn new(parts: &[PartMesh], instances: &[Instance]) -> Built {
        let mut out = Built::default();
        let mut roots = vec![u32::MAX; parts.len()];
        let mut local_boxes = vec![Aabb::EMPTY; parts.len()];
        for inst in instances {
            let p = inst.part as usize;
            if roots[p] != u32::MAX || p >= parts.len() {
                continue;
            }
            let tris = part_triangles(&parts[p]);
            if tris.is_empty() {
                continue;
            }
            let boxes: Vec<Aabb> = tris
                .iter()
                .map(|(c, _)| {
                    let mut b = Aabb::EMPTY;
                    c.iter().for_each(|&q| b.grow(q));
                    b
                })
                .collect();
            let tree = Bvh::build(&boxes, bvh::LEAF);
            let (node0, tri0) = (out.nodes.len() as u32, out.tris.len() as u32);
            for &k in &tree.order {
                let (c, n) = tris[k as usize];
                out.tris.push(Tri {
                    p0: c[0],
                    e1: c[1] - c[0],
                    e2: c[2] - c[0],
                    n,
                });
            }
            for n in &tree.nodes {
                let mut n = *n;
                n.first += if n.count > 0 { tri0 } else { node0 };
                out.nodes.push(n);
            }
            roots[p] = node0;
            local_boxes[p] = Aabb {
                min: tree.nodes[0].min,
                max: tree.nodes[0].max,
            };
        }
        // the placements' world boxes: the local box's corners placed
        let mut kept = Vec::new();
        let mut boxes = Vec::new();
        for inst in instances {
            let p = inst.part as usize;
            if p >= parts.len() || roots[p] == u32::MAX {
                continue;
            }
            let lb = local_boxes[p];
            let mut b = Aabb::EMPTY;
            for k in 0..8 {
                let c = v3(
                    if k & 1 == 0 { lb.min.x } else { lb.max.x },
                    if k & 2 == 0 { lb.min.y } else { lb.max.y },
                    if k & 4 == 0 { lb.min.z } else { lb.max.z },
                );
                b.grow(place(&inst.rows, c));
            }
            boxes.push(b);
            kept.push(GpuInstance {
                to_local: invert(&inst.rows),
                root: roots[p],
                material: inst.material,
            });
        }
        if kept.is_empty() {
            return out;
        }
        let mut all = Aabb::EMPTY;
        boxes.iter().for_each(|b| all.join(b));
        out.bounds = Some(all);
        let tree = Bvh::build(&boxes, 1);
        out.instances = tree.order.iter().map(|&k| kept[k as usize]).collect();
        let top = out.nodes.len() as u32;
        for n in &tree.nodes {
            let mut n = *n;
            if n.count == 0 {
                n.first += top;
            }
            out.nodes.push(n);
        }
        out.top = top;
        out
    }

    /// The nearest hit: distance, triangle, barycentrics, instance.
    pub fn trace(&self, o: V3, d: V3, t_max: f32) -> Option<Hit> {
        if self.instances.is_empty() {
            return None;
        }
        let mut best: Option<Hit> = None;
        bvh::closest(&self.nodes, self.top, o, d, t_max, |k, t_max| {
            let inst = &self.instances[k as usize];
            let r = &inst.to_local;
            let lo = place(r, o);
            let ld = v3(
                r[0][0] * d.x + r[0][1] * d.y + r[0][2] * d.z,
                r[1][0] * d.x + r[1][1] * d.y + r[1][2] * d.z,
                r[2][0] * d.x + r[2][1] * d.y + r[2][2] * d.z,
            );
            let mut uv = (0.0, 0.0);
            let hit = bvh::closest(&self.nodes, inst.root, lo, ld, t_max, |j, t_max| {
                let tri = &self.tris[j as usize];
                let h = bvh::hit_triangle(lo, ld, tri.p0, tri.e1, tri.e2, t_max)?;
                uv = (h.1, h.2);
                Some(h.0)
            });
            hit.map(|(t, j)| {
                best = Some(Hit {
                    t,
                    tri: j,
                    u: uv.0,
                    v: uv.1,
                    instance: k,
                });
                t
            })
        })?;
        best
    }

    /// The world normal of a hit, the corners' normals blended, and the triangle's own.
    pub fn normals(&self, h: &Hit) -> (V3, V3) {
        let tri = &self.tris[h.tri as usize];
        let r = &self.instances[h.instance as usize].to_local;
        let n = unoct(tri.n[0]) * (1.0 - h.u - h.v) + unoct(tri.n[1]) * h.u + unoct(tri.n[2]) * h.v;
        let g = tri.e1.cross(tri.e2);
        // normals go by the inverse transpose: the world-to-local rows, transposed
        let tw = |a: V3| {
            v3(
                r[0][0] * a.x + r[1][0] * a.y + r[2][0] * a.z,
                r[0][1] * a.x + r[1][1] * a.y + r[2][1] * a.z,
                r[0][2] * a.x + r[1][2] * a.y + r[2][2] * a.z,
            )
            .norm()
        };
        (tw(n), tw(g))
    }

    /// The arrays as the GPU's words: nodes (8 per node), triangles (12 per), placements (16 per).
    pub fn words(&self) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
        let mut nodes: Vec<u32> = self.nodes.iter().flat_map(|n| n.words()).collect();
        if nodes.is_empty() {
            nodes = vec![0; 8];
        }
        let mut tris: Vec<u32> = self
            .tris
            .iter()
            .flat_map(|t| {
                let b = |v: V3| [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()];
                let (p, e, f) = (b(t.p0), b(t.e1), b(t.e2));
                [p[0], p[1], p[2], t.n[0], e[0], e[1], e[2], t.n[1], f[0], f[1], f[2], t.n[2]]
            })
            .collect();
        if tris.is_empty() {
            tris = vec![0; 12];
        }
        let mut inst: Vec<u32> = self
            .instances
            .iter()
            .flat_map(|i| {
                let r = |k: usize| i.to_local[k].map(f32::to_bits);
                let (a, b, c) = (r(0), r(1), r(2));
                [
                    a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3], c[0], c[1], c[2], c[3], i.root, i.material, 0, 0,
                ]
            })
            .collect();
        if inst.is_empty() {
            inst = vec![0; 16];
        }
        (nodes, tris, inst)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub t: f32,
    pub tri: u32,
    pub u: f32,
    pub v: f32,
    pub instance: u32,
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::mesh::Lod;
    use crate::studio::v3::rand;

    /// A part of loose triangles, every corner's normal +z.
    pub fn soup(tris: &[[V3; 3]]) -> PartMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for (k, t) in tris.iter().enumerate() {
            for (j, p) in t.iter().enumerate() {
                for c in [p.x, p.y, p.z] {
                    vertices.extend(c.to_le_bytes());
                }
                vertices.extend([0u8, 0, 0, 0]);
                indices.extend(((k * 3 + j) as u32).to_le_bytes());
            }
        }
        PartMesh {
            center: [0.0; 3],
            radius: 1.0,
            lods: vec![Lod {
                chord: 0.1,
                vertices,
                indices,
                ..Lod::default()
            }],
        }
    }

    #[test]
    fn the_hierarchies_find_what_brute_force_finds() {
        let tris: Vec<[V3; 3]> = (0..500u32)
            .map(|k| {
                let c = v3(rand(k, 0, 0, 1), rand(k, 0, 1, 1), rand(k, 0, 2, 1)) * 10.0;
                let j = |d: u32| v3(rand(k, d, 0, 2), rand(k, d, 1, 2), rand(k, d, 2, 2)) - V3::splat(0.5);
                [c + j(1), c + j(2), c + j(3)]
            })
            .collect();
        let parts = [soup(&tris)];
        let turn = |a: f32, t: [f32; 3]| -> Rows {
            let (s, c) = a.sin_cos();
            [[c, -s, 0.0, t[0]], [s, c, 0.0, t[1]], [0.0, 0.0, 1.0, t[2]]]
        };
        let inst = [
            Instance { part: 0, rows: turn(0.0, [0.0; 3]), material: 0 },
            Instance { part: 0, rows: turn(0.7, [20.0, 3.0, -2.0]), material: 1 },
            Instance { part: 0, rows: turn(-1.9, [-6.0, 25.0, 4.0]), material: 2 },
        ];
        let built = Built::new(&parts, &inst);
        assert_eq!(built.instances.len(), 3);
        let mut hits = 0;
        for k in 0..2000u32 {
            let o = v3(rand(k, 9, 0, 5), rand(k, 9, 1, 5), rand(k, 9, 2, 5)) * 60.0 - V3::splat(20.0);
            let to = v3(rand(k, 9, 3, 5), rand(k, 9, 4, 5), rand(k, 9, 5, 5)) * 30.0 - V3::splat(5.0);
            let d = (to - o).norm();
            // brute force over every placed triangle
            let mut best = f32::INFINITY;
            for i in &inst {
                for t in &tris {
                    let p = t.map(|q| place(&i.rows, q));
                    if let Some(h) = bvh::hit_triangle(o, d, p[0], p[1] - p[0], p[2] - p[0], best) {
                        best = h.0;
                    }
                }
            }
            let got = built.trace(o, d, f32::INFINITY).map_or(f32::INFINITY, |h| h.t);
            if best.is_finite() {
                hits += 1;
                assert!((got - best).abs() < 1e-3 * best.max(1.0), "ray {k}: {got} vs {best}");
            } else {
                assert!(!got.is_finite(), "ray {k}: hit {got} where nothing is");
            }
        }
        assert!(hits > 100, "{hits}");
    }
}

//! Builds a part's levels of detail from the kernel's tessellation: finer ones at a chord, the
//! farthest by clustering a coarse one (a part's topology floors its tessellation: research/render).

use crate::mesh::{ALWAYS, EdgeRange, FaceRange, Lod, PartMesh, SEGMENT, VERTEX};
use forge_geom::SurfaceGeom;
use forge_math::{Point3, Vec3};
use forge_mesh::{Quality, TessError, tessellate};
use forge_topo::{Body, EdgeId, FaceId};
use std::collections::HashMap;

/// Normals closer than this across a B-rep edge make it a tangent edge, drawn faint (3 degrees).
const TANGENT_COS: f64 = 0.99863;
/// Triangles closer than this across a mesh edge can't show a silhouette worth a line (0.1 degree).
const FLAT_COS: f64 = 0.9999985;
/// (chord mm, largest chord turn in degrees) of the tessellated levels.
const CHORDS: [(f64, f64); 4] = [(0.01, 15.0), (0.05, 22.5), (0.25, 45.0), (1.0, 60.0)];
/// Each clustered level's chord is this many times the last, up to `CLUSTER_END` of the radius.
const CLUSTER_STEP: f32 = 3.0;
const CLUSTER_END: f32 = 0.5;
/// How far a vertex can move in a cluster, in cells (a cell's diagonal).
const CELL_REACH: f32 = 1.75;

/// What a level's numbers depend on, for the build cache's key.
pub fn ladder_key() -> String {
    format!("{CHORDS:?}{CLUSTER_STEP}{CLUSTER_END}{CELL_REACH}{TANGENT_COS}{FLAT_COS}")
}

/// Unit normal to octahedral i16x2.
fn oct(n: Vec3) -> [i16; 2] {
    let s = n.x.abs() + n.y.abs() + n.z.abs();
    let (mut x, mut y) = (n.x / s, n.y / s);
    if n.z < 0.0 {
        (x, y) = ((1.0 - y.abs()) * x.signum(), (1.0 - x.abs()) * y.signum());
    }
    [(x * 32767.0).round() as i16, (y * 32767.0).round() as i16]
}

fn point(out: &mut Vec<u8>, p: Point3) {
    for v in [p.x, p.y, p.z] {
        out.extend_from_slice(&(v as f32).to_le_bytes());
    }
}

fn pair(out: &mut Vec<u8>, v: [i16; 2]) {
    out.extend_from_slice(&v[0].to_le_bytes());
    out.extend_from_slice(&v[1].to_le_bytes());
}

fn segment(out: &mut Vec<u8>, a: Point3, b: Point3, x: [i16; 2], y: [i16; 2]) {
    point(out, a);
    pair(out, x);
    point(out, b);
    pair(out, y);
}

/// The normal of the face's surface at a point on it; `near` is what its triangles say, used
/// where the surface has none (an apex) or projecting found another sheet.
fn exact_normal(body: &Body, f: FaceId, p: Point3, near: Vec3) -> Vec3 {
    let face = body.face(f);
    let near = near.normalize().map_or(Vec3::new(0.0, 0.0, 1.0), |u| u.get());
    match face.surface.normal(face.surface.project(p)) {
        Some(n) => {
            let n = if face.same_sense { n.get() } else { -n.get() };
            if n.dot(near) > 0.2 { n } else { near }
        }
        None => near,
    }
}

/// Meshes the body at this quality into one level of detail.
pub fn build_lod(body: &Body, q: &Quality) -> Result<Lod, TessError> {
    let mut lod = Lod {
        chord: q.chord as f32,
        ..Lod::default()
    };
    let mut vertices = 0u32;
    let mut fins = Vec::new();
    // ids index the body's arrays only once the dead are gone
    let compacted;
    let body = if body.dead > 0 {
        compacted = body.compact();
        &compacted
    } else {
        body
    };
    let m = tessellate(body, q)?;

    // triangles by face
    let nf = body.faces.len();
    let mut starts = vec![0u32; nf + 1];
    for &f in &m.face_of {
        starts[f as usize + 1] += 1;
    }
    for i in 0..nf {
        starts[i + 1] += starts[i];
    }
    let mut order = vec![0u32; m.triangles.len()];
    let mut fill = starts.clone();
    for (t, &f) in m.face_of.iter().enumerate() {
        order[fill[f as usize] as usize] = t as u32;
        fill[f as usize] += 1;
    }

    // each face's vertices its own, with its own normals
    let mut near = vec![Vec3::new(0.0, 0.0, 0.0); m.positions.len()];
    let mut normals: HashMap<(u32, u32), (u32, Vec3)> = HashMap::new();
    for f in 0..nf {
        let tris = &order[starts[f] as usize..starts[f + 1] as usize];
        if tris.is_empty() {
            continue;
        }
        for &t in tris {
            let tri = m.triangles[t as usize];
            let n = m.triangle_normal(tri);
            tri.iter().for_each(|&v| near[v as usize] += n);
        }
        for &t in tris {
            for v in m.triangles[t as usize] {
                let (idx, _) = *normals.entry((f as u32, v)).or_insert_with(|| {
                    let p = m.positions[v as usize];
                    let n = exact_normal(body, FaceId(f as u32), p, near[v as usize]);
                    point(&mut lod.vertices, p);
                    pair(&mut lod.vertices, oct(n));
                    let i = vertices;
                    vertices += 1;
                    (i, n)
                });
                lod.indices.extend_from_slice(&idx.to_le_bytes());
            }
        }
        for &t in tris {
            for v in m.triangles[t as usize] {
                near[v as usize] = Vec3::new(0.0, 0.0, 0.0);
            }
        }
        lod.faces.push(FaceRange {
            face: f as u32,
            first_tri: starts[f],
            tris: tris.len() as u32,
        });
    }

    // the B-rep edges; a seam (one face on both sides) is no edge to see
    let mut sharp: Vec<(u32, u32)> = Vec::new();
    for (e, chain) in m.edges.iter().enumerate() {
        let edge = body.edge(EdgeId(e as u32));
        let faces: Vec<u32> = edge.coedges.iter().map(|&c| body.coedge_face(c).0).collect();
        if let [a, b] = faces[..]
            && a == b
        {
            continue;
        }
        let tangent = match faces[..] {
            [a, b] => chain
                .iter()
                .all(|&v| match (normals.get(&(a, v)), normals.get(&(b, v))) {
                    (Some(x), Some(y)) => x.1.dot(y.1) > TANGENT_COS,
                    _ => false,
                }),
            _ => false,
        };
        let first = (lod.segments.len() / SEGMENT) as u32;
        for w in chain.windows(2) {
            let (a, b) = (m.positions[w[0] as usize], m.positions[w[1] as usize]);
            segment(&mut lod.segments, a, b, [ALWAYS, 0], [tangent as i16, 0]);
            if !tangent {
                sharp.push((w[0].min(w[1]), w[0].max(w[1])));
            }
        }
        lod.edges.push(EdgeRange {
            edge: e as u32,
            first,
            count: chain.len() as u32 - 1,
        });
    }
    sharp.sort_unstable();

    // mesh edges where the surface turns: a silhouette can run along these
    let mut sides: Vec<(u32, u32, u32)> = m
        .triangles
        .iter()
        .enumerate()
        .flat_map(|(t, &[a, b, c])| [(a, b), (b, c), (c, a)].map(|(x, y)| (x.min(y), x.max(y), t as u32)))
        .collect();
    sides.sort_unstable();
    for side in sides.chunk_by(|x, y| (x.0, x.1) == (y.0, y.1)) {
        let [p, q] = side else { continue };
        if sharp.binary_search(&(p.0, p.1)).is_ok() {
            continue;
        }
        let unit = |t: u32| m.triangle_normal(m.triangles[t as usize]).normalize();
        let (Some(a), Some(b)) = (unit(p.2), unit(q.2)) else {
            continue;
        };
        if a.get().dot(b.get()) > FLAT_COS {
            continue;
        }
        fins.push((
            m.positions[p.0 as usize],
            m.positions[p.1 as usize],
            oct(a.get()),
            oct(b.get()),
        ));
    }
    lod.fins = (lod.segments.len() / SEGMENT) as u32;
    for (a, b, x, y) in fins {
        segment(&mut lod.segments, a, b, x, y);
    }
    Ok(lod)
}

fn vec3_at(b: &[u8], at: usize) -> [f32; 3] {
    [0, 4, 8].map(|k| f32::from_le_bytes(b[at + k..at + k + 4].try_into().expect("4 bytes")))
}

/// The vertices merged into one: their sums, and its index once it has one.
struct Cell {
    p: [f32; 3],
    n: [f32; 2],
    count: f32,
    id: u32,
}

/// `src` with vertices merged by grid cell (and which way they face, so a sharp corner stays
/// sharp): triangles that collapse go, the rest keep their face. No lines: a part this far away
/// is too small for them.
pub fn cluster(src: &Lod, center: [f32; 3], chord: f32) -> Lod {
    let cell = chord / CELL_REACH;
    let nv = src.vertices.len() / VERTEX;
    let mut sums: HashMap<(i32, i32, i32, u8), Cell> = HashMap::new();
    let mut key_of = Vec::with_capacity(nv);
    for v in 0..nv {
        let at = v * VERTEX;
        let p = vec3_at(&src.vertices, at);
        let nx = i16::from_le_bytes([src.vertices[at + 12], src.vertices[at + 13]]) as f32 / 32767.0;
        let ny = i16::from_le_bytes([src.vertices[at + 14], src.vertices[at + 15]]) as f32 / 32767.0;
        let c = |k: usize| ((p[k] - center[k]) / cell).floor() as i32;
        // the octahedral square in a 4x4 grid: about 8 facings
        let facing = (((nx + 1.0) * 1.999) as u8) * 4 + ((ny + 1.0) * 1.999) as u8;
        let key = (c(0), c(1), c(2), facing);
        let s = sums.entry(key).or_insert(Cell {
            p: [0.0; 3],
            n: [0.0; 2],
            count: 0.0,
            id: u32::MAX,
        });
        (0..3).for_each(|k| s.p[k] += p[k]);
        s.n[0] += nx;
        s.n[1] += ny;
        s.count += 1.0;
        key_of.push(key);
    }
    let mut out = Lod {
        chord,
        ..Lod::default()
    };
    let mut emitted = 0u32;
    let mut seen = std::collections::HashSet::new();
    for f in &src.faces {
        let first_tri = out.tris();
        for t in f.first_tri..f.first_tri + f.tris {
            let at = t as usize * 12;
            let v = [0, 4, 8]
                .map(|k| u32::from_le_bytes(src.indices[at + k..at + k + 4].try_into().expect("4 bytes")));
            let k = v.map(|i| key_of[i as usize]);
            if k[0] == k[1] || k[1] == k[2] || k[2] == k[0] {
                continue;
            }
            let ids = k.map(|key| {
                let s = sums.get_mut(&key).expect("every vertex has a cell");
                if s.id == u32::MAX {
                    s.id = emitted;
                    emitted += 1;
                    for a in s.p {
                        out.vertices.extend_from_slice(&(a / s.count).to_le_bytes());
                    }
                    let (x, y) = (s.n[0] / s.count, s.n[1] / s.count);
                    pair(&mut out.vertices, [(x * 32767.0) as i16, (y * 32767.0) as i16]);
                }
                s.id
            });
            // the same three corners from another face of the cell: once is enough
            let mut sorted = ids;
            let min = (0..3).min_by_key(|&i| ids[i]).expect("three");
            sorted.rotate_left(min);
            if !seen.insert(sorted) {
                continue;
            }
            ids.iter()
                .for_each(|i| out.indices.extend_from_slice(&i.to_le_bytes()));
        }
        if out.tris() > first_tri {
            out.faces.push(FaceRange {
                face: f.face,
                first_tri,
                tris: out.tris() - first_tri,
            });
        }
    }
    out
}

/// All of a part's levels: those the kernel can't mesh at that coarseness are left out.
pub fn build_part(body: &Body) -> Result<PartMesh, TessError> {
    let mut lods: Vec<Lod> = Vec::new();
    for (i, &(chord, turn)) in CHORDS.iter().enumerate() {
        let q = Quality {
            chord,
            max_angle: turn * core::f64::consts::PI / 180.0,
        };
        match build_lod(body, &q) {
            Ok(l) => lods.push(l),
            Err(e) if i == 0 => return Err(e),
            Err(_) => {}
        }
    }
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    let first = &lods[0];
    for at in (0..first.vertices.len()).step_by(VERTEX) {
        let p = vec3_at(&first.vertices, at);
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let center = [0, 1, 2].map(|k| (lo[k] + hi[k]) * 0.5);
    let mut radius = 0f32;
    for at in (0..first.vertices.len()).step_by(VERTEX) {
        let p = vec3_at(&first.vertices, at);
        radius = radius.max((0..3).map(|k| (p[k] - center[k]).powi(2)).sum::<f32>());
    }
    let radius = radius.sqrt();
    loop {
        let last = lods.last().expect("the finest");
        let chord = last.chord * CLUSTER_STEP;
        if chord > radius * CLUSTER_END || lods.len() == crate::scene::LODS {
            break;
        }
        let l = cluster(last, center, chord);
        if l.tris() == 0 {
            break;
        }
        lods.push(l);
    }
    Ok(PartMesh { center, radius, lods })
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_agent::bench;

    fn crank() -> Body {
        bench::model(bench::src("crank").unwrap()).body().unwrap().clone()
    }

    #[test]
    fn every_triangle_keeps_its_face_and_every_edge_its_polyline() {
        let body = crank();
        let q = Quality::default();
        let m = tessellate(&body, &q).unwrap();
        let l = build_lod(&body, &q).unwrap();
        assert_eq!(l.tris() as usize, m.triangles.len());
        let mut want = m.face_of.clone();
        want.sort_unstable();
        let mut got: Vec<u32> = (0..l.tris()).map(|t| l.face_of(t)).collect();
        got.sort_unstable();
        assert_eq!(got, want);
        // no seams on a B-rep edge list, each polyline follows on from itself
        for e in &l.edges {
            let at = |s: u32, off: usize| vec3_at(&l.segments, s as usize * SEGMENT + off);
            for s in e.first + 1..e.first + e.count {
                assert_eq!(at(s - 1, 16), at(s, 0));
            }
        }
        assert!(l.edges.len() <= body.edges.len() && !l.edges.is_empty());
    }

    #[test]
    fn normals_are_exact_and_face_outward() {
        let body = crank();
        let l = build_lod(&body, &Quality::default()).unwrap();
        // each vertex's normal against the triangles around it: exact normals sit within the
        // chord's turn of them, and on a curved face they differ from the facets' own
        let dec = |at: usize| {
            let (x, y) = (
                i16::from_le_bytes([l.vertices[at + 12], l.vertices[at + 13]]) as f64 / 32767.0,
                i16::from_le_bytes([l.vertices[at + 14], l.vertices[at + 15]]) as f64 / 32767.0,
            );
            let z = 1.0 - x.abs() - y.abs();
            let t = (-z).max(0.0);
            let n = Vec3::new(x - t * x.signum(), y - t * y.signum(), z);
            n.normalize().unwrap().get()
        };
        for t in 0..l.tris() as usize {
            let v = [0, 4, 8].map(|k| {
                u32::from_le_bytes(l.indices[t * 12 + k..t * 12 + k + 4].try_into().unwrap()) as usize
            });
            let p = v.map(|i| vec3_at(&l.vertices, i * VERTEX));
            let d = |a: [f32; 3], b: [f32; 3]| {
                Vec3::new((b[0] - a[0]) as f64, (b[1] - a[1]) as f64, (b[2] - a[2]) as f64)
            };
            let Some(face) = d(p[0], p[1]).cross(d(p[0], p[2])).normalize() else {
                continue;
            };
            for i in v {
                assert!(dec(i * VERTEX).dot(face.get()) > 0.5, "triangle {t}");
            }
        }
    }

    #[test]
    fn a_cylinders_normals_point_straight_out_of_its_axis() {
        let m = bench::model("extrude 10 circle 0 0 5");
        assert!(m.error.is_none());
        let l = build_lod(m.body().unwrap(), &Quality::default()).unwrap();
        let mut side = 0;
        for at in (0..l.vertices.len()).step_by(VERTEX) {
            let p = vec3_at(&l.vertices, at);
            let n =
                |k: usize| i16::from_le_bytes([l.vertices[at + k], l.vertices[at + k + 1]]) as f32 / 32767.0;
            let (nx, ny) = (n(12), n(14));
            let rim = (p[0] * p[0] + p[1] * p[1]).sqrt();
            if (nx.abs() + ny.abs() - 1.0).abs() < 1e-3 && rim > 4.99 {
                // on the equator of the octahedral square, x : y is the normal's own
                let turned = (p[0] * ny - p[1] * nx).abs() / rim;
                assert!(turned < 2e-4, "{p:?} {nx} {ny}");
                side += 1;
            }
        }
        assert!(side > 20);
    }

    #[test]
    fn levels_get_coarser() {
        let m = build_part(&crank()).unwrap();
        assert!(m.lods.len() >= 4);
        for w in m.lods.windows(2) {
            assert!(w[0].chord < w[1].chord);
            assert!(w[0].tris() > w[1].tris());
        }
        assert!(m.radius > 1.0);
    }
}

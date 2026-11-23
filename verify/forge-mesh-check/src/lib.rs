//! What `tessellate`'s meshes are checked for (tests/mesh.rs; proofs in lean/).

use forge_mesh::TriMesh;

/// The number of separate fans of triangles round each vertex (`lean/MeshCheck/Mesh.lean`,
/// `fans`): walking from triangle to triangle across the edges at the vertex. A closed manifold
/// mesh has one at every vertex it uses; `TriMesh::open_edges` does not see a pinch.
pub fn vertex_fans(m: &TriMesh) -> Vec<usize> {
    let n = m.positions.len();
    // at each vertex v, triangle t gives the directed step a -> b for its corner at v (a, b the
    // other two, in order): the fan walks b of one to a of the next
    let mut at: Vec<Vec<(u32, u32)>> = vec![Vec::new(); n];
    for &[a, b, c] in &m.triangles {
        at[a as usize].push((b, c));
        at[b as usize].push((c, a));
        at[c as usize].push((a, b));
    }
    at.iter()
        .map(|steps| {
            let mut seen = vec![false; steps.len()];
            let mut fans = 0;
            for s in 0..steps.len() {
                if seen[s] {
                    continue;
                }
                fans += 1;
                let mut k = s;
                while !seen[k] {
                    seen[k] = true;
                    let to = steps[k].1;
                    match steps.iter().position(|x| x.0 == to) {
                        Some(j) => k = j,
                        None => break,
                    }
                }
            }
            fans
        })
        .collect()
}

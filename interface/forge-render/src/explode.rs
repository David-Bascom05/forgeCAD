//! An assembly's exploded view: where each occurrence goes, fully apart. Units (a real
//! subassembly, or a browser group such as Frame or Casters) move out first, each the least way
//! that clears every unit already placed, so no two units' boxes overlap; a fastener rides with the
//! part it sits in, then backs out along its own axis by its whole length, clear of its hole, and
//! fasteners stacked on one axis (screw, washer, nut) are spaced so none overlaps another.

pub use forge_lang::explode::*;

/// A part's box in its own space, from a mesh's vertices (f32 x, y, z first in each).
pub fn mesh_box(mesh: &crate::mesh::PartMesh) -> Aabb {
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    if let Some(l) = mesh.lods.first() {
        for v in l.vertices.as_chunks::<{ crate::mesh::VERTEX }>().0 {
            for k in 0..3 {
                let x = f32::from_le_bytes([v[k * 4], v[k * 4 + 1], v[k * 4 + 2], v[k * 4 + 3]]);
                lo[k] = lo[k].min(x);
                hi[k] = hi[k].max(x);
            }
        }
    }
    if lo[0] > hi[0] {
        let (c, r) = (mesh.center, mesh.radius);
        return ([c[0] - r, c[1] - r, c[2] - r], [c[0] + r, c[1] + r, c[2] + r]);
    }
    (lo, hi)
}

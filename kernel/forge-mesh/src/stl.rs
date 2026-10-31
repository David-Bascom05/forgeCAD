use crate::mesh::TriMesh;

/// Binary STL: an 80-byte header, the triangle count, then per triangle its
/// unit normal, three corners (f32, little endian) and two spare bytes.
pub fn to_binary_stl(m: &TriMesh, header: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(84 + 50 * m.triangles.len());
    let mut h = [0u8; 80];
    for (d, s) in h.iter_mut().zip(header.bytes()) {
        *d = s;
    }
    out.extend_from_slice(&h);
    out.extend_from_slice(&(m.triangles.len() as u32).to_le_bytes());
    for &t in &m.triangles {
        let n = m
            .triangle_normal(t)
            .normalize()
            .map(|u| u.get())
            .unwrap_or_default();
        for v in [n.x, n.y, n.z] {
            out.extend_from_slice(&(v as f32).to_le_bytes());
        }
        for i in t {
            let p = m.positions[i as usize];
            for v in [p.x, p.y, p.z] {
                out.extend_from_slice(&(v as f32).to_le_bytes());
            }
        }
        out.extend_from_slice(&[0, 0]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::Point3;

    #[test]
    fn layout() {
        let m = TriMesh {
            positions: vec![
                Point3::ORIGIN,
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            triangles: vec![[0, 1, 2]],
            face_of: vec![0],
        };
        let b = to_binary_stl(&m, "forgecad");
        assert_eq!(b.len(), 84 + 50);
        assert_eq!(&b[..8], b"forgecad");
        assert_eq!(u32::from_le_bytes(b[80..84].try_into().unwrap()), 1);
        assert_eq!(f32::from_le_bytes(b[92..96].try_into().unwrap()), 1.0); // normal z
    }
}

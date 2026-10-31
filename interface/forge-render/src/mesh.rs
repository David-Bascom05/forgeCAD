//! The GPU-ready mesh of a part: vertices split per B-rep face with the face's exact normals, each
//! face's triangles contiguous, and each B-rep edge a polyline along the triangles' own sides.
//! Arrays are little-endian bytes, so a cached file goes to the GPU without being looked at.

/// Bytes per vertex: position f32x3, normal octahedral i16x2.
pub const VERTEX: usize = 16;
/// Bytes per line segment: end f32x3, i16x2, end f32x3, i16x2. The i16x2 are the normals of the
/// two triangles beside it, or `ALWAYS` and a tone for a B-rep edge.
pub const SEGMENT: usize = 32;
pub const ALWAYS: i16 = i16::MIN;
/// A file's first bytes; bump with any change to this layout.
const MAGIC: &[u8; 8] = b"FCMESH01";

/// A B-rep face's triangles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceRange {
    pub face: u32,
    pub first_tri: u32,
    pub tris: u32,
}

/// A B-rep edge's polyline in `Lod::segments`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeRange {
    pub edge: u32,
    pub first: u32,
    pub count: u32,
}

/// One level of detail.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lod {
    /// The largest gap from the exact shape, in mm.
    pub chord: f32,
    pub vertices: Vec<u8>,
    /// u32 triples, by face.
    pub indices: Vec<u8>,
    /// The edge polylines, then (from `fins`) the mesh edges a silhouette can run along.
    pub segments: Vec<u8>,
    pub fins: u32,
    pub faces: Vec<FaceRange>,
    pub edges: Vec<EdgeRange>,
}

impl Lod {
    pub fn tris(&self) -> u32 {
        (self.indices.len() / 12) as u32
    }

    /// The B-rep face a triangle came from.
    pub fn face_of(&self, tri: u32) -> u32 {
        let i = self.faces.partition_point(|f| f.first_tri + f.tris <= tri);
        self.faces[i].face
    }
}

/// A part's levels of detail, finest first, and a sphere holding it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartMesh {
    pub center: [f32; 3],
    pub radius: f32,
    pub lods: Vec<Lod>,
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn u32(&mut self) -> Option<u32> {
        let (a, b) = self.0.split_first_chunk::<4>()?;
        self.0 = b;
        Some(u32::from_le_bytes(*a))
    }
    fn f32(&mut self) -> Option<f32> {
        self.u32().map(f32::from_bits)
    }
    fn bytes(&mut self) -> Option<Vec<u8>> {
        let n = self.u32()? as usize;
        let (a, b) = self.0.split_at_checked(n)?;
        self.0 = b;
        Some(a.to_vec())
    }
    fn triples(&mut self) -> Option<Vec<[u32; 3]>> {
        let n = self.u32()? as usize;
        (0..n)
            .map(|_| Some([self.u32()?, self.u32()?, self.u32()?]))
            .collect()
    }
}

fn put_bytes(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
}

fn put_triples(out: &mut Vec<u8>, t: impl ExactSizeIterator<Item = [u32; 3]>) {
    out.extend_from_slice(&(t.len() as u32).to_le_bytes());
    t.for_each(|x| x.iter().for_each(|v| out.extend_from_slice(&v.to_le_bytes())));
}

impl PartMesh {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        for v in [self.center[0], self.center[1], self.center[2], self.radius] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&(self.lods.len() as u32).to_le_bytes());
        for l in &self.lods {
            out.extend_from_slice(&l.chord.to_le_bytes());
            out.extend_from_slice(&l.fins.to_le_bytes());
            put_bytes(&mut out, &l.vertices);
            put_bytes(&mut out, &l.indices);
            put_bytes(&mut out, &l.segments);
            put_triples(&mut out, l.faces.iter().map(|f| [f.face, f.first_tri, f.tris]));
            put_triples(&mut out, l.edges.iter().map(|e| [e.edge, e.first, e.count]));
        }
        out
    }

    /// None if it isn't a whole mesh file of this format.
    pub fn from_bytes(b: &[u8]) -> Option<PartMesh> {
        let mut r = Reader(b.strip_prefix(MAGIC)?);
        let center = [r.f32()?, r.f32()?, r.f32()?];
        let radius = r.f32()?;
        let n = r.u32()?;
        let mut lods = Vec::with_capacity(n as usize);
        for _ in 0..n {
            lods.push(Lod {
                chord: r.f32()?,
                fins: r.u32()?,
                vertices: r.bytes()?,
                indices: r.bytes()?,
                segments: r.bytes()?,
                faces: r
                    .triples()?
                    .into_iter()
                    .map(|[face, first_tri, tris]| FaceRange {
                        face,
                        first_tri,
                        tris,
                    })
                    .collect(),
                edges: r
                    .triples()?
                    .into_iter()
                    .map(|[edge, first, count]| EdgeRange { edge, first, count })
                    .collect(),
            });
        }
        r.0.is_empty().then_some(PartMesh { center, radius, lods })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_comes_back_whole_or_not_at_all() {
        let m = PartMesh {
            center: [1.0, 2.0, 3.0],
            radius: 4.0,
            lods: vec![Lod {
                chord: 0.5,
                vertices: vec![1; VERTEX],
                indices: vec![0; 12],
                segments: vec![2; SEGMENT],
                fins: 1,
                faces: vec![FaceRange {
                    face: 7,
                    first_tri: 0,
                    tris: 1,
                }],
                edges: vec![EdgeRange {
                    edge: 9,
                    first: 0,
                    count: 1,
                }],
            }],
        };
        let b = m.to_bytes();
        assert_eq!(PartMesh::from_bytes(&b).as_ref(), Some(&m));
        assert!(PartMesh::from_bytes(&b[..b.len() - 1]).is_none());
        assert!(PartMesh::from_bytes(&b"not a mesh"[..]).is_none());
    }

    #[test]
    fn a_triangle_finds_its_face() {
        let l = Lod {
            faces: vec![
                FaceRange {
                    face: 5,
                    first_tri: 0,
                    tris: 2,
                },
                FaceRange {
                    face: 3,
                    first_tri: 2,
                    tris: 1,
                },
            ],
            ..Lod::default()
        };
        assert_eq!([l.face_of(0), l.face_of(1), l.face_of(2)], [5, 5, 3]);
    }
}

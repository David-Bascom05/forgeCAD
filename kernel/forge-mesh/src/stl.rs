use crate::mesh::TriMesh;
use crate::scene::{Object, Scene, weld};
use forge_math::{Point3, Vec3};

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

/// A scene as one binary STL: every item where it is placed, its triangles in item order. STL has
/// no units: millimetres, as slicers read it.
pub fn scene_binary_stl(scene: &Scene) -> Vec<u8> {
    let flat = scene.flatten();
    let mut all = TriMesh::default();
    for o in &flat.objects {
        let base = all.positions.len() as u32;
        all.positions.extend_from_slice(&o.positions);
        all.triangles
            .extend(o.triangles.iter().map(|t| t.map(|i| i + base)));
    }
    to_binary_stl(&all, &scene.title)
}

/// The facet normal STL writes: the unit normal of the triangle as written (its corners in f32),
/// zero for one with no area.
fn facet_normal(c: &[Point3; 3]) -> Vec3 {
    (c[1] - c[0])
        .cross(c[2] - c[0])
        .normalize()
        .map_or(Vec3::ZERO, |u| u.get())
}

fn f32_point(p: Point3) -> Point3 {
    Point3::new(p.x as f32 as f64, p.y as f32 as f64, p.z as f32 as f64)
}

/// A scene as ASCII STL: a `solid` per item, named as the object is. Coordinates are written as
/// binary STL holds them (f32), so both read back to the same points.
pub fn scene_ascii_stl(scene: &Scene) -> String {
    let flat = scene.flatten();
    let mut s = String::new();
    for o in &flat.objects {
        let name: String = o
            .name
            .chars()
            .map(|c| if c.is_whitespace() { '_' } else { c })
            .collect();
        s += &format!("solid {name}\n");
        for t in &o.triangles {
            let c = t.map(|i| f32_point(o.positions[i as usize]));
            let n = facet_normal(&c);
            s += &format!(
                "  facet normal {} {} {}\n    outer loop\n",
                e(n.x),
                e(n.y),
                e(n.z)
            );
            for p in c {
                s += &format!("      vertex {} {} {}\n", e(p.x), e(p.y), e(p.z));
            }
            s += "    endloop\n  endfacet\n";
        }
        s += &format!("endsolid {name}\n");
    }
    s
}

/// An f32 as its shortest round-tripping text, in exponent form as ASCII STL is usually written.
fn e(x: f64) -> String {
    let x = x as f32;
    if x == 0.0 { "0e0".into() } else { format!("{x:e}") }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StlError(pub String);

/// An STL file, binary or ASCII, as objects: one per ASCII `solid` (one for a binary file),
/// corners at the very same point made one vertex. Facet normals are read and checked against
/// the corners: how many disagree is the second value.
pub fn read_stl(data: &[u8]) -> Result<(Vec<Object>, usize), StlError> {
    // the size a binary file of the count at 80..84 has: in u64, which no count overflows (a
    // usize is 32 bits in the browser, and an ASCII file's bytes there are any count)
    let binary_len = |d: &[u8]| 84 + 50 * u32::from_le_bytes(d[80..84].try_into().expect("4 bytes")) as u64;
    let ascii = data.starts_with(b"solid") && (data.len() < 84 || binary_len(data) != data.len() as u64);
    let mut wrong = 0;
    let mut check = |n: Vec3, c: &[Point3; 3]| {
        let f = facet_normal(c);
        if f != Vec3::ZERO && (n - f).length() > 1e-3 {
            wrong += 1;
        }
    };
    if !ascii {
        if data.len() < 84 {
            return Err(StlError("shorter than a binary STL header".into()));
        }
        let n = u32::from_le_bytes(data[80..84].try_into().expect("4 bytes")) as usize;
        if binary_len(data) != data.len() as u64 {
            return Err(StlError(format!("{n} triangles but {} bytes", data.len())));
        }
        let f = |at: usize| f32::from_le_bytes(data[at..at + 4].try_into().expect("4 bytes")) as f64;
        let mut tris = Vec::with_capacity(n);
        for k in 0..n {
            let at = 84 + 50 * k;
            let v = |j: usize| Point3::new(f(at + 12 * j), f(at + 12 * j + 4), f(at + 12 * j + 8));
            let c = [v(1), v(2), v(3)];
            let p = v(0);
            check(Vec3::new(p.x, p.y, p.z), &c);
            tris.push(c);
        }
        let name = String::from_utf8_lossy(&data[..80])
            .trim_end_matches('\0')
            .to_string();
        return Ok((vec![weld(&name, &tris)], wrong));
    }
    let text = std::str::from_utf8(data).map_err(|_| StlError("ASCII STL not UTF-8".into()))?;
    let mut out = Vec::new();
    let mut w = text.split_whitespace();
    let mut name = String::new();
    let mut tris: Vec<[Point3; 3]> = Vec::new();
    let mut normal = Vec3::ZERO;
    let mut corners: Vec<Point3> = Vec::new();
    let num = |w: &mut std::str::SplitWhitespace| -> Result<f64, StlError> {
        w.next()
            // STL's numbers are f32s, ASCII or binary
            .and_then(|t| t.parse::<f32>().ok())
            .map(f64::from)
            .ok_or_else(|| StlError("a number expected".into()))
    };
    while let Some(tok) = w.next() {
        match tok {
            "solid" => {
                name.clear();
                tris.clear();
                // the name runs to the end of the line: take what is up to "facet"
                let rest = w.clone().next().unwrap_or("");
                if rest != "facet" && rest != "endsolid" {
                    name = w.next().unwrap_or("").to_string();
                }
            }
            "facet" => {
                if w.next() != Some("normal") {
                    return Err(StlError("facet without normal".into()));
                }
                normal = Vec3::new(num(&mut w)?, num(&mut w)?, num(&mut w)?);
                corners.clear();
            }
            "vertex" => corners.push(Point3::new(num(&mut w)?, num(&mut w)?, num(&mut w)?)),
            "endfacet" => {
                if corners.len() != 3 {
                    return Err(StlError(format!("a facet of {} corners", corners.len())));
                }
                let c = [corners[0], corners[1], corners[2]];
                check(normal, &c);
                tris.push(c);
            }
            "endsolid" => {
                out.push(weld(&name, &tris));
                tris.clear();
                // the name after endsolid, if any, is skipped as an unknown word
            }
            _ => {}
        }
    }
    Ok((out, wrong))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_count_in_the_header_is_read_safely() {
        // an ASCII file whose bytes 80..84 read as a huge count, and a binary one that lies
        let mut ascii = b"solid x\n".to_vec();
        ascii.resize(100, b' ');
        ascii[80..84].copy_from_slice(b"zzzz"); // 0x7a7a7a7a triangles: past a 32-bit usize
        ascii.extend_from_slice(b"\nendsolid x\n");
        assert_eq!(read_stl(&ascii).unwrap().0.len(), 1);
        let mut bin = vec![0u8; 84];
        bin[80..84].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(read_stl(&bin).is_err());
    }

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
            edges: Vec::new(),
        };
        let b = to_binary_stl(&m, "forgecad");
        assert_eq!(b.len(), 84 + 50);
        assert_eq!(&b[..8], b"forgecad");
        assert_eq!(u32::from_le_bytes(b[80..84].try_into().unwrap()), 1);
        assert_eq!(f32::from_le_bytes(b[92..96].try_into().unwrap()), 1.0); // normal z
    }
}

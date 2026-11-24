//! Other systems' meshes read as triangles: STL (binary and ASCII) and Wavefront OBJ. What they
//! give is a mesh, for reference and measurement, not a boundary representation: no faces, edges
//! or exact surfaces come of it, and nothing here makes it a body the kernel builds on.
//!
//! Files are untrusted: every input ends in a mesh or a `ReadError` that says where, memory is
//! bounded by the file's own size (a binary STL's count must agree with its length), and nothing
//! here panics on any bytes. STL and OBJ carry no unit: their numbers are taken as millimetres.

use crate::TriMesh;
use forge_math::Point3;
use std::collections::HashMap;

/// Why a file did not read, and where: the byte it stopped at and its line (1-based; 0 in a
/// binary file).
#[derive(Clone, Debug, PartialEq)]
pub struct ReadError {
    pub at: usize,
    pub line: usize,
    pub why: String,
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(f, "line {} (byte {}): {}", self.line, self.at, self.why)
        } else {
            write!(f, "byte {}: {}", self.at, self.why)
        }
    }
}

/// The most triangles a mesh read here may have (some 3 GB of positions and indices at worst).
pub const MAX_TRIANGLES: usize = 64 << 20;

fn err<T>(at: usize, line: usize, why: impl Into<String>) -> Result<T, ReadError> {
    Err(ReadError {
        at,
        line,
        why: why.into(),
    })
}

/// Vertices joined where their coordinates are the same to the bit (an STL repeats each corner
/// in every triangle that meets there).
#[derive(Default)]
struct Weld {
    mesh: TriMesh,
    at: HashMap<[u64; 3], u32>,
}

impl Weld {
    fn vertex(&mut self, p: [f64; 3]) -> u32 {
        // -0 and 0 are one place
        let key = p.map(|c| (c + 0.0).to_bits());
        *self
            .at
            .entry(key)
            .or_insert_with(|| self.mesh.add_vertex(Point3::new(p[0], p[1], p[2])))
    }
}

fn finite(p: [f64; 3]) -> bool {
    p.iter().all(|c| c.is_finite())
}

/// An STL file, binary or ASCII (told apart by its length and its first word).
pub fn stl(data: &[u8]) -> Result<TriMesh, ReadError> {
    let binary_len = (data.len() >= 84)
        .then(|| u32::from_le_bytes([data[80], data[81], data[82], data[83]]) as usize)
        .map(|n| n.saturating_mul(50).saturating_add(84));
    let ascii = data
        .iter()
        .position(|c| !c.is_ascii_whitespace())
        .is_some_and(|k| data[k..].starts_with(b"solid"));
    // a binary file's header may begin "solid" too: its length settles it
    if binary_len == Some(data.len()) || !ascii {
        stl_binary(data)
    } else {
        stl_ascii(data)
    }
}

fn stl_binary(data: &[u8]) -> Result<TriMesh, ReadError> {
    if data.len() < 84 {
        return err(data.len(), 0, "a binary STL shorter than its 84-byte header");
    }
    let n = u32::from_le_bytes([data[80], data[81], data[82], data[83]]) as usize;
    if n > MAX_TRIANGLES {
        return err(80, 0, format!("{n} triangles, past the {MAX_TRIANGLES} read"));
    }
    if data.len() != 84 + 50 * n {
        return err(
            80,
            0,
            format!(
                "{n} triangles take {} bytes; the file has {}",
                84 + 50 * n,
                data.len()
            ),
        );
    }
    let mut w = Weld::default();
    w.mesh.triangles.reserve(n);
    for k in 0..n {
        let at = 84 + 50 * k;
        let f = |i: usize| -> f64 {
            let o = at + 12 + 4 * i;
            f64::from(f32::from_le_bytes([
                data[o],
                data[o + 1],
                data[o + 2],
                data[o + 3],
            ]))
        };
        let mut t = [0u32; 3];
        for (c, v) in t.iter_mut().enumerate() {
            let p = [f(3 * c), f(3 * c + 1), f(3 * c + 2)];
            if !finite(p) {
                return err(
                    at + 12 + 12 * c,
                    0,
                    format!("triangle {k}: a corner not a finite number"),
                );
            }
            *v = w.vertex(p);
        }
        w.mesh.triangles.push(t);
    }
    Ok(w.mesh)
}

/// Whitespace-separated words, each with its byte and line.
struct Words<'a> {
    b: &'a [u8],
    i: usize,
    line: usize,
}

impl<'a> Words<'a> {
    fn new(b: &'a [u8]) -> Self {
        Words { b, i: 0, line: 1 }
    }
    fn next(&mut self) -> Option<(&'a [u8], usize, usize)> {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            if self.b[self.i] == b'\n' {
                self.line += 1;
            }
            self.i += 1;
        }
        if self.i >= self.b.len() {
            return None;
        }
        let s = self.i;
        while self.i < self.b.len() && !self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
        Some((&self.b[s..self.i], s, self.line))
    }
}

fn number(w: &[u8]) -> Option<f64> {
    std::str::from_utf8(w)
        .ok()?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

fn stl_ascii(data: &[u8]) -> Result<TriMesh, ReadError> {
    let mut ws = Words::new(data);
    let mut w = Weld::default();
    let mut corners: Vec<u32> = Vec::with_capacity(3);
    let mut in_loop = false;
    while let Some((word, at, line)) = ws.next() {
        match word {
            b"vertex" => {
                if !in_loop {
                    return err(at, line, "a vertex outside an outer loop");
                }
                let mut p = [0.0; 3];
                for c in &mut p {
                    let Some((x, at, line)) = ws.next() else {
                        return err(data.len(), ws.line, "the file ends inside a vertex");
                    };
                    let Some(v) = number(x) else {
                        return err(
                            at,
                            line,
                            format!("{:?} is not a finite number", String::from_utf8_lossy(x)),
                        );
                    };
                    *c = v;
                }
                if corners.len() == 3 {
                    return err(at, line, "a loop of more than three vertices");
                }
                corners.push(w.vertex(p));
            }
            b"outer" => {
                if ws.next().map(|x| x.0) != Some(b"loop") {
                    return err(at, line, "`outer` without `loop`");
                }
                in_loop = true;
                corners.clear();
            }
            b"endloop" => {
                if corners.len() != 3 {
                    return err(at, line, format!("a loop of {} vertices, not 3", corners.len()));
                }
                if w.mesh.triangles.len() >= MAX_TRIANGLES {
                    return err(at, line, format!("past the {MAX_TRIANGLES} triangles read"));
                }
                w.mesh.triangles.push([corners[0], corners[1], corners[2]]);
                in_loop = false;
            }
            b"facet" => {
                // `normal nx ny nz`: the corners' order says which way it faces
                if ws.next().map(|x| x.0) == Some(b"normal") {
                    for _ in 0..3 {
                        ws.next();
                    }
                }
            }
            b"solid" | b"endsolid" => {
                // a name follows, to the end of the line
                while ws.i < data.len() && data[ws.i] != b'\n' {
                    ws.i += 1;
                }
            }
            b"endfacet" => {}
            other => {
                return err(
                    at,
                    line,
                    format!("{:?} is not an STL word", String::from_utf8_lossy(other)),
                );
            }
        }
    }
    if in_loop {
        return err(data.len(), ws.line, "the file ends inside a loop");
    }
    if w.mesh.triangles.is_empty() {
        return err(0, 1, "no triangles");
    }
    Ok(w.mesh)
}

/// A Wavefront OBJ file's faces (`v` and `f`; texture and normal indices are read past; a face of
/// more corners is cut into a fan). Every object and group in it goes into the one mesh. A line
/// that ends in a backslash goes on in the next (the format's continuation; trimesh and assimp
/// read it): what is wrong is said at the line it starts on.
pub fn obj(data: &[u8]) -> Result<TriMesh, ReadError> {
    let mut m = TriMesh::default();
    let mut at = 0;
    let mut lines = data.split(|&c| c == b'\n').enumerate();
    let mut joined: Vec<u8> = Vec::new();
    while let Some((k, raw)) = lines.next() {
        let line = k + 1;
        let here = at;
        at += raw.len() + 1;
        let mut text = raw.strip_suffix(b"\r").unwrap_or(raw);
        if text.ends_with(b"\\") {
            // continued: the pieces joined by a space, each without its backslash
            joined.clear();
            joined.extend_from_slice(&text[..text.len() - 1]);
            for (_, next) in lines.by_ref() {
                at += next.len() + 1;
                let n = next.strip_suffix(b"\r").unwrap_or(next);
                joined.push(b' ');
                match n.strip_suffix(b"\\") {
                    Some(more) => joined.extend_from_slice(more),
                    None => {
                        joined.extend_from_slice(n);
                        break;
                    }
                }
            }
            text = &joined;
        }
        let text = match text.iter().position(|&c| c == b'#') {
            Some(h) => &text[..h],
            None => text,
        };
        let mut ws = text.split(|c| c.is_ascii_whitespace()).filter(|w| !w.is_empty());
        match ws.next() {
            Some(b"v") => {
                let mut p = [0.0; 3];
                for c in &mut p {
                    let Some(v) = ws.next().and_then(number) else {
                        return err(here, line, "a vertex needs three finite numbers");
                    };
                    *c = v;
                }
                // a fourth (w) or colours may follow: not geometry
                m.add_vertex(Point3::new(p[0], p[1], p[2]));
            }
            Some(b"f") => {
                let n = m.positions.len() as i64;
                let mut idx: Vec<u32> = Vec::new();
                for w in ws {
                    let first = w.split(|&c| c == b'/').next().unwrap_or(b"");
                    let Some(i) = std::str::from_utf8(first)
                        .ok()
                        .and_then(|s| s.parse::<i64>().ok())
                    else {
                        return err(
                            here,
                            line,
                            format!("{:?} is not a vertex index", String::from_utf8_lossy(w)),
                        );
                    };
                    // 1-based, or back from the last vertex read
                    let i = if i < 0 { n + i } else { i - 1 };
                    if i < 0 || i >= n {
                        return err(here, line, format!("vertex {} of {n} read so far", i + 1));
                    }
                    idx.push(i as u32);
                }
                if idx.len() < 3 {
                    // the format's own rule (Wavefront's OBJ, appendix B1: a face element needs
                    // three vertices at least; two make a line, `l`): a file breaking it is not
                    // read past, the way some readers skip the face
                    return err(
                        here,
                        line,
                        format!(
                            "a face of {} corner{}: OBJ's faces have three at least (two points make a line, `l`)",
                            idx.len(),
                            if idx.len() == 1 { "" } else { "s" }
                        ),
                    );
                }
                if m.triangles.len() + idx.len() > MAX_TRIANGLES {
                    return err(here, line, format!("past the {MAX_TRIANGLES} triangles read"));
                }
                for j in 1..idx.len() - 1 {
                    m.triangles.push([idx[0], idx[j], idx[j + 1]]);
                }
            }
            _ => {}
        }
    }
    if m.triangles.is_empty() {
        return err(0, 1, "no faces");
    }
    Ok(m)
}

/// What a mesh measures, said as a mesh: its volume counts only where it is closed.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub triangles: usize,
    pub vertices: usize,
    /// Every edge shared by two triangles, one each way: a closed, consistently turned surface.
    pub closed: bool,
    /// Edges not so shared.
    pub open_edges: usize,
    /// Enclosed volume (mm³), when closed; its sign says the triangles face out (+) or in (−).
    pub volume: Option<f64>,
    pub area: f64,
    /// (min, max) corners.
    pub bbox: ([f64; 3], [f64; 3]),
}

pub fn summary(m: &TriMesh) -> Summary {
    let open = m.open_edges().len();
    let b = m.bbox();
    Summary {
        triangles: m.triangles.len(),
        vertices: m.positions.len(),
        closed: open == 0,
        open_edges: open,
        volume: (open == 0).then(|| m.volume()),
        area: m.area(),
        bbox: ([b.x.lo, b.y.lo, b.z.lo], [b.x.hi, b.y.hi, b.z.hi]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUBE_OBJ: &str = "# a unit cube\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 0 0 1\nv 1 0 1\nv 1 1 1\nv 0 1 1\n\
        f 1 4 3 2\nf 5 6 7 8\nf 1 2 6 5\nf 2 3 7 6\nf 3 4 8 7\nf 4 1 5 8\n";

    #[test]
    fn a_cube_reads_closed_with_its_volume_from_obj_and_both_stls() {
        let m = obj(CUBE_OBJ.as_bytes()).unwrap();
        let s = summary(&m);
        assert!(s.closed, "{s:?}");
        assert!((s.volume.unwrap() - 1.0).abs() < 1e-12 && (s.area - 6.0).abs() < 1e-12);
        let bin = crate::to_binary_stl(&m, "cube");
        let b = stl(&bin).unwrap();
        assert_eq!(summary(&b), s);
        let mut ascii = String::from("solid cube\n");
        for t in &m.triangles {
            ascii += "facet normal 0 0 0\n outer loop\n";
            for &i in t {
                let p = m.positions[i as usize];
                ascii += &format!("  vertex {} {} {}\n", p.x, p.y, p.z);
            }
            ascii += " endloop\nendfacet\n";
        }
        ascii += "endsolid cube\n";
        assert_eq!(summary(&stl(ascii.as_bytes()).unwrap()), s);
    }

    #[test]
    fn what_is_wrong_is_said_where() {
        let e = obj(b"v 0 0 0\nv 1 0 0\nf 1 2 9\n").unwrap_err();
        assert_eq!((e.line, e.at), (3, 16));
        let e = stl(b"solid x\nfacet normal 0 0 1\nouter loop\nvertex 0 0 nan\n").unwrap_err();
        assert_eq!(e.line, 4);
        // a binary count that the length does not hold
        let mut b = vec![0u8; 84];
        b[80..84].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(stl(&b).is_err());
        assert!(stl(b"").is_err() && obj(b"").is_err());
    }

    #[test]
    fn a_face_of_two_corners_is_refused_and_said_so() {
        // formats-corpus's bug 16 (trimesh's notenoughindices.obj): refused, the rule named
        let e = obj(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\nf 1 2\n").unwrap_err();
        assert_eq!(e.line, 5);
        assert!(
            e.why.contains("2 corners") && e.why.contains("three at least"),
            "{e:?}"
        );
    }

    #[test]
    fn a_continued_line_is_one_line() {
        // research/formats/repro/make.py's continuation.obj: a tetrahedron, one face over two lines
        let t = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nf 1 3 2\nf 1 2 4\nf 2 3 \\\n4\nf 1 4 3\n";
        let s = summary(&obj(t).unwrap());
        assert!(s.closed && (s.volume.unwrap() - 1.0 / 6.0).abs() < 1e-12, "{s:?}");
        // CRLF, a line continued twice, and the line after it still counted
        let t = b"v 0 0 0\r\nv 1 0 0\r\nv 0 1 0\r\nv 0 0 1\r\nf 1 \\\r\n3 \\\r\n2\r\nf 1 2 4\r\nf 2 3 4\r\nf 1 4 9\r\n";
        let e = obj(t).unwrap_err();
        assert_eq!(e.line, 10, "{e:?}");
    }
}

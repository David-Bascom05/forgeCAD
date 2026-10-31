//! Writing small TrueType fonts, for tests: no font files are kept with the code.

use crate::Seg;
use forge_math::Point2;

/// A glyph to write: closed contours (whole-unit points, outer ones clockwise) and an advance.
#[derive(Clone, Debug)]
pub struct TestGlyph {
    pub contours: Vec<Vec<Seg>>,
    pub advance: u16,
}

impl TestGlyph {
    /// A rectangle from (x0, y0) to (x1, y1).
    pub fn bar(x0: f64, y0: f64, x1: f64, y1: f64, advance: f64) -> TestGlyph {
        let p = Point2::new;
        let c = [p(x0, y0), p(x0, y1), p(x1, y1), p(x1, y0)];
        TestGlyph {
            contours: vec![(0..4).map(|k| Seg::Line(c[k], c[(k + 1) % 4])).collect()],
            advance: advance as u16,
        }
    }
}

/// An O: a rounded square of four quadratic arcs round a smaller one.
pub fn square_ring() -> TestGlyph {
    let p = Point2::new;
    let q = |a: (f64, f64), c: (f64, f64), b: (f64, f64)| Seg::Quad(p(a.0, a.1), p(c.0, c.1), p(b.0, b.1));
    TestGlyph {
        contours: vec![
            vec![
                q((300.0, 0.0), (0.0, 0.0), (0.0, 350.0)),
                q((0.0, 350.0), (0.0, 700.0), (300.0, 700.0)),
                q((300.0, 700.0), (600.0, 700.0), (600.0, 350.0)),
                q((600.0, 350.0), (600.0, 0.0), (300.0, 0.0)),
            ],
            vec![
                q((300.0, 150.0), (450.0, 150.0), (450.0, 350.0)),
                q((450.0, 350.0), (450.0, 550.0), (300.0, 550.0)),
                q((300.0, 550.0), (150.0, 550.0), (150.0, 350.0)),
                q((150.0, 350.0), (150.0, 150.0), (300.0, 150.0)),
            ],
        ],
        advance: 700,
    }
}

fn be16(v: &mut Vec<u8>, x: u16) {
    v.extend(x.to_be_bytes());
}

fn be32(v: &mut Vec<u8>, x: u32) {
    v.extend(x.to_be_bytes());
}

/// A simple glyph's glyf record.
pub fn simple(g: &TestGlyph) -> Vec<u8> {
    let mut v = Vec::new();
    let pts: Vec<(Vec<(Point2, bool)>, usize)> = g
        .contours
        .iter()
        .map(|c| {
            let mut pts = Vec::new();
            for s in c {
                match *s {
                    Seg::Line(a, _) => pts.push((a, true)),
                    Seg::Quad(a, ctrl, _) => pts.extend([(a, true), (ctrl, false)]),
                }
            }
            let len = pts.len();
            (pts, len)
        })
        .collect();
    be16(&mut v, g.contours.len() as u16);
    v.extend([0; 8]);
    let mut end = 0usize;
    for (_, len) in &pts {
        end += len;
        be16(&mut v, (end - 1) as u16);
    }
    be16(&mut v, 0);
    let all: Vec<(Point2, bool)> = pts.into_iter().flat_map(|p| p.0).collect();
    v.extend(all.iter().map(|p| p.1 as u8));
    for axis in 0..2 {
        let mut last = 0i32;
        for (q, _) in &all {
            let c = if axis == 0 { q.x } else { q.y } as i32;
            be16(&mut v, (c - last) as i16 as u16);
            last = c;
        }
    }
    v
}

/// A composite glyph's glyf record: parts by glyph id, each offset by whole units.
pub fn composite(parts: &[(u16, i16, i16)]) -> Vec<u8> {
    let mut v = Vec::new();
    be16(&mut v, 0xffff);
    v.extend([0; 8]);
    for (k, &(g, dx, dy)) in parts.iter().enumerate() {
        // word offsets, xy values, more to come but the last
        be16(&mut v, 0x1 | 0x2 | if k + 1 < parts.len() { 0x20 } else { 0 });
        be16(&mut v, g);
        be16(&mut v, dx as u16);
        be16(&mut v, dy as u16);
    }
    v
}

/// A TrueType font mapping each character to its glyph (sorted by character), with pair kerning.
pub fn build(units_per_em: u16, glyphs: &[(char, TestGlyph)], kern: &[(char, char, i16)]) -> Vec<u8> {
    let mut glyphs = glyphs.to_vec();
    glyphs.sort_by_key(|g| g.0);
    let id = |c: char| glyphs.iter().position(|g| g.0 == c).map_or(0, |k| k as u16 + 1);
    let table = (!kern.is_empty()).then(|| {
        let mut k = Vec::new();
        be16(&mut k, 0);
        be16(&mut k, 1);
        be16(&mut k, 0);
        be16(&mut k, (14 + 6 * kern.len()) as u16);
        be16(&mut k, 1);
        be16(&mut k, kern.len() as u16);
        k.extend([0; 6]);
        for &(a, b, v) in kern {
            be16(&mut k, id(a));
            be16(&mut k, id(b));
            be16(&mut k, v as u16);
        }
        k
    });
    let raw: Vec<(char, Vec<u8>, u16)> = glyphs.iter().map(|(c, g)| (*c, simple(g), g.advance)).collect();
    build_raw(units_per_em, &raw, table)
}

/// A font of glyf records as given (glyph k + 1 is the k-th character's, sorted), and a kern
/// table's bytes.
pub fn build_raw(units_per_em: u16, glyphs: &[(char, Vec<u8>, u16)], kern: Option<Vec<u8>>) -> Vec<u8> {
    let n = glyphs.len() + 1;
    let mut glyf = Vec::new();
    // glyph 0, the missing glyph, is empty
    let mut loca = vec![0u32, 0];
    for (_, g, _) in glyphs {
        glyf.extend(g);
        while glyf.len() % 4 != 0 {
            glyf.push(0);
        }
        loca.push(glyf.len() as u32);
    }
    let mut head = Vec::new();
    be32(&mut head, 0x0001_0000);
    head.extend([0; 14]);
    be16(&mut head, units_per_em);
    head.extend([0; 30]);
    be16(&mut head, 1); // long loca
    be16(&mut head, 0);
    let mut hhea = vec![0; 34];
    be16(&mut hhea, n as u16);
    let mut maxp = Vec::new();
    be32(&mut maxp, 0x0000_5000);
    be16(&mut maxp, n as u16);
    let mut hmtx = Vec::new();
    be32(&mut hmtx, 0);
    for (_, _, advance) in glyphs {
        be16(&mut hmtx, *advance);
        be16(&mut hmtx, 0);
    }
    let mut cmap = Vec::new();
    be16(&mut cmap, 0);
    be16(&mut cmap, 1);
    be16(&mut cmap, 3);
    be16(&mut cmap, 1);
    be32(&mut cmap, 12);
    let segs = glyphs.len() + 1;
    be16(&mut cmap, 4);
    be16(&mut cmap, (16 + 8 * segs) as u16);
    be16(&mut cmap, 0);
    be16(&mut cmap, 2 * segs as u16);
    cmap.extend([0; 6]);
    let chars: Vec<u16> = glyphs.iter().map(|g| g.0 as u16).chain([0xffff]).collect();
    chars.iter().for_each(|&c| be16(&mut cmap, c));
    be16(&mut cmap, 0);
    chars.iter().for_each(|&c| be16(&mut cmap, c));
    for (k, &c) in chars.iter().enumerate() {
        let delta = if c == 0xffff {
            1
        } else {
            (k as i32 + 1 - c as i32) as i16
        };
        be16(&mut cmap, delta as u16);
    }
    chars.iter().for_each(|_| be16(&mut cmap, 0));
    let mut tables: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"head", head),
        (b"hhea", hhea),
        (b"maxp", maxp),
        (b"hmtx", hmtx),
        (b"cmap", cmap),
        (b"loca", loca.iter().flat_map(|o| o.to_be_bytes()).collect()),
        (b"glyf", glyf),
    ];
    if let Some(k) = kern {
        tables.push((b"kern", k));
    }
    let mut out = Vec::new();
    be32(&mut out, 0x0001_0000);
    be16(&mut out, tables.len() as u16);
    out.extend([0; 6]);
    let mut at = 12 + 16 * tables.len();
    for (tag, t) in &tables {
        out.extend(*tag);
        be32(&mut out, 0);
        be32(&mut out, at as u32);
        be32(&mut out, t.len() as u32);
        at += t.len().div_ceil(4) * 4;
    }
    for (_, t) in &tables {
        out.extend(t);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }
    out
}

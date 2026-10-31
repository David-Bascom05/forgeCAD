//! A TrueType outline reader: glyphs as closed contours of lines and quadratic arcs (exact
//! parabolas), with advances from hmtx and pair kerning from the kern table. Reads the head,
//! hhea, maxp, cmap (formats 4 and 12), loca, glyf (simple and composite glyphs), hmtx and kern
//! tables. CFF outlines (OpenType 'OTTO' fonts) are refused.

pub mod testing;

use forge_math::Point2;

/// A piece of a glyph's outline, in font units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Line(Point2, Point2),
    /// From the first point to the last, pulled towards the control point between.
    Quad(Point2, Point2, Point2),
}

impl Seg {
    pub fn start(&self) -> Point2 {
        match *self {
            Seg::Line(a, _) | Seg::Quad(a, _, _) => a,
        }
    }
    pub fn end(&self) -> Point2 {
        match *self {
            Seg::Line(_, b) | Seg::Quad(_, _, b) => b,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FontError {
    /// The data ends early, or a table points outside it.
    Truncated,
    /// A table the reader needs is missing.
    Missing(&'static str),
    /// CFF (PostScript) outlines: not read yet; a TrueType (glyf) font is needed.
    Cff,
    /// Not a TrueType font, or something in it this reader does not follow.
    Unsupported(&'static str),
}

/// A parsed font: its tables' bytes and what the reader looked up once.
#[derive(Clone, Debug)]
pub struct Font {
    data: Vec<u8>,
    pub units_per_em: f64,
    glyphs: usize,
    long_loca: bool,
    loca: (usize, usize),
    glyf: (usize, usize),
    hmtx: (usize, usize),
    metrics: usize,
    /// Character ranges to glyph ids: (first, last, first glyph, or the id array's offset).
    cmap: Vec<(u32, u32, Map)>,
    kern: Vec<(u16, u16, i16)>,
}

#[derive(Clone, Copy, Debug)]
enum Map {
    /// Glyph id = char + delta (mod 65536).
    Delta(i64),
    /// Glyph id from the array at this offset, then + delta.
    Array(usize, i64),
}

fn u16_at(d: &[u8], o: usize) -> Result<u16, FontError> {
    d.get(o..o + 2)
        .map(|b| u16::from_be_bytes([b[0], b[1]]))
        .ok_or(FontError::Truncated)
}

fn i16_at(d: &[u8], o: usize) -> Result<i16, FontError> {
    u16_at(d, o).map(|v| v as i16)
}

fn u32_at(d: &[u8], o: usize) -> Result<u32, FontError> {
    d.get(o..o + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(FontError::Truncated)
}

impl Font {
    pub fn parse(data: Vec<u8>) -> Result<Font, FontError> {
        let d = &data[..];
        let version = u32_at(d, 0)?;
        if version == u32::from_be_bytes(*b"OTTO") {
            return Err(FontError::Cff);
        }
        if version != 0x0001_0000 && version != u32::from_be_bytes(*b"true") {
            return Err(FontError::Unsupported("not a TrueType font"));
        }
        let n = u16_at(d, 4)? as usize;
        let mut tables = Vec::with_capacity(n);
        for k in 0..n {
            let r = 12 + 16 * k;
            let tag = d.get(r..r + 4).ok_or(FontError::Truncated)?;
            let (off, len) = (u32_at(d, r + 8)? as usize, u32_at(d, r + 12)? as usize);
            if off.checked_add(len).is_none_or(|e| e > d.len()) {
                return Err(FontError::Truncated);
            }
            tables.push(([tag[0], tag[1], tag[2], tag[3]], off, len));
        }
        let find = |t: &[u8; 4]| tables.iter().find(|x| &x.0 == t).map(|x| (x.1, x.2));
        let need =
            |t: &'static str| find(t.as_bytes().try_into().expect("four bytes")).ok_or(FontError::Missing(t));
        if find(b"glyf").is_none() && (find(b"CFF ").is_some() || find(b"CFF2").is_some()) {
            return Err(FontError::Cff);
        }
        let (head, maxp, hhea) = (need("head")?.0, need("maxp")?.0, need("hhea")?.0);
        let units_per_em = u16_at(d, head + 18)? as f64;
        if units_per_em == 0.0 {
            return Err(FontError::Unsupported("zero units per em"));
        }
        let long_loca = i16_at(d, head + 50)? != 0;
        let glyphs = u16_at(d, maxp + 4)? as usize;
        let metrics = u16_at(d, hhea + 34)? as usize;
        let (hmtx, loca, glyf) = (need("hmtx")?, need("loca")?, need("glyf")?);
        if metrics == 0 || hmtx.1 < 4 * metrics || loca.1 < (glyphs + 1) * if long_loca { 4 } else { 2 } {
            return Err(FontError::Truncated);
        }
        let cmap = read_cmap(d, need("cmap")?.0)?;
        let kern = match find(b"kern") {
            Some((k, _)) => read_kern(d, k)?,
            None => Vec::new(),
        };
        Ok(Font {
            data,
            units_per_em,
            glyphs,
            long_loca,
            loca,
            glyf,
            hmtx,
            metrics,
            cmap,
            kern,
        })
    }

    /// The glyph id a character maps to (0, the missing glyph, if none).
    pub fn glyph(&self, c: char) -> u16 {
        let c = c as u32;
        for &(lo, hi, m) in &self.cmap {
            if c < lo || c > hi {
                continue;
            }
            let g = match m {
                Map::Delta(delta) => (c as i64 + delta).rem_euclid(65536),
                Map::Array(at, delta) => match u16_at(&self.data, at + 2 * (c - lo) as usize) {
                    Ok(0) | Err(_) => 0,
                    Ok(g) => (g as i64 + delta).rem_euclid(65536),
                },
            };
            return if (g as usize) < self.glyphs { g as u16 } else { 0 };
        }
        0
    }

    /// How far the pen moves after the glyph, in font units.
    pub fn advance(&self, g: u16) -> f64 {
        let k = (g as usize).min(self.metrics - 1);
        u16_at(&self.data, self.hmtx.0 + 4 * k).map_or(0.0, |a| a as f64)
    }

    /// The kerning between two glyphs set side by side, in font units.
    pub fn kerning(&self, left: u16, right: u16) -> f64 {
        self.kern
            .binary_search_by(|p| (p.0, p.1).cmp(&(left, right)))
            .map_or(0.0, |i| self.kern[i].2 as f64)
    }

    /// The glyph's closed contours, in font units.
    pub fn outline(&self, g: u16) -> Result<Vec<Vec<Seg>>, FontError> {
        let mut out = Vec::new();
        self.outline_into(g, [1.0, 0.0, 0.0, 1.0], (0.0, 0.0), 0, &mut out)?;
        Ok(out)
    }

    fn glyph_range(&self, g: u16) -> Result<(usize, usize), FontError> {
        let g = g as usize;
        if g >= self.glyphs {
            return Err(FontError::Unsupported("glyph id out of range"));
        }
        let at = |k: usize| -> Result<usize, FontError> {
            Ok(if self.long_loca {
                u32_at(&self.data, self.loca.0 + 4 * k)? as usize
            } else {
                2 * u16_at(&self.data, self.loca.0 + 2 * k)? as usize
            })
        };
        let (a, b) = (at(g)?, at(g + 1)?);
        if a > b || b > self.glyf.1 {
            return Err(FontError::Truncated);
        }
        Ok((self.glyf.0 + a, self.glyf.0 + b))
    }

    /// Appends glyph g's contours, mapped by the matrix m and the offset.
    fn outline_into(
        &self,
        g: u16,
        m: [f64; 4],
        off: (f64, f64),
        depth: usize,
        out: &mut Vec<Vec<Seg>>,
    ) -> Result<(), FontError> {
        if depth > 8 {
            return Err(FontError::Unsupported("composite glyphs nested too deep"));
        }
        let (a, b) = self.glyph_range(g)?;
        if a == b {
            return Ok(());
        }
        let d = &self.data[..b];
        let contours = i16_at(d, a)?;
        let map = |x: f64, y: f64| Point2::new(m[0] * x + m[2] * y + off.0, m[1] * x + m[3] * y + off.1);
        if contours >= 0 {
            let n = contours as usize;
            let mut ends = Vec::with_capacity(n);
            for k in 0..n {
                ends.push(u16_at(d, a + 10 + 2 * k)? as usize);
            }
            let points = ends.last().map_or(0, |e| e + 1);
            let ins = u16_at(d, a + 10 + 2 * n)? as usize;
            let mut o = a + 12 + 2 * n + ins;
            let mut flags = Vec::with_capacity(points);
            while flags.len() < points {
                let f = *d.get(o).ok_or(FontError::Truncated)?;
                o += 1;
                flags.push(f);
                if f & 8 != 0 {
                    let r = *d.get(o).ok_or(FontError::Truncated)?;
                    o += 1;
                    flags.extend(std::iter::repeat_n(f, r as usize));
                }
            }
            flags.truncate(points);
            let mut coords = |short: u8, same: u8| -> Result<Vec<f64>, FontError> {
                let mut v = 0i32;
                let mut out = Vec::with_capacity(points);
                for &f in &flags {
                    if f & short != 0 {
                        let dv = *d.get(o).ok_or(FontError::Truncated)? as i32;
                        o += 1;
                        v += if f & same != 0 { dv } else { -dv };
                    } else if f & same == 0 {
                        v += i16_at(d, o)? as i32;
                        o += 2;
                    }
                    out.push(v as f64);
                }
                Ok(out)
            };
            let xs = coords(2, 16)?;
            let ys = coords(4, 32)?;
            let mut first = 0;
            for &e in &ends {
                if e < first || e >= points {
                    return Err(FontError::Truncated);
                }
                let pts: Vec<(Point2, bool)> = (first..=e)
                    .map(|k| (map(xs[k], ys[k]), flags[k] & 1 != 0))
                    .collect();
                let c = contour(&pts);
                if !c.is_empty() {
                    out.push(c);
                }
                first = e + 1;
            }
            return Ok(());
        }
        let mut o = a + 10;
        loop {
            let flags = u16_at(d, o)?;
            let part = u16_at(d, o + 2)?;
            o += 4;
            let (dx, dy) = if flags & 1 != 0 {
                let v = (i16_at(d, o)? as f64, i16_at(d, o + 2)? as f64);
                o += 4;
                v
            } else {
                let v = (
                    *d.get(o).ok_or(FontError::Truncated)? as i8 as f64,
                    *d.get(o + 1).ok_or(FontError::Truncated)? as i8 as f64,
                );
                o += 2;
                v
            };
            if flags & 2 == 0 {
                return Err(FontError::Unsupported(
                    "composite glyphs placed by matching points",
                ));
            }
            let f2 = |o: usize| -> Result<f64, FontError> { Ok(i16_at(d, o)? as f64 / 16384.0) };
            let s = if flags & 8 != 0 {
                let s = f2(o)?;
                o += 2;
                [s, 0.0, 0.0, s]
            } else if flags & 0x40 != 0 {
                let s = [f2(o)?, 0.0, 0.0, f2(o + 2)?];
                o += 4;
                s
            } else if flags & 0x80 != 0 {
                let s = [f2(o)?, f2(o + 2)?, f2(o + 4)?, f2(o + 6)?];
                o += 8;
                s
            } else {
                [1.0, 0.0, 0.0, 1.0]
            };
            // the part's matrix, then its offset, then this glyph's mapping
            let mm = [
                m[0] * s[0] + m[2] * s[1],
                m[1] * s[0] + m[3] * s[1],
                m[0] * s[2] + m[2] * s[3],
                m[1] * s[2] + m[3] * s[3],
            ];
            let at = map(dx, dy);
            self.outline_into(part, mm, (at.x, at.y), depth + 1, out)?;
            if flags & 0x20 == 0 {
                return Ok(());
            }
        }
    }
}

/// A closed contour from TrueType points (on or off the curve): an off point between two others
/// is a quadratic's control, two off points in a row have an on point implied midway.
fn contour(pts: &[(Point2, bool)]) -> Vec<Seg> {
    let n = pts.len();
    if n < 2 {
        return Vec::new();
    }
    let mid = |a: Point2, b: Point2| Point2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    // start on an on-curve point, or midway between the first two off ones; end back there
    let (start, order): (Point2, Vec<(Point2, bool)>) = match pts.iter().position(|p| p.1) {
        Some(k) => (pts[k].0, pts[k + 1..].iter().chain(&pts[..k]).copied().collect()),
        None => (
            mid(pts[0].0, pts[1].0),
            pts[1..].iter().chain(&pts[..1]).copied().collect(),
        ),
    };
    let mut out = Vec::new();
    let (mut at, mut ctrl) = (start, None);
    for (p, on) in order.into_iter().chain([(start, true)]) {
        match (on, ctrl) {
            (true, None) => {
                if p != at {
                    out.push(Seg::Line(at, p));
                }
                at = p;
            }
            (true, Some(c)) => {
                out.push(Seg::Quad(at, c, p));
                at = p;
                ctrl = None;
            }
            (false, None) => ctrl = Some(p),
            (false, Some(c)) => {
                let m = mid(c, p);
                out.push(Seg::Quad(at, c, m));
                at = m;
                ctrl = Some(p);
            }
        }
    }
    out
}

fn read_cmap(d: &[u8], at: usize) -> Result<Vec<(u32, u32, Map)>, FontError> {
    let n = u16_at(d, at + 2)? as usize;
    let mut best: Option<(u32, usize)> = None;
    for k in 0..n {
        let r = at + 4 + 8 * k;
        let (plat, enc) = (u16_at(d, r)?, u16_at(d, r + 2)?);
        let sub = at + u32_at(d, r + 4)? as usize;
        let format = u16_at(d, sub)?;
        // full Unicode tables first, then the BMP ones
        let rank = match (plat, enc, format) {
            (3, 10, 12) | (0, 4, 12) | (0, 6, 12) => 3,
            (3, 1, 4) | (0, _, 4) => 2,
            _ => 0,
        };
        if rank > 0 && best.is_none_or(|b| rank > b.0) {
            best = Some((rank, sub));
        }
    }
    let (_, sub) = best.ok_or(FontError::Unsupported("no Unicode cmap of format 4 or 12"))?;
    let mut out = Vec::new();
    if u16_at(d, sub)? == 12 {
        let groups = u32_at(d, sub + 12)? as usize;
        for k in 0..groups {
            let g = sub + 16 + 12 * k;
            let (lo, hi, first) = (u32_at(d, g)?, u32_at(d, g + 4)?, u32_at(d, g + 8)?);
            out.push((lo, hi, Map::Delta(first as i64 - lo as i64)));
        }
    } else {
        let segs = u16_at(d, sub + 6)? as usize / 2;
        let ends = sub + 14;
        let starts = ends + 2 * segs + 2;
        let deltas = starts + 2 * segs;
        let ranges = deltas + 2 * segs;
        for k in 0..segs {
            let (hi, lo) = (u16_at(d, ends + 2 * k)? as u32, u16_at(d, starts + 2 * k)? as u32);
            let delta = i16_at(d, deltas + 2 * k)? as i64;
            let ro = u16_at(d, ranges + 2 * k)? as usize;
            if lo > hi || lo == 0xffff {
                continue;
            }
            let m = if ro == 0 {
                Map::Delta(delta)
            } else {
                Map::Array(ranges + 2 * k + ro, delta)
            };
            out.push((lo, hi, m));
        }
    }
    Ok(out)
}

fn read_kern(d: &[u8], at: usize) -> Result<Vec<(u16, u16, i16)>, FontError> {
    let mut out = Vec::new();
    if u16_at(d, at)? != 0 {
        // the Apple version 1 table: not read, no kerning
        return Ok(out);
    }
    let n = u16_at(d, at + 2)? as usize;
    let mut o = at + 4;
    for _ in 0..n {
        let (len, coverage) = (u16_at(d, o + 2)? as usize, u16_at(d, o + 4)?);
        // format 0, horizontal, not cross-stream or a minimum table
        if coverage >> 8 == 0 && coverage & 0b111 == 1 {
            let pairs = u16_at(d, o + 6)? as usize;
            for k in 0..pairs {
                let p = o + 14 + 6 * k;
                out.push((u16_at(d, p)?, u16_at(d, p + 2)?, i16_at(d, p + 4)?));
            }
        }
        o += len.max(6);
    }
    out.sort_by_key(|p| (p.0, p.1));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestGlyph, build, square_ring};

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    #[test]
    fn reads_a_written_font() {
        let ring = square_ring();
        let font = Font::parse(build(
            1000,
            &[
                ('I', TestGlyph::bar(100.0, 0.0, 200.0, 700.0, 300.0)),
                ('O', ring.clone()),
            ],
            &[('I', 'O', -40)],
        ))
        .expect("parses");
        assert_eq!(font.units_per_em, 1000.0);
        let (gi, go) = (font.glyph('I'), font.glyph('O'));
        assert!(gi > 0 && go > 0 && gi != go);
        assert_eq!(font.glyph('x'), 0);
        assert_eq!(font.advance(gi), 300.0);
        assert_eq!(font.kerning(gi, go), -40.0);
        assert_eq!(font.kerning(go, gi), 0.0);
        let bar = font.outline(gi).expect("outline");
        assert_eq!(bar.len(), 1);
        assert_eq!(bar[0].len(), 4);
        assert!(bar[0].iter().all(|s| matches!(s, Seg::Line(..))));
        let o = font.outline(go).expect("outline");
        assert_eq!(o, ring.contours);
        for c in &o {
            for k in 0..c.len() {
                assert_eq!(c[k].end(), c[(k + 1) % c.len()].start());
            }
        }
    }

    #[test]
    fn implied_on_points_and_off_starts() {
        // a diamond of four off-curve points: four quads through the implied midpoints
        let pts = [
            (p(0.0, -10.0), false),
            (p(10.0, 0.0), false),
            (p(0.0, 10.0), false),
            (p(-10.0, 0.0), false),
        ];
        let c = contour(&pts);
        assert_eq!(c.len(), 4);
        assert_eq!(c[0], Seg::Quad(p(5.0, -5.0), p(10.0, 0.0), p(5.0, 5.0)));
        assert_eq!(c[3].end(), c[0].start());
        let mixed = [(p(0.0, 0.0), false), (p(10.0, 0.0), true), (p(10.0, 10.0), true)];
        let c = contour(&mixed);
        assert_eq!(c.len(), 2);
        assert_eq!(c[1], Seg::Quad(p(10.0, 10.0), p(0.0, 0.0), p(10.0, 0.0)));
    }

    #[test]
    fn cff_and_junk_are_refused() {
        let mut otto = b"OTTO".to_vec();
        otto.extend([0; 8]);
        assert_eq!(Font::parse(otto).unwrap_err(), FontError::Cff);
        assert_eq!(Font::parse(vec![0, 1]).unwrap_err(), FontError::Truncated);
        assert!(matches!(
            Font::parse(b"wOFF0000000000000000".to_vec()),
            Err(FontError::Unsupported(_))
        ));
        let mut cut = build(1000, &[('I', TestGlyph::bar(0.0, 0.0, 1.0, 1.0, 2.0))], &[]);
        cut.truncate(cut.len() / 2);
        assert!(Font::parse(cut).is_err());
    }
}

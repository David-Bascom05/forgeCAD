//! TrueType styles: forge-font's glyph outlines flattened into even-odd rings, scaled so the
//! cap height is one em (OS/2's sCapHeight, else the height of H, else 0.7 em), with the font's
//! advances and pair kerning. The family and full names (the name table) are read too, since
//! some styles name a family ("Arial") rather than a file.

use super::glyph::Glyph;
use forge_font::{Font, FontError, Seg};

pub struct TtfFont {
    font: Font,
    /// The cap height, in font units.
    cap: f64,
    /// Family and full names, lower case.
    pub names: Vec<String>,
}

fn be16(d: &[u8], o: usize) -> Option<u16> {
    d.get(o..o + 2).map(|b| u16::from_be_bytes([b[0], b[1]]))
}

fn be32(d: &[u8], o: usize) -> Option<u32> {
    d.get(o..o + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// A table's (offset, length).
fn table(d: &[u8], tag: &[u8; 4]) -> Option<(usize, usize)> {
    let n = be16(d, 4)? as usize;
    (0..n).find_map(|k| {
        let r = 12 + 16 * k;
        (d.get(r..r + 4)? == tag).then_some(())?;
        let (o, l) = (be32(d, r + 8)? as usize, be32(d, r + 12)? as usize);
        (o.checked_add(l)? <= d.len()).then_some((o, l))
    })
}

/// OS/2's sCapHeight (version 2 on), in font units.
fn os2_cap(d: &[u8]) -> Option<f64> {
    let (o, l) = table(d, b"OS/2")?;
    if be16(d, o)? < 2 || l < 90 {
        return None;
    }
    let cap = be16(d, o + 88)? as i16;
    (cap > 0).then_some(cap as f64)
}

/// The family (1), full (4) and typographic family (16) names.
fn names(d: &[u8]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let Some((o, l)) = table(d, b"name") else {
        return out;
    };
    let t = &d[o..o + l];
    let (Some(count), Some(strings)) = (be16(t, 2), be16(t, 4)) else {
        return out;
    };
    for k in 0..count as usize {
        let r = 6 + 12 * k;
        let (Some(platform), Some(id), Some(len), Some(off)) =
            (be16(t, r), be16(t, r + 6), be16(t, r + 8), be16(t, r + 10))
        else {
            break;
        };
        if !matches!(id, 1 | 4 | 16) {
            continue;
        }
        let at = strings as usize + off as usize;
        let Some(b) = t.get(at..at + len as usize) else {
            continue;
        };
        let s = match platform {
            0 | 3 => String::from_utf16_lossy(
                &b.as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_be_bytes(*c))
                    .collect::<Vec<_>>(),
            ),
            1 => b.iter().map(|&c| c as char).collect(),
            _ => continue,
        };
        let s = s.trim().to_lowercase();
        if !s.is_empty() && !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

impl TtfFont {
    pub fn parse(data: Vec<u8>) -> Result<TtfFont, String> {
        let os2 = os2_cap(&data);
        let names = names(&data);
        let font = Font::parse(data).map_err(|e| match e {
            FontError::Cff => "CFF (PostScript) outlines are not read; a TrueType font is needed".to_string(),
            FontError::Truncated => "the font file ends early".to_string(),
            FontError::Missing(t) => format!("the font has no {t} table"),
            FontError::Unsupported(w) => w.to_string(),
        })?;
        let cap = os2
            .or_else(|| {
                let g = font.glyph('H');
                let top = font
                    .outline(g)
                    .ok()?
                    .iter()
                    .flatten()
                    .map(|s| s.end().y.max(s.start().y))
                    .fold(f64::NEG_INFINITY, f64::max);
                (g != 0 && top > 0.0).then_some(top)
            })
            .unwrap_or(0.7 * font.units_per_em);
        Ok(TtfFont { font, cap, names })
    }

    /// The cap height as a fraction of the em.
    pub fn cap_em(&self) -> f64 {
        self.cap / self.font.units_per_em
    }

    /// A character's rings and advance (cap height 1), or None if the font has no glyph for it.
    pub fn glyph(&self, c: char) -> Option<Glyph> {
        let g = self.font.glyph(c);
        if g == 0 {
            return None;
        }
        let k = 1.0 / self.cap;
        let tol = 0.002 * self.cap;
        let mut fills = Vec::new();
        for contour in self.font.outline(g).ok()? {
            let Some(first) = contour.first() else {
                continue;
            };
            let p0 = first.start();
            let mut ring = vec![[p0.x * k, p0.y * k]];
            for s in &contour {
                match *s {
                    Seg::Line(_, b) => ring.push([b.x * k, b.y * k]),
                    Seg::Quad(a, c, b) => {
                        // The largest gap between a parabola and its chord is a quarter of
                        // |a - 2c + b|; cut so each piece's is within tol.
                        let (ex, ey) = (a.x - 2.0 * c.x + b.x, a.y - 2.0 * c.y + b.y);
                        let dev = (ex * ex + ey * ey).sqrt() / 4.0;
                        let n = ((dev / tol).sqrt().ceil() as usize).clamp(1, 32);
                        for j in 1..=n {
                            let t = j as f64 / n as f64;
                            let u = 1.0 - t;
                            let x = u * u * a.x + 2.0 * u * t * c.x + t * t * b.x;
                            let y = u * u * a.y + 2.0 * u * t * c.y + t * t * b.y;
                            ring.push([x * k, y * k]);
                        }
                    }
                }
            }
            if ring.len() > 1 && ring.first() == ring.last() {
                ring.pop();
            }
            if ring.len() >= 3 {
                fills.push(ring);
            }
        }
        Some(Glyph {
            strokes: vec![],
            fills,
            advance: self.font.advance(g) / self.cap,
        })
    }

    /// The kerning between two characters set side by side, in em units.
    pub fn kerning(&self, a: char, b: char) -> f64 {
        self.font.kerning(self.font.glyph(a), self.font.glyph(b)) / self.cap
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use forge_font::testing::{TestGlyph, build, square_ring};

    /// Adds tables to a font's directory.
    pub fn with_tables(font: &[u8], extra: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let n = be16(font, 4).unwrap() as usize;
        let mut tables: Vec<([u8; 4], Vec<u8>)> = (0..n)
            .map(|k| {
                let r = 12 + 16 * k;
                let (o, l) = (
                    be32(font, r + 8).unwrap() as usize,
                    be32(font, r + 12).unwrap() as usize,
                );
                (font[r..r + 4].try_into().unwrap(), font[o..o + l].to_vec())
            })
            .collect();
        tables.extend(extra.iter().map(|(t, v)| (**t, v.clone())));
        let mut out = vec![0, 1, 0, 0];
        out.extend((tables.len() as u16).to_be_bytes());
        out.extend([0; 6]);
        let mut at = 12 + 16 * tables.len();
        for (tag, t) in &tables {
            out.extend(tag);
            out.extend(0u32.to_be_bytes());
            out.extend((at as u32).to_be_bytes());
            out.extend((t.len() as u32).to_be_bytes());
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

    /// A name table with a Windows (UTF-16) family name.
    pub fn name_table(family: &str) -> Vec<u8> {
        let s: Vec<u8> = family.encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
        let mut t = Vec::new();
        for v in [0u16, 1, 18, 3, 1, 0x409, 1, s.len() as u16, 0] {
            t.extend(v.to_be_bytes());
        }
        t.extend(s);
        t
    }

    pub fn test_font() -> Vec<u8> {
        build(
            1000,
            &[
                ('H', TestGlyph::bar(50.0, 0.0, 600.0, 700.0, 650.0)),
                ('O', square_ring()),
                ('V', TestGlyph::bar(0.0, 0.0, 500.0, 700.0, 600.0)),
            ],
            &[('V', 'O', -70)],
        )
    }

    #[test]
    fn cap_height_from_h() {
        let f = TtfFont::parse(test_font()).unwrap();
        assert_eq!(f.cap_em(), 0.7);
        let h = f.glyph('H').unwrap();
        assert_eq!(h.fills.len(), 1);
        let (lo, hi) = h.bbox().unwrap();
        assert!((hi[1] - 1.0).abs() < 1e-12 && lo[1] == 0.0);
        assert!((h.advance - 650.0 / 700.0).abs() < 1e-12);
        assert!((f.kerning('V', 'O') + 0.1).abs() < 1e-12);
        assert!(f.glyph('Z').is_none());
    }

    #[test]
    fn quadratics_flattened_to_rings() {
        let f = TtfFont::parse(test_font()).unwrap();
        let o = f.glyph('O').unwrap();
        assert_eq!(o.fills.len(), 2);
        // Every point of the outer ring is inside the control polygon's box and the ring has
        // more points than the four arcs' ends.
        assert!(o.fills[0].len() > 8);
        let (lo, hi) = o.bbox().unwrap();
        assert!(lo[0] >= 0.0 && hi[0] <= 600.0 / 700.0 + 1e-12 && hi[1] <= 1.0 + 1e-12);
        // The arc from (300, 0) via (0, 0) to (0, 350) passes (75, 87.5) at t = 1/2.
        let near = o.fills[0]
            .iter()
            .any(|p| ((p[0] - 75.0 / 700.0).powi(2) + (p[1] - 87.5 / 700.0).powi(2)).sqrt() < 1e-9);
        assert!(near);
    }

    #[test]
    fn os2_cap_height_and_names() {
        let mut os2 = vec![0u8; 96];
        os2[0..2].copy_from_slice(&2u16.to_be_bytes());
        os2[88..90].copy_from_slice(&500u16.to_be_bytes());
        let f = with_tables(
            &test_font(),
            &[(b"OS/2", os2), (b"name", name_table("Test Sans"))],
        );
        let f = TtfFont::parse(f).unwrap();
        assert_eq!(f.cap_em(), 0.5);
        assert_eq!(f.names, vec!["test sans".to_string()]);
    }

    #[test]
    fn refuses_non_fonts() {
        assert!(TtfFont::parse(b"OTTO and more bytes".to_vec()).is_err());
        assert!(TtfFont::parse(vec![0; 3]).is_err());
    }
}

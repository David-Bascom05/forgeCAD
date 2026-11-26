//! CFF (PostScript) outlines, as OpenType 'OTTO' fonts hold them: the CFF table's INDEXes and
//! DICTs, and Type 2 charstrings run to closed contours of lines and cubic Béziers. Hints are read
//! past and do nothing; `seac` accents and the deprecated arithmetic operators are refused.
//!
//! Every coordinate a charstring makes is a sum of its operands, each an integer or a 16.16
//! fixed-point number, so the outlines come out exact in f64 (multiples of 2^-16). The charstring
//! itself is run by `cff_core` in integers of 2^-16 units (proven in verify/aeneas/forge-font);
//! this shell reads the table and maps its points to f64.

use crate::{FontError, Seg, cff_core};
use forge_math::Point2;

/// An INDEX: where each of its objects lies in the data.
#[derive(Clone, Debug, Default)]
struct Index {
    /// count + 1 offsets, absolute.
    offs: Vec<usize>,
}

impl Index {
    fn len(&self) -> usize {
        self.offs.len().saturating_sub(1)
    }
    fn get<'a>(&self, d: &'a [u8], k: usize) -> Option<&'a [u8]> {
        let (a, b) = self.range(d, k)?;
        Some(&d[a..b])
    }
    /// Where object k lies in `d`.
    fn range(&self, d: &[u8], k: usize) -> Option<(usize, usize)> {
        let (a, b) = (*self.offs.get(k)?, *self.offs.get(k + 1)?);
        d.get(a..b)?;
        Some((a, b))
    }
}

fn byte(d: &[u8], o: usize) -> Result<u8, FontError> {
    d.get(o).copied().ok_or(FontError::Truncated)
}

/// Reads an INDEX at `o`; returns it and where the data after it starts.
fn index(d: &[u8], o: usize) -> Result<(Index, usize), FontError> {
    let count = crate::u16_at(d, o)? as usize;
    if count == 0 {
        return Ok((Index::default(), o + 2));
    }
    let size = byte(d, o + 2)? as usize;
    if !(1..=4).contains(&size) {
        return Err(FontError::Unsupported(
            "a CFF INDEX with offsets not 1 to 4 bytes",
        ));
    }
    let first = o + 3;
    let data = first
        .checked_add((count + 1) * size)
        .and_then(|e| e.checked_sub(1))
        .ok_or(FontError::Truncated)?;
    let mut offs = Vec::with_capacity(count + 1);
    for k in 0..=count {
        let at = first + k * size;
        let b = d.get(at..at + size).ok_or(FontError::Truncated)?;
        let v = b.iter().fold(0usize, |v, &x| (v << 8) | x as usize);
        let abs = data.checked_add(v).ok_or(FontError::Truncated)?;
        if abs > d.len() || offs.last().is_some_and(|&l| abs < l) || (k == 0 && v != 1) {
            return Err(FontError::Truncated);
        }
        offs.push(abs);
    }
    let end = *offs.last().expect("count + 1 offsets");
    Ok((Index { offs }, end))
}

/// A DICT's entries: (operator, operands); a two-byte operator is 1200 + its second byte.
fn dict(d: &[u8]) -> Result<Vec<(u16, Vec<f64>)>, FontError> {
    let bad = FontError::Unsupported("a CFF DICT that does not read");
    let mut out = Vec::new();
    let mut ops = Vec::new();
    let mut o = 0;
    while o < d.len() {
        let b0 = d[o];
        match b0 {
            0..=21 => {
                let op = if b0 == 12 {
                    o += 1;
                    1200 + byte(d, o)? as u16
                } else {
                    b0 as u16
                };
                o += 1;
                out.push((op, std::mem::take(&mut ops)));
            }
            28 => {
                ops.push(crate::i16_at(d, o + 1)? as f64);
                o += 3;
            }
            29 => {
                ops.push(crate::u32_at(d, o + 1)? as i32 as f64);
                o += 5;
            }
            30 => {
                // a real, in nibbles: kept to the precision it is written in
                let mut s = String::new();
                o += 1;
                'real: loop {
                    let b = byte(d, o)?;
                    o += 1;
                    for n in [b >> 4, b & 15] {
                        match n {
                            0..=9 => s.push((b'0' + n) as char),
                            0xa => s.push('.'),
                            0xb => s.push('E'),
                            0xc => s.push_str("E-"),
                            0xe => s.push('-'),
                            0xf => break 'real,
                            _ => return Err(bad),
                        }
                    }
                    if s.len() > 64 {
                        return Err(bad);
                    }
                }
                ops.push(s.parse::<f64>().map_err(|_| bad.clone())?);
            }
            32..=246 => {
                ops.push(b0 as f64 - 139.0);
                o += 1;
            }
            247..=250 => {
                ops.push((b0 as f64 - 247.0) * 256.0 + byte(d, o + 1)? as f64 + 108.0);
                o += 2;
            }
            251..=254 => {
                ops.push(-(b0 as f64 - 251.0) * 256.0 - byte(d, o + 1)? as f64 - 108.0);
                o += 2;
            }
            _ => return Err(bad),
        }
        if ops.len() > cff_core::STACK {
            return Err(bad);
        }
    }
    Ok(out)
}

fn get(dict: &[(u16, Vec<f64>)], op: u16) -> Option<&[f64]> {
    dict.iter().find(|e| e.0 == op).map(|e| &e.1[..])
}

/// An offset or size operand: a whole, non-negative number.
fn whole(v: f64) -> Result<usize, FontError> {
    if v >= 0.0 && v.fract() == 0.0 && v < (1u64 << 32) as f64 {
        Ok(v as usize)
    } else {
        Err(FontError::Unsupported("a CFF offset that is not a whole number"))
    }
}

/// One Private DICT's local subroutines.
#[derive(Clone, Debug, Default)]
struct Private {
    subrs: Index,
}

fn private(d: &[u8], at: &[f64]) -> Result<Private, FontError> {
    let [size, off] = at else {
        return Err(FontError::Unsupported(
            "a CFF Private entry without size and offset",
        ));
    };
    let (size, off) = (whole(*size)?, whole(*off)?);
    let end = off.checked_add(size).ok_or(FontError::Truncated)?;
    let p = dict(d.get(off..end).ok_or(FontError::Truncated)?)?;
    let subrs = match get(&p, 19) {
        Some([s]) => index(d, off.checked_add(whole(*s)?).ok_or(FontError::Truncated)?)?.0,
        _ => Index::default(),
    };
    Ok(Private { subrs })
}

/// A CFF table, read once: its charstrings and the subroutines they call.
#[derive(Clone, Debug)]
pub(crate) struct Cff {
    /// The table's bytes (the font's, from the table's start).
    data: Vec<u8>,
    chars: Index,
    gsubrs: Index,
    privates: Vec<Private>,
    /// Per glyph, which Private (FDSelect); empty: all the first.
    select: Vec<u8>,
}

impl Cff {
    pub(crate) fn parse(table: &[u8]) -> Result<Cff, FontError> {
        let d = table;
        if byte(d, 0)? != 1 {
            return Err(FontError::Unsupported(
                "a CFF table not of version 1 (CFF2 is not read)",
            ));
        }
        let hdr = byte(d, 2)? as usize;
        let (names, o) = index(d, hdr)?;
        if names.len() != 1 {
            return Err(FontError::Unsupported("a CFF table holding other than one font"));
        }
        let (tops, o) = index(d, o)?;
        let (_strings, o) = index(d, o)?;
        let (gsubrs, _) = index(d, o)?;
        let top = dict(tops.get(d, 0).ok_or(FontError::Truncated)?)?;
        if let Some(m) = get(&top, 1207) {
            // the outlines must be in the font's own units, as head's unitsPerEm reads them
            if m.len() != 6 || m[1] != 0.0 || m[2] != 0.0 || m[4] != 0.0 || m[5] != 0.0 || m[0] != m[3] {
                return Err(FontError::Unsupported("a CFF FontMatrix that skews or moves"));
            }
        }
        if get(&top, 1206).is_some_and(|t| t != [2.0]) {
            return Err(FontError::Unsupported("CFF charstrings not of Type 2"));
        }
        let chars = match get(&top, 17) {
            Some([c]) => index(d, whole(*c)?)?.0,
            _ => return Err(FontError::Missing("CFF CharStrings")),
        };
        let (privates, select) = if get(&top, 1230).is_some() {
            // a CID font: a Private per font dict, chosen per glyph
            let fda = match get(&top, 1236) {
                Some([a]) => index(d, whole(*a)?)?.0,
                _ => return Err(FontError::Missing("CFF FDArray")),
            };
            if fda.len() == 0 || fda.len() > 256 {
                return Err(FontError::Unsupported("a CFF FDArray of no or too many dicts"));
            }
            let mut ps = Vec::with_capacity(fda.len());
            for k in 0..fda.len() {
                let fd = dict(fda.get(d, k).ok_or(FontError::Truncated)?)?;
                ps.push(match get(&fd, 18) {
                    Some(at) => private(d, at)?,
                    None => Private::default(),
                });
            }
            let sel = match get(&top, 1237) {
                Some([s]) => fd_select(d, whole(*s)?, chars.len(), ps.len())?,
                _ => return Err(FontError::Missing("CFF FDSelect")),
            };
            (ps, sel)
        } else {
            let p = match get(&top, 18) {
                Some(at) => private(d, at)?,
                None => Private::default(),
            };
            (vec![p], Vec::new())
        };
        Ok(Cff {
            data: table.to_vec(),
            chars,
            gsubrs,
            privates,
            select,
        })
    }

    pub(crate) fn glyphs(&self) -> usize {
        self.chars.len()
    }

    /// Glyph g's closed contours (cff_core runs the charstring, in 2^-16 units).
    pub(crate) fn outline(&self, g: u16) -> Result<Vec<Vec<Seg>>, FontError> {
        let g = g as usize;
        let cs = self
            .chars
            .range(&self.data, g)
            .ok_or(FontError::Unsupported("glyph id out of range"))?;
        let p = &self.privates[*self.select.get(g).unwrap_or(&0) as usize];
        let out = cff_core::run(&self.data, cs, &self.gsubrs.offs, &p.subrs.offs)?;
        let pt = |q: cff_core::Pc| Point2::new(q.x as f64 / 65536.0, q.y as f64 / 65536.0);
        Ok(out
            .into_iter()
            .map(|c| {
                c.into_iter()
                    .map(|s| match s {
                        cff_core::SegC::Line(a, b) => Seg::Line(pt(a), pt(b)),
                        cff_core::SegC::Cubic(a, c1, c2, b) => Seg::Cubic(pt(a), pt(c1), pt(c2), pt(b)),
                    })
                    .collect()
            })
            .collect())
    }
}

fn fd_select(d: &[u8], o: usize, glyphs: usize, fds: usize) -> Result<Vec<u8>, FontError> {
    let mut sel = vec![0u8; glyphs];
    match byte(d, o)? {
        0 => {
            for (k, s) in sel.iter_mut().enumerate() {
                *s = byte(d, o + 1 + k)?;
            }
        }
        3 => {
            let n = crate::u16_at(d, o + 1)? as usize;
            for k in 0..n {
                let r = o + 3 + 3 * k;
                let (first, fd, next) = (
                    crate::u16_at(d, r)? as usize,
                    byte(d, r + 2)?,
                    crate::u16_at(d, r + 3)? as usize,
                );
                if first > next || next > glyphs {
                    return Err(FontError::Unsupported("a CFF FDSelect range out of order"));
                }
                sel[first..next].iter_mut().for_each(|s| *s = fd);
            }
        }
        _ => {
            return Err(FontError::Unsupported(
                "a CFF FDSelect of format other than 0 or 3",
            ));
        }
    }
    if sel.iter().any(|&s| s as usize >= fds) {
        return Err(FontError::Unsupported("a CFF FDSelect naming a dict not there"));
    }
    Ok(sel)
}

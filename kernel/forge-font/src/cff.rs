//! CFF (PostScript) outlines, as OpenType 'OTTO' fonts hold them: the CFF table's INDEXes and
//! DICTs, and Type 2 charstrings run to closed contours of lines and cubic Béziers. Hints are read
//! past and do nothing; `seac` accents and the deprecated arithmetic operators are refused.
//!
//! Every coordinate a charstring makes is a sum of its operands, each an integer or a 16.16
//! fixed-point number, so the outlines come out exact in f64 (multiples of 2^-16).

use crate::{FontError, Seg};
use forge_math::Point2;

/// What one outline may run: operators and operands (subroutine calls included) in all.
const STEPS: usize = 1 << 16;
/// Subroutine calls nested this deep are refused (the format's own limit).
const NESTING: usize = 10;
/// The operand stack's depth (Type 2's limit is 48; CFF2 allows 513 and is not read).
const STACK: usize = 48;

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
        let (a, b) = (*self.offs.get(k)?, *self.offs.get(k + 1)?);
        d.get(a..b)
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
        if ops.len() > STACK {
            return Err(bad);
        }
    }
    Ok(out)
}

fn get<'a>(dict: &'a [(u16, Vec<f64>)], op: u16) -> Option<&'a [f64]> {
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

    /// Glyph g's closed contours.
    pub(crate) fn outline(&self, g: u16) -> Result<Vec<Vec<Seg>>, FontError> {
        let g = g as usize;
        let cs = self
            .chars
            .get(&self.data, g)
            .ok_or(FontError::Unsupported("glyph id out of range"))?;
        let p = &self.privates[*self.select.get(g).unwrap_or(&0) as usize];
        let mut m = Machine {
            cff: self,
            local: &p.subrs,
            stack: Vec::with_capacity(STACK),
            at: (0.0, 0.0),
            start: None,
            contour: Vec::new(),
            out: Vec::new(),
            stems: 0,
            width_read: false,
            steps: STEPS,
            done: false,
        };
        m.run(cs, 0)?;
        if !m.done {
            return Err(FontError::Unsupported("a charstring that does not end"));
        }
        m.close();
        Ok(m.out)
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

fn bias(n: usize) -> i64 {
    if n < 1240 {
        107
    } else if n < 33900 {
        1131
    } else {
        32768
    }
}

/// A Type 2 charstring interpreter's state.
struct Machine<'a> {
    cff: &'a Cff,
    local: &'a Index,
    stack: Vec<f64>,
    at: (f64, f64),
    /// The open contour's first point.
    start: Option<Point2>,
    contour: Vec<Seg>,
    out: Vec<Vec<Seg>>,
    stems: usize,
    /// Whether the width (an optional first operand) has been read past.
    width_read: bool,
    steps: usize,
    done: bool,
}

impl Machine<'_> {
    fn pt(&self) -> Point2 {
        Point2::new(self.at.0, self.at.1)
    }

    fn close(&mut self) {
        if let Some(s) = self.start.take() {
            let p = self.pt();
            if p != s {
                self.contour.push(Seg::Line(p, s));
            }
            if !self.contour.is_empty() {
                self.out.push(std::mem::take(&mut self.contour));
            }
        }
    }

    fn move_to(&mut self, dx: f64, dy: f64) {
        self.close();
        self.at = (self.at.0 + dx, self.at.1 + dy);
        self.start = Some(self.pt());
    }

    fn line(&mut self, dx: f64, dy: f64) -> Result<(), FontError> {
        let a = self.pt();
        self.at = (self.at.0 + dx, self.at.1 + dy);
        self.open()?;
        let b = self.pt();
        if a != b {
            self.contour.push(Seg::Line(a, b));
        }
        Ok(())
    }

    fn curve(&mut self, d: [f64; 6]) -> Result<(), FontError> {
        let a = self.pt();
        let c1 = Point2::new(a.x + d[0], a.y + d[1]);
        let c2 = Point2::new(c1.x + d[2], c1.y + d[3]);
        let b = Point2::new(c2.x + d[4], c2.y + d[5]);
        self.at = (b.x, b.y);
        self.open()?;
        if !(a == c1 && c1 == c2 && c2 == b) {
            self.contour.push(Seg::Cubic(a, c1, c2, b));
        }
        Ok(())
    }

    /// Drawing before any moveto is an error in the format.
    fn open(&self) -> Result<(), FontError> {
        if self.start.is_none() {
            return Err(FontError::Unsupported(
                "a charstring drawing before its first moveto",
            ));
        }
        Ok(())
    }

    /// The width, if the stack holds one operand more than `want` (or an odd one for stems).
    fn width(&mut self, odd: bool, want: usize) {
        if !self.width_read {
            self.width_read = true;
            let extra = if odd {
                self.stack.len() % 2 == 1
            } else {
                self.stack.len() > want
            };
            if extra {
                self.stack.remove(0);
            }
        }
    }

    fn run(&mut self, cs: &[u8], depth: usize) -> Result<(), FontError> {
        if depth > NESTING {
            return Err(FontError::Unsupported("charstring subroutines nested too deep"));
        }
        let bad = |w: &'static str| FontError::Unsupported(w);
        let mut o = 0;
        while o < cs.len() && !self.done {
            self.steps = self
                .steps
                .checked_sub(1)
                .ok_or(bad("a charstring that runs too long"))?;
            let b0 = cs[o];
            o += 1;
            if b0 >= 32 || b0 == 28 {
                let v = match b0 {
                    28 => {
                        let v = crate::i16_at(cs, o)? as f64;
                        o += 2;
                        v
                    }
                    32..=246 => b0 as f64 - 139.0,
                    247..=250 => {
                        let v = (b0 as f64 - 247.0) * 256.0 + byte(cs, o)? as f64 + 108.0;
                        o += 1;
                        v
                    }
                    251..=254 => {
                        let v = -(b0 as f64 - 251.0) * 256.0 - byte(cs, o)? as f64 - 108.0;
                        o += 1;
                        v
                    }
                    _ => {
                        // 16.16 fixed
                        let v = crate::u32_at(cs, o)? as i32 as f64 / 65536.0;
                        o += 4;
                        v
                    }
                };
                if self.stack.len() >= STACK {
                    return Err(bad("a charstring's stack overflows"));
                }
                self.stack.push(v);
                continue;
            }
            let s = std::mem::take(&mut self.stack);
            let need = |n: usize| -> Result<(), FontError> {
                if s.len() < n {
                    Err(bad("a charstring operator short of operands"))
                } else {
                    Ok(())
                }
            };
            match b0 {
                // hstem vstem hstemhm vstemhm
                1 | 3 | 18 | 23 => {
                    self.stack = s;
                    self.width(true, 0);
                    self.stems += self.stack.len() / 2;
                    self.stack.clear();
                }
                // hintmask cntrmask: an implied vstem first, then the mask's bytes
                19 | 20 => {
                    self.stack = s;
                    self.width(true, 0);
                    self.stems += self.stack.len() / 2;
                    self.stack.clear();
                    o += self.stems.div_ceil(8);
                    if o > cs.len() {
                        return Err(FontError::Truncated);
                    }
                }
                // rmoveto hmoveto vmoveto
                21 | 22 | 4 => {
                    self.stack = s;
                    self.width(false, if b0 == 21 { 2 } else { 1 });
                    let s = std::mem::take(&mut self.stack);
                    match (b0, &s[..]) {
                        (21, [dx, dy]) => self.move_to(*dx, *dy),
                        (22, [dx]) => self.move_to(*dx, 0.0),
                        (4, [dy]) => self.move_to(0.0, *dy),
                        _ => return Err(bad("a charstring moveto of the wrong operands")),
                    }
                }
                // rlineto
                5 => {
                    if s.is_empty() || s.len() % 2 != 0 {
                        return Err(bad("rlineto of odd operands"));
                    }
                    for p in s.chunks(2) {
                        self.line(p[0], p[1])?;
                    }
                }
                // hlineto vlineto: alternating
                6 | 7 => {
                    need(1)?;
                    let mut h = b0 == 6;
                    for &v in &s {
                        if h {
                            self.line(v, 0.0)?;
                        } else {
                            self.line(0.0, v)?;
                        }
                        h = !h;
                    }
                }
                // rrcurveto
                8 => {
                    if s.is_empty() || s.len() % 6 != 0 {
                        return Err(bad("rrcurveto of operands not in sixes"));
                    }
                    for c in s.chunks(6) {
                        self.curve([c[0], c[1], c[2], c[3], c[4], c[5]])?;
                    }
                }
                // rcurveline: curves, then a line
                24 => {
                    if s.len() < 8 || (s.len() - 2) % 6 != 0 {
                        return Err(bad("rcurveline of the wrong operands"));
                    }
                    let (cs6, l) = s.split_at(s.len() - 2);
                    for c in cs6.chunks(6) {
                        self.curve([c[0], c[1], c[2], c[3], c[4], c[5]])?;
                    }
                    self.line(l[0], l[1])?;
                }
                // rlinecurve: lines, then a curve
                25 => {
                    if s.len() < 8 || (s.len() - 6) % 2 != 0 {
                        return Err(bad("rlinecurve of the wrong operands"));
                    }
                    let (ls, c) = s.split_at(s.len() - 6);
                    for p in ls.chunks(2) {
                        self.line(p[0], p[1])?;
                    }
                    self.curve([c[0], c[1], c[2], c[3], c[4], c[5]])?;
                }
                // vvcurveto: dx1? {dya dxb dyb dyc}+
                26 => {
                    let (mut dx1, rest) = if s.len() % 4 == 1 {
                        (s[0], &s[1..])
                    } else {
                        (0.0, &s[..])
                    };
                    if rest.is_empty() || rest.len() % 4 != 0 {
                        return Err(bad("vvcurveto of the wrong operands"));
                    }
                    for c in rest.chunks(4) {
                        self.curve([dx1, c[0], c[1], c[2], 0.0, c[3]])?;
                        dx1 = 0.0;
                    }
                }
                // hhcurveto: dy1? {dxa dxb dyb dxc}+
                27 => {
                    let (mut dy1, rest) = if s.len() % 4 == 1 {
                        (s[0], &s[1..])
                    } else {
                        (0.0, &s[..])
                    };
                    if rest.is_empty() || rest.len() % 4 != 0 {
                        return Err(bad("hhcurveto of the wrong operands"));
                    }
                    for c in rest.chunks(4) {
                        self.curve([c[0], dy1, c[1], c[2], c[3], 0.0])?;
                        dy1 = 0.0;
                    }
                }
                // vhcurveto hvcurveto: alternating, the last with an optional final delta
                30 | 31 => {
                    if s.len() < 4 || !(s.len() % 4 == 0 || s.len() % 4 == 1) {
                        return Err(bad("vhcurveto or hvcurveto of the wrong operands"));
                    }
                    let mut h = b0 == 31;
                    let n = s.len() / 4;
                    for k in 0..n {
                        let c = &s[4 * k..4 * k + 4];
                        let last = if k + 1 == n && s.len() % 4 == 1 {
                            s[s.len() - 1]
                        } else {
                            0.0
                        };
                        if h {
                            self.curve([c[0], 0.0, c[1], c[2], last, c[3]])?;
                        } else {
                            self.curve([0.0, c[0], c[1], c[2], c[3], last])?;
                        }
                        h = !h;
                    }
                }
                // callsubr callgsubr
                10 | 29 => {
                    let mut s = s;
                    let k = s.pop().ok_or(bad("a subroutine call with no number"))?;
                    let idx = if b0 == 10 { self.local } else { &self.cff.gsubrs };
                    if k.fract() != 0.0 {
                        return Err(bad("a subroutine number that is not whole"));
                    }
                    let n = k as i64 + bias(idx.len());
                    let sub = usize::try_from(n)
                        .ok()
                        .and_then(|n| idx.get(&self.cff.data, n))
                        .ok_or(bad("a call to a subroutine not there"))?;
                    self.stack = s;
                    self.run(sub, depth + 1)?;
                }
                // return
                11 => {
                    self.stack = s;
                    return Ok(());
                }
                // endchar
                14 => {
                    self.stack = s;
                    self.width(true, 0);
                    if self.stack.len() >= 4 {
                        return Err(bad("an accented glyph built by endchar (seac)"));
                    }
                    self.close();
                    self.done = true;
                }
                12 => {
                    let b1 = byte(cs, o)?;
                    o += 1;
                    match (b1, &s[..]) {
                        // flex
                        (35, [a, b, c, d, e, f, g, h, i, j, k, l, _fd]) => {
                            self.curve([*a, *b, *c, *d, *e, *f])?;
                            self.curve([*g, *h, *i, *j, *k, *l])?;
                        }
                        // hflex
                        (34, [dx1, dx2, dy2, dx3, dx4, dx5, dx6]) => {
                            self.curve([*dx1, 0.0, *dx2, *dy2, *dx3, 0.0])?;
                            self.curve([*dx4, 0.0, *dx5, -*dy2, *dx6, 0.0])?;
                        }
                        // hflex1
                        (36, [dx1, dy1, dx2, dy2, dx3, dx4, dx5, dy5, dx6]) => {
                            self.curve([*dx1, *dy1, *dx2, *dy2, *dx3, 0.0])?;
                            self.curve([*dx4, 0.0, *dx5, *dy5, *dx6, -(*dy1 + *dy2 + *dy5)])?;
                        }
                        // flex1: the last point's other delta returns to the start's line
                        (37, [d1, d2, d3, d4, d5, d6, d7, d8, d9, d10, d11]) => {
                            let dx = d1 + d3 + d5 + d7 + d9;
                            let dy = d2 + d4 + d6 + d8 + d10;
                            let (ex, ey) = if dx.abs() > dy.abs() {
                                (*d11, -dy)
                            } else {
                                (-dx, *d11)
                            };
                            self.curve([*d1, *d2, *d3, *d4, *d5, *d6])?;
                            self.curve([*d7, *d8, *d9, *d10, ex, ey])?;
                        }
                        (34..=37, _) => return Err(bad("a flex of the wrong operands")),
                        _ => return Err(bad("a deprecated or unknown charstring operator")),
                    }
                }
                _ => return Err(bad("an unknown charstring operator")),
            }
        }
        Ok(())
    }
}

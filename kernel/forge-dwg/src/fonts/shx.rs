//! Compiled AutoCAD shape fonts (.shx): "AutoCAD-86 shapes 1.0" and "1.1" files, unifonts and big
//! fonts, read from the published shape-description format (AutoCAD's Customization Guide,
//! "Shape Descriptions" and "Extended Big Font Files").
//!
//! A file holds numbered shapes, each a byte program for a pen: a byte 0xLD moves L units in
//! direction D (sixteen directions, the odd ones half a unit off the axis), and the codes 0 to 14
//! lift and lower the pen, scale, push and pop the position, call other shapes, move by X-Y
//! displacements and draw arcs (by octants, fractions of octants, or by bulge). Shape 0 is the
//! font's description: its name and how far capitals reach above the baseline, which is the cap
//! height here.
//!
//! The layouts:
//! - shapes 1.0 / 1.1: the signature, then the first and last shape numbers and the count (u16
//!   each), an index of (number, length) u16 pairs, and the definitions in index order.
//! - unifont: the signature, the shape count (u32), the font description's length (u16) and the
//!   description, then each shape as its number, length and definition.
//! - bigfont: the signature, the index entry count's companion length (u16), the shape count and
//!   the escape-range count (u16 each), the ranges (first, last lead byte as u16), then an index
//!   of (number u16, length u16, file offset u32).
//!
//! Every definition is its name, a NUL, then the bytes. Numbers are little-endian, but a shape
//! number inside a definition (a unifont's or big font's subshape) is high byte first, as the
//! shape compiler writes the source's bytes.

use super::glyph::{Glyph, arc_points};
use forge_math::dmath;
use std::collections::HashMap;

/// Polylines.
pub type Strokes = Vec<Vec<[f64; 2]>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Shapes,
    Unifont,
    Bigfont,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShxError {
    /// No "AutoCAD-86" signature.
    NotShx,
    /// The signature names something else.
    Unknown(String),
    /// The index or a definition runs past the end.
    Truncated,
}

impl std::fmt::Display for ShxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShxError::NotShx => write!(f, "not an SHX file"),
            ShxError::Unknown(s) => write!(f, "an SHX file of a kind not read: {s}"),
            ShxError::Truncated => write!(f, "the SHX file ends early"),
        }
    }
}

/// A parsed SHX file: its description and an index into its definitions.
#[derive(Clone, Debug)]
pub struct ShxFont {
    pub kind: Kind,
    /// Shape 0's name: the font's description.
    pub name: String,
    /// Units above the baseline capitals reach (a big font's character height).
    pub above: f64,
    pub below: f64,
    /// 0 horizontal only, 2 horizontal or vertical (code 14 matters).
    pub modes: u8,
    /// An extended big font's character width (its primitives' square scaled to this).
    pub width: Option<f64>,
    /// A big font's escape (lead byte) ranges.
    pub ranges: Vec<(u16, u16)>,
    /// Whether shape 0 described a text font (else a plain shape file, e.g. line-type shapes).
    pub text: bool,
    data: Vec<u8>,
    /// Shape number to the (start, end) of its program, past the name.
    shapes: HashMap<u16, (usize, usize)>,
}

fn u16_at(d: &[u8], o: usize) -> Result<u16, ShxError> {
    d.get(o..o + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or(ShxError::Truncated)
}

fn u32_at(d: &[u8], o: usize) -> Result<u32, ShxError> {
    d.get(o..o + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(ShxError::Truncated)
}

impl ShxFont {
    pub fn parse(data: Vec<u8>) -> Result<ShxFont, ShxError> {
        let head_end = data
            .iter()
            .take(64)
            .position(|&b| b == 0x1a)
            .ok_or(ShxError::NotShx)?;
        let sig = String::from_utf8_lossy(&data[..head_end]).trim().to_string();
        let words: Vec<&str> = sig.split_whitespace().collect();
        if words.first() != Some(&"AutoCAD-86") {
            return Err(ShxError::NotShx);
        }
        let kind = match words.get(1).map(|w| w.to_ascii_lowercase()).as_deref() {
            Some("shapes") => Kind::Shapes,
            Some("unifont") => Kind::Unifont,
            Some("bigfont") => Kind::Bigfont,
            _ => return Err(ShxError::Unknown(sig)),
        };
        let mut at = head_end + 1;
        let mut ranges = Vec::new();
        let mut spans: Vec<(u16, usize, usize)> = Vec::new();
        let d = &data[..];
        match kind {
            Kind::Shapes => {
                let n = u16_at(d, at + 4)? as usize;
                at += 6;
                let mut pos = at + 4 * n;
                for k in 0..n {
                    let (num, len) = (u16_at(d, at + 4 * k)?, u16_at(d, at + 4 * k + 2)? as usize);
                    spans.push((num, pos, len));
                    pos += len;
                }
            }
            Kind::Unifont => {
                let n = u32_at(d, at)? as usize;
                let info_len = u16_at(d, at + 4)? as usize;
                at += 6;
                spans.push((0, at, info_len));
                at += info_len;
                for _ in 1..n {
                    if at + 4 > d.len() {
                        break;
                    }
                    let (num, len) = (u16_at(d, at)?, u16_at(d, at + 2)? as usize);
                    spans.push((num, at + 4, len));
                    at += 4 + len;
                }
            }
            Kind::Bigfont => {
                let n = u16_at(d, at + 2)? as usize;
                let nr = u16_at(d, at + 4)? as usize;
                at += 6;
                for k in 0..nr {
                    ranges.push((u16_at(d, at + 4 * k)?, u16_at(d, at + 4 * k + 2)?));
                }
                at += 4 * nr;
                for k in 0..n {
                    let e = at + 8 * k;
                    let (num, len, off) = (
                        u16_at(d, e)?,
                        u16_at(d, e + 2)? as usize,
                        u32_at(d, e + 4)? as usize,
                    );
                    if num == 0 && len == 0 && off == 0 {
                        continue;
                    }
                    spans.push((num, off, len));
                }
            }
        }
        let mut shapes = HashMap::with_capacity(spans.len());
        let mut info: Option<(String, Vec<u8>)> = None;
        for (num, start, len) in spans {
            let end = start.checked_add(len).ok_or(ShxError::Truncated)?;
            let def = d.get(start..end).ok_or(ShxError::Truncated)?;
            let name_end = def.iter().position(|&b| b == 0).unwrap_or(def.len());
            let body = (start + (name_end + 1).min(len), end);
            if num == 0 {
                let name = String::from_utf8_lossy(&def[..name_end]).trim().to_string();
                info = Some((name, d[body.0..body.1].to_vec()));
            } else {
                shapes.insert(num, body);
            }
        }
        let (name, bytes) = info.clone().unwrap_or_default();
        let b = |k: usize| bytes.get(k).copied().unwrap_or(0) as f64;
        let mut f = ShxFont {
            kind,
            name,
            above: b(0),
            below: b(1),
            modes: b(2) as u8,
            width: None,
            ranges,
            text: info.is_some() && b(0) > 0.0,
            data,
            shapes,
        };
        if kind == Kind::Bigfont && bytes.len() >= 5 {
            // Extended: character height, 0, modes, character width, 0.
            f.width = Some(b(3)).filter(|&w| w > 0.0);
        }
        Ok(f)
    }

    pub fn has(&self, n: u16) -> bool {
        self.shapes.contains_key(&n)
    }

    /// How many shapes there are (shape 0 aside).
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Whether a byte leads a big font's two-byte character.
    pub fn is_lead(&self, b: u8) -> bool {
        self.ranges
            .iter()
            .any(|&(lo, hi)| (lo.min(255) as u8..=hi.min(255) as u8).contains(&b))
    }

    /// The height that maps to one em: the cap height (a big font's character height).
    pub fn height(&self) -> f64 {
        if self.above > 0.0 { self.above } else { 1.0 }
    }

    /// Shape `n` in em units, laid out horizontally (code 14's next command skipped), or None if
    /// there is no such shape.
    pub fn glyph(&self, n: u16) -> Option<Glyph> {
        self.glyph_in(n, false)
    }

    /// Shape `n` for vertical text (code 14's commands run): the advance is down the page.
    pub fn vertical_glyph(&self, n: u16) -> Option<Glyph> {
        self.glyph_in(n, true)
    }

    fn glyph_in(&self, n: u16, vertical: bool) -> Option<Glyph> {
        let (strokes, end) = self.run(n, vertical)?;
        let h = self.height();
        let s = |p: &[f64; 2]| [p[0] / h, p[1] / h];
        Some(Glyph {
            strokes: strokes.iter().map(|l| l.iter().map(s).collect()).collect(),
            fills: vec![],
            advance: if vertical { -end[1] / h } else { end[0] / h },
        })
    }

    /// Runs shape `n`'s program: its strokes and where the pen ends, in the font's units.
    pub fn run(&self, n: u16, vertical: bool) -> Option<(Strokes, [f64; 2])> {
        self.shapes.get(&n)?;
        let mut pen = Pen {
            font: self,
            vertical,
            tol: 0.002 * self.height(),
            at: [0.0, 0.0],
            down: true,
            scale: 1.0,
            stack: Vec::new(),
            map: Map::IDENTITY,
            strokes: Vec::new(),
            line: Vec::new(),
        };
        pen.shape(n, 0);
        pen.lift();
        Some((pen.strokes, pen.at))
    }
}

/// Local shape units to the glyph's: scale then offset (an extended big font's primitive).
#[derive(Clone, Copy, Debug)]
struct Map {
    s: [f64; 2],
    o: [f64; 2],
}

impl Map {
    const IDENTITY: Map = Map {
        s: [1.0, 1.0],
        o: [0.0, 0.0],
    };
    fn apply(&self, p: [f64; 2]) -> [f64; 2] {
        [p[0] * self.s[0] + self.o[0], p[1] * self.s[1] + self.o[1]]
    }
}

/// The direction vectors of a length-direction byte's low nibble.
const DIRS: [[f64; 2]; 16] = [
    [1.0, 0.0],
    [1.0, 0.5],
    [1.0, 1.0],
    [0.5, 1.0],
    [0.0, 1.0],
    [-0.5, 1.0],
    [-1.0, 1.0],
    [-1.0, 0.5],
    [-1.0, 0.0],
    [-1.0, -0.5],
    [-1.0, -1.0],
    [-0.5, -1.0],
    [0.0, -1.0],
    [0.5, -1.0],
    [1.0, -1.0],
    [1.0, -0.5],
];

const MAX_DEPTH: usize = 8;
const OCTANT: f64 = dmath::PI / 4.0;

struct Pen<'a> {
    font: &'a ShxFont,
    vertical: bool,
    tol: f64,
    /// The pen, in local units.
    at: [f64; 2],
    down: bool,
    /// Codes 3 and 4's factor on lengths.
    scale: f64,
    stack: Vec<[f64; 2]>,
    map: Map,
    strokes: Vec<Vec<[f64; 2]>>,
    /// The polyline being drawn, in glyph units.
    line: Vec<[f64; 2]>,
}

impl Pen<'_> {
    fn lift(&mut self) {
        if self.line.len() >= 2 {
            self.strokes.push(std::mem::take(&mut self.line));
        }
        self.line.clear();
    }

    /// Moves the pen to `q` (local units), drawing if it is down.
    fn to(&mut self, q: [f64; 2]) {
        if self.down {
            if self.line.is_empty() {
                self.line.push(self.map.apply(self.at));
            }
            self.line.push(self.map.apply(q));
        }
        self.at = q;
    }

    /// Jumps to `q` without drawing.
    fn jump(&mut self, q: [f64; 2]) {
        self.lift();
        self.at = q;
    }

    fn by(&mut self, dx: f64, dy: f64) {
        let q = [self.at[0] + dx * self.scale, self.at[1] + dy * self.scale];
        self.to(q);
    }

    /// An arc about `c` (local units) from angle `a0` through `sweep` radians.
    fn arc(&mut self, c: [f64; 2], r: f64, a0: f64, sweep: f64) {
        let mut pts = Vec::new();
        // The map may stretch: cut finely enough for the larger axis.
        let k = self.map.s[0].abs().max(self.map.s[1].abs()).max(1e-9);
        arc_points(c, r, a0, sweep, self.tol / k, &mut pts);
        for p in pts {
            self.to(p);
        }
    }

    fn shape(&mut self, n: u16, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        let Some(&(s, e)) = self.font.shapes.get(&n) else {
            return;
        };
        let def = &self.font.data[s..e];
        let mut i = 0;
        while i < def.len() {
            match self.step(def, i, depth) {
                Some(next) => i = next,
                None => return,
            }
        }
    }

    /// Runs the command at `i`; the next command's index, or None at the end of the shape.
    fn step(&mut self, def: &[u8], i: usize, depth: usize) -> Option<usize> {
        let b = |k: usize| def.get(i + k).copied();
        let sb = |k: usize| def.get(i + k).map(|&v| v as i8 as f64);
        let code = b(0)?;
        if code >= 0x10 {
            let len = (code >> 4) as f64;
            let d = DIRS[(code & 15) as usize];
            self.by(d[0] * len, d[1] * len);
            return Some(i + 1);
        }
        match code {
            0 => None,
            1 => {
                self.down = true;
                Some(i + 1)
            }
            2 => {
                self.lift();
                self.down = false;
                Some(i + 1)
            }
            3 => {
                let f = b(1)? as f64;
                if f > 0.0 {
                    self.scale /= f;
                }
                Some(i + 2)
            }
            4 => {
                self.scale *= b(1)? as f64;
                Some(i + 2)
            }
            5 => {
                self.stack.push(self.at);
                Some(i + 1)
            }
            6 => {
                if let Some(p) = self.stack.pop() {
                    self.jump(p);
                }
                Some(i + 1)
            }
            7 => self.subshape(def, i, depth),
            8 => {
                self.by(sb(1)?, sb(2)?);
                Some(i + 3)
            }
            9 => {
                let mut k = i + 1;
                loop {
                    let (dx, dy) = (*def.get(k)? as i8, *def.get(k + 1)? as i8);
                    k += 2;
                    if dx == 0 && dy == 0 {
                        return Some(k);
                    }
                    self.by(dx as f64, dy as f64);
                }
            }
            10 => {
                let r = b(1)? as f64 * self.scale;
                let o = b(2)?;
                let cw = o & 0x80 != 0;
                let start = ((o >> 4) & 7) as f64;
                let n = match o & 7 {
                    0 => 8,
                    n => n,
                } as f64;
                let a0 = start * OCTANT;
                let sweep = if cw { -n * OCTANT } else { n * OCTANT };
                self.arc_from_here(r, a0, sweep);
                Some(i + 3)
            }
            11 => {
                let (so, eo) = (b(1)? as f64, b(2)? as f64);
                let r = (b(3)? as f64 * 256.0 + b(4)? as f64) * self.scale;
                let o = b(5)?;
                let cw = o & 0x80 != 0;
                let start = ((o >> 4) & 7) as f64;
                let n = match o & 7 {
                    0 => 8,
                    n => n,
                } as f64;
                let sign = if cw { -1.0 } else { 1.0 };
                // Offsets are 256ths of an octant past a boundary, in the direction of travel:
                // the start's past the start octant's boundary, the end's past the boundary
                // where the last octant the arc touches begins (none: it ends on a boundary).
                let a0 = (start + sign * so / 256.0) * OCTANT;
                let a1 = if eo == 0.0 {
                    (start + sign * n) * OCTANT
                } else {
                    (start + sign * (n - 1.0) + sign * eo / 256.0) * OCTANT
                };
                let mut sweep = a1 - a0;
                if cw && sweep > 0.0 {
                    sweep -= dmath::TAU;
                } else if !cw && sweep < 0.0 {
                    sweep += dmath::TAU;
                }
                self.arc_from_here(r, a0, sweep);
                Some(i + 6)
            }
            12 => {
                self.bulge(sb(1)?, sb(2)?, sb(3)?);
                Some(i + 4)
            }
            13 => {
                let mut k = i + 1;
                loop {
                    let (dx, dy) = (*def.get(k)? as i8 as f64, *def.get(k + 1)? as i8 as f64);
                    if dx == 0.0 && dy == 0.0 {
                        return Some(k + 2);
                    }
                    let h = *def.get(k + 2)? as i8 as f64;
                    self.bulge(dx, dy, h);
                    k += 3;
                }
            }
            14 => {
                if self.vertical {
                    Some(i + 1)
                } else {
                    skip(self.font.kind, def, i + 1)
                }
            }
            _ => Some(i + 1),
        }
    }

    /// An arc starting at the pen: its radius, starting angle and sweep.
    fn arc_from_here(&mut self, r: f64, a0: f64, sweep: f64) {
        let (s, c) = dmath::sin_cos(a0);
        let centre = [self.at[0] - r * c, self.at[1] - r * s];
        self.arc(centre, r, a0, sweep);
    }

    /// Codes 12 and 13: to the pen plus (dx, dy), bowed by `bulge` (-127..127, 127 a half
    /// circle; positive anticlockwise). The bulge is twice the arc's height over its chord,
    /// which is the tangent of a quarter of its sweep.
    fn bulge(&mut self, dx: f64, dy: f64, bulge: f64) {
        let (dx, dy) = (dx * self.scale, dy * self.scale);
        let q = [self.at[0] + dx, self.at[1] + dy];
        let chord = (dx * dx + dy * dy).sqrt();
        if bulge == 0.0 || chord == 0.0 {
            self.to(q);
            return;
        }
        let t = bulge / 127.0;
        let sweep = 4.0 * dmath::atan(t);
        // The centre's distance from the chord's middle (left of travel for positive sweep):
        // (chord/2) / tan(sweep/2), and tan(sweep/2) = 2t / (1 - t^2).
        let off = (chord / 2.0) * (1.0 - t * t) / (2.0 * t);
        let m = [self.at[0] + dx / 2.0, self.at[1] + dy / 2.0];
        let n = [-dy / chord, dx / chord];
        let c = [m[0] + n[0] * off, m[1] + n[1] * off];
        let r = ((self.at[0] - c[0]).powi(2) + (self.at[1] - c[1]).powi(2)).sqrt();
        let a0 = dmath::atan2(self.at[1] - c[1], self.at[0] - c[0]);
        self.arc(c, r, a0, sweep);
        // Land exactly on the target.
        self.at = q;
        if let Some(last) = self.line.last_mut() {
            *last = self.map.apply(q);
        }
    }

    fn subshape(&mut self, def: &[u8], i: usize, depth: usize) -> Option<usize> {
        let b = |k: usize| def.get(i + k).copied();
        match self.font.kind {
            Kind::Shapes => {
                self.shape(b(1)? as u16, depth + 1);
                Some(i + 2)
            }
            Kind::Unifont => {
                self.shape(u16::from_be_bytes([b(1)?, b(2)?]), depth + 1);
                Some(i + 3)
            }
            Kind::Bigfont if b(1)? == 0 => {
                // 7, 0, primitive (2 bytes), base x, base y, width, height: the primitive cut
                // down to a unit square (the font's character height) and stretched to width by
                // height, its origin at the base point.
                let n = u16::from_be_bytes([b(2)?, b(3)?]);
                let (x, y) = (b(4)? as i8 as f64, b(5)? as i8 as f64);
                let (w, h) = (b(6)? as f64, b(7)? as f64);
                let unit = self.font.height();
                let parent = self.map;
                let origin = parent.apply([self.at[0] + x * self.scale, self.at[1] + y * self.scale]);
                let k = self.scale / unit;
                let sub = Map {
                    s: [parent.s[0] * w * k, parent.s[1] * h * k],
                    o: origin,
                };
                let saved = (self.at, self.down, self.scale, std::mem::take(&mut self.stack));
                self.lift();
                self.map = sub;
                self.at = [0.0, 0.0];
                self.down = true;
                self.scale = 1.0;
                self.shape(n, depth + 1);
                self.lift();
                self.map = parent;
                (self.at, self.down, self.scale, self.stack) = saved;
                Some(i + 8)
            }
            Kind::Bigfont => {
                self.shape(u16::from_be_bytes([b(1)?, b(2)?]), depth + 1);
                Some(i + 3)
            }
        }
    }
}

/// The index past the command at `i` (for code 14's skip in horizontal text), or None at the
/// end.
fn skip(kind: Kind, def: &[u8], i: usize) -> Option<usize> {
    let code = *def.get(i)?;
    if code >= 0x10 {
        return Some(i + 1);
    }
    Some(match code {
        0 => return None,
        3 | 4 => i + 2,
        7 => match kind {
            Kind::Shapes => i + 2,
            Kind::Unifont => i + 3,
            Kind::Bigfont if *def.get(i + 1)? == 0 => i + 8,
            Kind::Bigfont => i + 3,
        },
        8 => i + 3,
        9 => {
            let mut k = i + 1;
            while (*def.get(k)?, *def.get(k + 1)?) != (0, 0) {
                k += 2;
            }
            k + 2
        }
        10 => i + 3,
        11 => i + 6,
        12 => i + 4,
        13 => {
            let mut k = i + 1;
            while (*def.get(k)?, *def.get(k + 1)?) != (0, 0) {
                k += 3;
            }
            k + 2
        }
        14 => skip(kind, def, i + 1)?,
        _ => i + 1,
    })
}

/// Builds SHX files, for tests: shape numbers and their programs (each without its name, NUL or
/// closing 0).
pub mod build {
    fn def(name: &str, prog: &[u8]) -> Vec<u8> {
        let mut v = name.as_bytes().to_vec();
        v.push(0);
        v.extend_from_slice(prog);
        v.push(0);
        v
    }

    /// An "AutoCAD-86 shapes 1.0" (or 1.1) text font.
    pub fn shapes(version: &str, name: &str, above: u8, below: u8, shapes: &[(u16, &[u8])]) -> Vec<u8> {
        let mut defs = vec![(0u16, def(name, &[above, below, 0]))];
        defs.extend(shapes.iter().map(|(n, p)| (*n, def("", p))));
        let mut out = format!("AutoCAD-86 shapes {version}\r\n\x1a").into_bytes();
        let first = defs.iter().map(|d| d.0).min().unwrap_or(0);
        let last = defs.iter().map(|d| d.0).max().unwrap_or(0);
        out.extend_from_slice(&first.to_le_bytes());
        out.extend_from_slice(&last.to_le_bytes());
        out.extend_from_slice(&(defs.len() as u16).to_le_bytes());
        for (n, d) in &defs {
            out.extend_from_slice(&n.to_le_bytes());
            out.extend_from_slice(&(d.len() as u16).to_le_bytes());
        }
        for (_, d) in &defs {
            out.extend_from_slice(d);
        }
        out
    }

    /// An "AutoCAD-86 unifont 1.0" font.
    pub fn unifont(name: &str, above: u8, below: u8, shapes: &[(u16, &[u8])]) -> Vec<u8> {
        let mut out = b"AutoCAD-86 unifont 1.0\r\n\x1a".to_vec();
        out.extend_from_slice(&(shapes.len() as u32 + 1).to_le_bytes());
        let mut info = name.as_bytes().to_vec();
        info.extend_from_slice(&[0, above, below, 0, 0, 0, 0]);
        out.extend_from_slice(&(info.len() as u16).to_le_bytes());
        out.extend_from_slice(&info);
        for (n, p) in shapes {
            let d = def("", p);
            out.extend_from_slice(&n.to_le_bytes());
            out.extend_from_slice(&(d.len() as u16).to_le_bytes());
            out.extend_from_slice(&d);
        }
        out
    }

    /// An "AutoCAD-86 bigfont 1.0" font: `info` is shape 0's bytes after the name (height, 0,
    /// modes, 0; or for an extended big font height, 0, modes, width, 0).
    pub fn bigfont(name: &str, info: &[u8], ranges: &[(u16, u16)], shapes: &[(u16, &[u8])]) -> Vec<u8> {
        let mut defs = vec![(0u16, {
            let mut v = name.as_bytes().to_vec();
            v.push(0);
            v.extend_from_slice(info);
            v
        })];
        defs.extend(shapes.iter().map(|(n, p)| (*n, def("", p))));
        let mut out = b"AutoCAD-86 bigfont 1.0\r\n\x1a".to_vec();
        out.extend_from_slice(&8u16.to_le_bytes());
        out.extend_from_slice(&(defs.len() as u16).to_le_bytes());
        out.extend_from_slice(&(ranges.len() as u16).to_le_bytes());
        for (a, b) in ranges {
            out.extend_from_slice(&a.to_le_bytes());
            out.extend_from_slice(&b.to_le_bytes());
        }
        let mut off = out.len() + 8 * defs.len();
        for (n, d) in &defs {
            out.extend_from_slice(&n.to_le_bytes());
            out.extend_from_slice(&(d.len() as u16).to_le_bytes());
            out.extend_from_slice(&(off as u32).to_le_bytes());
            off += d.len();
        }
        for (_, d) in &defs {
            out.extend_from_slice(d);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::build;
    use super::*;

    fn close(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
    }

    fn run(f: &ShxFont, n: u16) -> (Strokes, [f64; 2]) {
        f.run(n, false).unwrap()
    }

    /// The largest distance of any point of the strokes from `c`, and the smallest.
    fn radii(strokes: &[Vec<[f64; 2]>], c: [f64; 2]) -> (f64, f64) {
        let d: Vec<f64> = strokes
            .iter()
            .flatten()
            .map(|p| ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2)).sqrt())
            .collect();
        (
            d.iter().cloned().fold(0.0, f64::max),
            d.iter().cloned().fold(f64::INFINITY, f64::min),
        )
    }

    #[test]
    fn vectors_pen_scale_and_stack() {
        // An "H" on a 6-unit cap: up 6, pen up, down 3, pen down, right 4, push, up 3, pop ...
        let h: &[u8] = &[
            0x64, // up 6
            2, 0x3C, // pen up, down 3
            1, 0x40, // pen down, right 4
            5,    // push (4, 3)
            0x34, // up 3
            6,    // pop: back to (4, 3)
            0x3C, // down 3
            2, 0x20, // pen up, right 2 (to 6, 0)
        ];
        let f = ShxFont::parse(build::shapes("1.0", "test", 6, 2, &[(72, h)])).unwrap();
        assert_eq!(f.kind, Kind::Shapes);
        assert_eq!((f.name.as_str(), f.above, f.below), ("test", 6.0, 2.0));
        let (s, end) = run(&f, 72);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0], vec![[0.0, 0.0], [0.0, 6.0]]);
        assert_eq!(s[1], vec![[0.0, 3.0], [4.0, 3.0], [4.0, 6.0]]);
        assert_eq!(s[2], vec![[4.0, 3.0], [4.0, 0.0]]);
        assert_eq!(end, [6.0, 0.0]);
        let g = f.glyph(72).unwrap();
        assert_eq!(g.advance, 1.0);
        assert_eq!(g.strokes[0][1], [0.0, 1.0]);

        // Scaling down by 2 then up by 4, and every direction.
        let dirs: Vec<u8> = (0..16).map(|d| 0x20 | d).collect();
        let f = ShxFont::parse(build::shapes(
            "1.1",
            "d",
            4,
            0,
            &[(65, &[3, 2, 0x20, 4, 4, 0x10, 1])],
        ))
        .unwrap();
        // Scale 1/2: 2 units right is 1; then x4 (scale 2): 1 unit right is 2.
        let (s, end) = run(&f, 65);
        assert_eq!(s, vec![vec![[0.0, 0.0], [1.0, 0.0], [3.0, 0.0]]]);
        assert_eq!(end, [3.0, 0.0]);
        let shapes: Vec<(u16, Vec<u8>)> = dirs.iter().map(|&d| (d as u16, vec![d])).collect();
        let refs: Vec<(u16, &[u8])> = shapes.iter().map(|(n, p)| (*n, &p[..])).collect();
        let f = ShxFont::parse(build::shapes("1.0", "d", 4, 0, &refs)).unwrap();
        for (k, &d) in dirs.iter().enumerate() {
            let (_, end) = run(&f, d as u16);
            let v = DIRS[k];
            assert_eq!(end, [2.0 * v[0], 2.0 * v[1]], "direction {k}");
        }
    }

    #[test]
    fn displacements() {
        // 8: one (dx, dy); 9: several until (0, 0).
        let p: &[u8] = &[
            8,
            3,
            (-2i8) as u8,
            9,
            1,
            1,
            2,
            0,
            (-1i8) as u8,
            4,
            0,
            0,
            2,
            8,
            5,
            0,
        ];
        let f = ShxFont::parse(build::shapes("1.0", "t", 4, 0, &[(1, p)])).unwrap();
        let (s, end) = run(&f, 1);
        assert_eq!(
            s,
            vec![vec![
                [0.0, 0.0],
                [3.0, -2.0],
                [4.0, -1.0],
                [6.0, -1.0],
                [5.0, 3.0]
            ]]
        );
        assert_eq!(end, [10.0, 3.0]);
    }

    #[test]
    fn octant_arcs() {
        // A full circle of radius 3 from octant 0, anticlockwise: it starts and ends at the pen,
        // centred 3 to the left.
        let f = ShxFont::parse(build::shapes(
            "1.0",
            "t",
            6,
            0,
            &[(1, &[10, 3, 0x00]), (2, &[10, 3, 0x22]), (3, &[10, 4, 0xA3])],
        ))
        .unwrap();
        let (s, end) = run(&f, 1);
        assert!(close(end, [0.0, 0.0]));
        let (hi, lo) = radii(&s, [-3.0, 0.0]);
        assert!((hi - 3.0).abs() < 1e-9 && (lo - 3.0).abs() < 1e-9);
        assert!(s[0].len() > 16);
        // Octant 2 (90 degrees) for 2 octants anticlockwise: from the top of a circle about
        // (0, -3) round to its left: ends at (-3, -3).
        let (s, end) = run(&f, 2);
        assert!(close(end, [-3.0, -3.0]), "{end:?}");
        let (hi, lo) = radii(&s, [0.0, -3.0]);
        assert!((hi - 3.0).abs() < 1e-9 && (lo - 3.0).abs() < 1e-9);
        // Clockwise (sign bit), from octant 2 for 3 octants, radius 4: centre (0, -4), ends at
        // 90 - 135 = -45 degrees.
        let (_, end) = run(&f, 3);
        let h = 4.0 * std::f64::consts::FRAC_1_SQRT_2;
        assert!(close(end, [h, -4.0 - h]), "{end:?}");
    }

    #[test]
    fn fractional_arcs() {
        // The guide's example: radius 3 from 55.98 to 99.47 degrees: start offset 62, end
        // offset 54, start octant 1, two octants.
        let f = ShxFont::parse(build::shapes(
            "1.0",
            "t",
            6,
            0,
            &[
                (1, &[11, 62, 54, 0, 3, 0x12]),
                (2, &[11, 0, 0, 1, 0, 0x02]),
                (3, &[11, 128, 0, 0, 2, 0x81]),
            ],
        ))
        .unwrap();
        let (s, end) = run(&f, 1);
        let a0 = (45.0 + 62.0 * 45.0 / 256.0f64).to_radians();
        let a1 = (90.0 + 54.0 * 45.0 / 256.0f64).to_radians();
        let c = [-3.0 * dmath::cos(a0), -3.0 * dmath::sin(a0)];
        let want = [c[0] + 3.0 * dmath::cos(a1), c[1] + 3.0 * dmath::sin(a1)];
        assert!(close(end, want), "{end:?} {want:?}");
        let (hi, lo) = radii(&s, c);
        assert!((hi - 3.0).abs() < 1e-9 && (lo - 3.0).abs() < 1e-9);
        // Radius 256 (high byte 1), from 0 for two octants ending on the boundary: a quarter.
        let (_, end) = run(&f, 2);
        assert!(close(end, [-256.0, 256.0]), "{end:?}");
        // Clockwise from octant 0 plus half an octant (so -22.5 degrees) to the boundary one
        // octant on (-45 degrees), radius 2.
        let (_, end) = run(&f, 3);
        let a0 = (-22.5f64).to_radians();
        let a1 = (-45.0f64).to_radians();
        let c = [-2.0 * dmath::cos(a0), -2.0 * dmath::sin(a0)];
        assert!(
            close(end, [c[0] + 2.0 * dmath::cos(a1), c[1] + 2.0 * dmath::sin(a1)]),
            "{end:?}"
        );
    }

    #[test]
    fn bulge_arcs() {
        // 12: to (4, 0) with bulge 127: a half circle bowing below (anticlockwise), centre
        // (2, 0). Bulge -127: above. Bulge 0: a straight line.
        let f = ShxFont::parse(build::shapes(
            "1.0",
            "t",
            6,
            0,
            &[
                (1, &[12, 4, 0, 127]),
                (2, &[12, 4, 0, (-127i8) as u8]),
                (3, &[12, 4, 0, 0]),
                // 13: two arcs then a straight piece, ended by (0, 0).
                (4, &[13, 4, 0, 127, 0, 4, 64, 2, 0, 0, 0, 0]),
            ],
        ))
        .unwrap();
        let (s, end) = run(&f, 1);
        assert_eq!(end, [4.0, 0.0]);
        let (hi, lo) = radii(&s, [2.0, 0.0]);
        assert!((hi - 2.0).abs() < 1e-9 && (lo - 2.0).abs() < 1e-9);
        assert!(s[0].iter().all(|p| p[1] <= 1e-9));
        let (s, _) = run(&f, 2);
        assert!(s[0].iter().all(|p| p[1] >= -1e-9));
        assert!(s[0].iter().map(|p| p[1]).fold(0.0, f64::max) > 1.98);
        let (s, _) = run(&f, 3);
        assert_eq!(s, vec![vec![[0.0, 0.0], [4.0, 0.0]]]);
        let (s, end) = run(&f, 4);
        assert_eq!(end, [6.0, 4.0]);
        // Bulge 64 is about a quarter-height arc: tan(sweep/4) = 64/127.
        let sweep = 4.0 * dmath::atan(64.0 / 127.0);
        let r = 2.0 / dmath::sin(sweep / 2.0);
        let pts = &s[0];
        let mid = pts
            .iter()
            .find(|p| p[1] > 0.0 && p[1] < 4.0 && (p[1] - 2.0).abs() < 0.2);
        assert!(mid.is_some());
        assert!(r > 2.0);
        assert!(close(*pts.last().unwrap(), [6.0, 4.0]));
    }

    #[test]
    fn subshapes() {
        // Shapes 1.0: 7 then a one-byte number; the subshape draws inline and leaves the pen.
        let f = ShxFont::parse(build::shapes(
            "1.0",
            "t",
            4,
            0,
            &[(10, &[0x14]), (65, &[7, 10, 0x10, 7, 10])],
        ))
        .unwrap();
        let (s, end) = run(&f, 65);
        assert_eq!(s, vec![vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 2.0]]]);
        assert_eq!(end, [1.0, 2.0]);
        // Unifont: two bytes, high first.
        let f = ShxFont::parse(build::unifont(
            "u",
            4,
            1,
            &[
                (0x0130, &[0x24]),
                (0x00c4, &[7, 0x01, 0x30, 0x20]),
                (0x4e2d, &[0x40]),
            ],
        ))
        .unwrap();
        assert_eq!(f.kind, Kind::Unifont);
        assert_eq!((f.above, f.below), (4.0, 1.0));
        let (s, end) = run(&f, 0xc4);
        assert_eq!(s, vec![vec![[0.0, 0.0], [0.0, 2.0], [2.0, 2.0]]]);
        assert_eq!(end, [2.0, 2.0]);
        assert_eq!(f.glyph(0x4e2d).unwrap().advance, 1.0);
        // A subshape that calls itself stops at the depth limit.
        let f = ShxFont::parse(build::shapes("1.0", "t", 4, 0, &[(1, &[0x10, 7, 1])])).unwrap();
        let (_, end) = run(&f, 1);
        assert_eq!(end, [(MAX_DEPTH + 1) as f64, 0.0]);
    }

    #[test]
    fn bigfont_ranges_and_extended_subshapes() {
        // A big font with lead bytes 0x81..0x9F and 0xE0..0xFC, height 8; shape 0x8140 draws a
        // primitive (0x0101: right 8, up 8) at base (2, 1) squeezed to 4 wide by
        // 8 high, then another plain.
        let prim: &[u8] = &[0x80, 0x84]; // right 8, up 8
        let char_def: &[u8] = &[7, 0, 0x01, 0x01, 2, 1, 4, 8, 2, 0x80, 1];
        let f = ShxFont::parse(build::bigfont(
            "big",
            &[8, 0, 0, 0],
            &[(0x81, 0x9f), (0xe0, 0xfc)],
            &[(0x0101, prim), (0x8140, char_def), (0x8141, &[7, 0x81, 0x40])],
        ))
        .unwrap();
        assert_eq!(f.kind, Kind::Bigfont);
        assert_eq!(f.ranges, vec![(0x81, 0x9f), (0xe0, 0xfc)]);
        assert!(f.is_lead(0x81) && f.is_lead(0xfc) && !f.is_lead(0x41) && !f.is_lead(0xa0));
        assert_eq!(f.height(), 8.0);
        let (s, end) = run(&f, 0x8140);
        // Primitive scaled by 4/8 across and 8/8 up, from (2, 1).
        assert_eq!(s, vec![vec![[2.0, 1.0], [6.0, 1.0], [6.0, 9.0]]]);
        // The parent's pen did not move for the primitive; then pen up, right 8.
        assert_eq!(end, [8.0, 0.0]);
        // A plain two-byte subshape.
        let (s2, _) = run(&f, 0x8141);
        assert_eq!(s2, s);
        // Extended header: height 0 modes width 0.
        let f = ShxFont::parse(build::bigfont(
            "x",
            &[16, 0, 0, 16, 0],
            &[(0x81, 0x84)],
            &[(0x8140, &[0x10])],
        ))
        .unwrap();
        assert_eq!((f.height(), f.width), (16.0, Some(16.0)));
    }

    #[test]
    fn vertical_only_commands() {
        // 14 then a command: skipped for horizontal text, run for vertical. Each kind of
        // command is skipped whole.
        let p: &[u8] = &[
            14, 8, 5, 5, // xy
            14, 9, 1, 1, 2, 2, 0, 0, // xys
            14, 10, 2, 0x01, // octant arc
            14, 11, 1, 2, 0, 3, 0x12, // fractional arc
            14, 12, 3, 3, 10, // bulge
            14, 13, 2, 2, 5, 1, 1, 0, 0, 0, // bulges
            14, 3, 2, // scale
            14, 7, 2, // subshape
            14, 0x4C, // vector
            0x10, // right 1
        ];
        let f = ShxFont::parse(build::shapes("1.0", "t", 4, 0, &[(1, p), (2, &[0x10])])).unwrap();
        let (s, end) = f.run(1, false).unwrap();
        assert_eq!(end, [1.0, 0.0]);
        assert_eq!(s, vec![vec![[0.0, 0.0], [1.0, 0.0]]]);
        let (_, vend) = f.run(1, true).unwrap();
        assert_ne!(vend, end);
        let v = f.vertical_glyph(1).unwrap();
        assert!(!v.strokes.is_empty());
    }

    #[test]
    fn rejects_what_is_not_shx() {
        assert_eq!(ShxFont::parse(b"hello".to_vec()).unwrap_err(), ShxError::NotShx);
        assert!(matches!(
            ShxFont::parse(b"AutoCAD-86 widgets 1.0\r\n\x1a".to_vec()),
            Err(ShxError::Unknown(_))
        ));
        let mut t = build::shapes("1.0", "t", 4, 0, &[(1, &[0x10])]);
        t.truncate(t.len() - 3);
        assert_eq!(ShxFont::parse(t).unwrap_err(), ShxError::Truncated);
    }
}

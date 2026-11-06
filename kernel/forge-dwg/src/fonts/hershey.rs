//! The built-in stand-ins for Autodesk's SHX fonts: Dr. A. V. Hershey's stroke fonts, which the
//! SHX text fonts (romans, romand, romanc, ..) were themselves drawn after, so the stand-ins have
//! their shapes and spacing.
//!
//! The glyph data is the Usenet distribution of the Hershey fonts (James Hurt's format), packed
//! by `examples/dwgfonts.rs pack` into `hershey.bin`; see `HERSHEY-NOTICE.txt` beside this file
//! for where it comes from and the acknowledgements its terms ask for. The distribution has the
//! printable ASCII characters of each face; the rest (Latin-1 and Central European accents, the
//! degree, plus-minus and diameter signs, superscripts, fractions, ..) are put together here from
//! those glyphs and a few strokes of our own, and Greek, Cyrillic and mathematical characters come
//! from the Greek, Cyrillic and math faces.
//!
//! Hershey coordinates: y down, the baseline at 9, capitals 21 units tall.

use super::glyph::{self, Glyph};
use std::sync::OnceLock;

/// A built-in face.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Face {
    Simplex,
    Duplex,
    Complex,
    ItalicComplex,
    Triplex,
    ItalicTriplex,
    ScriptSimplex,
    ScriptComplex,
    GothicEnglish,
    GothicGerman,
    GothicItalian,
    GreekSimplex,
    GreekComplex,
    Cyrillic,
    MathLower,
    MathUpper,
    Astrology,
    Meteorology,
    Music,
    Symbolic,
}

pub struct FaceInfo {
    pub face: Face,
    /// The distribution's file (without `.jhf`).
    pub file: &'static str,
    pub title: &'static str,
}

const fn fi(face: Face, file: &'static str, title: &'static str) -> FaceInfo {
    FaceInfo { face, file, title }
}

/// Every face, in `Face` order (the order of `hershey.bin`).
pub const FACES: [FaceInfo; 20] = [
    fi(Face::Simplex, "rowmans", "Hershey Roman Simplex"),
    fi(Face::Duplex, "rowmand", "Hershey Roman Duplex"),
    fi(Face::Complex, "timesr", "Hershey Roman Complex"),
    fi(Face::ItalicComplex, "timesi", "Hershey Italic Complex"),
    fi(Face::Triplex, "rowmant", "Hershey Roman Triplex"),
    fi(Face::ItalicTriplex, "timesib", "Hershey Italic Triplex"),
    fi(Face::ScriptSimplex, "scripts", "Hershey Script Simplex"),
    fi(Face::ScriptComplex, "scriptc", "Hershey Script Complex"),
    fi(Face::GothicEnglish, "gothiceng", "Hershey Gothic English"),
    fi(Face::GothicGerman, "gothicger", "Hershey Gothic German"),
    fi(Face::GothicItalian, "gothicita", "Hershey Gothic Italian"),
    fi(Face::GreekSimplex, "greeks", "Hershey Greek Simplex"),
    fi(Face::GreekComplex, "greekc", "Hershey Greek Complex"),
    fi(Face::Cyrillic, "cyrillic", "Hershey Cyrillic Complex"),
    fi(Face::MathLower, "mathlow", "Hershey Math (lower case)"),
    fi(Face::MathUpper, "mathupp", "Hershey Math (upper case)"),
    fi(Face::Astrology, "astrology", "Hershey Astrology"),
    fi(Face::Meteorology, "meteorology", "Hershey Meteorology"),
    fi(Face::Music, "music", "Hershey Music"),
    fi(Face::Symbolic, "symbolic", "Hershey Symbolic"),
];

impl Face {
    pub fn title(self) -> &'static str {
        FACES[self as usize].title
    }
    fn italic(self) -> bool {
        matches!(
            self,
            Face::ItalicComplex | Face::ItalicTriplex | Face::ScriptSimplex | Face::ScriptComplex
        )
    }
    /// The Greek face that goes with this one.
    fn greek(self) -> Face {
        match self {
            Face::Simplex | Face::ScriptSimplex | Face::GreekSimplex | Face::MathLower | Face::MathUpper => {
                Face::GreekSimplex
            }
            _ => Face::GreekComplex,
        }
    }
    /// The face Latin characters come from when this one has none (the symbol faces' letters
    /// are symbols).
    fn latin(self) -> Face {
        match self {
            Face::GreekSimplex | Face::MathLower | Face::MathUpper | Face::Symbolic => Face::Simplex,
            Face::GreekComplex | Face::Cyrillic | Face::Astrology | Face::Meteorology | Face::Music => {
                Face::Complex
            }
            f => f,
        }
    }
}

/// How the stand-in spaces its characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Spacing {
    /// The face's own advances.
    Normal,
    /// txt.shx's: the Simplex glyphs a little more widely set (txt's pitch is about one cap
    /// height).
    Txt,
    /// monotxt.shx's: every character one cap height wide.
    Mono,
}

/// The SHX font names with a stand-in, and the face each gets. Others are matched by prefix in
/// `for_shx` (iso*, gothic*, greek*, ..).
pub const SHX_NAMES: &[(&str, Face)] = &[
    ("txt", Face::Simplex),
    ("monotxt", Face::Simplex),
    ("simplex", Face::Simplex),
    ("romans", Face::Simplex),
    ("isocp", Face::Simplex),
    ("isocp2", Face::Simplex),
    ("isocp3", Face::Simplex),
    ("isoct", Face::Simplex),
    ("isoct2", Face::Simplex),
    ("isoct3", Face::Simplex),
    ("isocpeur", Face::Simplex),
    ("isocteur", Face::Simplex),
    ("romand", Face::Duplex),
    ("complex", Face::Complex),
    ("romanc", Face::Complex),
    ("italic", Face::ItalicComplex),
    ("italicc", Face::ItalicComplex),
    ("romant", Face::Triplex),
    ("italict", Face::ItalicTriplex),
    ("scripts", Face::ScriptSimplex),
    ("scriptc", Face::ScriptComplex),
    ("gothice", Face::GothicEnglish),
    ("gothicg", Face::GothicGerman),
    ("gothici", Face::GothicItalian),
    ("greeks", Face::GreekSimplex),
    ("greekc", Face::GreekComplex),
    ("cyrillic", Face::Cyrillic),
    ("cyriltlc", Face::Cyrillic),
    ("syastro", Face::Astrology),
    ("symath", Face::MathLower),
    ("symeteo", Face::Meteorology),
    ("symusic", Face::Music),
    ("symap", Face::Symbolic),
];

/// The stand-in for an SHX font's name (lower case, no path or extension), if it is one of
/// Autodesk's.
pub fn for_shx(stem: &str) -> Option<(Face, Spacing)> {
    let spacing = match stem {
        "txt" => Spacing::Txt,
        "monotxt" => Spacing::Mono,
        _ => Spacing::Normal,
    };
    if let Some(&(_, f)) = SHX_NAMES.iter().find(|(n, _)| *n == stem) {
        return Some((f, spacing));
    }
    let prefixed = [
        ("iso", Face::Simplex),
        ("gothic", Face::GothicEnglish),
        ("greekc", Face::GreekComplex),
        ("greek", Face::GreekSimplex),
        ("cyril", Face::Cyrillic),
        ("italict", Face::ItalicTriplex),
        ("italic", Face::ItalicComplex),
        ("scriptc", Face::ScriptComplex),
        ("script", Face::ScriptSimplex),
        ("romanc", Face::Complex),
        ("romand", Face::Duplex),
        ("romant", Face::Triplex),
        ("roman", Face::Simplex),
        ("simplex", Face::Simplex),
        ("complex", Face::Complex),
    ];
    prefixed
        .iter()
        .find(|(p, _)| stem.starts_with(p))
        .map(|&(_, f)| (f, Spacing::Normal))
}

const DATA: &[u8] = include_bytes!("hershey.bin");

/// A glyph as the distribution has it: bearings and strokes in Hershey units.
struct Raw {
    left: i8,
    right: i8,
    strokes: Vec<Vec<[i8; 2]>>,
}

fn faces() -> &'static [Vec<Raw>] {
    static FACES_DATA: OnceLock<Vec<Vec<Raw>>> = OnceLock::new();
    FACES_DATA.get_or_init(|| unpack(DATA).expect("hershey.bin holds together"))
}

fn unpack(d: &[u8]) -> Option<Vec<Vec<Raw>>> {
    if d.get(..4)? != b"HSY1" {
        return None;
    }
    let n = *d.get(4)? as usize;
    let mut at = 5;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        at += 1 + *d.get(at)? as usize;
        let mut glyphs = Vec::with_capacity(95);
        for _ in 0..95 {
            let (l, r) = (*d.get(at)?, *d.get(at + 1)?);
            let len = u16::from_le_bytes([*d.get(at + 2)?, *d.get(at + 3)?]) as usize;
            let pts = d.get(at + 4..at + 4 + len)?;
            at += 4 + len;
            let mut strokes: Vec<Vec<[i8; 2]>> = vec![vec![]];
            for p in pts.as_chunks::<2>().0 {
                if p == b" R" {
                    strokes.push(vec![]);
                } else {
                    let last = strokes.last_mut()?;
                    last.push([(p[0] as i16 - 82) as i8, (p[1] as i16 - 82) as i8]);
                }
            }
            strokes.retain(|s| !s.is_empty());
            glyphs.push(Raw {
                left: (l as i16 - 82) as i8,
                right: (r as i16 - 82) as i8,
                strokes,
            });
        }
        out.push(glyphs);
    }
    Some(out)
}

const UNIT: f64 = 1.0 / 21.0;

/// An ASCII character of a face, in em units.
fn ascii(face: Face, c: char) -> Option<Glyph> {
    let k = (c as u32).checked_sub(32)? as usize;
    let r = faces().get(face as usize)?.get(k)?;
    let x0 = r.left as f64;
    let mut strokes: Vec<Vec<[f64; 2]>> = Vec::with_capacity(r.strokes.len());
    for s in &r.strokes {
        let mut v: Vec<[f64; 2]> = s
            .iter()
            .map(|p| [(p[0] as f64 - x0) * UNIT, (9.0 - p[1] as f64) * UNIT])
            .collect();
        if v.len() == 1 {
            // A lone point: a dot.
            let p = v[0];
            v.push([p[0] + 1e-3, p[1]]);
        }
        strokes.push(v);
    }
    Some(Glyph {
        strokes,
        fills: vec![],
        advance: (r.right as f64 - x0) * UNIT,
    })
}

/// A character in a stand-in face, or None if there is nothing to draw it with.
pub fn glyph(face: Face, spacing: Spacing, c: char) -> Option<Glyph> {
    let mut g = base(face, c)?;
    match spacing {
        Spacing::Normal => {}
        Spacing::Txt => {
            g = g.transformed([1.0, 1.0], [1.5 * UNIT, 0.0]);
            g.advance += 1.5 * UNIT;
        }
        Spacing::Mono => {
            g = g.transformed([1.0, 1.0], [(1.0 - g.advance) / 2.0, 0.0]);
            g.advance = 1.0;
        }
    }
    Some(g)
}

fn base(face: Face, c: char) -> Option<Glyph> {
    if (' '..='~').contains(&c) {
        let own = matches!(
            face,
            Face::GreekSimplex
                | Face::GreekComplex
                | Face::Cyrillic
                | Face::MathLower
                | Face::MathUpper
                | Face::Astrology
                | Face::Meteorology
                | Face::Music
                | Face::Symbolic
        );
        // The symbol faces set ASCII in their own symbols: that is what the SHX font does too.
        return ascii(if own { face } else { face.latin() }, c);
    }
    let latin = face.latin();
    let lat = |c: char| ascii(latin, c);
    if let Some(a) = greek_ascii(c) {
        return ascii(face.greek(), a);
    }
    if let Some(a) = cyrillic_ascii(c) {
        return ascii(Face::Cyrillic, a);
    }
    if let Some(a) = math_ascii(c) {
        return ascii(Face::MathLower, a);
    }
    if let Some((b, acc)) = decompose(c) {
        let base = match b {
            'ı' => dotless_i(latin)?,
            'Е' | 'е' => ascii(Face::Cyrillic, cyrillic_ascii(b)?)?,
            _ => lat(b)?,
        };
        return Some(accented(base, acc, face.italic()));
    }
    let tol = 0.004;
    Some(match c {
        '\u{a0}' | '\u{2007}' | '\u{2009}' | '\u{202f}' => lat(' ')?,
        '\u{ad}' | '\u{2010}' | '\u{2011}' | '\u{2212}' | '\u{2013}' => lat('-')?,
        '\u{2014}' | '\u{2015}' => {
            let h = lat('-')?;
            let mut g = h.transformed([2.0, 1.0], [0.0, 0.0]);
            g.advance = h.advance * 2.0;
            g
        }
        '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{2032}' | '\u{b4}' => lat('\'')?,
        '\u{201c}' | '\u{201d}' | '\u{201e}' | '\u{2033}' => lat('"')?,
        '\u{2026}' => {
            let d = lat('.')?;
            let mut g = Glyph::default();
            for k in 0..3 {
                g.add(&d, [1.0, 1.0], [k as f64 * d.advance, 0.0]);
            }
            g.advance = 3.0 * d.advance;
            g
        }
        '\u{2300}' | '\u{2205}' => {
            // The diameter sign (%%c): a circle with a stroke through it.
            let c0 = [0.5, 0.45];
            let r = 0.36;
            Glyph {
                strokes: vec![
                    glyph::circle(c0, r, tol),
                    vec![[c0[0] - 0.42, c0[1] - 0.52], [c0[0] + 0.42, c0[1] + 0.52]],
                ],
                fills: vec![],
                advance: 1.0,
            }
        }
        '\u{b7}' | '\u{22c5}' | '\u{2022}' | '\u{2219}' => {
            let r = if c == '\u{2022}' { 0.09 } else { 0.035 };
            Glyph {
                strokes: vec![glyph::circle([0.25, 0.42], r, tol / 4.0)],
                fills: vec![],
                advance: 0.5,
            }
        }
        'Ø' | 'ø' => {
            let b = lat(if c == 'Ø' { 'O' } else { 'o' })?;
            slashed(b, 0.06)
        }
        'Ł' | 'ł' => {
            let b = lat(if c == 'Ł' { 'L' } else { 'l' })?;
            let (lo, _) = b.bbox()?;
            let x = if c == 'Ł' { lo[0] } else { b.advance / 2.0 - 0.02 };
            let mut g = b.clone();
            g.strokes.push(vec![[x - 0.12, 0.38], [x + 0.18, 0.62]]);
            g
        }
        'Ð' | 'Đ' => {
            let b = lat('D')?;
            let (lo, _) = b.bbox()?;
            let mut g = b.clone();
            g.strokes.push(vec![[lo[0] - 0.1, 0.5], [lo[0] + 0.3, 0.5]]);
            g
        }
        'đ' => {
            let b = lat('d')?;
            let (_, hi) = b.bbox()?;
            let mut g = b.clone();
            g.strokes.push(vec![[hi[0] - 0.25, 0.86], [hi[0] + 0.1, 0.86]]);
            g
        }
        'ß' => ascii(face.greek(), 'b')?,
        '\u{b5}' => ascii(face.greek(), 'l')?,
        '¿' | '¡' => {
            let b = lat(if c == '¿' { '?' } else { '!' })?;
            // Turned about the middle of the cap height, hanging below the baseline.
            b.transformed([-1.0, -1.0], [b.advance, 1.0 - 0.3])
        }
        '¹' | '²' | '³' => superscript(lat(match c {
            '¹' => '1',
            '²' => '2',
            _ => '3',
        })?),
        'ª' | 'º' => {
            let mut g = superscript(lat(if c == 'ª' { 'a' } else { 'o' })?);
            g.strokes.push(vec![[0.05, 0.36], [g.advance - 0.05, 0.36]]);
            g
        }
        '¼' | '½' | '¾' => {
            let (n, d) = match c {
                '¼' => ('1', '4'),
                '½' => ('1', '2'),
                _ => ('3', '4'),
            };
            let (n, d) = (lat(n)?, lat(d)?);
            let s = 0.55;
            let mut g = Glyph::default();
            g.add(&n, [s, s], [0.0, 0.45]);
            g.add(&d, [s, s], [n.advance * s + 0.12, 0.0]);
            g.strokes
                .push(vec![[n.advance * s - 0.1, 0.0], [n.advance * s + 0.25, 1.0]]);
            g.advance = (n.advance + d.advance) * s + 0.12;
            g
        }
        '«' | '»' | '‹' | '›' => {
            let one = |x: f64, left: bool| {
                if left {
                    vec![[x + 0.22, 0.6], [x, 0.38], [x + 0.22, 0.16]]
                } else {
                    vec![[x, 0.6], [x + 0.22, 0.38], [x, 0.16]]
                }
            };
            let left = matches!(c, '«' | '‹');
            let mut strokes = vec![one(0.15, left)];
            if matches!(c, '«' | '»') {
                strokes.push(one(0.37, left));
            }
            let advance = if strokes.len() == 2 { 0.75 } else { 0.55 };
            Glyph {
                strokes,
                fills: vec![],
                advance,
            }
        }
        '¬' => Glyph {
            strokes: vec![vec![[0.15, 0.5], [0.75, 0.5], [0.75, 0.25]]],
            fills: vec![],
            advance: 0.9,
        },
        '¦' => Glyph {
            strokes: vec![vec![[0.2, 1.1], [0.2, 0.55]], vec![[0.2, 0.3], [0.2, -0.25]]],
            fills: vec![],
            advance: 0.4,
        },
        '¯' => Glyph {
            strokes: vec![vec![[0.1, 1.15], [0.6, 1.15]]],
            fills: vec![],
            advance: 0.7,
        },
        '¨' | '¸' => {
            let acc = if c == '¨' {
                Accent::Diaeresis
            } else {
                Accent::Cedilla
            };
            let space = Glyph {
                advance: 0.5,
                ..Glyph::default()
            };
            let mut g = accented(space, acc, false);
            if c == '¨' {
                // Standing alone, at the height it has over a capital.
                g = g.transformed([1.0, 1.0], [0.0, 1.0]);
                g.advance = 0.5;
            }
            g
        }
        '©' | '®' => {
            let inner = lat(if c == '©' { 'C' } else { 'R' })?;
            let s = 0.5;
            let mut g = Glyph {
                strokes: vec![glyph::circle([0.6, 0.5], 0.58, tol)],
                fills: vec![],
                advance: 1.2,
            };
            let w = inner.advance * s;
            g.add(&inner, [s, s], [0.6 - w / 2.0, 0.25]);
            g
        }
        '¢' => {
            let mut g = lat('c')?;
            let x = g.advance / 2.0;
            g.strokes.push(vec![[x, -0.15], [x, 0.85]]);
            g
        }
        '¥' => {
            let mut g = lat('Y')?;
            let x = g.advance / 2.0;
            g.strokes.push(vec![[x - 0.3, 0.42], [x + 0.3, 0.42]]);
            g.strokes.push(vec![[x - 0.3, 0.24], [x + 0.3, 0.24]]);
            g
        }
        '\u{20ac}' => {
            let mut g = lat('C')?;
            let (lo, _) = g.bbox()?;
            g.strokes.push(vec![[lo[0] - 0.12, 0.6], [lo[0] + 0.45, 0.6]]);
            g.strokes.push(vec![[lo[0] - 0.12, 0.4], [lo[0] + 0.45, 0.4]]);
            g
        }
        '£' => Glyph {
            strokes: vec![
                vec![
                    [0.7, 0.86],
                    [0.64, 0.96],
                    [0.52, 1.0],
                    [0.42, 0.98],
                    [0.34, 0.9],
                    [0.32, 0.78],
                    [0.32, 0.4],
                    [0.28, 0.15],
                    [0.14, 0.0],
                    [0.78, 0.0],
                ],
                vec![[0.14, 0.52], [0.58, 0.52]],
            ],
            fills: vec![],
            advance: 0.9,
        },
        'Æ' | 'æ' | 'Œ' | 'œ' => {
            let (a, b, overlap) = match c {
                'Æ' => ('A', 'E', 0.3),
                'æ' => ('a', 'e', 0.25),
                'Œ' => ('O', 'E', 0.3),
                _ => ('o', 'e', 0.25),
            };
            let (a, b) = (lat(a)?, lat(b)?);
            let mut g = a.clone();
            let x = a.advance - overlap;
            g.add(&b, [1.0, 1.0], [x, 0.0]);
            g.advance = x + b.advance;
            g
        }
        '×' => ascii(Face::MathLower, '#')?,
        _ => return None,
    })
}

/// Greek letters, in the Greek faces' ASCII places (alphabetical from A).
fn greek_ascii(c: char) -> Option<char> {
    let u = c as u32;
    let k = match u {
        0x391..=0x3a1 => u - 0x391,
        0x3a3..=0x3a9 => u - 0x392,
        0x3b1..=0x3c1 => return char::from_u32('a' as u32 + u - 0x3b1),
        0x3c2 => return Some('r'),
        0x3c3..=0x3c9 => return char::from_u32('a' as u32 + u - 0x3b2),
        0x3d1 => return Some('h'),
        0x3d5 => return Some('u'),
        _ => return None,
    };
    char::from_u32('A' as u32 + k)
}

/// Cyrillic letters, in the Cyrillic face's ASCII places (a transliteration layout).
fn cyrillic_ascii(c: char) -> Option<char> {
    // А Б В Г Д Е Ж З И Й К Л М Н О П Р С Т У Ф Х Ц Ч Ш Щ Ъ Ы Ь Э Ю Я, and the lower case.
    const UPPER: &[u8; 32] = b"ABVGD[HZIEKLMNOPRSTYFX`JQW]$_CU^";
    const LOWER: &[u8; 32] = b"abvgd{hzieklmnoprstyfx%jqw|&~cu}";
    let u = c as u32;
    match u {
        0x410..=0x42f => Some(UPPER[(u - 0x410) as usize] as char),
        0x430..=0x44f => Some(LOWER[(u - 0x430) as usize] as char),
        _ => None,
    }
}

/// Mathematical signs, in the math face's ASCII places.
fn math_ascii(c: char) -> Option<char> {
    Some(match c {
        '±' => '!',
        '∓' => '"',
        '÷' => 'x',
        '°' | '\u{2218}' => '`',
        '≤' => '&',
        '≥' => '\'',
        '≠' => '?',
        '≡' => '@',
        '∞' => '_',
        '∝' => '^',
        '√' => 'b',
        '⊂' => 'd',
        '⊃' => 'e',
        '∪' => 'f',
        '∩' => 'g',
        '∈' => 'h',
        '→' => 'i',
        '↑' => 'j',
        '←' => 'k',
        '↓' => 'l',
        '∂' => 'm',
        '∇' => 'n',
        '∫' => 'p',
        '‖' => 'y',
        '⊥' => 'z',
        '∠' => '|',
        '∏' => ':',
        '∑' => ';',
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Accent {
    Grave,
    Acute,
    Circumflex,
    Tilde,
    Diaeresis,
    Ring,
    Cedilla,
    Caron,
    Macron,
    Dot,
    DoubleAcute,
    Breve,
    Ogonek,
}

/// An accented letter as its base letter and accent. `ı` is the dotless i.
fn decompose(c: char) -> Option<(char, Accent)> {
    const TABLE: [(Accent, &str); 13] = [
        (Accent::Grave, "ÀAÈEÌIÒOÙUàaèeìıòoùu"),
        (Accent::Acute, "ÁAÉEÍIÓOÚUÝYáaéeíıóoúuýyĆCćcŃNńnŚSśsŹZźzĹLĺlŔRŕr"),
        (Accent::Circumflex, "ÂAÊEÎIÔOÛUâaêeîıôoûuĈCĉcĜGĝgŜSŝsŴWŵwŶYŷy"),
        (Accent::Tilde, "ÃAÑNÕOãañnõoĨIĩıŨUũu"),
        (Accent::Diaeresis, "ÄAËEÏIÖOÜUäaëeïıöoüuÿyŸYЁЕёе"),
        (Accent::Ring, "ÅAåaŮUůu"),
        (Accent::Cedilla, "ÇCçcŞSşsŢTţt"),
        (Accent::Caron, "ČCčcĎDďdĚEěeŇNňnŘRřrŠSšsŤTťtŽZžz"),
        (Accent::Macron, "ĀAāaĒEēeĪIīıŌOōoŪUūu"),
        (Accent::Dot, "ŻZżzĖEėeĊCċcĠGġgİI"),
        (Accent::DoubleAcute, "ŐOőoŰUűu"),
        (Accent::Breve, "ĂAăaĞGğgŬUŭu"),
        (Accent::Ogonek, "ĄAąaĘEęeĮIįıŲUųu"),
    ];
    for (acc, pairs) in TABLE {
        let cs: Vec<char> = pairs.chars().collect();
        for p in cs.as_chunks::<2>().0 {
            if p[0] == c {
                return Some((p[1], acc));
            }
        }
    }
    None
}

/// The face's i without its dot (the strokes that stay below the x-height).
fn dotless_i(face: Face) -> Option<Glyph> {
    let mut g = ascii(face, 'i')?;
    g.strokes
        .retain(|s| s.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min) < 0.7);
    Some(g)
}

fn dot(x: f64, y: f64) -> Vec<[f64; 2]> {
    let r = 0.045;
    vec![[x, y + r], [x - r, y], [x, y - r], [x + r, y], [x, y + r]]
}

/// The accent's strokes about (0, 0), the middle of its foot (for the accents set above the
/// letter) or of its head (cedilla and ogonek, hung below).
fn accent_strokes(a: Accent) -> Vec<Vec<[f64; 2]>> {
    let tol = 0.004;
    match a {
        Accent::Grave => vec![vec![[0.07, 0.0], [-0.07, 0.17]]],
        Accent::Acute => vec![vec![[-0.07, 0.0], [0.07, 0.17]]],
        Accent::Circumflex => vec![vec![[-0.14, 0.0], [0.0, 0.15], [0.14, 0.0]]],
        Accent::Caron => vec![vec![[-0.14, 0.15], [0.0, 0.0], [0.14, 0.15]]],
        Accent::Tilde => vec![vec![
            [-0.17, 0.02],
            [-0.13, 0.1],
            [-0.07, 0.12],
            [0.07, 0.04],
            [0.13, 0.06],
            [0.17, 0.14],
        ]],
        Accent::Diaeresis => vec![dot(-0.12, 0.05), dot(0.12, 0.05)],
        Accent::Dot => vec![dot(0.0, 0.05)],
        Accent::Ring => vec![glyph::circle([0.0, 0.08], 0.08, tol / 4.0)],
        Accent::Macron => vec![vec![[-0.16, 0.04], [0.16, 0.04]]],
        Accent::DoubleAcute => vec![vec![[-0.13, 0.0], [-0.03, 0.17]], vec![[0.05, 0.0], [0.15, 0.17]]],
        Accent::Breve => {
            let mut v = vec![[-0.14, 0.15]];
            glyph::arc_points(
                [0.0, 0.15],
                0.14,
                forge_math::dmath::PI,
                forge_math::dmath::PI,
                tol,
                &mut v,
            );
            vec![v]
        }
        Accent::Cedilla => vec![vec![
            [0.0, 0.0],
            [0.0, -0.08],
            [0.06, -0.1],
            [0.08, -0.15],
            [0.04, -0.2],
            [-0.06, -0.21],
        ]],
        Accent::Ogonek => vec![vec![
            [0.0, 0.0],
            [-0.08, -0.08],
            [-0.08, -0.15],
            [0.0, -0.2],
            [0.06, -0.19],
        ]],
    }
}

fn accented(base: Glyph, a: Accent, italic: bool) -> Glyph {
    let (lo, hi) = base.bbox().unwrap_or(([0.0, 0.0], [base.advance, 0.67]));
    let mid = (lo[0] + hi[0]) / 2.0;
    let (x, y) = match a {
        Accent::Cedilla => (mid, 0.0),
        Accent::Ogonek => (hi[0] - 0.08, 0.0),
        _ => (mid, hi[1] + 0.08),
    };
    // Italic accents lean with the letter.
    let x = if italic { x + 0.2 * (y - 0.5 * hi[1]) } else { x };
    let mut g = base;
    for s in accent_strokes(a) {
        g.strokes.push(s.iter().map(|p| [p[0] + x, p[1] + y]).collect());
    }
    g
}

fn slashed(base: Glyph, over: f64) -> Glyph {
    let Some((lo, hi)) = base.bbox() else {
        return base;
    };
    let mut g = base;
    g.strokes
        .push(vec![[lo[0] - over, lo[1] - over], [hi[0] + over, hi[1] + over]]);
    g
}

fn superscript(g: Glyph) -> Glyph {
    let s = 0.6;
    let mut out = g.transformed([s, s], [0.0, 0.45]);
    out.advance = g.advance * s;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_holds_together() {
        assert_eq!(faces().len(), FACES.len());
        for (k, f) in FACES.iter().enumerate() {
            assert_eq!(f.face as usize, k);
        }
        // Simplex H: two stems and a bar, cap height 1, advance 22/21.
        let h = glyph(Face::Simplex, Spacing::Normal, 'H').unwrap();
        assert_eq!(h.strokes.len(), 3);
        let (lo, hi) = h.bbox().unwrap();
        assert!((hi[1] - 1.0).abs() < 1e-12 && lo[1].abs() < 1e-12);
        assert!((h.advance - 22.0 / 21.0).abs() < 1e-12);
    }

    #[test]
    fn every_face_has_its_ascii() {
        for f in &FACES {
            for c in '!'..='~' {
                let g = glyph(f.face, Spacing::Normal, c);
                assert!(g.is_some(), "{} {c}", f.file);
            }
        }
    }

    #[test]
    fn names_map_to_faces() {
        assert_eq!(for_shx("romans"), Some((Face::Simplex, Spacing::Normal)));
        assert_eq!(for_shx("txt"), Some((Face::Simplex, Spacing::Txt)));
        assert_eq!(for_shx("monotxt"), Some((Face::Simplex, Spacing::Mono)));
        assert_eq!(for_shx("isocpeur"), Some((Face::Simplex, Spacing::Normal)));
        assert_eq!(for_shx("romand").unwrap().0, Face::Duplex);
        assert_eq!(for_shx("romanc").unwrap().0, Face::Complex);
        assert_eq!(for_shx("italicc").unwrap().0, Face::ItalicComplex);
        assert_eq!(for_shx("romant").unwrap().0, Face::Triplex);
        assert_eq!(for_shx("scriptc").unwrap().0, Face::ScriptComplex);
        assert_eq!(for_shx("gothicg").unwrap().0, Face::GothicGerman);
        assert_eq!(for_shx("greekc").unwrap().0, Face::GreekComplex);
        assert_eq!(for_shx("syastro").unwrap().0, Face::Astrology);
        assert_eq!(for_shx("gbcbig"), None);
        assert_eq!(for_shx("arial"), None);
    }

    #[test]
    fn composed_characters() {
        let s = |c| glyph(Face::Simplex, Spacing::Normal, c);
        // Accented capitals carry the accent above the cap height, lower case above x-height.
        let (_, hi) = s('É').unwrap().bbox().unwrap();
        assert!(hi[1] > 1.1);
        let (_, hi) = s('é').unwrap().bbox().unwrap();
        assert!(hi[1] > 0.7 && hi[1] < 1.0);
        // í has no dot of its own under the accent.
        assert_eq!(s('í').unwrap().strokes.len(), 2);
        for c in "°±⌀∅ØøÄÖÜäöüßÇçÑñÅå²³¼½¾¿¡µ×÷©®€£¥ÆæŒœČšŽłαβγΩΣПривет…–—«»·•ЁёĄę".chars()
        {
            assert!(s(c).is_some(), "{c}");
        }
        assert_eq!(s('\u{4e2d}'), None);
        // Greek and Cyrillic come from those faces.
        assert_eq!(s('Ω'), glyph(Face::GreekSimplex, Spacing::Normal, 'X'));
        assert_eq!(s('Я'), glyph(Face::Cyrillic, Spacing::Normal, '^'));
        assert_eq!(s('щ'), glyph(Face::Cyrillic, Spacing::Normal, 'w'));
    }

    #[test]
    fn mono_and_txt_spacing() {
        let m = glyph(Face::Simplex, Spacing::Mono, 'i').unwrap();
        assert_eq!(m.advance, 1.0);
        let t = glyph(Face::Simplex, Spacing::Txt, 'A').unwrap();
        let n = glyph(Face::Simplex, Spacing::Normal, 'A').unwrap();
        assert!(t.advance > n.advance);
    }
}

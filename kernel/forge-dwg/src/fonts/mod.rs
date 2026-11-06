//! Fonts for a drawing's text: what shapes a TEXT or MTEXT run into strokes and filled rings.
//!
//! A STYLE names a font file ("txt", "romans.shx", "arial.ttf", sometimes a family such as
//! "Arial") and maybe a big font for East Asian text. `FontSet` resolves those names, in order,
//! to: a font the app has loaded by that file name (SHX or TrueType; case, path and extension
//! don't matter, a TrueType font's family name works too), else the built-in Hershey stand-in for
//! an Autodesk SHX name (`hershey`), else a default TrueType font if one is set, else Simplex. It
//! never touches a file system: fonts come in as bytes (`add`), so the browser build works the
//! same; `add_dir` reads a directory on native builds.
//!
//! Text goes in as the drawing stores it: the %%d, %%p, %%c, %%%, %%nnn, %%u and %%o codes and
//! the \U+XXXX and \M+nXXXX escapes are read here. With a big font, a lead byte in its escape
//! ranges and the byte after it (as the drawing's code-page bytes, read as Windows-1252) make one
//! big-font character.
//!
//! Shaping is deterministic (forge-math's trig, no platform libm), and cached: fonts are parsed
//! once, each (font, character) glyph is built once, and each style's names are resolved once.

pub mod glyph;
pub mod hershey;
pub mod shx;
mod text;
pub mod ttf;

pub use glyph::Glyph;
use hershey::{Face, Spacing};
use shx::{Kind, ShxFont};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, RwLock};
use text::Tok;
use ttf::TtfFont;

use crate::scene::{Fonts, Shaped};

/// A font file given to the set.
enum Loaded {
    Shx(ShxFont),
    Ttf(TtfFont),
}

/// Where a style's characters come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Src {
    /// A loaded font, by its index.
    File(usize),
    Builtin(Face, Spacing),
}

#[derive(Clone, Copy, Debug)]
struct Resolved {
    main: Src,
    /// The stand-in for characters the main font lacks.
    stand_in: (Face, Spacing),
    /// The big font: a loaded one's index, or Missing when one is named but not loaded.
    big: Big,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Big {
    None,
    File(usize),
    Missing,
}

/// A font a style names that was not found, and what stands in for it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Missing {
    /// The name as the style has it.
    pub font: String,
    /// What is used instead ("Hershey Roman Simplex (built-in stand-in)", ..).
    pub using: String,
}

impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "font {} not found, using {}", self.font, self.using)
    }
}

/// A key for the glyph cache: the source and the character or shape.
type Key = (Src, u32);

const CODE: u32 = 0x8000_0000;
const DBCS: u32 = 0x4000_0000;

/// Fonts loaded from bytes plus the built-in stand-ins.
#[derive(Default)]
pub struct FontSet {
    fonts: Vec<(String, Loaded)>,
    /// Lower-case file names, stems and family names to font indices.
    by_name: HashMap<String, usize>,
    default_ttf: Option<String>,
    /// Files that would not parse, and why.
    errors: Vec<(String, String)>,
    resolved: RwLock<HashMap<(String, String), Resolved>>,
    glyphs: RwLock<HashMap<Key, Arc<Glyph>>>,
    missing: Mutex<BTreeMap<String, String>>,
}

/// A name as a style might give it, reduced to a key: the file name without its path, lower case.
fn file_key(name: &str) -> String {
    let n = name.trim().trim_end_matches('\0');
    let n = n.rsplit(['/', '\\']).next().unwrap_or(n);
    n.trim().to_lowercase()
}

/// The key without a font extension.
fn stem(key: &str) -> &str {
    for ext in [".shx", ".ttf", ".ttc", ".otf", ".shp"] {
        if let Some(s) = key.strip_suffix(ext) {
            return s;
        }
    }
    key
}

impl FontSet {
    pub fn new() -> FontSet {
        FontSet::default()
    }

    /// Adds a font file (SHX or TrueType, told apart by its contents) under its file name. A
    /// file that will not parse is kept in `errors` and otherwise ignored.
    pub fn add(&mut self, file_name: &str, bytes: Vec<u8>) {
        let key = file_key(file_name);
        let loaded = if bytes.starts_with(b"AutoCAD-86") {
            ShxFont::parse(bytes).map(Loaded::Shx).map_err(|e| e.to_string())
        } else {
            TtfFont::parse(bytes).map(Loaded::Ttf)
        };
        let font = match loaded {
            Ok(f) => f,
            Err(e) => {
                self.errors.push((file_name.to_string(), e));
                return;
            }
        };
        let k = self.fonts.len();
        let shx = matches!(font, Loaded::Shx(_));
        let families = match &font {
            Loaded::Ttf(t) => t.names.clone(),
            Loaded::Shx(_) => vec![],
        };
        self.fonts.push((file_name.to_string(), font));
        // The stem: an SHX file wins over a TrueType one, as AutoCAD looks for .shx first.
        let s = stem(&key).to_string();
        let ttf_there = |j: usize| matches!(self.fonts[j].1, Loaded::Ttf(_));
        if self
            .by_name
            .get(&s)
            .is_none_or(|&j| shx && ttf_there(j) || s == key)
        {
            self.by_name.insert(s, k);
        }
        self.by_name.insert(key, k);
        for f in families {
            self.by_name.entry(f).or_insert(k);
        }
        self.forget();
    }

    /// Every .shx, .ttf and .otf file in a directory (not its subdirectories): how many loaded.
    #[cfg(not(target_family = "wasm"))]
    pub fn add_dir(&mut self, dir: &std::path::Path) -> std::io::Result<usize> {
        let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "shx" | "ttf" | "otf"))
            })
            .collect();
        paths.sort();
        let before = self.fonts.len();
        for p in paths {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            self.add(&name, std::fs::read(&p)?);
        }
        Ok(self.fonts.len() - before)
    }

    /// The TrueType font (by a name already added) to use for styles whose font is neither
    /// loaded nor an Autodesk SHX name.
    pub fn set_default_ttf(&mut self, name: Option<&str>) {
        self.default_ttf = name.map(file_key);
        self.forget();
    }

    /// Files given to `add` that would not parse, and why.
    pub fn errors(&self) -> &[(String, String)] {
        &self.errors
    }

    /// The fonts styles have asked for that were not loaded, and what stood in, for the UI to
    /// tell the user. Filled as text is shaped.
    pub fn missing(&self) -> Vec<Missing> {
        let m = self.missing.lock().unwrap_or_else(|e| e.into_inner());
        m.iter()
            .map(|(font, using)| Missing {
                font: font.clone(),
                using: using.clone(),
            })
            .collect()
    }

    fn forget(&mut self) {
        self.resolved.get_mut().unwrap_or_else(|e| e.into_inner()).clear();
        self.glyphs.get_mut().unwrap_or_else(|e| e.into_inner()).clear();
        self.missing.get_mut().unwrap_or_else(|e| e.into_inner()).clear();
    }

    fn find(&self, name: &str) -> Option<usize> {
        let key = file_key(name);
        if key.is_empty() {
            return None;
        }
        self.by_name
            .get(&key)
            .or_else(|| self.by_name.get(stem(&key)))
            .copied()
    }

    fn note(&self, font: &str, using: String) {
        let mut m = self.missing.lock().unwrap_or_else(|e| e.into_inner());
        m.entry(font.trim().to_string()).or_insert(using);
    }

    fn resolve(&self, font: &str, bigfont: &str) -> Resolved {
        let k = (font.to_string(), bigfont.to_string());
        if let Some(r) = self.resolved.read().unwrap_or_else(|e| e.into_inner()).get(&k) {
            return *r;
        }
        let r = self.resolve_new(font, bigfont);
        self.resolved
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(k, r);
        r
    }

    fn resolve_new(&self, font: &str, bigfont: &str) -> Resolved {
        // AutoCAD's default style font when a style names none.
        let font = if file_key(font).is_empty() { "txt" } else { font };
        let key = file_key(font);
        let builtin = hershey::for_shx(stem(&key));
        let stand_in = builtin.unwrap_or((Face::Simplex, Spacing::Normal));
        let main = if let Some(k) = self.find(font) {
            Src::File(k)
        } else if let Some((face, sp)) = builtin {
            self.note(font, format!("{} (built-in stand-in)", face.title()));
            Src::Builtin(face, sp)
        } else if let Some(k) = self.default_ttf.as_deref().and_then(|d| self.find(d)) {
            self.note(font, format!("{} (the default font)", self.fonts[k].0));
            Src::File(k)
        } else {
            self.note(font, format!("{} (built-in stand-in)", Face::Simplex.title()));
            Src::Builtin(Face::Simplex, Spacing::Normal)
        };
        let big = if file_key(bigfont).is_empty() {
            Big::None
        } else {
            match self.find(bigfont) {
                Some(k) if matches!(&self.fonts[k].1, Loaded::Shx(f) if f.kind == Kind::Bigfont) => {
                    Big::File(k)
                }
                _ => {
                    self.note(
                        bigfont,
                        "nothing: its East Asian characters show as boxes".to_string(),
                    );
                    Big::Missing
                }
            }
        };
        Resolved { main, stand_in, big }
    }

    fn shx(&self, k: usize) -> Option<&ShxFont> {
        match &self.fonts[k].1 {
            Loaded::Shx(f) => Some(f),
            Loaded::Ttf(_) => None,
        }
    }

    fn ttf(&self, src: Src) -> Option<&TtfFont> {
        match src {
            Src::File(k) => match &self.fonts[k].1 {
                Loaded::Ttf(f) => Some(f),
                Loaded::Shx(_) => None,
            },
            Src::Builtin(..) => None,
        }
    }

    /// A cached glyph, built by `make` the first time.
    fn cached(&self, key: Key, make: impl FnOnce() -> Option<Glyph>) -> Option<Arc<Glyph>> {
        if let Some(g) = self.glyphs.read().unwrap_or_else(|e| e.into_inner()).get(&key) {
            return Some(g.clone());
        }
        let g = Arc::new(make()?);
        self.glyphs
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, g.clone());
        Some(g)
    }

    /// A character from a source, or None if it has none.
    fn char_glyph(&self, src: Src, c: char) -> Option<Arc<Glyph>> {
        self.cached((src, c as u32), || match src {
            Src::Builtin(face, sp) => hershey::glyph(face, sp, c),
            Src::File(k) => match &self.fonts[k].1 {
                Loaded::Ttf(f) => f.glyph(c),
                Loaded::Shx(f) => shx_numbers(f, c).into_iter().find_map(|n| f.glyph(n)),
            },
        })
    }

    /// A character in the style: from its font, else the stand-in, else a question mark.
    fn glyph(&self, r: &Resolved, c: char) -> Arc<Glyph> {
        let (face, sp) = r.stand_in;
        self.char_glyph(r.main, c)
            .or_else(|| self.char_glyph(Src::Builtin(face, sp), c))
            .or_else(|| self.char_glyph(r.main, '?'))
            .or_else(|| self.char_glyph(Src::Builtin(face, sp), '?'))
            .unwrap_or_default()
    }

    /// %%nnn: an SHX font's shape nnn; elsewhere the code-page character.
    fn code_glyph(&self, r: &Resolved, n: u16) -> Arc<Glyph> {
        if let Src::File(k) = r.main
            && let Some(f) = self.shx(k)
            && let Some(g) = self.cached((r.main, CODE | n as u32), || f.glyph(n))
        {
            return g;
        }
        let c = match n {
            127 => '°',
            128 => '±',
            129 => '\u{2300}',
            0..=255 => crate::str::decode_1252(&[n as u8]).chars().next().unwrap_or('?'),
            _ => char::from_u32(n as u32).unwrap_or('?'),
        };
        self.glyph(r, c)
    }

    /// A big-font character: from the big font, else a box one em square.
    fn big_glyph(&self, r: &Resolved, code: u16) -> Arc<Glyph> {
        if let Big::File(k) = r.big
            && let Some(g) = self.cached((Src::File(k), DBCS | code as u32), || self.shx(k)?.glyph(code))
        {
            return g;
        }
        self.cached((Src::Builtin(Face::Simplex, Spacing::Normal), DBCS), || {
            Some(Glyph {
                strokes: vec![vec![
                    [0.1, -0.1],
                    [0.9, -0.1],
                    [0.9, 0.9],
                    [0.1, 0.9],
                    [0.1, -0.1],
                ]],
                fills: vec![],
                advance: 1.0,
            })
        })
        .unwrap_or_default()
    }

    /// Shapes text set vertically (a style with the vertical flag): characters stacked down
    /// from the origin, SHX fonts' vertical-only commands run. `advance` is the height taken,
    /// down the page.
    pub fn shape_vertical(&self, font: &str, bigfont: &str, text: &str) -> Shaped {
        let r = self.resolve(font, bigfont);
        let mut out = Shaped::default();
        let mut y = 0.0;
        for t in self.tokens(&r, text) {
            let g: Arc<Glyph> = match t {
                Tok::Ch(c) => {
                    let shx = match r.main {
                        Src::File(k) => self.shx(k),
                        Src::Builtin(..) => None,
                    };
                    let v = shx.and_then(|f| {
                        self.cached((r.main, 0x2000_0000 | c as u32), || {
                            shx_numbers(f, c).into_iter().find_map(|n| f.vertical_glyph(n))
                        })
                    });
                    match v {
                        Some(g) => {
                            add(&mut out, &g, [0.0, y]);
                            y -= g.advance;
                            continue;
                        }
                        None => self.glyph(&r, c),
                    }
                }
                Tok::Code(n) => self.code_glyph(&r, n),
                Tok::Dbcs(n) => self.big_glyph(&r, n),
                Tok::Under | Tok::Over => continue,
            };
            // A horizontal glyph in a column: centred on the column, one line per character.
            add(&mut out, &g, [-g.advance / 2.0, y - 1.0]);
            y -= 1.6;
        }
        out.advance = -y;
        out
    }

    /// The text's tokens, with big-font pairs joined.
    fn tokens(&self, r: &Resolved, text: &str) -> Vec<Tok> {
        let toks = text::tokens(text);
        if r.big == Big::None {
            return toks;
        }
        let lead = |b: u8| match r.big {
            Big::File(k) => self.shx(k).is_some_and(|f| f.is_lead(b)),
            _ => (0x81..=0xfe).contains(&b),
        };
        let mut out = Vec::with_capacity(toks.len());
        let mut k = 0;
        while k < toks.len() {
            if let (Tok::Ch(a), Some(Tok::Ch(b))) = (toks[k], toks.get(k + 1))
                && let (Some(x), Some(y)) = (cp_byte(a), cp_byte(*b))
                && x >= 0x80
                && lead(x)
            {
                out.push(Tok::Dbcs(u16::from_be_bytes([x, y])));
                k += 2;
                continue;
            }
            out.push(toks[k]);
            k += 1;
        }
        out
    }
}

/// A character's Windows-1252 byte, if it has one.
fn cp_byte(c: char) -> Option<u8> {
    let mut buf = [0u8; 4];
    match crate::str::encode_1252(c.encode_utf8(&mut buf))[..] {
        [b] => Some(b),
        _ => None,
    }
}

/// The shape numbers a character may have in an SHX font, best first.
fn shx_numbers(f: &ShxFont, c: char) -> Vec<u16> {
    let u = c as u32;
    let mut v = Vec::with_capacity(3);
    match f.kind {
        Kind::Unifont | Kind::Bigfont => {
            if u <= 0xffff {
                v.push(u as u16);
            }
        }
        Kind::Shapes => {
            if let Some(b) = cp_byte(c) {
                v.push(b as u16);
            }
        }
    }
    // The old fonts' %%d, %%p and %%c shapes, and the diameter's other code points.
    match c {
        '°' => v.push(127),
        '±' => v.push(128),
        '\u{2300}' | '\u{2205}' => v.extend([0x2205, 0x2300, 129]),
        _ => {}
    }
    v
}

fn add(out: &mut Shaped, g: &Glyph, d: [f64; 2]) {
    let m = |p: &[f64; 2]| [p[0] + d[0], p[1] + d[1]];
    out.strokes
        .extend(g.strokes.iter().map(|l| l.iter().map(m).collect::<Vec<_>>()));
    out.fills
        .extend(g.fills.iter().map(|l| l.iter().map(m).collect::<Vec<_>>()));
}

/// Where %%u and %%o draw their lines.
const UNDERLINE: f64 = -0.2;
const OVERLINE: f64 = 1.2;

impl Fonts for FontSet {
    fn shape(&self, font: &str, bigfont: &str, text: &str) -> Shaped {
        let r = self.resolve(font, bigfont);
        let ttf = self.ttf(r.main);
        let mut out = Shaped::default();
        let mut x = 0.0;
        let mut under: Option<f64> = None;
        let mut over: Option<f64> = None;
        let mut prev: Option<char> = None;
        for t in self.tokens(&r, text) {
            let g = match t {
                Tok::Ch(c) => {
                    if let (Some(f), Some(p)) = (ttf, prev) {
                        x += f.kerning(p, c);
                    }
                    prev = Some(c);
                    self.glyph(&r, c)
                }
                Tok::Code(n) => {
                    prev = None;
                    self.code_glyph(&r, n)
                }
                Tok::Dbcs(n) => {
                    prev = None;
                    self.big_glyph(&r, n)
                }
                Tok::Under | Tok::Over => {
                    let (line, y) = if t == Tok::Under {
                        (&mut under, UNDERLINE)
                    } else {
                        (&mut over, OVERLINE)
                    };
                    match line.take() {
                        Some(x0) => out.strokes.push(vec![[x0, y], [x, y]]),
                        None => *line = Some(x),
                    }
                    continue;
                }
            };
            add(&mut out, &g, [x, 0.0]);
            x += g.advance;
        }
        for (line, y) in [(under, UNDERLINE), (over, OVERLINE)] {
            if let Some(x0) = line
                && x > x0
            {
                out.strokes.push(vec![[x0, y], [x, y]]);
            }
        }
        out.advance = x;
        out
    }
}

#[cfg(test)]
mod tests;

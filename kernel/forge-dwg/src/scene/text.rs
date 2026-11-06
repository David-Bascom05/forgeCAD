//! Text: TEXT's special codes and alignment, MTEXT's formatting codes, wrapping, line spacing and
//! attachment. Everything is laid out in the text's own plane (x along the baseline, y up, the
//! insertion point at the origin); the builder places it.

use super::color::{Says, index_says};
use super::geom::P2;
use crate::scene::{Fonts, Shaped};
use forge_math::dmath;

/// How far below the baseline a font's descenders reach, in cap heights.
pub const DESCENT: f64 = 1.0 / 3.0;

/// Laid-out text: strokes and filled rings, each with an MTEXT colour override.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Laid {
    pub strokes: Vec<(Option<Says>, Vec<P2>)>,
    pub fills: Vec<(Option<Says>, Vec<Vec<P2>>)>,
    /// The box the text was laid out in (min, max).
    pub lo: P2,
    pub hi: P2,
}

impl Laid {
    pub fn map(&mut self, f: impl Fn(P2) -> P2) {
        for (_, s) in &mut self.strokes {
            s.iter_mut().for_each(|p| *p = f(*p));
        }
        for (_, rs) in &mut self.fills {
            rs.iter_mut().flatten().for_each(|p| *p = f(*p));
        }
    }
}

/// The font a piece of text is set in.
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    pub file: String,
    pub big: String,
}

/// A glyph run's look.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub font: Font,
    pub height: f64,
    pub width: f64,
    /// Radians, clockwise from vertical as AutoCAD measures it (positive leans right).
    pub oblique: f64,
    pub tracking: f64,
    pub color: Option<Says>,
    pub under: bool,
    pub over: bool,
    pub strike: bool,
}

fn tan(a: f64) -> f64 {
    let (s, c) = dmath::sin_cos(a);
    if c.abs() < 1e-12 { 0.0 } else { s / c }
}

/// Puts a run of shaped text at (x, y) with its look; its width.
fn place(sh: &Shaped, look: &Look, x: f64, y: f64, out: &mut Laid) -> f64 {
    let h = look.height;
    let w = look.width;
    let sk = tan(look.oblique);
    let f = |q: &[f64; 2]| [x + q[0] * h * w + q[1] * h * sk, y + q[1] * h];
    for s in &sh.strokes {
        out.strokes.push((look.color, s.iter().map(f).collect()));
    }
    if !sh.fills.is_empty() {
        out.fills.push((
            look.color,
            sh.fills.iter().map(|r| r.iter().map(f).collect()).collect(),
        ));
    }
    let adv = sh.advance * h * w;
    let mut deco = |dy: f64| {
        out.strokes
            .push((look.color, vec![[x, y + dy * h], [x + adv, y + dy * h]]))
    };
    if look.under {
        deco(-0.2);
    }
    if look.over {
        deco(1.2);
    }
    if look.strike {
        deco(0.5);
    }
    adv
}

/// TEXT's special codes: %%c %%d %%p %%% %%nnn, and %%u %%o turning underline and overline on
/// and off. The pieces with their underline and overline.
pub fn text_codes(s: &str) -> Vec<(String, bool, bool)> {
    let mut out = vec![];
    let mut cur = String::new();
    let (mut u, mut o) = (false, false);
    let ch: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < ch.len() {
        if ch[i] == '%' && i + 2 < ch.len() + 1 && ch.get(i + 1) == Some(&'%') && i + 2 < ch.len() {
            let c = ch[i + 2];
            i += 3;
            match c.to_ascii_lowercase() {
                'c' => cur.push('\u{2205}'),
                'd' => cur.push('\u{B0}'),
                'p' => cur.push('\u{B1}'),
                '%' => cur.push('%'),
                'u' | 'o' => {
                    if !cur.is_empty() {
                        out.push((std::mem::take(&mut cur), u, o));
                    }
                    if c.eq_ignore_ascii_case(&'u') {
                        u = !u;
                    } else {
                        o = !o;
                    }
                }
                d if d.is_ascii_digit() => {
                    let mut n = d.to_digit(10).unwrap_or(0);
                    let mut k = 0;
                    while k < 2 && i < ch.len() && ch[i].is_ascii_digit() {
                        n = n * 10 + ch[i].to_digit(10).unwrap_or(0);
                        i += 1;
                        k += 1;
                    }
                    if let Some(x) = char::from_u32(n) {
                        cur.push(x);
                    }
                }
                _ => {
                    cur.push('%');
                    cur.push('%');
                    cur.push(c);
                }
            }
            continue;
        }
        if ch[i] == '\\'
            && matches!(ch.get(i + 1), Some('U' | 'u'))
            && ch.get(i + 2) == Some(&'+')
            && let Some(x) = hex_char(&ch, i + 3)
        {
            cur.push(x);
            i += 7;
            continue;
        }
        cur.push(ch[i]);
        i += 1;
    }
    if !cur.is_empty() || out.is_empty() {
        out.push((cur, u, o));
    }
    out
}

fn hex_char(ch: &[char], at: usize) -> Option<char> {
    let s: String = ch.get(at..at + 4)?.iter().collect();
    char::from_u32(u32::from_str_radix(&s, 16).ok()?)
}

/// A single line of TEXT on its baseline from x = 0: the laid-out glyphs and the advance.
pub fn line(fonts: &dyn Fonts, s: &str, look: &Look) -> (Laid, f64) {
    let mut out = Laid::default();
    let mut x = 0.0;
    for (piece, u, o) in text_codes(s) {
        let l = Look {
            under: u,
            over: o,
            ..look.clone()
        };
        let sh = fonts.shape(&look.font.file, &look.font.big, &piece);
        x += place(&sh, &l, x, 0.0, &mut out);
    }
    out.lo = [0.0, -DESCENT * look.height];
    out.hi = [x, look.height];
    (out, x)
}

/// TEXT alignment: where the line's own origin goes, given the natural advance. Returns the
/// (x, y) offset of the baseline start from the reference point, with halign 0..5 (left,
/// centre, right, aligned, middle, fit) and valign 0..3 (baseline, bottom, middle, top).
pub fn align_offset(halign: i16, valign: i16, adv: f64, h: f64) -> P2 {
    let x = match halign {
        1 | 4 => -adv / 2.0,
        2 => -adv,
        _ => 0.0,
    };
    let y = if halign == 4 {
        -h / 2.0
    } else if matches!(halign, 3 | 5) {
        0.0
    } else {
        match valign {
            1 => DESCENT * h,
            2 => -h / 2.0,
            3 => -h,
            _ => 0.0,
        }
    };
    [x, y]
}

/// A token of MTEXT.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Word(String, Look),
    Space(Look, bool),
    Tab(Look),
    /// Top, bottom, the kind (^ tolerance, / fraction, # diagonal).
    Stack(String, String, char, Look),
    /// A paragraph ends; the next one's alignment when set (l, c, r).
    Para,
    Align(char),
}

/// MTEXT's formatting codes, into tokens.
pub fn parse(s: &str, base: &Look) -> Vec<Tok> {
    let ch: Vec<char> = s.chars().collect();
    let mut stack: Vec<Look> = vec![];
    let mut look = base.clone();
    let mut out = vec![];
    let mut word = String::new();
    let mut i = 0;
    let flush = |word: &mut String, look: &Look, out: &mut Vec<Tok>| {
        if !word.is_empty() {
            out.push(Tok::Word(std::mem::take(word), look.clone()));
        }
    };
    // a code's argument, up to the ';' (or the end)
    let arg = |i: &mut usize| -> String {
        let mut a = String::new();
        while *i < ch.len() && ch[*i] != ';' {
            a.push(ch[*i]);
            *i += 1;
        }
        *i += 1;
        a
    };
    let num = |a: &str| -> Option<f64> { a.trim().trim_end_matches(['x', 'X']).trim().parse::<f64>().ok() };
    while i < ch.len() {
        let c = ch[i];
        match c {
            '\\' if i + 1 < ch.len() => {
                let code = ch[i + 1];
                i += 2;
                match code {
                    'P' | 'X' | 'N' => {
                        flush(&mut word, &look, &mut out);
                        out.push(Tok::Para);
                    }
                    '~' => {
                        flush(&mut word, &look, &mut out);
                        out.push(Tok::Space(look.clone(), true));
                    }
                    '\\' | '{' | '}' => word.push(code),
                    'U' | 'u' if ch.get(i) == Some(&'+') => {
                        if let Some(x) = hex_char(&ch, i + 1) {
                            word.push(x);
                            i += 5;
                        }
                    }
                    'M' | 'm' if ch.get(i) == Some(&'+') => {
                        // a multibyte character in some code page: not ours to read
                        i += 6.min(ch.len() - i);
                        word.push('?');
                    }
                    'f' | 'F' => {
                        flush(&mut word, &look, &mut out);
                        let a = arg(&mut i);
                        let mut parts = a.split('|');
                        let name = parts.next().unwrap_or("").to_string();
                        if code == 'F' {
                            look.font = Font {
                                file: name,
                                big: parts.next().unwrap_or("").to_string(),
                            };
                        } else {
                            look.font = Font {
                                file: name,
                                big: String::new(),
                            };
                        }
                    }
                    'H' => {
                        flush(&mut word, &look, &mut out);
                        let a = arg(&mut i);
                        if let Some(v) = num(&a) {
                            if a.trim().ends_with(['x', 'X']) {
                                look.height *= v;
                            } else if v > 0.0 {
                                look.height = v;
                            }
                        }
                    }
                    'W' => {
                        flush(&mut word, &look, &mut out);
                        let a = arg(&mut i);
                        if let Some(v) = num(&a) {
                            if a.trim().ends_with(['x', 'X']) {
                                look.width *= v;
                            } else if v > 0.0 {
                                look.width = v;
                            }
                        }
                    }
                    'Q' => {
                        flush(&mut word, &look, &mut out);
                        if let Some(v) = num(&arg(&mut i)) {
                            look.oblique = v * dmath::PI / 180.0;
                        }
                    }
                    'T' => {
                        flush(&mut word, &look, &mut out);
                        if let Some(v) = num(&arg(&mut i)) {
                            look.tracking = v;
                        }
                    }
                    'A' => {
                        let _ = arg(&mut i);
                    }
                    'C' => {
                        flush(&mut word, &look, &mut out);
                        if let Ok(v) = arg(&mut i).trim().parse::<i32>() {
                            look.color = Some(index_says(v.clamp(0, 257) as i16));
                        }
                    }
                    'c' => {
                        flush(&mut word, &look, &mut out);
                        if let Ok(v) = arg(&mut i).trim().parse::<u32>() {
                            // a Windows COLORREF: red in the low byte
                            look.color = Some(Says::Rgb([v as u8, (v >> 8) as u8, (v >> 16) as u8]));
                        }
                    }
                    'L' | 'l' => {
                        flush(&mut word, &look, &mut out);
                        look.under = code == 'L';
                    }
                    'O' | 'o' => {
                        flush(&mut word, &look, &mut out);
                        look.over = code == 'O';
                    }
                    'K' | 'k' => {
                        flush(&mut word, &look, &mut out);
                        look.strike = code == 'K';
                    }
                    'S' => {
                        flush(&mut word, &look, &mut out);
                        // up to an unescaped ';'
                        let mut a = String::new();
                        while i < ch.len() && ch[i] != ';' {
                            if ch[i] == '\\' && i + 1 < ch.len() {
                                a.push(ch[i + 1]);
                                i += 2;
                                continue;
                            }
                            a.push(ch[i]);
                            i += 1;
                        }
                        i += 1;
                        let k = a.find(['^', '/', '#']);
                        match k {
                            Some(k) => {
                                let kind = a[k..].chars().next().unwrap_or('/');
                                let top = text_plain(&a[..k]);
                                let bot = text_plain(&a[k + 1..]);
                                out.push(Tok::Stack(top, bot.trim_start().to_string(), kind, look.clone()));
                            }
                            None => word.push_str(&a),
                        }
                    }
                    'p' => {
                        // paragraph indents, tabs and alignment: only the alignment is used
                        let a = arg(&mut i);
                        if let Some(q) = a.find('q')
                            && let Some(al) = a[q + 1..].chars().next()
                        {
                            out.push(Tok::Align(al));
                        }
                    }
                    _ => {}
                }
            }
            '{' => {
                flush(&mut word, &look, &mut out);
                stack.push(look.clone());
                i += 1;
            }
            '}' => {
                flush(&mut word, &look, &mut out);
                if let Some(l) = stack.pop() {
                    look = l;
                }
                i += 1;
            }
            ' ' => {
                flush(&mut word, &look, &mut out);
                out.push(Tok::Space(look.clone(), false));
                i += 1;
            }
            '\t' => {
                flush(&mut word, &look, &mut out);
                out.push(Tok::Tab(look.clone()));
                i += 1;
            }
            '\n' => {
                flush(&mut word, &look, &mut out);
                out.push(Tok::Para);
                i += 1;
            }
            '^' if i + 1 < ch.len() => {
                match ch[i + 1] {
                    'I' => {
                        flush(&mut word, &look, &mut out);
                        out.push(Tok::Tab(look.clone()));
                    }
                    'J' | 'M' => {
                        flush(&mut word, &look, &mut out);
                        out.push(Tok::Para);
                    }
                    ' ' => word.push('^'),
                    x => {
                        word.push('^');
                        word.push(x);
                    }
                }
                i += 2;
            }
            '%' if ch.get(i + 1) == Some(&'%') && i + 2 < ch.len() => {
                let three: String = ch[i..i + 3].iter().collect();
                match text_codes(&three).first() {
                    Some((t, _, _)) if !t.is_empty() && t != &three => {
                        word.push_str(t);
                        i += 3;
                    }
                    _ => {
                        word.push('%');
                        i += 1;
                    }
                }
            }
            c => {
                word.push(c);
                i += 1;
            }
        }
    }
    flush(&mut word, &look, &mut out);
    out
}

/// MTEXT's text without its codes (for stacks and plain uses).
pub fn text_plain(s: &str) -> String {
    let base = Look {
        font: Font {
            file: String::new(),
            big: String::new(),
        },
        height: 1.0,
        width: 1.0,
        oblique: 0.0,
        tracking: 1.0,
        color: None,
        under: false,
        over: false,
        strike: false,
    };
    let mut out = String::new();
    for t in parse(s, &base) {
        match t {
            Tok::Word(w, _) => out.push_str(&w),
            Tok::Space(..) | Tok::Tab(_) => out.push(' '),
            Tok::Stack(a, b, k, _) => {
                out.push_str(&a);
                out.push(k);
                out.push_str(&b);
            }
            Tok::Para => out.push('\n'),
            Tok::Align(_) => {}
        }
    }
    out
}

/// How MTEXT is set out.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// Wrap width (0: no wrapping).
    pub width: f64,
    /// 1..9: top left .. bottom right.
    pub attachment: i16,
    /// 1 at least, 2 exactly.
    pub spacing_style: i16,
    pub spacing: f64,
}

struct Placed {
    x: f64,
    dy: f64,
    sh: Shaped,
    look: Look,
}

#[derive(Default)]
struct Line {
    items: Vec<Placed>,
    width: f64,
    height: f64,
    align: Option<char>,
}

fn shape_run(fonts: &dyn Fonts, text: &str, look: &Look) -> Shaped {
    if (look.tracking - 1.0).abs() < 1e-9 || text.chars().count() < 2 {
        return fonts.shape(&look.font.file, &look.font.big, text);
    }
    // tracking spreads the characters: set each on its own
    let mut out = Shaped::default();
    for c in text.chars() {
        let s = fonts.shape(&look.font.file, &look.font.big, &c.to_string());
        let x = out.advance;
        out.strokes.extend(
            s.strokes
                .iter()
                .map(|st| st.iter().map(|p| [p[0] + x, p[1]]).collect()),
        );
        out.fills.extend(
            s.fills
                .iter()
                .map(|r| r.iter().map(|p| [p[0] + x, p[1]]).collect()),
        );
        out.advance += s.advance * look.tracking;
    }
    out
}

/// MTEXT laid out from the insertion point at the origin.
pub fn mtext(fonts: &dyn Fonts, s: &str, base: &Look, fr: &Frame) -> Laid {
    let toks = parse(s, base);
    let wrap = if fr.width > 0.0 {
        fr.width * 1.0001
    } else {
        f64::INFINITY
    };
    let mut lines: Vec<Line> = vec![Line::default()];
    let mut para_align: Option<char> = None;
    let mut pending_space = 0.0;
    for t in toks {
        let line = lines.last_mut().unwrap_or_else(|| unreachable!());
        match t {
            Tok::Para => {
                lines.push(Line {
                    align: para_align,
                    ..Line::default()
                });
                pending_space = 0.0;
            }
            Tok::Align(a) => {
                para_align = Some(a);
                line.align = Some(a);
            }
            Tok::Space(look, _) | Tok::Tab(look) => {
                let sh = fonts.shape(&look.font.file, &look.font.big, " ");
                let w = sh.advance * look.height * look.width * look.tracking;
                let w = if w > 0.0 {
                    w
                } else {
                    look.height * look.width * 0.5
                };
                pending_space += w;
                line.height = line.height.max(look.height);
            }
            Tok::Word(text, look) => {
                let sh = shape_run(fonts, &text, &look);
                let w = sh.advance * look.height * look.width;
                if line.width + pending_space + w > wrap && !line.items.is_empty() {
                    let align = line.align;
                    lines.push(Line {
                        align,
                        ..Line::default()
                    });
                    pending_space = 0.0;
                }
                let line = lines.last_mut().unwrap_or_else(|| unreachable!());
                let x = line.width + pending_space;
                pending_space = 0.0;
                line.height = line.height.max(look.height);
                line.width = x + w;
                line.items.push(Placed { x, dy: 0.0, sh, look });
            }
            Tok::Stack(top, bot, kind, look) => {
                let small = Look {
                    height: look.height * 0.7,
                    under: false,
                    over: false,
                    strike: false,
                    ..look.clone()
                };
                let st = fonts.shape(&look.font.file, &look.font.big, &top);
                let sb = fonts.shape(&look.font.file, &look.font.big, &bot);
                let wt = st.advance * small.height * small.width;
                let wb = sb.advance * small.height * small.width;
                let w = if kind == '#' {
                    wt + wb + small.height * 0.5
                } else {
                    wt.max(wb)
                };
                if line.width + pending_space + w > wrap && !line.items.is_empty() {
                    let align = line.align;
                    lines.push(Line {
                        align,
                        ..Line::default()
                    });
                    pending_space = 0.0;
                }
                let line = lines.last_mut().unwrap_or_else(|| unreachable!());
                let x = line.width + pending_space;
                pending_space = 0.0;
                let h = look.height;
                line.height = line.height.max(h);
                line.width = x + w;
                match kind {
                    '#' => {
                        line.items.push(Placed {
                            x,
                            dy: 0.5 * h,
                            sh: st,
                            look: small.clone(),
                        });
                        let slash = Shaped {
                            strokes: vec![vec![[0.0, -0.2], [0.5 / 0.7, 1.4]]],
                            fills: vec![],
                            advance: 0.0,
                        };
                        line.items.push(Placed {
                            x: x + wt,
                            dy: 0.0,
                            sh: slash,
                            look: small.clone(),
                        });
                        line.items.push(Placed {
                            x: x + wt + h * 0.35,
                            dy: -0.1 * h,
                            sh: sb,
                            look: small,
                        });
                    }
                    _ => {
                        let (xt, xb) = if kind == '^' {
                            (x, x)
                        } else {
                            (x + (w - wt) / 2.0, x + (w - wb) / 2.0)
                        };
                        line.items.push(Placed {
                            x: xt,
                            dy: 0.55 * h,
                            sh: st,
                            look: small.clone(),
                        });
                        line.items.push(Placed {
                            x: xb,
                            dy: -0.45 * h,
                            sh: sb,
                            look: small.clone(),
                        });
                        if kind == '/' {
                            let bar = Shaped {
                                strokes: vec![vec![[0.0, 0.0], [w / (small.height * small.width), 0.0]]],
                                fills: vec![],
                                advance: 0.0,
                            };
                            line.items.push(Placed {
                                x,
                                dy: 0.4 * h,
                                sh: bar,
                                look: small,
                            });
                        }
                    }
                }
            }
        }
    }
    // lines without anything take the base height
    let base_h = base.height;
    let mut y = 0.0;
    let mut baselines = vec![];
    for (i, l) in lines.iter().enumerate() {
        let h = if l.height > 0.0 { l.height } else { base_h };
        if i == 0 {
            y = -h;
        } else {
            let pitch = if fr.spacing_style == 2 { base_h } else { h };
            y -= pitch * fr.spacing * 5.0 / 3.0;
        }
        baselines.push(y);
    }
    let last = lines
        .last()
        .map_or(base_h, |l| if l.height > 0.0 { l.height } else { base_h });
    let bottom = y - DESCENT * last;
    let max_w = lines.iter().map(|l| l.width).fold(0.0, f64::max);
    let box_w = if fr.width > 0.0 { fr.width } else { max_w };
    let att = fr.attachment.clamp(1, 9);
    let col = (att - 1) % 3;
    let row = (att - 1) / 3;
    let box_x = match col {
        0 => 0.0,
        1 => -box_w / 2.0,
        _ => -box_w,
    };
    let dy = match row {
        0 => 0.0,
        1 => -y / 2.0,
        _ => -bottom,
    };
    let mut out = Laid::default();
    for (l, by) in lines.iter().zip(&baselines) {
        let al = l
            .align
            .map(|a| a.to_ascii_lowercase())
            .unwrap_or(['l', 'c', 'r'][col as usize]);
        let lx = match al {
            'c' => box_x + (box_w - l.width) / 2.0,
            'r' => box_x + box_w - l.width,
            _ => box_x,
        };
        for p in &l.items {
            place(&p.sh, &p.look, lx + p.x, by + dy + p.dy, &mut out);
        }
    }
    out.lo = [box_x, bottom + dy];
    out.hi = [box_x + box_w, dy];
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::BoxFont;

    fn look() -> Look {
        Look {
            font: Font {
                file: "txt".into(),
                big: String::new(),
            },
            height: 2.0,
            width: 1.0,
            oblique: 0.0,
            tracking: 1.0,
            color: None,
            under: false,
            over: false,
            strike: false,
        }
    }

    fn words(t: &[Tok]) -> Vec<String> {
        t.iter()
            .filter_map(|t| match t {
                Tok::Word(w, _) => Some(w.clone()),
                Tok::Para => Some("¶".into()),
                Tok::Space(..) => Some("_".into()),
                Tok::Stack(a, b, k, _) => Some(format!("{a}{k}{b}")),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn codes() {
        let t = parse(r"{\fArial|b1|i0|c0|p34;Big}\Pnext\~line \\x\{\}", &look());
        assert_eq!(words(&t), ["Big", "¶", "next", "_", "line", "_", r"\x{}"]);
        match &t[0] {
            Tok::Word(_, l) => assert_eq!(l.font.file, "Arial"),
            _ => panic!(),
        }
        // the group ends: back to the base font
        match &t[2] {
            Tok::Word(_, l) => assert_eq!(l.font.file, "txt"),
            _ => panic!(),
        }
        let t = parse(r"\H3x;a\H1.5;b\W0.5;\Q15;c\C1;d\c16711680;e", &look());
        let looks: Vec<&Look> = t
            .iter()
            .filter_map(|t| match t {
                Tok::Word(_, l) => Some(l),
                _ => None,
            })
            .collect();
        assert_eq!(looks[0].height, 6.0);
        assert_eq!(looks[1].height, 1.5);
        assert_eq!(looks[2].width, 0.5);
        assert!((looks[2].oblique - 15.0 * dmath::PI / 180.0).abs() < 1e-12);
        assert_eq!(looks[3].color, Some(Says::Rgb([255, 0, 0])));
        assert_eq!(looks[4].color, Some(Says::Rgb([0, 0, 255])));
        let t = parse(r"1\S1^2;x\S3/4;%%d\U+00B1", &look());
        assert_eq!(words(&t), ["1", "1^2", "x", "3/4", "°±"]);
        let t = parse(r"\pxqc;a^Jb\Lu\l", &look());
        assert!(t.contains(&Tok::Align('c')));
        assert_eq!(words(&t), ["a", "¶", "b", "u"]);
        assert_eq!(text_plain(r"{\C1;red} \P%%c5"), "red \n\u{2205}5");
    }

    #[test]
    fn text_specials() {
        assert_eq!(
            text_codes("%%c10 %%d%%p1"),
            vec![("\u{2205}10 °±1".to_string(), false, false)]
        );
        let p = text_codes("a%%ub%%uc");
        assert_eq!(
            p,
            vec![
                ("a".into(), false, false),
                ("b".into(), true, false),
                ("c".into(), false, false)
            ]
        );
        assert_eq!(text_codes("%%065")[0].0, "A");
    }

    #[test]
    fn wrapping_and_attachment() {
        // BoxFont: 0.9 advance per character, height 2 => 1.8 a character
        let fr = Frame {
            width: 10.0,
            attachment: 1,
            spacing_style: 1,
            spacing: 1.0,
        };
        let l = mtext(&BoxFont, "aaa bbb ccc", &look(), &fr);
        // "aaa bbb" is 5.4 + 1.8 + 5.4 > 10: three lines
        let ys: std::collections::BTreeSet<i64> = l
            .strokes
            .iter()
            .map(|(_, s)| (s[0][1] * 1000.0).round() as i64)
            .collect();
        assert_eq!(ys.len(), 3);
        // the first line's caps reach the insertion point
        let top = l
            .strokes
            .iter()
            .flat_map(|(_, s)| s.iter().map(|p| p[1]))
            .fold(f64::MIN, f64::max);
        assert!((top - 0.0).abs() < 1e-9);
        // bottom centre: the box's bottom middle at the origin
        let fr = Frame {
            width: 0.0,
            attachment: 8,
            spacing_style: 1,
            spacing: 1.0,
        };
        let l = mtext(&BoxFont, "ab", &look(), &fr);
        assert!((l.lo[0] + l.hi[0]).abs() < 1e-9);
        assert!((l.lo[1]).abs() < 1e-9);
        let xs: Vec<f64> = l
            .strokes
            .iter()
            .flat_map(|(_, s)| s.iter().map(|p| p[0]))
            .collect();
        let (mn, mx) = (
            xs.iter().cloned().fold(f64::MAX, f64::min),
            xs.iter().cloned().fold(f64::MIN, f64::max),
        );
        assert!(
            (mn + 1.8).abs() < 1e-9 && (mx - 1.8 + 0.4).abs() < 1e-9,
            "{mn} {mx}"
        );
    }

    #[test]
    fn alignment() {
        assert_eq!(align_offset(0, 0, 10.0, 2.0), [0.0, 0.0]);
        assert_eq!(align_offset(1, 0, 10.0, 2.0), [-5.0, 0.0]);
        assert_eq!(align_offset(2, 3, 10.0, 2.0), [-10.0, -2.0]);
        assert_eq!(align_offset(4, 0, 10.0, 2.0), [-5.0, -1.0]);
        assert_eq!(align_offset(0, 1, 10.0, 3.0), [0.0, 1.0]);
    }
}

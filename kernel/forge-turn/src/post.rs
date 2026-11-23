//! Lathe posts: the ops to NC text through a dialect table (data/lathe-*.toml), numbers printed
//! from exact integer quantisation (forge_post::quant, digits), and a reader of our own output.
//!
//! The program states the imaginary tip (G40): the nose centre plus the tool's tip vector, the
//! point a lathe's tool offset is set to. X is a diameter (G18, diameter mode), so X is quantised
//! as 2ρ; an arc's I is a radius increment, so its centre's X is rounded to the start's parity
//! (I then is whole in the last digit). Arcs are ccw in (ρ, z) ⇔ G02: the G18 plane read from +Y,
//! Z to the right and X up, reverses the (ρ, z) sense.
//!
//! G6 for turning: `round_trip` reads the text back and compares it with `motions` of the
//! quantised program; `posted_moves` turns those motions back into reference-point moves, and
//! certify checks them (the path the posted text means, not the path planned).
use crate::check::Move;
use crate::ops::{OpPath, PMove, Spindle, Step};
use forge_post::{Point, Q, digits, quant};
use forge_tooling::toml::{self, Table, Val};

/// A lathe dialect: its templates and number formats.
#[derive(Clone, Debug, PartialEq)]
pub struct Dialect {
    pub name: String,
    pub extension: String,
    pub decimals: u32,
    pub feed_decimals: u32,
    pub pitch_decimals: u32,
    pub point: Point,
    t: Table,
}

pub const DIALECTS: [(&str, &str); 3] = [
    ("fanuc-lathe", include_str!("../data/lathe-fanuc.toml")),
    ("haas-lathe", include_str!("../data/lathe-haas.toml")),
    ("linuxcnc-lathe", include_str!("../data/lathe-linuxcnc.toml")),
];

#[derive(Clone, Debug, PartialEq)]
pub enum PostError {
    Dialect(String),
    Range(String),
}

impl Dialect {
    pub fn get(name: &str) -> Result<Dialect, PostError> {
        for (n, src) in DIALECTS {
            if n == name {
                return Dialect::parse(src);
            }
        }
        Err(PostError::Dialect(format!(
            "no lathe dialect `{name}`; there are {}",
            DIALECTS.iter().map(|d| d.0).collect::<Vec<_>>().join(", ")
        )))
    }

    pub fn parse(src: &str) -> Result<Dialect, PostError> {
        let root = toml::parse(src).map_err(|e| PostError::Dialect(format!("line {}: {}", e.line, e.what)))?;
        let t = match root.get("dialect") {
            Some(Val::Table(t)) => t.clone(),
            _ => return Err(PostError::Dialect("no [dialect] table".into())),
        };
        let s = |k: &str| toml::text(&t, k).map(|x| x.to_string()).ok_or(PostError::Dialect(format!("missing `{k}`")));
        let n = |k: &str| toml::num(&t, k).ok_or(PostError::Dialect(format!("missing `{k}`")));
        let point = match s("point")?.as_str() {
            "always" => Point::Always,
            "never" => Point::Never,
            "full" => Point::Full,
            x => return Err(PostError::Dialect(format!("point `{x}`"))),
        };
        Ok(Dialect {
            name: s("name")?,
            extension: s("extension")?,
            decimals: n("decimals")? as u32,
            feed_decimals: n("feed_decimals")? as u32,
            pitch_decimals: n("pitch_decimals")? as u32,
            point,
            t,
        })
    }

    fn s(&self, k: &str) -> &str {
        toml::text(&self.t, k).unwrap_or("")
    }

    fn lines(&self, k: &str) -> Vec<String> {
        toml::texts(&self.t, k).unwrap_or_default()
    }
}

fn fill(tpl: &str, vars: &[(&str, String)]) -> String {
    let mut s = tpl.to_string();
    for (k, v) in vars {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

/// A quantised program record. Positions are (X as a diameter, Z) in units of 10^-decimals.
#[derive(Clone, Debug, PartialEq)]
pub enum QRec {
    /// The start of op n (a comment with its name).
    Op(usize, String),
    Tool { t: u32, name: String },
    Css { vc: Q, max_rpm: Q },
    Rpm(Q),
    Rapid([Q; 2]),
    Feed([Q; 2], Q),
    /// The centre's X is a diameter too, of the start's parity.
    Arc { to: [Q; 2], c: [Q; 2], cw: bool, f: Q },
    Thread { to: [Q; 2], pitch: Q },
    Stop,
    Home,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QProgram {
    pub number: u32,
    pub title: String,
    pub recs: Vec<QRec>,
}

/// What the control does, in integer units.
#[derive(Clone, Debug, PartialEq)]
pub enum Motion {
    Op(usize),
    Tool(u32),
    Css(Q, Q),
    Rpm(Q),
    Rapid([Q; 2]),
    Feed([Q; 2], Q),
    Arc { to: [Q; 2], c: [Q; 2], cw: bool, f: Q },
    Thread { to: [Q; 2], pitch: Q },
    Stop,
    Home,
}

fn q(x: f64, k: u32) -> Result<Q, PostError> {
    quant(x, k).map_err(|e| PostError::Range(format!("{e:?}")))
}

/// The program for the ops: each op's tool change, spindle, and steps, as the imaginary tip.
pub fn quantize(ops: &[OpPath], number: u32, title: &str, d: &Dialect) -> Result<QProgram, PostError> {
    let k = d.decimals;
    let mut recs = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        recs.push(QRec::Op(i, op.name.clone()));
        recs.push(QRec::Tool {
            t: op.t,
            name: op.tool.id.clone(),
        });
        match op.spindle {
            Spindle::Css { vc, max_rpm } => recs.push(QRec::Css {
                vc: q(vc, 0)?,
                max_rpm: q(max_rpm, 0)?,
            }),
            Spindle::Rpm(r) => recs.push(QRec::Rpm(q(r, 0)?)),
        }
        let tip = op.tool.tip;
        let pos = |p: [f64; 2]| -> Result<[Q; 2], PostError> { Ok([q(2.0 * (p[0] + tip[0]), k)?, q(p[1] + tip[1], k)?]) };
        let f = q(op.f, d.feed_decimals)?;
        let mut at = pos(op.start)?;
        recs.push(QRec::Rapid(at));
        for st in &op.steps {
            let r = match *st {
                Step::Rapid(p) => QRec::Rapid(pos(p)?),
                Step::Feed(p) => QRec::Feed(pos(p)?, f),
                Step::Thread { to, pitch } => QRec::Thread {
                    to: pos(to)?,
                    pitch: q(pitch, d.pitch_decimals)?,
                },
                Step::Arc { to, c, ccw } => {
                    let to = pos(to)?;
                    let cz = q(c[1] + tip[1], k)?;
                    let mut cx = q(2.0 * (c[0] + tip[0]), k)?;
                    if (cx - at[0]).rem_euclid(2) != 0 {
                        // to the start's parity, the nearer way to the true centre
                        let exact = 2.0 * (c[0] + tip[0]) * 10f64.powi(k as i32);
                        cx += if (exact - cx as f64) >= 0.0 { 1 } else { -1 };
                    }
                    if to == at {
                        // a vanishing arc (its chord under the quantum): a control would read
                        // a whole turn; it is the point it rounds to
                        continue;
                    }
                    QRec::Arc {
                        to,
                        c: [cx, cz],
                        cw: ccw,
                        f,
                    }
                }
            };
            match &r {
                QRec::Rapid(p) | QRec::Feed(p, _) => {
                    if *p == at {
                        continue;
                    }
                    at = *p;
                }
                QRec::Arc { to, .. } | QRec::Thread { to, .. } => at = *to,
                _ => {}
            }
            recs.push(r);
        }
        recs.push(QRec::Stop);
    }
    recs.push(QRec::Home);
    Ok(QProgram {
        number,
        title: title.to_string(),
        recs,
    })
}

/// What the program makes the control do.
pub fn motions(p: &QProgram) -> Vec<Motion> {
    p.recs
        .iter()
        .map(|r| match r {
            QRec::Op(i, _) => Motion::Op(*i),
            QRec::Tool { t, .. } => Motion::Tool(*t),
            QRec::Css { vc, max_rpm } => Motion::Css(*vc, *max_rpm),
            QRec::Rpm(r) => Motion::Rpm(*r),
            QRec::Rapid(p) => Motion::Rapid(*p),
            QRec::Feed(p, f) => Motion::Feed(*p, *f),
            QRec::Arc { to, c, cw, f } => Motion::Arc {
                to: *to,
                c: *c,
                cw: *cw,
                f: *f,
            },
            QRec::Thread { to, pitch } => Motion::Thread { to: *to, pitch: *pitch },
            QRec::Stop => Motion::Stop,
            QRec::Home => Motion::Home,
        })
        .collect()
}

/// Writes the program in the dialect.
pub fn print(p: &QProgram, d: &Dialect) -> String {
    let k = d.decimals;
    let num = |v: Q| digits(v, k, d.point);
    let fnum = |v: Q, kk: u32| digits(v, kk, if d.point == Point::Full { Point::Never } else { d.point });
    let mut lines: Vec<String> = Vec::new();
    let numbered = toml::boolean(&d.t, "line_numbers") == Some(true);
    let step = toml::num(&d.t, "line_step").unwrap_or(10.0) as i64;
    let mut n = toml::num(&d.t, "line_start").unwrap_or(10.0) as i64;
    let mut emit = |s: String, lines: &mut Vec<String>| {
        if s.is_empty() {
            return;
        }
        if numbered && s != "%" {
            lines.push(format!("N{n} {s}"));
            n += step;
        } else {
            lines.push(s);
        }
    };
    let strip = d.s("comment_strip").to_string();
    let clean = |t: &str| -> String { t.chars().filter(|c| !strip.contains(*c) && *c != '\n').collect() };
    for l in d.lines("program_open") {
        emit(fill(&l, &[("num", format!("{:04}", p.number)), ("title", clean(&p.title))]), &mut lines);
    }
    for l in d.lines("safe_start") {
        emit(l, &mut lines);
    }
    let mut pos: [Option<Q>; 2] = [None, None];
    let mut feed: Option<Q> = None;
    let mut motion: Option<&str> = None;
    let thread_g = d.s("thread").to_string();
    let thread_word = d.s("thread_word").to_string();
    for r in &p.recs {
        match r {
            QRec::Op(i, name) => emit(fill(d.s("comment"), &[("text", format!("op {}: {}", i + 1, clean(name)))]), &mut lines),
            QRec::Tool { t, name } => {
                for l in d.lines("tool_change") {
                    emit(fill(&l, &[("tt", format!("{t:02}")), ("t", t.to_string()), ("name", clean(name))]), &mut lines);
                }
                pos = [None, None];
                motion = None;
            }
            QRec::Css { vc, max_rpm } => {
                for l in d.lines("css") {
                    emit(fill(&l, &[("vc", vc.to_string()), ("max", max_rpm.to_string())]), &mut lines);
                }
            }
            QRec::Rpm(r) => {
                for l in d.lines("rpm") {
                    emit(fill(&l, &[("rpm", r.to_string())]), &mut lines);
                }
            }
            QRec::Rapid(v) | QRec::Feed(v, _) | QRec::Thread { to: v, .. } | QRec::Arc { to: v, .. } => {
                let (g, force) = match r {
                    QRec::Rapid(_) => ("G00", false),
                    QRec::Feed(..) => ("G01", false),
                    QRec::Thread { .. } => (thread_g.as_str(), true),
                    QRec::Arc { cw, .. } => (if *cw { "G02" } else { "G03" }, true),
                    _ => unreachable!(),
                };
                let mut w: Vec<String> = Vec::new();
                if motion != Some(g) || force {
                    w.push(g.to_string());
                    motion = Some(g);
                }
                let start = pos;
                for (i, letter) in [(0, 'X'), (1, 'Z')] {
                    if force || pos[i] != Some(v[i]) {
                        w.push(format!("{letter}{}", num(v[i])));
                        pos[i] = Some(v[i]);
                    }
                }
                match r {
                    QRec::Arc { c, f, .. } => {
                        let (sx, sz) = (start[0].unwrap_or(0), start[1].unwrap_or(0));
                        w.push(format!("I{}", num((c[0] - sx) / 2)));
                        w.push(format!("K{}", num(c[1] - sz)));
                        if feed != Some(*f) {
                            w.push(format!("F{}", fnum(*f, d.feed_decimals)));
                            feed = Some(*f);
                        }
                    }
                    QRec::Feed(_, f) => {
                        if feed != Some(*f) {
                            w.push(format!("F{}", fnum(*f, d.feed_decimals)));
                            feed = Some(*f);
                        }
                    }
                    QRec::Thread { pitch, .. } => {
                        w.push(format!("{thread_word}{}", fnum(*pitch, d.pitch_decimals)));
                        // a control's F after a thread is the lead: the next feed restates its own
                        feed = None;
                    }
                    _ => {}
                }
                if w.len() > 1 || (w.len() == 1 && !w[0].starts_with('G')) {
                    emit(w.join(" "), &mut lines);
                }
            }
            QRec::Stop => emit(d.s("spindle_stop").to_string(), &mut lines),
            QRec::Home => {
                for l in d.lines("home") {
                    emit(l, &mut lines);
                }
            }
        }
    }
    for l in d.lines("program_close") {
        emit(l, &mut lines);
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReadError {
    pub line: usize,
    pub what: String,
}

/// Parses a number as the dialect writes it, to an integer at k decimals (exactly: digits only).
fn parse_q(s: &str, k: u32) -> Option<Q> {
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let (ip, fp) = match s.split_once('.') {
        Some((a, b)) => (a, b),
        None => (s, ""),
    };
    if fp.len() > k as usize || (ip.is_empty() && fp.is_empty()) {
        return None;
    }
    if !ip.chars().all(|c| c.is_ascii_digit()) || !fp.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut v: i64 = if ip.is_empty() { 0 } else { ip.parse().ok()? };
    let mut frac = fp.to_string();
    while frac.len() < k as usize {
        frac.push('0');
    }
    for _ in 0..k {
        v = v.checked_mul(10)?;
    }
    let f: i64 = if frac.is_empty() { 0 } else { frac.parse().ok()? };
    v += f;
    Some(if neg { -v } else { v })
}

/// Reads a program this post wrote back to the control's motions (op markers from its
/// comments). Anything it does not know is an error with its line: never a guess.
pub fn read(text: &str, d: &Dialect) -> Result<Vec<Motion>, ReadError> {
    let k = d.decimals;
    let err = |line: usize, what: &str| ReadError {
        line,
        what: what.to_string(),
    };
    let thread_g = d.s("thread").to_string();
    let thread_word = d.s("thread_word").chars().next().unwrap_or('F');
    let mut out = Vec::new();
    let mut pos: [Option<Q>; 2] = [None, None];
    let mut feed: Option<Q> = None;
    let mut motion: Option<String> = None;
    let mut ops = 0usize;
    let mut last_max: Option<Q> = None;
    for (ln, raw) in text.lines().enumerate() {
        let ln = ln + 1;
        let mut s = raw.trim().to_string();
        if s.is_empty() || s == "%" {
            continue;
        }
        // a comment: an op's start when it stands alone after the line number
        let mut comment = None;
        if let Some(i) = s.find('(') {
            let j = s[i..].find(')').map(|j| i + j).ok_or(err(ln, "an unclosed comment"))?;
            comment = Some(s[i + 1..j].to_string());
            s = format!("{}{}", &s[..i], &s[j + 1..]);
        }
        if let Some(i) = s.find(';') {
            comment = Some(s[i + 1..].trim().to_string());
            s.truncate(i);
        }
        let words: Vec<&str> = s.split_whitespace().collect();
        let words: Vec<&str> = words.into_iter().filter(|w| !w.starts_with('N')).collect();
        if words.is_empty() {
            if let Some(c) = &comment
                && is_op_comment(raw)
                && let Some(rest) = c.strip_prefix("op ")
                && let Some((n, _)) = rest.split_once(':')
            {
                let n: usize = n.trim().parse().map_err(|_| err(ln, "an op comment's number"))?;
                if n != ops + 1 {
                    return Err(err(ln, &format!("op {n} after op {ops}")));
                }
                out.push(Motion::Op(ops));
                ops += 1;
            }
            continue;
        }
        let mut x: Option<Q> = None;
        let mut z: Option<Q> = None;
        let mut i_: Option<Q> = None;
        let mut k_: Option<Q> = None;
        let mut f: Option<String> = None;
        let mut pitch: Option<Q> = None;
        let mut gm: Option<String> = None;
        let mut s_word: Option<Q> = None;
        let mut others: Vec<String> = Vec::new();
        for w in &words {
            let (l, v) = w.split_at(1);
            match l {
                "G" => match v {
                    "00" | "01" | "02" | "03" => gm = Some(format!("G{v}")),
                    _ if format!("G{v}") == thread_g => gm = Some(thread_g.clone()),
                    _ => others.push(w.to_string()),
                },
                "X" => x = Some(parse_q(v, k).ok_or(err(ln, &format!("X `{v}`")))?),
                "Z" => z = Some(parse_q(v, k).ok_or(err(ln, &format!("Z `{v}`")))?),
                "I" => i_ = Some(parse_q(v, k).ok_or(err(ln, &format!("I `{v}`")))?),
                "K" if thread_word != 'K' || gm.as_deref() != Some(thread_g.as_str()) => {
                    k_ = Some(parse_q(v, k).ok_or(err(ln, &format!("K `{v}`")))?)
                }
                "S" => s_word = Some(parse_q(v, 0).ok_or(err(ln, &format!("S `{v}`")))?),
                _ if l.chars().next() == Some(thread_word) && gm.as_deref() == Some(thread_g.as_str()) => {
                    pitch = Some(parse_q(v, d.pitch_decimals).ok_or(err(ln, &format!("pitch `{v}`")))?)
                }
                "F" => f = Some(v.to_string()),
                _ => others.push(w.to_string()),
            }
        }
        // spindle and tool words
        let line = words.join(" ");
        if let Some(t) = tool_of(&line, d) {
            out.push(Motion::Tool(t));
            pos = [None, None];
            motion = None;
            continue;
        }
        if others.iter().any(|w| w == "G50") {
            last_max = Some(s_word.ok_or(err(ln, "G50 without S"))?);
            continue;
        }
        if others.iter().any(|w| w == "G96") {
            let max = css_max(&line, d)
                .or(last_max)
                .ok_or(err(ln, "constant surface speed without its top speed"))?;
            out.push(Motion::Css(s_word.ok_or(err(ln, "G96 without S"))?, max));
            continue;
        }
        if others.iter().any(|w| w == "G97") {
            out.push(Motion::Rpm(s_word.ok_or(err(ln, "G97 without S"))?));
            continue;
        }
        if line == d.s("spindle_stop") {
            out.push(Motion::Stop);
            continue;
        }
        if gm.is_none() && x.is_none() && z.is_none() {
            // a setting or a home line
            if d.lines("home").iter().any(|h| h == &line) {
                if !matches!(out.last(), Some(Motion::Home)) {
                    out.push(Motion::Home);
                }
            }
            continue;
        }
        if let Some(g) = gm {
            motion = Some(g);
        }
        let g = motion.clone().ok_or(err(ln, "a move with no motion mode"))?;
        let start = pos;
        let to = [x.or(pos[0]).ok_or(err(ln, "no X yet"))?, z.or(pos[1]).ok_or(err(ln, "no Z yet"))?];
        if let Some(fv) = &f {
            feed = Some(parse_q(fv, d.feed_decimals).ok_or(err(ln, &format!("F `{fv}`")))?);
        }
        pos = [Some(to[0]), Some(to[1])];
        match g.as_str() {
            "G00" => out.push(Motion::Rapid(to)),
            "G01" => out.push(Motion::Feed(to, feed.ok_or(err(ln, "a feed with no F"))?)),
            "G02" | "G03" => {
                let (sx, sz) = (start[0].ok_or(err(ln, "an arc with no start"))?, start[1].ok_or(err(ln, "an arc with no start"))?);
                let c = [sx + 2 * i_.ok_or(err(ln, "an arc without I"))?, sz + k_.ok_or(err(ln, "an arc without K"))?];
                out.push(Motion::Arc {
                    to,
                    c,
                    cw: g == "G02",
                    f: feed.ok_or(err(ln, "an arc with no F"))?,
                });
            }
            _ if g == thread_g => {
                out.push(Motion::Thread {
                    to,
                    pitch: pitch.ok_or(err(ln, "a thread pass without its lead"))?,
                });
                feed = None;
            }
            _ => return Err(err(ln, &format!("motion `{g}`"))),
        }
    }
    Ok(out)
}

fn is_op_comment(raw: &str) -> bool {
    // an op's comment stands alone on its line (after a line number)
    let s = raw.trim();
    let s = match s.split_once(' ') {
        Some((n, r)) if n.starts_with('N') => r,
        _ => s,
    };
    (s.starts_with('(') && s.ends_with(')')) || s.starts_with(';')
}

fn tool_of(line: &str, d: &Dialect) -> Option<u32> {
    let tpl = d.lines("tool_change");
    let first = tpl.first()?;
    if first.contains("{tt}{tt}") {
        // T0101
        let w = line.split_whitespace().next()?;
        let v = w.strip_prefix('T')?;
        if v.len() == 4 && v.chars().all(|c| c.is_ascii_digit()) && v[..2] == v[2..] {
            return v[..2].parse().ok();
        }
        None
    } else {
        // T1 M06 ...
        let mut it = line.split_whitespace();
        let w = it.next()?;
        let v = w.strip_prefix('T')?;
        if it.next()?.starts_with("M6") || line.contains("M06") {
            return v.parse().ok();
        }
        None
    }
}

/// The top speed of CSS when the dialect states it on the G96 line (LinuxCNC's D word); else
/// it came on a G50 line before.
fn css_max(line: &str, d: &Dialect) -> Option<Q> {
    if d.s("css_max_word") != "D" {
        return None;
    }
    for w in line.split_whitespace() {
        if let Some(v) = w.strip_prefix('D') {
            return parse_q(v, 0);
        }
    }
    None
}

/// G6: the text, read back, is the program's motions.
pub fn round_trip(p: &QProgram, d: &Dialect) -> Result<(String, Vec<Motion>), String> {
    let text = print(p, d);
    let back = read(&text, d).map_err(|e| format!("line {}: {}", e.line, e.what))?;
    let want = motions(p);
    if back != want {
        let i = back.iter().zip(want.iter()).position(|(a, b)| a != b).unwrap_or(back.len().min(want.len()));
        return Err(format!(
            "the text does not read back as the program: at motion {i}, read {:?}, meant {:?}",
            back.get(i),
            want.get(i)
        ));
    }
    Ok((text, back))
}

/// The posted motions as reference-point moves per op, for certify: (move, cutting, rapid,
/// step); the step indices count the op's posted moves.
pub fn posted_moves(ms: &[Motion], ops: &[OpPath], d: &Dialect) -> Vec<Vec<PMove>> {
    let scale = 10f64.powi(d.decimals as i32);
    let mut out: Vec<Vec<PMove>> = vec![Vec::new(); ops.len()];
    let mut cur: Option<usize> = None;
    let mut at: Option<[f64; 2]> = None;
    for m in ms {
        if let Motion::Op(i) = m {
            cur = Some(*i);
            at = None;
            continue;
        }
        let Some(oi) = cur else { continue };
        let tip = ops[oi].tool.tip;
        let refp = |p: [Q; 2]| [p[0] as f64 / (2.0 * scale) - tip[0], p[1] as f64 / scale - tip[1]];
        let (to, mv, cut, rapid) = match m {
            Motion::Rapid(p) => {
                let b = refp(*p);
                (b, at.map(|a| Move::Line { a, b }), false, true)
            }
            Motion::Feed(p, _) => {
                let b = refp(*p);
                (b, at.map(|a| Move::Line { a, b }), true, false)
            }
            Motion::Thread { to, .. } => {
                let b = refp(*to);
                (b, at.map(|a| Move::Line { a, b }), true, false)
            }
            Motion::Arc { to, c, cw, .. } => {
                let b = refp(*to);
                let cc = refp(*c);
                (b, at.map(|a| Move::Arc { a, b, c: cc, ccw: *cw }), true, false)
            }
            _ => continue,
        };
        if let Some(mv) = mv {
            let n = out[oi].len();
            out[oi].push((mv, cut, rapid, n));
        }
        at = Some(to);
    }
    out
}

//! forge-post: toolpaths to NC text through a data dialect, and back.
//!
//! One fixed engine, no scripting. A dialect (data/*.toml) says which words and codes exist,
//! their templates, the digits, and for each canned cycle whether the control has it (a template),
//! or it is expanded into moves (`expand`), or refused (`refuse`, a typed error with a fix).
//!
//! Numbers are printed from exact integer quantisation: each coordinate is the nearest integer
//! multiple of 10^-decimals (ties to even), computed from the f64's exact binary value, and then
//! its digits are written. No float formatting is involved, so the text is the same bits on every
//! target.
//!
//! The round trip (G6 of research/cam/design.md): `quantize` gives the program in integer
//! units; `motions` is what a control does with it (cycles expanded by the one expansion
//! below); `print` writes it; `read` parses our own text back to motions. The obligation, tested
//! on every dialect: read(print(q)) = motions(q), exactly. The path the checks must hold for is
//! therefore motions(quantize(g)), the rounded path, never g itself.
//!
//! A post's options (`PostOptions`, `Dialect::with`) change the dialect itself:
//! - fewer decimals, never more than the control's increment;
//! - line numbers on or off (off refused where every block is numbered);
//! - no safe start, with the modes it would have set in `assumes_modes`;
//! - inches: the dialect's cited `units_inch` code in place of `units_mm` (`{units}` in its header
//!   templates), lengths and feeds quantised exactly from mm (x · 10^k · 10 / 254, ties to even)
//!   at its `inch_decimals` and `inch_feed_decimals`. Integer units are then 10^-k inch
//!   (`QProgram::mm_per_unit`), and the reader refuses the other unit's code.
//!
//! The round trip holds for the dialect they return.
use forge_cam_geom::big::Nat;
use forge_cam_geom::real::Dy;
use forge_tooling::toml::{self, Table, num, text, texts};

pub const DIALECTS: [(&str, &str); 11] = [
    ("fanuc", include_str!("../data/fanuc.toml")),
    ("haas", include_str!("../data/haas.toml")),
    ("linuxcnc", include_str!("../data/linuxcnc.toml")),
    ("grbl", include_str!("../data/grbl.toml")),
    ("siemens", include_str!("../data/siemens.toml")),
    ("heidenhain", include_str!("../data/heidenhain.toml")),
    ("pathpilot", include_str!("../data/pathpilot.toml")),
    ("mach3", include_str!("../data/mach3.toml")),
    ("mach4", include_str!("../data/mach4.toml")),
    ("centroid-cnc12", include_str!("../data/centroid-cnc12.toml")),
    ("masso", include_str!("../data/masso.toml")),
];

/// Controls whose dialect we cannot restate: no openly published programming manual gives the
/// codes a post needs (research/cam/posts/BLOCKED.md). Asking for one is a typed refusal with
/// the reason, never a guessed dialect.
pub const UNAVAILABLE: [(&str, &str); 5] = [
    (
        "brother",
        "no open programming manual for Brother CNC-C00/D00 (it ships with the machine); Brother's public FAQs give 19 of 31 codes but not spindle M3/M4/M5, arcs, dwell or the drilling cycles",
    ),
    (
        "mazak-eia",
        "Mazak's EIA/ISO programming manuals are behind the MPower customer login; no open source gives the codes",
    ),
    (
        "mazak-eia-lathe",
        "Mazak's EIA/ISO programming manuals are behind the MPower customer login; no open source gives the codes",
    ),
    (
        "syntec",
        "Syntec's programming manuals need a syntecclub.com login; no open source gives the codes",
    ),
    (
        "okuma-osp",
        "Okuma OSP programming manuals come through distributors or on the control; no open source gives the codes",
    ),
];

#[derive(Clone, Debug, PartialEq)]
pub enum PostError {
    Dialect(String),
    /// A real control whose dialect is not available, and why.
    Unavailable {
        name: String,
        reason: String,
    },
    UnknownDialect {
        name: String,
        known: Vec<String>,
    },
    /// The dialect cannot do this: what, and the fix.
    Unsupported {
        what: String,
        fix: String,
    },
    /// A number past what the dialect's words can hold.
    Range {
        what: String,
        value: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    Always,
    Never,
    Full,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dialect {
    pub name: String,
    pub extension: String,
    pub decimals: u32,
    pub feed_decimals: u32,
    pub point: Point,
    /// Where each code comes from (`[[dialect.cite]]`): a dialect restated from a control maker's
    /// manual cites every code it uses (`cite_required = true`).
    pub cites: Vec<Cite>,
    /// The modes this dialect's program assumes the control is in at its start, because the
    /// safe start that would set them is left out (`PostOptions::safe_start = Some(false)`): the
    /// safe start's lines. Empty when the program sets them itself.
    pub assumes_modes: Vec<String>,
    /// Lengths and feeds in inches (`PostOptions::inch`): `decimals` and `feed_decimals` are then
    /// the inch ones, and the program's integer units 10^-decimals inch.
    pub inch: bool,
    t: Table,
}

/// What a post may change of its dialect (`Dialect::with`). None keeps the dialect's own.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PostOptions {
    /// Inch: the dialect's cited inch code (`units_inch`) in place of its mm one, lengths and
    /// feeds in inches at its inch decimals (refused where no source gives the code).
    pub inch: bool,
    /// Coordinate digits after the point, at most the dialect's (its control's increment).
    pub decimals: Option<u32>,
    /// Feed digits after the point, at most the dialect's.
    pub feed_decimals: Option<u32>,
    /// N-words on or off (off refused where the control numbers every block).
    pub line_numbers: Option<bool>,
    /// The dialect's safe start written, or left out (then `assumes_modes` lists its lines).
    pub safe_start: Option<bool>,
}

/// One code's source: the dialect key (`dwell`, `cycle_peck`, ...), the document and where in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Cite {
    pub key: String,
    /// `published` (the manual states it) or `refused` (not in any open manual: the post refuses it)
    pub status: String,
    pub source: String,
    pub taken: String,
    pub r#where: String,
    pub note: String,
}

/// The template keys a dialect may set; with `cite_required`, each one it sets is cited.
pub const KEYS: [&str; 42] = [
    "program_open",
    "program_close",
    "comment",
    "line_numbers",
    "block_end",
    "safe_start",
    "tool_change",
    "length_comp",
    "spindle_cw",
    "spindle_ccw",
    "spindle_stop",
    "wcs",
    "arc",
    "retract_home",
    "coolant_flood",
    "coolant_mist",
    "coolant_through",
    "coolant_off",
    "dwell",
    "cycle_drill",
    "cycle_dwell",
    "cycle_peck",
    "cycle_chipbreak",
    "cycle_tap",
    "cycle_ream",
    "cycle_next",
    "cycle_cancel",
    "point",
    "decimals",
    "comment_eol",
    "read_g_ignore",
    "read_g_skip",
    "read_words",
    "coolant_through_off",
    "sign",
    "g_digits",
    "tool_word",
    "line_numbers_required",
    "units_mm",
    "units_inch",
    "inch_decimals",
    "inch_feed_decimals",
];

impl Dialect {
    pub fn get(name: &str) -> Result<Dialect, PostError> {
        for (n, s) in DIALECTS {
            if n == name {
                return Dialect::parse(s);
            }
        }
        if let Some((_, why)) = UNAVAILABLE.iter().find(|(n, _)| *n == name) {
            return Err(PostError::Unavailable {
                name: name.to_string(),
                reason: why.to_string(),
            });
        }
        Err(PostError::UnknownDialect {
            name: name.to_string(),
            known: DIALECTS.iter().map(|d| d.0.to_string()).collect(),
        })
    }

    pub fn parse(src: &str) -> Result<Dialect, PostError> {
        let root =
            toml::parse(src).map_err(|e| PostError::Dialect(format!("line {}: {}", e.line, e.what)))?;
        let t = match root.get("dialect") {
            Some(toml::Val::Table(t)) => t.clone(),
            _ => return Err(PostError::Dialect("no [dialect] table".into())),
        };
        let name = text(&t, "name")
            .ok_or_else(|| PostError::Dialect("no name".into()))?
            .to_string();
        let decimals = num(&t, "decimals").ok_or_else(|| PostError::Dialect("no decimals".into()))? as u32;
        let point = match text(&t, "point").unwrap_or("always") {
            "always" => Point::Always,
            "never" => Point::Never,
            "full" => Point::Full,
            p => return Err(PostError::Dialect(format!("point = `{p}`"))),
        };
        let mut cites = Vec::new();
        for c in toml::tables(&t, "cite") {
            let key = text(c, "key").ok_or_else(|| PostError::Dialect("a cite with no key".into()))?;
            if !KEYS.contains(&key) {
                return Err(PostError::Dialect(format!("cite `{key}` names no dialect key")));
            }
            let status = text(c, "status").unwrap_or("published");
            if !["published", "refused"].contains(&status) {
                return Err(PostError::Dialect(format!("cite `{key}`: status `{status}`")));
            }
            let source = text(c, "source").unwrap_or("");
            if status == "published" && (source.is_empty() || text(c, "taken").is_none()) {
                return Err(PostError::Dialect(format!(
                    "cite `{key}`: a published code needs source and taken"
                )));
            }
            cites.push(Cite {
                key: key.to_string(),
                status: status.to_string(),
                source: source.to_string(),
                taken: text(c, "taken").unwrap_or("").to_string(),
                r#where: text(c, "where").unwrap_or("").to_string(),
                note: text(c, "note").unwrap_or("").to_string(),
            });
        }
        if t.get("cite_required") == Some(&toml::Val::Bool(true)) {
            for k in KEYS {
                if t.contains_key(k) && !cites.iter().any(|c| c.key == k) {
                    return Err(PostError::Dialect(format!("`{k}` is set but not cited")));
                }
            }
        }
        Ok(Dialect {
            cites,
            assumes_modes: Vec::new(),
            inch: false,
            extension: text(&t, "extension").unwrap_or("nc").to_string(),
            feed_decimals: num(&t, "feed_decimals").unwrap_or(1.0) as u32,
            name,
            decimals,
            point,
            t,
        })
    }

    /// The dialect with a post's options: every change is one the dialect allows, else a typed
    /// refusal with its fix; the result is what `quantize`, `print`, `read` and `round_trip` take,
    /// so the certified text is the one written.
    pub fn with(&self, o: &PostOptions) -> Result<Dialect, PostError> {
        let mut d = self.clone();
        if o.inch && !self.inch {
            let code = self.s("units_inch");
            let refused = self
                .cites
                .iter()
                .find(|c| c.key == "units_inch" && c.status == "refused");
            if code.is_empty() || refused.is_some() || self.s("units_mm").is_empty() {
                let why = refused.map(|c| format!(": {}", c.note)).unwrap_or_default();
                return Err(PostError::Unsupported {
                    what: format!("inch output ({}{why})", self.name),
                    fix: "post in mm (units: \"mm\")".into(),
                });
            }
            d.inch = true;
            d.decimals = self
                .n("inch_decimals")
                .map(|x| x as u32)
                .unwrap_or(self.decimals + 1);
            d.feed_decimals = self
                .n("inch_feed_decimals")
                .map(|x| x as u32)
                .unwrap_or(self.feed_decimals + 1);
        }
        let (dk, fk) = (d.decimals, d.feed_decimals);
        if let Some(k) = o.decimals {
            if k > dk {
                return Err(PostError::Unsupported {
                    what: format!("{k} decimals: {} writes at most {dk}", self.name),
                    fix: format!("decimals: {dk} or fewer"),
                });
            }
            d.decimals = k;
        }
        if let Some(k) = o.feed_decimals {
            if k > fk {
                return Err(PostError::Unsupported {
                    what: format!("{k} feed decimals: {} writes at most {fk}", self.name),
                    fix: format!("feed_decimals: {fk} or fewer"),
                });
            }
            d.feed_decimals = k;
        }
        if let Some(on) = o.line_numbers {
            if !on && self.t.get("line_numbers_required") == Some(&toml::Val::Bool(true)) {
                return Err(PostError::Unsupported {
                    what: format!("no line numbers: {} numbers every block", self.name),
                    fix: "line_numbers: true".into(),
                });
            }
            d.t.insert("line_numbers".into(), toml::Val::Bool(on));
        }
        if o.safe_start == Some(false) {
            d.assumes_modes = self.lines("safe_start");
            d.t.remove("safe_start");
        }
        Ok(d)
    }

    fn s(&self, k: &str) -> &str {
        text(&self.t, k).unwrap_or("")
    }
    /// The code that sets the program's units (`{units}` in the header templates).
    fn units(&self) -> &str {
        if self.inch {
            self.s("units_inch")
        } else {
            self.s("units_mm")
        }
    }
    /// A template as the dialect sets it ("" if unset; "refuse" / "expand" for cycles).
    pub fn template(&self, k: &str) -> &str {
        self.s(k)
    }
    /// Whether the dialect writes a multi-line key (program_open, retract_home, ...).
    pub fn has_lines(&self, k: &str) -> bool {
        !self.lines(k).is_empty()
    }
    /// A coordinate word's digits: `sign = "always"` writes + on non-negative values (X+10).
    fn coord(&self, q: Q) -> String {
        let s = digits(q, self.decimals, self.point);
        if self.s("sign") == "always" && q >= 0 {
            format!("+{s}")
        } else {
            s
        }
    }
    /// A motion G word: `g_digits = 2` writes G00 .. G03.
    fn g(&self, g: u32) -> String {
        if self.n("g_digits") == Some(2.0) {
            format!("G{g:02}")
        } else {
            format!("G{g}")
        }
    }
    /// A template that may be one line or several (an array).
    fn tpl_lines(&self, k: &str) -> Vec<String> {
        match self.t.get(k) {
            Some(toml::Val::Str(x)) => vec![x.clone()],
            _ => self.lines(k),
        }
    }
    fn lines(&self, k: &str) -> Vec<String> {
        texts(&self.t, k).unwrap_or_default()
    }
    fn n(&self, k: &str) -> Option<f64> {
        num(&self.t, k)
    }
}

/// Canned hole cycles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CycleKind {
    Drill,
    Dwell,
    Peck,
    ChipBreak,
    Tap,
    Ream,
}

impl CycleKind {
    fn key(self) -> &'static str {
        match self {
            CycleKind::Drill => "cycle_drill",
            CycleKind::Dwell => "cycle_dwell",
            CycleKind::Peck => "cycle_peck",
            CycleKind::ChipBreak => "cycle_chipbreak",
            CycleKind::Tap => "cycle_tap",
            CycleKind::Ream => "cycle_ream",
        }
    }
    #[allow(dead_code)]
    fn gcode(self) -> u32 {
        match self {
            CycleKind::Drill => 81,
            CycleKind::Dwell => 82,
            CycleKind::Peck => 83,
            CycleKind::ChipBreak => 73,
            CycleKind::Tap => 84,
            CycleKind::Ream => 85,
        }
    }
    fn from_g(g: u32) -> Option<CycleKind> {
        Some(match g {
            81 => CycleKind::Drill,
            82 => CycleKind::Dwell,
            83 => CycleKind::Peck,
            73 => CycleKind::ChipBreak,
            84 => CycleKind::Tap,
            85 => CycleKind::Ream,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coolant {
    Flood,
    Mist,
    Through,
    Off,
}

/// A program record, in mm, mm/min, rpm, seconds.
#[derive(Clone, Debug, PartialEq)]
pub enum Rec {
    Comment(String),
    Wcs(u32),
    ToolChange {
        t: u32,
        h: u32,
        name: String,
    },
    Spindle {
        rpm: f64,
        cw: bool,
    },
    SpindleStop,
    Coolant(Coolant),
    Rapid([f64; 3]),
    Line([f64; 3], f64),
    /// To `to` about `c` (absolute), in the xy plane, clockwise or not; helical if z changes.
    Arc {
        to: [f64; 3],
        c: [f64; 2],
        cw: bool,
        f: f64,
    },
    /// Holes at (x, y): from the clearance height down to `z` from the plane `r`; q the peck,
    /// dwell seconds, f the feed (for a tap: pitch × rpm).
    Cycle {
        kind: CycleKind,
        holes: Vec<[f64; 2]>,
        z: f64,
        r: f64,
        clear: f64,
        q: f64,
        dwell: f64,
        f: f64,
    },
    Dwell(f64),
    Home,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub number: u32,
    pub title: String,
    pub recs: Vec<Rec>,
}

/// Integer units: coordinates in 10^-decimals mm, feeds in 10^-feed_decimals mm/min, rpm whole,
/// dwell in ms.
pub type Q = i64;

#[derive(Clone, Debug, PartialEq)]
pub enum QRec {
    Comment(String),
    Wcs(u32),
    ToolChange {
        t: u32,
        h: u32,
        name: String,
    },
    Spindle {
        rpm: Q,
        cw: bool,
    },
    SpindleStop,
    Coolant(Coolant),
    Rapid([Q; 3]),
    Line([Q; 3], Q),
    /// `full`: the arc is meant as a whole turn (its end's X and Y are its start's).
    Arc {
        to: [Q; 3],
        c: [Q; 2],
        cw: bool,
        f: Q,
        full: bool,
    },
    Cycle {
        kind: CycleKind,
        holes: Vec<[Q; 2]>,
        z: Q,
        r: Q,
        clear: Q,
        q: Q,
        dwell: Q,
        f: Q,
    },
    Dwell(Q),
    Home,
}

/// What the control does: the motion and the machine-state events, in integer units.
#[derive(Clone, Debug, PartialEq)]
pub enum Motion {
    Rapid([Q; 3]),
    Feed([Q; 3], Q),
    /// `full`: a whole turn (helical if Z moves). A control decides it from X and Y alone:
    /// an arc block whose end's X and Y are its start's is a full circle.
    Arc {
        to: [Q; 3],
        c: [Q; 2],
        cw: bool,
        f: Q,
        full: bool,
    },
    Dwell(Q),
    Tool(u32),
    Spindle(Q, bool),
    SpindleStop,
    Coolant(Coolant),
    Home,
}

/// The nearest integer to x · 10^k, ties to even, exactly.
pub fn quant(x: f64, k: u32) -> Result<Q, PostError> {
    if !x.is_finite() {
        return Err(PostError::Range {
            what: "a non-finite number".into(),
            value: x,
        });
    }
    let d = Dy::from_f64(x);
    let mut m = d.mag.clone();
    let mut i = 0;
    while i < k {
        m = m.mul_small(10);
        i += 1;
    }
    let n = if d.exp >= 0 {
        m.shl(d.exp as u64)
    } else {
        let s = (-d.exp) as u64;
        let fl = m.shr(s);
        // the remainder against a half: bit s−1 and the bits below it
        let half = m.bit(s - 1);
        let below = m.low_bits_nonzero(s - 1);
        let odd = fl.bit(0);
        if half && (below || odd) {
            fl.add(&Nat::from_u64(1))
        } else {
            fl
        }
    };
    if n.bits() > 62 {
        return Err(PostError::Range {
            what: "a coordinate".into(),
            value: x,
        });
    }
    let mut v2 = 0i64;
    let mut j = n.limbs.len();
    while j > 0 {
        j -= 1;
        v2 = (v2 << 32) | n.limbs[j] as i64;
    }
    Ok(if d.neg { -v2 } else { v2 })
}

/// The nearest integer to x · 10^k · num / den, ties to even, exactly (x in mm to inches:
/// num 10, den 254).
pub fn quant_ratio(x: f64, k: u32, num: u32, den: u32) -> Result<Q, PostError> {
    if !x.is_finite() {
        return Err(PostError::Range {
            what: "a non-finite number".into(),
            value: x,
        });
    }
    let d = Dy::from_f64(x);
    let mut m = d.mag.mul_small(num);
    let mut i = 0;
    while i < k {
        m = m.mul_small(10);
        i += 1;
    }
    // x · 10^k · num / den = m · 2^exp / den
    let (top, bot) = if d.exp >= 0 {
        (m.shl(d.exp as u64), Nat::from_u64(den as u64))
    } else {
        (m, Nat::from_u64(den as u64).shl((-d.exp) as u64))
    };
    let (q, r) = top.divrem(&bot);
    // round half to even: 2r against the divisor
    let twice = r.shl(1);
    let n = match twice.cmp(&bot) {
        core::cmp::Ordering::Greater => q.add(&Nat::from_u64(1)),
        core::cmp::Ordering::Equal if q.bit(0) => q.add(&Nat::from_u64(1)),
        _ => q,
    };
    if n.bits() > 62 {
        return Err(PostError::Range {
            what: "a coordinate".into(),
            value: x,
        });
    }
    let mut v2 = 0i64;
    let mut j = n.limbs.len();
    while j > 0 {
        j -= 1;
        v2 = (v2 << 32) | n.limbs[j] as i64;
    }
    Ok(if d.neg { -v2 } else { v2 })
}

/// A length or feed in the program's units: mm as they are, or in inches.
fn qlen(x: f64, k: u32, inch: bool) -> Result<Q, PostError> {
    if inch {
        quant_ratio(x, k, 10, 254)
    } else {
        quant(x, k)
    }
}

fn pow10(k: u32) -> i64 {
    let mut p = 1i64;
    let mut i = 0;
    while i < k {
        p *= 10;
        i += 1;
    }
    p
}

/// The digits of q · 10^-k as the dialect writes them.
pub fn digits(q: Q, k: u32, point: Point) -> String {
    let p = pow10(k);
    let a = q.unsigned_abs();
    let (ip, fp) = (a / p as u64, a % p as u64);
    let mut s = String::new();
    if q < 0 {
        s.push('-');
    }
    s.push_str(&ip.to_string());
    if k == 0 {
        if point == Point::Always {
            s.push('.');
        }
        return s;
    }
    let mut frac = format!("{:0width$}", fp, width = k as usize);
    if point != Point::Full {
        while frac.ends_with('0') {
            frac.pop();
        }
    }
    if frac.is_empty() {
        if point == Point::Always {
            s.push('.');
        }
    } else {
        s.push('.');
        s.push_str(&frac);
    }
    s
}

pub fn quantize(p: &Program, d: &Dialect) -> Result<QProgram, PostError> {
    let k = d.decimals;
    let fk = d.feed_decimals;
    let inch = d.inch;
    let q3 = |v: [f64; 3]| -> Result<[Q; 3], PostError> {
        Ok([qlen(v[0], k, inch)?, qlen(v[1], k, inch)?, qlen(v[2], k, inch)?])
    };
    let mut out = Vec::new();
    // where the tool is, as the program states it (an arc's start)
    let mut at: Option<[f64; 3]> = None;
    for r in &p.recs {
        let from = at;
        match r {
            Rec::Rapid(v) | Rec::Line(v, _) | Rec::Arc { to: v, .. } => at = Some(*v),
            Rec::Cycle { .. } => at = None,
            _ => {}
        }
        // a control reads an arc block whose end's X and Y are its start's as a full circle
        // (a full helix if Z moves). An arc meant as less than a whole turn whose end's X and Y
        // round onto its start's: under a half turn it is a vanishing arc (its chord under the
        // quantum), written as the line it rounds to (a G1, with its Z); over a half turn it is
        // a whole turn but for less than the quantum, and is one. An arc whose X and Y end where
        // they start is a whole turn as meant.
        let mut full = false;
        if let (Rec::Arc { to, c, cw, f }, Some(s)) = (r, from) {
            let xy_same = to[0] == s[0] && to[1] == s[1];
            let q_same = qlen(to[0], k, inch)? == qlen(s[0], k, inch)?
                && qlen(to[1], k, inch)? == qlen(s[1], k, inch)?;
            if xy_same {
                full = true;
            } else if q_same {
                // less than a half turn: the end is on the turning side of the start (or on its line)
                let cross = (s[0] - c[0]) * (to[1] - c[1]) - (s[1] - c[1]) * (to[0] - c[0]);
                let short = if *cw { cross <= 0.0 } else { cross >= 0.0 };
                if short {
                    out.push(QRec::Line(q3(*to)?, qlen(*f, fk, inch)?));
                    continue;
                }
                full = true;
            }
        }
        out.push(match r {
            Rec::Comment(s) => QRec::Comment(s.clone()),
            Rec::Wcs(n) => QRec::Wcs(*n),
            Rec::ToolChange { t, h, name } => QRec::ToolChange {
                t: *t,
                h: *h,
                name: name.clone(),
            },
            Rec::Spindle { rpm, cw } => QRec::Spindle {
                rpm: quant(*rpm, 0)?,
                cw: *cw,
            },
            Rec::SpindleStop => QRec::SpindleStop,
            Rec::Coolant(c) => QRec::Coolant(*c),
            Rec::Rapid(v) => QRec::Rapid(q3(*v)?),
            Rec::Line(v, f) => QRec::Line(q3(*v)?, qlen(*f, fk, inch)?),
            Rec::Arc { to, c, cw, f } => QRec::Arc {
                to: q3(*to)?,
                c: [qlen(c[0], k, inch)?, qlen(c[1], k, inch)?],
                cw: *cw,
                f: qlen(*f, fk, inch)?,
                full,
            },
            Rec::Cycle {
                kind,
                holes,
                z,
                r,
                clear,
                q,
                dwell,
                f,
            } => QRec::Cycle {
                kind: *kind,
                holes: holes
                    .iter()
                    .map(|h| Ok([qlen(h[0], k, inch)?, qlen(h[1], k, inch)?]))
                    .collect::<Result<_, PostError>>()?,
                z: qlen(*z, k, inch)?,
                r: qlen(*r, k, inch)?,
                clear: qlen(*clear, k, inch)?,
                q: qlen(*q, k, inch)?,
                dwell: quant(*dwell * 1000.0, 0)?,
                f: qlen(*f, fk, inch)?,
            },
            Rec::Dwell(s) => QRec::Dwell(quant(*s * 1000.0, 0)?),
            Rec::Home => QRec::Home,
        });
    }
    Ok(QProgram {
        number: p.number,
        title: p.title.clone(),
        recs: out,
        decimals: k,
        feed_decimals: fk,
        inch,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct QProgram {
    pub number: u32,
    pub title: String,
    pub recs: Vec<QRec>,
    pub decimals: u32,
    pub feed_decimals: u32,
    /// Integer units of 10^-decimals inch (feeds 10^-feed_decimals inch/min), not mm.
    pub inch: bool,
}

impl QProgram {
    /// One length unit in mm, exactly: (numerator, denominator).
    pub fn mm_per_unit(&self) -> (i64, i64) {
        if self.inch {
            (254, pow10(self.decimals) * 10)
        } else {
            (1, pow10(self.decimals))
        }
    }
}

/// The peck cycles' retract clearance and chip-break lift, mm (the controls' usual parameters,
/// fixed here so the expansion is one definition).
pub const PECK_CLEAR: f64 = 0.5;
pub const CHIP_LIFT: f64 = 0.2;

/// One hole of a cycle as motion, from (x, y) at the current height (the expansion every
/// dialect and the reader share). G98 semantics: return to the clearance height.
#[allow(clippy::too_many_arguments)]
fn expand_hole(
    kind: CycleKind,
    h: [Q; 2],
    z: Q,
    r: Q,
    clear: Q,
    q: Q,
    dwell: Q,
    f: Q,
    k: u32,
    inch: bool,
    params: (f64, f64),
    out: &mut Vec<Motion>,
) {
    let (x, y) = (h[0], h[1]);
    out.push(Motion::Rapid([x, y, clear]));
    out.push(Motion::Rapid([x, y, r]));
    match kind {
        CycleKind::Drill | CycleKind::Dwell | CycleKind::Ream | CycleKind::Tap => {
            out.push(Motion::Feed([x, y, z], f));
            if kind == CycleKind::Dwell && dwell > 0 {
                out.push(Motion::Dwell(dwell));
            }
            if kind == CycleKind::Ream || kind == CycleKind::Tap {
                out.push(Motion::Feed([x, y, r], f));
            }
        }
        CycleKind::Peck | CycleKind::ChipBreak => {
            let step = if q > 0 { q } else { r - z };
            let pc = qlen(params.0, k, inch).unwrap_or(0);
            let lift = qlen(params.1, k, inch).unwrap_or(0);
            let mut depth = r;
            while depth > z {
                let next = if depth - step > z { depth - step } else { z };
                out.push(Motion::Feed([x, y, next], f));
                depth = next;
                if depth > z {
                    if kind == CycleKind::Peck {
                        out.push(Motion::Rapid([x, y, r]));
                        out.push(Motion::Rapid([x, y, depth + pc]));
                    } else {
                        out.push(Motion::Rapid([x, y, depth + lift]));
                    }
                }
            }
        }
    }
    out.push(Motion::Rapid([x, y, clear]));
}

/// What the program does (cycles expanded), in integer units.
pub fn motions(p: &QProgram) -> Vec<Motion> {
    motions_with(p, (PECK_CLEAR, CHIP_LIFT))
}

/// `motions` for a control whose peck cycle retracts to `params.0` above the depth reached and
/// whose chip-break cycle lifts `params.1` (mm): each control sets these as a parameter.
pub fn motions_with(p: &QProgram, params: (f64, f64)) -> Vec<Motion> {
    let mut out = Vec::new();
    for r in &p.recs {
        match r {
            QRec::Comment(_) | QRec::Wcs(_) => {}
            QRec::ToolChange { t, .. } => out.push(Motion::Tool(*t)),
            QRec::Spindle { rpm, cw } => out.push(Motion::Spindle(*rpm, *cw)),
            QRec::SpindleStop => out.push(Motion::SpindleStop),
            QRec::Coolant(c) => out.push(Motion::Coolant(*c)),
            QRec::Rapid(v) => out.push(Motion::Rapid(*v)),
            QRec::Line(v, f) => out.push(Motion::Feed(*v, *f)),
            QRec::Arc { to, c, cw, f, full } => out.push(Motion::Arc {
                to: *to,
                c: *c,
                cw: *cw,
                f: *f,
                full: *full,
            }),
            QRec::Cycle {
                kind,
                holes,
                z,
                r,
                clear,
                q,
                dwell,
                f,
            } => {
                for h in holes {
                    expand_hole(
                        *kind, *h, *z, *r, *clear, *q, *dwell, *f, p.decimals, p.inch, params, &mut out,
                    );
                }
            }
            QRec::Dwell(ms) => out.push(Motion::Dwell(*ms)),
            QRec::Home => out.push(Motion::Home),
        }
    }
    out
}

/// Fills a template: `{name}` or `{name:04}` (zero-padded integer).
fn fill(tpl: &str, vars: &[(&str, String)]) -> String {
    let mut out = String::new();
    let b = tpl.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'{'
            && let Some(j) = tpl[i..].find('}')
        {
            let key = &tpl[i + 1..i + j];
            let (name, pad) = match key.split_once(':') {
                Some((n, p)) => (n, p.parse::<usize>().ok()),
                None => (key, None),
            };
            if let Some((_, v)) = vars.iter().find(|(n, _)| *n == name) {
                match pad {
                    Some(w) => out.push_str(&format!("{:0>w$}", v, w = w % 100)),
                    None => out.push_str(v),
                }
            }
            i += j + 1;
            continue;
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}

struct Writer<'a> {
    d: &'a Dialect,
    lines: Vec<String>,
    n: i64,
}

impl Writer<'_> {
    fn emit(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let numbered = self.d.t.get("line_numbers") == Some(&toml::Val::Bool(true)) && s != "%";
        // a comment runs to the end of its line: no terminator after it
        let eol = self.d.s("comment_eol");
        let end = if !eol.is_empty() && s.contains(eol) {
            ""
        } else {
            self.d.s("block_end")
        };
        if numbered {
            self.lines.push(format!("N{} {}{end}", self.n, s));
            self.n += self.d.n("line_step").unwrap_or(10.0) as i64;
        } else {
            self.lines.push(format!("{s}{end}"));
        }
    }
    fn comment(&mut self, text: &str) {
        let strip = self.d.s("comment_strip");
        let clean: String = text
            .chars()
            .filter(|c| !strip.contains(*c) && *c != '\n')
            .collect();
        let tpl = self.d.s("comment").to_string();
        self.emit(&fill(&tpl, &[("text", clean)]));
    }
}

/// Writes the program in the dialect.
pub fn print(p: &QProgram, d: &Dialect) -> Result<String, PostError> {
    let k = d.decimals;
    let num_ = |q: Q| d.coord(q);
    let fnum = |q: Q| {
        digits(
            q,
            d.feed_decimals,
            if d.point == Point::Full {
                Point::Never
            } else {
                d.point
            },
        )
    };
    let mut w = Writer {
        d,
        lines: Vec::new(),
        n: d.n("line_start").unwrap_or(10.0) as i64,
    };
    let title: String = p
        .title
        .chars()
        .filter(|c| !d.s("comment_strip").contains(*c))
        .collect();
    let units = d.units().to_string();
    for l in d.lines("program_open") {
        let s = fill(
            &l,
            &[
                ("num", p.number.to_string()),
                ("title", title.clone()),
                ("units", units.clone()),
            ],
        );
        // a `%` line (a tape mark, or a header such as `%NAME G71 *`) is written as it is
        if s.contains('%') {
            w.lines.push(s)
        } else {
            w.emit(&s)
        }
    }
    for l in d.lines("safe_start") {
        w.emit(&fill(&l, &[("units", units.clone())]));
    }
    // modal state
    let mut pos: [Option<Q>; 3] = [None; 3];
    let mut feed: Option<Q> = None;
    let mut motion: Option<u32> = None;
    let word = |w: &mut Vec<String>, letter: char, v: Q, slot: &mut Option<Q>, force: bool| {
        if force || *slot != Some(v) {
            w.push(format!("{letter}{}", num_(v)));
            *slot = Some(v);
        }
    };
    for r in &p.recs {
        match r {
            QRec::Comment(s) => w.comment(s),
            QRec::Wcs(n) => {
                let first = d.n("wcs_first").unwrap_or(54.0) as u32;
                let count = d.n("wcs_count").unwrap_or(6.0) as u32;
                if *n >= count {
                    return Err(PostError::Unsupported {
                        what: format!("work offset {n} (the dialect has {count})"),
                        fix: "use a lower work offset".into(),
                    });
                }
                w.emit(&fill(d.s("wcs"), &[("n", (first + n).to_string())]));
            }
            QRec::ToolChange { t, h, name } => {
                for l in d.lines("tool_change") {
                    w.emit(&fill(
                        &l,
                        &[("t", t.to_string()), ("h", h.to_string()), ("name", name.clone())],
                    ));
                }
                let lc = d.s("length_comp").to_string();
                if !lc.is_empty() {
                    // the length offset goes with the first move after the change: here alone
                    w.emit(&fill(&lc, &[("h", h.to_string())]));
                }
                pos = [None; 3];
                motion = None;
            }
            QRec::Spindle { rpm, cw } => {
                let key = if *cw { "spindle_cw" } else { "spindle_ccw" };
                for l in d.tpl_lines(key) {
                    w.emit(&fill(&l, &[("rpm", rpm.to_string())]));
                }
            }
            QRec::SpindleStop => w.emit(d.s("spindle_stop")),
            QRec::Coolant(c) => {
                let key = match c {
                    Coolant::Flood => "coolant_flood",
                    Coolant::Mist => "coolant_mist",
                    Coolant::Through => "coolant_through",
                    Coolant::Off => "coolant_off",
                };
                let code = d.s(key);
                if code.is_empty() {
                    return Err(PostError::Unsupported {
                        what: format!("coolant {c:?}"),
                        fix: "choose a coolant the machine has (flood, mist) or none".into(),
                    });
                }
                w.emit(code);
            }
            QRec::Rapid(v) | QRec::Line(v, _) => {
                // a null move (no axis changes) is not written, and changes no modal state: the
                // G and F of a block are committed only with the block (a dropped null move must
                // not leave the mode it would have set)
                let g = if matches!(r, QRec::Rapid(_)) { 0 } else { 1 };
                let mut axes = Vec::new();
                for (i, letter) in ['X', 'Y', 'Z'].into_iter().enumerate() {
                    word(&mut axes, letter, v[i], &mut pos[i], false);
                }
                if !axes.is_empty() {
                    let mut ws = Vec::new();
                    if motion != Some(g) {
                        ws.push(d.g(g));
                        motion = Some(g);
                    }
                    ws.extend(axes);
                    if let QRec::Line(_, f) = r
                        && feed != Some(*f)
                    {
                        ws.push(format!("F{}", fnum(*f)));
                        feed = Some(*f);
                    }
                    w.emit(&ws.join(" "));
                }
            }
            QRec::Arc { to, c, cw, f, .. } => {
                let (sx, sy) = match (pos[0], pos[1]) {
                    (Some(x), Some(y)) => (x, y),
                    _ => {
                        return Err(PostError::Unsupported {
                            what: "an arc from an unknown position".into(),
                            fix: "move to the arc's start first".into(),
                        });
                    }
                };
                let g = if *cw { 2 } else { 3 };
                let centre_first = d.s("arc") == "centre_abs";
                if centre_first {
                    // the centre as its own block, absolute (HEIDENHAIN ISO: I J, then G02/G03)
                    w.emit(&format!("I{} J{}", num_(c[0]), num_(c[1])));
                }
                let mut ws = vec![d.g(g)];
                motion = Some(g);
                // an arc always states its end, so a full circle is unambiguous
                word(&mut ws, 'X', to[0], &mut pos[0], true);
                word(&mut ws, 'Y', to[1], &mut pos[1], true);
                word(&mut ws, 'Z', to[2], &mut pos[2], false);
                if !centre_first {
                    ws.push(format!("I{}", num_(c[0] - sx)));
                    ws.push(format!("J{}", num_(c[1] - sy)));
                }
                if feed != Some(*f) {
                    ws.push(format!("F{}", fnum(*f)));
                    feed = Some(*f);
                }
                w.emit(&ws.join(" "));
            }
            QRec::Cycle {
                kind,
                holes,
                z,
                r,
                clear,
                q,
                dwell,
                f,
            } => {
                let tpl = d.s(kind.key()).to_string();
                if tpl == "refuse" || tpl.is_empty() {
                    return Err(PostError::Unsupported {
                        what: format!("the {kind:?} cycle on {}", d.name),
                        fix: if *kind == CycleKind::Tap {
                            "thread mill the hole instead, or post for a control with rigid tapping".into()
                        } else {
                            "use a dialect with the cycle".into()
                        },
                    });
                }
                if tpl == "expand" {
                    let mut ms = Vec::new();
                    for h in holes {
                        expand_hole(
                            *kind,
                            *h,
                            *z,
                            *r,
                            *clear,
                            *q,
                            *dwell,
                            *f,
                            k,
                            d.inch,
                            (PECK_CLEAR, CHIP_LIFT),
                            &mut ms,
                        );
                    }
                    for m in ms {
                        let rr = match m {
                            Motion::Rapid(v) => QRec::Rapid(v),
                            Motion::Feed(v, f) => QRec::Line(v, f),
                            Motion::Dwell(ms) => QRec::Dwell(ms),
                            _ => continue,
                        };
                        // recursion on one record: write it as the main loop would
                        let one = QProgram {
                            number: 0,
                            title: String::new(),
                            recs: vec![rr],
                            decimals: k,
                            feed_decimals: d.feed_decimals,
                            inch: d.inch,
                        };
                        write_moves(&one, d, &mut w, &mut pos, &mut feed, &mut motion)?;
                    }
                    continue;
                }
                // the program has put the tool over the first hole at the clearance height
                let first = holes[0];
                if pos != [Some(first[0]), Some(first[1]), Some(*clear)] {
                    return Err(PostError::Unsupported {
                        what: "a cycle not started over its first hole at the clearance height".into(),
                        fix: "rapid to the first hole at the clearance height before the cycle".into(),
                    });
                }
                let vars = |h: [Q; 2]| {
                    vec![
                        ("x", num_(h[0])),
                        ("y", num_(h[1])),
                        ("z", num_(*z)),
                        ("r", num_(*r)),
                        ("q", num_(*q)),
                        ("f", fnum(*f)),
                        ("ms", dwell.to_string()),
                        ("s", digits(*dwell, 3, Point::Never)),
                    ]
                };
                w.emit(&fill(&tpl, &vars(first)));
                for h in &holes[1..] {
                    w.emit(&fill(d.s("cycle_next"), &vars(*h)));
                }
                w.emit(d.s("cycle_cancel"));
                let last = holes[holes.len() - 1];
                pos = [Some(last[0]), Some(last[1]), Some(*clear)];
                feed = None;
                motion = None;
            }
            QRec::Dwell(ms) => {
                let tpl = d.s("dwell").to_string();
                if tpl.is_empty() {
                    return Err(PostError::Unsupported {
                        what: format!("a dwell on {} (no open source gives its units)", d.name),
                        fix: "leave the dwell out, or post for a control whose dwell is known".into(),
                    });
                }
                w.emit(&fill(
                    &tpl,
                    &[("ms", ms.to_string()), ("s", digits(*ms, 3, Point::Never))],
                ));
            }
            QRec::Home => {
                if d.lines("retract_home").is_empty() {
                    return Err(PostError::Unsupported {
                        what: format!("a machine-home retract on {}", d.name),
                        fix: "end with a rapid to the clearance height instead".into(),
                    });
                }
                for l in d.lines("retract_home") {
                    w.emit(&l);
                }
                pos = [None; 3];
                motion = None;
            }
        }
    }
    for l in d.lines("program_close") {
        let l = fill(
            &l,
            &[
                ("num", p.number.to_string()),
                ("title", title.clone()),
                ("units", units.clone()),
            ],
        );
        if l.contains('%') {
            w.lines.push(l)
        } else {
            w.emit(&l)
        }
    }
    let mut s = w.lines.join("\n");
    s.push('\n');
    Ok(s)
}

fn write_moves(
    p: &QProgram,
    d: &Dialect,
    w: &mut Writer,
    pos: &mut [Option<Q>; 3],
    feed: &mut Option<Q>,
    motion: &mut Option<u32>,
) -> Result<(), PostError> {
    let num_ = |q: Q| d.coord(q);
    let fnum = |q: Q| {
        digits(
            q,
            d.feed_decimals,
            if d.point == Point::Full {
                Point::Never
            } else {
                d.point
            },
        )
    };
    for r in &p.recs {
        match r {
            QRec::Rapid(v) | QRec::Line(v, _) => {
                let g = if matches!(r, QRec::Rapid(_)) { 0 } else { 1 };
                let mut axes = Vec::new();
                for (i, letter) in ['X', 'Y', 'Z'].into_iter().enumerate() {
                    if pos[i] != Some(v[i]) {
                        axes.push(format!("{letter}{}", num_(v[i])));
                        pos[i] = Some(v[i]);
                    }
                }
                // as in print: modal state only moves with a written block
                if !axes.is_empty() {
                    let mut ws = Vec::new();
                    if *motion != Some(g) {
                        ws.push(d.g(g));
                        *motion = Some(g);
                    }
                    ws.extend(axes);
                    if let QRec::Line(_, f) = r
                        && *feed != Some(*f)
                    {
                        ws.push(format!("F{}", fnum(*f)));
                        *feed = Some(*f);
                    }
                    w.emit(&ws.join(" "));
                }
            }
            QRec::Dwell(ms) => {
                let tpl = d.s("dwell").to_string();
                if tpl.is_empty() {
                    return Err(PostError::Unsupported {
                        what: format!("a dwell on {} (no open source gives its units)", d.name),
                        fix: "leave the dwell out, or post for a control whose dwell is known".into(),
                    });
                }
                w.emit(&fill(
                    &tpl,
                    &[("ms", ms.to_string()), ("s", digits(*ms, 3, Point::Never))],
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReadError {
    pub line: usize,
    pub what: String,
}

/// A decimal word's value in integer units of 10^-k, exactly; more digits than k is an error.
fn parse_q(s: &str, k: u32, line: usize) -> Result<Q, ReadError> {
    let e = |w: &str| ReadError {
        line,
        what: format!("{w}: `{s}`"),
    };
    let (neg, body) = match s.strip_prefix('-') {
        Some(b) => (true, b),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (ip, fp) = match body.split_once('.') {
        Some((a, b)) => (a, b),
        None => (body, ""),
    };
    if fp.len() > k as usize
        || (ip.is_empty() && fp.is_empty())
        || !ip.chars().all(|c| c.is_ascii_digit())
        || !fp.chars().all(|c| c.is_ascii_digit())
    {
        return Err(e("not a number in this dialect's digits"));
    }
    let mut v: i64 = if ip.is_empty() {
        0
    } else {
        ip.parse().map_err(|_| e("too large"))?
    };
    v = v.checked_mul(pow10(k)).ok_or_else(|| e("too large"))?;
    let mut f = 0i64;
    if !fp.is_empty() {
        f = fp.parse::<i64>().map_err(|_| e("bad fraction"))? * pow10(k - fp.len() as u32);
    }
    let r = v + f;
    Ok(if neg { -r } else { r })
}

/// Reads our own output back into motion (the dialect's subset; anything else is an error).
pub fn read(text_: &str, d: &Dialect) -> Result<Vec<Motion>, ReadError> {
    let k = d.decimals;
    let fk = d.feed_decimals;
    let mut out = Vec::new();
    let mut pos: [Option<Q>; 3] = [None; 3];
    let mut feed: Q = 0;
    let mut motion: Option<u32> = None;
    let mut cycle: Option<(CycleKind, Q, Q, Q, Q, Q)> = None; // kind, z, r, q, dwell ms, clear
    let mut rpm: Q = 0;
    // the dialect's extras: an end-of-line comment mark, a block terminator, G codes that carry
    // no motion (ignored) or that make the block carry none (skipped), more word letters
    let eol = d.s("comment_eol").chars().next();
    let block_end = d.s("block_end").trim().to_string();
    let g_ignore_f: Vec<f64> = toml::nums(&d.t, "read_g_ignore").unwrap_or_default();
    let g_ignore: Vec<u32> = g_ignore_f
        .iter()
        .filter(|x| x.fract() == 0.0)
        .map(|x| *x as u32)
        .collect();
    let g_skip: Vec<u32> = toml::nums(&d.t, "read_g_skip")
        .unwrap_or_default()
        .into_iter()
        .map(|x| x as u32)
        .collect();
    let more_words = d.s("read_words").to_string();
    let centre_first = d.s("arc") == "centre_abs";
    let tool_word = d.t.get("tool_word") == Some(&toml::Val::Bool(true));
    let mut centre: Option<(Q, Q)> = None;
    // the dwell's word: the letter before the dwell template's `{ms}` or `{s}`
    let word_of = |tpl: &str| -> char {
        tpl.find("{ms}")
            .or_else(|| tpl.find("{s}"))
            .and_then(|i| tpl[..i].chars().last())
            .filter(|c| c.is_ascii_alphabetic())
            .unwrap_or('P')
    };
    let dwell_letter = word_of(d.s("dwell"));
    let cycle_dwell_letter = word_of(d.s("cycle_dwell"));
    for (n, raw) in text_.lines().enumerate() {
        let ln = n + 1;
        let raw = match eol.and_then(|c| raw.find(c)) {
            Some(i) => &raw[..i],
            None => raw,
        };
        let raw = if block_end.is_empty() {
            raw
        } else {
            raw.trim_end().strip_suffix(block_end.as_str()).unwrap_or(raw)
        };
        // comments and the program brackets
        let mut s = String::new();
        let mut depth = 0;
        for ch in raw.chars() {
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ if depth == 0 => s.push(ch),
                _ => {}
            }
        }
        let s = s.trim();
        // a tape mark or a `%` header/last block (`%NAME G71 *`, `N99999999 %NAME G71 *`); any
        // other `%` is refused as an unknown word
        if s.is_empty() || pct_block(s) {
            continue;
        }
        let mut words: Vec<(char, String)> = Vec::new();
        for w in s.split_whitespace() {
            let mut cs = w.chars();
            let l = cs.next().unwrap().to_ascii_uppercase();
            words.push((l, cs.as_str().to_string()));
        }
        let get = |c: char| words.iter().find(|(l, _)| *l == c).map(|(_, v)| v.clone());
        let mut gs: Vec<u32> = Vec::new();
        let mut ms: Vec<u32> = Vec::new();
        for (l, v) in &words {
            match l {
                'G' => {
                    // G43.4 and the like are not ours: only whole G codes
                    let g: f64 = v.parse().map_err(|_| ReadError {
                        line: ln,
                        what: format!("G{v}"),
                    })?;
                    if g.fract() == 0.0 {
                        gs.push(g as u32)
                    } else if g_ignore_f.contains(&g) {
                        // a decimal G the dialect lists as carrying no motion (G91.1)
                    } else {
                        gs.push(10_000)
                    }
                }
                'M' => ms.push(v.parse().map_err(|_| ReadError {
                    line: ln,
                    what: format!("M{v}"),
                })?),
                'N' | 'O' | 'X' | 'Y' | 'Z' | 'I' | 'J' | 'F' | 'R' | 'Q' | 'P' | 'S' | 'T' | 'H' | 'K' => {}
                c if more_words.contains(*c) => {}
                c => {
                    return Err(ReadError {
                        line: ln,
                        what: format!("an unknown word `{c}{v}`"),
                    });
                }
            }
        }
        if let Some(sv) = get('S') {
            rpm = parse_q(&sv, 0, ln)?;
        }
        // a dwell written with F (G4 F seconds) is not a feed
        if let Some(fv) = get('F')
            && !(dwell_letter == 'F' && gs.contains(&4))
        {
            feed = parse_q(&fv, fk, ln)?;
        }
        for m in &ms {
            match m {
                3 | 4 => out.push(Motion::Spindle(rpm, *m == 3)),
                5 => out.push(Motion::SpindleStop),
                6 => out.push(Motion::Tool(get('T').and_then(|t| t.parse().ok()).unwrap_or(0))),
                7 => out.push(Motion::Coolant(Coolant::Mist)),
                8 => out.push(Motion::Coolant(Coolant::Flood)),
                88 => out.push(Motion::Coolant(Coolant::Through)),
                9 | 89 => out.push(Motion::Coolant(Coolant::Off)),
                0 | 2 | 30 => {}
                m => {
                    return Err(ReadError {
                        line: ln,
                        what: format!("M{m}"),
                    });
                }
            }
        }
        // a control whose T block is the tool change (no M6): T with a number; a bare T changes
        // only the speed
        if tool_word
            && !ms.contains(&6)
            && let Some(t) = get('T')
            && let Ok(t) = t.parse::<u32>()
        {
            out.push(Motion::Tool(t));
        }
        // grbl's tool change is a pause with the tool in a comment: the comment named it
        if ms.contains(&0) && !ms.contains(&6) {
            let t = raw_tool(text_, ln);
            out.push(Motion::Tool(t));
        }
        let mut home = false;
        let mut skip_motion = false;
        for g in &gs {
            match g {
                0..=3 => {
                    motion = Some(*g);
                    cycle = None;
                }
                81 | 82 | 83 | 73 | 84 | 85 => {
                    let kind = CycleKind::from_g(*g).unwrap();
                    let z = parse_q(
                        &get('Z').ok_or_else(|| ReadError {
                            line: ln,
                            what: "a cycle with no Z".into(),
                        })?,
                        k,
                        ln,
                    )?;
                    let r = parse_q(
                        &get('R').ok_or_else(|| ReadError {
                            line: ln,
                            what: "a cycle with no R".into(),
                        })?,
                        k,
                        ln,
                    )?;
                    let q = match get('Q') {
                        Some(v) => parse_q(&v, k, ln)?,
                        None => 0,
                    };
                    let dw = match get(cycle_dwell_letter) {
                        Some(v) => parse_q(&v, if d.s("cycle_dwell").contains("{ms}") { 0 } else { 3 }, ln)?,
                        None => 0,
                    };
                    let clear = pos[2].ok_or_else(|| ReadError {
                        line: ln,
                        what: "a cycle from an unknown height".into(),
                    })?;
                    cycle = Some((kind, z, r, q, dw, clear));
                    motion = Some(80);
                }
                80 => {
                    cycle = None;
                    motion = None;
                }
                4 => {
                    let p = get(dwell_letter).ok_or_else(|| ReadError {
                        line: ln,
                        what: format!("G4 with no {dwell_letter}"),
                    })?;
                    // milliseconds (integer P) or seconds (with a point), as the dialect writes it
                    let ms = if d.s("dwell").contains("{ms}") {
                        parse_q(&p, 0, ln)?
                    } else {
                        parse_q(&p, 3, ln)?
                    };
                    out.push(Motion::Dwell(ms));
                    skip_motion = true;
                }
                // the program's units: its own code, never the other's
                20 | 21 if (*g == 20) != d.inch => {
                    return Err(ReadError {
                        line: ln,
                        what: format!(
                            "G{g}: this program is in {}",
                            if d.inch { "inches" } else { "mm" }
                        ),
                    });
                }
                90 | 94 | 17 | 20 | 21 | 40 | 49 | 98 | 64 | 54..=59 | 187 => {}
                43 => skip_motion = true,
                g if g_ignore.contains(g) => {}
                g if g_skip.contains(g) => skip_motion = true,
                28 | 53 | 91 => {
                    home = true;
                    skip_motion = true;
                }
                g => {
                    return Err(ReadError {
                        line: ln,
                        what: format!("G{g} is not in our subset"),
                    });
                }
            }
        }
        if home {
            if !out.ends_with(&[Motion::Home]) {
                out.push(Motion::Home);
            }
            pos = [None; 3];
            motion = None;
            continue;
        }
        if skip_motion {
            continue;
        }
        if centre_first && (get('I').is_some() || get('J').is_some()) {
            let need = |c: char| {
                get(c).ok_or_else(|| ReadError {
                    line: ln,
                    what: format!("a centre with no {c}"),
                })
            };
            centre = Some((parse_q(&need('I')?, k, ln)?, parse_q(&need('J')?, k, ln)?));
        }
        let has_xyz = get('X').is_some() || get('Y').is_some() || get('Z').is_some();
        if !has_xyz {
            continue;
        }
        let mut to = pos;
        for (i, c) in ['X', 'Y', 'Z'].into_iter().enumerate() {
            if let Some(v) = get(c) {
                to[i] = Some(parse_q(&v, k, ln)?);
            }
        }
        match motion {
            Some(80) => {
                let (kind, z, r, q, dw, clear) = cycle.unwrap();
                let (x, y) = (to[0].unwrap_or(0), to[1].unwrap_or(0));
                expand_hole(
                    kind,
                    [x, y],
                    z,
                    r,
                    clear,
                    q,
                    dw,
                    feed,
                    k,
                    d.inch,
                    (PECK_CLEAR, CHIP_LIFT),
                    &mut out,
                );
                pos = [Some(x), Some(y), Some(clear)];
                continue;
            }
            Some(g @ (0 | 1)) => {
                let p = [to[0], to[1], to[2]];
                if p.iter().any(|x| x.is_none()) {
                    return Err(ReadError {
                        line: ln,
                        what: "a move from an unknown position".into(),
                    });
                }
                let p = [p[0].unwrap(), p[1].unwrap(), p[2].unwrap()];
                out.push(if g == 0 {
                    Motion::Rapid(p)
                } else {
                    Motion::Feed(p, feed)
                });
            }
            Some(g @ (2 | 3)) => {
                let (sx, sy) = (
                    pos[0].ok_or_else(|| ReadError {
                        line: ln,
                        what: "an arc from nowhere".into(),
                    })?,
                    pos[1].unwrap(),
                );
                let (i, j) = if centre_first {
                    let (cx, cy) = centre.ok_or_else(|| ReadError {
                        line: ln,
                        what: "an arc with no centre defined".into(),
                    })?;
                    (cx - sx, cy - sy)
                } else {
                    let i = parse_q(
                        &get('I').ok_or_else(|| ReadError {
                            line: ln,
                            what: "an arc with no I".into(),
                        })?,
                        k,
                        ln,
                    )?;
                    let j = parse_q(
                        &get('J').ok_or_else(|| ReadError {
                            line: ln,
                            what: "an arc with no J".into(),
                        })?,
                        k,
                        ln,
                    )?;
                    (i, j)
                };
                let p = [
                    to[0].unwrap(),
                    to[1].unwrap(),
                    to[2].ok_or_else(|| ReadError {
                        line: ln,
                        what: "an arc from an unknown height".into(),
                    })?,
                ];
                out.push(Motion::Arc {
                    to: p,
                    c: [sx + i, sy + j],
                    cw: g == 2,
                    f: feed,
                    // as a control decides it
                    full: p[0] == sx && p[1] == sy,
                });
            }
            _ => {
                return Err(ReadError {
                    line: ln,
                    what: "coordinates with no motion mode".into(),
                });
            }
        }
        pos = to;
    }
    Ok(out)
}

/// A block that is a tape mark or a `%` header: `%`, `%NAME ...`, or `N<digits> %NAME ...`.
fn pct_block(s: &str) -> bool {
    strip_n(s).starts_with('%')
}

/// A block without its leading `N<digits>` (and the spaces around it).
fn strip_n(s: &str) -> &str {
    let t = s.trim_start();
    match t.strip_prefix(['N', 'n']) {
        Some(r) if r.starts_with(|c: char| c.is_ascii_digit()) => {
            r.trim_start_matches(|c: char| c.is_ascii_digit()).trim_start()
        }
        _ => t,
    }
}

/// The motion without its null moves (a rapid or feed to where the tool already is), which
/// change nothing and which a modal writer leaves out.
pub fn normalize(m: &[Motion]) -> Vec<Motion> {
    let mut out = Vec::new();
    let mut at: Option<[Q; 3]> = None;
    for x in m {
        match x {
            Motion::Rapid(p) | Motion::Feed(p, _) => {
                if at == Some(*p) {
                    continue;
                }
                at = Some(*p);
            }
            Motion::Arc { to, .. } => at = Some(*to),
            Motion::Home | Motion::Tool(_) => at = None,
            _ => {}
        }
        out.push(x.clone());
    }
    out
}

/// The tool a GRBL pause names, in the comment just before it: `(TOOL n: ...)`.
fn raw_tool(text_: &str, ln: usize) -> u32 {
    let lines: Vec<&str> = text_.lines().collect();
    if ln >= 2
        && let Some(r) = strip_n(lines[ln - 2]).trim_end().strip_prefix("(TOOL ")
    {
        return r
            .split(':')
            .next()
            .and_then(|t| t.trim().parse().ok())
            .unwrap_or(0);
    }
    0
}

/// The round trip (G6): the motion our text means is exactly the program's.
pub fn round_trip(p: &QProgram, d: &Dialect) -> Result<(String, Vec<Motion>), String> {
    let txt = print(p, d).map_err(|e| format!("{e:?}"))?;
    let back = read(&txt, d).map_err(|e| format!("line {}: {}", e.line, e.what))?;
    let (back, want) = (normalize(&back), normalize(&motions(p)));
    // the control's view drops nothing we emit; tool events in GRBL come from its pause
    if back != want {
        let i = back
            .iter()
            .zip(want.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(back.len().min(want.len()));
        return Err(format!(
            "round trip differs at motion {i}: read {:?}, meant {:?}",
            back.get(i),
            want.get(i)
        ));
    }
    Ok((txt, back))
}

#[cfg(test)]
mod tests;

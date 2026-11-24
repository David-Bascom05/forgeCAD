//! A G-code interpreter: posted text to what the control does, block by block, in exact integer
//! units. This is the semantics the simulation runs and verify/forge-machine (Lean) specifies.
//!
//! The subset (anything else is a typed error with its line, never a guess):
//! - words: N O G M T S F H D P Q R I J K and the control's axis letters; comments `( … )` and `;`
//!   to the end of the line; `%` lines; `/` block delete (the block is skipped: the switch is on).
//! - G0 G1 G2 G3 (I J K centres, incremental; or R, negative for more than half a turn), G4,
//!   G17 G18 G19, G21 (G20 inch is refused), G28 (home through an intermediate point), G40, G43 H
//!   G49, G53 (non-modal machine move), G54–G59, G64, G80–G85 G73 with G98 G99, G90 G91, G93 G94
//!   G95, G96 G97, G50 S (lathe: spindle clamp), G187 (Haas smoothing: no motion effect).
//! - skip (probe) moves G38.2–G38.5 / G31 where the dialect's `probe_move` names them (`Ev::Skip`,
//!   the shell's: outside the proven core).
//! - M0 M1 M2 M30 M3 M4 M5 M6 M7 M8 M9 M88 M89.
//!
//! Numbers: a word's value is held as an integer count of 10^-k (coordinates: k = `decimals`;
//! F: `feed_decimals`; S, T, H, N whole). More digits than k is an error, not a rounding. A word
//! without a point is read in calculator style (X10 is 10 mm), as forge-post's reader does.
//!
//! Within a block the order of execution is RS274/NGC's: feed mode, F, S, T, M6, M3–M5, coolant,
//! G4, plane, units, comp, length, WCS, distance mode, retract mode, G28/G53, motion.
//!
//! Canned cycles expand exactly as forge-post's `motions` does (the same peck clearance and chip
//! lift), so on the 3-axis subset the two agree word for word (tests/gcode_vs_post.rs).

pub type Q = i64;

use forge_machine_core as mcore;

#[derive(Clone, Debug, PartialEq)]
pub struct GError {
    pub line: usize,
    pub what: String,
}

fn ge(line: usize, what: impl Into<String>) -> GError {
    GError {
        line,
        what: what.into(),
    }
}

/// The skip codes a dialect's `probe_move` templates name: each template's leading G word
/// (`"G38.2 {axes} F{f}"` → 382, `"G31 {axes} F{f}"` → 310). Lines of a list each count.
fn skip_codes(d: &forge_tooling::toml::Table) -> Vec<u16> {
    use forge_tooling::toml;
    let ts = toml::texts(d, "probe_move")
        .or_else(|| toml::text(d, "probe_move").map(|t| vec![t.to_string()]))
        .unwrap_or_default();
    let mut out = Vec::new();
    for t in ts {
        for w in t.split_whitespace() {
            if let Some(g) = w.strip_prefix('G')
                && let Ok(v) = g.parse::<f64>()
            {
                let v = (v * 10.0).round();
                if (0.0..=9999.0).contains(&v) && !out.contains(&(v as u16)) {
                    out.push(v as u16);
                }
            }
        }
    }
    out
}

/// The probing macros a dialect's templates call: (P number, which).
fn macro_codes(d: &forge_tooling::toml::Table) -> Vec<(u32, Macro)> {
    use forge_tooling::toml;
    let mut out = Vec::new();
    for (k, m) in [
        ("probe_on", Macro::On),
        ("probe_off", Macro::Off),
        ("probe_protected", Macro::Protected),
        ("probe_surface", Macro::Surface),
    ] {
        let Some(t) = toml::text(d, k) else { continue };
        if !t.split_whitespace().any(|w| w == "G65") {
            continue;
        }
        if let Some(p) = t
            .split_whitespace()
            .find_map(|w| w.strip_prefix('P').and_then(|p| p.parse::<u32>().ok()))
        {
            out.push((p, m));
        }
    }
    out
}

/// What the interpreter needs to know about the control and the machine's words.
#[derive(Clone, Debug, PartialEq)]
pub struct Control {
    pub decimals: u32,
    pub feed_decimals: u32,
    /// The program is in inches (G20; `in_inches`): words are counts of 10^-decimals inch, feeds
    /// of 10^-feed_decimals inch/min; G21 is refused, as G20 is in a mm program.
    pub inch: bool,
    /// The dialect's inch decimals and feed decimals, when it has an inch code.
    pub inch_units: Option<(u32, u32)>,
    /// The axis words, in the order of the position vector.
    pub letters: Vec<char>,
    /// Which of `letters` are rotary (values in degrees).
    pub rotary: Vec<bool>,
    /// The letter programmed as a diameter (a lathe's X), if any.
    pub diameter: Option<char>,
    /// G4 P: milliseconds as an integer (Fanuc) or seconds with a point (`dwell_seconds`).
    pub dwell_seconds: bool,
    /// Lathe tool words: T0101 = station 1, offset 1, indexed at once (no M6).
    pub lathe: bool,
    /// The plane at program start: 17 (mill) or 18 (lathe).
    pub plane0: u8,
    /// G83's peck clearance and G73's chip-break lift, mm: the machine's (its control's
    /// parameters, e.g. Fanuc 5115/5114) when the record gives them, else forge-post's.
    pub peck_clear: f64,
    pub chip_lift: f64,
    /// The machine has a pallet changer: M60 swaps the pallet.
    pub pallets: bool,
    /// The M codes that are multi-channel waits (`wait_m = [lo, hi]` in the dialect).
    pub wait_m: Option<(u32, u32)>,
    /// The channel these letters drive (1: letters with no digit or a 1; 2: Z2, C2, ...).
    pub channel: u8,
    /// The dialect's lexical extras, read in this shell before the words reach the core.
    pub lex: Lex,
    /// The skip (probe) moves this dialect cites, in tenths (G38.2 = 382, G31 = 310): from its
    /// `probe_move` template. Empty: a skip move is refused as outside the subset.
    pub skip: Vec<u16>,
    /// The probing macros this dialect cites (G65 P<n>): each template's P number and what it is
    /// (`probe_on`, `probe_off`, `probe_protected`, `probe_surface`). Others are refused.
    pub macros: Vec<(u32, Macro)>,
}

/// A probing macro the simulation models from its published description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Macro {
    /// Probe on / off: a small Z move of unpublished size inside.
    On,
    Off,
    /// A feed move to the target that stops with an alarm if the probe triggers.
    Protected,
    /// Along one axis toward the surface, `Q` past it, then back to the start.
    Surface,
}

/// A dialect's text conventions beyond the Fanuc words the core interprets (forge-post's
/// data keys; research/cam/posts). They are rewritten here, in the shell, into the core's words:
/// the core and its Lean spec (verify/forge-machine) cover the Fanuc subset only.
#[derive(Clone, Debug, PartialEq)]
pub struct Lex {
    /// A block terminator to drop (HEIDENHAIN ISO `*`).
    pub block_end: String,
    /// G codes (tenths: G91.1 = 911) that carry no motion on this control (G291, G71, G91.1).
    pub g_ignore: Vec<u16>,
    /// The T block is the tool change (no M6); a bare T changes only the speed.
    pub tool_word: bool,
    /// The arc centre is its own block, absolute (I J), before G02/G03.
    pub centre_abs: bool,
    /// The dwell's word in a G4 block (P, or F / X on some controls).
    pub dwell_letter: char,
    /// A canned cycle's P counts milliseconds (G82 P250) though G4's dwell is in seconds.
    pub cycle_p_ms: bool,
    /// The tool change is a comment naming the tool, then M0 (GRBL, which has no M6): the
    /// comment's text before the number ("TOOL "). The M0 that follows changes the tool.
    pub tool_comment: Option<String>,
}

impl Default for Lex {
    fn default() -> Lex {
        Lex {
            block_end: String::new(),
            g_ignore: Vec::new(),
            tool_word: false,
            centre_abs: false,
            dwell_letter: 'P',
            cycle_p_ms: false,
            tool_comment: None,
        }
    }
}

impl Control {
    /// A 3-axis mill control on X Y Z with forge-post's numbers for a dialect.
    pub fn mill3(decimals: u32, feed_decimals: u32, dwell_seconds: bool) -> Control {
        Control {
            decimals,
            feed_decimals,
            inch: false,
            inch_units: None,
            letters: vec!['X', 'Y', 'Z'],
            rotary: vec![false; 3],
            diameter: None,
            dwell_seconds,
            lathe: false,
            plane0: 17,
            peck_clear: forge_post::PECK_CLEAR,
            chip_lift: forge_post::CHIP_LIFT,
            pallets: false,
            wait_m: None,
            channel: 1,
            lex: Lex::default(),
            skip: Vec::new(),
            macros: Vec::new(),
        }
    }

    /// The control for a dialect's text (forge-post's `[dialect]` TOML) on a machine definition,
    /// for one channel (1 = the letters without a digit). The machine's letters of that channel
    /// become the axis words.
    pub fn for_machine(
        def: &crate::def::MachineDef,
        dialect_src: &str,
        channel: u8,
    ) -> Result<Control, String> {
        use forge_tooling::toml;
        let root = toml::parse(dialect_src).map_err(|e| format!("dialect line {}: {}", e.line, e.what))?;
        let Some(toml::Val::Table(d)) = root.get("dialect") else {
            return Err("no [dialect] table".into());
        };
        let decimals = toml::num(d, "decimals").ok_or("the dialect has no decimals")? as u32;
        let feed_decimals = toml::num(d, "feed_decimals").ok_or("the dialect has no feed_decimals")? as u32;
        // a dialect may have no dwell (its units unknown: forge-post refuses one); a G4 in its
        // text is then read as Fanuc's P milliseconds
        let dwell = toml::text(d, "dwell").unwrap_or("G4 P{ms}");
        let mut letters = Vec::new();
        let mut rotary = Vec::new();
        let mut diameter = None;
        for j in &def.joints {
            let base = j.letter.trim_end_matches(|c: char| c.is_ascii_digit());
            let ch: u8 = j.letter[base.len()..].parse().unwrap_or(1);
            if ch != channel {
                continue;
            }
            let c = base.chars().next().unwrap();
            letters.push(c);
            rotary.push(j.ty == crate::def::JointType::Rotary);
            if j.diameter {
                diameter = Some(c);
            }
        }
        let lathe = def.kind.turns();
        // the dwell's word: the letter before the template's {ms} or {s}
        let dwell_letter = dwell
            .find("{ms}")
            .or_else(|| dwell.find("{s}"))
            .and_then(|i| dwell[..i].chars().last())
            .filter(|c| c.is_ascii_alphabetic())
            .unwrap_or('P');
        let lex = Lex {
            block_end: toml::text(d, "block_end").unwrap_or("").trim().to_string(),
            g_ignore: toml::nums(d, "read_g_ignore")
                .unwrap_or_default()
                .into_iter()
                .map(|g| (g * 10.0).round() as u16)
                .collect(),
            tool_word: d.get("tool_word") == Some(&toml::Val::Bool(true)),
            centre_abs: toml::text(d, "arc") == Some("centre_abs"),
            dwell_letter,
            cycle_p_ms: !dwell.contains("{ms}")
                && toml::text(d, "cycle_dwell").is_some_and(|t| t.contains("P{ms}")),
            // `tool_change = ["(TOOL {t}: {name})", "M0"]`: the comment before {t}
            tool_comment: toml::texts(d, "tool_change").and_then(|tc| {
                let first = tc.first()?.trim();
                let last = tc.last()?.trim();
                if tc.len() < 2 || !first.starts_with('(') || last != "M0" {
                    return None;
                }
                let i = first.find("{t}")?;
                Some(first[1..i].to_string())
            }),
        };
        let inch_units = (toml::text(d, "units_inch").is_some_and(|t| !t.is_empty())
            && !toml::tables(d, "cite").iter().any(|c| {
                toml::text(c, "key") == Some("units_inch") && toml::text(c, "status") == Some("refused")
            }))
        .then(|| {
            (
                toml::num(d, "inch_decimals").map_or(decimals + 1, |x| x as u32),
                toml::num(d, "inch_feed_decimals").map_or(feed_decimals + 1, |x| x as u32),
            )
        });
        Ok(Control {
            lex,
            decimals,
            feed_decimals,
            inch: false,
            inch_units,
            letters,
            rotary,
            diameter,
            dwell_seconds: !dwell.contains("{ms}"),
            lathe,
            plane0: if lathe { 18 } else { 17 },
            peck_clear: def.peck_clear.unwrap_or(forge_post::PECK_CLEAR),
            chip_lift: def.chip_lift.unwrap_or(forge_post::CHIP_LIFT),
            pallets: def.pallets.is_some(),
            channel,
            wait_m: toml::nums(d, "wait_m").and_then(|w| match w.as_slice() {
                [lo, hi] if lo <= hi && *lo >= 0.0 => Some((*lo as u32, *hi as u32)),
                _ => None,
            }),
            skip: skip_codes(d),
            macros: macro_codes(d),
        })
    }

    /// The control reading an inch program (posted with forge-post's `PostOptions::inch`): the
    /// dialect's inch decimals; refused where the dialect has no inch code.
    pub fn in_inches(&self) -> Result<Control, String> {
        let (k, fk) = self
            .inch_units
            .ok_or("the dialect has no inch code (post in mm)")?;
        let mut c = self.clone();
        c.inch = true;
        c.decimals = k;
        c.feed_decimals = fk;
        Ok(c)
    }

    /// A built-in forge-post dialect's text by name.
    pub fn dialect_src(name: &str) -> Option<&'static str> {
        forge_post::DIALECTS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, s)| *s)
    }

    /// For a built-in forge-post dialect by name.
    pub fn for_dialect(def: &crate::def::MachineDef, dialect: &str, channel: u8) -> Result<Control, String> {
        let src = forge_post::DIALECTS
            .iter()
            .find(|(n, _)| *n == dialect)
            .ok_or_else(|| format!("no built-in dialect `{dialect}`"))?
            .1;
        Control::for_machine(def, src, channel)
    }

    pub fn idx(&self, c: char) -> Option<usize> {
        self.letters.iter().position(|l| *l == c)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedMode {
    /// G94: mm/min (deg/min for a rotary-only move).
    PerMin,
    /// G93: the move takes 1/F minutes.
    InverseTime,
    /// G95: mm per spindle revolution.
    PerRev,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coolant {
    Flood,
    Mist,
    Through,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Centre {
    /// The centre in plane coordinates, program units (the start plus I J K; on a diameter X, I
    /// counts twice: a radius added to a diameter word).
    Abs([Q; 2]),
    /// The radius word: negative for an arc of more than half a turn.
    R(Q),
}

/// What the control does. Positions are program coordinates (the words: a diameter letter holds
/// the diameter), one entry per `Control::letters`; None where the program has not yet said.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Rapid(Vec<Option<Q>>),
    Feed {
        to: Vec<Option<Q>>,
        f: Q,
        mode: FeedMode,
    },
    /// In plane 17 (X Y), 18 (Z X) or 19 (Y Z), viewed from the plane normal's + side; `full`: a
    /// whole turn (the end's plane coordinates are the start's).
    Arc {
        to: Vec<Option<Q>>,
        plane: u8,
        centre: Centre,
        cw: bool,
        full: bool,
        f: Q,
        mode: FeedMode,
    },
    /// Non-modal machine-coordinate rapid (G53): (letter index, machine value).
    Machine(Vec<(usize, Q)>),
    /// G28: rapid to the intermediate point, then these letters to their reference (home).
    Home {
        via: Vec<Option<Q>>,
        axes: Vec<usize>,
    },
    Dwell {
        ms: Q,
    },
    /// T: the next tool (a mill's changer pre-selects).
    Select(u32),
    /// M6 on a mill: the selected tool goes in. Lathe T0101: station and offset.
    Tool {
        t: u32,
        offset: Option<u32>,
    },
    /// G43 H (Some) or G49 (None).
    Length(Option<u32>),
    Wcs(u32),
    /// rpm (G97) or surface speed m/min (G96), cw = M3.
    Spindle {
        s: Q,
        cw: bool,
        css: bool,
    },
    SpindleStop,
    /// G50 S on a lathe: the rpm clamp for G96.
    Clamp(Q),
    Coolant(Coolant),
    /// M0/M1 (optional stop), M2/M30.
    Stop {
        optional: bool,
    },
    /// A comment's text (the operator's message at a stop, a tool's name).
    Comment(String),
    End,
    /// M60 on a machine with a pallet changer: the pallet in the machining position is swapped.
    Pallet,
    /// A multi-channel wait (Fanuc M100–M199 by the dialect's `wait_m`): this channel holds until
    /// every channel with the same wait reaches it.
    Wait(u32),
    /// A skip (probe) move, G38.2–G38.5 or G31 (`code` in tenths): a feed toward `to` that stops
    /// somewhere unknown along it when the probe triggers (G38.4/.5: when contact is lost). The
    /// simulation sweeps the whole segment; afterwards the moved axes' positions are unknown to
    /// the interpreter until an absolute word sets them.
    Skip {
        to: Vec<Option<Q>>,
        f: Q,
        code: u16,
    },
    /// A probing macro switched the probe on or off: its small Z move is not published, so Z is
    /// unknown to the interpreter afterwards (the simulation does not move it).
    ProbeSwitch(bool),
    /// A protected positioning macro: a feed move to `to` that alarms if the probe triggers.
    Protected {
        to: Vec<Option<Q>>,
        f: Q,
    },
    /// A single-surface measure macro along letter index `axis` toward `at`, `q` past it (`to`:
    /// that end, the only word set), then back to the start (the macro's own feeds are not
    /// modelled: its time is unknown).
    /// `wcs`: the work offset (54..59) the macro sets from the error (S1..S6), if any.
    Surface {
        axis: usize,
        at: Q,
        q: Q,
        to: Vec<Option<Q>>,
        wcs: Option<u32>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// 1-based line in the text.
    pub line: usize,
    pub ev: Ev,
}

/// The modal state between blocks: the core's (forge-machine-core) and the words the core does
/// not read.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub core: mcore::State,
    pub feed_mode: FeedMode,
    pub rpm: Q,
    pub css: bool,
    pub selected: Option<u32>,
    /// The arc centre an absolute I J block set (`Lex::centre_abs`), in program units.
    pub centre: Option<(Q, Q)>,
    /// A tool a `Lex::tool_comment` comment named, waiting for its M0.
    pub comment_tool: Option<u32>,
    /// The protected positioning macro's feed: modal to that macro only (Inspection Plus O9810).
    pub protected_f: Option<Q>,
    /// The probe is switched on (a probing macro's `probe_on`): no tool change until it is off.
    pub probe_on: bool,
    /// The probe was switched off and the G28 reference return its macro requires has not come.
    pub probe_off_pending: bool,
}

impl State {
    pub fn new(c: &Control) -> State {
        State {
            core: mcore::State {
                pos: vec![None; c.letters.len()],
                absolute: true,
                mode: None,
                plane: c.plane0 as u32,
                feed: None,
                retract_initial: true,
                cyc: None,
            },
            feed_mode: FeedMode::PerMin,
            rpm: 0,
            css: false,
            selected: None,
            centre: None,
            comment_tool: None,
            protected_f: None,
            probe_on: false,
            probe_off_pending: false,
        }
    }
}

fn pow10(k: u32) -> i64 {
    10i64.pow(k)
}

/// A decimal word's value in integer units of 10^-k, exactly.
pub fn parse_q(s: &str, k: u32, line: usize) -> Result<Q, GError> {
    let bad = |w: &str| ge(line, format!("{w}: `{s}`"));
    let (neg, body) = match s.strip_prefix('-') {
        Some(b) => (true, b),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (ip, fp) = body.split_once('.').unwrap_or((body, ""));
    if fp.len() > k as usize
        || (ip.is_empty() && fp.is_empty())
        || !ip.bytes().all(|c| c.is_ascii_digit())
        || !fp.bytes().all(|c| c.is_ascii_digit())
    {
        return Err(bad("not a number with at most the control's digits"));
    }
    let i: i64 = if ip.is_empty() {
        0
    } else {
        ip.parse().map_err(|_| bad("too large"))?
    };
    let mut v = i.checked_mul(pow10(k)).ok_or_else(|| bad("too large"))?;
    if !fp.is_empty() {
        let f: i64 = fp.parse().map_err(|_| bad("bad fraction"))?;
        v = v
            .checked_add(f * pow10(k - fp.len() as u32))
            .ok_or_else(|| bad("too large"))?;
    }
    Ok(if neg { -v } else { v })
}

/// A block's words, upper-cased, in order, and its comments.
fn words(
    raw: &str,
    line: usize,
    comments: &mut Vec<String>,
    lex: &Lex,
) -> Result<Option<Vec<(char, String)>>, GError> {
    let raw = if lex.block_end.is_empty() {
        raw
    } else {
        let t = raw.trim_end();
        t.strip_suffix(lex.block_end.as_str()).unwrap_or(t)
    };
    let mut s = String::new();
    let mut depth = 0i32;
    let mut com = String::new();
    for ch in raw.chars() {
        match ch {
            '(' => {
                if depth > 0 {
                    com.push(ch);
                }
                depth += 1
            }
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return Err(ge(line, "a `)` with no `(`"));
                }
                if depth == 0 {
                    comments.push(std::mem::take(&mut com));
                } else {
                    com.push(ch);
                }
            }
            ';' if depth == 0 => break,
            _ if depth == 0 => s.push(ch),
            _ => com.push(ch),
        }
    }
    if depth != 0 {
        return Err(ge(line, "an unclosed comment"));
    }
    let s = s.trim();
    // a tape mark, or a program header / last block such as `%NAME G71 *` (`N99999999 %NAME G71
    // *`): skipped only when the block is `%` or starts with it after an optional N word; any
    // other `%` is refused below as a character where a word belongs
    if s.is_empty() || pct_block(s) {
        return Ok(None);
    }
    if s.starts_with('/') {
        return Ok(None);
    }
    // words may be written with or without spaces: X10.Y5.
    let mut out: Vec<(char, String)> = Vec::new();
    let b: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    let mut i = 0;
    while i < b.len() {
        let l = b[i].to_ascii_uppercase();
        if !l.is_ascii_alphabetic() {
            return Err(ge(line, format!("`{}` where a word letter belongs", b[i])));
        }
        let mut j = i + 1;
        while j < b.len()
            && (b[j].is_ascii_digit() || b[j] == '.' || ((b[j] == '-' || b[j] == '+') && j == i + 1))
        {
            j += 1;
        }
        let v: String = b[i + 1..j].iter().collect();
        if v.is_empty() && l == 'T' && lex.tool_word {
            // a bare T block: only the speed changes
            i = j;
            continue;
        }
        if v.is_empty() {
            return Err(ge(line, format!("the word `{l}` has no value")));
        }
        out.push((l, v));
        i = j;
    }
    Ok(Some(out))
}

/// A block that is a tape mark or a `%` header: `%`, `%NAME ...`, or `N<digits> %NAME ...`.
pub fn pct_block(s: &str) -> bool {
    let t = s.trim_start();
    let t = match t.strip_prefix(['N', 'n']) {
        Some(r) if r.starts_with(|c: char| c.is_ascii_digit()) => {
            r.trim_start_matches(|c: char| c.is_ascii_digit()).trim_start()
        }
        _ => t,
    };
    t.starts_with('%')
}

/// A value in integer units of 10^-k as a decimal word.
fn q_text(q: Q, k: u32) -> String {
    let p = pow10(k);
    let (sign, a) = if q < 0 { ("-", -q) } else { ("", q) };
    if k == 0 {
        format!("{sign}{a}")
    } else {
        format!("{sign}{}.{:0w$}", a / p, a % p, w = k as usize)
    }
}

/// The dialect's extras rewritten into the core's (Fanuc) words; None if the block only set
/// shell state (an absolute arc centre).
fn lex_rewrite(
    mut ws: Vec<(char, String)>,
    ln: usize,
    c: &Control,
    st: &mut State,
) -> Result<Option<Vec<(char, String)>>, GError> {
    let lex = &c.lex;
    let g_of = |v: &str| parse_q(v, 1, ln).ok();
    if !lex.g_ignore.is_empty() {
        ws.retain(|(l, v)| *l != 'G' || !g_of(v).is_some_and(|g| lex.g_ignore.contains(&(g as u16))));
    }
    let has_g = |ws: &[(char, String)], g: i64| ws.iter().any(|(l, v)| *l == 'G' && g_of(v) == Some(g * 10));
    // the dwell's word to P (G4 F seconds; G04 X seconds). Every word with that letter in a G4
    // block is renamed: a dwell block carries no motion, so the letter means only the dwell.
    if lex.dwell_letter != 'P' && has_g(&ws, 4) {
        for w in ws.iter_mut() {
            if w.0 == lex.dwell_letter {
                w.0 = 'P';
            }
        }
    }
    // a cycle's P in milliseconds where G4 counts seconds: to seconds for the core. The P counts
    // whole milliseconds (Siemens' ISO manual: G82 ... P1000 = 1 s); a fraction is refused.
    if lex.cycle_p_ms && [73, 81, 82, 83, 84, 85].iter().any(|g| has_g(&ws, *g)) {
        for w in ws.iter_mut() {
            if w.0 == 'P' {
                w.1 = q_text(parse_q(&w.1, 0, ln)?, 3);
            }
        }
    }
    // the T block is the tool change
    if lex.tool_word
        && ws.iter().any(|(l, v)| *l == 'T' && !v.is_empty())
        && !ws.iter().any(|(l, v)| *l == 'M' && parse_q(v, 0, ln) == Ok(6))
    {
        ws.push(('M', "6".into()));
    }
    if lex.centre_abs {
        let k = c.decimals;
        let ij: Vec<(char, String)> = ws
            .iter()
            .filter(|(l, _)| matches!(l, 'I' | 'J'))
            .cloned()
            .collect();
        let arc = has_g(&ws, 2) || has_g(&ws, 3);
        let radius = ws.iter().any(|(l, _)| *l == 'R');
        if !ij.is_empty() && !arc {
            let get = |ch: char| {
                ij.iter()
                    .find(|(l, _)| *l == ch)
                    .map(|(_, v)| parse_q(v, k, ln))
                    .ok_or_else(|| ge(ln, format!("a centre block with no {ch}")))?
            };
            st.centre = Some((get('I')?, get('J')?));
            ws.retain(|(l, _)| !matches!(l, 'I' | 'J'));
            if ws.iter().all(|(l, _)| *l == 'N') {
                return Ok(None);
            }
        } else if arc && !radius {
            // the absolute centre is I J in the XY plane only; G18/G19 (I K, J K) is not modelled
            if st.core.plane != 17 {
                return Err(ge(
                    ln,
                    "an absolute-centre arc outside G17 is not modelled: post the arc in G17 or with R",
                ));
            }
            let (cx, cy) = st.centre.ok_or_else(|| ge(ln, "an arc with no centre defined"))?;
            let at = |ch: char| {
                c.idx(ch)
                    .and_then(|i| st.core.pos[i])
                    .ok_or_else(|| ge(ln, "an arc from an unknown position"))
            };
            let (sx, sy) = (at('X')?, at('Y')?);
            ws.retain(|(l, _)| !matches!(l, 'I' | 'J'));
            ws.push(('I', q_text(cx - sx, k)));
            ws.push(('J', q_text(cy - sy, k)));
        }
    }
    Ok(Some(ws))
}

/// Interprets a whole program.
pub fn run(text: &str, c: &Control) -> Result<(Vec<Block>, State), GError> {
    let mut st = State::new(c);
    let mut out = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        block(raw, n + 1, c, &mut st, &mut out)?;
    }
    Ok((out, st))
}

/// One block: its events appended to `out`, the state advanced.
pub fn block(raw: &str, ln: usize, c: &Control, st: &mut State, out: &mut Vec<Block>) -> Result<(), GError> {
    let mut comments = Vec::new();
    let parsed = words(raw, ln, &mut comments, &c.lex)?;
    let push = |out: &mut Vec<Block>, ev: Ev| out.push(Block { line: ln, ev });
    for cm in comments {
        if let Some(pre) = &c.lex.tool_comment
            && let Some(rest) = cm.strip_prefix(pre.as_str())
        {
            let n: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
            if let Ok(t) = n.parse() {
                st.comment_tool = Some(t);
            }
        }
        push(out, Ev::Comment(cm));
    }
    let Some(ws) = parsed else { return Ok(()) };
    let Some(ws) = lex_rewrite(ws, ln, c, st)? else {
        return Ok(());
    };
    let k = c.decimals;
    let mut gs: Vec<u16> = Vec::new(); // G codes in tenths: G43.4 = 434
    let mut ms: Vec<u32> = Vec::new();
    let mut seen: Vec<char> = Vec::new();
    for (l, v) in &ws {
        match l {
            'G' => {
                let g = parse_q(v, 1, ln)?;
                if !(0..=9999).contains(&g) {
                    return Err(ge(ln, format!("G{v}")));
                }
                gs.push(g as u16);
            }
            'M' => ms.push(parse_q(v, 0, ln).map_err(|_| ge(ln, format!("M{v}")))? as u32),
            _ => {
                if seen.contains(l) {
                    return Err(ge(ln, format!("the word `{l}` twice in one block")));
                }
                seen.push(*l);
                let known = matches!(
                    l,
                    'N' | 'O' | 'T' | 'S' | 'F' | 'H' | 'D' | 'P' | 'Q' | 'R' | 'I' | 'J' | 'K'
                ) || c.letters.contains(l);
                if !known {
                    return Err(ge(
                        ln,
                        format!("an unknown word `{l}{v}` (axis words here: {:?})", c.letters),
                    ));
                }
            }
        }
    }
    let get = |ch: char| ws.iter().find(|(l, _)| *l == ch).map(|(_, v)| v.as_str());
    let coord = |ch: char| -> Result<Option<Q>, GError> { get(ch).map(|v| parse_q(v, k, ln)).transpose() };
    let whole = |ch: char| -> Result<Option<Q>, GError> { get(ch).map(|v| parse_q(v, 0, ln)).transpose() };
    let has_g = |g: u16| gs.contains(&(g * 10));
    // the G codes we know, by group; anything else is refused
    for g in &gs {
        let ok = c.skip.contains(g)
            || (*g == 650 && !c.macros.is_empty())
            || matches!(
                *g,
                0 | 10
                    | 20
                    | 30
                    | 40
                    | 170
                    | 180
                    | 190
                    | 280
                    | 400
                    | 430
                    | 490
                    | 500
                    | 530
                    | 540
                    | 550
                    | 560
                    | 570
                    | 580
                    | 590
                    | 640
                    | 730
                    | 800
                    | 810
                    | 820
                    | 830
                    | 840
                    | 850
                    | 900
                    | 910
                    | 930
                    | 940
                    | 950
                    | 960
                    | 970
                    | 980
                    | 990
                    | 1870
            );
        // the program's units: its own code, never the other's (no switch within a program)
        let ok = ok || (*g == 210 && !c.inch) || (*g == 200 && c.inch);
        if !ok {
            let what = match g {
                200 => "G20 (inch) in a program read in mm: read it with the control in inches".to_string(),
                210 => "G21 (mm) in a program read in inches".to_string(),
                410 | 420 => {
                    "cutter compensation (G41/G42) is not modelled: post the compensated path".to_string()
                }
                434 | 2340 => "TCP (G43.4) is not modelled yet".to_string(),
                _ => format!(
                    "G{}{} is not in the subset",
                    g / 10,
                    if g % 10 != 0 {
                        format!(".{}", g % 10)
                    } else {
                        String::new()
                    }
                ),
            };
            return Err(ge(ln, what));
        }
    }
    // a block's modal-group conflicts: two motions
    let motion_gs: Vec<u16> = gs
        .iter()
        .copied()
        .filter(|g| matches!(g, 0 | 10 | 20 | 30 | 730 | 800..=850))
        .collect();
    if motion_gs.len() > 1 {
        return Err(ge(ln, "two motion codes in one block"));
    }
    // 1. feed mode, F, S
    if has_g(93) {
        st.feed_mode = FeedMode::InverseTime;
    }
    if has_g(94) {
        st.feed_mode = FeedMode::PerMin;
    }
    if has_g(95) {
        st.feed_mode = FeedMode::PerRev;
    }
    let f_word = match get('F') {
        Some(f) => {
            let v = parse_q(f, c.feed_decimals, ln)?;
            if v < 0 {
                return Err(ge(ln, "a negative feed"));
            }
            Some(v)
        }
        None => None,
    };
    if has_g(96) {
        st.css = true;
    }
    if has_g(97) {
        st.css = false;
    }
    let s_word = match get('S') {
        Some(v) => {
            // S2000 or S2000. : whole
            let v = v.strip_suffix('.').unwrap_or(v);
            Some(parse_q(v, 0, ln)?)
        }
        None => None,
    };
    if has_g(50) {
        if !c.lathe {
            return Err(ge(
                ln,
                "G50 here is a lathe's spindle clamp; this control is a mill",
            ));
        }
        let s = s_word.ok_or_else(|| ge(ln, "G50 with no S"))?;
        push(out, Ev::Clamp(s));
    } else if let Some(s) = s_word
        && !has_g(65)
    {
        // (a probing macro's S is the work offset it sets, not a speed)
        st.rpm = s;
    }
    // 2. T, M6
    if let Some(t) = whole('T')? {
        if t < 0 {
            return Err(ge(ln, "a negative T"));
        }
        if c.lathe {
            // T0101: station, offset; T1 alone is station 1 with no offset change
            let txt = get('T').unwrap();
            let (stn, off) = if txt.len() >= 3 {
                (t / 100, Some((t % 100) as u32))
            } else {
                (t, None)
            };
            push(
                out,
                Ev::Tool {
                    t: stn as u32,
                    offset: off,
                },
            );
        } else {
            st.selected = Some(t as u32);
            push(out, Ev::Select(t as u32));
        }
    }
    let mut stop: Option<Ev> = None;
    for m in &ms {
        match m {
            6 => {
                if c.lathe {
                    return Err(ge(ln, "M6 on a lathe: the turret indexes on T"));
                }
                let t = st
                    .selected
                    .ok_or_else(|| ge(ln, "M6 with no tool selected (T)"))?;
                push(out, Ev::Tool { t, offset: None });
            }
            3 | 4 => push(
                out,
                Ev::Spindle {
                    s: st.rpm,
                    cw: *m == 3,
                    css: st.css,
                },
            ),
            5 => push(out, Ev::SpindleStop),
            7 => push(out, Ev::Coolant(Coolant::Mist)),
            8 => push(out, Ev::Coolant(Coolant::Flood)),
            88 => push(out, Ev::Coolant(Coolant::Through)),
            9 | 89 => push(out, Ev::Coolant(Coolant::Off)),
            0 | 1 => stop = Some(Ev::Stop { optional: *m == 1 }),
            2 | 30 => stop = Some(Ev::End),
            60 if c.pallets => push(out, Ev::Pallet),
            m if c.wait_m.is_some_and(|(lo, hi)| (lo..=hi).contains(m)) => push(out, Ev::Wait(*m)),
            m => return Err(ge(ln, format!("M{m} is not in the subset"))),
        }
    }
    // a manual change by comment (GRBL): the M0 after `(TOOL n: …)` puts tool n in
    let after_stop: Option<Ev> = match (&stop, c.lex.tool_comment.is_some()) {
        (Some(Ev::Stop { optional: false }), true) => {
            st.comment_tool.take().map(|t| Ev::Tool { t, offset: None })
        }
        _ => None,
    };
    // 3. dwell
    let mut nonmodal_used_words = false;
    if has_g(4) {
        let p = get('P').ok_or_else(|| ge(ln, "G4 with no P"))?;
        let ms = if c.dwell_seconds {
            parse_q(p, 3, ln)?
        } else {
            parse_q(p, 0, ln)?
        };
        if ms < 0 {
            return Err(ge(ln, "a negative dwell"));
        }
        push(out, Ev::Dwell { ms });
        nonmodal_used_words = true;
    }
    // 4. length, wcs (the shell's); plane, distance and retract modes go to the core
    if has_g(43) {
        let h = whole('H')?.ok_or_else(|| ge(ln, "G43 with no H"))?;
        push(out, Ev::Length(Some(h as u32)));
    }
    if has_g(49) {
        push(out, Ev::Length(None));
    }
    for n in 54..=59u16 {
        if has_g(n) {
            push(out, Ev::Wcs(n as u32));
        }
    }
    let mut axis: Vec<Option<Q>> = vec![None; c.letters.len()];
    for (i, l) in c.letters.iter().enumerate() {
        axis[i] = coord(*l)?;
    }
    let any_axis = axis.iter().any(|a| a.is_some());
    // the probe switch macros' rules (Renishaw H-2000-6222 p. 9-23): the probe is off before a
    // tool change, and the move after switching it off is a G28 reference return
    if st.probe_on && (ms.contains(&6) || (c.lathe && get('T').is_some())) {
        return Err(ge(
            ln,
            "a tool change while the probe is on: switch it off first (G65 P9833)",
        ));
    }
    if st.probe_off_pending && any_axis {
        if has_g(28) {
            st.probe_off_pending = false;
        } else {
            return Err(ge(
                ln,
                "a move after the probe is switched off that is not a G28 reference return (the off macro moves Z; Renishaw H-2000-6222 p. 9-23)",
            ));
        }
    }
    let absolute = if has_g(91) {
        Some(false)
    } else if has_g(90) {
        Some(true)
    } else {
        None
    };
    // 5. G53: a non-modal machine-coordinate move (the shell's: the core's position of those
    // axes becomes unknown)
    if has_g(53) {
        if !absolute.unwrap_or(st.core.absolute) {
            return Err(ge(ln, "G53 in G91: machine moves are absolute"));
        }
        let mv: Vec<(usize, Q)> = (0..axis.len()).filter_map(|i| axis[i].map(|v| (i, v))).collect();
        push(out, Ev::Machine(mv.clone()));
        if let Some(a) = absolute {
            st.core.absolute = a;
        }
        for (i, _) in mv {
            st.core.pos[i] = None;
        }
        for e in stop.into_iter().chain(after_stop) {
            push(out, e);
        }
        return Ok(());
    }
    // 5b. G91 G28 with every axis word 0 (posts write `G91 G28 Z0.` before M6): the intermediate
    // point is where the axes already are, so they go straight to their reference, exactly so
    // whether or not their position is known. The shell's, like G53 (the core refuses any
    // incremental word from an unknown position); the homed axes' program position becomes
    // unknown, as after the core's G28.
    if has_g(28)
        && !absolute.unwrap_or(st.core.absolute)
        && any_axis
        && axis.iter().all(|a| a.is_none_or(|v| v == 0))
        && (0..axis.len()).any(|i| axis[i].is_some() && st.core.pos[i].is_none())
        && gs.iter().all(|g| matches!(g, 280 | 900 | 910))
    {
        let axes: Vec<usize> = (0..axis.len()).filter(|&i| axis[i].is_some()).collect();
        push(
            out,
            Ev::Home {
                via: vec![None; axis.len()],
                axes: axes.clone(),
            },
        );
        if let Some(a) = absolute {
            st.core.absolute = a;
        }
        for i in axes {
            st.core.pos[i] = None;
        }
        for e in stop.into_iter().chain(after_stop) {
            push(out, e);
        }
        return Ok(());
    }
    // 5b'. a probing macro call (G65 P<n>, the dialect's cited ones): the shell's, outside the
    // proven core, modelled from the macro's published description
    if has_g(65) {
        let p = get('P').and_then(|v| v.trim_end_matches('.').parse::<u32>().ok());
        let Some(&(_, mac)) = c.macros.iter().find(|(n, _)| Some(*n) == p) else {
            return Err(ge(
                ln,
                format!("G65 P{}: not a probing macro this dialect cites", p.unwrap_or(0)),
            ));
        };
        if gs.len() > 1 {
            return Err(ge(ln, "a probing macro call with other G codes in the block"));
        }
        if !st.core.absolute {
            return Err(ge(
                ln,
                "a probing macro in G91 is not modelled: its words are positions",
            ));
        }
        let zi = c.idx('Z');
        match mac {
            Macro::On | Macro::Off => {
                st.probe_on = mac == Macro::On;
                st.probe_off_pending = mac == Macro::Off;
                push(out, Ev::ProbeSwitch(mac == Macro::On));
                if let Some(z) = zi {
                    st.core.pos[z] = None;
                }
            }
            Macro::Protected => {
                // its F is modal to this macro only
                let f = f_word
                    .or(st.protected_f)
                    .ok_or_else(|| ge(ln, "a protected move with no feed"))?;
                st.protected_f = Some(f);
                let mut to = vec![None; axis.len()];
                for i in 0..axis.len() {
                    if let Some(v) = axis[i] {
                        if st.core.pos[i].is_none() {
                            // the move starts from where the machine is: unknown here
                            return Err(ge(ln, "a protected move from an unknown position"));
                        }
                        to[i] = Some(v);
                    }
                }
                if !any_axis {
                    return Err(ge(ln, "a protected move with no axis word"));
                }
                push(out, Ev::Protected { to: to.clone(), f });
                for (i, t) in to.iter().enumerate() {
                    if let Some(v) = t {
                        st.core.pos[i] = Some(*v);
                    }
                }
            }
            Macro::Surface => {
                let named: Vec<usize> = (0..axis.len()).filter(|&i| axis[i].is_some()).collect();
                let [ax] = named[..] else {
                    return Err(ge(ln, "a single-surface measure names one axis"));
                };
                if c.letters[ax] != 'X' && c.letters[ax] != 'Y' && c.letters[ax] != 'Z' {
                    return Err(ge(ln, "a single-surface measure on X, Y or Z only"));
                }
                if st.core.pos.iter().any(|p| p.is_none()) {
                    return Err(ge(ln, "a single-surface measure from an unknown position"));
                }
                let q = coord('Q')?.ok_or_else(|| {
                    ge(
                        ln,
                        "a single-surface measure with no Q (its default is not assumed)",
                    )
                })?;
                if q <= 0 {
                    return Err(ge(ln, "a single-surface measure needs Q > 0"));
                }
                let at = axis[ax].unwrap_or(0);
                let from = st.core.pos[ax].unwrap_or(0);
                if at == from {
                    return Err(ge(ln, "a single-surface measure starting on its surface"));
                }
                let end = if at > from {
                    at.checked_add(q)
                } else {
                    at.checked_sub(q)
                }
                .ok_or_else(|| ge(ln, "a coordinate past the word range"))?;
                let mut to = vec![None; axis.len()];
                to[ax] = Some(end);
                let wcs = match s_word {
                    None => None,
                    Some(n @ 1..=6) => Some(n as u32 + 53),
                    Some(n) => return Err(ge(ln, format!("S{n}: only S1..S6 (G54..G59) are modelled"))),
                };
                push(
                    out,
                    Ev::Surface {
                        axis: ax,
                        at,
                        q,
                        to,
                        wcs,
                    },
                );
            }
        }
        st.core.mode = None;
        st.core.cyc = None;
        for e in stop.into_iter().chain(after_stop) {
            push(out, e);
        }
        return Ok(());
    }
    // 5c. a skip (probe) move: the shell's, outside the proven core (docs/cam-machine.md). Its
    // end is unknown, so the moved axes' program position becomes unknown, and the motion mode
    // is cleared (G38.x is modal on LinuxCNC, G31 one-shot on Fanuc: a bare axis word after it
    // is refused rather than read either way).
    if let Some(code) = gs.iter().copied().find(|g| c.skip.contains(g)) {
        if !motion_gs.is_empty() || gs.iter().filter(|g| c.skip.contains(g)).count() > 1 {
            return Err(ge(ln, "a skip move with another motion code in the block"));
        }
        if has_g(28) || get('I').is_some() || get('J').is_some() || get('K').is_some() || get('R').is_some() {
            return Err(ge(ln, "a skip move with arc or home words"));
        }
        if st.feed_mode != FeedMode::PerMin {
            return Err(ge(ln, "a skip move needs G94 (feed per minute)"));
        }
        if !any_axis {
            return Err(ge(ln, "a skip move with no axis word"));
        }
        let abs = absolute.unwrap_or(st.core.absolute);
        let mut to: Vec<Option<Q>> = vec![None; axis.len()];
        for i in 0..axis.len() {
            if let Some(v) = axis[i] {
                to[i] = Some(if abs {
                    v
                } else {
                    let p = st.core.pos[i]
                        .ok_or_else(|| ge(ln, refusal(mcore::Refuse::IncrementalFromUnknown)))?;
                    p.checked_add(v)
                        .ok_or_else(|| ge(ln, "a coordinate past the word range"))?
                });
            }
        }
        // the start must be known on every axis the move names (the segment starts there)
        if (0..axis.len()).any(|i| axis[i].is_some() && st.core.pos[i].is_none()) {
            return Err(ge(ln, "a skip move from an unknown position"));
        }
        let f = f_word
            .or(st.core.feed)
            .ok_or_else(|| ge(ln, "a skip move with no feed"))?;
        if f == 0 {
            return Err(ge(ln, "a skip move at feed 0"));
        }
        st.core.feed = Some(f);
        if let Some(a) = absolute {
            st.core.absolute = a;
        }
        for p in [17u16, 18, 19] {
            if has_g(p) {
                st.core.plane = p as u32;
            }
        }
        st.core.mode = None;
        st.core.cyc = None;
        push(
            out,
            Ev::Skip {
                to: to.clone(),
                f,
                code,
            },
        );
        for (i, t) in to.iter().enumerate() {
            if t.is_some() {
                st.core.pos[i] = None;
            }
        }
        for e in stop.into_iter().chain(after_stop) {
            push(out, e);
        }
        return Ok(());
    }
    if nonmodal_used_words && !any_axis {
        for e in stop.into_iter().chain(after_stop) {
            push(out, e);
        }
        return Ok(());
    }
    let motion_g = motion_gs.first().map(|g| (*g / 10) as u32);
    if !any_axis
        && motion_g.is_none()
        && (get('I').is_some()
            || get('J').is_some()
            || get('K').is_some()
            || (get('R').is_some() && st.core.cyc.is_none()))
    {
        return Err(ge(ln, "arc words with no axis word"));
    }
    // I is X's centre word: on a diameter X it is a radius, the core adds it to the X word
    let mut iw = coord('I')?;
    if c.diameter == Some('X') {
        iw = iw.map(|v| 2 * v);
    }
    let cycle = matches!(motion_g, Some(73 | 81..=85));
    let dwell_p = if cycle {
        match get('P') {
            Some(p) => Some(if c.dwell_seconds {
                parse_q(p, 3, ln)?
            } else {
                parse_q(p, 0, ln)?
            }),
            None => None,
        }
    } else {
        None
    };
    let plane = if has_g(19) {
        Some(19)
    } else if has_g(18) {
        Some(18)
    } else if has_g(17) {
        Some(17)
    } else {
        None
    };
    let b = mcore::Block {
        g: motion_g,
        absolute,
        retract_initial: if has_g(99) {
            Some(false)
        } else if has_g(98) {
            Some(true)
        } else {
            None
        },
        plane,
        home: has_g(28),
        ax: axis,
        i: iw,
        j: coord('J')?,
        k: coord('K')?,
        r: coord('R')?,
        q: if cycle { coord('Q')? } else { None },
        p: dwell_p,
        f: f_word,
    };
    let axes = mcore::Axes {
        x: c.idx('X').unwrap_or(usize::MAX),
        y: c.idx('Y').unwrap_or(usize::MAX),
        z: c.idx('Z').unwrap_or(usize::MAX),
    };
    // the machine's peck clearance and chip-break lift (forge-post's when the record is silent)
    let (pc, lift) = if c.inch {
        (
            forge_post::quant_ratio(c.peck_clear, k, 10, 254).unwrap_or(0),
            forge_post::quant_ratio(c.chip_lift, k, 10, 254).unwrap_or(0),
        )
    } else {
        (
            forge_post::quant(c.peck_clear, k).unwrap_or(0),
            forge_post::quant(c.chip_lift, k).unwrap_or(0),
        )
    };
    let (evs, next) = mcore::step(&axes, pc, lift, &st.core, &b).map_err(|r| ge(ln, refusal(r)))?;
    st.core = next;
    let mode = st.feed_mode;
    for e in evs {
        push(
            out,
            match e {
                mcore::Ev::Rapid(v) => Ev::Rapid(v),
                mcore::Ev::Feed(v, f) => Ev::Feed { to: v, f, mode },
                mcore::Ev::Arc(to, plane, centre, cw, full, f) => Ev::Arc {
                    to,
                    plane: plane as u8,
                    centre: match centre {
                        mcore::Centre::Abs(a, b) => Centre::Abs([a, b]),
                        mcore::Centre::R(r) => Centre::R(r),
                    },
                    cw,
                    full,
                    f,
                    mode,
                },
                mcore::Ev::Home(via, axes) => Ev::Home { via, axes },
                mcore::Ev::Dwell(ms) => Ev::Dwell { ms },
            },
        );
    }
    for e in stop.into_iter().chain(after_stop) {
        push(out, e);
    }
    Ok(())
}

/// A refusal's words.
fn refusal(r: mcore::Refuse) -> &'static str {
    use mcore::Refuse::*;
    match r {
        IncrementalFromUnknown => "an incremental move from an unknown position",
        HomeNoAxis => "G28 with no axis word (which axes go home is not assumed)",
        UnknownMotion => "a motion code not in the subset",
        CycleNotG17 => "canned cycles are modelled in G17 only",
        CycleNoZ => "a cycle with no Z",
        CycleNoR => "a cycle with no R",
        CycleIncremental => "a cycle in G91 is not modelled",
        CycleUnknownHeight => "a cycle from an unknown height",
        CycleOrder => "a cycle needs bottom Z ≤ R ≤ the initial height",
        CycleNegativePeck => "a negative peck",
        CycleZRepeat => "a Z word in a cycle's repeat block changes the bottom: write the cycle again",
        HoleUnknown => "a hole with no X, Y or feed",
        NoFeed => "a feed move with no F (F0)",
        ArcUnknown => "an arc from an unknown position (or in a plane this control has no axis for)",
        ArcR0 => "an arc with R0",
        ArcFullR => "a full circle cannot be written with R",
        ArcRAndCentre => "an arc with both R and a centre word",
        ArcNoCentre => "an arc with no centre (I J K) or R",
        NoMotion => "coordinates with no motion mode",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evs(src: &str) -> Vec<Ev> {
        run(src, &Control::mill3(3, 1, false))
            .unwrap()
            .0
            .into_iter()
            .map(|b| b.ev)
            .collect()
    }

    #[test]
    fn modal_and_incremental() {
        let e = evs("G90 G0 X1. Y2. Z3.\nX4.\nG91 G1 Y-1. F100.\nG90 Z0");
        assert_eq!(e[0], Ev::Rapid(vec![Some(1000), Some(2000), Some(3000)]));
        assert_eq!(e[1], Ev::Rapid(vec![Some(4000), Some(2000), Some(3000)]));
        assert_eq!(
            e[2],
            Ev::Feed {
                to: vec![Some(4000), Some(1000), Some(3000)],
                f: 1000,
                mode: FeedMode::PerMin
            }
        );
        assert_eq!(
            e[3],
            Ev::Feed {
                to: vec![Some(4000), Some(1000), Some(0)],
                f: 1000,
                mode: FeedMode::PerMin
            }
        );
    }

    #[test]
    fn refusals_are_typed() {
        let c = Control::mill3(3, 1, false);
        for (src, has) in [
            ("G20", "inch"),
            ("G41 D1", "cutter compensation"),
            ("G1 X1.", "F0"),
            ("G0 X1.0001", "digits"),
            ("G0 X1. X2.", "twice"),
            ("G0 G1 X1.", "two motion"),
            ("G91 G0 X1.", "unknown position"),
            ("G0 A1.", "unknown word"),
            ("M6", "no tool selected"),
            ("G0 X0 Y0 Z0\nG2 X1. Y1. F10.", "no centre"),
        ] {
            let e = run(src, &c).unwrap_err();
            assert!(e.what.contains(has), "{src}: {}", e.what);
        }
    }

    #[test]
    fn home_through_intermediate() {
        let e = evs("G0 X1. Y2. Z3.\nG91 G28 Z0.\nG90");
        assert_eq!(
            e[1],
            Ev::Home {
                via: vec![Some(1000), Some(2000), Some(3000)],
                axes: vec![2]
            }
        );
    }

    #[test]
    fn cycle_g99_returns_to_r() {
        let e = evs("G0 X0 Y0 Z10.\nG99 G81 X1. Y1. Z-2. R2. F50.\nX2.\nG80");
        // hole 1: rapid xy, rapid R, feed Z, rapid R (G99)
        assert_eq!(e.len(), 1 + 4 + 4);
        assert_eq!(e[4], Ev::Rapid(vec![Some(1000), Some(1000), Some(2000)]));
        assert_eq!(e[5], Ev::Rapid(vec![Some(2000), Some(1000), Some(2000)]));
    }
}

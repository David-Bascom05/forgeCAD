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

/// What the interpreter needs to know about the control and the machine's words.
#[derive(Clone, Debug, PartialEq)]
pub struct Control {
    pub decimals: u32,
    pub feed_decimals: u32,
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
}

impl Control {
    /// A 3-axis mill control on X Y Z with forge-post's numbers for a dialect.
    pub fn mill3(decimals: u32, feed_decimals: u32, dwell_seconds: bool) -> Control {
        Control {
            decimals,
            feed_decimals,
            letters: vec!['X', 'Y', 'Z'],
            rotary: vec![false; 3],
            diameter: None,
            dwell_seconds,
            lathe: false,
            plane0: 17,
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
        let dwell = toml::text(d, "dwell").ok_or("the dialect has no dwell template")?;
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
        let lathe = matches!(def.kind, crate::def::Kind::Lathe | crate::def::Kind::MillTurn);
        Ok(Control {
            decimals,
            feed_decimals,
            letters,
            rotary,
            diameter,
            dwell_seconds: !dwell.contains("{ms}"),
            lathe,
            plane0: if lathe { 18 } else { 17 },
        })
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

    fn idx(&self, c: char) -> Option<usize> {
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
    /// The centre is the start plus (I, J) / (I, K) / (J, K): plane coordinates, radius units.
    Offset([Q; 2]),
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// 1-based line in the text.
    pub line: usize,
    pub ev: Ev,
}

/// The modal state between blocks.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub pos: Vec<Option<Q>>,
    pub absolute: bool,
    /// 0 1 2 3 or 80 (a cycle is active) or None.
    pub motion: Option<u8>,
    pub plane: u8,
    pub feed_mode: FeedMode,
    pub feed: Q,
    pub rpm: Q,
    pub css: bool,
    /// G98 (true): back to the initial level; G99: to R.
    pub retract_initial: bool,
    pub cycle: Option<Cycle>,
    pub selected: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cycle {
    pub g: u8,
    pub z: Q,
    pub r: Q,
    pub q: Q,
    pub dwell_ms: Q,
    /// The height the cycle started from (G98's return level).
    pub initial: Q,
}

impl State {
    pub fn new(c: &Control) -> State {
        State {
            pos: vec![None; c.letters.len()],
            absolute: true,
            motion: None,
            plane: c.plane0,
            feed_mode: FeedMode::PerMin,
            feed: 0,
            rpm: 0,
            css: false,
            retract_initial: true,
            cycle: None,
            selected: None,
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
fn words(raw: &str, line: usize, comments: &mut Vec<String>) -> Result<Option<Vec<(char, String)>>, GError> {
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
    if s.is_empty() || s == "%" {
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
        if v.is_empty() {
            return Err(ge(line, format!("the word `{l}` has no value")));
        }
        out.push((l, v));
        i = j;
    }
    Ok(Some(out))
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
    let parsed = words(raw, ln, &mut comments)?;
    let push = |out: &mut Vec<Block>, ev: Ev| out.push(Block { line: ln, ev });
    for c in comments {
        push(out, Ev::Comment(c));
    }
    let Some(ws) = parsed else { return Ok(()) };
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
        let ok = matches!(
            *g,
            0 | 10
                | 20
                | 30
                | 40
                | 170
                | 180
                | 190
                | 210
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
        if !ok {
            let what = match g {
                200 => "G20 (inch) is not modelled: post in mm".to_string(),
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
    if let Some(f) = get('F') {
        st.feed = parse_q(f, c.feed_decimals, ln)?;
        if st.feed < 0 {
            return Err(ge(ln, "a negative feed"));
        }
    }
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
    } else if let Some(s) = s_word {
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
            m => return Err(ge(ln, format!("M{m} is not in the subset"))),
        }
    }
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
    // 4. plane, comp, length, wcs, distance, retract
    for (g, p) in [(17u16, 17u8), (18, 18), (19, 19)] {
        if has_g(g) {
            st.plane = p;
        }
    }
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
    if has_g(90) {
        st.absolute = true;
    }
    if has_g(91) {
        st.absolute = false;
    }
    if has_g(98) {
        st.retract_initial = true;
    }
    if has_g(99) {
        st.retract_initial = false;
    }
    // the axis words of this block
    let mut axis: Vec<Option<Q>> = vec![None; c.letters.len()];
    for (i, l) in c.letters.iter().enumerate() {
        axis[i] = coord(*l)?;
    }
    let any_axis = axis.iter().any(|a| a.is_some());
    // the target: absolute words replace, incremental add to a known position
    let target = |st: &State| -> Result<Vec<Option<Q>>, GError> {
        let mut to = st.pos.clone();
        for i in 0..axis.len() {
            if let Some(v) = axis[i] {
                to[i] = Some(if st.absolute {
                    v
                } else {
                    st.pos[i].ok_or_else(|| {
                        ge(
                            ln,
                            format!("an incremental {} from an unknown position", c.letters[i]),
                        )
                    })? + v
                });
            }
        }
        Ok(to)
    };
    // 5. G28, G53
    if has_g(28) {
        let via = target(st)?;
        let axes: Vec<usize> = (0..axis.len()).filter(|i| axis[*i].is_some()).collect();
        if axes.is_empty() {
            return Err(ge(
                ln,
                "G28 with no axis word (which axes go home is not assumed)",
            ));
        }
        push(
            out,
            Ev::Home {
                via: via.clone(),
                axes: axes.clone(),
            },
        );
        st.pos = via;
        for i in axes {
            st.pos[i] = None; // the reference point in program coordinates depends on offsets
        }
        if let Some(e) = stop {
            push(out, e);
        }
        return Ok(());
    }
    if has_g(53) {
        if !st.absolute {
            return Err(ge(ln, "G53 in G91: machine moves are absolute"));
        }
        let mv: Vec<(usize, Q)> = (0..axis.len()).filter_map(|i| axis[i].map(|v| (i, v))).collect();
        push(out, Ev::Machine(mv.clone()));
        for (i, _) in mv {
            st.pos[i] = None;
        }
        if let Some(e) = stop {
            push(out, e);
        }
        return Ok(());
    }
    // 6. motion
    if let Some(&g) = motion_gs.first() {
        match g {
            0 | 10 | 20 | 30 => {
                st.motion = Some((g / 10) as u8);
                st.cycle = None;
            }
            800 => {
                st.motion = None;
                st.cycle = None;
            }
            _ => {
                if st.plane != 17 {
                    return Err(ge(ln, "canned cycles are modelled in G17 only"));
                }
                let zi = c
                    .idx('Z')
                    .ok_or_else(|| ge(ln, "a cycle on a control with no Z"))?;
                let z = coord('Z')?.ok_or_else(|| ge(ln, "a cycle with no Z"))?;
                let r = coord('R')?.ok_or_else(|| ge(ln, "a cycle with no R"))?;
                if !st.absolute {
                    return Err(ge(ln, "a cycle in G91 is not modelled"));
                }
                let q = coord('Q')?.unwrap_or(0);
                let dwell_ms = match get('P') {
                    Some(p) => {
                        if c.dwell_seconds {
                            parse_q(p, 3, ln)?
                        } else {
                            parse_q(p, 0, ln)?
                        }
                    }
                    None => 0,
                };
                let initial = st.pos[zi].ok_or_else(|| ge(ln, "a cycle from an unknown height"))?;
                if r > initial || z > r {
                    return Err(ge(ln, "a cycle needs bottom Z ≤ R ≤ the initial height"));
                }
                if q < 0 {
                    return Err(ge(ln, "a negative peck"));
                }
                st.cycle = Some(Cycle {
                    g: (g / 10) as u8,
                    z,
                    r,
                    q,
                    dwell_ms,
                    initial,
                });
                st.motion = Some(80);
            }
        }
    }
    if nonmodal_used_words && !any_axis {
        if let Some(e) = stop {
            push(out, e);
        }
        return Ok(());
    }
    if has_g(43) && !any_axis {
        if let Some(e) = stop {
            push(out, e);
        }
        return Ok(());
    }
    if !any_axis {
        if motion_gs.is_empty()
            && (get('I').is_some()
                || get('J').is_some()
                || get('K').is_some()
                || (get('R').is_some() && st.cycle.is_none()))
        {
            return Err(ge(ln, "arc words with no axis word"));
        }
        if let Some(e) = stop {
            push(out, e);
        }
        return Ok(());
    }
    let to = target(st)?;
    match st.motion {
        Some(80) => {
            let cy = st.cycle.unwrap();
            let xi = c.idx('X').ok_or_else(|| ge(ln, "a cycle with no X axis"))?;
            let yi = c.idx('Y').ok_or_else(|| ge(ln, "a cycle with no Y axis"))?;
            let zi = c.idx('Z').unwrap();
            if axis[zi].is_some() && motion_gs.is_empty() {
                return Err(ge(
                    ln,
                    "a Z word in a cycle's repeat block changes the bottom: write the cycle again",
                ));
            }
            let (x, y) = (
                to[xi].ok_or_else(|| ge(ln, "a hole with no X"))?,
                to[yi].ok_or_else(|| ge(ln, "a hole with no Y"))?,
            );
            let back = if st.retract_initial { cy.initial } else { cy.r };
            let mut base = to.clone();
            base[zi] = None;
            let at = |z: Q| {
                let mut v = base.clone();
                v[xi] = Some(x);
                v[yi] = Some(y);
                v[zi] = Some(z);
                v
            };
            let f = st.feed;
            let mode = st.feed_mode;
            let rapid = |out: &mut Vec<Block>, z: Q| {
                out.push(Block {
                    line: ln,
                    ev: Ev::Rapid(at(z)),
                })
            };
            let feed = |out: &mut Vec<Block>, z: Q| {
                out.push(Block {
                    line: ln,
                    ev: Ev::Feed { to: at(z), f, mode },
                })
            };
            // as forge-post: to (x, y) at the current height, then R
            rapid(out, st.pos[zi].unwrap());
            rapid(out, cy.r);
            match cy.g {
                81 | 82 | 85 | 84 => {
                    feed(out, cy.z);
                    if cy.g == 82 && cy.dwell_ms > 0 {
                        push(out, Ev::Dwell { ms: cy.dwell_ms });
                    }
                    if cy.g == 85 || cy.g == 84 {
                        feed(out, cy.r);
                    }
                }
                83 | 73 => {
                    let step = if cy.q > 0 { cy.q } else { cy.r - cy.z };
                    let pc = forge_post::quant(forge_post::PECK_CLEAR, k).unwrap_or(0);
                    let lift = forge_post::quant(forge_post::CHIP_LIFT, k).unwrap_or(0);
                    let mut depth = cy.r;
                    while depth > cy.z {
                        let next = if depth - step > cy.z { depth - step } else { cy.z };
                        feed(out, next);
                        depth = next;
                        if depth > cy.z {
                            if cy.g == 83 {
                                rapid(out, cy.r);
                                rapid(out, depth + pc);
                            } else {
                                rapid(out, depth + lift);
                            }
                        }
                    }
                }
                _ => unreachable!(),
            }
            rapid(out, back);
            st.pos = at(back);
        }
        Some(0) => {
            push(out, Ev::Rapid(to.clone()));
            st.pos = to;
        }
        Some(1) => {
            if st.feed == 0 {
                return Err(ge(ln, "a feed move with F0 (or no F yet)"));
            }
            push(
                out,
                Ev::Feed {
                    to: to.clone(),
                    f: st.feed,
                    mode: st.feed_mode,
                },
            );
            st.pos = to;
        }
        Some(g @ (2 | 3)) => {
            if st.feed == 0 {
                return Err(ge(ln, "an arc with F0 (or no F yet)"));
            }
            let (a, b, wa, wb) = match st.plane {
                17 => ('X', 'Y', 'I', 'J'),
                18 => ('Z', 'X', 'K', 'I'),
                _ => ('Y', 'Z', 'J', 'K'),
            };
            let ai = c
                .idx(a)
                .ok_or_else(|| ge(ln, format!("an arc in a plane with no {a} axis")))?;
            let bi = c
                .idx(b)
                .ok_or_else(|| ge(ln, format!("an arc in a plane with no {b} axis")))?;
            let (sa, sb) = (
                st.pos[ai].ok_or_else(|| ge(ln, "an arc from an unknown position"))?,
                st.pos[bi].ok_or_else(|| ge(ln, "an arc from an unknown position"))?,
            );
            let (ea, eb) = (to[ai].unwrap(), to[bi].unwrap());
            let centre = match (coord('R')?, coord(wa)?, coord(wb)?) {
                (Some(r), None, None) => {
                    if r == 0 {
                        return Err(ge(ln, "an arc with R0"));
                    }
                    if ea == sa && eb == sb {
                        return Err(ge(ln, "a full circle cannot be written with R"));
                    }
                    Centre::R(r)
                }
                (None, ia, ib) if ia.is_some() || ib.is_some() => {
                    Centre::Offset([ia.unwrap_or(0), ib.unwrap_or(0)])
                }
                (Some(_), _, _) => return Err(ge(ln, "an arc with both R and a centre word")),
                _ => return Err(ge(ln, "an arc with no centre (I J K) or R")),
            };
            for (i, t) in to.iter().enumerate() {
                if i != ai && i != bi && t != &st.pos[i] && c.rotary[i] {
                    return Err(ge(ln, "a rotary axis moving during an arc is not modelled"));
                }
            }
            push(
                out,
                Ev::Arc {
                    to: to.clone(),
                    plane: st.plane,
                    centre,
                    cw: g == 2,
                    full: ea == sa && eb == sb,
                    f: st.feed,
                    mode: st.feed_mode,
                },
            );
            st.pos = to;
        }
        _ => return Err(ge(ln, "coordinates with no motion mode")),
    }
    if let Some(e) = stop {
        push(out, e);
    }
    Ok(())
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

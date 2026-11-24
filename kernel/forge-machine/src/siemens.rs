//! SINUMERIK 840D's native language, the subset NX writes for the Spinner U5-620 programs of the
//! open dataset (data/controls/siemens-840d.toml cites every construct): each block rewritten into
//! the Fanuc words the interpreter core reads, with the 840D's own state kept here. What is not in
//! the subset is refused, naming it.
//!
//! - `DEF REAL a, b` declares variables; `a=1.5` assigns a number (no expressions); an axis word
//!   `X=a` takes the variable's value (PGAsl 1.1.4).
//! - `SUPA G0 …` moves in machine coordinates with every frame suppressed, the tool offset not
//!   (PGsl 8.1, 12.9): a SUPA move of the length axis is read only with D0 in force.
//! - `T="name"` selects a tool of the setup by its name, `M6` loads it, `T0` deselects
//!   (an empty spindle) (PGsl 4.2.2). `D1` takes the loaded tool's length, `D0` none; after `M6`
//!   the length is machine data until a D is programmed, so a Z move before one is refused
//!   (PGsl 5.6). Only D0 and D1 (one cutting edge) are read.
//! - `MCALL CYCLE81(RTP, RFP, SDIS, DP)` runs the drilling cycle after each block with path
//!   motion until `MCALL` (PGAsl 1.24.3.4), as its documented motion: G0 to RFP shifted by SDIS
//!   toward RTP, G1 to DP at the modal feed, G0 to RTP (PGZ 2.1.3). The motion mode after it is
//!   not assumed: the next move must state its G.
//! - `CYCLE832(…)` and `CYCLE800()` move nothing here and are reported not modelled (their
//!   effect is the machine's settings: smoothing and compression; a swivel deselection).
//! - `MSG("…")` is a comment; `COMPOF`, `TRAFOOF` (no transformation is active in this subset),
//!   `FNORM`, `G60` `G601` (exact stop: what t_stop models) change nothing here; `G710` is G21;
//!   `TURN=0` is the default single-turn helix.
//! - Machine-specific M codes are read only where the setup names them (`m_no_motion`), and are
//!   reported as the setup's word.
use crate::gcode::{Control, Ev, GError, State, block_fanuc, q_text};

/// The 840D's own state: variables, the modal cycle call, the tool offset.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct S840 {
    /// Declared REAL variables and their value (decimal text) once assigned.
    pub vars: Vec<(String, Option<String>)>,
    /// MCALL CYCLE81's planes: RTP, RFP, SDIS, DP (decimal text).
    pub mcall: Option<[String; 4]>,
    /// The tool loaded by the last M6 (0: none).
    pub loaded: Option<u32>,
    /// After M6, no D yet: the length is machine data.
    pub d_unknown: bool,
    /// The active D (0 or 1).
    pub d: u32,
    /// NORM stated (group 17: the approach and retract of compensation, PGsl 10.2).
    pub norm: bool,
    /// G450 (Some(true): a circle about an outside corner) or G451 (Some(false): the
    /// equidistants' intersection), once stated (group 18, PGsl 10.3).
    pub corner: Option<bool>,
    /// Cutter radius compensation on (G41/G42).
    pub comp: bool,
    /// TRAORI on (3+2: the rotary axes held) and the programmable frame's rotation.
    pub traori: bool,
    pub frame: Option<[[f64; 3]; 3]>,
}

const I3: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// a · b (3 × 3).
fn mat_mul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut o = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            o[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    o
}

/// A right-handed rotation by `deg` about a coordinate axis (exact at multiples of 90°).
fn axis_rot(axis: char, deg: f64) -> [[f64; 3]; 3] {
    let (s, c) = crate::kin::sin_cos_deg(deg);
    match axis {
        'X' => [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]],
        'Y' => [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]],
        _ => [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]],
    }
}

fn err(line: usize, what: impl Into<String>) -> GError {
    GError {
        line,
        what: what.into(),
    }
}

/// A block split into tokens at spaces, keeping quoted text and parentheses whole.
fn tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quote, mut depth) = (false, 0i32);
    for ch in s.chars() {
        match ch {
            '"' => {
                quote = !quote;
                cur.push(ch);
            }
            '(' if !quote => {
                depth += 1;
                cur.push(ch);
            }
            ')' if !quote => {
                depth -= 1;
                cur.push(ch);
            }
            c if c.is_whitespace() && !quote && depth == 0 => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// A decimal literal (`-365.`, `.02`, `10`), as written.
fn is_number(s: &str) -> bool {
    let b = s.strip_prefix(['-', '+']).unwrap_or(s);
    let (i, f) = b.split_once('.').unwrap_or((b, ""));
    !(i.is_empty() && f.is_empty())
        && i.bytes().all(|c| c.is_ascii_digit())
        && f.bytes().all(|c| c.is_ascii_digit())
}

/// The arguments of `NAME(a,b,…)`, or None if the token is not that call.
fn call<'a>(tok: &'a str, name: &str) -> Option<Vec<&'a str>> {
    let rest = tok.strip_prefix(name)?.strip_prefix('(')?.strip_suffix(')')?;
    if rest.trim().is_empty() {
        return Some(Vec::new());
    }
    Some(rest.split(',').map(str::trim).collect())
}

impl S840 {
    fn value(&self, v: &str, ln: usize) -> Result<String, GError> {
        if is_number(v) {
            return Ok(v.to_string());
        }
        match self.vars.iter().find(|(n, _)| n == v) {
            Some((_, Some(x))) => Ok(x.clone()),
            Some((_, None)) => Err(err(
                ln,
                format!("the variable `{v}` is used before it is assigned"),
            )),
            None => Err(err(
                ln,
                format!("`{v}` is not a number or a declared variable (expressions are not read)"),
            )),
        }
    }
}

/// A decimal text in integer units of 10^-k.
fn q(v: &str, k: u32, ln: usize) -> Result<i64, GError> {
    crate::gcode::parse_q(v, k, ln)
}

/// One native 840D block: rewritten into Fanuc lines run through the core, with the 840D's own
/// state advanced and its notes pushed.
pub fn block(
    raw: &str,
    ln: usize,
    c: &Control,
    st: &mut State,
    out: &mut Vec<crate::gcode::Block>,
) -> Result<(), GError> {
    let push = |out: &mut Vec<crate::gcode::Block>, ev: Ev| out.push(crate::gcode::Block { line: ln, ev });
    // the comment after `;` (outside a string)
    let mut body = String::new();
    let mut quote = false;
    let mut comment = None;
    for (i, ch) in raw.char_indices() {
        if ch == '"' {
            quote = !quote;
        }
        if ch == ';' && !quote {
            comment = Some(raw[i + 1..].trim().to_string());
            break;
        }
        body.push(ch);
    }
    if let Some(cm) = comment.filter(|c| !c.is_empty()) {
        push(out, Ev::Comment(cm));
    }
    let mut toks = tokens(body.trim());
    // the block number
    let n_word = match toks.first() {
        Some(t) if t.starts_with('N') && t.len() > 1 && t[1..].bytes().all(|b| b.is_ascii_digit()) => {
            Some(toks.remove(0))
        }
        _ => None,
    };
    if toks.is_empty() {
        return Ok(());
    }
    let k = c.decimals;
    let s = &mut st.s840;
    // DEF REAL a, b, …
    if toks[0] == "DEF" {
        if toks.get(1).map(String::as_str) != Some("REAL") {
            return Err(err(ln, "only DEF REAL is read (other variable types are not)"));
        }
        for name in toks[2..]
            .join(" ")
            .split(',')
            .map(str::trim)
            .filter(|n| !n.is_empty())
        {
            if !name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                || name.chars().next().is_some_and(|ch| ch.is_ascii_digit())
            {
                return Err(err(ln, format!("DEF REAL `{name}`: not a plain variable name")));
            }
            if s.vars.iter().any(|(n, _)| n == name) {
                return Err(err(ln, format!("DEF REAL `{name}` twice")));
            }
            s.vars.push((name.to_string(), None));
        }
        return Ok(());
    }
    // TRAORI / TRAFOOF alone in a block; frames (TRANS, AROT), each in its own block (PGsl 12.2)
    match toks[0].as_str() {
        "TRAORI" if toks.len() == 1 => {
            // PGAsl 6.2: the orientation transformation; read with the rotary axes held (3+2)
            if s.traori {
                return Err(err(ln, "TRAORI while TRAORI is on"));
            }
            if s.comp {
                return Err(err(ln, "TRAORI with cutter compensation on is not modelled"));
            }
            s.traori = true;
            s.frame = Some(I3);
            push(out, Ev::Frame(Some(I3)));
            return Ok(());
        }
        "TRAFOOF" if toks.len() == 1 => {
            if s.traori {
                s.traori = false;
                s.frame = None;
                push(out, Ev::Frame(None));
            }
            return Ok(());
        }
        "TRANS" | "AROT" => {
            if !s.traori {
                return Err(err(
                    ln,
                    format!(
                        "`{}` outside TRAORI: frames are read only for 3+2 (TRAORI) machining",
                        toks[0]
                    ),
                ));
            }
            let f = s.frame.unwrap_or(I3);
            let f = if toks[0] == "TRANS" {
                // TRANS resets every component of the programmable frame (PGsl 12.3.1 p. 329): only
                // TRANS X0 Y0 Z0 (no offset) is read
                for t in &toks[1..] {
                    let mut ch = t.chars();
                    let ok = matches!(ch.next(), Some('X' | 'Y' | 'Z'))
                        && is_number(ch.as_str())
                        && ch.as_str().parse::<f64>().is_ok_and(|v| v == 0.0);
                    if !ok {
                        return Err(err(
                            ln,
                            format!("`TRANS {t}`: only TRANS X0 Y0 Z0 (a reset) is read"),
                        ));
                    }
                }
                I3
            } else {
                // AROT: additive, about the current frame's axis (PGsl 12.4 pp. 333-334); one axis per
                // block (several in one block turn in RPY order, not read)
                if toks.len() != 2 {
                    return Err(err(ln, "AROT with other than one axis in a block is not read"));
                }
                let t = &toks[1];
                let mut ch = t.chars();
                let axis = ch.next().filter(|a| matches!(a, 'X' | 'Y' | 'Z'));
                let deg = ch.as_str();
                let (Some(axis), true) = (axis, is_number(deg)) else {
                    return Err(err(ln, format!("`AROT {t}`: an axis X Y Z and a number")));
                };
                mat_mul(&f, &axis_rot(axis, deg.parse::<f64>().unwrap_or(0.0)))
            };
            s.frame = Some(f);
            push(out, Ev::Frame(Some(f)));
            return Ok(());
        }
        _ => {}
    }
    // a block of assignments to declared variables
    let declared: Vec<String> = s.vars.iter().map(|(n, _)| n.clone()).collect();
    let assign = |t: &str| -> Option<(String, String)> {
        let (n, v) = t.split_once('=')?;
        declared
            .iter()
            .any(|m| m == n)
            .then(|| (n.to_string(), v.to_string()))
    };
    if toks.iter().all(|t| assign(t).is_some()) {
        let pairs: Vec<(String, String)> = toks.iter().map(|t| assign(t).unwrap()).collect();
        for (n, v) in pairs {
            if !is_number(&v) {
                return Err(err(
                    ln,
                    format!("`{n}={v}`: only a number is read (expressions are not)"),
                ));
            }
            let slot = s.vars.iter_mut().find(|(m, _)| *m == n).unwrap();
            slot.1 = Some(v);
        }
        return Ok(());
    }
    if toks.iter().any(|t| assign(t).is_some()) {
        return Err(err(
            ln,
            "a variable assignment with other words in the block is not read",
        ));
    }
    let mut words: Vec<String> = Vec::new();
    let mut supa = false;
    let mut d_word: Option<u32> = None;
    let mut m6 = false;
    let mut mcall_set: Option<Option<[String; 4]>> = None;
    // G41 (Some(true)), G42 (Some(false)) or G40 (None) in this block
    let mut comp_set: Option<Option<bool>> = None;
    let mut notes: Vec<(String, &'static str)> = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i].as_str();
        i += 1;
        if let Some(args) = call(t, "MSG") {
            let text = args.join(",");
            let text = text.trim().trim_matches('"').to_string();
            push(out, Ev::Comment(text));
            continue;
        }
        if let Some(args) = call(t, "CYCLE832") {
            if args.len() > 3 {
                return Err(err(
                    ln,
                    "CYCLE832 with more than (tolerance, mode, …) is not read",
                ));
            }
            for a in args.iter().filter(|a| !a.is_empty()) {
                s.value(a, ln)?;
            }
            notes.push((
                "cycle832".into(),
                "CYCLE832's smoothing, compression and velocity control (the machine's settings, PGZ 3.17): the time under it is not modelled; t_min is a bound only to within its tolerance, t_stop assumes exact stops",
            ));
            continue;
        }
        if let Some(args) = call(t, "CYCLE800") {
            if !args.is_empty() {
                return Err(err(ln, "CYCLE800 with parameters (a swivel) is not modelled"));
            }
            notes.push((
                "cycle800".into(),
                "CYCLE800() deselects the swivel data record and deletes its frames (PGZ 3.16): taken as no motion (a machine's cycle settings may retract)",
            ));
            continue;
        }
        if t == "MCALL" {
            match toks.get(i).map(String::as_str) {
                None => mcall_set = Some(None),
                Some(next) => {
                    let args = call(next, "CYCLE81")
                        .ok_or_else(|| err(ln, format!("MCALL `{next}`: only CYCLE81 is read")))?;
                    if args.len() != 4 || args.iter().any(|a| a.is_empty()) {
                        return Err(err(
                            ln,
                            "MCALL CYCLE81 needs RTP, RFP, SDIS, DP all given (DPR is not read)",
                        ));
                    }
                    let v: Vec<String> = args.iter().map(|a| s.value(a, ln)).collect::<Result<_, _>>()?;
                    mcall_set = Some(Some([v[0].clone(), v[1].clone(), v[2].clone(), v[3].clone()]));
                    i += 1;
                }
            }
            continue;
        }
        match t {
            // cutter radius compensation (PGsl 10.1-10.3)
            "G40" | "G41" | "G42" => {
                comp_set = Some(match t {
                    "G41" => Some(true),
                    "G42" => Some(false),
                    _ => None,
                });
                continue;
            }
            "NORM" => {
                s.norm = true;
                continue;
            }
            "G450" | "G451" => {
                let arc = t == "G450";
                if s.comp && s.corner != Some(arc) {
                    return Err(err(
                        ln,
                        format!(
                            "`{t}` while compensation is on: a change of corner behaviour is not modelled"
                        ),
                    ));
                }
                s.corner = Some(arc);
                continue;
            }
            // DISC=0 is G450's transition circle itself; OFFN=0 no allowance (PGsl 10.3, 10.1);
            // the collision detection setting matters only where compensated blocks do not meet,
            // which is refused (PGsl 10.5)
            "DISC=0" | "OFFN=0" | "CDON" | "CDOF" | "CDOF2" => continue,
            "KONT" | "KONTC" | "KONTT" => {
                return Err(err(
                    ln,
                    format!("`{t}`: only NORM's approach and retract are read (PGsl 10.2)"),
                ));
            }
            "SUPA" => {
                supa = true;
                continue;
            }
            // no effect here: no compressor (COMPOF), no transformation active (TRAFOOF), the
            // default feed characteristic (FNORM), exact stop (what t_stop models)
            "COMPOF" | "FNORM" | "G60" | "G601" => continue,
            "TRAFOOF" if !s.traori => continue,
            "G710" => {
                words.push("G21".into());
                continue;
            }
            "TURN=0" => continue,
            "TRAORI" | "TRAFOOF" | "TRANS" | "AROT" => {
                return Err(err(
                    ln,
                    format!("`{t}` with other words in its block is not read"),
                ));
            }
            "ATRANS" | "ROT" | "AROTS" | "MIRROR" | "SCALE" | "COMPON" | "COMPCURV" | "COMPCAD" | "G64"
            | "G641" | "G642" | "G645" | "FFWON" | "FFWOF" | "SOFT" | "BRISK" | "TRAANG" | "TRACYL"
            | "TRANSMIT" | "ORIWKS" | "ORIMKS" => {
                return Err(err(ln, format!("`{t}` is not in the 840D subset read here")));
            }
            _ => {}
        }
        if let Some(name) = t.strip_prefix("T=") {
            let name = name
                .strip_prefix('"')
                .and_then(|n| n.strip_suffix('"'))
                .ok_or_else(|| err(ln, format!("`{t}`: a tool by name is T=\"name\"")))?;
            let (_, n) = c
                .tool_names
                .iter()
                .find(|(nm, _)| nm == name)
                .ok_or_else(|| err(ln, format!("no tool named \"{name}\" in the setup")))?;
            words.push(format!("T{n}"));
            continue;
        }
        if t == "T0" {
            words.push("T0".into());
            continue;
        }
        if let Some(rest) = t.strip_prefix('D')
            && !rest.is_empty()
            && rest.bytes().all(|b| b.is_ascii_digit())
        {
            let d: u32 = rest.parse().map_err(|_| err(ln, format!("`{t}`")))?;
            if d > 1 {
                return Err(err(
                    ln,
                    format!("D{d}: only D0 and D1 (one cutting edge) are read"),
                ));
            }
            d_word = Some(d);
            continue;
        }
        if let Some(rest) = t.strip_prefix("TURN=") {
            return Err(err(
                ln,
                format!("TURN={rest}: a helix of more than one turn is not read"),
            ));
        }
        if let Some(rest) = t.strip_prefix('M')
            && !rest.is_empty()
            && rest.bytes().all(|b| b.is_ascii_digit())
        {
            let m: u32 = rest.parse().map_err(|_| err(ln, format!("`{t}`")))?;
            if c.m_no_motion.contains(&m) {
                notes.push((
                    format!("M{m}"),
                    "a machine-specific M code the setup names as moving nothing (m_no_motion)",
                ));
                continue;
            }
            if m == 6 {
                m6 = true;
            }
            words.push(t.to_string());
            continue;
        }
        // AXIS=value: a declared variable's value or a number
        if let Some((ax, v)) = t.split_once('=') {
            let mut ch = ax.chars();
            if let (Some(l), None) = (ch.next(), ch.next())
                && c.letters.contains(&l)
            {
                if v.starts_with("AC(") || v.starts_with("IC(") {
                    return Err(err(ln, format!("`{t}`: AC/IC are not read")));
                }
                let val = s.value(v, ln)?;
                words.push(format!("{l}{val}"));
                continue;
            }
            return Err(err(ln, format!("`{t}` is not in the 840D subset read here")));
        }
        // what is left is an address word: one letter and a number; anything else (IF, GOTOF,
        // a call, an expression, a keyword) is not in the subset, refused by name
        let mut ch = t.chars();
        let addr = ch.next().is_some_and(|l| l.is_ascii_alphabetic()) && is_number(ch.as_str());
        if !addr {
            return Err(err(ln, format!("`{t}` is not in the 840D subset read here")));
        }
        words.push(t.to_string());
    }
    // the tool offset: D in force for this block
    let d_now = d_word.unwrap_or(s.d);
    let names_z = words.iter().any(|w| w.starts_with('Z'));
    if s.d_unknown && d_word.is_none() && names_z {
        return Err(err(
            ln,
            "a Z move with the tool length unknown (after M6 before a D, or a D with no tool loaded): machine data (PGsl 5.6)",
        ));
    }
    if names_z && d_word.is_some_and(|d| d != 0) && s.loaded.filter(|t| *t != 0).is_none() {
        return Err(err(
            ln,
            "a Z move with a D and no tool loaded: the offset is not stated (PGsl 5.6)",
        ));
    }
    if supa && names_z && d_now != 0 {
        return Err(err(
            ln,
            "SUPA with Z and a tool offset in force: SUPA suppresses frames, not the offset (PGsl 12.9); not modelled",
        ));
    }
    if s.traori {
        // 3+2 only: the rotary axes held, no machine-coordinate moves, no change of tool or
        // compensation under the transformation
        let rotary = words.iter().any(|w| {
            w.chars()
                .next()
                .is_some_and(|l| c.letters.contains(&l) && !matches!(l, 'X' | 'Y' | 'Z'))
        });
        let why = if rotary {
            Some("a rotary axis moving under TRAORI (simultaneous 5-axis) is not modelled")
        } else if supa {
            Some("SUPA under TRAORI is not modelled")
        } else if m6 {
            Some("a tool change under TRAORI is not modelled")
        } else if comp_set.is_some() {
            Some("cutter compensation under TRAORI is not modelled")
        } else if mcall_set.is_some() {
            Some("a modal cycle call under TRAORI is not modelled")
        } else {
            None
        };
        if let Some(w) = why {
            return Err(err(ln, w));
        }
    }
    if s.comp && (d_word.is_some_and(|d| d != s.d) || m6) {
        return Err(err(
            ln,
            "a D or a tool change while compensation is on is not modelled",
        ));
    }
    let names_xy = words.iter().any(|w| w.starts_with('X') || w.starts_with('Y'));
    // the compensation event, sent just before the block's move (comp::apply reads the next move
    // as the approach or the retract)
    let mut comp_ev: Option<Ev> = None;
    match comp_set {
        Some(Some(left)) => {
            if s.comp {
                return Err(err(
                    ln,
                    "G41/G42 while compensation is on (a change of side) is not modelled",
                ));
            }
            if !s.norm {
                return Err(err(
                    ln,
                    "G41/G42 with the approach not stated: program NORM (its default is machine data, MD20150; PGsl 10.2, 16.4 group 17)",
                ));
            }
            let arc = s.corner.ok_or_else(|| {
                err(
                    ln,
                    "G41/G42 with the corner behaviour not stated: program G450 or G451 (its default is machine data, MD20150; PGsl 10.3, 16.4 group 18)",
                )
            })?;
            if d_word.unwrap_or(s.d) == 0 {
                return Err(err(ln, "G41/G42 with D0: no tool radius in force"));
            }
            if st.core.plane != 17 {
                return Err(err(ln, "G41/G42 outside G17 is not modelled"));
            }
            if !names_xy {
                return Err(err(
                    ln,
                    "G41/G42 needs an X or Y move in its block (PGsl 10.1 p. 258)",
                ));
            }
            s.comp = true;
            comp_ev = Some(Ev::Comp {
                left: Some(left),
                arc,
                d: None,
            });
        }
        Some(None) if s.comp => {
            if !names_xy {
                return Err(err(
                    ln,
                    "G40 with no X or Y move in its block: the retract is not modelled",
                ));
            }
            s.comp = false;
            comp_ev = Some(Ev::Comp {
                left: None,
                arc: s.corner.unwrap_or(true),
                d: None,
            });
        }
        _ => {}
    }
    let mut lines: Vec<String> = Vec::new();
    if let Some(d) = d_word {
        if d == 0 {
            lines.push("G49".into());
            s.d_unknown = false;
        } else if let Some(t) = s.loaded.filter(|t| *t != 0) {
            lines.push(format!("G43 H{t}"));
            s.d_unknown = false;
        } else {
            // D activates the record of the active tool (PGsl 5.6); with none loaded by this
            // program the manual does not say what it holds: a Z move is refused until a D
            // after a change (or D0)
            s.d_unknown = true;
            notes.push((
                format!("D{d}"),
                "a D with no tool loaded by the program: the offset it selects is not stated (PGsl 5.6); a Z move is refused until a D after M6",
            ));
        }
        s.d = d;
    }
    let mcall_now = s.mcall.clone();
    let mcall_changed = mcall_set.is_some();
    if let Some(m) = mcall_set {
        s.mcall = m;
    }
    if !words.is_empty() {
        if supa {
            words.insert(0, "G53".into());
        }
        lines.push(words.join(" "));
    }
    // run them
    let n = n_word.map(|n| format!("{n} ")).unwrap_or_default();
    let last = lines.len();
    for (i, l) in lines.iter().enumerate() {
        if i + 1 == last
            && !words.is_empty()
            && let Some(e) = comp_ev.take()
        {
            push(out, e);
        }
        block_fanuc(&format!("{n}{l}"), ln, c, st, out)?;
    }
    if st.s840.comp && st.core.plane != 17 {
        return Err(err(
            ln,
            "a change of plane while compensation is on (PGsl 10.1 p. 265)",
        ));
    }
    for (path, check) in notes {
        push(out, Ev::NotModelled { path, check });
    }
    let s = &mut st.s840;
    if m6 {
        s.loaded = st.selected;
        s.d_unknown = true;
        s.d = 0;
    }
    // the modal drilling cycle after a block with path motion
    let motion = words
        .iter()
        .any(|w| w.chars().next().is_some_and(|l| c.letters.contains(&l)))
        || (matches!(st.core.mode, Some(0) | Some(1))
            && words
                .iter()
                .any(|w| w.starts_with('S') || w.starts_with('F') || w == "G0" || w == "G1"));
    if let (Some([rtp, rfp, sdis, dp]), true, false) = (&mcall_now, motion, mcall_changed) {
        let (rtp_q, rfp_q, sdis_q, dp_q) = (q(rtp, k, ln)?, q(rfp, k, ln)?, q(sdis, k, ln)?, q(dp, k, ln)?);
        if sdis_q < 0 {
            return Err(err(ln, "CYCLE81's SDIS is given without sign (PGZ 2.1.3)"));
        }
        if (rtp_q - dp_q).abs() <= (rfp_q - dp_q).abs() {
            return Err(err(
                ln,
                "CYCLE81: the retraction plane must lie farther from the depth than the reference plane (PGZ 2.1.3)",
            ));
        }
        let safe = rfp_q + if rtp_q >= rfp_q { sdis_q } else { -sdis_q };
        for l in [
            format!("G0 Z{}", q_text(safe, k)),
            format!("G1 Z{}", q_text(dp_q, k)),
            format!("G0 Z{}", q_text(rtp_q, k)),
        ] {
            block_fanuc(&l, ln, c, st, out)?;
        }
        // the cycle's own modal state afterwards is not assumed
        st.core.mode = None;
    }
    Ok(())
}

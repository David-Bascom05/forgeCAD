//! Tool life: Taylor's equation per tool material × work material, the life a program uses,
//! and where it reaches the user's wear limit.
//!
//! Model. Taylor (1907): v T^n = C, T the minutes to a stated wear criterion (e.g. flank wear
//! VB = 0.3 mm) at cutting speed v (m/min). The extended form adds the feed and the depth:
//! v T^n f^a ap^b = C, so T = (C / (v f^a ap^b))^(1/n). Each row in `data/life.toml` gives n, C,
//! a, b, which feed it means (`fz`, `hex`, `hm` or `f` per rev), the wear criterion, the range
//! the fit was made over, and its cite. A tool × material with no row has no life estimate:
//! the answer is "not estimated", never a guessed number.
//!
//! A program's life use is the linear damage sum D = Σ Δt_i / T(v_i, f_i, ap_i) over fixed-length
//! samples of the actual cut (the cumulative tool-life rule; the row's cite and
//! research/cam/toollife/model.md give its basis and limits): each sample's own speed, chip and
//! depth, not an op average. Air moves add nothing. D = 1 is the row's wear criterion; the
//! user's limit is a fraction of it (`Limit`).
//!
//! Samples outside the row's fitted range are still counted with the same formula, but their
//! minutes are reported (`ToolLife::outside`), so an extrapolated answer is visible as such.
use crate::LibError;
use crate::toml::{Table, num, nums, tables, text, texts};

/// Which feed a row's `a` exponent applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedVar {
    /// Programmed feed per tooth, mm.
    Fz,
    /// Peak chip thickness, mm.
    Hex,
    /// Mean chip thickness, mm.
    Hm,
    /// Feed per revolution (turning, drilling), mm/rev.
    FRev,
}

impl FeedVar {
    pub fn parse(s: &str) -> Option<FeedVar> {
        Some(match s {
            "fz" => FeedVar::Fz,
            "hex" => FeedVar::Hex,
            "hm" => FeedVar::Hm,
            "f" => FeedVar::FRev,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            FeedVar::Fz => "fz",
            FeedVar::Hex => "hex",
            FeedVar::Hm => "hm",
            FeedVar::FRev => "f",
        }
    }
}

/// One tool-life fit: tool material/coating × work material, with its cite.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: String,
    /// milling | turning | drilling.
    pub op: String,
    /// Tool substrate (`carbide`, `hss`, `ceramic`, `cbn`, ...) and coating (`` = any/none stated).
    pub tool: String,
    pub coating: String,
    /// Work materials the fit is for: forge-material names or aliases, compared case-blind.
    pub work: Vec<String>,
    /// ISO 513 group of the work, informational (rows match by `work`, never by group alone).
    pub group: String,
    pub n: f64,
    pub c: f64,
    pub a: f64,
    pub b: f64,
    pub feed: FeedVar,
    /// The wear criterion T is measured to: flank wear VB, mm (None: the source states another
    /// criterion, given in `criterion`).
    pub vb: Option<f64>,
    pub criterion: String,
    /// Fitted ranges: speed m/min, feed (in `feed`'s unit) mm, depth mm.
    pub vc_range: Option<[f64; 2]>,
    pub f_range: Option<[f64; 2]>,
    pub ap_range: Option<[f64; 2]>,
    pub source: String,
    /// Where in the source (table, equation, page).
    pub taken: String,
    /// The source's own statement, restated.
    pub value: String,
    pub licence: String,
    pub note: String,
}

fn range(t: &Table, k: &str, rec: &str) -> Result<Option<[f64; 2]>, LibError> {
    match nums(t, k) {
        None => Ok(None),
        Some(v) if v.len() == 2 && v[0] <= v[1] && v[0] >= 0.0 => Ok(Some([v[0], v[1]])),
        Some(v) => Err(LibError::Bad {
            record: rec.to_string(),
            field: "range",
            why: format!("`{k}` must be [lo, hi] with 0 <= lo <= hi (is {v:?})"),
        }),
    }
}

impl Row {
    pub fn from_table(t: &Table) -> Result<Row, LibError> {
        let id = text(t, "id")
            .ok_or_else(|| LibError::Missing {
                record: "[[life]]".into(),
                field: "id",
                fix: "give `id`".into(),
            })?
            .to_string();
        let rec = format!("life {id}");
        let need = |k: &'static str| {
            num(t, k).ok_or_else(|| LibError::Missing {
                record: rec.clone(),
                field: k,
                fix: format!("give `{k}`"),
            })
        };
        let n = need("n")?;
        let c = need("c")?;
        if !(n > 0.0 && n.is_finite() && c > 0.0 && c.is_finite()) {
            return Err(LibError::Bad {
                record: rec,
                field: "n",
                why: format!("n and c must be > 0 (n = {n}, c = {c})"),
            });
        }
        let a = num(t, "a").unwrap_or(0.0);
        let b = num(t, "b").unwrap_or(0.0);
        let fs = text(t, "feed").unwrap_or("fz");
        let feed = FeedVar::parse(fs).ok_or_else(|| LibError::Bad {
            record: rec.clone(),
            field: "feed",
            why: format!("`{fs}` is not fz | hex | hm | f"),
        })?;
        let source = text(t, "source").unwrap_or("").to_string();
        if source.is_empty() {
            return Err(LibError::Missing {
                record: rec,
                field: "source",
                fix: "cite the published source of n and C (no uncited rows)".into(),
            });
        }
        let work = texts(t, "work").unwrap_or_default();
        if work.is_empty() {
            return Err(LibError::Missing {
                record: rec,
                field: "work",
                fix: "list the work materials (forge-material names) the fit is for".into(),
            });
        }
        let s = |k: &str| text(t, k).unwrap_or("").to_string();
        Ok(Row {
            op: text(t, "op").unwrap_or("milling").to_string(),
            tool: s("tool"),
            coating: s("coating"),
            work,
            group: s("group"),
            n,
            c,
            a,
            b,
            feed,
            vb: num(t, "vb"),
            criterion: s("criterion"),
            vc_range: range(t, "vc_range", &rec)?,
            f_range: range(t, "f_range", &rec)?,
            ap_range: range(t, "ap_range", &rec)?,
            source,
            taken: s("taken"),
            value: s("value"),
            licence: s("licence"),
            note: s("note"),
            id,
        })
    }

    /// Minutes to the row's wear criterion at these conditions (None for non-positive input).
    pub fn life(&self, vc: f64, f: f64, ap: f64) -> Option<f64> {
        if !pos(vc) {
            return None;
        }
        let mut denom = vc;
        if self.a != 0.0 {
            if !pos(f) {
                return None;
            }
            denom *= forge_math::dmath::powf(f, self.a);
        }
        if self.b != 0.0 {
            if !pos(ap) {
                return None;
            }
            denom *= forge_math::dmath::powf(ap, self.b);
        }
        let t = forge_math::dmath::powf(self.c / denom, 1.0 / self.n);
        t.is_finite().then_some(t)
    }

    /// Whether these conditions are inside every range the row states.
    pub fn inside(&self, vc: f64, f: f64, ap: f64) -> bool {
        let ok = |r: Option<[f64; 2]>, x: f64| r.is_none_or(|r| x >= r[0] && x <= r[1]);
        ok(self.vc_range, vc)
            && (self.a == 0.0 || ok(self.f_range, f))
            && (self.b == 0.0 || ok(self.ap_range, ap))
    }

    pub fn matches(&self, op: &str, tool: &str, coating: &str, work: &str) -> bool {
        let eq = |x: &str, y: &str| x.trim().eq_ignore_ascii_case(y.trim());
        eq(&self.op, op)
            && eq(&self.tool, tool)
            && (self.coating.is_empty() || eq(&self.coating, coating))
            && self.work.iter().any(|w| eq(w, work))
    }
}

/// Every `[[life]]` row of a file.
pub fn read(src: &str, file: &str) -> Result<Vec<Row>, LibError> {
    let t = crate::toml::parse(src).map_err(|e| LibError::Toml {
        file: file.to_string(),
        line: e.line,
        what: e.what,
    })?;
    let mut out: Vec<Row> = Vec::new();
    for r in tables(&t, "life") {
        let x = Row::from_table(r)?;
        if out.iter().any(|o| o.id == x.id) {
            return Err(LibError::Duplicate {
                kind: "life row",
                name: x.id,
            });
        }
        out.push(x);
    }
    Ok(out)
}

/// The built-in rows (data/life.toml).
pub fn builtin() -> Vec<Row> {
    read(include_str!("../data/life.toml"), "builtin/life.toml").expect("the built-in life data reads")
}

/// The row for a tool on a work material: an exact coating match first, then a row that names
/// no coating. Every work name given (the material's name, then its aliases) is tried.
pub fn find<'a>(rows: &'a [Row], op: &str, tool: &str, coating: &str, work: &[&str]) -> Option<&'a Row> {
    let exact = |r: &&&Row| !r.coating.is_empty();
    for w in work {
        let all: Vec<&Row> = rows.iter().filter(|r| r.matches(op, tool, coating, w)).collect();
        if let Some(r) = all.iter().find(exact).or(all.first()) {
            return Some(r);
        }
    }
    None
}

/// One fixed-length sample of cutting, as the caller measured it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cut {
    /// The op (index in the program) and the NC line, when known.
    pub op: usize,
    /// The move in the op's path, and the NC line it posts to, when known.
    pub mv: Option<usize>,
    pub line: Option<usize>,
    /// Minutes the sample takes; zero-engagement samples should not be passed (air).
    pub minutes: f64,
    /// Cutting speed at the engaged diameter, m/min.
    pub vc: f64,
    pub fz: f64,
    pub hex: f64,
    pub hm: f64,
    /// Feed per revolution (turning/drilling), mm.
    pub f_rev: f64,
    pub ap: f64,
}

impl Cut {
    pub fn feed(&self, v: FeedVar) -> f64 {
        match v {
            FeedVar::Fz => self.fz,
            FeedVar::Hex => self.hex,
            FeedVar::Hm => self.hm,
            FeedVar::FRev => self.f_rev,
        }
    }
}

/// Where the wear limit is reached.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct At {
    pub op: usize,
    pub mv: Option<usize>,
    pub line: Option<usize>,
    /// Minutes in cut into this program when it is reached.
    pub minutes: f64,
}

/// Per-op share.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpLife {
    pub op: usize,
    pub minutes: f64,
    /// Fraction of the tool's life (D) this op uses; None: not estimated.
    pub used: Option<f64>,
}

/// One tool's life use over a program.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolLife {
    /// The row used (its id), or None: not estimated.
    pub row: Option<String>,
    pub minutes_in_cut: f64,
    /// D: fraction of the row's life used (1 = the row's wear criterion). None: not estimated.
    pub used: Option<f64>,
    /// Minutes of cut outside the row's fitted ranges (extrapolated).
    pub outside: f64,
    /// Where the life used reaches `limit - start` (the tool's remaining life), if it does.
    pub limit_at: Option<At>,
    pub ops: Vec<OpLife>,
    /// "not estimated: no tool-life data for carbide AlTiN in 6061" etc.
    pub note: String,
}

/// The user's wear limit and the tool's state before the program.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limit {
    /// Fraction of the row's life the user changes the tool at (1 = the row's criterion).
    pub at: f64,
    /// Life already used on the physical tool (from its instance log), same unit.
    pub start: f64,
}

impl Default for Limit {
    fn default() -> Limit {
        Limit { at: 1.0, start: 0.0 }
    }
}

/// Integrates the life a tool uses over `cuts` (in program order).
pub fn integrate(row: Option<&Row>, cuts: &[Cut], limit: Limit, not_found: &str) -> ToolLife {
    let mut ops: Vec<OpLife> = Vec::new();
    let mut minutes = 0.0;
    let mut outside = 0.0;
    let mut d = 0.0;
    let mut limit_at = None;
    let mut bad = false;
    let budget = limit.at - limit.start;
    for c in cuts {
        if !pos(c.minutes) {
            continue;
        }
        minutes += c.minutes;
        let used = row.and_then(|r| {
            let f = c.feed(r.feed);
            if !r.inside(c.vc, f, c.ap) {
                outside += c.minutes;
            }
            r.life(c.vc, f, c.ap).map(|t| c.minutes / t)
        });
        if row.is_some() && used.is_none() {
            bad = true;
        }
        let du = used.unwrap_or(0.0);
        if limit_at.is_none() && row.is_some() && !bad && d + du >= budget {
            // the fraction of this sample before the limit, linearly
            let frac = if du > 0.0 {
                ((budget - d) / du).clamp(0.0, 1.0)
            } else {
                0.0
            };
            limit_at = Some(At {
                op: c.op,
                mv: c.mv,
                line: c.line,
                minutes: minutes - c.minutes * (1.0 - frac),
            });
        }
        d += du;
        match ops.last_mut() {
            Some(o) if o.op == c.op => {
                o.minutes += c.minutes;
                o.used = o.used.zip(used).map(|(x, y)| x + y);
            }
            _ => ops.push(OpLife {
                op: c.op,
                minutes: c.minutes,
                used,
            }),
        }
    }
    let est = row.is_some() && !bad;
    if !est {
        for o in &mut ops {
            o.used = None;
        }
    }
    ToolLife {
        row: row.map(|r| r.id.clone()),
        minutes_in_cut: minutes,
        used: est.then_some(d),
        outside,
        limit_at: if est { limit_at } else { None },
        ops,
        note: match row {
            None => format!("not estimated: {not_found}"),
            Some(_) if bad => "not estimated: a sample had a zero speed, feed or depth the row needs".into(),
            Some(r) => format!(
                "{} (n = {}, C = {}{}{}), to {}",
                r.source,
                r.n,
                r.c,
                if r.a != 0.0 {
                    format!(", a = {} on {}", r.a, r.feed.name())
                } else {
                    String::new()
                },
                if r.b != 0.0 {
                    format!(", b = {} on ap", r.b)
                } else {
                    String::new()
                },
                r.vb.map(|v| format!("VB = {v} mm"))
                    .unwrap_or_else(|| r.criterion.clone()),
            ),
        },
    }
}

/// A non-blocking tool-life warning.
#[derive(Clone, Debug, PartialEq)]
pub struct Warning {
    pub tool: String,
    /// Life the program would use and what was left, as fractions of the row's life.
    pub used: f64,
    pub left: f64,
    /// Change the tool here (where the limit is reached), or use the sister tool.
    pub change_at: Option<At>,
    pub sister: Option<String>,
    pub why: String,
}

/// Warns when a program's predicted use exceeds the tool's remaining life. Never an error: the
/// program still posts. `sisters` are other loaded tools of the same library record.
pub fn check(tool: &str, life: &ToolLife, limit: Limit, sisters: &[&str]) -> Option<Warning> {
    let used = life.used?;
    let left = limit.at - limit.start;
    if used <= left {
        return None;
    }
    let sister = sisters.first().map(|s| s.to_string());
    let at = life.limit_at;
    let mut why = format!(
        "tool {tool}: this program uses {:.0}% of its life; {:.0}% is left",
        100.0 * used,
        100.0 * left.max(0.0)
    );
    if let Some(a) = at {
        why += &format!(
            "; change it in op {}{} ({:.1} min into the cut)",
            a.op + 1,
            match (a.line, a.mv) {
                (Some(l), _) => format!(" at line {l}"),
                (None, Some(m)) => format!(" at move {}", m + 1),
                _ => String::new(),
            },
            a.minutes
        );
    }
    if let Some(s) = &sister {
        why += &format!(", or continue with sister tool {s}");
    }
    if life.outside > 0.0 {
        why += &format!(
            " ({:.1} of {:.1} min are outside the data's fitted range)",
            life.outside, life.minutes_in_cut
        );
    }
    Some(Warning {
        tool: tool.to_string(),
        used,
        left,
        change_at: at,
        sister,
        why,
    })
}

/// x > 0 (false for NaN).
fn pos(x: f64) -> bool {
    x > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(n: f64, c: f64, a: f64, b: f64) -> Row {
        Row {
            id: "t".into(),
            op: "milling".into(),
            tool: "carbide".into(),
            coating: "".into(),
            work: vec!["X".into()],
            group: "P".into(),
            n,
            c,
            a,
            b,
            feed: FeedVar::Fz,
            vb: Some(0.3),
            criterion: String::new(),
            vc_range: Some([100.0, 300.0]),
            f_range: None,
            ap_range: None,
            source: "test".into(),
            taken: String::new(),
            value: String::new(),
            licence: String::new(),
            note: String::new(),
        }
    }

    fn cut(op: usize, minutes: f64, vc: f64) -> Cut {
        Cut {
            op,
            mv: None,
            line: Some(10 * op),
            minutes,
            vc,
            fz: 0.1,
            hex: 0.1,
            hm: 0.06,
            f_rev: 0.0,
            ap: 1.0,
        }
    }

    #[test]
    fn taylor_basic() {
        let r = row(0.25, 400.0, 0.0, 0.0);
        // v T^n = C: at v = 200, T = (400/200)^4 = 16 min
        assert!((r.life(200.0, 0.0, 0.0).unwrap() - 16.0).abs() < 1e-12);
        // doubling v divides T by 2^(1/n) = 16
        assert!((r.life(400.0, 0.0, 0.0).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn taylor_extended() {
        let r = row(0.25, 400.0, 0.5, 0.2);
        let t = r.life(200.0, 0.04, 2.0).unwrap();
        // v T^n f^a ap^b = C
        let back = 200.0
            * forge_math::dmath::powf(t, 0.25)
            * forge_math::dmath::powf(0.04, 0.5)
            * forge_math::dmath::powf(2.0, 0.2);
        assert!((back - 400.0).abs() < 1e-9);
        assert_eq!(r.life(200.0, 0.0, 2.0), None);
    }

    #[test]
    fn damage_sum_and_limit_point() {
        let r = row(0.25, 400.0, 0.0, 0.0); // T(200) = 16 min, T(400) = 1 min
        let cuts = [cut(0, 8.0, 200.0), cut(1, 0.25, 400.0), cut(1, 0.5, 400.0)];
        let l = integrate(Some(&r), &cuts, Limit::default(), "");
        // 8/16 + 0.25 + 0.5 = 1.25
        assert!((l.used.unwrap() - 1.25).abs() < 1e-12);
        assert_eq!(l.ops.len(), 2);
        assert!((l.ops[1].used.unwrap() - 0.75).abs() < 1e-12);
        // the limit (D = 1) is met 0.25 min into the third sample: 8 + 0.25 + 0.25
        let at = l.limit_at.unwrap();
        assert_eq!(at.op, 1);
        assert!((at.minutes - 8.5).abs() < 1e-12);
        assert_eq!(l.outside, 0.75); // vc 400 is outside [100, 300]
        let w = check("T3", &l, Limit::default(), &["T13"]).unwrap();
        assert_eq!(w.sister.as_deref(), Some("T13"));
        // a fresh tool with an 80% limit already 50% used
        let lim = Limit { at: 0.8, start: 0.5 };
        let l2 = integrate(Some(&r), &cuts, lim, "");
        assert!((l2.limit_at.unwrap().minutes - 4.8).abs() < 1e-12);
    }

    #[test]
    fn no_row_is_not_estimated() {
        let l = integrate(
            None,
            &[cut(0, 3.0, 200.0)],
            Limit::default(),
            "no data for hss in X",
        );
        assert_eq!(l.used, None);
        assert_eq!(l.minutes_in_cut, 3.0);
        assert!(l.note.starts_with("not estimated"));
        assert_eq!(check("T1", &l, Limit::default(), &[]), None);
    }

    #[test]
    fn builtin_reads_and_every_row_is_cited() {
        for r in builtin() {
            assert!(!r.source.is_empty() && !r.taken.is_empty(), "{} uncited", r.id);
        }
    }
}

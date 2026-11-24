//! Physical tool instances: a tool on the shelf (label or serial), the library record it is,
//! the minutes it has cut, the programs it ran, its regrinds and the user's wear observations.
//!
//! Kept in `tool-instances.toml` at a library level (project `cam/` or the user's `cam/`
//! folder), next to the tool library, never in the .fg. Written by `forgecad tool log` and the
//! UI; plain TOML in the CAM libraries' subset (crate::toml) so it diffs and hand-edits.
//!
//! Each run stores its cut as a few condition bins (vc, chip, depth, minutes), so the shop's
//! own Taylor fit (`calibrate`) can be redone later against any row.
use crate::LibError;
use crate::life::{Cut, FeedVar, Row};
use crate::toml::{Table, num, nums, tables, text};

pub const FILE: &str = "tool-instances.toml";

/// A condition bin of a run: minutes cut at (vc, fz, hex, hm, f_rev, ap).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bin {
    pub vc: f64,
    pub fz: f64,
    pub hex: f64,
    pub hm: f64,
    pub f_rev: f64,
    pub ap: f64,
    pub minutes: f64,
}

impl Bin {
    fn cut(&self) -> Cut {
        Cut {
            op: 0,
            mv: None,
            line: None,
            minutes: self.minutes,
            vc: self.vc,
            fz: self.fz,
            hex: self.hex,
            hm: self.hm,
            f_rev: self.f_rev,
            ap: self.ap,
        }
    }
}

/// Bins `cuts` to 2% steps in each value (log scale), keeping the time.
pub fn bins(cuts: &[Cut]) -> Vec<Bin> {
    let q = |x: f64| if x > 0.0 { (x.ln() / 0.02f64.ln_1p()).round() as i64 } else { i64::MIN };
    let mut out: Vec<(Vec<i64>, Bin)> = Vec::new();
    for c in cuts.iter().filter(|c| c.minutes > 0.0) {
        let k = vec![q(c.vc), q(c.fz), q(c.hex), q(c.hm), q(c.f_rev), q(c.ap)];
        if let Some((_, b)) = out.iter_mut().find(|(kk, _)| *kk == k) {
            // time-weighted means inside the bin
            let w = b.minutes + c.minutes;
            let m = |x: f64, y: f64| (x * b.minutes + y * c.minutes) / w;
            *b = Bin {
                vc: m(b.vc, c.vc),
                fz: m(b.fz, c.fz),
                hex: m(b.hex, c.hex),
                hm: m(b.hm, c.hm),
                f_rev: m(b.f_rev, c.f_rev),
                ap: m(b.ap, c.ap),
                minutes: w,
            };
        } else {
            out.push((
                k,
                Bin {
                    vc: c.vc,
                    fz: c.fz,
                    hex: c.hex,
                    hm: c.hm,
                    f_rev: c.f_rev,
                    ap: c.ap,
                    minutes: c.minutes,
                },
            ));
        }
    }
    out.into_iter().map(|(_, b)| b).collect()
}

/// A program the tool ran (predicted from the posted program, or marked run).
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub program: String,
    pub date: String,
    /// The work material it cut (forge-material name).
    pub work: String,
    pub minutes: f64,
    /// "posted" | "run".
    pub how: String,
    pub bins: Vec<Bin>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A wear observation; the tool stays in use.
    Observe,
    /// Taken out worn (to the user's criterion).
    Worn,
    Chipped,
    Broken,
    /// Reground: the life starts again (the record keeps the count).
    Regrind,
}

impl Event {
    pub fn parse(s: &str) -> Option<Event> {
        Some(match s {
            "observe" => Event::Observe,
            "worn" => Event::Worn,
            "chipped" => Event::Chipped,
            "broken" => Event::Broken,
            "regrind" => Event::Regrind,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Event::Observe => "observe",
            Event::Worn => "worn",
            Event::Chipped => "chipped",
            Event::Broken => "broken",
            Event::Regrind => "regrind",
        }
    }
    /// Ends this life of the tool (a new one starts after a regrind).
    pub fn ends_life(self) -> bool {
        !matches!(self, Event::Observe)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Log {
    pub date: String,
    pub event: Event,
    /// Flank wear VB, mm, when measured.
    pub vb: Option<f64>,
    pub note: String,
    /// Index of the run the event came after (runs.len() at logging time).
    pub after: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    pub label: String,
    /// The tool library record's id.
    pub tool: String,
    pub serial: String,
    pub note: String,
    /// "active" | "retired".
    pub state: String,
    pub runs: Vec<Run>,
    pub log: Vec<Log>,
}

impl Instance {
    pub fn new(label: &str, tool: &str) -> Instance {
        Instance {
            label: label.into(),
            tool: tool.into(),
            serial: String::new(),
            note: String::new(),
            state: "active".into(),
            runs: Vec::new(),
            log: Vec::new(),
        }
    }

    pub fn minutes(&self) -> f64 {
        self.runs.iter().map(|r| r.minutes).sum()
    }

    pub fn regrinds(&self) -> usize {
        self.log.iter().filter(|l| l.event == Event::Regrind).count()
    }

    /// The runs of the current life (since the last life-ending event).
    pub fn current_runs(&self) -> &[Run] {
        let from = self
            .log
            .iter()
            .filter(|l| l.event.ends_life())
            .map(|l| l.after)
            .max()
            .unwrap_or(0);
        &self.runs[from.min(self.runs.len())..]
    }

    /// Lives that ended, as (runs, event, vb): one per life-ending event.
    pub fn lives(&self) -> Vec<(&[Run], &Log)> {
        let mut out = Vec::new();
        let mut from = 0;
        for l in self.log.iter().filter(|l| l.event.ends_life()) {
            let to = l.after.min(self.runs.len());
            out.push((&self.runs[from.min(to)..to], l));
            from = to;
        }
        out
    }

    /// Life used in the current life by `row` (None when the runs cut a material the row has
    /// no data for, or a bin the row cannot evaluate).
    pub fn used(&self, row: &Row) -> Option<f64> {
        damage(self.current_runs(), row)
    }
}

/// D of `runs` under `row`; None if any run's material is not one of the row's or a bin can't
/// be evaluated.
pub fn damage(runs: &[Run], row: &Row) -> Option<f64> {
    let mut d = 0.0;
    for r in runs {
        if !row.work.iter().any(|w| w.eq_ignore_ascii_case(&r.work)) {
            return None;
        }
        // minutes logged by hand with no conditions: not evaluable
        if r.minutes > 0.0 && r.bins.is_empty() {
            return None;
        }
        for b in &r.bins {
            let c = b.cut();
            d += c.minutes / row.life(c.vc, c.feed(row.feed), c.ap)?;
        }
    }
    Some(d)
}

fn s(t: &Table, k: &str) -> String {
    text(t, k).unwrap_or("").to_string()
}

pub fn read(src: &str, file: &str) -> Result<Vec<Instance>, LibError> {
    let t = crate::toml::parse(src).map_err(|e| LibError::Toml {
        file: file.to_string(),
        line: e.line,
        what: e.what,
    })?;
    let mut out: Vec<Instance> = Vec::new();
    for it in tables(&t, "instance") {
        let label = s(it, "label");
        let rec = format!("tool instance {label}");
        if label.is_empty() {
            return Err(LibError::Missing {
                record: "[[instance]]".into(),
                field: "label",
                fix: "give `label`".into(),
            });
        }
        let mut runs = Vec::new();
        for r in tables(it, "run") {
            let mut bins = Vec::new();
            for b in tables(r, "bin") {
                let v = nums(b, "v").unwrap_or_default();
                if v.len() != 7 {
                    return Err(LibError::Bad {
                        record: rec.clone(),
                        field: "bin",
                        why: "`v` is [vc, fz, hex, hm, f_rev, ap, minutes]".into(),
                    });
                }
                bins.push(Bin {
                    vc: v[0],
                    fz: v[1],
                    hex: v[2],
                    hm: v[3],
                    f_rev: v[4],
                    ap: v[5],
                    minutes: v[6],
                });
            }
            runs.push(Run {
                program: s(r, "program"),
                date: s(r, "date"),
                work: s(r, "work"),
                minutes: num(r, "minutes").unwrap_or(0.0),
                how: s(r, "how"),
                bins,
            });
        }
        let mut log = Vec::new();
        for l in tables(it, "log") {
            let e = s(l, "event");
            let event = Event::parse(&e).ok_or_else(|| LibError::Bad {
                record: rec.clone(),
                field: "event",
                why: format!("`{e}` is not observe | worn | chipped | broken | regrind"),
            })?;
            log.push(Log {
                date: s(l, "date"),
                event,
                vb: num(l, "vb"),
                note: s(l, "note"),
                after: num(l, "after").unwrap_or(0.0) as usize,
            });
        }
        if out.iter().any(|o| o.label == label) {
            return Err(LibError::Duplicate {
                kind: "tool instance",
                name: label,
            });
        }
        out.push(Instance {
            tool: s(it, "tool"),
            serial: s(it, "serial"),
            note: s(it, "note"),
            state: text(it, "state").unwrap_or("active").to_string(),
            label,
            runs,
            log,
        });
    }
    Ok(out)
}

fn q(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn f(x: f64) -> String {
    // shortest round-trip form, always with a digit after the point for floats
    let s = format!("{x:?}");
    if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") { s } else { format!("{s}.0") }
}

pub fn write(all: &[Instance]) -> String {
    let mut o = String::from(
        "# Physical tool instances (forgecad tool log). One [[instance]] per tool on the shelf.\n",
    );
    for i in all {
        o += &format!("\n[[instance]]\nlabel = {}\ntool = {}\n", q(&i.label), q(&i.tool));
        if !i.serial.is_empty() {
            o += &format!("serial = {}\n", q(&i.serial));
        }
        if !i.note.is_empty() {
            o += &format!("note = {}\n", q(&i.note));
        }
        o += &format!("state = {}\n", q(&i.state));
        for r in &i.runs {
            o += &format!(
                "\n[[instance.run]]\nprogram = {}\ndate = {}\nwork = {}\nminutes = {}\nhow = {}\n",
                q(&r.program),
                q(&r.date),
                q(&r.work),
                f(r.minutes),
                q(&r.how)
            );
            for b in &r.bins {
                o += &format!(
                    "[[instance.run.bin]]\nv = [{}, {}, {}, {}, {}, {}, {}]\n",
                    f(b.vc),
                    f(b.fz),
                    f(b.hex),
                    f(b.hm),
                    f(b.f_rev),
                    f(b.ap),
                    f(b.minutes)
                );
            }
        }
        for l in &i.log {
            o += &format!(
                "\n[[instance.log]]\ndate = {}\nevent = {}\nafter = {}\n",
                q(&l.date),
                q(l.event.name()),
                l.after
            );
            if let Some(v) = l.vb {
                o += &format!("vb = {}\n", f(v));
            }
            if !l.note.is_empty() {
                o += &format!("note = {}\n", q(&l.note));
            }
        }
    }
    o
}

/// The shop's own Taylor fit for one row's tool class and material, from logged lives that
/// ended worn. Shown beside the cited row, never in its place.
#[derive(Clone, Debug, PartialEq)]
pub struct Calibration {
    pub row: String,
    /// Lives used (ended `worn`).
    pub n_logs: usize,
    /// The fitted n (the row's when there is too little spread in speed to fit it) and C.
    pub n: f64,
    pub n_fitted: bool,
    pub c: f64,
    /// Spread of the per-life C values: min and max, and the geometric standard deviation.
    pub c_min: f64,
    pub c_max: f64,
    pub gsd: f64,
    pub note: String,
}

/// C per ended life under exponent n (the row's a, b): the C for which that life's damage sum
/// is exactly 1. For D(n, C) = Σ t_k (v_k f_k^a ap_k^b / C)^(1/n): C = (Σ t_k (v_k f^a ap^b)^(1/n))^n.
fn c_of_life(runs: &[Run], row: &Row, n: f64) -> Option<f64> {
    let mut s = 0.0;
    for r in runs {
        for b in &r.bins {
            let c = b.cut();
            let fv = c.feed(row.feed);
            let mut x = c.vc;
            if row.a != 0.0 {
                x *= fv.powf(row.a);
            }
            if row.b != 0.0 {
                x *= c.ap.powf(row.b);
            }
            if !(x > 0.0) {
                return None;
            }
            s += c.minutes * x.powf(1.0 / n);
        }
    }
    (s > 0.0).then(|| s.powf(n))
}

fn stats(cs: &[f64]) -> (f64, f64, f64, f64) {
    let l: Vec<f64> = cs.iter().map(|c| c.ln()).collect();
    let m = l.iter().sum::<f64>() / l.len() as f64;
    let var = if l.len() > 1 {
        l.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (l.len() - 1) as f64
    } else {
        0.0
    };
    let lo = cs.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = cs.iter().cloned().fold(0.0, f64::max);
    (m.exp(), lo, hi, var.sqrt().exp())
}

/// Fits the shop's C (and n with at least 3 lives whose time-weighted speeds span ≥ 20%) for
/// `row` from every instance of `tools` (library ids of this row's class) whose ended-worn
/// lives cut only the row's materials. None with no such life.
pub fn calibrate(all: &[Instance], tools: &[&str], row: &Row) -> Option<Calibration> {
    let mut lives: Vec<Vec<Run>> = Vec::new();
    for i in all.iter().filter(|i| tools.contains(&i.tool.as_str())) {
        for (runs, l) in i.lives() {
            if l.event != Event::Worn || runs.is_empty() || damage(runs, row).is_none() {
                continue;
            }
            lives.push(runs.to_vec());
        }
    }
    if lives.is_empty() {
        return None;
    }
    let mean_v = |runs: &[Run]| {
        let (mut tv, mut t) = (0.0, 0.0);
        for b in runs.iter().flat_map(|r| &r.bins) {
            tv += b.vc * b.minutes;
            t += b.minutes;
        }
        tv / t.max(1e-300)
    };
    let vs: Vec<f64> = lives.iter().map(|r| mean_v(r)).collect();
    let span = vs.iter().cloned().fold(0.0, f64::max) / vs.iter().cloned().fold(f64::INFINITY, f64::min);
    let fit_n = lives.len() >= 3 && span >= 1.2;
    let cs_at = |n: f64| -> Option<Vec<f64>> { lives.iter().map(|r| c_of_life(r, row, n)).collect() };
    let mut n = row.n;
    if fit_n {
        // n minimising the spread of log C across lives (golden-section on [0.05, 1.0])
        let cost = |n: f64| cs_at(n).map(|c| stats(&c).3.ln()).unwrap_or(f64::INFINITY);
        let (mut a, mut b) = (0.05f64, 1.0f64);
        let g = (5f64.sqrt() - 1.0) / 2.0;
        for _ in 0..80 {
            let (x1, x2) = (b - g * (b - a), a + g * (b - a));
            if cost(x1) < cost(x2) {
                b = x2;
            } else {
                a = x1;
            }
        }
        n = 0.5 * (a + b);
    }
    let cs = cs_at(n)?;
    let (c, lo, hi, gsd) = stats(&cs);
    Some(Calibration {
        row: row.id.clone(),
        n_logs: cs.len(),
        n,
        n_fitted: fit_n,
        c,
        c_min: lo,
        c_max: hi,
        gsd,
        note: format!(
            "calibrated from your {} log{}: C = {:.0} (range {:.0}–{:.0}){}; cited: n = {}, C = {} ({})",
            cs.len(),
            if cs.len() == 1 { "" } else { "s" },
            c,
            lo,
            hi,
            if fit_n { format!(", n = {n:.3} fitted") } else { format!(", n = {} from the cite", row.n) },
            row.n,
            row.c,
            row.source
        ),
    })
}

/// The feed a row reads, for display.
pub fn feed_name(v: FeedVar) -> &'static str {
    v.name()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> Row {
        crate::life::read(
            "[[life]]\nid = \"r\"\ntool = \"carbide\"\nwork = [\"X\"]\nn = 0.25\nc = 400\nsource = \"test\"\ntaken = \"t\"\n",
            "t",
        )
        .unwrap()
        .remove(0)
    }

    fn run(vc: f64, minutes: f64) -> Run {
        Run {
            program: "p".into(),
            date: "2026-11-24".into(),
            work: "X".into(),
            minutes,
            how: "run".into(),
            bins: vec![Bin {
                vc,
                fz: 0.1,
                hex: 0.1,
                hm: 0.06,
                f_rev: 0.0,
                ap: 1.0,
                minutes,
            }],
        }
    }

    #[test]
    fn round_trip() {
        let mut i = Instance::new("T3-a", "harvey-12345");
        i.serial = "SN \"7\"".into();
        i.runs.push(run(200.0, 8.0));
        i.log.push(Log {
            date: "2026-11-24".into(),
            event: Event::Observe,
            vb: Some(0.12),
            note: "light".into(),
            after: 1,
        });
        let all = vec![i];
        let back = read(&write(&all), "t").unwrap();
        assert_eq!(back, all);
        assert!((back[0].used(&row()).unwrap() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn bins_merge() {
        let c = |vc: f64, m: f64| Cut {
            op: 0,
            mv: None,
            line: None,
            minutes: m,
            vc,
            fz: 0.1,
            hex: 0.1,
            hm: 0.06,
            f_rev: 0.0,
            ap: 1.0,
        };
        let b = bins(&[c(200.0, 1.0), c(200.5, 1.0), c(300.0, 2.0)]);
        assert_eq!(b.len(), 2);
        assert!((b[0].minutes - 2.0).abs() < 1e-12 && (b[0].vc - 200.25).abs() < 1e-12);
    }

    #[test]
    fn calibration_recovers_c_and_n() {
        // shop truth: n = 0.3, C = 300. Lives at 3 speeds, each worn exactly at D = 1.
        let (n, c) = (0.3f64, 300.0f64);
        let mut all = Vec::new();
        for (k, v) in [150.0, 200.0, 250.0].iter().enumerate() {
            let t = (c / v).powf(1.0 / n);
            let mut i = Instance::new(&format!("a{k}"), "tool-x");
            i.runs.push(run(*v, t));
            i.log.push(Log {
                date: String::new(),
                event: Event::Worn,
                vb: Some(0.3),
                note: String::new(),
                after: 1,
            });
            all.push(i);
        }
        let cal = calibrate(&all, &["tool-x"], &row()).unwrap();
        assert!(cal.n_fitted);
        assert!((cal.n - n).abs() < 1e-6, "n {}", cal.n);
        assert!((cal.c / c - 1.0).abs() < 1e-5, "c {}", cal.c);
        // with one life, n stays the cited 0.25 and C makes that life's D = 1
        let one = calibrate(&all[..1], &["tool-x"], &row()).unwrap();
        assert!(!one.n_fitted && one.n == 0.25);
        let t = (c / 150.0f64).powf(1.0 / n);
        assert!((one.c - 150.0 * t.powf(0.25)).abs() < 1e-9);
        assert!(one.note.contains("calibrated from your 1 log"));
    }
}

//! `forgecad machine check`: what can be said about a machine definition before a program runs
//! on it, each finding with how it stands (proven, found, sampled, not checked).
//!
//! - **structure**: the reader's own validation (MachineDef::validate, validate_cites).
//! - **home**: every pair of bodies at the home pose (collide::check_pose).
//! - **travel**: every pair of bodies through the joints' travel. The distance between two
//!   bodies depends only on the joints in exactly one of their two chains (a joint both chains
//!   share moves both rigidly). With none, the home verdict holds for all travel; with one, a
//!   sweep of that joint through its range is a proof (collide's interval check of a line in
//!   which one joint moves covers exactly that range); with two or more, the joints are swept
//!   one at a time with the others at each end of their ranges (a Hit there is a real
//!   collision), then the whole box of their travel is proven clear by bisection
//!   (`prove_box`); if that runs out of budget the pair is "sampled": its clear lines say
//!   nothing of the poses between them.
//! - **provenance**: per value, how it stands: published/estimate/unknown/conflict cites, the
//!   user's own (uncited in a user or project file), or uncited (a built-in non-real machine's
//!   placeholders). Values cited unknown are where a limit check is not a check of the machine.
//! - **envelopes**: each body's part inside its envelope; the caller fills it (it needs
//!   forge-lang: forge_machine_lang::check_parts).
use crate::catalog::Level;
use crate::write::status_name;
use forge_machine::collide::{self, Verdict};
use forge_machine::def::{JointType, MachineDef, MountKind};
use forge_machine::gcode::Control;
use forge_machine::sim::{Path as SegPath, SegKind, Segment, Setup, Timeline};

#[derive(Clone, Debug, PartialEq)]
pub enum Travel {
    /// Clear for every value of the joints that move the pair relative to each other.
    Proven,
    /// A pose where one body is inside the other: joint values (def.joints order).
    Hit { q: Vec<f64>, witness: [f64; 3] },
    /// Not proven clear and no witness: within the clearance somewhere on a swept line.
    Near { q: Vec<f64>, gap_lo: f64 },
    /// Lines swept clear; the poses between them are not proven.
    Sampled { lines: usize },
    /// Not checked: same link, or `ignore`d by construction.
    Skipped(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairCheck {
    pub a: String,
    pub b: String,
    /// Joints that move the pair relative to each other.
    pub joints: Vec<String>,
    pub travel: Travel,
    /// Bodies of the pair whose envelope is cited unknown: the verdict is about the stand-in.
    pub unknown_envelope: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EnvelopeCheck {
    Inside {
        vertices: usize,
    },
    Outside {
        vertices: usize,
        outside: usize,
        worst: f64,
    },
    NotChecked(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValueStatus {
    pub path: String,
    /// published | estimate | unknown | conflict | user | uncited; several if cited more than once.
    pub status: Vec<String>,
    pub source: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CheckReport {
    pub machine: String,
    pub level: Option<Level>,
    pub file: String,
    /// The reader's refusal, if the file does not load (nothing else is checked then).
    pub structure: Option<String>,
    pub home: Vec<(String, String, String)>,
    pub pairs: Vec<PairCheck>,
    pub values: Vec<ValueStatus>,
    pub envelopes: Vec<(String, EnvelopeCheck)>,
    /// Facts a reader of the report needs (accel absent, no power curve, ...).
    pub notes: Vec<String>,
}

impl CheckReport {
    /// No refusal, nothing at home, no Hit or Near, no part outside its envelope. Sampled and
    /// unknown values do not fail the check; they are listed.
    pub fn ok(&self) -> bool {
        self.structure.is_none()
            && self.home.is_empty()
            && !self
                .pairs
                .iter()
                .any(|p| matches!(p.travel, Travel::Hit { .. } | Travel::Near { .. }))
            && !self
                .envelopes
                .iter()
                .any(|(_, e)| matches!(e, EnvelopeCheck::Outside { .. }))
    }
    pub fn unknown_paths(&self) -> Vec<&str> {
        self.values
            .iter()
            .filter(|v| v.status.iter().any(|s| s == "unknown"))
            .map(|v| v.path.as_str())
            .collect()
    }
}

fn verdict_text(v: &Verdict) -> String {
    match v {
        Verdict::Clear => "clear".into(),
        Verdict::Hit { witness, .. } => {
            format!("hit at ({:.3}, {:.3}, {:.3})", witness[0], witness[1], witness[2])
        }
        Verdict::Near { gap_lo, .. } => format!("near (gap ≥ {gap_lo:.3} mm not proven ≥ clearance)"),
    }
}

/// The joints on the chain of `link` (indexes), base first; empty for the base.
fn chain(def: &MachineDef, link: &str) -> Vec<usize> {
    def.chain(link).unwrap_or_default()
}

fn setup(def: &MachineDef) -> Setup<'_> {
    let first = |k: MountKind| {
        def.mounts
            .iter()
            .find(|m| m.kind == k)
            .map(|m| m.name.clone())
            .unwrap_or_default()
    };
    Setup {
        def,
        control: Control::mill3(3, 0, false),
        joint_of: Vec::new(),
        tool_mount: first(MountKind::Tool),
        part_mount: first(MountKind::Part),
        wcs: Vec::new(),
        tools: Vec::new(),
        h: Vec::new(),
        geo: Vec::new(),
        dogleg: false,
        feed_combined: true,
        start: def.joints.iter().map(|j| j.home).collect(),
        start_tool: None,
    }
}

fn line(q0: Vec<f64>, q1: Vec<f64>) -> Segment {
    Segment {
        line: 0,
        kind: SegKind::Rapid,
        q0,
        q1,
        path: SegPath::Line,
        t0: 0.0,
        t_min: 0.0,
        t_stop: 0.0,
        tool: None,
        spindle: None,
    }
}

/// At most this many joints are swept corner by corner (2^(k−1) lines per joint).
const MAX_CORNER_JOINTS: usize = 6;

/// One pair through travel.
fn pair_travel(def: &MachineDef, ai: usize, bi: usize) -> (Vec<usize>, Travel) {
    let (a, b) = (&def.bodies[ai], &def.bodies[bi]);
    let (ca, cb) = (chain(def, &a.link), chain(def, &b.link));
    let mut js: Vec<usize> = ca
        .iter()
        .filter(|j| !cb.contains(j))
        .chain(cb.iter().filter(|j| !ca.contains(j)))
        .copied()
        .collect();
    js.sort();
    if a.link == b.link {
        return (js, Travel::Skipped("same link".into()));
    }
    if a.ignore.contains(&b.name) || b.ignore.contains(&a.name) {
        return (js, Travel::Skipped("ignored by construction".into()));
    }
    // a joint whose travel is UNKNOWN: the pair is not checked through it
    if let Some(&j) = js.iter().find(|&&j| def.joints[j].range.is_none()) {
        return (
            js,
            Travel::Skipped(format!(
                "joint {} travel UNKNOWN: not checked",
                def.joints[j].name
            )),
        );
    }
    let rg = |j: usize| {
        def.joints[j]
            .range
            .expect("checked above: every moving joint has its travel")
    };
    let mut d2 = def.clone();
    d2.bodies = vec![a.clone(), b.clone()];
    let home: Vec<f64> = def.joints.iter().map(|j| j.home).collect();
    let mut segs = Vec::new();
    if js.is_empty() {
        segs.push(line(home.clone(), home.clone()));
    } else if js.len() > MAX_CORNER_JOINTS {
        // one line per joint through its range, the others at home
        for &j in &js {
            let (mut q0, mut q1) = (home.clone(), home.clone());
            q0[j] = rg(j)[0];
            q1[j] = rg(j)[1];
            segs.push(line(q0, q1));
        }
    } else {
        for (k, &j) in js.iter().enumerate() {
            let others: Vec<usize> = js
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != k)
                .map(|(_, x)| *x)
                .collect();
            let corners = if js.len() == 1 { 1 } else { 1usize << others.len() };
            for c in 0..corners {
                let mut base = home.clone();
                if js.len() > 1 {
                    for (bit, &o) in others.iter().enumerate() {
                        base[o] = rg(o)[(c >> bit) & 1];
                    }
                }
                let (mut q0, mut q1) = (base.clone(), base);
                q0[j] = rg(j)[0];
                q1[j] = rg(j)[1];
                segs.push(line(q0, q1));
            }
        }
    }
    let n = segs.len();
    let s = setup(&d2);
    let mut near = None;
    // one line at a time: the first Hit ends the pair
    for seg in segs {
        let tl = Timeline {
            segments: vec![seg],
            marks: Vec::new(),
            violations: Vec::new(),
            t_min: 0.0,
            t_stop: 0.0,
            joints: Vec::new(),
            accel_missing: false,
            t_la: 0.0,
            seg_la: Vec::new(),
            t_scurve: 0.0,
            seg_scurve: Vec::new(),
        };
        let rep = collide::check(&s, &tl, &[]);
        for c in &rep.contacts {
            let seg = &tl.segments[c.segment];
            match &c.verdict {
                Verdict::Hit { s, witness } => {
                    return (
                        js,
                        Travel::Hit {
                            q: seg.at(*s),
                            witness: *witness,
                        },
                    );
                }
                Verdict::Near { s0, s1, gap_lo } => {
                    if near.is_none() {
                        near = Some(Travel::Near {
                            q: seg.at(0.5 * (s0 + s1)),
                            gap_lo: *gap_lo,
                        });
                    }
                }
                Verdict::Clear => {}
            }
        }
    }
    if let Some(n) = near {
        return (js, n);
    }
    // one moving joint (or none) and every line clear: proven
    if js.len() <= 1 {
        return (js, Travel::Proven);
    }
    // several: try to prove the whole box of their travel
    let lo: Vec<f64> = js.iter().map(|&j| rg(j)[0]).collect();
    let hi: Vec<f64> = js.iter().map(|&j| rg(j)[1]).collect();
    let mut budget = BOX_BUDGET;
    if prove_box(&s, &js, &home, &lo, &hi, &lo, &hi, &mut budget) {
        (js, Travel::Proven)
    } else {
        (js, Travel::Sampled { lines: n })
    }
}

/// Boxes tried per pair before the proof gives up (the pair is then "sampled").
const BOX_BUDGET: usize = 1024;

/// Clear for every joint value in the box [lo, hi] (per joint of `js`; the rest at `home`)?
/// A segment from lo to hi checked in one span encloses the joints' values over the whole box
/// (collide::check's enclosure of a line over s ∈ [0, 1] is each joint's [lo, hi], and the
/// kinematics and gap bounds are interval-sound); `Report::spans == 1` with no contact says the
/// first span was already clear, before any split along the diagonal. Otherwise the box is
/// split in half across the joint widest relative to its range, and both halves are proven.
#[allow(clippy::too_many_arguments)]
fn prove_box(
    s: &Setup,
    js: &[usize],
    home: &[f64],
    lo: &[f64],
    hi: &[f64],
    lo0: &[f64],
    hi0: &[f64],
    budget: &mut usize,
) -> bool {
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    let (mut q0, mut q1) = (home.to_vec(), home.to_vec());
    for (k, &j) in js.iter().enumerate() {
        q0[j] = lo[k];
        q1[j] = hi[k];
    }
    let tl = Timeline {
        segments: vec![line(q0, q1)],
        marks: Vec::new(),
        violations: Vec::new(),
        t_min: 0.0,
        t_stop: 0.0,
        joints: Vec::new(),
        accel_missing: false,
        t_la: 0.0,
        seg_la: Vec::new(),
        t_scurve: 0.0,
        seg_scurve: Vec::new(),
    };
    let rep = collide::check(s, &tl, &[]);
    if rep.pairs_checked == 1 && rep.spans == 1 && rep.contacts.is_empty() {
        return true;
    }
    let k = (0..js.len())
        .max_by(|&a, &b| {
            let ra = (hi[a] - lo[a]) / (hi0[a] - lo0[a]).max(f64::MIN_POSITIVE);
            let rb = (hi[b] - lo[b]) / (hi0[b] - lo0[b]).max(f64::MIN_POSITIVE);
            ra.total_cmp(&rb)
        })
        .unwrap();
    if hi[k] - lo[k] <= collide::RES {
        return false;
    }
    let mid = 0.5 * (lo[k] + hi[k]);
    let (mut h1, mut l2) = (hi.to_vec(), lo.to_vec());
    h1[k] = mid;
    l2[k] = mid;
    prove_box(s, js, home, lo, &h1, lo0, hi0, budget) && prove_box(s, js, home, &l2, hi, lo0, hi0, budget)
}

/// The check of a loaded machine (level and file say where it came from: they decide whether an
/// uncited value is the user's).
pub fn check(def: &MachineDef, level: Option<Level>, file: &str) -> CheckReport {
    let mut r = CheckReport {
        machine: def.name.clone(),
        level,
        file: file.to_string(),
        ..CheckReport::default()
    };
    let home: Vec<f64> = def.joints.iter().map(|j| j.home).collect();
    for (a, b, v) in collide::check_pose(def, &home) {
        r.home.push((a, b, verdict_text(&v)));
    }
    let unknown_body = |n: &str| {
        def.cites
            .iter()
            .any(|c| c.path == format!("body.{n}") && c.status == forge_machine::def::CiteStatus::Unknown)
    };
    for i in 0..def.bodies.len() {
        for j in i + 1..def.bodies.len() {
            let (js, travel) = pair_travel(def, i, j);
            let (a, b) = (&def.bodies[i].name, &def.bodies[j].name);
            r.pairs.push(PairCheck {
                a: a.clone(),
                b: b.clone(),
                joints: js.iter().map(|k| def.joints[*k].name.clone()).collect(),
                travel,
                unknown_envelope: [a, b].into_iter().filter(|n| unknown_body(n)).cloned().collect(),
            });
        }
    }
    // provenance
    let own = matches!(level, Some(Level::User | Level::Project));
    let mut paths = def.cite_paths();
    for c in &def.cites {
        if !paths.contains(&c.path) {
            paths.push(c.path.clone());
        }
    }
    for p in paths {
        let cs: Vec<_> = def.cites.iter().filter(|c| c.path == p).collect();
        let (status, source) = if cs.is_empty() {
            (vec![if own { "user" } else { "uncited" }.to_string()], Vec::new())
        } else {
            (
                cs.iter().map(|c| status_name(c.status).to_string()).collect(),
                cs.iter()
                    .map(|c| {
                        if c.source.is_empty() {
                            c.basis.clone()
                        } else {
                            c.source.clone()
                        }
                    })
                    .collect(),
            )
        };
        r.values.push(ValueStatus {
            path: p,
            status,
            source,
        });
    }
    // notes
    let no_accel: Vec<&str> = def
        .joints
        .iter()
        .filter(|j| j.accel.is_none())
        .map(|j| j.name.as_str())
        .collect();
    if !no_accel.is_empty() {
        r.notes.push(format!(
            "no `accel` on {}: exact-stop times assume infinite acceleration (t_stop = t_min there)",
            no_accel.join(", ")
        ));
    }
    for s in &def.spindles {
        if s.power.is_empty() {
            r.notes
                .push(format!("spindle `{}` has no power curve: no power check", s.name));
        }
    }
    for b in &def.bodies {
        if b.part.is_none() {
            r.envelopes.push((
                b.name.clone(),
                EnvelopeCheck::NotChecked("no part: the envelope is the body".into()),
            ));
        }
    }
    if def.joints.iter().any(|j| j.ty == JointType::Rotary && j.cyclic) {
        r.notes
            .push("cyclic rotaries are swept through their whole output range".into());
    }
    if !r.unknown_paths().is_empty() {
        r.notes.push(format!(
            "{} values are cited unknown: the simulation still checks against the number in the file until the format can hold UNKNOWN (request F1); treat those checks as not checked",
            r.unknown_paths().len()
        ));
    }
    r
}

/// The check of a file's text: every machine in it, or the reader's refusal.
pub fn check_text(src: &str, level: Option<Level>, file: &str) -> Vec<CheckReport> {
    match MachineDef::parse_all(src) {
        Ok(ms) => ms.iter().map(|m| check(m, level, file)).collect(),
        Err(e) => vec![CheckReport {
            file: file.to_string(),
            level,
            structure: Some(e.to_string()),
            ..CheckReport::default()
        }],
    }
}

fn jstr(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn jnum(x: f64) -> String {
    if x.is_finite() {
        format!("{x:?}")
    } else {
        "null".into()
    }
}

fn jlist(v: &[String]) -> String {
    format!("[{}]", v.iter().map(|s| jstr(s)).collect::<Vec<_>>().join(","))
}

impl CheckReport {
    /// `forgecad.machine-check/1`.
    pub fn json(&self) -> String {
        let pairs: Vec<String> = self
            .pairs
            .iter()
            .map(|p| {
                let (v, extra) = match &p.travel {
                    Travel::Proven => ("proven", String::new()),
                    Travel::Hit { q, witness } => (
                        "hit",
                        format!(
                            ",\"q\":[{}],\"witness\":[{}]",
                            q.iter().map(|x| jnum(*x)).collect::<Vec<_>>().join(","),
                            witness.iter().map(|x| jnum(*x)).collect::<Vec<_>>().join(",")
                        ),
                    ),
                    Travel::Near { q, gap_lo } => (
                        "near",
                        format!(
                            ",\"q\":[{}],\"gap_lo\":{}",
                            q.iter().map(|x| jnum(*x)).collect::<Vec<_>>().join(","),
                            jnum(*gap_lo)
                        ),
                    ),
                    Travel::Sampled { lines } => ("sampled", format!(",\"lines\":{lines}")),
                    Travel::Skipped(why) => ("skipped", format!(",\"why\":{}", jstr(why))),
                };
                format!(
                    "{{\"a\":{},\"b\":{},\"joints\":{},\"travel\":\"{v}\"{extra},\"unknown_envelope\":{}}}",
                    jstr(&p.a),
                    jstr(&p.b),
                    jlist(&p.joints),
                    jlist(&p.unknown_envelope)
                )
            })
            .collect();
        let values: Vec<String> = self
            .values
            .iter()
            .map(|v| {
                format!(
                    "{{\"path\":{},\"status\":{},\"source\":{}}}",
                    jstr(&v.path),
                    jlist(&v.status),
                    jlist(&v.source)
                )
            })
            .collect();
        let env: Vec<String> = self
            .envelopes
            .iter()
            .map(|(b, e)| match e {
                EnvelopeCheck::Inside { vertices } => {
                    format!("{{\"body\":{},\"status\":\"inside\",\"vertices\":{vertices}}}", jstr(b))
                }
                EnvelopeCheck::Outside {
                    vertices,
                    outside,
                    worst,
                } => format!(
                    "{{\"body\":{},\"status\":\"outside\",\"vertices\":{vertices},\"outside\":{outside},\"worst\":{}}}",
                    jstr(b),
                    jnum(*worst)
                ),
                EnvelopeCheck::NotChecked(why) => {
                    format!("{{\"body\":{},\"status\":\"not-checked\",\"why\":{}}}", jstr(b), jstr(why))
                }
            })
            .collect();
        let home: Vec<String> = self
            .home
            .iter()
            .map(|(a, b, v)| format!("{{\"a\":{},\"b\":{},\"verdict\":{}}}", jstr(a), jstr(b), jstr(v)))
            .collect();
        format!(
            "{{\"format\":\"forgecad.machine-check/1\",\"machine\":{},\"level\":{},\"file\":{},\"ok\":{},\"structure\":{},\"home\":[{}],\"pairs\":[{}],\"values\":[{}],\"envelopes\":[{}],\"notes\":{}}}",
            jstr(&self.machine),
            self.level.map_or("null".to_string(), |l| jstr(l.name())),
            jstr(&self.file),
            self.ok(),
            self.structure.as_deref().map_or("null".to_string(), jstr),
            home.join(","),
            pairs.join(","),
            values.join(","),
            env.join(","),
            jlist(&self.notes)
        )
    }

    /// The readable report.
    pub fn text(&self, def_joints: &[String]) -> String {
        let mut o = String::new();
        let lv = self.level.map_or("file".to_string(), |l| l.name().to_string());
        o.push_str(&format!(
            "machine `{}` ({lv}, {}): {}\n",
            self.machine,
            self.file,
            if self.ok() { "ok" } else { "PROBLEMS" }
        ));
        if let Some(e) = &self.structure {
            o.push_str(&format!("  structure: REFUSED: {e}\n"));
            return o;
        }
        o.push_str("  structure: ok\n");
        if self.home.is_empty() {
            o.push_str("  home pose: every pair clear\n");
        } else {
            for (a, b, v) in &self.home {
                o.push_str(&format!("  home pose: {a} / {b}: {v}\n"));
            }
        }
        let qtext = |q: &[f64]| {
            q.iter()
                .zip(def_joints)
                .map(|(x, n)| format!("{n}={x:.3}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        o.push_str("  travel (self-collision):\n");
        for p in &self.pairs {
            let via = if p.joints.is_empty() {
                "fixed to each other".to_string()
            } else {
                format!("via {}", p.joints.join(" "))
            };
            let v = match &p.travel {
                Travel::Proven => "clear through all travel (proven)".to_string(),
                Travel::Hit { q, witness } => format!(
                    "HIT at {} (point {:.3}, {:.3}, {:.3})",
                    qtext(q),
                    witness[0],
                    witness[1],
                    witness[2]
                ),
                Travel::Near { q, gap_lo } => {
                    format!(
                        "NEAR at {} (gap not proven ≥ clearance; bound {gap_lo:.3} mm)",
                        qtext(q)
                    )
                }
                Travel::Sampled { lines } => {
                    format!("{lines} swept lines clear; poses between them NOT proven")
                }
                Travel::Skipped(why) => format!("not checked ({why})"),
            };
            let unk = if p.unknown_envelope.is_empty() {
                String::new()
            } else {
                format!(" [envelope UNKNOWN: {}]", p.unknown_envelope.join(", "))
            };
            o.push_str(&format!("    {} / {} ({via}): {v}{unk}\n", p.a, p.b));
        }
        if !self.envelopes.is_empty() {
            o.push_str("  parts in envelopes:\n");
            for (b, e) in &self.envelopes {
                let t = match e {
                    EnvelopeCheck::Inside { vertices } => format!("inside ({vertices} vertices)"),
                    EnvelopeCheck::Outside {
                        vertices,
                        outside,
                        worst,
                    } => format!("OUTSIDE: {outside} of {vertices} vertices, worst {worst:.3} mm"),
                    EnvelopeCheck::NotChecked(w) => format!("not checked ({w})"),
                };
                o.push_str(&format!("    {b}: {t}\n"));
            }
        }
        let mut counts: Vec<(String, usize)> = Vec::new();
        for v in &self.values {
            for s in &v.status {
                match counts.iter_mut().find(|(k, _)| k == s) {
                    Some(c) => c.1 += 1,
                    None => counts.push((s.clone(), 1)),
                }
            }
        }
        o.push_str(&format!(
            "  provenance: {}\n",
            counts
                .iter()
                .map(|(k, n)| format!("{n} {k}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        let unk = self.unknown_paths();
        if !unk.is_empty() {
            o.push_str(&format!("    UNKNOWN (not checked): {}\n", unk.join(", ")));
        }
        for n in &self.notes {
            o.push_str(&format!("  note: {n}\n"));
        }
        o
    }
}

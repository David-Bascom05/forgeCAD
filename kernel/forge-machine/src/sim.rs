//! Posted G-code on a machine: the interpreter's events become joint motion, timed.
//!
//! The words are mapped to joint values the way a non-TCP control does it: for the letter of
//! joint j, q_j = word (halved for a diameter letter) + the active work offset of that letter + the
//! active tool offset on that joint. Tool offsets are the *registers* the setup gives (what the
//! control holds), not the tool's true geometry: a wrong H shows as a tool that is not where the
//! program means it, which the collision check and the stock see.
//!
//! Paths in joint space: a feed or a coordinated rapid is a straight line in joint space; an arc's
//! two plane joints follow the circle (others linearly); a dog-leg rapid (`Setup::dogleg`) moves
//! every joint at its own rapid rate, so it is piecewise linear with a break where each joint
//! arrives. G28 goes through its intermediate point to each named joint's `home`.
//!
//! Times: `t_min` holds under two facts about any control (the feed is never exceeded, nor any
//! joint's rapid or max feed); `t_stop` is the time with exact stops and trapezoidal acceleration
//! (joint `accel`; the path parameter limited by the slowest joint), an estimate, not a bound: a
//! blending control is faster, a jerk-limited one slower. Tool changes take the changer's
//! `change_s`; a turret index takes `index_s` per station (the shorter way round).
use crate::def::{JointType, MachineDef, MountKind};
use crate::gcode::{self, Block, Centre, Control, Ev, FeedMode, GError};
use forge_math::dmath;

/// A cutting tool as the simulation sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct SimTool {
    /// T number (mill pocket or turret station).
    pub t: u32,
    pub name: String,
    /// The tool's tip, from the tool mount's gauge point, in the mount's frame at home pose
    /// (a mill's: dir × gauge length).
    pub tip: [f64; 3],
    /// The tool's body from the tip toward the gauge, as stacked cylinders about its axis:
    /// (radius, from, to) distances along the axis from the tip. The first is the cutting part.
    pub body: Vec<(f64, f64, f64)>,
    /// The cutting edge's shape, for material removal.
    pub cut: Cut,
}

/// A cutting edge as the stock sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cut {
    /// A revolving cutter about the tool axis: radius r and corner radius (0 flat, r ball).
    Mill { r: f64, corner: f64 },
    /// A turning insert's nose: radius rn, its centre at the programmed tip plus `centre`
    /// (radial, axial) in the part's (r, z) plane.
    Nose { rn: f64, centre: [f64; 2] },
}

/// The setup: the machine, its control, the mounts used, offsets and tools.
#[derive(Clone, Debug)]
pub struct Setup<'a> {
    pub def: &'a MachineDef,
    pub control: Control,
    /// The joint each control letter drives (indexes def.joints), in `control.letters` order.
    pub joint_of: Vec<usize>,
    pub tool_mount: String,
    pub part_mount: String,
    /// Work offsets by number (54..59): per control letter, the joint value at word 0.
    pub wcs: Vec<(u32, Vec<f64>)>,
    pub tools: Vec<SimTool>,
    /// Length (H) registers for mills: (h, value) added on `length_joint`.
    pub h: Vec<(u32, f64)>,
    /// Lathe offset registers: (n, per control letter value).
    pub geo: Vec<(u32, Vec<f64>)>,
    /// Rapids move every joint at its own rate (true) or coordinated in a line (false).
    pub dogleg: bool,
    /// G94 with linear and rotary joints moving: F applies to the combined length of all joint
    /// motion (mm and degrees together, Fanuc-like) if true, to the linear part only (LinuxCNC)
    /// if false. Which one a control does is its setting: it is not assumed.
    pub feed_combined: bool,
    /// Joint values when the program starts.
    pub start: Vec<f64>,
    /// The tool in the spindle (or the turret station) at the start.
    pub start_tool: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SimError {
    G(GError),
    Setup(String),
    At { line: usize, what: String },
}

impl std::fmt::Display for SimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimError::G(e) => write!(f, "line {}: {}", e.line, e.what),
            SimError::Setup(s) => write!(f, "setup: {s}"),
            SimError::At { line, what } => write!(f, "line {line}: {what}"),
        }
    }
}

impl<'a> Setup<'a> {
    /// A setup on a machine with a control, using the first tool and part mounts, the machine's
    /// letters of channel 1, joints starting at home, no tools and no offsets.
    pub fn new(def: &'a MachineDef, control: Control) -> Result<Setup<'a>, SimError> {
        let mut joint_of = Vec::new();
        for l in &control.letters {
            let j = def
                .joints
                .iter()
                .position(|j| j.letter == l.to_string() || j.letter == format!("{l}1"))
                .or_else(|| def.joints.iter().position(|j| j.letter.starts_with(*l)))
                .ok_or_else(|| SimError::Setup(format!("the control's letter {l} drives no joint")))?;
            joint_of.push(j);
        }
        let tool_mount = def
            .mounts
            .iter()
            .find(|m| m.kind == MountKind::Tool)
            .unwrap()
            .name
            .clone();
        let part_mount = def
            .mounts
            .iter()
            .find(|m| m.kind == MountKind::Part)
            .unwrap()
            .name
            .clone();
        Ok(Setup {
            def,
            joint_of,
            tool_mount,
            part_mount,
            wcs: Vec::new(),
            tools: Vec::new(),
            h: Vec::new(),
            geo: Vec::new(),
            dogleg: false,
            feed_combined: true,
            start: def.joints.iter().map(|j| j.home).collect(),
            start_tool: None,
            control,
        })
    }

    pub fn tool(&self, t: u32) -> Option<&SimTool> {
        self.tools.iter().find(|x| x.t == t)
    }

    /// A tool's tip and axis (tip toward the gauge) in the part's frame at joint values q.
    pub fn tip_in_part(&self, q: &[f64], t: u32) -> Option<([f64; 3], [f64; 3])> {
        use crate::kin;
        let tool = self.tool(t)?;
        let p = kin::pose(self.def, q);
        let m = self.def.mount(&self.tool_mount)?;
        let l = kin::link(self.def, &p, &m.link);
        let tip = l.apply(kin::add(m.at, tool.tip));
        let f = kin::part_frame(self.def, &p, &self.part_mount)?.inverse();
        Some((f.apply(tip), f.dir(l.dir(kin::scale(m.dir, -1.0)))))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegKind {
    Rapid,
    Feed,
    Arc,
}

/// How joints move over a segment, s in [0, 1].
#[derive(Clone, Debug, PartialEq)]
pub enum Path {
    /// q(s) = q0 + s (q1 − q0).
    Line,
    /// Joints a and b about (ca, cb), from angle a0 swept by `sweep` radians (+ counter-clockwise
    /// in (a, b)); the radius goes linearly from r (the start's) to r1 (the end's: they differ by
    /// the end's rounding, which a control interpolates); every other joint linear in s.
    Arc {
        a: usize,
        b: usize,
        c: [f64; 2],
        r: f64,
        r1: f64,
        a0: f64,
        sweep: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub line: usize,
    pub kind: SegKind,
    pub q0: Vec<f64>,
    pub q1: Vec<f64>,
    pub path: Path,
    /// Seconds from the program start.
    pub t0: f64,
    /// Lower bound on the duration (s).
    pub t_min: f64,
    /// Exact-stop, trapezoidal estimate (s).
    pub t_stop: f64,
    /// The tool in the spindle during the segment.
    pub tool: Option<u32>,
    /// Spindle on (rpm, cw), or None.
    pub spindle: Option<(i64, bool)>,
}

impl Segment {
    /// Joint values at s ∈ [0, 1].
    pub fn at(&self, s: f64) -> Vec<f64> {
        let mut q: Vec<f64> = self
            .q0
            .iter()
            .zip(&self.q1)
            .map(|(a, b)| a + s * (b - a))
            .collect();
        if let Path::Arc {
            a,
            b,
            c,
            r,
            r1,
            a0,
            sweep,
        } = &self.path
        {
            let (sn, cs) = dmath::sin_cos(a0 + s * sweep);
            let rs = r + s * (r1 - r);
            q[*a] = c[0] + rs * cs;
            q[*b] = c[1] + rs * sn;
        }
        q
    }
}

/// A non-motion happening on the timeline.
#[derive(Clone, Debug, PartialEq)]
pub enum Happening {
    ToolChange { t: u32, seconds: f64 },
    Index { station: u32, seconds: f64 },
    Dwell { seconds: f64 },
    Spindle { rpm: i64, cw: bool, css: bool },
    SpindleStop,
    Coolant(gcode::Coolant),
    Stop { optional: bool },
    Comment(String),
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub line: usize,
    pub t: f64,
    pub what: Happening,
}

/// A limit the program breaks; the simulation goes on (the control would alarm or clamp).
#[derive(Clone, Debug, PartialEq)]
pub enum Violation {
    /// A joint outside its range at some point of a segment.
    Overtravel {
        line: usize,
        joint: String,
        value: f64,
        range: [f64; 2],
    },
    /// The programmed feed asks a joint for more than its max feed: the time is stretched.
    FeedClamped {
        line: usize,
        joint: String,
        asked: f64,
        max: f64,
    },
    /// A tool called that the setup does not have.
    NoTool { line: usize, t: u32 },
    /// A spindle speed outside the spindle's range.
    Rpm { line: usize, rpm: i64, range: [f64; 2] },
    /// The changer has fewer pockets (turret fewer stations) than the T asked.
    Pocket { line: usize, t: u32, max: u32 },
    /// The joints enter a zone the control forbids (a stored stroke check): the control alarms.
    Zone { line: usize, zone: String, at: Vec<f64> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Timeline {
    pub segments: Vec<Segment>,
    pub marks: Vec<Mark>,
    pub violations: Vec<Violation>,
    /// Total, lower bound and exact-stop estimate (s).
    pub t_min: f64,
    pub t_stop: f64,
    /// Joint names, the order of every q vector.
    pub joints: Vec<String>,
    /// True if some joint that moved has no `accel` (its t_stop is t_min).
    pub accel_missing: bool,
}

fn q_to_f(v: i64, k: u32) -> f64 {
    v as f64 / 10f64.powi(k as i32)
}

/// Trapezoidal time (s) for a unit path at speed limit v (1/s) and acceleration a (1/s²).
fn trap(v: f64, a: f64) -> f64 {
    if !a.is_finite() || a <= 0.0 {
        return 1.0 / v;
    }
    if v * v / a <= 1.0 {
        1.0 / v + v / a
    } else {
        2.0 * (1.0 / a).sqrt()
    }
}

/// Runs a program on a setup.
pub fn simulate(text: &str, s: &Setup) -> Result<Timeline, SimError> {
    let (blocks, _) = gcode::run(text, &s.control).map_err(SimError::G)?;
    simulate_blocks(&blocks, s)
}

pub fn simulate_blocks(blocks: &[Block], s: &Setup) -> Result<Timeline, SimError> {
    let def = s.def;
    let n = def.joints.len();
    let k = s.control.decimals;
    let fk = s.control.feed_decimals;
    if s.start.len() != n {
        return Err(SimError::Setup(format!(
            "start has {} values for {n} joints",
            s.start.len()
        )));
    }
    let mut q = s.start.clone();
    let mut tl = Timeline {
        segments: Vec::new(),
        marks: Vec::new(),
        violations: Vec::new(),
        t_min: 0.0,
        t_stop: 0.0,
        joints: def.joints.iter().map(|j| j.name.clone()).collect(),
        accel_missing: false,
    };
    let mut wcs: Vec<f64> = s
        .wcs
        .iter()
        .find(|(n, _)| *n == 54)
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| vec![0.0; s.joint_of.len()]);
    if wcs.len() != s.joint_of.len() {
        return Err(SimError::Setup(
            "a work offset needs one value per control letter".into(),
        ));
    }
    // per control letter: the tool offset now applied
    let mut toff: Vec<f64> = vec![0.0; s.joint_of.len()];
    let mut tool = s.start_tool;
    let mut spindle: Option<(i64, bool)> = None;
    let mut rpm_clamp: Option<i64> = None;
    // G96 in force: (surface speed m/min, rpm cap)
    let mut css_now: Option<(f64, f64)> = None;
    let length_letter = def
        .length_joint
        .as_ref()
        .and_then(|lj| s.joint_of.iter().position(|j| def.joints[*j].name == *lj));
    let mut t = 0.0f64;
    let mut t_est = 0.0f64;
    // the program position (control letters) to joint values, keeping unspecified joints
    let to_q = |words: &[Option<i64>], q: &[f64], wcs: &[f64], toff: &[f64]| -> Vec<f64> {
        let mut out = q.to_vec();
        for (i, w) in words.iter().enumerate() {
            if let Some(w) = w {
                let mut v = q_to_f(*w, k);
                if s.control.diameter == Some(s.control.letters[i]) {
                    v *= 0.5;
                }
                out[s.joint_of[i]] = v + wcs[i] + toff[i];
            }
        }
        out
    };
    let push_seg = |tl: &mut Timeline, t: &mut f64, t_est: &mut f64, seg: Segment| {
        *t += seg.t_min;
        *t_est += seg.t_stop;
        tl.segments.push(seg);
    };
    for b in blocks {
        let ln = b.line;
        let mark = |tl: &mut Timeline, t: f64, what: Happening| tl.marks.push(Mark { line: ln, t, what });
        match &b.ev {
            Ev::Rapid(words) => {
                let q1 = to_q(words, &q, &wcs, &toff);
                for seg in rapid(s, ln, &q, &q1, t, tool, spindle, &mut tl) {
                    push_seg(&mut tl, &mut t, &mut t_est, seg);
                }
                q = q1;
            }
            Ev::Machine(mv) => {
                let mut q1 = q.clone();
                for (i, v) in mv {
                    let mut x = q_to_f(*v, k);
                    if s.control.diameter == Some(s.control.letters[*i]) {
                        x *= 0.5;
                    }
                    q1[s.joint_of[*i]] = x;
                }
                for seg in rapid(s, ln, &q, &q1, t, tool, spindle, &mut tl) {
                    push_seg(&mut tl, &mut t, &mut t_est, seg);
                }
                q = q1;
            }
            Ev::Home { via, axes } => {
                let qv = to_q(via, &q, &wcs, &toff);
                for seg in rapid(s, ln, &q, &qv, t, tool, spindle, &mut tl) {
                    push_seg(&mut tl, &mut t, &mut t_est, seg);
                }
                let mut qh = qv.clone();
                for i in axes {
                    let j = s.joint_of[*i];
                    qh[j] = def.joints[j].home;
                }
                for seg in rapid(s, ln, &qv, &qh, t, tool, spindle, &mut tl) {
                    push_seg(&mut tl, &mut t, &mut t_est, seg);
                }
                q = qh;
            }
            Ev::Feed { to, f, mode } => {
                let q1 = to_q(to, &q, &wcs, &toff);
                let seg = feed_seg(
                    s,
                    ln,
                    SegKind::Feed,
                    &q,
                    &q1,
                    Path::Line,
                    q_to_f(*f, fk),
                    *mode,
                    spindle,
                    css_now,
                    t,
                    tool,
                    &mut tl,
                )?;
                push_seg(&mut tl, &mut t, &mut t_est, seg);
                q = q1;
            }
            Ev::Arc {
                to,
                plane,
                centre,
                cw,
                full,
                f,
                mode,
            } => {
                let q1 = to_q(to, &q, &wcs, &toff);
                let (la, lb) = match plane {
                    17 => ('X', 'Y'),
                    18 => ('Z', 'X'),
                    _ => ('Y', 'Z'),
                };
                let li = |l: char| s.control.letters.iter().position(|x| *x == l).unwrap();
                let (ia, ib) = (li(la), li(lb));
                let (ja, jb) = (s.joint_of[ia], s.joint_of[ib]);
                // a joint value is its word plus constant offsets (a diameter word halved), so
                // the circle in joint values is the program's circle; I J K are in radius units
                let (sa, sb) = (q[ja], q[jb]);
                let (ea, eb) = (q1[ja], q1[jb]);
                let (ca, cb, r) = match centre {
                    // I J K are incremental from the start and in radius units (a lathe's I too)
                    Centre::Offset([i, j]) => {
                        let (ca, cb) = (sa + q_to_f(*i, k), sb + q_to_f(*j, k));
                        let r = ((sa - ca).powi(2) + (sb - cb).powi(2)).sqrt();
                        (ca, cb, r)
                    }
                    Centre::R(rw) => {
                        let r = q_to_f(*rw, k);
                        let (dx, dy) = (ea - sa, eb - sb);
                        let d = (dx * dx + dy * dy).sqrt();
                        if d > 2.0 * r.abs() * (1.0 + 1e-12) {
                            return Err(SimError::At {
                                line: ln,
                                what: format!("R{} is less than half the chord {d}", r.abs()),
                            });
                        }
                        let h = (r * r - d * d / 4.0).max(0.0).sqrt();
                        // the centre left of the chord for a ccw arc under half a turn
                        let side = if *cw { -1.0 } else { 1.0 } * if r < 0.0 { -1.0 } else { 1.0 };
                        let (mx, my) = (sa + dx / 2.0, sb + dy / 2.0);
                        (mx - side * h * dy / d, my + side * h * dx / d, r.abs())
                    }
                };
                let a0 = dmath::atan2(sb - cb, sa - ca);
                let a1 = dmath::atan2(eb - cb, ea - ca);
                let mut sweep = a1 - a0;
                if *cw {
                    while sweep > 0.0 {
                        sweep -= dmath::TAU;
                    }
                    if *full {
                        sweep = -dmath::TAU;
                    }
                } else {
                    while sweep < 0.0 {
                        sweep += dmath::TAU;
                    }
                    if *full {
                        sweep = dmath::TAU;
                    }
                }
                // the end must be on the circle (a control alarms past its tolerance); report a gap
                let r_end = ((ea - ca).powi(2) + (eb - cb).powi(2)).sqrt();
                if (r_end - r).abs() > 0.01 {
                    return Err(SimError::At {
                        line: ln,
                        what: format!("the arc's end is {:.4} mm off its circle", (r_end - r).abs()),
                    });
                }
                let path = Path::Arc {
                    a: ja,
                    b: jb,
                    c: [ca, cb],
                    r,
                    r1: r_end,
                    a0,
                    sweep,
                };
                let seg = feed_seg(
                    s,
                    ln,
                    SegKind::Arc,
                    &q,
                    &q1,
                    path,
                    q_to_f(*f, fk),
                    *mode,
                    spindle,
                    css_now,
                    t,
                    tool,
                    &mut tl,
                )?;
                check_arc_range(s, &seg, &mut tl);
                push_seg(&mut tl, &mut t, &mut t_est, seg);
                q = q1;
            }
            Ev::Dwell { ms } => {
                let sec = *ms as f64 / 1000.0;
                mark(&mut tl, t, Happening::Dwell { seconds: sec });
                t += sec;
                t_est += sec;
            }
            Ev::Select(_) => {}
            Ev::Wcs(nw) => {
                wcs = s
                    .wcs
                    .iter()
                    .find(|(n, _)| n == nw)
                    .map(|(_, v)| v.clone())
                    .ok_or_else(|| SimError::At {
                        line: ln,
                        what: format!("G{nw}: the setup has no such work offset"),
                    })?;
            }
            Ev::Comment(c) => mark(&mut tl, t, Happening::Comment(c.clone())),
            Ev::Tool { t: tn, offset } => {
                if s.tool(*tn).is_none() {
                    tl.violations.push(Violation::NoTool { line: ln, t: *tn });
                }
                if s.control.lathe {
                    let turret = def.turrets.first();
                    if let Some(tu) = turret {
                        // stations or index time unknown: not checked, 0 s (t_min stays a bound)
                        if let Some(stations) = tu.stations
                            && (*tn > stations || *tn == 0)
                        {
                            tl.violations.push(Violation::Pocket {
                                line: ln,
                                t: *tn,
                                max: stations,
                            });
                        }
                        let swap = tu.manual && tool != Some(*tn);
                        if swap {
                            // a manual tool post: the operator swaps the holder
                            mark(&mut tl, t, Happening::Stop { optional: false });
                        }
                        let sec = match (tu.stations, tu.index_s) {
                            _ if tu.manual => {
                                if swap {
                                    tu.index_s.unwrap_or(0.0)
                                } else {
                                    0.0
                                }
                            }
                            (Some(stations), Some(index_s)) => {
                                let from = tool.unwrap_or(1) as i64;
                                let d = (*tn as i64 - from).rem_euclid(stations as i64);
                                d.min(stations as i64 - d) as f64 * index_s
                            }
                            _ => 0.0,
                        };
                        mark(
                            &mut tl,
                            t,
                            Happening::Index {
                                station: *tn,
                                seconds: sec,
                            },
                        );
                        t += sec;
                        t_est += sec;
                    }
                    if let Some(o) = offset {
                        toff = if *o == 0 {
                            vec![0.0; s.joint_of.len()]
                        } else {
                            s.geo
                                .iter()
                                .find(|(n, _)| n == o)
                                .map(|(_, v)| v.clone())
                                .ok_or_else(|| SimError::At {
                                    line: ln,
                                    what: format!("offset {o}: the setup has no such register"),
                                })?
                        };
                    }
                } else {
                    let (sec, max) = match &def.changer {
                        // unknowns: no pocket check, 0 s (t_min stays a lower bound)
                        Some(c) => (c.change_s.unwrap_or(0.0), c.pockets),
                        None => {
                            return Err(SimError::At {
                                line: ln,
                                what: "M6 on a machine with no tool changer".into(),
                            });
                        }
                    };
                    // the changer's own move (M06 sends Z to its change position): a rapid with the
                    // old tool, collision-checked like any other
                    if let Some(c) = &def.changer
                        && !c.position.is_empty()
                    {
                        let mut q1 = q.clone();
                        for (jn, v) in &c.position {
                            if let Some(j) = def.joint_index(jn) {
                                q1[j] = *v;
                            }
                        }
                        for seg in rapid(s, ln, &q, &q1, t, tool, spindle, &mut tl) {
                            push_seg(&mut tl, &mut t, &mut t_est, seg);
                        }
                        q = q1;
                    }
                    if let Some(max) = max
                        && *tn > max
                    {
                        tl.violations.push(Violation::Pocket {
                            line: ln,
                            t: *tn,
                            max,
                        });
                    }
                    // a manual change: the program stops for the operator
                    if def.changer.as_ref().is_some_and(|c| c.manual) {
                        mark(&mut tl, t, Happening::Stop { optional: false });
                    }
                    mark(&mut tl, t, Happening::ToolChange { t: *tn, seconds: sec });
                    t += sec;
                    t_est += sec;
                }
                tool = Some(*tn);
            }
            Ev::Length(h) => {
                let li = length_letter.ok_or_else(|| SimError::At {
                    line: ln,
                    what: "G43 on a machine with no length_joint".into(),
                })?;
                toff[li] = match h {
                    None | Some(0) => 0.0,
                    Some(h) => {
                        s.h.iter()
                            .find(|(n, _)| n == h)
                            .map(|(_, v)| *v)
                            .ok_or_else(|| SimError::At {
                                line: ln,
                                what: format!("H{h}: the setup has no such register"),
                            })?
                    }
                };
            }
            Ev::Spindle { s: sp, cw, css } => {
                css_now = if *css {
                    let cap = rpm_clamp
                        .map(|c| c as f64)
                        .or(def.spindles.first().and_then(|sd| sd.rpm.map(|r| r[1])))
                        .unwrap_or(f64::INFINITY);
                    Some((*sp as f64, cap))
                } else {
                    None
                };
                let rpm = if *css {
                    // the clamp bounds it; the true rpm follows the radius, so check the clamp
                    rpm_clamp.unwrap_or(i64::MAX)
                } else {
                    *sp
                };
                if let Some(range) = def.spindles.first().and_then(|sd| sd.rpm) {
                    let check = if *css { rpm_clamp } else { Some(rpm) };
                    if let Some(r) = check
                        && ((r as f64) < range[0] || (r as f64) > range[1])
                    {
                        tl.violations.push(Violation::Rpm {
                            line: ln,
                            rpm: r,
                            range,
                        });
                    }
                }
                spindle = Some((if *css { *sp } else { rpm }, *cw));
                mark(
                    &mut tl,
                    t,
                    Happening::Spindle {
                        rpm: *sp,
                        cw: *cw,
                        css: *css,
                    },
                );
            }
            Ev::SpindleStop => {
                spindle = None;
                css_now = None;
                mark(&mut tl, t, Happening::SpindleStop);
            }
            Ev::Clamp(c) => rpm_clamp = Some(*c),
            Ev::Coolant(c) => mark(&mut tl, t, Happening::Coolant(*c)),
            Ev::Stop { optional } => mark(&mut tl, t, Happening::Stop { optional: *optional }),
            Ev::End => mark(&mut tl, t, Happening::End),
        }
    }
    tl.t_min = t;
    tl.t_stop = t_est;
    for sg in &tl.segments {
        for z in &def.zones {
            if let Some(sv) = zone_entry(def, z, sg) {
                tl.violations.push(Violation::Zone {
                    line: sg.line,
                    zone: z.name.clone(),
                    at: sg.at(sv),
                });
            }
        }
    }
    Ok(tl)
}

/// Where a segment is strictly inside a zone (some s ∈ [0, 1]), if anywhere: exact for the
/// linear joints (the slab test); for an arc's two joints, bisection on s with bounds of the arc
/// over each span, to 2^-30 of the segment (an entry shallower than that is not reported).
pub fn zone_entry(def: &MachineDef, z: &crate::def::Zone, sg: &Segment) -> Option<f64> {
    let idx = def.zone_joints(z);
    let (arc_a, arc_b) = match &sg.path {
        Path::Arc { a, b, .. } => (Some(*a), Some(*b)),
        Path::Line => (None, None),
    };
    // the slab over the linear joints: s where every one is strictly inside
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for (k, &j) in idx.iter().enumerate() {
        if Some(j) == arc_a || Some(j) == arc_b {
            continue;
        }
        let (a, b) = (sg.q0[j], sg.q1[j]);
        let d = b - a;
        if d == 0.0 {
            if !(z.min[k] < a && a < z.max[k]) {
                return None;
            }
            continue;
        }
        let (t1, t2) = ((z.min[k] - a) / d, (z.max[k] - a) / d);
        lo = lo.max(t1.min(t2));
        hi = hi.min(t1.max(t2));
        if lo >= hi {
            return None;
        }
    }
    let inside = |q: &[f64]| {
        idx.iter()
            .enumerate()
            .all(|(k, &j)| z.min[k] < q[j] && q[j] < z.max[k])
    };
    if arc_a.is_none() || !idx.iter().any(|j| Some(*j) == arc_a || Some(*j) == arc_b) {
        let sm = 0.5 * (lo + hi);
        return inside(&sg.at(sm)).then_some(sm);
    }
    // the arc joints: bound each over a span and bisect
    let Path::Arc {
        a,
        b,
        c,
        r,
        r1,
        a0,
        sweep,
    } = &sg.path
    else {
        return None;
    };
    let bound = |s0: f64, s1: f64| -> [[f64; 2]; 2] {
        let (t0, t1) = (a0 + s0 * sweep, a0 + s1 * sweep);
        let (t0, t1) = (t0.min(t1), t0.max(t1));
        let rm = r.max(*r1) * (1.0 + 1e-12) + 1e-12;
        let rn = r.min(*r1);
        let mut cs = [f64::INFINITY, f64::NEG_INFINITY];
        let mut sn = [f64::INFINITY, f64::NEG_INFINITY];
        let mut take = |t: f64| {
            let (y, x) = dmath::sin_cos(t);
            for rr in [rm, rn] {
                cs = [cs[0].min(rr * x), cs[1].max(rr * x)];
                sn = [sn[0].min(rr * y), sn[1].max(rr * y)];
            }
        };
        take(t0);
        take(t1);
        let q = std::f64::consts::FRAC_PI_2;
        let mut k = (t0 / q).ceil();
        while k * q <= t1 {
            take(k * q);
            k += 1.0;
        }
        // a little slack for the rounding of sin/cos
        [
            [c[0] + cs[0] - 1e-9, c[0] + cs[1] + 1e-9],
            [c[1] + sn[0] - 1e-9, c[1] + sn[1] + 1e-9],
        ]
    };
    let ka = idx.iter().position(|j| j == a);
    let kb = idx.iter().position(|j| j == b);
    let mut stack = vec![(lo, hi)];
    while let Some((s0, s1)) = stack.pop() {
        let bd = bound(s0, s1);
        let miss = |kk: Option<usize>, w: [f64; 2]| kk.is_some_and(|k| w[1] <= z.min[k] || w[0] >= z.max[k]);
        if miss(ka, bd[0]) || miss(kb, bd[1]) {
            continue;
        }
        let sm = 0.5 * (s0 + s1);
        if inside(&sg.at(sm)) {
            return Some(sm);
        }
        if s1 - s0 > (hi - lo) * 2f64.powi(-30) {
            stack.push((sm, s1));
            stack.push((s0, sm));
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn rapid(
    s: &Setup,
    ln: usize,
    q0: &[f64],
    q1: &[f64],
    t0: f64,
    tool: Option<u32>,
    spindle: Option<(i64, bool)>,
    tl: &mut Timeline,
) -> Vec<Segment> {
    let def = s.def;
    let n = q0.len();
    check_range(s, ln, q1, tl);
    if q0 == q1 {
        return Vec::new();
    }
    let mk = |a: Vec<f64>, b: Vec<f64>, t0: f64, tl: &mut Timeline| -> Segment {
        // each joint's time at its rapid rate; coordinated: the slowest sets the pace
        let mut tmin = 0.0f64;
        let mut tstop = 0.0f64;
        for j in 0..n {
            let d = (b[j] - a[j]).abs();
            if d == 0.0 {
                continue;
            }
            // an unknown rapid is unbounded: no time from it (t_min stays a lower bound)
            let v = def.joints[j].rapid.unwrap_or(f64::INFINITY) / 60.0;
            tmin = tmin.max(d / v);
            match def.joints[j].accel {
                Some(acc) => tstop = tstop.max(trap(v / d, acc / d)),
                None => {
                    tl.accel_missing = true;
                    tstop = tstop.max(d / v);
                }
            }
        }
        Segment {
            line: ln,
            kind: SegKind::Rapid,
            q0: a,
            q1: b,
            path: Path::Line,
            t0,
            t_min: tmin,
            t_stop: tstop.max(tmin),
            tool,
            spindle,
        }
    };
    if !s.dogleg {
        return vec![mk(q0.to_vec(), q1.to_vec(), t0, tl)];
    }
    // dog-leg: every joint at its own rate; breakpoints where each arrives
    let times: Vec<f64> = (0..n)
        .map(|j| (q1[j] - q0[j]).abs() / (def.joints[j].rapid.unwrap_or(f64::INFINITY) / 60.0))
        .collect();
    let mut cuts: Vec<f64> = times.iter().copied().filter(|x| *x > 0.0).collect();
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let at = |tt: f64| -> Vec<f64> {
        (0..n)
            .map(|j| {
                if times[j] == 0.0 || tt >= times[j] {
                    q1[j]
                } else {
                    q0[j] + (q1[j] - q0[j]) * (tt / times[j])
                }
            })
            .collect()
    };
    let mut out = Vec::new();
    let mut prev = 0.0;
    let mut a = q0.to_vec();
    let mut tcur = t0;
    for c in cuts {
        let b = at(c);
        let mut seg = mk(a.clone(), b.clone(), tcur, tl);
        // the piece's time is the span, every joint at its own rate
        seg.t_min = c - prev;
        seg.t_stop = seg.t_stop.max(seg.t_min);
        tcur += seg.t_min;
        out.push(seg);
        prev = c;
        a = b;
    }
    out
}

fn check_range(s: &Setup, ln: usize, q: &[f64], tl: &mut Timeline) {
    for (j, jt) in s.def.joints.iter().enumerate() {
        // a cyclic rotary has no stop; an unknown range is not checked (sim::unchecked says so)
        let Some(range) = jt.range.filter(|_| !jt.cyclic) else {
            continue;
        };
        if q[j] < range[0] - 1e-9 || q[j] > range[1] + 1e-9 {
            tl.violations.push(Violation::Overtravel {
                line: ln,
                joint: jt.name.clone(),
                value: q[j],
                range,
            });
        }
    }
}

/// An arc's extreme values on its joints (the circle's axis-aligned extremes it sweeps through).
fn check_arc_range(s: &Setup, seg: &Segment, tl: &mut Timeline) {
    if let Path::Arc {
        a,
        b,
        c,
        r,
        r1,
        a0,
        sweep,
    } = &seg.path
    {
        let quarter = dmath::FRAC_PI_2;
        let (lo, hi) = if *sweep >= 0.0 {
            (*a0, a0 + sweep)
        } else {
            (a0 + sweep, *a0)
        };
        let mut k = (lo / quarter).ceil() as i64;
        while (k as f64) * quarter <= hi {
            let (sv, cv) = match k.rem_euclid(4) {
                0 => (0.0, 1.0),
                1 => (1.0, 0.0),
                2 => (0.0, -1.0),
                _ => (-1.0, 0.0),
            };
            let mut q = seg.q0.clone();
            let rs = r.max(*r1);
            q[*a] = c[0] + rs * cv;
            q[*b] = c[1] + rs * sv;
            for j in [*a, *b] {
                let jt = &s.def.joints[j];
                let Some(range) = jt.range else { continue };
                if q[j] < range[0] - 1e-9 || q[j] > range[1] + 1e-9 {
                    tl.violations.push(Violation::Overtravel {
                        line: seg.line,
                        joint: jt.name.clone(),
                        value: q[j],
                        range,
                    });
                }
            }
            k += 1;
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
fn feed_seg(
    s: &Setup,
    ln: usize,
    kind: SegKind,
    q0: &[f64],
    q1: &[f64],
    path: Path,
    f: f64,
    mode: FeedMode,
    spindle: Option<(i64, bool)>,
    css: Option<(f64, f64)>,
    t0: f64,
    tool: Option<u32>,
    tl: &mut Timeline,
) -> Result<Segment, SimError> {
    let def = s.def;
    let n = q0.len();
    check_range(s, ln, q1, tl);
    // each joint's travel over the segment (an arc's plane joints: their path lengths)
    let mut travel: Vec<f64> = (0..n).map(|j| (q1[j] - q0[j]).abs()).collect();
    let mut length_lin_sq = 0.0;
    let mut length_rot_sq = 0.0;
    if let Path::Arc {
        a, b, r, r1, sweep, ..
    } = &path
    {
        let arc = r.max(*r1) * sweep.abs();
        // a per-joint bound: a joint's travel along an arc is at most the arc length
        travel[*a] = arc;
        travel[*b] = arc;
        length_lin_sq += arc * arc;
    }
    for j in 0..n {
        if let Path::Arc { a, b, .. } = &path
            && (j == *a || j == *b)
        {
            continue;
        }
        let d = q1[j] - q0[j];
        match def.joints[j].ty {
            JointType::Linear => length_lin_sq += d * d,
            JointType::Rotary => length_rot_sq += d * d,
        }
    }
    let minutes = match mode {
        FeedMode::InverseTime => 1.0 / f,
        FeedMode::PerMin => {
            let len = if length_lin_sq == 0.0 || s.feed_combined {
                (length_lin_sq + length_rot_sq).sqrt()
            } else {
                length_lin_sq.sqrt()
            };
            if len == 0.0 { 0.0 } else { len / f }
        }
        FeedMode::PerRev => {
            let len = (length_lin_sq + length_rot_sq).sqrt();
            match (spindle, css) {
                (None, _) => {
                    return Err(SimError::At {
                        line: ln,
                        what: "G95 feed with the spindle stopped".into(),
                    });
                }
                (Some(_), Some((v, clamp))) => {
                    // G96: rpm = min(clamp, 1000 v / (2π r)) at the tip's radius r; the time is
                    // len/f · ∫ 1/rpm ds = len/f · ∫ max(1/clamp, r(s)/k) ds, k = 1000 v / 2π
                    let k = 1000.0 * v / dmath::TAU;
                    let c = 1.0 / clamp;
                    let ends = tool.and_then(|t| Some((s.tip_in_part(q0, t)?.0, s.tip_in_part(q1, t)?.0)));
                    let per_rev = match (&path, ends) {
                        (Path::Line, Some((a, b))) if a[1] == 0.0 && b[1] == 0.0 => {
                            // r(s) = |α + βs|, exact: piecewise linear between the breakpoints
                            let (al, be) = (a[0], b[0] - a[0]);
                            let mut cuts = vec![0.0, 1.0];
                            if be != 0.0 {
                                for x in [0.0, c * k, -c * k] {
                                    let u = (x - al) / be;
                                    if u > 0.0 && u < 1.0 {
                                        cuts.push(u);
                                    }
                                }
                            }
                            cuts.sort_by(f64::total_cmp);
                            let h = |u: f64| c.max((al + be * u).abs() / k);
                            cuts.windows(2)
                                .map(|w| (w[1] - w[0]) * 0.5 * (h(w[0]) + h(w[1])))
                                .sum::<f64>()
                        }
                        // elsewhere a bound: never faster than the clamp
                        _ => c,
                    };
                    len / f * per_rev
                }
                (Some((r, _)), None) if r > 0 => len / (f * r as f64),
                (Some(_), None) => {
                    return Err(SimError::At {
                        line: ln,
                        what: "G95 feed at S0".into(),
                    });
                }
            }
        }
    };
    let mut tmin = minutes * 60.0;
    // no joint faster than its max feed: stretch and say so
    for j in 0..n {
        if travel[j] == 0.0 {
            continue;
        }
        // an unknown max feed is unbounded: no clamp (sim::unchecked says so)
        let Some(max_feed) = def.joints[j].max_feed else {
            continue;
        };
        let need = travel[j] / (max_feed / 60.0);
        if need > tmin * (1.0 + 1e-12) {
            tl.violations.push(Violation::FeedClamped {
                line: ln,
                joint: def.joints[j].name.clone(),
                asked: if tmin > 0.0 {
                    travel[j] / tmin * 60.0
                } else {
                    f64::INFINITY
                },
                max: max_feed,
            });
            tmin = need;
        }
    }
    // exact stop: the path parameter's speed and acceleration limited by the slowest joint
    let mut tstop = tmin;
    if tmin > 0.0 {
        let v = 1.0 / tmin;
        let mut a = f64::INFINITY;
        for j in 0..n {
            if travel[j] == 0.0 {
                continue;
            }
            match def.joints[j].accel {
                Some(acc) => a = a.min(acc / travel[j]),
                None => tl.accel_missing = true,
            }
        }
        tstop = trap(v, a).max(tmin);
    }
    Ok(Segment {
        line: ln,
        kind,
        q0: q0.to_vec(),
        q1: q1.to_vec(),
        path,
        t0,
        t_min: tmin,
        t_stop: tstop,
        tool,
        spindle,
    })
}

/// The timeline as JSON (`forgecad.machine-sim/1`): every segment's line, kind, start time and
/// bounds, end joints; the marks; the violations; totals. Numbers printed with Rust's shortest
/// round-trip form (deterministic).
pub fn timeline_json(tl: &Timeline) -> String {
    fn esc(s: &str) -> String {
        let mut o = String::new();
        for c in s.chars() {
            match c {
                '"' => o.push_str("\\\""),
                '\\' => o.push_str("\\\\"),
                '\n' => o.push_str("\\n"),
                c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
                c => o.push(c),
            }
        }
        o
    }
    fn num(x: f64) -> String {
        if x.is_finite() {
            format!("{x:?}")
        } else {
            "null".into()
        }
    }
    let arr = |v: &[f64]| format!("[{}]", v.iter().map(|x| num(*x)).collect::<Vec<_>>().join(","));
    let mut o = String::from("{\"format\":\"forgecad.machine-sim/1\",");
    o.push_str(&format!(
        "\"joints\":[{}],",
        tl.joints
            .iter()
            .map(|j| format!("\"{}\"", esc(j)))
            .collect::<Vec<_>>()
            .join(",")
    ));
    o.push_str(&format!(
        "\"t_min\":{},\"t_stop\":{},\"accel_missing\":{},",
        num(tl.t_min),
        num(tl.t_stop),
        tl.accel_missing
    ));
    o.push_str("\"segments\":[");
    for (i, sg) in tl.segments.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let kind = match sg.kind {
            SegKind::Rapid => "rapid",
            SegKind::Feed => "feed",
            SegKind::Arc => "arc",
        };
        o.push_str(&format!(
            "{{\"line\":{},\"kind\":\"{kind}\",\"t0\":{},\"t_min\":{},\"t_stop\":{},\"q1\":{},\"tool\":{}}}",
            sg.line,
            num(sg.t0),
            num(sg.t_min),
            num(sg.t_stop),
            arr(&sg.q1),
            sg.tool.map(|t| t.to_string()).unwrap_or("null".into())
        ));
    }
    o.push_str("],\"marks\":[");
    for (i, m) in tl.marks.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        let what = match &m.what {
            Happening::ToolChange { t, seconds } => {
                format!("\"tool\",\"t\":{t},\"seconds\":{}", num(*seconds))
            }
            Happening::Index { station, seconds } => {
                format!("\"index\",\"t\":{station},\"seconds\":{}", num(*seconds))
            }
            Happening::Dwell { seconds } => format!("\"dwell\",\"seconds\":{}", num(*seconds)),
            Happening::Spindle { rpm, cw, css } => {
                format!("\"spindle\",\"s\":{rpm},\"cw\":{cw},\"css\":{css}")
            }
            Happening::SpindleStop => "\"spindle_stop\"".into(),
            Happening::Coolant(c) => format!("\"coolant\",\"mode\":\"{c:?}\""),
            Happening::Stop { optional } => format!("\"stop\",\"optional\":{optional}"),
            Happening::Comment(c) => format!("\"comment\",\"text\":\"{}\"", esc(c)),
            Happening::End => "\"end\"".into(),
        };
        o.push_str(&format!(
            "{{\"line\":{},\"at\":{},\"what\":{what}}}",
            m.line,
            num(m.t)
        ));
    }
    o.push_str("],\"violations\":[");
    for (i, v) in tl.violations.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        o.push_str(&match v {
            Violation::Overtravel {
                line,
                joint,
                value,
                range,
            } => format!(
                "{{\"kind\":\"overtravel\",\"line\":{line},\"joint\":\"{}\",\"value\":{},\"range\":{}}}",
                esc(joint),
                num(*value),
                arr(range)
            ),
            Violation::FeedClamped {
                line,
                joint,
                asked,
                max,
            } => format!(
                "{{\"kind\":\"feed_clamped\",\"line\":{line},\"joint\":\"{}\",\"asked\":{},\"max\":{}}}",
                esc(joint),
                num(*asked),
                num(*max)
            ),
            Violation::NoTool { line, t } => format!("{{\"kind\":\"no_tool\",\"line\":{line},\"t\":{t}}}"),
            Violation::Rpm { line, rpm, range } => format!(
                "{{\"kind\":\"rpm\",\"line\":{line},\"rpm\":{rpm},\"range\":{}}}",
                arr(range)
            ),
            Violation::Pocket { line, t, max } => {
                format!("{{\"kind\":\"pocket\",\"line\":{line},\"t\":{t},\"max\":{max}}}")
            }
            Violation::Zone { line, zone, at } => format!(
                "{{\"kind\":\"zone\",\"line\":{line},\"zone\":\"{}\",\"at\":[{}]}}",
                esc(zone),
                at.iter().map(|v| num(*v)).collect::<Vec<_>>().join(",")
            ),
        });
    }
    o.push_str("]}");
    o
}

/// A check the timeline could not make: the value it needs is unknown (omitted) or only an
/// estimate (docs/cam-machine.md, "Provenance": never a silent pass on a stand-in).
#[derive(Clone, Debug, PartialEq)]
pub struct Unchecked {
    /// The value, as a cite path: `joint.X.range`, `spindle.S1.rpm`, `changer.pockets`, `body.head`.
    pub path: String,
    /// "unknown" or "estimated".
    pub why: &'static str,
    /// The first program line whose check needed it (0: the machine itself, e.g. a body).
    pub line: usize,
    /// What was not checked.
    pub check: &'static str,
}

/// Every check of `tl` against `def` made against an unknown or estimated value, or not made at
/// all for want of one, each value once (its first line).
pub fn unchecked(def: &MachineDef, tl: &Timeline) -> Vec<Unchecked> {
    use crate::def::CiteStatus;
    let mut out: Vec<Unchecked> = Vec::new();
    let mut note = |path: String, known: bool, line: usize, check: &'static str| {
        let why = if !known {
            "unknown"
        } else {
            match def.cite_status(&path) {
                Some(CiteStatus::Unknown) => "unknown",
                Some(CiteStatus::Estimate) => "estimated",
                _ => return,
            }
        };
        if !out.iter().any(|u| u.path == path) {
            out.push(Unchecked {
                path,
                why,
                line,
                check,
            });
        }
    };
    for sg in &tl.segments {
        for (j, jt) in def.joints.iter().enumerate() {
            if sg.q0.get(j) == sg.q1.get(j) {
                continue;
            }
            if !jt.cyclic {
                note(
                    format!("joint.{}.range", jt.name),
                    jt.range.is_some(),
                    sg.line,
                    "overtravel",
                );
            }
            match sg.kind {
                SegKind::Rapid => note(
                    format!("joint.{}.rapid", jt.name),
                    jt.rapid.is_some(),
                    sg.line,
                    "rapid time",
                ),
                SegKind::Feed | SegKind::Arc => note(
                    format!("joint.{}.max_feed", jt.name),
                    jt.max_feed.is_some(),
                    sg.line,
                    "feed clamp",
                ),
            }
        }
    }
    for m in &tl.marks {
        match &m.what {
            Happening::Spindle { .. } => {
                if let Some(sd) = def.spindles.first() {
                    note(
                        format!("spindle.{}.rpm", sd.name),
                        sd.rpm.is_some(),
                        m.line,
                        "rpm range",
                    );
                }
            }
            Happening::ToolChange { .. } => {
                if let Some(c) = &def.changer {
                    note(
                        "changer.position".into(),
                        !c.position.is_empty(),
                        m.line,
                        "tool change move",
                    );
                    note(
                        "changer.pockets".into(),
                        c.pockets.is_some() || c.manual,
                        m.line,
                        "pocket count",
                    );
                    note(
                        "changer.change_s".into(),
                        c.change_s.is_some(),
                        m.line,
                        "tool change time",
                    );
                }
            }
            Happening::Index { .. } => {
                if let Some(tu) = def.turrets.first() {
                    note(
                        format!("turret.{}.stations", tu.name),
                        tu.stations.is_some() || tu.manual,
                        m.line,
                        "station count",
                    );
                    note(
                        format!("turret.{}.index_s", tu.name),
                        tu.index_s.is_some(),
                        m.line,
                        "index time",
                    );
                }
            }
            _ => {}
        }
    }
    for b in &def.bodies {
        note(format!("body.{}", b.name), !b.prims.is_empty(), 0, "collision");
    }
    out
}

/// The peck parameters a program's G83/G73 used, when the machine does not state them (forge-post's
/// 0.5 / 0.2 mm then stand in) or cites them unknown or estimated. Comments are skipped.
pub fn unchecked_cycles(def: &MachineDef, program: &str) -> Vec<Unchecked> {
    use crate::def::CiteStatus;
    let mut out = Vec::new();
    for (i, raw) in program.lines().enumerate() {
        let mut text = String::new();
        let mut depth = 0;
        for ch in raw.chars() {
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                ';' if depth == 0 => break,
                _ if depth == 0 => text.push(ch.to_ascii_uppercase()),
                _ => {}
            }
        }
        // a G word with this number (G83, G083), not G83.1
        let g = |code: u32| {
            text.match_indices('G').any(|(at, _)| {
                let n: String = text[at + 1..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                let after = text[at + 1 + n.len()..].chars().next();
                n.parse::<u32>() == Ok(code) && after != Some('.')
            })
        };
        for (code, path, value, check) in [
            (83, "machine.peck_clear", def.peck_clear, "G83 peck clearance"),
            (73, "machine.chip_lift", def.chip_lift, "G73 chip-break lift"),
        ] {
            if !g(code) || out.iter().any(|u: &Unchecked| u.path == path) {
                continue;
            }
            let why = match (value, def.cite_status(path)) {
                (None, _) | (_, Some(CiteStatus::Unknown)) => "unknown",
                (_, Some(CiteStatus::Estimate)) => "estimated",
                _ => continue,
            };
            out.push(Unchecked {
                path: path.into(),
                why,
                line: i + 1,
                check,
            });
        }
    }
    out
}

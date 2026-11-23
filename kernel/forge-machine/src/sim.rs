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
                        if *tn > tu.stations || *tn == 0 {
                            tl.violations.push(Violation::Pocket {
                                line: ln,
                                t: *tn,
                                max: tu.stations,
                            });
                        }
                        let from = tool.unwrap_or(1) as i64;
                        let d = (*tn as i64 - from).rem_euclid(tu.stations as i64);
                        let steps = d.min(tu.stations as i64 - d) as f64;
                        let sec = steps * tu.index_s;
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
                        Some(c) => (c.change_s, c.pockets),
                        None => {
                            return Err(SimError::At {
                                line: ln,
                                what: "M6 on a machine with no tool changer".into(),
                            });
                        }
                    };
                    if *tn > max {
                        tl.violations.push(Violation::Pocket {
                            line: ln,
                            t: *tn,
                            max,
                        });
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
                let rpm = if *css {
                    // the clamp bounds it; the true rpm follows the radius, so check the clamp
                    rpm_clamp.unwrap_or(i64::MAX)
                } else {
                    *sp
                };
                if let Some(sd) = def.spindles.first() {
                    let check = if *css { rpm_clamp } else { Some(rpm) };
                    if let Some(r) = check
                        && ((r as f64) < sd.rpm[0] || (r as f64) > sd.rpm[1])
                    {
                        tl.violations.push(Violation::Rpm {
                            line: ln,
                            rpm: r,
                            range: sd.rpm,
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
    Ok(tl)
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
            let v = def.joints[j].rapid / 60.0;
            tmin = tmin.max(d / v);
            match def.joints[j].accel {
                Some(acc) => tstop = tstop.max(d * trap(v / d, acc / d)),
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
        .map(|j| (q1[j] - q0[j]).abs() / (def.joints[j].rapid / 60.0))
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
        if jt.cyclic {
            continue;
        }
        if q[j] < jt.range[0] - 1e-9 || q[j] > jt.range[1] + 1e-9 {
            tl.violations.push(Violation::Overtravel {
                line: ln,
                joint: jt.name.clone(),
                value: q[j],
                range: jt.range,
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
                if q[j] < jt.range[0] - 1e-9 || q[j] > jt.range[1] + 1e-9 {
                    tl.violations.push(Violation::Overtravel {
                        line: seg.line,
                        joint: jt.name.clone(),
                        value: q[j],
                        range: jt.range,
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
            let rpm = match spindle {
                Some((r, _)) if r > 0 => r as f64,
                _ => {
                    return Err(SimError::At {
                        line: ln,
                        what: "G95 feed with the spindle stopped".into(),
                    });
                }
            };
            let len = (length_lin_sq + length_rot_sq).sqrt();
            len / (f * rpm)
        }
    };
    let mut tmin = minutes * 60.0;
    // no joint faster than its max feed: stretch and say so
    for j in 0..n {
        if travel[j] == 0.0 {
            continue;
        }
        let need = travel[j] / (def.joints[j].max_feed / 60.0);
        if need > tmin * (1.0 + 1e-12) {
            tl.violations.push(Violation::FeedClamped {
                line: ln,
                joint: def.joints[j].name.clone(),
                asked: if tmin > 0.0 {
                    travel[j] / tmin * 60.0
                } else {
                    f64::INFINITY
                },
                max: def.joints[j].max_feed,
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
        });
    }
    o.push_str("]}");
    o
}

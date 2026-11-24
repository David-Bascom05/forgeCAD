//! The machine simulation: a setup's posted program interpreted on its machine
//! (forge-machine: the text to joint motion, timed, every body checked through it), made into
//! the simulation's own timeline so the stock, the toolpath and the playback all follow what the
//! posted text makes the machine do, and the machine's bodies posed at any time.
//!
//! The view stays in the part's frame: the machine is drawn moving about the part (as the tool
//! sees it), so the workspace's part and the stock keep their places.

use super::Marker;
use super::timeline::{ArcOf, Kind, OpSpan, Seg, Timeline};
use crate::origin::WorldVert;
use forge_cam_lang::Cam;
use forge_cam_lang::sim::MachineRun;
use forge_machine::collide::Verdict;
use forge_machine::def::Prim;
use forge_machine::kin::{self, Rigid};
use forge_machine::sim::{Happening, Path, SegKind, Violation};
use forge_math::dmath;

/// A setup's program on its machine, as the simulation plays it.
pub struct MachineView {
    pub run: MachineRun,
    pub setup: usize,
    pub tl: Timeline,
    /// Per piece of `tl`: the machine segment it is.
    pub seg_of: Vec<usize>,
    pub markers: Vec<Marker>,
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// A circle in z = const through three points: centre and radius (None if they are in a line).
fn circle3(p: [f64; 3], q: [f64; 3], r: [f64; 3]) -> Option<([f64; 2], f64)> {
    let (ax, ay, bx, by, cx, cy) = (p[0], p[1], q[0], q[1], r[0], r[1]);
    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d.abs() < 1e-12 {
        return None;
    }
    let (a2, b2, c2) = (ax * ax + ay * ay, bx * bx + by * by, cx * cx + cy * cy);
    let ux = (a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by)) / d;
    let uy = (a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax)) / d;
    Some(([ux, uy], ((ax - ux).powi(2) + (ay - uy).powi(2)).sqrt()))
}

impl MachineView {
    /// The view of setup `setup`'s machine run.
    pub fn of(cam: &Cam, run: MachineRun, setup: usize) -> MachineView {
        let tools = cam.sim_tools();
        // the program's comments name its ops: each op's lines start at its comment
        // (a post may drop the parentheses inside a comment: compare the words)
        let words = |s: &str| {
            s.replace(['(', ')'], " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        };
        let names: Vec<String> = cam
            .ops
            .iter()
            .map(|o| words(&format!("{} ({})", o.name, o.kind)))
            .collect();
        let mut starts: Vec<(usize, usize)> = vec![];
        for m in &run.tl.marks {
            if let Happening::Comment(c) = &m.what
                && let Some(k) = names.iter().position(|n| *n == words(c))
            {
                starts.push((m.line, k));
            }
        }
        let op_of_line = |line: usize| starts.iter().rev().find(|(l, _)| *l <= line).map(|x| x.1);
        let mut segs = vec![];
        let mut seg_of = vec![];
        let mut count = vec![0usize; cam.ops.len()];
        for (k, s) in run.tl.segments.iter().enumerate() {
            let (Some(t), Some(op)) = (s.tool, op_of_line(s.line)) else {
                continue;
            };
            let Some(ti) = tools.iter().position(|u| u.t == t) else {
                continue;
            };
            let (Some(a), Some(b)) = (run.tip(&s.q0, t), run.tip(&s.q1, t)) else {
                continue;
            };
            let arc = match &s.path {
                Path::Arc {
                    a: ja, b: jb, c: jc, ..
                } => (|| {
                    // the machine's centre in the part's frame (the arc's two joints are linear
                    // axes, so the tip at the centre's joint values is the centre; else three
                    // points' circle); the radius from the start's to the end's, as the machine
                    // blends it; the sweep by unwrapping the angle along it, an eighth at a time
                    let p: Vec<[f64; 3]> = (0..=8)
                        .map(|k| run.tip(&s.at(k as f64 / 8.0), t))
                        .collect::<Option<_>>()?;
                    let linear = |j: usize| run.def.joints[j].ty == forge_machine::def::JointType::Linear;
                    let c = if linear(*ja) && linear(*jb) {
                        let mut q = s.at(0.0);
                        q[*ja] = jc[0];
                        q[*jb] = jc[1];
                        let o = run.tip(&q, t)?;
                        [o[0], o[1]]
                    } else {
                        circle3(p[2], p[4], p[6])?.0
                    };
                    let rad = |q: [f64; 3]| ((q[0] - c[0]).powi(2) + (q[1] - c[1]).powi(2)).sqrt();
                    let r = rad(p[0]);
                    let dr = rad(p[8]) - r;
                    let ang = |q: [f64; 3]| dmath::atan2(q[1] - c[1], q[0] - c[0]);
                    let a0 = ang(p[0]);
                    let mut sweep = 0.0;
                    let mut prev = a0;
                    for q in &p[1..] {
                        let x = ang(*q);
                        let mut d = x - prev;
                        while d > std::f64::consts::PI {
                            d -= std::f64::consts::TAU;
                        }
                        while d < -std::f64::consts::PI {
                            d += std::f64::consts::TAU;
                        }
                        sweep += d;
                        prev = x;
                    }
                    Some(ArcOf { c, r, dr, a0, sweep })
                })(),
                Path::Line => None,
            };
            let d = sub(b, a);
            let kind = match s.kind {
                SegKind::Rapid => Kind::Rapid,
                SegKind::Feed if d[0].abs() < 1e-9 && d[1].abs() < 1e-9 && d[2] < 0.0 => Kind::Plunge,
                SegKind::Feed | SegKind::Arc => Kind::Cut,
            };
            let len = match arc {
                Some(ar) => ((ar.r * ar.sweep).powi(2) + d[2].powi(2)).sqrt(),
                None => (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt(),
            };
            let dt = s.t_stop.max(s.t_min);
            segs.push(Seg {
                op: op as u32,
                path: 0,
                mv: count[op] as u32,
                tool: ti as u32,
                kind,
                a,
                b,
                arc,
                f: if dt > 0.0 { len / dt * 60.0 } else { 0.0 },
                t0: s.t0,
                dt,
                len,
            });
            count[op] += 1;
            seg_of.push(k);
        }
        let mut ops: Vec<OpSpan> = cam
            .ops
            .iter()
            .map(|o| OpSpan {
                name: o.name.clone(),
                kind: o.kind,
                setup: o.setup,
                status: o.status,
                first: 0,
                end: 0,
                t0: 0.0,
                t1: 0.0,
                tools: vec![],
            })
            .collect();
        for (i, g) in segs.iter().enumerate() {
            let sp = &mut ops[g.op as usize];
            if sp.end == 0 {
                sp.first = i;
                sp.t0 = g.t0;
            }
            sp.end = i + 1;
            sp.t1 = g.t1();
            if !sp.tools.contains(&g.tool) {
                sp.tools.push(g.tool);
            }
        }
        // an op the program doesn't hold: an empty span where it would be
        let mut last = 0.0f64;
        for sp in &mut ops {
            if sp.end == 0 {
                sp.t0 = last;
                sp.t1 = last;
            }
            last = sp.t1;
        }
        let total = run.tl.t_stop.max(segs.last().map_or(0.0, |g| g.t1()));
        let tl = Timeline { segs, ops, total };
        let mut mv = MachineView {
            run,
            setup,
            tl,
            seg_of,
            markers: vec![],
        };
        mv.markers = mv.markers_of();
        mv
    }

    /// The machine segment holding time t, and s along it.
    fn seg_at(&self, t: f64) -> Option<(usize, f64)> {
        let segs = &self.run.tl.segments;
        if segs.is_empty() {
            return None;
        }
        let i = segs
            .partition_point(|s| s.t0 + s.t_stop.max(s.t_min) <= t)
            .min(segs.len() - 1);
        let s = &segs[i];
        let dt = s.t_stop.max(s.t_min);
        Some((
            i,
            if dt > 0.0 {
                ((t - s.t0) / dt).clamp(0.0, 1.0)
            } else {
                1.0
            },
        ))
    }

    /// The joint values at time t.
    pub fn q_at(&self, t: f64) -> Vec<f64> {
        match self.seg_at(t) {
            Some((i, s)) => self.run.tl.segments[i].at(s),
            None => self.run.def.joints.iter().map(|j| j.home).collect(),
        }
    }

    /// Base coordinates to the document's part frame, at joint values q.
    pub fn to_part(&self, q: &[f64]) -> Rigid {
        let def = &self.run.def;
        let pose = kin::pose(def, q);
        let part = self.run.setup().part_mount.clone();
        let f = kin::part_frame(def, &pose, &part).unwrap_or(Rigid::ID);
        let shift = Rigid {
            r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            p: self.run.shift,
        };
        // part (doc) → part (machine) → base, inverted
        f.then(&shift).inverse()
    }

    /// The joints' values at t, by name, for the panel.
    pub fn joints_at(&self, t: f64) -> Vec<(String, f64)> {
        let q = self.q_at(t);
        self.run
            .def
            .joints
            .iter()
            .zip(q)
            .map(|(j, v)| (j.name.clone(), v))
            .collect()
    }

    /// The overtravel, collision and other issues the machine run found, as markers.
    fn markers_of(&self) -> Vec<Marker> {
        let mut out = vec![];
        let segs = &self.run.tl.segments;
        let at_line = |line: usize| segs.iter().position(|s| s.line == line);
        let tip_at = |i: usize, s: f64| {
            let g = &segs[i];
            g.tool.and_then(|t| self.run.tip(&g.at(s), t)).unwrap_or([0.0; 3])
        };
        let op_at = |t: f64| {
            let i = self.tl.seg_at(t)?;
            Some(self.tl.segs[i].op as usize)
        };
        for c in &self.run.contacts.contacts {
            let g = &segs[c.segment];
            let (s, word) = match c.verdict {
                Verdict::Hit { s, .. } => (s, "hits"),
                Verdict::Near { s0, .. } => (s0, "comes within the clearance of"),
                Verdict::Clear => continue,
            };
            let t = g.t0 + s * g.t_stop.max(g.t_min);
            out.push(Marker {
                op: op_at(t),
                setup: self.setup,
                rule: "machine_collision".into(),
                verdict: if matches!(c.verdict, Verdict::Hit { .. }) {
                    "fail".into()
                } else {
                    "near".into()
                },
                // a hit's witness is in the machine's base frame at s: into the part's
                at: match c.verdict {
                    Verdict::Hit { witness, .. } => self.to_part(&g.at(s)).apply(witness),
                    _ => tip_at(c.segment, s),
                },
                t,
                text: format!(
                    "line {}: {} {word} {} (posted program on {})",
                    c.line, c.b, c.a, self.run.def.name
                ),
                region: None,
            });
        }
        for v in &self.run.tl.violations {
            let (line, text) = match v {
                Violation::Overtravel {
                    line,
                    joint,
                    value,
                    range,
                } => (
                    *line,
                    format!(
                        "line {line}: {joint} overtravel {value:.3} outside {:.1}..{:.1}",
                        range[0], range[1]
                    ),
                ),
                Violation::FeedClamped {
                    line,
                    joint,
                    asked,
                    max,
                } => (
                    *line,
                    format!("line {line}: feed clamped on {joint}: {asked:.0} asked, {max:.0} max"),
                ),
                Violation::NoTool { line, t } => (*line, format!("line {line}: T{t} is not in the machine")),
                Violation::Rpm { line, rpm, range } => (
                    *line,
                    format!("line {line}: S{rpm} outside {:.0}..{:.0} rpm", range[0], range[1]),
                ),
                Violation::Pocket { line, t, max } => (
                    *line,
                    format!("line {line}: T{t} beyond the changer's {max} pockets"),
                ),
                Violation::Zone { line, zone, .. } => (
                    *line,
                    format!("line {line}: enters the machine's forbidden zone `{zone}`"),
                ),
            };
            let Some(i) = at_line(line) else { continue };
            let t = segs[i].t0;
            out.push(Marker {
                op: op_at(t),
                setup: self.setup,
                rule: match v {
                    Violation::Overtravel { .. } => "overtravel",
                    Violation::FeedClamped { .. } => "feed_clamped",
                    _ => "machine",
                }
                .into(),
                verdict: "fail".into(),
                at: tip_at(i, 0.0),
                t,
                text,
                region: None,
            });
        }
        out.sort_by(|a, b| a.t.total_cmp(&b.t));
        out
    }

    /// The machine's bodies at time t, in the part's frame (shaded); `hit` names drawn red.
    pub fn draw(&self, t: f64, hit: &[String], out: &mut Vec<WorldVert>) {
        let def = &self.run.def;
        let q = self.q_at(t);
        let pose = kin::pose(def, &q);
        let w = self.to_part(&q);
        for b in &def.bodies {
            let l = kin::link(def, &pose, &b.link);
            let m = w.then(l);
            let base = if hit.iter().any(|h| h == &b.name) {
                super::draw::look::FAIL
            } else {
                match b.name.as_str() {
                    n if n.contains("table") || n.contains("bed") || n.contains("base") => {
                        crate::viewcube::rgb(0x4A5260)
                    }
                    n if n.contains("spindle") || n.contains("head") => crate::viewcube::rgb(0x8C939E),
                    _ => crate::viewcube::rgb(0x6A7380),
                }
            };
            for p in &b.prims {
                prim(&m, p, base, out);
            }
        }
    }
}

fn f3(p: [f64; 3]) -> [f32; 3] {
    [p[0] as f32, p[1] as f32, p[2] as f32]
}

/// One primitive placed by m, shaded.
fn prim(m: &Rigid, p: &Prim, c: [f32; 3], out: &mut Vec<WorldVert>) {
    let quad = |q: [[f64; 3]; 4], n: [f64; 3], out: &mut Vec<WorldVert>| {
        let col = super::draw::shade_pub(c, f3(m.dir(n)));
        let v = q.map(|x| f3(m.apply(x)));
        for k in [0, 1, 2, 0, 2, 3] {
            out.push(WorldVert {
                at: v[k],
                colour: col,
            });
        }
    };
    match *p {
        Prim::Box { min: a, max: b } => {
            let v = |x: bool, y: bool, z: bool| {
                [
                    if x { b[0] } else { a[0] },
                    if y { b[1] } else { a[1] },
                    if z { b[2] } else { a[2] },
                ]
            };
            quad(
                [
                    v(false, false, true),
                    v(true, false, true),
                    v(true, true, true),
                    v(false, true, true),
                ],
                [0.0, 0.0, 1.0],
                out,
            );
            quad(
                [
                    v(false, false, false),
                    v(false, true, false),
                    v(true, true, false),
                    v(true, false, false),
                ],
                [0.0, 0.0, -1.0],
                out,
            );
            quad(
                [
                    v(true, false, false),
                    v(true, true, false),
                    v(true, true, true),
                    v(true, false, true),
                ],
                [1.0, 0.0, 0.0],
                out,
            );
            quad(
                [
                    v(false, false, false),
                    v(false, false, true),
                    v(false, true, true),
                    v(false, true, false),
                ],
                [-1.0, 0.0, 0.0],
                out,
            );
            quad(
                [
                    v(false, true, false),
                    v(false, true, true),
                    v(true, true, true),
                    v(true, true, false),
                ],
                [0.0, 1.0, 0.0],
                out,
            );
            quad(
                [
                    v(false, false, false),
                    v(true, false, false),
                    v(true, false, true),
                    v(false, false, true),
                ],
                [0.0, -1.0, 0.0],
                out,
            );
        }
        Prim::Cyl { base, axis, r, h } => {
            // a frame about the axis
            let u = if axis[2].abs() < 0.9 {
                [0.0, 0.0, 1.0]
            } else {
                [1.0, 0.0, 0.0]
            };
            let cr = |a: [f64; 3], b: [f64; 3]| {
                [
                    a[1] * b[2] - a[2] * b[1],
                    a[2] * b[0] - a[0] * b[2],
                    a[0] * b[1] - a[1] * b[0],
                ]
            };
            let e1 = cr(axis, u);
            let l = (e1[0] * e1[0] + e1[1] * e1[1] + e1[2] * e1[2]).sqrt();
            let e1 = [e1[0] / l, e1[1] / l, e1[2] / l];
            let e2 = cr(axis, e1);
            let top = [
                base[0] + axis[0] * h,
                base[1] + axis[1] * h,
                base[2] + axis[2] * h,
            ];
            let n = 32;
            let at = |o: [f64; 3], k: usize| {
                let (s, c) = dmath::sin_cos(std::f64::consts::TAU * k as f64 / n as f64);
                [
                    o[0] + r * (c * e1[0] + s * e2[0]),
                    o[1] + r * (c * e1[1] + s * e2[1]),
                    o[2] + r * (c * e1[2] + s * e2[2]),
                ]
            };
            for k in 0..n {
                let (s, c) = dmath::sin_cos(std::f64::consts::TAU * (k as f64 + 0.5) / n as f64);
                let nn = [
                    c * e1[0] + s * e2[0],
                    c * e1[1] + s * e2[1],
                    c * e1[2] + s * e2[2],
                ];
                quad(
                    [at(base, k), at(base, k + 1), at(top, k + 1), at(top, k)],
                    nn,
                    out,
                );
                // the caps as thin fans (two triangles per step, the second degenerate)
                quad([top, top, at(top, k), at(top, k + 1)], axis, out);
                quad(
                    [base, base, at(base, k + 1), at(base, k)],
                    [-axis[0], -axis[1], -axis[2]],
                    out,
                );
            }
        }
    }
}

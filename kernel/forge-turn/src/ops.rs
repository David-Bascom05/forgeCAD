//! The lathe setup and its ops.
//!
//! Every op is a path of the tool's reference point (the nose centre; a blade's left corner
//! centre; a drill's tip) in the (ρ, z) half-plane: rapids, feeds, arcs and thread passes. The
//! finishing path is the exact nose-radius compensation: the boundary of H ⊕ B_{r_ε + leave},
//! the exact offset of the half-profile (forge-cam-geom's offset, its vertices on a 2^-32 grid),
//! so the nose disc rolls on the part exactly; the program states the imaginary tip (G40), the
//! nose centre plus the tool's fixed tip vector, so the control adds nothing of its own.
//!
//! `certify` checks every move of every op, as posted (the quantised program read back), with
//! check.rs: the cutter's bodies against the part grown by the op's leave (G1), every body against
//! the chuck by the fixture clearance (G2), every body on a rapid and the non-cutting bodies on
//! every move against the in-process stock (G3), each axis position against the machine's
//! travel (G5). The stock is the bar less what each cut provably cleared (convex polygons inside
//! the cutter's outline swept along each piece of at most SUB of the cut): a superset of the true
//! stock, so a clearance from it is sound. The bodies that trail the cutter are checked piece by
//! piece against the stock as the pieces before left it. At the end, the leftover (G4): the final stock less the part grown by tol, exactly,
//! each region listed.
use crate::check::{self, Move, Obstacle, Verdict};
use crate::insert::{Body, Kind, TurnTool, tan};
use crate::machine::{Lathe, Place};
use crate::profile::{HalfProfile, poly, rect};
use forge_cam_geom::boolean::{self, Op};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::offset::offset;
use forge_cam_geom::region::{Loop, Region, Seg};
use forge_math::dmath;

/// The work holding: a three-jaw chuck's envelope as it turns (a region of revolution).
#[derive(Clone, Debug, PartialEq)]
pub struct Chuck {
    /// The jaws' face (their +z end), in part coordinates.
    pub z_face: f64,
    /// How far the jaws grip along the bar (−z from the face).
    pub grip: f64,
    /// The jaws stand this far out from the bar's surface.
    pub jaw_height: f64,
    /// The chuck body: radius, and length behind the jaws.
    pub body_r: f64,
    pub body_len: f64,
}

/// A lathe setup: the part's half-profile, the bar, the chuck and the clearances.
pub struct Setup {
    pub part: HalfProfile,
    /// The bar's radius, its front face (z), the chuck.
    pub bar_r: f64,
    pub bar_front: f64,
    pub chuck: Chuck,
    /// The fixture clearance (G2), the holder clearance (non-cutting bodies from part and stock),
    /// the rapid clearance (every body from the stock on a rapid), the gouge tolerance.
    pub fixture_clearance: f64,
    pub holder_clearance: f64,
    pub rapid_clearance: f64,
    pub tol: f64,
    /// The lathe (CAM-MACHINE's definition, machine.rs) and where the work and the tools sit on
    /// it: G5 is checked in full when both are given, by spans only when the place is not.
    pub machine: Option<Lathe>,
    pub place: Option<Place>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SetupError {
    Bad(String),
}

impl Setup {
    /// The chuck's envelope: its body and the jaws round the bar.
    pub fn chuck_region(&self) -> Region {
        let c = &self.chuck;
        let zb1 = c.z_face - c.grip;
        let zb0 = zb1 - c.body_len;
        let rj = self.bar_r + c.jaw_height;
        Region {
            loops: vec![poly(&[
                [0.0, zb0],
                [c.body_r.max(rj), zb0],
                [c.body_r.max(rj), zb1],
                [rj, zb1],
                [rj, c.z_face],
                [self.bar_r, c.z_face],
                [self.bar_r, zb1],
                [0.0, zb1],
            ])],
        }
    }

    /// The bar as loaded: from the jaws' back to its front face.
    pub fn bar(&self) -> Region {
        Region {
            loops: vec![rect(
                0.0,
                self.chuck.z_face - self.chuck.grip,
                self.bar_r,
                self.bar_front,
            )],
        }
    }

    pub fn check(&self) -> Result<(), SetupError> {
        let hb = self.part.bbox;
        if hb[2] > self.bar_r {
            return Err(SetupError::Bad(format!(
                "the part (radius {}) is larger than the bar (radius {})",
                hb[2], self.bar_r
            )));
        }
        if hb[3] > self.bar_front {
            return Err(SetupError::Bad("the part stands past the bar's front".into()));
        }
        if hb[1] < self.chuck.z_face + self.fixture_clearance {
            return Err(SetupError::Bad(format!(
                "the part reaches z = {} but the jaws' face is at {}: pull the bar out to at least {} past the jaws",
                hb[1], self.chuck.z_face, self.fixture_clearance
            )));
        }
        Ok(())
    }
}

/// One step of a path (positions of the reference point).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    Rapid([f64; 2]),
    Feed([f64; 2]),
    /// An arc to `to` about `c`, counter-clockwise in (ρ, z) when `ccw`.
    Arc {
        to: [f64; 2],
        c: [f64; 2],
        ccw: bool,
    },
    /// A thread pass to `to` at `pitch` per revolution.
    Thread {
        to: [f64; 2],
        pitch: f64,
    },
}

impl Step {
    pub fn to(&self) -> [f64; 2] {
        match *self {
            Step::Rapid(p) | Step::Feed(p) => p,
            Step::Arc { to, .. } | Step::Thread { to, .. } => to,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spindle {
    /// Constant surface speed (m/min), with a top speed.
    Css {
        vc: f64,
        max_rpm: f64,
    },
    Rpm(f64),
}

/// An op's path, ready to certify and post.
#[derive(Clone, Debug, PartialEq)]
pub struct OpPath {
    pub name: String,
    pub tool: TurnTool,
    /// The turret station (and offset number).
    pub t: u32,
    pub spindle: Spindle,
    /// Feed per revolution.
    pub f: f64,
    /// The part's leave for this op (G1 holds the cutter that far from the part).
    pub leave: f64,
    pub start: [f64; 2],
    pub steps: Vec<Step>,
    /// A declared removal (a thread's band): the part as this op is held to is P ∖ Δ.
    pub removal: Option<Region>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OpError {
    Tool(String),
    Geometry(String),
}

/// How far ahead of the stock a pass starts.
pub const APPROACH: f64 = 1.0;
/// A pass's retract, at 45° away from the cut.
pub const RETRACT: f64 = 0.5;

fn css(t: &TurnTool) -> Spindle {
    Spindle::Css {
        vc: t.vc,
        max_rpm: 3000.0,
    }
}

/// Facing: passes of depth ≤ ap from the bar's front down to z_face, the nose from outside the
/// bar to the centre line (its centre on ρ = 0).
pub fn face(s: &Setup, tool: &TurnTool, t: u32, z_face: f64, ap: f64) -> Result<OpPath, OpError> {
    if tool.kind != Kind::Turn {
        return Err(OpError::Tool(format!(
            "facing needs a turning tool, not `{}`",
            tool.id
        )));
    }
    if !(z_face < s.bar_front) {
        return Err(OpError::Geometry(
            "nothing to face: the face is at or past the bar's front".into(),
        ));
    }
    let re = tool.re;
    let x_out = s.bar_r + APPROACH + re;
    let n = ((s.bar_front - z_face) / ap).ceil().max(1.0) as usize;
    let start = [x_out, s.bar_front + APPROACH + re];
    let mut steps = vec![Step::Rapid(start)];
    for k in 1..=n {
        let z = if k == n {
            z_face
        } else {
            s.bar_front - (s.bar_front - z_face) * k as f64 / n as f64
        };
        let zc = z + re;
        steps.push(Step::Rapid([x_out, zc]));
        steps.push(Step::Feed([0.0, zc]));
        steps.push(Step::Feed([RETRACT, zc + RETRACT]));
        steps.push(Step::Rapid([x_out, zc + RETRACT]));
    }
    steps.push(Step::Rapid(start));
    Ok(OpPath {
        name: format!("face to z = {z_face}"),
        tool: tool.clone(),
        t,
        spindle: css(tool),
        f: tool.f,
        leave: 0.0,
        start,
        steps,
        removal: None,
    })
}

/// The loop of the offset H ⊕ B_d with the largest area (the outer boundary), counter-clockwise.
fn grown(s: &Setup, d: f64) -> Result<Loop, OpError> {
    let (loops, _) = offset(&s.part.region, -d).map_err(|e| OpError::Geometry(format!("offset: {e:?}")))?;
    let mut best: Option<(f64, Loop)> = None;
    for l in loops {
        let lp = Loop { segs: l.segs };
        let a = crate::profile::area2(&lp);
        if best.as_ref().is_none_or(|(b, _)| a.abs() > *b) {
            best = Some((a.abs(), lp));
        }
    }
    let (_, lp) = best.ok_or(OpError::Geometry("the grown part is empty".into()))?;
    Ok(if crate::profile::area2(&lp) < 0.0 {
        lp.reversed()
    } else {
        lp
    })
}

/// The heights where the vertical line ρ = x meets a piece (f64; the checker certifies).
fn meet_vertical(s: &Seg, x: f64, out: &mut Vec<f64>) {
    match *s {
        Seg::Line { a, b } => {
            if (a.x - x) * (b.x - x) <= 0.0 && a.x != b.x {
                let t = (x - a.x) / (b.x - a.x);
                out.push(a.y + t * (b.y - a.y));
            } else if a.x == x && b.x == x {
                out.push(a.y.max(b.y));
            }
        }
        Seg::Arc { a, b, c, ccw } => {
            let r = ((a.x - c.x).powi(2) + (a.y - c.y).powi(2)).sqrt();
            let dx = x - c.x;
            if dx.abs() > r {
                return;
            }
            let h = (r * r - dx * dx).sqrt();
            for y in [c.y + h, c.y - h] {
                if on_arc(a, b, c, ccw, [x, y]) {
                    out.push(y);
                }
            }
        }
    }
}

fn on_arc(
    a: forge_cam_geom::region::P2,
    b: forge_cam_geom::region::P2,
    c: forge_cam_geom::region::P2,
    ccw: bool,
    p: [f64; 2],
) -> bool {
    let ang = |x: f64, y: f64| dmath::atan2(y - c.y, x - c.x);
    let (t0, t1, tp) = (ang(a.x, a.y), ang(b.x, b.y), ang(p[0], p[1]));
    let tau = std::f64::consts::TAU;
    let norm = |t: f64| ((t % tau) + tau) % tau;
    if ccw {
        let span = if a == b { tau } else { norm(t1 - t0) };
        norm(tp - t0) <= span + 1e-12
    } else {
        let span = if a == b { tau } else { norm(t0 - t1) };
        norm(t0 - tp) <= span + 1e-12
    }
}

/// Roughing along −z (a G71-like cycle, expanded): passes at depth ap, each from ahead of the
/// stock down to the grown part H ⊕ B_{r_ε + leave} or to z_lo, retracting at 45°.
pub fn rough(
    s: &Setup,
    tool: &TurnTool,
    t: u32,
    leave: f64,
    ap: f64,
    z_lo: f64,
    z_top: f64,
) -> Result<OpPath, OpError> {
    if !(leave >= 0.0) {
        return Err(OpError::Geometry(format!(
            "a leave of {leave}: below the part (G1 holds the cutter from P ⊕ B_leave, leave ≥ 0)"
        )));
    }
    if tool.kind != Kind::Turn {
        return Err(OpError::Tool(format!(
            "roughing needs a turning tool, not `{}`",
            tool.id
        )));
    }
    let re = tool.re;
    let c = grown(s, re + leave)?;
    let z_start = z_top + APPROACH + re;
    let x_out = s.bar_r + APPROACH + re;
    let start = [x_out, z_start];
    let mut steps = vec![Step::Rapid(start)];
    // the pass radii: every ap from the bar, and every radius where the grown part has a
    // cylinder (so a step's band is not left between two levels)
    let mut xs: Vec<f64> = Vec::new();
    let mut k = 1;
    loop {
        let x = s.bar_r - ap * k as f64 + re;
        if x - re <= 0.0 {
            break;
        }
        xs.push(x);
        k += 1;
    }
    for sg in &c.segs {
        if let Seg::Line { a, b } = *sg
            && a.x == b.x
            && a.x - re > 0.0
            && a.x - re < s.bar_r - 1e-6
        {
            // just off the grown part's cylinder, so the pass runs down beside it rather than
            // stopping where the line first touches it
            xs.push(a.x + 1e-7 * (1.0 + a.x.abs()));
        }
    }
    xs.sort_by(|a, b| b.total_cmp(a));
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    for x in xs {
        let mut zs = Vec::new();
        for sg in &c.segs {
            meet_vertical(sg, x, &mut zs);
        }
        let hit = zs
            .iter()
            .copied()
            .filter(|z| *z < z_start)
            .fold(f64::NEG_INFINITY, f64::max);
        let z_end = hit.max(z_lo);
        if z_end >= z_top {
            // the pass would cut nothing below the stock's top
            continue;
        }
        steps.push(Step::Rapid([x, z_start]));
        steps.push(Step::Feed([x, z_end]));
        steps.push(Step::Feed([x + RETRACT, z_end + RETRACT]));
        steps.push(Step::Rapid([x + RETRACT, z_start]));
    }
    steps.push(Step::Rapid(start));
    Ok(OpPath {
        name: format!("rough OD, leave {leave}"),
        tool: tool.clone(),
        t,
        spindle: css(tool),
        f: tool.f,
        leave,
        start,
        steps,
        removal: None,
    })
}

/// Boring, rough: a hole of radius r0 (drilled) opened outward in axial passes at every ap and
/// at every bore diameter of the part, each down to the part grown by the nose radius and leave
/// (from inside), or to z_lo, retracting at 45° toward the axis.
pub fn bore_rough(
    s: &Setup,
    tool: &TurnTool,
    t: u32,
    leave: f64,
    ap: f64,
    z_lo: f64,
    z_top: f64,
    r0: f64,
) -> Result<OpPath, OpError> {
    if !(leave >= 0.0) {
        return Err(OpError::Geometry(format!(
            "a leave of {leave}: below the part (G1 holds the cutter from P ⊕ B_leave, leave ≥ 0)"
        )));
    }
    if tool.kind != Kind::Bore {
        return Err(OpError::Tool(format!(
            "boring needs a boring bar, not `{}`",
            tool.id
        )));
    }
    let re = tool.re;
    let c = grown(s, re + leave)?;
    let z_start = z_top + APPROACH + re;
    let x_in = (r0 - APPROACH).max(0.0);
    let start = [x_in, z_start];
    let mut steps = vec![Step::Rapid(start)];
    let mut xs: Vec<f64> = Vec::new();
    let mut k = 1;
    loop {
        let x = r0 + ap * k as f64 - re;
        if x + re >= s.part.bbox[2] {
            break;
        }
        xs.push(x);
        k += 1;
    }
    for sg in &c.segs {
        if let Seg::Line { a, b } = *sg
            && a.x == b.x
            && a.x + re > r0 + 1e-6
            && a.x + re < s.part.bbox[2]
        {
            xs.push(a.x - 1e-7 * (1.0 + a.x.abs()));
        }
    }
    xs.sort_by(|a, b| a.total_cmp(b));
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    for x in xs {
        let mut zs = Vec::new();
        for sg in &c.segs {
            meet_vertical(sg, x, &mut zs);
        }
        let hit = zs
            .iter()
            .copied()
            .filter(|z| *z < z_start)
            .fold(f64::NEG_INFINITY, f64::max);
        let z_end = hit.max(z_lo);
        if z_end >= z_top {
            continue;
        }
        steps.push(Step::Rapid([x, z_start]));
        steps.push(Step::Feed([x, z_end]));
        steps.push(Step::Feed([x - RETRACT, z_end + RETRACT]));
        steps.push(Step::Rapid([x - RETRACT, z_start]));
    }
    steps.push(Step::Rapid(start));
    Ok(OpPath {
        name: format!("rough bore, leave {leave}"),
        tool: tool.clone(),
        t,
        spindle: css(tool),
        f: tool.f,
        leave,
        start,
        steps,
        removal: None,
    })
}

/// Splits a piece at parameter points where it crosses z = z0, keeping the part above (or the
/// whole piece if it does not cross). Returns the kept piece and whether it ended at the line.
fn clip_above(s: &Seg, z0: f64) -> Option<(Seg, bool)> {
    let (a, b) = (s.start(), s.end());
    if a.y >= z0 && b.y >= z0 {
        // an arc may still dip below between its ends: refuse to clip there (rare: the path then
        // runs the whole arc, and the checker decides)
        return Some((*s, false));
    }
    if a.y < z0 {
        return None;
    }
    match *s {
        Seg::Line { a, b } => {
            let t = (a.y - z0) / (a.y - b.y);
            let p = forge_cam_geom::region::p2(a.x + t * (b.x - a.x), z0);
            if p == a {
                return None;
            }
            Some((Seg::Line { a, b: p }, true))
        }
        Seg::Arc { a, c, ccw, .. } => {
            let r = ((a.x - c.x).powi(2) + (a.y - c.y).powi(2)).sqrt();
            let dz = z0 - c.y;
            if dz.abs() > r {
                return Some((*s, false));
            }
            let w = (r * r - dz * dz).sqrt();
            for x in [c.x + w, c.x - w] {
                if on_arc(a, s.end(), c, ccw, [x, z0]) {
                    if forge_cam_geom::region::p2(x, z0) == a || (x - a.x).abs() + (z0 - a.y).abs() < 1e-9 {
                        return None;
                    }
                    return Some((
                        Seg::Arc {
                            a,
                            b: forge_cam_geom::region::p2(x, z0),
                            c,
                            ccw,
                        },
                        true,
                    ));
                }
            }
            Some((*s, false))
        }
    }
}

/// Finishing: the nose centre along the boundary of H ⊕ B_{r_ε + leave}, clockwise from where
/// it crosses ρ = ρ_min of the part (over the front face) down to z = z_lo.
pub fn finish(s: &Setup, tool: &TurnTool, t: u32, leave: f64, z_lo: f64) -> Result<OpPath, OpError> {
    if !(leave >= 0.0) {
        return Err(OpError::Geometry(format!(
            "a leave of {leave}: below the part (G1 holds the cutter from P ⊕ B_leave, leave ≥ 0)"
        )));
    }
    if !matches!(tool.kind, Kind::Turn | Kind::Bore) {
        return Err(OpError::Tool(format!(
            "finishing needs a turning tool or a boring bar, not `{}`",
            tool.id
        )));
    }
    let inside = tool.kind == Kind::Bore;
    let re = tool.re;
    // outside: clockwise round the grown part from over the front face at its least radius;
    // inside (a bore): counter-clockwise from over the front face, just outside the bore's mouth
    let c = if inside {
        grown(s, re + leave)?
    } else {
        grown(s, re + leave)?.reversed()
    };
    let x0 = if inside {
        let (lo, hi) = (s.part.bbox[0], s.part.bbox[2]);
        lo + (0.5 * (hi - lo)).min(1.0)
    } else {
        s.part.bbox[0].max(0.0)
    };
    let n = c.segs.len();
    let mut start_at = None;
    for (i, sg) in c.segs.iter().enumerate() {
        let (a, b) = (sg.start(), sg.end());
        let crosses = if inside {
            a.x >= x0 && b.x < x0
        } else {
            a.x <= x0 && b.x > x0
        };
        if crosses && a.y > s.part.bbox[3] {
            start_at = Some(i);
            break;
        }
    }
    let i0 = start_at.ok_or(OpError::Geometry(
        "the finish path finds no start over the front face".into(),
    ))?;
    let mut segs = Vec::new();
    // the first piece from x0 on
    let first = c.segs[i0];
    let p0 = match first {
        Seg::Line { a, b } => {
            let t = (x0 - a.x) / (b.x - a.x);
            forge_cam_geom::region::p2(x0, a.y + t * (b.y - a.y))
        }
        Seg::Arc { .. } => first.start(),
    };
    segs.push(match first {
        Seg::Line { b, .. } => Seg::Line { a: p0, b },
        x => x,
    });
    for k in 1..n {
        let sg = c.segs[(i0 + k) % n];
        match clip_above(&sg, z_lo) {
            None => break,
            Some((p, ended)) => {
                segs.push(p);
                if ended {
                    break;
                }
            }
        }
    }
    let p0 = [p0.x, p0.y];
    let start = [p0[0], p0[1] + APPROACH];
    let mut steps = vec![Step::Rapid(start), Step::Feed(p0)];
    for sg in &segs {
        let a = sg.start();
        match *sg {
            Seg::Line { b, .. } => {
                if a == b {
                    continue;
                }
                steps.push(Step::Feed([b.x, b.y]));
            }
            Seg::Arc { b, c, ccw, .. } => {
                if a == b {
                    continue;
                }
                steps.push(Step::Arc {
                    to: [b.x, b.y],
                    c: [c.x, c.y],
                    ccw,
                });
            }
        }
    }
    let last = steps.last().unwrap().to();
    if inside {
        // toward the axis, then straight up out of the bore
        steps.push(Step::Feed([last[0] - RETRACT, last[1] + RETRACT]));
        steps.push(Step::Rapid([last[0] - RETRACT, start[1]]));
    } else {
        steps.push(Step::Feed([last[0] + RETRACT, last[1] + RETRACT]));
        let out = [s.bar_r + APPROACH + re, last[1] + RETRACT];
        steps.push(Step::Rapid(out));
        steps.push(Step::Rapid([out[0], start[1]]));
    }
    Ok(OpPath {
        name: format!("finish {}, leave {leave}", if inside { "bore" } else { "OD" }),
        tool: tool.clone(),
        t,
        spindle: css(tool),
        f: tool.f,
        leave,
        start,
        steps,
        removal: None,
    })
}

/// An OD groove between z0 and z1 down to radius rho: plunges no wider apart than 0.8 of the
/// blade, the first and last at the walls.
pub fn groove(s: &Setup, tool: &TurnTool, t: u32, z0: f64, z1: f64, rho: f64) -> Result<OpPath, OpError> {
    if !matches!(tool.kind, Kind::Groove | Kind::Part) {
        return Err(OpError::Tool(format!(
            "grooving needs a blade, not `{}`",
            tool.id
        )));
    }
    let (w, rc) = (tool.width, tool.re);
    if z1 - z0 < w {
        return Err(OpError::Geometry(format!(
            "the groove ({}) is narrower than the blade ({w})",
            z1 - z0
        )));
    }
    let x_out = s.bar_r + APPROACH + rc;
    let (zr0, zr1) = (z0 + rc, z1 - (w - rc));
    let n = ((zr1 - zr0) / (0.8 * w)).ceil() as usize;
    let mut zs: Vec<f64> = (0..=n)
        .map(|i| {
            if n == 0 {
                zr0
            } else {
                zr0 + (zr1 - zr0) * i as f64 / n as f64
            }
        })
        .collect();
    zs.dedup();
    let start = [x_out, zr0];
    let mut steps = vec![Step::Rapid(start)];
    for z in zs {
        steps.push(Step::Rapid([x_out, z]));
        steps.push(Step::Feed([rho + rc, z]));
        // out of its own groove at feed: the blade's sides have no room for a rapid's clearance
        steps.push(Step::Feed([x_out, z]));
    }
    Ok(OpPath {
        name: format!("groove z {z0}..{z1} to radius {rho}"),
        tool: tool.clone(),
        t,
        spindle: css(tool),
        f: tool.f,
        leave: 0.0,
        start,
        steps,
        removal: None,
    })
}

/// Part-off: the blade's +z side on the part's back face (z = z_back), plunged past the centre.
pub fn part_off(s: &Setup, tool: &TurnTool, t: u32, z_back: f64) -> Result<OpPath, OpError> {
    if tool.kind != Kind::Part {
        return Err(OpError::Tool(format!(
            "parting needs a part-off blade, not `{}`",
            tool.id
        )));
    }
    let (w, rc) = (tool.width, tool.re);
    if tool.l < s.bar_r + rc {
        return Err(OpError::Tool(format!(
            "the blade's reach ({}) is short of the bar's radius ({})",
            tool.l, s.bar_r
        )));
    }
    let z = z_back - (w - rc);
    let x_out = s.bar_r + APPROACH + rc;
    let start = [x_out, z];
    let steps = vec![
        Step::Rapid(start),
        Step::Feed([-rc - 0.2, z]),
        Step::Feed([x_out, z]),
    ];
    Ok(OpPath {
        name: format!("part off at z = {z_back}"),
        tool: tool.clone(),
        t,
        spindle: Spindle::Css {
            vc: tool.vc,
            max_rpm: 2000.0,
        },
        f: tool.f,
        leave: 0.0,
        start,
        steps,
        removal: None,
    })
}

/// An external ISO metric thread (60°): major diameter d, pitch p, from z_start (ahead of the
/// part) to z_end, in `passes` radial infeeds of equal chip area (depth h·√(k/n), h = 0.6134 p).
/// The part as modelled shows the major diameter: the thread is a declared removal Δ, the band
/// [d/2 − h, d/2] over the thread's length and the insert's half width past its end, and G1 holds
/// the tool from P ∖ Δ (research/cam/design.md §7b).
pub fn thread(
    s: &Setup,
    tool: &TurnTool,
    t: u32,
    d: f64,
    p: f64,
    z_start: f64,
    z_end: f64,
    passes: usize,
) -> Result<OpPath, OpError> {
    if tool.kind != Kind::Thread {
        return Err(OpError::Tool(format!(
            "threading needs a thread insert, not `{}`",
            tool.id
        )));
    }
    if !(p >= tool.pitch[0] && p <= tool.pitch[1]) {
        return Err(OpError::Tool(format!(
            "pitch {p} is outside the insert's range {:?}",
            tool.pitch
        )));
    }
    let h = 0.6134 * p;
    let rt = tool.re;
    let rmaj = 0.5 * d;
    let x_out = s.bar_r + APPROACH + rt;
    let n = passes.max(1);
    let start = [x_out, z_start];
    let mut steps = vec![Step::Rapid(start)];
    for k in 1..=n {
        let depth = h * (k as f64 / n as f64).sqrt();
        let x = rmaj - depth + rt;
        steps.push(Step::Rapid([x, z_start]));
        steps.push(Step::Thread {
            to: [x, z_end],
            pitch: p,
        });
        // the pull-out at the thread's end cuts (it leaves through the groove): a feed
        steps.push(Step::Feed([x_out, z_end]));
        steps.push(Step::Rapid([x_out, z_start]));
    }
    // the band: the insert's flanks at ±30° from +ρ about the tip centre reach half width
    // (h + margin)·tan 30° + rt past the reference's end
    let half = (h + 0.2) * tan(30f64.to_radians()) + rt;
    let band = Region {
        loops: vec![rect(rmaj - h, z_end - half, s.bar_r + 1.0, s.part.bbox[3] + 1.0)],
    };
    Ok(OpPath {
        name: format!("thread M{d} x {p}"),
        tool: tool.clone(),
        t,
        spindle: Spindle::Rpm((1000.0 * tool.vc / (std::f64::consts::PI * d)).floor()),
        f: p,
        leave: 0.0,
        start,
        steps,
        removal: Some(band),
    })
}

/// An internal groove between z0 and z1 out to radius rho, in a bore of radius r_in on the way in
/// (the smallest radius between the bar's front and the groove): the blade enters along the
/// axis at r_in less the approach, then plunges +ρ no wider apart than 0.8 of the blade, the
/// first and last at the walls, each out of its own groove at feed.
pub fn id_groove(
    s: &Setup,
    tool: &TurnTool,
    t: u32,
    z0: f64,
    z1: f64,
    rho: f64,
    r_in: f64,
) -> Result<OpPath, OpError> {
    if tool.kind != Kind::IdGroove {
        return Err(OpError::Tool(format!(
            "internal grooving needs an ID grooving bar, not `{}`",
            tool.id
        )));
    }
    let (w, rc) = (tool.width, tool.re);
    if z1 - z0 < w {
        return Err(OpError::Geometry(format!(
            "the groove ({}) is narrower than the blade ({w})",
            z1 - z0
        )));
    }
    if !(rho > r_in) {
        return Err(OpError::Geometry(format!(
            "an internal groove to radius {rho} in a bore of radius {r_in}: it must lie outside the bore"
        )));
    }
    let x_in = r_in - APPROACH - rc;
    let z_entry = s.bar_front + APPROACH + rc;
    let (zr0, zr1) = (z0 + rc, z1 - (w - rc));
    let n = ((zr1 - zr0) / (0.8 * w)).ceil() as usize;
    let mut zs: Vec<f64> = (0..=n)
        .map(|i| {
            if n == 0 {
                zr0
            } else {
                zr0 + (zr1 - zr0) * i as f64 / n as f64
            }
        })
        .collect();
    zs.dedup();
    let start = [x_in, z_entry];
    let mut steps = vec![Step::Rapid(start)];
    for z in zs {
        steps.push(Step::Rapid([x_in, z]));
        steps.push(Step::Feed([rho - rc, z]));
        steps.push(Step::Feed([x_in, z]));
    }
    steps.push(Step::Rapid(start));
    Ok(OpPath {
        name: format!("ID groove z {z0}..{z1} to radius {rho}"),
        tool: tool.clone(),
        t,
        spindle: css(tool),
        f: tool.f,
        leave: 0.0,
        start,
        steps,
        removal: None,
    })
}

/// An internal ISO metric thread (60°): nominal (major) diameter d, pitch p, from z_start (ahead
/// of the part) to z_end, in `passes` radial infeeds of equal chip area out from the minor radius
/// r1 = d/2 − 5H/8 (H = p·√3/2) to d/2. The part as modelled shows the minor diameter: the thread
/// is a declared removal Δ, the band [r1, d/2] over the thread's length and the insert's half
/// width past its end, and G1 holds the tool from P ∖ Δ (as for the external thread).
pub fn id_thread(
    s: &Setup,
    tool: &TurnTool,
    t: u32,
    d: f64,
    p: f64,
    z_start: f64,
    z_end: f64,
    passes: usize,
) -> Result<OpPath, OpError> {
    if tool.kind != Kind::IdThread {
        return Err(OpError::Tool(format!(
            "internal threading needs an ID thread insert, not `{}`",
            tool.id
        )));
    }
    if !(p >= tool.pitch[0] && p <= tool.pitch[1]) {
        return Err(OpError::Tool(format!(
            "pitch {p} is outside the insert's range {:?}",
            tool.pitch
        )));
    }
    let h = 0.625 * p * 0.75f64.sqrt();
    let rt = tool.re;
    let rmaj = 0.5 * d;
    let r1 = rmaj - h;
    let x_in = r1 - APPROACH - rt;
    let n = passes.max(1);
    let start = [x_in, z_start];
    let mut steps = vec![Step::Rapid(start)];
    for k in 1..=n {
        let depth = h * (k as f64 / n as f64).sqrt();
        let x = r1 + depth - rt;
        steps.push(Step::Rapid([x, z_start]));
        steps.push(Step::Thread {
            to: [x, z_end],
            pitch: p,
        });
        // the pull-out at the thread's end cuts: a feed
        steps.push(Step::Feed([x_in, z_end]));
        steps.push(Step::Rapid([x_in, z_start]));
    }
    let half = (h + 0.2) * tan(30f64.to_radians()) + rt;
    let band = Region {
        loops: vec![rect(r1, z_end - half, rmaj, s.part.bbox[3] + 1.0)],
    };
    Ok(OpPath {
        name: format!("ID thread M{d} x {p}"),
        tool: tool.clone(),
        t,
        spindle: Spindle::Rpm((1000.0 * tool.vc / (std::f64::consts::PI * d)).floor()),
        f: p,
        leave: 0.0,
        start,
        steps,
        removal: Some(band),
    })
}

/// A drill on the centre line to depth z_end (its tip), pecking by `peck` (0: one feed).
pub fn drill(s: &Setup, tool: &TurnTool, t: u32, z_end: f64, peck: f64) -> Result<OpPath, OpError> {
    if tool.kind != Kind::Drill {
        return Err(OpError::Tool(format!(
            "drilling needs a drill, not `{}`",
            tool.id
        )));
    }
    let z0 = s.part.bbox[3] + APPROACH;
    let start = [0.0, z0];
    let mut steps = vec![Step::Rapid(start)];
    let mut z = z0;
    let step = if peck > 0.0 { peck } else { z0 - z_end };
    while z > z_end {
        let zn = (z - step).max(z_end);
        steps.push(Step::Feed([0.0, zn]));
        if zn > z_end {
            // out of the hole and back in at feed: the drill's margins ride its own wall, where
            // a rapid's clearance cannot be proven
            steps.push(Step::Feed([0.0, z0]));
            steps.push(Step::Feed([0.0, zn + 0.5]));
        }
        z = zn;
    }
    steps.push(Step::Feed(start));
    Ok(OpPath {
        name: format!("drill to z = {z_end}"),
        tool: tool.clone(),
        t,
        spindle: Spindle::Rpm((1000.0 * tool.vc / (std::f64::consts::PI * tool.width)).floor()),
        f: tool.f,
        leave: 0.0,
        start,
        steps,
        removal: None,
    })
}

/// One move that failed its check.
#[derive(Clone, Debug, PartialEq)]
pub struct Refusal {
    pub op: usize,
    pub step: usize,
    pub body: String,
    /// part (G1), chuck (G2), stock (G3) or travel (G5).
    pub rule: String,
    pub verdict: String,
    pub at: [f64; 2],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    pub checks: usize,
    pub least: f64,
    pub refusals: Vec<Refusal>,
    /// The leftover (G4): regions of the final stock outside the part grown by tol.
    pub leftover: Vec<Region>,
    /// Why each leftover region is there: "nose radius r" (it lies within the finishing tool's
    /// nose radius of the part: a sharp inside corner no nose of that radius reaches) or "uncut"
    /// (no op reached it).
    pub leftover_reasons: Vec<String>,
    /// Declared removals (threads), by op.
    pub removals: Vec<(usize, Region)>,
    /// Stock removals left out because their boolean would not round to a valid region (the
    /// stock model is then larger than it could be: sound, and the G4 leftover may list more).
    pub stock_skipped: usize,
    /// "exact" (booleans: the oracle, and G4) or "columns" (the normal path: stair.rs).
    pub stock_model: String,
    /// Whether the G4 leftover was computed (the exact model only).
    pub leftover_done: bool,
    /// "full" (every joint position checked), "except G5" (a machine but no placement: the
    /// spans only), or "no machine".
    pub g5: String,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.refusals.is_empty()
    }
}

/// One checked move: the move, whether it cuts, whether it is a rapid, and the step it is.
pub type PMove = (Move, bool, bool, usize);

pub fn steps_to_moves(op: &OpPath) -> Vec<PMove> {
    let mut v = Vec::new();
    let mut at = op.start;
    for (i, st) in op.steps.iter().enumerate() {
        let to = st.to();
        let (mv, cut, rapid) = match *st {
            Step::Rapid(p) => (Move::Line { a: at, b: p }, false, true),
            Step::Feed(p) => (Move::Line { a: at, b: p }, true, false),
            Step::Thread { to, .. } => (Move::Line { a: at, b: to }, true, false),
            Step::Arc { to, c, ccw } => (Move::Arc { a: at, b: to, c, ccw }, true, false),
        };
        if at != to {
            v.push((mv, cut, rapid, i));
        }
        at = to;
    }
    v
}

/// The grid cleared polygons are snapped to (2^-24 mm): coarse beside the booleans' 2^-32, so
/// their meets are not near-ties.
const SNAP: f64 = 1.0 / 16777216.0;

/// A convex polygon pulled in toward its centroid by `pull` SNAP (≥ 1), then each vertex rounded to SNAP:
/// every rounded vertex moves at most SNAP/√2 per axis, so the result stays inside the original.
fn snap_inside(q: &[[f64; 2]], pull: f64) -> Vec<[f64; 2]> {
    let n = q.len() as f64;
    let m = [
        q.iter().map(|p| p[0]).sum::<f64>() / n,
        q.iter().map(|p| p[1]).sum::<f64>() / n,
    ];
    let mut out: Vec<[f64; 2]> = Vec::new();
    for p in q {
        let v = [m[0] - p[0], m[1] - p[1]];
        let l = (v[0] * v[0] + v[1] * v[1]).sqrt();
        if !(l > 4.0 * pull * SNAP) {
            return Vec::new();
        }
        let k = pull * SNAP / l;
        let r = [
            ((p[0] + v[0] * k) / SNAP).round() * SNAP,
            ((p[1] + v[1] * k) / SNAP).round() * SNAP,
        ];
        if out.last() != Some(&r) {
            out.push(r);
        }
    }
    while out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    // a sliver (no area at this grid) is left out
    let mut a2 = 0.0;
    for i in 0..out.len() {
        let (u, v) = (out[i], out[(i + 1) % out.len()]);
        a2 += u[0] * v[1] - v[0] * u[1];
    }
    if !(a2 > 64.0 * SNAP * SNAP) {
        return Vec::new();
    }
    out
}

fn region_diff(a: &Region, b: &Region) -> Result<Region, String> {
    if b.loops.is_empty() {
        return Ok(a.clone());
    }
    boolean::boolean(a, b, Op::Diff)
        .map(|l| boolean::region_of(&l))
        .map_err(|e| format!("{e:?}"))
}

fn region_union(a: &Region, b: &Region) -> Result<Region, String> {
    if a.loops.is_empty() {
        return Ok(b.clone());
    }
    boolean::boolean(a, b, Op::Union)
        .map(|l| boolean::region_of(&l))
        .map_err(|e| format!("{e:?}"))
}

/// Points of a body's outline at depth 0, K ⊕ B_r', placed at p: each core vertex, and for
/// r' > 0 the points r' from it along its normal cone (three directions across it). Their convex
/// hull lies inside the outline (it is convex). r' is r less the margin `less`, and less a
/// relative 1e-6 (far more than the f64 rounding of the sums), so the points are strictly
/// inside.
fn outline_points(b: &Body, p: [f64; 2], less: f64, dense: bool, out: &mut Vec<[f64; 2]>) {
    let k = &b.core;
    let n = k.len();
    let rr = (b.r - less).max(0.0) * (1.0 - 1e-6);
    let c = [
        k.iter().map(|v| v[0]).sum::<f64>() / n as f64,
        k.iter().map(|v| v[1]).sum::<f64>() / n as f64,
    ];
    let xmin = k.iter().map(|v| v[0]).fold(f64::INFINITY, f64::min);
    for i in 0..n {
        // the vertex, pulled a relative 1e-9 toward the centroid (inside under rounding)
        let v = [
            k[i][0] + (c[0] - k[i][0]) * 1e-9,
            k[i][1] + (c[1] - k[i][1]) * 1e-9,
        ];
        out.push([p[0] + v[0], p[1] + v[1]]);
        if rr > 0.0 && n >= 2 {
            let (u, w) = (k[(i + n - 1) % n], k[(i + 1) % n]);
            // outward normals of the edges into and out of the vertex (counter-clockwise core)
            let e1 = [k[i][0] - u[0], k[i][1] - u[1]];
            let e2 = [w[0] - k[i][0], w[1] - k[i][1]];
            let t1 = dmath::atan2(e1[1], e1[0]) - std::f64::consts::FRAC_PI_2;
            let mut t2 = dmath::atan2(e2[1], e2[0]) - std::f64::consts::FRAC_PI_2;
            while t2 < t1 {
                t2 += std::f64::consts::TAU;
            }
            // for a finishing cut (leave 0), the cutting corners (the core's least x, and the
            // reference vertex: a boring nose) densely
            // enough that the polygon falls short of the disc by under SAMPLE_GAP, so the stock
            // model leaves no sliver on the finished surface; elsewhere three directions
            let m = if dense && (k[i][0] == xmin || k[i] == [0.0, 0.0]) {
                let step = 2.0 * dmath::acos_clamped(1.0 - SAMPLE_GAP / rr.max(SAMPLE_GAP));
                (((t2 - t1) / step).ceil() as usize).clamp(2, 64)
            } else {
                2
            };
            for j in 0..=m {
                let t = t1 + (t2 - t1) * j as f64 / m as f64;
                out.push([p[0] + v[0] + rr * dmath::cos(t), p[1] + v[1] + rr * dmath::sin(t)]);
            }
        }
    }
}

/// How far the cleared polygon may fall short of a cutting corner's disc (mm): under the usual
/// tolerance, so the stock model leaves no sliver the G4 leftover would list.
const SAMPLE_GAP: f64 = 5e-4;

/// The convex hull (counter-clockwise) of points.
fn hull2(mut p: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let cr =
        |o: [f64; 2], a: [f64; 2], b: [f64; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut lo: Vec<[f64; 2]> = Vec::new();
    for &q in &p {
        while lo.len() >= 2 && cr(lo[lo.len() - 2], lo[lo.len() - 1], q) <= 0.0 {
            lo.pop();
        }
        lo.push(q);
    }
    let mut hi: Vec<[f64; 2]> = Vec::new();
    for &q in p.iter().rev() {
        while hi.len() >= 2 && cr(hi[hi.len() - 2], hi[hi.len() - 1], q) <= 0.0 {
            hi.pop();
        }
        hi.push(q);
    }
    lo.pop();
    hi.pop();
    lo.extend(hi);
    lo
}

/// A convex polygon inside what the body's outline sweeps along the chord a → b, when the path
/// it stands for lies within `sag` of the chord (0 for a straight move): the hull of the outline
/// points shrunk by sag at both ends. The hull of two translates of a convex set K' is K' ⊕
/// [a, b]; each chord point lies within sag of a path point, and K' = K ⊕ B_{r − sag}.
fn swept_inner(b: &Body, a: [f64; 2], e: [f64; 2], sag: f64, dense: bool) -> Vec<[f64; 2]> {
    let mut pts = Vec::new();
    outline_points(b, a, sag, dense, &mut pts);
    outline_points(b, e, sag, dense, &mut pts);
    hull2(pts)
}

/// The longest piece a cut is checked in, for the bodies that trail the cutter through what it
/// has just cleared.
pub const SUB: f64 = 2.0;

/// A move as pieces of at most SUB (arcs as chords of at most 10° too), each with the sagitta
/// bound of its chord from the true path (0 for a line).
fn pieces(mv: &Move) -> Vec<(Move, [f64; 2], [f64; 2], f64)> {
    match *mv {
        Move::Line { a, b } => {
            let l = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
            let n = (l / SUB).ceil().max(1.0) as usize;
            let mut v = Vec::new();
            let mut prev = a;
            for i in 1..=n {
                let p = if i == n {
                    b
                } else {
                    let t = i as f64 / n as f64;
                    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
                };
                v.push((Move::Line { a: prev, b: p }, prev, p, 0.0));
                prev = p;
            }
            v
        }
        Move::Arc { a, b, c, ccw } => {
            let r = ((a[0] - c[0]).powi(2) + (a[1] - c[1]).powi(2)).sqrt();
            let rb = ((b[0] - c[0]).powi(2) + (b[1] - c[1]).powi(2)).sqrt();
            let ang = |p: [f64; 2]| dmath::atan2(p[1] - c[1], p[0] - c[0]);
            let (t0, mut t1) = (ang(a), ang(b));
            if ccw {
                while t1 <= t0 {
                    t1 += std::f64::consts::TAU;
                }
            } else {
                while t1 >= t0 {
                    t1 -= std::f64::consts::TAU;
                }
            }
            let sweep = (t1 - t0).abs();
            let n = (sweep / 10f64.to_radians())
                .ceil()
                .max((sweep * r / SUB).ceil())
                .max(1.0) as usize;
            let mut v = Vec::new();
            let mut prev = a;
            for i in 1..=n {
                let t = t0 + (t1 - t0) * i as f64 / n as f64;
                let p = if i == n {
                    b
                } else {
                    [c[0] + r * dmath::cos(t), c[1] + r * dmath::sin(t)]
                };
                let half = 0.5 * ((p[0] - prev[0]).powi(2) + (p[1] - prev[1]).powi(2)).sqrt();
                let rm = r.max(rb);
                // the sagitta, the radius mismatch at the ends, and a relative margin for the
                // f64 points on the circle
                let sag = rm - (rm * rm - half * half).max(0.0).sqrt()
                    + (r - rb).abs()
                    + 1e-12 * (1.0 + rm + c[0].abs() + c[1].abs());
                let sub = Move::Arc {
                    a: prev,
                    b: p,
                    c,
                    ccw,
                };
                v.push((sub, prev, p, sag * 1.01));
                prev = p;
            }
            v
        }
    }
}

/// Removes a convex polygon from the stock: the polygon snapped inside on a coarse grid, retried
/// smaller if the exact boolean would pinch; left out (counted) if it never makes a valid region.
fn remove(stock: &mut Region, obs: &mut Obstacle, q: &[[f64; 2]], rep: &mut Report) {
    let mut last = String::new();
    for pull in [4.0, 256.0, 16384.0] {
        let q = snap_inside(q, pull);
        if q.len() < 3 {
            return;
        }
        // the obstacle validates the region exactly; an invalid result keeps the old stock
        let next = region_diff(
            stock,
            &Region {
                loops: vec![poly(&q)],
            },
        )
        .and_then(|r| {
            Obstacle::of("stock", &r, 0.0)
                .map(|o| (r, o))
                .map_err(|e| format!("{e:?}"))
        });
        match next {
            Ok((r, o)) => {
                if r != *stock {
                    *stock = r;
                    *obs = o;
                }
                return;
            }
            Err(e) => last = e,
        }
    }
    if std::env::var("FORGE_TURN_TRACE").is_ok() {
        eprintln!("stock removal skipped: {last}");
    }
    rep.stock_skipped += 1;
}

/// The box of a move's path: a line's ends; an arc's circle box (a superset).
fn move_box(mv: &Move) -> ([f64; 2], [f64; 2]) {
    match *mv {
        Move::Line { a, b } => ([a[0].min(b[0]), a[1].min(b[1])], [a[0].max(b[0]), a[1].max(b[1])]),
        Move::Arc { a, c, .. } => {
            let r = Ival::pt(a[0])
                .sub(Ival::pt(c[0]))
                .sqr()
                .add(Ival::pt(a[1]).sub(Ival::pt(c[1])).sqr())
                .sqrt()
                .hi;
            (
                [
                    Ival::pt(c[0]).sub(Ival::pt(r)).lo,
                    Ival::pt(c[1]).sub(Ival::pt(r)).lo,
                ],
                [
                    Ival::pt(c[0]).add(Ival::pt(r)).hi,
                    Ival::pt(c[1]).add(Ival::pt(r)).hi,
                ],
            )
        }
    }
}

/// Certifies the ops in order on the setup (see the module doc). `posted`: each op's moves as
/// the posted program states them, read back (post.rs); None certifies the ops' own paths.
pub fn certify(s: &Setup, ops: &[OpPath], posted: Option<&[Vec<PMove>]>) -> Result<Report, String> {
    certify_with(s, ops, posted, true)
}

/// `certify` for the normal path: the in-process stock as columns (stair.rs: a superset by
/// construction, so G3 against it is sound, with no booleans), and no G4 leftover (the exact
/// stock and the leftover are `certify`'s, on demand and as the test oracle).
pub fn certify_fast(s: &Setup, ops: &[OpPath], posted: Option<&[Vec<PMove>]>) -> Result<Report, String> {
    certify_with(s, ops, posted, false)
}

/// The stock as certify keeps it.
enum StockModel {
    Exact(Region),
    Columns(crate::stair::Stair),
}

impl StockModel {
    fn region(&self) -> Region {
        match self {
            StockModel::Exact(r) => r.clone(),
            StockModel::Columns(c) => c.region(),
        }
    }

    fn remove(&mut self, obs: &mut Obstacle, q: &[[f64; 2]], rep: &mut Report) {
        match self {
            StockModel::Exact(r) => remove(r, obs, q, rep),
            StockModel::Columns(c) => {
                if c.clear_poly(q) {
                    match Obstacle::of_stair("stock", c) {
                        Ok(o) => *obs = o,
                        Err(_) => rep.stock_skipped += 1,
                    }
                }
            }
        }
    }
}

fn certify_with(
    s: &Setup,
    ops: &[OpPath],
    posted: Option<&[Vec<PMove>]>,
    exact: bool,
) -> Result<Report, String> {
    s.check().map_err(|e| format!("{e:?}"))?;
    let mut rep = Report {
        least: f64::INFINITY,
        ..Default::default()
    };
    let mut part = Obstacle::of("part", &s.part.region, s.part.mu).map_err(|e| format!("{e:?}"))?;
    let mut chuck = Obstacle::of("chuck", &s.chuck_region(), 0.0).map_err(|e| format!("{e:?}"))?;
    let mut stock_model = if exact {
        StockModel::Exact(s.bar())
    } else {
        StockModel::Columns(crate::stair::Stair::bar(
            s.bar_r,
            s.bar_front,
            s.chuck.z_face - s.chuck.grip,
        ))
    };
    rep.stock_model = if exact { "exact" } else { "columns" }.into();
    for (oi, op) in ops.iter().enumerate() {
        let moves = match posted {
            Some(p) => p[oi].clone(),
            None => steps_to_moves(op),
        };
        // the machine: the station on the turret, the spindle within its range (CSS under the
        // G50 clamp), and the station's offsets when the setup is placed
        let mut span = [f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY];
        let mut offsets: Option<[f64; 2]> = None;
        if let Some(m) = &s.machine {
            let mut refuse = |why: String| {
                rep.refusals.push(Refusal {
                    op: oi,
                    step: 0,
                    body: "machine".into(),
                    rule: "machine".into(),
                    verdict: why,
                    at: op.start,
                })
            };
            if op.t == 0 || op.t > m.stations {
                refuse(format!("station {} on a turret of {}", op.t, m.stations));
            }
            match op.spindle {
                Spindle::Css { max_rpm, .. } if max_rpm > m.rpm[1] => refuse(format!(
                    "a top speed of {max_rpm} rpm over the spindle's {}",
                    m.rpm[1]
                )),
                Spindle::Rpm(r) if r < m.rpm[0] || r > m.rpm[1] => {
                    refuse(format!("{r} rpm outside the spindle's {:?}", m.rpm))
                }
                _ => {}
            }
            if let Some(p) = &s.place {
                offsets = p.tools.iter().find(|(t, _)| *t == op.t).map(|x| x.1);
                if offsets.is_none() {
                    refuse(format!("no offsets for station {} in the placement", op.t));
                }
            }
        }
        let mut stock =
            Obstacle::of("stock", &stock_model.region(), 0.0).map_err(|e| format!("stock: {e:?}"))?;
        let mut target = match &op.removal {
            Some(d) => {
                rep.removals.push((oi, d.clone()));
                let pr = region_diff(&s.part.region, d)?;
                Some(
                    Obstacle::of("part less the declared removal", &pr, s.part.mu)
                        .map_err(|e| format!("{e:?}"))?,
                )
            }
            None => None,
        };
        for (si, (mv, cut, rapid, _step)) in moves.iter().enumerate() {
            // G5: the joints through the move (a line by its ends, an arc by its circle's box)
            if let Some(m) = &s.machine {
                let tip = op.tool.tip;
                let (lo, hi) = move_box(mv);
                let tlo = [
                    Ival::pt(lo[0]).add(Ival::pt(tip[0])),
                    Ival::pt(lo[1]).add(Ival::pt(tip[1])),
                ];
                let thi = [
                    Ival::pt(hi[0]).add(Ival::pt(tip[0])),
                    Ival::pt(hi[1]).add(Ival::pt(tip[1])),
                ];
                span[0] = span[0].min(tlo[0].lo);
                span[1] = span[1].max(thi[0].hi);
                span[2] = span[2].min(tlo[1].lo);
                span[3] = span[3].max(thi[1].hi);
                if let Some(off) = offsets {
                    let jx = [tlo[0].add(Ival::pt(off[0])).lo, thi[0].add(Ival::pt(off[0])).hi];
                    let z0 = s.place.as_ref().map_or(0.0, |p| p.z0);
                    let jz = [
                        tlo[1].add(Ival::pt(z0)).add(Ival::pt(off[1])).lo,
                        thi[1].add(Ival::pt(z0)).add(Ival::pt(off[1])).hi,
                    ];
                    if jx[0] < m.x[0] || jx[1] > m.x[1] || jz[0] < m.z[0] || jz[1] > m.z[1] {
                        rep.refusals.push(Refusal {
                            op: oi,
                            step: si,
                            body: "joints".into(),
                            rule: "travel".into(),
                            verdict: format!(
                                "the joints reach X {jx:?} Z {jz:?}; the travel is X {:?} Z {:?}",
                                m.x, m.z
                            ),
                            at: mv.start(),
                        });
                    }
                }
            }
            let run = |b: &Body, m: &Move, obs: &mut Obstacle, need: f64, rule: &str, rep: &mut Report| {
                rep.checks += 1;
                let v = check::clearance(b, m, obs, need);
                let at = match v {
                    Verdict::Clear { lower } => {
                        rep.least = rep.least.min(lower - need);
                        return;
                    }
                    Verdict::Hit { at, .. } => at,
                    Verdict::Undecided { .. } => m.start(),
                };
                if std::env::var("FORGE_TURN_TRACE").is_ok() {
                    eprintln!(
                        "REFUSED op {oi} step {si} {} vs {}: {v:?} on {m:?}",
                        b.name, obs.name
                    );
                }
                rep.refusals.push(Refusal {
                    op: oi,
                    step: si,
                    body: b.name.clone(),
                    rule: rule.into(),
                    verdict: format!("{v:?}"),
                    at,
                });
            };
            for b in &op.tool.bodies {
                // G1 (the part, or the part less the declared removal)
                let need = if b.cutter {
                    op.leave - s.tol
                } else {
                    s.holder_clearance
                };
                match target.as_mut() {
                    Some(t) => run(b, mv, t, need, "part", &mut rep),
                    None => run(b, mv, &mut part, need, "part", &mut rep),
                }
                // G2
                run(b, mv, &mut chuck, s.fixture_clearance, "chuck", &mut rep);
                // G3 on a rapid: every body off the stock
                if *rapid {
                    run(b, mv, &mut stock, s.rapid_clearance, "stock", &mut rep);
                }
            }
            if !*cut {
                continue;
            }
            // a cut: the non-cutting bodies against the stock as it stood before the move; where
            // that is not proven (a body trailing the cutter through what it has just cut),
            // piece by piece against the stock as the pieces before left it. Then the cut's
            // removal (none for a thread pass: the teeth stay, so the stock of revolution keeps
            // them).
            let clears = !matches!(op.tool.kind, Kind::Thread | Kind::IdThread);
            let cutters: Vec<&Body> = op
                .tool
                .bodies
                .iter()
                .filter(|b| b.cutter && b.depth[0] == 0.0)
                .collect();
            let mut whole = true;
            for b in op.tool.bodies.iter().filter(|b| !b.cutter) {
                rep.checks += 1;
                match check::clearance(b, mv, &mut stock, s.holder_clearance) {
                    Verdict::Clear { lower } => rep.least = rep.least.min(lower - s.holder_clearance),
                    _ => whole = false,
                }
            }
            if whole {
                if clears {
                    for (_, pa, pb, sag) in match mv {
                        Move::Line { a, b } => vec![(*mv, *a, *b, 0.0)],
                        Move::Arc { .. } => pieces(mv),
                    } {
                        for b in &cutters {
                            let q = swept_inner(b, pa, pb, sag, op.leave == 0.0);
                            stock_model.remove(&mut stock, &q, &mut rep);
                        }
                    }
                }
                continue;
            }
            for (pm, pa, pb, sag) in pieces(mv) {
                for b in op.tool.bodies.iter().filter(|b| !b.cutter) {
                    run(b, &pm, &mut stock, s.holder_clearance, "stock", &mut rep);
                }
                if clears {
                    for b in &cutters {
                        let q = swept_inner(b, pa, pb, sag, op.leave == 0.0);
                        stock_model.remove(&mut stock, &q, &mut rep);
                    }
                }
            }
        }
        if let (Some(m), None) = (&s.machine, offsets)
            && span[0] <= span[1]
            && (span[1] - span[0] > m.x[1] - m.x[0] || span[3] - span[2] > m.z[1] - m.z[0])
        {
            rep.refusals.push(Refusal {
                op: oi,
                step: 0,
                body: "joints".into(),
                rule: "travel".into(),
                verdict: format!(
                    "the tip spans X {:?} Z {:?}, wider than the travel",
                    [span[0], span[1]],
                    [span[2], span[3]]
                ),
                at: op.start,
            });
        }
        // a part-off ends the chain: the bar behind it is no longer the part's
        if op.tool.kind == Kind::Part {
            break;
        }
    }
    rep.g5 = match (&s.machine, &s.place) {
        (Some(_), Some(_)) => "full".into(),
        (Some(_), None) => "except G5 (no placement: the spans only)".into(),
        (None, _) => "no machine".into(),
    };
    let StockModel::Exact(stock_region) = stock_model else {
        return Ok(rep);
    };
    rep.leftover_done = true;
    // G4: the final stock less the part grown by tol (and less the declared removals)
    let (grown, _) = offset(&s.part.region, -s.tol).map_err(|e| format!("{e:?}"))?;
    let mut keep = boolean::region_of(&grown);
    for (_, d) in &rep.removals {
        keep = region_union(&keep, d)?;
    }
    // what lies behind the part's back face (the bar left in the chuck after part-off) is not
    // leftover of this part
    let behind = Region {
        loops: vec![rect(
            0.0,
            s.chuck.z_face - s.chuck.grip - 1.0,
            s.bar_r + 1.0,
            s.part.bbox[1],
        )],
    };
    let left = region_diff(&region_diff(&stock_region, &keep)?, &behind)?;
    // the corner radii of the finishing ops (leave 0: turning and boring noses, grooving blades),
    // decided exactly per region
    let mut radii: Vec<f64> = ops
        .iter()
        .filter(|o| {
            matches!(
                o.tool.kind,
                Kind::Turn | Kind::Bore | Kind::Groove | Kind::IdGroove
            ) && o.leave == 0.0
        })
        .map(|o| o.tool.re)
        .filter(|r| *r > 0.0)
        .collect();
    radii.sort_by(|a, b| a.total_cmp(b));
    radii.dedup();
    // the part grown by each finishing corner's radius (plus tol): a region inside the smallest
    // that holds it is a corner no corner of that radius reaches
    let near: Vec<(f64, Region)> = radii
        .iter()
        .filter_map(|&r| {
            offset(&s.part.region, -(r + s.tol))
                .ok()
                .map(|(l, _)| (r, boolean::region_of(&l)))
        })
        .collect();
    for l in left.loops {
        let rg = Region { loops: vec![l] };
        let reason = near
            .iter()
            .find(|(_, n)| region_diff(&rg, n).is_ok_and(|d| d.loops.is_empty()))
            .map(|(r, _)| format!("nose radius {r}: an inside corner the finishing nose cannot reach"))
            .unwrap_or_else(|| "uncut".to_string());
        rep.leftover.push(rg);
        rep.leftover_reasons.push(reason);
    }
    Ok(rep)
}

//! Cutter radius compensation (G41/G42) as two controls state it, applied to the shell's events
//! before the simulation: the programmed path between G41/G42 and G40 replaced by the tool
//! centre's. SINUMERIK 840D from its manual (PGsl 03/2013, chapter 10), below; LinuxCNC from its
//! interpreter's documented source (v2.9.9, rs274ngc/interp_convert.cc, the forge-post dialect's
//! cites), where it differs:
//!
//! - LinuxCNC's entry move ends displaced perpendicular to itself (convert_straight_comp1), and
//!   must be longer than the radius; it then meets the first contour block at a corner, handled
//!   as any other. Its outside corners get a circle about the corner (as G450), its inside ones
//!   the intersection; a turn within TOLERANCE_CONCAVE_CORNER (0.05 rad) gets neither, the next
//!   line starting where the last block ended (an arc so: refused). The move after G40 runs
//!   straight from the compensated end to the programmed point (a Z-only one too), and must not
//!   be an arc. The radius is the D word's tool's (no D, or D0: the loaded tool's).
//!
//! - The equidistant: each line shifted by the radius R along its normal to the side (G41 left of
//!   the direction of travel, G42 right); each arc about the same centre with its radius less or
//!   more R (PGsl 10.1). A negative R is the other side (PGsl 10.1 p. 263).
//! - The approach (NORM, PGsl 10.2 p. 269): the G41/G42 block moves in a straight line from the
//!   uncompensated position to the first contour block's start, displaced perpendicular to its
//!   tangent there. The retract (NORM): from the last contour block's end, displaced
//!   perpendicular to its tangent, in a straight line to the G40 block's uncompensated end.
//! - An outside corner (PGsl 10.3 pp. 274-275): G450 a circle of radius R about the corner from
//!   one equidistant's end to the next's start; G451 both equidistants to their intersection.
//! - An inside corner: both equidistants to their intersection (PGsl 10.1 p. 264, the
//!   intersection on the inside). Where they do not meet within the blocks (a bottleneck), the
//!   control's look-ahead decides (CDON/CDOF, G461/G462, PGsl 10.5, 10.4): refused.
//!
//! Refused (not modelled): a contour block that is not a feed line or a G17 arc, a block with no
//! X/Y motion while it is on, a tool change, offset, home or probe while it is on, a full circle
//! whose neighbours are not tangent, a compensated arc of radius ≤ 0, a G451 corner whose
//! equidistants do not meet.
//!
//! The tool centre's positions are computed in f64 and rounded to the control's units
//! (10^-decimals): each compensated point within half a unit of the exact one in X and Y; an
//! arc's centre is the programmed one (an R word's centre rounded likewise), its end radius
//! within a unit of its start's (the simulation follows that as a spiral).
use crate::gcode::{Block, Centre, CompModel, Ev, FeedMode, Q};
use forge_math::dmath;

type P = [f64; 2];

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}
fn add(a: P, b: P) -> P {
    [a[0] + b[0], a[1] + b[1]]
}
fn mul(a: P, k: f64) -> P {
    [a[0] * k, a[1] * k]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: P, b: P) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn norm(a: P) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: P) -> P {
    mul(a, 1.0 / norm(a))
}
/// The left normal of a direction.
fn left(t: P) -> P {
    [-t[1], t[0]]
}

#[derive(Clone, Debug)]
enum Geo {
    Line,
    /// centre, counter-clockwise, whole turn
    Arc {
        c: P,
        ccw: bool,
        full: bool,
    },
}

/// A contour block: its programmed start and end in the plane, and its event.
#[derive(Clone, Debug)]
struct El {
    a: P,
    b: P,
    geo: Geo,
    line: usize,
    to: Vec<Option<Q>>,
    f: Q,
    mode: FeedMode,
    /// The entry move on a control whose entry is perpendicular to itself (LinuxCNC): it starts
    /// uncompensated, where the tool is; `rapid` if it is a G0.
    entry: bool,
    rapid: bool,
}

impl El {
    fn t_at(&self, p: P) -> P {
        match &self.geo {
            Geo::Line => unit(sub(self.b, self.a)),
            Geo::Arc { c, ccw, .. } => {
                let r = unit(sub(p, *c));
                if *ccw { left(r) } else { mul(left(r), -1.0) }
            }
        }
    }
    fn t0(&self) -> P {
        self.t_at(self.a)
    }
    fn t1(&self) -> P {
        self.t_at(self.b)
    }
    /// The equidistant's point over programmed point p of this block (offset s: + left).
    fn off(&self, p: P, s: f64) -> P {
        add(p, mul(left(self.t_at(p)), s))
    }
    /// The equidistant as an infinite line (point, direction) or a circle (centre, radius).
    fn eq(&self, s: f64) -> Eq {
        match &self.geo {
            Geo::Line => Eq::Line(self.off(self.a, s), unit(sub(self.b, self.a))),
            Geo::Arc { c, ccw, .. } => {
                let r = norm(sub(self.a, *c));
                Eq::Circle(*c, if *ccw { r - s } else { r + s })
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Eq {
    Line(P, P),
    Circle(P, f64),
}

/// The intersections of two equidistants.
fn meet(x: Eq, y: Eq) -> Vec<P> {
    match (x, y) {
        (Eq::Line(p, d), Eq::Line(q, e)) => {
            let den = cross(d, e);
            if den.abs() < 1e-12 {
                return vec![];
            }
            let u = cross(sub(q, p), e) / den;
            vec![add(p, mul(d, u))]
        }
        (Eq::Line(p, d), Eq::Circle(c, r)) | (Eq::Circle(c, r), Eq::Line(p, d)) => {
            // |p + u d - c| = r, d unit
            let w = sub(p, c);
            let b = dot(w, d);
            let disc = b * b - (dot(w, w) - r * r);
            if disc < 0.0 {
                return vec![];
            }
            let h = disc.sqrt();
            vec![add(p, mul(d, -b - h)), add(p, mul(d, -b + h))]
        }
        (Eq::Circle(c0, r0), Eq::Circle(c1, r1)) => {
            let dd = sub(c1, c0);
            let d = norm(dd);
            if d < 1e-12 || d > r0 + r1 || d < (r0 - r1).abs() {
                return vec![];
            }
            let a = (r0 * r0 - r1 * r1 + d * d) / (2.0 * d);
            let h = (r0 * r0 - a * a).max(0.0).sqrt();
            let m = add(c0, mul(dd, a / d));
            let n = mul(left(dd), h / d);
            vec![add(m, n), sub(m, n)]
        }
    }
}

fn nearest(ps: Vec<P>, to: P) -> Option<P> {
    ps.into_iter()
        .min_by(|a, b| norm(sub(*a, to)).total_cmp(&norm(sub(*b, to))))
}

fn wrap(a: f64) -> f64 {
    // to [-π, π)
    let t = dmath::TAU;
    let mut a = a % t;
    if a >= dmath::PI {
        a -= t;
    }
    if a < -dmath::PI {
        a += t;
    }
    a
}

/// A refusal: the program line and what.
pub type Refusal = (usize, String);

/// The events with compensation applied: every Ev::Comp consumed. `radius(t)`: the radius
/// register of tool t's D1 (in wear mode, the wear; with the contour programmed, the radius plus
/// the wear). `ix`: the X and Y letters' indices; `k`: the control's decimals.
pub fn apply(
    blocks: &[Block],
    ix: [usize; 2],
    k: u32,
    start_tool: Option<u32>,
    model: CompModel,
    radius: &dyn Fn(u32) -> Option<f64>,
) -> Result<Vec<Block>, Refusal> {
    if !blocks.iter().any(|b| matches!(b.ev, Ev::Comp { .. })) {
        return Ok(blocks.to_vec());
    }
    let scale = 10f64.powi(k as i32);
    let qf = |q: Q| q as f64 / scale;
    let fq = |v: f64| (v * scale).round() as Q;
    let mut out = Vec::with_capacity(blocks.len());
    let mut tool = start_tool;
    let mut d_on = false;
    // the last position (program coordinates), from the last move's `to`
    let mut pos: Option<Vec<Option<Q>>> = None;
    let xy = |to: &[Option<Q>]| -> Option<P> { Some([qf(to[ix[0]]?), qf(to[ix[1]]?)]) };
    let mut i = 0;
    while i < blocks.len() {
        let b = &blocks[i];
        match &b.ev {
            Ev::Tool { t, .. } => tool = (*t != 0).then_some(*t),
            Ev::Length(h) => d_on = h.is_some(),
            Ev::Rapid(to) | Ev::Feed { to, .. } | Ev::Arc { to, .. } => pos = Some(to.clone()),
            Ev::Comp { left: None, .. } => {
                return Err((b.line, "G40 with compensation off".into()));
            }
            Ev::Comp {
                left: Some(lft),
                arc,
                d,
            } => {
                // SINUMERIK: D1 of the loaded tool (a D in force); LinuxCNC: the D word's tool,
                // or the loaded one (no D, or D0: the spindle's entry)
                let t = match d {
                    Some(t) if *t != 0 => Some(*t),
                    _ => tool,
                }
                .ok_or((b.line, "G41/G42 with no tool loaded".to_string()))?;
                if model.entry_norm && !d_on {
                    return Err((b.line, "G41/G42 with no tool offset (D) in force".into()));
                }
                let r = radius(t).ok_or((
                    b.line,
                    format!("G41/G42: the setup gives no radius register for tool {t} (`radius` on its [[setup.tool]])"),
                ))?;
                let s = if *lft { r } else { -r };
                let start = pos
                    .clone()
                    .and_then(|p| xy(&p).map(|v| (p, v)))
                    .ok_or((b.line, "G41/G42 from an unknown position".to_string()))?;
                let used = section(&blocks[i + 1..], s, *arc, model, start, ix, &qf, &fq, &mut out)?;
                // the position after: the retract's end
                for bb in &blocks[i + 1..i + 1 + used] {
                    if let Ev::Rapid(to) | Ev::Feed { to, .. } | Ev::Arc { to, .. } = &bb.ev {
                        pos = Some(to.clone());
                    }
                }
                i += 1 + used;
                continue;
            }
            _ => {}
        }
        out.push(b.clone());
        i += 1;
    }
    Ok(out)
}

/// One compensated section: `bs` from the approach move on, through G40's retract move. Pushes
/// the tool centre's events; returns how many of `bs` it used.
#[allow(clippy::too_many_arguments)]
fn section(
    bs: &[Block],
    s: f64,
    arc_corners: bool,
    model: CompModel,
    (_start_to, start): (Vec<Option<Q>>, P),
    ix: [usize; 2],
    qf: &dyn Fn(Q) -> f64,
    fq: &dyn Fn(f64) -> Q,
    out: &mut Vec<Block>,
) -> Result<usize, Refusal> {
    let xy = |to: &[Option<Q>], line: usize| -> Result<P, Refusal> {
        match (to[ix[0]], to[ix[1]]) {
            (Some(x), Some(y)) => Ok([qf(x), qf(y)]),
            _ => Err((line, "an unknown X or Y under compensation".into())),
        }
    };
    let mut j = 0;
    let mut passthrough: Vec<(usize, Block)> = Vec::new();
    let mut approach: Option<Block> = None;
    let mut els: Vec<El> = Vec::new();
    let mut retract: Option<Block> = None;
    let mut off = false;
    let mut at = start;
    while j < bs.len() {
        let b = &bs[j];
        j += 1;
        match &b.ev {
            Ev::Comment(_) | Ev::NotModelled { .. } | Ev::Spindle { .. } | Ev::Coolant(_) | Ev::Select(_) => {
                // kept, after the contour block they follow
                passthrough.push((els.len(), b.clone()));
            }
            Ev::Comp { left: None, .. } if approach.is_some() && !off => {
                if els.iter().all(|e| e.entry) {
                    return Err((b.line, "no contour block between G41/G42 and G40".into()));
                }
                off = true;
            }
            Ev::Rapid(to) | Ev::Feed { to, .. } if approach.is_none() => {
                let p = xy(to, b.line)?;
                let len = norm(sub(p, at));
                if len == 0.0 {
                    return Err((b.line, "the G41/G42 block does not move in X or Y".into()));
                }
                if !model.entry_norm {
                    // LinuxCNC: an error unless longer than the radius (convert_straight_comp1)
                    if len <= s.abs() {
                        return Err((
                            b.line,
                            "the entry move under compensation is not longer than the radius".into(),
                        ));
                    }
                    let (f, mode, rapid) = match &b.ev {
                        Ev::Feed { f, mode, .. } => (*f, *mode, false),
                        _ => (0, FeedMode::PerMin, true),
                    };
                    els.push(El {
                        a: at,
                        b: p,
                        geo: Geo::Line,
                        line: b.line,
                        to: to.clone(),
                        f,
                        mode,
                        entry: true,
                        rapid,
                    });
                }
                at = p;
                approach = Some(b.clone());
            }
            Ev::Rapid(_) | Ev::Feed { .. } if off => {
                // straight from the compensated end to the programmed point
                retract = Some(b.clone());
                break;
            }
            Ev::Arc { .. } if off => {
                return Err((b.line, "the move after G40 must be straight".into()));
            }
            Ev::Feed { to, f, mode } => {
                let p = xy(to, b.line)?;
                if norm(sub(p, at)) == 0.0 {
                    return Err((
                        b.line,
                        "a block with no X/Y motion while compensation is on is not modelled".into(),
                    ));
                }
                els.push(El {
                    a: at,
                    b: p,
                    geo: Geo::Line,
                    line: b.line,
                    to: to.clone(),
                    f: *f,
                    mode: *mode,
                    entry: false,
                    rapid: false,
                });
                at = p;
            }
            Ev::Arc { .. } if approach.is_none() => {
                return Err((b.line, "the G41/G42 block must be a G0 or G1 move".into()));
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
                if *plane != 17 {
                    return Err((b.line, "an arc outside G17 under compensation".into()));
                }
                let p = xy(to, b.line)?;
                let c = match centre {
                    Centre::Abs([a, bb]) => [qf(*a), qf(*bb)],
                    Centre::R(rw) => {
                        let r = qf(*rw);
                        let d = sub(p, at);
                        let dl = norm(d);
                        let h = (r * r - dl * dl / 4.0).max(0.0).sqrt();
                        let side = if *cw { -1.0 } else { 1.0 } * if r < 0.0 { -1.0 } else { 1.0 };
                        let m = add(at, mul(d, 0.5));
                        [m[0] - side * h * d[1] / dl, m[1] + side * h * d[0] / dl]
                    }
                };
                if !*full && norm(sub(p, at)) == 0.0 {
                    return Err((b.line, "an arc with no X/Y motion under compensation".into()));
                }
                els.push(El {
                    a: at,
                    b: p,
                    geo: Geo::Arc {
                        c,
                        ccw: !*cw,
                        full: *full,
                    },
                    line: b.line,
                    to: to.clone(),
                    f: *f,
                    mode: *mode,
                    entry: false,
                    rapid: false,
                });
                at = p;
            }
            Ev::Rapid(_) => {
                return Err((b.line, "a rapid while compensation is on is not modelled".into()));
            }
            _ => {
                return Err((
                    b.line,
                    "a tool change, offset, home, probe or other non-path event while compensation is on is not modelled".into(),
                ));
            }
        }
    }
    let (approach, retract) = match (approach, retract) {
        (Some(a), Some(r)) => (a, r),
        _ => {
            let line = bs.last().map(|b| b.line).unwrap_or(0);
            return Err((
                line,
                "compensation still on at the program's end (no G40 retract)".into(),
            ));
        }
    };
    let n = els.len();
    // each block's compensated ends; the entry (LinuxCNC) starts where the tool is
    let mut se: Vec<(P, P)> = els
        .iter()
        .map(|e| (if e.entry { e.a } else { e.off(e.a, s) }, e.off(e.b, s)))
        .collect();
    for e in &els {
        if let Geo::Arc { c, ccw, .. } = &e.geo {
            let r = norm(sub(e.a, *c));
            let r1 = if *ccw { r - s } else { r + s };
            if r1 <= 0.0 {
                return Err((
                    e.line,
                    format!("the compensated arc's radius {r1:.4} ≤ 0 (the tool is larger than the arc)"),
                ));
            }
        }
    }
    // corners: an inserted circle after block i (outside, on a control that rounds them)
    let mut corner_arc: Vec<Option<(P, P, bool)>> = vec![None; n];
    for i in 0..n.saturating_sub(1) {
        let (ea, eb) = (&els[i], &els[i + 1]);
        let p = ea.b;
        let (ta, tb) = (ea.t1(), eb.t0());
        let (cr, dt) = (cross(ta, tb), dot(ta, tb));
        let (e_end, f_start) = (se[i].1, se[i + 1].0);
        if norm(sub(e_end, f_start)) < 1e-9 {
            continue; // tangent (or a zero radius)
        }
        // the turn toward the outside (the side away from the tool), radians
        let conv = if cr.abs() < 1e-12 && dt < 0.0 {
            dmath::PI
        } else {
            let phi = dmath::atan2(cr, dt);
            if s > 0.0 { -phi } else { phi }
        };
        if matches!(ea.geo, Geo::Arc { full: true, .. }) || matches!(eb.geo, Geo::Arc { full: true, .. }) {
            return Err((
                eb.line,
                "a full circle meeting a block at a corner under compensation is not modelled".into(),
            ));
        }
        if model.tol > 0.0 {
            if conv.abs() <= model.tol {
                // within the control's tolerance: no circle, no trim; a line goes on from where
                // the last block ended (LinuxCNC convert_straight_comp2)
                if !matches!(eb.geo, Geo::Line) {
                    return Err((
                        eb.line,
                        "an arc meeting a block within the control's corner tolerance is not modelled".into(),
                    ));
                }
                se[i + 1].0 = e_end;
                continue;
            }
            if conv > dmath::PI - model.tol {
                return Err((eb.line, "a reversal under compensation is not modelled".into()));
            }
        }
        let outside = conv > 0.0;
        if outside && arc_corners {
            // a circle of radius |s| about the corner: clockwise when the tool is on the left
            corner_arc[i] = Some((f_start, p, s > 0.0));
            continue;
        }
        let x = nearest(meet(ea.eq(s), eb.eq(s)), p).ok_or((
            eb.line,
            if outside {
                "the equidistants do not meet at this outside corner (G451)".to_string()
            } else {
                "the compensated blocks do not meet at this inside corner (the control's look-ahead is not modelled)".to_string()
            },
        ))?;
        se[i].1 = x;
        se[i + 1].0 = x;
    }
    // every block kept its sense: not shortened past its start (an inside corner's bottleneck)
    for (i, e) in els.iter().enumerate() {
        let (a, b) = se[i];
        let ok = match &e.geo {
            Geo::Line => dot(sub(b, a), sub(e.b, e.a)) > 0.0,
            Geo::Arc { c, ccw, full } => {
                if *full {
                    true
                } else {
                    let sg = if *ccw { 1.0 } else { -1.0 };
                    let ang = |q: P| dmath::atan2(q[1] - c[1], q[0] - c[0]);
                    let mut th0 = sg * (ang(e.b) - ang(e.a));
                    while th0 <= 0.0 {
                        th0 += dmath::TAU;
                    }
                    let us = sg * wrap(ang(a) - ang(e.a));
                    let ue = th0 + sg * wrap(ang(b) - ang(e.b));
                    ue - us > 0.0
                }
            }
        };
        if !ok {
            return Err((
                e.line,
                "the compensated block is shortened past its start (a bottleneck: the control's look-ahead is not modelled)".into(),
            ));
        }
    }
    let with_xy = |to: &[Option<Q>], p: P| -> Vec<Option<Q>> {
        let mut v = to.to_vec();
        v[ix[0]] = Some(fq(p[0]));
        v[ix[1]] = Some(fq(p[1]));
        v
    };
    let qp = |p: P| [fq(p[0]), fq(p[1])];
    let mut last = qp(start);
    let mut pt = passthrough.into_iter().peekable();
    if model.entry_norm {
        // NORM: straight to the first block's compensated start
        let mut ap = approach.clone();
        match &mut ap.ev {
            Ev::Rapid(to) | Ev::Feed { to, .. } => *to = with_xy(to, se[0].0),
            _ => unreachable!(),
        }
        out.push(ap);
        last = qp(se[0].0);
    }
    while let Some((_, b)) = pt.next_if(|(k, _)| *k == 0) {
        out.push(b);
    }
    for (i, e) in els.iter().enumerate() {
        let (_, b) = se[i];
        let to = with_xy(&e.to, b);
        let ev = match &e.geo {
            Geo::Line if e.rapid => Ev::Rapid(to.clone()),
            Geo::Line => Ev::Feed {
                to: to.clone(),
                f: e.f,
                mode: e.mode,
            },
            Geo::Arc { c, ccw, full } => Ev::Arc {
                to: to.clone(),
                plane: 17,
                centre: Centre::Abs(qp(*c)),
                cw: !*ccw,
                full: *full,
                f: e.f,
                mode: e.mode,
            },
        };
        let end = qp(b);
        // a block whose rounded ends coincide moves nowhere (an arc so would be a whole turn)
        if end != last || matches!(e.geo, Geo::Arc { full: true, .. }) {
            out.push(Block { line: e.line, ev });
            last = end;
        }
        // the events that followed block i
        while let Some((_, b)) = pt.next_if(|(k, _)| *k == i + 1) {
            out.push(b);
        }
        if let Some((to_p, c, cw)) = corner_arc[i] {
            let nx = &els[i + 1];
            if qp(to_p) != last {
                let mut to2 = e.to.clone();
                to2[ix[0]] = Some(fq(to_p[0]));
                to2[ix[1]] = Some(fq(to_p[1]));
                out.push(Block {
                    line: nx.line,
                    ev: Ev::Arc {
                        to: to2,
                        plane: 17,
                        centre: Centre::Abs(qp(c)),
                        cw,
                        full: false,
                        f: nx.f,
                        mode: nx.mode,
                    },
                });
                last = qp(to_p);
            }
        }
    }
    for (_, b) in pt {
        out.push(b);
    }
    // the retract: from the last block's compensated end straight to the uncompensated end
    out.push(retract);
    Ok(j)
}

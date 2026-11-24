//! One bend on the press: the part placed in air bending, its pieces and the tools as exact
//! sections, and every check through the motion.
//!
//! Press frame: x across the bed, +x towards the backgauge; y along the bed, 0 at the bend's
//! middle; z up, 0 on the die's top. The part is turned so the bend's inside faces the punch (its
//! reference skin up for a bend towards it, down otherwise: `flip`), and end for end (`rotate`,
//! which leg goes to the backgauge). Both legs lie tangent to the die's shoulders and turn by φ/2
//! each; the punch tip sits on the zone's inside midline. With x_s, z_s = -r_s the right
//! shoulder's centre, the zone's outside midpoint is at
//!   z_O(φ) = (-x_s sin(φ/2) + (r_s + R' + t)(1 - cos(φ/2))) / cos(φ/2),  R' = BA/φ + (R - rn),
//! written with sinc and (1 - cos x)/x so φ → 0 is exact, and the tip at z_O + t.
//!
//! Phases: `approach` (the part flat on the die, the punch coming down from top dead centre),
//! `bend` (0 to the overbent turn and back to the turn, the punch following the tip), `return`
//! (the punch back up past the sprung part: a warning, since a part closed round the punch is
//! slid off along the bed).
use crate::bb::{self, Cell, Problem, Verdict};
use crate::geo::{Iso3, Region, Rigid2, Seg, Slot, V3, sd_prism};
use crate::model::{BendM, Model};
use crate::num::Sc;
use crate::tooling::{Die, Punch, Tooling, cover_within};
use forge_math::{Interval, Point2, Vec2, dmath};
use forge_sheet::{JointId, Strip};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rule {
    /// Part against a tool: punch, die, ram, bed or backgauge finger.
    ToolCollision,
    /// An already-bent flange closes on the punch (a box or a channel).
    ClosesOnPunch,
    /// The moving side of the part against the rest of it.
    SelfCollision,
    /// A leg too short to reach past its die shoulder's contact.
    MinFlange,
    /// The shoulder's contact falls on the bend zone, not on a leg: V too narrow for the bend.
    VTooNarrow,
    /// The zone reaches the bottom of the V.
    VDepth,
    /// The overbent included angle is sharper than the die's.
    DieAngle,
    /// The overbent included angle is sharper than the punch's tip.
    PunchAngle,
    /// The punch's tip radius is larger than the bend's inside radius.
    PunchRadius,
    /// No gauging edge the backgauge can reach and stop on, clear of the rest of the part.
    BackgaugeReach,
    /// The part does not fit under the punch at top dead centre.
    OpenHeight,
    /// The ram travel the bend needs against the stroke.
    Stroke,
    /// The part's length along the bed against the housing gap, the tooling's against the bed.
    BedLength,
    /// No sum of the tool's segments covers the bend.
    Segments,
    /// The air-bending force against the press and the tools (with the material's strength).
    Tonnage,
    /// The punch cannot come straight back up out of the sprung part (warning).
    Withdraw,
}

impl Rule {
    pub fn word(self) -> &'static str {
        match self {
            Rule::ToolCollision => "tool_collision",
            Rule::ClosesOnPunch => "closes_on_punch",
            Rule::SelfCollision => "self_collision",
            Rule::MinFlange => "min_flange",
            Rule::VTooNarrow => "v_too_narrow",
            Rule::VDepth => "v_depth",
            Rule::DieAngle => "die_angle",
            Rule::PunchAngle => "punch_angle",
            Rule::PunchRadius => "punch_radius",
            Rule::BackgaugeReach => "backgauge_reach",
            Rule::OpenHeight => "open_height",
            Rule::Stroke => "stroke",
            Rule::BedLength => "bed_length",
            Rule::Segments => "segments",
            Rule::Tonnage => "tonnage",
            Rule::Withdraw => "withdraw",
        }
    }

    pub const ALL: [Rule; 16] = [
        Rule::ToolCollision,
        Rule::ClosesOnPunch,
        Rule::SelfCollision,
        Rule::MinFlange,
        Rule::VTooNarrow,
        Rule::VDepth,
        Rule::DieAngle,
        Rule::PunchAngle,
        Rule::PunchRadius,
        Rule::BackgaugeReach,
        Rule::OpenHeight,
        Rule::Stroke,
        Rule::BedLength,
        Rule::Segments,
        Rule::Tonnage,
        Rule::Withdraw,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    /// The part flat on the die, nothing moving (gauging).
    Static,
    Approach,
    Bend,
    Return,
}

impl Phase {
    pub fn word(self) -> &'static str {
        match self {
            Phase::Static => "static",
            Phase::Approach => "approach",
            Phase::Bend => "bend",
            Phase::Return => "return",
        }
    }
}

/// Which face of a strip: its reference skin (`top`), the back one, or a side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Top,
    Bottom,
    Side,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartRef {
    pub strip: Strip,
    pub surface: Surface,
    /// An outline segment of a wall, where the place is its side face.
    pub segment: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub rule: Rule,
    pub pass: bool,
    pub severity: Severity,
    /// The measured value, and the limit it is held to: at most the limit when `at_most`.
    pub value: f64,
    pub limit: f64,
    pub at_most: bool,
    /// A two-sided rule's other limit (the value must also be at least it).
    pub lo: Option<f64>,
    /// For a distance that passes: the certified lower bound (`value` is met by a real point).
    pub bound: Option<f64>,
    /// Proven: an exact closed form, or a certified bound / witness.
    pub certified: bool,
    /// Neither proven clear nor proven violated (the search's budget, or a margin under its
    /// resolution with no witness): refused, as a failure.
    pub undecided: bool,
    pub part: Option<PartRef>,
    /// The other part piece, for a self collision.
    pub other: Option<PartRef>,
    pub tool: Option<String>,
    /// Where (press frame).
    pub at: Option<[f64; 3]>,
    /// The bend's turn there.
    pub phi: Option<f64>,
    pub phase: Option<Phase>,
    /// Not checked: the press or tool value it needs is unknown (named). Never a pass; a
    /// warning, so it does not block a plan.
    pub unchecked: Option<String>,
}

impl Check {
    /// A rule that cannot be checked: `what` is unknown.
    pub fn not_checked(rule: Rule, what: &str) -> Check {
        let mut c = Check::rule(rule, f64::NAN, f64::NAN, true);
        c.pass = false;
        c.severity = Severity::Warning;
        c.certified = false;
        c.unchecked = Some(what.into());
        c
    }

    fn rule(rule: Rule, value: f64, limit: f64, at_most: bool) -> Check {
        let pass = if at_most { value <= limit } else { value >= limit };
        Check {
            rule,
            pass,
            severity: Severity::Error,
            value,
            limit,
            at_most,
            lo: None,
            bound: None,
            certified: true,
            undecided: false,
            part: None,
            other: None,
            tool: None,
            at: None,
            phi: None,
            phase: None,
            unchecked: None,
        }
    }
}

/// One step's tooling and orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Setup {
    pub punch: usize,
    pub die: usize,
    pub rotate: bool,
}

/// Where the backgauge stops the part.
#[derive(Clone, Debug, PartialEq)]
pub struct Gauged {
    /// The stop face's distance behind the V centre line.
    pub x: f64,
    /// The fingers' bottom above the die top.
    pub r: f64,
    /// The fingers' places along the bed.
    pub fingers: Vec<f64>,
    pub part: PartRef,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    /// Index into `Model::bends`.
    pub bend: usize,
    pub joint: JointId,
    /// The turn and the overbend (radians).
    pub angle: f64,
    pub overbend: f64,
    pub setup: Setup,
    pub punch: String,
    pub die: String,
    pub punch_length: Option<f64>,
    pub die_length: Option<f64>,
    /// The part turned over (its reference skin down).
    pub flip: bool,
    pub gauge: Option<Gauged>,
    /// Ram travel from top dead centre to the bottom of the bend.
    pub travel: f64,
    /// Air-bending force, kN (with the material's strength).
    pub force: Option<f64>,
    pub checks: Vec<Check>,
    /// The setups tried before this one, in the planner's order, each with the first rule it
    /// failed (punch id, die id, rotated, rule): why these tools were chosen.
    pub rejected: Vec<(String, String, bool, Rule)>,
}

impl Step {
    /// Every error-level check passes.
    pub fn pass(&self) -> bool {
        self.checks
            .iter()
            .all(|c| c.pass || c.severity == Severity::Warning)
    }

    pub fn failures(&self) -> Vec<&Check> {
        self.checks
            .iter()
            .filter(|c| !c.pass && c.severity == Severity::Error)
            .collect()
    }
}

/// Which frame a piece's points are first carried to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Wall a's side: zone j's coordinates.
    A,
    /// Wall b's side: zone j's end coordinates.
    B,
    /// Zone j itself (in zone j's coordinates, moving).
    J,
}

#[derive(Clone, Debug)]
pub enum Kind {
    /// A wall: its region in parameter coordinates, carried to its side frame.
    Wall {
        w: usize,
        region: Region,
        to_param: Rigid2,
        iso: Iso3,
    },
    /// A zone fixed at turn `phi`, carried to its side frame; its region in zone coordinates.
    Zone {
        k: usize,
        phi: f64,
        iso: Iso3,
        region: Region,
    },
    /// The zone being bent.
    ZoneJ { region: Region },
}

impl Piece {
    pub fn region(&self) -> &Region {
        match &self.kind {
            Kind::Wall { region, .. } | Kind::Zone { region, .. } | Kind::ZoneJ { region } => region,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Piece {
    pub kind: Kind,
    pub side: Side,
    /// Walls a and b of the moving bend: their parameters are zone j's (end) coordinates, so the
    /// bend's span along y is an interval of v.
    pub near: bool,
    pub strip: Strip,
    /// Its parameters' box.
    pub dom: [Interval; 3],
    /// Inverse of `iso` (side frame to parameters), for distances to it.
    pub inv: Iso3,
    /// Bent relative to the moving bend (a done bend lies between it and the zone).
    pub folded: bool,
    /// In the plane of wall a or b: joined to it by unbent zones only.
    pub on_leg: bool,
    /// The done bends on its path to the moving bend (its own, for a zone), as a mask.
    pub path: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Attach {
    Punch,
    Die,
}

/// Which working part a tool element is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Work {
    Tip,
    Die,
    None,
}

#[derive(Clone, Debug)]
struct Element {
    name: String,
    sec: Region,
    y: (f64, f64),
    attach: Attach,
    /// The working part of a tool (punch tip, die): its contact with the moving bend's zone and
    /// legs is decided by the rules, not by distance.
    work: Work,
    punchy: bool,
}

/// The context every check of one step reads.
pub struct StepCtx<'a> {
    pub m: &'a Model,
    pub tl: &'a Tooling,
    pub j: usize,
    pub bj: &'a BendM,
    pub punch: &'a Punch,
    pub die: &'a Die,
    pub rot: f64,
    pub t: f64,
    pub theta: f64,
    pub phimax: f64,
    /// The fold state's fixed turns (zone j's entry unused).
    pub phi: Vec<f64>,
    pub done: Vec<bool>,
    pub pieces: Vec<Piece>,
    xs: f64,
    rs: f64,
    /// The punch tip's height at top dead centre.
    pub z_tdc: f64,
    lengths: (Option<f64>, Option<f64>),
    die_index: usize,
    punch_index: usize,
    /// The working station's centre along the bed from the bed's centre (the part is centred
    /// on its station: y = 0).
    pub oy: f64,
    /// Verdicts kept across steps: a piece's place depends only on the bends on its path to
    /// the moving one, so its verdict against a tool holds wherever those are bent alike.
    pub memo: Option<std::rc::Rc<std::cell::RefCell<Memo>>>,
    /// The legs' straight runs (far, near) and the part's length along the bed, if already known
    /// for these bends done: they read no tool.
    pub legs: Option<(f64, f64, f64)>,
}

/// A distance verdict's key: the piece (strip kind and id, the done bends on its path), the tool
/// element, the phase, the orientation, the moving bend and its overbent turn, the die (where it
/// moves the punch), and whether it was only decided or measured.
pub type MemoKey = (u8, u32, u64, String, Phase, bool, usize, u64, usize, bool);
pub type Memo = std::collections::HashMap<MemoKey, Verdict>;

/// The cells' budget for one piece against one target.
const BUDGET: usize = 60_000;

impl<'a> StepCtx<'a> {
    pub fn new(
        m: &'a Model,
        tl: &'a Tooling,
        done: &[bool],
        j: usize,
        setup: Setup,
        overbend: f64,
    ) -> StepCtx<'a> {
        let bj = &m.bends[j];
        let punch = &tl.punches[setup.punch];
        let die = &tl.dies[setup.die];
        let t = m.t;
        let theta = bj.theta;
        let phimax = (theta + overbend).min(dmath::PI - 1e-6);
        let mut phi: Vec<f64> = m
            .bends
            .iter()
            .enumerate()
            .map(|(k, b)| if done[k] { b.theta } else { 0.0 })
            .collect();
        phi[j] = 0.0;
        let frames = m.frames(&phi);
        let fzj = m.zone_frame(&frames, j);
        let sa = fzj;
        let sb = fzj.then(&Iso3::shift(bj.ba));
        let (sa_inv, sb_inv) = (sa.inverse(), sb.inverse());
        let folded_of = |w: usize| m.path_bends(j, w).iter().any(|&k| done[k] && k != j);
        // on a leg: every bend between it and zone j unbent (wall a's and b's own paths are empty)
        let on_leg = |w: usize| !folded_of(w);
        let path_mask = |w: usize| {
            m.path_bends(j, w)
                .into_iter()
                .filter(|&k| done[k] && k != j)
                .fold(0u64, |acc, k| acc | (1 << k))
        };
        let mut pieces = Vec::new();
        let zj_to_dev = bj.zdev;
        for (wi, w) in m.walls.iter().enumerate() {
            let beyond = m.beyond(j, wi);
            let side = if beyond { Side::B } else { Side::A };
            let near = wi == bj.a || wi == bj.b;
            let (to_param, iso) = if wi == bj.a {
                (zj_to_dev.inverse().then(&w.dev), Iso3::ID)
            } else if wi == bj.b {
                (
                    Rigid2::shift(-bj.ba, 0.0).then(&zj_to_dev.inverse()).then(&w.dev),
                    Iso3::ID,
                )
            } else {
                let s_inv = if beyond { &sb_inv } else { &sa_inv };
                (Rigid2::ID, s_inv.then(&frames[wi]))
            };
            let region = w.region(&m.pattern).moved(&to_param);
            let (lo, hi) = region.bbox();
            pieces.push(Piece {
                kind: Kind::Wall {
                    w: wi,
                    region,
                    to_param,
                    iso,
                },
                side,
                near,
                strip: Strip::Wall(w.id),
                dom: [
                    Interval::new(lo.x, hi.x),
                    Interval::new(lo.y, hi.y),
                    Interval::new(-t, 0.0),
                ],
                inv: iso.inverse(),
                folded: !near && folded_of(wi),
                on_leg: on_leg(wi),
                path: path_mask(wi),
            });
        }
        for (k, b) in m.bends.iter().enumerate() {
            let region = b.region(&m.pattern);
            let (lo, hi) = region.bbox();
            let dom = [
                Interval::new(lo.x, hi.x),
                Interval::new(lo.y, hi.y),
                Interval::new(-t, 0.0),
            ];
            if k == j {
                pieces.push(Piece {
                    kind: Kind::ZoneJ { region },
                    side: Side::J,
                    near: false,
                    strip: Strip::Bend(b.joint),
                    dom,
                    inv: Iso3::ID,
                    folded: false,
                    on_leg: false,
                    path: 0,
                });
                continue;
            }
            let beyond = m.beyond(j, b.b);
            let s_inv = if beyond { &sb_inv } else { &sa_inv };
            let iso = s_inv.then(&m.zone_frame(&frames, k));
            pieces.push(Piece {
                kind: Kind::Zone {
                    k,
                    phi: phi[k],
                    iso,
                    region,
                },
                side: if beyond { Side::B } else { Side::A },
                near: false,
                strip: Strip::Bend(b.joint),
                dom,
                inv: iso.inverse(),
                folded: done[k] || folded_of(b.a),
                on_leg: !done[k] && on_leg(b.a) && on_leg(b.b),
                path: path_mask(b.a) | path_mask(b.b) | (u64::from(done[k]) << k),
            });
        }
        let s = die.shoulder();
        let z_tdc = tl.press.open_height - die.height - punch.height;
        // the longest tooling that covers the bend less its allowed ends, short of the bend's own
        // length by the clearance at each end (flanges standing at the ends stay clear)
        let need = (bj.len - 2.0 * tl.short_end.unwrap_or(t.max(3.0))).max(0.0);
        let room = bj.len - 2.0 * tl.clearance;
        let station = tl.station(setup.punch, setup.die);
        let lengths = match station {
            Some(st) => (Some(st.punch_length), Some(st.die_length)),
            None => (
                cover_within(&punch.segments, need, room).map(|c| c.0),
                cover_within(&die.segments, need, room).map(|c| c.0),
            ),
        };
        let oy = station.map_or(0.0, |st| st.y);
        StepCtx {
            m,
            tl,
            j,
            bj,
            punch,
            die,
            rot: if setup.rotate { -1.0 } else { 1.0 },
            t,
            theta,
            phimax,
            phi,
            done: done.to_vec(),
            pieces,
            xs: s.x,
            rs: die.shoulder_radius,
            z_tdc,
            lengths,
            die_index: setup.die,
            punch_index: setup.punch,
            oy,
            memo: None,
            legs: None,
        }
    }

    /// A piece's point in its side frame, the moving bend at turn φ.
    pub fn side_point<T: Sc>(&self, pc: &Piece, u: T, v: T, w: T, phi: T) -> V3<T> {
        match &pc.kind {
            Kind::Wall { iso, .. } => iso.apply(V3::new(u, v, w)),
            Kind::Zone { k, phi: pk, iso, .. } => iso.apply(self.m.bends[*k].fold(u, v, w, T::c(*pk))),
            Kind::ZoneJ { .. } => self.bj.fold(u, v, w, phi),
        }
    }

    /// A side-frame point to the press, relative to the punch tip.
    pub fn rel<T: Sc>(&self, q: V3<T>, side: Side, phi: T) -> V3<T> {
        let b = self.bj;
        let p = match side {
            Side::B => b.end_apply(phi, q),
            _ => q,
        };
        let pm = b.fold(T::c(0.5 * b.ba), T::c(0.5 * b.len), T::c(b.w_inside(self.t)), phi);
        let h = phi * 0.5;
        let (s, c) = (h.sin(), h.cos());
        let d = p.minus(pm);
        let x = (d.x * c + d.z * (s * b.sigma)) * self.rot;
        let y = d.y * (self.rot * b.sigma);
        let z = d.z * (c * b.sigma) - d.x * s;
        V3::new(x, y, z)
    }

    /// The punch tip's height above the die top at turn φ.
    pub fn ztip<T: Sc>(&self, phi: T) -> T {
        let b = self.bj;
        let h = phi * 0.5;
        let k = self.rs + b.r - b.rn + self.t;
        let zo = (h.sin() * (-self.xs) + h.vers() * k + h.vsc() * (0.5 * b.ba)) * h.cos().recip();
        zo + self.t
    }

    /// Along the leg from the zone's end to the die shoulder's contact, at turn φ.
    pub fn contact<T: Sc>(&self, phi: T) -> T {
        let b = self.bj;
        let h = phi * 0.5;
        let (s, c) = (h.sin(), h.cos());
        let zo = self.ztip(phi) + (-self.t);
        let k = b.r - b.rn + self.t;
        let kx = s * (-self.rs) + self.xs;
        let kz = c * self.rs + (-self.rs);
        let ax = h.sinc() * (0.5 * b.ba) + s * k;
        let az = zo + h.vsc() * (0.5 * b.ba) + h.vers() * k;
        (kx - ax) * c + (kz - az) * s
    }

    fn image<T: Sc>(&self, pc: &Piece, q: [T; 4], phase: Phase, attach: Attach) -> V3<T> {
        let (phi, lift): (T, Option<T>) = match phase {
            Phase::Static => (T::c(0.0), None),
            Phase::Bend => (q[3], None),
            Phase::Approach => (T::c(0.0), Some(q[3])),
            Phase::Return => (T::c(self.theta), Some(q[3])),
        };
        let r = self.rel(self.side_point(pc, q[0], q[1], q[2], phi), pc.side, phi);
        let to_die = |r: V3<T>| V3::new(r.x, r.y, r.z + self.ztip(phi));
        match (attach, lift) {
            (Attach::Punch, None) => r,
            (Attach::Punch, Some(h)) => {
                let d = to_die(r);
                V3::new(d.x, d.y, d.z - h)
            }
            (Attach::Die, _) => to_die(r),
        }
    }

    fn roots(&self, pc: &Piece, s: Interval) -> Vec<Cell> {
        let [u, v, w] = pc.dom;
        vec![Cell {
            p: [u, v, w, s],
            tag: 0,
        }]
    }

    /// Whether the whole cell lies where a working tool cannot reach it: the moving zone (its
    /// contact is the rules'), or past a leg's tangent line on the material side of that leg's
    /// skin plane. The die lies wholly under each leg's outside skin plane there, the punch tip
    /// wholly over its inside one, while the angle and radius rules hold; so nothing there can
    /// meet them, and their resting contact is not a clearance.
    fn on_working(&self, pc: &Piece, c: &Cell, kind: Work) -> bool {
        if kind == Work::None {
            return false;
        }
        if matches!(pc.kind, Kind::ZoneJ { .. }) {
            return true;
        }
        let q = self.side_point(pc, c.p[0], c.p[1], c.p[2], Interval::point(0.0));
        let e = 1e-9;
        let past = match pc.side {
            Side::A => q.x.hi <= e,
            Side::B => q.x.lo >= -e,
            Side::J => return true,
        };
        if !past {
            return false;
        }
        let sg = self.bj.sigma;
        let w_in = self.bj.w_inside(self.t);
        let w_out = if sg > 0.0 { -self.t } else { 0.0 };
        match kind {
            // over the outside skin plane (towards the inside)
            Work::Die => {
                if sg > 0.0 {
                    q.z.lo >= w_out - e
                } else {
                    q.z.hi <= w_out + e
                }
            }
            // under the inside skin plane (towards the outside)
            Work::Tip => {
                if sg > 0.0 {
                    q.z.hi <= w_in + e
                } else {
                    q.z.lo >= w_in - e
                }
            }
            Work::None => false,
        }
    }

    fn may_hold(pc: &Piece, c: &Cell) -> bool {
        let p = Point2::new(c.p[0].mid(), c.p[1].mid());
        let hd = 0.5 * (c.p[0].width().powi(2) + c.p[1].width().powi(2)).sqrt();
        pc.region().dist_lower(p) <= hd + 1e-12
    }

    fn witness(pc: &Piece, c: &Cell) -> Option<[f64; 4]> {
        let m = c.mid();
        pc.region()
            .rep(Point2::new(m[0], m[1]))
            .map(|p| [p.x, p.y, m[2], m[3]])
    }

    /// The piece's place named for a witness at parameters q.
    pub fn part_ref(&self, pc: &Piece, q: [f64; 4]) -> PartRef {
        let t = self.t;
        let mut segment = None;
        let surface = if q[2] > -0.25 * t {
            Surface::Top
        } else if q[2] < -0.75 * t {
            Surface::Bottom
        } else {
            Surface::Side
        };
        if let Kind::Wall { w, to_param, .. } = &pc.kind {
            // the nearest outline segment, if the point is at the edge
            let p = to_param.inverse().apply(Point2::new(q[0], q[1]));
            let wall = &self.m.walls[*w];
            let mut best = (f64::INFINITY, None);
            for (id, s) in &wall.free {
                let d = s.dist(p);
                if d < best.0 {
                    best = (d, Some(*id));
                }
            }
            if best.0 < 0.5 * t {
                segment = best.1;
            }
        }
        PartRef {
            strip: pc.strip,
            surface,
            segment,
        }
    }

    fn elements(&self, phase: Phase, gauge: Option<&Gauged>) -> Vec<Element> {
        let tl = self.tl;
        let pr = &tl.press;
        let mut out = Vec::new();
        let (plen, dlen) = self.tool_lengths();
        let half = |l: Option<f64>| 0.5 * l.unwrap_or(self.bj.len);
        let (py, dy) = (half(plen), half(dlen));
        let bed = 0.5 * pr.bed_length;
        // the beams along the bed, in the station's frame
        let beams = (-bed - self.oy, bed - self.oy);
        let ph = self.punch.height;
        if phase != Phase::Static {
            out.push(Element {
                name: format!("punch:{}/tip", self.punch.id),
                sec: self.punch.tip_section(),
                y: (-py, py),
                attach: Attach::Punch,
                work: Work::Tip,
                punchy: true,
            });
            out.push(Element {
                name: format!("punch:{}/body", self.punch.id),
                sec: self.punch.body_section(),
                y: (-py, py),
                attach: Attach::Punch,
                work: Work::None,
                punchy: true,
            });
            out.push(Element {
                name: "ram".into(),
                sec: rect(-pr.ram.front, ph, pr.ram.back, ph + 2000.0),
                y: beams,
                attach: Attach::Punch,
                work: Work::None,
                punchy: true,
            });
        }
        if phase == Phase::Static || phase == Phase::Bend {
            out.push(Element {
                name: format!("die:{}", self.die.id),
                sec: self.die.section(),
                y: (-dy, dy),
                attach: Attach::Die,
                work: Work::Die,
                punchy: false,
            });
            let dh = self.die.height;
            out.push(Element {
                name: "bed".into(),
                sec: rect(-pr.bed.front, -dh - 2000.0, pr.bed.back, -dh),
                y: beams,
                attach: Attach::Die,
                work: Work::None,
                punchy: false,
            });
        }
        // the other stations' tools: punches ride the ram, set to this step's punch height
        // (their tips meet the same sheet line when all punches are one height; a taller or
        // shorter one is placed by its own height from the clamp)
        for (k, st) in tl.stations.iter().enumerate() {
            if st.punch == self.punch_index && st.die == self.die_index {
                continue;
            }
            let (p, d) = (&tl.punches[st.punch], &tl.dies[st.die]);
            let y = st.y - self.oy;
            if phase != Phase::Static {
                let lift = ph - p.height;
                for (part, sec) in [("tip", p.tip_section()), ("body", p.body_section())] {
                    out.push(Element {
                        name: format!("station{}:punch:{}/{part}", k + 1, p.id),
                        sec: shifted(&sec, 0.0, lift),
                        y: (y - 0.5 * st.punch_length, y + 0.5 * st.punch_length),
                        attach: Attach::Punch,
                        work: Work::None,
                        punchy: true,
                    });
                }
            }
            if phase == Phase::Static || phase == Phase::Bend {
                let drop = self.die.height - d.height;
                out.push(Element {
                    name: format!("station{}:die:{}", k + 1, d.id),
                    sec: shifted(&d.section(), 0.0, drop),
                    y: (y - 0.5 * st.die_length, y + 0.5 * st.die_length),
                    attach: Attach::Die,
                    work: Work::None,
                    punchy: false,
                });
            }
        }
        // the frame's side housings, behind their throats
        if let Some(f) = pr.frame {
            let g = 0.5 * pr.housing_gap;
            for (side, (y0, y1)) in [("left", (-g - f.thickness, -g)), ("right", (g, g + f.thickness))] {
                out.push(Element {
                    name: format!("frame:{side}"),
                    sec: rect(f.throat, -self.die.height - 4000.0, f.throat + f.depth, 6000.0),
                    y: (y0 - self.oy, y1 - self.oy),
                    attach: Attach::Die,
                    work: Work::None,
                    punchy: false,
                });
            }
        }
        if let Some(g) = gauge {
            let bg = &pr.backgauge;
            let gap = match phase {
                Phase::Static | Phase::Approach => 2.0 * tl.clearance,
                _ => bg.retract.max(2.0 * tl.clearance),
            };
            for (i, y) in g.fingers.iter().enumerate() {
                out.push(Element {
                    name: format!("backgauge:finger{}", i + 1),
                    sec: rect(
                        g.x + gap,
                        g.r,
                        g.x + gap + bg.finger.depth,
                        g.r + bg.finger.height,
                    ),
                    y: (y - 0.5 * bg.finger.width, y + 0.5 * bg.finger.width),
                    attach: Attach::Die,
                    work: Work::None,
                    punchy: false,
                });
            }
        }
        out
    }

    /// The punch's and the die's lengths: the shortest sums of their segments that cover the bend
    /// less its allowed ends.
    pub fn tool_lengths(&self) -> (Option<f64>, Option<f64>) {
        self.lengths
    }

    fn need(&self) -> f64 {
        let se = self.tl.short_end.unwrap_or(self.t.max(3.0));
        (self.bj.len - 2.0 * se).max(0.0)
    }

    /// Distance checks of every piece against the phase's tools.
    pub fn collisions(&self, phase: Phase, gauge: Option<&Gauged>, out: &mut Vec<Check>, quick: bool) {
        self.tool_checks(phase, gauge, out, quick, &|_| true);
    }

    /// `collisions` against the tools that ride the ram (punch, ram) when `punch`, else against
    /// the fixed ones (die, bed, backgauge).
    pub fn collisions_of(
        &self,
        phase: Phase,
        gauge: Option<&Gauged>,
        out: &mut Vec<Check>,
        quick: bool,
        punch: bool,
    ) {
        self.tool_checks(phase, gauge, out, quick, &|e| {
            (e.attach == Attach::Punch) == punch
        });
    }

    fn tool_checks(
        &self,
        phase: Phase,
        gauge: Option<&Gauged>,
        out: &mut Vec<Check>,
        quick: bool,
        keep: &dyn Fn(&Element) -> bool,
    ) {
        let s = match phase {
            Phase::Static => Interval::point(0.0),
            Phase::Approach => Interval::new(self.ztip(0.0), self.z_tdc.max(self.ztip(0.0))),
            Phase::Bend => Interval::new(0.0, self.phimax),
            Phase::Return => Interval::new(self.ztip(self.theta), self.z_tdc.max(self.ztip(self.theta))),
        };
        let limit = self.tl.clearance;
        let res = (limit / 4.0).max(0.01);
        let els: Vec<Element> = self
            .elements(phase, gauge)
            .into_iter()
            .filter(|e| keep(e))
            .collect();
        let mut worst: [Option<Check>; 3] = [None, None, None];
        let slot = |r: Rule| match r {
            Rule::ClosesOnPunch => 1,
            Rule::Withdraw => 2,
            _ => 0,
        };
        let mut lowest = (f64::INFINITY, f64::INFINITY, String::new());
        // the pairs, nearest first by their whole bound, so a measured distance cuts the rest short
        let mut pairs: Vec<(f64, &Element, &Piece)> = Vec::new();
        for el in &els {
            for pc in &self.pieces {
                if el.work != Work::None && matches!(pc.kind, Kind::ZoneJ { .. }) {
                    continue;
                }
                let pb = ToolPb {
                    ctx: self,
                    pc,
                    el,
                    phase,
                };
                let root = self.roots(pc, s)[0];
                pairs.push((pb.lower(&pb.enclose(&root)).0, el, pc));
            }
        }
        pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
        'outer: for (_, el, pc) in pairs {
            {
                let roots = self.roots(pc, s);
                let pb = ToolPb {
                    ctx: self,
                    pc,
                    el,
                    phase,
                };
                let c0 = bb::CELLS.load(std::sync::atomic::Ordering::Relaxed);
                let t0 = std::time::Instant::now();
                let key: Option<MemoKey> = (!el.name.starts_with("backgauge")).then(|| {
                    let (kind, id) = match pc.strip {
                        Strip::Wall(w) => (0u8, w.0),
                        Strip::Bend(b) => (1u8, b.0),
                    };
                    // where the punch is placed by the die (approach, return) or the tool is the die
                    let die = if el.attach == Attach::Die || phase != Phase::Bend {
                        self.die_index + 1
                    } else {
                        0
                    };
                    (
                        kind,
                        id,
                        pc.path,
                        // with stations, where a tool stands depends on the working station
                        // and its punch: kept apart
                        if self.tl.stations.is_empty() {
                            el.name.clone()
                        } else {
                            format!("{}@{}/{}", el.name, self.oy.to_bits(), self.punch_index)
                        },
                        phase,
                        self.rot < 0.0,
                        self.j,
                        self.phimax.to_bits(),
                        die,
                        quick,
                    )
                });
                let kept = key
                    .as_ref()
                    .and_then(|k| self.memo.as_ref().and_then(|m| m.borrow().get(k).copied()));
                let v = match kept {
                    // a measured verdict also settles a decision; a cut one is measured afresh
                    Some(v) => v,
                    None => {
                        let v = bb::certify_below(
                            &pb,
                            &roots,
                            limit,
                            res,
                            BUDGET,
                            !quick,
                            if quick { f64::INFINITY } else { lowest.1 },
                        );
                        if let (Some(k), Some(m)) = (key, self.memo.as_ref())
                            && (quick || lowest.1.is_infinite())
                        {
                            m.borrow_mut().insert(k, v);
                        }
                        v
                    }
                };
                if std::env::var_os("BENDSIM_TRACE").is_some() {
                    eprintln!(
                        "trace {:?} {} {:?} quick {} cells {} {:?} {:?}",
                        phase,
                        el.name,
                        pc.strip,
                        quick,
                        bb::CELLS.load(std::sync::atomic::Ordering::Relaxed) - c0,
                        t0.elapsed(),
                        v
                    );
                }
                let (hit, value, q, at) = match v {
                    Verdict::Clear { lower, upper } => {
                        if upper < lowest.1 {
                            lowest = (lower, upper, el.name.clone());
                        }
                        lowest.0 = lowest.0.min(lower);
                        continue;
                    }
                    Verdict::Hit { d, q, at } => (true, d, Some(q), Some(at)),
                    Verdict::Undecided { lower } => (false, lower, None, None),
                };
                let rule = if phase == Phase::Return {
                    Rule::Withdraw
                } else if el.punchy && pc.folded {
                    Rule::ClosesOnPunch
                } else {
                    Rule::ToolCollision
                };
                let k = slot(rule);
                if worst[k].is_none() {
                    let mut c = Check::rule(rule, value, limit, false);
                    c.pass = false;
                    c.certified = hit;
                    c.undecided = !hit;
                    c.severity = if rule == Rule::Withdraw {
                        Severity::Warning
                    } else {
                        Severity::Error
                    };
                    c.tool = Some(el.name.clone());
                    c.phase = Some(phase);
                    if let Some(q) = q {
                        c.part = Some(self.part_ref(pc, q));
                        c.phi = Some(match phase {
                            Phase::Bend => q[3],
                            Phase::Return => self.theta,
                            _ => 0.0,
                        });
                    }
                    c.at = at.map(|a| self.press_point(a, el.attach, phase, q));
                    worst[k] = Some(c);
                    if quick {
                        break 'outer;
                    }
                }
            }
        }
        let main = if phase == Phase::Return {
            Rule::Withdraw
        } else {
            Rule::ToolCollision
        };
        for (k, w) in worst.into_iter().enumerate() {
            match w {
                Some(c) => out.push(c),
                None if k == slot(main) => {
                    let mut c = Check::rule(main, lowest.1.min(1e9), limit, false);
                    c.bound = Some(lowest.0.min(1e9));
                    if !lowest.2.is_empty() {
                        c.tool = Some(lowest.2.clone());
                    }
                    c.pass = true;
                    c.phase = Some(phase);
                    c.severity = if main == Rule::Withdraw {
                        Severity::Warning
                    } else {
                        Severity::Error
                    };
                    out.push(c);
                }
                None => {}
            }
        }
    }

    /// A target-frame point back in the press frame (die-fixed).
    fn press_point(&self, a: [f64; 3], attach: Attach, phase: Phase, q: Option<[f64; 4]>) -> [f64; 3] {
        match attach {
            Attach::Die => a,
            Attach::Punch => {
                let h = match (phase, q) {
                    (Phase::Bend, Some(q)) => self.ztip(q[3]),
                    (Phase::Approach | Phase::Return, Some(q)) => q[3],
                    _ => self.ztip(0.0),
                };
                [a[0], a[1], a[2] + h]
            }
        }
    }

    /// The moving side against the rest of the part, through the bend.
    pub fn self_check(&self, quick: bool) -> Check {
        // the least gap allowed (0: touching is the limit), less the touch tolerance
        let limit = self.tl.self_clearance - 1e-5;
        let res = 0.02;
        let s = Interval::new(0.0, self.phimax);
        let (a, b) = (self.bj.a, self.bj.b);
        let wall_of = |pc: &Piece| match pc.kind {
            Kind::Wall { w, .. } => Some(w),
            _ => None,
        };
        let mut lowest = (f64::INFINITY, f64::INFINITY);
        let movers: Vec<&Piece> = self.pieces.iter().filter(|p| p.side != Side::A).collect();
        for mv in &movers {
            for st in self.pieces.iter().filter(|p| p.side != Side::J) {
                if st.side == mv.side {
                    continue;
                }
                // zone j borders walls a and b
                if mv.side == Side::J && (wall_of(st) == Some(a) || wall_of(st) == Some(b)) {
                    continue;
                }
                let pb = SelfPb {
                    ctx: self,
                    mv,
                    st,
                    frame: st.side,
                };
                let roots = self.roots(mv, s);
                let c0 = bb::CELLS.load(std::sync::atomic::Ordering::Relaxed);
                let t0 = std::time::Instant::now();
                let v = bb::certify(&pb, &roots, limit, res, BUDGET, !quick);
                if std::env::var_os("BENDSIM_TRACE").is_some() {
                    eprintln!(
                        "trace self {:?} vs {:?} quick {} cells {} {:?} {:?}",
                        mv.strip,
                        st.strip,
                        quick,
                        bb::CELLS.load(std::sync::atomic::Ordering::Relaxed) - c0,
                        t0.elapsed(),
                        v
                    );
                }
                match v {
                    Verdict::Clear { lower, upper } => {
                        if upper < lowest.1 {
                            lowest = (lower, upper);
                        }
                    }
                    v => {
                        let mut c = Check::rule(Rule::SelfCollision, 0.0, limit, false);
                        c.pass = false;
                        c.phase = Some(Phase::Bend);
                        match v {
                            Verdict::Hit { d, q, at } => {
                                c.value = d;
                                c.part = Some(self.part_ref(mv, q));
                                c.phi = Some(q[3]);
                                c.at = Some(self.frame_to_press(at, st.side, q[3]));
                                let pq = st.inv.apply(V3::new(at[0], at[1], at[2]));
                                c.other = Some(self.part_ref(st, [pq.x, pq.y, pq.z, 0.0]));
                            }
                            Verdict::Undecided { lower } => {
                                c.value = lower;
                                c.certified = false;
                                c.undecided = true;
                                c.other = Some(self.part_ref(st, [0.0; 4]));
                            }
                            Verdict::Clear { .. } => unreachable!(),
                        }
                        let _ = quick;
                        return c;
                    }
                }
            }
        }
        let mut c = Check::rule(Rule::SelfCollision, lowest.1.min(1e9), limit, false);
        c.bound = Some(lowest.0.min(1e9));
        c.pass = true;
        c.phase = Some(Phase::Bend);
        c
    }

    fn frame_to_press(&self, p: [f64; 3], side: Side, phi: f64) -> [f64; 3] {
        let r = self.rel(V3::new(p[0], p[1], p[2]), side, phi);
        [r.x, r.y, r.z + self.ztip(phi)]
    }

    /// The straight run of a leg from the zone's tangent line, the least along the bend: from
    /// wall b's tangent line outwards when `far`, else from wall a's. Unbent bends continue it.
    pub fn leg_run(&self, far: bool) -> f64 {
        let b = self.bj;
        let m = self.m;
        let (u0, sgn) = if far { (b.ba, 1.0) } else { (0.0, -1.0) };
        let dir = b.zdev.vec(Vec2::new(sgn, 0.0));
        let mut ys: Vec<f64> = vec![1e-6 * b.len, 0.5 * b.len, b.len * (1.0 - 1e-6)];
        let to_zone = b.zdev.inverse();
        let mut outlines: Vec<(usize, Vec<Seg>)> = Vec::new();
        for (wi, w) in m.walls.iter().enumerate() {
            for l in &w.outline {
                let segs: Vec<Seg> = l.iter().map(|s| s.moved(&w.dev)).collect();
                for s in &segs {
                    for p in [s.start(), s.end()] {
                        let y = to_zone.apply(p).y;
                        if y > 0.0 && y < b.len {
                            ys.push((y - 1e-6).max(1e-6 * b.len));
                            ys.push((y + 1e-6).min(b.len * (1.0 - 1e-6)));
                        }
                    }
                }
                outlines.push((wi, segs));
            }
        }
        ys.sort_by(f64::total_cmp);
        ys.dedup();
        let slots: Vec<(usize, Slot)> = m
            .bends
            .iter()
            .enumerate()
            .map(|(k, z)| (k, z.slot().moved(&z.zdev)))
            .collect();
        let outline_regions: Vec<(Rigid2, Region)> = m
            .walls
            .iter()
            .map(|w| (w.dev.inverse(), Region::of(w.outline.clone())))
            .collect();
        let material = |p: Point2| -> bool {
            for (k, s) in &slots {
                if *k != self.j && self.done[*k] && s.inside(p, 0.0) {
                    return false;
                }
            }
            if slots[self.j].1.inside(p, 0.0) {
                return false;
            }
            let in_wall = outline_regions.iter().any(|(inv, r)| r.in_loops(inv.apply(p)));
            in_wall
                || slots
                    .iter()
                    .any(|(k, s)| *k != self.j && !self.done[*k] && s.inside(p, 0.0))
        };
        let mut least = f64::INFINITY;
        for y in ys {
            let p0 = b.zdev.apply(Point2::new(u0, y));
            // a sample starting off the material (in a relief, or in a bent neighbour's zone at
            // a corner) has no leg there
            if !material(p0 + dir * 1e-7) {
                continue;
            }
            let mut hits: Vec<f64> = Vec::new();
            for (_, segs) in &outlines {
                for s in segs {
                    s.ray_hits(p0, dir, &mut hits);
                }
            }
            for (_, s) in &slots {
                for g in s.segs() {
                    g.ray_hits(p0, dir, &mut hits);
                }
            }
            hits.retain(|h| *h > 1e-9);
            hits.sort_by(f64::total_cmp);
            hits.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
            let mut prev = 0.0;
            let mut run = 0.0;
            for h in hits.iter().copied().chain(std::iter::once(1e7)) {
                let mid = p0 + dir * (0.5 * (prev + h));
                if material(mid) {
                    run = h.min(1e7);
                    prev = h;
                } else {
                    break;
                }
            }
            least = least.min(run);
        }
        if least.is_finite() { least } else { 0.0 }
    }

    /// The closed-form rules of the step: angles, radius, V, legs, stroke, lengths, force.
    pub fn rules(&self, uts: Option<f64>) -> (Vec<Check>, f64, Option<f64>) {
        let mut out = Vec::new();
        let included = dmath::PI - self.phimax;
        out.push(Check::rule(Rule::DieAngle, included, self.die.angle, false));
        out.push(Check::rule(
            Rule::PunchAngle,
            included,
            self.punch.tip_angle,
            false,
        ));
        let r_in = self.bj.inside_radius(self.phimax);
        out.push(Check::rule(
            Rule::PunchRadius,
            self.punch.tip_radius,
            r_in + 1e-9,
            true,
        ));
        // the shoulder's contact on the leg through the motion, certified by bisection
        // certified: the contact's least place along the leg, and its farthest
        let (cmin, cmax, at, proven) = range_1d(|i| self.contact(i), |x| self.contact(x), 0.0, self.phimax);
        let mut c = Check::rule(Rule::VTooNarrow, cmin, 0.0, false);
        c.phi = Some(at);
        if !proven {
            c.pass = false;
            c.certified = false;
            c.undecided = true;
        }
        out.push(c);
        let (far, near, ylen) = self.leg_geometry();
        let mut mf = Check::rule(Rule::MinFlange, far.min(near), cmax.max(0.0), false);
        if !proven {
            mf.pass = false;
            mf.certified = false;
            mf.undecided = true;
        }
        let leg = if far <= near { self.bj.b } else { self.bj.a };
        mf.part = Some(PartRef {
            strip: Strip::Wall(self.m.walls[leg].id),
            surface: Surface::Bottom,
            segment: None,
        });
        out.push(mf);
        let zo = self.ztip(self.phimax) - self.t;
        out.push(Check::rule(
            Rule::VDepth,
            zo + self.die.depth(),
            self.tl.clearance,
            false,
        ));
        let travel = self.z_tdc - self.ztip(self.phimax);
        out.push(Check::rule(Rule::Stroke, travel, self.tl.press.stroke, true));
        let mut oh = Check::rule(Rule::OpenHeight, self.z_tdc, self.t + self.tl.clearance, false);
        oh.phase = Some(Phase::Approach);
        out.push(oh);
        let (pl, dl) = self.tool_lengths();
        let need = self.need();
        let mut seg = Check::rule(
            Rule::Segments,
            pl.unwrap_or(0.0).min(dl.unwrap_or(0.0)),
            need,
            false,
        );
        if pl.is_none() || dl.is_none() {
            seg.pass = false;
            seg.tool = Some(if pl.is_none() {
                format!("punch:{}", self.punch.id)
            } else {
                format!("die:{}", self.die.id)
            });
        }
        out.push(seg);

        let tool_len = pl.unwrap_or(0.0).max(dl.unwrap_or(0.0));
        let pr = &self.tl.press;
        // with the frame known, a part longer than the housing gap may pass through the
        // throats: the housings are checked by distance, so the part's own limit is the
        // bed's frame-to-frame span through them (none here)
        let ylim = if pr.frame.is_some() {
            f64::INFINITY
        } else {
            pr.housing_gap
        };
        let mut bl = Check::rule(Rule::BedLength, ylen, ylim, true);
        // the segments are summed in tenths of a millimetre: equal to the bed is not over it
        if tool_len > pr.bed_length + 1e-6 {
            bl = Check::rule(Rule::BedLength, tool_len, pr.bed_length, true);
            bl.tool = Some("bed".into());
        }
        out.push(bl);
        let force = uts.map(|u| 1.42 * u * self.t * self.t / self.die.v * self.bj.len / 1000.0);
        if let Some(f) = force {
            let per_m = f / (self.bj.len / 1000.0);
            let mut lim = pr.max_force.unwrap_or(f64::INFINITY);
            let mut tool = None;
            let mut value = f;
            for (load, name) in [
                (self.punch.max_load, &self.punch.id),
                (self.die.max_load, &self.die.id),
            ] {
                // the most binding limit, by ratio (the press in kN, a tool in kN per metre)
                if let Some(l) = load
                    && per_m / l > value / lim
                {
                    value = per_m;
                    lim = l;
                    tool = Some(name.clone());
                }
            }
            // limits not known: the press's force (a real press's record), a real tool's load
            let mut unknown: Vec<String> = Vec::new();
            if pr.max_force.is_none() && pr.unchecked.iter().any(|u| u == "force") {
                unknown.push(format!("force of press {}", pr.id));
            }
            if lim.is_finite() {
                let mut c = Check::rule(Rule::Tonnage, value, lim, true);
                c.tool = tool;
                out.push(c);
            }
            if !unknown.is_empty() || !lim.is_finite() {
                if unknown.is_empty() {
                    unknown.push("every load limit".into());
                }
                let mut c = Check::not_checked(Rule::Tonnage, &unknown.join(", "));
                c.value = f;
                out.push(c);
            }
        }
        (out, travel, force)
    }

    /// The legs' straight runs (far, near) and the part's length along the bed.
    pub fn leg_geometry(&self) -> (f64, f64, f64) {
        self.legs
            .unwrap_or_else(|| (self.leg_run(true), self.leg_run(false), self.part_extent_y()))
    }

    /// The part's length along the bed (press y), exact for walls, enclosed for zones.
    pub fn part_extent_y(&self) -> f64 {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for pc in &self.pieces {
            let roots = self.roots(pc, Interval::point(0.0));
            for c in roots {
                // split u into eight to keep the zones' enclosures tight
                let n = 8;
                for i in 0..n {
                    let du = c.p[0].width() / n as f64;
                    let u = Interval::new(c.p[0].lo + du * i as f64, c.p[0].lo + du * (i + 1) as f64);
                    let mut cc = c;
                    cc.p[0] = u;
                    if !Self::may_hold(pc, &cc) {
                        continue;
                    }
                    let e = self.image(pc, cc.p, Phase::Static, Attach::Die);
                    lo = lo.min(e.y.lo);
                    hi = hi.max(e.y.hi);
                }
            }
        }
        hi - lo
    }

    /// The backgauge: the outermost stop on the gauged leg's side the fingers reach, clear of the
    /// rest of the part.
    pub fn gauge(&self) -> (Check, Option<Gauged>) {
        if self.tl.press.unchecked.iter().any(|u| u == "backgauge") {
            let mut c = Check::not_checked(Rule::BackgaugeReach, "backgauge ranges of the press");
            c.phase = Some(Phase::Static);
            return (c, None);
        }
        let bg = &self.tl.press.backgauge;
        let mut cands: Vec<Stop> = Vec::new();
        let die_pt =
            |pc: &Piece, u: f64, v: f64, w: f64| self.image(pc, [u, v, w, 0.0], Phase::Static, Attach::Die);
        for pc in &self.pieces {
            let Kind::Wall {
                w, region, to_param, ..
            } = &pc.kind
            else {
                continue;
            };
            let wall = &self.m.walls[*w];
            // side faces of free straight edges
            for (id, s) in &wall.free {
                let Seg::Line { a, b } = s.moved(to_param) else {
                    continue;
                };
                let d = b - a;
                let len = d.length();
                if len <= 1e-9 {
                    continue;
                }
                let nrm = Vec2::new(d.y, -d.x) * (1.0 / len);
                let corners = [
                    die_pt(pc, a.x, a.y, 0.0),
                    die_pt(pc, b.x, b.y, 0.0),
                    die_pt(pc, a.x, a.y, -self.t),
                    die_pt(pc, b.x, b.y, -self.t),
                ];
                let o = corners[0];
                let pn = die_pt(pc, a.x + nrm.x, a.y + nrm.y, 0.0);
                let n = [pn.x - o.x, pn.y - o.y, pn.z - o.z];
                if n[0] < 0.9998 {
                    continue;
                }
                let xs: Vec<f64> = corners.iter().map(|c| c.x).collect();
                if xs.iter().any(|x| (x - xs[0]).abs() > 1e-6) {
                    continue;
                }
                let ys = corners.iter().map(|c| c.y);
                let zs = corners.iter().map(|c| c.z);
                let span = |it: &mut dyn Iterator<Item = f64>| {
                    it.fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), v| {
                        (l.min(v), h.max(v))
                    })
                };
                cands.push((
                    xs[0],
                    span(&mut ys.into_iter()),
                    span(&mut zs.into_iter()),
                    PartRef {
                        strip: pc.strip,
                        surface: Surface::Side,
                        segment: Some(*id),
                    },
                ));
            }
            // a skin facing the backgauge (a flange standing at the back)
            let o = die_pt(pc, 0.0, 0.0, 0.0);
            let pz = die_pt(pc, 0.0, 0.0, 1.0);
            let n = [pz.x - o.x, pz.y - o.y, pz.z - o.z];
            for (sgn, w, surf) in [(1.0, 0.0, Surface::Top), (-1.0, -self.t, Surface::Bottom)] {
                if sgn * n[0] < 0.9998 {
                    continue;
                }
                let px = die_pt(pc, 1.0, 0.0, w);
                let py = die_pt(pc, 0.0, 1.0, w);
                let base = die_pt(pc, 0.0, 0.0, w);
                let ext = |k: usize| {
                    let e = |p: &V3<f64>| [p.x, p.y, p.z][k];
                    let dv = Vec2::new(e(&px) - e(&base), e(&py) - e(&base));
                    let (l, h) = region
                        .segs()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), s| {
                            let (a, b) = s.extent(dv);
                            (l.min(a), h.max(b))
                        });
                    (e(&base) + l, e(&base) + h)
                };
                cands.push((
                    base.x,
                    ext(1),
                    ext(2),
                    PartRef {
                        strip: pc.strip,
                        surface: surf,
                        segment: None,
                    },
                ));
            }
        }
        cands.retain(|c| c.0 > 0.0);
        cands.sort_by(|a, b| {
            b.0.total_cmp(&a.0)
                .then((b.1.1 - b.1.0).total_cmp(&(a.1.1 - a.1.0)))
        });
        let fw = bg.finger.width;
        let mut tried = None;
        for (x, (ya, yb), (za, zb), part) in &cands {
            if *x < bg.x.0 || *x > bg.x.1 {
                tried.get_or_insert((*x, *part));
                continue;
            }
            let r = za.clamp(bg.r.0, bg.r.1);
            if r >= zb - 1e-6 || r + bg.finger.height <= za + 1e-6 {
                tried.get_or_insert((*x, *part));
                continue;
            }
            // the fingers' Z travel, in the station's frame
            let (ya, yb) = match bg.z {
                Some((zl, zh)) => (ya.max(zl - self.oy + 0.5 * fw), yb.min(zh - self.oy - 0.5 * fw)),
                None => (*ya, *yb),
            };
            if yb < ya {
                tried.get_or_insert((*x, *part));
                continue;
            }
            let span = yb - ya;
            let fingers = if bg.fingers >= 2 && span >= 2.0 * fw + 4.0 * self.tl.clearance {
                vec![ya + 0.25 * span, yb - 0.25 * span]
            } else {
                vec![0.5 * (ya + yb)]
            };
            let g = Gauged {
                x: *x,
                r,
                fingers,
                part: *part,
            };
            // the fingers, backed off twice the clearance from their stop, clear of the part
            let mut tc = Vec::new();
            self.tool_checks(Phase::Static, Some(&g), &mut tc, true, &|e| {
                e.name.starts_with("backgauge")
            });
            let finger_hit = tc
                .iter()
                .any(|c| !c.pass && c.tool.as_deref().is_some_and(|t| t.starts_with("backgauge")));
            if finger_hit {
                tried.get_or_insert((*x, *part));
                continue;
            }
            let mut c = Check::rule(Rule::BackgaugeReach, *x, bg.x.1, true);
            c.lo = Some(bg.x.0);
            c.part = Some(*part);
            c.phase = Some(Phase::Static);
            c.tool = Some("backgauge".into());
            return (c, Some(g));
        }
        let mut c = Check::rule(
            Rule::BackgaugeReach,
            tried.map_or(f64::NAN, |t| t.0),
            bg.x.1,
            true,
        );
        c.lo = Some(bg.x.0);
        c.pass = false;
        c.part = tried.map(|t| t.1);
        c.phase = Some(Phase::Static);
        c.tool = Some("backgauge".into());
        (c, None)
    }
}

/// A place a backgauge finger may stop on: its x, its span along y and in z, and what it is.
type Stop = (f64, (f64, f64), (f64, f64), PartRef);

/// A tool section moved by (dx, dz).
fn shifted(r: &Region, dx: f64, dz: f64) -> Region {
    Region::of(
        r.loops
            .iter()
            .map(|l| l.iter().map(|s| s.shifted(dx, dz)).collect())
            .collect(),
    )
}

fn rect(x0: f64, z0: f64, x1: f64, z1: f64) -> Region {
    let pts = [(x0, z0), (x1, z0), (x1, z1), (x0, z1)];
    Region::of(vec![
        (0..4).map(|i| Seg::line(pts[i], pts[(i + 1) % 4])).collect(),
    ])
}

/// The least and greatest of f over [lo, hi], within 1e-6, by interval bisection; and where the
/// least falls.
/// Certified bounds on f over [lo, hi] by interval bisection: (a lower bound on its least value,
/// an upper bound on its greatest, where the least point found is, proven). Each interval is
/// settled once its enclosure lies within 1e-4 of the extremes found at points; the bounds are
/// the settled enclosures'. Not proven (false) when the split budget runs out first.
fn range_1d(
    fi: impl Fn(Interval) -> Interval,
    fp: impl Fn(f64) -> f64,
    lo: f64,
    hi: f64,
) -> (f64, f64, f64, bool) {
    let ends = [fp(lo), fp(hi)];
    let mut best_lo = (ends[0].min(ends[1]), if ends[0] <= ends[1] { lo } else { hi });
    let mut best_hi = ends[0].max(ends[1]);
    let (mut cert_lo, mut cert_hi) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut stack = vec![Interval::new(lo, hi)];
    let mut n = 0;
    while let Some(i) = stack.pop() {
        n += 1;
        if n > 20_000 {
            return (best_lo.0, best_hi, best_lo.1, false);
        }
        let e = fi(i);
        let m = fp(i.mid());
        if m < best_lo.0 {
            best_lo = (m, i.mid());
        }
        best_hi = best_hi.max(m);
        if (e.lo >= best_lo.0 - 1e-4 && e.hi <= best_hi + 1e-4) || i.width() < 1e-9 {
            cert_lo = cert_lo.min(e.lo);
            cert_hi = cert_hi.max(e.hi);
            continue;
        }
        let (a, b) = i.split();
        stack.push(a);
        stack.push(b);
    }
    (cert_lo.min(best_lo.0), cert_hi.max(best_hi), best_lo.1, true)
}

struct ToolPb<'a, 'b> {
    ctx: &'b StepCtx<'a>,
    pc: &'b Piece,
    el: &'b Element,
    phase: Phase,
}

impl Problem for ToolPb<'_, '_> {
    fn may_hold(&self, c: &Cell) -> bool {
        StepCtx::may_hold(self.pc, c) && !self.ctx.on_working(self.pc, c, self.el.work)
    }
    fn enclose(&self, c: &Cell) -> [Interval; 3] {
        let e = self.ctx.image(self.pc, c.p, self.phase, self.el.attach);
        [e.x, e.y, e.z]
    }
    fn witness(&self, c: &Cell) -> Option<[f64; 4]> {
        let q = StepCtx::witness(self.pc, c)?;
        let pt = Cell {
            p: q.map(Interval::point),
            tag: 0,
        };
        if self.ctx.on_working(self.pc, &pt, self.el.work) {
            return None;
        }
        Some(q)
    }
    fn image(&self, q: [f64; 4]) -> [f64; 3] {
        self.ctx.image(self.pc, q, self.phase, self.el.attach).arr()
    }
    fn sd(&self, p: [f64; 3]) -> f64 {
        sd_prism(&self.el.sec, self.el.y.0, self.el.y.1, p)
    }
    /// The tool is a prism along y: the bound in (x, z) and the exact least distance in y.
    fn lower(&self, e: &[Interval; 3]) -> (f64, f64) {
        let (y0, y1) = self.el.y;
        let hd2 = (0.25 * (e[0].width().powi(2) + e[2].width().powi(2))).sqrt() + bb::SLACK;
        let s2 = self.el.sec.sd(Point2::new(e[0].mid(), e[2].mid())) - hd2;
        // dy(y) = max(y0 - y, y - y1) is least at the range's middle
        let ym = 0.5 * (y0 + y1);
        let yc = ym.clamp(e[1].lo, e[1].hi);
        let dy = (y0 - yc).max(yc - y1);
        let lb = if s2 <= 0.0 && dy <= 0.0 {
            s2.max(dy)
        } else {
            (s2.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt()
        };
        // past the tool's ends the cell's length along y counts too
        let size = if e[1].lo >= y0 && e[1].hi <= y1 {
            hd2
        } else {
            (hd2 * hd2 + 0.25 * e[1].width().powi(2)).sqrt()
        };
        (lb, size)
    }
    fn weights(&self) -> [f64; 3] {
        [1.0, 0.05, 1.0]
    }
}

/// A moving piece against a piece of the other side, in that side's frame.
struct SelfPb<'a, 'b> {
    ctx: &'b StepCtx<'a>,
    mv: &'b Piece,
    st: &'b Piece,
    frame: Side,
}

impl SelfPb<'_, '_> {
    fn img<T: Sc>(&self, q: [T; 4]) -> V3<T> {
        let c = self.ctx;
        let p = c.side_point(self.mv, q[0], q[1], q[2], q[3]);
        match (self.mv.side, self.frame) {
            (Side::B, Side::A) => c.bj.end_apply(q[3], p),
            (Side::J, Side::B) => {
                let (e, tx, nz) = c.bj.end(q[3]);
                let d = p.minus(e);
                V3::new(d.dot(tx), d.y, d.dot(nz))
            }
            _ => p,
        }
    }
}

impl Problem for SelfPb<'_, '_> {
    fn may_hold(&self, c: &Cell) -> bool {
        StepCtx::may_hold(self.mv, c)
    }
    fn enclose(&self, c: &Cell) -> [Interval; 3] {
        let e = self.img(c.p);
        [e.x, e.y, e.z]
    }
    fn witness(&self, c: &Cell) -> Option<[f64; 4]> {
        StepCtx::witness(self.mv, c)
    }
    fn image(&self, q: [f64; 4]) -> [f64; 3] {
        self.img(q).arr()
    }
    fn sd(&self, p: [f64; 3]) -> f64 {
        piece_sd(self.ctx, self.st, p).0
    }
    fn sd_bounds(&self, p: [f64; 3]) -> (f64, f64) {
        piece_sd(self.ctx, self.st, p)
    }
}

/// Bounds on the signed distance from a side-frame point to a fixed piece: (lower, upper).
fn piece_sd(ctx: &StepCtx, pc: &Piece, p: [f64; 3]) -> (f64, f64) {
    let t = ctx.t;
    let q = pc.inv.apply(V3::new(p[0], p[1], p[2]));
    let slab = |s2: f64, w: f64| {
        let dw = (-t - w).max(w);
        if s2 <= 0.0 && dw <= 0.0 {
            s2.max(dw)
        } else {
            (s2.max(0.0).powi(2) + dw.max(0.0).powi(2)).sqrt()
        }
    };
    let exact = |d: f64| (d, d);
    match &pc.kind {
        Kind::Wall { region, .. } => exact(slab(region.sd(Point2::new(q.x, q.y)), q.z)),
        Kind::Zone { k, phi, region, .. } => {
            let b = &ctx.m.bends[*k];
            if *phi <= 0.0 {
                return exact(slab(region.sd(Point2::new(q.x, q.y)), q.z));
            }
            zone_sd(b, *phi, t, region, [q.x, q.y, q.z])
        }
        Kind::ZoneJ { .. } => (f64::INFINITY, f64::INFINITY),
    }
}

/// Bounds (lower, upper) on the signed distance to zone b at turn phi (> 0), in its
/// coordinates. Against the zone's whole slot the distance is exact. Where the pattern takes some
/// of the slot away (a relief, a hole), a point's angle and height give its developed (u, y); two
/// points of the slot solid at radii >= r_in and developed distance d are at least
/// (2/pi) min(1, r_in / rn) d apart, and at most max(1, r_out / rn) d.
pub fn zone_sd(b: &BendM, phi: f64, t: f64, region: &Region, p: [f64; 3]) -> (f64, f64) {
    let whole = zone_sd_slot(b, phi, t, p);
    if region.bound.len() == 4 {
        // nothing taken away: the slot is the zone
        return (whole, whole);
    }
    let rho = b.ba / phi + b.c0;
    let s = b.sigma;
    let (dx, dz) = (p[0], p[2] - s * rho);
    let psi = dmath::atan2(dx, -s * dz);
    let rn = b.ba / phi;
    let (ri, ro) = if s > 0.0 { (rho, rho + t) } else { (rho - t, rho) };
    if whole > 0.0 || !(0.0..=phi).contains(&psi) {
        // outside the slot solid: at least as far as from the slot; no point named nearer
        return (whole, f64::INFINITY);
    }
    let q = Point2::new(psi * rn, p[1]);
    let d2 = region.sd(q);
    if d2 > 0.0 {
        // in a hole of the zone: clear of it by at least this
        return (d2 * (2.0 / dmath::PI) * (ri / rn).min(1.0), f64::INFINITY);
    }
    // in the material: no deeper than the slot says, and within the stretched developed depth
    (whole, whole.max(d2 * (ro / rn).max(1.0)))
}

fn zone_sd_slot(b: &BendM, phi: f64, t: f64, p: [f64; 3]) -> f64 {
    let rho = b.ba / phi + b.c0;
    let s = b.sigma;
    // centre axis at (0, s rho) in (x, z); r grows into the material with -s w
    let (dx, dz) = (p[0], p[2] - s * rho);
    let (ri, ro) = if s > 0.0 { (rho, rho + t) } else { (rho - t, rho) };
    let r = (dx * dx + dz * dz).sqrt();
    // angle from the zone's start direction (straight back from the centre to the skin at u = 0)
    let psi = dmath::atan2(dx, -s * dz);
    let y = p[1];
    let d1 = |v: f64, lo: f64, hi: f64| (lo - v).max(v - hi);
    if (0.0..=phi).contains(&psi) {
        let dr = d1(r, ri, ro);
        let dy = d1(y, 0.0, b.len);
        if dr <= 0.0 && dy <= 0.0 {
            let ang = (r * dmath::sin(psi.min(dmath::FRAC_PI_2)))
                .min(r * dmath::sin((phi - psi).min(dmath::FRAC_PI_2)));
            return dr.max(dy).max(-ang);
        }
        return (dr.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    }
    // past the ends: the nearer end cap (a meridian rectangle)
    let cap = |a: f64| {
        let (sa, ca) = dmath::sin_cos(a);
        let (ex, ez) = (sa, -s * ca);
        let along = dx * ex + dz * ez;
        let off = dx * ez - dz * ex;
        let da = d1(along, ri, ro).max(0.0);
        let dy = d1(y, 0.0, b.len).max(0.0);
        (da * da + dy * dy + off * off).sqrt()
    };
    cap(0.0).min(cap(phi))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_finds_extremes() {
        let (lo, hi, at, ok) = range_1d(|i| i.sin(), dmath::sin, 0.0, 3.0);
        assert!(
            ok && (hi - 1.0).abs() < 1e-4 && lo <= 0.0 && lo > -1e-4 && at == 0.0,
            "{lo} {hi}"
        );
        // sin really reaches 1: the upper bound is at least it
        assert!(hi >= 1.0);
    }
}

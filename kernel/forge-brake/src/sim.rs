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
use crate::tooling::{Die, Punch, Role, Tooling, cover_within};
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
    /// The operator's move into the step (lift, pull out, turn, push in, set down) is not
    /// proven clear of the press (warning).
    Handling,
    /// The sheet is thicker than the tool's maker rates it for (its published max thickness).
    ToolRating,
    /// A hem's pre-bend passes, but no tool on the press flattens it (`Role::flattens`), or none
    /// that does passes the flattening's checks.
    NoFlattening,
    /// An offset: the part's offset against the inserts' Z.
    Offset,
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
            Rule::Handling => "handling",
            Rule::ToolRating => "tool_rating",
            Rule::NoFlattening => "no_flattening",
            Rule::Offset => "offset",
        }
    }

    pub const ALL: [Rule; 20] = [
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
        Rule::Handling,
        Rule::ToolRating,
        Rule::NoFlattening,
        Rule::Offset,
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
    /// A distance failing against a body whose envelope is an estimate erring large (a real
    /// press's beams, with no published geometry): a refusal, not a proven collision.
    pub estimate: bool,
    /// Near (David): a distance proven at least `bound` (= `value`), within this resolution of
    /// the limit but not proven at or past it. Not a pass (a pass is proven at the limit); a
    /// mild warning, so it does not block a plan.
    pub near: Option<f64>,
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

    pub fn rule(rule: Rule, value: f64, limit: f64, at_most: bool) -> Check {
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
            estimate: false,
            near: None,
        }
    }

    /// A distance proven at least `lower`, within `res` of `limit` but not proven at it: Near.
    pub fn near(rule: Rule, lower: f64, limit: f64, res: f64) -> Check {
        let mut c = Check::rule(rule, lower, limit, false);
        c.pass = false;
        c.severity = Severity::Warning;
        c.certified = false;
        c.bound = Some(lower);
        c.near = Some(res);
        c
    }
}

/// Every wall's and zone's region within the flat pattern, in its own coordinates: a function
/// of the model alone, made once and shared by every step a search simulates.
#[derive(Clone, Debug)]
pub struct Regions {
    pub walls: Vec<Region>,
    pub zones: Vec<Region>,
}

impl Regions {
    pub fn of(m: &Model) -> Regions {
        Regions {
            walls: m.walls.iter().map(|w| w.region(&m.pattern)).collect(),
            zones: m.bends.iter().map(|b| b.region(&m.pattern)).collect(),
        }
    }
}

/// Where a step's part and a tool come within the clearance (or into each other): the part's
/// faces and the cells of them (parameters u, v, w and the motion s), points on both bodies, how
/// near or how deep, and when in the motion. The cells are never fewer than the true contact
/// (`crate::contact`): `proven` cells hold a real point under the limit, the rest are bounded.
#[derive(Clone, Debug, PartialEq)]
pub struct Contact {
    pub rule: Rule,
    pub phase: Phase,
    pub severity: Severity,
    /// The tool's body (`punch:ID/body`, `die:ID`, `ram`, `bed`, `backgauge:finger1`,
    /// `station2:punch:ID/tip`, ..).
    pub tool: String,
    /// The tool's envelope is an estimate (errs large).
    pub estimate: bool,
    pub limit: f64,
    /// The least distance a real point of the region is proven to have (negative: that deep
    /// into the tool); None when no point is proven under the limit (a Near, an undecided).
    pub depth: Option<f64>,
    /// No point of the region is nearer than this.
    pub lower: f64,
    /// A Near: every point proven at least this far (within the check's resolution of the
    /// limit, not at it).
    pub near: Option<f64>,
    /// Every cell settled within the extraction's budget (else what is left was kept whole).
    pub complete: bool,
    pub faces: Vec<ContactFace>,
    /// Part against part (`self_collision`): the other piece's faces the region reaches (the
    /// `tool` is then `part`); empty against a tool.
    pub others: Vec<PartRef>,
    /// The motion's span over the region: the bend's turn (radians), or the ram's height (mm,
    /// approach and return).
    pub span: (f64, f64),
    /// Press frame, mm: the region's points on the part (a proven point where a cell holds one,
    /// else the cell's centre) and the tool's nearest points to them.
    pub points: Vec<[f64; 3]>,
    pub tool_points: Vec<[f64; 3]>,
    /// Press frame, mm: a box holding every cell's every point through its stretch of motion.
    pub bbox: ([f64; 3], [f64; 3]),
}

/// One face of the part in a contact: its cells (u, v, w, s) and how many hold a proven point.
#[derive(Clone, Debug, PartialEq)]
pub struct ContactFace {
    pub part: PartRef,
    pub cells: Vec<[Interval; 4]>,
    pub proven: usize,
}

/// The cells an extraction may examine per piece and tool (a contact's region is small; what is
/// left past this is kept whole).
const REGION_BUDGET: usize = 20_000;

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
    /// What the stroke does to the bend: all of it in the air, or one of a hem's two strokes.
    pub stroke: Stroke,
    pub joint: JointId,
    /// The turn and the overbend (radians).
    pub angle: f64,
    pub overbend: f64,
    pub setup: Setup,
    pub punch: String,
    pub die: String,
    pub punch_length: Option<f64>,
    pub die_length: Option<f64>,
    /// The station it works at (index into the tooling's stations), when stations are set.
    pub station: Option<usize>,
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

/// A hem's pre-bend, as included angle: WILA, "The first step when creating a hem is to make a
/// 30° bend" (https://www.wilatooling.com/en-us/applications/hemming/, read 2026-11-25).
pub const HEM_PRE_BEND: f64 = 30.0 * dmath::PI / 180.0;

/// The turn a bend's first stroke makes: the whole of it, or a hem's pre-bend.
pub fn first_turn(b: &BendM) -> f64 {
    if b.hem.is_some() {
        // a nanoradian short: a 30 degree tool is not refused by the subtraction's rounding
        (dmath::PI - HEM_PRE_BEND - 1e-9).min(b.theta)
    } else {
        b.theta
    }
}

/// What a step's stroke does to its bend.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Stroke {
    /// An air bend through the whole turn (and its overbend).
    #[default]
    Air,
    /// A hem's first stroke: an air bend to the pre-bend's turn `to`, short of the hem's.
    PreBend { to: f64 },
    /// A hem's second: flattened from the pre-bend's turn `from` to the hem's
    /// (`StepCtx::flattening`).
    Flatten { from: f64 },
    /// An offset: the bend and its neighbour `k` together, in one stroke (`StepCtx::set_offset`).
    Offset { k: usize },
}

impl Stroke {
    pub fn word(self) -> &'static str {
        match self {
            Stroke::Air => "bend",
            Stroke::PreBend { .. } => "pre-bend",
            Stroke::Flatten { .. } => "flatten",
            Stroke::Offset { .. } => "offset",
        }
    }
}

impl Step {
    /// The bends this step leaves made, marked in `done`: its own, and an offset's second.
    pub fn mark(&self, done: &mut [bool]) {
        done[self.bend] = true;
        if let Stroke::Offset { k } = self.stroke {
            done[k] = true;
        }
    }

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
    /// An offset's far side: beyond its second bend k, in zone k's end coordinates (k turning
    /// with j).
    C,
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
    /// An offset's second zone, turning with j: zone k's coordinates carried by `iso` into j's
    /// end coordinates.
    ZoneK { k: usize, iso: Iso3, region: Region },
}

impl Piece {
    pub fn region(&self) -> &Region {
        match &self.kind {
            Kind::Wall { region, .. }
            | Kind::Zone { region, .. }
            | Kind::ZoneJ { region }
            | Kind::ZoneK { region, .. } => region,
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
    /// The working station (index into the tooling's stations), when stations are set.
    pub station: Option<usize>,
    /// `contact_range` already made for this bend and die (by a search).
    pub contact_kept: Option<(f64, f64, f64, bool)>,
    /// Verdicts kept across steps: a piece's place depends only on the bends on its path to
    /// the moving one, so its verdict against a tool holds wherever those are bent alike.
    pub memo: Option<std::rc::Rc<std::cell::RefCell<Memo>>>,
    /// The legs' straight runs (far, near) and the part's length along the bed, if already known
    /// for these bends done: they read no tool.
    pub legs: Option<(f64, f64, f64)>,
    /// A hem's flattening stroke, in place of an air bend (`flattening`).
    pub flatten: Option<Flat>,
    /// An offset's stroke: bend j and its neighbour k together (`set_offset`).
    pub offset: Option<Off>,
}

/// An offset's stroke: bends j and k (k off j's wall b, the web) turned together by the same
/// turn, opposite ways, the outer walls kept level: wall a on the die's high face, the far wall
/// pushed down by the punch's step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Off {
    pub k: usize,
    /// Zone k's coordinates into j's end coordinates (at the stroke's every turn: the web does
    /// not bend).
    pub zk: Iso3,
    /// Where across the bed (zone j's x) the tools' step is: the web's middle.
    pub x_ref: f64,
}

/// An offset's part against its inserts' Z: a hundredth either way (mm).
pub const OFFSET_TOL: f64 = 0.01;

/// Flattening: how far below the flat die's top a cell of the base's side may be enclosed and
/// still rest on it (mm): an enclosure's width where a zone leaves the plane tangentially.
const FLAT_REST: f64 = 1e-3;

/// A hem's flattening stroke: the part's base (zone j's wall a side) flat on a flat die, the leg
/// closed from the pre-bend's turn to the hem's by a flat punch face coming down on the part's
/// highest point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flat {
    /// The pre-bend's turn, where the stroke starts.
    pub from: f64,
    /// The leg's reach from the zone's far tangent line (zone-end x').
    pub reach: f64,
    /// Where across the bed (zone x) the tools' centre line is: the hem's middle.
    pub x_ref: f64,
}

/// A distance verdict's key: the piece (strip kind and id, the done bends on its path), the tool
/// element (with the punch it rides on, for what rides the ram: the ram stands at its height),
/// the phase, the orientation, the moving bend and its overbent turn, the die (where it places
/// the tool against the part), whether it was only decided or measured, and the search's own question,
/// exactly: its limit, its resolution and its start cells (each interval's ends and the tag, as
/// bits), so a kept verdict only ever answers the search it was found by (FALLBACK's review).
pub type MemoKey = (
    u8,
    u32,
    u64,
    String,
    Phase,
    bool,
    usize,
    u64,
    usize,
    bool,
    Vec<u64>,
);

/// The search's question as exact bits: limit, resolution, then each start cell.
fn question(limit: f64, res: f64, roots: &[Cell]) -> Vec<u64> {
    let mut q = vec![limit.to_bits(), res.to_bits()];
    for c in roots {
        for i in &c.p {
            q.push(i.lo.to_bits());
            q.push(i.hi.to_bits());
        }
        q.push(c.tag as u64);
    }
    q
}
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
        StepCtx::with_regions(m, tl, done, j, setup, overbend, &Regions::of(m))
    }

    /// `new`, with the model's regions already made (a search makes them once).
    pub fn with_regions(
        m: &'a Model,
        tl: &'a Tooling,
        done: &[bool],
        j: usize,
        setup: Setup,
        overbend: f64,
        rg: &Regions,
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
            let region = rg.walls[wi].moved(&to_param);
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
            let region = rg.zones[k].clone();
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
        let own = (
            cover_within(&punch.segments, need, room).map(|c| c.0),
            cover_within(&die.segments, need, room).map(|c| c.0),
        );
        let station_ix = if tl.stations.is_empty() {
            None
        } else {
            let r = |c: Option<f64>| c.map_or(room, |c| c.max(room));
            tl.station_for(setup.punch, setup.die, need, (r(own.0), r(own.1)))
        };
        let station = station_ix.map(|k| &tl.stations[k]);
        let lengths = match station {
            Some(st) => (Some(st.punch_length), Some(st.die_length)),
            None => own,
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
            station: station_ix,
            contact_kept: None,
            memo: None,
            legs: None,
            flatten: None,
            offset: None,
        }
    }

    /// A piece's point in its side frame, the moving bend at turn φ.
    pub fn side_point<T: Sc>(&self, pc: &Piece, u: T, v: T, w: T, phi: T) -> V3<T> {
        match &pc.kind {
            Kind::Wall { iso, .. } => iso.apply(V3::new(u, v, w)),
            Kind::Zone { k, phi: pk, iso, .. } => iso.apply(self.m.bends[*k].fold(u, v, w, T::c(*pk))),
            Kind::ZoneJ { .. } => self.bj.fold(u, v, w, phi),
            Kind::ZoneK { k, iso, .. } => iso.apply(self.m.bends[*k].fold(u, v, w, phi)),
        }
    }

    /// A side-frame point to the press, relative to the punch tip.
    pub fn rel<T: Sc>(&self, q: V3<T>, side: Side, phi: T) -> V3<T> {
        let b = self.bj;
        let p = match side {
            Side::B => b.end_apply(phi, q),
            Side::C => {
                let o = self.offset.as_ref().expect("a far side only in an offset");
                b.end_apply(phi, o.zk.apply(self.m.bends[o.k].end_apply(phi, q)))
            }
            _ => q,
        };
        if let Some(o) = &self.offset {
            // level: wall a on the die's high face, its z flipped so the web goes down as the
            // turn grows (y with it: a turn, not a mirror)
            let x = (p.x + (-o.x_ref)) * self.rot;
            let y = (p.y + (-0.5 * b.len)) * (-self.rot * b.sigma);
            let z = p.z * (-b.sigma) + self.off_lift() - self.ztip(phi);
            return V3::new(x, y, z);
        }
        if let Some(f) = &self.flatten {
            // zone coordinates keep wall a flat at every turn: it lies on the die, its outside
            // skin down (z flipped with sigma, and y with it, a turn, not a mirror)
            let x = (p.x + (-f.x_ref)) * self.rot;
            let y = (p.y + (-0.5 * b.len)) * (self.rot * b.sigma);
            let z = p.z * b.sigma + self.flat_off() - self.ztip(phi);
            return V3::new(x, y, z);
        }
        let pm = b.fold(T::c(0.5 * b.ba), T::c(0.5 * b.len), T::c(b.w_inside(self.t)), phi);
        let h = phi * 0.5;
        let (s, c) = (h.sin(), h.cos());
        let d = p.minus(pm);
        let x = (d.x * c + d.z * (s * b.sigma)) * self.rot;
        let y = d.y * (self.rot * b.sigma);
        let z = d.z * (c * b.sigma) - d.x * s;
        V3::new(x, y, z)
    }

    /// The step worked at station `k` (a lock: a bend the user set to a station), in place of
    /// the one its tools and stacks would pick: that station's place along the bed and its
    /// stacks' lengths.
    pub fn at_station(mut self, k: usize) -> Self {
        let st = &self.tl.stations[k];
        self.station = Some(k);
        self.oy = st.y;
        self.lengths = (Some(st.punch_length), Some(st.die_length));
        self
    }

    /// This step as a hem's flattening stroke from the pre-bend's turn `from` (at least a
    /// quarter turn) to the hem's own: the base flat on the die, the leg closed by a flat face.
    /// None when the leg has walls beyond it (not simulated).
    pub fn flattening(mut self, from: f64) -> Option<Self> {
        self.set_flattening(from).then_some(self)
    }

    /// `flattening` in place: false (nothing changed) where it does not model the stroke.
    pub fn set_flattening(&mut self, from: f64) -> bool {
        let b = self.bj;
        // past half a turn (a teardrop, a roll) the leg drops below the fold's apex: a flat face
        // coming down meets the apex alone and cannot close it further (the tool that closes a
        // teardrop is not published: research/bendsim/tools/hems-data.md)
        if b.theta > dmath::PI + 1e-9
            || from < 0.5 * dmath::PI
            || self
                .pieces
                .iter()
                .any(|p| p.side == Side::B && !matches!(p.kind, Kind::Wall { w, .. } if w == b.b))
        {
            return false;
        }
        let Some(leg) = self
            .pieces
            .iter()
            .find(|p| matches!(p.kind, Kind::Wall { w, .. } if w == b.b))
        else {
            return false;
        };
        let reach = leg.dom[0].hi;
        self.theta = b.theta;
        self.phimax = b.theta;
        self.flatten = Some(Flat {
            from,
            reach,
            x_ref: -0.5 * reach,
        });
        true
    }

    /// A pre-bend: the air bend stops at turn `to` (and its overbend), short of the bend's own.
    pub fn set_pre_bend(&mut self, to: f64, overbend: f64) {
        self.theta = to;
        self.phimax = (to + overbend).min(dmath::PI - 1e-6);
    }

    /// A planned step's bend after `done`, with its tools and its stroke, at the bed's centre
    /// (`of_step` puts it at its station).
    pub fn of_stroke(m: &'a Model, tl: &'a Tooling, done: &[bool], st: &Step) -> StepCtx<'a> {
        let mut ctx = StepCtx::new(m, tl, done, st.bend, st.setup, st.overbend);
        match st.stroke {
            Stroke::Air => {}
            Stroke::PreBend { to } => ctx.set_pre_bend(to, st.overbend),
            Stroke::Flatten { from } => {
                ctx.set_flattening(from);
            }
            Stroke::Offset { k } => {
                ctx.set_offset(k);
            }
        }
        ctx
    }

    /// The context a planned step was simulated in: its bend after `done`, its tools, its stroke,
    /// and the station it was worked at.
    pub fn of_step(m: &'a Model, tl: &'a Tooling, done: &[bool], st: &Step) -> StepCtx<'a> {
        let ctx = StepCtx::of_stroke(m, tl, done, st);
        match st.station {
            Some(i) if ctx.station != Some(i) && i < tl.stations.len() => ctx.at_station(i),
            _ => ctx,
        }
    }

    /// Where the stroke starts: 0 for an air bend, the pre-bend's turn for a flattening.
    pub fn phi0(&self) -> f64 {
        self.flatten.map_or(0.0, |f| f.from)
    }

    /// Flattening: the height of wall a's outside skin over its zone coordinates' z (sigma
    /// applied), so that skin lies on the die top.
    fn flat_off(&self) -> f64 {
        if self.bj.sigma > 0.0 { self.t } else { 0.0 }
    }

    /// Flattening: the part's highest point at turn phi (a quarter turn or more), where the flat
    /// face meets it. The leg is flat: its highest point is a corner at its far end, on one skin;
    /// the fold's outside skin is highest at its apex (a quarter turn in). So the greatest of the
    /// three is the part's top; a cell's interval of turns gets an enclosure of it.
    fn flat_top<T: Sc>(&self, f: &Flat, phi: T) -> T {
        let b = self.bj;
        let (sg, off) = (b.sigma, self.flat_off());
        let w_out = if sg > 0.0 { -self.t } else { 0.0 };
        let z0 = T::c(0.0);
        let apex = b.fold(phi.recip() * (b.ba * 0.5 * dmath::PI), z0, T::c(w_out), phi);
        let e0 = b.end_apply(phi, V3::new(T::c(f.reach), z0, z0));
        let e1 = b.end_apply(phi, V3::new(T::c(f.reach), z0, T::c(-self.t)));
        (apex.z * sg).max(e0.z * sg).max(e1.z * sg) + off
    }

    /// Offset: wall a's lift so its die side lies at z = 0.
    fn off_lift(&self) -> f64 {
        if self.bj.sigma > 0.0 { 0.0 } else { self.t }
    }

    /// Offset: the punch's lower face, on the far wall's upper skin at turn phi (the wall is
    /// level: one point of it holds its height). It comes down as the turn grows.
    fn off_top<T: Sc>(&self, o: &Off, phi: T) -> T {
        let b = self.bj;
        let bk = &self.m.bends[o.k];
        let z0 = T::c(0.0);
        let at = |w: f64| {
            let q = bk.end_apply(phi, V3::new(z0, z0, T::c(w)));
            b.end_apply(phi, o.zk.apply(q)).z * (-b.sigma)
        };
        at(0.0).max(at(-self.t)) + self.off_lift()
    }

    /// This step as an offset: bends j and k (k off j's web, the same turn the other way, on a
    /// parallel line, k not yet bent, nothing else off the web) in one stroke. False, nothing
    /// changed, where the part is not such a pair.
    pub fn set_offset(&mut self, k: usize) -> bool {
        let (j, bj) = (self.j, self.bj);
        let m = self.m;
        let bk = &m.bends[k];
        if k == j
            || self.done[k]
            || bk.a != bj.b
            || bk.sigma == bj.sigma
            || (bk.theta - bj.theta).abs() > 1e-9
            || m.kids[bj.b].iter().any(|&x| x != k)
        {
            return false;
        }
        let Some(zi) = self
            .pieces
            .iter()
            .position(|p| matches!(p.kind, Kind::Zone { k: x, .. } if x == k))
        else {
            return false;
        };
        let Kind::Zone { iso: zk, region, .. } = self.pieces[zi].kind.clone() else {
            return false;
        };
        // parallel lines: zone k's y axis along j's
        let ey = zk
            .apply(V3::new(0.0, 1.0, 0.0))
            .minus(zk.apply(V3::new(0.0, 0.0, 0.0)));
        if (ey.y.abs() - 1.0).abs() > 1e-9 {
            return false;
        }
        let ek = zk.then(&Iso3::shift(bk.ba));
        let ek_inv = ek.inverse();
        for (i, pc) in self.pieces.iter_mut().enumerate() {
            if i == zi {
                pc.kind = Kind::ZoneK {
                    k,
                    iso: zk,
                    region: region.clone(),
                };
                pc.inv = zk.inverse();
                continue;
            }
            let beyond = match pc.kind {
                Kind::Wall { w, .. } => m.beyond(k, w),
                Kind::Zone { k: x, .. } => m.beyond(k, m.bends[x].b),
                _ => false,
            };
            if !beyond {
                continue;
            }
            match &mut pc.kind {
                Kind::Wall { iso, .. } | Kind::Zone { iso, .. } => {
                    *iso = ek_inv.then(iso);
                    pc.inv = iso.inverse();
                }
                _ => {}
            }
            pc.side = Side::C;
        }
        // the web's middle across the bed, at the start (zone j's coordinates)
        let far = zk.apply(V3::new(bk.ba, 0.0, 0.0));
        self.offset = Some(Off {
            k,
            zk,
            x_ref: 0.5 * (bj.ba + far.x),
        });
        true
    }

    /// The shoulder's contact on the leg through the motion: its least and farthest place, where
    /// the least is, and whether both are certified. It reads the bend, the die and the overbend
    /// only (a search keeps it by bend and die: `contact_kept`).
    pub fn contact_range(&self) -> (f64, f64, f64, bool) {
        if let Some(c) = self.contact_kept {
            return c;
        }
        range_1d(|i| self.contact(i), |x| self.contact(x), 0.0, self.phimax)
    }

    /// The punch tip's height above the die top at turn φ.
    pub fn ztip<T: Sc>(&self, phi: T) -> T {
        let b = self.bj;
        if let Some(f) = &self.flatten {
            return self.flat_top(f, phi);
        }
        if let Some(o) = &self.offset {
            return self.off_top(o, phi);
        }
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
            Phase::Static => (T::c(self.phi0()), None),
            Phase::Bend => (q[3], None),
            Phase::Approach => (T::c(self.phi0()), Some(q[3])),
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
        if let Some(o) = &self.offset {
            // the offset itself between the inserts' faces: its outer walls, the web and both
            // zones (the stroke's own contact; its rules are the offset's)
            let (b, bk) = (self.bj, &self.m.bends[o.k]);
            return match pc.kind {
                Kind::ZoneJ { .. } | Kind::ZoneK { .. } => true,
                Kind::Wall { w, .. } => w == b.a || w == b.b || w == bk.b,
                _ => false,
            };
        }
        if self.flatten.is_some() {
            // the hem itself between the faces: the base on the die, the fold and the leg under
            // the punch face (the stroke's own contact; its rules are the flattening's)
            let b = self.bj;
            let own = match pc.kind {
                Kind::ZoneJ { .. } => true,
                Kind::Wall { w, .. } => w == b.a || w == b.b,
                _ => false,
            };
            if own {
                return true;
            }
            // the rest of the base's side resting on the die's flat top: a cell wholly on or above
            // its plane cannot enter the die (the base's side does not move in a flattening)
            if kind == Work::Die && pc.side == Side::A {
                let q = self.side_point(pc, c.p[0], c.p[1], c.p[2], Interval::point(self.phi0()));
                let z = q.z * b.sigma + self.flat_off();
                // a zone rounding up off the base touches the plane at its tangent line: a micron
                // below it, from the enclosure's width there, is resting, not entering
                return z.lo >= -FLAT_REST;
            }
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
            Side::C => return false,
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
            if Some(k) == self.station {
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
            Phase::Approach => Interval::new(self.ztip(self.phi0()), self.z_tdc.max(self.ztip(self.phi0()))),
            Phase::Bend => Interval::new(self.phi0(), self.phimax),
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
        // the least proven lower bound of all, and against what: under the limit, a Near
        let mut least = (f64::INFINITY, String::new());
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
                let t0 = trace_clock();
                let key: Option<MemoKey> = (!el.name.starts_with("backgauge")).then(|| {
                    let (kind, id) = match pc.strip {
                        Strip::Wall(w) => (0u8, w.0),
                        Strip::Bend(b) => (1u8, b.0),
                    };
                    // the die only where it places the tool against the part: what stands on it
                    // (image's to_die, by ztip, which reads its width and shoulder radius), and
                    // the punch lifted from it (approach, return). Bending (and static), the punch
                    // and what rides it see the part as rel(side_point(..)) from the punch tip,
                    // which read the bend, the thickness and the orientation, never the die
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
                        // what rides the ram stands by this step's punch's height (the ram, the
                        // other stations' punches): kept apart by the punch
                        if self.tl.stations.is_empty() && el.attach == Attach::Punch {
                            format!("{}@punch{}", el.name, self.punch_index)
                        } else if self.tl.stations.is_empty() {
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
                        question(limit, res, &roots),
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
                        t0.map(|t| t.elapsed()),
                        v
                    );
                }
                let (hit, value, q, at) = match v {
                    // a near pair is proven to its bound like a clear one: the summary is near
                    // when the least bound of all is under the limit
                    Verdict::Clear { lower, upper } | Verdict::Near { lower, upper } => {
                        if upper < lowest.1 {
                            lowest = (lower, upper, el.name.clone());
                        }
                        lowest.0 = lowest.0.min(lower);
                        if lower < least.0 {
                            least = (lower, el.name.clone());
                        }
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
                    if self
                        .tl
                        .press
                        .estimated
                        .iter()
                        .any(|e| el.name.starts_with(e.as_str()))
                    {
                        c.estimate = true;
                        c.certified = false;
                    }
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
                // every distance proven within the resolution of the limit, one not at it: Near
                None if k == slot(main) && least.0 < limit => {
                    let mut c = Check::near(main, least.0, limit, res);
                    c.tool = Some(least.1.clone());
                    c.phase = Some(phase);
                    out.push(c);
                }
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
                    _ => self.ztip(self.phi0()),
                };
                [a[0], a[1], a[2] + h]
            }
        }
    }

    /// The motion's interval in a phase (as the checks search it).
    fn phase_span(&self, phase: Phase) -> Interval {
        match phase {
            Phase::Static => Interval::point(0.0),
            Phase::Approach => Interval::new(self.ztip(self.phi0()), self.z_tdc.max(self.ztip(self.phi0()))),
            Phase::Bend => Interval::new(self.phi0(), self.phimax),
            Phase::Return => Interval::new(self.ztip(self.theta), self.z_tdc.max(self.ztip(self.theta))),
        }
    }

    /// Every contact in a phase: each piece against each tool not proven clear of the clearance
    /// (a decision, cheap), and for those the region (`crate::contact`). For a phase whose check
    /// failed or is Near; a passing pair costs a decision only.
    pub fn contacts(&self, phase: Phase, gauge: Option<&Gauged>) -> Vec<Contact> {
        let s = self.phase_span(phase);
        let limit = self.tl.clearance;
        let res = (limit / 4.0).max(0.01);
        let mut out = Vec::new();
        for el in self.elements(phase, gauge) {
            for pc in &self.pieces {
                if el.work != Work::None && matches!(pc.kind, Kind::ZoneJ { .. }) {
                    continue;
                }
                let pb = ToolPb {
                    ctx: self,
                    pc,
                    el: &el,
                    phase,
                };
                let roots = self.roots(pc, s);
                let near = match bb::certify(&pb, &roots, limit, res, BUDGET, false) {
                    Verdict::Clear { lower, .. } if lower >= limit => continue,
                    Verdict::Clear { lower, .. } => Some(lower),
                    _ => None,
                };
                let r = crate::contact::region(&pb, &roots, limit, res, REGION_BUDGET);
                if r.cells.is_empty() {
                    continue;
                }
                let rule = if phase == Phase::Return {
                    Rule::Withdraw
                } else if el.punchy && pc.folded {
                    Rule::ClosesOnPunch
                } else {
                    Rule::ToolCollision
                };
                let mut faces: Vec<ContactFace> = Vec::new();
                let mut points = Vec::new();
                let mut tool_points = Vec::new();
                let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
                for c in &r.cells {
                    let (q, a) = match c.proven {
                        Some((_, q, a)) => (q, a),
                        None => {
                            let q = c.cell.mid();
                            (q, pb.image(q))
                        }
                    };
                    let part = self.part_ref(pc, q);
                    match faces.iter_mut().find(|f| f.part == part) {
                        Some(f) => {
                            f.cells.push(c.cell.p);
                            f.proven += usize::from(c.proven.is_some());
                        }
                        None => faces.push(ContactFace {
                            part,
                            cells: vec![c.cell.p],
                            proven: usize::from(c.proven.is_some()),
                        }),
                    }
                    points.push(self.press_point(a, el.attach, phase, Some(q)));
                    // the tool's surface nearest the point: its section's boundary, its length
                    let p2 = Point2::new(a[0], a[2]);
                    let near = el
                        .sec
                        .bound
                        .iter()
                        .map(|sg| sg.closest(p2))
                        .min_by(|x, y| x.distance(p2).total_cmp(&y.distance(p2)))
                        .unwrap_or(p2);
                    let ty = a[1].clamp(el.y.0, el.y.1);
                    tool_points.push(self.press_point([near.x, ty, near.y], el.attach, phase, Some(q)));
                    // the box: the cell's enclosure, lifted by the ram over the cell's motion
                    let e = pb.enclose(&c.cell);
                    let lift = match (el.attach, phase) {
                        (Attach::Die, _) => Interval::point(0.0),
                        (Attach::Punch, Phase::Bend) => self.ztip(c.cell.p[3]),
                        (Attach::Punch, Phase::Approach | Phase::Return) => c.cell.p[3],
                        (Attach::Punch, Phase::Static) => Interval::point(self.ztip(self.phi0())),
                    };
                    let z = e[2] + lift;
                    for (k, iv) in [e[0], e[1], z].into_iter().enumerate() {
                        lo[k] = lo[k].min(iv.lo);
                        hi[k] = hi[k].max(iv.hi);
                    }
                }
                let estimate = self
                    .tl
                    .press
                    .estimated
                    .iter()
                    .any(|e| el.name.starts_with(e.as_str()));
                out.push(Contact {
                    rule,
                    phase,
                    severity: if rule == Rule::Withdraw {
                        Severity::Warning
                    } else {
                        Severity::Error
                    },
                    tool: el.name.clone(),
                    estimate,
                    limit,
                    depth: r.depth(),
                    lower: near.unwrap_or(r.lower()).max(r.lower()),
                    near,
                    complete: r.complete,
                    faces,
                    others: Vec::new(),
                    span: r.span().unwrap_or((s.lo, s.hi)),
                    points,
                    tool_points,
                    bbox: (lo, hi),
                });
            }
        }
        out
    }

    /// The moving side against the rest of the part, through the bend.
    pub fn self_check(&self, quick: bool) -> Check {
        // the least gap allowed (0: touching is the limit), less the touch tolerance
        let limit = self.tl.self_clearance - 1e-5;
        let res = 0.02;
        let s = Interval::new(self.phi0(), self.phimax);
        let (a, b) = (self.bj.a, self.bj.b);
        let wall_of = |pc: &Piece| match pc.kind {
            Kind::Wall { w, .. } => Some(w),
            _ => None,
        };
        let mut lowest = (f64::INFINITY, f64::INFINITY);
        // the least proven lower bound of every pair (the bound; under the limit, a Near)
        let mut least = f64::INFINITY;
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
                // a hem's leg closing on its base, in either stroke, is the hem: the two meet
                // only at the fold, whose radius here is the hem's own (a closed hem's next to
                // none), and their gap is the sheet's, built and checked there
                if self.bj.hem.is_some() && wall_of(mv) == Some(b) && wall_of(st) == Some(a) {
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
                let t0 = trace_clock();
                let v = bb::certify(&pb, &roots, limit, res, BUDGET, !quick);
                if std::env::var_os("BENDSIM_TRACE").is_some() {
                    eprintln!(
                        "trace self {:?} vs {:?} quick {} cells {} {:?} {:?}",
                        mv.strip,
                        st.strip,
                        quick,
                        bb::CELLS.load(std::sync::atomic::Ordering::Relaxed) - c0,
                        t0.map(|t| t.elapsed()),
                        v
                    );
                }
                match v {
                    Verdict::Clear { lower, upper } | Verdict::Near { lower, upper } => {
                        if upper < lowest.1 {
                            lowest = (lower, upper);
                        }
                        least = least.min(lower);
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
                            Verdict::Clear { .. } | Verdict::Near { .. } => unreachable!(),
                        }
                        let _ = quick;
                        return c;
                    }
                }
            }
        }
        if least < limit {
            let mut c = Check::near(Rule::SelfCollision, least, limit, res);
            c.phase = Some(Phase::Bend);
            return c;
        }
        let mut c = Check::rule(Rule::SelfCollision, lowest.1.min(1e9), limit, false);
        c.bound = Some(least.min(1e9));
        c.pass = true;
        c.phase = Some(Phase::Bend);
        c
    }

    /// The part against itself through the bend, where it comes within the self clearance:
    /// each moving piece against each piece of the other side not proven clear, with its region
    /// on both (the moving piece's faces and cells, the other's faces), as `contacts`.
    pub fn self_contacts(&self) -> Vec<Contact> {
        let limit = self.tl.self_clearance - 1e-5;
        let res = 0.02;
        let s = Interval::new(self.phi0(), self.phimax);
        let (a, b) = (self.bj.a, self.bj.b);
        let wall_of = |pc: &Piece| match pc.kind {
            Kind::Wall { w, .. } => Some(w),
            _ => None,
        };
        let mut out = Vec::new();
        for mv in self.pieces.iter().filter(|p| p.side != Side::A) {
            for st in self.pieces.iter().filter(|p| p.side != Side::J) {
                if st.side == mv.side
                    || (mv.side == Side::J && (wall_of(st) == Some(a) || wall_of(st) == Some(b)))
                    || (self.bj.hem.is_some() && wall_of(mv) == Some(b) && wall_of(st) == Some(a))
                {
                    continue;
                }
                let pb = SelfPb {
                    ctx: self,
                    mv,
                    st,
                    frame: st.side,
                };
                let roots = self.roots(mv, s);
                let near = match bb::certify(&pb, &roots, limit, res, BUDGET, false) {
                    Verdict::Clear { lower, .. } if lower >= limit => continue,
                    Verdict::Clear { lower, .. } => Some(lower),
                    _ => None,
                };
                let r = crate::contact::region(&pb, &roots, limit, res, REGION_BUDGET);
                if r.cells.is_empty() {
                    continue;
                }
                let mut faces: Vec<ContactFace> = Vec::new();
                let mut others: Vec<PartRef> = Vec::new();
                let (mut points, mut near_points) = (Vec::new(), Vec::new());
                let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
                for c in &r.cells {
                    let (q, at) = match c.proven {
                        Some((_, q, a)) => (q, a),
                        None => {
                            let q = c.cell.mid();
                            (q, pb.image(q))
                        }
                    };
                    let part = self.part_ref(mv, q);
                    match faces.iter_mut().find(|f| f.part == part) {
                        Some(f) => {
                            f.cells.push(c.cell.p);
                            f.proven += usize::from(c.proven.is_some());
                        }
                        None => faces.push(ContactFace {
                            part,
                            cells: vec![c.cell.p],
                            proven: usize::from(c.proven.is_some()),
                        }),
                    }
                    // the other piece where the point stands against it
                    let pq = st.inv.apply(V3::new(at[0], at[1], at[2]));
                    let o = self.part_ref(st, [pq.x, pq.y, pq.z, 0.0]);
                    if !others.contains(&o) {
                        others.push(o);
                    }
                    let pp = self.frame_to_press(at, st.side, q[3]);
                    points.push(pp);
                    near_points.push(pp);
                    // the box: the cell's enclosure carried to the press frame over its turn
                    let e = pb.enclose(&c.cell);
                    let rr = self.rel(V3::new(e[0], e[1], e[2]), st.side, c.cell.p[3]);
                    let z = rr.z + self.ztip(c.cell.p[3]);
                    for (k, iv) in [rr.x, rr.y, z].into_iter().enumerate() {
                        lo[k] = lo[k].min(iv.lo);
                        hi[k] = hi[k].max(iv.hi);
                    }
                }
                out.push(Contact {
                    rule: Rule::SelfCollision,
                    phase: Phase::Bend,
                    severity: Severity::Error,
                    tool: "part".into(),
                    estimate: false,
                    limit,
                    depth: r.depth(),
                    lower: near.unwrap_or(r.lower()).max(r.lower()),
                    near,
                    complete: r.complete,
                    faces,
                    others,
                    span: r.span().unwrap_or((s.lo, s.hi)),
                    points,
                    tool_points: near_points,
                    bbox: (lo, hi),
                });
            }
        }
        out
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
        if self.flatten.is_some() {
            return self.flat_rules();
        }
        if self.offset.is_some() {
            return self.off_rules();
        }
        let mut out = Vec::new();
        let included = dmath::PI - self.phimax;
        out.push(Check::rule(Rule::DieAngle, included, self.die.angle, false));
        out.push(Check::rule(
            Rule::PunchAngle,
            included,
            self.punch.tip_angle,
            false,
        ));
        // a hem's pre-bend: its radius is the tool's, closed to the hem's by the flattening
        if self.bj.hem.is_none() {
            let r_in = self.bj.inside_radius(self.phimax);
            out.push(Check::rule(
                Rule::PunchRadius,
                self.punch.tip_radius,
                r_in + 1e-9,
                true,
            ));
        }
        self.tool_rating(&mut out);
        // the shoulder's contact on the leg through the motion, certified by bisection
        // certified: the contact's least place along the leg, and its farthest
        let (cmin, cmax, at, proven) = self.contact_range();
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
        // the clamps' heights unknown: how far the ram must come down is not known either
        out.push(if self.tl.press.unchecked.iter().any(|u| u == "open_height") {
            Check::not_checked(
                Rule::Stroke,
                "the tool clamps' heights under the press's open height",
            )
        } else {
            Check::rule(Rule::Stroke, travel, self.tl.press.stroke, true)
        });
        // the clamps only take room: without them the tools already taller than the daylight
        // leave the part none, a failure known without the clamps' heights
        let room = Check::rule(Rule::OpenHeight, self.z_tdc, self.t + self.tl.clearance, false);
        let mut oh = if room.pass && self.tl.press.unchecked.iter().any(|u| u == "open_height") {
            Check::not_checked(
                Rule::OpenHeight,
                "the tool clamps' heights under the press's open height",
            )
        } else {
            room
        };
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

    /// The sheet against each tool's published rating (only where the maker gives one).
    fn tool_rating(&self, out: &mut Vec<Check>) {
        for (max, name) in [
            (self.punch.max_thickness, format!("punch:{}", self.punch.id)),
            (self.die.max_thickness, format!("die:{}", self.die.id)),
        ] {
            if let Some(max) = max {
                let mut c = Check::rule(Rule::ToolRating, self.t, max, true);
                c.tool = Some(name);
                out.push(c);
            }
        }
    }

    /// A hem's flattening: the stroke the press must make, the part under the punch at top dead
    /// centre (the pre-bent leg standing up), the tools' segments and the bed, the tools'
    /// ratings. Not the air bend's (the V's, the shoulder's, the punch tip's): the faces are flat.
    /// The flattening force is not checked: the makers' tables (WILA catalogue p.111, Mate p.47)
    /// are read but their columns not yet confirmed against the pages.
    fn flat_rules(&self) -> (Vec<Check>, f64, Option<f64>) {
        let mut out = Vec::new();
        let top0 = self.ztip(self.phi0());
        let travel = self.z_tdc - self.ztip(self.phimax);
        out.push(if self.tl.press.unchecked.iter().any(|u| u == "open_height") {
            Check::not_checked(
                Rule::Stroke,
                "the tool clamps' heights under the press's open height",
            )
        } else {
            Check::rule(Rule::Stroke, travel, self.tl.press.stroke, true)
        });
        let mut oh = Check::rule(Rule::OpenHeight, self.z_tdc, top0 + self.tl.clearance, false);
        oh.phase = Some(Phase::Approach);
        out.push(oh);
        let (pl, dl) = self.tool_lengths();
        let mut seg = Check::rule(
            Rule::Segments,
            pl.unwrap_or(0.0).min(dl.unwrap_or(0.0)),
            self.need(),
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
        self.tool_rating(&mut out);
        out.push(Check::not_checked(
            Rule::Tonnage,
            "the flattening force: the makers' tables are not yet confirmed",
        ));
        (out, travel, None)
    }

    /// An offset: the part's offset (how far the stroke takes the far wall down) against the
    /// inserts' Z, both inserts the same Z; the stroke, the open height, the segments and the
    /// ratings. The force is not checked: no maker publishes what an offset takes.
    fn off_rules(&self) -> (Vec<Check>, f64, Option<f64>) {
        let mut out = Vec::new();
        let drop = self.ztip(self.phi0()) - self.ztip(self.phimax);
        let z = |r: Role| match r {
            Role::Offset { z } => Some(z),
            _ => None,
        };
        let mut c = match (z(self.punch.role), z(self.die.role)) {
            (Some(a), Some(b)) if (a - b).abs() <= 1e-9 => {
                // a hundredth either way: the part is drawn to the inserts' Z
                let mut c = Check::rule(Rule::Offset, drop, a + OFFSET_TOL, true);
                c.lo = Some(a - OFFSET_TOL);
                c.pass = c.pass && drop >= a - OFFSET_TOL;
                c
            }
            _ => Check::rule(Rule::Offset, drop, f64::NAN, true),
        };
        if c.limit.is_nan() {
            c.pass = false;
            c.tool = Some(format!("die:{}", self.die.id));
        }
        out.push(c);
        let travel = self.z_tdc - self.ztip(self.phimax);
        out.push(Check::rule(Rule::Stroke, travel, self.tl.press.stroke, true));
        let mut oh = Check::rule(
            Rule::OpenHeight,
            self.z_tdc,
            self.ztip(self.phi0()) + self.tl.clearance,
            false,
        );
        oh.phase = Some(Phase::Approach);
        out.push(oh);
        let (pl, dl) = self.tool_lengths();
        let mut seg = Check::rule(
            Rule::Segments,
            pl.unwrap_or(0.0).min(dl.unwrap_or(0.0)),
            self.need(),
            false,
        );
        if pl.is_none() || dl.is_none() {
            seg.pass = false;
        }
        out.push(seg);
        self.tool_rating(&mut out);
        out.push(Check::not_checked(
            Rule::Tonnage,
            "the force an offset takes: no maker publishes it",
        ));
        (out, travel, None)
    }

    /// The legs' straight runs (far, near) and the part's length along the bed.
    pub fn leg_geometry(&self) -> (f64, f64, f64) {
        self.legs
            .unwrap_or_else(|| (self.leg_run(true), self.leg_run(false), self.part_extent_y()))
    }

    /// The part's length along the bed (press y), exact for walls, enclosed for zones.
    pub fn part_extent_y(&self) -> f64 {
        let (lo, hi) = self.part_span_y();
        hi - lo
    }

    /// Where the part reaches along the bed, in the working station's frame (y = 0 the bend's
    /// middle): an enclosure, so it errs wide. The bend turns about a line along y, so this holds
    /// through the whole step.
    pub fn part_span_y(&self) -> (f64, f64) {
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
        (lo, hi)
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

/// The trace's clock: only when tracing, and never on wasm32 (no clock there: Instant panics).
fn trace_clock() -> Option<std::time::Instant> {
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("BENDSIM_TRACE").is_some() {
        return Some(std::time::Instant::now());
    }
    None
}

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
    fn lower(&self, e: &[Interval; 3]) -> (f64, f64) {
        prism_lower(self.el, e)
    }
    fn weights(&self) -> [f64; 3] {
        [1.0, 0.05, 1.0]
    }
}

/// The tool is a prism along y: the bound in (x, z) and the exact least distance in y.
fn prism_lower(el: &Element, e: &[Interval; 3]) -> (f64, f64) {
    let (y0, y1) = el.y;
    let hd2 = (0.25 * (e[0].width().powi(2) + e[2].width().powi(2))).sqrt() + bb::SLACK;
    let s2 = el.sec.sd(Point2::new(e[0].mid(), e[2].mid())) - hd2;
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

/// A rigid move of the whole part (the operator's handling between two steps) over its parameter
/// f in [0, 1]: one formula between consecutive knots.
pub trait Moving {
    /// 0 = k0 < k1 < .. < kn = 1: the move is smooth between consecutive knots.
    fn knots(&self) -> &[f64];
    /// A point of the part (press frame, as the step places it) at f.
    fn place(&self, p: [f64; 3], f: f64) -> [f64; 3];
    /// Encloses `place` over every point of `p` at every f of `f` (f within one knot span).
    fn enclose(&self, p: [Interval; 3], f: Interval) -> [Interval; 3];
}

/// A body the handled part is held clear of: a tool prism in its own step's frame (a station's
/// `oy`, what rides the ram lifted to `top`), over the knot spans it stands through.
pub struct HandleTarget {
    el: Element,
    oy: f64,
    top: f64,
    spans: Vec<usize>,
}

impl HandleTarget {
    pub fn name(&self) -> &str {
        &self.el.name
    }
    /// The move's spans it is checked through.
    pub fn spans(&self) -> &[usize] {
        &self.spans
    }
    /// The body's box in the press frame (its section's box, its length along the bed), grown
    /// past rounding.
    pub fn bbox(&self) -> ([f64; 3], [f64; 3]) {
        let (lo, hi) = self.el.sec.bbox();
        let dz = if self.el.attach == Attach::Punch {
            self.top
        } else {
            0.0
        };
        let e = 1e-6;
        (
            [lo.x - e, self.el.y.0 + self.oy - e, lo.y + dz - e],
            [hi.x + e, self.el.y.1 + self.oy + e, hi.y + dz + e],
        )
    }
    /// The same body, checked through fewer spans.
    pub fn only(&self, spans: Vec<usize>) -> HandleTarget {
        HandleTarget {
            el: self.el.clone(),
            oy: self.oy,
            top: self.top,
            spans,
        }
    }
    /// The signed distance from a press-frame point to the body.
    pub fn sd(&self, p: [f64; 3]) -> f64 {
        let z = if self.el.attach == Attach::Punch {
            p[2] - self.top
        } else {
            p[2]
        };
        sd_prism(&self.el.sec, self.el.y.0, self.el.y.1, [p[0], p[1] - self.oy, z])
    }
}

/// A piece of the part, carried by the move from where the step places it, against a target.
struct HandlePb<'a, 'b> {
    ctx: &'b StepCtx<'a>,
    pc: &'b Piece,
    tg: &'b HandleTarget,
    mv: &'b dyn Moving,
}

impl HandlePb<'_, '_> {
    /// From the press frame to the target's.
    fn local<T: Sc>(&self, p: [T; 3]) -> [T; 3] {
        let z = if self.tg.el.attach == Attach::Punch {
            p[2] + (-self.tg.top)
        } else {
            p[2]
        };
        [p[0], p[1] + (-self.tg.oy), z]
    }
    fn rest<T: Sc>(&self, q: [T; 4]) -> [T; 3] {
        let r = self
            .ctx
            .image(self.pc, [q[0], q[1], q[2], T::c(0.0)], Phase::Static, Attach::Die);
        [r.x, r.y + self.ctx.oy, r.z]
    }
}

impl Problem for HandlePb<'_, '_> {
    fn may_hold(&self, c: &Cell) -> bool {
        StepCtx::may_hold(self.pc, c)
    }
    fn enclose(&self, c: &Cell) -> [Interval; 3] {
        self.local(self.mv.enclose(self.rest(c.p), c.p[3]))
    }
    fn witness(&self, c: &Cell) -> Option<[f64; 4]> {
        StepCtx::witness(self.pc, c)
    }
    fn image(&self, q: [f64; 4]) -> [f64; 3] {
        self.local(self.mv.place(self.rest(q), q[3]))
    }
    fn sd(&self, p: [f64; 3]) -> f64 {
        sd_prism(&self.tg.el.sec, self.tg.el.y.0, self.tg.el.y.1, p)
    }
    fn lower(&self, e: &[Interval; 3]) -> (f64, f64) {
        prism_lower(&self.tg.el, e)
    }
    fn weights(&self) -> [f64; 3] {
        [1.0, 0.05, 1.0]
    }
}

impl StepCtx<'_> {
    /// The punch tip's height while the part is handled: top dead centre, never below the flat
    /// sheet (as the scene stands it).
    fn handle_top(&self) -> f64 {
        self.z_tdc.max(self.ztip(self.phi0()))
    }

    /// The step's bodies during handling, the ram at top dead centre: with `gauge`, its
    /// backgauge alone (placed as at `phase`), else every other body. Each stands through
    /// `spans` of the move; a terrain (the bed, a die without a profile: a vertical line meets
    /// each in one interval down to its foot) is left out of the `vertical` spans, where the part
    /// moves straight up off it (or down onto it): a point above a terrain moving up only moves
    /// away from it, and resting on it is the step's own check.
    pub fn handle_targets(
        &self,
        gauge: Option<(&Gauged, Phase)>,
        spans: &[usize],
        vertical: &[usize],
    ) -> Vec<HandleTarget> {
        let els = match gauge {
            Some((g, phase)) => self
                .elements(phase, Some(g))
                .into_iter()
                .filter(|e| e.name.starts_with("backgauge"))
                .collect::<Vec<_>>(),
            None => self.elements(Phase::Bend, None),
        };
        let top = self.handle_top();
        els.into_iter()
            .map(|el| {
                let terrain = el.name == "bed"
                    || el
                        .name
                        .split_once("die:")
                        .and_then(|(_, id)| self.tl.die(id))
                        .is_some_and(|d| d.profile.is_none());
                let spans = spans
                    .iter()
                    .copied()
                    .filter(|s| !(terrain && vertical.contains(s)))
                    .collect();
                HandleTarget {
                    el,
                    oy: self.oy,
                    top,
                    spans,
                }
            })
            .collect()
    }

    /// The part carried by `mv` (this step's pieces as it places them) against `targets`: a
    /// handling check, every verdict certified as every other distance is. A proven collision
    /// fails, an unsettled one is undecided, a margin within the resolution is Near.
    pub fn handling(&self, mv: &dyn Moving, targets: &[HandleTarget]) -> Check {
        let limit = self.tl.clearance;
        let res = (limit / 4.0).max(0.01);
        let knots = mv.knots();
        let mut lowest = (f64::INFINITY, f64::INFINITY, String::new());
        let mut least = (f64::INFINITY, String::new());
        let mut hit: Option<Check> = None;
        let mut undecided: Option<Check> = None;
        for tg in targets {
            if tg.spans.is_empty() {
                continue;
            }
            for pc in &self.pieces {
                let [u, v, w] = pc.dom;
                let roots: Vec<Cell> = tg
                    .spans
                    .iter()
                    .map(|&s| Cell {
                        p: [u, v, w, Interval::new(knots[s], knots[s + 1])],
                        tag: 0,
                    })
                    .collect();
                let pb = HandlePb {
                    ctx: self,
                    pc,
                    tg,
                    mv,
                };
                // decided, not measured: a pass is proven at the limit (its value the nearest
                // real point found), and measuring every near pair past it costs the most
                match bb::certify(&pb, &roots, limit, res, BUDGET, false) {
                    Verdict::Clear { lower, upper } | Verdict::Near { lower, upper } => {
                        if upper < lowest.1 {
                            lowest = (lower, upper, tg.el.name.clone());
                        }
                        lowest.0 = lowest.0.min(lower);
                        if lower < least.0 {
                            least = (lower, tg.el.name.clone());
                        }
                    }
                    Verdict::Hit { d, q, at } => {
                        if hit.as_ref().is_none_or(|c| d < c.value) {
                            let mut c = Check::rule(Rule::Handling, d, limit, false);
                            c.pass = false;
                            c.certified = true;
                            c.tool = Some(tg.el.name.clone());
                            if self
                                .tl
                                .press
                                .estimated
                                .iter()
                                .any(|e| tg.el.name.starts_with(e.as_str()))
                            {
                                c.estimate = true;
                                c.certified = false;
                            }
                            c.part = Some(self.part_ref(pc, q));
                            // back in the press frame
                            let z = if tg.el.attach == Attach::Punch {
                                at[2] + tg.top
                            } else {
                                at[2]
                            };
                            c.at = Some([at[0], at[1] + tg.oy, z]);
                            hit = Some(c);
                        }
                    }
                    Verdict::Undecided { lower } => {
                        if undecided.is_none() {
                            let mut c = Check::rule(Rule::Handling, lower, limit, false);
                            c.pass = false;
                            c.certified = false;
                            c.undecided = true;
                            c.tool = Some(tg.el.name.clone());
                            c.part = Some(self.part_ref(pc, [0.0; 4]));
                            undecided = Some(c);
                        }
                    }
                }
            }
        }
        let mut c = if let Some(c) = hit.or(undecided) {
            c
        } else if least.0 < limit {
            let mut c = Check::near(Rule::Handling, least.0, limit, res);
            c.tool = Some(least.1);
            c
        } else {
            // the nearest real point found, else the proven bound
            let value = if lowest.1.is_finite() { lowest.1 } else { lowest.0 };
            let mut c = Check::rule(Rule::Handling, value.min(1e9), limit, false);
            c.bound = Some(lowest.0.min(1e9));
            c.pass = true;
            c.certified = true;
            if !lowest.2.is_empty() {
                c.tool = Some(lowest.2);
            }
            c
        };
        // the operator's move: theirs to choose, never a block
        c.severity = Severity::Warning;
        c
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
        // moving with the stroke: never a fixed piece another is measured against
        Kind::ZoneJ { .. } | Kind::ZoneK { .. } => (f64::INFINITY, f64::INFINITY),
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

/// A planned step's contacts: in each phase whose tool check failed or is Near (`withdraw`
/// included), every piece against every tool not proven clear, with its region. A step whose
/// tool checks all pass has none, and costs nothing.
pub fn contacts_of(m: &Model, tl: &Tooling, done: &[bool], step: &Step) -> Vec<Contact> {
    let selfs = step
        .checks
        .iter()
        .any(|c| c.rule == Rule::SelfCollision && (!c.pass || c.near.is_some()));
    let mut phases: Vec<Phase> = step
        .checks
        .iter()
        .filter(|c| matches!(c.rule, Rule::ToolCollision | Rule::ClosesOnPunch | Rule::Withdraw))
        .filter(|c| !c.pass || c.near.is_some())
        .filter_map(|c| c.phase)
        .collect();
    phases.sort_by_key(|p| *p as u8);
    phases.dedup();
    if phases.is_empty() && !selfs {
        return Vec::new();
    }
    let ctx = StepCtx::of_step(m, tl, done, step);
    let mut out = Vec::new();
    if selfs {
        out.extend(ctx.self_contacts());
    }
    for ph in phases {
        out.extend(ctx.contacts(ph, step.gauge.as_ref()));
    }
    out
}

#[cfg(test)]
mod contact_tests {
    use super::*;
    use crate::plan::{Options, plan};
    use forge_math::Frame;
    use forge_ops::{ProfileLoop, Segment};
    use forge_sheet::{BendRule, FeatureId, Gauge, Sheet};

    fn channel(w: f64, len: f64, h: f64) -> Sheet {
        let pts = [(0.0, 0.0), (w, 0.0), (w, len), (0.0, len)];
        let lp = ProfileLoop::new(
            (0..4)
                .map(|i| {
                    let (a, b) = (pts[i], pts[(i + 1) % 4]);
                    (
                        Segment::Line {
                            a: Point2::new(a.0, a.1),
                            b: Point2::new(b.0, b.1),
                        },
                        i as u64,
                    )
                })
                .collect(),
        );
        let (mut s, base) = Sheet::face(
            FeatureId(1),
            Gauge {
                thickness: 2.0,
                radius: 2.0,
                rule: BendRule::K(0.4),
            },
            Frame::WORLD,
            &[lp],
        )
        .unwrap();
        let q = dmath::PI / 2.0;
        s.flange(base, 1, q, h).unwrap();
        s.flange(base, 3, q, h).unwrap();
        s
    }

    fn boxed(w: f64, d: f64, h: f64) -> Sheet {
        let pts = [(0.0, 0.0), (w, 0.0), (w, d), (0.0, d)];
        let lp = ProfileLoop::new(
            (0..4)
                .map(|i| {
                    let (a, b) = (pts[i], pts[(i + 1) % 4]);
                    (
                        Segment::Line {
                            a: Point2::new(a.0, a.1),
                            b: Point2::new(b.0, b.1),
                        },
                        i as u64,
                    )
                })
                .collect(),
        );
        let (mut s, base) = Sheet::face(
            FeatureId(1),
            Gauge {
                thickness: 1.5,
                radius: 1.5,
                rule: BendRule::K(0.4),
            },
            Frame::WORLD,
            &[lp],
        )
        .unwrap();
        for seg in 0..4 {
            s.flange(base, seg, dmath::PI / 2.0, h).unwrap();
        }
        s
    }

    /// The part against itself: a box's last flange meets its neighbours at the corners. With
    /// the self clearance past that gap, every point of the moving side proven under it lies in a
    /// contact cell, the neighbours are among the contact's other faces, and the box holds the
    /// points.
    #[test]
    fn a_self_contact_holds_every_point_that_meets_the_part() {
        let m = Model::new(&boxed(120.0, 60.0, 25.0)).unwrap();
        let mut tl = crate::library::std_eu();
        let su = Setup {
            punch: 0,
            die: 0,
            rotate: false,
        };
        let done = [true, true, true, false];
        let c0 = StepCtx::new(&m, &tl, &done, 3, su, 0.0).self_check(false);
        assert!(c0.pass);
        tl.self_clearance = c0.value + 0.03;
        let ctx = StepCtx::new(&m, &tl, &done, 3, su, 0.0);
        let cs = ctx.self_contacts();
        assert!(!cs.is_empty());
        assert!(
            cs.iter()
                .all(|c| c.rule == Rule::SelfCollision && c.tool == "part" && !c.others.is_empty())
        );
        let limit = tl.self_clearance - 1e-5;
        let s = Interval::new(0.0, ctx.phimax);
        let mut checked = 0;
        for c in &cs {
            for p in &c.points {
                for ((x, lo), hi) in p.iter().zip(c.bbox.0).zip(c.bbox.1) {
                    assert!(lo - 1e-6 <= *x && *x <= hi + 1e-6);
                }
            }
        }
        for mv in ctx.pieces.iter().filter(|p| p.side != Side::A) {
            for st in ctx
                .pieces
                .iter()
                .filter(|p| p.side != Side::J && p.side != mv.side)
            {
                // zone j borders walls a and b: never paired (as the check)
                let borders = matches!(st.kind, Kind::Wall { w, .. } if w == ctx.bj.a || w == ctx.bj.b);
                if mv.side == Side::J && borders {
                    continue;
                }
                let pb = SelfPb {
                    ctx: &ctx,
                    mv,
                    st,
                    frame: st.side,
                };
                for root in ctx.roots(mv, s) {
                    let n = 30;
                    for i in 0..=n {
                        for j in 0..=n {
                            for w in [root.p[2].lo, root.p[2].hi] {
                                for k in 0..=6 {
                                    let f = |iv: Interval, t: usize, n: usize| {
                                        iv.lo + iv.width() * t as f64 / n as f64
                                    };
                                    let q = [f(root.p[0], i, n), f(root.p[1], j, n), w, f(root.p[3], k, 6)];
                                    let pt = Cell {
                                        p: q.map(Interval::point),
                                        tag: root.tag,
                                    };
                                    if !StepCtx::may_hold(mv, &pt) {
                                        continue;
                                    }
                                    if pb.sd_bounds(pb.image(q)).1 < limit {
                                        checked += 1;
                                        let inside = cs.iter().any(|c| {
                                            c.faces.iter().any(|fc| {
                                                fc.cells.iter().any(|cl| {
                                                    (0..4).all(|t| cl[t].lo <= q[t] && q[t] <= cl[t].hi)
                                                })
                                            })
                                        });
                                        assert!(
                                            inside,
                                            "{:?} vs {:?} {q:?}: in no contact cell",
                                            mv.strip, st.strip
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(checked > 0, "the sample found where the part meets itself");
    }

    /// A narrow, tall channel on straight punches only: its second flange closes on the punch.
    /// Sampled densely through the failing phase, every point of every piece under the clearance
    /// lies in a cell of a contact with that tool (the region is never smaller than the truth);
    /// the faces it touches are among the contact's; the contact's lower bound is at most the
    /// sampled least distance, and its proven depth within a leaf of it.
    #[test]
    fn a_contact_holds_every_point_that_collides() {
        let s = channel(30.0, 200.0, 60.0);
        let m = Model::new(&s).unwrap();
        let tl = crate::library::std_eu();
        let straight: Vec<usize> = (0..tl.punches.len())
            .filter(|&k| tl.punches[k].kind == crate::tooling::PunchKind::Straight)
            .collect();
        let p = plan(
            &m,
            &tl,
            &Options {
                punches: Some(straight),
                sequence: Some(vec![0, 1]),
                ..Options::default()
            },
        );
        assert!(!p.feasible);
        let st = &p.steps[1];
        let done = vec![st.bend != 0, st.bend != 1];
        let cs = contacts_of(&m, &tl, &done, st);
        assert!(!cs.is_empty(), "{:?}", st.failures());
        assert!(
            cs.iter()
                .any(|c| c.rule == Rule::ClosesOnPunch && c.depth.is_some())
        );
        for c in &cs {
            assert!(!c.faces.is_empty() && c.points.len() == c.tool_points.len());
            assert!(c.lower < c.limit);
            for k in 0..3 {
                assert!(c.bbox.0[k] <= c.bbox.1[k]);
            }
        }
        // brute force, the failing phases' every piece against every tool
        let mut ctx = StepCtx::new(&m, &tl, &done, st.bend, st.setup, st.overbend);
        if let Some(k) = st.station {
            ctx = ctx.at_station(k);
        }
        let phases: Vec<Phase> = cs.iter().map(|c| c.phase).collect();
        let mut checked = 0;
        for ph in [Phase::Static, Phase::Approach, Phase::Bend, Phase::Return] {
            if !phases.contains(&ph) {
                continue;
            }
            let span = ctx.phase_span(ph);
            for el in ctx.elements(ph, st.gauge.as_ref()) {
                for pc in &ctx.pieces {
                    if el.work != Work::None && matches!(pc.kind, Kind::ZoneJ { .. }) {
                        continue;
                    }
                    let mine: Vec<&Contact> =
                        cs.iter().filter(|c| c.phase == ph && c.tool == el.name).collect();
                    let mut least = f64::INFINITY;
                    for root in ctx.roots(pc, span) {
                        let n = 24;
                        for i in 0..=n {
                            for j in 0..=n {
                                for w in [root.p[2].lo, root.p[2].hi] {
                                    for k in 0..=8 {
                                        let f = |iv: Interval, t: usize, n: usize| {
                                            iv.lo + iv.width() * t as f64 / n as f64
                                        };
                                        let q =
                                            [f(root.p[0], i, n), f(root.p[1], j, n), w, f(root.p[3], k, 8)];
                                        let pt = Cell {
                                            p: q.map(Interval::point),
                                            tag: root.tag,
                                        };
                                        if !StepCtx::may_hold(pc, &pt) || ctx.on_working(pc, &pt, el.work) {
                                            continue;
                                        }
                                        let a = ctx.image(pc, q, ph, el.attach).arr();
                                        let d = sd_prism(&el.sec, el.y.0, el.y.1, a);
                                        least = least.min(d);
                                        if d < tl.clearance - 1e-6 {
                                            checked += 1;
                                            let inside = mine.iter().any(|c| {
                                                c.faces.iter().any(|fc| {
                                                    fc.cells.iter().any(|cl| {
                                                        (0..4).all(|t| cl[t].lo <= q[t] && q[t] <= cl[t].hi)
                                                    })
                                                })
                                            });
                                            assert!(
                                                inside,
                                                "{ph:?} {} {:?} {q:?} at {d}: in no contact cell",
                                                el.name, pc.strip
                                            );
                                            let face = ctx.part_ref(pc, q);
                                            assert!(
                                                mine.iter().any(|c| c
                                                    .faces
                                                    .iter()
                                                    .any(|fc| fc.part.strip == face.strip)),
                                                "{:?} not among the contact's faces",
                                                face
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if least < tl.clearance {
                        let c = mine.iter().min_by(|a, b| a.lower.total_cmp(&b.lower)).unwrap();
                        assert!(c.lower <= least + 1e-9, "lower {} vs sampled {least}", c.lower);
                    }
                }
            }
        }
        assert!(checked > 0, "the sample found the collision");
        let deepest = cs.iter().filter_map(|c| c.depth).fold(f64::INFINITY, f64::min);
        assert!(deepest < tl.clearance);
    }
}

#[cfg(test)]
mod hem_tests {
    use super::*;
    use forge_math::Frame;
    use forge_ops::{ProfileLoop, Segment};
    use forge_sheet::{BendRule, FeatureId, Gauge, HemKind, HemSpec, Sheet};

    /// A 120 x 60 plate, 1 mm, a hem of `kind` on its right edge, leg 10.
    fn hemmed(kind: HemKind, up: bool) -> Sheet {
        let pts = [(0.0, 0.0), (120.0, 0.0), (120.0, 60.0), (0.0, 60.0)];
        let lp = ProfileLoop::new(
            (0..4)
                .map(|i| {
                    let (a, b) = (pts[i], pts[(i + 1) % 4]);
                    (
                        Segment::Line {
                            a: Point2::new(a.0, a.1),
                            b: Point2::new(b.0, b.1),
                        },
                        i as u64,
                    )
                })
                .collect(),
        );
        let (mut s, base) = Sheet::face(
            FeatureId(1),
            Gauge {
                thickness: 1.0,
                radius: 1.0,
                rule: BendRule::K(0.4),
            },
            Frame::WORLD,
            &[lp],
        )
        .unwrap();
        s.hem(base, 1, HemSpec { kind, leg: 10.0, up }, None).unwrap();
        s
    }

    fn su() -> Setup {
        Setup {
            punch: 0,
            die: 0,
            rotate: false,
        }
    }

    /// Every sampled point of the part at turn phi, in the press frame.
    fn points(ctx: &StepCtx, phi: f64) -> Vec<[f64; 3]> {
        let mut out = Vec::new();
        for pc in &ctx.pieces {
            let [u, v, w] = pc.dom;
            for i in 0..=8 {
                for k in 0..=2 {
                    for ww in [w.lo, w.hi] {
                        let (uu, vv) = (
                            (u.lo + u.width() * i as f64 / 8.0).clamp(u.lo + 1e-7, u.hi - 1e-7),
                            (v.lo + v.width() * k as f64 / 2.0).clamp(v.lo + 1e-7, v.hi - 1e-7),
                        );
                        let inside = match &pc.kind {
                            Kind::Wall { region, .. } => region.contains(Point2::new(uu, vv)),
                            _ => true,
                        };
                        if !inside {
                            continue;
                        }
                        let q = ctx.side_point(pc, uu, vv, ww, phi);
                        let r = ctx.rel(q, pc.side, phi);
                        out.push([r.x, r.y, r.z + ctx.ztip(phi)]);
                    }
                }
            }
        }
        out
    }

    /// Flattening an open hem from the 30 degree pre-bend: the base lies on the die (its lowest
    /// point at 0) at every turn; the punch face is the part's highest point (no sampled point
    /// above it, one at it); it comes down as the turn grows; at the hem's turn it stands at two
    /// thicknesses and the gap; and a cell's interval of turns encloses the face's heights.
    #[test]
    fn flattening_lays_the_base_flat_and_the_face_on_the_top() {
        let tl = crate::library::std_eu();
        for up in [true, false] {
            let gap = 1.0;
            let m = Model::new(&hemmed(HemKind::Open { gap }, up)).unwrap();
            let from = dmath::PI - 30f64.to_radians();
            let ctx = StepCtx::new(&m, &tl, &[false], 0, su(), 0.0)
                .flattening(from)
                .expect("a leg with nothing beyond");
            assert_eq!(ctx.phi0(), from);
            assert_eq!(ctx.phimax, m.bends[0].theta);
            let mut last = f64::INFINITY;
            for i in 0..=20 {
                let phi = from + (ctx.phimax - from) * i as f64 / 20.0;
                let top = ctx.ztip(phi);
                let ps = points(&ctx, phi);
                let lo = ps.iter().map(|p| p[2]).fold(f64::INFINITY, f64::min);
                let hi = ps.iter().map(|p| p[2]).fold(f64::NEG_INFINITY, f64::max);
                assert!(lo.abs() < 1e-9, "up {up} phi {phi}: the base at {lo}");
                assert!(
                    hi <= top + 1e-9 && hi >= top - 0.05,
                    "up {up} phi {phi}: top {top}, part {hi}"
                );
                assert!(top <= last + 1e-12, "up {up}: the face goes up at {phi}");
                last = top;
            }
            let end = ctx.ztip(ctx.phimax);
            assert!((end - (2.0 * m.t + gap)).abs() < 1e-9, "up {up}: closed at {end}");
            for k in 0..10 {
                let a = from + (ctx.phimax - from) * k as f64 / 10.0;
                let b = from + (ctx.phimax - from) * (k + 1) as f64 / 10.0;
                let e = ctx.ztip(Interval::new(a, b));
                for i in 0..=10 {
                    let z = ctx.ztip(a + (b - a) * i as f64 / 10.0);
                    assert!(e.lo <= z && z <= e.hi, "{z} outside {e:?}");
                }
            }
        }
    }

    /// The pre-bend must be past a quarter turn, and a leg with walls beyond it is not flattened.
    #[test]
    fn flattening_refuses_what_it_does_not_model() {
        let tl = crate::library::std_eu();
        let m = Model::new(&hemmed(HemKind::Open { gap: 1.0 }, true)).unwrap();
        assert!(
            StepCtx::new(&m, &tl, &[false], 0, su(), 0.0)
                .flattening(1.0)
                .is_none()
        );
    }
}

#[cfg(test)]
mod offset_tests {
    use super::*;
    use forge_math::Frame;
    use forge_ops::{ProfileLoop, Segment};
    use forge_sheet::{BendRule, FeatureId, Gauge, Sheet, WallId};

    /// A Z: a 60 x 100 base, a web `z` high off its right edge (+90), a 30 long far wall off the
    /// web's top (-90), 1 mm.
    pub(super) fn zed(z: f64) -> Sheet {
        let pts = [(0.0, 0.0), (60.0, 0.0), (60.0, 100.0), (0.0, 100.0)];
        let lp = ProfileLoop::new(
            (0..4)
                .map(|i| {
                    let (a, b) = (pts[i], pts[(i + 1) % 4]);
                    (
                        Segment::Line {
                            a: Point2::new(a.0, a.1),
                            b: Point2::new(b.0, b.1),
                        },
                        i as u64,
                    )
                })
                .collect(),
        );
        let (mut s, base) = Sheet::face(
            FeatureId(1),
            Gauge {
                thickness: 1.0,
                radius: 1.0,
                rule: BendRule::K(0.4),
            },
            Frame::WORLD,
            &[lp],
        )
        .unwrap();
        let q = dmath::PI / 2.0;
        let (web, _) = s.flange(base, 1, q, z).unwrap();
        let _ = web;
        s.flange(WallId(1), 1, -q, 30.0).unwrap();
        s
    }

    fn su() -> Setup {
        Setup {
            punch: 0,
            die: 0,
            rotate: false,
        }
    }

    fn at(ctx: &StepCtx, pc: &Piece, u: f64, v: f64, w: f64, phi: f64) -> [f64; 3] {
        let r = ctx.rel(ctx.side_point(pc, u, v, w, phi), pc.side, phi);
        [r.x, r.y, r.z + ctx.ztip(phi)]
    }

    /// An offset's stroke: the base stays on the die (its die side at 0) and level; the far
    /// wall stays level, its upper skin where the punch's face is, coming down as the turn
    /// grows; intervals of turns enclose the face's heights; and at the bends' own turn the
    /// part is the folded part (every distance between its points the model's).
    #[test]
    fn an_offsets_outer_walls_stay_level_and_the_end_is_the_part() {
        let tl = crate::library::std_eu();
        let m = Model::new(&zed(8.0)).unwrap();
        assert_eq!(m.bends.len(), 2);
        let mut ctx = StepCtx::new(&m, &tl, &[false, false], 0, su(), 0.0);
        assert!(ctx.set_offset(1));
        let (a, c) = (m.bends[0].a, m.bends[1].b);
        let wall = |w: usize| {
            ctx.pieces
                .iter()
                .find(|p| matches!(p.kind, Kind::Wall { w: x, .. } if x == w))
                .unwrap()
        };
        assert_eq!(wall(c).side, Side::C);
        let corners = |pc: &Piece| {
            let [u, v, _] = pc.dom;
            let mut out = Vec::new();
            for uu in [u.lo + 0.5, u.hi - 0.5] {
                for vv in [v.lo + 0.5, v.hi - 0.5] {
                    out.push((uu, vv));
                }
            }
            out
        };
        let mut last = f64::INFINITY;
        let theta = m.bends[0].theta;
        for i in 0..=10 {
            let phi = theta * i as f64 / 10.0;
            let za: Vec<f64> = corners(wall(a))
                .into_iter()
                .flat_map(|(u, v)| [0.0, -1.0].map(|w| at(&ctx, wall(a), u, v, w, phi)[2]))
                .collect();
            let lo = za.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = za.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            assert!(
                lo.abs() < 1e-9 && (hi - 1.0).abs() < 1e-9,
                "phi {phi}: base {lo}..{hi}"
            );
            let zc: Vec<f64> = corners(wall(c))
                .into_iter()
                .flat_map(|(u, v)| [0.0, -1.0].map(|w| at(&ctx, wall(c), u, v, w, phi)[2]))
                .collect();
            let top = ctx.ztip(phi);
            let chi = zc.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let clo = zc.iter().copied().fold(f64::INFINITY, f64::min);
            assert!(
                (chi - top).abs() < 1e-9 && (chi - clo - 1.0).abs() < 1e-9,
                "phi {phi}: far {clo}..{chi}, face {top}"
            );
            assert!(top <= last + 1e-12, "the face goes up at {phi}");
            last = top;
        }
        for k in 0..10 {
            let (x, y) = (theta * k as f64 / 10.0, theta * (k + 1) as f64 / 10.0);
            let e = ctx.ztip(Interval::new(x, y));
            for i in 0..=10 {
                let z = ctx.ztip(x + (y - x) * i as f64 / 10.0);
                assert!(e.lo <= z && z <= e.hi);
            }
        }
        // the end: the folded part, up to a rigid motion
        let f = m.frames(&[theta, theta]);
        let mut mine = Vec::new();
        let mut theirs = Vec::new();
        for w in [a, m.bends[0].b, c] {
            let pc = wall(w);
            let Kind::Wall { to_param, .. } = &pc.kind else {
                unreachable!()
            };
            for (u, v) in corners(pc) {
                mine.push(at(&ctx, pc, u, v, 0.0, theta));
                // the wall's own coordinates of that parameter point
                let own = to_param.inverse().apply(Point2::new(u, v));
                theirs.push(f[w].apply(V3::new(own.x, own.y, 0.0)).arr());
            }
        }
        let d = |p: [f64; 3], q: [f64; 3]| {
            ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
        };
        for i in 0..mine.len() {
            for j in 0..mine.len() {
                assert!(
                    (d(mine[i], mine[j]) - d(theirs[i], theirs[j])).abs() < 1e-9,
                    "{i} {j}"
                );
            }
        }
        // what is not such a pair is refused
        let mut other = StepCtx::new(&m, &tl, &[false, false], 1, su(), 0.0);
        assert!(!other.set_offset(0));
    }
}

#[cfg(test)]
use offset_tests::zed as offset_tests_zed;

#[cfg(test)]
mod offset_plan_tests {
    use super::*;
    use crate::plan::{Options, plan};

    /// std-eu with an offset pair of Z `z` (90 degrees, rated to `max_t`): a test fixture, not a
    /// maker's tools.
    fn with_offset(z: f64, max_t: Option<f64>) -> Tooling {
        let mut tl = crate::library::std_eu();
        let p0 = tl.punches[0].clone();
        tl.punches.push(Punch {
            id: "test/offset-top".into(),
            role: Role::Offset { z },
            tip_angle: dmath::PI / 2.0,
            tip_height: 30.0,
            width: 23.0,
            profile: None,
            relief: None,
            drawn_width: None,
            max_thickness: max_t,
            ..p0
        });
        let d0 = tl.dies[0].clone();
        tl.dies.push(Die {
            id: "test/offset-bottom".into(),
            role: Role::Offset { z },
            angle: dmath::PI / 2.0,
            width: 60.0,
            profile: None,
            max_thickness: max_t,
            ..d0
        });
        tl
    }

    fn drop_of(m: &Model) -> f64 {
        let tl = crate::library::std_eu();
        let su = Setup {
            punch: 0,
            die: 0,
            rotate: false,
        };
        let mut ctx = StepCtx::new(m, &tl, &[false, false], 0, su, 0.0);
        assert!(ctx.set_offset(1));
        ctx.ztip(0.0) - ctx.ztip(ctx.phimax)
    }

    /// A Z on an offset pair of its own Z: one stroke makes both bends, passing, with the offset
    /// rule at the part's offset; inserts of another Z are refused by the offset rule, and
    /// inserts rated thinner than the sheet by their rating: the Z is then bent as two air
    /// bends, as without the inserts.
    #[test]
    fn a_z_is_made_in_one_stroke_on_inserts_of_its_offset() {
        let m = Model::new(&super::offset_tests_zed(8.0)).unwrap();
        let z = drop_of(&m);
        let tl = with_offset(z, Some(1.5));
        let p = plan(&m, &tl, &Options::default());
        assert!(p.feasible, "{:?}", p.blocking);
        assert_eq!(
            p.steps.len(),
            1,
            "{:?}",
            p.steps.iter().map(|s| s.stroke).collect::<Vec<_>>()
        );
        let st = &p.steps[0];
        assert_eq!(st.stroke, Stroke::Offset { k: 1 });
        assert_eq!(
            (st.punch.as_str(), st.die.as_str()),
            ("test/offset-top", "test/offset-bottom")
        );
        let off = st.checks.iter().find(|c| c.rule == Rule::Offset).unwrap();
        assert!(off.pass && (off.value - z).abs() < 1e-9, "{off:?}");
        assert!(st.checks.iter().any(|c| c.rule == Rule::ToolRating && c.pass));
        // another Z, or a rating under the sheet: two air bends instead, and why not the inserts
        for tl in [with_offset(z + 0.5, Some(1.5)), with_offset(z, Some(0.5))] {
            let p = plan(&m, &tl, &Options::default());
            assert!(
                p.steps.iter().all(|s| s.stroke == Stroke::Air),
                "{:?}",
                p.steps.iter().map(|s| s.stroke).collect::<Vec<_>>()
            );
            let su = Setup {
                punch: tl.punches.len() - 1,
                die: tl.dies.len() - 1,
                rotate: false,
            };
            let mut ctx = StepCtx::new(&m, &tl, &[false, false], 0, su, 0.0);
            assert!(ctx.set_offset(1));
            let (checks, _, _) = ctx.rules(None);
            assert!(
                checks
                    .iter()
                    .any(|c| matches!(c.rule, Rule::Offset | Rule::ToolRating) && !c.pass),
                "{checks:?}"
            );
        }
    }
    /// A flange standing on the base, under a wide offset punch holder: the offset's tool check
    /// proves the hit and names the punch.
    /// The scene draws a Z's offset stroke at every instant, the far wall ending the offset below
    /// the base.
    #[test]
    fn an_offset_meets_a_flange_and_the_scene_draws_one() {
        use forge_sheet::WallId;
        let m0 = Model::new(&super::offset_tests_zed(8.0)).unwrap();
        let z = drop_of(&m0);
        let tl = with_offset(z, None);
        let su = Setup {
            punch: tl.punches.len() - 1,
            die: tl.dies.len() - 1,
            rotate: false,
        };
        // the scene of a Z made in one stroke
        let p = plan(&m0, &tl, &Options::default());
        assert_eq!(p.steps.len(), 1);
        let mut sc = crate::scene::Scene::new(&m0, &tl, &p);
        for t in [0.0, 0.2, 0.3, 0.5, 0.7, 0.8, 1.0] {
            assert!(sc.bodies(0, t).iter().any(|b| b.look.is_sheet()), "t {t}");
        }
        let lo = sc
            .bodies(0, 0.84)
            .iter()
            .filter(|b| b.look.is_sheet())
            .map(|b| b.mesh.bounds().unwrap().0[2])
            .fold(f32::MAX, f32::min);
        assert!((f64::from(lo) + z).abs() < 1e-3, "{lo} vs {z}");
        // a 40 high flange on the base's far edge, 60 from the web, bent first, and a punch
        // holder 150 wide reaching over it
        let mut s = super::offset_tests_zed(8.0);
        s.flange(WallId(0), 3, dmath::PI / 2.0, 40.0).unwrap();
        let mut tl = tl;
        tl.punches.last_mut().unwrap().width = 150.0;
        let m = Model::new(&s).unwrap();
        let (j, k) = (0, 1);
        let f = 2;
        let mut done = vec![false; 3];
        done[f] = true;
        let mut ctx = StepCtx::new(&m, &tl, &done, j, su, 0.0);
        assert!(ctx.set_offset(k));
        let mut checks = Vec::new();
        ctx.collisions(Phase::Bend, None, &mut checks, false);
        // proven: the flange within the clearance of the punch (the check stops at its first
        // proven failure)
        let fails: Vec<&Check> = checks.iter().filter(|c| !c.pass).collect();
        assert!(!fails.is_empty(), "{checks:?}");
        for c in &fails {
            assert!(c.certified && c.value < c.limit, "{c:?}");
            assert!(
                c.tool.as_deref().is_some_and(|t| t.contains("test/offset-top")),
                "{c:?}"
            );
        }
    }
}

//! Reliefs (David, 2026-11-03: option C): data on a joint's ends and at corners, cut by the
//! builder, sized by a shop rule the way CAD sheet metal rules hold them (Inventor's Sheet Metal
//! Rule, Fusion's sheet metal rule, SolidWorks' auto relief, Creo's SMT_DFLT parameters,
//! Onshape's feature options: research in sheet-metal/reliefs-prior-art.md).
//!
//! No ISO or DIN standard fixes relief geometry (DIN 9003-5 surfaced; its relief content is
//! unconfirmed). The default sizes below are RULES OF THUMB from fabricators' design guides, not
//! standards:
//! - depth t + R + 0.5 mm from the bend line: SendCutSend's bend relief guide (sendcutsend.com,
//!   "at least t + R, plus 0.02 in / 0.5 mm, measured from the bend line"); Xometry and Protocase
//!   agree on at least t + R;
//! - width t: Xometry and Protocase ("at least the material thickness"); SendCutSend allows t/2
//!   on lasers, a design handbook 2t, MachineMFG t + R: guides disagree on width;
//! - tear reliefs are not cuttable on a laser (zero kerf): ours is a slit the shop's kerf wide;
//!   the 0.2 mm default kerf is a stand-in for a fibre laser's, not a cited figure;
//! - minimum remnant, corner relief size and the three-bend corner have no fabricator figure we
//!   could cite: off, "just clear of both bends", and none, until a shop states its own.
//!
//! A bend relief is a notch in the wall beside the bend zone's end, from past the edge to its
//! depth; its side continues the zone's end face. Ending on the tangent line beside the zone is
//! built (a free wall edge on the line's extension); a round end touching the line at a point is
//! refused (a zero-width spot in the solid).
use crate::error::{End, JointId, WallId};
use crate::shop::Scaled;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReliefKind {
    /// Square-bottomed.
    Rect,
    /// Round-bottomed: a half circle the relief's width across.
    Obround,
    /// A slit the shop's kerf wide.
    Tear,
}

impl ReliefKind {
    pub fn name(self) -> &'static str {
        match self {
            ReliefKind::Rect => "rect",
            ReliefKind::Obround => "obround",
            ReliefKind::Tear => "tear",
        }
    }
}

/// Where a bend relief's depth is measured from (the CAD rules' "depth reference").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepthRef {
    /// From the bend line, as fabricators state it, taken in folded terms: the mold line,
    /// where the two walls' reference faces meet. (On the flat the bend line sits half an
    /// allowance past the tangent line; measuring from it would let K move a relief in the
    /// folded part, and K moves only the flat.)
    BendLine,
    /// Up to the bend: the relief's straight sides end on the tangent line (its depth unused).
    UpToBend,
    /// Tangent to the bend: its deepest point on the tangent line (its depth unused); a round
    /// end then touches the line at a point and is refused.
    TangentToBend,
}

impl DepthRef {
    pub fn name(self) -> &'static str {
        match self {
            DepthRef::BendLine => "bend_line",
            DepthRef::UpToBend => "up_to_bend",
            DepthRef::TangentToBend => "tangent_to_bend",
        }
    }
}

/// What a joint end asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Relief {
    /// The rule's relief, where one is needed; nothing where none is.
    Auto,
    /// No relief, even where one is needed (then `NeedsRelief`).
    None,
    /// This relief, needed or not, its depth from the rule's reference. A tear's width is the
    /// kerf.
    Cut {
        kind: ReliefKind,
        width: f64,
        depth: f64,
    },
}

/// The bend relief part of a rule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BendReliefRule {
    pub kind: ReliefKind,
    pub width: Scaled,
    pub depth: Scaled,
    pub reference: DepthRef,
    /// Material left between a relief and the wall's edge narrower than this is taken too (the
    /// relief widened to the edge). Off at zero.
    pub min_remnant: Scaled,
    /// Lengthen reliefs to clear flat-pattern collisions (Onshape's option). Not built: true is
    /// refused (`ReliefExtendUnbuilt`).
    pub extend: bool,
}

/// Where two bends off one wall meet (CAD rules' two-bend corner relief).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CornerShape {
    Round,
    Square,
    /// A kerf-wide slit along the corner's diagonal, from past the edge to past both tangent
    /// lines' crossing.
    Tear,
    /// The corner cut to both bends' tangent lines, across both zones.
    TrimToBend,
    /// A slot of the rule's size (default t) along the diagonal, as far as a tear.
    ConstantWidth,
    /// Flanges extended to meet (closed, weld variants): typed, not built.
    Closed,
}

/// Where a round or square corner relief is centred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// On the crossing of the two bend lines on the flat.
    Intersection,
    /// On the crossing of the two tangent lines.
    Tangent,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerRule {
    pub shape: CornerShape,
    pub placement: Placement,
    /// The radius or half side; None: just clear of both bends.
    pub size: Option<Scaled>,
    /// The gap an open corner leaves between two walls (fabricators: at least 0.4 mm for a
    /// corner closed by welding; a rule of thumb).
    pub gap: Scaled,
}

/// Where three bends meet. No current feature makes such a corner; the rule is held as data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner3Shape {
    None,
    Intersection,
    FullRound,
    Round,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner3Rule {
    pub shape: Corner3Shape,
    pub radius: Scaled,
}

/// A shop's relief rule for one stock: bend reliefs, two- and three-bend corners, and the kerf.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReliefRule {
    pub bend: BendReliefRule,
    pub corner: CornerRule,
    pub corner3: Corner3Rule,
    pub kerf: f64,
}

impl Default for ReliefRule {
    /// The rules of thumb in the module's note: a rect relief t wide, t + R + 0.5 deep from the
    /// bend line; round corners just clear of both bends at the bend lines' crossing; kerf 0.2.
    fn default() -> ReliefRule {
        ReliefRule {
            bend: BendReliefRule {
                kind: ReliefKind::Rect,
                width: Scaled {
                    per_t: 1.0,
                    per_r: 0.0,
                    plus: 0.0,
                },
                depth: Scaled {
                    per_t: 1.0,
                    per_r: 1.0,
                    plus: 0.5,
                },
                reference: DepthRef::BendLine,
                min_remnant: Scaled::NONE,
                extend: false,
            },
            corner: CornerRule {
                shape: CornerShape::Round,
                placement: Placement::Intersection,
                size: None,
                gap: Scaled {
                    per_t: 0.0,
                    per_r: 0.0,
                    plus: 0.4,
                },
            },
            corner3: Corner3Rule {
                shape: Corner3Shape::None,
                radius: Scaled::NONE,
            },
            kerf: 0.2,
        }
    }
}

/// A relief the builder cut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReliefMade {
    pub joint: JointId,
    pub end: End,
    pub wall: WallId,
    pub kind: ReliefKind,
    pub width: f64,
    /// From the mold line, as cut.
    pub depth: f64,
    /// Added because the wall needed it (not asked for).
    pub automatic: bool,
    /// Widened to the wall's edge: the remnant there was under the rule's minimum.
    pub remnant_taken: bool,
}

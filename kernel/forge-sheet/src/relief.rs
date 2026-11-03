//! Bend reliefs (David, 2026-11-03: option C): data on a joint's ends, cut by the builder. A
//! relief is a notch in the wall beside the bend zone's end, from past the edge to `depth` from
//! the mold line, always past the tangent line, so its side continues the zone's end face and
//! nothing ends on a tangent line. Where a wall keeps material beside a bend and its end is left
//! `Auto`, the builder adds the sheet's default relief; an explicit `None` there stays
//! `NeedsRelief`.
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

/// What a joint end asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Relief {
    /// The sheet's default, where one is needed; nothing where none is.
    Auto,
    /// No relief, even where one is needed (then `NeedsRelief`).
    None,
    /// This relief, needed or not. A tear's width is the kerf.
    Cut {
        kind: ReliefKind,
        width: f64,
        depth: f64,
    },
}

/// The default relief, sized from thickness and the bend's inside radius as shops state it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReliefDefault {
    pub kind: ReliefKind,
    pub width: Scaled,
    /// From the mold line.
    pub depth: Scaled,
    /// The shop's cutting width: a tear relief's.
    pub kerf: f64,
}

impl Default for ReliefDefault {
    /// Width t, depth t + R + 0.5, kerf 0.2: a stand-in until a shop states its own.
    fn default() -> ReliefDefault {
        ReliefDefault {
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
    pub depth: f64,
    /// Added because the wall needed it (not asked for).
    pub automatic: bool,
}

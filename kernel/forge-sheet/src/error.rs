//! Everything that can go wrong or be flagged, typed: each variant names the feature and the entity
//! and carries the measured value and the limit it broke. No string reasons.
use crate::region::RegionError;
use crate::shop::RuleError;
use forge_math::{Point2, Point3};
use forge_ops::ProfileError;

/// The sheet feature in the part's history; its names root here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureId(pub u64);

/// Ids are given when the entity is made and never reused, so names built from them stay put
/// when other walls, joints or cuts come and go.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WallId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JointId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CutId(pub u32);

/// Which wall of a joint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
    A,
    B,
}

/// Which end of a joint's mold line: where its y is 0, or its length.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum End {
    Start,
    End,
}

/// One edge of a trim, as the user or the graph made it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TrimRef {
    /// Segment `segment` of a wall's outline (its own profile's id).
    Outline { wall: WallId, segment: u64 },
    /// Segment `segment` of a cut's loop.
    Cut { cut: CutId, segment: u64 },
    /// A bend's tangent line on the side of wall `side`.
    TangentLine { joint: JointId, side: Side },
    /// The side of a bend zone at one end of its mold line.
    BendEnd { joint: JointId, end: End },
    /// A joint's mold line, as the edge of the band a wall gives up to its bend.
    MoldLine { joint: JointId, side: Side },
}

/// A wall or a bend zone: a piece of the sheet with its own developed coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Strip {
    Wall(WallId),
    Bend(JointId),
}

/// The curve kinds a trim can have, for refusals by kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrimCurveKind {
    Line,
    Arc,
    Spline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointKindName {
    Bend,
    Rip,
    Tangent,
}

/// Where two boundaries touch at a point: in 2D a region, but a solid would have zero width there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pinch {
    pub at: Point2,
    pub a: TrimRef,
    pub b: TrimRef,
    pub width: f64,
    pub limit: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SheetError {
    Thickness {
        feature: FeatureId,
        thickness: f64,
        limit: f64,
    },
    /// The inside radius a bend would take.
    Radius {
        feature: FeatureId,
        radius: f64,
        limit: f64,
    },
    /// A face's profile, as the profile check found it.
    Profile {
        feature: FeatureId,
        wall: WallId,
        why: ProfileError,
    },
    NoSuchWall {
        feature: FeatureId,
        wall: WallId,
    },
    NoSuchSegment {
        feature: FeatureId,
        wall: WallId,
        segment: u64,
    },
    /// A flange asked for on an edge that is not a straight line.
    NotStraight {
        feature: FeatureId,
        trim: TrimRef,
        curve: TrimCurveKind,
    },
    /// The edge already carries joint `joint`.
    EdgeTaken {
        feature: FeatureId,
        trim: TrimRef,
        joint: JointId,
    },
    /// A bend angle outside (0, pi) in size: `limit` is the bound it broke.
    BendAngle {
        feature: FeatureId,
        joint: JointId,
        angle: f64,
        limit: f64,
    },
    /// A wall with no length left once the bend's setback is taken off.
    WallTooShort {
        feature: FeatureId,
        wall: WallId,
        joint: JointId,
        length: f64,
        limit: f64,
    },
    /// The joint's mold line has different lengths on its two walls.
    JointLengths {
        feature: FeatureId,
        joint: JointId,
        a: f64,
        b: f64,
        limit: f64,
    },
    /// The joint's walls are not both in the sheet, or a wall reached twice.
    JointWalls {
        feature: FeatureId,
        joint: JointId,
    },
    /// A wall no bend reaches from the first wall.
    WallUnplaced {
        feature: FeatureId,
        wall: WallId,
    },
    /// Joints of this kind are typed but not built yet.
    JointKindUnbuilt {
        feature: FeatureId,
        joint: JointId,
        kind: JointKindName,
    },
    BendRule {
        feature: FeatureId,
        joint: JointId,
        why: RuleError,
    },
    /// Places where a trim leaves a strip with zero width.
    Pinched {
        feature: FeatureId,
        strip: Strip,
        pinches: Vec<Pinch>,
    },
    /// Two trim edges run side by side closer than the tolerance can tell from coincident.
    NearCoincident {
        feature: FeatureId,
        strip: Strip,
        a: TrimRef,
        b: TrimRef,
        at: Point2,
        gap: f64,
        limit: f64,
    },
    /// The kept trim pieces do not chain into loops at `at` (leaving and arriving counts).
    Unchained {
        feature: FeatureId,
        strip: Strip,
        at: Point2,
        trim: TrimRef,
        leaving: u32,
        arriving: u32,
    },
    /// A hole no outer loop holds.
    HoleOutside {
        feature: FeatureId,
        strip: Strip,
        trim: TrimRef,
        at: Point2,
    },
    /// A trim edge lies along a tangent line, from `from` to `to` (in the strip's coordinates).
    TrimAlongBendLine {
        feature: FeatureId,
        joint: JointId,
        trim: TrimRef,
        from: Point2,
        to: Point2,
    },
    /// A trim touches a tangent line without crossing it.
    TrimTouchesBendLine {
        feature: FeatureId,
        joint: JointId,
        trim: TrimRef,
        at: Point2,
        gap: f64,
        limit: f64,
    },
    /// The wall keeps material beside the bend zone at one end of the joint, so the bend would
    /// tear from it along the zone's side: the joint needs a relief there.
    NeedsRelief {
        feature: FeatureId,
        joint: JointId,
        wall: WallId,
        end: End,
        at: Point2,
    },
    /// The wall's material at its tangent line and the bend zone's disagree (a wall edge meets
    /// the mold line at a slant): the stretch of tangent line ending at `at` misses by `gap`.
    TangentMismatch {
        feature: FeatureId,
        joint: JointId,
        side: Side,
        at: Point2,
        gap: f64,
        limit: f64,
    },
    /// An arc whose sweep is not finite, is zero, or turns more than once (`limit` is the bound).
    ArcSweep {
        feature: FeatureId,
        trim: TrimRef,
        sweep: f64,
        limit: f64,
    },
    /// A trim curve of a kind this strip cannot carry yet (arcs and splines in a bend zone).
    TrimCurveUnsupported {
        feature: FeatureId,
        strip: Strip,
        trim: TrimRef,
        curve: TrimCurveKind,
    },
    /// The side wall along a trim edge in a bend is not a valid ruled surface.
    WallFolds {
        feature: FeatureId,
        joint: JointId,
        trim: TrimRef,
    },
    /// A trim edge too short to carry a curve.
    EdgeTooShort {
        feature: FeatureId,
        strip: Strip,
        trim: TrimRef,
        length: f64,
        limit: f64,
    },
}

/// Built anyway, but worth a look.
#[derive(Clone, Debug, PartialEq)]
pub enum SheetWarning {
    /// Two walls overlap in the flat pattern.
    FlatOverlap {
        feature: FeatureId,
        walls: (WallId, WallId),
        at: Point2,
        area: f64,
        limit: f64,
    },
    /// Two walls of the folded part pass through each other.
    FoldedClash {
        feature: FeatureId,
        walls: (WallId, WallId),
        at: Point3,
        depth: f64,
        limit: f64,
    },
    /// A relief leaves a sliver of material beside the bend.
    ReliefRemnant {
        feature: FeatureId,
        joint: JointId,
        remnant: f64,
        limit: f64,
    },
}

/// A part against the shop's abilities (`ShopProfile`). Violations never change geometry.
#[derive(Clone, Debug, PartialEq)]
pub enum Violation {
    HoleNearBend {
        feature: FeatureId,
        trim: TrimRef,
        joint: JointId,
        distance: f64,
        limit: f64,
        thickness: f64,
    },
    HoleNearEdge {
        feature: FeatureId,
        trim: TrimRef,
        distance: f64,
        limit: f64,
    },
    FlangeTooShort {
        feature: FeatureId,
        wall: WallId,
        joint: JointId,
        length: f64,
        limit: f64,
    },
    RadiusNotTooled {
        feature: FeatureId,
        joint: JointId,
        radius: f64,
        nearest: f64,
    },
    ThicknessNotStocked {
        feature: FeatureId,
        thickness: f64,
        nearest: f64,
    },
    SlotTooNarrow {
        feature: FeatureId,
        trim: TrimRef,
        width: f64,
        limit: f64,
        thickness: f64,
    },
}

impl SheetError {
    /// A region boolean's refusal in one strip, its edges named through `trim_of`.
    pub fn from_region(
        feature: FeatureId,
        strip: Strip,
        e: RegionError,
        trim_of: impl Fn(u64) -> TrimRef,
    ) -> SheetError {
        match e {
            RegionError::Pinched { pinches } => {
                let pinches: Vec<Pinch> = pinches
                    .iter()
                    .map(|p| Pinch {
                        at: p.at,
                        a: trim_of(p.a),
                        b: trim_of(p.b),
                        width: p.width,
                        limit: p.limit,
                    })
                    .collect();
                // a trim corner on a tangent line is the bend's business, not a pinch of the trims
                for p in &pinches {
                    for (t, other) in [(p.a, p.b), (p.b, p.a)] {
                        if let TrimRef::TangentLine { joint, .. } = t
                            && !matches!(other, TrimRef::TangentLine { .. })
                        {
                            return SheetError::TrimTouchesBendLine {
                                feature,
                                joint,
                                trim: other,
                                at: p.at,
                                gap: p.width,
                                limit: p.limit,
                            };
                        }
                    }
                }
                SheetError::Pinched {
                    feature,
                    strip,
                    pinches,
                }
            }
            RegionError::NearCoincident { a, b, at, gap, limit } => SheetError::NearCoincident {
                feature,
                strip,
                a: trim_of(a),
                b: trim_of(b),
                at,
                gap,
                limit,
            },
            RegionError::Unchained {
                at,
                edge,
                leaving,
                arriving,
            } => SheetError::Unchained {
                feature,
                strip,
                at,
                trim: trim_of(edge),
                leaving,
                arriving,
            },
            RegionError::HoleOutside { edge, at } => SheetError::HoleOutside {
                feature,
                strip,
                trim: trim_of(edge),
                at,
            },
        }
    }
}

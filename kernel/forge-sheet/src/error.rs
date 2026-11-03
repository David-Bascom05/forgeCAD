//! Everything that can go wrong or be flagged, typed: each variant names the feature and the entity
//! and carries the measured value and the limit it broke. No string reasons.
use crate::region::RegionError;
use crate::shop::RuleError;
use forge_math::{Point2, Point3, Vec2};
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
    /// Segment `segment` of the relief at one end of a joint.
    Relief { joint: JointId, end: End, segment: u64 },
    /// Segment `segment` of the corner relief where joints a and b meet.
    Corner { a: JointId, b: JointId, segment: u64 },
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
    /// Two strips of the folded part pass through each other: a point of `a`'s boundary lies `depth`
    /// inside `b`'s material at `at` (deeper than `limit`, so not a touch).
    FoldedClash {
        feature: FeatureId,
        a: Strip,
        b: Strip,
        at: Point3,
        depth: f64,
        limit: f64,
    },
    /// Two strips overlap in the flat pattern: no flat body or blank can be made of it. The
    /// pair overlapping most, `area` theirs, `total` over every pair.
    FlatOverlap {
        feature: FeatureId,
        a: Strip,
        b: Strip,
        at: Point2,
        area: f64,
        total: f64,
        limit: f64,
    },
    /// Hardware placed off its wall (its centre outside the wall's outline or in one of its
    /// holes): it would cut nothing and press into nothing.
    HardwareOffWall {
        feature: FeatureId,
        wall: WallId,
        at: Point2,
    },
    /// Two strips touch in the flat pattern without overlapping (an edge on an edge, a corner on
    /// an edge): the blank has zero width there.
    FlatTouch {
        feature: FeatureId,
        a: Strip,
        b: Strip,
        at: Point2,
        /// Where a cut along a wall's side would part them, when they lie along each other.
        seam: Option<Seam>,
    },
    /// Two strips of the folded part touch without passing into each other (walls meeting along
    /// a line at a corner): faces of one solid meeting, no valid solid.
    FoldedTouch {
        feature: FeatureId,
        a: Strip,
        b: Strip,
        at: Point3,
    },
    /// Two strips' flats come so close to touching (an edge `gap` from another, under `limit`)
    /// that the exact overlap check cannot tell overlapping from touching.
    FlatOverlapUndecided {
        feature: FeatureId,
        a: Strip,
        b: Strip,
        at: Point2,
        gap: f64,
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
    /// A relief that does not reach past the bend's tangent line (`limit`, from the mold line).
    ReliefTooShallow {
        feature: FeatureId,
        joint: JointId,
        end: End,
        depth: f64,
        limit: f64,
    },
    /// A corner treatment typed but not built (closed, overlapping), or a size that is not
    /// positive.
    CornerUnbuilt {
        feature: FeatureId,
        joints: [JointId; 2],
        size: f64,
    },
    /// The rule asks reliefs to be lengthened for flat collisions: not built yet.
    ReliefExtendUnbuilt {
        feature: FeatureId,
        joint: JointId,
        end: End,
    },
    /// Cuts part the sheet: strip `b` (a piece of it) is no longer joined to `a`'s.
    SheetSplit {
        feature: FeatureId,
        a: Strip,
        b: Strip,
    },
    /// A relief with no width.
    ReliefWidth {
        feature: FeatureId,
        joint: JointId,
        end: End,
        width: f64,
        limit: f64,
    },
    /// A flange's position would move an edge whose neighbour already carries a bend.
    EdgeMoveBlocked {
        feature: FeatureId,
        trim: TrimRef,
        joint: JointId,
    },
    /// A flange's position would move an edge whose neighbour is not a straight line.
    EdgeMoveCurve {
        feature: FeatureId,
        trim: TrimRef,
        curve: TrimCurveKind,
    },
    /// A contour flange's profile, refused (`crate::contour`).
    Contour {
        feature: FeatureId,
        why: crate::contour::ContourError,
    },
    /// A corner seam, refused (`crate::corner`).
    Seam {
        feature: FeatureId,
        joints: [JointId; 2],
        why: crate::corner::SeamError,
    },
    /// A hem, refused (`crate::hem`); `joint` is the id it would have had.
    Hem {
        feature: FeatureId,
        joint: JointId,
        why: crate::hem::HemError,
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

/// A run along which two strips touch in the flat, on one wall's side: from `from` to `to` in
/// that wall's own coordinates, `into` the unit way into its material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seam {
    pub wall: WallId,
    pub from: Point2,
    pub to: Point2,
    pub into: Vec2,
}

/// Built anyway, but worth a look.
#[derive(Clone, Debug, PartialEq)]
pub enum SheetWarning {
    /// Two strips overlap in the flat pattern (the folded part may be fine): a single blank
    /// cannot. `at` (developed coordinates) is on the overlap's edge; `area` is exact.
    FlatOverlap {
        feature: FeatureId,
        a: Strip,
        b: Strip,
        at: Point2,
        area: f64,
        /// Over every overlapping pair.
        total: f64,
        limit: f64,
    },
    /// The flat touches itself (see `SheetError::FlatTouch`): the folded part builds.
    FlatTouch {
        feature: FeatureId,
        a: Strip,
        b: Strip,
        at: Point2,
        /// Where a cut along a wall's side would part them, when they lie along each other.
        seam: Option<Seam>,
    },
    /// The folded part touches itself (see `SheetError::FoldedTouch`): the flat builds.
    FoldedTouch {
        feature: FeatureId,
        a: Strip,
        b: Strip,
        at: Point3,
    },
    /// The flat overlap check could not decide one pair (see `SheetError`).
    FlatOverlapUndecided {
        feature: FeatureId,
        a: Strip,
        b: Strip,
        at: Point2,
        gap: f64,
        limit: f64,
    },
    /// Two strips of the folded part pass through each other (the flat may be fine).
    FoldedClash {
        feature: FeatureId,
        a: Strip,
        b: Strip,
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
    /// Hardware whose catalogue numbers are placeholders, not a maker's published table.
    UnverifiedHardware { feature: FeatureId, cut: CutId },
}

/// A part against the shop's abilities (`ShopProfile`). Violations never change geometry.
#[derive(Clone, Debug, PartialEq)]
pub enum Violation {
    /// A hole or slot `distance` from a bend zone (its tangent line), in developed terms;
    /// `away` (unit, in the cut's wall coordinates) is the way it would have to move.
    HoleNearBend {
        feature: FeatureId,
        trim: TrimRef,
        joint: JointId,
        distance: f64,
        limit: f64,
        thickness: f64,
        away: Vec2,
    },
    /// A hole or slot `distance` from the part's outer edge.
    HoleNearEdge {
        feature: FeatureId,
        trim: TrimRef,
        distance: f64,
        limit: f64,
        away: Vec2,
    },
    /// A cut reaching into a bend zone. `intended` when the cut was marked as meant to cross it
    /// (then it is information, not a violation).
    CutCrossesBend {
        feature: FeatureId,
        trim: TrimRef,
        joint: JointId,
        intended: bool,
        away: Vec2,
    },
    /// A round hole smaller across than the shop can pierce in this thickness.
    HoleTooSmall {
        feature: FeatureId,
        trim: TrimRef,
        diameter: f64,
        limit: f64,
        thickness: f64,
    },
    /// A relief asked for smaller than the shop's rule makes them (both from the mold line).
    ReliefUndersized {
        feature: FeatureId,
        joint: JointId,
        end: End,
        width: f64,
        depth: f64,
        min_width: f64,
        min_depth: f64,
    },
    /// Hardware in a sheet outside its thickness range.
    HardwareThickness {
        feature: FeatureId,
        cut: CutId,
        thickness: f64,
        min: f64,
        max: f64,
    },
    /// Hardware whose hole centre lies `distance` from an edge or a bend zone, under the
    /// catalogue's minimum.
    HardwareNearEdge {
        feature: FeatureId,
        cut: CutId,
        distance: f64,
        limit: f64,
        away: Vec2,
    },
    /// The flat blank is larger than the fabricator takes (the coated limit for a coated part):
    /// `size` its smallest bounding rectangle, (shorter, longer), against `limit`.
    FlatTooLarge {
        feature: FeatureId,
        size: (f64, f64),
        limit: (f64, f64),
        coated: bool,
    },
    /// The part is smaller than the fabricator handles: `size` (shorter, longer) under `limit`.
    PartTooSmall {
        feature: FeatureId,
        size: (f64, f64),
        limit: (f64, f64),
    },
    /// A bend line longer than the brake.
    BendTooLong {
        feature: FeatureId,
        joint: JointId,
        length: f64,
        limit: f64,
    },
    /// A flange round a closed box taller than the tooling lets the box close over.
    BoxFlangeTooTall {
        feature: FeatureId,
        wall: crate::error::WallId,
        joint: JointId,
        height: f64,
        limit: f64,
    },
    /// A bend angle (unsigned, radians) outside what the brake makes.
    BendAngleOutOfRange {
        feature: FeatureId,
        joint: JointId,
        angle: f64,
        min: f64,
        max: f64,
    },
    /// Hardware whose mounting hole runs into another hole (or sits inside one): the other's
    /// edge `distance` from its centre, under the hole's radius (0: the centre inside it).
    HardwareOverlapsHole {
        feature: FeatureId,
        cut: CutId,
        other: TrimRef,
        distance: f64,
        /// The mounting hole's diameter.
        hole: f64,
        /// The part's edge distance, which a move clears too.
        limit: f64,
        /// From the other hole towards the centre (in the wall's coordinates), when the centre is
        /// outside it.
        away: Vec2,
    },
    /// Hardware whose centre is `distance` from another hole's edge, under the part's minimum
    /// edge distance (the catalogue's centre to edge, any edge).
    HardwareNearHole {
        feature: FeatureId,
        cut: CutId,
        other: TrimRef,
        distance: f64,
        limit: f64,
        away: Vec2,
    },
    /// The wall is too narrow for the hardware: across its centre, from edge or bend to edge or
    /// bend, `width` where the part needs `need` (its edge distance both ways). No move fixes it.
    LipTooShort {
        feature: FeatureId,
        cut: CutId,
        wall: crate::error::WallId,
        width: f64,
        need: f64,
    },
    /// The sheet is harder than the hardware can clinch into (Rockwell B).
    SheetTooHard {
        feature: FeatureId,
        cut: CutId,
        hardness: f64,
        limit: f64,
    },
    /// Hardware needs the sheet's hardness, and the shop's stock does not state it.
    HardnessUnknown {
        feature: FeatureId,
        cut: CutId,
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
    /// The refusal of one map, as a warning on the other: a flat overlap, a folded clash.
    pub fn as_warning(&self) -> Option<SheetWarning> {
        match *self {
            SheetError::FlatOverlap {
                feature,
                a,
                b,
                at,
                area,
                total,
                limit,
            } => Some(SheetWarning::FlatOverlap {
                feature,
                a,
                b,
                at,
                area,
                total,
                limit,
            }),
            SheetError::FoldedClash {
                feature,
                a,
                b,
                at,
                depth,
                limit,
            } => Some(SheetWarning::FoldedClash {
                feature,
                a,
                b,
                at,
                depth,
                limit,
            }),
            SheetError::FlatTouch {
                feature,
                a,
                b,
                at,
                seam,
            } => Some(SheetWarning::FlatTouch {
                feature,
                a,
                b,
                at,
                seam,
            }),
            SheetError::FoldedTouch { feature, a, b, at } => {
                Some(SheetWarning::FoldedTouch { feature, a, b, at })
            }
            SheetError::FlatOverlapUndecided {
                feature,
                a,
                b,
                at,
                gap,
                limit,
            } => Some(SheetWarning::FlatOverlapUndecided {
                feature,
                a,
                b,
                at,
                gap,
                limit,
            }),
            _ => None,
        }
    }

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

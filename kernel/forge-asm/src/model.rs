//! What an assembly document says: instances with written poses, joints between named anchors,
//! relations between joint values, declared fits. Nothing here is solved state.

use forge_math::Frame;
use std::collections::BTreeMap;

/// An instance's stable id within its assembly document. Never reused (the `next N` rule).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JointId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationId(pub u64);

/// A part or assembly document, by its project path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocId(pub std::sync::Arc<str>);

/// A document's content hash as the op log records it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash(pub [u8; 32]);

/// One body of a part document: the multibody feature id that made it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartRef {
    pub doc: DocId,
    pub body: u64,
    /// The version slot. None: the part as of the same project state as the assembly. Some: the
    /// part must have this hash (pinning later; today a mismatch is a warning).
    pub pin: Option<ContentHash>,
}

/// An occurrence: instance ids from the top document down through subassemblies.
pub type Path = Vec<InstanceId>;

#[derive(Clone, Debug, PartialEq)]
pub enum Component {
    Part(PartRef),
    /// A subassembly document, rigid as a whole in v1.
    Assembly(DocId),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    pub id: InstanceId,
    pub of: Component,
    /// The pose as written in the document: the solve starts here every time.
    pub pose: Frame,
    pub ground: bool,
    pub suppressed: bool,
    /// Checked for interference, never counted in the BOM or exported (an envelope, a floor).
    pub reference: bool,
}

impl Instance {
    pub fn part(id: u64, part: PartRef, pose: Frame) -> Instance {
        Instance {
            id: InstanceId(id),
            of: Component::Part(part),
            pose,
            ground: false,
            suppressed: false,
            reference: false,
        }
    }
    pub fn grounded(mut self) -> Instance {
        self.ground = true;
        self
    }
}

/// One free motion a joint leaves, in the reference (B) anchor's frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Freedom {
    Tx,
    Ty,
    Tz,
    Rx,
    Ry,
    Rz,
}

impl Freedom {
    pub fn is_angle(self) -> bool {
        matches!(self, Freedom::Rx | Freedom::Ry | Freedom::Rz)
    }
    pub fn name(self) -> &'static str {
        match self {
            Freedom::Tx => "Tx",
            Freedom::Ty => "Ty",
            Freedom::Tz => "Tz",
            Freedom::Rx => "Rx",
            Freedom::Ry => "Ry",
            Freedom::Rz => "Rz",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JointKind {
    Fastened,
    Revolute,
    Slider,
    Cylindrical,
    PinSlot,
    Planar,
    Ball,
}

impl JointKind {
    /// The values a joint of this kind leaves free, in the order its values are listed.
    pub fn freedoms(self) -> &'static [Freedom] {
        use Freedom::*;
        match self {
            JointKind::Fastened => &[],
            JointKind::Revolute => &[Rz],
            JointKind::Slider => &[Tz],
            JointKind::Cylindrical => &[Tz, Rz],
            JointKind::PinSlot => &[Tx, Rz],
            JointKind::Planar => &[Tx, Ty, Rz],
            JointKind::Ball => &[Rx, Ry, Rz],
        }
    }
    /// The primitive constraints this joint is, between A's anchor (flipped if asked) and B's.
    /// Pin-slot's line is B's x, given here as B's frame turned so its z is B's x.
    pub fn bundle(self) -> &'static [ConstraintKind] {
        use ConstraintKind::*;
        match self {
            JointKind::Fastened => &[Lock],
            JointKind::Revolute => &[Coincident, Parallel],
            JointKind::Slider => &[PointOnLine, Parallel, Twist { angle: 0.0 }],
            JointKind::Cylindrical => &[PointOnLine, Parallel],
            JointKind::PinSlot => &[PointOnLine, Parallel],
            JointKind::Planar => &[PointOnPlane { offset: 0.0 }, Parallel],
            JointKind::Ball => &[Coincident],
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            JointKind::Fastened => "fastened",
            JointKind::Revolute => "revolute",
            JointKind::Slider => "slider",
            JointKind::Cylindrical => "cylindrical",
            JointKind::PinSlot => "pin_slot",
            JointKind::Planar => "planar",
            JointKind::Ball => "ball",
        }
    }
}

/// Where a joint end attaches: an anchor of the part at the end of `path`.
#[derive(Clone, Debug, PartialEq)]
pub struct End {
    pub path: Path,
    pub anchor: AnchorRef,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnchorRef {
    /// A reference the part document declares: an anchor, or a role-named face, edge or axis
    /// (Forge's stable role names), resolved when the part was built.
    Named(String),
    /// An implicit reference stored in the assembly, in the occurrence's part coordinates.
    Local(Reference),
}

/// What a reference is. All are carried as a frame; the kind says which of its parts mean
/// something: a point its origin, an axis its origin and z, a plane its origin and normal z.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefKind {
    Point,
    Axis,
    Plane,
    Frame,
}

impl RefKind {
    pub fn name(self) -> &'static str {
        match self {
            RefKind::Point => "point",
            RefKind::Axis => "axis",
            RefKind::Plane => "plane",
            RefKind::Frame => "frame",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reference {
    pub kind: RefKind,
    pub frame: Frame,
}

impl Reference {
    pub fn frame(f: Frame) -> Reference {
        Reference {
            kind: RefKind::Frame,
            frame: f,
        }
    }
    pub fn point(p: forge_math::Point3) -> Reference {
        Reference {
            kind: RefKind::Point,
            frame: Frame::WORLD.with_origin(p),
        }
    }
    pub fn axis(o: forge_math::Point3, dir: forge_math::UnitVec3) -> Reference {
        Reference {
            kind: RefKind::Axis,
            frame: Frame::from_z(o, dir),
        }
    }
    pub fn plane(o: forge_math::Point3, normal: forge_math::UnitVec3) -> Reference {
        Reference {
            kind: RefKind::Plane,
            frame: Frame::from_z(o, normal),
        }
    }
}

/// Which end of a joint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
    /// The moving end: its frame is placed by the joint's motion in B's frame.
    A,
    /// The reference end: values and limits are measured in its frame.
    B,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Joint {
    pub id: JointId,
    pub kind: JointKind,
    pub a: End,
    pub b: End,
    /// A's z opposes B's z (A's frame turned half a turn about its x). Chosen by the author, never
    /// by the solver.
    pub flip: bool,
    /// Values given as inputs (params): (freedom, value). Angles in radians, lengths in mm.
    pub drive: Vec<(Freedom, f64)>,
    /// (freedom, lo, hi).
    pub limits: Vec<(Freedom, f64, f64)>,
    pub suppressed: bool,
}

impl Joint {
    pub fn new(id: u64, kind: JointKind, a: End, b: End) -> Joint {
        Joint {
            id: JointId(id),
            kind,
            a,
            b,
            flip: false,
            drive: Vec::new(),
            limits: Vec::new(),
            suppressed: false,
        }
    }
    pub fn driven(mut self, f: Freedom, v: f64) -> Joint {
        self.drive.push((f, v));
        self
    }
    pub fn limited(mut self, f: Freedom, lo: f64, hi: f64) -> Joint {
        self.limits.push((f, lo, hi));
        self
    }
    pub fn flipped(mut self) -> Joint {
        self.flip = true;
        self
    }
    pub fn end(&self, s: Side) -> &End {
        match s {
            Side::A => &self.a,
            Side::B => &self.b,
        }
    }
}

/// A joint value: one freedom of one joint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueRef {
    pub joint: JointId,
    pub freedom: Freedom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RelationKind {
    /// Follower angle = -ratio × leader angle (external mesh turns the other way; `ratio` < 0 for
    /// an internal gear).
    Gear { ratio: f64 },
    /// Follower length = radius × leader angle.
    RackPinion { radius: f64 },
    /// Follower length = lead / 2π × leader angle.
    Screw { lead: f64 },
    /// Follower length = ratio × leader length.
    Linear { ratio: f64 },
}

impl RelationKind {
    /// d follower / d leader.
    pub fn slope(self) -> f64 {
        match self {
            RelationKind::Gear { ratio } => -ratio,
            RelationKind::RackPinion { radius } => radius,
            RelationKind::Screw { lead } => lead / forge_math::dmath::TAU,
            RelationKind::Linear { ratio } => ratio,
        }
    }
    /// (leader is an angle, follower is an angle).
    pub fn dims(self) -> (bool, bool) {
        match self {
            RelationKind::Gear { .. } => (true, true),
            RelationKind::RackPinion { .. } | RelationKind::Screw { .. } => (true, false),
            RelationKind::Linear { .. } => (false, false),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            RelationKind::Gear { .. } => "gear",
            RelationKind::RackPinion { .. } => "rack_pinion",
            RelationKind::Screw { .. } => "screw",
            RelationKind::Linear { .. } => "linear",
        }
    }
}

/// follower = slope × leader + offset. The offset is read from the written poses unless given, as
/// Onshape relations hold relative to where the parts are.
#[derive(Clone, Debug, PartialEq)]
pub struct Relation {
    pub id: RelationId,
    pub kind: RelationKind,
    pub leader: ValueRef,
    pub follower: ValueRef,
    pub offset: Option<f64>,
    pub suppressed: bool,
}

/// What an anchor is for, so fit rules can match it by kind and size: the four built-in kinds, or
/// one a part file declares (an open set, each declared kind its own type).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureKind {
    ExternalThread,
    InternalThread,
    Shaft,
    Bore,
    Declared(String),
}

impl FeatureKind {
    pub fn is_thread(&self) -> bool {
        matches!(self, FeatureKind::ExternalThread | FeatureKind::InternalThread)
    }
    pub fn name(&self) -> &str {
        match self {
            FeatureKind::ExternalThread => "external_thread",
            FeatureKind::InternalThread => "internal_thread",
            FeatureKind::Shaft => "shaft",
            FeatureKind::Bore => "bore",
            FeatureKind::Declared(n) => n,
        }
    }
}

/// A feature on a part's anchor: kind, nominal size (mm; M5 is 5) and, for a thread, its length
/// from the anchor's origin along its z.
#[derive(Clone, Debug, PartialEq)]
pub struct Feature {
    pub kind: FeatureKind,
    pub size: f64,
    pub depth: Option<f64>,
}

impl Feature {
    pub fn new(kind: FeatureKind, size: f64) -> Feature {
        Feature {
            kind,
            size,
            depth: None,
        }
    }
}

/// How much two threads a fit rule covers must engage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MinEngagement {
    /// At least this length (mm).
    Length(f64),
    /// The screw fills the internal thread's whole depth (the nut's, the tapped hole's).
    Full,
}

/// A thread engaged less than this many times its size, where a rule gives no minimum, is too
/// shallow (the shop's rule: one diameter).
pub const SHOP_THREAD_ENGAGEMENT: f64 = 1.0;

/// One side of a fit rule.
#[derive(Clone, Debug, PartialEq)]
pub enum FitMatch {
    /// Any occurrence of this part (any body when `body` is None).
    Part { doc: DocId, body: Option<u64> },
    /// An occurrence with an anchor carrying this feature (any size when `size` is None). When
    /// both sides match by feature, the two anchors must also share an axis there.
    Feature { kind: FeatureKind, size: Option<f64> },
}

/// Part-level intended contact, as SolidWorks groups fasteners: every pair it matches overlaps on
/// purpose (a bolt M5 in a tapped M5 hole), with no pair declarations.
#[derive(Clone, Debug, PartialEq)]
pub struct FitRule {
    pub id: u64,
    pub a: FitMatch,
    pub b: FitMatch,
    /// Least thread engagement; None: the shop's (`SHOP_THREAD_ENGAGEMENT` sizes) for threads.
    pub min_engagement: Option<MinEngagement>,
}

/// Declared intended contact between two occurrences (FastCAD's `fit`): a press fit, a bolt in a
/// tapped hole. Reported, never an error.
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    pub a: Path,
    pub b: Path,
}

/// A primitive constraint between two references. Each removes a known number of freedoms
/// (`removes`); joints are named bundles of these, and loose ones mix freely with joints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConstraintKind {
    /// Point on point.
    Coincident,
    /// Point A at `offset` along plane B's normal.
    PointOnPlane {
        offset: f64,
    },
    /// Point A on axis B.
    PointOnLine,
    /// Planes parallel, A at `offset` along B's normal. Same normals: flush; with `flip`: mate.
    PlanePlane {
        offset: f64,
    },
    /// Axes on one line.
    Concentric,
    /// Concentric, and A's origin at `offset` along B's axis (a bolt in a hole: concentric plus
    /// a mate of the head on the face).
    Insert {
        offset: f64,
    },
    /// Directions (axes or plane normals) parallel.
    Parallel,
    Perpendicular,
    /// The angle between two directions; ill-conditioned near 0 and π (use Parallel there).
    Angle {
        angle: f64,
    },
    /// Distance between two points; ill-conditioned near 0 (use Coincident there).
    Distance {
        distance: f64,
    },
    /// Frames equal: all six.
    Lock,
    /// A's x at `angle` about B's z, measured toward B's y (a joint's twist lock).
    Twist {
        angle: f64,
    },
}

impl ConstraintKind {
    /// Freedoms this constraint removes when independent.
    pub fn removes(self) -> usize {
        match self {
            ConstraintKind::Coincident => 3,
            ConstraintKind::PointOnPlane { .. } => 1,
            ConstraintKind::PointOnLine => 2,
            ConstraintKind::PlanePlane { .. } => 3,
            ConstraintKind::Concentric => 4,
            ConstraintKind::Insert { .. } => 5,
            ConstraintKind::Parallel => 2,
            ConstraintKind::Perpendicular
            | ConstraintKind::Angle { .. }
            | ConstraintKind::Distance { .. } => 1,
            ConstraintKind::Lock => 6,
            ConstraintKind::Twist { .. } => 1,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            ConstraintKind::Coincident => "coincident",
            ConstraintKind::PointOnPlane { .. } => "point_on_plane",
            ConstraintKind::PointOnLine => "point_on_line",
            ConstraintKind::PlanePlane { .. } => "plane_plane",
            ConstraintKind::Concentric => "concentric",
            ConstraintKind::Insert { .. } => "insert",
            ConstraintKind::Parallel => "parallel",
            ConstraintKind::Perpendicular => "perpendicular",
            ConstraintKind::Angle { .. } => "angle",
            ConstraintKind::Distance { .. } => "distance",
            ConstraintKind::Lock => "lock",
            ConstraintKind::Twist { .. } => "twist",
        }
    }
    /// The reference kinds each end accepts.
    pub fn accepts(self) -> [&'static [RefKind]; 2] {
        use RefKind::*;
        const PT: &[RefKind] = &[Point, Frame];
        const LINE: &[RefKind] = &[Axis, Frame];
        const PLANE: &[RefKind] = &[Plane, Frame];
        const DIR: &[RefKind] = &[Axis, Plane, Frame];
        const FRAME: &[RefKind] = &[Frame];
        match self {
            ConstraintKind::Coincident | ConstraintKind::Distance { .. } => [PT, PT],
            ConstraintKind::PointOnPlane { .. } => [PT, PLANE],
            ConstraintKind::PointOnLine => [PT, LINE],
            ConstraintKind::PlanePlane { .. } => [PLANE, PLANE],
            ConstraintKind::Concentric | ConstraintKind::Insert { .. } => [LINE, LINE],
            ConstraintKind::Parallel | ConstraintKind::Perpendicular | ConstraintKind::Angle { .. } => {
                [DIR, DIR]
            }
            ConstraintKind::Lock | ConstraintKind::Twist { .. } => [FRAME, FRAME],
        }
    }
}

/// A loose constraint, written in the assembly beside the joints.
#[derive(Clone, Debug, PartialEq)]
pub struct Constraint {
    pub id: u64,
    pub kind: ConstraintKind,
    pub a: End,
    pub b: End,
    /// A's frame turned half a turn about its x first: opposed normals or axes (a mate, not a
    /// flush). Chosen by the author, never by the solver.
    pub flip: bool,
    pub suppressed: bool,
}

impl Constraint {
    pub fn new(id: u64, kind: ConstraintKind, a: End, b: End) -> Constraint {
        Constraint {
            id,
            kind,
            a,
            b,
            flip: false,
            suppressed: false,
        }
    }
    pub fn flipped(mut self) -> Constraint {
        self.flip = !self.flip;
        self
    }
    pub fn end(&self, s: Side) -> &End {
        match s {
            Side::A => &self.a,
            Side::B => &self.b,
        }
    }
}

/// Which part of a joint a primitive constraint is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JointPart {
    /// The joint kind's n-th primitive (see `JointKind::bundle`).
    Prim(u8),
    /// A driven value.
    Drive(Freedom),
    /// A limit held at its bound.
    Limit(Freedom),
}

/// A primitive constraint's id: a loose one's own, or its place in a joint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConstraintId {
    Loose(u64),
    Joint { joint: JointId, part: JointPart },
}

impl ConstraintId {
    pub fn joint(self) -> Option<JointId> {
        match self {
            ConstraintId::Loose(_) => None,
            ConstraintId::Joint { joint, .. } => Some(joint),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Assembly {
    pub instances: Vec<Instance>,
    pub joints: Vec<Joint>,
    pub constraints: Vec<Constraint>,
    pub relations: Vec<Relation>,
    pub fits: Vec<Fit>,
    pub fit_rules: Vec<FitRule>,
    /// Blame order: a joint's, loose constraint's or relation's place in the document (by its
    /// raw id). Of a conflicting set the last is left out, and of implied rows the later is the
    /// redundant one; ids without a place come after, by id. Poses do not depend on it.
    pub rank: BTreeMap<u64, u64>,
}

/// Why a part could not give an anchor it declares; decided when the part was built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnchorFault {
    /// The entity the anchor is defined from no longer exists.
    EntityGone { entity: String },
    /// The entity is there but gives no frame (a zero-length axis, a face with no plane).
    Degenerate { entity: String },
}

/// A part as the solver and checks see it, made once per content hash.
#[derive(Clone, Debug, Default)]
pub struct PartInfo {
    pub hash: Option<ContentHash>,
    /// Anchors (frames) and role-named faces, edges and axes, by name.
    pub refs: BTreeMap<String, Result<Reference, AnchorFault>>,
    /// Features on named anchors (a thread, a bore), for fit rules.
    pub features: Vec<(String, Feature)>,
    pub mesh: Option<std::sync::Arc<crate::check::PartMesh>>,
}

/// What the solver reads from the project: built parts and subassembly documents.
pub trait Parts: Sync {
    fn part(&self, r: &PartRef) -> Option<&PartInfo>;
    fn assembly(&self, doc: &DocId) -> Option<&Assembly>;
}

/// An in-memory project: what tests use, and the shape the Forge wiring fills.
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub parts: BTreeMap<(DocId, u64), PartInfo>,
    pub assemblies: BTreeMap<DocId, Assembly>,
}

impl<T: Parts + ?Sized> Parts for &T {
    fn part(&self, r: &PartRef) -> Option<&PartInfo> {
        (**self).part(r)
    }
    fn assembly(&self, doc: &DocId) -> Option<&Assembly> {
        (**self).assembly(doc)
    }
}

impl<T: Parts + ?Sized + Send> Parts for std::sync::Arc<T> {
    fn part(&self, r: &PartRef) -> Option<&PartInfo> {
        (**self).part(r)
    }
    fn assembly(&self, doc: &DocId) -> Option<&Assembly> {
        (**self).assembly(doc)
    }
}

impl Parts for Catalog {
    fn part(&self, r: &PartRef) -> Option<&PartInfo> {
        self.parts.get(&(r.doc.clone(), r.body))
    }
    fn assembly(&self, doc: &DocId) -> Option<&Assembly> {
        self.assemblies.get(doc)
    }
}

impl PartRef {
    pub fn new(doc: &str, body: u64) -> PartRef {
        PartRef {
            doc: DocId(doc.into()),
            body,
            pin: None,
        }
    }
}

impl End {
    pub fn named(path: &[u64], anchor: &str) -> End {
        End {
            path: path.iter().map(|&i| InstanceId(i)).collect(),
            anchor: AnchorRef::Named(anchor.into()),
        }
    }
    pub fn local(path: &[u64], frame: Frame) -> End {
        End::reference(path, Reference::frame(frame))
    }
    pub fn reference(path: &[u64], r: Reference) -> End {
        End {
            path: path.iter().map(|&i| InstanceId(i)).collect(),
            anchor: AnchorRef::Local(r),
        }
    }
}

//! Typed results of a solve. No free text: each variant carries what a caller (or an agent) needs,
//! and each has one fixed severity.

use crate::model::{
    AnchorFault, ConstraintId, ContentHash, DocId, Freedom, InstanceId, JointId, PartRef, Path, RefKind,
    RelationId, Side,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Reported, nothing wrong (intended contact).
    Info,
    Warning,
    Error,
}

/// Why a contact is intended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Intent {
    /// The assembly's declared fit, by its index in `fits`.
    Fit { index: usize },
    /// A part-level fit rule, by id.
    Rule { rule: u32 },
}

/// Why a joint end gives no frame.
#[derive(Clone, Debug, PartialEq)]
pub enum AnchorReason {
    /// No instance with this id at this level of the path (index into the path).
    NoInstance { at: usize },
    /// The path's instance is suppressed.
    Suppressed { at: usize },
    /// The path goes on past a part, or stops at a subassembly.
    BadPath { at: usize },
    /// The part or subassembly document is not in the project.
    NoDocument { doc: DocId },
    /// The part declares no anchor of this name.
    NoAnchor,
    /// The part declares it, but its build could not resolve it.
    Part(AnchorFault),
}

/// Something a joint or relation constrains: what a conflict or redundancy names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Item {
    /// A primitive constraint: a loose one, or a part of the joint named beside it.
    Constraint {
        id: ConstraintId,
        joint: Option<JointId>,
    },
    Relation(RelationId),
    /// A grounded instance's fixed pose.
    Ground(InstanceId),
}

/// A typed edit that removes a diagnostic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fix {
    /// Set the joint value's param (its drive) to this value.
    SetDrive {
        joint: JointId,
        freedom: Freedom,
        value: f64,
    },
}

impl Item {
    pub fn of(id: ConstraintId) -> Item {
        Item::Constraint {
            id,
            joint: id.joint(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Diagnostic {
    /// A joint end that resolves to no frame. The joint is left out; its instances keep their
    /// written poses (or follow their other joints).
    BrokenAnchor {
        /// A loose constraint's id (None for a joint end).
        constraint: Option<ConstraintId>,
        joint: Option<JointId>,
        side: Side,
        anchor: String,
        reason: AnchorReason,
    },
    /// Consistent at the solution, but adds nothing: `rows` of `item`'s equations are implied by
    /// `implied_by`. A warning: real mechanisms are redundant in 3D.
    Redundant {
        item: Item,
        rows: usize,
        implied_by: Vec<Item>,
    },
    /// No pose meets all of `items`; every proper subset can be met. `residuals[i]` is item i's
    /// largest residual at the best attempt (mm, or rad × block size). The last item is left out
    /// of the solve and its instances keep their last good pose.
    Conflict {
        items: Vec<Item>,
        residuals: Vec<f64>,
    },
    /// No instance is grounded: each partition is held at its least instance's written pose.
    NoGround,
    /// A loop block that did not reach a solution within the iteration cap, though no conflict
    /// was proven. Its instances keep their last good pose.
    NotConverged {
        block: usize,
        items: Vec<Item>,
        residual: f64,
    },
    /// A value outside its joint's limits: a drive, a written pose, or a loop that cannot close
    /// inside them. The value is held at the nearer limit when the joint is in a tree.
    LimitViolated {
        joint: JointId,
        freedom: Freedom,
        value: f64,
        range: (f64, f64),
        /// The edit that removes it, where a drive caused it.
        fix: Option<Fix>,
    },
    /// A drive or limit on a freedom the joint does not have, or a drive on a ball joint.
    BadFreedom {
        joint: JointId,
        freedom: Freedom,
    },
    /// A relation between values of the wrong dimension, or on a missing joint.
    BadRelation {
        relation: RelationId,
    },
    DuplicateInstance {
        id: InstanceId,
    },
    DuplicateJoint {
        id: JointId,
    },
    /// A reference of a kind the constraint cannot take (a point where a plane is needed).
    WrongReference {
        constraint: Option<ConstraintId>,
        joint: Option<JointId>,
        side: Side,
        anchor: String,
        want: &'static [RefKind],
        found: RefKind,
    },
    DuplicateConstraint {
        id: u32,
    },
    /// Both ends on the same reference path.
    SelfJoint {
        constraint: Option<ConstraintId>,
        joint: Option<JointId>,
    },
    /// A subassembly that contains itself.
    CyclicAssembly {
        doc: DocId,
    },
    /// A part whose built hash differs from its pin.
    PartChanged {
        part: PartRef,
        pinned: ContentHash,
        found: Option<ContentHash>,
    },
    /// Two occurrences share volume (or their meshes cross, unconfirmed): a warning, since contact
    /// is sometimes meant. A fit or fit rule moves the pair to `IntendedContact`.
    Interference {
        a: Path,
        b: Path,
        /// Exact shared volume (mm³), where kernel solids confirmed it.
        volume: Option<f64>,
        confirmed: bool,
    },
    /// Contact a fit or fit rule says is meant.
    IntendedContact {
        a: Path,
        b: Path,
        by: Intent,
    },
    /// A diagnostic from inside a subassembly occurrence.
    Inside {
        path: Path,
        diagnostic: Box<Diagnostic>,
    },
}

impl Diagnostic {
    pub fn severity(&self) -> Severity {
        match self {
            Diagnostic::Redundant { .. }
            | Diagnostic::PartChanged { .. }
            | Diagnostic::Interference { .. } => Severity::Warning,
            Diagnostic::IntendedContact { .. } => Severity::Info,
            Diagnostic::Inside { diagnostic, .. } => diagnostic.severity(),
            _ => Severity::Error,
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Diagnostic::BrokenAnchor { .. } => "BrokenAnchor",
            Diagnostic::Redundant { .. } => "Redundant",
            Diagnostic::Conflict { .. } => "Conflict",
            Diagnostic::NoGround => "NoGround",
            Diagnostic::NotConverged { .. } => "NotConverged",
            Diagnostic::LimitViolated { .. } => "LimitViolated",
            Diagnostic::BadFreedom { .. } => "BadFreedom",
            Diagnostic::BadRelation { .. } => "BadRelation",
            Diagnostic::DuplicateInstance { .. } => "DuplicateInstance",
            Diagnostic::DuplicateJoint { .. } => "DuplicateJoint",
            Diagnostic::DuplicateConstraint { .. } => "DuplicateConstraint",
            Diagnostic::WrongReference { .. } => "WrongReference",
            Diagnostic::SelfJoint { .. } => "SelfJoint",
            Diagnostic::CyclicAssembly { .. } => "CyclicAssembly",
            Diagnostic::PartChanged { .. } => "PartChanged",
            Diagnostic::Inside { .. } => "Inside",
            Diagnostic::Interference { .. } => "Interference",
            Diagnostic::IntendedContact { .. } => "IntendedContact",
        }
    }
}

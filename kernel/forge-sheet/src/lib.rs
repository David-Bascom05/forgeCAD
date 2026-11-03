//! Sheet metal: a wall-and-joint graph in folded design terms, each wall's trim held in its own
//! developed coordinates, and both the folded and the flat solid built from it directly as
//! ordinary bodies, with no booleans and nothing recognised. Design: research/sheet-metal/.
pub mod build;
pub mod checks;
pub mod contour;
pub mod error;
pub mod graph;
pub mod hardware;
pub mod hem;
pub mod pattern;
pub mod region;
pub mod relief;
pub mod roles;
pub mod shop;

pub use build::{
    BendPlace, Built, CornerMade, Layout, Map, Place2, WallPlace, build, build_with, setback_of,
};
pub use checks::check;
pub use contour::{ContourError, ContourMade, Hand};
pub use error::{
    CutId, End, FeatureId, JointId, JointKindName, Pinch, Seam, SheetError, SheetWarning, Side, Strip,
    TrimCurveKind, TrimRef, Violation, WallId,
};
pub use graph::{Corner, CornerKind, Cut, Gauge, Joint, JointKind, Position, Sheet, Wall};
pub use hardware::{Hardware, HardwareKind, Mount, Source, placeholder_catalogue};
pub use hem::{GAP_FLOOR, HemError, HemJoint, HemKind, HemName, HemRow, HemSpec, HemTable};
pub use pattern::{BendLine, Curve2, NOTES_LAYER, Pattern, bend_note, dxf, dxf_noted};
pub use relief::{
    BendReliefRule, Corner3Rule, Corner3Shape, CornerRule, CornerShape, DepthRef, Placement, Relief,
    ReliefKind, ReliefMade, ReliefRule,
};
pub use roles::{Roles, Skin};
pub use shop::{AllowanceRow, BendRule, KRow, Process, RuleError, Scaled, ShopProfile, Stock};

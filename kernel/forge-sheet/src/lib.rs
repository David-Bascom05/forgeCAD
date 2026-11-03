//! Sheet metal: a wall-and-joint graph in folded design terms, each wall's trim held in its own
//! developed coordinates, and both the folded and the flat solid built from it directly as
//! ordinary bodies, with no booleans and nothing recognised. Design: research/sheet-metal/.
pub mod build;
pub mod error;
pub mod graph;
pub mod region;
pub mod shop;

pub use build::{Map, build};
pub use error::{
    CutId, End, FeatureId, JointId, JointKindName, Pinch, SheetError, SheetWarning, Side, Strip,
    TrimCurveKind, TrimRef, Violation, WallId,
};
pub use graph::{Corner, CornerKind, Cut, Gauge, Joint, JointKind, Sheet, Wall};
pub use shop::{AllowanceRow, BendRule, KRow, RuleError, Scaled, ShopProfile, Stock};

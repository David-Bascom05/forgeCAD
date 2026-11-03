//! Sheet metal: a wall-and-joint graph in folded design terms, each wall's trim held in its own
//! developed coordinates, and both the folded and the flat solid built from it directly as
//! ordinary bodies, with no booleans and nothing recognised. Design: research/sheet-metal/.
pub mod error;
pub mod region;
pub mod shop;

pub use error::{
    CutId, End, FeatureId, JointId, JointKindName, Pinch, SheetError, SheetWarning, Side, Strip, TrimCurveKind,
    TrimRef, Violation, WallId,
};
pub use shop::{AllowanceRow, BendRule, KRow, RuleError, Scaled, ShopProfile, Stock};

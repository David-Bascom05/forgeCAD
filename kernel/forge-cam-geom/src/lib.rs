//! forge-cam-geom: the proven core of ForgeCAD's CAM.
//!
//! - [`big`], [`real`]: exact numbers. Inputs are f64 (exact dyadic rationals); constructed values
//!   are an expression DAG whose sign is decided exactly (a filter, then big-float intervals to
//!   the BFMSS separation bound).
//! - [`region`]: closed line/arc loops and regions, validated exactly.
//! - [`offset`]: the exact offset of a region, R ⊖ B_r, through any change of topology.
//! - [`tool`]: cutters and holders as exact revolved line/arc profiles, and their signed distance
//!   in certified intervals.
//! - [`check`]: the certified tool–part clearance checker: branch and bound over the part's faces
//!   and the motion, verdicts Clear (proven), Hit (a real witness) or Undecided (refused).
//!
//! The crate depends on nothing, so the whole of it is the Aeneas target (L4). It is written in
//! that style: loops over indices, no closures in the algorithms, no iterator chains, no u128.
// Arithmetic is spelled as plain methods (Aeneas translates them; operator traits it does not).
#![allow(clippy::should_implement_trait, clippy::manual_map, clippy::manual_div_ceil)]
// `!(x > 0.0)` is deliberate: it refuses NaN as well.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod arr;
pub mod big;
pub mod check;
pub mod dual;
pub mod ival;
pub mod offset;
pub mod real;
pub mod region;
pub mod slab;
pub mod tool;

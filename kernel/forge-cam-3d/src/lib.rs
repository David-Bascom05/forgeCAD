//! forge-cam-3d: 3-axis finishing.
//!
//! - [`scallop`]: closed-form cusp heights for flat, ball and bull-nose cutters, with certified
//!   interval enclosures, and the inverse: the largest stepover proven to leave at most a target
//!   cusp (research/cam/toolpath-quality/surface-finish.md §1, statements S1 and S2).
//!
//! Written in forge-cam-geom's style (index loops, no closures in the algorithms) so that it
//! stays an Aeneas target.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod scallop;

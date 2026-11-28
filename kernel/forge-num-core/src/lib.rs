//! Exact naturals, signed integers and dyadic rationals used by the geometry kernel.
//! Keep this crate independent of higher arithmetic so enabling exact reals for another
//! consumer preserves the kernel's specialized vector growth.

#![allow(clippy::should_implement_trait, clippy::manual_div_ceil, clippy::manual_map)]
#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod dy;
pub mod int;
pub mod nat;

pub use dy::Dy;
pub use int::Big;
pub use nat::Nat;

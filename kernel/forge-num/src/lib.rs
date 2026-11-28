//! forge-num: the numbers every exact part of ForgeCAD shares, in one place.
//!
//! - [`nat`]: naturals of any size (base 2^32 limbs).
//! - [`int`]: [`Big`], a signed integer held inline as an `i128` while it is under 2^126 (the
//!   common case) and as a [`Nat`] past it.
//! - [`dy`]: [`Dy`], an exact dyadic `m · 2^e`: exact `+ − ×`, and the two roundings to f64 its
//!   callers pin (nearest-or-adjacent for output).
//! - `ival` (feature `real`): `Ival`, f64 intervals rounded outward, the whole line where
//!   undefined: the filter in front of every exact sign, and the clearance checker's arithmetic.
//! - `real` (feature `real`): exact reals, an expression DAG over exact dyadic inputs with the
//!   operations + − × ÷ √, whose sign is decided exactly (a filter, exact rational and one-radical
//!   routes, then big-float intervals to the BFMSS separation bound). Its literals are its own normalised
//!   dyadic, `real::Dy`.
//!
//! The integer and dyadic types come from forge-num-core. CAM and copper pours enable
//! the higher arithmetic here without changing how the shared integer operations compile.
#![allow(clippy::should_implement_trait, clippy::manual_div_ceil, clippy::manual_map)]
// `!(x > 0.0)` is deliberate: it refuses NaN as well.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub use forge_num_core::{Big, Dy, Nat, dy, int, nat};
#[cfg(feature = "real")]
pub mod ival;
#[cfg(feature = "real")]
pub mod real;

#[cfg(feature = "real")]
pub use ival::Ival;

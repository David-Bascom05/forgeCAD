//! forge-num: the numbers every exact part of ForgeCAD shares, in one place.
//!
//! - [`nat`]: naturals of any size (base 2^32 limbs).
//! - [`int`]: [`Big`], a signed integer held inline as an `i128` while it is under 2^126 (the
//!   common case) and as a [`Nat`] past it.
//! - [`dy`]: [`Dy`], an exact dyadic `m · 2^e`: exact `+ − ×`, and the two roundings to f64 its
//!   callers pin (nearest-or-adjacent for output).
//!
//! The crate depends on nothing, so the whole of it can be an Aeneas target (L4). Speed comes
//! first: where two implementations differed, the faster one is kept, measured. Within that it is
//! written in the style Aeneas takes where that costs nothing: loops over indices, arithmetic as
//! plain methods (Aeneas translates them; operator traits it does not), and the machine-integer
//! builtins it needs named once, in [`int`]'s helpers.
#![allow(clippy::should_implement_trait, clippy::manual_div_ceil)]

pub mod dy;
pub mod int;
pub mod nat;

pub use dy::Dy;
pub use int::Big;
pub use nat::Nat;

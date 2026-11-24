//! forge-turn: turning on the exact part.
//!
//! - [`profile`]: the part's half-profile in the (ρ, z) half-plane, exact in kind.
//! - [`insert`]: ISO inserts and holders, as exact outlines and the slab model of their body.
//! - [`check`]: the certified 2D clearance of a tool moving in the half-plane.
//! - [`machine`]: the lathe (CAM-MACHINE's definition format) and the setup's placement (G5).
//! - [`ops`]: the lathe setup and ops: face, rough, finish (exact nose-radius compensation),
//!   groove, part-off, thread, drill on the centre line; the in-process stock; the leftover.
//! - [`post`]: lathe dialects (diameter mode), exact quantisation, a reader, the round trip.
#![allow(clippy::too_many_arguments)]
// `!(x > 0.0)` is deliberate: it refuses NaN as well.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod check;
pub mod insert;
pub mod machine;
pub mod ops;
pub mod post;
pub mod profile;
pub mod stair;

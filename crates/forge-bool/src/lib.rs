//! Boolean operations on ForgeCAD solids.

pub mod boolean;
pub mod classify;
pub mod ssi;

pub use boolean::{BoolError, Op, boolean, boolean_into, phase_times};
pub use classify::{Index, Where};

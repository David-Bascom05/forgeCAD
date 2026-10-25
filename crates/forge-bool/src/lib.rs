//! Boolean operations on ForgeCAD solids.

pub mod boolean;
pub mod classify;
pub mod ssi;

pub use boolean::{BoolError, Op, boolean};
pub use classify::{Index, Where};

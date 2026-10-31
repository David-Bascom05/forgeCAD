//! Boolean operations on ForgeCAD solids.

pub mod boolean;
pub mod classify;
pub mod imprint;
pub mod nary;
pub mod shape;
pub mod ssi;

pub use boolean::{BoolError, Op, boolean, boolean_into, boolean_unlinked, phase_times};
pub use classify::{Index, Where};
pub use nary::{Batch, CpuBatch, nary, nary_phase_times, nary_with};
pub use shape::{Member, Seg2, Shape};

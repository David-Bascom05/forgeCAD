//! Boolean operations on ForgeCAD solids.

/// Its statements run only with the `decisions` feature (the decision log, `decisions.rs`); off,
/// they are not compiled at all.
macro_rules! decided {
    ($($t:tt)*) => {
        #[cfg(feature = "decisions")]
        {
            $($t)*
        }
    };
}

pub mod boolean;
pub mod classify;
#[cfg(feature = "decide_once")]
pub mod decide_once;
#[cfg(feature = "decide_once_curved")]
pub mod decide_once_curved;
#[cfg(all(feature = "decide_once_curved", any(test, feature = "curved_reference")))]
pub mod decide_once_curved_ref;
#[cfg(feature = "decide_once")]
mod decide_once_row;
#[cfg(feature = "decisions")]
pub mod decisions;
pub mod imprint;
pub mod nary;
pub mod shape;
pub mod ssi;
pub mod unify;

pub use boolean::{
    BoolError, FaceRef, Lost, Op, Unhandled, boolean, boolean_into, boolean_unlinked, phase_times,
};
pub use classify::{Index, Where};
pub use nary::{Batch, CpuBatch, nary, nary_phase_times, nary_with};
pub use shape::{Member, Seg2, Shape};
pub use unify::unify;

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

mod apex;
/// This crate's Cargo features and whether this build has each: a result cache keyed by the
/// sources alone would hand one build's answers to a build with other features
/// (forge-lang's `store::key` folds them in).
pub const FEATURES: &[(&str, bool)] = &[
    ("decisions", cfg!(feature = "decisions")),
    ("decide_once", cfg!(feature = "decide_once")),
    ("imprint_column", cfg!(feature = "imprint_column")),
];

pub mod boolean;
#[cfg(any(test, feature = "decisions"))]
pub mod carrier_key;
pub mod classify;
#[cfg(feature = "decide_once")]
pub mod decide_once;
#[cfg(feature = "decisions")]
pub mod decisions;
#[cfg(any(test, feature = "decisions"))]
pub mod edge_screen;
pub mod imprint;
pub mod nary;
#[cfg(any(test, feature = "decisions"))]
pub mod rim_cert;
pub mod shape;
// (debug builds only: release compiles none of it, so its code and layout are as before)
#[cfg(debug_assertions)]
pub mod size;
pub mod ssi;
pub mod unify;

pub use boolean::{
    BoolError, FaceRef, Lost, Op, Unhandled, boolean, boolean_into, boolean_unlinked, phase_times,
};
pub use classify::{Index, Where};
pub use nary::{Batch, CpuBatch, nary, nary_phase_times, nary_with};
pub use shape::{Member, Seg2, Shape};
pub use unify::unify;

// last, so that with the feature off every line above is the line it was
#[cfg(feature = "certify")]
pub mod certify;

//! Parametric 2D sketches: points, lines, circles and arcs held in place by
//! geometric constraints, a solver that moves them as little as possible to
//! satisfy those constraints, degree-of-freedom analysis, and extraction of
//! the closed regions as profiles for modelling operations.

mod dual;
pub mod profile;
pub mod sketch;
pub mod solve;

pub use profile::{ExtractError, profiles};
pub use sketch::{Constraint, ConstraintId, Entity, EntityId, Sketch, SketchError};
pub use solve::{Analysis, SolveReport, Status};

#[cfg(test)]
mod tests;

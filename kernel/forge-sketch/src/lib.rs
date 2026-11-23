//! Parametric 2D sketches: points, lines, circles and arcs held in place by
//! geometric constraints, a solver that moves them as little as possible to
//! satisfy those constraints, degree-of-freedom analysis, and extraction of
//! the closed regions as profiles for modelling operations; and the part's edges, points and
//! axes laid on a sketch's plane as the entities that hold their images (`project`).

mod dual;
pub mod profile;
pub mod project;
pub mod sketch;
pub mod solve;
mod winding;

pub use profile::{ExtractError, profiles, profiles_among};
pub use sketch::{Constraint, ConstraintId, Entity, EntityId, Sketch, SketchError};
pub use solve::{Analysis, SolveReport, Status};

#[cfg(test)]
mod tests;

//! Triangulation for ForgeCAD meshes: exact 2D predicates and a constrained
//! Delaunay triangulator for faces laid out in their parameter plane.

pub mod cdt;
pub mod predicates;

pub use cdt::{CdtError, Triangulation, triangulate};
pub use predicates::{incircle, orient2d};

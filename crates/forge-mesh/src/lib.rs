//! Meshes for ForgeCAD: exact 2D predicates, a constrained Delaunay
//! triangulator, tessellation of B-rep bodies into watertight triangle meshes,
//! and STL output.

pub mod cdt;
pub mod mesh;
pub mod predicates;
pub mod stl;
pub mod tessellate;

pub use cdt::{CdtError, Triangulation, triangulate};
pub use mesh::TriMesh;
pub use predicates::{incircle, orient2d};
pub use stl::to_binary_stl;
pub use tessellate::{Quality, TessError, tessellate};

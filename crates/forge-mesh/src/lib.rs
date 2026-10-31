//! Meshes for ForgeCAD: a constrained Delaunay
//! triangulator, tessellation of B-rep bodies into watertight triangle meshes,
//! and STL output.

pub mod cdt;
pub mod mesh;
pub mod stl;
pub mod tessellate;

pub use cdt::{CdtError, Triangulation, triangulate};
pub use mesh::TriMesh;
pub use stl::to_binary_stl;
pub use tessellate::{Quality, TessError, tessellate};

//! Meshes for ForgeCAD: a constrained Delaunay
//! triangulator, tessellation of B-rep bodies into watertight triangle meshes,
//! and mesh files: STL (binary and ASCII), OBJ and 3MF, written and read back.

pub mod cdt;
pub mod mesh;
pub mod obj;
pub mod read;
pub mod scene;
pub mod stl;
pub mod tessellate;
pub mod threemf;
pub mod zip;

pub use cdt::{CdtError, Fault, Triangulation, triangulate};
pub use mesh::TriMesh;
pub use obj::{ObjOptions, read_obj, to_obj};
pub use scene::{Material, MeshCheck, Object, Scene, Solids, body_objects};
pub use stl::{read_stl, scene_ascii_stl, scene_binary_stl, to_binary_stl};
pub use tessellate::{Quality, TessError, tessellate, tessellate_tolerant};
pub use threemf::{read_3mf, to_3mf};

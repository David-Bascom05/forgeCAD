//! Boundary representation for ForgeCAD solids: which faces, edges and
//! vertices exist, how they connect, and which exact surface or curve each one
//! lies on. Every entity carries a stable `Name` derived from how it was made.

pub mod body;
pub mod check;
pub mod mass;
pub mod name;
pub mod uv;

pub use body::{
    Body, Coedge, CoedgeId, Edge, EdgeId, Face, FaceId, Loop, LoopId, Shell, ShellId, Vertex, VertexId,
};
pub use check::{Problem, certify, check, genus};
pub use name::Name;

//! Boundary representation for ForgeCAD solids: which faces, edges and
//! vertices exist, how they connect, and which exact surface or curve each one
//! lies on. Every entity carries a stable `Name` derived from how it was made.

/// This crate's Cargo features and whether this build has each: a result cache keyed by the
/// sources alone would hand one build's answers to a build with other features
/// (forge-lang's `store::key` folds them in).
pub const FEATURES: &[(&str, bool)] = &[("decisions", cfg!(feature = "decisions"))];

pub mod body;
pub mod check;
pub mod chords;
pub mod extent;
pub mod mass;
pub mod name;
pub mod small;
pub mod topo;
pub mod topo_shell;
pub mod topo_edges;
pub mod topo_euler;
pub mod uv;

pub use body::{
    Body, Coedge, CoedgeId, Coedges, Edge, EdgeId, Face, FaceId, Joins, Loop, LoopId, Shell, ShellId,
    Unlinked, Vertex, VertexId,
};
pub use check::{
    Entity, Problem, certify, changed_faces, check, check_against, edge_off_face, genus, sides_of,
};
pub use name::Name;

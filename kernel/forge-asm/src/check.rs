//! Interference and clearance between placed instances.

use forge_math::Point3;

/// A part's cached mesh in part coordinates, and how far it may lie from the true surface.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartMesh {
    pub positions: Vec<Point3>,
    pub triangles: Vec<[u32; 3]>,
    pub tolerance: f64,
}

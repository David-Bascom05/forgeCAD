//! The kernel's one tolerance rule. Lengths are millimetres.
//!
//! Coordinates are stored as plain f64. These values decide when two computed
//! things are "the same": a vertex on a curve, two edges meeting, a curve
//! lying on a surface. They are fixed, not per-model, so the same input always
//! makes the same decisions.

/// Half the side of the cube every model must fit in: 1 km across, centred on the origin.
pub const MODEL_HALF_SIZE: f64 = 500_000.0;

/// Two points closer than this are the same point (10 nm).
pub const LINEAR: f64 = 1e-5;

/// Two directions closer than this (radians) are the same direction. LINEAR over
/// the model size, so an angle error never moves a far point more than LINEAR.
pub const ANGULAR: f64 = 1e-11;

/// Smallest radius or length a feature may have; anything shorter is a point. The same number
/// as LINEAR: there is one tolerance.
pub const MIN_LENGTH: f64 = LINEAR;

pub fn same_point(a: crate::Point3, b: crate::Point3) -> bool {
    a.distance_sq(b) <= LINEAR * LINEAR
}

pub fn is_zero_length(l: f64) -> bool {
    l.abs() <= LINEAR
}

pub fn in_model_box(p: crate::Point3) -> bool {
    p.x.abs() <= MODEL_HALF_SIZE && p.y.abs() <= MODEL_HALF_SIZE && p.z.abs() <= MODEL_HALF_SIZE
}

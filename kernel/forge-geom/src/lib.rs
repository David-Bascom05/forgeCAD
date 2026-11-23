//! Exact geometry for the ForgeCAD kernel: analytic curves and surfaces, and
//! their intersections. Each shape is stored by its defining data (a cylinder
//! is an axis frame and a radius), never as an approximation.

/// This crate's Cargo features and whether this build has each: a result cache keyed by the
/// sources alone would hand one build's answers to a build with other features
/// (forge-lang's `store::key` folds them in).
pub const FEATURES: &[(&str, bool)] = &[("decisions", cfg!(feature = "decisions"))];

pub mod blend;
pub mod budget;
pub mod corner;
pub mod curve;
pub mod intcurve;
pub mod intersect;
pub mod loftmeet;
pub mod meet;
mod meet_gen;
pub mod ruled;
pub mod screw;
pub mod sheet;
pub mod skinned;
pub mod smooth;
pub mod spline;
pub mod surface;
#[doc(hidden)]
pub mod testing;
pub mod tube;
pub mod varblend;
pub mod wrap;

pub use curve::{Circle, Curve, CurveGeom, Domain1, Ellipse, Hyperbola, Line, Parabola};
pub use surface::{Cone, Cylinder, Domain2, Plane, ShapeBounds, Sphere, Surface, SurfaceGeom, Torus};

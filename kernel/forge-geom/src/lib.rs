//! Exact geometry for the ForgeCAD kernel: analytic curves and surfaces, and
//! their intersections. Each shape is stored by its defining data (a cylinder
//! is an axis frame and a radius), never as an approximation.

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

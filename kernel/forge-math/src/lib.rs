//! Numeric foundation of the ForgeCAD kernel. Everything here gives bit-identical
//! results on every target: plain IEEE arithmetic, no fused multiply-add, no
//! platform math library (see `dmath`).

// the wasm heap
#[cfg(target_family = "wasm")]
use forge_alloc as _;

pub mod dmath;
pub mod frame;
pub mod hash;
pub mod interval;
pub mod linalg;
pub mod par;
pub mod poly;
pub mod predicates;
pub mod rng;
pub mod tol;
pub mod vec;

pub use frame::Frame;
pub use interval::{Box2, Box3, Interval};
pub use vec::{Point2, Point3, UnitVec3, Vec2, Vec3};

//! The 2D drafting kernel: the geometry under an AutoCAD-like editor's commands.
//!
//! - `Curve`: lines, rays, xlines, circles, arcs, ellipses and elliptical arcs, polylines (line
//!   and arc segments), a SPLINE's NURBS, and the exact offset of an ellipse or spline. Evaluated
//!   in closed form (`point`, `deriv`, `tangent`, `length`, `param_at_length`, `closest`,
//!   `feet`, `bbox`, `sub`, `split_at`, `reversed`, `transformed`); splines and offset curves
//!   numerically where nothing closed exists.
//! - `entity`: `read_entity`/`from_entity` (in WCS XY; OCS -Z by the arbitrary axis; anything
//!   else not editable), `curve_of` (2D POLYLINEs with their vertices), `to_data`, `write_curve`
//!   and `apply_to` (the natural entity type), `transform_data` (curves, TEXT with MIRRTEXT,
//!   MTEXT, INSERT, points).
//! - `intersect`: points (tangent or crossing) and overlaps, with apparent ones on extensions.
//! - `ops`: TRIM, EXTEND, BREAK, JOIN, LENGTHEN. `offset`: OFFSET. `fillet`: FILLET, CHAMFER.
//!   `xform`: MOVE, ROTATE, SCALE, MIRROR, ARRAY maps and STRETCH.
//! - `snap`: object snaps, ranked. `commit`: results into a drawing through `edit`.
//!
//! Everything is deterministic: plain IEEE arithmetic and `forge_math::dmath` for the
//! transcendental functions, the same bits native and in wasm. Tolerances come from the size of
//! the geometry at hand (`Tol`).

pub mod commit;
pub mod curve;
pub mod entity;
pub mod fillet;
pub mod geom;
pub mod intersect;
pub mod nurbs;
pub mod offset;
pub mod ops;
pub mod snap;
pub mod xform;

#[cfg(test)]
mod determinism;
#[cfg(test)]
mod e2e;

pub use curve::{Arc, Curve, Ellipse, Offset, Poly, Seg};
pub use geom::{Affine, Bbox, Tol, V2, v2};
pub use nurbs::Nurbs;

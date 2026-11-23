//! Sheets: surface bodies, no material, from a sketch's chains extruded along its plane's normal
//! (`extrude_sheet`) or turned about an axis (`revolve_sheet`). A crate of its own beside
//! forge-ops, whose extrude and revolve builders it drives (the extrude's through a sink of its
//! own), so that the solid operations are compiled as they were. And `thicken`, the kernel's one
//! thicken (of a solid's faces or a sheet's, a thin wall's too): a sheet's rim, creased rims
//! mitred, spline walls carried; `forge_ops::thicken` is the thicken as it was before sheets, kept
//! unchanged with forge-ops and no longer used by the model.

mod chain;
pub mod extrude;
pub mod revolve;
pub mod thicken;

pub use extrude::extrude_sheet;
pub use revolve::revolve_sheet;
pub use thicken::{ThickenError, thicken};

//! Thermoforming: the user models the part's shape, and the rest is generated from it. The
//! shape's surface is the part's inner skin (a male mould) or its outer one (female); the formed
//! part is that surface offset by the sheet's thickness, trimmed, with its flange; the mould is the
//! shape grown by the shrink, on a base or in a block, vented. The shop's checks and the wall's
//! thickness run on every build. Sources: research/thermoform/.
pub mod build;
pub mod checks;
pub mod field;
pub mod fit;
pub mod material;
pub mod prepare;
pub mod scale;
pub mod sdf;
pub mod simulated;
pub mod stretch;

pub use build::{FormError, Formed, Report, Spec, Tool, analyse, form, plane_frame};
pub use checks::{Violation, check};
pub use field::{Field, Region, Valley};
pub use material::{
    Curve, DataError, Fit, FormShop, LawAt, Machine, Material, PerT, from_record, placeholder, shop_of,
    unsourced,
};
pub use prepare::{Given, Prepared, prepare};
pub use scale::{Unscalable, scaled};
pub use simulated::{
    MeshMass, OnTool, PartMesh, Simulated, ToolSide, WallField, blank, mesh_mass, part_mesh, simulated,
    simulated_from, simulated_on, simulated_part,
};
pub use stretch::{Clamp, Law, Process, Step, Stretch, Surface, stretch};

#[cfg(test)]
mod bench;
#[cfg(test)]
mod tests;

//! Thermoforming: a sheet formed over a modelled mould. The mould's surface is the part's inner
//! skin (male tool) or outer one (female); the part is that surface offset by the sheet's thickness,
//! trimmed, with its flange; the tool is the mould grown by the shrink, on a base or in a block,
//! vented. The shop's checks run on every build. Sources: research/thermoform/.
pub mod build;
pub mod checks;
pub mod data;
pub mod field;
pub mod material;
pub mod scale;
pub mod stretch;

pub use build::{FormError, Formed, Report, Side, Spec, analyse, form, plane_frame};
pub use checks::{Violation, check};
pub use field::{Field, Region, Valley};
pub use material::{Curve, DataError, Fit, FormShop, LawAt, Machine, Material, PerT, from_data, placeholder};
pub use scale::{Unscalable, scaled};
pub use stretch::{Law, Process, Stretch, stretch};

#[cfg(test)]
mod bench;
#[cfg(test)]
mod tests;

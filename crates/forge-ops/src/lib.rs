//! Modelling operations: each takes geometry plus a feature id from the
//! history and returns a new solid whose entities are named from that id.

pub mod extrude;
pub mod profile;
pub mod revolve;

pub use extrude::{ExtrudeError, extrude};
pub use profile::{Profile, ProfileError, ProfileLoop, Segment};
pub use revolve::{Axis2, RevolveError, revolve};

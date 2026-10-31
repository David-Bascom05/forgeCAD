//! Modelling operations: each takes geometry plus a feature id from the
//! history and returns a new solid whose entities are named from that id.

pub mod chamfer;
pub mod corner;
pub mod draft;
pub mod extrude;
pub mod fillet;
pub mod profile;
pub mod rebuild;
pub mod revolve;
mod selfx;
pub mod shell;

pub use chamfer::{ChamferError, ChamferSize, chamfer};
pub use draft::{DraftError, draft};
pub use extrude::{ExtrudeError, extrude};
pub use fillet::{FilletEdge, FilletError, Radius, fillet};
pub use profile::{Profile, ProfileError, ProfileLoop, Segment};
pub use rebuild::RebuildError;
pub use revolve::{Axis2, RevolveError, revolve};
pub use shell::{ShellError, shell};

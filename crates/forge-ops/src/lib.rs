//! Modelling operations: each takes geometry plus a feature id from the
//! history and returns a new solid whose entities are named from that id.

pub mod chamfer;
pub mod corner;
pub mod direct;
pub mod draft;
pub mod extrude;
pub mod fillet;
pub mod glyph;
pub mod loft;
pub mod profile;
pub mod rebuild;
pub mod revolve;
mod selfx;
pub mod shell;
mod skin;
pub mod strip;
pub mod sweep;
pub mod thicken;

pub use chamfer::{ChamferError, ChamferSize, chamfer};
pub use direct::{DirectError, delete_faces, move_faces, offset_faces, replace_face};
pub use draft::{DraftError, draft};
pub use extrude::{ExtrudeError, extrude};
pub use fillet::{FilletEdge, FilletError, FilletOptions, Radius, fillet, fillet_with, law_start};
pub use glyph::{GlyphError, GlyphSeg, Place, glyph};
pub use loft::{LoftError, LoftGuides, LoftKind, loft, loft_bounds};
pub use profile::{Profile, ProfileError, ProfileLoop, Segment};
pub use rebuild::RebuildError;
pub use revolve::{Axis2, RevolveError, revolve};
pub use shell::{ShellError, shell};
pub use strip::{face_fillet, full_round};
pub use sweep::{
    CoilError, Handed, PathSeg, Stretch, SweepError, Winding, Wound, coil, coil_motion, sweep,
    sweep_stretches,
};
pub use thicken::{ThickenError, thicken};

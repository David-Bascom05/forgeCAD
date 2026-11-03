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
pub mod roles;
mod selfx;
pub mod shell;
mod skin;
pub mod strip;
pub mod sweep;
pub mod taper;
pub mod thicken;
mod wavefront;

pub use chamfer::{ChamferCorner, ChamferError, ChamferSize, chamfer, chamfer_with};
pub use direct::{DirectError, Rounded, delete_faces, move_faces, offset_faces, replace_face};
pub use draft::{DraftError, draft};
pub use extrude::{ExtrudeError, extrude, extrude_unlinked};
pub use fillet::{
    FilletEdge, FilletError, FilletOptions, Radius, fillet, fillet_kind, fillet_with, law_starts,
};
pub use glyph::{GlyphError, GlyphSeg, Place, glyph};
pub use loft::{Apex, LoftError, LoftGuides, LoftKind, LoftOptions, loft, loft_and_bounds, loft_bounds};
pub use profile::{Profile, ProfileError, ProfileLoop, Segment, Touch};
pub use rebuild::RebuildError;
pub use revolve::{Axis2, RevolveError, revolve, revolve_cutter};
pub use shell::{ShellError, offset_solid, shell};
pub use strip::{face_fillet, full_round};
pub use sweep::{
    CoilError, Handed, PathSeg, Stretch, SweepError, Winding, Wound, coil, coil_motion, sweep,
    sweep_stretches,
};
pub use taper::{TaperError, tapered};
pub use thicken::{ThickenError, thicken};

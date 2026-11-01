//! STEP, ISO 10303: part 21 files of the AP203, AP214 and AP242 boundary-representation entities.
//! Written from the standard's definitions. `import` also takes OpenCascade's `.brep` and IGES solids.

mod brep;
pub use brep::{BrepTopo, Used};
pub mod fit;
mod heal;
mod iges;
pub mod nurbs;
pub mod part21;
pub mod read;
pub mod write;

pub use fit::{DEFAULT_TOL, Fit};
pub use part21::Limits;
pub use read::{ImportError, Imported, Refusal, Solid, import};
pub use write::{Export, ExportError, Options, Schema, export};

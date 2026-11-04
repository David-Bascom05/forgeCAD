//! Drawings: exact hidden-line views of parts and assemblies, sections and details, sheets with
//! borders and title blocks, dimensions, and their PDF, DXF and SVG.

pub mod annot;
pub mod dxf;
pub mod fits;
pub mod gdt;
pub mod geom2;
pub mod grid;
pub mod hlr;
pub mod page;
pub mod pdf;
pub mod section;
pub mod sheet;
pub mod svg;
pub mod view;

pub use geom2::{Geom2, Place};
pub use hlr::{Class, Instance, Options, Piece, Source, hlr};
pub use page::{Align, Page, Pen, Stroke, Text};
pub use view::View;

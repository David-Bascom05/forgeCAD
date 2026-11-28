//! A rigid board with one polygonal perimeter, a physical stack in top-to-bottom
//! order and named copper routes. All planar geometry is stored on a 1 nm grid.
mod arc;
mod check;
#[path = "../standard_library/drc/board.rs"]
pub(crate) mod standard;
pub use check::{Check, Finding, Rule, Witness};
mod floorplan;
pub mod packing;
mod geometry;
mod laminate;
mod library;
pub use laminate::{Finish, Laminate};
mod layers;
pub use layers::{CopperObject, DrawingRole, LayerGraphic, PairRole, PlotLayer};
pub use library::query;
mod parts;
mod process;
pub use process::{SelectedService, profiles};
mod pour;
pub use parts::*;
pub use pour::{Island, Zone};
mod artwork;
mod fabrication;
mod reader;
mod reader_layout;
mod report;
#[path = "../standard_library/drc/spacing.rs"]
mod spacing;
use crate::syntax::Span;
pub use geometry::{Point, Shape};
pub use reader::compile;
pub use report::response;
use std::collections::BTreeMap;

pub const SCHEMA: &str = "forgecad.pcb/1";
pub fn is_board(src: &str) -> bool {
    crate::electronics::header(src) == Some("pcb")
}
pub fn is_document(src: &str) -> bool {
    matches!(crate::electronics::header(src), Some("pcb" | "footprint_library"))
}

#[derive(Debug)]
pub enum LayerMaterial {
    Copper { weight_oz_ft2: f64 },
    Dielectric { material: String },
}

#[derive(Debug)]
pub struct Layer {
    pub laminate: Laminate,
    pub id: String,
    pub thickness_nm: i64,
    pub material: LayerMaterial,
}

#[derive(Debug)]
pub struct Net {
    pub name: String,
    pub terminal: Option<String>,
}

#[derive(Debug)]
pub struct Track {
    pub net: String,
    pub layer: String,
    pub width_nm: i64,
    pub points: Vec<Point>,
    pub span: Span,
}

#[derive(Debug)]
pub struct ThroughVia {
    pub mask_top: Option<i64>,
    pub mask_bottom: Option<i64>,
    pub net: String,
    pub at: Point,
    pub diameter_nm: i64,
    pub drill_nm: i64,
    pub span: Span,
}

#[derive(Debug)]
pub struct NetClass {
    pub min_width_nm: i64,
    pub min_spacing_nm: i64,
    pub nets: std::collections::BTreeSet<String>,
}
#[derive(Debug)]
pub struct Process {
    pub min_slot_nm: Option<i64>,
    pub inner_drill_clearance_nm: Option<i64>,
    pub selected: Option<SelectedService>,
    pub min_edge_nm: Option<i64>,
    pub hole_clearance_nm: Option<i64>,
    pub min_mask_web_nm: Option<i64>,
    pub silk_clearance_nm: Option<i64>,
    pub id: String,
    pub min_width_nm: i64,
    pub min_spacing_nm: i64,
    pub min_drill_nm: i64,
    pub min_annular_ring_nm: i64,
}

#[derive(Debug)]
pub struct SchematicSource {
    pub file: String,
    pub document: crate::schematic::Document,
    pub circuit: crate::schematic::Circuit,
}

#[derive(Debug)]
pub struct Board {
    pub packages: crate::electronics_library::Loaded,
    pub severities: BTreeMap<String, String>,
    pub layout: Layout,
    pub classes: BTreeMap<String, NetClass>,
    pub outline: Vec<Point>,
    pub layers: Vec<Layer>,
    pub nets: BTreeMap<String, Net>,
    pub tracks: BTreeMap<String, Track>,
    pub vias: BTreeMap<String, ThroughVia>,
    pub process: Option<Process>,
    pub schematic: Option<SchematicSource>,
}

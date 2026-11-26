//! A rigid board with one polygonal perimeter, a physical stack in top-to-bottom
//! order and named copper routes. All planar geometry is stored on a 1 nm grid.
mod check;
pub use check::{Check, Finding, Rule, Witness};
mod geometry;
mod reader;
mod report;
mod spacing;
use crate::syntax::Span;
pub use geometry::Point;
pub use reader::compile;
pub use report::response;
use std::collections::BTreeMap;

pub const SCHEMA: &str = "forgecad.pcb/1";
pub fn is_document(src: &str) -> bool {
    crate::electronics::header(src) == Some("pcb")
}

#[derive(Debug)]
pub enum LayerMaterial {
    Copper { weight_oz_ft2: f64 },
    Dielectric { material: String },
}

#[derive(Debug)]
pub struct Layer {
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
    pub net: String,
    pub at: Point,
    pub diameter_nm: i64,
    pub drill_nm: i64,
    pub span: Span,
}

#[derive(Debug)]
pub struct Process {
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
    pub outline: Vec<Point>,
    pub layers: Vec<Layer>,
    pub nets: BTreeMap<String, Net>,
    pub tracks: BTreeMap<String, Track>,
    pub vias: BTreeMap<String, ThroughVia>,
    pub process: Option<Process>,
    pub schematic: Option<SchematicSource>,
}

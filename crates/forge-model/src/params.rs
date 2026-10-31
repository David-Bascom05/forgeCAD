//! Parameters and sketches: history steps that make nothing themselves. The part file
//! evaluates them as it reads each line (forge-agent), so later features hold plain values.

use forge_math::{Frame, Point3};
use forge_topo::Name;

/// What a value measures: units are checked where a value is used, never converted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    None,
    Mm,
    Deg,
}

/// `name = expr [unit]`: a named value for the lines below.
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: String,
    /// As written.
    pub expr: String,
    pub unit: Unit,
    pub value: f64,
}

/// A 2D sketch, solved: its rows (geometry, constraints and dimensions, each a row of tokens
/// as written) and the solved geometry later features take profiles and points from.
#[derive(Clone, Debug, PartialEq)]
pub struct Sketch {
    pub plane: Frame,
    /// What the plane was read from, if not a frame written out: kept so the file says it
    /// again, and read afresh each time the sketch's line is.
    pub on: Option<On>,
    /// The origin written after it (`at X Y Z`), if one was.
    pub at: Option<Point3>,
    pub rows: Vec<Vec<String>>,
    pub solved: forge_sketch::Sketch,
    /// Each driven dimension (`ref dist l1`): its row and the value it measures (mm, or radians
    /// for an angle).
    pub driven: Vec<(usize, f64)>,
    /// Each named entity (`l3`, `c1`, `o`) and its id in `solved`.
    pub names: Vec<(String, forge_sketch::EntityId)>,
}

/// A plane given by reference.
#[derive(Clone, Debug, PartialEq)]
pub enum On {
    /// A planar face, its outside.
    Face(Name),
    /// An earlier feature's plane (a work plane or a sketch), flipped or not, its origin at one
    /// of a sketch's points if one is named.
    Feature {
        feature: usize,
        point: Option<String>,
        flip: bool,
    },
}

//! Parameters and sketches: history steps that make nothing themselves. The part file
//! evaluates them as it reads each line (forge-agent), so later features hold plain values.

use forge_math::Frame;
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
    /// The planar face the sketch was placed on, if it was.
    pub on: Option<Name>,
    pub rows: Vec<Vec<String>>,
    pub solved: forge_sketch::Sketch,
    /// Each named entity (`l3`, `c1`, `o`) and its id in `solved`.
    pub names: Vec<(String, forge_sketch::EntityId)>,
}

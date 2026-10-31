//! Parameters and sketches: history steps that make nothing themselves. The part file
//! evaluates them as it reads each line (forge-agent), so later features hold plain values.

use forge_math::{Frame, Point2, Point3};
use forge_sketch::{Entity, EntityId};
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
    /// What a re-read with only dimension values changed starts from instead of the rows:
    /// the parameters as written, each dimension row's constraint, each driven row's entities.
    pub written: Vec<f64>,
    pub dims: Vec<(usize, forge_sketch::ConstraintId)>,
    pub driven_of: Vec<(String, Vec<forge_sketch::EntityId>)>,
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

impl Sketch {
    /// A point by its name in the block: `p1`, a line's or arc's end `l2.a` `a1.b`, a circle's
    /// or arc's centre `c1` `a1.c`; in the sketch's plane.
    pub fn point(&self, t: &str) -> Option<Point2> {
        let (base, part) = t.split_once('.').unwrap_or((t, ""));
        let e = self.names.iter().find(|(n, _)| n == base)?.1;
        let q = &self.solved;
        let at = |p: EntityId| Some(q.point(p));
        match (q.entity(e)?, part) {
            (Entity::Point { .. }, "") => at(e),
            (Entity::Line { a, .. } | Entity::Arc { start: a, .. }, "a") => at(a),
            (Entity::Line { b, .. } | Entity::Arc { end: b, .. }, "b") => at(b),
            (Entity::Circle { center, .. } | Entity::Arc { center, .. }, "" | "c") => at(center),
            _ => None,
        }
    }

    /// The point a reference's name stands for (`point_name`), its name and where it is.
    pub fn point_by(&self, n: Name) -> Option<(String, Point3)> {
        let t = self
            .names
            .iter()
            .flat_map(|(b, _)| ["", ".a", ".b", ".c"].map(|p| format!("{b}{p}")))
            .find(|t| point_name(t) == n)?;
        let p = self.plane.plane_point(self.point(&t)?);
        Some((t, p))
    }
}

/// The name a reference to a sketch's point (`l2.b`) carries.
pub fn point_name(t: &str) -> Name {
    Name::NONE.with_str("sketch point").with_str(t)
}

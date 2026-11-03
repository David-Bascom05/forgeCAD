//! Self-clinching hardware as data: nuts, studs and standoffs pressed into a sheet, each with the
//! sheet thickness it takes, the mounting hole it needs, how close to an edge its hole centre may
//! come and the hardest sheet it clinches into. A mount is a hole in the graph plus the part.
//!
//! PLACEHOLDER VALUES: `placeholder_catalogue` is not a maker's table. Its nut rows resemble
//! figures quoted in third-party copies of PEM bulletin CL (type CLS) but were not checked against
//! the bulletin; its stud and standoff rows are made up. Every row says so (`Source::Placeholder`),
//! and a sheet using one carries `SheetWarning::UnverifiedHardware`. Put a cited table in the shop
//! before making parts from it.
use crate::error::{CutId, WallId};
use forge_math::Point2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HardwareKind {
    Nut,
    Stud,
    Standoff,
}

impl HardwareKind {
    pub fn name(self) -> &'static str {
        match self {
            HardwareKind::Nut => "nut",
            HardwareKind::Stud => "stud",
            HardwareKind::Standoff => "standoff",
        }
    }
    pub fn from_name(s: &str) -> Option<HardwareKind> {
        [HardwareKind::Nut, HardwareKind::Stud, HardwareKind::Standoff]
            .into_iter()
            .find(|k| k.name() == s)
    }
}

/// Where a row's numbers come from.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// Not a maker's published figure: never make parts from it.
    Placeholder,
    /// A published table, as cited (bulletin, page, revision).
    Cited(String),
}

/// One catalogue row. Lengths in mm, hardness Rockwell B.
#[derive(Clone, Debug, PartialEq)]
pub struct Hardware {
    pub kind: HardwareKind,
    /// The thread, as the maker names it ("M3").
    pub size: String,
    /// The maker's part code, if any ("CLS-M3-1").
    pub code: String,
    pub sheet_min: f64,
    pub sheet_max: f64,
    /// The mounting hole's diameter.
    pub hole: f64,
    /// The least distance from the hole's centre to an edge (bends count as edges).
    pub edge: f64,
    /// The hardest sheet it clinches into.
    pub max_hardness: f64,
    pub source: Source,
}

/// Hardware placed in the sheet: its mounting hole is cut `cut`, centred at `at` in `wall`'s
/// coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct Mount {
    pub cut: CutId,
    pub wall: WallId,
    pub at: Point2,
    pub part: Hardware,
}

/// Placeholder rows (see the module's note): nuts, studs and standoffs, M3 to M5.
pub fn placeholder_catalogue() -> Vec<Hardware> {
    let row = |kind, size: &str, sheet_min, hole, edge| Hardware {
        kind,
        size: size.into(),
        code: String::new(),
        sheet_min,
        sheet_max: 6.0,
        hole,
        edge,
        max_hardness: 80.0,
        source: Source::Placeholder,
    };
    use HardwareKind::*;
    vec![
        row(Nut, "M3", 1.0, 4.22, 4.8),
        row(Nut, "M4", 1.0, 5.41, 6.9),
        row(Nut, "M5", 1.0, 6.35, 7.1),
        row(Stud, "M3", 1.0, 3.0, 5.0),
        row(Stud, "M4", 1.0, 4.0, 6.0),
        row(Stud, "M5", 1.2, 5.0, 7.0),
        row(Standoff, "M3", 1.0, 4.2, 6.0),
        row(Standoff, "M4", 1.0, 5.4, 7.0),
        row(Standoff, "M5", 1.2, 7.2, 8.0),
    ]
}

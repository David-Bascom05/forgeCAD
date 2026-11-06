//! AutoCAD R2000 (AC1015) DWG, written natively: a 2D drawing in model space (layers with their
//! colours, line types and lineweights, and lines, arcs, circles, ellipses, polylines, splines,
//! points, solids and text) as the binary file AutoCAD and every DWG reader opens, laid out per the
//! Open Design Specification for .dwg files. `read` reads such a file back.

pub mod bits;
pub mod draft;
pub mod edit;
pub mod file;
pub mod fonts;
pub mod header;
pub mod io;
pub mod object;
pub mod scene;
pub mod str;
pub mod types;
pub mod version;
mod write;

pub use write::write;

/// A drawing: what goes in the file. Units are millimetres.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Doc {
    /// The layers, "0" first. A layer named by an entity and missing here is made, white and
    /// continuous.
    pub layers: Vec<Layer>,
    /// Line types besides ByBlock, ByLayer and Continuous (which every drawing has).
    pub line_types: Vec<LineType>,
    pub entities: Vec<Entity>,
    /// The drawing's limits: its sheet (min, max).
    pub limits: ([f64; 2], [f64; 2]),
    /// The Standard text style's font file ("txt", "arial.ttf", ..).
    pub font: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub name: String,
    /// AutoCAD colour index, 1..255.
    pub colour: i16,
    /// A line type's name: "Continuous" or one of `Doc::line_types`.
    pub line_type: String,
    pub weight: Weight,
}

/// A lineweight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    Default,
    ByLayer,
    ByBlock,
    /// Hundredths of a millimetre, one of AutoCAD's: 0, 5, 9, 13, 15, 18, 20, 25, 30, 35, 40,
    /// 50, 53, 60, 70, 80, 90, 100, 106, 120, 140, 158, 200 or 211 (others go to the nearest).
    Mm100(i16),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineType {
    pub name: String,
    pub description: String,
    /// Dash lengths (mm): positive drawn, negative a gap, zero a dot.
    pub pattern: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub layer: String,
    pub shape: Shape,
}

/// Where a text's insertion point is on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HAlign {
    Left,
    Centre,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Line {
        a: [f64; 2],
        b: [f64; 2],
    },
    Circle {
        c: [f64; 2],
        r: f64,
    },
    /// Counter-clockwise from `start` to `end` (radians).
    Arc {
        c: [f64; 2],
        r: f64,
        start: f64,
        end: f64,
    },
    /// c + cos(t) major + sin(t) minor for t from `t0` to `t1`, the minor axis `ratio` times
    /// the major one and a quarter turn counter-clockwise from it.
    Ellipse {
        c: [f64; 2],
        major: [f64; 2],
        ratio: f64,
        t0: f64,
        t1: f64,
    },
    Polyline {
        points: Vec<[f64; 2]>,
        closed: bool,
    },
    /// A clamped NURBS: `knots` has `points.len() + degree + 1` values.
    Spline {
        degree: u32,
        knots: Vec<f64>,
        points: Vec<[f64; 2]>,
        weights: Option<Vec<f64>>,
    },
    Point {
        at: [f64; 2],
    },
    /// A filled quadrilateral (corners in AutoCAD's order: the third and fourth swap).
    Solid {
        corners: [[f64; 2]; 4],
    },
    Text {
        at: [f64; 2],
        /// Height (mm).
        h: f64,
        /// Radians counter-clockwise.
        angle: f64,
        align: HAlign,
        /// AutoCAD's text: %%c, %%d and %%p for the diameter, degree and plus-minus signs.
        text: String,
    },
}

/// The lineweights AutoCAD has, by their index in DWG.
pub(crate) const WEIGHTS: [i16; 24] = [
    0, 5, 9, 13, 15, 18, 20, 25, 30, 35, 40, 50, 53, 60, 70, 80, 90, 100, 106, 120, 140, 158, 200, 211,
];

impl Weight {
    /// The index DWG stores.
    pub(crate) fn code(self) -> u8 {
        match self {
            Weight::ByLayer => 29,
            Weight::ByBlock => 30,
            Weight::Default => 31,
            Weight::Mm100(w) => (0..WEIGHTS.len())
                .min_by_key(|&k| (WEIGHTS[k] - w).abs())
                .unwrap_or(0) as u8,
        }
    }
}

/// Text in the drawing's code page (Windows-1252): ASCII and Latin-1 as themselves, anything else
/// as AutoCAD's \U+XXXX.
pub(crate) fn encode(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for c in s.chars() {
        match c as u32 {
            u @ (0..0x80 | 0xA0..0x100) => out.push(u as u8),
            u if u <= 0xFFFF => out.extend_from_slice(format!("\\U+{u:04X}").as_bytes()),
            u => out.extend_from_slice(
                format!(
                    "\\U+{:04X}\\U+{:04X}",
                    0xD800 + ((u - 0x10000) >> 10),
                    0xDC00 + ((u - 0x10000) & 0x3FF)
                )
                .as_bytes(),
            ),
        }
    }
    out
}

//! A drawn sheet: strokes, filled shapes and text in sheet millimetres (origin bottom left), what
//! the PDF, DXF and SVG writers take.

use crate::geom2::Geom2;
use forge_math::Point2;

/// How a line is drawn: ISO 128 / ASME Y14.2 line types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Pen {
    Visible,
    Hidden,
    Tangent,
    Center,
    Phantom,
    /// Dimension, extension and leader lines, hatching, thin text.
    Thin,
    Hatch,
    /// The cutting-plane line of a section.
    Cut,
    Border,
    /// A sick annotation: its model reference is gone (Inventor's magenta).
    Sick,
}

impl Pen {
    /// Line width (mm).
    pub fn width(self) -> f64 {
        match self {
            Pen::Visible | Pen::Cut | Pen::Border => 0.5,
            Pen::Hidden | Pen::Sick => 0.35,
            Pen::Tangent | Pen::Center | Pen::Phantom | Pen::Thin | Pen::Hatch => 0.25,
        }
    }
    /// Dash pattern: on, off, on, off.. (mm); empty for continuous.
    pub fn dashes(self) -> &'static [f64] {
        match self {
            Pen::Hidden => &[3.0, 1.5],
            Pen::Center => &[12.0, 2.0, 2.0, 2.0],
            Pen::Phantom => &[12.0, 2.0, 2.0, 2.0, 2.0, 2.0],
            Pen::Cut => &[12.0, 2.0, 2.0, 2.0],
            _ => &[],
        }
    }
    /// RGB, 0..1.
    pub fn colour(self) -> [f64; 3] {
        match self {
            Pen::Sick => [0.85, 0.0, 0.75],
            _ => [0.0, 0.0, 0.0],
        }
    }
    /// The DXF layer the pen's lines go on.
    pub fn layer(self) -> &'static str {
        match self {
            Pen::Visible => "VISIBLE",
            Pen::Hidden => "HIDDEN",
            Pen::Tangent => "TANGENT",
            Pen::Center => "CENTER",
            Pen::Phantom => "PHANTOM",
            Pen::Thin => "DIM",
            Pen::Hatch => "HATCH",
            Pen::Cut => "SECTION",
            Pen::Border => "BORDER",
            Pen::Sick => "SICK",
        }
    }
    pub const ALL: [Pen; 10] = [
        Pen::Visible,
        Pen::Hidden,
        Pen::Tangent,
        Pen::Center,
        Pen::Phantom,
        Pen::Thin,
        Pen::Hatch,
        Pen::Cut,
        Pen::Border,
        Pen::Sick,
    ];
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub pen: Pen,
    pub geom: Geom2,
}

/// Where a text's anchor point is on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Centre,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    /// The baseline point the alignment refers to.
    pub at: Point2,
    /// Cap height (mm).
    pub h: f64,
    /// Degrees counter-clockwise.
    pub angle: f64,
    pub align: Align,
    pub text: String,
    pub pen: Pen,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Page {
    /// Width and height (mm).
    pub w: f64,
    pub h: f64,
    pub strokes: Vec<Stroke>,
    /// Filled polygons (arrowheads, the projection symbol's).
    pub fills: Vec<(Pen, Vec<Point2>)>,
    pub texts: Vec<Text>,
}

/// How wide `text` is at cap height `h` in Helvetica (the PDF's base font), from its widths.
pub fn text_width(text: &str, h: f64) -> f64 {
    // Helvetica advance widths per 1000 em; cap height is 718/1000 em
    let em = h / 0.718;
    text.chars().map(|c| helvetica(c) as f64).sum::<f64>() * em / 1000.0
}

fn helvetica(c: char) -> u16 {
    match c {
        ' ' | '!' | ',' | '.' | '/' | ':' | ';' | '[' | ']' | '\\' | 'I' => 278,
        '"' => 355,
        '#'
        | '$'
        | '0'..='9'
        | '_'
        | 'a'
        | 'b'
        | 'd'
        | 'e'
        | 'g'
        | 'h'
        | 'n'
        | 'o'
        | 'p'
        | 'q'
        | 'u'
        | '?'
        | 'L' => 556,
        '%' => 889,
        '&' | 'A' | 'B' | 'E' | 'K' | 'P' | 'S' | 'V' | 'X' | 'Y' => 667,
        '\'' => 191,
        '(' | ')' | '-' | 'r' | '`' => 333,
        '*' => 389,
        '+' | '<' | '=' | '>' | '~' | '\u{b1}' => 584,
        '@' => 1015,
        'C' | 'D' | 'H' | 'N' | 'R' | 'U' => 722,
        'F' | 'T' | 'Z' => 611,
        'G' | 'O' | 'Q' => 778,
        'J' | 'c' | 'k' | 's' | 'v' | 'x' | 'y' | 'z' => 500,
        'M' => 833,
        'W' => 944,
        'f' | 't' => 278,
        'i' | 'j' | 'l' | '|' => 222,
        'm' => 833,
        'w' => 722,
        '\u{b0}' => 400,
        '\u{d8}' | '\u{f8}' => 778,
        _ => 556,
    }
}

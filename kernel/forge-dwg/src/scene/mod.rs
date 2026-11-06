//! A drawing as what is seen: one space (model space or a layout) as 2D paths and filled
//! regions in world units, each with its colour, lineweight and layer, blocks expanded, line
//! types applied, text turned into strokes or outlines. What a viewer draws and picks from.
//!
//! The builder is deterministic (forge-math's trig), so a scene is the same on every target.

mod build;
mod color;
mod geom;
mod hatch;
mod ltype;
mod text;

pub use color::aci_rgb;

use crate::file::Drawing;

/// Which space to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    Model,
    /// A layout, by its LAYOUT object's handle.
    Layout(u32),
}

/// What shapes text: given a style's font file names and the text, glyph strokes and outlines.
pub trait Fonts {
    /// The text set at the origin, baseline along +x, one unit cap height: strokes (single-line
    /// SHX-like fonts) and filled rings (TrueType), and the advance it took.
    fn shape(&self, font: &str, bigfont: &str, text: &str) -> Shaped;
}

/// Shaped text in em units (cap height 1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shaped {
    pub strokes: Vec<Vec<[f64; 2]>>,
    /// Even-odd filled rings.
    pub fills: Vec<Vec<[f64; 2]>>,
    pub advance: f64,
}

/// No fonts: each character a box outline, a crude stand-in while fonts load.
pub struct BoxFont;

impl Fonts for BoxFont {
    fn shape(&self, _font: &str, _bigfont: &str, text: &str) -> Shaped {
        let mut s = Shaped::default();
        for c in text.chars() {
            if !c.is_whitespace() {
                let x = s.advance;
                s.strokes
                    .push(vec![[x, 0.0], [x + 0.7, 0.0], [x + 0.7, 1.0], [x, 1.0], [x, 0.0]]);
            }
            s.advance += 0.9;
        }
        s
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Prim {
    /// A polyline in world units.
    Path { pts: Vec<[f64; 2]>, closed: bool },
    /// Filled rings, even-odd.
    Fill { rings: Vec<Vec<[f64; 2]>> },
    /// Separate straight segments, each consecutive pair of points one (a line type's dashes
    /// along a path, a hatch's pattern lines): many short lines as one item.
    Segments { pts: Vec<[f64; 2]> },
    /// Filled rings, even-odd, in the canvas's background colour whatever `Item::rgb` says: a
    /// WIPEOUT, or an MTEXT's background mask. It hides what was drawn before it.
    Mask { rings: Vec<Vec<[f64; 2]>> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// The top-level entity this came from (an INSERT's pieces carry the INSERT's handle).
    pub entity: u32,
    /// Index into `Scene::layers`.
    pub layer: usize,
    pub rgb: [u8; 3],
    /// Millimetres; 0 the thinnest line.
    pub weight: f32,
    pub prim: Prim,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneLayer {
    pub handle: u32,
    pub name: String,
    pub rgb: [u8; 3],
    pub on: bool,
    pub frozen: bool,
    pub locked: bool,
    pub plot: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub layers: Vec<SceneLayer>,
    pub items: Vec<Item>,
    /// The box round every item, or None for an empty space.
    pub extents: Option<([f64; 2], [f64; 2])>,
}

/// How finely curves are cut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    /// The largest gap between a curve and its chords, in world units (0: chosen from the
    /// drawing's extents).
    pub chord: f64,
}

impl Default for Options {
    fn default() -> Options {
        Options { chord: 0.0 }
    }
}

impl Scene {
    pub fn grow_extents(&mut self) {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        let mut add = |p: &[f64; 2]| {
            if p[0] < lo[0] {
                lo[0] = p[0];
            }
            if p[0] > hi[0] {
                hi[0] = p[0];
            }
            if p[1] < lo[1] {
                lo[1] = p[1];
            }
            if p[1] > hi[1] {
                hi[1] = p[1];
            }
        };
        for it in &self.items {
            match &it.prim {
                Prim::Path { pts, .. } | Prim::Segments { pts } => pts.iter().for_each(&mut add),
                Prim::Fill { rings } | Prim::Mask { rings } => rings.iter().flatten().for_each(&mut add),
            }
        }
        self.extents = (lo[0] <= hi[0]).then_some((lo, hi));
    }
}

/// The space's scene, as AutoCAD shows it in plan: every entity type it can draw (blocks
/// expanded, dimensions, leaders, hatches, text and the rest), colours, line types and
/// lineweights resolved, layers off or frozen left out; a layout with model space through its
/// viewports. Rays and construction lines are cut to a box well beyond everything else and left
/// out of `extents`.
pub fn build(d: &Drawing, space: Space, fonts: &dyn Fonts, opts: &Options) -> Scene {
    build::build(d, space, fonts, opts)
}

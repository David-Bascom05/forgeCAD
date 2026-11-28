//! Local package geometry and board-side placement. Board coordinates are always viewed from above.
use super::{Point, Shape};
use crate::electronics::{Fault, Result};
use crate::syntax::Span;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Top,
    Bottom,
}
impl Side {
    pub const BOTH: [Side; 2] = [Side::Top, Side::Bottom];
    pub fn name(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "top" => Some(Self::Top),
            "bottom" => Some(Self::Bottom),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Pose {
    pub at: Point,
    pub side: Side,
    pub angle: f64,
}
impl Pose {
    fn transform(&self, [x, y]: [f64; 2], span: Span) -> Result<Point> {
        let x = if self.side == Side::Bottom { -x } else { x };
        let (s, c) = forge_math::dmath::sin_cos(self.angle.to_radians());
        let pt = [
            self.at[0] as f64 + x * c - y * s,
            self.at[1] as f64 + x * s + y * c,
        ];
        if pt.iter().any(|p| p.abs() > 500_000_000.0 + 0.000001) {
            return Err(Fault::new(
                "BoardGrid",
                span,
                "transformed artwork exceeds the board grid",
            ));
        }
        Ok(pt.map(|v| v.round() as i64))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadShape {
    Rect,
    Round,
    Oval,
}
#[derive(Clone, Debug)]
pub struct PadDef {
    pub number: String,
    pub at: Point,
    pub size: Point,
    pub shape: PadShape,
    pub drill: Option<i64>,
    pub slot_length: Option<i64>,
    /// Signed per-edge expansion. None disables the aperture.
    pub mask: Option<i64>,
    pub paste: Option<i64>,
}
impl PadDef {
    pub fn drill_axis(&self, pose: Pose, span: Span) -> Result<[Point; 2]> {
        let half = (self.slot_length.unwrap_or(0) - self.drill.unwrap_or(0)).max(0) as f64 / 2.;
        let k = usize::from(self.size[1] > self.size[0]);
        let mut a = self.at.map(|v| v as f64);
        let mut b = a;
        a[k] -= half;
        b[k] += half;
        Ok([pose.transform(a, span)?, pose.transform(b, span)?])
    }
    pub fn artwork(&self, pose: Pose, margin: i64, span: Span) -> Result<Shape> {
        let [w, h] = self.size.map(|v| v + 2 * margin);
        if w <= 0 || h <= 0 || w > 1_000_000_000 || h > 1_000_000_000 {
            return Err(Fault::new(
                "InvalidAperture",
                span,
                "aperture expansion must leave a positive size within the board grid",
            ));
        }
        let coord = |x: f64, y: f64| pose.transform([self.at[0] as f64 + x, self.at[1] as f64 + y], span);
        let shape = match self.shape {
            PadShape::Round => Shape::disk(coord(0., 0.)?, w),
            PadShape::Oval => {
                let dx = (w - h).max(0) as f64 / 2.;
                let dy = (h - w).max(0) as f64 / 2.;
                Shape::Stroke {
                    axis: [coord(-dx, -dy)?, coord(dx, dy)?],
                    width: w.min(h),
                }
            }
            PadShape::Rect => {
                let x = w as f64 / 2.;
                let y = h as f64 / 2.;
                let ps = vec![coord(-x, -y)?, coord(x, -y)?, coord(x, y)?, coord(-x, y)?];
                if !super::geometry::simple(&ps) {
                    return Err(Fault::new(
                        "InvalidAperture",
                        span,
                        "pad degenerates on the board grid",
                    ));
                }
                Shape::Region(ps)
            }
        };
        Ok(shape)
    }
}
#[derive(Debug)]
pub struct Footprint {
    pub(super) artwork: Vec<(super::PairRole, super::layers::Primitive)>,
    pub pads: Vec<PadDef>,
    pub courtyard: Option<Vec<Point>>,
    pub legend: Vec<Legend>,
    pub paste_windows: Vec<Vec<Point>>,
}
#[derive(Debug)]
pub enum Legend {
    Line {
        points: Vec<Point>,
        width: i64,
    },
    Text {
        at: Point,
        value: String,
        height: i64,
        width: i64,
    },
}
#[derive(Debug)]
pub struct Placement {
    pub footprint: String,
    pub occurrence: Option<String>,
    pub reference: String,
    pub courtyard: Option<Shape>,
    pub pose: Pose,
    pub span: Span,
}
#[derive(Debug)]
pub struct Pad {
    pub placement: String,
    pub number: String,
    pub net: String,
    pub side: Side,
    pub at: Point,
    pub copper: Shape,
    pub mask: Option<Shape>,
    pub paste: Option<Shape>,
    pub drill: Option<i64>,
    pub drill_axis: [Point; 2],
    pub min_diameter: i64,
    pub span: Span,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Film {
    Mask,
    Paste,
    Silk,
}
impl Film {
    pub fn name(self) -> &'static str {
        match self {
            Self::Mask => "mask",
            Self::Paste => "paste",
            Self::Silk => "silk",
        }
    }
    pub const ALL: [Film; 3] = [Film::Mask, Film::Paste, Film::Silk];
}
#[derive(Debug)]
pub struct Graphic {
    pub film: Film,
    pub side: Side,
    pub shapes: Vec<Shape>,
    pub span: Span,
}
#[derive(Debug)]
pub struct Hole {
    pub at: Point,
    pub to: Option<Point>,
    pub drill: i64,
    pub span: Span,
}
#[derive(Debug)]
pub struct Area {
    /// Empty means every copper object.
    pub restrict: std::collections::BTreeSet<super::CopperObject>,
    pub layer: String,
    pub net: Option<String>,
    pub polygon: Vec<Point>,
    pub span: Span,
}
#[derive(Debug, Default)]
pub struct Layout {
    pub finishes: BTreeMap<(Side, Film), super::Finish>,
    pub user_layers: BTreeMap<String, (String, bool)>,
    pub layer_graphics: BTreeMap<String, super::LayerGraphic>,
    pub footprints: BTreeMap<String, Footprint>,
    pub placements: BTreeMap<String, Placement>,
    pub pads: BTreeMap<String, Pad>,
    pub graphics: BTreeMap<String, Graphic>,
    pub holes: BTreeMap<String, Hole>,
    pub cutouts: BTreeMap<String, Vec<Point>>,
    pub areas: BTreeMap<String, Area>,
    pub zones: BTreeMap<String, super::Zone>,
    pub fills: BTreeMap<String, super::Island>,
}
pub(super) fn transform(pose: Pose, at: Point, span: Span) -> Result<Point> {
    pose.transform(at.map(|x| x as f64), span)
}

pub(super) fn legend(text: &str, height: i64, width: i64, pose: Pose, span: Span) -> Result<Vec<Shape>> {
    use forge_dwg::fonts::hershey::{Face, Spacing, glyph};
    let mut shapes = Vec::new();
    let mut x = 0.;
    for ch in text.chars() {
        let g = glyph(Face::Simplex, Spacing::Normal, ch)
            .filter(|g| g.fills.is_empty())
            .ok_or_else(|| {
                Fault::new(
                    "UnsupportedLegendGlyph",
                    span,
                    format!("no stroke glyph for {ch}"),
                )
            })?;
        for stroke in g.strokes {
            let ps = stroke
                .into_iter()
                .map(|p| pose.transform([(x + p[0]) * height as f64, p[1] * height as f64], span))
                .collect::<Result<Vec<_>>>()?;
            for p in ps.windows(2) {
                shapes.push(Shape::Stroke {
                    axis: [p[0], p[1]],
                    width,
                });
            }
        }
        x += g.advance;
    }
    Ok(shapes)
}

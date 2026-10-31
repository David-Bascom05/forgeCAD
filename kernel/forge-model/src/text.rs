//! Embossed and engraved text: a string set in a TrueType font (advances and pair kerning),
//! laid out on a plane or wrapped round a cylinder, each glyph's regions raised (add) or sunk
//! (cut) as solids with exact parabolic or wrapped walls (see `forge_ops::glyph`).
//!
//! The kernel reads no files: the host finds the font the feature names (a path, say) and hands
//! it over parsed, in the feature's `FontRef`.

use crate::{Combine, Feature, History, RegenError, Tool};
use forge_bool::Shape;
use forge_font::{Font, FontError, Seg};
use forge_math::{Frame, Point2};
use forge_ops::{GlyphError, GlyphSeg, Place, glyph};
use forge_topo::Name;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub text: String,
    pub font: FontRef,
    /// The font's em, mm.
    pub size: f64,
    pub on: TextOn,
    /// How far the letters stand up (add) or are cut down (cut).
    pub depth: f64,
    pub op: Combine,
}

/// Where the text is set: the baseline from the origin along x, the letters up y.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextOn {
    /// On the frame's xy, standing up its z.
    Plane(Frame),
    /// Round the cylinder of `radius` about the frame's z, from its x anticlockwise about z,
    /// the letters up z, standing out from the axis.
    Cylinder { axis: Frame, radius: f64 },
}

/// A font by its name in the part file (a path), and the font the host found there; None until
/// it has looked. Two refs are equal when they name the same font.
#[derive(Clone, Debug)]
pub struct FontRef {
    pub name: String,
    pub font: Option<Arc<Font>>,
}

impl FontRef {
    pub fn named(name: &str) -> FontRef {
        FontRef {
            name: name.to_string(),
            font: None,
        }
    }
}

impl PartialEq for FontRef {
    fn eq(&self, o: &FontRef) -> bool {
        self.name == o.name
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TextError {
    /// The host found no font by the feature's name.
    NoFont,
    Font(FontError),
    /// A size, depth or radius that is not positive, or an op other than add or cut.
    BadSize,
    /// No character of the text has an outline.
    Empty,
    /// The font has no glyph for this character: the text would come out without it.
    NoGlyph(char),
    Glyph(GlyphError),
}

/// The text's glyph regions in layout units (mm): each an outline, counter-clockwise, then its
/// holes, clockwise. Lines break at newlines, 1.2 em apart.
pub fn layout(font: &Font, text: &str, size: f64) -> Result<Vec<Vec<Vec<GlyphSeg>>>, TextError> {
    let k = size / font.units_per_em;
    let mut regions = Vec::new();
    let (mut x, mut y) = (0.0, 0.0);
    let mut prev = None;
    for c in text.chars() {
        if c == '\n' {
            (x, y, prev) = (0.0, y - 1.2 * size, None);
            continue;
        }
        let g = font.glyph(c);
        if g == 0 && !c.is_whitespace() {
            return Err(TextError::NoGlyph(c));
        }
        if let Some(p) = prev {
            x += font.kerning(p, g) * k;
        }
        let at = |q: Point2| Point2::new(x + q.x * k, y + q.y * k);
        let contours: Vec<Vec<GlyphSeg>> = font
            .outline(g)
            .map_err(TextError::Font)?
            .iter()
            .map(|c| {
                c.iter()
                    .map(|s| match *s {
                        Seg::Line(a, b) => GlyphSeg::Line(at(a), at(b)),
                        Seg::Quad(a, m, b) => GlyphSeg::Quad(at(a), at(m), at(b)),
                    })
                    .collect()
            })
            .collect();
        regions.extend(nest(contours));
        x += font.advance(g) * k;
        prev = Some(g);
    }
    Ok(regions)
}

/// Points along a contour, its quadratic arcs in eight chords each.
fn polygon(c: &[GlyphSeg]) -> Vec<Point2> {
    let mut out = Vec::new();
    for s in c {
        match *s {
            GlyphSeg::Line(a, _) => out.push(a),
            GlyphSeg::Quad(a, m, b) => {
                for i in 0..8 {
                    let t = i as f64 / 8.0;
                    let (u, v) = ((1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t));
                    out.push(Point2::new(
                        u * a.x + v * m.x + t * t * b.x,
                        u * a.y + v * m.y + t * t * b.y,
                    ));
                }
            }
        }
    }
    out
}

/// Twice the signed area (anticlockwise positive), the arcs' bulges included exactly.
fn area2(c: &[GlyphSeg]) -> f64 {
    c.iter()
        .map(|s| match *s {
            GlyphSeg::Line(a, b) => a.x * b.y - b.x * a.y,
            // the chord's, and the segment between chord and arc: 2/3 of the control triangle
            GlyphSeg::Quad(a, m, b) => a.x * b.y - b.x * a.y + 2.0 / 3.0 * (m - a).cross(b - a),
        })
        .sum()
}

fn inside(p: Point2, poly: &[Point2]) -> bool {
    let n = poly.len();
    let mut wind = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            wind = !wind;
        }
    }
    wind
}

fn reversed(c: &[GlyphSeg]) -> Vec<GlyphSeg> {
    c.iter()
        .rev()
        .map(|s| match *s {
            GlyphSeg::Line(a, b) => GlyphSeg::Line(b, a),
            GlyphSeg::Quad(a, m, b) => GlyphSeg::Quad(b, m, a),
        })
        .collect()
}

/// A glyph's contours as regions: those inside an even number of others are outlines, each
/// holding the contours directly inside it as holes.
fn nest(contours: Vec<Vec<GlyphSeg>>) -> Vec<Vec<Vec<GlyphSeg>>> {
    let polys: Vec<Vec<Point2>> = contours.iter().map(|c| polygon(c)).collect();
    let areas: Vec<f64> = contours.iter().map(|c| area2(c).abs()).collect();
    let within = |i: usize, j: usize| i != j && areas[j] > areas[i] && inside(polys[i][0], &polys[j]);
    let depth: Vec<usize> = (0..contours.len())
        .map(|i| (0..contours.len()).filter(|&j| within(i, j)).count())
        .collect();
    let mut regions: Vec<(usize, Vec<Vec<GlyphSeg>>)> = Vec::new();
    for (i, c) in contours.iter().enumerate() {
        if depth[i].is_multiple_of(2) {
            let outer = if area2(c) > 0.0 { c.clone() } else { reversed(c) };
            regions.push((i, vec![outer]));
        }
    }
    for (i, c) in contours.iter().enumerate() {
        if !depth[i].is_multiple_of(2) {
            // the smallest outline holding it
            let owner = regions
                .iter_mut()
                .filter(|r| within(i, r.0))
                .min_by(|a, b| areas[a.0].total_cmp(&areas[b.0]));
            if let Some(r) = owner {
                r.1.push(if area2(c) < 0.0 { c.clone() } else { reversed(c) });
            }
        }
    }
    regions.into_iter().map(|r| r.1).collect()
}

/// The solids text feature `i` combines with the part: one per glyph region.
pub(crate) fn tools(features: History<'_>, i: usize) -> Result<Vec<Tool>, RegenError> {
    let Feature::Text(t) = &features[i] else {
        unreachable!("only text")
    };
    let e = |error| RegenError::Text { feature: i, error };
    let pos = |v: f64| v > 0.0 && v.is_finite();
    if !(pos(t.size) && pos(t.depth) && matches!(t.op, Combine::Add | Combine::Cut)) {
        return Err(e(TextError::BadSize));
    }
    let f = t.font.font.clone().ok_or(e(TextError::NoFont))?;
    let regions = layout(&f, &t.text, t.size).map_err(e)?;
    if regions.is_empty() {
        return Err(e(TextError::Empty));
    }
    let d = t.depth;
    // embossed from the surface up; engraved from the depth to as far above it
    let (place, lo, hi) = match t.on {
        TextOn::Plane(f) => (Place::Flat(f), if t.op == Combine::Add { 0.0 } else { -d }, d),
        TextOn::Cylinder { axis, radius } => {
            if !pos(radius) || (t.op == Combine::Cut && d >= radius) {
                return Err(e(TextError::BadSize));
            }
            let lo = if t.op == Combine::Add { radius } else { radius - d };
            (Place::Wrapped { axis, radius }, lo, radius + d)
        }
    };
    let root = Name::feature(features.id(i));
    regions
        .iter()
        .enumerate()
        .map(|(k, r)| {
            let body = glyph(r, place, lo, hi, root.with(k as u64)).map_err(|g| e(TextError::Glyph(g)))?;
            let bbox = forge_bool::Index::new(&body).bbox;
            Ok((body, t.op, Shape::Brep(bbox)))
        })
        .collect()
}

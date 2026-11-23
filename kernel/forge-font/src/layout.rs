//! Setting a string in a font, in font units: each character's glyph moved to where the pen puts
//! it (advances, pair kerning), lines `ascender − descender + line gap` apart, each line moved
//! along its baseline as `Align` asks. Advances and kerning are whole font units and the
//! outlines are dyadic (integers, halves, 2^-14 and 2^-16 steps), so every placed point is the
//! sum of the font's own numbers, exact in f64: a left-aligned line is moved by nothing, a
//! centred one by half its width.

use crate::{Font, FontError, Seg};
use forge_math::Point2;

/// Where each line's baseline starts, relative to the pen's origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    /// The line starts at the origin.
    Left,
    /// The line's advance is centred on the origin.
    Center,
    /// The line ends at the origin.
    Right,
}

/// One character with an outline, where the layout put it.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    /// Its place in the string, counted in chars.
    pub index: usize,
    pub ch: char,
    /// How many of the same character come before it in the string.
    pub occurrence: usize,
    /// Its closed contours, in font units, placed.
    pub contours: Vec<Vec<Seg>>,
    /// Each contour's part of the glyph (`Font::outline_parts`): a composite's component.
    pub parts: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutError {
    Font(FontError),
    /// The font has no glyph for the character at this place in the string.
    NoGlyph {
        index: usize,
        ch: char,
    },
    /// Lines to set, and the font's line spacing is not positive.
    NoLineSpacing,
}

/// Sets `text` in `font`: the characters with an outline (spaces and other blank glyphs have
/// none) in string order. `tracking` (font units) is added after every character but a line's
/// last.
pub fn lay_out(font: &Font, text: &str, align: Align, tracking: f64) -> Result<Vec<Placed>, LayoutError> {
    let pitch = font.ascender - font.descender + font.line_gap;
    if text.contains('\n') && pitch <= 0.0 {
        return Err(LayoutError::NoLineSpacing);
    }
    let mut out = Vec::new();
    let mut seen: Vec<(char, usize)> = Vec::new();
    for (line_no, line) in text.split('\n').enumerate() {
        let y = -(line_no as f64) * pitch;
        // the line's own pen positions first, then its shift
        let mut x = 0.0;
        let mut prev = None;
        let mut line_out = Vec::new();
        let base = text
            .split('\n')
            .take(line_no)
            .map(|l| l.chars().count() + 1)
            .sum::<usize>();
        for (k, c) in line.chars().enumerate() {
            let index = base + k;
            let g = font.glyph(c);
            if g == 0 && !c.is_whitespace() {
                return Err(LayoutError::NoGlyph { index, ch: c });
            }
            if let Some(p) = prev {
                x += font.kerning(p, g) + tracking;
            }
            let contours = font.outline_parts(g).map_err(LayoutError::Font)?;
            if !contours.is_empty() {
                line_out.push((index, c, x, contours));
            }
            x += font.advance(g);
            prev = Some(g);
        }
        let shift = match align {
            Align::Left => 0.0,
            Align::Center => -x / 2.0,
            Align::Right => -x,
        };
        for (index, ch, at, contours) in line_out {
            let occurrence = match seen.iter_mut().find(|s| s.0 == ch) {
                Some(s) => {
                    s.1 += 1;
                    s.1 - 1
                }
                None => {
                    seen.push((ch, 1));
                    0
                }
            };
            let mv = |p: Point2| Point2::new(p.x + at + shift, p.y + y);
            let parts = contours.iter().map(|c| c.0).collect();
            let contours = contours
                .iter()
                .map(|(_, c)| {
                    c.iter()
                        .map(|s| match *s {
                            Seg::Line(a, b) => Seg::Line(mv(a), mv(b)),
                            Seg::Quad(a, m, b) => Seg::Quad(mv(a), mv(m), mv(b)),
                            Seg::Cubic(a, m, n, b) => Seg::Cubic(mv(a), mv(m), mv(n), mv(b)),
                        })
                        .collect()
                })
                .collect();
            out.push(Placed {
                index,
                ch,
                occurrence,
                contours,
                parts,
            });
        }
    }
    Ok(out)
}

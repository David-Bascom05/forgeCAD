//! Sheet formats: sizes, the border with its zones, the title block and the projection symbol.

use crate::geom2::Geom2;
use crate::page::{Align, Page, Pen, Stroke, Text};
use forge_math::{Point2, Vec2, dmath};

/// The sheet sizes, landscape, width and height in mm: ANSI A-E and ISO A4-A0.
pub const SIZES: [(&str, f64, f64); 10] = [
    ("A", 279.4, 215.9),
    ("B", 431.8, 279.4),
    ("C", 558.8, 431.8),
    ("D", 863.6, 558.8),
    ("E", 1117.6, 863.6),
    ("A4", 297.0, 210.0),
    ("A3", 420.0, 297.0),
    ("A2", 594.0, 420.0),
    ("A1", 841.0, 594.0),
    ("A0", 1189.0, 841.0),
];

pub fn size(name: &str) -> Option<(f64, f64)> {
    SIZES.iter().find(|s| s.0 == name).map(|s| (s.1, s.2))
}

/// The border's distance from the sheet's edge, and the zone band inside it.
pub const MARGIN: f64 = 10.0;
pub const BAND: f64 = 5.0;
/// The title block's size.
pub const TITLE_W: f64 = 180.0;
pub const TITLE_H: f64 = 36.0;

/// What the title block says.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fields {
    pub title: String,
    pub number: String,
    pub rev: String,
    pub size: String,
    pub material: String,
    pub scale: String,
    pub sheet: String,
    pub drawn: String,
    pub date: String,
    pub mass: String,
    pub company: String,
}

pub fn line(a: Point2, b: Point2) -> Geom2 {
    Geom2::Line { a, b }
}

fn rect(page: &mut Page, pen: Pen, lo: Point2, hi: Point2) {
    let c = [lo, Point2::new(hi.x, lo.y), hi, Point2::new(lo.x, hi.y)];
    for k in 0..4 {
        page.strokes.push(Stroke {
            pen,
            geom: line(c[k], c[(k + 1) % 4]),
        });
    }
}

pub fn text(page: &mut Page, at: Point2, h: f64, align: Align, s: &str) {
    page.texts.push(Text {
        at,
        h,
        angle: 0.0,
        align,
        text: s.to_string(),
        pen: Pen::Thin,
    });
}

/// The drawable area inside the border's zone band.
pub fn inside(w: f64, h: f64) -> (Point2, Point2) {
    (
        Point2::new(MARGIN + BAND, MARGIN + BAND),
        Point2::new(w - MARGIN - BAND, h - MARGIN - BAND),
    )
}

/// The border, its zones (numbers right to left along the top and bottom, letters bottom to top
/// along the sides, as ANSI sheets have them), the title block and the projection symbol.
pub fn format(page: &mut Page, f: &Fields, third_angle: bool) {
    let (w, h) = (page.w, page.h);
    let (o0, o1) = (Point2::new(MARGIN, MARGIN), Point2::new(w - MARGIN, h - MARGIN));
    let (i0, i1) = inside(w, h);
    rect(page, Pen::Border, o0, o1);
    rect(page, Pen::Thin, i0, i1);
    let nx = ((w - 2.0 * MARGIN) / 100.0).round().max(2.0) as usize * 2;
    let ny = ((h - 2.0 * MARGIN) / 100.0).round().max(2.0) as usize * 2;
    let zw = (o1.x - o0.x) / nx as f64;
    let zh = (o1.y - o0.y) / ny as f64;
    for k in 0..=nx {
        let x = o0.x + zw * k as f64;
        if k > 0 && k < nx {
            for (y0, y1) in [(o0.y, i0.y), (i1.y, o1.y)] {
                page.strokes.push(Stroke {
                    pen: Pen::Thin,
                    geom: line(Point2::new(x, y0), Point2::new(x, y1)),
                });
            }
        }
        if k < nx {
            let label = (nx - k).to_string();
            for y in [o0.y + 1.25, i1.y + 1.25] {
                text(page, Point2::new(x + zw / 2.0, y), 2.5, Align::Centre, &label);
            }
        }
    }
    for k in 0..=ny {
        let y = o0.y + zh * k as f64;
        if k > 0 && k < ny {
            for (x0, x1) in [(o0.x, i0.x), (i1.x, o1.x)] {
                page.strokes.push(Stroke {
                    pen: Pen::Thin,
                    geom: line(Point2::new(x0, y), Point2::new(x1, y)),
                });
            }
        }
        if k < ny {
            let label = ((b'A' + k as u8) as char).to_string();
            for x in [o0.x + BAND / 2.0, i1.x + BAND / 2.0] {
                text(
                    page,
                    Point2::new(x, y + zh / 2.0 - 1.25),
                    2.5,
                    Align::Centre,
                    &label,
                );
            }
        }
    }
    title_block(page, f, Point2::new(i1.x - TITLE_W, i0.y), third_angle);
}

/// The title block with its lower left corner at `at`.
fn title_block(page: &mut Page, f: &Fields, at: Point2, third_angle: bool) {
    let r = TITLE_H / 4.0;
    rect(page, Pen::Border, at, at + Vec2::new(TITLE_W, TITLE_H));
    // rows from the top: title; number, rev, size; material, scale, sheet; drawn, date, mass, symbol
    let rows: [&[(&str, &str, f64)]; 4] = [
        &[("TITLE", &f.title, 135.0), ("COMPANY", &f.company, 45.0)],
        &[
            ("DWG NO", &f.number, 110.0),
            ("REV", &f.rev, 25.0),
            ("SIZE", &f.size, 45.0),
        ],
        &[
            ("MATERIAL", &f.material, 110.0),
            ("SCALE", &f.scale, 35.0),
            ("SHEET", &f.sheet, 35.0),
        ],
        &[
            ("DRAWN", &f.drawn, 55.0),
            ("DATE", &f.date, 55.0),
            ("MASS", &f.mass, 35.0),
            ("", "", 35.0),
        ],
    ];
    for (k, row) in rows.iter().enumerate() {
        let y0 = at.y + TITLE_H - r * (k + 1) as f64;
        if k > 0 {
            page.strokes.push(Stroke {
                pen: Pen::Thin,
                geom: line(Point2::new(at.x, y0 + r), Point2::new(at.x + TITLE_W, y0 + r)),
            });
        }
        let mut x = at.x;
        for (i, &(label, value, cw)) in row.iter().enumerate() {
            if i > 0 {
                page.strokes.push(Stroke {
                    pen: Pen::Thin,
                    geom: line(Point2::new(x, y0), Point2::new(x, y0 + r)),
                });
            }
            if !label.is_empty() {
                text(page, Point2::new(x + 1.0, y0 + r - 2.6), 1.6, Align::Left, label);
                let vh = if k == 0 && i == 0 { 4.0 } else { 3.0 };
                let mut v = value.to_string();
                // a value too long for its cell is cut, never run into the next
                while !v.is_empty() && crate::page::text_width(&v, vh) > cw - 3.0 {
                    v.pop();
                }
                text(page, Point2::new(x + cw / 2.0, y0 + 1.6), vh, Align::Centre, &v);
            } else {
                symbol(
                    page,
                    Point2::new(x + cw / 2.0, y0 + r / 2.0),
                    r * 0.32,
                    third_angle,
                );
            }
            x += cw;
        }
    }
}

/// The projection symbol (ISO 5456-2): a truncated cone and its end view, the end view on the
/// side the cone's small end points to for third angle, on the other side for first.
fn symbol(page: &mut Page, c: Point2, s: f64, third_angle: bool) {
    let side = if third_angle { 1.0 } else { -1.0 };
    let circles = c + Vec2::new(side * 2.2 * s, 0.0);
    for r in [s, 0.55 * s] {
        page.strokes.push(Stroke {
            pen: Pen::Thin,
            geom: Geom2::Conic {
                c: circles,
                u: Vec2::new(r, 0.0),
                v: Vec2::new(0.0, r),
                t0: 0.0,
                t1: dmath::TAU,
            },
        });
    }
    // the cone: big end away from the end view
    let cone = c + Vec2::new(-side * 1.2 * s, 0.0);
    let (big, small) = (cone.x - side * 1.1 * s, cone.x + side * 1.1 * s);
    let pts = [
        Point2::new(big, cone.y - s),
        Point2::new(small, cone.y - 0.55 * s),
        Point2::new(small, cone.y + 0.55 * s),
        Point2::new(big, cone.y + s),
    ];
    for k in 0..4 {
        page.strokes.push(Stroke {
            pen: Pen::Thin,
            geom: line(pts[k], pts[(k + 1) % 4]),
        });
    }
    page.strokes.push(Stroke {
        pen: Pen::Center,
        geom: line(
            Point2::new(big - side * 0.6 * s, cone.y),
            Point2::new(circles.x + side * 1.4 * s, cone.y),
        ),
    });
}

/// A scale as a drawing writes it: `1:4`, `2:1`, `1:2.5`.
pub fn scale_text(s: f64) -> String {
    let short = |x: f64| {
        let t = format!("{x:.3}");
        t.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    if s >= 1.0 {
        format!("{}:1", short(s))
    } else {
        format!("1:{}", short(1.0 / s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_and_scales() {
        assert_eq!(size("B"), Some((431.8, 279.4)));
        assert_eq!(scale_text(0.25), "1:4");
        assert_eq!(scale_text(2.0), "2:1");
        assert_eq!(scale_text(0.4), "1:2.5");
        let mut p = Page {
            w: 431.8,
            h: 279.4,
            ..Page::default()
        };
        format(&mut p, &Fields::default(), true);
        assert!(p.strokes.len() > 20);
        assert!(p.texts.iter().any(|t| t.text == "DWG NO"));
    }
}

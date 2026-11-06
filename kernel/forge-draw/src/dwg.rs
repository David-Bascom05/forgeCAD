//! A page as an AutoCAD R2000 DWG: the same drawing the DXF writer makes (a layer per line
//! type with its colour, line type and lineweight; lines, arcs, circles and ellipses exactly,
//! other curves as polylines; text; arrowheads as solids), written natively by forge-dwg.

use crate::dxf::{colour, dxf_text, line_type, line_types, weight};
use crate::geom2::Geom2;
use crate::page::{Align, Page, Pen};
use forge_dwg::{Doc, Entity, HAlign, Layer, LineType, Shape, Weight};
use forge_math::dmath;

fn pt(p: forge_math::Point2) -> [f64; 2] {
    [p.x, p.y]
}

fn shape(g: &Geom2) -> Shape {
    match g {
        Geom2::Line { a, b } => Shape::Line { a: pt(*a), b: pt(*b) },
        Geom2::Conic { c, u, v, t0, t1 } => {
            if let Some((_, r)) = g.circle() {
                if t1 - t0 >= dmath::TAU * (1.0 - 1e-12) {
                    return Shape::Circle { c: pt(*c), r };
                }
                let alpha = dmath::atan2(u.y, u.x);
                let (lo, hi) = if u.cross(*v) > 0.0 {
                    (alpha + t0, alpha + t1)
                } else {
                    (alpha - t1, alpha - t0)
                };
                return Shape::Arc {
                    c: pt(*c),
                    r,
                    start: lo,
                    end: hi,
                };
            }
            // principal axes of the conjugate pair, as the DXF writer finds them
            let (a, b, k) = (u.dot(*u), v.dot(*v), u.dot(*v));
            let tau = 0.5 * dmath::atan2(2.0 * k, a - b);
            let (st, ct) = dmath::sin_cos(tau);
            let (mut major, mut minor) = (*u * ct + *v * st, *v * ct - *u * st);
            let mut shift = tau;
            if minor.length() > major.length() {
                let m = major;
                major = minor;
                minor = m * -1.0;
                shift += dmath::FRAC_PI_2;
            }
            let ccw = major.cross(minor) > 0.0;
            let (p0, p1) = if ccw {
                (t0 - shift, t1 - shift)
            } else {
                (shift - t1, shift - t0)
            };
            Shape::Ellipse {
                c: pt(*c),
                major: [major.x, major.y],
                ratio: minor.length() / major.length(),
                t0: p0,
                t1: p1,
            }
        }
        Geom2::Poly(pts) => Shape::Polyline {
            points: pts.iter().map(|p| pt(*p)).collect(),
            closed: false,
        },
    }
}

fn mm100(p: Pen) -> Weight {
    match weight(p).parse::<i16>() {
        Ok(w) if w >= 0 => Weight::Mm100(w),
        _ => Weight::Default,
    }
}

/// The page as DWG bytes.
pub fn dwg(p: &Page) -> Vec<u8> {
    let mut layers = vec![Layer {
        name: "0".into(),
        colour: 7,
        line_type: "Continuous".into(),
        weight: Weight::Default,
    }];
    for pen in Pen::ALL {
        layers.push(Layer {
            name: pen.layer().into(),
            colour: colour(pen).parse().unwrap_or(7),
            line_type: line_type(pen).into(),
            weight: mm100(pen),
        });
    }
    layers.push(Layer {
        name: "TEXT".into(),
        colour: 3,
        line_type: "Continuous".into(),
        weight: Weight::Default,
    });
    let line_types = line_types()
        .into_iter()
        .map(|(n, d, pat)| LineType {
            name: n.into(),
            description: d.into(),
            pattern: pat,
        })
        .collect();
    let mut entities = vec![];
    for s in &p.strokes {
        entities.push(Entity {
            layer: s.pen.layer().into(),
            shape: shape(&s.geom),
        });
    }
    for (pen, poly) in &p.fills {
        for k in 1..poly.len().saturating_sub(1) {
            let q = [poly[0], poly[k], poly[k + 1], poly[k + 1]];
            entities.push(Entity {
                layer: pen.layer().into(),
                shape: Shape::Solid { corners: q.map(pt) },
            });
        }
    }
    for t in &p.texts {
        entities.push(Entity {
            layer: if t.pen == Pen::Sick { "SICK" } else { "TEXT" }.into(),
            shape: Shape::Text {
                at: pt(t.at),
                h: t.h,
                angle: t.angle.to_radians(),
                align: match t.align {
                    Align::Left => HAlign::Left,
                    Align::Centre => HAlign::Centre,
                    Align::Right => HAlign::Right,
                },
                text: dxf_text(&t.text),
            },
        });
    }
    forge_dwg::write(&Doc {
        layers,
        line_types,
        entities,
        limits: ([0.0, 0.0], [p.w, p.h]),
        font: "arial.ttf".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::{Stroke, Text};
    use forge_math::{Point2, Vec2};

    #[test]
    fn a_page_reads_back_with_its_layers_and_entities() {
        let page = Page {
            w: 297.0,
            h: 210.0,
            strokes: vec![
                Stroke {
                    pen: Pen::Hidden,
                    geom: Geom2::Line {
                        a: Point2::new(0.0, 0.0),
                        b: Point2::new(10.0, 0.0),
                    },
                },
                Stroke {
                    pen: Pen::Visible,
                    geom: Geom2::Conic {
                        c: Point2::new(50.0, 50.0),
                        u: Vec2::new(10.0, 5.0),
                        v: Vec2::new(-2.0, 6.0),
                        t0: 0.3,
                        t1: 2.0,
                    },
                },
            ],
            fills: vec![(
                Pen::Thin,
                vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(2.0, 0.0),
                    Point2::new(1.0, 1.0),
                ],
            )],
            texts: vec![Text {
                at: Point2::new(5.0, 5.0),
                h: 3.5,
                angle: 0.0,
                align: Align::Centre,
                text: "\u{2300}6".into(),
                pen: Pen::Thin,
            }],
        };
        let d = forge_dwg::file::read(&dwg(&page)).unwrap();
        assert!(d.warnings.is_empty());
        assert!(d.layer("HIDDEN").is_some() && d.layer("VISIBLE").is_some());
        let kinds: Vec<String> = d
            .objects
            .values()
            .filter(|o| o.is_entity())
            .map(|o| o.data.name())
            .collect();
        for k in ["Line", "Ellipse", "Solid", "Text"] {
            assert!(kinds.iter().any(|x| x == k), "{k} in {kinds:?}");
        }
    }
}

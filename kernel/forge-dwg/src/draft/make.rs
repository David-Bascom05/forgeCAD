//! New entities' data, as the draw commands make them: TEXT and MTEXT in the current text style,
//! POINT, a SPLINE through fit points, and a HATCH (solid or a predefined pattern) on a found
//! boundary. Curves (lines, circles, arcs, ellipses, polylines) come from `entity::to_data`.

use super::boundary::{Boundary, Loop};
use super::curve::Curve;
use super::geom::{TAU, V2, v2, wrap};
use super::nurbs::{Nurbs, Param};
use crate::file::Drawing;
use crate::io::Href;
use crate::types::{self, Data, HatchEdge, HatchLine, HatchPath, HatchSpline};

/// A single-line TEXT, left-aligned at `at`.
pub fn text(d: &Drawing, at: V2, height: f64, rotation: f64, s: &str) -> Data {
    Data::Text(types::Text {
        insert: at.arr(),
        height,
        rotation: wrap(rotation),
        text: d.s(s),
        style: Href::new(5, d.current("TEXTSTYLE").unwrap_or(0)),
        ..types::Text::default()
    })
}

/// An MTEXT at `at` (its top left), wrapping at `width` (0: no wrapping).
pub fn mtext(d: &Drawing, at: V2, width: f64, height: f64, s: &str) -> Data {
    let ctx = types::MTextContext {
        attachment: 1,
        insert: at.arr3(0.0),
        rect_width: width,
        ..types::MTextContext::default()
    };
    Data::MText(types::MText {
        insert: at.arr3(0.0),
        rect_width: width,
        text_height: height,
        attachment: 1,
        text: d.s(s),
        style: Href::new(5, d.current("TEXTSTYLE").unwrap_or(0)),
        context: Some(ctx),
        ..types::MText::default()
    })
}

pub fn point(at: V2) -> Data {
    Data::Point(types::Point {
        at: at.arr3(0.0),
        ..types::Point::default()
    })
}

/// SPLINE's fit method: a cubic through the points (chord-length parameters), closed with a
/// matching tangent at the seam. Stored by its control points and knots.
pub fn spline_through(pts: &[V2], closed: bool) -> Option<Data> {
    let mut pts = pts.to_vec();
    pts.dedup();
    if pts.len() < 2 {
        return None;
    }
    let n = if closed && pts.len() > 2 {
        let t = (pts[1] - pts[pts.len() - 1]).unit()?;
        pts.push(pts[0]);
        let mut n = Nurbs::interpolate(&pts, 3, Some(t), Some(t), Param::Chord)?;
        n.closed = true;
        n
    } else {
        Nurbs::interpolate(&pts, 3, None, None, Param::Chord)?
    };
    super::entity::to_data(&Curve::Spline(n), 0.0)
}

/// What a hatch is filled with.
#[derive(Clone, Debug, PartialEq)]
pub enum Fill {
    Solid,
    /// A predefined pattern by name, turned and scaled.
    Pattern {
        name: String,
        angle: f64,
        scale: f64,
    },
}

/// The predefined patterns this knows: their definition lines at angle 0, scale 1 (acadiso's,
/// in millimetres): (angle in degrees, base, offset, dashes).
/// A pattern line: angle (degrees), base, offset, dashes.
type PatLine = (f64, [f64; 2], [f64; 2], Vec<f64>);

fn pattern_def(name: &str) -> Option<Vec<PatLine>> {
    Some(match name.to_ascii_uppercase().as_str() {
        "ANSI31" => vec![(45.0, [0.0, 0.0], [0.0, 3.175], vec![])],
        "ANSI32" => vec![
            (45.0, [0.0, 0.0], [0.0, 9.525], vec![]),
            (45.0, [4.49013, 0.0], [0.0, 9.525], vec![]),
        ],
        "ANSI37" => vec![
            (45.0, [0.0, 0.0], [0.0, 3.175], vec![]),
            (135.0, [0.0, 0.0], [0.0, 3.175], vec![]),
        ],
        "LINE" => vec![(0.0, [0.0, 0.0], [0.0, 3.175], vec![])],
        "NET" => vec![
            (0.0, [0.0, 0.0], [0.0, 3.175], vec![]),
            (90.0, [0.0, 0.0], [0.0, 3.175], vec![]),
        ],
        _ => return None,
    })
}

/// The pattern's lines as a HATCH stores them: turned by `angle` and scaled, offsets and bases
/// in world units.
pub fn pattern_lines(name: &str, angle: f64, scale: f64) -> Option<Vec<HatchLine>> {
    Some(
        pattern_def(name)?
            .into_iter()
            .map(|(a, base, off, dashes)| {
                let la = a.to_radians() + angle;
                // the offset is in the line's own frame
                let o = v2(off[0], off[1]).rotate(la) * scale;
                let b = v2(base[0], base[1]).rotate(angle) * scale;
                HatchLine {
                    angle: wrap(la),
                    base: b.arr(),
                    offset: o.arr(),
                    dashes: dashes.iter().map(|x| x * scale).collect(),
                }
            })
            .collect(),
    )
}

/// A curve as hatch boundary edges.
pub fn edges_of(c: &Curve) -> Vec<HatchEdge> {
    edge(c)
}

/// A hatch boundary edge as a curve.
pub fn edge_curve(e: &HatchEdge) -> Option<Curve> {
    Some(match e {
        HatchEdge::Line { a, b } => Curve::line(V2::from(*a), V2::from(*b)),
        HatchEdge::Arc {
            center,
            radius,
            start,
            end,
            ccw,
        } => {
            let mut sweep = (end - start).rem_euclid(TAU);
            if sweep == 0.0 || (end - start).abs() >= TAU - 1e-12 {
                sweep = TAU;
            }
            let (s0, sw) = if *ccw { (*start, sweep) } else { (-start, -sweep) };
            Curve::Arc(super::curve::Arc {
                c: V2::from(*center),
                r: *radius,
                start: s0,
                sweep: sw,
            })
        }
        HatchEdge::Ellipse {
            center,
            major,
            ratio,
            start,
            end,
            ccw,
        } => {
            let mut sweep = (end - start).rem_euclid(TAU);
            if sweep == 0.0 {
                sweep = TAU;
            }
            let (s0, sw) = if *ccw { (*start, sweep) } else { (-start, -sweep) };
            Curve::Ellipse(super::curve::Ellipse {
                c: V2::from(*center),
                major: V2::from(*major),
                ratio: *ratio,
                start: s0,
                sweep: sw,
            })
        }
        HatchEdge::Spline(sp) => {
            let n = Nurbs {
                degree: sp.degree.max(1) as usize,
                knots: sp.knots.clone(),
                ctrl: sp.ctrl.iter().map(|p| V2::from(*p)).collect(),
                weights: (sp.rational && sp.weights.len() == sp.ctrl.len()).then(|| sp.weights.clone()),
                closed: false,
            };
            if !n.is_valid() {
                return None;
            }
            Curve::Spline(n)
        }
    })
}

fn edge(c: &Curve) -> Vec<HatchEdge> {
    match c {
        Curve::Line { a, b } => vec![HatchEdge::Line {
            a: a.arr(),
            b: b.arr(),
        }],
        Curve::Circle { c, r } => vec![HatchEdge::Arc {
            center: c.arr(),
            radius: *r,
            start: 0.0,
            end: TAU,
            ccw: true,
        }],
        Curve::Arc(a) => {
            // clockwise arcs are stored with their angles mirrored
            let (s, e) = (a.start, a.start + a.sweep);
            let ccw = a.sweep >= 0.0;
            let (s, e) = if ccw { (s, e) } else { (-s, -e) };
            let full = a.sweep.abs() >= TAU - 1e-12;
            vec![HatchEdge::Arc {
                center: a.c.arr(),
                radius: a.r,
                start: if full { 0.0 } else { wrap(s) },
                end: if full { TAU } else { wrap(s) + (e - s) },
                ccw,
            }]
        }
        Curve::Ellipse(el) => {
            let (s, e) = (el.start, el.start + el.sweep);
            let ccw = el.sweep >= 0.0;
            let (s, e) = if ccw { (s, e) } else { (-s, -e) };
            vec![HatchEdge::Ellipse {
                center: el.c.arr(),
                major: el.major.arr(),
                ratio: el.ratio,
                start: wrap(s),
                end: wrap(s) + (e - s),
                ccw,
            }]
        }
        Curve::Spline(n) => vec![HatchEdge::Spline(HatchSpline {
            degree: n.degree as i32,
            rational: n.weights.is_some(),
            periodic: false,
            knots: n.knots.clone(),
            ctrl: n.ctrl.iter().map(|p| p.arr()).collect(),
            weights: n.weights.clone().unwrap_or_default(),
            ..HatchSpline::default()
        })],
        Curve::Poly(p) => p.segs().iter().flat_map(|s| edge(&s.curve())).collect(),
        Curve::Offset(_) => super::entity::to_data(c, 0.0)
            .and_then(|d| super::entity::from_entity(&d))
            .map(|s| edge(&s))
            .unwrap_or_default(),
        Curve::Ray { .. } | Curve::Xline { .. } => vec![],
    }
}

fn path(l: &Loop, flag: i32) -> HatchPath {
    HatchPath {
        flag,
        edges: l.iter().flat_map(edge).collect(),
        ..HatchPath::default()
    }
}

/// A HATCH on a boundary: the outer loop (external, outermost) and its islands, normal style.
pub fn hatch(d: &Drawing, b: &Boundary, fill: &Fill) -> Option<Data> {
    let mut paths = vec![path(&b.outer, 1 | 16)];
    paths.extend(b.islands.iter().map(|l| path(l, 0)));
    let seed = b.outer.first().map(|c| c.point(c.domain().0)).unwrap_or(V2::ZERO);
    let mut h = types::Hatch {
        paths,
        style: 0,
        pattern_type: 1,
        seeds: vec![seed.arr()],
        ..types::Hatch::default()
    };
    match fill {
        Fill::Solid => {
            h.solid = true;
            h.name = d.s("SOLID");
        }
        Fill::Pattern { name, angle, scale } => {
            h.solid = false;
            h.name = d.s(&name.to_ascii_uppercase());
            h.angle = *angle;
            h.scale = *scale;
            h.lines = pattern_lines(name, *angle, *scale)?;
        }
    }
    Some(Data::Hatch(h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ansi31_turned_and_scaled() {
        let l = pattern_lines("ansi31", 0.0, 2.0).unwrap();
        assert_eq!(l.len(), 1);
        assert!((l[0].angle - std::f64::consts::FRAC_PI_4).abs() < 1e-15);
        // the offset a quarter turn from the lines, 6.35 long
        let o = V2::from(l[0].offset);
        assert!((o.len() - 6.35).abs() < 1e-12);
        assert!(o.dot(V2::polar(l[0].angle, 1.0)).abs() < 1e-12);
    }

    #[test]
    fn clockwise_arcs_mirror_their_angles() {
        let a = Curve::Arc(super::super::curve::Arc {
            c: V2::ZERO,
            r: 1.0,
            start: 1.0,
            sweep: -0.5,
        });
        let HatchEdge::Arc { start, end, ccw, .. } = edge(&a)[0] else {
            panic!()
        };
        assert!(!ccw);
        assert!((start - (TAU - 1.0)).abs() < 1e-12 && (end - start - 0.5).abs() < 1e-12);
    }
}

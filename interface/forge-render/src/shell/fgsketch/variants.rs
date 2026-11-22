//! Every Sketch tab tool and variant: drawn by its clicks (and typed values), it writes the rows
//! and constraints it should, a Forge block that checks, and Edit Sketch reads that block back to
//! the same geometry, finishing unchanged.

use super::{block, open};
use crate::shell::sketching::rows::dsl::*;
use crate::shell::sketching::rows::{D, Row};
use crate::shell::sketching::{SkTool, SketchFor, SketchMode};
use forge_math::Frame;

type P = (f64, f64);

enum Step {
    Click(P),
    /// A value typed in the field: its place, its text.
    Type(usize, &'static str),
    Enter,
    Esc,
    Tool(SkTool),
}
use Step::*;

const REACH: f64 = 0.5;

fn draw(steps: &[Step]) -> SketchMode {
    let mut m = SketchMode::new(SketchFor::New, "xy".into(), Frame::default(), vec![]);
    for s in steps {
        match s {
            Click(p) => m.click(*p, REACH),
            Type(k, v) => {
                let f = &mut m.fields[*k];
                f.text = v.to_string();
                f.locked = true;
            }
            Enter => m.enter(REACH),
            Esc => m.escape(),
            Tool(t) => m.set_tool(Some(*t)),
        }
    }
    m
}

/// Each entity's kind and outline, in the order made: what the two sketches are compared by.
fn shapes(m: &SketchMode) -> Vec<(char, Vec<P>)> {
    let s = m.solved.as_ref().expect("solved");
    s.names
        .iter()
        .filter(|n| !n.0.starts_with('#') && n.0 != "o")
        .map(|n| (n.0.chars().next().unwrap_or('?'), m.outline_of(&n.0)))
        .collect()
}

/// The same shapes, to the solver's few microns (a free point settles a hair otherwise when
/// solved from its guess again).
fn same(a: &[(char, Vec<P>)], b: &[(char, Vec<P>)]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.0 == y.0
                && x.1.len() == y.1.len()
                && x.1
                    .iter()
                    .zip(&y.1)
                    .all(|(p, q)| ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt() < 5e-3)
        })
}

/// A row a drawing is to have: this one or one that starts so (its lists may go on), or one whose
/// Forge text starts so.
enum W {
    R(Row),
    S(&'static str),
}

impl W {
    fn is(&self, r: &Row) -> bool {
        let text = r.to_string();
        match self {
            W::R(w) => text.starts_with(w.to_string().trim_end_matches([')', ']'])),
            W::S(s) => text.starts_with(s),
        }
    }
}

/// Drawn as `steps`, with `want` among its rows: the Forge block checks, opens editable, solves to
/// the same shapes and finishes with nothing to write.
fn round_trip(name: &str, steps: &[Step], want: &[W]) {
    let m = draw(steps);
    assert!(m.error.is_none(), "{name}: {:?}\n{:?}", m.error, m.rows);
    assert!(!m.rows.is_empty(), "{name}: drew nothing");
    for w in want {
        assert!(
            m.rows.iter().any(|r| w.is(r)),
            "{name}: no row so in {:#?}",
            m.rows
        );
    }
    let text = block("sk", "xy", &m.rows).unwrap_or_else(|e| panic!("{name}: {}\n{:?}", e.text(), m.rows));
    let opts = forge_lang::Options {
        build: false,
        ..forge_lang::Options::default()
    };
    let run = forge_lang::run(&text, opts, Default::default()).0;
    assert!(run.errors.is_empty(), "{name}:\n{text}\n{:?}", run.errors);
    let o = open(&text, "sk", None).unwrap_or_else(|e| panic!("{name}: {}", e.text()));
    assert!(o.read_only.is_none(), "{name}: {:?}\n{text}", o.read_only);
    let mut back = SketchMode::new(
        SketchFor::Forge(o.id.clone()),
        "xy".into(),
        Frame::default(),
        o.rows.clone(),
    );
    back.set_params(o.params.clone());
    assert!(
        back.error.is_none(),
        "{name}: {:?}\n{text}\n{:?}",
        back.error,
        o.rows
    );
    let (was, now) = (shapes(&m), shapes(&back));
    assert!(
        same(&was, &now),
        "{name}: read back otherwise\n{text}\n{was:?}\n{now:?}"
    );
    let ops = o.write_back(&back.rows, back.kept()).unwrap();
    assert!(ops.is_empty(), "{name}: finished unchanged as {ops:?}");
}

#[test]
fn lines_draw_chain_infer_and_read_back() {
    round_trip(
        "Line",
        &[
            Tool(SkTool::Line),
            Click((0.0, 0.0)),
            Click((20.0, 0.4)),
            Click((20.0, 12.0)),
            Esc,
        ],
        &[
            W::R(Row::Line(at("0", "0"), at("20", "0"))),
            W::R(Row::Hor(vec![rf("l1")])),
            W::S("line(l1.b"),
            W::R(Row::Ver(vec![rf("l2")])),
        ],
    );
    round_trip(
        "Midpoint Line",
        &[Tool(SkTool::LineMid), Click((5.0, 5.0)), Click((15.0, 5.3))],
        &[
            W::R(Row::Line(at("-5", "5"), at("15", "5"))),
            W::R(Row::Hor(vec![rf("l1")])),
            W::R(Row::Pt(at("5", "5"))),
            W::R(Row::Mid(rf("p1"), rf("l1"))),
        ],
    );
    round_trip(
        "Midpoint Line, typed",
        &[
            Tool(SkTool::LineMid),
            Click((0.0, 0.0)),
            Type(0, "30"),
            Type(1, "30"),
            Click((10.0, 1.0)),
        ],
        &[
            W::S("line("),
            W::R(Row::Dim(D::Dist, vec![rf("l1")], Some(v("30")))),
            W::R(Row::Mid(rf("p1"), rf("l1"))),
        ],
    );
    let m = draw(&[
        Tool(SkTool::LineMid),
        Click((0.0, 0.0)),
        Type(0, "30"),
        Type(1, "30"),
        Click((10.0, 1.0)),
    ]);
    let s = m.solved.as_ref().unwrap();
    let (a, b) = crate::shell::sketching::rows::line(s, "l1").unwrap();
    assert!((((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt() - 30.0).abs() < 1e-6);
    assert!((forge_math::dmath::atan2(b.y - a.y, b.x - a.x).to_degrees() - 30.0).abs() < 1e-6);
    assert!(
        (a.x + b.x).abs() < 1e-6 && (a.y + b.y).abs() < 1e-6,
        "centred on the click"
    );
    // construction lines, from the Line dropdown's toggle
    round_trip(
        "Construction line",
        &[
            Tool(SkTool::Construction),
            Tool(SkTool::LineMid),
            Click((0.0, 0.0)),
            Click((0.0, 9.0)),
        ],
        &[
            W::R(Row::Line(at("0", "-9"), at("0", "9")).cons()),
            W::R(Row::Ver(vec![rf("l1")])),
        ],
    );
}

#[test]
fn circles_and_arcs_by_each_way() {
    round_trip(
        "Center Point Circle",
        &[
            Tool(SkTool::Circle),
            Click((0.0, 0.0)),
            Type(0, "12"),
            Click((5.0, 0.0)),
        ],
        &[
            W::R(Row::Circle(at("0", "0"), v("6"))),
            W::R(Row::Dim(D::Dia, vec![rf("c1")], Some(v("12")))),
        ],
    );
    round_trip(
        "Three Point Circle",
        &[
            Tool(SkTool::Circle3),
            Click((-5.0, 0.0)),
            Click((5.0, 0.0)),
            Click((0.0, 5.0)),
        ],
        &[W::R(Row::Circle(at("0", "0"), v("5")))],
    );
    round_trip(
        "Tangent Circle",
        &[
            Tool(SkTool::Line),
            Click((0.0, 0.0)),
            Click((20.0, 0.0)),
            Click((10.0, 15.0)),
            Click((0.0, 0.0)),
            Esc,
            Esc,
            Tool(SkTool::CircleTan),
            Click((10.0, 0.0)),
            Click((15.0, 7.5)),
            Click((5.0, 7.5)),
        ],
        &[
            W::S("circle("),
            W::R(Row::Tan(rf("l1"), rf("c1"))),
            W::R(Row::Tan(rf("l2"), rf("c1"))),
            W::R(Row::Tan(rf("l3"), rf("c1"))),
        ],
    );
    round_trip(
        "Three Point Arc",
        &[
            Tool(SkTool::Arc3),
            Click((-5.0, 0.0)),
            Click((5.0, 0.0)),
            Click((0.0, 5.0)),
        ],
        &[W::R(Row::Arc(at("0", "0"), at("-5", "0"), at("5", "0"), true))],
    );
    round_trip(
        "Center Point Arc",
        &[
            Tool(SkTool::Arc),
            Click((0.0, 0.0)),
            Click((5.0, 0.0)),
            Click((0.0, 5.0)),
        ],
        &[W::S("arc(center: near(0, 0), from: near(5, 0)")],
    );
    round_trip(
        "Tangent Arc",
        &[
            Tool(SkTool::Line),
            Click((0.0, 0.0)),
            Click((10.0, 0.0)),
            Esc,
            Esc,
            Tool(SkTool::ArcTan),
            Click((10.0, 0.0)),
            Click((10.0, 10.0)),
        ],
        &[
            W::R(Row::Arc(at("10", "5"), pn("l1.b"), at("10", "10"), false)),
            W::R(Row::Tan(rf("l1"), rf("a1"))),
        ],
    );
}

#[test]
fn rectangles_two_and_three_point_by_corner_and_by_centre() {
    round_trip(
        "Two Point Rectangle",
        &[Tool(SkTool::Rectangle), Click((0.0, 0.0)), Click((30.0, 20.0))],
        &[W::R(Row::Rect(v("0"), v("0"), v("30"), v("20")))],
    );
    round_trip(
        "Three Point Rectangle",
        &[
            Tool(SkTool::Rect3),
            Click((0.0, 0.0)),
            Click((20.0, 0.3)),
            Click((12.0, 8.0)),
        ],
        &[
            W::R(Row::Poly(
                vec![at("0", "0"), at("20", "0"), at("20", "8"), at("0", "8")],
                false,
            )),
            W::R(Row::Perp(rf("l1"), rf("l2"))),
            W::R(Row::Hor(vec![rf("l1")])),
        ],
    );
    round_trip(
        "Two Point Center Rectangle",
        &[Tool(SkTool::RectCenter), Click((0.0, 0.0)), Click((15.0, 10.0))],
        &[
            W::R(Row::Rect(v("-15"), v("-10"), v("30"), v("20"))),
            W::R(Row::Line(pn("l1.a"), pn("l3.a")).cons()),
            W::R(Row::Line(pn("l2.a"), pn("l4.a")).cons()),
            W::R(Row::Pt(at("0", "0"))),
            W::R(Row::Mid(rf("p1"), rf("l5"))),
        ],
    );
    round_trip(
        "Two Point Center Rectangle, typed",
        &[
            Tool(SkTool::RectCenter),
            Click((5.0, 5.0)),
            Type(0, "40"),
            Type(1, "16"),
            Click((9.0, 9.0)),
        ],
        &[
            W::R(Row::Rect(v("-15"), v("-3"), v("40"), v("16"))),
            W::R(Row::Dim(D::HDist, vec![rf("l1.a"), rf("l1.b")], Some(v("40")))),
            W::R(Row::Dim(D::VDist, vec![rf("l2.a"), rf("l2.b")], Some(v("16")))),
            W::R(Row::Mid(rf("p1"), rf("l5"))),
        ],
    );
    // its centre snapped to a point there: tied to that point
    round_trip(
        "Two Point Center Rectangle on a point",
        &[
            Tool(SkTool::Point),
            Click((2.0, 3.0)),
            Tool(SkTool::RectCenter),
            Click((2.1, 3.1)),
            Click((12.0, 9.0)),
        ],
        &[
            W::R(Row::Rect(v("-8"), v("-3"), v("20"), v("12"))),
            W::R(Row::Mid(rf("p1"), rf("l5"))),
        ],
    );
    round_trip(
        "Three Point Center Rectangle",
        &[
            Tool(SkTool::RectCenter3),
            Click((0.0, 0.0)),
            Click((10.0, 10.0)),
            Click((-3.0, 3.0)),
        ],
        &[
            W::R(Row::Poly(vec![], false)),
            W::R(Row::Perp(rf("l1"), rf("l2"))),
            W::R(Row::Mid(rf("p1"), rf("l5"))),
        ],
    );
    let m = draw(&[
        Tool(SkTool::RectCenter3),
        Click((0.0, 0.0)),
        Type(0, "30"),
        Click((10.0, 0.2)),
        Type(0, "12"),
        Click((3.0, 4.0)),
    ]);
    assert!(
        m.rows.iter().any(|r| *r
            == Row::Poly(
                vec![at("-15", "-6"), at("15", "-6"), at("15", "6"), at("-15", "6")],
                false
            )),
        "{:?}",
        m.rows
    );
    assert!(
        m.rows
            .iter()
            .any(|r| *r == Row::Dim(D::Dist, vec![rf("l1")], Some(v("30"))))
            && m.rows
                .iter()
                .any(|r| *r == Row::Dim(D::Dist, vec![rf("l2")], Some(v("12"))))
    );
    assert!(
        m.rows.iter().any(|r| *r == Row::Hor(vec![rf("l1")])),
        "{:?}",
        m.rows
    );
}

#[test]
fn every_slot() {
    let cases: [(SkTool, &str, &[P]); 5] = [
        (
            SkTool::SlotCC,
            "Center to Center Slot",
            &[(0.0, 0.0), (20.0, 0.0), (10.0, 4.0)],
        ),
        (
            SkTool::SlotOverall,
            "Overall Slot",
            &[(0.0, 0.0), (30.0, 0.0), (10.0, 4.0)],
        ),
        (
            SkTool::SlotCenter,
            "Center Point Slot",
            &[(0.0, 0.0), (10.0, 0.0), (0.0, 4.0)],
        ),
        (
            SkTool::SlotArc3,
            "Three Point Arc Slot",
            &[(-20.0, 0.0), (20.0, 0.0), (0.0, 20.0), (0.0, 23.0)],
        ),
        (
            SkTool::SlotArcCenter,
            "Center Point Arc Slot",
            &[(0.0, 0.0), (20.0, 0.0), (0.0, 20.0), (0.0, 23.0)],
        ),
    ];
    for (tool, name, pts) in cases {
        let mut steps = vec![Tool(tool)];
        steps.extend(pts.iter().map(|p| Click(*p)));
        let want: &[W] = if matches!(tool, SkTool::SlotArc3 | SkTool::SlotArcCenter) {
            &[W::S("arc("), W::R(Row::Conc(rf("a1"), rf("a3")))]
        } else {
            &[
                W::S("line("),
                W::S("arc("),
                W::R(Row::Tan(rf("l1"), rf("a1"))),
                W::R(Row::Hor(vec![rf("l1")])),
            ]
        };
        round_trip(name, &steps, want);
    }
    // a typed width is the end arcs' radius
    round_trip(
        "Center to Center Slot, typed",
        &[
            Tool(SkTool::SlotCC),
            Click((0.0, 0.0)),
            Type(0, "25"),
            Click((10.0, 0.0)),
            Type(0, "8"),
            Click((5.0, 9.0)),
        ],
        &[
            W::R(Row::Line(at("0", "4"), at("25", "4"))),
            W::R(Row::Dim(D::Dist, vec![rf("l1")], Some(v("25")))),
            W::R(Row::Dim(D::Rad, vec![rf("a1")], Some(v("4")))),
        ],
    );
}

#[test]
fn splines_through_and_by_control_vertices() {
    let pts = [(0.0, 0.0), (10.0, 10.0), (20.0, 0.0), (30.0, 10.0)];
    for (tool, name, row) in [
        (
            SkTool::Spline,
            "Spline Interpolation",
            W::R(Row::Spline(vec![at("0", "0")], false)),
        ),
        // cubic, open
        (
            SkTool::SplineCv,
            "Spline Control Vertex",
            W::S("spline([near(0, 0), "),
        ),
    ] {
        let mut steps = vec![Tool(tool)];
        steps.extend(pts.iter().map(|p| Click(*p)));
        steps.push(Enter);
        round_trip(name, &steps, &[row]);
        if tool == SkTool::SplineCv {
            let m = draw(&steps);
            assert!(
                matches!(&m.rows[0], Row::BSpline(d, _, true) if *d == v("3")),
                "{:?}",
                m.rows
            );
        }
    }
    let m = draw(&[
        Tool(SkTool::SplineCv),
        Click((0.0, 0.0)),
        Click((10.0, 10.0)),
        Click((20.0, 0.0)),
        Enter,
    ]);
    assert_eq!(
        m.rows,
        [Row::BSpline(
            v("2"),
            vec![at("0", "0"), at("10", "10"), at("20", "0")],
            true
        )]
    );
    let text = block("sk", "xy", &m.rows).unwrap();
    assert!(text.contains("closed: false, cv: true)"), "{text}");
}

#[test]
fn three_ellipses() {
    round_trip(
        "Center Point Ellipse",
        &[
            Tool(SkTool::Ellipse),
            Click((0.0, 0.0)),
            Click((10.0, 0.0)),
            Click((3.0, 4.0)),
        ],
        &[W::R(Row::Ellipse(at("0", "0"), at("10", "0"), v("4")))],
    );
    round_trip(
        "Axis Ellipse",
        &[
            Tool(SkTool::EllipseAxis),
            Click((-10.0, 2.0)),
            Click((10.0, 2.2)),
            Click((0.0, 6.0)),
        ],
        &[W::R(Row::Ellipse(at("0", "2"), at("10", "2"), v("4")))],
    );
    round_trip(
        "Axis Ellipse, typed",
        &[
            Tool(SkTool::EllipseAxis),
            Click((0.0, 0.0)),
            Type(0, "24"),
            Click((0.0, 7.0)),
            Type(0, "5"),
            Click((3.0, 3.0)),
        ],
        &[
            W::R(Row::Ellipse(at("0", "12"), at("0", "24"), v("5"))),
            W::R(Row::Dim(D::Rad, vec![rf("e1")], Some(v("5")))),
        ],
    );
    round_trip(
        "Bounding Box Ellipse",
        &[Tool(SkTool::EllipseBox), Click((0.0, 0.0)), Click((20.0, 10.0))],
        &[W::R(Row::Ellipse(at("10", "5"), at("20", "5"), v("5")))],
    );
    // a box taller than wide stands its major axis up
    round_trip(
        "Bounding Box Ellipse, tall",
        &[Tool(SkTool::EllipseBox), Click((0.0, 0.0)), Click((6.0, 20.0))],
        &[W::R(Row::Ellipse(at("3", "10"), at("3", "20"), v("3")))],
    );
}

#[test]
fn polygons_inscribed_and_circumscribed() {
    round_trip(
        "Inscribed Polygon",
        &[Tool(SkTool::Polygon), Click((0.0, 0.0)), Click((10.0, 0.0))],
        &[
            W::R(Row::Poly(vec![at("10", "0"), at("5", "8.66")], false)),
            W::R(Row::Eq(vec![rf("l1"), rf("l2")])),
        ],
    );
    let m = draw(&[Tool(SkTool::PolygonOut), Click((0.0, 0.0)), Click((10.0, 0.0))]);
    // the click is a side's middle: that side stands at x = 10, plumb
    let s = m.solved.as_ref().unwrap();
    let (a, b) = crate::shell::sketching::rows::line(s, "l1").unwrap();
    assert!(
        (a.x - 10.0).abs() < 1e-2 && (b.x - 10.0).abs() < 1e-2,
        "{a:?} {b:?}"
    );
    round_trip(
        "Circumscribed Polygon",
        &[
            Tool(SkTool::PolygonOut),
            Click((0.0, 0.0)),
            Type(0, "8"),
            Click((0.0, 3.0)),
        ],
        &[
            W::R(Row::Poly(vec![], false)),
            W::R(Row::Eq(vec![rf("l1"), rf("l2")])),
        ],
    );
}

#[test]
fn points_corners_and_text() {
    round_trip(
        "Point",
        &[Tool(SkTool::Point), Click((3.0, 4.0))],
        &[W::R(Row::Pt(at("3", "4")))],
    );
    for (tool, name) in [(SkTool::SkFillet, "Fillet"), (SkTool::SkChamfer, "Chamfer")] {
        round_trip(
            name,
            &[
                Tool(SkTool::Line),
                Click((0.0, 0.0)),
                Click((20.0, 0.0)),
                Click((20.0, 20.0)),
                Esc,
                Esc,
                Tool(tool),
                Click((19.0, 1.0)),
            ],
            &[W::S("line(")],
        );
    }
}

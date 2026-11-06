//! Writes a Doc with every shape through the R2000 exporter. `dwgexport_test OUT.dwg`
use forge_dwg::*;
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let e = |layer: &str, shape: Shape| Entity { layer: layer.into(), shape };
    let doc = Doc {
        layers: vec![
            Layer { name: "0".into(), colour: 7, line_type: "Continuous".into(), weight: Weight::Default },
            Layer { name: "VISIBLE".into(), colour: 7, line_type: "Continuous".into(), weight: Weight::Mm100(50) },
            Layer { name: "HIDDEN".into(), colour: 8, line_type: "HIDDEN".into(), weight: Weight::Mm100(35) },
        ],
        line_types: vec![LineType { name: "HIDDEN".into(), description: "Hidden __ __".into(), pattern: vec![3.0, -1.5] }],
        entities: vec![
            e("VISIBLE", Shape::Line { a: [0.0, 0.0], b: [100.0, 50.0] }),
            e("VISIBLE", Shape::Circle { c: [50.0, 50.0], r: 20.0 }),
            e("HIDDEN", Shape::Arc { c: [80.0, 20.0], r: 10.0, start: 0.2, end: 2.5 }),
            e("VISIBLE", Shape::Ellipse { c: [120.0, 60.0], major: [15.0, 5.0], ratio: 0.4, t0: 0.0, t1: 4.0 }),
            e("VISIBLE", Shape::Polyline { points: vec![[0.0, 80.0], [20.0, 90.0], [40.0, 80.0]], closed: false }),
            e("VISIBLE", Shape::Spline { degree: 2, knots: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0], points: vec![[60.0, 80.0], [70.0, 95.0], [80.0, 80.0]], weights: Some(vec![1.0, 0.7071, 1.0]) }),
            e("VISIBLE", Shape::Point { at: [5.0, 5.0] }),
            e("VISIBLE", Shape::Solid { corners: [[90.0, 0.0], [95.0, 0.0], [90.0, 5.0], [95.0, 5.0]] }),
            e("TEXT", Shape::Text { at: [10.0, 30.0], h: 3.5, angle: 0.0, align: HAlign::Left, text: "%%c6 H7 \u{b0} caf\u{e9}".into() }),
            e("TEXT", Shape::Text { at: [60.0, 30.0], h: 2.5, angle: 0.5, align: HAlign::Centre, text: "CENTRED".into() }),
        ],
        limits: ([0.0, 0.0], [297.0, 210.0]),
        font: "txt".into(),
    };
    std::fs::write(&out, write(&doc)).unwrap();
}

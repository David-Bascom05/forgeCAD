//! Sheets and thin walls to the bit, natively and as wasm32-wasip1 (the two must print the same):
//! each case's measures and every vertex, as f64 bit patterns hashed.
//!   cargo run --release -p forge-model --example sheetbits [--target wasm32-wasip1]
use forge_geom::spline::PlaneSpline;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{Axis2, Segment, extrude_sheet, revolve_sheet, thicken};
use forge_topo::Body;
use forge_topo::mass::mass;
use std::sync::Arc;

fn line(a: (f64, f64), b: (f64, f64)) -> Segment {
    Segment::Line {
        a: Point2::new(a.0, a.1),
        b: Point2::new(b.0, b.1),
    }
}

fn hash(b: &Body) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for v in &b.vertices {
        for x in [v.point.x, v.point.y, v.point.z] {
            h = (h ^ x.to_bits()).wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

fn show(name: &str, b: &Body) {
    let m = mass(b).expect("measured");
    println!(
        "{name:12} faces {:3} vol {:016x} area {:016x} sheet {:016x} verts {:016x}",
        b.faces.len(),
        m.volume.to_bits(),
        m.area.to_bits(),
        m.sheet_area.to_bits(),
        hash(b)
    );
}

fn main() {
    let l = vec![
        (line((0.0, 0.0), (40.0, 0.0)), 1),
        (line((40.0, 0.0), (40.0, 25.0)), 2),
    ];
    let arc = vec![
        (line((0.0, 0.0), (10.0, 0.0)), 1),
        (
            Segment::Arc {
                center: Point2::new(10.0, 5.0),
                radius: 5.0,
                start: -dmath::PI / 2.0,
                sweep: dmath::PI / 2.0,
            },
            2,
        ),
    ];
    let sp = PlaneSpline::through(
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(8.0, 3.0),
            Point2::new(16.0, -2.0),
            Point2::new(24.0, 4.0),
        ],
        None,
        None,
    )
    .expect("a spline");
    let spline = vec![(Segment::Spline(Arc::new(sp)), 1)];
    let y = Axis2 {
        origin: Point2::ORIGIN,
        dir: forge_math::Vec2 { x: 0.0, y: 1.0 },
    };
    for (name, chain) in [("L", l), ("arc", arc), ("spline", spline)] {
        let s = extrude_sheet(&Frame::WORLD, std::slice::from_ref(&chain), 10.0, 1).expect("a sheet");
        show(&format!("{name}-sheet"), &s);
        let faces: Vec<_> = s.face_ids().map(|f| s.face(f).name).collect();
        for (side, lo, hi) in [("out", 0.0, 1.5), ("in", -1.5, 0.0), ("both", -0.75, 0.75)] {
            show(
                &format!("{name}-{side}"),
                &thicken(&s, &faces, lo, hi, 2).expect("a wall"),
            );
        }
    }
    let wall = vec![
        (line((5.0, 0.0), (5.0, 8.0)), 1),
        (line((5.0, 8.0), (2.0, 12.0)), 2),
    ];
    show(
        "revolve",
        &revolve_sheet(&Frame::WORLD, &[wall], y, dmath::TAU, 1).expect("a sheet"),
    );
}

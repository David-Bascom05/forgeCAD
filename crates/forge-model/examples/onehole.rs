//! One small through hole drilled into the finished torture plate, timed over many runs:
//! the cost of a local boolean on a big body. ONEHOLE_RUNS sets the count (default 2000).
use forge_bool::{Op, boolean_into};
use forge_math::{Frame, Point2, Point3};
use forge_model::torture;
use forge_ops::{Profile, extrude};
use std::time::Instant;
fn main() {
    let body = torture::part(usize::MAX).regenerate().expect("regenerates");
    // the regen keeps its boxes cached on the body; warm them so each clone carries them
    for f in body.face_ids() {
        body.face_box(f);
    }
    let profile = Profile {
        plane: Frame::WORLD.with_origin(Point3::new(3.0, 3.0, -1.0)),
        loops: vec![Profile::circle(Point2::ORIGIN, 1.0, 0)],
    };
    let tool = extrude(&profile, 100.0, 9999).expect("tool");
    let runs: usize = std::env::var("ONEHOLE_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2000);
    let (mut clone_ns, mut bool_ns) = (0u128, 0u128);
    for _ in 0..runs {
        let t0 = Instant::now();
        let mut b = body.clone();
        let t1 = Instant::now();
        boolean_into(&mut b, &tool, Op::Subtract, 9999).expect("boolean");
        let t2 = Instant::now();
        std::hint::black_box(&b);
        clone_ns += (t1 - t0).as_nanos();
        bool_ns += (t2 - t1).as_nanos();
    }
    println!(
        "{} faces: clone {:.1} us, hole {:.1} us",
        body.faces.len(),
        clone_ns as f64 / runs as f64 / 1e3,
        bool_ns as f64 / runs as f64 / 1e3
    );
}

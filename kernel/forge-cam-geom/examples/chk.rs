use forge_cam_geom::check::*;
use forge_cam_geom::ival::Ival;
use forge_cam_geom::tool::{Cutter, assemble};
fn main() {
    let t = assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap();
    let cut = &t.bodies[0];
    let f = Face::new(Surf::Plane { o: [10.0, 0.0, 0.0], u: [0.0, 1.0, 0.0], v: [0.0, 0.0, 1.0] }, Ival::new(-50.0, 50.0), Ival::new(-20.0, 0.0), None, 0.0, 0);
    for (x, b) in [(7.0, 100_000), (7.01, 100_000), (6.0, 100_000)] {
        let mv = Move::Line { a: [x, -20.0, -5.0], b: [x, 20.0, -5.0] };
        let t0 = std::time::Instant::now();
        let (v, n) = certify(cut, &f, &mv, -0.001, 1e-4, b);
        println!("x={x}: {v:?} cells {n} {:?}", t0.elapsed());
    }
    let boss = Face::new(Surf::Cyl { o: [0.0, 0.0, -10.0], x: [1.0, 0.0, 0.0], y: [0.0, 1.0, 0.0], ax: [0.0, 0.0, 1.0], r: 10.0 }, Ival::new(-1.0, 1.0), Ival::new(0.0, 10.0), None, 0.0, 1);
    for r in [13.0, 12.98, 13.5] {
        let mv = Move::Arc { c: [0.0, 0.0, -5.0], x: [1.0, 0.0, 0.0], y: [0.0, 1.0, 0.0], r, t0: -0.5, t1: 0.5 };
        let t0 = std::time::Instant::now();
        let (v, n) = certify(cut, &boss, &mv, -0.01, 2e-3, 200_000);
        println!("arc r={r}: {v:?} cells {n} {:?}", t0.elapsed());
    }
}

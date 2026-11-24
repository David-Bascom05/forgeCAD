use forge_cam_geom::check::{Face, Move, Surf};
use forge_cam_geom::drop::{Column, drop, gap_move};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::tool::{Cutter, assemble};
fn main() {
    // a concave fillet: the quarter of a cylinder (axis along y, radius 4, centre z = 4) below
    // and beside its axis: angles from straight down to straight -x
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let th = (std::f64::consts::PI / 8.0).tan();
    let f = Face::new(
        Surf::Cyl { o: [0.0, 0.0, 4.0], x: [-s, 0.0, -s], y: [-s, 0.0, s], ax: [0.0, 1.0, 0.0], r: 4.0 },
        Ival::new(-th, th), Ival::new(-20.0, 20.0), None, 1e-5, 0);
    let b = assemble(&Cutter::Ball { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap().bodies[0].clone();
    let col = Column::of(&b).unwrap();
    println!("bbox z {:?}; closed {:?}", f.bbox[2], forge_cam_geom::drop::closed(&col, &f, -0.5, 0.0));
    {
        let x = -0.5;
        let a = drop(&col, std::slice::from_ref(&f), &[0], x, -1.0, 1e-4, 1_000_000);
        let c = drop(&col, std::slice::from_ref(&f), &[0], x, 1.0, 1e-4, 1_000_000);
        let mv = Move::Line { a: [x, -1.0, a.hi], b: [x, 1.0, c.hi] };
        for lim in [-0.0025, -0.005, -0.01, -0.02, -0.04] {
            let (g, n) = gap_move(&col, &f, &mv, lim, 1e-4, 1_000_000);
            println!("limit {lim}: {g:?} cells {n}");
        }
    }
    for x in [-0.5f64, -1.0, -2.0] {
        let t = std::time::Instant::now();
        let a = drop(&col, std::slice::from_ref(&f), &[0], x, -1.0, 1e-3, 1_000_000);
        let c = drop(&col, std::slice::from_ref(&f), &[0], x, 1.0, 1e-3, 1_000_000);
        let mv = Move::Line { a: [x, -1.0, a.hi], b: [x, 1.0, c.hi] };
        let (g, n) = gap_move(&col, &f, &mv, -0.005, 1e-3, 1_000_000);
        println!("along x={x}: drop {a:?}; gap {g:?} cells {n} {:?}", t.elapsed());
        let a = drop(&col, std::slice::from_ref(&f), &[0], x, 0.0, 1e-3, 1_000_000);
        let c = drop(&col, std::slice::from_ref(&f), &[0], x - 0.5, 0.0, 1e-3, 1_000_000);
        let mv = Move::Line { a: [x, 0.0, a.hi], b: [x - 0.5, 0.0, c.hi] };
        let t = std::time::Instant::now();
        let (g, n) = gap_move(&col, &f, &mv, -0.005, 1e-3, 1_000_000);
        println!("across x={x}: gap {g:?} cells {n} {:?}", t.elapsed());
    }
}
#[allow(dead_code)]
fn dbg() {}

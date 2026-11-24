#[path = "surf_dome.rs"]
#[allow(dead_code)]
mod dome;
use forge_cam_geom::check::Move;
use forge_cam_geom::drop::{Column, closed, drop, gap_move};
use forge_cam_geom::ival::Ival;
use forge_tooling::Library;
fn main() {
    let faces = dome::dome();
    let lib = Library::builtin();
    let tl = lib.tool("ball-6").unwrap().clone();
    let shape = tl.shape(tl.lcf + 15.0, None).unwrap();
    let col = Column::of(&shape.bodies[0]).unwrap();
    let f = &faces[2];
    let mv = Move::Line { a: [14.285714281357144, 19.266943255979413, -0.049999899999998224], b: [14.311224485478318, 19.28436704939503, -0.049999899999998224] };
    for budget in [1000usize, 10_000, 100_000] {
        let t = std::time::Instant::now();
        let g = gap_move(&col, f, &mv, -0.01, 0.0025, budget);
        println!("budget {budget}: {g:?} {:?}", t.elapsed());
    }
    let mv2 = Move::Line { a: [14.285714281357144, 19.266943255979413, 0.0], b: [14.311224485478318, 19.28436704939503, 0.0] };
    println!("at 0: {:?}", gap_move(&col, f, &mv2, -0.01, 0.0025, 100_000));
    println!("cl {:?}", drop(&col, &faces, &[0, 1, 2], 14.285714281357144, 19.266943255979413, 1e-5, 100000));
    for (x, y) in [(31.0, 32.0), (30.0, 30.0), (36.0, 31.0), (30.5, 30.2)] {
        let c = drop(&col, &faces, &[0, 1, 2], x, y, 1e-4, 100_000);
        let cf = closed(&col, f, x, y);
        let holds = cf.and_then(|c| c.1).map(|(p, q)| f.holds(Ival::pt(p), Ival::pt(q)));
        let mv = Move::Line { a: [x, y, c.hi - 0.05], b: [x + 0.5, y, c.hi - 0.05] };
        let t = std::time::Instant::now();
        let g = gap_move(&col, f, &mv, -0.0025, 0.0025, 200_000);
        println!("({x},{y}) cl {c:?} closed {cf:?} holds {holds:?} gap {g:?} {:?}", t.elapsed());
    }
}

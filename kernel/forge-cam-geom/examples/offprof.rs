use forge_cam_geom::offset::offset;
use forge_cam_geom::region::{Loop, Region, Seg, p2};
fn main() {
    let (a, b, c, d) = (p2(0.0, 0.0), p2(40.0, 0.0), p2(40.0, 10.0), p2(0.0, 10.0));
    let r = Loop { segs: vec![Seg::Line { a, b }, Seg::Line { a: b, b: c }, Seg::Line { a: c, b: d }, Seg::Line { a: d, b: a }] };
    let h = p2(22.0, 5.0);
    let isl = Loop { segs: vec![Seg::Arc { a: h, b: h, c: p2(20.0, 5.0), ccw: false }] };
    let rg = Region { loops: vec![r, isl] };
    for rr in [1.0, 1.6, 3.0] {
        let t = std::time::Instant::now();
        let (out, st) = offset(&rg, rr).unwrap();
        println!("{rr}: {} loops {:?} {:?}", out.len(), st, t.elapsed());
    }
}

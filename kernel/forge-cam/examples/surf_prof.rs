#[path = "surf_dome.rs"]
#[allow(dead_code)]
mod dome;
use forge_cam::surf::Surfacer;
use forge_tooling::Library;
fn main() {
    let faces = dome::dome();
    let lib = Library::builtin();
    for id in ["ball-6", "bull-10-r1", "flat-10"] {
        let t = lib.tool(id).unwrap().clone();
        let sh = t.shape(t.lcf + 15.0, None).unwrap();
        let sf = Surfacer::new(&faces, &sh.bodies[0], 0.0, 0.01, 0.0).unwrap();
        let mut worst = vec![];
        let t0 = std::time::Instant::now();
        let mut cells = 0;
        for iy in 0..20 { for ix in 0..20 { let (x, y) = (-3.0 + ix as f64 * 3.3, -3.0 + iy as f64 * 3.3);
            let tt = std::time::Instant::now(); let c = sf.cl(x, y); if tt.elapsed().as_millis() > 20 { println!("  {id} slow {x} {y} {:?} {c:?}", tt.elapsed()); } cells += c.cells; worst.push((c.cells, x, y)); } }
        worst.sort_by(|a, b| b.0.cmp(&a.0));
        println!("{id}: 3600 drops {:?} cells {cells} worst {:?}", t0.elapsed(), &worst[..4]);
    }
}

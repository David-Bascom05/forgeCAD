//! Opening a model as the app opens a machine's model (`forge_render::machine_model::read`): the
//! file read, then each solid's display mesh. Prints the wall time of each, for the target of a
//! big model open in a second (research/import-speed). `stepopen FILE [REPEAT]`.
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("a STEP file");
    let repeat: usize = args.next().and_then(|r| r.parse().ok()).unwrap_or(1);
    for _ in 0..repeat {
        let t0 = Instant::now();
        let text = std::fs::read(&path).expect("read");
        let t1 = Instant::now();
        let imp = forge_step::import(&text, &forge_step::Limits::default()).expect("a model");
        let t2 = Instant::now();
        let (mut tris, mut kept, mut failed) = (0, 0, 0);
        for s in &imp.solids {
            kept += s.mesh.is_some() as usize;
            match s.display_mesh() {
                Ok(m) => tris += m.triangles.len(),
                Err(_) => failed += 1,
            }
        }
        let t3 = Instant::now();
        let ms = |a: Instant, b: Instant| (b - a).as_secs_f64() * 1e3;
        println!(
            "{path}: file {:.0} ms, import {:.0} ms, display meshes {:.0} ms ({kept}/{} kept from the read), total {:.0} ms; {} solids, {tris} triangles, {failed} not drawn",
            ms(t0, t1),
            ms(t1, t2),
            ms(t2, t3),
            imp.solids.len(),
            ms(t0, t3),
            imp.solids.len()
        );
    }
}

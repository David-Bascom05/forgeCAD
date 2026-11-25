//! Opening a model as the app opens a machine's model (`forge_render::machine_model`): the file
//! read as far as its solids can be shown (`forge_step::open`, every solid's display mesh), then
//! the rest of the read (`Opened::finish`: the checks and measures, the same `Imported` as
//! `import_drawn`). Prints the wall time of each, for the target of a big model shown in a second
//! (research/import-speed). `stepopen FILE [REPEAT]`.
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("a STEP file");
    let repeat: usize = args.next().and_then(|r| r.parse().ok()).unwrap_or(1);
    for _ in 0..repeat {
        let t0 = Instant::now();
        let text = std::fs::read(&path).expect("read");
        let t1 = Instant::now();
        let opened = forge_step::open(&text, &forge_step::Limits::default()).expect("a model");
        let shown = opened.shown();
        let (n, tris) = (
            shown.len(),
            shown.iter().map(|(_, m)| m.triangles.len()).sum::<usize>(),
        );
        let t2 = Instant::now();
        let imp = opened.finish().expect("a model");
        let t3 = Instant::now();
        let kept: usize = imp
            .solids
            .iter()
            .map(|s| s.mesh.as_ref().map_or(0, |m| m.triangles.len()))
            .sum();
        let ms = |a: Instant, b: Instant| (b - a).as_secs_f64() * 1e3;
        println!(
            "{path}: file {:.0} ms, shown {:.0} ms ({n} solids, {tris} triangles), checked {:.0} ms more ({} solids, {kept} triangles, {} refused); shown at {:.0} ms, all read at {:.0} ms",
            ms(t0, t1),
            ms(t1, t2),
            ms(t2, t3),
            imp.solids.len(),
            imp.refusals.len(),
            ms(t0, t2),
            ms(t0, t3),
        );
    }
}

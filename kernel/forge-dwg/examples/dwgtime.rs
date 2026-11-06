//! Times reading, scene building and writing a file. `dwgtime FILE`
use std::time::Instant;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let t = Instant::now();
    let buf = std::fs::read(&path).unwrap();
    let t_io = t.elapsed();
    let t = Instant::now();
    let s = forge_dwg::file::sections(&buf).unwrap();
    let t_sec = t.elapsed();
    let t = Instant::now();
    let d = forge_dwg::file::read(&buf).unwrap();
    let t_read = t.elapsed();
    let t = Instant::now();
    let sc = forge_dwg::scene::build(&d, forge_dwg::scene::Space::Model, &forge_dwg::scene::BoxFont, &Default::default());
    let t_scene = t.elapsed();
    let t = Instant::now();
    let out = forge_dwg::file::write(&d).unwrap();
    let t_write = t.elapsed();
    println!("file read {t_io:?}; container+decompress {t_sec:?} ({} MB objects); full read {t_read:?} ({} objects); scene {t_scene:?} ({} items); write {t_write:?} ({} MB)",
        s.objects.len() / 1_000_000, d.objects.len(), sc.items.len(), out.len() / 1_000_000);
}

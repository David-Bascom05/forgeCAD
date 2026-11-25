//! A machine model file opened as the app opens it (`machine_model::file_tris`), timed: when its
//! triangles are first shown, and when its read's checks finish and the final ones replace them.
//! The first time (or with `FORGE_RENDER_CACHE` empty) the file is read; again, it comes from the
//! cache. `model_open FILE`.
fn main() {
    let path = std::env::args().nth(1).expect("a model file");
    let t0 = std::time::Instant::now();
    let shown = forge_render::machine_model::file_tris(std::path::Path::new(&path)).expect("a model");
    let t1 = t0.elapsed().as_secs_f64() * 1e3;
    while forge_render::machine_model::pending() {
        forge_render::machine_model::poll();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let t2 = t0.elapsed().as_secs_f64() * 1e3;
    let done = forge_render::machine_model::file_tris(std::path::Path::new(&path)).expect("a model");
    println!(
        "{path}: shown at {t1:.0} ms ({} triangles), final at {t2:.0} ms ({} triangles)",
        shown.tri.len(),
        done.tri.len()
    );
}

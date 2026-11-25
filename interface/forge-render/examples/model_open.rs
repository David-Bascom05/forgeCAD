//! A machine model file opened as the app opens it (`machine_model::read`), timed: the first time
//! (or `FORGE_RENDER_CACHE` empty) reads the file, again it comes from the cache.
//! `model_open FILE`.
fn main() {
    let path = std::env::args().nth(1).expect("a model file");
    let t0 = std::time::Instant::now();
    let bytes = std::fs::read(&path).expect("read");
    let name = std::path::Path::new(&path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let t = forge_render::machine_model::read(&name, &bytes).expect("a model");
    println!(
        "{path}: {} triangles in {:.0} ms",
        t.tri.len(),
        t0.elapsed().as_secs_f64() * 1e3
    );
}

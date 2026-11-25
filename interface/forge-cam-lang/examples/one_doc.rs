//! One machining document's reply (its certification, warnings and the fixture's pieces).
fn main() {
    let p = std::path::PathBuf::from(std::env::args().nth(1).expect("DOC"));
    let dir = std::env::args()
        .nth(2)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| p.parent().unwrap().to_path_buf());
    let t0 = std::time::Instant::now();
    let cam = forge_cam_lang::run(&std::fs::read_to_string(&p).expect("read"), &dir);
    let dt = t0.elapsed().as_secs_f64();
    println!("ok {} in {dt:.3}s", cam.ok);
    for e in &cam.errors {
        println!("  error: {e}");
    }
    for w in &cam.warnings {
        println!("  warn: {w}");
    }
    if let Some(f) = cam.setup_view(0).and_then(|s| s.fixture) {
        println!(
            "  fixture {} table_z {:?} jaw_top {} enveloped {:?}",
            f.name, f.table_z, f.jaw_top, f.enveloped
        );
        println!("  {} faces", f.faces().len());
        for (n, lo, hi) in f.boxes() {
            println!(
                "    {n:28} {:?} {:?}",
                lo.map(|x| (x * 100.0).round() / 100.0),
                hi.map(|x| (x * 100.0).round() / 100.0)
            );
        }
    }
    println!(
        "{}",
        cam.programs
            .first()
            .map(|p| format!("program {} chars", p.text.as_deref().unwrap_or("").len()))
            .unwrap_or("no program".into())
    );
}

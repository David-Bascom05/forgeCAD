//! Times `forge_cam_lang::run` on machining documents: wall time per document, and the reply's
//! certificate totals (cells, pairs) per op.
//!
//! cargo run --release -p forge-cam-lang --example cam_time -- DOC.fg [DOC.fg ..]
fn main() {
    let mut total = 0.0;
    for a in std::env::args().skip(1) {
        let p = std::path::Path::new(&a);
        let src = std::fs::read_to_string(p).expect("read");
        let dir = p.parent().unwrap_or(std::path::Path::new("."));
        let t0 = std::time::Instant::now();
        let cam = forge_cam_lang::run(&src, dir);
        let dt = t0.elapsed().as_secs_f64();
        total += dt;
        if let Ok(d) = std::env::var("CAM_TIME_JSON") {
            let name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let mut text = cam.json().to_string();
            for pr in &cam.programs {
                text.push_str(&format!("\n{:?}", pr.text));
            }
            std::fs::write(std::path::Path::new(&d).join(format!("{name}.json")), text).expect("write");
        }
        let mut cells = 0usize;
        let mut ops = Vec::new();
        for op in &cam.ops {
            cells += op.cells;
            ops.push(format!(
                "  {:<14} {:<10} moves {:>6} pairs {:>8} cells {:>9} {}",
                op.name, op.kind, op.moves, op.pairs, op.cells, op.status
            ));
        }
        println!("{a}: {dt:.3} s  ok {}  cells {cells}", cam.ok);
        if std::env::var("CAM_TIME_OPS").is_ok() {
            for o in ops {
                println!("{o}");
            }
        }
    }
    println!("total {total:.3} s");
}

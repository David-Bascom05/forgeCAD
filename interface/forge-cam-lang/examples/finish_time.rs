//! Times each op of a machining document alone (after its first op, the roughing):
//! cargo run --release -p forge-cam-lang --example finish_time -- examples/machined/X_cam.fg
fn main() {
    let path = std::path::PathBuf::from(std::env::args().nth(1).expect("doc.fg"));
    let dir = path.parent().unwrap().to_path_buf();
    let src = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = src.lines().collect();
    let ops: Vec<usize> = (0..lines.len())
        .filter(|&i| {
            let l = lines[i];
            l.contains("(op1") && !l.starts_with('#') && !l.starts_with("prog") && !l.starts_with("rough")
        })
        .collect();
    for &k in &ops {
        let doc: String = (0..lines.len())
            .filter(|&i| !ops.contains(&i) || i == k)
            .map(|i| format!("{}\n", lines[i]))
            .collect();
        let t0 = std::time::Instant::now();
        let cam = forge_cam_lang::run(&doc, &dir);
        let op = cam.ops.iter().find(|o| lines[k].starts_with(o.name.as_str()));
        println!(
            "{:60} {:6.1} s  ok {} moves {:?} cells {:?}",
            &lines[k][..lines[k].len().min(60)],
            t0.elapsed().as_secs_f64(),
            cam.ok,
            op.map(|o| o.moves),
            op.map(|o| o.cells)
        );
    }
}

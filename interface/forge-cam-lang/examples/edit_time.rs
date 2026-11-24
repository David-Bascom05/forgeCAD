//! Edit latency: a machining document solved cold, again unchanged, and after one-parameter
//! edits, all in one process as the app does (it re-runs the whole text at every edit).
//!
//! cargo run --release -p forge-cam-lang --example edit_time -- DOC.fg [FILE:OLD=>NEW ..]
//!
//! The document's directory is copied to a scratch directory; each edit replaces the first
//! OLD in FILE (a file of that directory, e.g. the document or its part) with NEW, is run, and
//! is undone before the next. Each line: wall seconds, the reply's ok, its cells.
fn run(dir: &std::path::Path, doc: &str) -> (f64, bool, usize, String) {
    let src = std::fs::read_to_string(dir.join(doc)).expect("read");
    let t0 = std::time::Instant::now();
    let cam = forge_cam_lang::run(&src, dir);
    let dt = t0.elapsed().as_secs_f64();
    let mut text = cam.json().to_string();
    for pr in &cam.programs {
        text.push_str(&format!("\n{:?}", pr.text));
    }
    (dt, cam.ok, cam.ops.iter().map(|o| o.cells).sum(), text)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let docp = std::path::PathBuf::from(args.next().expect("DOC.fg"));
    let src_dir = docp.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
    let doc = docp.file_name().expect("doc name").to_string_lossy().into_owned();
    let work = std::env::temp_dir().join(format!("edit_time_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("scratch");
    for e in std::fs::read_dir(&src_dir).expect("dir").flatten() {
        if e.path().is_file() {
            std::fs::copy(e.path(), work.join(e.file_name())).expect("copy");
        }
    }
    let show = |what: &str, r: &(f64, bool, usize, String)| {
        println!("{what:<40} {:>8.3} s  ok {}  cells {}", r.0, r.1, r.2)
    };
    let cold = run(&work, &doc);
    show("cold", &cold);
    let again = run(&work, &doc);
    show("rerun", &again);
    // a rerun's reply and posted text are the cold run's, byte for byte
    println!(
        "rerun's reply and NC the cold run's: {}",
        if again.3 == cold.3 { "yes" } else { "NO" }
    );
    for e in args {
        let (file, rest) = e.split_once(':').expect("FILE:OLD=>NEW");
        let (old, new) = rest.split_once("=>").expect("OLD=>NEW");
        let path = work.join(file);
        let orig = std::fs::read_to_string(&path).expect("edit file");
        assert!(orig.contains(old), "{file} has no `{old}`");
        std::fs::write(&path, orig.replacen(old, &new.replace("\\n", "\n"), 1)).expect("write");
        show(&format!("{file}: {old} => {new}"), &run(&work, &doc));
        std::fs::write(&path, orig).expect("undo");
        // back to the original text: the cold run's reply again
        let back = run(&work, &doc);
        if back.3 != cold.3 {
            println!("  undone: the reply is NOT the cold run's");
        }
    }
    let _ = std::fs::remove_dir_all(&work);
}

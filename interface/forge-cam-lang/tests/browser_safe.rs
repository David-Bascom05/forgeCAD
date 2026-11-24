//! The CAM crates run in a browser page (wasm32-unknown-unknown), where std's clock panics and
//! std's file system is not there: they use forge_math::clock and forge_dirs::fs. A guard on the
//! source text, since clippy's disallowed-methods is workspace-wide and the CLI may use both.
use std::path::Path;

fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in forge_dirs::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn the_cam_crates_use_no_std_clock_or_file_system() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = vec![];
    for c in [
        "interface/forge-cam-lang/src",
        "kernel/forge-cam/src",
        "kernel/forge-cam-geom/src",
        "kernel/forge-cam-3d/src",
        "kernel/forge-post/src",
    ] {
        sources(&root.join(c), &mut files);
    }
    assert!(!files.is_empty());
    let banned = ["std::fs", "std::time", "Instant::now", "SystemTime"];
    let mut found = vec![];
    for f in &files {
        let s = forge_dirs::fs::read_to_string(f).unwrap();
        for (i, line) in s.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            for b in banned {
                // the kernel's clock is the allowed one
                if code.contains(b) && !code.contains("forge_math::clock::Instant::now") {
                    found.push(format!("{}:{}: {}", f.display(), i + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "std clock or file system in a CAM crate:\n{}",
        found.join("\n")
    );
}

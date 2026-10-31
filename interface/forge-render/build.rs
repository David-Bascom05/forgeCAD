// The build cache's "kernel version": a hash of the sources that decide a part's mesh, ours too.
use std::path::Path;

fn walk(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, files);
        } else if p.extension().is_some_and(|x| x == "rs") {
            files.push(p);
        }
    }
}

fn main() {
    let mut files = Vec::new();
    for d in ["../../kernel", "../forge-agent/src", "../forge-doc/src"] {
        println!("cargo:rerun-if-changed={d}");
        walk(Path::new(d), &mut files);
    }
    for f in ["src/tess.rs", "src/mesh.rs"] {
        println!("cargo:rerun-if-changed={f}");
        files.push(f.into());
    }
    files.sort();
    let mut h = 0xcbf29ce484222325u64;
    for f in files {
        for b in std::fs::read(f).unwrap_or_default() {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
    }
    println!("cargo:rustc-env=FORGE_KERNEL_HASH={h:016x}");
    println!("cargo:rerun-if-changed=build.rs");
}

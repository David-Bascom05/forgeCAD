// The result caches' kernel version: a hash of every source that decides what a document builds
// to, this crate's too. Any change to them and nothing cached before is read.
use std::path::Path;

fn walk(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, files);
        } else if p
            .extension()
            .is_some_and(|x| x == "rs" || x == "md" || x == "toml")
        {
            files.push(p);
        }
    }
}

fn main() {
    let mut files = Vec::new();
    for d in ["../../kernel", "../forge-agent/src", "../forge-doc/src", "src"] {
        println!("cargo:rerun-if-changed={d}");
        walk(Path::new(d), &mut files);
    }
    files.sort();
    let mut h = 0xcbf29ce484222325u64;
    for f in files {
        for b in f
            .to_string_lossy()
            .bytes()
            .chain(std::fs::read(&f).unwrap_or_default())
        {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
    }
    println!("cargo:rustc-env=FORGE_KERNEL_HASH={h:016x}");
    println!("cargo:rerun-if-changed=build.rs");
}

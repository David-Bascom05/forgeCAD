// The result caches' kernel version: a hash of every source that decides what a document builds
// to. Those are the crates forge-lang links, listed in hash-inputs.txt (checked against `cargo
// metadata` by tests/hash_inputs.rs), with every file of each but its tests, benches and
// examples, and the toolchain, flags and profile. Any change to them and nothing cached before is
// read; a change elsewhere (a crate forge-lang does not link) leaves the key as it was.
use std::path::{Path, PathBuf};

/// Directories of a crate that cannot change what its library computes.
const SKIP: [&str; 4] = ["tests", "benches", "examples", "target"];

fn walk(dir: &Path, top: bool, files: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name();
        let name = name.to_string_lossy();
        if p.is_dir() {
            if name.starts_with('.') || (top && SKIP.contains(&name.as_ref())) {
                continue;
            }
            walk(&p, false, files);
        } else {
            files.push(p);
        }
    }
}

fn main() {
    let root = Path::new("../..");
    let list = "hash-inputs.txt";
    println!("cargo:rerun-if-changed={list}");
    let text = std::fs::read_to_string(list).expect("interface/forge-lang/hash-inputs.txt");
    let mut files: Vec<PathBuf> = Vec::new();
    let mut deps: Vec<&str> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line.split_once(' ') {
            Some(("path", dir)) => {
                let d = root.join(dir);
                assert!(d.is_dir(), "hash-inputs.txt: {dir} is not a directory");
                println!("cargo:rerun-if-changed={}", d.display());
                walk(&d, true, &mut files);
            }
            Some(("dep", _)) => deps.push(line),
            _ => panic!("hash-inputs.txt: {line:?} is neither `path DIR` nor `dep NAME VERSION`"),
        }
    }
    // and what changes the numbers a build computes: the toolchain, the flags, the profile
    for f in [
        "../../rust-toolchain.toml",
        "../../Cargo.toml",
        "../../.cargo/config.toml",
        "../../.cargo/config",
    ] {
        // only a file that exists: cargo counts a missing one as changed, every build
        if Path::new(f).exists() {
            println!("cargo:rerun-if-changed={f}");
            files.push(f.into());
        }
    }
    files.sort();
    let mut h = 0xcbf29ce484222325u64;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
    };
    for k in [
        "PROFILE",
        "OPT_LEVEL",
        "DEBUG",
        "CARGO_ENCODED_RUSTFLAGS",
        "TARGET",
        "RUSTC",
    ] {
        println!("cargo:rerun-if-env-changed={k}");
        eat(k.as_bytes());
        eat(std::env::var(k).unwrap_or_default().as_bytes());
    }
    let debug_assertions = std::env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some();
    eat(&[debug_assertions as u8]);
    for d in deps {
        eat(d.as_bytes());
    }
    for f in files {
        eat(f.to_string_lossy().as_bytes());
        eat(&std::fs::read(&f).unwrap_or_default());
    }
    println!("cargo:rustc-env=FORGE_KERNEL_HASH={h:016x}");
    println!("cargo:rerun-if-changed=build.rs");
}

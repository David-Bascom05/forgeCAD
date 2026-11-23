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
    println!("cargo:rerun-if-changed=build.rs");
    // a measurement build pins it (tools/speed/gate.sh, FORGE_KERNEL_HASH_PIN): the hash is in
    // every cache key, so a kernel change that compiles to the same code (a comment, a module
    // behind a feature that is off) would otherwise move the keys and with them the count
    println!("cargo:rerun-if-env-changed=FORGE_KERNEL_HASH_PIN");
    if let Ok(pin) = std::env::var("FORGE_KERNEL_HASH_PIN") {
        println!("cargo:rustc-env=FORGE_KERNEL_HASH={pin}");
        return;
    }
    let mut files = Vec::new();
    for d in ["../../kernel", "../forge-agent/src", "src"] {
        println!("cargo:rerun-if-changed={d}");
        walk(Path::new(d), &mut files);
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
    for k in [
        "PROFILE",
        "OPT_LEVEL",
        "DEBUG",
        "CARGO_ENCODED_RUSTFLAGS",
        "TARGET",
        "RUSTC",
    ] {
        println!("cargo:rerun-if-env-changed={k}");
        for b in k.bytes().chain(std::env::var(k).unwrap_or_default().into_bytes()) {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
    }
    let debug_assertions = std::env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some();
    h = (h ^ debug_assertions as u64).wrapping_mul(0x100000001b3);
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
}

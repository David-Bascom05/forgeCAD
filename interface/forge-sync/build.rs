//! FORGE_KERNEL_ID: the pinned toolchain and the git tree hashes of the code that decides
//! geometry (kernel/, interface/forge-lang). "unknown" outside a git checkout (then every such
//! build counts as one kernel: a CI build always has its checkout).
use std::process::Command;

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(&root)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let tree = |p: &str| git(&["rev-parse", &format!("HEAD:{p}")]).unwrap_or_else(|| "unknown".into());
    // the working tree's own changes count too
    let dirty = git(&["status", "--porcelain", "--", "kernel", "interface/forge-lang"])
        .is_some_and(|s| !s.is_empty());
    let toolchain = std::fs::read_to_string(root.join("rust-toolchain.toml"))
        .ok()
        .and_then(|t| {
            t.lines()
                .find(|l| l.trim_start().starts_with("channel"))
                .map(|l| l.split('"').nth(1).unwrap_or("").to_string())
        })
        .unwrap_or_else(|| "unknown".into());
    let id = format!(
        "rust-{toolchain}/kernel-{}/lang-{}{}",
        &tree("kernel")[..12.min(tree("kernel").len())],
        &tree("interface/forge-lang")[..12.min(tree("interface/forge-lang").len())],
        if dirty { "+dirty" } else { "" }
    );
    println!("cargo:rustc-env=FORGE_KERNEL_ID={id}");
    // the id follows the code: any change under kernel/ or forge-lang (a directory is watched
    // whole), the toolchain, and the checkout's HEAD
    for p in ["../../rust-toolchain.toml", "../../kernel", "../forge-lang", "../../.git/HEAD"] {
        println!("cargo:rerun-if-changed={p}");
    }
}

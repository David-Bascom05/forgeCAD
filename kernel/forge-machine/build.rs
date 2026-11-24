//! Embeds every built-in machine file, data/machines/<vendor>.toml, so each vendor's library is
//! its own file (no shared list to edit): generic.toml first, then the rest by name.
use std::fmt::Write;

fn main() {
    let dir = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("data/machines");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".toml"))
        .collect();
    names.sort_by_key(|n| (n != "generic.toml", n.clone()));
    let mut out = String::from("pub const BUILTIN: &[(&str, &str)] = &[\n");
    for n in &names {
        println!("cargo:rerun-if-changed={}", dir.join(n).display());
        writeln!(
            out,
            "    ({n:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/data/machines/{n}\"))),"
        )
        .unwrap();
    }
    out.push_str("];\n");
    let dest = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("builtin.rs");
    std::fs::write(dest, out).unwrap();
}

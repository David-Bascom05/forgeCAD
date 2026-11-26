//! Embeds every compiled workholding model, data/check/NAME.txt, by its file name, so a model is
//! added by compiling it (forge-workholding-builder's wh_compile) and no code names it.
use std::fmt::Write;

fn main() {
    let dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("data/check");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".txt"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    let mut out = String::from("pub(crate) const CHECKS: &[(&str, &str)] = &[\n");
    for n in &names {
        println!("cargo:rerun-if-changed={}", dir.join(n).display());
        writeln!(
            out,
            "    ({:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/data/check/{n}\"))),",
            n.trim_end_matches(".txt")
        )
        .unwrap();
    }
    out.push_str("];\n");
    let dst = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("checks.rs");
    std::fs::write(dst, out).unwrap();
}

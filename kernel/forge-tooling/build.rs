//! Embeds the built-in library: the generic files first (their order is the library's order),
//! then every vendor file under data/{tools,holders,collets,turning}/ by directory and name, so a
//! vendor's library is its own file and no list is edited to add one.
use std::fmt::Write;

fn main() {
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("data");
    let mut files: Vec<String> = [
        "fixtures.toml",
        "tools.toml",
        "holders.toml",
        "machines.toml",
        "cutting.toml",
        "generic.toml",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for dir in ["tools", "holders", "collets", "turning"] {
        let d = root.join(dir);
        println!("cargo:rerun-if-changed={}", d.display());
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut names: Vec<String> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".toml"))
            .collect();
        names.sort();
        files.extend(names.into_iter().map(|n| format!("{dir}/{n}")));
    }
    let mut out = String::from("pub const BUILTIN: &[(&str, &str)] = &[\n");
    for f in &files {
        println!("cargo:rerun-if-changed={}", root.join(f).display());
        writeln!(
            out,
            "    (\"builtin/{f}\", include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/data/{f}\"))),"
        )
        .unwrap();
    }
    out.push_str("];\n");
    // maker-published libraries in bulk (data/libraries/, research/cam/tooling/libraries.md):
    // embedded only with the `maker-libraries` feature, so the default library stays small
    out.push_str("pub const MAKER_LIBRARIES: &[(&str, &str)] = &[\n");
    let libs = root.join("libraries");
    println!("cargo:rerun-if-changed={}", libs.display());
    if std::env::var_os("CARGO_FEATURE_MAKER_LIBRARIES").is_some()
        && let Ok(rd) = std::fs::read_dir(&libs)
    {
        let mut names: Vec<String> = rd
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".toml"))
            .collect();
        names.sort();
        for n in names {
            println!("cargo:rerun-if-changed={}", libs.join(&n).display());
            writeln!(
                out,
                "    (\"libraries/{n}\", include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/data/libraries/{n}\"))),"
            )
            .unwrap();
        }
    }
    out.push_str("];\n");
    std::fs::write(
        std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("builtin.rs"),
        out,
    )
    .unwrap();
}

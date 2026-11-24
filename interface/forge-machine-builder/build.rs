//! Embeds the built-in machines' body parts (kernel/forge-machine/parts/**.fg) so a clone of a
//! built-in machine can carry its real parts to the user's or project's folder.
use std::fmt::Write;
use std::path::Path;

fn walk(dir: &Path, rel: &str, out: &mut Vec<(String, String)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut es: Vec<_> = rd.filter_map(|e| e.ok()).collect();
    es.sort_by_key(|e| e.file_name());
    for e in es {
        let n = e.file_name().to_string_lossy().into_owned();
        let p = e.path();
        let r = if rel.is_empty() {
            n.clone()
        } else {
            format!("{rel}/{n}")
        };
        if p.is_dir() {
            walk(&p, &r, out);
        } else if n.ends_with(".fg") {
            out.push((format!("parts/{r}"), p.display().to_string()));
        }
    }
}

fn main() {
    let root =
        Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../kernel/forge-machine/parts");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    walk(&root, "", &mut files);
    let mut out = String::from("pub const PARTS: &[(&str, &str)] = &[\n");
    for (rel, abs) in &files {
        println!("cargo:rerun-if-changed={abs}");
        writeln!(out, "    ({rel:?}, include_str!({abs:?})),").unwrap();
    }
    out.push_str("];\n");
    std::fs::write(
        Path::new(&std::env::var("OUT_DIR").unwrap()).join("parts.rs"),
        out,
    )
    .unwrap();
}

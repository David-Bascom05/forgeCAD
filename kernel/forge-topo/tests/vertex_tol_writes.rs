//! A vertex's `tol` is raised only through `Body::add_tolerant_vertex` and `Body::grow_vertex_tol`,
//! which keep `Body::vertex_tol` (the bound point_in_face looks past, CacheOK G9) above it: no
//! source in the repository writes it directly (forge-topo's own body.rs aside).
use std::path::{Path, PathBuf};

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if name != "target" && !name.starts_with('.') {
                sources(&p, out);
            }
        } else if name.ends_with(".rs") {
            out.push(p);
        }
    }
}

/// Whether `l` (whitespace removed) assigns to `x.tol`.
fn writes_tol(l: &str, x: &str) -> bool {
    [".tol=", ".tol+=", ".tol*="]
        .iter()
        .any(|op| l.contains(&format!("{x}{op}")))
}

#[test]
fn no_source_writes_a_vertex_tolerance() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for d in ["kernel", "interface", "verify"] {
        sources(&root.join(d), &mut files);
    }
    assert!(files.len() > 100, "{} sources", files.len());
    let own = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/body.rs")
        .canonicalize()
        .unwrap();
    let mut bad = Vec::new();
    for f in &files {
        if f.canonicalize().ok().as_deref() == Some(own.as_path()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        let lines: Vec<String> = text
            .lines()
            .map(|line| line.chars().filter(|c| !c.is_whitespace()).collect())
            .collect();
        // names bound to a vertex held mutably: `let x = &mut ….vertices[…];`, and a loop's
        // `for x in &mut ….vertices` or `for x in ….vertices.iter_mut()`
        let looped: Vec<String> = lines
            .iter()
            .filter_map(|l| {
                let rest = l.strip_prefix("for")?;
                let (x, over) = rest.split_once("in")?;
                ((over.starts_with("&mut") && over.contains("vertices"))
                    || over.contains("vertices.iter_mut()"))
                .then(|| x.trim_start_matches("mut").to_string())
            })
            .collect();
        let held: Vec<String> = lines
            .iter()
            .filter(|l| l.starts_with("letmut") || l.starts_with("let"))
            .filter_map(|l| {
                let (lhs, rhs) = l.split_once('=')?;
                (rhs.starts_with("&mut") && rhs.contains("vertices[")).then(|| {
                    lhs.trim_start_matches("let")
                        .trim_start_matches("mut")
                        .to_string()
                })
            })
            .collect();
        let held: Vec<String> = held.into_iter().chain(looped).collect();
        for (n, l) in lines.iter().enumerate() {
            if l.starts_with("//") {
                continue;
            }
            // `….vertices[…].tol = …`
            let direct = l.split("vertices[").skip(1).any(|r| {
                r.split_once(']')
                    .is_some_and(|(_, after)| writes_tol(&format!("§{after}"), "§"))
            });
            // (a write its writer bounds, marked so on the line: `// vertex_tol: bounded below`)
            let through = held.iter().any(|x| writes_tol(l, x)) && !l.contains("//vertex_tol:bounded");
            if direct || through {
                bad.push(format!(
                    "{}:{}: {}",
                    f.display(),
                    n + 1,
                    text.lines().nth(n).unwrap_or("").trim()
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "raise a vertex's tol with grow_vertex_tol:\n{}",
        bad.join("\n")
    );
}

//! hash-inputs.txt, what FORGE_KERNEL_HASH (the result caches' key, build.rs) hashes, is exactly
//! forge-lang's build closure as `cargo metadata` resolves it: every package forge-lang links
//! (normal and build dependencies, any target; dev-dependencies are not linked into it). A crate
//! missing from the list would let a cache outlive a change that moves its results; one too many
//! only makes the key change when it need not. And every file a listed crate's code includes
//! (include_str!, include_bytes!) lies where build.rs hashes it: inside the crate, outside its
//! tests/, benches/ and examples/.
#![cfg(not(target_family = "wasm"))]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The list's `path` and `dep` lines.
fn listed() -> (BTreeSet<String>, BTreeSet<String>) {
    let text =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("hash-inputs.txt")).unwrap();
    let (mut paths, mut deps) = (BTreeSet::new(), BTreeSet::new());
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line.split_once(' ') {
            Some(("path", d)) => assert!(paths.insert(d.to_string()), "{d} listed twice"),
            Some(("dep", d)) => assert!(deps.insert(d.to_string()), "{d} listed twice"),
            _ => panic!("bad line {line:?}"),
        }
    }
    (paths, deps)
}

/// A minimal JSON reader for `cargo metadata`'s output: enough for objects, arrays, strings,
/// numbers, booleans and null (no crate is pulled in for it).
enum J {
    Null,
    Bool,
    Num,
    Str(String),
    Arr(Vec<J>),
    Obj(BTreeMap<String, J>),
}

impl J {
    fn get(&self, k: &str) -> &J {
        match self {
            J::Obj(m) => m.get(k).unwrap_or(&J::Null),
            _ => &J::Null,
        }
    }
    fn arr(&self) -> &[J] {
        match self {
            J::Arr(v) => v,
            _ => &[],
        }
    }
    fn str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }
}

fn parse(s: &[u8], i: &mut usize) -> J {
    let ws = |i: &mut usize| {
        while *i < s.len() && s[*i].is_ascii_whitespace() {
            *i += 1;
        }
    };
    ws(i);
    match s[*i] {
        b'{' => {
            *i += 1;
            let mut m = BTreeMap::new();
            loop {
                ws(i);
                if s[*i] == b'}' {
                    *i += 1;
                    break;
                }
                let J::Str(k) = parse(s, i) else { panic!("key") };
                ws(i);
                assert_eq!(s[*i], b':');
                *i += 1;
                m.insert(k, parse(s, i));
                ws(i);
                if s[*i] == b',' {
                    *i += 1;
                }
            }
            J::Obj(m)
        }
        b'[' => {
            *i += 1;
            let mut v = Vec::new();
            loop {
                ws(i);
                if s[*i] == b']' {
                    *i += 1;
                    break;
                }
                v.push(parse(s, i));
                ws(i);
                if s[*i] == b',' {
                    *i += 1;
                }
            }
            J::Arr(v)
        }
        b'"' => {
            *i += 1;
            let mut out = Vec::new();
            while s[*i] != b'"' {
                if s[*i] == b'\\' {
                    *i += 1;
                    match s[*i] {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'u' => {
                            let h = std::str::from_utf8(&s[*i + 1..*i + 5]).unwrap();
                            let c = char::from_u32(u32::from_str_radix(h, 16).unwrap()).unwrap_or('?');
                            out.extend(c.to_string().bytes());
                            *i += 4;
                        }
                        c => out.push(c),
                    }
                } else {
                    out.push(s[*i]);
                }
                *i += 1;
            }
            *i += 1;
            J::Str(String::from_utf8(out).unwrap())
        }
        b't' => {
            *i += 4;
            J::Bool
        }
        b'f' => {
            *i += 5;
            J::Bool
        }
        b'n' => {
            *i += 4;
            J::Null
        }
        _ => {
            while *i < s.len() && !matches!(s[*i], b',' | b'}' | b']') && !s[*i].is_ascii_whitespace() {
                *i += 1;
            }
            J::Num
        }
    }
}

/// forge-lang's build closure from `cargo metadata`: workspace crates by directory (from the
/// repository root), others as "NAME VERSION".
fn closure() -> (BTreeSet<String>, BTreeSet<String>) {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--offline"])
        .current_dir(root())
        .output()
        .expect("cargo metadata");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let m = parse(&out.stdout, &mut 0);
    let mut pkg = BTreeMap::new();
    for p in m.get("packages").arr() {
        pkg.insert(p.get("id").str().unwrap().to_string(), p);
    }
    let mut deps: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for n in m.get("resolve").get("nodes").arr() {
        let linked = n
            .get("deps")
            .arr()
            .iter()
            .filter(|d| {
                d.get("dep_kinds")
                    .arr()
                    .iter()
                    .any(|k| matches!(k.get("kind"), J::Null) || k.get("kind").str() == Some("build"))
            })
            .map(|d| d.get("pkg").str().unwrap().to_string())
            .collect();
        deps.insert(n.get("id").str().unwrap().to_string(), linked);
    }
    let start = pkg
        .iter()
        .find(|(_, p)| p.get("name").str() == Some("forge-lang"))
        .map(|(id, _)| id.clone())
        .unwrap();
    let (mut seen, mut stack) = (BTreeSet::new(), vec![start]);
    while let Some(id) = stack.pop() {
        if seen.insert(id.clone()) {
            stack.extend(deps.get(&id).into_iter().flatten().cloned());
        }
    }
    let root = root();
    let (mut paths, mut others) = (BTreeSet::new(), BTreeSet::new());
    for id in seen {
        let p = pkg[&id];
        if matches!(p.get("source"), J::Null) {
            let dir = Path::new(p.get("manifest_path").str().unwrap()).parent().unwrap();
            let rel = dir.canonicalize().unwrap();
            let rel = rel.strip_prefix(&root).unwrap();
            paths.insert(rel.to_string_lossy().replace('\\', "/"));
        } else {
            others.insert(format!(
                "{} {}",
                p.get("name").str().unwrap(),
                p.get("version").str().unwrap()
            ));
        }
    }
    (paths, others)
}

#[test]
fn the_list_is_forge_langs_build_closure() {
    let (paths, deps) = listed();
    let (want_paths, want_deps) = closure();
    let missing: Vec<_> = want_paths.difference(&paths).collect();
    let extra: Vec<_> = paths.difference(&want_paths).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "hash-inputs.txt: add `path` lines for {missing:?}, remove {extra:?}"
    );
    let missing: Vec<_> = want_deps.difference(&deps).collect();
    let extra: Vec<_> = deps.difference(&want_deps).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "hash-inputs.txt: add `dep` lines for {missing:?}, remove {extra:?}"
    );
}

/// Every include_str!/include_bytes! under a listed crate's src/ names a file build.rs hashes.
#[test]
fn every_included_file_is_hashed() {
    let root = root();
    let (paths, _) = listed();
    let mut bad = vec![];
    for dir in &paths {
        let krate = root.join(dir).canonicalize().unwrap();
        let mut stack = vec![krate.join("src"), krate.join("build.rs")];
        while let Some(p) = stack.pop() {
            if p.is_dir() {
                for e in std::fs::read_dir(&p).unwrap().flatten() {
                    stack.push(e.path());
                }
                continue;
            }
            if p.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else {
                continue;
            };
            for mac in ["include_str!(\"", "include_bytes!(\""] {
                for (at, _) in text.match_indices(mac) {
                    let rest = &text[at + mac.len()..];
                    let name = &rest[..rest.find('"').unwrap()];
                    let file = p.parent().unwrap().join(name);
                    let Ok(file) = file.canonicalize() else {
                        bad.push(format!("{}: {name} not found", p.display()));
                        continue;
                    };
                    let inside = file
                        .strip_prefix(&krate)
                        .ok()
                        .and_then(|r| r.components().next())
                        .map(|c| c.as_os_str().to_string_lossy().into_owned());
                    match inside {
                        Some(top)
                            if !["tests", "benches", "examples", "target"].contains(&top.as_str())
                                && !top.starts_with('.') => {}
                        _ => bad.push(format!("{}: {name} is not hashed", p.display())),
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

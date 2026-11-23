//! What the fuzzer found and was fixed, replayed: each case ends in a value or a typed error, in
//! its time. A case is a file in tests/corpus/TARGET/: either the input itself, or a recipe (`.case`)
//! that makes it from a file in the repository, so no dataset file and no large mutant is kept:
//!
//!   # why it failed, and what fixed it
//!   from: path/in/the/repository
//!   replace: text in that file          (as many replace/with pairs as it takes; `\n` is a newline)
//!   with: what the fuzzer put there     (`repeat N:` before the text repeats it N times)
//!   expect: refused | ok | any
//!   max-ms: 20000

use forge_read_fuzz::{Outcome, Target, quiet_panics, run};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Case {
    name: String,
    data: Vec<u8>,
    expect: String,
    max_ms: u128,
}

fn unescape(s: &str) -> String {
    s.replace("\\n", "\n").replace("\\t", "\t")
}

fn text(s: &str) -> String {
    match s.strip_prefix("repeat ").and_then(|r| r.split_once(':')) {
        Some((n, t)) => unescape(t.trim_start()).repeat(n.trim().parse().expect("a count")),
        None => unescape(s),
    }
}

fn recipe(path: &Path) -> Case {
    let src = std::fs::read_to_string(path).expect("a recipe");
    let (mut data, mut expect, mut max_ms) = (None::<Vec<u8>>, "any".to_string(), 20_000u128);
    let mut pending: Option<String> = None;
    for line in src.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let (k, v) = line.split_once(": ").unwrap_or((line.trim_end_matches(':'), ""));
        match k {
            "from" => data = Some(std::fs::read(root().join(v)).unwrap_or_else(|e| panic!("{v}: {e}"))),
            "replace" => pending = Some(text(v)),
            "with" => {
                let old = pending.take().expect("replace before with");
                let d = data.as_mut().expect("from first");
                let s = String::from_utf8_lossy(d).into_owned();
                assert!(
                    s.contains(&old),
                    "{}: the seed no longer holds {old:?}",
                    path.display()
                );
                *d = s.replacen(&old, &text(v), 1).into_bytes();
            }
            "expect" => expect = v.to_string(),
            "max-ms" => max_ms = v.parse().expect("a number"),
            other => panic!("{}: unknown line {other}", path.display()),
        }
    }
    Case {
        name: path.display().to_string(),
        data: data.expect("from"),
        expect,
        max_ms,
    }
}

fn cases(t: Target) -> Vec<Case> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus")
        .join(t.name());
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return vec![];
    };
    let mut ps: Vec<PathBuf> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    ps.sort();
    ps.iter()
        .map(|p| {
            if p.extension().is_some_and(|e| e == "case") {
                recipe(p)
            } else {
                Case {
                    name: p.display().to_string(),
                    data: std::fs::read(p).expect("a case"),
                    expect: "any".into(),
                    max_ms: 20_000,
                }
            }
        })
        .collect()
}

#[test]
fn every_case_found_ends_in_a_value_or_a_typed_error_in_its_time() {
    quiet_panics();
    let mut bad = vec![];
    let mut n = 0;
    for &(_, t) in forge_read_fuzz::TARGETS {
        for c in cases(t) {
            n += 1;
            let t0 = Instant::now();
            let o = run(t, &c.data);
            let ms = t0.elapsed().as_millis();
            let fine = !matches!(
                (&o, c.expect.as_str()),
                (Outcome::Panic(_), _) | (Outcome::Refused(_), "ok") | (Outcome::Ok, "refused")
            );
            if !fine || ms > c.max_ms {
                bad.push(format!(
                    "{} ({}): {o:?} in {ms} ms (expected {}, at most {} ms)",
                    c.name,
                    t.name(),
                    c.expect,
                    c.max_ms
                ));
            }
        }
    }
    // the quiet hook would swallow the assertions' own words
    drop(std::panic::take_hook());
    assert!(n > 0, "no cases found");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// The seeds the CI's short run starts from are there, for every target but SHX (no font in the
/// repository) and building a part (the kernel's own fuzzing).
#[test]
fn every_target_has_seeds_in_the_repository() {
    for &(name, t) in forge_read_fuzz::TARGETS {
        if t == Target::Shx {
            continue;
        }
        let s = forge_read_fuzz::seeds(t, &[root()], 512 << 10);
        assert!(!s.is_empty(), "{name}: no seeds");
    }
}

//! Every place in the kernel that can make an op's output depend on something other than its
//! inputs (a static, a thread-local, an env var, the clock, thread count, pointer identity, hash
//! order, unspecified float precision) is listed in tools/proof/hidden-inputs.txt with what it
//! is: `keyed` (in the op memo's key), `speed` (a cache or counter whose answers are a function
//! of their own key), `no-output` (reviewed: does not reach an output), or `must-fix N` (an item
//! of forgecad-gate/instant-load/op-memo-proof.md). A new one fails here until it is reviewed and
//! listed: the op memo (research/forge-lang/lean/ForgeLang/OpMemo.lean, `memo_exact_iff`) is exact
//! only when each op is a function of its key.
//!
//! `FG_REGEN=1 cargo test -p forge-model --test hidden_inputs` adds the new ones as `unreviewed`
//! (which still fails) and drops the ones gone.
//!
//! Sources compiled only behind a cargo feature that no shipped build turns on (`GATED`) are
//! scanned only when this test is built with that feature: the default run covers the shipped
//! kernel, and `cargo test -p forge-model --test hidden_inputs --features decide_once` covers
//! the decide-once boolean too. That second run may fail for now (its sources are not all
//! reviewed yet), but it is required to pass before `decide_once` is ever turned on by default
//! or in any build that ships: `gated_files_are_gated` keeps the list honest.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const CRATES: [&str; 12] = [
    "forge-par",
    "forge-geom",
    "forge-topo",
    "forge-bool",
    "forge-ops",
    "forge-model",
    "forge-sheet",
    "forge-thermo",
    "forge-math",
    "forge-gpu",
    "forge-mesh",
    "forge-harness",
];

/// What to look for, by name; a line can match several.
fn kinds(line: &str) -> Vec<&'static str> {
    let t = line.trim_start();
    if t.starts_with("//") {
        return vec![];
    }
    let mut out = vec![];
    let has = |p: &str| line.contains(p);
    let word = |p: &str| {
        line.match_indices(p).any(|(i, _)| {
            let b = line.as_bytes();
            let before = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
            let j = i + p.len();
            let after = j >= b.len() || !(b[j].is_ascii_alphanumeric() || b[j] == b'_');
            before && after
        })
    };
    if has("thread_local!") {
        out.push("thread_local");
    }
    let stat = t
        .strip_prefix("pub(crate) ")
        .or_else(|| t.strip_prefix("pub "))
        .unwrap_or(t);
    if stat.starts_with("static ") && !stat.starts_with("static ref") {
        out.push("static");
    }
    if has("OnceLock") || has("LazyLock") || has("OnceCell") {
        out.push("lazy");
    }
    if has("Atomic") && (has("AtomicU") || has("AtomicI") || has("AtomicBool") || has("AtomicPtr")) {
        out.push("atomic");
    }
    if has("env::var") || has("option_env!") || has("env!(") {
        out.push("env");
    }
    if has("Instant::now") || has("SystemTime") {
        out.push("clock");
    }
    if has("HashMap<") || has("HashSet<") || has("RandomState") {
        out.push("hash-order");
    }
    if has("available_parallelism") || has("thread::spawn") || has("thread::scope") || has("par::scope") {
        out.push("threads");
    }
    if has("ptr_eq") || has("as_ptr") {
        out.push("pointer");
    }
    if word("powi") || word("mul_add") || has("is_x86_feature_detected") || has("is_aarch64_feature_detected")
    {
        out.push("float");
    }
    if has("rand::") || word("thread_rng") {
        out.push("random");
    }
    out
}

fn kernel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Kernel sources behind a cargo feature no shipped build enables: (file, feature, the line in
/// its crate's lib.rs that gates it). Scanned only when this test has the feature.
const GATED: [(&str, &str, &str); 1] = [(
    "forge-bool/src/decide_once.rs",
    "decide_once",
    "#[cfg(feature = \"decide_once\")]\npub mod decide_once;",
)];

/// Whether a source file is scanned in this build of the test.
fn scanned(rel: &str) -> bool {
    GATED
        .iter()
        .all(|(f, feature, _)| *f != rel || (*feature == "decide_once" && cfg!(feature = "decide_once")))
}

/// Each gated file is still compiled only behind its feature (its lib.rs says so), and that
/// feature is in no default feature set of its crate.
#[test]
fn gated_files_are_gated() {
    for (f, feature, gate) in GATED {
        let krate = f.split('/').next().expect("a crate");
        let lib = std::fs::read_to_string(kernel().join(krate).join("src/lib.rs")).expect("lib.rs");
        assert!(
            lib.contains(gate),
            "{f}: {krate}/src/lib.rs no longer gates it with {gate:?}"
        );
        let toml = std::fs::read_to_string(kernel().join(krate).join("Cargo.toml")).expect("Cargo.toml");
        let default = toml
            .lines()
            .find(|l| l.trim_start().starts_with("default"))
            .unwrap_or("");
        assert!(
            !default.contains(feature),
            "{krate}: `{feature}` is a default feature: {f} ships"
        );
    }
}

/// `crate/path.rs|kind|trimmed line` -> how many such lines.
fn scan() -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for c in CRATES {
        let mut files = vec![];
        rs_files(&kernel().join(c).join("src"), &mut files);
        files.sort();
        for f in files {
            let rel = f
                .strip_prefix(kernel())
                .expect("in the kernel")
                .to_string_lossy()
                .replace('\\', "/");
            if !scanned(&rel) {
                continue;
            }
            let text = std::fs::read_to_string(&f).expect("read");
            scan_text(&rel, &text, &mut out);
        }
    }
    out
}

/// One file's entries: every line but those of `#[cfg(test)]` items.
fn scan_text(rel: &str, text: &str, out: &mut BTreeMap<String, usize>) {
    for line in live_lines(text) {
        for k in kinds(line) {
            *out.entry(format!("{rel}|{k}|{}", line.trim())).or_default() += 1;
        }
    }
}

/// The lines of `text` outside `#[cfg(test)]` items. Each such item (a fn, a mod, a use, ...) is
/// skipped from its attribute to its matching closing brace (or its `;`, for one without a body),
/// and the scan goes on after it: a test item mid-file hides only itself. Braces inside strings,
/// raw strings, char literals and comments are not counted.
fn live_lines(text: &str) -> Vec<&str> {
    let mut out = vec![];
    let mut starts = vec![];
    let mut at = 0;
    for l in text.split_inclusive('\n') {
        starts.push(at);
        at += l.len();
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim_start();
        if let Some(rest) = t.strip_prefix("#[cfg(test)]") {
            let from = starts[i] + (lines[i].len() - rest.len());
            let end = item_end(text, from);
            // resume on the line after the one the item ends on
            i = starts.partition_point(|&s| s <= end);
            continue;
        }
        out.push(lines[i]);
        i += 1;
    }
    out
}

/// The byte offset of the end of the item starting at or after `from`: its body's closing brace,
/// or a `;` before any brace. The end of the text if neither comes.
fn item_end(text: &str, from: usize) -> usize {
    let b = text.as_bytes();
    let mut i = from;
    let mut depth = 0usize;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let mut nest = 1;
                i += 2;
                while i < b.len() && nest > 0 {
                    if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                        nest += 1;
                        i += 1;
                    } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                        nest -= 1;
                        i += 1;
                    }
                    i += 1;
                }
                continue;
            }
            b'r' if (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
                && matches!(b.get(i + 1), Some(b'"') | Some(b'#')) =>
            {
                // a raw string r"..", r#".."#: its end is `"` and as many `#`s
                let mut j = i + 1;
                let mut hashes = 0;
                while b.get(j) == Some(&b'#') {
                    hashes += 1;
                    j += 1;
                }
                if b.get(j) == Some(&b'"') {
                    j += 1;
                    loop {
                        if j >= b.len() {
                            return b.len();
                        }
                        if b[j] == b'"'
                            && b[j + 1..].iter().take(hashes).filter(|&&c| c == b'#').count() == hashes
                        {
                            i = j + 1 + hashes;
                            break;
                        }
                        j += 1;
                    }
                    continue;
                }
            }
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'\'' => {
                // a char literal ('x', '\n', '\u{7b}', '{'); otherwise a lifetime
                if b.get(i + 1) == Some(&b'\\') {
                    i += 2;
                    while i < b.len() && b[i] != b'\'' {
                        i += 1;
                    }
                } else if let Some(c) = text[i + 1..].chars().next() {
                    let n = c.len_utf8();
                    if b.get(i + 1 + n) == Some(&b'\'') {
                        i += 1 + n;
                    }
                }
            }
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return i;
                }
            }
            b';' if depth == 0 => return i,
            _ => {}
        }
        i += 1;
    }
    b.len()
}

/// A `#[cfg(test)]` item mid-file hides only itself: an env read after it is found (and, not
/// being listed, would fail `every_hidden_input_is_reviewed`), and nothing inside it is.
#[test]
fn a_test_item_mid_file_hides_only_itself() {
    let text = r##"pub fn a() -> f64 { 1.0 }

#[cfg(test)]
fn helper() -> usize {
    let s = "}"; let c = '{'; let r = r#"{{"#; // }
    /* { */ std::env::var("INSIDE").map_or(0, |v| v.len())
}

#[cfg(test)]
mod tests;

pub fn b() -> bool {
    std::env::var("AFTER").is_ok()
}

#[cfg(test)]
mod more {
    #[test]
    fn t() { let _ = std::env::var("IN_MOD"); }
}

pub fn c<'a>(x: &'a str) -> &'a str { let _ = std::env::var("LAST"); x }
"##;
    let mut found = BTreeMap::new();
    scan_text("fixture/src/lib.rs", text, &mut found);
    let keys: Vec<&String> = found.keys().collect();
    let has = |p: &str| keys.iter().any(|k| k.contains(p));
    assert!(has("env::var(\"AFTER\")"), "{keys:?}");
    assert!(has("env::var(\"LAST\")"), "{keys:?}");
    assert!(!has("INSIDE") && !has("IN_MOD"), "{keys:?}");
    let (_, list) = listed();
    assert!(
        found.keys().all(|k| !list.contains_key(k)),
        "the fixture is not listed"
    );
}

const LIST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/proof/hidden-inputs.txt");

/// The list: `class  #  crate/path.rs|kind|line` (the count of identical lines after `x N`).
fn listed() -> (Vec<String>, BTreeMap<String, (String, usize)>) {
    let text = std::fs::read_to_string(LIST).unwrap_or_default();
    let mut head = vec![];
    let mut out = BTreeMap::new();
    for l in text.lines() {
        if l.starts_with('#') || l.trim().is_empty() {
            head.push(l.to_string());
            continue;
        }
        let (class, rest) = l.split_once("  @ ").expect("class  @ entry");
        let (entry, n) = match rest.rsplit_once("  x ") {
            Some((e, n)) => (e.to_string(), n.parse().expect("a count")),
            None => (rest.to_string(), 1),
        };
        out.insert(entry, (class.trim().to_string(), n));
    }
    (head, out)
}

const CLASSES: [&str; 4] = ["keyed", "speed", "no-output", "must-fix"];

#[test]
fn every_hidden_input_is_reviewed() {
    let found = scan();
    let (head, list) = listed();
    if std::env::var_os("FG_REGEN").is_some() {
        let mut lines = head;
        let kept = list
            .iter()
            .filter(|(e, _)| !scanned(e.split('|').next().unwrap_or("")));
        let mut found = found.clone();
        for (e, (_, n)) in kept {
            found.insert(e.clone(), *n);
        }
        for (e, n) in &found {
            let class = list.get(e).map_or("unreviewed".to_string(), |x| x.0.clone());
            let count = if *n > 1 { format!("  x {n}") } else { String::new() };
            lines.push(format!("{class:<14}  @ {e}{count}"));
        }
        std::fs::write(LIST, lines.join("\n") + "\n").expect("write the list");
        return;
    }
    let mut bad = vec![];
    for (e, n) in &found {
        match list.get(e) {
            None => bad.push(format!("not listed: {e}")),
            Some((class, m)) => {
                if !CLASSES
                    .iter()
                    .any(|c| class == c || class.starts_with(&format!("{c} ")))
                {
                    bad.push(format!("{class}: {e}"));
                }
                if m != n {
                    bad.push(format!("{n} lines, listed {m}: {e}"));
                }
            }
        }
    }
    for e in list
        .keys()
        .filter(|e| !found.contains_key(*e) && scanned(e.split('|').next().unwrap_or("")))
    {
        bad.push(format!("listed but gone (FG_REGEN=1 to drop): {e}"));
    }
    assert!(
        bad.is_empty(),
        "hidden inputs not reviewed (tools/proof/hidden-inputs.txt):\n{}",
        bad.join("\n")
    );
}

/// The must-fix items are what stands between the kernel and an exact op memo: this lists them,
/// so the count only goes down.
#[test]
fn the_must_fix_list_only_shrinks() {
    let (_, list) = listed();
    let n = list.values().filter(|(c, _)| c.starts_with("must-fix")).count();
    assert!(
        n <= MUST_FIX,
        "{n} must-fix entries, more than the {MUST_FIX} recorded: fix, don't add"
    );
}

const MUST_FIX: usize = 37;

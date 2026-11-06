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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const CRATES: [&str; 11] = [
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
    if has("available_parallelism") || has("thread::spawn") || has("thread::scope") {
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
            let text = std::fs::read_to_string(&f).expect("read");
            // test modules sit at the end of a file: stop at `#[cfg(test)]`
            for line in text
                .lines()
                .take_while(|l| !l.trim_start().starts_with("#[cfg(test)]"))
            {
                for k in kinds(line) {
                    *out.entry(format!("{rel}|{k}|{}", line.trim())).or_default() += 1;
                }
            }
        }
    }
    out
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
    for e in list.keys().filter(|e| !found.contains_key(*e)) {
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

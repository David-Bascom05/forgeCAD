//! The session memo against a fresh build (research/forge-lang/lean/ForgeLang/OpMemo.lean):
//! `memo_exact` says a rebuild through the memo is the fresh rebuild exactly when the key holds
//! everything an op reads. Here, on the example parts:
//!
//! - random edit walks (a param nudged by a percent or by its last bit, a feature renamed, an edit
//!   taken back) through one session's cache, each version against a fresh build: the body to the
//!   bit (every vertex, every face's and edge's name) and the report;
//! - one mutation per part of the key the Lean model requires (`OpMemo.Counter.Inp`), each
//!   needing a miss: the run after it is the fresh run;
//! - `the_key_parts_are_the_proofs`: every field of the Lean `Inp` is covered here or given a
//!   reason, so a part the proof adds is a test this file must add.
//!
//! Deterministic (fixed seed), temp dirs. `FG_MEMO_LONG=1` walks longer.

use forge_lang::build::Cache;
use forge_lang::error::Op;
use forge_lang::{Options, Run, report};
use std::path::{Path, PathBuf};

fn opts(dir: &Path) -> Options {
    Options {
        dir: Some(dir.to_path_buf()),
        ..Options::default()
    }
}

fn bits(r: &Run) -> Vec<u64> {
    let Some(b) = &r.body else { return vec![] };
    let mut out = vec![b.face_ids().count() as u64, b.edge_ids().count() as u64];
    for v in &b.vertices {
        out.extend([v.point.x, v.point.y, v.point.z].map(f64::to_bits));
    }
    out.extend(b.face_ids().map(|f| b.face(f).name.0));
    out.extend(b.edge_ids().map(|e| b.edge(e).name.0));
    out
}

/// The report less how it was built (times, counts of rebuilt and reused work).
fn shown(r: &Run, src: &str) -> String {
    let mut s = String::new();
    report::json(r, src).write(&mut s);
    let mut s = match s.find("\"timings_ms\":{") {
        Some(at) => {
            let end = at + s[at..].find('}').expect("closed") + 2;
            format!("{}{}", &s[..at], &s[end..])
        }
        None => s,
    };
    for key in ["\"rebuilt\":", "\"reused\":", "\"remade\":"] {
        if let Some(at) = s.find(key) {
            let end = at + s[at..].find([',', '}']).expect("ends") + 1;
            s = format!("{}{}", &s[..at], &s[end..]);
        }
    }
    s
}

struct Rng(u64);
impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n.max(1) as u64) as usize
    }
}

/// `param`/`shared` lines with a plain number: (line, name, number text).
fn numbers(src: &str) -> Vec<(String, String, String)> {
    src.lines()
        .filter(|l| l.starts_with("param ") || l.starts_with("shared "))
        .filter_map(|l| {
            let (head, rhs) = l.split_once(" = ")?;
            let num: String = rhs
                .chars()
                .enumerate()
                .take_while(|(k, c)| c.is_ascii_digit() || *c == '.' || (*k == 0 && *c == '-'))
                .map(|x| x.1)
                .collect();
            num.parse::<f64>().ok()?;
            Some((l.to_string(), head.split_whitespace().nth(1)?.to_string(), num))
        })
        .collect()
}

fn set_number(src: &str, line: &str, num: &str, v: f64) -> String {
    src.replacen(
        line,
        &line.replacen(&format!(" = {num}"), &format!(" = {v:?}"), 1),
        1,
    )
}

/// The next version: a param nudged by about a percent, or by its last bit, or a feature renamed.
fn mutate(src: &str, rng: &mut Rng) -> Option<String> {
    let nums = numbers(src);
    match rng.below(3) {
        0 | 1 if !nums.is_empty() => {
            let (line, _, num) = &nums[rng.below(nums.len())];
            let v: f64 = num.parse().ok()?;
            let w = if rng.below(2) == 0 {
                v * 1.01 + 0.1
            } else {
                f64::from_bits(v.to_bits() + 1)
            };
            Some(set_number(src, line, num, w))
        }
        _ => {
            let feats: Vec<&str> = src
                .lines()
                .filter_map(|l| l.split_once(" = ").map(|x| x.0))
                .filter(|n| n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !n.is_empty())
                .collect();
            let from = feats.get(rng.below(feats.len()))?;
            let op = Op::Rename {
                from: from.to_string(),
                to: format!("{from}_r"),
            };
            forge_lang::edit::apply(src, &op).ok()
        }
    }
}

fn examples() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    [
        "demo/part_tools.fg",
        "machined/flange.fg",
        "rack18u/frame/post.fg",
        "thermoform/tray.fg",
        "demo/sheet_tools.fg",
    ]
    .iter()
    .map(|f| root.join(f))
    .filter(|p| p.exists())
    .collect()
}

/// A session run and a fresh run of `src` agree, or the difference.
fn agree(src: &str, dir: &Path, cache: Cache) -> (Option<String>, Cache) {
    let (kept, cache) = forge_lang::run(src, opts(dir), cache);
    let (fresh, _) = forge_lang::run(src, opts(dir), Cache::default());
    let d = if bits(&kept) != bits(&fresh) {
        Some("the body differs from a fresh build's".into())
    } else if shown(&kept, src) != shown(&fresh, src) {
        Some("the report differs from a fresh build's".into())
    } else {
        None
    };
    (d, cache)
}

#[test]
fn every_version_of_an_edit_walk_is_a_fresh_build() {
    let steps = if std::env::var_os("FG_MEMO_LONG").is_some() {
        40
    } else {
        8
    };
    let mut rng = Rng(0x6d65_6d6f_0000_0001);
    let mut bad = vec![];
    for path in examples() {
        let dir = path.parent().expect("a dir");
        let first = std::fs::read_to_string(&path).expect("read");
        let mut versions = vec![first.clone()];
        let mut cache = Cache::default();
        let mut src = first;
        for k in 0..steps {
            // now and then an edit taken back: a version from before, served from the memo
            src = if k % 3 == 2 {
                versions[rng.below(versions.len())].clone()
            } else {
                mutate(&src, &mut rng).unwrap_or(src)
            };
            versions.push(src.clone());
            let (d, c) = agree(&src, dir, cache);
            cache = c;
            if let Some(d) = d {
                bad.push(format!("{} step {k}: {d}", path.display()));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "the memo served a build a fresh build does not make:\n{}",
        bad.join("\n")
    );
}

/// Two versions differing only in one part of the key: after the first, the second must miss
/// (be built as a fresh build builds it).
fn after_is_fresh(before: &str, after: &str, dir: &Path) -> Option<String> {
    let (_, cache) = forge_lang::run(before, opts(dir), Cache::default());
    agree(after, dir, cache).0
}

fn part_tools() -> (String, PathBuf) {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/demo/part_tools.fg");
    (
        std::fs::read_to_string(&p).expect("read"),
        p.parent().expect("dir").to_path_buf(),
    )
}

// `params`: a parameter's last bit
#[test]
fn key_part_params() {
    let (src, dir) = part_tools();
    let (line, _, num) = numbers(&src).into_iter().next().expect("a number");
    let v: f64 = num.parse().expect("a number");
    let after = set_number(&src, &line, &num, f64::from_bits(v.to_bits() + 1));
    assert_eq!(after_is_fresh(&src, &after, &dir), None);
}

// `naming`: the same feature under another name stamps other ids
#[test]
fn key_part_naming() {
    let (src, dir) = part_tools();
    let after = forge_lang::edit::apply(
        &src,
        &Op::Rename {
            from: "extrude1".into(),
            to: "extrude9".into(),
        },
    )
    .expect("renames");
    assert_eq!(after_is_fresh(&src, &after, &dir), None);
}

// `inputs` and `before`: an earlier feature changed, a later one's own text not
#[test]
fn key_part_before() {
    let (src, dir) = part_tools();
    let (line, _, num) = numbers(&src).into_iter().next().expect("a number");
    let v: f64 = num.parse().expect("a number");
    let after = set_number(&src, &line, &num, v * 0.9);
    assert_eq!(after_is_fresh(&src, &after, &dir), None);
}

// `kind`: the same edges and size, a chamfer for a fillet
#[test]
fn key_part_kind() {
    let (src, dir) = part_tools();
    let after = src.replacen("fillet1 = fillet(", "fillet1 = chamfer(", 1);
    assert_ne!(after, src, "part_tools has fillet1");
    assert_eq!(after_is_fresh(&src, &after, &dir), None);
}

// `file`: a font of the same name with other content (F1 in forgecad-gate/instant-load/
// op-memo-proof.md): the session reuses a file-reading step only on the very file it read
#[test]
fn key_part_file_content() {
    let (src, dir) = part_tools();
    let other = ["/usr/share/fonts/truetype/bebasneue/BebasNeue-Regular.ttf"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists());
    let Some(other) = other else {
        eprintln!("key_part_file_content: skipped, no second font here");
        return;
    };
    let tmp = std::env::temp_dir().join(format!("memo-diff-font-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("mkdir");
    let font = "PublicSans-SemiBold.ttf";
    std::fs::copy(dir.join(font), tmp.join(font)).expect("copy font");
    let (_, cache) = forge_lang::run(&src, opts(&tmp), Cache::default());
    std::fs::copy(&other, tmp.join(font)).expect("swap font");
    let (d, _) = agree(&src, &tmp, cache);
    assert_eq!(d, None, "a font changed under its name");
}

/// `file`, by content: a font replaced by another of the same length with its old modification
/// time put back (what `cp -p`, `rsync -t` or an editor that keeps times do). A key that holds the
/// file's path, length and time and not its bytes serves the old font: `key_needs_file_content`.
#[test]
fn key_part_file_content_same_stamp() {
    let other = PathBuf::from("/usr/share/fonts/truetype/bebasneue/BebasNeue-Regular.ttf");
    if !other.exists() {
        eprintln!("key_part_file_content_same_stamp: skipped, no second font here");
        return;
    }
    let (src, dir) = part_tools();
    let font = "PublicSans-SemiBold.ttf";
    let a = std::fs::read(dir.join(font)).expect("font");
    let b = std::fs::read(&other).expect("font");
    let n = a.len().max(b.len());
    let pad = |mut v: Vec<u8>| {
        v.resize(n, 0);
        v
    };
    let (a, b) = (pad(a), pad(b));
    let tmp = std::env::temp_dir().join(format!("memo-diff-stamp-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("mkdir");
    let when = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    let put = |name: &str, bytes: &[u8]| {
        std::fs::write(tmp.join(name), bytes).expect("write");
        let f = std::fs::File::options()
            .write(true)
            .open(tmp.join(name))
            .expect("open");
        f.set_modified(when).expect("set time");
    };
    // the same text naming another file holding each content: what a fresh read gives
    let fresh = |name: &str, bytes: &[u8]| {
        put(name, bytes);
        let (r, _) = forge_lang::run(&src.replace(font, name), opts(&tmp), Cache::default());
        bits(&r)
    };
    let fresh_a = fresh("a.ttf", &a);
    let fresh_b = fresh("b.ttf", &b);
    assert_ne!(fresh_a, fresh_b, "the two fonts make different text");
    put(font, &a);
    let (first, cache) = forge_lang::run(&src, opts(&tmp), Cache::default());
    assert_eq!(bits(&first), fresh_a, "the file's name does not change the part");
    put(font, &b);
    let (kept, _) = forge_lang::run(&src, opts(&tmp), cache);
    assert_eq!(
        bits(&kept),
        fresh_b,
        "the old font served for new bytes of the same length and time"
    );
}

/// `backend` and thread count: a fresh build is the same on one thread and on all, and with the
/// batch backend on or off (when it is not, the backend must be in the key: op-memo-proof.md 5).
#[test]
fn key_part_backend_and_threads() {
    let fg = env!("CARGO_BIN_EXE_fg");
    let tmp = std::env::temp_dir().join(format!("memo-diff-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("mkdir");
    let mut bad = vec![];
    for path in examples() {
        let out = |envs: &[(&str, &str)], tag: &str| {
            let step = tmp.join(format!("{tag}.step"));
            let mut c = std::process::Command::new(fg);
            c.args(["run", path.to_str().expect("utf8"), "--json", "--step"])
                .arg(&step);
            c.env("FORGECAD_CACHE", "off");
            for (k, v) in envs {
                c.env(k, v);
            }
            let o = c.output().expect("fg runs");
            let json = String::from_utf8(o.stdout).expect("utf8");
            let json = match json.find("\"timings_ms\":{") {
                Some(at) => {
                    let end = at + json[at..].find('}').expect("closed") + 2;
                    format!("{}{}", &json[..at], &json[end..])
                }
                None => json,
            };
            let step: String = std::fs::read_to_string(&step)
                .unwrap_or_default()
                .lines()
                .filter(|l| !l.starts_with("FILE_NAME"))
                .collect();
            (json, step)
        };
        let base = out(&[], "base");
        for (envs, what) in [
            (vec![("FORGECAD_THREADS", "1")], "one thread"),
            (vec![("FORGE_BATCH", "cpu")], "FORGE_BATCH=cpu"),
            (vec![("FORGE_BATCH", "gpu")], "FORGE_BATCH=gpu"),
        ] {
            if out(&envs, "other") != base {
                bad.push(format!("{}: {what} differs", path.display()));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "a fresh build depends on how it is run:\n{}",
        bad.join("\n")
    );
}

/// Every part of the input the Lean proof says a key must hold (`OpMemo.Counter.Inp`), and where
/// it is checked here or why it needs no test.
const COVERED: [(&str, &str); 10] = [
    ("kind", "key_part_kind"),
    ("params", "key_part_params"),
    (
        "inputs",
        "key_part_before (the shapes an op takes are the stages before it)",
    ),
    ("naming", "key_part_naming"),
    (
        "tol",
        "no test: kernel tolerances are constants (forge-geom tol.rs), so `version` holds them",
    ),
    ("backend", "key_part_backend_and_threads"),
    (
        "before",
        "key_part_before, every_version_of_an_edit_walk_is_a_fresh_build",
    ),
    ("file", "key_part_file_content"),
    ("units", "no test: units are a literal's suffix, part of `params`"),
    (
        "version",
        "no test: the session memo lives in one process, one kernel",
    ),
];

#[test]
fn the_key_parts_are_the_proofs() {
    let lean = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/forge-lang/lean/ForgeLang/OpMemo.lean"),
    )
    .expect("OpMemo.lean");
    let at = lean.find("structure Inp where").expect("the Lean Inp");
    let fields: Vec<&str> = lean[at..]
        .lines()
        .skip(1)
        .take_while(|l| l.starts_with("  ") && !l.trim_start().starts_with("deriving"))
        .filter_map(|l| l.trim().split_once(" : ").map(|x| x.0))
        .collect();
    assert!(fields.len() >= 10, "read {fields:?}");
    for f in &fields {
        assert!(
            COVERED.iter().any(|(k, _)| k == f),
            "the proof's key part `{f}` has no test here"
        );
    }
    for (k, _) in COVERED {
        assert!(fields.contains(&k), "`{k}` is not a key part of the proof");
    }
}

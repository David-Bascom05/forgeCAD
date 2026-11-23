//! `syntax::reparse_number` against `syntax::parse`: on every number of every .fg file in the
//! repository, edited several ways (a dimension scaled, grown, shrunk, its unit dropped, its
//! sign flipped), and on edits that are not a number's: wherever it patches the old tree, the
//! tree is the one the new text parses to, spans and all; where it cannot, it says so and leaves
//! the tree as it was.
//!
//! By default a sample: every 13th token of each file, two edits of each number, so the test run
//! stays short. `FG_REPARSE_FULL=1` edits every token every way (some minutes).

use forge_lang::syntax::{self, TokKind};

fn fg_files() -> Vec<std::path::PathBuf> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut dirs = vec![std::path::PathBuf::from(root)];
    let mut out = Vec::new();
    while let Some(d) = dirs.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !name.starts_with('.') && name != "target" && !name.starts_with("target") {
                    dirs.push(path);
                }
            } else if name.ends_with(".fg") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Whether `old` edited into `new` patches, and if so to `parse(new)`.
fn check(old: &str, new: &str, patched: &mut usize, refused: &mut usize) {
    let Ok(mut prog) = syntax::parse(old) else { return };
    let before = prog.clone();
    if syntax::reparse_number(old, &mut prog, new) {
        let fresh =
            syntax::parse(new).unwrap_or_else(|e| panic!("patched, but {new:?} does not parse: {e:?}"));
        if prog != fresh {
            let p = old.bytes().zip(new.bytes()).take_while(|(a, b)| a == b).count();
            let k = prog.iter().zip(&fresh).position(|(a, b)| a != b);
            panic!(
                "patched tree differs from the parse of the edit at byte {p}: {:?} -> {:?}\n patched {:?}\n parsed  {:?}",
                &old[p.saturating_sub(10)..(p + 10).min(old.len())],
                &new[p.saturating_sub(10)..(p + 10).min(new.len())],
                k.map(|k| &prog[k]),
                k.map(|k| &fresh[k])
            );
        }
        *patched += 1;
    } else {
        assert!(prog == before, "refused, but the tree changed");
        *refused += 1;
    }
}

#[test]
fn a_number_edited_patches_to_the_parse_of_the_edit() {
    let full = std::env::var_os("FG_REPARSE_FULL").is_some();
    let (mut patched, mut refused) = (0, 0);
    let files = fg_files();
    assert!(files.len() > 20, "{} .fg files", files.len());
    for path in &files {
        let src = std::fs::read_to_string(path).expect("a .fg file");
        let Ok(toks) = syntax::lex(&src) else { continue };
        if syntax::parse(&src).is_err() {
            continue;
        }
        for (k, t) in toks.iter().enumerate() {
            if !full && k % 13 != 0 {
                continue;
            }
            let (lo, hi) = (t.span.lo as usize, t.span.hi as usize);
            let text = &src[lo..hi];
            let mut variants: Vec<String> = Vec::new();
            if let TokKind::Num(v, _) = t.kind {
                let unit = if text.ends_with("mm") {
                    "mm"
                } else if text.ends_with("deg") {
                    "deg"
                } else {
                    ""
                };
                variants.push(format!("{}{unit}", v * 1.01));
                variants.push(format!("{}{unit}", v + 1.0));
                variants.push(format!("0{unit}"));
                variants.push(format!("{}{unit}", v * 1000.0 + 0.25));
                variants.push(format!("{v}"));
                variants.push(format!("{}{unit}", -v));
                if !full {
                    variants.truncate(2);
                }
            } else {
                // not a number: a space before the token, and a digit after it
                variants.push(format!(" {text}"));
                variants.push(format!("{text}1"));
            }
            for var in variants {
                let new = format!("{}{var}{}", &src[..lo], &src[hi..]);
                check(&src, &new, &mut patched, &mut refused);
            }
        }
        // each `shared`/`param` line written out again with its value scaled, its comment
        // dropped, as fgcount's edit writes it
        let mut at = 0;
        for line in src.split_inclusive('\n') {
            let body = line.trim_end();
            if let Some(h) = body
                .strip_prefix("shared ")
                .or_else(|| body.strip_prefix("param "))
                && let Some((_, v)) = h.split_once(" = ")
                && let Ok(x) = v.split('#').next().unwrap_or("").trim().parse::<f64>()
            {
                let prefix = &line[..line.find(" = ").expect("found") + 3];
                let new = format!("{}{prefix}{}\n{}", &src[..at], x * 1.01, &src[at + line.len()..]);
                check(&src, &new, &mut patched, &mut refused);
            }
            at += line.len();
        }
        // a comment edited, and the text unchanged
        if let Some(k) = src.find('#') {
            let new = format!("{}# x{}", &src[..k], &src[k + 1..]);
            check(&src, &new, &mut patched, &mut refused);
        }
        check(&src, &src, &mut patched, &mut refused);
    }
    assert!(
        patched > if full { 1000 } else { 100 },
        "{patched} patched, {refused} refused"
    );
}

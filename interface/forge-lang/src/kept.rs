//! A whole run kept on disk: under a key of the document's place and text (its literals and all),
//! the arguments and the kernel, with the files it read (`deps`): the used files' functions and
//! shared values, parts, imports, fonts and material libraries. Served only while every one of
//! those holds what it held, and served whole: the output of that very text, so its params,
//! spans and provenance are the text's own.

use crate::deps::{self, Deps};
use crate::store::{self, In, Out};
use std::path::Path;

/// The key of a run of the document at `place` holding `src`, with these arguments.
pub fn run_key(place: &str, src: &str, args: &[String]) -> u128 {
    store::key("fg-run", |h| {
        h.str(place);
        h.str(src);
        h.word(args.len() as u64);
        for a in args {
            h.str(a);
        }
    })
}

/// The bytes kept under `key`, if every file they were made from still holds what it held.
pub fn fetch(kind: &str, key: u128) -> Option<Vec<u8>> {
    open(&store::get(kind, key)?)
}

fn open(entry: &[u8]) -> Option<Vec<u8>> {
    let mut i = In(entry);
    let d = Deps::read(&mut i)?;
    d.current().then(|| i.0.to_vec())
}

/// Keeps `bytes` under `key`, with what they were made from.
pub fn keep(kind: &str, key: u128, from: &Deps, bytes: &[u8]) {
    store::put(kind, key, &entry(from, bytes));
}

fn entry(from: &Deps, bytes: &[u8]) -> Vec<u8> {
    let mut o = Out::default();
    from.write(&mut o);
    o.0.extend_from_slice(bytes);
    o.0
}

/// `make`'s answer and what it read, the document at `place` first.
pub fn watched<T>(place: &Path, src: &str, make: impl FnOnce() -> T) -> (T, Deps) {
    let w = deps::watch();
    deps::read(place, Some(src.as_bytes()));
    let t = make();
    (t, w.done())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Options, report};
    use std::path::PathBuf;

    fn dir(tag: &str, files: &[(&str, &str)]) -> PathBuf {
        let d = std::env::temp_dir().join(format!("forge-kept-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).expect("temp dir");
        for (name, text) in files {
            std::fs::write(d.join(name), text).expect("write");
        }
        d
    }

    /// The report of a fresh run, and what it read.
    fn fresh(d: &Path, src: &str) -> (String, Deps) {
        let place = d.join("part.fg");
        std::fs::write(&place, src).expect("write");
        watched(&place, src, || {
            let opts = Options {
                dir: Some(d.to_path_buf()),
                ..Options::default()
            };
            let (run, _) = crate::run(src, opts, Default::default());
            let mut s = String::new();
            report::json(&run, src).write(&mut s);
            s
        })
    }

    /// A report less its timings, the one thing two runs of one text differ in.
    fn untimed(s: &str) -> String {
        let Some(at) = s.find("\"timings_ms\":{") else {
            return s.into();
        };
        let end = at + s[at..].find('}').expect("closed") + 2;
        format!("{}{}", &s[..at], &s[end..])
    }

    /// What a cache would serve for `src` now, given an entry kept for `kept_src`.
    fn served(d: &Path, kept_src: &str, src: &str) -> Option<String> {
        let place = d.join("part.fg").to_string_lossy().to_string();
        let (out, from) = fresh(d, kept_src);
        let e = entry(&from, out.as_bytes());
        (run_key(&place, src, &[]) == run_key(&place, kept_src, &[]))
            .then(|| open(&e))
            .flatten()
            .map(|b| String::from_utf8(b).expect("text"))
    }

    const USED: &str = "shared t = 2\nfn twice(x) {\n  return 2 * x\n}\n";
    const PART: &str = "lib = use(\"lib.fg\")\nshared w = 10\nsketch s(on: xy) {\n  o = rect(corner: (0, 0), size: (w, lib.twice(5)))\n}\nb = extrude(s, lib.t)\n";

    #[test]
    fn an_unchanged_document_is_served_its_own_output() {
        let d = dir("same", &[("lib.fg", USED)]);
        let got = served(&d, PART, PART).expect("served");
        assert_eq!(untimed(&got), untimed(&fresh(&d, PART).0));
    }

    // key_needs_literals: the same text but for a number is another key
    #[test]
    fn a_literal_is_part_of_the_key() {
        let d = dir("lit", &[("lib.fg", USED)]);
        let other = PART.replace("w = 10", "w = 11");
        assert_eq!(served(&d, PART, &other), None);
    }

    // key_needs_defs: a function the document calls, changed in its file, is a miss
    #[test]
    fn a_used_function_is_part_of_what_is_checked() {
        let d = dir("defs", &[("lib.fg", USED)]);
        let (_, from) = fresh(&d, PART);
        assert!(from.current());
        std::fs::write(d.join("lib.fg"), USED.replace("2 * x", "3 * x")).expect("write");
        assert!(!from.current(), "the used file's function changed");
        let (now, _) = fresh(&d, PART);
        let (was, _) = {
            std::fs::write(d.join("lib.fg"), USED).expect("write");
            fresh(&d, PART)
        };
        assert_ne!(now, was, "the function's body changes the result");
    }

    // key_needs_shared: a shared value the document reads, changed where it is defined, is a miss
    #[test]
    fn a_read_shared_value_is_part_of_what_is_checked() {
        let d = dir("shared", &[("lib.fg", USED)]);
        let (_, from) = fresh(&d, PART);
        std::fs::write(d.join("lib.fg"), USED.replace("t = 2", "t = 3")).expect("write");
        assert!(!from.current(), "the used file's shared value changed");
        std::fs::write(d.join("lib.fg"), USED).expect("write");
        assert!(from.current());
    }

    // and the kernel: another build's entries are never read (store's own test), and its key differs
    #[test]
    fn the_kernel_is_part_of_the_key() {
        let a = store::key("fg-run", |h| h.str("x"));
        let mut h = forge_math::hash::Hash128::new();
        h.str("another kernel");
        h.str("fg-run");
        h.str("x");
        assert_ne!(a, h.finish());
    }

    // concrete_cache_wrong: two params of equal value; an edit of one is served nothing kept for
    // the other, and what is served carries the text's own params, so SetParam edits the right one
    #[test]
    fn equal_values_keep_their_own_provenance() {
        let src = "shared small = 80\nshared big = 80\nsketch s(on: xy) {\n  o = rect(corner: (0, 0), size: (small, big))\n}\nb = extrude(s, 2)\n";
        let d = dir("prov", &[]);
        let served_out = served(&d, src, src).expect("served");
        assert_eq!(untimed(&served_out), untimed(&fresh(&d, src).0));
        assert!(served_out.contains("\"param\":\"big\",\"shared\":true,\"structural\":false,\"line\":2"));
        let op = crate::error::Op::SetParam {
            param: crate::error::ParamKey {
                owner: vec![],
                name: "big".into(),
            },
            value: crate::error::Literal {
                value: 81.0,
                dim: crate::values::Dim::Length,
            },
        };
        let edited = crate::edit::apply(src, &op).expect("edits");
        assert!(edited.contains("small = 80") && edited.contains("big = 81"));
        assert_eq!(served(&d, src, &edited), None, "the edited text is another key");
        let (out, _) = fresh(&d, &edited);
        assert!(out.contains("\"param\":\"big\"") && out.contains("81"), "{out}");
    }
}

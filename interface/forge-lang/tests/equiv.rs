//! The table-driven parser against the hand parser it replaced (tests/old_parser): identical
//! trees, or the same error at the same span, on every Forge text in the crate, the fair-test
//! models, random sentences of grammar.ll1 and random mutations of all of those.
//! `FG_GRAMMAR=path` draws the sentences from another grammar file; `FG_FAIR=dir` adds texts.

mod ll1;
mod old_parser;

use forge_lang::error::{Error, SyntaxError};
use forge_lang::syntax::{self, Stmt};
use ll1::{Rng, min_lengths, sentence};

type Parsed = Result<Vec<Stmt>, Error>;

#[derive(Default)]
struct Tally {
    texts: usize,
    same_tree: usize,
    same_error: usize,
    /// Same error and span, worded differently.
    wording: Vec<String>,
    differ: Vec<String>,
}

impl Tally {
    fn check(&mut self, src: &str) {
        let old = old_parser::parse(src);
        let new = syntax::parse(src);
        self.texts += 1;
        match (&old, &new) {
            (Ok(a), Ok(b)) if a == b => self.same_tree += 1,
            (Err(a), Err(b)) if a == b => self.same_error += 1,
            (
                Err(Error::Syntax(SyntaxError::Expected {
                    what: w0, span: s0, ..
                })),
                Err(Error::Syntax(SyntaxError::Expected {
                    what: w1,
                    span: s1,
                    expected,
                })),
            ) if s0 == s1 => {
                self.same_error += 1;
                if w0 != w1 {
                    self.wording.push(format!(
                        "{src:?}\n  old: expected {w0}\n  new: expected {w1} (one of {:?})",
                        expected.names()
                    ));
                }
            }
            _ => self
                .differ
                .push(format!("{src:?}\n  old: {}\n  new: {}", show(&old), show(&new))),
        }
    }

    fn report(&self, what: &str) {
        println!(
            "{what}: {} texts, {} same tree, {} same error, {} worded differently, {} differ",
            self.texts,
            self.same_tree,
            self.same_error,
            self.wording.len(),
            self.differ.len()
        );
        for d in self.differ.iter().take(20) {
            println!("DIFF {d}");
        }
        let mut seen = std::collections::BTreeSet::new();
        for w in &self.wording {
            let key = w.split_once('\n').map_or("", |x| x.1).to_string();
            if seen.insert(key) && seen.len() <= 12 {
                println!("WORDING {w}");
            }
        }
    }
}

fn show(p: &Parsed) -> String {
    match p {
        Ok(s) => {
            let t = format!("{s:?}");
            if t.len() > 400 {
                format!("{}...", &t[..400])
            } else {
                t
            }
        }
        Err(e) => format!("{e:?}"),
    }
}

/// Every Forge text in the crate: parts, string literals in the tests and sources, and the code
/// blocks of the docs.
fn corpus() -> Vec<(String, String)> {
    let root = env!("CARGO_MANIFEST_DIR");
    let mut out = Vec::new();
    let mut files = vec![];
    let mut dirs = vec![
        format!("{root}/parts"),
        format!("{root}/tests"),
        format!("{root}/src"),
    ];
    if let Ok(fair) = std::env::var("FG_FAIR") {
        dirs.push(fair);
    }
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                dirs.push(p.to_string_lossy().into());
            } else {
                files.push(p);
            }
        }
    }
    files.sort();
    for p in files {
        let name = p.to_string_lossy().to_string();
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        match p.extension().and_then(|e| e.to_str()) {
            Some("fg") => out.push((name, text)),
            Some("rs") if !name.contains("old_parser") => {
                for (k, lit) in string_literals(&text).into_iter().enumerate() {
                    out.push((format!("{name}#{k}"), lit));
                }
            }
            Some("md") => {
                for (k, block) in text.split("```").skip(1).step_by(2).enumerate() {
                    let body = block.split_once('\n').map_or(block, |x| x.1);
                    out.push((format!("{name}#{k}"), body.to_string()));
                }
            }
            _ => {}
        }
    }
    out
}

/// The string literals of a Rust file, escapes undone (plain and raw strings).
fn string_literals(src: &str) -> Vec<String> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i] == b'\'' {
            // a char literal or a lifetime: skip `'"'` and `'\''`
            if b.get(i + 2) == Some(&b'\'') {
                i += 3;
            } else if b.get(i + 1) == Some(&b'\\') && b.get(i + 3) == Some(&b'\'') {
                i += 4;
            } else {
                i += 1;
            }
        } else if b[i] == b'r'
            && (b.get(i + 1) == Some(&b'#') || b.get(i + 1) == Some(&b'"'))
            && (i == 0 || !b[i - 1].is_ascii_alphanumeric())
        {
            let mut h = 0;
            let mut j = i + 1;
            while b.get(j) == Some(&b'#') {
                h += 1;
                j += 1;
            }
            if b.get(j) != Some(&b'"') {
                i += 1;
                continue;
            }
            let close = format!("\"{}", "#".repeat(h));
            let body = &src[j + 1..];
            let end = body.find(&close).unwrap_or(body.len());
            out.push(body[..end].to_string());
            i = j + 1 + end + close.len();
        } else if b[i] == b'"' {
            let mut s = String::new();
            i += 1;
            while i < b.len() && b[i] != b'"' {
                if b[i] == b'\\' {
                    i += 1;
                    match b.get(i) {
                        Some(b'n') => s.push('\n'),
                        Some(b't') => s.push('\t'),
                        Some(b'\n') => {
                            while i + 1 < b.len() && b[i + 1].is_ascii_whitespace() {
                                i += 1;
                            }
                        }
                        Some(c) => s.push(*c as char),
                        None => {}
                    }
                    i += 1;
                } else {
                    let ch = src[i..].chars().next().unwrap_or(' ');
                    s.push(ch);
                    i += ch.len_utf8();
                }
            }
            i += 1;
            out.push(s);
        } else {
            i += 1;
        }
    }
    out
}

/// A text with a few tokens deleted, doubled or swapped (on whitespace boundaries).
fn mutate(src: &str, rng: &mut Rng) -> String {
    let mut words: Vec<&str> = src
        .split_inclusive(|c: char| c.is_whitespace() || "(),[]{}.:=".contains(c))
        .collect();
    if words.is_empty() {
        return String::new();
    }
    for _ in 0..1 + rng.below(3) {
        let k = rng.below(words.len());
        match rng.below(3) {
            0 => {
                words.remove(k);
            }
            1 => words.insert(k, words[k]),
            _ => {
                let j = rng.below(words.len());
                words.swap(k, j);
            }
        }
        if words.is_empty() {
            break;
        }
    }
    words.concat()
}

#[test]
fn same_trees_on_every_forge_text() {
    let mut t = Tally::default();
    let texts = corpus();
    for (_, src) in &texts {
        t.check(src);
    }
    t.report("corpus");
    let mut rng = Rng(0x5eed_f00d);
    let mut m = Tally::default();
    for _ in 0..5 {
        for (_, src) in &texts {
            m.check(&mutate(src, &mut rng));
        }
    }
    m.report("corpus mutations");
    assert!(t.differ.is_empty() && m.differ.is_empty());
}

#[test]
fn same_trees_on_random_sentences() {
    let path =
        std::env::var("FG_GRAMMAR").unwrap_or(concat!(env!("CARGO_MANIFEST_DIR"), "/grammar.ll1").into());
    let g = ll1::read(&std::fs::read_to_string(path).expect("grammar"));
    let minlen = min_lengths(&g);
    let n: usize = std::env::var("FG_SENTENCES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(5000);
    let mut rng = Rng(0x0123_4567_89ab_cdef);
    let mut t = Tally::default();
    let mut m = Tally::default();
    let mut new_ok = 0;
    for _ in 0..n {
        let s = sentence(&g, &mut rng, &minlen);
        if syntax::parse(&s).is_ok() {
            new_ok += 1;
        }
        t.check(&s);
        m.check(&mutate(&s, &mut rng));
    }
    t.report("sentences");
    println!("sentences the new parser accepts: {new_ok} of {n}");
    m.report("sentence mutations");
    assert!(t.differ.is_empty() && m.differ.is_empty());
}

/// Best of many runs, the two parsers alternating so that load on the machine hits both alike.
#[test]
fn parse_time_against_the_hand_parser() {
    for name in ["bracket", "crank", "manifold"] {
        let src =
            std::fs::read_to_string(format!("{}/parts/{name}.fg", env!("CARGO_MANIFEST_DIR"))).expect("part");
        let mut best = [f64::MAX; 4];
        for _ in 0..1000 {
            let runs: [&dyn Fn() -> bool; 4] = [
                &|| old_parser::parse(&src).is_ok(),
                &|| syntax::parse(&src).is_ok(),
                &|| old_parser::lex(&src).is_ok(),
                &|| syntax::lex(&src).is_ok(),
            ];
            for (k, f) in runs.iter().enumerate() {
                let t = std::time::Instant::now();
                for _ in 0..10 {
                    assert!(std::hint::black_box(f()));
                }
                best[k] = best[k].min(t.elapsed().as_secs_f64() * 1e5);
            }
        }
        println!(
            "{name}: hand {:.1} us (lex {:.1}), table {:.1} us (lex {:.1})",
            best[0], best[2], best[1], best[3]
        );
    }
}

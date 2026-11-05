//! The parser against the proven one. `research/forge-lang/lean/ForgeLang/LL1/Impl.lean` proves
//! the grammar of table.rs LL(1) and its TABLE equal to the choice of the Lean parser `parse`
//! (`table_choose`), so what is left is the hand-written part: the stack driver in syntax.rs, its
//! actions and the lexer. Here, on every generated input, after the lexer:
//!
//! - `reference`, a line-by-line port of Lean's `parse` (LL1/Proof.lean) on TABLE, gives the
//!   tree or none. Its answers are the Lean oracle's (`lake exe fgparse`, Oracle/Parse.lean), by
//!   a digest of every answer kept in tests/parse_diff.digest. `FG_LEAN=1` runs the oracle,
//!   compares answer by answer and rewrites the digest.
//! - the parser accepts exactly the inputs the reference has a tree for, but for the checks its
//!   actions make beyond the grammar (`ACTION_CHECKS`).
//! - its syntax tree is the reference tree's: the same as for the text printed from the reference
//!   tree with every operation bracketed (left-associated chains, right-associated `^`), compared
//!   without spans.
//!
//! Inputs: every token string of up to `SHORT` tokens whose proper prefixes the grammar allows
//! (so every sentence that short and every first error), random sentences of the grammar and one
//! random edit of each, and every .fg file in the repository. `FG_DIFF_LONG=1` goes longer.

mod ll1;

use forge_lang::error::{Error, SyntaxError};
use forge_lang::syntax::{self, TokKind, table};
use std::fmt::Write as _;

const DIGEST_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/parse_diff.digest");
const LEAN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/forge-lang/lean");

fn long() -> bool {
    std::env::var_os("FG_DIFF_LONG").is_some()
}

/// The terminal a token is, as the parser numbers it.
fn term(k: &TokKind) -> u8 {
    match k {
        TokKind::Num(..) => table::t::NUM,
        TokKind::Str(_) => table::t::STR,
        TokKind::Id(_) => table::t::ID,
        TokKind::Nl => table::t::NL,
        TokKind::Eof => table::t::EOF,
        TokKind::Kw(w) | TokKind::Op(w) => {
            table::TERMS.iter().position(|t| t == w).expect("a terminal") as u8
        }
    }
}

const NUMS: [&str; 12] = [
    "1", "2", "0", "12", "2.5", "1e3", "0.5mm", "90deg", "-1", "-2.5mm", "7.", "1.25e-2",
];
const IDS: [&str; 6] = ["a", "b", "x", "w_2", "f", "plan"];

/// A token string as text: one space between tokens, `v` picking among the spellings of a class.
fn render(w: &[u8], mut v: impl FnMut() -> usize) -> String {
    let mut out = String::new();
    for &a in w {
        let s = match a {
            table::t::NUM => NUMS[v() % NUMS.len()].to_string(),
            table::t::ID => IDS[v() % IDS.len()].to_string(),
            table::t::STR => "\"s\"".into(),
            table::t::NL => "\n".into(),
            table::t::EOF => continue,
            x => table::TERMS[x as usize].to_string(),
        };
        out.push_str(&s);
        out.push(' ');
    }
    out
}

#[derive(Debug)]
enum T {
    Leaf(usize),
    Node(usize, Vec<T>),
}

/// Lean's `choose` on `la`: TABLE for a token (`table_choose`); at the end of the input, the empty
/// rule of a nonterminal the end may follow.
fn choose(a: usize, la: Option<u8>) -> Option<usize> {
    match la {
        Some(x) => Some(table::TABLE[a][x as usize])
            .filter(|p| *p != table::NONE)
            .map(usize::from),
        None => (0..table::NP).find(|&p| {
            table::LHS[p] as usize == a
                && table::FOLLOW_END[a]
                && table::RHS[p]
                    .iter()
                    .all(|s| *s >= table::NT_BASE && table::NULLABLE[(*s - table::NT_BASE) as usize])
        }),
    }
}

/// Lean's `parse G T n X w`, with `w` as `w[k..]`.
fn parse_sym(n: usize, x: u8, w: &[u8], k: usize) -> Option<(T, usize)> {
    let n = n.checked_sub(1)?;
    if x < table::NT_BASE {
        return (w.get(k) == Some(&x)).then_some((T::Leaf(k), k + 1));
    }
    let r = choose((x - table::NT_BASE) as usize, w.get(k).copied())?;
    let (f, k) = parse_seq(n, table::RHS[r], w, k)?;
    Some((T::Node(r, f), k))
}

/// Lean's `parseF`.
fn parse_seq(n: usize, alpha: &[u8], w: &[u8], k: usize) -> Option<(Vec<T>, usize)> {
    let n = n.checked_sub(1)?;
    let Some((&x, rest)) = alpha.split_first() else {
        return Some((Vec::new(), k));
    };
    let (t, k) = parse_sym(n, x, w, k)?;
    let (mut f, k) = parse_seq(n, rest, w, k)?;
    f.insert(0, t);
    Some((f, k))
}

/// Oracle/Parse.lean's `answer`, as the tree.
fn reference(w: &[u8]) -> Option<T> {
    let n = (w.len() + 2) * (table::NN + 2);
    let go = |n| match parse_sym(n, table::NT_BASE, w, 0) {
        Some((t, k)) if k == w.len() => Some(t),
        _ => None,
    };
    go(n).or_else(|| go(4 * n))
}

fn preorder(t: &T, out: &mut Vec<usize>) {
    if let T::Node(r, f) = t {
        out.push(*r);
        for c in f {
            preorder(c, out);
        }
    }
}

/// Oracle/Parse.lean's answer line.
fn answer(t: &Option<T>) -> String {
    match t {
        None => "0".into(),
        Some(t) => {
            let mut p = Vec::new();
            preorder(t, &mut p);
            let mut s = "1".to_string();
            for r in p {
                write!(s, " {r}").unwrap();
            }
            s
        }
    }
}

fn nt(t: &T) -> &'static str {
    match t {
        T::Node(r, _) => table::NONTERMS[table::LHS[*r] as usize],
        T::Leaf(_) => "",
    }
}

fn kids(t: &T) -> &[T] {
    match t {
        T::Node(_, f) => f,
        T::Leaf(_) => &[],
    }
}

/// The reference tree printed with every operation in brackets.
fn bracketed(t: &T, toks: &[syntax::Tok], src: &str) -> String {
    let p = |t: &T| bracketed(t, toks, src);
    let f = kids(t);
    match (t, nt(t)) {
        (T::Leaf(k), _) => {
            let s = toks[*k].span;
            src[s.lo as usize..s.hi as usize].to_string()
        }
        (_, "expr") => match kids(&f[1]) {
            [_, c, _, e] => format!("(({}) if ({}) else ({}))", p(&f[0]), p(c), p(e)),
            _ => p(&f[0]),
        },
        (_, "or" | "and" | "cmp" | "add" | "mul") => {
            let mut acc = p(&f[0]);
            let mut rest = &f[1];
            while let [op, x, more] = kids(rest) {
                acc = format!("(({acc}) {} ({}))", p(op), p(x));
                rest = more;
            }
            acc
        }
        (_, "not" | "unary") if f.len() == 2 => format!("({} ({}))", p(&f[0]), p(&f[1])),
        (_, "range" | "power") => match kids(&f[1]) {
            [op, x] => format!("(({}) {} ({}))", p(&f[0]), p(op), p(x)),
            _ => p(&f[0]),
        },
        (_, "postfix") => {
            let mut acc = p(&f[0]);
            let mut rest = &f[1];
            loop {
                match kids(rest) {
                    [open, mid, close, more] => {
                        acc = format!("({acc}){}{}{}", p(open), p(mid), p(close));
                        rest = more;
                    }
                    [dot, name, more] => {
                        acc = format!("({acc}){}{}", p(dot), p(name));
                        rest = more;
                    }
                    _ => break,
                }
            }
            acc
        }
        _ => f
            .iter()
            .map(p)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// A tree's Debug text without its spans.
fn shape(stmts: &[syntax::Stmt]) -> String {
    let d = format!("{stmts:?}");
    let mut out = String::with_capacity(d.len());
    let mut rest = d.as_str();
    while let Some(i) = rest.find("Span { lo: ") {
        out.push_str(&rest[..i]);
        let j = rest[i..].find('}').expect("a span ends");
        out.push('S');
        rest = &rest[i + j + 1..];
    }
    out.push_str(rest);
    out
}

/// What the actions refuse beyond the grammar: a sentence may fail with these, never otherwise.
fn action_check(e: &Error) -> Option<&'static str> {
    match e {
        Error::Syntax(SyntaxError::AssignToNonName { .. }) => Some("AssignToNonName"),
        Error::Syntax(SyntaxError::KeywordNotName { .. }) => Some("KeywordNotName"),
        Error::Syntax(SyntaxError::PositionalAfterKeyword { .. }) => Some("PositionalAfterKeyword"),
        Error::Syntax(SyntaxError::ParamWithoutOwner { .. }) => Some("ParamWithoutOwner"),
        Error::Syntax(SyntaxError::Expected {
            what: "an instance index",
            ..
        }) => Some("InstanceIndex"),
        _ => None,
    }
}

#[derive(Default)]
struct Tally {
    inputs: usize,
    sentences: usize,
    rejected: usize,
    lex_errors: usize,
    checks: std::collections::BTreeMap<&'static str, usize>,
    mismatches: Vec<String>,
    /// (token string, reference answer), for the oracle.
    answers: Vec<(String, String)>,
}

impl Tally {
    fn check(&mut self, src: &str, keep: bool) {
        self.inputs += 1;
        let toks = match syntax::lex(src) {
            Ok(t) => t,
            Err(_) => {
                self.lex_errors += 1;
                if syntax::parse(src).is_ok() {
                    self.mismatches.push(format!("parses but does not lex: {src:?}"));
                }
                return;
            }
        };
        let w: Vec<u8> = toks.iter().map(|t| term(&t.kind)).collect();
        let r = reference(&w);
        if keep {
            let ws: Vec<String> = w.iter().map(u8::to_string).collect();
            self.answers.push((ws.join(" "), answer(&r)));
        }
        let mine = syntax::parse(src);
        match (&r, &mine) {
            (Some(t), Ok(stmts)) => {
                self.sentences += 1;
                let b = bracketed(t, &toks, src);
                match syntax::parse(&b) {
                    Ok(again) if shape(&again) == shape(stmts) => {}
                    Ok(again) => self.mismatches.push(format!(
                        "tree differs from the grammar's:\n  text      {src:?}\n  bracketed {b:?}\n  parser    {}\n  grammar   {}",
                        shape(stmts),
                        shape(&again)
                    )),
                    Err(e) => self.mismatches.push(format!(
                        "the bracketed text of a sentence does not parse: {src:?} -> {b:?}: {e:?}"
                    )),
                }
            }
            (Some(_), Err(e)) => match action_check(e) {
                Some(c) => *self.checks.entry(c).or_default() += 1,
                None => self
                    .mismatches
                    .push(format!("a sentence of the grammar is refused: {src:?}: {e:?}")),
            },
            (None, Ok(_)) => self
                .mismatches
                .push(format!("not a sentence of the grammar, but it parses: {src:?}")),
            (None, Err(Error::Syntax(_))) => self.rejected += 1,
            (None, Err(e)) => self
                .mismatches
                .push(format!("refused, but not as syntax: {src:?}: {e:?}")),
        }
    }

    fn report(&self, what: &str) {
        if !self.mismatches.is_empty() {
            let n = self.mismatches.len();
            let some: Vec<&String> = self.mismatches.iter().take(8).collect();
            panic!(
                "{what}: {n} mismatches, the first:\n{}",
                some.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
            );
        }
    }

    fn oracle(&self, corpus: &str, keep: bool) {
        oracle("fgparse", corpus, &self.answers, keep);
    }
}

/// FNV-1a of every (input, answer), in order.
fn digest(answers: &[(String, String)]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (w, a) in answers {
        for b in w.bytes().chain(*b"\t").chain(a.bytes()).chain(*b"\n") {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

/// The Lean oracle `exe` answers as the Rust side did: by the digest kept in parse_diff.digest,
/// or with `FG_LEAN=1` answer by answer (and the digest rewritten when `keep`).
fn oracle(exe: &str, corpus: &str, answers: &[(String, String)], keep: bool) {
    let line = format!("{corpus} {} {:016x}", answers.len(), digest(answers));
    if std::env::var_os("FG_LEAN").is_none() {
        let have = std::fs::read_to_string(DIGEST_PATH).unwrap_or_default();
        assert!(
            have.lines().any(|l| l == line),
            "{corpus}: no Lean answers for these inputs ({line}); run the oracle: (cd research/forge-lang/lean && lake build {exe}) then FG_LEAN=1 cargo test -p forge-lang --test parse_diff"
        );
        return;
    }
    let path = format!("{LEAN_DIR}/.lake/build/bin/{exe}");
    let mut child = std::process::Command::new(&path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("{path}: {e}; lake build {exe} first"));
    let mut input = String::new();
    for (w, _) in answers {
        input.push_str(w);
        input.push('\n');
    }
    let mut stdin = child.stdin.take().expect("stdin");
    let feed = std::thread::spawn(move || {
        use std::io::Write;
        stdin.write_all(input.as_bytes()).expect("write to the oracle");
    });
    let out = child.wait_with_output().expect("the oracle runs");
    feed.join().expect("fed");
    let out = String::from_utf8(out.stdout).expect("utf8");
    let lean: Vec<&str> = out.lines().collect();
    assert_eq!(lean.len(), answers.len(), "{corpus}: one answer per input");
    let bad: Vec<String> = answers
        .iter()
        .zip(&lean)
        .filter(|((_, a), l)| a != *l)
        .take(5)
        .map(|((w, a), l)| format!("  {w}\n    Rust {a}\n    Lean {l}"))
        .collect();
    assert!(
        bad.is_empty(),
        "{corpus}: Rust and Lean disagree:\n{}",
        bad.join("\n")
    );
    if !keep {
        return;
    }
    static FILE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = FILE.lock().unwrap_or_else(|e| e.into_inner());
    let have = std::fs::read_to_string(DIGEST_PATH).unwrap_or_default();
    let mut lines: Vec<String> = have
        .lines()
        .filter(|l| l.split(' ').next() != Some(corpus))
        .map(str::to_string)
        .collect();
    lines.push(line);
    lines.sort();
    std::fs::write(DIGEST_PATH, lines.join("\n") + "\n").expect("write the digest");
}

/// One step of the LL(1) automaton on TABLE: take terminal `a`, or false.
fn step(stack: &mut Vec<u8>, a: u8) -> bool {
    while let Some(&s) = stack.last() {
        if s < table::NT_BASE {
            stack.pop();
            return s == a;
        }
        let p = table::TABLE[(s - table::NT_BASE) as usize][a as usize];
        if p == table::NONE {
            return false;
        }
        stack.pop();
        stack.extend(table::RHS[p as usize].iter().rev());
    }
    false
}

/// Every token string of at most `len` tokens whose proper prefixes are allowed: the sentences
/// and the strings that end at their first error.
fn exhaustive(len: usize, out: &mut Vec<Vec<u8>>) {
    fn go(prefix: &mut Vec<u8>, stack: &[u8], len: usize, out: &mut Vec<Vec<u8>>) {
        for a in 0..table::NT as u8 {
            let mut s = stack.to_vec();
            prefix.push(a);
            if !step(&mut s, a) || s.is_empty() {
                out.push(prefix.clone());
            } else if prefix.len() < len {
                go(prefix, &s, len, out);
            }
            prefix.pop();
        }
    }
    go(&mut Vec::new(), &[table::NT_BASE], len, out);
}

fn short() -> usize {
    if long() { 5 } else { 4 }
}

#[test]
fn every_short_token_string() {
    let mut ws = Vec::new();
    exhaustive(short(), &mut ws);
    let mut t = Tally::default();
    for w in &ws {
        t.check(&render(w, || 0), true);
    }
    t.report("short token strings");
    t.oracle(&format!("exhaustive{}", short()), true);
    assert!(
        t.sentences > 100 && t.rejected > 10000,
        "{} sentences, {} rejected",
        t.sentences,
        t.rejected
    );
}

#[test]
fn random_sentences_and_edits() {
    let g = ll1::read(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/grammar.ll1")).expect("grammar"),
    );
    let minlen = ll1::min_lengths(&g);
    let mut rng = ll1::Rng(0x5eed_f00d_cafe_0001);
    let n = if long() { 40000 } else { 3000 };
    let mut t = Tally::default();
    for _ in 0..n {
        let s = ll1::sentence(&g, &mut rng, &minlen);
        t.check(&s, true);
        // one edit: a token dropped, doubled or replaced by another terminal
        let Ok(toks) = syntax::lex(&s) else { continue };
        let mut w: Vec<u8> = toks
            .iter()
            .map(|x| term(&x.kind))
            .filter(|x| *x != table::t::EOF)
            .collect();
        if w.is_empty() {
            continue;
        }
        let i = rng.below(w.len());
        match rng.below(3) {
            0 => {
                w.remove(i);
            }
            1 => w.insert(i, w[i]),
            _ => w[i] = rng.below(table::NT) as u8,
        }
        let e = render(&w, || rng.below(64));
        t.check(&e, true);
    }
    t.report("random sentences and edits");
    t.oracle(&format!("random{n}"), true);
    assert!(t.sentences > n / 2, "{} sentences", t.sentences);
}

/// Every .fg in the repository, and each with one token dropped at a few places (no digest: the
/// files change; `FG_LEAN=1` still asks Lean).
#[test]
fn every_fg_file() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut files = Vec::new();
    let mut dirs = vec![std::path::PathBuf::from(root)];
    while let Some(d) = dirs.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name();
            let name = name.to_string_lossy();
            if p.is_dir() {
                if !name.starts_with('.') && name != "target" {
                    dirs.push(p);
                }
            } else if name.ends_with(".fg") {
                files.push(p);
            }
        }
    }
    files.sort();
    let mut t = Tally::default();
    for f in &files {
        let src = std::fs::read_to_string(f).expect("read");
        if forge_lang::drawing::is_drawing(&src) {
            continue;
        }
        t.check(&src, true);
        let Ok(toks) = syntax::lex(&src) else { continue };
        for k in (0..toks.len().saturating_sub(1)).step_by(toks.len() / 7 + 1) {
            let s = toks[k].span;
            let cut = format!("{}{}", &src[..s.lo as usize], &src[s.hi as usize..]);
            t.check(&cut, true);
        }
    }
    t.report("the .fg files");
    assert!(
        files.len() > 50 && t.sentences > 50,
        "{} files, {} sentences",
        files.len(),
        t.sentences
    );
    if std::env::var_os("FG_LEAN").is_some() {
        t.oracle("files", false);
    }
}

/// The Rust lexer's pieces, tokens and trivia in order, as Oracle/Lex.lean writes the model's.
fn lexed(src: &str) -> String {
    use forge_lang::syntax::{KEYWORDS, OPS, TriviaKind, Unit};
    let (toks, trivia) = match syntax::lex_lossless(src) {
        Ok(x) => x,
        Err(Error::Syntax(e)) => {
            let (c, s) = match e {
                SyntaxError::BadNumber { span } => ("eN", span),
                SyntaxError::UnknownUnit { span } => ("eU", span),
                SyntaxError::OpenString { span } => ("eS", span),
                SyntaxError::BadChar { span } => ("eC", span),
                other => return format!("other {other:?}"),
            };
            return format!("{c} {} {}", s.lo, s.hi);
        }
        Err(e) => return format!("other {e:?}"),
    };
    let mut pieces: Vec<(u32, u32, String)> = Vec::new();
    for t in &toks {
        let code = match &t.kind {
            TokKind::Num(_, Unit::None) => "n0".into(),
            TokKind::Num(_, Unit::Mm) => "n1".into(),
            TokKind::Num(_, Unit::Deg) => "n2".into(),
            TokKind::Str(_) => "s".into(),
            TokKind::Id(_) => "i".into(),
            TokKind::Kw(w) => format!("k{}", KEYWORDS.iter().position(|k| k == w).expect("a keyword")),
            TokKind::Op(o) => format!("o{}", OPS.iter().position(|k| k == o).expect("an operator")),
            TokKind::Nl => "l".into(),
            TokKind::Eof => continue,
        };
        pieces.push((t.span.lo, t.span.hi, code));
    }
    for t in &trivia {
        let code = match t.kind {
            TriviaKind::Space => "w",
            TriviaKind::Comment => "c",
            TriviaKind::Newline => "j",
        };
        pieces.push((t.span.lo, t.span.hi, code.into()));
    }
    pieces.sort();
    let mut at = 0;
    let mut out = Vec::new();
    for (lo, hi, c) in pieces {
        assert_eq!(lo, at, "pieces tile the source: {src:?}");
        out.push(format!("{c}:{}", hi - lo));
        at = hi;
    }
    assert_eq!(at as usize, src.len(), "pieces tile the source: {src:?}");
    out.join(" ")
}

fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

/// Bytes the lexer treats differently, one or two of each kind.
const ALPHABET: [&str; 30] = [
    "0", "7", ".", "-", "+", "e", "m", "d", "g", "a", "_", "\"", "#", "\n", " ", "\t", "(", ")", "[", "]",
    "{", "}", "=", "<", ">", "!", ":", ",", "*", "é",
];

/// The Rust lexer against the Lean model (`Lex.lean`): every text of up to three bytes of the
/// alphabet, random texts of it, and random programs; the .fg files with `FG_LEAN=1`.
#[test]
fn lexer_is_the_model() {
    let mut texts: Vec<String> = vec![String::new()];
    let n = if long() { 4 } else { 3 };
    let mut level = vec![String::new()];
    for _ in 0..n {
        let next: Vec<String> = level
            .iter()
            .flat_map(|p| ALPHABET.iter().map(move |a| format!("{p}{a}")))
            .collect();
        texts.extend(next.iter().cloned());
        level = next;
    }
    let mut rng = ll1::Rng(0x1e8e_4a11_0000_0002);
    let words = [
        "1.5",
        "2mm",
        "-3",
        "1e3",
        "1e-",
        "1.2.3",
        "..",
        "..=",
        "+-",
        "==",
        "!=",
        "<=",
        ">=",
        "true",
        "false",
        "in",
        "component",
        "x_1",
        "\"s\"",
        "# c\n",
        "\r",
        "deg",
        "1..2",
        "7.",
    ];
    for _ in 0..if long() { 100000 } else { 8000 } {
        let k = rng.below(24);
        let mut s = String::new();
        for _ in 0..k {
            if rng.below(3) == 0 {
                s.push_str(words[rng.below(words.len())]);
            } else {
                s.push_str(ALPHABET[rng.below(ALPHABET.len())]);
            }
        }
        texts.push(s);
    }
    let g = ll1::read(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/grammar.ll1")).expect("grammar"),
    );
    let minlen = ll1::min_lengths(&g);
    for _ in 0..2000 {
        texts.push(ll1::sentence(&g, &mut rng, &minlen));
    }
    let answers: Vec<(String, String)> = texts.iter().map(|t| (hex(t), lexed(t))).collect();
    for kind in [
        "eN ", "eU ", "eS ", "eC ", "n0:", "n1:", "n2:", "k", "o", "s:", "c:", "j:",
    ] {
        let k = answers
            .iter()
            .filter(|(_, a)| a.starts_with(kind) || a.contains(&format!(" {kind}")))
            .count();
        assert!(k > 20, "only {k} answers with {kind:?}");
    }
    oracle("fglex", &format!("lex{}", answers.len()), &answers, true);
}

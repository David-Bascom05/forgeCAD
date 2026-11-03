//! grammar.ll1 as data: FIRST, FOLLOW, the LL(1) table, and the generated src/syntax/table.rs.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::fmt::Write as _;

pub struct Grammar {
    pub nts: Vec<String>,
    pub terms: Vec<String>,
    /// (lhs, rhs) in file order; a symbol is a terminal index or `NT_BASE` + a nonterminal index.
    pub rules: Vec<(usize, Vec<usize>)>,
    pub nullable: Vec<bool>,
    pub first: Vec<BTreeSet<usize>>,
    /// `END` is the end marker after the start symbol.
    pub follow: Vec<BTreeSet<usize>>,
}

pub const NT_BASE: usize = 128;
pub const END: usize = usize::MAX;

pub fn terminal(s: &str) -> bool {
    s.starts_with('\'') || s.chars().all(|c| c.is_ascii_uppercase())
}

pub fn read(text: &str) -> Grammar {
    let body: String = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let mut defs: Vec<(String, Vec<Vec<String>>)> = Vec::new();
    // a rule is NAME := alternatives separated by |, ended by a bare ;
    let toks: Vec<&str> = body.split_whitespace().collect();
    for r in toks.split(|t| *t == ";").filter(|r| !r.is_empty()) {
        assert_eq!(r.get(1), Some(&":="), "NAME := alts, at {r:?}");
        let alts = r[2..]
            .split(|t| *t == "|")
            .map(|a| {
                a.iter()
                    .filter(|s| **s != "%empty")
                    .map(|s| s.to_string())
                    .collect()
            })
            .collect();
        assert!(!defs.iter().any(|d| d.0 == r[0]), "{} defined twice", r[0]);
        defs.push((r[0].to_string(), alts));
    }
    let nts: Vec<String> = defs.iter().map(|d| d.0.clone()).collect();
    let mut terms: Vec<String> = Vec::new();
    for (_, alts) in &defs {
        for s in alts.iter().flatten() {
            if terminal(s) {
                if !terms.contains(s) {
                    terms.push(s.clone());
                }
            } else {
                assert!(nts.contains(s), "{s} is used but not defined");
            }
        }
    }
    assert!(terms.len() <= 128 && nts.len() <= 127, "TermSet is a u128");
    let sym = |s: &String| match terms.iter().position(|t| t == s) {
        Some(t) => t,
        None => NT_BASE + nts.iter().position(|n| n == s).expect("defined"),
    };
    let rules: Vec<(usize, Vec<usize>)> = defs
        .iter()
        .enumerate()
        .flat_map(|(a, (_, alts))| alts.iter().map(move |alt| (a, alt)))
        .map(|(a, alt)| (a, alt.iter().map(sym).collect()))
        .collect();
    let n = nts.len();
    let mut g = Grammar {
        nts,
        terms,
        rules,
        nullable: vec![false; n],
        first: vec![BTreeSet::new(); n],
        follow: vec![BTreeSet::new(); n],
    };
    g.follow[0].insert(END);
    loop {
        let mut changed = false;
        for (a, rhs) in &g.rules {
            let (f, null) = first_of(rhs, &g.first, &g.nullable);
            if null && !g.nullable[*a] {
                g.nullable[*a] = true;
                changed = true;
            }
            let before = g.first[*a].len();
            g.first[*a].extend(f);
            changed |= g.first[*a].len() != before;
            for (i, &s) in rhs.iter().enumerate() {
                if s < NT_BASE {
                    continue;
                }
                let (mut f, null) = first_of(&rhs[i + 1..], &g.first, &g.nullable);
                if null {
                    f.extend(g.follow[*a].iter().copied());
                }
                let fw = &mut g.follow[s - NT_BASE];
                let before = fw.len();
                fw.extend(f);
                changed |= fw.len() != before;
            }
        }
        if !changed {
            return g;
        }
    }
}

pub fn first_of(seq: &[usize], first: &[BTreeSet<usize>], nullable: &[bool]) -> (BTreeSet<usize>, bool) {
    let mut out = BTreeSet::new();
    for &s in seq {
        if s < NT_BASE {
            out.insert(s);
            return (out, false);
        }
        out.extend(first[s - NT_BASE].iter().copied());
        if !nullable[s - NT_BASE] {
            return (out, false);
        }
    }
    (out, true)
}

impl Grammar {
    pub fn name(&self, s: usize) -> &str {
        if s < NT_BASE {
            &self.terms[s]
        } else {
            &self.nts[s - NT_BASE]
        }
    }

    /// Every LL(1) conflict, in words.
    pub fn conflicts(&self) -> Vec<String> {
        let mut out = vec![];
        for (a, name) in self.nts.iter().enumerate() {
            let sets: Vec<(BTreeSet<usize>, bool)> = self
                .rules
                .iter()
                .filter(|r| r.0 == a)
                .map(|r| first_of(&r.1, &self.first, &self.nullable))
                .collect();
            let names = |s: Vec<&usize>| s.iter().map(|t| self.terms[**t].clone()).collect::<Vec<_>>();
            for i in 0..sets.len() {
                for j in i + 1..sets.len() {
                    let both: Vec<&usize> = sets[i].0.intersection(&sets[j].0).collect();
                    if !both.is_empty() {
                        out.push(format!(
                            "{name}: alternatives {i} and {j} both start with {:?}",
                            names(both)
                        ));
                    }
                }
                if sets[i].1 {
                    for (j, other) in sets.iter().enumerate().filter(|(j, _)| *j != i) {
                        let both: Vec<&usize> = other.0.intersection(&self.follow[a]).collect();
                        if !both.is_empty() {
                            out.push(format!(
                                "{name}: alternative {j} starts with {:?}, which may follow {name}",
                                names(both)
                            ));
                        }
                    }
                }
            }
            let empties = sets.iter().filter(|s| s.1).count();
            if empties > 1 {
                out.push(format!("{name}: {empties} alternatives can be empty"));
            }
        }
        out
    }

    /// table[nonterminal][terminal]: the rule to expand by, on that lookahead.
    pub fn table(&self) -> Vec<Vec<Option<usize>>> {
        let mut t = vec![vec![None; self.terms.len()]; self.nts.len()];
        for (p, (a, rhs)) in self.rules.iter().enumerate() {
            let (mut f, null) = first_of(rhs, &self.first, &self.nullable);
            if null {
                f.extend(self.follow[*a].iter().copied().filter(|x| *x != END));
            }
            for x in f {
                assert!(
                    t[*a][x].is_none(),
                    "LL(1) conflict at {} on {}",
                    self.nts[*a],
                    self.terms[x]
                );
                t[*a][x] = Some(p);
            }
        }
        t
    }
}

fn camel(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or(String::new(), |f| {
                f.to_ascii_uppercase().to_string() + &c.as_str().to_ascii_lowercase()
            })
        })
        .collect()
}

/// A terminal's name in Rust: NUM, STR, ID, NL and EOF as they are, a keyword in capitals, an
/// operator by what it looks like.
fn term_ident(t: &str) -> String {
    let Some(q) = t.strip_prefix('\'').and_then(|q| q.strip_suffix('\'')) else {
        return t.to_string();
    };
    if q.chars().all(|c| c.is_ascii_alphabetic()) {
        return q.to_ascii_uppercase();
    }
    let name = match q {
        "(" => "LPAREN",
        ")" => "RPAREN",
        "[" => "LBRACK",
        "]" => "RBRACK",
        "{" => "LBRACE",
        "}" => "RBRACE",
        "," => "COMMA",
        ":" => "COLON",
        ";" => "SEMI",
        "." => "DOT",
        ".." => "DOTDOT",
        "..=" => "DOTDOTEQ",
        "=" => "ASSIGN",
        "==" => "EQEQ",
        "!=" => "NE",
        "<" => "LT",
        "<=" => "LE",
        ">" => "GT",
        ">=" => "GE",
        "+" => "PLUS",
        "-" => "MINUS",
        "+-" => "PLUSMINUS",
        "*" => "STAR",
        "/" => "SLASH",
        "^" => "CARET",
        "%" => "PERCENT",
        _ => panic!("give the operator {q} a name in tests/grammar.rs term_ident"),
    };
    name.into()
}

fn bits(s: &BTreeSet<usize>) -> String {
    let v: u128 = s
        .iter()
        .filter(|x| **x != END)
        .map(|x| 1u128 << x)
        .fold(0, |a, b| a | b);
    format!("0x{v:x}")
}

/// src/syntax/table.rs for a grammar.
pub fn generate(g: &Grammar) -> String {
    let table = g.table();
    let mut o = String::new();
    let w = &mut o;
    let (nt, nn, np) = (g.terms.len(), g.nts.len(), g.rules.len());
    assert!(np < 255, "rules are numbered in a u8");
    writeln!(
        w,
        "//! Generated from grammar.ll1 by tests/grammar.rs; do not edit. After a grammar change:"
    )
    .unwrap();
    writeln!(
        w,
        "//! `FG_REGEN=1 cargo test -p forge-lang --test grammar`, then the actions in syntax.rs."
    )
    .unwrap();
    writeln!(
        w,
        "//! Terminals and nonterminals are numbered as research/forge-lang/lean/gen_grammar.py"
    )
    .unwrap();
    writeln!(
        w,
        "//! numbers them, so NULLABLE, FIRST and FOLLOW are the sets the Lean check certifies."
    )
    .unwrap();
    writeln!(w).unwrap();
    writeln!(w, "pub const NT: usize = {nt};").unwrap();
    writeln!(w, "pub const NN: usize = {nn};").unwrap();
    writeln!(w, "pub const NP: usize = {np};").unwrap();
    writeln!(w, "/// No rule: a syntax error on this lookahead.").unwrap();
    writeln!(w, "pub const NONE: u8 = 255;").unwrap();
    writeln!(
        w,
        "/// In a rule's right side, a symbol at or above this is a nonterminal."
    )
    .unwrap();
    writeln!(w, "pub const NT_BASE: u8 = {NT_BASE};").unwrap();
    writeln!(w).unwrap();
    writeln!(w, "pub mod t {{").unwrap();
    for (i, t) in g.terms.iter().enumerate() {
        writeln!(w, "    pub const {}: u8 = {i};", term_ident(t)).unwrap();
    }
    writeln!(w, "}}").unwrap();
    writeln!(w).unwrap();
    writeln!(
        w,
        "/// A terminal as written: the quoted text, or its class name."
    )
    .unwrap();
    writeln!(w, "pub const TERMS: [&str; NT] = [").unwrap();
    for t in &g.terms {
        let s = t
            .strip_prefix('\'')
            .and_then(|q| q.strip_suffix('\''))
            .unwrap_or(t);
        writeln!(w, "    {s:?},").unwrap();
    }
    writeln!(w, "];").unwrap();
    writeln!(w).unwrap();
    writeln!(w, "pub const NONTERMS: [&str; NN] = [").unwrap();
    for n in &g.nts {
        writeln!(w, "    {n:?},").unwrap();
    }
    writeln!(w, "];").unwrap();
    writeln!(w).unwrap();
    let mut pnames = Vec::new();
    for (a, rhs) in &g.rules {
        let first = rhs.first().map_or("Empty".to_string(), |s| {
            if *s < NT_BASE {
                camel(&term_ident(g.name(*s)))
            } else {
                camel(g.name(*s))
            }
        });
        let p = camel(&g.nts[*a]) + &first;
        assert!(!pnames.contains(&p), "two rules named {p}");
        pnames.push(p);
    }
    writeln!(
        w,
        "/// A rule, named by its left side and the first symbol of its right side."
    )
    .unwrap();
    writeln!(w, "#[derive(Clone, Copy, Debug, PartialEq, Eq)]").unwrap();
    writeln!(w, "pub enum P {{").unwrap();
    for (i, (a, rhs)) in g.rules.iter().enumerate() {
        let text: Vec<&str> = rhs.iter().map(|s| g.name(*s)).collect();
        let text = if text.is_empty() {
            "%empty".to_string()
        } else {
            text.join(" ")
        };
        writeln!(w, "    /// {} := {text}", g.nts[*a]).unwrap();
        writeln!(w, "    {},", pnames[i]).unwrap();
    }
    writeln!(w, "}}").unwrap();
    writeln!(w).unwrap();
    writeln!(w, "pub const RULES: [P; NP] = [").unwrap();
    for p in &pnames {
        writeln!(w, "    P::{p},").unwrap();
    }
    writeln!(w, "];").unwrap();
    writeln!(w).unwrap();
    writeln!(
        w,
        "pub const LHS: [u8; NP] = {:?};",
        g.rules.iter().map(|r| r.0).collect::<Vec<_>>()
    )
    .unwrap();
    writeln!(w).unwrap();
    writeln!(w, "pub const RHS: [&[u8]; NP] = [").unwrap();
    for (_, rhs) in &g.rules {
        writeln!(w, "    &{rhs:?},").unwrap();
    }
    writeln!(w, "];").unwrap();
    writeln!(w).unwrap();
    writeln!(w, "pub const NULLABLE: [bool; NN] = {:?};", g.nullable).unwrap();
    writeln!(w).unwrap();
    writeln!(w, "/// Bit k: terminal k.").unwrap();
    writeln!(w, "pub const FIRST: [u128; NN] = [").unwrap();
    for f in &g.first {
        writeln!(w, "    {},", bits(f)).unwrap();
    }
    writeln!(w, "];").unwrap();
    writeln!(w).unwrap();
    writeln!(
        w,
        "/// Bit k: terminal k; FOLLOW_END: the end marker may follow (the start symbol only)."
    )
    .unwrap();
    writeln!(w, "pub const FOLLOW: [u128; NN] = [").unwrap();
    for f in &g.follow {
        writeln!(w, "    {},", bits(f)).unwrap();
    }
    writeln!(w, "];").unwrap();
    let ends: Vec<bool> = g.follow.iter().map(|f| f.contains(&END)).collect();
    writeln!(w, "pub const FOLLOW_END: [bool; NN] = {ends:?};").unwrap();
    writeln!(w).unwrap();
    writeln!(
        w,
        "/// TABLE[nonterminal][lookahead]: the rule to expand by, or NONE."
    )
    .unwrap();
    writeln!(w, "pub const TABLE: [[u8; NT]; NN] = [").unwrap();
    for row in &table {
        let r: Vec<String> = row
            .iter()
            .map(|c| c.map_or("NONE".to_string(), |p| p.to_string()))
            .collect();
        writeln!(w, "    [{}],", r.join(", ")).unwrap();
    }
    writeln!(w, "];").unwrap();
    writeln!(w).unwrap();
    let empty: Vec<String> = (0..nn)
        .map(|a| {
            g.rules
                .iter()
                .position(|r| r.0 == a && first_of(&r.1, &g.first, &g.nullable).1)
                .map_or("NONE".to_string(), |p| p.to_string())
        })
        .collect();
    writeln!(
        w,
        "/// A nullable nonterminal's rule that derives the empty string, or NONE."
    )
    .unwrap();
    writeln!(w, "pub const EMPTY: [u8; NN] = [{}];", empty.join(", ")).unwrap();
    o
}

pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// A random sentence of the grammar, as text: expansions are random until a depth, then the
/// shortest.
pub fn sentence(g: &Grammar, rng: &mut Rng, minlen: &[usize]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut stack: Vec<(usize, usize)> = vec![(NT_BASE, 0)];
    while let Some((s, depth)) = stack.pop() {
        if s < NT_BASE {
            out.push(render(&g.terms[s], rng, out.last().map(String::as_str)));
            continue;
        }
        let a = s - NT_BASE;
        let alts: Vec<&Vec<usize>> = g.rules.iter().filter(|r| r.0 == a).map(|r| &r.1).collect();
        let cost = |rhs: &Vec<usize>| {
            rhs.iter()
                .map(|x| if *x < NT_BASE { 1 } else { minlen[x - NT_BASE] })
                .sum::<usize>()
        };
        let rhs = if depth > 12 || out.len() > 120 {
            *alts.iter().min_by_key(|r| cost(r)).expect("an alternative")
        } else {
            alts[rng.below(alts.len())]
        };
        stack.extend(rhs.iter().rev().map(|x| (*x, depth + 1)));
    }
    out.join(" ")
}

fn render(t: &str, rng: &mut Rng, prev: Option<&str>) -> String {
    match t {
        "NUM" => {
            let lits = [
                "0", "1", "2", "3", "12", "2.5", "1e3", "0.5mm", "90deg", "4mm", "1.25e-2", "7.",
            ];
            let v = lits[rng.below(lits.len())];
            let value_before = prev.is_some_and(|p| {
                p.chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '"' || c == '_')
                    && ![
                        "shared", "param", "fn", "for", "in", "if", "else", "return", "expect", "and", "or",
                        "not", "sketch", "path",
                    ]
                    .contains(&p)
                    || [")", "]", "}"].contains(&p)
            });
            if !value_before && rng.below(5) == 0 {
                format!("-{v}")
            } else {
                v.into()
            }
        }
        "ID" => ["a", "b", "x", "plan", "w_2", "f"][rng.below(6)].into(),
        "STR" => "\"s\"".into(),
        "NL" => "\n".into(),
        "EOF" => String::new(),
        q => q.trim_matches('\'').into(),
    }
}

pub fn min_lengths(g: &Grammar) -> Vec<usize> {
    let mut m = vec![usize::MAX / 4; g.nts.len()];
    loop {
        let mut changed = false;
        for (a, rhs) in &g.rules {
            let c: usize = rhs
                .iter()
                .map(|x| if *x < NT_BASE { 1 } else { m[x - NT_BASE] })
                .sum();
            if c < m[*a] {
                m[*a] = c;
                changed = true;
            }
        }
        if !changed {
            return m;
        }
    }
}

//! The grammar in grammar.ll1 is LL(1): at every choice, the alternatives' first tokens (and,
//! where one can be empty, the tokens that may follow) are disjoint. An LL(1) grammar is
//! unambiguous, so every text has one tree.

use std::collections::{BTreeMap, BTreeSet};

type Rules = BTreeMap<String, Vec<Vec<String>>>;

fn read() -> (Rules, String) {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/grammar.ll1")).expect("grammar");
    let body: String = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let mut rules = Rules::new();
    let mut start = String::new();
    // a rule is NAME := alternatives separated by |, ended by a bare ;
    let toks: Vec<&str> = body.split_whitespace().collect();
    for r in toks.split(|t| *t == ";").filter(|r| !r.is_empty()) {
        assert_eq!(r.get(1), Some(&":="), "NAME := alts, at {r:?}");
        let name = r[0].to_string();
        if start.is_empty() {
            start = name.clone();
        }
        let alts = r[2..]
            .split(|t| *t == "|")
            .map(|a| {
                a.iter()
                    .filter(|s| **s != "%empty")
                    .map(|s| s.to_string())
                    .collect()
            })
            .collect();
        assert!(rules.insert(name.clone(), alts).is_none(), "{name} defined twice");
    }
    (rules, start)
}

fn terminal(s: &str) -> bool {
    s.starts_with('\'') || s.chars().all(|c| c.is_ascii_uppercase())
}

fn first_of(
    seq: &[String],
    first: &BTreeMap<String, BTreeSet<String>>,
    nullable: &BTreeSet<String>,
) -> (BTreeSet<String>, bool) {
    let mut out = BTreeSet::new();
    for s in seq {
        if terminal(s) {
            out.insert(s.clone());
            return (out, false);
        }
        out.extend(first[s].iter().cloned());
        if !nullable.contains(s) {
            return (out, false);
        }
    }
    (out, true)
}

#[test]
fn the_grammar_is_ll1() {
    let (rules, start) = read();
    for alts in rules.values() {
        for a in alts {
            for s in a {
                assert!(
                    terminal(s) || rules.contains_key(s),
                    "{s} is used but not defined"
                );
            }
        }
    }
    let mut nullable = BTreeSet::new();
    let mut first: BTreeMap<String, BTreeSet<String>> =
        rules.keys().map(|k| (k.clone(), BTreeSet::new())).collect();
    let mut follow: BTreeMap<String, BTreeSet<String>> = first.clone();
    follow.get_mut(&start).expect("start").insert("EOF".into());
    loop {
        let mut changed = false;
        for (n, alts) in &rules {
            for a in alts {
                let (f, null) = first_of(a, &first, &nullable);
                let fs = first.get_mut(n).expect("rule");
                let before = fs.len();
                fs.extend(f);
                changed |= fs.len() != before;
                if null && nullable.insert(n.clone()) {
                    changed = true;
                }
            }
        }
        for (n, alts) in &rules {
            for a in alts {
                for (i, s) in a.iter().enumerate() {
                    if terminal(s) {
                        continue;
                    }
                    let (mut f, null) = first_of(&a[i + 1..], &first, &nullable);
                    if null {
                        f.extend(follow[n].iter().cloned());
                    }
                    let fw = follow.get_mut(s).expect("rule");
                    let before = fw.len();
                    fw.extend(f);
                    changed |= fw.len() != before;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let mut conflicts = vec![];
    for (n, alts) in &rules {
        let sets: Vec<(BTreeSet<String>, bool)> =
            alts.iter().map(|a| first_of(a, &first, &nullable)).collect();
        for i in 0..sets.len() {
            for j in i + 1..sets.len() {
                let both: Vec<&String> = sets[i].0.intersection(&sets[j].0).collect();
                if !both.is_empty() {
                    conflicts.push(format!("{n}: alternatives {i} and {j} both start with {both:?}"));
                }
            }
            if sets[i].1 {
                for (j, other) in sets.iter().enumerate().filter(|(j, _)| *j != i) {
                    let both: Vec<&String> = other.0.intersection(&follow[n]).collect();
                    if !both.is_empty() {
                        conflicts.push(format!(
                            "{n}: alternative {j} starts with {both:?}, which may follow {n}"
                        ));
                    }
                }
            }
        }
        let empties = sets.iter().filter(|s| s.1).count();
        if empties > 1 {
            conflicts.push(format!("{n}: {empties} alternatives can be empty"));
        }
    }
    assert!(conflicts.is_empty(), "LL(1) conflicts:\n{}", conflicts.join("\n"));
    assert!(rules.len() > 50);
}

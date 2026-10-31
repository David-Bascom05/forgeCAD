//! cargo run --release -p forge-model --example hunt -- [seeds per family] [--literals]
//! Runs every adversarial family (and suite fuzz), shrinks each failure to a
//! minimal history, dedups by (family, failure kind), and prints a catalogue.
use forge_model::adversarial::{canonical, families, handled, literal, outcome, shrink};
use forge_model::{Part, suite};
use std::collections::BTreeMap;
use std::time::Instant;

type Generator = Box<dyn Fn(u64) -> Part>;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seeds: u64 = args.iter().find_map(|a| a.parse().ok()).unwrap_or(12);
    let show = args.iter().any(|a| a == "--literals");
    let emit = args
        .iter()
        .position(|a| a == "--emit-tests")
        .and_then(|i| args.get(i + 1))
        .cloned();
    std::panic::set_hook(Box::new(|_| {}));
    let t0 = Instant::now();
    let mut gens: Vec<(String, Generator)> = families()
        .into_iter()
        .map(|(n, f)| (n.to_string(), Box::new(f) as Generator))
        .collect();
    gens.push(("fuzz".to_string(), Box::new(|s| suite::fuzz(s + 1, 40))));
    // (family, kind) -> (hits, minimal part)
    let mut found: BTreeMap<(String, String), (usize, Part)> = BTreeMap::new();
    let mut runs = 0;
    let mut ok = 0;
    for (name, g) in &gens {
        for seed in 0..seeds {
            let part = canonical(&g(seed));
            runs += 1;
            let o = outcome(&part);
            let kind = match o.kind() {
                Some(k) if !handled(name, &o) => k,
                _ => {
                    ok += 1;
                    continue;
                }
            };
            let key = (name.clone(), kind);
            match found.get_mut(&key) {
                Some(e) => {
                    e.0 += 1;
                    // keep the smaller starting point for shrinking
                    if part.features.len() < e.1.features.len() {
                        e.1 = part;
                    }
                }
                None => {
                    found.insert(key, (1, part));
                }
            }
        }
    }
    println!(
        "{runs} parts, {ok} handled (valid, or refused where they touch by design), {} distinct (family, failure) pairs, {:.1} s to find",
        found.len(),
        t0.elapsed().as_secs_f64()
    );
    let mut by_kind: BTreeMap<String, Vec<(String, usize, usize)>> = BTreeMap::new();
    for ((fam, kind), (hits, part)) in found.iter_mut() {
        let small = shrink(part);
        *part = small;
        by_kind
            .entry(kind.clone())
            .or_default()
            .push((fam.clone(), *hits, part.features.len()));
    }
    println!(
        "\n{:<70} {:>5}  families (hits/min features)",
        "failure kind", "hits"
    );
    for (kind, fams) in &by_kind {
        let total: usize = fams.iter().map(|f| f.1).sum();
        let list: Vec<String> = fams.iter().map(|(f, h, m)| format!("{f}({h}/{m})")).collect();
        println!(
            "{:<70} {:>5}  {}",
            &kind[..kind.len().min(70)],
            total,
            list.join(" ")
        );
    }
    if show {
        for ((fam, kind), (_, part)) in &found {
            println!(
                "\n// {fam}: {kind} ({} features)\n{}",
                part.features.len(),
                literal(part)
            );
        }
    }
    if let Some(path) = emit {
        let mut out = String::from(
            "//! Minimal histories the adversarial hunt found breaking regeneration\n\
             //! (cargo run --release -p forge-model --example hunt -- 80 --emit-tests ...).\n\
             //! Each is ignored until the kernel handles it; un-ignore as they are fixed.\n\n\
             use forge_math::{Point2, Vec2};\n\
             use forge_model::adversarial::fr;\n\
             use forge_model::{Combine, Feature, Part};\n\
             use forge_ops::{Axis2, Profile, ProfileLoop, Segment};\n\n\
             fn regenerates_valid(part: Part) {\n    \
                 let body = part.regenerate().expect(\"regenerates\");\n    \
                 let problems = forge_topo::check(&body);\n    \
                 assert!(problems.is_empty(), \"{problems:?}\");\n\
             }\n",
        );
        for ((fam, kind), (_, part)) in &found {
            let slug: String = kind
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '_'
                    }
                })
                .collect::<String>()
                .split('_')
                .filter(|w| {
                    !w.is_empty()
                        && *w != "error"
                        && *w != "boolean"
                        && *w != "internal"
                        && *w != "unsupported"
                })
                .take(6)
                .collect::<Vec<_>>()
                .join("_");
            let body = literal(part).replace('\n', "\n    ");
            out.push_str(&format!(
                "\n#[test]\n#[ignore = \"known failure: {}\"]\n#[allow(clippy::approx_constant, clippy::excessive_precision, clippy::unreadable_literal)]\nfn {fam}_{slug}() {{\n    regenerates_valid({body});\n}}\n",
                kind.replace('"', "'")
            ));
        }
        std::fs::write(&path, out).expect("write tests");
        println!("wrote {} regression tests to {path}", found.len());
    }
    println!("\ntotal {:.1} s", t0.elapsed().as_secs_f64());
}

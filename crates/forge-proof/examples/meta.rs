//! The deep tier: every property over fuzz histories, the suite's parts and any part files given,
//! each failure shrunk to a minimal history and written as a part file.
//!
//! cargo run --release -p forge-proof --example meta -- [seeds N] [count N] [only PROP,..]
//!     [out DIR] [known FILE] [FILE.fc..]
//! known: lines `property: kind` already filed; those are counted but not shrunk, and any other
//! failure makes the run fail (the CI job).
use forge_model::{Part, suite};
use forge_proof::props::{self, Finding, Property};
use std::collections::BTreeMap;
use std::time::Instant;

/// A (property, kind)'s hits, its smallest failing part, that part's seed, the finding there
/// and the part's name.
type Hit = (usize, Part, u64, Finding, String);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opt = |k: &str| {
        args.iter()
            .position(|a| a == k)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let seeds: u64 = opt("seeds").and_then(|s| s.parse().ok()).unwrap_or(20);
    let count: usize = opt("count").and_then(|s| s.parse().ok()).unwrap_or(12);
    let only = opt("only");
    let out = opt("out");
    let known: Vec<String> = opt("known")
        .map(|f| std::fs::read_to_string(f).expect("read known failures"))
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    let files: Vec<&String> = args.iter().filter(|a| a.ends_with(".fc")).collect();
    std::panic::set_hook(Box::new(|_| {}));

    let mut parts: Vec<(String, Part)> = (0..seeds)
        .map(|s| (format!("fuzz{s}"), suite::fuzz(s + 1000, count)))
        .collect();
    parts.extend(suite::parts().into_iter().map(|(n, p)| (n.to_string(), p)));
    for f in &files {
        let src = std::fs::read_to_string(f).expect("read part file");
        match forge_doc::text::parse(&src) {
            Ok(p) => parts.push((f.to_string(), p)),
            Err((line, e)) => eprintln!("{f}: line {line}: {e}"),
        }
    }
    let props: Vec<(&str, Property)> = props::all()
        .into_iter()
        .filter(|(n, _)| only.as_ref().is_none_or(|o| o.split(',').any(|x| x == *n)))
        .collect();

    let mut found: BTreeMap<(String, String), Hit> = BTreeMap::new();
    let t0 = Instant::now();
    for (name, f) in &props {
        let t = Instant::now();
        let mut fails = 0;
        for (i, (pname, part)) in parts.iter().enumerate() {
            let seed = i as u64 + 1;
            let Some(hit) = f(part, seed) else { continue };
            fails += 1;
            let key = (name.to_string(), hit.kind.clone());
            match found.get_mut(&key) {
                Some(e) => {
                    e.0 += 1;
                    if part.features.len() < e.1.features.len() {
                        (e.1, e.2, e.3, e.4) = (part.clone(), seed, hit, pname.clone());
                    }
                }
                None => {
                    found.insert(key, (1, part.clone(), seed, hit, pname.clone()));
                }
            }
        }
        println!(
            "{name:<11} {:>4} parts {fails:>3} failing  {:>7.1} s",
            parts.len(),
            t.elapsed().as_secs_f64()
        );
    }
    println!(
        "\n{} distinct (property, kind) in {:.1} s\n",
        found.len(),
        t0.elapsed().as_secs_f64()
    );
    let mut new = 0;
    for ((prop, kind), (hits, part, seed, hit, from)) in &found {
        if known.contains(&format!("{prop}: {kind}")) {
            println!("== {prop}: {kind}  ({hits} hits, known)");
            continue;
        }
        new += 1;
        let f = props.iter().find(|(n, _)| n == prop).expect("a property").1;
        let small = props::shrink(part, f, *seed);
        let again = f(&small, *seed).unwrap_or_else(|| hit.clone());
        let text = forge_doc::text::print(&small);
        let w = props::witness(prop, &small, *seed).map(|w| forge_doc::text::print(&w));
        println!(
            "== {prop}: {kind}  ({hits} hits, first {from}, seed {seed}, {} -> {} features)\n   {}\n{}",
            part.features.len(),
            small.features.len(),
            again.detail,
            text
        );
        if let Some(w) = w {
            println!("-- against\n{w}");
        }
        if let Some(dir) = &out {
            let slug: String = format!("{prop}-{kind}")
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
                .split('-')
                .filter(|w| !w.is_empty())
                .take(6)
                .collect::<Vec<_>>()
                .join("-");
            let path = format!("{dir}/{slug}-s{seed}.fc");
            let head = format!("# {prop} (seed {seed}): {kind}\n# {}\n", again.detail);
            std::fs::write(&path, head + &text).expect("write repro");
            if let Some(w) = props::witness(prop, &small, *seed) {
                let path = format!("{dir}/{slug}-s{seed}.b.fc");
                std::fs::write(&path, forge_doc::text::print(&w)).expect("write repro");
            }
        }
    }
    if !known.is_empty() && new > 0 {
        println!("{new} failures not in the known list");
        std::process::exit(1);
    }
}

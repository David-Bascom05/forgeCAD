//! A suite part or a fuzz part (or a prefix of its history) against the exact curved reference, on
//! both kernel paths. With `locate`, each prefix is regenerated in turn with a `prefix K` line on
//! stderr first, so the decision log (FORGE_DECISIONS_DUMP, feature `decisions`) shows which
//! feature brings a decision in.
//! cargo run --release -p forge-exact-fuzz --example curved_one -- (suite NAME | fuzz SEED)
//!     [prefix K] [quadric] [locate]
use forge_exact_fuzz::Reference;
use forge_exact_fuzz::curved::{curved_part, judge_curved};
use forge_model::{Part, suite};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rule = std::env::var("FORGE_EXACT_RULE").unwrap_or_else(|_| "snap".into());
    let mut part: Part = match args[0].as_str() {
        "suite" => suite::part(&args[1]).expect("a suite part"),
        "fuzz" => suite::fuzz(args[1].parse().expect("a seed"), 80),
        s => panic!("suite NAME or fuzz SEED, not {s}"),
    };
    let at = |w: &str| args.iter().position(|a| a == w);
    if let Some(i) = at("prefix") {
        let k: usize = args[i + 1].parse().expect("prefix K");
        part.features.truncate(k);
    }
    if at("locate").is_some() {
        let all = part.features.clone();
        for k in 1..=all.len() {
            part.features = all[..k].to_vec();
            let d = format!("{:?}", part.features[k - 1]);
            eprintln!("prefix {k} {}", d.chars().take(60).collect::<String>());
            let r = part.regenerate();
            eprintln!("prefix-end {k} {}", if r.is_ok() { "ok" } else { "refused" });
        }
        return;
    }
    if at("quadric").is_some() {
        let n = part.features.len();
        part = curved_part(&part);
        println!("quadric sub-history: {} of {n} features", part.features.len());
    }
    let mut r = Reference::start();
    for (path, class, ans) in judge_curved(&mut r, &part, &rule) {
        let note = ans.and_then(|a| a.notes.first().cloned()).unwrap_or_default();
        println!("{path}: {class:?} {note}");
    }
}

//! cargo run --release -p forge-model --example namehunt -- [cases] [seed] [--big]
//! Random upstream edits of the corpus (inserts, deletes and swaps among them: features keep
//! their ids through those), each reference judged by geometry; prints every finding by
//! class, then the tally.
use forge_model::names::{Class, Edit, corpus, run};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let nums: Vec<u64> = args.iter().filter_map(|a| a.parse().ok()).collect();
    let cases = nums.first().copied().unwrap_or(100) as usize;
    let seed = nums.get(1).copied().unwrap_or(1);
    let big = args.iter().any(|a| a == "--big");
    let corpus = corpus();
    let t0 = std::time::Instant::now();
    let all = run(&corpus, seed, cases, big, if big { 8 } else { 1 }, 60);
    let mut tally: BTreeMap<(String, Class), usize> = BTreeMap::new();
    let (mut kept, mut gone, mut piece, mut unjudged, mut used) = (0, 0, 0, 0, 0);
    for c in &all {
        used += 1;
        kept += c.outcome.kept;
        gone += c.outcome.gone;
        piece += c.outcome.piece;
        unjudged += c.outcome.unjudged;
        let edit = match &c.edit {
            Edit::Insert { at, .. } => format!("Insert at {at}"),
            e => format!("{e:?}"),
        };
        for f in &c.outcome.findings {
            let kind = edit.split([' ', '{']).next().unwrap_or("").to_string();
            *tally.entry((kind, f.class)).or_default() += 1;
            println!(
                "{:?}{} {:?} {:?} {} @{} {edit}: was {:?} now {:?} truth {:?}",
                f.class,
                if c.outcome.reshaped { "" } else { "!" },
                f.kind,
                f.name,
                c.part,
                c.before,
                f.was,
                f.now,
                f.truth
            );
        }
    }
    println!(
        "{used} cases, {kept} kept, {gone} gone, {piece} on a piece, {unjudged} unjudged, {:.1}s",
        t0.elapsed().as_secs_f64()
    );
    for ((k, c), n) in tally {
        println!("  {k:8} {c:?}: {n}");
    }
}

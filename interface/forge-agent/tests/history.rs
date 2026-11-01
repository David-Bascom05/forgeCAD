//! Random whole-kernel histories (`forge_agent::history`): every line builds a valid part or is
//! refused with a clear error, and every finished part meshes closed and regenerates the same
//! n-ary. Failures the kernel still has are listed by kind in history_known.txt; any other kind
//! fails here. Hunt and shrink: cargo run --release -p forge-model --example shrink -- gen 0 200
use forge_agent::history::{MAKERS, SEEDS, run};
use forge_agent::text::KINDS;
use std::collections::BTreeMap;

const LEN: usize = 12;
const RELY_ON_NAME_FALLBACK: &[u64] = &[16];

#[test]
fn every_kind_has_a_maker() {
    for k in KINDS {
        assert!(
            MAKERS.iter().any(|m| m.name == k.name),
            "no history maker for '{}': add a row to forge_agent::history::MAKERS",
            k.name
        );
    }
}

/// Every seed fails only in kinds history_known.txt lists (a line there matches any failure
/// whose kind holds it), and every line there still matches a failure: the list only shrinks.
/// The seeds run on four threads.
#[test]
#[cfg_attr(target_family = "wasm", ignore = "a kernel panic aborts the wasm run")]
fn random_histories() {
    let known: Vec<&str> = include_str!("history_known.txt")
        .lines()
        .map(|l| l.split(" #").next().unwrap_or("").trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let runs: Vec<(u64, forge_agent::history::Run)> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..4)
            .map(|_| {
                s.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(&seed) = SEEDS.get(i) else { break };
                        let t = std::time::Instant::now();
                        let r = run(seed, LEN);
                        eprintln!(
                            "seed {seed}: {:.1} s, {} lines",
                            t.elapsed().as_secs_f64(),
                            r.src.lines().count()
                        );
                        out.push((seed, r));
                    }
                    out
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().expect("history worker"))
            .collect()
    });
    let mut kinds: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut new = Vec::new();
    let mut seen = vec![false; known.len()];
    for (seed, r) in &runs {
        for (k, why) in &r.tried {
            let e = kinds.entry(k).or_default();
            e.0 += 1;
            e.1 += why.is_none() as usize;
        }
        for (what, src) in &r.fails {
            let mut hit = false;
            for (i, k) in known.iter().enumerate() {
                if what.contains(k) {
                    seen[i] = true;
                    hit = true;
                }
            }
            if !hit {
                new.push(format!("seed {seed}: {what}\n{src}"));
            }
        }
    }
    eprintln!("kind (tried, built): {kinds:?}");
    assert!(
        new.is_empty(),
        "{} failures of a kind not in history_known.txt (save one as F.fc and shrink it: \
         cargo run --release -p forge-model --example shrink -- F.fc):\n{}",
        new.len(),
        new.join("\n")
    );
    // seeds whose finished part the n-ary regen builds only through the sequential one (a later
    // feature's reference named as only that one names it, docs/naming.md): the list only shrinks
    let relying: Vec<u64> = runs
        .iter()
        .filter(|(_, r)| r.name_fallbacks > 0)
        .map(|(s, _)| *s)
        .collect();
    let mut relying = relying;
    relying.sort_unstable();
    assert_eq!(
        relying, RELY_ON_NAME_FALLBACK,
        "seeds relying on the n-ary name fallback"
    );
    let gone: Vec<&str> = known
        .iter()
        .zip(&seen)
        .filter(|(_, s)| !**s)
        .map(|(k, _)| *k)
        .collect();
    assert!(
        gone.is_empty(),
        "no seed fails as these any more: remove them from history_known.txt\n{}",
        gone.join("\n")
    );
}

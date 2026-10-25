//! Regenerates every benchmark part and a batch of random histories, reporting
//! time, size and validity, and where each failure first appears.
use forge_model::{Part, RegenError, suite};
use std::time::Instant;

/// The first feature count at which regeneration fails (prefixes before it succeed).
fn first_failure(part: &Part) -> (usize, RegenError) {
    let n = part.features.len();
    let prefix = |k: usize| {
        Part {
            features: part.features[..k].to_vec(),
        }
        .regenerate()
    };
    let (mut lo, mut hi) = (1, n);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if prefix(mid).is_ok() {
            lo = mid + 1
        } else {
            hi = mid
        }
    }
    (lo - 1, prefix(lo).expect_err("the full part fails"))
}

fn main() {
    println!(
        "{:<10} {:>8} {:>6} {:>10} {:>10} {:>8}  result",
        "part", "features", "faces", "ms", "ms/feat", "problems"
    );
    for (name, part) in suite::parts() {
        let n = part.features.len();
        let mut best = f64::INFINITY;
        let mut out = None;
        for _ in 0..3 {
            let t = Instant::now();
            let r = part.regenerate();
            best = best.min(t.elapsed().as_secs_f64() * 1e3);
            out = Some(r);
        }
        match out.expect("ran") {
            Ok(b) => {
                let problems = forge_topo::check(&b).len();
                let verdict = if problems == 0 { "ok" } else { "INVALID" };
                println!(
                    "{name:<10} {n:>8} {:>6} {best:>10.2} {:>10.3} {problems:>8}  {verdict}",
                    b.faces.len(),
                    best / n as f64
                );
            }
            Err(_) => {
                let (i, e) = first_failure(&part);
                println!(
                    "{name:<10} {n:>8} {:>6} {:>10} {:>10} {:>8}  FAILS at feature {i}: {e:?}",
                    "-", "-", "-", "-"
                );
            }
        }
    }
    // FUZZ_SEEDS widens the run (default 20)
    let seeds: u64 = std::env::var("FUZZ_SEEDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let count = 80usize;
    let mut ok = 0;
    let mut failures = Vec::new();
    let t = Instant::now();
    for seed in 1..=seeds {
        let part = suite::fuzz(seed, count);
        match part.regenerate() {
            Ok(b) if forge_topo::check(&b).is_empty() => ok += 1,
            Ok(b) => failures.push(format!(
                "seed {seed}: invalid result, {} problems",
                forge_topo::check(&b).len()
            )),
            Err(_) => {
                let (i, e) = first_failure(&part);
                failures.push(format!("seed {seed}: fails at feature {i}: {e:?}"));
            }
        }
    }
    println!(
        "fuzz: {ok}/{seeds} histories of {count} features regenerate valid ({:.0} ms)",
        t.elapsed().as_secs_f64() * 1e3
    );
    for f in failures {
        println!("  {f}");
    }
}

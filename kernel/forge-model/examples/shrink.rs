//! cargo run --release -p forge-model --example shrink -- <suite part | fuzz seed | adversarial family>
//! Shrinks a failing history to a minimal one and prints it as a literal.
//!   shrink gen FROM TO [LEN]   whole-kernel random histories (forge_agent::gen), seeds FROM..TO:
//!                              what each kind did, and each kind of failure shrunk to a part file
//!                              (NOSHRINK=1: as found)
//!   shrink FILE.fc             a part file's failure, shrunk
use forge_agent::history;
use forge_geom::CurveGeom;
use forge_model::{adversarial, suite};
use std::collections::BTreeMap;

/// Hunts seeds `from..to` of the history generator and prints each failure kind's smallest repro.
fn hunt(from: u64, to: u64, len: usize) {
    if std::env::var_os("LOUD").is_none() {
        std::panic::set_hook(Box::new(|_| {}));
    }
    let t0 = std::time::Instant::now();
    // per kind: tried, built, refusals by reason
    let mut kinds: BTreeMap<&str, (usize, usize, BTreeMap<String, usize>)> = BTreeMap::new();
    let mut fails: BTreeMap<String, (usize, u64, String)> = BTreeMap::new();
    let mut lines = 0;
    for seed in from..to {
        let t = std::time::Instant::now();
        let run = history::run(seed, len);
        eprintln!("seed {seed}: {:.1} s", t.elapsed().as_secs_f64());
        if run.slowest.0 > 5.0 {
            eprintln!(
                "seed {seed}: one line took {:.1} s:\n{}",
                run.slowest.0, run.slowest.1
            );
        }
        lines += run.src.lines().count();
        for (k, why) in &run.tried {
            let e = kinds.entry(k).or_default();
            e.0 += 1;
            match why {
                None => e.1 += 1,
                Some(w) => *e.2.entry(w.clone()).or_default() += 1,
            }
        }
        for (what, src) in run.fails {
            eprintln!("seed {seed}: {what}");
            let e = fails.entry(what).or_insert((0, seed, src.clone()));
            e.0 += 1;
            if src.lines().count() < e.2.lines().count() {
                (e.1, e.2) = (seed, src);
            }
        }
    }
    let n = to - from;
    println!(
        "{n} histories, {lines} lines built, {} failing, {:.1} s",
        fails.values().map(|f| f.0).sum::<usize>(),
        t0.elapsed().as_secs_f64()
    );
    println!(
        "\n{:<9} {:>6} {:>6}  refusals (most common)",
        "kind", "tried", "built"
    );
    for (k, (t, b, why)) in &kinds {
        let mut w: Vec<_> = why.iter().collect();
        w.sort_by(|a, b| b.1.cmp(a.1));
        let top: Vec<String> = w.iter().take(4).map(|(m, c)| format!("{c}x {m}")).collect();
        println!("{k:<9} {t:>6} {b:>6}  {}", top.join(" | "));
    }
    // NOSHRINK: only the table
    let shrink = std::env::var_os("NOSHRINK").is_none();
    for (what, (hits, seed, src)) in &fails {
        let small = if shrink {
            history::shrink(src, &history::Verdict::Fail(what.clone()))
        } else {
            src.clone()
        };
        println!(
            "\n# {what}  ({hits} seeds, first {seed}; {} of {} lines)\n{small}",
            small.lines().count(),
            src.lines().count()
        );
    }
}

fn main() {
    let arg = std::env::args().nth(1).expect("suite part name or fuzz seed");
    let num = |k: usize, d: u64| std::env::args().nth(k).and_then(|a| a.parse().ok()).unwrap_or(d);
    if arg == "gen" {
        return hunt(num(2, 0), num(3, 50), num(4, 30) as usize);
    }
    if arg.ends_with(".fc") {
        let src = std::fs::read_to_string(&arg).expect("a part file");
        let v = history::judge(&src);
        println!("# {v:?}");
        if matches!(v, history::Verdict::Fail(_)) {
            print!("{}", history::shrink(&src, &v));
        }
        return;
    }
    // a fuzz seed, a suite part, or an adversarial family (its first failing seed)
    let part = match arg.parse::<u64>() {
        Ok(seed) => suite::fuzz(seed, 80),
        Err(_) => match suite::parts().into_iter().find(|(n, _)| *n == arg) {
            Some((_, p)) => p,
            None => {
                let (_, family) = adversarial::families()
                    .into_iter()
                    .find(|(n, _)| *n == arg)
                    .expect("no such part or family");
                (0..200)
                    .map(|s| adversarial::canonical(&family(s)))
                    .find(|p| adversarial::outcome(p) != adversarial::Outcome::Ok)
                    .expect("no failing seed")
            }
        },
    };
    let small = adversarial::shrink(&part);
    println!("{:?}", adversarial::outcome(&small));
    println!("{}", adversarial::literal(&small));
    // with FORGE_DEBUG set, the diagnostics after this line are the minimal part's
    eprintln!("=== rerun");
    match small.regenerate() {
        Ok(b) => {
            let problems = forge_topo::check(&b);
            println!("{problems:?}");
            for p in &problems {
                if let forge_topo::Problem::VertexOffEdge { edge, .. } = p {
                    let e = b.edge(*edge);
                    let kind = format!("{:?}", e.curve);
                    println!(
                        "edge {edge:?}: {} range {:?} start {:?} -> {:?} end {:?} -> {:?}",
                        &kind[..kind.len().min(300)],
                        e.range,
                        b.vertex(e.start).point,
                        e.curve.eval(e.range.lo),
                        b.vertex(e.end).point,
                        e.curve.eval(e.range.hi)
                    );
                }
            }
        }
        Err(e) => println!("{e:?}"),
    }
}

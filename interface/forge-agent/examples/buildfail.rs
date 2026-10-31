//! The census of what will not build (research/build-failures): random histories over every
//! feature kind, each line refused or failed counted by kind of message, the smallest history
//! of each kind shrunk and written as a part file.
//! buildfail FIRST_SEED COUNT [LEN] [OUT_DIR] [all]
//! buildfail shrink FILE: a part file's refusal or failure, shrunk
//! all: every refused history up to 30 of a kind as found, not shrunk (for the OCCT oracle's dir: set)
use forge_agent::history::{MAKERS, judge, run, shrink};
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
struct Kind {
    hits: usize,
    failure: bool,
    smallest: String,
    every: Vec<String>,
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.first().is_some_and(|s| s == "shrink") {
        let src = std::fs::read_to_string(&a[1]).expect("a part file");
        let v = judge(&src);
        println!("# {v:?}\n{}", shrink(&src, &v));
        return;
    }
    let first: u64 = a.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let count: u64 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(100);
    let len: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(12);
    let out = a.get(3);
    let all = a.get(4).is_some_and(|s| s == "all");
    std::panic::set_hook(Box::new(|_| {}));
    let next = AtomicU64::new(first);
    let kinds: Mutex<BTreeMap<String, Kind>> = Mutex::default();
    let tally: Mutex<BTreeMap<&'static str, (usize, usize)>> = Mutex::default();
    std::thread::scope(|s| {
        for _ in 0..3 {
            s.spawn(|| {
                loop {
                    let seed = next.fetch_add(1, Ordering::Relaxed);
                    if seed >= first + count {
                        break;
                    }
                    let r = run(seed, len);
                    let mut t = tally.lock().expect("tally");
                    for (kind, why) in &r.tried {
                        let e = t.entry(kind).or_default();
                        e.0 += 1;
                        e.1 += why.is_some() as usize;
                    }
                    drop(t);
                    let mut k = kinds.lock().expect("kinds");
                    let lists = [(&r.refused, false), (&r.fails, true)];
                    for (list, failure) in lists {
                        for (why, src) in list {
                            let e = k.entry(why.clone()).or_default();
                            e.hits += 1;
                            e.failure = failure;
                            if e.every.len() < 30 {
                                e.every.push(src.clone());
                            }
                            if e.smallest.is_empty() || src.len() < e.smallest.len() {
                                e.smallest = src.clone();
                            }
                        }
                    }
                }
            });
        }
    });
    let tally = tally.into_inner().expect("tally");
    println!("{:<12} {:>6} {:>8} {:>7}", "kind", "tried", "refused", "%");
    for m in MAKERS {
        let (t, r) = tally.get(m.name).copied().unwrap_or_default();
        println!(
            "{:<12} {t:>6} {r:>8} {:>6.1}%",
            m.name,
            100.0 * r as f64 / t.max(1) as f64
        );
    }
    let kinds = kinds.into_inner().expect("kinds");
    let mut rows: Vec<_> = kinds.iter().collect();
    rows.sort_by_key(|(_, k)| std::cmp::Reverse(k.hits));
    println!();
    for (i, (why, k)) in rows.iter().enumerate() {
        println!("{:>5} {} {why}", k.hits, if k.failure { "FAIL" } else { "ref " });
        if let (Some(dir), true) = (out, all) {
            for (n, src) in k.every.iter().enumerate() {
                std::fs::write(format!("{dir}/k{i:03}-{n:02}.fc"), format!("# {why}\n{src}")).expect("write");
            }
        } else if let Some(dir) = out {
            let small = shrink(&k.smallest, &judge(&k.smallest));
            std::fs::write(format!("{dir}/k{i:03}.fc"), format!("# {why}\n{small}")).expect("write");
        }
    }
}

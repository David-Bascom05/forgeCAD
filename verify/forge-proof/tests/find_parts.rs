//! The CI tier of tools/find-parts: a few dozen parts picked for the kernel branches they reach per
//! CPU-second, from the generators (standard parts, feature pairs, surface-pair configurations,
//! sweeps of the bench parts). No part may do worse than ci-corpus.txt records; regenerate it with
//! tools/find-parts/ci_pick.py. A part that now does better is printed, for the next regeneration.
use forge_agent::history::{Verdict, judge};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
#[cfg_attr(target_family = "wasm", ignore = "a kernel panic aborts the wasm run")]
fn no_part_does_worse() {
    let corpus = include_str!("../../../tools/find-parts/ci-corpus.txt");
    let parts: Vec<(&str, &str, &str)> = corpus
        .split("### ")
        .skip(1)
        .map(|c| {
            let (head, src) = c.split_once('\n').expect("a record's head");
            let (name, want) = head.split_once(" | ").expect("name | verdict");
            (name, want, src)
        })
        .collect();
    assert!(parts.len() >= 30, "{} parts", parts.len());
    std::panic::set_hook(Box::new(|_| {}));
    let next = AtomicUsize::new(0);
    let worse: Vec<String> = std::thread::scope(|s| {
        let threads = std::thread::available_parallelism()
            .map_or(2, |n| n.get())
            .clamp(2, 4);
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut out = Vec::new();
                    while let Some(&(name, want, src)) = parts.get(next.fetch_add(1, Ordering::Relaxed)) {
                        let got = judge(src);
                        let bad = match (&got, want) {
                            (Verdict::Ok, "ok") => false,
                            (Verdict::Ok, _) => {
                                eprintln!("better: {name} now builds");
                                false
                            }
                            (_, "ok") => true,
                            (Verdict::Refused(_), "refused") => false,
                            (Verdict::Refused(_), _) => {
                                eprintln!("better: {name} now refuses clearly");
                                false
                            }
                            (Verdict::Fail(_), "refused") => true,
                            // a recorded failure stays recorded until the kernel bug is fixed
                            (Verdict::Fail(_), _) => false,
                        };
                        if bad {
                            out.push(format!("{name}: was {want}, now {got:?}\n{src}"));
                        }
                    }
                    out
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().expect("worker"))
            .collect()
    });
    let _ = std::panic::take_hook();
    assert!(
        worse.is_empty(),
        "{} parts do worse:\n{}",
        worse.len(),
        worse.join("\n")
    );
}

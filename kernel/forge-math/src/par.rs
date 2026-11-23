//! Independent work on all cores: items taken one at a time (their costs differ), answers in
//! item order, so a result never depends on which thread made it.

/// The scoped threads every parallel path of the kernel runs on: std's, or a host's (`forge_par`).
pub use forge_par::{Scope, ScopedJoinHandle, scope, spawner};

/// The threads to use: a host's spawner's count where one is installed (one on a thread that may
/// not wait); else `FORGECAD_THREADS` if set, else the cores there are (one on wasm).
pub fn threads() -> usize {
    if let Some(sp) = forge_par::spawner() {
        return if (sp.can_block)() {
            (sp.threads)().max(1)
        } else {
            1
        };
    }
    if cfg!(target_family = "wasm") {
        return 1;
    }
    std::env::var("FORGECAD_THREADS")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
        .max(1)
}

/// `requested` threads, but no more than [`threads`] gives (the cores there are, or
/// `FORGECAD_THREADS`), and at least one. Asking for more threads than cores only adds the cost
/// of starting and switching them: 16 on 4 cores made the n-ary regen of a small part 66% slower
/// (research/booleans/speedups.md, "Threads").
pub fn cap(requested: usize) -> usize {
    requested.min(threads()).max(1)
}

/// How long the calling thread works alone before [`map_adaptive`] starts helpers: past this,
/// the work left is likely to pay for starting them (tens of microseconds each).
pub const HELPERS_AFTER: std::time::Duration = std::time::Duration::from_micros(200);

/// `f` of each of `0..n`, in order, on up to `threads` threads, each given at least `per` items.
/// The calling thread starts alone, and the others join only once it has worked for
/// [`HELPERS_AFTER`] with items left, so small batches never pay for threads. The answers are
/// collected by item index: they are the same, and in the same order, whatever the thread count
/// or the scheduling (`par_map_eq`, research/booleans/lean/Speedups.lean).
pub fn map_adaptive<T: Send>(n: usize, threads: usize, per: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let threads = threads.min(n / per.max(1));
    if threads <= 1 || n < 2 {
        return (0..n).map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let work = || {
        let mut got = Vec::new();
        loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            if i >= n {
                break got;
            }
            got.push((i, f(i)));
        }
    };
    let parts: Vec<Vec<(usize, T)>> = std::thread::scope(|s| {
        let start = std::time::Instant::now();
        let mut mine = Vec::new();
        let mut helpers = Vec::new();
        loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            if i >= n {
                break;
            }
            mine.push((i, f(i)));
            if helpers.is_empty() && next.load(Ordering::Relaxed) < n && start.elapsed() > HELPERS_AFTER {
                helpers = (1..threads).map(|_| s.spawn(work)).collect();
            }
        }
        let mut parts = vec![mine];
        parts.extend(
            helpers
                .into_iter()
                .map(|w| w.join().unwrap_or_else(|p| std::panic::resume_unwind(p))),
        );
        parts
    });
    let mut out: Vec<Option<T>> = (0..n).map(|_| None).collect();
    for (i, t) in parts.into_iter().flatten() {
        out[i] = Some(t);
    }
    out.into_iter().map(|t| t.expect("every item done")).collect()
}

/// `f` of each of `0..n`, in order.
pub fn map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let threads = threads().min(n);
    if threads <= 1 {
        return (0..n).map(f).collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut got: Vec<(usize, T)> = scope(|s| {
        let hs: Vec<_> = (0..threads)
            .map(|_| {
                let (f, next) = (&f, &next);
                s.spawn(move || {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if i >= n {
                            return out;
                        }
                        out.push((i, f(i)));
                    }
                })
            })
            .collect();
        hs.into_iter()
            .flat_map(|h| h.join().unwrap_or_else(|p| std::panic::resume_unwind(p)))
            .collect()
    });
    got.sort_unstable_by_key(|x| x.0);
    got.into_iter().map(|x| x.1).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn answers_come_in_item_order() {
        let v = super::map(100, |i| i * i);
        assert_eq!(v, (0..100).map(|i| i * i).collect::<Vec<_>>());
    }
}

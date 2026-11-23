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
        .unwrap_or_else(cores)
        .max(1)
}

/// The cores there are, asked once: `available_parallelism` reads the cgroup's files on Linux
/// (several system calls), and `threads` is asked at every parallel map.
fn cores() -> usize {
    static CORES: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *CORES.get_or_init(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
}

/// `requested` threads, but no more than [`threads`] gives (the cores there are, or
/// `FORGECAD_THREADS`), and at least one. Asking for more threads than cores only adds the cost
/// of starting and switching them: 16 on 4 cores made the n-ary regen of a small part 66% slower
/// (research/booleans/speedups.md, "Threads").
pub fn cap(requested: usize) -> usize {
    requested.min(threads()).max(1)
}

/// How long the calling thread works alone before [`map_adaptive`] considers helpers.
pub const HELPERS_AFTER: std::time::Duration = std::time::Duration::from_micros(200);

/// The serial work still left, projected from the items done so far, below which
/// [`map_adaptive`] never starts helpers. Starting and joining 3 helpers costs about 0.1–0.2 ms
/// on a 4-core VM, so break-even is about 0.3 ms of work left. Measured across the bench parts,
/// 1 ms still lost on some (hole-grid n-ary: helpers whose items contend), 4 ms gave up wins;
/// 2 ms kept every win and no loss beyond noise (research/booleans/speedups.md, "Fastest wins").
pub const HELPERS_FOR: std::time::Duration = std::time::Duration::from_millis(2);

/// `f` of each of `0..n`, in order, on up to `threads` threads, each given at least `per` items.
/// The calling thread starts alone. The others join only once it has worked for
/// [`HELPERS_AFTER`] and the work left, projected from the items done, exceeds [`HELPERS_FOR`],
/// so batches too small to pay for threads stay on one. The answers are
/// collected by item index: they are the same, and in the same order, whatever the thread count
/// or the scheduling (`par_map_eq`, research/booleans/lean/Speedups.lean).
pub fn map_adaptive<T: Send>(n: usize, threads: usize, per: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    map_adaptive_for(n, threads, per, HELPERS_FOR, f)
}

/// [`map_adaptive`] with its own threshold of work left, `helpers_for`, for items whose helpers
/// pay more than their start: items that fill a per-thread cache (a new thread starts it cold,
/// and recomputes what the caller already has).
pub fn map_adaptive_for<T: Send>(
    n: usize,
    threads: usize,
    per: usize,
    helpers_for: std::time::Duration,
    f: impl Fn(usize) -> T + Sync,
) -> Vec<T> {
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
    scope(|s| {
        let start = crate::clock::Instant::now();
        // alone, this thread takes the items in order: its answers go straight into place
        let mut first: Vec<T> = Vec::with_capacity(n);
        let mut helpers = Vec::new();
        while helpers.is_empty() {
            let i = next.fetch_add(1, Ordering::Relaxed);
            if i >= n {
                return first;
            }
            first.push(f(i));
            // asked after every item: items differ widely in cost, so the projection can grow
            let taken = i + 1;
            let elapsed = start.elapsed();
            if taken < n && elapsed > HELPERS_AFTER {
                // the work left, at the pace so far
                let left = elapsed.mul_f64((n - taken) as f64 / taken as f64);
                if left > helpers_for {
                    // one helper per `helpers_for` of work left: a new thread costs more than its
                    // start (its own allocator arena, cold pages and caches), so a few ms of
                    // work pays for one or two, not for every core
                    let want = (left.as_secs_f64() / helpers_for.as_secs_f64()) as usize;
                    let k = want.min(threads - 1).min(n - taken).max(1);
                    helpers = (0..k).map(|_| s.spawn(work)).collect();
                }
            }
        }
        // with helpers: the rest by index
        let done = first.len();
        let mut rest: Vec<Option<T>> = (done..n).map(|_| None).collect();
        for (i, t) in work() {
            rest[i - done] = Some(t);
        }
        for h in helpers {
            for (i, t) in h.join().unwrap_or_else(|p| std::panic::resume_unwind(p)) {
                rest[i - done] = Some(t);
            }
        }
        first.extend(rest.into_iter().map(|t| t.expect("every item done")));
        first
    })
}

/// `f` of each of `0..n`, in order, on up to [`threads`] threads, which join only where the
/// work pays for them ([`map_adaptive`]).
pub fn map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    map_adaptive(n, threads(), 1, f)
}

#[cfg(test)]
mod tests {
    #[test]
    fn answers_come_in_item_order() {
        let v = super::map(100, |i| i * i);
        assert_eq!(v, (0..100).map(|i| i * i).collect::<Vec<_>>());
    }
}

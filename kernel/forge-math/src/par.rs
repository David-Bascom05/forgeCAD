//! Independent work on all cores: items taken one at a time (their costs differ), answers in
//! item order, so a result never depends on which thread made it.

/// The threads to use: `FORGECAD_THREADS` if set, else the cores there are.
pub fn threads() -> usize {
    if cfg!(target_family = "wasm") {
        return 1;
    }
    std::env::var("FORGECAD_THREADS")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
        .max(1)
}

/// `f` of each of `0..n`, in order.
pub fn map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let threads = threads().min(n);
    if threads <= 1 {
        return (0..n).map(f).collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut got: Vec<(usize, T)> = std::thread::scope(|s| {
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

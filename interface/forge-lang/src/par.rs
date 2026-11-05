//! Independent work on all cores (the kernel's `par`), answers in item order; what each item
//! reads is recorded for the caller (`deps`), as if it had run here.

pub use forge_math::par::threads;

/// `f` of each of `0..n`, in order.
pub fn map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    if !crate::deps::watching() {
        return forge_math::par::map(n, f);
    }
    let got = forge_math::par::map(n, |i| {
        let w = crate::deps::watch();
        let t = f(i);
        (t, w.done())
    });
    got.into_iter()
        .map(|(t, d)| {
            crate::deps::merge(&d);
            t
        })
        .collect()
}

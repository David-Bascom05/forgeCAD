//! Independent work on all cores (the kernel's `par`), answers in item order; what each item
//! reads is recorded for the caller (`deps`), as if it had run here.

pub use forge_math::par::threads;

/// `f` of each of `0..n`, in order.
pub fn map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let (watching, libs) = (crate::deps::watching(), crate::deps::recording_libraries());
    if !watching && !libs {
        return forge_math::par::map(n, f);
    }
    let got = forge_math::par::map(n, |i| {
        let w = watching.then(crate::deps::watch);
        let l = libs.then(crate::deps::libraries);
        let t = f(i);
        (t, w.map(|w| w.done()), l.map(|l| l.done()))
    });
    got.into_iter()
        .map(|(t, d, l)| {
            if let Some(d) = d {
                crate::deps::merge(&d);
            }
            if let Some(l) = l {
                crate::deps::libraries_used(&l);
            }
            t
        })
        .collect()
}

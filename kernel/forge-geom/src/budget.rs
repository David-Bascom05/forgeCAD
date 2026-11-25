//! How much work the reading of one thing from a file may take: a count of evaluations of a file's
//! spline surfaces and of steps round a mesh's largest fans, the costs nothing else bounds. The same on every machine, so a file is
//! refused or not alike everywhere. Off unless a `scoped` budget lives, and nothing made here counts.

use std::cell::Cell;

thread_local! {
    static LEFT: Cell<u64> = const { Cell::new(u64::MAX) };
    static LIMITED: Cell<bool> = const { Cell::new(false) };
}

/// What the reading of one solid from a file may spend, where its caller sets no other (the import's
/// `Limits::max_evals`, measured in research/step/README.md).
pub const FILE: u64 = 1 << 25;

/// From now on, `most` evaluations: only through `scoped`, whose drop ends it.
fn begin(most: u64) {
    LEFT.set(most);
    LIMITED.set(true);
}

/// No limit again.
fn end() {
    LEFT.set(u64::MAX);
    LIMITED.set(false);
}

/// Whether a `scoped` budget lives on this thread. (Not `left() == u64::MAX`: what is spent with
/// no limit set still counts down from there.)
pub fn limited() -> bool {
    LIMITED.get()
}

/// `most` evaluations while this lives, no limit again when it goes, unwinding too: a read that
/// panics leaves no limit behind for what runs next on its thread.
pub struct Budget(());

#[must_use = "the budget ends when this is dropped"]
pub fn scoped(most: u64) -> Budget {
    begin(most);
    Budget(())
}

impl Drop for Budget {
    fn drop(&mut self) {
        end();
    }
}

#[inline]
pub fn spend() {
    LEFT.set(LEFT.get().saturating_sub(1));
}

pub fn spend_many(n: u64) {
    LEFT.set(LEFT.get().saturating_sub(n));
}

/// Whether what `scoped` allowed is gone: the caller stops and says so.
pub fn over() -> bool {
    LEFT.get() == 0
}

/// How many are left (`u64::MAX` less what was spent, with no limit set).
pub fn left() -> u64 {
    LEFT.get()
}

/// `f` of each of `0..n`, as one after another on this thread under its budget would give them,
/// worked out on all cores (`forge_math::par::map`) as far as that is the same.
///
/// Each item runs under a budget of its own: what was left, less what the items done so far
/// spent. An item that ran its own out is not taken. Then, in item order, the answers are taken
/// while the items' spends summed in order stay below what was left: such an item, one by one,
/// would have started with more than it spent, as it had here, so no `over` was true in either
/// run (and nothing else of a budget changes what an item computes): the same answer. The items
/// from the first not taken on are run one by one on this thread, as before, and this thread is
/// charged what the taken ones spent. With no budget set, the items just run on all cores.
pub fn map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    map_until(n, f, |_| false)
}

/// [`map`] for a caller that, one by one, stops after the first answer `stop` holds of: the
/// answers up to and including it (the others are not wanted), and the budget charged what the
/// items up to it spent, as one by one. (With no budget set, every item runs.)
pub fn map_until<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync, stop: impl Fn(&T) -> bool) -> Vec<T> {
    use std::sync::atomic::{AtomicU64, Ordering};
    if !limited() {
        let mut all = forge_math::par::map(n, f);
        if let Some(k) = all.iter().position(&stop) {
            all.truncate(k + 1);
        }
        return all;
    }
    let left = left();
    let done = AtomicU64::new(0);
    let got = forge_math::par::map(n, |k| {
        let own = left.saturating_sub(done.load(Ordering::Relaxed));
        if own == 0 {
            return None;
        }
        // this thread's own budget put back after, unwinding too (the caller's thread runs items)
        struct Put(u64, bool);
        impl Drop for Put {
            fn drop(&mut self) {
                LEFT.set(self.0);
                LIMITED.set(self.1);
            }
        }
        let _put = Put(LEFT.get(), LIMITED.get());
        LEFT.set(own);
        LIMITED.set(true);
        let t = f(k);
        let rest = LEFT.get();
        let spent = own - rest;
        done.fetch_add(spent, Ordering::Relaxed);
        (rest > 0).then_some((t, spent))
    });
    let mut out = Vec::with_capacity(n);
    let mut spent = 0u64;
    for g in got {
        match g {
            Some((t, s)) if spent + s < left => {
                spent += s;
                let last = stop(&t);
                out.push(t);
                if last {
                    spend_many(spent);
                    return out;
                }
            }
            _ => break,
        }
    }
    spend_many(spent);
    for k in out.len()..n {
        let t = f(k);
        let last = stop(&t);
        out.push(t);
        if last {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    // catches the panic it causes: under WASI a panic aborts (no unwinding), so native only
    #[cfg(not(target_os = "wasi"))]
    #[test]
    fn a_read_that_panics_leaves_no_limit_behind() {
        let r = std::panic::catch_unwind(|| {
            let _b = super::scoped(3);
            super::spend_many(5);
            panic!("a read went wrong");
        });
        assert!(r.is_err());
        assert_eq!(super::left(), u64::MAX);
        assert!(!super::limited());
    }

    /// Items that spend less than is left together: their answers, the budget charged their sum,
    /// as one after another; more: the answers one by one would give, the budget run out where
    /// one by one runs it out.
    #[test]
    fn map_answers_as_one_by_one() {
        let one = |k: usize| {
            super::spend_many(k as u64);
            (k, super::over())
        };
        let b = super::scoped(100);
        let got = super::map(10, one);
        assert_eq!(got, (0..10).map(|k| (k, false)).collect::<Vec<_>>());
        assert_eq!(super::left(), 55);
        // 0 + 1 + .. + 9 = 45 < 55; item 10 runs it out
        let got = super::map(12, one);
        let want: Vec<_> = (0..12).map(|k| (k, k >= 10)).collect();
        assert_eq!(got, want);
        assert_eq!(super::left(), 0);
        assert!(super::limited());
        drop(b);
        // stopped at the first answer asked for: charged up to it, as one by one
        let b = super::scoped(100);
        let got = super::map_until(10, one, |&(k, _)| k == 4);
        assert_eq!(got, (0..5).map(|k| (k, false)).collect::<Vec<_>>());
        assert_eq!(super::left(), 90);
        let got = super::map_until(30, one, |&(k, _)| k == 20);
        // 0..=12 spend 78 of the 90; 13 runs it out, and every one after finds it out
        assert_eq!(got, (0..21).map(|k| (k, k >= 13)).collect::<Vec<_>>());
        drop(b);
        assert!(!super::limited());
    }

    #[test]
    fn what_is_spent_with_no_limit_leaves_none_set() {
        super::spend_many(7);
        assert!(!super::limited());
        let b = super::scoped(10);
        assert!(super::limited());
        drop(b);
        assert!(!super::limited());
    }
}

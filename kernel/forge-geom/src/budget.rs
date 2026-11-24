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

/// What one top-level operation that is not a file's read may spend (a boolean, a body's check),
/// where its caller sets no other: eight files' solids' worth, so no op that finished before it was
/// set refuses (the Haas corpus, research/kernel-stress), and one whose NURBS work would never end
/// stops in under a few minutes (UMC-400's bore ran unbounded in `subdivide_pieces`).
pub const OP: u64 = 1 << 28;

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
/// panics leaves no limit behind for what runs next on its thread. Nested in another, it shares
/// it: the inner may spend no more than the outer has left, and what it spends is the outer's.
pub struct Budget {
    /// The outer budget's left when this began, or none.
    outer: Option<u64>,
    /// This one's left when it began.
    start: u64,
}

#[must_use = "the budget ends when this is dropped"]
pub fn scoped(most: u64) -> Budget {
    let outer = LIMITED.get().then(|| LEFT.get());
    let start = outer.map_or(most, |o| o.min(most));
    LEFT.set(start);
    LIMITED.set(true);
    Budget { outer, start }
}

impl Drop for Budget {
    fn drop(&mut self) {
        match self.outer {
            Some(o) => {
                let spent = self.start - LEFT.get().min(self.start);
                LEFT.set(o.saturating_sub(spent));
            }
            None => end(),
        }
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

    #[test]
    fn a_budget_inside_another_shares_it() {
        let outer = super::scoped(100);
        super::spend_many(30);
        {
            // asks more than is left: gets what is left
            let _inner = super::scoped(1000);
            assert_eq!(super::left(), 70);
            super::spend_many(20);
        }
        assert!(super::limited());
        assert_eq!(super::left(), 50);
        {
            let _inner = super::scoped(10);
            assert_eq!(super::left(), 10);
            super::spend_many(10);
            assert!(super::over());
        }
        assert_eq!(super::left(), 40);
        drop(outer);
        assert!(!super::limited());
        assert_eq!(super::left(), u64::MAX);
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

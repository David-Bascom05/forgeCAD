//! How much work the reading of one thing from a file may take: a count of evaluations of a file's
//! spline surfaces and of steps round a mesh's largest fans, the costs nothing else bounds. The same on every machine, so a file is
//! refused or not alike everywhere. Off unless `begin` set it, and nothing made here counts.

use std::cell::Cell;

thread_local! {
    static LEFT: Cell<u64> = const { Cell::new(u64::MAX) };
}

/// From now on, `most` evaluations.
pub fn begin(most: u64) {
    LEFT.set(most);
}

/// No limit again.
pub fn end() {
    LEFT.set(u64::MAX);
}

/// `most` evaluations while this lives, no limit again when it goes, unwinding too: a read that
/// panics leaves no limit behind for what runs next on its thread.
pub struct Budget(());

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

/// Whether what `begin` allowed is gone: the caller stops and says so.
pub fn over() -> bool {
    LEFT.get() == 0
}

/// How many are left (`u64::MAX` less what was spent, with no limit set).
pub fn left() -> u64 {
    LEFT.get()
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_read_that_panics_leaves_no_limit_behind() {
        let r = std::panic::catch_unwind(|| {
            let _b = super::scoped(3);
            super::spend_many(5);
            panic!("a read went wrong");
        });
        assert!(r.is_err());
        assert_eq!(super::left(), u64::MAX);
    }
}

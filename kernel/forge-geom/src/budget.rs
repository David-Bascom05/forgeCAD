//! How much work the reading of one thing from a file may take: a count of evaluations of a file's
//! spline surfaces, the one cost nothing else bounds. The same on every machine, so a file is
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

#[inline]
pub fn spend() {
    LEFT.set(LEFT.get().saturating_sub(1));
}

/// Whether what `begin` allowed is gone: the caller stops and says so.
pub fn over() -> bool {
    LEFT.get() == 0
}

/// How many are left (`u64::MAX` less what was spent, with no limit set).
pub fn left() -> u64 {
    LEFT.get()
}

//! How much work the reading of one thing from a file may take: a count of evaluations of a file's
//! spline surfaces and of steps round a mesh's largest fans, the costs nothing else bounds. The same on every machine, so a file is
//! refused or not alike everywhere. Off unless a `scoped` budget lives, and nothing made here counts.
//!
//! The budget is handed out a slice (`SLICE`) at a time: `over` is a compare with zero, and only
//! where a slice runs out does it look further, at what is left in the bank and at a `Watch`, if
//! one is set on the thread. Through the watch another thread sees how much an op has spent, and
//! can stop it: the op's budget is then gone, and it refuses as for its budget, within a slice's
//! work. The one who stopped it knows to take nothing from that refusal.

use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

thread_local! {
    /// What is left of the slice in hand.
    static LEFT: Cell<u64> = const { Cell::new(u64::MAX) };
    /// What is left beyond it.
    static BANK: Cell<u64> = const { Cell::new(0) };
    /// The slice last handed out (what a watch is told was spent at the next).
    static GIVEN: Cell<u64> = const { Cell::new(0) };
    static LIMITED: Cell<bool> = const { Cell::new(false) };
    static WATCH: RefCell<Option<Arc<Watch>>> = const { RefCell::new(None) };
}

/// What the reading of one solid from a file may spend, where its caller sets no other (the import's
/// `Limits::max_evals`, measured in research/step/README.md).
pub const FILE: u64 = 1 << 25;

/// What one top-level operation that is not a file's read may spend (a boolean, a body's check),
/// where its caller sets no other: sixteen files' solids' worth. The Haas corpus's dearest op that
/// finishes (ST-20LY #1015581 with a copy beside it, 50 s) spends 289M, so none that finished
/// before it was set refuses (research/kernel-stress); one whose NURBS work would never end stops
/// in about a minute and a half (UMC-400's bore ran unbounded in `subdivide_pieces`).
pub const OP: u64 = 1 << 29;

/// How much is handed out at once: how often a watch is looked at (a boolean's 2^20 is a tenth
/// of a second or two, a mesh's a few milliseconds).
pub const SLICE: u64 = 1 << 20;

/// Another thread's view of the ops run on a thread (`watched`): how much they have spent, and a
/// stop.
#[derive(Debug, Default)]
pub struct Watch {
    stop: AtomicBool,
    spent: AtomicU64,
}

impl Watch {
    pub fn new() -> Arc<Watch> {
        Arc::new(Watch::default())
    }

    /// Every budgeted op on the watched thread refuses, as past its budget, within a slice's work
    /// of now, and so does every one begun after, until the watch is taken off.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    /// What the watched thread's budgeted ops have spent, to the last slice (for progress: an op
    /// spends at most its budget, `OP` where nothing else is set).
    pub fn spent(&self) -> u64 {
        self.spent.load(Ordering::Relaxed)
    }
}

/// `w` watches this thread's budgeted ops while this lives (the one before it again after).
pub struct Watching(Option<Arc<Watch>>);

#[must_use = "the watch is taken off when this is dropped"]
pub fn watched(w: Arc<Watch>) -> Watching {
    Watching(WATCH.with(|x| x.borrow_mut().replace(w)))
}

impl Drop for Watching {
    fn drop(&mut self) {
        let before = self.0.take();
        WATCH.with(|x| *x.borrow_mut() = before);
    }
}

/// `n` to spend from now: a slice in hand, the rest in the bank.
fn set_total(n: u64) {
    let s = n.min(SLICE);
    LEFT.set(s);
    BANK.set(n - s);
    GIVEN.set(s);
}

/// No limit again.
fn end() {
    LEFT.set(u64::MAX);
    BANK.set(0);
    GIVEN.set(0);
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
    let outer = LIMITED.get().then(left);
    let start = outer.map_or(most, |o| o.min(most));
    set_total(start);
    LIMITED.set(true);
    Budget { outer, start }
}

impl Drop for Budget {
    fn drop(&mut self) {
        match self.outer {
            Some(o) => {
                let spent = self.start - left().min(self.start);
                set_total(o.saturating_sub(spent));
            }
            None => end(),
        }
    }
}

#[inline]
pub fn spend() {
    let l = LEFT.get();
    if l != 0 {
        LEFT.set(l - 1);
    } else {
        overdraw(1);
    }
}

pub fn spend_many(n: u64) {
    let l = LEFT.get();
    if n <= l {
        LEFT.set(l - n);
    } else {
        LEFT.set(0);
        overdraw(n - l);
    }
}

/// Spent past the slice in hand, before the caller looked at `over`: from the bank, so every
/// evaluation counts whatever the slices (a search that looks only at its start spends on past
/// a slice's end: UMC-400's bore, uncounted so, ran half again as long).
#[cold]
#[inline(never)]
fn overdraw(n: u64) {
    BANK.set(BANK.get().saturating_sub(n));
}

/// Whether what `scoped` allowed is gone, or a watch stopped it: the caller stops and says so.
#[inline]
pub fn over() -> bool {
    LEFT.get() == 0 && !refill()
}

/// The slice in hand is spent: tell the watch, and hand out the next if there is one and no stop.
#[cold]
#[inline(never)]
fn refill() -> bool {
    if !LIMITED.get() {
        return false;
    }
    let stopped = WATCH.with(|w| match &*w.borrow() {
        Some(w) => {
            w.spent.fetch_add(GIVEN.get(), Ordering::Relaxed);
            w.stopped()
        }
        None => false,
    });
    if stopped {
        BANK.set(0);
        return false;
    }
    let b = BANK.get();
    if b == 0 {
        return false;
    }
    let s = b.min(SLICE);
    BANK.set(b - s);
    LEFT.set(s);
    GIVEN.set(s);
    true
}

/// How many are left (`u64::MAX` less what was spent, with no limit set).
pub fn left() -> u64 {
    LEFT.get().saturating_add(BANK.get())
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
    fn a_budget_is_spent_slice_by_slice_and_whole() {
        let n = 3 * super::SLICE + 5;
        let _b = super::scoped(n);
        let mut spent = 0;
        while !super::over() {
            super::spend_many(1000);
            spent += 1000;
        }
        // all of it, not a slice: within a spend of the budget
        assert!(spent >= n && spent < n + 1000, "{spent} of {n}");
    }

    #[test]
    fn a_watched_op_is_stopped_from_another_thread_and_shows_its_progress() {
        let w = super::Watch::new();
        let seen = w.clone();
        let h = std::thread::spawn(move || {
            let _w = super::watched(w);
            let _b = super::scoped(super::OP);
            let mut n = 0u64;
            while !super::over() {
                super::spend_many(64);
                n += 64;
            }
            n
        });
        while seen.spent() < 4 * super::SLICE {
            std::thread::yield_now();
        }
        seen.stop();
        let n = h.join().unwrap();
        // it stopped well short of its budget, within a slice of the stop
        assert!(n < super::OP / 2, "{n}");
        assert!(seen.spent() >= 4 * super::SLICE);
        // and the thread that ran it has no limit left behind
    }

    #[test]
    fn a_watch_unstopped_changes_nothing() {
        let _w = super::watched(super::Watch::new());
        let _b = super::scoped(10 * super::SLICE);
        let mut n = 0u64;
        while !super::over() {
            super::spend_many(4096);
            n += 4096;
        }
        assert_eq!(n, 10 * super::SLICE);
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

//! How much work the reading of one thing from a file may take: a count of evaluations of a file's
//! spline surfaces and of steps round a mesh's largest fans, the costs nothing else bounds. The same on every machine, so a file is
//! refused or not alike everywhere. Off unless a `scoped` budget lives, and nothing made here counts.
//!
//! The budget is handed out a slice (`SLICE`) at a time: `over` is a compare with zero, and only
//! where a slice runs out does it look further, at what is left in the bank and at a `Watch`, if
//! one is set on the thread. Through the watch another thread sees how much an op has spent, and
//! can stop it: the op's budget is then gone, and it refuses as for its budget, within a slice's
//! work. The one who stopped it knows to take nothing from that refusal.
//!
//! Threads an op starts (`forge_math::par`'s) share its budget and its watch: on the first, the
//! bank becomes one all its threads draw slices from, each giving back what it holds of its slice
//! when it ends, so what is spent is counted whole wherever it was spent. A thread that finds the
//! shared bank empty marks it dry, and the op, whichever thread looks (`exhausted`), refuses.

use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

thread_local! {
    /// What is left of the slice in hand.
    static LEFT: Cell<u64> = const { Cell::new(u64::MAX) };
    /// What is left beyond it (while no bank is shared).
    static BANK: Cell<u64> = const { Cell::new(0) };
    /// The slice last handed out (what a watch is told was spent at the next).
    static GIVEN: Cell<u64> = const { Cell::new(0) };
    static LIMITED: Cell<bool> = const { Cell::new(false) };
    static WATCH: RefCell<Option<Arc<Watch>>> = const { RefCell::new(None) };
    /// The bank this thread shares with the op's other threads, once it has started one.
    static SHARED: RefCell<Option<Arc<Shared>>> = const { RefCell::new(None) };
    /// The slice handed out at once on this thread (`SLICE`; less only in tests: `slice_for_tests`).
    static SLICE_NOW: Cell<u64> = const { Cell::new(SLICE) };
}

/// For tests: slices of `n` on this thread (a stop then reaches an op after `n` of its work, not a
/// slice's, so a small part's op can be seen cut short). Slices are only how often a watch is
/// looked at: what is spent and what is allowed are the same whatever their size.
#[doc(hidden)]
pub fn slice_for_tests(n: u64) {
    SLICE_NOW.set(n.max(1));
}

/// An op's bank, shared by its threads.
#[derive(Debug)]
struct Shared {
    bank: AtomicU64,
    /// A thread found it empty (or the watch stopped): the op's work is cut short somewhere.
    dry: AtomicBool,
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

    /// Every budgeted op on the watched thread (and the threads it starts) refuses, as past its
    /// budget, within a slice's work of now, and so does every one begun after, until the watch is
    /// taken off.
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
    hand_down();
    Watching(WATCH.with(|x| x.borrow_mut().replace(w)))
}

impl Drop for Watching {
    fn drop(&mut self) {
        let before = self.0.take();
        WATCH.with(|x| *x.borrow_mut() = before);
    }
}

/// Whether a watch on this thread was stopped: what is made now may have been cut short (an op
/// refused as past its budget, a fallback taken for it), so nothing made is to be kept for later.
pub fn stopped() -> bool {
    WATCH.with(|w| w.borrow().as_ref().is_some_and(|w| w.stopped()))
}

fn shared() -> Option<Arc<Shared>> {
    SHARED.with(|s| s.borrow().clone())
}

/// What the bank holds beyond the slice in hand.
fn bank() -> u64 {
    match shared() {
        Some(s) => s.bank.load(Ordering::Relaxed),
        None => BANK.get(),
    }
}

/// Up to `want` from the bank: what it gave.
fn bank_take(want: u64) -> u64 {
    match shared() {
        Some(s) => {
            let was = s
                .bank
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |b| Some(b - b.min(want)))
                .unwrap_or(0);
            was.min(want)
        }
        None => {
            let b = BANK.get();
            let t = b.min(want);
            BANK.set(b - t);
            t
        }
    }
}

/// `n` to spend from now: a slice in hand, the rest in the bank (this thread's own).
fn set_total(n: u64) {
    let s = n.min(SLICE_NOW.get());
    SHARED.with(|x| *x.borrow_mut() = None);
    LEFT.set(s);
    BANK.set(n - s);
    GIVEN.set(s);
}

/// No limit again.
fn end() {
    SHARED.with(|x| *x.borrow_mut() = None);
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
/// On a thread sharing an op's bank (one the op started, or the op's own once it has) it only
/// shares that: the op's budget holds.
pub struct Budget {
    /// The outer budget's left when this began, or none.
    outer: Option<u64>,
    /// This one's left when it began.
    start: u64,
    /// Begun on a shared bank: the op's, nothing of its own.
    within: bool,
}

#[must_use = "the budget ends when this is dropped"]
pub fn scoped(most: u64) -> Budget {
    hand_down();
    if shared().is_some() {
        return Budget {
            outer: None,
            start: 0,
            within: true,
        };
    }
    let outer = LIMITED.get().then(left);
    let start = outer.map_or(most, |o| o.min(most));
    set_total(start);
    LIMITED.set(true);
    Budget {
        outer,
        start,
        within: false,
    }
}

impl Drop for Budget {
    fn drop(&mut self) {
        if self.within {
            return;
        }
        // (the threads it started are joined: what they held of their slices is back in the bank)
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
    bank_take(n);
}

/// Whether what `scoped` allowed is gone, or a watch stopped it: the caller stops and says so.
#[inline]
pub fn over() -> bool {
    LEFT.get() == 0 && !refill()
}

/// `over`, or the op's shared bank run dry on another of its threads (whose work is then cut
/// short): what an op asks at its end, before it answers.
pub fn exhausted() -> bool {
    over() || shared().is_some_and(|s| s.dry.load(Ordering::Relaxed))
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
    GIVEN.set(0);
    let sh = shared();
    if stopped {
        match &sh {
            Some(s) => s.bank.store(0, Ordering::Relaxed),
            None => BANK.set(0),
        }
    }
    let s = if stopped { 0 } else { bank_take(SLICE_NOW.get()) };
    if s == 0 {
        if let Some(sh) = sh {
            sh.dry.store(true, Ordering::Relaxed);
        }
        return false;
    }
    LEFT.set(s);
    GIVEN.set(s);
    true
}

/// How many are left (`u64::MAX` less what was spent, with no limit set): on a shared bank, what
/// the op's other threads hold of their slices aside.
pub fn left() -> u64 {
    LEFT.get().saturating_add(bank())
}

/// What a thread an op starts takes from it: its bank (shared from now on) and its watch.
struct Handed {
    shared: Option<Arc<Shared>>,
    watch: Option<Arc<Watch>>,
}

/// A started thread's state before it took the op's, put back when it is done.
struct Taken {
    left: u64,
    bank: u64,
    given: u64,
    limited: bool,
    watch: Option<Arc<Watch>>,
    shared: Option<Arc<Shared>>,
}

impl Drop for Taken {
    fn drop(&mut self) {
        // what it holds of its slice, back to the op
        if let Some(s) = shared() {
            s.bank.fetch_add(LEFT.get(), Ordering::Relaxed);
        }
        if let Some(w) = WATCH.with(|w| w.borrow().clone()) {
            w.spent
                .fetch_add(GIVEN.get() - LEFT.get().min(GIVEN.get()), Ordering::Relaxed);
        }
        LEFT.set(self.left);
        BANK.set(self.bank);
        GIVEN.set(self.given);
        LIMITED.set(self.limited);
        WATCH.with(|w| *w.borrow_mut() = self.watch.take());
        SHARED.with(|x| *x.borrow_mut() = self.shared.take());
    }
}

fn capture() -> Option<Box<dyn std::any::Any + Send>> {
    let watch = WATCH.with(|w| w.borrow().clone());
    let limited = LIMITED.get();
    if !limited && watch.is_none() {
        return None;
    }
    let shared = limited.then(|| {
        shared().unwrap_or_else(|| {
            let s = Arc::new(Shared {
                bank: AtomicU64::new(BANK.get()),
                dry: AtomicBool::new(false),
            });
            BANK.set(0);
            SHARED.with(|x| *x.borrow_mut() = Some(s.clone()));
            s
        })
    });
    Some(Box::new(Handed { shared, watch }))
}

fn enter(h: Box<dyn std::any::Any + Send>) -> Box<dyn std::any::Any> {
    let Ok(h) = h.downcast::<Handed>() else {
        return Box::new(());
    };
    let Handed { shared, watch } = *h;
    let taken = Taken {
        left: LEFT.get(),
        bank: BANK.get(),
        given: GIVEN.get(),
        limited: LIMITED.get(),
        watch: WATCH.with(|w| w.borrow_mut().take()),
        shared: SHARED.with(|x| x.borrow_mut().take()),
    };
    let limited = shared.is_some();
    LEFT.set(if limited { 0 } else { u64::MAX });
    BANK.set(0);
    GIVEN.set(0);
    LIMITED.set(limited);
    WATCH.with(|w| *w.borrow_mut() = watch);
    SHARED.with(|x| *x.borrow_mut() = shared);
    Box::new(taken)
}

/// Threads the kernel starts take the budget and watch of the thread that starts them
/// (`forge_par::Inherit`), set once.
fn hand_down() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        forge_math::par::set_inherit(forge_math::par::Inherit { capture, enter });
    });
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
    fn the_threads_an_op_starts_spend_its_budget_and_every_spend_counts() {
        let outer = super::scoped(u64::MAX >> 1);
        let before = super::left();
        {
            let _op = super::scoped(super::OP);
            // 4 threads started here and this one: 100k each
            let work = || {
                for _ in 0..100 {
                    super::spend_many(1000);
                    let _ = super::over();
                }
            };
            forge_math::par::scope(|s| {
                let hs: Vec<_> = (0..4).map(|_| s.spawn(work)).collect();
                work();
                for h in hs {
                    h.join().unwrap();
                }
            });
            assert!(!super::exhausted());
        }
        assert_eq!(before - super::left(), 5 * 100 * 1000);
        drop(outer);
        // a budget the threads run past: the op, on its own thread, is told
        let _op = super::scoped(3 * super::SLICE);
        let work = || {
            for _ in 0..1000 {
                super::spend_many(1000);
                if super::over() {
                    break;
                }
            }
        };
        forge_math::par::scope(|s| {
            for _ in 0..4 {
                s.spawn(work);
            }
        });
        assert!(super::exhausted());
    }

    #[test]
    fn a_stop_reaches_the_threads_an_op_starts() {
        let w = super::Watch::new();
        let seen = w.clone();
        let h = std::thread::spawn(move || {
            let _w = super::watched(w);
            let _op = super::scoped(super::OP);
            let n = std::sync::atomic::AtomicU64::new(0);
            let work = || {
                while !super::over() {
                    super::spend_many(64);
                    n.fetch_add(64, std::sync::atomic::Ordering::Relaxed);
                }
            };
            forge_math::par::scope(|s| {
                for _ in 0..4 {
                    s.spawn(work);
                }
            });
            (n.into_inner(), super::exhausted())
        });
        while seen.spent() < 8 * super::SLICE {
            std::thread::yield_now();
        }
        seen.stop();
        let (n, told) = h.join().unwrap();
        assert!(n < super::OP / 2, "{n}");
        assert!(told);
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

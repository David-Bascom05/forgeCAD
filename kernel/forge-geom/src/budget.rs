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

use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

thread_local! {
    /// What is left of the slice in hand.
    static LEFT: Cell<u64> = const { Cell::new(u64::MAX) };
    /// What this thread has drawn into its hand so far (wrapping): `spent` is this less `LEFT`, kept
    /// by the cold paths only (a slice handed out, an overdraw), so `spend` stays as it was.
    static DRAWN: Cell<u64> = const { Cell::new(u64::MAX) };
    /// What is left beyond it (while no bank is shared).
    static BANK: Cell<u64> = const { Cell::new(0) };
    /// The slice last handed out (what a watch is told was spent at the next).
    static GIVEN: Cell<u64> = const { Cell::new(0) };
    static LIMITED: Cell<bool> = const { Cell::new(false) };
    static WATCH: Cell<Option<Arc<Watch>>> = const { Cell::new(None) };
    /// The bank this thread shares with the op's other threads, once it has started one.
    static SHARED: Cell<Option<Arc<Shared>>> = const { Cell::new(None) };
    /// The slice handed out at once on this thread (`SLICE`; less only in tests: `slice_for_tests`).
    static SLICE_NOW: Cell<u64> = const { Cell::new(SLICE) };
    /// Inside `uncounted`: the op's budget held aside, its counters lifted, so nothing here spends
    /// it, is stopped by its watch or cut short by its shared bank (and no `scoped` adds a cap).
    static FREE: Cell<bool> = const { Cell::new(false) };
    /// What the budget in force allows in all (its `scoped` or `isolated` start, less a `cap_total`
    /// cut), while no bank is shared.
    static TOTAL: Cell<u64> = const { Cell::new(u64::MAX) };
    /// What a `cap_total` cut could not take at once (the op's other threads held it in hand):
    /// paid from the bank before anything else is taken from it, while no bank is shared.
    static OWED: Cell<u64> = const { Cell::new(0) };
    /// Whether the budget in force was cut by `cap_total` (once only), while no bank is shared.
    static CAPPED: Cell<bool> = const { Cell::new(false) };
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
    /// `TOTAL`, `OWED` and `CAPPED` for the op's threads together.
    total: AtomicU64,
    owed: std::sync::Mutex<u64>,
    owing: AtomicBool,
    capped: AtomicBool,
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
    Watching(WATCH.with(|x| x.replace(Some(w))))
}

impl Drop for Watching {
    fn drop(&mut self) {
        let before = self.0.take();
        WATCH.with(|x| x.set(before));
    }
}

/// Whether a watch on this thread was stopped: what is made now may have been cut short (an op
/// refused as past its budget, a fallback taken for it), so nothing made is to be kept for later.
pub fn stopped() -> bool {
    !FREE.get() && with_watch(|w| w.as_ref().is_some_and(|w| w.stopped()))
}

fn shared() -> Option<Arc<Shared>> {
    if FREE.get() {
        return None;
    }
    SHARED.with(|s| {
        let v = s.take();
        let c = v.clone();
        s.set(v);
        c
    })
}

/// `f` of this thread's watch (taken out of its cell while `f` runs: `f` must not look at it).
fn with_watch<R>(f: impl FnOnce(&Option<Arc<Watch>>) -> R) -> R {
    WATCH.with(|c| {
        let v = c.take();
        let r = f(&v);
        c.set(v);
        r
    })
}

/// What the bank holds beyond the slice in hand.
fn bank() -> u64 {
    match shared() {
        Some(s) => s.bank.load(Ordering::Relaxed).saturating_sub(owed()),
        None => BANK.get().saturating_sub(OWED.get()),
    }
}

/// What a `cap_total` cut still owes the bank.
fn owed() -> u64 {
    match shared() {
        Some(s) => {
            if s.owing.load(Ordering::Relaxed) {
                *s.owed.lock().unwrap_or_else(|e| e.into_inner())
            } else {
                0
            }
        }
        None => OWED.get(),
    }
}

/// What the budget in force allows in all (see `TOTAL`).
fn total() -> u64 {
    match shared() {
        Some(s) => s.total.load(Ordering::Relaxed),
        None => TOTAL.get(),
    }
}

/// What the budget in force has spent (on all the op's threads, their slices back in the bank).
fn spent_of_total() -> u64 {
    let t = total();
    t - (LEFT.get().saturating_add(bank())).min(t)
}

/// The op's budget, from now on, `m` in all where it allowed more: what it allowed past `m` taken
/// out of the bank (and the slice in hand), what other threads hold in theirs owed, paid first
/// from what they put back; an op past `m` already is over (`exhausted`). Once per budget: the
/// first call cuts, later ones do nothing. For a path that may run long and may still answer
/// (a branch carried onto a spline patch's rim, intcurve.rs): whether it is taken is decided by
/// the op's geometry, and what is spent does not hang on threads or order, so the verdict is the
/// same everywhere. Nothing inside `uncounted`, or with no budget.
pub fn cap_total(m: u64) {
    if FREE.get() || !LIMITED.get() {
        return;
    }
    let mut cut = match shared() {
        Some(s) => {
            if s.capped.swap(true, Ordering::Relaxed) {
                return;
            }
            let t = s.total.load(Ordering::Relaxed);
            if t <= m {
                return;
            }
            s.total.store(m, Ordering::Relaxed);
            let want = t - m;
            let was = s
                .bank
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |b| Some(b - b.min(want)))
                .unwrap_or(0);
            want - was.min(want)
        }
        None => {
            if CAPPED.replace(true) {
                return;
            }
            let t = TOTAL.get();
            if t <= m {
                return;
            }
            TOTAL.set(m);
            let want = t - m;
            let b = BANK.get();
            BANK.set(b - b.min(want));
            want - b.min(want)
        }
    };
    // from the slice in hand (allowed, not spent: `spent` unmoved)
    let l = LEFT.get();
    let t = l.min(cut);
    LEFT.set(l - t);
    DRAWN.set(DRAWN.get().wrapping_sub(t));
    cut -= t;
    if cut > 0 {
        match shared() {
            Some(s) => {
                *s.owed.lock().unwrap_or_else(|e| e.into_inner()) += cut;
                s.owing.store(true, Ordering::Relaxed);
            }
            None => OWED.set(OWED.get() + cut),
        }
    }
}

/// Up to `want` from the bank: what it gave.
fn bank_take(want: u64) -> u64 {
    match shared() {
        Some(s) => {
            // (a cut's debt first: see cap_total)
            if s.owing.load(Ordering::Relaxed) {
                let mut o = s.owed.lock().unwrap_or_else(|e| e.into_inner());
                let was = s
                    .bank
                    .try_update(Ordering::Relaxed, Ordering::Relaxed, |b| Some(b - b.min(*o)))
                    .unwrap_or(0);
                *o -= was.min(*o);
                if *o == 0 {
                    s.owing.store(false, Ordering::Relaxed);
                }
            }
            let was = s
                .bank
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |b| Some(b - b.min(want)))
                .unwrap_or(0);
            was.min(want)
        }
        None => {
            let o = OWED.get();
            if o > 0 {
                let b = BANK.get();
                BANK.set(b - b.min(o));
                OWED.set(o - b.min(o));
            }
            let b = BANK.get();
            let t = b.min(want);
            BANK.set(b - t);
            t
        }
    }
}

/// `n` to spend from now: a slice in hand, the rest in the bank (this thread's own).
fn set_total(n: u64) {
    TOTAL.set(n);
    OWED.set(0);
    CAPPED.set(false);
    let s = n.min(SLICE_NOW.get());
    SHARED.with(|x| x.set(None));
    DRAWN.set(spent().wrapping_add(s));
    LEFT.set(s);
    BANK.set(n - s);
    GIVEN.set(s);
}

/// No limit again.
fn end() {
    TOTAL.set(u64::MAX);
    OWED.set(0);
    CAPPED.set(false);
    SHARED.with(|x| x.set(None));
    DRAWN.set(spent().wrapping_add(u64::MAX));
    LEFT.set(u64::MAX);
    BANK.set(0);
    GIVEN.set(0);
    LIMITED.set(false);
}

/// Whether a `scoped` budget lives on this thread. (Not `left() == u64::MAX`: what is spent with
/// no limit set still counts down from there.)
pub fn limited() -> bool {
    LIMITED.get() && !FREE.get()
}

/// `most` evaluations while this lives, no limit again when it goes, unwinding too: a read that
/// panics leaves no limit behind for what runs next on its thread. Nested in another, it shares
/// it: the inner may spend no more than the outer has left, and what it spends is the outer's.
/// On a thread sharing an op's bank (one the op started, or the op's own once it has) it only
/// shares that: the op's budget holds.
pub struct Budget {
    /// The outer budget's left when this began, or none.
    outer: Option<u64>,
    /// Begun on a shared bank: the op's, nothing of its own.
    within: bool,
}

#[must_use = "the budget ends when this is dropped"]
pub fn scoped(most: u64) -> Budget {
    hand_down();
    if FREE.get() || shared().is_some() {
        return Budget {
            outer: None,
            within: true,
        };
    }
    let outer = LIMITED.get().then(left);
    let start = outer.map_or(most, |o| o.min(most));
    set_total(start);
    LIMITED.set(true);
    Budget { outer, within: false }
}

impl Drop for Budget {
    fn drop(&mut self) {
        if self.within {
            return;
        }
        // (the threads it started are joined: what they held of their slices is back in the bank)
        match self.outer {
            Some(o) => {
                // (what was spent: a cap_total cut was allowed, not spent)
                let spent = spent_of_total();
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
    DRAWN.set(DRAWN.get().wrapping_add(n));
    bank_take(n);
}

/// What this thread has spent, all told (wrapping; a difference of two readings is what was spent
/// between them, on this thread): what a memo that answers a question again charges, the cost
/// the first answer had, so an op spends the same whether its answers are fresh or remembered.
pub fn spent() -> u64 {
    DRAWN.get().wrapping_sub(LEFT.get())
}

/// Whether what `scoped` allowed is gone, or a watch stopped it: the caller stops and says so.
#[inline]
pub fn over() -> bool {
    LEFT.get() == 0 && !refill()
}

/// `over`, or the op's shared bank run dry on another of its threads (whose work is then cut
/// short): what an op asks at its end, before it answers.
pub fn exhausted() -> bool {
    over() || shared().is_some_and(|s| s.dry.load(Ordering::Relaxed)) || owed() > bank_held()
}

/// The bank as it holds, before a cut's debt is paid from it.
fn bank_held() -> u64 {
    match shared() {
        Some(s) => s.bank.load(Ordering::Relaxed),
        None => BANK.get(),
    }
}

/// The slice in hand is spent: tell the watch, and hand out the next if there is one and no stop.
#[cold]
#[inline(never)]
fn refill() -> bool {
    if !LIMITED.get() {
        return false;
    }
    let stopped = with_watch(|w| match w {
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
    DRAWN.set(DRAWN.get().wrapping_add(s));
    LEFT.set(s);
    GIVEN.set(s);
    true
}

/// How many are left (`u64::MAX` less what was spent, with no limit set): on a shared bank, what
/// the op's other threads hold of their slices aside.
pub fn left() -> u64 {
    LEFT.get().saturating_add(bank())
}

/// `f`'s evaluations not counted: a lazy cache's filling (a spline's seed grid, a face's box), which
/// whatever asks first would otherwise pay, so that what an op spends, and whether it runs past
/// its budget, would hang on what ran before it and, on several threads, on which got there
/// first. Uncounted, a cache costs every op the same: nothing. `f` runs as with no budget (no
/// limit, no watch, no shared bank), and this thread's budget is as it was after.
#[inline]
pub fn uncounted<T>(f: impl FnOnce() -> T) -> T {
    // only the slice in hand held aside (a cache fill is the most frequent call here: taking the
    // whole state out and back was 0.8% of a small part's regen); the bank, the watch and the
    // shared bank stay where they are, unseen while FREE. (`spent` inside counts on from a slice
    // of u64::MAX: only its differences mean anything there.) A fill inside a fill (an edge's box
    // inside its face's) is free already. One call of `f`, so it inlines where it is written.
    let _back = (LIMITED.get() && !FREE.get()).then(|| Freed {
        left: LEFT.replace(u64::MAX),
        free: FREE.replace(true),
    });
    f()
}

/// `uncounted`'s slice in hand put back when it goes, unwinding too.
struct Freed {
    left: u64,
    free: bool,
}

impl Drop for Freed {
    #[inline]
    fn drop(&mut self) {
        LEFT.set(self.left);
        FREE.set(self.free);
    }
}

/// `f` on a counter of its own: at most `limit`, drawn from no bank but its own (not the op's, even
/// on a thread sharing one), what it spends returned and charged nowhere else. For work whose
/// spend must be its own whatever thread runs it (an item of a parallel step summed in item
/// order: IMPORT-SPEED's `map_until`). A stop on this thread's watch reaches it as any budget.
/// What `f` sees as over is `f`'s own budget, as for `scoped`.
pub fn isolated<T>(limit: u64, f: impl FnOnce() -> T) -> (u64, T) {
    hand_down();
    // inside `uncounted`, the op's watch is out of reach here too
    let free = FREE.replace(false);
    let was = Taken {
        left: LEFT.get(),
        drawn: DRAWN.get(),
        bank: BANK.get(),
        given: GIVEN.get(),
        limited: LIMITED.get(),
        free,
        total: TOTAL.get(),
        owed: OWED.get(),
        capped: CAPPED.get(),
        watch: if free {
            WATCH.with(|w| w.take())
        } else {
            with_watch(|w| w.clone())
        },
        shared: SHARED.with(|x| x.take()),
    };
    let _back = Back(was);
    set_total(limit);
    LIMITED.set(true);
    let r = f();
    (spent_of_total(), r)
}

/// A thread's budget put back as it was when this goes.
struct Back(Taken);

impl Drop for Back {
    fn drop(&mut self) {
        self.0.put_back();
    }
}

/// What a thread an op starts takes from it: its bank (shared from now on) and its watch.
struct Handed {
    shared: Option<Arc<Shared>>,
    watch: Option<Arc<Watch>>,
}

/// A started thread's state before it took the op's, put back when it is done.
struct Taken {
    left: u64,
    drawn: u64,
    bank: u64,
    given: u64,
    limited: bool,
    free: bool,
    total: u64,
    owed: u64,
    capped: bool,
    watch: Option<Arc<Watch>>,
    shared: Option<Arc<Shared>>,
}

impl Taken {
    /// The thread's state put back as it was taken.
    fn put_back(&mut self) {
        LEFT.set(self.left);
        DRAWN.set(self.drawn);
        BANK.set(self.bank);
        GIVEN.set(self.given);
        LIMITED.set(self.limited);
        FREE.set(self.free);
        TOTAL.set(self.total);
        OWED.set(self.owed);
        CAPPED.set(self.capped);
        WATCH.with(|w| w.set(self.watch.take()));
        SHARED.with(|x| x.set(self.shared.take()));
    }
}

/// A started thread done: what it holds of its slice back to the op, its own state back.
struct Started(Taken);

impl Drop for Started {
    fn drop(&mut self) {
        if let Some(s) = shared() {
            s.bank.fetch_add(LEFT.get(), Ordering::Relaxed);
        }
        if let Some(w) = with_watch(|w| w.clone()) {
            w.spent
                .fetch_add(GIVEN.get() - LEFT.get().min(GIVEN.get()), Ordering::Relaxed);
        }
        self.0.put_back();
    }
}

fn capture() -> Option<Box<dyn std::any::Any + Send>> {
    // a thread started inside `uncounted` runs as it does: with no budget, no watch
    if FREE.get() {
        return None;
    }
    let watch = with_watch(|w| w.clone());
    let limited = LIMITED.get();
    if !limited && watch.is_none() {
        return None;
    }
    let shared = limited.then(|| {
        shared().unwrap_or_else(|| {
            let s = Arc::new(Shared {
                bank: AtomicU64::new(BANK.get()),
                dry: AtomicBool::new(false),
                total: AtomicU64::new(TOTAL.get()),
                owed: std::sync::Mutex::new(OWED.get()),
                owing: AtomicBool::new(OWED.get() > 0),
                capped: AtomicBool::new(CAPPED.get()),
            });
            BANK.set(0);
            OWED.set(0);
            SHARED.with(|x| x.set(Some(s.clone())));
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
        drawn: DRAWN.get(),
        bank: BANK.get(),
        given: GIVEN.get(),
        limited: LIMITED.get(),
        free: FREE.replace(false),
        total: TOTAL.get(),
        owed: OWED.get(),
        capped: CAPPED.get(),
        watch: WATCH.with(|w| w.take()),
        shared: SHARED.with(|x| x.take()),
    };
    let limited = shared.is_some();
    let l = if limited { 0 } else { u64::MAX };
    DRAWN.set(spent().wrapping_add(l));
    LEFT.set(l);
    BANK.set(0);
    GIVEN.set(0);
    // (a cut and its debt are the shared bank's: see cap_total)
    TOTAL.set(u64::MAX);
    OWED.set(0);
    CAPPED.set(false);
    LIMITED.set(limited);
    WATCH.with(|w| w.set(watch));
    SHARED.with(|x| x.set(shared));
    Box::new(Started(taken))
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
    #[cfg_attr(target_os = "wasi", ignore = "a thread of its own: WASI has none")]
    fn a_watched_op_is_stopped_from_another_thread_and_shows_its_progress() {
        let w = super::Watch::new();
        let seen = w.clone();
        let h = std::thread::spawn(move || {
            let _w = super::watched(w);
            // (a budget it cannot run out of before the stop comes)
            let _b = super::scoped(u64::MAX >> 4);
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
        assert!(n < u64::MAX >> 5, "{n}");
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
    #[cfg_attr(target_os = "wasi", ignore = "a thread of its own: WASI has none")]
    fn a_stop_reaches_the_threads_an_op_starts() {
        let w = super::Watch::new();
        let seen = w.clone();
        let h = std::thread::spawn(move || {
            let _w = super::watched(w);
            let _op = super::scoped(u64::MAX >> 4);
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
        assert!(n < u64::MAX >> 5, "{n}");
        assert!(told);
    }

    #[test]
    fn isolated_work_is_counted_on_its_own_even_on_a_shared_bank() {
        let _op = super::scoped(1000);
        super::spend_many(100);
        forge_math::par::scope(|s| {
            s.spawn(|| {
                // a helper, sharing the op's bank: isolated, its spend is its own
                let (n, ()) = super::isolated(50, || super::spend_many(30));
                assert_eq!(n, 30);
                let (n, over) = super::isolated(50, || {
                    super::spend_many(60);
                    super::over()
                });
                assert_eq!(n, 50);
                assert!(over);
            })
            .join()
            .unwrap();
        });
        // and nothing of it is the op's
        assert_eq!(super::left(), 900);
    }

    #[test]
    fn a_cap_cuts_the_budget_in_force_once_and_charges_only_what_was_spent() {
        let outer = super::scoped(u64::MAX >> 1);
        let before = super::left();
        for slice in [100, super::SLICE] {
            super::slice_for_tests(slice);
            {
                let _op = super::scoped(10_000);
                super::spend_many(3000);
                super::cap_total(5000);
                // (later cuts do nothing)
                super::cap_total(100);
                super::spend_many(1999);
                assert!(!super::exhausted(), "slice {slice}");
                super::spend_many(1);
                assert!(super::exhausted(), "slice {slice}");
            }
            // the outer budget is charged what was spent, not what the cut took away
            {
                let was = super::left();
                {
                    let _op = super::scoped(10_000);
                    super::spend_many(3000);
                    super::cap_total(5000);
                    super::spend_many(1000);
                }
                assert_eq!(was - super::left(), 4000, "slice {slice}");
            }
            // past the cap already: over at once
            let _op = super::scoped(10_000);
            super::spend_many(6000);
            super::cap_total(5000);
            assert!(super::exhausted(), "slice {slice}");
        }
        super::slice_for_tests(super::SLICE);
        assert!(before > super::left());
        drop(outer);
    }

    /// Cut on any of an op's threads, while the others hold slices: the op is as if it had started
    /// with the cap (its bank less what the cut took; owed where the threads held it in hand). As
    /// for any budget shared by threads, within a few slices of the cap what the threads hold
    /// decides; clear of it, the verdict is the spend's.
    #[test]
    fn a_cap_on_any_thread_is_the_ops_whatever_the_threads_hold() {
        for (cap, over) in [(100_000, true), (50_000_000, false)] {
            let _op = super::scoped(super::OP);
            // five items of 100k, the third cutting the op's budget at its start
            let work = |k: usize| {
                if k == 2 {
                    super::cap_total(cap);
                }
                for _ in 0..100 {
                    super::spend_many(1000);
                    if super::over() {
                        break;
                    }
                }
            };
            forge_math::par::scope(|s| {
                let hs: Vec<_> = (0..4).map(|k| s.spawn(move || work(k))).collect();
                work(4);
                for h in hs {
                    h.join().unwrap();
                }
            });
            assert_eq!(super::exhausted(), over, "cap {cap}");
        }
    }

    #[test]
    fn uncounted_work_leaves_the_budget_as_it_was() {
        let _op = super::scoped(1000);
        super::spend_many(100);
        let over = super::uncounted(|| {
            super::spend_many(5000);
            super::over()
        });
        assert!(!over);
        assert_eq!(super::left(), 900);
        assert!(super::limited());
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

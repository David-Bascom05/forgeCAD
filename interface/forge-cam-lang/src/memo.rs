//! Results of pure kernel calls kept across runs of the document, keyed by every argument.
//!
//! The app re-runs the whole document at every edit; most of its ops and their checks see the
//! same inputs as the run before. A kernel call that is a function of its arguments alone (no
//! clock, no file, no state other than the kernel's own content-keyed memos) gives the same
//! result for the same arguments, so the result made before is the result.
//!
//! The key is the arguments' full `Debug` text, compared whole (the 64-bit hash only picks the
//! bucket): f64s print as the shortest text that reads back to the same number, and −0.0 as
//! "-0.0", so two keys are equal only if every number is the same number. A key holding "NaN"
//! (whose text does not tell NaNs apart) is never kept. Every type in a key derives `Debug`, so
//! no field is left out of it (a hand-written Debug impl in a key's type would have to print
//! every field). Bounded: the least recently used entries go past `MAX_BYTES` of keys.
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

const MAX_BYTES: usize = 64 << 20;

std::thread_local! {
    /// The tests' switch: every call on this thread made anew (`bypass`).
    static BYPASS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Whose calls this thread's are, for `computed`: its own (a number taken when first
    /// asked), or, while it does work handed over by another thread (`Context::enter`), that
    /// thread's.
    static GROUP: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Calls made anew, per group and memo (by its address: the memos are statics).
static COMPUTED: Mutex<Option<HashMap<(u64, usize), u64>>> = Mutex::new(None);
static NEXT_GROUP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn group() -> u64 {
    GROUP.with(|g| {
        if g.get() == 0 {
            g.set(NEXT_GROUP.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        }
        g.get()
    })
}

/// A thread's memo state (`bypass` and whose calls are counted), for work it hands to other
/// threads: each enters it (`enter`) while doing that work, so a test's bypass holds there and
/// the calls made there count as the test's.
#[derive(Clone, Copy)]
pub struct Context {
    group: u64,
    bypass: bool,
}

/// The calling thread's memo state.
pub fn context() -> Context {
    Context {
        group: group(),
        bypass: BYPASS.with(|b| b.get()),
    }
}

impl Context {
    /// `f`, on this thread, under the state of the thread the context was taken on; this
    /// thread's own state back after (also on a panic).
    pub fn enter<T>(&self, f: impl FnOnce() -> T) -> T {
        struct Back(u64, bool);
        impl Drop for Back {
            fn drop(&mut self) {
                GROUP.with(|g| g.set(self.0));
                BYPASS.with(|b| b.set(self.1));
            }
        }
        let _back = Back(GROUP.with(|g| g.get()), BYPASS.with(|b| b.get()));
        GROUP.with(|g| g.set(self.group));
        BYPASS.with(|b| b.set(self.bypass));
        f()
    }
}

struct Entry<V> {
    key: String,
    value: V,
    used: u64,
}

pub struct Memo<V> {
    inner: Mutex<Inner<V>>,
}

struct Inner<V> {
    map: Option<HashMap<u64, Vec<Entry<V>>>>,
    bytes: usize,
    clock: u64,
}

impl<V: Clone> Memo<V> {
    pub const fn new() -> Memo<V> {
        Memo {
            inner: Mutex::new(Inner {
                map: None,
                bytes: 0,
                clock: 0,
            }),
        }
    }

    /// The value for `key`: the one made before for the same key, else `make()`, kept.
    pub fn get_or(&self, key: String, make: impl FnOnce() -> V) -> V {
        if key.contains("NaN") || BYPASS.with(|b| b.get()) {
            self.count();
            return make();
        }
        let h = {
            let mut s = std::collections::hash_map::DefaultHasher::new();
            key.hash(&mut s);
            s.finish()
        };
        if let Ok(mut g) = self.inner.lock() {
            g.clock += 1;
            let now = g.clock;
            if let Some(v) = g
                .map
                .get_or_insert_with(HashMap::new)
                .get_mut(&h)
                .and_then(|b| b.iter_mut().find(|e| e.key == key))
            {
                v.used = now;
                return v.value.clone();
            }
        }
        self.count();
        // made outside the lock: other threads' lookups go on meanwhile
        let value = make();
        if let Ok(mut g) = self.inner.lock() {
            let len = key.len();
            if len <= MAX_BYTES / 4 {
                g.bytes += len;
                let now = g.clock;
                g.map
                    .get_or_insert_with(HashMap::new)
                    .entry(h)
                    .or_default()
                    .push(Entry {
                        key,
                        value: value.clone(),
                        used: now,
                    });
                // past the budget: the least recently used go (in order of use, ties by key)
                while g.bytes > MAX_BYTES {
                    let map = g.map.as_mut().expect("the map");
                    let Some((&bh, i)) = map
                        .iter()
                        .flat_map(|(bh, b)| b.iter().enumerate().map(move |(i, e)| ((bh, i), e.used)))
                        .min_by_key(|(_, u)| *u)
                        .map(|(k, _)| k)
                    else {
                        break;
                    };
                    let b = map.get_mut(&bh).expect("the bucket");
                    let gone = b.remove(i);
                    if b.is_empty() {
                        map.remove(&bh);
                    }
                    g.bytes -= gone.key.len();
                }
            }
        }
        value
    }

    /// A call made anew: counted for the calling thread's group (`computed`).
    fn count(&self) {
        let k = (group(), self as *const Memo<V> as usize);
        let mut c = COMPUTED.lock().unwrap_or_else(|e| e.into_inner());
        *c.get_or_insert_with(HashMap::new).entry(k).or_insert(0) += 1;
    }

    /// How many calls this thread made anew (not taken from the memo), with those made for it
    /// on other threads (`Context`): for the tests, which run in parallel, each counting only
    /// its own calls.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn computed(&self) -> u64 {
        let k = (group(), self as *const Memo<V> as usize);
        let c = COMPUTED.lock().unwrap_or_else(|e| e.into_inner());
        c.as_ref().and_then(|c| c.get(&k)).copied().unwrap_or(0)
    }
}

impl<V: Clone> Default for Memo<V> {
    fn default() -> Self {
        Memo::new()
    }
}

/// Every memo's calls on this thread made anew while `on` (for the tests: a run through the
/// memos against the same run made fresh).
#[cfg_attr(not(test), allow(dead_code))]
pub fn bypass(on: bool) {
    BYPASS.with(|b| b.set(on));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Work handed to another thread under the caller's context counts as the caller's calls
    /// and keeps the caller's bypass; without it, the other thread's calls are its own.
    #[test]
    fn work_on_other_threads_counts_and_bypasses_as_the_callers() {
        static M: Memo<u64> = Memo::new();
        let on_other = |f: &(dyn Fn() -> u64 + Sync)| std::thread::scope(|s| s.spawn(f).join().unwrap());
        let mc = context();
        let n0 = M.computed();
        assert_eq!(on_other(&|| mc.enter(|| M.get_or("ctx a".into(), || 1))), 1);
        assert_eq!(M.computed(), n0 + 1, "made on another thread, for this one");
        // the same key again, elsewhere: taken from the memo
        assert_eq!(on_other(&|| mc.enter(|| M.get_or("ctx a".into(), || 2))), 1);
        assert_eq!(M.computed(), n0 + 1);
        // bypass on here: carried, so made anew there
        bypass(true);
        let mb = context();
        bypass(false);
        assert_eq!(on_other(&|| mb.enter(|| M.get_or("ctx a".into(), || 3))), 3);
        assert_eq!(M.computed(), n0 + 2, "the caller's bypass held there");
        // no context: the other thread's own calls
        assert_eq!(on_other(&|| M.get_or("ctx b".into(), || 4)), 4);
        assert_eq!(M.computed(), n0 + 2);
        // this thread's own state back after `enter`
        bypass(true);
        mc.enter(|| ());
        assert!(BYPASS.with(|b| b.get()), "enter left this thread's bypass changed");
        bypass(false);
    }
}

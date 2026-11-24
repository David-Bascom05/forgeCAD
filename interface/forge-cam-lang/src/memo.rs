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
    /// Calls made anew on this thread, per memo (by its address: the memos are statics).
    static COMPUTED: std::cell::RefCell<HashMap<usize, u64>> = std::cell::RefCell::new(HashMap::new());
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
        if key.contains("NaN") {
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

    /// A call made anew: counted on the calling thread (`computed`).
    fn count(&self) {
        let me = self as *const Memo<V> as usize;
        COMPUTED.with(|c| *c.borrow_mut().entry(me).or_insert(0) += 1);
    }

    /// How many calls this thread made anew (not taken from the memo): for the tests, which
    /// run in parallel, each counting only its own calls.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn computed(&self) -> u64 {
        let me = self as *const Memo<V> as usize;
        COMPUTED.with(|c| c.borrow().get(&me).copied().unwrap_or(0))
    }
}

impl<V: Clone> Default for Memo<V> {
    fn default() -> Self {
        Memo::new()
    }
}

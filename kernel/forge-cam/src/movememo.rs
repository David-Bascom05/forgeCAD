//! The certification's per-move memo (`ops::certify_with`): a move's checks taken from an earlier
//! certification when every input they read is the same, so an edit to a part re-certifies only
//! the moves whose faces it changed.
//!
//! A move's checks (`MoveOut`) are a function of: the move and its phase; the tool (its bodies,
//! whether it probes, the column the surfacing route uses); the tolerance; and, in visiting order,
//! each body's face sets with their limits and the faces it is checked against, the faces each
//! given in full. The scratch (`shared`) is per move and filled in that order, so the same plan
//! fills it the same way. The move's index in its path appears only in its checks' `move_index`,
//! put back on the way out.
//!
//! Keys hold no bare hash: a face's or a tool's text (its `Debug`, every field; f64 printed as its
//! shortest round trip, so two numbers that differ print differently) is interned to a number
//! that is never reused for another text, and a key's own text is compared whole. When the store
//! passes its size the interned texts and the memo are dropped together (numbers keep counting
//! up, so a number from before is never matched by a new text).
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether `certify_with` uses the memo (tests turn it off to compare against fresh work).
#[doc(hidden)]
pub static ON: AtomicBool = AtomicBool::new(true);

const MAX_BYTES: usize = 128 << 20;

struct Store<V> {
    texts: HashMap<String, u64>,
    next: u64,
    memo: HashMap<String, V>,
    bytes: usize,
    computed: u64,
    taken: u64,
}

pub struct MoveMemo<V> {
    inner: Mutex<Option<Store<V>>>,
}

impl<V: Clone> Default for MoveMemo<V> {
    fn default() -> MoveMemo<V> {
        MoveMemo::new()
    }
}

impl<V: Clone> MoveMemo<V> {
    pub const fn new() -> MoveMemo<V> {
        MoveMemo {
            inner: Mutex::new(None),
        }
    }

    fn with<R>(&self, f: impl FnOnce(&mut Store<V>) -> R) -> Option<R> {
        let mut g = self.inner.lock().ok()?;
        let s = g.get_or_insert_with(|| Store {
            texts: HashMap::new(),
            next: 0,
            memo: HashMap::new(),
            bytes: 0,
            computed: 0,
            taken: 0,
        });
        Some(f(s))
    }

    /// The number for a text: the same number for the same text (compared whole), a new one for a
    /// new text. None if the store is unavailable (a poisoned lock): the caller then works anew.
    pub fn intern(&self, text: String) -> Option<u64> {
        self.with(|s| {
            if let Some(&n) = s.texts.get(&text) {
                return n;
            }
            s.next += 1;
            let n = s.next;
            s.bytes += text.len();
            s.texts.insert(text, n);
            n
        })
    }

    /// The value for a key, if made before.
    pub fn get(&self, key: &str) -> Option<V> {
        self.with(|s| {
            let v = s.memo.get(key).cloned();
            if v.is_some() {
                s.taken += 1;
            }
            v
        })
        .flatten()
    }

    /// Keeps a value made for a key; past the size, everything is dropped first.
    pub fn put(&self, key: String, v: V) {
        self.with(|s| {
            s.computed += 1;
            if s.bytes + key.len() > MAX_BYTES {
                s.texts.clear();
                s.memo.clear();
                s.bytes = 0;
            }
            s.bytes += key.len();
            s.memo.insert(key, v);
        });
    }

    /// (moves worked out, moves taken from the memo) so far (tests).
    pub fn counts(&self) -> (u64, u64) {
        self.with(|s| (s.computed, s.taken)).unwrap_or((0, 0))
    }
}

/// Whether the memo is on: `ON`, and FORGE_CAM_NO_MOVE_MEMO not set (to compare against fresh
/// work outside tests).
pub fn on() -> bool {
    static ENV_OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    ON.load(Ordering::Relaxed)
        && !*ENV_OFF.get_or_init(|| std::env::var_os("FORGE_CAM_NO_MOVE_MEMO").is_some())
}

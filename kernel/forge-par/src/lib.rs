//! Scoped threads for every parallel path of the kernel. With no spawner installed (native, WASI,
//! the single-threaded browser build) `scope` is `std::thread::scope` and `Scope::spawn` its
//! `spawn`: the same code, the same threads. A host with threads std can't start (a browser page's
//! Web Workers on shared memory) installs a `Spawner`; then each spawned closure is handed to it
//! and every one is joined before `scope` returns, panics included, as std joins its own. On a
//! thread that may not wait (a page's main thread: `memory.atomic.wait` traps there) each closure
//! runs at once, where it is spawned. A result never depends on which: the kernel's parallel
//! paths take items in any order and put the answers back in theirs.
//!
//! One difference from std's signature: the closure is given `&Scope<'scope, 'env>`, a reference
//! of any lifetime (std's is `&'scope`), so the std path wraps std's scope with no unsafe code.
//! The one `unsafe` block is the lifetime erasure in `Scope::spawn` (see its SAFETY).

use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};

/// A job for a host's thread.
pub type Job = Box<dyn FnOnce() + Send + 'static>;

/// A host's threads.
#[derive(Clone, Copy)]
pub struct Spawner {
    /// Runs the job on another thread (now or soon; never on the caller's, which may wait for it).
    pub spawn: fn(Job),
    /// Whether this thread may wait for another.
    pub can_block: fn() -> bool,
    /// How many threads a parallel path should use.
    pub threads: fn() -> usize,
}

static SPAWNER: OnceLock<Spawner> = OnceLock::new();

/// Installs the host's spawner, once (false if one was already).
pub fn set_spawner(s: Spawner) -> bool {
    SPAWNER.set(s).is_ok()
}

/// The installed spawner.
pub fn spawner() -> Option<Spawner> {
    SPAWNER.get().copied()
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// What a hooked scope waits for: how many of its closures are still running, and how many
/// panicked unjoined.
#[derive(Default)]
struct State {
    live: Mutex<usize>,
    done: Condvar,
    panics: AtomicUsize,
}

impl State {
    fn wait_all(&self) {
        let mut live = lock(&self.live);
        while *live > 0 {
            live = self.done.wait(live).unwrap_or_else(|e| e.into_inner());
        }
    }
}

/// Waits for every closure of a hooked scope when dropped: on the way out of `scope`, panics
/// included.
struct JoinAll<'a>(&'a State);

impl Drop for JoinAll<'_> {
    fn drop(&mut self) {
        self.0.wait_all();
    }
}

/// One closure's answer, when it has one.
struct Packet<T> {
    slot: Mutex<Option<std::thread::Result<T>>>,
    ready: Condvar,
}

enum How<'scope, 'env> {
    Std(&'scope std::thread::Scope<'scope, 'env>),
    Hooked(Spawner, Arc<State>),
    /// A thread that may not wait: each closure runs where it is spawned.
    Inline,
}

pub struct Scope<'scope, 'env: 'scope> {
    how: How<'scope, 'env>,
    _scope: PhantomData<&'scope mut &'env ()>,
}

/// A spawned closure, to join.
pub struct ScopedJoinHandle<'scope, T>(Handle<'scope, T>);

enum Handle<'scope, T> {
    Std(std::thread::ScopedJoinHandle<'scope, T>),
    Hooked(Arc<Packet<T>>, Arc<State>, PhantomData<&'scope ()>),
    Inline(std::thread::Result<T>),
}

/// A scope whose spawned closures may borrow from outside it, all joined before it returns.
pub fn scope<'env, F, T>(f: F) -> T
where
    F: for<'scope> FnOnce(&Scope<'scope, 'env>) -> T,
{
    let Some(sp) = spawner() else {
        // WASI (wasip1) starts no threads: every spawn runs here, in order, as on a host thread
        // that may not wait (results do not depend on the thread count)
        if cfg!(target_os = "wasi") {
            return f(&Scope {
                how: How::Inline,
                _scope: PhantomData,
            });
        }
        return std::thread::scope(|s| {
            f(&Scope {
                how: How::Std(s),
                _scope: PhantomData,
            })
        });
    };
    if !(sp.can_block)() {
        return f(&Scope {
            how: How::Inline,
            _scope: PhantomData,
        });
    }
    let state = Arc::new(State::default());
    let got = {
        let _join = JoinAll(&state);
        let s = Scope {
            how: How::Hooked(sp, state.clone()),
            _scope: PhantomData,
        };
        catch_unwind(AssertUnwindSafe(|| f(&s)))
    };
    match got {
        Err(e) => resume_unwind(e),
        Ok(_) if state.panics.load(Ordering::SeqCst) > 0 => panic!("a scoped thread panicked"),
        Ok(v) => v,
    }
}

impl<'scope, 'env> Scope<'scope, 'env> {
    /// `f` on another thread (std's, the host's), or here (a thread that may not wait).
    pub fn spawn<F, T>(&self, f: F) -> ScopedJoinHandle<'scope, T>
    where
        F: FnOnce() -> T + Send + 'scope,
        T: Send + 'scope,
    {
        let (sp, state) = match &self.how {
            How::Std(s) => return ScopedJoinHandle(Handle::Std(s.spawn(f))),
            How::Inline => return ScopedJoinHandle(Handle::Inline(catch_unwind(AssertUnwindSafe(f)))),
            How::Hooked(sp, state) => (*sp, state.clone()),
        };
        let packet = Arc::new(Packet {
            slot: Mutex::new(None),
            ready: Condvar::new(),
        });
        *lock(&state.live) += 1;
        let (theirs, st) = (packet.clone(), state.clone());
        let job: Box<dyn FnOnce() + Send + 'scope> = Box::new(move || {
            let r = catch_unwind(AssertUnwindSafe(f));
            if r.is_err() {
                st.panics.fetch_add(1, Ordering::SeqCst);
            }
            *lock(&theirs.slot) = Some(r);
            theirs.ready.notify_all();
            // the last of what this closure borrows ('scope) goes before the scope is told
            drop(theirs);
            let mut live = lock(&st.live);
            *live -= 1;
            st.done.notify_all();
        });
        // SAFETY: the job is let outlive 'scope only in its type. Every job spawned in this scope
        // is counted in `state.live` before it is handed over, and `scope` does not return (nor
        // unwind) until `JoinAll` has seen the count fall to 0. The count falls as the very last
        // step of the job, after `f` has run (or panicked: `catch_unwind`) and been dropped, and
        // after the packet holding its `'scope` answer was let go by the job; what the job still
        // holds then (`st`) is `'static`. So nothing of `'scope` is touched after `scope` returns.
        let job: Job = unsafe { std::mem::transmute::<Box<dyn FnOnce() + Send + 'scope>, Job>(job) };
        (sp.spawn)(job);
        ScopedJoinHandle(Handle::Hooked(packet, state, PhantomData))
    }
}

impl<T> ScopedJoinHandle<'_, T> {
    /// The closure's answer, or its panic, once it has finished.
    pub fn join(self) -> std::thread::Result<T> {
        match self.0 {
            Handle::Std(h) => h.join(),
            Handle::Inline(r) => r,
            Handle::Hooked(p, state, _) => {
                let mut slot = lock(&p.slot);
                let r = loop {
                    if let Some(r) = slot.take() {
                        break r;
                    }
                    slot = p.ready.wait(slot).unwrap_or_else(|e| e.into_inner());
                };
                // a panic seen by its joiner is the joiner's to raise, not the scope's
                if r.is_err() {
                    state.panics.fetch_sub(1, Ordering::SeqCst);
                }
                r
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        match &self.0 {
            Handle::Std(h) => h.is_finished(),
            Handle::Inline(_) => true,
            Handle::Hooked(p, _, _) => lock(&p.slot).is_some(),
        }
    }
}

#[cfg(test)]
mod tests;

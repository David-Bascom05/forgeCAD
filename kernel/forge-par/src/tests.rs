//! The hooked path, with a test spawner on std's threads (the std path, with none installed, is
//! tests/std_path.rs: a process of its own). A thread-local says whether this thread may wait, to
//! test the inline path too.

use super::*;
use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

thread_local! {
    static MAY_WAIT: Cell<bool> = const { Cell::new(true) };
}

fn hooked() {
    set_spawner(Spawner {
        spawn: |j| {
            std::thread::spawn(j);
        },
        can_block: || MAY_WAIT.with(Cell::get),
        threads: || 4,
    });
    assert!(spawner().is_some());
}

#[test]
// runs on the hook spawner's std threads: WASI (wasip1) starts none
#[cfg(not(target_os = "wasi"))]
fn every_closure_runs_and_answers_in_its_own_handle() {
    hooked();
    let xs: Vec<u64> = (0..50).collect();
    let got: Vec<u64> = scope(|s| {
        let hs: Vec<_> = xs.iter().map(|x| s.spawn(move || x * x)).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert_eq!(got, xs.iter().map(|x| x * x).collect::<Vec<_>>());
}

#[test]
// runs on the hook spawner's std threads: WASI (wasip1) starts none
#[cfg(not(target_os = "wasi"))]
fn borrows_are_never_outlived_by_a_closure_left_unjoined() {
    hooked();
    let mut slots = vec![0u64; 16];
    scope(|s| {
        for (i, slot) in slots.iter_mut().enumerate() {
            s.spawn(move || {
                // late, so a scope that didn't wait would return first
                std::thread::sleep(Duration::from_millis(5 + (i as u64 % 4) * 5));
                *slot = i as u64 + 1;
            });
        }
    });
    assert_eq!(slots, (1..=16).collect::<Vec<u64>>());
}

#[test]
// a panic caught on purpose: under WASI a panic aborts
#[cfg(not(target_os = "wasi"))]
fn a_panicking_closure_joins_everything_before_the_scope_returns() {
    hooked();
    let done: Vec<AtomicBool> = (0..8).map(|_| AtomicBool::new(false)).collect();
    let r = catch_unwind(AssertUnwindSafe(|| {
        scope(|s| {
            for (i, d) in done.iter().enumerate() {
                s.spawn(move || {
                    if i == 3 {
                        panic!("the third");
                    }
                    std::thread::sleep(Duration::from_millis(20));
                    d.store(true, Ordering::SeqCst);
                });
            }
        })
    }));
    assert!(r.is_err(), "an unjoined panic is the scope's");
    let all: Vec<bool> = done.iter().map(|d| d.load(Ordering::SeqCst)).collect();
    assert_eq!(all, [true, true, true, false, true, true, true, true]);
}

#[test]
// a panic caught on purpose: under WASI a panic aborts
#[cfg(not(target_os = "wasi"))]
fn a_panic_in_the_scope_itself_still_waits_for_its_closures() {
    hooked();
    let done = AtomicBool::new(false);
    let r = catch_unwind(AssertUnwindSafe(|| {
        scope(|s| {
            s.spawn(|| {
                std::thread::sleep(Duration::from_millis(30));
                done.store(true, Ordering::SeqCst);
            });
            panic!("the scope's own");
        })
    }));
    assert!(r.is_err());
    assert!(done.load(Ordering::SeqCst), "the closure ran to its end first");
}

#[test]
// a panic caught on purpose: under WASI a panic aborts
#[cfg(not(target_os = "wasi"))]
fn a_joined_panic_is_the_joiners_not_the_scopes() {
    hooked();
    let got = scope(|s| s.spawn(|| -> u8 { panic!("mine") }).join().is_err());
    assert!(got);
}

#[test]
// runs on the hook spawner's std threads: WASI (wasip1) starts none
#[cfg(not(target_os = "wasi"))]
fn scopes_nest() {
    hooked();
    let total = AtomicUsize::new(0);
    scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                scope(|inner| {
                    for k in 1..=3 {
                        let total = &total;
                        inner.spawn(move || total.fetch_add(k, Ordering::SeqCst));
                    }
                });
            });
        }
    });
    assert_eq!(total.load(Ordering::SeqCst), 4 * 6);
}

#[test]
fn a_thread_that_may_not_wait_runs_each_closure_where_it_is_spawned() {
    hooked();
    MAY_WAIT.with(|m| m.set(false));
    let here = std::thread::current().id();
    let ran_on: Vec<_> = scope(|s| {
        (0..3)
            .map(|_| s.spawn(|| std::thread::current().id()))
            .map(|h| h.join().unwrap())
            .collect()
    });
    MAY_WAIT.with(|m| m.set(true));
    assert!(ran_on.iter().all(|t| *t == here));
}

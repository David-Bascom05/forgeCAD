//! With no spawner installed (a process of its own: the unit tests install one) a scope is std's:
//! its closures run on std's threads, borrow, and are joined.

use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn with_no_spawner_a_scope_is_stds() {
    assert!(forge_par::spawner().is_none());
    let here = std::thread::current().id();
    let mut slots = [0usize; 8];
    let others = AtomicUsize::new(0);
    forge_par::scope(|s| {
        for (i, slot) in slots.iter_mut().enumerate() {
            let others = &others;
            s.spawn(move || {
                *slot = i * 2;
                if std::thread::current().id() != here {
                    others.fetch_add(1, Ordering::SeqCst);
                }
            });
        }
    });
    assert_eq!(slots, [0, 2, 4, 6, 8, 10, 12, 14]);
    assert_eq!(others.load(Ordering::SeqCst), 8, "each on a thread of its own");
    let r = std::panic::catch_unwind(|| forge_par::scope(|s| s.spawn(|| panic!("std's")).join().is_err()));
    assert_eq!(r.ok(), Some(true));
}

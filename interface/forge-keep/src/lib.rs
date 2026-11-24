//! Values kept on a thread until they are let go all at once: forge-lang keeps each sketch it
//! evaluates here (a sketch with a named entity is an `Rc` cycle, which outlives its run), and the
//! app lets them go between builds, so a run itself frees nothing and does no more than it did.
//! Its own crate: thread-local code inside forge-lang moved how that crate was compiled (string
//! matches and inlining elsewhere in it), and with it the speed gate's counts.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::mem::ManuallyDrop;
use std::rc::Rc;

/// How many are kept in place (a run between two releases mostly makes fewer); past them, a list
/// on the heap: one grown mid-run moved where the run's own blocks went.
const SLOTS: usize = 64;

type Slots = [Cell<Option<Rc<dyn Any>>>; SLOTS];

thread_local! {
    // all zeros to start and no destructor: nothing copied into a new thread, nothing run at its end
    static COUNT: Cell<usize> = const { Cell::new(0) };
    static SLOT: ManuallyDrop<Slots> =
        const { ManuallyDrop::new([const { Cell::new(None) }; SLOTS]) };
    static MORE: ManuallyDrop<RefCell<Vec<Rc<dyn Any>>>> = const { ManuallyDrop::new(RefCell::new(Vec::new())) };
}

/// Keeps `x` on this thread until `take_all`.
pub fn keep(x: Rc<dyn Any>) {
    let n = COUNT.get();
    if n < SLOTS {
        SLOT.with(|s| s[n].set(Some(x)));
        COUNT.set(n + 1);
    } else {
        MORE.with(|m| m.borrow_mut().push(x));
    }
}

/// How many are kept on this thread (what a long-lived loop's tests hold bounded).
pub fn kept() -> usize {
    COUNT.get() + MORE.with(|m| m.borrow().len())
}

/// Everything kept on this thread so far, no longer kept.
pub fn take_all() -> Vec<Rc<dyn Any>> {
    let n = COUNT.replace(0);
    let mut all: Vec<Rc<dyn Any>> = SLOT.with(|s| s[..n].iter().filter_map(Cell::take).collect());
    MORE.with(|m| all.append(&mut m.borrow_mut()));
    all
}

#[cfg(test)]
mod tests {
    #[test]
    fn kept_till_taken_in_place_and_past_the_slots() {
        let probes: Vec<std::rc::Rc<usize>> = (0..super::SLOTS + 5).map(std::rc::Rc::new).collect();
        for p in &probes {
            super::keep(p.clone());
        }
        assert!(probes.iter().all(|p| std::rc::Rc::strong_count(p) == 2));
        let all = super::take_all();
        assert_eq!(all.len(), probes.len());
        drop(all);
        assert!(probes.iter().all(|p| std::rc::Rc::strong_count(p) == 1));
        assert!(super::take_all().is_empty());
    }
}

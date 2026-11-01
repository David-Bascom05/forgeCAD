//! An n-ary regen's result does not depend on its threads or on how they are scheduled: which thread
//! takes which pair, and what that thread remembers from before, changes no bit. A guard only: it passed with
//! the near match in `ssi::searched` too, which `bits` showed under load. The test that fails on that cause is
//! `a_box_a_little_off_is_another_question` in `ssi.rs` (research/drag-speed).
#![cfg(not(target_family = "wasm"))]
use forge_math::hash::BitHasher;
use forge_model::{Part, suite};

fn bits(part: &Part, threads: usize) -> u64 {
    let mut h = BitHasher::new();
    match part.regenerate_nary(threads) {
        Ok(b) => {
            b.vertices.iter().for_each(|v| h.p3(v.point));
            for e in &b.edges {
                h.f64(e.range.lo);
                h.f64(e.range.hi);
            }
            h.u64(b.faces.len() as u64);
        }
        Err(_) => h.u64(u64::MAX),
    }
    h.finish()
}

#[test]
fn threads_and_load_change_no_bit() {
    let stop = std::sync::atomic::AtomicBool::new(false);
    let mut parts: Vec<(String, Part)> = suite::parts()
        .into_iter()
        .filter(|(n, _)| ["plate", "housing", "mixed"].contains(n))
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    parts.extend([1, 8].map(|s| (format!("fuzz{s}"), suite::fuzz(s, 80))));
    std::thread::scope(|s| {
        // the load: threads that only take the processors from it, so helpers start at odd moments
        for _ in 0..8 {
            s.spawn(|| {
                let mut x = 1u64;
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    x = x.wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(7);
                }
                x
            });
        }
        let held = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for (name, part) in &parts {
                let one = bits(part, 1);
                for threads in [2, 4, 4, 4] {
                    assert_eq!(bits(part, threads), one, "{name} on {threads} threads");
                }
            }
        }));
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Err(p) = held {
            std::panic::resume_unwind(p);
        }
    });
}

//! Phase timing for speed work (research/cam-speed): off unless `FORGE_CAM_PROF` is set, when
//! each named span's wall time is summed per name (`take` hands them over and clears them).
//! Off, a span is one relaxed load and no clock read: it never touches what a run makes.
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};

/// 0 not yet read, 1 off, 2 on
static ON: AtomicU8 = AtomicU8::new(0);
static SUMS: Mutex<Vec<(String, f64, u64)>> = Mutex::new(Vec::new());

/// Whether spans are being timed (`FORGE_CAM_PROF` set when first asked).
pub fn on() -> bool {
    match ON.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => {
            let v = std::env::var_os("FORGE_CAM_PROF").is_some();
            ON.store(if v { 2 } else { 1 }, Ordering::Relaxed);
            v
        }
    }
}

/// A span: its time is added to its name's sum when it is dropped.
pub struct Span(Option<(String, forge_math::clock::Instant)>);

/// A span named by `name()` (called only when timing is on).
pub fn span(name: impl FnOnce() -> String) -> Span {
    Span(on().then(|| (name(), forge_math::clock::Instant::now())))
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some((n, t0)) = self.0.take() {
            let dt = t0.elapsed().as_secs_f64();
            let mut s = SUMS.lock().unwrap_or_else(|e| e.into_inner());
            match s.iter_mut().find(|x| x.0 == n) {
                Some(x) => {
                    x.1 += dt;
                    x.2 += 1;
                }
                None => s.push((n, dt, 1)),
            }
        }
    }
}

/// The sums so far, in first-seen order (name, seconds, count); cleared.
pub fn take() -> Vec<(String, f64, u64)> {
    std::mem::take(&mut *SUMS.lock().unwrap_or_else(|e| e.into_inner()))
}

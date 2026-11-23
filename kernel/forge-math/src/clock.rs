//! The clock, for timings only (no result depends on it). Natively, and on WASI, it is std's;
//! a browser has no clock std can read (`Instant::now` panics on wasm32-unknown-unknown), so there
//! the host hands one over with `set` (the page's `performance.now()` and `Date.now()`), and until
//! it does time stands still.

pub use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use std::time::Instant;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use web::{Instant, set};

/// The time of day (`SystemTime::now`, which panics in a browser).
pub fn system_now() -> SystemTime {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        SystemTime::now()
    }
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        web::system_now()
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod web {
    use std::sync::OnceLock;
    use std::time::Duration;

    /// The host's clocks: milliseconds, monotonic; milliseconds since 1970.
    static CLOCKS: OnceLock<(fn() -> f64, fn() -> f64)> = OnceLock::new();

    /// Gives the clock its sources, once.
    pub fn set(monotonic_ms: fn() -> f64, unix_ms: fn() -> f64) {
        let _ = CLOCKS.set((monotonic_ms, unix_ms));
    }

    fn ms(f: impl Fn(&(fn() -> f64, fn() -> f64)) -> f64) -> f64 {
        CLOCKS.get().map_or(0.0, f)
    }

    fn span(ms: f64) -> Duration {
        Duration::from_secs_f64(ms.max(0.0) / 1e3)
    }

    #[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
    pub struct Instant(f64);

    impl Instant {
        pub fn now() -> Instant {
            Instant(ms(|c| (c.0)()))
        }
        pub fn elapsed(&self) -> Duration {
            span(Instant::now().0 - self.0)
        }
        pub fn duration_since(&self, earlier: Instant) -> Duration {
            span(self.0 - earlier.0)
        }
        pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
            self.duration_since(earlier)
        }
    }

    impl std::ops::Add<Duration> for Instant {
        type Output = Instant;
        fn add(self, d: Duration) -> Instant {
            Instant(self.0 + d.as_secs_f64() * 1e3)
        }
    }

    impl std::ops::Sub<Instant> for Instant {
        type Output = Duration;
        fn sub(self, o: Instant) -> Duration {
            self.duration_since(o)
        }
    }

    pub fn system_now() -> std::time::SystemTime {
        std::time::UNIX_EPOCH + span(ms(|c| (c.1)()))
    }
}

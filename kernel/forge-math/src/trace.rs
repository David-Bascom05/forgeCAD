//! Where the time goes, on stderr when `FORGECAD_TRACE` is set: `let _t = trace::span("hlr scene");`
//! prints the span's name and milliseconds when it ends.

use std::sync::OnceLock;
use std::time::Instant;

pub fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("FORGECAD_TRACE").is_some())
}

pub struct Span(&'static str, Option<Instant>);

pub fn span(name: &'static str) -> Span {
    Span(name, on().then(Instant::now))
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some(t) = self.1 {
            eprintln!("trace {} {:.1} ms", self.0, t.elapsed().as_secs_f64() * 1e3);
        }
    }
}

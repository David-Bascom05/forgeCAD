//! The n-ary regen's classify step with and without the tools' analytic solids.
use forge_model::{suite, torture};
use std::time::Instant;

fn main() {
    let threads: usize = std::env::var("THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(16);
    let mut parts = suite::parts();
    parts.push(("torture", torture::part(221)));
    for (name, part) in parts {
        for shapes in [false, true] {
            let mut best = f64::INFINITY;
            let mut cls = 0.0;
            let mut pick = 0.0;
            let mut merge = 0.0;
            let mut ok = false;
            for _ in 0..5 {
                let t = Instant::now();
                let r = part.regenerate_nary_with(threads, shapes);
                let ms = t.elapsed().as_secs_f64() * 1e3;
                if ms < best {
                    best = ms;
                    let ph = forge_bool::nary_phase_times();
                    cls = ph[4] as f64 / 1e6;
                    pick = ph[6] as f64 / 1e6;
                    merge = ph[7] as f64 / 1e6;
                }
                ok = r.is_ok_and(|b| forge_topo::check(&b).is_empty());
            }
            println!(
                "{name:<10} shapes {shapes:<5} total {best:>8.2} ms  classify {cls:>7.2} ms (picking points {pick:.2} cpu-ms, merge {merge:.2})  {}",
                if ok { "ok" } else { "FAIL" }
            );
        }
    }
}

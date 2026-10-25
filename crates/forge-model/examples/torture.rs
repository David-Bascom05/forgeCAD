use forge_model::torture;
use std::time::Instant;
fn main() {
    let all = torture::part(usize::MAX).features.len();
    let part = torture::part(all);
    let t = Instant::now();
    match part.regenerate() {
        Ok(b) => {
            let dt = t.elapsed();
            println!(
                "{all} features: {:.2} ms, {} faces {} edges, problems {}",
                dt.as_secs_f64() * 1e3,
                b.faces.len(),
                b.edges.len(),
                if std::env::var("NO_CHECK").is_ok() {
                    0
                } else {
                    forge_topo::check(&b).len()
                }
            );
            if std::env::var_os("FORGE_PHASES").is_some() {
                let names = [
                    "intersect",
                    "imprint",
                    "place+pieces",
                    "split",
                    "classify",
                    "keep",
                    "apply",
                ];
                for (name, ns) in names.iter().zip(forge_bool::phase_times()) {
                    println!("  {name:<13} {:8.2} ms", ns as f64 / 1e6);
                }
            }
        }
        Err(e) => {
            println!("failed after {:.2} ms: {e:?}", t.elapsed().as_secs_f64() * 1e3);
            // find the first failing prefix
            let (mut lo, mut hi) = (1, all);
            while lo < hi {
                let mid = (lo + hi) / 2;
                if torture::part(mid).regenerate().is_ok() {
                    lo = mid + 1
                } else {
                    hi = mid
                }
            }
            println!(
                "first failing feature count: {lo} ({:?})",
                torture::part(lo).features.last()
            );
        }
    }
}

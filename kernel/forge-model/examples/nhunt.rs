//! The adversarial families through both regens: how many parts each handles (a valid solid,
//! or a zero-thickness refusal in a family that touches by design; `zt` counts the refusals).
//! cargo run --release -p forge-model --example nhunt -- [seeds per family]
use forge_model::adversarial::{canonical, families, touches_by_design};
use forge_model::{Part, suite};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

/// (valid, refused for zero thickness, ms)
fn valid(f: impl FnOnce() -> Result<forge_topo::Body, forge_model::RegenError>) -> (bool, bool, f64) {
    let t = Instant::now();
    let r = catch_unwind(AssertUnwindSafe(f)).ok();
    let zt = matches!(
        r,
        Some(Err(forge_model::RegenError::Boolean {
            error: forge_bool::BoolError::ZeroThickness { .. },
            ..
        }))
    );
    let ok = r
        .and_then(|r| r.ok())
        .is_some_and(|b| forge_topo::check(&b).is_empty());
    (ok, zt, t.elapsed().as_secs_f64() * 1e3)
}

type Generator = Box<dyn Fn(u64) -> Part>;

fn main() {
    let seeds: u64 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(80);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    std::panic::set_hook(Box::new(|_| {}));
    let mut gens: Vec<(String, Generator)> = families()
        .into_iter()
        .map(|(n, f)| (n.to_string(), Box::new(f) as Generator))
        .collect();
    gens.push(("fuzz".to_string(), Box::new(|s| suite::fuzz(s + 1, 40))));
    println!(
        "{:<22} {:>6} {:>6} {:>6} {:>7} {:>7} {:>5} {:>5} {:>9} {:>9}",
        "family", "parts", "seq", "nary", "only s", "only n", "zt s", "zt n", "seq ms", "nary ms"
    );
    let mut tot = [0usize; 7];
    let (mut ts, mut tn) = (0.0, 0.0);
    for (name, g) in &gens {
        let mut row = [0usize; 7];
        let by_design = touches_by_design(name);
        let (mut fs, mut fnn) = (0.0, 0.0);
        for seed in 0..seeds {
            let part = canonical(&g(seed));
            let (s, zs, a) = valid(|| part.regenerate());
            let (n, zn, b) = valid(|| part.regenerate_nary(threads));
            let (s, n) = (s || zs && by_design, n || zn && by_design);
            row[5] += zs as usize;
            row[6] += zn as usize;
            fs += a;
            fnn += b;
            row[0] += 1;
            row[1] += s as usize;
            row[2] += n as usize;
            row[3] += (s && !n) as usize;
            row[4] += (n && !s) as usize;
            if s && !n && std::env::var_os("SHOW").is_some() {
                println!("    {name} seed {seed}: nary fails");
            }
        }
        println!(
            "{name:<22} {:>6} {:>6} {:>6} {:>7} {:>7} {:>5} {:>5} {fs:>9.1} {fnn:>9.1}",
            row[0], row[1], row[2], row[3], row[4], row[5], row[6]
        );
        for k in 0..7 {
            tot[k] += row[k];
        }
        ts += fs;
        tn += fnn;
    }
    println!(
        "{:<22} {:>6} {:>6} {:>6} {:>7} {:>7} {:>5} {:>5} {ts:>9.1} {tn:>9.1}",
        "total", tot[0], tot[1], tot[2], tot[3], tot[4], tot[5], tot[6]
    );
}

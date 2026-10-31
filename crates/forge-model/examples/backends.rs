//! Whole n-ary regens of every suite part on each batch backend (cpu, gpu, auto, or only those
//! FORGE_BATCH names): median and best of `REPS` (default 15) in ms on `THREADS` (default 4)
//! threads, the batch call alone in µs, and whether every backend gives the same exact bits.
//! Also the crossover Auto measured.
use forge_math::hash::BitHasher;
use forge_model::{Backend, RegenError, suite, torture};
use std::time::Instant;

fn bits(r: &Result<forge_topo::Body, RegenError>) -> u64 {
    let mut h = BitHasher::new();
    match r {
        Ok(b) => {
            for v in &b.vertices {
                h.p3(v.point);
            }
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

fn main() {
    let env = |k: &str, d: usize| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let (reps, threads) = (env("REPS", 15), env("THREADS", 4));
    let backends = match std::env::var("FORGE_BATCH") {
        Ok(_) => vec![Backend::from_env()],
        Err(_) => vec![Backend::Cpu, Backend::Gpu, Backend::Auto],
    };
    if let Some(g) = forge_gpu::gpu() {
        println!("{}", g.name);
    }
    let mut parts = suite::parts();
    parts.push(("torture", torture::part(221)));
    print!("{:<9}", "ms/µs");
    for b in &backends {
        print!(" {:>24}", format!("{b:?} med/best | batch"));
    }
    println!();
    let mut sums = vec![0.0; backends.len()];
    for (name, part) in &parts {
        let mut hashes = Vec::new();
        let mut t = vec![Vec::new(); backends.len()];
        let mut batch = vec![Vec::new(); backends.len()];
        // the backends take turns each round, so load on the machine falls on all alike
        for _ in 0..reps {
            for (k, &b) in backends.iter().enumerate() {
                let s = Instant::now();
                let r = part.regenerate_nary_on(threads, b);
                t[k].push(s.elapsed().as_secs_f64() * 1e3);
                batch[k].push(forge_bool::nary_phase_times()[8] as f64 / 1e3);
                hashes.push(bits(&r));
            }
        }
        print!("{name:<9}");
        for k in 0..backends.len() {
            t[k].sort_by(f64::total_cmp);
            batch[k].sort_by(f64::total_cmp);
            sums[k] += t[k][reps / 2];
            print!(
                " {:>8.2}/{:<7.2}| {:>6.0}",
                t[k][reps / 2],
                t[k][0],
                batch[k][reps / 2]
            );
        }
        let same = hashes.iter().all(|&h| h == hashes[0]);
        println!("  {}", if same { "same bits" } else { "DIFFERENT" });
        assert!(same, "{name}: a backend changed a regen's result");
    }
    print!("{:<9}", "sum");
    for s in sums {
        print!(" {s:>8.2}{:16}", "");
    }
    println!();
    match forge_gpu::crossover() {
        Some(usize::MAX) => println!("auto crossover: the GPU never won"),
        Some(n) => println!("auto crossover: {n} points"),
        None => println!("auto crossover: not probed"),
    }
}

//! The n-ary regen's region verdicts on the CPU and on the GPU: time, validity, agreement.
use forge_bool::{Batch, CpuBatch};
use forge_gpu::{Gpu, GpuBatch, Packed};
use forge_model::{Part, suite, torture};
use std::time::Instant;

fn best<T>(k: usize, mut f: impl FnMut() -> T) -> (f64, T) {
    let mut out = None;
    let mut best = f64::INFINITY;
    for _ in 0..k {
        let t = Instant::now();
        let r = f();
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
        out = Some(r);
    }
    (best, out.expect("ran"))
}

fn main() {
    let t = Instant::now();
    let gpu = Gpu::new().expect("a CUDA GPU");
    println!(
        "{} ready in {:.0} ms (context + PTX JIT, once)",
        gpu.name,
        t.elapsed().as_secs_f64() * 1e3
    );
    let threads: usize = std::env::var("THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(16);
    let mut parts: Vec<(String, Part)> = suite::parts()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    parts.push(("torture".into(), torture::part(221)));
    println!(
        "{:<10} {:>9} {:>9} {:>9}   {:>9} {:>9}  {:>7} {:>6}  result",
        "part", "cpu ms", "cpubat ms", "gpu ms", "cpu bat", "gpu bat", "regions", "undec"
    );
    for (name, part) in parts {
        let (shapes, ops) = part.nary_solids();
        let cpu_batch = CpuBatch {
            shapes: &shapes,
            ops: &ops,
            threads,
        };
        let gpu_batch = GpuBatch {
            gpu: &gpu,
            packed: Packed::new(&shapes, &ops),
            single: true,
        };
        let (tc, rc) = best(5, || part.regenerate_nary_with(threads, true));
        let (tcb, _) = best(5, || part.regenerate_nary_batch(threads, true, Some(&cpu_batch)));
        let bc = forge_bool::nary_phase_times()[8] as f64 / 1e6;
        let (tg, rg) = best(5, || part.regenerate_nary_batch(threads, true, Some(&gpu_batch)));
        let bg = forge_bool::nary_phase_times()[8] as f64 / 1e6;
        // the two batches must agree exactly: same points through both
        let pts: Vec<forge_math::Point3> = (0..4000)
            .map(|i| {
                let f = i as f64;
                forge_math::Point3::new((f * 7.31) % 300.0, (f * 3.77) % 200.0, (f * 1.13) % 40.0 - 5.0)
            })
            .collect();
        let sides: Vec<u32> = (0..pts.len()).map(|i| (i % shapes.len()) as u32).collect();
        let boxes: Vec<forge_math::Box3> =
            vec![forge_math::Box3::from_point(forge_math::Point3::ORIGIN).grow(1e4); shapes.len()];
        let a = cpu_batch.verdicts(&pts, &sides, &boxes);
        let b = gpu_batch.verdicts(&pts, &sides, &boxes);
        let differ = a.iter().zip(&b).filter(|(x, y)| x != y).count();
        let same = match (&rc, &rg) {
            (Ok(x), Ok(y)) => x.face_ids().count() == y.face_ids().count() && forge_topo::check(y).is_empty(),
            _ => false,
        };
        println!(
            "{name:<10} {tc:>9.2} {tcb:>9.2} {tg:>9.2}   {bc:>9.3} {bg:>9.3}  {:>13}  {} (batch disagreements on 4000 probes: {differ})",
            "",
            if same { "same solid" } else { "DIFFERENT" }
        );
    }
}

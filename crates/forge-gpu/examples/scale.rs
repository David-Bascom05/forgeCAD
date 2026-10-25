//! The verdict batch alone, CPU (all threads) against GPU, as the number of points grows:
//! random points in the torture plate tested against all 221 tools.
use forge_bool::{Batch, CpuBatch};
use forge_gpu::{Gpu, GpuBatch, Packed};
use forge_math::rng::Rng;
use forge_math::{Box3, Point3};
use std::time::Instant;

fn main() {
    let gpu = Gpu::new().expect("a CUDA GPU");
    let part = forge_model::torture::part(221);
    let (shapes, ops) = part.nary_solids();
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let cpu = CpuBatch {
        shapes: &shapes,
        ops: &ops,
        threads,
    };
    let g = GpuBatch {
        gpu: &gpu,
        packed: Packed::new(&shapes, &ops),
        single: false,
    };
    let g32 = GpuBatch {
        gpu: &gpu,
        packed: Packed::new(&shapes, &ops),
        single: true,
    };
    // the whole plate as every tool's box: no culling, the worst case for both
    let boxes = vec![Box3::from_point(Point3::ORIGIN).grow(1e4); shapes.len()];
    let mut r = Rng::new(5);
    println!(
        "{:>9} {:>10} {:>12} {:>12} {:>8} {:>10} {:>12}",
        "points", "cpu ms", "gpu f64 ms", "gpu f32 ms", "agree", "f32 undec", "f32 wrong"
    );
    for n in [1_000usize, 10_000, 100_000, 1_000_000] {
        let pts: Vec<Point3> = (0..n)
            .map(|_| Point3::new(r.range(0.0, 300.0), r.range(0.0, 200.0), r.range(-2.0, 30.0)))
            .collect();
        let sides: Vec<u32> = (0..n).map(|i| (i % shapes.len()) as u32).collect();
        let time = |b: &dyn Batch| {
            let mut best = f64::INFINITY;
            let mut out = Vec::new();
            for _ in 0..3 {
                let t = Instant::now();
                out = b.verdicts(&pts, &sides, &boxes);
                best = best.min(t.elapsed().as_secs_f64() * 1e3);
            }
            (best, out)
        };
        let (tc, a) = time(&cpu);
        let (tg, b) = time(&g);
        let (t32, c) = time(&g32);
        // single precision may leave a point undecided, never answer it differently
        let undec = c.iter().zip(&a).filter(|(x, y)| **x == 3 && **y != 3).count();
        let wrong = c.iter().zip(&a).filter(|(x, y)| **x != 3 && x != y).count();
        println!(
            "{n:>9} {tc:>10.2} {tg:>12.2} {t32:>12.2} {:>8} {undec:>10} {wrong:>12}",
            a == b
        );
    }
}

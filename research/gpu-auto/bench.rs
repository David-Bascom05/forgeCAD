//! The GPU batch against the CPU batch. Times are best and median of `REPS` (default 30) in µs,
//! on `THREADS` (default 4) CPU threads.
//!
//! First the crossover: each part's regen batch repeated or cut to sizes from 16 points up,
//! each call after an idle gap with the GPU woken just before, as in a regen. Then whole regens
//! with the CPU batch and the auto batch: their exact bits must agree.
//!
//! RTX 5060 Laptop, 4 threads (2026-10-30), medians: the GPU's round trip is ~85 µs up to a
//! few hundred points, and it wins from 256 points on every suite part (torture: 128 points
//! cpu 75 / gpu 83 µs, 256 points cpu 175 / gpu 85), hence CROSSOVER = 256. Regen batches are
//! 3000-9000 points: the batch call takes 2.4 ms on the GPU against 4.4 ms on the CPU summed
//! over the suite.
use forge_bool::{Batch, CpuBatch};
use forge_gpu::{AutoBatch, Gpu, Packed};
use forge_math::hash::BitHasher;
use forge_math::{Box3, Point3};
use forge_model::{suite, torture};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Points, sides and boxes.
type Asked = (Vec<Point3>, Vec<u32>, Vec<Box3>);

/// A CPU batch that keeps what it was asked.
struct Record<'a> {
    cpu: CpuBatch<'a>,
    got: Mutex<Option<Asked>>,
}

impl Batch for Record<'_> {
    fn verdicts(&self, points: &[Point3], sides: &[u32], boxes: &[Box3]) -> Vec<u8> {
        *self.got.lock().expect("record") = Some((points.to_vec(), sides.to_vec(), boxes.to_vec()));
        self.cpu.verdicts(points, sides, boxes)
    }
}

fn stats(mut t: Vec<f64>) -> (f64, f64) {
    t.sort_by(f64::total_cmp);
    (t[0], t[t.len() / 2])
}

/// `f` timed after an idle gap, `before` run at the start of the gap's last 200 µs.
fn after_gap(reps: usize, before: impl Fn(), mut f: impl FnMut()) -> (f64, f64) {
    stats(
        (0..reps)
            .map(|_| {
                std::thread::sleep(Duration::from_millis(10));
                before();
                let s = Instant::now();
                while s.elapsed() < Duration::from_micros(200) {}
                let s = Instant::now();
                f();
                s.elapsed().as_secs_f64() * 1e6
            })
            .collect(),
    )
}

fn bits(r: &Result<forge_topo::Body, forge_model::RegenError>) -> u64 {
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
    let reps = env("REPS", 30);
    let threads = env("THREADS", 4);
    let gpu = Gpu::new().expect("a CUDA GPU");
    println!("{}", gpu.name);
    let mut parts: Vec<(String, forge_model::Part)> = suite::parts()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p))
        .collect();
    parts.push(("torture".into(), torture::part(221)));

    println!("crossover: cpu best/med | gpu best/med (µs)");
    for (name, part) in &parts {
        let (shapes, ops) = part.nary_solids();
        let rec = Record {
            cpu: CpuBatch {
                shapes: &shapes,
                ops: &ops,
                threads,
            },
            got: Mutex::new(None),
        };
        part.regenerate_nary_batch(threads, true, Some(&rec))
            .expect("regen");
        let (pts, sides, boxes) = rec.got.into_inner().expect("record").expect("a batch");
        let cpu = CpuBatch {
            shapes: &shapes,
            ops: &ops,
            threads,
        };
        let packed = Packed::new(&shapes, &ops);
        print!("{name:<9} {:>5} points:", pts.len());
        for n in [16usize, 64, 128, 256, 512, 1024, 4096, pts.len(), 65536] {
            let p: Vec<Point3> = (0..n).map(|i| pts[i % pts.len()]).collect();
            let s: Vec<u32> = (0..n).map(|i| sides[i % pts.len()]).collect();
            let c = after_gap(
                reps,
                || {},
                || drop(std::hint::black_box(cpu.verdicts(&p, &s, &boxes))),
            );
            let g = after_gap(
                reps,
                || gpu.wake(),
                || drop(gpu.run(&packed, &p, &s, &boxes).expect("gpu")),
            );
            print!(
                "  {n}: {:.0}/{:.0} | {:.0}/{:.0}{}",
                c.0,
                c.1,
                g.0,
                g.1,
                if g.1 < c.1 { " gpu" } else { "" }
            );
        }
        println!();
    }

    println!("regens: whole regen cpu, auto (ms med) | batch call cpu, auto (µs best/med)");
    let mut sums = [0.0; 4];
    for (name, part) in &parts {
        let (shapes, ops) = part.nary_solids();
        let cpu = CpuBatch {
            shapes: &shapes,
            ops: &ops,
            threads,
        };
        let auto = AutoBatch::new(
            CpuBatch {
                shapes: &shapes,
                ops: &ops,
                threads,
            },
            Some(&gpu),
        );
        let mut regen = [Vec::new(), Vec::new()];
        let mut batch = [Vec::new(), Vec::new()];
        let mut hashes = [0u64; 2];
        for _ in 0..reps {
            for k in 0..2 {
                let b: &dyn Batch = if k == 0 { &cpu } else { &auto };
                let s = Instant::now();
                let r = part.regenerate_nary_batch(threads, true, Some(b));
                regen[k].push(s.elapsed().as_secs_f64() * 1e3);
                batch[k].push(forge_bool::nary_phase_times()[8] as f64 / 1e3);
                hashes[k] = bits(&r);
            }
        }
        let [rc, ra] = regen.map(stats);
        let [bc, ba] = batch.map(stats);
        sums[0] += rc.1;
        sums[1] += ra.1;
        sums[2] += bc.1;
        sums[3] += ba.1;
        println!(
            "{name:<9} {:>7.2} {:>7.2} | {:>6.0}/{:<6.0} {:>6.0}/{:<6.0} {}",
            rc.1,
            ra.1,
            bc.0,
            bc.1,
            ba.0,
            ba.1,
            if hashes[0] == hashes[1] {
                "same bits"
            } else {
                "DIFFERENT"
            }
        );
        assert_eq!(hashes[0], hashes[1], "{name}: the GPU changed a regen's result");
    }
    println!(
        "sum of medians: regen cpu {:.2}, auto {:.2} ms; batch cpu {:.0}, auto {:.0} µs",
        sums[0], sums[1], sums[2], sums[3]
    );
}

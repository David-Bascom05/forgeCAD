//! The GPU batch against the CPU batch on every suite part (skipped where there is no CUDA GPU,
//! and on wasm): every verdict the GPU gives is the CPU's, and a regen on the GPU backend has
//! the CPU batch's exact result.
use forge_bool::{Batch, CpuBatch};
use forge_gpu::Packed;
use forge_math::rng::Rng;
use forge_math::{Box3, Point3};
use forge_model::{Backend, suite, torture};
use std::sync::Mutex;

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

#[test]
fn gpu_verdicts_are_the_cpus() {
    let Some(gpu) = forge_gpu::gpu() else {
        eprintln!("no CUDA GPU: skipped");
        return;
    };
    let mut parts = suite::parts();
    parts.push(("torture", torture::part(221)));
    parts.push(("upto", suite::upto(false)));
    for (name, part) in parts {
        let (shapes, ops) = part.nary_solids().expect("tools build");
        let cpu = CpuBatch {
            shapes: &shapes,
            ops: &ops,
            threads: 4,
        };
        let rec = Record {
            cpu: CpuBatch { ..cpu },
            got: Mutex::new(None),
        };
        let want = part.regenerate_nary_batch(4, true, Some(&rec));
        // a history ending after a part-changing feature has no batch
        let Some((mut pts, mut sides, boxes)) = rec.got.into_inner().expect("record") else {
            continue;
        };
        // and random points over the part, each with a random own tool
        let all = boxes.iter().fold(Box3::EMPTY, |a, b| a.hull(b));
        let mut r = Rng::new(7);
        for _ in 0..20_000 {
            pts.push(Point3::new(
                r.range(all.x.lo, all.x.hi),
                r.range(all.y.lo, all.y.hi),
                r.range(all.z.lo, all.z.hi),
            ));
            sides.push(r.below(shapes.len()) as u32);
        }
        let cpu_v = cpu.verdicts(&pts, &sides, &boxes);
        let gpu_v = gpu
            .run(&Packed::new(&shapes, &ops), &pts, &sides, &boxes)
            .expect("gpu run");
        let wrong = gpu_v
            .iter()
            .zip(&cpu_v)
            .filter(|(g, c)| **g != 3 && g != c)
            .count();
        let wary = gpu_v
            .iter()
            .zip(&cpu_v)
            .filter(|(g, c)| **g == 3 && **c != 3)
            .count();
        assert_eq!(wrong, 0, "{name}: GPU verdicts differ from the CPU's");
        assert_eq!(wary, 0, "{name}: GPU undecided where the CPU decides");

        let got = part.regenerate_nary_on(4, Backend::Gpu);
        match (want, got) {
            (Ok(w), Ok(g)) => {
                assert_eq!(w.vertices.len(), g.vertices.len(), "{name}");
                for (a, b) in w.vertices.iter().zip(&g.vertices) {
                    assert_eq!(a.point, b.point, "{name}");
                }
            }
            (Err(_), Err(_)) => {}
            _ => panic!("{name}: the GPU changed whether the regen succeeds"),
        }
    }
}

//! What opening the GPU costs a process (forge-gpu's OPEN_COST): `open_cost [runs]` times
//! forge_gpu::gpu() from cold, then a regen's first GPU batch, in a fresh child per run.
use forge_model::{Backend, torture};
use std::process::Command;
use std::time::Instant;

fn main() {
    let mut args = std::env::args();
    let me = args.next().expect("program");
    match args.next().as_deref() {
        Some("child") => {
            let t = Instant::now();
            let g = forge_gpu::gpu();
            let open = t.elapsed().as_secs_f64() * 1e3;
            let part = torture::part(221);
            let t = Instant::now();
            part.regenerate_nary_on(4, Backend::Gpu).expect("regen");
            let regen = t.elapsed().as_secs_f64() * 1e3;
            println!("open {open:.0} ms, first gpu regen {regen:.0} ms, gpu {}", g.is_some());
        }
        runs => {
            for _ in 0..runs.and_then(|r| r.parse().ok()).unwrap_or(5) {
                let t = Instant::now();
                let out = Command::new(&me).arg("child").output().expect("child");
                print!("{}", String::from_utf8_lossy(&out.stdout));
                println!("  whole process incl. exit {:.0} ms", t.elapsed().as_secs_f64() * 1e3);
            }
        }
    }
}

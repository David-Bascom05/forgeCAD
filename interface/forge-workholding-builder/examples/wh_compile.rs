//! Compiles the built-in workholding models (kernel/forge-workholding/data/models.toml): each
//! model's STEP (data/models/, Git LFS: `git lfs pull --include=...` first) to its checker faces
//! (data/check/NAME.txt) and its bodies' meshes (data/models/, beside the STEP).
//!
//! cargo run --release -p forge-workholding-builder --example wh_compile [NAME..]
use std::path::Path;

fn main() {
    let only: Vec<String> = std::env::args().skip(1).collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../kernel/forge-workholding/data");
    let specs = forge_workholding::model::specs(
        &std::fs::read_to_string(root.join("models.toml")).expect("models.toml"),
    )
    .expect("models.toml");
    for m in specs {
        if !only.is_empty() && !only.contains(&m.name) {
            continue;
        }
        let step = std::fs::read(root.join("models").join(&m.step)).expect("the STEP (git lfs pull it)");
        assert!(
            !step.starts_with(b"version https://git-lfs"),
            "{}: an LFS pointer: git lfs pull it",
            m.step
        );
        let t0 = std::time::Instant::now();
        let c = forge_workholding_builder::compile::compile(
            &step,
            &forge_workholding_builder::compile::Spec::of(&m),
        )
        .unwrap_or_else(|e| panic!("{}: {e}", m.name));
        std::fs::write(root.join("check").join(format!("{}.txt", m.name)), &c.text).expect("write");
        for (g, bytes) in &c.stl {
            let mesh = &m.groups.iter().find(|x| &x.name == g).expect("group").mesh;
            if !mesh.is_empty() {
                std::fs::write(root.join("models").join(mesh), bytes).expect("write mesh");
            }
        }
        println!(
            "{}: {} solids: {} faces exact, {} enclosed, {} meshed ({} triangles); {} bytes; {:.2}s",
            m.name,
            c.solids,
            c.exact,
            c.enclosed,
            c.meshed,
            c.triangles,
            c.text.len(),
            t0.elapsed().as_secs_f64()
        );
        for (g, lo, hi) in &c.tight {
            println!("  {g:12} {lo:?} {hi:?}");
        }
    }
}

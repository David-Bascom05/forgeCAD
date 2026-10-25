//! Meshes every suite part (and a few fuzz histories) and reports failures.
use forge_mesh::{Quality, tessellate};
use forge_model::suite;
fn main() {
    let mut parts = suite::parts();
    for seed in [1u64, 2, 12, 15, 16] {
        parts.push(("fuzz", suite::fuzz(seed, 80)));
    }
    for q in [
        Quality::default(),
        Quality {
            chord: 0.25,
            max_angle: 0.7,
        },
    ] {
        for (name, part) in &parts {
            let Ok(b) = part.regenerate() else { continue };
            match tessellate(&b, &q) {
                Ok(m) => println!(
                    "{name:10} chord {:5}: {} tris, {} open",
                    q.chord,
                    m.triangles.len(),
                    m.open_edges().len()
                ),
                Err(e) => println!("{name:10} chord {:5}: {e:?}", q.chord),
            }
        }
    }
}

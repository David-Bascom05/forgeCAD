//! A machine model file read as the app and the CLI read it (`model_files::read`), timed, with a
//! hash of its pieces (names, corners' bits, triangles) to compare one build's with another's.
//! `model_read FILE...`.
use std::hash::{Hash, Hasher};

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read");
        let t = std::time::Instant::now();
        let r = forge_machine_builder::model_files::read(&path, &bytes);
        let s = t.elapsed().as_secs_f64();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        match &r {
            Ok(ps) => {
                for p in ps {
                    p.name.hash(&mut h);
                    for c in &p.pos {
                        c.map(f64::to_bits).hash(&mut h);
                    }
                    p.tri.hash(&mut h);
                }
                let tris: usize = ps.iter().map(|p| p.tri.len()).sum();
                println!("{path} pieces {} tris {tris} hash {:016x} {s:.3}s", ps.len(), h.finish());
            }
            Err(e) => println!("{path} err {e} {s:.3}s"),
        }
    }
}

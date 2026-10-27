//! A hash of every regenerated part's exact bits (vertex points, edge ranges, face count), seq
//! and n-ary: builds that must agree bit for bit (another target, another machine, wasm) print
//! the same lines.
use forge_math::hash::BitHasher;
use forge_model::{suite, torture};

fn main() {
    let mut parts = suite::parts();
    parts.push(("torture", torture::part(221)));
    let fuzz: Vec<_> = (1..=10).map(|s| suite::fuzz(s, 80)).collect();
    for (name, part) in parts.iter().map(|(n, p)| (n.to_string(), p)).chain(
        fuzz.iter()
            .enumerate()
            .map(|(i, p)| (format!("fuzz{}", i + 1), p)),
    ) {
        for (path, r) in [("seq", part.regenerate()), ("nary", part.regenerate_nary(4))] {
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
            println!("{name} {path} {:016x}", h.finish());
        }
    }
}

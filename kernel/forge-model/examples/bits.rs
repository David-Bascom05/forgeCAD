//! A hash of every regenerated part's exact bits (vertex points, edge ranges, face count), seq
//! and n-ary: builds that must agree bit for bit (another target, another machine, wasm) print
//! the same lines. `bits measures`: each part's validity, meshed volume and area (chord 0.02) and
//! face count in place of the hash, for comparing two builds' results where their bits differ.
use forge_math::hash::BitHasher;
use forge_model::{suite, torture};

fn main() {
    let measures = std::env::args().nth(1).as_deref() == Some("measures");
    let mut parts = suite::parts();
    parts.push(("torture", torture::part(221)));
    let fuzz: Vec<_> = (1..=10).map(|s| suite::fuzz(s, 80)).collect();
    for (name, part) in parts.iter().map(|(n, p)| (n.to_string(), p)).chain(
        fuzz.iter()
            .enumerate()
            .map(|(i, p)| (format!("fuzz{}", i + 1), p)),
    ) {
        for (path, r) in [("seq", part.regenerate()), ("nary", part.regenerate_nary(4))] {
            if measures {
                println!("{name} {path} {}", measure(r.as_ref().ok().into_iter()));
                continue;
            }
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

/// Valid, meshed volume and area, and face count of the bodies (or "error").
fn measure<'a>(bodies: impl Iterator<Item = &'a forge_topo::Body>) -> String {
    let q = forge_mesh::Quality {
        chord: 0.02,
        max_angle: 0.5,
    };
    let (mut valid, mut vol, mut area, mut faces, mut n) = (true, 0.0, 0.0, 0, 0);
    for b in bodies {
        n += 1;
        valid &= forge_topo::check(b).is_empty();
        faces += b.faces.len();
        match forge_mesh::tessellate(b, &q) {
            Ok(m) => {
                vol += m.volume();
                area += m.area();
            }
            Err(_) => valid = false,
        }
    }
    if n == 0 {
        return "error".into();
    }
    format!(
        "{} {vol:.6} {area:.6} {faces}",
        if valid { "valid" } else { "INVALID" }
    )
}

//! A hash of every regenerated part's exact bits (vertex points, edge ranges, face count), seq
//! and n-ary: builds that must agree bit for bit (another target, another machine, wasm) print
//! the same lines. `bits sorted`: a hash of the vertex points and each edge's end points, sorted,
//! and the face count, so two writes of one result that number or parametrise its entities
//! differently print the same lines (BITS_COUNTS=1: the counts to stderr).
use forge_math::hash::BitHasher;
use forge_model::{suite, torture};

fn main() {
    let sorted = std::env::args().nth(1).as_deref() == Some("sorted");
    let mut parts = suite::parts();
    parts.push(("torture", torture::part(221)));
    // BITS_PREFIX=n: each part's first n features only
    if let Some(n) = std::env::var("BITS_PREFIX").ok().and_then(|n| n.parse::<usize>().ok()) {
        for (_, p) in &mut parts {
            p.features.truncate(n);
        }
    }
    let fuzz: Vec<_> = (1..=10).map(|s| suite::fuzz(s, 80)).collect();
    for (name, part) in parts.iter().map(|(n, p)| (n.to_string(), p)).chain(
        fuzz.iter()
            .enumerate()
            .map(|(i, p)| (format!("fuzz{}", i + 1), p)),
    ) {
        for (path, r) in [("seq", part.regenerate()), ("nary", part.regenerate_nary(4))] {
            let mut h = BitHasher::new();
            match r {
                Ok(b) if sorted => {
                    let b = b.compact();
                    if std::env::var_os("BITS_COUNTS").is_some() {
                        eprintln!("{name} {path}: {} vertices, {} edges, {} faces", b.vertices.len(), b.edges.len(), b.faces.len());
                    }
                    let key = |x: f64| x.to_bits();
                    let mut ps: Vec<[u64; 3]> = b.vertices.iter().map(|v| [key(v.point.x), key(v.point.y), key(v.point.z)]).collect();
                    ps.sort_unstable();
                    // an edge by its end points (its range depends on how its curve is
                    // parametrised, which a write may keep or remake)
                    let pt = |v: forge_topo::VertexId| {
                        let p = b.vertex(v).point;
                        [key(p.x), key(p.y), key(p.z)]
                    };
                    let mut es: Vec<[[u64; 3]; 2]> = b
                        .edges
                        .iter()
                        .map(|e| {
                            let (a, z) = (pt(e.start), pt(e.end));
                            if a <= z { [a, z] } else { [z, a] }
                        })
                        .collect();
                    es.sort_unstable();
                    if std::env::var("BITS_DUMP").is_ok_and(|d| d == format!("{name} {path}")) {
                        for p in &ps {
                            eprintln!("v {:?}", p.map(f64::from_bits));
                        }
                        for e in &es {
                            eprintln!("e {:?} {:?}", e[0].map(f64::from_bits), e[1].map(f64::from_bits));
                        }
                        for f in b.face_ids() {
                            let n: usize = b.face(f).loops.iter().map(|&l| b.lp(l).coedges().len()).sum();
                            eprintln!("f {} loops {} coedges", b.face(f).loops.len(), n);
                        }
                    }
                    for p in ps {
                        for x in p {
                            h.u64(x);
                        }
                    }
                    for e in es {
                        for x in e.iter().flatten() {
                            h.u64(*x);
                        }
                    }
                    h.u64(b.faces.len() as u64);
                }
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

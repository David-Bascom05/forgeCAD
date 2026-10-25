//! cargo run --release -p forge-model --example cracks -- <suite part | fuzz seed>
//! Meshes a part and reports which faces its open (cracked) mesh edges belong to.
use forge_mesh::{Quality, tessellate};
use forge_model::suite;
fn main() {
    let arg = std::env::args().nth(1).expect("suite part name or fuzz seed");
    let part = match arg.parse::<u64>() {
        Ok(seed) => suite::fuzz(seed, 80),
        Err(_) => {
            suite::parts()
                .into_iter()
                .find(|(n, _)| *n == arg)
                .expect("no such part")
                .1
        }
    };
    let b = part.regenerate().expect("regenerates").compact();
    let m = tessellate(&b, &Quality::default()).expect("meshes");
    let open = m.open_edges();
    let mut per_face: Vec<(u32, usize, usize)> = Vec::new();
    for (t, &f) in m.triangles.iter().zip(&m.face_of) {
        for (a, c) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            if open.binary_search(&(a, c)).is_ok() {
                match per_face.iter_mut().find(|x| x.0 == f) {
                    Some(x) => x.1 += 1,
                    None => per_face.push((f, 1, 0)),
                }
            }
        }
    }
    println!("{} open edges", open.len());
    for &(a, c) in open.iter().take(6) {
        println!(
            "  open {a}->{c}: {:?} -> {:?}",
            m.positions[a as usize], m.positions[c as usize]
        );
    }
    for (f, n, _) in &per_face {
        let face = b.face(forge_topo::FaceId(*f));
        let k = format!("{:?}", face.surface);
        let edges: Vec<String> = face
            .loops
            .iter()
            .flat_map(|&l| b.lp(l).coedges.iter())
            .map(|&c| {
                let e = b.edge(b.coedge(c).edge);
                let k = format!("{:?}", e.curve);
                format!("{}[{:.4}]", &k[..k.find('(').unwrap_or(k.len())], e.range.width())
            })
            .collect();
        println!(
            "face {f}: {n} open, {} loops, {} | {}",
            face.loops.len(),
            &k[..k.find('(').unwrap_or(k.len())],
            edges.join(" ")
        );
    }
}

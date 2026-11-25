//! Diagnostic: the faces along a solid's display mesh's open edges, with their surface kinds.
fn main() {
    let p = std::env::args().nth(1).unwrap();
    let imp = forge_step::import(&std::fs::read(&p).unwrap(), &forge_step::Limits::default()).unwrap();
    for s in &imp.solids {
        let m = s.display_mesh().unwrap();
        let open = m.open_edges();
        let mut by: std::collections::BTreeMap<u32, usize> = Default::default();
        for (t, tri) in m.triangles.iter().enumerate() {
            for k in 0..3 {
                let e = (tri[k], tri[(k + 1) % 3]);
                if open.binary_search(&e).is_ok() {
                    *by.entry(m.face_of[t]).or_default() += 1;
                }
            }
        }
        for (f, n) in by {
            let face = s.body.face(forge_topo::FaceId(f));
            let kind = format!("{:?}", face.surface).chars().take(90).collect::<String>();
            let edges: Vec<String> = face.loops.iter().flat_map(|&l| s.body.lp(l).coedges()).map(|c| {
                let e = s.body.edge(s.body.coedge(c).edge);
                format!("{}", format!("{:?}", e.curve).split(['(', ' ']).next().unwrap())
            }).collect();
            println!("face {f} open {n} | {kind} | edges {:?}", edges);
        }
    }
}

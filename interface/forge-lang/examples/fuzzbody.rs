//! (fuzz comparison) build a program as the fuzzer does and print the last body: per vertex its point
//! and how many faces meet there; then the faces' count.
fn main() {
    let src = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let b = forge_lang::fuzz::build(&src);
    let n = b.built();
    let body = b.body_after(n).unwrap();
    let mut faces_at: std::collections::BTreeMap<u32, std::collections::BTreeSet<u32>> = Default::default();
    for f in body.face_ids() {
        for &lp in body.face(f).loops.iter() {
            for c in body.lp(lp).coedges() {
                faces_at.entry(body.coedge_start(c).0).or_default().insert(f.0);
            }
        }
    }
    println!("faces {} verts {}", body.face_ids().count(), faces_at.len());
    let mut rows: Vec<String> = faces_at
        .iter()
        .map(|(v, fs)| {
            let p = body.vertex(forge_topo::VertexId(*v)).point;
            format!("{:10.4} {:10.4} {:10.4}  faces {}", p.x, p.y, p.z, fs.len())
        })
        .collect();
    rows.sort();
    for r in rows {
        println!("{r}");
    }
    let mut names: Vec<String> = body
        .face_ids()
        .map(|f| {
            let bx = body.face_box(f);
            format!(
                "face {:?} z [{:.3}, {:.3}] plane {}",
                body.face(f).name,
                bx.z.lo,
                bx.z.hi,
                matches!(body.face(f).surface, forge_geom::Surface::Plane(_))
            )
        })
        .collect();
    names.sort();
    for n in names {
        println!("{n}");
    }
}

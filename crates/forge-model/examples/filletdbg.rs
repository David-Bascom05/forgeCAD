//! Prints a fillet case's base body (faces, vertex degrees) and the fillet's outcome.
use forge_model::fillets;
fn main() {
    let want = std::env::args().nth(1).unwrap_or_default();
    for (name, part, _) in fillets::cases() {
        if name != want {
            continue;
        }
        let mut base = part.clone();
        base.features.pop();
        let b = base.regenerate().unwrap();
        for f in b.face_ids() {
            println!(
                "face {f:?} {:?} loops {}",
                b.face(f).surface,
                b.face(f).loops.len()
            );
        }
        for v in b.vertex_ids() {
            let deg = b
                .edge_ids()
                .filter(|&e| b.edge(e).start == v || b.edge(e).end == v)
                .count();
            println!("vertex {v:?} {:?} degree {deg}", b.vertex(v).point);
        }
        match part.regenerate() {
            Ok(r) => println!("ok, problems {:?}", forge_topo::check(&r)),
            Err(e) => println!("{e:?}"),
        }
    }
}

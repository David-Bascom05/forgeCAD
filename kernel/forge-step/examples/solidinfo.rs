//! Each solid's verdict in a file: whether its checks agreed (`unverified`), its volume and area,
//! its largest edge tolerance, and its display mesh's volume. `solidinfo FILE...`
fn main() {
    for p in std::env::args().skip(1) {
        let imp = forge_step::import(&std::fs::read(&p).unwrap(), &forge_step::Limits::default()).unwrap();
        for r in &imp.refusals {
            println!("{p} refused #{} {}: {}", r.entity, r.kind, r.why);
        }
        for s in &imp.solids {
            let m = s.mass.as_ref().map(|m| (m.volume, m.area));
            let tol = s.body.edges.iter().map(|e| e.tol).fold(0.0, f64::max);
            let mv = s.display_mesh().ok().map(|m| m.volume());
            println!(
                "{p} #{} faces {} unverified {:?} mass {:?} mesh volume {:?} max edge tol {:.2e}",
                s.entity, s.body.faces.len(), s.body.unverified, m, mv, tol
            );
        }
    }
}

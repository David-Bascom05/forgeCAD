#[path = "surf_dome.rs"]
#[allow(dead_code)]
mod dome;
use forge_cam_geom::drop::{Column, drop};
use forge_cam_geom::ival::Ival;
use forge_tooling::Library;
fn main() {
    let faces = dome::dome();
    let lib = Library::builtin();
    let t = lib.tool("flat-10").unwrap().clone();
    let sh = t.shape(t.lcf + 15.0, None).unwrap();
    let col = Column::of(&sh.bodies[0]).unwrap();
    let f = &faces[0];
    for (p, q) in [(0.5, 0.5), (0.1, 0.1), (1.0, 2.0), (30.0, 1.0)] {
        println!("holds {p} {q}: {}", f.holds(Ival::pt(p), Ival::pt(q)));
    }
    let c = drop(&col, &faces, &[0], -3.0, -3.0, 1e-3, 2000);
    println!("{c:?}");
}

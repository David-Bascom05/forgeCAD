//! One regen and one mesh of a part, for instruction counting: `mesh1 <part> <chord>`
//! (subtract `regen1 <part> seq` for the mesh alone). Parts as in regen1.
use forge_mesh::{Quality, tessellate};
use forge_model::{suite, torture};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let name = args.get(1).map_or("torture", |s| s.as_str());
    let chord: f64 = args.get(2).and_then(|c| c.parse().ok()).unwrap_or(0.01);
    let part = match name {
        "torture" => torture::part(221),
        n if n.starts_with("fuzz") => suite::fuzz(n[4..].parse().expect("fuzz seed"), 80),
        n => {
            suite::parts()
                .into_iter()
                .find(|p| p.0 == n)
                .expect("a suite part")
                .1
        }
    };
    let b = part.regenerate().expect("regen");
    let q = Quality {
        chord,
        ..Quality::default()
    };
    let m = tessellate(&b, &q).expect("mesh");
    assert!(m.open_edges().is_empty(), "{name}: open mesh");
    println!("{name} {} tris volume {:.3}", m.triangles.len(), m.volume());
}

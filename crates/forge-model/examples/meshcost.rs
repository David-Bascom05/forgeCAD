//! Mesh cost, for instruction counting: `meshcost <part> <chord> <times>` regenerates a suite
//! part or fillet case once and meshes it `times` times (0: regenerate only; the difference
//! over `times` is one mesh).
use forge_mesh::{Quality, tessellate};
use forge_model::{fillets, suite, torture};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let name = args[1].as_str();
    let chord: f64 = args[2].parse().expect("chord");
    let times: usize = args[3].parse().expect("times");
    let part = match name {
        "torture" => torture::part(221),
        n if n.starts_with("fuzz") => suite::fuzz(n[4..].parse().expect("seed"), 80),
        n => suite::part(n).unwrap_or_else(|| {
            fillets::cases()
                .into_iter()
                .chain(fillets::variable_cases())
                .find(|c| c.0 == n)
                .expect("a suite part or fillet case")
                .1
        }),
    };
    let b = part.regenerate().expect("regen");
    let q = Quality {
        chord,
        ..Quality::default()
    };
    for _ in 0..times {
        let m = tessellate(&b, &q).expect("mesh");
        std::hint::black_box(&m);
    }
}

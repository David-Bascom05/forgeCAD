//! One fillet case, for instruction counting: `fillet1 <case> <base|fillet>`.
//! `base` regenerates the part without its fillet, `fillet` with it; the
//! difference over the edge count is the cost per filleted edge.
use forge_model::{Feature, fillets};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (want, mode) = (args[1].as_str(), args[2].as_str());
    for (name, mut part, _) in fillets::cases() {
        if name != want {
            continue;
        }
        let edges = match part.features.last() {
            Some(Feature::Fillet { edges }) => edges.len(),
            _ => 0,
        };
        if mode == "base" {
            part.features.pop();
        }
        let b = part.regenerate().expect("regenerates");
        println!("{name}: {} faces, {edges} filleted edges", b.faces.len());
    }
}

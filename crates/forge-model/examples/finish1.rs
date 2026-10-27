//! One regen of a finishing part, for instruction counting: `finish1 <part> <seq|nary1> [features]`.
//! With a feature count, only that many features of the history are regenerated.
use forge_model::{Part, finishing};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let name = args.get(1).map_or("chamfer-all", |s| s.as_str());
    let path = args.get(2).map_or("seq", |s| s.as_str());
    let mut part = finishing::parts()
        .into_iter()
        .find(|p| p.0 == name)
        .expect("a finishing part")
        .1;
    if let Some(n) = args.get(3) {
        part = Part {
            features: part.features[..n.parse::<usize>().expect("a count")].to_vec(),
        };
    }
    let ok = match path {
        "seq" => part.regenerate().is_ok(),
        _ => part.regenerate_nary(1).is_ok(),
    };
    assert!(ok, "{name} {path} failed");
}

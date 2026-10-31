//! One full regen of a part, for instruction counting: `regen1 <part> <seq|nary1|naryN>`.
//! Parts: a suite name, torture, upto, upto-flat, or fuzzN. The n-ary batch runs where FORGE_BATCH says.
use forge_model::{suite, torture};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let name = args.get(1).map_or("torture", |s| s.as_str());
    let path = args.get(2).map_or("seq", |s| s.as_str());
    let part = match name {
        "torture" => torture::part(221),
        // kept out of the suite list, which every count here pays to build
        "upto" => suite::upto(false),
        "upto-flat" => suite::upto(true),
        n if n.starts_with("fuzz") => suite::fuzz(n[4..].parse().expect("fuzz seed"), 80),
        n => {
            suite::parts()
                .into_iter()
                .find(|p| p.0 == n)
                .expect("a suite part")
                .1
        }
    };
    let ok = match path {
        "seq" => part.regenerate().is_ok(),
        "nary1" => part.regenerate_nary(1).is_ok(),
        _ => part.regenerate_nary(16).is_ok(),
    };
    assert!(ok, "{name} {path} failed");
}

//! cargo run --release -p forge-model --example dbgfuzz -- <seed> <failing feature index>
//! Prints the history up to the failing feature and reruns that boolean with diagnostics.
use forge_model::{Feature, Part, suite};
fn main() {
    let a: Vec<u64> = std::env::args().skip(1).map(|x| x.parse().unwrap()).collect();
    let part = suite::fuzz(a[0], 200);
    let n = a[1] as usize;
    for (i, f) in part.features.iter().take(n + 1).enumerate() {
        let d = match f {
            Feature::Extrude {
                profile,
                distance,
                op,
            } => format!(
                "extrude {op:?} d={distance:.3} plane {:?} loops {:?}",
                profile.plane, profile.loops
            ),
            Feature::Revolve { profile, op, .. } => format!(
                "revolve {op:?} plane {:?} loops {:?}",
                profile.plane, profile.loops
            ),
            Feature::Ball { frame, radius, op } => format!("ball {op:?} r={radius:.3} at {:?}", frame.origin),
            other => format!("{other:?}"),
        };
        println!("{i}: {}", &d[..d.len().min(600)]);
    }
    let prev = Part {
        features: part.features[..n].to_vec(),
    }
    .regenerate()
    .unwrap();
    println!(
        "before: {} faces, problems {:?}",
        prev.faces.len(),
        forge_topo::check(&prev)
    );
    let r = Part {
        features: part.features[..=n].to_vec(),
    }
    .regenerate();
    println!("result: {:?}", r.err());
}

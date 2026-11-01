//! cargo run --release -p forge-model --example splinehunt -- [first] [last] [family]
use forge_model::adversarial::{canonical, families, literal, outcome, shrink};
use std::collections::BTreeMap;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let first: u64 = a.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let last: u64 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);
    let only = a.get(2).cloned();
    std::panic::set_hook(Box::new(|_| {}));
    for (fam, g) in families().into_iter().filter(|f| {
        only.as_deref()
            .map_or(f.0.contains("spline") || f.0 == "ellipse_walls", |o| o == f.0)
    }) {
        let mut kinds: BTreeMap<String, (usize, u64)> = BTreeMap::new();
        let mut ok = 0;
        for s in first..last {
            let p = canonical(&g(s));
            match outcome(&p).kind() {
                None => ok += 1,
                Some(k) => {
                    let e = kinds.entry(k.clone()).or_insert((0, s));
                    e.0 += 1;
                    if a.iter().any(|x| x == "--list") {
                        println!("   seed {s}: {k}");
                    }
                }
            }
        }
        println!("{fam}: ok {ok}/{}", last - first);
        for (k, (n, s)) in &kinds {
            println!("  {n:4} {k}  first seed {s}");
            if a.iter().any(|x| x == "--shrink") {
                let m = shrink(&canonical(&g(*s)));
                println!("{}", literal(&m));
            }
        }
    }
}

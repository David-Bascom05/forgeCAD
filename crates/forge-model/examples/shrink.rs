//! cargo run --release -p forge-model --example shrink -- <suite part name | fuzz seed>
//! Shrinks a failing history to a minimal one and prints it as a literal.
use forge_geom::CurveGeom;
use forge_model::{adversarial, suite};
fn main() {
    let arg = std::env::args().nth(1).expect("suite part name or fuzz seed");
    let part = match arg.parse::<u64>() {
        Ok(seed) => suite::fuzz(seed, 80),
        Err(_) => {
            suite::parts()
                .into_iter()
                .find(|(n, _)| *n == arg)
                .expect("no such part")
                .1
        }
    };
    let small = adversarial::shrink(&part);
    println!("{:?}", adversarial::outcome(&small));
    println!("{}", adversarial::literal(&small));
    // with FORGE_DEBUG set, the diagnostics after this line are the minimal part's
    eprintln!("=== rerun");
    match small.regenerate() {
        Ok(b) => {
            let problems = forge_topo::check(&b);
            println!("{problems:?}");
            for p in &problems {
                if let forge_topo::Problem::VertexOffEdge { edge, .. } = p {
                    let e = b.edge(*edge);
                    let kind = format!("{:?}", e.curve);
                    println!(
                        "edge {edge:?}: {} range {:?} start {:?} -> {:?} end {:?} -> {:?}",
                        &kind[..kind.len().min(300)],
                        e.range,
                        b.vertex(e.start).point,
                        e.curve.eval(e.range.lo),
                        b.vertex(e.end).point,
                        e.curve.eval(e.range.hi)
                    );
                }
            }
        }
        Err(e) => println!("{e:?}"),
    }
}

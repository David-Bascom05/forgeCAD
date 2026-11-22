//! One pattern, B's ends coincident with A's moved by delta x LINEAR on the given axes.
//! cargo run --release -p forge-exact --example box_one -- x/y/z op delta axes   (e.g. meets/overlaps/equal union 1.5 0)
use forge_bool::Op;
use forge_exact::{Pattern, Rel, block, exact, exact_boxes, kernel_on};
use forge_math::tol::LINEAR;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let names: Vec<&str> = a[0].split('/').collect();
    let op = match a[1].as_str() {
        "union" => Op::Union,
        "subtract" => Op::Subtract,
        _ => Op::Intersect,
    };
    let dl: f64 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let axes: Vec<usize> = a.get(3).map_or(vec![0, 1, 2], |s| {
        s.chars()
            .filter_map(|c| c.to_digit(10))
            .map(|d| d as usize)
            .collect()
    });
    let p = Pattern {
        rel: [0, 1, 2].map(|k| Rel::by_name(names[k]).expect("relation")),
        op,
    };
    let (ba, mut bb) = p.boxes(10.0);
    for &k in &axes {
        for s in 0..2 {
            if ba[k].contains(&bb[k][s]) {
                bb[k][s] += if s == 0 { -dl * LINEAR } else { dl * LINEAR };
            }
        }
    }
    println!("A {ba:?}\nB {bb:?}");
    let got = kernel_on(block(ba, 1), block(bb, 2), op);
    println!(
        "kernel {}  coincident {}  apart {}",
        got.word(),
        exact(&p, 10.0).word(),
        exact_boxes(ba, bb, op).word()
    );
    if std::env::var_os("SHOW").is_some() {
        show(ba, bb, op);
    }
}

/// With SHOW=1: the result's vertices, its edges' ends, and `check`'s problems.
#[allow(dead_code)]
fn show(a: [[f64; 2]; 3], b: [[f64; 2]; 3], op: Op) {
    let r = forge_bool::boolean(&block(a, 1), &block(b, 2), op, 3);
    let Ok(body) = r else {
        println!("refused {:?}", r.err());
        return;
    };
    for v in body.vertex_ids() {
        let x = body.vertex(v);
        println!("  {v:?} {:?} tol {:e}", x.point, x.tol);
    }
    for e in body.edge_ids() {
        let x = body.edge(e);
        println!("  {e:?} {:?} -> {:?} tol {:e}", x.start, x.end, x.tol);
    }
    for p in forge_topo::check(&body) {
        println!("  problem {p:?}");
    }
}

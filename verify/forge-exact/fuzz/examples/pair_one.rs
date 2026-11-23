//! One box-box pattern against the reference, the kernel's body shown.
//! cargo run --release -p forge-exact-fuzz --example pair_one -- X/Y/Z OP [AXES DELTA]
//! e.g. `meets/overlaps/overlaps subtract`, `equal/meets/equal union 0 1.5`.
use forge_exact_fuzz::pairs::{OPS, Pair, Pattern, RELS, block, kernel_pair, op_word, pair_input};
use forge_exact_fuzz::{Reference, classify};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rels: Vec<_> = args[0]
        .split('/')
        .map(|n| RELS.into_iter().find(|r| r.name == n).expect("a relation"))
        .collect();
    let op = OPS.into_iter().find(|o| op_word(*o) == args[1]).expect("an op");
    let p = Pattern {
        rel: [rels[0], rels[1], rels[2]],
        op,
    };
    let (a, b) = match args.get(2) {
        Some(ax) => {
            let axes: Vec<usize> = ax
                .chars()
                .filter_map(|c| c.to_digit(10))
                .map(|d| d as usize)
                .collect();
            p.banded(10.0, &axes, args[3].parse().expect("δ"))
                .expect("a coincident end")
        }
        None => p.boxes(10.0),
    };
    let pair = Pair {
        name: p.label(),
        a,
        b,
        op,
    };
    println!("{}\nA {:?}\nB {:?}", pair.source(), a, b);
    let (ba, bb) = (block(a, 1), block(b, 2));
    let mut r = Reference::start();
    let ans = r.ask(&pair_input(&ba, &bb, op).expect("flat"));
    println!("reference: {ans:?}");
    let k = kernel_pair(&ba, &bb, op);
    println!("kernel: {k:?}\nclass: {:?}", classify(&ans, &k));
    if let Ok(body) = forge_bool::boolean(&ba, &bb, op, 3) {
        let body = body.compact();
        for f in body.face_ids() {
            let face = body.face(f);
            let pts: Vec<String> = face
                .loops
                .iter()
                .map(|&l| {
                    body.lp(l)
                        .coedges()
                        .map(|c| {
                            let ce = body.coedge(c);
                            let ed = body.edge(ce.edge);
                            let v = body.vertex(if ce.forward { ed.start } else { ed.end }).point;
                            format!("({},{},{})", v.x, v.y, v.z)
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect();
            println!("face {f:?}: {}", pts.join(" | "));
        }
    }
}

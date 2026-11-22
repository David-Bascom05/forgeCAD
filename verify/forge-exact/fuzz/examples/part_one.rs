//! One family part (its flat sub-history, optionally only some features) against the reference,
//! the kernel's faces listed by carrier.
//! cargo run --release -p forge-exact-fuzz --example part_one -- FAMILY SEED [i,j,..] [ulp SEED]
use forge_exact_fuzz::{Reference, classify, judge_body, nudged_last, reference_input_rule, sequential};
use forge_geom::Surface;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rule = std::env::var("FORGE_EXACT_RULE").unwrap_or_else(|_| "snap".into());
    let mut part = if args[0] == "notched" {
        forge_exact_fuzz::notched()
    } else {
        let g = forge_model::adversarial::families()
            .into_iter()
            .find(|(f, _)| *f == args[0])
            .expect("a family")
            .1;
        forge_exact_fuzz::flat_part(&forge_model::adversarial::canonical(&g(args[1]
            .parse()
            .expect("seed"))))
    };
    if let Some(keep) = args
        .get(2)
        .filter(|k| k.contains(',') || k.parse::<usize>().is_ok())
    {
        let keep: Vec<usize> = keep.split(',').map(|i| i.parse().expect("an index")).collect();
        part.features = keep.iter().map(|&i| part.features[i].clone()).collect();
    }
    let ulp = args
        .iter()
        .position(|a| a == "ulp")
        .map(|i| args[i + 1].parse::<u64>().expect("ulp seed"));
    let mut r = Reference::start();
    let ans = r.ask(&reference_input_rule(&part, &rule).expect("flat"));
    println!("reference ({rule}): {ans:?}");
    let k = match ulp {
        Some(s) => nudged_last(&part, s),
        None => sequential(&part),
    };
    println!("kernel: {k:?}\nclass: {:?}", classify(&ans, &k));
    let body = match ulp {
        Some(_) => return,
        None => part.regenerate().map_err(|e| format!("{e:?}")),
    };
    let _ = judge_body(body.clone());
    if let Ok(b) = body {
        let b = b.compact();
        for f in b.face_ids() {
            let face = b.face(f);
            let Surface::Plane(pl) = &face.surface else {
                continue;
            };
            let (o, n) = (pl.frame.origin, pl.frame.z().get());
            let pts: Vec<String> = face
                .loops
                .iter()
                .flat_map(|&l| {
                    b.lp(l)
                        .coedges()
                        .map(|c| {
                            let ed = b.edge(b.coedge(c).edge);
                            let v = b
                                .vertex(if b.coedge(c).forward { ed.start } else { ed.end })
                                .point;
                            format!("({:.6},{:.6},{:.6})", v.x, v.y, v.z)
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            println!(
                "face {:?} n ({:.0},{:.0},{:.0}) d {:.7}: {}",
                f,
                n.x,
                n.y,
                n.z,
                o.x * n.x + o.y * n.y + o.z * n.z,
                pts.join(" ")
            );
        }
    }
}

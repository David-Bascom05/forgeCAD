//! Every final name of the bench parts, to compare two versions: nameprobe dump [PART..]
//! Copy to interface/forge-agent/examples/ to run; see research/names-arena/README.md.
use forge_agent::bench;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parts: Vec<&str> = if args.len() > 1 { args[1..].iter().map(|s| s.as_str()).collect() } else { bench::NAMES.to_vec() };
    for part in parts {
        let m = bench::model(bench::src(part).expect("a bench part"));
        for s in m.solids_after(m.built()) {
            let b = &s.body;
            for f in b.face_ids() {
                println!("{part} f {} {:?}", s.id, b.face(f).name);
            }
            for e in b.edge_ids() {
                println!("{part} e {} {:?}", s.id, b.edge(e).name);
            }
            let mut seen = std::collections::HashSet::new();
            for e in b.edge_ids() {
                for v in [b.edge(e).start, b.edge(e).end] {
                    if seen.insert(v) {
                        println!("{part} v {} {:?}", s.id, b.vertex(v).name);
                    }
                }
            }
        }
    }
}

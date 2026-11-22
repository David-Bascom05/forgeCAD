//! Every final name of the bench parts, to compare two versions: nameprobe dump [PART..]
//! Copy to interface/forge-lang/examples/ to run; see research/names-arena/README.md. The parts are
//! forge-lang's (`parts/bench/*.fg`), built with their font beside them.
use forge_lang::bench;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parts: Vec<&str> = if args.len() > 1 { args[1..].iter().map(|s| s.as_str()).collect() } else { bench::NAMES.to_vec() };
    let dir = std::env::temp_dir().join(format!("nameprobe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    std::fs::write(dir.join(bench::FONT), bench::font_bytes()).expect("the font");
    let opts = forge_lang::Options { dir: Some(dir.clone()), checks: false, ..Default::default() };
    for part in parts {
        let (run, _) = forge_lang::run(bench::src(part).expect("a bench part"), opts.clone(), Default::default());
        assert!(run.errors.is_empty(), "{part}: {:?}", run.errors);
        let b = run.body.as_ref().expect("a part");
        for s in &run.solids {
            // a solid's faces and edges are a run of the body's arenas
            for f in b.face_ids().filter(|f| s.faces.contains(&f.0)) {
                println!("{part} f {} {:?}", s.solid, b.face(f).name);
            }
            let edges: Vec<_> = b.edge_ids().filter(|e| s.edges.contains(&e.0)).collect();
            for &e in &edges {
                println!("{part} e {} {:?}", s.solid, b.edge(e).name);
            }
            let mut seen = std::collections::HashSet::new();
            for &e in &edges {
                for v in [b.edge(e).start, b.edge(e).end] {
                    if seen.insert(v) {
                        println!("{part} v {} {:?}", s.solid, b.vertex(v).name);
                    }
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

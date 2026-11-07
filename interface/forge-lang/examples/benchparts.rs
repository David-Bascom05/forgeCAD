//! The real-part benchmarks (parts/bench/*.fg), for the speed and bit gate:
//!   benchparts bits      each part's exact result, as its .fg run builds it, hashed
//!   benchparts check     each part built: features, faces, edges and the build's time
use forge_math::hash::BitHasher;

const NAMES: [&str; 15] = [
    "bracket", "clamp", "conrod", "crank", "frame", "gearbox", "guard", "heatsink", "hub", "insert", "jack",
    "manifold", "quadarm", "valve", "volute",
];

/// Part `name`'s .fg run, the bench font beside it, and the time it took (ms).
fn build(name: &str) -> (forge_lang::Run, f64) {
    let dir = std::env::temp_dir().join(format!("forge-benchparts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let font = dir.join(forge_agent::bench::FONT);
    if !font.exists() {
        std::fs::write(&font, forge_agent::bench::font_bytes()).expect("font");
    }
    let path = format!("{}/parts/bench/{name}.fg", env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(&path).expect("part file");
    let opts = forge_lang::Options {
        dir: Some(dir),
        ..forge_lang::Options::default()
    };
    let t = std::time::Instant::now();
    let run = forge_lang::run(&src, opts, Default::default()).0;
    let ms = t.elapsed().as_secs_f64() * 1e3;
    assert!(
        run.errors.is_empty(),
        "{name}: {}",
        forge_lang::report::text(&run, &src)
    );
    (run, ms)
}

fn main() {
    let what = std::env::args().nth(1).unwrap_or_default();
    if what != "bits" && what != "check" {
        eprintln!("usage: benchparts bits | check");
        std::process::exit(2);
    }
    for name in NAMES {
        let (run, ms) = build(name);
        let b = run.body.as_ref().expect("a body");
        if what == "bits" {
            let mut h = BitHasher::new();
            for v in &b.vertices {
                h.p3(v.point);
            }
            for e in &b.edges {
                h.f64(e.range.lo);
                h.f64(e.range.hi);
            }
            h.u64(b.faces.len() as u64);
            println!("{name} fg {:016x}", h.finish());
        } else {
            println!(
                "{name}: {} features, {} faces, {} edges, built in {ms:.0} ms",
                run.feats.len(),
                b.faces.len(),
                b.edges.len()
            );
        }
    }
}

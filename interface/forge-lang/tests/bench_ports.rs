//! The holes lane's bench parts (tools/bench-parts), ported by hand to parts/bench: each Forge
//! port builds to what its .fc part builds to, both measured here the same way (volume, faces,
//! edges, bodies), with no errors and no lints. Where a port waits on a feature another lane
//! wires, the .fc lines it leaves out are named, and the .fc part without them is the reference.

use forge_lang::{Options, Run};

/// What a .fc part builds to: (volume, faces, edges, bodies), its bodies finished and joined as
/// Forge's stages join them.
fn fc_measure(src: &str) -> (f64, usize, usize, usize) {
    let m = forge_agent::bench::model(src);
    if let Some(e) = &m.error {
        panic!("the .fc part: line {}: {}", m.lines[e.at].no, e.msg);
    }
    let part = forge_model::Part {
        features: m.features.clone(),
    };
    let solids = part.regenerate_solids().expect("the .fc part builds");
    let mut all = forge_topo::Body::new();
    for s in &solids {
        all.append(&forge_model::finish(s.body.clone()));
    }
    let mass = forge_topo::mass::mass(&all).expect("mass");
    (
        mass.volume,
        all.face_ids().count(),
        all.edge_ids().count(),
        solids.len(),
    )
}

/// A directory holding the bench font, for the ports' text: written once per process, so no
/// test reads it while another is writing it (a truncated font was a flake).
fn font_dir() -> std::path::PathBuf {
    // made once for the whole run: tests on other threads read the font while one would be
    // writing it again (a file is read whole each time, never kept by its size and time)
    static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        // wasi has no temp_dir or pid; its runner opens the whole filesystem
        let d = if cfg!(target_os = "wasi") {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock");
            std::path::PathBuf::from("/tmp").join(format!("forge-lang-bench-{}", t.as_nanos()))
        } else {
            std::env::temp_dir().join(format!("forge-lang-bench-{}", std::process::id()))
        };
        std::fs::create_dir_all(&d).expect("temp dir");
        std::fs::write(d.join(forge_agent::bench::FONT), forge_agent::bench::font_bytes()).expect("font");
        d
    })
    .clone()
}

fn run(name: &str) -> (Run, String) {
    let path = format!("{}/parts/bench/{name}.fg", env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(&path).expect("part file");
    let opts = Options {
        dir: Some(font_dir()),
        ..Options::default()
    };
    (forge_lang::run(&src, opts, Default::default()).0, src)
}

/// The Forge port of bench part `name` against its .fc part less the lines naming `waits`.
fn parity(name: &str, waits: &[&str]) {
    let fc: String = forge_agent::bench::src(name)
        .expect("a bench part")
        .lines()
        .filter(|l| !waits.iter().any(|w| l.contains(w)))
        .map(|l| format!("{l}\n"))
        .collect();
    let want = fc_measure(&fc);
    let (r, src) = run(name);
    let report = forge_lang::report::text(&r, &src);
    assert!(r.errors.is_empty(), "{name}: {report}");
    let lints: Vec<_> = r
        .warnings
        .iter()
        .filter(|w| !matches!(w, forge_lang::error::Warning::UnusedParam { .. }) || waits.is_empty())
        .collect();
    assert!(lints.is_empty(), "{name}: {report}");
    let m = r.measure.as_ref().expect("measured");
    assert!(
        (m.volume - want.0).abs() < 1e-6 * want.0,
        "{name}: volume {} vs .fc {}",
        m.volume,
        want.0
    );
    assert_eq!(
        (m.faces, m.edges, m.bodies, m.problems),
        (want.1, want.2, want.3, 0),
        "{name}: faces, edges, bodies, problems vs the .fc part"
    );
}

#[test]
fn valve() {
    parity("valve", &[]);
}

#[test]
fn hub() {
    parity("hub", &[]);
}

#[test]
fn insert() {
    parity("insert", &[]);
}

#[test]
fn clamp() {
    parity("clamp", &[]);
}

#[test]
fn conrod() {
    parity("conrod", &[]);
}

#[test]
fn heatsink() {
    parity("heatsink", &[]);
}

#[test]
fn bracket() {
    parity("bracket", &[]);
}

#[test]
fn crank() {
    parity("crank", &[]);
}

#[test]
fn jack() {
    parity("jack", &[]);
}

#[test]
fn gearbox() {
    parity("gearbox", &[]);
}

#[test]
fn guard() {
    parity("guard", &[]);
}

#[test]
fn frame() {
    parity("frame", &[]);
}

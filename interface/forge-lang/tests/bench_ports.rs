//! The bench parts (parts/bench), ported by hand from the retired part format: each builds with no
//! errors and no lints to what its original built to. The numbers are the originals' own,
//! measured the way these are (volume, faces, edges, bodies, each body finished and joined as
//! Forge's stages join them) on the last kernel that still read the old format, when every port
//! matched them.

use forge_lang::Options;

/// A directory holding the bench font, for the ports' text: written once per process, so no
/// test reads it while another is writing it (a truncated font was a flake).
fn font_dir() -> std::path::PathBuf {
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
        std::fs::write(d.join(forge_lang::bench::FONT), forge_lang::bench::font_bytes()).expect("font");
        d
    })
    .clone()
}

/// Bench part `name` against its original's (volume, faces, edges, bodies).
fn parity(name: &str, want: (f64, usize, usize, usize)) {
    let src = forge_lang::bench::src(name).expect("a bench part");
    let opts = Options {
        dir: Some(font_dir()),
        ..Options::default()
    };
    let r = forge_lang::run(src, opts, Default::default()).0;
    let report = forge_lang::report::text(&r, src);
    assert!(r.errors.is_empty(), "{name}: {report}");
    assert!(r.warnings.is_empty(), "{name}: {report}");
    let m = r.measure.as_ref().expect("measured");
    assert!(
        (m.volume - want.0).abs() < 1e-6 * want.0,
        "{name}: volume {} vs the original's {}",
        m.volume,
        want.0
    );
    assert_eq!(
        (m.faces, m.edges, m.bodies, m.problems),
        (want.1, want.2, want.3, 0),
        "{name}: faces, edges, bodies, problems vs the original's"
    );
}

#[test]
fn valve() {
    parity("valve", (302785.4380714959, 204, 524, 1));
}

#[test]
fn hub() {
    parity("hub", (233035.39977029603, 46, 114, 1));
}

#[test]
fn insert() {
    parity("insert", (293840.5021819439, 42, 112, 1));
}

#[test]
fn clamp() {
    parity("clamp", (187986.7681180064, 65, 140, 4));
}

#[test]
fn conrod() {
    parity("conrod", (81458.13214465848, 113, 284, 2));
}

#[test]
fn heatsink() {
    parity("heatsink", (82389.83155497232, 438, 1168, 1));
}

#[test]
fn bracket() {
    parity("bracket", (230513.20997850536, 37, 88, 1));
}

#[test]
fn crank() {
    parity("crank", (116672.18051036284, 119, 315, 1));
}

#[test]
fn jack() {
    parity("jack", (54115.855783368, 333, 940, 1));
}

#[test]
fn gearbox() {
    parity("gearbox", (999983.0686296044, 211, 605, 1));
}

#[test]
fn guard() {
    parity("guard", (2439762.3356834813, 74, 195, 2));
}

#[test]
fn frame() {
    parity("frame", (422204.6047341148, 178, 507, 1));
}

#[test]
fn manifold() {
    parity("manifold", (884334.4675612217, 78, 192, 1));
}

#[test]
fn quadarm() {
    parity("quadarm", (19375.961884329805, 73, 167, 1));
}

#[test]
fn volute() {
    parity("volute", (802514.1405986968, 70, 165, 1));
}

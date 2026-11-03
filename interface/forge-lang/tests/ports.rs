//! The fair test's three parts, ported by hand from their .fc versions
//! (fastcad-lab/k/fair/fc/{crank,bracket,manifold}/model.fc): each builds, with the .fc part's
//! volume, face and edge counts, and no errors or lints.

use forge_lang::{Options, Run};

fn build(name: &str) -> (Run, String) {
    let path = format!("{}/parts/{name}.fg", env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(&path).expect("part file");
    (
        forge_lang::run(&src, Options::default(), Default::default()).0,
        src,
    )
}

fn parity(name: &str, volume: f64, faces: usize, edges: usize) {
    parity_bodies(name, volume, faces, edges, 1);
}

fn parity_bodies(name: &str, volume: f64, faces: usize, edges: usize, bodies: usize) {
    let (r, src) = build(name);
    assert!(
        r.errors.is_empty(),
        "{name}: {}",
        forge_lang::report::text(&r, &src)
    );
    assert!(
        r.warnings.is_empty(),
        "{name}: {}",
        forge_lang::report::text(&r, &src)
    );
    let m = r.measure.as_ref().expect("measured");
    // the .fc build reports volume to 0.1 mm3
    assert!(
        (m.volume - volume).abs() < 0.1,
        "{name}: volume {} vs {volume}",
        m.volume
    );
    assert_eq!(
        (m.faces, m.edges, m.bodies, m.problems),
        (faces, edges, bodies, 0),
        "{name}"
    );
    assert!(r.expects.iter().all(|e| e.ok));
}

#[test]
fn crank_matches_its_fc_version() {
    parity("crank", 107177.0, 65, 141);
}

#[test]
fn bracket_matches_its_fc_version() {
    parity("bracket", 183449.2, 137, 316);
}

#[test]
fn manifold_matches_its_fc_version() {
    parity("manifold", 687248.8, 209, 484);
}

// Bench parts (tools/bench-parts), lane fg-shapes. Against the .fc built with the lines that wait
// on other lanes taken out: jack without its modeled thread and text (fg-holes); quadarm whole
// (its root hole's cosmetic thread changes nothing measured).

#[test]
fn bench_jack_matches_its_fc_version_less_thread_and_text() {
    parity("bench_jack", 54989.1, 166, 478);
}

#[test]
fn bench_quadarm_matches_its_fc_version() {
    parity("bench_quadarm", 19376.0, 73, 167);
}

// volute whole (its two holes' cosmetic threads change nothing measured), manifold whole.

#[test]
fn bench_volute_matches_its_fc_version() {
    parity("bench_volute", 802514.1, 70, 165);
}

#[test]
fn bench_manifold_matches_its_fc_version() {
    parity("bench_manifold", 884334.5, 78, 192);
}

// guard without the lines that work `in guard` (louvre, sight hole, tabs, their holes: bodies, lane
// fg-holes): the envelope and the sheet as a body of its own, each valid.

#[test]
fn bench_guard_matches_its_fc_version_less_its_body_targeted_cuts() {
    parity_bodies("bench_guard", 2442097.7, 29, 60, 2);
}

// gearbox less its modeled and cosmetic threads (fg-holes).

#[test]
fn bench_gearbox_matches_its_fc_version_less_threads() {
    parity("bench_gearbox", 1000499.5, 159, 449);
}

// frame less its BSA cups and hanger thread and the cosmetic threads (fg-holes). (Two edges
// short until the kernel's unify stopped keeping seams by name: then the count followed ids.)
#[test]
fn bench_frame_matches_its_fc_version_less_threads() {
    parity("bench_frame", 423092.7, 129, 366);
}

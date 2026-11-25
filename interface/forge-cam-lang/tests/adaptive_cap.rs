//! Adaptive clearing's contract is its engagement cap: level cutting measured over it (by more
//! than 5°, entries aside) is reported on the op, never left silent. Today's adaptive breaks the
//! cap (its cleanup rings and stalled passes, with a 10 mm tool most of the level): this pins the
//! report, and the case flips when the cap is held, deliberately.
mod common;
use std::path::Path;

fn warnings(cap: &str) -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../verify/forge-cam-quality/parts");
    let src = std::fs::read_to_string(dir.join("boss_adaptive_cam.fg"))
        .unwrap()
        .replace("engagement: 40", &format!("engagement: {cap}"));
    let cam = forge_cam_lang::run(&src, &dir);
    assert!(cam.errors.is_empty(), "{:?}", cam.errors);
    cam.warnings
        .into_iter()
        .filter(|w| w.contains("engagement cap"))
        .collect()
}

#[test]
fn cutting_over_the_engagement_cap_is_reported() {
    let w = warnings("40");
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        w[0].starts_with("rough: ") && w[0].contains("more than its 40° engagement cap (up to "),
        "{}",
        w[0]
    );
}

#[test]
fn a_cap_of_a_full_slot_is_never_exceeded() {
    assert!(warnings("180").is_empty());
}

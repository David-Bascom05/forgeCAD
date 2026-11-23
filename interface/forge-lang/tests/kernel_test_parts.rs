//! The Forge parts other tests read (tests/parts) build valid, to the size each was ported at.
mod common;

fn part(name: &str) -> forge_lang::Measure {
    let path = format!("{}/tests/parts/{name}.fg", env!("CARGO_MANIFEST_DIR"));
    common::holds(&std::fs::read_to_string(&path).expect("part file"))
}

fn sized(name: &str, volume: f64, faces: usize, edges: usize) {
    let m = part(name);
    assert!(
        (m.volume - volume).abs() < 1e-6 * volume,
        "{name}: volume {}",
        m.volume
    );
    assert_eq!((m.faces, m.edges, m.bodies), (faces, edges, 1), "{name}");
}

#[test]
fn balls_at_pole() {
    sized("balls-at-pole", 24345.744936, 14, 43);
}

// crank and frame: the ported edge counts less the redundant vertices unify now joins (one
// across an ellipse's start in crank, four between traces of one branch in frame)
#[test]
fn crank() {
    sized("crank", 116600.869023, 153, 413);
}

#[test]
fn frame() {
    sized("frame", 330944.186405, 174, 497);
}

#[test]
fn frame_slot_ids() {
    sized("frame-slot-ids", 122520.328670, 30, 76);
}

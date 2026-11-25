//! No crazy paths on the machined examples (forge_cam_quality::crazy says what that means): one
//! corpus guard per document, each measuring that document alone, so a heavy one does not hold
//! the rest back. Corpus guards have their own budget, 15 s each alone (not the unit tests' 2 s):
//! one over it is a CAM speed bug to report, never a guard to loosen. The benchmark parts are
//! checked by tests/guards.rs, in the measure it already makes of them.
use forge_cam_quality::{Config, crazy, measure};
use std::path::PathBuf;

fn examples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/machined")
}

fn guard(stem: &str) {
    let cfg = Config {
        cell: Some(0.1),
        machine: None,
        feeds: Vec::new(),
        lookahead: None,
    };
    let j = measure(&examples().join(format!("{stem}.fg")), &cfg);
    // (an op refused by its document is not measured: `broken` reads built ops only)
    let (bad, table) = crazy::check(stem, &j.ops);
    assert!(bad.is_empty(), "{}\n\nall:\n{}", bad.join("\n"), table.join("\n"));
}

/// The documents with a corpus guard below: every example must be one of them.
const DOCS: &[&str] = &[
    "block_m6_cam",
    "bracket_cam",
    "flange_cam",
    "manifold_cam",
    "mold_core_cam",
    "mold_core_finish_cam",
    "plate_break_cam",
    "plate_mark_cam",
    "slot_plate_cam",
    "well_island_cam",
];

#[test]
fn every_example_has_a_corpus_guard() {
    let mut found: Vec<String> = std::fs::read_dir(examples())
        .unwrap()
        .filter_map(|e| {
            let n = e.unwrap().file_name().to_string_lossy().into_owned();
            n.strip_suffix(".fg")
                .filter(|s| s.ends_with("_cam"))
                .map(str::to_string)
        })
        .collect();
    found.sort();
    assert_eq!(
        found,
        DOCS.to_vec(),
        "add a corpus_guard_<doc> test (and its DOCS entry) for each new example"
    );
}

#[test]
fn corpus_guard_block_m6() {
    guard("block_m6_cam");
}

#[test]
fn corpus_guard_bracket() {
    guard("bracket_cam");
}

#[test]
fn corpus_guard_flange() {
    guard("flange_cam");
}

#[test]
fn corpus_guard_manifold() {
    guard("manifold_cam");
}

#[test]
fn corpus_guard_mold_core() {
    guard("mold_core_cam");
}

#[test]
fn corpus_guard_mold_core_finish() {
    guard("mold_core_finish_cam");
}

#[test]
fn corpus_guard_plate_break() {
    guard("plate_break_cam");
}

#[test]
fn corpus_guard_plate_mark() {
    guard("plate_mark_cam");
}

#[test]
fn corpus_guard_slot_plate() {
    guard("slot_plate_cam");
}

#[test]
fn corpus_guard_well_island() {
    guard("well_island_cam");
}

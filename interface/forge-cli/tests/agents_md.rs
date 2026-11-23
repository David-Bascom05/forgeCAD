//! AGENTS.md as an agent reads it: its example part builds valid and lint-free, and each op it
//! shows applies to that part. Native only: they start `forgecad` as a process.
#![cfg(not(target_os = "wasi"))]

use std::process::Command;

const DOC: &str = include_str!("../../../AGENTS.md");

/// The doc's fenced blocks, in order.
fn blocks() -> Vec<&'static str> {
    DOC.split("```").skip(1).step_by(2).collect()
}

fn forgecad(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .args(args)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.success(), text)
}

fn example(name: &str) -> String {
    let d = forge_dirs::temp_dir().join(format!("forgecad-agents-md-{name}-{}", forge_dirs::process_id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("e.fg");
    std::fs::write(&f, blocks()[0]).unwrap();
    f.to_str().unwrap().to_string()
}

#[test]
fn the_example_builds_valid_with_no_lints() {
    let f = example("build");
    let (ok, text) = forgecad(&["build", &f]);
    assert!(ok, "{text}");
    assert!(text.contains("valid"), "{text}");
    assert!(!text.contains("warning"), "{text}");
}

#[test]
fn every_op_shown_applies_to_the_example() {
    let ops: Vec<&str> = blocks()
        .iter()
        .flat_map(|b| b.lines())
        .filter(|l| l.starts_with("{\"op\""))
        .collect();
    assert!(ops.len() >= 4, "{ops:?}");
    let f = example("ops");
    for op in ops {
        let (ok, text) = forgecad(&["edit", &f, op]);
        assert!(ok && text.contains("valid"), "{op}: {text}");
    }
}

#[test]
fn help_gives_the_forge_language() {
    let (ok, all) = forgecad(&["help", "forge"]);
    assert!(ok && all.starts_with("# Forge (.fg) reference"), "{all}");
    let (_, holes) = forgecad(&["help", "holes"]);
    assert!(holes.starts_with("## Holes and threads"), "{holes}");
    let (_, sel) = forgecad(&["help", "selectors"]);
    assert!(sel.starts_with("## Selections"), "{sel}");
    let (_, none) = forgecad(&["help"]);
    assert!(none.contains("help forge") && none.contains("Queries"), "{none}");
}

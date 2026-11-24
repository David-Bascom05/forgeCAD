//! A drawing whose part was not there when it was opened draws the part once it is there and the
//! document is built again: the page opens a deep-linked file before the project's other files
//! have come (they follow its first frame, then it is built again), and a part saved beside a
//! drawing that names it is the same on the desktop. Before: the rebuild kept "flange.fg: no such
//! file" and an empty sheet.

use forge_render::gui::{Gui, Headless};

/// The drawing made (not being made again).
fn settled(g: &Gui) -> bool {
    g.shell
        .drawing
        .as_ref()
        .is_some_and(|d| d.made.is_some() && !d.busy)
}

fn missing(g: &Gui) -> Vec<String> {
    g.shell
        .drawing
        .as_ref()
        .and_then(|d| d.made.as_ref())
        .map(|m| {
            m.errors
                .iter()
                .map(|e| format!("{e:?}"))
                .filter(|t| t.contains("no such file"))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_drawing_draws_its_part_once_the_part_is_there() {
    let Ok(mut h) = Headless::new((1280.0, 800.0), 1.0) else {
        eprintln!("no GPU adapter: skipped");
        return;
    };
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = std::env::temp_dir().join(format!("forge-late-part-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(
        repo.join("examples/machined/flange_dwg.fg"),
        dir.join("flange_dwg.fg"),
    )
    .unwrap();
    h.gui.open(dir.join("flange_dwg.fg").to_str().unwrap());
    assert!(h.until(settled), "never made: {}", h.gui.shell.status);
    assert!(!missing(&h.gui).is_empty(), "the part was there already?");

    // the part arrives; what's open is made again (as the page asks once every file has come)
    std::fs::copy(repo.join("examples/machined/flange.fg"), dir.join("flange.fg")).unwrap();
    h.gui.rebuild_open();
    h.step();
    assert!(h.until(settled), "never made again: {}", h.gui.shell.status);
    for _ in 0..5 {
        h.step();
    }
    assert!(
        missing(&h.gui).is_empty(),
        "built again with its part there, still: {:?}",
        missing(&h.gui)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

//! A long session keeps no more sketches than one build made: the build worker frees each step's
//! (`plat::release_sketches` after `Worker::step`), and the UI thread each frame's. A sketch with a
//! named entity is an `Rc` cycle and a forge-lang run frees none of them itself, so without the
//! release every edit of a part kept its sketches, without bound (11 MB after 2000 runs).

use forge_render::gui::{Gui, Headless};
use std::sync::atomic::Ordering;

const PART: &str = "shared t = 6
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 30))
  bore = circle((15, 15), 3)
}
base = extrude(plan, t)
";

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

#[test]
fn edits_keep_no_sketches_between_builds() {
    let Ok(mut h) = Headless::new((1280.0, 800.0), 1.0) else {
        eprintln!("no GPU adapter: skipped");
        return;
    };
    let dir = std::env::temp_dir().join(format!("forge-sketches-bounded-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    h.gui.open(file.to_str().unwrap());
    assert!(h.until(settled), "never built: {}", h.gui.shell.status);
    for k in 0..15 {
        let j = forge_agent::json::Json::parse(&format!(
            r#"{{"op":"SetParam","param":"t","value":{{"value":{},"dim":"Length"}}}}"#,
            5 + k % 3
        ))
        .unwrap();
        let op = forge_lang::error::Op::from_json(&j).unwrap();
        h.gui.fg_apply(op).expect("the edit applies");
        assert!(h.until(settled), "edit {k} never built: {}", h.gui.shell.status);
        h.step();
        assert_eq!(
            forge_render::fg::WORKER_KEPT.load(Ordering::Relaxed),
            0,
            "edit {k}: the build worker kept its sketches"
        );
        assert_eq!(
            forge_keep::kept(),
            0,
            "edit {k}: the UI thread kept sketches after a frame"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

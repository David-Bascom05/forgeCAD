//! A harness's properties in a short window (1280×480): the list is laid in what height there
//! is. Before: `f32::clamp(80, max_h - 150)` panicked once the view was under 350 px tall
//! (min 80 > max), so opening a harness's properties in a short window crashed the app.

use forge_render::gui::{Gui, Headless};
use std::path::Path;

const EXAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/harness");

fn ready(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

#[test]
fn a_harness_s_properties_open_in_a_short_window() {
    let Ok(mut h) = Headless::new((1280.0, 480.0), 1.0) else {
        eprintln!("no GPU adapter: skipped");
        return;
    };
    let d = std::env::temp_dir().join(format!("fg-harness-short-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    for f in ["header.fg", "plate.fg", "robot.fg"] {
        std::fs::copy(Path::new(EXAMPLE).join(f), d.join(f)).unwrap();
    }
    h.step();
    h.gui.open(d.join("robot.fg").to_str().unwrap());
    assert!(h.until(ready), "never built: {}", h.gui.shell.status);
    let v = h.gui.shell.view_rect;
    assert!(
        v.height() < 350.0,
        "the view isn't short enough to show it: {v:?}"
    );
    let a = h.gui.shell.fg.as_mut().unwrap().asm.as_mut().unwrap();
    let loom = a
        .harnesses
        .iter()
        .find(|x| x.id == "loom")
        .expect("the loom harness");
    let key = format!("h:loom:w:{}", loom.wires[0].name);
    a.props = Some(key.clone());
    a.selected = Some(key);
    for _ in 0..6 {
        h.step();
    }
    assert!(
        h.gui.shell.place("asm:props:close").is_some() || h.placed("asm:props").is_some(),
        "no Properties window"
    );
    let _ = std::fs::remove_dir_all(&d);
}

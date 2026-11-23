//! With nothing open the ribbon's tools that need a part are drawn off and do nothing when
//! clicked; Settings still opens; and the commands themselves, sent as the agent or a key would,
//! are refused without a dialog, a sketch pick or a panic.

use forge_render::gui::Headless;
use forge_render::shell::Action;

fn app() -> Option<Headless> {
    match Headless::new((1400.0, 900.0), 1.0) {
        Ok(mut h) => {
            for _ in 0..3 {
                h.step();
            }
            Some(h)
        }
        Err(e) => {
            eprintln!("skipped: {e}");
            None
        }
    }
}

#[test]
fn tools_that_need_a_part_are_off_and_refuse_cleanly_with_none_open() {
    let Some(mut h) = app() else { return };
    assert!(h.gui.shell.fg.is_none());
    // the ribbon's buttons: there, but a click does nothing
    for tool in ["Extrude", "Fillet", "Start Sketch", "Hole"] {
        assert!(h.click(&format!("tool:{tool}")), "no {tool}");
        h.step();
        h.step();
        assert!(!h.gui.shell.sketch_pick, "{tool} began a sketch pick");
        assert!(h.gui.shell.fg.is_none());
    }
    // the empty state's buttons are there to start from
    for name in ["empty:new", "empty:sheet", "empty:open"] {
        assert!(h.gui.shell.place(name).is_some(), "no {name}");
    }
    // the commands themselves, as another caller sends them
    for a in [
        Action::NewFeature("extrude"),
        Action::NewFeature("fillet"),
        Action::StartSketch,
        Action::FinishSketch,
        Action::DialogOk,
        Action::Undo,
    ] {
        h.gui.shell.out.push(a);
        h.step();
        h.step();
        assert!(h.gui.shell.fg.is_none(), "{a:?} opened something");
        assert!(h.gui.shell.sketch.is_none(), "{a:?} began a sketch");
    }
    // Settings still opens
    h.gui.shell.tab = 4;
    h.step();
    assert!(h.click("tool:Settings"));
    for _ in 0..3 {
        h.step();
    }
    assert!(h.gui.shell.prefs.open);
}

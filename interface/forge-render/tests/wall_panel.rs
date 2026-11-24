//! Inspect > Wall: on the desktop the part's thinnest wall is worked out as the panel opens (on a
//! worker), and Cancel stops it: the panel waits for Measure thinnest wall, and no answer from
//! the search that was stopped lands in it. (A page whose jobs run on its one thread waits for the
//! button from the start: web/tests/wall.cjs.)

use forge_render::gui::{Gui, Headless};
use forge_render::shell::inspect::InspectCmd;

const PART: &str = "sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 30))
}
base = extrude(plan, 6)
";

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

fn panel(h: &Headless) -> &forge_render::shell::inspect::MeasurePanel {
    h.gui.shell.inspect.measure.as_ref().expect("the Wall panel")
}

#[test]
fn wall_measures_as_it_opens_and_cancel_stops_it() {
    let Ok(mut h) = Headless::new((1280.0, 800.0), 1.0) else {
        eprintln!("no GPU adapter: skipped");
        return;
    };
    let dir = std::env::temp_dir().join(format!("forge-wall-panel-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    h.gui.open(file.to_str().unwrap());
    assert!(h.until(settled), "never built: {}", h.gui.shell.status);

    // opened: measured, no button pressed
    h.gui.shell.inspect_command(InspectCmd::Wall);
    assert!(
        h.until(|g| g.shell.inspect.measure.as_ref().is_some_and(|m| m.wall_at.is_some())),
        "no wall reading"
    );
    match &panel(&h).wall_at {
        Some(Ok((t, _))) => assert!((t - 6.0).abs() < 1e-9, "the plate's wall: {t}"),
        other => panic!("the plate's wall: {other:?}"),
    }

    // opened again and stopped at once: it waits for the button, and nothing lands after
    h.gui.shell.inspect_command(InspectCmd::Wall);
    h.gui.shell.inspect_command(InspectCmd::Wall);
    h.step();
    h.gui.shell.inspect.measure.as_mut().unwrap().wall_stop = true;
    h.step();
    let m = panel(&h);
    assert!(m.wall_held && !m.busy, "Cancel: held {} busy {}", m.wall_held, m.busy);
    for _ in 0..20 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(panel(&h).wall_at.is_none(), "a stopped search answered: {:?}", panel(&h).wall_at);

    // Measure thinnest wall: worked out again
    h.gui.shell.inspect.measure.as_mut().unwrap().wall_go = true;
    assert!(
        h.until(|g| g.shell.inspect.measure.as_ref().is_some_and(|m| m.wall_at.is_some())),
        "no reading after Measure thinnest wall"
    );
    assert!(!panel(&h).wall_held);
    let _ = std::fs::remove_dir_all(&dir);
}

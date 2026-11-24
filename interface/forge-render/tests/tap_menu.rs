//! A tap on File (the press and its release in one frame: a touchpad's tap, or any click in a
//! page whose frames lag the mouse) opens its menu, and the menu stays open. Before: the menu shut
//! in the frame it opened, the press that opened it taken for a press outside it, so in a browser
//! File never opened at all.

use forge_render::gui::Headless;

const PART: &str = "sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 30))
}
base = extrude(plan, 6)
";

#[test]
fn a_tap_on_file_opens_its_menu() {
    let Ok(mut h) = Headless::new((1280.0, 800.0), 1.0) else {
        eprintln!("no GPU adapter: skipped");
        return;
    };
    let dir = std::env::temp_dir().join(format!("forge-tap-menu-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    h.gui.open(file.to_str().unwrap());
    assert!(
        h.until(|g| g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq)),
        "never built: {}",
        h.gui.shell.status
    );
    let file_button = h
        .gui
        .shell
        .places
        .iter()
        .find(|(n, _)| n == "file")
        .map(|(_, r)| r.center())
        .expect("the File button placed");
    h.move_to(file_button.x, file_button.y);
    h.step();
    // pressed and let go before the next frame
    h.button(true);
    h.button(false);
    h.step();
    h.step();
    h.step();
    assert!(h.gui.shell.file_menu, "File's menu shut in the frame it opened");
    // and a press outside it still shuts it
    h.click_at(700.0, 500.0);
    assert!(
        !h.gui.shell.file_menu,
        "File's menu stayed open after a click outside"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

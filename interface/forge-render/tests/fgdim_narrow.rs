//! A feature selected in a narrow window (480 px): its dimension field is placed in the view
//! however little of it there is. Before: `f32::clamp` panicked (min 376.2 > max 328.0) when the
//! view was narrower than the field and its word, so selecting an extrude crashed the app.

use forge_render::gui::{Gui, Headless};

const PART: &str = "sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 40))
}
extrude1 = extrude(plan, 24)
";

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

#[test]
fn a_feature_selected_in_a_narrow_window_places_its_dimension() {
    let Ok(mut h) = Headless::new((480.0, 860.0), 1.0) else {
        eprintln!("no GPU adapter: skipped");
        return;
    };
    let dir = std::env::temp_dir().join(format!("forge-fgdim-narrow-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    h.gui.open(file.to_str().unwrap());
    assert!(h.until(settled), "never built: {}", h.gui.shell.status);
    h.gui.shell.fg.as_mut().unwrap().selected = Some("extrude1".into());
    for _ in 0..6 {
        h.step();
    }
    assert_eq!(
        h.gui.shell.fg.as_ref().unwrap().selected.as_deref(),
        Some("extrude1")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

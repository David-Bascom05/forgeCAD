//! The status bar in a narrow window (480 px): the status line ends short of Markup, cut with "…"
//! and whole on hover, and leads with the short state (valid, volume, faces) for the cut to keep.
//! Before: it ran on under Markup and the issues chip, both drawn over it.

use forge_render::gui::{Gui, Headless};

const PART: &str = "sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 40))
}
base = extrude(plan, 24)
";

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

#[test]
fn the_status_line_stops_short_of_markup_in_a_narrow_window() {
    let Ok(mut h) = Headless::new((480.0, 860.0), 1.0) else {
        eprintln!("no GPU adapter: skipped");
        return;
    };
    let dir = std::env::temp_dir().join(format!("forge-status-narrow-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("plate.fg");
    std::fs::write(&file, PART).unwrap();
    h.gui.open(file.to_str().unwrap());
    assert!(h.until(settled), "never built: {}", h.gui.shell.status);
    for _ in 0..4 {
        h.step();
    }
    let status = h.gui.shell.status.clone();
    // the short state first, so the cut takes the details: valid, the volume in cm³, the faces
    assert!(status.starts_with("valid · 57.6 cm³ · 6 faces · "), "{status}");
    let said = h.placed("status:said").expect("the status line");
    // cut between its parts, the short state whole
    let shown = h.gui.shell.status_shown.clone();
    assert!(
        shown.starts_with("valid \u{b7} 57.6 cm³") && shown.ends_with(" \u{b7} \u{2026}"),
        "{shown}"
    );
    let markup = h.placed("status:markup").expect("Markup");
    assert!(
        said.right() <= markup.left(),
        "the status \"{status}\" ({said:?}) runs under Markup ({markup:?})"
    );
    if let Ok(d) = std::env::var("FORGE_UI_SHOTS")
        && let Ok(png) = h.png()
    {
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(std::path::Path::new(&d).join("status-480.png"), png).unwrap();
    }
    let _ = std::fs::remove_dir_all(&dir);
}

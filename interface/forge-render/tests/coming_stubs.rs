//! What's coming shows in its dialog, held off: Fillet's conic section (being built in the
//! kernel). Hovering says why; clicking changes nothing the dialog writes. `FORGE_UI_SHOTS=DIR` keeps a picture.

use forge_render::gui::{Gui, Headless};

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

const PART: &str = "shared t = 4
sketch base(on: xy) {
  r = rect(corner: (0, 0), size: (60, 30))
}
plate = extrude(base.r, t)
sketch boss(on: plate.end) {
  c = circle((10, 10), 3)
}
";

#[test]
fn the_conic_section_is_shown_held_off() {
    let dir = std::env::temp_dir().join(format!("fg-coming-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.fg");
    std::fs::write(&file, PART).unwrap();
    let mut h = match Headless::new((1500.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    assert!(h.until(ready));
    for (tool, row, held) in [("Fillet", "Section", "Conic")] {
        assert!(h.click(&format!("tool:{tool}")), "no {tool}");
        assert!(h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some())));
        for _ in 0..4 {
            h.step();
        }
        let place = format!("fg:tool:stub:{row}:{held}");
        assert!(h.gui.shell.place(&place).is_some(), "no {place}");
        let before = h.gui.shell.fg.as_ref().unwrap().tool.clone().unwrap();
        assert!(h.click(&place));
        h.step();
        assert_eq!(
            h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap().text(),
            before.text(),
            "{place} changed what the dialog writes"
        );
        if tool == "Fillet"
            && let Ok(dir) = std::env::var("FORGE_UI_SHOTS")
        {
            let r = h.gui.shell.place(&place).unwrap();
            h.move_to(r.center().x, r.center().y);
            for _ in 0..40 {
                h.step();
            }
            if let Ok(png) = h.png() {
                std::fs::write(std::path::Path::new(&dir).join("stub-fillet-conic.png"), png).unwrap();
            }
        }
        h.key(egui::Key::Escape);
        for _ in 0..3 {
            h.step();
        }
        assert!(
            h.gui.shell.fg.as_ref().unwrap().tool.is_none(),
            "{tool} still open"
        );
    }
}

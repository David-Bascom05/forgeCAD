//! Sheets and thin walls in the app: Extrude's Body (Solid, Thin, Surface) and Revolve's
//! (Solid, Surface), each writing the statement agents write; a sheet's row in the browser shown
//! as a surface, not sheet metal; a thin wall's side. `FORGE_UI_SHOTS=DIR` keeps pictures.

use forge_render::gui::{Gui, Headless};

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn settle(h: &mut Headless) {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 30.0 && !h.until(ready) {}
    assert!(ready(&h.gui), "never ready: {}", h.gui.shell.status);
    for _ in 0..4 {
        h.step();
    }
}

fn shot(h: &mut Headless, name: &str) {
    for _ in 0..12 {
        h.step();
    }
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS")
        && let Ok(png) = h.png()
    {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}.png")), png).unwrap();
    }
}

const PART: &str = "shared h = 20
sketch base(on: xy) {
  r = rect(corner: (0, 0), size: (60, 40))
}
plate = extrude(base.r, 4)
sketch walls(on: plate.end) {
  path wall(start: (5, 5)) {
    line(to: (40, 5))
    line(to: (40, 30))
  }
}
sketch fence(on: plate.end) {
  path f(start: (50, 5)) {
    line(to: (55, 35))
  }
}
";

fn open() -> Option<Headless> {
    let dir = std::env::temp_dir().join(format!("fg-surfaces-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.fg");
    std::fs::write(&file, PART).unwrap();
    let _ = std::fs::remove_file(forge_lang::undo::log_path(&file));
    let mut h = match Headless::new((1500.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    settle(&mut h);
    Some(h)
}

fn tool(h: &mut Headless) -> &mut forge_render::shell::fgtool::FgTool {
    h.gui.shell.fg.as_mut().unwrap().tool.as_mut().unwrap()
}

fn click(h: &mut Headless, place: &str) {
    assert!(h.click(place), "no {place}");
    h.step();
    h.step();
}

fn ok(h: &mut Headless) -> String {
    settle(h);
    let note = h
        .gui
        .shell
        .fg
        .as_ref()
        .unwrap()
        .tool
        .as_ref()
        .unwrap()
        .note
        .clone();
    assert!(note.is_none(), "{note:?}");
    click(h, "fg:tool:ok");
    assert!(h.gui.shell.fg.as_ref().unwrap().tool.is_none());
    settle(h);
    h.gui.shell.fg.as_ref().unwrap().source.clone()
}

#[test]
fn a_surface_and_a_thin_wall_are_made_from_extrudes_dialog() {
    let Some(mut h) = open() else { return };
    // a surface: Body Surface takes a path
    click(&mut h, "tool:Extrude");
    assert!(h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some())));
    click(&mut h, "fg:tool:choice:Body:Surface");
    assert_eq!(tool(&mut h).slots[0].label, "Path");
    // no Side row but with Thin
    assert!(h.gui.shell.place("fg:tool:choice:Side:Both").is_none());
    tool(&mut h).take(Ok("walls.wall".into()));
    tool(&mut h).params[0].text = "15".into();
    shot(&mut h, "surface-1-extrude-dialog");
    let src = ok(&mut h);
    assert!(
        src.contains("extrude1 = extrude(walls.wall, d, surface: true)"),
        "{src}"
    );
    // its row: a surface, not sheet metal
    let row = h
        .gui
        .shell
        .fg
        .as_ref()
        .unwrap()
        .feats
        .iter()
        .find(|f| f.id == "extrude1")
        .cloned()
        .expect("its row");
    assert_eq!(row.kind, "surface");
    assert!(!h.gui.shell.sheet.env, "a surface isn't sheet metal");

    // a thin wall, both sides
    click(&mut h, "tool:Extrude");
    assert!(h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some())));
    click(&mut h, "fg:tool:choice:Body:Thin");
    click(&mut h, "fg:tool:choice:Side:Both");
    tool(&mut h).take(Ok("fence.f".into()));
    shot(&mut h, "surface-2-thin-dialog");
    let src = ok(&mut h);
    assert!(
        src.contains("extrude2 = extrude(fence.f, d, thin: t, side: \"both\")"),
        "{src}"
    );
    let p = h.gui.shell.fg.as_ref().unwrap();
    assert!(
        p.issues
            .iter()
            .all(|i| i.severity != forge_render::shell::fg::Severity::Error),
        "{:?}",
        p.issues
    );
    assert_eq!(
        p.feats
            .iter()
            .find(|f| f.id == "extrude2")
            .map(|f| f.kind.as_str()),
        Some("extrude"),
        "a thin wall is a solid"
    );
    shot(&mut h, "surface-3-browser");
}

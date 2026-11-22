//! A tour of the app's main states, kept as pictures for the UI's review: the empty start, a part
//! open with its browser, the sketcher, a tool's dialog, the measure tool, a refusal, settings in
//! either theme, an assembly and a sheet-metal part. `FORGE_UI_SHOTS=DIR` writes them there;
//! without it the tour only checks that each state opens.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::Action;
use forge_render::shell::inspect::InspectCmd;
use std::path::{Path, PathBuf};

const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn settle(h: &mut Headless) {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 30.0 {
        if h.until(ready) {
            break;
        }
    }
    let _ = h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving()));
    for _ in 0..4 {
        h.step();
    }
}

fn shot(h: &mut Headless, name: &str) {
    // past a window's fade-in
    for _ in 0..12 {
        h.step();
    }
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        if let Ok(png) = h.png() {
            std::fs::write(Path::new(&dir).join(format!("{name}.png")), png).unwrap();
        }
    }
}

fn app(size: (f32, f32)) -> Option<Headless> {
    match Headless::new(size, 1.0) {
        Ok(mut h) => {
            h.step();
            Some(h)
        }
        Err(e) => {
            eprintln!("skipped: {e}");
            None
        }
    }
}

fn copy(rel: &str, tag: &str) -> PathBuf {
    let src = Path::new(EXAMPLES).join(rel);
    let dir = std::env::temp_dir().join(format!("fg-ui-tour-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for e in std::fs::read_dir(src.parent().unwrap()).unwrap().flatten() {
        if e.path().is_file() {
            let _ = std::fs::copy(e.path(), dir.join(e.file_name()));
        }
    }
    let f = dir.join(src.file_name().unwrap());
    let _ = std::fs::remove_file(forge_lang::undo::log_path(&f));
    f
}

fn open(h: &mut Headless, file: &Path) {
    h.gui.open(file.to_str().unwrap());
    settle(h);
}

#[test]
fn the_tour_of_the_main_states() {
    let Some(mut h) = app((1600.0, 1000.0)) else {
        return;
    };
    for _ in 0..4 {
        h.step();
    }
    shot(&mut h, "01-empty");

    let part = copy("demo/part_tools.fg", "part");
    open(&mut h, &part);
    shot(&mut h, "02-part");

    // hover over the part
    let r = h.gui.shell.view_rect;
    h.move_to(r.center().x, r.center().y);
    let _ = h.settle_hover();
    shot(&mut h, "03-hover");

    // a tool's dialog
    if h.click("tool:Fillet") {
        let _ = h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some()));
        settle(&mut h);
        shot(&mut h, "04-tool-dialog");
        h.key(egui::Key::Escape);
        h.step();
        h.step();
    }

    // the sketcher, on XY
    if h.click("tool:Start Sketch") {
        h.step();
        shot(&mut h, "05-sketch-pick");
        h.key(egui::Key::Escape);
        h.step();
    }

    // measure
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Measure));
    for _ in 0..10 {
        h.step();
    }
    shot(&mut h, "06-measure");
    h.key(egui::Key::Escape);
    h.key(egui::Key::Escape);
    h.step();

    // settings, dark then light
    h.gui.shell.tab = 4;
    h.step();
    shot(&mut h, "07-manage-tab");
    if h.click("tool:Settings") {
        for _ in 0..4 {
            h.step();
        }
        shot(&mut h, "08-settings-dark");
        if h.click("settings:theme:Light") {
            for _ in 0..4 {
                h.step();
            }
            shot(&mut h, "09-settings-light");
        }
        h.key(egui::Key::Escape);
        h.gui.shell.prefs.open = false;
        h.step();
    }
    h.gui.shell.tab = 0;
    for _ in 0..3 {
        h.step();
    }
    shot(&mut h, "10-part-light");
}

#[test]
fn the_tour_of_a_refusal_assembly_and_sheet_metal() {
    let Some(mut h) = app((1600.0, 1000.0)) else {
        return;
    };
    // a part with an error in it
    let dir = std::env::temp_dir().join(format!("fg-ui-tour-{}-err", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("bad.fg");
    std::fs::write(
        &bad,
        "shared L = 60\nshared W = 30\nsketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (L, W))\n}\nbase = extrude(plan, 6)\nround = fillet(edges(of: base, crease: \"convex\"), 40)\n",
    )
    .unwrap();
    open(&mut h, &bad);
    shot(&mut h, "11-error");
    let _ = h.click("fg:issues:status");
    for _ in 0..3 {
        h.step();
    }
    shot(&mut h, "12-error-issues");

    let asm = copy("unified/hinged_box.fg", "asm");
    open(&mut h, &asm);
    shot(&mut h, "13-assembly");

    let sm = copy("sheetmetal/load_tray.fg", "sm");
    open(&mut h, &sm);
    shot(&mut h, "14-sheetmetal");

    let dwg = copy("machined/flange_dwg.fg", "dwg");
    open(&mut h, &dwg);
    shot(&mut h, "15-drawing");
}

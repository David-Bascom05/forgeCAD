//! The simulation in the app, native and headless: the flange's machining document opens on
//! Manufacture, its run is simulated and driven by the simulation's own words
//! (`Gui::camsim_command`); a turning document gets no milling simulation (its run is a lathe's).
//! `FORGE_UI_SHOTS=DIR` keeps the review set (docs/ux/STANDARD.md §10: 1400×860, 1600×1000 and
//! 1024×700).

use forge_render::gui::{Gui, Headless};
use std::path::{Path, PathBuf};

fn machined() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined")).to_path_buf()
}

/// A copy of `files` from examples/machined in a directory of the test's own.
fn copied(tag: &str, files: &[&str]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fg-camsim-ui-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in files {
        std::fs::copy(machined().join(f), dir.join(f)).unwrap();
    }
    dir
}

fn run_done(g: &Gui) -> bool {
    g.shell
        .cam
        .as_ref()
        .is_some_and(|w| w.run.is_some() && w.running.is_none() && !w.want_run)
}

/// Opens `file` at `size`, waits for its run (and, if `sim`, its simulation).
fn open(file: &Path, size: (f32, f32), sim: bool) -> Option<Headless> {
    let mut h = match Headless::new(size, 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 120.0 {
        let ready = |g: &Gui| run_done(g) && (!sim || g.shell.camsim.is_some());
        if h.until(ready) {
            break;
        }
    }
    assert!(run_done(&h.gui), "never ran: {}", h.gui.shell.status);
    for _ in 0..6 {
        h.step();
    }
    Some(h)
}

fn shot(h: &mut Headless, name: &str) {
    for _ in 0..8 {
        h.step();
    }
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS")
        && let Ok(png) = h.png()
    {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(Path::new(&dir).join(format!("{name}.png")), png).unwrap();
    }
}

/// The flange plays, at each review size: start, mid-op, the end compared, the move list, the
/// posted code on its machine.
#[test]
fn the_flange_simulates_in_the_app() {
    let dir = copied("flange", &["flange.fg", "flange_cam.fg"]);
    for (w, hgt) in [(1400.0, 860.0), (1600.0, 1000.0), (1024.0, 700.0)] {
        let Some(mut h) = open(&dir.join("flange_cam.fg"), (w, hgt), true) else {
            return;
        };
        let tag = format!("{}x{}", w as u32, hgt as u32);
        assert!(h.gui.camsim_state().is_some(), "the simulation's state");
        assert_eq!(h.gui.shell.camsim.as_ref().unwrap().sim.tl.ops.len(), 5);
        h.gui.camsim_command("t=0;part=hidden").unwrap();
        shot(&mut h, &format!("native-sim-01-start-{tag}"));
        h.gui.camsim_command("op=2;at=0.55").unwrap();
        shot(&mut h, &format!("native-sim-02-contour-{tag}"));
        h.gui.camsim_command("at=1;colour=compare;part=hidden").unwrap();
        shot(&mut h, &format!("native-sim-03-end-compare-{tag}"));
        h.gui
            .camsim_command("colour=material;part=hidden;op=3;locate=3,0,40,0.5;tab=moves")
            .unwrap();
        shot(&mut h, &format!("native-sim-04-moves-{tag}"));
        if h.gui
            .camsim_command("tab=program;source=machine;machine=1;at=0.3")
            .is_ok()
        {
            shot(&mut h, &format!("native-sim-05-machine-{tag}"));
        }
    }
}

/// A turning document is a lathe's run: no milling simulation of it, and no milling errors in
/// the app for it (the web build once showed one).
#[test]
fn a_turning_document_gets_no_milling_simulation() {
    let dir = copied("shaft", &["shaft.fg", "shaft_turn.fg"]);
    let Some(mut h) = open(&dir.join("shaft_turn.fg"), (1400.0, 860.0), false) else {
        return;
    };
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert_eq!(w.run.as_ref().unwrap().process, "turning");
    for _ in 0..30 {
        h.step();
    }
    assert!(
        h.gui.shell.camsim.is_none(),
        "a milling simulation of a turning document"
    );
    shot(&mut h, "native-sim-06-turning-no-sim-1400x860");
}

/// Docked: an environment's properties column hosts the panel (`Shell::camsim_properties`, drawn
/// before the simulation's own chrome each frame); the floating panel then stands down and the
/// view's free part reaches its right edge. Hosting stops, the panel floats again.
#[test]
fn the_panel_docks_in_an_environments_properties() {
    let dir = copied("dock", &["flange.fg", "flange_cam.fg"]);
    let Some(mut h) = open(&dir.join("flange_cam.fg"), (1400.0, 860.0), true) else {
        return;
    };
    let ctx = egui::Context::default();
    let host = |h: &mut Headless| {
        let mut drawn = false;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let r = egui::Rect::from_min_size(egui::pos2(1000.0, 100.0), egui::vec2(340.0, 700.0));
            drawn = h.gui.shell.camsim_properties(ui, r);
        });
        // this test's own context: its font atlas is never uploaded (debug builds check)
        out.textures_delta.clear();
        drawn
    };
    assert!(host(&mut h), "a simulation to host");
    h.step();
    let st = h.gui.shell.camsim.as_ref().unwrap();
    assert!(st.docked, "hosted before the frame: docked");
    assert!(!st.hosted, "taken by the frame");
    // not hosted on the next frame: floating again
    h.step();
    assert!(!h.gui.shell.camsim.as_ref().unwrap().docked);
}

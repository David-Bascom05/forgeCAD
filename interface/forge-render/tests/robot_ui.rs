//! The Robot workspace (research/export/urdf-editor.md §6) in the app, headless: a robot document
//! opens its source assembly with the Robot panel; driving joints moves the parts as the robot's
//! own forward kinematics says (the panel's values held within limits, a frozen joint not
//! driven); selecting a joint shows its values. `FORGE_UI_SHOTS=DIR` writes the pictures
//! (research/export/urdf-ui/); without an adapter the test says so and stops.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::robot_ui::Sel;
use std::path::{Path, PathBuf};

const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
        && g.shell
            .robot
            .as_ref()
            .is_some_and(|r| r.view.is_some() && !r.stale)
}

fn settle(h: &mut Headless) {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 60.0 {
        if h.until(ready) {
            break;
        }
    }
    let _ = h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving()));
    for _ in 0..6 {
        h.step();
    }
}

fn shot(h: &mut Headless, name: &str) {
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

/// The robot example's folder copied (the app writes beside a file it opens).
fn copy() -> PathBuf {
    let src = Path::new(EXAMPLES).join("robot");
    let dir = std::env::temp_dir().join(format!("fg-robot-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for e in std::fs::read_dir(&src).unwrap().flatten() {
        if e.path().is_file() {
            let _ = std::fs::copy(e.path(), dir.join(e.file_name()));
        }
    }
    dir.join("arm2_robot.fg")
}

/// Where the scene has the placement of the occurrence at `path`.
fn rows(h: &Headless, path: &str) -> [[f32; 4]; 3] {
    let (occ, _) = h.gui.fg_asm.as_ref().expect("an assembly");
    let pl = occ
        .iter()
        .find(|o| o.path == path)
        .and_then(|o| o.placement)
        .expect("placed");
    h.gui.view.as_ref().expect("a view").scene.placements[pl].rows
}

#[test]
fn a_robot_opens_with_its_panel_and_its_joints_move_the_parts() {
    let h = match Headless::new((1600.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    let mut h = h;
    h.step();
    let file = copy();
    h.gui.open(file.to_str().unwrap());
    settle(&mut h);
    let p = h.gui.shell.robot.as_ref().expect("the robot panel");
    assert!(p.error.is_none(), "{:?}", p.error);
    let view = p.view.as_ref().expect("the view");
    assert_eq!(view.root, "base");
    assert!(view.joints.iter().any(|j| j.name == "elbow"));
    shot(&mut h, "01-robot-open");

    let before = rows(&h, "fore");
    let p = h.gui.shell.robot.as_mut().expect("panel");
    assert!(p.set("shoulder", 50f64.to_radians()));
    assert!(p.set("elbow", 300f64.to_radians()), "held within its limits");
    assert_eq!(p.q["elbow"], 90f64.to_radians(), "the robot's own limit (±90°)");
    // the frozen grip is fixed in the robot: not a joint to drive
    assert!(!p.set("grip", 0.005));
    p.sel = Some(Sel::Joint("elbow".into()));
    for _ in 0..4 {
        h.step();
    }
    let after = rows(&h, "fore");
    assert_ne!(before, after, "driving the shoulder and elbow moves the forearm");
    // where the robot's forward kinematics puts it
    let p = h.gui.shell.robot.as_ref().expect("panel");
    let v = p.view.as_ref().expect("view");
    let placed = v.occurrences(&v.place(&p.q));
    let f = placed.iter().find(|x| x.0 == "fore").expect("fore").1;
    let want = forge_render::fg::rows_of(&f);
    for i in 0..3 {
        for j in 0..4 {
            assert!((after[i][j] - want[i][j]).abs() < 1e-3, "rows {i}{j}");
        }
    }
    shot(&mut h, "02-robot-driven-joint");

    let p = h.gui.shell.robot.as_mut().expect("panel");
    p.sel = Some(Sel::Link("forearm".into()));
    shot(&mut h, "03-robot-link");
    let p = h.gui.shell.robot.as_mut().expect("panel");
    p.home();
    for _ in 0..4 {
        h.step();
    }
    assert_eq!(rows(&h, "fore"), before, "back to the export pose");
}

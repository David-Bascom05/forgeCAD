//! The Robot workspace (research/export/urdf-editor.md §6) in the app, headless: a robot document
//! opens its source assembly with the Robot panel; driving joints moves the parts as the robot's
//! own forward kinematics says (the panel's values held within limits, a frozen joint not
//! driven); selecting a joint shows its values. `FORGE_UI_SHOTS=DIR` writes the pictures
//! (research/export/urdf-ui/); without an adapter the test says so and stops.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::robot::Sel;
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
fn copy(tag: &str) -> PathBuf {
    let src = Path::new(EXAMPLES).join("robot");
    let dir = std::env::temp_dir().join(format!("fg-robot-ui-{}-{tag}", std::process::id()));
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
    let file = copy("open");
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

fn robot_text(file: &Path) -> String {
    std::fs::read_to_string(file).unwrap()
}

/// Steps until the panel has done what it was asked (its asks taken, the export run again).
fn settle_panel(h: &mut Headless) {
    for _ in 0..8 {
        h.step();
    }
    settle(h);
}

#[test]
fn the_panel_edits_the_robot_document_and_the_source() {
    let Ok(mut h) = Headless::new((1600.0, 1000.0), 1.0) else {
        eprintln!("skipped: no adapter");
        return;
    };
    h.step();
    let file = copy("edits");
    let source = file.parent().unwrap().join("arm2.fg");
    h.gui.open(file.to_str().unwrap());
    settle(&mut h);

    // the shoulder's limits overridden from the panel: a statement with the source's value kept
    let p = h.gui.shell.robot.as_mut().expect("panel");
    let shoulder = p
        .view
        .as_ref()
        .unwrap()
        .joints
        .iter()
        .find(|j| j.name == "shoulder")
        .unwrap()
        .clone();
    p.set_joint(
        &shoulder,
        "limit",
        "(-45deg, 45deg)".into(),
        Some("(-170deg, 170deg)".into()),
    );
    p.sel = Some(Sel::Joint("shoulder".into()));
    settle_panel(&mut h);
    let text = robot_text(&file);
    assert!(
        text.contains("joint(src.shoulder, limit: (-45deg, 45deg), limit_was: (-170deg, 170deg))"),
        "{text}"
    );
    let p = h.gui.shell.robot.as_ref().expect("panel");
    assert!(p.error.is_none(), "{:?}", p.error);
    let sh = p
        .view
        .as_ref()
        .unwrap()
        .joints
        .iter()
        .find(|j| j.name == "shoulder")
        .unwrap();
    assert!((sh.limit.unwrap().1 - 45f64.to_radians()).abs() < 1e-12);
    shot(&mut h, "04-robot-override-limits");

    // undo: the document as it was
    h.gui
        .shell
        .robot
        .as_mut()
        .unwrap()
        .out
        .push(forge_render::shell::robot::RobotAct::Undo);
    settle_panel(&mut h);
    let said = h.gui.shell.robot.as_ref().unwrap().said.clone();
    assert!(
        !robot_text(&file).contains("joint(src.shoulder"),
        "{said}: {}",
        robot_text(&file)
    );

    // the source's elbow limits change under the robot's override: a conflict, kept
    let src = robot_text(&source).replace("param elbow.hi = 135deg", "param elbow.hi = 120deg");
    std::fs::write(&source, src).unwrap();
    h.gui.shell.robot.as_mut().unwrap().stale = true;
    settle_panel(&mut h);
    let p = h.gui.shell.robot.as_ref().expect("panel");
    assert_eq!(p.conflicts.len(), 1, "{:?}", p.error);
    assert_eq!(p.conflicts[0].now, "(-135deg, 120deg)");
    let c = p.conflicts[0].clone();
    shot(&mut h, "05-robot-conflict");
    h.gui
        .shell
        .robot
        .as_mut()
        .unwrap()
        .out
        .push(forge_render::shell::robot::RobotAct::Edit(
            forge_lang::robot::Edit::Set {
                line: Some(c.line),
                call: String::new(),
                target: String::new(),
                field: "limit_was".into(),
                text: c.now.clone(),
                was: None,
            },
        ));
    settle_panel(&mut h);
    let p = h.gui.shell.robot.as_ref().expect("panel");
    assert!(p.conflicts.is_empty() && p.error.is_none(), "{:?}", p.error);
    assert!(robot_text(&file).contains("limit_was: (-135deg, 120deg)"));

    // the shoulder's effort is the document's actuation(effort: eff), shown as such
    let p = h.gui.shell.robot.as_ref().expect("panel");
    let sh = p
        .view
        .as_ref()
        .unwrap()
        .joints
        .iter()
        .find(|j| j.name == "shoulder")
        .unwrap();
    assert_eq!((sh.effort, sh.velocity), (Some(5.0), Some(1.0)));

    // the elbow's robot limits written in the assembly: the source changes, the override goes
    let line = p
        .overrides
        .iter()
        .find(|o| o.1 == "joint(elbow)" && o.2 == "limit")
        .unwrap()
        .0;
    h.gui
        .shell
        .robot
        .as_mut()
        .unwrap()
        .out
        .push(forge_render::shell::robot::RobotAct::Assembly {
            joint: "elbow".into(),
            arg: "limit".into(),
            text: "(-100deg, 100deg)".into(),
            then: forge_lang::robot::Edit::Reset {
                line,
                field: "limit".into(),
            },
        });
    settle_panel(&mut h);
    let said = h.gui.shell.robot.as_ref().unwrap().said.clone();
    assert!(robot_text(&source).contains("limit: (-100deg, 100deg)"), "{said}");
    let text = robot_text(&file);
    assert!(text.contains("joint(src.elbow, damping: 0.1)"), "{said}: {text}");
    let p = h.gui.shell.robot.as_ref().expect("panel");
    assert!(p.error.is_none(), "{:?}", p.error);
    let el = p
        .view
        .as_ref()
        .unwrap()
        .joints
        .iter()
        .find(|j| j.name == "elbow")
        .unwrap();
    assert!(
        (el.limit.unwrap().1 - 100f64.to_radians()).abs() < 1e-12,
        "the assembly's own now"
    );

    // export: the URDF beside the document
    h.gui
        .shell
        .robot
        .as_mut()
        .unwrap()
        .out
        .push(forge_render::shell::robot::RobotAct::Export);
    settle_panel(&mut h);
    let out = file.parent().unwrap().join("arm2_robot_urdf/arm2_robot.urdf");
    assert!(out.is_file(), "{}", h.gui.shell.robot.as_ref().unwrap().said);
}

#[test]
fn the_tree_is_edited_from_the_panel() {
    use forge_lang::robot::Edit;
    use forge_render::shell::robot::RobotAct;
    let Ok(mut h) = Headless::new((1600.0, 1000.0), 1.0) else {
        eprintln!("skipped: no adapter");
        return;
    };
    h.step();
    let file = copy("tree");
    h.gui.open(file.to_str().unwrap());
    settle(&mut h);
    let ask = |h: &mut Headless, e: Edit| {
        h.gui.shell.robot.as_mut().unwrap().out.push(RobotAct::Edit(e));
        settle_panel(h);
        let p = h.gui.shell.robot.as_ref().unwrap();
        assert!(p.error.is_none(), "{:?}", p.error);
    };
    // the elbow frozen where the panel has it (-60deg), then the forearm merged into the upper arm
    ask(
        &mut h,
        Edit::Add {
            text: "freeze(src.elbow, at: -60deg)".into(),
        },
    );
    let v = h.gui.shell.robot.as_ref().unwrap().view.clone().unwrap();
    assert_eq!(v.joints.iter().find(|j| j.name == "elbow").unwrap().kind, "fixed");
    ask(
        &mut h,
        Edit::Add {
            text: "merge(src.upper, forearm)".into(),
        },
    );
    let v = h.gui.shell.robot.as_ref().unwrap().view.clone().unwrap();
    assert!(v.links.iter().all(|l| l.name != "forearm"));
    let upper = v.links.iter().find(|l| l.name == "upper").unwrap();
    assert!(upper.bodies.contains(&"fore".to_string()) && upper.bodies.contains(&"finger".to_string()));
    // the upper arm the root, a frame on it, its collision its hull
    ask(
        &mut h,
        Edit::Put {
            call: "root".into(),
            text: "root(src.upper)".into(),
        },
    );
    ask(
        &mut h,
        Edit::Add {
            text: "cam = frame(on: src.upper, at: pose(at: (60, 0, 20)))".into(),
        },
    );
    ask(
        &mut h,
        Edit::Add {
            text: "collision(src.upper, hull)".into(),
        },
    );
    let p = h.gui.shell.robot.as_ref().unwrap();
    let v = p.view.as_ref().unwrap();
    assert_eq!(v.root, "upper");
    assert!(v.joints.iter().any(|j| j.child == "cam" && j.parent == "upper"));
    let text = robot_text(&file);
    assert!(
        text.contains("root(src.upper)") && text.contains("merge(src.upper, forearm)"),
        "{text}"
    );
    // the parts still where the robot's kinematics puts them
    let placed = v.occurrences(&v.place(&p.q));
    let f = placed.iter().find(|x| x.0 == "fore").expect("fore").1;
    let want = forge_render::fg::rows_of(&f);
    for _ in 0..4 {
        h.step();
    }
    let got = rows(&h, "fore");
    for i in 0..3 {
        for j in 0..4 {
            assert!((got[i][j] - want[i][j]).abs() < 1e-3);
        }
    }
    h.gui.shell.robot.as_mut().unwrap().sel = Some(Sel::Link("upper".into()));
    shot(&mut h, "06-robot-tree-edited");
}

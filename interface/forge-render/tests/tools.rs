//! Each enabled tool of the ribbon as a person uses it: its button, picks made by pointing, values
//! typed, OK; then what the file now builds is measured.

use forge_agent::model::Model;
use forge_render::gui::Headless;
use forge_render::shell::Action;

const BLOCK: &str = "base: extrude 20 rect -30 -30 60 60\n";

fn open(text: &str, tag: &str) -> Option<(Headless, std::path::PathBuf)> {
    let dir = std::env::temp_dir().join(format!("fc-tool-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.fc");
    std::fs::write(&file, text).unwrap();
    let mut h = Headless::new((1200.0, 800.0), 1.0).ok()?;
    h.step();
    h.gui.open(file.to_str().unwrap());
    h.settle();
    h.until(|g| g.view.as_ref().is_some_and(|v| !v.facts.face_item.is_empty()));
    for _ in 0..3 {
        h.step();
    }
    Some((h, file))
}

/// Clicks where this place in the world is on screen.
fn point_at(h: &mut Headless, world: [f32; 3]) {
    let (cam, size) = h.gui.shell.camera.unwrap();
    let on = cam.pixel_of(world, size.0, size.1).unwrap();
    let r = h.gui.shell.view_rect;
    h.move_to(r.min.x + on.0, r.min.y + on.1);
    for _ in 0..30 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    h.button(true);
    h.step();
    h.button(false);
    for _ in 0..30 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

fn field(h: &mut Headless, label: &str, text: &str) {
    let d = h.gui.shell.dialog.as_mut().expect("a dialog");
    let labels: Vec<String> = d.fields.iter().map(|f| f.0.clone()).collect();
    let f = d
        .fields
        .iter_mut()
        .find(|f| f.0 == label)
        .unwrap_or_else(|| panic!("no field {label} in {labels:?}"));
    f.1 = text.to_string();
    h.step();
}

fn ok(h: &mut Headless, what: &str) {
    assert!(
        h.until(|g| {
            let d = g.shell.dialog.as_ref().unwrap();
            d.error.is_some() || (d.seq > 0 && !d.waiting_for_pick())
        }),
        "{what}: no live result: {:?}",
        h.gui.shell.dialog
    );
    let d = h.gui.shell.dialog.as_ref().unwrap();
    assert!(d.error.is_none(), "{what}: {:?} for `{}`", d.error, d.line());
    h.gui.shell.out.push(Action::DialogOk);
    assert!(
        h.until(|g| g.shell.dialog.is_none()),
        "{what}: OK did not close it: {:?}",
        h.gui.shell.dialog
    );
}

/// What the file builds: it builds, it's valid, and its volume.
fn built(file: &std::path::Path, what: &str) -> f64 {
    let text = std::fs::read_to_string(file).unwrap();
    let mut m = Model::new(&text);
    assert!(m.error.is_none(), "{what}: {:?}\n{text}", m.error);
    let b = m.body().unwrap_or_else(|| panic!("{what}: no body\n{text}"));
    let problems = forge_topo::check::check(b);
    assert!(problems.is_empty(), "{what}: invalid: {problems:?}\n{text}");
    m.mass().unwrap_or_else(|e| panic!("{what}: {e}")).volume
}

fn button(h: &mut Headless, tool: &str) {
    h.click(&format!("tool:{tool}"));
    assert!(h.until(|g| g.shell.dialog.is_some()), "{tool}: no dialog");
    for _ in 0..3 {
        h.step();
    }
}

#[test]
fn fillet_rounds_the_edge_picked() {
    let Some((mut h, file)) = open(BLOCK, "fillet") else {
        return;
    };
    let was = built(&file, "before");
    button(&mut h, "Fillet");
    point_at(&mut h, [30.0, 0.0, 20.0]);
    field(&mut h, "Radius", "4");
    ok(&mut h, "fillet");
    let now = built(&file, "fillet");
    assert!(now < was && now > was * 0.9, "{was} {now}");
}

#[test]
fn chamfer_cuts_the_edge_picked() {
    let Some((mut h, file)) = open(BLOCK, "chamfer") else {
        return;
    };
    let was = built(&file, "before");
    button(&mut h, "Chamfer");
    point_at(&mut h, [30.0, 0.0, 20.0]);
    ok(&mut h, "chamfer");
    assert!(built(&file, "chamfer") < was);
}

// off in the ribbon: it builds on a plain block and fails on every bench part tried
#[test]
#[ignore]
fn shell_hollows_out_with_the_face_picked_open() {
    let Some((mut h, file)) = open(BLOCK, "shell") else {
        return;
    };
    let was = built(&file, "before");
    button(&mut h, "Shell");
    point_at(&mut h, [0.0, 0.0, 20.0]);
    field(&mut h, "Thickness", "2");
    ok(&mut h, "shell");
    let now = built(&file, "shell");
    assert!(now < was * 0.5, "{was} {now}");
}

#[test]
fn draft_tapers_the_face_picked() {
    let Some((mut h, file)) = open(BLOCK, "draft") else {
        return;
    };
    let was = built(&file, "before");
    button(&mut h, "Draft");
    point_at(&mut h, [30.0, 0.0, 10.0]);
    ok(&mut h, "draft");
    assert!((built(&file, "draft") - was).abs() > 1.0);
}

#[test]
fn thread_runs_on_the_cylinder_picked() {
    let boss = "base: extrude 20 circle 0 0 10\n";
    let Some((mut h, file)) = open(boss, "thread") else {
        return;
    };
    let was = built(&file, "before");
    button(&mut h, "Thread");
    let c = core::f32::consts::FRAC_1_SQRT_2 * 10.0;
    point_at(&mut h, [c, -c, 10.0]);
    ok(&mut h, "thread");
    assert!((built(&file, "thread") - was).abs() > 0.1);
}

const SKETCHED: &str = "sk1: sketch xz\n  rect 10 0 10 20\nend\n";

#[test]
fn extrude_reads_the_free_sketch() {
    let Some((mut h, file)) = open(SKETCHED, "extrude") else {
        return;
    };
    button(&mut h, "Extrude");
    ok(&mut h, "extrude");
    let v = built(&file, "extrude");
    assert!((v - 10.0 * 20.0 * 10.0).abs() < 1e-6, "{v}");
}

#[test]
fn revolve_reads_the_free_sketch() {
    let Some((mut h, file)) = open(SKETCHED, "revolve") else {
        return;
    };
    button(&mut h, "Revolve");
    ok(&mut h, "revolve");
    let v = built(&file, "revolve");
    let want = core::f64::consts::PI * (400.0 - 100.0) * 20.0;
    assert!((v - want).abs() < want * 0.01, "{v} {want}");
}

#[test]
fn hole_drills_at_each_sketch_point_picked() {
    let part = "base: extrude 20 rect -30 -30 60 60\nsk1: sketch xy at 0 0 20\n  pt 10 10\n  circle -10 -10 3\nend\n";
    let Some((mut h, file)) = open(part, "hole") else {
        return;
    };
    let was = built(&file, "before");
    button(&mut h, "Hole");
    assert!(
        h.gui.shell.dialog.as_ref().unwrap().waiting_for_pick(),
        "nothing till it's placed"
    );
    point_at(&mut h, [10.0, 10.0, 20.0]);
    point_at(&mut h, [-10.0, -10.0, 20.0]);
    let d = h.gui.shell.dialog.as_ref().unwrap();
    assert!(d.line().contains("pts p1 c1"), "{}", d.line());
    ok(&mut h, "hole");
    let v = built(&file, "hole");
    let each = core::f64::consts::PI * 2.5 * 2.5 * 20.0;
    assert!((was - v - 2.0 * each).abs() < each * 0.02, "{was} {v}");
}

#[test]
fn the_tools_with_no_passing_test_are_off() {
    let Some(mut h) = open(BLOCK, "off").map(|o| o.0) else {
        return;
    };
    for name in [
        "Sweep", "Loft", "Coil", "Rib", "Shell", "Combine", "Plane", "Axis", "Point", "Linear",
    ] {
        h.click(&format!("tool:{name}"));
        for _ in 0..3 {
            h.step();
        }
        assert!(h.gui.shell.dialog.is_none(), "{name} is on with no test");
    }
}

#[test]
fn a_part_in_inches_takes_and_shows_inches_and_its_geometry_stays() {
    let Some((mut h, file)) = open(BLOCK, "units") else {
        return;
    };
    let was = built(&file, "before");
    h.gui.shell.out.push(Action::SetUnits("in"));
    assert!(h.until(|g| g.shell.units == "in"));
    let written = file.clone();
    h.until(|_| std::fs::read_to_string(&written).unwrap().starts_with("# units"));
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.starts_with("# units in\n"), "{text}");
    assert_eq!(
        built(&file, "units").to_bits(),
        was.to_bits(),
        "only how it reads"
    );
    assert!(h.until(|g| g.shell.rows.len() == 1), "the comment is no feature");
    // a number typed takes the unit; what the part has shows in mm until it's edited
    button(&mut h, "Fillet");
    point_at(&mut h, [30.0, 0.0, 20.0]);
    assert_eq!(
        h.gui
            .shell
            .dialog
            .as_ref()
            .unwrap()
            .fields
            .iter()
            .find(|f| f.0 == "Radius")
            .unwrap()
            .1,
        "2 mm"
    );
    field(&mut h, "Radius", "1/8 in");
    ok(&mut h, "fillet in inches");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("fillet (1/8)in @e"), "{text}");
    let now = built(&file, "fillet");
    assert!(now < was);
    // a part reopened reads in its own unit
    let Some((h2, _)) = open(&text, "units2") else {
        return;
    };
    assert_eq!(h2.gui.shell.units, "in");
}

#[test]
fn extrude_takes_its_extent_and_way_and_operation_from_toggles_and_plus_opens_another() {
    let part = "base: extrude 20 rect -30 -30 60 60\nsk1: sketch xy at 0 0 20\n  rect -5 -5 10 10\nend\n";
    let Some((mut h, file)) = open(part, "toggles") else {
        return;
    };
    let was = built(&file, "before");
    button(&mut h, "Extrude");
    h.click("dialog:op:Cut");
    for _ in 0..3 {
        h.step();
    }
    h.click("dialog:dir:Back");
    for _ in 0..3 {
        h.step();
    }
    field(&mut h, "Distance", "6");
    assert_eq!(h.gui.shell.dialog.as_ref().unwrap().line(), "extrude cut -6 sk1");
    assert!(h.until(|g| {
        let d = g.shell.dialog.as_ref().unwrap();
        d.error.is_some() || d.sent.as_deref() == Some("extrude cut -6 sk1")
    }));
    ok(&mut h, "cut back");
    let v = built(&file, "cut back");
    assert!((was - v - 600.0).abs() < 1e-6, "{was} {v}");
    // through all, from the toggle; + leaves another up
    let was = v;
    let sk2 = "sk2: sketch xy at 0 0 20\n  rect 10 10 5 5\nend\n";
    let text = std::fs::read_to_string(&file).unwrap() + sk2;
    let Some((mut h, file)) = open(&text, "toggles2") else {
        return;
    };
    button(&mut h, "Extrude");
    h.click("dialog:op:Cut");
    for _ in 0..3 {
        h.step();
    }
    h.click("dialog:extent:Through All");
    for _ in 0..3 {
        h.step();
    }
    h.click("dialog:dir:Back");
    assert!(h.until(|g| g.shell.dialog.as_ref().unwrap().line().contains("thru back")));
    assert!(h.until(|g| g.shell.dialog.as_ref().unwrap().seq > 0));
    h.until(|g| {
        g.shell
            .dialog
            .as_ref()
            .is_some_and(|d| d.sent.as_deref() == Some(d.line().as_str()))
    });
    h.click("dialog:apply");
    assert!(
        h.until(|_| std::fs::read_to_string(&file).unwrap().contains("thru back")),
        "{:?}",
        h.gui.shell.dialog
    );
    // no free sketch left: another starts with one
    assert!(
        h.until(|g| g.shell.sketch_pick || g.shell.sketch.is_some()),
        "another's on its way"
    );
    assert!((was - built(&file, "thru") - 25.0 * 20.0).abs() < 1e-6);
}

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
    // what's under it known, as a hand gives it time to be; a sketch has no mark to wait for
    let mut waited = 0;
    loop {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
        waited += 1;
        let v = h.gui.view.as_ref().unwrap();
        let known = v.nav.marks().iter().any(|m| !m.selected) && !v.nav.pick_pending();
        if (known && waited > 10) || waited > 120 {
            break;
        }
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
    for name in ["Rib", "Shell"] {
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

fn settle(h: &mut Headless) {
    for _ in 0..10 {
        h.step();
    }
}

fn items(file: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(file)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn sweep_takes_a_region_then_a_sketch_to_follow() {
    let part = "sk1: sketch xy\n  circle 0 0 5\nend\nsk2: sketch xz\n  line 0 0 0 40\nend\n";
    let Some((mut h, file)) = open(part, "sweep") else {
        return;
    };
    button(&mut h, "Sweep");
    assert!(
        h.gui.shell.dialog.as_ref().unwrap().waiting_for_pick(),
        "nothing from nothing"
    );
    point_at(&mut h, [2.0, 2.0, 0.0]);
    point_at(&mut h, [0.0, 0.0, 20.0]);
    ok(&mut h, "sweep");
    let v = built(&file, "sweep");
    let want = core::f64::consts::PI * 25.0 * 40.0;
    assert!((v - want).abs() < want * 0.01, "{v} {want}");
}

#[test]
fn loft_takes_two_regions_in_order() {
    let part =
        "sk1: sketch xy\n  rect -10 -10 20 20\nend\nsk2: sketch xy at 0 0 30\n  rect -5 -5 10 10\nend\n";
    let Some((mut h, file)) = open(part, "loft") else {
        return;
    };
    button(&mut h, "Loft");
    assert!(h.gui.shell.dialog.as_ref().unwrap().waiting_for_pick());
    point_at(&mut h, [8.0, -8.0, 0.0]);
    assert!(
        h.gui.shell.dialog.as_ref().unwrap().waiting_for_pick(),
        "one's not a loft"
    );
    point_at(&mut h, [0.0, 0.0, 30.0]);
    ok(&mut h, "loft");
    let v = built(&file, "loft");
    // a frustum: 30/3 * (400 + 100 + 200)
    assert!((v - 7000.0).abs() < 70.0, "{v}");
}

#[test]
fn coil_takes_a_region_and_a_line_of_its_sketch_to_turn_about() {
    let part = "sk1: sketch xz\n  circle 20 0 2\n  line 0 -30 0 30 cons\nend\n";
    let Some((mut h, file)) = open(part, "coil") else {
        return;
    };
    button(&mut h, "Coil");
    assert!(h.gui.shell.dialog.as_ref().unwrap().waiting_for_pick());
    point_at(&mut h, [20.0, 0.0, 0.0]);
    point_at(&mut h, [0.0, 0.0, 25.0]);
    settle(&mut h);
    ok(&mut h, "coil");
    let text = items(&file).join("\n");
    assert!(text.contains("axis l1"), "{text}");
    assert!(built(&file, "coil") > 100.0);
}

#[test]
fn linear_pattern_copies_the_feature_picked_along_the_way_chosen() {
    let part = "base: extrude 10 rect -60 -60 120 120\nboss: extrude 5 xy at 0 0 10 rect 0 0 10 10\n";
    let Some((mut h, file)) = open(part, "pattern") else {
        return;
    };
    let was = built(&file, "before");
    button(&mut h, "Linear");
    assert!(h.gui.shell.dialog.as_ref().unwrap().waiting_for_pick());
    point_at(&mut h, [5.0, 5.0, 15.0]);
    h.click("dialog:way:Y");
    settle(&mut h);
    field(&mut h, "Count", "3");
    field(&mut h, "Spacing", "20");
    ok(&mut h, "pattern");
    let text = items(&file).join("\n");
    assert!(text.contains("pattern boss lin 0 1 0 3 20"), "{text}");
    let v = built(&file, "pattern");
    assert!((v - was - 2.0 * 500.0).abs() < 1e-6, "{was} {v}");
}

#[test]
fn combine_joins_the_bodies_picked() {
    let part = "a: extrude 10 rect 0 0 20 20\nb: extrude new 10 rect 25 0 20 20\n";
    let Some((mut h, file)) = open(part, "combine") else {
        return;
    };
    button(&mut h, "Combine");
    assert!(h.gui.shell.dialog.as_ref().unwrap().waiting_for_pick());
    point_at(&mut h, [15.0, 3.0, 10.0]);
    point_at(&mut h, [40.0, 3.0, 10.0]);
    ok(&mut h, "combine");
    let text = items(&file).join("\n");
    assert!(text.contains("combine add a b"), "{text}");
    let v = built(&file, "combine");
    assert!((v - 2.0 * 20.0 * 20.0 * 10.0).abs() < 1e-6, "{v}");
}

#[test]
fn plane_axis_and_point_come_from_what_is_picked() {
    for (tool, at, word) in [
        ("Plane", [0.0f32, 0.0, 20.0], "plane off base @f"),
        ("Axis", [30.0, 0.0, 20.0], "axis base @e"),
        ("Point", [30.0, 0.0, 20.0], "point mid base @e"),
    ] {
        let Some((mut h, file)) = open(BLOCK, tool) else {
            return;
        };
        let was = built(&file, "before");
        button(&mut h, tool);
        assert!(h.gui.shell.dialog.as_ref().unwrap().waiting_for_pick(), "{tool}");
        point_at(&mut h, at);
        ok(&mut h, tool);
        let text = items(&file).join("\n");
        assert!(text.contains(word), "{tool}: {text}");
        assert_eq!(
            built(&file, tool).to_bits(),
            was.to_bits(),
            "{tool} adds no solid"
        );
    }
}

fn to_next(part: &str, tag: &str, cut: bool, back: bool) -> (Result<f64, String>, String) {
    let Some((mut h, file)) = open(part, tag) else {
        return (Err("no GPU".into()), String::new());
    };
    button(&mut h, "Extrude");
    if cut {
        h.click("dialog:op:Cut");
        settle(&mut h);
    }
    h.click("dialog:extent:To Next");
    settle(&mut h);
    if back {
        h.click("dialog:dir:Back");
        settle(&mut h);
    }
    let line = h.gui.shell.dialog.as_ref().unwrap().line();
    let want = line.clone();
    h.until(|g| {
        let d = g.shell.dialog.as_ref().unwrap();
        d.sent.as_deref() == Some(want.as_str()) && d.seq > 0
    });
    for _ in 0..100 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let d = h.gui.shell.dialog.as_ref().unwrap();
    if let Some(e) = &d.error {
        return (Err(e.clone()), line);
    }
    h.gui.shell.out.push(Action::DialogOk);
    assert!(
        h.until(|g| g.shell.dialog.is_none()),
        "{line}: {:?}",
        h.gui.shell.dialog
    );
    (Ok(built(&file, &line)), line)
}

#[test]
fn to_next_stops_at_the_next_face_either_way_or_says_there_is_none() {
    // one body, a C on its side: a floor at z 0..10, a roof at z 40..50, a wall joining them at x 30..40
    let c = "c: extrude 60 yz at -30 0 0 axis y poly -30 0 -30 0 -30 0\n";
    let _ = c;
    let body = "floor: extrude 10 rect -30 -30 70 60\nwall: extrude 40 xy at 0 0 10 rect 30 -30 10 60\nroof: extrude 10 xy at 0 0 50 rect -30 -30 70 60\n";
    let on_plane = |z: f32| format!("{body}sk1: sketch xy at 0 0 {z}\n  rect -5 -5 10 10\nend\n");
    let base = 70.0 * 60.0 * 10.0 * 2.0 + 10.0 * 60.0 * 40.0;
    let area = 100.0;
    // up from 25 to the roof's underside at 50; down to the floor's top at 10
    let (up, l) = to_next(&on_plane(25.0), "next-up", false, false);
    assert!(
        (up.clone().unwrap() - (base + area * 25.0)).abs() < 1e-6,
        "{l}: {up:?}"
    );
    // the kernel's `next back` refuses this today (raised with a repro): it must say so plainly, and
    // once it's fixed this should stop at the floor, at base + area * 15
    let (down, l) = to_next(&on_plane(25.0), "next-down", false, true);
    assert!(
        down.as_ref().is_err_and(|w| !w.contains("extent:"))
            || down
                .as_ref()
                .is_ok_and(|v| (v - (base + area * 15.0)).abs() < 1e-6),
        "{l}: {down:?}"
    );
    // from the floor's top, up to the roof
    let (r, l) = to_next(&on_plane(10.0), "next-face", false, false);
    assert!(
        (r.clone().unwrap() - (base + area * 40.0)).abs() < 1e-6,
        "{l}: {r:?}"
    );
    // a cut down from the top of a block into a pocket's floor: a cavity below stops it
    let cavity = "a: extrude 30 rect -30 -30 60 60\np: extrude cut 10 xy at 0 0 10 rect -20 -20 40 40\nsk1: sketch xy at 0 0 30\n  rect -5 -5 10 10\nend\n";
    let block = 60.0 * 60.0 * 30.0 - 40.0 * 40.0 * 10.0;
    let (r, l) = to_next(cavity, "next-cut", true, true);
    assert!(
        (r.clone().unwrap() - (block - area * 10.0)).abs() < 1e-6,
        "{l}: {r:?}"
    );
    // nothing to reach: it says so, and nothing's written
    let alone = "a: extrude 10 rect -30 -30 60 60\nsk1: sketch xy at 0 0 20\n  rect -5 -5 10 10\nend\n";
    let (r, l) = to_next(alone, "next-none", false, false);
    let why = r.expect_err(&l);
    assert!(!why.is_empty() && !why.contains("Rebuild"), "{l}: {why}");
}

#[test]
fn combine_on_overlapping_bodies_takes_each_from_the_view_or_the_tree() {
    let part = "a: extrude 10 rect 0 0 20 20\nb: extrude new 10 rect 10 0 20 20\n";
    let want = 30.0 * 20.0 * 10.0;
    let Some((mut h, file)) = open(part, "overlap") else {
        return;
    };
    button(&mut h, "Combine");
    point_at(&mut h, [15.0, 3.0, 10.0]);
    point_at(&mut h, [28.0, 3.0, 10.0]);
    ok(&mut h, "from the view");
    assert!((built(&file, "view") - want).abs() < 1e-6);
    let Some((mut h, file)) = open(part, "overlap2") else {
        return;
    };
    button(&mut h, "Combine");
    h.click("tree:a");
    for _ in 0..4 {
        h.step();
    }
    h.click("tree:b");
    ok(&mut h, "from the tree");
    assert!((built(&file, "tree") - want).abs() < 1e-6);
}

fn thread_on(part: &str, at: [f32; 3], tag: &str) -> Option<(Headless, std::path::PathBuf, f64)> {
    let (mut h, file) = open(part, tag)?;
    let was = built(&file, "before");
    button(&mut h, "Thread");
    point_at(&mut h, at);
    Some((h, file, was))
}

#[test]
fn a_thread_on_a_hole_made_cylinder_and_on_any_diameter_comes_from_the_catalogue() {
    let c = core::f32::consts::FRAC_1_SQRT_2;
    // a boss no coarse size is exactly: the nearest fits
    let Some((mut h, file, was)) = thread_on(
        "base: extrude 20 circle 0 0 6.5\n",
        [6.5 * c, -6.5 * c, 10.0],
        "th13",
    ) else {
        return;
    };
    let d = h.gui.shell.dialog.as_ref().unwrap();
    assert_eq!(d.thread.designation, "M14x2", "{}", d.line());
    assert_eq!(d.thread.class, "4h", "an outside thread's classes");
    h.gui
        .shell
        .dialog
        .as_mut()
        .unwrap()
        .choose_thread(None, None, None, Some("6g"));
    ok(&mut h, "boss 13");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.contains("thread M14x2 face base @f") && text.contains("class 6g"),
        "{text}"
    );
    assert!((built(&file, "boss 13") - was).abs() > 0.1, "it cut something");
    // reopened, it's what was chosen
    let id = h.gui.shell.doc.as_ref().unwrap().items.last().unwrap().id;
    h.gui.shell.out.push(Action::Edit(id));
    assert!(h.until(|g| g.shell.dialog.is_some()));
    let d = h.gui.shell.dialog.as_ref().unwrap();
    assert_eq!(
        (d.thread.designation.as_str(), d.thread.class.as_str()),
        ("M14x2", "6g"),
        "{:?}",
        d.fields
    );

    // a hole's wall: the bore's size picks the size whose minor it is, and the thread's inside
    let drilled = "base: extrude 20 rect -30 -30 60 60\nh: hole 12 -xy at 0 0 20 thru\n";
    let Some((mut h, file, was)) = thread_on(drilled, [-6.0 * c, 6.0 * c, 16.0], "thhole") else {
        return;
    };
    let d = h.gui.shell.dialog.as_ref().unwrap();
    assert_eq!(d.thread.designation, "M14x2", "{}", d.line());
    assert!(d.thread.internal);
    assert_eq!(d.thread.class, "5H");
    ok(&mut h, "tapped");
    assert!((built(&file, "tapped") - was).abs() > 0.1);

    // a type that can't be cut says so, and OK waits
    let Some((mut h, _, _)) = thread_on(
        "base: extrude 20 circle 0 0 10\n",
        [10.0 * c, -10.0 * c, 10.0],
        "thtr",
    ) else {
        return;
    };
    h.gui
        .shell
        .dialog
        .as_mut()
        .unwrap()
        .choose_thread(Some("ISO Trapezoidal"), None, None, None);
    for _ in 0..3 {
        h.step();
    }
    let d = h.gui.shell.dialog.as_ref().unwrap();
    assert!(d.thread_blocked().is_some() && d.waiting_for_pick());
    // the same pick, left-handed, in inches
    let d = h.gui.shell.dialog.as_mut().unwrap();
    d.choose_thread(Some("UNC"), Some("3/4"), None, None);
    d.fields.iter_mut().find(|f| f.0 == "Hand").unwrap().1 = "left".to_string();
    assert!(d.thread_blocked().is_none());
    assert!(d.line().starts_with("thread 0.75-10 face"), "{}", d.line());
    assert!(d.line().contains("full left class 1A"), "{}", d.line());
}

const FOR_HOLES: &str = "base: extrude 20 rect -30 -30 60 60\nsk1: sketch xy at 0 0 20\n  pt 10 10\nend\n";

/// A hole dialog with its point picked.
fn hole_at_point(tag: &str) -> Option<(Headless, std::path::PathBuf, f64)> {
    let (mut h, file) = open(FOR_HOLES, tag)?;
    let was = built(&file, "before");
    button(&mut h, "Hole");
    point_at(&mut h, [10.0, 10.0, 20.0]);
    Some((h, file, was))
}

fn pi_r2(d: f64) -> f64 {
    core::f64::consts::PI * d * d / 4.0
}

#[test]
fn a_clearance_hole_comes_from_its_fastener_and_fit() {
    let Some((mut h, file, was)) = hole_at_point("clear") else {
        return;
    };
    h.click("dialog:holekind:Clearance");
    settle(&mut h);
    let d = h.gui.shell.dialog.as_ref().unwrap();
    assert!(
        d.line().starts_with("hole M6 normal -on sk1 pts p1 thru"),
        "{}",
        d.line()
    );
    ok(&mut h, "clearance");
    // ISO 273 normal for M6 is 6.6
    let v = built(&file, "clearance");
    assert!((was - v - pi_r2(6.6) * 20.0).abs() < 1.0, "{was} {v}");
}

#[test]
fn a_tapped_hole_is_drilled_to_its_threads_tap_size_and_says_its_class() {
    let Some((mut h, file, was)) = hole_at_point("tapped") else {
        return;
    };
    h.click("dialog:holekind:Tapped");
    settle(&mut h);
    let d = h.gui.shell.dialog.as_mut().unwrap();
    d.choose_thread(None, Some("M8"), None, Some("6H"));
    d.settle_hole();
    h.step();
    let line = h.gui.shell.dialog.as_ref().unwrap().line();
    assert!(
        line.contains("thread M8x1.25") && line.contains("class 6H"),
        "{line}"
    );
    let drill = forge_model::standard::screw("M8").unwrap().tap[0];
    assert!(line.starts_with(&format!("hole {drill} ")), "{line}");
    assert!(forge_model::standard::screw("M8").is_some());
    ok(&mut h, "tapped");
    let v = built(&file, "tapped");
    assert!((was - v - pi_r2(drill) * 20.0).abs() < 1.0, "{was} {v}");
    // it reads back as chosen
    let id = h.gui.shell.doc.as_ref().unwrap().items.last().unwrap().id;
    h.gui.shell.out.push(Action::Edit(id));
    assert!(h.until(|g| g.shell.dialog.is_some()));
    let d = h.gui.shell.dialog.as_ref().unwrap();
    assert_eq!(
        d.hole.as_ref().unwrap().kind,
        forge_render::shell::dialog::HoleKind::Tapped
    );
    assert_eq!(
        (d.thread.designation.as_str(), d.thread.class.as_str()),
        ("M8x1.25", "6H")
    );
    assert_eq!(d.line(), line, "the same line back");
}

#[test]
fn a_seat_and_a_depth_and_a_point_are_cut_as_chosen() {
    let Some((mut h, file, was)) = hole_at_point("seat") else {
        return;
    };
    h.click("dialog:seat:Counterbore");
    settle(&mut h);
    // a simple hole has no fastener to take a seat's size from: it waits to be told
    let d = h.gui.shell.dialog.as_ref().unwrap();
    assert!(d.waiting_for_pick() && d.hole.as_ref().unwrap().seat_diameter.is_empty());
    let hole = h.gui.shell.dialog.as_mut().unwrap().hole.as_mut().unwrap();
    hole.seat_diameter = "10".to_string();
    hole.seat_depth = "5".to_string();
    settle(&mut h);
    h.click("dialog:end:Distance");
    settle(&mut h);
    h.click("dialog:point:Angled 118");
    settle(&mut h);
    let line = h.gui.shell.dialog.as_ref().unwrap().line();
    assert!(line.contains("cb 10 5 10 pt 118"), "{line}");
    ok(&mut h, "counterbored");
    let v = built(&file, "counterbored");
    // 5 of the seat, 5 more of the hole to its depth, and the point
    let plain = pi_r2(10.0) * 5.0 + pi_r2(5.0) * 5.0;
    assert!(was - v > plain && was - v < plain + 30.0, "{was} {v} {plain}");
}

#[test]
fn a_fasteners_seat_comes_from_its_row_and_a_countersink_cuts_that_cone() {
    let Some((mut h, file, was)) = hole_at_point("csink") else {
        return;
    };
    h.click("dialog:holekind:Clearance");
    settle(&mut h);
    h.click("dialog:seat:Countersink");
    settle(&mut h);
    let s = forge_model::standard::screw("M6").unwrap();
    let (big, angle) = s.countersink.unwrap();
    let d = s.clearance[1];
    let hole = h.gui.shell.dialog.as_ref().unwrap().hole.clone().unwrap();
    assert_eq!(hole.seat_diameter, big.to_string());
    ok(&mut h, "countersunk");
    let v = built(&file, "countersunk");
    let (sin, cos) = forge_math::dmath::sin_cos(angle / 2.0);
    let deep = (big - d) / 2.0 * cos / sin;
    let cone = core::f64::consts::PI * deep / 3.0 * (big * big + big * d + d * d) / 4.0;
    let want = pi_r2(d) * (20.0 - deep) + cone;
    assert!((was - v - want).abs() < want * 0.01, "{} {want}", was - v);
}

#[test]
fn a_spotface_takes_its_width_from_the_fastener_and_its_depth_from_the_person() {
    let Some((mut h, file, was)) = hole_at_point("spot") else {
        return;
    };
    h.click("dialog:holekind:Clearance");
    settle(&mut h);
    h.click("dialog:seat:Spotface");
    settle(&mut h);
    let s = forge_model::standard::screw("M6").unwrap();
    let wide = s.counterbore[0].0;
    let d = h.gui.shell.dialog.as_ref().unwrap();
    assert_eq!(d.hole.as_ref().unwrap().seat_diameter, wide.to_string());
    assert!(d.waiting_for_pick(), "how deep is theirs to say");
    h.gui
        .shell
        .dialog
        .as_mut()
        .unwrap()
        .hole
        .as_mut()
        .unwrap()
        .seat_depth = "1".to_string();
    settle(&mut h);
    ok(&mut h, "spotface");
    let want = pi_r2(wide) * 1.0 + pi_r2(s.clearance[1]) * 19.0;
    let got = was - built(&file, "spotface");
    assert!((got - want).abs() < want * 0.01, "{got} {want}");
}

#[test]
fn a_hole_to_next_stops_at_the_next_face() {
    let part = "a: extrude 30 rect -30 -30 60 60\np: extrude cut 10 xy at 0 0 10 rect -20 -20 40 40\nsk1: sketch xy at 0 0 30\n  pt 10 10\nend\n";
    let Some((mut h, file)) = open(part, "hnext") else {
        return;
    };
    let was = built(&file, "before");
    button(&mut h, "Hole");
    point_at(&mut h, [10.0, 10.0, 30.0]);
    h.click("dialog:end:To Next");
    settle(&mut h);
    ok(&mut h, "to next");
    let got = was - built(&file, "to next");
    let want = pi_r2(5.0) * 10.0;
    assert!((got - want).abs() < want * 0.01, "{got} {want}");
}

#[test]
fn a_flipped_hole_goes_the_other_way_by_volume() {
    // a sketch 17 up in a block 20 high: 5 down is all in it; 5 up has only 3 to cut
    let part = "a: extrude 20 rect -30 -30 60 60\nsk1: sketch xy at 0 0 17\n  pt 10 10\nend\n";
    let cut = |flip: bool, tag: &str| -> Option<f64> {
        let (mut h, file) = open(part, tag)?;
        let was = built(&file, "before");
        button(&mut h, "Hole");
        point_at(&mut h, [10.0, 10.0, 17.0]);
        h.click("dialog:end:Distance");
        settle(&mut h);
        h.gui.shell.dialog.as_mut().unwrap().hole.as_mut().unwrap().depth = "5".to_string();
        settle(&mut h);
        if flip {
            h.click("dialog:flip:Along the normal");
            settle(&mut h);
        }
        ok(&mut h, "flip");
        Some(was - built(&file, "flip"))
    };
    let Some(down) = cut(false, "hflip0") else { return };
    let Some(up) = cut(true, "hflip1") else { return };
    let area = pi_r2(5.0);
    assert!((down - area * 5.0).abs() < 1.0, "{down}");
    assert!((up - area * 3.0).abs() < 1.0, "{up}");
}

#[test]
fn flipping_a_hole_sends_it_the_other_way_and_there_it_says_so() {
    let Some((mut h, _, _)) = hole_at_point("flip") else {
        return;
    };
    h.click("dialog:flip:Along the normal");
    settle(&mut h);
    let line = h.gui.shell.dialog.as_ref().unwrap().line();
    assert!(line.contains(" -on sk1 ") || line.contains(" on sk1 "), "{line}");
    let was_on = line.contains(" -on sk1 ");
    h.click("dialog:flip:Against the normal");
    settle(&mut h);
    let now = h.gui.shell.dialog.as_ref().unwrap().line();
    assert_ne!(now.contains(" -on sk1 "), was_on, "{line} {now}");
}

#[test]
fn a_marked_edge_is_its_full_colour_on_a_shadowed_inner_corner() {
    // the deep corner where a wall meets a floor: the occlusion is at its darkest here
    let part = "floor: extrude 10 rect -30 -30 60 60\nwall: extrude 40 xy at 0 0 10 rect -30 -30 10 60\n";
    let Some((mut h, _)) = open(part, "edgeglow") else {
        return;
    };
    button(&mut h, "Fillet");
    point_at(&mut h, [-20.0, 15.0, 10.0]);
    assert!(
        !h.gui.view.as_ref().unwrap().nav.selected.is_empty(),
        "the edge is picked"
    );
    settle(&mut h);
    let (cam, size) = h.gui.shell.camera.unwrap();
    let r = h.gui.shell.view_rect;
    let on = cam.pixel_of([-20.0, 15.0, 10.0], size.0, size.1).unwrap();
    let img = h.gui_pixels();
    let (cx, cy) = ((r.min.x + on.0) as usize, (r.min.y + on.1) as usize);
    let mut warmest = 0;
    for dy in 0..5 {
        for dx in 0..5 {
            let p = img.at(cx + dx - 2, cy + dy - 2);
            warmest = warmest.max(p[0] as i32 - p[2] as i32);
        }
    }
    // the chosen amber undimmed: red well over blue, however dark the corner
    assert!(warmest > 120, "{warmest}");
}

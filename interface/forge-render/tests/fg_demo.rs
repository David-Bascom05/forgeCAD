//! The demo part built in the app, as a person builds it with the ribbon: a sketch on the XY
//! plane, an extrude, a boss, a counterbored hole, a fillet, a circular pattern, a mirror and
//! embossed text. The file it saves is examples/demo/part_tools.fg, to the byte.
//! `FORGE_DEMO_SHOTS=DIR` writes a frame of each step there.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::Action;
use forge_render::shell::sketching::SkTool as T;

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn settle(h: &mut Headless) {
    assert!(h.until(ready), "{}", h.gui.shell.status);
    assert!(h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving())));
    for _ in 0..4 {
        h.step();
    }
}

fn shot(h: &mut Headless, name: &str) {
    if let Ok(dir) = std::env::var("FORGE_DEMO_SHOTS") {
        for _ in 0..3 {
            h.step();
        }
        std::fs::write(format!("{dir}/{name}.ppm"), h.ppm()).unwrap();
    }
}

fn point_at(h: &mut Headless, world: [f32; 3]) {
    assert!(h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving())));
    let (cam, size) = h.gui.shell.camera.unwrap();
    let on = cam.pixel_of(world, size.0, size.1).unwrap();
    let r = h.gui.shell.view_rect;
    h.move_to(r.min.x + on.0, r.min.y + on.1);
    // what's under it read for the part as built now, not a count of frames
    assert!(
        h.settle_hover(),
        "the pointer's pick never came: {}",
        h.gui.shell.status
    );
    h.button(true);
    h.step();
    h.button(false);
    for _ in 0..10 {
        h.step();
    }
}

fn click(h: &mut Headless, place: &str) {
    assert!(h.click(place), "no {place}");
    h.step();
    h.step();
}

fn button(h: &mut Headless, tool: &str) {
    click(h, &format!("tool:{tool}"));
    assert!(
        h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some())),
        "{tool}: no dialog: {}",
        h.gui.shell.status
    );
}

fn choose(h: &mut Headless, slot: &str, text: &str) {
    click(h, &format!("fg:tool:slot:{slot}"));
    click(h, &format!("fg:choice:{slot}:{text}"));
}

fn set(h: &mut Headless, param: &str, text: &str) {
    let t = h.gui.shell.fg.as_mut().unwrap().tool.as_mut().unwrap();
    t.params.iter_mut().find(|p| p.name == param).unwrap().text = text.into();
    h.step();
}

fn ok(h: &mut Headless, what: &str) {
    settle(h);
    let t = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap();
    assert!(t.note.is_none(), "{what}: {:?}", t.note);
    click(h, "fg:tool:ok");
    assert!(
        h.gui.shell.fg.as_ref().unwrap().tool.is_none(),
        "{what}: {:?}",
        h.gui
            .shell
            .fg
            .as_ref()
            .unwrap()
            .tool
            .as_ref()
            .and_then(|t| t.error.clone())
    );
    settle(h);
}

/// A sketch on what's under the point, drawn by tool and plane clicks, finished.
fn sketch(h: &mut Headless, on: [f32; 3], draw: &[(T, &[(f64, f64)])], name: &str) {
    click(h, "tool:Start Sketch");
    point_at(h, on);
    assert!(h.gui.shell.sketch.is_some(), "{}", h.gui.shell.status);
    settle(h);
    for (tool, pts) in draw {
        h.gui.shell.out.push(Action::SketchTool(*tool));
        h.step();
        for p in *pts {
            h.gui.shell.sketch.as_mut().unwrap().click(*p, 0.5);
            h.step();
        }
    }
    let m = h.gui.shell.sketch.as_ref().unwrap();
    assert!(m.error.is_none(), "{:?} {:?}", m.error, m.rows);
    shot(h, name);
    h.gui.shell.out.push(Action::FinishSketch);
    h.step();
    assert!(
        h.gui.shell.sketch.is_none(),
        "{:?}",
        h.gui.shell.fg.as_ref().unwrap().refused
    );
    settle(h);
}

#[test]
fn the_demo_part_is_built_with_the_ribbons_tools() {
    let dir = std::env::temp_dir().join(format!("fg-demo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("part_tools.fg");
    std::fs::write(
        &file,
        "# ForgeCAD's 3D Model and Sketch tools, used in the app (tests/fg_demo.rs)\n",
    )
    .unwrap();
    let font = concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/PublicSans-SemiBold.ttf");
    std::fs::copy(font, dir.join("PublicSans-SemiBold.ttf")).unwrap();
    let mut h = match Headless::new((1700.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    settle(&mut h);

    // a centre rectangle on XY, extruded
    sketch(
        &mut h,
        [15.0, -15.0, 0.0],
        &[(T::RectCenter, &[(0.0, 0.0), (40.0, 30.0)])],
        "model-demo-1-sketch",
    );
    button(&mut h, "Extrude");
    choose(&mut h, "Profile", "sk1");
    set(&mut h, "d", "12");
    shot(&mut h, "model-demo-2-extrude");
    ok(&mut h, "plate");

    // a boss and a hole's point on the top, then the boss
    sketch(
        &mut h,
        [-30.0, 20.0, 12.0],
        &[
            (T::Circle, &[(0.0, 0.0), (12.0, 0.0)]),
            (T::Point, &[(20.0, 0.0)]),
        ],
        "model-demo-3-sketch2",
    );
    button(&mut h, "Extrude");
    choose(&mut h, "Profile", "sk2.c1");
    set(&mut h, "d", "8");
    ok(&mut h, "boss");

    // a counterbored hole at the point
    button(&mut h, "Hole");
    click(&mut h, "fg:tool:choice:Seat:Counterbore");
    choose(&mut h, "Points", "sk2.p1");
    set(&mut h, "d", "5");
    set(&mut h, "cbore_d", "9");
    set(&mut h, "cbore_depth", "3");
    shot(&mut h, "model-demo-4-hole");
    ok(&mut h, "hole");

    // the boss's top rounded
    button(&mut h, "Fillet");
    point_at(&mut h, [8.485, -8.485, 20.0]);
    set(&mut h, "r", "2");
    shot(&mut h, "model-demo-5-fillet");
    ok(&mut h, "fillet");

    // the hole round the boss's axis, four times
    button(&mut h, "Circular");
    choose(&mut h, "Features", "hole1");
    choose(&mut h, "Rotation Axis", "extrude2.axis");
    set(&mut h, "n", "4");
    shot(&mut h, "model-demo-6-circular");
    ok(&mut h, "circular pattern");

    // a pocket near one end, mirrored to the other about the plate's mid plane
    sketch(
        &mut h,
        [-30.0, 20.0, 12.0],
        &[(T::SlotCC, &[(32.0, -10.0), (32.0, 10.0), (35.0, 10.0)])],
        "model-demo-7-slot",
    );
    button(&mut h, "Extrude");
    choose(&mut h, "Profile", "sk3");
    click(&mut h, "fg:tool:choice:Output:Cut");
    set(&mut h, "d", "4");
    ok(&mut h, "pocket");
    button(&mut h, "Plane");
    click(&mut h, "fg:tool:choice:Method:On");
    choose(&mut h, "Plane", "yz");
    ok(&mut h, "mid plane");
    button(&mut h, "Mirror");
    choose(&mut h, "Features", "extrude3");
    choose(&mut h, "Mirror Plane", "wplane1");
    shot(&mut h, "model-demo-8-mirror");
    ok(&mut h, "mirror");

    // text placed in a sketch on the top, embossed
    sketch(
        &mut h,
        [-30.0, 20.0, 12.0],
        &[(T::Text, &[(-30.0, -24.0)])],
        "model-demo-9-text",
    );
    let t = h
        .gui
        .shell
        .fg
        .as_mut()
        .unwrap()
        .tool
        .as_mut()
        .expect("Emboss opens");
    t.words.iter_mut().find(|w| w.0 == "text").unwrap().1 = "FORGE".into();
    set(&mut h, "size", "6");
    set(&mut h, "depth", "0.6");
    shot(&mut h, "model-demo-10-emboss");
    ok(&mut h, "emboss");
    h.gui.fg_save();
    // the home view, as the part's done
    if let Some(v) = &mut h.gui.view {
        v.nav.home(&v.scene);
        v.mark_dirty();
    }
    // home reached before the picture
    h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving()));
    shot(&mut h, "model-demo-11-done");

    let src = std::fs::read_to_string(&file).unwrap();
    let opts = forge_lang::Options {
        dir: Some(dir.clone()),
        ..forge_lang::Options::default()
    };
    let run = forge_lang::run(&src, opts, Default::default()).0;
    assert!(run.ok(), "{:?}\n{src}", run.errors);
    assert_eq!(run.measure.as_ref().unwrap().problems, 0, "{src}");
    if let Ok(out) = std::env::var("FORGE_DEMO_OUT") {
        std::fs::write(out, &src).unwrap();
    }
    let want = include_str!("../../../examples/demo/part_tools.fg");
    assert_eq!(
        src, want,
        "the demo the app builds differs from examples/demo/part_tools.fg"
    );
}

/// A tapped hole where hole1 already is: the preview's refusal is said in words and OK is held.
#[test]
fn a_refused_preview_holds_ok_and_says_why_in_words() {
    let dir = std::env::temp_dir().join(format!("fg-demo-refused-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("part_tools.fg");
    let demo = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/demo/part_tools.fg");
    std::fs::copy(demo, &file).unwrap();
    let font = concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/PublicSans-SemiBold.ttf");
    std::fs::copy(font, dir.join("PublicSans-SemiBold.ttf")).unwrap();
    let mut h = match Headless::new((1700.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    settle(&mut h);
    let before = h.gui.fg.as_ref().unwrap().src.clone();
    button(&mut h, "Hole");
    click(&mut h, "fg:tool:choice:Type:Tapped");
    click(&mut h, "fg:choice:Points:sk2.p1");
    assert!(
        h.until(|g| g
            .shell
            .fg
            .as_ref()
            .and_then(|p| p.tool.as_ref())
            .is_some_and(|t| t.note.is_some())),
        "no refusal"
    );
    settle(&mut h);
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
        .clone()
        .unwrap();
    assert!(
        note.contains("not supported yet") && !note.contains('{'),
        "{note}"
    );
    click(&mut h, "fg:tool:ok");
    settle(&mut h);
    assert!(
        h.gui.shell.fg.as_ref().unwrap().tool.is_some(),
        "OK closed a refused dialog"
    );
    assert_eq!(h.gui.fg.as_ref().unwrap().src, before);
}

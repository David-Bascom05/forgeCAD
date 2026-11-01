//! The app with no window: its chrome and its view driven as a pointer would, on a real part.

use forge_agent::tools::{Call, Part};
use forge_render::gui::Headless;
use forge_render::shell::{NavTool, TABS};
use forge_render::viewcube::Hit;

fn app() -> Option<Headless> {
    let mut h = Headless::new((1200.0, 800.0), 1.0).ok()?;
    h.step();
    h.gui.open("bracket");
    h.settle();
    h.step();
    h.step();
    Some(h)
}

fn src(name: &str) -> String {
    forge_render::load::source(name).unwrap()
}

#[test]
fn the_tree_shows_the_parts_real_features_and_a_click_selects_in_its_own_frame() {
    let Some(mut h) = app() else { return };
    let want = forge_doc::Document::parse(&src("bracket"));
    let names: Vec<String> = want
        .features()
        .filter_map(|i| match i.body {
            forge_doc::Body::Feature { .. } | forge_doc::Body::Sketch { .. } => want.name(i.id),
            _ => None,
        })
        .collect();
    let got: Vec<String> = h.gui.shell.rows.iter().map(|r| r.name.clone()).collect();
    assert_eq!(got, names);
    assert!(got.contains(&"boss".to_string()));
    assert_eq!(h.gui.shell.name, "bracket.fc");
    // the tree counts by kind; the file's labels are only in the tooltip's line
    let titles: Vec<&str> = h.gui.shell.rows.iter().map(|r| r.title.as_str()).collect();
    assert_eq!(
        &titles[..5],
        ["Extrude1", "Fillet1", "Fillet2", "Extrude2", "Axis1"]
    );
    assert!(
        h.gui.shell.rows[3].line.starts_with("boss:"),
        "{}",
        h.gui.shell.rows[3].line
    );

    assert_eq!(h.gui.shell.selected, None);
    assert!(h.click("tree:boss"));
    h.step();
    let id = h.gui.shell.rows.iter().find(|r| r.name == "boss").unwrap().id;
    assert_eq!(h.gui.shell.selected, Some(id));
}

#[test]
fn tabs_change_the_ribbon_and_only_what_works_is_enabled() {
    let Some(mut h) = app() else { return };
    assert!(h.gui.shell.place("tool:Extrude").is_some());
    assert!(h.gui.shell.place("tool:Near").is_none());
    h.click(&format!("tab:{}", TABS[3]));
    h.step();
    h.step();
    assert_eq!(h.gui.shell.tab, 3);
    assert!(h.gui.shell.place("tool:Near").is_some());
    // a live button does what it says, and shows it
    h.click("tool:Far");
    h.step();
    h.step();
    assert_eq!(h.gui.view.as_ref().unwrap().renderer.line_mode.name(), "far");
    // a disabled one is hover only
    h.click("tab:3D Model");
    h.step();
    h.step();
    let calls = h.gui.calls.len();
    h.click("tool:Extrude");
    h.step();
    assert_eq!(h.gui.calls.len(), calls);
}

#[test]
fn nothing_is_drawn_when_nothing_moves_and_chrome_hover_leaves_the_view_alone() {
    let Some(mut h) = app() else { return };
    // what the worker's still saying can arrive on any of these: it's the frame with nothing new
    let mut frames = 0;
    let o = loop {
        let o = h.step();
        frames += 1;
        if o.again_in.is_none() && !o.times.view_drawn {
            break o;
        }
        assert!(frames < 200, "it never went idle");
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    assert!(!h.step().times.view_drawn, "and it stays idle");
    assert!(o.again_in.is_none());
    // moving over the ribbon repaints chrome only
    let r = h.gui.shell.place("tool:Extrude").unwrap();
    h.move_to(r.center().x, r.center().y);
    let o = h.step();
    assert!(!o.times.view_drawn);
    // moving over the view reaches the view
    let v = h.gui.shell.view_rect;
    h.move_to(v.center().x, v.center().y);
    h.step();
    h.step();
    h.step();
    let hit = h.gui.view.as_ref().unwrap().nav.marks();
    assert!(!hit.is_empty(), "the part's under the middle of the view");
}

#[test]
fn a_chosen_feature_with_its_arrow_and_field_up_still_goes_idle() {
    let Some(mut h) = app() else { return };
    h.click("tree:boss");
    assert!(h.until(|g| g.shell.dim.is_some()));
    // off anything with a tooltip
    h.move_to(800.0, 790.0);
    let mut frames = 0;
    loop {
        let o = h.step();
        frames += 1;
        if o.again_in.is_none() {
            break;
        }
        assert!(frames < 200, "it never went idle: {:?}", o.again_in);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(!h.step().times.view_drawn);
}

#[test]
fn the_view_cube_answers_through_the_chrome() {
    let Some(mut h) = app() else { return };
    // off the corner it opens on, which a click on that corner wouldn't change
    h.gui.view.as_mut().unwrap().nav.camera.yaw += 0.4;
    h.step();
    let (v, place) = {
        let g = &h.gui;
        (g.shell.view_rect, g.view.as_ref().unwrap().nav.cube_place())
    };
    let mid = (
        v.left() + place.at.0 + place.side * 0.5,
        v.top() + place.at.1 + place.side * 0.5,
    );
    h.move_to(mid.0, mid.1);
    h.step();
    let hover = h.gui.view.as_ref().unwrap().nav.cube_hover;
    assert!(matches!(hover, Some(Hit::Region(_))), "{hover:?}");
    let before = h.gui.view.as_ref().unwrap().nav.camera;
    h.button(true);
    h.step();
    h.button(false);
    for _ in 0..40 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let after = h.gui.view.as_ref().unwrap().nav.camera;
    assert_ne!(
        (before.yaw, before.pitch),
        (after.yaw, after.pitch),
        "it went to the view"
    );
}

#[test]
fn a_nav_tool_changes_what_the_left_button_does() {
    let Some(mut h) = app() else { return };
    h.click("nav:pan");
    h.step();
    h.step();
    assert_eq!(h.gui.shell.view.tool, NavTool::Pan);
    let v = h.gui.shell.view_rect;
    let before = h.gui.view.as_ref().unwrap().nav.camera;
    h.move_to(v.left() + 40.0, v.bottom() - 40.0);
    h.step();
    h.button(true);
    h.step();
    h.move_to(v.left() + 90.0, v.bottom() - 60.0);
    h.step();
    h.button(false);
    h.step();
    let after = h.gui.view.as_ref().unwrap().nav.camera;
    assert_eq!((before.yaw, before.pitch), (after.yaw, after.pitch));
    assert_ne!(before.target, after.target);
}

#[test]
fn undo_is_a_tool_call_and_the_calls_replay_on_a_fresh_part() {
    // the same calls by hand on the part as the worker's, then through the buttons
    let mut p = Part::new("w = 40\nbase: extrude 10 rect 0 0 w 30\n");
    let base = p.model.doc.find("base").unwrap();
    assert!(
        p.call(
            vec![Call::SetArg {
                item: base,
                arg: 0,
                text: "12".into()
            }],
            None
        )
        .ok
    );
    assert!(p.call(vec![Call::Undo], None).ok);
    assert!(p.model.src().contains("extrude 10"));

    let Some(mut h) = app() else { return };
    let was = h.gui.shell.rows.clone();
    h.click("undo");
    h.step();
    h.step();
    assert_eq!(h.gui.calls, vec![Call::Undo]);
    // nothing to undo on a fresh file: it says so, and the document stands
    for _ in 0..200 {
        h.step();
        if !h.gui.shell.status.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(
        h.gui.shell.status.contains("nothing to undo"),
        "{}",
        h.gui.shell.status
    );
    assert_eq!(h.gui.shell.rows, was);
}

#[test]
fn a_part_that_cannot_open_says_why_and_the_app_goes_on() {
    let Ok(mut h) = Headless::new((800.0, 600.0), 1.25) else {
        return;
    };
    h.step();
    h.gui.open("/nonexistent/x.fc");
    for _ in 0..2000 {
        h.step();
        if !h.gui.loading() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(h.gui.shell.status.contains("x.fc"), "{}", h.gui.shell.status);
    assert!(h.gui.view.is_none());
}

#[test]
fn themes_change_every_colour_including_the_views() {
    let Some(mut h) = app() else { return };
    h.click("theme");
    h.step();
    h.step();
    h.step();
    assert!(!h.gui.shell.theme.dark);
    let v = h.gui.view.as_ref().unwrap();
    assert_eq!(v.renderer.back_colour, forge_render::viewcube::rgb(0xDDE2E8));
}

#[test]
fn at_a_fractional_scale_the_view_is_the_layouts_size_in_the_windows_pixels() {
    // the crash on David's 1.5 screen: a scale forced on top of the window's own
    let Ok(mut h) = Headless::new((1600.0, 900.0), 1.5) else {
        return;
    };
    h.step();
    h.gui.open("bracket");
    h.settle();
    h.step();
    let scale = h.gui.ctx.pixels_per_point();
    assert_eq!(scale, 1.5);
    for _ in 0..4 {
        h.step();
        assert_eq!(
            h.gui.ctx.pixels_per_point(),
            1.5,
            "it stays put frame after frame"
        );
    }
    let r = h.gui.shell.view_rect;
    assert!((r.right() - 1600.0).abs() < 1.0, "{r:?}");
    let v = h.gui.view.as_ref().unwrap();
    assert_eq!(v.nav.size.0 as f32, (r.width() * 1.5).round());
    assert!(v.nav.size.0 <= 2400 && v.nav.size.1 <= 1350, "{:?}", v.nav.size);
}

fn title_of<'a>(h: &'a Headless, name: &str) -> &'a forge_render::shell::Row {
    h.gui.shell.rows.iter().find(|r| r.name == name).unwrap()
}

fn source(h: &Headless) -> String {
    h.gui.shell.doc.as_ref().unwrap().print()
}

#[test]
fn choosing_a_feature_shows_its_number_and_a_drag_is_real_millimetres_along_the_arrow() {
    let Some(mut h) = app() else { return };
    assert!(h.click("tree:boss"));
    assert!(h.until(|g| g.shell.dim.is_some()), "its number comes up");
    h.step();
    h.step();
    let boss = title_of(&h, "boss").id;
    let was = h.gui.shell.dim.as_ref().unwrap().clone();
    assert_eq!(was.handle.item, boss);
    assert_eq!(was.value, 30.0);
    assert!(!h.gui.view.as_ref().unwrap().facts.faces_of(boss).is_empty());
    let (grip, cam, size, origin) = {
        let (c, s) = h.gui.shell.camera.unwrap();
        (h.gui.shell.place("dim:arrow").unwrap(), c, s, was.handle.origin)
    };
    // 10 mm along the arrow, as the screen shows it
    let v = h.gui.shell.view_rect;
    let at = |mm: f64| {
        let p = [0, 1, 2].map(|k| (origin[k] + was.handle.dir[k] * mm) as f32);
        let px = cam.pixel_of(p, size.0, size.1).unwrap();
        (v.left() + px.0, v.top() + px.1)
    };
    let (a, b) = (at(0.0), at(10.0));
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let c = grip.center();
    h.move_to(c.x, c.y);
    h.step();
    h.button(true);
    h.step();
    h.move_to(c.x + dx * 0.5, c.y + dy * 0.5);
    h.step();
    h.move_to(c.x + dx, c.y + dy);
    h.step();
    assert!(h.until(|g| g.view.as_ref().unwrap().live_drag));
    let live = h.gui.shell.dim.as_ref().unwrap().value;
    assert!((live - 40.0).abs() < 0.6, "10 mm out gives 40, got {live}");
    h.button(false);
    assert!(
        h.until(|g| !g.view.as_ref().unwrap().live_drag),
        "the full part comes back"
    );
    assert!(
        h.until(|g| g
            .shell
            .doc
            .as_ref()
            .unwrap()
            .print()
            .contains(&format!("boss: extrude {}", forge_agent::short(live)))),
        "status {} calls {:?} dim {:?}",
        h.gui.shell.status,
        h.gui.calls,
        h.gui.shell.dim
    );
    // the same tool call on a fresh part makes the same file
    let calls = h.gui.calls.clone();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let mut p = Part::new(&src("bracket"));
    assert!(p.call(calls, None).ok);
    assert_eq!(p.model.src(), source(&h));
}

#[test]
fn escape_calls_a_drag_off_and_the_shape_goes_back() {
    let Some(mut h) = app() else { return };
    h.click("tree:boss");
    assert!(h.until(|g| g.shell.dim.is_some()));
    h.step();
    h.step();
    let before = source(&h);
    let c = h.gui.shell.place("dim:arrow").unwrap().center();
    h.move_to(c.x, c.y);
    h.step();
    h.button(true);
    h.step();
    h.move_to(c.x + 30.0, c.y - 30.0);
    h.step();
    assert!(h.gui.shell.dim.as_ref().unwrap().dragging);
    h.key(egui::Key::Escape);
    h.step();
    h.button(false);
    assert!(h.until(|g| !g.shell.dim.as_ref().unwrap().dragging));
    h.until(|g| !g.view.as_ref().unwrap().live_drag);
    assert_eq!(source(&h), before);
    assert!(h.gui.calls.is_empty());
    assert_eq!(h.gui.shell.dim.as_ref().unwrap().value, 30.0);
}

#[test]
fn a_typed_value_commits_on_enter_and_one_that_cannot_build_says_so_at_the_dimension() {
    let Some(mut h) = app() else { return };
    h.click("tree:boss");
    assert!(h.until(|g| g.shell.dim.is_some()));
    h.step();
    h.step();
    let before = source(&h);
    let f = h.gui.shell.place("dim:field").unwrap().center();
    h.move_to(f.x, f.y);
    h.step();
    h.button(true);
    h.step();
    h.button(false);
    h.step();
    h.key(egui::Key::End);
    h.step();
    for _ in 0..4 {
        h.key(egui::Key::Backspace);
        h.step();
    }
    h.text("nope");
    h.step();
    h.key(egui::Key::Enter);
    h.step();
    assert!(
        h.until(|g| g.shell.dim.as_ref().is_some_and(|d| d.error.is_some())),
        "an error, at the dimension"
    );
    assert!(h.gui.shell.place("dim:error").is_some() || h.until(|g| g.shell.place("dim:error").is_some()));
    assert_eq!(source(&h), before, "the last good part stands");
    // then a good one
    let f = h.gui.shell.place("dim:field").unwrap().center();
    h.move_to(f.x, f.y);
    h.step();
    h.button(true);
    h.step();
    h.button(false);
    h.step();
    h.key(egui::Key::End);
    h.step();
    for _ in 0..4 {
        h.key(egui::Key::Backspace);
        h.step();
    }
    h.text("35");
    h.step();
    h.key(egui::Key::Enter);
    h.step();
    assert!(
        h.until(|g| g.shell.doc.as_ref().unwrap().print().contains("boss: extrude 35")),
        "{}",
        source(&h)
    );
    assert!(h.until(|g| {
        g.shell
            .dim
            .as_ref()
            .is_some_and(|d| d.error.is_none() && d.handle.value == 35.0)
    }));
}

#[test]
fn clicking_a_face_chooses_the_feature_that_made_it() {
    let Some(mut h) = app() else { return };
    assert!(h.until(|g| !g.view.as_ref().unwrap().facts.face_item.is_empty()));
    let v = h.gui.shell.view_rect;
    h.move_to(v.center().x, v.center().y);
    h.step();
    h.step();
    h.step();
    let hit = h.gui.view.as_ref().unwrap().nav.marks();
    assert!(!hit.is_empty());
    let mid = v.center();
    h.move_to(mid.x, mid.y);
    h.step();
    h.button(true);
    h.step();
    h.button(false);
    assert!(
        h.until(|g| g.shell.selected.is_some()),
        "the face's feature is chosen"
    );
    let id = h.gui.shell.selected.unwrap();
    assert!(!h.gui.view.as_ref().unwrap().facts.faces_of(id).is_empty());
    // and Esc clears it
    assert!(h.click("tree:boss"));
    assert!(h.until(|g| g.shell.selected == Some(title_of_id(g, "boss"))));
}

fn title_of_id(g: &forge_render::gui::Gui, name: &str) -> forge_doc::ItemId {
    g.shell.rows.iter().find(|r| r.name == name).unwrap().id
}

#[test]
fn origin_opens_to_its_items_shows_them_by_the_eye_and_a_click_on_one_selects_it() {
    use forge_render::origin::Item;
    let Some(mut h) = app() else { return };
    assert!(h.gui.shell.place("tree:XY Plane").is_none());
    h.click("tree:Origin");
    h.step();
    h.step();
    for name in [
        "YZ Plane",
        "XZ Plane",
        "XY Plane",
        "X Axis",
        "Y Axis",
        "Z Axis",
        "Center Point",
    ] {
        assert!(h.gui.shell.place(&format!("tree:{name}")).is_some(), "{name}");
    }
    assert!(h.gui.shell.origin_drawn().is_empty(), "hidden by default");
    // hovering one draws it, strongly, and only while hovering
    let r = h.gui.shell.place("tree:XY Plane").unwrap().center();
    h.move_to(r.x, r.y);
    let first = h.step();
    let o = h.step();
    assert_eq!(h.gui.shell.origin_drawn(), vec![(Item::XyPlane, true)]);
    assert!(first.times.view_drawn || o.times.view_drawn, "the view shows it");
    // clicking selects it
    h.click("tree:XY Plane");
    h.step();
    h.step();
    assert_eq!(h.gui.shell.origin_selected, Some(Item::XyPlane));
    assert_eq!(h.gui.shell.selected, None);
    // the eye keeps it shown
    h.click("tree:Z Axis:eye");
    h.step();
    h.step();
    h.move_to(600.0, 400.0);
    h.step();
    h.step();
    assert!(h.gui.shell.origin_drawn().contains(&(Item::ZAxis, false)));
    h.click("tree:Origin:eye");
    h.step();
    h.step();
    assert!(
        h.gui.shell.origin_shown.iter().all(|s| !*s),
        "the folder's eye hides them all"
    );
}

#[test]
fn selecting_shows_no_dialog_a_double_click_does_and_cancel_closes_it() {
    let Some(mut h) = app() else { return };
    h.click("tree:boss");
    h.step();
    h.step();
    assert!(h.gui.shell.dialog.is_none() && h.gui.shell.place("dialog").is_none());
    // the second click of a pair, at once
    let r = h.gui.shell.place("tree:boss").unwrap().center();
    h.move_to(r.x, r.y);
    for _ in 0..2 {
        h.button(true);
        h.step();
        h.button(false);
        h.step();
    }
    h.step();
    h.step();
    assert!(h.gui.shell.dialog.is_some());
    assert!(h.gui.shell.place("dialog").is_some());
    h.key(egui::Key::Escape);
    h.step();
    h.step();
    assert!(h.gui.shell.dialog.is_none());
}

fn step_some(h: &mut Headless) {
    for _ in 0..3 {
        h.step();
    }
}

#[test]
fn tree_operations_are_tool_calls_that_replay_on_a_fresh_part() {
    let dir = std::env::temp_dir().join(format!("fc-tree-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.fc");
    std::fs::write(
        &file,
        "base: extrude 20 rect 0 0 40 40\nh1: hole 5 -xy at 10 10 20 thru\nh2: hole 5 -xy at 30 30 20 thru\n",
    )
    .unwrap();
    let Ok(mut h) = Headless::new((1200.0, 800.0), 1.0) else {
        return;
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    h.settle();
    step_some(&mut h);
    let rows: Vec<String> = h.gui.shell.rows.iter().map(|r| r.title.clone()).collect();
    assert_eq!(rows, ["Extrude1", "Hole1", "Hole2"]);

    // the right-click menu suppresses
    let c = h.gui.shell.place("tree:h1").unwrap().center();
    h.right_drag((c.x, c.y), (c.x, c.y));
    step_some(&mut h);
    assert!(h.gui.shell.row_menu.is_some());
    let id = title_of(&h, "h1").id;
    h.gui.shell.row_menu = None;
    h.gui.shell.out.push(forge_render::shell::Action::Suppress(id));
    assert!(h.until(|g| g.shell.rows.iter().any(|r| r.suppressed)));
    let off = h.gui.shell.rows.iter().find(|r| r.suppressed).unwrap().clone();
    assert_eq!(off.title, "Hole1", "it keeps its place in the tree");
    // and back
    h.gui
        .shell
        .out
        .push(forge_render::shell::Action::Unsuppress(off.id));
    assert!(h.until(|g| g.shell.rows.iter().all(|r| !r.suppressed)));

    // rename by F2 and typing
    let c = h.gui.shell.place("tree:h2").unwrap().center();
    h.move_to(c.x, c.y);
    h.step();
    h.button(true);
    h.step();
    h.button(false);
    step_some(&mut h);
    h.key(egui::Key::F2);
    step_some(&mut h);
    h.text("X");
    step_some(&mut h);
    h.key(egui::Key::Enter);
    assert!(
        h.until(|g| g.shell.rows.iter().any(|r| r.title.contains('X'))),
        "{:?}",
        h.gui.calls
    );
    let renamed = h
        .gui
        .shell
        .rows
        .iter()
        .find(|r| r.title.contains('X'))
        .unwrap()
        .clone();
    assert!(renamed.title.len() > 1);

    // drag a row: h2 above h1
    let (from, to) = (
        h.gui.shell.place("tree:h2").unwrap(),
        h.gui.shell.place("tree:h1").unwrap(),
    );
    h.move_to(from.center().x, from.center().y);
    h.step();
    h.button(true);
    h.step();
    h.move_to(to.center().x, to.top() + 1.0);
    step_some(&mut h);
    h.button(false);
    assert!(h.until(|g| g.shell.rows[1].name == "h2"), "{:?}", h.gui.calls);

    // delete
    let c = h.gui.shell.place("tree:h1").unwrap().center();
    h.move_to(c.x, c.y);
    h.step();
    h.button(true);
    h.step();
    h.button(false);
    step_some(&mut h);
    h.key(egui::Key::Delete);
    assert!(h.until(|g| g.shell.rows.len() == 2));

    // all of it, the same on a fresh part
    let on_disk = std::fs::read_to_string(&file).unwrap();
    let mut p = Part::new(
        "base: extrude 20 rect 0 0 40 40\nh1: hole 5 -xy at 10 10 20 thru\nh2: hole 5 -xy at 30 30 20 thru\n",
    );
    for c in h.gui.calls.clone() {
        let d = p.call(vec![c.clone()], None);
        assert!(d.ok, "{c:?}: {}", d.text);
    }
    assert_eq!(p.model.src(), on_disk);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_end_of_part_marker_rolls_the_part_back_and_rows_below_it_dim() {
    let Some(mut h) = app() else { return };
    assert!(h.until(|g| !g.view.as_ref().unwrap().facts.face_item.is_empty()));
    let full = h.gui.view.as_ref().unwrap().facts.face_item.len();
    let m = h.gui.shell.place("tree:End of part").unwrap().center();
    let to = h.gui.shell.place("tree:boss").unwrap();
    h.move_to(m.x, m.y);
    h.step();
    h.button(true);
    h.step();
    h.move_to(to.center().x, to.top() + 1.0);
    step_some(&mut h);
    h.button(false);
    let corner = title_of(&h, "corner").id;
    assert!(
        h.until(|g| g.shell.end_at == Some(corner)),
        "{:?}",
        h.gui.shell.end_at
    );
    assert!(
        h.until(|g| {
            let n = g.view.as_ref().unwrap().facts.face_item.len();
            n > 0 && n < full
        }),
        "the part as of that feature"
    );
    // and forward again
    h.gui.shell.out.push(forge_render::shell::Action::EndAt(None));
    assert!(h.until(|g| g.view.as_ref().unwrap().facts.face_item.len() == full));
}

#[test]
fn the_right_button_in_the_view_is_the_marking_menu() {
    let Some(mut h) = app() else { return };
    assert!(h.until(|g| !g.view.as_ref().unwrap().facts.face_item.is_empty()));
    let v = h.gui.shell.view_rect;
    let c = (v.left() + 60.0, v.bottom() - 120.0);
    // a click brings it up, Esc takes it away, and nothing panned
    let cam = h.gui.view.as_ref().unwrap().nav.camera;
    h.right_drag(c, c);
    step_some(&mut h);
    assert!(h.gui.shell.marking.is_some());
    assert!(h.gui.shell.place("mark:Fit All").is_some());
    h.key(egui::Key::Escape);
    step_some(&mut h);
    assert!(h.gui.shell.marking.is_none());
    assert_eq!(h.gui.view.as_ref().unwrap().nav.camera, cam);
    // a flick straight up chooses the top place without showing it: Undo
    h.right_drag(c, (c.0, c.1 - 80.0));
    step_some(&mut h);
    assert!(h.gui.shell.marking.is_none());
    assert_eq!(h.gui.calls, vec![Call::Undo]);
}

use forge_render::shell::Action;

fn file_part(text: &str) -> Option<(Headless, std::path::PathBuf)> {
    let dir = std::env::temp_dir().join(format!(
        "fc-dlg-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.fc");
    std::fs::write(&file, text).unwrap();
    let mut h = Headless::new((1200.0, 800.0), 1.0).ok()?;
    h.step();
    h.gui.open(file.to_str().unwrap());
    h.settle();
    h.until(|g| g.view.as_ref().is_some_and(|v| !v.facts.face_item.is_empty()));
    step_some(&mut h);
    Some((h, file))
}

const BLOCK: &str = "base: extrude 20 rect -30 -30 60 60\n";

fn set_field(h: &mut Headless, label: &str, text: &str) {
    let d = h.gui.shell.dialog.as_mut().unwrap();
    let f = d.fields.iter_mut().find(|f| f.0 == label).unwrap();
    f.1 = text.to_string();
}

#[test]
fn ribbon_buttons_make_features_through_a_dialog_with_a_live_result_and_ok_commits() {
    let Some((mut h, file)) = file_part(BLOCK) else {
        return;
    };
    h.gui.shell.out.push(Action::NewFeature("plane"));
    assert!(h.until(|g| g.shell.dialog.is_some()));
    h.gui.shell.dialog = None;
    let mut d = forge_render::shell::dialog::Dialog::new_feature("hole");
    d.pick = None;
    d.fields = vec![
        ("Diameter".into(), "5".into()),
        ("Plane".into(), "-xy at 5 5 20".into()),
        ("Depth".into(), "thru".into()),
    ];
    h.gui.shell.dialog = Some(d);
    let v0 = h.gui.view.as_ref().unwrap().facts.face_item.len();
    h.step();
    assert!(h.until(
        |g| g.shell.dialog.as_ref().unwrap().seq > 0 && g.shell.dialog.as_ref().unwrap().error.is_none()
    ));
    // nothing's in the file yet
    assert!(!std::fs::read_to_string(&file).unwrap().contains("hole"));
    h.gui.shell.out.push(Action::DialogOk);
    assert!(h.until(|g| g.shell.dialog.is_none()), "{:?}", h.gui.shell.dialog);
    assert!(h.until(|g| g.shell.rows.len() == 2));
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("hole 5 -xy at 5 5 20 thru")
    );
    assert!(h.until(|g| g.view.as_ref().unwrap().facts.face_item.len() > v0));
    assert_eq!(h.gui.shell.rows[1].title, "Hole1");
    // the same call on a fresh part
    let mut p = Part::new(BLOCK);
    let want = std::fs::read_to_string(&file).unwrap();
    for c in h.gui.calls.clone() {
        assert!(p.call(vec![c], None).ok);
    }
    assert_eq!(p.model.src(), want);
}

#[test]
fn a_fillet_waits_for_picks_takes_them_from_the_view_and_a_bad_value_says_so() {
    let Some((mut h, file)) = file_part(BLOCK) else {
        return;
    };
    h.click("tool:Fillet");
    assert!(h.until(|g| g.shell.dialog.is_some()));
    step_some(&mut h);
    let d = h.gui.shell.dialog.as_ref().unwrap();
    assert!(d.waiting_for_pick(), "it says what it wants");
    assert_eq!(d.seq, 0, "and builds nothing yet");
    // an edge picked in the view lands in the field, as the part file names it
    let v = h.gui.view.as_mut().unwrap();
    v.nav.selected = vec![(0, forge_render::render::Entity::Edge(0))];
    v.mark_dirty();
    h.step();
    let d = h.gui.shell.dialog.as_ref().unwrap();
    let edges = d.fields.iter().find(|f| f.0 == "Edges").unwrap().1.clone();
    assert!(edges.starts_with("@e"), "{edges}");
    assert!(h.until(|g| g.shell.dialog.as_ref().unwrap().seq > 0));
    // too big: a clear word, and OK waits on it
    set_field(&mut h, "Radius", "500");
    h.step();
    assert!(h.until(|g| g.shell.dialog.as_ref().unwrap().error.is_some()));
    // a size that fits commits
    set_field(&mut h, "Radius", "2");
    h.step();
    assert!(h.until(|g| {
        let d = g.shell.dialog.as_ref().unwrap();
        d.error.is_none() && d.sent.as_deref().is_some_and(|l| l.starts_with("fillet 2"))
    }));
    h.gui.shell.out.push(Action::DialogOk);
    assert!(h.until(|g| g.shell.dialog.is_none()), "{:?}", h.gui.shell.dialog);
    assert!(std::fs::read_to_string(&file).unwrap().contains("fillet 2 @e"));
}

#[test]
fn editing_a_feature_keeps_its_id_and_cancel_puts_all_back() {
    let Some((mut h, file)) = file_part(BLOCK) else {
        return;
    };
    let id = title_of(&h, "base").id;
    h.gui.shell.out.push(Action::Edit(id));
    assert!(h.until(|g| g.shell.dialog.is_some()));
    step_some(&mut h);
    let was = std::fs::read_to_string(&file).unwrap();
    set_field(&mut h, "Distance", "33");
    h.step();
    assert!(h.until(|g| g.shell.dialog.as_ref().unwrap().seq > 0));
    h.gui.shell.out.push(Action::DialogCancel);
    assert!(h.until(|g| g.shell.dialog.is_none()));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), was);
    // again, OK
    h.gui.shell.out.push(Action::Edit(id));
    assert!(h.until(|g| g.shell.dialog.is_some()));
    step_some(&mut h);
    set_field(&mut h, "Distance", "33");
    h.step();
    h.gui.shell.out.push(Action::DialogOk);
    assert!(h.until(|g| g.shell.dialog.is_none()));
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("base: extrude 33")
    );
    assert!(h.until(|g| g.shell.rows.first().is_some_and(|r| r.id == id)));
}

#[test]
fn a_sketch_starts_on_a_plane_solves_as_it_goes_and_finish_commits_one_call_that_extrudes() {
    use forge_render::shell::sketching::SkTool;
    let Some((mut h, file)) = file_part(BLOCK) else {
        return;
    };
    // nothing chosen: it waits for a pick, with the planes up
    h.click("tool:Start Sketch");
    step_some(&mut h);
    assert!(h.gui.shell.sketch_pick);
    h.key(egui::Key::Escape);
    h.gui.shell.sketch_pick = false;
    // on the XY plane, chosen from the tree
    h.gui.shell.origin_selected = Some(forge_render::origin::Item::XyPlane);
    h.gui.shell.out.push(Action::StartSketch);
    assert!(h.until(|g| g.shell.sketch.is_some()), "{}", h.gui.shell.status);
    assert_eq!(h.gui.shell.tab, 1, "the Sketch tab");
    step_some(&mut h);
    assert!(h.gui.shell.place("tool:Line").is_some());
    // the camera comes square on
    assert!(h.until(|g| {
        let (f, _, _) = g.view.as_ref().unwrap().nav.camera.axes();
        f[2] < -0.999
    }));
    // draw with real clicks: rectangle tool, two corners on the screen
    h.gui.shell.out.push(Action::SketchTool(SkTool::Rectangle));
    step_some(&mut h);
    let v = h.gui.shell.view_rect;
    let c = v.center();
    for (x, y) in [(c.x - 60.0, c.y - 40.0), (c.x + 60.0, c.y + 40.0)] {
        h.move_to(x, y);
        step_some(&mut h);
        h.button(true);
        h.step();
        h.button(false);
        step_some(&mut h);
    }
    let m = h.gui.shell.sketch.as_ref().unwrap();
    assert!(
        m.rows.len() == 1 && m.rows[0].starts_with("rect "),
        "{:?}",
        m.rows
    );
    assert!(m.error.is_none(), "{:?}", m.error);
    // the view stayed put while the tool had the button
    h.gui.shell.out.push(Action::FinishSketch);
    assert!(
        h.until(|g| g.shell.rows.iter().any(|r| r.kind == "sketch")),
        "{:?} {}",
        h.gui.calls,
        h.gui.shell.status
    );
    assert!(h.gui.shell.sketch.is_none());
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("sketch xy") && text.contains("rect "), "{text}");
    // a replay makes the same file
    let mut p = Part::new(BLOCK);
    for c in h.gui.calls.clone() {
        assert!(p.call(vec![c], None).ok);
    }
    assert_eq!(p.model.src(), text);
}

#[test]
fn an_inline_profile_shows_as_the_features_sketch_and_editing_writes_it_as_a_line_in_one_commit() {
    use forge_render::shell::SketchChild;
    let text = "base: extrude 20 rect -30 -30 60 60\nf: fillet 2 (edges top along +x)\n";
    let Some((mut h, file)) = file_part(text) else {
        return;
    };
    assert!(matches!(
        h.gui.shell.rows[0].sketch,
        Some(SketchChild::Inline(..))
    ));
    let before = h.gui.view.as_ref().unwrap().facts.face_item.len();
    h.click("tree:base:open");
    step_some(&mut h);
    assert!(h.gui.shell.place("tree:base:sketch").is_some());
    let id = title_of(&h, "base").id;
    h.gui.shell.out.push(Action::EditInline(id));
    assert!(h.until(|g| g.shell.sketch.is_some()), "{}", h.gui.shell.status);
    // looked at and left: it's as it was, and nothing's committed
    h.gui.shell.out.push(Action::FinishSketch);
    assert!(h.until(|g| g.shell.sketch.is_none()));
    step_some(&mut h);
    assert_eq!(std::fs::read_to_string(&file).unwrap().trim(), text.trim());
    assert!(h.gui.calls.is_empty());
    // with a change, it's written as a sketch line
    h.gui.shell.out.push(Action::EditInline(id));
    assert!(h.until(|g| g.shell.sketch.is_some()));
    assert!(h.gui.shell.sketch.as_mut().unwrap().add("pt 1 1".into()));
    h.gui.shell.out.push(Action::FinishSketch);
    assert!(
        h.until(|g| g.shell.rows.iter().any(|r| r.kind == "sketch")),
        "{}",
        h.gui.shell.status
    );
    let now = std::fs::read_to_string(&file).unwrap();
    assert!(now.contains("sk1: id 3 sketch xy"), "{now}");
    assert!(now.contains("base: id 1 extrude 20 sk1"), "{now}");
    // the sketch sits under its feature, and the part's the same, downstream fillet and all
    assert!(h.until(|g| g.shell.rows.iter().any(|r| r.kind == "sketch" && r.tucked)));
    assert!(matches!(title_of(&h, "base").sketch, Some(SketchChild::Line(..))));
    assert!(
        h.until(|g| g.view.as_ref().unwrap().facts.face_item.len() == before),
        "{}",
        h.gui.shell.status
    );
    assert!(h.gui.shell.failed.is_none());
    // one undo takes both lines back
    h.gui.shell.out.push(Action::Undo);
    assert!(h.until(|g| g.shell.rows.iter().all(|r| r.kind != "sketch")));
    // but for the file's note that an id's been used, so it won't be handed out again
    let back = std::fs::read_to_string(&file).unwrap();
    let back: Vec<&str> = back.lines().filter(|l| !l.starts_with("next ")).collect();
    assert_eq!(back.join("\n"), text.trim());
}

#[test]
fn a_new_extrude_reads_a_sketch_and_with_none_starts_one_then_opens_on_it() {
    use forge_render::shell::sketching::SkTool;
    let Some((mut h, _)) = file_part(BLOCK) else {
        return;
    };
    h.gui.shell.origin_selected = Some(forge_render::origin::Item::XyPlane);
    h.click("tool:Extrude");
    assert!(h.until(|g| g.shell.sketch.is_some()), "it starts a sketch");
    h.gui.shell.out.push(Action::SketchTool(SkTool::Rectangle));
    h.step();
    let m = h.gui.shell.sketch.as_mut().unwrap();
    assert!(m.add("rect 40 40 10 10".into()));
    h.gui.shell.out.push(Action::FinishSketch);
    assert!(h.until(|g| g.shell.dialog.is_some()), "then the dialog, on it");
    let d = h.gui.shell.dialog.as_ref().unwrap();
    let profile = d.fields.iter().find(|f| f.0 == "Profile").unwrap().1.clone();
    assert!(profile.starts_with("sketch"), "{profile}");
    h.step();
    assert!(h.until(|g| g.shell.dialog.as_ref().unwrap().seq > 0));
    assert!(h.until(|g| g.shell.dialog.as_ref().unwrap().error.is_none()));
    h.gui.shell.out.push(Action::DialogOk);
    assert!(h.until(|g| g.shell.dialog.is_none()), "{:?}", h.gui.shell.dialog);
    assert!(h.until(|g| g.shell.rows.iter().filter(|r| r.kind == "extrude").count() == 2));
}

#[test]
fn home_and_the_opening_view_are_the_cubes_front_right_top_corner_dead_on() {
    use forge_render::nav::{HOME_CORNER, region_view};
    let Some(mut h) = app() else { return };
    let opened = h.gui.view.as_ref().unwrap().nav.camera;
    let corner = region_view(opened, HOME_CORNER);
    assert_eq!(
        (opened.yaw, opened.pitch, opened.roll),
        (corner.yaw, corner.pitch, corner.roll)
    );
    let v = h.gui.view.as_mut().unwrap();
    v.nav.camera.yaw += 0.7;
    v.nav.camera.pitch = 0.1;
    let scene = &v.scene;
    v.nav.home(scene);
    for _ in 0..80 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let home = h.gui.view.as_ref().unwrap().nav.camera;
    assert_eq!(
        (home.yaw, home.pitch, home.roll),
        (corner.yaw, corner.pitch, corner.roll)
    );
    assert_eq!(home.parallel, opened.parallel);
}

#[test]
fn the_cube_can_name_its_faces_and_set_home_and_they_stay_with_the_file() {
    use forge_render::viewcube::FACE_NAMES;
    let Some((mut h, file)) = file_part(BLOCK) else {
        return;
    };
    let view_file = format!("{}.view", file.display());
    let _ = std::fs::remove_file(&view_file);
    let (right, top) = (FACE_NAMES[5].1, FACE_NAMES[2].1);
    assert_eq!(right, [1, 0, 0]);
    // the +x face becomes FRONT, and z stays up
    h.gui
        .shell
        .out
        .push(Action::SetFaceAs([1, 0, 0], FACE_NAMES[0].1));
    step_some(&mut h);
    let o = h.gui.view.as_ref().unwrap().nav.orient;
    assert_eq!(o.named([1, 0, 0]), [0, -1, 0]);
    assert_eq!(o.world(top), [0, 0, 1]);
    // the part didn't change, and the name's beside the file
    assert_eq!(std::fs::read_to_string(&file).unwrap().trim(), BLOCK.trim());
    assert!(
        std::fs::read_to_string(&view_file)
            .unwrap()
            .starts_with("orient ")
    );
    // Home goes to the named corner
    h.gui.shell.out.push(Action::Home);
    for _ in 0..80 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let cam = h.gui.view.as_ref().unwrap().nav.camera;
    let want = forge_render::nav::region_view_as(cam, o.world(forge_render::nav::HOME_CORNER), &o);
    assert!((cam.yaw - want.yaw).abs() < 1e-4 && (cam.pitch - want.pitch).abs() < 1e-4);
    // a Home of its own
    h.gui.shell.out.push(Action::SetHomeAt([0, 0, 1]));
    step_some(&mut h);
    let home = h.gui.view.as_ref().unwrap().nav.home_view.unwrap();
    // a new app on the same file has them back, and opens there
    let mut again = Headless::new((1200.0, 800.0), 1.0).unwrap();
    again.step();
    again.gui.open(file.to_str().unwrap());
    again.settle();
    again.step();
    let n = &again.gui.view.as_ref().unwrap().nav;
    assert_eq!(n.orient, o);
    let kept = n.home_view.unwrap();
    assert_eq!(
        (kept.yaw, kept.pitch, kept.roll, kept.parallel),
        (home.yaw, home.pitch, home.roll, home.parallel)
    );
    assert_eq!((n.camera.yaw, n.camera.pitch), (home.yaw, home.pitch));
    // reset
    h.gui.shell.out.push(Action::ResetOrientation);
    step_some(&mut h);
    assert_eq!(h.gui.view.as_ref().unwrap().nav.orient, Default::default());
    let _ = std::fs::remove_file(&view_file);
}

#[test]
fn a_press_finds_what_is_under_it_with_no_move_since_a_menu_closed() {
    let Some(mut h) = app() else { return };
    let (v, place) = {
        let g = &h.gui;
        (g.shell.view_rect, g.view.as_ref().unwrap().nav.cube_place())
    };
    let mid = (
        v.left() + place.at.0 + place.side * 0.5,
        v.top() + place.at.1 + place.side * 0.5,
    );
    h.right_drag(mid, mid);
    step_some(&mut h);
    assert!(h.gui.shell.cube_menu.is_some(), "the first right-click");
    h.key(egui::Key::Escape);
    step_some(&mut h);
    assert!(h.gui.shell.cube_menu.is_none());
    // the pointer hasn't moved, and the last it knew the view was under the menu
    h.right_click_here();
    step_some(&mut h);
    assert!(
        h.gui.shell.cube_menu.is_some(),
        "the second, with no move between"
    );
}

#[test]
fn drawing_tabs_through_its_dimensions_from_the_start_and_typing_goes_in_the_active_one() {
    use forge_render::shell::sketching::SkTool;
    let Some((mut h, _)) = file_part(BLOCK) else {
        return;
    };
    h.gui.shell.origin_selected = Some(forge_render::origin::Item::XyPlane);
    h.gui.shell.out.push(Action::StartSketch);
    assert!(h.until(|g| g.shell.sketch.is_some()));
    h.gui.shell.out.push(Action::SketchTool(SkTool::Rectangle));
    step_some(&mut h);
    let v = h.gui.shell.view_rect;
    let c = v.center();
    h.move_to(c.x - 60.0, c.y - 40.0);
    step_some(&mut h);
    h.button(true);
    h.step();
    h.button(false);
    step_some(&mut h);
    h.move_to(c.x + 60.0, c.y + 40.0);
    step_some(&mut h);
    let m = h.gui.shell.sketch.as_ref().unwrap();
    assert_eq!(
        (m.fields.len(), m.active),
        (2, 0),
        "width's active, with no click"
    );
    assert!(h.gui.shell.place("sketch:field:Width").is_some());
    // type straight in; Tab to the height; Shift+Tab back; Enter places it
    h.text("25");
    step_some(&mut h);
    let m = h.gui.shell.sketch.as_ref().unwrap();
    assert!(m.fields[0].locked && m.fields[0].text == "25", "{:?}", m.fields);
    h.key(egui::Key::Tab);
    step_some(&mut h);
    assert_eq!(h.gui.shell.sketch.as_ref().unwrap().active, 1);
    h.text("7");
    step_some(&mut h);
    h.key_with(egui::Key::Tab, egui::Modifiers::SHIFT);
    step_some(&mut h);
    assert_eq!(h.gui.shell.sketch.as_ref().unwrap().active, 0);
    h.key(egui::Key::Enter);
    step_some(&mut h);
    let m = h.gui.shell.sketch.as_ref().unwrap();
    assert!(
        m.rows
            .iter()
            .any(|r| r.starts_with("hdist") && r.ends_with(" 25")),
        "{:?}",
        m.rows
    );
    assert!(
        m.rows.iter().any(|r| r.starts_with("vdist") && r.ends_with(" 7")),
        "{:?}",
        m.rows
    );
    assert!(m.error.is_none(), "{:?}", m.error);
}

#[test]
fn origin_planes_show_in_an_empty_part_only_and_a_tree_plane_starts_a_sketch() {
    use forge_render::origin::Item;
    let mut s = forge_render::shell::Shell::default();
    assert_eq!(s.origin_drawn().len(), 3, "an empty part shows them, to start on");
    s.set_doc(BLOCK, 1, None);
    assert!(
        s.origin_drawn().is_empty(),
        "once there's anything, they're hidden"
    );
    let Some((mut h, _)) = file_part(BLOCK) else {
        return;
    };
    h.gui.shell.out.push(Action::StartSketch);
    step_some(&mut h);
    assert!(h.gui.shell.sketch_pick);
    assert!(h.gui.shell.origin_drawn().is_empty(), "not forced on");
    h.click("tree:Origin");
    step_some(&mut h);
    h.click("tree:XZ Plane");
    assert!(h.until(|g| g.shell.sketch.is_some()), "{}", h.gui.shell.status);
    assert_eq!(
        h.gui.shell.sketch.as_ref().unwrap().head,
        Item::XzPlane.reference()
    );
}

#[test]
fn a_new_part_starts_empty_with_its_planes_and_its_first_sketch_makes_a_solid() {
    use forge_render::origin::Item;
    let Some((mut h, path)) = file_part("") else {
        return;
    };
    assert!(h.gui.shell.rows.is_empty());
    assert_eq!(h.gui.shell.origin_drawn().len(), 3);
    h.gui.shell.out.push(Action::StartSketch);
    step_some(&mut h);
    assert!(h.gui.shell.sketch_pick);
    h.click("tree:Origin");
    step_some(&mut h);
    h.click("tree:XY Plane");
    assert!(h.until(|g| g.shell.sketch.is_some()), "{}", h.gui.shell.status);
    assert_eq!(
        h.gui.shell.sketch.as_ref().unwrap().head,
        Item::XyPlane.reference()
    );
    let m = h.gui.shell.sketch.as_mut().unwrap();
    for r in [
        "line 0 0 40 0",
        "line l1.b 40 30",
        "line l2.b 0 30",
        "line l3.b l1.a",
    ] {
        assert!(m.add(r.to_string()), "{r}: {:?}", m.error);
    }
    step_some(&mut h);
    h.gui.shell.out.push(Action::FinishSketch);
    assert!(h.until(|g| !g.shell.rows.is_empty()), "{}", h.gui.shell.status);
    // off the tree row it was chosen from
    let c = h.gui.shell.view_rect.center();
    h.move_to(c.x, c.y);
    step_some(&mut h);
    assert!(
        h.gui.shell.origin_drawn().is_empty(),
        "the planes go with the first feature"
    );
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("sketch"), "{written}");
}

const TWO_SKETCHES: &str =
    "sk1: sketch xy\n  rect 0 0 40 30\nend\nsk2: sketch xz\n  circle 0 0 15\nend\nbase: extrude 10 sk1\n";

#[test]
fn a_sketch_nothing_reads_shows_and_one_that_is_read_hides_till_its_eye_says() {
    let Some((mut h, _)) = file_part(TWO_SKETCHES) else {
        return;
    };
    let id = |h: &Headless, n: &str| h.gui.shell.doc.as_ref().unwrap().find(n).unwrap();
    let (one, two) = (id(&h, "sk1"), id(&h, "sk2"));
    assert_eq!(h.gui.shell.sketch_shapes.len(), 2);
    assert!(!h.gui.shell.sketch_shown(one), "read by the extrude");
    assert!(h.gui.shell.sketch_shown(two), "nothing reads it");
    h.gui.shell.out.push(Action::SketchVisibility(one));
    h.gui.shell.out.push(Action::SketchVisibility(two));
    step_some(&mut h);
    assert!(h.gui.shell.sketch_shown(one));
    assert!(!h.gui.shell.sketch_shown(two));
}

#[test]
fn pointing_at_a_sketch_in_the_view_chooses_it_and_a_face_under_it_does_not_light() {
    let Some((mut h, _)) =
        file_part("sk2: sketch xy\n  circle 60 60 15\nend\nbase: extrude 10 rect 0 0 120 30\n")
    else {
        return;
    };
    let (cam, size) = h.gui.shell.camera.unwrap();
    let rect = h.gui.shell.view_rect;
    let on = cam.pixel_of([75.0, 60.0, 0.0], size.0, size.1).unwrap();
    h.move_to(rect.min.x + on.0, rect.min.y + on.1);
    step_some(&mut h);
    let id = h.gui.shell.doc.as_ref().unwrap().find("sk2").unwrap();
    assert_eq!(h.gui.shell.sketch_hover, Some(id));
    h.button(true);
    h.step();
    h.button(false);
    step_some(&mut h);
    assert_eq!(h.gui.shell.selected, Some(id));
}

#[test]
fn extrude_picks_a_region_of_a_visible_sketch_in_the_view() {
    let two = "sk1: sketch xy\n  rect 0 0 20 20\n  rect 40 0 20 20\nend\n";
    let Some((mut h, _)) = file_part(two) else {
        return;
    };
    h.gui.shell.out.push(Action::NewFeature("extrude"));
    assert!(h.until(|g| g.shell.dialog.is_some()));
    let (cam, size) = h.gui.shell.camera.unwrap();
    let rect = h.gui.shell.view_rect;
    let on = cam.pixel_of([50.0, 10.0, 0.0], size.0, size.1).unwrap();
    h.move_to(rect.min.x + on.0, rect.min.y + on.1);
    step_some(&mut h);
    h.button(true);
    h.step();
    h.button(false);
    step_some(&mut h);
    let d = h.gui.shell.dialog.as_ref().unwrap();
    let profile = d.fields.iter().find(|f| f.0 == "Profile").unwrap();
    assert!(profile.1.starts_with("sk1 l"), "{profile:?}");
    assert_ne!(profile.1, "sk1 l1", "the second, not the first");
    assert!(d.error.is_none(), "{:?}", d.error);
}

#[test]
fn in_a_sketch_faces_do_not_answer_the_pointer_and_orbiting_stays_put_until_look_at() {
    let Some((mut h, _)) = file_part(BLOCK) else {
        return;
    };
    h.gui.shell.origin_selected = Some(forge_render::origin::Item::XyPlane);
    h.gui.shell.out.push(Action::StartSketch);
    assert!(h.until(|g| g.shell.sketch.is_some()));
    h.gui.shell.tab = 1;
    for _ in 0..80 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let c = h.gui.shell.view_rect.center();
    h.move_to(c.x, c.y);
    for _ in 0..4 {
        h.step();
    }
    assert!(
        h.gui
            .view
            .as_ref()
            .unwrap()
            .nav
            .marks()
            .iter()
            .all(|m| !m.selected)
    );
    let was = h.gui.view.as_ref().unwrap().nav.camera;
    h.gui.view.as_mut().unwrap().nav.camera.yaw += 0.7;
    for _ in 0..30 {
        h.step();
    }
    let turned = h.gui.view.as_ref().unwrap().nav.camera;
    assert!((turned.yaw - was.yaw - 0.7).abs() < 1e-4, "no snap back");
    h.gui.shell.out.push(Action::LookAt);
    for _ in 0..200 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let squared = h.gui.view.as_ref().unwrap().nav.camera;
    assert!(
        (squared.yaw - was.yaw).abs() < 1e-3,
        "{} {}",
        squared.yaw,
        was.yaw
    );
}

fn esc(h: &mut Headless) {
    h.key(egui::Key::Escape);
    step_some(h);
}

fn at_rest(h: &Headless) {
    let s = &h.gui.shell;
    assert!(
        !s.sketch_pick && s.sketch.is_none() && s.dialog.is_none() && !s.menu_open() && s.renaming.is_none(),
        "pick {} sketch {} dialog {} menu {}",
        s.sketch_pick,
        s.sketch.is_some(),
        s.dialog.is_some(),
        s.menu_open()
    );
}

#[test]
fn esc_backs_out_of_every_mode_one_level_at_a_time() {
    use forge_render::shell::sketching::SkTool;
    let Some((mut h, _)) = file_part(TWO_SKETCHES) else {
        return;
    };
    let base = h.gui.shell.doc.as_ref().unwrap().find("base").unwrap();
    // picking a plane
    h.gui.shell.out.push(Action::StartSketch);
    step_some(&mut h);
    assert!(h.gui.shell.sketch_pick);
    assert!(h.gui.shell.place("mode:cancel").is_some(), "a visible way out");
    esc(&mut h);
    at_rest(&h);
    // in a sketch with a half-drawn line: the line, the tool, then the sketch
    h.gui.shell.origin_selected = Some(forge_render::origin::Item::XyPlane);
    h.gui.shell.out.push(Action::StartSketch);
    assert!(h.until(|g| g.shell.sketch.is_some()));
    h.gui.shell.out.push(Action::SketchTool(SkTool::Line));
    step_some(&mut h);
    h.gui.shell.sketch.as_mut().unwrap().click((1.0, 1.0), 0.5);
    assert!(h.gui.shell.place("mode:finish").is_some());
    assert!(h.gui.shell.place("mode:cancel").is_some());
    esc(&mut h);
    assert!(
        h.gui.shell.sketch.as_ref().unwrap().tool.is_some(),
        "only what was half drawn"
    );
    esc(&mut h);
    assert!(
        h.gui.shell.sketch.as_ref().unwrap().tool.is_none(),
        "then the tool"
    );
    esc(&mut h);
    at_rest(&h);
    // a dialog; each menu; a rename; a selection
    h.gui.shell.out.push(Action::NewFeature("fillet"));
    step_some(&mut h);
    assert!(h.gui.shell.dialog.is_some());
    esc(&mut h);
    at_rest(&h);
    h.gui.shell.row_menu = Some((base, egui::pos2(400.0, 400.0)));
    step_some(&mut h);
    esc(&mut h);
    at_rest(&h);
    h.gui.shell.cube_menu = Some(egui::pos2(400.0, 400.0));
    step_some(&mut h);
    esc(&mut h);
    at_rest(&h);
    h.gui.shell.marking = Some(egui::pos2(400.0, 400.0));
    step_some(&mut h);
    esc(&mut h);
    at_rest(&h);
    h.gui.shell.file_menu = true;
    step_some(&mut h);
    esc(&mut h);
    at_rest(&h);
    h.gui.shell.out.push(Action::StartRename(base));
    step_some(&mut h);
    assert!(h.gui.shell.renaming.is_some());
    esc(&mut h);
    at_rest(&h);
    h.gui.shell.selected = Some(base);
    step_some(&mut h);
    esc(&mut h);
    assert_eq!(h.gui.shell.selected, None);
}

#[test]
fn picking_a_sketch_while_starting_one_opens_it_and_cancel_always_leaves() {
    let Some((mut h, _)) = file_part(TWO_SKETCHES) else {
        return;
    };
    h.gui.shell.out.push(Action::StartSketch);
    step_some(&mut h);
    assert!(h.gui.shell.sketch_pick);
    // a feature isn't something to sketch on: still picking, and it says so
    h.click("tree:base");
    step_some(&mut h);
    assert!(h.gui.shell.sketch_pick);
    assert!(h.gui.shell.status.contains("feature"), "{}", h.gui.shell.status);
    h.click("tree:sk2");
    assert!(h.until(|g| g.shell.sketch.is_some()), "{}", h.gui.shell.status);
    assert!(!h.gui.shell.sketch_pick);
    assert!(!h.gui.shell.sketch.as_ref().unwrap().is_new());
    h.click("mode:cancel");
    step_some(&mut h);
    at_rest(&h);
    // and a new sketch with nothing drawn leaves nothing behind
    let n = h.gui.shell.rows.len();
    h.gui.shell.origin_selected = Some(forge_render::origin::Item::XyPlane);
    h.gui.shell.out.push(Action::StartSketch);
    assert!(h.until(|g| g.shell.sketch.is_some()));
    h.click("mode:finish");
    step_some(&mut h);
    at_rest(&h);
    assert_eq!(h.gui.shell.rows.len(), n);
}

/// A chosen face's fill in the middle of the view, turned a little at a time: every pixel of a patch
/// of it carries the fill, on a flat face and a curved one, whatever level of detail is showing.
#[test]
fn a_faces_fill_has_no_holes_as_the_view_turns() {
    for (part, flat) in [(BLOCK, true), ("base: extrude 40 circle 0 0 30\n", false)] {
        let Some((mut h, _)) = file_part(part) else {
            return;
        };
        let id = h.gui.shell.doc.as_ref().unwrap().find("base").unwrap();
        h.gui.shell.selected = Some(id);
        for k in 0..24 {
            let v = h.gui.view.as_mut().unwrap();
            v.nav.camera.yaw = 0.3 + k as f32 * 0.17;
            v.nav.camera.pitch = 0.5;
            v.nav.camera.dist *= if k % 2 == 0 { 0.93 } else { 1.0 / 0.93 };
            v.mark_dirty();
            step_some(&mut h);
            let img = h.gui_pixels();
            let (w, c) = (h.gui.shell.camera.unwrap().1, h.gui.shell.view_rect);
            let _ = w;
            let (mid_x, mid_y) = (c.center().x as usize, c.center().y as usize);
            let mut plain = 0;
            for dy in 0..30 {
                for dx in 0..30 {
                    let p = img.at(mid_x - 15 + dx, mid_y - 15 + dy);
                    // the chosen fill is warm: red well over blue, where bare metal is grey
                    if (p[0] as i32) - (p[2] as i32) < 18 {
                        plain += 1;
                    }
                }
            }
            assert_eq!(
                plain, 0,
                "flat {flat}, turn {k}: {plain} of 900 pixels without the fill"
            );
        }
    }
}

#[test]
fn a_hover_mark_stays_on_every_frame_as_the_pointer_crosses_a_face() {
    let Some((mut h, _)) = file_part(BLOCK) else {
        return;
    };
    let c = h.gui.shell.view_rect.center();
    h.move_to(c.x, c.y);
    assert!(h.until(|g| g.view.as_ref().unwrap().nav.marks().iter().any(|m| !m.selected)));
    for k in 0..40 {
        h.move_to(c.x + (k % 10) as f32, c.y + (k % 7) as f32);
        h.step();
        assert!(
            h.gui
                .view
                .as_ref()
                .unwrap()
                .nav
                .marks()
                .iter()
                .any(|m| !m.selected),
            "dark at step {k}"
        );
    }
}

#[test]
fn a_constraint_tool_lights_what_it_would_pick_and_a_mark_lights_what_it_ties() {
    use forge_render::shell::sketching::SkTool;
    let part = "sk1: sketch xy\n  line 0 0 40 0\n  line 0 20 40 20\n  par l1 l2\nend\n";
    let Some((mut h, _)) = file_part(part) else {
        return;
    };
    let id = h.gui.shell.doc.as_ref().unwrap().find("sk1").unwrap();
    h.gui.shell.out.push(Action::Edit(id));
    assert!(h.until(|g| g.shell.sketch.is_some()));
    for _ in 0..80 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    h.gui.shell.out.push(Action::SketchTool(SkTool::Parallel));
    step_some(&mut h);
    let (cam, size) = h.gui.shell.camera.unwrap();
    let r = h.gui.shell.view_rect;
    let on = cam.pixel_of([20.0, 20.0, 0.0], size.0, size.1).unwrap();
    h.move_to(r.min.x + on.0, r.min.y + on.1);
    step_some(&mut h);
    assert_eq!(
        h.gui.shell.sketch.as_ref().unwrap().would_pick.as_deref(),
        Some("l2")
    );
    let g = h
        .gui
        .shell
        .place("sketch:glyph:par l1 l2")
        .expect("the constraint's mark");
    h.move_to(g.center().x, g.center().y);
    step_some(&mut h);
    assert_eq!(
        h.gui.shell.sketch.as_ref().unwrap().glyph_over.as_deref(),
        Some("par l1 l2")
    );
}

/// Words on the cube's faces toward the eye read left to right and upward.
fn cube_words_upright(h: &Headless) {
    use forge_render::viewcube;
    let v = h.gui.view.as_ref().unwrap();
    let place = v.nav.cube_place();
    for l in viewcube::labels(&v.nav.camera, &place, None, &v.nav.orient) {
        if l.alpha < 0.5 {
            continue;
        }
        let (a, b) = (
            viewcube::on_face(&v.nav.camera, &place, &l, 0.0, 0.0),
            viewcube::on_face(&v.nav.camera, &place, &l, 1.0, 0.0),
        );
        let c = viewcube::on_face(&v.nav.camera, &place, &l, 0.0, 1.0);
        assert!(b[0] > a[0], "{} reads backward", l.text);
        assert!(c[1] < a[1], "{} is upside down", l.text);
    }
}

#[test]
fn a_sketch_opens_with_the_cubes_words_upright_on_every_plane_and_a_face() {
    use forge_render::origin::Item;
    for plane in [
        Some(Item::XyPlane),
        Some(Item::XzPlane),
        Some(Item::YzPlane),
        None,
    ] {
        let Some((mut h, _)) = file_part(BLOCK) else {
            return;
        };
        match plane {
            Some(p) => {
                h.gui.shell.origin_selected = Some(p);
                h.gui.shell.out.push(Action::StartSketch);
            }
            None => {
                h.gui.shell.out.push(Action::StartSketch);
                step_some(&mut h);
                let c = h.gui.shell.view_rect.center();
                let (cam, size) = h.gui.shell.camera.unwrap();
                let on = cam.pixel_of([30.0, 0.0, 10.0], size.0, size.1).unwrap();
                let r = h.gui.shell.view_rect;
                let _ = c;
                h.move_to(r.min.x + on.0, r.min.y + on.1);
                for _ in 0..30 {
                    h.step();
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                h.button(true);
                h.step();
                h.button(false);
            }
        }
        assert!(h.until(|g| g.shell.sketch.is_some()), "{plane:?}");
        for _ in 0..80 {
            h.step();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        cube_words_upright(&h);
    }
}

fn eye_shows(h: &Headless, id: forge_doc::ItemId) -> bool {
    let k = h
        .gui
        .shell
        .sketch_shapes
        .iter()
        .position(|s| s.item == Some(id))
        .unwrap();
    h.gui.shell.sketches_drawn().iter().any(|d| d.0 == k)
}

#[test]
fn a_sketch_read_by_features_shows_and_hides_by_its_eye_its_child_eye_and_its_menu() {
    let Some(mut h) = app() else { return };
    assert!(h.until(|g| !g.shell.sketch_shapes.is_empty()));
    step_some(&mut h);
    let row = h
        .gui
        .shell
        .rows
        .iter()
        .find(|r| r.kind == "sketch")
        .unwrap()
        .clone();
    assert!(
        !h.gui.shell.sketch_shown(row.id),
        "bracket's sketch is read: hidden"
    );
    // its own eye
    h.move_to(0.0, 0.0);
    step_some(&mut h);
    assert!(!eye_shows(&h, row.id));
    h.click(&format!("tree:{}:eye", row.name));
    step_some(&mut h);
    assert!(eye_shows(&h, row.id), "the eye shows it");
    assert!(h.gui.shell.sketch_shown(row.id));
    h.click(&format!("tree:{}:eye", row.name));
    step_some(&mut h);
    h.move_to(0.0, 0.0);
    step_some(&mut h);
    assert!(!eye_shows(&h, row.id), "and hides it");
    // the menu's Visibility
    let r = h.gui.shell.place(&format!("tree:{}", row.name)).unwrap();
    h.right_drag((r.center().x, r.center().y), (r.center().x, r.center().y));
    step_some(&mut h);
    let at = h.gui.shell.row_menu.expect("its menu").1;
    h.click_at(at.x + 40.0, at.y + 4.0 + 28.0 * 1.5);
    step_some(&mut h);
    assert!(h.gui.shell.sketch_shown(row.id), "from the menu");
    assert!(h.gui.shell.row_menu.is_none());
}

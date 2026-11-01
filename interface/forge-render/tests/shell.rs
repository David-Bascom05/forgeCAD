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
    let mut frames = 0;
    let o = loop {
        let o = h.step();
        frames += 1;
        if o.again_in.is_none() {
            break o;
        }
        assert!(frames < 30, "it never went idle");
    };
    assert!(!o.times.view_drawn);
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
    let id = title_of(&h, "boss").id;
    assert_eq!(h.gui.shell.dialog, Some(id));
    assert!(h.gui.shell.place("dialog").is_some());
    h.key(egui::Key::Escape);
    h.step();
    h.step();
    assert_eq!(h.gui.shell.dialog, None);
}

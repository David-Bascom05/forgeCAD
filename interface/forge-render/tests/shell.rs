//! The app with no window: its chrome and its view driven as a pointer would, on real Forge
//! parts: a bench part by its name, and parts written for a test.

use forge_render::gui::Headless;
use forge_render::shell::{Action, NavTool, TABS};
use forge_render::viewcube::Hit;

/// The Forge part's build and its whole mesh are in.
fn built(g: &forge_render::gui::Gui) -> bool {
    g.fg.as_ref()
        .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
        && g.view.is_some()
}

fn app() -> Option<Headless> {
    let mut h = Headless::new((1200.0, 800.0), 1.0).ok()?;
    h.step();
    h.gui.open("bracket");
    assert!(h.until(built), "{}", h.gui.shell.status);
    h.step();
    h.step();
    Some(h)
}

fn step_some(h: &mut Headless) {
    for _ in 0..4 {
        h.step();
    }
}

fn file_part(text: &str) -> Option<(Headless, std::path::PathBuf)> {
    let dir = std::env::temp_dir().join(format!(
        "fg-shell-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.fg");
    std::fs::write(&file, text).unwrap();
    let mut h = Headless::new((1200.0, 800.0), 1.0).ok()?;
    h.step();
    h.gui.open(file.to_str().unwrap());
    assert!(h.until(built), "{}", h.gui.shell.status);
    // its faces under the pointer: the anchors that name them are in
    h.until(|g| g.fg_anchors.is_some());
    step_some(&mut h);
    Some((h, file))
}

/// A block 60 by 60 about the origin, 20 high.
const BLOCK: &str = "sketch plan(on: xy) {
  outline = rect(corner: (-30, -30), size: (60, 60))
}
base = extrude(plan, 20)
";

/// A new sketch on one of the origin's planes, squared on to it, as a click on the plane starts it.
fn sketch_on(h: &mut Headless, plane: &str) {
    use forge_render::shell::sketching::{SketchFor, SketchMode};
    let Some(forge_lang::eval::Value::Plane(p)) = forge_lang::builtins::constant(plane) else {
        panic!("{plane}")
    };
    let mut m = SketchMode::new(SketchFor::New, plane.into(), p.frame, Vec::new());
    m.units = "mm";
    h.gui.shell.sketch = Some(m);
    h.gui.shell.tab = 1;
    h.step();
    h.gui.shell.out.push(Action::LookAt);
    step_some(h);
}

#[test]
fn tabs_change_the_ribbon_and_only_what_works_is_enabled() {
    let Some(mut h) = app() else { return };
    assert!(h.gui.shell.place("tool:Extrude").is_some());
    assert!(h.gui.shell.place("tool:Visual Style").is_none());
    h.click(&format!("tab:{}", TABS[3]));
    h.step();
    h.step();
    assert_eq!(h.gui.shell.tab, 3);
    assert!(h.gui.shell.place("tool:Visual Style").is_some());
    // a live button does what it says, and shows it: the style list, then a style from it
    h.click("tool:Visual Style");
    h.step();
    assert!(h.click("style:Wireframe with Hidden Edges"));
    h.step();
    h.step();
    let style = h.gui.view.as_ref().unwrap().renderer.style;
    assert_eq!(style, forge_render::render::VisualStyle::WireframeHidden);
    assert!(
        h.gui.shell.place("style:Wireframe").is_none(),
        "the list closes on a choice"
    );
    // a live one on 3D Model opens its Forge tool
    h.click("tab:3D Model");
    h.step();
    h.step();
    h.click("tool:Fillet");
    step_some(&mut h);
    let tool = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().map(|t| t.kind);
    assert_eq!(tool, Some("fillet"));
    // Extrude with every sketch read already starts a sketch for its profile, as `.fc`'s did
    h.gui.shell.fg.as_mut().unwrap().tool = None;
    h.click("tool:Extrude");
    step_some(&mut h);
    assert!(h.gui.shell.sketch_pick, "{}", h.gui.shell.status);
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
fn a_part_that_cannot_open_says_why_and_the_app_goes_on() {
    let Ok(mut h) = Headless::new((800.0, 600.0), 1.25) else {
        return;
    };
    h.step();
    h.gui.open("/nonexistent/x.fg");
    h.step();
    assert!(h.gui.shell.status.contains("x.fg"), "{}", h.gui.shell.status);
    assert!(h.gui.view.is_none());
    assert!(h.gui.documents().is_empty());
    // what's no Forge part, DWG file or bench part says so too
    h.gui.open("/nonexistent/x.step");
    h.step();
    assert!(h.gui.shell.status.contains("x.step"), "{}", h.gui.shell.status);
}

#[test]
fn themes_change_every_colour_including_the_views() {
    let Some(mut h) = app() else { return };
    assert!(
        h.gui.shell.place("theme").is_none(),
        "the title bar has no theme button"
    );
    h.gui.shell.tab = 4;
    h.step();
    assert!(h.click("tool:Settings"));
    h.step();
    for _ in 0..3 {
        h.step();
    }
    assert!(h.click("settings:theme:Light"));
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

#[test]
fn the_right_button_in_the_view_is_the_marking_menu() {
    let Some(mut h) = app() else { return };
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
    // a flick straight up chooses the top place without showing it: Undo, an edit taken back
    let src = h.gui.fg.as_ref().unwrap().src.clone();
    let p = h
        .gui
        .shell
        .fg
        .as_ref()
        .unwrap()
        .params
        .iter()
        .find(|p| p.driving())
        .unwrap()
        .clone();
    let (v, dim) = p.value.unwrap();
    let op = forge_lang::error::Op::SetParam {
        param: p.key.clone(),
        value: forge_lang::error::Literal { value: v + 1.0, dim },
    };
    h.gui.fg_apply(op).unwrap();
    assert_ne!(h.gui.fg.as_ref().unwrap().src, src);
    h.right_drag(c, (c.0, c.1 - 80.0));
    step_some(&mut h);
    assert!(h.gui.shell.marking.is_none());
    assert_eq!(h.gui.fg.as_ref().unwrap().src, src);
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
    sketch_on(&mut h, "xy");
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
fn in_a_sketch_faces_do_not_answer_the_pointer_and_orbiting_stays_put_until_look_at() {
    let Some((mut h, _)) = file_part(BLOCK) else {
        return;
    };
    sketch_on(&mut h, "xy");
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
    let p = s.fg.as_ref().unwrap();
    assert!(
        !s.sketch_pick && s.sketch.is_none() && p.tool.is_none() && !s.menu_open() && p.renaming.is_none(),
        "pick {} sketch {} dialog {} menu {}",
        s.sketch_pick,
        s.sketch.is_some(),
        p.tool.is_some(),
        s.menu_open()
    );
}

#[test]
fn esc_backs_out_of_every_mode_one_level_at_a_time() {
    use forge_render::shell::sketching::SkTool;
    let Some((mut h, _)) = file_part(BLOCK) else {
        return;
    };
    // picking a plane
    h.gui.shell.out.push(Action::StartSketch);
    step_some(&mut h);
    assert!(h.gui.shell.sketch_pick);
    assert!(h.gui.shell.place("mode:cancel").is_some(), "a visible way out");
    esc(&mut h);
    at_rest(&h);
    // in a sketch with a half-drawn line: the line, the tool, then the sketch
    sketch_on(&mut h, "xy");
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
    assert!(h.gui.shell.fg.as_ref().unwrap().tool.is_some());
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
    h.gui.shell.fg.as_mut().unwrap().renaming = Some(("base".into(), "base".into()));
    step_some(&mut h);
    esc(&mut h);
    at_rest(&h);
    h.gui.shell.origin_selected = Some(forge_render::origin::Item::XyPlane);
    step_some(&mut h);
    esc(&mut h);
    assert_eq!(h.gui.shell.origin_selected, None);
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
    let Some((mut h, _)) = file_part(BLOCK) else {
        return;
    };
    sketch_on(&mut h, "xy");
    let m = h.gui.shell.sketch.as_mut().unwrap();
    for r in ["line 0 0 40 0", "line 0 20 40 20", "par l1 l2"] {
        assert!(m.add(r.into()), "{r}: {:?}", m.error);
    }
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
    for plane in [Some("xy"), Some("xz"), Some("yz"), None] {
        let Some((mut h, _)) = file_part(BLOCK) else {
            return;
        };
        match plane {
            Some(p) => sketch_on(&mut h, p),
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

#[test]
fn a_left_drag_is_a_selection_window_and_f4_or_shift_middle_orbits() {
    let Some((mut h, _)) = file_part(BLOCK) else {
        return;
    };
    let v = h.gui.shell.view_rect;
    let (cam, size) = h.gui.shell.camera.unwrap();
    let was = cam;
    assert_eq!(h.gui.shell.view.tool, NavTool::Select);
    let px = |w: [f32; 3]| {
        let u = cam.pixel_of(w, size.0, size.1).unwrap();
        (v.min.x + u.0, v.min.y + u.1)
    };
    let (a, b) = (px([30.0, -30.0, 20.0]), px([-30.0, 30.0, 20.0]));
    let top = (px([-30.0, -30.0, 20.0]), px([30.0, 30.0, 20.0]));
    // left to right round the top face only: what's wholly in it, solid
    let lo = (top.0.0 - 6.0, a.1.min(b.1).min(top.0.1).min(top.1.1) - 6.0);
    let hi = (top.1.0 + 6.0, a.1.max(b.1).max(top.0.1).max(top.1.1) + 6.0);
    h.move_to(lo.0, lo.1);
    h.step();
    h.button(true);
    h.step();
    h.move_to(hi.0, hi.1);
    step_some(&mut h);
    assert!(h.gui.shell.window_box.is_some());
    h.button(false);
    step_some(&mut h);
    let camera_now = h.gui.view.as_ref().unwrap().nav.camera;
    assert_eq!(camera_now, was, "it doesn't orbit");
    let wholly = h.gui.view.as_ref().unwrap().nav.selected.len();
    assert!(wholly >= 1, "the top face is in it");
    // right to left over the same box also takes what it only crosses: the sides
    h.move_to(hi.0, hi.1);
    h.step();
    h.button(true);
    h.step();
    h.move_to(lo.0, lo.1);
    step_some(&mut h);
    h.button(false);
    step_some(&mut h);
    let crossing = h.gui.view.as_ref().unwrap().nav.selected.len();
    assert!(crossing > wholly, "{crossing} {wholly}");
    // and never what can't be seen: of a block's six faces, three show from here
    assert!(crossing <= 3, "{crossing} faces, some of them hidden");
    // F4 held: the left button orbits
    let f4 = winit::keyboard::KeyCode::F4;
    assert!(h.gui.held_key(f4, true));
    h.move_to(v.left() + 40.0, v.bottom() - 40.0);
    h.step();
    h.button(true);
    h.step();
    h.move_to(v.left() + 120.0, v.bottom() - 40.0);
    step_some(&mut h);
    h.button(false);
    step_some(&mut h);
    h.gui.held_key(f4, false);
    assert_ne!(h.gui.view.as_ref().unwrap().nav.camera.yaw, was.yaw);
}

#[test]
fn the_marking_menu_fits_where_things_stand_and_takes_a_direction() {
    use forge_render::shell::sketching::SkTool;
    let Some((mut h, _)) = file_part(BLOCK) else {
        return;
    };
    // on nothing, the part's commands; the rest in a list
    let (items, more) = h.gui.shell.marking_items();
    assert!(items.iter().any(|m| m.0 == "Start Sketch") && !more.is_empty());
    // shown, a click in a wedge's direction takes it, well off its word
    let c = h.gui.shell.view_rect.center();
    h.gui.shell.marking = Some(c);
    step_some(&mut h);
    assert!(h.gui.shell.place("mark:Start Sketch").is_some());
    h.click_at(c.x + 22.0, c.y - 22.0);
    step_some(&mut h);
    assert!(h.gui.shell.marking.is_none());
    assert!(h.gui.shell.sketch_pick, "up and right is Start Sketch");
    h.key(egui::Key::Escape);
    step_some(&mut h);
    // in the dead zone a click takes nothing
    h.gui.shell.marking = Some(c);
    step_some(&mut h);
    h.click_at(c.x + 3.0, c.y + 3.0);
    step_some(&mut h);
    let tool = h.gui.shell.fg.as_ref().unwrap().tool.is_some();
    assert!(h.gui.shell.marking.is_none() && !tool && !h.gui.shell.sketch_pick);
    // in a sketch, its tools
    sketch_on(&mut h, "xy");
    let items = h.gui.shell.marking_items().0;
    assert!(
        items
            .iter()
            .any(|m| m.0 == "Finish Sketch" && m.2 == Some(Action::FinishSketch))
    );
    assert!(
        items
            .iter()
            .any(|m| m.0 == "Line" && m.2 == Some(Action::SketchTool(SkTool::Line)))
    );
}

#[test]
fn a_held_right_button_shows_its_menu_and_a_quick_flick_never_does() {
    let Some((mut h, _)) = file_part(BLOCK) else {
        return;
    };
    let c = h.gui.shell.view_rect.center();
    h.move_to(c.x, c.y);
    h.step();
    h.other_button_for_test(true);
    h.step();
    assert!(h.gui.shell.marking.is_none(), "not at once");
    std::thread::sleep(std::time::Duration::from_millis(260));
    let o = h.step();
    assert!(o.again_in.is_some(), "it keeps checking while held");
    h.step();
    assert!(h.gui.shell.marking.is_some(), "held: it shows");
    h.other_button_for_test(false);
    step_some(&mut h);
    assert!(h.gui.shell.marking.is_some(), "and stays to be chosen from");
    h.key(egui::Key::Escape);
    step_some(&mut h);
    assert!(h.gui.shell.marking.is_none());
}

#[test]
fn dragging_the_browsers_edge_resizes_it_and_the_view_follows_and_a_double_click_resets_it() {
    let Some(mut h) = app() else { return };
    let w0 = h.gui.shell.side_w();
    let y = h.gui.shell.view_rect.center().y;
    h.move_to(w0, y);
    h.step();
    h.button(true);
    h.step();
    for i in 1..=10 {
        h.move_to(w0 + 12.0 * i as f32, y);
        h.step();
    }
    h.button(false);
    h.step();
    h.step();
    let w1 = h.gui.shell.side_w();
    assert!((w1 - (w0 + 120.0)).abs() < 2.0, "{w0} -> {w1}");
    assert!((h.gui.shell.view_rect.left() - (w1 + 1.0)).abs() < 1.0);
    // never past its floor
    h.move_to(w1, y);
    h.step();
    h.button(true);
    h.step();
    h.move_to(10.0, y);
    h.step();
    h.button(false);
    h.step();
    assert!(h.gui.shell.side_w() >= 180.0);
    let w2 = h.gui.shell.side_w();
    for _ in 0..2 {
        h.click_at(w2, y);
    }
    h.step();
    assert_eq!(h.gui.shell.side_w(), w0);
}

#[test]
fn appearance_cards_and_the_view_tab_change_the_look_the_view_draws() {
    let Some(mut h) = app() else { return };
    h.gui.shell.tab = 4;
    h.step();
    assert!(h.click("tool:Settings"));
    for _ in 0..4 {
        h.step();
    }
    let look = |h: &Headless| h.gui.view.as_ref().unwrap().renderer.look.clone();
    let click = |h: &mut Headless, name: &str| {
        assert!(h.click(name), "{name}");
        for _ in 0..3 {
            h.step();
        }
    };
    click(&mut h, "settings:scheme:forest");
    assert_eq!(h.gui.shell.settings.scheme, "forest");
    let forest = forge_render::look::scheme("forest");
    assert_eq!(Some(look(&h).top), forest.top);
    // the rest is below: the wheel over the window's page
    let at = h.gui.shell.place("settings:scheme:forest").unwrap().center();
    for _ in 0..4 {
        h.move_to(at.x, at.y);
        h.step();
        h.wheel(-400.0);
        for _ in 0..20 {
            h.step();
        }
    }
    click(&mut h, "settings:lighting:warm_light");
    assert_eq!(look(&h).lighting.id, "warm_light");
    click(&mut h, "settings:environment:night_sky");
    assert_eq!(look(&h).environment.id, "night_sky");
    let was = look(&h).object_shadows;
    click(&mut h, "settings:ground:object_shadows");
    assert_eq!(look(&h).object_shadows, !was);
    // View > Ground Plane is the same switch
    h.gui.shell.prefs.open = false;
    h.step();
    h.gui.shell.tab = 3;
    h.step();
    let was = look(&h).ground_plane;
    click(&mut h, "tool:Ground Plane");
    assert_eq!(look(&h).ground_plane, !was);
    assert_eq!(h.gui.shell.settings.ground_plane, !was);
}

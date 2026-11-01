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

//! Measure in the app driven as a person drives it: the pointer moved and clicked on the view
//! (no picks set by hand), orbiting, other tabs, Esc. GPU tests: skipped where there is none.

use forge_render::gui::{Gui, Headless};
use forge_render::render::Entity;
use forge_render::shell::Action;
use forge_render::shell::inspect::InspectCmd;

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

/// The demo part, opened from a copy of its own.
fn open(tag: &str) -> Option<Headless> {
    let dir = std::env::temp_dir().join(format!("fg-measure-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // the demo's folder: its font is beside it
    let demo = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/demo"));
    for e in std::fs::read_dir(demo).unwrap().flatten() {
        if e.path().is_file() {
            std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
        }
    }
    let file = dir.join("part_tools.fg");
    let mut h = match Headless::new((1400.0, 900.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    assert!(h.until(ready), "never ready: {}", h.gui.shell.status);
    for _ in 0..5 {
        h.step();
    }
    Some(h)
}

/// The place on screen of a point of the part, and what the pointer's over there once the view's
/// pick has answered.
fn hover(h: &mut Headless, world: [f32; 3]) -> ((f32, f32), Option<(u32, Entity)>) {
    assert!(h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving())));
    let (cam, size) = h.gui.shell.camera.unwrap();
    let on = cam.pixel_of(world, size.0, size.1).unwrap();
    let r = h.gui.shell.view_rect;
    let at = (r.min.x + on.0, r.min.y + on.1);
    h.move_to(at.0, at.1);
    for waited in 0..200 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let v = h.gui.view.as_ref().unwrap();
        if waited > 10 && !v.nav.pick_pending() && v.nav.marks().iter().any(|m| !m.selected) {
            break;
        }
    }
    let got = h.gui.view.as_ref().unwrap().nav.marks().into_iter().rfind(|m| !m.selected);
    (at, got.map(|m| (m.placement, m.entity)))
}

/// Points on the demo part: the base's top face, the boss's top.
const TOP: [f32; 3] = [30.0, 22.0, 12.0];
const BOSS_TOP: [f32; 3] = [4.0, 4.0, 20.0];

fn picks(h: &Headless) -> Vec<(u32, Entity)> {
    h.gui.shell.inspect.measure.as_ref().map_or(vec![], |m| m.marked())
}

fn measure(h: &mut Headless) {
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Measure));
    h.step();
    h.step();
    assert!(h.gui.shell.inspect.measure.is_some());
}

/// A click that doesn't wait for the hover's pick: pressed and let go before the view has said
/// what's there, as a quick hand on a busy view clicks.
fn quick_click(h: &mut Headless, x: f32, y: f32) {
    h.move_to(x, y);
    h.button(true);
    h.button(false);
    for _ in 0..6 {
        h.step();
    }
}

#[test]
fn a_quick_click_on_a_face_is_a_pick_not_a_click_on_nothing() {
    let Some(mut h) = open("quick") else { return };
    measure(&mut h);
    let (at, want) = hover(&mut h, TOP);
    let want = want.expect("the top face under the pointer");
    let (at2, want2) = hover(&mut h, BOSS_TOP);
    let want2 = want2.expect("the boss's top under the pointer");
    assert!(matches!(want.1, Entity::Face(_)) && matches!(want2.1, Entity::Face(_)));
    // away, then straight onto the face and click
    h.move_to(5.0, 5.0);
    h.step();
    quick_click(&mut h, at.0, at.1);
    assert_eq!(picks(&h), vec![want]);
    quick_click(&mut h, at2.0, at2.1);
    assert_eq!(picks(&h), vec![want, want2]);
}

/// A click that waits for the hover's pick, as a steady hand clicks.
fn click(h: &mut Headless, world: [f32; 3]) -> (u32, Entity) {
    let (at, e) = hover(h, world);
    h.button(true);
    h.step();
    h.button(false);
    for _ in 0..6 {
        h.step();
    }
    let _ = at;
    e.expect("something under the pointer")
}

fn reading(h: &mut Headless) -> forge_agent::measure::Reading {
    assert!(h.until(|g| {
        g.shell
            .inspect
            .measure
            .as_ref()
            .is_some_and(|m| m.reading.is_some() && !m.busy)
    }));
    let m = h.gui.shell.inspect.measure.as_ref().unwrap();
    m.reading.clone().unwrap().unwrap()
}

#[test]
fn measuring_feeds_no_other_pick_the_browser_or_a_feature_dialog() {
    let Some(mut h) = open("leak") else { return };
    measure(&mut h);
    let e = click(&mut h, [0.0, -30.0, 12.0]);
    assert!(matches!(e.1, Entity::Edge(_)), "{e:?}");
    assert_eq!(picks(&h), vec![e]);
    let p = h.gui.shell.fg.as_ref().unwrap();
    assert!(p.picked.is_none() && h.gui.shell.selected.is_none() && h.gui.shell.dim.is_none());
    assert!(!h.gui.shell.status.starts_with("Picked"), "{}", h.gui.shell.status);
    // a Fillet dialog waiting for edges takes none of Measure's
    h.gui.fg_open_tool("fillet");
    h.step();
    let f = click(&mut h, TOP);
    let tool = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap();
    assert!(tool.slots.iter().all(|s| s.picks.is_empty()), "the fillet took Measure's pick");
    assert!(tool.error.is_none(), "the fillet was given Measure's face: {:?}", tool.error);
    assert_eq!(picks(&h), vec![e, f]);
    // what the view marks is what's measured
    assert_eq!(h.gui.view.as_ref().unwrap().nav.selected, vec![e, f]);
}

#[test]
fn esc_clears_the_picks_then_ends_measure() {
    let Some(mut h) = open("esc") else { return };
    measure(&mut h);
    click(&mut h, TOP);
    let _ = reading(&mut h);
    h.key(egui::Key::Escape);
    h.step();
    h.step();
    let m = h.gui.shell.inspect.measure.as_ref().expect("still measuring");
    assert!(m.picks.is_empty() && m.reading.is_none());
    h.key(egui::Key::Escape);
    h.step();
    h.step();
    assert!(h.gui.shell.inspect.measure.is_none());
}

#[test]
fn a_window_drag_while_measuring_changes_nothing_picked() {
    let Some(mut h) = open("window") else { return };
    measure(&mut h);
    let a = click(&mut h, TOP);
    let b = click(&mut h, BOSS_TOP);
    assert_eq!(picks(&h), vec![a, b]);
    let r = h.gui.shell.view_rect;
    h.move_to(r.left() + 20.0, r.bottom() - 20.0);
    h.step();
    h.button(true);
    h.step();
    for k in 1..=10 {
        h.move_to(r.left() + 20.0 + 60.0 * k as f32, r.bottom() - 20.0 - 40.0 * k as f32);
        h.step();
    }
    h.button(false);
    // not even for a frame
    for _ in 0..6 {
        h.step();
        assert_eq!(picks(&h), vec![a, b]);
        assert_eq!(h.gui.view.as_ref().unwrap().nav.selected, vec![a, b]);
    }
}

#[test]
fn a_rebuild_keeps_the_reading_on_screen_and_reads_it_again() {
    let Some(mut h) = open("rebuild") else { return };
    measure(&mut h);
    let a = click(&mut h, TOP);
    let b = click(&mut h, BOSS_TOP);
    let _ = (a, b);
    let before = reading(&mut h);
    let gap = before.get(forge_agent::measure::Quantity::Distance).unwrap();
    assert!((gap - 8.0).abs() < 1e-6, "{gap}");
    // the boss 10 high: the same faces, further apart
    h.gui
        .fg_apply(forge_lang::error::Op::SetParam {
            param: forge_lang::error::ParamKey::parse("extrude2.d").unwrap(),
            value: forge_lang::error::Literal {
                value: 10.0,
                dim: forge_lang::values::Dim::Length,
            },
        })
        .unwrap();
    let mut blank = 0;
    for _ in 0..2000 {
        h.step();
        let m = h.gui.shell.inspect.measure.as_ref().unwrap();
        if m.reading.is_none() {
            blank += 1;
        }
        if let Some(Ok(r)) = &m.reading
            && !m.busy
            && r.get(forge_agent::measure::Quantity::Distance)
                .is_some_and(|d| (d - 10.0).abs() < 1e-6)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let after = reading(&mut h);
    let gap = after.get(forge_agent::measure::Quantity::Distance).unwrap();
    assert!((gap - 10.0).abs() < 1e-6, "{gap}");
    assert_eq!(blank, 0, "the reading went blank for {blank} frames");
}

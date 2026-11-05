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
    eprintln!("DBG hover {world:?} at {at:?} rect {r:?} got {got:?} pending {}", h.gui.view.as_ref().unwrap().nav.pick_pending());
    (at, got.map(|m| (m.placement, m.entity)))
}

/// Points on the demo part: the base's top face, the boss's top, a corner of the base.
const TOP: [f32; 3] = [30.0, 22.0, 12.0];
const BOSS_TOP: [f32; 3] = [4.0, 4.0, 20.0];
const CORNER: [f32; 3] = [40.0, 30.0, 12.0];

fn picks(h: &Headless) -> Vec<(u32, Entity)> {
    h.gui.shell.inspect.measure.as_ref().map_or(vec![], |m| m.picks.clone())
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

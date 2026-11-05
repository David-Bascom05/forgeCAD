//! Clicks however quick: pressed and let go within one frame, before the pointer's hover was
//! seen, or across frames, on a ribbon button, a browser row, the view and a dialog's button; and
//! tooltips that come on a plain rest of the pointer. GPU tests: skipped where there is none.

use forge_render::gui::{Gui, Headless};

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

/// The demo part, opened from a copy of its folder.
fn open(tag: &str) -> Option<Headless> {
    let dir = std::env::temp_dir().join(format!("fg-input-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let demo = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/demo"));
    for e in std::fs::read_dir(demo).unwrap().flatten() {
        if e.path().is_file() {
            std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
        }
    }
    let mut h = match Headless::new((1400.0, 900.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(dir.join("part_tools.fg").to_str().unwrap());
    assert!(h.until(ready), "never ready: {}", h.gui.shell.status);
    for _ in 0..5 {
        h.step();
    }
    Some(h)
}

/// How a click comes.
#[derive(Clone, Copy, Debug)]
enum Tap {
    /// Pointer there a frame first, then pressed and let go between two frames.
    Rested,
    /// Moved there, pressed and let go, all before the next frame.
    Quick,
    /// Pressed in one frame, let go in the next.
    Across,
}

const TAPS: [Tap; 3] = [Tap::Rested, Tap::Quick, Tap::Across];

fn tap_at(h: &mut Headless, at: egui::Pos2, how: Tap) {
    match how {
        Tap::Rested => {
            h.move_to(at.x, at.y);
            h.step();
            h.button(true);
            h.button(false);
        }
        Tap::Quick => {
            h.move_to(at.x, at.y);
            h.button(true);
            h.button(false);
        }
        Tap::Across => {
            h.move_to(at.x, at.y);
            h.button(true);
            h.step();
            h.button(false);
        }
    }
    for _ in 0..6 {
        h.step();
    }
}

fn tap(h: &mut Headless, place: &str, how: Tap) {
    let r = h.gui.shell.place(place).unwrap_or_else(|| panic!("no {place}"));
    tap_at(h, r.center(), how);
}

/// Somewhere off everything, so the next tap starts from away, and past a double click's time.
fn away(h: &mut Headless) {
    h.move_to(3.0, 450.0);
    for _ in 0..40 {
        h.step();
    }
}

#[test]
fn a_ribbon_button_takes_every_kind_of_click() {
    let Some(mut h) = open("ribbon") else { return };
    h.gui.shell.tab = 2;
    h.step();
    h.step();
    for how in TAPS {
        away(&mut h);
        let was = h.gui.shell.inspect.measure.is_some();
        tap(&mut h, "tool:Measure", how);
        assert_ne!(
            h.gui.shell.inspect.measure.is_some(),
            was,
            "{how:?}: the click on Measure was lost"
        );
    }
}

#[test]
fn a_browser_row_takes_every_kind_of_click() {
    let Some(mut h) = open("browser") else { return };
    for how in TAPS {
        away(&mut h);
        if let Some(f) = &mut h.gui.shell.fg {
            f.selected = None;
        }
        h.step();
        tap(&mut h, "fg:row:extrude1", how);
        let picked = h.gui.shell.fg.as_ref().and_then(|f| f.selected.clone());
        assert_eq!(
            picked.as_deref(),
            Some("extrude1"),
            "{how:?}: the click on the row was lost"
        );
    }
}

#[test]
fn a_dialog_button_takes_every_kind_of_click() {
    let Some(mut h) = open("dialog") else { return };
    h.gui.shell.tab = 2;
    h.step();
    for how in TAPS {
        h.gui.shell.out.push(forge_render::shell::Action::Inspect(
            forge_render::shell::inspect::InspectCmd::Measure,
        ));
        for _ in 0..3 {
            h.step();
        }
        assert!(h.gui.shell.inspect.measure.is_some());
        away(&mut h);
        tap(&mut h, "measure:close:close", how);
        assert!(
            h.gui.shell.inspect.measure.is_none(),
            "{how:?}: the click on Close was lost"
        );
    }
}

#[test]
fn a_click_in_the_view_takes_what_is_there_even_before_the_view_has_looked() {
    let Some(mut h) = open("view") else { return };
    assert!(h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving())));
    let (cam, size) = h.gui.shell.camera.unwrap();
    // the boss's top
    let on = cam.pixel_of([4.0, 4.0, 20.0], size.0, size.1).unwrap();
    let r = h.gui.shell.view_rect;
    let at = egui::pos2(r.min.x + on.0, r.min.y + on.1);
    for how in TAPS {
        away(&mut h);
        if let Some(v) = &mut h.gui.view {
            v.nav.selected.clear();
        }
        tap_at(&mut h, at, how);
        let v = h.gui.view.as_ref().unwrap();
        assert!(
            !v.nav.selected.is_empty(),
            "{how:?}: the click on the boss chose nothing"
        );
    }
}

#[test]
fn a_tooltip_comes_on_a_plain_rest_and_the_longer_one_after_it() {
    use forge_render::shell::Shell;
    let Some(mut h) = open("tip") else { return };
    let (short, long) = (h.gui.shell.settings.tip_delay, h.gui.shell.settings.tip_long);
    assert_eq!((short, long), (0.6, 2.0));
    let r = h.gui.shell.place("tool:Extrude").unwrap();
    away(&mut h);
    h.gui.ctx.data_mut(|d| d.clear());
    h.move_to(r.center().x, r.center().y);
    // the pointer still, no key pressed: frames a sixtieth of a second apart
    let (mut first, mut longer) = (None, None);
    for k in 1..=200 {
        h.step();
        match Shell::tip_shown(&h.gui.ctx) {
            Some((name, false)) if first.is_none() => {
                assert_eq!(name, "Extrude");
                first = Some(k);
            }
            Some((_, true)) if longer.is_none() => longer = Some(k),
            _ => {}
        }
    }
    let secs = |k: Option<usize>| k.map(|k| k as f32 / 60.0);
    let (a, b) = (
        secs(first).expect("no tooltip on a rest"),
        secs(longer).expect("no longer tooltip"),
    );
    assert!((a - short).abs() < 0.1, "the tooltip came after {a} s");
    assert!((b - long).abs() < 0.1, "the longer one came after {b} s");
    // the delays are Settings': half a second sooner
    h.gui.shell.settings.tip_delay = 0.1;
    away(&mut h);
    h.gui.ctx.data_mut(|d| d.clear());
    h.move_to(r.center().x, r.center().y);
    let mut at = None;
    for k in 1..=60 {
        h.step();
        if at.is_none() && Shell::tip_shown(&h.gui.ctx).is_some() {
            at = Some(k);
        }
    }
    assert!(secs(at).expect("no tooltip") < 0.25);
}

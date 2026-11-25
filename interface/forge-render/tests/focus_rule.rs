//! STANDARD §9, a dialog's focus: a pick-first feature dialog keeps the keys for the view, but a
//! digit typed with nothing focused starts the entry in its first number field (pick the edges,
//! type 2, Enter); the pick slot taking picks is the focus the audit sees (`focus:pick`).

use forge_render::gui::Headless;

const PART: &str = "shared t = 6\nsketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (60, 40))\n}\n\
plate = extrude(plan, t)\n";

#[test]
fn a_digit_typed_in_a_pick_first_dialog_starts_its_first_number() {
    let Ok(mut h) = Headless::new((1400.0, 860.0), 1.0) else {
        eprintln!("skipped: no GPU");
        return;
    };
    let d = std::env::temp_dir().join(format!("fg-focus-rule-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("plate.fg"), PART).unwrap();
    h.gui.open(d.join("plate.fg").to_str().unwrap());
    assert!(h.until(|g| g.fg.as_ref().is_some_and(|s| s.seq > 0 && s.done == s.seq)));
    for _ in 0..6 {
        h.step();
    }
    assert!(h.click("tool:Fillet"));
    for _ in 0..4 {
        h.step();
    }
    assert!(
        h.gui.shell.place("focus:pick").is_some(),
        "the Edges slot takes picks: it's the focus"
    );
    let first = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap().params[0].name;
    h.text("2");
    for _ in 0..3 {
        h.step();
    }
    let tool = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap();
    let p = tool.params.iter().find(|p| p.name == first).unwrap();
    assert_eq!(p.text.trim(), "2", "the digit went to {first}");
    assert!(
        h.gui.ctx.memory(|m| m.focused().is_some()),
        "and the field has the keys now"
    );
}

fn fillet(h: &mut Headless) -> &'static str {
    let d = std::env::temp_dir().join(format!("fg-focus-rule2-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("plate.fg"), PART).unwrap();
    h.gui.open(d.join("plate.fg").to_str().unwrap());
    assert!(h.until(|g| g.fg.as_ref().is_some_and(|s| s.seq > 0 && s.done == s.seq)));
    for _ in 0..6 {
        h.step();
    }
    assert!(h.click("tool:Fillet"));
    for _ in 0..4 {
        h.step();
    }
    h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap().params[0].name
}

fn first_text(h: &Headless, name: &str) -> String {
    let tool = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap();
    tool.params.iter().find(|p| p.name == name).unwrap().text.clone()
}

#[test]
fn esc_after_a_digit_puts_the_old_value_back_and_the_keys_with_the_pick() {
    let Ok(mut h) = Headless::new((1400.0, 860.0), 1.0) else {
        return;
    };
    let first = fillet(&mut h);
    let was = first_text(&h, first);
    h.text("7");
    for _ in 0..3 {
        h.step();
    }
    assert_eq!(first_text(&h, first).trim(), "7");
    h.key(egui::Key::Escape);
    for _ in 0..3 {
        h.step();
    }
    assert!(
        h.gui.shell.fg.as_ref().unwrap().tool.is_some(),
        "the dialog stays open"
    );
    assert_eq!(first_text(&h, first), was, "its old value back");
    assert!(
        h.gui.ctx.memory(|m| m.focused().is_none()),
        "the keys back with the view"
    );
    assert!(
        h.gui.shell.place("focus:pick").is_some(),
        "the pick slot still takes picks"
    );
    // a second Esc closes it, as ever
    h.key(egui::Key::Escape);
    for _ in 0..3 {
        h.step();
    }
    assert!(h.gui.shell.fg.as_ref().unwrap().tool.is_none());
}

#[test]
fn a_digit_typed_mid_drag_is_not_handed_to_a_field() {
    let Ok(mut h) = Headless::new((1400.0, 860.0), 1.0) else {
        return;
    };
    let first = fillet(&mut h);
    let was = first_text(&h, first);
    h.move_to(900.0, 500.0);
    h.step();
    h.button(true);
    h.step();
    h.move_to(940.0, 520.0);
    h.text("3");
    for _ in 0..3 {
        h.step();
    }
    h.button(false);
    h.step();
    assert_eq!(
        first_text(&h, first),
        was,
        "a drag's digit stays out of the field"
    );
    assert!(h.gui.ctx.memory(|m| m.focused().is_none()));
}

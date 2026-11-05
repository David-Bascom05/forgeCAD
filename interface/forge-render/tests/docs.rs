//! Several documents open at once, Inventor's way: a tab each along the bottom, each keeping its
//! own camera, selection and undo; Ctrl+Tab round them; closing one with changes asks first; a
//! part saved from its tab refreshes the assembly that uses it; a part the agent makes opens.

use forge_lang::error::{Literal, Op, ParamKey};
use forge_lang::values::Dim;
use forge_render::gui::{Gui, Headless};
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-docs-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

const PLATE: &str = "shared t = 6
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (60, 40))
}
plate = extrude(plan, t)
";

fn bar(len: f64) -> String {
    format!(
        "sketch plan(on: xy) {{
  outline = rect(corner: (-4, -4), size: ({}, 8))
  p0 = pt(0, 0)
  p1 = pt({len}, 0)
}}
bar = extrude(plan, 4)
a = anchor(plane(at: plan.p0, normal: plan.plane.normal, x: plan.outline[0].dir))
b = anchor(plane(at: plan.p1, normal: plan.plane.normal, x: plan.outline[0].dir))
",
        len + 8.0
    )
}

const ASM: &str = "assembly

base = part(\"bar.fg\", ground: true)
top = part(\"bar.fg\", ground: true, at: pose(at: (0, 0, 4)))
";

/// A plate, a bar and an assembly of two bars.
fn files(d: &Path) -> [PathBuf; 3] {
    let f = [d.join("plate.fg"), d.join("bar.fg"), d.join("frame.fg")];
    std::fs::write(&f[0], PLATE).unwrap();
    std::fs::write(&f[1], bar(100.0)).unwrap();
    std::fs::write(&f[2], ASM).unwrap();
    f
}

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.is_loading()
}

fn ready(h: &mut Headless) {
    assert!(
        h.until(settled),
        "{} never ready: {} view {} loading {}",
        h.gui.shell.name,
        h.gui.shell.status,
        h.gui.view.is_some(),
        h.gui.is_loading()
    );
    for _ in 0..3 {
        h.step();
    }
}

fn open_in(h: &mut Headless, file: &Path) {
    h.gui.open(file.to_str().unwrap());
    ready(h);
}

fn headless() -> Option<Headless> {
    match Headless::new((1300.0, 900.0), 1.0) {
        Ok(mut h) => {
            h.step();
            Some(h)
        }
        Err(e) => {
            eprintln!("skipped: {e}");
            None
        }
    }
}

fn set_t(v: f64) -> Op {
    Op::SetParam {
        param: ParamKey::shared("t"),
        value: Literal {
            value: v,
            dim: Dim::Length,
        },
    }
}

fn yaw(h: &Headless) -> f32 {
    h.gui.view.as_ref().unwrap().nav.camera.yaw
}

fn titles(h: &Headless) -> Vec<String> {
    h.gui.documents().iter().map(|d| d.title.clone()).collect()
}

#[test]
fn three_documents_each_keep_their_own() {
    let Some(mut h) = headless() else { return };
    let d = scratch("three");
    let [plate, bar_f, frame] = files(&d);
    open_in(&mut h, &plate);
    // the plate: turned, its extrude chosen, an edit to undo
    h.gui.view.as_mut().unwrap().nav.camera.yaw += 0.7;
    let plate_yaw = yaw(&h);
    h.gui.shell.fg.as_mut().unwrap().selected = Some("plate".into());
    h.gui.fg_apply(set_t(9.0)).unwrap();
    ready(&mut h);
    open_in(&mut h, &bar_f);
    open_in(&mut h, &frame);
    assert_eq!(titles(&h), ["plate.fg", "bar.fg", "frame.fg"]);
    assert_eq!(h.gui.active_document(), 2);
    assert!(h.gui.shell.fg.as_ref().unwrap().assembly);
    // the assembly's tab is a tab in the strip
    for t in ["plate.fg", "bar.fg", "frame.fg"] {
        assert!(h.gui.shell.place(&format!("doc:tab:{t}")).is_some(), "{t}");
    }
    // the bar has nothing chosen and nothing to undo
    h.gui.activate(1);
    h.step();
    assert_eq!(h.gui.shell.name, "bar.fg");
    assert!(h.gui.shell.fg.as_ref().unwrap().selected.is_none());
    assert!(!h.gui.fg.as_mut().unwrap().undo());
    // a click on the plate's tab brings it back as it was left: unsaved, turned, chosen
    assert!(h.click("doc:tab:plate.fg"));
    h.step();
    assert_eq!(h.gui.active_document(), 0);
    assert_eq!(h.gui.shell.name, "plate.fg");
    assert_eq!(yaw(&h), plate_yaw);
    assert_eq!(
        h.gui.shell.fg.as_ref().unwrap().selected.as_deref(),
        Some("plate")
    );
    assert!(h.gui.documents()[0].dirty);
    assert!(h.gui.fg.as_ref().unwrap().src.contains("shared t = 9"));
    // its undo is its own: the bar's file is untouched by it
    h.gui.shell.out.push(forge_render::shell::Action::Undo);
    h.step();
    assert!(h.gui.fg.as_ref().unwrap().src.contains("shared t = 6"));
    assert!(!h.gui.documents()[0].dirty);
    assert_eq!(std::fs::read_to_string(&bar_f).unwrap(), bar(100.0));
    // opening one that's open brings its tab up, no second one
    h.gui.open(frame.to_str().unwrap());
    h.step();
    assert_eq!(h.gui.documents().len(), 3);
    assert_eq!(h.gui.active_document(), 2);
    // Ctrl+Tab round, Ctrl+Shift+Tab back
    h.key_with(egui::Key::Tab, egui::Modifiers::CTRL);
    h.step();
    assert_eq!(h.gui.active_document(), 0);
    h.key_with(egui::Key::Tab, egui::Modifiers::CTRL | egui::Modifiers::SHIFT);
    h.step();
    assert_eq!(h.gui.active_document(), 2);
    // a tab dragged to the front: the assembly first, still the one up
    h.gui.move_document(2, 0);
    h.step();
    assert_eq!(titles(&h), ["frame.fg", "plate.fg", "bar.fg"]);
    assert_eq!(h.gui.active_document(), 0);
    assert!(h.gui.shell.fg.as_ref().unwrap().assembly);
}

#[test]
fn closing_with_changes_asks() {
    let Some(mut h) = headless() else { return };
    let d = scratch("close");
    let [plate, bar_f, _] = files(&d);
    open_in(&mut h, &plate);
    open_in(&mut h, &bar_f);
    h.gui.activate(0);
    h.step();
    h.gui.fg_apply(set_t(11.0)).unwrap();
    ready(&mut h);
    assert!(h.gui.documents()[0].dirty);
    // its × asks; Cancel keeps it
    assert!(h.click("doc:close:plate.fg"));
    h.step();
    h.step();
    assert_eq!(h.gui.shell.close_ask, Some(0));
    assert!(h.click("doc:ask:cancel"));
    h.step();
    assert_eq!(h.gui.shell.close_ask, None);
    assert_eq!(h.gui.documents().len(), 2);
    // Ctrl+W asks again; Save writes it and closes it, the bar comes up
    h.key_with(egui::Key::W, egui::Modifiers::CTRL);
    h.step();
    h.step();
    assert_eq!(h.gui.shell.close_ask, Some(0));
    assert!(h.click("doc:ask:save"));
    h.step();
    assert_eq!(titles(&h), ["bar.fg"]);
    assert_eq!(h.gui.shell.name, "bar.fg");
    assert!(std::fs::read_to_string(&plate).unwrap().contains("shared t = 11"));
    // a tab with nothing to save closes at once; the last leaves the window empty
    h.step();
    assert!(h.click("doc:close:bar.fg"));
    h.step();
    assert!(h.gui.documents().is_empty());
    assert!(h.gui.fg.is_none() && h.gui.view.is_none());
    h.step();
    // Don't Save: the file stays as it was
    open_in(&mut h, &plate);
    h.gui.fg_apply(set_t(13.0)).unwrap();
    ready(&mut h);
    h.gui.request_close(0);
    h.step();
    assert!(h.click("doc:ask:discard"));
    h.step();
    assert!(h.gui.documents().is_empty());
    assert!(std::fs::read_to_string(&plate).unwrap().contains("shared t = 11"));
}

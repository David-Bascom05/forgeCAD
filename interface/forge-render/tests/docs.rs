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
        "shared ln = {len}
sketch plan(on: xy) {{
  outline = rect(corner: (-4, -4), size: (ln + 8, 8))
  p0 = pt(0, 0)
  p1 = pt(ln, 0)
}}
bar = extrude(plan, 4)
a = anchor(plane(at: plan.p0, normal: plan.plane.normal, x: plan.outline[0].dir))
b = anchor(plane(at: plan.p1, normal: plan.plane.normal, x: plan.outline[0].dir))
"
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

fn set_len(v: f64) -> Op {
    Op::SetParam {
        param: ParamKey::shared("ln"),
        value: Literal {
            value: v,
            dim: Dim::Length,
        },
    }
}

fn boxes(h: &Headless) -> String {
    format!("{:?}", h.gui.fg_asm_boxes)
}

#[test]
fn a_part_saved_refreshes_its_assembly() {
    let Some(mut h) = headless() else { return };
    let d = scratch("refresh");
    let [_, bar_f, frame] = files(&d);
    open_in(&mut h, &frame);
    let was = boxes(&h);
    open_in(&mut h, &bar_f);
    // an edit not saved is the bar's own: the assembly reads the file
    h.gui.fg_apply(set_len(200.0)).unwrap();
    ready(&mut h);
    h.gui.activate(0);
    ready(&mut h);
    assert_eq!(boxes(&h), was);
    // saved from its tab: the assembly, idle, builds again on its own
    h.gui.activate(1);
    h.step();
    h.gui.fg_save();
    h.step();
    assert!(!h.gui.documents()[1].dirty);
    h.gui.activate(0);
    assert!(
        h.until(|g| settled(g) && format!("{:?}", g.fg_asm_boxes) != was),
        "the assembly never saw the bar change"
    );
    // its undo is untouched: nothing to undo in the assembly
    assert!(!h.gui.fg.as_mut().unwrap().undo());
}

#[test]
fn a_file_changed_on_disk_reads_again() {
    let Some(mut h) = headless() else { return };
    let d = scratch("disk");
    let [plate, ..] = files(&d);
    open_in(&mut h, &plate);
    std::fs::write(&plate, PLATE.replace("t = 6", "t = 20")).unwrap();
    h.gui.check_files();
    ready(&mut h);
    let s = h.gui.fg.as_ref().unwrap();
    assert!(s.src.contains("t = 20") && !s.dirty());
    // changed here and on disk: what's here is kept
    h.gui.fg_apply(set_t(7.0)).unwrap();
    // a length of its own: two writes in one tick of the file clock still differ
    std::fs::write(&plate, PLATE.replace("t = 6", "t = 300")).unwrap();
    h.gui.check_files();
    let said = h.gui.shell.status.clone();
    assert!(said.contains("changed on disk"), "{said}");
    h.step();
    assert!(h.gui.fg.as_ref().unwrap().src.contains("t = 7"));
}

#[test]
fn a_part_the_agent_makes_opens() {
    let Some(mut h) = headless() else { return };
    let d = scratch("agent");
    let [plate, ..] = files(&d);
    open_in(&mut h, &plate);
    // not in an agent's turn: a new file is left alone
    std::fs::write(d.join("other.fg"), PLATE).unwrap();
    h.gui.check_files();
    assert_eq!(h.gui.documents().len(), 1);
    h.gui.agent_turn(true);
    std::fs::write(d.join("bracket.fg"), bar(40.0)).unwrap();
    h.gui.check_files();
    assert_eq!(titles(&h), ["plate.fg", "bracket.fg"]);
    assert_eq!(h.gui.active_document(), 1);
    ready(&mut h);
    assert_eq!(h.gui.shell.name, "bracket.fg");
    // the agent's tab says what it wrote: opened once, not again by the watch
    std::fs::write(d.join("hinge.fg"), bar(30.0)).unwrap();
    let hinge = d.join("hinge.fg");
    assert_eq!(h.gui.open_document(hinge.to_str().unwrap(), true), Some(2));
    h.gui.check_files();
    h.gui.agent_turn(false);
    assert_eq!(titles(&h), ["plate.fg", "bracket.fg", "hinge.fg"]);
    // opened in the background: the active one stays
    let other = d.join("other.fg");
    assert_eq!(h.gui.open_document(other.to_str().unwrap(), false), Some(3));
    assert_eq!(h.gui.active_document(), 2);
    ready(&mut h);
    assert_eq!(h.gui.shell.name, "hinge.fg");
}

#[test]
fn an_occurrence_opens_its_part_in_a_tab() {
    let Some(mut h) = headless() else { return };
    let d = scratch("occ");
    let [_, _, frame] = files(&d);
    open_in(&mut h, &frame);
    assert!(h.menu_item("asm:row:top", "asm:menu:open"), "no Open on the row");
    ready(&mut h);
    assert_eq!(titles(&h), ["frame.fg", "bar.fg"]);
    assert_eq!(h.gui.active_document(), 1);
    assert!(!h.gui.shell.fg.as_ref().unwrap().assembly);
}

#[test]
fn new_is_an_untitled_tab() {
    let Some(mut h) = headless() else { return };
    let d = scratch("new");
    let [plate, ..] = files(&d);
    open_in(&mut h, &plate);
    h.gui.shell.out.push(forge_render::shell::Action::New);
    ready(&mut h);
    assert_eq!(titles(&h), ["plate.fg", "Part1.fg"]);
    assert_eq!(h.gui.active_document(), 1);
    // its first Save asks the host where
    h.gui.fg_save();
    let out = h.step();
    assert!(out.host.contains(&forge_render::shell::Action::SaveAs));
    let to = d.join("bracket.fg");
    h.gui.save_as(to.to_str().unwrap());
    h.step();
    assert!(to.exists());
    assert_eq!(titles(&h), ["plate.fg", "bracket.fg"]);
    // and it's that file now: opening it finds its tab
    h.gui.activate(0);
    assert_eq!(h.gui.open_document(to.to_str().unwrap(), true), Some(1));
}

#[test]
fn idle_tabs_past_the_cap_let_their_pictures_go() {
    let Some(mut h) = headless() else { return };
    let d = scratch("cap");
    let n = forge_render::gui::gui_docs::RESIDENT_VIEWS + 2;
    let mut yaws = vec![];
    for k in 0..n {
        let f = d.join(format!("bar{k}.fg"));
        std::fs::write(&f, bar(20.0 + k as f64)).unwrap();
        open_in(&mut h, &f);
        h.gui.view.as_mut().unwrap().nav.camera.yaw += 0.1 * k as f32;
        yaws.push(yaw(&h));
    }
    let kept: Vec<bool> = h.gui.documents().iter().map(|d| d.resident()).collect();
    assert_eq!(
        kept.iter().filter(|k| **k).count(),
        forge_render::gui::gui_docs::RESIDENT_VIEWS
    );
    // the first two, least lately used, let theirs go
    assert!(!kept[0] && !kept[1], "{kept:?}");
    // back up: built again, the camera where it was left
    h.gui.activate(0);
    ready(&mut h);
    assert_eq!(yaw(&h), yaws[0]);
    assert!(h.gui.documents()[0].resident());
}

#[test]
fn what_follows_an_open_is_for_the_document_it_came_from() {
    let Some(mut h) = headless() else { return };
    let d = scratch("order");
    let [plate, bar_f, _] = files(&d);
    open_in(&mut h, &plate);
    h.gui.fg_apply(set_t(15.0)).unwrap();
    ready(&mut h);
    // one frame's acts: open the bar, then save, then a tab ask
    let p = h.gui.shell.fg.as_mut().unwrap();
    p.out.push(forge_render::shell::fg::FgAct::OpenFile(
        bar_f.to_string_lossy().into_owned(),
    ));
    p.out.push(forge_render::shell::fg::FgAct::Save);
    h.step();
    ready(&mut h);
    assert_eq!(titles(&h), ["plate.fg", "bar.fg"]);
    assert_eq!(h.gui.active_document(), 1);
    // the Save was the plate's
    assert!(std::fs::read_to_string(&plate).unwrap().contains("shared t = 15"));
    assert!(!h.gui.documents()[0].dirty);
    assert_eq!(std::fs::read_to_string(&bar_f).unwrap(), bar(100.0));
}

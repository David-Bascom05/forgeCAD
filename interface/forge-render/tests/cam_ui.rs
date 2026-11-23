//! The Manufacture workspace in the app: a machining document opens on its Manufacture tab over
//! its part, generates (every op certified, as `forgecad cam` says), and the browser, the op and
//! setup dialogs, the post dialog and the tool library each make their edit to the document's
//! text, which stays the source of truth. `FORGE_UI_SHOTS=DIR` keeps pictures.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::CAM_TAB;

fn generated(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg.as_ref().is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
        && g.shell
            .cam
            .as_ref()
            .is_some_and(|w| w.run.is_some() && w.running.is_none() && !w.want_run)
}

fn settle(h: &mut Headless) {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 120.0 {
        if h.until(generated) {
            break;
        }
    }
    assert!(generated(&h.gui), "never generated: {}", h.gui.shell.status);
    for _ in 0..4 {
        h.step();
    }
}

fn shot(h: &mut Headless, name: &str) {
    for _ in 0..12 {
        h.step();
    }
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS")
        && let Ok(png) = h.png()
    {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}.png")), png).unwrap();
    }
}

/// The flange and its machining document, copied to a directory of the test's own.
fn open(tag: &str) -> Option<(Headless, std::path::PathBuf)> {
    let ex = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
    let dir = std::env::temp_dir().join(format!("fg-cam-ui-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["flange.fg", "flange_cam.fg"] {
        std::fs::copy(ex.join(f), dir.join(f)).unwrap();
    }
    let file = dir.join("flange_cam.fg");
    let mut h = match Headless::new((1500.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    settle(&mut h);
    Some((h, file))
}

fn text(h: &Headless) -> String {
    h.gui.shell.cam.as_ref().unwrap().text.clone()
}

#[test]
fn a_machining_document_opens_on_manufacture_and_generates() {
    let Some((mut h, _)) = open("open") else { return };
    assert_eq!(h.gui.shell.tab, CAM_TAB);
    assert!(h.gui.shell.place("tab:Manufacture").is_some());
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert_eq!(w.run.as_ref().unwrap().certification, "except G5");
    for op in ["skim", "top", "prof", "bore", "bolts"] {
        assert_eq!(w.status(op), "built", "{op}");
        assert!(h.gui.shell.place(&format!("cam:row:{op}")).is_some(), "{op} row");
    }
    for tool in ["New Setup", "Face", "2D Contour", "Pocket", "Drill", "Generate", "Post Process"] {
        assert!(h.gui.shell.place(&format!("tool:{tool}")).is_some(), "{tool}");
    }
    shot(&mut h, "cam-opened");
}

#[test]
fn the_op_dialog_edits_the_text_and_marks_it_stale() {
    let Some((mut h, file)) = open("op") else { return };
    let before = text(&h);
    assert!(h.click("cam:row:prof"));
    h.step();
    assert_eq!(h.gui.shell.cam.as_ref().unwrap().selected.as_deref(), Some("prof"));
    shot(&mut h, "cam-selected");
    h.gui.shell.cam.as_mut().unwrap().auto = false;
    h.gui.shell.cam.as_mut().unwrap().command(forge_render::shell::cam::Cmd::Edit("prof".into()));
    h.step();
    assert!(h.placed("cam:op:ok").is_some(), "the op dialog");
    // the Passes tab: stock to leave
    let tabs = forge_render::shell::cam::schema::tabs(forge_render::shell::cam::schema::spec("contour").unwrap());
    let k = tabs.iter().position(|t| *t == forge_render::shell::cam::schema::Tab::Passes).unwrap();
    assert!(h.click(&format!("cam:op:tab:{k}")));
    h.step();
    assert!(h.click("cam:op:arg:leave"));
    h.text("0.2");
    h.step();
    shot(&mut h, "cam-op-dialog");
    assert!(h.click("cam:op:ok"));
    h.step();
    let after = text(&h);
    assert_ne!(after, before);
    assert!(after.contains("leave: 0.2)"), "{after}");
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert_eq!(w.status("prof"), "stale");
    assert_eq!(w.status("skim"), "built", "an earlier op is untouched");
    // the file is the document's only when saved
    assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    h.gui.shell.cam.as_mut().unwrap().command(forge_render::shell::cam::Cmd::Save);
    h.step();
    assert_eq!(std::fs::read_to_string(&file).unwrap(), after);
}

#[test]
fn post_process_shows_the_program_and_writes_it() {
    let Some((mut h, file)) = open("post") else { return };
    assert!(h.click("tool:Post Process"));
    h.step();
    assert!(h.placed("cam:post:text").is_some());
    shot(&mut h, "cam-post");
    assert!(h.click("cam:post:write"));
    h.step();
    let nc = std::fs::read_to_string(file.with_file_name("prog.nc")).expect("written");
    assert!(nc.lines().count() > 100, "{nc}");
}

#[test]
fn the_tool_library_adds_a_tool_to_the_document() {
    let Some((mut h, _)) = open("tools") else { return };
    assert!(h.click("tool:Tool Library"));
    h.step();
    shot(&mut h, "cam-tools");
    let row = h
        .gui
        .shell
        .places
        .iter()
        .map(|p| p.0.clone())
        .find(|n| n.starts_with("cam:tools:row:flat-6"))
        .expect("a 6 mm flat in the library");
    assert!(h.click(&row));
    h.step();
    assert!(text(&h).contains("tool(\"flat-6\", t: 2"), "{}", text(&h));
}

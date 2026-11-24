//! The Manufacture workspace in the app: a machining document opens on its Manufacture tab over
//! its part, generates (every op certified, as `forgecad cam` says), and the browser, the op and
//! setup dialogs, the post dialog and the tool library each make their edit to the document's
//! text, which stays the source of truth. `FORGE_UI_SHOTS=DIR` keeps pictures.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::CAM_TAB;

fn generated(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
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
    for tool in [
        "New Setup",
        "Face",
        "2D Contour",
        "Pocket",
        "Drill",
        "Generate",
        "Post Process",
    ] {
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
    assert_eq!(
        h.gui.shell.cam.as_ref().unwrap().selected.as_deref(),
        Some("prof")
    );
    shot(&mut h, "cam-selected");
    h.gui.shell.cam.as_mut().unwrap().auto = false;
    h.gui
        .shell
        .cam
        .as_mut()
        .unwrap()
        .command(forge_render::shell::cam::Cmd::Edit("prof".into()));
    h.step();
    assert!(h.placed("cam:op:ok").is_some(), "the op dialog");
    // the Passes tab: stock to leave
    let tabs =
        forge_render::shell::cam::schema::tabs(forge_render::shell::cam::schema::spec("contour").unwrap());
    let k = tabs
        .iter()
        .position(|t| *t == forge_render::shell::cam::schema::Tab::Passes)
        .unwrap();
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
    h.gui
        .shell
        .cam
        .as_mut()
        .unwrap()
        .command(forge_render::shell::cam::Cmd::Save);
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
    // the flange's setup is not placed (G5 unchecked): refused until allowed
    assert!(h.click("cam:post:write"));
    h.step();
    assert!(!file.with_file_name("prog.nc").exists());
    let said = h.gui.shell.cam.as_ref().unwrap().said.clone().unwrap_or_default();
    assert!(said.contains("Allow uncertified"), "{said}");
    h.gui.shell.cam.as_mut().unwrap().allow_uncertified = true;
    assert!(h.click("cam:post:write"));
    h.step();
    h.step();
    let nc = std::fs::read_to_string(file.with_file_name("prog.nc")).expect("written");
    assert!(nc.lines().count() > 100, "{nc}");
    assert!(nc.contains("(NOT CERTIFIED IN FULL: except G5"), "stamped");
}

/// NC Programs: a program's file named and placed by its statement (typed in the window), posted
/// into the document's folder, current until the ops change, stale after, re-posted; Export… asks
/// the host's Save As with the same file; the setup sheet and tool list as two files.
#[test]
fn nc_programs_post_export_and_go_stale() {
    use forge_render::shell::cam::nc::Posted;
    let Some((mut h, file)) = open("nc") else { return };
    assert!(h.click("tool:NC Programs"));
    h.step();
    shot(&mut h, "cam-nc-open");
    assert!(h.click("cam:programs:allow"));
    h.step();
    // the file's name and folder, typed: written to the statement when the field is left
    assert!(h.click("cam:programs:prog:file"));
    h.text("flange-1");
    h.step();
    assert!(h.click("cam:programs:prog:out"));
    h.text("nc");
    h.step();
    assert!(h.click("cam:programs:prog:tools"));
    h.step();
    let t = text(&h);
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert!(t.contains("file: \"flange-1\""), "{:?} {:?} {}", w.said, w.prog_field, t.lines().filter(|l| l.starts_with("prog")).collect::<String>());
    assert!(t.contains("tools: \"list\""), "{t}");
    settle(&mut h);
    let st = |h: &mut forge_render::gui::Headless| h.gui.shell.cam.as_mut().unwrap().posted();
    assert!(h.placed("cam:programs:prog:post").is_some());
    let s = st(&mut h);
    assert_eq!(s[0].1, Posted::No, "{s:?}");
    assert!(t.contains("out: \"nc\""), "{t}");
    assert_eq!(s[0].2, "nc/flange-1.nc");
    assert!(h.click("cam:programs:prog:post"));
    h.step();
    h.step();
    let nc = std::fs::read_to_string(file.with_file_name("nc").join("flange-1.nc")).expect("posted");
    assert!(nc.contains("(TOOLS)") && nc.contains("(NOT CERTIFIED IN FULL"), "{nc}");
    assert_eq!(st(&mut h)[0].1, Posted::Current);
    shot(&mut h, "cam-nc-programs");
    // the same file for Export…: the host's Save As is asked
    assert!(h.click("cam:programs:prog:export"));
    h.step();
    let x = h.gui.shell.cam.as_mut().unwrap().export.take().expect("asked");
    assert_eq!(x.files.len(), 1);
    assert_eq!(x.files[0].text, nc);
    assert!(x.dir.ends_with("nc"), "{:?}", x.dir);
    // an op changed: the run changes, the file is stale; Re-post Stale makes it current
    let w = h.gui.shell.cam.as_mut().unwrap();
    w.edit(&forge_render::shell::cam::doc::Edit::Set { id: "prof".into(), arg: "leave".into(), text: "0.2".into() });
    settle(&mut h);
    assert_eq!(st(&mut h)[0].1, Posted::Stale);
    assert!(h.click("cam:programs:repost"));
    h.step();
    h.step();
    assert_eq!(st(&mut h)[0].1, Posted::Current);
    // the setup sheet and the tool list
    assert!(h.click("cam:programs:sheet"));
    h.step();
    let x = h.gui.shell.cam.as_mut().unwrap().export.take().expect("asked");
    let names: Vec<&str> = x.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(names, ["flange_cam-sheet.html", "flange_cam-tools.txt"]);
    assert!(x.files[1].text.starts_with("T1 "), "{}", x.files[1].text);
    let d = file.with_file_name("exported");
    let ps = forge_render::shell::cam::nc::write_to(&x.files, &d).unwrap();
    assert!(ps.iter().all(|p| p.exists()));
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

/// Feeds & Speeds: a milling op's tab shows each value as the run resolved it (its source as the
/// placeholder) with the power, torque and checks; a value typed is the op's argument.
#[test]
fn feeds_and_speeds_tab_writes_the_ops_cutting_data() {
    use forge_render::shell::cam::schema::{Tab, spec, tabs};
    let Some((mut h, _)) = open("feeds") else { return };
    let w = h.gui.shell.cam.as_mut().unwrap();
    w.auto = false;
    let r = w.run.as_ref().unwrap().ops.iter().find(|o| o.name == "prof").unwrap().resolved.clone();
    let r = r.expect("prof's cutting data resolved");
    assert!(r.rpm.v > 0.0 && r.feed.v > 0.0);
    w.command(forge_render::shell::cam::Cmd::Edit("prof".into()));
    h.step();
    let k = tabs(spec("contour").unwrap()).iter().position(|t| *t == Tab::Feeds).expect("a Feeds & Speeds tab");
    assert!(!tabs(spec("drill").unwrap()).contains(&Tab::Feeds), "drilling keeps its own feeds");
    assert!(h.click(&format!("cam:op:tab:{k}")));
    h.step();
    shot(&mut h, "cam-feeds");
    assert!(h.click("cam:op:arg:rpm"));
    h.text("6000");
    h.step();
    assert!(h.click("cam:op:ok"));
    h.step();
    let t = text(&h);
    assert!(t.lines().any(|l| l.starts_with("prof") && l.contains("rpm: 6000")), "{t}");
}

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
            .is_some_and(|w| {
                w.running.is_none()
                    && !w.want_run
                    // the run shown, and its quality measured too (off the UI thread, after the
                    // run is shown): what a test then reads or clicks does not race it
                    && w.run.as_ref().is_some_and(|r| r.cam.is_none() || r.quality.is_some())
            })
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

/// A click by name once it is on screen and laid out (a panel just opened draws its widgets a
/// frame or more later, and a change above a button can move it): steps until the place is there
/// and has held still for a frame, then clicks it.
fn tap(h: &mut Headless, name: &str) {
    let t0 = std::time::Instant::now();
    let mut last = None;
    loop {
        h.step();
        let now = h.gui.shell.place(name);
        if now.is_some() && now == last {
            break;
        }
        assert!(t0.elapsed().as_secs() < 60, "`{name}` never on screen");
        last = now;
    }
    assert!(h.click(name), "{name}");
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
    open_with(tag, false)
}

/// The same with the setup not placed (no `at:`): its machine positions unchecked (G5).
fn open_unplaced(tag: &str) -> Option<(Headless, std::path::PathBuf)> {
    open_with(tag, true)
}

fn open_with(tag: &str, unplaced: bool) -> Option<(Headless, std::path::PathBuf)> {
    open_sized(tag, unplaced, (1500.0, 1000.0))
}

fn open_sized(tag: &str, unplaced: bool, size: (f32, f32)) -> Option<(Headless, std::path::PathBuf)> {
    let ex = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
    let dir = std::env::temp_dir().join(format!("fg-cam-ui-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["flange.fg", "flange_cam.fg"] {
        std::fs::copy(ex.join(f), dir.join(f)).unwrap();
    }
    let file = dir.join("flange_cam.fg");
    if unplaced {
        let t = std::fs::read_to_string(&file).unwrap().replace(", at: (0, 0, -250)", "");
        assert!(!t.contains("at: ("));
        std::fs::write(&file, t).unwrap();
    }
    let mut h = match Headless::new(size, 1.0) {
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
    assert_eq!(w.run.as_ref().unwrap().certification, "full");
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
    // the sheet docks in the column a frame after it opens: its places settle first
    for _ in 0..4 {
        h.step();
    }
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
    let Some((mut h, file)) = open_unplaced("post") else { return };
    // the flange's setup is not placed (G5 unchecked): its program cannot be released, so Post
    // Process is off, the status says why, and its fix opens the setup
    h.step();
    let line = forge_render::shell::cam::status_line(h.gui.shell.cam.as_ref().unwrap());
    assert!(line.contains("not releasable: setup not placed") && !line.contains("ops certified"), "{line}");
    assert!(!line.contains("G5"), "no rule codes on the screen: {line}");
    let why = forge_render::shell::cam::release_block(h.gui.shell.cam.as_ref().unwrap()).expect("blocked");
    assert!(why.contains("at:"), "the syntax on hover: {why}");
    // one sentence for a person, and what to do (UX-LEAD); the code and syntax on hover only
    let b = forge_render::shell::cam::release_words(h.gui.shell.cam.as_ref().unwrap()).expect("blocked");
    assert_eq!(b.sentence, "Program prog can't be released: setup op1 isn't placed on the machine.");
    assert_eq!(b.next, "Place it to check machine travel, or allow uncertified output in NC Programs.");
    assert!(h.placed("cam:blocked").is_some(), "the sentence under Tools");
    // a refused export says the sentence on the status bar
    h.gui.shell.cam.as_mut().unwrap().command(forge_render::shell::cam::Cmd::ExportNc(None));
    h.step();
    let line = forge_render::shell::cam::status_line(h.gui.shell.cam.as_ref().unwrap());
    assert_eq!(line, b.sentence);
    tap(&mut h, "tool:Post Process");
    h.step();
    assert!(h.placed("cam:post:text").is_none(), "Post Process is off while not releasable");
    // the fix is seen whatever the words before it: never under the markup toggle
    let (fx, mk) = (h.gui.shell.place("status:fix").unwrap(), h.gui.shell.place("status:markup").unwrap());
    assert!(fx.right() <= mk.left(), "{fx:?} under {mk:?}");
    tap(&mut h, "status:fix");
    h.step();
    assert!(h.gui.shell.cam.as_ref().unwrap().setup_dlg.is_some(), "the fix opens the setup");
    h.gui.shell.cam.as_mut().unwrap().setup_dlg = None;
    // allowed uncertified: Post Process opens, writes the program stamped
    h.gui.shell.cam.as_mut().unwrap().allow_uncertified = true;
    h.step();
    tap(&mut h, "tool:Post Process");
    h.step();
    assert!(h.placed("cam:post:text").is_some());
    shot(&mut h, "cam-post");
    tap(&mut h, "cam:post:write");
    // the write lands a few frames on (sooner or later under a loaded machine)
    let t0 = std::time::Instant::now();
    while !file.with_file_name("prog.nc").exists() && t0.elapsed().as_secs() < 30 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
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
    let Some((mut h, file)) = open_unplaced("nc") else { return };
    tap(&mut h, "tool:NC Programs");
    h.step();
    shot(&mut h, "cam-nc-open");
    // a tab of the properties column: Quality beside it, back again
    // a tab's click can be taken by a frame that lays the column out again: clicked again until
    // the tab is open (a few tries)
    let quality = |g: &Gui| g.shell.cam.as_ref().is_some_and(|w| w.quality_open && !w.programs_open);
    for k in 0.. {
        tap(&mut h, "cam:dock:quality");
        for _ in 0..20 {
            if quality(&h.gui) {
                break;
            }
            h.step();
        }
        if quality(&h.gui) {
            break;
        }
        assert!(k < 4, "the Quality tab never opened");
    }
    shot(&mut h, "cam-quality-tab");
    tap(&mut h, "cam:dock:programs");
    h.step();
    h.step();
    tap(&mut h, "cam:programs:allow");
    h.step();
    // the file's name and folder, typed: written to the statement when the field is left
    tap(&mut h, "cam:programs:prog:file");
    h.text("flange-1");
    h.step();
    tap(&mut h, "cam:programs:prog:out");
    h.text("nc");
    h.step();
    tap(&mut h, "cam:programs:prog:tools");
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
    tap(&mut h, "cam:programs:prog:post");
    h.step();
    h.step();
    let nc = std::fs::read_to_string(file.with_file_name("nc").join("flange-1.nc")).expect("posted");
    assert!(nc.contains("(TOOLS)") && nc.contains("(NOT CERTIFIED IN FULL"), "{nc}");
    assert_eq!(st(&mut h)[0].1, Posted::Current);
    shot(&mut h, "cam-nc-programs");
    // the same file for Export…: the host's Save As is asked
    tap(&mut h, "cam:programs:prog:export");
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
    tap(&mut h, "cam:programs:repost");
    h.step();
    h.step();
    assert_eq!(st(&mut h)[0].1, Posted::Current);
    // the setup sheet and the tool list
    tap(&mut h, "cam:programs:sheet");
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
    assert!(h.click("cam:tools:search"));
    h.text("flat-6");
    h.step();
    let row = h
        .gui
        .shell
        .places
        .iter()
        .map(|p| p.0.clone())
        .find(|n| n.starts_with("cam:tools:row:flat-6"))
        .expect("a 6 mm flat in the library");
    // a row shows the tool (its cross-section); Add to document puts it in
    assert!(h.click(&row));
    h.step();
    assert!(h.placed("cam:tools:profile").is_some());
    shot(&mut h, "cam-tools-picked");
    assert!(!text(&h).contains("flat-6"));
    assert!(h.click("cam:tools:add"));
    h.step();
    assert!(text(&h).contains("tool(\"flat-6\", t: 2"), "{}", text(&h));
    h.step();
    // the document's tools first, with their numbers; the one just added is recent
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert_eq!(w.tools_recent.first().map(String::as_str), Some("flat-6"));
    // the type chip and the search narrow the list
    h.gui.shell.cam.as_mut().unwrap().tools_find = Default::default();
    h.step();
    assert!(h.click("cam:tools:kind:ball"));
    h.step();
    let rows = |h: &mut Headless| -> Vec<String> {
        h.step();
        h.gui.shell.places.iter().filter_map(|p| p.0.strip_prefix("cam:tools:row:").map(str::to_string)).collect()
    };
    let kind_of = |h: &Headless, id: &str| h.gui.shell.cam.as_ref().unwrap().lib.tools.iter().find(|t| t.id == id).map(|t| t.kind);
    let balls = rows(&mut h);
    assert!(!balls.is_empty() && balls.iter().all(|id| kind_of(&h, id) == Some(forge_tooling::Kind::Ball)), "{balls:?}");
    assert!(h.click("cam:tools:kind:all"));
    assert!(h.click("cam:tools:search"));
    h.text("chamfer");
    let ch = rows(&mut h);
    assert!(!ch.is_empty() && ch.iter().all(|id| kind_of(&h, id) == Some(forge_tooling::Kind::Chamfer)), "{ch:?}");
    h.gui.shell.cam.as_mut().unwrap().tools_find = Default::default();
    // in the document: its stick-out edited there
    assert!(h.click("cam:tools:row:flat-10"));
    h.step();
    shot(&mut h, "cam-tools-instance");
    assert!(h.click("cam:tools:inst:t1:stickout"));
    h.key_with(egui::Key::A, egui::Modifiers::COMMAND);
    h.text("40");
    h.step();
    assert!(h.click("cam:tools:apply"));
    h.step();
    assert!(text(&h).contains("stickout: 40"), "{}", text(&h));
    // derived: a longer flute, saved to the project's cam/tools.toml and shown
    assert!(h.click("cam:tools:derive"));
    h.step();
    shot(&mut h, "cam-tools-derive");
    assert!(h.click("cam:tools:form:lcf"));
    h.key_with(egui::Key::A, egui::Modifiers::COMMAND);
    h.text("40");
    h.step();
    assert!(h.click("cam:tools:save"));
    h.step();
    let w = h.gui.shell.cam.as_ref().unwrap();
    let saved = std::fs::read_to_string(w.dir.join("cam/tools.toml")).expect("cam/tools.toml written");
    assert!(saved.contains("id = \"flat-10-custom\"") && saved.contains("lcf = 40"), "{saved}");
    assert!(w.lib.tools.iter().any(|t| t.id == "flat-10-custom" && t.lcf == 40.0));
    assert_eq!(w.tools_sel.as_deref(), Some("flat-10-custom"));
    let _ = std::fs::remove_file(w.dir.join("cam/tools.toml"));
    // a page in place of the view and the column; Back (or Esc) returns to Manufacture
    let page = h.placed("cam:tools:page").expect("the library page");
    let view = h.gui.shell.view_rect;
    assert!(page.min.x <= view.min.x + 1.0 && page.max.x > view.max.x, "{page:?} over {view:?}");
    assert!(h.click("cam:tools:back"));
    h.step();
    assert!(!h.gui.shell.cam.as_ref().unwrap().tools_open);
    assert!(h.click("tool:Tool Library"));
    h.step();
    assert!(h.click("cam:tools:derive"));
    h.step();
    h.key(egui::Key::Escape);
    h.step();
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert!(w.tools_open && w.tools_form.is_none(), "Esc leaves the form first");
    h.key(egui::Key::Escape);
    h.step();
    assert!(!h.gui.shell.cam.as_ref().unwrap().tools_open);
}

/// The dialogs' parity arguments are the language's: a contour with axial stock, conventional
/// direction, finishing and spring passes, on bar stock, generates (no GPU needed).
#[test]
fn parity_arguments_the_dialogs_write_generate() {
    let ex = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
    let src = std::fs::read_to_string(ex.join("flange_cam.fg")).unwrap();
    let prof = src.lines().find(|l| l.starts_with("prof")).unwrap();
    let new = prof.replacen(
        "step: 4",
        "step: 4, leave_axial: 0.1, direction: \"conventional\", finish: 2, finish_step: 0.5, spring: 1",
        1,
    );
    let src = src.replace(prof, &new);
    let cam = forge_cam_lang::run(&src, ex);
    let op = cam.ops.iter().find(|o| o.name == "prof").expect("prof");
    assert!(op.refusal.is_none(), "{:?}", op.refusal);
    // CAM-2D-FINISH's: leads and tabs on the contour, and Bore in place of the pocket
    let lead = prof.replacen("step: 4", "step: 4, lead: \"arc\", tabs: 4, tab_height: 1", 1);
    let bore = src.lines().find(|l| l.starts_with("bore")).unwrap().to_string();
    let src2 = src
        .replace(&new, &lead)
        .replace(&bore, "bore = bore(op1, f.bore_h, tool: t1, finish_leave: 0.2, spring: 1)");
    let cam = forge_cam_lang::run(&src2, ex);
    for n in ["prof", "bore"] {
        let op = cam.ops.iter().find(|o| o.name == n).expect(n);
        assert!(op.refusal.is_none(), "{n}: {:?}", op.refusal);
    }
    // which op offers which field: every_dialog_field_is_an_argument_the_language_takes (ARGS)
}

/// Every milling form offers only arguments the language takes (forge-cam-lang's ARGS: any other
/// name is refused as UnknownArg), so a dialog cannot write what a run refuses.
#[test]
fn every_dialog_field_is_an_argument_the_language_takes() {
    let args = |f: &str| forge_cam_lang::ARGS.iter().find(|a| a.0 == f).map(|a| a.1);
    for s in forge_render::shell::cam::schema::OPS {
        let Some(known) = args(s.func) else { panic!("{} is not a built-in of forge-cam-lang", s.func) };
        for f in s.fields {
            assert!(known.contains(&f.arg), "{} offers `{}`, which the language refuses", s.func, f.arg);
        }
    }
    // the setup and program statements the dialogs write
    for (f, keys) in [
        ("setup", &["machine", "stock", "origin", "wcs", "tol", "clearance", "clear", "retract", "at", "fixture", "from"][..]),
        ("program", &["post", "number", "ops", "file", "ext", "out", "comment", "tools"][..]),
        ("cylinder", &["d", "margin", "below"][..]),
        ("tube", &["d", "id", "margin", "below"][..]),
    ] {
        let known = args(f).unwrap_or_else(|| panic!("no {f}"));
        for k in keys {
            assert!(known.contains(k), "{f} takes no `{k}`");
        }
    }
}

/// NC Programs: a program's ops, ticked: unticking one writes `ops:` (the others, in order), the
/// program posts without it; ticking it again takes `ops:` out.
#[test]
fn a_programs_ops_are_chosen_in_the_window() {
    let Some((mut h, _)) = open("ops") else { return };
    assert!(h.click("tool:NC Programs"));
    h.step();
    assert!(h.click("cam:programs:prog:op:bolts"));
    h.step();
    let t = text(&h);
    assert!(t.contains("ops: [skim, top, prof, bore]"), "{t}");
    settle(&mut h);
    let w = h.gui.shell.cam.as_ref().unwrap();
    let p = &w.run.as_ref().unwrap().programs[0];
    let nc = p.text.clone().unwrap_or_default();
    assert!(!nc.is_empty() && !nc.contains("T12"), "{:?} {nc}", p.error);
    assert_eq!(w.run.as_ref().unwrap().ops.iter().find(|o| o.name == "bolts").unwrap().status, "unposted");
    shot(&mut h, "cam-nc-ops");
    assert!(h.click("cam:programs:prog:op:bolts"));
    h.step();
    assert!(!text(&h).contains("ops:"), "{}", text(&h));
}

/// NC Programs: one file per setup (`split: "setup"`, a part per setup named `prog/op1`) and the
/// decimals typed; Post writes `{file}-{setup}`.
#[test]
fn a_split_program_posts_a_file_per_setup() {
    let Some((mut h, file)) = open("split") else { return };
    assert!(h.click("tool:NC Programs"));
    h.step();
    assert!(h.click("cam:programs:prog:split"));
    h.step();
    assert!(h.click("cam:programs:prog:decimals"));
    h.text("3");
    h.step();
    assert!(h.click("cam:programs:prog:file"));
    h.step();
    let t = text(&h);
    assert!(t.contains("split: \"setup\"") && t.contains("decimals: 3"), "{t}");
    settle(&mut h);
    let names: Vec<String> = h.gui.shell.cam.as_ref().unwrap().run.as_ref().unwrap().programs.iter().map(|p| p.name.clone()).collect();
    assert_eq!(names, ["prog/op1"]);
    assert!(h.click("cam:programs:prog:post"));
    h.step();
    h.step();
    let nc = std::fs::read_to_string(file.with_file_name("prog-op1.nc")).expect("posted per setup");
    assert!(nc.lines().count() > 50);
    shot(&mut h, "cam-nc-split");
}

/// A lathe document: no milling simulation beside it (it would refuse every turning built-in);
/// the section view docked at the view's bottom right, clear of a floating window; and the
/// floating windows one at a time.
#[test]
fn a_turning_document_docks_its_section_and_windows_take_turns() {
    let ex = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
    let dir = std::env::temp_dir().join(format!("fg-cam-ui-{}-turn", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["shaft.fg", "shaft_turn.fg"] {
        std::fs::copy(ex.join(f), dir.join(f)).unwrap();
    }
    let mut h = match Headless::new((1500.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    h.gui.open(dir.join("shaft_turn.fg").to_str().unwrap());
    let t = std::time::Instant::now();
    while t.elapsed().as_secs() < 120 && !h.until(generated) {}
    assert!(generated(&h.gui), "never generated: {}", h.gui.shell.status);
    for _ in 0..6 {
        h.step();
    }
    assert!(h.gui.shell.camsim.is_none(), "no milling simulation of a lathe document");
    // the Section window covers the view's bottom right: the part framed in what it leaves
    // (UX-LEAD), not small behind it
    {
        let w = h.gui.shell.cam.as_ref().unwrap();
        let sec = w.section_at.expect("the Section window open");
        assert!(w.view_side != [0.0, 0.0], "framed around the Section window");
        let vr = h.gui.shell.view_rect;
        let cam = w.framed.expect("framed");
        // the subject as the re-frame takes it: the shown box, else the scene's bounds
        let scene = &h.gui.view.as_ref().unwrap().scene;
        let b = match scene.shown_box() {
            Some((lo, hi)) => forge_render::frame::Bounds::new(lo, hi),
            None => {
                let (c, r) = scene.bounds();
                forge_render::frame::Bounds::new([c[0] - r, c[1] - r, c[2] - r], [c[0] + r, c[1] + r, c[2] + r])
            }
        };
        let px: Vec<(f32, f32)> = b
            .corners()
            .iter()
            .filter_map(|c| cam.pixel_of(*c, vr.width() as u32, vr.height() as u32))
            .map(|(x, y)| (vr.left() + x, vr.top() + y))
            .collect();
        let part = egui::Rect::from_points(&px.iter().map(|p| egui::pos2(p.0, p.1)).collect::<Vec<_>>());
        let free = if w.view_side[0] > 0.0 {
            egui::Rect::from_min_max(vr.min, egui::pos2(vr.right() - w.view_side[0], vr.bottom() - w.view_cover))
        } else {
            egui::Rect::from_min_max(vr.min, egui::pos2(vr.right(), vr.bottom() - w.view_cover - w.view_side[1]))
        };
        assert!(!part.shrink(2.0).intersects(sec), "the part {part:?} behind the Section window {sec:?}");
        let fill = (part.width() / free.width()).max(part.height() / free.height());
        assert!(fill > 0.55, "the part fills {fill:.2} of what is left");
    }
    shot(&mut h, "cam-turn-section-framed");
    {
        // the part as drawn (not only where the camera says): its light grey pixels in the view,
        // clear of the Section window. A view change ended in one slow frame was left undrawn.
        let px = h.gui_pixels();
        let sec = h.gui.shell.cam.as_ref().unwrap().section_at.expect("the Section window open");
        let vr = h.gui.shell.view_rect;
        let (mut x0, mut x1) = (f32::MAX, f32::MIN);
        for y in (vr.top() as usize + 40..vr.bottom() as usize - 10).step_by(4) {
            for x in (vr.left() as usize + 5..sec.left() as usize - 4).step_by(2) {
                let c = px.at(x, y);
                if c.iter().all(|&v| v > 175) {
                    x0 = x0.min(x as f32);
                    x1 = x1.max(x as f32);
                }
            }
        }
        // centred in what the window leaves (the free rect's centre), not in the whole view
        assert!(x0 < x1, "the part drawn");
        let (mid, free_mid) = ((x0 + x1) * 0.5, (vr.left() + sec.left()) * 0.5);
        assert!(
            (mid - free_mid).abs() < 0.15 * (sec.left() - vr.left()),
            "the part drawn {x0}..{x1}, centred at {mid}; what is left's centre is {free_mid}"
        );
    }
    assert!(h.click("tool:NC Programs"));
    h.step();
    h.step();
    shot(&mut h, "cam-turn-programs");
    // the two windows' extents, from every place each puts in it
    let span = |h: &Headless, pre: &str| {
        h.gui.shell.places.iter().filter(|p| p.0.starts_with(pre)).map(|p| p.1).reduce(|a, b| a.union(b))
    };
    let sec = span(&h, "cam:turn").expect("the section view");
    let progs = span(&h, "cam:programs").expect("NC Programs");
    assert!(!sec.intersects(progs), "section {sec:?} under NC Programs {progs:?}");
    assert!(h.click("tool:Tool Library"));
    h.step();
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert!(w.tools_open && !w.programs_open, "one floating window at a time");
}

/// The op dialog's keys: Esc cancels (the text unchanged), Enter is OK (the edit written).
#[test]
fn the_op_dialog_answers_enter_and_escape() {
    use forge_render::shell::cam::schema::{Tab, spec, tabs};
    let Some((mut h, _)) = open("keys") else { return };
    let before = text(&h);
    h.gui.shell.cam.as_mut().unwrap().auto = false;
    h.gui.shell.cam.as_mut().unwrap().command(forge_render::shell::cam::Cmd::Edit("prof".into()));
    h.step();
    assert!(h.placed("cam:op:ok").is_some());
    h.key(egui::Key::Escape);
    h.step();
    assert!(h.gui.shell.cam.as_ref().unwrap().op_dlg.is_none(), "Esc closes it");
    assert_eq!(text(&h), before);
    h.gui.shell.cam.as_mut().unwrap().command(forge_render::shell::cam::Cmd::Edit("prof".into()));
    h.step();
    let k = tabs(spec("contour").unwrap()).iter().position(|t| *t == Tab::Passes).unwrap();
    assert!(h.click(&format!("cam:op:tab:{k}")));
    h.step();
    assert!(h.click("cam:op:arg:leave"));
    h.text("0.3");
    h.step();
    h.key(egui::Key::Enter);
    h.step();
    h.step();
    assert!(text(&h).contains("leave: 0.3"), "Enter is OK: {}", text(&h));
    assert!(h.gui.shell.cam.as_ref().unwrap().op_dlg.is_none());
}

/// The op dialog docks in the Manufacture environment's properties column (no floating window
/// over the view): its OK inside the column, the view narrowed by it.
#[test]
fn the_op_dialog_docks_in_the_properties_column() {
    let Some((mut h, _)) = open("dock") else { return };
    let view_before = h.gui.shell.view_rect;
    h.gui.shell.cam.as_mut().unwrap().command(forge_render::shell::cam::Cmd::Edit("prof".into()));
    for _ in 0..3 {
        h.step();
    }
    let col = h.placed("env:manufacture:properties").expect("the properties column");
    let ok = h.placed("cam:op:ok").expect("the op dialog");
    assert!(col.contains_rect(ok), "OK {ok:?} outside the column {col:?}");
    assert!(h.gui.shell.view_rect.right() <= col.left() + 1.0, "the view gives up the column");
    assert!(h.gui.shell.view_rect.width() <= view_before.width());
}

/// A narrow window (under 1280 px): the column shows the open sheet or the simulation, as tabs,
/// the sheet's first; choosing Simulation keeps the dialog open; its commit buttons are pinned
/// in the column; the view is framed again when the column narrows it.
#[test]
fn a_narrow_window_tabs_the_sheet_and_the_simulation() {
    let Some((mut h, _)) = open_sized("narrow", false, (1100.0, 800.0)) else { return };
    for _ in 0..20 {
        h.step();
    }
    let before = h.gui.view.as_ref().unwrap().nav.camera;
    let w0 = h.gui.shell.view_rect.width();
    h.gui.shell.cam.as_mut().unwrap().command(forge_render::shell::cam::Cmd::Edit("prof".into()));
    for _ in 0..30 {
        h.step();
    }
    let has_sim = h.gui.shell.camsim.is_some();
    let col = h.placed("env:manufacture:properties").expect("the column");
    let ok = h.placed("cam:op:ok").expect("OK in sight");
    assert!(col.contains_rect(ok), "OK {ok:?} in the column {col:?}");
    shot(&mut h, "cam-narrow-sheet");
    if has_sim {
        assert!(h.click("cam:dock:sim"), "the Simulation tab");
        h.step();
        h.step();
        let w = h.gui.shell.cam.as_ref().unwrap();
        assert!(w.op_dlg.is_some(), "the sheet kept open behind its tab");
        assert!(h.placed("cam:op:ok").is_none(), "not drawn");
        assert!(h.click("cam:dock:sheet"));
        h.step();
        h.step();
        assert!(h.placed("cam:op:ok").is_some());
    }
    // the column came only if it was not there before: then the view was framed again
    if h.gui.shell.view_rect.width() < w0 - 1.0 {
        assert_ne!(h.gui.view.as_ref().unwrap().nav.camera, before, "framed again");
    }
}

/// A run's quality is measured off the UI thread after the run is shown; an older run's measure
/// can end after the shown one's. The shown run takes its own whichever order they land in (one
/// slot let the older overwrite it: the run never had its quality, and a wait on it never ended).
#[test]
fn a_late_measure_of_an_older_text_does_not_lose_the_shown_ones() {
    let Some((mut h, _)) = open("measure-order") else { return };
    let w = h.gui.shell.cam.as_mut().unwrap();
    let r = w.run.as_mut().unwrap();
    let q = r.quality.take().expect("measured");
    let text = r.text.clone();
    {
        let mut slot = w.quality_job.lock().unwrap();
        slot.push((text, q.clone()));
        // the older text's, ending after
        slot.push(("an older text".into(), q));
    }
    h.step();
    let r = h.gui.shell.cam.as_ref().unwrap().run.as_ref().unwrap();
    assert!(r.quality.is_some(), "the shown run's own measure taken");
}

/// Export NC on a certified document answers the click at once, in every host (BUTTON-AUDITOR:
/// headless there is no save dialog, and nothing changed): the status asks where, and the export
/// is handed to the host.
#[test]
fn export_nc_answers_its_click() {
    let Some((mut h, _)) = open("export-click") else { return };
    assert!(forge_render::shell::cam::release_block(h.gui.shell.cam.as_ref().unwrap()).is_none(), "the flange is placed");
    tap(&mut h, "tool:Export NC");
    h.step();
    let w = h.gui.shell.cam.as_ref().unwrap();
    let line = forge_render::shell::cam::status_line(w);
    assert_eq!(line, "Export NC: choose where to save prog.nc\u{2026}");
    assert!(w.export.as_ref().is_some_and(|x| x.files.len() == 1), "handed to the host's Save As");
}

/// A host that does not answer an export (no dialog showing, or one the user has not seen): 10 s
/// on, the line says it is still waiting, never that none opened (the app cannot know), with Try
/// again; Try again asks again; the host's answer ends the wait.
#[test]
fn an_unanswered_export_says_it_is_waiting_and_asks_again() {
    use forge_render::shell::cam::nc::{EXPORT_WAIT_MS, RETRY_TIP};
    let Some((mut h, _)) = open("export-wait") else { return };
    tap(&mut h, "tool:Export NC");
    h.step();
    // headless, nothing takes the export: as a host that never answers. 10 s on:
    let w = h.gui.shell.cam.as_mut().unwrap();
    w.export = None;
    w.export_asked.as_mut().expect("asked").at_ms -= EXPORT_WAIT_MS + 1.0;
    h.step();
    h.step();
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert_eq!(forge_render::shell::cam::status_line(w), "Export NC: waiting for the save dialog");
    assert_eq!(h.gui.shell.status, "Export NC: waiting for the save dialog", "the shell's status too");
    assert_eq!(forge_render::shell::cam::status_fix_tip(w), Some(RETRY_TIP));
    // Try again: asked again, the click's words again
    tap(&mut h, "status:fix");
    h.step();
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert!(w.export.is_some(), "asked of the host again");
    assert_eq!(forge_render::shell::cam::status_line(w), "Export NC: choose where to save prog.nc\u{2026}");
    assert!(w.export_asked.as_ref().is_some_and(|a| !a.waiting));
    // the host answers: the wait ends, its words stand
    h.gui.shell.cam.as_mut().unwrap().said = Some("Export cancelled".into());
    h.step();
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert!(w.export_asked.is_none(), "no longer waiting");
    assert_eq!(forge_render::shell::cam::status_line(w), "Export cancelled");
}

/// A phone-wide window (480 px: a view about 200 px wide beside the browser): Manufacture opens
/// and draws with its simulation (BUTTON-AUDITOR, bracket_cam: the timeline's width went negative
/// and egui panicked, in a debug build), the timeline folded away where its controls do not fit.
#[test]
fn manufacture_opens_in_a_phone_wide_window() {
    let ex = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
    let dir = std::env::temp_dir().join(format!("fg-cam-ui-{}-narrow", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["bracket.fg", "bracket_cam.fg"] {
        std::fs::copy(ex.join(f), dir.join(f)).unwrap();
    }
    let mut h = match Headless::new((480.0, 860.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    h.gui.open(dir.join("bracket_cam.fg").to_str().unwrap());
    settle(&mut h);
    // the simulation comes after the run: drawn a few frames with it
    let t0 = std::time::Instant::now();
    while h.gui.shell.camsim.is_none() {
        assert!(t0.elapsed().as_secs() < 60, "no simulation");
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    for _ in 0..10 {
        h.step();
    }
    assert!(h.gui.shell.view_rect.width() < 300.0, "a narrow view: {:?}", h.gui.shell.view_rect);
    shot(&mut h, "cam-480");
}

/// A cycle time is the lower bound and the estimate together (never the estimate alone), and the
/// estimate counts the tool change the bound counts (the flange changes T1 for T12).
#[test]
fn cycle_time_is_the_bound_and_the_estimate_together() {
    let Some((h, _)) = open("cycle") else { return };
    let w = h.gui.shell.cam.as_ref().unwrap();
    let r = w.run.as_ref().unwrap();
    let q = r.quality.as_ref().expect("measured");
    let p = q.progs.first().expect("a timed program");
    assert!(p.error.is_none(), "{:?}", p.error);
    assert!(p.tool_changes > 0, "the flange changes tools");
    assert!(p.t_min > 0.0 && p.t_scurve >= p.t_min, "t_min {} t_scurve {}", p.t_min, p.t_scurve);
    let (lo, est) = r.cycle(r.ops.iter().map(|o| o.name.as_str())).expect("timed");
    assert!(est >= lo);
    for o in &q.ops {
        assert!(o.t_scurve + 1e-9 >= o.t_min, "{}: t_min {} t_scurve {}", o.name, o.t_min, o.t_scurve);
    }
    let line = forge_render::shell::cam::status_line(w);
    assert!(line.contains('\u{2265}') && line.contains("est."), "{line}");
    let words = forge_render::shell::cam::cycle_words((lo, est));
    assert!(words.starts_with('\u{2265}') && words.contains(" est. "), "{words}");
}

/// Adaptive is opt-in: a small button after the 2D group's three big ones (never Clear's place),
/// and the fields its form writes generate.
#[test]
fn adaptive_is_an_opt_in_op_whose_fields_generate() {
    use forge_render::shell::cam::schema::OPS;
    let two_d: Vec<&str> = OPS.iter().filter(|o| o.group == "2D").map(|o| o.func).collect();
    let at = two_d.iter().position(|f| *f == "adaptive").expect("adaptive in 2D");
    assert!(at >= 3, "adaptive is a big (default-looking) button: {two_d:?}");
    assert_eq!(at, two_d.len() - 1, "adaptive is not last of 2D: {two_d:?}");
    let ex = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
    let src = std::fs::read_to_string(ex.join("flange_cam.fg")).unwrap();
    let top = src.lines().find(|l| l.starts_with("top")).unwrap();
    let src = src.replace(
        top,
        "top = adaptive(op1, tool: t1, to: 12, step: 4, engagement: 50, passes: \"two_way\", sweep: 90, leave: 0.2)",
    );
    let cam = forge_cam_lang::run(&src, ex);
    let op = cam.ops.iter().find(|o| o.name == "top").expect("top");
    assert!(op.refusal.is_none(), "{:?}", op.refusal);
    assert!(op.moves > 0, "adaptive made no moves");
}

/// Suppressing an op generates again (Generate as I edit): the ops after it are made, not left
/// stale.
#[test]
fn suppressing_an_op_generates_again() {
    use forge_render::shell::cam::Cmd;
    let Some((mut h, _)) = open("suppress") else { return };
    h.gui.shell.cam.as_mut().unwrap().command(Cmd::Suppress("top".into()));
    settle(&mut h);
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert_eq!(w.status("top"), "suppressed");
    // made again (bolts may be refused: its drill now meets the stock top would have cleared)
    for op in ["prof", "bore", "bolts"] {
        assert!(["built", "refused"].contains(&w.status(op)), "{op}: {}", w.status(op));
    }
    assert_eq!(w.status("prof"), "built");
}

/// The mould's 3D ops (zlevel, parallel, waterline, pencil, rest) are ops of the workspace: in
/// the browser with their rows, in the ribbon's 3D group, each form's fields ones the language
/// takes.
#[test]
fn a_mould_lists_its_3d_ops() {
    let ex = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
    let dir = std::env::temp_dir().join(format!("fg-cam-ui-{}-mould", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["mold_core.fg", "mold_core_cam.fg"] {
        std::fs::copy(ex.join(f), dir.join(f)).unwrap();
    }
    let text = std::fs::read_to_string(dir.join("mold_core_cam.fg")).unwrap();
    let w = forge_render::shell::cam::CamWs::open(dir.join("mold_core_cam.fg").to_str().unwrap(), &text);
    let ops: Vec<(String, String)> = w
        .stmts
        .iter()
        .filter(|s| s.is_op())
        .map(|s| (s.name.clone(), s.func.clone()))
        .collect();
    for (name, func) in [("rough", "zlevel"), ("fin", "parallel"), ("walls", "waterline"), ("corner", "pencil"), ("left", "rest")] {
        assert!(ops.contains(&(name.into(), func.into())), "{name} ({func}) not an op: {ops:?}");
        let spec = forge_render::shell::cam::schema::spec(func).unwrap_or_else(|| panic!("no form for {func}"));
        assert_eq!(spec.group, "3D");
    }
    let Some((mut h, _)) = headless_open(&dir.join("mold_core_cam.fg")) else { return };
    for (name, _) in &ops {
        assert!(h.placed(&format!("cam:row:{name}")).is_some(), "{name} row");
    }
    for tool in ["3D Rough", "Parallel"] {
        assert!(h.placed(&format!("tool:{tool}")).is_some(), "{tool} in the ribbon");
    }
}

fn headless_open(file: &std::path::Path) -> Option<(Headless, std::path::PathBuf)> {
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
    Some((h, file.to_path_buf()))
}

/// Checks (the ribbon) opens the simulation's own Checks tab, which carries each op's verdict;
/// no window floats over the part.
#[test]
fn checks_open_in_the_simulations_tab() {
    let Some((mut h, _)) = open("checks") else { return };
    // the simulation is built after the run
    let t0 = std::time::Instant::now();
    while h.gui.shell.camsim.is_none() && t0.elapsed().as_secs() < 120 {
        h.until(|g| g.shell.camsim.is_some());
    }
    assert!(h.gui.shell.camsim.is_some(), "no simulation");
    assert!(h.click("tool:Checks"));
    h.step();
    h.step();
    let st = h.gui.shell.camsim.as_deref().unwrap();
    assert_eq!(st.tab, forge_render::gui_camsim_panel::Tab::Checks);
    let names: Vec<&str> = st.op_checks.iter().map(|o| o.0.as_str()).collect();
    assert_eq!(st.op_checks.len(), 5, "{names:?}");
    assert!(st.op_checks.iter().all(|o| o.1 == "certified"), "{:?}", st.op_checks);
    assert!(!h.gui.shell.cam.as_ref().unwrap().checks_open);
    shot(&mut h, "cam-checks-tab");
}

/// Document Text is a panel across the bottom of the view (above the timeline), not a window
/// over the part; Apply writes what was typed.
#[test]
fn document_text_is_a_bottom_panel() {
    let Some((mut h, _)) = open("text") else { return };
    assert!(h.click("tool:Document Text"));
    h.step();
    h.step();
    let v = h.gui.shell.view_rect;
    let e = h.placed("cam:text:edit").expect("the text");
    assert!(e.min.y > v.center().y - 40.0 && e.width() > v.width() * 0.6, "{e:?} in {v:?}");
    // the view framed again into what the panel leaves
    for _ in 0..3 {
        h.step();
    }
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert!(w.view_cover >= forge_render::shell::cam::text_panel_h(v.height() * 0.5), "cover {}", w.view_cover);
    shot(&mut h, "cam-text-panel");
    // typed: the whole text replaced
    let typed = text(&h).replace("stickout: 32", "stickout: 34");
    assert!(h.click("cam:text:edit"));
    h.key_with(egui::Key::A, egui::Modifiers::COMMAND);
    h.text(&typed);
    h.step();
    assert!(h.click("cam:text:apply"));
    h.step();
    assert!(text(&h).contains("stickout: 34"), "{}", text(&h));
}

/// An adaptive op selected: clear's time on the same setup, tool and heights is measured off the
/// UI thread and shown beside its own (TOOLPATH-QUALITY's alternative_clear).
#[test]
fn an_adaptive_op_shows_what_clear_would_take() {
    use forge_render::shell::cam::Cmd;
    let Some((mut h, _)) = open("altclear") else { return };
    let text = text(&h);
    let top = text.lines().find(|l| l.starts_with("top")).unwrap().to_string();
    let w = h.gui.shell.cam.as_mut().unwrap();
    w.set_text(text.replace(&top, "top  = adaptive(op1, tool: t1, from: 18, to: 12, step: 3)"));
    settle(&mut h);
    h.gui.shell.cam.as_mut().unwrap().command(Cmd::Select("top".into()));
    let t0 = std::time::Instant::now();
    while h.gui.shell.cam.as_ref().unwrap().alt.is_none() && t0.elapsed().as_secs() < 120 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let w = h.gui.shell.cam.as_ref().unwrap();
    let (op, _, got) = w.alt.as_ref().expect("clear's time measured");
    assert_eq!(op, "top");
    let (lo, est) = got.expect("clear builds on the flange");
    assert!(lo > 0.0 && est >= lo);
    // the row says whose time and of what (UX-LEAD): "≥ … · est. … · N% less time"
    let own = w.run.as_ref().unwrap().cycle(["top"]).expect("adaptive's own time");
    let words = forge_render::shell::cam::alt_clear_words((lo, est), own);
    let pct = ((est / own.1 - 1.0) * 100.0).abs();
    assert!(words.ends_with(&format!("{pct:.0}% {} time", if est < own.1 { "less" } else { "more" })), "{words}");
    assert!(words.starts_with('\u{2265}'), "{words}");
    shot(&mut h, "cam-adaptive-vs-clear");
}

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
    open_with(tag, false)
}

/// The same with the setup not placed (no `at:`): its machine positions unchecked (G5).
fn open_unplaced(tag: &str) -> Option<(Headless, std::path::PathBuf)> {
    open_with(tag, true)
}

fn open_with(tag: &str, unplaced: bool) -> Option<(Headless, std::path::PathBuf)> {
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
    let Some((mut h, file)) = open_unplaced("post") else { return };
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
    let Some((mut h, file)) = open_unplaced("nc") else { return };
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

/// Probing (PROBING's built-ins): a probe among the tools, Probe WCS and Inspect in the ribbon's
/// Probing group, their dialogs' fields, and the measurement in the op's details.
#[test]
fn probing_ops_have_a_ribbon_group_dialogs_and_details() {
    let ex = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
    let dir = std::env::temp_dir().join(format!("fg-cam-ui-{}-probe", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(ex.join("flange.fg"), dir.join("flange.fg")).unwrap();
    let src = std::fs::read_to_string(ex.join("flange_cam.fg"))
        .unwrap()
        .replace(
            "op1  = setup(",
            "p    = probe(\"renishaw-omp40-2\", t: 20, ball: 6, length: 50, stem: 3, body: [(20, 0, 69)])\nop1  = setup(",
        )
        .replace(
            "prog = program(op1, post: \"fanuc\", number: 1001)",
            "z0 = probe_wcs(op1, tool: p, kind: \"surface\", at: (25, 25, 20), wcs: 54)\nprog = program(op1, post: \"linuxcnc\", number: 1001)",
        );
    let file = dir.join("flange_cam.fg");
    std::fs::write(&file, &src).unwrap();
    let mut h = match Headless::new((1500.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    settle(&mut h);
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert_eq!(w.status("z0"), "built", "{:?}", w.run.as_ref().unwrap().errors);
    let probe = w.run.as_ref().unwrap().op("z0").and_then(|o| o.probe.clone()).expect("a measurement");
    assert_eq!(probe.measures, "z");
    assert!(h.placed("tool:Probe WCS").is_some() && h.placed("tool:Inspect").is_some(), "the Probing group");
    assert!(h.placed("cam:tool:p").is_some(), "the probe among the tools");
    assert!(h.click("cam:row:z0"));
    h.step();
    shot(&mut h, "cam-probe-details");
    h.gui.shell.cam.as_mut().unwrap().command(forge_render::shell::cam::Cmd::Edit("z0".into()));
    h.step();
    let tabs = forge_render::shell::cam::schema::tabs(forge_render::shell::cam::schema::spec("probe_wcs").unwrap());
    let k = tabs.iter().position(|t| *t == forge_render::shell::cam::schema::Tab::Geometry).unwrap();
    assert!(h.click(&format!("cam:op:tab:{k}")));
    h.step();
    assert!(h.placed("cam:op:arg:kind").is_some() && h.placed("cam:op:arg:wcs").is_some());
    shot(&mut h, "cam-probe-dialog");
}

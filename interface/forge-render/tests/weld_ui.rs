//! Weldments in the app: the Weldments tab and its tools, members made through their dialog
//! (one AddFeature, the same op an agent sends), the Cut List panel grouped as the `cutlist`
//! query has it, and its CSV written beside the part. `FORGE_UI_SHOTS=DIR` keeps pictures.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::{FgCmd, WELD_TAB};

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn settle(h: &mut Headless) {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 30.0 {
        if h.until(ready) {
            break;
        }
    }
    assert!(ready(&h.gui), "never ready: {}", h.gui.shell.status);
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

fn open(text: &str, tag: &str) -> Option<(Headless, std::path::PathBuf)> {
    let dir = std::env::temp_dir().join(format!("fg-weld-ui-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("frame.fg");
    std::fs::write(&file, text).unwrap();
    let _ = std::fs::remove_file(forge_lang::undo::log_path(&file));
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

const LAYOUT: &str = "shared W = 600
shared D = 400
sketch lay(on: xy) {
  r = rect(corner: (0, 0), size: (W, D))
}
";

fn source(h: &Headless) -> String {
    h.gui.shell.fg.as_ref().unwrap().source.clone()
}

#[test]
fn members_are_made_through_their_dialog_and_listed_in_the_cut_list() {
    let Some((mut h, file)) = open(LAYOUT, "members") else {
        return;
    };
    // the tab and its tools
    assert!(h.click("tab:Weldments"), "no Weldments tab");
    h.step();
    h.step();
    assert_eq!(h.gui.shell.tab, WELD_TAB);
    for tool in ["Members", "End Trim", "End Cap", "Corner Plate", "Cut List"] {
        assert!(h.gui.shell.place(&format!("tool:{tool}")).is_some(), "no {tool}");
    }
    // Members: the layout picked, the section from the library, miter corners: OK
    assert!(h.click("tool:Members"));
    assert!(
        h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some())),
        "no dialog: {}",
        h.gui.shell.status
    );
    {
        let t = h.gui.shell.fg.as_mut().unwrap().tool.as_mut().unwrap();
        assert_eq!(t.kind, "members");
        // the section list is the library's
        let sections = t.options_of("section").expect("a section list");
        assert!(sections.iter().any(|s| s == "SHS 40x40x3"));
        assert!(sections.iter().any(|s| s.starts_with("IPE ")));
        t.take(Ok("lay.r".into()));
        t.words.iter_mut().find(|w| w.0 == "section").unwrap().1 = "SHS 30x30x3".into();
    }
    settle(&mut h);
    shot(&mut h, "weld-1-members-dialog");
    assert!(h.click("fg:tool:ok"));
    h.step();
    settle(&mut h);
    let src = source(&h);
    assert!(
        src.contains("frame1 = members(lay.r, section: \"SHS 30x30x3\")"),
        "{src}"
    );
    assert!(h.gui.shell.fg.as_ref().unwrap().tool.is_none());
    // the cut list: two rows of two (the long and the short sides), mitered both ends
    let list = h.gui.shell.fg.as_ref().unwrap().cutlist.clone();
    assert_eq!(list.lines.len(), 2, "{list:?}");
    assert!(
        list.lines
            .iter()
            .all(|l| l.qty == 2 && l.section == "SHS 30x30x3")
    );
    assert!(
        list.lines
            .iter()
            .all(|l| l.ends[0] == "45 deg" && l.ends[1] == "45 deg"),
        "{list:?}"
    );
    // the panel, and its CSV beside the part
    h.gui
        .shell
        .out
        .push(forge_render::shell::Action::Fg(FgCmd::CutList));
    for _ in 0..4 {
        h.step();
    }
    assert!(h.gui.shell.place("weld:cutlist:row:1").is_some());
    shot(&mut h, "weld-2-cutlist");
    assert!(h.click("weld:cutlist:export"));
    h.step();
    let csv = std::fs::read_to_string(file.with_file_name("frame-cutlist.csv")).expect("the CSV");
    assert!(csv.starts_with("item,qty,section"), "{csv}");
    assert_eq!(
        csv.lines().filter(|l| l.contains("SHS 30x30x3")).count(),
        2,
        "{csv}"
    );
    assert!(h.click("weld:cutlist:close"));
    h.step();
    assert!(!h.gui.shell.fg.as_ref().unwrap().cutlist_open);
}

#[test]
fn an_end_cap_and_a_corner_plate_take_members_from_the_browser() {
    let text = format!("{LAYOUT}fr = members(lay.r, section: \"SHS 40x40x3\")\n");
    let Some((mut h, _)) = open(&text, "plates") else {
        return;
    };
    assert!(h.click("tab:Weldments"));
    h.step();
    // a member row in the browser is what a member slot takes
    assert!(
        h.gui
            .shell
            .fg
            .as_ref()
            .unwrap()
            .feats
            .iter()
            .any(|f| f.id == "fr[0]" && f.kind == "member"),
        "{:?}",
        h.gui
            .shell
            .fg
            .as_ref()
            .unwrap()
            .feats
            .iter()
            .map(|f| (&f.id, &f.kind))
            .collect::<Vec<_>>()
    );
    assert!(h.click("tool:Corner Plate"));
    assert!(h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some())));
    {
        let t = h.gui.shell.fg.as_mut().unwrap().tool.as_mut().unwrap();
        t.take(Ok("fr[0]".into()));
        t.take(Ok("fr[1]".into()));
    }
    settle(&mut h);
    shot(&mut h, "weld-3-corner-plate");
    assert!(h.click("fg:tool:ok"));
    h.step();
    settle(&mut h);
    let src = source(&h);
    assert!(
        src.contains("plate1 = corner_plate(fr[0], fr[1], size: size, t: t, chamfer: corner_cut)"),
        "{src}"
    );
    let list = h.gui.shell.fg.as_ref().unwrap().cutlist.clone();
    assert_eq!(list.plates.len(), 1, "{list:?}");
    assert_eq!(list.plates[0].on, vec!["fr[0]".to_string(), "fr[1]".to_string()]);
}

#[test]
fn the_table_frame_example_lists_its_members_and_plates() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/weldments/table_frame.fg"
    ))
    .unwrap();
    let Some((mut h, _)) = open(&src, "table") else {
        return;
    };
    assert!(h.click("tab:Weldments"));
    h.gui
        .shell
        .out
        .push(forge_render::shell::Action::Fg(FgCmd::CutList));
    for _ in 0..4 {
        h.step();
    }
    let list = h.gui.shell.fg.as_ref().unwrap().cutlist.clone();
    // rails long and short, the four legs alike; four feet and two corner plates
    assert_eq!(list.lines.iter().map(|l| l.qty).sum::<usize>(), 8, "{list:?}");
    assert!(list.lines.iter().any(|l| l.qty == 4), "{list:?}");
    assert_eq!(list.plates.len(), 6, "{list:?}");
    assert!(h.gui.shell.place("weld:cutlist:row:0").is_some());
    shot(&mut h, "weld-0-table-frame");
    // the legs' row lights the four legs, and only them; again, off
    let k = list.lines.iter().position(|l| l.qty == 4).unwrap();
    assert!(h.gui.fg_lit_faces().is_empty());
    assert!(h.click(&format!("weld:cutlist:row:{k}")));
    for _ in 0..3 {
        h.step();
    }
    let lit = h.gui.fg_lit_faces();
    let a = h.gui.fg_anchors.as_ref().unwrap();
    let mut legs: Vec<u32> = list.lines[k]
        .members
        .iter()
        .flat_map(|m| {
            a.role_faces
                .get(&format!("{m}.faces"))
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    legs.sort_unstable();
    legs.dedup();
    assert!(!lit.is_empty());
    assert_eq!(lit, legs);
    shot(&mut h, "weld-5-cutlist-lit");
    assert!(h.click(&format!("weld:cutlist:row:{k}")));
    for _ in 0..3 {
        h.step();
    }
    assert!(h.gui.fg_lit_faces().is_empty());
}

/// Clicks where this place in the world is on screen, once the view knows what's under it.
fn point_at(h: &mut Headless, world: [f32; 3]) {
    assert!(h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving())));
    let (cam, size) = h.gui.shell.camera.unwrap();
    let on = cam.pixel_of(world, size.0, size.1).unwrap();
    let r = h.gui.shell.view_rect;
    h.move_to(r.min.x + on.0, r.min.y + on.1);
    assert!(h.settle_hover(), "no pick: {}", h.gui.shell.status);
    h.button(true);
    h.step();
    h.button(false);
    for _ in 0..10 {
        h.step();
    }
}

#[test]
fn a_member_is_picked_by_a_click_on_it_and_a_section_by_family_then_size() {
    let text = format!("{LAYOUT}fr = members(lay.r, section: \"SHS 40x40x3\")\n");
    let Some((mut h, _)) = open(&text, "viewpick") else {
        return;
    };
    assert!(h.click("tab:Weldments"));
    h.step();
    assert!(h.click("tool:End Cap"));
    assert!(h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some())));
    settle(&mut h);
    // the outer side of the first rail, half way along: fr[0]
    point_at(&mut h, [300.0, -20.0, 0.0]);
    let t = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap();
    let a = h.gui.fg_anchors.as_ref().unwrap();
    let named: Vec<&String> = a.faces.iter().flatten().take(6).collect();
    assert_eq!(
        t.slots[0].picks,
        vec!["fr[0]".to_string()],
        "{:?} hover {:?} faces named {}/{} e.g. {named:?}",
        t.error,
        h.gui.shell.status,
        a.faces.iter().flatten().count(),
        a.faces.len()
    );
    h.key(egui::Key::Escape);
    for _ in 0..3 {
        h.step();
    }
    // Members: its section chosen by family, then size
    assert!(h.click("tool:Members"));
    assert!(h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some())));
    for _ in 0..4 {
        h.step();
    }
    assert!(h.gui.shell.place("fg:tool:word:section:family").is_some());
    assert!(h.gui.shell.place("fg:tool:word:section").is_some());
    shot(&mut h, "weld-4-section-picker");
}

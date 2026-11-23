//! Sheet-metal vendors in the app: the Setup group's Vendor window and the DFM panel. A part on
//! SendCutSend's 5052 with a hole too near its bend: the panel checks it off the window's thread
//! and lists the violation; choosing it lights the hole's faces and marks it on the flat. The
//! Vendor window's choice is written into a plain sheet as `vendor:`, `material:` and its
//! thickness's param. With SHEET_SHOTS=DIR each step is a picture there.

use forge_render::gui::{Gui, Headless};

const PART: &str = r#"shared t = 0.063in
param plan.L = 80
param plan.W = 50
param up.h = 20
param hs.x = 75
param hs.y = 25
param hs.r = 2
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = sheet(plan, t: t, vendor: "sendcutsend", material: "5052")
up = flange(base.edge(plan.outline[1]), height: h)
sketch hs(on: base.top) {
  c = circle((x, y), r)
}
vent = cutout(hs.c, wall: base)
"#;

const PLAIN: &str = r#"shared t = 2
shared r = 2
shared k = ratio(0.44)
param plan.L = 80
param plan.W = 50
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
"#;

fn ready(g: &Gui) -> bool {
    g.fg_anchors.is_some()
        && g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn open(text: &str, tag: &str) -> Option<Headless> {
    let dir = std::env::temp_dir().join(format!("vendor-ui-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.fg");
    std::fs::write(&file, text).unwrap();
    let mut h = match Headless::new((1500.0, 960.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    assert!(h.until(ready), "never ready: {}", h.gui.shell.status);
    for _ in 0..3 {
        h.step();
    }
    h.gui.shell.tab = forge_render::shell::SHEET_TAB;
    h.step();
    Some(h)
}

fn shot(h: &mut Headless, name: &str) {
    let Ok(dir) = std::env::var("SHEET_SHOTS") else {
        return;
    };
    for _ in 0..30 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    std::fs::write(format!("{dir}/vendor-{name}.ppm"), h.ppm()).unwrap();
}

#[test]
fn the_dfm_panel_lists_lights_and_marks_a_violation() {
    let Some(mut h) = open(PART, "dfm") else { return };
    assert!(h.click("tool:Check (DFM)"), "no DFM tool: {}", h.gui.shell.status);
    assert!(
        h.until(|g| g.shell.sheet.vendor.report.is_some()),
        "never checked"
    );
    h.step();
    let r = h.gui.shell.sheet.vendor.report.clone().unwrap();
    let s = &r.sheets[0];
    assert_eq!(s.vendor.as_ref().unwrap().id, "sendcutsend");
    let k = s
        .items
        .iter()
        .position(|i| i.rule == "HoleNearBend")
        .expect("the hole near the bend");
    shot(&mut h, "dfm");
    // chosen: its faces lit, apart from the build's own tints
    let before = h.gui.fg_flag_colours();
    h.gui.shell.sheet.vendor.chosen = Some((0, k));
    h.step();
    let lit: Vec<u32> = h
        .gui
        .fg_flag_colours()
        .iter()
        .filter(|c| c.rgba == forge_render::shell::vendorui::LIT)
        .map(|c| c.face)
        .collect();
    assert!(!lit.is_empty(), "nothing lit: before {before:?}");
    let want = &h.gui.fg_anchors.as_ref().unwrap().role_faces["vent.faces"];
    assert!(
        lit.iter().all(|f| want.contains(f)),
        "{lit:?} not the hole's {want:?}"
    );
    shot(&mut h, "dfm-chosen");
    // and on the flat pattern, its mark the chosen one
    let marks = h.gui.shell.sheet.vendor.flat_marks("base");
    assert!(marks.iter().any(|m| m.1), "{marks:?}");
    assert!(h.click("tool:Go to Flat Pattern"));
    h.step();
    shot(&mut h, "dfm-flat");
    assert!(h.click("tool:Go to Folded Part"));
    h.step();
    // moved clear of the bend, the next check finds nothing there
    let fixed = PART.replace("param hs.x = 75", "param hs.x = 40");
    let path = h.gui.fg.as_ref().unwrap().path.clone().unwrap();
    std::fs::write(&path, &fixed).unwrap();
    h.gui.open(&path);
    assert!(h.until(ready));
    assert!(h.until(|g| {
        g.shell.sheet.vendor.report.as_ref().is_some_and(|r| {
            r.sheets
                .first()
                .is_some_and(|s| s.items.iter().all(|i| i.rule != "HoleNearBend"))
        })
    }));
}

#[test]
fn the_vendor_window_writes_the_choice_into_the_sheet() {
    let Some(mut h) = open(PLAIN, "pick") else { return };
    assert!(h.click("tool:Vendor"));
    h.step();
    h.step();
    assert!(h.gui.shell.sheet.vendor.open);
    shot(&mut h, "window");
    let fg = h.gui.shell.fg.as_ref().unwrap();
    let ops =
        forge_render::shell::vendorui::apply_ops(fg, "fabworks", "5052-H32 aluminum", 0.063 * 25.4).unwrap();
    // t is the shared param's: set there, not overwritten in the call
    assert!(
        matches!(&ops[2], forge_lang::error::Op::SetParam { param, .. } if param.name == "t"),
        "{ops:?}"
    );
    let mut src = PLAIN.to_string();
    for op in &ops {
        src = forge_lang::edit::apply(&src, op).unwrap();
    }
    assert!(
        src.contains(r#"vendor: "fabworks""#) && src.contains(r#"material: "5052-H32 aluminum""#),
        "{src}"
    );
    let run = forge_lang::run(&src, forge_lang::Options::default(), Default::default()).0;
    // the part's own r and bend stay as written: they win over the vendor's
    assert!(run.errors.is_empty(), "{:?}\n{src}", run.errors);
}

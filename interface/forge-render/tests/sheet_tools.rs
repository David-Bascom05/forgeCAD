//! The Sheet Metal environment as a person uses it: File > New Sheet Metal, a sketch, then the
//! Sheet Metal tab's commands, each through its dialog (picks from the part's names, numbers
//! typed, OK): Face, Flange on two edges at once, Corner Seam, Hem, Cut, PunchTool, Insert
//! Hardware; then the Flat Pattern with its bend table and a DXF with Inventor's layers; and a
//! rack panel's flange edited through its dialog. Each step's text is checked and the part built.
//! With SHEET_SHOTS=DIR each step is a picture there; with SHEET_DEMO=FILE the part is saved.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::Action;
use forge_render::shell::sheet::SheetCmd;
use forge_render::shell::sketching::{SketchFor, SketchMode};

fn idle(g: &Gui) -> bool {
    g.fg.as_ref()
        .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
        && g.view.is_some()
}

fn settle(h: &mut Headless) {
    assert!(h.until(idle), "never built: {}", h.gui.shell.status);
    for _ in 0..4 {
        h.step();
    }
    assert!(h.until(idle));
}

fn shot(h: &mut Headless, name: &str) {
    let Ok(dir) = std::env::var("SHEET_SHOTS") else {
        return;
    };
    // windows fade in and the view turns: the picture once they're done
    for _ in 0..30 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    std::fs::write(format!("{dir}/sheet-{name}.ppm"), h.ppm()).unwrap();
}

fn click(h: &mut Headless, name: &str) {
    assert!(
        h.click(name),
        "nothing called {name}; status: {}",
        h.gui.shell.status
    );
    h.step();
    h.step();
}

/// A number typed into a dialog field, over what it held.
fn type_in(h: &mut Headless, field: &str, text: &str) {
    assert!(
        h.placed(&format!("sheet:field:{field}")).is_some(),
        "no field {field}"
    );
    h.gui.shell.sheet.dlg.as_mut().unwrap().set(field, text);
    h.step();
    h.step();
}

fn src(h: &Headless) -> String {
    h.gui.fg.as_ref().unwrap().src.clone()
}

fn built_ok(h: &Headless, what: &str) -> forge_lang::Run {
    let s = src(h);
    let run = forge_lang::run(&s, forge_lang::Options::default(), Default::default()).0;
    assert!(run.ok(), "{what}: {:?}\n{s}", run.errors);
    run
}

/// OK on the dialog: kept, the dialog gone, built.
fn ok(h: &mut Headless, what: &str) -> forge_lang::Run {
    settle(h);
    click(h, "sheet:ok");
    assert!(
        h.gui.shell.sheet.dlg.is_none(),
        "{what}: {:?}",
        h.gui.shell.sheet.dlg.as_ref().map(|d| d.error.clone())
    );
    settle(h);
    built_ok(h, what)
}

/// A sketch drawn on a plane the part names (an origin plane or a wall's skin), finished.
fn sketch(h: &mut Headless, on: &str, rows: &[&str]) {
    let plane = match on {
        "xy" => forge_math::Frame::WORLD,
        _ => *h
            .gui
            .fg_anchors
            .as_ref()
            .unwrap()
            .planes
            .get(&format!("{on}.plane"))
            .unwrap_or_else(|| panic!("no plane {on}")),
    };
    let mut m = SketchMode::new(SketchFor::New, on.to_string(), plane, Vec::new(), None);
    m.units = "mm";
    for r in rows {
        m.add(r.to_string());
    }
    h.gui.shell.sketch = Some(m);
    h.gui.shell.tab = 1;
    h.step();
    h.gui.shell.out.push(Action::FinishSketch);
    h.step();
    settle(h);
}

fn headless() -> Option<Headless> {
    match Headless::new((1600.0, 960.0), 1.0) {
        Ok(h) => Some(h),
        Err(e) => {
            eprintln!("skipped: {e}");
            None
        }
    }
}

#[test]
fn a_sheet_metal_part_made_with_the_tools_then_flat_and_its_dxf() {
    let Some(mut h) = headless() else { return };
    let dir = std::env::temp_dir().join(format!("sheet-tools-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("sheet_tools.fg");
    h.step();
    h.gui.new_sheet_part(file.to_str().unwrap()).unwrap();
    settle(&mut h);
    assert_eq!(h.gui.shell.tab, forge_render::shell::SHEET_TAB);
    assert!(src(&h).starts_with(forge_render::shell::sheet::PART_MARK));
    shot(&mut h, "00-new");

    // the base face's sketch, and Face
    sketch(&mut h, "xy", &["rect 0 0 120 80"]);
    h.gui.shell.tab = forge_render::shell::SHEET_TAB;
    h.step();
    click(&mut h, "tool:Face");
    click(&mut h, "sheet:choice:Profile:sk1");
    shot(&mut h, "01-face");
    let run = ok(&mut h, "face");
    assert!(
        src(&h).contains("face1 = sheet(sk1, t: t, r: r, bend: kfactor(k))"),
        "{}",
        src(&h)
    );
    assert!((run.measure.as_ref().unwrap().volume - 120.0 * 80.0 * 2.0).abs() < 1e-6);

    // Sheet Metal Defaults: thickness 1.5, set on the part's shared t
    click(&mut h, "tool:Sheet Metal Defaults");
    shot(&mut h, "02-defaults");
    h.gui.shell.sheet.defaults_open.as_mut().unwrap().t = 1.5;
    click(&mut h, "sheet:defaults:ok");
    settle(&mut h);
    assert!(src(&h).contains("shared t = 1.5"), "{}", src(&h));

    // Flange on two edges at once: flange1 and flange2, 30 high, outside the face's extents
    click(&mut h, "tool:Flange");
    click(&mut h, "sheet:choice:Edges:face1.edge(sk1.r1[1])");
    click(&mut h, "sheet:choice:Edges:face1.edge(sk1.r1[3])");
    type_in(&mut h, "h", "30");
    click(&mut h, "sheet:field:position");
    click(&mut h, "sheet:opt:position:outside");
    settle(&mut h);
    shot(&mut h, "03-flange");
    ok(&mut h, "flanges");
    let s = src(&h);
    assert!(s.contains("param flange1.h = 30"), "{s}");
    assert!(
        s.contains("flange1 = flange(face1.edge(sk1.r1[1]), height: h, position: \"outside\")"),
        "{s}"
    );
    assert!(
        s.contains("flange2 = flange(face1.edge(sk1.r1[3]), height: h, position: \"outside\")"),
        "{s}"
    );
    // a third along the front, and the corner it makes with flange1 seamed
    click(&mut h, "tool:Flange");
    click(&mut h, "sheet:choice:Edges:face1.edge(sk1.r1[0])");
    type_in(&mut h, "h", "30");
    click(&mut h, "sheet:field:position");
    click(&mut h, "sheet:opt:position:outside");
    ok(&mut h, "front flange");
    click(&mut h, "tool:Corner Seam");
    click(&mut h, "sheet:choice:Flange 1:flange3");
    click(&mut h, "sheet:choice:Flange 2:flange1");
    type_in(&mut h, "gap", "0.5");
    settle(&mut h);
    shot(&mut h, "04-corner-seam");
    ok(&mut h, "corner seam");
    assert!(
        src(&h).contains("seam1 = seam(flange3, flange1, gap: gap)"),
        "{}",
        src(&h)
    );

    // an open hem along flange2's top
    click(&mut h, "tool:Hem");
    click(&mut h, "sheet:choice:Edge:flange2.tip");
    click(&mut h, "sheet:field:kind");
    click(&mut h, "sheet:opt:kind:open");
    settle(&mut h);
    // folded in, it meets the front flange: the dialog says so and OK waits
    assert!(
        h.gui.shell.place("sheet:error").is_some(),
        "{:?}",
        h.gui.shell.fg.as_ref().unwrap().issues
    );
    shot(&mut h, "05-hem-refused");
    click(&mut h, "sheet:field:side");
    click(&mut h, "sheet:opt:side:down");
    settle(&mut h);
    shot(&mut h, "05-hem");
    ok(&mut h, "hem");
    assert!(
        src(&h).contains("hem1 = hem(flange2.tip, kind: \"open\", length: leg, gap: gap, side: \"down\")"),
        "{}",
        src(&h)
    );

    // a cut: a window sketched on the face's top
    sketch(&mut h, "face1.top", &["rect 45 25 30 30"]);
    h.gui.shell.tab = forge_render::shell::SHEET_TAB;
    h.step();
    click(&mut h, "tool:Cut");
    click(&mut h, "sheet:choice:Profile:sk2");
    click(&mut h, "sheet:slot:Face");
    click(&mut h, "sheet:choice:Face:face1");
    settle(&mut h);
    shot(&mut h, "06-cut");
    // the preview's built in the part's place; the file's text is what OK steps from
    assert!(h.gui.fg.as_ref().unwrap().previewing);
    let base = src(&h);
    let before = forge_lang::run(&base, forge_lang::Options::default(), Default::default()).0;
    let before = before.measure.unwrap().volume;
    let run = ok(&mut h, "cut");
    assert!(src(&h).contains("cut1 = cutout(sk2, wall: face1)"), "{}", src(&h));
    let window = 30.0 * 30.0 * 1.5;
    assert!(
        (before - run.measure.unwrap().volume - window).abs() < 1e-3,
        "{before}"
    );

    // PunchTool: an obround from the library at a point typed on the face, turned 90
    click(&mut h, "tool:PunchTool");
    click(&mut h, "sheet:choice:Face:face1");
    click(&mut h, "sheet:typed");
    click(&mut h, "sheet:tool:OB20X6");
    type_in(&mut h, "x", "25");
    type_in(&mut h, "y", "40");
    type_in(&mut h, "ang", "90");
    settle(&mut h);
    shot(&mut h, "07-punch");
    ok(&mut h, "punch");
    assert!(
        src(&h).contains("shared tool_ob20x6 = (\"OB20X6\", \"obround\", 20, 6)\npunch1 = punch(tool_ob20x6, at: (x, y), wall: face1, angle: ang)"),
        "{}",
        src(&h)
    );
    // and a dimple from the forming tools
    click(&mut h, "tool:PunchTool");
    click(&mut h, "sheet:choice:Face:face1");
    click(&mut h, "sheet:typed");
    click(&mut h, "sheet:lib:Forming tools");
    click(&mut h, "sheet:tool:DM20");
    type_in(&mut h, "x", "95");
    type_in(&mut h, "y", "40");
    settle(&mut h);
    shot(&mut h, "08-form");
    ok(&mut h, "form");
    assert!(
        src(&h).contains("punch2 = form(tool_dm20, at: (x, y), wall: face1)"),
        "{}",
        src(&h)
    );

    // Insert Hardware: a nut from the catalogue, at a point typed on the face
    click(&mut h, "tool:Insert Hardware");
    click(&mut h, "sheet:choice:Face:face1");
    click(&mut h, "sheet:typed");
    click(&mut h, "sheet:hw:nut:M4");
    type_in(&mut h, "x", "60");
    type_in(&mut h, "y", "12");
    settle(&mut h);
    shot(&mut h, "09-hardware");
    ok(&mut h, "pem");
    assert!(
        src(&h).contains("pem1 = pem(\"nut\", \"M4\", at: pem1_at.p, wall: face1)"),
        "{}",
        src(&h)
    );

    // one undo takes the whole hardware back out, one redo puts it back
    h.gui.shell.out.push(Action::Undo);
    h.step();
    assert!(!src(&h).contains("pem1"));
    h.gui.shell.out.push(Action::Redo);
    h.step();
    assert!(src(&h).contains("pem1 = pem("));
    settle(&mut h);

    // editing a flange through its dialog: double click opens it with the part's numbers
    h.gui.shell.sheet.edit_req = Some("flange1".into());
    h.step();
    h.step();
    assert_eq!(
        h.gui.shell.sheet.dlg.as_ref().unwrap().field("h").unwrap().num(),
        "30"
    );
    type_in(&mut h, "h", "36");
    settle(&mut h);
    shot(&mut h, "10-edit-flange");
    ok(&mut h, "edit flange1");
    assert!(src(&h).contains("param flange1.h = 36"), "{}", src(&h));

    // the issues a click away in the status bar, each with its fixes
    if h.gui.shell.place("fg:issues:status").is_some() {
        click(&mut h, "fg:issues:status");
        shot(&mut h, "11-issues");
        click(&mut h, "fg:issues:status");
    }

    // the flat pattern: its environment, bend lines, the bend table, the DXF with layers
    click(&mut h, "tool:Go to Flat Pattern");
    settle(&mut h);
    assert_eq!(h.gui.shell.tab, forge_render::shell::FLAT_TAB);
    assert!(h.gui.shell.fg.as_ref().unwrap().flat);
    let f = h.gui.shell.sheet.flats[0].clone();
    assert_eq!(f.bends.len(), 4, "{:?}", f.bends);
    assert!(f.bends.iter().any(|b| b.feature == "flange1"));
    assert!(!f.tools.is_empty());
    shot(&mut h, "12-flat");
    click(&mut h, "tool:Bend Table");
    shot(&mut h, "13-bend-table");
    click(&mut h, "tool:Bend Table");
    click(&mut h, "tool:Save Copy As DXF");
    // tangent lines too
    click(&mut h, "sheet:dxf:on:4");
    shot(&mut h, "14-dxf");
    click(&mut h, "sheet:dxf:ok");
    let dxf = std::fs::read_to_string(dir.join("sheet_tools.dxf")).expect("the DXF");
    for layer in [
        "IV_OUTER_PROFILE",
        "IV_INTERIOR_PROFILES",
        "IV_BEND",
        "IV_TANGENT",
        "IV_TOOL_CENTER",
        "IV_FEATURE_PROFILES",
    ] {
        assert!(dxf.contains(&format!("\n{layer}\n")), "{layer}");
    }
    assert!(dxf.contains("UP 90.0 DEG"), "bend notes");
    click(&mut h, "tool:Go to Folded Part");
    assert_eq!(h.gui.shell.tab, forge_render::shell::SHEET_TAB);
    assert!(!h.gui.shell.fg.as_ref().unwrap().flat);

    h.gui.shell.out.push(Action::Fg(forge_render::shell::FgCmd::Save));
    h.step();
    let saved = std::fs::read_to_string(&file).unwrap();
    assert_eq!(saved, src(&h));
    if let Ok(out) = std::env::var("SHEET_DEMO") {
        std::fs::write(out, &saved).unwrap();
    }
}

#[test]
fn a_rack_panels_return_flange_edited_through_its_dialog() {
    let Some(mut h) = headless() else { return };
    let from = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/rack18u/panels/side_panel.fg"
    );
    let dir = std::env::temp_dir().join(format!("sheet-rack-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("side_panel.fg");
    std::fs::copy(from, &file).unwrap();
    h.step();
    h.gui.open(file.to_str().unwrap());
    settle(&mut h);
    // a sheet metal part opens in its environment
    assert_eq!(h.gui.shell.tab, forge_render::shell::SHEET_TAB);
    // its flange row's Edit Feature
    assert!(h.menu_item("fg:row:rtn", "fg:menu:edit:rtn"));
    h.step();
    let d = h.gui.shell.sheet.dlg.as_ref().expect("the flange's dialog");
    assert_eq!(d.cmd, SheetCmd::Flange);
    assert_eq!(d.field("h").unwrap().num(), "20");
    // its height is the shared `ret`: SetParam on it
    type_in(&mut h, "h", "24");
    settle(&mut h);
    shot(&mut h, "15-rack-edit");
    ok(&mut h, "rack flange");
    assert!(src(&h).contains("shared ret = 24 "), "{}", src(&h));
    // one undo is the whole edit
    h.gui.shell.out.push(Action::Undo);
    h.step();
    assert!(src(&h).contains("shared ret = 20 "));
}

const BOX: &str = "shared t = 1.5
shared r = 2
shared k = ratio(0.44)
param plan.L = 100
param plan.W = 60
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
param right.h = 25
right = flange(base.edge(plan.outline[1]), height: h, position: \"outside\")
param back.h = 25
back = flange(base.edge(plan.outline[2]), height: h, position: \"outside\")
param lip.a = 15
param lip.b = 12
sketch lip(on: yz) {
  l1 = line((0, 0), (0, a))
  l2 = line((0, a), (-b, a))
}
";

#[test]
fn the_other_commands_preview_from_their_dialogs() {
    let Some(mut h) = headless() else { return };
    let dir = std::env::temp_dir().join(format!("sheet-others-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("box.fg");
    std::fs::write(&file, BOX).unwrap();
    // Fold: a plate bent along a line typed on it, square across
    let plate = dir.join("plate.fg");
    std::fs::write(&plate, BOX.split("param right.h").next().unwrap()).unwrap();
    h.step();
    h.gui.open(plate.to_str().unwrap());
    settle(&mut h);
    click(&mut h, "tool:Fold");
    click(&mut h, "sheet:choice:Face:base");
    for (k, v) in [("x0", "60"), ("y0", "-5"), ("x1", "60"), ("y1", "65")] {
        type_in(&mut h, k, v);
    }
    type_in(&mut h, "ang", "30");
    settle(&mut h);
    shot(&mut h, "17-fold");
    assert!(
        h.gui.shell.place("sheet:error").is_none(),
        "{:?}",
        h.gui.shell.fg.as_ref().unwrap().issues
    );
    ok(&mut h, "fold");
    assert!(src(&h).contains("param fold1.ang = 30deg"), "{}", src(&h));
    h.gui.open(file.to_str().unwrap());
    settle(&mut h);
    assert_eq!(h.gui.shell.tab, forge_render::shell::SHEET_TAB);
    // Corner Relief at the corner the two flanges make
    click(&mut h, "tool:Corner Relief");
    click(&mut h, "sheet:choice:Flange 1:right");
    click(&mut h, "sheet:choice:Flange 2:back");
    click(&mut h, "sheet:field:Relief shape");
    click(&mut h, "sheet:opt:Relief shape:square");
    type_in(&mut h, "size", "4");
    settle(&mut h);
    shot(&mut h, "16-corner-relief");
    ok(&mut h, "corner relief");
    assert!(
        src(&h).contains("corner1 = corner_relief(right, back, square_relief(size))"),
        "{}",
        src(&h)
    );
    // Contour Flange: an open sketch run along the front edge
    click(&mut h, "tool:Contour Flange");
    click(&mut h, "sheet:choice:Profile:lip.l1");
    click(&mut h, "sheet:choice:Profile:lip.l2");
    click(&mut h, "sheet:slot:Edge");
    click(&mut h, "sheet:choice:Edge:base.edge(plan.outline[0])");
    settle(&mut h);
    shot(&mut h, "18-contour-flange");
    assert!(
        h.gui.shell.place("sheet:error").is_none(),
        "{:?}",
        h.gui.shell.fg.as_ref().unwrap().issues
    );
    ok(&mut h, "contour flange");
    assert!(
        src(&h).contains("contour1 = contour([lip.l1, lip.l2], edge: base.edge(plan.outline[0]))"),
        "{}",
        src(&h)
    );
    // Jog, Unfold and the rules editor, as they open
    click(&mut h, "tool:Jog");
    click(&mut h, "sheet:choice:Face:base");
    shot(&mut h, "19-jog");
    click(&mut h, "sheet:cancel");
    click(&mut h, "tool:Unfold");
    click(&mut h, "sheet:choice:Sheet:base");
    settle(&mut h);
    shot(&mut h, "20-unfold");
    click(&mut h, "sheet:cancel");
    settle(&mut h);
    click(&mut h, "tool:Rules");
    shot(&mut h, "21-rules");
    click(&mut h, "sheet:rule:save");
    settle(&mut h);
    let s = src(&h);
    assert!(s.contains("shared rule1 = shop([rule1_stock], kerf: 0.2)"), "{s}");
    assert!(
        s.contains("base = sheet(plan, t: t, r: r, bend: kfactor(k), shop: rule1)"),
        "{s}"
    );
    built_ok(&h, "with the new rule");
}

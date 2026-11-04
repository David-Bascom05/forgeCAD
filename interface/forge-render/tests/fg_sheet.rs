//! Sheet metal on a Forge (.fg) part in the app: a sheet from a sketch, a flange off an edge
//! picked in the view, a cutout and an unfold from the tree's names, the sheet's report, and the
//! shop's broken rules fixed by their typed buttons.

use forge_render::gui::{Gui, Headless};

const PLAN: &str = "shared t = 2
param plan.L = 40
param plan.W = 30
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
";

/// An L, a hole sketched on its flange.
const L: &str = "shared t = 2
shared r = 2
shared k = ratio(0.4)
param plan.L = 40
param plan.W = 30
param up.h = 30
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
base = sheet(plan, t: t, r: r, bend: kfactor(k))
up = flange(base.edge(plan.outline[1]), height: h, measure: \"inner\")
param hs.x = 15
param hs.y = 15
param hs.d = 6
sketch hs(on: up.top) {
  c = circle((x, y), d)
}
";

/// The L made for a shop that tools other radii and wants longer flanges.
const SHOP: &str = "shared t = 2
shared r = 2
shared k = ratio(0.4)
param plan.L = 40
param plan.W = 30
param up.h = 30
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, W))
}
shared steel = stock(t: 2, bend: kfactor(ratio(0.4)), radii: [3, 4], min_flange: per_t(ratio(18)))
shared acme = shop([steel])
base = sheet(plan, t: t, r: r, bend: kfactor(k), shop: acme)
up = flange(base.edge(plan.outline[1]), height: h, measure: \"inner\")
";

fn ready(g: &Gui) -> bool {
    g.fg_anchors.is_some()
        && g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn open(text: &str, tag: &str) -> Option<Headless> {
    let dir = std::env::temp_dir().join(format!("fg-sheet-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.fg");
    std::fs::write(&file, text).unwrap();
    let mut h = match Headless::new((1300.0, 1000.0), 1.0) {
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
    Some(h)
}

fn point_at(h: &mut Headless, world: [f32; 3]) {
    // a view still turning (to a sketch's plane, say) puts the point elsewhere by the click
    assert!(h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving())));
    let (cam, size) = h.gui.shell.camera.unwrap();
    let on = cam.pixel_of(world, size.0, size.1).unwrap();
    let r = h.gui.shell.view_rect;
    h.move_to(r.min.x + on.0, r.min.y + on.1);
    let mut waited = 0;
    loop {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
        waited += 1;
        let v = h.gui.view.as_ref().unwrap();
        let known = v.nav.marks().iter().any(|m| !m.selected) && !v.nav.pick_pending();
        if (known && waited > 10) || waited > 120 {
            break;
        }
    }
    h.button(true);
    h.step();
    h.button(false);
    for _ in 0..10 {
        h.step();
    }
}

fn button(h: &mut Headless, tool: &str) {
    if h.gui.shell.tab != forge_render::shell::SHEET_TAB {
        h.gui.shell.tab = forge_render::shell::SHEET_TAB;
        h.step();
        h.step();
    }
    assert!(h.click(&format!("tool:{tool}")), "no button {tool}");
    assert!(h.until(|g| g.shell.sheet.dlg.is_some()));
    h.step();
    h.step();
}

fn choose(h: &mut Headless, slot: &str, text: &str) {
    let name = format!("sheet:choice:{slot}:{text}");
    if h.gui.shell.place(&name).is_none() {
        assert!(h.click(&format!("sheet:slot:{slot}")), "no slot {slot}");
        h.step();
        h.step();
    }
    assert!(h.click(&name), "no choice {name}");
    h.step();
    h.step();
}

fn ok(h: &mut Headless, what: &str) -> (String, forge_lang::Run) {
    assert!(h.until(ready));
    assert!(h.click("sheet:ok"));
    h.step();
    h.step();
    assert!(
        h.gui.shell.sheet.dlg.is_none(),
        "{what}: {:?}",
        h.gui.shell.sheet.dlg.as_ref().map(|d| d.error.clone())
    );
    assert!(h.until(ready));
    for _ in 0..3 {
        h.step();
    }
    let src = h.gui.fg.as_ref().unwrap().src.clone();
    let run = forge_lang::run(&src, forge_lang::Options::default(), Default::default()).0;
    assert!(run.ok(), "{what}: {:?}\n{src}", run.errors);
    (src, run)
}

#[test]
fn a_sheet_from_a_sketch_and_a_flange_off_the_edge_picked() {
    let Some(mut h) = open(PLAN, "flange") else { return };
    // a part with no sheet yet: Convert to Sheet Metal gives it the Sheet Metal tab
    assert!(h.click("tool:Convert to Sheet Metal"));
    h.step();
    h.step();
    assert_eq!(h.gui.shell.tab, forge_render::shell::SHEET_TAB);
    assert!(h.gui.shell.place("tool:Flange").is_some());
    button(&mut h, "Face");
    choose(&mut h, "Profile", "plan");
    let (src, run) = ok(&mut h, "sheet");
    assert!(
        src.contains("face1 = sheet(plan, t: t, r: r, bend: kfactor(k))"),
        "{src}"
    );
    // the defaults it lacked, written above it
    assert!(src.contains("shared r = 2\nshared k = ratio(0.44)\n"), "{src}");
    let flat_plate = 40.0 * 30.0 * 2.0;
    assert!((run.measure.as_ref().unwrap().volume - flat_plate).abs() < 1e-6);
    // its report in the panel: thickness, radius, its bends (none yet)
    let sheets = h.gui.shell.fg.as_ref().unwrap().sheets.clone();
    assert_eq!(sheets.len(), 1);
    assert!(
        sheets[0].facts.iter().any(|f| f.0 == "thickness" && f.1 == "2"),
        "{:?}",
        sheets[0].facts
    );
    // a flange off the right edge, picked on the top skin in the view
    button(&mut h, "Flange");
    point_at(&mut h, [40.0, 15.0, 0.0]);
    let picks = h.gui.shell.sheet.dlg.as_ref().unwrap().slots[0].picks.clone();
    assert_eq!(picks, ["face1.edge(plan.outline[1])"], "{}", h.gui.shell.status);
    let (src, run) = ok(&mut h, "flange");
    assert!(
        src.contains("param flange1.h = 20\nflange1 = flange(face1.edge(plan.outline[1]), height: h)"),
        "{src}"
    );
    assert!(run.measure.as_ref().unwrap().volume > flat_plate + 20.0 * 30.0 * 2.0 * 0.8);
    let sheets = h.gui.shell.fg.as_ref().unwrap().sheets.clone();
    let bends = sheets[0].lists.iter().find(|l| l.0 == "bends").unwrap();
    assert_eq!(bends.1.len(), 1, "{bends:?}");
    assert!(bends.1[0].contains("flange flange1"), "{bends:?}");
    assert!(h.gui.shell.place("fg:sheet:face1:bends").is_some());
}

#[test]
fn a_cutout_on_a_flange_and_an_unfold_from_the_trees_names() {
    let Some(mut h) = open(L, "cutout") else { return };
    let before = forge_lang::run(L, forge_lang::Options::default(), Default::default()).0;
    let v0 = before.measure.as_ref().unwrap().volume;
    button(&mut h, "Cut");
    choose(&mut h, "Profile", "hs.c");
    choose(&mut h, "Face", "up");
    let (src, run) = ok(&mut h, "cutout");
    assert!(src.contains("cut1 = cutout(hs.c, wall: up)"), "{src}");
    let hole = std::f64::consts::PI * 36.0 * 2.0;
    assert!((run.measure.as_ref().unwrap().volume - (v0 - hole)).abs() < 1e-3);
    button(&mut h, "Unfold");
    choose(&mut h, "Sheet", "base");
    let (src, run) = ok(&mut h, "unfold");
    assert!(src.contains("flat1 = unfold(base)"), "{src}");
    assert_eq!(run.measure.as_ref().unwrap().bodies, 2);
}

#[test]
fn the_shops_broken_rules_show_with_their_fixes_and_a_fix_clears_its_own() {
    let Some(mut h) = open(SHOP, "shop") else { return };
    let p = h.gui.shell.fg.as_ref().unwrap();
    let shop: Vec<(usize, String)> = p
        .issues
        .iter()
        .enumerate()
        .filter(|(_, i)| i.severity == forge_render::shell::fg::Severity::Violation)
        .map(|(k, i)| (k, i.kind.clone()))
        .collect();
    let kinds: Vec<&str> = shop.iter().map(|x| x.1.as_str()).collect();
    assert_eq!(kinds, ["RadiusNotTooled", "FlangeTooShort"], "{:?}", p.issues);
    let (k, _) = shop[0].clone();
    let fix = p.issues[k].fixes[0].clone();
    assert!(h.click(&format!("fg:shopfix:{k}:0")));
    h.step();
    assert_eq!(h.gui.fg.as_ref().unwrap().ops, [fix]);
    assert!(h.until(ready));
    h.step();
    let left: Vec<String> = h
        .gui
        .shell
        .fg
        .as_ref()
        .unwrap()
        .issues
        .iter()
        .filter(|i| i.severity == forge_render::shell::fg::Severity::Violation)
        .map(|i| i.kind.clone())
        .collect();
    assert_eq!(left, ["FlangeTooShort"]);
}

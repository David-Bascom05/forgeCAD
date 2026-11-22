//! What the retired `.fc` document mode did in the app, done on a Forge (.fg) part: a number
//! dragged on the model, a browser row lighting its faces, suppression and the End of Part, the
//! build's flags on the model, the dialogs' placings and axes, window selection, a sketch started
//! for an extrude and picked in the view, undo kept with the file, literal sketch points held
//! free, the part's own unit, and a part the check faults (tests/faulty). Each is driven as a
//! person would, headless; each writes Forge.

use forge_lang::error::Op;
use forge_render::gui::{Gui, Headless};
use forge_render::shell::Action;
use forge_render::shell::fgdim::Target;

const BLOCK: &str = "shared L = 60
shared t = 20
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, L))
}
base = extrude(plan, t)
param marks.px = 10
param marks.py = 10
param marks.cx = 45
param marks.cy = 45
param marks.r = 5
param marks.sx = 10
param marks.sy = 40
param marks.s = 6
sketch marks(on: base.end) {
  p = pt(px, py)
  c = circle((cx, cy), r)
  sq = rect(corner: (sx, sy), size: (s, s))
}
param boss.h = 10
boss = extrude(marks.c, h)
";

fn ready(g: &Gui) -> bool {
    g.fg_anchors.is_some()
        && g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn shots() -> std::path::PathBuf {
    let d = std::path::PathBuf::from(
        std::env::var("FG_GAP_SHOTS")
            .unwrap_or_else(|_| std::env::temp_dir().join("fg-gap-shots").display().to_string()),
    );
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The window as it is, kept as a picture.
fn shot(h: &mut Headless, name: &str) {
    for _ in 0..3 {
        h.step();
    }
    if let Ok(png) = h.png() {
        let _ = std::fs::write(shots().join(format!("{name}.png")), png);
    }
}

fn file(text: &str, tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fg-gap-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.fg");
    std::fs::write(&file, text).unwrap();
    let _ = std::fs::remove_file(forge_lang::undo::log_path(&file));
    file
}

fn open_file(file: &std::path::Path) -> Option<Headless> {
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

fn open(text: &str, tag: &str) -> Option<(Headless, std::path::PathBuf)> {
    let f = file(text, tag);
    Some((open_file(&f)?, f))
}

fn src(h: &Headless) -> String {
    h.gui.fg.as_ref().unwrap().src.clone()
}

fn settle(h: &mut Headless) {
    assert!(h.until(ready), "{}", h.gui.shell.status);
    for _ in 0..3 {
        h.step();
    }
}

fn volume_of(text: &str) -> f64 {
    let run = forge_lang::run(text, forge_lang::Options::default(), Default::default()).0;
    assert!(run.ok(), "{:?}\n{text}", run.errors);
    run.measure.as_ref().unwrap().volume
}

/// Clicks where this place in the world is on screen, once the view knows what's under it.
fn point_at(h: &mut Headless, world: [f32; 3]) {
    assert!(h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving())));
    let (cam, size) = h.gui.shell.camera.unwrap();
    let on = cam.pixel_of(world, size.0, size.1).unwrap();
    let r = h.gui.shell.view_rect;
    h.move_to(r.min.x + on.0, r.min.y + on.1);
    assert!(h.settle_hover(), "the pointer's pick never came");
    h.button(true);
    h.step();
    h.button(false);
    for _ in 0..10 {
        h.step();
    }
}

fn screen(h: &Headless, world: [f64; 3]) -> egui::Pos2 {
    let (cam, size) = h.gui.shell.camera.unwrap();
    let on = cam.pixel_of(world.map(|v| v as f32), size.0, size.1).unwrap();
    h.gui.shell.view_rect.min + egui::vec2(on.0, on.1)
}

fn button(h: &mut Headless, tool: &str) {
    assert!(h.click(&format!("tool:{tool}")), "no button {tool}");
    assert!(
        h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some())),
        "{tool}: no form: {}",
        h.gui.shell.status
    );
    h.step();
    h.step();
}

fn click(h: &mut Headless, place: &str) {
    assert!(h.click(place), "no {place}");
    h.step();
    h.step();
}

fn choose(h: &mut Headless, slot: &str, text: &str) {
    click(h, &format!("fg:tool:slot:{slot}"));
    click(h, &format!("fg:choice:{slot}:{text}"));
}

fn picks(h: &Headless, slot: &str) -> Vec<String> {
    let t = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap();
    t.slots.iter().find(|s| s.label == slot).unwrap().picks.clone()
}

fn set(h: &mut Headless, param: &str, text: &str) {
    let t = h.gui.shell.fg.as_mut().unwrap().tool.as_mut().unwrap();
    t.params.iter_mut().find(|p| p.name == param).unwrap().text = text.into();
    h.step();
}

/// OK: the tool's statement applied and built; the part's text.
fn ok(h: &mut Headless, what: &str, name: &str) -> String {
    settle(h);
    shot(h, name);
    assert!(h.click("fg:tool:ok"));
    h.step();
    let p = h.gui.shell.fg.as_ref().unwrap();
    assert!(
        p.tool.is_none(),
        "{what}: the tool stayed open: {:?} {:?}",
        p.tool.as_ref().and_then(|t| t.error.clone()),
        p.refused
    );
    settle(h);
    src(h)
}

/// Drags the number's arrow `px` points further along its way, a few moves at a time, and lets go
/// unless `hold`.
fn drag_arrow(h: &mut Headless, px: f32, hold: bool) {
    let d = h.gui.shell.fg_dim.clone().expect("an arrow");
    let grip = h.gui.shell.place("dim:arrow").expect("its grip");
    let base = screen(h, d.origin);
    let way = (grip.center() - base).normalized();
    h.move_to(grip.center().x, grip.center().y);
    h.step();
    h.button(true);
    h.step();
    for k in 1..=8 {
        let at = grip.center() + way * px * k as f32 / 8.0;
        h.move_to(at.x, at.y);
        h.step();
    }
    if !hold {
        h.button(false);
        h.step();
    }
}

#[test]
fn a_number_dragged_on_the_model_previews_live_and_writes_its_param() {
    let text = format!(
        "{BLOCK}param side.cx = 15\nparam side.cy = 15\nparam side.r = 5\nsketch side(on: base.end) {{\n  c = circle((cx, cy), r)\n}}\nlump = extrude(side.c, 6)\n"
    );
    let Some((mut h, _)) = open(&text, "dim") else {
        return;
    };
    click(&mut h, "fg:row:boss");
    assert!(h.until(|g| g.shell.fg_dim.is_some()), "no arrow");
    let d = h.gui.shell.fg_dim.clone().unwrap();
    assert_eq!(d.what, "depth");
    assert_eq!(d.value, 10.0);
    assert!(
        matches!(&d.target, Target::Param(k) if k.to_string() == "boss.h"),
        "{:?}",
        d.target
    );
    shot(&mut h, "gap01-dim-arrow");
    // held: the part follows, the file's text kept
    drag_arrow(&mut h, 60.0, true);
    assert!(h.until(|g| g.fg.as_ref().unwrap().previewing));
    let moved = h.gui.shell.fg_dim.as_ref().unwrap().value;
    assert!(moved > 10.0, "{moved}");
    assert!(src(&h).contains("param boss.h = 10\n"));
    settle(&mut h);
    shot(&mut h, "gap01-dim-drag-preview");
    // let go: written, one undo step
    h.button(false);
    h.step();
    settle(&mut h);
    let now = src(&h);
    assert!(!now.contains("param boss.h = 10\n"), "{now}");
    let v: f64 = now
        .lines()
        .find_map(|l| l.strip_prefix("param boss.h = "))
        .unwrap()
        .parse()
        .unwrap();
    assert!((v - moved).abs() < 1e-9, "{v} {moved}");
    assert!(!h.gui.fg.as_ref().unwrap().previewing);
    // a number written in the call: lifted to a param of the feature, then set
    click(&mut h, "fg:row:lump");
    assert!(h.until(|g| g.shell.fg_dim.as_ref().is_some_and(|d| d.feat == "lump")));
    assert!(matches!(
        h.gui.shell.fg_dim.as_ref().unwrap().target,
        Target::Literal { .. }
    ));
    drag_arrow(&mut h, 40.0, false);
    settle(&mut h);
    let now = src(&h);
    assert!(now.contains("lump = extrude(side.c, depth)"), "{now}");
    let depth: f64 = now
        .lines()
        .find_map(|l| l.strip_prefix("param lump.depth = "))
        .unwrap()
        .parse()
        .unwrap();
    assert!(depth > 6.0, "{depth}");
    // undone, both ops at once
    h.gui.shell.out.push(Action::Undo);
    h.step();
    assert!(src(&h).contains("lump = extrude(side.c, 6)"));
}

#[test]
fn a_browser_row_lights_the_faces_it_made_and_a_flagged_one_is_tinted() {
    let text = format!(
        "{BLOCK}param side.cx = 15\nparam side.cy = 15\nparam side.r = 5\nsketch side(on: base.end) {{\n  c = circle((cx, cy), r)\n}}\nlump = extrude(side.c, 6)\n"
    );
    let Some((mut h, _)) = open(&text, "lit") else {
        return;
    };
    assert!(h.gui.fg_lit_faces().is_empty());
    // a lint is flagged on its row (the tree), not on the model until it's chosen
    assert!(h.gui.fg_flag_colours().is_empty());
    click(&mut h, "fg:row:boss");
    let want = h.gui.fg_anchors.as_ref().unwrap().role_faces["boss.faces"].clone();
    assert_eq!(want.len(), 2, "the boss's wall and top");
    assert_eq!(h.gui.fg_lit_faces(), want);
    shot(&mut h, "gap02-row-lights-faces");
    click(&mut h, "fg:row:lump");
    let lump = h.gui.fg_anchors.as_ref().unwrap().role_faces["lump.faces"].clone();
    assert_eq!(h.gui.fg_lit_faces(), lump);
    let tint = h.gui.fg_flag_colours();
    assert_eq!(
        tint.iter().map(|c| c.face).collect::<Vec<_>>(),
        lump,
        "the loose number's feature, amber"
    );
    assert!(tint.iter().all(|c| c.rgba[0] > c.rgba[2]), "{tint:?}");
    shot(&mut h, "gap04-flag-on-model");
    click(&mut h, "fg:row:lump");
    assert!(h.gui.fg_lit_faces().is_empty(), "chosen again: let go");
}

#[test]
fn suppress_unsuppress_and_the_end_of_part_in_the_browser() {
    let Some((mut h, _)) = open(BLOCK, "suppress") else {
        return;
    };
    let whole = volume_of(BLOCK);
    assert!(h.menu_item("fg:row:boss", "fg:menu:suppress:boss"));
    settle(&mut h);
    let now = src(&h);
    assert!(now.contains("#~boss boss = extrude(marks.c, h)"), "{now}");
    assert!(
        h.gui.shell.place("fg:suppressed:boss").is_some(),
        "its row stays, struck through"
    );
    assert!(
        !h.gui
            .shell
            .fg
            .as_ref()
            .unwrap()
            .feats
            .iter()
            .any(|f| f.id == "boss")
    );
    shot(&mut h, "gap03-suppressed");
    assert!(h.menu_item("fg:suppressed:boss", "fg:menu:unsuppress:boss"));
    settle(&mut h);
    assert_eq!(src(&h), BLOCK);
    // the End of Part moved up to the base: what follows evaluated, not built
    assert!(h.gui.shell.place("fg:row:endofpart").is_some());
    assert!(h.menu_item("fg:row:base", "fg:menu:rollback:base"));
    settle(&mut h);
    assert_eq!(h.gui.fg.as_ref().unwrap().until().as_deref(), Some("base"));
    let p = h.gui.shell.fg.as_ref().unwrap();
    let boss = p.feats.iter().find(|f| f.id == "boss").unwrap();
    assert_eq!(boss.status, forge_render::shell::fg::Status::NotBuilt);
    assert!(p.summary.contains("volume 72000 mm³"), "{}", p.summary);
    let eop = h.gui.shell.place("fg:row:endofpart").unwrap();
    let base_row = h.gui.shell.place("fg:row:base").unwrap();
    let boss_row = h.gui.shell.place("fg:row:boss").unwrap();
    assert!(
        base_row.top() < eop.top() && eop.top() < boss_row.top(),
        "just under the base"
    );
    assert_eq!(src(&h), BLOCK, "the file's text kept");
    shot(&mut h, "gap03-end-of-part");
    assert!(h.menu_item("fg:row:endofpart", "fg:menu:rollforward"));
    settle(&mut h);
    let p = h.gui.shell.fg.as_ref().unwrap();
    assert!(
        p.summary
            .contains(&format!("volume {} mm³", (whole * 10.0).round() / 10.0)),
        "{}",
        p.summary
    );
}

#[test]
fn a_window_takes_the_edges_it_holds_for_an_edges_selection() {
    let Some((mut h, _)) = open(BLOCK, "window") else {
        return;
    };
    button(&mut h, "Fillet");
    assert!(picks(&h, "Edges").is_empty());
    // from the view's top right (clear of the dialog) across the whole part: what it crosses
    let r = h.gui.shell.view_rect;
    let (a, b) = (
        egui::pos2(r.max.x - 30.0, r.min.y + 30.0),
        egui::pos2(r.center().x - 260.0, r.max.y - 30.0),
    );
    h.move_to(a.x, a.y);
    h.step();
    h.button(true);
    h.step();
    for k in 1..=10 {
        let at = a + (b - a) * k as f32 / 10.0;
        h.move_to(at.x, at.y);
        h.step();
    }
    shot(&mut h, "gap06-window-select");
    h.button(false);
    for _ in 0..4 {
        h.step();
    }
    let got = picks(&h, "Edges");
    assert!(got.len() >= 6, "{got:?}: {}", h.gui.shell.status);
    assert!(got.iter().all(|e| e.starts_with("edges(between:")), "{got:?}");
    shot(&mut h, "gap06-window-edges-taken");
}

#[test]
fn a_hole_from_two_edges_round_an_edge_or_on_a_sketchs_points() {
    let Some((mut h, _)) = open(BLOCK, "holes") else {
        return;
    };
    // a sketch's every point
    button(&mut h, "Hole");
    choose(&mut h, "Points", "marks.points");
    assert_eq!(picks(&h, "Points"), ["marks.points"]);
    click(&mut h, "fg:tool:cancel");
    // on the top, 10 in from its front edge and 15 from its right
    button(&mut h, "Hole");
    click(&mut h, "fg:tool:choice:Placement:Linear");
    point_at(&mut h, [30.0, 20.0, 20.0]);
    point_at(&mut h, [30.0, 0.0, 20.0]);
    point_at(&mut h, [60.0, 25.0, 20.0]);
    set(&mut h, "d1", "10");
    set(&mut h, "d2", "15");
    assert_eq!(picks(&h, "Face"), ["base.end"]);
    let s = ok(&mut h, "linear hole", "gap05-hole-linear");
    assert!(s.contains("sketch hole1_at(on: base.end) {"), "{s}");
    assert!(
        s.contains("hole1 = hole(at: hole1_at.p, d: d, depth: thru)"),
        "{s}"
    );
    let holes = forge_lang::query::ask(&s, None, None, "holes").text;
    assert!(holes.contains("centre (45, 10, 10)"), "{holes}");
    // round the boss's top rim, into it
    button(&mut h, "Hole");
    click(&mut h, "fg:tool:choice:Placement:Concentric");
    click(&mut h, "fg:tool:choice:Termination:Distance");
    point_at(&mut h, [45.0, 45.0, 30.0]);
    point_at(&mut h, [45.0, 40.0, 30.0]);
    let s = ok(&mut h, "concentric hole", "gap05-hole-concentric");
    let line = s.lines().find(|l| l.starts_with("hole2 = ")).unwrap();
    assert!(line.starts_with("hole2 = hole(at: edges(between: ("), "{line}");
    assert!(
        line.ends_with(").center, on: boss.end, d: d, depth: depth, point: point)"),
        "{line}"
    );
    let holes = forge_lang::query::ask(&s, None, None, "holes").text;
    assert!(holes.contains("centre (45, 45,"), "{holes}");
}

#[test]
fn revolve_about_a_work_axis_an_origin_axis_or_a_straight_edge() {
    let text = format!(
        "{BLOCK}wax = workaxis(along: edges(between: (base.end, base.side(plan.outline[0]))))\nparam side.x = 50\nsketch side(on: xz) {{\n  sq = rect(corner: (x, 0), size: (5, 5))\n}}\n"
    );
    let Some((mut h, _)) = open(&text, "revolve") else {
        return;
    };
    // the work axis, from what the selection offers
    h.gui.fg_open_tool("revolve");
    h.step();
    choose(&mut h, "Profile", "marks.sq");
    choose(&mut h, "Axis", "wax");
    assert_eq!(picks(&h, "Axis"), ["wax"]);
    set(&mut h, "ang", "90");
    let s = ok(&mut h, "about a work axis", "gap05-revolve-work-axis");
    assert!(
        s.contains("revolve1 = revolve(marks.sq, axis: wax, angle: ang)"),
        "{s}"
    );
    // the origin's Z
    button(&mut h, "Revolve");
    choose(&mut h, "Profile", "side.sq");
    choose(&mut h, "Axis", "Z");
    set(&mut h, "ang", "90");
    let s = ok(&mut h, "about Z", "gap05-revolve-origin-axis");
    assert!(
        s.contains("revolve2 = revolve(side.sq, axis: Z, angle: ang)"),
        "{s}"
    );
    // a straight edge of the part, picked in the view
    h.gui.fg_open_tool("revolve");
    h.step();
    choose(&mut h, "Profile", "marks.sq");
    click(&mut h, "fg:tool:slot:Axis");
    point_at(&mut h, [30.0, 0.0, 20.0]);
    let e = "edges(between: (base.end, base.side(plan.outline[0])))";
    assert_eq!(picks(&h, "Axis"), [format!("axis({e}.start, {e}.end)")]);
    set(&mut h, "ang", "45");
    let s = ok(&mut h, "about an edge", "gap05-revolve-edge");
    assert!(
        s.contains(&format!(
            "revolve3 = revolve(marks.sq, axis: axis({e}.start, {e}.end), angle: ang)"
        )),
        "{s}"
    );
    let _ = volume_of(&s);
}

#[test]
fn extrude_up_to_a_face_picked_and_a_rib_finds_its_side() {
    let text = "shared L = 60
shared t = 20
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, L))
}
base = extrude(plan, t)
param stepsk.w = 20
sketch stepsk(on: base.end) {
  r = rect(corner: (0, 0), size: (w, L))
}
param step.h = 10
step = extrude(stepsk.r, h)
param low.r = 5
sketch low(on: base.end) {
  c = circle((40, 30), r)
}
";
    let Some((mut h, _)) = open(text, "upto") else {
        return;
    };
    button(&mut h, "Extrude");
    choose(&mut h, "Profile", "low.c");
    click(&mut h, "fg:tool:choice:Extents:To");
    click(&mut h, "fg:tool:slot:To");
    point_at(&mut h, [10.0, 30.0, 30.0]);
    assert_eq!(picks(&h, "To"), ["step.end"]);
    let s = ok(&mut h, "up to a face", "gap05-extrude-upto");
    assert!(s.contains("extrude1 = extrude(low.c, upto(step.end))"), "{s}");
    let grew = volume_of(&s) - volume_of(text);
    assert!((grew - std::f64::consts::PI * 25.0 * 10.0).abs() < 0.5, "{grew}");

    // a rib starts on its path's left; refused there, its other side is tried by itself
    let cup = "shared L = 60
shared H = 30
shared t = 2
sketch base(on: xy) {
  r = rect(center: (0, 0), size: (L, L))
  x = line((0, 0), (L / 2, 0), construction: true)
}
box = extrude(base, H)
cup = shell(t, open: box.end)
sketch web(on: rotate(base.plane, about: base.x, by: 90deg)) {
  l = line((0 - L / 4, H / 2), (L / 4, H / 2))
}
";
    let Some((mut h, _)) = open(cup, "rib") else {
        return;
    };
    button(&mut h, "Rib");
    choose(&mut h, "Path", "web.l");
    assert!(
        h.until(|g| g.shell.fg.as_ref().unwrap().tool.as_ref().unwrap().flags[0].1),
        "the side it belongs on never found"
    );
    let s = ok(&mut h, "rib", "gap05-rib-side");
    assert!(s.contains("rib1 = rib(web.l, t, flip: true)"), "{s}");
}

#[test]
fn new_extrude_with_no_sketch_free_starts_one_and_a_sketch_is_picked_in_the_view() {
    let Some((mut h, _)) = open(BLOCK, "nofree") else {
        return;
    };
    // every sketch is read: Extrude starts a sketch, then takes it
    assert!(h.click("tool:Extrude"));
    h.step();
    h.step();
    assert!(h.gui.shell.sketch_pick, "{}", h.gui.shell.status);
    assert!(h.gui.shell.fg.as_ref().unwrap().tool.is_none());
    point_at(&mut h, [20.0, 20.0, 20.0]);
    assert!(h.gui.shell.sketch.is_some(), "{}", h.gui.shell.status);
    let m = h.gui.shell.sketch.as_mut().unwrap();
    assert!(m.add("circle 5 5 3".into()), "{:?}", m.error);
    h.gui.shell.out.push(Action::FinishSketch);
    h.step();
    h.step();
    let p = h.gui.shell.fg.as_ref().unwrap();
    let t = p.tool.as_ref().expect("the extrude, opened on it");
    assert_eq!(t.kind, "extrude");
    assert_eq!(t.slots[0].picks, ["sk1"]);
    settle(&mut h);
    shot(&mut h, "gap07-extrude-after-sketch");
    click(&mut h, "fg:tool:cancel");

    // a sketch's loop picked where it's drawn on the part
    h.gui.fg_open_tool("extrude");
    settle(&mut h);
    let c = screen(&h, [50.0, 45.0, 20.0]);
    h.move_to(c.x, c.y);
    for _ in 0..4 {
        h.step();
    }
    assert!(
        h.gui.shell.place("fg:sketch3d:marks.c").is_some(),
        "the circle lit under the pointer"
    );
    shot(&mut h, "gap07-sketch-picked-in-view");
    h.button(true);
    h.step();
    h.button(false);
    for _ in 0..4 {
        h.step();
    }
    assert_eq!(picks(&h, "Profile"), ["marks.c"]);
}

#[test]
fn undo_depth_is_kept_with_the_file() {
    let f = file(BLOCK, "undo");
    let Some(mut h) = open_file(&f) else { return };
    for v in [11.0, 12.0] {
        h.gui
            .fg_apply(Op::SetParam {
                param: forge_lang::error::ParamKey {
                    owner: vec![forge_lang::syntax::Seg {
                        name: "boss".into(),
                        index: vec![],
                    }],
                    name: "h".into(),
                },
                value: forge_lang::error::Literal {
                    value: v,
                    dim: forge_lang::values::Dim::Length,
                },
            })
            .unwrap();
    }
    h.gui.fg_save();
    settle(&mut h);
    drop(h);
    let Some(mut h) = open_file(&f) else { return };
    assert_eq!(h.gui.fg.as_ref().unwrap().depth(), (2, 0));
    h.gui.shell.out.push(Action::Undo);
    h.step();
    assert!(src(&h).contains("param boss.h = 11\n"));
    h.gui.shell.out.push(Action::Undo);
    h.step();
    assert_eq!(src(&h), BLOCK);
    // and the CLI's undo reads the same log
    h.gui.shell.out.push(Action::Redo);
    h.step();
    h.gui.fg_save();
    assert_eq!(
        forge_lang::undo::step(&f, forge_lang::undo::Step::Undo).as_deref(),
        Ok(BLOCK)
    );
}

#[test]
fn a_sketchs_literal_points_open_free_and_parallel_would_pick() {
    use forge_render::shell::sketching::SkTool;
    let text = "sketch plan(on: xy) {
  outline = rect(corner: (-30, -30), size: (60, 60))
}
base = extrude(plan, 20)
sketch lines(on: base.end) {
  l1 = line((0, 0), (40, 0))
  l2 = line((0, 20), (40, 25))
}
";
    let Some((mut h, _)) = open(text, "literal") else {
        return;
    };
    h.gui
        .shell
        .fg
        .as_mut()
        .unwrap()
        .out
        .push(forge_render::shell::fg::FgAct::EditSketch("lines".into()));
    for _ in 0..4 {
        h.step();
    }
    let m = h.gui.shell.sketch.as_ref().expect("the sketch open");
    assert!(!m.rows.iter().any(|r| r.starts_with("fix")), "{:?}", m.rows);
    for _ in 0..40 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    h.gui.shell.out.push(Action::SketchTool(SkTool::Parallel));
    for _ in 0..4 {
        h.step();
    }
    let at = screen(&h, [20.0, 22.5, 20.0]);
    h.move_to(at.x, at.y);
    for _ in 0..4 {
        h.step();
    }
    assert_eq!(
        h.gui.shell.sketch.as_ref().unwrap().would_pick.as_deref(),
        Some("l2"),
        "{:?}",
        h.gui.shell.sketch.as_ref().unwrap().error
    );
    shot(&mut h, "gap10-parallel-would-pick");
}

#[test]
fn a_parts_unit_is_kept_in_its_text_and_typed_numbers_take_it() {
    let f = file(BLOCK, "units");
    let Some(mut h) = open_file(&f) else { return };
    let was = volume_of(BLOCK);
    h.gui.shell.out.push(Action::SetUnits("in"));
    h.step();
    assert_eq!(h.gui.shell.units, "in");
    assert!(src(&h).starts_with("# units in\n"), "{}", src(&h));
    assert_eq!(volume_of(&src(&h)).to_bits(), was.to_bits(), "only how it reads");
    // a length typed bare is in inches; what the tool showed already says mm
    button(&mut h, "Fillet");
    assert_eq!(
        h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap().params[0].text,
        "2 mm"
    );
    point_at(&mut h, [30.0, 0.0, 20.0]);
    set(&mut h, "r", "1/8");
    let s = ok(&mut h, "fillet in inches", "gap11-units-inch");
    assert!(s.contains("param fillet1.r = 3.175"), "{s}");
    h.gui.fg_save();
    drop(h);
    // reopened, it reads in its own unit
    let Some(h) = open_file(&f) else { return };
    assert_eq!(h.gui.shell.units, "in");
}

/// A part the kernel builds and the check faults, kept in tests/faulty.
fn faulty(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/faulty/{name}", env!("CARGO_MANIFEST_DIR")))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The check's problems of a part, by kind, and the features whose bodies have them.
fn faults(text: &str) -> Vec<(&'static str, String)> {
    let run = forge_lang::run(text, forge_lang::Options::default(), Default::default()).0;
    assert!(run.errors.is_empty(), "it builds: {:?}", run.errors);
    let m = run.measure.expect("measured");
    m.invalid.into_iter().map(|i| (i.kind, i.body)).collect()
}

fn summary(h: &Headless) -> String {
    h.gui.shell.fg.as_ref().unwrap().summary.clone()
}

#[test]
fn a_part_the_check_faults_says_so_and_fixing_the_part_clears_it() {
    // a part the kernel builds and the check doesn't pass: a fillet whose loops cross
    let part = faulty("check-loops-cross.fg");
    let found = faults(&part);
    assert!(found.contains(&("LoopsCross", "block".to_string())), "{found:?}");
    let Some((mut h, file)) = open(&part, "fault") else {
        return;
    };
    assert!(
        h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.summary.contains("INVALID"))),
        "the check found nothing on a part known to fail it: {}",
        summary(&h)
    );
    // the fillet taken out: the part is sound, and it says so by itself
    assert!(h.menu_item("fg:row:corners", "fg:menu:delete:corners"));
    settle(&mut h);
    assert!(
        h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.summary.contains(" valid"))),
        "{}\n{}",
        summary(&h),
        std::fs::read_to_string(&file).unwrap()
    );
    assert!(faults(&src(&h)).is_empty());
}

#[test]
fn a_part_with_an_edge_off_its_face_is_found_too() {
    let part = faulty("check-edge-off-face.fg");
    let found = faults(&part);
    assert!(found.iter().any(|f| f.0 == "EdgeOffFace"), "{found:?}");
    // its broken face has no mesh, so the view is never whole: wait on the build's word alone
    let mut h = match Headless::new((1300.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    h.gui.open(file(&part, "edgeoff").to_str().unwrap());
    assert!(
        h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.summary.contains("INVALID"))),
        "the check found nothing on a part known to fail it: {}",
        summary(&h)
    );
}

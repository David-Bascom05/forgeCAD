//! The ribbon's tools on a Forge (.fg) part, as a person uses them: the button, picks made by
//! pointing at the part (each becomes the anchor Forge names it by), choices from the part's
//! sketches and features, OK. Each writes one Forge statement through a typed AddFeature op; the
//! text is checked, and what it builds is measured.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::Action;

const BLOCK: &str = "shared L = 60
shared t = 20
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, L))
}
base = extrude(plan, t)
param marks.px = 10
param marks.py = 10
param marks.qx = 20
param marks.cx = 45
param marks.cy = 45
param marks.r = 5
param marks.sx = 10
param marks.sy = 40
param marks.s = 6
sketch marks(on: base.end) {
  p = pt(px, py)
  q = pt(qx, py)
  c = circle((cx, cy), r)
  sq = rect(corner: (sx, sy), size: (s, s))
}
param boss.h = 10
boss = extrude(marks.c, h)
";

/// A plain block.
const PLAIN: &str = "shared L = 60
shared t = 20
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, L))
}
base = extrude(plan, t)
";

/// On the boss's wall, the side the view looks from (its home corner is +x -y +z).
const BOSS_WALL: [f32; 3] = [48.5, 41.5, 25.0];

/// The block with a second body beside it, for combining.
const TWO: &str = "shared L = 60
shared t = 20
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, L))
}
base = extrude(plan, t)
param side.cx = 60
param side.cy = 30
param side.r = 8
sketch side(on: base.end) {
  c = circle((cx, cy), r)
}
param lump.h = 10
lump = extrude(side.c, h, new: true)
";

/// A profile, a path square to it, an axis beside it, and a second profile above: for sweep,
/// coil and loft (the reference's own example).
const SPRING: &str = "shared r = 3
shared w = 10
shared h = 30
shared R = 15
sketch sec(on: xy) {
  c = circle((0, 0), r)
  base = line((0, 0), (w, 0), construction: true)
}
sketch rail(on: rotate(sec.plane, about: sec.base, by: 90deg)) {
  path route(start: (0, 0)) {
    line(to: (0, h))
    tarc(to: (R, h + R))
  }
}
sketch spr(on: rotate(sec.plane, about: sec.base, by: 90deg)) {
  c = circle((2 * R, 0), r / 2)
  ax = line((0, 0), (0, h), construction: true)
}
sketch top(on: offset(sec.plane, h)) {
  sq = rect(center: (0, 0), size: (w, w))
}
";

fn ready(g: &Gui) -> bool {
    g.fg_anchors.is_some()
        && g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn open(text: &str, tag: &str) -> Option<(Headless, std::path::PathBuf)> {
    let dir = std::env::temp_dir().join(format!("fg-tool-{}-{tag}", std::process::id()));
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
    Some((h, file))
}

/// Clicks where this place in the world is on screen, once the view knows what's under it.
fn point_at(h: &mut Headless, world: [f32; 3]) {
    // a view still turning (to a sketch's plane, say) puts the point elsewhere by the click
    assert!(h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving())));
    let (cam, size) = h.gui.shell.camera.unwrap();
    let on = cam.pixel_of(world, size.0, size.1).unwrap();
    let r = h.gui.shell.view_rect;
    h.move_to(r.min.x + on.0, r.min.y + on.1);
    // what's under it read for the part as built now, not a count of frames
    assert!(
        h.settle_hover(),
        "the pointer's pick never came: {}",
        h.gui.shell.status
    );
    h.button(true);
    h.step();
    h.button(false);
    for _ in 0..10 {
        h.step();
    }
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

fn choose(h: &mut Headless, slot: &str, text: &str) {
    // the selection row first, as a person points at it, then what it takes
    assert!(h.click(&format!("fg:tool:slot:{slot}")), "no slot {slot}");
    h.step();
    h.step();
    let name = format!("fg:choice:{slot}:{text}");
    assert!(h.click(&name), "no choice {name}");
    h.step();
    h.step();
}

fn set(h: &mut Headless, param: &str, text: &str) {
    let t = h.gui.shell.fg.as_mut().unwrap().tool.as_mut().unwrap();
    let p = t.params.iter_mut().find(|p| p.name == param).unwrap();
    p.text = text.into();
    h.step();
}

fn picks(h: &Headless, slot: &str) -> Vec<String> {
    let t = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap();
    t.slots.iter().find(|s| s.label == slot).unwrap().picks.clone()
}

/// OK: the tool's statement applied and built; the part's text and what it builds.
fn ok(h: &mut Headless, what: &str) -> (String, forge_lang::Run) {
    // the preview built and the dialog settled, as a person sees it before OK
    assert!(h.until(ready));
    for _ in 0..3 {
        h.step();
    }
    let before = h.gui.fg.as_ref().unwrap().seq;
    assert!(h.click("fg:tool:ok"));
    h.step();
    let p = h.gui.shell.fg.as_ref().unwrap();
    assert!(
        p.tool.is_none(),
        "{what}: the tool stayed open: {:?} {:?}",
        p.tool.as_ref().and_then(|t| t.error.clone()),
        p.refused
    );
    assert!(h.gui.fg.as_ref().unwrap().seq > before);
    assert!(h.until(ready));
    let src = h.gui.fg.as_ref().unwrap().src.clone();
    let run = forge_lang::run(&src, forge_lang::Options::default(), Default::default()).0;
    assert!(run.ok(), "{what}: {:?}\n{src}", run.errors);
    assert!(
        run.measure.as_ref().is_some_and(|m| m.problems == 0),
        "{what}: invalid\n{src}"
    );
    (src, run)
}

fn volume(r: &forge_lang::Run) -> f64 {
    r.measure.as_ref().unwrap().volume
}

fn base_volume() -> f64 {
    60.0 * 60.0 * 20.0 + std::f64::consts::PI * 25.0 * 10.0
}

#[test]
fn fillet_rounds_the_edge_picked_by_its_anchor() {
    let Some((mut h, _)) = open(BLOCK, "fillet") else {
        return;
    };
    button(&mut h, "Fillet");
    point_at(&mut h, [30.0, 0.0, 20.0]);
    assert_eq!(
        picks(&h, "Edges"),
        ["edges(between: (base.end, base.side(plan.outline[0])))"]
    );
    set(&mut h, "r", "3");
    let (src, run) = ok(&mut h, "fillet");
    assert!(
        src.contains("param fillet1.r = 3\nfillet1 = fillet(edges(between: (base.end, base.side(plan.outline[0]))), r)\n"),
        "{src}"
    );
    let cut = (9.0 - std::f64::consts::PI * 9.0 / 4.0) * 60.0;
    assert!(
        (volume(&run) - (base_volume() - cut)).abs() < 1.0,
        "{}",
        volume(&run)
    );
}

#[test]
fn chamfer_cuts_two_edges_picked() {
    let Some((mut h, _)) = open(BLOCK, "chamfer") else {
        return;
    };
    button(&mut h, "Chamfer");
    point_at(&mut h, [30.0, 0.0, 20.0]);
    point_at(&mut h, [60.0, 30.0, 20.0]);
    assert_eq!(picks(&h, "Edges").len(), 2, "{:?}", picks(&h, "Edges"));
    let (src, run) = ok(&mut h, "chamfer");
    assert!(src.contains(" + edges(between: "), "{src}");
    assert!(volume(&run) < base_volume() - 50.0);
}

#[test]
fn shell_opens_the_face_picked() {
    let Some((mut h, _)) = open(PLAIN, "shell") else {
        return;
    };
    button(&mut h, "Shell");
    point_at(&mut h, [5.0, 30.0, 20.0]);
    assert_eq!(picks(&h, "Remove Faces"), ["base.end"]);
    let (src, run) = ok(&mut h, "shell");
    assert!(src.contains("shell1 = shell(t, open: base.end)"), "{src}");
    let walls = 60.0 * 60.0 * 20.0 - 56.0 * 56.0 * 18.0;
    assert!((volume(&run) - walls).abs() < 1.0, "{}", volume(&run));
}

#[test]
fn draft_tilts_a_side_about_a_sketch_plane_chosen() {
    let Some((mut h, _)) = open(BLOCK, "draft") else {
        return;
    };
    button(&mut h, "Draft");
    point_at(&mut h, [30.0, 0.0, 10.0]);
    assert_eq!(picks(&h, "Faces"), ["base.side(plan.outline[0])"]);
    choose(&mut h, "Neutral", "plan");
    let (src, run) = ok(&mut h, "draft");
    assert!(
        src.contains("draft1 = draft(base.side(plan.outline[0]), angle: ang, neutral: plan)"),
        "{src}"
    );
    assert!((volume(&run) - base_volume()).abs() > 10.0);
}

#[test]
fn hole_drills_at_a_sketchs_points() {
    let Some((mut h, _)) = open(BLOCK, "hole") else {
        return;
    };
    button(&mut h, "Hole");
    choose(&mut h, "Points", "marks.points");
    let (src, run) = ok(&mut h, "hole");
    assert!(
        src.contains("hole1 = hole(at: marks.points, d: d, depth: thru)"),
        "{src}"
    );
    let drilled = 2.0 * std::f64::consts::PI * 6.25 * 20.0;
    assert!(
        (volume(&run) - (base_volume() - drilled)).abs() < 1.0,
        "{}",
        volume(&run)
    );
}

#[test]
fn extrude_cuts_a_sketch_loop_chosen() {
    let Some((mut h, _)) = open(BLOCK, "extrude") else {
        return;
    };
    button(&mut h, "Extrude");
    choose(&mut h, "Profile", "marks.sq");
    set(&mut h, "d", "5");
    assert!(h.click("fg:tool:choice:Output:Cut"));
    h.step();
    let (src, run) = ok(&mut h, "extrude");
    assert!(src.contains("extrude1 = cut(marks.sq, d)"), "{src}");
    assert!(
        (volume(&run) - (base_volume() - 180.0)).abs() < 1e-3,
        "{}",
        volume(&run)
    );
}

#[test]
fn revolve_turns_a_loop_about_one_of_its_sides() {
    let Some((mut h, _)) = open(BLOCK, "revolve") else {
        return;
    };
    button(&mut h, "Revolve");
    choose(&mut h, "Profile", "marks.sq");
    choose(&mut h, "Axis", "marks.sq[3]");
    let (src, run) = ok(&mut h, "revolve");
    assert!(
        src.contains("revolve1 = revolve(marks.sq, axis: marks.sq[3], angle: ang)"),
        "{src}"
    );
    // half the cylinder it sweeps stands above the top
    let half = std::f64::consts::PI * 36.0 * 6.0 / 2.0;
    assert!(
        (volume(&run) - (base_volume() + half)).abs() < 1.0,
        "{}",
        volume(&run)
    );
}

#[test]
fn thread_runs_on_the_cylinder_picked() {
    let Some((mut h, _)) = open(BLOCK, "thread") else {
        return;
    };
    button(&mut h, "Thread");
    point_at(&mut h, BOSS_WALL);
    assert_eq!(picks(&h, "Cylinder"), ["boss.side(marks.c)"]);
    let t = h.gui.shell.fg.as_mut().unwrap().tool.as_mut().unwrap();
    t.words[0].1 = "M10".into();
    let (src, run) = ok(&mut h, "thread");
    assert!(
        src.contains("thread1 = thread(face: boss.side(marks.c), spec: \"M10\", length: \"full\")"),
        "{src}"
    );
    assert!(volume(&run) < base_volume());
}

#[test]
fn work_plane_axis_and_point_come_from_picks() {
    let Some((mut h, _)) = open(BLOCK, "work") else {
        return;
    };
    button(&mut h, "Plane");
    point_at(&mut h, [5.0, 30.0, 20.0]);
    let (src, _) = ok(&mut h, "plane");
    assert!(
        src.contains("wplane1 = workplane(offset: base.end, d: d)"),
        "{src}"
    );
    button(&mut h, "Axis");
    point_at(&mut h, BOSS_WALL);
    let (src, _) = ok(&mut h, "axis");
    assert!(
        src.contains("waxis1 = workaxis(along: boss.side(marks.c))"),
        "{src}"
    );
    button(&mut h, "Point");
    point_at(&mut h, [30.0, 0.0, 20.0]);
    let (src, _) = ok(&mut h, "point");
    assert!(
        src.contains("wpoint1 = workpoint(mid: edges(between: (base.end, base.side(plan.outline[0]))))"),
        "{src}"
    );
}

#[test]
fn linear_pattern_copies_a_feature_along_a_faces_normal() {
    let Some((mut h, _)) = open(BLOCK, "pattern") else {
        return;
    };
    button(&mut h, "Rectangular");
    choose(&mut h, "Features", "boss");
    h.click("fg:tool:slot:Direction");
    h.step();
    point_at(&mut h, [30.0, 0.0, 10.0]);
    assert_eq!(picks(&h, "Direction"), ["base.side(plan.outline[0]).normal"]);
    set(&mut h, "n", "2");
    set(&mut h, "step", "20");
    let (src, run) = ok(&mut h, "pattern");
    assert!(
        src.contains("pattern1 = pattern(of: boss, layout: lin(along: base.side(plan.outline[0]).normal, n: n, step: step))"),
        "{src}"
    );
    let boss = std::f64::consts::PI * 25.0 * 10.0;
    assert!(
        (volume(&run) - (base_volume() + boss)).abs() < 1.0,
        "{}",
        volume(&run)
    );
}

#[test]
fn combine_joins_the_bodies_chosen() {
    let Some((mut h, _)) = open(TWO, "combine") else {
        return;
    };
    button(&mut h, "Combine");
    choose(&mut h, "Target", "base");
    choose(&mut h, "Tools", "lump");
    let (src, run) = ok(&mut h, "combine");
    assert!(
        src.contains("combine1 = combine(base, tools: lump, op: \"add\")"),
        "{src}"
    );
    assert_eq!(run.measure.as_ref().unwrap().bodies, 1);
}

#[test]
fn sweep_coil_and_loft_take_sketch_entities() {
    let Some((mut h, _)) = open(SPRING, "swept") else {
        return;
    };
    button(&mut h, "Sweep");
    choose(&mut h, "Profile", "sec.c");
    choose(&mut h, "Path", "rail.route");
    assert_eq!(picks(&h, "Path"), ["rail.route"]);
    let (src, run) = ok(&mut h, "sweep");
    assert!(src.contains("sweep1 = sweep(sec.c, along: rail.route)"), "{src}");
    let v1 = volume(&run);
    button(&mut h, "Coil");
    choose(&mut h, "Profile", "spr.c");
    choose(&mut h, "Axis", "spr.ax");
    let (src, run) = ok(&mut h, "coil");
    assert!(
        src.contains("coil1 = coil(spr.c, axis: spr.ax, pitch: pitch, turns: turns)"),
        "{src}"
    );
    let v2 = volume(&run);
    assert!(v2 > v1);
    button(&mut h, "Loft");
    choose(&mut h, "Profiles", "sec.c");
    choose(&mut h, "Profiles", "top.sq");
    let (src, run) = ok(&mut h, "loft");
    assert!(src.contains("loft1 = loft([sec.c, top.sq])"), "{src}");
    assert!(volume(&run) > v2);
}

#[test]
fn a_sketch_drawn_on_a_face_is_written_in_forge_and_extruded() {
    let Some((mut h, _)) = open(BLOCK, "sketch") else {
        return;
    };
    assert!(h.click("tool:Start Sketch"));
    h.step();
    assert!(h.gui.shell.sketch_pick);
    point_at(&mut h, [5.0, 30.0, 20.0]);
    assert!(h.gui.shell.sketch.is_some(), "{}", h.gui.shell.status);
    assert_eq!(h.gui.shell.sketch.as_ref().unwrap().head, "base.end");
    h.gui.shell.out.push(Action::SketchTool(
        forge_render::shell::sketching::SkTool::Rectangle,
    ));
    h.step();
    point_at(&mut h, [2.0, 20.0, 20.0]);
    point_at(&mut h, [8.0, 30.0, 20.0]);
    let rows = h.gui.shell.sketch.as_ref().unwrap().rows.clone();
    assert!(rows.iter().any(|r| r.starts_with("rect")), "{rows:?}");
    h.gui.shell.out.push(Action::FinishSketch);
    h.step();
    assert!(
        h.gui.shell.sketch.is_none(),
        "{:?}",
        h.gui.shell.fg.as_ref().unwrap().refused
    );
    assert!(h.until(ready));
    let src = h.gui.fg.as_ref().unwrap().src.clone();
    assert!(
        src.contains("sketch sk1(on: base.end) {\n  r1 = rect(corner: (x1, y2), size: (w3, h4))\n}"),
        "{src}"
    );
    for _ in 0..3 {
        h.step();
    }
    button(&mut h, "Extrude");
    choose(&mut h, "Profile", "sk1.r1");
    set(&mut h, "d", "4");
    let (src, run) = ok(&mut h, "extrude on the sketch");
    assert!(src.contains("extrude1 = extrude(sk1.r1, d)"), "{src}");
    // the rect as drawn (its size the snapped one, read back from its params)
    let size = |p: &str| -> f64 {
        let at = src.find(&format!("param sk1.{p} = ")).unwrap() + format!("param sk1.{p} = ").len();
        src[at..].lines().next().unwrap().parse().unwrap()
    };
    let area = size("w3") * size("h4");
    assert!(area > 30.0, "{area}: {rows:?}\n{src}");
    assert!(
        (volume(&run) - (base_volume() + area * 4.0)).abs() < 1e-3,
        "{}",
        volume(&run)
    );
}

#[test]
fn what_a_forge_part_cannot_take_is_refused_typed() {
    let Some((mut h, _)) = open(BLOCK, "refuse") else {
        return;
    };
    // a pick the slot doesn't take
    button(&mut h, "Fillet");
    point_at(&mut h, [5.0, 30.0, 20.0]);
    let e = h
        .gui
        .shell
        .fg
        .as_ref()
        .unwrap()
        .tool
        .as_ref()
        .unwrap()
        .error
        .clone()
        .unwrap();
    assert_eq!(e.kind(), "WrongPick", "{e:?}");
    // OK with nothing picked
    assert!(h.click("fg:tool:ok"));
    h.step();
    let e = h
        .gui
        .shell
        .fg
        .as_ref()
        .unwrap()
        .tool
        .as_ref()
        .unwrap()
        .error
        .clone()
        .unwrap();
    assert_eq!(e.kind(), "MissingPick");
    assert!(h.click("fg:tool:cancel"));
    h.step();
    // units: a Forge part reads in mm, shown in the unit chosen; nothing refuses it
    h.gui.shell.out.push(Action::SetUnits("in"));
    h.step();
    assert_eq!(h.gui.shell.units, "in");
    assert!(h.gui.shell.fg.as_ref().unwrap().refused.is_none());
    // a click with no tool shows what Forge calls it
    point_at(&mut h, [30.0, 0.0, 20.0]);
    let picked = h.gui.shell.fg.as_ref().unwrap().picked.clone().unwrap();
    assert_eq!(
        picked,
        Ok("edges(between: (base.end, base.side(plan.outline[0])))".to_string())
    );
}

/// A cup with a sketch line across it, half way up.
const CUP: &str = "shared L = 60
shared W = 40
shared H = 30
shared t = 2
sketch base(on: xy) {
  r = rect(center: (0, 0), size: (L, W))
  x = line((0, 0), (L / 2, 0), construction: true)
}
box = extrude(base, H)
cup = shell(t, open: box.end)
sketch web(on: rotate(base.plane, about: base.x, by: 90deg)) {
  l = line((0 - L / 4, H / 2), (L / 4, H / 2))
}
";

#[test]
fn rib_fills_from_a_sketch_line_to_the_part() {
    let Some((mut h, _)) = open(CUP, "rib") else {
        return;
    };
    let before = forge_lang::run(CUP, forge_lang::Options::default(), Default::default()).0;
    button(&mut h, "Rib");
    choose(&mut h, "Path", "web.l");
    h.gui.shell.fg.as_mut().unwrap().tool.as_mut().unwrap().flags[0].1 = true;
    let (src, run) = ok(&mut h, "rib");
    assert!(src.contains("rib1 = rib(web.l, t, flip: true)"), "{src}");
    assert!(
        (volume(&run) - volume(&before) - 2.0 * 56.0 * 13.0).abs() < 1e-6,
        "{}",
        volume(&run)
    );
}

#[test]
fn a_feature_renamed_in_the_tree_is_renamed_everywhere() {
    let Some((mut h, _)) = open(BLOCK, "rename") else {
        return;
    };
    // its menu's Rename: the name typed over in its row, Enter
    assert!(h.menu_item("fg:row:boss", "fg:menu:rename:boss"));
    h.step();
    assert!(h.gui.shell.place("fg:renametext").is_some());
    h.text("knob");
    h.step();
    h.key(egui::Key::Enter);
    h.step();
    h.step();
    assert_eq!(
        h.gui.fg.as_ref().unwrap().ops,
        [forge_lang::error::Op::Rename {
            from: "boss".into(),
            to: "knob".into()
        }]
    );
    let src = h.gui.fg.as_ref().unwrap().src.clone();
    assert!(
        src.contains("param knob.h = 10\nknob = extrude(marks.c, h)"),
        "{src}"
    );
    assert!(h.until(ready));
    h.step();
    assert!(
        h.gui
            .shell
            .fg
            .as_ref()
            .unwrap()
            .feats
            .iter()
            .any(|f| f.id == "knob")
    );
}

#[test]
fn a_double_click_on_a_feature_opens_its_dialog_filled_and_ok_replaces_it_in_place() {
    let Some((mut h, _)) = open(BLOCK, "edit") else {
        return;
    };
    let row = h.gui.shell.place("fg:row:boss").expect("the boss row");
    h.move_to(row.center().x, row.center().y);
    h.step();
    for _ in 0..2 {
        h.button(true);
        h.step();
        h.button(false);
        h.step();
    }
    assert!(
        h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some())),
        "no dialog: {}",
        h.gui.shell.status
    );
    let t = h.gui.shell.fg.as_ref().unwrap().tool.clone().unwrap();
    assert_eq!(
        (t.kind, t.edit.as_deref(), t.name.as_str()),
        ("extrude", Some("boss"), "boss")
    );
    assert_eq!(picks(&h, "Profile"), ["marks.c"]);
    // its own param's value, in the field
    assert_eq!(t.params.iter().find(|p| p.name == "d").unwrap().text, "10");
    // no + on an edit
    h.step();
    assert!(h.gui.shell.place("fg:tool:again").is_none());
    set(&mut h, "d", "14");
    let (src, _) = ok(&mut h, "edit boss");
    assert!(
        src.ends_with("param boss.d = 14\nboss = extrude(marks.c, d)\n"),
        "{src}"
    );
    assert!(!src.contains("param boss.h"), "{src}");
    assert!(matches!(
        h.gui.fg.as_ref().unwrap().ops.last(),
        Some(forge_lang::error::Op::ReplaceFeature { id, .. }) if id == "boss"
    ));
    // the base reads a shared param: its field keeps it as written
    assert!(h.menu_item("fg:row:base", "fg:menu:edit:base"));
    assert!(h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some())));
    let t = h.gui.shell.fg.as_ref().unwrap().tool.clone().unwrap();
    assert_eq!(t.params.iter().find(|p| p.name == "d").unwrap().text, "t");
    let (src, _) = ok(&mut h, "edit base");
    assert!(
        src.contains("param base.d = t\nbase = extrude(plan, d)\n"),
        "{src}"
    );
}

#[test]
fn a_pick_made_before_a_build_lands_is_of_what_is_shown_after() {
    // the pointer on the top face, its pick answered against the plain block's mesh
    let Some((mut h, _)) = open(PLAIN, "stale-pick") else {
        return;
    };
    let (cam, size) = h.gui.shell.camera.unwrap();
    let on = cam.pixel_of([5.0, 30.0, 20.0], size.0, size.1).unwrap();
    let r = h.gui.shell.view_rect;
    h.move_to(r.min.x + on.0, r.min.y + on.1);
    for _ in 0..40 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(!h.gui.view.as_ref().unwrap().nav.pick_pending());
    // Shell opened without the pointer moving: its preview (a hollow block, other faces) lands
    h.gui.fg_open_tool("shell");
    let seq = h.gui.fg.as_ref().unwrap().seq;
    assert!(h.until(|g| g.fg.as_ref().is_some_and(|s| s.done >= seq) && ready(g)));
    // and a click where the pointer is takes the face that's there now
    h.button(true);
    h.step();
    h.button(false);
    for _ in 0..10 {
        h.step();
    }
    assert_eq!(
        picks(&h, "Remove Faces"),
        ["base.end"],
        "status: {}",
        h.gui.shell.status
    );
}

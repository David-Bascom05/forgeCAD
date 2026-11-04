//! The 3D Model tab's tools as Inventor has them, on a Forge part: the ribbon button, the
//! dialog's choices, picks in the view or from the browser, values, OK. Each writes its
//! statement through a typed AddFeature op, previews it before OK, and what it builds is measured.

use forge_render::gui::{Gui, Headless};

/// A block with a boss, sketch points, a vertical work axis at its middle and a mid plane.
const PART: &str = "shared L = 60
shared t = 20
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, L))
}
base = extrude(plan, t)
sketch marks(on: base.end) {
  p = pt(45, 45)
  q = pt(15, 15)
  m = pt(30, 30)
  r = pt(15, 45)
  c = circle((45, 45), 5)
  ax = line((10, 30), (50, 30), construction: true)
}
boss = extrude(marks.c, 10)
cax = workaxis(normal_to: base.end, at: marks.m)
half = workplane(mid: (base.side(plan.outline[1]), base.side(plan.outline[3])))
";

const BOSS: f64 = std::f64::consts::PI * 25.0 * 10.0;
const BLOCK: f64 = 60.0 * 60.0 * 20.0;

fn ready(g: &Gui) -> bool {
    g.fg_anchors.is_some()
        && g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn open(tag: &str) -> Option<Headless> {
    let dir = std::env::temp_dir().join(format!("fg-model-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("p.fg");
    std::fs::write(&file, PART).unwrap();
    let font = concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/PublicSans-SemiBold.ttf");
    std::fs::copy(font, dir.join("sans.ttf")).unwrap();
    let mut h = match Headless::new((1700.0, 1000.0), 1.0) {
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

/// Clicks where this place in the world is on screen, once the view knows what's under it.
fn point_at(h: &mut Headless, world: [f32; 3]) {
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
    assert!(h.click(&format!("tool:{tool}")), "no button {tool}");
    assert!(
        h.until(|g| g.shell.fg.as_ref().is_some_and(|p| p.tool.is_some())),
        "{tool}: no dialog: {}",
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

/// The selection row, then what it takes from the browser or a sketch.
fn choose(h: &mut Headless, slot: &str, text: &str) {
    click(h, &format!("fg:tool:slot:{slot}"));
    click(h, &format!("fg:choice:{slot}:{text}"));
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

/// OK once the preview's built: one statement, one undo step; the part's text and its run.
fn ok(h: &mut Headless, what: &str) -> (String, forge_lang::Run) {
    assert!(h.until(ready));
    for _ in 0..3 {
        h.step();
    }
    let t = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap();
    assert!(t.note.is_none(), "{what}: the preview says {:?}", t.note);
    assert!(
        h.gui.fg.as_ref().unwrap().previewing(),
        "{what}: no preview before OK"
    );
    let ops = h.gui.fg.as_ref().unwrap().ops.len();
    assert!(h.click("fg:tool:ok"));
    h.step();
    let p = h.gui.shell.fg.as_ref().unwrap();
    assert!(
        p.tool.is_none(),
        "{what}: the dialog stayed: {:?} {:?}",
        p.tool.as_ref().and_then(|t| t.error.clone()),
        p.refused
    );
    assert_eq!(h.gui.fg.as_ref().unwrap().ops.len(), ops + 1);
    assert!(h.until(ready));
    let s = h.gui.fg.as_ref().unwrap();
    let opts = forge_lang::Options {
        dir: s
            .path
            .as_deref()
            .and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf())),
        ..forge_lang::Options::default()
    };
    let run = forge_lang::run(&s.src, opts, Default::default()).0;
    assert!(run.ok(), "{what}: {:?}\n{}", run.errors, s.src);
    assert!(
        run.measure.as_ref().is_some_and(|m| m.problems == 0),
        "{what}: invalid\n{}",
        s.src
    );
    (s.src.clone(), run)
}

fn volume(r: &forge_lang::Run) -> f64 {
    r.measure.as_ref().unwrap().volume
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

#[test]
fn circular_pattern_copies_a_boss_about_a_work_axis() {
    let Some(mut h) = open("circ") else { return };
    button(&mut h, "Circular");
    choose(&mut h, "Features", "boss");
    choose(&mut h, "Rotation Axis", "cax");
    set(&mut h, "n", "4");
    let (src, run) = ok(&mut h, "circular");
    assert!(
        src.contains("pattern1 = pattern(of: boss, layout: circ(about: cax, n: n))"),
        "{src}"
    );
    assert!(near(volume(&run), BLOCK + 4.0 * BOSS, 1.0), "{}", volume(&run));
    // undo takes the whole pattern back in one step
    h.gui.shell.out.push(forge_render::shell::Action::Undo);
    h.step();
    assert!(!h.gui.fg.as_ref().unwrap().src.contains("pattern1"));
}

#[test]
fn mirror_reflects_a_feature_in_a_plane() {
    let Some(mut h) = open("mirror") else { return };
    button(&mut h, "Mirror");
    choose(&mut h, "Features", "boss");
    choose(&mut h, "Mirror Plane", "half");
    let (src, run) = ok(&mut h, "mirror");
    assert!(
        src.contains("pattern1 = pattern(of: boss, layout: mirror(on: half))"),
        "{src}"
    );
    assert!(near(volume(&run), BLOCK + 2.0 * BOSS, 1.0), "{}", volume(&run));
}

#[test]
fn sketch_driven_pattern_puts_copies_on_sketch_points() {
    let Some(mut h) = open("places") else { return };
    button(&mut h, "Sketch");
    choose(&mut h, "Features", "boss");
    choose(&mut h, "Points", "marks.p");
    choose(&mut h, "Points", "marks.q");
    let (src, run) = ok(&mut h, "sketch pattern");
    assert!(
        src.contains("pattern1 = pattern(of: boss, layout: places(at: [marks.p, marks.q]))"),
        "{src}"
    );
    assert!(near(volume(&run), BLOCK + 2.0 * BOSS, 1.0), "{}", volume(&run));
}

#[test]
fn emboss_raises_and_engraves_text_on_a_face() {
    let Some(mut h) = open("emboss") else { return };
    button(&mut h, "Emboss");
    // the font beside the part is offered and taken
    let t = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap();
    assert_eq!(t.word("font"), "sans.ttf");
    point_at(&mut h, [10.0, 50.0, 20.0]);
    assert_eq!(picks(&h, "Face"), ["base.end"]);
    choose(&mut h, "Position", "marks.q");
    let (src, run) = ok(&mut h, "emboss");
    assert!(
        src.contains("emboss1 = text(\"FORGE\", font: \"sans.ttf\", size: size, depth: depth, on: base.end, at: marks.q)"),
        "{src}"
    );
    let raised = volume(&run) - (BLOCK + BOSS);
    assert!(raised > 10.0, "{raised}");
    button(&mut h, "Emboss");
    click(&mut h, "fg:tool:choice:Type:Engrave");
    point_at(&mut h, [10.0, 50.0, 20.0]);
    choose(&mut h, "Position", "marks.m");
    let (src, run) = ok(&mut h, "engrave");
    assert!(src.contains("at: marks.m, cut: true)"), "{src}");
    assert!(volume(&run) < BLOCK + BOSS + raised - 10.0);
}

#[test]
fn emboss_wraps_text_round_a_cylinder() {
    let Some(mut h) = open("wrap") else { return };
    button(&mut h, "Emboss");
    point_at(&mut h, [10.0, 50.0, 20.0]);
    choose(&mut h, "Position", "marks.p");
    let t = h.gui.shell.fg.as_mut().unwrap().tool.as_mut().unwrap();
    t.flags[0].1 = true;
    t.words[0].1 = "OK".into();
    set(&mut h, "wrap", "5");
    set(&mut h, "size", "4");
    set(&mut h, "depth", "0.5");
    let (src, run) = ok(&mut h, "wrapped");
    assert!(src.contains("at: marks.p, wrap: wrap)"), "{src}");
    assert!(volume(&run) > BLOCK + BOSS + 1.0);
}

#[test]
fn split_part_and_trim_solid_cut_on_a_work_plane() {
    let Some(mut h) = open("split") else { return };
    button(&mut h, "Split");
    choose(&mut h, "Split Tool", "half");
    let (src, run) = ok(&mut h, "split");
    assert!(src.contains("split1 = split(by: half, keep: \"both\")"), "{src}");
    assert_eq!(run.measure.as_ref().unwrap().bodies, 2);
    h.gui.shell.out.push(forge_render::shell::Action::Undo);
    h.step();
    button(&mut h, "Split");
    click(&mut h, "fg:tool:choice:Method:Trim Solid");
    choose(&mut h, "Split Tool", "half");
    let (src, run) = ok(&mut h, "trim");
    assert!(src.contains("split1 = split(by: half, keep: \"plus\")"), "{src}");
    assert_eq!(run.measure.as_ref().unwrap().bodies, 1);
    assert!(volume(&run) < BLOCK);
}

#[test]
fn thicken_adds_a_slab_on_the_faces_picked() {
    let Some(mut h) = open("thicken") else { return };
    button(&mut h, "Thicken");
    point_at(&mut h, [30.0, 0.0, 10.0]);
    assert_eq!(picks(&h, "Faces"), ["base.side(plan.outline[0])"]);
    let (src, run) = ok(&mut h, "thicken");
    assert!(
        src.contains("thicken1 = thicken(base.side(plan.outline[0]), t, side: \"out\")"),
        "{src}"
    );
    assert!(
        near(volume(&run), BLOCK + BOSS + 60.0 * 20.0 * 2.0, 1.0),
        "{}",
        volume(&run)
    );
}

#[test]
fn direct_edit_moves_sizes_and_deletes_faces() {
    let Some(mut h) = open("direct") else { return };
    button(&mut h, "Direct");
    point_at(&mut h, [30.0, 0.0, 10.0]);
    click(&mut h, "fg:tool:slot:Direction");
    point_at(&mut h, [30.0, 0.0, 10.0]);
    assert_eq!(picks(&h, "Direction"), ["base.side(plan.outline[0]).normal"]);
    let (src, run) = ok(&mut h, "move face");
    assert!(
        src.contains(
            "direct1 = move_faces(base.side(plan.outline[0]), along: base.side(plan.outline[0]).normal, d: d)"
        ),
        "{src}"
    );
    assert!(
        near(volume(&run), BLOCK + BOSS + 60.0 * 20.0 * 5.0, 1.0),
        "{}",
        volume(&run)
    );
    button(&mut h, "Direct");
    click(&mut h, "fg:tool:choice:Action:Size");
    point_at(&mut h, [60.0, 30.0, 10.0]);
    set(&mut h, "d", "2");
    let (src, run) = ok(&mut h, "size face");
    assert!(
        src.contains("direct2 = offset_faces(base.side(plan.outline[1]), d)"),
        "{src}"
    );
    // the side, now 65 long, out 2
    assert!(
        near(
            volume(&run),
            BLOCK + BOSS + 60.0 * 20.0 * 5.0 + 65.0 * 20.0 * 2.0,
            1.0
        ),
        "{}",
        volume(&run)
    );
    button(&mut h, "Delete Face");
    point_at(&mut h, [45.0, 45.0, 30.0]);
    point_at(&mut h, [47.5, 40.67, 25.0]);
    let t = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap();
    assert_eq!(
        picks(&h, "Faces"),
        ["boss.end", "boss.side(marks.c)"],
        "{:?} {:?} {:?}",
        t.error,
        t.note,
        h.gui.shell.fg.as_ref().unwrap().picked
    );
    let (src, run) = ok(&mut h, "delete face");
    assert!(
        src.contains("direct3 = delete_faces(boss.end + boss.side(marks.c))"),
        "{src}"
    );
    // the boss taken away, the top closed over where it stood
    assert!(
        near(volume(&run), BLOCK + 60.0 * 20.0 * 5.0 + 65.0 * 20.0 * 2.0, 1.0),
        "{}",
        volume(&run)
    );
}

#[test]
fn replace_face_puts_the_top_on_a_work_plane() {
    let Some(mut h) = open("replace") else { return };
    button(&mut h, "Plane");
    point_at(&mut h, [10.0, 50.0, 20.0]);
    set(&mut h, "d", "5");
    ok(&mut h, "plane");
    button(&mut h, "Replace");
    point_at(&mut h, [10.0, 50.0, 20.0]);
    choose(&mut h, "New Face", "wplane1");
    let (src, run) = ok(&mut h, "replace");
    assert!(
        src.contains("direct1 = replace_face(base.end, with: wplane1)"),
        "{src}"
    );
    // the boss stands half in the new top
    assert!(
        near(volume(&run), BLOCK + 60.0 * 60.0 * 5.0 + BOSS / 2.0, 2.0),
        "{}",
        volume(&run)
    );
}

#[test]
fn work_features_by_every_method_inventor_has() {
    let Some(mut h) = open("work") else { return };
    button(&mut h, "Plane");
    click(&mut h, "fg:tool:choice:Method:Midplane");
    choose(&mut h, "Plane 1", "plan");
    choose(&mut h, "Plane 2", "marks");
    let (src, _) = ok(&mut h, "midplane");
    assert!(src.contains("wplane1 = workplane(mid: (plan, marks))"), "{src}");
    button(&mut h, "Plane");
    click(&mut h, "fg:tool:choice:Method:3 Points");
    for p in ["marks.p", "marks.q", "marks.r"] {
        choose(&mut h, "Points", p);
    }
    let (src, _) = ok(&mut h, "3 points");
    assert!(
        src.contains("wplane2 = workplane(through: (marks.p, marks.q, marks.r))"),
        "{src}"
    );
    button(&mut h, "Axis");
    click(&mut h, "fg:tool:choice:Method:2 Planes");
    choose(&mut h, "Plane 1", "half");
    choose(&mut h, "Plane 2", "plan");
    let (src, _) = ok(&mut h, "axis");
    assert!(src.contains("waxis1 = workaxis(meet: (half, plan))"), "{src}");
    button(&mut h, "Point");
    click(&mut h, "fg:tool:choice:Method:On Point");
    choose(&mut h, "Point", "marks.q");
    let (src, _) = ok(&mut h, "point");
    assert!(src.contains("wpoint1 = workpoint(at: marks.q)"), "{src}");
    button(&mut h, "UCS");
    choose(&mut h, "Plane", "half");
    let (src, _) = ok(&mut h, "ucs");
    assert!(src.contains("ucs1 = ucs(on: half)"), "{src}");
}

#[test]
fn copy_and_move_bodies_by_a_slide() {
    let Some(mut h) = open("bodies") else { return };
    button(&mut h, "Copy");
    choose(&mut h, "Bodies", "base");
    click(&mut h, "fg:tool:slot:Direction");
    point_at(&mut h, [60.0, 30.0, 10.0]);
    set(&mut h, "d", "80");
    let (src, run) = ok(&mut h, "copy");
    assert!(
        src.contains(
            "copybody1 = copybody(of: base, moves: [slide(along: base.side(plan.outline[1]).normal, by: d)])"
        ),
        "{src}"
    );
    assert_eq!(run.measure.as_ref().unwrap().bodies, 2);
    assert!(near(volume(&run), 2.0 * (BLOCK + BOSS), 2.0), "{}", volume(&run));
}

#[test]
fn fillet_by_type_and_hole_by_type() {
    let Some(mut h) = open("types") else { return };
    button(&mut h, "Fillet");
    click(&mut h, "fg:tool:choice:Type:Variable");
    point_at(&mut h, [30.0, 0.0, 20.0]);
    let (src, run) = ok(&mut h, "variable fillet");
    assert!(
        src.contains(
            "fillet1 = fillet(edges(between: (base.end, base.side(plan.outline[0]))), vary(r_start, r_end))"
        ),
        "{src}"
    );
    assert!(volume(&run) < BLOCK + BOSS - 10.0);
    button(&mut h, "Hole");
    click(&mut h, "fg:tool:choice:Type:Clearance");
    choose(&mut h, "Points", "marks.q");
    let (src, run) = ok(&mut h, "clearance hole");
    assert!(
        src.contains("hole1 = hole(at: marks.q, screw: \"M5\", fit: \"normal\", depth: thru)"),
        "{src}"
    );
    let drilled = std::f64::consts::PI * 2.75 * 2.75 * 20.0;
    let before = volume(&run) + drilled;
    button(&mut h, "Hole");
    click(&mut h, "fg:tool:choice:Seat:Counterbore");
    choose(&mut h, "Points", "marks.m");
    let (src, run) = ok(&mut h, "counterbore");
    assert!(
        src.contains("hole2 = hole(at: marks.m, d: d, cbore: (cbore_d, cbore_depth), depth: thru)"),
        "{src}"
    );
    let cb = std::f64::consts::PI * (6.25 * 15.0 + 25.0 * 5.0);
    assert!(near(volume(&run), before - drilled - cb, 1.0), "{}", volume(&run));
}

#[test]
fn extrude_symmetric_new_solid_and_through_all() {
    let Some(mut h) = open("extents") else { return };
    button(&mut h, "Extrude");
    choose(&mut h, "Profile", "marks.c");
    click(&mut h, "fg:tool:choice:Extents:Through All");
    click(&mut h, "fg:tool:choice:Output:Cut");
    let (src, run) = ok(&mut h, "thru all");
    assert!(src.contains("extrude1 = cut(marks.c, thru_all)"), "{src}");
    assert!(near(volume(&run), BLOCK - BOSS * 2.0, 1.0), "{}", volume(&run));
}

fn sketch_tool(h: &mut Headless, t: forge_render::shell::sketching::SkTool) {
    h.gui.shell.out.push(forge_render::shell::Action::SketchTool(t));
    h.step();
}

fn sketch_click(h: &mut Headless, at: (f64, f64)) {
    h.gui.shell.sketch.as_mut().unwrap().click(at, 0.5);
    h.step();
}

#[test]
fn a_slot_constrained_and_text_on_a_face_become_forge_then_cut_and_emboss() {
    use forge_render::shell::sketching::SkTool as T;
    let Some(mut h) = open("slot") else { return };
    assert!(h.click("tool:Start Sketch"));
    h.step();
    point_at(&mut h, [10.0, 50.0, 20.0]);
    assert_eq!(h.gui.shell.sketch.as_ref().unwrap().head, "base.end");
    sketch_tool(&mut h, T::SlotCenter);
    for p in [(30.0, 20.0), (40.0, 20.0), (30.0, 24.0)] {
        sketch_click(&mut h, p);
    }
    // the slot's straight sides made level, and the two arcs equal
    sketch_tool(&mut h, T::Horizontal);
    sketch_click(&mut h, (30.0, 24.0));
    sketch_tool(&mut h, T::Equal);
    sketch_click(&mut h, (16.0, 20.0));
    sketch_click(&mut h, (44.0, 20.0));
    sketch_tool(&mut h, T::Text);
    sketch_click(&mut h, (10.0, 40.0));
    let m = h.gui.shell.sketch.as_ref().unwrap();
    assert!(m.error.is_none(), "{:?} {:?}", m.error, m.rows);
    assert_eq!(m.rows.len(), 11, "{:?}", m.rows);
    h.gui.shell.out.push(forge_render::shell::Action::FinishSketch);
    h.step();
    assert!(
        h.gui.shell.sketch.is_none(),
        "{:?}",
        h.gui.shell.fg.as_ref().unwrap().refused
    );
    // the text goes on to Emboss, on the sketch at its point
    let t = h
        .gui
        .shell
        .fg
        .as_ref()
        .unwrap()
        .tool
        .as_ref()
        .expect("Emboss opens");
    assert_eq!(t.kind, "emboss");
    assert_eq!(picks(&h, "Position"), ["sk1.p1"]);
    let t = h.gui.shell.fg.as_mut().unwrap().tool.as_mut().unwrap();
    t.words[0].1 = "AB".into();
    let (src, run) = ok(&mut h, "emboss from the sketch");
    assert!(src.contains("tangent(l1, a1)"), "{src}");
    assert!(
        src.contains("equal(a1, a2)") || src.contains("equal(a2, a1)"),
        "{src}"
    );
    assert!(
        src.contains(
            "emboss1 = text(\"AB\", font: \"sans.ttf\", size: size, depth: depth, on: sk1, at: sk1.p1)"
        ),
        "{src}"
    );
    let embossed = volume(&run);
    button(&mut h, "Extrude");
    choose(&mut h, "Profile", "sk1");
    click(&mut h, "fg:tool:choice:Output:Cut");
    set(&mut h, "d", "2");
    let (src, run) = ok(&mut h, "slot cut");
    let slot = 2.0 * 4.0 * 20.0 + std::f64::consts::PI * 16.0;
    assert!(
        near(volume(&run), embossed - 2.0 * slot, 1e-3),
        "{}\n{src}",
        embossed - volume(&run)
    );
}

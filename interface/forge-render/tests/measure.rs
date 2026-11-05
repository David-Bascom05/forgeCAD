//! Measure in the app driven as a person drives it: the pointer moved and clicked on the view
//! (no picks set by hand), orbiting, other tabs, Esc. GPU tests: skipped where there is none.

use forge_render::gui::{Gui, Headless};
use forge_render::render::Entity;
use forge_render::shell::Action;
use forge_render::shell::inspect::InspectCmd;

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

/// The demo part, opened from a copy of its own.
fn open(tag: &str) -> Option<Headless> {
    let dir = std::env::temp_dir().join(format!("fg-measure-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // the demo's folder: its font is beside it
    let demo = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/demo"));
    for e in std::fs::read_dir(demo).unwrap().flatten() {
        if e.path().is_file() {
            std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
        }
    }
    let file = dir.join("part_tools.fg");
    let mut h = match Headless::new((1400.0, 900.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    assert!(h.until(ready), "never ready: {}", h.gui.shell.status);
    for _ in 0..5 {
        h.step();
    }
    Some(h)
}

/// The place on screen of a point of the part, and what the pointer's over there once the view's
/// pick has answered.
fn hover(h: &mut Headless, world: [f32; 3]) -> ((f32, f32), Option<(u32, Entity)>) {
    assert!(h.until(|g| g.view.as_ref().is_some_and(|v| !v.nav.moving())));
    let (cam, size) = h.gui.shell.camera.unwrap();
    let on = cam.pixel_of(world, size.0, size.1).unwrap();
    let r = h.gui.shell.view_rect;
    let at = (r.min.x + on.0, r.min.y + on.1);
    h.move_to(at.0, at.1);
    for waited in 0..200 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let v = h.gui.view.as_ref().unwrap();
        if waited > 10 && !v.nav.pick_pending() && v.nav.marks().iter().any(|m| !m.selected) {
            break;
        }
    }
    let got = h.gui.view.as_ref().unwrap().nav.marks().into_iter().rfind(|m| !m.selected);
    (at, got.map(|m| (m.placement, m.entity)))
}

/// Points on the demo part: the base's top face, the boss's top.
const TOP: [f32; 3] = [30.0, 22.0, 12.0];
const BOSS_TOP: [f32; 3] = [4.0, 4.0, 20.0];

fn picks(h: &Headless) -> Vec<(u32, Entity)> {
    h.gui.shell.inspect.measure.as_ref().map_or(vec![], |m| m.marked())
}

fn measure(h: &mut Headless) {
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Measure));
    // till the panel's laid out and its buttons named
    for _ in 0..10 {
        h.step();
        if h.gui.shell.place("measure:filter:Any").is_some() {
            break;
        }
    }
    assert!(h.gui.shell.inspect.measure.is_some());
}

/// A click that doesn't wait for the hover's pick: pressed and let go before the view has said
/// what's there, as a quick hand on a busy view clicks.
fn quick_click(h: &mut Headless, x: f32, y: f32) {
    h.move_to(x, y);
    h.button(true);
    h.button(false);
    for _ in 0..6 {
        h.step();
    }
}

#[test]
fn a_quick_click_on_a_face_is_a_pick_not_a_click_on_nothing() {
    let Some(mut h) = open("quick") else { return };
    measure(&mut h);
    let (at, want) = hover(&mut h, TOP);
    let want = want.expect("the top face under the pointer");
    let (at2, want2) = hover(&mut h, BOSS_TOP);
    let want2 = want2.expect("the boss's top under the pointer");
    assert!(matches!(want.1, Entity::Face(_)) && matches!(want2.1, Entity::Face(_)));
    // away, then straight onto the face and click
    h.move_to(5.0, 5.0);
    h.step();
    quick_click(&mut h, at.0, at.1);
    assert_eq!(picks(&h), vec![want]);
    quick_click(&mut h, at2.0, at2.1);
    assert_eq!(picks(&h), vec![want, want2]);
}

/// A click that waits for the hover's pick, as a steady hand clicks.
fn click(h: &mut Headless, world: [f32; 3]) -> (u32, Entity) {
    let (at, e) = hover(h, world);
    h.button(true);
    h.step();
    h.button(false);
    for _ in 0..6 {
        h.step();
    }
    let _ = at;
    e.expect("something under the pointer")
}

fn reading(h: &mut Headless) -> forge_agent::measure::Reading {
    assert!(h.until(|g| {
        g.shell
            .inspect
            .measure
            .as_ref()
            .is_some_and(|m| m.reading.is_some() && !m.busy)
    }));
    // a frame to show it: its rows and marks on screen
    h.step();
    h.step();
    let m = h.gui.shell.inspect.measure.as_ref().unwrap();
    m.reading.clone().unwrap().unwrap()
}

#[test]
fn measuring_feeds_no_other_pick_the_browser_or_a_feature_dialog() {
    let Some(mut h) = open("leak") else { return };
    measure(&mut h);
    let e = click(&mut h, [0.0, -30.0, 12.0]);
    assert!(matches!(e.1, Entity::Edge(_)), "{e:?}");
    assert_eq!(picks(&h), vec![e]);
    let p = h.gui.shell.fg.as_ref().unwrap();
    assert!(p.picked.is_none() && h.gui.shell.selected.is_none() && h.gui.shell.dim.is_none());
    assert!(!h.gui.shell.status.starts_with("Picked"), "{}", h.gui.shell.status);
    // a Fillet dialog waiting for edges takes none of Measure's
    h.gui.fg_open_tool("fillet");
    h.step();
    let f = click(&mut h, TOP);
    let tool = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap();
    assert!(tool.slots.iter().all(|s| s.picks.is_empty()), "the fillet took Measure's pick");
    assert!(tool.error.is_none(), "the fillet was given Measure's face: {:?}", tool.error);
    assert_eq!(picks(&h), vec![e, f]);
    // what the view marks is what's measured
    assert_eq!(h.gui.view.as_ref().unwrap().nav.selected, vec![e, f]);
}

#[test]
fn esc_clears_the_picks_then_ends_measure() {
    let Some(mut h) = open("esc") else { return };
    measure(&mut h);
    click(&mut h, TOP);
    let _ = reading(&mut h);
    h.key(egui::Key::Escape);
    h.step();
    h.step();
    let m = h.gui.shell.inspect.measure.as_ref().expect("still measuring");
    assert!(m.picks.is_empty() && m.reading.is_none());
    h.key(egui::Key::Escape);
    h.step();
    h.step();
    assert!(h.gui.shell.inspect.measure.is_none());
}

#[test]
fn a_window_drag_while_measuring_changes_nothing_picked() {
    let Some(mut h) = open("window") else { return };
    measure(&mut h);
    let a = click(&mut h, TOP);
    let b = click(&mut h, BOSS_TOP);
    assert_eq!(picks(&h), vec![a, b]);
    let r = h.gui.shell.view_rect;
    h.move_to(r.left() + 20.0, r.bottom() - 20.0);
    h.step();
    h.button(true);
    h.step();
    for k in 1..=10 {
        h.move_to(r.left() + 20.0 + 60.0 * k as f32, r.bottom() - 20.0 - 40.0 * k as f32);
        h.step();
    }
    h.button(false);
    // not even for a frame
    for _ in 0..6 {
        h.step();
        assert_eq!(picks(&h), vec![a, b]);
        assert_eq!(h.gui.view.as_ref().unwrap().nav.selected, vec![a, b]);
    }
}

#[test]
fn a_rebuild_keeps_the_reading_on_screen_and_reads_it_again() {
    let Some(mut h) = open("rebuild") else { return };
    measure(&mut h);
    let a = click(&mut h, TOP);
    let b = click(&mut h, BOSS_TOP);
    let _ = (a, b);
    let before = reading(&mut h);
    let gap = before.get(forge_agent::measure::Quantity::Distance).unwrap();
    assert!((gap - 8.0).abs() < 1e-6, "{gap}");
    // the boss 10 high: the same faces, further apart
    h.gui
        .fg_apply(forge_lang::error::Op::SetParam {
            param: forge_lang::error::ParamKey::parse("extrude2.d").unwrap(),
            value: forge_lang::error::Literal {
                value: 10.0,
                dim: forge_lang::values::Dim::Length,
            },
        })
        .unwrap();
    let mut blank = 0;
    for _ in 0..2000 {
        h.step();
        let m = h.gui.shell.inspect.measure.as_ref().unwrap();
        if m.reading.is_none() {
            blank += 1;
        }
        if let Some(Ok(r)) = &m.reading
            && !m.busy
            && r.get(forge_agent::measure::Quantity::Distance)
                .is_some_and(|d| (d - 10.0).abs() < 1e-6)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let after = reading(&mut h);
    let gap = after.get(forge_agent::measure::Quantity::Distance).unwrap();
    assert!((gap - 10.0).abs() < 1e-6, "{gap}");
    assert_eq!(blank, 0, "the reading went blank for {blank} frames");
}

/// A value of a `measure` answer, by its label, in mm (or mm², degrees).
#[track_caller]
fn value(text: &str, label: &str) -> f64 {
    let line = text
        .lines()
        .find(|l| l.starts_with(&format!("{label}: ")))
        .unwrap_or_else(|| panic!("no {label} in\n{text}"));
    line[label.len() + 2..]
        .split([' ', '°'])
        .next()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| panic!("{line}"))
}

fn measured(h: &mut Headless, a: &str, b: Option<&str>) -> String {
    h.gui.measure(a, b).unwrap_or_else(|e| panic!("measure({a}, {b:?}): {e}"))
}

#[track_caller]
fn close(got: f64, want: f64) {
    // the panel's 3 places
    assert!((got - want).abs() < 6e-4, "{got} != {want}");
}

#[test]
fn every_kind_of_reading_on_the_demo_part_is_exact() {
    use std::f64::consts::PI;
    let Some(mut h) = open("exact") else { return };
    // two faces: the base 12 thick, parallel
    let t = measured(&mut h, "extrude1.end", Some("extrude1.start"));
    close(value(&t, "Minimum distance"), 12.0);
    close(value(&t, "Delta Z"), 12.0);
    close(value(&t, "Angle"), 0.0);
    // a face: the bottom, less the four 5 mm holes through it; its perimeter round them too
    let t = measured(&mut h, "extrude1.start", None);
    close(value(&t, "Area"), 4800.0 - 4.0 * PI * 2.5 * 2.5);
    close(value(&t, "Perimeter"), 280.0 + 4.0 * PI * 5.0);
    // the boss's top, inside its 2 mm round
    let t = measured(&mut h, "extrude2.end", None);
    close(value(&t, "Area"), PI * 100.0);
    close(value(&t, "Perimeter"), 20.0 * PI);
    // cylinders: the boss, and the counterbore 9 across, 3 deep
    let t = measured(&mut h, "extrude2.side(sk2.c1)", None);
    close(value(&t, "Diameter"), 24.0);
    close(value(&t, "Area"), 2.0 * PI * 12.0 * 6.0);
    let t = measured(&mut h, "hole1.bore", None);
    close(value(&t, "Radius"), 4.5);
    close(value(&t, "Area"), 2.0 * PI * 4.5 * 3.0);
    // the counterbore's seat: a ring
    close(value(&measured(&mut h, "hole1.seat", None), "Area"), PI * (4.5 * 4.5 - 2.5 * 2.5));
    // an edge: the base's back bottom edge, and the loop round the bottom it's in
    let back = "edges(between: (extrude1.side(sk1.r1[2]), extrude1.start))";
    let front = "edges(between: (extrude1.side(sk1.r1[0]), extrude1.start))";
    let t = measured(&mut h, back, None);
    close(value(&t, "Length"), 80.0);
    close(value(&t, "Loop length"), 280.0);
    // two edges: 60 apart, the farthest ends 100 apart (80 by 60)
    let t = measured(&mut h, back, Some(front));
    close(value(&t, "Minimum distance"), 60.0);
    close(value(&t, "Maximum distance"), 100.0);
    // a side and the bottom: square
    close(value(&measured(&mut h, "extrude1.side(sk1.r1[1])", Some("extrude1.start")), "Angle"), 90.0);
    // origin items: the XY plane to the boss's top, the Z axis to the hole 20 off it
    close(value(&measured(&mut h, "xy", Some("extrude2.end")), "Minimum distance"), 20.0);
    let t = measured(&mut h, "z", Some("hole1.bore"));
    close(value(&t, "Axis to axis"), 20.0);
    close(value(&t, "Minimum distance"), 20.0 - 4.5);
    // the whole body: its volume and area are the part's own, its box 80 by 60 by 20
    let t = measured(&mut h, "body", None);
    let dir = h.gui.fg.as_ref().unwrap().path.clone().unwrap();
    let src = std::fs::read_to_string(&dir).unwrap();
    let opts = forge_lang::Options {
        dir: std::path::Path::new(&dir).parent().map(|d| d.to_path_buf()),
        ..forge_lang::Options::default()
    };
    let run = forge_lang::run(&src, opts, Default::default()).0;
    let whole = run.measure.as_ref().unwrap();
    close(value(&t, "Volume"), whole.volume);
    close(value(&t, "Area"), whole.area);
    close(value(&t, "Box X"), 80.0);
    close(value(&t, "Box Y"), 60.0);
    close(value(&t, "Box Z"), 20.0);
}

#[test]
fn in_the_rack_occurrences_are_measured_where_they_stand() {
    let rack = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/rack18u/asm/rack.fg");
    let mut h = match Headless::new((1400.0, 900.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    h.gui.open(rack);
    assert!(h.until(ready), "never ready: {}", h.gui.shell.status);
    for _ in 0..10 {
        h.step();
    }
    // the rails: 890 tall, 8 to 898 (SPEC 6.3), the same part each
    for r in ["rail_lf", "rail_rr"] {
        let t = measured(&mut h, &format!("{r}/body"), None);
        close(value(&t, "Box Z"), 890.0);
    }
    // the rear right rail is the front left turned half round about the rack's middle (y 412.5)
    let p = |t: &str| (value(t, "X"), value(t, "Y"), value(t, "Z"));
    let lf = p(&measured(&mut h, "rail_lf/vertex:0", None));
    let rr = p(&measured(&mut h, "rail_rr/vertex:0", None));
    close(rr.0, -lf.0);
    close(rr.1, 825.0 - lf.1);
    close(rr.2, lf.2);
    let t = measured(&mut h, "rail_lf/vertex:0", Some("rail_rr/vertex:0"));
    close(value(&t, "Minimum distance"), ((rr.0 - lf.0).powi(2) + (rr.1 - lf.1).powi(2)).sqrt());
    // the pans: the top pan's outer face at 906 (SPEC 5.2), so the gap is 906 less both pans
    let hb = value(&measured(&mut h, "base_pan/body", None), "Box Z");
    let ht = value(&measured(&mut h, "top_pan/body", None), "Box Z");
    let t = measured(&mut h, "base_pan/body", Some("top_pan/body"));
    close(value(&t, "Minimum distance"), 906.0 - hb - ht);
    close(value(&t, "Delta Z"), 906.0 - hb - ht);
}

fn place(h: &Headless, name: &str) -> egui::Rect {
    h.gui
        .shell
        .place(name)
        .unwrap_or_else(|| panic!("no {name} on screen"))
}

#[test]
fn the_dimensions_stay_on_the_model_as_it_turns_and_sit_under_the_panel() {
    use forge_agent::measure::Shown;
    let Some(mut h) = open("marks") else { return };
    measure(&mut h);
    click(&mut h, TOP);
    let r = reading(&mut h);
    let centre = r
        .shown
        .iter()
        .find_map(|s| match s {
            Shown::Face(_, c) => Some(*c),
            _ => None,
        })
        .expect("the face tinted");
    let on_screen = |h: &Headless| {
        let (cam, size) = h.gui.shell.camera.unwrap();
        let p = cam
            .pixel_of([centre.x as f32, centre.y as f32, centre.z as f32], size.0, size.1)
            .unwrap();
        let v = h.gui.shell.view_rect;
        egui::pos2(v.min.x + p.0, v.min.y + p.1)
    };
    // the area's label stands off the face's middle by its leader, wherever the view turns
    let before = place(&h, "measure:mark:Area");
    let anchor = on_screen(&h);
    assert!((before.center() - (anchor + egui::vec2(28.0, -28.0))).length() < 1.0);
    // turned and moved off: the face's middle is the view's, so a turn alone hardly moves it
    if let Some(v) = &mut h.gui.view {
        v.nav.camera.yaw += 0.6;
        v.nav.camera.pitch -= 0.2;
        v.nav.camera.target[0] += 25.0;
        v.mark_dirty();
    }
    h.step();
    h.step();
    let after = place(&h, "measure:mark:Area");
    let anchor = on_screen(&h);
    assert!((after.center() - before.center()).length() > 5.0, "the label didn't move with the view");
    assert!((after.center() - (anchor + egui::vec2(28.0, -28.0))).length() < 1.0);
    // the panel dragged over the label: the panel's what shows there
    let bodies = place(&h, "measure:filter:Bodies");
    let blank = egui::pos2(bodies.right() + 30.0, bodies.center().y);
    // dragged by its own background (a label's text would take the drag to select itself)
    let grip = blank;
    let by = after.center() - blank;
    h.move_to(grip.x, grip.y);
    h.step();
    h.button(true);
    h.step();
    for k in 1..=8 {
        let p = grip + by * (k as f32 / 8.0);
        h.move_to(p.x, p.y);
        h.step();
    }
    h.button(false);
    for _ in 0..4 {
        h.step();
    }
    let label = place(&h, "measure:mark:Area").center();
    let bodies = place(&h, "measure:filter:Bodies");
    let blank = egui::pos2(bodies.right() + 30.0, bodies.center().y);
    assert!((blank - label).length() < 3.0, "the panel didn't land on the label: {blank:?} {label:?}");
    let px = h.gui_pixels();
    let at = |p: egui::Pos2| px.at(p.x as usize, p.y as usize);
    let s = h.gui.shell.theme.surface;
    assert_eq!(at(label), [s.r(), s.g(), s.b()], "the label shows through the panel");
}

#[test]
fn the_selection_priority_picks_faces_edges_points_or_whole_bodies() {
    use forge_render::shell::inspect::{Filter, MPick};
    let Some(mut h) = open("filter") else { return };
    measure(&mut h);
    let set = |h: &mut Headless, f: Filter| {
        assert!(h.click(&format!("measure:filter:{}", f.word())));
        h.step();
        h.step();
    };
    let first = |h: &Headless| h.gui.shell.inspect.measure.as_ref().unwrap().picks.first().cloned();
    set(&mut h, Filter::Edges);
    click(&mut h, TOP);
    assert!(matches!(first(&h), Some(MPick::Ent(_, Entity::Edge(_)))), "{:?}", first(&h));
    assert!(reading(&mut h).get(forge_agent::measure::Quantity::Length).is_some());
    set(&mut h, Filter::Points);
    click(&mut h, TOP);
    assert!(matches!(first(&h), Some(MPick::Ent(_, Entity::Vertex(_)))), "{:?}", first(&h));
    // a click on an edge with faces first: the face of it turned to the eye
    set(&mut h, Filter::Faces);
    let (_, under) = hover(&mut h, [0.0, -30.0, 12.0]);
    assert!(matches!(under, Some((_, Entity::Edge(_)))), "{under:?}");
    // marked before it's clicked, as what it'll pick
    let m = h.gui.shell.inspect.measure.as_ref().unwrap();
    assert!(matches!(m.hover, Some(MPick::Ent(_, Entity::Face(_)))), "{:?}", m.hover);
    click(&mut h, [0.0, -30.0, 12.0]);
    assert!(matches!(first(&h), Some(MPick::Ent(_, Entity::Face(_)))));
    set(&mut h, Filter::Bodies);
    click(&mut h, TOP);
    assert_eq!(first(&h), Some(MPick::Whole(0)));
    let r = reading(&mut h);
    assert!((r.get(forge_agent::measure::Quantity::SizeX).unwrap() - 80.0).abs() < 1e-6);
}

#[test]
fn accumulate_adds_up_what_it_is_given() {
    use forge_agent::measure::Quantity;
    let Some(mut h) = open("accum") else { return };
    measure(&mut h);
    click(&mut h, BOSS_TOP);
    let a = reading(&mut h).get(Quantity::Area).unwrap();
    assert!(h.click("measure:add:Area"));
    h.step();
    // a click on nothing starts again; what's added up stays
    let v = h.gui.shell.view_rect;
    quick_click(&mut h, v.left() + 10.0, v.bottom() - 10.0);
    assert!(picks(&h).is_empty());
    click(&mut h, BOSS_TOP);
    let _ = reading(&mut h);
    assert!(h.click("measure:add:Area"));
    h.step();
    h.step();
    let m = h.gui.shell.inspect.measure.as_ref().unwrap();
    let total: f64 = m.accum.iter().filter(|x| x.q == Quantity::Area).map(|x| x.x).sum();
    assert!((total - 2.0 * a).abs() < 1e-9);
    assert!(h.gui.shell.place("measure:accum:Areas").is_some());
    assert!(h.click("measure:clear"));
    h.step();
    assert!(h.gui.shell.inspect.measure.as_ref().unwrap().accum.is_empty());
}

#[test]
fn origin_items_from_the_browser_and_sketch_geometry_are_picked() {
    use forge_agent::measure::{Free, Quantity};
    use forge_render::shell::inspect::MPick;
    let Some(mut h) = open("origin") else { return };
    measure(&mut h);
    // the XY plane chosen in the browser, then the boss's top: 20 apart
    h.gui.shell.origin_selected = Some(forge_render::origin::Item::XyPlane);
    h.step();
    h.step();
    assert!(matches!(
        h.gui.shell.inspect.measure.as_ref().unwrap().picks.first(),
        Some(MPick::Free(Free::Plane(..), _))
    ));
    click(&mut h, BOSS_TOP);
    assert!((reading(&mut h).get(Quantity::Distance).unwrap() - 20.0).abs() < 1e-9);
    // in a sketch on XY, a line of it
    use forge_render::shell::sketching::{SketchFor, SketchMode};
    let Some(forge_lang::eval::Value::Plane(p)) = forge_lang::builtins::constant("xy") else {
        panic!("xy")
    };
    let mut m = SketchMode::new(SketchFor::New, "xy".into(), p.frame, vec!["line -60 -50 -20 -50".into()], None);
    m.units = "mm";
    h.gui.shell.sketch = Some(m);
    h.gui.shell.tab = 1;
    if let Some(me) = &mut h.gui.shell.inspect.measure {
        me.restart();
    }
    for _ in 0..10 {
        h.step();
    }
    let (cam, size) = h.gui.shell.camera.unwrap();
    let on = cam.pixel_of([-40.0, -50.0, 0.0], size.0, size.1).unwrap();
    let r = h.gui.shell.view_rect;
    h.click_at(r.min.x + on.0, r.min.y + on.1);
    for _ in 0..4 {
        h.step();
    }
    let me = h.gui.shell.inspect.measure.as_ref().unwrap();
    assert!(matches!(me.picks.first(), Some(MPick::Free(Free::Segment(..), _))), "{:?}", me.picks);
    assert!((reading(&mut h).get(Quantity::Length).unwrap() - 40.0).abs() < 1e-9);
}

#[test]
fn measuring_goes_on_across_the_tabs() {
    let Some(mut h) = open("tabs") else { return };
    measure(&mut h);
    let a = click(&mut h, TOP);
    let _ = reading(&mut h);
    for tab in [0, 3, 2] {
        h.gui.shell.tab = tab;
        for _ in 0..3 {
            h.step();
        }
        assert_eq!(picks(&h), vec![a], "tab {tab}");
        assert!(h.gui.shell.inspect.measure.as_ref().unwrap().reading.is_some());
    }
    h.gui.shell.tab = 0;
    h.step();
    let b = click(&mut h, BOSS_TOP);
    assert_eq!(picks(&h), vec![a, b]);
}

/// Pixels changed between two frames, off the part and off the panels: a face's tint stays on
/// the face.
fn changed_off_the_part(h: &Headless, before: &forge_render::gui::Pixels, after: &forge_render::gui::Pixels) -> usize {
    let (cam, size) = h.gui.shell.camera.unwrap();
    let r = h.gui.shell.view_rect;
    let mut lo = egui::pos2(f32::MAX, f32::MAX);
    let mut hi = egui::pos2(f32::MIN, f32::MIN);
    for x in [-40.0, 40.0] {
        for y in [-30.0, 30.0] {
            for z in [0.0, 20.0] {
                let p = cam.pixel_of([x, y, z], size.0, size.1).unwrap();
                let p = egui::pos2(r.min.x + p.0, r.min.y + p.1);
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
    }
    let part = egui::Rect::from_min_max(lo, hi).expand(12.0);
    // the panel, as wide as its rows make it
    let panel = h
        .gui
        .shell
        .places
        .iter()
        .filter(|p| p.0 == "measure:panel")
        .fold(egui::Rect::NOTHING, |a, p| a.union(p.1))
        // its shadow and frame
        .expand(48.0);
    // the labels in the view, and their leaders' reach
    let labels: Vec<egui::Rect> = h
        .gui
        .shell
        .places
        .iter()
        .filter(|p| p.0.starts_with("measure:mark:"))
        .map(|p| p.1.expand(4.0))
        .collect();
    let mut n = 0;
    for y in (r.min.y as usize + 80)..(r.max.y as usize - 4) {
        for x in (r.min.x as usize + 4)..(r.max.x as usize - 180) {
            let at = egui::pos2(x as f32, y as f32);
            let label = labels.iter().any(|l| l.contains(at));
            if !part.contains(at) && !panel.contains(at) && !label && before.at(x, y) != after.at(x, y) {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn a_picked_face_is_tinted_on_itself_and_nowhere_else() {
    tinted_on_itself(TOP, "tint");
}

#[test]
fn a_picked_cylinder_is_drawn_on_itself_and_nowhere_else() {
    tinted_on_itself([0.0, -12.0, 15.0], "tint-cyl");
}

fn tinted_on_itself(world: [f32; 3], tag: &str) {
    let Some(mut h) = open(tag) else { return };
    measure(&mut h);
    let (at, _) = hover(&mut h, world);
    h.move_to(5.0, 5.0);
    for _ in 0..10 {
        h.step();
    }
    let before = h.gui_pixels();
    h.move_to(at.0, at.1);
    click(&mut h, world);
    let _ = reading(&mut h);
    for _ in 0..10 {
        h.step();
    }
    let hovered = h.gui_pixels();
    let n = changed_off_the_part(&h, &before, &hovered);
    assert!(n < 50, "{n} pixels off the part changed with the face picked and hovered");
    h.move_to(5.0, 5.0);
    for _ in 0..10 {
        h.step();
    }
    let picked = h.gui_pixels();
    let n = changed_off_the_part(&h, &before, &picked);
    assert!(n < 50, "{n} pixels off the part changed with the face picked");
}

/// Picks set straight on the panel (free geometry), and the reading they give.
fn read_free(h: &mut Headless, picks: Vec<forge_agent::measure::Free>) -> forge_agent::measure::Reading {
    use forge_render::shell::inspect::MPick;
    let m = h.gui.shell.inspect.measure.as_mut().unwrap();
    m.restart();
    m.picks = picks.into_iter().map(|f| MPick::Free(f, "free".into())).collect();
    reading(h)
}

fn marks(h: &Headless) -> Vec<String> {
    let mut v: Vec<String> = h
        .gui
        .shell
        .places
        .iter()
        .filter_map(|p| p.0.strip_prefix("measure:mark:").map(str::to_string))
        .collect();
    v.sort();
    v
}

#[test]
fn the_deltas_follow_the_distance_chosen_and_the_view_draws_that_one() {
    use forge_agent::measure::{Free, Quantity};
    use forge_math::{Point3, Vec3};
    use forge_render::shell::inspect::{Follow, panel_values};
    let Some(mut h) = open("follow") else { return };
    measure(&mut h);
    let r = read_free(
        &mut h,
        vec![Free::Circle(Point3::new(0.0, 0.0, 30.0), Vec3::Z, 3.0), Free::Point(Point3::new(10.0, 0.0, 34.0))],
    );
    assert!(r.get(Quantity::CentreDistance).is_some() && r.get(Quantity::MaxDistance).is_some());
    assert!(marks(&h).contains(&"Minimum distance".to_string()), "{:?}", marks(&h));
    assert!(!marks(&h).contains(&"Center to center".to_string()));
    assert!(h.click("measure:follow:Center"));
    h.step();
    h.step();
    let m = h.gui.shell.inspect.measure.as_ref().unwrap();
    assert_eq!(m.follow, Follow::Centre);
    let ms = marks(&h);
    assert!(ms.contains(&"Center to center".to_string()) && !ms.contains(&"Minimum distance".to_string()), "{ms:?}");
    // centre (0, 0, 30) to (10, 0, 34)
    let v = panel_values(&r, Follow::Centre, None, false);
    let get = |q| v.iter().find(|x| x.0 == q).unwrap().1;
    close(get(Quantity::Dx), 10.0);
    close(get(Quantity::Dz), 4.0);
    // the farthest point of the rim, (-3, 0, 30)
    let v = panel_values(&r, Follow::Max, None, false);
    close(v.iter().find(|x| x.0 == Quantity::Dx).unwrap().1, 13.0);
    // and with the deltas hidden, none listed
    assert!(panel_values(&r, Follow::Min, None, true).iter().all(|x| !matches!(x.0, Quantity::Dx | Quantity::Dy | Quantity::Dz)));
    // a value clicked: drawn forward, and a distance clicked is the one followed
    assert!(h.click("measure:label:Maximum distance"));
    h.step();
    let m = h.gui.shell.inspect.measure.as_ref().unwrap();
    assert_eq!((m.focus, m.follow), (Some(Quantity::MaxDistance), Follow::Max));
}

#[test]
fn advanced_settings_give_a_second_unit_and_a_coordinate_system() {
    use forge_agent::measure::{Free, Quantity};
    use forge_math::Point3;
    use forge_render::shell::inspect::{Follow, panel_values, shown_dual};
    let Some(mut h) = open("advanced") else { return };
    measure(&mut h);
    assert!(h.gui.shell.place("measure:dual").is_none());
    // the panel's settled
    for _ in 0..5 {
        h.step();
    }
    assert!(h.click("measure:advanced"));
    h.step();
    h.step();
    assert!(h.gui.shell.inspect.measure.as_ref().unwrap().advanced);
    for p in ["measure:dual", "measure:ucs", "measure:delta"] {
        assert!(h.gui.shell.place(p).is_some(), "no {p}");
    }
    assert_eq!(shown_dual(Quantity::Length, 25.4, ("mm", "deg"), 3, Some("in")), "25.400 mm (1.000 in)");
    assert_eq!(shown_dual(Quantity::Angle, 90.0, ("mm", "deg"), 1, Some("in")), "90.0°");
    // a point in a sketch's frame: sk2 sits on the base's top, 12 up
    let r = read_free(&mut h, vec![Free::Point(Point3::new(3.0, 4.0, 17.0))]);
    let fg = h.gui.shell.fg.as_ref().unwrap();
    let frame = fg.feats.iter().find(|f| f.id == "sk2").and_then(|f| f.frame).expect("sk2's frame");
    let v = panel_values(&r, Follow::Min, Some(&frame), false);
    let local = frame.point_to_local(Point3::new(3.0, 4.0, 17.0));
    close(v.iter().find(|x| x.0 == Quantity::Z).unwrap().1, local.z);
    close(local.z.abs(), 5.0);
    // chosen in the panel, the frames listed are the document's
    h.gui.shell.inspect.measure.as_mut().unwrap().ucs = Some("sk2".into());
    h.step();
    assert!(h.gui.shell.place("measure:ucs").is_some());
}

#[test]
fn accumulate_adds_takes_away_and_deletes_a_value_or_a_group() {
    use forge_agent::measure::Quantity;
    let Some(mut h) = open("accum2") else { return };
    measure(&mut h);
    click(&mut h, BOSS_TOP);
    let a = reading(&mut h).get(Quantity::Area).unwrap();
    let p = reading(&mut h).get(Quantity::Perimeter).unwrap();
    assert!(h.click("measure:add:Area"));
    assert!(h.click("measure:add:Area"));
    assert!(h.click("measure:sub:Area"));
    assert!(h.click("measure:add:Perimeter"));
    h.step();
    let total = |h: &Headless, q: Quantity| -> f64 {
        h.gui.shell.inspect.measure.as_ref().unwrap().accum.iter().filter(|x| x.q == q).map(|x| x.x).sum()
    };
    assert!((total(&h, Quantity::Area) - a).abs() < 1e-9);
    assert!((total(&h, Quantity::Perimeter) - p).abs() < 1e-9);
    // each value is listed with what it was of; one deleted, then the lengths' group
    assert!(h.gui.shell.place("measure:accum:entry:3").is_some());
    assert!(h.click("measure:accum:entry:0"));
    h.step();
    assert!((total(&h, Quantity::Area) - 0.0).abs() < 1e-9);
    assert!(h.click("measure:accum:drop:Lengths"));
    h.step();
    let m = h.gui.shell.inspect.measure.as_ref().unwrap();
    assert_eq!(m.accum.len(), 2);
    assert!(m.accum.iter().all(|x| x.q == Quantity::Area && !x.what.is_empty()));
}

#[test]
fn shift_on_a_third_point_reads_the_angle_at_the_second() {
    use forge_agent::measure::{Free, Quantity};
    use forge_math::Point3;
    use forge_render::shell::inspect::MPick;
    let Some(mut h) = open("three") else { return };
    measure(&mut h);
    let p = |x: f64, y: f64| MPick::Free(Free::Point(Point3::new(x, y, 0.0)), "point".into());
    let m = h.gui.shell.inspect.measure.as_mut().unwrap();
    m.take(p(10.0, 0.0));
    m.take(p(0.0, 0.0));
    m.take_with(p(0.0, 10.0), true);
    assert_eq!(m.picks.len(), 3);
    close(reading(&mut h).get(Quantity::Angle).unwrap(), 90.0);
    assert!(h.gui.shell.place("measure:pick3").is_some());
    // without Shift a third starts again
    let m = h.gui.shell.inspect.measure.as_mut().unwrap();
    m.take(p(5.0, 5.0));
    assert_eq!(m.picks.len(), 1);
}

#[test]
fn the_labels_in_the_view_keep_clear_of_each_other() {
    let Some(mut h) = open("labels") else { return };
    measure(&mut h);
    click(&mut h, [0.0, -12.0, 15.0]);
    let _ = reading(&mut h);
    h.step();
    let ls: Vec<egui::Rect> = h
        .gui
        .shell
        .places
        .iter()
        .filter(|p| p.0.starts_with("measure:mark:"))
        .map(|p| p.1)
        .collect();
    assert!(ls.len() >= 2, "{ls:?}");
    for (i, a) in ls.iter().enumerate() {
        for b in &ls[i + 1..] {
            assert!(!a.intersects(*b), "{a:?} on {b:?}");
        }
    }
}

#[test]
fn measure_from_a_dialog_field_puts_the_value_in_it() {
    use forge_agent::measure::Free;
    use forge_lang::values::Dim;
    use forge_math::Point3;
    let Some(mut h) = open("into") else { return };
    // the Extrude dialog on extrude1: its distance
    h.gui.fg_edit_feature("extrude1");
    for _ in 0..4 {
        h.step();
    }
    let tool = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().expect("the extrude's dialog");
    let name = tool.params.iter().find(|p| p.dim == Dim::Length).expect("a length").name;
    h.gui.shell.measure_into_param(name);
    h.step();
    h.step();
    assert!(h.gui.shell.inspect.measure.as_ref().is_some_and(|m| m.into.is_some()));
    let _ = read_free(&mut h, vec![Free::Point(Point3::new(0.0, 0.0, 0.0)), Free::Point(Point3::new(0.0, 0.0, 17.5))]);
    for _ in 0..3 {
        h.step();
    }
    // only lengths go in a length
    assert!(h.gui.shell.place("measure:use:Angle").is_none());
    assert!(h.click("measure:use:Minimum distance"));
    h.step();
    assert!(h.gui.shell.inspect.measure.is_none(), "Measure is done once a value is used");
    let tool = h.gui.shell.fg.as_ref().unwrap().tool.as_ref().unwrap();
    assert_eq!(tool.params.iter().find(|p| p.name == name).unwrap().text, "17.5");
}

#[test]
fn the_two_selections_are_marked_each_in_its_own_colour() {
    let Some(mut h) = open("sets") else { return };
    measure(&mut h);
    let (a, _) = hover(&mut h, [30.0, 22.0, 12.0]);
    click(&mut h, TOP);
    let (b, _) = hover(&mut h, BOSS_TOP);
    click(&mut h, BOSS_TOP);
    let _ = reading(&mut h);
    h.move_to(5.0, 5.0);
    for _ in 0..10 {
        h.step();
    }
    let px = h.gui_pixels();
    let [r1, g1, b1] = px.at(a.0 as usize, a.1 as usize).map(i32::from);
    let [r2, g2, b2] = px.at(b.0 as usize, b.1 as usize).map(i32::from);
    // the first orange, the second green
    assert!(r1 > g1 && r1 > b1, "first {r1} {g1} {b1}");
    assert!(g2 > r2 && g2 > b2, "second {r2} {g2} {b2}");
}

#[test]
fn the_values_column_is_dragged_wider_and_narrower() {
    let Some(mut h) = open("column") else { return };
    measure(&mut h);
    click(&mut h, BOSS_TOP);
    let _ = reading(&mut h);
    for _ in 0..4 {
        h.step();
    }
    let edge = h.gui.shell.place("measure:column").expect("the column's edge");
    let before = h.gui.shell.place("measure:value:Area").unwrap();
    let (x, y) = (edge.center().x, edge.center().y);
    h.move_to(x, y);
    h.step();
    h.button(true);
    h.step();
    for k in 1..=8 {
        h.move_to(x + 5.0 * k as f32, y);
        h.step();
    }
    h.button(false);
    for _ in 0..3 {
        h.step();
    }
    let m = h.gui.shell.inspect.measure.as_ref().unwrap();
    assert!((m.label_w - (112.0 + 40.0)).abs() < 2.0, "{}", m.label_w);
    let after = h.gui.shell.place("measure:value:Area").unwrap();
    assert!(after.left() > before.left() + 30.0, "{before:?} {after:?}");
}

#[test]
fn the_panel_stands_still_from_its_first_frame() {
    let Some(mut h) = open("still") else { return };
    h.gui.shell.out.push(Action::Inspect(InspectCmd::Measure));
    let names = ["measure:filter:Any", "measure:pick2", "measure:advanced"];
    let mut first = None;
    for _ in 0..8 {
        h.step();
        let now: Vec<Option<egui::Rect>> = names.iter().map(|n| h.gui.shell.place(n)).collect();
        if now.iter().all(Option::is_some) {
            // once its buttons are named, they're where they stay
            let was = first.get_or_insert(now.clone());
            assert_eq!(*was, now);
        }
    }
    assert!(first.is_some());
}

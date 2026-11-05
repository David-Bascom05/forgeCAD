//! Dragging in an assembly, Inventor's: a press and drag on a component moves it within what its
//! joints leave free, live and transient; Keep Position writes it (one undo step), Return to
//! Design (or Esc) goes back; a fastened or grounded part stays and the status says what holds
//! it; a click still selects and a press on nothing still draws a window. On the 18U rack: a door
//! swings with the pointer and stops at 105°, its latch going with it, each move timed.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::asmdrag::{Plan, plan};
use forge_render::shell::{Action, FgCmd};
use std::path::{Path, PathBuf};

fn rack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/rack18u/asm/rack.fg")
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-drag-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg.as_ref().is_some_and(|s| s.done == s.seq)
        && g.shell.fg.as_ref().is_some_and(|p| p.asm.is_some())
}

fn open(file: &Path, size: (f32, f32)) -> Option<Headless> {
    let mut h = match Headless::new(size, 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    settle(&mut h, if file.ends_with("rack.fg") { 600.0 } else { 60.0 });
    Some(h)
}

fn settle(h: &mut Headless, secs: f64) {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < secs {
        if h.until(ready) {
            break;
        }
    }
    let issues: Vec<String> = h
        .gui
        .shell
        .fg
        .as_ref()
        .map(|p| {
            p.issues
                .iter()
                .map(|i| format!("{}: {}", i.kind, i.text))
                .collect()
        })
        .unwrap_or_default();
    assert!(ready(&h.gui), "never ready: {} {issues:?}", h.gui.shell.status);
    for _ in 0..3 {
        h.step();
    }
}

fn asm(h: &Headless) -> &forge_render::shell::fgasm::AsmPanel {
    h.gui.shell.fg.as_ref().unwrap().asm.as_ref().unwrap()
}

fn src(h: &Headless) -> String {
    h.gui.fg.as_ref().unwrap().src.clone()
}

/// An occurrence's placement.
fn placement(h: &Headless, path: &str) -> usize {
    let (occ, _) = h.gui.fg_asm.as_ref().unwrap();
    occ.iter()
        .find(|o| o.path == path)
        .and_then(|o| o.placement)
        .unwrap_or_else(|| panic!("no {path}"))
}

type Rows = [[f32; 4]; 3];

fn rows(h: &Headless, k: usize) -> Rows {
    h.gui.view.as_ref().unwrap().scene.rows_of(k)
}

fn apply(r: &Rows, p: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|i| r[i][0] * p[0] + r[i][1] * p[1] + r[i][2] * p[2] + r[i][3])
}

/// A world point's place in the window.
fn window_px(h: &Headless, p: [f32; 3]) -> Option<(f32, f32)> {
    let v = h.gui.view.as_ref().unwrap();
    let (x, y) = v.nav.camera.pixel_of(p, v.nav.size.0, v.nav.size.1)?;
    let o = h.gui.shell.view_rect.min;
    Some((o.x + x, o.y + y))
}

/// What placement the pointer is over here, once the pick has said.
fn under(h: &mut Headless, at: (f32, f32)) -> Option<u32> {
    h.move_to(at.0, at.1);
    for _ in 0..6 {
        h.step();
    }
    let v = h.gui.view.as_ref().unwrap();
    v.nav
        .marks()
        .into_iter()
        .find(|m| !m.selected)
        .map(|m| m.placement)
}

/// A point on placement `k` the pointer lands on: its box's middle, or near it.
fn spot_on(h: &mut Headless, k: usize) -> (f32, f32) {
    let (m, r) = h.gui.fg_asm.as_ref().unwrap().1[k];
    let b = h.gui.fg_asm_boxes[m];
    for f in [
        [0.5, 0.5, 0.5],
        [0.3, 0.5, 0.5],
        [0.5, 0.3, 0.5],
        [0.5, 0.5, 0.3],
        [0.7, 0.5, 0.5],
        [0.5, 0.7, 0.5],
        [0.5, 0.5, 0.7],
        [0.3, 0.3, 0.3],
        [0.7, 0.7, 0.7],
        [0.2, 0.8, 0.5],
        [0.8, 0.2, 0.5],
    ] {
        let p = [0, 1, 2].map(|i| b.0[i] + (b.1[i] - b.0[i]) * f[i]);
        let Some(px) = window_px(h, apply(&r, p)) else {
            continue;
        };
        if under(h, px) == Some(k as u32) {
            return px;
        }
    }
    panic!("no spot on placement {k} under the pointer");
}

/// Frames till what was last sent is built.
fn built(h: &mut Headless) {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 120.0 && !h.gui.fg.as_ref().is_some_and(|s| s.done == s.seq) {
        h.step();
    }
    h.step();
}

fn param(h: &Headless, key: &str) -> Option<f64> {
    asm(h)
        .posing
        .params
        .iter()
        .find(|p| p.0.to_string() == key)
        .map(|p| p.1.value)
}

/// The press held and the pointer taken to `to` in `n` moves; each move's time from the move to
/// its pose drawn.
fn drag_to(h: &mut Headless, from: (f32, f32), to: (f32, f32), n: usize, times: &mut Vec<f64>) {
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let at = (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
        let t0 = std::time::Instant::now();
        h.move_to(at.0, at.1);
        h.step();
        built(h);
        times.push(t0.elapsed().as_secs_f64() * 1e3);
    }
}

fn press(h: &mut Headless, at: (f32, f32)) {
    h.move_to(at.0, at.1);
    for _ in 0..4 {
        h.step();
    }
    h.button(true);
    h.step();
    h.step();
}

fn release(h: &mut Headless) {
    h.button(false);
    h.step();
    h.step();
}

fn shot(h: &Headless, name: &str) {
    let dir = std::env::var("DRAG_SHOTS").map_or_else(|_| scratch("shots"), PathBuf::from);
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join(name);
    std::fs::write(&p, h.ppm()).unwrap();
    eprintln!("shot: {}", p.display());
}

fn summary(v: &[f64]) -> String {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    if s.is_empty() {
        return "none".into();
    }
    let at = |q: f64| s[((s.len() - 1) as f64 * q).round() as usize];
    format!(
        "{} moves: median {:.1} ms, p90 {:.1}, worst {:.1}",
        s.len(),
        at(0.5),
        at(0.9),
        at(1.0)
    )
}

/// One placement's origin in another's frame, from their rows.
fn in_frame(of: &Rows, p: &Rows) -> [f32; 3] {
    let d = [0, 1, 2].map(|i| p[i][3] - of[i][3]);
    [0, 1, 2].map(|c| (0..3).map(|i| of[i][c] * d[i]).sum())
}

fn near(a: &Rows, b: &Rows) -> bool {
    (0..3).all(|i| (0..4).all(|k| (a[i][k] - b[i][k]).abs() < 1e-3))
}

#[test]
fn a_rack_door_swings_with_the_pointer_stops_at_its_limit_and_keeps_or_returns() {
    let Some(mut h) = open(&rack(), (1400.0, 900.0)) else {
        return;
    };
    let file = src(&h);
    // no relationship glyphs till called for
    assert!(asm(&h).glyph_list().is_empty(), "glyphs on open");
    let door = placement(&h, "f_door");
    let hdl = placement(&h, "f_hdl");
    let (door0, hdl0) = (rows(&h, door), rows(&h, hdl));
    let rel0 = in_frame(&door0, &hdl0);
    // what holds what: the door on its hinge's turn, the latch handle on its own, the frame fixed
    let p = h.gui.shell.fg.as_ref().unwrap();
    let a = asm(&h);
    let Plan::Joint(j) = plan(a, &p.feats, &p.params, "f_door") else {
        panic!("the door: {:?}", plan(a, &p.feats, &p.params, "f_door"))
    };
    assert_eq!(j.joint, "f_door_f");
    assert_eq!(j.moving(false).len(), 1, "{:?}", j.dofs);
    for locked in ["top_pan", "f_brk_b", "base_pan"] {
        let pl = plan(a, &p.feats, &p.params, locked);
        assert!(matches!(pl, Plan::Locked(_)), "{locked}: {pl:?}");
        eprintln!("{locked}: {pl:?}");
    }
    let Plan::Joint(lj) = plan(a, &p.feats, &p.params, "f_hdl") else {
        panic!("the latch handle")
    };
    assert_eq!(lj.joint, "f_latch_f");

    // press on the door, drag to where 60° would put the pressed point
    let at = spot_on(&mut h, door);
    press(&mut h, at);
    assert!(asm(&h).posing.grab.is_some(), "the press didn't take the door");
    let grabbed = asm(&h).posing.grab.as_ref().unwrap().at;
    let turn = j.dofs.iter().position(|d| d.word == "turn").unwrap();
    let target = |h: &Headless, deg: f64| {
        let mut v: Vec<f64> = j.dofs.iter().map(|d| d.value).collect();
        v[turn] = deg.to_radians();
        let q = j.world_motion(&v).point_to_world(grabbed);
        window_px(h, [q.x as f32, q.y as f32, q.z as f32]).unwrap()
    };
    let (to60, to150) = (target(&h, 60.0), target(&h, 150.0));
    let mut times = vec![];
    drag_to(&mut h, at, to60, 12, &mut times);
    let v = param(&h, "f_door_f.turn").expect("no turn set");
    assert!((v - 60.0).abs() < 4.0, "turn {v}°, wanted about 60°");
    assert!(
        h.gui.shell.status.starts_with("Dragging: f_door_f.turn = "),
        "{}",
        h.gui.shell.status
    );
    assert!(
        h.gui.shell.window_box.is_none(),
        "a window drawn while dragging a part"
    );
    let (door1, hdl1) = (rows(&h, door), rows(&h, hdl));
    assert!(!near(&door1, &door0), "the door didn't move");
    let rel1 = in_frame(&door1, &hdl1);
    for i in 0..3 {
        assert!(
            (rel1[i] - rel0[i]).abs() < 0.05,
            "the latch stayed behind: {rel0:?} {rel1:?}"
        );
    }
    shot(&h, "rack-door-60.ppm");
    // on past the limit: held at 105°
    drag_to(&mut h, to60, to150, 10, &mut times);
    let v = param(&h, "f_door_f.turn").unwrap();
    assert!((v - 105.0).abs() < 0.01, "turn {v}°: not held at its 105° limit");
    assert!(h.gui.shell.status.contains("limit"), "{}", h.gui.shell.status);
    shot(&h, "rack-door-105.ppm");
    let fg = h.gui.fg.as_ref().unwrap();
    eprintln!(
        "rack door drag, move to pose drawn: {}; last drive round {:.1} ms, solve {:.2} ms",
        summary(&times),
        fg.round_ms,
        fg.build_ms
    );
    // let go: the pose built whole, the file as it was
    release(&mut h);
    built(&mut h);
    assert_eq!(src(&h), file, "a drag wrote the file");
    assert!(
        h.gui.shell.status.contains("Keep Position"),
        "{}",
        h.gui.shell.status
    );
    let solved = asm(&h)
        .joints
        .iter()
        .find(|j| j.id == "f_door_f")
        .unwrap()
        .values
        .clone();
    assert!(
        solved
            .iter()
            .any(|v| v.0 == "turn" && (v.1.to_degrees() - 105.0).abs() < 0.01),
        "{solved:?}"
    );
    assert!(!near(&rows(&h, door), &door0));

    // Keep Position: one SetParam, one undo step
    h.gui.shell.out.push(Action::Fg(FgCmd::KeepPosition));
    h.step();
    built(&mut h);
    let kept = src(&h);
    assert!(
        kept.contains("param f_door_f.turn = 105deg"),
        "{}",
        h.gui.shell.status
    );
    assert!(asm(&h).posing.is_empty());
    h.gui.shell.out.push(Action::Undo);
    h.step();
    built(&mut h);
    assert_eq!(src(&h), file, "undo didn't take the kept pose back");
    assert!(near(&rows(&h, door), &door0), "the door didn't come back on undo");

    // a drag, then Esc: the design's pose back
    let at = spot_on(&mut h, door);
    press(&mut h, at);
    let mut more = vec![];
    let to40 = target(&h, 40.0);
    drag_to(&mut h, at, to40, 6, &mut more);
    release(&mut h);
    assert!(
        param(&h, "f_door_f.turn").is_some_and(|v| v > 5.0),
        "{:?}",
        asm(&h).posing.params
    );
    h.key(egui::Key::Escape);
    h.step();
    assert_eq!(h.gui.shell.status, "Returned to the design");
    built(&mut h);
    assert!(asm(&h).posing.is_empty(), "Esc left the pose");
    assert!(near(&rows(&h, door), &door0), "Esc didn't bring the door back");
    assert_eq!(src(&h), file);

    // a part fastened to the frame: no motion, and why
    let pan = placement(&h, "top_pan");
    let pan0 = rows(&h, pan);
    let at = spot_on(&mut h, pan);
    press(&mut h, at);
    let mut none = vec![];
    drag_to(&mut h, at, (at.0 + 80.0, at.1 + 40.0), 4, &mut none);
    assert!(
        h.gui.shell.status.starts_with("Locked: "),
        "{}",
        h.gui.shell.status
    );
    eprintln!("top_pan dragged: {}", h.gui.shell.status);
    assert_eq!(rows(&h, pan), pan0);
    assert!(asm(&h).posing.is_empty());
    release(&mut h);

    // a click without a drag still selects
    let at = spot_on(&mut h, door);
    h.click_at(at.0, at.1);
    h.step();
    h.step();
    assert_eq!(asm(&h).selected.as_deref(), Some("f_door"));
    assert!(asm(&h).posing.is_empty());
    // the chosen door's relationships' glyphs, and only theirs
    let a = asm(&h);
    let shown: Vec<&str> = a.glyph_list().iter().map(|g| a.links[g.0].id.as_str()).collect();
    for want in ["f_door_f", "f_latch_f"] {
        assert!(shown.contains(&want), "{shown:?}");
    }
    for g in a.glyph_list() {
        let l = &a.links[g.0];
        assert!(
            l.a.0 == "f_door" || l.b.0 == "f_door",
            "{} is not the door's",
            l.id
        );
    }
    eprintln!("door chosen: glyphs of {shown:?}");
    shot(&h, "rack-door-chosen-glyphs.ppm");

    // a press on nothing still draws a window
    let o = h.gui.shell.view_rect.min;
    let empty = (o.x + 30.0, o.y + 120.0);
    assert_eq!(under(&mut h, empty), None, "not empty there");
    press(&mut h, empty);
    h.move_to(empty.0 + 120.0, empty.1 + 90.0);
    h.step();
    assert!(h.gui.shell.window_box.is_some(), "no window from empty space");
    release(&mut h);
    // nothing chosen: no glyphs; Show All: every one
    h.click_at(empty.0, empty.1);
    h.step();
    h.step();
    assert_eq!(asm(&h).selected, None);
    assert!(asm(&h).glyph_list().is_empty(), "glyphs with nothing chosen");
    h.gui.shell.out.push(Action::Fg(FgCmd::Relationships));
    h.step();
    assert!(
        asm(&h).glyph_list().len() > 100,
        "Show All showed {}",
        asm(&h).glyph_list().len()
    );
}

fn link(len: f64) -> String {
    format!(
        "shared ln = {len}
shared w = 8
shared t = 4
sketch plan(on: xy) {{
  outline = rect(corner: (-w / 2, -w / 2), size: (ln + w, w))
  p0 = pt(0, 0)
  p1 = pt(ln, 0)
}}
bar = extrude(plan, t)
a = anchor(plane(at: plan.p0, normal: plan.plane.normal, x: plan.outline[0].dir))
b = anchor(plane(at: plan.p1, normal: plan.plane.normal, x: plan.outline[0].dir))
"
    )
}

const MECH: &str = "assembly

base = part(\"base.fg\", ground: true)
arm = part(\"arm.fg\", at: pose(at: (100, 0, 0)))
param pivot.turn = 20deg
pivot = revolute(arm.a, base.b, turn: turn, limit: (0deg, 90deg))
tip = part(\"arm.fg\", at: pose(at: (140, 0, 0)))
hold = fastened(tip.a, arm.b)
peg = part(\"arm.fg\", at: pose(at: (0, -40, 0)))
param rail.along = 5
rail = slider(peg.a, base.a, along: along, limit: (0, 30))
loose = part(\"arm.fg\", at: pose(at: (0, 80, 0)))
";

fn mech(name: &str) -> PathBuf {
    let d = scratch(name);
    std::fs::write(d.join("base.fg"), link(100.0)).unwrap();
    std::fs::write(d.join("arm.fg"), link(40.0)).unwrap();
    let f = d.join("top.fg");
    std::fs::write(&f, MECH).unwrap();
    f
}

#[test]
fn a_pivot_a_slider_and_a_free_part_drag_as_their_joints_allow() {
    let Some(mut h) = open(&mech("mech"), (1300.0, 900.0)) else {
        return;
    };
    let p = h.gui.shell.fg.as_ref().unwrap();
    let a = asm(&h);
    let plans: Vec<(&str, Plan)> = ["arm", "tip", "peg", "loose", "base"]
        .into_iter()
        .map(|o| (o, plan(a, &p.feats, &p.params, o)))
        .collect();
    for (o, pl) in &plans {
        let ok = match *o {
            "arm" | "tip" => matches!(pl, Plan::Joint(j) if j.joint == "pivot"),
            "peg" => matches!(pl, Plan::Joint(j) if j.joint == "rail"),
            "loose" => matches!(pl, Plan::Free { .. }),
            _ => matches!(pl, Plan::Locked(w) if w.contains("grounded")),
        };
        assert!(ok, "{o}: {pl:?}");
    }
    let file = src(&h);

    // the tip (fastened to the arm) turns the arm about its pivot
    let tip = placement(&h, "tip");
    let Plan::Joint(j) = plans[1].1.clone() else {
        unreachable!()
    };
    let at = spot_on(&mut h, tip);
    press(&mut h, at);
    let grabbed = asm(&h).posing.grab.as_ref().unwrap().at;
    let q = j.world_motion(&[60f64.to_radians()]).point_to_world(grabbed);
    let to = window_px(&h, [q.x as f32, q.y as f32, q.z as f32]).unwrap();
    let mut times = vec![];
    drag_to(&mut h, at, to, 8, &mut times);
    let v = param(&h, "pivot.turn").unwrap();
    assert!((v - 60.0).abs() < 3.0, "pivot {v}°");
    release(&mut h);
    built(&mut h);

    // the peg slides on its rail, stopping at 30
    let peg = placement(&h, "peg");
    let Plan::Joint(j) = plans[2].1.clone() else {
        unreachable!()
    };
    let at = spot_on(&mut h, peg);
    press(&mut h, at);
    let grabbed = asm(&h).posing.grab.as_ref().unwrap().at;
    let px = |h: &Headless, v: f64| {
        let q = j.world_motion(&[v]).point_to_world(grabbed);
        window_px(h, [q.x as f32, q.y as f32, q.z as f32]).unwrap()
    };
    let (to15, to80) = (px(&h, 15.0), px(&h, 80.0));
    drag_to(&mut h, at, to15, 6, &mut times);
    let v = param(&h, "rail.along").unwrap();
    assert!((v - 15.0).abs() < 1.0, "rail {v}");
    drag_to(&mut h, to15, to80, 6, &mut times);
    assert_eq!(param(&h, "rail.along"), Some(30.0));
    release(&mut h);
    built(&mut h);

    // the loose part moves freely; Keep Position writes it all in one undo step
    let loose = placement(&h, "loose");
    let l0 = rows(&h, loose);
    let at = spot_on(&mut h, loose);
    press(&mut h, at);
    drag_to(&mut h, at, (at.0 + 60.0, at.1 + 30.0), 4, &mut times);
    assert!(!near(&rows(&h, loose), &l0), "the loose part didn't follow");
    shot(&h, "mech-loose-drag.ppm");
    release(&mut h);
    built(&mut h);
    assert!(
        !near(&rows(&h, loose), &l0),
        "the loose part went back when let go"
    );
    let posing = &asm(&h).posing;
    assert!(posing.poses.iter().any(|p| p.0 == "loose"), "{posing:?}");
    assert_eq!(src(&h), file);
    eprintln!("mech drags, move to pose drawn: {}", summary(&times));
    let said = h.gui.keep_position();
    assert!(said.as_ref().is_ok_and(|s| s.starts_with("Kept: ")), "{said:?}");
    built(&mut h);
    let kept = src(&h);
    assert!(kept.contains("param pivot.turn = "), "{kept}");
    assert!(kept.contains("param rail.along = 30"), "{kept}");
    assert!(
        !kept.contains("loose = part(\"arm.fg\", at: pose(at: (0, 80, 0)))"),
        "{kept}"
    );
    h.gui.shell.out.push(Action::Undo);
    h.step();
    built(&mut h);
    assert_eq!(src(&h), file, "not one undo step");
}

#[test]
fn the_agents_drive_is_transient_held_at_its_limits_and_kept_or_returned() {
    let Some(mut h) = open(&mech("agent"), (1200.0, 800.0)) else {
        return;
    };
    let file = src(&h);
    let turn = |h: &Headless| {
        asm(h).joints.iter().find(|j| j.id == "pivot").unwrap().values[0]
            .1
            .to_degrees()
    };
    assert_eq!(h.gui.drive_joint("pivot", 45.0), Ok("pivot.turn = 45°".into()));
    built(&mut h);
    assert!((turn(&h) - 45.0).abs() < 1e-6, "{}", turn(&h));
    assert_eq!(src(&h), file);
    assert_eq!(
        h.gui.drive_joint("rail.along", 99.0),
        Ok("rail.along = 30 mm (held at its limit)".into())
    );
    assert!(h.gui.drive_joint("hold", 1.0).is_err());
    assert!(h.gui.drive_joint("pivot.along", 1.0).is_err());
    assert_eq!(h.gui.return_to_design(), Ok("Returned to the design".into()));
    built(&mut h);
    assert!((turn(&h) - 20.0).abs() < 1e-6, "{}", turn(&h));
    assert_eq!(src(&h), file);
    h.gui.drive_joint("pivot.turn", 70.0).unwrap();
    built(&mut h);
    h.gui.keep_position().unwrap();
    built(&mut h);
    assert!(src(&h).contains("param pivot.turn = 70deg"), "{}", src(&h));
}

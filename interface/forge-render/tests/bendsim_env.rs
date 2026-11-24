//! The Bend Simulation environment end to end, headless on the view's own renderer: a channel
//! bent on a real press (Trumpf TruBend 5130, from the library) with the maker's own tools,
//! scrubbed through its stages, the press, tools and part drawn in 3D; and a deliberate
//! collision (a narrow U whose second flange closes on Bystronic's straight punch) drawn with
//! the failing tool and its witness in red.
//!
//! `BENDSIM_SHOTS=DIR` writes the frames (PPM) for research/bendsim/env/.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::bendsim::{Choice, Framing, Pane};
use forge_render::shell::env::EnvId;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-bendsim-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

const CHANNEL: &str = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\nshared h = 30\n\
sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (100, 200))\n}\n\
base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
right = flange(base.edge(plan.outline[1]), height: h)\n\
left = flange(base.edge(plan.outline[3]), height: h)\n";

const NARROW_U: &str = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\nshared h = 70\n\
sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (40, 300))\n}\n\
base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
right = flange(base.edge(plan.outline[1]), height: h)\n\
left = flange(base.edge(plan.outline[3]), height: h)\n";

fn headless() -> Option<Headless> {
    match Headless::new((1400.0, 860.0), 1.0) {
        Ok(mut h) => {
            h.step();
            Some(h)
        }
        Err(e) => {
            eprintln!("skipped: {e}");
            None
        }
    }
}

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

fn open(h: &mut Headless, file: &str) {
    h.gui.open(file);
    assert!(h.until(settled), "{file} never built: {}", h.gui.shell.status);
    for _ in 0..4 {
        h.step();
    }
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

/// The view framed as the ribbon's View tools frame it.
fn frame(h: &mut Headless, how: Framing) {
    let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
    b.frame = Some(how);
    b.t = (b.t + 1e-6).min(1.0);
}

fn shot(h: &mut Headless, name: &str) {
    for _ in 0..6 {
        h.step();
    }
    let Ok(dir) = std::env::var("BENDSIM_SHOTS") else {
        return;
    };
    std::fs::write(format!("{dir}/{name}.ppm"), h.ppm()).unwrap();
}

/// Into the environment on `press` (and only `punches`, if named).
fn enter(h: &mut Headless, src: &str, name: &str, press: &str, punches: Option<&[&str]>) {
    let d = scratch(name);
    let f = d.join(format!("{name}.fg"));
    std::fs::write(&f, src).unwrap();
    open(h, f.to_str().unwrap());
    click(h, "tab:Environments");
    click(h, "tool:Bend Simulation");
    assert_eq!(h.gui.shell.env, Some(EnvId::BendSim), "{}", h.gui.shell.status);
    let b = h.gui.shell.sheet.bend_sim.as_mut().expect("simulated");
    let c = Choice {
        press: Some(press.into()),
        punches: punches.map(|p| p.iter().map(|s| s.to_string()).collect()),
        ..Default::default()
    };
    let t = b.tooling.clone();
    b.redo(c, &t);
    h.step();
    h.step();
}

#[test]
fn a_channel_on_a_trubend_in_3d() {
    let Some(mut h) = headless() else { return };
    enter(&mut h, CHANNEL, "channel", "trumpf-trubend-5130", None);
    {
        let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
        let sim = b.sim.as_ref().expect("a plan");
        assert!(sim.plan.feasible, "{:?}", sim.plan.blocking);
        assert_eq!(sim.tooling.press.id, "trumpf-trubend-5130");
    }
    // the view holds the press: the design's part put aside
    assert!(h.gui.bend.showing());
    assert!(h.gui.shell.place("env:bendsim:timeline").is_some());
    assert!(
        h.gui.shell.place("env:bendsim:properties").is_some(),
        "the bend's sheet docked"
    );
    let n = h.gui.view.as_ref().unwrap().scene.placements.len();
    assert!(n > 6, "press, tools and part: {n} bodies");
    shot(&mut h, "00-trubend-default-view");
    // the machine and its tools: the library as cards
    h.gui.shell.sheet.bend_sim.as_mut().unwrap().pane = Pane::Setup;
    shot(&mut h, "07-machine-and-tools");
    h.gui.shell.sheet.bend_sim.as_mut().unwrap().pane = Pane::Program;
    // the bend being made is marked on the part
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.step = 0;
        b.t = 0.6;
    }
    for _ in 0..3 {
        h.step();
    }
    let v = h.gui.view.as_ref().unwrap();
    assert!(
        v.scene
            .placements
            .iter()
            .any(|p| p.tint[0] > 0.75 && (0.4..0.7).contains(&p.tint[1]) && p.tint[2] < 0.2),
        "the bend zone in amber"
    );
    assert!(h.click("tool:Press Overview"), "the ribbon's overview");
    shot(&mut h, "00-trubend-whole-press");
    frame(&mut h, Framing::Work);
    for (step, t, name) in [
        (0, 0.3, "01-trubend-approach"),
        (0, 0.6, "02-trubend-bend-1"),
        (1, 0.08, "03-trubend-handling"),
        (1, 0.075, "03b-trubend-handling-turn"),
        (1, 0.7, "04-trubend-bend-2"),
    ] {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.step = step;
        b.t = t;
        h.step();
        // as "Part and Tools" frames it at that instant
        frame(&mut h, Framing::Work);
        shot(&mut h, name);
    }
    // the program written as the part's bending document: the editor's layout and steps
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.write_doc().expect("written");
        assert!(b.doc.is_some());
        b.step = 1;
        b.t = 0.6;
    }
    shot(&mut h, "06-program-editor");
    // its stations on the bed, dragged to place
    h.gui.shell.sheet.bend_sim.as_mut().unwrap().pane = Pane::Setup;
    shot(&mut h, "08-stations-on-the-bed");
    assert!(h.gui.shell.place("env:bendsim:browser").is_some());
    // scrub to the end: the part folded, the design back on Finish
    click(&mut h, "tool:Finish Bend Simulation");
    h.step();
    assert!(!h.gui.bend.showing());
}

#[test]
fn a_flange_closing_on_a_straight_punch_shows_in_red() {
    let Some(mut h) = headless() else { return };
    enter(
        &mut h,
        NARROW_U,
        "narrow",
        "bystronic-xpert-pro-100-3100",
        Some(&["bystronic/P6_RF-A_H175_R1.0"]),
    );
    let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
    assert!(b.as_given, "no feasible order: the part's own shown");
    let sim = b.sim.as_ref().expect("simulated");
    assert!(!sim.plan.feasible);
    // the step that fails, at its bend: the punch and the witness drawn red
    let k = sim
        .plan
        .steps
        .iter()
        .position(|s| !s.pass())
        .expect("a failing step");
    let phi = sim.plan.steps[k].failures()[0].phi.unwrap();
    let phimax = sim.plan.steps[k].angle + sim.plan.steps[k].overbend;
    b.step = k;
    b.t = 0.4 + 0.35 * (phi / phimax).clamp(0.0, 1.0);
    frame(&mut h, Framing::Work);
    shot(&mut h, "05-bystronic-collision");
    let v = h.gui.view.as_ref().unwrap();
    assert!(
        v.scene
            .placements
            .iter()
            .any(|p| p.tint[0] > 0.3 && p.tint[0] > 4.0 * p.tint[1] && p.tint[0] > 4.0 * p.tint[2]),
        "a body in red"
    );
}

/// At 1024 x 700 (docs/ux/STANDARD.md §10): the browser, the view and the bend's sheet all in,
/// nothing cut off.
#[test]
fn a_small_window_holds_it_all() {
    let Ok(mut h) = Headless::new((1024.0, 700.0), 1.0) else {
        return;
    };
    h.step();
    enter(&mut h, CHANNEL, "small", "trumpf-trubend-5130", None);
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.step = 0;
        b.t = 0.6;
    }
    frame(&mut h, Framing::Work);
    shot(&mut h, "09-small-window-1024x700");
    let view = h.gui.shell.place("env:bendsim:view").expect("the view");
    let dock = h.gui.shell.place("env:bendsim:properties").expect("the sheet");
    assert!(
        view.right() <= dock.left() + 1.0,
        "the view stops at the docked sheet"
    );
    assert!(view.width() > 300.0, "the view keeps room: {view:?}");
}

/// The view cube in the simulation (David): its LEFT and RIGHT along the bend line, so LEFT is
/// the view down the bend; and see-through: the first flange, between the eye and the second
/// bend from the backgauge's side, drawn as a ghost; solid again with see-through off.
#[test]
fn the_cube_looks_down_the_bend_and_what_hides_it_is_seen_through() {
    let Some(mut h) = headless() else { return };
    enter(&mut h, CHANNEL, "cube", "trumpf-trubend-5130", None);
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.step = 1;
        b.t = 0.7;
    }
    frame(&mut h, Framing::Work);
    for _ in 0..4 {
        h.step();
    }
    // the cube's LEFT, as a click on it goes: down the bend line (along the bed)
    let v = h.gui.view.as_mut().unwrap();
    assert_eq!(v.nav.orient, forge_render::gui::gui_bendsim::bend_orient());
    let left = v.nav.orient.world([-1, 0, 0]);
    assert_eq!((left[0], left[2]), (0, 0), "LEFT is along the bed: {left:?}");
    let cam = forge_render::nav::region_view_as(v.nav.camera, left, &v.nav.orient);
    v.nav.jump_to(cam);
    let f = cam.axes().0;
    assert!(f[1].abs() > 0.99, "looking along the bend line: {f:?}");
    shot(&mut h, "10-down-the-bend-line");
    // from the operator's side and above, the part's first flange and web stand in front of
    // the second bend: seen through
    let v = h.gui.view.as_mut().unwrap();
    let mut c = v.nav.camera;
    c.yaw = 150f32.to_radians();
    c.pitch = 30f32.to_radians();
    v.nav.jump_to(c);
    shot(&mut h, "11-seen-through");
    let n = h.gui.bend.ghosts.len();
    assert!(n > 0, "a flange between the eye and the bend is seen through");
    h.gui.shell.sheet.bend_sim.as_mut().unwrap().see_through = false;
    for _ in 0..3 {
        h.step();
    }
    assert!(h.gui.bend.ghosts.is_empty(), "off: everything solid");
}

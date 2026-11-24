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
    // see-through settled: FADE_AFTER frames, then FADE_FRAMES
    for _ in 0..16 {
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
    // the view is under 560 px here: the compact bar, one row (nothing drawn over the step's
    // words); a view 560 to 900 px wide puts the settings on a row of their own
    let bar = h.gui.shell.place("env:bendsim:timeline").expect("the bar");
    let w = h.gui.shell.place("env:bendsim:view").expect("the view").width();
    assert_eq!(
        forge_render::shell::bendsim::Bar::for_width(w),
        forge_render::shell::bendsim::Bar::Compact
    );
    assert!(bar.height() < 80.0, "{bar:?}");
    assert_eq!(
        forge_render::shell::bendsim::Bar::for_width(700.0),
        forge_render::shell::bendsim::Bar::Narrow
    );
    assert_eq!(
        forge_render::shell::bendsim::Bar::for_width(1100.0),
        forge_render::shell::bendsim::Bar::Wide
    );
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
    {
        let v = h.gui.view.as_mut().unwrap();
        let mut c = v.nav.camera;
        c.yaw = 180f32.to_radians();
        c.pitch = 70f32.to_radians();
        v.nav.jump_to(c);
    }
    // the cube from above, the operator's side below: TOP reads upright
    shot(&mut h, "12-cube-from-above");
    // from the operator's side and above, the part's first flange (already bent) stands in
    // front of the second bend: seen through (the walls either side of the bend being made never)
    let v = h.gui.view.as_mut().unwrap();
    let mut c = v.nav.camera;
    c.yaw = 240f32.to_radians();
    c.pitch = 25f32.to_radians();
    v.nav.jump_to(c);
    // its fade run through (FADE_AFTER frames, then FADE_FRAMES)
    for _ in 0..16 {
        h.step();
    }
    shot(&mut h, "11-seen-through");
    let n = h.gui.bend.ghosts.len();
    assert!(n > 0, "a flange between the eye and the bend is seen through");
    h.gui.shell.sheet.bend_sim.as_mut().unwrap().see_through = false;
    // it fades back over FADE_FRAMES frames, not at once (no flicker)
    h.step();
    assert!(!h.gui.bend.ghosts.is_empty(), "fading back, not snapped");
    for _ in 0..forge_render::gui::gui_bendsim::FADE_FRAMES + 2 {
        h.step();
    }
    assert!(h.gui.bend.ghosts.is_empty(), "off: everything solid");
}

/// Follow bend through the operator's handling (David, via the coordinator): the part pulled
/// out toward the operator and turned stays in view, the eye widening for it, and the next bend
/// is framed on its station.
#[test]
fn following_keeps_the_handled_part_in_view() {
    let Some(mut h) = headless() else { return };
    enter(&mut h, CHANNEL, "follow", "trumpf-trubend-5130", None);
    h.gui.shell.sheet.bend_sim.as_mut().unwrap().follow = true;
    let mut seen = Vec::new();
    for t in [0.0, 0.02, 0.05, 0.08, 0.11, 0.15, 0.5] {
        {
            let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
            b.step = 1;
            b.t = t;
        }
        for _ in 0..3 {
            h.step();
        }
        // the part's middle, as the view's camera projects it
        let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
        let sim = b.sim.as_ref().unwrap();
        let mut sc = forge_brake::scene::Scene::new(&sim.model, &sim.tooling, &sim.plan);
        let bodies = sc.bodies(1, t);
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for x in bodies.iter().filter(|x| x.look.is_sheet()) {
            let (a, c) = x.mesh.bounds().unwrap();
            for k in 0..3 {
                lo[k] = lo[k].min(a[k]);
                hi[k] = hi[k].max(c[k]);
            }
        }
        let (cam, size) = h.gui.shell.camera.expect("the view's camera");
        let m = cam.view(size.0, size.1).matrix;
        let mut inside = true;
        for corner in 0..8 {
            let p = [0, 1, 2].map(|k| if corner >> k & 1 == 0 { lo[k] } else { hi[k] });
            let c = |r: usize| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r];
            let w = c(3);
            inside &= w > 0.0 && (c(0) / w).abs() <= 1.0 && (c(1) / w).abs() <= 1.0;
        }
        seen.push((t, inside));
        if t == 0.05 {
            shot(&mut h, "13-following-the-handling");
        }
    }
    assert!(
        seen.iter().all(|s| s.1),
        "the whole part in view at every instant: {seen:?}"
    );
}

/// No flicker (David): the eye swept round the second bend through the views where the first
/// flange crosses in front of it, a frame a step: what is seen through in the x-ray window never
/// jumps; it fades in and out a step a frame.
#[test]
fn see_through_fades_and_never_flickers() {
    let Some(mut h) = headless() else { return };
    enter(&mut h, CHANNEL, "flicker", "trumpf-trubend-5130", None);
    let opacity = {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.see_through = true;
        b.step = 1;
        b.t = 0.7;
        b.opacity
    };
    frame(&mut h, Framing::Work);
    for _ in 0..4 {
        h.step();
    }
    let step = (1.0 - opacity) / forge_render::gui::gui_bendsim::FADE_FRAMES as f32 + 0.01;
    // as it stands before the sweep (what already fades counts from there)
    let names = h.gui.bend.drawn_names();
    let mut was: std::collections::HashMap<String, f32> = h
        .gui
        .bend
        .ghosts
        .iter()
        .map(|g| (names[g.placement as usize].clone(), g.rgba[3] as f32 / 255.0))
        .collect();
    let mut worst = 0f32;
    let mut ghosted = 0usize;
    for i in 0..=80 {
        {
            let v = h.gui.view.as_mut().unwrap();
            let mut c = v.nav.camera;
            c.yaw = (200.0 + i as f32).to_radians();
            c.pitch = 25f32.to_radians();
            v.nav.jump_to(c);
        }
        h.step();
        // nothing cut (David), and the bend being made and the walls either side of it never
        // seen through
        assert!(h.gui.bend.whole(), "a body cut at {i}");
        let bending = h.gui.bend.bending_names();
        assert!(!bending.is_empty());
        for n in h.gui.bend.ghosted_names() {
            assert!(
                !bending.contains(&n),
                "{n} is the bend being made, seen through at {i}"
            );
        }
        // by the body's name
        let names = h.gui.bend.drawn_names();
        let now: std::collections::HashMap<String, f32> = h
            .gui
            .bend
            .ghosts
            .iter()
            .map(|g| (names[g.placement as usize].clone(), g.rgba[3] as f32 / 255.0))
            .collect();
        // a body seen through from one frame to the next fades a step at most; one that starts
        // to be seen through comes in near solid (it fades in)
        for (k, b) in &now {
            let a = was.get(k).copied().unwrap_or(1.0);
            worst = worst.max(a - b);
            if was.contains_key(k) {
                worst = worst.max(b - a);
            }
        }
        ghosted = ghosted.max(now.len());
        was = now;
    }
    assert!(
        ghosted > 0,
        "something stood in front of the bend at some instant"
    );
    assert!(
        worst <= step,
        "a see-through jump of {worst} in one frame (a step is {step})"
    );
}

/// At 1000 x 700 the view between the browser and the bend's sheet is under 400 px: the play
/// bar goes compact (icons, "1/2 · stage", the view's settings in a View menu), one row, nothing
/// over anything (UX-LEAD, STANDARD.md §5).
#[test]
fn a_narrow_view_gets_a_compact_bar() {
    let Ok(mut h) = Headless::new((1000.0, 700.0), 1.0) else {
        return;
    };
    h.step();
    enter(&mut h, CHANNEL, "narrow1000", "trumpf-trubend-5130", None);
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.step = 0;
        b.t = 0.6;
    }
    frame(&mut h, Framing::Work);
    shot(&mut h, "14-compact-bar-1000x700");
    let view = h.gui.shell.place("env:bendsim:view").expect("the view");
    assert!(view.width() < 560.0, "{view:?}");
    assert_eq!(
        forge_render::shell::bendsim::Bar::for_width(view.width()),
        forge_render::shell::bendsim::Bar::Compact
    );
    let bar = h.gui.shell.place("env:bendsim:timeline").expect("the bar");
    assert!(bar.height() < 80.0, "one row and the track: {bar:?}");
    // the view's chips stop short of the cube's home button (the rest stay in Settings)
    let cube = h.gui.shell.cube_place.expect("the cube");
    let home = view.left() + cube.at.0 / cube.scale;
    for name in ["axes", "grid", "planes", "anchors"] {
        if let Some(r) = h.gui.shell.place(&format!("view:{name}")) {
            assert!(
                r.right() < home,
                "{name} chip {r:?} reaches the home button at {home}"
            );
        }
    }
    assert!(
        h.gui.shell.place("view:axes").is_some(),
        "the first chips still shown"
    );
}

/// Bend 2 as a recording sees it (Follow bend on, coordinator's frame strip): at every instant of
/// its forming the part is a real share of the picture, not a sliver under the ram.
#[test]
fn bend_two_followed_shows_the_part() {
    let Some(mut h) = headless() else { return };
    enter(&mut h, CHANNEL, "strip", "trumpf-trubend-5130", None);
    h.gui.shell.sheet.bend_sim.as_mut().unwrap().follow = true;
    let mut least = f32::MAX;
    for (i, t) in [0.2, 0.4, 0.5, 0.6, 0.7, 0.8].into_iter().enumerate() {
        {
            let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
            b.step = 1;
            b.t = t;
        }
        for _ in 0..16 {
            h.step();
        }
        shot(&mut h, &format!("15-bend-2-strip-{i}"));
        assert!(h.gui.bend.whole(), "a body cut at t = {t}");
        let bending = h.gui.bend.bending_names();
        for n in h.gui.bend.ghosted_names() {
            assert!(
                !bending.contains(&n),
                "{n}, the bend being made, seen through at t = {t}"
            );
        }
        let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
        let sim = b.sim.as_ref().unwrap();
        let mut sc = forge_brake::scene::Scene::new(&sim.model, &sim.tooling, &sim.plan);
        let (cam, size) = h.gui.shell.camera.expect("the view's camera");
        let m = cam.view(size.0, size.1).matrix;
        let (mut ylo, mut yhi) = (f32::MAX, f32::MIN);
        for x in sc.bodies(1, t).iter().filter(|x| x.look.is_sheet()) {
            for p in &x.mesh.pos {
                let c = |r: usize| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r];
                let y = c(1) / c(3);
                ylo = ylo.min(y);
                yhi = yhi.max(y);
            }
        }
        // the part's height on the screen, as a share of the view's (NDC spans 2)
        least = least.min((yhi.min(1.0) - ylo.max(-1.0)) / 2.0);
    }
    assert!(
        least > 0.2,
        "the part only {least:.2} of the view's height at some instant"
    );
}

/// Any part in the way (David): from the Press Overview the press's frame stands between the
/// eye and the bend; it is seen through whole, nothing cut, the bend being made solid.
#[test]
fn a_press_part_in_the_way_is_seen_through_whole() {
    let Some(mut h) = headless() else { return };
    enter(&mut h, CHANNEL, "xray", "trumpf-trubend-5130", None);
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.step = 1;
        b.t = 0.6;
    }
    frame(&mut h, Framing::Overview);
    for _ in 0..20 {
        h.step();
    }
    let ghosted = h.gui.bend.ghosted_names();
    let press: Vec<&String> = ghosted
        .iter()
        .filter(|n| !n.starts_with("part:") && !n.contains("punch:") && !n.contains("die:"))
        .collect();
    assert!(!press.is_empty(), "a press part seen through: {ghosted:?}");
    assert!(h.gui.bend.whole(), "nothing cut");
    let bending = h.gui.bend.bending_names();
    assert!(ghosted.iter().all(|n| !bending.contains(n)), "{ghosted:?}");
    // the die can't hide the bend line from above it (UX-LEAD): it stays solid
    assert!(ghosted.iter().all(|n| !n.contains("die:")), "{ghosted:?}");
}

/// A seen-through body is drawn, and seen (David: "it's still invisible"): where it stands,
/// the view with it seen through differs clearly both from the view with it gone (opacity 0)
/// and from the view with it solid (see-through off).
#[test]
fn a_ghost_is_seen_between_gone_and_solid() {
    let Some(mut h) = headless() else { return };
    enter(&mut h, CHANNEL, "ghost", "trumpf-trubend-5130", None);
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.step = 1;
        b.t = 0.7;
    }
    frame(&mut h, Framing::Work);
    for _ in 0..4 {
        h.step();
    }
    let v = h.gui.view.as_mut().unwrap();
    let mut c = v.nav.camera;
    c.yaw = 240f32.to_radians();
    c.pitch = 25f32.to_radians();
    v.nav.jump_to(c);
    let opacity = h.gui.shell.sheet.bend_sim.as_ref().unwrap().opacity;
    let at = |h: &mut Headless, o: f32, see: bool| {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.opacity = o;
        b.see_through = see;
        for _ in 0..30 {
            h.step();
        }
        assert_eq!(h.gui.bend.ghosts.is_empty(), !see, "seen through: {see}");
        h.ppm()
    };
    let gone = at(&mut h, 0.0, true);
    let ghost = at(&mut h, opacity, true);
    // where the seen-through bodies stand on the screen: their own triangles, projected (not
    // what differs, which takes in the shadow a solid body casts)
    let (cam, size) = h.gui.shell.camera.expect("the view's camera");
    let r = h.gui.shell.place("env:bendsim:view").expect("the view");
    let m = cam.view(size.0, size.1).matrix;
    let (w, ht) = (1400usize, 860usize);
    let mut cover = vec![false; w * ht];
    for x in h.gui.bend.ghosted() {
        let to = |p: [f32; 3]| {
            let c = |k: usize| m[k] * p[0] + m[4 + k] * p[1] + m[8 + k] * p[2] + m[12 + k];
            (
                r.left() + (c(0) / c(3) + 1.0) * 0.5 * r.width(),
                r.top() + (1.0 - c(1) / c(3)) * 0.5 * r.height(),
            )
        };
        for t in &x.mesh.tri {
            let [a, b, c] = t.map(|i| to(x.mesh.pos[i as usize]));
            let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
            if area.abs() < 1e-3 {
                continue;
            }
            let (x0, x1) = (
                a.0.min(b.0).min(c.0).max(r.left()),
                a.0.max(b.0).max(c.0).min(r.right()),
            );
            let (y0, y1) = (
                a.1.min(b.1).min(c.1).max(r.top()),
                a.1.max(b.1).max(c.1).min(r.bottom()),
            );
            for py in y0 as usize..(y1.ceil() as usize).min(ht) {
                for px in x0 as usize..(x1.ceil() as usize).min(w) {
                    let q = (px as f32 + 0.5, py as f32 + 0.5);
                    let e = |u: (f32, f32), v: (f32, f32)| {
                        ((v.0 - u.0) * (q.1 - u.1) - (v.1 - u.1) * (q.0 - u.0)) * area.signum()
                    };
                    if e(a, b) >= 0.0 && e(b, c) >= 0.0 && e(c, a) >= 0.0 {
                        cover[py * w + px] = true;
                    }
                }
            }
        }
    }
    let solid = at(&mut h, opacity, false);
    let px = |p: &[u8], i: usize| [p[i * 3], p[i * 3 + 1], p[i * 3 + 2]];
    let far = |a: [u8; 3], b: [u8; 3], by: u8| a.iter().zip(b).any(|(x, y)| x.abs_diff(y) > by);
    // past the ppm's header, the same in all three
    let head = gone.len() - w * ht * 3;
    let (gone, ghost, solid) = (&gone[head..], &ghost[head..], &solid[head..]);
    // the body: where it stands and, solid, it is seen (not where it would match what's behind)
    let body: Vec<usize> = (0..w * ht)
        .filter(|&i| cover[i] && far(px(gone, i), px(solid, i), 24))
        .collect();
    assert!(body.len() > w * ht / 100, "the body covers {} px", body.len());
    let seen = body
        .iter()
        .filter(|&&i| far(px(ghost, i), px(gone, i), 12) && far(px(ghost, i), px(solid, i), 12))
        .count();
    assert!(
        seen * 4 >= body.len() * 3,
        "the ghost seen over {seen} of the body's {} px",
        body.len()
    );
}

/// A seen-through body is drawn at its opacity (the coordinator, after RENDER-STUDIO found the
/// punch's drawn not at all): the same view at 0.13 and at 0.8 differs wherever a ghost is.
#[test]
fn a_ghost_is_drawn_at_its_opacity() {
    let Some(mut h) = headless() else { return };
    enter(&mut h, CHANNEL, "ghost", "trumpf-trubend-5130", None);
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.step = 1;
        b.t = 0.7;
    }
    frame(&mut h, Framing::Work);
    for _ in 0..4 {
        h.step();
    }
    let v = h.gui.view.as_mut().unwrap();
    let mut c = v.nav.camera;
    c.yaw = 240f32.to_radians();
    c.pitch = 25f32.to_radians();
    v.nav.jump_to(c);
    let at = |h: &mut Headless, o: f32| {
        h.gui.shell.sheet.bend_sim.as_mut().unwrap().opacity = o;
        for _ in 0..20 {
            h.step();
        }
        assert!(!h.gui.bend.ghosts.is_empty(), "something seen through at {o}");
        h.ppm()
    };
    let lo = at(&mut h, 0.13);
    let hi = at(&mut h, 0.8);
    assert_eq!(lo.len(), hi.len());
    let differ = lo
        .chunks(3)
        .zip(hi.chunks(3))
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 8))
        .count();
    // a hundredth of the view: the ghosts drawn only where something opaque stood in front of
    // them (the depth compared the wrong way round) differed in some 600 pixels, drawn they
    // differ in some 26 000
    let floor = 1400 * 860 / 100;
    assert!(
        differ > floor,
        "the ghost drawn at its opacity: {differ} pixels differ, under {floor}"
    );
}

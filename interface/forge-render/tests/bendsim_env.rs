//! The Bend Simulation environment end to end, headless on the view's own renderer: a channel
//! bent on a real press (Trumpf TruBend 5130, from the library) with the maker's own tools,
//! scrubbed through its stages, the press, tools and part drawn in 3D; and a deliberate
//! collision (a narrow U whose second flange closes on Bystronic's straight punch) drawn with
//! the failing tool and its witness in red.
//!
//! `BENDSIM_SHOTS=DIR` writes the frames (PPM) for research/bendsim/env/.

use forge_math::ipow::Powi as _;
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

/// Follow bend as an operator watches (David, via the coordinator): through a whole program the
/// eye keeps its orientation, its distance grows once at most, and its speed never changes at
/// once (eased from rest to rest: no cut, no hunting); the part's small motions (the slide to the stop, the lift,
/// the ram's stroke, a flange's swing, a turn over) leave it still; and at the tools the part is
/// always wholly in view (the handling may carry it past the frame's edge, the eye still).
#[test]
fn following_keeps_the_handled_part_in_view() {
    for (w, ht) in [(1400.0, 860.0), (1000.0, 700.0)] {
        follow_through_the_program(w, ht);
    }
}

fn follow_through_the_program(w: f32, ht: f32) {
    let Ok(mut h) = Headless::new((w, ht), 1.0) else {
        return;
    };
    h.step();
    let part = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sheetmetal/load_tray.fg"
    );
    open(&mut h, part);
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    let tl = h.gui.shell.env_timeline().expect("the timeline");
    let total = tl.duration(&h.gui.shell);
    tl.seek(&mut h.gui.shell, 0.0);
    for _ in 0..20 {
        h.step();
    }
    let start = h.gui.view.as_ref().unwrap().nav.camera;
    let (mut was, mut grew, mut jolt, mut inside, n) = (start, 0, 0f32, 0, 240);
    let mut step_was = [0f32; 3];
    let mut tools = 0;
    for i in 0..n {
        let tl = h.gui.shell.env_timeline().unwrap();
        tl.seek(&mut h.gui.shell, total * i as f64 / (n - 1) as f64);
        h.step();
        let cam = h.gui.view.as_ref().unwrap().nav.camera;
        assert!(
            (cam.yaw - start.yaw).abs() < 1e-6 && (cam.pitch - start.pitch).abs() < 1e-6,
            "{w}x{ht} frame {i}: the eye turned"
        );
        assert!(cam.dist >= was.dist - 1e-3, "{w}x{ht} frame {i}: the eye came in");
        if cam.dist > was.dist + 1e-3 && was.dist <= start.dist + 1e-3 {
            grew += 1;
        }
        // a move's speed changes a little from frame to frame (eased); a cut changes it all at once
        let step = [0, 1, 2].map(|k| cam.target[k] - was.target[k]);
        let dv = (0..3)
            .map(|k| (step[k] - step_was[k]).ipow(2))
            .sum::<f32>()
            .sqrt();
        jolt = jolt.max(dv);
        step_was = step;
        was = cam;
        // the part wholly in view
        let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
        let sim = b.sim.as_ref().unwrap();
        let mut sc = forge_brake::scene::Scene::new(&sim.model, &sim.tooling, &sim.plan);
        let (cam, size) = h.gui.shell.camera.expect("the view's camera");
        let m = cam.view(size.0, size.1).matrix;
        let all = sc
            .bodies(b.step, b.t)
            .iter()
            .filter(|x| x.look.is_sheet())
            .all(|x| {
                x.mesh.pos.iter().all(|p| {
                    let c = |r: usize| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r];
                    c(3) > 0.0 && (c(0) / c(3)).abs() <= 1.0 && (c(1) / c(3)).abs() <= 1.0
                })
            });
        if !all {
            eprintln!(
                "OUTDBG {w} frame {i} step {} t {:.3} stage {:?}",
                b.step,
                b.t,
                forge_brake::scene::stage_at(b.t).0
            );
        }
        // the operator's handling may carry the part past the frame's edge, the eye still
        if forge_brake::scene::stage_at(b.t).0 == forge_brake::scene::Stage::Handle {
            continue;
        }
        tools += 1;
        inside += all as usize;
    }
    assert!(grew <= 1, "{w}x{ht}: the eye drew back {grew} times");
    assert!(
        jolt <= 12.0,
        "{w}x{ht}: the eye's speed changed {jolt:.1} mm a frame at once"
    );
    assert!(
        inside == tools,
        "{w}x{ht}: the part wholly in view at the tools in {inside} of {tools} frames"
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
    // a step of a fade: an x-ray's from solid to the opacity set, a cut-away's from solid to
    // gone, each over FADE_FRAMES
    let frames = forge_render::gui::gui_bendsim::FADE_FRAMES as f32;
    let step = ((1.0 - opacity) / frames).max(1.0 / frames) + 0.01;
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

/// Any part in the way (David), only where it is in the way: the Press Overview turned all
/// round the press at a low eye, the bend's watched points seen from every side, nothing cut, the
/// bend never seen through; and from some side a press part (the frame, the ram, the backgauge)
/// stands in front of the bend and is seen through whole.
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
    for _ in 0..8 {
        h.step();
    }
    let mut press_ghosted = Vec::new();
    for k in 0..12 {
        {
            let v = h.gui.view.as_mut().unwrap();
            let mut c = v.nav.camera;
            c.yaw = (30.0 * k as f32).to_radians();
            c.pitch = 15f32.to_radians();
            v.nav.jump_to(c);
        }
        for _ in 0..16 {
            h.step();
        }
        let (seen, _) = see_metric(&h);
        assert!(
            seen >= 0.95,
            "from {} degrees: {:.1} % of the bend seen",
            30 * k,
            seen * 100.0
        );
        assert!(h.gui.bend.whole(), "nothing cut");
        let ghosted = h.gui.bend.ghosted_names();
        let bending = h.gui.bend.bending_names();
        assert!(ghosted.iter().all(|n| !bending.contains(n)), "{ghosted:?}");
        press_ghosted.extend(
            ghosted
                .into_iter()
                .filter(|n| !n.starts_with("part:") && !n.contains("punch:") && !n.contains("die:")),
        );
    }
    assert!(
        !press_ghosted.is_empty(),
        "a press part in the way, seen through, from some side"
    );
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
    assert!(body.len() > w * ht / 200, "the body covers {} px", body.len());
    // seen against what is behind it on nearly all of it; against the body solid on most (a
    // flange in shade is a mid tone, as the flat ghost is: its blue and its outline mark it there)
    let off_gone = body
        .iter()
        .filter(|&&i| far(px(ghost, i), px(gone, i), 12))
        .count();
    let off_solid = body
        .iter()
        .filter(|&&i| far(px(ghost, i), px(solid, i), 12))
        .count();
    assert!(
        off_gone * 20 >= body.len() * 17,
        "the ghost seen against what is behind it over {off_gone} of the body's {} px",
        body.len()
    );
    assert!(
        off_solid * 3 >= body.len() * 2,
        "the ghost told from the body solid over {off_solid} of its {} px",
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

/// Near (David; UX-LEAD's words): a clearance proven only within the simulation's resolution of
/// its limit is not a pass but a mild warning, "near", listed after the warnings and before the
/// passes, never hidden behind the step's "Passes" badge. The panel's side of it: a step's tool
/// clearance made a Near as forge-brake makes one (which of a part's pairs end near is the
/// kernel's, tested there; a finer search proves most of them clear), on a narrow channel whose
/// step also has a warning.
#[test]
fn a_near_clearance_is_listed_as_near() {
    let Some(mut h) = headless() else { return };
    let narrow = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\n\
sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (40, 200))\n}\n\
base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
right = flange(base.edge(plan.outline[1]), height: 60)\n\
left = flange(base.edge(plan.outline[3]), height: 60)\n";
    let d = scratch("near");
    let f = d.join("near.fg");
    std::fs::write(&f, narrow).unwrap();
    open(&mut h, f.to_str().unwrap());
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    // the standard tooling alone, on no press (as `query F bendsim` asks it by default)
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().expect("simulated");
        b.redo(Choice::default(), "std-eu");
    }
    h.step();
    h.step();
    let b = h.gui.shell.sheet.bend_sim.as_mut().expect("simulated");
    let plan = &mut b.sim.as_mut().expect("a program").plan;
    assert!(plan.feasible);
    let at = plan.steps.len() - 1;
    let s = &mut plan.steps[at];
    let k = s
        .checks
        .iter()
        .position(|c| c.rule == forge_brake::Rule::ToolCollision && c.pass)
        .expect("a passing tool clearance");
    let was = s.checks[k].clone();
    let mut c = forge_brake::Check::near(was.rule, was.limit - 0.004, was.limit, was.limit / 4.0);
    c.tool = was.tool.clone();
    c.phase = was.phase;
    s.checks[k] = c;
    assert!(s.pass(), "a near is a warning: the step still passes");
    let c = &s.checks[k];
    assert!(!c.pass && c.severity == forge_brake::Severity::Warning && c.near.is_some());
    b.step = at;
    b.all_checks = true;
    for _ in 0..4 {
        h.step();
    }
    shot(&mut h, "16-near-in-the-checks");
}

/// A clip for David (the coordinator): `BENDSIM_CLIP=DIR` writes 270 frames (9 s at 30 fps)
/// through the whole program of a narrow channel on the standard tooling (it has a near row),
/// seeked on the environment's timeline as a recording seeks it, and the Checks panel at the
/// near step. Off unless asked.
#[test]
#[ignore]
fn a_clip_for_david() {
    let Ok(dir) = std::env::var("BENDSIM_CLIP") else {
        return;
    };
    let Some(mut h) = headless() else { return };
    let narrow = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\n\
sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (40, 200))\n}\n\
base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
right = flange(base.edge(plan.outline[1]), height: 60)\n\
left = flange(base.edge(plan.outline[3]), height: 60)\n";
    let d = scratch("clip");
    let f = d.join("narrow-channel.fg");
    std::fs::write(&f, narrow).unwrap();
    open(&mut h, f.to_str().unwrap());
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    h.gui
        .shell
        .sheet
        .bend_sim
        .as_mut()
        .unwrap()
        .redo(Choice::default(), "std-eu");
    for _ in 0..4 {
        h.step();
    }
    let tl = h.gui.shell.env_timeline().expect("the timeline");
    let total = tl.duration(&h.gui.shell);
    assert!(total > 0.0);
    // settle on the first instant before the first frame
    tl.seek(&mut h.gui.shell, 0.0);
    for _ in 0..20 {
        h.step();
    }
    let n = 270;
    for i in 0..n {
        let tl = h.gui.shell.env_timeline().unwrap();
        tl.seek(&mut h.gui.shell, total * i as f64 / (n - 1) as f64);
        h.step();
        std::fs::write(format!("{dir}/frame-{i:04}.ppm"), h.ppm()).unwrap();
    }
    // the Checks panel at the near step, every check shown
    let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
    let at = b
        .sim
        .as_ref()
        .unwrap()
        .plan
        .steps
        .iter()
        .position(|s| s.checks.iter().any(|c| c.near.is_some()))
        .expect("a near");
    b.step = at;
    b.t = 0.6;
    b.all_checks = true;
    for _ in 0..20 {
        h.step();
    }
    std::fs::write(format!("{dir}/checks.ppm"), h.ppm()).unwrap();
    eprintln!("clip: {n} frames over {total} s of program");
}

/// The frames the coordinator saw the bed fill (the channel on the TruBend 5130, native frame
/// ~125 at 30 fps: t = 4.17 s, the handling before bend 2), as they stand now. Off unless asked.
#[test]
#[ignore]
fn the_handling_frames_on_the_trubend() {
    let Ok(dir) = std::env::var("BENDSIM_CLIP") else {
        return;
    };
    let Some(mut h) = headless() else { return };
    enter(&mut h, CHANNEL, "trubend-clip", "trumpf-trubend-5130", None);
    let tl = h.gui.shell.env_timeline().expect("the timeline");
    tl.seek(&mut h.gui.shell, 0.0);
    for _ in 0..20 {
        h.step();
    }
    for i in 0..=130u32 {
        let tl = h.gui.shell.env_timeline().unwrap();
        tl.seek(&mut h.gui.shell, i as f64 / 30.0);
        h.step();
        if i >= 120 {
            std::fs::write(format!("{dir}/trubend-{i:04}.ppm"), h.ppm()).unwrap();
        }
    }
}

/// A complex part's whole program for David (the coordinator): `BENDSIM_CLIP=DIR` and
/// `BENDSIM_CLIP_PART=FILE.fg` write 360 frames (12 s at 30 fps) through every bend, flips and
/// turns included, seeked on the timeline as a recording seeks it; the press the environment
/// picks if its program is feasible, else the standard tooling alone. Off unless asked.
#[test]
#[ignore]
fn a_complex_part_clip() {
    let (Ok(dir), Ok(part)) = (std::env::var("BENDSIM_CLIP"), std::env::var("BENDSIM_CLIP_PART")) else {
        return;
    };
    let Some(mut h) = headless() else { return };
    open(&mut h, &part);
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    let feasible = |h: &Headless| {
        h.gui
            .shell
            .sheet
            .bend_sim
            .as_ref()
            .and_then(|b| b.sim.as_ref().ok())
            .is_some_and(|s| s.plan.feasible)
    };
    if !feasible(&h) {
        h.gui
            .shell
            .sheet
            .bend_sim
            .as_mut()
            .unwrap()
            .redo(Choice::default(), "std-eu");
        for _ in 0..4 {
            h.step();
        }
    }
    assert!(feasible(&h), "{}", h.gui.shell.status);
    {
        let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
        eprintln!("clip on {} {:?}: {} bends", b.tooling, b.choice.press, b.steps());
    }
    // BENDSIM_PROFILE=1: the working view end on, down the bend line; BENDSIM_ORBIT=DEG: the
    // eye turned that much more round the subject each frame (see-through follows the live eye)
    let profile = std::env::var("BENDSIM_PROFILE").is_ok();
    let orbit: f32 = std::env::var("BENDSIM_ORBIT")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(0.0);
    {
        // framed afresh in it, as the View tools frame
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.profile = profile;
        b.frame = Some(Framing::Work);
    }
    let base = if profile {
        forge_render::gui::gui_bendsim::PROFILE_YAW
    } else {
        215.0
    };
    let tl = h.gui.shell.env_timeline().expect("the timeline");
    let total = tl.duration(&h.gui.shell);
    tl.seek(&mut h.gui.shell, 0.0);
    for _ in 0..20 {
        h.step();
    }
    let n = 360;
    for i in 0..n {
        let tl = h.gui.shell.env_timeline().unwrap();
        tl.seek(&mut h.gui.shell, total * i as f64 / (n - 1) as f64);
        h.step();
        if orbit != 0.0 {
            let v = h.gui.view.as_mut().unwrap();
            let mut c = v.nav.camera;
            c.yaw = (base + orbit * i as f32).to_radians();
            v.nav.jump_to(c);
            h.step();
        }
        std::fs::write(format!("{dir}/frame-{i:04}.ppm"), h.ppm()).unwrap();
    }
    eprintln!("clip: {n} frames over {total} s of program");
}

/// The see-through's measure (David's "an order of magnitude"), from what is drawn alone: of the
/// points watched (the bend zone's top along its length, the punch tip's contact, a witness), the
/// share no solid body hides from the eye; and the share of the view the ghosts cover (a coarse
/// raster of their triangles). Self-contained, so the same measure runs on any version.
fn see_metric(h: &Headless) -> (f32, f32) {
    use forge_brake::scene::{Body, Look};
    let g = &h.gui.bend;
    let drawn: &[Body] = g.drawn();
    let ghosted: Vec<String> = g.ghosted_names();
    let bending = g.bending_names();
    let (cam, size) = h.gui.shell.camera.expect("the view's camera");
    let eye = cam.eye();
    // the watched points
    let spread = |mut pts: Vec<[f32; 3]>, n: usize| -> Vec<[f32; 3]> {
        if pts.len() <= n {
            return pts;
        }
        pts.sort_by(|a, b| {
            a[1].total_cmp(&b[1])
                .then(a[2].total_cmp(&b[2]))
                .then(a[0].total_cmp(&b[0]))
        });
        (0..n).map(|k| pts[k * (pts.len() - 1) / (n - 1)]).collect()
    };
    let roi_body = |x: &Body| {
        x.look == Look::BendZone
            || x.look == Look::Witness
            || (x.name.starts_with("punch:") && x.name.ends_with("/tip"))
    };
    let mut aims = spread(forge_render::gui::gui_bendsim::zone_top(drawn), 48);
    let tips: Vec<&Body> = drawn
        .iter()
        .filter(|x| x.name.starts_with("punch:") && x.name.ends_with("/tip"))
        .collect();
    if let Some(low) = tips
        .iter()
        .flat_map(|x| x.mesh.pos.iter())
        .map(|p| p[2])
        .reduce(f32::min)
    {
        aims.extend(spread(
            tips.iter()
                .flat_map(|x| x.mesh.pos.iter().copied())
                .filter(|p| p[2] <= low + 1.0)
                .collect(),
            16,
        ));
    }
    aims.extend(spread(
        drawn
            .iter()
            .filter(|x| x.look == Look::Witness)
            .flat_map(|x| x.mesh.pos.iter().copied())
            .collect(),
        8,
    ));
    if aims.is_empty() {
        return (1.0, 0.0);
    }
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let hides = |x: &Body, p: [f32; 3]| {
        let d = sub(p, eye);
        let len = dot(d, d).sqrt();
        let tmax = 1.0 - 0.5 / len.max(1e-3);
        x.mesh.tri.iter().any(|t| {
            let [a, b, c] = t.map(|i| x.mesh.pos[i as usize]);
            let (e1, e2) = (sub(b, a), sub(c, a));
            let hh = cross(d, e2);
            let det = dot(e1, hh);
            if det.abs() < 1e-12 {
                return false;
            }
            let f = 1.0 / det;
            let s0 = sub(eye, a);
            let u = f * dot(s0, hh);
            let q = cross(s0, e1);
            let v = f * dot(d, q);
            let tt = f * dot(e2, q);
            (0.0..=1.0).contains(&u) && v >= 0.0 && u + v <= 1.0 && tt > 1e-6 && tt < tmax
        })
    };
    let solid: Vec<&Body> = drawn
        .iter()
        .filter(|x| {
            !ghosted.contains(&x.name) && !roi_body(x) && !bending.contains(&x.name) && x.look != Look::Floor
        })
        .collect();
    let seen = aims
        .iter()
        .filter(|&&p| !solid.iter().any(|x| hides(x, p)))
        .count();
    // the ghosts' cover of the view, on a coarse raster
    let m = cam.view(size.0, size.1).matrix;
    let (gw, gh) = (140usize, 86usize);
    let mut cover = vec![false; gw * gh];
    for x in drawn.iter().filter(|x| ghosted.contains(&x.name)) {
        let to = |p: [f32; 3]| {
            let c = |r: usize| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r];
            let w = c(3);
            (w > 1e-6).then(|| {
                (
                    ((c(0) / w) + 1.0) * 0.5 * gw as f32,
                    (1.0 - c(1) / w) * 0.5 * gh as f32,
                )
            })
        };
        for t in &x.mesh.tri {
            let [Some(a), Some(b), Some(c)] = t.map(|i| to(x.mesh.pos[i as usize])) else {
                continue;
            };
            let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
            if area.abs() < 1e-6 {
                continue;
            }
            let x0 = a.0.min(b.0).min(c.0).max(0.0) as usize;
            let x1 = (a.0.max(b.0).max(c.0).ceil().max(0.0) as usize).min(gw);
            let y0 = a.1.min(b.1).min(c.1).max(0.0) as usize;
            let y1 = (a.1.max(b.1).max(c.1).ceil().max(0.0) as usize).min(gh);
            for py in y0..y1 {
                for px in x0..x1 {
                    let q = (px as f32 + 0.5, py as f32 + 0.5);
                    let e = |u: (f32, f32), v: (f32, f32)| {
                        ((v.0 - u.0) * (q.1 - u.1) - (v.1 - u.1) * (q.0 - u.0)) * area.signum()
                    };
                    if e(a, b) >= 0.0 && e(b, c) >= 0.0 && e(c, a) >= 0.0 {
                        cover[py * gw + px] = true;
                    }
                }
            }
        }
    }
    let ghost = cover.iter().filter(|&&c| c).count() as f32 / (gw * gh) as f32;
    (seen as f32 / aims.len() as f32, ghost)
}

/// The see-through measured on the load tray (`BENDSIM_METRIC=1`): at each bend mid-stroke, from
/// the default view and turned 60, 120 and 180 degrees round it, the watched points seen and the
/// view ghosted. Off unless asked; prints a table.
#[test]
#[ignore]
fn see_through_measured_on_the_load_tray() {
    if std::env::var("BENDSIM_METRIC").is_err() {
        return;
    }
    let Some(mut h) = headless() else { return };
    let part = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sheetmetal/load_tray.fg"
    );
    open(&mut h, part);
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    let n = h.gui.shell.sheet.bend_sim.as_ref().unwrap().steps();
    let mut rows = Vec::new();
    for turn in [0f32, 60.0, 120.0, 180.0] {
        let (mut vis, mut gh, mut ms) = (0f32, 0f32, 0f64);
        for k in 0..n {
            {
                let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
                b.follow = true;
                b.step = k;
                b.t = 0.6;
            }
            for _ in 0..4 {
                h.step();
            }
            if turn != 0.0 {
                let v = h.gui.view.as_mut().unwrap();
                let mut c = v.nav.camera;
                c.yaw += turn.to_radians();
                v.nav.jump_to(c);
            }
            let t0 = std::time::Instant::now();
            for _ in 0..16 {
                h.step();
            }
            ms += t0.elapsed().as_secs_f64() * 1000.0 / 16.0;
            let (a, g) = see_metric(&h);
            vis += a;
            gh += g;
        }
        rows.push((turn, vis / n as f32, gh / n as f32, ms / n as f64));
    }
    for (turn, vis, gh, ms) in &rows {
        eprintln!(
            "METRIC turn {turn:>5.0}: watched seen {:>5.1} %, view ghosted {:>5.1} %, frame {ms:.1} ms",
            vis * 100.0,
            gh * 100.0
        );
    }
}

/// David's see-through, held (the coordinator: view-dependent and minimal): on the load tray at
/// its last bend's stroke, from the default view and from behind, every watched point (the bend's
/// zone, the punch's contact) is seen through what hides it, and the ghosts cover little of the
/// view (before: 88 % seen from behind, 11 % of the view ghosted).
#[test]
fn only_what_hides_the_bend_is_seen_through() {
    let Some(mut h) = headless() else { return };
    let part = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sheetmetal/load_tray.fg"
    );
    open(&mut h, part);
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    let last = h.gui.shell.sheet.bend_sim.as_ref().unwrap().steps() - 2;
    for turn in [0f32, 180.0] {
        {
            let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
            b.follow = true;
            b.step = last;
            b.t = 0.6;
        }
        for _ in 0..4 {
            h.step();
        }
        if turn != 0.0 {
            let v = h.gui.view.as_mut().unwrap();
            let mut c = v.nav.camera;
            c.yaw += turn.to_radians();
            v.nav.jump_to(c);
        }
        for _ in 0..16 {
            h.step();
        }
        let (seen, ghost) = see_metric(&h);
        assert!(
            seen >= 0.97,
            "turned {turn}: {:.1} % of the bend seen",
            seen * 100.0
        );
        assert!(
            ghost <= 0.2,
            "turned {turn}: {:.1} % of the view ghosted",
            ghost * 100.0
        );
        assert!(h.gui.bend.whole());
    }
}

/// A staged program's clip (BENDSIM-TOOLS): the part on a library press, its tools laid out as
/// stations along the bed by the planner, every bend at its station. Off unless BENDSIM_CLIP
/// names a folder; BENDSIM_CLIP_PART the part, BENDSIM_CLIP_PRESS the press
/// (default trumpf-trubend-5130). Frames frame-NNNN.ppm, and the plan's stations on stderr.
#[test]
fn a_staged_clip() {
    let (Ok(dir), Ok(part)) = (std::env::var("BENDSIM_CLIP"), std::env::var("BENDSIM_CLIP_PART")) else {
        return;
    };
    let press = std::env::var("BENDSIM_CLIP_PRESS").unwrap_or_else(|_| "trumpf-trubend-5130".into());
    // BENDSIM_CLIP_SIZE=1920x1080: the window's size (default 1400x860)
    let size = std::env::var("BENDSIM_CLIP_SIZE")
        .ok()
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some((w.parse::<f32>().ok()?, h.parse::<f32>().ok()?))
        })
        .unwrap_or((1400.0, 860.0));
    let Ok(mut h) = Headless::new(size, 1.0) else { return };
    h.step();
    open(&mut h, &part);
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    // BENDSIM_CLIP_PRESS=none: the stand-in tooling (no stations); BENDSIM_CLIP_PUNCHES=a,b and
    // BENDSIM_CLIP_SEQUENCE=name,name: the tools and the order asked (a collision case)
    let list = |k: &str| {
        std::env::var(k)
            .ok()
            .map(|v| v.split(',').map(str::to_string).collect::<Vec<_>>())
    };
    let staged = press != "none";
    let choice = Choice {
        press: staged.then_some(press),
        stations_auto: staged,
        punches: list("BENDSIM_CLIP_PUNCHES"),
        dies: list("BENDSIM_CLIP_DIES"),
        sequence: list("BENDSIM_CLIP_SEQUENCE"),
        ..Choice::default()
    };
    h.gui
        .shell
        .sheet
        .bend_sim
        .as_mut()
        .unwrap()
        .redo(choice, "std-eu");
    for _ in 0..4 {
        h.step();
    }
    {
        let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
        let sim = b.sim.as_ref().expect("simulated");
        assert!(sim.plan.feasible || !staged, "{}", h.gui.shell.status);
        for (k, s) in sim.stage.iter().flat_map(|st| st.stations.iter()).enumerate() {
            eprintln!(
                "station {} {} {} y {} bends {:?}",
                k + 1,
                s.punch,
                s.die,
                s.y,
                s.steps
            );
        }
    }
    // BENDSIM_CLIP_VIEW=overview: the whole press, the eye held (the layout and the part's
    // moves between stations in view), not following the bend
    if std::env::var("BENDSIM_CLIP_VIEW").is_ok_and(|v| v == "overview") {
        h.gui.shell.sheet.bend_sim.as_mut().unwrap().follow = false;
        frame(&mut h, Framing::Overview);
        for _ in 0..8 {
            h.step();
        }
    }
    let total = h
        .gui
        .shell
        .env_timeline()
        .expect("the timeline")
        .duration(&h.gui.shell);
    let n: usize = std::env::var("BENDSIM_CLIP_FRAMES")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(360);
    // settled on the first instant before the first frame
    h.gui.shell.env_timeline().unwrap().seek(&mut h.gui.shell, 0.0);
    for _ in 0..20 {
        h.step();
    }
    for i in 0..n {
        let tl = h.gui.shell.env_timeline().unwrap();
        tl.seek(&mut h.gui.shell, total * i as f64 / (n - 1) as f64);
        h.step();
        std::fs::write(format!("{dir}/frame-{i:04}.ppm"), h.ppm()).unwrap();
    }
    eprintln!("clip: {n} frames over {total} s of program");
}

/// A re-plan never silently replaces what the user set in the bending document (UX-LEAD's P1,
/// BENDSIM-TOOLS' audit): with the same press and tools it plans the order around the document's
/// own stations (a station the user moved stays where they put it); another press or tooling asks
/// first, and replacing is undone by Undo.
#[test]
fn a_replan_keeps_the_users_stations() {
    let Some(mut h) = headless() else { return };
    enter(&mut h, CHANNEL, "replan", "trumpf-trubend-5130", None);
    let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
    b.write_doc().expect("the program written");
    let file = b.companion();
    let src = std::fs::read_to_string(&file).unwrap();
    // the user moves the first station 40 mm along the bed
    let at = src.find("y: ").expect("a station's place") + 3;
    let end = at + src[at..].find(',').unwrap();
    let y: f64 = src[at..end].trim().parse().unwrap();
    let moved = format!("{}{}{}", &src[..at], y + 40.0, &src[end..]);
    std::fs::write(&file, &moved).unwrap();
    b.reload();
    let mine = format!("y: {}", y + 40.0);
    // simulated again, the same press and tools: planned around the user's stations
    let (c, t) = (b.choice.clone(), b.tooling.clone());
    b.redo(c, &t);
    assert!(b.pending_replace.is_none(), "nothing to ask");
    let now = std::fs::read_to_string(&file).unwrap();
    assert!(
        now.contains(&mine),
        "the station stays where the user put it:\n{now}"
    );
    assert!(b.sim.as_ref().is_ok_and(|s| s.plan.feasible), "{:?}", b.note);
    // a bend locked where it cannot pass (its station's stacks cut to 10 mm): refused, by name,
    // nothing written (BENDSIM-TOOLS' replan)
    let line = now
        .lines()
        .find(|l| l.contains("= bend("))
        .expect("a bend")
        .to_string();
    let st = line
        .split("at: ")
        .nth(1)
        .and_then(|r| r.split([',', ')']).next())
        .expect("its station")
        .trim()
        .to_string();
    let sline = now
        .lines()
        .find(|l| l.starts_with(&format!("{st} = station(")))
        .expect("the station")
        .to_string();
    let short = cut_lengths(&sline);
    let locked = now
        .replace(&line, &line.replacen(')', ", lock: true)", 1))
        .replace(&sline, &short);
    std::fs::write(&file, &locked).unwrap();
    b.reload();
    let (c, t) = (b.choice.clone(), b.tooling.clone());
    b.redo(c, &t);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), locked, "nothing written");
    let note = b.note.clone().unwrap_or_default();
    assert!(
        note.contains("locked") && note.contains("unlock"),
        "{note}\n{locked}"
    );
    std::fs::write(&file, &now).unwrap();
    b.reload();
    // stations laid out afresh: asked first, nothing written until the user says so
    let mut c = b.choice.clone();
    c.stations_auto = true;
    let t = b.tooling.clone();
    b.redo(c, &t);
    assert!(b.pending_replace.is_some(), "asked before replacing");
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        now,
        "nothing written yet"
    );
    let ask = b.pending_replace.clone();
    // Keep mine is the default: Enter keeps, and so does Esc (UX-LEAD)
    for key in [egui::Key::Enter, egui::Key::Escape] {
        h.gui.shell.sheet.bend_sim.as_mut().unwrap().pending_replace = ask.clone();
        h.step();
        h.step();
        h.key(key);
        h.step();
        let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
        assert!(b.pending_replace.is_none(), "{key:?}: the question answered");
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            now,
            "{key:?} kept the user's"
        );
    }
    let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
    let (c, t) = ask.unwrap();
    b.redo_replacing(c, &t);
    assert!(b.pending_replace.is_none());
    // replaced, and undone
    b.doc_step(true).expect("undo");
    assert!(
        std::fs::read_to_string(&file).unwrap().contains(&mine),
        "undo brings the user's station back"
    );
}

/// The punch and the part move on one clock (David: "the tool and the part bending do not line
/// up"): every frame of every step, the punch tip never passes into the sheet, and while the bend
/// turns the tip stands on the sheet, the turn growing only as the tip goes down. On the bench
/// tooling and on the TruBend's five stations, whose 300 mm punch stands taller than the press's
/// 385 mm open height leaves room for (the open height check fails: the punch then stood 15 mm
/// under the sheet at the stroke's top and came up through the part).
#[test]
fn the_punch_meets_the_bend_every_frame() {
    const TOL: f64 = 0.02;
    let Some(mut h) = headless() else { return };
    let part = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sheetmetal/load_tray.fg"
    );
    open(&mut h, part);
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    for staged in [false, true] {
        if staged {
            let c = Choice {
                press: Some("trumpf-trubend-5130".into()),
                stations_auto: true,
                ..Choice::default()
            };
            h.gui.shell.sheet.bend_sim.as_mut().unwrap().redo(c, "std-eu");
            h.step();
        }
        let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
        let sim = b.sim.as_ref().unwrap();
        let mut sc = forge_brake::scene::Scene::new(&sim.model, &sim.tooling, &sim.plan);
        for k in 0..sim.plan.steps.len() {
            let rp = sim.tooling.punches[sim.plan.steps[k].setup.punch].tip_radius;
            // the punch left standing on the sheet only where the open height check fails
            if sc.pose(k, 0.0).tip <= sc.pose(k, 0.4).tip {
                let st = &sim.plan.steps[k];
                assert!(
                    st.failures()
                        .iter()
                        .any(|c| c.rule == forge_brake::sim::Rule::OpenHeight),
                    "staged {staged} step {k}: no room over the sheet, the open height check passed"
                );
            }
            let mut last: Option<forge_brake::scene::Pose> = None;
            for i in 0..=120 {
                let t = i as f64 / 120.0;
                let pose = sc.pose(k, t);
                let bodies = sc.bodies(k, t);
                // the tip's drawn bottom is the pose's (to the arc's sampling: a chord's sag)
                let low = bodies
                    .iter()
                    .filter(|x| x.name.starts_with("punch:") && x.name.ends_with("/tip"))
                    .flat_map(|x| x.mesh.pos.iter())
                    .map(|p| p[2] as f64)
                    .fold(f64::MAX, f64::min);
                assert!(
                    (low - pose.tip).abs() < 0.01,
                    "step {k} t {t}: tip drawn at {low}, posed at {}",
                    pose.tip
                );
                // the sheet's nearest point to the tip's round (inside it: in the punch)
                let gap = bodies
                    .iter()
                    .filter(|x| x.look == forge_brake::scene::Look::BendZone)
                    .flat_map(|x| x.mesh.pos.iter())
                    .map(|q| {
                        let (x, z) = (q[0] as f64, q[2] as f64 - (pose.tip + rp));
                        (x * x + z * z).sqrt() - rp
                    })
                    .fold(f64::MAX, f64::min);
                let what = format!(
                    "staged {staged} step {k} t {t:.3} {:?} phi {:.2} tip {:.3} gap {gap:.3}",
                    pose.stage,
                    pose.phi.to_degrees(),
                    pose.tip
                );
                use forge_brake::scene::Stage;
                if pose.stage != Stage::Handle {
                    assert!(gap > -TOL, "the punch in the sheet: {what}");
                }
                if matches!(pose.stage, Stage::Bend | Stage::Springback) {
                    assert!(gap < TOL, "the bend turns with the punch off the sheet: {what}");
                }
                if let Some(a) = last.filter(|a| a.stage == pose.stage && pose.stage == Stage::Bend) {
                    // the turn grows as the tip goes down, and only then
                    assert!(
                        pose.phi >= a.phi && pose.tip <= a.tip,
                        "the turn against the stroke: {what}"
                    );
                    assert_eq!(
                        pose.phi > a.phi,
                        pose.tip < a.tip,
                        "the turn without the stroke: {what}"
                    );
                }
                if let Some(a) = last.filter(|a| pose.stage == Stage::Approach && a.stage == Stage::Approach)
                {
                    assert!(pose.tip <= a.tip, "the punch going up on its way down: {what}");
                }
                last = Some(pose);
            }
        }
    }
}

/// UX-LEAD's shots (off unless BENDSIM_SHOTS names a folder): the TruBend's stations, the ones not
/// at work dimmed; the re-plan question over a written bending document; bend 4's handling
/// caption on its backing plate.
#[test]
fn shots_for_ux_lead() {
    if std::env::var("BENDSIM_SHOTS").is_err() {
        return;
    }
    let Some(mut h) = headless() else { return };
    let d = scratch("ux-lead");
    let f = d.join("load_tray.fg");
    let src = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sheetmetal/load_tray.fg"
    );
    std::fs::copy(src, &f).unwrap();
    open(&mut h, f.to_str().unwrap());
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    let c = Choice {
        press: Some("trumpf-trubend-5130".into()),
        stations_auto: true,
        ..Choice::default()
    };
    let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
    b.redo(c, "std-eu");
    h.step();
    let n = h
        .gui
        .shell
        .sheet
        .bend_sim
        .as_ref()
        .unwrap()
        .sim
        .as_ref()
        .unwrap()
        .plan
        .steps
        .len();
    for k in 0..n {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.playing = false;
        b.step = k;
        b.t = 0.6;
        shot(&mut h, &format!("ux-stations-bend-{}", k + 1));
    }
    // bend 4, the operator turning the part: the caption on its plate
    let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
    b.step = 3.min(n - 1);
    b.t = 0.07;
    shot(&mut h, "ux-bend-4-handling");
    // the re-plan question, over the written document
    let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
    b.write_doc().expect("the program written");
    b.reload();
    b.pending_replace = Some((b.choice.clone(), b.tooling.clone()));
    shot(&mut h, "ux-replan-question");
}

/// A station's line with every stack length (`length: N`, `punch_length: N`, `die_length: N`)
/// cut to 10 mm.
fn cut_lengths(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(i) = rest.find("length: ") {
        out.push_str(&rest[..i + 8]);
        rest = &rest[i + 8..];
        let end = rest.find([',', ')']).unwrap_or(rest.len());
        out.push_str("10");
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// No silent button (BUTTON-AUDITOR): with no feasible order, Simulate Again and Suggest Order
/// say so with the first reason; at bend 1, First and Previous are off (clicked, nothing moves).
#[test]
fn no_feasible_order_is_said_and_bend_one_has_no_previous() {
    let Some(mut h) = headless() else { return };
    enter(
        &mut h,
        NARROW_U,
        "narrow-say",
        "bystronic-xpert-pro-100-3100",
        Some(&["bystronic/P6_RF-A_H175_R1.0"]),
    );
    assert!(!h.gui.shell.sheet.bend_sim.as_ref().unwrap().sim.as_ref().unwrap().plan.feasible);
    for tool in ["tool:Simulate Again", "tool:Suggest Order"] {
        h.gui.shell.status.clear();
        click(&mut h, tool);
        let st = h.gui.shell.status.clone();
        assert!(st.starts_with("No feasible order: left fails: ") && st.contains("it closes on the punch"), "{tool}: {st:?}");
    }
    shot(&mut h, "ux-no-feasible-order");
    let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
    b.go(0);
    for tool in ["tool:First Bend", "tool:Previous Bend"] {
        h.click(tool);
        h.step();
        assert_eq!(h.gui.shell.sheet.bend_sim.as_ref().unwrap().step, 0, "{tool}");
    }
    click(&mut h, "tool:Next Bend");
    assert_eq!(h.gui.shell.sheet.bend_sim.as_ref().unwrap().step, 1);
}

/// The press overview labels its stations (UX-LEAD): a chip each, over its place on the bed; a
/// click on one goes to that station's first bend, framed at its tools.
#[test]
fn the_overview_labels_its_stations_and_a_chip_selects_one() {
    let Some(mut h) = headless() else { return };
    let part = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/sheetmetal/load_tray.fg");
    open(&mut h, part);
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    let c = Choice {
        press: Some("trumpf-trubend-5130".into()),
        stations_auto: true,
        ..Choice::default()
    };
    h.gui.shell.sheet.bend_sim.as_mut().unwrap().redo(c, "std-eu");
    h.step();
    let (n, want) = {
        let sim = h.gui.shell.sheet.bend_sim.as_ref().unwrap().sim.as_ref().unwrap();
        let n = sim.tooling.stations.len();
        // the last station's first bend
        (n, sim.plan.steps.iter().position(|s| s.station == Some(n.saturating_sub(1))))
    };
    assert!(n >= 2, "stations: {n}");
    click(&mut h, "tool:Press Overview");
    for _ in 0..30 {
        h.step();
    }
    shot(&mut h, "ux-overview-station-chips");
    // the press's body hiding the tool row is seen through; no tool is
    let ghosts = h.gui.bend.ghosted_names();
    assert!(
        ghosts.iter().any(|g| g.starts_with("frame:") || g == "ram" || g == "bed"),
        "the press body ghosted: {ghosts:?}"
    );
    assert!(
        !ghosts.iter().any(|g| g.contains("punch:") || g.contains("die:")),
        "a tool ghosted: {ghosts:?}"
    );
    for k in 1..=n {
        assert!(h.placed(&format!("env:bendsim:station:{k}")).is_some(), "station {k}'s chip");
    }
    let want = want.unwrap();
    click(&mut h, &format!("env:bendsim:station:{n}"));
    let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
    assert_eq!(b.step, want);
    assert!(!b.overview, "the overview left for the station");
}

/// A 480 px window (BUTTON-AUDITOR: the view some 200 px, or less): entering Bend Simulation
/// does not panic; a bar too narrow to draw is left out, and stepping still works.
#[test]
fn a_480_px_window_enters_without_a_panic() {
    let Ok(mut h) = Headless::new((480.0, 860.0), 1.0) else {
        return;
    };
    h.step();
    let part = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/sheetmetal/load_tray.fg");
    open(&mut h, part);
    // the ribbon folds at this width: into the environment as its button would
    h.gui
        .shell
        .out
        .push(forge_render::shell::Action::EnterEnv(EnvId::BendSim));
    for _ in 0..10 {
        h.step();
    }
    let b = h.gui.shell.sheet.bend_sim.as_mut().expect("in the simulation");
    b.go(1);
    b.t = 0.5;
    for _ in 0..10 {
        h.step();
    }
    if let Some(view) = h.gui.shell.place("env:bendsim:view") {
        // the bar's narrowest band (MIN_BAR, 240 px)
        if view.width() - 24.0 < 240.0 {
            assert!(h.gui.shell.place("env:bendsim:timeline").is_none());
        }
    }
}

/// The eye goes to every station the part moves to (David: "the camera only moves on the last
/// tool"): played as a recording plays it (Follow bend), at each step's bend the eye's target
/// stands at that step's station along the bed, every station change, not only those that put
/// the part near the frame's edge.
#[test]
fn the_eye_goes_to_every_station() {
    let Some(mut h) = headless() else { return };
    let part = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/sheetmetal/load_tray.fg");
    open(&mut h, part);
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    let c = Choice {
        press: Some("trumpf-trubend-5130".into()),
        stations_auto: true,
        ..Choice::default()
    };
    h.gui.shell.sheet.bend_sim.as_mut().unwrap().redo(c, "std-eu");
    for _ in 0..4 {
        h.step();
    }
    let total = h.gui.shell.env_timeline().unwrap().duration(&h.gui.shell);
    let n = 240;
    h.gui.shell.env_timeline().unwrap().seek(&mut h.gui.shell, 0.0);
    for _ in 0..20 {
        h.step();
    }
    let mut seen = std::collections::BTreeMap::new();
    for i in 0..n {
        let tl = h.gui.shell.env_timeline().unwrap();
        tl.seek(&mut h.gui.shell, total * i as f64 / (n - 1) as f64);
        h.step();
        let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
        // the middle of the bend: the eye has had the handling's end, the gauge and the approach
        if (b.t - 0.55).abs() > 0.03 {
            continue;
        }
        let sim = b.sim.as_ref().unwrap();
        let st = &sim.plan.steps[b.step];
        let y = sim.tooling.stations[st.station.expect("staged")].y as f32;
        let eye = h.gui.view.as_ref().unwrap().nav.camera.target[1];
        seen.insert(b.step, (st.station.unwrap(), y, eye));
    }
    // why each bend has its tools (David): the first bend at a station chosen from what was
    // tried, a later one at the same station kept
    {
        let sim = h.gui.shell.sheet.bend_sim.as_ref().unwrap().sim.as_ref().unwrap();
        let (first, list) = forge_render::shell::bendsim::why_tools(sim, None, 0);
        assert!(first.starts_with("chosen from ") && first.contains("setups tried"), "{first}");
        assert!(!list.is_empty());
        let (second, _) = forge_render::shell::bendsim::why_tools(sim, None, 1);
        assert!(second.starts_with("kept: station 1"), "{second}");
    }
    // the shots: step 3's move to station 2 (its caption), and the overview's chips
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.playing = false;
        b.step = 2;
        b.t = 0.2;
    }
    shot(&mut h, "ux-station-change-caption");
    click(&mut h, "tool:Press Overview");
    for _ in 0..20 {
        h.step();
    }
    shot(&mut h, "ux-overview-station-chips");
    let stations: std::collections::BTreeSet<usize> = seen.values().map(|x| x.0).collect();
    assert!(stations.len() >= 3, "{seen:?}");
    for (k, (s, y, eye)) in &seen {
        assert!((eye - y).abs() < 15.0, "step {} at S{}: the eye at y {eye:.0}, the station at {y:.0}", k + 1, s + 1);
    }
}

/// Stations set up by hand (David: "a good way to set up or manually change the tools used"):
/// a station's punch swapped, a station added and an unused one removed, each an edit of the
/// bending document simulated again at once, Undo bringing it back.
#[test]
fn a_stations_tools_are_changed_by_hand() {
    use forge_lang::error::Op;
    let Some(mut h) = headless() else { return };
    let d = scratch("pickers");
    let f = d.join("load_tray.fg");
    let src = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/sheetmetal/load_tray.fg");
    std::fs::copy(src, &f).unwrap();
    open(&mut h, f.to_str().unwrap());
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    let c = Choice {
        press: Some("trumpf-trubend-5130".into()),
        stations_auto: true,
        ..Choice::default()
    };
    let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
    b.redo(c, "std-eu");
    b.write_doc().expect("written");
    b.reload();
    let file = b.companion();
    let stations = |b: &forge_render::shell::bendsim::BendSimUi| b.sim.as_ref().unwrap().tooling.stations.len();
    let n0 = stations(b);
    // swap station 1's punch for another the simulation has
    let sim = b.sim.as_ref().unwrap();
    let s1 = &sim.tooling.stations[0];
    let cur = sim.tooling.punches[s1.punch].id.clone();
    // a punch the library has that is on no station yet (the pickers offer the library's)
    let cat = b.catalog.as_ref().and_then(|c| c.as_ref().ok()).expect("the library");
    let on_bed: Vec<String> = sim.tooling.punches.iter().map(|p| p.id.clone()).collect();
    let other = cat
        .punches
        .iter()
        .filter(|r| r.punch.is_ok() && r.id.starts_with("trumpf/") && !on_bed.contains(&r.id))
        .map(|r| r.id.clone())
        .next()
        .expect("another Trumpf punch");
    let name = b.doc.as_ref().unwrap().stations[0].clone();
    b.doc_op(&Op::SetArg { id: name.clone(), arg: "punch".into(), text: format!("{other:?}") })
        .expect("the punch swapped");
    let sim = b.sim.as_ref().expect("simulated again");
    assert_eq!(sim.tooling.punches[sim.tooling.stations[0].punch].id, other);
    // the pickers as the user sees them, after the swap (Machine & Tools)
    b.pane = forge_render::shell::bendsim::Pane::Setup;
    shot(&mut h, "ux-station-pickers");
    let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
    let sim = b.sim.as_ref().unwrap();
    // a station added (past the others), then removed (no bend works at it)
    let before = std::fs::read_to_string(&file).unwrap();
    let first_bend = b.doc.as_ref().unwrap().bends.first().cloned();
    b.doc_op(&Op::AddFeature {
        before: first_bend,
        text: format!("s9 = station(punch: {other:?}, die: {:?}, y: 1200, punch_length: 100, die_length: 100)", sim.tooling.dies[0].id),
    })
    .expect("added");
    assert_eq!(stations(b), n0 + 1);
    b.doc_op(&Op::DeleteFeature { id: "s9".into() }).expect("removed");
    assert_eq!(stations(b), n0);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    // undo the swap: station 1's punch as it was
    b.doc_step(true).expect("undo");
    b.doc_step(true).expect("undo");
    b.doc_step(true).expect("undo");
    let sim = b.sim.as_ref().unwrap();
    assert_eq!(sim.tooling.punches[sim.tooling.stations[0].punch].id, cur);
}

/// The narrow, tall channel BENDSIM-TOOLS' contacts are tested on (a 30 x 200 plate, 60 mm
/// flanges, straight punches, right then left): its second bend runs the first flange into the
/// punch.
const TALL_CHANNEL: &str = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\n\
sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (30, 200))\n}\n\
base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
right = flange(base.edge(plan.outline[1]), height: 60)\n\
left = flange(base.edge(plan.outline[3]), height: 60)\n";

/// Collisions (David: "exact collision areas per step, tinted"): where the part meets the punch is
/// drawn on the part in red while the bend passes through the region, and not before it; the
/// switch takes it away.
#[test]
fn a_collision_is_drawn_on_the_part_where_it_is() {
    let Some(mut h) = headless() else { return };
    let d = scratch("tall-channel");
    let f = d.join("tall_channel.fg");
    std::fs::write(&f, TALL_CHANNEL).unwrap();
    open(&mut h, f.to_str().unwrap());
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    let c = Choice {
        punches: Some(vec!["std-eu/P85-R08-H120".into(), "std-eu/P88-R08".into()]),
        sequence: Some(vec!["right".into(), "left".into()]),
        ..Choice::default()
    };
    h.gui.shell.sheet.bend_sim.as_mut().unwrap().redo(c, "std-eu");
    h.step();
    let at = |h: &mut Headless, t: f64| {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.playing = false;
        b.step = 1;
        b.t = t;
        for _ in 0..3 {
            h.step();
        }
        // the proven collisions drawn (a Static contact without a proven depth, the die's here,
        // is drawn at every instant in amber: not counted)
        h.gui
            .bend
            .drawn()
            .iter()
            .filter(|x| x.name.starts_with("contact:") && x.hit)
            .count()
    };
    // through the bend: drawn at some instant; before the bend (the gauge), never
    let during = (0..=14).map(|i| at(&mut h, 0.4 + 0.35 * i as f64 / 14.0)).max().unwrap();
    assert!(during > 0, "no proven collision drawn through the bend");
    // the shot: at 45 degrees of the turn (where the flange closes on the punch), framed on the work
    at(&mut h, 0.4 + 0.35 * 0.5);
    frame(&mut h, Framing::Work);
    shot(&mut h, "ux-collision-region");
    assert_eq!(at(&mut h, 0.2), 0, "a bend's collision drawn before the bend");
    let t_hit = (0..=14)
        .map(|i| 0.4 + 0.35 * i as f64 / 14.0)
        .find(|&t| at(&mut h, t) > 0)
        .unwrap();
    // the switch off: none drawn
    h.gui.shell.sheet.bend_sim.as_mut().unwrap().collisions = false;
    assert_eq!(at(&mut h, t_hit), 0);
}

/// Frame 0 of a showcase draws no collision when every check passes (the coordinator: red means a
/// collision verdict). The view's caches were keyed by the simulation's address, and a re-plan's
/// new simulation can take the old one's: seeking back to the first instant then drew the old
/// plan's bodies, its failing tools red. Each showcase's setup: entered (the stand-in plan drawn
/// at the first instant), re-planned as the clip asks, sought to the start.
#[test]
fn a_replans_first_frame_draws_its_own_plan() {
    let ex = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/sheetmetal/");
    let cases: [(&str, Choice); 2] = [
        (
            "hem_joggle.fg",
            Choice {
                press: Some("trumpf-trubend-5130".into()),
                stations_auto: true,
                punches: Some(
                    ["trumpf/OW222-K", "wila/BIU-041", "wila/HU-001+INZU-002-Z6"].map(String::from).to_vec(),
                ),
                dies: Some(
                    ["trumpf/EV002_30deg", "wila/OZU-384", "wila/HU-006+INZU-002-Z6"].map(String::from).to_vec(),
                ),
                ..Choice::default()
            },
        ),
        (
            "load_tray.fg",
            Choice {
                press: Some("trumpf-trubend-5130".into()),
                stations_auto: true,
                ..Choice::default()
            },
        ),
    ];
    for (part, choice) in cases {
        let Some(mut h) = headless() else { return };
        open(&mut h, &format!("{ex}{part}"));
        click(&mut h, "tab:Environments");
        click(&mut h, "tool:Bend Simulation");
        // the plan on entry drawn at the first instant
        {
            let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
            b.playing = false;
            b.step = 0;
            b.t = 0.0;
        }
        for _ in 0..4 {
            h.step();
        }
        h.gui.shell.sheet.bend_sim.as_mut().unwrap().redo(choice, "std-eu");
        for _ in 0..4 {
            h.step();
        }
        h.gui.shell.env_timeline().unwrap().seek(&mut h.gui.shell, 0.0);
        h.step();
        let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
        let sim = b.sim.as_ref().expect("simulated");
        assert!(
            sim.plan.feasible && sim.plan.steps.iter().all(|s| s.pass()),
            "{part}: the showcase's plan passes"
        );
        let red: Vec<String> = h.gui.bend.drawn().iter().filter(|x| x.hit).map(|x| x.name.clone()).collect();
        assert!(red.is_empty(), "{part}: frame 0 draws {red:?} red, every check passing");
    }
}

/// Showcase 4 (off unless BENDSIM_PICKERS_CLIP names a folder): the stations set up by hand. The
/// TruBend tray as planned, passing; station 1's punch swapped in its picker for a gooseneck,
/// which meets the ram at once (said red); Simulate Again plans around the stations as they now
/// are; the new program played. 1920 x 1080 frames, frame-NNNN.ppm.
#[test]
fn a_pickers_clip() {
    use forge_lang::error::Op;
    let Ok(dir) = std::env::var("BENDSIM_PICKERS_CLIP") else { return };
    let Ok(mut h) = Headless::new((1920.0, 1080.0), 1.0) else { return };
    h.step();
    let d = scratch("pickers-clip");
    let f = d.join("load_tray.fg");
    let src = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/sheetmetal/load_tray.fg");
    std::fs::copy(src, &f).unwrap();
    open(&mut h, f.to_str().unwrap());
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    let c = Choice {
        press: Some("trumpf-trubend-5130".into()),
        stations_auto: true,
        ..Choice::default()
    };
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        b.redo(c, "std-eu");
        b.write_doc().expect("written");
        b.reload();
        b.follow = true;
    }
    let mut k = 0usize;
    let mut hold = |h: &mut Headless, frames: usize, set: &dyn Fn(&mut forge_render::shell::bendsim::BendSimUi)| {
        for _ in 0..frames {
            set(h.gui.shell.sheet.bend_sim.as_mut().unwrap());
            h.step();
            std::fs::write(format!("{dir}/frame-{k:04}.ppm"), h.ppm()).unwrap();
            k += 1;
        }
    };
    // as planned: Machine & Tools, bend 1 being made, every check passing
    hold(&mut h, 75, &|b| {
        b.pane = forge_render::shell::bendsim::Pane::Setup;
        b.playing = false;
        b.step = 0;
        b.t = 0.6;
    });
    // station 1's punch picked: the gooseneck OW200/K
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        let name = b.doc.as_ref().unwrap().stations[0].clone();
        b.doc_op(&Op::SetArg { id: name, arg: "punch".into(), text: "\"trumpf/OW200-K\"".into() })
            .expect("the punch picked");
    }
    let at = {
        let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
        let st = &b.sim.as_ref().unwrap().plan.steps[0];
        let phimax = st.angle + st.overbend;
        st.failures().iter().find_map(|c| c.phi).map_or(0.6, |p| 0.4 + 0.35 * (p / phimax).clamp(0.0, 1.0))
    };
    hold(&mut h, 105, &|b| {
        b.playing = false;
        b.step = 0;
        b.t = at;
    });
    // Simulate Again: planned around the stations as they now stand
    {
        let b = h.gui.shell.sheet.bend_sim.as_mut().unwrap();
        let (c, t) = (b.choice.clone(), b.tooling.clone());
        b.redo(c, &t);
    }
    hold(&mut h, 45, &|b| {
        b.playing = false;
        b.step = 0;
        b.t = 0.0;
    });
    // the new program, played
    let n = h.gui.shell.sheet.bend_sim.as_ref().unwrap().steps();
    let frames = 120 * n.max(1);
    for i in 0..frames {
        let x = i as f64 / frames as f64 * n as f64;
        hold(&mut h, 1, &|b| {
            b.pane = forge_render::shell::bendsim::Pane::Program;
            b.playing = false;
            b.set_position(x);
        });
    }
    let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
    eprintln!("pickers clip: {k} frames; re-planned feasible {:?}; note {:?}", b.sim.as_ref().map(|s| s.plan.feasible), b.note);
}

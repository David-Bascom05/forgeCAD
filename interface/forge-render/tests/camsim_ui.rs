//! The simulation in the app, native and headless: the flange's machining document opens on
//! Manufacture, its run is simulated and driven by the simulation's own words
//! (`Gui::camsim_command`); a turning document gets no milling simulation (its run is a lathe's).
//! `FORGE_UI_SHOTS=DIR` keeps the review set (docs/ux/STANDARD.md §10: 1400×860, 1600×1000 and
//! 1024×700).

use forge_render::gui::{Gui, Headless};
use std::path::{Path, PathBuf};

fn machined() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined")).to_path_buf()
}

/// A copy of `files` from examples/machined in a directory of the test's own.
fn copied(tag: &str, files: &[&str]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fg-camsim-ui-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in files {
        std::fs::copy(machined().join(f), dir.join(f)).unwrap();
    }
    dir
}

fn run_done(g: &Gui) -> bool {
    g.shell
        .cam
        .as_ref()
        .is_some_and(|w| w.run.is_some() && w.running.is_none() && !w.want_run)
}

/// Opens `file` at `size`, waits for its run (and, if `sim`, its simulation).
fn open(file: &Path, size: (f32, f32), sim: bool) -> Option<Headless> {
    let mut h = match Headless::new(size, 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 120.0 {
        let ready = |g: &Gui| run_done(g) && (!sim || g.shell.camsim.is_some());
        if h.until(ready) {
            break;
        }
    }
    assert!(run_done(&h.gui), "never ran: {}", h.gui.shell.status);
    for _ in 0..6 {
        h.step();
    }
    Some(h)
}

fn shot(h: &mut Headless, name: &str) {
    for _ in 0..8 {
        h.step();
    }
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS")
        && let Ok(png) = h.png()
    {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(Path::new(&dir).join(format!("{name}.png")), png).unwrap();
    }
}

/// The flange plays, at each review size: start, mid-op, the end compared, the move list, the
/// posted code on its machine.
#[test]
fn the_flange_simulates_in_the_app() {
    let dir = copied("flange", &["flange.fg", "flange_cam.fg"]);
    for (w, hgt) in [(1400.0, 860.0), (1600.0, 1000.0), (1024.0, 700.0)] {
        let Some(mut h) = open(&dir.join("flange_cam.fg"), (w, hgt), true) else {
            return;
        };
        let tag = format!("{}x{}", w as u32, hgt as u32);
        assert!(h.gui.camsim_state().is_some(), "the simulation's state");
        assert_eq!(h.gui.shell.camsim.as_ref().unwrap().sim.tl.ops.len(), 5);
        h.gui.camsim_command("t=0;part=hidden").unwrap();
        shot(&mut h, &format!("native-sim-01-start-{tag}"));
        h.gui.camsim_command("op=2;at=0.55").unwrap();
        shot(&mut h, &format!("native-sim-02-contour-{tag}"));
        h.gui.camsim_command("at=1;colour=compare;part=hidden").unwrap();
        shot(&mut h, &format!("native-sim-03-end-compare-{tag}"));
        h.gui
            .camsim_command("colour=material;part=hidden;op=3;locate=3,0,40,0.5;tab=moves")
            .unwrap();
        shot(&mut h, &format!("native-sim-04-moves-{tag}"));
        if w == 1400.0 {
            h.gui.camsim_command("tab=checks;source=toolpath").unwrap();
            shot(&mut h, &format!("native-sim-07-checks-{tag}"));
        }
        if h.gui
            .camsim_command("tab=program;source=machine;machine=1;at=0.3")
            .is_ok()
        {
            shot(&mut h, &format!("native-sim-05-machine-{tag}"));
        }
    }
}

/// A turning document is a lathe's run: no milling simulation of it, and no milling errors in
/// the app for it (the web build once showed one).
#[test]
fn a_turning_document_gets_no_milling_simulation() {
    let dir = copied("shaft", &["shaft.fg", "shaft_turn.fg"]);
    let Some(mut h) = open(&dir.join("shaft_turn.fg"), (1400.0, 860.0), false) else {
        return;
    };
    let w = h.gui.shell.cam.as_ref().unwrap();
    assert_eq!(w.run.as_ref().unwrap().process, "turning");
    for _ in 0..30 {
        h.step();
    }
    assert!(
        h.gui.shell.camsim.is_none(),
        "a milling simulation of a turning document"
    );
    shot(&mut h, "native-sim-06-turning-no-sim-1400x860");
}

/// Docked: an environment's properties column hosts the panel (`Shell::camsim_properties`, drawn
/// before the simulation's own chrome each frame); the floating panel then stands down and the
/// view's free part reaches its right edge. Hosting stops, the panel floats again.
#[test]
fn the_panel_docks_in_an_environments_properties() {
    let dir = copied("dock", &["flange.fg", "flange_cam.fg"]);
    let Some(mut h) = open(&dir.join("flange_cam.fg"), (1400.0, 860.0), true) else {
        return;
    };
    let ctx = egui::Context::default();
    let host = |h: &mut Headless| {
        let mut drawn = false;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let r = egui::Rect::from_min_size(egui::pos2(1000.0, 100.0), egui::vec2(340.0, 700.0));
            drawn = h.gui.shell.camsim_properties(ui, r);
        });
        // this test's own context: its font atlas is never uploaded (debug builds check)
        out.textures_delta.clear();
        drawn
    };
    assert!(host(&mut h), "a simulation to host");
    h.step();
    let st = h.gui.shell.camsim.as_ref().unwrap();
    assert!(st.docked, "hosted before the frame: docked");
    assert!(!st.hosted, "taken by the frame");
    // the next frame: the Manufacture environment's properties column hosts it (CAM-UI's
    // adapter, every frame it is in Manufacture), so it stays docked, not floating
    h.step();
    assert!(
        h.gui.shell.camsim.as_ref().unwrap().docked,
        "docked by the environment"
    );
    assert!(h.placed("env:manufacture:properties").is_some());
}

/// The tools as they are drawn (fluted from their records, holders turned), close up: the flange
/// mid-contour, a drill and a tap on the manifold, ball-nose surfacing on the mould.
/// `FORGE_UI_SHOTS=DIR` keeps them.
#[test]
fn realistic_tools_close_up() {
    // (document, files it needs, (op, shot name) per close-up)
    type Shots<'a> = (&'a str, &'a [&'a str], &'a [(&'a str, &'a str)]);
    let shots: [Shots; 3] = [
        (
            "flange_cam.fg",
            &["flange.fg", "flange_cam.fg"],
            &[("prof", "native-tools-01-flange-contour")],
        ),
        (
            "manifold_cam.fg",
            &["manifold.fg", "manifold_cam.fg"],
            &[
                ("bored", "native-tools-02-manifold-drill"),
                ("tap", "native-tools-03-manifold-tap"),
            ],
        ),
        (
            "mold_core_finish_cam.fg",
            &["mold_core.fg", "mold_core_finish_cam.fg"],
            &[("", "native-tools-04-mould-ball")],
        ),
    ];
    for (doc, files, at) in shots {
        let dir = copied(&format!("tools-{doc}"), files);
        let Some(mut h) = open(&dir.join(doc), (1400.0, 860.0), true) else {
            return;
        };
        // the part's own view loads after the run: let it, before the camera is set
        for _ in 0..20 {
            h.step();
        }
        for (op, name) in at {
            let st = h.gui.shell.camsim.as_ref().unwrap();
            let ops = &st.sim.tl.ops;
            // the named op, or the one with a ball cutter
            let k = ops
                .iter()
                .position(|o| o.name == *op)
                .or_else(|| {
                    ops.iter().position(|o| {
                        o.tools.iter().any(|&t| {
                            st.sim.cam.sim_tools()[t as usize].tool.kind == forge_tooling::Kind::Ball
                        })
                    })
                })
                .unwrap_or_else(|| panic!("{doc}: no op {op}"));
            // the start of its first feed: the cutter just over the material, in full view
            let sp = &ops[k];
            let at = st.sim.tl.segs[sp.first..sp.end]
                .iter()
                .find(|g| g.kind != forge_render::camsim::timeline::Kind::Rapid)
                .map_or(sp.t0, |g| g.t0 + 1e-6);
            // the simulation's default framing: the stock and the tool together
            h.gui
                .camsim_command(&format!("t={at};part=hidden;follow=0;fit=sim"))
                .unwrap();
            let st = h.gui.shell.camsim.as_ref().unwrap();
            let (tool, _, _) = st.tool_now().unwrap();
            let lk = &st.sim.looks[tool];
            assert!(lk.tris > 1000, "{doc} {op}: a fluted tool");
            shot(&mut h, name);
            // and the opt-in close-up, following the tool, for the flange's
            if name.contains("flange") {
                h.gui.camsim_command("follow=1;fit=tool").unwrap();
                shot(&mut h, "native-tools-05-flange-close-up");
                h.gui.camsim_command("follow=0").unwrap();
            }
        }
    }
}

/// Issues of two kinds in the Checks tab (UX-LEAD's review): the flange's bolt holes drilled alone
/// under a clear height below the stock's top rapid into it, and drilling alone leaves
/// the plate's material. A place is picked: its kind's row opens to it, highlighted, and the
/// playhead is on it. The first line names the failure in words.
#[test]
fn issues_of_two_kinds_in_the_checks() {
    let dir = copied("issues", &["flange.fg"]);
    let src = "machining\nf = part(\"flange.fg\", group: \"N\")\nm = machine(\"generic-vmc3\")\nt1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\nop1 = setup(f, machine: m, stock: box(margin: (3, 3, 2), below: 3), origin: \"top-center\", tol: 0.01, clear: -0.5, retract: -1)\nbolts = drill(op1, f.bolts, plan: \"auto\")\nprog = program(op1)\n";
    std::fs::write(dir.join("flange_broken_cam.fg"), src).unwrap();
    let Some(mut h) = open(&dir.join("flange_broken_cam.fg"), (1400.0, 860.0), true) else {
        return;
    };
    let st = h.gui.shell.camsim.as_ref().unwrap();
    let kinds = |rule: &str| st.sim.markers.iter().filter(|m| m.rule == rule).count();
    assert!(kinds("rapid_into_stock") > 0, "rapids into the stock");
    assert!(kinds("leftover") > 0, "material left");
    let (line, failed) =
        forge_render::gui_camsim_panel::checked_line(&st.sim.markers, st.sim.cam.leftover_done);
    assert!(failed && line.contains("rapids into the stock"), "{line}");
    let k = st
        .sim
        .markers
        .iter()
        .position(|m| m.rule == "rapid_into_stock")
        .unwrap();
    h.gui
        .camsim_command(&format!("tab=checks;marker={k};part=hidden"))
        .unwrap();
    shot(&mut h, "native-sim-08-issues-1400x860");
    let st = h.gui.shell.camsim.as_ref().unwrap();
    assert_eq!(st.issue, Some(k));
    assert!(!st.issue_new, "opened and scrolled to once");
}

/// What one frame draws: (opaque vertices, translucent ones) at the simulation's time now.
fn drawn(h: &mut Headless) -> (usize, usize) {
    let cam = h.gui.view.as_ref().unwrap().nav.camera;
    let st = h.gui.shell.camsim.as_deref_mut().unwrap();
    st.drawn_key = None;
    let mut world = vec![];
    let solid = forge_render::gui::gui_camsim::geometry(st, &cam, (1400, 860), &mut world);
    (solid.len(), world.len())
}

/// Soft jaws (David): while the bracket's jaws are cut, the part they will hold is not in the
/// machine. It is not drawn unless asked (then faint, where it will sit: the gripping setup's
/// part, in the same frame), and once the jaws are done it is drawn seated in them. No other
/// setup's toolpath is drawn in the jaws' frame (op1's floated over them).
#[test]
fn soft_jaws_hold_no_part_while_they_are_cut() {
    let dir = copied("jaws", &["bracket.fg", "bracket_cam.fg"]);
    let Some(mut h) = open(&dir.join("bracket_cam.fg"), (1400.0, 860.0), true) else {
        return;
    };
    for _ in 0..20 {
        h.step();
    }
    let st = h.gui.shell.camsim.as_ref().unwrap();
    let jaws = st
        .sim
        .setups
        .iter()
        .position(|s| s.jaws_for.is_some())
        .expect("a jaw setup");
    let grip = st.sim.setups[jaws].jaws_for.unwrap();
    assert_eq!(st.sim.setups[grip].name, "op2");
    let part_v = st.sim.part_tris[grip].len() * 3;
    assert!(part_v > 0);
    let cutj = st.sim.tl.ops.iter().find(|o| o.setup == jaws).unwrap().clone();
    // mid-cut, the workspace's selection on op1's setup: nothing of op1's in the jaws' frame
    h.gui
        .camsim_command(&format!(
            "t={};part=solid;paths=1;jawpart=0;tool=0;stock=0;fixture=0",
            0.5 * (cutj.t0 + cutj.t1)
        ))
        .unwrap();
    let (solid_off, world_off) = drawn(&mut h);
    assert_eq!(
        solid_off, 0,
        "nothing solid: no part (the jaw setup's own is the stock)"
    );
    shot(&mut h, "native-sim-09-soft-jaws-cut-1400x860");
    // the reference, asked for: faint (translucent), the gripping setup's part
    h.gui.camsim_command("jawpart=1").unwrap();
    let (solid_on, world_on) = drawn(&mut h);
    assert_eq!(solid_on, 0);
    assert_eq!(world_on - world_off, part_v, "the part, faint");
    // the jaws done: the part seated, solid, asked or not
    h.gui.camsim_command(&format!("jawpart=0;t={}", cutj.t1)).unwrap();
    let (solid_end, _) = drawn(&mut h);
    assert_eq!(solid_end, part_v, "seated in the finished jaws");
    // the paths drawn mid-cut are the jaw setup's alone
    let st = h.gui.shell.camsim.as_ref().unwrap();
    let show = forge_render::camsim::draw::PathShow {
        setup: Some(jaws),
        ops: &vec![true; st.sim.tl.ops.len()],
        rapids: true,
        upto: None,
        played_only: false,
        width: 2.0,
        picked: None,
        values: None,
        selected: None,
    };
    let all = forge_render::camsim::draw::PathShow { setup: None, ..show };
    let eye = forge_render::camsim::draw::Eye {
        at: [0.0, 0.0, 500.0],
        forward: [0.0, 0.0, -1.0],
        parallel: true,
        mm_px: 0.1,
    };
    let (mut a, mut b) = (vec![], vec![]);
    forge_render::camsim::draw::toolpath(&st.sim.tl, &eye, &show, &mut a);
    forge_render::camsim::draw::toolpath(&st.sim.tl, &eye, &all, &mut b);
    assert!(!a.is_empty() && a.len() < b.len(), "{} of {}", a.len(), b.len());
}

/// Tool changes (David): the flange's T1 → T11 (the spot drill) after the bore, and T11 → T12 (the
/// drill) between the bolts' passes, are on the timeline: up clear, the swap for the machine
/// record's 2.5 s (generic-vmc3's changer), across and down; the old tool before half the swap,
/// the new one after; the status line names the new tool.
#[test]
fn tool_changes_are_played() {
    let dir = copied("change", &["flange.fg", "flange_cam.fg"]);
    let Some(mut h) = open(&dir.join("flange_cam.fg"), (1400.0, 860.0), true) else {
        return;
    };
    let st = h.gui.shell.camsim.as_ref().unwrap();
    let tl = &st.sim.tl;
    let tools = st.sim.cam.sim_tools();
    let ts: Vec<(u32, u32)> = tl
        .changes
        .iter()
        .map(|c| (tools[c.from as usize].t, tools[c.to as usize].t))
        .collect();
    assert_eq!(ts, vec![(1, 11), (11, 12)]);
    let c = tl.changes[0];
    assert!(c.cited, "generic-vmc3's changer cites its time");
    let pause: f64 = tl.segs[c.first..]
        .iter()
        .take_while(|g| g.kind == forge_render::camsim::timeline::Kind::Change)
        .filter(|g| g.a == g.b)
        .map(|g| g.dt)
        .sum();
    assert!((pause - 2.5).abs() < 1e-9, "the swap, 2.5 s: {pause}");
    // the old tool up to mid-swap, the new one after; the pieces are no path's move
    let mid = 0.5 * (c.t0 + c.t1);
    let tool_at = |h: &Headless, t: f64| {
        let st = h.gui.shell.camsim.as_ref().unwrap();
        let i = st.sim.tl.seg_at(t).unwrap();
        st.sim.tl.segs[i].tool
    };
    assert_eq!(tool_at(&h, c.t0 + 1e-6), c.from);
    assert_eq!(tool_at(&h, c.t1 - 1e-6), c.to);
    h.gui
        .camsim_command(&format!("t={mid};tab=program;part=hidden"))
        .unwrap();
    shot(&mut h, "native-sim-10-tool-change-1400x860");
}

/// The tool's label never travels with the tool (David): while playing it is not drawn; paused,
/// only with the pointer on the tool (or asked for, as the review shots do).
#[test]
fn the_tool_label_does_not_follow_the_tool() {
    let dir = copied("label", &["flange.fg", "flange_cam.fg"]);
    let Some(mut h) = open(&dir.join("flange_cam.fg"), (1400.0, 860.0), true) else {
        return;
    };
    h.gui.camsim_command("op=2;at=0.5;play").unwrap();
    for _ in 0..6 {
        h.step();
    }
    assert!(h.placed("camsim:toolnote").is_none(), "no label while playing");
    h.gui.camsim_command("pause").unwrap();
    for _ in 0..4 {
        h.step();
    }
    assert!(
        h.placed("camsim:toolnote").is_none(),
        "paused, the pointer elsewhere: no label"
    );
    h.gui.camsim_command("label=1").unwrap();
    for _ in 0..4 {
        h.step();
    }
    assert!(h.placed("camsim:toolnote").is_some(), "asked for: shown");
    // a window of the workspace open over the view (the tool library here; an op dialog the
    // same): nothing of the simulation's floats behind it
    h.gui.shell.cam.as_mut().unwrap().tools_open = true;
    for _ in 0..4 {
        h.step();
    }
    assert!(
        h.gui.shell.cam.as_ref().unwrap().tools_open,
        "the window stays open"
    );
    assert!(
        h.placed("camsim:toolnote").is_none(),
        "no label behind a dialog or window"
    );
}

/// The cut surface as played (task 2 of the realistic simulation): the mould core's ball-end
/// finishing at its end, the stock alone, close. The top is lit through the columns' corners,
/// so a ball's cusps and the stepover show. `FORGE_UI_SHOTS` keeps it.
#[test]
fn the_mould_cores_cut_surface() {
    let dir = copied("surface", &["mold_core.fg", "mold_core_finish_cam.fg"]);
    let Some(mut h) = open(&dir.join("mold_core_finish_cam.fg"), (1400.0, 860.0), true) else {
        return;
    };
    for _ in 0..20 {
        h.step();
    }
    h.gui
        .camsim_command("at=1;colour=material;part=hidden;paths=0;tool=0;follow=0;fit=stock")
        .unwrap();
    let t0 = std::time::Instant::now();
    shot(&mut h, "native-surface-01-mould-finish-1400x860");
    eprintln!("stock drawn and shot: {:.0} ms", t0.elapsed().as_secs_f64() * 1e3);
    // close on the cut mid-way through the steep-and-shallow finishing: the detail window's
    // cusps beside the coarser stock
    let st = h.gui.shell.camsim.as_ref().unwrap();
    let k = st.sim.tl.ops.iter().position(|o| o.name == "ss").unwrap();
    let t = 0.5 * (st.sim.tl.ops[k].t0 + st.sim.tl.ops[k].t1);
    h.gui
        .camsim_command(&format!("t={t};tool=1;detail=1;follow=1;fit=tool"))
        .unwrap();
    shot(&mut h, "native-surface-02-mould-detail-1400x860");
    h.gui.camsim_command("detail=0").unwrap();
    shot(&mut h, "native-surface-03-mould-no-detail-1400x860");
}

/// The flange's posted program played on the Haas VF-2 (CAM-HAAS's cited record, named in the
/// document through the one machine list): the machine drawn from its record's parts, the table
/// carrying the part under the head. `FORGE_UI_SHOTS` keeps the shot.
#[test]
fn the_flange_on_a_haas_vf2() {
    let dir = copied("vf2", &["flange.fg"]);
    let src = std::fs::read_to_string(machined().join("flange_cam.fg"))
        .unwrap()
        .replace("machine(\"generic-vmc3\")", "machine(\"haas-vf-2\")")
        // held in a vise, and placed by the sim: the vise's base on the table, the work offset
        // from that (David: "why no vise? it's floating")
        .replace(
            ", at: (0, 0, -250))",
            ", fixture: vise(\"generic-vise-150\", across: \"y\", clamp: 3))",
        )
        .replace("post: \"fanuc\"", "post: \"haas\"")
        // 6 below the part, 3 of it in the jaws: the profile's 0.5 mm break-through stays 2.5
        // above them
        .replace("below: 3)", "below: 6)");
    std::fs::write(dir.join("flange_vf2_cam.fg"), src).unwrap();
    let Some(mut h) = open(&dir.join("flange_vf2_cam.fg"), (1400.0, 860.0), true) else {
        return;
    };
    for _ in 0..20 {
        h.step();
    }
    let st = h.gui.shell.camsim.as_ref().unwrap();
    assert!(st.sim.cam.ok, "{:?}", st.sim.cam.errors);
    h.gui
        .camsim_command("tab=program;source=machine;machine=1;at=0.3;part=hidden;fit=machine")
        .unwrap();
    let mv = h
        .gui
        .shell
        .camsim
        .as_ref()
        .unwrap()
        .sim
        .machine()
        .expect("the VF-2 runs it");
    assert_eq!(mv.run.def.name, "haas-vf-2");
    assert!(mv.markers.is_empty(), "{:?}", mv.markers.first().map(|m| &m.text));
    assert_eq!(mv.run.seat, Some(0.0), "the vise on the table");
    let notes = forge_render::gui_camsim_panel::machine_notes(h.gui.shell.camsim.as_ref().unwrap());
    assert!(notes.iter().all(|n| !n.contains("No workholding")), "{notes:?}");
    shot(&mut h, "native-machine-01-vf2-flange-1400x860");
    // close on the vise: the stock in its jaws, the vise on the table
    h.gui
        .camsim_command("at=0;part=solid;paths=0;fixture=1;machine=1;follow=0;fit=holding")
        .unwrap();
    // low and from the front: the vise's base on the table's top
    if let Some(v) = h.gui.view.as_mut() {
        v.nav.camera.pitch = 8f32.to_radians();
        v.nav.camera.dist *= 1.6;
        v.mark_dirty();
    }
    shot(&mut h, "native-machine-04-vf2-vise-1400x860");
    // the geometry the collision check takes, see-through over what is drawn
    h.gui.camsim_command("checked=1;machine=1;fit=machine").unwrap();
    shot(&mut h, "native-machine-05-vf2-checked-1400x860");
    h.gui.camsim_command("checked=0").unwrap();
    // the operator's view: the eye stays put in the machine, so as the table carries the part
    // the target moves in the part's frame by just the table's travel
    let target = |h: &Headless| h.gui.view.as_ref().unwrap().nav.camera.target;
    let joints = |h: &Headless, t: f64| {
        let mv = h.gui.shell.camsim.as_ref().unwrap().sim.machine().unwrap();
        let q = mv.joints_at(t);
        let get = |n: &str| q.iter().find(|j| j.0 == n).unwrap().1;
        (get("X"), get("Y"))
    };
    h.gui.camsim_command("eye=shop;follow=0;at=0.3").unwrap();
    shot(&mut h, "native-machine-02-vf2-shop-eye-a");
    let (a, ta) = (target(&h), h.gui.shell.camsim.as_ref().unwrap().t);
    h.gui.camsim_command("at=0.62").unwrap();
    shot(&mut h, "native-machine-03-vf2-shop-eye-b");
    let (b, tb) = (target(&h), h.gui.shell.camsim.as_ref().unwrap().t);
    let ((xa, ya), (xb, yb)) = (joints(&h, ta), joints(&h, tb));
    // the table moves −x for +X: in the part's frame a still eye moves +x by the same
    let (dx, dy) = ((b[0] - a[0]) as f64, (b[1] - a[1]) as f64);
    assert!(
        (dx - (xb - xa)).abs() < 1e-3 && (dy - (yb - ya)).abs() < 1e-3,
        "eye moved ({dx}, {dy}), joints ({}, {})",
        xb - xa,
        yb - ya
    );
    assert!((xb - xa).abs() + (yb - ya).abs() > 1.0, "the table moved");
}

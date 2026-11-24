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
        .replace("at: (0, 0, -250)", "at: (-381, -203, -400)")
        .replace("post: \"fanuc\"", "post: \"haas\"");
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
    shot(&mut h, "native-machine-01-vf2-flange-1400x860");
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

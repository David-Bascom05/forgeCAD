//! Environments (shell/env, research/ui/environments.md), headless: entered from the
//! Environments tab, the ribbon's tabs replaced and given back by Finish; each document keeps the
//! one it is in; asked by name (a deep link, `--env`, the agent) once the part has built; the bend
//! simulation drawn in the view, its place kept across a Finish.
//!
//! `ENV_SHOTS=DIR` writes the frames (PPM) for research/ui/environments/.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::env::{BENDSIM_TAB, ENV_TAB, EnvId, RENDER_TAB};
use forge_render::shell::{FLAT_TAB, SHEET_TAB, THERMO_TAB};
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-env-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

const SHEET: &str = "shared t = 2\nshared r = 2\nshared k = ratio(0.4)\n\
sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (120, 60))\n}\n\
base = sheet(plan, t: t, r: r, bend: kfactor(k))\n\
up = flange(base.edge(plan.outline[1]), height: 30)\n\
down = flange(base.edge(plan.outline[3]), height: 20)\n";

const PLATE: &str = "shared t = 6\nsketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (60, 40))\n}\n\
plate = extrude(plan, t)\n";

fn settled(g: &Gui) -> bool {
    g.view.is_some() && g.fg.as_ref().is_some_and(|s| s.done == s.seq) && !g.loading()
}

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

fn shot(h: &mut Headless, name: &str) {
    let Ok(dir) = std::env::var("ENV_SHOTS") else {
        return;
    };
    for _ in 0..20 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    std::fs::write(format!("{dir}/{name}.ppm"), h.ppm()).unwrap();
}

#[test]
fn thermoform_from_the_environments_tab_and_back() {
    let Some(mut h) = headless() else { return };
    let cover = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/thermoform/cover.fg");
    open(&mut h, cover);
    // the design: its tabs, then Environments; no Thermoform tab among them
    let tabs = h.gui.shell.tab_order();
    assert!(tabs.contains(&ENV_TAB) && !tabs.contains(&THERMO_TAB), "{tabs:?}");
    assert_eq!(h.gui.shell.env, None);
    click(&mut h, "tab:Environments");
    assert_eq!(h.gui.shell.tab, ENV_TAB);
    // a tile per environment; those not on main yet off
    for id in EnvId::ALL {
        assert!(
            h.gui.shell.place(&format!("tool:{}", id.name())).is_some(),
            "{id:?}"
        );
    }
    assert!(id_off(&h, EnvId::Manufacture) && id_off(&h, EnvId::Simulation));
    shot(&mut h, "01-environments-tab");
    click(&mut h, "tool:Thermoform");
    assert_eq!(h.gui.shell.env, Some(EnvId::Thermoform));
    assert_eq!(h.gui.shell.tab, THERMO_TAB);
    assert_eq!(h.gui.shell.tab_order(), vec![THERMO_TAB, 2, 3, 4]);
    assert!(h.gui.shell.place("tab:Thermoform").is_some());
    assert!(
        h.gui.shell.place("tab:3D Model").is_none(),
        "the design's tabs are hidden"
    );
    shot(&mut h, "02-thermoform");
    // its tools work in it: View keeps its own, and back
    click(&mut h, "tab:View");
    assert_eq!(h.gui.shell.env, Some(EnvId::Thermoform));
    click(&mut h, "tab:Thermoform");
    click(&mut h, "tool:Finish Thermoform");
    assert_eq!(h.gui.shell.env, None);
    assert_eq!(h.gui.shell.tab, 0);
    assert!(h.gui.shell.place("tab:3D Model").is_some());
}

fn id_off(h: &Headless, id: EnvId) -> bool {
    id.available(&h.gui.shell).is_err()
}

#[test]
fn each_document_keeps_the_environment_it_was_left_in() {
    let Some(mut h) = headless() else { return };
    let d = scratch("docs");
    let (a, b) = (d.join("a.fg"), d.join("b.fg"));
    std::fs::write(&a, PLATE).unwrap();
    std::fs::write(&b, PLATE).unwrap();
    open(&mut h, a.to_str().unwrap());
    // looking straight down first, so the framing on entry is seen to happen
    {
        let v = h.gui.view.as_mut().unwrap();
        v.nav.camera.yaw = 0.0;
        v.nav.camera.pitch = 89f32.to_radians();
    }
    h.gui
        .shell
        .out
        .push(forge_render::shell::Action::EnterEnv(EnvId::RenderStudio));
    h.step();
    h.step();
    assert_eq!(h.gui.shell.env, Some(EnvId::RenderStudio));
    assert_eq!(h.gui.shell.tab, RENDER_TAB);
    // entered from the 3/4 view (docs/ux/STANDARD.md §2): yaw -45°, pitch 35.26°
    for _ in 0..60 {
        h.step();
    }
    let cam = h.gui.view.as_ref().unwrap().nav.camera;
    assert!(
        (cam.yaw.to_degrees() + 45.0).abs() < 0.5,
        "yaw {}",
        cam.yaw.to_degrees()
    );
    assert!(
        (cam.pitch.to_degrees() - 35.26).abs() < 0.5,
        "pitch {}",
        cam.pitch.to_degrees()
    );
    assert!(h.gui.shell.place("tool:Finish Render Studio").is_some());
    shot(&mut h, "03-render-studio");
    open(&mut h, b.to_str().unwrap());
    assert_eq!(h.gui.active_document(), 1);
    assert_eq!(h.gui.shell.env, None, "a new document is in the design");
    assert!(h.gui.shell.place("tab:3D Model").is_some());
    h.gui.activate(0);
    h.step();
    h.step();
    assert_eq!(h.gui.shell.env, Some(EnvId::RenderStudio));
    assert_eq!(h.gui.shell.tab, RENDER_TAB);
    h.gui.activate(1);
    h.step();
    assert_eq!(h.gui.shell.env, None);
    // an environment that is not about the part owns the window: the part's tree goes
    h.gui
        .shell
        .out
        .push(forge_render::shell::Action::EnterEnv(EnvId::MachineBuilder));
    for _ in 0..3 {
        h.step();
    }
    assert_eq!(h.gui.shell.env, Some(EnvId::MachineBuilder));
    assert!(
        h.gui.shell.place("env:machine-builder:browser").is_some(),
        "its browser, not the part's tree"
    );
    assert!(h.gui.shell.place("tool:Finish Machine Builder").is_some());
    shot(&mut h, "05-machine-builder");
    click(&mut h, "tool:Finish Machine Builder");
    for _ in 0..3 {
        h.step();
    }
    assert_eq!(h.gui.shell.env, None);
    assert!(h.gui.shell.machine.is_none());
}

#[test]
fn asked_by_name_once_built_and_by_the_agent() {
    let Some(mut h) = headless() else { return };
    let d = scratch("named");
    let f = d.join("sheet.fg");
    std::fs::write(&f, SHEET).unwrap();
    // as `--env` / `?env=` ask it: before the part has built, entered once it has
    h.gui.open(f.to_str().unwrap());
    h.gui.env_asked = Some("flat-pattern".into());
    assert!(h.until(|g| g.shell.env.is_some() || g.env_asked.is_none() && settled(g)));
    for _ in 0..4 {
        h.step();
    }
    assert_eq!(
        h.gui.shell.env,
        Some(EnvId::FlatPattern),
        "{}",
        h.gui.shell.status
    );
    assert_eq!(h.gui.shell.tab, FLAT_TAB);
    // the flat's own way out leaves the environment
    click(&mut h, "tool:Go to Folded Part");
    assert_eq!(h.gui.shell.env, None);
    assert_eq!(h.gui.shell.tab, SHEET_TAB);
    // the agent's tool: what there is, then in and out
    let r = h
        .gui
        .app_tool("environment", &forge_agent::json::Json::obj(vec![]));
    assert!(!r.error && r.text.contains("\"active\":null"), "{}", r.text);
    assert!(
        r.text
            .contains("\"key\":\"manufacture\",\"name\":\"Manufacture\",\"available\":false")
    );
    let arg = |n: &str| forge_agent::json::Json::obj(vec![("name", n.into())]);
    let r = h.gui.app_tool("environment", &arg("Render Studio"));
    assert!(!r.error && r.text.contains("\"active\":\"render\""), "{}", r.text);
    let r = h.gui.app_tool("environment", &arg("simulation"));
    assert!(r.error && r.text.contains("solver"), "{}", r.text);
    assert_eq!(
        h.gui.shell.env,
        Some(EnvId::RenderStudio),
        "a refusal leaves it where it was"
    );
    let r = h.gui.app_tool("environment", &arg("exit"));
    assert!(!r.error && h.gui.shell.env.is_none(), "{}", r.text);
    let r = h.gui.app_tool("environment", &arg("warp drive"));
    assert!(r.error && r.text.contains("there are"), "{}", r.text);
}

#[test]
fn bend_simulation_in_the_view_keeps_its_place() {
    let Some(mut h) = headless() else { return };
    let d = scratch("bendsim");
    let f = d.join("sheet.fg");
    std::fs::write(&f, SHEET).unwrap();
    open(&mut h, f.to_str().unwrap());
    // a sheet metal part opens in Sheet Metal; its Bend Simulation is the environment's
    assert_eq!(h.gui.shell.tab, SHEET_TAB);
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    assert_eq!(h.gui.shell.env, Some(EnvId::BendSim), "{}", h.gui.shell.status);
    assert_eq!(h.gui.shell.tab, BENDSIM_TAB);
    let steps = {
        let b = h.gui.shell.sheet.bend_sim.as_ref().expect("simulated");
        b.sim.as_ref().expect("a plan").plan.steps.len()
    };
    assert_eq!(steps, 2);
    // drawn in the view, which it takes; its order in the browser
    assert!(h.gui.shell.place("env:bendsim:view").is_some());
    assert!(h.gui.shell.place("env:bendsim:browser").is_some());
    click(&mut h, "tool:Next Bend");
    assert_eq!(h.gui.shell.sheet.bend_sim.as_ref().unwrap().step, 1);
    shot(&mut h, "04-bend-simulation");
    // a stand-in set, from the machine list (stand-ins are not on the ribbon: STANDARD.md §1)
    h.gui
        .shell
        .sheet
        .bend_sim
        .as_mut()
        .unwrap()
        .redo(Default::default(), "std-us");
    h.step();
    let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
    assert_eq!(b.tooling, "std-us");
    assert_eq!(b.step, 1, "the step kept across a tooling change");
    click(&mut h, "tool:Finish Bend Simulation");
    assert_eq!(h.gui.shell.env, None);
    assert_eq!(h.gui.shell.tab, SHEET_TAB);
    // back in: where it was left, the part unchanged
    click(&mut h, "tab:Environments");
    click(&mut h, "tool:Bend Simulation");
    let b = h.gui.shell.sheet.bend_sim.as_ref().unwrap();
    assert_eq!((b.step, b.tooling.as_str()), (1, "std-us"));
    // Flat Pattern from inside: one environment at a time
    h.gui
        .shell
        .out
        .push(forge_render::shell::Action::EnterEnv(EnvId::FlatPattern));
    h.step();
    h.step();
    assert_eq!(h.gui.shell.env, Some(EnvId::FlatPattern));
    assert_eq!(h.gui.shell.tab, FLAT_TAB);
    click(&mut h, "tool:Bend Simulation");
    assert_eq!(h.gui.shell.env, Some(EnvId::BendSim));
    assert!(!h.gui.shell.sheet.flat_env, "the flat left for the simulation");
}

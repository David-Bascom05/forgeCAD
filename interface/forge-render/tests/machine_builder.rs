//! The Machine Builder workspace (research/cam/machine-builder.md §4): opened from the Manage
//! tab over a part, a machine made from a template, jogged into a self-collision (shown live),
//! checked, saved to the project's folder and seen by the catalog a setup reads; every template
//! shown; closing puts the part back. `FORGE_UI_SHOTS=DIR` keeps the pictures.

use forge_machine_builder::catalog::{Catalog, Level};
use forge_machine_builder::make::Template;
use forge_render::gui::{Gui, Headless};
use forge_render::shell::Action;
use forge_render::shell::machine::Tab;
use std::path::{Path, PathBuf};

const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg
            .as_ref()
            .is_some_and(|s| s.done == s.seq && s.whole >= s.changed)
}

fn shot(h: &mut Headless, name: &str) {
    for _ in 0..12 {
        h.step();
    }
    if let Ok(dir) = std::env::var("FORGE_UI_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        if let Ok(png) = h.png() {
            std::fs::write(Path::new(&dir).join(format!("{name}.png")), png).unwrap();
        }
    }
}

fn project() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fg-machine-builder-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for e in std::fs::read_dir(Path::new(EXAMPLES).join("demo"))
        .unwrap()
        .flatten()
    {
        if e.path().is_file() {
            std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
        }
    }
    dir.join("part_tools.fg")
}

#[test]
fn make_jog_check_save_and_close() {
    let mut h = match Headless::new((1600.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    let part = project();
    let dir = part.parent().unwrap().to_path_buf();
    h.gui.open(part.to_str().unwrap());
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 60.0 && !h.until(ready) {}
    for _ in 0..4 {
        h.step();
    }
    let part_tris = h.gui.view.as_ref().unwrap().scene.placements.len();

    // open from the Manage tab's button
    assert!(h.click("tab:Manage"), "no Manage tab");
    h.step();
    assert!(h.click("tool:Machine Builder"), "no Machine Builder button");
    h.step();
    assert!(h.gui.shell.machine.is_some(), "the workspace did not open");
    shot(&mut h, "01-library");
    // the library's search narrows the list; the Manage tab carries the builder's commands
    h.gui.shell.machine.as_mut().unwrap().filter = "trunnion".into();
    for _ in 0..3 {
        h.step();
    }
    assert!(h.gui.shell.place("machine:pick:generic-trunnion-bc").is_some());
    assert!(h.gui.shell.place("machine:pick:generic-vmc3").is_none());
    shot(&mut h, "01b-search");
    h.gui.shell.machine.as_mut().unwrap().filter.clear();
    h.step();
    assert!(
        h.gui.shell.place("tool:Check Travel").is_some(),
        "the builder's ribbon group"
    );
    {
        let m = h.gui.shell.machine.as_ref().unwrap();
        assert_eq!(m.project.as_deref(), Some(dir.as_path()));
        assert!(m.def.is_some(), "the first machine is in the editor");
        let n = m.def.as_ref().unwrap().bodies.len();
        assert_eq!(
            h.gui.view.as_ref().unwrap().scene.placements.len(),
            n,
            "one placement per body"
        );
    }

    // a 5-axis trunnion from its template, by the panel's button
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        m.new_name = "shop-trunnion".into();
        m.template = Template::Trunnion;
    }
    assert!(h.click("machine:from-template"));
    h.step();
    {
        let m = h.gui.shell.machine.as_ref().unwrap();
        let d = m.def.as_ref().unwrap();
        assert_eq!(d.name, "shop-trunnion");
        assert!(m.problem.is_none(), "{:?}", m.problem);
        assert!(m.changed);
        assert!(m.live.is_empty(), "contact at home: {:?}", m.live);
    }
    h.gui.shell.machine.as_mut().unwrap().tab = Tab::Joints;
    shot(&mut h, "02-trunnion-template");

    // jog into the head-cradle contact `machine check` finds for this layout
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        let d = m.def.clone().unwrap();
        for (n, v) in [("X", -153.586), ("Y", -300.0), ("Z", -350.0), ("B", -120.0)] {
            m.q[d.joint_index(n).unwrap()] = v;
        }
        m.jogged();
        assert!(
            m.live
                .iter()
                .any(|(a, b, hit)| *hit && [a.as_str(), b.as_str()].contains(&"head")),
            "{:?}",
            m.live
        );
    }
    shot(&mut h, "03-jogged-into-collision");
    {
        let m = h.gui.shell.machine.as_ref().unwrap();
        let x = m.def.as_ref().unwrap().joint_index("X").unwrap();
        assert_eq!(
            m.q[x], -153.586,
            "the jog bar changed a joint value it was only showing"
        );
    }

    // an edit makes a copied value the user's; the check reports it
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        let d = m.def.as_mut().unwrap();
        let z = d.joint_index("Z").unwrap();
        d.joints[z].range = [-250.0, 0.0];
        d.cites.retain(|c| c.path != "joint.Z.range");
        m.changed = true;
        m.edited(false);
    }
    assert!(h.click("machine:check"));
    h.step();
    {
        let m = h.gui.shell.machine.as_ref().unwrap();
        assert_eq!(m.tab, Tab::Check);
        let r = m.report.as_ref().unwrap();
        let z = r.values.iter().find(|v| v.path == "joint.Z.range").unwrap();
        assert_eq!(z.status, vec!["user".to_string()]);
        let x = r.values.iter().find(|v| v.path == "joint.X.range").unwrap();
        assert_eq!(x.status, vec!["unknown".to_string()]);
    }
    shot(&mut h, "04-check");

    // saved to the project: the catalog a setup reads sees it at once
    h.gui.shell.machine.as_mut().unwrap().level = Level::Project;
    assert!(h.click("machine:save"));
    h.step();
    let f = dir.join("cam/machines/shop-trunnion.toml");
    assert!(f.is_file(), "{}", h.gui.shell.machine.as_ref().unwrap().message);
    let cat = Catalog::load(Some(&dir), None);
    let e = cat.get("shop-trunnion").unwrap();
    assert_eq!(e.level, Level::Project);
    assert_eq!(e.def.joint("Z").unwrap().range, [-250.0, 0.0]);
    assert!(dir.join("cam/machines/parts/shop-trunnion/cradle.fg").is_file());
    shot(&mut h, "05-saved");

    // a body's geometry from files beside the saved machine: a STEP file (wrapped in a .fg that
    // imports it) and the built-in .fg part, each with its envelope fitted to its bounds
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        m.tab = Tab::Bodies;
        let d = m.def.as_ref().unwrap();
        m.pick = d.bodies.iter().position(|b| b.name == "support-l").unwrap();
    }
    let step = Path::new(EXAMPLES).join("rack18u/asm/order/RACK-312_strike.step");
    let dir2 = dir.join("cam/machines");
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        let mut d = m.def.clone().unwrap();
        let b = m.pick;
        let msg = forge_render::shell::machine::attach_and_fit(&mut d, b, &dir2, &step).unwrap();
        assert!(msg.contains("STEP"), "{msg}");
        let part = d.bodies[b].part.clone().unwrap();
        assert!(dir2.join(&part).is_file());
        assert!(matches!(
            d.bodies[b].prims[..],
            [forge_machine::def::Prim::Box { .. }]
        ));
        assert!(
            !d.cites.iter().any(|c| c.path == "body.support-l"),
            "the fitted envelope is the user's"
        );
        m.def = Some(d);
        m.changed = true;
        m.edited(true);
        assert!(m.problem.is_none(), "{:?}", m.problem);
    }
    shot(&mut h, "06-body-from-step");

    // every template, as the builder shows it
    for (k, tp) in Template::ALL.into_iter().enumerate() {
        {
            let m = h.gui.shell.machine.as_mut().unwrap();
            m.new_name = format!("t-{}", tp.word());
            m.template = tp;
            m.make_template();
            assert!(m.problem.is_none(), "{}: {:?}", tp.word(), m.problem);
            m.tab = Tab::Machine;
        }
        h.step();
        shot(&mut h, &format!("1{k}-template-{}", tp.word()));
    }

    // the tooling libraries in the same workspace: a cutting tool derived from a built-in one,
    // its length edited (the user's value), checked, saved to the project; a holder shown
    h.gui.shell.machine.as_mut().unwrap().set_lib("tool");
    for _ in 0..3 {
        h.step();
    }
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        let tu = m.tooling.as_mut().unwrap();
        let e = tu
            .entries
            .iter()
            .find(|e| e.def.id == "flat-10")
            .cloned()
            .unwrap();
        tu.pick(e);
    }
    shot(&mut h, "20-tool-library");
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        let project = m.project.clone();
        let tu = m.tooling.as_mut().unwrap();
        tu.new_id = "shop-flat-10".into();
        tu.derive(project.as_deref());
        assert_eq!(tu.cur.as_ref().unwrap().id, "shop-flat-10", "{}", tu.message);
        forge_tooling_builder::dynamic::set_user(&mut tu.table, "oal", forge_tooling::toml::Val::Num(90.0));
        tu.reread();
        assert!(tu.problem.is_none(), "{:?}", tu.problem);
        assert_eq!(
            forge_tooling_builder::dynamic::status_of(&tu.table, "oal").as_deref(),
            Some("user")
        );
        tu.check(project.as_deref());
        assert!(tu.findings.is_empty(), "{:?}", tu.findings);
        tu.save(Level::Project, project.as_deref());
        assert!(tu.message.starts_with("Saved"), "{}", tu.message);
    }
    assert!(dir.join("cam/tools/shop-flat-10.toml").is_file());
    let (lib, _) = forge_tooling_builder::library(Some(&dir), None);
    assert_eq!(
        lib.tools.iter().find(|t| t.id == "shop-flat-10").unwrap().oal,
        90.0
    );
    shot(&mut h, "21-tool-derived");
    h.gui.shell.machine.as_mut().unwrap().set_lib("holder");
    for _ in 0..3 {
        h.step();
    }
    assert!(
        h.gui
            .shell
            .machine
            .as_ref()
            .unwrap()
            .tooling
            .as_ref()
            .unwrap()
            .preview()
            .is_some()
    );
    shot(&mut h, "22-holder");
    // workholding: a vise derived from a built-in one, its jaw width the user's, checked, saved
    h.gui.shell.machine.as_mut().unwrap().set_lib("workholding");
    for _ in 0..3 {
        h.step();
    }
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        let w = m.workholding.as_mut().unwrap();
        let e = w
            .entries
            .iter()
            .find(|e| e.def.kind.name() == "vise" && !e.def.standin)
            .cloned()
            .expect("a built-in vise");
        w.pick(e);
        assert!(w.preview().is_some_and(|b| !b.is_empty()), "the vise previews");
    }
    shot(&mut h, "30-workholding-vise");
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        let project = m.project.clone();
        let w = m.workholding.as_mut().unwrap();
        w.new_id = "shop-vise".into();
        w.derive();
        assert_eq!(w.cur.as_ref().unwrap().name, "shop-vise", "{}", w.message);
        let k = w.cur.as_ref().unwrap().vals.keys().next().cloned().unwrap();
        let v = w.cur.as_ref().unwrap().vals[&k] + 1.0;
        w.set(&k, v);
        assert_eq!(w.cur.as_ref().unwrap().vals[&k], v, "{}", w.message);
        w.check();
        w.save(Level::Project, project.as_deref());
        assert!(w.message.starts_with("Saved"), "{}", w.message);
    }
    assert!(dir.join("cam/workholding/shop-vise.toml").is_file());
    // a list (parallels' heights) and a word edited as the user's
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        let w = m.workholding.as_mut().unwrap();
        if let Some(e) = w
            .entries
            .iter()
            .find(|e| e.def.kind.name() == "parallels")
            .cloned()
        {
            w.pick(e);
            w.set_list("heights", vec![10.0, 20.0, 30.0]);
            assert_eq!(
                w.cur.as_ref().unwrap().lists["heights"],
                vec![10.0, 20.0, 30.0],
                "{}",
                w.message
            );
        }
        let vise = w
            .entries
            .iter()
            .find(|e| e.def.name == "shop-vise")
            .cloned()
            .unwrap();
        w.pick(vise);
        if let Some(k) = w.cur.as_ref().unwrap().words.keys().next().cloned() {
            w.set_word(&k, "user-word");
            assert_eq!(w.cur.as_ref().unwrap().words[&k], "user-word", "{}", w.message);
        }
    }
    shot(&mut h, "31-workholding-derived");
    h.gui.shell.machine.as_mut().unwrap().set_lib("machine");
    h.step();

    // close: the part is back
    h.gui.shell.out.push(Action::MachineBuilder);
    for _ in 0..3 {
        h.step();
    }
    assert!(h.gui.shell.machine.is_none());
    assert_eq!(h.gui.view.as_ref().unwrap().scene.placements.len(), part_tris);
    let _ = std::fs::remove_dir_all(&dir);
}

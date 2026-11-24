//! The library builder's press-brake tabs (research/cam/library-builder.md): punches, dies and
//! presses from forge-brake, listed by level; a die from the template, a value edited (the
//! user's) and one made unknown (shown as such, the check failing on it), filled in again,
//! saved to the project's bend/tooling and seen by the catalog; a press's unpublished values
//! reported not checked; each kind's sections drawn. `FORGE_UI_SHOTS=DIR` keeps the pictures.

use forge_brake_builder::{Dies, Rec};
use forge_library::{Catalog, Level};
use forge_render::gui::{Gui, Headless};
use forge_render::shell::machine_brake::{get, status_of};
use forge_vendor::toml::Item;
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
    let dir = std::env::temp_dir().join(format!("fg-brake-builder-{}", std::process::id()));
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
fn punches_dies_presses() {
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
    assert!(h.click("tab:Manage"), "no Manage tab");
    h.step();
    assert!(h.click("tool:Machine Builder"), "no Machine Builder button");
    h.step();

    // each kind: its built-in records listed, the first drawn
    for (w, _) in forge_brake_builder::KINDS {
        let m = h.gui.shell.machine.as_mut().unwrap();
        m.set_lib(w);
        let b = m.brake.as_ref().unwrap();
        assert!(
            b.entries.iter().filter(|e| e.level == Level::Builtin).count() >= 20,
            "{w}: {} records",
            b.entries.len()
        );
        assert!(b.errors.is_empty(), "{w}: {:?}", b.errors);
        assert!(b.problem.is_none(), "{w}: {:?}", b.problem);
        // one with its working geometry published (the first may hold an unknown one: no section)
        let e = b
            .entries
            .iter()
            .find(|e| forge_brake_builder::preview(&e.def).is_ok())
            .cloned()
            .unwrap_or_else(|| panic!("{w}: no record with a section"));
        let m = h.gui.shell.machine.as_mut().unwrap();
        let b = m.brake.as_mut().unwrap();
        // the list narrowed to the record's maker: its group open, with thumbnails
        b.filter = forge_render::shell::machine_brake::get(&e.def.table, "maker")
            .and_then(|m| match m {
                Item::Str(s) => s.split_whitespace().next().map(str::to_string),
                _ => None,
            })
            .unwrap_or_default();
        b.pick(e);
        shot(&mut h, &format!("brake-0-{w}"));
        h.gui
            .shell
            .machine
            .as_mut()
            .unwrap()
            .brake
            .as_mut()
            .unwrap()
            .filter
            .clear();
        let m = h.gui.shell.machine.as_ref().unwrap();
        // solids from the record's dimensions, and what it does not give as labelled ghosts
        let p = m.library_preview().unwrap();
        assert!(
            p.parts.iter().all(|(t, _)| !t.tri.is_empty()),
            "{w}: an empty solid"
        );
        assert!(!p.parts.is_empty() || !p.ghosts.is_empty(), "{w}: nothing drawn");
        assert!(
            // labels in words, not record keys (STANDARD §4.2, §5)
            p.ghosts.iter().all(|g| !g.label.contains('_')
                && !g.label.contains("frame.")
                && !g.label.contains("ram.")
                && !g.label.contains("bed.")),
            "{w}: {:?}",
            p.ghosts.iter().map(|g| &g.label).collect::<Vec<_>>()
        );
        assert!(!p.parts.is_empty(), "{w}: drawn solid, not as wire boxes");
        if *w != "press" {
            assert!(p.section.is_some(), "{w}: its section inset");
        }
    }

    // units as forge-brake keeps them: a press's force in kN, its speeds in mm/s, a tool's load
    // in kN/m (never shown as N or m/min)
    let unit = |k: &str, p: &str| {
        forge_brake_builder::fields(k)
            .into_iter()
            .find(|f| f.path == p)
            .unwrap()
            .unit
            .suffix()
    };
    assert_eq!(unit("press", "force"), "kN");
    assert_eq!(unit("press", "speeds.work"), "mm/s");
    assert_eq!(unit("die", "max_load"), "kN/m");
    assert_eq!(unit("press", "bed_length"), "mm");
    // an inch window shows the lengths in inches
    h.gui.shell.units = "in";
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        m.set_lib("press");
    }
    shot(&mut h, "brake-0b-press-inch");
    h.gui.shell.units = "mm";

    // a press: its unpublished values are "not checked", not failures
    {
        let m = h.gui.shell.machine.as_mut().unwrap();
        m.set_lib("press");
        let b = m.brake.as_mut().unwrap();
        let e = b
            .entries
            .iter()
            .find(|e| forge_brake_builder::check(&e.def).iter().any(|f| f.not_checked))
            .cloned()
            .expect("a press with an unpublished value");
        b.pick(e);
        b.check();
        assert!(b.findings.iter().any(|f| f.not_checked), "{:?}", b.findings);
        assert!(b.findings.iter().all(|f| f.not_checked), "{:?}", b.findings);
    }
    shot(&mut h, "brake-1-press-check");

    // a die from the template, by the panel's buttons
    h.gui.shell.machine.as_mut().unwrap().set_lib("die");
    h.step();
    h.gui
        .shell
        .machine
        .as_mut()
        .unwrap()
        .brake
        .as_mut()
        .unwrap()
        .new_id = "shop-v16".into();
    assert!(h.click("brake:template"));
    h.step();
    {
        let b = h.gui.shell.machine.as_mut().unwrap().brake.as_mut().unwrap();
        assert!(b.problem.is_none(), "{:?} {}", b.problem, b.message);
        assert!(b.origin.is_none() && b.changed);
        assert_eq!(b.cur.as_ref().unwrap().id, "shop-v16");
        // the user's own V opening
        let f = forge_brake_builder::fields("die")
            .into_iter()
            .find(|f| f.path == "v")
            .unwrap();
        forge_render::shell::machine_brake::set_user(&mut b.table, &f, Item::Num(16.0));
        b.reread();
        assert!(b.problem.is_none(), "{:?}", b.problem);
        assert_eq!(status_of(&b.table, "v").as_deref(), Some("user"));
        // the shoulder radius not known: gone, cited unknown, the check fails on it
        forge_render::shell::machine_brake::set_unknown(&mut b.table, "shoulder_radius");
        b.reread();
        assert!(b.problem.is_none(), "{:?}", b.problem);
        assert!(get(&b.table, "shoulder_radius").is_none());
        assert_eq!(status_of(&b.table, "shoulder_radius").as_deref(), Some("unknown"));
        b.check();
        assert!(
            b.findings
                .iter()
                .any(|f| f.path == "shoulder_radius" && !f.not_checked),
            "{:?}",
            b.findings
        );
        assert!(
            b.preview().is_none(),
            "a die with unknown working geometry has no exact section"
        );
        let (ghost, stood) = b.preview_ghost().expect("drawn as a ghost from stand-ins");
        assert!(!ghost.is_empty());
        assert_eq!(stood, ["shoulder_radius"]);
    }
    shot(&mut h, "brake-2-die-unknown");
    {
        let b = h.gui.shell.machine.as_mut().unwrap().brake.as_mut().unwrap();
        let f = forge_brake_builder::fields("die")
            .into_iter()
            .find(|f| f.path == "shoulder_radius")
            .unwrap();
        forge_render::shell::machine_brake::set_user(&mut b.table, &f, Item::Num(2.5));
        b.reread();
        b.check();
        assert!(b.findings.is_empty(), "{:?}", b.findings);
        assert!(b.preview().is_some_and(|v| !v.is_empty()));
    }
    shot(&mut h, "brake-3-die");

    // saved to the project's bend/tooling: the catalog sees it at once
    h.gui.shell.machine.as_mut().unwrap().level = Level::Project;
    assert!(h.click("brake:save"));
    h.step();
    let f = dir.join("bend/tooling/shop-v16.toml");
    assert!(
        f.is_file(),
        "{}",
        h.gui
            .shell
            .machine
            .as_ref()
            .unwrap()
            .brake
            .as_ref()
            .unwrap()
            .message
    );
    let cat = Catalog::<Dies>::load(Some(&dir), None);
    let e = cat.get("shop-v16").unwrap();
    assert_eq!(e.level, Level::Project);
    let r: &Rec = &e.def;
    assert_eq!(get(&r.table, "v"), Some(&Item::Num(16.0)));
    // the punches' catalog reads that file as no punches, not as a broken file
    let p = Catalog::<forge_brake_builder::Punches>::load(Some(&dir), None);
    assert!(p.errors.is_empty(), "{:?}", p.errors);
    {
        let b = h.gui.shell.machine.as_ref().unwrap().brake.as_ref().unwrap();
        assert!(
            b.entries
                .iter()
                .any(|e| e.def.id == "shop-v16" && e.level == Level::Project)
        );
        assert!(!b.changed);
    }
    shot(&mut h, "brake-4-saved");
    let _ = std::fs::remove_dir_all(&dir);
}

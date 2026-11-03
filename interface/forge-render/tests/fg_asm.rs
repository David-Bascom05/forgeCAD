//! Assemblies (.fg) in the app: the posed parts drawn (each part meshed once, placed at every
//! occurrence), the instance tree, choosing, hiding and isolating instances, a part's anchors,
//! a drive moved by its slider (a SetParam), the typed results shown, the overlap check on
//! request, expects, DOF and the BOM; and the 18U rack opened.

use forge_lang::error::Op;
use forge_render::gui::{Gui, Headless};
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-asm-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A bar `len` long, anchors `a` at its start and `b` at its end, z up.
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

fn base(d: f64) -> String {
    format!(
        "shared d = {d}
shared w = 20
shared t = 3
sketch plan(on: xy) {{
  outline = rect(corner: (-w, -w - 300), size: (d + w + w, w + w))
  g0 = pt(0, 0)
  g1 = pt(d, 0)
}}
plate = extrude(plan, t)
o2 = anchor(plane(at: plan.g0, normal: plan.plane.normal, x: plan.outline[0].dir))
o4 = anchor(plane(at: plan.g1, normal: plan.plane.normal, x: plan.outline[0].dir))
"
    )
}

/// The four-bar (crank 40, coupler 120, rocker 80, ground 100) driven by `j1.turn`, with three
/// spare rods on the ground (one part, three occurrences) and an expect.
const FOUR_BAR: &str = "assembly

param j1.turn = 30deg
shared y0 = 100
shared gap = 30

base = part(\"base.fg\", ground: true)
crank = part(\"crank.fg\", at: pose(rz: 30deg))
coupler = part(\"coupler.fg\", at: pose(at: (35, 20, 0), rz: 10deg))
rocker = part(\"rocker.fg\", at: pose(at: (100, 0, 0), rz: 70deg))
j1 = revolute(crank.a, base.o2, turn: turn)
j2 = revolute(coupler.a, crank.b)
j3 = revolute(rocker.b, coupler.b)
j4 = revolute(rocker.a, base.o4)
for i in 0..3 {
  rod = part(\"rod.fg\", ground: true, at: pose(at: (0, 0 - y0 - gap * i, 3)))
}
expect mobility = 0
";

fn fixtures(d: &Path) {
    for (f, t) in [
        ("base.fg", base(100.0)),
        ("crank.fg", link(40.0)),
        ("coupler.fg", link(120.0)),
        ("rocker.fg", link(80.0)),
        ("rod.fg", link(60.0)),
    ] {
        std::fs::write(d.join(f), t).unwrap();
    }
}

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg_asm.is_some()
        && g.fg.as_ref().is_some_and(|s| s.done == s.seq)
        && g.shell
            .fg
            .as_ref()
            .is_some_and(|p| p.asm.as_ref().is_some_and(|a| !a.checking))
}

fn open(file: &Path) -> Option<Headless> {
    open_within(file, 5.0)
}

/// Opens it and waits up to `secs` for its first solve on screen.
fn open_within(file: &Path, secs: f64) -> Option<Headless> {
    let mut h = match Headless::new((1300.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    let t = std::time::Instant::now();
    let mut got = false;
    while t.elapsed().as_secs_f64() < secs {
        if h.until(ready) {
            got = true;
            break;
        }
    }
    assert!(
        got,
        "never ready: {}\n{:?}",
        h.gui.shell.status,
        h.gui.shell.fg.as_ref().map(|p| p
            .issues
            .iter()
            .map(|i| format!("{} {}", i.kind, i.text))
            .collect::<Vec<_>>())
    );
    for _ in 0..3 {
        h.step();
    }
    Some(h)
}

/// A browser row's right-click menu, and one of its items.
fn menu(h: &mut Headless, row: &str, item: &str) {
    let r = h
        .gui
        .shell
        .place(&format!("asm:row:{row}"))
        .unwrap_or_else(|| panic!("no row {row}"));
    h.move_to(r.center().x, r.center().y);
    h.step();
    h.right_click_here();
    h.step();
    assert!(h.click(&format!("asm:menu:{item}")), "no {item} in {row}'s menu");
    h.step();
    h.step();
}

/// The Drive dialog for the top document's `k`th relationship, from its row's menu.
fn drive(h: &mut Headless, k: usize) {
    assert!(h.click("asm:open:r:"));
    h.step();
    h.step();
    let kind = h
        .gui
        .shell
        .places
        .iter()
        .find(|p| p.0.starts_with("asm:open:r::"))
        .map(|p| p.0.clone())
        .unwrap();
    assert!(h.click(&kind));
    h.step();
    h.step();
    menu(h, &format!("l:{k}"), "drive");
}

fn command(h: &mut Headless, c: forge_render::shell::FgCmd) {
    h.gui.shell.fg_command(c);
    h.step();
    h.step();
}

fn asm(h: &Headless) -> &forge_render::shell::fgasm::AsmPanel {
    h.gui.shell.fg.as_ref().unwrap().asm.as_ref().unwrap()
}

fn placement_of(h: &Headless, path: &str) -> usize {
    let (occ, _) = h.gui.fg_asm.as_ref().unwrap();
    occ.iter().find(|o| o.path == path).unwrap().placement.unwrap()
}

#[test]
fn a_four_bar_opens_posed_with_its_tree_joints_dof_bom_and_expects() {
    let d = scratch("open");
    fixtures(&d);
    let file = d.join("mech.fg");
    std::fs::write(&file, FOUR_BAR).unwrap();
    let Some(mut h) = open(&file) else { return };
    let a = asm(&h);
    let paths: Vec<&str> = a.occ.iter().map(|o| o.path.as_str()).collect();
    for p in ["base", "crank", "coupler", "rocker", "rod[0]", "rod[2]"] {
        assert!(paths.contains(&p), "{p} not in {paths:?}");
    }
    assert_eq!(a.joints.len(), 4);
    // driven: the crank's turn is given, so nothing is left free
    assert_eq!(a.mobility, 0);
    assert_eq!(
        a.bom.iter().find(|b| b.0.ends_with("rod.fg")).map(|b| b.1),
        Some(3)
    );
    // the expects wait for the checks, as the overlap check does
    assert!(a.expects.is_empty() && a.overlap.is_none());
    // one mesh per part, placed at every occurrence: 7 placements of 5 meshes
    let v = h.gui.view.as_ref().unwrap();
    assert_eq!(v.scene.placements.len(), 7);
    let mut meshes: Vec<u32> = v.scene.placements.iter().map(|p| p.part).collect();
    meshes.sort();
    meshes.dedup();
    assert_eq!(meshes.len(), 5);
    // the browser: Origin and Relationships first, then the parts as the file makes them, one
    // statement's loop under one row, shut until opened
    assert!(h.gui.shell.place("asm:row:o:").is_some());
    assert!(h.gui.shell.place("asm:row:r:").is_some());
    assert!(h.gui.shell.place("asm:row:p:rod").is_some());
    assert!(h.gui.shell.place("asm:row:rod[1]").is_none());
    assert!(h.click("asm:open:p:rod"));
    h.step();
    h.step();
    assert!(h.gui.shell.place("asm:row:rod[1]").is_some());
    // no side panel: drives in their dialog, the BOM in its own
    assert!(h.gui.shell.place("asm:drive:j1:turn").is_none());
    drive(&mut h, 0);
    assert!(h.gui.shell.place("asm:drive:j1:turn").is_some());
    command(&mut h, forge_render::shell::FgCmd::Bom);
    assert!(h.gui.shell.place("asm:bom:rod").is_some());
    // run on request, from Analyze Interference: the expect measured and passed
    command(&mut h, forge_render::shell::FgCmd::Interference);
    assert!(h.click("asm:check"));
    h.step();
    assert!(h.until(ready));
    let a = asm(&h);
    assert_eq!(a.expects.len(), 1);
    assert!(a.expects[0].ok, "{:?}", a.expects);
    assert!(a.overlap.is_some());
}

#[test]
fn choosing_hiding_isolating_and_a_parts_anchors() {
    let d = scratch("choose");
    fixtures(&d);
    let file = d.join("mech.fg");
    std::fs::write(&file, FOUR_BAR).unwrap();
    let Some(mut h) = open(&file) else { return };
    let crank = placement_of(&h, "crank");
    assert!(h.click("asm:row:crank"));
    h.step();
    assert_eq!(asm(&h).selected.as_deref(), Some("crank"));
    let v = h.gui.view.as_ref().unwrap();
    assert_eq!(v.scene.placements[crank].tint, forge_render::fg::PICKED_TINT);
    assert_ne!(
        v.scene.placements[placement_of(&h, "rocker")].tint,
        forge_render::fg::PICKED_TINT
    );
    // hidden from its menu: out of sight; shown again
    menu(&mut h, "crank", "hide");
    assert_eq!(h.gui.view.as_ref().unwrap().scene.rows_of(crank), [[0.0; 4]; 3]);
    menu(&mut h, "crank", "hide");
    assert_ne!(h.gui.view.as_ref().unwrap().scene.rows_of(crank), [[0.0; 4]; 3]);
    // isolated: only it
    menu(&mut h, "crank", "isolate");
    let v = h.gui.view.as_ref().unwrap();
    assert_ne!(v.scene.rows_of(crank), [[0.0; 4]; 3]);
    assert_eq!(v.scene.rows_of(placement_of(&h, "rocker")), [[0.0; 4]; 3]);
    menu(&mut h, "crank", "isolate");
    // a loop's row: its three rods at once
    assert!(h.click("asm:row:p:rod"));
    h.step();
    let v = h.gui.view.as_ref().unwrap();
    for k in 0..3 {
        assert_eq!(
            v.scene.placements[placement_of(&h, &format!("rod[{k}]"))].tint,
            forge_render::fg::PICKED_TINT
        );
    }
    // a relationship's row: its two parts
    assert!(h.click("asm:open:crank"));
    h.step();
    h.step();
    let j1 = h
        .gui
        .shell
        .places
        .iter()
        .find(|p| p.0.starts_with("asm:row:crank>l:"))
        .map(|p| p.0.clone())
        .unwrap();
    assert!(h.click(&j1));
    h.step();
    let v = h.gui.view.as_ref().unwrap();
    assert_eq!(v.scene.placements[crank].tint, forge_render::fg::PICKED_TINT);
    assert_eq!(
        v.scene.placements[placement_of(&h, "base")].tint,
        forge_render::fg::PICKED_TINT
    );
    // the part file and its anchors, in its iProperties
    menu(&mut h, "crank", "props");
    assert!(h.gui.shell.place("asm:file").is_some());
    assert!(h.click("asm:anchors"));
    assert!(h.until(|g| {
        g.shell
            .fg
            .as_ref()
            .unwrap()
            .asm
            .as_ref()
            .unwrap()
            .anchors
            .is_some()
    }));
    let (file, got) = asm(&h).anchors.clone().unwrap();
    assert!(file.ends_with("crank.fg"), "{file}");
    let got = got.unwrap();
    assert!(got.contains(&"bar.end".to_string()), "{got:?}");
}

#[test]
fn a_drive_slider_is_a_set_param_and_the_crank_turns() {
    let d = scratch("drive");
    fixtures(&d);
    let file = d.join("mech.fg");
    std::fs::write(&file, FOUR_BAR).unwrap();
    let Some(mut h) = open(&file) else { return };
    let crank = placement_of(&h, "crank");
    let before = h.gui.view.as_ref().unwrap().scene.rows_of(crank);
    drive(&mut h, 0);
    // a click on the slider's track sets it there
    let r = h.gui.shell.place("asm:drive:j1:turn").unwrap();
    h.move_to(r.left() + r.width() * 0.3, r.center().y);
    h.step();
    h.button(true);
    h.step();
    h.button(false);
    h.step();
    let ops = h.gui.fg.as_ref().unwrap().ops.clone();
    assert_eq!(ops.len(), 1, "{ops:?}");
    let Op::SetParam { param, value } = &ops[0] else {
        panic!("{ops:?}")
    };
    assert_eq!(param.to_string(), "j1.turn");
    assert_ne!(value.value, 30.0);
    assert!(h.until(ready));
    h.step();
    let after = h.gui.view.as_ref().unwrap().scene.rows_of(crank);
    assert_ne!(before, after, "the crank did not turn");
    // the joint reports the new angle
    let j1 = asm(&h).joints.iter().find(|j| j.id == "j1").unwrap().clone();
    assert!((j1.values[0].1.to_degrees() - value.value).abs() < 1e-6, "{j1:?}");
}

#[test]
fn a_conflict_shows_its_set_and_the_overlap_check_runs_on_request() {
    let d = scratch("diag");
    fixtures(&d);
    let file = d.join("mech.fg");
    std::fs::write(&file, format!("{FOUR_BAR}j5 = revolute(crank.b, base.o4)\n")).unwrap();
    let Some(mut h) = open(&file) else { return };
    // the issues, a click on the status bar away; the sick joints red in the browser
    assert!(h.click("asm:issues"));
    h.step();
    h.step();
    let a = asm(&h);
    let sick: Vec<&str> = a
        .tree
        .nodes
        .iter()
        .filter(|n| n.flag == Some(forge_render::shell::fgasm::Flag::Error))
        .map(|n| n.label.as_str())
        .collect();
    assert!(
        sick.contains(&"Relationships") && sick.contains(&"j5"),
        "{sick:?}"
    );
    let p = h.gui.shell.fg.as_ref().unwrap();
    let k = p
        .issues
        .iter()
        .position(|i| i.kind == "Conflict")
        .unwrap_or_else(|| panic!("{:?}", p.issues));
    assert!(
        p.issues[k]
            .fixes
            .iter()
            .any(|f| matches!(f, Op::DeleteFeature { .. }))
    );
    assert!(h.click(&format!("asm:show:{k}")));
    h.step();
    let items = asm(&h).highlight_items.clone();
    assert!(
        items.len() >= 2 && items.iter().all(|i| i.starts_with('j')),
        "{items:?}"
    );
    // interference: none until asked, then counted
    assert!(asm(&h).overlap.is_none());
    command(&mut h, forge_render::shell::FgCmd::Interference);
    assert!(h.click("asm:check"));
    h.step();
    assert!(asm(&h).checking);
    assert!(h.until(ready));
    let (n, _) = asm(&h).overlap.unwrap();
    // the links sit on one another at their pins: they interfere, each warning with its volume
    let warns: Vec<_> = h
        .gui
        .shell
        .fg
        .as_ref()
        .unwrap()
        .issues
        .iter()
        .filter(|i| i.kind == "Interference")
        .cloned()
        .collect();
    assert_eq!(warns.len(), n);
    if let Some(w) = warns.first() {
        let k = h
            .gui
            .shell
            .fg
            .as_ref()
            .unwrap()
            .issues
            .iter()
            .position(|i| *i == *w)
            .unwrap();
        assert!(h.click(&format!("asm:show:{k}")));
        h.step();
        let hl = asm(&h).highlight.clone();
        assert_eq!(hl.len(), 2);
        let v = h.gui.view.as_ref().unwrap();
        assert_eq!(
            v.scene.placements[placement_of(&h, &hl[0])].tint,
            forge_render::fg::CLASH_TINT
        );
    }
}

/// The 18U rack, as it stands: it opens, every drawn occurrence is a placement of a mesh made
/// once per part, and orbiting draws frames quickly.
#[test]
fn the_18u_rack_opens_and_orbits() {
    let rack = std::path::Path::new(env!("HOME")).join("projects/forgecad-gate/rack18u/asm/rack.fg");
    if !rack.exists() {
        eprintln!("skipped: no {}", rack.display());
        return;
    }
    let t = std::time::Instant::now();
    let Some(h) = open_within(&rack, 600.0) else {
        return;
    };
    let mut h = h;
    let open_ms = t.elapsed().as_secs_f64() * 1e3;
    let a = asm(&h);
    let n = a.occ.len();
    let drawn = a.occ.iter().filter(|o| o.drawn).count();
    let v = h.gui.view.as_ref().unwrap();
    let mut meshes: Vec<u32> = v.scene.placements.iter().map(|p| p.part).collect();
    meshes.sort();
    meshes.dedup();
    eprintln!(
        "rack.fg: {n} occurrences ({drawn} drawn) of {} meshes, open {open_ms:.0} ms; {}; {} issues",
        meshes.len(),
        a.timings,
        h.gui.shell.fg.as_ref().unwrap().issues.len()
    );
    let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
    for i in &h.gui.shell.fg.as_ref().unwrap().issues {
        *kinds.entry(i.kind.clone()).or_default() += 1;
    }
    eprintln!("rack.fg issues by kind: {kinds:?}");
    assert!(n > 100, "{n}");
    assert_eq!(v.scene.placements.len(), drawn.max(1));
    assert!(
        meshes.len() * 4 < drawn,
        "instancing: {} meshes for {drawn}",
        meshes.len()
    );
    // orbit: frames while the view turns
    let mut times = vec![];
    for _ in 0..30 {
        h.gui.turn_view();
        let t = std::time::Instant::now();
        h.step();
        times.push(t.elapsed().as_secs_f64() * 1e3);
    }
    times.sort_by(f64::total_cmp);
    eprintln!(
        "rack.fg orbit: frame median {:.1} ms, worst {:.1} ms",
        times[15], times[29]
    );
    assert!(times[15] < 50.0, "median frame {:.1} ms", times[15]);
}

/// The rack's checks, on request: every expect passes and nothing interferes.
#[test]
fn the_18u_racks_checks_all_pass() {
    let rack = std::path::Path::new(env!("HOME")).join("projects/forgecad-gate/rack18u/asm/rack.fg");
    if !rack.exists() {
        eprintln!("skipped: no {}", rack.display());
        return;
    }
    let Some(mut h) = open_within(&rack, 600.0) else {
        return;
    };
    let t = std::time::Instant::now();
    command(&mut h, forge_render::shell::FgCmd::Interference);
    assert!(h.click("asm:check"));
    h.step();
    assert!(asm(&h).checking, "Run checks was not pressed");
    let mut done = false;
    while t.elapsed().as_secs() < 1800 {
        if h.until(ready) {
            done = true;
            break;
        }
    }
    assert!(done, "the checks never finished");
    let a = asm(&h);
    let failed: Vec<_> = a.expects.iter().filter(|e| !e.ok).collect();
    eprintln!(
        "rack.fg checks: {:.0} s, {} expects ({} failed), overlap {:?}; {}",
        t.elapsed().as_secs_f64(),
        a.expects.len(),
        failed.len(),
        a.overlap,
        a.timings
    );
    assert!(failed.is_empty(), "{failed:?}");
    assert_eq!(a.expects.len(), 537);
    assert_eq!(a.overlap.map(|o| o.0), Some(0));
}

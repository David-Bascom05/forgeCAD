//! The assembly tools in the app: Place (the first grounded), a joint from the Joint dialog
//! previewed before OK and one undo step after, editing a statement in its place, suppressing a
//! component with its relationships, deleting one with them, a relationship's row opening its
//! dialog, Free Move written as a pose.

use forge_render::gui::{Gui, Headless};
use forge_render::shell::asmtools::{AsmTool, LinkForm, Pick};
use forge_render::shell::fg::FgAct;
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-tools-{}-{name}", std::process::id()));
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

fn fixtures(name: &str, asm: &str) -> PathBuf {
    let d = scratch(name);
    std::fs::write(d.join("base.fg"), link(100.0)).unwrap();
    std::fs::write(d.join("arm.fg"), link(40.0)).unwrap();
    let f = d.join("top.fg");
    std::fs::write(&f, asm).unwrap();
    f
}

fn ready(g: &Gui) -> bool {
    g.view.is_some()
        && g.fg.as_ref().is_some_and(|s| s.done == s.seq)
        && g.shell.fg.as_ref().is_some_and(|p| p.asm.is_some())
}

fn open(file: &Path) -> Option<Headless> {
    let mut h = match Headless::new((1300.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file.to_str().unwrap());
    settle(&mut h);
    Some(h)
}

/// Frames till the Forge worker has built what was last sent.
fn settle(h: &mut Headless) {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < 20.0 {
        if h.until(ready) {
            break;
        }
    }
    assert!(ready(&h.gui), "never ready: {}", h.gui.shell.status);
    for _ in 0..3 {
        h.step();
    }
}

fn act(h: &mut Headless, a: FgAct) {
    h.gui.shell.fg.as_mut().unwrap().out.push(a);
    h.step();
    settle(h);
}

fn src(h: &Headless) -> String {
    h.gui.fg.as_ref().unwrap().src.clone()
}

fn asm(h: &Headless) -> &forge_render::shell::fgasm::AsmPanel {
    h.gui.shell.fg.as_ref().unwrap().asm.as_ref().unwrap()
}

fn pick(r: &str) -> Option<Pick> {
    Some(Pick {
        occ: r.split('.').next().unwrap().into(),
        reference: r.into(),
        kind: None,
        world: None,
        local: None,
    })
}

#[test]
fn place_then_a_joint_previewed_before_ok_and_undone_in_one_step() {
    let f = fixtures("place", "assembly\n");
    let Some(mut h) = open(&f) else { return };
    act(
        &mut h,
        FgAct::AsmPlace {
            file: "base.fg".into(),
            like: None,
        },
    );
    act(
        &mut h,
        FgAct::AsmPlace {
            file: "arm.fg".into(),
            like: None,
        },
    );
    let s = src(&h);
    assert!(
        s.contains("base_1 = part(\"base.fg\", at: pose(at: (0, 0, 0)"),
        "{s}"
    );
    assert!(s.contains("ground: true)\narm_1 = part(\"arm.fg\""), "{s}");
    assert!(
        !s.contains("arm_1 = part(\"arm.fg\", at: pose(at: (0, 0, 0)"),
        "beside, not on top: {s}"
    );
    assert_eq!(asm(&h).mobility, 6);
    // the Joint dialog's statement, previewed: the solve moves, the file doesn't
    let mut form = LinkForm::new_joint(&s);
    form.kind = 2;
    form.picks = [pick("arm_1.a"), pick("base_1.b")];
    form.values = vec!["30".into()];
    let ops = form.ops(&s).unwrap();
    act(&mut h, FgAct::AsmPreview(Some(ops.clone())));
    assert!(h.gui.fg.as_ref().unwrap().previewing);
    assert_eq!(src(&h), s, "a preview leaves the file");
    assert_eq!(asm(&h).mobility, 0, "a driven revolute leaves nothing free");
    act(&mut h, FgAct::AsmOps(ops));
    let joined = src(&h);
    assert!(
        joined.ends_with("param joint_1.turn = 30deg\njoint_1 = revolute(arm_1.a, base_1.b, turn: turn)\n"),
        "{joined}"
    );
    assert!(
        h.gui
            .shell
            .fg
            .as_ref()
            .unwrap()
            .issues
            .iter()
            .all(|i| i.severity != forge_render::shell::fg::Severity::Error)
    );
    h.gui.shell.out.push(forge_render::shell::Action::Undo);
    h.step();
    settle(&mut h);
    assert_eq!(src(&h), s, "OK is one undo step");
}

const TWO: &str = "assembly

base = part(\"base.fg\", ground: true)
arm = part(\"arm.fg\", at: pose(at: (0, 30, 0)))
param pivot.turn = 20deg
pivot = revolute(arm.a, base.b, turn: turn)
# after
tip = part(\"arm.fg\", at: pose(at: (0, 60, 0)))
hold = fastened(tip.a, arm.b)
";

#[test]
fn a_relationship_is_edited_in_its_place_and_its_row_opens_the_dialog() {
    let f = fixtures("edit", TWO);
    let Some(mut h) = open(&f) else { return };
    let mut form = LinkForm::edit(TWO, "pivot").unwrap();
    assert_eq!(form.values, vec!["20deg".to_string()]);
    form.kind = 4;
    form.limits = vec![
        (false, String::new(), String::new()),
        (true, "0".into(), "90".into()),
    ];
    form.values = vec![String::new(), "20deg".into()];
    let ops = form.ops(TWO).unwrap();
    act(&mut h, FgAct::AsmOps(ops));
    let s = src(&h);
    assert!(
        s.contains("param pivot.turn = 20deg\nparam pivot.turn_lo = 0deg\nparam pivot.turn_hi = 90deg\npivot = cylindrical(arm.a, base.b, turn: turn, turn_limit: (turn_lo, turn_hi))\n# after"),
        "{s}"
    );
    assert_eq!(asm(&h).mobility, 1, "the cylinder slides");
    // a relationship's menu opens the same dialog, filled
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
    let row = h.gui.shell.place("asm:row:l:0").expect("a relationship row");
    h.move_to(row.center().x, row.center().y);
    h.step();
    h.right_click_here();
    h.step();
    assert!(h.click("asm:menu:edit"));
    h.step();
    match &asm(&h).tool {
        Some(AsmTool::Link(f)) => assert_eq!(f.editing.as_deref(), Some(asm(&h).links[0].id.as_str())),
        t => panic!("no Edit dialog: {t:?}"),
    }
}

#[test]
fn suppress_takes_the_relationships_along_and_delete_takes_them_out() {
    let f = fixtures("suppress", TWO);
    let Some(mut h) = open(&f) else { return };
    let ops = forge_render::shell::asmtools::suppress_ops(asm(&h), "arm", true);
    act(&mut h, FgAct::AsmOps(ops));
    let s = src(&h);
    assert!(
        s.contains("arm = part(\"arm.fg\", at: pose(at: (0, 30, 0)), suppressed: true)"),
        "{s}"
    );
    assert!(
        s.contains("pivot = revolute(arm.a, base.b, turn: turn, suppressed: true)"),
        "{s}"
    );
    assert!(
        s.contains("hold = fastened(tip.a, arm.b, suppressed: true)"),
        "{s}"
    );
    assert!(
        h.gui
            .shell
            .fg
            .as_ref()
            .unwrap()
            .issues
            .iter()
            .all(|i| i.severity != forge_render::shell::fg::Severity::Error)
    );
    let ops = forge_render::shell::asmtools::delete_ops(asm(&h), "tip");
    act(&mut h, FgAct::AsmOps(ops));
    let s = src(&h);
    assert!(!s.contains("tip") && !s.contains("hold"), "{s}");
}

#[test]
fn free_move_is_the_view_till_applied_as_a_pose() {
    let f = fixtures("free", TWO);
    let Some(mut h) = open(&f) else { return };
    let before = src(&h);
    let a = h.gui.shell.fg.as_mut().unwrap().asm.as_mut().unwrap();
    a.tool = Some(AsmTool::Free(false));
    a.moved.insert(
        "tip".into(),
        forge_math::Frame::WORLD.with_origin(forge_math::Point3::new(0.0, 0.0, 25.0)),
    );
    h.step();
    assert_eq!(src(&h), before, "a move is the view's");
    act(&mut h, FgAct::AsmApplyMoves);
    let s = src(&h);
    assert!(s.contains(", 25), z: (0, 0, 1), x: ("), "{s}");
    assert_ne!(s, before);
}

#[test]
fn a_double_click_on_a_relationship_opens_its_dialog() {
    let f = fixtures("dclick", TWO);
    let Some(mut h) = open(&f) else { return };
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
    let row = h.gui.shell.place("asm:row:l:0").expect("a relationship row");
    h.move_to(row.center().x, row.center().y);
    h.step();
    for _ in 0..2 {
        h.button(true);
        h.step();
        h.button(false);
        h.step();
    }
    h.step();
    assert!(
        matches!(&asm(&h).tool, Some(AsmTool::Link(f)) if f.editing.is_some()),
        "{:?}",
        asm(&h).tool
    );
}

#[test]
fn mirror_makes_a_reflected_part_file_and_copy_reuses_or_copies() {
    let f = fixtures("mirror", TWO);
    let Some(mut h) = open(&f) else { return };
    let dir = f.parent().unwrap().to_path_buf();
    act(
        &mut h,
        FgAct::AsmMirror {
            inst: "tip".into(),
            plane: Some(2),
            file: Some("arm_mirror.fg".into()),
        },
    );
    let made = std::fs::read_to_string(dir.join("arm_mirror.fg")).unwrap();
    assert!(
        made.contains("mirror_cut = split(by: mirror_plane, keep: \"plus\")"),
        "{made}"
    );
    let s = src(&h);
    assert!(
        s.contains("arm_mirror_1 = part(\"arm_mirror.fg\", at: pose("),
        "{s}"
    );
    // the mirrored part's anchors land reflected in the yz plane: x negated, y and z kept
    let pose = |id: &str| asm(&h).poses.iter().find(|p| p.0 == id).unwrap().1;
    let opts = forge_lang::Options {
        dir: Some(dir.clone()),
        ..Default::default()
    };
    let (run, _) = forge_lang::run(&s, opts, Default::default());
    assert!(
        run.errors.is_empty(),
        "{:?}",
        run.errors.iter().map(|e| e.text()).collect::<Vec<_>>()
    );
    let centroid = |text: &str| {
        let (r, _) = forge_lang::run(text, forge_lang::Options::default(), Default::default());
        let c = r.measure.unwrap().centroid;
        forge_math::Point3::new(c[0], c[1], c[2])
    };
    let w = pose("tip").point_to_world(centroid(&link(40.0)));
    let wm = pose("arm_mirror_1").point_to_world(centroid(&made));
    assert!(
        // the mass centroid is integrated: to its own tolerance
        (w.x + wm.x).abs() < 1e-2 && (w.y - wm.y).abs() < 1e-2 && (w.z - wm.z).abs() < 1e-2,
        "{w:?} {wm:?}"
    );
    // its anchors were the unmirrored part's: taken out, to be picked again
    assert!(forge_lang::asm::part_anchors(&made, None).is_empty());
    act(
        &mut h,
        FgAct::AsmMirror {
            inst: "tip".into(),
            plane: None,
            file: None,
        },
    );
    assert!(src(&h).contains("arm_1 = part(\"arm.fg\""), "{}", src(&h));
}

/// Both picks on one component: the live solve's SelfLink is shown and OK is held.
#[test]
fn ok_is_held_while_the_live_solve_reports_an_error() {
    let f = fixtures("selflink", TWO);
    let Some(mut h) = open(&f) else { return };
    let before = src(&h);
    let mut form = LinkForm::new_constraint(&before);
    form.picks = [pick("tip.a"), pick("tip.b")];
    form.waiting = None;
    let a = h.gui.shell.fg.as_mut().unwrap().asm.as_mut().unwrap();
    a.tool = Some(AsmTool::Link(Box::new(form)));
    h.step();
    settle(&mut h);
    h.step();
    assert!(
        h.gui.shell.places.iter().any(|p| p.0 == "asm:problem:SelfLink"),
        "{:?}",
        h.gui.shell.fg.as_ref().unwrap().issues
    );
    assert!(h.click("asm:ok"));
    h.step();
    settle(&mut h);
    assert_eq!(src(&h), before, "OK wrote a link in error");
    assert!(matches!(&asm(&h).tool, Some(AsmTool::Link(_))));
}

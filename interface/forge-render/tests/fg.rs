//! A Forge (.fg) part in the app with no window: opened, edited through the panel's typed ops,
//! rebuilt through the warm stages, fixed, saved. A machine with no GPU has nothing to test here.

use forge_lang::edit;
use forge_lang::error::{Literal, Op, ParamKey, Scope};
use forge_lang::values::Dim;
use forge_render::gui::{Gui, Headless};
use forge_render::shell::fg::Severity;
use std::path::PathBuf;

const PLATE: &str = "shared L = 60
shared t = 6
shared n = count(2)

sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (L, L))
}
base = extrude(plan, t)

param psk.x = 8
param psk.y = 8
param psk.w = 16
param psk.gap = 26
param pocket.depth = 2
for i in 0..n {
  sketch psk(on: base.end) {
    r = rect(corner: (x + gap * i, y), size: (w, w))
  }
  pocket = cut(psk, depth)
}

fn boss(on, r, h) {
  sketch face(on: on) {
    c = circle((r, r), r)
  }
  post = extrude(face, h)
  return post
}
param lug.r = 6
param lug.h = t - 2
lug = boss(base.end, r, h)

param mark.d = 2
param mark.cy = 40
sketch mark(on: base.end) {
  c = circle((30, cy), d)
}
param dent.depth = 1
dent = cut(mark, depth)
";

/// A fresh directory of the test's own, under the system's temp.
fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge-fg-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn app(file: &str) -> Option<Headless> {
    let mut h = match Headless::new((1300.0, 1000.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return None;
        }
    };
    h.step();
    h.gui.open(file);
    assert!(
        h.until(|g| built(g) && g.view.is_some()),
        "never built: {}",
        h.gui.shell.status
    );
    h.step();
    Some(h)
}

/// The latest text is built and back.
fn built(g: &Gui) -> bool {
    g.fg.as_ref().is_some_and(|s| s.done == s.seq)
}

fn settle(h: &mut Headless) {
    assert!(h.until(built));
    h.step();
    h.step();
}

fn param(g: &Gui, key: &str) -> forge_render::shell::fg::ParamRow {
    let p = g.shell.fg.as_ref().unwrap();
    p.params
        .iter()
        .find(|p| p.key.to_string() == key)
        .cloned()
        .unwrap()
}

#[test]
fn open_edit_a_param_rebuild_fix_and_save() {
    let dir = scratch("edit");
    let file = dir.join("plate.fg");
    std::fs::write(&file, PLATE).unwrap();
    let Some(mut h) = app(file.to_str().unwrap()) else {
        return;
    };
    let fg = h.gui.shell.fg.as_ref().unwrap();
    assert_eq!(h.gui.shell.name, "plate.fg");
    assert!(h.gui.shell.doc.is_none() && h.gui.shell.rows.is_empty());

    // the tree is the ids: loops' instances under their names, a call's features under its own
    let tree: Vec<(String, usize)> = fg.tree().into_iter().map(|r| (r.key, r.depth)).collect();
    for want in [
        ("base", 0),
        ("psk", 0),
        ("psk[0]", 1),
        ("psk[1]", 1),
        ("pocket[1]", 1),
        ("lug", 0),
        ("lug.face", 1),
        ("lug.post", 1),
        ("dent", 0),
    ] {
        assert!(
            tree.contains(&(want.0.to_string(), want.1)),
            "{want:?} not in {tree:?}"
        );
    }
    assert!(h.gui.shell.place("fg:row:lug.post").is_some());

    // driving, driven and structural params
    assert!(param(&h.gui, "t").driving());
    let lh = param(&h.gui, "lug.h");
    assert!(!lh.driving());
    assert_eq!(lh.links, [ParamKey::shared("t")]);
    assert!(param(&h.gui, "n").structural);
    assert!(!param(&h.gui, "t").structural);

    // the one lint, inline, with its two typed fixes
    let lint = fg.issues.iter().position(|i| i.kind == "LooseLiteral").unwrap();
    assert_eq!(fg.issues[lint].severity, Severity::Lint);
    assert_eq!(fg.issues[lint].feature.as_deref(), Some("mark"));
    assert_eq!(fg.issues[lint].fixes.len(), 2);

    // a param edited in its field in the Parameters dialog: a SetParam, rebuilt
    let before = h.gui.fg.as_ref().unwrap().seq;
    assert!(h.gui.shell.place("fg:param:t").is_none());
    h.gui.shell.fg_command(forge_render::shell::FgCmd::Parameters);
    h.step();
    h.step();
    assert!(h.click("fg:param:t"));
    h.step();
    h.step();
    h.text("8");
    h.key(egui::Key::Enter);
    h.step();
    h.step();
    let set = Op::SetParam {
        param: ParamKey::shared("t"),
        value: Literal {
            value: 8.0,
            dim: Dim::Length,
        },
    };
    assert_eq!(
        h.gui.fg.as_ref().unwrap().ops,
        std::slice::from_ref(&set),
        "{}",
        h.gui.shell.status
    );
    assert!(h.gui.fg.as_ref().unwrap().seq > before);
    settle(&mut h);
    assert_eq!(param(&h.gui, "t").value.unwrap().0, 8.0);
    // what reads t moved with it
    assert_eq!(param(&h.gui, "lug.h").value.unwrap().0, 6.0);
    assert!(h.gui.shell.fg.as_ref().unwrap().dirty);

    // the lint's first fix, by its button
    let lint = h
        .gui
        .shell
        .fg
        .as_ref()
        .unwrap()
        .issues
        .iter()
        .position(|i| i.kind == "LooseLiteral")
        .unwrap();
    let fix = h.gui.shell.fg.as_ref().unwrap().issues[lint].fixes[0].clone();
    assert!(matches!(&fix, Op::LiftLiteral { scope: Scope::Local(o), .. } if o[0].name == "mark"));
    // the issues are a line of counts until opened: a lint alone leaves them shut
    assert!(h.gui.shell.place(&format!("fg:fix:{lint}:0")).is_none());
    assert!(h.click("fg:issues:status"));
    h.step();
    h.step();
    assert!(h.click(&format!("fg:fix:{lint}:0")));
    h.step();
    assert_eq!(h.gui.fg.as_ref().unwrap().ops, [set.clone(), fix.clone()]);
    settle(&mut h);
    assert!(
        h.gui
            .shell
            .fg
            .as_ref()
            .unwrap()
            .issues
            .iter()
            .all(|i| i.kind != "LooseLiteral")
    );

    // saved from File > Save: exactly the ops' text edits, applied to what was opened
    assert!(h.gui.shell.place("title:name").is_some());
    assert!(h.click("file"));
    h.step();
    h.step();
    assert!(h.click("file-menu:Save"));
    h.step();
    h.step();
    let saved = std::fs::read_to_string(&file).unwrap();
    let want = edit::apply(&edit::apply(PLATE, &set).unwrap(), &fix).unwrap();
    assert_eq!(saved, want);
    assert!(!h.gui.shell.fg.as_ref().unwrap().dirty);
    // every byte outside the edits is as it was
    assert!(saved.contains("shared t = 8\n"));
    assert_eq!(saved.lines().count(), PLATE.lines().count() + 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tree_actions_are_typed_ops_and_a_refusal_is_typed() {
    let dir = scratch("tree");
    let file = dir.join("plate.fg");
    std::fs::write(&file, PLATE).unwrap();
    let Some(mut h) = app(file.to_str().unwrap()) else {
        return;
    };

    // the base is read below it: deleting it is refused, nothing changes
    assert!(h.click("fg:row:base"));
    h.step();
    assert_eq!(h.gui.shell.fg.as_ref().unwrap().selected.as_deref(), Some("base"));
    assert!(h.menu_item("fg:row:base", "fg:menu:delete:base"));
    let refused = h.gui.shell.fg.as_ref().unwrap().refused.clone().unwrap();
    assert!(refused.starts_with("StillRead"), "{refused}");
    assert!(h.gui.fg.as_ref().unwrap().ops.is_empty());
    assert_eq!(h.gui.fg.as_ref().unwrap().src, PLATE);

    // one loop instance gets its own width
    assert!(h.menu_item("fg:row:psk[1]", "fg:menu:split:psk.w:psk[1]"));
    let src = h.gui.fg.as_ref().unwrap().src.clone();
    assert!(src.contains("param psk[1].w = "), "{src}");
    settle(&mut h);
    assert!(
        h.gui
            .shell
            .fg
            .as_ref()
            .unwrap()
            .params
            .iter()
            .any(|p| p.key.to_string() == "psk[1].w")
    );

    // the mark moves up above the lug, and ids hold
    assert!(h.menu_item("fg:row:mark", "fg:menu:up:mark"));
    let ops = &h.gui.fg.as_ref().unwrap().ops;
    assert_eq!(
        ops.last().unwrap(),
        &Op::MoveFeature {
            id: "mark".into(),
            before: Some("lug".into())
        }
    );
    settle(&mut h);
    let ids: Vec<String> = h
        .gui
        .shell
        .fg
        .as_ref()
        .unwrap()
        .feats
        .iter()
        .map(|f| f.id.clone())
        .collect();
    let mark = ids.iter().position(|i| i == "mark").unwrap();
    let lug = ids.iter().position(|i| i == "lug.post").unwrap();
    assert!(mark < lug, "{ids:?}");

    // a feature added at the end; undo takes it back out
    let n = h.gui.shell.fg.as_ref().unwrap().feats.len();
    h.gui
        .fg_apply(Op::AddFeature {
            before: None,
            text: "rim = chamfer(edges(of: dent, shape: \"circle\", crease: \"convex\"), t / n / n / n)"
                .into(),
        })
        .unwrap();
    h.step();
    assert!(matches!(
        h.gui.fg.as_ref().unwrap().ops.last(),
        Some(Op::AddFeature { before: None, .. })
    ));
    settle(&mut h);
    assert_eq!(h.gui.shell.fg.as_ref().unwrap().feats.len(), n + 1);
    h.click("undo");
    h.step();
    settle(&mut h);
    assert_eq!(h.gui.shell.fg.as_ref().unwrap().feats.len(), n);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_fc_part_still_opens_after_a_forge_part() {
    let dir = scratch("both");
    let file = dir.join("plate.fg");
    std::fs::write(&file, PLATE).unwrap();
    let Some(mut h) = app(file.to_str().unwrap()) else {
        return;
    };
    h.gui.open("bracket");
    h.settle();
    assert!(h.until(|g| !g.shell.rows.is_empty() && g.view.as_ref().is_some_and(|v| v.live.is_some())));
    assert!(h.gui.fg.is_none() && h.gui.shell.fg.is_none());
    assert_eq!(h.gui.shell.name, "bracket.fc");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A rebuild in the app: the same text, then a late param, through the worker's warm stages.
#[test]
fn a_warm_rebuild_in_the_app_is_milliseconds() {
    let part = concat!(env!("CARGO_MANIFEST_DIR"), "/../forge-lang/parts/bracket.fg");
    let dir = scratch("warm");
    let file = dir.join("bracket.fg");
    std::fs::copy(part, &file).unwrap();
    let Some(mut h) = app(file.to_str().unwrap()) else {
        return;
    };
    let mut unchanged = vec![];
    for _ in 0..5 {
        let s = h.gui.fg.as_mut().unwrap();
        s.rebuild();
        settle(&mut h);
        unchanged.push(h.gui.fg.as_ref().unwrap().round_ms);
    }
    unchanged.sort_by(f64::total_cmp);
    let summary = h.gui.shell.fg.as_ref().unwrap().summary.clone();
    assert!(summary.contains("rebuilt 0"), "{summary}");
    // the last feature's param: one stage rebuilt, then the values tried before none
    let mut late = vec![];
    for r in [1.25, 1.5, 1.25, 1.5, 1.25] {
        h.gui
            .fg_apply(Op::SetParam {
                param: ParamKey::parse("hub_edge.r").unwrap(),
                value: Literal {
                    value: r,
                    dim: Dim::Length,
                },
            })
            .unwrap();
        settle(&mut h);
        let s = h.gui.fg.as_ref().unwrap();
        eprintln!(
            "  late edit: round {:.1} ms = build {:.1} (kernel {:.1}, measure {:.1}) + changed faces {:.1}",
            s.round_ms, s.build_ms, s.timings.kernel_ms, s.timings.measure_ms, s.mesh_ms
        );
        late.push(s.round_ms);
    }
    late.sort_by(f64::total_cmp);
    // the whole mesh follows the faces that changed
    assert!(h.until(|g| g.fg.as_ref().is_some_and(|s| s.whole == s.seq)));
    let whole_ms = h.gui.fg.as_ref().unwrap().whole_ms;
    let late_summary = h.gui.shell.fg.as_ref().unwrap().summary.clone();
    // a value the session built before is served its kept stage: nothing rebuilt
    assert!(late_summary.contains("rebuilt 0,"), "{late_summary}");
    eprintln!(
        "bracket.fg in the app: unchanged rebuild median {:.1} ms ({summary}); last param edited median {:.1} ms on screen ({late_summary}), whole mesh after it {whole_ms:.1} ms",
        unchanged[2], late[2]
    );
    assert!(unchanged[2] < 50.0, "unchanged rebuild {:.1} ms", unchanged[2]);
    assert!(late[2] < 150.0, "late edit {:.1} ms", late[2]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_sheet_part_shows_flat_and_exports_its_dxf() {
    let part = concat!(env!("CARGO_MANIFEST_DIR"), "/../forge-lang/parts/sheet/l.fg");
    let dir = scratch("sheet");
    let file = dir.join("l.fg");
    std::fs::copy(part, &file).unwrap();
    let Some(mut h) = app(file.to_str().unwrap()) else {
        return;
    };
    let sheets = h.gui.shell.fg.as_ref().unwrap().sheets.clone();
    assert_eq!(sheets.len(), 1, "{sheets:?}");
    assert_eq!(sheets[0].bends, 1);
    assert!(sheets[0].refused.is_none());
    let folded = h.gui.view.as_ref().unwrap().scene.bounds();
    // flat, from the browser's Flat Pattern row: the base and the flange laid out in one
    // plane, so longer than the folded L
    assert!(h.click("fg:row:flat"));
    assert!(h.until(|g| g.shell.fg.as_ref().is_some_and(|f| f.flat)));
    h.step();
    let flat = h.gui.view.as_ref().unwrap().scene.bounds();
    assert!(flat.1 > folded.1, "flat radius {} vs folded {}", flat.1, folded.1);
    // the flat pattern as DXF beside the part, from the ribbon
    for name in ["tool:Save Copy As DXF", "sheet:dxf:ok"] {
        h.step();
        assert!(h.click(name), "no {name}");
        h.step();
        h.step();
    }
    assert!(h.click("tool:Go to Folded Part"));
    assert!(h.until(|g| g.shell.fg.as_ref().is_some_and(|f| !f.flat)));
    let dxf = std::fs::read_to_string(dir.join("l.dxf")).unwrap();
    assert!(
        dxf.contains("SECTION") && dxf.contains("IV_BEND") && dxf.contains("IV_OUTER_PROFILE"),
        "{dxf}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The browser opens parts as mesh files the native build exports (web/build.sh); a .fg part
/// exports as an .fc one does, and reads back as the browser reads it. No GPU needed.
#[test]
fn a_forge_part_exports_the_mesh_the_browser_opens() {
    let part = concat!(env!("CARGO_MANIFEST_DIR"), "/../forge-lang/parts/bracket.fg");
    let (mesh, cached) = forge_render::load::open(part, false).unwrap();
    assert!(!cached);
    assert!(mesh.radius > 50.0 && !mesh.lods.is_empty());
    let back = forge_render::mesh::PartMesh::from_bytes(&mesh.to_bytes()).unwrap();
    assert_eq!(back, mesh);
    // what the kernel built: the bracket's 137 faces, as `fg run` reports them
    let body = forge_render::load::body(part).unwrap();
    assert_eq!(body.face_ids().count(), 137);
    // a part that doesn't build says why, with its typed error's kind
    let dir = scratch("export");
    let bad = dir.join("bad.fg");
    std::fs::write(&bad, "shared a = 10\nshared b = 30deg\nx = a + b\n").unwrap();
    let e = forge_render::load::open(bad.to_str().unwrap(), false).unwrap_err();
    assert!(e.contains("DimMismatch"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

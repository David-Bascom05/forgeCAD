//! File > Export STL / 3MF / OBJ: the menu's entries ask the host for the export, and what the
//! host then runs (`fg::export_mesh`) writes a closed mesh of the part or assembly beside it.

use forge_lang::meshfile::Format;
use forge_render::gui::Headless;
use forge_render::shell::Action;
use std::path::{Path, PathBuf};

fn repo(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fg-mesh-export-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn export_writes_a_closed_mesh_of_a_part_and_an_assembly() {
    let dir = scratch("files");
    for (part, objects) in [
        ("examples/unified/knob.fg", 1),
        ("examples/unified/peg_board.fg", 4),
    ] {
        let path = repo(part);
        let src = std::fs::read_to_string(&path).unwrap();
        for f in [Format::Stl, Format::ThreeMf, Format::Obj] {
            let out = dir.join(format!("x.{}", f.ext()));
            let said = forge_render::fg::export_mesh(&src, path.to_str(), &out, f).unwrap();
            assert!(
                said.contains("watertight and manifold") && !said.contains("not"),
                "{said}"
            );
            let bytes = std::fs::read(&out).unwrap();
            let checks: Vec<forge_mesh::MeshCheck> = match f {
                Format::ThreeMf => forge_mesh::read_3mf(&bytes).unwrap().checks(),
                Format::Obj => forge_mesh::read_obj(std::str::from_utf8(&bytes).unwrap())
                    .unwrap()
                    .iter()
                    .map(forge_mesh::Object::check)
                    .collect(),
                Format::Stl => vec![forge_mesh::read_stl(&bytes).unwrap().0[0].check()],
            };
            if f != Format::Stl {
                assert_eq!(checks.len(), objects, "{part} {f:?}");
            }
            assert!(
                checks.iter().all(|c| c.volume > 0.0 && c.triangles >= 4),
                "{part} {f:?}"
            );
            if f != Format::Stl || objects == 1 {
                assert!(
                    checks.iter().all(forge_mesh::MeshCheck::watertight),
                    "{part} {f:?}"
                );
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_file_menu_asks_for_each_export() {
    let dir = scratch("menu");
    let demo = repo("examples/demo");
    for e in std::fs::read_dir(demo).unwrap().flatten() {
        if e.path().is_file() {
            std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
        }
    }
    let mut h = match Headless::new((1400.0, 900.0), 1.0) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("skipped: {e}");
            return;
        }
    };
    h.step();
    h.gui.open(dir.join("part_tools.fg").to_str().unwrap());
    assert!(
        h.until(|g| g.fg.as_ref().is_some_and(|s| s.done == s.seq)),
        "never built: {}",
        h.gui.shell.status
    );
    for (label, f) in [
        ("Export STL...", Format::Stl),
        ("Export 3MF...", Format::ThreeMf),
        ("Export OBJ...", Format::Obj),
    ] {
        assert!(h.click("file"), "no File button");
        h.step();
        h.step();
        let place = format!("file-menu:{label}");
        let r = h.gui.shell.place(&place).unwrap_or_else(|| panic!("no {place}"));
        h.move_to(r.center().x, r.center().y);
        h.step();
        h.button(true);
        h.step();
        h.button(false);
        for _ in 0..4 {
            h.step();
        }
        assert!(!h.gui.shell.file_menu, "{label}: the menu closes");
        // the writer's options first, then Export asks the host
        assert_eq!(
            h.gui.shell.mesh_dialog.as_ref().map(|o| o.format),
            Some(f),
            "{label}: no options"
        );
        if f == Format::Stl
            && let Ok(shots) = std::env::var("FORGE_UI_SHOTS")
        {
            for _ in 0..12 {
                h.step();
            }
            if let Ok(png) = h.png() {
                std::fs::write(std::path::Path::new(&shots).join("mesh-export-options.png"), png).unwrap();
            }
        }
        let r = h.gui.shell.place("mesh:export").expect("an Export button");
        h.move_to(r.center().x, r.center().y);
        h.step();
        h.button(true);
        let mut asked = h.step().host.contains(&Action::ExportMesh(f));
        h.button(false);
        for _ in 0..4 {
            asked |= h.step().host.contains(&Action::ExportMesh(f));
        }
        assert!(asked, "{label} asked for no export");
        assert!(h.gui.shell.mesh_dialog.is_none(), "{label}: the dialog closes");
        assert_eq!(h.gui.shell.mesh_opts.as_ref().map(|o| o.format), Some(f));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_options_set_in_the_dialog_are_the_ones_written() {
    let dir = scratch("opts");
    let src = std::fs::read_to_string(repo("examples/demo/part_tools.fg")).unwrap();
    let part = dir.join("part_tools.fg");
    for e in std::fs::read_dir(repo("examples/demo")).unwrap().flatten() {
        if e.path().is_file() {
            std::fs::copy(e.path(), dir.join(e.file_name())).unwrap();
        }
    }
    // ASCII STL, a coarser chord: a text file, fewer triangles than the default's
    let mut o = forge_lang::meshfile::Options::new(Format::Stl);
    let fine = dir.join("fine.stl");
    let said = forge_render::fg::export_mesh_with(&src, part.to_str(), &fine, &o).unwrap();
    assert!(said.contains("watertight and manifold"), "{said}");
    o.ascii = true;
    o.tol *= 20.0;
    let coarse = dir.join("coarse.stl");
    forge_render::fg::export_mesh_with(&src, part.to_str(), &coarse, &o).unwrap();
    let text = std::fs::read_to_string(&coarse).expect("ASCII");
    assert!(text.starts_with("solid"), "{}", &text[..40.min(text.len())]);
    let facets = text.matches("facet normal").count();
    // binary STL: 80-byte header, a count, 50 bytes a triangle
    let fine_tris = (std::fs::metadata(&fine).unwrap().len() as usize - 84) / 50;
    assert!(facets > 0 && facets < fine_tris, "{facets} vs {fine_tris}");
    let _ = std::fs::remove_dir_all(&dir);
}

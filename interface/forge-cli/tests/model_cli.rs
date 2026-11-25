//! `forgecad import MODEL` and `forgecad convert MODEL -o OUT.step` as a user runs them, on a
//! Parasolid body cut from the Haas corpus: the part file written beside the model builds to the
//! body the reader made, nothing is overwritten, and the STEP written reads back to the same solid.
//! Native only: they start `forgecad` as a process.
#![cfg(not(target_os = "wasi"))]

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../kernel/forge-step/tests/data/xt")
        .join(name)
}

fn scratch(name: &str) -> PathBuf {
    let d = forge_dirs::temp_dir().join(format!("forgecad-model-{name}-{}", forge_dirs::process_id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn forgecad(dir: &Path, args: &[&str]) -> (bool, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    (
        o.status.success(),
        String::from_utf8_lossy(&o.stdout).into_owned() + &String::from_utf8_lossy(&o.stderr),
    )
}

#[test]
fn a_parasolid_file_imports_as_a_part_that_builds() {
    let d = scratch("import");
    let src = fixture("haas_spun_sp.x_t");
    let (ok, text) = forgecad(&d, &["import", src.to_str().unwrap()]);
    assert!(ok, "{text}");
    assert!(text.contains("1 solid(s), 0 refused"), "{text}");
    let part = std::fs::read_to_string(d.join("haas_spun_sp.fg")).unwrap();
    assert!(part.contains("body = import(\"haas_spun_sp.x_t\")"), "{part}");
    assert!(part.contains("SOLIDWORKS"), "the writer is named: {part}");
    assert!(d.join("haas_spun_sp.x_t").exists());
    // the body the reader made, its tolerant vertices kept when it joins the part
    let (ok, text) = forgecad(&d, &["build", "haas_spun_sp.fg"]);
    assert!(ok, "{text}");
    assert!(
        text.contains("volume 2773.26 mm3") && text.contains("valid"),
        "{text}"
    );
    // again: nothing overwritten
    let (ok, text) = forgecad(&d, &["import", src.to_str().unwrap()]);
    assert!(!ok && text.contains("not overwritten"), "{text}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_parasolid_file_converts_to_step_that_reads_back() {
    let d = scratch("convert");
    let src = fixture("haas_sphere.x_t");
    let (ok, text) = forgecad(
        &d,
        &["convert", src.to_str().unwrap(), "-o", "out.step", "--json"],
    );
    assert!(ok, "{text}");
    assert!(text.contains("\"solids\":1"), "{text}");
    let step = std::fs::read(d.join("out.step")).unwrap();
    assert!(step.starts_with(b"ISO-10303-21;"));
    let back = forge_step::import(&step, &Default::default()).unwrap();
    assert_eq!(back.solids.len(), 1, "{:?}", back.refusals);
    let v = back.solids[0].mass.as_ref().map(|m| m.volume).unwrap();
    assert!((v - 14.348845).abs() < 1e-4 * 14.35, "{v}");
    // a file no reader knows
    std::fs::write(d.join("junk.x_t"), b"not a model").unwrap();
    let (ok, text) = forgecad(&d, &["convert", "junk.x_t", "-o", "j.step"]);
    assert!(!ok, "{text}");
    let _ = std::fs::remove_dir_all(&d);
}

//! `forgecad cam F post [PROGRAM] [-o FILE|DIR] [--allow-uncertified]`: the files written are the
//! ones the app writes (forge_cam_lang::export::files on the same run), named and placed by the
//! program statement's `file:`/`out:`/`ext:`; a program not certified in full is refused (typed,
//! exit 1) unless `--allow-uncertified`, which stamps its header.
//! Native only: they start `forgecad` as a process.
#![cfg(not(target_os = "wasi"))]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)
}

/// A fresh folder holding copies of example `files` (a machining document and its part).
fn scratch(name: &str, files: &[&str]) -> PathBuf {
    let d = forge_dirs::temp_dir().join(format!("forgecad-campost-{name}-{}", forge_dirs::process_id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for f in files {
        std::fs::copy(repo(&format!("examples/machined/{f}")), d.join(f)).unwrap();
    }
    std::fs::canonicalize(d).unwrap()
}

fn post(doc: &Path, words: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forgecad"))
        .arg("cam")
        .arg(doc)
        .arg("post")
        .args(words)
        .output()
        .unwrap()
}

#[test]
fn a_certified_program_is_written_where_its_statement_says_and_equals_the_apps() {
    let d = scratch("mark", &["plate_mark_cam.fg", "plate_mark.fg"]);
    let doc = d.join("plate_mark_cam.fg");
    let src = std::fs::read_to_string(&doc).unwrap().replace(
        "prog = program(op1, post: \"fanuc\", number: 3001)",
        "prog = program(op1, post: \"fanuc\", number: 3001, file: \"mark-1\", out: \"nc\", comment: \"plate, mark side\", tools: \"list\")",
    );
    std::fs::write(&doc, &src).unwrap();
    let o = post(&doc, &[]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let wrote = d.join("nc/mark-1.nc");
    let text = std::fs::read_to_string(&wrote).unwrap();
    assert!(text.starts_with("%\n(plate, mark side)\n(TOOLS)\n(T1 flat-10"), "{text}");
    // the app's files: the same run, the same function
    let cam = forge_cam_lang::run(&src, &d);
    let app = forge_cam_lang::export::files(&src, &cam.export_programs(), None, false).unwrap();
    assert_eq!(app.len(), 1);
    assert_eq!(app[0].path, "nc/mark-1.nc");
    assert_eq!(app[0].text, text, "the CLI wrote what the app writes");
    // -o FILE for one program
    let o = post(&doc, &["prog", "-o", d.join("x.tap").to_str().unwrap()]);
    assert!(o.status.success());
    assert_eq!(std::fs::read_to_string(d.join("x.tap")).unwrap(), text);
}

#[test]
fn an_uncertified_program_is_refused_unless_allowed_and_then_stamped() {
    let d = scratch("flange", &["flange_cam.fg", "flange.fg"]);
    let doc = d.join("flange_cam.fg");
    // the setup not placed: its positions on the machine are unchecked (G5)
    let src = std::fs::read_to_string(&doc).unwrap().replace(", at: (0, 0, -250)", "");
    assert!(!src.contains("at: ("));
    std::fs::write(&doc, src).unwrap();
    let o = post(&doc, &[]);
    assert!(!o.status.success());
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("refused: program `prog` is certified except G5"), "{err}");
    assert!(err.contains("fix: add `at: (x, y, z)` to the setup"), "{err}");
    assert!(!d.join("prog.nc").exists(), "nothing written when refused");
    let o = post(&doc, &["--allow-uncertified"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let text = std::fs::read_to_string(d.join("prog.nc")).unwrap();
    assert!(text.lines().take(3).any(|l| l.starts_with("(NOT CERTIFIED IN FULL: except G5")), "{text}");
    let o = post(&doc, &["nope"]);
    assert!(String::from_utf8_lossy(&o.stderr).contains("no program `nope` (the document's: prog)"));
}

#[test]
fn a_turning_program_goes_into_a_folder() {
    let d = scratch("shaft", &["shaft_turn.fg", "shaft.fg"]);
    let out = d.join("out");
    let o = post(&d.join("shaft_turn.fg"), &["-o", &format!("{}/", out.display())]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(std::fs::read_to_string(out.join("prog.nc")).unwrap().contains("G"));
}

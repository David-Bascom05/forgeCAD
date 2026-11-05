//! Make, Extract and Inline Component: all or nothing, the same model before and after where the
//! proof says so, and refused typed (every reader named) where it would not be.

use forge_lang::refactor::{Found, Refactor, Written, apply_checked};
use forge_lang::{Options, Run};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("refactor-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_in(d: &Path, src: &str) -> Run {
    let opts = Options {
        dir: Some(d.to_path_buf()),
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn errors(r: &Run) -> Vec<String> {
    r.errors
        .iter()
        .map(|e| format!("{} {}", e.kind(), e.text()))
        .collect()
}

/// Each part's volume by the file (or call) it is, times its count: what the model is.
fn volumes(r: &Run) -> BTreeMap<String, f64> {
    let a = r.asm.as_ref().expect("an assembly");
    let mut out = BTreeMap::new();
    for (f, n) in &a.bom {
        let v: f64 = a.part_docs[f].masses().unwrap().iter().map(|m| m.volume).sum();
        out.insert(f.clone(), v * *n as f64);
    }
    out
}

fn total(r: &Run) -> f64 {
    volumes(r).values().sum()
}

fn apply_in(d: &Path, src: &str, r: &Refactor) -> Result<Written, forge_lang::error::Error> {
    let read = |f: &str| std::fs::read_to_string(d.join(f)).map_or(Found::Missing, Found::Text);
    let w = apply_checked(src, r, &read)?;
    for (f, t) in &w.files {
        std::fs::write(d.join(f), t).unwrap();
    }
    Ok(w)
}

const PEGS: &str = "shared small = 6
shared big = 10
shared plen = 30
shared W = 2
properties(material: \"6061-T6\")
fn half(x) {
  return x / 2
}
sketch plan(on: xy) {
  o = rect(center: (0, 0), size: (120, 60))
}
board = extrude(plan, 8)
component peg(d: 6, h: 30) {
  sketch s(on: xy) {
    c = circle((0, 0), half(d) + W)
  }
  shank = extrude(s, h)
  properties(part_number: \"PEG\", material: \"AISI 304\")
}
p1 = part(peg(d: small, h: plen), at: (-30, 0, 8))
p2 = part(peg(d: big, h: plen), at: (30, 0, 8))
p3 = part(peg, at: (0, 20, 8))
ground(board)
";

#[test]
fn a_component_moves_to_its_own_file_and_back_the_same_model() {
    let d = dir("extract");
    let before = run_in(&d, PEGS);
    assert!(before.ok(), "{:?}", errors(&before));
    let w = apply_in(
        &d,
        PEGS,
        &Refactor::ExtractComponent {
            component: "peg".into(),
            file: "peg.fg".into(),
        },
    )
    .unwrap();
    let file = &w.files[0].1;
    // the shared param it reads is the file's own, passed at every call; its function copied
    assert!(
        file.contains("shared W = 2\n") && file.contains("shared d = 6\n") && file.contains("fn half(x)"),
        "{file}"
    );
    assert!(
        w.doc
            .contains("p1 = part(\"peg.fg\", with: {d: small, h: plen, W: W}, at: (-30, 0, 8))"),
        "{}",
        w.doc
    );
    assert!(
        w.doc
            .contains("p3 = part(\"peg.fg\", with: {W: W}, at: (0, 20, 8))"),
        "{}",
        w.doc
    );
    assert!(!w.doc.contains("component peg"));
    let mid = run_in(&d, &w.doc);
    assert!(mid.ok(), "{:?}", errors(&mid));
    assert!(
        (total(&mid) - total(&before)).abs() < 1e-6,
        "{:?} {:?}",
        volumes(&mid),
        volumes(&before)
    );
    // and back: the calls are calls again, the model the same
    let back = apply_in(
        &d,
        &w.doc,
        &Refactor::InlineComponent {
            file: "peg.fg".into(),
            name: "peg".into(),
        },
    )
    .unwrap();
    // W, passed through, is the document's shared param again (no param of the block)
    assert!(back.doc.contains("component peg(d: 6, h: 30) {"), "{}", back.doc);
    assert!(
        back.doc
            .contains("p1 = part(peg(d: small, h: plen), at: (-30, 0, 8))"),
        "{}",
        back.doc
    );
    assert!(
        back.doc.contains("p3 = part(peg, at: (0, 20, 8))"),
        "{}",
        back.doc
    );
    let after = run_in(&d, &back.doc);
    assert!(after.ok(), "{:?}", errors(&after));
    assert!((total(&after) - total(&before)).abs() < 1e-6);
}

#[test]
fn extract_refuses_typed() {
    let d = dir("extract-no");
    // read other than placed
    let src = PEGS.replace("ground(board)", "ground(board)\nq = peg\n");
    let e = apply_in(
        &d,
        &src,
        &Refactor::ExtractComponent {
            component: "peg".into(),
            file: "peg.fg".into(),
        },
    )
    .unwrap_err();
    assert_eq!(e.kind(), "ReadOtherwise", "{}", e.text());
    // a file that is there already
    std::fs::write(d.join("peg.fg"), "shared x = 1\n").unwrap();
    let e = apply_in(
        &d,
        PEGS,
        &Refactor::ExtractComponent {
            component: "peg".into(),
            file: "peg.fg".into(),
        },
    )
    .unwrap_err();
    assert_eq!(e.kind(), "FileExists");
}

const TWO: &str = "shared t = 4
sketch plan(on: xy) {
  o = rect(center: (0, 0), size: (60, 40))
}
base = extrude(plan, t)
sketch ks(on: xy) {
  c = circle((80, 0), t)
}
knob = extrude(ks, t, new: true)
";

#[test]
fn bodies_become_a_component_placed_where_they_were() {
    let d = dir("make");
    let before = run_in(&d, TWO);
    assert!(before.ok());
    let v0 = before.measure.as_ref().unwrap().volume;
    let w = apply_in(
        &d,
        TWO,
        &Refactor::MakeComponent {
            statements: vec!["ks".into(), "knob".into()],
            name: "knobc".into(),
        },
    )
    .unwrap();
    assert!(
        w.doc.contains("component knobc() {\n  sketch ks(on: xy) {"),
        "{}",
        w.doc
    );
    assert!(w.doc.contains("knobc_1 = part(knobc)"), "{}", w.doc);
    let after = run_in(&d, &w.doc);
    assert!(after.ok(), "{:?}", errors(&after));
    assert!((total(&after) - v0).abs() < 1e-6, "{:?} {v0}", volumes(&after));
}

#[test]
fn make_refuses_with_every_reader() {
    let d = dir("make-no");
    // the lid is sketched on the base: moving the lid alone would leave `base` unread inside
    let src = "shared t = 3
sketch plan(on: xy) {
  o = rect(center: (0, 0), size: (60, 40))
}
base = extrude(plan, 20)
sketch lp(on: base.end) {
  o = rect(center: (0, 0), size: (60, 40))
}
lid = extrude(lp, t, new: true)
hp = plane(at: edges(between: (lid.start, lid.side(lp.o[2]))).center, normal: lp.o[2].dir)
ground(base)
hinge = revolute(lid.hp, base.hp)
";
    let e = apply_in(
        &d,
        src,
        &Refactor::MakeComponent {
            statements: vec!["lp".into(), "lid".into()],
            name: "lidc".into(),
        },
    )
    .unwrap_err();
    assert_eq!(e.kind(), "ReadsOutside", "{}", e.text());
    // a body whose own statements read nothing outside, but which is read after: every reader
    let src2 = TWO.to_string() + "ground(base)\nj = fastened(knob, base)\nk2 = extrude(ks, t, new: true)\n";
    let e = apply_in(
        &d,
        &src2,
        &Refactor::MakeComponent {
            statements: vec!["ks".into(), "knob".into()],
            name: "kc".into(),
        },
    )
    .unwrap_err();
    assert_eq!(e.kind(), "StillReadBy", "{}", e.text());
    assert!(e.text().contains("j") && e.text().contains("k2"), "{}", e.text());
}

#[test]
fn a_new_file_is_never_written_through_a_link_or_outside() {
    use forge_lang::refactor::write_new;
    let d = dir("write-new");
    let target = d.join("victim.txt");
    std::fs::write(&target, "keep").unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, d.join("cup.fg")).unwrap();
        std::os::unix::fs::symlink(d.join("nowhere"), d.join("ghost.fg")).unwrap();
        assert!(write_new(&d, "cup.fg", "x").is_err());
        assert!(write_new(&d, "ghost.fg", "x").is_err());
        assert!(!d.join("nowhere").exists());
    }
    for bad in ["../x.fg", "/tmp/x.fg", "a/b.fg", ".fg", "x.txt", ".."] {
        assert!(write_new(&d, bad, "x").is_err(), "{bad}");
    }
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "keep");
    write_new(&d, "fresh.fg", "shared a = 1\n").unwrap();
    assert!(write_new(&d, "fresh.fg", "again").is_err());
    // the refactor itself refuses a name that is not plain
    let e = apply_in(
        &d,
        PEGS,
        &Refactor::ExtractComponent {
            component: "peg".into(),
            file: "../peg.fg".into(),
        },
    )
    .unwrap_err();
    assert_eq!(e.kind(), "BadFileName", "{}", e.text());
}

#[test]
fn each_refusal_on_disk_is_its_own() {
    use forge_lang::refactor::run_on_disk;
    let d = dir("on-disk");
    let doc = d.join("pegs.fg");
    std::fs::write(&doc, PEGS).unwrap();
    let ext = |file: &str| Refactor::ExtractComponent {
        component: "peg".into(),
        file: file.into(),
    };
    let kind =
        |doc: Option<&Path>, r: &Refactor| run_on_disk(doc, None, PEGS, r).unwrap_err().kind().to_string();
    assert_eq!(kind(None, &ext("peg.fg")), "UntitledDocument");
    assert_eq!(kind(Some(&doc), &ext("../peg.fg")), "BadFileName");
    std::fs::write(d.join("taken.fg"), "shared x = 1\n").unwrap();
    assert_eq!(kind(Some(&doc), &ext("taken.fg")), "FileExists");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(d.join("nowhere"), d.join("ghost.fg")).unwrap();
        assert_eq!(kind(Some(&doc), &ext("ghost.fg")), "TargetIsLink");
    }
}

#[test]
fn inline_reads_nothing_outside_the_project() {
    use forge_lang::refactor::run_on_disk;
    let base = dir("fence-inline");
    let proj = base.join("proj");
    std::fs::create_dir_all(proj.join(".git")).unwrap();
    std::fs::create_dir_all(proj.join("hw")).unwrap();
    std::fs::create_dir_all(base.join("out")).unwrap();
    std::fs::write(base.join("out/secret.fg"), "shared s = 1\nbox(size: (s, s, s))\n").unwrap();
    std::fs::write(proj.join("hw/ok.fg"), "shared s = 1\nbox(size: (s, s, s))\n").unwrap();
    let doc = proj.join("a.fg");
    let src = "x = part(\"../out/secret.fg\")\n";
    std::fs::write(&doc, src).unwrap();
    let inl = |file: &str| Refactor::InlineComponent {
        file: file.into(),
        name: "s".into(),
    };
    let e = run_on_disk(Some(&doc), None, src, &inl("../out/secret.fg")).unwrap_err();
    assert_eq!(e.kind(), "Outside", "{}", e.text());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(base.join("out/secret.fg"), proj.join("link.fg")).unwrap();
        let e = run_on_disk(Some(&doc), None, src, &inl("link.fg")).unwrap_err();
        assert_eq!(e.kind(), "Outside", "{}", e.text());
    }
    // inside the project, up a folder, is read
    let sub = proj.join("asm");
    std::fs::create_dir_all(&sub).unwrap();
    let doc2 = sub.join("b.fg");
    let src2 = "x = part(\"../hw/ok.fg\")\n";
    std::fs::write(&doc2, src2).unwrap();
    let e = run_on_disk(Some(&doc2), None, src2, &inl("../hw/ok.fg"));
    assert!(e.as_ref().map_or_else(|e| e.kind() != "Outside", |_| true));
}

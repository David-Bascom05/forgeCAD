//! Parts pinned to a named version: `part("bracket.fg", version: 1)` builds the file as that
//! version saved it, whatever the working copy says now; latest follows the working copy; a
//! version that is not there is a typed error naming the newest.

use forge_git::Sig;
use forge_history::{History, Saved};
use forge_lang::{Options, Run};
use std::path::{Path, PathBuf};

fn dir(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("pins-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn who(t: i64) -> Sig {
    Sig {
        name: "David Bascom".into(),
        email: "davidsprojects7@gmail.com".into(),
        time: t,
        tz: 0,
    }
}

fn block(w: f64) -> String {
    format!(
        "shared w = {w}
sketch plan(on: xy) {{
  outline = rect(corner: (0, 0), size: (w, w))
}}
block = extrude(plan, 5)
"
    )
}

fn run_in(d: &Path, src: &str) -> Run {
    let opts = Options {
        dir: Some(d.to_path_buf()),
        root: Some(d.to_path_buf()),
        render: true,
        ..Options::default()
    };
    forge_lang::run(src, opts, Default::default()).0
}

fn volume(r: &Run, inst: &str) -> f64 {
    let a = r.asm.as_ref().expect("an assembly");
    let i = a
        .occurrences
        .iter()
        .position(|o| o.0 == inst)
        .expect("the occurrence");
    let b = &a.parts[a.occ_part[i].expect("a part")];
    forge_topo::mass::mass(b).expect("mass").volume
}

#[test]
fn a_pinned_part_builds_as_its_version_saved_it() {
    let d = std::fs::canonicalize(dir("v")).unwrap();
    let h = History::turn_on(&d).unwrap();
    let part = d.join("bracket.fg");
    std::fs::write(&part, block(10.0)).unwrap();
    assert!(matches!(
        h.save_as(&part, &[], "v1", None, &who(1)).unwrap(),
        Saved::New(_)
    ));
    h.create_version_as("bracket.fg", "small", "", None, &who(2))
        .unwrap();
    std::fs::write(&part, block(20.0)).unwrap();
    assert!(matches!(
        h.save_as(&part, &[], "v2", None, &who(3)).unwrap(),
        Saved::New(_)
    ));
    h.create_version_as("bracket.fg", "big", "", None, &who(4))
        .unwrap();
    // the working copy moves on, unsaved
    std::fs::write(&part, block(30.0)).unwrap();
    forge_lang::pins::forget();

    let r = run_in(
        &d,
        "assembly
old = part(\"bracket.fg\", version: 1, ground: true)
second = part(\"bracket.fg\", version: 2, at: pose(at: (100, 0, 0)), ground: true)
now = part(\"bracket.fg\", at: pose(at: (200, 0, 0)), ground: true)
",
    );
    assert!(
        r.errors.is_empty(),
        "{:?}",
        r.errors.iter().map(|e| e.text()).collect::<Vec<_>>()
    );
    assert!((volume(&r, "old") - 500.0).abs() < 1e-6);
    assert!((volume(&r, "second") - 2000.0).abs() < 1e-6);
    assert!((volume(&r, "now") - 4500.0).abs() < 1e-6);
    assert_eq!(forge_lang::pins::newest(&d, "bracket.fg"), Some(2));

    let r = run_in(&d, "assembly\nx = part(\"bracket.fg\", version: 7)\n");
    let texts: Vec<String> = r.errors.iter().map(|e| e.text()).collect();
    assert!(
        texts.iter().any(|t| t.contains("no version 7 (the newest is 2)")),
        "{texts:?}"
    );
}

#[test]
fn a_version_where_no_history_is_kept_says_so() {
    let d = dir("none");
    std::fs::write(d.join("bracket.fg"), block(10.0)).unwrap();
    forge_lang::pins::forget();
    // the test's own temp dir sits inside the ForgeCAD checkout's target: give it a root of its own
    let r = run_in(&d, "assembly\nx = part(\"bracket.fg\", version: 1)\n");
    let texts: Vec<String> = r.errors.iter().map(|e| e.text()).collect();
    assert!(texts.iter().any(|t| t.contains("bracket.fg")), "{texts:?}");
}

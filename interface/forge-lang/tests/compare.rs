//! Compare between saves: a hole added between two versions of a block is one added face, its
//! drilled faces changed, the walls the same, and exactly the hole's volume removed; the feature
//! and param lists say what was edited.

use forge_git::Sig;
use forge_history::History;
use forge_lang::compare::{self, Change};
use std::path::PathBuf;

#[test]
fn edits_are_said_shortly() {
    let d = compare::deltas(
        "sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (w, 30))\n}",
        "sketch plan(on: xy) {\n  outline = rect(corner: (0, 0), size: (w, 36))\n}",
    );
    assert_eq!(
        d,
        [(
            "outline size".to_string(),
            "(w, 30)".to_string(),
            "(w, 36)".to_string()
        )]
    );
    let d = compare::deltas("base = extrude(plan, 10)", "base = extrude(plan, 12)");
    assert_eq!(
        d,
        [("argument 2".to_string(), "10".to_string(), "12".to_string())]
    );
    assert_eq!(compare::title("sheet_flange"), "Sheet flange");
}

fn who(t: i64) -> Sig {
    Sig {
        name: "David Bascom".into(),
        email: "davidsprojects7@gmail.com".into(),
        time: t,
        tz: 0,
    }
}

const V1: &str = "shared w = 40
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (w, 30))
}
base = extrude(plan, 10)
";

const V2: &str = "shared w = 40
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (w, 30))
}
base = extrude(plan, 10)
sketch spot(on: base.end) {
  p = pt(20, 15)
}
hole1 = hole(at: spot.p, d: 6, depth: thru)
";

#[test]
fn a_hole_between_two_versions() {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("compare-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let d = std::fs::canonicalize(d).unwrap();
    let h = History::turn_on(&d).unwrap();
    let f = d.join("block.fg");
    std::fs::write(&f, V1).unwrap();
    h.save_as(&f, &[], "block", None, &who(1)).unwrap();
    h.create_version_as("block.fg", "plain", "", None, &who(2))
        .unwrap();
    std::fs::write(&f, V2.replace("w = 40", "w = 45")).unwrap();
    h.save_as(&f, &[], "block: hole", None, &who(3)).unwrap();
    h.create_version_as("block.fg", "drilled", "", None, &who(4))
        .unwrap();
    std::fs::write(&f, "not even forge (\n").unwrap();
    forge_lang::pins::forget();

    let c = compare::between(&h, "block.fg", "v1", "v2").unwrap();
    assert!(c.base.run.errors.is_empty() && c.target.run.errors.is_empty());
    let feat = |n: &str| c.features.iter().find(|x| x.name == n).map(|x| x.change);
    assert_eq!(feat("hole1"), Some(Change::Added));
    assert_eq!(feat("base"), Some(Change::Same));
    let hole = c.features.iter().find(|x| x.name == "hole1").unwrap();
    assert_eq!(hole.summary(), "Hole Ø6 added");
    assert!(
        !hole.faces.1.is_empty() && hole.faces.0.is_empty(),
        "{:?}",
        hole.faces
    );
    let w = c.params.iter().find(|p| p.name == "w").unwrap();
    assert_eq!(w.summary(), "40 to 45 mm");
    assert_eq!(
        (w.change, w.old.as_deref(), w.new.as_deref()),
        (Change::Changed, Some("40"), Some("45"))
    );

    let fc = c.faces.as_ref().unwrap();
    // the hole's wall is new; the wider top, bottom, front and back are changed; the two ends
    // keep their shape (the far one moved: changed)
    assert_eq!(fc.added, 1, "{fc:?}");
    assert_eq!(fc.removed, 0);
    assert_eq!(fc.same + fc.changed, 6);
    assert!(fc.same >= 1, "{fc:?}");

    let (a, b) = (
        c.base.run.body.as_ref().unwrap(),
        c.target.run.body.as_ref().unwrap(),
    );
    let v = compare::volume(a, b).unwrap();
    let hole = std::f64::consts::PI * 9.0 * 10.0;
    assert!((v.added - 5.0 * 30.0 * 10.0).abs() < 1e-6, "{}", v.added);
    assert!((v.removed - hole).abs() < 1e-3 * hole, "{} vs {hole}", v.removed);

    let j = c.json(Some(&Ok(v))).to_string();
    assert!(j.contains("\"hole1\"") && j.contains("added_mm3"), "{j}");
    // the working copy too, though it does not build
    let now = compare::between(&h, "block.fg", "v2", "current").unwrap();
    assert!(!now.target.run.errors.is_empty());
}

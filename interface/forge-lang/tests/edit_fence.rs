//! An edit's evaluations are fenced as the document's own (`edit::At`): under a session root, a
//! `use` of a file outside it, by `..` or by an absolute path, is refused (`UseOutside`) and never
//! read; with only the document's directory the same file is read. The source's evaluation for a
//! param's dimension (SetParam) has the same fence.

use forge_agent::json::Json;
use forge_lang::edit::{self, At};
use forge_lang::error::Op;
use std::path::PathBuf;

/// `root/doc/` holding the document, and `root/x.fg` beside it, outside `root/doc/`.
fn place(tag: &str) -> (PathBuf, PathBuf) {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("edit-fence-{tag}"));
    let doc = base.join("doc");
    std::fs::create_dir_all(&doc).unwrap();
    std::fs::write(base.join("x.fg"), "shared W = 7\n").unwrap();
    let base = std::fs::canonicalize(base).unwrap();
    (base.join("doc"), base.join("x.fg"))
}

const PART: &str =
    "shared L = 10\nsketch s(on: xy) {\n  o = rect(corner: (0, 0), size: (L, L))\n}\nb = extrude(s, 2)\n";

fn add(text: &str) -> Op {
    let j = Json::obj(vec![
        ("op", "AddFeature".into()),
        ("before", Json::Null),
        ("text", text.into()),
    ]);
    Op::from_json(&j).expect("an op")
}

fn refused_outside(r: Result<(String, Option<edit::Reach>), forge_lang::error::Error>) -> bool {
    match r {
        Err(e) => e.kind() == "UseOutside",
        Ok(_) => false,
    }
}

#[test]
fn a_use_outside_the_root_is_refused() {
    let (doc, x) = place("rel");
    let fenced = At {
        dir: Some(doc.clone()),
        root: Some(doc.clone()),
    };
    let open = At {
        dir: Some(doc.clone()),
        root: None,
    };
    for file in ["../x.fg".to_string(), x.display().to_string()] {
        let op = add(&format!("t = use({file:?})"));
        let r = edit::apply_checked_at(PART, &op, &fenced);
        assert!(refused_outside(r.clone()), "{file} under the root: {r:?}");
        // the fence is what refuses it: with the directory alone the file is read
        let r = edit::apply_checked_at(PART, &op, &open);
        assert!(r.is_ok(), "{file} with no root: {r:?}");
    }
}

#[test]
fn a_set_param_reads_its_dimension_under_the_fence() {
    let (doc, _) = place("dim");
    // `W` is the used file's: read under no root, refused under one (then no dimension to keep)
    let src = "t = use(\"../x.fg\")\nshared W = t.W\nshared L = 10\n";
    let op = Op::from_json(
        &Json::parse(r#"{"op":"SetParam","param":"L","value":{"value":4,"dim":"Length"}}"#).unwrap(),
    )
    .unwrap();
    let fenced = At {
        dir: Some(doc.clone()),
        root: Some(doc.clone()),
    };
    let text = edit::apply_at(src, &op, &fenced).expect("L is set under the fence too");
    assert!(text.contains("shared L = 4"), "{text}");
    let checked = edit::apply_checked_at(src, &op, &fenced);
    // the source already had the refusal: the edit adds none, so it is taken
    assert!(checked.is_ok(), "{checked:?}");
}

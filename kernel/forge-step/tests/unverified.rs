//! A body the reader cannot check is imported with a warning in words, not refused: its integrals and
//! mesh disagree, or its volume cannot be worked out. Only a budget run out refuses (mesh_bound.rs).
use forge_math::{Frame, Point2};
use forge_ops::{Profile, extrude};
use forge_step::{Limits, Options, export, import};

/// A 10 x 6 x 2 plate with a 2 x 2 square hole, as written here.
fn plate() -> String {
    let p = Profile {
        plane: Frame::WORLD,
        loops: vec![
            Profile::rectangle(Point2::new(-5.0, -3.0), 10.0, 6.0, 0),
            Profile::rectangle(Point2::new(-1.0, -1.0), 2.0, 2.0, 9),
        ],
    };
    export(&extrude(&p, 2.0, 1).unwrap(), &Options::default())
        .unwrap()
        .text
}

fn read(text: &str) -> forge_topo::Body {
    let got = import(text.as_bytes(), &Limits::default()).expect("reads");
    assert!(got.refusals.is_empty(), "{:?}", got.refusals);
    assert_eq!(got.solids.len(), 1);
    got.solids.into_iter().next().unwrap().body
}

fn moved(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "{from}");
    text.replace(from, to)
}

#[test]
fn a_body_that_checks_carries_no_warning() {
    assert_eq!(read(&plate()).unverified, None);
}

/// The bottom's plane 0.3 above its edges: valid, but its integrals and its mesh tell two volumes.
#[test]
fn integrals_and_mesh_that_disagree_import_with_a_warning() {
    let text = moved(
        &plate(),
        "#33=CARTESIAN_POINT('',(0.,0.,0.));",
        "#33=CARTESIAN_POINT('',(0.,0.,0.3));",
    );
    let b = read(&text);
    assert_eq!(forge_topo::check(&b), vec![]);
    let why = b.unverified.expect("warned");
    assert!(why.contains("one of the two is wrong"), "{why}");
}

/// The top's outer loop said the wrong way round: no region to integrate.
#[test]
fn a_volume_that_cannot_be_worked_out_imports_with_a_warning() {
    let text = moved(
        &plate(),
        "#209=FACE_OUTER_BOUND('',#208,.T.);",
        "#209=FACE_OUTER_BOUND('',#208,.F.);",
    );
    let why = read(&text).unverified.expect("warned");
    assert!(why.starts_with("its volume cannot be worked out"), "{why}");
}

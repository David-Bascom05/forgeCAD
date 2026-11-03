//! Edits on parts with the wired features (the valve and hub ports): a pattern's count, a
//! hole's size and a thread's length changed, a pattern deleted, a hole's deletion refused while
//! its thread and pattern read it, a work plane moved. After each the part builds, every feature
//! the edit did not add or delete keeps its id, and the copies a pattern made keep their names.

use forge_lang::edit::{apply, apply_checked};
use forge_lang::error::{Literal, Op, ParamKey};
use forge_lang::values::Dim;
use forge_lang::{Options, Run};
use std::collections::BTreeSet;

fn part(name: &str) -> String {
    std::fs::read_to_string(format!("{}/parts/bench/{name}.fg", env!("CARGO_MANIFEST_DIR"))).expect("part")
}

fn build(src: &str) -> Run {
    let r = forge_lang::run(src, Options::default(), Default::default()).0;
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, src));
    r
}

fn ids(r: &Run) -> Vec<String> {
    r.feats.iter().map(|f| f.id.clone()).collect()
}

fn names(r: &Run) -> BTreeSet<u64> {
    let b = r.body.as_ref().expect("a body");
    b.face_ids().map(|f| b.face(f).name.0).collect()
}

fn set(param: &str, value: f64, dim: Dim) -> Op {
    Op::SetParam {
        param: ParamKey::parse(param).expect("a param"),
        value: Literal { value, dim },
    }
}

#[test]
fn edits_on_the_valve_keep_ids() {
    let src = part("valve");
    let r0 = build(&src);
    // three by three screws: the faces there were keep their names
    let s1 = apply(&src, &set("screws.n", 3.0, Dim::Count)).expect("applies");
    let r1 = build(&s1);
    assert_eq!(ids(&r0), ids(&r1));
    assert!(names(&r0).iter().filter(|n| names(&r1).contains(n)).count() > names(&r0).len() * 9 / 10);
    // a port's drill and the P thread's length
    let s2 = apply(&src, &set("port", 7.0, Dim::Length)).expect("applies");
    assert_eq!(ids(&build(&s2)), ids(&r0));
    let s3 = apply(&src, &set("pt_thread", 10.0, Dim::Length)).expect("applies");
    assert_eq!(ids(&build(&s3)), ids(&r0));
    // the rings' pattern deleted: everything else keeps its id
    let s4 = apply(&src, &Op::DeleteFeature { id: "rings".into() }).expect("applies");
    let r4 = build(&s4);
    let mut want = ids(&r0);
    want.retain(|i| i != "rings");
    assert_eq!(ids(&r4), want);
    // the screw hole is read by its pattern and its thread: deleting it is refused, typed
    let e = apply_checked(&src, &Op::DeleteFeature { id: "screw".into() }).expect_err("refused");
    assert_eq!(e.kind(), "StillRead");
    // a gallery's work plane moved: the drill that ends on it follows, ids hold
    let s5 = apply(&src, &set("py", 10.0, Dim::Length)).expect("applies");
    assert_eq!(ids(&build(&s5)), ids(&r0));
}

#[test]
fn edits_on_the_hub_keep_ids() {
    let src = part("hub");
    let r0 = build(&src);
    for (p, v, d) in [
        ("ns", 8.0, Dim::Count),
        ("pcd", 110.0, Dim::Length),
        ("grooves.step", 26.0, Dim::Length),
        ("stud.d", 13.0, Dim::Length),
    ] {
        let s = apply(&src, &set(p, v, d)).expect("applies");
        assert_eq!(ids(&build(&s)), ids(&r0), "{p}");
    }
}

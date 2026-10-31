//! Part files the random history generator (forge_agent::history) shrank from its failures:
//! each builds valid, meshes closed and regenerates the same n-ary, or is refused with a clear
//! error; never a panic or a hang.
use forge_agent::history::{Verdict, judge};
use forge_agent::model::Model;
use forge_model::Part;

fn refused(src: &str, why: &str) {
    match judge(src) {
        Verdict::Refused(w) if w.contains(why) => {}
        v => panic!("{v:?}, not refused with '{why}'"),
    }
}

fn ok(src: &str) {
    assert_eq!(judge(src), Verdict::Ok);
}

/// Builds valid feature by feature and n-ary (the mesh left to forge-mesh's own tests).
fn builds_valid(src: &str) {
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let b = m.body().expect("a body");
    assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
    let part = Part {
        features: m.features.clone(),
    };
    let b = part.regenerate_nary(1).expect("regenerates in one pass");
    assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
}

/// A revolve keep whose tool misses the part leaves nothing: refused there, so a split of what
/// would be left no longer panics on an empty box.
#[test]
fn keep_that_leaves_nothing() {
    let src = "extrude 14.83 rect 0 0 117.88 47.1
fillet 0:4.2,0.72:2.6,1:2.18 (edges near 58.94 0 0) (edges near 117.88 47.1 7.42) (edges near 58.94 47.1 14.83)
draft -7.36 xy at 0 0 11.59 (faces plane near 0 23.55 7.42)
revolve keep sym 140.83 xz at 111.27 40.53 26.03 axis 0 0 0 -1 poly 14.6 5.61 14.6 -5.61 25.83 0
";
    refused(src, "leaves nothing");
    refused(
        &format!("{src}split plane z 1 0 0 at 0 0 0 +\n"),
        "leaves nothing",
    );
}

/// A swept tube patterned so its copies overlap, then a fillet of the sweep's arcs: some picked
/// arcs are seams between coplanar faces the copies left, removed when the faces merge (it
/// panicked looking them up). They are dropped; the rest round.
#[test]
fn fillet_picks_a_flush_seam() {
    builds_valid("L0: extrude 28.15 rect 0 0 97.81 38.36
L1: sweep add z 0 -1 0 x 1 0 0 at 48.91 31.45 29.19 rect -2.09 -2.09 4.18 4.18 path line 48.91 31.45 29.19 48.91 3.5500000000000007 29.19 arc z 0 0 1 x -1 0 0 at 55.92999999999999 3.5500000000000007 29.19 7.02 0 90 line 55.92999999999999 -3.469999999999999 29.19 75.58999999999999 -3.47 29.19
L2: pattern L1 lin 0.82 -0.58 0 4 10.8
L3: hole 9.06 z -1 0 0 at 68.63 -6.59 28.17 thru pt 118
L5: fillet 0.09..0.09 (edges circle of L1)
");
}

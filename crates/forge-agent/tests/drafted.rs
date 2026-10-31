//! Real parts on drafted faces (draft tilts planes and turns rounds into cones): each builds
//! valid, one feature after another and all at once.

use forge_agent::model::Model;
use forge_model::Part;

fn builds_valid(src: &str) {
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error);
    let body = m.body().expect("built");
    let problems = forge_topo::check(body);
    assert!(problems.is_empty(), "seq: {problems:?}");
    let part = Part {
        features: m.features.clone(),
    };
    let nary = part.regenerate_nary(2).expect("n-ary regenerates");
    let problems = forge_topo::check(&nary);
    assert!(problems.is_empty(), "n-ary: {problems:?}");
}

/// A modeled thread in a boss on a 2 degree wall: the tilted wall meets the thread's faces
/// where their tracks turn, finer than the region's tracing step.
#[test]
fn thread_in_boss_on_drafted_wall() {
    builds_valid(
        "box: extrude 95 rect -120 -65 240 130
draft -2 xy at 0 0 95 (faces of box normal +x)
extrude 12 yz at 114 0 22 circle 0 0 14
hole 14.4 -yz at 126 0 22 16 thread 16 1.5
thread hole hole1 12
",
    );
}

/// A drafted rib whose side runs through the vertex blend at a foot's corner: the cut crosses
/// the blend's rims where it meets the foot's top and the wall tangentially. (The half-round
/// boss is two quarter arcs and the foot is drawn where the part's patterned copy stood: a single
/// half arc names its joints alike, and a fillet after a pattern misses its edges n-ary, two
/// separate bugs.)
#[test]
fn drafted_rib_into_blend() {
    builds_valid(
        "extrude 95 rect -120 -65 240 130
extrude 44 xz at 62 87 95 loop line -42 0 42 0 arc 0 0 42 0 -90 arc 0 0 42 -90 -90
feet: extrude 16 loop line 52 -55 52 -85 arc 60 -85 8 180 90 line 60 -93 88 -93 arc 88 -85 8 270 90 line 96 -85 96 -55 line 96 -55 52 -55
fillet 5 (edges between feet f1 concave)
rib 8 draft 1 yz at 62 0 0 line 63 20 83 49
",
    );
}

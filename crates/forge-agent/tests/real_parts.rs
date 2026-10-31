//! Real parts that broke on drafted faces (draft tilts planes and turns rounds into cones) and
//! on lofts: each builds valid, one feature after another and all at once.

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

/// A tapered loft arm into a boss, filleted round the junction: the ball rolls on the loft's
/// face, offset along its normal like any other surface.
#[test]
fn fillet_loft_into_boss() {
    builds_valid(
        "arm: loft profile yz at 12 0 0 rrect -15 -12 30 24 6 profile yz at 166 0 0 rrect -9 -7.5 18 15 4
pedal: extrude sym 18 at 170 0 0 circle 0 0 12
fillet 1.2 (edges between arm pedal concave)
",
    );
}

/// A boss standing on a smooth three-profile loft, filleted round its foot.
#[test]
fn fillet_boss_into_smooth_loft() {
    builds_valid(
        "tube: loft smooth profile at 0 0 0 circle 0 0 10 profile at 0 5 30 circle 0 0 14 profile at 0 15 60 circle 0 0 18
boss: extrude 25 yz at 0 5 30 circle 0 0 4
fillet 2 (edges between boss tube concave)
",
    );
}

/// An internal thread in a spindle boss with a smooth loft arm just outside it: the thread's
/// faces meet the loft's region but not the loft (R13).
#[test]
fn thread_near_smooth_loft() {
    builds_valid(
        "arm: loft smooth profile yz at 12 0 0 rrect -15 -12 30 24 6 profile yz at 85 0 0 rrect -11.5 -9.5 23 19 5 profile yz at 162 0 0 rrect -9 -7.5 18 15 4
extrude sym 30 circle 0 0 21
hole 20.9 -xy at 0 0 15 7
thread M22x1 -xy at 0 0 15 int 6
",
    );
}

/// A cut whose floor and wall both cross a smooth spiral loft (a volute's chamber, R17).
#[test]
fn cut_across_smooth_spiral_loft() {
    builds_valid(
        "volute: loft smooth profile z -sin(60) cos(60) 0 x cos(60) sin(60) 0 at 87.17*cos(60) 87.17*sin(60) 0 circle 0 0 16.67 profile z -sin(110) cos(110) 0 x cos(110) sin(110) 0 at 89.81*cos(110) 89.81*sin(110) 0 circle 0 0 18.89 profile z -sin(160) cos(160) 0 x cos(160) sin(160) 0 at 92.44*cos(160) 92.44*sin(160) 0 circle 0 0 21.11 profile z -sin(210) cos(210) 0 x cos(210) sin(210) 0 at 95.08*cos(210) 95.08*sin(210) 0 circle 0 0 23.33 profile z -sin(260) cos(260) 0 x cos(260) sin(260) 0 at 97.72*cos(260) 97.72*sin(260) 0 circle 0 0 25.56 profile z -sin(310) cos(310) 0 x cos(310) sin(310) 0 at 100.4*cos(310) 100.4*sin(310) 0 circle 0 0 27.78 profile z -sin(360) cos(360) 0 x cos(360) sin(360) 0 at 103*cos(360) 103*sin(360) 0 circle 0 0 30 profile z 0 1 0 x 1 0 0 at 103 55 0 circle 0 0 33.5 profile z 0 1 0 x 1 0 0 at 103 110 0 circle 0 0 37
chamber: extrude cut 37 at 0 0 -15 circle 0 0 78
",
    );
}

/// A cylinder extruded on from a smooth loft's end circle: the loft meets the cylinder along
/// that circle, touching it where the loft stands straight up (R15).
#[test]
fn extrude_from_smooth_loft_end() {
    builds_valid(
        "loft smooth profile at 0 0 0 circle 0 0 10 profile at 0 5 30 circle 0 0 14 profile at 0 15 60 circle 0 0 18
extrude 40 at 0 15 60 circle 0 0 18
",
    );
}

/// A fillet where a half-round boss meets a 2 degree drafted wall: the springs are ellipses'
/// feet, and their speed bound held a whole turn's box, which holds the boss's axis (R5).
#[test]
fn fillet_boss_on_drafted_wall() {
    builds_valid(
        "box: extrude 95 rect -120 -65 240 130
draft -2 xy at 0 0 95 (faces of box normal +y)
boss: extrude 36 xz at -72 87 95 loop line -33.5 0 33.5 0 arc 0 0 33.5 0 -90 arc 0 0 33.5 -90 -90
fillet 2 (edges between boss box concave)
",
    );
}

/// A half-round boss across a flange, over a drafted and filleted corner: the corner's
/// ellipse rims on the top face stood across it as their own plane, so the boss's edge never
/// split there (R1).
#[test]
fn boss_through_drafted_fillet() {
    builds_valid(
        "box: extrude 95 rect -120 -65 240 130
dr: draft -2 xy at 0 0 95 (faces of box normal +x) (faces of box normal -x) (faces of box normal +y) (faces of box normal -y)
fillet 12 (edges of box shorter 100)
flange: extrude 14 -xy at 0 0 95 rect -142 -87 284 174
extrude sym 174 xz at -78 0 95 loop line -33.5 0 33.5 0 arc 0 0 33.5 0 -90 arc 0 0 33.5 -90 -90
",
    );
}

/// A shell of a plain loft between offset circles (an oblique cone, in two tangent halves):
/// the halves' seam vertices were pinned by the halves only once (R16).
#[test]
fn shell_oblique_loft() {
    builds_valid(
        "loft profile at 0 0 0 circle 0 0 10 profile at 0 5 30 circle 0 0 14
shell 2 (faces plane top)
",
    );
}

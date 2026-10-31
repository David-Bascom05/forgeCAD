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

/// Builds valid feature by feature.
fn seq_valid(src: &str) {
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let b = m.body().expect("a body");
    assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
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
/// panicked looking them up). They are dropped; the rest round. Feature by feature only: the
/// n-ary regen names the arcs apart (the n-ary naming work has that).
#[test]
fn fillet_picks_a_flush_seam() {
    seq_valid("L0: extrude 28.15 rect 0 0 97.81 38.36
L1: sweep add z 0 -1 0 x 1 0 0 at 48.91 31.45 29.19 rect -2.09 -2.09 4.18 4.18 path line 48.91 31.45 29.19 48.91 3.5500000000000007 29.19 arc z 0 0 1 x -1 0 0 at 55.92999999999999 3.5500000000000007 29.19 7.02 0 90 line 55.92999999999999 -3.469999999999999 29.19 75.58999999999999 -3.47 29.19
L2: pattern L1 lin 0.82 -0.58 0 4 10.8
L3: hole 9.06 z -1 0 0 at 68.63 -6.59 28.17 thru pt 118
L5: fillet 0.09..0.09 (edges circle of L1)
");
}

/// A twisted ruled loft joined to a block: its sections are quadrilaterals whose area is
/// quadratic in height, so Simpson's rule is exact. The mesh once gave 103240 mm^3.
#[test]
fn twisted_loft_on_a_block_volume() {
    let src = "extrude 15 rect 0 0 100 60
loft profile xy at 30 30 10 rect -15 -15 30 30 profile z 0 0 1 x 0.8775825618903728 0.479425538604203 0 at 30 30 40 rect -8 -8 16 16
";
    let (s, c) = forge_math::dmath::sin_cos(0.5);
    let area = |t: f64| {
        let p: Vec<(f64, f64)> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .map(|&(x, y)| {
                let (bx, by) = (8.0 * (c * x - s * y), 8.0 * (s * x + c * y));
                ((1.0 - t) * 15.0 * x + t * bx, (1.0 - t) * 15.0 * y + t * by)
            })
            .collect();
        0.5 * (0..4)
            .map(|i| p[i].0 * p[(i + 1) % 4].1 - p[(i + 1) % 4].0 * p[i].1)
            .sum::<f64>()
    };
    let t0 = 1.0 / 6.0;
    let want = 90000.0 + 30.0 * (1.0 - t0) / 6.0 * (area(t0) + 4.0 * area(0.5 * (t0 + 1.0)) + area(1.0));
    let m = Model::new(src);
    let q = forge_mesh::Quality {
        chord: 0.002,
        ..Default::default()
    };
    let nary = Part {
        features: m.features.clone(),
    }
    .regenerate_nary(1)
    .expect("n-ary");
    for b in [m.body().expect("a body").compact(), nary] {
        let v = forge_mesh::tessellate(&b, &q).expect("meshes").volume();
        assert!((v - want).abs() < 1.0, "{v} vs {want}");
    }
}

/// A rod cut across a pocket wall just above a variable fillet round the pocket floor: the rod's
/// circle on the wall crosses the fillet's spring curve, which lies where the wall and the blend
/// touch. It was cut against a plane square to the wall through the curve's middle, off the
/// curved spring line by 0.008 there, so no cut was found ("inconsistent classification"); the
/// spring curve's own fence (the rays from the ball's centre through it) finds it.
#[test]
fn cut_across_a_variable_fillet_spring_line() {
    let src = "L0: extrude 40 rect 0 0 160 110
L1: extrude cut 2 -xy at 80 55 40 rrect -30 -7 60 14 4
L2: fillet 0.6..0.9 (edges near 80 62 38) (edges near 110 55 38) (edges near 80 48 38) (edges near 50 55 38) 0.9..0.6 (edges near 108.83 60.83 38) (edges near 108.83 49.17 38) (edges near 51.17 49.17 38) (edges near 51.17 60.83 38)
L3: extrude cut 20 xz at 72 75 40.5 circle 0 0 2
";
    builds_valid(src);
    assert_eq!(judge(src), Verdict::Ok);
}

/// Two mirrored swept arcs 12 apart: each tube's torus face had the whole torus as its box, the
/// two boxes overlapped, and the pair (tori whose centre circles cross away from the faces) has
/// no closed form. A partial torus face's box now follows the arc its loops span.
#[test]
fn disjoint_swept_arcs_never_meet() {
    let m = Model::new(
        "s: sweep z 0 1 0 at 0 10 0 circle 0 0 4 path arc yz at 0 10 -100 100 90 -40\npattern s mirror xz\n",
    );
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
}

/// A sweep cut boring a bent tube in a general plane: the closed-form meet of the bore's and
/// the tube's tori found thousands of events and never returned.
#[test]
fn bent_tube_bore_in_a_general_plane() {
    builds_valid("sweep z -0.59048313866411 0.10148065235589 -0.800644328120049 at -130.647058563804 10 441.057078039369 circle 0 0 7 path line -130.647058563804 10 441.057078039369 -313.740381258896 41.4664866995883 192.798286663673 arc z 0.804799093005477 -6.93889390390723e-18 -0.593547319004611 x -0.58966414290279 0.114200861333259 -0.799533840337893 at -306.512352962495 160.846988255089 202.598871100508 120 -90.7330947118864 2.44323436805379
sweep cut z -0.59048313866411 0.10148065235589 -0.800644328120049 at -130.647058563804 10 441.057078039369 circle 0 0 6 path line -130.647058563804 10 441.057078039369 -313.740381258896 41.4664866995883 192.798286663673 arc z 0.804799093005477 -6.93889390390723e-18 -0.593547319004611 x -0.58966414290279 0.114200861333259 -0.799533840337893 at -306.512352962495 160.846988255089 202.598871100508 120 -90.7330947118864 2.44323436805379
");
}

/// A runner swept and bored along a bend through a flange, then the flange's face thickened
/// (B5): it hung in the bore like the bent tube, then left open edges. Feature by feature only:
/// the n-ary regen names the flange face apart (the n-ary naming work has that).
#[test]
fn thicken_after_a_bent_bore() {
    seq_valid("flange: extrude 12 yz at -12 0 0 rrect -60 -30 120 60 8
runner: sweep yz at -12 0 0 circle 0 0 19 path line -12 0 0 30 0 0 arc xz at 30 0 -60 60 90 -90 line 90 0 -60 90 0 -115
sweep cut yz at -12 0 0 circle 0 0 15.5 path line -12 0 0 30 0 0 arc xz at 30 0 -60 60 90 -90 line 90 0 -60 90 0 -130
thicken 1.5 (faces of flange normal -x)
");
}

/// A tilted pad over a through hole leaves a 1.6 mm2 lens of its side plane between its edge and
/// two arcs of the hole meeting at the cylinder's seam (M9). The body was right; check's chords of
/// the arcs cut into the thin lens past its probe (FaceWrongSide). A miss is now looked at again
/// with the loops drawn finely.
#[test]
fn thin_lens_at_a_hole_seam() {
    builds_valid("extrude new 15 rect 0 0 120 60
extrude cut 17 at 75 19 -1 circle 0 0 3
extrude add 8.945478516509796 z -0.061633258296996694 0.2939571355937309 0.9538294102748273 x -0.08391472378619189 0.9507366012975499 -0.29842626239168546 at 89.69521759175012 13.908448595830624 3.4058649537841124 rrect -13.632567574771072 -10.946070359275362 27.265135149542143 21.892140718550724 7.854519259083712
");
}

/// A ball centred on a block's edge with its poles on the edge (M5): the cuts on the sphere run
/// through a pole, where u says nothing, and the ways out were ordered by garbage directions.
/// They are ordered in the tangent plane there; both frames build valid alike.
#[test]
fn ball_with_its_poles_on_an_edge() {
    for frame in ["xz", "-xz"] {
        builds_valid(&format!(
            "extrude new 20 at -20 -20 -20 rect 0 0 40 40\nball add 5 {frame} at 20 0 0\n"
        ));
    }
}

/// A bent square sweep patterned so the copies overlap (r21): a new edge leaves an old vertex
/// along the tangent of the old arc beside it, and the face split took the wrong way there.
#[test]
fn new_edge_leaves_tangent_to_an_old_one() {
    builds_valid("L0: extrude 32.31 rrect 0 0 99.32 87.16 13.8
L2: sweep add z 0 -1 0 x 0 0 -1 at 80.59 80.34 -1.59 rect -3.19 -3.19 6.37 6.37 path line 80.59 80.34 -1.59 80.59 67.73 -1.59 arc z 1 0 0 x 0 0 1 at 80.59 67.73 -6.72 5.13 0 45 line 80.59 64.10254221251301 -3.0925422125130106 80.59 52.60498595041975 -14.590098474606272 extend 3.18 3.81
L4: pattern L2 lin 0.21 -0.98 0 2 8.64
");
}

/// Two histories that took minutes: seed 2's closed-form meet recounted every interval for each
/// sign flicker it dropped, and seed 24's trim subdivided a traced curve running a hair off a
/// spring curve's fence down to a millionth of its length. Both finish in seconds.
#[test]
fn slow_histories_finish() {
    builds_valid(
        r"L0: extrude 14.83 rect 0 0 117.88 47.1
L2: sweep cut z -1 0 0 x 0 1 0 at 31.54 30.49 7.89 rrect -2.54 -2.54 5.08 5.08 0.76 path line 31.54 30.49 7.89 20.22 30.49 7.89 arc z 0 0 -1 x 0 -1 0 at 20.22 36.18 7.89 5.69 0 60 line 15.292315452466543 33.335 7.89 6.2273154524665415 49.036040570611874 7.89
L4: ball cut 15.78 at 16.66 34.04 11.59
L5: plane z 0 0 1 at 58.94 24.91 35.19
L10: pattern L2 mirror z -0.03 1 0 at 58.85 27.94 0
L12: pattern L4 circ 73.75 10.87 0 0 0 1 4 32.1 step
L13: pattern L4 L10 lin 0.45 0.89 0 4 17.35 -0.89 0.45 0 3 17.35
",
    );
    let m = Model::new(
        r"L0: extrude 29.78 rect 0 0 109.97 42.04
L1: hole 7.43 z 0 0 -1 at 36.42 6.19 29.78 thru
L3: ball add 3.89 at 31.51 39.61 14.89
len0 = 8.4
L5: sweep cut z 0 1 0 x 0 0 -1 at 13.41 9.48 -1.65 rrect -3.3 -3.3 6.6 6.6 0.99 path line 13.41 9.48 -1.65 13.41 30.54 -1.65 arc z -1 0 0 x 0 0 1 at 13.41 30.54 -10.39 8.74 0 45 line 13.41 36.720113267570426 -4.209886732429575 13.41 57.66461612631596 -25.15438959117511
len1 = 8.24 mm
L8: extrude add 12.72 z 0.992 0.058 0.108 at 109.65 21.02 27.2 poly 8.82 1.04 2.69 8.81 -5.73 8.85 -9.56 -0.34 -4.67 -6.03 1.72 -7.68
L9: fillet 4.93..2.1 (edges near 54.99 42.04 0)
L10: sketch z 1 0 0 at 10.11 32.44 0.46
  rect -9.36 -6.24 18.72 12.36
  circle 0 0 2.81
  dia c1 4.51
  dist l1 len1*2.62
end
L11: extrude add 22.1 L10
L12: pattern body lin 0.98 0.19 0 2 13.21
",
    );
    // (its last line, a pattern of the whole body, still leaves open edges)
    assert!(m.body().is_some());
}

/// A draft of a face a variable fillet rounds into (r03): tilting it tore the tangent join and
/// the draft caught its own invalid body. It is refused: draft before rounding.
#[test]
fn draft_of_a_filleted_face() {
    refused(
        "L0: extrude 30.98 rect 0 0 146.94 108.69
L1: fillet 8.41..7.56 (edges near 73.47 0 0) (edges convex near 146.94 54.35 30.98)
L2: hole 8.1 z 0 -1 0 at 70.26 108.69 28.63 cb 13.51 6.07 thru pt 118
L3: fillet 1.24 (edges convex near 66.96 51.31 30.98) (edges near 73.56 51.31 30.98) (edges near 64.71 108.69 24.77)
L7: draft 1.57 xy at 0 0 -9.27 (faces plane near 146.94 54 16.7)
",
        "Tangent",
    );
}

/// A pattern of a pattern and its source (r23): the source's ball came in twice, once as itself
/// and once through the inner pattern, and two equal spheres cut in one pass left open edges.
#[test]
fn pattern_of_a_pattern_and_its_source() {
    builds_valid(
        "L0: extrude 29.9 rect 0 0 74.83 101.22
L1: ball cut 3.11 at 57.27 50.61 29.62
L2: pattern L1 lin 0.03 -1 0 4 7.72 1 0.03 0 3 7.72
L3: pattern L2 L1 lin 0.11 0.99 0 2 9.35
",
    );
    builds_valid(
        "L0: extrude 29.9 rect 0 0 74.83 101.22\nL1: ball cut 3.11 at 57.27 50.61 29.62\nL3: pattern L1 L1 lin 0.11 0.99 0 2 9.35\n",
    );
}

/// A round tube swept on past a bend in a square one it crosses (r28): the round tube's bend
/// (a torus) against the square tube's bend (a cylinder square to the torus's axis) came out
/// Empty, the pieces of the cylinder face were not split, and the classification flooded
/// across. The closed form now finds where two pairs of roots meet at once.
#[test]
fn tube_bend_across_a_square_bend() {
    builds_valid("L0: extrude 34.57 rect 0 0 127.51 107.86
L4: extrude cut thru z 0 0 1 at 61.39 24.53 -0.61 poly 15.26 2.4 5.49 12.01 -6.61 11.1 -18.54 2.18 -6.26 -16.88 9.22 -9.43
L6: sweep add z 0 -1 0 x 0 0 -1 at 60.44 47.57 17.29 rrect -6.63 -6.63 13.26 13.26 1.99 path line 60.44 47.57 17.29 60.44 19.68 17.29 arc z 1 0 0 x 0 0 1 at 60.44 19.68 -2.289999999999999 19.58 0 30 line 60.44 9.890000000000002 14.66677740609931 60.44 -8.357155257738123 4.131777406099312
L11: sweep add z 0 1 0 x 1 0 0 at 60.44 -13.66 26.6 circle 0 0 5.35 path line 60.44 -13.66 26.6 60.44 16.46 26.6 arc z 0 0 -1 x -1 0 0 at 85.87 16.46 26.6 25.43 0 45 line 67.8882745544261 34.4417254455739 26.6 84.75984235353712 51.31329324468493 26.6 extend 1.84 3.22
");
}

/// A countersunk hole on a face its edge's variable fillet rolls off (r10): the hole's top plane,
/// on that face, was traced against the blend, which only touches it along the spring curve; the
/// traced touch came out 1e-4 off the edge, a sliver piece split the face, and the flood failed.
/// A plane a blend rolls off along an edge only touches it there: the pair is skipped.
#[test]
fn countersink_beside_a_variable_fillet() {
    builds_valid(
        "L0: extrude 12.43 rect 0 0 69.44 92.38
L1: fillet 1.99..3.51 (edges line of L0)
L3: hole 8.39 z 0 -1 0 at 34.85 92.38 3.5 cs 15.68 82 thru pt 118 thread 10.07 1.46
",
    );
}

/// The same ball cut twice: in one n-ary pass the two equal spheres met everywhere and left open
/// edges. A tool equal to an earlier one combined the same way is left out of the pass.
#[test]
fn the_same_cut_twice() {
    builds_valid(
        "extrude 29.9 rect 0 0 74.83 101.22\nball cut 3.11 at 57.27 50.61 29.62\nball cut 3.11 at 57.27 50.61 29.62\n",
    );
    builds_valid(
        "extrude 20 rect 0 0 40 30\nextrude cut 5 at 10 10 20 circle 0 0 3\nextrude cut 5 at 10 10 20 circle 0 0 3\nextrude add 4 at 30 20 20 rect 0 0 5 5\nextrude add 4 at 30 20 20 rect 0 0 5 5\n",
    );
}

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

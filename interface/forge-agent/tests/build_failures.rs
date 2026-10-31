//! Parts the kernel used to refuse though they can be built (research/build-failures), each valid
//! feature by feature and n-ary, the two paths the same solid.
use forge_agent::model::Model;
use forge_mesh::{Quality, tessellate};
use forge_model::Part;

fn volume(b: &forge_topo::Body) -> f64 {
    let q = Quality {
        chord: 0.01,
        ..Quality::default()
    };
    tessellate(&b.compact(), &q).expect("meshes").volume()
}

/// Builds valid on both paths with the same volume, which it returns.
fn builds(src: &str) -> f64 {
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let b = m.body().expect("a body");
    assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
    let part = Part {
        features: m.features.clone(),
    };
    let n = part.regenerate_nary(1).expect("n-ary");
    assert!(forge_topo::check(&n).is_empty(), "n-ary: {:?}", forge_topo::check(&n));
    let (vs, vn) = (volume(b), volume(&n));
    assert!((vs - vn).abs() < 1e-3 * vs, "seq {vs}, n-ary {vn}");
    vs
}

/// Two balls touching at a point on a block's top, a third over the point: the top's edges meet
/// there along one tangent, and the way on from one is told from the way straight through by
/// how they bend (adversarial ball_poles: "face split crossed itself").
#[test]
fn ball_over_where_two_balls_touch() {
    for ops in [["add", "add", "add"], ["add", "cut", "add"], ["cut", "cut", "cut"], ["cut", "add", "cut"]] {
        builds(&format!(
            "extrude 10 rect 0 0 60 40
ball {} 4 xz at 30 15 10
ball {} 4 xz at 30 23 10
ball {} 2 xz at 30 19 10
",
            ops[0], ops[1], ops[2]
        ));
    }
}

/// A rod lying on a cap touches it along a line where the one probe of an untouched face landed:
/// other points of the face say which side it is on (degenerate perpendicular_on_cap).
#[test]
fn a_probe_on_a_touch_asks_again() {
    let plain = builds("extrude 20 circle 0 0 10\n");
    let v = builds(
        "extrude 20 circle 0 0 10
extrude cut 40 yz at -20 0 25 circle 0 0 5
",
    );
    assert!((v - plain).abs() < 1e-2 * plain, "the cut touches and takes nothing");
}

/// Up to the next face past a fillet, chamfer or face edit: the pick looks at the part as it
/// stands, here a hole down to a rounded block's floor and a rod inside a chamfered one.
#[test]
fn next_face_after_a_fillet() {
    let drilled = builds(
        "extrude 20 rect 0 0 60 40
fillet 2 (edges along +x top)
hole 6 -xy at 30 20 20 next
",
    );
    let block = builds(
        "extrude 20 rect 0 0 60 40
fillet 2 (edges along +x top)
",
    );
    let bore = forge_math::dmath::PI * 9.0 * 20.0;
    assert!((block - drilled - bore).abs() < 1e-2 * bore, "{block} {drilled} {bore}");
    builds(
        "extrude 20 rect 0 0 60 40
chamfer 2 (edges along +x top)
extrude add next -xy at 30 20 10 circle 0 0 3
",
    );
}

/// Cylinders of one radius butted end to end, built from the joint outwards: the joint circle is
/// one closed edge the two run round opposite ways, which its ends cannot say (one reversed use
/// made an open shell, or a false touch). The commonest refusal on real parts.
#[test]
fn butted_cylinders_of_one_radius() {
    let pi = forge_math::dmath::PI;
    let v = builds(
        "extrude 12 z 0 1 0 x 1 0 0 circle 0 0 6
extrude add 30 z 0 -1 0 x 1 0 0 circle 0 0 6
",
    );
    assert!((v - pi * 36.0 * 42.0).abs() < 1e-2 * v, "{v}");
    let v = builds(
        "extrude 9 z 0 -1 0 x 1 0 0 circle 0 0 11 circle 0 0 8
extrude add 5 z 0 1 0 x 1 0 0 circle 0 0 11
",
    );
    assert!((v - pi * (121.0 * 14.0 - 64.0 * 9.0)).abs() < 1e-2 * v, "{v}");
}

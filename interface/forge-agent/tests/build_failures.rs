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
    assert!(
        forge_topo::check(&n).is_empty(),
        "n-ary: {:?}",
        forge_topo::check(&n)
    );
    let (vs, vn) = (volume(b), volume(&n));
    assert!((vs - vn).abs() < 1e-3 * vs, "seq {vs}, n-ary {vn}");
    vs
}

/// Two balls touching at a point on a block's top, a third over the point: the top's edges meet
/// there along one tangent, and the way on from one is told from the way straight through by
/// how they bend (adversarial ball_poles: "face split crossed itself").
#[test]
fn ball_over_where_two_balls_touch() {
    for ops in [
        ["add", "add", "add"],
        ["add", "cut", "add"],
        ["cut", "cut", "cut"],
        ["cut", "add", "cut"],
    ] {
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
    assert!(
        (v - plain).abs() < 1e-2 * plain,
        "the cut touches and takes nothing"
    );
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
    assert!(
        (block - drilled - bore).abs() < 1e-2 * bore,
        "{block} {drilled} {bore}"
    );
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

/// A tool's edge a hair clear of a wall makes no cut on the curve passing it, and that curve's
/// middle lands on the edge's face boundary: other samples show it is a new edge, not one along
/// a boundary. The cut did nothing, silently; 2e-7 is what a model rounded to float32 carries.
#[test]
fn a_tool_edge_a_hair_off_the_wall_still_cuts() {
    let part = |x: f64| {
        format!(
            "extrude 30 z 0 -1 0 x 1 0 0 circle 0 0 15
extrude cut two 35 35 z 1 0 0 x 0 0 -1 poly -1 -39 0 -30 {x} -4 22 -14 22 -37
"
        )
    };
    let (exact, noisy) = (builds(&part(15.0)), builds(&part(15.0000002)));
    assert!((exact - noisy).abs() < 1e-3 * exact, "{exact} {noisy}");
    assert!(
        exact < 0.95 * forge_math::dmath::PI * 225.0 * 30.0,
        "the cut took something: {exact}"
    );
}

/// A pocket whose edge lies on the sharp edge a rib meets a plate along: the part's two faces there
/// are one inside the tool and one outside, so no class floods across that edge.
#[test]
fn a_tool_edge_on_a_part_edge() {
    let v = builds(
        "extrude 5 z 0 1 0 x 1 0 0 poly 0 -150 0 0 200 0 200 -150
extrude add 200 z 1 0 0 x 0 0 -1 poly 0 -80 -100 0 0 0
extrude cut 90 z 0 -1 0 x 1 0 0 poly 190 100 190 10 10 10 10 100
",
    );
    // plate + rib of triangular section, less the pocket's slice of the rib
    let slice = 80.0 * (90.0 - (100.0 * 100.0 - 10.0 * 10.0) / 200.0);
    let want = 200.0 * 150.0 * 5.0 + 200.0 * 4000.0 - 180.0 * slice;
    assert!((v - want).abs() < 1e-3 * want, "{v} {want}");
}

/// Faces flush to within tolerance though tilted past the angle tolerance, and a tool's flat end
/// tangent to a rod: neither may be taken for a crossing, whose edges the tolerance cannot tell
/// apart.
#[test]
fn nearly_flush_faces() {
    // a triangle's side within 1e-7 of tangent to a tube, its corner on the wall
    builds(
        "extrude 150 z 0 1 0 x 1 0 0 circle 0 0 8 circle 0 0 6
extrude cut two 9 9 z 0 0 1 x 1 0 0 poly 7.99999994 150.4 -7 150 8 138
",
    );
    // a cross hole ending on the plane tangent to the rod
    let v = builds(
        "extrude 80 z 1 0 0 x 0 0 -1 circle 0 0 40
extrude cut two 40 50 z 0 1 0 x 1 0 0 circle 30 0 10
",
    );
    let rod = forge_math::dmath::PI * 1600.0 * 80.0;
    assert!(v < rod && v > 0.9 * rod, "{v}");
}

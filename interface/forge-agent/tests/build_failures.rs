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

/// A ball over where two balls touch on a block's top, a third to the side: the top's edges meet
/// there along one tangent, and the way on from one is told from the way straight through by how
/// they bend (adversarial ball_poles: "face split crossed itself"). Where the two alone leave
/// a solid pinched at the point it is refused, at that line.
#[test]
fn ball_over_where_two_balls_touch() {
    for ops in [
        ["add", "add", "add"],
        ["add", "cut", "add"],
        ["cut", "cut", "cut"],
        ["cut", "add", "cut"],
    ] {
        let src = format!(
            "extrude 10 rect 0 0 60 40
ball {} 4 xz at 30 15 10
ball {} 4 xz at 30 23 10
ball {} 2 xz at 30 19 10
",
            ops[0], ops[1], ops[2]
        );
        let m = Model::new(&src);
        match &m.error {
            Some(e) => assert!(e.msg.contains("ZeroThickness") && e.at == 2, "{}", e.msg),
            None => {
                builds(&src);
            }
        }
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

/// A flat cut on a rod from its end by a profile whose arc lies on the rod's own circle: the flat's
/// two ends take their names from what meets there, alike; they are told apart by where they lie.
#[test]
fn flat_on_a_rod_by_an_arc_on_its_circle() {
    builds(
        "extrude two 15 15 z 0 1 0 x 1 0 0 circle 0 0 6
extrude cut 7 z 0 -1 0 x -1 0 0 at 0 15 0 loop line -2 5.656854249 -2 -5.656854249 arc 0 0 6 -109.471220634 -141.057558732
",
    );
}

/// A tool's side all but tangent to a round floor's edge, the floor's wall a hair wider than the
/// tool reaches: the imprint runs a hair from the boundary between the same vertices, and is it.
#[test]
fn imprint_a_hair_from_the_boundary() {
    builds(
        "extrude 4 z 0 1 0 x 1 0 0 circle 0 0 7
extrude cut 3 z 0 1 0 x 1 0 0 circle 0 0 5.00000006
extrude add 4 z 1 0 0 x 0 0 -1 at 5 0 0 poly -4 2.5 -4 0 4 0 4 2.5
",
    );
}

/// A cone that comes to a point stays one when shelled towards its axis: its apex moves along the
/// axis. Refused at any thickness before, for a boss's tip or a drill point shelled outward.
#[test]
fn shell_towards_a_cone_point() {
    let v = builds(
        "extrude 30 rect 0 0 80 80
revolve add xz at 40 40 30 axis y poly 0 0 8 0 0 10
shell 2 (faces plane normal -z bottom)
",
    );
    let half = forge_math::dmath::atan2(8.0, 10.0);
    let h = 40.0 - 2.0 / forge_math::dmath::sin(half) - 28.0;
    let r = h * forge_math::dmath::sin(half) / forge_math::dmath::cos(half);
    let pi = forge_math::dmath::PI;
    let want = 192000.0 + pi * 640.0 / 3.0 - (76.0 * 76.0 * 28.0 + pi * r * r * h / 3.0);
    assert!((v - want).abs() < 1e-3 * want, "{v} {want}");
    builds(
        "extrude 30 rect 0 0 80 80
hole 6 -xy at 40 40 30 15 pt 118
shell 2 out (faces plane normal +z top)
",
    );
}

/// Balls whose poles touch on a block's top leave it pinched at the point: refused the same way
/// exact and a hair of noise off, not built invalid (handed over by bugs-5; ulp on ball_poles).
#[test]
fn balls_touching_at_poles_are_refused() {
    let part = |y: f64| {
        format!(
            "extrude new 10 rect 0 0 60 40
ball cut 3.6 -xz at 17.98 23.83 10
ball add 3.6 -xz at 17.98 {y} 10
"
        )
    };
    for y in [31.03, 31.03 + 1e-12, 31.03 - 1e-12] {
        let e = Model::new(&part(y)).error;
        assert!(e.is_some_and(|e| e.msg.contains("ZeroThickness")), "y {y}");
    }
}

/// One solid pinched cannot be made: the refusal says where, and the line that made it.
#[test]
fn pinches_say_where() {
    let refused = |src: &str| Model::new(src).error.expect("refused");
    let e = refused("extrude 5 circle 0 0 40 circle 0 -30 10\n");
    assert!(
        e.msg.contains("ZeroThickness") && e.msg.contains("-40") && e.at == 0,
        "{}",
        e.msg
    );
    let e = refused(
        "extrude 10 rect 0 0 20 20
extrude add 10 at 20 0 0 rect 0 20 20 20
",
    );
    assert!(
        e.msg.contains("ZeroThickness") && e.msg.contains("(20") && e.at == 1,
        "{}",
        e.msg
    );
}

/// Loops that touch are for the result to answer. A cut by such a profile whose touch lands in
/// empty space, here beyond the plate, leaves a sound part; a plate whose hole touches its outline
/// is refused above, and so is a boss whose outline pinches.
#[test]
fn a_touching_profile_is_judged_by_its_result() {
    let pi = forge_math::dmath::PI;
    // the touch at (0, -30), the plate ends at y = -25
    let v = builds(
        "extrude 4 rect -60 -25 120 80
extrude cut 6 at 0 0 -1 circle 0 0 30 circle 0 -20 10
",
    );
    let lens = pi * 900.0 - pi * 100.0;
    assert!(
        v < 120.0 * 80.0 * 4.0 && v > 120.0 * 80.0 * 4.0 - lens * 4.0,
        "{v}"
    );
    // an island of the plate left touching the rest along a line: separate solids, sound
    builds(
        "extrude 4 rect -60 -60 120 120
extrude cut 6 at 0 0 -1 circle 0 0 30 circle 0 -20 10
",
    );
    // the same outline stood on the plate: one solid, joined below, meeting itself along the line
    assert!(
        Model::new(
            "extrude 4 rect -60 -60 120 120
extrude add 6 at 0 0 4 circle 0 0 30 circle 0 -20 10
"
        )
        .error
        .is_some_and(|e| e.msg.contains("ZeroThickness") && e.at == 1)
    );
}

/// A cut taking one whole body of several deletes it and says so, on both paths; a cut taking every
/// body leaves no part, and is refused.
#[test]
fn a_cut_taking_a_whole_body() {
    let src = "a: extrude 8 rect 0 0 10 10
b: extrude new 8 rect 40 0 10 10
extrude cut 10 at 0 0 -1 rect -1 -1 12 12 in all
";
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    assert_eq!(m.removed_bodies(), vec![(3, 1)]);
    let part = Part {
        features: m.features.clone(),
    };
    assert_eq!(part.regenerate_solids().expect("seq").len(), 1);
    let solids = part.regenerate_nary_solids(1).expect("n-ary");
    assert_eq!(solids.len(), 1);
    assert_eq!(solids[0].id, 1);
    let every = Model::new(
        "a: extrude 8 rect 0 0 10 10
b: extrude new 8 rect 40 0 10 10
extrude cut 10 at 0 0 -1 rect -1 -1 60 12 in all
",
    );
    assert!(every.error.is_some_and(|e| e.msg.contains("leaves nothing")));
}

/// A boss near a rod's end whose fillet runs off over the rim and back: the rim is split, the
/// blend cuts the cap across the stretch that goes, and the wall's loop runs on through the rim's
/// closed vertex (volume agrees with OCCT to 1e-5).
#[test]
fn fillet_runs_over_a_rim_and_back() {
    let src = "extrude 51.4445 z 0 0 1 x 1 0 0 at 0 0 0 circle 0 0 17.7138
extrude add 35.7306 z 0.249995868143 0.880986519465 0.401702400332 x 0 0.41487599798 -0.909877962311 at 2.42676623008 -0.570108876966 28.6203986907 circle 0 0 11.5667
fillet 2.13853177 (edges concave)
";
    builds(src);
    let v = forge_topo::mass::mass(Model::new(src).body().expect("a body"))
        .expect("mass")
        .volume;
    assert!((v - 58260.8).abs() < 2.0, "{v}");
}

/// Taking every wall of a disc leaves its ends bounding nothing: refused, not a body of faces with
/// no loops.
#[test]
fn deleting_every_wall_is_refused() {
    let e = Model::new("extrude 20 circle 0 0 20 circle 0 0 12\ndelete (faces cyl)\n").error;
    assert!(e.is_some_and(|e| e.msg.contains("CannotHeal")));
}

/// A cone with a hexagon cut through it, its top edges rounded: a clear refusal, not a panic.
#[test]
fn rounding_a_hex_cut_cone_top_does_not_panic() {
    let m = Model::new(
        "revolve xz axis y poly 0 0 20 0 10 20 0 20
extrude cut thru -xy at 0 0 200 poly 20 0 10 17.320508 -10 17.320508 -20 0 -10 -17.320508 10 -17.320508
fillet 0.5 (edges top)
",
    );
    // (it builds since corner patches stopped following storage order; a refusal must name the fillet)
    assert!(m.error.is_none_or(|e| e.msg.contains("fillet")));
}

/// A slot through a plate whose straight side is two lines a hair off one line, as float noise
/// leaves it: where the plate's face crosses them, the neighbour across their edge meets at so
/// shallow an angle that the crossing was lost. The edge's own plane finds it.
#[test]
fn a_kink_of_a_hair_in_a_slots_side() {
    builds(
        "extrude new two 2.4 2.4 z 0 1 0 x 1 0 0 at 0 0 0 rect -20 0 40 40
extrude cut 12 z 0 -1 0 x 1 0 0 at 0 -1 0 loop line 2.5 -34 2.499985 -21.5 line 2.499985 -21.5 2.5 -9 arc 0 -9 2.5 0 180 line -2.5 -9 -2.5 -21.5 line -2.5 -21.5 -2.5 -34 arc 0 -34 2.5 180 180 in all
",
    );
}

/// A tube with a triangular prism cut, its slanted face passing exactly through the tube's far rim:
/// the ellipse where it meets the wall only touches the rim there, one cut on a closed curve, and
/// the whole loop was taken to run along the boundary. The cut lost a tenth of itself, silently.
#[test]
fn an_ellipse_touching_a_rim_still_cuts() {
    let src = "extrude 150 z 0 1 0 x 1 0 0 circle 0 0 8 circle 0 0 6
extrude cut two 9 9 z 0 0 1 x 1 0 0 poly 8 150 -8 150 8 135
";
    let v = builds(src);
    let exact = forge_topo::mass::mass(Model::new(src).body().expect("a body"))
        .expect("mass")
        .volume;
    assert!((v - exact).abs() < 1e-2 * exact, "{v} {exact}");
    // the same a hair off, where the ellipse crosses the rim
    let near = forge_topo::mass::mass(
        Model::new(
            "extrude 150 z 0 1 0 x 1 0 0 circle 0 0 8 circle 0 0 6
extrude cut two 9 9 z 0 0 1 x 1 0 0 poly 8.0000002 150.0000002 -8.0000002 150.0000002 8.0000002 135
",
        )
        .body()
        .expect("a body"),
    )
    .expect("mass")
    .volume;
    assert!((near - exact).abs() < 1e-3 * exact, "{near} {exact}");
}

/// A ball whose equator lies on a boss's top rim, its lower half inside: a dome on the boss, smooth
/// at the rim, one valid solid. Main refused it as "no closed solid" over the boss's foot.
#[test]
fn a_ball_tangent_to_a_boss_rim_is_a_dome() {
    let v = builds("extrude 8 circle 0 0 10\nextrude add 8 xy at 0 0 8 circle 0 0 4\nball add 4 xy at 0 0 16\n");
    let pi = std::f64::consts::PI;
    let want = 800.0 * pi + 128.0 * pi + 2.0 / 3.0 * pi * 64.0;
    assert!((v - want).abs() < 1e-6 * want, "{v} vs {want}");
}

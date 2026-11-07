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
    assert!((v - want).abs() < 3e-3 * want, "{v} {want}");
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
    assert!((v - want).abs() < 3e-3 * want, "{v} {want}");
    builds(
        "extrude 30 rect 0 0 80 80
hole 6 -xy at 40 40 30 15 pt 118
shell 2 out (faces plane normal +z top)
",
    );
}

/// Balls whose poles touch on a block's top, a third over the touch or beside it: the pinch where
/// two circles touch is told apart by how they bend, on both paths (handed over by bugs-5).
#[test]
fn balls_touching_at_poles_on_a_top_face() {
    builds(
        "extrude new 10 rect 0 0 60 40
ball cut 4.1 -xz at 33.3 25.3 10
ball add 4.1 -xz at 33.3 33.5 10
ball add 5.1 -xz at 30.7 12.4 10
ball cut 5.1 -xz at 30.7 22.6 10
",
    );
    builds(
        "extrude new 10 rect 0 0 60 40
ball cut 4.7742110386764 -xz at 36.529760196271496 12.25756041565283 10
ball add 4.7742110386764 -xz at 36.529760196271496 21.80598249300563 10
ball add 3.71 -xz at 32.83 24.37 10
ball add 3.71 -xz at 32.83 31.8 10
",
    );
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

/// A cone with a hexagon cut through it, its top edges rounded: no panic, and sound if it builds.
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
    let v =
        builds("extrude 8 circle 0 0 10\nextrude add 8 xy at 0 0 8 circle 0 0 4\nball add 4 xy at 0 0 16\n");
    let pi = std::f64::consts::PI;
    let want = 800.0 * pi + 128.0 * pi + 2.0 / 3.0 * pi * 64.0;
    assert!((v - want).abs() < 2e-3 * want, "{v} vs {want}");
}

/// The same touch ulps off: nothing round the point can say, and a face came out on its wrong side.
/// Valid or refused where it lies, never built invalid; the exact one builds.
#[test]
fn balls_touching_a_hair_off_are_never_invalid() {
    let part = |y: f64| {
        format!(
            "extrude new 10 rect 0 0 60 40
ball cut 3.6 -xz at 17.98 23.83 10
ball add 3.6 -xz at 17.98 {y} 10
"
        )
    };
    builds(&part(31.03));
    for y in [31.03 + 1e-12, 31.03 - 1e-12, 31.03 + 3e-13, 31.03 - 3e-13] {
        let m = Model::new(&part(y));
        match &m.error {
            Some(e) => assert!(
                e.msg.contains("ZeroThickness") || e.msg.contains("thick"),
                "y {y}: {}",
                e.msg
            ),
            None => assert!(forge_topo::check(m.body().unwrap()).is_empty(), "y {y}"),
        }
    }
}

/// A full round whose side face keeps edges it no longer holds: refused, not a body 5 off its face
/// (repros/check-edge-off-face.fc).
#[test]
fn full_round_leaving_an_edge_off_its_face_is_refused() {
    let m = Model::new(
        "L0: extrude 8.52 rect 0 0 53.78 114.36
L1: split plane z 0.951 0.258 0.172 at 37.9 60.16 6.25 +
L2: movebody by 35.98 2.3 -10.61 rot 38.28 57.24 4.26 0 0 1 42.91
L3: fillet 3.04..2.35 from (verts near 35.53 135.81 -10.61) (edges near 61.7 86.03 -10.61) (edges convex near 24.74 125.78 -10.61)
L5: movebody by 158.74 119.01 26.09
L8: sweep cut z 0 0 -1 x -1 0 0 at 221.81 208.74 99.68 rrect -12.99 -12.99 25.98 25.98 3.9 path line 221.81 208.74 99.68 221.81 208.74 10.52000000000001 arc z 0 1 0 x 1 0 0 at 192.41 208.74 10.52000000000001 29.4 0 30 line 217.87114687126248 208.74 -4.179999999999987 186.24614687126248 208.74 -58.956106789365734
len0 = 13.04
L11: extrude add 78.13 z 0.826 0.52 0.217 at 263.16 180.65 20.27 loop line -30.04 0 30.04 0 arc 0 0 30.04 0 180
L12: fillet 0.06 (edges convex circle of L8)
L17: offset -4.58 (faces near 209.64 238.28 19.73)
L19: hole 21.99 z 0 0 -1 at 162.07 252.77 37.23 34.89 pt 118 thread 26.39 3.5
L25: fullround (faces near 195.14 231.14 24) (faces near 204.84 236.72 19.73) (faces near 198.43 232.31 15.48)
",
    );
    let e = m.error.as_ref().expect("refused");
    assert!(e.msg.contains("full round"), "{}", e.msg);
}

/// Rays past a lemon must not count the apple its quartic also holds: a point in a corner of the
/// box was inside (WHUCAD 100003006, one point in 13).
#[test]
fn classify_beside_a_lemon() {
    let a = forge_math::dmath::acos(5.0 / 13.0) * 180.0 / forge_math::dmath::PI;
    let m = Model::new(&format!(
        "revolve xz axis y loop arc -5 0 13 {} {} line 0 12 0 -12\n",
        -a,
        2.0 * a
    ));
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let body = m.body().expect("a body").compact();
    let ix = forge_bool::Index::new(&body);
    for x in [6.5, 7.0, 7.5] {
        for y in [-5.0, 0.0, 5.0] {
            let p = forge_math::Point3::new(x, y, x);
            assert_eq!(ix.classify(p), forge_bool::Where::Outside, "{p:?}");
        }
    }
    let p = forge_math::Point3::new(5.0, 1.0, 0.5);
    assert_eq!(ix.classify(p), forge_bool::Where::Inside);
}

/// A revolved arc ending on the axis, its centre off it: the tube of a torus reaching through its
/// axis, centre on the arc's side (an apple) or across (a lemon, once built as a sphere, invalid).
/// A boolean through its apex is refused for now: a branch may change sheets there.
#[test]
fn arcs_ending_on_the_axis() {
    let mass = |src: &str| {
        let m = Model::new(src);
        assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
        let b = m.body().expect("a body");
        assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
        forge_topo::mass::mass(&b.compact()).expect("a mass")
    };
    // by hand: pi times the integral of r^2 dz along the arc
    let revolved = |rc: f64, a: f64| {
        let n = 200_000;
        let (mut v, mut area) = (0.0, 0.0);
        let pi = forge_math::dmath::PI;
        for k in 0..n {
            let t = -a + 2.0 * a * (k as f64 + 0.5) / n as f64;
            let (r, h) = (rc + 13.0 * forge_math::dmath::cos(t), 2.0 * a / n as f64);
            v += pi * r * r * 13.0 * forge_math::dmath::cos(t) * h;
            area += 2.0 * pi * r * 13.0 * h;
        }
        (v, area)
    };
    let a = forge_math::dmath::acos(5.0 / 13.0);
    let deg = a * 180.0 / forge_math::dmath::PI;
    for (src, rc, half) in [
        (
            format!(
                "revolve xz axis y loop arc -5 0 13 {} {} line 0 12 0 -12\n",
                -deg,
                2.0 * deg
            ),
            -5.0,
            a,
        ),
        (
            format!(
                "revolve xz axis y loop arc 5 0 13 {} {} line 0 12 0 -12\n",
                deg - 180.0,
                2.0 * (180.0 - deg)
            ),
            5.0,
            std::f64::consts::PI - a,
        ),
    ] {
        let m = mass(&src);
        let (v, area) = revolved(rc, half);
        assert!((m.volume - v).abs() < 1e-5 * v, "{} vs {v}", m.volume);
        assert!((m.area - area).abs() < 1e-5 * area, "{} vs {area}", m.area);
        // a cut along the axis builds now, judged by the definition in spindle_cuts.rs
        builds(&format!("{src}extrude cut thru xy at 0 0 -30 circle 3 0 2\n"));
    }
    // a dome on a flange, the arc's centre off the axis but not reaching it
    mass(
        "revolve xz axis y loop line 0 -3 15 -3 line 15 -3 15 0 arc 2 0 13 0 90 line 2 13 0 13 line 0 13 0 -3\n",
    );
    // a whole circle tangent to the axis would pinch there
    let e = Model::new("revolve xz axis y circle 4 0 4\n").error;
    assert!(e.is_some_and(|e| e.msg.contains("pinches")));
    // a profile whose corner only touches it builds if what it makes closes, or is refused
    let m = Model::new("revolve xz axis y loop line 0 0 6 0 line 6 0 6 2 line 6 2 0 0\n");
    match (&m.error, m.body()) {
        (Some(e), _) => assert!(e.msg.contains("pinches"), "{}", e.msg),
        (None, Some(b)) => assert!(forge_topo::check(b).is_empty()),
        _ => panic!("neither"),
    }
    // as a cut it is a tool, and what it leaves is sound: a cone standing on the flat (WHUCAD 100002292)
    let v = builds(
        "extrude 4 rect -10 -10 20 20\nrevolve cut xz axis y loop line 0 0 6 0 line 6 0 6 2 line 6 2 0 0\n",
    );
    assert!((v - (1600.0 - 48.0 * std::f64::consts::PI)).abs() < 2.0, "{v}");
    // not on part of a turn: a wedge's point
    builds("revolve 90 xz axis y loop line 0 0 6 0 line 6 0 6 2 line 6 2 0 0\n");
}

/// Points along the axis between axis lines are on nothing a whole turn makes: they were left as
/// loose vertices.
#[test]
fn a_point_between_two_lines_on_the_axis() {
    builds("revolve xz axis y loop line 0 0 5 0 line 5 0 5 3 line 5 3 0 3 line 0 3 0 1 line 0 1 0 0\n");
}

/// A cut through everything reaches past all the material there can be: lofts, sweeps and coils
/// were left out of that, so it went a millimetre into them, valid and wrong.
#[test]
fn a_cut_through_a_loft_or_a_sweep_goes_through() {
    let pi = std::f64::consts::PI;
    let frustum = 15.0 * (40.0f64 * 40.0 + 40.0 * 16.0 + 16.0 * 16.0) / 3.0;
    let v = builds(
        "loft smooth profile z 0 0 1 x 1 0 0 at 0 0 0 rect -20 -20 40 40 profile z 0 0 1 x 1 0 0 at 0 0 15 rect -8 -8 16 16
extrude cut thru z 0 0 -1 x 1 0 0 at 0 0 15 circle 0 0 4
",
    );
    let want = frustum - pi * 16.0 * 15.0;
    assert!((v - want).abs() < 2e-3 * want, "loft {v} vs {want}");
    let v = builds(
        "sweep z 0 0 1 x 1 0 0 at 0 0 0 rect -20 -20 40 40 path line 0 0 0 0 0 15
hole 8 z 0 0 -1 x 1 0 0 at 0 0 15 thru
",
    );
    let want = 1600.0 * 15.0 - pi * 16.0 * 15.0;
    assert!((v - want).abs() < 2e-3 * want, "sweep {v} vs {want}");
}

/// The box an arc in a tilted plane reports is the arc's own, not the bound of it: a sweep over a
/// long arc read 0.85 too tall (volume and area right all along).
#[test]
fn the_box_of_a_tilted_arc_is_tight() {
    let mut m = Model::new(
        "sweep z 0 -1 0 x 1 0 0 at 0 0 0 circle 0 0 10 path line 0 0 0 0 40 0 arc z 1 0 0 x 0 0 -1 at 0 40 45 45 0 40 arc z -1 0 0 x 0 -0.642787609687 0.766044443119 at 0 88.20907072649 -12.453333233923 30 0 80 arc z 1 0 0 x 0 -0.642787609687 -0.766044443119 at 0 396.747123376029 355.247999463186 450 0 12 line 0 185.484920122378 -42.078417323331 0 198.729134015262 -49.120490765119
",
    );
    assert!(m.error.is_none());
    // the second arc's centre, up its radius, and the profile's
    let top = -12.453333233923 + 30.0 + 10.0;
    let z = m.stats().2.z.hi;
    assert!((z - top).abs() < 0.03, "{z} vs {top}");
}

/// Equal crossing holes, the second's wall tangent to the first's cap: the curve touches the cap's rim
/// a few 1e-15 off it, so the closed form saw no root, and the rim got no vertex (FaceWrongSide on the
/// wall, on a body returned as good).
#[test]
fn a_conic_touching_a_rim_within_rounding_gets_a_vertex() {
    let pi = std::f64::consts::PI;
    let v = builds(
        "extrude 14 z 0 -1 0 x 1 0 0 at 0 0 0 rect -8 -5 16 10
extrude cut 10.5 z 0 1 0 x 1 0 0 at 0 -14 0 circle 0 0 3.5
extrude cut 12.15 z 1 0 0 x 0 -1 0 at -9 0 0 circle 7 0 3.5
",
    );
    assert!(v < 16.0 * 10.0 * 14.0 - pi * 3.5 * 3.5 * 10.5, "{v}");
    // a tab whose end edge lies tangent to a rod: the circle touches the tab's slanted faces
    builds(
        "extrude sym 60 z 0 -1 0 x 1 0 0 at 0 0 0 circle 0 0 2
extrude add sym 4 z 1 0 0 x 0 1 0 at 0 0 0 loop line -20 2 -48.7059047745 4.8296291314 arc -50 0 5 75 210 line -48.7059047745 -4.8296291314 -20 -2 line -20 -2 -20 2
",
    );
}

/// A body of several shells: what a boolean added all went to the first, whichever shell it joined.
/// The solid was right and the shells wrong (ShellEuler on a void's pocket, ShellsOverlap on a lump).
#[test]
fn what_a_boolean_adds_joins_its_own_shell() {
    let pi = std::f64::consts::PI;
    // a pocket in the floor of a void
    let v = builds(
        "extrude 40 z 0 0 1 x 1 0 0 at 0 0 0 circle 0 0 20
extrude cut 20 z 0 0 1 x 1 0 0 at 0 0 10 circle 0 0 15
extrude cut 10 z 0 0 1 x 1 0 0 at 5 0 5 circle 0 0 5
",
    );
    let want = pi * (400.0 * 40.0 - 225.0 * 20.0 - 25.0 * 5.0);
    assert!((v - want).abs() < 2e-3 * want, "{v} vs {want}");
    // a hole through two separate discs
    let v = builds(
        "extrude 1 z 0 -1 0 x 1 0 0 at 0 0 0 circle 0 0 10
extrude add 0.5 z 0 1 0 x 1 0 0 at 0 -10 0 circle 0 0 25
extrude cut sym 25 z 0 1 0 x 1 0 0 at 0 -10 0 circle 0 0 6
",
    );
    let want = pi * (100.0 - 36.0 + 0.5 * (625.0 - 36.0));
    assert!((v - want).abs() < 2e-3 * want, "{v} vs {want}");
    // a slot parting a bar in one cut
    builds(
        "extrude 10 rect 0 0 40 10
extrude cut sym 30 rect 18 -5 4 20
extrude cut 3 at 5 5 7 circle 0 0 1
extrude cut 3 at 35 5 7 circle 0 0 1
",
    );
}

/// A half disc starting flush with a block's face and crossing it further round: the flush edge is
/// named as the cut it would be, and the new edge is that cut (DuplicateName).
#[test]
fn a_flush_edge_named_as_a_cut_that_is_there() {
    builds(
        "extrude 60 z 1 0 0 x 0 1 0 at -60 -30 -10 rect 0 0 60 30
revolve add 185 z 1 0 0 x 0 1 0 at 0 0 0 axis 0 0 0 -1 rect 0 0 12 8
",
    );
}

/// A piece of a wall split where the wall's seam bounds it, across that seam from a piece a cut
/// named by it: they came to one name (DuplicateName). A pocket through a tube's rim, and a ring
/// that fills part of it.
#[test]
fn pieces_across_a_seam_are_named_apart() {
    let m = Model::new(
        "extrude 25 z 0 0 1 x 1 0 0 at 0 0 0 circle 0 0 67
extrude cut 20 z 0 0 1 x 1 0 0 at 0 0 0 circle 0 0 30
extrude cut 20 z 0 0 1 x 1 0 0 at 0 0 0 circle 0 0 30
extrude cut 20 z 0 0 1 x 1 0 0 at -60 -20 0 circle 0 60 30
extrude add 20 z 0 0 1 x 1 0 0 at -50 0 0 circle 50 0 47 circle 50 0 30
",
    );
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    for i in 0..m.built() {
        for s in m.solids_after(i + 1) {
            let p = forge_topo::check(&s.body);
            assert!(p.is_empty(), "line {i}: {p:?}");
        }
    }
    builds(
        "extrude 25 z 0 0 1 x 1 0 0 at 0 0 0 circle 0 0 67
extrude cut 20 z 0 0 1 x 1 0 0 at 0 0 0 circle 0 0 30
extrude cut 20 z 0 0 1 x 1 0 0 at 0 0 0 circle 0 0 30
extrude cut 20 z 0 0 1 x 1 0 0 at -60 -20 0 circle 0 60 30
extrude add 20 z 0 0 1 x 1 0 0 at -50 0 0 circle 50 0 47 circle 50 0 30
",
    );
}

/// Two opened faces side by side: the rims end along the edge they share. A square tube's corner
/// cut open, against its closed form.
#[test]
fn a_shell_with_two_opened_faces_side_by_side() {
    let v = builds(
        "extrude 16 rect -10 -10 20 20
shell 2 (faces plane normal +z) (faces plane normal +x)
",
    );
    let want = 20.0 * 20.0 * 16.0 - 18.0 * 16.0 * 14.0;
    assert!((v - want).abs() < 1e-3 * want, "{v} vs {want}");
}

/// A block with every edge rounded, its top opened: the walls leave the top tangentially, so the
/// offsets never reach it, and they end in strips square to it, one wall thick.
#[test]
fn a_shell_opening_a_face_its_neighbours_leave_tangentially() {
    let v = builds(
        "extrude 40 rect 0 0 100 60
fillet 5 (edges)
shell 2 (faces plane normal +z)
",
    );
    // the skin's area less the opening, times the wall, about
    assert!(v > 0.0, "{v}");
    // a rod's rounded end, its other end opened: the circle between tangent faces moves with them
    let pi = std::f64::consts::PI;
    let v = builds(
        "extrude 30 at 0 0 0 circle 0 0 40
fillet 20 (edges circle top)
shell 5 (faces plane normal -z)
",
    );
    assert!(v > 0.0 && v < pi * 1600.0 * 30.0 * 0.6, "{v}");
}

/// Writing a varying fillet's start into its line must not turn the law round: the regen runs a group
/// in order of name, the start was worked out in the order of the line (2 mm^3, silent, on a sketch
/// line's first save).
#[test]
fn writing_a_law_start_moves_nothing() {
    for pick in [
        "(edges near 100 20 0) (edges near 50 0 0)",
        "(edges near 50 0 0) (edges near 100 20 0)",
        "(edges near 100 20 0) (edges near 50 40 0)",
        "(edges near 50 40 0) (edges near 100 20 0)",
    ] {
        let mut m = Model::new(&format!("extrude 30 rect 0 0 100 40\nfillet 4..2 {pick}\n"));
        assert!(m.error.is_none(), "{pick}");
        let before = m.mass().unwrap().volume;
        let wrote = m.fill_law_starts();
        assert!(!wrote.is_empty(), "{pick}");
        let after = m.mass().unwrap().volume;
        assert!((before - after).abs() < 1e-9 * before, "{pick}: {before} {after}");
    }
}

/// A varying radius over a chain runs from the chain's lowest free end, in whatever order the edges
/// were given and whatever they are named.
#[test]
fn a_law_over_a_chain_starts_at_its_lowest_free_end() {
    let vol = |pick: &str| {
        let mut m = Model::new(&format!("extrude 30 rect 0 0 100 40\nfillet 4..2 {pick}\n"));
        assert!(m.error.is_none(), "{pick}");
        m.mass().unwrap().volume
    };
    let a = vol("(edges near 100 20 0) (edges near 50 0 0)");
    let b = vol("(edges near 50 0 0) (edges near 100 20 0)");
    assert_eq!(a.to_bits(), b.to_bits());
    let c = vol("(edges near 100 20 0) (edges near 50 40 0)");
    let d = vol("(edges near 50 40 0) (edges near 100 20 0)");
    assert_eq!(c.to_bits(), d.to_bits());
}

/// To the next face backwards, as forwards with the frame flipped: a floor, a wall and a roof, and a
/// block from mid air down to the floor or up to the roof, both ways round.
#[test]
fn to_next_backwards() {
    let base = "extrude 10 rect -30 -30 60 60
extrude add 50 at 0 0 10 rect -30 -30 5 60
extrude add 10 at 0 0 50 rect -30 -30 60 60
";
    let full = 60.0 * 60.0 * 10.0 * 2.0 + 5.0 * 60.0 * 40.0;
    let back = builds(&format!(
        "{base}extrude next back xy at 0 0 25 rect -5 -5 10 10\n"
    ));
    let flip = builds(&format!("{base}extrude next -xy at 0 0 25 rect -5 -5 10 10\n"));
    assert!((back - flip).abs() < 1e-6 * back, "{back} {flip}");
    assert!((back - (full + 100.0 * 15.0)).abs() < 1e-3 * back, "{back}");
    let up = builds(&format!("{base}extrude next xy at 0 0 25 rect -5 -5 10 10\n"));
    assert!((up - (full + 100.0 * 25.0)).abs() < 1e-3 * up, "{up}");
    // a cut from inside, and a slanted roof over it
    let cut = builds(&format!(
        "{base}extrude cut next back xy at 0 0 5 rect -5 -5 10 10\n"
    ));
    assert!((cut - (full - 100.0 * 5.0)).abs() < 1e-3 * cut, "{cut}");
}

/// To the next face in a part of several bodies: of whichever body comes first along the rays.
#[test]
fn to_next_among_bodies() {
    let m = Model::new(
        "a: extrude new 10 rect -30 -30 60 60
b: extrude new 10 at 0 0 40 rect -30 -30 60 60
extrude add next xy at 0 0 10 rect -5 -5 10 10 in a
",
    );
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| e.msg.clone()));
    let part = Part {
        features: m.features.clone(),
    };
    let total = |solids: Vec<forge_model::Solid>| -> f64 {
        solids
            .iter()
            .map(|s| forge_topo::mass::mass(&s.body).unwrap().volume)
            .sum()
    };
    let want = 2.0 * 36000.0 + 100.0 * 30.0;
    let v = total(part.regenerate_solids().expect("seq"));
    assert!((v - want).abs() < 1e-6 * want, "{v}");
    let v = total(part.regenerate_nary_solids(1).expect("n-ary"));
    assert!((v - want).abs() < 1e-6 * want, "{v}");
}

/// A tilted cutter whose rim hovers within tolerance of a part's bottom over a chord: under that stretch
/// the curve where its wall meets the bottom is a new edge all the same, or being in the tool flips across
/// the rim ("inconsistent classification"). The slit between chord and curve is a hair wide.
#[test]
fn a_tilted_cutter_hovering_over_a_face() {
    let src = "extrude 0.6 z 0 0 1 x 1 0 0 at 0 0 0 circle 0 0 0.4
extrude cut 0.5 z 0.0174524064 0 -0.9998476952 x 0.9998476952 0 0.0174524064 at 0 0 0.50515927 circle 0 0 0.3
";
    builds(src);
    let v = Model::new(src).mass().unwrap().volume;
    // the cylinder less the slanted hole: axis length 0.5 at any tilt
    let pi = std::f64::consts::PI;
    let want = pi * (0.16 * 0.6 - 0.09 * 0.5);
    assert!((v - want).abs() < 1e-3 * want, "{v} vs {want}");
}

/// A dome on a square's top, its rim tangent to the sides: where they touch the two solids also share
/// the top right up to the point, so no material meets itself there (false ZeroThickness).
#[test]
fn a_dome_inscribed_on_a_square_top() {
    let pi = std::f64::consts::PI;
    for scale in [1.0, 40.0] {
        let (a, r) = (0.6 * scale, 0.3 * scale);
        let src = format!(
            "extrude {a} rect {} {} {a} {a}
revolve add z 1 0 0 x 0 1 0 at 0 0 {a} axis 0 {r} 0 -1 loop line 0 0 {r} 0 arc 0 0 {r} 0 90 line 0 {r} 0 0
",
            -r, -r
        );
        builds(&src);
        let want = a * a * a + 2.0 * pi * r * r * r / 3.0;
        let got = Model::new(&src).mass().unwrap().volume;
        assert!((got - want).abs() < 1e-6 * want, "{got} {want}");
    }
}

/// A profile leaving the axis and coming back: the disc shaped void inside is a shell of its own.
#[test]
fn a_revolve_closing_a_void_on_its_axis() {
    let pi = std::f64::consts::PI;
    let v = builds(
        "revolve xz axis y loop line 0 0 8 0 line 8 0 8 30 line 8 30 0 30 line 0 30 0 20 line 0 20 4 20 line 4 20 4 10 line 4 10 0 10 line 0 10 0 0\n",
    );
    let want = pi * (64.0 * 30.0 - 16.0 * 10.0);
    assert!((v - want).abs() < 3e-3 * want, "{v} {want}");
}

/// A revolve's axis drawn as a plain line in its sketch is no part of the profile; one of the
/// profile's own edges stays one.
#[test]
fn a_revolve_axis_drawn_as_a_plain_line_or_an_edge_of_the_profile() {
    let pi = std::f64::consts::PI;
    let sk = "sk1: sketch xz\n  rect 10 0 10 20\n  line 0 0 0 30\nend\n";
    let v = Model::new(&format!("{sk}revolve sk1 axis l5\n"))
        .mass()
        .unwrap()
        .volume;
    assert!((v - pi * 300.0 * 20.0).abs() < 1e-6 * v, "{v}");
    // with the line not the axis, it's an open chain, said so
    let m = Model::new(&format!("{sk}revolve sk1 axis 0 0 0 1\n"));
    assert!(
        m.error.as_ref().is_some_and(|e| e.msg.contains("open")),
        "{:?}",
        m.error
    );
    let v = Model::new("sk1: sketch xz\n  rect 10 0 10 20\nend\nrevolve 90 sk1 axis l4\n")
        .mass()
        .unwrap()
        .volume;
    assert!((v - pi * 100.0 * 20.0 / 4.0).abs() < 1e-6 * v, "{v}");
}

/// A cut that takes a whole body away, then an add: it goes to the body left, as sequential does; one
/// run made the add's tool a body of its own (Fusion 51895_0007).
#[test]
fn an_add_after_a_cut_emptied_a_body_goes_to_the_body_left() {
    let m = Model::new(
        "extrude 34 rect 0 0 100 100
extrude new 2 at 0 0 40 rect 10 10 50 50
extrude cut 2 at 0 0 40 rect 10 10 50 50
extrude add 2 at 0 0 50 rect 0 0 10 10
",
    );
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let part = Part {
        features: m.features.clone(),
    };
    let nary = part.regenerate_nary_solids(1).expect("n-ary");
    assert_eq!(nary.len(), m.solids_after(m.built()).len());
    assert_eq!(nary.len(), 1);
}

/// A cut nearly tangent to a blend: a step of the chord walk landed on the far side of the tube, the
/// face's integral never settled, and its bound said 10% (fuzz p09444). This one settles on what the
/// mesh says.
#[test]
fn a_cut_nearly_tangent_to_a_blend_has_a_volume() {
    let m = Model::new(
        "body: extrude 33.8538 z 0 0 1 x 1 0 0 at 0 0 0 circle 0 0 18.6873
boss: extrude add 34.7602 z -0.372619194143 -0.813273460032 -0.446924171828 x 0 -0.481607449972 0.876387051554 at -4.34617495356 1.52942340372 14.4425234406 circle 0 0 10.5378
fillet 1.71121082 (edges between body boss concave)
c0: extrude cut 43.8377 z -0.234138759071 -0.811155896008 0.535915248777 x -0 0.551237904395 0.834348112456 at -21.1807704343 -6.15178668063 5.35835986907 rect -87.6754 -87.6754 175.351 175.351
c1: extrude cut 87.6754 z -0.927984340469 0.326558636907 0.179456179902 x -0.18986516855 0 -0.981810174001 at 19.4999129356 -20.4673606945 -2.5085832624 circle 1.02307 -1.47558 3.04561528
",
    );
    let b = m.body().expect("builds").compact();
    let p = forge_topo::mass::mass(&b).expect("a mass");
    assert!(p.err[0] <= 1e-5 * p.volume, "{} +- {}", p.volume, p.err[0]);
    let q = forge_mesh::Quality {
        chord: 0.01,
        ..forge_mesh::Quality::default()
    };
    let v = forge_mesh::tessellate(&b, &q).expect("mesh").volume();
    assert!((p.volume - v).abs() <= 1e-3 * v, "{} vs the mesh's {v}", p.volume);
}

/// A cut that starts on a face and runs away from the part removes nothing: its face is still the
/// block's, or a pick on it finds no maker.
#[test]
fn a_cut_removing_nothing_leaves_each_face_its_maker() {
    let m = Model::new("extrude 30 rect -30 -30 60 60\nextrude cut 10 at 0 0 30 rect -10 -10 20 20\n");
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let b = m.body().expect("a body");
    let makers = m.face_makers();
    for f in b.face_ids() {
        assert_eq!(makers[f.index()], Some(0), "{f:?}");
    }
}

/// A flat side crossing a round end a hair past the inner corner: the stretch of the end between them is
/// shorter than the tolerance's reach, was taken as on the tool's edge and never imprinted, and the
/// tool's floor came out a slit (false ZeroThickness; WHUCAD 100003369 had this).
#[test]
fn a_stretch_shorter_than_the_tolerance_between_two_crossings_is_imprinted() {
    let src = "extrude 0.1 loop arc 0.09 0 0.09 180 180 arc 0.162 0 0.018 0 180 arc 0.09 0 0.054 0 -180 arc 0.018 0 0.018 0 180
extrude add 0.03 loop arc 0.098588751 0.0072 0.063498271 -173.489291243 170.489291243 line 0.162 0.003876757 0.0355 0
";
    builds(src);
    let v = Model::new(src).mass().expect("a mass").volume;
    // the banana 0.1 high, and the part of the D past its hollow 0.03 high: areas on a fine grid
    assert!((v - 0.00105669).abs() < 1e-3 * 0.00105669, "{v}");
}

/// A chamfer on a rim ending at a corner where the wall behind it turns away: its plane meets the rim beyond
/// the corner, not on it, and the flat beside it runs on that far (was TooLarge at any size). OCCT's volume.
#[test]
fn a_chamfer_ending_at_a_concave_corner_of_the_rim() {
    let mut v = Model::new(
        "extrude 10 poly 0 0 40 0 40 10 25 10 20 20 0 20
chamfer 1 (edges near 22.5 15 10)
",
    );
    assert!(v.error.is_none(), "{:?}", v.error.as_ref().map(|e| &e.msg));
    let b = v.body().expect("a body");
    assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
    let m = v.mass().expect("a mass").volume;
    assert!((m - 6244.40983).abs() < 1e-4, "{m}");
}

/// A chamfer beside an earlier one of the same size: its plane runs through the far end of the edge the first
/// left at the corner, so that edge goes and its far vertex is the corner (was TooLarge, an edge of length 0).
/// By hand: 1000 - 5 - (5 - 1/3), each cut 1/2 x 10, less the 1/3 the two share.
#[test]
fn a_chamfer_beside_an_equal_one_takes_the_short_edge_between() {
    let mut v = Model::new(
        "extrude 10 rect 0 0 10 10\nchamfer 1 (edges near 5 0 10)\nchamfer 1 (edges near 10 5 10)\n",
    );
    assert!(v.error.is_none(), "{:?}", v.error.as_ref().map(|e| &e.msg));
    let b = v.body().expect("a body");
    assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
    let m = v.mass().expect("a mass").volume;
    assert!((m - (990.0 + 1.0 / 3.0)).abs() < 1e-6, "{m}");
}

/// Three chamfers of one feature at a corner, `corner point`: their cut planes run on to a point (CATIA's corner;
/// the default is Parasolid's flat triangle). Own numbers: the volume of |x|,|y|,|z| <= 5 with |x|+|y|, |y|+|z|,
/// |z|+|x| <= 9, by inclusion-exclusion: 1000 - 12 * 5 + 8 * 3/4. The line prints back as written.
#[test]
fn three_chamfers_at_a_corner_meet_at_a_point_when_asked() {
    let mut v = Model::new("extrude 10 rect 0 0 10 10\nchamfer 1 corner point (edges)\n");
    assert!(v.error.is_none(), "{:?}", v.error.as_ref().map(|e| &e.msg));
    let b = v.body().expect("a body");
    assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
    assert_eq!(b.faces.len(), 18);
    let m = v.mass().expect("a mass").volume;
    assert!((m - 946.0).abs() < 1e-6, "{m}");
    let mut p = Model::new("extrude 10 rect 0 0 10 10\nchamfer 1 (edges)\n");
    assert_eq!(p.body().expect("a body").faces.len(), 26);
    assert!(p.mass().expect("a mass").volume < 946.0);
}

/// A closed spline turned about an axis: its integrals have to stop at the section's knots, where the
/// spline has only two derivatives, or they do not settle (refused here, 0.7% off read back from a
/// file). Pappus, from the same spline extruded: its area and its centroid.
#[test]
fn a_closed_spline_turned_has_its_volume() {
    use forge_math::dmath;
    let pts: Vec<String> = (0..=60)
        .map(|k| {
            let t = k as f64 * dmath::TAU / 60.0;
            let x = 12.0 * dmath::cos(t) - 2.0 * dmath::cos(4.0 * t) + 40.0;
            let y = 12.0 * dmath::sin(t) - 2.0 * dmath::sin(4.0 * t);
            format!("{x:.6} {y:.6}")
        })
        .collect();
    let loop_ = pts.join(" ");
    let mut flat = Model::new(&format!("extrude 10 thru {loop_}\n"));
    assert!(flat.error.is_none(), "{:?}", flat.error.as_ref().map(|e| &e.msg));
    let m = flat.mass().expect("a mass");
    let want = dmath::TAU * m.centroid.x * m.volume / 10.0;
    let mut turned = Model::new(&format!("revolve xz axis y thru {loop_}\n"));
    assert!(
        turned.error.is_none(),
        "{:?}",
        turned.error.as_ref().map(|e| &e.msg)
    );
    let v = turned.mass().expect("a mass").volume;
    assert!((v - want).abs() < 1e-5, "{v} {want}");
    let b = turned.body().expect("a body");
    let file = forge_step::write::export(b, &forge_step::write::Options::default()).unwrap();
    let back = forge_step::read::import(file.text.as_bytes(), &forge_step::Limits::default()).unwrap();
    let again = forge_topo::mass::mass(&back.solids[0].body).unwrap().volume;
    assert!((again - want).abs() < 1e-5 * want, "{again} {want}");
}

/// A blade's root rounded where its long walls and ends mitre: the wall runs a little past the loft's
/// end, and its fit in a file has to go with it or the file's edges lie off their faces.
#[test]
fn a_mitred_wall_past_a_lofts_end_is_all_in_the_file() {
    let m = Model::new(
        "base: extrude 10 rect -40 -40 80 80\n\
         b: loft smooth profile z 0 0 1 x 1 0 0 at 0 0 5 rect -25 -3 50 6 profile z 0 0 1 x 1 0 0 at 0 0 30 rect -22 -3 44 6 \
         profile z 0 0 1 x 1 0 0 at 0 0 60 rect -18 -3 36 6\n\
         fillet 1.5 (edges between base b)\n",
    );
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let b = m.body().expect("a body");
    let file = forge_step::write::export(b, &forge_step::write::Options::default()).unwrap();
    assert!(file.uncertainty < 2e-3, "{}", file.uncertainty);
}

/// A round between two discs a little off centre from each other: its blend runs on a traced spine,
/// and the circle on its foot goes round the blend's parameter unevenly. Its mass has to settle.
#[test]
fn a_round_between_eccentric_discs_has_its_mass() {
    let m = Model::new(
        "extrude 0.125 at -0.3127 0 0 circle 0.316 0 0.316\n\
         extrude add 0.03 at -0.3751 0 0 circle 0.379 0 0.379\n\
         fillet 0.0625 (edges concave)\n",
    );
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let b = m.body().expect("a body");
    let got = forge_topo::mass::mass(b).expect("a mass");
    // the loop's closure reports how far the rules round the circle miss the turn
    assert!(got.err[0] < 1e-4 * got.volume, "{} of {}", got.err[0], got.volume);
    let q = Quality {
        chord: 0.0001,
        ..Quality::default()
    };
    let mesh = tessellate(b, &q).unwrap();
    let meshed: f64 = mesh
        .triangles
        .iter()
        .map(|t| {
            let [p, r, s] = t.map(|i| mesh.positions[i as usize].to_vec());
            p.dot(r.cross(s)) / 6.0
        })
        .sum();
    assert!(
        (got.volume - meshed).abs() < 1e-3 * got.volume,
        "{} {meshed}",
        got.volume
    );
}

/// A curved blade with round ends on a hub, its end running over the hub's concave round. The round
/// and the extended end meet in a loop with a kink: tracing one way stopped short of the kink and the
/// other way got past it and closed, and the two joined ran round the loop twice.
#[test]
fn a_loop_traced_closed_on_the_way_back_is_not_run_round_twice() {
    use forge_math::dmath;
    let frame = |z: f64, deg: f64| {
        let a = deg.to_radians();
        format!("z 0 0 1 x {} {} 0 at 0 0 {z}", dmath::cos(a), dmath::sin(a))
    };
    let blade = |cx: f64, r: f64, t: f64| {
        let at = |deg: f64| {
            (
                cx + r * dmath::cos(deg.to_radians()),
                r * dmath::sin(deg.to_radians()),
            )
        };
        let (s, e) = (at(74.0), at(162.0));
        format!(
            "loop arc {cx} 0 {} 74 88 arc {} {} {t} 162 180 arc {cx} 0 {} 162 -88 arc {} {} {t} 254 180",
            r + t,
            e.0,
            e.1,
            r - t,
            s.0,
            s.1
        )
    };
    let round = 34f64;
    let (top, drop) = (39f64, 39f64 - 5.42);
    let a = dmath::asin(drop / round);
    let (x, z) = (20.0 + round - round * dmath::cos(a), top - round * dmath::sin(a));
    let hub = format!(
        "hub: revolve xz axis y loop line 0 -16 24 -16 line 24 -16 24 0 line 24 0 92 0 line 92 0 92 2.6 line 92 2.6 {x} {z} \
         arc {} {top} {round} {} {} line 20 {top} 20 46 line 20 46 0 46 line 0 46 0 -16\n",
        20.0 + round,
        180.0 + a.to_degrees(),
        -a.to_degrees()
    );
    let volume = |turn: f64| {
        let loft = format!(
            "blade: loft smooth profile {} {} profile {} {} profile {} {}\n",
            frame(1.5, turn),
            blade(41.0, 61.0, 1.5),
            frame(23.0, turn - 10.0),
            blade(40.0, 69.0, 1.275),
            frame(45.0, turn - 26.0),
            blade(40.0, 81.0, 1.05)
        );
        let m = Model::new(&format!("{hub}{loft}"));
        assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
        let body = m.body().unwrap().clone();
        assert!(forge_topo::check(&body).is_empty());
        forge_topo::mass::mass(&body).unwrap().volume
    };
    let (a, b) = (volume(-47.0), volume(-60.0));
    // turned about the hub's axis it is the same solid
    assert!((a - b).abs() < 1e-5, "{a} {b}");
    // OpenCascade reads the exported solid at 245091.55
    assert!((a - 245091.54).abs() < 0.05, "{a}");
}

/// A cut a hair off tangent to two pillars under a plate: where the cut's circle and each pillar's
/// touch, the halves of the conic solve gave two more vertices 1e-4 along the tangent, and the
/// plate's face split crossed itself.
#[test]
fn a_cut_a_hair_off_tangent_to_two_pillars() {
    let pi = std::f64::consts::PI;
    for e in ["0", "0.0001", "0.001", "0.013", "0.05"] {
        let src = format!(
            "extrude 1 circle 0 0 10
extrude 5 -xy at 0 9 0 circle 0 0 1
extrude 5 -xy at 0 -9 0 circle 0 0 1
extrude cut 1 at {e} 0 0 circle 0 0 8
"
        );
        builds(&src);
        let v = forge_topo::mass::mass(Model::new(&src).body().expect("a body"))
            .expect("mass")
            .volume;
        // a gap under LINEAR (at 0.013) is a touch: the sliver under it, 4e-5, may count
        assert!((v - 46.0 * pi).abs() < 1e-4, "{e}: {v}");
    }
}

/// A plate with a half-round notch whose arc leaves the side wall in a cusp, and a cylinder through
/// the cusp, tangent to both: it runs inside the plate on the wall's side and outside past the
/// notch, so the cusp's edge must split it (refused as "inconsistent classification").
#[test]
fn a_cylinder_through_a_cusp_tangent_to_both_sides() {
    let pi = std::f64::consts::PI;
    let plate =
        "extrude 0.12 loop arc 0.2 0 0.2 180 -180 line 0.4 0 0.4 0.5 line 0.4 0.5 0 0.5 line 0 0.5 0 0\n";
    let r: f64 = 0.25;
    let area = 0.2 - 0.02 * pi;
    // the circle over the plate: its upper half less the segment past x = 0.4, less the notch inside it
    let d = 0.4 - r;
    let segment = r * r * forge_math::dmath::acos(d / r) - d * (r * r - d * d).sqrt();
    let over = pi * r * r / 2.0 - segment / 2.0 - 0.02 * pi;
    let disk = pi * r * r;
    for (tool, want) in [
        ("extrude add 0.12 circle 0.25 0 0.25", 0.12 * (area + disk - over)),
        (
            "extrude add 0.2 circle 0.25 0 0.25",
            0.12 * (area + disk - over) + 0.08 * disk,
        ),
        (
            "extrude add 0.1 at 0 0 0.01 circle 0.25 0 0.25",
            0.12 * area + 0.1 * (disk - over),
        ),
        ("extrude cut 0.1 circle 0.25 0 0.25", 0.12 * area - 0.1 * over),
        (
            "extrude cut 0.1 at 0 0 0.01 circle 0.25 0 0.25",
            0.12 * area - 0.1 * over,
        ),
    ] {
        let src = format!("{plate}{tool}\n");
        builds(&src);
        let v = forge_topo::mass::mass(Model::new(&src).body().expect("a body"))
            .expect("mass")
            .volume;
        assert!((v - want).abs() < 1e-9, "{tool}: {v} vs {want}");
    }
}

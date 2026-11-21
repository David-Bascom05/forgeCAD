//! Parts the kernel used to refuse though they can be built (research/build-failures), as Forge
//! parts, each valid feature by feature and n-ary, the two paths the same solid.
mod common;
mod kernel_util;

use forge_mesh::{Quality, tessellate};
use kernel_util::{builds_alike, built, refusal};

/// Builds valid on both paths with the same volume (by meshes), which it returns.
fn builds(src: &str) -> f64 {
    builds_alike(src, 0.01)
}

/// The part's exact volume.
fn mass(src: &str) -> f64 {
    built(src).volume()
}

/// Two balls touching at a point on a block's top, a third over the point: the top's edges meet
/// there along one tangent, and the way on from one is told from the way straight through by
/// how they bend (adversarial ball_poles: "face split crossed itself").
#[test]
fn ball_over_where_two_balls_touch() {
    for src in [
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (60, 40))
}
extrude1 = extrude(s1, 10)
sketch d2(on: offset(xy, 10)) {
  p = pt(30, 15)
}
sketch s3(on: plane(at: d2.p, normal: -Y, x: X)) {
  c = pt(0, 0)
}
ball1 = ball(s3.c, 4)
sketch d4(on: offset(xy, 10)) {
  p = pt(30, 23)
}
sketch s5(on: plane(at: d4.p, normal: -Y, x: X)) {
  c = pt(0, 0)
}
ball2 = ball(s5.c, 4)
sketch d6(on: offset(xy, 10)) {
  p = pt(30, 19)
}
sketch s7(on: plane(at: d6.p, normal: -Y, x: X)) {
  c = pt(0, 0)
}
ball3 = ball(s7.c, 2)
"#,
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (60, 40))
}
extrude1 = extrude(s1, 10)
sketch d2(on: offset(xy, 10)) {
  p = pt(30, 15)
}
sketch s3(on: plane(at: d2.p, normal: -Y, x: X)) {
  c = pt(0, 0)
}
ball1 = ball(s3.c, 4)
sketch d4(on: offset(xy, 10)) {
  p = pt(30, 23)
}
sketch s5(on: plane(at: d4.p, normal: -Y, x: X)) {
  c = pt(0, 0)
}
ball2 = ball(s5.c, 4, cut: true)
sketch d6(on: offset(xy, 10)) {
  p = pt(30, 19)
}
sketch s7(on: plane(at: d6.p, normal: -Y, x: X)) {
  c = pt(0, 0)
}
ball3 = ball(s7.c, 2)
"#,
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (60, 40))
}
extrude1 = extrude(s1, 10)
sketch d2(on: offset(xy, 10)) {
  p = pt(30, 15)
}
sketch s3(on: plane(at: d2.p, normal: -Y, x: X)) {
  c = pt(0, 0)
}
ball1 = ball(s3.c, 4, cut: true)
sketch d4(on: offset(xy, 10)) {
  p = pt(30, 23)
}
sketch s5(on: plane(at: d4.p, normal: -Y, x: X)) {
  c = pt(0, 0)
}
ball2 = ball(s5.c, 4, cut: true)
sketch d6(on: offset(xy, 10)) {
  p = pt(30, 19)
}
sketch s7(on: plane(at: d6.p, normal: -Y, x: X)) {
  c = pt(0, 0)
}
ball3 = ball(s7.c, 2, cut: true)
"#,
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (60, 40))
}
extrude1 = extrude(s1, 10)
sketch d2(on: offset(xy, 10)) {
  p = pt(30, 15)
}
sketch s3(on: plane(at: d2.p, normal: -Y, x: X)) {
  c = pt(0, 0)
}
ball1 = ball(s3.c, 4, cut: true)
sketch d4(on: offset(xy, 10)) {
  p = pt(30, 23)
}
sketch s5(on: plane(at: d4.p, normal: -Y, x: X)) {
  c = pt(0, 0)
}
ball2 = ball(s5.c, 4)
sketch d6(on: offset(xy, 10)) {
  p = pt(30, 19)
}
sketch s7(on: plane(at: d6.p, normal: -Y, x: X)) {
  c = pt(0, 0)
}
ball3 = ball(s7.c, 2, cut: true)
"#,
    ] {
        builds(src);
    }
}

/// A rod lying on a cap touches it along a line where the one probe of an untouched face landed:
/// other points of the face say which side it is on (degenerate perpendicular_on_cap).
#[test]
fn a_probe_on_a_touch_asks_again() {
    let plain = builds(
        r#"sketch s1(on: xy) {
  c1 = circle((0, 0), 10)
}
extrude1 = extrude(s1, 20)
"#,
    );
    let v = builds(
        r#"sketch s1(on: xy) {
  c1 = circle((0, 0), 10)
}
extrude1 = extrude(s1, 20)
sketch d2(on: offset(xy, 25)) {
  p = pt(-20, 0)
}
sketch s3(on: plane(at: d2.p, normal: X, x: Y)) {
  c1 = circle((0, 0), 5)
}
cut1 = cut(s3, -40)
"#,
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
        r#"sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (60, 40))
}
block = extrude(s, 20)
round = fillet(edges(between: (block.end, block.side(s.r[0]))) + edges(between: (block.end, block.side(s.r[2]))), 2)
sketch top(on: offset(xy, 20)) {
  p = pt(30, 20)
}
drilled = hole(at: top.p, d: 6, depth: next)
"#,
    );
    let block = builds(
        r#"sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (60, 40))
}
block = extrude(s, 20)
round = fillet(edges(between: (block.end, block.side(s.r[0]))) + edges(between: (block.end, block.side(s.r[2]))), 2)
"#,
    );
    let bore = forge_math::dmath::PI * 9.0 * 20.0;
    assert!(
        (block - drilled - bore).abs() < 1e-2 * bore,
        "{block} {drilled} {bore}"
    );
    builds(
        r#"sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (60, 40))
}
block = extrude(s, 20)
bevel = chamfer(edges(between: (block.end, block.side(s.r[0]))) + edges(between: (block.end, block.side(s.r[2]))), 2)
sketch o(on: offset(xy, 10)) {
  p = pt(30, 20)
}
sketch rod(on: plane(at: o.p, normal: -Z, x: X)) {
  c = circle((0, 0), 3)
}
pin = extrude(rod, next)
"#,
    );
}

/// Cylinders of one radius butted end to end, built from the joint outwards: the joint circle is
/// one closed edge the two run round opposite ways, which its ends cannot say (one reversed use
/// made an open shell, or a false touch). The commonest refusal on real parts.
#[test]
fn butted_cylinders_of_one_radius() {
    let pi = forge_math::dmath::PI;
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 6)
}
extrude1 = extrude(s1, 12)
sketch s2(on: plane(at: o.p, normal: -Y, x: X)) {
  c1 = circle((0, 0), 6)
}
extrude2 = extrude(s2, 30)
"#,
    );
    assert!((v - pi * 36.0 * 42.0).abs() < 1e-2 * v, "{v}");
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: -Y, x: X)) {
  c1 = circle((0, 0), 11)
  c2 = circle((0, 0), 8)
}
extrude1 = extrude(s1, 9)
sketch s2(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 11)
}
extrude2 = extrude(s2, 5)
"#,
    );
    assert!((v - pi * (121.0 * 14.0 - 64.0 * 9.0)).abs() < 1e-2 * v, "{v}");
}

/// A tool's edge a hair clear of a wall makes no cut on the curve passing it, and that curve's
/// middle lands on the edge's face boundary: other samples show it is a new edge, not one along
/// a boundary. The cut did nothing, silently; 2e-7 is what a model rounded to float32 carries.
#[test]
fn a_tool_edge_a_hair_off_the_wall_still_cuts() {
    let (exact, noisy) = (
        builds(
            r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: -Y, x: X)) {
  c1 = circle((0, 0), 15)
}
extrude1 = extrude(s1, 30)
sketch s2(on: plane(at: o.p, normal: X, x: -Z)) {
  p1 = poly([(-1, -39), (0, -30), (15, -4), (22, -14), (22, -37)])
}
cut1 = cut(s2, two(-35, -35))
"#,
        ),
        builds(
            r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: -Y, x: X)) {
  c1 = circle((0, 0), 15)
}
extrude1 = extrude(s1, 30)
sketch s2(on: plane(at: o.p, normal: X, x: -Z)) {
  p1 = poly([(-1, -39), (0, -30), (15.0000002, -4), (22, -14), (22, -37)])
}
cut1 = cut(s2, two(-35, -35))
"#,
        ),
    );
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
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  p1 = poly([(0, -150), (0, 0), (200, 0), (200, -150)])
}
extrude1 = extrude(s1, 5)
sketch s2(on: plane(at: o.p, normal: X, x: -Z)) {
  p1 = poly([(0, -80), (-100, 0), (0, 0)])
}
extrude2 = extrude(s2, 200)
sketch s3(on: plane(at: o.p, normal: -Y, x: X)) {
  p1 = poly([(190, 100), (190, 10), (10, 10), (10, 100)])
}
cut1 = cut(s3, -90)
"#,
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
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 8)
  c2 = circle((0, 0), 6)
}
extrude1 = extrude(s1, 150)
sketch s2(on: plane(at: o.p, normal: Z, x: X)) {
  p1 = poly([(7.99999994, 150.4), (-7, 150), (8, 138)])
}
cut1 = cut(s2, two(-9, -9))
"#,
    );
    // a cross hole ending on the plane tangent to the rod
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: X, x: -Z)) {
  c1 = circle((0, 0), 40)
}
extrude1 = extrude(s1, 80)
sketch s2(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((30, 0), 10)
}
cut1 = cut(s2, two(-40, -50))
"#,
    );
    let rod = forge_math::dmath::PI * 1600.0 * 80.0;
    assert!(v < rod && v > 0.9 * rod, "{v}");
}

/// A flat cut on a rod from its end by a profile whose arc lies on the rod's own circle: the flat's
/// two ends take their names from what meets there, alike; they are told apart by where they lie.
#[test]
fn flat_on_a_rod_by_an_arc_on_its_circle() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 6)
}
extrude1 = extrude(s1, two(15, 15))
sketch d2(on: offset(xy, 0)) {
  p = pt(0, 15)
}
sketch s3(on: plane(at: d2.p, normal: -Y, x: -X)) {
  l1 = line((-1.999999999951556, 5.656854249509507), (-1.9999999999515534, -5.656854249509508))
  a2 = arc(center: (0, 0), from: (-1.9999999999515534, -5.656854249509508), to: (-1.999999999951556, 5.656854249509507), cw: true)
}
cut1 = cut(s3, -7)
"#,
    );
}

/// A tool's side all but tangent to a round floor's edge, the floor's wall a hair wider than the
/// tool reaches: the imprint runs a hair from the boundary between the same vertices, and is it.
#[test]
fn imprint_a_hair_from_the_boundary() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 7)
}
extrude1 = extrude(s1, 4)
sketch s2(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 5.00000006)
}
cut1 = cut(s2, -3)
sketch d3(on: offset(xy, 0)) {
  p = pt(5, 0)
}
sketch s4(on: plane(at: d3.p, normal: X, x: -Z)) {
  p1 = poly([(-4, 2.5), (-4, 0), (4, 0), (4, 2.5)])
}
extrude2 = extrude(s4, 4)
"#,
    );
}

/// A cone that comes to a point stays one when shelled towards its axis: its apex moves along the
/// axis. Refused at any thickness before, for a boss's tip or a drill point shelled outward.
#[test]
fn shell_towards_a_cone_point() {
    let v = builds(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (80, 80))
}
extrude1 = extrude(s1, 30)
sketch d2(on: offset(xy, 30)) {
  p = pt(40, 40)
}
sketch s3(on: plane(at: d2.p, normal: -Y, x: X)) {
  ax = line((0, 0), (0, 1), construction: true)
  p1 = poly([(0, 0), (8, 0), (0, 10)])
}
revolve1 = revolve(s3, axis: s3.ax)
cup = shell(2, open: extrude1.start)
"#,
    );
    let half = forge_math::dmath::atan2(8.0, 10.0);
    let h = 40.0 - 2.0 / forge_math::dmath::sin(half) - 28.0;
    let r = h * forge_math::dmath::sin(half) / forge_math::dmath::cos(half);
    let pi = forge_math::dmath::PI;
    let want = 192000.0 + pi * 640.0 / 3.0 - (76.0 * 76.0 * 28.0 + pi * r * r * h / 3.0);
    assert!((v - want).abs() < 3e-3 * want, "{v} {want}");
    builds(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (80, 80))
}
extrude1 = extrude(s1, 30)
sketch d2(on: offset(xy, 30)) {
  p = pt(40, 40)
}
sketch s3(on: plane(at: d2.p, normal: -Z, x: X)) {
  p = pt(0, 0)
}
hole1 = hole(at: s3.p, d: 6, depth: 15, flip: true, point: 118deg)
cup = shell(2, open: extrude1.end, out: true)
"#,
    );
}

/// Balls whose poles touch on a block's top, a third over the touch or beside it: the pinch where
/// two circles touch is told apart by how they bend, on both paths (handed over by bugs-5).
#[test]
fn balls_touching_at_poles_on_a_top_face() {
    builds(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (60, 40))
}
extrude1 = extrude(s1, 10, new: true)
sketch d2(on: offset(xy, 10)) {
  p = pt(33.3, 25.3)
}
sketch s3(on: plane(at: d2.p, normal: Y, x: X)) {
  c = pt(0, 0)
}
ball1 = ball(s3.c, 4.1, cut: true)
sketch d4(on: offset(xy, 10)) {
  p = pt(33.3, 33.5)
}
sketch s5(on: plane(at: d4.p, normal: Y, x: X)) {
  c = pt(0, 0)
}
ball2 = ball(s5.c, 4.1)
sketch d6(on: offset(xy, 10)) {
  p = pt(30.7, 12.4)
}
sketch s7(on: plane(at: d6.p, normal: Y, x: X)) {
  c = pt(0, 0)
}
ball3 = ball(s7.c, 5.1)
sketch d8(on: offset(xy, 10)) {
  p = pt(30.7, 22.6)
}
sketch s9(on: plane(at: d8.p, normal: Y, x: X)) {
  c = pt(0, 0)
}
ball4 = ball(s9.c, 5.1, cut: true)
"#,
    );
    builds(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (60, 40))
}
extrude1 = extrude(s1, 10, new: true)
sketch d2(on: offset(xy, 10)) {
  p = pt(36.529760196271496, 12.25756041565283)
}
sketch s3(on: plane(at: d2.p, normal: Y, x: X)) {
  c = pt(0, 0)
}
ball1 = ball(s3.c, 4.7742110386764, cut: true)
sketch d4(on: offset(xy, 10)) {
  p = pt(36.529760196271496, 21.80598249300563)
}
sketch s5(on: plane(at: d4.p, normal: Y, x: X)) {
  c = pt(0, 0)
}
ball2 = ball(s5.c, 4.7742110386764)
sketch d6(on: offset(xy, 10)) {
  p = pt(32.83, 24.37)
}
sketch s7(on: plane(at: d6.p, normal: Y, x: X)) {
  c = pt(0, 0)
}
ball3 = ball(s7.c, 3.71)
sketch d8(on: offset(xy, 10)) {
  p = pt(32.83, 31.8)
}
sketch s9(on: plane(at: d8.p, normal: Y, x: X)) {
  c = pt(0, 0)
}
ball4 = ball(s9.c, 3.71)
"#,
    );
}

/// One solid pinched cannot be made: the refusal says where, and the feature that made it.
#[test]
fn pinches_say_where() {
    let e = refusal(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (20, 20))
}
extrude1 = extrude(s1, 10)
sketch d2(on: offset(xy, 0)) {
  p = pt(20, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  r1 = rect(corner: (0, 20), size: (20, 20))
}
extrude2 = extrude(s3, 10)
"#,
    );
    assert!(
        e.contains("ZeroThickness") && e.contains("x: 20.0, y: 20.0") && e.contains("feature: 4"),
        "{e}"
    );
}

/// The other half of `pinches_say_where`: a disc whose hole touches its rim. The .fc part:
///   extrude 5 circle 0 0 40 circle 0 -30 10
/// refused by the kernel as ZeroThickness at (0, -40, ..), on the line that made it.
#[test]
fn a_pinched_disc_says_where() {
    let e = refusal(
        r#"sketch s1(on: xy) {
  c1 = circle((0, 0), 40)
  c2 = circle((0, -30), 10)
}
extrude1 = extrude(s1, 5)
"#,
    );
    assert!(e.contains("ZeroThickness") && e.contains("y: -40.0"), "{e}");
}

/// Loops that touch are for the result to answer. A cut by such a profile whose touch lands in
/// empty space, here beyond the plate, leaves a sound part; a plate whose hole touches its outline
/// is refused above, and so is a boss whose outline pinches. The .fc parts:
///   extrude 4 rect -60 -25 120 80 / extrude cut 6 at 0 0 -1 circle 0 0 30 circle 0 -20 10   builds
///   extrude 4 rect -60 -60 120 120 / the same cut                                       builds
///   extrude 4 rect -60 -60 120 120 / extrude add 6 at 0 0 4 circle 0 0 30 circle 0 -20 10   ZeroThickness
#[test]
fn a_touching_profile_is_judged_by_its_result() {
    let pi = forge_math::dmath::PI;
    let v = builds(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (-60, -25), size: (120, 80))
}
extrude1 = extrude(s1, 4)
sketch d2(on: offset(xy, -1)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 30)
  c2 = circle((0, -20), 10)
}
cut1 = cut(s3, -6)
"#,
    );
    let lens = pi * 900.0 - pi * 100.0;
    assert!(
        v < 120.0 * 80.0 * 4.0 && v > 120.0 * 80.0 * 4.0 - lens * 4.0,
        "{v}"
    );
    builds(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (-60, -60), size: (120, 120))
}
extrude1 = extrude(s1, 4)
sketch d2(on: offset(xy, -1)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 30)
  c2 = circle((0, -20), 10)
}
cut1 = cut(s3, -6)
"#,
    );
    assert!(
        refusal(
            r#"sketch s1(on: xy) {
  r1 = rect(corner: (-60, -60), size: (120, 120))
}
extrude1 = extrude(s1, 4)
sketch d2(on: offset(xy, 4)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 30)
  c2 = circle((0, -20), 10)
}
extrude2 = extrude(s3, 6)
"#
        )
        .contains("ZeroThickness")
    );
}

/// A cut taking one whole body of several deletes it, on both paths; a cut taking every body
/// leaves no part, and is refused. (A Forge cut acts on the last body made, so the body it
/// takes is made last.)
#[test]
fn a_cut_taking_a_whole_body() {
    let b = built(
        r#"sketch sb(on: xy) {
  r = rect(corner: (40, 0), size: (10, 10))
}
b = extrude(sb, 8)
sketch sa(on: xy) {
  r = rect(corner: (0, 0), size: (10, 10))
}
a = extrude(sa, 8, new: true)
sketch sc(on: offset(xy, -1)) {
  r = rect(corner: (-1, -1), size: (12, 12))
}
gone = cut(sc, -10)
"#,
    );
    assert_eq!(b.solids.len(), 1);
    // the one left is the first block's, made by the history's first extrude
    let first = b
        .features
        .iter()
        .position(|f| matches!(f, forge_model::Feature::Extrude { .. }))
        .expect("an extrude");
    assert_eq!(b.seq().expect("seq").len(), 1);
    let solids = b.nary(1).expect("n-ary");
    assert_eq!(solids.len(), 1);
    assert_eq!(solids[0].id, first);
    assert!(
        refusal(
            r#"sketch sa(on: xy) {
  r = rect(corner: (0, 0), size: (10, 10))
}
a = extrude(sa, 8)
sketch sc(on: offset(xy, -1)) {
  r = rect(corner: (-1, -1), size: (60, 12))
}
gone = cut(sc, -10)
"#
        )
        .contains("Empty")
    );
}

/// The .fc part cut every body at once (`in all`):
///   a: extrude 8 rect 0 0 10 10
///   b: extrude new 8 rect 40 0 10 10
///   extrude cut 10 at 0 0 -1 rect -1 -1 12 12 in all       takes a whole, keeps b
///   extrude cut 10 at 0 0 -1 rect -1 -1 60 12 in all       takes both: refused
#[test]
#[ignore = "gap: a Forge cut acts on the last body only; it has no `bodies:` (.fc `extrude cut .. in all`)"]
fn a_cut_in_every_body() {
    let src = "sketch sa(on: xy) {
  r = rect(corner: (0, 0), size: (10, 10))
}
a = extrude(sa, 8)
sketch sb(on: xy) {
  r = rect(corner: (40, 0), size: (10, 10))
}
b = extrude(sb, 8, new: true)
sketch sc(on: offset(xy, -1)) {
  r = rect(corner: (-1, -1), size: (60, 12))
}
gone = cut(sc, -10, bodies: \"all\")
";
    assert!(refusal(src).contains("Empty"));
}

/// A boss near a rod's end whose fillet runs off over the rim and back: the rim is split, the
/// blend cuts the cap across the stretch that goes, and the wall's loop runs on through the rim's
/// closed vertex (volume agrees with OCCT to 1e-5).
#[test]
fn fillet_runs_over_a_rim_and_back() {
    let src = r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 17.7138)
}
extrude1 = extrude(s1, 51.4445)
sketch d2(on: offset(xy, 28.6203986907)) {
  p = pt(2.42676623008, -0.570108876966)
}
sketch d3(on: offset(xy, 0.401702400332)) {
  p = pt(0.249995868143, 0.880986519465)
}
sketch d4(on: offset(xy, -0.909877962311)) {
  p = pt(0, 0.41487599798)
}
sketch s5(on: plane(at: d2.p, normal: axis(o.p, d3.p), x: axis(o.p, d4.p))) {
  c1 = circle((0, 0), 11.5667)
}
extrude2 = extrude(s5, 35.7306)
round = fillet(edges(between: (extrude1, extrude2), crease: "concave"), 2.13853177)
"#;
    builds(src);
    let v = mass(src);
    assert!((v - 58260.8).abs() < 2.0, "{v}");
}

/// Taking every wall of a disc leaves its ends bounding nothing: refused, not a body of faces with
/// no loops.
#[test]
fn deleting_every_wall_is_refused() {
    assert!(
        refusal(
            r#"sketch s1(on: xy) {
  c1 = circle((0, 0), 20)
  c2 = circle((0, 0), 12)
}
extrude1 = extrude(s1, 20)
fill = delete_faces(faces(of: extrude1, shape: "cyl"))
"#
        )
        .contains("CannotHeal")
    );
}

/// A cone with a hexagon cut through it, its top edges rounded: no panic, and sound if it builds.
#[test]
fn rounding_a_hex_cut_cone_top_does_not_panic() {
    let src = r#"sketch s1(on: xz) {
  ax = line((0, 0), (0, 1), construction: true)
  p1 = poly([(0, 0), (20, 0), (10, 20), (0, 20)])
}
cone = revolve(s1, axis: s1.ax)
sketch top(on: offset(xy, 200)) {
  hex = poly([(20, 0), (10, 17.320508), (-10, 17.320508), (-20, 0), (-10, -17.320508), (10, -17.320508)])
}
hexcut = cut(top, thru)
round = fillet(edges(between: (hexcut, cone.side(s1.p1[1]))), 0.5)
"#;
    let (r, _) = common::run(src);
    // (it builds since corner patches stopped following storage order; a refusal must name the fillet)
    match r.errors.first() {
        None => assert!(r.measure.expect("measured").problems == 0),
        Some(e) => assert!(format!("{e:?}").contains("round"), "{e:?}"),
    }
}

/// A slot through a plate whose straight side is two lines a hair off one line, as float noise
/// leaves it: where the plate's face crosses them, the neighbour across their edge meets at so
/// shallow an angle that the crossing was lost. The edge's own plane finds it.
#[test]
fn a_kink_of_a_hair_in_a_slots_side() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  r1 = rect(corner: (-20, 0), size: (40, 40))
}
extrude1 = extrude(s1, two(2.4, 2.4), new: true)
sketch d2(on: offset(xy, 0)) {
  p = pt(0, -1)
}
sketch s3(on: plane(at: d2.p, normal: -Y, x: X)) {
  l1 = line((2.5, -34), (2.499985, -21.5))
  l2 = line((2.499985, -21.5), (2.5, -9))
  a3 = arc(center: (0, -9), from: (2.5, -9), to: (-2.5, -9))
  l4 = line((-2.5, -9), (-2.5, -21.5))
  l5 = line((-2.5, -21.5), (-2.5, -34))
  a6 = arc(center: (0, -34), from: (-2.5, -34), to: (2.5, -34))
}
cut1 = cut(s3, -12)  # in all
"#,
    );
}

/// A tube with a triangular prism cut, its slanted face passing exactly through the tube's far rim:
/// the ellipse where it meets the wall only touches the rim there, one cut on a closed curve, and
/// the whole loop was taken to run along the boundary. The cut lost a tenth of itself, silently.
#[test]
fn an_ellipse_touching_a_rim_still_cuts() {
    let src = r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 8)
  c2 = circle((0, 0), 6)
}
extrude1 = extrude(s1, 150)
sketch s2(on: plane(at: o.p, normal: Z, x: X)) {
  p1 = poly([(8, 150), (-8, 150), (8, 135)])
}
cut1 = cut(s2, two(-9, -9))
"#;
    let v = builds(src);
    let exact = mass(src);
    assert!((v - exact).abs() < 1e-2 * exact, "{v} {exact}");
    // the same a hair off, where the ellipse crosses the rim
    let near = mass(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 8)
  c2 = circle((0, 0), 6)
}
extrude1 = extrude(s1, 150)
sketch s2(on: plane(at: o.p, normal: Z, x: X)) {
  p1 = poly([(8.0000002, 150.0000002), (-8.0000002, 150.0000002), (8.0000002, 135)])
}
cut1 = cut(s2, two(-9, -9))
"#,
    );
    assert!((near - exact).abs() < 1e-3 * exact, "{near} {exact}");
}

/// A ball whose equator lies on a boss's top rim, its lower half inside: a dome on the boss, smooth
/// at the rim, one valid solid. Main refused it as "no closed solid" over the boss's foot.
#[test]
fn a_ball_tangent_to_a_boss_rim_is_a_dome() {
    let v = builds(
        r#"sketch s1(on: xy) {
  c1 = circle((0, 0), 10)
}
extrude1 = extrude(s1, 8)
sketch d2(on: offset(xy, 8)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 4)
}
extrude2 = extrude(s3, 8)
sketch d4(on: offset(xy, 16)) {
  p = pt(0, 0)
}
sketch s5(on: plane(at: d4.p, normal: Z, x: X)) {
  c = pt(0, 0)
}
ball1 = ball(s5.c, 4)
"#,
    );
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
            "sketch s1(on: xy) {{
  r1 = rect(corner: (0, 0), size: (60, 40))
}}
extrude1 = extrude(s1, 10, new: true)
sketch d2(on: offset(xy, 10)) {{
  p = pt(17.98, 23.83)
}}
sketch s3(on: plane(at: d2.p, normal: Y, x: X)) {{
  c = pt(0, 0)
}}
ball1 = ball(s3.c, 3.6, cut: true)
sketch d4(on: offset(xy, 10)) {{
  p = pt(17.98, {y})
}}
sketch s5(on: plane(at: d4.p, normal: Y, x: X)) {{
  c = pt(0, 0)
}}
ball2 = ball(s5.c, 3.6)
"
        )
    };
    builds(&part(31.03));
    for y in [31.03 + 1e-12, 31.03 - 1e-12, 31.03 + 3e-13, 31.03 - 3e-13] {
        let (r, _) = common::run(&part(y));
        match r.errors.first() {
            Some(e) => {
                let e = format!("{e:?}");
                assert!(e.contains("ZeroThickness") || e.contains("thick"), "y {y}: {e}")
            }
            None => assert_eq!(r.measure.expect("measured").problems, 0, "y {y}"),
        }
    }
}

/// A full round whose side face keeps edges it no longer holds: refused, not a body 5 off its face
/// (repros/check-edge-off-face.fc). The .fc part, a fuzzed one whose picks are by place:
///   L0: extrude 8.52 rect 0 0 53.78 114.36
///   L1: split plane z 0.951 0.258 0.172 at 37.9 60.16 6.25 +
///   L2: movebody by 35.98 2.3 -10.61 rot 38.28 57.24 4.26 0 0 1 42.91
///   L3: fillet 3.04..2.35 from (verts near 35.53 135.81 -10.61) (edges near 61.7 86.03 -10.61) (edges convex near 24.74 125.78 -10.61)
///   L5: movebody by 158.74 119.01 26.09
///   L8: sweep cut z 0 0 -1 x -1 0 0 at 221.81 208.74 99.68 rrect -12.99 -12.99 25.98 25.98 3.9 path line 221.81 208.74 99.68 221.81 208.74 10.52000000000001 arc z 0 1 0 x 1 0 0 at 192.41 208.74 10.52000000000001 29.4 0 30 line 217.87114687126248 208.74 -4.179999999999987 186.24614687126248 208.74 -58.956106789365734
///   L11: extrude add 78.13 z 0.826 0.52 0.217 at 263.16 180.65 20.27 loop line -30.04 0 30.04 0 arc 0 0 30.04 0 180
///   L12: fillet 0.06 (edges convex circle of L8)
///   L17: offset -4.58 (faces near 209.64 238.28 19.73)
///   L19: hole 21.99 z 0 0 -1 at 162.07 252.77 37.23 34.89 pt 118 thread 26.39 3.5
///   L25: fullround (faces near 195.14 231.14 24) (faces near 204.84 236.72 19.73) (faces near 198.43 232.31 15.48)
#[test]
#[ignore = "gap: not yet written in Forge: its picks are by place (`near X Y Z`) after two body moves and a split, which Forge's selections (by role, never by place) name only once each face is traced to the feature that made it"]
fn full_round_leaving_an_edge_off_its_face_is_refused() {
    panic!("the .fc part above, to be written as a Forge part");
}

/// Rays past a lemon must not count the apple its quartic also holds: a point in a corner of the
/// box was inside (WHUCAD 100003006, one point in 13).
#[test]
fn classify_beside_a_lemon() {
    let body = built(
        r#"sketch sec(on: xz) {
  a = arc(center: (-5, 0), from: (0, -12), to: (0, 12))
  l = line((0, 12), (0, -12))
}
lemon = revolve(sec, axis: sec.l)
"#,
    )
    .body
    .compact();
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
#[test]
fn arcs_ending_on_the_axis() {
    let props = |src: &str| {
        let b = built(src);
        assert!(
            forge_topo::check(&b.body).is_empty(),
            "{:?}",
            forge_topo::check(&b.body)
        );
        forge_topo::mass::mass(&b.body.compact()).expect("a mass")
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
    for (src, cut, rc, half) in [
        (
            r#"sketch sec(on: xz) {
  a = arc(center: (-5, 0), from: (0, -12), to: (0, 12))
  l = line((0, 12), (0, -12))
}
lemon = revolve(sec, axis: sec.l)
"#,
            r#"sketch sec(on: xz) {
  a = arc(center: (-5, 0), from: (0, -12), to: (0, 12))
  l = line((0, 12), (0, -12))
}
lemon = revolve(sec, axis: sec.l)
sketch under(on: offset(xy, -30)) {
  p = pt(0, 0)
}
sketch rod(on: plane(at: under.p, normal: -Z, x: X)) {
  c = circle((3, 0), 2)
}
bored = cut(rod, thru)
"#,
            -5.0,
            a,
        ),
        (
            r#"sketch s1(on: xz) {
  ax = line((0, 0), (0, 1), construction: true)
  a1 = arc(center: (5, 0), from: (0, -12), to: (0, 12))
  l2 = line((0, 12), (0, -12))
}
revolve1 = revolve(s1, axis: s1.ax)
"#,
            r#"sketch sec(on: xz) {
  a = arc(center: (5, 0), from: (0, -12), to: (0, 12))
  l = line((0, 12), (0, -12))
}
apple = revolve(sec, axis: sec.l)
sketch under(on: offset(xy, -30)) {
  p = pt(0, 0)
}
sketch rod(on: plane(at: under.p, normal: -Z, x: X)) {
  c = circle((3, 0), 2)
}
bored = cut(rod, thru)
"#,
            5.0,
            std::f64::consts::PI - a,
        ),
    ] {
        let m = props(src);
        let (v, area) = revolved(rc, half);
        assert!((m.volume - v).abs() < 1e-5 * v, "{} vs {v}", m.volume);
        assert!((m.area - area).abs() < 1e-5 * area, "{} vs {area}", m.area);
        // a cut along the axis builds now, judged by the definition in kernel_spindle_cuts.rs
        builds(cut);
    }
    // a dome on a flange, the arc's centre off the axis but not reaching it
    props(
        r#"sketch s1(on: xz) {
  ax = line((0, 0), (0, 1), construction: true)
  l1 = line((0, -3), (15, -3))
  l2 = line((15, -3), (15, 0))
  a3 = arc(center: (2, 0), from: (15, 0), to: (2.000000000000001, 13))
  l4 = line((2.000000000000001, 13), (0, 13))
  l5 = line((0, 13), (0, -3))
}
revolve1 = revolve(s1, axis: s1.ax)
"#,
    );
    // a whole circle tangent to the axis would pinch there
    assert!(
        refusal(
            r#"sketch s1(on: xz) {
  ax = line((0, 0), (0, 1), construction: true)
  c1 = circle((4, 0), 4)
}
revolve1 = revolve(s1, axis: s1.ax)
"#
        )
        .contains("pinches")
    );
    // a profile whose corner only touches it builds if what it makes closes, or is refused
    let (r, _) = common::run(
        r#"sketch s1(on: xz) {
  ax = line((0, 0), (0, 1), construction: true)
  l1 = line((0, 0), (6, 0))
  l2 = line((6, 0), (6, 2))
  l3 = line((6, 2), (0, 0))
}
revolve1 = revolve(s1, axis: s1.ax)
"#,
    );
    match r.errors.first() {
        Some(e) => assert!(format!("{e:?}").contains("pinches"), "{e:?}"),
        None => assert_eq!(r.measure.expect("measured").problems, 0),
    }
    // as a cut it is a tool, and what it leaves is sound: a cone standing on the flat (WHUCAD 100002292)
    let v = builds(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (-10, -10), size: (20, 20))
}
extrude1 = extrude(s1, 4)
sketch s2(on: xz) {
  ax = line((0, 0), (0, 1), construction: true)
  l1 = line((0, 0), (6, 0))
  l2 = line((6, 0), (6, 2))
  l3 = line((6, 2), (0, 0))
}
revolve1 = revolve(s2, axis: s2.ax, cut: true)
"#,
    );
    assert!((v - (1600.0 - 48.0 * std::f64::consts::PI)).abs() < 2.0, "{v}");
    // not on part of a turn: a wedge's point
    builds(
        r#"sketch s1(on: xz) {
  ax = line((0, 0), (0, 1), construction: true)
  l1 = line((0, 0), (6, 0))
  l2 = line((6, 0), (6, 2))
  l3 = line((6, 2), (0, 0))
}
revolve1 = revolve(s1, axis: s1.ax, angle: 90deg)
"#,
    );
}

/// Points along the axis between axis lines are on nothing a whole turn makes: they were left as
/// loose vertices.
#[test]
fn a_point_between_two_lines_on_the_axis() {
    builds(
        r#"sketch s1(on: xz) {
  ax = line((0, 0), (0, 1), construction: true)
  l1 = line((0, 0), (5, 0))
  l2 = line((5, 0), (5, 3))
  l3 = line((5, 3), (0, 3))
  l4 = line((0, 3), (0, 1))
  l5 = line((0, 1), (0, 0))
}
revolve1 = revolve(s1, axis: s1.ax)
"#,
    );
}

/// A cut through everything reaches past all the material there can be: lofts, sweeps and coils
/// were left out of that, so it went a millimetre into them, valid and wrong.
#[test]
fn a_cut_through_a_loft_or_a_sweep_goes_through() {
    let pi = std::f64::consts::PI;
    let frustum = 15.0 * (40.0f64 * 40.0 + 40.0 * 16.0 + 16.0 * 16.0) / 3.0;
    let v = builds(
        r#"sketch p0(on: xy) {
  r = rect(corner: (-20, -20), size: (40, 40))
}
sketch p1(on: offset(xy, 15)) {
  r = rect(corner: (-8, -8), size: (16, 16))
}
frustum = loft([p0, p1], smooth: true)
sketch top(on: p1.plane) {
  c = circle((0, 0), 4)
}
bore = cut(top, thru)
"#,
    );
    let want = frustum - pi * 16.0 * 15.0;
    assert!((v - want).abs() < 2e-3 * want, "loft {v} vs {want}");
    let v = builds(
        r#"sketch s(on: xy) {
  r = rect(corner: (-20, -20), size: (40, 40))
}
sketch rail(on: xz) {
  l = line((0, 0), (0, 15))
}
block = sweep(s, along: rail.l)
sketch top(on: offset(xy, 15)) {
  p = pt(0, 0)
}
bore = hole(at: top.p, d: 8, depth: thru)
"#,
    );
    let want = 1600.0 * 15.0 - pi * 16.0 * 15.0;
    assert!((v - want).abs() < 2e-3 * want, "sweep {v} vs {want}");
}

/// The box an arc in a tilted plane reports is the arc's own, not the bound of it: a sweep over a
/// long arc read 0.85 too tall (volume and area right all along).
#[test]
fn the_box_of_a_tilted_arc_is_tight() {
    let b = built(
        r#"sketch sec(on: xz) {
  c = circle((0, 0), 10)
}
sketch rail(on: yz) {
  path route(start: (0, 0)) {
    line(to: (40, 0))
    tarc(to: (68.92544243589427, 10.52800005964599))
    tarc(to: (107.4926990170784, 10.528000059652879))
    tarc(to: (185.4849201222435, -42.0784173232596))
    line(to: (198.729134015262, -49.120490765119))
  }
}
tube = sweep(sec.c, along: rail.route)
"#,
    )
    .body
    .compact();
    // the part's box as the app reads it: the mesh's points (a dome's top) and its edges' own boxes
    let mesh = tessellate(&b, &Quality::default()).expect("meshes");
    let mut z = mesh.positions.iter().map(|p| p.z).fold(f64::MIN, f64::max);
    for e in b.edge_ids() {
        let hi = match &b.edge(e).curve {
            forge_geom::Curve::Line(_) => b.edge_box(e).z.hi,
            forge_geom::Curve::Circle(c) => c.tight_box(b.edge(e).range).z.hi,
            _ => b.edge_points(e, 64).iter().map(|p| p.z).fold(f64::MIN, f64::max),
        };
        z = z.max(hi);
    }
    // the second arc's centre, up its radius, and the profile's
    let top = -12.453333233923 + 30.0 + 10.0;
    assert!((z - top).abs() < 0.03, "{z} vs {top}");
}

/// Equal crossing holes, the second's wall tangent to the first's cap: the curve touches the cap's rim
/// a few 1e-15 off it, so the closed form saw no root, and the rim got no vertex (FaceWrongSide on the
/// wall, on a body returned as good).
#[test]
fn a_conic_touching_a_rim_within_rounding_gets_a_vertex() {
    let pi = std::f64::consts::PI;
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: -Y, x: X)) {
  r1 = rect(corner: (-8, -5), size: (16, 10))
}
extrude1 = extrude(s1, 14)
sketch d2(on: offset(xy, 0)) {
  p = pt(0, -14)
}
sketch s3(on: plane(at: d2.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 3.5)
}
cut1 = cut(s3, -10.5)
sketch d4(on: offset(xy, 0)) {
  p = pt(-9, 0)
}
sketch s5(on: plane(at: d4.p, normal: X, x: -Y)) {
  c1 = circle((7, 0), 3.5)
}
cut2 = cut(s5, -12.15)
"#,
    );
    assert!(v < 16.0 * 10.0 * 14.0 - pi * 3.5 * 3.5 * 10.5, "{v}");
    // a tab whose end edge lies tangent to a rod: the circle touches the tab's slanted faces
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: -Y, x: X)) {
  c1 = circle((0, 0), 2)
}
extrude1 = extrude(s1, sym(60))
sketch s2(on: plane(at: o.p, normal: X, x: Y)) {
  l1 = line((-20, 2), (-48.705904774487394, 4.8296291314453415))
  a2 = arc(center: (-50, 0), from: (-48.705904774487394, 4.8296291314453415), to: (-48.7059047744874, -4.829629131445342))
  l3 = line((-48.7059047744874, -4.829629131445342), (-20, -2))
  l4 = line((-20, -2), (-20, 2))
}
extrude2 = extrude(s2, sym(4))
"#,
    );
}

/// A body of several shells: what a boolean added all went to the first, whichever shell it joined.
/// The solid was right and the shells wrong (ShellEuler on a void's pocket, ShellsOverlap on a lump).
#[test]
fn what_a_boolean_adds_joins_its_own_shell() {
    let pi = std::f64::consts::PI;
    // a pocket in the floor of a void
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 20)
}
extrude1 = extrude(s1, 40)
sketch d2(on: offset(xy, 10)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 15)
}
cut1 = cut(s3, -20)
sketch d4(on: offset(xy, 5)) {
  p = pt(5, 0)
}
sketch s5(on: plane(at: d4.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 5)
}
cut2 = cut(s5, -10)
"#,
    );
    let want = pi * (400.0 * 40.0 - 225.0 * 20.0 - 25.0 * 5.0);
    assert!((v - want).abs() < 2e-3 * want, "{v} vs {want}");
    // a hole through two separate discs
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: -Y, x: X)) {
  c1 = circle((0, 0), 10)
}
extrude1 = extrude(s1, 1)
sketch d2(on: offset(xy, 0)) {
  p = pt(0, -10)
}
sketch s3(on: plane(at: d2.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 25)
}
extrude2 = extrude(s3, 0.5)
sketch s4(on: plane(at: d2.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 6)
}
cut1 = cut(s4, sym(25))
"#,
    );
    let want = pi * (100.0 - 36.0 + 0.5 * (625.0 - 36.0));
    assert!((v - want).abs() < 2e-3 * want, "{v} vs {want}");
    // a slot parting a bar in one cut
    builds(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (40, 10))
}
extrude1 = extrude(s1, 10)
sketch s2(on: xy) {
  r1 = rect(corner: (18, -5), size: (4, 20))
}
cut1 = cut(s2, sym(30))
sketch d3(on: offset(xy, 7)) {
  p = pt(5, 5)
}
sketch s4(on: plane(at: d3.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 1)
}
cut2 = cut(s4, -3)
sketch d5(on: offset(xy, 7)) {
  p = pt(35, 5)
}
sketch s6(on: plane(at: d5.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 1)
}
cut3 = cut(s6, -3)
"#,
    );
}

/// A half disc starting flush with a block's face and crossing it further round: the flush edge is
/// named as the cut it would be, and the new edge is that cut (DuplicateName).
#[test]
fn a_flush_edge_named_as_a_cut_that_is_there() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch d1(on: offset(xy, -10)) {
  p = pt(-60, -30)
}
sketch s2(on: plane(at: d1.p, normal: X, x: Y)) {
  r1 = rect(corner: (0, 0), size: (60, 30))
}
extrude1 = extrude(s2, 60)
sketch s3(on: plane(at: o.p, normal: X, x: Y)) {
  ax = line((0, 0), (0, -1), construction: true)
  r1 = rect(corner: (0, 0), size: (12, 8))
}
revolve1 = revolve(s3, axis: s3.ax, angle: 185deg)
"#,
    );
}

/// A piece of a wall split where the wall's seam bounds it, across that seam from a piece a cut
/// named by it: they came to one name (DuplicateName). A pocket through a tube's rim, and a ring
/// that fills part of it.
#[test]
fn pieces_across_a_seam_are_named_apart() {
    let src = r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 67)
}
extrude1 = extrude(s1, 25)
sketch s2(on: plane(at: o.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 30)
}
cut1 = cut(s2, -20)
sketch s3(on: plane(at: o.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 30)
}
cut2 = cut(s3, -20)
sketch d4(on: offset(xy, 0)) {
  p = pt(-60, -20)
}
sketch s5(on: plane(at: d4.p, normal: Z, x: X)) {
  c1 = circle((0, 60), 30)
}
cut3 = cut(s5, -20)
sketch d6(on: offset(xy, 0)) {
  p = pt(-50, 0)
}
sketch s7(on: plane(at: d6.p, normal: Z, x: X)) {
  c1 = circle((50, 0), 47)
  c2 = circle((50, 0), 30)
}
extrude2 = extrude(s7, 20)
"#;
    for (i, stage) in built(src).stages().iter().enumerate() {
        for s in stage {
            let p = forge_topo::check(&s.body);
            assert!(p.is_empty(), "feature {i}: {p:?}");
        }
    }
    builds(src);
}

/// Two opened faces side by side: the rims end along the edge they share. A square tube's corner
/// cut open, against its closed form.
#[test]
fn a_shell_with_two_opened_faces_side_by_side() {
    let v = builds(
        r#"sketch s(on: xy) {
  r = rect(corner: (-10, -10), size: (20, 20))
}
b = extrude(s, 16)
tube = shell(2, open: b.end + b.side(s.r[1]))
"#,
    );
    let want = 20.0 * 20.0 * 16.0 - 18.0 * 16.0 * 14.0;
    assert!((v - want).abs() < 1e-3 * want, "{v} vs {want}");
}

/// A block with every edge rounded, its top opened: the walls leave the top tangentially, so the
/// offsets never reach it, and they end in strips square to it, one wall thick.
#[test]
fn a_shell_opening_a_face_its_neighbours_leave_tangentially() {
    let v = builds(
        r#"sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (100, 60))
}
b = extrude(s, 40)
round = fillet(edges(of: b), 5)
cup = shell(2, open: b.end)
"#,
    );
    // the skin's area less the opening, times the wall, about
    assert!(v > 0.0, "{v}");
    // a rod's rounded end, its other end opened: the circle between tangent faces moves with them
    let pi = std::f64::consts::PI;
    let v = builds(
        r#"sketch s(on: xy) {
  c = circle((0, 0), 40)
}
rod = extrude(s, 30)
round = fillet(edges(of: rod.end), 20)
cup = shell(5, open: rod.start)
"#,
    );
    assert!(v > 0.0 && v < pi * 1600.0 * 30.0 * 0.6, "{v}");
}

/// A varying radius over a chain runs from the chain's lowest free end, in whatever order the edges
/// were given and whatever they are named.
#[test]
fn a_law_over_a_chain_starts_at_its_lowest_free_end() {
    let a = mass(
        r#"sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (100, 40))
}
b = extrude(s, 30)
round = fillet(edges(between: (b.start, b.side(s.r[1]))) + edges(between: (b.start, b.side(s.r[0]))), vary(4, 2))
"#,
    );
    let b = mass(
        r#"sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (100, 40))
}
b = extrude(s, 30)
round = fillet(edges(between: (b.start, b.side(s.r[0]))) + edges(between: (b.start, b.side(s.r[1]))), vary(4, 2))
"#,
    );
    assert_eq!(a.to_bits(), b.to_bits());
    let c = mass(
        r#"sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (100, 40))
}
b = extrude(s, 30)
round = fillet(edges(between: (b.start, b.side(s.r[1]))) + edges(between: (b.start, b.side(s.r[2]))), vary(4, 2))
"#,
    );
    let d = mass(&r#"sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (100, 40))
}
b = extrude(s, 30)
round = fillet(edges(between: (b.start, b.side(s.r[1]))) + edges(between: (b.start, b.side(s.r[2]))), vary(4, 2))
"#.replace(
        "edges(between: (b.start, b.side(s.r[1]))) + edges(between: (b.start, b.side(s.r[2])))",
        "edges(between: (b.start, b.side(s.r[2]))) + edges(between: (b.start, b.side(s.r[1])))",
    ));
    assert_eq!(c.to_bits(), d.to_bits());
}

/// To the next face backwards, as forwards with the frame flipped: a floor, a wall and a roof, and a
/// block from mid air down to the floor or up to the roof. (Forge's add goes forwards only, so the
/// block down is drawn on a flipped plane; the .fc `extrude next back xy at 0 0 25 ..` gave the
/// same.)
#[test]
fn to_next_backwards() {
    let full = 60.0 * 60.0 * 10.0 * 2.0 + 5.0 * 60.0 * 40.0;
    let flip = builds(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (-30, -30), size: (60, 60))
}
extrude1 = extrude(s1, 10)
sketch d2(on: offset(xy, 10)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  r1 = rect(corner: (-30, -30), size: (5, 60))
}
extrude2 = extrude(s3, 50)
sketch d4(on: offset(xy, 50)) {
  p = pt(0, 0)
}
sketch s5(on: plane(at: d4.p, normal: Z, x: X)) {
  r1 = rect(corner: (-30, -30), size: (60, 60))
}
extrude3 = extrude(s5, 10)
sketch d6(on: offset(xy, 25)) {
  p = pt(0, 0)
}
sketch s7(on: plane(at: d6.p, normal: -Z, x: X)) {
  r1 = rect(corner: (-5, -5), size: (10, 10))
}
extrude4 = extrude(s7, next)
"#,
    );
    assert!((flip - (full + 100.0 * 15.0)).abs() < 1e-3 * flip, "{flip}");
    let up = builds(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (-30, -30), size: (60, 60))
}
extrude1 = extrude(s1, 10)
sketch d2(on: offset(xy, 10)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  r1 = rect(corner: (-30, -30), size: (5, 60))
}
extrude2 = extrude(s3, 50)
sketch d4(on: offset(xy, 50)) {
  p = pt(0, 0)
}
sketch s5(on: plane(at: d4.p, normal: Z, x: X)) {
  r1 = rect(corner: (-30, -30), size: (60, 60))
}
extrude3 = extrude(s5, 10)
sketch d6(on: offset(xy, 25)) {
  p = pt(0, 0)
}
sketch s7(on: plane(at: d6.p, normal: Z, x: X)) {
  r1 = rect(corner: (-5, -5), size: (10, 10))
}
extrude4 = extrude(s7, next)
"#,
    );
    assert!((up - (full + 100.0 * 25.0)).abs() < 1e-3 * up, "{up}");
    // a cut from inside, backwards up to the floor's top
    let cut = builds(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (-30, -30), size: (60, 60))
}
extrude1 = extrude(s1, 10)
sketch d2(on: offset(xy, 10)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  r1 = rect(corner: (-30, -30), size: (5, 60))
}
extrude2 = extrude(s3, 50)
sketch d4(on: offset(xy, 50)) {
  p = pt(0, 0)
}
sketch s5(on: plane(at: d4.p, normal: Z, x: X)) {
  r1 = rect(corner: (-30, -30), size: (60, 60))
}
extrude3 = extrude(s5, 10)
sketch d6(on: offset(xy, 5)) {
  p = pt(0, 0)
}
sketch s7(on: plane(at: d6.p, normal: Z, x: X)) {
  r1 = rect(corner: (-5, -5), size: (10, 10))
}
cut1 = cut(s7, next)
"#,
    );
    assert!((cut - (full - 100.0 * 5.0)).abs() < 1e-3 * cut, "{cut}");
}

/// To the next face in a part of several bodies: of whichever body comes first along the rays. (A
/// Forge extrude adds to the last body made, so the body it adds to is made last.)
#[test]
fn to_next_among_bodies() {
    let b = built(
        r#"sketch d1(on: offset(xy, 40)) {
  p = pt(0, 0)
}
sketch s2(on: plane(at: d1.p, normal: Z, x: X)) {
  r1 = rect(corner: (-30, -30), size: (60, 60))
}
b = extrude(s2, 10, new: true)
sketch s3(on: xy) {
  r1 = rect(corner: (-30, -30), size: (60, 60))
}
a = extrude(s3, 10, new: true)
sketch d4(on: offset(xy, 10)) {
  p = pt(0, 0)
}
sketch s5(on: plane(at: d4.p, normal: Z, x: X)) {
  r1 = rect(corner: (-5, -5), size: (10, 10))
}
extrude3 = extrude(s5, next)
"#,
    );
    let total = |solids: Vec<forge_model::Solid>| -> f64 {
        solids
            .iter()
            .map(|s| forge_topo::mass::mass(&s.body).unwrap().volume)
            .sum()
    };
    let want = 2.0 * 36000.0 + 100.0 * 30.0;
    let v = total(b.seq().expect("seq"));
    assert!((v - want).abs() < 1e-6 * want, "{v}");
    let v = total(b.nary(1).expect("n-ary"));
    assert!((v - want).abs() < 1e-6 * want, "{v}");
}

/// A tilted cutter whose rim hovers within tolerance of a part's bottom over a chord: under that stretch
/// the curve where its wall meets the bottom is a new edge all the same, or being in the tool flips across
/// the rim ("inconsistent classification"). The slit between chord and curve is a hair wide.
#[test]
fn a_tilted_cutter_hovering_over_a_face() {
    let src = r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 0.4)
}
extrude1 = extrude(s1, 0.6)
sketch d2(on: offset(xy, 0.50515927)) {
  p = pt(0, 0)
}
sketch d3(on: offset(xy, -0.9998476952)) {
  p = pt(0.0174524064, 0)
}
sketch d4(on: offset(xy, 0.0174524064)) {
  p = pt(0.9998476952, 0)
}
sketch s5(on: plane(at: d2.p, normal: axis(o.p, d3.p), x: axis(o.p, d4.p))) {
  c1 = circle((0, 0), 0.3)
}
cut1 = cut(s5, -0.5)
"#;
    builds(src);
    let v = mass(src);
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
    for (src, scale) in [
        (
            r#"sketch s1(on: xy) {
  r1 = rect(corner: (-0.3, -0.3), size: (0.6, 0.6))
}
extrude1 = extrude(s1, 0.6)
sketch d2(on: offset(xy, 0.6)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: X, x: Y)) {
  ax = line((0, 0.3), (0, -0.7), construction: true)
  l1 = line((0, 0), (0.3, 0))
  a2 = arc(center: (0, 0), from: (0.3, 0), to: (1.8369701987210297e-17, 0.3))
  l3 = line((1.8369701987210297e-17, 0.3), (0, 0))
}
revolve1 = revolve(s3, axis: s3.ax)
"#,
            1.0,
        ),
        (
            r#"sketch s1(on: xy) {
  r1 = rect(corner: (-12, -12), size: (24, 24))
}
extrude1 = extrude(s1, 24)
sketch d2(on: offset(xy, 24)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: X, x: Y)) {
  ax = line((0, 12), (0, 11), construction: true)
  l1 = line((0, 0), (12, 0))
  a2 = arc(center: (0, 0), from: (12, 0), to: (7.347880794884119e-16, 12))
  l3 = line((7.347880794884119e-16, 12), (0, 0))
}
revolve1 = revolve(s3, axis: s3.ax)
"#,
            40.0,
        ),
    ] {
        let (a, r): (f64, f64) = (0.6 * scale, 0.3 * scale);
        builds(src);
        let want = a * a * a + 2.0 * pi * r * r * r / 3.0;
        let got = mass(src);
        assert!((got - want).abs() < 1e-6 * want, "{got} {want}");
    }
}

/// A profile leaving the axis and coming back: the disc shaped void inside is a shell of its own.
#[test]
fn a_revolve_closing_a_void_on_its_axis() {
    let pi = std::f64::consts::PI;
    let v = builds(
        r#"sketch s1(on: xz) {
  ax = line((0, 0), (0, 1), construction: true)
  l1 = line((0, 0), (8, 0))
  l2 = line((8, 0), (8, 30))
  l3 = line((8, 30), (0, 30))
  l4 = line((0, 30), (0, 20))
  l5 = line((0, 20), (4, 20))
  l6 = line((4, 20), (4, 10))
  l7 = line((4, 10), (0, 10))
  l8 = line((0, 10), (0, 0))
}
revolve1 = revolve(s1, axis: s1.ax)
"#,
    );
    let want = pi * (64.0 * 30.0 - 16.0 * 10.0);
    assert!((v - want).abs() < 3e-3 * want, "{v} {want}");
}

/// A revolve's axis drawn as a line in its sketch is no part of the profile (in Forge, a
/// construction line); one of the profile's own edges stays one.
#[test]
fn a_revolve_axis_drawn_as_a_line_or_an_edge_of_the_profile() {
    let pi = std::f64::consts::PI;
    let v = mass(
        r#"sketch sk(on: xz) {
  r = rect(corner: (10, 0), size: (10, 20))
  l = line((0, 0), (0, 30), construction: true)
}
turned = revolve(sk, axis: sk.l)
"#,
    );
    assert!((v - pi * 300.0 * 20.0).abs() < 1e-6 * v, "{v}");
    // a plain line left in the sketch is an open chain, said so
    let (r, _) = common::run(
        r#"sketch sk(on: xz) {
  r = rect(corner: (10, 0), size: (10, 20))
  l = line((0, 0), (0, 30))
  ax = line((0, 0), (0, 1), construction: true)
}
turned = revolve(sk, axis: sk.ax)
"#,
    );
    let e = format!("{:?}", r.errors.first().expect("refused"));
    assert!(e.contains("OpenChain"), "{e}");
    let v = mass(
        r#"sketch sk(on: xz) {
  r = rect(corner: (10, 0), size: (10, 20))
}
turned = revolve(sk.r, axis: sk.r[3], angle: 90deg)
"#,
    );
    assert!((v - pi * 100.0 * 20.0 / 4.0).abs() < 1e-6 * v, "{v}");
}

/// A cut that takes a whole body away, then an add: it goes to the body left, as sequential does; one
/// run made the add's tool a body of its own (Fusion 51895_0007).
#[test]
fn an_add_after_a_cut_emptied_a_body_goes_to_the_body_left() {
    let b = built(
        r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (100, 100))
}
extrude1 = extrude(s1, 34)
sketch d2(on: offset(xy, 40)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  r1 = rect(corner: (10, 10), size: (50, 50))
}
extrude2 = extrude(s3, 2, new: true)
sketch s4(on: plane(at: d2.p, normal: Z, x: X)) {
  r1 = rect(corner: (10, 10), size: (50, 50))
}
cut1 = cut(s4, -2)
sketch d5(on: offset(xy, 50)) {
  p = pt(0, 0)
}
sketch s6(on: plane(at: d5.p, normal: Z, x: X)) {
  r1 = rect(corner: (0, 0), size: (10, 10))
}
extrude3 = extrude(s6, 2)
"#,
    );
    let nary = b.nary(1).expect("n-ary");
    assert_eq!(nary.len(), b.solids.len());
    assert_eq!(nary.len(), 1);
}

/// A cut nearly tangent to a blend: a step of the chord walk landed on the far side of the tube, the
/// face's integral never settled, and its bound said 10% (fuzz p09444). This one settles on what the
/// mesh says.
#[test]
fn a_cut_nearly_tangent_to_a_blend_has_a_volume() {
    let b = built(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 18.6873)
}
body = extrude(s1, 33.8538)
sketch d2(on: offset(xy, 14.4425234406)) {
  p = pt(-4.34617495356, 1.52942340372)
}
sketch d3(on: offset(xy, -0.446924171828)) {
  p = pt(-0.372619194143, -0.813273460032)
}
sketch d4(on: offset(xy, 0.876387051554)) {
  p = pt(0, -0.481607449972)
}
sketch s5(on: plane(at: d2.p, normal: axis(o.p, d3.p), x: axis(o.p, d4.p))) {
  c1 = circle((0, 0), 10.5378)
}
boss = extrude(s5, 34.7602)
round = fillet(edges(between: (body, boss), crease: "concave"), 1.71121082)
sketch d6(on: offset(xy, 5.35835986907)) {
  p = pt(-21.1807704343, -6.15178668063)
}
sketch d7(on: offset(xy, 0.535915248777)) {
  p = pt(-0.234138759071, -0.811155896008)
}
sketch d8(on: offset(xy, 0.834348112456)) {
  p = pt(0, 0.551237904395)
}
sketch s9(on: plane(at: d6.p, normal: axis(o.p, d7.p), x: axis(o.p, d8.p))) {
  r1 = rect(corner: (-87.6754, -87.6754), size: (175.351, 175.351))
}
cut0 = cut(s9, -43.8377)
sketch d10(on: offset(xy, -2.5085832624)) {
  p = pt(19.4999129356, -20.4673606945)
}
sketch d11(on: offset(xy, 0.179456179902)) {
  p = pt(-0.927984340469, 0.326558636907)
}
sketch d12(on: offset(xy, -0.981810174001)) {
  p = pt(-0.18986516855, 0)
}
sketch s13(on: plane(at: d10.p, normal: axis(o.p, d11.p), x: axis(o.p, d12.p))) {
  c1 = circle((1.02307, -1.47558), 3.04561528)
}
cut1 = cut(s13, -87.6754)
"#,
    )
    .body
    .compact();
    let p = forge_topo::mass::mass(&b).expect("a mass");
    assert!(p.err[0] <= 1e-5 * p.volume, "{} +- {}", p.volume, p.err[0]);
    let q = Quality {
        chord: 0.01,
        ..Quality::default()
    };
    let v = tessellate(&b, &q).expect("mesh").volume();
    assert!((p.volume - v).abs() <= 1e-3 * v, "{} vs the mesh's {v}", p.volume);
}

/// A cut that starts on a face and runs away from the part removes nothing: its faces are still the
/// block's, so the block's edges are all of the block's edges (a chamfer on them is the same as on
/// the block alone).
#[test]
fn a_cut_removing_nothing_leaves_each_face_its_maker() {
    let alone = "sketch s(on: xy) {
  r = rect(corner: (-30, -30), size: (60, 60))
}
block = extrude(s, 30)
bevel = chamfer(edges(of: block), 1)
";
    let (a, b) = (
        mass(alone),
        mass(
            r#"sketch s(on: xy) {
  r = rect(corner: (-30, -30), size: (60, 60))
}
block = extrude(s, 30)
sketch t(on: block.end) {
  r = rect(corner: (-10, -10), size: (20, 20))
}
nothing = cut(t, -10)
bevel = chamfer(edges(of: block), 1)
"#,
        ),
    );
    assert!((a - b).abs() < 1e-9 * a, "{a} {b}");
}

/// A flat side crossing a round end a hair past the inner corner: the stretch of the end between them is
/// shorter than the tolerance's reach, was taken as on the tool's edge and never imprinted, and the
/// tool's floor came out a slit (false ZeroThickness; WHUCAD 100003369 had this).
#[test]
fn a_stretch_shorter_than_the_tolerance_between_two_crossings_is_imprinted() {
    let src = r#"sketch s1(on: xy) {
  a1 = arc(center: (0.09, 0), from: (0, 2.2043642384652355e-18), to: (0.18, -2.2043642384652358e-17))
  a2 = arc(center: (0.162, 0), from: (0.18, -2.2043642384652358e-17), to: (0.14400000000000002, 2.2043642384652355e-18))
  a3 = arc(center: (0.09, 0), from: (0.14400000000000002, 2.2043642384652355e-18), to: (0.036, -6.613092715395707e-18), cw: true)
  a4 = arc(center: (0.018, 0), from: (0.036, -6.613092715395707e-18), to: (0, 2.2043642384652355e-18))
}
extrude1 = extrude(s1, 0.1)
sketch s2(on: xy) {
  a1 = arc(center: (0.098588751, 0.0072), from: (0.03550000065041929, 4.738048235003456e-11), to: (0.16199999982644986, 0.003876757267441411))
  l2 = line((0.16199999982644986, 0.003876757267441411), (0.03550000065041929, 4.738048235003456e-11))
}
extrude2 = extrude(s2, 0.03)
"#;
    builds(src);
    let v = mass(src);
    // the banana 0.1 high, and the part of the D past its hollow 0.03 high: areas on a fine grid
    assert!((v - 0.00105669).abs() < 1e-3 * 0.00105669, "{v}");
}

/// A chamfer on a rim ending at a corner where the wall behind it turns away: its plane meets the rim beyond
/// the corner, not on it, and the flat beside it runs on that far (was TooLarge at any size). OCCT's volume.
#[test]
fn a_chamfer_ending_at_a_concave_corner_of_the_rim() {
    let b = built(
        r#"sketch s(on: xy) {
  p = poly([(0, 0), (40, 0), (40, 10), (25, 10), (20, 20), (0, 20)])
}
b = extrude(s, 10)
bevel = chamfer(edges(between: (b.end, b.side(s.p[3]))), 1)
"#,
    );
    assert!(
        forge_topo::check(&b.body).is_empty(),
        "{:?}",
        forge_topo::check(&b.body)
    );
    let m = b.volume();
    assert!((m - 6244.40983).abs() < 1e-4, "{m}");
}

/// A chamfer beside an earlier one of the same size: its plane runs through the far end of the edge the first
/// left at the corner, so that edge goes and its far vertex is the corner (was TooLarge, an edge of length 0).
/// By hand: 1000 - 5 - (5 - 1/3), each cut 1/2 x 10, less the 1/3 the two share.
#[test]
fn a_chamfer_beside_an_equal_one_takes_the_short_edge_between() {
    let b = built(
        r#"sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (10, 10))
}
b = extrude(s, 10)
first = chamfer(edges(between: (b.end, b.side(s.r[0]))), 1)
second = chamfer(edges(between: (b.end, b.side(s.r[1]))), 1)
"#,
    );
    assert!(
        forge_topo::check(&b.body).is_empty(),
        "{:?}",
        forge_topo::check(&b.body)
    );
    let m = b.volume();
    assert!((m - (990.0 + 1.0 / 3.0)).abs() < 1e-6, "{m}");
}

/// Three chamfers of one feature at a corner meet in Parasolid's flat triangle, the default.
#[test]
fn three_chamfers_at_a_corner_patch_by_default() {
    let b = built(
        r#"sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (10, 10))
}
b = extrude(s, 10)
bevel = chamfer(edges(of: b), 1)
"#,
    );
    assert_eq!(b.body.faces.len(), 26);
    assert!(b.volume() < 946.0);
}

/// Three chamfers of one feature at a corner, `corner point`: their cut planes run on to a point (CATIA's
/// corner). Own numbers: the volume of |x|,|y|,|z| <= 5 with |x|+|y|, |y|+|z|, |z|+|x| <= 9, by
/// inclusion-exclusion: 1000 - 12 * 5 + 8 * 3/4. The .fc part:
///   extrude 10 rect 0 0 10 10
///   chamfer 1 corner point (edges)
#[test]
#[ignore = "gap: Forge's chamfer has no corner option (.fc `chamfer D corner point`); it always patches"]
fn three_chamfers_at_a_corner_meet_at_a_point_when_asked() {
    let b = built(
        "sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (10, 10))
}
b = extrude(s, 10)
bevel = chamfer(edges(of: b), 1, corner: \"point\")
",
    );
    assert!(
        forge_topo::check(&b.body).is_empty(),
        "{:?}",
        forge_topo::check(&b.body)
    );
    assert_eq!(b.body.faces.len(), 18);
    assert!((b.volume() - 946.0).abs() < 1e-6, "{}", b.volume());
}

/// A closed spline turned about an axis: its integrals have to stop at the section's knots, where the
/// spline has only two derivatives, or they do not settle (refused here, 0.7% off read back from a
/// file). Pappus, from the same spline extruded: its area and its centroid.
#[test]
fn a_closed_spline_turned_has_its_volume() {
    let flat = built(
        r#"sketch s(on: xy) {
  o = spline([(50.000000, 0.000000), (50.107172, 0.440868), (50.399510, 1.008651), (50.794644, 1.806091), (51.171602, 2.891796), (51.392305, 4.267949), (51.326238, 5.877853), (50.874033, 7.613744), (49.985862, 9.333561), (48.671457, 10.883774), (47.000000, 12.124356), (45.089897, 12.951589), (43.090170, 13.314791), (41.156679, 13.224061), (39.427251, 12.747736), (38.000000, 12.000000), (36.918568, 11.120789), (36.166798, 10.251482), (35.673762, 9.510565), (35.328217, 8.973502), (35.000000, 8.660254), (34.564611, 8.532633), (33.926728, 8.501915), (33.038557, 8.445391), (31.909830, 8.228994), (30.607695, 7.732051), (29.246511, 6.869884), (27.969288, 5.610317), (26.923968, 3.981230), (26.238646, 2.067815), (26.000000, 0.000000), (26.238646, -2.067815), (26.923968, -3.981230), (27.969288, -5.610317), (29.246511, -6.869884), (30.607695, -7.732051), (31.909830, -8.228994), (33.038557, -8.445391), (33.926728, -8.501915), (34.564611, -8.532633), (35.000000, -8.660254), (35.328217, -8.973502), (35.673762, -9.510565), (36.166798, -10.251482), (36.918568, -11.120789), (38.000000, -12.000000), (39.427251, -12.747736), (41.156679, -13.224061), (43.090170, -13.314791), (45.089897, -12.951589), (47.000000, -12.124356), (48.671457, -10.883774), (49.985862, -9.333561), (50.874033, -7.613744), (51.326238, -5.877853), (51.392305, -4.267949), (51.171602, -2.891796), (50.794644, -1.806091), (50.399510, -1.008651), (50.107172, -0.440868)], closed: true)
}
flat = extrude(s, 10)
"#,
    );
    let m = forge_topo::mass::mass(&flat.body).expect("a mass");
    let want = forge_math::dmath::TAU * m.centroid.x * m.volume / 10.0;
    let turned = built(
        r#"sketch s(on: xz) {
  o = spline([(50.000000, 0.000000), (50.107172, 0.440868), (50.399510, 1.008651), (50.794644, 1.806091), (51.171602, 2.891796), (51.392305, 4.267949), (51.326238, 5.877853), (50.874033, 7.613744), (49.985862, 9.333561), (48.671457, 10.883774), (47.000000, 12.124356), (45.089897, 12.951589), (43.090170, 13.314791), (41.156679, 13.224061), (39.427251, 12.747736), (38.000000, 12.000000), (36.918568, 11.120789), (36.166798, 10.251482), (35.673762, 9.510565), (35.328217, 8.973502), (35.000000, 8.660254), (34.564611, 8.532633), (33.926728, 8.501915), (33.038557, 8.445391), (31.909830, 8.228994), (30.607695, 7.732051), (29.246511, 6.869884), (27.969288, 5.610317), (26.923968, 3.981230), (26.238646, 2.067815), (26.000000, 0.000000), (26.238646, -2.067815), (26.923968, -3.981230), (27.969288, -5.610317), (29.246511, -6.869884), (30.607695, -7.732051), (31.909830, -8.228994), (33.038557, -8.445391), (33.926728, -8.501915), (34.564611, -8.532633), (35.000000, -8.660254), (35.328217, -8.973502), (35.673762, -9.510565), (36.166798, -10.251482), (36.918568, -11.120789), (38.000000, -12.000000), (39.427251, -12.747736), (41.156679, -13.224061), (43.090170, -13.314791), (45.089897, -12.951589), (47.000000, -12.124356), (48.671457, -10.883774), (49.985862, -9.333561), (50.874033, -7.613744), (51.326238, -5.877853), (51.392305, -4.267949), (51.171602, -2.891796), (50.794644, -1.806091), (50.399510, -1.008651), (50.107172, -0.440868)], closed: true)
  ax = line((0, 0), (0, 1), construction: true)
}
turned = revolve(s, axis: s.ax)
"#,
    );
    let v = turned.volume();
    assert!((v - want).abs() < 1e-5, "{v} {want}");
    let file = forge_step::write::export(&turned.body, &forge_step::write::Options::default()).unwrap();
    let back = forge_step::read::import(file.text.as_bytes(), &forge_step::Limits::default()).unwrap();
    let again = forge_topo::mass::mass(&back.solids[0].body).unwrap().volume;
    assert!((again - want).abs() < 1e-5 * want, "{again} {want}");
}

/// A blade's root rounded where its long walls and ends mitre: the wall runs a little past the loft's
/// end, and its fit in a file has to go with it or the file's edges lie off their faces.
#[test]
fn a_mitred_wall_past_a_lofts_end_is_all_in_the_file() {
    let b = built(
        r#"sketch s(on: xy) {
  r = rect(corner: (-40, -40), size: (80, 80))
}
base = extrude(s, 10)
sketch p0(on: offset(xy, 5)) {
  r = rect(corner: (-25, -3), size: (50, 6))
}
sketch p1(on: offset(xy, 30)) {
  r = rect(corner: (-22, -3), size: (44, 6))
}
sketch p2(on: offset(xy, 60)) {
  r = rect(corner: (-18, -3), size: (36, 6))
}
b = loft([p0, p1, p2], smooth: true)
root = fillet(edges(between: (base, b)), 1.5)
"#,
    );
    let file = forge_step::write::export(&b.body, &forge_step::write::Options::default()).unwrap();
    assert!(file.uncertainty < 2e-3, "{}", file.uncertainty);
}

/// A round between two discs a little off centre from each other: its blend runs on a traced spine,
/// and the circle on its foot goes round the blend's parameter unevenly. Its mass has to settle.
#[test]
fn a_round_between_eccentric_discs_has_its_mass() {
    let b = built(
        r#"sketch d1(on: offset(xy, 0)) {
  p = pt(-0.3127, 0)
}
sketch s2(on: plane(at: d1.p, normal: Z, x: X)) {
  c1 = circle((0.316, 0), 0.316)
}
extrude1 = extrude(s2, 0.125)
sketch d3(on: offset(xy, 0)) {
  p = pt(-0.3751, 0)
}
sketch s4(on: plane(at: d3.p, normal: Z, x: X)) {
  c1 = circle((0.379, 0), 0.379)
}
extrude2 = extrude(s4, 0.03)
round = fillet(edges(between: (extrude1, extrude2), crease: "concave"), 0.0625)
"#,
    )
    .body;
    let got = forge_topo::mass::mass(&b).expect("a mass");
    // the loop's closure reports how far the rules round the circle miss the turn
    assert!(got.err[0] < 1e-4 * got.volume, "{} of {}", got.err[0], got.volume);
    let q = Quality {
        chord: 0.0001,
        ..Quality::default()
    };
    let mesh = tessellate(&b, &q).unwrap();
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
/// other way got past it and closed, and the two joined ran round the loop twice. (The blade's
/// sections at -47 and -60 degrees about the hub's axis.)
#[test]
fn a_loop_traced_closed_on_the_way_back_is_not_run_round_twice() {
    let volume = |src: &str| {
        let b = built(src);
        assert!(forge_topo::check(&b.body).is_empty());
        b.volume()
    };
    let (a, b) = (
        volume(
            r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: xz) {
  ax = line((0, 0), (0, 1), construction: true)
  l1 = line((0, -16), (24, -16))
  l2 = line((24, -16), (24, 0))
  l3 = line((24, 0), (92, 0))
  l4 = line((92, 0), (92, 2.6))
  l5 = line((92, 2.6), (48.67237388699241, 5.420000000000009))
  a6 = arc(center: (54, 39), from: (48.67237388699241, 5.420000000000009), to: (20, 39.00000000000001), cw: true)
  l7 = line((20, 39.00000000000001), (20, 46))
  l8 = line((20, 46), (0, 46))
  l9 = line((0, 46), (0, -16))
}
hub = revolve(s1, axis: s1.ax)
sketch d2(on: offset(xy, 1.5)) {
  p = pt(0, 0)
}
sketch d3(on: offset(xy, 0)) {
  p = pt(0.6819983600624985, -0.7313537016191705)
}
sketch s4(on: plane(at: d2.p, normal: Z, x: axis(o.p, d3.p))) {
  a1 = arc(center: (41, 0), from: (58.22733473856245, 60.07885599614493), to: (-18.441032268447096, 19.31356214843422))
  a2 = arc(center: (-17.014447494004365, 18.850036656871797), from: (-18.441032268447096, 19.31356214843422), to: (-15.587862719561635, 18.386511165309376))
  a3 = arc(center: (41, 0), from: (-15.587862719561635, 18.386511165309376), to: (57.40042267111145, 57.195070908329974), cw: true)
  a4 = arc(center: (57.81387870483695, 58.63696345223745), from: (57.40042267111145, 57.195070908329974), to: (58.22733473856245, 60.07885599614493))
}
sketch d5(on: offset(xy, 23)) {
  p = pt(0, 0)
}
sketch d6(on: offset(xy, 0)) {
  p = pt(0.5446390350150271, -0.838670567945424)
}
sketch s7(on: plane(at: d5.p, normal: Z, x: axis(o.p, d6.p))) {
  a1 = arc(center: (40, 0), from: (59.370415180039615, 67.55266568206537), to: (-26.835496682641917, 21.71616927969944))
  a2 = arc(center: (-25.622899624365587, 21.322172611871377), from: (-26.835496682641917, 21.71616927969944), to: (-24.410302566089268, 20.928175944043318))
  a3 = arc(center: (40, 0), from: (-24.410302566089268, 20.928175944043318), to: (58.66753992270627, 65.10144835742264), cw: true)
  a4 = arc(center: (59.01897755137294, 66.327057019744), from: (58.66753992270627, 65.10144835742264), to: (59.370415180039615, 67.55266568206537))
}
sketch d8(on: offset(xy, 45)) {
  p = pt(0, 0)
}
sketch d9(on: offset(xy, 0)) {
  p = pt(0.29237170472273677, -0.9563047559630354)
}
sketch s10(on: plane(at: d8.p, normal: Z, x: axis(o.p, d9.p))) {
  a1 = arc(center: (40, 0), from: (62.616045044784784, 78.87152215173907), to: (-38.034187162017346, 25.354844388464443))
  a2 = arc(center: (-37.03557781990743, 25.03037654437075), from: (-38.034187162017346, 25.354844388464443), to: (-36.03696847779752, 24.705908700277053))
  a3 = arc(center: (40, 0), from: (-36.03696847779752, 24.705908700277053), to: (62.03720659756908, 76.8528725902686), cw: true)
  a4 = arc(center: (62.32662582117693, 77.86219737100383), from: (62.03720659756908, 76.8528725902686), to: (62.616045044784784, 78.87152215173907))
}
blade = loft([s4, s7, s10], smooth: true)
"#,
        ),
        volume(
            r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: xz) {
  ax = line((0, 0), (0, 1), construction: true)
  l1 = line((0, -16), (24, -16))
  l2 = line((24, -16), (24, 0))
  l3 = line((24, 0), (92, 0))
  l4 = line((92, 0), (92, 2.6))
  l5 = line((92, 2.6), (48.67237388699241, 5.420000000000009))
  a6 = arc(center: (54, 39), from: (48.67237388699241, 5.420000000000009), to: (20, 39.00000000000001), cw: true)
  l7 = line((20, 39.00000000000001), (20, 46))
  l8 = line((20, 46), (0, 46))
  l9 = line((0, 46), (0, -16))
}
hub = revolve(s1, axis: s1.ax)
sketch d2(on: offset(xy, 1.5)) {
  p = pt(0, 0)
}
sketch d3(on: offset(xy, 0)) {
  p = pt(0.5000000000000001, -0.8660254037844386)
}
sketch s4(on: plane(at: d2.p, normal: Z, x: axis(o.p, d3.p))) {
  a1 = arc(center: (41, 0), from: (58.22733473856245, 60.07885599614493), to: (-18.441032268447096, 19.31356214843422))
  a2 = arc(center: (-17.014447494004365, 18.850036656871797), from: (-18.441032268447096, 19.31356214843422), to: (-15.587862719561635, 18.386511165309376))
  a3 = arc(center: (41, 0), from: (-15.587862719561635, 18.386511165309376), to: (57.40042267111145, 57.195070908329974), cw: true)
  a4 = arc(center: (57.81387870483695, 58.63696345223745), from: (57.40042267111145, 57.195070908329974), to: (58.22733473856245, 60.07885599614493))
}
sketch d5(on: offset(xy, 23)) {
  p = pt(0, 0)
}
sketch d6(on: offset(xy, 0)) {
  p = pt(0.3420201433256688, -0.9396926207859083)
}
sketch s7(on: plane(at: d5.p, normal: Z, x: axis(o.p, d6.p))) {
  a1 = arc(center: (40, 0), from: (59.370415180039615, 67.55266568206537), to: (-26.835496682641917, 21.71616927969944))
  a2 = arc(center: (-25.622899624365587, 21.322172611871377), from: (-26.835496682641917, 21.71616927969944), to: (-24.410302566089268, 20.928175944043318))
  a3 = arc(center: (40, 0), from: (-24.410302566089268, 20.928175944043318), to: (58.66753992270627, 65.10144835742264), cw: true)
  a4 = arc(center: (59.01897755137294, 66.327057019744), from: (58.66753992270627, 65.10144835742264), to: (59.370415180039615, 67.55266568206537))
}
sketch d8(on: offset(xy, 45)) {
  p = pt(0, 0)
}
sketch d9(on: offset(xy, 0)) {
  p = pt(0.06975647374412523, -0.9975640502598242)
}
sketch s10(on: plane(at: d8.p, normal: Z, x: axis(o.p, d9.p))) {
  a1 = arc(center: (40, 0), from: (62.616045044784784, 78.87152215173907), to: (-38.034187162017346, 25.354844388464443))
  a2 = arc(center: (-37.03557781990743, 25.03037654437075), from: (-38.034187162017346, 25.354844388464443), to: (-36.03696847779752, 24.705908700277053))
  a3 = arc(center: (40, 0), from: (-36.03696847779752, 24.705908700277053), to: (62.03720659756908, 76.8528725902686), cw: true)
  a4 = arc(center: (62.32662582117693, 77.86219737100383), from: (62.03720659756908, 76.8528725902686), to: (62.616045044784784, 78.87152215173907))
}
blade = loft([s4, s7, s10], smooth: true)
"#,
        ),
    );
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
            "sketch s1(on: xy) {{
  c1 = circle((0, 0), 10)
}}
extrude1 = extrude(s1, 1)
sketch d2(on: offset(xy, 0)) {{
  p = pt(0, 9)
}}
sketch s3(on: plane(at: d2.p, normal: -Z, x: X)) {{
  c1 = circle((0, 0), 1)
}}
extrude2 = extrude(s3, 5)
sketch d4(on: offset(xy, 0)) {{
  p = pt(0, -9)
}}
sketch s5(on: plane(at: d4.p, normal: -Z, x: X)) {{
  c1 = circle((0, 0), 1)
}}
extrude3 = extrude(s5, 5)
sketch d6(on: offset(xy, 0)) {{
  p = pt({e}, 0)
}}
sketch s7(on: plane(at: d6.p, normal: Z, x: X)) {{
  c1 = circle((0, 0), 8)
}}
cut1 = cut(s7, -1)
"
        );
        builds(&src);
        let v = mass(&src);
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
    let plate = r#"sketch s1(on: xy) {
  a1 = arc(center: (0.2, 0), from: (0, 2.4492935982947065e-17), to: (0.4, 0), cw: true)
  l2 = line((0.4, 0), (0.4, 0.5))
  l3 = line((0.4, 0.5), (0, 0.5))
  l4 = line((0, 0.5), (0, 2.4492935982947065e-17))
}
extrude1 = extrude(s1, 0.12)
sketch s2(on: xy) {
  c1 = circle((0.25, 0), 0.25)
}
cut1 = cut(s2, -0.1)
"#
    .split("sketch s2")
    .next()
    .unwrap()
    .to_string();
    let r: f64 = 0.25;
    let area = 0.2 - 0.02 * pi;
    // the circle over the plate: its upper half less the segment past x = 0.4, less the notch inside it
    let d = 0.4 - r;
    let segment = r * r * forge_math::dmath::acos(d / r) - d * (r * r - d * d).sqrt();
    let over = pi * r * r / 2.0 - segment / 2.0 - 0.02 * pi;
    let disk = pi * r * r;
    let tool = |at: f64, op: &str, depth: &str| {
        format!(
            "sketch d2(on: offset(xy, {at})) {{
  p = pt(0, 0)
}}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {{
  c1 = circle((0.25, 0), 0.25)
}}
tool = {op}(s3, {depth})
"
        )
    };
    for (tool, want) in [
        (tool(0.0, "extrude", "0.12"), 0.12 * (area + disk - over)),
        (
            tool(0.0, "extrude", "0.2"),
            0.12 * (area + disk - over) + 0.08 * disk,
        ),
        (tool(0.01, "extrude", "0.1"), 0.12 * area + 0.1 * (disk - over)),
        (tool(0.0, "cut", "-0.1"), 0.12 * area - 0.1 * over),
        (tool(0.01, "cut", "-0.1"), 0.12 * area - 0.1 * over),
    ] {
        let src = format!("{plate}{tool}");
        builds(&src);
        let v = mass(&src);
        assert!((v - want).abs() < 1e-9, "{tool}: {v} vs {want}");
    }
}

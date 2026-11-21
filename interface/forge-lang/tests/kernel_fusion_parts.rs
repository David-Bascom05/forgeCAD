//! Parts modeled the way real CAD histories model them (research/find-parts), as Forge parts, each
//! valid feature by feature and n-ary, the two paths the same solid.
mod common;
mod kernel_util;

use forge_math::dmath;
use kernel_util::{builds_alike, built, refusal};

/// Builds valid on both paths with the same volume (by meshes), which it returns.
fn builds(src: &str) -> f64 {
    builds_alike(src, 0.002)
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-3 * b, "{a} vs {b}");
}

fn tan(deg: f64) -> f64 {
    let (s, c) = dmath::sin_cos(deg * dmath::PI / 180.0);
    s / c
}

/// A circle leans in to a frustum, or out; the profile stays as drawn at the sketch.
#[test]
fn tapered_circle_is_a_frustum() {
    for (angle, sign) in [(5.0, -1.0), (-5.0, 1.0)] {
        let top = 10.0 + sign * 8.0 * tan(5.0);
        let want = dmath::PI * 8.0 / 3.0 * (100.0 + 10.0 * top + top * top);
        let src = format!(
            "sketch s1(on: xy) {{
  c1 = circle((0, 0), 10)
}}
extrude1 = extrude(s1, 8, taper: {angle}deg)
"
        );
        near(builds(&src), want);
    }
}

/// A rectangle's walls are planes re-met at the corners; extruded down its frame is flipped.
#[test]
fn tapered_rect_is_a_pyramid_frustum() {
    let d = 5.0 * tan(10.0);
    let (a, b) = (40.0 - 2.0 * d, 20.0 - 2.0 * d);
    let want = 5.0 / 3.0 * (800.0 + a * b + (800.0 * a * b).sqrt());
    near(
        builds(
            r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (40, 20))
}
extrude1 = extrude(s1, 5, taper: 10deg)
"#,
        ),
        want,
    );
    near(
        builds(
            r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (40, 20))
}
extrude1 = extrude(s1, -5, taper: 10deg)
"#,
        ),
        want,
    );
}

/// A hole flares where the outline narrows: the profile moves in one sense all round.
#[test]
fn tapered_ring_narrows_both_walls() {
    let d = 6.0 * tan(5.0);
    let frustum = |r: f64, q: f64| dmath::PI * 6.0 / 3.0 * (r * r + r * q + q * q);
    near(
        builds(
            r#"sketch s1(on: xy) {
  c1 = circle((0, 0), 20)
  c2 = circle((0, 0), 10)
}
extrude1 = extrude(s1, 6, taper: 5deg)
"#,
        ),
        frustum(20.0, 20.0 - d) - frustum(10.0, 10.0 + d),
    );
}

/// Tapered tools cut and join like any other; lines and tangent arcs both lean.
#[test]
fn tapered_tools_cut_and_add() {
    let d = 4.0 * tan(5.0);
    let cone = dmath::PI * 4.0 / 3.0 * (100.0 + 10.0 * (10.0 - d) + (10.0 - d) * (10.0 - d));
    near(
        builds(
            r#"sketch s1(on: xy) {
  r1 = rect(corner: (-30, -30), size: (60, 60))
}
extrude1 = extrude(s1, 10)
sketch d2(on: offset(xy, 10)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: -Z, x: X)) {
  c1 = circle((0, 0), 10)
}
cut1 = cut(s3, -4, taper: 5deg)
"#,
        ),
        36000.0 - cone,
    );
    near(
        builds(
            r#"sketch s1(on: xy) {
  r1 = rect(corner: (-30, -30), size: (60, 60))
}
extrude1 = extrude(s1, 10)
sketch d2(on: offset(xy, 10)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 10)
}
extrude2 = extrude(s3, 4, taper: 5deg)
"#,
        ),
        36000.0 + cone,
    );
    builds(
        r#"sketch s1(on: xy) {
  l1 = line((0, 0), (20, 0))
  a2 = arc(center: (20, 5), from: (20, 0), to: (20, 10))
  l3 = line((20, 10), (0, 10))
  a4 = arc(center: (0, 5), from: (3.061616997868383e-16, 10), to: (-9.184850993605148e-16, 0))
}
extrude1 = extrude(s1, 5, taper: 3deg)
"#,
    );
}

/// Arcs of one circle whose centres carry noise: each pair within tolerance of one surface, the
/// chain of them not. Faces merged all the way left edges off the face they joined. (A sketch's
/// loop shares its points, so here the arcs whose centres are a hair off have radii a hair off
/// too, ending where their neighbours do.)
#[test]
fn arcs_a_hair_off_one_circle_stay_on_their_faces() {
    let v = builds(
        r#"sketch s(on: xy) {
  p = pt(-10.606601717798213, -10.606601717798213)
  q = pt(10.606601717798213, -10.606601717798213)
  r = pt(10.606601717798213, 10.606601717798213)
  t = pt(-10.606601717798213, 10.606601717798213)
  a = arc(center: (0, -0.000008), from: p, to: q)
  b = arc(center: (0, 0), from: q, to: r)
  c = arc(center: (0, 0.000008), from: r, to: t)
  d = arc(center: (0, 0), from: t, to: p)
}
rod = extrude(s, 24)
"#,
    );
    near(v, dmath::PI * 225.0 * 24.0);
}

/// A long window in a face curved across it, a hair inside the face's end: the point that says
/// which region holds the hole was a thousandth of the window's length off it, on the far side.
#[test]
fn window_near_the_end_of_a_curved_face() {
    let area = 700.0 + 1200.0 + dmath::PI * 400.0;
    let mut cut = 0.0;
    let n = 4000;
    for i in 0..n {
        let x = 11.0 + 37.0 * (i as f64 + 0.5) / n as f64;
        cut += (30.0 + (1600.0 - (x - 10.0) * (x - 10.0)).sqrt()) * 37.0 / n as f64;
    }
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Z, x: X)) {
  l1 = line((50, 0), (0, 0))
  l2 = line((0, 0), (0, 70))
  l3 = line((0, 70), (10, 70))
  a4 = arc(center: (10, 30), from: (10.000000000000002, 70), to: (50, 30), cw: true)
  l5 = line((50, 30), (50, 0))
}
extrude1 = extrude(s1, 40)
sketch s2(on: plane(at: o.p, normal: Y, x: X)) {
  p1 = poly([(48, -38), (11, -38), (11, -2), (48, -2)])
}
cut1 = cut(s2, -140)
"#,
    );
    near(v, area * 40.0 - cut * 36.0);
}

/// A disc on a tool's flat underside, its rim tangent to the underside's edges: the rim's middle
/// lands on the far edge, and it is still a new edge of the underside. Not a pinch: the disc is
/// joined to the tool across its whole top.
#[test]
fn disc_rim_tangent_to_a_flat_it_sits_on() {
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 16)
  c2 = circle((0, 0), 5)
}
extrude1 = extrude(s1, 5)
sketch s2(on: plane(at: o.p, normal: Z, x: X)) {
  p1 = poly([(-18, 21), (18, 21), (16, 5.00000015), (-16, 5)])
}
extrude2 = extrude(s2, two(25, 25))
"#,
    );
    near(v, dmath::PI * 231.0 * 5.0 + 34.0 * 16.0 * 50.0);
}

/// A lug on a block's top, its round side tangent to the block's end face along the top's edge:
/// each face ends where the other starts and one skin runs on. Faces facing apart would be a pinch.
#[test]
fn round_side_runs_on_tangent_from_a_face() {
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Z, x: X)) {
  p1 = poly([(0, 0), (0, 30), (15, 30), (15, 12), (100, 12), (100, 0)])
}
extrude1 = extrude(s1, 40)
sketch d2(on: offset(xy, 0)) {
  p = pt(15, 0)
}
sketch s3(on: plane(at: d2.p, normal: -X, x: -Z)) {
  l1 = line((0, -30), (-9, -30))
  a2 = arc(center: (-20, -30), from: (-9, -30), to: (-31, -30), cw: true)
  l3 = line((-31, -30), (-40, -30))
  a4 = arc(center: (-20, -30), from: (-40, -29.999999999999996), to: (0, -30.000000000000004))
}
extrude2 = extrude(s3, 15)
"#,
    );
    near(v, 1470.0 * 40.0 + dmath::PI / 2.0 * 279.0 * 15.0);
}

/// A plate standing against a rod, tangent along a line: true zero thickness, refused.
#[test]
fn plate_tangent_to_a_rod_is_a_pinch() {
    let e = refusal(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 20)
}
extrude1 = extrude(s1, 100)
sketch s2(on: plane(at: o.p, normal: Z, x: X)) {
  p1 = poly([(-20, 30), (-20, 90), (-60, 90), (-60, 30)])
}
extrude2 = extrude(s2, two(5, 5))
"#,
    );
    assert!(e.contains("ZeroThickness"), "{e}");
}

/// A cross hole whose axis lies a hair inside a tube's surface: where the hole's curve crosses its
/// own seam is anywhere along a stretch, and the tube's seam crosses that stretch. One vertex there.
#[test]
fn cross_hole_tangent_to_a_tube() {
    // (OCCT agrees: tools/occt)
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: X, x: -Z)) {
  c1 = circle((0, 0), 25)
  c2 = circle((0, 0), 22.5)
}
extrude1 = extrude(s1, two(15, 15))
sketch s2(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 24.99999988), 4)
}
cut1 = cut(s2, two(-25, -25))
"#,
    );
    near(v, 10830.571);
}

/// Coaxial bores stacked with their seams apart join into a face whose seam steps; a hole in it goes
/// in the span the outline has at its height, not the first that holds its middle.
#[test]
fn hole_in_a_bore_whose_seam_steps() {
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: -Z, x: X)) {
  r1 = rect(corner: (-40, -60), size: (80, 120))
  c2 = circle((0, 20), 15)
}
extrude1 = extrude(s1, 40)
sketch d2(on: offset(xy, -40)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: -Z, x: -X)) {
  r1 = rect(corner: (-50, -70), size: (100, 140))
  c2 = circle((0, -20), 15)
}
extrude2 = extrude(s3, 30)
sketch d4(on: offset(xy, 0)) {
  p = pt(0, -70)
}
sketch s5(on: plane(at: d4.p, normal: Y, x: X)) {
  c1 = circle((0, 25), 7)
}
cut1 = cut(s5, -40)
"#,
    );
    near(v, 750606.77);
}

/// A taper past where the profile closes stops there: a pyramid, a roof, a cone.
#[test]
fn taper_past_the_profile_closing() {
    near(
        builds(
            r#"sketch s1(on: xy) {
  p1 = poly([(20, 20), (20, -20), (-20, -20), (-20, 20)])
}
extrude1 = extrude(s1, 70, taper: 16deg)
"#,
        ),
        1600.0 * (20.0 / tan(16.0)) / 3.0,
    );
    near(
        builds(
            r#"sketch s1(on: xy) {
  r1 = rect(corner: (0, 0), size: (40, 20))
}
extrude1 = extrude(s1, 15, taper: 45deg)
"#,
        ),
        10.0 / 6.0 * (80.0 + 20.0) * 20.0,
    );
    near(
        builds(
            r#"sketch s1(on: xy) {
  c1 = circle((0, 0), 10)
}
extrude1 = extrude(s1, 20, taper: 45deg)
"#,
        ),
        dmath::PI * 100.0 * 10.0 / 3.0,
    );
    near(
        builds(
            r#"sketch s1(on: xy) {
  p1 = poly([(0, 0), (30, 0), (0, 30)])
}
extrude1 = extrude(s1, 20, taper: 45deg)
"#,
        ),
        {
            let r = 30.0 / (2.0 + 2.0f64.sqrt());
            450.0 * r / 3.0
        },
    );
}

/// Rounded corners lean in to their points on the way up, sharp above: the wall below and above
/// is one plane, a cone's point at each corner.
#[test]
fn taper_wipes_out_rounded_corners() {
    let t = tan(30.0);
    let mut want = 0.0;
    let n = 4000;
    for i in 0..n {
        let d = (i as f64 + 0.5) / n as f64 * 10.0 * t;
        let r = (2.0 - d).max(0.0);
        want += ((40.0 - 2.0 * d) * (40.0 - 2.0 * d) - (4.0 - dmath::PI) * r * r) * 10.0 / n as f64;
    }
    near(
        builds(
            r#"sketch s1(on: xy) {
  rr1 = hull(disc((2, 2), 2), disc((38, 2), 2), disc((38, 38), 2), disc((2, 38), 2))
}
extrude1 = extrude(s1, 10, taper: 30deg)
"#,
        ),
        want,
    );
}

/// A hole's axis a hair inside a tube's surface, its seam crossing the tube twice a hair apart: the
/// curve runs out and back along the seam. One vertex for the two.
#[test]
fn cross_hole_seam_out_and_back_over_a_tube() {
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: X, x: -Z)) {
  c1 = circle((0, 0), 25.0000012)
  c2 = circle((0, 0), 22.5)
}
extrude1 = extrude(s1, two(15, 15))
sketch s2(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, -25), 4)
}
cut1 = cut(s2, two(-25, -25))
"#,
    );
    near(v, 10830.576);
}

/// Bodies butted with corners a hair apart: "within tolerance" chains, a vertex near two cuts that
/// are not near each other must not end in two vertices.
#[test]
fn butted_corners_chain_within_tolerance() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: -Y, x: X)) {
  p1 = poly([(2.449811762, -0.498419836), (2.5, 0), (-2.5, 0), (-2.5, -0.505), (0, -12.0000003)])
}
extrude1 = extrude(s1, 2)
sketch s2(on: plane(at: o.p, normal: Y, x: X)) {
  a1 = arc(center: (0, 0), from: (-2.5, 3.061616997868383e-16), to: (2.5, -6.123233995736766e-16))
  l2 = line((2.50000004, 0), (2.449811762, 0.498419836))
  a3 = arc(center: (0, 0), from: (2.449811761552074, 0.498419836042993), to: (-2.449811761552074, 0.4984198360429931))
  l4 = line((-2.4498118, 0.4984198), (-2.5, 0))
}
extrude2 = extrude(s2, 2)
"#,
    );
}

/// Two blocks flush along a wall, each rounding off from it at one height: beyond, a groove of no
/// width at its root. One skin turns back along the root; below it the blocks are one.
#[test]
fn groove_rooted_where_rounded_corners_part() {
    let v = builds(
        r#"sketch s1(on: xy) {
  l1 = line((5, -20), (5, 20))
  a2 = arc(center: (4, 20), from: (5, 20), to: (4, 21))
  l3 = line((4, 21), (-20, 21))
  l4 = line((-20, 21), (-20, -20))
  l5 = line((-20, -20), (5, -20))
}
extrude1 = extrude(s1, 10)
sketch s2(on: xy) {
  l1 = line((5, -20), (30, -20))
  l2 = line((30, -20), (30, 21))
  l3 = line((30, 21), (6, 21))
  a4 = arc(center: (6, 20), from: (6, 21), to: (5, 20))
  l5 = line((5, 20), (5, -20))
}
extrude2 = extrude(s2, 6)
"#,
    );
    let area = 25.0 * 41.0 - (1.0 - dmath::PI / 4.0);
    near(v, area * 16.0);
}

/// A boss flush on a rod, one wall leaving the rod tangent: its knife edge's two sides run within
/// tolerance for a stretch, and the tip ends where they part. Not a pinch.
#[test]
fn knife_edge_tangent_on_a_rod() {
    let v = builds(
        r#"sketch s1(on: xy) {
  c1 = circle((0, 0), 20)
}
extrude1 = extrude(s1, 30)
sketch s2(on: xy) {
  a1 = arc(center: (0, 0), from: (-3.472963553338606, 19.69615506024416), to: (-17.320508075688775, 9.999999999999998))
  l2 = line((-17.320508075688775, 9.999999999999998), (-2.320508, 35.980762))
  l3 = line((-2.320508, 35.980762), (-3.472963553338606, 19.69615506024416))
}
extrude2 = extrude(s2, 15)
"#,
    );
    near(v, dmath::PI * 400.0 * 30.0 + 85.84 * 15.0);
}

/// Holes on a tube's far sides, one over its seam: the outline, stepped round that hole, need not come
/// first among the face's loops, and the other hole belongs in its window.
#[test]
fn holes_across_a_tube_one_on_its_seam() {
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: X, x: -Z)) {
  c1 = circle((0, 0), 4)
  c2 = circle((0, 0), 2.5)
}
extrude1 = extrude(s1, two(2, 2))
sketch s2(on: plane(at: o.p, normal: X, x: -Z)) {
  l1 = line((-3.064178, 2.57115), (-3.255689, 2.731847))
  a2 = arc(center: (0, 0), from: (-3.2556888832556563, 2.731847341167793), to: (3.2556888832556568, 2.731847341167792), cw: true)
  l3 = line((3.255689, 2.731847), (3.064178, 2.57115))
  a4 = arc(center: (0, 0), from: (3.064177772475912, 2.571150438746157), to: (-3.0641777724759116, 2.571150438746158))
}
extrude2 = extrude(s2, two(2, 2))
sketch s3(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 4), 0.6)
}
cut1 = cut(s3, two(-4, -4))
sketch s4(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, -4), 0.6)
}
cut2 = cut(s4, two(-4, -4))
"#,
    );
    near(v, 126.75731);
}

/// A flat running on into an arc whose lowest point lies a hair past their shared edge: the flat's
/// curve with a rod has its tangent root off the edge, and still ends on it.
#[test]
fn curve_ends_on_an_edge_past_its_tangent_root() {
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: X, x: -Z)) {
  c1 = circle((0, 0), 14)
}
extrude1 = extrude(s1, 150)
sketch s2(on: plane(at: o.p, normal: Y, x: X)) {
  a1 = arc(center: (14.00007, -28), from: (34.29169388770305, -19.500000000000057), to: (13.999993000000112, -6.000000000134751))
  l2 = line((14, -6), (0, -6))
  l3 = line((0, -6), (0, -19.5))
  l4 = line((0, -19.5), (14, -19.5))
  l5 = line((14, -19.5), (34.291694, -19.5))
}
cut1 = cut(s2, two(-18, -18))
"#,
    );
    near(v, 88776.474);
}

/// A plate's end a hair into a tube's bore, along a line of its wall, joined across: the bore's wall
/// keeps no slit there. (The plate goes on the first tube before the second is made: a Forge
/// extrude adds to the last body.)
#[test]
fn plate_end_tangent_inside_a_bore_leaves_no_slit() {
    let b = built(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch t0(on: plane(at: o.p, normal: Y, x: X)) {
  c = circle((-14, 0), 6)
  bore = circle((-14, 0), 4.5)
}
b0 = extrude(t0, 40)
sketch pl(on: xy) {
  p = poly([(-9.5, 12), (-9.5, 30), (9.50000015, 30), (9.50000015, 12)])
}
plate = extrude(pl, 8)
sketch t1(on: t0.plane) {
  c = circle((14, 0), 6)
  bore = circle((14, 0), 4.5)
}
b1 = extrude(t1, 40, new: true)
joined = union(b0, tools: b1)
"#,
    );
    assert!(forge_topo::check(&b.body).is_empty());
    kernel_util::mesh_volume(&b.body, 0.002);
}

/// One boss with a knife edge on a rod: the rod at x, the boss its round ends and a tangent side.
fn knife_boss(x: f64) -> String {
    let at = |cx: f64, cy: f64, r: f64, deg: f64| {
        let a = deg.to_radians();
        (cx + r * dmath::cos(a), cy + r * dmath::sin(a))
    };
    // the round end's start and the boss's tangent side's foot; the two arcs meet where their
    // circles cross, left of the rod's centre
    let p = at(x, -25.0, 10.0, 163.0265);
    let t = at(x, 0.0, 18.0, -109.3809 - 45.6191);
    let a: f64 = (100.0 - 324.0 + 625.0) / 50.0;
    let q = (x - (100.0 - a * a).sqrt(), -25.0 + a);
    let s = q;
    format!(
        "  a = arc(center: ({x}, -25), from: ({}, {}), to: ({}, {}), cw: true)
  b = arc(center: ({x}, 0), from: ({}, {}), to: ({}, {}), cw: true)
  l = line(({}, {}), ({}, {}))",
        p.0, p.1, q.0, q.1, s.0, s.1, t.0, t.1, t.0, t.1, p.0, p.1
    )
}

/// Knife edges one after another, each refused plain and built blunted: a retry rebuilding the part
/// through retries of its own doubled with each (16 took 30 s, 20 over 40). Linear: under a minute in
/// any profile, however loaded.
#[test]
fn many_knife_edges_stay_linear() {
    for bodies in [false, true] {
        let mut src = String::new();
        for k in 0..40 {
            let x = 200.0 * k as f64 + 7.0;
            let new = if bodies && k > 0 { ", new: true" } else { "" };
            src += &format!(
                "sketch rod{k}(on: xy) {{
  c = circle(({x}, 0), 18)
}}
r{k} = extrude(rod{k}, 20{new})
sketch boss{k}(on: xy) {{
{}
}}
b{k} = extrude(boss{k}, 8)
",
                knife_boss(x)
            );
        }
        let t = std::time::Instant::now();
        let b = built(&src);
        b.seq().expect("sequential");
        b.nary(1).expect("n-ary");
        assert!(t.elapsed().as_secs() < 60, "{:?}", t.elapsed());
    }
}

/// A hexagon's hole a hair inside its flats, a disc flush in the hole: every step off a flat's middle
/// crosses the hole, so the underside's probe goes off another piece.
#[test]
fn hole_a_hair_inside_the_flats_of_its_outline() {
    let v = builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: X, x: -Z)) {
  c1 = circle((0, 0), 9)
  c2 = circle((0, 0), 4.5)
}
extrude1 = extrude(s1, 5)
sketch s2(on: plane(at: o.p, normal: X, x: -Z)) {
  p1 = poly([(5.1965, 9.0002), (-5.1965, 9.0002), (-10.393, 0), (-5.1965, -9.0002), (5.1965, -9.0002), (10.393, 0)])
  c2 = circle((0, 0), 9)
}
extrude2 = extrude(s2, 5)
"#,
    );
    near(v, 5.0 * (3.0 * 10.393 * 9.0002 - dmath::PI * 20.25));
}

// (the hair edge, a line ending within a tolerance of where it meets an arc; volume from `builds` on both paths)
#[test]
fn hair_edge_at_the_tolerance_floor_is_valid() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  a1 = arc(center: (30, -40), from: (31.249999662018446, -44.84122927002572), to: (31.250000337981557, -35.15877090450721))
  a2 = arc(center: (30, -40), from: (31.25000033798156, -35.15877090450721), to: (31.249999662018446, -44.84122927002572))
  c3 = circle((30, -40), 2.5)
}
b0 = extrude(s1, 2)
sketch d2(on: offset(xy, 0)) {
  p = pt(0, 2)
}
sketch s3(on: plane(at: d2.p, normal: Y, x: X)) {
  a1 = arc(center: (42, -40.000001), from: (42.49999986480738, -41.936492708010285), to: (42.50000013519262, -38.06350936180288))
  l2 = line((42.500000135, -38.063509362), (31.250000338, -35.158770905))
  a3 = arc(center: (30, -40), from: (31.25000033798156, -35.15877090450721), to: (31.249999662018446, -44.84122927002572))
  l4 = line((31.249999662, -44.84122927), (42.499999865, -41.936492708))
  c5 = circle((30, -40), 2.5)
  c6 = circle((42, -40.000001), 1)
}
extrude2 = extrude(s3, 2)
"#,
    );
}

#[test]
fn wall_slivers_shallower_than_the_chord_mesh_closed() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  c1 = circle((0, 0), 9)
  a2 = arc(center: (0, 0), from: (4.9980396156881515, -0.14000000004225616), to: (4.998039615690596, 0.1399999999550239))
  l3 = line((4.998039616, 0.14), (-2.377776251, 4.398429276))
  a4 = arc(center: (0, 0), from: (-2.377776251277661, 4.39842927632808), to: (-2.620263364336168, 4.258429276331556))
  l5 = line((-2.620263364, 4.258429276), (-2.620263364, -4.258429276))
  a6 = arc(center: (0, 0), from: (-2.6202633644104925, -4.258429276285824), to: (-2.3777762513544283, -4.3984292762865795))
  l7 = line((-2.377776251, -4.398429276), (4.998039616, -0.14))
}
b0 = extrude(s1, 12)
sketch s2(on: plane(at: o.p, normal: Y, x: X)) {
  p1 = poly([(4.998039616, -0.14), (4.998039616, 0.14), (-2.377776251, 4.398429276), (-2.620263364, 4.258429276), (-2.620263364, -4.258429276), (-2.377776251, -4.398429276)])
}
extrude2 = extrude(s2, 4)
"#,
    );
}

#[test]
fn slit_left_in_a_face_by_a_tool_edge_meshes() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  p1 = poly([(-10, -20), (-10, 20), (10, 20), (10, -20)])
}
b0 = extrude(s1, 40)
sketch d2(on: offset(xy, 0)) {
  p = pt(10, 0)
}
sketch s3(on: plane(at: d2.p, normal: -X, x: -Z)) {
  p1 = poly([(-20, -15), (-20, -40), (6, -40), (6, -27), (-9, -27)])
}
cut1 = cut(s3, -15)
sketch d4(on: offset(xy, -6)) {
  p = pt(-3, 27)
}
sketch d5(on: offset(xy, 0)) {
  p = pt(1, 1)
}
sketch d6(on: offset(xy, 0.0007)) {
  p = pt(-1, 1)
}
sketch s7(on: plane(at: d4.p, normal: axis(o.p, d5.p), x: axis(o.p, d6.p))) {
  p1 = poly([(0, 15), (0, 0), (18, 0)])
}
cut2 = cut(s7, -15)
"#,
    );
}

#[test]
fn tool_edge_along_two_collinear_edges_rounded_off_their_wall() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Z, x: X)) {
  p1 = poly([(80, 90), (75, 102), (70, 114), (75, 120), (280, 280), (300, 100)])
}
b0 = extrude(s1, 300)
sketch d2(on: offset(xy, 0)) {
  p = pt(-5, 54.583333333)
}
sketch d3(on: offset(xy, 0)) {
  p = pt(-0.384615385, 0.923076923)
}
sketch d4(on: offset(xy, 0)) {
  p = pt(0.923076923, 0.384615385)
}
sketch s5(on: plane(at: d2.p, normal: axis(o.p, d3.p), x: axis(o.p, d4.p))) {
  p1 = poly([(92.083333333, 0), (212.083333333, -150), (32.083333333, -150), (32.083333333, 0)])
}
cut1 = cut(s5, -26)
"#,
    );
}

#[test]
fn corner_on_a_rim_the_coplanar_edge_all_but_runs_along() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Z, x: X)) {
  c1 = circle((30, 8), 3)
}
b0 = extrude(s1, 9)
sketch s2(on: plane(at: o.p, normal: -Z, x: -X)) {
  p1 = poly([(-29.9993, 10.9999998), (-24, 10.9999998), (-24, 6), (-30, 6), (-36, 6), (-36, 10.9999998)])
}
extrude2 = extrude(s2, 10)
"#,
    );
}

#[test]
fn floor_of_a_d_hole_narrower_than_the_usual_probe() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Z, x: X)) {
  c1 = circle((0, 0), 12)
  a2 = arc(center: (0, 0), from: (2.0000000000000004, -3.4641016151377544), to: (2.0601522996402166, -3.4286692028084493))
  l3 = line((2.0601523, -3.428669203), (2, -3.464101615))
}
b0 = extrude(s1, 4)
sketch s2(on: plane(at: o.p, normal: -Z, x: X)) {
  c1 = circle((0, 0), 12)
}
extrude2 = extrude(s2, 2)
"#,
    );
}

#[test]
fn arcs_ending_on_a_wall_a_hair_off_the_cylinder_axis() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch d1(on: offset(xy, -50)) {
  p = pt(0, 0)
}
sketch s2(on: plane(at: d1.p, normal: Z, x: X)) {
  p1 = poly([(10, 0), (10.000001, -30), (50, -30), (50, 0)])
}
b0 = extrude(s2, 100)
sketch s3(on: plane(at: o.p, normal: -Y, x: -X)) {
  a1 = arc(center: (-30, 0), from: (-10.000000000040831, 15.000000000054445), to: (-10.000000000040831, -15.000000000054445), cw: true)
  a2 = arc(center: (-30, 0), from: (-10.000000000040831, -15.000000000054445), to: (-49.99999999995917, -15.000000000054445), cw: true)
  a3 = arc(center: (-30, 0), from: (-49.99999999995917, -15.000000000054445), to: (-49.999999999959165, 15.00000000005445), cw: true)
  a4 = arc(center: (-30, 0), from: (-49.99999999995917, 15.000000000054445), to: (-10.000000000040831, 15.000000000054445), cw: true)
}
cut1 = cut(s3, -5)
"#,
    );
}

const CROSSED_THEN_FILLED: &str = r#"sketch s1(on: xy) {
  r1 = rect(corner: (-20, -20), size: (40, 40))
}
extrude1 = extrude(s1, 20)
sketch d2(on: offset(xy, 10)) {
  p = pt(-30, 0)
}
sketch s3(on: plane(at: d2.p, normal: X, x: Y)) {
  c1 = circle((0, 0), 3)
}
cut1 = cut(s3, -60)
sketch d4(on: offset(xy, 10)) {
  p = pt(0, 30)
}
sketch s5(on: plane(at: d4.p, normal: -Y, x: X)) {
  c1 = circle((0, 0), 3)
}
cut2 = cut(s5, -60)
sketch s6(on: plane(at: d2.p, normal: X, x: Y)) {
  c1 = circle((0, 0), 3)
}
extrude2 = extrude(s6, 60)
"#;

// Equal bores crossing, one filled again: what is left of the other touches itself at two points.
// A feature that clears them leaves a sound part: only the finished part has to be sound.
#[test]
fn pinch_at_an_intermediate_line_builds_when_the_part_ends_sound() {
    let b = built(CROSSED_THEN_FILLED);
    let fans = |p: &forge_topo::Problem| matches!(p, forge_topo::Problem::VertexFans { .. });
    assert!(forge_topo::check(&b.body).iter().any(fans));
    let src = format!(
        "{CROSSED_THEN_FILLED}sketch s7(on: xy) {{
  c1 = circle((0, 0), 5)
}}
cut3 = cut(s7, -20)
"
    );
    // block 32000, r5 bore 500 pi, r3 bore outside it 360 pi - 269.3706 (the integral of
    // 2 sqrt(9 - x^2) 2 sqrt(25 - x^2) over |x| < 3), the fill's stubs past the block 180 pi
    near(
        builds(&src),
        32000.0 - 500.0 * dmath::PI - 360.0 * dmath::PI + 269.3706 + 180.0 * dmath::PI,
    );
}

// The check's slack is for rounding past the floor: past it, an edge the tolerance hides is still found.
#[test]
fn vertex_tolerance_slack_hides_nothing_past_the_floor() {
    let base = built(
        r#"sketch s1(on: xy) {
  p1 = poly([(0, 0), (10, 0), (10, 5), (5.000015, 5), (5, 5), (0, 5)])
}
extrude1 = extrude(s1, 5)
"#,
    )
    .solids[0]
        .body
        .compact();
    assert!(forge_topo::check(&base).is_empty());
    let hair = base
        .vertex_ids()
        .find(|&v| (base.vertex(v).point.x - 5.000015).abs() < 1e-9)
        .unwrap();
    let with_tol = |t: f64| {
        let mut b = base.clone();
        b.vertices[hair.index()].tol = t;
        forge_topo::check(&b)
    };
    let hides = |p: &forge_topo::Problem| matches!(p, forge_topo::Problem::VertexTolerance { .. });
    assert!(with_tol(1.05e-5).is_empty());
    assert!(with_tol(1.11e-5).iter().any(hides));
    assert!(with_tol(5e-5).iter().any(hides));
    // nor does it reach the distance a vertex may lie off its edge
    let mut off = base.clone();
    off.vertices[hair.index()].point.y += 1.05e-5;
    assert!(
        forge_topo::check(&off)
            .iter()
            .any(|p| matches!(p, forge_topo::Problem::VertexOffEdge { .. }))
    );
}

// ---- gaps: what the .fc parts did that Forge cannot write yet

/// A disc a hair inside its scoop, the scoop's sides running into its arcs as knife edges.
/// The .fc part:
///   extrude 2 z 0 1 0 x 1 0 0 at 0 0 0 loop arc 0 0 4.5 170 -160 line 4.431635 0.781417 7.878462 -18.610815 arc 0 -20 8 10 80 arc 0 -20 8 90 80 line -7.878462 -18.610815 -4.431635 0.781417 circle 0 0 2.25
///   extrude add 5 z 0 1 0 x 1 0 0 at 0 0 0 loop arc 0 -20 8 170 -80 arc 0 -20 8 90 -80 arc 0 -20 8 10 -200 circle 0 -20 4
#[test]
#[ignore = "gap: Forge refuses the scoop's sketch (SketchProfile: self intersection) where its sides meet the arcs as knife edges; .fc handed the loop to the kernel, which builds it"]
fn disc_in_a_scoop_with_knife_edges() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  a1 = arc(center: (0, 0), from: (-4.431634888554936, 0.7814167995011863), to: (4.431634888554936, 0.7814167995011865), cw: true)
  l2 = line((4.431634888554936, 0.7814167995011865), (7.878462024097664, -18.610814578664556))
  a3 = arc(center: (0, -20), from: (7.878462024097664, -18.610814578664556), to: (4.898587196589413e-16, -12))
  a4 = arc(center: (0, -20), from: (4.898587196589413e-16, -12), to: (-7.878462024097664, -18.61081457866456))
  l5 = line((-7.878462024097664, -18.61081457866456), (-4.431634888554936, 0.7814167995011863))
  c6 = circle((0, 0), 2.25)
}
extrude1 = extrude(s1, 2)
sketch s2(on: plane(at: o.p, normal: Y, x: X)) {
  a1 = arc(center: (0, -20), from: (-7.878462024097664, -18.61081457866456), to: (4.898587196589413e-16, -12), cw: true)
  a2 = arc(center: (0, -20), from: (4.898587196589413e-16, -12), to: (7.878462024097664, -18.610814578664556), cw: true)
  a3 = arc(center: (0, -20), from: (7.878462024097664, -18.610814578664556), to: (-7.878462024097664, -18.610814578664556), cw: true)
  c4 = circle((0, -20), 4)
}
extrude2 = extrude(s2, 5)
"#,
    );
}

/// A boss whose bore touches its flat at a vertex on a plate: its cap's two loops meet at a point the
/// triangulation parts by a hair, the 3D points staying where they are. The .fc part:
///   extrude 6 z 0 1 0 x 1 0 0 at 0 0 0 poly 0 -30 0 30 120 30 120 -30 circle 14 0 7 circle 106 0 7
///   extrude add 60 z 0 0 -1 x 1 0 0 at 0 0 30 loop arc 60 -28 34 40.320215 -260.64043 line 34.077037 -6 60 -6 line 60 -6 85.922963 -6 circle 60 -28 22
#[test]
#[ignore = "gap: Forge refuses a sketch whose hole touches its outline at a point (SketchProfile: self intersection); .fc handed it to the kernel, which builds it"]
fn cap_whose_loops_touch_meshes() {
    builds(
        r#"sketch o(on: xy) {
  p = pt(0, 0)
}
sketch s1(on: plane(at: o.p, normal: Y, x: X)) {
  p1 = poly([(0, -30), (0, 30), (120, 30), (120, -30)])
  c2 = circle((14, 0), 7)
  c3 = circle((106, 0), 7)
}
extrude1 = extrude(s1, 6)
sketch d2(on: offset(xy, 30)) {
  p = pt(0, 0)
}
sketch s3(on: plane(at: d2.p, normal: -Z, x: X)) {
  a1 = arc(center: (60, -28), from: (85.92296282049824, -6.000000031657592), to: (34.07703717950176, -6.000000031657596), cw: true)
  l2 = line((34.07703717950176, -6.000000031657596), (60, -6))
  l3 = line((60, -6), (85.92296282049824, -6.000000031657592))
  c4 = circle((60, -28), 22)
}
extrude2 = extrude(s3, 60)
"#,
    );
}

// A sheared prism keeps its section's area at every height (Cavalieri): area x height. The .fc
// parts, `extrude D along X Y Z PROFILE` (the walls run along X Y Z, the ends D along the normal):
//   extrude 10 along 0.3 0.1 1 rect 0 0 8 5            400
//   extrude 10 along -2 0 1 circle 0 0 3               90 pi
//   extrude 10 along 0 1 1 circle 0 0 4 circle 0 0 1   150 pi
//   extrude -10 along 0.5 0 1 rect 0 0 8 5             400
//   extrude 10 along -0.5 0 -1 rect 0 0 8 5            400
//   extrude sym 10 along 0.5 0 1 circle 0 0 3          90 pi
//   extrude two 6 4 along 0.5 0 1 rect 0 0 8 5        400
// and tools: a plate 40 x 40 x 10 less `extrude cut 10 along 0.5 0 1 circle 0 0 3` (16000 - 90 pi),
// plus `extrude add 8 along 0.5 0 1 at 0 0 10 rect -5 -5 10 10` (16800), less `extrude cut thru
// along 0.5 0 1 circle 0 0 3`; and `extrude 10 along 1 0 0 rect 0 0 8 5` refused (in the plane).
// The closest Forge: the same extrudes with an `along:` direction, which `extrude` does not take.
#[test]
#[ignore = "gap: Forge's extrude has no oblique direction (.fc `extrude .. along X Y Z`)"]
fn oblique_extrude_keeps_area_times_height() {
    let src = "sketch s(on: xy) {
  r = rect(corner: (0, 0), size: (8, 5))
}
sketch d(on: offset(xy, 1)) {
  p = pt(0.3, 0.1)
}
sketch o(on: xy) {
  p = pt(0, 0)
}
sheared = extrude(s, 10, along: axis(o.p, d.p))
";
    near(builds(src), 400.0);
}

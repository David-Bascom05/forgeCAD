//! Parts modeled the way real CAD histories model them (research/find-parts), each valid feature
//! by feature and n-ary, the two paths the same solid.
use forge_agent::model::Model;
use forge_math::dmath;
use forge_mesh::{Quality, tessellate};
use forge_model::Part;

fn volume(b: &forge_topo::Body) -> f64 {
    let q = Quality {
        chord: 0.002,
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
        near(builds(&format!("extrude 8 taper {angle} circle 0 0 10")), want);
    }
}

/// A rectangle's walls are planes re-met at the corners; extruded down its frame is flipped.
#[test]
fn tapered_rect_is_a_pyramid_frustum() {
    let d = 5.0 * tan(10.0);
    let (a, b) = (40.0 - 2.0 * d, 20.0 - 2.0 * d);
    let want = 5.0 / 3.0 * (800.0 + a * b + (800.0 * a * b).sqrt());
    near(builds("extrude 5 taper 10 rect 0 0 40 20"), want);
    near(builds("extrude -5 taper 10 rect 0 0 40 20"), want);
}

/// A hole flares where the outline narrows: the profile moves in one sense all round.
#[test]
fn tapered_ring_narrows_both_walls() {
    let d = 6.0 * tan(5.0);
    let frustum = |r: f64, q: f64| dmath::PI * 6.0 / 3.0 * (r * r + r * q + q * q);
    near(
        builds("extrude 6 taper 5 circle 0 0 20 circle 0 0 10"),
        frustum(20.0, 20.0 - d) - frustum(10.0, 10.0 + d),
    );
}

/// Tapered tools cut and join like any other; lines and tangent arcs both lean.
#[test]
fn tapered_tools_cut_and_add() {
    let plate = "extrude 10 rect -30 -30 60 60\n";
    let d = 4.0 * tan(5.0);
    let cone = dmath::PI * 4.0 / 3.0 * (100.0 + 10.0 * (10.0 - d) + (10.0 - d) * (10.0 - d));
    near(
        builds(&format!(
            "{plate}extrude cut 4 taper 5 -xy at 0 0 10 circle 0 0 10\n"
        )),
        36000.0 - cone,
    );
    near(
        builds(&format!(
            "{plate}extrude add 4 taper 5 xy at 0 0 10 circle 0 0 10\n"
        )),
        36000.0 + cone,
    );
    builds("extrude 5 taper 3 loop line 0 0 20 0 arc 20 5 5 -90 180 line 20 10 0 10 arc 0 5 5 90 180");
}

/// Arcs of one circle whose centres carry noise: each pair within tolerance of one surface, the
/// chain of them not. Faces merged all the way left edges off the face they joined.
#[test]
fn arcs_a_hair_off_one_circle_stay_on_their_faces() {
    let v = builds(
        "extrude 24 loop arc 0 -0.000008 15 -135 90 arc 0 0 15 -45 90 arc 0 0.000008 15 45 90.00004 arc 0 0 15 135 90",
    );
    near(v, dmath::PI * 225.0 * 24.0);
}

/// A long window in a face curved across it, a hair inside the face's end: the point that says
/// which region holds the hole was a thousandth of the window's length off it, on the far side.
#[test]
fn window_near_the_end_of_a_curved_face() {
    let block = "extrude 40 z 0 0 1 x 1 0 0 at 0 0 0 loop line 50 0 0 0 line 0 0 0 70 line 0 70 10 70 arc 10 30 40 90 -90 line 50 30 50 0\n";
    let area = 700.0 + 1200.0 + dmath::PI * 400.0;
    let mut cut = 0.0;
    let n = 4000;
    for i in 0..n {
        let x = 11.0 + 37.0 * (i as f64 + 0.5) / n as f64;
        cut += (30.0 + (1600.0 - (x - 10.0) * (x - 10.0)).sqrt()) * 37.0 / n as f64;
    }
    let v = builds(&format!(
        "{block}extrude cut 140 z 0 1 0 x 1 0 0 at 0 0 0 poly 48 -38 11 -38 11 -2 48 -2\n"
    ));
    near(v, area * 40.0 - cut * 36.0);
}

/// A disc on a tool's flat underside, its rim tangent to the underside's edges: the rim's middle
/// lands on the far edge, and it is still a new edge of the underside. Not a pinch: the disc is
/// joined to the tool across its whole top.
#[test]
fn disc_rim_tangent_to_a_flat_it_sits_on() {
    let v = builds(
        "extrude 5 z 0 1 0 x 1 0 0 at 0 0 0 circle 0 0 16 circle 0 0 5
extrude add two 25 25 z 0 0 1 x 1 0 0 at 0 0 0 poly -18 21 18 21 16 5.00000015 -16 5
",
    );
    near(v, dmath::PI * 231.0 * 5.0 + 34.0 * 16.0 * 50.0);
}

/// A lug on a block's top, its round side tangent to the block's end face along the top's edge:
/// each face ends where the other starts and one skin runs on. Faces facing apart would be a pinch.
#[test]
fn round_side_runs_on_tangent_from_a_face() {
    let v = builds(
        "extrude 40 z 0 0 1 x 1 0 0 at 0 0 0 poly 0 0 0 30 15 30 15 12 100 12 100 0
extrude add 15 z -1 0 0 x 0 0 -1 at 15 0 0 loop line 0 -30 -9 -30 arc -20 -30 11 0 -180 line -31 -30 -40 -30 arc -20 -30 20 180 180
",
    );
    near(v, 1470.0 * 40.0 + dmath::PI / 2.0 * 279.0 * 15.0);
}

/// A plate standing against a rod, tangent along a line: true zero thickness, refused.
#[test]
fn plate_tangent_to_a_rod_is_a_pinch() {
    let m = Model::new(
        "extrude 100 z 0 1 0 x 1 0 0 at 0 0 0 circle 0 0 20
extrude add two 5 5 z 0 0 1 x 1 0 0 at 0 0 0 poly -20 30 -20 90 -60 90 -60 30
",
    );
    let e = m.error.as_ref().expect("refused");
    assert!(e.msg.contains("ZeroThickness"), "{}", e.msg);
}

/// A cross hole whose axis lies a hair inside a tube's surface: where the hole's curve crosses its
/// own seam is anywhere along a stretch, and the tube's seam crosses that stretch. One vertex there.
#[test]
fn cross_hole_tangent_to_a_tube() {
    // (OCCT agrees: tools/occt)
    let v = builds(
        "extrude two 15 15 z 1 0 0 x 0 0 -1 at 0 0 0 circle 0 0 25 circle 0 0 22.5
extrude cut two 25 25 z 0 1 0 x 1 0 0 at 0 0 0 circle 0 24.99999988 4
",
    );
    near(v, 10830.571);
}

/// Coaxial bores stacked with their seams apart join into a face whose seam steps; a hole in it goes
/// in the span the outline has at its height, not the first that holds its middle.
#[test]
fn hole_in_a_bore_whose_seam_steps() {
    let v = builds(
        "extrude 40 z 0 0 -1 x 1 0 0 at 0 0 0 rect -40 -60 80 120 circle 0 20 15
extrude add 30 z 0 0 -1 x -1 0 0 at 0 0 -40 rect -50 -70 100 140 circle 0 -20 15
extrude cut 40 z 0 1 0 x 1 0 0 at 0 -70 0 circle 0 25 7
",
    );
    near(v, 750606.77);
}

/// A taper past where the profile closes stops there: a pyramid, a roof, a cone.
#[test]
fn taper_past_the_profile_closing() {
    near(
        builds("extrude 70 taper 16 poly 20 20 20 -20 -20 -20 -20 20"),
        1600.0 * (20.0 / tan(16.0)) / 3.0,
    );
    near(
        builds("extrude 15 taper 45 rect 0 0 40 20"),
        10.0 / 6.0 * (80.0 + 20.0) * 20.0,
    );
    near(
        builds("extrude 20 taper 45 circle 0 0 10"),
        dmath::PI * 100.0 * 10.0 / 3.0,
    );
    near(builds("extrude 20 taper 45 poly 0 0 30 0 0 30"), {
        let r = 30.0 / (2.0 + 2.0f64.sqrt());
        450.0 * r / 3.0
    });
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
    near(builds("extrude 10 taper 30 rrect 0 0 40 40 2"), want);
}

/// A hole's axis a hair inside a tube's surface, its seam crossing the tube twice a hair apart: the
/// curve runs out and back along the seam. One vertex for the two.
#[test]
fn cross_hole_seam_out_and_back_over_a_tube() {
    let v = builds(
        "extrude two 15 15 z 1 0 0 x 0 0 -1 at 0 0 0 circle 0 0 25.0000012 circle 0 0 22.5
extrude cut two 25 25 z 0 1 0 x 1 0 0 at 0 0 0 circle 0 -25 4
",
    );
    near(v, 10830.576);
}

/// Bodies butted with corners a hair apart: "within tolerance" chains, a vertex near two cuts that
/// are not near each other must not end in two vertices.
#[test]
fn butted_corners_chain_within_tolerance() {
    builds(
        "extrude 2 z 0 -1 0 x 1 0 0 at 0 0 0 poly 2.449811762 -0.498419836 2.5 0 -2.5 0 -2.5 -0.505 0 -12.0000003
extrude add 2 z 0 1 0 x 1 0 0 at 0 0 0 loop arc 0 0 2.5 180 180 line 2.50000004 0 2.449811762 0.498419836 arc 0 0 2.5 11.5 157 line -2.4498118 0.4984198 -2.5 0
",
    );
}

/// Two blocks flush along a wall, each rounding off from it at one height: beyond, a groove of no
/// width at its root. One skin turns back along the root; below it the blocks are one.
#[test]
fn groove_rooted_where_rounded_corners_part() {
    let v = builds(
        "extrude 10 loop line 5 -20 5 20 arc 4 20 1 0 90 line 4 21 -20 21 line -20 21 -20 -20 line -20 -20 5 -20
extrude add 6 loop line 5 -20 30 -20 line 30 -20 30 21 line 30 21 6 21 arc 6 20 1 90 90 line 5 20 5 -20
",
    );
    let area = 25.0 * 41.0 - (1.0 - dmath::PI / 4.0);
    near(v, area * 16.0);
}

/// A boss flush on a rod, one wall leaving the rod tangent: its knife edge's two sides run within
/// tolerance for a stretch, and the tip ends where they part. Not a pinch.
#[test]
fn knife_edge_tangent_on_a_rod() {
    let v = builds(
        "extrude 30 circle 0 0 20
extrude add 15 loop arc 0 0 20 100 50 line -17.320508 10 -2.320508 35.980762 line -2.320508 35.980762 -3.472964 19.696155
",
    );
    near(v, dmath::PI * 400.0 * 30.0 + 85.84 * 15.0);
}

/// Holes on a tube's far sides, one over its seam: the outline, stepped round that hole, need not come
/// first among the face's loops, and the other hole belongs in its window.
#[test]
fn holes_across_a_tube_one_on_its_seam() {
    let v = builds(
        "extrude two 2 2 z 1 0 0 x 0 0 -1 at 0 0 0 circle 0 0 4 circle 0 0 2.5
extrude add two 2 2 z 1 0 0 x 0 0 -1 at 0 0 0 loop line -3.064178 2.57115 -3.255689 2.731847 arc 0 0 4.25 140 -100 line 3.255689 2.731847 3.064178 2.57115 arc 0 0 4 40 100
extrude cut two 4 4 z 0 1 0 x 1 0 0 at 0 0 0 circle 0 4 0.6
extrude cut two 4 4 z 0 1 0 x 1 0 0 at 0 0 0 circle 0 -4 0.6
",
    );
    near(v, 126.75731);
}

/// A flat running on into an arc whose lowest point lies a hair past their shared edge: the flat's
/// curve with a rod has its tangent root off the edge, and still ends on it.
#[test]
fn curve_ends_on_an_edge_past_its_tangent_root() {
    let v = builds(
        "extrude 150 z 1 0 0 x 0 0 -1 at 0 0 0 circle 0 0 14
extrude cut two 18 18 z 0 1 0 x 1 0 0 at 0 0 0 loop arc 14.00007 -28 22 22.728422597513 67.271777937715 line 14 -6 0 -6 line 0 -6 0 -19.5 line 0 -19.5 14 -19.5 line 14 -19.5 34.291694 -19.5
",
    );
    near(v, 88776.474);
}

/// A plate's end a hair into a tube's bore, along a line of its wall, joined across: the bore's wall
/// keeps no slit there.
#[test]
fn plate_end_tangent_inside_a_bore_leaves_no_slit() {
    let m = Model::new(
        "b0: extrude 40 z 0 1 0 x 1 0 0 at 0 0 0 circle -14 0 6 circle -14 0 4.5
b1: extrude new 40 z 0 1 0 x 1 0 0 at 0 0 0 circle 14 0 6 circle 14 0 4.5
extrude add 8 z 0 0 1 x 1 0 0 at 0 0 0 poly -9.5 12 -9.5 30 9.50000015 30 9.50000015 12 in b0
combine add b0 b1
",
    );
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let b = m.body().expect("a body");
    assert!(forge_topo::check(b).is_empty());
    volume(b);
}

/// Knife edges one after another, each refused plain and built blunted: a retry rebuilding the part
/// through retries of its own doubled with each (16 took 30 s, 20 over 40). Linear: under a minute in
/// any profile, however loaded.
#[test]
fn many_knife_edges_stay_linear() {
    let mut src = String::new();
    for k in 0..40 {
        let x = 200.0 * k as f64 + 7.0;
        src += &format!(
            "extrude {}20 circle {x} 0 18\nextrude add 8 loop arc {x} -25 10 163.0265 -36.348 arc {x} 0 18 -109.3809 -45.6191 line {:.6} -7.607129 {:.6} -22.080708\n",
            if k == 0 { "" } else { "add " },
            x - 16.31354,
            x - 9.564399,
        );
    }
    let t = std::time::Instant::now();
    let m = Model::new(&src);
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let part = Part {
        features: m.features.clone(),
    };
    part.regenerate().expect("sequential");
    part.regenerate_nary(1).expect("n-ary");
    assert!(t.elapsed().as_secs() < 60, "{:?}", t.elapsed());
    // and in several bodies
    let mut src = String::new();
    for k in 0..40 {
        let x = 200.0 * k as f64 + 7.0;
        src += &format!(
            "b{k}: extrude {}20 circle {x} 0 18\nextrude add 8 loop arc {x} -25 10 163.0265 -36.348 arc {x} 0 18 -109.3809 -45.6191 line {:.6} -7.607129 {:.6} -22.080708 in b{k}\n",
            if k == 0 { "" } else { "new " },
            x - 16.31354,
            x - 9.564399,
        );
    }
    let t = std::time::Instant::now();
    let m = Model::new(&src);
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let part = Part {
        features: m.features.clone(),
    };
    part.regenerate_solids().expect("sequential");
    part.regenerate_nary_solids(1).expect("n-ary");
    assert!(t.elapsed().as_secs() < 60, "{:?}", t.elapsed());
}

/// A plate whose scoop a disc sits in, ending in knife edges: it is the plate's tips, already in the
/// part, that the disc trips on. The part again with its knife edges blunted builds.
#[test]
fn disc_in_a_scoop_with_knife_edges() {
    builds(
        "extrude 2 z 0 1 0 x 1 0 0 at 0 0 0 loop arc 0 0 4.5 170 -160 line 4.431635 0.781417 7.878462 -18.610815 arc 0 -20 8 10 80 arc 0 -20 8 90 80 line -7.878462 -18.610815 -4.431635 0.781417 circle 0 0 2.25
extrude add 5 z 0 1 0 x 1 0 0 at 0 0 0 loop arc 0 -20 8 170 -80 arc 0 -20 8 90 -80 arc 0 -20 8 10 -200 circle 0 -20 4
",
    );
}

/// A hexagon's hole a hair inside its flats, a disc flush in the hole: every step off a flat's middle
/// crosses the hole, so the underside's probe goes off another piece.
#[test]
fn hole_a_hair_inside_the_flats_of_its_outline() {
    let v = builds(
        "extrude 5 z 1 0 0 x 0 0 -1 at 0 0 0 circle 0 0 9 circle 0 0 4.5
extrude add 5 z 1 0 0 x 0 0 -1 at 0 0 0 poly 5.1965 9.0002 -5.1965 9.0002 -10.393 0 -5.1965 -9.0002 5.1965 -9.0002 10.393 0 circle 0 0 9
",
    );
    near(v, 5.0 * (3.0 * 10.393 * 9.0002 - dmath::PI * 20.25));
}

/// A boss whose bore touches its flat at a vertex on a plate: its cap's two loops meet at a point the
/// triangulation parts by a hair, the 3D points staying where they are.
#[test]
fn cap_whose_loops_touch_meshes() {
    let m = Model::new(
        "extrude 6 z 0 1 0 x 1 0 0 at 0 0 0 poly 0 -30 0 30 120 30 120 -30 circle 14 0 7 circle 106 0 7
extrude add 60 z 0 0 -1 x 1 0 0 at 0 0 30 loop arc 60 -28 34 40.320215 -260.64043 line 34.077037 -6 60 -6 line 60 -6 85.922963 -6 circle 60 -28 22
",
    );
    assert!(volume(m.body().expect("a body")) > 0.0);
}

// (the hair edge, a line ending within a tolerance of where it meets an arc; volume from `builds` on both paths)
#[test]
fn hair_edge_at_the_tolerance_floor_is_valid() {
    builds(
        "b0: extrude 2 z 0 1 0 x 1 0 0 at 0 0 0 loop arc 30 -40 5 -75.52249181407 151.04497562814 arc 30 -40 5 75.52248381407 208.95502437186 circle 30 -40 2.5
extrude add 2 z 0 1 0 x 1 0 0 at 0 2 0 loop arc 42 -40.000001 2 -75.52249181407 151.04497562814 line 42.500000135 -38.063509362 31.250000338 -35.158770905 arc 30 -40 5 75.52248381407 208.95502437186 line 31.249999662 -44.84122927 42.499999865 -41.936492708 circle 30 -40 2.5 circle 42 -40.000001 1
",
    );
}

#[test]
fn wall_slivers_shallower_than_the_chord_mesh_closed() {
    builds(
        "b0: extrude 12 z 0 1 0 x 1 0 0 at 0 0 0 circle 0 0 9 loop arc 0 0 5.0 -1.604491527 3.208983053 line 4.998039616 0.140000000 -2.377776251 4.398429276 arc 0 0 5.0 118.395508473 3.208983053 line -2.620263364 4.258429276 -2.620263364 -4.258429276 arc 0 0 5.0 238.395508473 3.208983053 line -2.377776251 -4.398429276 4.998039616 -0.140000000
extrude add 4 z 0 1 0 x 1 0 0 at 0 0 0 poly 4.998039616 -0.140000000 4.998039616 0.140000000 -2.377776251 4.398429276 -2.620263364 4.258429276 -2.620263364 -4.258429276 -2.377776251 -4.398429276
",
    );
}

#[test]
fn slit_left_in_a_face_by_a_tool_edge_meshes() {
    builds(
        "b0: extrude 40 z 0 1 0 x 1 0 0 at 0 0 0 poly -10 -20 -10 20 10 20 10 -20
extrude cut 15 z -1 0 0 x 0 0 -1 at 10 0 0 poly -20 -15 -20 -40 6 -40 6 -27 -9 -27
extrude cut 15 z 1 1 0 x -1 1 0.0007 at -3 27 -6 poly 0 15 0 0 18 0
",
    );
}

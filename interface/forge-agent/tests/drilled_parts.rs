//! Part files from realistic drillings that once broke a boolean: each builds valid feature by
//! feature (as the app does) and in one pass, on one thread and on two.
use forge_agent::model::Model;
use forge_model::Part;

fn builds_valid(src: &str) -> f64 {
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let seq = m.body().expect("a body");
    assert!(forge_topo::check(seq).is_empty(), "{:?}", forge_topo::check(seq));
    let part = Part {
        features: m.features.clone(),
    };
    let q = forge_mesh::Quality {
        chord: 0.002,
        max_angle: 0.05,
    };
    let v = forge_mesh::tessellate(seq, &q).expect("meshes").volume();
    for threads in [1, 2] {
        let b = part.regenerate_nary(threads).expect("regenerates in one pass");
        assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
        let w = forge_mesh::tessellate(&b, &q).expect("meshes").volume();
        assert!((v - w).abs() <= 1e-6 * v, "{v} vs {w}");
    }
    v
}

/// Two equal through holes whose axes cross (a + drilling): the bores meet in two ellipses
/// crossing where they touch. Removed: two bores less the Steinmetz solid 16 r^3 / 3 they share.
#[test]
fn equal_cross_holes() {
    let v = builds_valid(
        "extrude 40 rect -20 -20 40 40\nhole 8 -xy at 0 0 40 thru\nhole 8 yz at -20 0 20 thru\n",
    );
    let want = 64000.0 - (2.0 * forge_math::dmath::PI * 16.0 * 40.0 - 16.0 * 64.0 / 3.0);
    assert!((v - want).abs() < 1e-3 * (64000.0 - want), "{v} vs {want}");
    // a T, ending at the other's axis
    builds_valid("extrude 40 rect -20 -20 40 40\nhole 8 -xy at 0 0 40 thru\nhole 8 yz at -20 0 20 20\n");
}

/// A cross hole whose lowest line is tangent to another hole's flat floor: the bore curve
/// touches the floor circle from inside the bore's wall, a vertex on both.
#[test]
fn cross_drill_tangent_to_floor() {
    builds_valid("extrude 55 rect -45 -35 90 70\nhole 7.5 -xy at 0 0 55 30\nhole 10 xz at 0 35 30 35\n");
}

/// The same with drill points, the cross hole drilled to a plane through the first's axis.
#[test]
fn cross_drill_tangent_to_floor_pointed() {
    builds_valid(
        "extrude 55 rect -45 -35 90 70\nhole 8 -xy at 0 -12 55 35 pt 118\nyP: plane -xz at 0 -12 0\nhole 10 -xz at 0 -35 25 to yP pt 118\n",
    );
}

/// A blind hole whose rim circle just touches a chamfer's top boundary line (the curve touching
/// an edge from inside, which gets a vertex on both since the touch vertices went in).
#[test]
fn hole_tangent_to_chamfer_edge() {
    builds_valid("extrude 55 rect -45 -35 90 70\nchamfer 1.5 (edges)\nhole 15 -xy at -36 -27 55 9\n");
}

/// Two blocks apart in one body: two lumps, a shell each (the union once kept one shell for
/// both, and check counted its Euler characteristic as 4).
#[test]
fn disjoint_lumps_are_shells_apart() {
    let v = builds_valid("extrude 10 rect 0 0 10 10\nextrude 10 rect 20 0 10 10\n");
    assert!((v - 2000.0).abs() < 1e-6 * 2000.0, "{v}");
    let m = Model::new("extrude 10 rect 0 0 10 10\nextrude 10 rect 20 0 10 10\n");
    assert_eq!(m.body().expect("a body").shells.len(), 2);
}

/// A pocket whose mouth stands 2e-5 (two tolerances) above the top face, and a pad laid 2e-5
/// under the top across it: the pocket's wall keeps a sliver two tolerances tall under the pad.
/// The n-ary regen without shapes probed that sliver from a point a step outside it and kept it,
/// leaving open edges; every point of it is within tolerance of a face, so it is refused, never
/// built invalid.
#[test]
fn sliver_two_tolerances_tall() {
    let m = Model::new(
        "extrude new 10 rect 0 0 60 40
extrude cut 3.00001 at 15.77764088281603 16.262082660203596 7.00001 circle 0 0 3
extrude add 3 at 15.14935454448211 11.64952639287997 9.99998 rect 0 0 10 8
",
    );
    assert!(m.error.is_none());
    let part = Part {
        features: m.features.clone(),
    };
    for shapes in [true, false] {
        if let Ok(b) = part.regenerate_nary_with(1, shapes) {
            assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
        }
    }
}

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

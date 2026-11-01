//! Shells whose walls meet as the thickness grows: the topology of the inner skin changes. Each
//! against the closed form of what the wall took, and built the same feature by feature and in one
//! pass.
use forge_agent::model::Model;
use forge_model::Part;

/// The part's exact volume; it must build valid, and the same in one pass.
fn volume(src: &str) -> f64 {
    let m = Model::new(src);
    assert!(m.error.is_none(), "{:?}", m.error.as_ref().map(|e| &e.msg));
    let b = m.body().expect("a body");
    assert!(forge_topo::check(b).is_empty(), "{:?}", forge_topo::check(b));
    let v = forge_topo::mass::mass(b).expect("a volume").volume;
    let part = Part {
        features: m.features.clone(),
    };
    let n = part.regenerate_nary(1).expect("regenerates in one pass");
    let w = forge_topo::mass::mass(&n).expect("a volume").volume;
    assert!((v - w).abs() <= 1e-9 * v, "{v} vs {w} in one pass");
    v
}

/// A boss 1.6 by 3 on a plate, shelled open below: thicker than half its width, its walls meet and
/// the wall runs on flat across under it (the plate's cavity alone); thinner, the cavity reaches up
/// into it, its footprint less the wall all round (this refused: the boss's walls crossed).
#[test]
fn a_boss_the_walls_close_over() {
    let part = "extrude 10 rect 0 0 40 30\nextrude 2 at 0 0 10 rect 15 10 1.6 3\n";
    let whole = 40.0 * 30.0 * 10.0 + 1.6 * 3.0 * 2.0;
    for (t, lobe) in [
        (1.0, 0.0),
        (0.9, 0.0),
        (0.5, 0.6 * 2.0 * 2.0),
        (0.3, 1.0 * 2.4 * 2.0),
    ] {
        let v = volume(&format!("{part}shell {t} (faces normal -z bottom)\n"));
        let cavity = (40.0 - 2.0 * t) * (30.0 - 2.0 * t) * (10.0 - t) + lobe;
        assert!(
            (whole - v - cavity).abs() < 1e-9 * whole,
            "t {t}: {} vs {cavity}",
            whole - v
        );
    }
}

/// A triangular boss, the walls meeting at its incentre: right-angled 3, 4, 5, its inradius 1.
#[test]
fn a_triangle_closed_over_past_its_inradius() {
    let part = "extrude 10 rect 0 0 40 30\nextrude 2 at 0 0 10 poly 10 10 13 10 10 14\n";
    let whole = 40.0 * 30.0 * 10.0 + 6.0 * 2.0;
    for (t, lobe) in [(1.2, 0.0), (0.5, 6.0 * 0.25 * 2.0)] {
        // the footprint less t all round: the same triangle scaled about the incentre by (1 - t / r)
        let v = volume(&format!("{part}shell {t} (faces normal -z bottom)\n"));
        let cavity = (40.0 - 2.0 * t) * (30.0 - 2.0 * t) * (10.0 - t) + lobe;
        assert!(
            (whole - v - cavity).abs() < 1e-9 * whole,
            "t {t}: {} vs {cavity}",
            whole - v
        );
    }
}

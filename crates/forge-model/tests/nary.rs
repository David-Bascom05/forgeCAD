//! The one-pass n-ary regen gives the same solids as the feature-by-feature one.
use forge_model::{suite, torture};

#[test]
fn nary_matches_sequential_on_the_suite() {
    let mut parts = suite::parts();
    parts.push(("torture", torture::part(221)));
    for (name, part) in parts {
        let seq = part.regenerate().expect("sequential regen");
        let nary = part.regenerate_nary(4).expect("n-ary regen");
        assert!(
            forge_topo::check(&nary).is_empty(),
            "{name}: {:?}",
            forge_topo::check(&nary)
        );
        let q = forge_mesh::Quality {
            chord: 0.05,
            max_angle: 0.5,
        };
        let (a, b) = (
            forge_mesh::tessellate(&seq, &q).expect("mesh").volume(),
            forge_mesh::tessellate(&nary, &q).expect("mesh").volume(),
        );
        assert!((a - b).abs() <= 1e-3 * a, "{name}: volume {a} vs {b}");
    }
}

#[test]
fn nary_matches_sequential_on_fuzz() {
    for seed in [1, 2, 5, 10, 20, 103] {
        let part = suite::fuzz(seed, 80);
        let nary = part.regenerate_nary(4).expect("n-ary regen");
        assert!(forge_topo::check(&nary).is_empty(), "seed {seed}");
        assert_eq!(
            nary.face_ids().count(),
            part.regenerate().expect("sequential regen").face_ids().count(),
            "seed {seed}"
        );
    }
}

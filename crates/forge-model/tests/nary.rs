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

/// Cut points at x = 0.0 and x = -0.0 fell in two slabs of the vertex clustering that compared
/// equal but sorted apart, so two of them that are one vertex stayed two: an open shell.
#[test]
fn fuzz11_signed_zero_cut_points() {
    let part = suite::fuzz(11, 80);
    for threads in [1, 2] {
        let nary = part.regenerate_nary(threads).expect("n-ary regen");
        assert!(
            forge_topo::check(&nary).is_empty(),
            "{threads} threads: {:?}",
            forge_topo::check(&nary)
        );
    }
}

/// Every suite part's n-ary result, bit for bit, at 1, 2 and 4 threads (wasm runs one).
#[test]
fn nary_bits_do_not_depend_on_threads() {
    let mut parts = suite::parts();
    parts.push(("torture", torture::part(221)));
    for (name, part) in parts {
        let bits = |threads: usize| {
            let mut h = forge_math::hash::BitHasher::new();
            match part.regenerate_nary(threads) {
                Ok(b) => {
                    for v in &b.vertices {
                        h.p3(v.point);
                    }
                    for e in &b.edges {
                        h.f64(e.range.lo);
                        h.f64(e.range.hi);
                    }
                    h.u64(b.faces.len() as u64);
                }
                Err(_) => h.u64(u64::MAX),
            }
            h.finish()
        };
        let one = bits(1);
        assert_eq!(bits(2), one, "{name}: 2 threads");
        assert_eq!(bits(4), one, "{name}: 4 threads");
    }
}

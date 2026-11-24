//! The one-pass n-ary regen gives the same solids as the feature-by-feature one.
use forge_model::{suite, torture};

#[test]
fn nary_matches_sequential_on_the_suite() {
    let mut parts = suite::parts();
    parts.push(("torture", torture::part(221)));
    suite::each(&parts, |&(name, ref part)| {
        let seq = part.regenerate().expect("sequential regen");
        let nary = part.regenerate_nary(4).expect("n-ary regen");
        assert!(
            forge_topo::check(&nary).is_empty(),
            "{name}: {:?}",
            forge_topo::check(&nary)
        );
        let mass = |b| forge_topo::mass::mass(b).expect("mass properties");
        let (a, b) = (mass(&seq), mass(&nary));
        let bound = a.err[0] + b.err[0] + 1e-12 * a.volume;
        assert!(
            (a.volume - b.volume).abs() <= bound,
            "{name}: volume {} vs {} (bound {bound})",
            a.volume,
            b.volume
        );
        // coincident faces: the one there first is kept, and keeps a later face's imprint
        assert_eq!(nary.face_ids().count(), seq.face_ids().count(), "{name}: faces");
    });
}

#[test]
fn nary_matches_sequential_on_fuzz() {
    suite::each(&[1, 2, 5, 10, 20, 103], |&seed| {
        let part = suite::fuzz(seed, 80);
        let nary = part.regenerate_nary(4).expect("n-ary regen");
        assert!(forge_topo::check(&nary).is_empty(), "seed {seed}");
        assert_eq!(
            nary.face_ids().count(),
            part.regenerate().expect("sequential regen").face_ids().count(),
            "seed {seed}"
        );
    });
}

/// `fuzz_keep`: histories with a large tool kept (an intersect) now and then, so the n-ary pass
/// has an `Intersect` among its tools (research/coverage-2026-11-23 found none in the corpus).
/// Seeds whose sequential regen builds: the n-ary one, at one thread and at four, the same solid.
#[test]
fn nary_matches_sequential_with_intersects() {
    suite::each(&[1, 2, 4, 8], |&seed| {
        let part = suite::fuzz_keep(seed, 80);
        let seq = part.regenerate().expect("sequential regen");
        for threads in [1, 4] {
            let nary = part.regenerate_nary(threads).expect("n-ary regen");
            assert!(
                forge_topo::check(&nary).is_empty(),
                "seed {seed}, {threads} threads"
            );
            assert_eq!(
                nary.face_ids().count(),
                seq.face_ids().count(),
                "seed {seed}, {threads} threads"
            );
            let mass = |b| forge_topo::mass::mass(b).expect("mass properties");
            let (a, b) = (mass(&seq), mass(&nary));
            let bound = a.err[0] + b.err[0] + 1e-12 * a.volume;
            assert!(
                (a.volume - b.volume).abs() <= bound,
                "seed {seed}, {threads} threads: volume {} vs {} (bound {bound})",
                a.volume,
                b.volume
            );
        }
    });
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
    suite::each(&parts, |&(name, ref part)| {
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
    });
}

/// The auto regen takes the sequential path for a short history and the n-ary one for a long.
#[test]
fn auto_picks_by_history_length() {
    let bits = |b: &forge_topo::Body| b.vertices.iter().map(|v| v.point.x.to_bits()).collect::<Vec<_>>();
    let short = suite::fuzz(3, 10);
    assert_eq!(
        bits(&short.regenerate_auto(2).expect("auto")),
        bits(&short.regenerate().expect("seq"))
    );
    let long = torture::part(221);
    assert_eq!(
        bits(&long.regenerate_auto(2).expect("auto")),
        bits(&long.regenerate_nary(2).expect("nary"))
    );
}

/// A pocket whose mouth stands 0.00002 above the block's top: the n-ary took the strip of wall
/// above the top for part of the wall below (its probe stepped across the strip), leaving open
/// edges. Both regens give the plain pocket.
#[test]
fn pocket_mouth_a_hair_above_the_top() {
    use forge_math::{Frame, Point2, Point3};
    use forge_model::{Combine, Extent, Feature, Part};
    use forge_ops::Profile;
    let ex = |o: Point3, l, d, op| Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD.with_origin(o),
            loops: vec![l],
        },
        extent: Extent::Distance(d),
        op,
    };
    let part = Part {
        features: vec![
            ex(
                Point3::ORIGIN,
                Profile::rectangle(Point2::ORIGIN, 60.0, 40.0, 0),
                10.0,
                Combine::New,
            ),
            ex(
                Point3::new(20.829744830901628, 31.98774163479715, 7.00001),
                Profile::circle(Point2::ORIGIN, 3.0, 0),
                3.00001,
                Combine::Cut,
            ),
        ],
    };
    let count = |b: &forge_topo::Body| (b.face_ids().count(), b.edge_ids().count(), b.vertex_ids().count());
    let seq = part.regenerate().expect("sequential regen");
    let nary = part.regenerate_nary(1).expect("n-ary regen");
    assert!(
        forge_topo::check(&nary).is_empty(),
        "{:?}",
        forge_topo::check(&nary)
    );
    assert_eq!(count(&nary), count(&seq));
}

/// fuzz_keep seed 166's features 0, 2, 5 and 14: a pocket whose floor lies 2.3e-3 over the block's
/// bottom, a boss, and a block kept whose side crosses the pocket. On that side the pocket's section
/// is a hole 2.3e-3 over the region below the part, and the point that finds its host was put a
/// thousandth of its 17 long floor off it (1.7e-2): in the region below. The section of the part
/// went without its hole, the floor's new edge had one face, and the pinch check refused
/// ZeroThickness where the result is a plain corner. (The probe's step is now held under half the
/// way to the outer cycles' pieces near it, 587d0859, which also fixed zerothick-groove.fg.)
#[test]
fn a_hole_a_hair_over_a_boundary_finds_its_host() {
    let part = suite::fuzz_keep(166, 80);
    let part = forge_model::Part {
        features: [0, 2, 5, 14].iter().map(|&i| part.features[i].clone()).collect(),
    };
    let seq = part.regenerate().expect("sequential regen");
    assert!(
        forge_topo::check(&seq).is_empty(),
        "{:?}",
        forge_topo::check(&seq)
    );
    let nary = part.regenerate_nary(1).expect("n-ary regen");
    assert!(forge_topo::check(&nary).is_empty());
    let mass = |b| forge_topo::mass::mass(b).expect("mass properties");
    let (a, b) = (mass(&seq), mass(&nary));
    assert!((a.volume - b.volume).abs() <= a.err[0] + b.err[0] + 1e-12 * a.volume);
}

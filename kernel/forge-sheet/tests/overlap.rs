//! Both checks on every build: a flat overlap refuses the flat (and its DXF) and warns on the
//! folded build; a folded clash refuses the folded and warns on the flat. Flanges facing across
//! notches overlap only when flat; a near miss does not; a fuzz against the exact overlap area;
//! and the checks' cost on an L, a U and a box.
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendRule, FeatureId, Gauge, Map, Sheet, SheetError, SheetWarning, Strip, WallId, build_with, setback_of,
};

fn poly(pts: &[(f64, f64)]) -> ProfileLoop {
    let n = pts.len();
    ProfileLoop::new(
        (0..n)
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                (
                    Segment::Line {
                        a: Point2::new(a.0, a.1),
                        b: Point2::new(b.0, b.1),
                    },
                    i as u64,
                )
            })
            .collect(),
    )
}

/// A 100 x 60 base with `notches` slots from its top edge, each `gap` wide and 30 deep; in each,
/// a flange up off both sides, `h1` and `h2` high (from the mold line) at `angle`.
#[allow(clippy::too_many_arguments)]
fn notched(t: f64, r: f64, k: f64, gap: f64, h1: f64, h2: f64, angle: f64, notches: usize) -> Sheet {
    let (l, w, d) = (100.0, 60.0, 30.0);
    let mut pts = vec![(0.0, 0.0), (l, 0.0), (l, w)];
    // notches between x = 20 and 80, from right to left along the top
    let centres: Vec<f64> = (0..notches).map(|i| 70.0 - 40.0 * i as f64).collect();
    for &c in &centres {
        pts.extend([
            (c + gap / 2.0, w),
            (c + gap / 2.0, w - d),
            (c - gap / 2.0, w - d),
            (c - gap / 2.0, w),
        ]);
    }
    pts.push((0.0, w));
    let g = Gauge {
        thickness: t,
        radius: r,
        rule: BendRule::K(k),
    };
    let (mut s, base) = Sheet::face(FeatureId(4), g, Frame::WORLD, &[poly(&pts)]).unwrap();
    for i in 0..notches {
        // the notch's right side runs down (segment 3 + 4i), its left side up (5 + 4i)
        let (right, left) = (3 + 4 * i as u64, 5 + 4 * i as u64);
        s.flange(base, right, angle, h2).unwrap();
        s.flange(base, left, angle, h1).unwrap();
    }
    s
}

/// The exact overlap of the two flats in one notch: each spans the notch's depth, from its edge
/// to s - BA - (h - s) past it.
fn reference(t: f64, r: f64, k: f64, gap: f64, h1: f64, h2: f64, angle: f64) -> f64 {
    let g = Gauge {
        thickness: t,
        radius: r,
        rule: BendRule::K(k),
    };
    let s = setback_of(&g, r, angle);
    let ba = angle.abs() * (r + k * t);
    let reach = |h: f64| -s + ba + (h - s);
    (reach(h1) + reach(h2) - gap).max(0.0) * 30.0
}

/// Whether a line of one side's flat (tangent lines, tip) comes within the region boolean's
/// near-coincidence limit of a line of the other's.
fn near_coincident(t: f64, r: f64, k: f64, gap: f64, h1: f64, h2: f64, angle: f64) -> bool {
    let g = Gauge {
        thickness: t,
        radius: r,
        rule: BendRule::K(k),
    };
    let s = setback_of(&g, r, angle);
    let ba = angle.abs() * (r + k * t);
    // measured from the left side's edge, into the notch
    let left = [-s, -s + ba, -s + ba + (h1 - s)];
    let right = [gap + s, gap + s - ba, gap + s - ba - (h2 - s)];
    left.iter()
        .any(|a| right.iter().any(|b| (a - b).abs() < 2e-5 * 1.5))
}

#[test]
fn facing_flanges_overlap_only_when_flat() {
    // two notches, four flanges: folded they stand apart, flat they reach into each other
    let s = notched(2.0, 2.0, 0.4, 20.0, 15.0, 15.0, dmath::PI / 2.0, 2);
    let fold = build_with(&s, Map::Folded).expect("folded builds");
    let want = reference(2.0, 2.0, 0.4, 20.0, 15.0, 15.0, dmath::PI / 2.0);
    match fold.warnings.as_slice() {
        [SheetWarning::FlatOverlap { a, b, total, .. }] => {
            assert!(matches!((a, b), (Strip::Wall(_), Strip::Wall(_))), "{a:?} {b:?}");
            // two notches: twice the one
            assert!(
                (total - 2.0 * want).abs() < 1e-9,
                "{total} against {}",
                2.0 * want
            );
        }
        w => panic!("{w:?}"),
    }
    assert!(matches!(
        build_with(&s, Map::Flat),
        Err(SheetError::FlatOverlap { total, .. }) if (total - 2.0 * want).abs() < 1e-9
    ));
    // a near miss: just short of reaching
    let g = Gauge {
        thickness: 2.0,
        radius: 2.0,
        rule: BendRule::K(0.4),
    };
    let s0 = setback_of(&g, 2.0, dmath::PI / 2.0);
    let ba = dmath::PI / 2.0 * (2.0 + 0.8);
    let h = (20.0 / 2.0 + 2.0 * s0 - ba) - 1e-3;
    let s = notched(2.0, 2.0, 0.4, 20.0, h, h, dmath::PI / 2.0, 2);
    let fold = build_with(&s, Map::Folded).unwrap();
    assert!(fold.warnings.is_empty(), "{:?}", fold.warnings);
    assert!(build_with(&s, Map::Flat).is_ok());
}

#[test]
fn a_folded_clash_warns_on_the_flat() {
    // flanges off both sides of a plain base folded back over it until they cross: flat, they
    // lie apart on either side
    let g = Gauge {
        thickness: 2.0,
        radius: 2.0,
        rule: BendRule::K(0.4),
    };
    let rect = poly(&[(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0)]);
    let (mut s, base) = Sheet::face(FeatureId(1), g, Frame::WORLD, &[rect]).unwrap();
    for e in [1, 3] {
        s.flange(base, e, 150f64.to_radians(), 30.0).unwrap();
    }
    assert!(matches!(
        build_with(&s, Map::Folded),
        Err(SheetError::FoldedClash { .. })
    ));
    let flat = build_with(&s, Map::Flat).expect("flat builds");
    assert!(
        matches!(flat.warnings.as_slice(), [SheetWarning::FoldedClash { .. }]),
        "{:?}",
        flat.warnings
    );
    assert!(flat.pattern.is_some());
}

#[test]
fn fuzz_against_the_exact_overlap() {
    let mut rng = Rng::new(0x0f1a7);
    let (mut overlapped, mut clear, mut clashed, mut undecided) = (0, 0, 0, 0);
    for _ in 0..300 {
        let t = rng.range(0.8, 3.0);
        let r = rng.range(0.5, 3.0);
        let k = rng.range(0.25, 0.5);
        let gap = rng.range(12.0, 30.0);
        let angle = rng.range(40.0, 130.0).to_radians();
        let g = Gauge {
            thickness: t,
            radius: r,
            rule: BendRule::K(k),
        };
        let s0 = setback_of(&g, r, angle);
        let (h1, h2) = (rng.range(s0 + t, 25.0), rng.range(s0 + t, 25.0));
        let s = notched(t, r, k, gap, h1, h2, angle, 1);
        let want = reference(t, r, k, gap, h1, h2, angle);
        let flat = build_with(&s, Map::Flat);
        let fold = build_with(&s, Map::Folded);
        let flat_area = match &flat {
            Ok(b) => {
                assert!(
                    !b.warnings
                        .iter()
                        .any(|w| matches!(w, SheetWarning::FlatOverlap { .. }))
                );
                0.0
            }
            Err(SheetError::FlatOverlap { total, .. }) => *total,
            Err(SheetError::FlatOverlapUndecided { gap: g, .. }) => {
                // only where the two sides' lines come within the boolean's tolerance
                assert!(near_coincident(t, r, k, gap, h1, h2, angle), "undecided at {g}");
                undecided += 1;
                continue;
            }
            Err(e) => panic!("{e:?}"),
        };
        // the folded build carries the same overlap as a warning, or refuses for its own clash
        match &fold {
            Ok(b) => {
                let w: f64 = b
                    .warnings
                    .iter()
                    .filter_map(|w| match w {
                        SheetWarning::FlatOverlap { total, .. } => Some(*total),
                        _ => None,
                    })
                    .sum();
                assert!((w - flat_area).abs() < 1e-9);
            }
            Err(SheetError::FoldedClash { .. }) => {
                clashed += 1;
                if let Ok(b) = &flat {
                    assert!(
                        b.warnings
                            .iter()
                            .any(|w| matches!(w, SheetWarning::FoldedClash { .. }))
                    );
                }
            }
            Err(e) => panic!("{e:?}"),
        }
        if want > 1e-6 {
            assert!(
                (flat_area - want).abs() <= 1e-9 * want.max(1.0),
                "{flat_area} against {want}: {t} {r} {k} {gap} {h1} {h2} {}",
                angle.to_degrees()
            );
            overlapped += 1;
        } else {
            assert_eq!(
                flat_area,
                0.0,
                "{want} {t} {r} {k} {gap} {h1} {h2} {}",
                angle.to_degrees()
            );
            clear += 1;
        }
    }
    println!("overlapped {overlapped}, clear {clear}, clashed folded {clashed}, undecided {undecided}");
    assert!(overlapped > 30 && clear > 30);
}

#[test]
fn the_checks_cost_microseconds() {
    let g = Gauge {
        thickness: 2.0,
        radius: 2.0,
        rule: BendRule::K(0.4),
    };
    let rect = poly(&[(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0)]);
    let mk = |edges: &[u64]| {
        let (mut s, base) =
            Sheet::face(FeatureId(1), g.clone(), Frame::WORLD, std::slice::from_ref(&rect)).unwrap();
        for &e in edges {
            s.flange(base, e, dmath::PI / 2.0, 20.0).unwrap();
        }
        s
    };
    for (name, s) in [("L", mk(&[1])), ("U", mk(&[1, 3])), ("box", mk(&[0, 1, 2, 3]))] {
        for map in [Map::Folded, Map::Flat] {
            let n = 200;
            let t0 = std::time::Instant::now();
            for _ in 0..n {
                std::hint::black_box(build_with(&s, map).unwrap());
            }
            println!(
                "{name} {map:?}: {:.1} us a build",
                t0.elapsed().as_secs_f64() * 1e6 / n as f64
            );
        }
    }
    let _ = WallId(0);
}

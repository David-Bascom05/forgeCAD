//! Corner reliefs where two flanges off one wall meet: a closed box, its corners circular or
//! square (asked-for and automatic sizes), against closed forms folded and flat, rectangle by
//! rectangle on the flat; and random boxes with automatic corners building clean or refused typed.
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendRule, CornerKind, FeatureId, Gauge, JointId, Map, Placement, Sheet, SheetError, build_with,
};
use forge_topo::Body;

fn rect_loop(l: f64, w: f64) -> ProfileLoop {
    let p = Point2::new;
    let c = [p(0.0, 0.0), p(l, 0.0), p(l, w), p(0.0, w)];
    ProfileLoop::new(
        (0..4)
            .map(|i| {
                (
                    Segment::Line {
                        a: c[i],
                        b: c[(i + 1) % 4],
                    },
                    i as u64,
                )
            })
            .collect(),
    )
}

fn vol(b: &Body) -> (f64, f64) {
    let m = forge_topo::mass::mass(b).unwrap();
    (m.volume, m.err[0])
}

#[derive(Clone, Copy)]
struct Rect {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
}

/// The area of a rectangle inside a shape centred at c: a square of half side h, or a circle of
/// radius h (by the chord, integrated finely).
fn inside(r: Rect, c: Point2, h: f64, square: bool) -> f64 {
    if square {
        let dx = (r.x1.min(c.x + h) - r.x0.max(c.x - h)).max(0.0);
        let dy = (r.y1.min(c.y + h) - r.y0.max(c.y - h)).max(0.0);
        return dx * dy;
    }
    let (a, b) = (r.x0.max(c.x - h), r.x1.min(c.x + h));
    if b <= a {
        return 0.0;
    }
    let n = 200_000;
    let f = |x: f64| {
        let half = (h * h - (x - c.x) * (x - c.x)).max(0.0).sqrt();
        (r.y1.min(c.y + half) - r.y0.max(c.y - half)).max(0.0)
    };
    // Simpson
    let dx = (b - a) / n as f64;
    let mut s = f(a) + f(b);
    for k in 1..n {
        s += f(a + k as f64 * dx) * if k % 2 == 1 { 4.0 } else { 2.0 };
    }
    s * dx / 3.0
}

const T: f64 = 1.5;
const R: f64 = 1.0;
const K: f64 = 0.4;
const L: f64 = 60.0;
const W: f64 = 40.0;
const H: f64 = 20.0;

fn boxed(kind: Option<CornerKind>) -> Sheet {
    boxed_at(kind, dmath::PI / 2.0)
}

/// The box with its walls turned `th` (under 90 degrees they splay out and never meet).
fn boxed_at(kind: Option<CornerKind>, th: f64) -> Sheet {
    let g = Gauge {
        thickness: T,
        radius: R,
        rule: BendRule::K(K),
    };
    let (mut s, base) = Sheet::face(FeatureId(9), g, Frame::WORLD, &[rect_loop(L, W)]).unwrap();
    for e in 0..4 {
        s.flange(base, e, th, H).unwrap();
    }
    if let Some(k) = kind {
        s.corner_default = Some(k);
    }
    s
}

/// Folded and flat volumes of the box with corner shapes of size `h` taken out, on the flat.
fn closed(h: f64, square: bool, off: f64, th: f64) -> (f64, f64) {
    let g = Gauge {
        thickness: T,
        radius: R,
        rule: BendRule::K(K),
    };
    let s = forge_sheet::setback_of(&g, R, th);
    let ba = th * (R + K * T);
    let rn = ba / th;
    // centred `off` out of each corner along both edges' outward normals
    let corners = [
        (L, 0.0, 1.0, -1.0),
        (L, W, 1.0, 1.0),
        (0.0, W, -1.0, 1.0),
        (0.0, 0.0, -1.0, -1.0),
    ]
    .map(|(x, y, sx, sy)| Point2::new(x + sx * off, y + sy * off));
    let after = |r: Rect| {
        (r.x1 - r.x0) * (r.y1 - r.y0) - corners.iter().map(|c| inside(r, *c, h, square)).sum::<f64>()
    };
    let walls_flat = [
        // the base inside its bands
        Rect {
            x0: s,
            x1: L - s,
            y0: s,
            y1: W - s,
        },
        // each flange's wall, past its zone
        Rect {
            x0: 0.0,
            x1: L,
            y0: s - ba - (H - s),
            y1: s - ba,
        },
        Rect {
            x0: 0.0,
            x1: L,
            y0: W - s + ba,
            y1: W - s + ba + (H - s),
        },
        Rect {
            x0: L - s + ba,
            x1: L - s + ba + (H - s),
            y0: 0.0,
            y1: W,
        },
        Rect {
            x0: s - ba - (H - s),
            x1: s - ba,
            y0: 0.0,
            y1: W,
        },
    ];
    let zones = [
        Rect {
            x0: 0.0,
            x1: L,
            y0: s - ba,
            y1: s,
        },
        Rect {
            x0: 0.0,
            x1: L,
            y0: W - s,
            y1: W - s + ba,
        },
        Rect {
            x0: L - s,
            x1: L - s + ba,
            y0: 0.0,
            y1: W,
        },
        Rect {
            x0: s - ba,
            x1: s,
            y0: 0.0,
            y1: W,
        },
    ];
    let wa: f64 = walls_flat.iter().map(|r| after(*r)).sum();
    let za: f64 = zones.iter().map(|r| after(*r)).sum();
    (T * wa + T * za * (R + 0.5 * T) / rn, T * (wa + za))
}

#[test]
fn boxes_with_corner_reliefs_match_closed_forms() {
    for (kind, square) in [
        (
            CornerKind::Square {
                size: Some(4.0),
                placement: Placement::Tangent,
            },
            true,
        ),
        (
            CornerKind::Circular {
                size: Some(4.5),
                placement: Placement::Intersection,
            },
            false,
        ),
        (
            CornerKind::Square {
                size: None,
                placement: Placement::Intersection,
            },
            true,
        ),
        (
            CornerKind::Circular {
                size: None,
                placement: Placement::Tangent,
            },
            false,
        ),
    ] {
        // splayed 10 degrees out, the walls never meet: the reliefs alone, against the closed form
        let th = 80f64.to_radians();
        let s = boxed_at(Some(kind), th);
        let fold = build_with(&s, Map::Folded).unwrap_or_else(|e| panic!("{kind:?}: {e:?}"));
        let flat = build_with(&s, Map::Flat).unwrap();
        for b in [&fold.body, &flat.body] {
            let p = forge_topo::check::check(b);
            assert!(p.is_empty(), "{kind:?}: {p:?}");
        }
        assert_eq!(fold.corners.len(), 4, "{kind:?}");
        let h = fold.corners[0].size;
        assert!(
            fold.corners.iter().all(|c| (c.size - h).abs() < 1e-12),
            "{kind:?}: {:?}",
            fold.corners
        );
        let g = Gauge {
            thickness: T,
            radius: R,
            rule: BendRule::K(K),
        };
        let sb = forge_sheet::setback_of(&g, R, th);
        let off = match kind {
            CornerKind::Square { placement, .. } | CornerKind::Circular { placement, .. } => {
                match placement {
                    Placement::Tangent => -sb,
                    Placement::Intersection => 0.5 * th * (R + K * T) - sb,
                }
            }
            _ => 0.0,
        };
        let (vf, vl) = closed(h, square, off, th);
        let ((f, ef), (l, el)) = (vol(&fold.body), vol(&flat.body));
        let tol = if square { 1e-9 } else { 1e-6 };
        assert!((f - vf).abs() <= ef + tol, "{kind:?}: folded {f} against {vf}");
        assert!((l - vl).abs() <= el + tol, "{kind:?}: flat {l} against {vl}");
    }
}

/// A box on a w x d rectangle turned `rot` about its centre, its outline starting at corner
/// `from`, flanges added in `order` (by outline edge): which edges' walls are trimmed at each
/// corner (by edge, sorted), the folded and flat volumes.
fn trims_of(w: f64, d: f64, rot: f64, from: usize, order: &[usize]) -> (Vec<usize>, f64, f64) {
    let (sn, cs) = dmath::sin_cos(rot);
    let c = [(0.0, 0.0), (w, 0.0), (w, d), (0.0, d)];
    let p = |k: usize| {
        let (x, y) = c[(k + from) % 4];
        let (x, y) = (x - 0.5 * w, y - 0.5 * d);
        Point2::new(cs * x - sn * y, sn * x + cs * y)
    };
    let lp = ProfileLoop::new(
        (0..4)
            .map(|i| (Segment::Line { a: p(i), b: p(i + 1) }, i as u64))
            .collect(),
    );
    let g = Gauge {
        thickness: 3.02,
        radius: 1.6,
        rule: BendRule::K(0.38),
    };
    let (mut s, base) = Sheet::face(FeatureId(9), g, Frame::WORLD, &[lp]).unwrap();
    for &e in order {
        s.flange_at(
            base,
            e as u64,
            dmath::PI / 2.0,
            30.0,
            1.6,
            forge_sheet::Position::Inside,
        )
        .unwrap();
    }
    let fold = build_with(&s, Map::Folded).unwrap();
    let flat = build_with(&s, Map::Flat).unwrap();
    assert_eq!(fold.corners, flat.corners);
    // a trim's first joint is the trimmed wall's; its edge, counted from the box's own first corner
    let mut cut: Vec<usize> = fold
        .corners
        .iter()
        .filter(|c| matches!(c.kind, CornerKind::Open { .. }))
        .map(|c| (s.joint(c.joints[0]).unwrap().on_a as usize + from) % 4)
        .collect();
    cut.sort();
    (cut, vol(&fold.body).0, vol(&flat.body).0)
}

#[test]
fn corner_trims_go_by_the_part_not_the_flange_order() {
    // a rectangle: both short walls are trimmed at both ends, mirror symmetric, whatever the
    // order the flanges came in, however the box is turned, wherever its outline starts
    let orders: [&[usize]; 3] = [&[0, 1, 2, 3], &[3, 2, 1, 0], &[2, 0, 3, 1]];
    let want = trims_of(200.0, 150.0, 0.0, 0, orders[0]);
    assert_eq!(want.0, vec![1, 1, 3, 3]);
    for (rot, from) in [(0.0, 0), (0.0, 1), (0.6, 2), (-2.1, 3)] {
        for order in orders {
            let got = trims_of(200.0, 150.0, rot, from, order);
            assert_eq!(got.0, want.0, "{rot} {from} {order:?}");
            assert!(
                (got.1 - want.1).abs() < 1e-6 && (got.2 - want.2).abs() < 1e-6,
                "{got:?} {want:?}"
            );
        }
    }
    // a square: each wall trimmed once, at the end where its edge starts (counter-clockwise):
    // four identical trims, the part alike at a quarter turn, again whatever the order
    let want = trims_of(150.0, 150.0, 0.0, 0, orders[0]);
    assert_eq!(want.0, vec![0, 1, 2, 3]);
    for (rot, from) in [(0.0, 1), (0.6, 2), (-2.1, 3)] {
        for order in orders {
            let got = trims_of(150.0, 150.0, rot, from, order);
            assert_eq!(got.0, want.0, "{rot} {from} {order:?}");
            assert!(
                (got.1 - want.1).abs() < 1e-6 && (got.2 - want.2).abs() < 1e-6,
                "{got:?} {want:?}"
            );
        }
    }
}

#[test]
fn upright_walls_meeting_at_corners_are_trimmed_a_gap_apart() {
    // upright on a square corner, two walls would meet along the corner's upright: the later
    // is trimmed back t and the rule's gap from the corner, reported as an automatic open trim
    for kind in [
        None,
        Some(CornerKind::Square {
            size: Some(4.0),
            placement: Placement::Tangent,
        }),
    ] {
        let s = boxed(kind);
        let gap = s.rule.corner.gap.at(T, R);
        for map in [Map::Folded, Map::Flat] {
            let b = build_with(&s, map).unwrap_or_else(|e| panic!("{kind:?} {map:?}: {e:?}"));
            let p = forge_topo::check::check(&b.body);
            assert!(p.is_empty(), "{p:?}");
            assert!(b.warnings.is_empty(), "{:?}", b.warnings);
            let trims: Vec<_> = b
                .corners
                .iter()
                .filter(|c| matches!(c.kind, CornerKind::Open { .. }))
                .collect();
            assert_eq!(trims.len(), 4, "{:?}", b.corners);
            assert!(
                trims
                    .iter()
                    .all(|c| c.automatic && (c.size - (T + gap)).abs() < 1e-12)
            );
        }
        // folded, the trimmed wall's end stands the gap clear of its neighbour's face
        let fold = build_with(&s, Map::Folded).unwrap();
        let near = fold
            .body
            .vertex_ids()
            .map(|v| fold.body.vertex(v).point)
            .filter(|p| p.z > H - 1e-9 && p.x > L - 1e-9)
            .map(|p| p.y)
            .filter(|y| *y > 1e-9)
            .fold(f64::INFINITY, f64::min);
        assert!((near - (T + gap)).abs() < 1e-9, "{near}");
    }
    // an explicit open corner keeps its own gap
    let s = boxed(Some(CornerKind::Open { gap: 1.0 }));
    let b = build_with(&s, Map::Folded).unwrap();
    let trims = b
        .corners
        .iter()
        .filter(|c| !c.automatic && (c.size - (T + 1.0)).abs() < 1e-12)
        .count();
    assert_eq!(trims, 4, "{:?}", b.corners);
}

#[test]
fn closed_corners_are_typed_refusals() {
    let s = boxed(Some(CornerKind::Closed));
    // (the rule's default for the rest)
    assert!(matches!(
        build_with(&s, Map::Folded),
        Err(SheetError::CornerUnbuilt { .. })
    ));
    let mut s = boxed(None);
    s.set_corner(
        JointId(0),
        JointId(1),
        CornerKind::Circular {
            size: Some(0.0),
            placement: Placement::Intersection,
        },
    );
    assert!(matches!(
        build_with(&s, Map::Folded),
        Err(SheetError::CornerUnbuilt { size, .. }) if size == 0.0
    ));
}

#[test]
fn random_boxes_with_automatic_corners() {
    let mut rng = Rng::new(0xc0e7);
    let (mut built, mut refused) = (0, 0);
    for _ in 0..200 {
        let t = rng.range(0.8, 3.0);
        let r = rng.range(0.5, 4.0);
        let g = Gauge {
            thickness: t,
            radius: r,
            rule: BendRule::K(rng.range(0.25, 0.5)),
        };
        let (l, w) = (rng.range(40.0, 90.0), rng.range(30.0, 70.0));
        let (mut s, base) = Sheet::face(FeatureId(9), g, Frame::WORLD, &[rect_loop(l, w)]).unwrap();
        let up = rng.range(0.0, 1.0) < 0.8;
        for e in 0..4 {
            if rng.range(0.0, 1.0) < 0.25 {
                continue;
            }
            let a = rng.range(45.0, 100.0).to_radians() * if up { 1.0 } else { -1.0 };
            s.flange(base, e, a, rng.range(10.0, 25.0)).unwrap();
        }
        match build_with(&s, Map::Folded) {
            Ok(b) => {
                let p = forge_topo::check::check(&b.body);
                assert!(p.is_empty(), "{p:?}");
                assert!(build_with(&s, Map::Flat).is_ok());
                built += 1;
            }
            Err(e) => {
                // only typed refusals: a clash of tall flanges leaning in, or (1 in 200) corner
                // reliefs parting a short flange from the sheet
                assert!(
                    matches!(e, SheetError::FoldedClash { .. } | SheetError::SheetSplit { .. }),
                    "{e:?}"
                );
                refused += 1;
            }
        }
    }
    println!("built {built}, refused {refused} (clashes and splits)");
    assert!(built > 150, "built {built}, refused {refused}");
}

#[test]
fn down_flanges_at_a_corner_with_an_open_corner() {
    // two flanges turned down off adjacent edges stand inside them (no edge move going down):
    // their walls share a t x t column, a real clash; an open corner trims the later wall back
    let g = Gauge {
        thickness: T,
        radius: R,
        rule: BendRule::K(K),
    };
    let th = dmath::PI / 2.0;
    let (mut s, base) = Sheet::face(FeatureId(9), g, Frame::WORLD, &[rect_loop(40.0, 100.0)]).unwrap();
    s.flange(base, 1, -th, 35.0).unwrap();
    s.flange(base, 0, -th, 25.0).unwrap();
    // left to the builder, the walls would meet: trimmed apart by the rule's gap
    let b = build_with(&s, Map::Folded).unwrap();
    assert!(
        b.corners
            .iter()
            .any(|c| matches!(c.kind, CornerKind::Open { .. }) && c.automatic)
    );
    s.corner_default = Some(CornerKind::Open { gap: 0.4 });
    for map in [Map::Folded, Map::Flat] {
        let b = build_with(&s, map).unwrap_or_else(|e| panic!("{map:?}: {e:?}"));
        let p = forge_topo::check::check(&b.body);
        assert!(p.is_empty(), "{p:?}");
        assert_eq!(b.corners.len(), 2, "the relief and the trim");
    }
}

#[test]
fn trim_to_bend_parts_the_zones_by_a_kerf() {
    // cut exactly to both tangent lines, the zones met at the point the lines cross (a flat
    // touching itself, and a slow clash hunt there): a kerf past them parts them. Upright or
    // splayed, both maps build clean and quickly
    for th in [90f64, 80.0] {
        let s = boxed_at(Some(CornerKind::TrimToBend), th.to_radians());
        for map in [Map::Folded, Map::Flat] {
            let t0 = std::time::Instant::now();
            let b = build_with(&s, map).unwrap_or_else(|e| panic!("{th} {map:?}: {e:?}"));
            let took = t0.elapsed().as_secs_f64();
            assert!(b.warnings.is_empty(), "{th} {map:?}: {:?}", b.warnings);
            let p = forge_topo::check::check(&b.body);
            assert!(p.is_empty(), "{p:?}");
            assert!(took < 0.05, "{th} {map:?}: {took} s");
        }
    }
}

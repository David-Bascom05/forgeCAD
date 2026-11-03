//! Corner reliefs where two flanges off one wall meet: a closed box, its corners circular or
//! square (asked-for and automatic sizes), against closed forms folded and flat, rectangle by
//! rectangle on the flat; and random boxes with automatic corners building clean or refused typed.
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{BendRule, CornerKind, FeatureId, Gauge, JointId, Map, Sheet, SheetError, build_with};
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
    let g = Gauge {
        thickness: T,
        radius: R,
        rule: BendRule::K(K),
    };
    let (mut s, base) = Sheet::face(FeatureId(9), g, Frame::WORLD, &[rect_loop(L, W)]).unwrap();
    for e in 0..4 {
        s.flange(base, e, dmath::PI / 2.0, H).unwrap();
    }
    if let Some(k) = kind {
        s.corner_default = k;
    }
    s
}

/// Folded and flat volumes of the box with corner shapes of size `h` taken out, on the flat.
fn closed(h: f64, square: bool) -> (f64, f64) {
    let th = dmath::PI / 2.0;
    let s = R;
    let ba = th * (R + K * T);
    let rn = ba / th;
    let corners = [(L, 0.0), (L, W), (0.0, W), (0.0, 0.0)].map(|(x, y)| Point2::new(x, y));
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
        (CornerKind::Square { size: Some(4.0) }, true),
        (CornerKind::Circular { size: Some(4.5) }, false),
        (CornerKind::Square { size: None }, true),
        (CornerKind::Circular { size: None }, false),
    ] {
        let s = boxed(Some(kind));
        let fold = build_with(&s, Map::Folded).unwrap_or_else(|e| panic!("{kind:?}: {e:?}"));
        let flat = build_with(&s, Map::Flat).unwrap();
        for b in [&fold.body, &flat.body] {
            let p = forge_topo::check::check(b);
            assert!(p.is_empty(), "{kind:?}: {p:?}");
        }
        assert_eq!(fold.corners.len(), 4, "{kind:?}");
        let h = fold.corners[0].size;
        assert!(fold.corners.iter().all(|c| c.size == h));
        let (vf, vl) = closed(h, square);
        let ((f, ef), (l, el)) = (vol(&fold.body), vol(&flat.body));
        let tol = if square { 1e-9 } else { 1e-6 };
        assert!((f - vf).abs() <= ef + tol, "{kind:?}: folded {f} against {vf}");
        assert!((l - vl).abs() <= el + tol, "{kind:?}: flat {l} against {vl}");
    }
}

#[test]
fn closed_corners_are_typed_refusals() {
    let s = boxed(Some(CornerKind::Closed));
    assert!(matches!(
        build_with(&s, Map::Folded),
        Err(SheetError::CornerUnbuilt { .. })
    ));
    let mut s = boxed(None);
    s.set_corner(JointId(0), JointId(1), CornerKind::Circular { size: Some(0.0) });
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
                // only a typed clash of tall flanges leaning in may refuse
                assert!(matches!(e, SheetError::FoldedClash { .. }), "{e:?}");
                refused += 1;
            }
        }
    }
    println!("built {built}, refused {refused}");
    assert!(built > 150, "built {built}, refused {refused}");
}

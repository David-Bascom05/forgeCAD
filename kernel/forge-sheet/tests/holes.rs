//! The shop's hole and slot rules and pressed-in hardware, each measured exactly against where
//! the cut was put: distance to a bend's tangent line and to the edge, the way out, width, round
//! hole size; hardware thickness range, centre to edge or bend, sheet hardness, placeholder rows.
use forge_math::{Frame, Point2, Vec2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendRule, CutId, FeatureId, Gauge, Map, Scaled, Sheet, SheetWarning, ShopProfile, Stock, Violation,
    WallId, build_with, check, checks, placeholder_catalogue,
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

fn circle(x: f64, y: f64, r: f64) -> ProfileLoop {
    ProfileLoop::new(vec![(
        Segment::Circle {
            center: Point2::new(x, y),
            radius: r,
            ccw: true,
        },
        0,
    )])
}

const T: f64 = 2.0;
const R: f64 = 2.0;

/// A 60 x 40 base with a flange up 90 off its right edge (x = 60).
fn l() -> Sheet {
    let g = Gauge {
        thickness: T,
        radius: R,
        rule: BendRule::K(0.4),
    };
    let (mut s, base) = Sheet::face(
        FeatureId(2),
        g,
        Frame::WORLD,
        &[poly(&[(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)])],
    )
    .unwrap();
    s.flange(base, 1, dmath::PI / 2.0, 30.0).unwrap();
    s
}

fn per_t(f: f64, per_r: f64) -> Scaled {
    Scaled {
        per_t: f,
        per_r,
        plus: 0.0,
    }
}

/// Holes 2.5 t + R from a bend, 2 t from the edge, round holes t across, slots t wide.
fn shop(hardness: Option<f64>) -> ShopProfile {
    ShopProfile {
        stock: vec![Stock {
            thickness: T,
            rule: BendRule::K(0.4),
            radii: vec![R],
            min_flange: Scaled::NONE,
            hole_to_bend: per_t(2.5, 1.0),
            hole_to_edge: per_t(2.0, 0.0),
            min_slot: per_t(1.0, 0.0),
            min_hole: per_t(1.0, 0.0),
            relief: forge_sheet::ReliefRule::default(),
            hardness,
        }],
        hardware: placeholder_catalogue(),
        kerf: 0.2,
    }
}

#[test]
fn holes_near_a_bend_and_an_edge() {
    let mut s = l();
    // the base's tangent line is at x = 60 - R = 58; the rule wants 2.5 t + R = 7 from it
    s.cut(WallId(0), circle(50.0, 20.0, 3.0)).unwrap(); // 5 from the tangent line
    s.cut(WallId(0), circle(20.0, 5.0, 2.0)).unwrap(); // 3 from the edge y = 0 (rule 4)
    s.cut(WallId(0), circle(20.0, 25.0, 0.8)).unwrap(); // 1.6 across (rule 2)
    s.cut(
        WallId(0),
        poly(&[(30.0, 15.0), (40.0, 15.0), (40.0, 16.5), (30.0, 16.5)]),
    )
    .unwrap(); // a slot 1.5 wide
    s.cut(WallId(0), circle(0.0, 20.0, 3.0)).unwrap(); // a notch in the edge: not a hole
    let v = check(&s, &shop(Some(70.0)));
    let near_bend: Vec<_> = v
        .iter()
        .filter_map(|x| match x {
            Violation::HoleNearBend {
                distance,
                limit,
                away,
                ..
            } => Some((*distance, *limit, *away)),
            _ => None,
        })
        .collect();
    assert_eq!(near_bend.len(), 1, "{v:?}");
    let (d, lim, away) = near_bend[0];
    assert!((d - 5.0).abs() < 1e-9 && lim == 7.0, "{d} {lim}");
    assert!((away - Vec2::new(-1.0, 0.0)).length() < 1e-12, "{away:?}");
    let near_edge: Vec<_> = v
        .iter()
        .filter_map(|x| match x {
            Violation::HoleNearEdge {
                distance,
                limit,
                away,
                ..
            } => Some((*distance, *limit, *away)),
            _ => None,
        })
        .collect();
    assert_eq!(near_edge.len(), 1, "{v:?}");
    assert!((near_edge[0].0 - 3.0).abs() < 1e-9 && near_edge[0].1 == 4.0);
    assert!(
        (near_edge[0].2 - Vec2::new(0.0, 1.0)).length() < 1e-9,
        "{:?}",
        near_edge[0].2
    );
    assert!(
        v.iter()
            .any(|x| matches!(x, Violation::HoleTooSmall { diameter, limit, .. }
        if (diameter - 1.6).abs() < 1e-12 && *limit == 2.0))
    );
    assert!(
        v.iter()
            .any(|x| matches!(x, Violation::SlotTooNarrow { width, limit, .. }
        if (width - 1.5).abs() < 1e-6 && *limit == 2.0)),
        "{v:?}"
    );
    // nothing about the notch
    assert!(!v.iter().any(|x| matches!(
        x,
        Violation::HoleNearEdge {
            trim: forge_sheet::TrimRef::Cut { cut: CutId(4), .. },
            ..
        } | Violation::HoleTooSmall {
            trim: forge_sheet::TrimRef::Cut { cut: CutId(4), .. },
            ..
        }
    )));
}

#[test]
fn holes_in_a_flange_and_in_a_bend() {
    let mut s = l();
    // in the flange's coordinates its tangent line is at x = R; 4 past it
    s.cut(WallId(1), circle(R + 4.0 + 1.5, 20.0, 1.5)).unwrap();
    // wholly in the bend zone, developed from the base
    s.cut(WallId(0), circle(59.0, 30.0, 0.5)).unwrap();
    let v = check(&s, &shop(Some(70.0)));
    let d: Vec<f64> = v
        .iter()
        .filter_map(|x| match x {
            Violation::HoleNearBend { distance, .. } => Some(*distance),
            _ => None,
        })
        .collect();
    assert_eq!(d.len(), 2, "{v:?}");
    assert!(d.iter().any(|x| (x - 4.0).abs() < 1e-9), "{d:?}");
    assert!(d.contains(&0.0), "{d:?}");
}

#[test]
fn hardware_against_its_row() {
    let nut = placeholder_catalogue()
        .into_iter()
        .find(|h| h.kind == forge_sheet::HardwareKind::Nut && h.size == "M3")
        .unwrap();
    // 4 from the edge y = 0 (the row wants 4.8), well clear of the bend
    let mut s = l();
    let cut = s.mount(WallId(0), Point2::new(20.0, 4.0), nut.clone()).unwrap();
    let fold = build_with(&s, Map::Folded).unwrap();
    let plain = build_with(&l(), Map::Folded).unwrap();
    let vol = |b: &forge_topo::Body| forge_topo::mass::mass(b).unwrap().volume;
    let hole = dmath::PI * 0.25 * nut.hole * nut.hole * T;
    assert!(((vol(&plain.body) - vol(&fold.body)) - hole).abs() < 1e-7);
    let v = checks::mounts(&s, shop(Some(70.0)).stock.first());
    assert!(
        matches!(v.as_slice(), [Violation::HardwareNearEdge { cut: c, distance, limit, away, .. }]
            if *c == cut && (distance - 4.0).abs() < 1e-12 && *limit == 4.8 && (*away - Vec2::new(0.0, 1.0)).length() < 1e-12),
        "{v:?}"
    );
    // too hard, hardness not stated, thickness out of range
    let v = checks::mounts(&s, shop(Some(90.0)).stock.first());
    assert!(v.iter().any(|x| matches!(x, Violation::SheetTooHard { hardness, limit, .. } if *hardness == 90.0 && *limit == 80.0)));
    let v = checks::mounts(&s, None);
    assert!(v.iter().any(|x| matches!(x, Violation::HardnessUnknown { .. })));
    let mut thin = s.clone();
    thin.gauge.thickness = 0.6;
    let v = checks::mounts(&thin, shop(Some(70.0)).stock.first());
    assert!(v.iter().any(|x| matches!(x, Violation::HardwareThickness { thickness, min, .. } if *thickness == 0.6 && *min == 1.0)));
    // placeholder rows are said to be
    assert!(
        matches!(checks::warnings(&s).as_slice(), [SheetWarning::UnverifiedHardware { cut: c, .. }] if *c == cut)
    );
    // a mounting hole is the hardware's business, not the shop's hole rules
    let v = check(&s, &shop(Some(70.0)));
    assert!(
        !v.iter().any(|x| matches!(x, Violation::HoleNearEdge { .. })),
        "{v:?}"
    );
}

//! The rules a vendor profile adds to the shop, each tried on a part just inside and just outside
//! its limit, the distance measured exactly on the flat: bend to edge (from the tangent line or
//! the bend's centre), hole to bend from the centre, webs between holes and across a neck of the
//! outline, operations offered, a stock's own angle range and bend length, the bent blank's size,
//! and what a partial profile leaves unchecked.
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendRef, BendRule, FeatureId, Gauge, Map, Operation, Process, Scaled, Sheet, ShopProfile, Stock,
    Violation, WallId, build_with, check,
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
/// Just past a limit, either way.
const EPS: f64 = 1e-4;

fn gauge() -> Gauge {
    Gauge {
        thickness: T,
        radius: R,
        rule: BendRule::K(0.4),
    }
}

fn plate() -> Sheet {
    Sheet::face(
        FeatureId(2),
        gauge(),
        Frame::WORLD,
        &[poly(&[(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)])],
    )
    .unwrap()
    .0
}

/// A 60 x 40 base with a flange up 90 off its right edge (x = 60), 30 tall.
fn l() -> Sheet {
    let mut s = plate();
    s.flange(WallId(0), 1, dmath::PI / 2.0, 30.0).unwrap();
    s
}

fn stock() -> Stock {
    Stock {
        thickness: T,
        rule: BendRule::K(0.4),
        radii: vec![R],
        min_flange: Scaled::NONE,
        flange_outside: false,
        hole_to_bend: Scaled::NONE,
        hole_to_edge: Scaled::NONE,
        min_slot: Scaled::NONE,
        min_hole: Scaled::NONE,
        bend_to_edge: Scaled::NONE,
        min_web: Scaled::NONE,
        angles: None,
        bend_ref: BendRef::Tangent,
        bend_length_max: None,
        not: vec![],
        relief: forge_sheet::ReliefRule::default(),
        hardness: None,
    }
}

fn shop(s: Stock) -> ShopProfile {
    ShopProfile {
        stock: vec![s],
        kerf: 0.2,
        ..Default::default()
    }
}

fn len(v: f64) -> Scaled {
    Scaled {
        per_t: 0.0,
        per_r: 0.0,
        plus: v,
    }
}

/// The base's side of the bend zone on the flat (its tangent line's x), and the zone's width.
fn zone(s: &Sheet) -> (f64, f64) {
    let b = build_with(s, Map::Flat).unwrap();
    let z = &b.layout.bends[0];
    let x0 = z.developed.origin.x;
    let x1 = z.developed.apply(Point2::new(z.allowance, 0.0)).x;
    (x0.min(x1), z.allowance)
}

fn kinds(v: &[Violation]) -> Vec<&'static str> {
    v.iter()
        .map(|x| match x {
            Violation::BendNearEdge { .. } => "BendNearEdge",
            Violation::WebTooNarrow { .. } => "WebTooNarrow",
            Violation::OperationNotOffered { .. } => "OperationNotOffered",
            Violation::BendAngleOutOfRange { .. } => "BendAngleOutOfRange",
            Violation::BendTooLong { .. } => "BendTooLong",
            Violation::FlatTooLarge { .. } => "FlatTooLarge",
            Violation::PartTooSmall { .. } => "PartTooSmall",
            Violation::HoleNearBend { .. } => "HoleNearBend",
            Violation::RadiusNotTooled { .. } => "RadiusNotTooled",
            Violation::HardnessUnknown { .. } => "HardnessUnknown",
            _ => "other",
        })
        .collect()
}

/// An L with a square notch up from the bottom edge, its side at x = 52, in front of the bend.
fn notched() -> Sheet {
    let mut s = l();
    s.cut(
        WallId(0),
        poly(&[(45.0, -1.0), (52.0, -1.0), (52.0, 10.0), (45.0, 10.0)]),
    )
    .unwrap();
    s
}

#[test]
fn bend_to_edge_from_the_tangent_line() {
    let s = notched();
    let (tangent, _) = zone(&s);
    let d = tangent - 52.0;
    assert!(d > 1.0, "the notch is in front of the zone: {d}");
    let at = |limit: f64| {
        let st = Stock {
            bend_to_edge: len(limit),
            ..stock()
        };
        check(&s, &shop(st))
    };
    let v = at(d + EPS);
    assert_eq!(kinds(&v), ["BendNearEdge"], "{v:?}");
    let Violation::BendNearEdge {
        distance,
        limit,
        at: p,
        ..
    } = v[0]
    else {
        unreachable!()
    };
    assert!((distance - d).abs() < 1e-9, "{distance} {d}");
    assert_eq!(limit, d + EPS);
    // the nearest point is on the notch's side
    assert!(
        (p.x - 52.0).abs() < 1e-9 && p.y > 0.0 && p.y <= 10.0 + 1e-9,
        "{p:?}"
    );
    assert!(at(d - EPS).is_empty(), "{:?}", at(d - EPS));
    // the base's far edge (x = 0) and the flange's tip run the bend's whole length: the minimum
    // flange's to check, not this
    assert!(
        check(
            &l(),
            &shop(Stock {
                bend_to_edge: len(1e3),
                ..stock()
            })
        )
        .is_empty()
    );
}

#[test]
fn bend_to_edge_and_hole_to_bend_from_the_centre() {
    let s = notched();
    let (tangent, ba) = zone(&s);
    let d = tangent - 52.0 + 0.5 * ba;
    let at = |limit: f64| {
        let st = Stock {
            bend_to_edge: len(limit),
            bend_ref: BendRef::Centre,
            ..stock()
        };
        check(&s, &shop(st))
    };
    assert_eq!(kinds(&at(d + EPS)), ["BendNearEdge"]);
    assert!(at(d - EPS).is_empty());
    // a hole: from its edge to the zone's centre line
    let mut h = l();
    h.cut(WallId(0), circle(40.0, 20.0, 3.0)).unwrap();
    let d = tangent - 43.0 + 0.5 * ba;
    let at = |limit: f64| {
        let st = Stock {
            hole_to_bend: len(limit),
            bend_ref: BendRef::Centre,
            ..stock()
        };
        check(&h, &shop(st))
    };
    let v = at(d + EPS);
    assert_eq!(kinds(&v), ["HoleNearBend"], "{v:?}");
    let Violation::HoleNearBend { distance, .. } = v[0] else {
        unreachable!()
    };
    assert!((distance - d).abs() < 1e-9, "{distance} {d}");
    assert!(at(d - EPS).is_empty());
}

#[test]
fn webs_between_holes() {
    let mut s = plate();
    // edges 2.5 apart: (20 + 2) to (26.5 - 2)
    s.cut(WallId(0), circle(20.0, 20.0, 2.0)).unwrap();
    s.cut(WallId(0), circle(26.5, 20.0, 2.0)).unwrap();
    let at = |limit: f64| {
        check(
            &s,
            &shop(Stock {
                min_web: len(limit),
                ..stock()
            }),
        )
    };
    let v = at(2.5 + EPS);
    assert_eq!(kinds(&v), ["WebTooNarrow"], "{v:?}");
    let Violation::WebTooNarrow { width, at: a, to, .. } = v[0] else {
        unreachable!()
    };
    assert!((width - 2.5).abs() < 1e-9, "{width}");
    assert!((a.distance(to) - 2.5).abs() < 1e-9);
    assert!(at(2.5 - EPS).is_empty());
}

#[test]
fn webs_across_a_neck_of_the_outline() {
    let mut s = plate();
    // notches up from the bottom and down from the top leave a neck 3 wide at x 28..32
    s.cut(
        WallId(0),
        poly(&[(28.0, -1.0), (32.0, -1.0), (32.0, 18.5), (28.0, 18.5)]),
    )
    .unwrap();
    s.cut(
        WallId(0),
        poly(&[(28.0, 21.5), (32.0, 21.5), (32.0, 41.0), (28.0, 41.0)]),
    )
    .unwrap();
    let at = |limit: f64| {
        check(
            &s,
            &shop(Stock {
                min_web: len(limit),
                ..stock()
            }),
        )
    };
    let v = at(3.0 + EPS);
    assert_eq!(kinds(&v), ["WebTooNarrow"], "{v:?}");
    let Violation::WebTooNarrow { width, .. } = v[0] else {
        unreachable!()
    };
    assert!((width - 3.0).abs() < 1e-9, "{width}");
    assert!(at(3.0 - EPS).is_empty());
    // a plain plate has no neck, whatever the limit up to its own width
    assert!(
        check(
            &plate(),
            &shop(Stock {
                min_web: len(39.0),
                ..stock()
            })
        )
        .is_empty()
    );
}

#[test]
fn operations_the_shop_does_not_offer() {
    let s = l();
    let offered = |ops: Vec<Operation>| ShopProfile {
        operations: Some(ops),
        ..shop(stock())
    };
    assert!(check(&s, &offered(vec![Operation::Laser, Operation::Bend])).is_empty());
    let v = check(&s, &offered(vec![Operation::Laser]));
    assert_eq!(kinds(&v), ["OperationNotOffered"]);
    assert!(matches!(
        v[0],
        Violation::OperationNotOffered {
            operation: Operation::Bend,
            ..
        }
    ));
    // a stock the shop cuts but does not bend
    let cut_only = Stock {
        not: vec![Operation::Bend],
        ..stock()
    };
    assert_eq!(
        kinds(&check(&s, &shop(cut_only.clone()))),
        ["OperationNotOffered"]
    );
    assert!(check(&plate(), &shop(cut_only)).is_empty());
}

#[test]
fn a_stocks_own_angles_and_bend_length() {
    let s = l();
    let right = dmath::PI / 2.0;
    let angles = |hi: f64| {
        check(
            &s,
            &shop(Stock {
                angles: Some((0.0, hi)),
                ..stock()
            }),
        )
    };
    assert_eq!(kinds(&angles(right - 1e-6)), ["BendAngleOutOfRange"]);
    assert!(angles(right + 1e-6).is_empty());
    // the bend line is the edge's 40
    let long = |max: f64| {
        check(
            &s,
            &shop(Stock {
                bend_length_max: Some(max),
                ..stock()
            }),
        )
    };
    assert_eq!(kinds(&long(40.0 - 1e-3)), ["BendTooLong"]);
    assert!(long(40.0 + 1e-3).is_empty());
}

#[test]
fn a_bent_blank_is_held_to_the_brakes_size() {
    let s = l();
    let process = |bent: (f64, f64)| ShopProfile {
        process: Process {
            flat_max: Some((1e3, 1e3)),
            bent_max: Some(bent),
            ..Default::default()
        },
        ..shop(stock())
    };
    assert_eq!(kinds(&check(&s, &process((39.0, 80.0)))), ["FlatTooLarge"]);
    assert!(check(&s, &process((41.0, 200.0))).is_empty());
    // a flat part is the laser's, not the brake's
    assert!(check(&plate(), &process((10.0, 10.0))).is_empty());
}

#[test]
fn a_partial_profile_checks_only_what_it_states() {
    let mut s = l();
    s.cut(WallId(0), circle(20.0, 20.0, 1.5)).unwrap();
    // no radii stated: the bend's radius is not refused
    let v = check(
        &s,
        &shop(Stock {
            radii: vec![],
            ..stock()
        }),
    );
    assert!(v.is_empty(), "{v:?}");
    let v = check(
        &s,
        &shop(Stock {
            radii: vec![R + 1.0],
            ..stock()
        }),
    );
    assert_eq!(kinds(&v), ["RadiusNotTooled"]);
    // a hardware row stating no hardness limit is not refused for the stock's not stating its
    let mut h = plate();
    let part = forge_sheet::Hardware {
        max_hardness: f64::INFINITY,
        ..forge_sheet::placeholder_catalogue()[0].clone()
    };
    h.mount(WallId(0), Point2::new(30.0, 20.0), part).unwrap();
    assert!(!kinds(&check(&h, &shop(stock()))).contains(&"HardnessUnknown"));
}

#[test]
fn a_thickness_matches_its_stock_to_the_rounding_it_is_written_with() {
    // 0.063 in is 1.6002 mm; written 1.60 it is still that stock
    let sh = shop(Stock {
        thickness: 0.063 * 25.4,
        ..stock()
    });
    assert!(sh.stock(1.6).is_some());
    assert!(sh.stock(1.6 + 0.01).is_none());
}

#[test]
fn a_minimum_flange_to_the_outside() {
    let s = l();
    // the flange reaches 30 from the mold line on its reference skin, the bend's inside here;
    // its outside dimension is t tan 45deg more
    let reach = |limit: f64, outside: bool| {
        check(
            &s,
            &shop(Stock {
                min_flange: len(limit),
                flange_outside: outside,
                ..stock()
            }),
        )
    };
    let short = |v: &[Violation]| v.iter().any(|x| matches!(x, Violation::FlangeTooShort { .. }));
    assert!(short(&reach(30.0 + EPS, false)));
    assert!(!short(&reach(30.0 - EPS, false)));
    assert!(short(&reach(30.0 + T + EPS, true)));
    assert!(!short(&reach(30.0 + T - EPS, true)));
}

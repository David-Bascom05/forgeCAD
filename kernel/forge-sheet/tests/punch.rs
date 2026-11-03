//! Cut punches: every tool shape's area exact against the folded and flat volumes, a hit on a
//! flange carried to the flat and its DXF on the PUNCH layer, the shop checks (tool in the
//! library, clear of bends, web between hits), typed refusals, a fuzz, timing.
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendRule, FeatureId, Gauge, Map, PunchError, PunchViolation, Sheet, SheetError, ShopProfile, Tool,
    ToolShape, Violation, WallId, build_with, dxf,
};
use forge_topo::Body;
use std::time::Instant;

const T: f64 = 1.5;

fn gauge() -> Gauge {
    Gauge {
        thickness: T,
        radius: 2.0,
        rule: BendRule::K(0.4),
    }
}

fn rect(l: f64, w: f64) -> ProfileLoop {
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

/// A base 80 x 50 with a flange 30 up off its right edge.
fn l() -> Sheet {
    let (mut s, base) = Sheet::face(FeatureId(3), gauge(), Frame::WORLD, &[rect(80.0, 50.0)]).unwrap();
    s.flange(base, 1, dmath::PI / 2.0, 30.0).unwrap();
    s
}

fn checked(b: &Body) {
    let problems = forge_topo::check::check(b);
    assert!(problems.is_empty(), "{problems:?}");
    let mesh = forge_mesh::tessellate(b, &forge_mesh::Quality::default()).expect("a mesh");
    assert!(mesh.open_edges().is_empty(), "open mesh");
}

fn vol(b: &Body) -> (f64, f64) {
    let m = forge_topo::mass::mass(b).unwrap();
    (m.volume, m.err[0])
}

fn tools() -> Vec<Tool> {
    let t = |id: &str, shape| Tool { id: id.into(), shape };
    vec![
        t("R6", ToolShape::Round { d: 6.0 }),
        t(
            "OB12x5",
            ToolShape::Obround {
                length: 12.0,
                width: 5.0,
            },
        ),
        t("SQ8", ToolShape::Square { side: 8.0 }),
        t(
            "RE14x4",
            ToolShape::Rect {
                length: 14.0,
                width: 4.0,
            },
        ),
        t("D8", ToolShape::D { d: 8.0, flat: 2.5 }),
        t(
            "KH9",
            ToolShape::Keyhole {
                d: 9.0,
                width: 4.0,
                length: 9.0,
            },
        ),
        t("HX10", ToolShape::Hex { across_flats: 10.0 }),
        t(
            "CL3",
            ToolShape::Cluster {
                d: 2.0,
                px: 4.0,
                py: 3.5,
                nx: 3,
                ny: 2,
            },
        ),
    ]
}

#[test]
fn every_tool_takes_out_its_area() {
    let plain = l();
    let (f0, l0) = (
        vol(&build_with(&plain, Map::Folded).unwrap().body).0,
        vol(&build_with(&plain, Map::Flat).unwrap().body).0,
    );
    for tool in tools() {
        for angle in [0.0, 0.7] {
            let mut s = l();
            // on the base, and on the flange
            s.punch(WallId(0), &tool, Point2::new(30.0, 25.0), angle).unwrap();
            s.punch(WallId(1), &tool, Point2::new(18.0, 20.0), angle).unwrap();
            let (f, fl) = (
                build_with(&s, Map::Folded).unwrap(),
                build_with(&s, Map::Flat).unwrap(),
            );
            checked(&f.body);
            checked(&fl.body);
            let want = 2.0 * T * tool.shape.area();
            let ((vf, ef), (vl, el)) = (vol(&f.body), vol(&fl.body));
            assert!(
                ((f0 - vf) - want).abs() <= ef + 1e-9 * f0,
                "{}: {} {want}",
                tool.id,
                f0 - vf
            );
            assert!(
                ((l0 - vl) - want).abs() <= el + 1e-9 * l0,
                "{}: {} {want}",
                tool.id,
                l0 - vl
            );
        }
    }
}

#[test]
fn hits_reach_the_flat_and_its_dxf() {
    let mut s = l();
    let r6 = tools()[0].clone();
    s.punch(WallId(1), &r6, Point2::new(18.0, 20.0), 0.0).unwrap();
    let flat = build_with(&s, Map::Flat).unwrap();
    let p = flat.pattern.as_ref().unwrap();
    assert_eq!(p.hits.len(), 1);
    // the hit is the centre of the punched circle on the flat
    let centre = p
        .contours
        .iter()
        .flatten()
        .find_map(|c| match c {
            forge_sheet::Curve2::Arc { c, r, .. } if (*r - 3.0).abs() < 1e-12 => Some(*c),
            _ => None,
        })
        .expect("the hole");
    assert!(
        centre.distance(p.hits[0].at) < 1e-12,
        "{centre:?} {:?}",
        p.hits[0].at
    );
    // folded: 18 up the flange; flat: past the base's straight, the allowance, then 18 less the
    // flange's setback
    let ba = dmath::PI / 2.0 * (2.0 + 0.4 * T);
    assert!((p.hits[0].at.x - (80.0 - 2.0 + ba + (18.0 - 2.0))).abs() < 1e-12);
    let text = dxf(p);
    assert!(
        text.contains("\nPUNCH\n") && text.contains("\nR6\n"),
        "no hit in the DXF"
    );
    assert_eq!(text.matches("\nPOINT\n").count(), 1);
}

#[test]
fn shop_checks() {
    let lib = tools();
    let shop = ShopProfile {
        tools: lib.clone(),
        ..Default::default()
    };
    let punches = |s: &Sheet| -> Vec<PunchViolation> {
        forge_sheet::check(s, &shop)
            .into_iter()
            .filter_map(|v| match v {
                Violation::Punch { why, .. } => Some(why),
                _ => None,
            })
            .collect()
    };
    let mut s = l();
    s.punch(WallId(0), &lib[0], Point2::new(30.0, 25.0), 0.0).unwrap();
    assert!(punches(&s).is_empty(), "{:?}", punches(&s));
    // a tool the shop does not have
    let odd = Tool {
        id: "R7".into(),
        shape: ToolShape::Round { d: 7.0 },
    };
    s.punch(WallId(0), &odd, Point2::new(50.0, 25.0), 0.0).unwrap();
    assert_eq!(punches(&s), vec![PunchViolation::NotInShop { tool: "R7".into() }]);
    // a hit 1 from the base's tangent line (x = 78) reaching 2 into the zone
    let mut s = l();
    s.punch(WallId(0), &lib[0], Point2::new(79.0, 25.0), 0.0).unwrap();
    match &punches(&s)[..] {
        [PunchViolation::AcrossBend { depth, .. }] => assert!((depth - 4.0).abs() < 1e-12, "{depth}"),
        o => panic!("{o:?}"),
    }
    // two rounds 7 apart: a web of 1, under t
    let mut s = l();
    s.punch(WallId(0), &lib[0], Point2::new(30.0, 25.0), 0.0).unwrap();
    s.punch(WallId(0), &lib[0], Point2::new(37.0, 25.0), 0.0).unwrap();
    assert_eq!(
        punches(&s),
        vec![PunchViolation::Spacing {
            other: 1,
            web: 1.0,
            limit: T
        }]
    );
}

#[test]
fn refusals_are_typed() {
    let mut s = l();
    let bad = Tool {
        id: "D".into(),
        shape: ToolShape::D { d: 8.0, flat: 5.0 },
    };
    assert_eq!(
        s.punch(WallId(0), &bad, Point2::new(30.0, 25.0), 0.0),
        Err(SheetError::Punch {
            feature: FeatureId(3),
            why: PunchError::Size {
                which: "flat",
                value: 5.0,
                limit: 4.0
            }
        })
    );
    let cl = Tool {
        id: "C".into(),
        shape: ToolShape::Cluster {
            d: 3.0,
            px: 2.0,
            py: 5.0,
            nx: 2,
            ny: 2,
        },
    };
    assert!(matches!(
        s.punch(WallId(0), &cl, Point2::new(30.0, 25.0), 0.0),
        Err(SheetError::Punch {
            why: PunchError::Size { which: "px", .. },
            ..
        })
    ));
    assert!(s.punches.is_empty() && s.cuts.is_empty());
}

#[test]
fn fuzz_hits() {
    let mut rng = Rng::new(0x9c4);
    let lib = tools();
    let (mut built, mut refused) = (0, 0);
    let (f0, l0) = {
        let s = l();
        (
            vol(&build_with(&s, Map::Folded).unwrap().body).0,
            vol(&build_with(&s, Map::Flat).unwrap().body).0,
        )
    };
    for seed in 0..120 {
        let mut s = l();
        let mut area = 0.0;
        // hits in separate bands of the base so they never overlap each other
        let n = 1 + rng.below(3);
        for i in 0..n {
            let tool = &lib[rng.below(lib.len())];
            let x = 12.0 + 20.0 * i as f64 + rng.range(-1.0, 1.0);
            let at = Point2::new(x, rng.range(12.0, 38.0));
            s.punch(WallId(0), tool, at, rng.range(-3.0, 3.0)).unwrap();
            area += tool.shape.area();
        }
        match (build_with(&s, Map::Folded), build_with(&s, Map::Flat)) {
            (Ok(f), Ok(fl)) => {
                checked(&f.body);
                checked(&fl.body);
                let ((vf, ef), (vl, el)) = (vol(&f.body), vol(&fl.body));
                assert!(((f0 - vf) - T * area).abs() <= ef + 1e-9 * f0, "seed {seed}");
                assert!(((l0 - vl) - T * area).abs() <= el + 1e-9 * l0, "seed {seed}");
                built += 1;
            }
            (Err(SheetError::NearCoincident { .. } | SheetError::Pinched { .. }), _) => refused += 1,
            (e, g) => panic!("seed {seed}: {:?} {:?}", e.err(), g.err()),
        }
    }
    println!("{built} built, {refused} refused");
    assert!(built > 100);
}

#[test]
fn timing() {
    let lib = tools();
    let mut s = l();
    for (i, t) in lib.iter().enumerate() {
        s.punch(
            WallId(0),
            t,
            Point2::new(8.0 + 9.0 * i as f64, if i % 2 == 0 { 15.0 } else { 35.0 }),
            0.0,
        )
        .unwrap();
    }
    let n = 50;
    let at = Instant::now();
    for _ in 0..n {
        std::hint::black_box(build_with(&s, Map::Folded).unwrap());
    }
    let fold = at.elapsed().as_secs_f64() / n as f64;
    let at = Instant::now();
    for _ in 0..n {
        std::hint::black_box(build_with(&s, Map::Flat).unwrap());
    }
    println!(
        "eight hits on an L: folded {:.0} us, flat {:.0} us",
        fold * 1e6,
        at.elapsed().as_secs_f64() / n as f64 * 1e6
    );
}

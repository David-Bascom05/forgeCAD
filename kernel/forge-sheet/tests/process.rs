//! The fabricator's process limits, with SendCutSend's numbers for cold rolled steel (the rack's
//! SPEC section 3, read 2026-11-03): bending flats up to 44 x 30 in (1117.6 x 762), powder coat up
//! to 30 x 36 in (762 x 914.4), bend lines up to 44 in, angles to 130 degrees, a four-sided box
//! with hardware: flanges up to 3 in (76.2). The least part size here is the test's own number.
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendRule, FeatureId, Gauge, Process, Scaled, Sheet, ShopProfile, Stock, Violation, WallId, checks,
    placeholder_catalogue,
};

const T: f64 = 1.88;

fn poly(pts: &[Point2]) -> ProfileLoop {
    let n = pts.len();
    ProfileLoop::new(
        (0..n)
            .map(|i| {
                (
                    Segment::Line {
                        a: pts[i],
                        b: pts[(i + 1) % n],
                    },
                    i as u64,
                )
            })
            .collect(),
    )
}

/// A w x l plate turned `rot`.
fn plate(w: f64, l: f64, rot: f64) -> Sheet {
    let (sn, cs) = dmath::sin_cos(rot);
    let p = |x: f64, y: f64| Point2::new(cs * x - sn * y, sn * x + cs * y);
    let g = Gauge {
        thickness: T,
        radius: 1.6,
        rule: BendRule::K(0.4),
    };
    Sheet::face(
        FeatureId(3),
        g,
        Frame::WORLD,
        &[poly(&[p(0.0, 0.0), p(w, 0.0), p(w, l), p(0.0, l)])],
    )
    .unwrap()
    .0
}

fn scs() -> Process {
    Process {
        flat_max: Some((762.0, 1117.6)),
        coated_max: Some((762.0, 914.4)),
        bend_length_max: Some(1117.6),
        box_flange_max: Some(76.2),
        box_flange_with_hardware: true,
        angles: Some((0.0, 130f64.to_radians())),
        part_min: Some((12.7, 25.4)),
    }
}

fn kinds(v: &[Violation]) -> Vec<&'static str> {
    v.iter()
        .map(|x| match x {
            Violation::FlatTooLarge { .. } => "FlatTooLarge",
            Violation::PartTooSmall { .. } => "PartTooSmall",
            Violation::BendTooLong { .. } => "BendTooLong",
            Violation::BoxFlangeTooTall { .. } => "BoxFlangeTooTall",
            Violation::BendAngleOutOfRange { .. } => "BendAngleOutOfRange",
            _ => "other",
        })
        .collect()
}

#[test]
fn flat_size_raw_and_coated_either_way_round() {
    // 700 x 900 fits both; 700 x 950 is too long to coat; 800 x 900 too wide for either
    for rot in [0.0, 0.5, -2.0] {
        assert!(checks::process(&plate(700.0, 900.0, rot), &scs()).is_empty());
        let mut s = plate(700.0, 950.0, rot);
        assert!(checks::process(&s, &scs()).is_empty());
        s.coated = true;
        let v = checks::process(&s, &scs());
        assert!(
            matches!(v.as_slice(), [Violation::FlatTooLarge { size, limit, coated: true, .. }]
                if (size.0 - 700.0).abs() < 1e-6 && (size.1 - 950.0).abs() < 1e-6 && *limit == (762.0, 914.4)),
            "{rot}: {v:?}"
        );
        let v = checks::process(&plate(800.0, 900.0, rot), &scs());
        assert_eq!(kinds(&v), ["FlatTooLarge"], "{rot}: {v:?}");
    }
    // at the limit exactly: fits
    assert!(checks::process(&plate(762.0, 1117.6, 0.3), &scs()).is_empty());
    // too small
    let v = checks::process(&plate(10.0, 30.0, 0.0), &scs());
    assert!(
        matches!(v.as_slice(), [Violation::PartTooSmall { size, .. }] if (size.0 - 10.0).abs() < 1e-9),
        "{v:?}"
    );
}

#[test]
fn bend_length_and_angle() {
    let th = dmath::PI / 2.0;
    let mut s = plate(1200.0, 300.0, 0.0);
    s.flange(WallId(0), 0, th, 20.0).unwrap(); // along the 1200 edge
    s.flange(WallId(0), 1, 2.4, 20.0).unwrap(); // 137.5 degrees on the 300 edge
    let mut lim = scs();
    lim.flat_max = None;
    lim.coated_max = None;
    let v = checks::process(&s, &lim);
    assert!(
        v.iter()
            .any(|x| matches!(x, Violation::BendTooLong { length, limit, .. }
            if (length - 1200.0).abs() < 1e-6 && *limit == 1117.6)),
        "{v:?}"
    );
    assert!(
        v.iter()
            .any(|x| matches!(x, Violation::BendAngleOutOfRange { angle, max, .. }
            if (angle - 2.4).abs() < 1e-12 && (max - 130f64.to_radians()).abs() < 1e-12)),
        "{v:?}"
    );
    assert_eq!(v.len(), 2, "{v:?}");
}

#[test]
fn flanges_round_a_box() {
    // a 300 x 200 pan with four flanges up 80: too tall with hardware, fine without (the
    // fabricator's limit is for a box carrying hardware), fine at 70
    let pan = |h: f64, hardware: bool| {
        let mut s = plate(300.0, 200.0, 0.0);
        for e in 0..4 {
            s.flange(WallId(0), e, dmath::PI / 2.0, h).unwrap();
        }
        if hardware {
            let nut = placeholder_catalogue().into_iter().next().unwrap();
            s.mount(WallId(0), Point2::new(150.0, 100.0), nut).unwrap();
        }
        s
    };
    let mut lim = scs();
    lim.flat_max = None;
    lim.coated_max = None;
    let v = checks::process(&pan(80.0, true), &lim);
    assert_eq!(kinds(&v), ["BoxFlangeTooTall"; 4], "{v:?}");
    assert!(
        v.iter()
            .all(|x| matches!(x, Violation::BoxFlangeTooTall { height, limit, .. }
        if *height > 76.2 && *limit == 76.2))
    );
    assert!(checks::process(&pan(80.0, false), &lim).is_empty());
    assert!(checks::process(&pan(70.0, true), &lim).is_empty());
    lim.box_flange_with_hardware = false;
    assert_eq!(checks::process(&pan(80.0, false), &lim).len(), 4);
    // three flanges: not a box
    let mut s = plate(300.0, 200.0, 0.0);
    for e in 0..3 {
        s.flange(WallId(0), e, dmath::PI / 2.0, 80.0).unwrap();
    }
    assert!(checks::process(&s, &lim).is_empty());
}

#[test]
fn the_shop_check_runs_them() {
    let shop = ShopProfile {
        stock: vec![Stock {
            thickness: T,
            rule: BendRule::K(0.4),
            radii: vec![1.6],
            min_flange: Scaled::NONE,
            hole_to_bend: Scaled::NONE,
            hole_to_edge: Scaled::NONE,
            min_slot: Scaled::NONE,
            min_hole: Scaled::NONE,
            relief: forge_sheet::ReliefRule::default(),
            hardness: Some(57.0),
        }],
        hardware: vec![],
        kerf: 0.2,
        process: scs(),
        ..Default::default()
    };
    let v = forge_sheet::check(&plate(800.0, 900.0, 0.0), &shop);
    assert_eq!(kinds(&v), ["FlatTooLarge"], "{v:?}");
}

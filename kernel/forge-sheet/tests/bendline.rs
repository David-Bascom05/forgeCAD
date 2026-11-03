//! Sketched bends and jogs against closed forms: a centred bend keeps the flat as drawn, a
//! positioned one the folded part; a jog's offset between reference skins; what lies beyond the
//! line goes with the turning wall; a wall hanging off a bend keeps its parent's side; typed
//! refusals; the folded clash still checked; a fuzz; timing.
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendAt, BendLineError, BendRule, FeatureId, Gauge, Map, Position, Sheet, SheetError, WallId, build_with,
};
use forge_topo::Body;
use std::time::Instant;

const T: f64 = 1.5;
const R: f64 = 2.0;
const K: f64 = 0.4;
const PI: f64 = dmath::PI;

fn gauge() -> Gauge {
    Gauge {
        thickness: T,
        radius: R,
        rule: BendRule::K(K),
    }
}

fn poly(c: &[(f64, f64)]) -> ProfileLoop {
    let n = c.len();
    ProfileLoop::new(
        (0..n)
            .map(|i| {
                let (a, b) = (c[i], c[(i + 1) % n]);
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

fn plate(l: f64, w: f64) -> Sheet {
    Sheet::face(
        FeatureId(6),
        gauge(),
        Frame::WORLD,
        &[poly(&[(0.0, 0.0), (l, 0.0), (l, w), (0.0, w)])],
    )
    .unwrap()
    .0
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

fn near(what: &str, got: (f64, f64), want: f64) {
    assert!(
        (got.0 - want).abs() <= 3.0 * got.1 + 1e-9 * want.abs(),
        "{what}: {} against {want}",
        got.0
    );
}

fn across(x: f64) -> (Point2, Point2) {
    (Point2::new(x, -10.0), Point2::new(x, 60.0))
}

/// The setback on the reference skin, the allowance and the zone's volume per unit length.
fn bend(angle: f64) -> (f64, f64, f64) {
    let th = angle.abs();
    let rho = if angle > 0.0 { R } else { R + T };
    (
        rho * dmath::sin(th) / (1.0 + dmath::cos(th)),
        th * (R + K * T),
        th * (R * T + 0.5 * T * T),
    )
}

#[test]
fn centred_bends_keep_the_flat_positioned_ones_the_fold() {
    let (l, w) = (100.0, 40.0);
    for angle in [PI / 2.0, -PI / 2.0, 1.0, -2.3] {
        let (s, ba, zone) = bend(angle);
        let mut c = plate(l, w);
        let (p0, p1) = across(50.0);
        c.sketched_bend(WallId(0), p0, p1, angle, R, BendAt::Centre)
            .unwrap();
        let (f, fl) = (
            build_with(&c, Map::Folded).unwrap(),
            build_with(&c, Map::Flat).unwrap(),
        );
        checked(&f.body);
        checked(&fl.body);
        near("centre flat", vol(&fl.body), T * w * l);
        near("centre folded", vol(&f.body), T * w * (l - ba) + w * zone);
        for p in [Position::Outside, Position::Inside, Position::BendOutside] {
            let mut c = plate(l, w);
            c.sketched_bend(WallId(0), p0, p1, angle, R, BendAt::Position(p))
                .unwrap();
            let (f, fl) = (
                build_with(&c, Map::Folded).unwrap(),
                build_with(&c, Map::Flat).unwrap(),
            );
            checked(&f.body);
            checked(&fl.body);
            near(
                "positioned folded",
                vol(&f.body),
                T * w * (l - 2.0 * s) + w * zone,
            );
            near("positioned flat", vol(&fl.body), T * w * (l - 2.0 * s + ba));
        }
    }
}

#[test]
fn a_jog_stands_off_by_its_offset() {
    let (l, w, x, off) = (120.0, 40.0, 40.0, 10.0);
    for angle in [PI / 2.0, -PI / 2.0, 1.2] {
        let mut s = plate(l, w);
        let (p0, p1) = across(x);
        let (mid, _, far, _) = s
            .jog(
                WallId(0),
                p0,
                p1,
                off,
                angle,
                R,
                BendAt::Position(Position::Outside),
            )
            .unwrap();
        let f = build_with(&s, Map::Folded).unwrap();
        checked(&f.body);
        checked(&build_with(&s, Map::Flat).unwrap().body);
        let place = |id: WallId| f.layout.walls.iter().find(|p| p.wall == id).unwrap().frame;
        let fr = place(far);
        // the far wall parallel to the first, its reference skin `off` along the first's normal
        assert!((fr.z().get().z - 1.0).abs() < 1e-12, "{:?}", fr.z());
        assert!(
            (fr.origin.z - angle.signum() * off).abs() < 1e-9,
            "{}",
            fr.origin.z
        );
        let _ = place(mid);
        let ((s1, _, z1), (s2, _, z2)) = (bend(angle), bend(-angle));
        let m = off / dmath::sin(angle.abs());
        // Outside up puts the first mold line on x; down, Outside stands a sharp out
        let th = angle.abs();
        let sharp = T * dmath::sin(th) / (1.0 + dmath::cos(th));
        let d = if angle > 0.0 { 0.0 } else { sharp };
        let straights = (x + d - s1) + (m - s1 - s2) + (l - x - d - m - s2);
        near("jog folded", vol(&f.body), T * w * straights + w * (z1 + z2));
    }
}

#[test]
fn what_lies_beyond_goes_with_the_turning_wall() {
    let (l, w) = (100.0, 40.0);
    let hole = |x: f64| {
        ProfileLoop::new(vec![(
            Segment::Circle {
                center: Point2::new(x, 20.0),
                radius: 3.0,
                ccw: true,
            },
            0,
        )])
    };
    let mut s = plate(l, w);
    s.cut(WallId(0), hole(75.0)).unwrap();
    s.cut(WallId(0), hole(20.0)).unwrap();
    let (p0, p1) = across(50.0);
    let (b, _) = s
        .sketched_bend(
            WallId(0),
            p0,
            p1,
            PI / 2.0,
            R,
            BendAt::Position(Position::Outside),
        )
        .unwrap();
    assert_eq!(s.cuts[0].wall, b);
    assert_eq!(s.cuts[1].wall, WallId(0));
    let (sb, ba, zone) = bend(PI / 2.0);
    let f = build_with(&s, Map::Folded).unwrap();
    checked(&f.body);
    near(
        "holes",
        vol(&f.body),
        T * w * (l - 2.0 * sb) + w * zone - 2.0 * PI * 9.0 * T,
    );
    let _ = ba;
    // a flange's wall bent again: the line read so its parent's side stays
    let mut s = plate(60.0, 40.0);
    let (fl, _) = s.flange(WallId(0), 1, PI / 2.0, 50.0).unwrap();
    // the flange's x runs from its mold line: a line whose right side holds the mold line
    let (b, j) = s
        .sketched_bend(
            fl,
            Point2::new(25.0, 50.0),
            Point2::new(25.0, -10.0),
            PI / 2.0,
            R,
            BendAt::Centre,
        )
        .unwrap();
    let joint = s.joint(j).unwrap();
    assert_eq!((joint.a, joint.b), (fl, b));
    let f = build_with(&s, Map::Folded).unwrap();
    checked(&f.body);
}

#[test]
fn refusals_are_typed() {
    let why = |s: &mut Sheet, p0: Point2, p1: Point2| match s.sketched_bend(
        WallId(0),
        p0,
        p1,
        PI / 2.0,
        R,
        BendAt::Centre,
    ) {
        Err(SheetError::BendLine { why, .. }) => why,
        o => panic!("{:?}", o.map(|_| ())),
    };
    let p = Point2::new;
    let mut s = plate(100.0, 40.0);
    assert!(matches!(
        why(&mut s, p(150.0, 0.0), p(150.0, 40.0)),
        BendLineError::Crossings { count: 0, .. }
    ));
    assert!(matches!(
        why(&mut s, p(40.0, 0.0), p(60.0, 40.0)),
        BendLineError::NotSquare { .. }
    ));
    assert!(matches!(
        why(&mut s, p(0.0, 0.0), p(0.0, 40.0)),
        BendLineError::NotSquare { .. } | BendLineError::Crossings { .. }
    ));
    match s.sketched_bend(WallId(0), p(1.0, 0.0), p(1.0, 40.0), PI / 2.0, R, BendAt::Centre) {
        Err(SheetError::WallTooShort { wall: WallId(0), .. }) => {}
        o => panic!("{:?}", o.map(|_| ())),
    }
    assert!(matches!(
        why(&mut s, p(5.0, 5.0), p(5.0, 5.0)),
        BendLineError::Line { .. }
    ));
    // a U-shaped wall crossed four times
    let mut u = Sheet::face(
        FeatureId(6),
        gauge(),
        Frame::WORLD,
        &[poly(&[
            (0.0, 0.0),
            (100.0, 0.0),
            (100.0, 40.0),
            (70.0, 40.0),
            (70.0, 10.0),
            (30.0, 10.0),
            (30.0, 40.0),
            (0.0, 40.0),
        ])],
    )
    .unwrap()
    .0;
    assert!(matches!(
        why(&mut u, p(-10.0, 25.0), p(110.0, 25.0)),
        BendLineError::Crossings { count: 4, .. }
    ));
    // a bend already on a crossed edge
    let mut s = plate(100.0, 40.0);
    s.flange(WallId(0), 0, PI / 2.0, 20.0).unwrap();
    assert!(matches!(
        why(&mut s, p(50.0, -10.0), p(50.0, 60.0)),
        BendLineError::EdgeTaken { segment: 0, .. }
    ));
    // a hole of the outline across the band
    let mut h = Sheet::face(
        FeatureId(6),
        gauge(),
        Frame::WORLD,
        &[poly(&[(0.0, 0.0), (100.0, 0.0), (100.0, 40.0), (0.0, 40.0)]), {
            let mut h = poly(&[(45.0, 10.0), (45.0, 30.0), (55.0, 30.0), (55.0, 10.0)]);
            h.segments.iter_mut().for_each(|s| s.1 += 10);
            h
        }],
    )
    .unwrap()
    .0;
    assert!(matches!(
        why(&mut h, p(50.0, -10.0), p(50.0, 60.0)),
        BendLineError::HoleAcross { .. }
    ));
    // a jog too short for its two setbacks
    let mut s = plate(100.0, 40.0);
    match s.jog(
        WallId(0),
        p(40.0, -10.0),
        p(40.0, 60.0),
        2.0,
        PI / 2.0,
        R,
        BendAt::Centre,
    ) {
        Err(SheetError::BendLine {
            why: BendLineError::JogTooShort { .. },
            ..
        }) => {}
        o => panic!("{:?}", o.map(|_| ())),
    }
    // and the folded clash still checked: two ends folded back onto each other
    let mut s = plate(100.0, 40.0);
    let mut wall = WallId(0);
    for x in [30.0, 70.0] {
        wall = s
            .sketched_bend(
                wall,
                p(x, -10.0),
                p(x, 60.0),
                170.0 * PI / 180.0,
                R,
                BendAt::Centre,
            )
            .unwrap()
            .0;
    }
    match build_with(&s, Map::Folded) {
        Err(SheetError::FoldedClash { .. } | SheetError::FoldedTouch { .. }) => {}
        o => panic!("{:?}", o.map(|_| ())),
    }
}

#[test]
fn fuzz_bend_lines() {
    let mut rng = Rng::new(0xbe7d);
    let (mut built, mut refused) = (0, 0);
    for _ in 0..300 {
        let (l, w) = (rng.range(40.0, 150.0), rng.range(10.0, 60.0));
        let x = rng.range(-5.0, l + 5.0);
        let angle = rng.range(-2.8, 2.8);
        let r = rng.range(0.5, 5.0);
        let at = match rng.below(4) {
            0 => BendAt::Centre,
            1 => BendAt::Position(Position::Outside),
            2 => BendAt::Position(Position::Inside),
            _ => BendAt::Position(Position::BendOutside),
        };
        let mut s = plate(l, w);
        let (p0, p1) = (Point2::new(x, -1.0), Point2::new(x, w + 1.0));
        let r0 = if rng.bool() {
            s.sketched_bend(WallId(0), p0, p1, angle, r, at).map(|_| ())
        } else {
            s.jog(WallId(0), p0, p1, rng.range(1.0, 20.0), angle, r, at)
                .map(|_| ())
        };
        match r0 {
            Err(
                SheetError::BendLine { .. } | SheetError::BendAngle { .. } | SheetError::WallTooShort { .. },
            ) => {
                refused += 1;
                continue;
            }
            Err(e) => panic!("{e:?}"),
            Ok(()) => {}
        }
        for map in [Map::Folded, Map::Flat] {
            match build_with(&s, map) {
                Ok(b) => checked(&b.body),
                Err(
                    SheetError::WallTooShort { .. }
                    | SheetError::FoldedClash { .. }
                    | SheetError::FoldedTouch { .. }
                    | SheetError::FlatOverlap { .. }
                    | SheetError::FlatTouch { .. }
                    | SheetError::NeedsRelief { .. }
                    | SheetError::Pinched { .. }
                    | SheetError::HoleOutside { .. }
                    | SheetError::Unchained { .. }
                    | SheetError::TrimTouchesBendLine { .. }
                    | SheetError::EdgeTooShort { .. },
                ) => {}
                Err(e) => panic!("{map:?} x {x} l {l} angle {angle} r {r} {at:?}: {e:?}"),
            }
        }
        built += 1;
    }
    println!("{built} built, {refused} refused");
    assert!(built >= 150, "{built} built, {refused} refused");
}

#[test]
fn timing() {
    let mut s = plate(200.0, 50.0);
    for (i, x) in [40.0, 90.0, 140.0].into_iter().enumerate() {
        let wall = s.walls[s.walls.len() - 1].id;
        let _ = i;
        s.sketched_bend(
            wall,
            Point2::new(x, -10.0),
            Point2::new(x, 60.0),
            0.6,
            R,
            BendAt::Centre,
        )
        .unwrap();
    }
    let n = 50;
    let at = Instant::now();
    for _ in 0..n {
        std::hint::black_box(build_with(&s, Map::Folded).unwrap());
    }
    println!(
        "three sketched bends: folded {:.0} us",
        at.elapsed().as_secs_f64() / n as f64 * 1e6
    );
}

//! Forming tools: dimples and extruded holes against closed forms (folded volume by Pappus,
//! taken here piece by piece from the tool's numbers alone; the flat's pierce), both directions,
//! the flat and its DXF marks on the FORM layer, the shop checks, typed refusals (louvres and
//! lances unbuilt), the folded clash check catching a form against a flange and another form, a
//! fuzz, timing.
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendRule, FeatureId, FormAgainst, FormError, FormRules, FormShape, FormTool, FormViolation, Gauge, Map,
    Sheet, SheetError, SheetWarning, ShopProfile, Violation, WallId, build_with, dxf,
};
use forge_topo::Body;
use std::time::Instant;

const T: f64 = 1.5;
const PI: f64 = dmath::PI;

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
    let (mut s, base) = Sheet::face(FeatureId(5), gauge(), Frame::WORLD, &[rect(80.0, 50.0)]).unwrap();
    s.flange(base, 1, PI / 2.0, 30.0).unwrap();
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

fn dimple(d: f64, depth: f64, angle: f64, die_r: f64, punch_r: f64) -> FormShape {
    FormShape::Dimple {
        d,
        depth,
        angle,
        die_r,
        punch_r,
    }
}

fn hole(d: f64, height: f64, die_r: f64, pierce: f64) -> FormShape {
    FormShape::ExtrudedHole {
        d,
        height,
        die_r,
        pierce,
    }
}

fn tool(id: &str, shape: FormShape) -> FormTool {
    FormTool {
        id: id.into(),
        shape,
        up: true,
        down: true,
    }
}

/// The formed material's volume less the disc of sheet it replaces, from the tool's numbers by
/// Pappus: each piece's area times the path of its centroid, written out piece by piece.
fn closed_form(shape: FormShape, t: f64) -> (f64, f64) {
    let tau = 2.0 * PI;
    match shape {
        FormShape::Dimple {
            d,
            depth: h,
            angle: b,
            die_r,
            punch_r,
        } => {
            let r1 = 0.5 * d;
            let (ro, ri) = (die_r + t, die_r);
            let (po, pi_) = (punch_r + t, punch_r);
            // a ring sector of radii ri..ro through angle b about r = c, bowing towards the axis
            // (-1) or away (+1): its first moment about the axis
            let sector = |c: f64, ri: f64, ro: f64, toward: f64| {
                0.5 * b * (ro * ro - ri * ri) * c + toward * (ro.powi(3) - ri.powi(3)) * (1.0 - dmath::cos(b)) / 3.0
            };
            let rim = sector(r1, ri, ro, -1.0);
            let wall = (h - (ro + pi_) * (1.0 - dmath::cos(b))) / dmath::sin(b);
            let r2 = r1 - (ro + pi_) * dmath::sin(b) - wall * dmath::cos(b);
            let crown = sector(r2, pi_, po, 1.0);
            // the wall: a rectangle wall x t, centroid half way along and t/2 across
            let mid = r1 - ro * dmath::sin(b) - 0.5 * wall * dmath::cos(b) + 0.5 * t * dmath::sin(b);
            let disc = 0.5 * r2 * r2 * t;
            (
                tau * (rim + wall * t * mid + crown + disc) - PI * r1 * r1 * t,
                0.0,
            )
        }
        FormShape::ExtrudedHole {
            d,
            height,
            die_r,
            pierce,
        } => {
            let rb = 0.5 * d;
            let ro = die_r + t;
            let r1 = rb + ro;
            // a quarter ring sector about r = r1, bowing towards the axis
            let rim = 0.25 * PI * (ro * ro - die_r * die_r) * r1 - (ro.powi(3) - die_r.powi(3)) / 3.0;
            let collar = (height - ro) * t * (rb + 0.5 * t);
            (
                tau * (rim + collar) - PI * r1 * r1 * t,
                -PI * 0.25 * pierce * pierce * t,
            )
        }
        _ => unreachable!("built kinds only"),
    }
}

#[test]
fn forms_against_closed_forms() {
    let plain = l();
    let (f0, l0) = (
        vol(&build_with(&plain, Map::Folded).unwrap().body).0,
        vol(&build_with(&plain, Map::Flat).unwrap().body).0,
    );
    let folded_faces = build_with(&plain, Map::Folded).unwrap().body.face_ids().count();
    let mut worst: f64 = 0.0;
    let shapes = [
        dimple(20.0, 4.0, 1.0, 1.0, 2.0),
        dimple(24.0, 3.0, 0.5, 2.0, 1.5),
        dimple(16.0, 6.0, 1.4, 0.8, 1.0),
        hole(8.0, 5.0, 1.0, 5.0),
        hole(10.0, 4.0, 0.5, 6.0),
    ];
    for shape in shapes {
        for up in [false, true] {
            for angle in [0.0, 0.7] {
                let mut s = l();
                s.form(WallId(0), &tool("F", shape), Point2::new(30.0, 25.0), angle, up)
                    .unwrap();
                let (f, fl) = (
                    build_with(&s, Map::Folded).unwrap(),
                    build_with(&s, Map::Flat).unwrap(),
                );
                checked(&f.body);
                checked(&fl.body);
                let (df, dl) = closed_form(shape, T);
                let ((vf, ef), (vl, el)) = (vol(&f.body), vol(&fl.body));
                worst = worst.max((vf - f0 - df).abs() / df.abs());
                assert!(
                    f.body.face_ids().count() > folded_faces,
                    "the form's faces are there"
                );
                assert!(
                    (vf - f0 - df).abs() <= 1e-6 * f0 + 3.0 * ef,
                    "{shape:?} up {up}: folded {vf} vs {} (err {ef})",
                    f0 + df
                );
                assert!(
                    (vl - l0 - dl).abs() <= 1e-6 * l0 + 3.0 * el,
                    "{shape:?}: flat {vl} vs {}",
                    l0 + dl
                );
                assert!(f.warnings.is_empty() && fl.warnings.is_empty());
            }
        }
    }
    println!("worst folded volume change off its closed form: {worst:.2e} of it");
    assert!(worst < 1e-7);
}

#[test]
fn the_kernels_volume_change_is_the_closed_form() {
    for shape in [dimple(20.0, 4.0, 1.0, 1.0, 2.0), hole(8.0, 5.0, 1.0, 5.0)] {
        let (df, _) = closed_form(shape, T);
        let k = shape.volume_change(T).unwrap();
        assert!((k - df).abs() <= 1e-12 * df.abs().max(1.0), "{k} vs {df}");
    }
}

#[test]
fn marks_reach_the_flat_and_its_dxf() {
    let mut s = l();
    // on the flange, pressed up (towards its reference side), turned
    s.form(
        WallId(1),
        &tool("DM20", dimple(20.0, 4.0, 1.0, 1.0, 2.0)),
        Point2::new(16.0, 25.0),
        0.3,
        true,
    )
    .unwrap();
    s.form(
        WallId(0),
        &tool("EH8", hole(8.0, 5.0, 1.0, 5.0)),
        Point2::new(20.0, 20.0),
        0.0,
        false,
    )
    .unwrap();
    let flat = build_with(&s, Map::Flat).unwrap();
    let pat = flat.pattern.as_ref().unwrap();
    assert_eq!(pat.forms.len(), 2);
    let w1 = flat.layout.walls.iter().find(|w| w.wall == WallId(1)).unwrap();
    let m = &pat.forms[0];
    assert!(m.at.distance(w1.developed.apply(Point2::new(16.0, 25.0))) < 1e-12 && m.up);
    assert!((m.angle - (0.3 + dmath::atan2(w1.developed.x.y, w1.developed.x.x))).abs() < 1e-12);
    // the pierce is a real cut on the flat: a circle among the contours; the dimple cuts nothing
    let circles = pat
        .contours
        .iter()
        .flatten()
        .filter(|c| matches!(c, forge_sheet::Curve2::Arc { r, .. } if (r - 2.5).abs() < 1e-12))
        .count();
    assert!(circles >= 1, "{:?}", pat.contours);
    let text = dxf(pat);
    assert!(text.contains("\nFORM\n") && text.contains("DM20 UP") && text.contains("EH8 DOWN"));
    // a part with no forms writes no FORM layer: its file is as it was
    let plain = build_with(&l(), Map::Flat).unwrap();
    assert!(!dxf(plain.pattern.as_ref().unwrap()).contains("FORM"));
}

fn shop() -> ShopProfile {
    let mut down_only = tool("DM20", dimple(20.0, 4.0, 1.0, 1.0, 2.0));
    down_only.up = false;
    ShopProfile {
        forms: vec![down_only, tool("EH8", hole(8.0, 5.0, 1.0, 5.0))],
        form_rules: FormRules::default(),
        ..ShopProfile::default()
    }
}

#[test]
fn shop_checks() {
    let rules = FormRules::default();
    let viol = |s: &Sheet| -> Vec<FormViolation> {
        forge_sheet::check(s, &shop())
            .into_iter()
            .filter_map(|v| match v {
                Violation::Form { why, .. } => Some(why),
                _ => None,
            })
            .collect()
    };
    let dm = tool("DM20", dimple(20.0, 4.0, 1.0, 1.0, 2.0));
    // well clear, the shop's tool, its direction
    let mut s = l();
    s.form(WallId(0), &dm, Point2::new(30.0, 25.0), 0.0, false)
        .unwrap();
    assert!(viol(&s).is_empty(), "{:?}", viol(&s));
    // not the shop's tool; the shop's tool the wrong way
    let mut s = l();
    s.form(
        WallId(0),
        &tool("X", dimple(22.0, 4.0, 1.0, 1.0, 2.0)),
        Point2::new(30.0, 25.0),
        0.0,
        false,
    )
    .unwrap();
    s.form(WallId(0), &dm, Point2::new(55.0, 25.0), 0.0, true)
        .unwrap();
    let v = viol(&s);
    assert!(
        v.contains(&FormViolation::NotInShop { tool: "X".into() }),
        "{v:?}"
    );
    assert!(v.contains(&FormViolation::Direction {
        tool: "DM20".into(),
        up: true
    }));
    // too deep for the rule; near the bend; near an edge; too close to another form
    let deep = tool("D", dimple(30.0, 10.0, 1.0, 1.0, 2.0));
    let mut s = l();
    s.form(WallId(0), &deep, Point2::new(25.0, 25.0), 0.0, false)
        .unwrap();
    let v = viol(&s);
    assert!(v.iter().any(|x| matches!(x, FormViolation::TooDeep { depth, limit } if *depth == 10.0 && (*limit - 6.0 * T).abs() < 1e-12)));
    let mut s = l();
    // rim 10 from (66, 25): the bend's tangent line is at 80 - s on the base
    s.form(WallId(0), &dm, Point2::new(66.0, 25.0), 0.0, false)
        .unwrap();
    let v = viol(&s);
    // the reference skin's setback: R tan(45deg)
    let setback = 2.0;
    let want = 80.0 - setback - 66.0 - 10.0;
    assert!(
        v.iter().any(
            |x| matches!(x, FormViolation::NearBend { distance, limit, away, .. }
            if (distance - want).abs() < 1e-9 && (*limit - (3.0 * T + 2.0)).abs() < 1e-12 && away.x < -0.999)
        ),
        "{v:?}"
    );
    let mut s = l();
    s.form(WallId(0), &dm, Point2::new(30.0, 12.0), 0.0, false)
        .unwrap();
    let v = viol(&s);
    assert!(v.iter().any(|x| matches!(x, FormViolation::NearEdge { segment: 0, distance, .. } if (distance - 2.0).abs() < 1e-9)), "{v:?}");
    let mut s = l();
    s.form(WallId(0), &dm, Point2::new(20.0, 25.0), 0.0, false)
        .unwrap();
    s.form(WallId(0), &dm, Point2::new(42.0, 25.0), 0.0, false)
        .unwrap();
    let v = viol(&s);
    assert!(
        v.contains(&FormViolation::Spacing {
            other: 1,
            web: 2.0,
            limit: rules.spacing.at(T, 0.0)
        }) || v
            .iter()
            .any(|x| matches!(x, FormViolation::Spacing { other: 1, web, .. } if (web - 2.0).abs() < 1e-9)),
        "{v:?}"
    );
}

#[test]
fn refusals_are_typed() {
    let form = |shape: FormShape| {
        let mut s = l();
        s.form(WallId(0), &tool("F", shape), Point2::new(30.0, 25.0), 0.0, false)
            .map(|_| ())
    };
    let why = |shape| match form(shape) {
        Err(SheetError::Form { why, .. }) => why,
        o => panic!("{o:?}"),
    };
    assert!(matches!(
        why(dimple(-1.0, 4.0, 1.0, 1.0, 2.0)),
        FormError::Size { which: "d", .. }
    ));
    assert!(matches!(
        why(dimple(20.0, f64::NAN, 1.0, 1.0, 2.0)),
        FormError::Size { which: "depth", .. }
    ));
    assert!(matches!(
        why(dimple(20.0, 4.0, PI / 2.0, 1.0, 2.0)),
        FormError::Size { which: "angle", .. }
    ));
    assert!(matches!(
        why(dimple(20.0, 1.0, 1.0, 1.0, 2.0)),
        FormError::TooShallow { .. }
    ));
    assert!(matches!(
        why(dimple(12.0, 6.0, 1.0, 1.0, 2.0)),
        FormError::CrownTooSmall { .. }
    ));
    assert!(matches!(
        why(dimple(4.0, 6.0, 1.0, 1.0, 2.0)),
        FormError::Size { which: "d", .. }
    ));
    assert!(matches!(
        why(hole(8.0, 2.0, 1.0, 5.0)),
        FormError::TooShallow { .. }
    ));
    assert!(matches!(
        why(hole(8.0, 5.0, 1.0, 8.0)),
        FormError::Size { which: "pierce", .. }
    ));
    for (shape, kind) in [
        (
            FormShape::Louvre {
                length: 20.0,
                width: 5.0,
                height: 3.0,
            },
            forge_sheet::FormName::Louvre,
        ),
        (
            FormShape::Lance {
                length: 20.0,
                width: 5.0,
                height: 3.0,
            },
            forge_sheet::FormName::Lance,
        ),
    ] {
        assert_eq!(why(shape), FormError::Unbuilt { kind });
    }
    // not clear: into the bend's zone, over a hole, onto another form, off the wall
    let dm = tool("DM20", dimple(20.0, 4.0, 1.0, 1.0, 2.0));
    let clear = |s: &Sheet| {
        for map in [Map::Folded, Map::Flat] {
            match build_with(s, map) {
                Err(SheetError::Form {
                    why: FormError::NotClear { trim, .. },
                    ..
                }) => {
                    let _ = trim;
                }
                o => panic!("{map:?}: {:?}", o.map(|_| ())),
            }
        }
    };
    let mut s = l();
    s.form(WallId(0), &dm, Point2::new(72.0, 25.0), 0.0, false)
        .unwrap();
    clear(&s);
    let mut s = l();
    s.form(WallId(0), &dm, Point2::new(30.0, 25.0), 0.0, false)
        .unwrap();
    let c = Segment::Circle {
        center: Point2::new(30.0, 25.0),
        radius: 2.0,
        ccw: true,
    };
    s.cut(WallId(0), ProfileLoop::new(vec![(c, 0)])).unwrap();
    clear(&s);
    let mut s = l();
    s.form(WallId(0), &dm, Point2::new(30.0, 25.0), 0.0, false)
        .unwrap();
    s.form(WallId(0), &dm, Point2::new(45.0, 25.0), 0.0, true)
        .unwrap();
    clear(&s);
    let mut s = l();
    s.form(WallId(0), &dm, Point2::new(-30.0, 25.0), 0.0, false)
        .unwrap();
    clear(&s);
}

/// A base 80 x 50 with a flange folded back over it (150 degrees) off its right edge.
fn folded_back(height: f64) -> Sheet {
    let (mut s, base) = Sheet::face(FeatureId(5), gauge(), Frame::WORLD, &[rect(80.0, 50.0)]).unwrap();
    s.flange(base, 1, 150.0 * PI / 180.0, height).unwrap();
    s
}

#[test]
fn the_folded_clash_check_sees_forms() {
    let dm = tool("DM", dimple(20.0, 12.0, 1.3, 0.5, 0.5));
    // pressed down, away from the flange: clear; up, into it: a clash on the folded part, a
    // warning on the flat
    let mut s = folded_back(40.0);
    s.form(WallId(0), &dm, Point2::new(56.0, 25.0), 0.0, false)
        .unwrap();
    build_with(&s, Map::Folded).unwrap();
    let mut s = folded_back(40.0);
    s.form(WallId(0), &dm, Point2::new(56.0, 25.0), 0.0, true)
        .unwrap();
    match build_with(&s, Map::Folded) {
        Err(SheetError::FormClash {
            form: 0,
            with,
            depth,
            limit,
            ..
        }) => {
            assert!(depth > limit);
            assert!(matches!(with, FormAgainst::Strip(_)), "{with:?}");
        }
        o => panic!("{:?}", o.map(|_| ())),
    }
    let flat = build_with(&s, Map::Flat).unwrap();
    assert!(
        flat.warnings
            .iter()
            .any(|w| matches!(w, SheetWarning::FormClash { form: 0, .. }))
    );
    // a short flange stays clear of it
    let mut s = folded_back(8.0);
    s.form(WallId(0), &dm, Point2::new(30.0, 25.0), 0.0, true)
        .unwrap();
    build_with(&s, Map::Folded).unwrap();
    // a U: dimples on both flanges pressed in towards each other meet in the middle
    let (mut u, base) = Sheet::face(FeatureId(5), gauge(), Frame::WORLD, &[rect(30.0, 60.0)]).unwrap();
    u.flange(base, 1, PI / 2.0, 40.0).unwrap();
    u.flange(base, 3, PI / 2.0, 40.0).unwrap();
    let deep = tool("DD", dimple(28.0, 17.0, 1.3, 1.0, 2.0));
    u.form(WallId(1), &deep, Point2::new(22.0, 30.0), 0.0, true)
        .unwrap();
    u.form(WallId(2), &deep, Point2::new(22.0, 30.0), 0.0, true)
        .unwrap();
    match build_with(&u, Map::Folded) {
        Err(SheetError::FormClash { with, .. }) => assert!(matches!(with, FormAgainst::Form(_)), "{with:?}"),
        o => panic!("{:?}", o.map(|_| ())),
    }
}

#[test]
fn fuzz_forms() {
    let mut rng = Rng::new(0xf0f0);
    let (mut built, mut refused) = (0, 0);
    let plain = l();
    let (f0, l0) = (
        vol(&build_with(&plain, Map::Folded).unwrap().body).0,
        vol(&build_with(&plain, Map::Flat).unwrap().body).0,
    );
    for _ in 0..300 {
        let shape = if rng.unit() < 0.6 {
            dimple(
                rng.range(12.0, 30.0),
                rng.range(1.0, 8.0),
                rng.range(0.3, 1.4),
                rng.range(0.3, 2.0),
                rng.range(0.3, 2.0),
            )
        } else {
            let d = rng.range(3.0, 16.0);
            hole(
                d,
                rng.range(1.0, 9.0),
                rng.range(0.3, 3.0),
                rng.range(0.5, 1.1) * d,
            )
        };
        let at = Point2::new(rng.range(5.0, 70.0), rng.range(5.0, 45.0));
        let (angle, up) = (rng.range(-PI, PI), rng.unit() < 0.5);
        let mut s = l();
        match s.form(WallId(0), &tool("F", shape), at, angle, up) {
            Err(SheetError::Form { .. }) => {
                refused += 1;
                continue;
            }
            Err(e) => panic!("{e:?}"),
            Ok(_) => {}
        }
        match (build_with(&s, Map::Folded), build_with(&s, Map::Flat)) {
            (Ok(f), Ok(fl)) => {
                checked(&f.body);
                checked(&fl.body);
                let (df, dl) = closed_form(shape, T);
                let ((vf, ef), (vl, el)) = (vol(&f.body), vol(&fl.body));
                assert!(
                    (vf - f0 - df).abs() <= 1e-9 * f0 + 3.0 * ef,
                    "{shape:?} {at:?} up {up}"
                );
                assert!((vl - l0 - dl).abs() <= 1e-9 * l0 + 3.0 * el, "{shape:?} {at:?}");
                built += 1;
            }
            (Err(SheetError::Form { .. }), Err(SheetError::Form { .. })) => refused += 1,
            (a, b) => panic!(
                "{shape:?} {at:?} up {up}: {:?} / {:?}",
                a.map(|_| ()),
                b.map(|_| ())
            ),
        }
    }
    println!("{built} built, {refused} refused");
    assert!(built >= 100, "{built} built, {refused} refused");
}

#[test]
fn timing() {
    let mut s = l();
    let dm = tool("DM", dimple(14.0, 3.0, 1.2, 0.5, 1.0));
    let eh = tool("EH", hole(6.0, 4.0, 1.0, 4.0));
    for i in 0..4 {
        for j in 0..2 {
            let t = if (i + j) % 2 == 0 { &dm } else { &eh };
            s.form(
                WallId(0),
                t,
                Point2::new(12.0 + 16.0 * i as f64, 14.0 + 22.0 * j as f64),
                0.0,
                i % 2 == 0,
            )
            .unwrap();
        }
    }
    let n = 30;
    let at = Instant::now();
    for _ in 0..n {
        std::hint::black_box(build_with(&s, Map::Folded).unwrap());
    }
    let fold = at.elapsed().as_secs_f64() / n as f64;
    let at = Instant::now();
    for _ in 0..n {
        std::hint::black_box(build_with(&s, Map::Flat).unwrap());
    }
    let flat = at.elapsed().as_secs_f64() / n as f64;
    let at = Instant::now();
    for _ in 0..n {
        std::hint::black_box(build_with(&l(), Map::Folded).unwrap());
    }
    println!(
        "eight forms on an L: folded {:.0} us, flat {:.0} us (the L alone {:.0} us)",
        fold * 1e6,
        flat * 1e6,
        at.elapsed().as_secs_f64() / n as f64 * 1e6
    );
}

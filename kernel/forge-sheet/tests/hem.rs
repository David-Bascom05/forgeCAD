//! Hems against closed forms: the layers' gap, the edge as the fold's outside, volume folded and
//! flat, flat length with a table's allowance, the fold/unfold round trip; rolled hems ending free;
//! a closed hem's designed contact not a clash, a teardrop's leg through the wall one; typed
//! refusals; a fuzz; timing.
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{
    BendRule, Built, FeatureId, GAP_FLOOR, Gauge, HemError, HemKind, HemName, HemRow, HemSpec, HemTable, Map,
    Sheet, SheetError, WallId, build_with,
};
use forge_topo::Body;
use std::time::Instant;

const PI: f64 = dmath::PI;

fn gauge(t: f64, r: f64, k: f64) -> Gauge {
    Gauge {
        thickness: t,
        radius: r,
        rule: BendRule::K(k),
    }
}

fn base(g: &Gauge, len: f64, w: f64) -> Sheet {
    let p = [(0.0, 0.0), (len, 0.0), (len, w), (0.0, w)];
    let lp = ProfileLoop::new(
        (0..4)
            .map(|i| {
                let (a, b) = (p[i], p[(i + 1) % 4]);
                (
                    Segment::Line {
                        a: Point2::new(a.0, a.1),
                        b: Point2::new(b.0, b.1),
                    },
                    i as u64,
                )
            })
            .collect(),
    );
    Sheet::face(FeatureId(4), g.clone(), Frame::WORLD, &[lp])
        .unwrap()
        .0
}

fn checked(b: &Body) {
    let problems = forge_topo::check::check(b);
    assert!(problems.is_empty(), "{problems:?}");
    let mesh = forge_mesh::tessellate(b, &forge_mesh::Quality::default()).expect("a mesh");
    assert!(mesh.open_edges().is_empty(), "open mesh");
}

fn volume(b: &Body) -> (f64, f64) {
    let m = forge_topo::mass::mass(b).expect("mass");
    (m.volume, m.err[0])
}

fn both(s: &Sheet) -> (Built, Built) {
    let (f, l) = (
        build_with(s, Map::Folded).unwrap(),
        build_with(s, Map::Flat).unwrap(),
    );
    checked(&f.body);
    checked(&l.body);
    (f, l)
}

fn extent(b: &Body, axis: usize) -> (f64, f64) {
    let v: Vec<f64> = b
        .vertex_ids()
        .map(|i| {
            let p = b.vertex(i).point;
            [p.x, p.y, p.z][axis]
        })
        .collect();
    (
        v.iter().cloned().fold(f64::MAX, f64::min),
        v.iter().cloned().fold(f64::MIN, f64::max),
    )
}

/// The largest coordinate along `dir` the body reaches: its vertices, and each bend cylinder's
/// apex (every hem turns past a quarter, so its outer skin's apex is on the face).
fn reach(b: &Body, dir: forge_math::Vec3) -> f64 {
    let mut m = b
        .vertex_ids()
        .map(|i| (b.vertex(i).point - forge_math::Point3::ORIGIN).dot(dir))
        .fold(f64::MIN, f64::max);
    for f in b.face_ids() {
        if let forge_geom::Surface::Cylinder(c) = &b.face(f).surface {
            let along = c.frame.z().get();
            let off = dir - along * along.dot(dir);
            if off.length() > 1.0 - 1e-12 {
                m = m.max((c.frame.origin - forge_math::Point3::ORIGIN).dot(dir) + c.radius);
            }
        }
    }
    m
}

fn table(t: f64, rows: &[(HemName, f64, f64, f64)]) -> HemTable {
    HemTable {
        closed_gap: GAP_FLOOR,
        rows: rows
            .iter()
            .map(|&(kind, radius, angle, allowance)| HemRow {
                kind,
                thickness: t,
                radius,
                angle,
                allowance,
            })
            .collect(),
    }
}

/// A base `len` x `w`, a hem on its right edge: (folded, flat) volumes and the flat length.
#[allow(clippy::too_many_arguments)]
fn closed_form(
    t: f64,
    k: f64,
    len: f64,
    w: f64,
    r: f64,
    angle: f64,
    leg: f64,
    ba: Option<f64>,
) -> (f64, f64, f64) {
    let s = r + t;
    let ba = ba.unwrap_or(angle * (r + k * t));
    let straights = len - s + leg;
    (
        t * w * straights + angle * (r * t + 0.5 * t * t) * w,
        t * w * (straights + ba),
        straights + ba,
    )
}

#[test]
fn open_and_closed_hems_against_closed_forms() {
    let (t, k, len, w) = (1.5, 0.4, 50.0, 30.0);
    let g = gauge(t, 2.0, k);
    for up in [true, false] {
        for (kind, gap, ba) in [
            (HemKind::Open { gap: 1.2 }, 1.2, None),
            (HemKind::Closed, GAP_FLOOR, Some(2.1)),
        ] {
            let leg = 12.0;
            let mut s = base(&g, len, w);
            let tab = table(t, &[(HemName::Closed, 0.0, PI, 2.1)]);
            s.hem(WallId(0), 1, HemSpec { kind, leg, up }, Some(&tab))
                .unwrap();
            let (f, l) = both(&s);
            let r = 0.5 * gap;
            let (vf, vl, flat_len) = closed_form(t, k, len, w, r, PI, leg, ba);
            let ((a, ea), (b, eb)) = (volume(&f.body), volume(&l.body));
            assert!((a - vf).abs() <= ea + 1e-9 * vf, "{kind:?} {up}: {a} {vf}");
            assert!((b - vl).abs() <= eb + 1e-9 * vl, "{kind:?} {up}: {b} {vl}");
            let (x0, x1) = extent(&l.body, 0);
            assert!(
                ((x1 - x0) - flat_len).abs() <= 1e-12 * flat_len,
                "{} {flat_len}",
                x1 - x0
            );
            // the edge is the fold's outside
            let fx1 = reach(&f.body, forge_math::Vec3::X);
            assert!((fx1 - len).abs() <= 1e-12 * len, "{fx1}");
            // the leg lies `gap` off the wall, on the side it folded to
            let leg_frame = f.layout.walls[1].frame;
            let z_ref = leg_frame.origin.z;
            let want = if up { gap } else { -t - gap };
            // the leg's skin nearest the wall
            let near = if up { z_ref } else { z_ref + t };
            assert!((near - want).abs() <= 1e-12, "{kind:?} {up}: {near} {want}");
            assert!(leg_frame.z().get().z.abs() > 1.0 - 1e-15);
            round_trip(&s, &f, &l, t);
        }
    }
}

#[test]
fn teardrop_and_rolled() {
    let (t, k) = (1.0, 0.45);
    let g = gauge(t, 1.0, k);
    // a teardrop, short leg: fine; long: through the wall, a clash
    let td = HemKind::Teardrop {
        radius: 2.0,
        angle: 1.2 * PI,
    };
    let mut s = base(&g, 60.0, 20.0);
    s.hem(
        WallId(0),
        1,
        HemSpec {
            kind: td,
            leg: 3.0,
            up: true,
        },
        None,
    )
    .unwrap();
    let (f, l) = both(&s);
    let (vf, vl, _) = closed_form(t, k, 60.0, 20.0, 2.0, 1.2 * PI, 3.0, None);
    assert!((volume(&f.body).0 - vf).abs() <= volume(&f.body).1 + 1e-9 * vf);
    assert!((volume(&l.body).0 - vl).abs() <= volume(&l.body).1 + 1e-9 * vl);
    round_trip(&s, &f, &l, t);
    let mut s = base(&g, 60.0, 20.0);
    s.hem(
        WallId(0),
        1,
        HemSpec {
            kind: td,
            leg: 30.0,
            up: true,
        },
        None,
    )
    .unwrap();
    assert!(matches!(
        build_with(&s, Map::Folded),
        Err(SheetError::FoldedClash { .. })
    ));
    // a rolled hem of three quarters of a turn ending free, and one with a leg
    for leg in [0.0, 1.0] {
        let rl = HemKind::Rolled {
            radius: 3.0,
            angle: 1.5 * PI,
        };
        let mut s = base(&g, 60.0, 20.0);
        s.hem(
            WallId(0),
            1,
            HemSpec {
                kind: rl,
                leg,
                up: true,
            },
            None,
        )
        .unwrap();
        let (f, l) = both(&s);
        let (vf, vl, flat_len) = closed_form(t, k, 60.0, 20.0, 3.0, 1.5 * PI, leg, None);
        assert!(
            (volume(&f.body).0 - vf).abs() <= volume(&f.body).1 + 1e-9 * vf,
            "{leg}"
        );
        assert!(
            (volume(&l.body).0 - vl).abs() <= volume(&l.body).1 + 1e-9 * vl,
            "{leg}"
        );
        let (x0, x1) = extent(&l.body, 0);
        assert!(((x1 - x0) - flat_len).abs() <= 1e-12 * flat_len);
        // folded, round to the edge
        let fx1 = reach(&f.body, forge_math::Vec3::X);
        assert!((fx1 - 60.0).abs() <= 1e-12 * 60.0, "{fx1}");
        assert_eq!(f.body.face_ids().count(), if leg == 0.0 { 8 } else { 10 });
    }
}

#[test]
fn a_hem_on_a_flange_tip() {
    let (t, k) = (2.0, 0.4);
    let g = gauge(t, 2.0, k);
    let mut s = base(&g, 40.0, 30.0);
    let (up, _) = s.flange(WallId(0), 1, PI / 2.0, 30.0).unwrap();
    s.hem(
        up,
        1,
        HemSpec {
            kind: HemKind::Open { gap: 1.0 },
            leg: 10.0,
            up: false,
        },
        None,
    )
    .unwrap();
    let (f, l) = both(&s);
    // the flange keeps its height: the hem's outside at its top
    let z1 = reach(&f.body, forge_math::Vec3::Z);
    assert!((z1 - 30.0).abs() <= 1e-12 * 30.0, "{z1}");
    round_trip(&s, &f, &l, t);
}

#[test]
fn a_hem_stood_outside_its_edge() {
    let (t, k) = (1.5, 0.4);
    let g = gauge(t, 2.0, k);
    let spec = HemSpec {
        kind: HemKind::Open { gap: 1.0 },
        leg: 8.0,
        up: true,
    };
    let mut inside = base(&g, 50.0, 30.0);
    inside.hem_at(WallId(0), 1, spec, None, false).unwrap();
    let mut outside = base(&g, 50.0, 30.0);
    outside.hem_at(WallId(0), 1, spec, None, true).unwrap();
    let (fi, li) = both(&inside);
    let (fo, lo) = both(&outside);
    // the tangent line on the old edge: the fold's outside R + t past it
    let s = 0.5 + t;
    assert!((reach(&fo.body, forge_math::Vec3::X) - (50.0 + s)).abs() <= 1e-12 * 60.0);
    assert!((reach(&fi.body, forge_math::Vec3::X) - 50.0).abs() <= 1e-12 * 60.0);
    // the same hem, s more wall
    let dv = volume(&lo.body).0 - volume(&li.body).0;
    assert!((dv - s * 30.0 * t).abs() <= 1e-9, "{dv}");
    round_trip(&outside, &fo, &lo, t);
}

#[test]
fn refusals_are_typed() {
    let g = gauge(1.0, 1.0, 0.4);
    let hem = |kind, leg, tab: Option<&HemTable>| {
        let mut s = base(&g, 40.0, 20.0);
        let r = s.hem(WallId(0), 1, HemSpec { kind, leg, up: true }, tab);
        (r, s.walls.len())
    };
    let why = |r: Result<(WallId, forge_sheet::JointId), SheetError>| match r {
        Err(SheetError::Hem { feature, why, .. }) => {
            assert_eq!(feature, FeatureId(4));
            why
        }
        o => panic!("{o:?}"),
    };
    let (r, n) = hem(HemKind::Open { gap: 0.01 }, 5.0, None);
    assert_eq!(
        why(r),
        HemError::Gap {
            gap: 0.01,
            limit: GAP_FLOOR
        }
    );
    assert_eq!(n, 1, "a refusal adds nothing");
    assert!(matches!(
        why(hem(HemKind::Closed, 5.0, None).0),
        HemError::NotTabled {
            kind: HemName::Closed,
            ..
        }
    ));
    assert!(matches!(
        why(hem(
            HemKind::Teardrop {
                radius: 1.0,
                angle: 0.9 * PI
            },
            5.0,
            None
        )
        .0),
        HemError::Angle {
            kind: HemName::Teardrop,
            ..
        }
    ));
    assert!(matches!(
        why(hem(HemKind::Open { gap: 1.0 }, 0.0, None).0),
        HemError::Leg {
            kind: HemName::Open,
            ..
        }
    ));
    assert!(matches!(
        why(hem(
            HemKind::Rolled {
                radius: 50.0,
                angle: PI
            },
            0.0,
            None
        )
        .0),
        HemError::NoRoom { .. }
    ));
    // a closed hem's gap from the table, never under the floor
    let tab = HemTable {
        closed_gap: 0.001,
        rows: vec![],
    };
    assert!(matches!(
        why(hem(HemKind::Closed, 5.0, Some(&tab)).0),
        HemError::Gap { .. }
    ));
}

/// Each vertex, read in the frame of a wall it lies on, at the same wall coordinates folded and
/// flat.
fn round_trip(sheet: &Sheet, fold: &Built, flat: &Built, t: f64) {
    let mut worst = 0.0f64;
    let mut seen = 0;
    for v in fold.body.vertex_ids() {
        let name = fold.body.vertex(v).name;
        let p = fold.body.vertex(v).point;
        let q = flat
            .body
            .vertex_ids()
            .find(|&u| flat.body.vertex(u).name == name)
            .map(|u| flat.body.vertex(u).point)
            .expect("the same names");
        for (wf, wl) in fold.layout.walls.iter().zip(&flat.layout.walls) {
            let a = wf.frame.point_to_local(p);
            if a.z.abs() > 1e-9 && (a.z + t).abs() > 1e-9 {
                continue;
            }
            let wall = sheet.wall(wf.wall).expect("its wall");
            let pts: Vec<Point2> = wall.outline[0]
                .segments
                .iter()
                .flat_map(|s| s.0.samples(2))
                .collect();
            let lo = pts.iter().fold(Point2::new(f64::MAX, f64::MAX), |m, q| {
                Point2::new(m.x.min(q.x), m.y.min(q.y))
            });
            let hi = pts.iter().fold(Point2::new(f64::MIN, f64::MIN), |m, q| {
                Point2::new(m.x.max(q.x), m.y.max(q.y))
            });
            if a.x < lo.x - 1e-9 || a.x > hi.x + 1e-9 || a.y < lo.y - 1e-9 || a.y > hi.y + 1e-9 {
                continue;
            }
            let b = wl.frame.point_to_local(q);
            worst = worst.max(a.distance(b) / a.x.abs().max(a.y.abs()).max(1.0));
            seen += 1;
        }
    }
    assert!(seen > 0);
    assert!(worst < 1e-12, "round trip {worst}");
}

#[test]
fn fuzz_hems() {
    let mut rng = Rng::new(0x04e3);
    let (mut built, mut refused, mut clashes) = (0, 0, 0);
    let mut slowest = 0.0f64;
    for seed in 0..200 {
        let t = rng.range(0.5, 2.5);
        let k = rng.range(0.3, 0.5);
        let g = gauge(t, rng.range(0.5, 2.0) * t, k);
        let mut s = base(&g, rng.range(30.0, 80.0), rng.range(10.0, 40.0));
        let mut wall = WallId(0);
        if rng.bool() {
            let a = rng.range(0.3, 2.8) * if rng.bool() { 1.0 } else { -1.0 };
            match s.flange(WallId(0), 1, a, rng.range(15.0, 40.0)) {
                Ok((w, _)) => wall = w,
                Err(_) => continue,
            }
        }
        let kind = match rng.below(4) {
            0 => HemKind::Closed,
            1 => HemKind::Open {
                gap: rng.range(0.05, 3.0),
            },
            2 => HemKind::Teardrop {
                radius: rng.range(0.3, 3.0) * t,
                angle: rng.range(1.05, 1.6) * PI,
            },
            _ => HemKind::Rolled {
                radius: rng.range(0.5, 4.0) * t,
                angle: rng.range(0.6, 1.9) * PI,
            },
        };
        let leg = if matches!(kind, HemKind::Rolled { .. }) && rng.bool() {
            0.0
        } else {
            rng.range(2.0, 15.0)
        };
        let tab = table(t, &[(HemName::Closed, 0.0, PI, PI * (GAP_FLOOR / 2.0 + 0.3 * t))]);
        match s.hem(
            wall,
            1,
            HemSpec {
                kind,
                leg,
                up: rng.bool(),
            },
            Some(&tab),
        ) {
            Ok(_) => {}
            Err(SheetError::Hem {
                why: HemError::NoRoom { .. },
                ..
            }) => {
                refused += 1;
                continue;
            }
            Err(e) => panic!("seed {seed}: {e:?}"),
        }
        let at = Instant::now();
        let f = match build_with(&s, Map::Folded) {
            Ok(f) => f,
            Err(SheetError::FoldedClash { .. }) => {
                clashes += 1;
                continue;
            }
            Err(e) => panic!("seed {seed} {kind:?}: {e:?}"),
        };
        let l = build_with(&s, Map::Flat).unwrap_or_else(|e| panic!("seed {seed}: {e:?}"));
        slowest = slowest.max(at.elapsed().as_secs_f64());
        let ok = |b: &Body| forge_topo::check::check(b).is_empty();
        assert!(
            ok(&f.body) && ok(&l.body),
            "seed {seed} {kind:?} leg {leg} on {wall:?}: {:?}",
            s.joints
        );
        checked(&f.body);
        checked(&l.body);
        round_trip(&s, &f, &l, t);
        built += 1;
    }
    println!(
        "{built} built, {refused} refused, {clashes} clashes; slowest {:.0} us",
        slowest * 1e6
    );
    assert!(built > 100 && clashes > 0);
}

#[test]
fn timing() {
    let g = gauge(1.5, 2.0, 0.4);
    let mut s = base(&g, 50.0, 30.0);
    s.hem(
        WallId(0),
        1,
        HemSpec {
            kind: HemKind::Open { gap: 1.0 },
            leg: 10.0,
            up: true,
        },
        None,
    )
    .unwrap();
    let n = 200;
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
    println!("open hem: folded {:.1} us, flat {:.1} us", fold * 1e6, flat * 1e6);
}

//! Contour flanges against closed forms: the folded reference skin on the profile itself, flat
//! length (lines + allowances, less nothing: the profile is tangent to tangent), volume both ways,
//! the fold/unfold round trip through the walls' frames, a contour off an edge identical to the
//! flanges it stands for, typed refusals, a fuzz on random profiles, and timing.
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, Vec2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::contour::ContourError;
use forge_sheet::{BendRule, FeatureId, Gauge, Hand, Map, Sheet, SheetError, WallId, build, build_with};
use forge_topo::Body;
use std::time::Instant;

fn gauge(t: f64, r: f64, k: f64) -> Gauge {
    Gauge {
        thickness: t,
        radius: r,
        rule: BendRule::K(k),
    }
}

fn line(a: (f64, f64), b: (f64, f64)) -> Segment {
    Segment::Line {
        a: Point2::new(a.0, a.1),
        b: Point2::new(b.0, b.1),
    }
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

/// A profile from a start point and moves: (length of the straight, then turn and radius of the
/// arc or sharp corner after it). Radius None is a sharp corner.
fn profile(start: Point2, dir: f64, moves: &[(f64, f64, Option<f64>)]) -> Vec<Segment> {
    let mut out = vec![];
    let (mut p, mut a) = (start, dir);
    for (i, &(len, th, r)) in moves.iter().enumerate() {
        let d = Vec2::new(dmath::cos(a), dmath::sin(a));
        let q = p + d * len;
        out.push(Segment::Line { a: p, b: q });
        p = q;
        if i + 1 == moves.len() {
            break;
        }
        if let Some(r) = r {
            // the centre to the left for a left turn
            let n = if th > 0.0 {
                Vec2::new(-d.y, d.x)
            } else {
                Vec2::new(d.y, -d.x)
            };
            let c = p + n * r;
            let start = dmath::atan2(p.y - c.y, p.x - c.x);
            out.push(Segment::Arc {
                center: c,
                radius: r,
                start,
                sweep: th,
            });
            let (s, k) = dmath::sin_cos(start + th);
            p = Point2::new(c.x + r * k, c.y + r * s);
        }
        a += th;
    }
    out
}

/// Closed forms: straights, bend angles and inside radii from the moves.
struct Closed {
    flat_len: f64,
    v_fold: f64,
    v_flat: f64,
}

fn closed(g: &Gauge, k: f64, moves: &[(f64, f64, Option<f64>)], hand: Hand, w: f64) -> Closed {
    let t = g.thickness;
    let mut straights = 0.0;
    let mut zones = 0.0;
    let mut bas = 0.0;
    for (i, &(len, th, r)) in moves.iter().enumerate() {
        straights += len;
        if i + 1 == moves.len() {
            break;
        }
        let toward = (th > 0.0) == (hand == Hand::Right);
        let (inside, sharp) = match r {
            Some(r) => (if toward { r } else { r - t }, false),
            None => (g.radius, true),
        };
        if sharp {
            // a sharp corner is the mold line: both straights lose their reference setback
            let rho = if toward { inside } else { inside + t };
            straights -= 2.0 * forge_sheet::shop::setback(rho, th.abs());
        }
        bas += th.abs() * (inside + k * t);
        zones += th.abs() * (inside * t + 0.5 * t * t) * w;
    }
    Closed {
        flat_len: straights + bas,
        v_fold: t * w * straights + zones,
        v_flat: t * w * (straights + bas),
    }
}

fn hat() -> Vec<(f64, f64, Option<f64>)> {
    let q = dmath::PI / 2.0;
    vec![
        (15.0, q, Some(3.0)),
        (20.0, -q, Some(4.0)),
        (30.0, -q, None),
        (20.0, q, Some(2.5)),
        (15.0, 0.0, None),
    ]
}

#[test]
fn a_hat_against_closed_forms() {
    let (t, r, k, w) = (1.5, 2.0, 0.4, 25.0);
    for hand in [Hand::Left, Hand::Right] {
        let g = gauge(t, r, k);
        let moves = hat();
        let prof = profile(Point2::new(-10.0, 3.0), 0.3, &moves);
        let section = Frame::from_zx(
            Point3::new(1.0, 2.0, 3.0),
            forge_math::Vec3::new(0.2, -0.3, 1.0).normalize().unwrap(),
            forge_math::Vec3::new(1.0, 0.0, 0.0),
        )
        .unwrap();
        let (s, made) = Sheet::contour(FeatureId(3), g.clone(), section, &prof, w, hand, 0.0).unwrap();
        assert_eq!(made.walls.len(), 5);
        assert_eq!(made.joints.len(), 4);
        let fold = build_with(&s, Map::Folded).unwrap();
        let flat = build_with(&s, Map::Flat).unwrap();
        checked(&fold.body);
        checked(&flat.body);
        let e = closed(&g, k, &moves, hand, w);
        let ((vf, ef), (vl, el)) = (volume(&fold.body), volume(&flat.body));
        assert!(
            (vf - e.v_fold).abs() <= ef + 1e-9 * e.v_fold,
            "{hand:?} {vf} {}",
            e.v_fold
        );
        assert!((vl - e.v_flat).abs() <= el + 1e-9 * e.v_flat, "{vl} {}", e.v_flat);
        // flat length along the first wall's x, in its own frame
        let f0 = flat.layout.walls[0].frame;
        let xs: Vec<f64> = flat
            .body
            .vertex_ids()
            .map(|v| f0.point_to_local(flat.body.vertex(v).point).x)
            .collect();
        let len = xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min);
        assert!(
            (len - e.flat_len).abs() <= 1e-12 * e.flat_len,
            "{len} {}",
            e.flat_len
        );
        // every end of a profile line, at both ends of the run, is a folded vertex, except at a
        // sharp corner (there the line ends on the mold line, the virtual sharp of the bend)
        let n = section.z().get();
        let ends: Vec<Point2> = prof
            .iter()
            .enumerate()
            .flat_map(|(i, seg)| {
                let Segment::Line { a, b } = seg else {
                    return vec![];
                };
                let sharp = |j: Option<usize>| {
                    j.and_then(|j| prof.get(j))
                        .is_some_and(|x| matches!(x, Segment::Line { .. }))
                };
                let mut v = vec![];
                if !sharp(i.checked_sub(1)) {
                    v.push(*a);
                }
                if !sharp(Some(i + 1)) {
                    v.push(*b);
                }
                v
            })
            .collect();
        assert_eq!(ends.len(), 8);
        for p in &ends {
            {
                for along in [0.0, w] {
                    let q = section.point_to_world(Point3::new(p.x, p.y, 0.0)) + n * along;
                    let near = fold
                        .body
                        .vertex_ids()
                        .map(|v| fold.body.vertex(v).point.distance(q))
                        .fold(f64::MAX, f64::min);
                    assert!(near < 1e-12 * 100.0, "{hand:?}: {q:?} misses by {near}");
                }
            }
        }
        round_trip(&s, &fold, &flat, t);
    }
}

/// Each vertex, read in the frame of a wall it lies on, sits at the same wall coordinates folded
/// and flat (the names are the same both ways).
fn round_trip(sheet: &Sheet, fold: &forge_sheet::Built, flat: &forge_sheet::Built, t: f64) {
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
            // on the wall itself, not its plane carried on
            let wall = sheet.wall(wf.wall).expect("its wall");
            let pts: Vec<Point2> = wall.outline[0]
                .segments
                .iter()
                .flat_map(|s| s.0.samples(2))
                .collect();
            let (lo, hi) = pts.iter().fold(
                (Point2::new(f64::MAX, f64::MAX), Point2::new(f64::MIN, f64::MIN)),
                |(lo, hi), q| {
                    (
                        Point2::new(lo.x.min(q.x), lo.y.min(q.y)),
                        Point2::new(hi.x.max(q.x), hi.y.max(q.y)),
                    )
                },
            );
            if a.x < lo.x - 1e-9 || a.x > hi.x + 1e-9 || a.y < lo.y - 1e-9 || a.y > hi.y + 1e-9 {
                continue;
            }
            let b = wl.frame.point_to_local(q);
            if (a.z - b.z).abs() > 1e-9 {
                continue;
            }
            let scale = a.x.abs().max(a.y.abs()).max(1.0);
            worst = worst.max(a.distance(b) / scale);
            seen += 1;
        }
    }
    assert!(seen > 0);
    assert!(worst < 1e-12, "round trip {worst}");
}

#[test]
fn a_contour_off_an_edge_is_its_flanges() {
    let g = gauge(2.0, 2.0, 0.42);
    let base = || {
        let p = [(0.0, 0.0), (40.0, 0.0), (40.0, 30.0), (0.0, 30.0)];
        let lp = ProfileLoop::new(
            (0..4)
                .map(|i| {
                    let (a, b) = (p[i], p[(i + 1) % 4]);
                    (line(a, b), i as u64)
                })
                .collect(),
        );
        Sheet::face(FeatureId(1), g.clone(), Frame::WORLD, &[lp])
            .unwrap()
            .0
    };
    // up 20 sharp, out 15 over an arc r 3 (turning away from the reference side), down 10 sharp
    let q = dmath::PI / 2.0;
    let r = 3.0;
    let prof = vec![
        line((0.0, 0.0), (0.0, 20.0 - r)),
        Segment::Arc {
            center: Point2::new(r, 20.0 - r),
            radius: r,
            start: dmath::PI,
            sweep: -q,
        },
        line((r, 20.0), (15.0, 20.0)),
        line((15.0, 20.0), (15.0, 10.0)),
    ];
    let mut a = base();
    let made = a.contour_on(WallId(0), 1, &prof, 0.0).unwrap();
    assert_eq!(made.walls.len(), 3);
    let mut b = base();
    let (w1, _) = b.flange_r(WallId(0), 1, q, 20.0, 2.0).unwrap();
    // turning away from the reference side: inside radius r - t
    let (w2, _) = b.flange_r(w1, 1, -q, 15.0, r - 2.0).unwrap();
    b.flange_r(w2, 1, -q, 10.0, 2.0).unwrap();
    assert_eq!(a, b);
    for map in [Map::Folded, Map::Flat] {
        let (x, y) = (build(&a, map).unwrap(), build(&b, map).unwrap());
        checked(&x);
        let px: Vec<_> = x.vertex_ids().map(|v| x.vertex(v).point).collect();
        let py: Vec<_> = y.vertex_ids().map(|v| y.vertex(v).point).collect();
        assert_eq!(px, py);
    }
}

#[test]
fn refusals_are_typed() {
    let g = gauge(2.0, 2.0, 0.4);
    let q = dmath::PI / 2.0;
    let c = |prof: &[Segment], hand, min| {
        Sheet::contour(FeatureId(5), g.clone(), Frame::WORLD, prof, 20.0, hand, min)
    };
    let why = |r: Result<(Sheet, forge_sheet::ContourMade), SheetError>| match r {
        Err(SheetError::Contour { feature, why }) => {
            assert_eq!(feature, FeatureId(5));
            why
        }
        o => panic!("{o:?}"),
    };
    // an arc of r 1.5 turning away from the reference side: inside radius -0.5
    let p = profile(Point2::ORIGIN, 0.0, &[(10.0, q, Some(1.5)), (10.0, 0.0, None)]);
    assert!(matches!(
        why(c(&p, Hand::Left, 0.0)),
        ContourError::RadiusBelowMin { piece: 1, .. }
    ));
    // the same arc the other way round is fine, but not below the shop's least radius
    assert!(c(&p, Hand::Right, 0.0).is_ok());
    assert!(matches!(
        why(c(&p, Hand::Right, 2.0)),
        ContourError::RadiusBelowMin { piece: 1, radius, limit } if radius == 1.5 && limit == 2.0
    ));
    // a line too short for the setbacks either side
    let p = profile(
        Point2::ORIGIN,
        0.0,
        &[(10.0, q, None), (2.0, q, None), (10.0, 0.0, None)],
    );
    assert!(matches!(
        why(c(&p, Hand::Right, 0.0)),
        ContourError::WallTooShort { piece: 1, .. }
    ));
    // a spline
    let sp = forge_geom::spline::PlaneSpline::control(
        2,
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(5.0, 5.0),
            Point2::new(10.0, 0.0),
        ],
        None,
        None,
    )
    .unwrap();
    let p = vec![Segment::Spline(std::sync::Arc::new(sp))];
    assert_eq!(why(c(&p, Hand::Right, 0.0)), ContourError::Spline { piece: 0 });
    // a turn back on itself
    let p = vec![line((0.0, 0.0), (10.0, 0.0)), line((10.0, 0.0), (0.0, 0.0))];
    assert!(matches!(
        why(c(&p, Hand::Right, 0.0)),
        ContourError::Turn { piece: 1, .. }
    ));
    // run off an edge: the start must be on the edge, and a bend there
    let (mut s, w) = Sheet::face(
        FeatureId(5),
        g.clone(),
        Frame::WORLD,
        &[ProfileLoop::new(vec![
            (line((0.0, 0.0), (30.0, 0.0)), 0),
            (line((30.0, 0.0), (30.0, 20.0)), 1),
            (line((30.0, 20.0), (0.0, 20.0)), 2),
            (line((0.0, 20.0), (0.0, 0.0)), 3),
        ])],
    )
    .unwrap();
    assert!(matches!(
        s.contour_on(w, 1, &[line((0.5, 0.0), (0.5, 10.0))], 0.0),
        Err(SheetError::Contour {
            why: ContourError::StartOffEdge { .. },
            ..
        })
    ));
    assert!(matches!(
        s.contour_on(w, 1, &[line((0.0, 0.0), (10.0, 0.0))], 0.0),
        Err(SheetError::Contour {
            why: ContourError::Coplanar { piece: 0 },
            ..
        })
    ));
    // the sheet is untouched by a refusal
    assert_eq!(s.walls.len(), 1);
}

#[test]
fn fuzz_random_profiles() {
    let mut rng = Rng::new(0xc0_7012);
    let (mut built, mut refused, mut clashes) = (0, 0, 0);
    let mut worst_time = 0.0f64;
    for seed in 0..250 {
        let t = rng.range(0.5, 3.0);
        let k = rng.range(0.25, 0.5);
        let g = gauge(t, rng.range(0.5, 2.0) * t, k);
        let n = 2 + rng.below(4);
        let mut moves = vec![];
        for i in 0..n {
            let len = rng.range(8.0, 40.0);
            let th = if i + 1 == n {
                0.0
            } else {
                rng.range(0.15, 2.8) * if rng.bool() { 1.0 } else { -1.0 }
            };
            let r = if rng.bool() {
                Some(rng.range(1.2, 4.0) * t)
            } else {
                None
            };
            moves.push((len, th, r));
        }
        let hand = if rng.bool() { Hand::Left } else { Hand::Right };
        let w = rng.range(5.0, 40.0);
        let prof = profile(
            Point2::new(rng.range(-5.0, 5.0), 0.0),
            rng.range(-3.0, 3.0),
            &moves,
        );
        let s = match Sheet::contour(FeatureId(9), g.clone(), Frame::WORLD, &prof, w, hand, 0.0) {
            Ok((s, _)) => s,
            Err(SheetError::Contour {
                why: ContourError::WallTooShort { .. } | ContourError::RadiusBelowMin { .. },
                ..
            })
            | Err(SheetError::WallTooShort { .. }) => {
                refused += 1;
                continue;
            }
            Err(e) => panic!("seed {seed}: {e:?}"),
        };
        let at = Instant::now();
        let fold = match build_with(&s, Map::Folded) {
            Ok(b) => b,
            Err(SheetError::FoldedClash { .. }) => {
                clashes += 1;
                continue;
            }
            Err(e) => panic!("seed {seed}: {e:?}"),
        };
        let flat = build_with(&s, Map::Flat).unwrap();
        worst_time = worst_time.max(at.elapsed().as_secs_f64());
        checked(&fold.body);
        checked(&flat.body);
        let e = closed(&g, k, &moves, hand, w);
        let ((vf, ef), (vl, el)) = (volume(&fold.body), volume(&flat.body));
        assert!(
            (vf - e.v_fold).abs() <= ef + 1e-9 * e.v_fold,
            "seed {seed}: {vf} {}",
            e.v_fold
        );
        assert!(
            (vl - e.v_flat).abs() <= el + 1e-9 * e.v_flat,
            "seed {seed}: {vl} {}",
            e.v_flat
        );
        round_trip(&s, &fold, &flat, t);
        built += 1;
    }
    println!(
        "{built} built, {refused} refused, {clashes} clash; slowest fold+flat {:.0} us",
        worst_time * 1e6
    );
    assert!(built > 100 && clashes > 0, "{built} built, {clashes} clashes");
}

#[test]
fn timing() {
    let g = gauge(1.5, 2.0, 0.4);
    let prof = profile(Point2::ORIGIN, 0.0, &hat());
    let n = 200;
    let at = Instant::now();
    for _ in 0..n {
        let (s, _) = Sheet::contour(
            FeatureId(1),
            g.clone(),
            Frame::WORLD,
            &prof,
            25.0,
            Hand::Right,
            0.0,
        )
        .unwrap();
        std::hint::black_box(&s);
    }
    let graph = at.elapsed().as_secs_f64() / n as f64;
    let (s, _) = Sheet::contour(FeatureId(1), g, Frame::WORLD, &prof, 25.0, Hand::Right, 0.0).unwrap();
    let at = Instant::now();
    for _ in 0..n {
        std::hint::black_box(build(&s, Map::Folded).unwrap());
    }
    let fold = at.elapsed().as_secs_f64() / n as f64;
    println!(
        "hat contour: graph {:.1} us, folded build {:.1} us",
        graph * 1e6,
        fold * 1e6
    );
}

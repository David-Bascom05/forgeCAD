//! The audit's probes (research/kernel-audit/audit2.md §2.1, §2.2, §2.6): parts that failed or
//! took the wrong construction before, each held to what it should build.
use forge_geom::Surface;
use forge_math::{Frame, Point2, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{BendRule, FeatureId, Gauge, Map, Sheet, WallId, build_with};

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

fn gauge() -> Gauge {
    Gauge {
        thickness: 2.0,
        radius: 2.0,
        rule: BendRule::K(0.4),
    }
}

#[test]
fn a_slot_a_rounding_off_square_to_its_bend_is_square_to_it() {
    // rank 11: a slot across the bend whose lower side is 1e-12 off square: its side faces in the
    // zone are planes square to the axis and its edges circles, as for an exact one, not a
    // ruled surface between wrapped lines
    for off in [0.0, 1e-12, 3e-9] {
        let (mut s, base) = Sheet::face(
            FeatureId(1),
            gauge(),
            Frame::WORLD,
            &[poly(&[(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)])],
        )
        .unwrap();
        s.flange(base, 1, dmath::PI / 2.0, 30.0).unwrap();
        s.cut(
            WallId(0),
            poly(&[(50.0, 15.0), (70.0, 15.0 + off), (70.0, 25.0), (50.0, 25.0)]),
        )
        .unwrap();
        let b = build_with(&s, Map::Folded).expect("builds");
        assert!(forge_topo::check::check(&b.body).is_empty(), "off {off}");
        let ruled = b
            .body
            .faces
            .iter()
            .filter(|f| matches!(f.surface, Surface::Procedural(_)))
            .count();
        assert_eq!(ruled, 0, "off {off}: no ruled side faces");
    }
}

use forge_math::Vec2;
use forge_sheet::{CornerKind, EndCondition, Placement};

/// The audit's pentagon: an interior angle `deg` at (60, 0), both far ends square to their edges.
fn pentagon(deg: f64) -> Vec<(f64, f64)> {
    let a = (180.0 - deg).to_radians();
    let v1 = Point2::new(60.0, 0.0);
    let d1 = Vec2::new(dmath::cos(a), dmath::sin(a));
    let v2 = v1 + d1 * 40.0;
    let v3 = v2 + Vec2::new(-d1.y, d1.x) * 15.0;
    vec![(0.0, 0.0), (60.0, 0.0), (v2.x, v2.y), (v3.x, v3.y), (0.0, 45.0)]
}

/// The audit's quad: an interior angle `deg` at (60, 0), the far end of its second edge slanted.
fn quad(deg: f64) -> Vec<(f64, f64)> {
    let a = (180.0 - deg).to_radians();
    let v2 = Point2::new(60.0, 0.0) + Vec2::new(dmath::cos(a), dmath::sin(a)) * 40.0;
    vec![(0.0, 0.0), (60.0, 0.0), (v2.x, v2.y), (0.0, v2.y)]
}

/// Flanges on edges 0 and 1 (or one of them), up or down, with the corner kind asked for: each
/// map's outcome, "ok" for a valid body.
fn probe(pts: &[(f64, f64)], shape: &str, angle: f64, ends: EndCondition) -> [String; 2] {
    let (mut s, base) = Sheet::face(FeatureId(1), gauge(), Frame::WORLD, &[poly(pts)]).unwrap();
    s.bend_ends = ends;
    s.corner_default = match shape {
        "circ" => Some(CornerKind::Circular {
            size: None,
            placement: Placement::Tangent,
        }),
        "sq" => Some(CornerKind::Square {
            size: None,
            placement: Placement::Tangent,
        }),
        "tear" => Some(CornerKind::Tear),
        "trim" => Some(CornerKind::TrimToBend),
        "open" => Some(CornerKind::Open { gap: 0.5 }),
        _ => None,
    };
    if shape != "one1" {
        s.flange(base, 0, angle, 20.0).unwrap();
    }
    if shape != "one0" {
        s.flange(base, 1, angle, 20.0).unwrap();
    }
    [Map::Folded, Map::Flat].map(|m| match build_with(&s, m) {
        Ok(b) => {
            let p = forge_topo::check::check(&b.body);
            if p.is_empty() {
                "ok".to_string()
            } else {
                format!("invalid {p:?}")
            }
        }
        Err(e) => format!("{e:?}"),
    })
}

fn sweep(
    name: &str,
    family: fn(f64) -> Vec<(f64, f64)>,
    angles: &[f64],
    kinds: &[&str],
    ends: EndCondition,
) -> Vec<String> {
    let mut bad = vec![];
    for &deg in angles {
        for up in [true, false] {
            for &shape in kinds {
                let angle = if up { dmath::PI / 2.0 } else { -dmath::PI / 2.0 };
                let [f, l] = probe(&family(deg), shape, angle, ends);
                if f != "ok" || l != "ok" {
                    let cut = |x: &str| x.chars().take(140).collect::<String>();
                    bad.push(format!(
                        "{name} {ends:?} {deg} {} {shape}: {} | {}",
                        if up { "up" } else { "down" },
                        cut(&f),
                        cut(&l)
                    ));
                }
            }
        }
    }
    bad
}

const KINDS: [&str; 8] = ["one0", "one1", "auto", "circ", "sq", "tear", "trim", "open"];

const ANGLES: [f64; 8] = [60.0, 75.0, 85.0, 90.0, 95.0, 100.0, 120.0, 135.0];

#[test]
fn flanges_beside_corners_off_square_build() {
    // rank 1: a flange on an edge whose neighbour meets it off square, and flanges whose far ends
    // are slanted, under either end condition: each builds, folded and flat, to a valid body
    let mut bad = vec![];
    for ends in [EndCondition::Normal, EndCondition::Aligned] {
        bad.extend(sweep("pentagon", pentagon, &ANGLES, &["one0", "one1"], ends));
        bad.extend(sweep(
            "quad",
            quad,
            &[90.0, 100.0, 120.0, 135.0],
            &["one1", "auto"],
            ends,
        ));
    }
    assert!(bad.is_empty(), "{} failures:\n{}", bad.len(), bad.join("\n"));
}

#[test]
fn the_two_end_conditions_differ_by_their_triangle() {
    // one flange up off the pentagon's second edge, its start beside a corner of A < 90: the
    // wall's edge leaves the tangent line s cot A along it. Normal keeps the zone's rectangle (a
    // free edge beside the wall there); aligned runs the zone's end from that point to the
    // flange's square side: a triangle of legs BA and s cot A less, on the flat
    let g = gauge();
    let (r, t, k) = (g.radius, g.thickness, 0.4);
    let ba = dmath::PI / 2.0 * (r + k * t);
    let s = r; // up: the reference skin inside, setback R tan 45
    for deg in [60.0f64, 75.0, 85.0] {
        let flat = |ends: EndCondition| {
            let (mut sh, base) =
                Sheet::face(FeatureId(1), gauge(), Frame::WORLD, &[poly(&pentagon(deg))]).unwrap();
            sh.bend_ends = ends;
            sh.flange(base, 1, dmath::PI / 2.0, 20.0).unwrap();
            let b = build_with(&sh, Map::Flat).expect("builds");
            let m = forge_topo::mass::mass(&b.body).expect("mass");
            (m.volume, m.err[0])
        };
        let ((vn, en), (va, ea)) = (flat(EndCondition::Normal), flat(EndCondition::Aligned));
        let (sn, cs) = dmath::sin_cos(deg.to_radians());
        let want = 0.5 * ba * s * cs / sn * t;
        assert!(
            ((vn - va) - want).abs() <= en + ea + 1e-9,
            "{deg}: {} against {want}",
            vn - va
        );
    }
}

#[test]
fn corners_off_square_build_at_every_kind_and_sense() {
    // rank 2: both flanges at a corner of A, up or down, every corner kind: folded and flat,
    // valid (the corner built in the true frame of the two edges, not as if they were square)
    let mut bad = sweep("pentagon", pentagon, &ANGLES, &KINDS, EndCondition::Normal);
    bad.extend(sweep(
        "quad",
        quad,
        &[90.0, 100.0, 120.0, 135.0],
        &KINDS,
        EndCondition::Normal,
    ));
    assert!(bad.is_empty(), "{} failures:\n{}", bad.len(), bad.join("\n"));
}

#[test]
fn the_probes_refold() {
    // §2.0's refold check over the families: wherever both maps build, the flat refolds to the
    // folded body and every condition holds
    let mut bad = vec![];
    let mut checked = 0;
    for (name, family, angles) in [
        ("pentagon", pentagon as fn(f64) -> Vec<(f64, f64)>, &ANGLES[..]),
        ("quad", quad, &[90.0, 100.0, 120.0, 135.0][..]),
    ] {
        for &deg in angles {
            for angle in [dmath::PI / 2.0, -dmath::PI / 2.0, dmath::PI / 3.0] {
                for shape in KINDS {
                    for ends in [EndCondition::Normal, EndCondition::Aligned] {
                        let (mut s, base) =
                            Sheet::face(FeatureId(1), gauge(), Frame::WORLD, &[poly(&family(deg))]).unwrap();
                        s.bend_ends = ends;
                        s.corner_default = match shape {
                            "tear" => Some(CornerKind::Tear),
                            "trim" => Some(CornerKind::TrimToBend),
                            _ => None,
                        };
                        if shape != "one1" {
                            s.flange(base, 0, angle, 20.0).unwrap();
                        }
                        if shape != "one0" {
                            s.flange(base, 1, angle, 20.0).unwrap();
                        }
                        match forge_refold::check(&s) {
                            Ok(_) => checked += 1,
                            Err(forge_refold::RefoldError::Build(..)) => {}
                            Err(e) => bad.push(format!("{name} {deg} {angle} {shape} {ends:?}: {e:?}")),
                        }
                    }
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{}:\n{}",
        bad.len(),
        bad[..bad.len().min(5)].join("\n")
    );
    assert!(checked > 300, "{checked}");
}

#[test]
fn the_refold_check_tells_a_wrong_flat() {
    // the checker against itself: an L refolds within far under LINEAR; its flat built with
    // another K (zones of another width) against the first's fold does not
    let l = |k: f64| {
        let g = Gauge {
            thickness: 2.0,
            radius: 2.0,
            rule: BendRule::K(k),
        };
        let (mut s, base) = Sheet::face(
            FeatureId(1),
            g,
            Frame::WORLD,
            &[poly(&[(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)])],
        )
        .unwrap();
        s.flange(base, 1, dmath::PI / 2.0, 30.0).unwrap();
        s.flange(base, 3, -dmath::PI / 3.0, 20.0).unwrap();
        s
    };
    let r = forge_refold::check(&l(0.4)).expect("refolds");
    assert!(r.vertices > 0 && r.bends == 2 && r.worst < 1e-10, "{r:?}");
    let (a, b) = (l(0.4), l(0.3));
    let fold = build_with(&a, Map::Folded).unwrap();
    let flat = build_with(&b, Map::Flat).unwrap();
    assert!(forge_refold::judge(&a, &fold, &flat).is_err());
}

#[test]
fn mold_lines_one_by_the_rule_or_refused() {
    // band (e): a flange whose mold line is longer than its base edge by d. Within LINEAR the two
    // far ends are one vertex, the two mold lines one segment (the base's): built, valid, and the
    // flat refolds; past it, two segments no fold joins: JointLengths
    use forge_ops::Segment;
    for (d, ok) in [
        (0.0, true),
        (0.9e-5, true),
        (-0.9e-5, true),
        (1.1e-5, false),
        (-1.1e-5, false),
    ] {
        let (mut s, base) = Sheet::face(
            FeatureId(1),
            gauge(),
            Frame::WORLD,
            &[poly(&[(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)])],
        )
        .unwrap();
        let (fw, j) = s.flange(base, 1, dmath::PI / 2.0, 20.0).unwrap();
        // the flange's mold-line segment lengthened by d along itself
        let on_b = s.joints.iter().find(|x| x.id == j).unwrap().on_b;
        let w = s.walls.iter_mut().find(|w| w.id == fw).unwrap();
        let segs = &mut w.outline[0].segments;
        let k = segs.iter().position(|x| x.1 == on_b).unwrap();
        let n = segs.len();
        let Segment::Line { a, b } = segs[k].0 else {
            panic!("a line")
        };
        let u = (b - a) * (1.0 / a.distance(b));
        let b2 = b + u * d;
        segs[k].0 = Segment::Line { a, b: b2 };
        if let Segment::Line { b: e, .. } = segs[(k + 1) % n].0 {
            segs[(k + 1) % n].0 = Segment::Line { a: b2, b: e };
        }
        match forge_refold::check(&s) {
            Ok(_) => assert!(ok, "{d}: built"),
            Err(forge_refold::RefoldError::Build(_, forge_sheet::SheetError::JointLengths { .. })) => {
                assert!(!ok, "{d}: refused")
            }
            Err(e) => panic!("{d}: {e:?}"),
        }
    }
}

/// A plate turned `rot` with a flange, and a slot crossing its bend at a slant: in the folded
/// body, the worst distance from an edge's own ends to its vertices (where the zone's edges meet
/// the walls' vertices at the crossings).
fn slanted_slot_gap(rot: f64) -> f64 {
    use forge_geom::CurveGeom;
    let (sn, cs) = dmath::sin_cos(rot);
    let turn = |x: f64, y: f64| (cs * x - sn * y, sn * x + cs * y);
    let plate: Vec<(f64, f64)> = [(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)]
        .iter()
        .map(|&(x, y)| turn(x, y))
        .collect();
    let (mut s, base) = Sheet::face(FeatureId(1), gauge(), Frame::WORLD, &[poly(&plate)]).unwrap();
    s.flange(base, 1, dmath::PI / 2.0, 30.0).unwrap();
    let slot: Vec<(f64, f64)> = [(50.0, 12.0), (75.0, 21.0), (75.0, 25.0), (50.0, 16.0)]
        .iter()
        .map(|&(x, y)| turn(x, y))
        .collect();
    s.cut(WallId(0), poly(&slot)).unwrap();
    let b = build_with(&s, Map::Folded).expect("builds");
    assert!(forge_topo::check::check(&b.body).is_empty());
    let mut worst: f64 = 0.0;
    for e in &b.body.edges {
        let (p, q) = (e.curve.eval(e.range.lo), e.curve.eval(e.range.hi));
        let (v, w) = (b.body.vertex(e.start).point, b.body.vertex(e.end).point);
        worst = worst
            .max(p.distance(v).min(p.distance(w)))
            .max(q.distance(v).min(q.distance(w)));
    }
    worst
}

#[test]
fn a_crossing_is_the_walls_in_the_zone_too() {
    // the slot's crossings of the tangent lines are decided once, in the walls, and carried to
    // the zone: the zone's edges end on the walls' vertices to the folds' rounding
    for rot in [0.3, 0.7, 1.1, 2.0] {
        let gap = slanted_slot_gap(rot);
        eprintln!("rot {rot}: worst gap {gap:e}");
        assert!(gap < 1e-12, "{rot}: {gap:e}");
    }
}

#[test]
fn a_hairpin_whose_setback_is_not_known_to_the_tolerance_is_refused() {
    // the setback rho tan(theta/2) near a half turn: its computed value's proven error bound grows
    // as (1 + cos theta)^-2; past LINEAR / 2 the bend is BendAngle, its limit the largest turn that
    // holds. R = 1 inside, up (rho = 1): the band starts about 0.0073 short of pi, where the
    // setback is about 270, so the walls are long
    let g = Gauge {
        thickness: 0.5,
        radius: 1.0,
        rule: BendRule::K(0.4),
    };
    let at = |turn: f64| {
        let (mut s, base) = Sheet::face(
            FeatureId(1),
            g.clone(),
            Frame::WORLD,
            &[poly(&[(0.0, 0.0), (2000.0, 0.0), (2000.0, 40.0), (0.0, 40.0)])],
        )
        .unwrap();
        s.flange(base, 1, turn, 1500.0).map_err(|e| format!("{e:?}"))?;
        build_with(&s, Map::Flat).map(|_| ()).map_err(|e| match e {
            forge_sheet::SheetError::BendAngle { limit, .. } => format!("limit {limit}"),
            e => format!("{e:?}"),
        })
    };
    assert_eq!(at(dmath::PI - 0.02), Ok(()));
    let r = at(dmath::PI - 0.005);
    let Err(l) = &r else { panic!("built at pi - 0.005") };
    let limit: f64 = l
        .strip_prefix("limit ")
        .unwrap_or_else(|| panic!("{l}"))
        .parse()
        .unwrap();
    assert!(limit > dmath::PI - 0.01 && limit < dmath::PI - 0.005, "{limit}");
    assert_eq!(at(limit - 1e-5), Ok(()), "the limit holds: {limit}");
}

#[test]
fn a_flange_tip_resting_on_a_bends_outer_skin_is_a_touch() {
    // the base's right flange bends down about the axis x = 96, z = -4 (inside radius 2, outer
    // skin radius 4, t = 2); a chain from the base's narrower left tongue (y in [10, 30]) climbs,
    // runs right and drops a flange at x in [97, 99] whose tip comes down onto that outer skin,
    // inside the zone's width (y in [0, 40]): its corner (97, y, -4 + sqrt 15) is on the skin. A
    // tip on a face with no gap is no valid solid: FoldedTouch. 0.5 higher it builds.
    let at = |h5: f64| {
        let (mut s, base) = Sheet::face(
            FeatureId(1),
            gauge(),
            Frame::WORLD,
            &[poly(&[
                (10.0, 0.0),
                (100.0, 0.0),
                (100.0, 40.0),
                (10.0, 40.0),
                (10.0, 30.0),
                (0.0, 30.0),
                (0.0, 10.0),
                (10.0, 10.0),
            ])],
        )
        .unwrap();
        let down = -dmath::FRAC_PI_2;
        s.flange(base, 1, down, 30.0).unwrap();
        let (f3, _) = s.flange(base, 5, -down, 40.0).unwrap();
        let (f4, _) = s.flange(f3, 1, -down, 97.0).unwrap();
        let (f5, _) = s.flange(f4, 1, -down, h5).unwrap();
        (build_with(&s, Map::Folded).map(|_| ()), f5)
    };
    let on = 44.0 - 15f64.sqrt();
    let (r, f5) = at(on);
    match r {
        Err(forge_sheet::SheetError::FoldedTouch { a, b, at, .. }) => {
            let strips = [a, b];
            assert!(strips.contains(&forge_sheet::Strip::Wall(f5)), "{a:?} {b:?}");
            assert!(
                strips.iter().any(|s| matches!(s, forge_sheet::Strip::Bend(_))),
                "{a:?} {b:?}"
            );
            // on the skin, inside the zone's width
            let d = ((at.x - 96.0).powi(2) + (at.z + 4.0).powi(2)).sqrt();
            assert!((d - 4.0).abs() < 1e-4 && at.y > 9.0 && at.y < 31.0, "{at:?}");
        }
        r => panic!("a tip on the bend's skin: {r:?}"),
    }
    assert_eq!(at(on - 0.5).0, Ok(()));
}

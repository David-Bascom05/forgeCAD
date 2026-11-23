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

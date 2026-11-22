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

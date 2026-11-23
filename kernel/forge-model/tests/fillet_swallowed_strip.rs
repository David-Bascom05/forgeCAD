//! Rounds that once failed or came out unsound: a ball resting beyond a narrow band it swallows
//! whole, at the seam vertex of a ring; and a boss's rim rounded where the round runs over the
//! boss's far end, which once left the boss's seam as a spur pinching its face.
use forge_geom::CurveGeom;
use forge_math::{Frame, Point2, Point3, Vec2, Vec3, dmath};
use forge_model::{Combine, Extent, Feature, Part, Turn};
use forge_ops::{Axis2, Profile, ProfileLoop, Radius, Segment};
use forge_topo::{Body, EdgeId, Name, check};

fn volume(b: &Body) -> f64 {
    forge_topo::mass::mass(b).expect("a closed solid").volume
}

fn fillet(names: Vec<Name>, r: f64) -> Feature {
    Feature::Fillet {
        edges: names.into_iter().map(|n| (n, Radius::Constant(r))).collect(),
        starts: Vec::new(),
        setbacks: Vec::new(),
        only: false,
        conic: None,
    }
}

/// The part before its round, regenerated, and the names of its edges `pick` accepts.
fn picked(features: &[Feature], pick: impl Fn(&Body, EdgeId) -> bool) -> (Body, Vec<Name>) {
    let body = Part {
        features: features.to_vec(),
    }
    .regenerate()
    .expect("the part before the round");
    let names: Vec<Name> = body
        .edge_ids()
        .filter(|&e| pick(&body, e))
        .map(|e| body.edge(e).name)
        .collect();
    assert!(!names.is_empty());
    (body, names)
}

/// A bowl turned from a polyline outline (radius, height), the cavity a ball of radius 55 about the
/// origin. The rim lies on the outline's band from (50, 10.5) to (55, 13.6), so close to its end that
/// a ball of 3 rests beyond the kink on the next band, swallowing the strip between: every
/// section of the round, the seam's too, lies on the band beyond.
fn bowl() -> Vec<Feature> {
    let outline = [
        (0.0, 0.0),
        (30.0, 2.5),
        (45.0, 8.0),
        (50.0, 10.5),
        (55.0, 13.6),
        (59.0, 17.0),
        (63.0, 21.0),
        (66.0, 26.0),
        (66.0, 70.0),
        (0.0, 70.0),
    ]
    .map(|(x, y)| Point2::new(x, y));
    let n = outline.len();
    let lines = (0..n)
        .map(|i| {
            (
                Segment::Line {
                    a: outline[i],
                    b: outline[(i + 1) % n],
                },
                i as u64,
            )
        })
        .collect();
    let cavity = vec![
        (
            Segment::Line {
                a: Point2::new(0.0, 55.0),
                b: Point2::new(0.0, -55.0),
            },
            0,
        ),
        (
            Segment::Arc {
                center: Point2::ORIGIN,
                radius: 55.0,
                start: -0.5 * dmath::PI,
                sweep: dmath::PI,
            },
            1,
        ),
    ];
    let turned = |loops, dir: f64, op| Feature::Revolve {
        profile: Profile {
            plane: Frame::WORLD,
            loops: vec![ProfileLoop::new(loops)],
        },
        axis: Axis2 {
            origin: Point2::ORIGIN,
            dir: Vec2::new(0.0, dir),
        },
        extent: Turn::Angle(dmath::TAU),
        op,
    };
    vec![
        turned(lines, -1.0, Combine::New),
        turned(cavity, 1.0, Combine::Cut),
    ]
}

/// The rim: circles of radius about 53.5 where the cavity meets the outline.
fn rim(b: &Body, e: EdgeId) -> bool {
    let x = b.edge(e);
    let p = x.curve.eval(x.range.mid());
    let radial = (p.x * p.x + p.z * p.z).sqrt();
    (p.to_vec().length() - 55.0).abs() < 1e-6 && (radial - 53.518).abs() < 0.01
}

#[test]
fn a_ball_resting_beyond_a_swallowed_band_rounds_the_whole_rim() {
    let mut features = bowl();
    let (before, names) = picked(&features, rim);
    features.push(fillet(names, 3.0));
    let after = Part { features }.regenerate().expect("the rim rounds");
    assert_eq!(check(&after), vec![]);
    // Pappus: the meridian region the ball cuts away (between the cavity's arc, the rim's strip,
    // the band beyond up to the tangency, and the ball's arc), 4.68723 mm2 about the axis,
    // worked out from the outline apart from the kernel: 1595.9049434 mm3
    let removed = volume(&before) - volume(&after);
    assert!((removed - 1_595.904_943_4).abs() < 1e-4, "{removed}");
}

/// A square block with a round boss leaving its +x side aslant: the boss only just clears the side
/// where the round of their edge runs over the boss's far end.
fn boss() -> Vec<Feature> {
    let block = Feature::Extrude {
        profile: Profile {
            plane: Frame::WORLD,
            loops: vec![Profile::rectangle(Point2::new(-24.0, -24.0), 48.0, 48.0, 0)],
        },
        extent: Extent::Distance(78.0),
        op: Combine::New,
    };
    let z = Vec3::new(0.86, 0.5, 0.13).normalize().expect("an axis");
    let plane =
        Frame::from_zx(Point3::new(3.0, -5.0, 38.4), z, Vec3::new(0.0, 0.25, -0.97)).expect("a frame");
    let boss = Feature::Extrude {
        profile: Profile {
            plane,
            loops: vec![Profile::circle(Point2::ORIGIN, 10.3, 0)],
        },
        extent: Extent::Distance(33.5),
        op: Combine::Add,
    };
    vec![block, boss]
}

/// The edge where the boss leaves the block's +x side.
fn boss_rim(b: &Body, e: EdgeId) -> bool {
    let x = b.edge(e);
    x.start == x.end && (x.curve.eval(x.range.mid()).x - 24.0).abs() < 1e-9
}

#[test]
fn a_round_over_a_boss_end_leaves_no_seam_spur() {
    let features = boss();
    let (before, names) = picked(&features, boss_rim);
    for r in [2.3, 0.2] {
        let mut f = features.clone();
        f.push(fillet(names.clone(), r));
        let after = Part { features: f }.regenerate().expect("the boss's rim rounds");
        assert_eq!(check(&after), vec![], "r {r}");
        let added = volume(&after) - volume(&before);
        assert!(added > 0.0, "r {r}: {added}");
    }
}

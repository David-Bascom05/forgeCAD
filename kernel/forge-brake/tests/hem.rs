//! Hems: the refold of a hem joint against the builder.
use forge_brake::model::Model;
use forge_math::{Frame, Point2, Point3, dmath};
use forge_ops::{ProfileLoop, Segment};
use forge_sheet::{BendRule, FeatureId, Gauge, HemKind, HemSpec, Map, Sheet};

fn rect(w: f64, h: f64) -> ProfileLoop {
    let pts = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
    ProfileLoop::new(
        (0..4)
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % 4]);
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

fn hemmed(kind: HemKind, up: bool) -> Sheet {
    hemmed_on(kind, up, 120.0, 60.0, 1)
}

/// A `w` x `h` plate, 1 mm, a hem on outline segment `seg`, leg 10.
fn hemmed_on(kind: HemKind, up: bool, w: f64, h: f64, seg: u64) -> Sheet {
    let (mut s, base) = Sheet::face(
        FeatureId(1),
        Gauge {
            thickness: 1.0,
            radius: 1.0,
            rule: BendRule::K(0.4),
        },
        Frame::WORLD,
        &[rect(w, h)],
    )
    .unwrap();
    s.hem(base, seg, HemSpec { kind, leg: 10.0, up }, None).unwrap();
    s
}

/// A hem joint is a bend of the model like any other: its full refold is the builder's part, to
/// the bit, folded up and down.
#[test]
fn a_hems_refold_is_the_builders_part() {
    for up in [true, false] {
        let s = hemmed(HemKind::Open { gap: 1.0 }, up);
        let m = Model::new(&s).unwrap();
        let built = forge_sheet::build_with(&s, Map::Folded).unwrap();
        assert!((m.bends[0].theta - dmath::PI).abs() < 1e-12);
        let phi: Vec<f64> = m.bends.iter().map(|b| b.theta).collect();
        let f = m.frames(&phi);
        for (wi, w) in m.walls.iter().enumerate() {
            let place = built.layout.walls.iter().find(|p| p.wall == w.id).unwrap();
            for p in [(0.0, 0.0, 0.0), (5.0, 5.0, -1.0), (3.0, 40.0, 0.0)] {
                let mine = f[wi].apply(forge_brake::geo::V3::new(p.0, p.1, p.2));
                let theirs = place.frame.point_to_world(Point3::new(p.0, p.1, p.2));
                assert_eq!(
                    [mine.x, mine.y, mine.z],
                    [theirs.x, theirs.y, theirs.z],
                    "up {up} wall {wi}"
                );
            }
        }
    }
}

/// std-eu with a flattening pair added: a test fixture (a block punch and a block die, flat
/// faces), not a maker's tools.
fn with_flattening() -> forge_brake::Tooling {
    let mut tl = forge_brake::library::std_eu();
    let p0 = tl.punches[0].clone();
    tl.punches.push(forge_brake::Punch {
        id: "test/flatten-top".into(),
        role: forge_brake::Role::Flatten,
        tip_height: 10.0,
        width: 40.0,
        profile: None,
        relief: None,
        drawn_width: None,
        ..p0
    });
    let d0 = tl.dies[0].clone();
    tl.dies.push(forge_brake::Die {
        id: "test/flatten-bottom".into(),
        role: forge_brake::Role::Flatten,
        width: 60.0,
        profile: None,
        ..d0
    });
    tl
}

/// A narrow strip with an open hem along its long edge (the base rises a quarter turn and more
/// in the pre-bend: a wide one meets the ram on short acute tools, as it would on the press).
fn strip() -> Sheet {
    hemmed_on(HemKind::Open { gap: 1.0 }, true, 120.0, 20.0, 0)
}

/// An open hem planned: its pre-bend (an air bend to 30 degrees included, on acute tools) and
/// then its flattening (on the flattening pair), both passing, in that order; the pre-bend's
/// checks are an air bend's, the flattening's its own (no V rules).
#[test]
fn an_open_hem_is_pre_bent_then_flattened() {
    use forge_brake::{Options, Rule, Stroke, plan};
    let m = Model::new(&strip()).unwrap();
    let tl = with_flattening();
    let p = plan(&m, &tl, &Options::default());
    assert!(p.feasible, "{:?}", p.blocking);
    assert_eq!(
        p.steps.len(),
        2,
        "{:?}",
        p.steps.iter().map(|s| s.stroke).collect::<Vec<_>>()
    );
    let (pre, flat) = (&p.steps[0], &p.steps[1]);
    let to = dmath::PI - 30f64.to_radians() - 1e-9;
    assert!(matches!(pre.stroke, Stroke::PreBend { to: x } if (x - to).abs() < 1e-12));
    assert!((pre.angle - to).abs() < 1e-12);
    assert!(matches!(flat.stroke, Stroke::Flatten { from } if (from - to).abs() < 1e-12));
    assert_eq!((pre.bend, flat.bend), (0, 0));
    assert_eq!(tl.punches[pre.setup.punch].kind, forge_brake::PunchKind::Acute);
    assert_eq!(flat.punch, "test/flatten-top");
    assert_eq!(flat.die, "test/flatten-bottom");
    assert!(
        pre.pass() && flat.pass(),
        "{:?} {:?}",
        pre.failures(),
        flat.failures()
    );
    assert!(pre.checks.iter().any(|c| c.rule == Rule::VTooNarrow));
    assert!(
        !flat
            .checks
            .iter()
            .any(|c| matches!(c.rule, Rule::VTooNarrow | Rule::MinFlange | Rule::DieAngle))
    );
    assert!(flat.checks.iter().any(|c| c.rule == Rule::ToolCollision));
}

/// No tool on the press flattens a hem: the pre-bend may pass, but the plan is refused, saying so.
#[test]
fn a_hem_with_no_flattening_tool_is_refused() {
    use forge_brake::{Options, Rule, plan};
    let m = Model::new(&strip()).unwrap();
    let p = plan(&m, &forge_brake::library::std_eu(), &Options::default());
    assert!(!p.feasible);
    let why: Vec<Rule> = p
        .blocking
        .iter()
        .flat_map(|b| b.checks.iter().map(|c| c.rule))
        .collect();
    let rejected: Vec<Rule> = p
        .blocking
        .iter()
        .flat_map(|b| b.attempt.rejected.iter().map(|r| r.3))
        .collect();
    assert!(
        why.contains(&Rule::NoFlattening) || rejected.contains(&Rule::NoFlattening),
        "{:?} {:?}",
        why,
        rejected
    );
}

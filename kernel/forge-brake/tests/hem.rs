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

/// The scene through a hem: every instant of both strokes draws the part; the flattening starts
/// where the pre-bend left the part (the handover joins), and ends with the hem closed on the
/// die: the part's top at two thicknesses and the gap.
#[test]
fn the_scene_draws_a_hem_through_both_strokes() {
    use forge_brake::scene::Scene;
    use forge_brake::{Options, plan};
    let m = Model::new(&strip()).unwrap();
    let tl = with_flattening();
    let p = plan(&m, &tl, &Options::default());
    assert!(p.feasible);
    let mut sc = Scene::new(&m, &tl, &p);
    for k in 0..p.steps.len() {
        for t in [0.0, 0.1, 0.2, 0.3, 0.5, 0.7, 0.8, 0.9, 1.0] {
            let bs = sc.bodies(k, t);
            assert!(bs.iter().any(|b| b.look.is_sheet()), "step {k} t {t}");
        }
    }
    let sheet_z = |bs: &[forge_brake::scene::Body]| {
        bs.iter()
            .filter(|b| b.look.is_sheet())
            .fold((f32::MAX, f32::MIN), |acc, b| {
                let (lo, hi) = b.mesh.bounds().unwrap();
                (acc.0.min(lo[2]), acc.1.max(hi[2]))
            })
    };
    // the end of the springback, then the return: the flattened hem on the die
    let (lo, hi) = sheet_z(&sc.bodies(1, 0.84));
    assert!(lo.abs() < 1e-3 && (hi - 3.0).abs() < 1e-3, "{lo} {hi}");
    // the handover: where the pre-bend leaves the part is where the flattening's handling starts
    let (a, b) = (sheet_z(&sc.bodies(0, 1.0)), sheet_z(&sc.bodies(1, 0.0)));
    assert!(
        (a.0 - b.0).abs() < 0.05 && (a.1 - b.1).abs() < 0.05,
        "{a:?} vs {b:?}"
    );
}

/// A flange standing beside the hem, under the flattening punch's block: the flattening's tool
/// check proves the flange closes on the punch, and names it (the base's side resting on the
/// flat die is not a hit); the plan refuses the hem. The same part without the flange flattens clear.
#[test]
fn a_flange_under_the_flattening_punch_is_a_proven_hit() {
    use forge_brake::sim::{Phase, Rule, Setup, StepCtx};
    use forge_brake::{Options, plan};
    let tl = with_flattening();
    let fp = tl
        .punches
        .iter()
        .position(|p| p.id == "test/flatten-top")
        .unwrap();
    let fd = tl
        .dies
        .iter()
        .position(|d| d.id == "test/flatten-bottom")
        .unwrap();
    let su = Setup {
        punch: fp,
        die: fd,
        rotate: false,
    };
    let from = dmath::PI - 30f64.to_radians();
    // the strip, a 40 high flange on its far edge (20 from the hem's fold), bent first
    let mut s = strip();
    s.flange(forge_sheet::WallId(0), 2, dmath::PI / 2.0, 40.0)
        .unwrap();
    let m = Model::new(&s).unwrap();
    let hem = m.bends.iter().position(|b| b.hem.is_some()).unwrap();
    let flange = 1 - hem;
    let mut done = vec![false; 2];
    done[flange] = true;
    let ctx = StepCtx::new(&m, &tl, &done, hem, su, 0.0)
        .flattening(from)
        .unwrap();
    let mut checks = Vec::new();
    ctx.collisions(Phase::Bend, None, &mut checks, false);
    let hit = checks
        .iter()
        .find(|c| c.rule == Rule::ClosesOnPunch && !c.pass)
        .unwrap_or_else(|| panic!("{checks:?}"));
    assert!(hit.certified && hit.value < 0.0, "{hit:?}");
    assert!(
        hit.tool
            .as_deref()
            .is_some_and(|t| t.contains("test/flatten-top")),
        "{hit:?}"
    );
    let p = plan(
        &m,
        &tl,
        &Options {
            sequence: Some(vec![flange, hem]),
            ..Options::default()
        },
    );
    // the flange stands by the acute punch in the pre-bend too: the hem is blocked, not passed
    assert!(!p.feasible);
    assert!(p.blocking.iter().any(|b| b.bend == hem && !b.checks.is_empty()));
    // without the flange: clear
    let m0 = Model::new(&strip()).unwrap();
    let ctx0 = StepCtx::new(&m0, &tl, &[false], 0, su, 0.0)
        .flattening(from)
        .unwrap();
    let mut c0 = Vec::new();
    ctx0.collisions(Phase::Bend, None, &mut c0, false);
    assert!(c0.iter().all(|c| c.pass), "{c0:?}");
}

/// A closed hem (the shop's table giving its allowance) is pre-bent and flattened, closed on the
/// die at two thicknesses and the shop's closed gap; a teardrop (past half a turn) is refused:
/// a flat face meets only its apex, and the tool that closes one is not published.
#[test]
fn a_closed_hem_flattens_and_a_teardrop_is_refused() {
    use forge_brake::{Options, Stroke, plan};
    use forge_sheet::{HemName, HemRow, HemTable};
    let table = HemTable {
        closed_gap: forge_sheet::GAP_FLOOR,
        rows: vec![HemRow {
            kind: HemName::Closed,
            thickness: 1.0,
            radius: 0.0,
            angle: dmath::PI,
            allowance: 2.0,
        }],
    };
    let (mut s, base) = Sheet::face(
        FeatureId(1),
        Gauge {
            thickness: 1.0,
            radius: 1.0,
            rule: BendRule::K(0.4),
        },
        Frame::WORLD,
        &[rect(120.0, 20.0)],
    )
    .unwrap();
    s.hem(
        base,
        0,
        HemSpec {
            kind: HemKind::Closed,
            // a closed hem's fold is tight: the 8 mm V needs a longer leg than an open hem's
            leg: 14.0,
            up: true,
        },
        Some(&table),
    )
    .unwrap();
    let m = Model::new(&s).unwrap();
    let tl = with_flattening();
    let p = plan(&m, &tl, &Options::default());
    assert!(p.feasible, "{:?}", p.blocking);
    assert!(matches!(p.steps[1].stroke, Stroke::Flatten { .. }));
    let mut sc = forge_brake::scene::Scene::new(&m, &tl, &p);
    let hi = sc
        .bodies(1, 0.84)
        .iter()
        .filter(|b| b.look.is_sheet())
        .map(|b| b.mesh.bounds().unwrap().1[2])
        .fold(f32::MIN, f32::max);
    assert!(
        (f64::from(hi) - (2.0 + forge_sheet::GAP_FLOOR)).abs() < 1e-3,
        "{hi}"
    );
    // a teardrop: pre-bent, then refused for want of a flattening that closes it
    let t = hemmed_on(
        HemKind::Teardrop {
            radius: 2.0,
            angle: 1.1 * dmath::PI,
        },
        true,
        120.0,
        20.0,
        0,
    );
    let m = Model::new(&t).unwrap();
    let p = plan(&m, &tl, &Options::default());
    assert!(!p.feasible);
    let su = forge_brake::sim::Setup {
        punch: tl
            .punches
            .iter()
            .position(|p| p.id == "test/flatten-top")
            .unwrap(),
        die: tl
            .dies
            .iter()
            .position(|d| d.id == "test/flatten-bottom")
            .unwrap(),
        rotate: false,
    };
    let from = dmath::PI - 30f64.to_radians();
    assert!(
        forge_brake::sim::StepCtx::new(&m, &tl, &[false], 0, su, 0.0)
            .flattening(from)
            .is_none(),
        "a flat face does not close a teardrop"
    );
}

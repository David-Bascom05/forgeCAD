//! What the checks refuse, and what a better tool makes certifiable.
mod common;
use forge_turn::insert::builtin;
use forge_turn::ops::{self, Chuck, Setup};
use forge_turn::profile::half_profile;

fn setup_of(body: forge_topo::Body, bar_r: f64) -> Setup {
    Setup {
        part: half_profile(&body).unwrap(),
        bar_r,
        bar_front: 1.0,
        chuck: Chuck {
            z_face: -70.0,
            grip: 30.0,
            jaw_height: 15.0,
            body_r: 80.0,
            body_len: 60.0,
        },
        fixture_clearance: 1.0,
        holder_clearance: 0.25,
        rapid_clearance: 0.2,
        tol: 0.002,
        machine: None,
        place: None,
        fixtures: vec![],
    }
}

/// A shaft of radius 10 with a 45° V-groove 3 deep between z = -20 and -14.
fn v_groove() -> Setup {
    let b = common::part(&[
        (0.0, -50.0),
        (10.0, -50.0),
        (10.0, -20.0),
        (7.0, -17.0),
        (10.0, -14.0),
        (10.0, 0.0),
        (0.0, 0.0),
    ]);
    setup_of(b, 12.0)
}

#[test]
fn a_55_degree_insert_gouges_the_falling_flank_and_a_35_degree_one_does_not() {
    let s = v_groove();
    let fin = ops::finish(&s, &builtin("od-finish-dnmg").unwrap(), 2, 0.0, -45.0).unwrap();
    let rep = ops::certify(&s, &[fin], None).unwrap();
    assert!(!rep.ok());
    // the refusal is on the part (G1), by the insert, on the falling flank
    let r = &rep.refusals[0];
    assert_eq!((r.rule.as_str(), r.body.as_str()), ("part", "insert"), "{r:?}");
    let fin = ops::finish(&s, &builtin("od-profile-vbmt").unwrap(), 3, 0.0, -45.0).unwrap();
    let rep = ops::certify(&s, &[fin], None).unwrap();
    for r in &rep.refusals {
        eprintln!("{r:?}");
    }
    assert!(rep.ok());
}

#[test]
fn a_drill_into_a_solid_shaft_is_refused() {
    let s = v_groove();
    let d = ops::drill(&s, &builtin("drill-8").unwrap(), 6, -10.0, 3.0).unwrap();
    let rep = ops::certify(&s, &[d], None).unwrap();
    assert!(
        rep.refusals
            .iter()
            .any(|r| r.rule == "part" && r.verdict.starts_with("Hit")),
        "{:?}",
        rep.refusals
    );
}

#[test]
fn a_bored_part_is_drilled() {
    // a bush: bore radius 4.5 through, OD 10
    let b = common::part(&[(4.5, -30.0), (10.0, -30.0), (10.0, 0.0), (4.5, 0.0)]);
    let s = setup_of(b, 11.0);
    let d = ops::drill(&s, &builtin("drill-8").unwrap(), 6, -34.0, 6.0).unwrap();
    let rep = ops::certify(&s, &[d], None).unwrap();
    assert!(rep.ok(), "{:?}", rep.refusals);
}

#[test]
fn a_short_blade_cannot_part_a_big_bar() {
    let s = setup_of(
        common::part(&[(0.0, -50.0), (28.0, -50.0), (28.0, 0.0), (0.0, 0.0)]),
        30.0,
    );
    assert!(ops::part_off(&s, &builtin("part-3").unwrap(), 5, -50.0).is_err());
}

#[test]
fn a_groove_to_the_part_certifies_and_one_too_deep_is_refused() {
    // radius 10 with a groove 4 wide to radius 8 between z = -20 and -16
    let b = common::part(&[
        (0.0, -50.0),
        (10.0, -50.0),
        (10.0, -20.0),
        (8.0, -20.0),
        (8.0, -16.0),
        (10.0, -16.0),
        (10.0, 0.0),
        (0.0, 0.0),
    ]);
    let s = setup_of(b, 10.0);
    let t = builtin("groove-3").unwrap();
    let g = ops::groove(&s, &t, 3, -20.0, -16.0, 8.0).unwrap();
    let rep = ops::certify(&s, &[g], None).unwrap();
    assert!(rep.ok(), "{:?}", rep.refusals);
    let g = ops::groove(&s, &t, 3, -20.0, -16.0, 7.5).unwrap();
    let rep = ops::certify(&s, &[g], None).unwrap();
    assert!(
        rep.refusals
            .iter()
            .any(|r| r.rule == "part" && r.verdict.starts_with("Hit")),
        "{:?}",
        rep.refusals
    );
}

#[test]
fn a_part_bigger_than_the_bar_is_refused_at_the_setup() {
    let s = setup_of(
        common::part(&[(0.0, -20.0), (10.0, -20.0), (10.0, 0.0), (0.0, 0.0)]),
        9.0,
    );
    assert!(s.check().is_err());
}

#[test]
fn a_filleted_shoulder_lowers_to_an_arc() {
    // radius 10 to z = -20, a fillet of radius 2 into a shoulder of radius 15
    use common::{arc, line};
    let h = std::f64::consts::FRAC_PI_2;
    let b = common::part_segs(vec![
        line((0.0, -40.0), (15.0, -40.0)),
        line((15.0, -40.0), (15.0, -22.0)),
        line((15.0, -22.0), (12.0, -22.0)),
        // centre (12, -20), from straight down (-π/2) a quarter turn clockwise to (10, -20)
        arc((12.0, -20.0), 2.0, -h, -h),
        line((10.0, -20.0), (10.0, 0.0)),
        line((10.0, 0.0), (0.0, 0.0)),
        line((0.0, 0.0), (0.0, -40.0)),
    ]);
    let hp = half_profile(&b).unwrap();
    let arcs: Vec<_> = hp.region.loops[0]
        .segs
        .iter()
        .filter(|s| matches!(s, forge_cam_geom::region::Seg::Arc { .. }))
        .collect();
    assert_eq!(arcs.len(), 1, "{:?}", hp.region);
    // and a finishing pass rolls round it
    let s = setup_of(b, 16.0);
    let fin = ops::finish(&s, &builtin("od-finish-dnmg").unwrap(), 2, 0.0, -38.0).unwrap();
    assert!(fin.steps.iter().any(|st| matches!(st, ops::Step::Arc { .. })));
    let rep = ops::certify(&s, &[fin], None).unwrap();
    assert!(rep.ok(), "{:?}", rep.refusals);
}

#[test]
fn a_tailstock_centre_is_a_fixture() {
    // a live centre at the front face (its 60° point 0.5 ahead of it, growing to Ø20, its body
    // Ø20 to z = 40, the quill Ø40 beyond): facing to the centre line and finishing from over
    // it meet the centre; rough, thread and part-off from a bar already faced certify
    let mut s = common::setup();
    let t30 = forge_math::dmath::sin(30f64.to_radians()) / forge_math::dmath::cos(30f64.to_radians());
    s.fixtures = vec![(
        "tailstock".into(),
        forge_cam_geom::region::Region {
            loops: vec![forge_turn::profile::poly(&[
                [0.0, 0.5],
                [10.0, 0.5 + 10.0 / t30],
                [10.0, 40.0],
                [20.0, 40.0],
                [20.0, 80.0],
                [0.0, 80.0],
            ])],
        },
    )];
    let job = common::job(&s);
    let rep = ops::certify_fast(&s, &job, None).unwrap();
    assert!(
        rep.refusals.iter().any(|r| r.op == 0 && r.rule == "tailstock"),
        "{:?}",
        rep.refusals
    );
    s.bar_front = 0.0;
    // rough, thread and part off (the finish starts over the front face at the centre line,
    // where the centre is: refused too)
    let rest = vec![job[1].clone(), job[3].clone(), job[4].clone()];
    let fin = ops::certify_fast(&s, &job[1..3], None).unwrap();
    assert!(fin.refusals.iter().any(|r| r.op == 1 && r.rule == "tailstock"));
    let rep = ops::certify_fast(&s, &rest, None).unwrap();
    assert!(rep.ok(), "{:?}", rep.refusals);
}

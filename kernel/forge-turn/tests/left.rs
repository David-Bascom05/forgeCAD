//! Left-hand tools: a collar whose back chamfer faces the chuck. A right-hand tool finishes down
//! to the collar's edge, a blade cuts the neck behind it, and a left-hand VBMT finishes up the
//! chamfer. Refused: the right-hand tool on down the chamfer, and the left-hand one started low
//! in the neck (its insert hangs toward the chuck into the shoulder).
mod common;
use forge_turn::insert::{Kind, builtin, decode_holder};
use forge_turn::ops::{self, Chuck, OpPath, Setup};
use forge_turn::post;
use forge_turn::profile::half_profile;

fn collar() -> Setup {
    // Ø16 from the front face to z = -10, a Ø24 collar to -16, its back a 45° chamfer down to
    // the Ø16 neck at -20, the neck to -30, a Ø24 shoulder to the back face at -40
    let b = common::part(&[
        (0.0, -40.0),
        (12.0, -40.0),
        (12.0, -30.0),
        (8.0, -30.0),
        (8.0, -20.0),
        (12.0, -16.0),
        (12.0, -10.0),
        (8.0, -10.0),
        (8.0, 0.0),
        (0.0, 0.0),
    ]);
    Setup {
        part: half_profile(&b).unwrap(),
        bar_r: 13.0,
        bar_front: 1.0,
        chuck: Chuck {
            z_face: -55.0,
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

fn show(rep: &ops::Report) {
    for r in &rep.refusals {
        eprintln!(
            "REFUSED op {} step {} {} {} {} at {:?}",
            r.op, r.step, r.body, r.rule, r.verdict, r.at
        );
    }
}

fn job(s: &Setup) -> Vec<OpPath> {
    let rough_t = builtin("od-rough-cnmg").unwrap();
    let prof_t = builtin("od-profile-vbmt").unwrap();
    vec![
        ops::face(s, &rough_t, 1, 0.0, 1.0).unwrap(),
        ops::rough(s, &rough_t, 1, 0.3, 2.0, -41.0, 0.0).unwrap(),
        ops::finish(s, &prof_t, 2, 0.0, -16.0).unwrap(),
        ops::groove(s, &builtin("groove-3").unwrap(), 5, -30.0, -20.0, 8.0).unwrap(),
        ops::finish_up(s, &builtin("od-profile-vbmt-left").unwrap(), 9, 0.0, -21.0, -16.0).unwrap(),
    ]
}

#[test]
fn a_left_hand_holder_is_the_mirror() {
    assert_eq!(decode_holder("SVJBL2525M16").unwrap().hand, 'L');
    let r = builtin("od-profile-vbmt").unwrap();
    let l = builtin("od-profile-vbmt-left").unwrap();
    assert!(l.left && !r.left && l.kind == Kind::Turn);
    assert_eq!((r.tip_number, l.tip_number), (3, 4));
    assert_eq!(l.tip, [r.tip[0], -r.tip[1]]);
    // a left-hand tool is refused where the op feeds toward the chuck
    let s = collar();
    assert!(ops::finish(&s, &l, 9, 0.0, -16.0).is_err());
    assert!(ops::finish_up(&s, &r, 2, 0.0, -29.0, -16.0).is_err());
}

#[test]
fn the_collar_certifies_with_a_left_hand_tool() {
    let s = collar();
    let job = job(&s);
    let rep = ops::certify(&s, &job, None).unwrap();
    show(&rep);
    assert!(rep.ok());
    // the corners: the blade's 0.2 at the neck's back, the left-hand nose's 0.4 under the
    // chamfer, the right-hand nose's 0.4 at the collar's front; the shoulder's leave is "uncut"
    let mut why = rep.leftover_reasons.clone();
    why.sort();
    assert_eq!(why.len(), 4, "{why:?}");
    assert!(why[0].starts_with("nose radius 0.2") && why[1].starts_with("nose radius 0.4"));
    assert!(why[2].starts_with("nose radius 0.4") && why[3] == "uncut");
    assert!(ops::certify_fast(&s, &job, None).unwrap().ok());
    for (name, _) in post::DIALECTS {
        let d = post::Dialect::get(name).unwrap();
        let q = post::quantize(&job, 1, "collar", &d).unwrap();
        let (_, motions) = post::round_trip(&q, &d).unwrap_or_else(|e| panic!("{name}: {e}"));
        let posted = post::posted_moves(&motions, &job, &d);
        let rep = ops::certify_fast(&s, &job, Some(&posted)).unwrap();
        show(&rep);
        assert!(rep.ok(), "{name}");
    }
}

#[test]
fn the_wrong_hand_or_too_low_is_refused() {
    let s = collar();
    let mut j = job(&s);
    // the right-hand VBMT on down the back chamfer: its trailing edge meets the collar
    let rh = ops::finish(&s, &builtin("od-profile-vbmt").unwrap(), 2, 0.0, -21.0).unwrap();
    let mut down = j.clone();
    down[2] = rh;
    let rep = ops::certify_fast(&s, &down, None).unwrap();
    assert!(
        rep.refusals.iter().any(|r| r.op == 2 && r.body == "insert"),
        "{:?}",
        rep.refusals
    );
    // the left-hand VBMT started 1 mm over the shoulder: its insert hangs 9 mm toward the chuck
    j[4] = ops::finish_up(
        &s,
        &builtin("od-profile-vbmt-left").unwrap(),
        9,
        0.0,
        -29.0,
        -16.0,
    )
    .unwrap();
    let rep = ops::certify_fast(&s, &j, None).unwrap();
    assert!(
        rep.refusals.iter().any(|r| r.op == 4 && r.body == "insert"),
        "{:?}",
        rep.refusals
    );
}

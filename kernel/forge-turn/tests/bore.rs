//! Boring: a bush from a Ø52 bar: face, drill Ø26, rough and finish the Ø28 bore with a CCMT in an
//! S20 bar; and a bar too big for the hole is refused.
mod common;
use forge_turn::insert::{builtin, decode_bar};
use forge_turn::ops::{self, Chuck, Setup};
use forge_turn::profile::half_profile;

fn bush() -> Setup {
    let b = common::part(&[(14.0, -30.0), (25.0, -30.0), (25.0, 0.0), (14.0, 0.0)]);
    Setup {
        part: half_profile(&b).unwrap(),
        bar_r: 26.0,
        bar_front: 1.0,
        chuck: Chuck {
            z_face: -45.0,
            grip: 30.0,
            jaw_height: 15.0,
            body_r: 90.0,
            body_len: 60.0,
        },
        fixture_clearance: 1.0,
        holder_clearance: 0.25,
        rapid_clearance: 0.2,
        tol: 0.002,
        machine: None,
        place: None,
    }
}

#[test]
fn the_bar_decodes() {
    let b = decode_bar("S20R-SCLCR09").unwrap();
    assert_eq!((b.d, b.length, b.shape, b.kapr), (20.0, 200.0, 'C', 95.0));
    let t = builtin("id-bore-ccmt").unwrap();
    assert_eq!(t.kind, forge_turn::insert::Kind::Bore);
    assert!(t.bodies.iter().any(|b| b.cyl.is_some()));
}

#[test]
fn a_bush_is_bored() {
    let s = bush();
    let face = ops::face(&s, &builtin("od-rough-cnmg").unwrap(), 1, 0.0, 1.0).unwrap();
    let drill = ops::drill(&s, &builtin("drill-26").unwrap(), 6, -40.0, 0.0).unwrap();
    let bore_t = builtin("id-bore-ccmt").unwrap();
    let rough = ops::bore_rough(&s, &bore_t, 3, 0.2, 1.0, -31.0, 0.0, 13.0).unwrap();
    let fin = ops::finish(&s, &bore_t, 3, 0.0, -30.0).unwrap();
    let rep = ops::certify(&s, &[face, drill, rough, fin], None).unwrap();
    for r in &rep.refusals {
        eprintln!(
            "REFUSED op {} step {} {} {} {} at {:?}",
            r.op, r.step, r.body, r.rule, r.verdict, r.at
        );
    }
    assert!(rep.ok());
}

#[test]
fn a_bar_in_a_hole_too_small_is_refused() {
    // the same bar from the Ø8 drill's hole: the bar meets the stock
    let s = bush();
    let bore_t = builtin("id-bore-ccmt").unwrap();
    let rough = ops::bore_rough(&s, &bore_t, 3, 0.2, 1.0, -31.0, 0.0, 4.0).unwrap();
    let drill = ops::drill(&s, &builtin("drill-8").unwrap(), 6, -33.0, 0.0).unwrap();
    let rep = ops::certify(&s, &[drill, rough], None).unwrap();
    assert!(rep.refusals.iter().any(|r| r.body == "bar"), "{:?}", rep.refusals);
}

//! A whole lathe job on a stepped shaft: face, rough, finish, groove, thread, part off; every
//! move certified, the leftover listed; and the checks refuse what would gouge.
mod common;
use forge_turn::insert::builtin;
use forge_turn::ops::{self, Chuck, Setup};
use forge_turn::profile::half_profile;

pub fn setup() -> Setup {
    // a shaft: radius 10 from z = -40 to -1, a 1 mm chamfer to the front face at z = 0, a
    // shoulder to radius 15 from z = -40 to -55, its back face at -55
    let b = common::part(&[
        (0.0, -55.0),
        (15.0, -55.0),
        (15.0, -40.0),
        (10.0, -40.0),
        (10.0, -1.0),
        (9.0, 0.0),
        (0.0, 0.0),
    ]);
    Setup {
        part: half_profile(&b).unwrap(),
        bar_r: 16.0,
        bar_front: 2.0,
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
        travel: None,
    }
}

#[test]
fn the_shaft_job_certifies() {
    let s = setup();
    let rough_t = builtin("od-rough-cnmg").unwrap();
    let fin_t = builtin("od-finish-dnmg").unwrap();
    let part_t = builtin("part-3").unwrap();
    let thr_t = builtin("thread-60").unwrap();
    let face = ops::face(&s, &rough_t, 1, 0.0, 1.0).unwrap();
    let rough = ops::rough(&s, &rough_t, 1, 0.3, 2.0, -56.0, 0.0).unwrap();
    let fin = ops::finish(&s, &fin_t, 2, 0.0, -55.0).unwrap();
    let thr = ops::thread(&s, &thr_t, 4, 20.0, 1.5, 3.0, -30.0, 8).unwrap();
    let part = ops::part_off(&s, &part_t, 5, -55.0).unwrap();
    let job = vec![face, rough, fin, thr, part];
    let t0 = std::time::Instant::now();
    let rep = ops::certify(&s, &job, None).unwrap();
    eprintln!("{} checks in {:?}, least margin {}, stock skipped {}", rep.checks, t0.elapsed(), rep.least, rep.stock_skipped);
    for r in &rep.refusals {
        eprintln!("REFUSED op {} step {} {} {} {} at {:?}", r.op, r.step, r.body, r.rule, r.verdict, r.at);
    }
    for l in &rep.leftover {
        eprintln!("leftover {l:?}");
    }
    assert!(rep.ok());
}

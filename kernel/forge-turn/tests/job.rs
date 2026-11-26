//! A whole lathe job on a stepped shaft: face, rough, finish, groove, thread, part off; every
//! move certified, the leftover listed; and the checks refuse what would gouge.
mod common;
use common::{job, setup};
use forge_turn::insert::builtin;
use forge_turn::ops;
use forge_turn::post;

#[test]
fn the_posted_job_round_trips_and_certifies() {
    let s = setup();
    let job = job(&s);
    for (name, _) in post::DIALECTS {
        let d = post::Dialect::get(name).unwrap();
        let q = post::quantize(&job, 1, "shaft", &d).unwrap();
        let (text, motions) = post::round_trip(&q, &d).unwrap_or_else(|e| panic!("{name}: {e}"));
        if std::env::var("FORGE_TURN_SHOW").is_ok() {
            eprintln!("{text}");
        }
        let posted = post::posted_moves(&motions, &job, &d);
        let rep = ops::certify(&s, &job, Some(&posted)).unwrap();
        for r in &rep.refusals {
            eprintln!(
                "{name} REFUSED op {} step {} {} {} {}",
                r.op, r.step, r.body, r.rule, r.verdict
            );
        }
        assert!(rep.ok(), "{name}");
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
    eprintln!(
        "{} checks in {:?}, least margin {}, stock skipped {}",
        rep.checks,
        t0.elapsed(),
        rep.least,
        rep.stock_skipped
    );
    for r in &rep.refusals {
        eprintln!(
            "REFUSED op {} step {} {} {} {} at {:?}",
            r.op, r.step, r.body, r.rule, r.verdict, r.at
        );
    }
    for (l, why) in rep.leftover.iter().zip(&rep.leftover_reasons) {
        eprintln!("leftover ({why}) {l:?}");
    }
    assert!(rep.ok());
    // the one leftover: the shoulder's sharp inside corner, under the finishing nose's radius
    assert_eq!(rep.leftover.len(), 1);
    assert!(
        rep.leftover_reasons[0].starts_with("nose radius 0.4"),
        "{:?}",
        rep.leftover_reasons
    );
}

#[test]
fn the_normal_path_certifies_the_job_with_the_stock_as_columns() {
    let s = setup();
    let job = job(&s);
    let t0 = std::time::Instant::now();
    let fast = ops::certify_fast(&s, &job, None).unwrap();
    let tf = t0.elapsed();
    for r in &fast.refusals {
        eprintln!(
            "FAST REFUSED op {} step {} {} {} {}",
            r.op, r.step, r.body, r.rule, r.verdict
        );
    }
    let t0 = std::time::Instant::now();
    let full = ops::certify(&s, &job, None).unwrap();
    eprintln!("fast {tf:?} ({} checks), exact {:?}", fast.checks, t0.elapsed());
    assert!(fast.ok() && full.ok());
    assert_eq!(fast.stock_model, "columns");
    assert!(!fast.leftover_done && full.leftover_done);
}

//! Internal grooving and threading: a groove and an M30 x 1.5 thread in bushes bored from a Ø26
//! drilled hole, ID tools on Ø16 bars; and the bars refused where they do not fit.
mod common;
use forge_turn::insert::{Kind, builtin};
use forge_turn::ops::{self, Chuck, OpPath, Setup};
use forge_turn::post;
use forge_turn::profile::half_profile;

fn bush(pts: &[(f64, f64)]) -> Setup {
    Setup {
        part: half_profile(&common::part(pts)).unwrap(),
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

fn r1() -> f64 {
    15.0 - 0.625 * 1.5 * 0.75f64.sqrt()
}

fn bored(s: &Setup, z_fin: f64) -> Vec<OpPath> {
    let bore_t = builtin("id-bore-ccmt").unwrap();
    vec![
        ops::face(s, &builtin("od-rough-cnmg").unwrap(), 1, 0.0, 1.0).unwrap(),
        ops::drill(s, &builtin("drill-26").unwrap(), 6, -40.0, 0.0).unwrap(),
        ops::bore_rough(s, &bore_t, 3, 0.2, 1.0, -31.0, 0.0, 13.0).unwrap(),
        ops::finish(s, &bore_t, 3, 0.0, z_fin).unwrap(),
    ]
}

fn show(rep: &ops::Report) {
    for r in &rep.refusals {
        eprintln!(
            "REFUSED op {} step {} {} {} {} at {:?}",
            r.op, r.step, r.body, r.rule, r.verdict, r.at
        );
    }
}

/// Posted in every dialect, read back, and the path the text means certified.
fn posted_ok(s: &Setup, ops: &[OpPath]) {
    for (name, _) in post::DIALECTS {
        let d = post::Dialect::get(name).unwrap();
        let q = post::quantize(ops, 1, "bush", &d).unwrap();
        let (_, motions) = post::round_trip(&q, &d).unwrap_or_else(|e| panic!("{name}: {e}"));
        let posted = post::posted_moves(&motions, ops, &d);
        let rep = ops::certify_fast(s, ops, Some(&posted)).unwrap();
        show(&rep);
        assert!(rep.ok(), "{name}");
    }
}

#[test]
fn the_id_tools_decode() {
    let g = builtin("id-groove-2").unwrap();
    assert_eq!(g.kind, Kind::IdGroove);
    let t = builtin("id-thread-60").unwrap();
    assert_eq!(t.kind, Kind::IdThread);
    assert!(
        g.bodies
            .iter()
            .chain(&t.bodies)
            .filter(|b| b.cyl.is_some())
            .count()
            == 2
    );
}

#[test]
fn an_internal_groove_certifies() {
    let s = bush(&[
        (14.0, -30.0),
        (25.0, -30.0),
        (25.0, 0.0),
        (14.0, 0.0),
        (14.0, -17.0),
        (16.0, -17.0),
        (16.0, -20.0),
        (14.0, -20.0),
    ]);
    let mut ops = bored(&s, -16.5);
    ops.push(ops::id_groove(&s, &builtin("id-groove-2").unwrap(), 7, -20.0, -17.0, 16.0, 14.0).unwrap());
    let rep = ops::certify(&s, &ops, None).unwrap();
    show(&rep);
    assert!(rep.ok());
    let fast = ops::certify_fast(&s, &ops, None).unwrap();
    assert!(fast.ok());
    posted_ok(&s, &ops);
    // the groove's sharp corners under the blade's 0.1 corners, the bore's under the 0.4 nose
    assert_eq!(
        rep.leftover_reasons
            .iter()
            .filter(|r| r.starts_with("nose radius 0.1"))
            .count(),
        2
    );
    // half a millimetre too deep: G1
    ops.pop();
    ops.push(ops::id_groove(&s, &builtin("id-groove-2").unwrap(), 7, -20.0, -17.0, 16.5, 14.0).unwrap());
    let rep = ops::certify_fast(&s, &ops, None).unwrap();
    assert!(
        rep.refusals.iter().any(|r| r.op == 4 && r.body == "blade"),
        "{:?}",
        rep.refusals
    );
}

#[test]
fn an_internal_thread_certifies() {
    let r1 = r1();
    let s = bush(&[(r1, -30.0), (25.0, -30.0), (25.0, 0.0), (r1, 0.0)]);
    let mut ops = bored(&s, -30.0);
    ops.push(ops::id_thread(&s, &builtin("id-thread-60").unwrap(), 8, 30.0, 1.5, 3.0, -20.0, 8).unwrap());
    let rep = ops::certify(&s, &ops, None).unwrap();
    show(&rep);
    assert!(rep.ok());
    for (l, why) in rep.leftover.iter().zip(&rep.leftover_reasons) {
        let pts: Vec<_> = l
            .loops
            .iter()
            .flat_map(|lp| lp.segs.iter().map(|sg| sg.start()))
            .collect();
        let bx = pts.iter().fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, p| {
            [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)]
        });
        // only the OD, which no op here turns: nothing left in the bore or the thread's band
        assert!(why == "uncut" && bx[0] > 25.0, "{why} {bx:?}");
    }
    let fast = ops::certify_fast(&s, &ops, None).unwrap();
    assert!(fast.ok());
    posted_ok(&s, &ops);
    // an M31 x 1.5 in the same bore cuts past the declared band: G1
    ops.pop();
    ops.push(ops::id_thread(&s, &builtin("id-thread-60").unwrap(), 8, 31.0, 1.5, 3.0, -20.0, 8).unwrap());
    let rep = ops::certify_fast(&s, &ops, None).unwrap();
    assert!(rep.refusals.iter().any(|r| r.op == 4), "{:?}", rep.refusals);
}

#[test]
fn id_tools_that_do_not_fit_are_refused() {
    // an M12 x 1.5 thread in a bush bored to its minor diameter: the Ø16 bar does not fit
    let r = 6.0 - 0.625 * 1.5 * 0.75f64.sqrt();
    let s = bush(&[(r, -30.0), (25.0, -30.0), (25.0, 0.0), (r, 0.0)]);
    let ops = vec![
        ops::drill(&s, &builtin("drill-8").unwrap(), 6, -33.0, 0.0).unwrap(),
        ops::id_thread(&s, &builtin("id-thread-60").unwrap(), 8, 12.0, 1.5, 3.0, -20.0, 4).unwrap(),
    ];
    let rep = ops::certify(&s, &ops, None).unwrap();
    assert!(rep.refusals.iter().any(|r| r.body == "bar"), "{:?}", rep.refusals);
    // an OD blade is no ID grooving tool
    assert!(ops::id_groove(&s, &builtin("groove-3").unwrap(), 5, -20.0, -17.0, 9.0, 5.0).is_err());
}

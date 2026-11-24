//! The trigon (W) insert and left-hand bars: a WNMG in a PWLNR holder roughs the shaft and
//! certifies; its core closes (six equal sides); a left-hand bar decodes as its right-hand twin
//! mirrored and is refused where the op feeds toward the chuck.
mod common;
use forge_turn::insert::{boring_tool, decode_insert, turning_tool};
use forge_turn::ops;

#[test]
fn a_trigon_decodes_and_roughs() {
    let w = decode_insert("WNMG080408").unwrap();
    assert_eq!(
        (w.shape, w.eps, w.ic, w.thickness, w.re),
        ('W', 80.0, 12.7, 4.76, 0.8)
    );
    // the side between an 80° and a 160° corner, as catalogues state it (8.69)
    assert!((w.l - 8.687).abs() < 0.005, "{}", w.l);
    let t = turning_tool("w", "WNMG080408", "PWLNR2525M08", None).unwrap();
    let core = &t.bodies[0].core;
    assert_eq!(core.len(), 6);
    // the last side closes on the nose: every side the same length
    let side = |a: [f64; 2], b: [f64; 2]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
    let e = side(core[0], core[1]);
    for i in 0..6 {
        assert!((side(core[i], core[(i + 1) % 6]) - e).abs() < 1e-9);
    }
    let s = common::setup();
    let job = vec![
        ops::face(&s, &t, 1, 0.0, 1.0).unwrap(),
        ops::rough(&s, &t, 1, 0.3, 2.0, -56.0, 0.0).unwrap(),
    ];
    let rep = ops::certify_fast(&s, &job, None).unwrap();
    assert!(rep.ok(), "{:?}", rep.refusals);
}

#[test]
fn a_left_hand_bar_is_the_mirror_and_is_refused_feeding_toward_the_chuck() {
    let r = boring_tool("r", "CCMT09T304", "S20R-SCLCR09", None).unwrap();
    let l = boring_tool("l", "CCMT09T304", "S20R-SCLCL09", None).unwrap();
    assert!(l.left && !r.left);
    assert_eq!((r.tip_number, l.tip_number), (2, 1));
    let c = |t: &forge_turn::insert::TurnTool| t.bodies[1].cyl.unwrap().z;
    assert_eq!(c(&l), [-c(&r)[1], -c(&r)[0]]);
    let s = common::setup();
    assert!(ops::bore_rough(&s, &l, 3, 0.2, 1.0, -31.0, 0.0, 13.0).is_err());
    assert!(ops::finish(&s, &l, 3, 0.0, -30.0).is_err());
}

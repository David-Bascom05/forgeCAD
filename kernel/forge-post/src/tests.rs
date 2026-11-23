use super::*;

fn prog(with_tap: bool) -> Program {
    let mut recs = vec![
        Rec::Comment("op1 (face)".into()),
        Rec::Wcs(0),
        Rec::ToolChange {
            t: 1,
            h: 1,
            name: "flat-6".into(),
        },
        Rec::Spindle {
            rpm: 9549.3,
            cw: true,
        },
        Rec::Coolant(Coolant::Flood),
        Rec::Rapid([-43.0, -43.0, 25.0]),
        Rec::Rapid([-43.0, -43.0, 2.0]),
        Rec::Line([-43.0, -43.0, -1.0], 300.0),
        Rec::Line([43.0, -43.0, -1.0], 1200.0),
        Rec::Arc {
            to: [46.0, -40.0, -1.0],
            c: [43.0, -40.0],
            cw: false,
            f: 1200.0,
        },
        Rec::Line([46.0, 40.0, -1.0], 1200.0),
        // a full circle, then a helix down
        Rec::Arc {
            to: [46.0, 40.0, -1.0],
            c: [40.0, 40.0],
            cw: true,
            f: 1000.0,
        },
        Rec::Arc {
            to: [34.0, 40.0, -2.5],
            c: [40.0, 40.0],
            cw: true,
            f: 1000.0,
        },
        Rec::Line([0.1 + 0.2, 1.0 / 3.0, -2.5], 1000.0),
        Rec::Rapid([0.3, 0.3333333333333333, 25.0]),
        Rec::Rapid([30.0, 0.0, 25.0]),
        Rec::Cycle {
            kind: CycleKind::Peck,
            holes: vec![[30.0, 0.0], [0.0, 30.0], [-30.0, 0.0]],
            z: -14.0,
            r: 2.0,
            clear: 25.0,
            q: 4.0,
            dwell: 0.0,
            f: 150.0,
        },
        Rec::Rapid([0.0, -30.0, 25.0]),
        Rec::Cycle {
            kind: CycleKind::Dwell,
            holes: vec![[0.0, -30.0]],
            z: -3.0,
            r: 2.0,
            clear: 25.0,
            q: 0.0,
            dwell: 0.25,
            f: 120.0,
        },
    ];
    if with_tap {
        recs.push(Rec::Cycle {
            kind: CycleKind::Tap,
            holes: vec![[0.0, -30.0]],
            z: -10.0,
            r: 2.0,
            clear: 25.0,
            q: 0.0,
            dwell: 0.0,
            f: 1000.0,
        });
    }
    recs.extend([Rec::Dwell(0.5), Rec::Coolant(Coolant::Off), Rec::SpindleStop]);
    Program {
        number: 1001,
        title: "flange (op 1)".into(),
        recs,
    }
}

#[test]
fn quantisation_is_exact() {
    assert_eq!(quant(2.5, 0).unwrap(), 2);
    assert_eq!(quant(3.5, 0).unwrap(), 4);
    assert_eq!(quant(-2.5, 0).unwrap(), -2);
    // 0.0005 is just above 5e-4 as a double: it rounds up at 3 digits
    assert_eq!(quant(0.0005, 3).unwrap(), 1);
    // 1.0005 is just below as a double: down
    assert_eq!(quant(1.0005, 3).unwrap(), 1000);
    assert_eq!(quant(0.1 + 0.2, 3).unwrap(), 300);
    assert_eq!(quant(-1.0 / 3.0, 4).unwrap(), -3333);
    assert_eq!(digits(10000, 3, Point::Always), "10.");
    assert_eq!(digits(-500, 3, Point::Always), "-0.5");
    assert_eq!(digits(10000, 3, Point::Never), "10");
    assert_eq!(digits(10500, 3, Point::Full), "10.500");
    assert!(quant(f64::NAN, 3).is_err());
}

#[test]
fn every_dialect_round_trips() {
    for (name, _) in DIALECTS {
        let d = Dialect::get(name).unwrap();
        let mut p = prog(false);
        if name != "grbl" {
            p.recs.push(Rec::Home);
        }
        let q = quantize(&p, &d).unwrap();
        let (txt, back) = round_trip(&q, &d).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(back.len() > 20, "{name}");
        // the same text everywhere: no float formatting was involved
        assert_eq!(txt, print(&q, &d).unwrap());
        if name == "fanuc" {
            assert!(txt.contains("G98 G83 X30. Y0. Z-14. R2. Q4. F150."), "{txt}");
            assert!(txt.starts_with("%\nN10 O1001 (flange op 1)\n"), "{txt}");
        }
        if name == "grbl" {
            assert!(!txt.contains("G83") && !txt.contains("G81"), "{txt}");
        }
    }
}

#[test]
fn refusals_are_typed() {
    let g = Dialect::get("grbl").unwrap();
    let q = quantize(&prog(true), &g).unwrap();
    assert!(matches!(print(&q, &g), Err(PostError::Unsupported { .. })));
    let f = Dialect::get("fanuc").unwrap();
    let q = quantize(&prog(true), &f).unwrap();
    assert!(round_trip(&q, &f).is_ok());
    assert!(matches!(
        Dialect::get("heidenhain"),
        Err(PostError::UnknownDialect { .. })
    ));
    // a cycle not started over its first hole
    let mut p = prog(false);
    p.recs.retain(|r| !matches!(r, Rec::Rapid(v) if v[0] == 30.0));
    let q = quantize(&p, &f).unwrap();
    assert!(matches!(print(&q, &f), Err(PostError::Unsupported { .. })));
}

#[test]
fn reader_refuses_what_is_not_ours() {
    let f = Dialect::get("fanuc").unwrap();
    assert!(read("G0 X1.2345\n", &f).is_err());
    assert!(read("G41 D1\n", &f).is_err());
    assert!(read("G0 X1. Y1. Z1.\nG1 X2. F100.\n", &f).is_ok());
}

#[test]
fn a_dropped_null_move_leaves_no_mode_behind() {
    // found by the Lean model of G6: a null feed after a rapid must not make the next feed a rapid
    // (nor a null rapid after a feed make the next retract a feed)
    for (name, _) in DIALECTS {
        let d = Dialect::get(name).unwrap();
        for null_rapid in [false, true] {
            let mut recs = vec![
                Rec::ToolChange {
                    t: 1,
                    h: 1,
                    name: "x".into(),
                },
                Rec::Rapid([0.0, 0.0, 5.0]),
                Rec::Line([0.0, 0.0, 0.0], 100.0),
                Rec::Rapid([0.0, 0.0, 5.0]),
            ];
            if null_rapid {
                recs.push(Rec::Line([0.0, 0.0, -3.0], 100.0));
                recs.push(Rec::Rapid([0.0, 0.0, -3.0]));
                recs.push(Rec::Line([0.0, 0.0, 5.0], 100.0));
            } else {
                recs.push(Rec::Line([0.0, 0.0, 5.0], 100.0));
                recs.push(Rec::Line([0.0, 0.0, -3.0], 100.0));
            }
            let q = quantize(
                &Program {
                    number: 1,
                    title: "t".into(),
                    recs,
                },
                &d,
            )
            .unwrap();
            round_trip(&q, &d).unwrap_or_else(|e| panic!("{name} {null_rapid}: {e}"));
            // the plunge to z −3 reads back as a feed, never a rapid
            let txt = print(&q, &d).unwrap();
            let back = read(&txt, &d).unwrap();
            let plunge = back
                .iter()
                .find(|m| matches!(m, Motion::Feed(p, _) | Motion::Rapid(p) if p[2] < 0))
                .expect("the plunge");
            assert!(
                matches!(plunge, Motion::Feed(..)),
                "{name} {null_rapid}: {plunge:?}\n{txt}"
            );
        }
    }
}

#[test]
fn a_vanishing_arc_is_not_a_full_circle() {
    // an arc of 1e-13 rad: its end rounds onto its start; G-code would read a full circle
    let d = Dialect::get("fanuc").unwrap();
    let s = [73.0, 34.0, 1.0];
    let to = [73.00000000000004, 33.999999999999964, 1.0];
    let p = Program {
        number: 1,
        title: "t".into(),
        recs: vec![
            Rec::Rapid(s),
            Rec::Arc {
                to,
                c: [70.0, 30.0],
                cw: true,
                f: 500.0,
            },
            Rec::Line([80.0, 34.0, 1.0], 500.0),
        ],
    };
    let q = quantize(&p, &d).unwrap();
    assert!(
        !q.recs.iter().any(|r| matches!(r, QRec::Arc { .. })),
        "{:?}",
        q.recs
    );
    let txt = print(&q, &d).unwrap();
    assert!(!txt.split_whitespace().any(|w| w == "G2" || w == "G3"), "{txt}");
    // a full circle stays one
    let p = Program {
        number: 1,
        title: "t".into(),
        recs: vec![
            Rec::Rapid(s),
            Rec::Arc {
                to: s,
                c: [70.0, 30.0],
                cw: true,
                f: 500.0,
            },
        ],
    };
    assert!(
        quantize(&p, &d)
            .unwrap()
            .recs
            .iter()
            .any(|r| matches!(r, QRec::Arc { .. }))
    );
}

#[test]
fn a_vanishing_helix_piece_is_not_a_full_helix() {
    // found by the Lean model (7a47f6a): X and Y round onto the start while Z moves; a control
    // decides a full circle from X and Y alone, so the old rule (all three equal) posted a 360°
    // helix in place of a 0.0002 mm piece
    for (name, _) in DIALECTS {
        let d = Dialect::get(name).unwrap();
        let s = [5.0, 0.0, 0.0];
        let to = [5.0 - 1e-9, 0.00004, -0.5];
        let p = Program {
            number: 1,
            title: "t".into(),
            recs: vec![
                Rec::ToolChange {
                    t: 1,
                    h: 1,
                    name: "x".into(),
                },
                Rec::Rapid([5.0, 0.0, 5.0]),
                Rec::Line(s, 100.0),
                Rec::Arc {
                    to,
                    c: [0.0, 0.0],
                    cw: false,
                    f: 100.0,
                },
                Rec::Line([6.0, 0.0, -0.5], 100.0),
            ],
        };
        let q = quantize(&p, &d).unwrap();
        assert!(
            !q.recs.iter().any(|r| matches!(r, QRec::Arc { .. })),
            "{name}: {:?}",
            q.recs
        );
        let (txt, back) = round_trip(&q, &d).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(
            !txt.split_whitespace().any(|w| w == "G2" || w == "G3"),
            "{name}: {txt}"
        );
        assert!(
            back.iter().any(|m| matches!(m, Motion::Feed(p, _) if p[2] < 0)),
            "{name}"
        );
        // a whole helical turn, meant, stays one
        let p = Program {
            number: 1,
            title: "t".into(),
            recs: vec![
                Rec::ToolChange {
                    t: 1,
                    h: 1,
                    name: "x".into(),
                },
                Rec::Rapid([5.0, 0.0, 5.0]),
                Rec::Line(s, 100.0),
                Rec::Arc {
                    to: [5.0, 0.0, -0.5],
                    c: [0.0, 0.0],
                    cw: false,
                    f: 100.0,
                },
            ],
        };
        let q = quantize(&p, &d).unwrap();
        let (_, back) = round_trip(&q, &d).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(
            back.iter().any(|m| matches!(m, Motion::Arc { full: true, .. })),
            "{name}"
        );
    }
}

#[test]
fn round_trip_compares_what_the_control_executes() {
    // an arc meant as a part of a turn whose X and Y nevertheless print equal is a full circle
    // to the control: the round trip refuses it (by motion, not by text)
    let d = Dialect::get("fanuc").unwrap();
    let q = QProgram {
        number: 1,
        title: "t".into(),
        recs: vec![
            QRec::Rapid([5000, 0, 0]),
            QRec::Arc {
                to: [5000, 0, -500],
                c: [0, 0],
                cw: false,
                f: 1000,
                full: false,
            },
        ],
        decimals: d.decimals,
        feed_decimals: d.feed_decimals,
    };
    let e = round_trip(&q, &d).unwrap_err();
    assert!(e.contains("differs"), "{e}");
}

/// Haas (research/cam/machines/haas-dialect-audit.md): dwell P in whole milliseconds (a P with a
/// decimal point would be seconds on Haas), through-tool coolant M88/M89, and mist refused: no
/// Haas M-code list has M07.
#[test]
fn haas_dwell_coolant_and_no_mist() {
    let h = Dialect::get("haas").unwrap();
    let mut p = prog(false);
    p.recs.push(Rec::Dwell(0.5));
    p.recs.push(Rec::Coolant(Coolant::Through));
    p.recs.push(Rec::Coolant(Coolant::Off));
    let txt = print(&quantize(&p, &h).unwrap(), &h).unwrap();
    assert!(txt.contains("G04 P500\n"), "{txt}");
    assert!(txt.contains("M88\n"), "{txt}");
    p.recs.push(Rec::Coolant(Coolant::Mist));
    match print(&quantize(&p, &h).unwrap(), &h) {
        Err(PostError::Unsupported { what, fix }) => {
            assert_eq!(what, "coolant Mist");
            assert!(fix.contains("haas") && fix.contains("coolant_mist"), "{fix}");
        }
        r => panic!("{r:?}"),
    }
}

/// G187 is opt-in: `haas` posts none (an option-less control may alarm on it), `haas-hsm` posts
/// G187 P2 and is otherwise the same dialect line for line.
#[test]
fn g187_only_with_the_hsm_option() {
    let (h, s) = (Dialect::get("haas").unwrap(), Dialect::get("haas-hsm").unwrap());
    let p = prog(false);
    let th = print(&quantize(&p, &h).unwrap(), &h).unwrap();
    let ts = print(&quantize(&p, &s).unwrap(), &s).unwrap();
    assert!(!th.contains("G187"), "{th}");
    assert!(ts.contains("G187 P2\n"), "{ts}");
    // the same program apart from that block (and the line numbers after it)
    let strip = |t: &str| -> Vec<String> {
        t.lines()
            .filter(|l| !l.contains("G187"))
            .map(|l| {
                l.split_once(' ')
                    .map_or(l, |(n, rest)| if n.starts_with('N') { rest } else { l })
                    .to_string()
            })
            .collect()
    };
    assert_eq!(strip(&th), strip(&ts));
    let src = |n: &str| DIALECTS.iter().find(|d| d.0 == n).unwrap().1;
    let keys = |t: &str| -> Vec<String> {
        t.lines()
            .filter(|l| !l.starts_with('#') && !l.starts_with("name") && !l.starts_with("safe_start"))
            .map(str::to_string)
            .collect()
    };
    assert_eq!(keys(src("haas")), keys(src("haas-hsm")));
}

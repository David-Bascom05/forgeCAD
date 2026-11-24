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

/// The test program without what the dialect refuses by design (a dwell whose units no open
/// source gives, a coolant whose code is machine-configured): those refusals are tested apart.
fn prog_for(d: &Dialect, tap: bool) -> Program {
    let mut p = prog(tap);
    if d.s("dwell").is_empty() {
        p.recs.retain(|r| {
            !matches!(
                r,
                Rec::Dwell(_)
                    | Rec::Cycle {
                        kind: CycleKind::Dwell,
                        ..
                    }
            )
        });
    }
    if d.s("coolant_flood").is_empty() {
        p.recs.retain(|r| !matches!(r, Rec::Coolant(_)));
    }
    p
}

#[test]
fn every_dialect_round_trips() {
    for (name, _) in DIALECTS {
        let d = Dialect::get(name).unwrap();
        let mut p = prog_for(&d, false);
        if !d.lines("retract_home").is_empty() {
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
        Dialect::get("no-such-control"),
        Err(PostError::UnknownDialect { .. })
    ));
    // a real control whose manual is not open: refused with the reason
    for (n, _) in UNAVAILABLE {
        assert!(
            matches!(Dialect::get(n), Err(PostError::Unavailable { .. })),
            "{n}"
        );
    }
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

/// The data-driven extras a non-Fanuc control needs: an end-of-line comment, a block terminator,
/// a `%` header, a dwell in another word, G codes that carry no motion; and per-code citations.
const EXTRAS: &str = r#"
[dialect]
name = "extras"
decimals = 3
feed_decimals = 0
point = "always"
program_open = ["%P{num} G71", "G15 H1"]
program_close = ["M30"]
comment = "; {text}"
comment_eol = ";"
comment_strip = ";"
block_end = " *"
line_numbers = true
safe_start = ["G90 G17 G94"]
tool_change = ["T{t} M06"]
length_comp = "G56 H{h}"
spindle_cw = "S{rpm} M03"
spindle_ccw = "S{rpm} M04"
spindle_stop = "M05"
wcs = "G15 H{n}"
wcs_first = 1
wcs_count = 6
arc = "ijk"
retract_home = ["G53 Z0."]
coolant_flood = "M08"
coolant_mist = "M07"
coolant_off = "M09"
dwell = "G04 F{s}"
cycle_drill = "expand"
cycle_dwell = "expand"
cycle_peck = "expand"
cycle_chipbreak = "expand"
cycle_tap = "refuse"
cycle_ream = "expand"
read_g_ignore = [15, 71]
read_g_skip = [56]
"#;

#[test]
fn data_driven_extras_round_trip() {
    let d = Dialect::parse(EXTRAS).unwrap();
    let mut p = prog(false);
    p.recs.push(Rec::Home);
    let q = quantize(&p, &d).unwrap();
    let (txt, back) = round_trip(&q, &d).unwrap_or_else(|e| panic!("{e}"));
    assert!(back.len() > 20);
    assert!(txt.starts_with("%P1001 G71\nN10 G15 H1 *\n"), "{txt}");
    // a comment runs to the end of its line: no terminator after it
    assert!(txt.contains("N30 ; op1 (face)\n"), "{txt}");
    assert!(txt.contains("G04 F0.5 *"), "{txt}");
    // a comment after a block is dropped by the reader, the terminator too
    assert!(read("G0 X1. Y1. Z1. ; here *\nG1 X2. F100 *\n", &d).is_ok());
    // what the dialect does not list is still refused
    assert!(read("G0 X1. Y1. Z1. *\nG41 D1 *\n", &d).is_err());
}

#[test]
fn a_cited_dialect_cites_every_code_it_sets() {
    let uncited = format!("{EXTRAS}cite_required = true\n");
    assert!(matches!(Dialect::parse(&uncited), Err(PostError::Dialect(_))));
    // every key cited: accepted, and the cites are kept
    let mut cited = uncited.clone();
    let keys: Vec<&str> = KEYS
        .iter()
        .copied()
        .filter(|k| EXTRAS.contains(&format!("\n{k} =")))
        .collect();
    for k in &keys {
        cited.push_str(&format!(
            "[[dialect.cite]]\nkey = \"{k}\"\nsource = \"https://example.org/manual.pdf\"\ntaken = \"2026-11-23\"\nwhere = \"p. 1\"\n"
        ));
    }
    let d = Dialect::parse(&cited).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(d.cites.len(), keys.len());
    // a published cite without its source is refused
    let bad = format!("{cited}[[dialect.cite]]\nkey = \"dwell\"\n");
    assert!(Dialect::parse(&bad).is_err());
}

#[test]
fn siemens_iso_program_reads_as_the_manual_says() {
    let d = Dialect::get("siemens").unwrap();
    assert!(d.cites.len() >= 25);
    let mut p = prog(true);
    p.recs.push(Rec::Home);
    let q = quantize(&p, &d).unwrap();
    let (txt, _) = round_trip(&q, &d).unwrap();
    // ISO mode selected alone in the first block, metric in its own block, G43 under G00
    assert!(txt.starts_with("N10 G291\nN20 (flange op 1)\nN30 G21\n"), "{txt}");
    assert!(txt.contains("G00 G43 H1"), "{txt}");
    assert!(txt.contains("G04 X0.5"), "{txt}");
    assert!(txt.contains("G98 G82 X0. Y-30. Z-3. R2. P250 F120."), "{txt}");
    // peck and chip-break retracts are GUD settings: posted as moves
    assert!(!txt.contains("G83") && !txt.contains("G73"), "{txt}");
    eprintln!("{txt}");
}

#[test]
fn heidenhain_iso_program_reads_as_the_manual_says() {
    let d = Dialect::get("heidenhain").unwrap();
    let q = quantize(&prog(false), &d).unwrap();
    let (txt, _) = round_trip(&q, &d).unwrap();
    assert!(
        txt.starts_with("%P1001 G71 *\nN10 G40 G90*\nN20 ;op1 (face)\n"),
        "{txt}"
    );
    assert!(txt.ends_with("M30*\nN99999999 %P1001 G71 *\n"), "{txt}");
    // tool call by T (no M6), speed in a bare T block, centre-first arcs, signed words
    assert!(
        txt.contains("T1 G17*") && txt.contains("T G17 S9549*") && txt.contains("M3*"),
        "{txt}"
    );
    assert!(
        txt.contains("I+43 J-40*\nN") && txt.contains("G03 X+46 Y-40"),
        "{txt}"
    );
    assert!(txt.contains("G04 F0.25*"), "{txt}");
    assert!(
        !txt.contains("G81") && !txt.contains("G83") && !txt.contains("M6"),
        "{txt}"
    );
    // tapping and a machine-home retract are refused, a second work offset too
    let mut p = prog(true);
    let qt = quantize(&p, &d).unwrap();
    assert!(matches!(print(&qt, &d), Err(PostError::Unsupported { .. })));
    p = prog(false);
    p.recs.push(Rec::Home);
    assert!(matches!(
        print(&quantize(&p, &d).unwrap(), &d),
        Err(PostError::Unsupported { .. })
    ));
    p = prog(false);
    p.recs.insert(0, Rec::Wcs(1));
    assert!(matches!(
        print(&quantize(&p, &d).unwrap(), &d),
        Err(PostError::Unsupported { .. })
    ));
    eprintln!("{txt}");
}

#[test]
fn machine_configured_codes_are_refused_not_guessed() {
    // Mach3: the dwell unit and the flood/mist outputs are settings the program cannot see
    let d = Dialect::get("mach3").unwrap();
    let mut p = prog(false);
    p.recs.retain(|r| !matches!(r, Rec::Coolant(_)));
    assert!(matches!(
        print(&quantize(&p, &d).unwrap(), &d),
        Err(PostError::Unsupported { .. })
    ));
    let p = prog_for(&d, false);
    let mut with_flood = p.clone();
    with_flood.recs.insert(3, Rec::Coolant(Coolant::Flood));
    assert!(matches!(
        print(&quantize(&with_flood, &d).unwrap(), &d),
        Err(PostError::Unsupported { .. })
    ));
    assert!(print(&quantize(&p, &d).unwrap(), &d).is_ok());
    // PathPilot's G84 dwells by default when P is left out: refused; MASSO has no G84
    for n in ["pathpilot", "masso"] {
        let d = Dialect::get(n).unwrap();
        let q = quantize(&prog_for(&d, true), &d).unwrap();
        assert!(matches!(print(&q, &d), Err(PostError::Unsupported { .. })), "{n}");
    }
    // Centroid: parentheses are expressions there, so no comment is written with them
    let d = Dialect::get("centroid-cnc12").unwrap();
    let txt = print(&quantize(&prog(false), &d).unwrap(), &d).unwrap();
    assert!(!txt.contains('(') && txt.contains("; op1 face"), "{txt}");
    // every dialect restated from a maker's manual cites every code it sets
    for n in [
        "siemens",
        "heidenhain",
        "pathpilot",
        "mach3",
        "mach4",
        "centroid-cnc12",
        "masso",
    ] {
        assert!(!Dialect::get(n).unwrap().cites.is_empty(), "{n}");
    }
}

/// A contour with cutter compensation (wear): on before the lead line, off before the lead-out
/// line. A dialect without cited comp keys and corner behaviour refuses it; one with them
/// prints G41 D / G40 and reads them back as the same events.
fn comp_prog() -> Program {
    Program {
        number: 1,
        title: "comp".into(),
        recs: vec![
            Rec::ToolChange {
                t: 3,
                h: 3,
                name: "flat-10".into(),
            },
            Rec::Spindle {
                rpm: 5000.0,
                cw: true,
            },
            Rec::Rapid([-20.0, -20.0, 5.0]),
            Rec::Line([-20.0, -20.0, -2.0], 300.0),
            Rec::Comp(Some((CompSide::Left, 3))),
            Rec::Line([-10.0, -10.0, -2.0], 800.0),
            Rec::Line([10.0, -10.0, -2.0], 800.0),
            Rec::Comp(None),
            Rec::Line([20.0, -20.0, -2.0], 800.0),
            Rec::Rapid([20.0, -20.0, 5.0]),
        ],
    }
}

#[test]
fn cutter_comp_rules_print_and_read() {
    let fanuc = Dialect::get("fanuc").unwrap();
    let q = quantize(&comp_prog(), &fanuc).unwrap();
    // no dialect declares its corner behaviour yet: refused, with the fix
    match print(&q, &fanuc) {
        Err(PostError::Unsupported { what, .. }) => assert!(what.contains("cutter compensation"), "{what}"),
        r => panic!("{r:?}"),
    }
    // LinuxCNC cites G41/G42/G40 but not its corner behaviour: refused too
    let lc = Dialect::get("linuxcnc").unwrap();
    let ql = quantize(&comp_prog(), &lc).unwrap();
    assert!(matches!(print(&ql, &lc), Err(PostError::Unsupported { .. })));
    // a test dialect with the four keys
    let src = format!(
        "{}\ncomp_left = \"G41 D{{d}}\"\ncomp_right = \"G42 D{{d}}\"\ncomp_off = \"G40\"\ncomp_corner = \"arc\"\n",
        DIALECTS
            .iter()
            .find(|d| d.0 == "fanuc")
            .unwrap()
            .1
            .replace("cite_required = true", "")
    );
    let d = Dialect::parse(&src).unwrap();
    let q = quantize(&comp_prog(), &d).unwrap();
    let (txt, back) = round_trip(&q, &d).unwrap();
    assert!(txt.lines().any(|l| l.contains("G41 D3")), "{txt}");
    assert!(txt.lines().any(|l| l.ends_with("G40")), "{txt}");
    assert_eq!(back.iter().filter(|m| matches!(m, Motion::Comp(_))).count(), 2);
    // the rules
    let mut p = comp_prog();
    p.recs[5] = Rec::Arc {
        to: [-10.0, -10.0, -2.0],
        c: [-15.0, -15.0],
        cw: false,
        f: 800.0,
    };
    assert!(quantize(&p, &d).is_err(), "an arc after G41");
    let mut p = comp_prog();
    p.recs.truncate(8);
    p.recs.remove(7);
    assert!(quantize(&p, &d).is_err(), "on at the end");
    let mut p = comp_prog();
    p.recs.insert(6, Rec::Home);
    assert!(quantize(&p, &d).is_err(), "home while on");
}

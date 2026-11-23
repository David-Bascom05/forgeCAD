use super::*;

fn prog(with_tap: bool) -> Program {
    let mut recs = vec![
        Rec::Comment("op1 (face)".into()),
        Rec::Wcs(0),
        Rec::ToolChange { t: 1, h: 1, name: "flat-6".into() },
        Rec::Spindle { rpm: 9549.3, cw: true },
        Rec::Coolant(Coolant::Flood),
        Rec::Rapid([-43.0, -43.0, 25.0]),
        Rec::Rapid([-43.0, -43.0, 2.0]),
        Rec::Line([-43.0, -43.0, -1.0], 300.0),
        Rec::Line([43.0, -43.0, -1.0], 1200.0),
        Rec::Arc { to: [46.0, -40.0, -1.0], c: [43.0, -40.0], cw: false, f: 1200.0 },
        Rec::Line([46.0, 40.0, -1.0], 1200.0),
        // a full circle, then a helix down
        Rec::Arc { to: [46.0, 40.0, -1.0], c: [40.0, 40.0], cw: true, f: 1000.0 },
        Rec::Arc { to: [34.0, 40.0, -2.5], c: [40.0, 40.0], cw: true, f: 1000.0 },
        Rec::Line([0.1 + 0.2, 1.0 / 3.0, -2.5], 1000.0),
        Rec::Rapid([0.3, 0.3333333333333333, 25.0]),
        Rec::Rapid([30.0, 0.0, 25.0]),
        Rec::Cycle { kind: CycleKind::Peck, holes: vec![[30.0, 0.0], [0.0, 30.0], [-30.0, 0.0]], z: -14.0, r: 2.0, clear: 25.0, q: 4.0, dwell: 0.0, f: 150.0 },
        Rec::Rapid([0.0, -30.0, 25.0]),
        Rec::Cycle { kind: CycleKind::Dwell, holes: vec![[0.0, -30.0]], z: -3.0, r: 2.0, clear: 25.0, q: 0.0, dwell: 0.25, f: 120.0 },
    ];
    if with_tap {
        recs.push(Rec::Cycle { kind: CycleKind::Tap, holes: vec![[0.0, -30.0]], z: -10.0, r: 2.0, clear: 25.0, q: 0.0, dwell: 0.0, f: 1000.0 });
    }
    recs.extend([Rec::Dwell(0.5), Rec::Coolant(Coolant::Off), Rec::SpindleStop]);
    Program { number: 1001, title: "flange (op 1)".into(), recs }
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
    assert!(matches!(Dialect::get("heidenhain"), Err(PostError::UnknownDialect { .. })));
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

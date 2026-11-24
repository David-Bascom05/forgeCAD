//! SINUMERIK 840D native programs (the Spinner U5-620 dataset's, tests/data/spinner-u5, CC BY 4.0):
//! read by the shell's 840D front end (src/siemens.rs, data/controls/siemens-840d.toml).
use forge_machine::gcode::{self, Control, Ev};
use forge_machine::library::builtin_named;
use forge_machine::sim::{self, Cut, Setup, SimTool};

const KEYCHAIN: &str = include_str!("data/spinner-u5/Keychain_Schluesselanhaenger.mpf");
const BOTTLE: &str = include_str!("data/spinner-u5/Bottle_opener_Flaschenoeffner.txt");

const TOOLS: [&str; 5] = [
    "ENTFRAESER_D8_N_90GRAD",
    "BOHRER_D3.3_VHM",
    "ENTFRAESER_D8_90GRAD",
    "SCHAFT_D4_N_AP20",
    "SCHAFT_D8_N_AP20",
];

fn control(m: &forge_machine::MachineDef) -> Control {
    let mut c = Control::for_dialect(m, "siemens-840d", 1).unwrap();
    c.tool_names = TOOLS
        .iter()
        .enumerate()
        .map(|(i, n)| (n.to_string(), i as u32 + 1))
        .collect();
    c.m_no_motion = vec![27, 29, 50, 51, 65];
    c
}

#[test]
fn the_dataset_programs_read() {
    let m = builtin_named("generic-trunnion-bc").unwrap();
    let c = control(&m);
    assert!(c.lex.native840d);
    for (name, text) in [("keychain", KEYCHAIN), ("bottle", BOTTLE)] {
        let (bs, _) = gcode::run(text, &c).unwrap_or_else(|e| panic!("{name} line {}: {}", e.line, e.what));
        let n = |f: &dyn Fn(&Ev) -> bool| bs.iter().filter(|b| f(&b.ev)).count();
        eprintln!(
            "{name}: {} events, {} rapids, {} feeds, {} arcs, {} machine moves, {} tool changes, {} not modelled",
            bs.len(),
            n(&|e| matches!(e, Ev::Rapid(_))),
            n(&|e| matches!(e, Ev::Feed { .. })),
            n(&|e| matches!(e, Ev::Arc { .. })),
            n(&|e| matches!(e, Ev::Machine(_))),
            n(&|e| matches!(e, Ev::Tool { .. })),
            n(&|e| matches!(e, Ev::NotModelled { .. })),
        );
    }
}

/// Block events of a line (1-based).
fn at(bs: &[gcode::Block], line: usize) -> Vec<&Ev> {
    bs.iter().filter(|b| b.line == line).map(|b| &b.ev).collect()
}

#[test]
fn mcall_cycle81_drills_at_the_next_position_and_stops() {
    let m = builtin_named("generic-trunnion-bc").unwrap();
    let c = control(&m);
    let (bs, _) = gcode::run(KEYCHAIN, &c).unwrap();
    // N6320 MCALL CYCLE81(RTP 10, RFP 0, SDIS 3, DP -6.1835) (PGZ 2.1.3); N6330 X-12.5 Y-58.:
    // to the point, rapid to RFP + SDIS, feed to DP, rapid to RTP (Q in 10^-4 mm)
    let ev = at(&bs, 633);
    let z = |e: &Ev| match e {
        Ev::Rapid(to) => ('R', to[2]),
        Ev::Feed { to, .. } => ('F', to[2]),
        _ => ('?', None),
    };
    let zs: Vec<_> = ev.iter().map(|e| z(e)).collect();
    assert_eq!(
        zs,
        vec![
            ('R', Some(100000)),
            ('R', Some(30000)),
            ('F', Some(-61835)),
            ('R', Some(100000))
        ],
        "{ev:?}"
    );
    // the lone MCALL ends it: the retract to home is a plain move
    assert!(at(&bs, 637).iter().all(|e| !matches!(e, Ev::Feed { .. })));
}

#[test]
fn the_machine_settings_are_reported_not_modelled() {
    let m = builtin_named("generic-trunnion-bc").unwrap();
    let c = control(&m);
    let (bs, _) = gcode::run(BOTTLE, &c).unwrap();
    let mut seen: Vec<(String, &str)> = bs
        .iter()
        .filter_map(|b| match &b.ev {
            Ev::NotModelled { path, check } => Some((path.clone(), *check)),
            _ => None,
        })
        .collect();
    seen.sort();
    seen.dedup();
    eprintln!("{seen:?}");
    for p in ["cycle800", "cycle832", "M27", "M29", "M50", "M51", "M65"] {
        assert!(seen.iter().any(|(q, _)| q.starts_with(p)), "{p}: {seen:?}");
    }
}

#[test]
fn the_dataset_programs_simulate() {
    let m = builtin_named("generic-trunnion-bc").unwrap();
    let mut s = Setup::new(&m, control(&m)).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -400.0, 0.0, 0.0])];
    s.tools = (1..=5)
        .map(|t| SimTool {
            t,
            name: TOOLS[t as usize - 1].into(),
            tip: [0.0, 0.0, -100.0],
            body: vec![(4.0, 0.0, 25.0), (12.0, 25.0, 80.0)],
            cut: Cut::Mill { r: 4.0, corner: 0.0 },
        })
        .collect();
    s.h = (1..=5).map(|t| (t, 100.0)).collect();
    for (name, text) in [("keychain", KEYCHAIN), ("bottle", BOTTLE)] {
        let tl = sim::simulate(text, &s).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let un = sim::unchecked(&m, &tl);
        eprintln!(
            "{name}: {} segments, t {:.2} min, {} violations",
            tl.segments.len(),
            tl.t_min,
            tl.violations.len()
        );
        for u in &un {
            eprintln!("  {} {} line {} {}", u.path, u.why, u.line, u.check);
        }
        for p in ["cycle800", "cycle832"] {
            assert!(un.iter().any(|u| u.path.starts_with(p)), "{name} {p}");
        }
        // the program ends with T0 M6: the spindle empty
        assert!(
            tl.segments.last().is_some_and(|g| g.tool.is_none())
                || tl.segments.iter().rev().any(|g| g.tool.is_none())
        );
    }
}

/// A refusal: the program's first refused line and what the refusal names.
fn refused(text: &str) -> (usize, String) {
    let m = builtin_named("generic-trunnion-bc").unwrap();
    let e = gcode::run(text, &control(&m))
        .err()
        .unwrap_or_else(|| panic!("read: {text}"));
    (e.line, e.what)
}

const HEAD: &str = "N10 G17 G710 G94 G90\nN20 T=\"SCHAFT_D8_N_AP20\"\nN30 M6\n";

#[test]
fn what_is_outside_the_subset_is_refused_by_name() {
    for (body, word) in [
        ("N40 TRAORI(1)\n", "TRAORI"),
        ("N40 AROT Z30.\n", "AROT"),
        ("N40 TRANS X10.\n", "TRANS"),
        ("N40 D1\nN50 G1 X=10.+_A F100.\n", "10.+_A"),
        ("N40 D2\n", "D2"),
        ("N40 G0 Z50.\n", "Z move"),
        ("N40 D1\nN50 SUPA G0 Z100.\n", "SUPA"),
        ("N40 T=\"NO_SUCH_TOOL\"\n", "NO_SUCH_TOOL"),
        ("N40 TURN=1\n", "TURN"),
        ("N40 IF _A>1 GOTOF L1\n", "IF"),
        ("N40 MCALL CYCLE83(10.,0.,3.,-6.)\n", "CYCLE83"),
    ] {
        let (line, what) = refused(&format!("{HEAD}{body}"));
        assert!(line >= 4, "{body}: line {line}: {what}");
        assert!(what.contains(word), "{body}: {what}");
    }
}

#[test]
fn the_offset_rules() {
    let m = builtin_named("generic-trunnion-bc").unwrap();
    let c = control(&m);
    // after M6 a D then Z: the loaded tool's length (G43 H of its number)
    let (bs, _) = gcode::run(&format!("{HEAD}N40 G0 X0 Y0 Z50. D1\n"), &c).unwrap();
    assert!(bs.iter().any(|b| matches!(b.ev, Ev::Length(Some(5)))), "{bs:?}");
    // D0 then SUPA Z: no offset, machine coordinates
    let (bs, _) = gcode::run(&format!("{HEAD}N40 SUPA G0 Z100. D0\n"), &c).unwrap();
    assert!(bs.iter().any(|b| matches!(b.ev, Ev::Length(None))));
    assert!(bs.iter().any(|b| matches!(&b.ev, Ev::Machine(v) if v.len() == 1)));
    // T0 then M6 empties the spindle
    let (bs, _) = gcode::run(&format!("{HEAD}N40 T0\nN50 M6\n"), &c).unwrap();
    assert!(bs.iter().any(|b| matches!(b.ev, Ev::Tool { t: 0, .. })));
}

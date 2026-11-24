//! `[[machine.probe]]`, `[[machine.setter]]` and `[[machine.homing]]`: read, validated and cited.
use forge_machine::def::{MachineDef, Side};

const VMC: &str = include_str!("../data/machines/generic.toml");

/// generic.toml with `extra` appended to its last machine, and that machine's name and a body on
/// its table side.
fn last_with(extra: &str) -> Result<MachineDef, String> {
    MachineDef::parse_all(&format!("{VMC}\n{extra}"))
        .map(|v| v.last().cloned().expect("a machine"))
        .map_err(|e| e.to_string())
}

fn table_body() -> String {
    let d = MachineDef::parse_all(VMC).unwrap().pop().unwrap();
    d.bodies
        .iter()
        .find(|b| b.link == "base" || d.joint(&b.link).is_some_and(|j| j.side == Side::Table))
        .expect("a table-side body")
        .name
        .clone()
}

fn head_body() -> Option<String> {
    let d = MachineDef::parse_all(VMC).unwrap().pop().unwrap();
    d.bodies
        .iter()
        .find(|b| d.joint(&b.link).is_some_and(|j| j.side == Side::Head))
        .map(|b| b.name.clone())
}

#[test]
fn probe_setter_and_homing_read() {
    let tb = table_body();
    let d = last_with(&format!(
        "[[machine.probe]]\nname = \"p\"\ndevice = \"x\"\noffer = \"option\"\nt = 20\n\
         [[machine.setter]]\nname = \"s\"\ndevice = \"y\"\nbody = \"{tb}\"\nat = [1, 2, 3]\ndir = [0, 0, 2]\n\
         [[machine.homing]]\norder = [[\"Z\"], [\"X\", \"Y\"]]\n"
    ))
    .unwrap();
    assert_eq!(d.probes[0].t, Some(20));
    assert_eq!(d.setters[0].dir, [0.0, 0.0, 1.0]);
    assert_eq!(
        d.homing,
        Some(vec![
            vec!["Z".to_string()],
            vec!["X".to_string(), "Y".to_string()]
        ])
    );
    let paths = d.cite_paths();
    for p in ["probe.p", "setter.s.at", "setter.s.dir", "homing"] {
        assert!(paths.iter().any(|q| q == p), "{p}");
    }
}

#[test]
fn refusals() {
    let tb = table_body();
    for (extra, want) in [
        ("[[machine.probe]]\nname = \"p\"\ndevice = \"x\"\noffer = \"maybe\"\n".to_string(), "offer"),
        (
            "[[machine.setter]]\nname = \"s\"\ndevice = \"y\"\nbody = \"nope\"\nat = [0, 0, 0]\ndir = [0, 0, 1]\n"
                .to_string(),
            "nope",
        ),
        ("[[machine.homing]]\norder = [[\"Z\"], [\"Z\"]]\n".to_string(), "twice"),
        ("[[machine.homing]]\norder = [[\"Q\"]]\n".to_string(), "Q"),
        (
            format!("[[machine.setter]]\nname = \"s\"\ndevice = \"y\"\nbody = \"{tb}\"\nat = [0, 0, 0]\ndir = [0, 0, 0]\n"),
            "zero",
        ),
    ] {
        let e = last_with(&extra).unwrap_err();
        assert!(e.contains(want), "{extra}: {e}");
    }
    if let Some(hb) = head_body() {
        let e = last_with(&format!(
            "[[machine.setter]]\nname = \"s\"\ndevice = \"y\"\nbody = \"{hb}\"\nat = [0, 0, 0]\ndir = [0, 0, 1]\n"
        ))
        .unwrap_err();
        assert!(e.contains("table side"), "{e}");
    }
}

// ---- skip (probe) moves in the interpreter and the simulation ----

use forge_machine::gcode::{self, Control, Ev};
use forge_machine::library::builtin;
use forge_machine::sim::{self, Cut, Happening, Setup, SimTool};

fn skip_control(codes: &[u16]) -> Control {
    let mut c = Control::mill3(3, 1, false);
    c.skip = codes.to_vec();
    c
}

#[test]
fn a_skip_move_is_refused_unless_the_dialect_cites_it() {
    let e = gcode::run("G90 G0 X0. Y0. Z10.\nG38.2 Z-5. F100.\n", &skip_control(&[])).unwrap_err();
    assert!(e.what.contains("not in the subset"), "{e:?}");
    let e = gcode::run("G90 G0 X0. Y0. Z10.\nG31 Z-5. F100.\n", &skip_control(&[382])).unwrap_err();
    assert!(e.what.contains("not in the subset"), "{e:?}");
}

#[test]
fn a_skip_move_leaves_its_axes_unknown() {
    let c = skip_control(&[382]);
    let (bs, st) = gcode::run("G90 G0 X0. Y0. Z10.\nG38.2 Z-5. F100.\n", &c).unwrap();
    let sk = bs.iter().find_map(|b| match &b.ev {
        Ev::Skip { to, f, code } => Some((to.clone(), *f, *code)),
        _ => None,
    });
    assert_eq!(sk, Some((vec![None, None, Some(-5000)], 1000, 382)));
    assert_eq!(st.core.pos, vec![Some(0), Some(0), None]);
    // incremental from the unknown Z: refused; absolute: fine
    let e = gcode::run("G90 G0 X0. Y0. Z10.\nG38.2 Z-5. F100.\nG91 G0 Z1.\n", &c).unwrap_err();
    assert!(e.what.contains("unknown position"), "{e:?}");
    gcode::run("G90 G0 X0. Y0. Z10.\nG38.2 Z-5. F100.\nG90 G0 Z10.\n", &c).unwrap();
    // a bare axis word after it: no motion mode is assumed
    assert!(gcode::run("G90 G0 X0. Y0. Z10.\nG38.2 Z-5. F100.\nZ10.\n", &c).is_err());
    // incremental skip from a known start
    let (bs, _) = gcode::run("G90 G0 X0. Y0. Z10.\nG91 G38.2 Z-5. F100.\n", &c).unwrap();
    assert!(
        bs.iter()
            .any(|b| matches!(&b.ev, Ev::Skip { to, .. } if to[2] == Some(5000)))
    );
    for (prog, want) in [
        ("G90 G0 X0. Y0. Z10.\nG38.2 Z-5.\n", "no feed"),
        ("G38.2 Z-5. F100.\n", "unknown position"),
        ("G90 G0 X0. Y0. Z10.\nG1 G38.2 Z-5. F100.\n", "another motion"),
        ("G90 G0 X0. Y0. Z10.\nG38.2 F100.\n", "no axis word"),
    ] {
        let e = gcode::run(prog, &c).unwrap_err();
        assert!(e.what.contains(want), "{prog}: {e:?}");
    }
}

#[test]
fn the_simulation_sweeps_a_skip_to_its_end_and_counts_no_time_for_it() {
    let m = builtin().remove(0);
    let mut c = Control::for_dialect(&m, "fanuc", 1).unwrap();
    c.skip = vec![310];
    let mut s = Setup::new(&m, c).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -300.0])];
    s.tools = vec![SimTool {
        t: 1,
        name: "probe".into(),
        tip: [0.0, 0.0, -100.0],
        body: vec![(3.0, 0.0, 50.0), (20.0, 50.0, 100.0)],
        cut: Cut::Mill { r: 3.0, corner: 3.0 },
    }];
    s.h = vec![(1, 100.0)];
    let prog = "G90 G17 G21\nT1 M06\nG54\nG43 H1\nG0 X10. Y20. Z5.\nG31 Z-10. F200.\nG0 Z5.\n";
    let tl = sim::simulate(prog, &s).unwrap();
    let k = tl
        .marks
        .iter()
        .position(|m| {
            m.what
                == Happening::Skip {
                    code: 310,
                    stop: None,
                }
        })
        .unwrap();
    let ln = tl.marks[k].line;
    let seg = tl.segments.iter().find(|sg| sg.line == ln).unwrap();
    assert_eq!(seg.t_min, 0.0);
    assert!(seg.t_stop > 0.0);
    // swept to its programmed end (15 mm down)
    let z = tl.joints.iter().position(|j| j == "Z").unwrap();
    assert!((seg.q0[z] - seg.q1[z] - 15.0).abs() < 1e-9);
    let un = sim::unchecked(&m, &tl);
    assert!(un.iter().any(|u| u.path == "skip.end" && u.line == ln));
    assert!(sim::timeline_json(&tl).contains("\"skip\",\"g\":\"G31\",\"stop\":null"));
}

#[test]
fn the_trigger_is_the_balls_first_contact_with_the_stock_as_cut() {
    use forge_machine::removal;
    let m = builtin().remove(0);
    let mut c = Control::for_dialect(&m, "fanuc", 1).unwrap();
    c.skip = vec![310];
    let mut s = Setup::new(&m, c).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -300.0])];
    s.tools = vec![SimTool {
        t: 1,
        name: "probe".into(),
        tip: [0.0, 0.0, -100.0],
        body: vec![(3.0, 0.0, 50.0), (20.0, 50.0, 100.0)],
        cut: Cut::Probe { r: 3.0 },
    }];
    s.h = vec![(1, 100.0)];
    // the words are 200 mm below the part frame here: stock top at part z 200 (word Z0); the
    // probe comes down at (0, 0) from Z5 toward Z-10: it triggers at Z0
    let prog = "G90 G17 G21\nT1 M06\nG54\nG43 H1\nG0 X0. Y0. Z5.\nG31 Z-10. F200.\nG0 Z5.\nG1 Z-2. F100.\n";
    let tl = sim::simulate(prog, &s).unwrap();
    let r = removal::mill(&s, &tl, [-20.0, -20.0, 180.0], [20.0, 20.0, 200.0], 0.5).unwrap();
    assert_eq!(r.triggers.len(), 1, "{:?}", r.triggers);
    let tr = &r.triggers[0];
    assert!((tr.tip[2] - 200.0).abs() < 1e-6, "{tr:?}");
    assert!((tr.s - 5.0 / 15.0).abs() < 1e-6, "{tr:?}");
    // nothing removed by the probe
    assert!(r.removed.iter().all(|v| *v == 0.0));
    // taken to the segment's end, the retract starts inside the stock: a contact on it too
    assert_eq!(r.probe_hits.len(), 2);
    // settled: the retract starts at the trigger; only the plain feed into the stock afterwards
    // is a probe crash, 2 mm deep
    let text = prog;
    let (tl, r) = removal::mill_probing(&s, text, [-20.0, -20.0, 180.0], [20.0, 20.0, 200.0], 0.5).unwrap();
    assert!(
        tl.marks.iter().any(
            |m| matches!(m.what, Happening::Skip { stop: Some(u), .. } if (u - 5.0 / 15.0).abs() < 1e-6)
        )
    );
    // the stop is the stock model's: the moves after it stay reported, as estimated
    let un = sim::unchecked(&m, &tl);
    let se = un.iter().find(|u| u.path == "skip.end").unwrap();
    assert_eq!(se.why, "estimated");
    assert_eq!(r.triggers.len(), 1);
    assert_eq!(r.probe_hits.len(), 1, "{:?}", r.probe_hits);
    assert!((r.probe_hits[0].depth - 2.0).abs() < 1e-6, "{:?}", r.probe_hits);
}

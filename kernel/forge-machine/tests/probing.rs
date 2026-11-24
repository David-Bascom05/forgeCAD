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

/// The haas dialect with the Inspection Plus templates forge-post's probing/post adds.
fn haas_with_macros() -> String {
    let src = Control::dialect_src("haas").unwrap();
    if src.contains("probe_on") {
        return src.to_string();
    }
    src.replacen(
        "[dialect]\n",
        "[dialect]\nprobe_on = \"G65 P9832\"\nprobe_off = \"G65 P9833\"\nprobe_protected = \"G65 P9810 X{x} Y{y} Z{z} F{f}\"\nprobe_surface = \"G65 P9811 {axis}{at} Q{q}{set}\"\n",
        1,
    )
}

#[test]
fn inspection_plus_macros_are_read_and_simulated() {
    use forge_machine::gcode::Macro;
    use forge_machine::removal;
    let m = builtin().remove(0);
    let c = Control::for_machine(&m, &haas_with_macros(), 1).unwrap();
    assert_eq!(
        c.macros,
        vec![
            (9832, Macro::On),
            (9833, Macro::Off),
            (9810, Macro::Protected),
            (9811, Macro::Surface)
        ]
    );
    // refused where the dialect does not cite them
    let plain = Control::for_dialect(&m, "fanuc", 1).unwrap();
    assert!(gcode::run("G90 G0 X0. Y0. Z10.\nG65 P9810 Z5. F3000.\n", &plain).is_err());
    let mut s = Setup::new(&m, c.clone()).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -300.0])];
    s.tools = vec![SimTool {
        t: 20,
        name: "probe".into(),
        tip: [0.0, 0.0, -100.0],
        body: vec![(3.0, 0.0, 50.0), (20.0, 50.0, 100.0)],
        cut: Cut::Probe { r: 3.0 },
    }];
    s.h = vec![(20, 100.0)];
    // probe on; protected move down to Z5; measure Z toward Z0 with 4 mm search; then up
    let prog = "G90 G17 G21\nT20 M06\nG54\nG43 H20\nG0 X0. Y0. Z50.\nG65 P9832\nG0 Z50.\nG65 P9810 Z5. F3000.\nG65 P9811 Z0. Q4.\nG65 P9810 Z50.\nG65 P9833\nG91 G28 Z0.\nG90\n";
    // the off macro's rules: a G28 next, and no tool change while on
    for (bad, want) in [
        (
            "G90 G0 X0. Y0. Z50.\nG65 P9832\nG65 P9833\nG0 Z60.\n",
            "not a G28",
        ),
        (
            "G90 G0 X0. Y0. Z50.\nG65 P9832\nT2 M06\n",
            "tool change while the probe is on",
        ),
    ] {
        let e = gcode::run(bad, &c).unwrap_err();
        assert!(e.what.contains(want), "{bad}: {e:?}");
    }
    let (bs, _) = gcode::run(prog, &c).unwrap();
    assert!(
        bs.iter()
            .any(|b| matches!(&b.ev, Ev::Surface { to, .. } if to[2] == Some(-4000)))
    );
    // a G91 word after the switch's unknown Z is refused
    assert!(gcode::run("G90 G0 X0. Y0. Z50.\nG65 P9832\nG91 G0 Z1.\n", &c).is_err());
    let (tl, r) = removal::mill_probing(&s, prog, [-20.0, -20.0, 180.0], [20.0, 20.0, 200.0], 0.5).unwrap();
    // the measure triggers at the stock top (word Z0 = part 200) after 5 of its 9 mm
    assert_eq!(r.triggers.len(), 1, "{:?}", r.triggers);
    assert!((r.triggers[0].tip[2] - 200.0).abs() < 1e-6);
    assert!((r.triggers[0].s - 5.0 / 9.0).abs() < 1e-6);
    assert!(r.probe_hits.is_empty(), "{:?}", r.probe_hits);
    // the measure returns to its start: the next protected move starts at Z5
    let after = tl.segments.iter().find(|sg| sg.line == 10).unwrap();
    let z = tl.joints.iter().position(|j| j == "Z").unwrap();
    let before = tl.segments.iter().find(|sg| sg.line == 8).unwrap();
    assert_eq!(after.q0[z], before.q1[z]);
    let un = sim::unchecked(&m, &tl);
    for p in ["probe.switch", "probe.measure_time"] {
        assert!(un.iter().any(|u| u.path == p), "{p}");
    }
    assert!(sim::timeline_json(&tl).contains("\"measure\""));
}

#[test]
fn homing_follows_the_published_order_or_says_it_is_generic() {
    use forge_machine::homing;
    let m = builtin().remove(0);
    let s = Setup::new(&m, Control::for_dialect(&m, "fanuc", 1).unwrap()).unwrap();
    let from: Vec<f64> = m
        .joints
        .iter()
        .map(|j| j.range.map_or(j.home, |r| 0.5 * (r[0] + r[1])))
        .collect();
    let g = homing::home(&s, &from, false, 0.0);
    assert!(!g.published);
    assert_eq!(g.steps.len(), 1);
    assert!(g.unchecked.iter().any(|u| u.path == "homing"));
    assert!(g.unchecked.iter().any(|u| u.path == "homing.start"));
    // Z first, then the rest one at a time; Z to its plus end
    let text = include_str!("../data/machines/generic.toml");
    let first = MachineDef::parse_all(text).unwrap()[0].name.clone();
    let mut defs = MachineDef::parse_all(text).unwrap();
    let mut d = defs.remove(0);
    assert_eq!(d.name, first);
    d.homing = Some(vec![vec!["Z".to_string()]]);
    let zi = d.joint_index("Z").unwrap();
    d.joints[zi].reference = Some(1);
    let s = Setup::new(&d, Control::for_dialect(&d, "fanuc", 1).unwrap()).unwrap();
    let h = homing::home(&s, &from, true, 0.0);
    assert!(h.published);
    assert_eq!(h.steps[0], vec![zi]);
    assert_eq!(h.steps.len(), d.joints.len());
    assert_eq!(h.end[zi], d.joints[zi].range.unwrap()[1]);
    // the first segment moves Z only
    let sg = &h.segments[0];
    for j in 0..d.joints.len() {
        if j != zi {
            assert_eq!(sg.q0[j], sg.q1[j]);
        }
    }
    assert!(h.unchecked.iter().any(|u| u.path == "homing.rest"));
    // a pose the caller knows is not reported
    assert!(!h.unchecked.iter().any(|u| u.path == "homing.start"));
    assert!(!h.unchecked.iter().any(|u| u.path == "joint.Z.ref"));
}

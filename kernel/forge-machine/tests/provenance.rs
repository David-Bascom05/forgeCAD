//! F1/F2: a required number may be omitted only when cited `unknown`; status `user`; every check
//! against an unknown or estimated value is reported as not checked, never passed silently.
use forge_machine::collide;
use forge_machine::def::{CiteStatus, MachineDef};
use forge_machine::gcode::Control;
use forge_machine::sim::{self, Setup, Violation};

/// generic-vmc3's record, as text.
fn vmc3() -> String {
    let all = include_str!("../data/machines/generic.toml");
    let a = all.find("[[machine]]").unwrap();
    let b = a + 1 + all[a + 1..].find("[[machine]]").unwrap();
    all[a..b].to_string()
}

const CITES: &str = r#"
[[machine.cite]]
path = "joint.Y.range"
status = "unknown"
basis = "not published"

[[machine.cite]]
path = "joint.Y.rapid"
status = "unknown"
basis = "not published: unbounded in time bounds"

[[machine.cite]]
path = "changer.pockets"
status = "unknown"
basis = "not published"

[[machine.cite]]
path = "joint.X.max_feed"
status = "estimate"
basis = "the same as Y's"

[[machine.cite]]
path = "spindle.S1.rpm"
status = "user"
"#;

/// Y's range and rapid and the changer's pockets omitted.
fn omitted() -> String {
    let s = vmc3();
    let s = s.replacen("range = [-200, 200]\n", "", 1);
    let s = s.replacen("rapid = 30000\n", "", 1);
    s.replacen("pockets = 24\n", "", 1)
}

#[test]
fn an_omitted_number_needs_an_unknown_cite() {
    let e = MachineDef::parse_all(&omitted()).unwrap_err().to_string();
    assert!(e.contains("range"), "{e}");
    assert!(e.contains("cite it unknown"), "{e}");
}

#[test]
fn unknown_numbers_are_none_and_user_is_a_status() {
    let m = &MachineDef::parse_all(&(omitted() + CITES)).unwrap()[0];
    let y = m.joint("Y").unwrap();
    assert_eq!((y.range, y.rapid), (None, None));
    assert_eq!(m.changer.as_ref().unwrap().pockets, None);
    assert_eq!(m.joint("X").unwrap().range, Some([-300.0, 300.0]));
    assert_eq!(m.cite_status("spindle.S1.rpm"), Some(CiteStatus::User));
    assert_eq!(m.cite_status("joint.X.max_feed"), Some(CiteStatus::Estimate));
    assert_eq!(m.cite_status("joint.Z.range"), None);
}

#[test]
fn checks_on_unknowns_and_estimates_are_reported_not_passed() {
    let m = &MachineDef::parse_all(&(omitted() + CITES)).unwrap()[0];
    let mut s = Setup::new(m, Control::for_dialect(m, "fanuc", 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -500.0])];
    // Y far past the built-in's 200: no overtravel claimed either way; T99: no pocket check
    let tl = sim::simulate("G0 X10. Y450. Z-10.\nG1 X20. F500.\nT99 M6\n", &s).unwrap();
    assert!(
        !tl.violations.iter().any(
            |v| matches!(v, Violation::Overtravel { joint, .. } if joint == "Y")
                || matches!(v, Violation::Pocket { .. })
        ),
        "{:?}",
        tl.violations
    );
    assert!(tl.t_min.is_finite());
    let u = sim::unchecked(m, &tl);
    let got: Vec<(&str, &str)> = u.iter().map(|u| (u.path.as_str(), u.why)).collect();
    for want in [
        ("joint.Y.range", "unknown"),
        ("joint.Y.rapid", "unknown"),
        ("joint.X.max_feed", "estimated"),
        ("changer.pockets", "unknown"),
    ] {
        assert!(got.contains(&want), "{want:?} not in {got:?}");
    }
    // published or user values are checked, not listed
    assert!(
        !got.iter()
            .any(|(p, _)| *p == "joint.Z.range" || *p == "spindle.S1.rpm"),
        "{got:?}"
    );
}

#[test]
fn a_body_with_no_envelope_is_left_out_and_reported() {
    let s = vmc3();
    let a = s.find("[[machine.body]]\nname = \"head\"").unwrap();
    let rest = &s[a..];
    let end = rest[1..]
        .find("[[machine.body]]")
        .map(|e| a + 1 + e)
        .unwrap_or(s.len());
    let head: String = s[a..end]
        .lines()
        .take_while(|l| !l.starts_with("[[machine.body.box]]") && !l.starts_with("[[machine.body.cyl]]"))
        .map(|l| format!("{l}\n"))
        .collect();
    let src = format!(
        "{}{}{}\n[[machine.cite]]\npath = \"body.head\"\nstatus = \"unknown\"\nbasis = \"no drawing\"\n",
        &s[..a],
        head,
        &s[end..]
    );
    let m = &MachineDef::parse_all(&src).unwrap()[0];
    assert!(m.bodies.iter().any(|b| b.name == "head" && b.prims.is_empty()));
    assert!(!collide::machine_bodies(m).iter().any(|b| b.name == "head"));
    let tl = sim::Timeline {
        segments: Vec::new(),
        marks: Vec::new(),
        violations: Vec::new(),
        t_min: 0.0,
        t_stop: 0.0,
        joints: Vec::new(),
        accel_missing: false,
    };
    let u = sim::unchecked(m, &tl);
    assert!(
        u.iter().any(|u| u.path == "body.head" && u.check == "collision"),
        "{u:?}"
    );
}

#[test]
fn m6_moves_to_the_change_position_and_that_move_is_checked() {
    let m = &forge_machine::library::builtin_named("generic-vmc3").unwrap();
    assert_eq!(m.changer.as_ref().unwrap().position, vec![("Z".to_string(), 0.0)]);
    let mut s = Setup::new(m, Control::for_dialect(m, "fanuc", 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -500.0])];
    let tl = sim::simulate("G0 X0 Y0 Z-100.\nT2 M6\n", &s).unwrap();
    let z = m.joint_index("Z").unwrap();
    let up: Vec<&sim::Segment> = tl.segments.iter().filter(|sg| sg.line == 2).collect();
    assert_eq!(up.len(), 1, "{:?}", tl.segments);
    assert_eq!((up[0].q0[z], up[0].q1[z]), (-600.0, 0.0));
    assert!(up[0].t_min > 0.0);
    assert!(
        !sim::unchecked(m, &tl)
            .iter()
            .any(|u| u.path == "changer.position")
    );
    // a changer that does not state its move: the move is not checked, and the reply says so
    let mut m2 = m.clone();
    m2.changer.as_mut().unwrap().position.clear();
    let mut s2 = Setup::new(&m2, Control::for_dialect(&m2, "fanuc", 1).unwrap()).unwrap();
    s2.wcs = s.wcs.clone();
    let tl2 = sim::simulate("G0 X0 Y0 Z-100.\nT2 M6\n", &s2).unwrap();
    assert!(!tl2.segments.iter().any(|sg| sg.line == 2));
    assert!(
        sim::unchecked(&m2, &tl2)
            .iter()
            .any(|u| u.path == "changer.position" && u.why == "unknown")
    );
}

#[test]
fn the_machine_states_its_peck_parameters_and_the_interpreter_uses_them() {
    let src = vmc3().replacen(
        "clearance = 1.0\n",
        "clearance = 1.0\npeck_clear = 1.0\nchip_lift = 0.3\n",
        1,
    );
    let m = &MachineDef::parse_all(&src).unwrap()[0];
    assert_eq!((m.peck_clear, m.chip_lift), (Some(1.0), Some(0.3)));
    let c = Control::for_dialect(m, "fanuc", 1).unwrap();
    assert_eq!((c.peck_clear, c.chip_lift), (1.0, 0.3));
    let prog = "G0 X0 Y0 Z10.\nG98 G83 X0 Y0 Z-6. R2. Q3. F100.\nG80\n";
    let (blocks, _) = forge_machine::gcode::run(prog, &c).unwrap();
    // after the peck to -4 it comes back down to -4 + 1.0 (forge-post's 0.5 would give -3.5)
    let text = format!("{blocks:?}");
    assert!(text.contains("-3000") && !text.contains("-3500"), "{text}");
    assert!(sim::unchecked_cycles(m, prog).is_empty());
    // not stated: forge-post's numbers, and the reply says so
    let g = &forge_machine::library::builtin_named("generic-vmc3").unwrap();
    let u = sim::unchecked_cycles(
        g,
        "(G83 in a comment)\nG98 G83 X0 Y0 Z-6. R2. Q3. F100.\nG73 X1.\n",
    );
    let got: Vec<(&str, usize)> = u.iter().map(|u| (u.path.as_str(), u.line)).collect();
    assert_eq!(got, vec![("machine.peck_clear", 2), ("machine.chip_lift", 3)]);
}

#[test]
fn entering_a_zone_is_a_violation_on_lines_and_arcs() {
    // a zone over X and Y joint values: (-10, 10) × (40, 60)
    let src = vmc3()
        + "\n[[machine.zone]]\nname = \"clamp\"\njoints = [\"X\", \"Y\"]\nmin = [-10, 40]\nmax = [10, 60]\n";
    let m = &MachineDef::parse_all(&src).unwrap()[0];
    let mut s = Setup::new(m, Control::for_dialect(m, "fanuc", 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -500.0])];
    let zone_lines = |prog: &str| -> Vec<usize> {
        sim::simulate(prog, &s)
            .unwrap()
            .violations
            .iter()
            .filter_map(|v| match v {
                Violation::Zone { line, zone, .. } if zone == "clamp" => Some(*line),
                _ => None,
            })
            .collect()
    };
    // a line through it; one along its edge (the boundary is travel); one past it
    assert_eq!(zone_lines("G0 X-50. Y50. Z0\nG1 X50. F1000.\n"), vec![2]);
    assert_eq!(
        zone_lines("G0 X-50. Y40. Z0\nG1 X50. F1000.\n"),
        Vec::<usize>::new()
    );
    assert_eq!(
        zone_lines("G0 X-50. Y70. Z0\nG1 X50. F1000.\n"),
        Vec::<usize>::new()
    );
    // an arc whose ends are outside but whose top passes through it (centre (0, 0), r 50)
    assert_eq!(
        zone_lines("G0 X50. Y0 Z0\nG3 X-50. Y0 I-50. J0 F1000.\n"),
        vec![2]
    );
    // the same circle the other way round misses it
    assert_eq!(
        zone_lines("G0 X50. Y0 Z0\nG2 X-50. Y0 I-50. J0 F1000.\n"),
        Vec::<usize>::new()
    );
    // r 39: passes under it
    assert_eq!(
        zone_lines("G0 X39. Y0 Z0\nG3 X-39. Y0 I-39. J0 F1000.\n"),
        Vec::<usize>::new()
    );
}

#[test]
fn a_manual_changer_has_no_pockets_and_stops_for_the_swap() {
    let src = vmc3()
        .replacen("pockets = 24\n", "kind = \"manual\"\n", 1)
        .replacen("max_tool_kg = 8\n", "", 1);
    let m = &MachineDef::parse_all(&src).unwrap()[0];
    let c = m.changer.as_ref().unwrap();
    assert!(c.manual && c.pockets.is_none() && c.max_tool_kg.is_none());
    assert!(!m.cite_paths().iter().any(|p| p == "changer.pockets"));
    let mut s = Setup::new(m, Control::for_dialect(m, "fanuc", 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -500.0])];
    let tl = sim::simulate("G0 X0 Y0 Z0\nT99 M6\n", &s).unwrap();
    assert!(
        !tl.violations
            .iter()
            .any(|v| matches!(v, Violation::Pocket { .. }))
    );
    assert!(
        tl.marks
            .iter()
            .any(|mk| mk.line == 2 && mk.what == sim::Happening::Stop { optional: false })
    );
    assert!(!sim::unchecked(m, &tl).iter().any(|u| u.path == "changer.pockets"));
    // an unknown kind is refused
    assert!(
        MachineDef::parse_all(&vmc3().replacen("pockets = 24\n", "pockets = 24\nkind = \"carousel\"\n", 1))
            .is_err()
    );
}

/// generic-lathe's record, as text.
fn lathe() -> String {
    let all = include_str!("../data/machines/generic.toml");
    let a = all.find("name = \"generic-lathe\"").unwrap();
    let a = all[..a].rfind("[[machine]]").unwrap();
    let b = a + 1 + all[a + 1..].find("[[machine]]").unwrap();
    all[a..b].to_string()
}

#[test]
fn a_manual_tool_post_swaps_by_hand() {
    let src = lathe();
    let a = src.find("[[machine.turret]]").unwrap();
    let b = a + src[a..].find("index_axis").unwrap();
    let b = b + src[b..].find('\n').unwrap() + 1;
    let src = format!(
        "{}[[machine.toolpost]]\nname = \"QC\"\nlink = \"X\"\ncenter = [270, 0, 0]\nchange_s = 20\n{}",
        &src[..a],
        &src[b..]
    );
    let m = &MachineDef::parse_all(&src).unwrap()[0];
    let tp = &m.turrets[0];
    assert!(tp.manual && tp.stations.is_none() && tp.index_s == Some(20.0));
    let d = "[dialect]\nname = \"t\"\ndecimals = 3\nfeed_decimals = 3\ndwell = \"G04 P{ms}\"\n";
    let mut s = Setup::new(m, Control::for_machine(m, d, 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![100.0, -120.0])];
    s.geo = vec![(1, vec![0.0, 0.0]), (2, vec![0.0, 0.0]), (40, vec![0.0, 0.0])];
    s.start_tool = Some(1);
    let tl = sim::simulate("T0101\nT0202\nT4040\n", &s).unwrap();
    // T40 is no pocket error on a tool post; T01 (already in) is no swap; T02 and T40 are
    assert!(
        !tl.violations
            .iter()
            .any(|v| matches!(v, Violation::Pocket { .. })),
        "{:?}",
        tl.violations
    );
    let stops: Vec<usize> = tl
        .marks
        .iter()
        .filter(|mk| mk.what == sim::Happening::Stop { optional: false })
        .map(|mk| mk.line)
        .collect();
    assert_eq!(stops, vec![2, 3]);
    assert!(
        !sim::unchecked(m, &tl)
            .iter()
            .any(|u| u.path.ends_with(".stations"))
    );
}

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

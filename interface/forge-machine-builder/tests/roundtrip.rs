//! The writer against the reader over every built-in machine and template.
use forge_machine::def::MachineDef;
use forge_machine_builder::{make, write};

fn all() -> Vec<MachineDef> {
    let mut v = forge_machine::library::builtin();
    for t in make::Template::ALL {
        v.push(t.source());
    }
    v
}

#[test]
fn parse_of_write_is_the_machine() {
    for m in all() {
        let t = write::machine(&m).unwrap();
        let back = MachineDef::parse_all(&t).unwrap_or_else(|e| panic!("{}: {e}\n{t}", m.name));
        assert_eq!(back, vec![m.clone()], "{}", m.name);
    }
}

#[test]
fn write_is_byte_stable() {
    for m in all() {
        let t = write::file("header", std::slice::from_ref(&m)).unwrap();
        let back = MachineDef::parse_all(&t).unwrap();
        assert_eq!(write::file("header", &back).unwrap(), t, "{}", m.name);
    }
}

#[test]
fn awkward_numbers_and_strings() {
    let mut m = forge_machine::library::builtin_named("generic-vmc3").unwrap();
    m.clearance = 0.1 + 0.2;
    m.joints[0].range = Some([-1e-7, 123456789.125]);
    m.note = "a \"quoted\" note\\with\tescapes\nand a # hash".into();
    let t = write::machine(&m).unwrap();
    assert_eq!(MachineDef::parse_all(&t).unwrap(), vec![m.clone()]);
    m.note = "bell\u{7}".into();
    assert!(write::machine(&m).is_err());
}

/// F1: a number the user does not know is absent and cited unknown; it round-trips as UNKNOWN,
/// never as a default.
#[test]
fn unknown_numbers_round_trip_as_unknown() {
    use forge_machine::def::{Cite, CiteStatus};
    let mut m = forge_machine::library::builtin_named("generic-vmc3").unwrap();
    m.joints[0].rapid = None;
    m.spindles[0].rpm = None;
    for p in ["joint.Y.rapid", "spindle.S1.rpm"] {
        m.cites.push(Cite {
            path: p.into(),
            status: CiteStatus::Unknown,
            source: String::new(),
            taken: String::new(),
            value: String::new(),
            basis: "not known".into(),
            note: String::new(),
        });
    }
    m.cites.push(Cite {
        path: "joint.X.range".into(),
        status: CiteStatus::User,
        source: String::new(),
        taken: String::new(),
        value: String::new(),
        basis: String::new(),
        note: String::new(),
    });
    let t = write::machine(&m).unwrap();
    assert!(
        !t.contains("rapid = 30000\nmax_feed = 10000\naccel = 3000\n\n[[machine.joint]]\nname = \"X\""),
        "{t}"
    );
    let back = MachineDef::parse_all(&t).unwrap_or_else(|e| panic!("{e}\n{t}"));
    assert_eq!(back, vec![m.clone()]);
    assert_eq!(back[0].joints[0].rapid, None);
}

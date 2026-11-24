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
        Some(vec![vec!["Z".to_string()], vec!["X".to_string(), "Y".to_string()]])
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

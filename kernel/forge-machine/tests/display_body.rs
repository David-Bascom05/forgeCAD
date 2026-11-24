//! A display-only body (`check = false`): kept in the record and drawn, left out of every
//! collision check, and reported as such; the key refuses anything but true or false.
use forge_machine::MachineDef;
use forge_machine::collide;

fn with_body(extra: &str) -> Result<Vec<MachineDef>, forge_machine::def::DefError> {
    let src = include_str!("../data/machines/generic.toml");
    // a body on the base around the whole work area (an enclosure shell): it overlaps the table,
    // the head and every other body, so a checked one is in contact everywhere
    let i = src.find("[[machine.body]]").unwrap();
    let body = format!(
        "[[machine.body]]\nname = \"shell\"\nlink = \"base\"\n{extra}\n[[machine.body.box]]\nmin = [-2000, -2000, -2000]\nmax = [2000, 2000, 2000]\n\n"
    );
    MachineDef::parse_all(&format!("{}{}{}", &src[..i], body, &src[i..]))
}

#[test]
fn a_display_body_is_kept_drawn_and_never_checked() {
    let m = with_body("check = false").unwrap().remove(0);
    let b = m.bodies.iter().find(|b| b.name == "shell").unwrap();
    assert!(!b.check && !b.prims.is_empty(), "kept, with its envelope to draw");
    assert!(!collide::machine_bodies(&m).iter().any(|c| c.name == "shell"));
    // a checked body: in the collision set
    let c = with_body("").unwrap().remove(0);
    assert!(c.bodies.iter().find(|b| b.name == "shell").unwrap().check);
    assert!(collide::machine_bodies(&c).iter().any(|c| c.name == "shell"));
}

#[test]
fn a_display_body_is_reported_as_display_only() {
    let m = with_body("check = false").unwrap().remove(0);
    let c = forge_machine::gcode::Control::for_dialect(&m, "fanuc", 1).unwrap();
    let s = forge_machine::sim::Setup::new(&m, c).unwrap();
    let tl = forge_machine::sim::simulate("G90\n", &s).unwrap();
    let u = forge_machine::sim::unchecked(&m, &tl);
    let e = u.iter().find(|u| u.path == "body.shell").expect("reported");
    assert_eq!(e.why, "display");
}

#[test]
fn check_takes_only_true_or_false() {
    assert!(with_body("check = true").is_ok());
    for bad in ["check = \"no\"", "check = 0"] {
        assert!(with_body(bad).is_err(), "{bad}");
    }
}

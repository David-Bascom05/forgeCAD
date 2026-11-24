//! G5 and the machine's limits on CAM-MACHINE's generic lathe (forge-machine's built-in library).
mod common;
use forge_turn::machine::{Place, lathe};
use forge_turn::ops;

/// The built-in machine file that holds generic-lathe.
fn src() -> &'static str {
    forge_machine::library::BUILTIN
        .iter()
        .find(|(_, t)| t.contains("name = \"generic-lathe\""))
        .unwrap()
        .1
}

#[test]
fn the_generic_lathe_reads() {
    let m = lathe(src(), "generic-lathe").unwrap();
    assert_eq!(m.control, "fanuc-lathe");
    assert_eq!(
        (m.x, m.rpm, m.stations),
        (Some([-10.0, 250.0]), Some([50.0, 4500.0]), Some(12))
    );
    // Z's travel as forge-machine defines it (CAM-MACHINE moves its low end for a self-collision)
    let def = forge_machine::library::builtin()
        .into_iter()
        .find(|d| d.name == "generic-lathe")
        .unwrap();
    let zj = def.joints.iter().find(|j| j.letter == "Z").unwrap();
    assert_eq!(m.z, zj.range);
    assert!(forge_turn::post::Dialect::get(&m.control).is_ok());
    assert_eq!(forge_turn::machine::builtin("generic-lathe").unwrap(), m);
}

#[test]
fn placed_the_shaft_job_is_certified_in_full_and_off_the_travel_it_is_refused() {
    let mut s = common::setup();
    s.machine = Some(lathe(src(), "generic-lathe").unwrap());
    let job = common::job(&s);
    // unplaced: the spans only
    let rep = ops::certify(&s, &job, None).unwrap();
    assert!(rep.ok(), "{:?}", rep.refusals);
    assert!(rep.g5.starts_with("except G5"));
    // placed: every joint position
    let tools = [1, 2, 4, 5].iter().map(|&t| (t, [0.0, 0.0])).collect();
    s.place = Some(Place { z0: 100.0, tools });
    let rep = ops::certify(&s, &job, None).unwrap();
    assert!(rep.ok(), "{:?}", rep.refusals);
    assert_eq!(rep.g5, "full");
    // the work too close to the chuck end of Z: the face's tip below Z's lower limit
    if let Some(p) = s.place.as_mut() {
        p.z0 = -10.0;
    }
    let rep = ops::certify(&s, &job, None).unwrap();
    assert!(
        rep.refusals.iter().any(|r| r.rule == "travel"),
        "{:?}",
        rep.refusals
    );
}

#[test]
fn a_station_past_the_turret_and_a_speed_over_the_clamp_are_refused() {
    let mut s = common::setup();
    s.machine = Some(lathe(src(), "generic-lathe").unwrap());
    let mut job = common::job(&s);
    job[0].t = 13;
    if let ops::Spindle::Css { max_rpm, .. } = &mut job[1].spindle {
        *max_rpm = 5000.0;
    }
    let rep = ops::certify(&s, &job, None).unwrap();
    let m: Vec<_> = rep.refusals.iter().filter(|r| r.rule == "machine").collect();
    assert_eq!(m.len(), 2, "{:?}", rep.refusals);
}

#[test]
fn a_turn_mill_reads_as_a_lathe() {
    let mut def = forge_machine::library::builtin()
        .into_iter()
        .find(|d| d.name == "generic-lathe")
        .unwrap();
    def.kind = forge_machine::def::Kind::TurnMill;
    let m = forge_turn::machine::from_def(&def).unwrap();
    assert_eq!(m, lathe(src(), "generic-lathe").unwrap());
    // a machine that does not turn is refused by name
    def.kind = forge_machine::def::Kind::Vmc3;
    assert!(forge_turn::machine::from_def(&def).is_err());
}

#[test]
fn an_unknown_value_is_not_checked_and_says_so() {
    // a definition that cites the X travel and the turret's stations as unknown
    let mut s = common::setup();
    let mut m = lathe(src(), "generic-lathe").unwrap();
    m.x = None;
    m.stations = None;
    s.machine = Some(m);
    let tools = [1, 2, 4, 5].iter().map(|&t| (t, [0.0, 0.0])).collect();
    s.place = Some(Place { z0: 100.0, tools });
    let mut job = common::job(&s);
    // a station past any turret: with the stations unknown it is not refused
    job[0].t = 99;
    if let Some(p) = s.place.as_mut() {
        p.tools.push((99, [0.0, 0.0]));
    }
    let rep = ops::certify(&s, &job, None).unwrap();
    assert!(rep.ok(), "{:?}", rep.refusals);
    assert_eq!(
        rep.g5,
        "except G5 (not checked, unknown in the machine's definition: X travel, turret stations)"
    );
}

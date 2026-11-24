//! Two channels (sim::multi): the generic mill-turn's B head (channel 1: Z X Y B C) and its
//! sub-spindle (channel 2: Z2 C2), synchronised at waits, checked over every timing the bounds
//! allow.
use forge_machine::gcode::Control;
use forge_machine::library::builtin_named;
use forge_machine::multi::{self, Channel, MultiError};
use forge_machine::sim::{self, Setup};

const DIALECT: &str =
    "[dialect]\nname = \"t\"\ndecimals = 3\nfeed_decimals = 3\ndwell = \"G04 P{ms}\"\nwait_m = [100, 199]\n";

fn setups(m: &forge_machine::MachineDef) -> (Setup<'_>, Setup<'_>) {
    let mut s1 = Setup::new(m, Control::for_machine(m, DIALECT, 1).unwrap()).unwrap();
    // channel 1's letters are C Z X Y B: words are joint values (X a diameter)
    s1.wcs = vec![(54, vec![0.0; s1.joint_of.len()])];
    let mut s2 = Setup::new(m, Control::for_machine(m, DIALECT, 2).unwrap()).unwrap();
    s2.part_mount = "sub".into();
    s2.wcs = vec![(54, vec![0.0; s2.joint_of.len()])];
    (s1, s2)
}

/// The head parks at X-100 (joint -50) Z560 beside the sub-spindle's path, then leaves turning C
/// (C has no accel: no upper time). The sub-spindle comes to Z300 (clear), dwells 2 s, then to
/// Z400, where its head meets the parked B head (sub-head bottom at z 570).
const HEAD: &str = "G0 X-100. Z560.\nG0 X600. C90.\n";
const SUB: &str = "G0 Z300.\nG04 P2000\nG0 Z400.\n";

#[test]
fn a_contact_only_some_timing_allows_is_reported_and_said_to_be_conservative() {
    let m = builtin_named("generic-mill-turn").unwrap();
    let (s1, s2) = setups(&m);
    let run = multi::run(
        &m,
        &[
            Channel {
                ch: 1,
                setup: &s1,
                program: HEAD,
            },
            Channel {
                ch: 2,
                setup: &s2,
                program: SUB,
            },
        ],
        &[],
    )
    .unwrap();
    // at the lower bounds the head has left (line 2 ends) long before the sub-spindle's last move
    // starts: no contact at that timing ...
    let (t1, t2) = (&run.timelines[0], &run.timelines[1]);
    let head_gone = t1
        .segments
        .iter()
        .rev()
        .find(|s| s.line == 2)
        .map(|s| s.t0 + s.t_min)
        .unwrap();
    let sub_last = t2.segments.iter().find(|s| s.line == 3).unwrap().t0;
    assert!(head_gone + 1.0 < sub_last, "{head_gone} {sub_last}");
    // ... but the head's leaving has no upper time (C has no accel), so some timing within the
    // bounds keeps it there: the contact is reported, and the open window is said
    assert!(
        run.cross.iter().any(|c| c.b_line == 3
            && (c.a == "bhead" || c.b == "bhead")
            && (c.a == "subhead" || c.b == "subhead")),
        "{:?}",
        run.cross
    );
    assert!(
        run.conservative.iter().any(|c| c.starts_with("channel 1:")),
        "{:?}",
        run.conservative
    );
    assert!(run.t[1].is_infinite());
}

#[test]
fn a_wait_orders_the_channels_and_the_contact_goes() {
    let m = builtin_named("generic-mill-turn").unwrap();
    let (s1, s2) = setups(&m);
    // the sub-spindle waits for the head to be gone before its last move
    let head = format!("{HEAD}M100\n");
    let sub = "G0 Z300.\nM100\nG0 Z400.\n";
    let run = multi::run(
        &m,
        &[
            Channel {
                ch: 1,
                setup: &s1,
                program: &head,
            },
            Channel {
                ch: 2,
                setup: &s2,
                program: sub,
            },
        ],
        &[],
    )
    .unwrap();
    assert!(
        !run.cross.iter().any(|c| c.b_line == 3 || c.a_line == 3),
        "{:?}",
        run.cross
    );
}

#[test]
fn bounded_windows_that_meet_are_checked_and_ones_that_do_not_are_not() {
    let m = builtin_named("generic-mill-turn").unwrap();
    let (s1, s2) = setups(&m);
    // the head parks and stays: the sub-spindle's last move meets it, whatever the timing
    let run = multi::run(
        &m,
        &[
            Channel {
                ch: 1,
                setup: &s1,
                program: "G0 X-100. Z560.\n",
            },
            Channel {
                ch: 2,
                setup: &s2,
                program: SUB,
            },
        ],
        &[],
    )
    .unwrap();
    assert!(
        run.cross
            .iter()
            .any(|c| c.b_line == 3 && matches!(c.verdict, forge_machine::collide::Verdict::Hit { .. }))
    );
    // the sub-spindle stops short (Z300): nothing to report
    let run = multi::run(
        &m,
        &[
            Channel {
                ch: 1,
                setup: &s1,
                program: "G0 X-100. Z560.\n",
            },
            Channel {
                ch: 2,
                setup: &s2,
                program: "G0 Z300.\n",
            },
        ],
        &[],
    )
    .unwrap();
    assert!(run.cross.is_empty(), "{:?}", run.cross);
}

#[test]
fn crossed_waits_are_a_deadlock() {
    let m = builtin_named("generic-mill-turn").unwrap();
    let (s1, s2) = setups(&m);
    let e = multi::run(
        &m,
        &[
            Channel {
                ch: 1,
                setup: &s1,
                program: "M100\nM101\n",
            },
            Channel {
                ch: 2,
                setup: &s2,
                program: "M101\nM100\n",
            },
        ],
        &[],
    )
    .unwrap_err();
    assert_eq!(e, MultiError::Deadlock(vec![(1, 1, 100), (2, 1, 101)]));
    // a dialect without wait codes refuses them as before
    let plain = "[dialect]\nname = \"t\"\ndecimals = 3\nfeed_decimals = 3\ndwell = \"G04 P{ms}\"\n";
    let s = Setup::new(&m, Control::for_machine(&m, plain, 1).unwrap()).unwrap();
    assert!(sim::simulate("M100\n", &s).is_err());
}

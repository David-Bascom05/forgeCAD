//! MACHINE-LIB C's real machines (syil, grizzly, sherline, datron, ...): one sample job per
//! architecture through the simulation and the collision check. The machines' controls (Siemens
//! 828D, DATRON next) have no forge-post dialect yet, so the jobs are posted in `fanuc`: this tests
//! the machine definitions (kinematics, travel, envelopes), not the controls.
use forge_machine::collide;
use forge_machine::library::builtin_named;
use forge_machine::setup::SetupSpec;
use forge_machine::sim;

const FANUC: &str = include_str!("../../forge-post/data/fanuc.toml");

/// Runs a setup on a built-in machine: the timeline's violations and the contacts.
fn run(machine: &str, setup: &str, prog: &str) -> (Vec<sim::Violation>, Vec<collide::Contact>) {
    let m = builtin_named(machine).unwrap_or_else(|| panic!("no machine {machine}"));
    let spec = SetupSpec::parse(setup).unwrap();
    let s = spec.build(&m, FANUC).unwrap();
    let tl = sim::simulate(prog, &s).unwrap();
    let work = spec.work_bodies(&s);
    let rep = collide::check(&s, &tl, &work);
    (tl.violations, rep.contacts)
}

// A knee-less VMC (table X on saddle Y, head Z): SYIL X7 Pro. The table top centre is at
// X = -200, Y = -150 (the travel's centre); the nose is 530 above the table at Z = 0. The stock
// sits on a 100 mm riser (a 100 mm tool reaches 50 mm above the table at Z's bottom).
const SYIL_SETUP: &str = r#"
[setup]
machine = "syil-x7-pro"
dialect = "fanuc"
program = "x.nc"
step = 1
start_tool = 1

[[setup.wcs]]
n = 54
x = -200
y = -150
z = -420             # Z0 at part z = 110 (the stock top): 530 - 420 = 110

[[setup.tool]]
t = 1
name = "flat 10"
tip = [0, 0, -100]
body = [5, 0, 30, 8, 30, 60, 20, 60, 100]
mill = [5, 0]
h = 100

[[setup.stock]]
min = [-50, -40, 100]
max = [50, 40, 110]

[[setup.fixture]]
min = [-60, -50, 0]
max = [60, 50, 100]
"#;

const SLOT: &str = "G90 G94 G17 G49 G40 G80\nG21\nT1 M06\nG54\nG43 H1\nS8000 M03\nG0 X-60. Y0 Z50.\nZ5.\nG1 Z-2. F300.\nX60. F800.\nG0 Z50.\nM05\nM30\n";

#[test]
fn syil_x7_slot_runs_clear() {
    let (v, c) = run("syil-x7-pro", SYIL_SETUP, SLOT);
    assert!(v.is_empty(), "{v:?}");
    assert!(c.is_empty(), "{c:?}");
}

#[test]
fn syil_x7_rapid_through_the_riser_is_caught() {
    // a rapid at Z-50 across the part: the tool passes through the riser
    let crash = SLOT.replace("G0 X-60. Y0 Z50.\nZ5.", "G0 X-90. Y0 Z50.\nZ-50.\nX90.\nZ5.");
    let (_, c) = run("syil-x7-pro", SYIL_SETUP, &crash);
    assert!(!c.is_empty(), "the rapid through the riser was not caught");
}

#[test]
fn syil_x7_overtravel_is_reported() {
    // X300 from the part's X0 (the travel centre) asks for X joint 100: past the + end
    let far = SLOT.replace("X60. F800.", "X300. F800.");
    let (v, _) = run("syil-x7-pro", SYIL_SETUP, &far);
    assert!(
        v.iter().any(|v| matches!(v, sim::Violation::Overtravel { joint, .. } if joint == "X")),
        "{v:?}"
    );
}

// A moving-gantry router (Y gantry, X carriage, Z on it; the part on the bed): DATRON neo.
// The bed centre is at X = -260, Y = -210; the nose is 225 above the bed at Z = 0.
const NEO_SETUP: &str = r#"
[setup]
machine = "datron-neo"
dialect = "fanuc"
program = "x.nc"
step = 1
start_tool = 1

[[setup.wcs]]
n = 54
x = -260
y = -210
z = -225             # Z0 at the bed top

[[setup.tool]]
t = 1
name = "flat 6"
tip = [0, 0, -40]
body = [3, 0, 12, 3, 12, 40]
mill = [3, 0]
h = 40

[[setup.stock]]
min = [-40, -30, 0]
max = [40, 30, 10]
"#;

#[test]
fn datron_neo_slot_runs_clear() {
    let prog = SLOT
        .replace("S8000", "S20000")
        .replace("Z-2.", "Z8.")
        .replace("X-60.", "X-50.")
        .replace("X60.", "X50.");
    let (v, c) = run("datron-neo", NEO_SETUP, &prog);
    assert!(v.is_empty(), "{v:?}");
    assert!(c.is_empty(), "{c:?}");
}

#[test]
fn datron_neo_plunge_through_the_stock_is_caught() {
    // a plunge to Z-25 (Z0 is the bed top): the tool goes into the bed
    let prog = SLOT
        .replace("S8000", "S20000")
        .replace("Z-2.", "Z-25.")
        .replace("X-60.", "X0")
        .replace("X60.", "X1.");
    let (_, c) = run("datron-neo", NEO_SETUP, &prog);
    assert!(!c.is_empty(), "the plunge into the bed was not caught");
}

//! MACHINE-LIB C's real machines (syil, grizzly, sherline, datron, ...): one sample job per
//! architecture through the simulation and the collision check. The machines' controls (Siemens
//! 828D, DATRON next) have no forge-post dialect yet, so the jobs are posted in `fanuc`: this tests
//! the machine definitions (kinematics, travel, envelopes), not the controls.
use forge_machine::collide;
use forge_machine::library::builtin_named;
use forge_machine::setup::SetupSpec;
use forge_machine::sim;

const FANUC: &str = include_str!("../../forge-post/data/fanuc.toml");

/// The work bodies the collision check sees: fixtures only. The stock is the removal model's (as
/// cut so far), as forge-machine-lang's run_setup does; the interval check would test the stock as
/// first given and flag every retract out of a cut.
fn fixtures(spec: &SetupSpec, s: &sim::Setup) -> Vec<collide::CBody> {
    spec.work_bodies(s)
        .into_iter()
        .filter(|b| b.role != collide::Role::Stock)
        .collect()
}

/// Runs a setup on a built-in machine: the timeline's violations and the contacts.
fn run(machine: &str, setup: &str, prog: &str) -> (Vec<sim::Violation>, Vec<collide::Contact>) {
    let m = builtin_named(machine).unwrap_or_else(|| panic!("no machine {machine}"));
    let spec = SetupSpec::parse(setup).unwrap();
    let s = spec.build(&m, FANUC).unwrap();
    let tl = sim::simulate(prog, &s).unwrap();
    let rep = collide::check(&s, &tl, &fixtures(&spec, &s));
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
        v.iter()
            .any(|v| matches!(v, sim::Violation::Overtravel { joint, .. } if joint == "X")),
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

// A fixed bridge over a moving Y table (X and Z on the bridge): Roland SRM-20. The table centre is
// at X = -101.6, Y = -76.2; the collet tip (gauge) is 130.75 above the table at Z = 0. No changer
// (manual change): the job starts with the tool in.
const SRM_SETUP: &str = r#"
[setup]
machine = "roland-srm-20"
dialect = "fanuc"
program = "x.nc"
step = 1
start_tool = 1

[[setup.wcs]]
n = 54
x = -101.6
y = -76.2
z = -100.75          # Z0 at part z = 30 (the stock top)

[[setup.tool]]
t = 1
name = "flat 3"
tip = [0, 0, -50]
body = [1.5, 0, 10, 3, 10, 50]
mill = [1.5, 0]
h = 50

[[setup.stock]]
min = [-50, -40, 0]
max = [50, 40, 30]
"#;

#[test]
fn roland_srm20_slot_runs_clear() {
    let prog = SLOT.replace("T1 M06\n", "").replace("S8000", "S6000");
    let (v, c) = run("roland-srm-20", SRM_SETUP, &prog);
    assert!(v.is_empty(), "{v:?}");
    assert!(c.is_empty(), "{c:?}");
}

#[test]
fn roland_srm20_m6_is_refused_without_a_changer() {
    // manual change is not in the format yet (FORMAT-GAPS row 8): M6 is an error
    let m = builtin_named("roland-srm-20").unwrap();
    let spec = SetupSpec::parse(SRM_SETUP).unwrap();
    let s = spec.build(&m, FANUC).unwrap();
    assert!(sim::simulate(&SLOT.replace("S8000", "S6000"), &s).is_err());
}

#[test]
fn roland_srm20_head_below_the_reach_is_caught() {
    // Z-35 from the stock top (part z 30): the tool goes 5 mm into the table
    let prog = SLOT
        .replace("T1 M06\n", "")
        .replace("S8000", "S6000")
        .replace("Z-2.", "Z-35.");
    let (_, c) = run("roland-srm-20", SRM_SETUP, &prog);
    assert!(!c.is_empty(), "the tool into the table was not caught");
}

// A 2-axis lathe (Z carriage, X cross slide as a diameter, the turret on X): Grizzly G0884. The
// turret's gauge face is 110 mm off the axis at X = 0 and 667 mm from the chuck face at Z = 0; the
// tool's tip is 110 mm in and 50 mm toward the chuck from it (offset 1 = minus that).
const G0884_SETUP: &str = r#"
[setup]
machine = "grizzly-g0884"
dialect = "lathe-example"
program = "x.nc"
step = 0.05

[[setup.wcs]]
n = 54
z = -567             # word Z0 at the bar's face, 100 mm out: 667 - 567 - 50 + 50 = 100
x = -110

[[setup.tool]]
t = 1
name = "OD turning, 0.8 nose"
tip = [-110, 0, -50]
body = [6, 0, 30, 12, 30, 110]
nose = [0.8, 0.8, 0.8]

[[setup.geo]]
n = 1
x = 110
z = 50

[[setup.stock]]
bar = [25, 0, 101]
"#;

const LATHE_DIALECT: &str =
    "[dialect]\nname = \"lathe-example\"\ndecimals = 3\nfeed_decimals = 3\ndwell = \"G04 P{ms}\"\n";

const TURN: &str = "G18 G21 G40\nT0101\nG50 S3000\nG96 S200 M03\nG0 X60. Z0\nG95 G1 X-2. F0.2\nG0 Z2.\nX40.\nG1 Z-30. F0.25\nX60.\nG0 Z5.\nX200. Z150.\nM05\nM30\n";

fn run_lathe(prog: &str) -> (Vec<sim::Violation>, Vec<collide::Contact>) {
    let m = builtin_named("grizzly-g0884").unwrap();
    let spec = SetupSpec::parse(G0884_SETUP).unwrap();
    let s = spec.build(&m, LATHE_DIALECT).unwrap();
    let tl = sim::simulate(prog, &s).unwrap();
    let rep = collide::check(&s, &tl, &fixtures(&spec, &s));
    (tl.violations, rep.contacts)
}

#[test]
fn grizzly_g0884_face_and_turn_runs_clear() {
    let (v, c) = run_lathe(TURN);
    assert!(v.is_empty(), "{v:?}");
    assert!(c.is_empty(), "{c:?}");
}

#[test]
fn grizzly_g0884_turret_into_the_chuck_is_caught() {
    // Z-120 runs the tool 20 mm past the chuck face, into the chuck
    let (_, c) = run_lathe(&TURN.replace("G1 Z-30. F0.25", "G1 Z-120. F0.25"));
    assert!(!c.is_empty(), "the move into the chuck was not caught");
}

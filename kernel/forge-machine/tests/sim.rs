use forge_machine::gcode::Control;
use forge_machine::library::builtin;
use forge_machine::sim::{self, Cut, Happening, Path, Setup, SimTool, Violation};

fn vmc() -> forge_machine::MachineDef {
    builtin().remove(0)
}

fn mill_tool(t: u32, l: f64, r: f64) -> SimTool {
    SimTool {
        t,
        name: format!("t{t}"),
        tip: [0.0, 0.0, -l],
        body: vec![(r, 0.0, 20.0), (r, 20.0, l)],
        cut: Cut::Mill { r, corner: 0.0 },
    }
}

fn vmc_setup(m: &forge_machine::MachineDef) -> Setup<'_> {
    let c = Control::for_dialect(m, "fanuc", 1).unwrap();
    let mut s = Setup::new(m, c).unwrap();
    // letters are Y X Z (the definition's order); the WCS origin 300 mm below the gauge at home
    assert_eq!(s.control.letters, vec!['Y', 'X', 'Z']);
    s.wcs = vec![(54, vec![0.0, 0.0, -300.0])];
    s.tools = vec![mill_tool(1, 100.0, 5.0)];
    s.h = vec![(1, 100.0)];
    s
}

#[test]
fn vmc_tip_lands_on_the_words() {
    let m = vmc();
    let s = vmc_setup(&m);
    let prog = "G90 G17 G21\nT1 M06\nG54\nG43 H1\nG0 X10. Y20. Z5.\nG1 Z-1. F600.\n";
    let tl = sim::simulate(prog, &s).unwrap();
    let last = tl.segments.last().unwrap();
    // q: Y = 20, X = 10, Z = -1 + -300 + 100
    assert_eq!(last.q1, vec![20.0, 10.0, -201.0]);
    let (tip, _) = forge_machine::kin::tip_in_part(&m, &last.q1, "spindle", "table", 100.0).unwrap();
    // gauge at 500 - 201 = 299, tip 199 in the base; the WCS z origin is at 500 - 300 = 200
    assert_eq!(tip, [10.0, 20.0, 199.0]);
    assert!(tl.violations.is_empty(), "{:?}", tl.violations);
    // the feed: 6 mm at 600 mm/min = 0.6 s; the change 2.5 s
    assert!((last.t_min - 0.6).abs() < 1e-12);
    assert!(
        tl.marks
            .iter()
            .any(|m| matches!(m.what, Happening::ToolChange { t: 1, seconds } if seconds == 2.5))
    );
    assert!(tl.t_stop >= tl.t_min);
}

#[test]
fn overtravel_and_feed_clamp_are_reported() {
    let m = vmc();
    let s = vmc_setup(&m);
    let prog = "T1 M06\nG43 H1\nG0 X350. Y0 Z50.\nG1 Y150. F20000.\n";
    let tl = sim::simulate(prog, &s).unwrap();
    assert!(
        tl.violations
            .iter()
            .any(|v| matches!(v, Violation::Overtravel { joint, .. } if joint == "X")),
        "{:?}",
        tl.violations
    );
    assert!(
        tl.violations.iter().any(
            |v| matches!(v, Violation::FeedClamped { joint, max, .. } if joint == "Y" && *max == 10000.0)
        )
    );
    // clamped: 150 mm at 10000 mm/min
    assert!((tl.segments.last().unwrap().t_min - 0.9).abs() < 1e-12);
}

#[test]
fn arc_runs_through_its_extremes() {
    let m = vmc();
    let s = vmc_setup(&m);
    // a full circle of radius 10 about (0, 0): passes X = -10 .. 10
    let prog = "T1 M06\nG43 H1\nG0 X10. Y0 Z5.\nG2 X10. Y0 I-10. J0 F1000.\n";
    let tl = sim::simulate(prog, &s).unwrap();
    let a = tl.segments.last().unwrap();
    match &a.path {
        Path::Arc { r, sweep, .. } => {
            assert_eq!(*r, 10.0);
            assert!((sweep + forge_math::dmath::TAU).abs() < 1e-15);
        }
        p => panic!("{p:?}"),
    }
    let mid = a.at(0.5);
    // clockwise from angle 0 by half a turn: (-10, 0)
    assert!((mid[1] + 10.0).abs() < 1e-12 && mid[0].abs() < 1e-12, "{mid:?}");
    // 2π·10 at 1000 mm/min
    assert!((a.t_min - forge_math::dmath::TAU * 10.0 / 1000.0 * 60.0).abs() < 1e-9);
}

#[test]
fn g28_goes_through_the_intermediate_point_home() {
    let m = vmc();
    let s = vmc_setup(&m);
    let prog = "T1 M06\nG43 H1\nG0 X10. Y0 Z5.\nG91 G28 Z0.\nG90\n";
    let tl = sim::simulate(prog, &s).unwrap();
    assert_eq!(tl.segments.last().unwrap().q1[2], 0.0);
}

#[test]
fn lathe_diameter_and_turret() {
    let m = builtin().remove(2);
    let lathe_dialect = "[dialect]\nname = \"t\"\ndecimals = 3\nfeed_decimals = 3\ndwell = \"G04 P{ms}\"\n";
    let c = Control::for_machine(&m, lathe_dialect, 1).unwrap();
    assert_eq!(c.letters, vec!['Z', 'X']);
    assert_eq!(c.diameter, Some('X'));
    let mut s = Setup::new(&m, c).unwrap();
    // the WCS: Z 0 at the part's face 100 mm out; X word 0 = spindle centre for a zero tool
    s.wcs = vec![(54, vec![100.0, -120.0])];
    // tool 1: tip 120 in from the turret face, 10 forward
    s.tools = vec![SimTool {
        t: 1,
        name: "OD".into(),
        tip: [-120.0, 0.0, 10.0],
        body: vec![(6.0, 0.0, 30.0)],
        cut: Cut::Nose {
            rn: 0.8,
            centre: [0.8, 0.8],
        },
    }];
    s.geo = vec![(1, vec![-10.0, 120.0])];
    let prog =
        "G18 G21\nT0101\nG50 S3000\nG96 S200 M03\nG0 X50. Z2.\nG95 G1 Z-20. F0.2\nG02 X60. Z-25. I5. K0\n";
    let tl = sim::simulate(prog, &s).unwrap();
    assert!(tl.violations.is_empty(), "{:?}", tl.violations);
    let rp = &tl.segments[0];
    // X50 is a 25 mm radius: q_X = 25 - 120 + 120 = 25; Z 2: q_Z = 2 + 100 - 10
    assert_eq!(rp.q1, vec![92.0, 25.0]);
    let (tip, _) = s.tip_in_part(&rp.q1, 1).unwrap();
    // radius 25, 2 mm off the part's face at z = 100
    assert_eq!(tip, [25.0, 0.0, 102.0]);
    assert!(
        tl.marks
            .iter()
            .any(|m| matches!(m.what, Happening::Index { station: 1, .. }))
    );
    // G96 S200 at radius 25: rpm = 1000·200/(2π·25) ≈ 1273, under the 3000 clamp; 22 mm at
    // 0.2 mm/rev
    let rpm = 1000.0 * 200.0 / (forge_math::dmath::TAU * 25.0);
    assert!(
        (tl.segments[1].t_min - 22.0 / 0.2 / rpm * 60.0).abs() < 1e-9,
        "{}",
        tl.segments[1].t_min
    );
}

#[test]
fn trunnion_tilts() {
    let m = builtin().remove(1);
    let c = Control::for_dialect(&m, "fanuc", 1).unwrap();
    assert_eq!(c.letters, vec!['X', 'Y', 'Z', 'B', 'C']);
    let mut s = Setup::new(&m, c).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -250.0, 0.0, 0.0])];
    s.h = vec![(1, 100.0)];
    let prog = "G43 H1\nG0 X0 Y0 Z100. B0 C0\nG0 B90. C45.\nG1 B-30. F3600.\n";
    let tl = sim::simulate(prog, &s).unwrap();
    assert!(tl.violations.is_empty(), "{:?}", tl.violations);
    // 120 degrees at 3600 deg/min = 2 s
    assert!((tl.segments.last().unwrap().t_min - 2.0).abs() < 1e-12);
    let j = sim::timeline_json(&tl);
    assert!(j.starts_with("{\"format\":\"forgecad.machine-sim/1\""));
}

#[test]
fn rapid_time_bound_and_exact_stop() {
    let m = vmc();
    let s = vmc_setup(&m);
    // a 45 mm rapid on X alone: 500 mm/s, 3000 mm/s²: t_min = 0.09 s; with exact stops the
    // trapezoid never reaches 500 mm/s (45 < 500²/3000), so 2·√(45/3000)
    let tl = sim::simulate("G0 X0 Y0 Z0\nX45.\n", &s).unwrap();
    let sg = tl.segments.last().unwrap();
    assert!((sg.t_min - 0.09).abs() < 1e-15);
    assert!(
        (sg.t_stop - 2.0 * (45.0f64 / 3000.0).sqrt()).abs() < 1e-12,
        "{}",
        sg.t_stop
    );
}

#[test]
fn css_facing_to_the_axis_meets_the_clamp() {
    let m = builtin().remove(2);
    let d = "[dialect]\nname = \"t\"\ndecimals = 3\nfeed_decimals = 3\ndwell = \"G04 P{ms}\"\n";
    let mut s = Setup::new(&m, Control::for_machine(&m, d, 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![100.0, -120.0])];
    s.tools = vec![SimTool {
        t: 1,
        name: "OD".into(),
        tip: [-120.0, 0.0, 10.0],
        body: vec![(6.0, 0.0, 30.0)],
        cut: Cut::Nose {
            rn: 0.8,
            centre: [0.8, 0.8],
        },
    }];
    s.geo = vec![(1, vec![-10.0, 120.0])];
    // face from radius 30 to the axis at 0.2 mm/rev, V = 200 m/min, clamp 2000 rpm
    let tl = sim::simulate("T0101\nG50 S2000\nG96 S200 M03\nG0 X60. Z0\nG95 G1 X0 F0.2\n", &s).unwrap();
    let k = 1000.0 * 200.0 / forge_math::dmath::TAU;
    let rc = k / 2000.0; // the radius below which the clamp holds
    // ∫ over r from 0 to 30 of max(1/2000, r/k) dr / 0.2, in minutes
    let want = (rc / 2000.0 + (30.0f64.powi(2) - rc * rc) / (2.0 * k)) / 0.2 * 60.0;
    assert!(
        (tl.segments.last().unwrap().t_min - want).abs() < 1e-9,
        "{} vs {want}",
        tl.segments.last().unwrap().t_min
    );
}

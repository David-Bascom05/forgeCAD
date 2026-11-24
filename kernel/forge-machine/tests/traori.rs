//! SINUMERIK TRAORI with the rotary axes held (3+2), as the RPTU/Spinner dataset's
//! Coordinate_system_representation_Achskreuz program uses it (tests/data/spinner-u5, CC BY 4.0),
//! on the spinner-u5-620 kinematics fitted from the same dataset's log. The simulated machine
//! positions are checked against the log (Zenodo 15735480, data_v1_0_1.csv, CC BY 4.0) on two
//! operations of one run (2024-03-12): one the fit used (B90 C0) and one it did not need (B90 C90,
//! a different frame chain).
use forge_machine::MachineDef;
use forge_machine::gcode::{self, Control, Ev};
use forge_machine::sim::{self, Cut, Setup, SimTool};

const ACHSKREUZ: &str = include_str!("data/spinner-u5/Coordinate_system_representation_Achskreuz.mpf");
const U5_620: &str = include_str!("data/spinner-u5/u5-620-kinematics.toml");

/// The tools and their gauge lengths (mm): from the fit for the two the log rows use; the others
/// only need to be plausible here.
const TOOLS: [(&str, f64); 4] = [
    ("PLAN_D50_N_AP4", 120.0),
    ("SCHAFT_D12_N_AP30", 250.1),
    ("SCHAFT_D6_N_AP15", 123.5),
    ("ENTFRAESER_D8_N_90GRAD", 137.0),
];

fn machine() -> MachineDef {
    MachineDef::parse_all(U5_620).unwrap().remove(0)
}

fn setup(m: &MachineDef) -> Setup<'_> {
    let mut c = Control::for_dialect(m, "siemens-840d", 1).unwrap();
    c.m_no_motion = vec![27, 29, 50, 51, 65];
    let mut s = Setup::new(m, c).unwrap();
    // G54 of this run, in the table's frame from the log's BCS at N10330-N10360 (x 64.6625,
    // y -16.2014, z 327.4325): as this setup's joint values at word 0 (X, Y; Z less the nose's 610)
    s.wcs = vec![(54, vec![64.6625, -16.2014, 327.4325 - 610.0, 0.0, 0.0])];
    s.tools = TOOLS
        .iter()
        .enumerate()
        .map(|(i, (n, _))| SimTool {
            t: i as u32 + 1,
            name: n.to_string(),
            tip: [0.0, 0.0, -100.0],
            body: vec![(3.0, 0.0, 20.0)],
            cut: Cut::Mill { r: 3.0, corner: 0.0 },
        })
        .collect();
    s.h = TOOLS
        .iter()
        .enumerate()
        .map(|(i, (_, l))| (i as u32 + 1, *l))
        .collect();
    // the programs start at the SUPA home: X-365 Y224 Z610 (this record's Z: 0)
    let j = |n: &str| m.joints.iter().position(|jt| jt.name == n).unwrap();
    s.start[j("X")] = -365.0;
    s.start[j("Y")] = 224.0;
    s.start[j("Z")] = 0.0;
    s
}

#[test]
fn the_3_plus_2_program_reads_and_its_frames_put_z_on_the_tool() {
    let m = machine();
    let s = setup(&m);
    let mut c = s.control.clone();
    c.tool_names = s.tools.iter().map(|t| (t.name.clone(), t.t)).collect();
    let (bs, _) = gcode::run(ACHSKREUZ, &c).unwrap_or_else(|e| panic!("line {}: {}", e.line, e.what));
    // every TRAORI operation's last frame: its Z the tool's axis in the table's frame,
    // R_z(C) R_y(90) e_z = (cos C, sin C, 0) at B 90 (the fitted senses; PGsl 12.4: AROT additive
    // about the current axes, right-handed)
    let mut frames = 0;
    let mut bc = (0.0f64, 0.0f64);
    let mut last: Option<[[f64; 3]; 3]> = None;
    let bi = c.idx('B').unwrap();
    let ci = c.idx('C').unwrap();
    for b in &bs {
        match &b.ev {
            Ev::Rapid(to) => {
                if let (Some(x), Some(y)) = (to[bi], to[ci]) {
                    bc = (x as f64 / 1e4, y as f64 / 1e4);
                }
            }
            // the operation's frame: the last before TRANS X0 Y0 Z0 resets it
            Ev::Frame(Some(f)) if *f != [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] => {
                last = Some(*f)
            }
            Ev::Frame(None) => {
                let f = last.take().unwrap();
                if bc.0 == 90.0 {
                    let (sn, cs) = forge_machine::kin::sin_cos_deg(bc.1);
                    let want = [cs, sn, 0.0];
                    for k in 0..3 {
                        assert!(
                            (f[k][2] - want[k]).abs() < 1e-9,
                            "line {}: {f:?} vs {want:?}",
                            b.line
                        );
                    }
                    frames += 1;
                }
            }
            _ => {}
        }
    }
    assert_eq!(frames, 21);
}

/// A segment on text line `line` (1-based, as the log's Program_line_number).
fn seg(tl: &sim::Timeline, line: usize) -> &sim::Segment {
    tl.segments
        .iter()
        .find(|g| g.line == line)
        .unwrap_or_else(|| panic!("no segment on line {line}"))
}

#[test]
fn the_machine_follows_the_logged_positions() {
    let m = machine();
    let s = setup(&m);
    let tl = sim::simulate(ACHSKREUZ, &s).unwrap_or_else(|e| panic!("{e}"));
    let j = |n: &str| m.joints.iter().position(|jt| jt.name == n).unwrap();
    // (text line, joint, logged machine position, this record's offset): along a feed one machine
    // axis moves; the other two are held at the logged values (machine Z = this record's + 610)
    for (line, axis, logged) in [
        // N10360 Y-5.001 (B90 C0, AROT Y90): 2024-03-12 08:58:13-15
        (1037, "X", -246.939),
        (1037, "Z", 362.7827 - 610.0),
        // N16060 X56.001 (B90 C90, AROT X-90 then Z90): 2024-03-12 09:04:34-35
        (1607, "Y", -18.6658),
        (1607, "Z", 331.9257 - 610.0),
    ] {
        let g = seg(&tl, line);
        let (a, b) = (g.q0[j(axis)], g.q1[j(axis)]);
        assert!((a - b).abs() < 1e-9, "line {line}: {axis} moves");
        // the fit's residuals and the intersecting-axes assumption: within 0.5 mm
        assert!(
            (a - logged).abs() < 0.5,
            "line {line} {axis}: {a:.4} vs logged {logged:.4}"
        );
        eprintln!(
            "line {line} {axis}: simulated {a:.4}, logged {logged:.4} ({:+.3})",
            a - logged
        );
    }
    assert_eq!(seg(&tl, 1037).q1[j("B")], 90.0);
    assert_eq!(seg(&tl, 1607).q1[j("C")], 90.0);
}

#[test]
fn what_3_plus_2_does_not_cover_is_refused() {
    let m = machine();
    let s = setup(&m);
    let mut c = s.control.clone();
    c.tool_names = s.tools.iter().map(|t| (t.name.clone(), t.t)).collect();
    let head = "N10 T=\"SCHAFT_D12_N_AP30\"\nN20 M6\nN30 G0 B90. C0.0\nN40 TRAORI\nN50 G54\n";
    for (body, word) in [
        ("N60 G0 B45.\n", "rotary"),
        ("N60 SUPA G0 Z0 D0\n", "SUPA"),
        ("N60 AROT X90. Y45.\n", "one axis"),
        ("N60 TRANS X10.\n", "TRANS X0 Y0 Z0"),
        ("N60 G41 G1 X0 Y0\n", "compensation"),
        ("N60 TRAORI\n", "TRAORI while"),
        ("N60 T=\"SCHAFT_D6_N_AP15\"\nN70 M6\n", "tool change"),
    ] {
        let e = gcode::run(&format!("{head}{body}"), &c).unwrap_err();
        assert!(e.what.contains(word), "{body}: {}", e.what);
    }
    // a frame outside TRAORI
    let e = gcode::run("N10 AROT Z30.\n", &c).unwrap_err();
    assert!(e.what.contains("outside TRAORI"), "{}", e.what);
}

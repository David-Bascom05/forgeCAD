//! Timing of the three RPTU/Spinner programs on the U5-620 kinematics fitted from the dataset's log
//! (tests/data/spinner-u5): t_min, t_stop and t_la against the logged run times. An analysis, not a
//! pass/fail test: it prints the table (cargo test --test timing_u5 -- --nocapture).
use forge_machine::MachineDef;
use forge_machine::def::JointType;
use forge_machine::gcode::Control;
use forge_machine::lookahead::{self, LookAhead};
use forge_machine::sim::{self, Cut, Setup, SimTool};

fn tool(t: u32, name: &str) -> SimTool {
    SimTool {
        t,
        name: name.into(),
        tip: [0.0, 0.0, -100.0],
        body: vec![(3.0, 0.0, 20.0)],
        cut: Cut::Mill { r: 3.0, corner: 0.0 },
    }
}

/// (name, program, tools with gauge lengths, G54 as this setup's joint values at word 0)
type Program = (&'static str, &'static str, Vec<(&'static str, f64)>, [f64; 3]);

fn programs() -> Vec<Program> {
    vec![
        (
            "Keychain",
            include_str!("data/spinner-u5/Keychain_Schluesselanhaenger.mpf"),
            vec![("ENTFRAESER_D8_N_90GRAD", 137.0), ("BOHRER_D3.3_VHM", 120.0)],
            // the log, 2024-03-20 08:22:55-57, N340: machine X Y Z 51.47777 -20.40535 378.72619
            // at words X-10.46 Y-22.721 Z0 (tool 1)
            [61.93777, 2.31565, 378.72619 - 610.0 - 137.0],
        ),
        (
            "Bottle opener",
            include_str!("data/spinner-u5/Bottle_opener_Flaschenoeffner.txt"),
            vec![
                ("SCHAFT_D8_N_AP20", 120.0),
                ("SCHAFT_D4_N_AP20", 120.0),
                ("ENTFRAESER_D8_90GRAD", 120.0),
            ],
            // 2024-03-20 07:01:03-04, N340 Z2.: machine -104.77315 -19.80911 310.48492 at X0 Y-7 Z2
            [-104.77315, -12.80911, 310.48492 - 610.0 - 2.0 - 120.0],
        ),
        (
            "Achskreuz (3+2)",
            include_str!("data/spinner-u5/Coordinate_system_representation_Achskreuz.mpf"),
            vec![
                ("PLAN_D50_N_AP4", 120.0),
                ("SCHAFT_D12_N_AP30", 250.1),
                ("SCHAFT_D6_N_AP15", 123.5),
                ("ENTFRAESER_D8_N_90GRAD", 137.0),
            ],
            // the table-frame G54 from the log's BCS (tests/traori.rs)
            [64.6625, -16.2014, 327.4325 - 610.0],
        ),
    ]
}

#[test]
fn the_table() {
    let base = MachineDef::parse_all(include_str!("data/spinner-u5/u5-620-kinematics.toml"))
        .unwrap()
        .remove(0);
    println!("| program | t_min | A mm/s² | t_stop | t_la 15 blocks | t_la 100 blocks |");
    for (name, text, tools, g54) in programs() {
        let mut first = true;
        for acc in [1000.0, 2000.0, 5000.0] {
            let mut m = base.clone();
            for j in m.joints.iter_mut() {
                j.accel = Some(match j.ty {
                    JointType::Linear => acc,
                    JointType::Rotary => 3600.0,
                });
            }
            let mut c = Control::for_dialect(&m, "siemens-840d", 1).unwrap();
            c.m_no_motion = vec![27, 29, 50, 51, 65];
            let mut s = Setup::new(&m, c).unwrap();
            s.wcs = vec![(54, vec![g54[0], g54[1], g54[2], 0.0, 0.0])];
            s.tools = tools
                .iter()
                .enumerate()
                .map(|(i, (n, _))| tool(i as u32 + 1, n))
                .collect();
            s.h = tools
                .iter()
                .enumerate()
                .map(|(i, (_, l))| (i as u32 + 1, *l))
                .collect();
            let jx = |n: &str| m.joints.iter().position(|jt| jt.name == n).unwrap();
            s.start[jx("X")] = -365.0;
            s.start[jx("Y")] = 224.0;
            let tl = sim::simulate(text, &s).unwrap_or_else(|e| panic!("{name}: {e}"));
            let la100 = lookahead::plan(
                &m,
                &tl,
                &LookAhead {
                    blocks: 100,
                    ..Default::default()
                },
            )
            .1;
            println!(
                "| {} | {} | {acc:.0} | {:.1} | {:.1} | {:.1} |",
                if first { name } else { "" },
                if first {
                    format!("{:.1}", tl.t_min)
                } else {
                    String::new()
                },
                tl.t_stop,
                tl.t_la,
                la100
            );
            first = false;
            // per line (t_min, t_la) for the per-operation comparison, at 2000 mm/s²
            if acc == 2000.0
                && let Ok(dir) = std::env::var("TIMING_DUMP")
            {
                let xyz: Vec<usize> = ["X", "Y", "Z"].iter().map(|n| jx(n)).collect();
                let dist: f64 = tl
                    .segments
                    .iter()
                    .map(|g| {
                        // chord length over 64 steps (arcs)
                        (0..64)
                            .map(|i| {
                                let (a, b) = (g.at(i as f64 / 64.0), g.at((i + 1) as f64 / 64.0));
                                xyz.iter().map(|j| (b[*j] - a[*j]).powi(2)).sum::<f64>().sqrt()
                            })
                            .sum::<f64>()
                    })
                    .sum();
                println!("    {name}: machine X Y Z travel {dist:.0} mm");
                let mut out = String::from("line,t_min,t_la,t_stop,kind\n");
                for (g, la) in tl.segments.iter().zip(&tl.seg_la) {
                    out.push_str(&format!(
                        "{},{},{},{},{:?}\n",
                        g.line, g.t_min, la, g.t_stop, g.kind
                    ));
                }
                std::fs::write(format!("{dir}/{}.csv", name.split(' ').next().unwrap()), out).unwrap();
            }
        }
    }
}

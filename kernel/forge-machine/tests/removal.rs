use forge_machine::gcode::Control;
use forge_machine::library::builtin;
use forge_machine::removal;
use forge_machine::sim::{self, Cut, Setup, SimTool};

fn vmc_setup(m: &forge_machine::MachineDef, r: f64, corner: f64) -> Setup<'_> {
    let mut s = Setup::new(m, Control::for_dialect(m, "fanuc", 1).unwrap()).unwrap();
    // WCS = the table's top centre: tip = 500 + Z + wcs_z when H holds the tool's length
    s.wcs = vec![(54, vec![0.0, 0.0, -500.0])];
    s.h = vec![(1, 100.0)];
    s.tools = vec![SimTool {
        t: 1,
        name: "t1".into(),
        tip: [0.0, 0.0, -100.0],
        body: vec![(r, 0.0, 30.0)],
        cut: Cut::Mill { r, corner },
    }];
    s.start_tool = Some(1);
    s
}

#[test]
fn a_flat_slot_is_exact_at_the_nodes() {
    let m = builtin().remove(0);
    let s = vmc_setup(&m, 5.0, 0.0);
    let prog = "G43 H1\nG0 X-10. Y15. Z20.\nZ5.\nG1 X60. F500.\nG0 Z20.\n";
    let tl = sim::simulate(prog, &s).unwrap();
    let r = removal::mill(&s, &tl, [0.0, 0.0, 0.0], [50.0, 30.0, 10.0], 0.5).unwrap();
    // nodes with |y − 15| ≤ 5 are at 5, the rest at 10
    for j in 0..r.map.ny {
        for i in 0..r.map.nx {
            let (_, y) = r.map.node(i, j);
            let want = if (y - 15.0).abs() <= 5.0 { 5.0 } else { 10.0 };
            assert_eq!(r.map.at(i, j), want, "node ({i}, {j})");
        }
    }
    assert!(r.rapid_cuts.is_empty());
    // 21 node rows of 101 at 5 mm, cells of 0.25 mm²
    assert!((r.removed.iter().sum::<f64>() - 21.0 * 101.0 * 5.0 * 0.25).abs() < 1e-9);
}

#[test]
fn a_ball_plunge_leaves_its_sphere() {
    let m = builtin().remove(0);
    let s = vmc_setup(&m, 5.0, 5.0);
    let prog = "G43 H1\nG0 X25. Y15. Z20.\nG1 Z4. F200.\nG0 Z20.\n";
    let tl = sim::simulate(prog, &s).unwrap();
    let r = removal::mill(&s, &tl, [0.0, 0.0, 0.0], [50.0, 30.0, 10.0], 0.5).unwrap();
    for j in 0..r.map.ny {
        for i in 0..r.map.nx {
            let (x, y) = r.map.node(i, j);
            let d = ((x - 25.0).powi(2) + (y - 15.0).powi(2)).sqrt();
            let want = if d <= 5.0 {
                (4.0 + 5.0 - (25.0 - d * d).sqrt()).min(10.0)
            } else {
                10.0
            };
            assert!(
                (r.map.at(i, j) - want).abs() < 1e-9,
                "({x}, {y}): {} vs {want}",
                r.map.at(i, j)
            );
        }
    }
}

#[test]
fn rapids_through_cut_air_pass_and_through_material_do_not() {
    let m = builtin().remove(0);
    let s = vmc_setup(&m, 5.0, 0.0);
    // the slot at Z5 along y = 15, then a rapid back along it at Z6: air
    let ok = "G43 H1\nG0 X-10. Y15. Z20.\nZ5.\nG1 X60. F500.\nG0 Z6.\nX-10.\nZ20.\n";
    let tl = sim::simulate(ok, &s).unwrap();
    let r = removal::mill(&s, &tl, [0.0, 0.0, 0.0], [50.0, 30.0, 10.0], 0.5).unwrap();
    assert!(r.rapid_cuts.is_empty(), "{:?}", r.rapid_cuts);
    // the same rapid 10 mm off the slot: through material
    let bad = "G43 H1\nG0 X-10. Y15. Z20.\nZ5.\nG1 X60. F500.\nG0 Z6.\nY25.\nX-10.\nZ20.\n";
    let tl = sim::simulate(bad, &s).unwrap();
    let r = removal::mill(&s, &tl, [0.0, 0.0, 0.0], [50.0, 30.0, 10.0], 0.5).unwrap();
    // Y25 at X60 is outside the stock's reach; X-10 at Y25 runs through 4 mm of it
    assert_eq!(r.rapid_cuts.len(), 1, "{:?}", r.rapid_cuts);
    assert_eq!(r.rapid_cuts[0].line, 7);
    assert!((r.rapid_cuts[0].depth - 4.0).abs() < 1e-12);
}

#[test]
fn a_turned_diameter_and_a_face() {
    let m = builtin().remove(2);
    let d = "[dialect]\nname = \"t\"\ndecimals = 3\nfeed_decimals = 3\ndwell = \"G04 P{ms}\"\n";
    let mut s = Setup::new(&m, Control::for_machine(&m, d, 1).unwrap()).unwrap();
    // WCS: the part's face at z = 100 (part frame), X word 0 on the axis for this tool's offsets
    s.wcs = vec![(54, vec![100.0, -120.0])];
    let rn = 0.8;
    s.tools = vec![SimTool {
        t: 1,
        name: "OD".into(),
        tip: [-120.0, 0.0, 10.0],
        body: vec![(6.0, 0.0, 30.0)],
        cut: Cut::Nose { rn, centre: [rn, rn] },
    }];
    s.geo = vec![(1, vec![-10.0, 120.0])];
    // face to Z0 from X 60 down past the axis, then turn ⌀40 to Z-30
    let prog = "T0101\nG0 X60. Z0\nG1 X-2. F200.\nG0 Z2.\nX40.\nG1 Z-30.\nX60.\nG0 Z5.\n";
    let tl = sim::simulate(prog, &s).unwrap();
    // a bar 1 mm longer than the faced length
    let r = removal::turn(&s, &tl, 25.0, 0.0, 101.0, 0.05).unwrap();
    assert!(r.rapid_cuts.is_empty(), "{:?}", r.rapid_cuts);
    for i in 0..r.map.r.len() {
        let z = r.map.z(i);
        let got = r.map.r[i];
        // beyond the face: gone
        if z > 100.0 + 1e-9 {
            assert_eq!(got, 0.0, "z {z}");
        } else if z > 70.0 + 2.0 * rn + 1e-9 && z < 100.0 - 1e-9 {
            // the turned length: radius 20 exactly where the nose's flat ... is its lowest point
            assert!((got - 20.0).abs() < 1e-9, "z {z}: {got}");
        } else if z < 70.0 - 1e-9 {
            assert_eq!(got, 25.0, "z {z}");
        }
    }
}

//! Timing only (ignored; `cargo test --release -p forge-machine --test removal_speed -- --ignored --nocapture`): a pocket on a flange-sized stock at CAM-SIM's grid, with its digest (0xa9118efcb77c7167 through the exact speed work; 0x16fc5f615e70afe1 with the closed form, within docs/cam-machine.md's bound).
use forge_machine::gcode::Control;
use forge_machine::library::builtin_named;
use forge_machine::removal;
use forge_machine::sim::{self, Cut, Setup, SimTool};

#[test]
#[ignore]
fn pocket_timing() {
    let m = builtin_named("generic-vmc3").unwrap();
    let mut s = Setup::new(&m, Control::for_dialect(&m, "fanuc", 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -500.0])];
    s.h = vec![(1, 100.0)];
    s.tools = vec![SimTool {
        t: 1,
        name: "e10".into(),
        tip: [0.0, 0.0, -100.0],
        body: vec![(5.0, 0.0, 25.0), (12.0, 25.0, 80.0)],
        cut: Cut::Mill { r: 5.0, corner: 1.0 },
    }];
    s.start_tool = Some(1);
    let mut p = String::from("G43 H1\nG0 X0 Y0 Z30.\n");
    // 8 levels of a zig-zag over 80 x 80, 6 mm stepover, plus arcs around
    for lvl in 1..=8 {
        let z = 23.0 - 2.0 * lvl as f64;
        p.push_str(&format!("G0 X-40. Y-40.\nG1 Z{z:.1} F500.\n"));
        let mut y = -40.0;
        let mut dir = 1.0;
        while y <= 40.0 {
            p.push_str(&format!("G1 X{:.1} Y{y:.1} F1500.\n", 40.0 * dir));
            y += 6.0;
            p.push_str(&format!("G1 Y{y:.1}\n"));
            dir = -dir;
        }
        p.push_str("G0 X30. Y0\nG2 X30. Y0 I-30. J0 F1200.\nG0 Z30.\n");
    }
    let tl = sim::simulate(&p, &s).unwrap();
    let t = std::time::Instant::now();
    let r = removal::mill(&s, &tl, [-43.0, -43.0, 0.0], [43.0, 43.0, 23.0], 0.5).unwrap();
    let mut h = 0u64;
    for z in &r.map.z {
        h = h.wrapping_mul(0x100000001b3) ^ z.to_bits();
    }
    println!(
        "segments {} time {:?} digest {h:#x} holder {} rapid {}",
        tl.segments.len(),
        t.elapsed(),
        r.holder_hits.len(),
        r.rapid_cuts.len()
    );
}

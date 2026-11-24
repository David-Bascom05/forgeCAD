//! Native = wasm, bit for bit: one digest over a simulated program's joint values and times,
//! interval poses, collision verdicts and removal heights on two machines, pinned here and
//! checked on both targets (tools/test-wasm.sh runs every kernel test under wasm32-wasip1).
use forge_machine::collide::{self, Role, Verdict};
use forge_machine::def::Prim;
use forge_machine::gcode::Control;
use forge_machine::library::builtin;
use forge_machine::removal;
use forge_machine::sim::{self, Cut, Setup, SimTool};

fn fnv(h: &mut u64, b: &[u8]) {
    for x in b {
        *h ^= *x as u64;
        *h = h.wrapping_mul(0x100000001b3);
    }
}

fn f(h: &mut u64, v: f64) {
    fnv(h, &v.to_bits().to_le_bytes());
}

// re-pinned on train 4a: #21's sound Interval sin/cos (judged a fix, David OK) moves the
// interval kinematics' bounds; with main's forge-math interval.rs the old 0xff9328d4d74088e4
// returns exactly. The same value on wasm32-wasip1.
const PINNED: u64 = 0xcd78466c86f8e8b5;

#[test]
fn the_same_bits_everywhere() {
    let mut h = 0xcbf29ce484222325u64;
    let ms = builtin();
    // the trunnion: tilts, an arc, a rapid into the cradle
    let m = &ms[1];
    let mut s = Setup::new(m, Control::for_dialect(m, "fanuc", 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -400.0, 0.0, 0.0])];
    s.h = vec![(1, 100.0)];
    s.tools = vec![SimTool {
        t: 1,
        name: "b6".into(),
        tip: [0.0, 0.0, -100.0],
        body: vec![(3.0, 0.0, 20.0), (10.0, 20.0, 100.0)],
        cut: Cut::Mill { r: 3.0, corner: 3.0 },
    }];
    s.start_tool = Some(1);
    let prog = "G43 H1\nG0 X0 Y0 Z250. B0 C0\nB37.5 C-121.25\nG1 X-13.7 Y22.1 Z180. F2500.\nG3 X13.7 Y22.1 I13.7 J-22.1 F900.\nG0 B90. C0\nX-200.\nZ100.\nX0\n";
    let tl = sim::simulate(prog, &s).unwrap();
    for sg in &tl.segments {
        for k in 0..=8 {
            for v in sg.at(k as f64 / 8.0) {
                f(&mut h, v);
            }
        }
        f(&mut h, sg.t_min);
        f(&mut h, sg.t_stop);
    }
    let q: Vec<forge_math::interval::Interval> = tl.segments[2]
        .q1
        .iter()
        .map(|v| forge_math::interval::Interval::new(v - 0.25, v + 0.25))
        .collect();
    for p in forge_machine::kin::ipose(m, &q) {
        for i in 0..3 {
            f(&mut h, p.p[i].lo);
            f(&mut h, p.p[i].hi);
            for j in 0..3 {
                f(&mut h, p.r[i][j].lo);
                f(&mut h, p.r[i][j].hi);
            }
        }
    }
    let stock = collide::part_body(
        &s,
        "stock",
        &[Prim::Box {
            min: [-40.0, -40.0, 0.0],
            max: [40.0, 40.0, 30.0],
        }],
        Role::Stock,
    );
    let r = collide::check(&s, &tl, &[stock]);
    fnv(&mut h, &(r.spans as u64).to_le_bytes());
    for c in &r.contacts {
        fnv(&mut h, c.a.as_bytes());
        fnv(&mut h, c.b.as_bytes());
        match c.verdict {
            Verdict::Hit { s, witness } => {
                f(&mut h, s);
                witness.iter().for_each(|v| f(&mut h, *v));
            }
            Verdict::Near { s0, s1, gap_lo } => [s0, s1, gap_lo].iter().for_each(|v| f(&mut h, *v)),
            Verdict::Clear => {}
        }
    }
    // the VMC: a ball slot's heights
    let m = &ms[0];
    let mut s = Setup::new(m, Control::for_dialect(m, "fanuc", 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -500.0])];
    s.h = vec![(1, 100.0)];
    s.tools = vec![SimTool {
        t: 1,
        name: "b10".into(),
        tip: [0.0, 0.0, -100.0],
        body: vec![(5.0, 0.0, 30.0)],
        cut: Cut::Mill { r: 5.0, corner: 5.0 },
    }];
    s.start_tool = Some(1);
    let prog =
        "G43 H1\nG0 X-10. Y7.3 Z20.\nZ6.1\nG1 X60. Y21.9 Z4.4 F700.\nG2 X36. Y21.9 I-12. J0 F500.\nG0 Z20.\n";
    let tl = sim::simulate(prog, &s).unwrap();
    let r = removal::mill(&s, &tl, [0.0, 0.0, 0.0], [50.0, 30.0, 10.0], 0.5).unwrap();
    for z in &r.map.z {
        f(&mut h, *z);
    }
    assert_eq!(h, PINNED, "digest {h:#018x}");
}

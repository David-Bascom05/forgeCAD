use forge_machine::collide::{self, Role, Verdict};
use forge_machine::def::Prim;
use forge_machine::gcode::Control;
use forge_machine::library::builtin;
use forge_machine::sim::{self, Cut, Setup, SimTool};

#[test]
fn builtin_machines_are_clear_at_home() {
    for m in builtin() {
        let q: Vec<f64> = m.joints.iter().map(|j| j.home).collect();
        let c = collide::check_pose(&m, &q);
        assert!(c.is_empty(), "{}: {c:?}", m.name);
    }
}

/// Every joint to each end of its travel, one at a time and all together: the machine's own
/// bodies stay clear (the envelope is consistent with the travel).
#[test]
fn builtin_machines_are_clear_through_their_travel() {
    for m in builtin() {
        let n = m.joints.len();
        let mut corners: Vec<Vec<f64>> = Vec::new();
        for j in 0..n {
            for end in 0..2 {
                let mut q: Vec<f64> = m.joints.iter().map(|j| j.home).collect();
                q[j] = m.joints[j].range[end];
                corners.push(q);
            }
        }
        for q in corners {
            let c = collide::check_pose(&m, &q);
            assert!(c.is_empty(), "{} at {q:?}: {c:?}", m.name);
        }
    }
}

fn lathe_setup(m: &forge_machine::MachineDef) -> Setup<'_> {
    let d = "[dialect]\nname = \"t\"\ndecimals = 3\nfeed_decimals = 3\ndwell = \"G04 P{ms}\"\n";
    let mut s = Setup::new(m, Control::for_machine(m, d, 1).unwrap()).unwrap();
    s.wcs = vec![(54, vec![100.0, -120.0])];
    s.tools = vec![SimTool {
        t: 1,
        name: "OD".into(),
        tip: [-120.0, 0.0, 10.0],
        body: vec![(6.0, 0.0, 30.0), (12.0, 30.0, 110.0)],
        cut: Cut::Nose {
            rn: 0.8,
            centre: [0.8, 0.8],
        },
    }];
    s.geo = vec![(1, vec![-10.0, 120.0])];
    s
}

#[test]
fn lathe_turret_into_the_chuck_is_a_hit() {
    let m = builtin().remove(2);
    let s = lathe_setup(&m);
    // fine: along the part, well clear of the chuck face
    let ok = "T0101\nG0 X80. Z5.\nG1 Z-50. F500.\n";
    let tl = sim::simulate(ok, &s).unwrap();
    let r = collide::check(&s, &tl, &[]);
    assert!(r.contacts.is_empty(), "{:?}", r.contacts);
    // a rapid to Z-120 (20 mm off the chuck jaws' face) at X 60: the turret's disc reaches the chuck
    let bad = "T0101\nG0 X60. Z5.\nZ-120.\n";
    let tl = sim::simulate(bad, &s).unwrap();
    let r = collide::check(&s, &tl, &[]);
    assert!(
        r.contacts
            .iter()
            .any(|c| matches!(c.verdict, Verdict::Hit { .. }) && (c.a == "chuck" || c.b == "chuck")),
        "{:?}",
        r.contacts
    );
}

#[test]
fn rapid_into_the_stock_and_into_the_tilted_cradle() {
    let m = builtin().remove(1);
    let mut s = Setup::new(&m, Control::for_dialect(&m, "fanuc", 1).unwrap()).unwrap();
    // WCS Z0 on the rotary table's top: tip = 400 + q_z − 100 = 0 at q_z = −300, so −400 + H
    s.wcs = vec![(54, vec![0.0, 0.0, -400.0, 0.0, 0.0])];
    s.h = vec![(1, 100.0)];
    s.tools = vec![SimTool {
        t: 1,
        name: "flat-10".into(),
        tip: [0.0, 0.0, -100.0],
        body: vec![(5.0, 0.0, 30.0), (8.0, 30.0, 60.0), (20.0, 60.0, 100.0)],
        cut: Cut::Mill { r: 5.0, corner: 0.0 },
    }];
    s.start_tool = Some(1);
    let stock = collide::part_body(
        &s,
        "stock",
        &[Prim::Box {
            min: [-40.0, -40.0, 0.0],
            max: [40.0, 40.0, 30.0],
        }],
        Role::Stock,
    );
    // a rapid across the stock 10 mm above the table: the cutter goes through it
    let prog = "G43 H1\nG0 X-100. Y0 Z150. B0 C0\nZ10.\nX0\n";
    let tl = sim::simulate(prog, &s).unwrap();
    let r = collide::check(&s, &tl, std::slice::from_ref(&stock));
    assert!(
        r.contacts
            .iter()
            .all(|c| c.line == 4 && matches!(c.verdict, Verdict::Hit { .. })),
        "{:?}",
        r.contacts
    );
    assert!(
        r.contacts.iter().any(|c| c.a == "stock" || c.b == "stock"),
        "{:?}",
        r.contacts
    );
    // the same as a feed: the cutter in the stock is cutting; the holder pieces stay above it
    let prog = "G43 H1\nG0 X-100. Y0 Z150. B0 C0\nZ10.\nG1 X0 F500.\n";
    let tl = sim::simulate(prog, &s).unwrap();
    let r = collide::check(&s, &tl, std::slice::from_ref(&stock));
    assert!(r.contacts.is_empty(), "{:?}", r.contacts);
    // B 90 turns the cradle up into the x−z plane: it stands at x ∈ [−100, 10] up to z = 150. A
    // rapid at tip height 100 from x = −200 to 0 runs into it at x = −100.
    let prog = "G43 H1\nG0 X0 Y0 Z250. B0 C0\nB90.\nX-200.\nZ100.\nX0\n";
    let tl = sim::simulate(prog, &s).unwrap();
    let r = collide::check(&s, &tl, std::slice::from_ref(&stock));
    assert!(!r.contacts.is_empty());
    for c in &r.contacts {
        assert_eq!(c.line, 6, "{c:?}");
        match c.verdict {
            Verdict::Hit { witness, .. } => assert!((witness[0] + 100.0).abs() < 0.1, "{c:?}"),
            _ => panic!("{c:?}"),
        }
        assert!(c.a == "cradle" || c.b == "cradle", "{c:?}");
    }
}

/// Soundness against sampling: wherever a pose is reported clear, no sample point of one body is
/// inside the other (1000 random poses of the trunnion with a stock block).
#[test]
fn clear_is_never_contradicted_by_samples() {
    let m = builtin().remove(1);
    let mut s = Setup::new(&m, Control::for_dialect(&m, "fanuc", 1).unwrap()).unwrap();
    s.tools = vec![SimTool {
        t: 1,
        name: "t".into(),
        tip: [0.0, 0.0, -150.0],
        body: vec![(5.0, 0.0, 150.0)],
        cut: Cut::Mill { r: 5.0, corner: 0.0 },
    }];
    let stock = collide::part_body(
        &s,
        "stock",
        &[Prim::Box {
            min: [-60.0, -60.0, 0.0],
            max: [60.0, 60.0, 80.0],
        }],
        Role::Fixture,
    );
    let mut x: u64 = 7;
    let mut rnd = || {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    let (mut clear, mut hit) = (0, 0);
    for _ in 0..1000 {
        let q: Vec<f64> = m
            .joints
            .iter()
            .map(|j| j.range[0] + rnd() * (j.range[1] - j.range[0]))
            .collect();
        let seg = forge_machine::sim::Segment {
            line: 1,
            kind: forge_machine::sim::SegKind::Rapid,
            q0: q.clone(),
            q1: q.clone(),
            path: forge_machine::sim::Path::Line,
            t0: 0.0,
            t_min: 0.0,
            t_stop: 0.0,
            tool: Some(1),
            spindle: None,
        };
        let tl = forge_machine::sim::Timeline {
            segments: vec![seg],
            marks: vec![],
            violations: vec![],
            t_min: 0.0,
            t_stop: 0.0,
            joints: vec![],
            accel_missing: false,
        };
        let r = collide::check(&s, &tl, std::slice::from_ref(&stock));
        let pose = forge_machine::kin::pose(&m, &q);
        let mut bodies = collide::machine_bodies(&m);
        bodies.extend(collide::tool_bodies(&s, 1));
        bodies.push(stock.clone());
        for i in 0..bodies.len() {
            for j in i + 1..bodies.len() {
                let (a, b) = (&bodies[i], &bodies[j]);
                if a.link == b.link || a.ignore.contains(&b.name) || b.ignore.contains(&a.name) {
                    continue;
                }
                let reported = r
                    .contacts
                    .iter()
                    .any(|c| (c.a == a.name && c.b == b.name) || (c.a == b.name && c.b == a.name));
                let inside = collide::dense_overlap(a, b, &pose);
                if inside {
                    assert!(
                        reported,
                        "{} and {} overlap at {q:?} but were cleared",
                        a.name, b.name
                    );
                    hit += 1;
                } else if !reported {
                    clear += 1;
                }
            }
        }
    }
    assert!(clear > 1000 && hit > 10, "clear {clear} hit {hit}");
}

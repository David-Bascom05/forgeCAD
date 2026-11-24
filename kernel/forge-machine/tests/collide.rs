use forge_machine::collide::{self, Role, Verdict};
use forge_machine::def::{MountKind, Prim};
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
                q[j] = m.joints[j].range.unwrap()[end];
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
            .map(|j| j.range.unwrap()[0] + rnd() * (j.range.unwrap()[1] - j.range.unwrap()[0]))
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
            after_skip: Vec::new(),
            skip_unmodelled: Vec::new(),
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

fn rapid(q0: Vec<f64>, q1: Vec<f64>) -> sim::Segment {
    sim::Segment {
        line: 0,
        kind: sim::SegKind::Rapid,
        q0,
        q1,
        path: sim::Path::Line,
        t0: 0.0,
        t_min: 0.0,
        t_stop: 0.0,
        tool: None,
        spindle: None,
    }
}

/// The pieces [a, b] of the line q0 → q1 outside every zone of the record (zones are open boxes:
/// their boundaries are travel).
fn outside_zones(m: &forge_machine::MachineDef, q0: &[f64], q1: &[f64]) -> Vec<(f64, f64)> {
    let mut inside: Vec<(f64, f64)> = Vec::new();
    for z in &m.zones {
        let (mut lo, mut hi) = (0.0f64, 1.0f64);
        for (k, j) in m.zone_joints(z).into_iter().enumerate() {
            let (a, d) = (q0[j], q1[j] - q0[j]);
            if d == 0.0 {
                if !(z.min[k] < a && a < z.max[k]) {
                    hi = lo;
                }
                continue;
            }
            let (t1, t2) = ((z.min[k] - a) / d, (z.max[k] - a) / d);
            lo = lo.max(t1.min(t2));
            hi = hi.min(t1.max(t2));
        }
        if lo < hi {
            inside.push((lo, hi));
        }
    }
    inside.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    let mut at = 0.0;
    for (a, b) in inside {
        if a > at {
            out.push((at, a));
        }
        at = at.max(b);
    }
    if at < 1.0 {
        out.push((at, 1.0));
    }
    out
}

/// Every pair of bodies through the joints that move one relative to the other: each such joint
/// swept through its whole range with the others at every combination of their ends. A Hit is a
/// pose inside the declared travel where the machine's own bodies collide (wrong data); a Near is
/// one not proven clear. Neither may happen in a built-in.
#[test]
fn builtin_machines_have_no_self_collision_in_travel() {
    let mut bad = Vec::new();
    for m in builtin() {
        let home: Vec<f64> = m.joints.iter().map(|j| j.home).collect();
        let chain = |l: &str| m.chain(l).unwrap_or_default();
        for (ai, a) in m.bodies.iter().enumerate() {
            for b in &m.bodies[ai + 1..] {
                if a.link == b.link || a.ignore.contains(&b.name) || b.ignore.contains(&a.name) {
                    continue;
                }
                let (ca, cb) = (chain(&a.link), chain(&b.link));
                let mut js: Vec<usize> = ca.iter().filter(|j| !cb.contains(j)).copied().collect();
                js.extend(cb.iter().filter(|j| !ca.contains(j)));
                js.sort();
                let mut segs = Vec::new();
                for (k, &j) in js.iter().enumerate() {
                    let others: Vec<usize> = js
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != k)
                        .map(|(_, x)| *x)
                        .collect();
                    for c in 0..(1usize << others.len()) {
                        let mut base = home.clone();
                        for (bit, &o) in others.iter().enumerate() {
                            base[o] = m.joints[o].range.unwrap()[(c >> bit) & 1];
                        }
                        let (mut q0, mut q1) = (base.clone(), base);
                        q0[j] = m.joints[j].range.unwrap()[0];
                        q1[j] = m.joints[j].range.unwrap()[1];
                        // the record's zones are not travel: only the pieces outside them
                        for (a, b) in outside_zones(&m, &q0, &q1) {
                            let at = |t: f64| q0.iter().zip(&q1).map(|(x, y)| x + t * (y - x)).collect();
                            segs.push(rapid(at(a), at(b)));
                        }
                    }
                }
                let mut d2 = m.clone();
                d2.bodies = vec![a.clone(), b.clone()];
                let mount = |k: MountKind| {
                    d2.mounts
                        .iter()
                        .find(|x| x.kind == k)
                        .map(|x| x.name.clone())
                        .unwrap()
                };
                let s2 = Setup {
                    def: &d2,
                    control: Control::mill3(3, 0, false),
                    joint_of: Vec::new(),
                    tool_mount: mount(MountKind::Tool),
                    part_mount: mount(MountKind::Part),
                    wcs: Vec::new(),
                    tools: Vec::new(),
                    h: Vec::new(),
                    geo: Vec::new(),
                    dogleg: false,
                    feed_combined: true,
                    start: home.clone(),
                    start_tool: None,
                    skip_stops: Vec::new(),
                };
                for seg in segs {
                    let tl = sim::Timeline {
                        segments: vec![seg],
                        marks: Vec::new(),
                        violations: Vec::new(),
                        t_min: 0.0,
                        t_stop: 0.0,
                        joints: Vec::new(),
                        accel_missing: false,
                        after_skip: Vec::new(),
                        skip_unmodelled: Vec::new(),
                    };
                    let rep = collide::check(&s2, &tl, &[]);
                    if let Some(c) = rep.contacts.first() {
                        let q = match &c.verdict {
                            Verdict::Hit { s, .. } => tl.segments[0].at(*s),
                            Verdict::Near { s0, s1, .. } => tl.segments[0].at(0.5 * (s0 + s1)),
                            Verdict::Clear => continue,
                        };
                        bad.push(format!(
                            "{}: {} vs {} at {q:?}: {:?}",
                            m.name, a.name, b.name, c.verdict
                        ));
                        break;
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// The moves after a skip (probe) move are checked for every place the probe can stop, not only
/// where the simulation stops it: with the stop estimated early (high, as a stock model can put
/// it), a rapid in X with Z still at the stop clears a low block, which the one-stop check
/// accepts; from a later, lower stop it meets the block, which the check finds. A retract that
/// sets Z first is proven for every stop.
#[test]
fn moves_after_a_skip_are_checked_for_every_stop() {
    let m = builtin().remove(0);
    let mut c = Control::for_dialect(&m, "fanuc", 1).unwrap();
    c.skip = vec![382];
    let mut s = Setup::new(&m, c).unwrap();
    s.wcs = vec![(54, vec![0.0, 0.0, -300.0])];
    s.tools = vec![SimTool {
        t: 1,
        name: "probe".into(),
        tip: [0.0, 0.0, -100.0],
        body: vec![(3.0, 0.0, 20.0), (8.0, 20.0, 100.0)],
        cut: Cut::Mill { r: 3.0, corner: 0.0 },
    }];
    s.h = vec![(1, 100.0)];
    s.start_tool = Some(1);
    // a low block at X 100 (this setup's tip is at part z = word Z + 200): the tool meets it from
    // a stop below word Z 15
    let block = collide::part_body(
        &s,
        "block",
        &[Prim::Box {
            min: [95.0, -20.0, 200.0],
            max: [105.0, 20.0, 215.0],
        }],
        Role::Fixture,
    );
    // retract first: proven for every stop
    let ok = "G43 H1\nG0 X0 Y0 Z50.\nG38.2 Z5. F100.\nG0 Z200.\nG0 X100.\n";
    let tl = sim::simulate(ok, &s).unwrap();
    assert_eq!(tl.after_skip.len(), 1, "{:?}", tl.after_skip);
    let z = m.joint_index("Z").unwrap();
    let d = &tl.after_skip[0];
    assert!(d.start[z] && !d.end[z]);
    let r = collide::check(&s, &tl, std::slice::from_ref(&block));
    assert!(r.contacts.is_empty(), "{:?}", r.contacts);
    assert_eq!(r.skips_proven, vec![3]);
    let mut u = sim::unchecked(&m, &tl);
    collide::narrow_skip_unchecked(&mut u, &tl, &r);
    assert!(
        u.iter()
            .any(|x| x.path == "skip.end" && x.check.contains("collisions proven for every stop")),
        "{u:?}"
    );
    // across in X with Z still at the stop, the stop estimated at the top (u = 0)
    s.skip_stops = vec![(3, 0.0)];
    let bad = "G43 H1\nG0 X0 Y0 Z50.\nG38.2 Z5. F100.\nG0 X100.\n";
    let tl = sim::simulate(bad, &s).unwrap();
    let mut one_stop = tl.clone();
    one_stop.after_skip.clear();
    let r1 = collide::check(&s, &one_stop, std::slice::from_ref(&block));
    assert!(r1.contacts.is_empty(), "{:?}", r1.contacts);
    let r = collide::check(&s, &tl, std::slice::from_ref(&block));
    assert!(
        r.contacts.iter().any(|c| c.line == 4
            && (c.a == "block" || c.b == "block")
            && matches!(c.verdict, Verdict::Hit { .. })),
        "{:?}",
        r.contacts
    );
    assert!(r.skips_proven.is_empty());
    let mut u = sim::unchecked(&m, &tl);
    collide::narrow_skip_unchecked(&mut u, &tl, &r);
    assert!(
        u.iter()
            .any(|x| x.path == "skip.end" && x.why == "estimated" && !x.check.contains("proven"))
    );
    // an arc after the skip is not modelled: the skip is reported, not proven
    s.skip_stops.clear();
    let arc = "G43 H1\nG0 X0 Y0 Z50.\nG38.2 Z5. F100.\nG2 X10. Y0 I5. J0 F500.\n";
    let tl = sim::simulate(arc, &s).unwrap();
    assert_eq!(tl.skip_unmodelled, vec![3]);
    assert!(collide::check(&s, &tl, &[]).skips_proven.is_empty());
}

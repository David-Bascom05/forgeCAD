//! Textbook mechanisms (prior-art §11.3, §11.4): mobility from the rank agrees with a
//! finite-difference count and the textbook number; output follows the closed forms.

mod common;
use common::*;
use forge_asm::*;
use forge_math::{Frame, dmath};

fn opts() -> Options {
    Options { threads: 1 }
}

#[test]
fn four_bar_moves_with_one_freedom_and_three_redundant_rows() {
    let asm = four_bar(40.0, 120.0, 80.0, 100.0, 0.6);
    let sol = solve(&asm, &catalog(), &opts());
    assert_eq!(sol.errors().count(), 0, "{:?}", sol.diagnostics);
    assert!(worst(&asm, &sol) < 1e-8);
    assert_eq!(sol.mobility, 1);
    assert_eq!(fd_mobility(&asm, &sol), 1);
    // a planar loop of revolutes is three times redundant in 3D
    assert_eq!(redundant_rows(&sol), 3, "{:?}", sol.diagnostics);
    assert!(sol.diagnostics.iter().all(|d| d.severity() == Severity::Warning));
    assert_eq!(sol.blocks.len(), 1);
    assert_eq!(sol.blocks[0].structural_mobility, -2);
    // every joint of the loop moves
    assert!(sol.joints.iter().all(|j| j.free == 1), "{:?}", sol.joints);
}

#[test]
fn four_bar_follows_its_closed_form() {
    let (a, b, c, d) = (40.0, 120.0, 80.0, 100.0);
    let mut asm = four_bar(a, b, c, d, 0.6);
    asm.joints[0] = asm.joints[0].clone().driven(Freedom::Rz, 0.6);
    let parts = catalog();
    let mut s = Solver::new(asm, &parts, opts());
    assert_eq!(s.solution().mobility, 0);
    for i in 0..40 {
        let t2 = 0.6 + i as f64 * 0.05;
        let sol = s.set_drive(JointId(1), Freedom::Rz, t2).clone();
        assert_eq!(sol.errors().count(), 0, "{:?}", sol.diagnostics);
        let (t3, t4) = four_bar_angles(a, b, c, d, t2).unwrap();
        let rocker = sol.pose(InstanceId(4)).unwrap().x().get();
        let coupler = sol.pose(InstanceId(3)).unwrap().x().get();
        let gap = |x: forge_math::Vec3, t: f64| {
            let (sn, cs) = dmath::sin_cos(t);
            ((x.x - cs).abs()).max((x.y - sn).abs())
        };
        assert!(gap(rocker, t4) < 1e-9, "step {i}: rocker {:?} want {t4}", rocker);
        assert!(gap(coupler, t3) < 1e-9);
        assert!(worst(s.assembly(), &sol) < 1e-8);
        // a drag re-solves the loop and nothing else
        assert_eq!(sol.stats.partitions_solved, 1);
    }
}

#[test]
fn slider_crank_has_one_freedom_and_tracks_the_slider_formula() {
    let (r, l) = (30.0, 100.0);
    let asm = slider_crank(r, l, 0.4);
    let sol = solve(&asm, &catalog(), &opts());
    assert_eq!(sol.errors().count(), 0, "{:?}", sol.diagnostics);
    assert_eq!(sol.mobility, 1);
    assert_eq!(fd_mobility(&asm, &sol), 1);
    let mut asm = asm;
    asm.joints[0] = asm.joints[0].clone().driven(Freedom::Rz, 0.4);
    let parts = catalog();
    let mut s = Solver::new(asm, &parts, opts());
    for i in 0..30 {
        let t = 0.4 + 0.1 * i as f64;
        let sol = s.set_drive(JointId(1), Freedom::Rz, t);
        let (sn, cs) = dmath::sin_cos(t);
        let want = r * cs + (l * l - r * r * sn * sn).sqrt();
        let got = sol.pose(InstanceId(4)).unwrap().origin.x;
        assert!((got - want).abs() < 1e-9, "t {t}: {got} vs {want}");
    }
}

fn gear_pair(ratio: f64) -> Assembly {
    let g2 = at(60.0, 0.0, 0.0);
    Assembly {
        instances: vec![
            inst(1, Frame::WORLD).grounded(),
            inst(2, Frame::WORLD),
            inst(3, g2),
        ],
        joints: vec![
            joint(1, JointKind::Revolute, (2, Frame::WORLD), (1, Frame::WORLD)),
            joint(2, JointKind::Revolute, (3, Frame::WORLD), (1, g2)),
        ],
        relations: vec![Relation {
            id: RelationId(1),
            kind: RelationKind::Gear { ratio },
            leader: ValueRef {
                joint: JointId(1),
                freedom: Freedom::Rz,
            },
            follower: ValueRef {
                joint: JointId(2),
                freedom: Freedom::Rz,
            },
            offset: None,
            suppressed: false,
        }],
        ..Assembly::default()
    }
}

#[test]
fn gear_pair_has_one_freedom_and_its_ratio() {
    let asm = gear_pair(0.5);
    let parts = catalog();
    let sol = solve(&asm, &parts, &opts());
    assert_eq!(sol.errors().count(), 0, "{:?}", sol.diagnostics);
    assert_eq!(sol.mobility, 1);
    let mut asm = asm;
    asm.joints[0] = asm.joints[0].clone().driven(Freedom::Rz, 0.0);
    let mut s = Solver::new(asm, &parts, opts());
    assert_eq!(s.solution().mobility, 0);
    for i in 0..20 {
        let t = -1.0 + 0.13 * i as f64;
        let sol = s.set_drive(JointId(1), Freedom::Rz, t);
        let x = sol.pose(InstanceId(3)).unwrap().x().get();
        let (sn, cs) = dmath::sin_cos(-0.5 * t);
        assert!((x.x - cs).abs() < 1e-12 && (x.y - sn).abs() < 1e-12);
        assert_eq!(sol.stats.blocks_solved, 0, "gears on fixed axes are trees");
    }
}

#[test]
fn rack_and_pinion_and_screw_follow_their_leads() {
    let along_x = frame(p(0.0, 0.0, 0.0), v(1.0, 0.0, 0.0), v(0.0, 1.0, 0.0));
    let mut asm = Assembly {
        instances: vec![
            inst(1, Frame::WORLD).grounded(),
            inst(2, Frame::WORLD),
            inst(3, at(0.0, -20.0, 0.0)),
            inst(4, at(0.0, 50.0, 0.0)),
        ],
        joints: vec![
            joint(1, JointKind::Revolute, (2, Frame::WORLD), (1, Frame::WORLD)).driven(Freedom::Rz, 0.0),
            joint(
                2,
                JointKind::Slider,
                (3, along_x),
                (1, at(0.0, -20.0, 0.0).compose(&along_x)),
            ),
            joint(
                3,
                JointKind::Cylindrical,
                (4, Frame::WORLD),
                (1, at(0.0, 50.0, 0.0)),
            )
            .driven(Freedom::Rz, 0.0),
        ],
        ..Assembly::default()
    };
    let rel = |id, kind, lj, lf, fj, ff| Relation {
        id: RelationId(id),
        kind,
        leader: ValueRef {
            joint: JointId(lj),
            freedom: lf,
        },
        follower: ValueRef {
            joint: JointId(fj),
            freedom: ff,
        },
        offset: None,
        suppressed: false,
    };
    asm.relations = vec![
        rel(
            1,
            RelationKind::RackPinion { radius: 20.0 },
            1,
            Freedom::Rz,
            2,
            Freedom::Tz,
        ),
        rel(
            2,
            RelationKind::Screw { lead: 1.5 },
            3,
            Freedom::Rz,
            3,
            Freedom::Tz,
        ),
    ];
    let parts = catalog();
    let mut s = Solver::new(asm, &parts, opts());
    assert_eq!(s.solution().mobility, 0, "{:?}", s.solution().joints);
    s.set_drive(JointId(1), Freedom::Rz, 0.7);
    let sol = s.set_drive(JointId(3), Freedom::Rz, 4.0 * dmath::PI).clone();
    assert_eq!(sol.errors().count(), 0, "{:?}", sol.diagnostics);
    assert!((sol.pose(InstanceId(3)).unwrap().origin.x - 14.0).abs() < 1e-12);
    assert!((sol.pose(InstanceId(4)).unwrap().origin.z - 3.0).abs() < 1e-12);
}

#[test]
fn a_spatial_four_bar_counts_its_idle_spin() {
    // RSSR: revolute crank, ball, ball, revolute rocker on a skew axis: 1 + the coupler's spin
    let rocker_axis = frame(p(100.0, 0.0, 30.0), v(0.0, 1.0, 0.0), v(1.0, 0.0, 0.0));
    let crank_tip = p(40.0, 0.0, 0.0);
    let rocker_tip = rocker_axis.point_to_world(p(50.0, 0.0, 0.0));
    let asm = Assembly {
        instances: vec![
            inst(1, Frame::WORLD).grounded(),
            inst(2, Frame::WORLD),
            inst(3, Frame::WORLD.with_origin(crank_tip)),
            inst(4, rocker_axis),
        ],
        joints: vec![
            joint(1, JointKind::Revolute, (2, Frame::WORLD), (1, Frame::WORLD)),
            joint(2, JointKind::Ball, (3, Frame::WORLD), (2, at(40.0, 0.0, 0.0))),
            joint(
                3,
                JointKind::Ball,
                (4, at(50.0, 0.0, 0.0)),
                (
                    3,
                    Frame::WORLD.with_origin(p(0.0, 0.0, 0.0) + (rocker_tip - crank_tip)),
                ),
            ),
            joint(4, JointKind::Revolute, (4, Frame::WORLD), (1, rocker_axis)),
        ],
        ..Assembly::default()
    };
    let sol = solve(&asm, &catalog(), &opts());
    assert_eq!(sol.errors().count(), 0, "{:?}", sol.diagnostics);
    assert!(worst(&asm, &sol) < 1e-8);
    assert_eq!(sol.mobility, 2);
    assert_eq!(fd_mobility(&asm, &sol), 2);
    assert_eq!(redundant_rows(&sol), 0);
}

/// The same mechanism written as loose constraints instead of joints.
fn loosen(asm: &Assembly) -> Assembly {
    let mut out = asm.clone();
    out.joints.clear();
    let mut id = 0;
    for j in &asm.joints {
        for &k in j.kind.bundle() {
            id += 1;
            let mut b = j.b.clone();
            if j.kind == JointKind::PinSlot && k == ConstraintKind::PointOnLine {
                let AnchorRef::Local(r) = &b.anchor else {
                    unreachable!()
                };
                let f = r.frame;
                b.anchor = AnchorRef::Local(Reference::frame(
                    Frame::from_zx(f.origin, f.x(), f.y().get()).unwrap(),
                ));
            }
            let mut c = Constraint::new(id, k, j.a.clone(), b);
            c.flip = j.flip;
            out.constraints.push(c);
        }
    }
    out
}

#[test]
fn loose_constraints_give_what_joints_give() {
    for asm in [
        four_bar(40.0, 120.0, 80.0, 100.0, 0.6),
        slider_crank(30.0, 100.0, 0.4),
    ] {
        let parts = catalog();
        let a = solve(&asm, &parts, &opts());
        let loose = loosen(&asm);
        let b = solve(&loose, &parts, &opts());
        assert_eq!(b.errors().count(), 0, "{:?}", b.diagnostics);
        assert_eq!(a.mobility, b.mobility);
        assert_eq!(redundant_rows(&a), redundant_rows(&b));
        for (x, y) in a.poses.iter().zip(&b.poses) {
            assert!(x.1.origin.distance(y.1.origin) < 1e-12);
            assert!((x.1.x().get() - y.1.x().get()).length() < 1e-12);
        }
        assert!(fd_mobility(&asm, &a) == fd_mobility(&asm, &b));
        // the per-constraint report counts what each removes, and the redundancy
        let removed: usize = b.constraints.iter().map(|c| c.removes - c.redundant).sum();
        assert_eq!(removed, 18 - b.mobility);
    }
}

#[test]
fn a_screw_in_a_loop_keeps_whole_turns() {
    // a crank-rocker four-bar whose coupler turns twice against the crank as the crank turns
    // twice; a screw relation from that joint drives a slider, which must count both turns
    let (a, b, c, d) = (40.0, 120.0, 80.0, 100.0);
    let mut asm = four_bar(a, b, c, d, 0.6);
    asm.joints[0] = asm.joints[0].clone().driven(Freedom::Rz, 0.6);
    let shaft = at(0.0, -200.0, 0.0);
    asm.instances.push(inst(5, shaft));
    asm.joints
        .push(joint(5, JointKind::Slider, (5, Frame::WORLD), (1, shaft)));
    let lead = 3.0;
    asm.relations.push(Relation {
        id: RelationId(1),
        kind: RelationKind::Screw { lead },
        leader: ValueRef {
            joint: JointId(2),
            freedom: Freedom::Rz,
        },
        follower: ValueRef {
            joint: JointId(5),
            freedom: Freedom::Tz,
        },
        offset: None,
        suppressed: false,
    });
    let parts = catalog();
    let mut s = Solver::new(asm, &parts, opts());
    assert_eq!(s.solution().errors().count(), 0, "{:?}", s.solution().diagnostics);
    let (t3_0, _) = four_bar_angles(a, b, c, d, 0.6).unwrap();
    for (turns, extra) in [(0.0, 1.0), (1.0, 1.0), (2.0, 0.0), (-1.0, -2.0)] {
        let t2 = 0.6 + dmath::TAU * turns + extra;
        let sol = s.set_drive(JointId(1), Freedom::Rz, t2).clone();
        assert_eq!(sol.errors().count(), 0, "{:?}", sol.diagnostics);
        let (t3, _) = four_bar_angles(a, b, c, d, t2).unwrap();
        // the coupler only rocks: its change is the principal one; the crank's counts every turn
        let dt3 = joint_unwrap(t3 - t3_0);
        let want = lead / dmath::TAU * (dt3 - (t2 - 0.6));
        let got = sol.pose(InstanceId(5)).unwrap().origin.z;
        assert!(
            (got - want).abs() < 1e-9,
            "{turns} turns + {extra}: slider {got}, want {want}"
        );
        let j2 = sol.joints.iter().find(|j| j.joint == JointId(2)).unwrap();
        assert!(j2.in_loop);
    }
}

fn joint_unwrap(a: f64) -> f64 {
    a - dmath::TAU * (a / dmath::TAU).round()
}

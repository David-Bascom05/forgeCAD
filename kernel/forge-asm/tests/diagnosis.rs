//! Error typing (prior-art §11.8): deliberately broken assemblies, each giving exactly the
//! expected variant and naming the minimal set of constraints.

mod common;
use common::*;
use forge_asm::*;
use forge_math::{Frame, UnitVec3};

fn opts() -> Options {
    Options { threads: 1 }
}

fn prim(j: u32, i: u8) -> Item {
    Item::of(ConstraintId::Joint {
        joint: JointId(j),
        part: JointPart::Prim(i),
    })
}

fn loose(i: u32) -> Item {
    Item::of(ConstraintId::Loose(i))
}

fn conflicts(sol: &Solution) -> Vec<Vec<Item>> {
    sol.diagnostics
        .iter()
        .filter_map(|d| match d {
            Diagnostic::Conflict { items, .. } => Some(items.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_second_pivot_conflicts_with_the_first_and_only_those_are_named() {
    let mut asm = four_bar(40.0, 120.0, 80.0, 100.0, 0.6);
    // the crank pinned again at a point 10 from its pivot, to a ground point 15 away
    asm.joints.push(joint(
        5,
        JointKind::Revolute,
        (2, at(10.0, 0.0, 0.0)),
        (1, at(-15.0, 0.0, 0.0)),
    ));
    let sol = solve(&asm, &catalog(), &opts());
    assert_eq!(
        conflicts(&sol),
        vec![vec![prim(1, 0), prim(5, 0)]],
        "{:?}",
        sol.diagnostics
    );
    assert_eq!(sol.errors().count(), 1);
    // the culprit is left out; the rest still closes
    let j5 = sol.joints.iter().find(|j| j.joint == JointId(5)).unwrap();
    assert!(j5.failed);
    let rest = Assembly {
        joints: asm.joints[..4].to_vec(),
        ..asm.clone()
    };
    assert!(worst(&rest, &sol) < 1e-8);
    let c = sol
        .constraints
        .iter()
        .find(|c| {
            c.id == ConstraintId::Joint {
                joint: JointId(5),
                part: JointPart::Prim(0),
            }
        })
        .unwrap();
    assert!(c.failed && c.joint == Some(JointId(5)));
}

fn plate_on_ground(offsets: &[f64]) -> Assembly {
    let face = Reference::plane(p(0.0, 0.0, 0.0), UnitVec3::Z);
    Assembly {
        instances: vec![inst(1, Frame::WORLD).grounded(), inst(2, at(0.0, 0.0, 1.0))],
        constraints: offsets
            .iter()
            .enumerate()
            .map(|(i, &o)| {
                Constraint::new(
                    i as u32 + 1,
                    ConstraintKind::PlanePlane { offset: o },
                    End::reference(&[2], face),
                    End::reference(&[1], face),
                )
            })
            .collect(),
        ..Assembly::default()
    }
}

#[test]
fn two_flush_constraints_at_different_offsets_conflict() {
    let sol = solve(&plate_on_ground(&[0.0, 5.0]), &catalog(), &opts());
    assert_eq!(
        conflicts(&sol),
        vec![vec![loose(1), loose(2)]],
        "{:?}",
        sol.diagnostics
    );
    // the survivor holds the plate flush at the first offset
    assert!(sol.pose(InstanceId(2)).unwrap().origin.z.abs() < 1e-12);
}

#[test]
fn two_flush_constraints_that_agree_are_redundant() {
    let sol = solve(&plate_on_ground(&[2.0, 2.0]), &catalog(), &opts());
    assert_eq!(sol.errors().count(), 0, "{:?}", sol.diagnostics);
    assert_eq!(
        sol.diagnostics,
        vec![Diagnostic::Redundant {
            item: loose(2),
            rows: 3,
            implied_by: vec![loose(1)],
        }]
    );
    // a plate flush on a face keeps three freedoms (slide, slide, turn)
    assert_eq!(sol.mobility, 3);
    let c2 = sol
        .constraints
        .iter()
        .find(|c| c.id == ConstraintId::Loose(2))
        .unwrap();
    assert_eq!((c2.removes, c2.redundant), (3, 3));
}

#[test]
fn mate_and_insert_place_a_bolt() {
    // a bolt into a hole: insert (concentric + seated) leaves only the spin
    let hole = Reference::axis(p(10.0, 5.0, 0.0), UnitVec3::Z);
    let shank = Reference::axis(p(0.0, 0.0, 0.0), -UnitVec3::Z);
    let asm = Assembly {
        instances: vec![inst(1, Frame::WORLD).grounded(), inst(2, at(9.0, 6.0, 3.0))],
        constraints: vec![
            Constraint::new(
                1,
                ConstraintKind::Insert { offset: 0.0 },
                End::reference(&[2], shank),
                End::reference(&[1], hole),
            )
            .flipped(),
        ],
        ..Assembly::default()
    };
    let sol = solve(&asm, &catalog(), &opts());
    assert_eq!(sol.errors().count(), 0, "{:?}", sol.diagnostics);
    assert_eq!(sol.mobility, 1);
    let o = sol.pose(InstanceId(2)).unwrap().origin;
    assert!(o.distance(p(10.0, 5.0, 0.0)) < 1e-12, "{o:?}");
}

#[test]
fn fastened_cycles_are_decided_in_closed_form() {
    let mk = |off: f64| Assembly {
        instances: vec![
            inst(1, Frame::WORLD).grounded(),
            inst(2, at(10.0, 0.0, 0.0)),
            inst(3, at(20.0, 0.0, 0.0)),
        ],
        joints: vec![
            joint(1, JointKind::Fastened, (2, Frame::WORLD), (1, at(10.0, 0.0, 0.0))),
            joint(2, JointKind::Fastened, (3, Frame::WORLD), (2, at(10.0, 0.0, 0.0))),
            joint(
                3,
                JointKind::Fastened,
                (3, Frame::WORLD),
                (1, at(20.0 + off, 0.0, 0.0)),
            ),
        ],
        ..Assembly::default()
    };
    let sol = solve(&mk(0.0), &catalog(), &opts());
    assert_eq!(
        sol.diagnostics,
        vec![Diagnostic::Redundant {
            item: prim(3, 0),
            rows: 0,
            implied_by: vec![prim(1, 0), prim(2, 0)],
        }]
    );
    let sol = solve(&mk(0.5), &catalog(), &opts());
    assert_eq!(conflicts(&sol), vec![vec![prim(1, 0), prim(2, 0), prim(3, 0)]]);
}

#[test]
fn broken_references_are_typed_before_any_solve() {
    let mut parts = catalog();
    let mut info = PartInfo::default();
    info.refs.insert("hub".into(), Ok(Reference::frame(Frame::WORLD)));
    info.refs.insert(
        "boss".into(),
        Err(AnchorFault::EntityGone {
            entity: "boss.face.top".into(),
        }),
    );
    info.refs
        .insert("rim".into(), Ok(Reference::point(p(5.0, 0.0, 0.0))));
    parts.parts.insert((DocId("wheel".into()), 0), info);
    let wheel = |id| Instance::part(id, PartRef::new("wheel", 0), Frame::WORLD);
    let asm = Assembly {
        instances: vec![inst(1, Frame::WORLD).grounded(), wheel(2), wheel(3)],
        joints: vec![
            Joint::new(
                1,
                JointKind::Revolute,
                End::named(&[2], "hub"),
                End::local(&[1], Frame::WORLD),
            ),
            Joint::new(
                2,
                JointKind::Revolute,
                End::named(&[3], "axle"),
                End::local(&[1], Frame::WORLD),
            ),
            Joint::new(
                3,
                JointKind::Revolute,
                End::named(&[3], "boss"),
                End::local(&[1], Frame::WORLD),
            ),
            Joint::new(
                4,
                JointKind::Revolute,
                End::named(&[9], "hub"),
                End::local(&[1], Frame::WORLD),
            ),
        ],
        constraints: vec![Constraint::new(
            1,
            ConstraintKind::PointOnPlane { offset: 0.0 },
            End::named(&[3], "hub"),
            End::named(&[2], "rim"),
        )],
        ..Assembly::default()
    };
    let broken = check_anchors(&asm, &parts);
    let want = vec![
        Diagnostic::BrokenAnchor {
            constraint: None,
            joint: Some(JointId(2)),
            side: Side::A,
            anchor: "axle".into(),
            reason: AnchorReason::NoAnchor,
        },
        Diagnostic::BrokenAnchor {
            constraint: None,
            joint: Some(JointId(3)),
            side: Side::A,
            anchor: "boss".into(),
            reason: AnchorReason::Part(AnchorFault::EntityGone {
                entity: "boss.face.top".into(),
            }),
        },
        Diagnostic::BrokenAnchor {
            constraint: None,
            joint: Some(JointId(4)),
            side: Side::A,
            anchor: "hub".into(),
            reason: AnchorReason::NoInstance { at: 0 },
        },
    ];
    assert_eq!(broken, want);
    let sol = solve(&asm, &parts, &opts());
    for d in &want {
        assert!(sol.diagnostics.contains(d));
    }
    // a point where a plane is needed
    assert!(sol.diagnostics.iter().any(|d| matches!(
        d,
        Diagnostic::WrongReference {
            constraint: Some(ConstraintId::Loose(1)),
            side: Side::B,
            found: RefKind::Point,
            ..
        }
    )));
    // the good joint still places its wheel; the broken ones leave theirs where written
    assert!(sol.joints.iter().any(|j| j.joint == JointId(1)));
    assert_eq!(sol.pose(InstanceId(3)), Some(Frame::WORLD));
}

#[test]
fn no_ground_is_an_error_and_the_least_instance_holds() {
    let mut asm = four_bar(40.0, 120.0, 80.0, 100.0, 0.6);
    asm.instances[0].ground = false;
    let sol = solve(&asm, &catalog(), &opts());
    assert!(sol.diagnostics.contains(&Diagnostic::NoGround));
    assert_eq!(sol.pose(InstanceId(1)), Some(Frame::WORLD));
    assert_eq!(sol.mobility, 7);
}

#[test]
fn a_drive_past_its_limit_is_held_at_the_limit() {
    let asm = Assembly {
        instances: vec![inst(1, Frame::WORLD).grounded(), inst(2, Frame::WORLD)],
        joints: vec![
            joint(1, JointKind::Revolute, (2, Frame::WORLD), (1, Frame::WORLD))
                .limited(Freedom::Rz, -1.0, 1.0)
                .driven(Freedom::Rz, 1.5),
        ],
        ..Assembly::default()
    };
    let sol = solve(&asm, &catalog(), &opts());
    assert_eq!(
        sol.diagnostics,
        vec![Diagnostic::LimitViolated {
            joint: JointId(1),
            freedom: Freedom::Rz,
            value: 1.5,
            range: (-1.0, 1.0),
            fix: Some(Fix::SetDrive {
                joint: JointId(1),
                freedom: Freedom::Rz,
                value: 1.0,
            }),
        }]
    );
    assert_eq!(sol.joints[0].values, vec![(Freedom::Rz, 1.0)]);
}

#[test]
fn a_loop_held_at_a_limit_rests_there() {
    let mut asm = four_bar(40.0, 120.0, 80.0, 100.0, 0.6);
    asm.joints[0] = asm.joints[0].clone().driven(Freedom::Rz, 0.6);
    // the rocker may not turn past this; the drive asks for more
    let (_, t4) = four_bar_angles(40.0, 120.0, 80.0, 100.0, 0.6).unwrap();
    asm.joints[3] = asm.joints[3].clone().limited(Freedom::Rz, t4 - 0.01, t4 + 0.01);
    let parts = catalog();
    let mut s = Solver::new(asm, &parts, opts());
    assert_eq!(s.solution().errors().count(), 0);
    let sol = s.set_drive(JointId(1), Freedom::Rz, 1.2);
    assert!(
        sol.diagnostics.iter().any(|d| matches!(
            d,
            Diagnostic::LimitViolated {
                joint: JointId(4),
                ..
            }
        )),
        "{:?}",
        sol.diagnostics
    );
}

#[test]
fn a_joint_inside_a_rigid_cluster_is_checked_against_it() {
    let mk = |x: f64| Assembly {
        instances: vec![inst(1, Frame::WORLD).grounded(), inst(2, at(10.0, 0.0, 0.0))],
        joints: vec![
            joint(1, JointKind::Fastened, (2, Frame::WORLD), (1, at(10.0, 0.0, 0.0))),
            joint(
                2,
                JointKind::Revolute,
                (2, at(1.0, 0.0, 0.0)),
                (1, at(x, 0.0, 0.0)),
            ),
        ],
        ..Assembly::default()
    };
    let sol = solve(&mk(11.0), &catalog(), &opts());
    assert_eq!(sol.errors().count(), 0);
    assert_eq!(redundant_rows(&sol), 0);
    assert_eq!(common::count(&sol, "Redundant"), 2);
    let sol = solve(&mk(12.0), &catalog(), &opts());
    assert_eq!(conflicts(&sol), vec![vec![prim(1, 0), prim(2, 0)]]);
}

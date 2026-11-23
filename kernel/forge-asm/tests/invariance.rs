//! Rigid-motion invariance (audit 2 §3.1): moving a whole assembly, its ground and every written
//! pose by one rigid motion, moves the solved poses by that motion and changes no other answer:
//! the diagnostics and what they name, mobility, every block's rank and every joint's free
//! motions and values. Every decision is made in the arrangement's own frame (its lever about its
//! centroid), never in world coordinates.

mod common;
use common::*;
use forge_asm::*;
use forge_math::{Frame, dmath};

fn opts() -> Options {
    Options { threads: 1 }
}

/// The assembly with every instance moved by `m`.
fn moved(asm: &Assembly, m: &Frame) -> Assembly {
    let mut out = asm.clone();
    for i in &mut out.instances {
        i.pose = m.compose(&i.pose);
    }
    out
}

/// What a diagnostic says, without its measured numbers (which carry the move's rounding).
fn said(d: &Diagnostic) -> String {
    match d {
        Diagnostic::Conflict { items, .. } => format!("Conflict {items:?}"),
        Diagnostic::NotConverged { block, items, .. } => format!("NotConverged {block} {items:?}"),
        Diagnostic::LimitViolated {
            joint,
            freedom,
            range,
            ..
        } => format!("LimitViolated {joint:?} {freedom:?} {range:?}"),
        d => format!("{d:?}"),
    }
}

fn same_answers(name: &str, asm: &Assembly, m: &Frame) {
    let a = solve(asm, &catalog(), &opts());
    let b = solve(&moved(asm, m), &catalog(), &opts());
    let what = format!("{name} moved by {:?}", m.origin);
    let da: Vec<String> = a.diagnostics.iter().map(said).collect();
    let db: Vec<String> = b.diagnostics.iter().map(said).collect();
    assert_eq!(da, db, "{what}: diagnostics");
    assert_eq!(a.mobility, b.mobility, "{what}: mobility");
    let blocks = |s: &Solution| -> Vec<_> {
        s.blocks
            .iter()
            .map(|x| (x.block, x.rows, x.rank, x.generic_rank, x.rank_clear, x.mobility))
            .collect()
    };
    assert_eq!(blocks(&a), blocks(&b), "{what}: blocks");
    assert_eq!(a.joints.len(), b.joints.len());
    for (x, y) in a.joints.iter().zip(&b.joints) {
        assert_eq!(
            (x.joint, x.free, x.in_loop, x.failed),
            (y.joint, y.free, y.in_loop, y.failed),
            "{what}"
        );
        for ((f, u), (g, w)) in x.values.iter().zip(&y.values) {
            assert_eq!(f, g);
            assert!(
                (u - w).abs() < 1e-6,
                "{what}: {:?} {f:?} {u} against {w}",
                x.joint
            );
        }
    }
    let reach = m.origin.to_vec().length().max(1.0);
    for (id, p) in &a.poses {
        let q = b.pose(*id).expect("placed in both");
        let want = m.compose(p);
        assert!(
            want.origin.distance(q.origin) < 1e-9 * reach + 1e-7,
            "{what}: {id:?} at {:?}, want {:?}",
            q.origin,
            want.origin
        );
        assert!((want.z().get() - q.z().get()).length() < 1e-9, "{what}: {id:?}");
        assert!((want.x().get() - q.x().get()).length() < 1e-9, "{what}: {id:?}");
    }
}

fn motions() -> Vec<Frame> {
    let turn = |o: (f64, f64, f64), z: (f64, f64, f64), x: (f64, f64, f64)| {
        frame(p(o.0, o.1, o.2), v(z.0, z.1, z.2), v(x.0, x.1, x.2))
    };
    vec![
        at(1000.0, -2000.0, 500.0),
        at(1e5, 1e5, -1e5),
        turn((0.0, 0.0, 0.0), (0.3, -0.5, 0.8), (1.0, 0.2, 0.0)),
        turn((-3e4, 7e3, 2e4), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)),
        Frame::WORLD.rotated_about_z(2.5).with_origin(p(250.0, 4e4, -9.0)),
    ]
}

/// A cycle of fastened joints that closes to an angle of `misfit` radians at a lever of 50 mm:
/// within the tolerance (5e-9 · 50 mm = 2.5e-7 mm), so the cycle is redundant wherever it is.
fn fastened_cycle(misfit: f64) -> Assembly {
    let instances = vec![
        inst(1, Frame::WORLD).grounded(),
        inst(2, at(50.0, 0.0, 0.0)),
        inst(3, at(50.0, 50.0, 0.0)),
    ];
    let joints = vec![
        joint(1, JointKind::Fastened, (2, Frame::WORLD), (1, at(50.0, 0.0, 0.0))),
        joint(2, JointKind::Fastened, (3, Frame::WORLD), (2, at(0.0, 50.0, 0.0))),
        joint(
            3,
            JointKind::Fastened,
            (3, Frame::WORLD),
            (1, rz(p(50.0, 50.0, 0.0), misfit)),
        ),
    ];
    Assembly {
        instances,
        joints,
        ..Assembly::default()
    }
}

/// Two revolutes on one axis between the same two bodies, closed through a third: a loop block
/// with redundant rows.
fn doubled_hinge() -> Assembly {
    let instances = vec![
        inst(1, Frame::WORLD).grounded(),
        inst(2, rz(p(0.0, 0.0, 0.0), 0.4)),
    ];
    let joints = vec![
        joint(1, JointKind::Revolute, (2, Frame::WORLD), (1, Frame::WORLD)),
        joint(
            2,
            JointKind::Revolute,
            (2, at(0.0, 0.0, 30.0)),
            (1, at(0.0, 0.0, 30.0)),
        ),
    ];
    Assembly {
        instances,
        joints,
        ..Assembly::default()
    }
}

#[test]
fn moving_the_whole_assembly_changes_no_answer() {
    let cases = vec![
        ("four-bar", four_bar(40.0, 120.0, 80.0, 100.0, 0.6)),
        ("slider-crank", slider_crank(30.0, 90.0, 0.7)),
        ("fastened cycle", fastened_cycle(5e-9)),
        // (met at the origin by the old world-sized tolerance, a conflict a hundred metres off)
        ("fastened cycle, finer", fastened_cycle(1e-9)),
        ("fastened cycle, off", fastened_cycle(1e-3)),
        ("doubled hinge", doubled_hinge()),
    ];
    for (name, asm) in &cases {
        for m in motions() {
            same_answers(name, asm, &m);
        }
    }
}

/// The cycle's verdict is the tolerance's, at the cycle's own lever: met under it, a conflict over
/// it, at the origin and a hundred metres away alike.
#[test]
fn a_fastened_cycle_is_met_or_not_by_its_own_lever() {
    for m in [Frame::WORLD, at(1e5, 0.0, 0.0)] {
        let ok = solve(&moved(&fastened_cycle(5e-9), &m), &catalog(), &opts());
        assert_eq!(ok.errors().count(), 0, "{:?}", ok.diagnostics);
        let off = solve(&moved(&fastened_cycle(1e-3), &m), &catalog(), &opts());
        assert!(
            off.diagnostics
                .iter()
                .any(|d| matches!(d, Diagnostic::Conflict { .. })),
            "{:?}",
            off.diagnostics
        );
    }
    let _ = dmath::PI;
}

//! Construct-then-solve (prior-art §11.2): choose poses first, derive joints that hold there, start
//! from perturbed written poses. Trees must return the chosen poses exactly; loops must satisfy
//! the residual oracle (and with every value driven, the chosen poses too).

mod common;
use common::*;
use forge_asm::*;
use forge_math::rng::Rng;
use forge_math::{Frame, dmath};

fn rand_frame(r: &mut Rng, reach: f64) -> Frame {
    let z = v(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
    let x = v(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
    let o = p(
        r.range(-reach, reach),
        r.range(-reach, reach),
        r.range(-reach, reach),
    );
    Frame::from_zx(o, z.normalize().unwrap_or(forge_math::UnitVec3::Z), x)
        .unwrap_or(Frame::WORLD.with_origin(o))
}

/// The joint's motion for its values (the test's own statement of the conventions).
fn motion(kind: JointKind, q: &[f64], ball: &Frame) -> Frame {
    match kind {
        JointKind::Fastened => Frame::WORLD,
        JointKind::Revolute => rz(p(0.0, 0.0, 0.0), q[0]),
        JointKind::Slider => at(0.0, 0.0, q[0]),
        JointKind::Cylindrical => rz(p(0.0, 0.0, q[0]), q[1]),
        JointKind::PinSlot => rz(p(q[0], 0.0, 0.0), q[1]),
        JointKind::Planar => rz(p(q[0], q[1], 0.0), q[2]),
        JointKind::Ball => ball.with_origin(p(0.0, 0.0, 0.0)),
    }
}

const KINDS: [JointKind; 7] = [
    JointKind::Fastened,
    JointKind::Revolute,
    JointKind::Slider,
    JointKind::Cylindrical,
    JointKind::PinSlot,
    JointKind::Planar,
    JointKind::Ball,
];

/// A joint between `child` and `parent` that holds at the poses `truth`, with values chosen.
fn derive(
    r: &mut Rng,
    id: u32,
    kind: JointKind,
    child: u32,
    parent: u32,
    truth: &[Frame],
    drive: bool,
) -> Joint {
    let wb = rand_frame(r, 60.0);
    let q: Vec<f64> = kind
        .freedoms()
        .iter()
        .map(|f| {
            if f.is_angle() {
                r.range(-3.0, 3.0)
            } else {
                r.range(-40.0, 40.0)
            }
        })
        .collect();
    let ball = rand_frame(r, 0.0);
    let wa_eff = wb.compose(&motion(kind, &q, &ball));
    let flip = r.bool();
    let wa = if flip { wa_eff.flipped() } else { wa_eff };
    let la = truth[child as usize - 1].inverse().compose(&wa);
    let lb = truth[parent as usize - 1].inverse().compose(&wb);
    let mut j = joint(id, kind, (child, la), (parent, lb));
    j.flip = flip;
    if drive && kind != JointKind::Ball {
        for (f, v) in kind.freedoms().iter().zip(&q) {
            j = j.driven(*f, *v);
        }
    }
    j
}

fn nudge(r: &mut Rng, f: &Frame, s: f64) -> Frame {
    let w = v(r.range(-s, s), r.range(-s, s), r.range(-s, s)) * 0.01;
    let t = w.length();
    let rot = |u: forge_math::Vec3| {
        if t == 0.0 {
            return u;
        }
        let k = w / t;
        let (sn, cs) = dmath::sin_cos(t);
        u * cs + k.cross(u) * sn + k * (k.dot(u) * (1.0 - cs))
    };
    let o = f.origin + v(r.range(-s, s), r.range(-s, s), r.range(-s, s));
    Frame::from_zx(o, rot(f.z().get()).normalize().unwrap(), rot(f.x().get())).unwrap()
}

fn close(a: &Frame, b: &Frame, tol: f64) -> bool {
    a.origin.distance(b.origin) < tol
        && (a.x().get() - b.x().get()).length() < tol
        && (a.z().get() - b.z().get()).length() < tol
}

fn build(r: &mut Rng, n: usize, extra: usize, drive: bool, balls: bool) -> (Assembly, Vec<Frame>) {
    let truth: Vec<Frame> = (0..n)
        .map(|i| if i == 0 { Frame::WORLD } else { rand_frame(r, 80.0) })
        .collect();
    let mut joints = Vec::new();
    let pick = |r: &mut Rng| loop {
        let k = KINDS[r.below(KINDS.len())];
        if balls || k != JointKind::Ball {
            break k;
        }
    };
    for c in 2..=n as u32 {
        let parent = 1 + r.below(c as usize - 1) as u32;
        let k = pick(r);
        joints.push(derive(r, c - 1, k, c, parent, &truth, drive));
    }
    for e in 0..extra {
        let a = 2 + r.below(n - 1) as u32;
        let mut b = 1 + r.below(n) as u32;
        if b == a {
            b = 1;
        }
        let k = pick(r);
        joints.push(derive(r, (n + e) as u32, k, a, b, &truth, drive));
    }
    let asm = Assembly {
        instances: (0..n)
            .map(|i| {
                let i0 = inst(i as u32 + 1, truth[i]);
                if i == 0 { i0.grounded() } else { i0 }
            })
            .collect(),
        joints,
        ..Assembly::default()
    };
    (asm, truth)
}

#[test]
fn driven_trees_come_back_to_their_chosen_poses() {
    let mut r = Rng::new(0xa55e);
    for case in 0..300 {
        let n = 2 + r.below(12);
        let (mut asm, truth) = build(&mut r, n, 0, true, false);
        for i in asm.instances.iter_mut().skip(1) {
            i.pose = nudge(&mut r, &i.pose, 2.0);
        }
        let sol = solve(&asm, &catalog(), &Options { threads: 1 });
        assert_eq!(sol.errors().count(), 0, "case {case}: {:?}", sol.diagnostics);
        assert_eq!(sol.stats.blocks_solved, 0, "trees take no Newton");
        for (i, t) in truth.iter().enumerate() {
            let got = sol.pose(InstanceId(i as u32 + 1)).unwrap();
            assert!(
                close(&got, t, 1e-9),
                "case {case} instance {}: {got:?} vs {t:?}",
                i + 1
            );
        }
        assert!(worst(&asm, &sol) < 1e-8);
    }
}

#[test]
fn undriven_trees_with_balls_keep_true_written_poses() {
    let mut r = Rng::new(0xba11);
    for case in 0..200 {
        let n = 2 + r.below(10);
        let (asm, truth) = build(&mut r, n, 0, false, true);
        let sol = solve(&asm, &catalog(), &Options { threads: 1 });
        assert_eq!(sol.errors().count(), 0, "case {case}");
        for (i, t) in truth.iter().enumerate() {
            assert!(
                close(&sol.pose(InstanceId(i as u32 + 1)).unwrap(), t, 1e-9),
                "case {case}"
            );
        }
    }
}

#[test]
fn loops_close_from_perturbed_starts() {
    let mut r = Rng::new(0x100b);
    let mut loops = 0;
    for case in 0..300 {
        let n = 3 + r.below(6);
        let extra = 1 + r.below(2);
        let drive = case % 2 == 0;
        let (mut asm, truth) = build(&mut r, n, extra, drive, true);
        for i in asm.instances.iter_mut().skip(1) {
            i.pose = nudge(&mut r, &i.pose, 0.05);
        }
        let sol = solve(&asm, &catalog(), &Options { threads: 1 });
        assert_eq!(sol.errors().count(), 0, "case {case}: {:?}", sol.diagnostics);
        assert!(worst(&asm, &sol) < 1e-7, "case {case}: {}", worst(&asm, &sol));
        loops += sol.stats.blocks_solved;
        if drive {
            // every value given: the chosen poses, wherever ball joints leave none free
            if sol.mobility == 0 {
                for (i, t) in truth.iter().enumerate() {
                    assert!(
                        close(&sol.pose(InstanceId(i as u32 + 1)).unwrap(), t, 1e-7),
                        "case {case}"
                    );
                }
            }
        }
    }
    assert!(loops > 200, "{loops} loop blocks");
}

/// Each joint written as its bundle of loose constraints (pin-slot kept a joint: its line is not
/// its anchor's frame). `rotate_parallel` turns the Parallel's A frame about its z, which leaves the
/// rows' meaning alone but no longer reads as a joint: the numeric path.
fn loosen(asm: &Assembly, r: &mut Rng, rotate_parallel: bool) -> Assembly {
    let mut out = asm.clone();
    out.joints.clear();
    for j in &asm.joints {
        if j.kind == JointKind::PinSlot {
            out.joints.push(j.clone());
            continue;
        }
        let kinds: Vec<ConstraintKind> = match (j.kind, r.bool()) {
            (JointKind::Revolute, true) => vec![ConstraintKind::Insert { offset: 0.0 }],
            (JointKind::Cylindrical, true) => vec![ConstraintKind::Concentric],
            (JointKind::Planar, true) => vec![ConstraintKind::PlanePlane { offset: 0.0 }],
            _ => j.kind.bundle().to_vec(),
        };
        for k in kinds {
            let mut a = j.a.clone();
            if rotate_parallel && k == ConstraintKind::Parallel {
                let AnchorRef::Local(rf) = &a.anchor else {
                    unreachable!()
                };
                a.anchor = AnchorRef::Local(Reference::frame(rf.frame.rotated_about_z(0.3)));
            }
            let id = out.constraints.len() as u32 + 1;
            let mut c = Constraint::new(id, k, a, j.b.clone());
            c.flip = j.flip;
            out.constraints.push(c);
        }
    }
    out
}

#[test]
fn loose_bundles_in_trees_place_as_their_joints() {
    let mut r = Rng::new(0xb0d1e);
    for case in 0..200 {
        let n = 2 + r.below(10);
        let (mut asm, _) = build(&mut r, n, 0, false, true);
        for i in asm.instances.iter_mut().skip(1) {
            i.pose = nudge(&mut r, &i.pose, 2.0);
        }
        let loose = loosen(&asm, &mut r, false);
        let (a, b) = (
            solve(&asm, &catalog(), &Options { threads: 1 }),
            solve(&loose, &catalog(), &Options { threads: 1 }),
        );
        assert_eq!(b.errors().count(), 0, "case {case}: {:?}", b.diagnostics);
        assert_eq!(
            b.stats.blocks_solved, 0,
            "case {case}: every bundle read as its joint"
        );
        assert_eq!(a.mobility, b.mobility);
        for (x, y) in a.poses.iter().zip(&b.poses) {
            assert!(close(&x.1, &y.1, 1e-12), "case {case} instance {:?}", x.0);
        }
    }
}

#[test]
fn recognised_bundles_match_the_numeric_path() {
    let mut r = Rng::new(0x5a3e);
    let mut numeric_blocks = 0;
    for case in 0..200 {
        let n = 2 + r.below(10);
        let (asm, truth) = build(&mut r, n, 0, false, true);
        let mut r2 = r.clone();
        let closed = solve(&loosen(&asm, &mut r, false), &catalog(), &Options { threads: 1 });
        let numeric = solve(&loosen(&asm, &mut r2, true), &catalog(), &Options { threads: 1 });
        assert_eq!(numeric.errors().count(), 0, "case {case}");
        numeric_blocks += numeric.stats.blocks_solved;
        assert_eq!(closed.stats.blocks_solved, 0);
        assert_eq!(closed.mobility, numeric.mobility, "case {case}");
        for ((x, y), t) in closed.poses.iter().zip(&numeric.poses).zip(&truth) {
            assert!(close(&x.1, &y.1, 1e-12), "case {case} instance {:?}", x.0);
            assert!(close(&x.1, t, 1e-12));
        }
    }
    assert!(
        numeric_blocks > 300,
        "{numeric_blocks} bundles went the numeric way"
    );
}

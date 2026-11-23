//! Two walls crossing at a tiny angle, against the exact reference (research/kernel-audit/
//! tilt-wrong-ok.md). A is the box [0,L]×[0,10]×[0,10]; B the prism under A's y = 0 wall whose top
//! wall runs from (0, y0) to (L, y1), extruded 10 (`examples/tilt_probe.rs` prints every answer).
//! Each test asks that the kernel agree with the reference on one op; each fails today, so each is
//! ignored with today's kernel answer (`cargo test -p forge-exact-fuzz --test tilt_repros --
//! --ignored`, after `lake build` in verify/forge-exact). In every case below the reference
//! answers a solid with no ambiguity notes.
//!
//! B1: L = 1.6e7 (inside InputSize), y0 = -2e-5, y1 = 6e-5: |n1×n2| = 5e-12 ≤ ANGULAR, so
//! forge-geom's `plane_plane` decides at a frame origin and says Empty both ways round, though
//! the walls cross at x = 4e6. B2: y0 = -1.2e-5, y1 = 1.2e-5 (the angle is past ANGULAR and
//! `plane_plane` returns the crossing line): the root cause is UNKNOWN, downstream of D2.

use forge_bool::Op;
use forge_exact_fuzz::pairs::{kernel_pair, pair_input};
use forge_exact_fuzz::{Class, Reference, classify};
use forge_math::{Frame, Point2, Point3};
use forge_ops::{Profile, ProfileLoop, Segment, extrude};
use forge_topo::Body;

fn prism(pts: &[(f64, f64)], h: f64, feature: u64) -> Body {
    let n = pts.len();
    let segs = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            (
                Segment::Line {
                    a: Point2::new(a.0, a.1),
                    b: Point2::new(b.0, b.1),
                },
                feature * 10 + i as u64,
            )
        })
        .collect();
    let profile = Profile {
        plane: Frame::WORLD.with_origin(Point3::new(0.0, 0.0, 0.0)),
        loops: vec![ProfileLoop::new(segs)],
    };
    extrude(&profile, h, feature).expect("a prism extrudes")
}

/// A and B of the family.
fn tilt(l: f64, y0: f64, y1: f64) -> (Body, Body) {
    let a = prism(&[(0.0, 0.0), (l, 0.0), (l, 10.0), (0.0, 10.0)], 10.0, 1);
    let b = prism(&[(0.0, -10.0), (l, -10.0), (l, y1), (0.0, y0)], 10.0, 2);
    (a, b)
}

/// The reference answers a solid with no notes, and the kernel agrees with it.
fn agrees(l: f64, y0: f64, y1: f64, op: Op) {
    let (a, b) = tilt(l, y0, y1);
    let mut r = Reference::start();
    let ans = r.ask(&pair_input(&a, &b, op).expect("flat"));
    assert_eq!(ans.verdict, "solid", "reference: {ans:?}");
    assert!(ans.notes.is_empty(), "reference: {ans:?}");
    let k = kernel_pair(&a, &b, op);
    let c = classify(&ans, &k);
    assert!(
        matches!(c, Class::Agree),
        "reference: {ans:?}\nkernel: {k:?}\n{} {}",
        c.key(),
        c.detail()
    );
}

// B1: plane_plane's parallel branch, L = 1.6e7.
#[test]
#[ignore = "B1: kernel refuses ZeroThickness at (8e6, 2e-5, 0); reference: solid"]
fn b1_union() {
    agrees(1.6e7, -2e-5, 6e-5, Op::Union);
}
#[test]
#[ignore = "B1: kernel refuses ZeroThickness at (1.6e7, 6e-5, 0); reference: solid"]
fn b1_subtract() {
    agrees(1.6e7, -2e-5, 6e-5, Op::Subtract);
}
#[test]
#[ignore = "B1: WRONG OK: kernel returns an invalid body (EdgeUses); reference: solid"]
fn b1_intersect() {
    agrees(1.6e7, -2e-5, 6e-5, Op::Intersect);
}

// B2: the thin wedge, ±1.2e-5. Wrong Ok at L = 1e5 … 2e6.
#[test]
#[ignore = "B2: WRONG OK at L = 1e5, 3e5, 1e6, 2e6: kernel returns an invalid body (EdgeUses); reference: solid"]
fn b2_intersect_wrong_ok() {
    for l in [1e5, 3e5, 1e6, 2e6] {
        agrees(l, -1.2e-5, 1.2e-5, Op::Intersect);
    }
}
#[test]
#[ignore = "B2: kernel refuses ZeroThickness at L = 1e5 … 2e6; reference: solid"]
fn b2_union_subtract() {
    for l in [1e5, 3e5, 1e6, 2e6] {
        for op in [Op::Union, Op::Subtract] {
            agrees(l, -1.2e-5, 1.2e-5, op);
        }
    }
}
#[test]
#[ignore = "B2: kernel refuses Internal(InconsistentClassification) at L = 1e3, 1e4 (every op); reference: solid"]
fn b2_small() {
    for l in [1e3, 1e4] {
        for op in [Op::Union, Op::Subtract, Op::Intersect] {
            agrees(l, -1.2e-5, 1.2e-5, op);
        }
    }
}

/// The controls either side, which agree today: the same 2·LINEAR gaps with an angle past
/// ANGULAR, and B1's wall turned past ANGULAR (±1e-4 at 1.6e7).
#[test]
#[cfg_attr(
    not(feature = "reference"),
    ignore = "asks the Lean reference: lake build in verify/forge-exact, then cargo test -p forge-exact-fuzz --features reference"
)]
fn controls_agree() {
    for (l, y0, y1) in [
        (1e3, -2e-5, 2e-5),
        (1e6, -2e-5, 2e-5),
        (2e6, -2e-5, 2e-5),
        (1e4, -2e-5, 6e-5),
        (1.6e7, -1e-4, 1e-4),
    ] {
        for op in [Op::Union, Op::Subtract, Op::Intersect] {
            agrees(l, y0, y1, op);
        }
    }
}

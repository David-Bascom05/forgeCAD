//! G5 (research/kernel-audit/coverage.md §2): a face drafted or tilted by θ against a face of the
//! other body it nearly lies on, with a corner of one body d = 5 θ² (drafted lid) or 2.5 θ²
//! (`cap`) off the other's face. Two bands, each `#[ignore]`d with its site while it fails, and
//! the band edges pinned by passing tests.
//!
//! G5b, θ > NEAR (1e-4) and d ≤ LINEAR: the two faces are within LINEAR of each other everywhere
//! they overlap, so the tolerance rule (ForgeExact.Snap `tolEq`) merges them and the result is a
//! valid body. `nearly_flush` (forge-bool ssi.rs:66, `c <= NEAR`) refuses to merge them past its
//! angle cap, which leaves a strip of a face within LINEAR of the other body everywhere. That
//! strip is then refused: by the ray probe (classify_specs, boolean.rs:6018-6056,
//! UntouchedOnOther) in the drafted lid, and by the four-use test (1739-1749, ZeroThickness) in
//! `cap`'s subtract.
//!
//! G5a, LINEAR < d ≤ φ·LINEAR: the corner is off the other body's edge, since d > LINEAR. The
//! coplanar imprint samples the stretch of the corner's edge between the corner and the crossing
//! at 0.5, then 0.382, then 0.618 (boolean.rs:4227-4244). Those samples are (1 - f) d from the
//! edge, so all three read On while d ≤ LINEAR / 0.618 = φ·LINEAR, and the stretch is dropped as
//! lying along the boundary. The region it should close is left with a spur. The result is
//! InconsistentClassification in the flood (6004-6013) for `cap`, and ZeroThickness at the
//! four-use test (1746) for the drafted lid's add.
use forge_bool::Op;
use forge_coverage::Outcome;
use forge_coverage::draft::{expected, part};
use forge_coverage::kernel_on;
use forge_coverage::tilt::{family, reading};
use forge_model::Combine;

const OPS: [Combine; 3] = [Combine::Add, Combine::Cut, Combine::Keep];

/// The drafted part on both regen paths: right when valid and within 1e-6 relative of the exact.
fn draft_right(deg: f64, op: Combine) -> Result<(), String> {
    let angle = deg.to_radians();
    let want = expected(angle, op);
    let p = part(angle, op);
    for (path, r) in [("sequential", p.regenerate()), ("nary", p.regenerate_nary(1))] {
        let b = r.map_err(|e| format!("{deg} deg {op:?} {path}: {e:?}"))?;
        if let Some(pr) = forge_topo::check(&b).first() {
            return Err(format!("{deg} deg {op:?} {path}: invalid {pr:?}"));
        }
        let q = forge_mesh::Quality {
            chord: 0.02,
            max_angle: 0.5,
        };
        let v = forge_mesh::tessellate(&b, &q)
            .map_err(|e| format!("{e:?}"))?
            .volume();
        if (v - want).abs() > 1e-6 * want.max(1.0) {
            return Err(format!("{deg} deg {op:?} {path}: volume {v}, exact {want}"));
        }
    }
    Ok(())
}

fn tilt_right(fam: &str, th: f64, op: Op) -> Result<(), String> {
    let ((ab, ap), (bb, bp)) = family(fam, th);
    let got = kernel_on(ab, bb, op);
    let want = reading(&ap, &bp, op);
    match got {
        Outcome::Ok(v) if (v - want).abs() <= 1e-6 * want.abs().max(1.0) => Ok(()),
        Outcome::Empty if want.abs() <= 1e-6 => Ok(()),
        o => Err(format!("{fam} {op:?} at {th:e}: {} (exact {want})", o.word())),
    }
}

#[test]
#[ignore = "coverage gap G5b: 0.05 deg drafted lid (d = 0.38 LINEAR, theta > NEAR): UntouchedOnOther at boolean.rs:6056, the pair kept apart by nearly_flush's NEAR cap ssi.rs:66"]
fn g5b_drafted_lid_0_05_deg() {
    for op in OPS {
        draft_right(0.05, op).unwrap();
    }
}

#[test]
#[ignore = "coverage gap G5b: cap subtract at theta 1e-3 (d = 0.25 LINEAR): ZeroThickness at the four-use test boolean.rs:1746, the pair kept apart by nearly_flush's NEAR cap ssi.rs:66"]
fn g5b_cap_subtract_1e_3() {
    tilt_right("cap", 1e-3, Op::Subtract).unwrap();
}

#[test]
#[ignore = "coverage gap G5a: 0.1 deg drafted lid add (d = 1.52 LINEAR): imprint stretch samples 0.5/0.382/0.618 all On at boolean.rs:4227-4244, ZeroThickness at 1746"]
fn g5a_drafted_lid_0_1_deg_add() {
    draft_right(0.1, Combine::Add).unwrap();
}

#[test]
#[ignore = "coverage gap G5a: cap at theta 2.2e-3 (d = 1.21 LINEAR): imprint stretch samples all On at boolean.rs:4227-4244, InconsistentClassification at 6004-6013"]
fn g5a_cap_2_2e_3() {
    for op in [Op::Union, Op::Subtract, Op::Intersect] {
        tilt_right("cap", 2.2e-3, op).unwrap();
    }
}

/// The bands' edges, in the drafted lid: NEAR and LINEAR below and above G5b, LINEAR and φ·LINEAR
/// below and above G5a (d = 5 θ²: 0.0057, 0.0810 and 0.1031 degrees).
#[test]
fn g5_drafted_lid_edges() {
    for op in OPS {
        draft_right(0.005, op).unwrap(); // θ = 8.7e-5 < NEAR: merged, right
        draft_right(0.105, op).unwrap(); // d = 1.68 LINEAR > φ·LINEAR: right
        draft_right(0.2, op).unwrap();
    }
    // inside G5a only the add fails; cut and keep are right
    draft_right(0.1, Combine::Cut).unwrap();
    draft_right(0.1, Combine::Keep).unwrap();
    // today, inside the bands
    assert!(
        draft_right(0.006, Combine::Add).is_err(),
        "G5b's lower edge moved"
    );
    assert!(draft_right(0.08, Combine::Keep).is_err(), "G5b changed");
    assert!(
        draft_right(0.103, Combine::Add).is_err(),
        "G5a's upper edge moved"
    );
}

/// The bands' edges in `cap` (d = 2.5 θ²): G5a spans θ ∈ (2.0e-3, 2.5441e-3].
#[test]
fn g5_cap_edges() {
    for op in [Op::Union, Op::Subtract, Op::Intersect] {
        tilt_right("cap", 2.5442e-3, op).unwrap();
        tilt_right("cap", 3e-3, op).unwrap();
        assert!(
            tilt_right("cap", 2.5440e-3, op).is_err(),
            "G5a's upper edge moved"
        );
    }
    tilt_right("cap", 9e-5, Op::Subtract).unwrap(); // θ < NEAR: merged, right
    tilt_right("cap", 1.99e-3, Op::Union).unwrap(); // d < LINEAR: only the subtract (G5b) fails
}

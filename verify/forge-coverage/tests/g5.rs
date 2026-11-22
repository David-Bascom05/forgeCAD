//! G5 (research/kernel-audit/coverage.md §2): a face drafted or tilted by θ against a face of the
//! other body it nearly lies on, with a corner of one body d = 5 θ² (drafted lid) or 2.5 θ²
//! (`cap`) off the other's face. Two bands, closed: each
//! test names the site that refused before, and the sweeps cross both bands' edges.
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
fn g5b_drafted_lid_0_05_deg() {
    for op in OPS {
        draft_right(0.05, op).unwrap();
    }
}

#[test]
fn g5b_cap_subtract_1e_3() {
    tilt_right("cap", 1e-3, Op::Subtract).unwrap();
}

#[test]
fn g5a_drafted_lid_0_1_deg_add() {
    draft_right(0.1, Combine::Add).unwrap();
}

#[test]
fn g5a_cap_2_2e_3() {
    for op in [Op::Union, Op::Subtract, Op::Intersect] {
        tilt_right("cap", 2.2e-3, op).unwrap();
    }
}

/// The bands' edges, and across them, in the drafted lid: NEAR, LINEAR and φ·LINEAR at 0.0057,
/// 0.0810 and 0.1031 degrees (d = 5 θ²). Every draft from 0.0002 degrees up builds right, on all
/// three ops (0.0003-0.0011 degrees, where the bottom dips 2.6-9.6 LINEAR across the block's top,
/// is the probe floor's band, G3/G4, closed by the probe fix). Below, the bottom dips under
/// 1.6 LINEAR: a sub-LINEAR sliver, not G5.
#[test]
fn g5_drafted_lid_edges() {
    let mut bad = Vec::new();
    let degs = [
        0.0002, 0.0003, 0.0005, 0.0007, 0.001, 0.0012, 0.002, 0.005, 0.0057, 0.0058, 0.006, 0.01, 0.02, 0.03,
        0.05, 0.07, 0.08, 0.0809, 0.0811, 0.085, 0.09, 0.095, 0.1, 0.102, 0.103, 0.1031, 0.1032, 0.105, 0.11,
        0.15, 0.2, 0.5, 1.0,
    ];
    for deg in degs {
        for op in OPS {
            if let Err(e) = draft_right(deg, op) {
                bad.push(e);
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// The bands in `cap` (d = 2.5 θ²: NEAR at 1e-4, LINEAR at 2.0e-3, φ·LINEAR at 2.5441e-3), and
/// across them: every θ right on all three ops.
#[test]
fn g5_cap_edges() {
    let mut bad = Vec::new();
    let ths = [
        5e-5, 9e-5, 1e-4, 1.1e-4, 2e-4, 5e-4, 1e-3, 1.5e-3, 1.99e-3, 1.997e-3, 2.0001e-3, 2.01e-3, 2.2e-3,
        2.4e-3, 2.544e-3, 2.5441e-3, 2.5442e-3, 2.6e-3, 3e-3, 5e-3, 1e-2,
    ];
    for th in ths {
        for op in [Op::Union, Op::Subtract, Op::Intersect] {
            if let Err(e) = tilt_right("cap", th, op) {
                bad.push(e);
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// What is left of G5: a knife edge just under d = LINEAR, d in (LINEAR (1 - θ), LINEAR]. The
/// tolerance rule's region (the faces' boxes grown by LINEAR) reaches LINEAR past the dipped
/// corner, where the pair's gap is d + θ LINEAR > LINEAR, so the pair is kept apart, as the
/// reference keeps it (ForgeExact.Snap `tolEq`); but the corner itself is within LINEAR of the
/// other face's edge, so it is decided on it and the imprint drops its stretch. Measured: the
/// drafted lid 0.080968 degrees right, 0.080988 to 0.081028 refused; `cap` 1.9975e-3 right,
/// 1.998e-3 to 2e-3 refused.
#[test]
fn g5_knife_edge_at_linear() {
    // the lid at d = 0.9993 LINEAR, `cap` at d = LINEAR
    assert!(draft_right(0.081, Combine::Add).is_err(), "the knife edge moved");
    assert!(
        tilt_right("cap", 2e-3, Op::Union).is_err(),
        "the knife edge moved"
    );
}

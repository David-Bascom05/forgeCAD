//! The tolerance band's gaps (research/kernel-audit/coverage.md §3): inputs whose coincidences
//! the kernel reads inconsistently, one minimal case a band, each `#[ignore]`d with its site while
//! it fails, and the band's edges pinned by passing tests on both sides.
use forge_bool::Op;
use forge_coverage::tilt::{family, reading};
use forge_coverage::{Outcome, Pattern, Rel, band_boxes, block, exact_boxes, kernel_on};

fn pattern(x: &str, y: &str, z: &str, op: Op) -> Pattern {
    Pattern {
        rel: [x, y, z].map(|n| Rel::by_name(n).expect("a relation")),
        op,
    }
}

/// The kernel on the pattern moved into the band, and whether it is right: a valid body within
/// 1e-4 relative of either reading's volume (the coincident pattern or the moved boxes taken
/// exactly), Empty or ZeroThickness where a reading is that.
fn band_run(p: &Pattern, unit: f64, delta: f64, axes: &[usize]) -> (Outcome, bool) {
    let (a, b) = band_boxes(p, unit, delta, axes).expect("a coincident end on those axes");
    let got = kernel_on(block(a, 1), block(b, 2), p.op);
    let (ra, rb) = (p.boxes(unit), (a, b));
    let readings = [exact_boxes(ra.0, ra.1, p.op), exact_boxes(rb.0, rb.1, p.op)];
    let fine = readings.iter().any(|r| match (&got, r) {
        (Outcome::Ok(v), Outcome::Ok(w)) => (v - w).abs() <= 1e-4 * w.max(1.0),
        (g, r) => g == r,
    });
    (got, fine)
}

fn assert_band(p: &Pattern, unit: f64, delta: f64, axes: &[usize]) {
    let (got, fine) = band_run(p, unit, delta, axes);
    assert!(
        fine,
        "{} moved {delta} LINEAR on {axes:?}: kernel {}",
        p.label(),
        got.word()
    );
}

// ---- band 1: two or three axes moved, delta in (LINEAR/sqrt 2, LINEAR] ----
// Each plane still reads coincident (|d| <= LINEAR, forge-geom intersect.rs:853-855), but a corner
// moved on two axes is sqrt(2) delta > LINEAR from the edge it should be on. The decision log
// (branch claude/repo-review-mbdrlj-decisions) shows every failing run contradicting itself at
// classify.rs:588 (`point_in_face`'s nearest edge, decided both ways), and `on_edge`
// (boolean.rs:283) at share_cuts 4522 against place_cuts' end vertex (4584), none of which any
// passing run does; the retry ladder (ABSORB_HAIR_ROOTS, CHORDS, NO_RUN_ON) then rescues none.
#[test]
#[ignore = "coverage gap: band 1 (two axes, 0.85 LINEAR): SplitCrossed after on_edge at boolean.rs:4522 (share_cuts) disagrees with place_cuts' end vertex 4584 and classify.rs:588"]
fn band_two_axes_meets_equal_equal() {
    assert_band(
        &pattern("meets", "equal", "equal", Op::Union),
        10.0,
        0.85,
        &[0, 1],
    );
}

#[test]
fn band_two_axes_edges() {
    let p = pattern("meets", "equal", "equal", Op::Union);
    // inside LINEAR/sqrt 2 on each axis, and past band 2 too (1.05 is still in band 2 on x): right
    assert_band(&p, 10.0, 0.70, &[0, 1]);
    assert_band(&p, 10.0, 5.0, &[0, 1]);
    // today, inside the band: lost
    let (got, fine) = band_run(&p, 10.0, 0.85, &[0, 1]);
    assert!(
        !fine && matches!(got, Outcome::Internal(_)),
        "band 1 changed: {}",
        got.word()
    );
}

// ---- band 2 (closed): one axis moved, |delta| in (LINEAR, 4.8828125 LINEAR] at unit 10 ----
// B's face beside A's runs past A's end by delta: a sliver region delta wide on B's face. Coplanar
// classification (`classify_specs`) once probed it by stepping (`clear_step`, halving down to
// 2 LINEAR, then a 0.05 step across the sliver onto A's face: OnOpposite instead of Out). Now a
// region no step clears is given a point from its own boundary (`planar_inner`), and where that
// point is still within a tolerance of A's edge, the region, wider than a tolerance, is on the side
// of the imprinted edge it is bounded by (`side_of_partner`).
#[test]
fn band_one_axis_sliver_1_5() {
    assert_band(&pattern("equal", "meets", "equal", Op::Union), 10.0, 1.5, &[0]);
}

#[test]
fn band_one_axis_sliver_at_edge() {
    assert_band(&pattern("equal", "meets", "equal", Op::Union), 10.0, 4.8828, &[0]);
}

#[test]
fn band_one_axis_edges() {
    let p = pattern("equal", "meets", "equal", Op::Union);
    for delta in [0.9, 1.01, 1.5, 2.0, 3.0, 4.8828, 4.8829, -1.5, -4.8828, -4.8829] {
        assert_band(&p, 10.0, delta, &[0]);
    }
    // the old edge at unit 1 was 7.8125 LINEAR
    for delta in [1.01, 1.5, 3.0, 7.80, 7.82] {
        assert_band(&p, 1.0, delta, &[0]);
    }
}

/// Every pattern of the family, its coincident ends on one axis moved by deltas across the old
/// band (both ways, each axis, units 1 and 10): every run a reading's answer.
#[test]
fn band_one_axis_sweep() {
    let mut bad = Vec::new();
    for unit in [1.0, 10.0] {
        for axis in 0..3 {
            for delta in [
                1.05, 1.5, 1.95, 3.0, 4.8828, 7.8, -1.05, -1.5, -1.95, -3.0, -4.8828, -7.8,
            ] {
                for p in Pattern::all() {
                    if band_boxes(&p, unit, delta, &[axis]).is_none() {
                        continue;
                    }
                    let (got, fine) = band_run(&p, unit, delta, &[axis]);
                    if !fine {
                        bad.push(format!(
                            "{} unit {unit} axis {axis} delta {delta}: {}",
                            p.label(),
                            got.word()
                        ));
                    }
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} runs neither reading:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

// ---- band 3 (closed): a tilted face flush at its ends, 5 theta in (LINEAR, 2 * 4.8828125 LINEAR] ----
// `cap_over`: B covers A's top, turned by theta about the top's middle line, B's sides flush with
// A's. nearly_flush (ssi.rs) reads the top pair Coincident while every region corner is within
// LINEAR (5 theta <= LINEAR); past that the planes cross, and the strip between the crossing line
// and B's ends is a sliver the stepping probe could not enter: decided as band 2 now is.
fn tilt_run(fam: &str, th: f64, op: Op) -> (Outcome, bool) {
    let ((ab, ap), (bb, bp)) = family(fam, th);
    let ((_, a0), (_, b0)) = family(fam, 0.0);
    let got = kernel_on(ab, bb, op);
    let vols = [reading(&ap, &bp, op), reading(&a0, &b0, op)];
    let fine = match &got {
        Outcome::Ok(v) => vols.iter().any(|w| (v - w).abs() <= 1e-6 * w.abs().max(1.0)),
        Outcome::Empty => vols.iter().any(|w| w.abs() <= 1e-6),
        _ => false,
    };
    (got, fine)
}

#[test]
fn tilt_cap_over_5e_6() {
    for op in [Op::Union, Op::Subtract, Op::Intersect] {
        let (got, fine) = tilt_run("cap_over", 5e-6, op);
        assert!(fine, "{op:?}: {}", got.word());
    }
}

// Under 5 theta = 2 LINEAR (theta up to about 4e-6) B's end corners are within two tolerances of
// A's top edges, and the intersection graph is already inconsistent before classification: a new
// edge on A's side face ends on A's corner edge with no continuation on the next face (cap_over at
// 2.4e-6: InconsistentClassification across that corner edge; cap and wall_flush: SplitCrossed),
// the corner-within-tolerance site of band 1, not the probe.
#[test]
#[ignore = "coverage gap: band 1's corner site in the tilt (5 theta in (LINEAR, 2 LINEAR]): new edge ends on A's corner edge without continuing (SplitCrossed / InconsistentClassification)"]
fn tilt_cap_over_corner_band() {
    for op in [Op::Union, Op::Subtract, Op::Intersect] {
        let (got, fine) = tilt_run("cap_over", 2.4e-6, op);
        assert!(fine, "{op:?}: {}", got.word());
    }
}

#[test]
fn tilt_band_edges() {
    for op in [Op::Union, Op::Subtract, Op::Intersect] {
        for th in [
            1.5e-6, 4e-6, 5e-6, 8e-6, 1e-5, 1.5e-5, 1.95e-5, 1.96e-5, 1e-4, 1e-3,
        ] {
            let (got, fine) = tilt_run("cap_over", th, op);
            assert!(fine, "cap_over {op:?} at {th:e}: {}", got.word());
        }
    }
}

/// Faces turned against a face they share no boundary with pass at every angle from 1e-13 to 0.5.
#[test]
fn tilt_without_shared_boundary_is_right() {
    for fam in ["lid", "plug", "wall", "twist", "ridge"] {
        for op in [Op::Union, Op::Subtract, Op::Intersect] {
            for th in [0.0, 1e-12, 1e-10, 1e-8, 1e-6, 5e-6, 1e-4, 1e-3, 1e-2, 0.1] {
                let (got, fine) = tilt_run(fam, th, op);
                assert!(fine, "{fam} {op:?} at {th:e}: {}", got.word());
            }
        }
    }
}

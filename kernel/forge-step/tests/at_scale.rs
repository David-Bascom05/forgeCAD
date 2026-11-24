//! Forms of real files read at scale (research/step/scale.md): an edge on a closed B-spline (clamped, closed in
//! fact, not periodic) that starts or ends at the curve's closing point takes the curve's first parameter at its start
//! and its last at its end, and a near-flat cone is its plane.
//! Two edges on one closed spline, A to C and C to A, make a whole rim, as AutoMate's parts write a cross hole's.
//! The files are research/step/repro/*.step, written by mk.py there.
use forge_step::{Limits, import};
use forge_topo::{check, mass};

fn reads_whole(text: &str) {
    let imp = import(text.as_bytes(), &Limits::default()).expect("reads");
    assert!(
        imp.refusals.is_empty(),
        "refused: {:?}",
        imp.refusals.first().map(|r| &r.why)
    );
    assert_eq!(imp.solids.len(), 1);
    let b = &imp.solids[0].body;
    assert!(check(b).is_empty(), "{:?}", check(b));
    let m = mass::mass(b).expect("mass");
    // a cylinder of radius 10, height 20
    let v = std::f64::consts::PI * 100.0 * 20.0;
    assert!((m.volume - v).abs() < 1e-6 * v, "{} against {v}", m.volume);
}

#[test]
fn a_rim_of_two_edges_on_one_closed_spline_reads_whole() {
    reads_whole(include_str!("data/split_rim_bspline_t.step"));
}

#[test]
fn the_same_with_both_edges_against_the_spline() {
    reads_whole(include_str!("data/split_rim_bspline_f.step"));
}

/// A cone of half angle a hair under a right angle (ABC 00012071 writes flat rings so) reads as the plane square
/// to its axis: a tube 4..10 by 10 whose top ring is such a cone (research/step/repro/near_flat_cone.step).
#[test]
fn a_near_flat_cone_reads_as_its_plane() {
    let imp = import(
        include_str!("data/near_flat_cone.step").as_bytes(),
        &Limits::default(),
    )
    .expect("reads");
    assert!(
        imp.refusals.is_empty(),
        "refused: {:?}",
        imp.refusals.first().map(|r| &r.why)
    );
    let b = &imp.solids[0].body;
    assert!(check(b).is_empty(), "{:?}", check(b));
    let v = std::f64::consts::PI * (100.0 - 16.0) * 10.0;
    let m = mass::mass(b).expect("mass");
    // the plane stands at the cone's circle, 6.4e-7 rad from where the ring lies: a few 1e-7 of the volume
    assert!((m.volume - v).abs() < 1e-6 * v, "{} against {v}", m.volume);
}

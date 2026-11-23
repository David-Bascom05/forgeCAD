//! A torus whose tube reaches its axis (a horn torus, R = r, or a spindle, R < r), as OpenCascade
//! writes one: a toroidal surface of those radii, one face bounded only by its seam to the poles
//! on the axis (and back). Read as the part of the tube off the axis, the solid it bounds measured
//! to its closed form. Files made from nothing by research/formats/repro/make_brep.py (OCCT).
use forge_step::{Limits, import};
use std::f64::consts::PI;

fn read(name: &str) -> (forge_topo::Body, f64, f64) {
    let path = format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    let got = import(&std::fs::read(path).unwrap(), &Limits::default()).expect("reads");
    assert!(got.refusals.is_empty(), "{:?}", got.refusals);
    assert_eq!(got.solids.len(), 1);
    let s = got.solids.into_iter().next().unwrap();
    assert!(s.body.unverified.is_none(), "{:?}", s.body.unverified);
    let m = forge_topo::mass::mass_rel(&s.body, 1e-9).unwrap();
    (s.body, m.volume.abs(), m.area)
}

/// The solid swept by the part of a disc of radius r, centred R from the axis, off the axis.
fn apple(big: f64, r: f64) -> f64 {
    let h = (r * r - big * big).max(0.0).sqrt();
    let across = 2.0 * h * (r * r + big * big)
        - 2.0 * h.powi(3) / 3.0
        - 2.0 * big * (h * big + r * r * forge_math::dmath::asin(h / r));
    2.0 * PI * (PI * big * r * r + across / 2.0)
}

#[test]
fn a_spindle_tori_outer_part_reads_whole() {
    let (b, v, _) = read("spindle-torus-surface.brep");
    assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
    let want = apple(4.0, 5.0);
    assert!((v - want).abs() < 1e-9 * want, "{v} {want}");
}

#[test]
fn a_horn_torus_reads_with_its_volume() {
    // the solid is pinched at its centre, where the tube's two sides meet the axis at one point;
    // the kernel's check, which has no such vertex, says so (an edge with both ends there)
    let (_, v, a) = read("horn-torus-surface.brep");
    let want = 2.0 * PI * PI * 10.0 * 100.0;
    assert!((v - want).abs() < 1e-9 * want, "{v} {want}");
    assert!((a - 4.0 * PI * PI * 10.0 * 10.0).abs() < 1e-9 * a, "{a}");
}

#[test]
fn a_whole_sphere_bounded_by_its_seam_reads() {
    let (b, v, a) = read("sphere.brep");
    assert!(forge_topo::check(&b).is_empty(), "{:?}", forge_topo::check(&b));
    assert!((v - 4.0 / 3.0 * PI * 125.0).abs() < 1e-9 * v && (a - 4.0 * PI * 25.0).abs() < 1e-9 * a);
}

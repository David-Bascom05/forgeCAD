//! A spline wall reaching the axis, exported to STEP: at its pole the surface leaves u free, the
//! loop drew the pole a whole turn from its neighbour, and the pcurve's last span swept round the
//! axis, so the file stated 12.39 mm of accuracy at a tol of 1e-4 (research/spline-loops/
//! step-pole-fit.md on the fix-shell branch). The file must state its tol's order and read back to
//! the exact volume (a 30-digit integration of the section about the axis) within its area times it.

mod common;

const DOME: &str = "param cup.h = 30
param cup.r = 30
param cup.m = (-24, 20)
param cup.k = (1, 0)
sketch cup(on: xz) {
  l0 = line((0, 0), (-r, 0))
  s1 = spline([(-r, 0), m, (0, h)], closed: false, end_slope: k)
  l2 = line((0, h), (0, 0))
  ax = line((0, h), (0, 0), construction: true)
}
body = revolve(cup, axis: cup.ax)
";

#[test]
fn a_spline_dome_exports_within_its_stated_accuracy() {
    let exact = 59629.338687932;
    let (r, cache) = common::run(DOME);
    assert!(r.errors.is_empty(), "{}", forge_lang::report::text(&r, DOME));
    let (f, ids) = cache.history();
    let solids = forge_model::Part { features: f.to_vec() }
        .regenerate_solids_ids(Some(ids))
        .expect("builds");
    let body = &solids.last().expect("a solid").body;
    let opts = forge_step::Options {
        tol: 1e-4,
        ..forge_step::Options::default()
    };
    let e = forge_step::export(body, &opts).expect("writes");
    assert!(
        e.uncertainty <= 10.0 * opts.tol,
        "the file states {} mm",
        e.uncertainty
    );
    let back = forge_step::import(e.text.as_bytes(), &forge_step::Limits::default()).expect("reads");
    assert!(back.refusals.is_empty(), "{:?}", back.refusals);
    let m = forge_topo::mass::mass_rel(&back.solids[0].body, 1e-9).expect("a mass");
    assert!(
        (m.volume - exact).abs() <= m.area * opts.tol,
        "volume {} against {exact}: off {:.3e}",
        m.volume,
        m.volume - exact
    );
}

//! shadow.rs's torus bound (a torus about a vertical axis, an arc about that axis): every Clear
//! it gives is at most the signed distance at dense samples of the face and the move; it is tried
//! only where its preconditions hold (a vertical axis, an arc about the axis); and where it
//! applies it decides in a few cells what the branch and bound takes thousands for.
use forge_cam_geom::check::{Face, Move, Planar, Surf, Verdict, certify_bb};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::shadow;
use forge_cam_geom::tool::{Body, Cutter, assemble};

struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn r(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }
}

/// The rational map's point: (cos, sin) of 2 atan(t) (no std trigonometry in the workspace).
fn circ(t: f64) -> (f64, f64) {
    let d = 1.0 + t * t;
    ((1.0 - t * t) / d, 2.0 * t / d)
}

/// A fillet's torus about the vertical axis through (ox, oy): tube centre radius `big`, tube
/// radius r, the tube angle about φm, the whole turn of the azimuth (p over the rational map's
/// [−1, 1], a half turn, and the face's q over `qr`).
#[allow(clippy::too_many_arguments)]
fn torus(ox: f64, oy: f64, oz: f64, big: f64, r: f64, tm: f64, qr: Ival, up: bool) -> Face {
    let (cm, sm) = circ(tm);
    let surf = Surf::Torus {
        o: [ox, oy, oz],
        x: [1.0, 0.0, 0.0],
        y: [0.0, 1.0, 0.0],
        ax: [0.0, 0.0, if up { 1.0 } else { -1.0 }],
        big,
        r,
        cm,
        sm,
    };
    Face::new(surf, Ival::new(-1.0, 1.0), qr, None, 0.0, 0)
}

/// An arc about c, radius r, at height z, from the rational map's t0 to t1 (under a half turn).
fn arc(c: [f64; 2], r: f64, z: f64, t0: f64, t1: f64) -> Move {
    Move::Arc {
        c: [c[0], c[1], z],
        x: [1.0, 0.0, 0.0],
        y: [0.0, 1.0, 0.0],
        r,
        t0,
        t1,
    }
}

/// The least sampled signed distance of the moved body to the face.
fn sampled(body: &Body, f: &Face, mv: &Move) -> f64 {
    let mut least = f64::INFINITY;
    for i in 0..=32 {
        let p = f.p.lo + (f.p.hi - f.p.lo) * i as f64 / 32.0;
        for j in 0..=16 {
            let q = f.q.lo + (f.q.hi - f.q.lo) * j as f64 / 16.0;
            let y = f.surf.eval(Ival::pt(p), Ival::pt(q));
            let y = [y[0].mid(), y[1].mid(), y[2].mid()];
            for k in 0..=12 {
                let a = mv.at(Ival::pt(k as f64 / 12.0));
                let a = [a[0].mid(), a[1].mid(), a[2].mid()];
                let rho = ((y[0] - a[0]).powi(2) + (y[1] - a[1]).powi(2)).sqrt();
                let sd = body.sd_rz(Ival::pt(rho), Ival::pt(y[2] - a[2])).mid();
                least = least.min(sd);
            }
        }
    }
    least
}

fn tools() -> Vec<Body> {
    [
        assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap(),
        assemble(&Cutter::Ball { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap(),
        assemble(&Cutter::Bull { d: 8.0, re: 1.0 }, 15.0, 8.0, 30.0, None).unwrap(),
    ]
    .into_iter()
    .map(|t| t.bodies[0].clone())
    .collect()
}

/// Every Clear is a lower bound on the sampled least signed distance: arcs about the axis (within
/// the offsets' rounding of it), inside and outside the tube, above, beside and below it, at
/// limits around the least, so that both outcomes come up.
#[test]
fn a_torus_clear_is_below_every_sample() {
    let mut g = Rng(11);
    let bodies = tools();
    let (mut clear, mut none) = (0, 0);
    for case in 0..120 {
        let body = &bodies[case % bodies.len()];
        let (ox, oy) = (g.r(-20.0, 20.0), g.r(-20.0, 20.0));
        let big = g.r(4.0, 12.0);
        let r = g.r(0.5, 3.0);
        let f = torus(
            ox,
            oy,
            g.r(-5.0, 5.0),
            big,
            r,
            g.r(-1.0, 1.0),
            Ival::new(-0.4, 0.4),
            case % 5 != 0,
        );
        let off = 4e-3;
        let c = [ox + g.r(-off, off), oy + g.r(-off, off)];
        let mv = arc(c, g.r(0.5, 16.0), g.r(-8.0, 8.0), -g.r(0.05, 0.5), g.r(0.05, 0.5));
        let pl = mv.planar().expect("a planar arc");
        let least = sampled(body, &f, &mv);
        for lim in [least - 0.5, least - 0.01, least + 0.01] {
            let (v, _) = shadow::clear(body, &f, &mv, &pl, lim, 1e-3, 20_000);
            match v {
                Some(lower) => {
                    clear += 1;
                    assert!(lower >= lim, "case {case}: a Clear under its limit");
                    assert!(
                        lower <= least + 1e-9,
                        "case {case}: Clear {lower} over the sampled least {least}"
                    );
                }
                None => none += 1,
            }
        }
    }
    assert!(clear > 100 && none > 25, "clear {clear}, none {none}");
}

/// The bound is tried only where its preconditions hold: a torus whose axis is not vertical,
/// an arc whose centre is off the axis, and a line move all go to the branch and bound.
#[test]
fn a_torus_bound_is_tried_only_about_a_vertical_axis() {
    let body = &tools()[0];
    let f = torus(1.0, 2.0, 0.0, 8.0, 2.0, 0.3, Ival::new(-0.4, 0.4), true);
    // about the axis: decided by symmetry
    let mv = arc([1.0, 2.0], 14.0, -1.0, -0.4, 0.4);
    let (v, n) = shadow::clear(body, &f, &mv, &mv.planar().unwrap(), -0.01, 1e-3, 20_000);
    assert!(v.is_some() && n > 0, "{v:?} {n}");
    // an arc about another centre (0.02 off): not tried
    let mv = arc([1.02, 2.0], 14.0, -1.0, -0.4, 0.4);
    assert_eq!(
        shadow::clear(body, &f, &mv, &mv.planar().unwrap(), -0.01, 1e-3, 20_000),
        (None, 0)
    );
    // a line: not tried
    let mv = Move::Line {
        a: [20.0, 2.0, -1.0],
        b: [20.0, 6.0, -1.0],
    };
    assert_eq!(
        shadow::clear(body, &f, &mv, &mv.planar().unwrap(), -0.01, 1e-3, 20_000),
        (None, 0)
    );
    // a tilted axis: not tried
    let mut t = f.clone();
    if let Surf::Torus { ref mut ax, .. } = t.surf {
        *ax = [0.0, 0.6, 0.8];
    }
    let mv = arc([1.0, 2.0], 14.0, -1.0, -0.4, 0.4);
    assert_eq!(
        shadow::clear(body, &t, &mv, &mv.planar().unwrap(), -0.01, 1e-3, 20_000),
        (None, 0)
    );
}

/// Where it applies, a few cells: a pocket's arcs about a filleted bore, the cutter's side
/// tangent to the fillet's foot within a hundredth (manifold's `shal`, where the branch and
/// bound took thousands of cells per arc), cleared at −tol; and a gouge is not cleared.
#[test]
fn a_pocket_arc_about_a_fillet_is_cleared_in_few_cells() {
    let body = &tools()[0];
    // the fillet round a bore's mouth: tube centre radius 12, tube radius 2, its quarter between
    // the bore's wall and the top (tube angle from 180° to 270°, about 225°)
    let f = torus(82.0, 38.0, 29.0, 12.0, 2.0, -2.414, Ival::new(-0.41, 0.41), true);
    let Planar::Arc { .. } = arc([81.9997, 37.9997], 6.25, 23.0, -0.41, 0.41).planar().unwrap() else {
        panic!("a planar arc");
    };
    let mv = arc([81.9997, 37.9997], 6.25, 23.0, -0.41, 0.41);
    let pl = mv.planar().unwrap();
    let (v, n) = shadow::clear(body, &f, &mv, &pl, -0.01, 0.01 / 4.0, 200_000);
    assert!(v.is_some() && n < 500, "{v:?} in {n} cells");
    // the same arc pushed into the fillet's foot: the branch and bound finds the gouge, and the
    // bound never claims a clearance
    let mv = arc([81.9997, 37.9997], 7.5, 27.5, -0.41, 0.41);
    let pl = mv.planar().unwrap();
    let (vb, _) = certify_bb(body, &f, &mv, -0.01, 0.01 / 4.0, 200_000);
    if let Verdict::Hit { .. } = vb {
        assert_eq!(
            shadow::clear(body, &f, &mv, &pl, -0.01, 0.01 / 4.0, 200_000).0,
            None
        );
    } else {
        panic!("expected a gouge: {vb:?}");
    }
}

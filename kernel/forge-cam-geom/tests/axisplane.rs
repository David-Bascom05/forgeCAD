//! The exact closed form for an axis-aligned plane face (axisplane.rs): a tool exactly at its
//! limit from a workholding box's face (the top of a jaw at the fixture clearance; a plate it
//! rests on at distance 0) is proven clear, where the branch and bound alone leaves it undecided;
//! a hair closer is never proven clear; and against the branch and bound on random boxes and
//! moves it never proves Clear where that proves a Hit.
use forge_cam_geom::check::{Face, Move, Surf, Verdict, certify, certify_bb};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::tool::{Cutter, assemble};

const RES: f64 = 0.0025;
const BUDGET: usize = 100_000;

/// The top of a box (a horizontal face at z = h over [x0, x1] × [y0, y1]), as `Stock::faces` makes it.
fn top(x0: f64, x1: f64, y0: f64, y1: f64, h: f64, mu: f64) -> Face {
    Face::new(
        Surf::Plane {
            o: [0.0, 0.0, h],
            u: [1.0, 0.0, 0.0],
            v: [0.0, 1.0, 0.0],
        },
        Ival::new(x0, x1),
        Ival::new(y0, y1),
        None,
        mu,
        1,
    )
}

/// A side of a box at x = c facing −x, over [y0, y1] × [z0, z1].
fn side_x(c: f64, y0: f64, y1: f64, z0: f64, z1: f64, mu: f64) -> Face {
    Face::new(
        Surf::Plane {
            o: [c, 0.0, 0.0],
            u: [0.0, -1.0, 0.0],
            v: [0.0, 0.0, 1.0],
        },
        Ival::new(-y1, -y0),
        Ival::new(z0, z1),
        None,
        mu,
        2,
    )
}

fn clear(v: &Verdict) -> bool {
    matches!(v, Verdict::Clear { .. })
}

fn hit(v: &Verdict) -> bool {
    matches!(v, Verdict::Hit { .. })
}

#[test]
fn a_pass_exactly_at_the_clearance_over_a_jaw_is_clear() {
    let t = assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap();
    let body = &t.bodies[0];
    // the jaw's top at z = 10; the tip 1 above it, the limit 1: a tie
    let f = top(-20.0, 20.0, -5.0, 5.0, 10.0, 0.0);
    let mv = Move::Line {
        a: [-30.0, 0.0, 11.0],
        b: [30.0, 0.0, 11.0],
    };
    let (v, _) = certify(body, &f, &mv, 1.0, RES, BUDGET);
    assert!(clear(&v), "a tie at the limit: {v:?}");
    if let Verdict::Clear { lower } = v {
        assert!(lower >= 1.0, "{lower}");
    }
    // the search alone cannot prove it (its margin): the case this closed form is for
    let (b, _) = certify_bb(body, &f, &mv, 1.0, RES, BUDGET);
    assert!(!clear(&b), "the branch and bound proved a tie: {b:?}");
    // a hair lower is not proven clear
    let lower = Move::Line {
        a: [-30.0, 0.0, 10.999999],
        b: [30.0, 0.0, 10.999999],
    };
    let (v, _) = certify(body, &f, &lower, 1.0, RES, BUDGET);
    assert!(!clear(&v), "under the limit, proven clear: {v:?}");
    // nor the tie when the face carries a tolerance (its true place may be that much nearer)
    let fm = top(-20.0, 20.0, -5.0, 5.0, 10.0, 1e-9);
    let (v, _) = certify(body, &fm, &mv, 1.0, RES, BUDGET);
    assert!(!clear(&v), "a tie with mu > 0, proven clear: {v:?}");
}

#[test]
fn a_tool_resting_on_a_plate_at_distance_zero_is_clear_at_limit_zero() {
    let t = assemble(&Cutter::Bull { d: 8.0, re: 1.5 }, 15.0, 8.0, 30.0, None).unwrap();
    let body = &t.bodies[0];
    let f = top(-50.0, 50.0, -50.0, 50.0, 0.0, 0.0);
    // a cut along the plate's top, and an arc at that height
    let line = Move::Line {
        a: [-10.0, 3.0, 0.0],
        b: [12.0, -4.0, 0.0],
    };
    let (v, _) = certify(body, &f, &line, 0.0, RES, BUDGET);
    assert!(clear(&v), "{v:?}");
    let arc = Move::Arc {
        c: [0.0, 0.0, 0.0],
        x: [1.0, 0.0, 0.0],
        y: [0.0, 1.0, 0.0],
        r: 10.0,
        t0: 0.0,
        t1: 3.0,
    };
    let (v, _) = certify(body, &f, &arc, 0.0, RES, BUDGET);
    assert!(clear(&v), "{v:?}");
    // into it, never clear
    let into = Move::Line {
        a: [-10.0, 3.0, 0.0],
        b: [12.0, -4.0, -0.0001],
    };
    let (v, _) = certify(body, &f, &into, 0.0, RES, BUDGET);
    assert!(!clear(&v), "{v:?}");
}

#[test]
fn a_pass_beside_a_jaw_exactly_at_the_clearance_is_clear() {
    let t = assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap();
    let body = &t.bodies[0];
    let r = body.rmax;
    // the jaw's face at x = 0 (facing −x); the axis rmax + 1 from it, the limit 1
    let f = side_x(0.0, -40.0, 40.0, -10.0, 10.0, 0.0);
    let x = -(r + 1.0);
    // only where that sum is exact is the tie a tie
    assert_eq!(x + r, -1.0);
    let mv = Move::Line {
        a: [x, -20.0, 0.0],
        b: [x, 20.0, 0.0],
    };
    let (v, _) = certify(body, &f, &mv, 1.0, RES, BUDGET);
    assert!(clear(&v), "{v:?}");
    let nearer = Move::Line {
        a: [x, -20.0, 0.0],
        b: [x + 1e-6, 20.0, 0.0],
    };
    let (v, _) = certify(body, &f, &nearer, 1.0, RES, BUDGET);
    assert!(!clear(&v), "{v:?}");
}

struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    /// On a grid of 1/8 (exact in binary), so ties happen.
    fn g(&mut self, a: f64, b: f64) -> f64 {
        ((a + (b - a) * self.f()) * 8.0).round() / 8.0
    }
}

#[test]
fn never_clear_where_the_branch_and_bound_hits() {
    let mut g = Rng(7);
    let tools = [
        assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap(),
        assemble(&Cutter::Ball { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap(),
        assemble(&Cutter::Bull { d: 8.0, re: 1.5 }, 15.0, 8.0, 30.0, None).unwrap(),
    ];
    let (mut decided, mut hits) = (0, 0);
    for case in 0..600 {
        let t = &tools[case % 3];
        let body = &t.bodies[g.0 as usize % t.bodies.len()];
        let f = if case % 2 == 0 {
            let (x0, y0) = (g.g(-10.0, 0.0), g.g(-10.0, 0.0));
            top(
                x0,
                x0 + g.g(1.0, 20.0),
                y0,
                y0 + g.g(1.0, 20.0),
                g.g(-5.0, 5.0),
                0.0,
            )
        } else {
            let z0 = g.g(-10.0, 0.0);
            side_x(g.g(-5.0, 5.0), -10.0, 10.0, z0, z0 + g.g(1.0, 15.0), 0.0)
        };
        let a = [g.g(-15.0, 15.0), g.g(-15.0, 15.0), g.g(-8.0, 12.0)];
        let mv = if case % 4 == 3 {
            Move::Arc {
                c: a,
                x: [1.0, 0.0, 0.0],
                y: [0.0, 1.0, 0.0],
                r: g.g(1.0, 8.0),
                t0: 0.0,
                t1: g.g(0.5, 6.0),
            }
        } else {
            Move::Line {
                a,
                b: [
                    a[0] + g.g(-10.0, 10.0),
                    a[1] + g.g(-10.0, 10.0),
                    a[2] + g.g(-6.0, 6.0),
                ],
            }
        };
        let lim = [0.0, 0.5, 1.0][case % 3];
        let (v, _) = certify(body, &f, &mv, lim, RES, BUDGET);
        let (b, _) = certify_bb(body, &f, &mv, lim, RES, BUDGET);
        assert!(
            !(clear(&v) && hit(&b)) && !(hit(&v) && clear(&b)),
            "case {case}: {v:?} vs the branch and bound's {b:?}"
        );
        if clear(&v) || hit(&v) {
            decided += 1;
        }
        if hit(&b) {
            hits += 1;
        }
    }
    assert!(decided > 300 && hits > 50, "decided {decided}, hits {hits}");
}

#[test]
fn a_negative_limit_or_mu_is_never_cleared_here() {
    let t = assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap();
    let body = &t.bodies[0];
    let mv = Move::Line {
        a: [-30.0, 0.0, 11.0],
        b: [30.0, 0.0, 11.0],
    };
    // well clear (1 above the face), but the closed form takes only limit ≥ 0 and mu ≥ 0
    let f = top(-20.0, 20.0, -5.0, 5.0, 10.0, 0.0);
    assert!(forge_cam_geom::axisplane::clear(body, &f, &mv, -0.5).is_none());
    let fneg = top(-20.0, 20.0, -5.0, 5.0, 10.0, -0.25);
    assert!(forge_cam_geom::axisplane::clear(body, &fneg, &mv, 0.5).is_none());
    // the same with limit 0.5 and mu 0: proven
    assert!(forge_cam_geom::axisplane::clear(body, &f, &mv, 0.5).is_some());
}

#[test]
fn a_bodys_radius_includes_an_arcs_bulge() {
    use forge_cam_geom::tool::{Body, PSeg, Part};
    // a lollipop-like profile: up the axis to (2, 0), an arc bulging out to ρ = 1 + √2 about
    // (1, 1) up to (2, 2), then back to the axis
    let r = 2f64.sqrt();
    let chain = vec![
        PSeg::Line {
            a: [0.0, 0.0],
            b: [2.0, 0.0],
        },
        PSeg::Arc {
            a: [2.0, 0.0],
            b: [2.0, 2.0],
            c: [1.0, 1.0],
            r,
            ccw: true,
        },
        PSeg::Line {
            a: [2.0, 2.0],
            b: [0.0, 2.0],
        },
    ];
    let b = Body::new(chain, Part::NonCutting).unwrap();
    assert!(b.rmax >= 1.0 + r, "{}", b.rmax);
    // and the cutters' own: their arcs end at their widest, so rmax is their radius
    for (c, want) in [
        (Cutter::Flat { d: 6.0 }, 3.0),
        (Cutter::Ball { d: 6.0 }, 3.0),
        (Cutter::Bull { d: 8.0, re: 1.5 }, 4.0),
    ] {
        let t = assemble(&c, 15.0, 6.0, 30.0, None).unwrap();
        assert_eq!(t.bodies[0].rmax, want, "{c:?}");
    }
}

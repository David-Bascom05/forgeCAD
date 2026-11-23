//! L6 for the 2.5D closed form: against the general branch and bound (an independent method)
//! on random prism faces and planar moves. The two may differ in Undecided, never in a proven
//! verdict: a Clear from one and a Hit from the other is a bug in one of them.
use forge_cam_geom::check::{Face, Move, Surf, TSeg, Trim, Verdict, certify, certify_bb};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::tool::{Cutter, assemble};

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

fn proven(v: &Verdict) -> Option<bool> {
    match v {
        Verdict::Clear { .. } => Some(true),
        Verdict::Hit { .. } => Some(false),
        Verdict::Undecided { .. } => None,
    }
}

#[test]
fn closed_form_never_contradicts_the_branch_and_bound() {
    let mut g = Rng(42);
    let tools = [
        assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap(),
        assemble(&Cutter::Ball { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap(),
        assemble(&Cutter::Bull { d: 8.0, re: 1.5 }, 15.0, 8.0, 30.0, None).unwrap(),
    ];
    let (mut agree, mut decided, mut hits) = (0, 0, 0);
    for case in 0..400 {
        let t = &tools[case % 3];
        let body = &t.bodies[0];
        // a face: a wall along x at y = 0, a cylinder about z, or a floor rectangle
        let f = match case % 3 {
            0 => Face::new(
                Surf::Plane {
                    o: [0.0, 0.0, 0.0],
                    u: [1.0, 0.0, 0.0],
                    v: [0.0, 0.0, 1.0],
                },
                Ival::new(-10.0, 10.0),
                Ival::new(-5.0, 5.0),
                None,
                0.0,
                0,
            ),
            1 => Face::new(
                Surf::Cyl {
                    o: [0.0, 0.0, -5.0],
                    x: [1.0, 0.0, 0.0],
                    y: [0.0, 1.0, 0.0],
                    ax: [0.0, 0.0, 1.0],
                    r: 5.0,
                },
                Ival::new(-0.8, 0.8),
                Ival::new(0.0, 10.0),
                None,
                0.0,
                1,
            ),
            _ => {
                let segs = vec![
                    TSeg::Line {
                        a: [-8.0, -6.0],
                        b: [8.0, -6.0],
                    },
                    TSeg::Line {
                        a: [8.0, -6.0],
                        b: [8.0, 6.0],
                    },
                    TSeg::Line {
                        a: [8.0, 6.0],
                        b: [-8.0, 6.0],
                    },
                    TSeg::Line {
                        a: [-8.0, 6.0],
                        b: [-8.0, -6.0],
                    },
                ];
                Face::new(
                    Surf::Plane {
                        o: [0.0, 0.0, -2.0],
                        u: [1.0, 0.0, 0.0],
                        v: [0.0, 1.0, 0.0],
                    },
                    Ival::new(-8.0, 8.0),
                    Ival::new(-6.0, 6.0),
                    Some(Trim { segs }),
                    0.0,
                    2,
                )
            }
        };
        assert!(f.prism.is_some(), "case {case}");
        // a move passing near it at a random height
        let z = g.r(-6.0, 4.0);
        let mv = if case % 5 == 4 {
            let (x, y) = (g.r(-12.0, 12.0), g.r(-9.0, 9.0));
            Move::Line {
                a: [x, y, z + 4.0],
                b: [x, y, z - 3.0],
            }
        } else if case % 2 == 0 {
            let a = [g.r(-12.0, 12.0), g.r(-9.0, 9.0), z];
            let b = [a[0] + g.r(-6.0, 6.0), a[1] + g.r(-6.0, 6.0), z];
            Move::Line { a, b }
        } else {
            let th = g.r(0.0, std::f64::consts::TAU);
            let (c0, s0) = (forge_math_free_cos(th), forge_math_free_sin(th));
            Move::Arc {
                c: [g.r(-6.0, 6.0), g.r(-6.0, 6.0), z],
                x: [c0, s0, 0.0],
                y: [-s0, c0, 0.0],
                r: g.r(1.0, 9.0),
                t0: -g.r(0.05, 0.9),
                t1: g.r(0.05, 0.9),
            }
        };
        let lim = -0.01;
        let (a, _) = certify(body, &f, &mv, lim, 0.0025, 100_000);
        let (b, _) = certify_bb(body, &f, &mv, lim, 0.0025, 100_000);
        if let (Some(x), Some(y)) = (proven(&a), proven(&b)) {
            decided += 1;
            assert_eq!(
                x, y,
                "case {case}: closed form {a:?}, branch and bound {b:?}, move {mv:?}"
            );
            agree += 1;
            hits += usize::from(!x);
        }
        // the closed form decides everything here
        assert!(proven(&a).is_some(), "case {case}: {a:?}");
    }
    assert!(decided > 300, "{decided}");
    assert_eq!(agree, decided);
    // both verdicts are exercised
    assert!(hits > 50 && decided - hits > 50, "{hits} hits of {decided}");
}

/// Ramps (lines moving in xy and z) near prism faces: the closed form takes the xy segment swept
/// over the move's heights, a superset, and its Clear must never meet a Hit of the branch and
/// bound on the move itself. `certify` keeps only that Clear, else decides as before.
#[test]
fn ramps_closed_form_never_contradicts_the_branch_and_bound() {
    use forge_cam_geom::slab::{Sec, certify_path};
    let mut g = Rng(7);
    let tools = [
        assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap(),
        assemble(&Cutter::Ball { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap(),
        assemble(&Cutter::Bull { d: 8.0, re: 1.5 }, 15.0, 8.0, 30.0, None).unwrap(),
    ];
    let (mut clear, mut hits, mut both) = (0, 0, 0);
    for case in 0..300 {
        let body = &tools[case % 3].bodies[0];
        let f = if case % 2 == 0 {
            Face::new(
                Surf::Plane {
                    o: [0.0, 0.0, 0.0],
                    u: [1.0, 0.0, 0.0],
                    v: [0.0, 0.0, 1.0],
                },
                Ival::new(-10.0, 10.0),
                Ival::new(-5.0, 5.0),
                None,
                0.0,
                0,
            )
        } else {
            Face::new(
                Surf::Cyl {
                    o: [0.0, 0.0, -5.0],
                    x: [1.0, 0.0, 0.0],
                    y: [0.0, 1.0, 0.0],
                    ax: [0.0, 0.0, 1.0],
                    r: 5.0,
                },
                Ival::new(-0.8, 0.8),
                Ival::new(0.0, 10.0),
                None,
                0.0,
                1,
            )
        };
        let pr = f.prism.clone().expect("a prism face");
        let a = [g.r(-12.0, 12.0), g.r(-9.0, 9.0), g.r(-6.0, 6.0)];
        let b = [
            a[0] + g.r(-8.0, 8.0),
            a[1] + g.r(-8.0, 8.0),
            a[2] + g.r(-4.0, 4.0),
        ];
        let mv = Move::Line { a, b };
        let lim = -0.01;
        let sec = Sec::Seg {
            a: [a[0], a[1]],
            b: [b[0], b[1]],
        };
        let zt = Ival::new(a[2].min(b[2]), a[2].max(b[2]));
        let (sup, _) = certify_path(body, &f, &pr, &sec, zt, lim, 0.0025, 100_000, &mut None, 0.0);
        let (bb, _) = certify_bb(body, &f, &mv, lim, 0.0025, 100_000);
        let (cv, _) = certify(body, &f, &mv, lim, 0.0025, 100_000);
        if matches!(sup, Verdict::Clear { .. }) {
            clear += 1;
            assert!(
                !matches!(bb, Verdict::Hit { .. }),
                "case {case}: superset clear, branch and bound hit {bb:?}, move {mv:?}"
            );
        }
        if matches!(bb, Verdict::Hit { .. }) {
            hits += 1;
        }
        // certify: the superset's Clear, or the branch and bound's own verdict
        match (&sup, &cv) {
            (Verdict::Clear { .. }, Verdict::Clear { .. }) => both += 1,
            (Verdict::Clear { .. }, _) => panic!("case {case}: {cv:?}"),
            _ => assert_eq!(proven(&cv), proven(&bb), "case {case}"),
        }
    }
    // both kinds are exercised
    assert!(clear > 50 && hits > 50, "{clear} clear, {hits} hits");
    assert_eq!(both, clear);
}

// cos and sin by series (the test's own, deterministic)
fn forge_math_free_sin(x: f64) -> f64 {
    let x = x - std::f64::consts::PI;
    let (mut t, mut s) = (x, 0.0);
    for k in 0..30 {
        s += t;
        t *= -x * x / (((2 * k + 2) * (2 * k + 3)) as f64);
    }
    -s
}
fn forge_math_free_cos(x: f64) -> f64 {
    let x = x - std::f64::consts::PI;
    let (mut t, mut s) = (1.0, 0.0);
    for k in 0..30 {
        s += t;
        t *= -x * x / (((2 * k + 1) * (2 * k + 2)) as f64);
    }
    -s
}

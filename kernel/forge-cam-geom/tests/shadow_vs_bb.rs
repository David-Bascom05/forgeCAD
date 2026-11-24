//! L6 for the closed forms by symmetry (shadow.rs): sloped planes with a horizontal direction and
//! vertical-axis cones, against the general branch and bound (an independent method) and against
//! sampling. shadow only ever proves a clearance: where it does, the branch and bound must not
//! find a hit, and no sampled pair of a face point and a tool position may be nearer than its
//! bound. Moves along the faces' symmetry (lines along the plane, arcs about the cone's axis) and
//! across it, through a 45° chamfer tool and a flat end mill.
use forge_cam_geom::check::{Face, Move, Surf, Verdict, certify_bb};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::shadow;
use forge_cam_geom::tool::{Cutter, Shape, assemble};

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

/// The unit vector at the rational half-angle parameter m (no libm in a test that must match
/// wasm).
fn unit(m: f64) -> [f64; 2] {
    let w = 1.0 + m * m;
    [(1.0 - m * m) / w, 2.0 * m / w]
}

fn tools() -> Vec<Shape> {
    vec![
        assemble(
            &Cutter::Chamfer {
                d_tip: 1.0,
                angle: 90.0,
                d: 10.0,
            },
            8.0,
            10.0,
            25.0,
            None,
        )
        .unwrap(),
        assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap(),
    ]
}

/// The least of the body's signed distance over a grid of face points and tool positions (an
/// upper bound on the true least: a sample).
fn sampled(body: &forge_cam_geom::tool::Body, f: &Face, mv: &Move) -> f64 {
    let mut m = f64::INFINITY;
    let n = 24;
    for i in 0..=n {
        for j in 0..=n {
            let pp = f.p.lo + (f.p.hi - f.p.lo) * i as f64 / n as f64;
            let qq = f.q.lo + (f.q.hi - f.q.lo) * j as f64 / n as f64;
            let y = f.surf.eval(Ival::pt(pp), Ival::pt(qq));
            let y = [y[0].mid(), y[1].mid(), y[2].mid()];
            for k in 0..=n {
                let a = mv.at(Ival::pt(k as f64 / n as f64));
                let a = [a[0].mid(), a[1].mid(), a[2].mid()];
                let rho = ((y[0] - a[0]).powi(2) + (y[1] - a[1]).powi(2)).sqrt();
                m = m.min(body.sd_rz(Ival::pt(rho), Ival::pt(y[2] - a[2])).mid());
            }
        }
    }
    m
}

#[test]
fn shadow_never_contradicts_the_branch_and_bound_or_the_samples() {
    let mut g = Rng(7);
    let tools = tools();
    let mut proven = 0;
    let cases = 300;
    for case in 0..cases {
        let t = &tools[case % 2];
        let cone = case % 4 >= 2;
        // the face
        let (e, z0) = (unit(g.r(-1.0, 1.0)), g.r(-2.0, 2.0));
        let f = if cone {
            let k = [1.0, -1.0, 0.5, -0.5][case % 3];
            let r0 = g.r(0.5, 6.0);
            let m = g.r(-0.4, 0.4);
            Face::new(
                Surf::Cone {
                    o: [g.r(-3.0, 3.0), g.r(-3.0, 3.0), z0],
                    x: [e[0], e[1], 0.0],
                    y: [-e[1], e[0], 0.0],
                    ax: [0.0, 0.0, if case % 3 == 0 { 1.0 } else { -1.0 }],
                    r0,
                    k,
                },
                Ival::new(m - 0.3, m + 0.3),
                Ival::new(0.0, g.r(0.2, 1.5)),
                None,
                1e-9,
                case as u32,
            )
        } else {
            // across e, down a slope of rational angle
            let sl = unit(g.r(0.2, 0.9));
            let n = [e[1], -e[0]];
            Face::new(
                Surf::Plane {
                    o: [g.r(-3.0, 3.0), g.r(-3.0, 3.0), z0],
                    u: [e[0], e[1], 0.0],
                    v: [n[0] * sl[0], n[1] * sl[0], -sl[1]],
                },
                Ival::new(0.0, g.r(2.0, 20.0)),
                Ival::new(0.0, g.r(0.2, 1.5)),
                None,
                1e-9,
                case as u32,
            )
        };
        // the move: along the symmetry (offset across it), or anywhere
        let z = z0 + g.r(-1.5, 0.5);
        let along = case % 3 != 2;
        let mv = match (&f.surf, along) {
            (Surf::Plane { o, u, .. }, true) => {
                let off = g.r(-1.5, 3.0);
                let s0 = g.r(-3.0, 10.0);
                let (n0, n1) = (u[1], -u[0]);
                let a = [o[0] + s0 * u[0] + off * n0, o[1] + s0 * u[1] + off * n1, z];
                let l = g.r(1.0, 15.0);
                Move::Line {
                    a,
                    b: [a[0] + l * u[0], a[1] + l * u[1], z],
                }
            }
            (Surf::Cone { o, x, y, .. }, true) => {
                let h = g.r(0.1, 0.6);
                Move::Arc {
                    c: [o[0], o[1], z],
                    x: *x,
                    y: *y,
                    r: g.r(0.3, 9.0),
                    t0: -h,
                    t1: h,
                }
            }
            _ => {
                let a = [g.r(-8.0, 8.0), g.r(-8.0, 8.0), z];
                Move::Line {
                    a,
                    b: [a[0] + g.r(-6.0, 6.0), a[1] + g.r(-6.0, 6.0), z],
                }
            }
        };
        let pl = mv.planar().unwrap();
        for body in &t.bodies {
            let limit = if case % 5 == 0 { 0.1 } else { -0.01 };
            let (lower, _) = shadow::clear(body, &f, &mv, &pl, limit, 0.0025, 200_000);
            let Some(lower) = lower else { continue };
            proven += 1;
            assert!(
                lower >= limit,
                "case {case}: a clearance below its limit: {lower} < {limit}"
            );
            let (v, _) = certify_bb(body, &f, &mv, limit, 0.0025, 200_000);
            if let Verdict::Hit { d, .. } = v {
                panic!("case {case}: shadow proved {lower}, the branch and bound found a hit at {d}");
            }
            let s = sampled(body, &f, &mv);
            assert!(
                s >= lower - 1e-9,
                "case {case}: a sample at {s} is nearer than the bound {lower}"
            );
        }
    }
    // not vacuous: most moves along the symmetry are proven
    // (474 at this seed)
    assert!(proven >= 300, "only {proven} clearances proven");
}

/// The edge break's own case: a 45° chamfer plane, the chamfer tool on a line along it with its
/// flank on the plane (distance 0): proven within tol. The same line 0.05 mm into the part: not
/// proven by shadow, and a hit for the branch and bound.
#[test]
fn a_flank_on_the_chamfer_clears_and_a_gouge_does_not() {
    let t = &tools()[0];
    let body = &t.bodies[0];
    let (s, rt2) = (0.3, std::f64::consts::FRAC_1_SQRT_2);
    // the plate's edge along x at y = 0, the top at z = 8, the part at y > 0: E's edge at y = s
    let f = Face::new(
        Surf::Plane {
            o: [0.0, s, 8.0],
            u: [1.0, 0.0, 0.0],
            v: [0.0, -rt2, -rt2],
        },
        Ival::new(0.0, 30.0),
        Ival::new(0.0, s * std::f64::consts::SQRT_2),
        None,
        1e-9,
        1,
    );
    // the axis c = r_tip + 0.2 off the edge, the tip 0.2 below the break
    let (c, zt) = (0.7, 8.0 - s - 0.2);
    for (inward, clears) in [(0.0, true), (0.05, false)] {
        let y = -c + inward;
        let mv = Move::Line {
            a: [2.0, y, zt],
            b: [28.0, y, zt],
        };
        let pl = mv.planar().unwrap();
        let (lower, cells) = shadow::clear(body, &f, &mv, &pl, -0.01, 0.0025, 200_000);
        let (v, _) = certify_bb(body, &f, &mv, -0.01, 0.0025, 200_000);
        if clears {
            let lower = lower.expect("the flank on the chamfer is proven");
            assert!((-0.01..=1e-6).contains(&lower), "{lower}");
            assert!(cells < 2000, "{cells} cells");
            assert!(!matches!(v, Verdict::Hit { .. }));
        } else {
            assert!(lower.is_none(), "a gouge of 0.05 / √2 proven clear: {lower:?}");
            assert!(matches!(v, Verdict::Hit { .. }), "{v:?}");
        }
    }
}

//! A helical move against a curved face (a torus: a fillet) by the general branch and bound,
//! which takes the helix's arc at every height of its range: every verdict against dense samples
//! of the real helix (its height linear in the angle, as a control interpolates it). A Clear
//! must hold at every sample; a Hit must be a real contact at its instant.
use forge_cam_geom::check::{Face, Move, Surf, Verdict, certify_bb};
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

/// atan by its series (|t| <= 0.4 here: 60 terms are exact to rounding)
fn atan(t: f64) -> f64 {
    let (mut p, mut s) = (t, 0.0);
    for k in 0..60 {
        s += p / (2 * k + 1) as f64;
        p *= -t * t;
    }
    s
}

/// The rational map's point: (cos, sin) of 2 atan(t).
fn circ(t: f64) -> (f64, f64) {
    let d = 1.0 + t * t;
    ((1.0 - t * t) / d, 2.0 * t / d)
}

#[test]
fn a_helix_against_a_torus_is_decided_and_every_verdict_holds() {
    let mut g = Rng(7);
    let tool = assemble(&Cutter::Flat { d: 6.0 }, 15.0, 6.0, 30.0, None).unwrap();
    let body = &tool.bodies[0];
    // a fillet's torus about z: tube centre radius 18, tube radius 3, the quarter facing out
    // and up about φm = 135° (the tube angle measured from the radial direction)
    let (cm, sm) = (-std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2);
    let (big, r) = (18.0, 3.0);
    let surf = Surf::Torus {
        o: [0.0, 0.0, 0.0],
        x: [1.0, 0.0, 0.0],
        y: [0.0, 1.0, 0.0],
        ax: [0.0, 0.0, 1.0],
        big,
        r,
        cm,
        sm,
    };
    let (pr, qr) = (Ival::new(-0.3, 0.3), Ival::new(-0.4, 0.4));
    let f = Face::new(surf, pr, qr, None, 0.0, 0);
    let (mut clear, mut hit, mut undecided) = (0, 0, 0);
    for _ in 0..120 {
        // a helix whose arc lies near the torus, below, beside or above it
        // (directions from normalised random vectors: no std trigonometry in the workspace)
        let u = g.r(-0.55, 0.55);
        let rc = g.r(6.0, 14.0);
        let n = (1.0 + u * u).sqrt();
        let c = [rc / n, rc * u / n, 0.0];
        let (ax, ay) = loop {
            let (vx, vy) = (g.r(-1.0, 1.0), g.r(-1.0, 1.0));
            let m = (vx * vx + vy * vy).sqrt();
            if m > 0.1 && m <= 1.0 {
                break (vx / m, vy / m);
            }
        };
        let (x, y) = ([ax, ay, 0.0], [-ay, ax, 0.0]);
        let z0 = g.r(-1.0, 8.0);
        let mv = Move::Helix {
            c,
            x,
            y,
            r: g.r(1.0, 5.0),
            t0: -g.r(0.05, 0.4),
            t1: g.r(0.05, 0.4),
            z0,
            z1: z0 - g.r(0.1, 1.0),
        };
        let Move::Helix {
            r: hr, t0, t1, z1, ..
        } = mv
        else {
            unreachable!()
        };
        // the real helix at s (t linear in s, its height linear in the angle)
        let (a0, a1) = (2.0 * atan(t0), 2.0 * atan(t1));
        let at = |s: f64| {
            let t = t0 + s * (t1 - t0);
            let (cc, ss) = circ(t);
            let ang = 2.0 * atan(t);
            let z = z0 + (z1 - z0) * (ang - a0) / (a1 - a0);
            [
                c[0] + hr * (x[0] * cc + y[0] * ss),
                c[1] + hr * (x[1] * cc + y[1] * ss),
                z,
            ]
        };
        let sd_at = |p: f64, q: f64, s: f64| {
            let yv = f.surf.eval(Ival::pt(p), Ival::pt(q));
            let gp = at(s);
            body.sd(
                Ival::pt(yv[0].mid() - gp[0]),
                Ival::pt(yv[1].mid() - gp[1]),
                Ival::pt(yv[2].mid() - gp[2]),
            )
            .mid()
        };
        let limit = -0.01;
        match certify_bb(body, &f, &mv, limit, 0.0025, 200_000).0 {
            Verdict::Clear { .. } => {
                clear += 1;
                let n = 40;
                for i in 0..=n {
                    for j in 0..=n {
                        for k in 0..=n {
                            let p = pr.lo + pr.width() * i as f64 / n as f64;
                            let q = qr.lo + qr.width() * j as f64 / n as f64;
                            let s = k as f64 / n as f64;
                            let d = sd_at(p, q, s);
                            assert!(d >= limit - 1e-9, "Clear, but {d} at p {p} q {q} s {s}: {mv:?}");
                        }
                    }
                }
            }
            Verdict::Hit { d, p, q, s, .. } => {
                hit += 1;
                let real = sd_at(p, q, s);
                assert!(real <= d + 1e-9, "Hit {d}, but the helix is {real} there: {mv:?}");
            }
            Verdict::Undecided { .. } => undecided += 1,
        }
    }
    assert!(
        clear > 20 && hit > 10,
        "{clear} clear, {hit} hit, {undecided} undecided"
    );
}

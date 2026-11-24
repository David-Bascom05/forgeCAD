//! L6 for shadow.rs's torus with a vertical axis (a fillet round a boss or a hole): against the
//! general branch and bound and against sampling, on arcs about the axis, arcs whose centre is a
//! little off it (a posted arc: its centre from rounded words) and lines anywhere. shadow only
//! proves clearances: where it does, the branch and bound finds no hit and no sample is nearer.
use forge_cam_geom::check::{Face, Move, Surf, Verdict, certify_bb};
use forge_cam_geom::ival::Ival;
use forge_cam_geom::shadow;
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

/// The unit vector at the rational half-angle parameter m (no std trigonometry).
fn unit(m: f64) -> [f64; 2] {
    let w = 1.0 + m * m;
    [(1.0 - m * m) / w, 2.0 * m / w]
}

fn sampled(body: &forge_cam_geom::tool::Body, f: &Face, mv: &Move) -> f64 {
    let mut m = f64::INFINITY;
    let n = 16;
    for i in 0..=n {
        for j in 0..=n {
            let pp = f.p.lo + (f.p.hi - f.p.lo) * i as f64 / n as f64;
            let qq = f.q.lo + (f.q.hi - f.q.lo) * j as f64 / n as f64;
            let y = f.surf.eval(Ival::pt(pp), Ival::pt(qq));
            let y = [y[0].mid(), y[1].mid(), y[2].mid()];
            for k in 0..=n {
                let a = mv.at(Ival::pt(k as f64 / n as f64));
                let a = [a[0].mid(), a[1].mid(), a[2].mid()];
                let (dx, dy) = (y[0] - a[0], y[1] - a[1]);
                let rho = (dx * dx + dy * dy).sqrt();
                m = m.min(body.sd_rz(Ival::pt(rho), Ival::pt(y[2] - a[2])).mid());
            }
        }
    }
    m
}

#[test]
fn a_torus_about_z_is_cleared_in_closed_form_and_never_wrongly() {
    let mut g = Rng(2026);
    let tools = [
        assemble(&Cutter::Flat { d: 10.0 }, 25.0, 10.0, 40.0, None).unwrap(),
        assemble(&Cutter::Flat { d: 3.0 }, 10.0, 3.0, 26.0, None).unwrap(),
    ];
    let (mut proven, mut tried_coaxial, mut proven_coaxial) = (0, 0, 0);
    for case in 0..96 {
        let t = &tools[case % 2];
        // a fillet's quarter tube: the foot of a boss (concave, below-outside) or of a hole
        let big = g.r(6.0, 20.0);
        let r = g.r(1.0, 4.0);
        let o = [g.r(-5.0, 5.0), g.r(-5.0, 5.0), g.r(0.0, 10.0)];
        let quarter = case % 4;
        let (cm, sm) = match quarter {
            0 => (1.0, 0.0),
            1 => (0.0, 1.0),
            2 => (-1.0, 0.0),
            _ => (0.0, -1.0),
        };
        let xa = unit(g.r(-1.0, 1.0));
        let f = Face::new(
            Surf::Torus {
                o,
                x: [xa[0], xa[1], 0.0],
                y: [-xa[1], xa[0], 0.0],
                ax: [0.0, 0.0, 1.0],
                big,
                r,
                cm,
                sm,
            },
            Ival::new(-1.0, 1.0),
            Ival::new(-0.41, 0.41),
            None,
            1e-9,
            case as u32,
        );
        let z = o[2] + g.r(-r - 2.0, r + 2.0);
        let mv = if case % 3 != 2 {
            // an arc about the axis, its centre 0 to 1e-3 off it
            tried_coaxial += 1;
            let eps = if case % 2 == 0 { 0.0 } else { g.r(0.0, 1e-3) };
            let h = g.r(0.1, 0.8);
            Move::Arc {
                c: [o[0] + eps, o[1] - eps, z],
                x: [1.0, 0.0, 0.0],
                y: [0.0, 1.0, 0.0],
                r: g.r(0.5, big + r + 8.0),
                t0: -h,
                t1: h,
            }
        } else {
            let a = [o[0] + g.r(-25.0, 25.0), o[1] + g.r(-25.0, 25.0), z];
            Move::Line {
                a,
                b: [a[0] + g.r(-6.0, 6.0), a[1] + g.r(-6.0, 6.0), z],
            }
        };
        let pl = mv.planar().unwrap();
        for body in &t.bodies {
            let limit = -0.01;
            let (lower, _) = shadow::clear(body, &f, &mv, &pl, limit, 0.0025, 200_000);
            let Some(lower) = lower else { continue };
            proven += 1;
            if case % 3 != 2 {
                proven_coaxial += 1;
            }
            assert!(lower >= limit, "case {case}: {lower} < {limit}");
            let (v, _) = certify_bb(body, &f, &mv, limit, 0.0025, 20_000);
            if let Verdict::Hit { d, .. } = v {
                panic!("case {case}: shadow proved {lower}, the branch and bound hit at {d}");
            }
            let s = sampled(body, &f, &mv);
            assert!(
                s >= lower - 1e-9,
                "case {case}: a sample at {s} nearer than {lower}"
            );
        }
    }
    eprintln!("{proven} proven, {proven_coaxial} of {tried_coaxial} arcs");
    assert!(
        proven >= 50 && proven_coaxial * 2 >= tried_coaxial,
        "{proven} proven, {proven_coaxial} of {tried_coaxial} arcs"
    );
}

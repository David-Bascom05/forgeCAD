//! The kernel's surfaces and fits against what lean/ForgeMass proves about them.
//!
//! - `normals_match_lean`: `deriv`'s du × dv, at random frames and parameters, against the closed
//!   forms of Surfaces.lean (`cylinder_n`, `cone_n`, `sphere_n`, `torus_n`), and the signed
//!   area integrands against r² cos v and r(R + r cos v).
//! - `chebyshev_weights_interpolate`: the one assumption of the fits along b. `cheb`'s weights
//!   (mass.rs:1849–1859, transcribed) make the barycentric formula reproduce every polynomial of
//!   degree < n, on the n = 4 and 5 points `Along::Poly` uses.
//! - `exact_breps_far_away`: boxes held exactly (axis-aligned, dyadic sizes, dyadic origins up to
//!   2^30 away), whose volume is exact: `mass` is within its `err`.

use forge_geom::{Cone, Cylinder, Sphere, SurfaceGeom, Torus};
use forge_math::rng::Rng;
use forge_math::{Frame, Point2, Point3, Vec3, dmath};

fn frame(r: &mut Rng) -> Frame {
    let z = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0))
        .normalize()
        .unwrap();
    let o = Point3::new(r.range(-50.0, 50.0), r.range(-50.0, 50.0), r.range(-50.0, 50.0));
    Frame::from_zx(o, z, Vec3::new(r.range(-1.0, 1.0), 1.0, 0.3)).unwrap()
}

fn rad(f: &Frame, u: f64) -> Vec3 {
    f.x().get() * dmath::cos(u) + f.y().get() * dmath::sin(u)
}

#[test]
fn normals_match_lean() {
    let mut r = Rng::new(11);
    let mut worst = 0.0f64;
    let mut check = |got: Vec3, want: Vec3, scale: f64| {
        let d = (got - want).length() / scale;
        worst = worst.max(d);
        assert!(d < 1e-13, "{got:?} vs {want:?}");
    };
    for _ in 0..500 {
        let f = frame(&mut r);
        let (u, v) = (r.range(-7.0, 7.0), r.range(-1.5, 1.5));
        let z = f.z().get();
        // cylinder_n: n = r rad u
        let rr = r.range(0.5, 20.0);
        let d = Cylinder { frame: f, radius: rr }.deriv(Point2::new(u, v));
        check(d.du.cross(d.dv), rad(&f, u) * rr, rr);
        // cone_n: n = ρ (rad u − k Z), ρ = r + v k
        let half = r.range(0.1, 1.2);
        let cone = Cone::new(f, rr, half).unwrap();
        let k = cone.tan_a();
        let rho = rr + v * k;
        let d = cone.deriv(Point2::new(u, v));
        check(d.du.cross(d.dv), (rad(&f, u) - z * k) * rho, rr * (1.0 + k * k));
        // sphere_n: n = r² cos v (cos v rad u + sin v Z); sphere_signed: n·(p − o)/r = r² cos v
        let s = Sphere { frame: f, radius: rr };
        let d = s.deriv(Point2::new(u, v));
        let n = d.du.cross(d.dv);
        check(
            n,
            (rad(&f, u) * dmath::cos(v) + z * dmath::sin(v)) * (rr * rr * dmath::cos(v)),
            rr * rr,
        );
        let signed = n.dot(d.p - f.origin) / rr;
        assert!((signed - rr * rr * dmath::cos(v)).abs() < 1e-12 * rr * rr);
        assert!(
            (signed - n.length()).abs() < 1e-12 * rr * rr,
            "between the poles it is |n|"
        );
        // torus_n: n = r ρ (cos v rad u + sin v Z), ρ = R + r cos v; torus_signed: −n·dvv/r = r ρ
        let minor = r.range(0.5, 5.0);
        let major = minor + r.range(0.5, 20.0);
        let t = Torus::new(f, major, minor).unwrap();
        let d = t.deriv(Point2::new(u, v));
        let rho = major + minor * dmath::cos(v);
        let n = d.du.cross(d.dv);
        check(
            n,
            (rad(&f, u) * dmath::cos(v) + z * dmath::sin(v)) * (minor * rho),
            minor * major,
        );
        assert!((-n.dot(d.dvv) / minor - minor * rho).abs() < 1e-12 * minor * major);
    }
    eprintln!("worst relative difference: {worst:e}");
}

/// mass.rs:1849–1859
fn cheb(n: usize, lo: f64, hi: f64) -> (Vec<f64>, Vec<f64>) {
    (0..n)
        .map(|k| {
            let (s, c) = dmath::sin_cos((2 * k + 1) as f64 * dmath::PI / (2 * n) as f64);
            (
                0.5 * (lo + hi) - 0.5 * (hi - lo) * c,
                if k % 2 == 0 { s } else { -s },
            )
        })
        .unzip()
}

#[test]
fn chebyshev_weights_interpolate() {
    let mut r = Rng::new(5);
    let mut worst = 0.0f64;
    for n in [4, 5] {
        for _ in 0..200 {
            let (lo, hi) = (r.range(-10.0, 0.0), r.range(0.1, 10.0));
            let (bs, lam) = cheb(n, lo, hi);
            let c: Vec<f64> = (0..n).map(|_| r.range(-1.0, 1.0)).collect();
            let p = |x: f64| c.iter().rev().fold(0.0, |a, &ci| a * x + ci);
            let b = r.range(lo, hi);
            // mass.rs:2040–2050
            let w: Vec<f64> = (0..n).map(|i| lam[i] / (b - bs[i])).collect();
            let sum: f64 = w.iter().sum();
            let got: f64 = (0..n).map(|i| w[i] / sum * p(bs[i])).sum();
            let scale =
                c.iter().map(|x| x.abs()).sum::<f64>() * (1.0 + lo.abs().max(hi.abs())).powi(n as i32);
            let d = (got - p(b)).abs() / scale;
            worst = worst.max(d);
            assert!(d < 1e-12, "n {n}: {got} vs {}", p(b));
        }
    }
    eprintln!("worst relative difference: {worst:e}");
}

#[test]
fn exact_breps_far_away() {
    use forge_ops::{Profile, extrude};
    for e in [0, 10, 20, 30] {
        let far = 2.0f64.powi(e);
        for (a, b, t) in [(1.0, 0.5, 0.25), (3.0, 1.25, 0.75), (0.375, 2.5, 1.5)] {
            let mut f = Frame::WORLD;
            f.origin = Point3::new(far, -far, far * 0.5);
            let p = Profile {
                plane: f,
                loops: vec![Profile::rectangle(Point2::new(far, 0.0), a, b, 0)],
            };
            let m = forge_topo::mass::mass(&extrude(&p, t, 1).unwrap()).unwrap();
            let off = (m.volume - a * b * t).abs();
            assert!(off <= m.err[0], "2^{e}: off {off:e}, err {:e}", m.err[0]);
        }
    }
}

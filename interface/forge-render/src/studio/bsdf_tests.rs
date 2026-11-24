//! The BSDF's maths, checked by Monte Carlo with fixed seeds: energy, reciprocity, the pdf, the
//! sampler against the pdf, and Fresnel's limits.

use super::bsdf::*;
use super::v3::{V3, rand, v3};
use std::f32::consts::PI;
use std::sync::OnceLock;

fn lut() -> &'static Lut {
    static L: OnceLock<Lut> = OnceLock::new();
    L.get_or_init(|| Lut::new(1024))
}

fn dir(mu: f32, phi: f32) -> V3 {
    let s = (1.0 - mu * mu).max(0.0).sqrt();
    v3(s * phi.cos(), s * phi.sin(), mu)
}

fn uniform_sphere(u1: f32, u2: f32) -> V3 {
    dir(1.0 - 2.0 * u1, 2.0 * PI * u2)
}

/// The mean of f cos / pdf over `sample`'s directions: the albedo seen from `wo`, transmission
/// counted as energy (times eta^2, radiance's compression undone).
fn albedo(b: &Bsdf, wo: V3, n: u32, seed: u32) -> (f32, f32) {
    let (mut r, mut t) = (0.0f64, 0.0f64);
    for s in 0..n {
        let u = [rand(7, s, 0, seed), rand(7, s, 1, seed), rand(7, s, 2, seed)];
        if let Some((wi, f, pdf)) = sample(b, lut(), wo, u) {
            let w = f.lum() * wi.z.abs() / pdf;
            if (wi.z > 0.0) == (wo.z > 0.0) {
                r += w as f64;
            } else {
                let eta = if wo.z > 0.0 { b.ior } else { 1.0 / b.ior };
                t += (w * eta * eta) as f64;
            }
        }
    }
    ((r / n as f64) as f32, (t / n as f64) as f32)
}

fn materials() -> Vec<(&'static str, Bsdf)> {
    let mut v = Vec::new();
    for rough in [0.05, 0.3, 0.6, 1.0] {
        v.push((
            "white plastic",
            Bsdf {
                base: V3::ONE,
                rough,
                ..Bsdf::default()
            },
        ));
        v.push((
            "white mirror metal",
            Bsdf {
                base: V3::ONE,
                f82: V3::ONE,
                metal: 1.0,
                rough,
                ..Bsdf::default()
            },
        ));
        v.push((
            "aluminium",
            Bsdf {
                base: v3(0.913, 0.922, 0.924),
                f82: v3(0.97, 0.97, 0.97),
                metal: 1.0,
                rough,
                ..Bsdf::default()
            },
        ));
        v.push((
            "coated white",
            Bsdf {
                base: V3::ONE,
                rough,
                coat: 1.0,
                coat_rough: 0.25,
                ..Bsdf::default()
            },
        ));
        v.push((
            "clear glass",
            Bsdf {
                base: V3::ONE,
                rough,
                transmission: 1.0,
                ..Bsdf::default()
            },
        ));
    }
    v
}

#[test]
fn the_tables_hold_what_ggx_reflects() {
    let l = lut();
    // a smooth mirror with F = 1 reflects everything, at any angle
    for mu in [0.2, 0.5, 1.0] {
        assert!(
            (l.e1(mu, 0.0) - 1.0).abs() < 0.01,
            "E1({mu}, 0) = {}",
            l.e1(mu, 0.0)
        );
    }
    // roughness loses energy to single scattering, most at grazing
    // at alpha 1, D is flat: about 0.31 at normal incidence (checked by brute force)
    assert!((l.e1(1.0, 1.0) - 0.307).abs() < 0.01 && l.e1_avg(1.0) < 0.5);
    // Schlick's F0 = 0.04 at normal incidence, smooth: 4 %
    assert!((l.e_spec(0.04, 1.0, 0.0) - 0.04).abs() < 0.005);
    for j in 0..LUT_N {
        let r = lut_rough(j);
        assert!(
            l.e1_avg(r) <= 1.0 + 1e-3 && l.e1_avg(r) > 0.3,
            "{r}: {}",
            l.e1_avg(r)
        );
    }
}

#[test]
fn nothing_reflects_more_than_arrives() {
    for (name, b) in materials() {
        for mu in [0.05, 0.3, 0.7, 1.0] {
            let wo = dir(mu, 0.3);
            let (r, t) = albedo(&b, wo, 20000, 1);
            assert!(r + t <= 1.02, "{name} rough {} at mu {mu}: {r} + {t}", b.rough);
            if b.transmission > 0.0 {
                // and from inside the glass
                let (r2, t2) = albedo(&b, -wo, 20000, 2);
                assert!(
                    r2 + t2 <= 1.02,
                    "{name} inside rough {} at mu {mu}: {r2} + {t2}",
                    b.rough
                );
            }
        }
    }
}

#[test]
fn white_furnaces_lose_almost_nothing() {
    // a white conductor (F = 1) with the multiple-scattering term, and white plastic (Kelemen's
    // coupling): all of it comes back, at every roughness, within the tables' error
    for (name, b) in materials() {
        if name != "white mirror metal" && name != "white plastic" {
            continue;
        }
        for mu in [0.2, 0.5, 1.0] {
            let (r, _) = albedo(&b, dir(mu, 1.0), 40000, 3);
            assert!(r > 0.95 && r < 1.02, "{name} rough {} at mu {mu}: {r}", b.rough);
        }
    }
    // smooth glass: reflected plus transmitted energy is all of it
    let glass = Bsdf {
        base: V3::ONE,
        rough: 0.05,
        transmission: 1.0,
        ..Bsdf::default()
    };
    for mu in [0.3, 0.8] {
        let (r, t) = albedo(&glass, dir(mu, 0.0), 40000, 4);
        assert!((r + t - 1.0).abs() < 0.03, "glass at {mu}: {r} + {t}");
        let f = dielectric_fresnel(mu, 1.5);
        assert!((r - f).abs() < 0.02, "glass reflects {r}, Fresnel says {f}");
    }
}

#[test]
fn light_paths_reverse() {
    for (name, b) in materials() {
        for s in 0..200u32 {
            let wo = uniform_sphere(rand(1, s, 0, 9), rand(1, s, 1, 9));
            let wi = uniform_sphere(rand(1, s, 2, 9), rand(1, s, 3, 9));
            let (f1, _) = eval(&b, lut(), wo, wi);
            let (f2, _) = eval(&b, lut(), wi, wo);
            if b.transmission > 0.0 && (wo.z > 0.0) != (wi.z > 0.0) {
                // f(i, o) / eta_o^2 = f(o, i) / eta_i^2
                let (eo, ei) = if wo.z > 0.0 { (1.0, b.ior) } else { (b.ior, 1.0) };
                let (a, c) = (f1.lum() / (eo * eo), f2.lum() / (ei * ei));
                assert!(
                    (a - c).abs() <= 1e-3 * a.max(c) + 1e-6,
                    "{name} {wo:?} {wi:?}: {a} vs {c}"
                );
            } else if (wo.z > 0.0) == (wi.z > 0.0) {
                let (a, c) = (f1.lum(), f2.lum());
                assert!(
                    (a - c).abs() <= 1e-3 * a.max(c) + 1e-6,
                    "{name} {wo:?} {wi:?}: {a} vs {c}"
                );
            }
        }
    }
}

#[test]
fn every_pdf_integrates_to_at_most_one() {
    for (name, b) in materials() {
        if b.rough < 0.3 {
            // a spike: a uniform estimate of its integral is all noise
            continue;
        }
        for mu in [0.2, 0.9] {
            let wo = dir(mu, 0.7);
            let n = 200000;
            let mut s = 0.0f64;
            for k in 0..n {
                let wi = uniform_sphere(rand(3, k, 0, 5), rand(3, k, 1, 5));
                s += eval(&b, lut(), wo, wi).1 as f64;
            }
            let total = (s / n as f64 * 4.0 * PI as f64) as f32;
            // the spiky lobes leave the uniform estimate noisy: smooth ones must come out near 1
            assert!(total < 1.08, "{name} rough {} mu {mu}: {total}", b.rough);
            // rough lobes reflect some of their normals below the horizon: lost, not wrong
            if b.transmission == 0.0 && b.rough <= 0.6 {
                assert!(total > 0.85, "{name} rough {} mu {mu}: {total}", b.rough);
            }
        }
    }
}

#[test]
fn the_sampler_draws_what_the_pdf_says() {
    // bins over cos(theta) and phi, both hemispheres: counts of sampled directions against the
    // pdf integrated over each bin
    let (nt, np) = (8usize, 12usize);
    let bin = |w: V3| {
        let t = (((w.z + 1.0) * 0.5) * nt as f32).min(nt as f32 - 1.0) as usize;
        let p = ((w.y.atan2(w.x) / (2.0 * PI) + 0.5) * np as f32).min(np as f32 - 1.0) as usize;
        t * np + p
    };
    for (name, b) in materials() {
        if b.rough < 0.3 {
            continue;
        }
        let wo = dir(0.6, 0.4);
        let n = 100000u32;
        let m = 2000000u32;
        let mut got = vec![0f64; nt * np];
        let mut want = vec![0f64; nt * np];
        let mut kept = 0u32;
        for s in 0..n {
            let u = [rand(5, s, 0, 11), rand(5, s, 1, 11), rand(5, s, 2, 11)];
            if let Some((wi, _, _)) = sample(&b, lut(), wo, u) {
                got[bin(wi)] += 1.0;
                kept += 1;
            }
        }
        for s in 0..m {
            let w = uniform_sphere(rand(6, s, 0, 12), rand(6, s, 1, 12));
            want[bin(w)] += eval(&b, lut(), wo, w).1 as f64 * 4.0 * PI as f64 * n as f64 / m as f64;
        }
        let mut chi = 0.0;
        let mut dof = 0;
        for k in 0..nt * np {
            let e = want[k];
            if e > 20.0 {
                let d = got[k] - e;
                chi += d * d / e;
                dof += 1;
            }
        }
        // the expected counts are themselves estimates: allow their noise as well
        assert!(
            chi / (dof as f64) < 4.0,
            "{name} rough {}: chi2/dof {} over {dof} bins, kept {kept}",
            b.rough,
            chi / dof as f64
        );
    }
}

#[test]
fn fresnel_meets_its_limits() {
    // an absorbing medium with k = 0 is a dielectric
    for c in [0.1, 0.5, 1.0] {
        let a = conductor_reflectance(1.5, 0.0, c);
        let d = dielectric_fresnel(c, 1.5);
        assert!((a - d).abs() < 1e-4, "{c}: {a} vs {d}");
    }
    assert!((f0_of_ior(1.5) - 0.04).abs() < 1e-6);
    // F82-tint: F0 at normal incidence, F82 at cos 1/7, white at grazing
    let (f0, f82) = (v3(0.9, 0.6, 0.3), v3(0.8, 0.7, 0.9));
    let at = |c| conductor_fresnel(f0, f82, c);
    assert!((at(1.0) - f0).len() < 1e-5);
    let s = schlick(0.6, 1.0 / 7.0);
    assert!((at(1.0 / 7.0).y - 0.7 * s).abs() < 1e-4);
    assert!((at(0.0) - V3::ONE).len() < 1e-5);
    // its mean, by quadrature
    let n = 20000;
    let mut m = V3::ZERO;
    for k in 0..n {
        let c = (k as f32 + 0.5) / n as f32;
        m = m + at(c) * (2.0 * c / n as f32);
    }
    let avg = conductor_fresnel_avg(f0, f82);
    assert!((m - avg).len() < 1e-3, "{m:?} vs {avg:?}");
    // at normal incidence the closed form ((n - 1)^2 + k^2) / ((n + 1)^2 + k^2)
    let (n, k) = (0.958f32, 6.69f32);
    let al = conductor_reflectance(n, k, 1.0);
    let want = ((n - 1.0).powi(2) + k * k) / ((n + 1.0).powi(2) + k * k);
    assert!((al - want).abs() < 1e-5, "{al} vs {want}");
}

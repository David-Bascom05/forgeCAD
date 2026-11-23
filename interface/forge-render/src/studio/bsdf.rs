//! The studio's BSDF on the CPU: the same model, sampling and tables as `trace.wgsl`, written
//! once more in Rust so its maths can be tested where there is no GPU.
//!
//! The model, in the shading frame (z the normal, `wo` toward the eye, `wi` toward the light):
//! a clear coat over a mix of three bases, weighted by `metal` and `transmission`:
//!
//! - conductor: GGX (Trowbridge-Reitz) with Smith height-correlated masking, the F82-tint Fresnel
//!   (Hoffman 2023, "Generalization of Adobe's Fresnel Model"), and Kulla-Conty's multiple-scatter
//!   term (Kulla & Conty 2017, "Revisiting Physically Based Shading at Imageworks");
//! - dielectric over diffuse ("plastic"): GGX with Schlick's Fresnel, Lambert below it coupled by
//!   the specular's directional albedo (Kelemen & Szirmay-Kalos 2001), so it stays reciprocal and
//!   never reflects more than arrives;
//! - rough glass: GGX reflection and refraction with the exact dielectric Fresnel (Walter et al.
//!   2007, "Microfacet Models for Refraction through Rough Surfaces");
//! - the coat: GGX with Schlick at IOR 1.5, coupled to what is under it the same way as the
//!   diffuse.
//!
//! Sampling: visible normals by spherical caps (Dupuy & Benyoub 2023, "Sampling Visible GGX
//! Normals with Spherical Caps"), cosine for the diffuse; one lobe picked by its estimated albedo,
//! the pdf the whole mixture's.

use super::v3::{V3, v3};
use std::f32::consts::PI;

/// The roughest a surface can be smooth: below this GGX's peak no longer fits in f32 well.
pub const ALPHA_MIN: f32 = 1.0e-3;
/// The tables' size per axis: cos(theta) and roughness.
pub const LUT_N: usize = 32;

/// A surface's appearance, resolved: what the GPU's material record holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bsdf {
    /// Albedo for dielectrics, F0 (normal reflectance) for conductors, the tint for glass.
    pub base: V3,
    pub metal: f32,
    /// A conductor's reflectance at 82 degrees (F82), its edge colour.
    pub f82: V3,
    /// Perceptual roughness: GGX's alpha is its square.
    pub rough: f32,
    pub ior: f32,
    pub transmission: f32,
    pub coat: f32,
    pub coat_rough: f32,
}

impl Default for Bsdf {
    fn default() -> Bsdf {
        Bsdf {
            base: V3::splat(0.8),
            metal: 0.0,
            f82: V3::ONE,
            rough: 0.5,
            ior: 1.5,
            transmission: 0.0,
            coat: 0.0,
            coat_rough: 0.03,
        }
    }
}

pub fn alpha(rough: f32) -> f32 {
    (rough * rough).max(ALPHA_MIN)
}

// ---------------------------------------------------------------- microfacets

/// GGX's distribution of normals, isotropic.
pub fn ggx_d(m: V3, a: f32) -> f32 {
    if m.z <= 0.0 {
        return 0.0;
    }
    let a2 = a * a;
    let c2 = m.z * m.z;
    let d = c2 * (a2 - 1.0) + 1.0;
    a2 / (PI * d * d)
}

/// Smith's Lambda for GGX.
pub fn lambda(w: V3, a: f32) -> f32 {
    let c2 = w.z * w.z;
    let s2 = (1.0 - c2).max(0.0);
    if s2 <= 0.0 {
        return 0.0;
    }
    let t2 = s2 / c2;
    0.5 * (-1.0 + (1.0 + a * a * t2).sqrt())
}

pub fn g1(w: V3, a: f32) -> f32 {
    1.0 / (1.0 + lambda(w, a))
}

/// Height-correlated masking-shadowing.
pub fn g2(wo: V3, wi: V3, a: f32) -> f32 {
    1.0 / (1.0 + lambda(wo, a) + lambda(wi, a))
}

/// A visible normal for `wo` (upper hemisphere), by spherical caps.
pub fn sample_vndf(wo: V3, a: f32, u1: f32, u2: f32) -> V3 {
    let wi_std = v3(a * wo.x, a * wo.y, wo.z).norm();
    let phi = 2.0 * PI * u1;
    let z = (1.0 - u2) * (1.0 + wi_std.z) - wi_std.z;
    let s = (1.0 - z * z).clamp(0.0, 1.0).sqrt();
    let c = v3(s * phi.cos(), s * phi.sin(), z);
    let h = c + wi_std;
    v3(a * h.x, a * h.y, h.z.max(0.0)).norm()
}

/// The density of `sample_vndf`'s normals.
pub fn vndf_pdf(wo: V3, m: V3, a: f32) -> f32 {
    let om = wo.dot(m);
    if om <= 0.0 || wo.z <= 0.0 {
        return 0.0;
    }
    g1(wo, a) * om * ggx_d(m, a) / wo.z
}

pub fn reflect(w: V3, m: V3) -> V3 {
    m * (2.0 * w.dot(m)) - w
}

/// `w` refracted through `m` (both on the same side), `eta` = inside / outside index; none at
/// total internal reflection.
pub fn refract(w: V3, m: V3, eta: f32) -> Option<V3> {
    let c = w.dot(m);
    let k = 1.0 - (1.0 - c * c) / (eta * eta);
    if k <= 0.0 {
        return None;
    }
    Some((m * (c / eta - k.sqrt()) - w / eta).norm())
}

// ---------------------------------------------------------------- Fresnel

pub fn schlick(f0: f32, c: f32) -> f32 {
    let m = (1.0 - c).clamp(0.0, 1.0);
    let m2 = m * m;
    f0 + (1.0 - f0) * m2 * m2 * m
}

/// The F82-tint conductor Fresnel: Schlick bent down to meet F82 at cos = 1/7.
pub fn conductor_fresnel(f0: V3, f82: V3, c: f32) -> V3 {
    let mb: f32 = 1.0 / 7.0;
    let k = 1.0 / (mb * (1.0 - mb).powi(6));
    let fs = |f0: f32| schlick(f0, mb);
    let a = v3(
        (fs(f0.x) - f82.x * fs(f0.x)) * k,
        (fs(f0.y) - f82.y * fs(f0.y)) * k,
        (fs(f0.z) - f82.z * fs(f0.z)) * k,
    );
    let t = c * (1.0 - c).powi(6);
    v3(
        (schlick(f0.x, c) - a.x * t).clamp(0.0, 1.0),
        (schlick(f0.y, c) - a.y * t).clamp(0.0, 1.0),
        (schlick(f0.z, c) - a.z * t).clamp(0.0, 1.0),
    )
}

/// Its cosine-weighted hemispherical mean: Schlick's is F0 + (1 - F0) / 21, the bend's 1/126.
pub fn conductor_fresnel_avg(f0: V3, f82: V3) -> V3 {
    let mb: f32 = 1.0 / 7.0;
    let k = 1.0 / (mb * (1.0 - mb).powi(6));
    let one = |f0: f32, f82: f32| {
        let fs = schlick(f0, mb);
        let a = (fs - f82 * fs) * k;
        (f0 + (1.0 - f0) / 21.0 - a / 126.0).clamp(0.0, 1.0)
    };
    v3(one(f0.x, f82.x), one(f0.y, f82.y), one(f0.z, f82.z))
}

/// The exact Fresnel reflectance of a dielectric, for light arriving at cos `ci` from the side
/// whose index is 1 and `eta` the other side's (relative).
pub fn dielectric_fresnel(ci: f32, eta: f32) -> f32 {
    let ci = ci.abs().min(1.0);
    let s2 = (1.0 - ci * ci) / (eta * eta);
    if s2 >= 1.0 {
        return 1.0;
    }
    let ct = (1.0 - s2).sqrt();
    let rs = (ci - eta * ct) / (ci + eta * ct);
    let rp = (eta * ci - ct) / (eta * ci + ct);
    0.5 * (rs * rs + rp * rp)
}

pub fn f0_of_ior(ior: f32) -> f32 {
    let r = (ior - 1.0) / (ior + 1.0);
    r * r
}

/// An absorbing medium's normal-incidence reflectance from its complex index n + ik, against air:
/// how measured optical constants become a metal's F0 and, at 82 degrees, its F82.
pub fn conductor_reflectance(n: f32, k: f32, cos_t: f32) -> f32 {
    // Born & Wolf, the exact s and p reflectances of an absorbing medium
    let c2 = cos_t * cos_t;
    let s2 = 1.0 - c2;
    let n2k2 = n * n - k * k - s2;
    let a2b2 = (n2k2 * n2k2 + 4.0 * n * n * k * k).sqrt();
    let a = (0.5 * (a2b2 + n2k2)).max(0.0).sqrt();
    let rs = (a2b2 + c2 - 2.0 * a * cos_t) / (a2b2 + c2 + 2.0 * a * cos_t);
    let rp = rs * (a2b2 * c2 + s2 * s2 - 2.0 * a * cos_t * s2) / (a2b2 * c2 + s2 * s2 + 2.0 * a * cos_t * s2);
    0.5 * (rs + rp)
}

// ---------------------------------------------------------------- tables

/// The GGX specular's directional albedo, split as F0 * A + B (Schlick is linear in F0), over
/// cos(theta) and perceptual roughness, and its cosine-weighted means.
#[derive(Clone, Debug)]
pub struct Lut {
    pub a: Vec<f32>,
    pub b: Vec<f32>,
    pub a_avg: Vec<f32>,
    pub b_avg: Vec<f32>,
}

/// Hammersley's points: the tables' fixed quadrature.
fn radical_inverse(mut i: u32) -> f32 {
    i = i.rotate_right(16);
    i = ((i & 0x55555555) << 1) | ((i & 0xAAAAAAAA) >> 1);
    i = ((i & 0x33333333) << 2) | ((i & 0xCCCCCCCC) >> 2);
    i = ((i & 0x0F0F0F0F) << 4) | ((i & 0xF0F0F0F0) >> 4);
    i = ((i & 0x00FF00FF) << 8) | ((i & 0xFF00FF00) >> 8);
    i as f32 * (1.0 / 4294967296.0)
}

pub fn lut_mu(k: usize) -> f32 {
    (k as f32 / (LUT_N - 1) as f32).max(1.0e-3)
}

pub fn lut_rough(j: usize) -> f32 {
    j as f32 / (LUT_N - 1) as f32
}

impl Lut {
    pub fn new(samples: u32) -> Lut {
        let n = LUT_N;
        let mut a = vec![0.0; n * n];
        let mut b = vec![0.0; n * n];
        for j in 0..n {
            let al = alpha(lut_rough(j));
            for k in 0..n {
                let mu = lut_mu(k);
                let wo = v3((1.0 - mu * mu).sqrt(), 0.0, mu);
                let (mut sa, mut sb) = (0.0f64, 0.0f64);
                for s in 0..samples {
                    let m = sample_vndf(wo, al, (s as f32 + 0.5) / samples as f32, radical_inverse(s));
                    let wi = reflect(wo, m);
                    if wi.z <= 0.0 {
                        continue;
                    }
                    // f cos / pdf with F = 1: G2 / G1(wo)
                    let w = g2(wo, wi, al) / g1(wo, al);
                    let c = (1.0 - wo.dot(m)).clamp(0.0, 1.0);
                    let c5 = c * c * c * c * c;
                    sa += (w * (1.0 - c5)) as f64;
                    sb += (w * c5) as f64;
                }
                a[j * n + k] = (sa / samples as f64) as f32;
                b[j * n + k] = (sb / samples as f64) as f32;
            }
        }
        // the means 2 * integral of E(mu) mu dmu, by the trapezoid rule on the grid
        let avg = |t: &[f32], j: usize| {
            let mut s = 0.0;
            for k in 0..n - 1 {
                let (m0, m1) = (lut_mu(k), lut_mu(k + 1));
                s += (t[j * n + k] * m0 + t[j * n + k + 1] * m1) * 0.5 * (m1 - m0);
            }
            2.0 * s
        };
        let a_avg = (0..n).map(|j| avg(&a, j)).collect();
        let b_avg = (0..n).map(|j| avg(&b, j)).collect();
        Lut { a, b, a_avg, b_avg }
    }

    fn at(t: &[f32], mu: f32, rough: f32) -> f32 {
        let n = LUT_N;
        let x = (mu.clamp(0.0, 1.0) * (n - 1) as f32).clamp(0.0, (n - 1) as f32);
        let y = (rough.clamp(0.0, 1.0) * (n - 1) as f32).clamp(0.0, (n - 1) as f32);
        let (x0, y0) = ((x as usize).min(n - 2), (y as usize).min(n - 2));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let v = |i: usize, j: usize| t[j * n + i];
        let lo = v(x0, y0) * (1.0 - fx) + v(x0 + 1, y0) * fx;
        let hi = v(x0, y0 + 1) * (1.0 - fx) + v(x0 + 1, y0 + 1) * fx;
        lo * (1.0 - fy) + hi * fy
    }

    fn avg(t: &[f32], rough: f32) -> f32 {
        let n = LUT_N;
        let y = (rough.clamp(0.0, 1.0) * (n - 1) as f32).clamp(0.0, (n - 1) as f32);
        let y0 = (y as usize).min(n - 2);
        let f = y - y0 as f32;
        t[y0] * (1.0 - f) + t[y0 + 1] * f
    }

    /// The Schlick specular's albedo at cos `mu`.
    pub fn e_spec(&self, f0: f32, mu: f32, rough: f32) -> f32 {
        (f0 * Lut::at(&self.a, mu, rough) + Lut::at(&self.b, mu, rough)).min(1.0)
    }

    pub fn e_spec_avg(&self, f0: f32, rough: f32) -> f32 {
        (f0 * Lut::avg(&self.a_avg, rough) + Lut::avg(&self.b_avg, rough)).min(1.0)
    }

    /// The F = 1 specular's albedo (single scattering).
    pub fn e1(&self, mu: f32, rough: f32) -> f32 {
        self.e_spec(1.0, mu, rough)
    }

    pub fn e1_avg(&self, rough: f32) -> f32 {
        self.e_spec_avg(1.0, rough)
    }

    /// The tables as the GPU's buffer: A, B, then the means of A and B.
    pub fn flat(&self) -> Vec<f32> {
        let mut v = self.a.clone();
        v.extend(&self.b);
        v.extend(&self.a_avg);
        v.extend(&self.b_avg);
        v
    }
}

// ---------------------------------------------------------------- the lobes

/// A GGX reflection with a Fresnel already found: f without F, and its pdf by `sample_vndf`.
fn spec_reflect(wo: V3, wi: V3, a: f32) -> (f32, f32, V3) {
    if wo.z <= 0.0 || wi.z <= 0.0 {
        return (0.0, 0.0, V3::ZERO);
    }
    let m = (wo + wi).norm();
    let d = ggx_d(m, a);
    let f = d * g2(wo, wi, a) / (4.0 * wo.z * wi.z);
    let pdf = g1(wo, a) * d / (4.0 * wo.z);
    (f, pdf, m)
}

fn cosine_pdf(wi: V3) -> f32 {
    wi.z.max(0.0) / PI
}

/// How the lobes are picked for `wo`: coat, conductor spec, plastic spec, diffuse, glass.
fn lobe_odds(b: &Bsdf, lut: &Lut, mu: f32) -> [f32; 5] {
    let coat = b.coat * lut.e_spec(f0_of_ior(1.5), mu, b.coat_rough);
    let under = 1.0 - coat;
    let glass = (1.0 - b.metal) * b.transmission;
    let plastic = (1.0 - b.metal) * (1.0 - b.transmission);
    let es = lut.e_spec(f0_of_ior(b.ior), mu, b.rough);
    let diffuse = (1.0 - es) * b.base.max_elem().max(0.02);
    let spec_share = es / (es + diffuse).max(1e-6);
    let mut p = [
        coat,
        under * b.metal,
        under * plastic * spec_share,
        under * plastic * (1.0 - spec_share),
        under * glass,
    ];
    let sum: f32 = p.iter().sum();
    for x in &mut p {
        *x /= sum.max(1e-9);
    }
    p
}

/// What the light from `wi` gives toward `wo` (radiance BSDF, without the cosine), and the pdf
/// of `sample` choosing `wi`.
pub fn eval(b: &Bsdf, lut: &Lut, wo: V3, wi: V3) -> (V3, f32) {
    let flip = wo.z < 0.0;
    // glass sees both sides; the opaque lobes only the side `wo` is on
    let (wo, wi) = if flip { (-wo, -wi) } else { (wo, wi) };
    let mu_o = wo.z.max(1e-6);
    let odds = lobe_odds(b, lut, mu_o);
    let mut f = V3::ZERO;
    let mut pdf = 0.0;

    // the coat over everything
    let f0c = f0_of_ior(1.5);
    let ac = alpha(b.coat_rough);
    let mut under_o = 1.0;
    let mut under_i = 1.0;
    if b.coat > 0.0 {
        under_o = 1.0 - b.coat * lut.e_spec(f0c, mu_o, b.coat_rough);
        under_i = 1.0 - b.coat * lut.e_spec(f0c, wi.z.abs(), b.coat_rough);
        let (fs, ps, m) = spec_reflect(wo, wi, ac);
        if fs > 0.0 {
            f = f + V3::splat(b.coat * schlick(f0c, wo.dot(m)) * fs);
            pdf += odds[0] * ps;
        }
    }
    let under = under_o * under_i;

    let a = alpha(b.rough);
    // conductor
    if b.metal > 0.0 && wi.z > 0.0 {
        let (fs, ps, m) = spec_reflect(wo, wi, a);
        let fr = conductor_fresnel(b.base, b.f82, wo.dot(m));
        let e1o = lut.e1(mu_o, b.rough);
        let e1i = lut.e1(wi.z, b.rough);
        let eavg = lut.e1_avg(b.rough);
        let favg = conductor_fresnel_avg(b.base, b.f82);
        let ms = (1.0 - e1o) * (1.0 - e1i) / (PI * (1.0 - eavg).max(1e-4));
        let fms = v3(
            favg.x * favg.x * eavg / (1.0 - favg.x * (1.0 - eavg)),
            favg.y * favg.y * eavg / (1.0 - favg.y * (1.0 - eavg)),
            favg.z * favg.z * eavg / (1.0 - favg.z * (1.0 - eavg)),
        );
        f = f + (fr * fs + fms * ms) * (b.metal * under);
        pdf += odds[1] * ps;
    }
    // plastic: Schlick spec over Lambert
    let plastic = (1.0 - b.metal) * (1.0 - b.transmission);
    if plastic > 0.0 && wi.z > 0.0 {
        let f0 = f0_of_ior(b.ior);
        let (fs, ps, m) = spec_reflect(wo, wi, a);
        let es_o = lut.e_spec(f0, mu_o, b.rough);
        let es_i = lut.e_spec(f0, wi.z, b.rough);
        let es_avg = lut.e_spec_avg(f0, b.rough);
        let diffuse = b.base * ((1.0 - es_o) * (1.0 - es_i) / (PI * (1.0 - es_avg).max(1e-4)));
        f = f + (V3::splat(schlick(f0, wo.dot(m)) * fs) + diffuse) * (plastic * under);
        pdf += odds[2] * ps + odds[3] * cosine_pdf(wi);
    }
    // glass
    let glass = (1.0 - b.metal) * b.transmission;
    if glass > 0.0 {
        let (fg, pg) = glass_eval(b, wo, wi, flip, a);
        f = f + fg * (glass * under);
        pdf += odds[4] * pg;
    }
    (f, pdf)
}

/// Rough glass, `wo` on the upper side; `inside`: the upper side is the glass's inside.
fn glass_eval(b: &Bsdf, wo: V3, wi: V3, inside: bool, a: f32) -> (V3, f32) {
    // eta: the index of wi's lower side over wo's upper side
    let eta = if inside { 1.0 / b.ior } else { b.ior };
    if wi.z > 0.0 {
        let (fs, ps, m) = spec_reflect(wo, wi, a);
        let fr = dielectric_fresnel(wo.dot(m), eta);
        return (V3::splat(fr * fs), fr * ps);
    }
    if wi.z == 0.0 {
        return (V3::ZERO, 0.0);
    }
    // the half vector of a refraction, toward the upper side
    let mut m = (wo + wi * eta).norm();
    if m.z < 0.0 {
        m = -m;
    }
    let om = wo.dot(m);
    let im = wi.dot(m);
    if om <= 0.0 || im >= 0.0 {
        return (V3::ZERO, 0.0);
    }
    let fr = dielectric_fresnel(om, eta);
    let d = ggx_d(m, a);
    let den = om + eta * im;
    let den2 = den * den;
    // Walter's eq. 21 for radiance, with eta_o = 1 (relative): the 1/eta^2 of radiance entering
    let ft = (1.0 - fr) * d * g2(wo, wi, a) * om * (-im) / (wo.z * (-wi.z) * den2);
    let jac = eta * eta * (-im) / den2;
    let pdf = (1.0 - fr) * vndf_pdf(wo, m, a) * jac;
    // Walter's form is homogeneous in the indices, so the relative one is exact; the tint once
    // per crossing
    (b.base * ft.max(0.0), pdf)
}

/// A direction toward the light for `wo`, by the lobe the first number picks; its f and pdf as
/// `eval` gives them.
pub fn sample(b: &Bsdf, lut: &Lut, wo: V3, u: [f32; 3]) -> Option<(V3, V3, f32)> {
    let flip = wo.z < 0.0;
    let wl = if flip { -wo } else { wo };
    let odds = lobe_odds(b, lut, wl.z.max(1e-6));
    let mut x = u[0];
    let mut lobe = 0;
    while lobe < 4 && x >= odds[lobe] {
        x -= odds[lobe];
        lobe += 1;
    }
    let wi = match lobe {
        0 => reflect(wl, sample_vndf(wl, alpha(b.coat_rough), u[1], u[2])),
        1 | 2 => reflect(wl, sample_vndf(wl, alpha(b.rough), u[1], u[2])),
        3 => {
            let r = u[1].sqrt();
            let phi = 2.0 * PI * u[2];
            v3(r * phi.cos(), r * phi.sin(), (1.0 - u[1]).max(0.0).sqrt())
        }
        _ => {
            let a = alpha(b.rough);
            let m = sample_vndf(wl, a, u[1], u[2]);
            let eta = if flip { 1.0 / b.ior } else { b.ior };
            let fr = dielectric_fresnel(wl.dot(m), eta);
            // the lobe's own number, rescaled, picks reflection or refraction
            let pick = (x / odds[4].max(1e-9)).clamp(0.0, 1.0);
            if pick < fr {
                reflect(wl, m)
            } else {
                refract(wl, m, eta).unwrap_or_else(|| reflect(wl, m))
            }
        }
    };
    let wi = if flip { -wi } else { wi };
    let (f, pdf) = eval(b, lut, wo, wi);
    (pdf > 0.0 && f.max_elem() > 0.0).then_some((wi, f, pdf))
}

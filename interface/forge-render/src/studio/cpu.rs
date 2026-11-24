//! The path tracer once more on the CPU: `trace.wgsl`'s algorithm in Rust, slow and plain, the
//! reference a GPU picture is checked against (their means agree within Monte Carlo error).

use super::bsdf::{self, Bsdf, Lut};
use super::env::Env;
use super::scene::Built;
use super::v3::{V3, basis, rand, v3};
use super::{Backdrop, Settings, View};
use std::f32::consts::PI;

fn power(a: f32, b: f32) -> f32 {
    let a2 = a * a;
    a2 / (a2 + b * b).max(1e-30)
}

fn concentric(u: f32, v: f32) -> (f32, f32) {
    let (x, y) = (2.0 * u - 1.0, 2.0 * v - 1.0);
    if x == 0.0 && y == 0.0 {
        return (0.0, 0.0);
    }
    let (r, th) = if x.abs() > y.abs() {
        (x, PI / 4.0 * (y / x))
    } else {
        (y, PI / 2.0 - PI / 4.0 * (x / y))
    };
    (r * th.cos(), r * th.sin())
}

pub struct Scene<'a> {
    pub built: &'a Built,
    pub materials: &'a [Bsdf],
    pub lut: &'a Lut,
    pub env: &'a Env,
}

/// One sample of one pixel: radiance and alpha.
pub fn sample(s: &Scene, view: &View, set: &Settings, size: (u32, u32), x: u32, y: u32, index: u32) -> [f32; 4] {
    let (w, h) = size;
    let pixel = y * w + x;
    let mut dim = 0u32;
    let mut next = || {
        let r = rand(pixel, index, dim, set.seed);
        dim += 1;
        r
    };
    let (jx, jy) = (next(), next());
    let sx = 2.0 * (x as f32 + jx) / w as f32 - 1.0;
    let sy = 1.0 - 2.0 * (y as f32 + jy) / h as f32;
    let (lu, lv) = (next(), next());
    let eye = V3::from(view.eye);
    let (fwd, right, up) = (V3::from(view.forward), V3::from(view.right), V3::from(view.up));
    let (mut o, mut d);
    if view.parallel > 0.0 {
        let half_w = view.parallel * w as f32 / h as f32;
        o = eye + right * (sx * half_w) + up * (sy * view.parallel);
        d = fwd;
    } else {
        let ty = (view.fovy * 0.5).tan();
        let tx = ty * w as f32 / h as f32;
        let dir = fwd + right * (sx * tx) + up * (sy * ty);
        o = eye;
        d = dir.norm();
        if view.aperture > 0.0 {
            let focus = eye + dir * view.focus;
            let (a, b) = concentric(lu, lv);
            o = eye + right * (a * view.aperture) + up * (b * view.aperture);
            d = (focus - o).norm();
        }
    }
    let mut radiance = V3::ZERO;
    let mut through = V3::ONE;
    let mut alpha = 1.0;
    let mut last_pdf = 0.0;
    let scale = eye.len().max(1.0);
    let clamp = V3::splat(set.clamp);
    let turn = set.env_turn;
    let env_rad = |d: V3| s.env.radiance(d, turn) * set.env_strength;
    let mut blur: f32 = 0.0;
    for bounce in 0..set.bounces {
        let mut dim = 8 + bounce * 16;
        let mut next = || {
            let r = rand(pixel, index, dim, set.seed);
            dim += 1;
            r
        };
        let mut hit = s.built.trace(o, d, f32::INFINITY);
        let mut on_ground = false;
        let mut t = hit.map_or(f32::INFINITY, |h| h.t);
        if let Some((gz, _)) = set.ground
            && d.z < 0.0
            && o.z > gz
        {
            let tg = (gz - o.z) / d.z;
            if tg < t {
                on_ground = true;
                t = tg;
                hit = None;
            }
        }
        if hit.is_none() && !on_ground {
            if bounce == 0 {
                match set.backdrop {
                    Backdrop::Environment => radiance = radiance + through.times(env_rad(d)),
                    Backdrop::Colour(c) => radiance = radiance + through.times(V3::from(c)),
                    Backdrop::Transparent => alpha = 0.0,
                }
            } else {
                let mis = if last_pdf <= 0.0 { 1.0 } else { power(last_pdf, s.env.pdf(d, turn)) };
                radiance = radiance + (through.times(env_rad(d)) * mis).min(clamp);
            }
            break;
        }
        let p = o + d * t;
        let (mut n, mut ng, mut mat) = match hit {
            Some(h) => {
                let (n, g) = s.built.normals(&h);
                (n, g, s.materials[s.built.instances[h.instance as usize].material as usize])
            }
            None => (v3(0.0, 0.0, 1.0), v3(0.0, 0.0, 1.0), s.materials[set.ground.unwrap().1 as usize]),
        };
        if bounce > 0 {
            mat.rough = mat.rough.max(blur);
            mat.coat_rough = mat.coat_rough.max(blur);
        }
        blur = blur.max(mat.rough.min(0.6) * 0.7);
        let wo_w = -d;
        if mat.transmission <= 0.0 || on_ground {
            if ng.dot(wo_w) < 0.0 {
                ng = -ng;
            }
            if n.dot(ng) < 0.0 {
                n = -n;
            }
        } else if n.dot(ng) < 0.0 {
            n = -n;
        }
        let (tx, ty) = basis(n);
        let local = |w: V3| v3(w.dot(tx), w.dot(ty), w.dot(n));
        let world = |w: V3| tx * w.x + ty * w.y + n * w.z;
        let wo = local(wo_w);
        let eps = 1e-4 * scale.max(p.len());
        let (ld, lrgb, lpdf) = s.env.sample(next(), next(), turn);
        let lrgb = lrgb * set.env_strength;
        let wi_l = local(ld);
        let (f, pdf_b) = bsdf::eval(&mat, s.lut, wo, wi_l);
        if f.max_elem() > 0.0 && lpdf > 0.0 {
            let side = if ld.dot(ng) > 0.0 { 1.0 } else { -1.0 };
            let mut blocked = s.built.trace(p + ng * (eps * side), ld, f32::INFINITY).is_some();
            if !blocked && set.ground.is_some() && ld.z < 0.0 && !on_ground {
                blocked = true;
            }
            if !blocked {
                let mis = power(lpdf, pdf_b);
                let add = through.times(f).times(lrgb) * (wi_l.z.abs() * mis / lpdf);
                radiance = radiance + if bounce > 0 { add.min(clamp) } else { add };
            }
        }
        let Some((wi, f, pdf)) = bsdf::sample(&mat, s.lut, wo, [next(), next(), next()]) else {
            break;
        };
        through = through.times(f) * (wi.z.abs() / pdf);
        last_pdf = pdf;
        let wi_w = world(wi).norm();
        let side = if wi_w.dot(ng) > 0.0 { 1.0 } else { -1.0 };
        o = p + ng * (eps * side);
        d = wi_w;
        if bounce >= 3 {
            let q = through.max_elem().clamp(0.05, 0.95);
            if next() > q {
                break;
            }
            through = through / q;
        }
    }
    [radiance.x, radiance.y, radiance.z, alpha]
}

/// The mean of `spp` samples per pixel, rows from the top.
pub fn render(s: &Scene, view: &View, set: &Settings, size: (u32, u32), spp: u32) -> Vec<[f32; 4]> {
    let (w, h) = size;
    let mut out = vec![[0.0f32; 4]; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f64; 4];
            for k in 0..spp {
                let v = sample(s, view, set, size, x, y, k);
                for c in 0..4 {
                    acc[c] += v[c] as f64;
                }
            }
            out[(y * w + x) as usize] = acc.map(|v| (v / spp as f64) as f32);
        }
    }
    out
}

/// The display's encoding of a mean: exposed, ACES fit, sRGB, 8 bits (as `show.wgsl`).
pub fn to_rgba8(mean: &[[f32; 4]], ev: f32) -> Vec<u8> {
    let k = 2f32.powf(ev);
    let filmic = |x: f32| ((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0.0, 1.0);
    let srgb = |c: f32| if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
    mean.iter()
        .flat_map(|p| {
            let c = |v: f32| (srgb(filmic(v.max(0.0) * k)) * 255.0).round().clamp(0.0, 255.0) as u8;
            [c(p[0]), c(p[1]), c(p[2]), (p[3].clamp(0.0, 1.0) * 255.0).round() as u8]
        })
        .collect()
}

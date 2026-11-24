//! Light from all around: an equirectangular radiance map (z up), from a Radiance `.hdr` file or
//! drawn here as a photo studio, and the tables that sample it in proportion to what it gives
//! (luminance times sin(theta), a marginal over rows and a conditional per row; Pharr, Jakob &
//! Humphreys, PBRT 4th ed., 12.5). The GPU reads `words()`; the CPU mirror samples the same.

use super::v3::{V3, v3};
use std::f32::consts::PI;

#[derive(Clone, Debug)]
pub struct Env {
    pub w: usize,
    pub h: usize,
    /// Linear RGB, row 0 at the zenith.
    pub rgb: Vec<[f32; 3]>,
    pub marginal: Vec<f32>,
    pub conditional: Vec<f32>,
    /// What the map's pixels add up to, by sin(theta): the pdf's normaliser.
    pub total: f32,
}

/// A Radiance RGBE file (Ward 1991), flat or run-length encoded, to linear RGB.
pub fn read_hdr(bytes: &[u8]) -> Result<(usize, usize, Vec<[f32; 3]>), String> {
    let mut at = 0;
    let mut line = || -> Result<String, String> {
        let end = bytes[at..]
            .iter()
            .position(|&b| b == b'\n')
            .ok_or("hdr: header ends early")?;
        let s = String::from_utf8_lossy(&bytes[at..at + end]).into_owned();
        at += end + 1;
        Ok(s)
    };
    let first = line()?;
    if !first.starts_with("#?") {
        return Err("hdr: not a Radiance file".into());
    }
    loop {
        let l = line()?;
        if l.is_empty() {
            break;
        }
        if l.starts_with("FORMAT=") && l != "FORMAT=32-bit_rle_rgbe" {
            return Err(format!("hdr: {l} is not RGBE"));
        }
    }
    let size = line()?;
    let p: Vec<&str> = size.split_whitespace().collect();
    if p.len() != 4 || p[0] != "-Y" || p[2] != "+X" {
        return Err(format!("hdr: orientation {size} not supported"));
    }
    let h: usize = p[1].parse().map_err(|_| "hdr: height")?;
    let w: usize = p[3].parse().map_err(|_| "hdr: width")?;
    let mut rgbe = vec![[0u8; 4]; w * h];
    let mut pos = at;
    let byte = |pos: &mut usize| -> Result<u8, String> {
        let b = *bytes.get(*pos).ok_or("hdr: data ends early")?;
        *pos += 1;
        Ok(b)
    };
    for y in 0..h {
        let row = &mut rgbe[y * w..(y + 1) * w];
        let head = [
            bytes.get(pos).copied(),
            bytes.get(pos + 1).copied(),
            bytes.get(pos + 2).copied(),
        ];
        let rle = (8..0x8000).contains(&w)
            && head == [Some(2), Some(2), head[2]]
            && head[2].is_some_and(|b| b & 0x80 == 0);
        if rle {
            pos += 4;
            for c in 0..4 {
                let mut x = 0;
                while x < w {
                    let n = byte(&mut pos)? as usize;
                    if n > 128 {
                        let v = byte(&mut pos)?;
                        for _ in 0..n - 128 {
                            *row.get_mut(x).ok_or("hdr: run past the row")? = {
                                let mut p = row[x];
                                p[c] = v;
                                p
                            };
                            x += 1;
                        }
                    } else {
                        for _ in 0..n {
                            let v = byte(&mut pos)?;
                            row.get_mut(x).ok_or("hdr: run past the row")?[c] = v;
                            x += 1;
                        }
                    }
                }
            }
        } else {
            for p in row.iter_mut() {
                *p = [byte(&mut pos)?, byte(&mut pos)?, byte(&mut pos)?, byte(&mut pos)?];
            }
        }
    }
    let rgb = rgbe
        .iter()
        .map(|p| {
            if p[3] == 0 {
                return [0.0; 3];
            }
            let f = f32::powi(2.0, p[3] as i32 - 136);
            [p[0] as f32 * f, p[1] as f32 * f, p[2] as f32 * f]
        })
        .collect();
    Ok((w, h, rgb))
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The direction of a map's pixel centre (u, v in 0..1), before the turn.
pub fn dir_of(u: f32, v: f32) -> V3 {
    let phi = 2.0 * PI * u;
    let theta = PI * v;
    let s = theta.sin();
    v3(s * phi.cos(), s * phi.sin(), theta.cos())
}

/// A photo studio drawn as a map: a dim grey room, a large softbox overhead, a key box to the
/// front left, a strip either side, and a floor. `key` scales the boxes.
pub fn studio(w: usize, h: usize, key: f32) -> Vec<[f32; 3]> {
    let mut out = vec![[0.0; 3]; w * h];
    // a box is a cone of light about a direction: its centre, half-width cosines across and up
    let boxes: [(V3, f32, f32, f32); 4] = [
        (v3(0.0, 0.0, 1.0), 0.93, 0.93, 2.0),
        (v3(-0.55, -0.65, 0.52).norm(), 0.95, 0.90, 3.0),
        (v3(0.95, -0.15, 0.25).norm(), 0.99, 0.80, 1.4),
        (v3(-0.20, 0.95, 0.25).norm(), 0.99, 0.80, 0.9),
    ];
    for y in 0..h {
        for x in 0..w {
            let d = dir_of((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
            let wall = 0.10 + 0.06 * smooth(-0.2, 0.8, d.z);
            let floor = 0.05 + 0.04 * smooth(-1.0, -0.05, d.z);
            let mut c = if d.z > 0.0 { wall } else { floor };
            for (axis, across, up, power) in boxes {
                // the box's frame: across is horizontal, up the rest
                let side = if axis.z.abs() > 0.99 {
                    v3(1.0, 0.0, 0.0)
                } else {
                    v3(0.0, 0.0, 1.0).cross(axis).norm()
                };
                let top = axis.cross(side);
                let ca = d.dot(axis);
                if ca <= 0.0 {
                    continue;
                }
                let sx = (d.dot(side) / ca).abs();
                let sy = (d.dot(top) / ca).abs();
                let (hx, hy) = (
                    (1.0 - across * across).sqrt() / across,
                    (1.0 - up * up).sqrt() / up,
                );
                let edge = smooth(1.0, 0.92, (sx / hx).max(sy / hy));
                c += edge * power * key;
            }
            out[y * w + x] = [c, c, c * 1.02];
        }
    }
    out
}

impl Env {
    pub fn new(w: usize, h: usize, rgb: Vec<[f32; 3]>) -> Env {
        // pixel weights: luminance times the solid angle's sin(theta), with a floor so no
        // direction is impossible
        let mut weights = vec![0.0f32; w * h];
        for y in 0..h {
            let s = (PI * (y as f32 + 0.5) / h as f32).sin();
            for x in 0..w {
                let c = rgb[y * w + x];
                let l = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
                weights[y * w + x] = (l.max(0.0) + 1e-4) * s;
            }
        }
        let mut conditional = vec![0.0f32; h * (w + 1)];
        let mut rows = vec![0.0f32; h];
        for y in 0..h {
            let c = &mut conditional[y * (w + 1)..(y + 1) * (w + 1)];
            let mut acc = 0.0f64;
            for x in 0..w {
                acc += weights[y * w + x] as f64;
                c[x + 1] = acc as f32;
            }
            rows[y] = acc as f32;
            let total = acc.max(1e-30) as f32;
            c.iter_mut().for_each(|v| *v /= total);
            c[w] = 1.0;
        }
        let mut marginal = vec![0.0f32; h + 1];
        let mut acc = 0.0f64;
        for y in 0..h {
            acc += rows[y] as f64;
            marginal[y + 1] = acc as f32;
        }
        let total = acc as f32;
        marginal.iter_mut().for_each(|v| *v /= total.max(1e-30));
        marginal[h] = 1.0;
        Env {
            w,
            h,
            rgb,
            marginal,
            conditional,
            total,
        }
    }

    /// A map's pixel for a world direction, the map turned by `turn` about z: column, row.
    pub fn pixel(&self, d: V3, turn: f32) -> (usize, usize) {
        let mut u = (d.y.atan2(d.x) - turn) / (2.0 * PI);
        u -= u.floor();
        let v = d.z.clamp(-1.0, 1.0).acos() / PI;
        (
            ((u * self.w as f32) as usize).min(self.w - 1),
            ((v * self.h as f32) as usize).min(self.h - 1),
        )
    }

    pub fn radiance(&self, d: V3, turn: f32) -> V3 {
        let (x, y) = self.pixel(d, turn);
        V3::from(self.rgb[y * self.w + x])
    }

    /// The density of `sample` choosing `d`, per solid angle.
    pub fn pdf(&self, d: V3, turn: f32) -> f32 {
        let (x, y) = self.pixel(d, turn);
        let row = self.marginal[y + 1] - self.marginal[y];
        let c = &self.conditional[y * (self.w + 1)..];
        let col = c[x + 1] - c[x];
        let s = (PI * (y as f32 + 0.5) / self.h as f32).sin();
        row * col * (self.w * self.h) as f32 / (2.0 * PI * PI * s.max(1e-6))
    }

    /// A direction in proportion to the light: the direction, its radiance, its pdf.
    pub fn sample(&self, u1: f32, u2: f32, turn: f32) -> (V3, V3, f32) {
        let y = find(&self.marginal, u1).min(self.h - 1);
        let c = &self.conditional[y * (self.w + 1)..(y + 1) * (self.w + 1)];
        let x = find(c, u2).min(self.w - 1);
        // within the pixel, where the numbers fall: keeps the map's own resolution
        let fy = (u1 - self.marginal[y]) / (self.marginal[y + 1] - self.marginal[y]).max(1e-30);
        let fx = (u2 - c[x]) / (c[x + 1] - c[x]).max(1e-30);
        let u = (x as f32 + fx.clamp(0.0, 0.999)) / self.w as f32;
        let v = (y as f32 + fy.clamp(0.0, 0.999)) / self.h as f32;
        let d = dir_of(u + turn / (2.0 * PI), v);
        let rgb = V3::from(self.rgb[y * self.w + x]);
        let row = self.marginal[y + 1] - self.marginal[y];
        let col = c[x + 1] - c[x];
        let s = (PI * (y as f32 + 0.5) / self.h as f32).sin();
        (
            d,
            rgb,
            row * col * (self.w * self.h) as f32 / (2.0 * PI * PI * s.max(1e-6)),
        )
    }

    /// The GPU's array: the marginal, the conditionals, then the pixels as RGB triples.
    pub fn words(&self) -> Vec<f32> {
        let mut v = self.marginal.clone();
        v.extend(&self.conditional);
        for p in &self.rgb {
            v.extend(p);
        }
        v
    }
}

/// The last index whose cdf value is at most `u`: the bin `u` falls in.
fn find(cdf: &[f32], u: f32) -> usize {
    let (mut lo, mut hi) = (0usize, cdf.len() - 1);
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if cdf[mid] <= u {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::v3::rand;

    #[test]
    fn sampling_matches_the_pdf_and_the_light() {
        let (w, h) = (64, 32);
        let env = Env::new(w, h, studio(w, h, 1.0));
        let turn = 0.7;
        // the pdf integrates to one over the sphere (uniform estimate)
        let n = 400000;
        let mut s = 0.0f64;
        let mut light = 0.0f64;
        for k in 0..n {
            let z = 1.0 - 2.0 * rand(1, k, 0, 3);
            let phi = 2.0 * PI * rand(1, k, 1, 3);
            let r = (1.0 - z * z).sqrt();
            let d = v3(r * phi.cos(), r * phi.sin(), z);
            s += env.pdf(d, turn) as f64;
            light += env.radiance(d, turn).lum() as f64;
        }
        let total = s / n as f64 * 4.0 * PI as f64;
        assert!((total - 1.0).abs() < 0.02, "{total}");
        // the light's integral by importance sampling agrees with the uniform estimate
        let mut imp = 0.0f64;
        let m = 100000;
        for k in 0..m {
            let (d, rgb, pdf) = env.sample(rand(2, k, 0, 4), rand(2, k, 1, 4), turn);
            assert!(
                (pdf - env.pdf(d, turn)).abs() <= 1e-3 * pdf,
                "{pdf} vs {}",
                env.pdf(d, turn)
            );
            assert!((rgb - env.radiance(d, turn)).len() < 1e-6);
            imp += (rgb.lum() / pdf) as f64;
        }
        let uni = light / n as f64 * 4.0 * PI as f64;
        let imp = imp / m as f64;
        assert!((imp - uni).abs() < 0.02 * uni, "{imp} vs {uni}");
    }

    #[test]
    fn a_radiance_file_reads() {
        // a 2x8 file, flat, then the same run-length encoded (8 wide is the smallest RLE row)
        let mut flat = b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 2 +X 8\n".to_vec();
        for k in 0..16u8 {
            flat.extend([k * 8, 64, 255, 129]);
        }
        let (w, h, px) = read_hdr(&flat).unwrap();
        assert_eq!((w, h), (8, 2));
        assert_eq!(px[1], [8.0 / 128.0, 0.5, 255.0 / 128.0]);
        let mut rle = b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 2 +X 8\n".to_vec();
        for y in 0..2u8 {
            rle.extend([2, 2, 0, 8]);
            // red: literal; green, blue, exponent: runs
            rle.push(8);
            rle.extend((0..8u8).map(|x| (y * 8 + x) * 8));
            rle.extend([128 + 8, 64, 128 + 8, 255, 128 + 8, 129]);
        }
        let (_, _, px2) = read_hdr(&rle).unwrap();
        assert_eq!(px, px2);
    }
}

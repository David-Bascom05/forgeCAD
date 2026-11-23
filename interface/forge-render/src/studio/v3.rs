//! A 3-vector of f32, the GPU's arithmetic on the CPU: the studio's CPU mirror computes what
//! `trace.wgsl` computes, in the same order where it matters.

use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub const fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    pub const ZERO: V3 = v3(0.0, 0.0, 0.0);
    pub const ONE: V3 = v3(1.0, 1.0, 1.0);

    pub const fn splat(s: f32) -> V3 {
        v3(s, s, s)
    }
    pub fn from(a: [f32; 3]) -> V3 {
        v3(a[0], a[1], a[2])
    }
    pub fn arr(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
    pub fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: V3) -> V3 {
        v3(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> V3 {
        self / self.len()
    }
    pub fn mul(self, o: V3) -> V3 {
        v3(self.x * o.x, self.y * o.y, self.z * o.z)
    }
    pub fn min(self, o: V3) -> V3 {
        v3(self.x.min(o.x), self.y.min(o.y), self.z.min(o.z))
    }
    pub fn max(self, o: V3) -> V3 {
        v3(self.x.max(o.x), self.y.max(o.y), self.z.max(o.z))
    }
    pub fn max_elem(self) -> f32 {
        self.x.max(self.y).max(self.z)
    }
    pub fn get(self, k: usize) -> f32 {
        match k {
            0 => self.x,
            1 => self.y,
            _ => self.z,
        }
    }
    pub fn lum(self) -> f32 {
        0.2126 * self.x + 0.7152 * self.y + 0.0722 * self.z
    }
    pub fn lerp(self, o: V3, t: f32) -> V3 {
        self + (o - self) * t
    }
}

impl Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for V3 {
    type Output = V3;
    fn mul(self, s: f32) -> V3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Div<f32> for V3 {
    type Output = V3;
    fn div(self, s: f32) -> V3 {
        v3(self.x / s, self.y / s, self.z / s)
    }
}
impl Neg for V3 {
    type Output = V3;
    fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
}

/// An orthonormal frame about a unit normal (Duff et al. 2017, "Building an Orthonormal Basis,
/// Revisited"): tangent, bitangent.
pub fn basis(n: V3) -> (V3, V3) {
    let s = if n.z >= 0.0 { 1.0 } else { -1.0 };
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    (
        v3(1.0 + s * n.x * n.x * a, s * b, -s * n.x),
        v3(b, s + n.y * n.y * a, -n.y),
    )
}

/// The PCG hash (Jarzynski & Olano 2020, "Hash Functions for GPU Rendering"): the studio's only
/// source of randomness, so a seed fixes a render.
pub fn pcg(v: u32) -> u32 {
    let state = v.wrapping_mul(747796405).wrapping_add(2891336453);
    let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277803737);
    (word >> 22) ^ word
}

/// A uniform number in [0, 1) from a pixel, a sample, and a dimension of the path.
pub fn rand(pixel: u32, sample: u32, dim: u32, seed: u32) -> f32 {
    let h = pcg(pixel ^ pcg(sample ^ pcg(dim ^ pcg(seed))));
    (h >> 8) as f32 * (1.0 / 16777216.0)
}

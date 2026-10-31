use crate::{Point2, Point3, Vec3};

/// FNV-1a over exact bit patterns, for checking that two targets computed the
/// same thing. All NaNs hash the same (their payload bits may legally differ).
#[derive(Clone, Debug)]
pub struct BitHasher(u64);

impl Default for BitHasher {
    fn default() -> Self {
        BitHasher(0xcbf2_9ce4_8422_2325)
    }
}

impl BitHasher {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn u64(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
    pub fn f64(&mut self, v: f64) {
        let bits = if v.is_nan() {
            0x7ff8_0000_0000_0000
        } else {
            v.to_bits()
        };
        self.u64(bits);
    }
    pub fn p3(&mut self, p: Point3) {
        self.f64(p.x);
        self.f64(p.y);
        self.f64(p.z);
    }
    pub fn v3(&mut self, v: Vec3) {
        self.f64(v.x);
        self.f64(v.y);
        self.f64(v.z);
    }
    pub fn p2(&mut self, p: Point2) {
        self.f64(p.x);
        self.f64(p.y);
    }
    pub fn finish(&self) -> u64 {
        self.0
    }
}

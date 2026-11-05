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

/// Two lanes of a multiply-xorshift mix over 64-bit words, 128 bits out: a fast content key for
/// caches (not a checksum against malice). Bytes go in as words, their count after them.
#[derive(Clone, Copy, Debug)]
pub struct Hash128(u64, u64);

impl Default for Hash128 {
    fn default() -> Hash128 {
        Hash128(0x243f_6a88_85a3_08d3, 0x1319_8a2e_0370_7344)
    }
}

impl Hash128 {
    pub fn new() -> Hash128 {
        Hash128::default()
    }
    pub fn word(&mut self, x: u64) {
        self.0 = (self.0 ^ x).wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(31);
        self.1 = (self.1 ^ x.rotate_left(17))
            .wrapping_mul(0xc2b2_ae3d_27d4_eb4f)
            .rotate_left(29);
    }
    pub fn f64(&mut self, v: f64) {
        self.word(v.to_bits());
    }
    pub fn bytes(&mut self, b: &[u8]) {
        let (words, rest) = b.as_chunks::<8>();
        for c in words {
            self.word(u64::from_le_bytes(*c));
        }
        let mut last = [0u8; 8];
        last[..rest.len()].copy_from_slice(rest);
        self.word(u64::from_le_bytes(last));
        self.word(b.len() as u64);
    }
    pub fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
    pub fn u128(&mut self, x: u128) {
        self.word(x as u64);
        self.word((x >> 64) as u64);
    }
    pub fn finish(&self) -> u128 {
        let f = |mut z: u64| {
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        let (a, b) = (
            f(self.0 ^ self.1.rotate_left(7)),
            f(self.1 ^ self.0.rotate_left(13)),
        );
        (a as u128) << 64 | b as u128
    }
}

/// `Hash128` of some bytes.
pub fn hash128(b: &[u8]) -> u128 {
    let mut h = Hash128::new();
    h.bytes(b);
    h.finish()
}

//! Object ids and the two hashes git names objects by: SHA-1 (FIPS 180-4) and SHA-256.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HashKind {
    Sha1,
    Sha256,
}

impl HashKind {
    #[allow(clippy::len_without_is_empty)]
    pub fn len(self) -> usize {
        match self {
            HashKind::Sha1 => 20,
            HashKind::Sha256 => 32,
        }
    }

    pub fn hasher(self) -> Hasher {
        match self {
            HashKind::Sha1 => Hasher::Sha1(Sha1::new()),
            HashKind::Sha256 => Hasher::Sha256(Sha256::new()),
        }
    }

    pub fn null(self) -> Oid {
        Oid {
            b: [0; 32],
            kind: self,
        }
    }

    pub fn of(self, data: &[u8]) -> Oid {
        let mut h = self.hasher();
        h.update(data);
        h.finish()
    }
}

/// An object's id; ordered by its bytes, as packs and trees order them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Oid {
    b: [u8; 32],
    kind: HashKind,
}

impl Oid {
    pub fn from_bytes(kind: HashKind, bytes: &[u8]) -> Option<Oid> {
        if bytes.len() != kind.len() {
            return None;
        }
        let mut b = [0; 32];
        b[..bytes.len()].copy_from_slice(bytes);
        Some(Oid { b, kind })
    }

    pub fn from_hex(kind: HashKind, hex: &str) -> Option<Oid> {
        let h = hex.as_bytes();
        if h.len() != kind.len() * 2 {
            return None;
        }
        let mut b = [0; 32];
        for (i, pair) in h.chunks(2).enumerate() {
            b[i] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
        }
        Some(Oid { b, kind })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.b[..self.kind.len()]
    }

    pub fn kind(&self) -> HashKind {
        self.kind
    }

    pub fn is_null(&self) -> bool {
        self.b.iter().all(|&x| x == 0)
    }

    pub fn hex(&self) -> String {
        let mut s = String::with_capacity(self.kind.len() * 2);
        for x in self.as_bytes() {
            s.push(HEX[(x >> 4) as usize] as char);
            s.push(HEX[(x & 15) as usize] as char);
        }
        s
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

impl fmt::Display for Oid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.hex())
    }
}

impl fmt::Debug for Oid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.hex()[..12])
    }
}

pub enum Hasher {
    Sha1(Sha1),
    Sha256(Sha256),
}

impl Hasher {
    pub fn update(&mut self, data: &[u8]) {
        match self {
            Hasher::Sha1(h) => h.update(data),
            Hasher::Sha256(h) => h.update(data),
        }
    }

    pub fn finish(self) -> Oid {
        match self {
            Hasher::Sha1(h) => Oid::from_bytes(HashKind::Sha1, &h.finish()).expect("20"),
            Hasher::Sha256(h) => Oid::from_bytes(HashKind::Sha256, &h.finish()).expect("32"),
        }
    }
}

/// Buffers input into 64-byte blocks; both hashes share the padding (big-endian bit length).
struct Blocks {
    buf: [u8; 64],
    fill: usize,
    total: u64,
}

impl Blocks {
    fn new() -> Blocks {
        Blocks {
            buf: [0; 64],
            fill: 0,
            total: 0,
        }
    }

    fn update(&mut self, mut data: &[u8], mut block: impl FnMut(&[u8; 64])) {
        self.total = self.total.wrapping_add(data.len() as u64);
        if self.fill > 0 {
            let n = (64 - self.fill).min(data.len());
            self.buf[self.fill..self.fill + n].copy_from_slice(&data[..n]);
            self.fill += n;
            data = &data[n..];
            if self.fill < 64 {
                return;
            }
            block(&self.buf);
            self.fill = 0;
        }
        let (chunks, rest) = data.as_chunks::<64>();
        for c in chunks {
            block(c);
        }
        self.buf[..rest.len()].copy_from_slice(rest);
        self.fill = rest.len();
    }

    fn pad(&mut self, mut block: impl FnMut(&[u8; 64])) {
        let bits = self.total.wrapping_mul(8);
        let mut tail = [0u8; 128];
        tail[..self.fill].copy_from_slice(&self.buf[..self.fill]);
        tail[self.fill] = 0x80;
        let n = if self.fill < 56 { 64 } else { 128 };
        tail[n - 8..n].copy_from_slice(&bits.to_be_bytes());
        for c in tail[..n].as_chunks::<64>().0 {
            block(c);
        }
    }
}

pub struct Sha1 {
    h: [u32; 5],
    blocks: Blocks,
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha1 {
    pub fn new() -> Sha1 {
        Sha1 {
            h: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
            blocks: Blocks::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        let h = &mut self.h;
        self.blocks.update(data, |b| sha1_block(h, b));
    }

    pub fn finish(mut self) -> [u8; 20] {
        let h = &mut self.h;
        self.blocks.pad(|b| sha1_block(h, b));
        let mut out = [0; 20];
        for (i, w) in self.h.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&w.to_be_bytes());
        }
        out
    }
}

fn sha1_block(h: &mut [u32; 5], b: &[u8; 64]) {
    let mut w = [0u32; 80];
    for i in 0..16 {
        w[i] = u32::from_be_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let [mut a, mut bb, mut c, mut d, mut e] = *h;
    macro_rules! round {
        ($range:expr, $f:expr, $k:expr) => {
            for wi in &w[$range] {
                let f: u32 = $f(bb, c, d);
                let t = a
                    .rotate_left(5)
                    .wrapping_add(f)
                    .wrapping_add(e)
                    .wrapping_add($k)
                    .wrapping_add(*wi);
                e = d;
                d = c;
                c = bb.rotate_left(30);
                bb = a;
                a = t;
            }
        };
    }
    round!(0..20, |b: u32, c: u32, d: u32| d ^ (b & (c ^ d)), 0x5A827999u32);
    round!(20..40, |b: u32, c: u32, d: u32| b ^ c ^ d, 0x6ED9EBA1u32);
    round!(
        40..60,
        |b: u32, c: u32, d: u32| (b & c) | (d & (b | c)),
        0x8F1BBCDCu32
    );
    round!(60..80, |b: u32, c: u32, d: u32| b ^ c ^ d, 0xCA62C1D6u32);
    h[0] = h[0].wrapping_add(a);
    h[1] = h[1].wrapping_add(bb);
    h[2] = h[2].wrapping_add(c);
    h[3] = h[3].wrapping_add(d);
    h[4] = h[4].wrapping_add(e);
}

pub struct Sha256 {
    h: [u32; 8],
    blocks: Blocks,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

const K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

impl Sha256 {
    pub fn new() -> Sha256 {
        Sha256 {
            h: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            blocks: Blocks::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        let h = &mut self.h;
        self.blocks.update(data, |b| sha256_block(h, b));
    }

    pub fn finish(mut self) -> [u8; 32] {
        let h = &mut self.h;
        self.blocks.pad(|b| sha256_block(h, b));
        let mut out = [0; 32];
        for (i, w) in self.h.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&w.to_be_bytes());
        }
        out
    }
}

fn sha256_block(h: &mut [u32; 8], b: &[u8; 64]) {
    let mut w = [0u32; 64];
    for i in 0..16 {
        w[i] = u32::from_be_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
    }
    let mut v = *h;
    for i in 0..64 {
        let [a, bb, c, d, e, f, g, hh] = v;
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = hh
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(K256[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & bb) ^ (a & c) ^ (bb & c);
        let t2 = s0.wrapping_add(maj);
        v = [t1.wrapping_add(t2), a, bb, c, d.wrapping_add(t1), e, f, g];
    }
    for (x, y) in h.iter_mut().zip(v) {
        *x = x.wrapping_add(y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_digests() {
        let s1 = |d: &[u8]| HashKind::Sha1.of(d).hex();
        let s2 = |d: &[u8]| HashKind::Sha256.of(d).hex();
        assert_eq!(s1(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(s1(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            s1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
        assert_eq!(
            s2(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            s2(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // a blob, as git names it
        assert_eq!(s1(b"blob 0\0"), "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    }

    #[test]
    fn split_updates_agree() {
        let data: Vec<u8> = (0..1000u32).map(|i| (i * 7 + 3) as u8).collect();
        for kind in [HashKind::Sha1, HashKind::Sha256] {
            let whole = kind.of(&data);
            for cut in [0, 1, 55, 56, 63, 64, 65, 500, 999] {
                let mut h = kind.hasher();
                h.update(&data[..cut]);
                h.update(&data[cut..]);
                assert_eq!(h.finish(), whole);
            }
        }
    }
}

//! The bit-coded values DWG is made of (Open Design Specification for .dwg files, chapter 2):
//! a writer and a reader, most significant bit first, and the 16-bit CRC.

/// A stream of bits, most significant bit of each byte first.
#[derive(Clone, Debug, Default)]
pub struct BitWriter {
    pub bytes: Vec<u8>,
    /// Bits written.
    pub len: usize,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter::default()
    }
    pub fn bit(&mut self, b: bool) {
        if self.len % 8 == 0 {
            self.bytes.push(0);
        }
        if b {
            let k = self.len / 8;
            self.bytes[k] |= 0x80 >> (self.len % 8);
        }
        self.len += 1;
    }
    /// The low `n` bits of `v`, high first.
    pub fn bits(&mut self, n: u32, v: u64) {
        for k in (0..n).rev() {
            self.bit((v >> k) & 1 == 1);
        }
    }
    pub fn raw(&mut self, bytes: &[u8]) {
        if self.len % 8 == 0 {
            self.bytes.extend_from_slice(bytes);
            self.len += 8 * bytes.len();
        } else {
            for &b in bytes {
                self.bits(8, b as u64);
            }
        }
    }
    /// B: one bit.
    pub fn b(&mut self, b: bool) {
        self.bit(b);
    }
    /// BB: two bits.
    pub fn bb(&mut self, v: u8) {
        self.bits(2, v as u64);
    }
    /// RC: a raw byte.
    pub fn rc(&mut self, v: u8) {
        self.raw(&[v]);
    }
    /// RS: a raw short, little-endian.
    pub fn rs(&mut self, v: u16) {
        self.raw(&v.to_le_bytes());
    }
    /// RL: a raw long, little-endian.
    pub fn rl(&mut self, v: u32) {
        self.raw(&v.to_le_bytes());
    }
    /// RD: a raw double.
    pub fn rd(&mut self, v: f64) {
        self.raw(&v.to_le_bytes());
    }
    /// BS: 0 and 256 in two bits, 1..255 in ten, the rest in eighteen.
    pub fn bs(&mut self, v: i16) {
        match v {
            0 => self.bb(2),
            256 => self.bb(3),
            1..=255 => {
                self.bb(1);
                self.rc(v as u8);
            }
            _ => {
                self.bb(0);
                self.raw(&v.to_le_bytes());
            }
        }
    }
    /// BL: 0 in two bits, 1..255 in ten, the rest in thirty-four.
    pub fn bl(&mut self, v: i32) {
        match v {
            0 => self.bb(2),
            1..=255 => {
                self.bb(1);
                self.rc(v as u8);
            }
            _ => {
                self.bb(0);
                self.raw(&v.to_le_bytes());
            }
        }
    }
    /// BD: 0.0 and 1.0 in two bits, the rest in sixty-six.
    pub fn bd(&mut self, v: f64) {
        if v.to_bits() == 0.0f64.to_bits() {
            self.bb(2);
        } else if v == 1.0 {
            self.bb(1);
        } else {
            self.bb(0);
            self.rd(v);
        }
    }
    /// DD: a double as a patch on `default`: nothing when it is the default, its low four bytes
    /// when only they differ, six when the top two agree, else all eight.
    pub fn dd(&mut self, v: f64, default: f64) {
        let (b, d) = (v.to_le_bytes(), default.to_le_bytes());
        if b == d {
            self.bb(0);
        } else if b[4..] == d[4..] {
            self.bb(1);
            self.raw(&b[0..4]);
        } else if b[6..] == d[6..] {
            self.bb(2);
            self.raw(&b[4..6]);
            self.raw(&b[0..4]);
        } else {
            self.bb(3);
            self.rd(v);
        }
    }
    /// H: a handle reference: its code, then the handle's bytes, most significant first.
    pub fn h(&mut self, code: u8, handle: u32) {
        let bytes = handle.to_be_bytes();
        let skip = bytes.iter().take_while(|&&b| b == 0).count();
        let n = 4 - skip;
        self.rc((code << 4) | n as u8);
        self.raw(&bytes[skip..]);
    }
    /// T: a BS length and the bytes (already in the drawing's code page).
    pub fn t(&mut self, s: &[u8]) {
        self.bs(s.len() as i16);
        self.raw(s);
    }
    /// BE: +z in one bit, else 3BD.
    pub fn be(&mut self, v: [f64; 3]) {
        if v == [0.0, 0.0, 1.0] {
            self.b(true);
        } else {
            self.b(false);
            self.bd3(v);
        }
    }
    /// BT: zero in one bit, else BD.
    pub fn bt(&mut self, v: f64) {
        if v.to_bits() == 0 {
            self.b(true);
        } else {
            self.b(false);
            self.bd(v);
        }
    }
    pub fn bd2(&mut self, v: [f64; 2]) {
        self.bd(v[0]);
        self.bd(v[1]);
    }
    pub fn bd3(&mut self, v: [f64; 3]) {
        self.bd(v[0]);
        self.bd(v[1]);
        self.bd(v[2]);
    }
    pub fn rd2(&mut self, v: [f64; 2]) {
        self.rd(v[0]);
        self.rd(v[1]);
    }
    /// Overwrites 32 bits at bit `at` with `v` as an RL.
    pub fn patch_rl(&mut self, at: usize, v: u32) {
        for (k, byte) in v.to_le_bytes().iter().enumerate() {
            for j in 0..8 {
                let pos = at + 8 * k + j;
                let mask = 0x80 >> (pos % 8);
                if byte & (0x80 >> j) != 0 {
                    self.bytes[pos / 8] |= mask;
                } else {
                    self.bytes[pos / 8] &= !mask;
                }
            }
        }
    }
    /// Another stream's bits on the end of this one.
    pub fn append(&mut self, o: &BitWriter) {
        for k in 0..o.len {
            self.bit(o.bytes[k / 8] & (0x80 >> (k % 8)) != 0);
        }
    }
}

/// Reads what `BitWriter` writes.
#[derive(Clone, Debug)]
pub struct BitReader<'a> {
    pub d: &'a [u8],
    /// The next bit.
    pub p: usize,
}

/// Ran off the end, or a code no writer uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bad(pub String);

pub type R<T> = Result<T, Bad>;

pub fn bad<T>(what: impl Into<String>) -> R<T> {
    Err(Bad(what.into()))
}

impl<'a> BitReader<'a> {
    pub fn new(d: &'a [u8], byte: usize) -> BitReader<'a> {
        BitReader { d, p: byte * 8 }
    }
    pub fn bit(&mut self) -> R<bool> {
        let Some(&byte) = self.d.get(self.p / 8) else {
            return bad("past the end");
        };
        let b = byte & (0x80 >> (self.p % 8)) != 0;
        self.p += 1;
        Ok(b)
    }
    pub fn bits(&mut self, n: u32) -> R<u64> {
        let mut v = 0;
        for _ in 0..n {
            v = (v << 1) | self.bit()? as u64;
        }
        Ok(v)
    }
    pub fn raw<const N: usize>(&mut self) -> R<[u8; N]> {
        let mut out = [0; N];
        for b in &mut out {
            *b = self.bits(8)? as u8;
        }
        Ok(out)
    }
    pub fn bytes(&mut self, n: usize) -> R<Vec<u8>> {
        (0..n).map(|_| Ok(self.bits(8)? as u8)).collect()
    }
    pub fn b(&mut self) -> R<bool> {
        self.bit()
    }
    pub fn bb(&mut self) -> R<u8> {
        Ok(self.bits(2)? as u8)
    }
    pub fn rc(&mut self) -> R<u8> {
        Ok(self.bits(8)? as u8)
    }
    pub fn rs(&mut self) -> R<u16> {
        Ok(u16::from_le_bytes(self.raw()?))
    }
    pub fn rl(&mut self) -> R<u32> {
        Ok(u32::from_le_bytes(self.raw()?))
    }
    pub fn rd(&mut self) -> R<f64> {
        Ok(f64::from_le_bytes(self.raw()?))
    }
    pub fn bs(&mut self) -> R<i16> {
        Ok(match self.bb()? {
            0 => i16::from_le_bytes(self.raw()?),
            1 => self.rc()? as i16,
            2 => 0,
            _ => 256,
        })
    }
    pub fn bl(&mut self) -> R<i32> {
        match self.bb()? {
            0 => Ok(i32::from_le_bytes(self.raw()?)),
            1 => Ok(self.rc()? as i32),
            2 => Ok(0),
            _ => bad("BL code 3"),
        }
    }
    pub fn bd(&mut self) -> R<f64> {
        match self.bb()? {
            0 => self.rd(),
            1 => Ok(1.0),
            2 => Ok(0.0),
            _ => bad("BD code 3"),
        }
    }
    pub fn dd(&mut self, default: f64) -> R<f64> {
        let mut b = default.to_le_bytes();
        match self.bb()? {
            0 => {}
            1 => b[0..4].copy_from_slice(&self.raw::<4>()?),
            2 => {
                b[4..6].copy_from_slice(&self.raw::<2>()?);
                b[0..4].copy_from_slice(&self.raw::<4>()?);
            }
            _ => return self.rd(),
        }
        Ok(f64::from_le_bytes(b))
    }
    /// (code, handle)
    pub fn h(&mut self) -> R<(u8, u32)> {
        let code = self.bits(4)? as u8;
        let n = self.bits(4)? as usize;
        if n > 4 {
            return bad("handle longer than 4 bytes");
        }
        let mut v = 0u32;
        for _ in 0..n {
            v = (v << 8) | self.rc()? as u32;
        }
        Ok((code, v))
    }
    pub fn t(&mut self) -> R<Vec<u8>> {
        let n = self.bs()?;
        if n < 0 {
            return bad("negative text length");
        }
        self.bytes(n as usize)
    }
    pub fn be(&mut self) -> R<[f64; 3]> {
        if self.b()? {
            Ok([0.0, 0.0, 1.0])
        } else {
            self.bd3()
        }
    }
    pub fn bt(&mut self) -> R<f64> {
        if self.b()? { Ok(0.0) } else { self.bd() }
    }
    pub fn bd2(&mut self) -> R<[f64; 2]> {
        Ok([self.bd()?, self.bd()?])
    }
    pub fn bd3(&mut self) -> R<[f64; 3]> {
        Ok([self.bd()?, self.bd()?, self.bd()?])
    }
    pub fn rd2(&mut self) -> R<[f64; 2]> {
        Ok([self.rd()?, self.rd()?])
    }
}

/// The 16-bit CRC DWG uses (a CRC-16 with polynomial 0xA001), from `seed`.
pub fn crc16(seed: u16, data: &[u8]) -> u16 {
    let mut crc = seed;
    for &x in data {
        let mut v = (crc ^ x as u16) & 0xFF;
        for _ in 0..8 {
            v = if v & 1 != 0 { (v >> 1) ^ 0xA001 } else { v >> 1 };
        }
        crc = (crc >> 8) ^ v;
    }
    crc
}

/// MS: a modular short, 15 bits a word, low word first: an object's size.
pub fn ms(out: &mut Vec<u8>, mut v: u32) {
    while v >= 0x8000 {
        out.extend_from_slice(&(((v & 0x7FFF) | 0x8000) as u16).to_le_bytes());
        v >>= 15;
    }
    out.extend_from_slice(&(v as u16).to_le_bytes());
}

/// An unsigned modular char: 7 bits a byte, low first (the object map's handle steps).
pub fn umc(out: &mut Vec<u8>, mut v: u32) {
    while v >= 0x80 {
        out.push((v & 0x7F) as u8 | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

/// A signed modular char: as `umc`, the last byte's 0x40 the sign (the object map's offsets).
pub fn mc(out: &mut Vec<u8>, v: i64) {
    let mut a = v.unsigned_abs();
    while a >= 0x40 {
        out.push((a & 0x7F) as u8 | 0x80);
        a >>= 7;
    }
    out.push(a as u8 | if v < 0 { 0x40 } else { 0 });
}

/// Reads an MS at `at`: (value, the byte after it).
pub fn read_ms(d: &[u8], mut at: usize) -> R<(u32, usize)> {
    let (mut v, mut shift) = (0u32, 0);
    loop {
        let Some(w) = d.get(at..at + 2) else {
            return bad("MS past the end");
        };
        let w = u16::from_le_bytes([w[0], w[1]]);
        at += 2;
        v |= ((w & 0x7FFF) as u32) << shift;
        shift += 15;
        if w & 0x8000 == 0 {
            return Ok((v, at));
        }
        if shift > 30 {
            return bad("MS too long");
        }
    }
}

/// Reads a modular char at `at`, signed or not: (value, the byte after it).
pub fn read_mc(d: &[u8], mut at: usize, signed: bool) -> R<(i64, usize)> {
    let (mut v, mut shift) = (0i64, 0);
    loop {
        let Some(&b) = d.get(at) else {
            return bad("MC past the end");
        };
        at += 1;
        if b & 0x80 != 0 {
            v |= ((b & 0x7F) as i64) << shift;
            shift += 7;
            if shift > 35 {
                return bad("MC too long");
            }
        } else if signed {
            v |= ((b & 0x3F) as i64) << shift;
            return Ok((if b & 0x40 != 0 { -v } else { v }, at));
        } else {
            v |= (b as i64) << shift;
            return Ok((v, at));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitcodes_read_back() {
        let mut w = BitWriter::new();
        w.b(true);
        w.bs(0);
        w.bs(256);
        w.bs(15);
        w.bs(-1);
        w.bs(1000);
        w.bl(0);
        w.bl(200);
        w.bl(-5);
        w.bd(0.0);
        w.bd(1.0);
        w.bd(-2.5);
        w.dd(3.0, 3.0);
        w.dd(3.0000001, 3.0);
        w.dd(100.0, 3.0);
        w.h(5, 0x5E7);
        w.h(4, 0);
        w.t(b"Standard");
        w.be([0.0, 0.0, 1.0]);
        w.be([0.0, 1.0, 0.0]);
        w.bt(0.0);
        w.bt(2.0);
        let mut r = BitReader::new(&w.bytes, 0);
        assert!(r.b().unwrap());
        for v in [0, 256, 15, -1, 1000] {
            assert_eq!(r.bs().unwrap(), v);
        }
        for v in [0, 200, -5] {
            assert_eq!(r.bl().unwrap(), v);
        }
        for v in [0.0, 1.0, -2.5] {
            assert_eq!(r.bd().unwrap(), v);
        }
        for v in [3.0, 3.0000001, 100.0] {
            assert_eq!(r.dd(3.0).unwrap(), v);
        }
        assert_eq!(r.h().unwrap(), (5, 0x5E7));
        assert_eq!(r.h().unwrap(), (4, 0));
        assert_eq!(r.t().unwrap(), b"Standard");
        assert_eq!(r.be().unwrap(), [0.0, 0.0, 1.0]);
        assert_eq!(r.be().unwrap(), [0.0, 1.0, 0.0]);
        assert_eq!(r.bt().unwrap(), 0.0);
        assert_eq!(r.bt().unwrap(), 2.0);
        assert_eq!(r.p, w.len);
    }

    #[test]
    fn spec_examples() {
        // 2.13: an entity's layer 5E7 as a hard pointer is 0101.0010.00000101.11100111
        let mut w = BitWriter::new();
        w.h(5, 0x5E7);
        assert_eq!(w.bytes, [0x52, 0x05, 0xE7]);
        // 2.6: 4610 and -1413 as modular chars
        let mut out = vec![];
        mc(&mut out, 4610);
        assert_eq!(out, [0b1000_0010, 0b0010_0100]);
        out.clear();
        mc(&mut out, -1413);
        assert_eq!(out, [0b1000_0101, 0b0100_1011]);
        assert_eq!(read_mc(&out, 0, true).unwrap(), (-1413, 2));
        // 2.7: 4650033 as a modular short
        out.clear();
        ms(&mut out, 4650033);
        assert_eq!(out, [0b0011_0001, 0b1111_0100, 0b1000_1101, 0]);
        assert_eq!(read_ms(&out, 0).unwrap(), (4650033, 4));
        // 2.2: 257, 0, 256, 15, 0 as bitshorts
        let mut w = BitWriter::new();
        for v in [257, 0, 256, 15, 0] {
            w.bs(v);
        }
        let s: String = (0..w.len)
            .map(|k| {
                if w.bytes[k / 8] & (0x80 >> (k % 8)) != 0 {
                    '1'
                } else {
                    '0'
                }
            })
            .collect();
        assert_eq!(s, "0000000001000000011011010000111110");
    }

    #[test]
    fn crc_matches_the_spec_table() {
        // the spec's table starts 0x0000, 0xC0C1, 0xC181, 0x0140: a CRC of one byte from 0
        let one = |b: u8| crc16(0, &[b]);
        assert_eq!([one(0), one(1), one(2), one(3)], [0x0000, 0xC0C1, 0xC181, 0x0140]);
        assert_eq!(one(255), 0x4040);
    }
}

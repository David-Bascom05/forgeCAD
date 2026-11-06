//! R2007 files (chapter 5): a Reed-Solomon coded file header, a page map and a section map in
//! system pages, and each section's data pages, compressed with the R21 LZ77 variant (5.10) and
//! checked with 64-bit CRCs (5.12).

use crate::bits::{R, bad};

/// Where page offsets count from: the meta data and the file header before them.
pub const PAGES_START: u64 = 0x480;

/// `n` bytes at `offset` from `PAGES_START`.
fn at_page(buf: &[u8], offset: u64, n: usize) -> Option<&[u8]> {
    let at = usize::try_from(offset.checked_add(PAGES_START)?).ok()?;
    buf.get(at..at.checked_add(n)?)
}

fn u64_at(b: &[u8], at: usize) -> R<u64> {
    match b.get(at..at + 8) {
        Some(x) => Ok(u64::from_le_bytes(x.try_into().unwrap_or([0; 8]))),
        None => bad("past the end"),
    }
}

// ---- 5.12: 64-bit CRCs ----

const fn crc_table(mirrored: bool) -> [u64; 256] {
    let mut t = [0u64; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = if mirrored { i as u64 } else { (i as u64) << 56 };
        let mut k = 0;
        while k < 8 {
            c = if mirrored {
                if c & 1 != 0 {
                    (c >> 1) ^ 0x95AC_9329_AC4B_C9B5
                } else {
                    c >> 1
                }
            } else if c >> 63 != 0 {
                (c << 1) ^ 0x42F0_E1EB_A9EA_3693
            } else {
                c << 1
            };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
}

/// The normal table (ECMA-182) and the mirrored one (the reflected 0xAD93D23594C935A9).
const NORMAL: [u64; 256] = crc_table(false);
const MIRRORED: [u64; 256] = crc_table(true);

/// The order the CRCs and the page checksum take bytes in: whole 8-byte groups as 6 7 4 5 2 3 0 1,
/// then the rest by its count (the spec's table, with each block taken in its own order).
const TAIL_ORDER: [&[usize]; 8] = [
    &[],
    &[0],
    &[0, 1],
    &[0, 1, 2],
    &[2, 3, 0, 1],
    &[2, 3, 0, 1, 4],
    &[2, 3, 0, 1, 4, 5],
    &[2, 3, 0, 1, 4, 5, 6],
];

fn each_in_order(data: &[u8], mut f: impl FnMut(u8)) {
    let (chunks, rest) = data.as_chunks::<8>();
    for c in chunks {
        for k in [6, 7, 4, 5, 2, 3, 0, 1] {
            f(c[k]);
        }
    }
    for &k in TAIL_ORDER[rest.len()] {
        f(rest[k]);
    }
}

/// The normal 64-bit CRC from `seed` (already updated), inverted at the end.
pub fn crc64(seed: u64, data: &[u8]) -> u64 {
    let mut c = seed;
    each_in_order(data, |b| c = NORMAL[((c >> 56) as u8 ^ b) as usize] ^ (c << 8));
    !c
}

/// The mirrored 64-bit CRC from `seed` (already updated).
pub fn crc64_mirrored(seed: u64, data: &[u8]) -> u64 {
    let mut c = seed;
    each_in_order(data, |b| c = MIRRORED[(c as u8 ^ b) as usize] ^ (c >> 8));
    c
}

/// UpdateSeed1: the seed for a CRC of `len` bytes.
pub fn seed1(seed: u64, len: usize) -> u64 {
    let mut s = seed
        .wrapping_add(len as u64)
        .wrapping_mul(0x343FD)
        .wrapping_add(0x269EC3);
    s |= s.wrapping_mul(0x343FD << 32).wrapping_add(0x269EC3 << 32);
    !s
}

/// UpdateSeed2: the seed for a CRC of `len` bytes (the file header's).
pub fn seed2(seed: u64, len: usize) -> u64 {
    let s = seed
        .wrapping_add(len as u64)
        .wrapping_mul(0x343FD)
        .wrapping_add(0x269EC3);
    !s.wrapping_mul((1 << 32) + 0x343FD)
        .wrapping_add(len as u64 + 0x269EC3)
}

/// A data page's 32-bit checksum (5.4.1).
pub fn checksum(seed: u64, data: &[u8]) -> u32 {
    let seed = seed
        .wrapping_add(data.len() as u64)
        .wrapping_mul(0x343FD)
        .wrapping_add(0x269EC3);
    let (mut s1, mut s2) = (seed as u32 & 0xFFFF, (seed >> 16) as u32 & 0xFFFF);
    for chunk in data.chunks(0x15B0) {
        each_in_order(chunk, |b| {
            s1 += b as u32;
            s2 += s1;
        });
        s1 %= 0xFFF1;
        s2 %= 0xFFF1;
    }
    (s2 << 16) | (s1 & 0xFFFF)
}

/// Where the CRC random encoding (5.11) puts a value's bits: (value bit, bit in the high half),
/// then (value bit, bit in the low half).
const RANDOM_HI: [(u16, u32); 5] = [
    (1, 0x800_0000),
    (2, 0x20_0000),
    (4, 0x8000),
    (8, 0x200),
    (0x10, 0x8),
];
const RANDOM_LO: [(u16, u32); 5] = [
    (0x20, 0x2000_0000),
    (0x40, 0x80_0000),
    (0x80, 0x2_0000),
    (0x100, 0x800),
    (0x200, 0x20),
];

/// The 10 bits a value made by the CRC random encoding (5.11) carries.
pub fn decode_random(v: u64) -> u16 {
    let (hi, lo) = ((v >> 32) as u32, v as u32);
    let hi_bits = RANDOM_HI.iter().filter(|b| hi & b.1 != 0).map(|b| b.0);
    let lo_bits = RANDOM_LO.iter().filter(|b| lo & b.1 != 0).map(|b| b.0);
    hi_bits.chain(lo_bits).fold(0, |r, b| r | b)
}

/// The seed ODA's writer always uses for the CRC random encoding.
pub const ODA_RANDOM_SEED: u64 = 0x79B5_5A16_3141_F35C;

/// The CRC random encoding (5.11): a table from a seed, random values from it, and padding.
pub struct Random {
    table: [u32; 0x270],
    index: usize,
    /// The bytes every padding takes from its start.
    pub padding: Vec<u8>,
}

impl Random {
    pub fn new(seed: u64) -> Random {
        let mut t = [0u32; 0x270];
        t[0] = (seed as u32).wrapping_mul(0x343FD).wrapping_add(0x269EC3);
        t[1] = ((seed >> 32) as u32).wrapping_mul(0x343FD).wrapping_add(0x269EC3);
        let mut v = t[1];
        for (i, e) in t.iter_mut().enumerate().skip(2) {
            v = ((v >> 30) ^ v).wrapping_mul(0x6C07_8965).wrapping_add(i as u32);
            *e = v;
        }
        let padding = t[..0x80].iter().flat_map(|v| v.to_le_bytes()).collect();
        Random {
            table: t,
            index: 0x80,
            padding,
        }
    }
    /// GetNextUInt64.
    pub fn next_u64(&mut self) -> u64 {
        self.index += 2;
        if self.index >= 0x270 {
            self.index = 0;
        }
        self.table[self.index] as u64 | (self.table[self.index + 1] as u64) << 32
    }
    /// `v`'s low 10 bits spread among random ones.
    pub fn encode(&mut self, v: u16) -> u64 {
        let r = self.next_u64();
        let mut lo = r as u32 & 0xDF7D_F7DF;
        let mut hi = (r >> 32) as u32 & 0xF7DF_7DF7;
        for &(bit, m) in &RANDOM_HI {
            if v & bit != 0 {
                hi |= m;
            }
        }
        for &(bit, m) in &RANDOM_LO {
            if v & bit != 0 {
                lo |= m;
            }
        }
        lo as u64 | (hi as u64) << 32
    }
    /// `n` padding bytes.
    fn pad(&self, n: usize) -> impl Iterator<Item = u8> + '_ {
        self.padding.iter().copied().cycle().take(n)
    }
}

/// A 64-bit rotate left by `control`'s low five bits (the check data's Encode).
fn rotate(v: u64, control: u64) -> u64 {
    v.rotate_left((control & 0x1F) as u32)
}

// ---- 5.13: Reed-Solomon ----

/// The data of `blocks` interleaved RS code words (k data bytes each): byte j of block b sits at
/// `blocks * j + b`. Parity is dropped, not checked.
pub fn deinterleave(coded: &[u8], blocks: usize, k: usize) -> R<Vec<u8>> {
    if coded.len() < blocks * 255 {
        return bad("Reed-Solomon blocks past the end");
    }
    let mut out = Vec::with_capacity(blocks * k);
    for b in 0..blocks {
        out.extend((0..k).map(|j| coded[blocks * j + b]));
    }
    Ok(out)
}

/// The RS primitive polynomials (5.13, x^8 implied): system pages and the file header
/// (1, 0, 0, 1, 0, 1, 1, 0), data pages (1, 0, 1, 1, 1, 0, 0, 0).
const SYSTEM_POLY: u16 = 0x169;
const DATA_POLY: u16 = 0x11D;

/// A systematic RS (255, 255 - parity) encoder over GF(2^8), the generator's roots alpha^1 up.
pub struct Rs {
    exp: [u8; 255],
    log: [u8; 256],
    /// The generator polynomial, lowest power first.
    generator: Vec<u8>,
}

impl Rs {
    pub fn new(poly: u16, parity: usize) -> Rs {
        let mut rs = Rs {
            exp: [0; 255],
            log: [0; 256],
            generator: vec![1],
        };
        let mut x: u16 = 1;
        for i in 0..255 {
            rs.exp[i] = x as u8;
            rs.log[x as usize] = i as u8;
            x <<= 1;
            if x & 0x100 != 0 {
                x ^= poly;
            }
        }
        for i in 1..=parity {
            let root = rs.exp[i % 255];
            let mut g = vec![0u8; rs.generator.len() + 1];
            for (j, &c) in rs.generator.iter().enumerate() {
                g[j + 1] ^= c;
                g[j] ^= rs.mul(c, root);
            }
            rs.generator = g;
        }
        rs
    }
    /// (255, 239), for system pages and the file header.
    pub fn system() -> Rs {
        Rs::new(SYSTEM_POLY, 16)
    }
    /// (255, 251), for data pages.
    pub fn data() -> Rs {
        Rs::new(DATA_POLY, 4)
    }
    fn mul(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            return 0;
        }
        self.exp[(self.log[a as usize] as usize + self.log[b as usize] as usize) % 255]
    }
    /// Data bytes per code word.
    pub fn k(&self) -> usize {
        256 - self.generator.len()
    }
    /// One block's parity bytes, its last data byte the highest power.
    pub fn parity(&self, data: &[u8]) -> Vec<u8> {
        let n = self.generator.len() - 1;
        let mut bb = vec![0u8; n];
        for &d in data.iter().rev() {
            let fb = d ^ bb[n - 1];
            for j in (1..n).rev() {
                bb[j] = bb[j - 1] ^ self.mul(self.generator[j], fb);
            }
            bb[0] = self.mul(self.generator[0], fb);
        }
        bb
    }
    /// `data` as interleaved code words, the last block's data filled with `pad`.
    pub fn interleave(&self, data: &[u8], mut pad: impl Iterator<Item = u8>) -> Vec<u8> {
        let k = self.k();
        let blocks = data.len().div_ceil(k);
        let mut out = vec![0u8; blocks * 255];
        for (b, chunk) in data.chunks(k).enumerate() {
            let mut block = chunk.to_vec();
            while block.len() < k {
                block.push(pad.next().unwrap_or(0));
            }
            let parity = self.parity(&block);
            for (j, &x) in block.iter().chain(&parity).enumerate() {
                out[blocks * j + b] = x;
            }
        }
        out
    }
}

// ---- 5.10: decompression ----

/// The literal copy orders (5.10.1): for n bytes, (count, source offset) blocks in output order,
/// each block in its own count's order.
const LITERAL: [&[(usize, usize)]; 33] = [
    &[],
    &[(1, 0)],
    &[(1, 1), (1, 0)],
    &[(1, 2), (1, 1), (1, 0)],
    &[(1, 0), (1, 1), (1, 2), (1, 3)],
    &[(1, 4), (4, 0)],
    &[(1, 5), (4, 1), (1, 0)],
    &[(2, 5), (4, 1), (1, 0)],
    &[(4, 0), (4, 4)],
    &[(1, 8), (8, 0)],
    &[(1, 9), (8, 1), (1, 0)],
    &[(2, 9), (8, 1), (1, 0)],
    &[(4, 8), (8, 0)],
    &[(1, 12), (4, 8), (8, 0)],
    &[(1, 13), (4, 9), (8, 1), (1, 0)],
    &[(2, 13), (4, 9), (8, 1), (1, 0)],
    &[(8, 8), (8, 0)],
    &[(8, 9), (1, 8), (8, 0)],
    &[(1, 17), (16, 1), (1, 0)],
    &[(3, 16), (16, 0)],
    &[(4, 16), (16, 0)],
    &[(1, 20), (4, 16), (16, 0)],
    &[(2, 20), (4, 16), (16, 0)],
    &[(3, 20), (4, 16), (16, 0)],
    &[(8, 16), (16, 0)],
    &[(8, 17), (1, 16), (16, 0)],
    &[(1, 25), (8, 17), (1, 16), (16, 0)],
    &[(2, 25), (8, 17), (1, 16), (16, 0)],
    &[(4, 24), (8, 16), (16, 0)],
    &[(1, 28), (4, 24), (8, 16), (16, 0)],
    &[(2, 28), (4, 24), (8, 16), (16, 0)],
    &[(1, 30), (4, 26), (8, 18), (16, 2), (2, 0)],
    &[(16, 16), (16, 0)],
];

/// Source offsets in output order for a literal block of `n` bytes.
fn literal_order(n: usize, base: usize, out: &mut Vec<usize>) {
    if n == 1 {
        out.push(base);
        return;
    }
    for &(c, s) in LITERAL[n] {
        literal_order(c, base + s, out);
    }
}

/// Source offsets in output order for literal blocks of 0 to 32 bytes.
fn literal_orders() -> Vec<Vec<usize>> {
    (0..33)
        .map(|n| {
            let mut o = vec![];
            if n > 0 {
                literal_order(n, 0, &mut o);
            }
            o
        })
        .collect()
}

/// Undoes 5.10's compression into exactly `size` bytes.
pub fn decompress(src: &[u8], size: usize) -> R<Vec<u8>> {
    let orders = literal_orders();
    let end = src.len();
    let mut out: Vec<u8> = Vec::with_capacity(size);
    let mut i = 0usize;
    let byte = |i: &mut usize| -> R<usize> {
        let Some(&b) = src.get(*i) else {
            return bad("compressed data ends early");
        };
        *i += 1;
        Ok(b as usize)
    };
    // (next opcode, offset, length) from an instruction opcode
    let instruction = |i: &mut usize, op: usize| -> R<(usize, usize, usize)> {
        Ok(match op >> 4 {
            0 => {
                let off = byte(i)?;
                let next = byte(i)?;
                (
                    next,
                    ((next & 0x78) << 5) + 1 + off,
                    (op & 0xF) + 0x13 + ((next >> 3) & 0x10),
                )
            }
            1 => {
                let off = byte(i)?;
                let next = byte(i)?;
                (next, ((next & 0xF8) << 5) + 1 + off, (op & 0xF) + 3)
            }
            2 => {
                let mut off = byte(i)?;
                off |= byte(i)? << 8;
                let mut len = op & 7;
                let next;
                if op & 8 == 0 {
                    next = byte(i)?;
                    len += next & 0xF8;
                } else {
                    off += 1;
                    len += byte(i)? << 3;
                    next = byte(i)?;
                    len += ((next & 0xF8) << 8) + 0x100;
                }
                (next, off, len)
            }
            _ => {
                let next = byte(i)?;
                (next, ((next & 0xF8) << 1) + (op & 0xF) + 1, op >> 4)
            }
        })
    };
    let mut op = byte(&mut i)?;
    let mut length = 0;
    if op >> 4 == 2 {
        i = 3;
        length = byte(&mut i)? & 7;
    }
    while i < end {
        if length == 0 {
            length = op + 8;
            if length == 0x17 {
                let n = byte(&mut i)?;
                length += n;
                if n == 0xFF {
                    loop {
                        let n = byte(&mut i)? | (byte(&mut i)? << 8);
                        length += n;
                        if n != 0xFFFF {
                            break;
                        }
                    }
                }
            }
        }
        let Some(lit) = src.get(i..i + length) else {
            return bad("literal past the end");
        };
        if out.len() + length > size {
            return bad("decompressed past the size");
        }
        for chunk in lit.chunks(32) {
            out.extend(orders[chunk.len()].iter().map(|&k| chunk[k]));
        }
        i += length;
        if i >= end {
            break;
        }
        let first = byte(&mut i)?;
        let (mut next, mut offset, mut count) = instruction(&mut i, first)?;
        loop {
            let Some(from) = out.len().checked_sub(offset).filter(|_| offset > 0 || count == 0) else {
                return bad("compression offset out of range");
            };
            if out.len() + count > size {
                return bad("decompressed past the size");
            }
            for k in 0..count {
                let b = out[from + k];
                out.push(b);
            }
            length = next & 7;
            if length != 0 || i >= end {
                break;
            }
            op = byte(&mut i)?;
            if op >> 4 == 0 {
                break;
            }
            if op >> 4 == 0xF {
                op &= 0xF;
            }
            (next, offset, count) = instruction(&mut i, op)?;
        }
    }
    if out.len() != size {
        return bad(format!("decompressed {} bytes, not {size}", out.len()));
    }
    Ok(out)
}

// ---- 5.2: the file header ----

/// The file header (5.2), decompressed, by field; `raw` keeps its 0x110 bytes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileHeader {
    pub raw: Vec<u8>,
    pub header_size: u64,
    pub file_size: u64,
    pub pages_map_crc_compressed: u64,
    pub pages_map_correction: u64,
    pub pages_map_crc_seed: u64,
    pub pages_map2_offset: u64,
    pub pages_map2_id: u64,
    pub pages_map_offset: u64,
    pub pages_map_id: u64,
    pub header2_offset: u64,
    pub pages_map_size_compressed: u64,
    pub pages_map_size: u64,
    pub pages_amount: u64,
    pub pages_max_id: u64,
    pub pages_map_crc: u64,
    pub sections_amount: u64,
    pub sections_map_crc: u64,
    pub sections_map_size_compressed: u64,
    pub sections_map2_id: u64,
    pub sections_map_id: u64,
    pub sections_map_size: u64,
    pub sections_map_crc_compressed: u64,
    pub sections_map_correction: u64,
    pub sections_map_crc_seed: u64,
    pub stream_version: u64,
    pub crc_seed: u64,
    pub crc_seed_encoded: u64,
    pub random_seed: u64,
    pub crc: u64,
}

/// The file header page's frame around the header: the RS coded prefix and the check data.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeaderFrame {
    /// CRC of the checking sequence, and its first value (5.2.1.4).
    pub sequence_crc: u64,
    pub sequence: u64,
    pub compressed_crc: u64,
    /// Negative when the header is stored uncompressed.
    pub compressed_len: i64,
    /// The five values at 0x3D8: normal CRC, mirrored CRC, random 1, random 2, encoded CRC seed.
    pub check: [u64; 5],
}

pub fn file_header(buf: &[u8]) -> R<(HeaderFrame, FileHeader)> {
    let Some(page) = buf.get(0x80..0x480) else {
        return bad("file header past the end");
    };
    let d = deinterleave(page, 3, 239)?;
    let mut f = HeaderFrame {
        sequence_crc: u64_at(&d, 0)?,
        sequence: u64_at(&d, 8)?,
        compressed_crc: u64_at(&d, 0x10)?,
        compressed_len: u64_at(&d, 0x18)? as i64,
        check: [0; 5],
    };
    for (k, c) in f.check.iter_mut().enumerate() {
        *c = u64_at(page, 0x3D8 + 8 * k)?;
    }
    let seq = [
        f.sequence.to_le_bytes(),
        rotate(f.sequence, f.sequence).to_le_bytes(),
    ]
    .concat();
    if crc64(seed1(0, 16), &seq) != f.sequence_crc {
        return bad("file header checking sequence CRC");
    }
    check_data(&f.check)?;
    let n = usize::try_from(f.compressed_len.unsigned_abs())
        .unwrap_or(usize::MAX)
        .min(d.len());
    let Some(data) = d.get(0x20..0x20 + n) else {
        return bad("file header data past its page");
    };
    if crc64(seed2(0, n), data) != f.compressed_crc {
        return bad("file header compressed CRC");
    }
    let raw = if f.compressed_len > 0 {
        decompress(data, 0x110)?
    } else {
        data.to_vec()
    };
    if raw.len() < 0x110 {
        return bad("file header short");
    }
    let mut zeroed = raw[..0x110].to_vec();
    zeroed[0x108..].fill(0);
    let g = |at: usize| u64_at(&raw, at);
    let h = FileHeader {
        header_size: g(0)?,
        file_size: g(0x08)?,
        pages_map_crc_compressed: g(0x10)?,
        pages_map_correction: g(0x18)?,
        pages_map_crc_seed: g(0x20)?,
        pages_map2_offset: g(0x28)?,
        pages_map2_id: g(0x30)?,
        pages_map_offset: g(0x38)?,
        pages_map_id: g(0x40)?,
        header2_offset: g(0x48)?,
        pages_map_size_compressed: g(0x50)?,
        pages_map_size: g(0x58)?,
        pages_amount: g(0x60)?,
        pages_max_id: g(0x68)?,
        pages_map_crc: g(0x80)?,
        sections_amount: g(0xA0)?,
        sections_map_crc: g(0xA8)?,
        sections_map_size_compressed: g(0xB0)?,
        sections_map2_id: g(0xB8)?,
        sections_map_id: g(0xC0)?,
        sections_map_size: g(0xC8)?,
        sections_map_crc_compressed: g(0xD0)?,
        sections_map_correction: g(0xD8)?,
        sections_map_crc_seed: g(0xE0)?,
        stream_version: g(0xE8)?,
        crc_seed: g(0xF0)?,
        crc_seed_encoded: g(0xF8)?,
        random_seed: g(0x100)?,
        crc: g(0x108)?,
        raw,
    };
    if crc64(seed2(0, 0x110), &zeroed) != h.crc {
        return bad("file header CRC");
    }
    Ok((f, h))
}

/// Checks the check data's two CRCs against its random values (5.2.1.1.5).
fn check_data(c: &[u64; 5]) -> R<()> {
    let [normal, mirrored, r1, r2, _] = *c;
    if check_crcs(r1, r2) != (normal, mirrored) {
        return bad("file header check data CRC");
    }
    Ok(())
}

/// The check data's normal and mirrored CRCs from its two random values.
fn check_crcs(r1: u64, r2: u64) -> (u64, u64) {
    let bytes = |b: [u64; 8]| b.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>();
    let mut b = [0u64; 8];
    b[0] = rotate(r1, r2);
    b[1] = rotate(b[0], b[0]);
    b[2] = rotate(r2, b[1]);
    b[3] = rotate(b[2], b[2]);
    b[4] = rotate(r1, b[3]);
    b[5] = rotate(b[4], b[4]);
    b[6] = rotate(b[5], b[5]);
    b[7] = rotate(b[6], b[6]);
    let normal = crc64(!r2, &bytes(b));
    b[1] = rotate(normal, b[0]);
    b[2] = rotate(r2, b[1]);
    b[3] = rotate(normal, b[2]);
    b[4] = rotate(r1, b[3]);
    b[5] = rotate(normal, b[4]);
    b[6] = rotate(r2, b[5]);
    b[7] = rotate(b[6], b[6]);
    (normal, crc64_mirrored(!r1, &bytes(b)))
}

// ---- 5.3, 5.4: pages ----

/// A system page's data (5.3) at page offset `at`: `copies` RS-coded copies of the (maybe
/// compressed) data, of which the first is used. CRCs are mirrored, seeded with `seed`.
#[allow(clippy::too_many_arguments)]
fn system_page(
    buf: &[u8],
    at: u64,
    size_compressed: u64,
    size: u64,
    copies: u64,
    seed: u64,
    crc_compressed: u64,
    crc: u64,
) -> R<Vec<u8>> {
    if size > 0x100_0000 || size_compressed > size.max(0x100) || copies == 0 || copies > 0x1000 {
        return bad("system page sizes");
    }
    let (n, copies) = (size_compressed as usize, copies as usize);
    let blocks = (n.next_multiple_of(8) * copies).div_ceil(239);
    let Some(coded) = at_page(buf, at, blocks * 255) else {
        return bad("system page past the end");
    };
    let d = deinterleave(coded, blocks, 239)?;
    let data = &d[..n];
    if crc64_mirrored(seed1(seed, n), data) != crc_compressed {
        return bad(format!("system page at {at:#x}: compressed CRC"));
    }
    let out = if n < size as usize {
        decompress(data, size as usize)?
    } else {
        data.to_vec()
    };
    if crc64_mirrored(seed1(seed, out.len()), &out) != crc {
        return bad(format!("system page at {at:#x}: CRC"));
    }
    Ok(out)
}

/// A page map entry: page `id` (negative for a gap) of `size` bytes at `PAGES_START + offset`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Page {
    pub id: i64,
    pub size: u64,
    pub offset: u64,
}

/// One page of a section, as the section map lists it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SectionPage {
    /// Where its data starts in the section.
    pub offset: u64,
    pub size: u64,
    pub id: u64,
    pub size_uncompressed: u64,
    pub size_compressed: u64,
    pub checksum: u64,
    pub crc: u64,
}

/// A section as the section map describes it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Section {
    pub name: String,
    pub size: u64,
    pub max_size: u64,
    pub encryption: u64,
    pub hash: u64,
    pub unknown: u64,
    /// 1: stored plainly, 4: RS (255, 251) interleaved.
    pub encoding: u64,
    pub pages: Vec<SectionPage>,
}

/// The file's container: its header, pages and sections.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub frame: HeaderFrame,
    pub header: FileHeader,
    pub pages: Vec<Page>,
    pub sections: Vec<Section>,
}

impl Layout {
    pub fn page(&self, id: u64) -> Option<&Page> {
        self.pages.iter().find(|p| p.id == id as i64)
    }
    /// The file address of a section's first page (where AcDb:Preview's addresses count from).
    pub fn section_address(&self, name: &str) -> Option<u64> {
        let s = self.sections.iter().find(|s| s.name == name)?;
        Some(PAGES_START + self.page(s.pages.first()?.id)?.offset)
    }
}

pub fn layout(buf: &[u8]) -> R<Layout> {
    let (frame, h) = file_header(buf)?;
    let map = system_page(
        buf,
        h.pages_map_offset,
        h.pages_map_size_compressed,
        h.pages_map_size,
        h.pages_map_correction,
        decode_random(h.pages_map_crc_seed) as u64,
        h.pages_map_crc_compressed,
        h.pages_map_crc,
    )?;
    let mut pages = vec![];
    let mut offset = 0u64;
    for e in map.as_chunks::<16>().0 {
        let size = u64_at(e, 0)?;
        pages.push(Page {
            id: u64_at(e, 8)? as i64,
            size,
            offset,
        });
        offset = offset.saturating_add(size);
    }
    let Some(smap_page) = pages.iter().find(|p| p.id == h.sections_map_id as i64) else {
        return bad("no section map page");
    };
    let smap = system_page(
        buf,
        smap_page.offset,
        h.sections_map_size_compressed,
        h.sections_map_size,
        h.sections_map_correction,
        decode_random(h.sections_map_crc_seed) as u64,
        h.sections_map_crc_compressed,
        h.sections_map_crc,
    )?;
    let mut sections = vec![];
    let mut at = 0;
    while at + 0x40 <= smap.len() {
        let g = |k: usize| u64_at(&smap, at + 8 * k);
        let mut s = Section {
            size: g(0)?,
            max_size: g(1)?,
            encryption: g(2)?,
            hash: g(3)?,
            unknown: g(5)?,
            encoding: g(6)?,
            ..Section::default()
        };
        let name_len = usize::try_from(g(4)?).unwrap_or(usize::MAX);
        let count = g(7)?;
        at += 0x40;
        let Some(name) = smap.get(at..at.saturating_add(name_len)) else {
            return bad("section name past the end");
        };
        let units: Vec<u16> = name
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| u16::from_le_bytes(c))
            .take_while(|&u| u != 0)
            .collect();
        s.name = String::from_utf16_lossy(&units);
        at += name_len;
        for _ in 0..count {
            let g = |k: usize| u64_at(&smap, at + 8 * k);
            s.pages.push(SectionPage {
                offset: g(0)?,
                size: g(1)?,
                id: g(2)?,
                size_uncompressed: g(3)?,
                size_compressed: g(4)?,
                checksum: g(5)?,
                crc: g(6)?,
            });
            at += 56;
        }
        sections.push(s);
    }
    Ok(Layout {
        frame,
        header: h,
        pages,
        sections,
    })
}

/// A data page's bytes (5.4): RS decoded, CRC checked, decompressed and checksummed.
pub fn page_data(buf: &[u8], l: &Layout, s: &Section, p: &SectionPage) -> R<Vec<u8>> {
    let Some(page) = l.page(p.id) else {
        return bad(format!("{}: no page {}", s.name, p.id));
    };
    if p.size_uncompressed > 0x100_0000 || p.size_compressed > p.size_uncompressed.max(0x100) {
        return bad(format!("{}: page {} sizes", s.name, p.id));
    }
    let n = p.size_compressed as usize;
    let data = match s.encoding {
        1 => at_page(buf, page.offset, n).map(|d| d.to_vec()),
        4 => {
            let blocks = n.next_multiple_of(8).div_ceil(251);
            match at_page(buf, page.offset, blocks * 255) {
                Some(coded) => Some(deinterleave(coded, blocks, 251)?[..n].to_vec()),
                None => None,
            }
        }
        e => return bad(format!("{}: page encoding {e}", s.name)),
    };
    let Some(data) = data else {
        return bad(format!("{}: page {} past the end", s.name, p.id));
    };
    if crc64_mirrored(seed1(l.header.crc_seed, n), &data) != p.crc {
        return bad(format!("{}: page {} CRC", s.name, p.id));
    }
    let size = p.size_uncompressed as usize;
    let out = if n < size { decompress(&data, size)? } else { data };
    if checksum(0, &out) as u64 != p.checksum {
        return bad(format!("{}: page {} checksum", s.name, p.id));
    }
    Ok(out)
}

/// A section's bytes: its pages laid at their offsets (pages of zeros aren't stored).
pub fn section_data(buf: &[u8], l: &Layout, name: &str) -> R<Option<Vec<u8>>> {
    let Some(s) = l.sections.iter().find(|s| s.name == name) else {
        return Ok(None);
    };
    if s.encryption == 1 {
        return bad(format!("{name}: encrypted"));
    }
    if s.size > 0x4000_0000 {
        return bad(format!("{name}: size {:#x}", s.size));
    }
    let mut out = vec![0u8; s.size as usize];
    for p in &s.pages {
        let d = page_data(buf, l, s, p)?;
        let start = usize::try_from(p.offset).unwrap_or(usize::MAX).min(out.len());
        let n = d.len().min(out.len() - start);
        out[start..start + n].copy_from_slice(&d[..n]);
    }
    Ok(Some(out))
}

// ---- writing ----

/// What the spec says of each section: (name, hash code, page size, encoding).
const KNOWN: [(&str, u32, u64, u32); 14] = [
    ("AcDb:Security", 0x4A02_04EA, 0xF800, 1),
    ("AcDb:FileDepList", 0x6C42_05CA, 0x100, 1),
    ("AcDb:VBAProject", 0x586E_0544, 0x100, 1),
    ("AcDb:AppInfo", 0x3FA0_043E, 0x300, 1),
    ("AcDb:Preview", 0x40AA_0473, 0x400, 1),
    ("AcDb:SummaryInfo", 0x717A_060F, 0x80, 1),
    ("AcDb:RevHistory", 0x60A2_05B3, 0x1000, 4),
    ("AcDb:AcDbObjects", 0x674C_05A9, 0xF800, 4),
    ("AcDb:ObjFreeSpace", 0x77E2_061F, 0xF800, 4),
    ("AcDb:Template", 0x4A14_04CE, 0x400, 4),
    ("AcDb:Handles", 0x3F6E_0450, 0xF800, 4),
    ("AcDb:Classes", 0x3F54_045F, 0xF800, 4),
    ("AcDb:AuxHeader", 0x54F0_050A, 0x800, 4),
    ("AcDb:Header", 0x32B8_03D9, 0x800, 4),
];

/// The order pages are laid in the file (5.1); sections this doesn't know go before the objects.
const STREAM_ORDER: [&str; 14] = [
    "AcDb:SummaryInfo",
    "AcDb:Preview",
    "AcDb:VBAProject",
    "AcDb:AppInfo",
    "AcDb:FileDepList",
    "AcDb:RevHistory",
    "AcDb:Security",
    "AcDb:AcDbObjects",
    "AcDb:ObjFreeSpace",
    "AcDb:Template",
    "AcDb:Handles",
    "AcDb:Classes",
    "AcDb:AuxHeader",
    "AcDb:Header",
];

/// A section to write and what the section map says of it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Content {
    pub name: String,
    pub data: Vec<u8>,
    pub max_size: u64,
    pub encryption: u64,
    pub hash: u64,
    pub unknown: u64,
    pub encoding: u64,
}

impl Content {
    /// A section as the spec describes it by name (unknown names: RS coded, compressed).
    pub fn new(name: &str, data: Vec<u8>) -> Content {
        let (hash, max, encoding) = KNOWN
            .iter()
            .find(|k| k.0 == name)
            .map_or((0, 0xF800, 4), |k| (k.1, k.2, k.3));
        Content {
            name: name.to_string(),
            data,
            max_size: max,
            hash: hash as u64,
            encoding: encoding as u64,
            ..Content::default()
        }
    }
}

/// GetSystemPageSize (5.3.1): room for the data twice, RS coded, at least 0x400.
fn system_page_size(n: usize) -> usize {
    let size = (n.next_multiple_of(8) * 2).div_ceil(239) * 255;
    if size < 0x400 {
        0x400
    } else {
        size.next_multiple_of(0x20)
    }
}

/// The data a page stores: compressed when that is shorter.
fn pack(data: &[u8]) -> Vec<u8> {
    let c = compress(data);
    if c.len() < data.len() { c } else { data.to_vec() }
}

/// A system page as written (5.3) and what the file header says of it.
struct SystemPage {
    bytes: Vec<u8>,
    size: usize,
    size_compressed: usize,
    correction: usize,
    crc: u64,
    crc_compressed: u64,
}

fn system_page_out(data: &[u8], seed: u64, rs: &Rs, rnd: &Random) -> SystemPage {
    let stored = pack(data);
    let page = system_page_size(data.len());
    let mut aligned = stored.clone();
    aligned.resize(stored.len().next_multiple_of(8).max(8), 0);
    let correction = page / 255 * rs.k() / aligned.len();
    let mut bytes = rs.interleave(&aligned.repeat(correction), rnd.pad(usize::MAX));
    bytes.extend(rnd.pad(page - bytes.len()));
    SystemPage {
        bytes,
        size: data.len(),
        size_compressed: stored.len(),
        correction,
        crc: crc64_mirrored(seed1(seed, data.len()), data),
        crc_compressed: crc64_mirrored(seed1(seed, stored.len()), &stored),
    }
}

/// A data page as written (5.4): its bytes, padded to 0x20, and its section map entry.
fn data_page_out(data: &[u8], encoding: u64, seed: u64, rs: &Rs, rnd: &Random) -> (Vec<u8>, SectionPage) {
    let stored = if encoding == 4 { pack(data) } else { data.to_vec() };
    let mut bytes = stored.clone();
    if encoding == 4 {
        bytes.resize(stored.len().next_multiple_of(8), 0);
        bytes = rs.interleave(&bytes, rnd.pad(usize::MAX));
    }
    bytes.resize(bytes.len().next_multiple_of(0x20), 0);
    let p = SectionPage {
        size_uncompressed: data.len() as u64,
        size_compressed: stored.len() as u64,
        checksum: checksum(0, data) as u64,
        crc: crc64_mirrored(seed1(seed, stored.len()), &stored),
        ..SectionPage::default()
    };
    (bytes, p)
}

/// Writes an R2007 file.
pub fn write(d: &super::Drawing) -> R<Vec<u8>> {
    use super::r2000::{aux_header, write_classes, write_map, write_objects};
    let ver = d.ver();
    let (objs, locs) = write_objects(d, 4)?;
    let mut objects = 0x0DCAu32.to_le_bytes().to_vec();
    objects.extend_from_slice(&objs);
    let mut contents = vec![
        Content::new(
            "AcDb:Header",
            crate::header::write_header(&d.header, ver, d.maint)?,
        ),
        Content::new("AcDb:Classes", write_classes(&d.classes, ver, d.maint)?),
        Content::new("AcDb:Handles", write_map(&locs)),
        Content::new("AcDb:AcDbObjects", objects),
        Content::new("AcDb:AuxHeader", aux_header(d)),
    ];
    for e in d.other.iter().filter(|e| e.name != "AcDb:AuxHeader") {
        let mut data = e.data.clone();
        if e.name == "AcDb:ObjFreeSpace" && data.len() >= 8 {
            data[4..8].copy_from_slice(&(locs.len() as u32).to_le_bytes());
        }
        let mut c = Content::new(&e.name, data);
        if e.hash != 0 {
            // as read from an R2007 file
            (c.hash, c.max_size, c.encoding) = (e.hash as u64, e.max_page as u64, e.encoding as u64);
            (c.encryption, c.unknown) = (e.encrypted as u64, e.unknown as u64);
        }
        contents.push(c);
    }
    container(&d.file_header, d.maint, d.codepage, &contents)
}

/// Lays sections out as an R2007 file, the way ODA's writer does.
pub fn container(file_header: &[u8], maint: u8, codepage: u16, contents: &[Content]) -> R<Vec<u8>> {
    let known = |n: &str| KNOWN.iter().any(|k| k.0 == n);
    let mut contents: Vec<Content> = contents.to_vec();
    for c in &mut contents {
        if c.max_size == 0 || c.encoding != 4 && c.encoding != 1 {
            return bad(format!(
                "{}: page size {:#x}, encoding {}",
                c.name, c.max_size, c.encoding
            ));
        }
        if c.encoding == 1 {
            // stored as one page, not RS coded
            c.max_size = c.max_size.max(c.data.len().next_multiple_of(0x20) as u64);
        }
    }
    let crc_seed = 0u64;
    let mut rnd = Random::new(ODA_RANDOM_SEED);
    let (sys_rs, data_rs) = (Rs::system(), Rs::data());
    // the data pages, in stream order, numbered from 3 as ODA and AutoCAD do
    let mut stream: Vec<&Content> = vec![];
    for n in STREAM_ORDER {
        if n == "AcDb:AcDbObjects" {
            stream.extend(contents.iter().filter(|c| !known(&c.name)));
        }
        stream.extend(contents.iter().filter(|c| c.name == n));
    }
    let count = stream
        .iter()
        .map(|c| c.data.len().div_ceil(c.max_size as usize))
        .sum::<usize>()
        + 4;
    let pm_size = system_page_size(16 * count) as u64;
    let mut pages: Vec<(u64, Vec<u8>)> = vec![];
    let mut section_pages: Vec<(String, SectionPage)> = vec![];
    let mut at = PAGES_START + 2 * pm_size;
    for c in &stream {
        let mut data = std::borrow::Cow::from(&c.data);
        if c.name == "AcDb:Preview" {
            // its entries hold file addresses
            super::r2000::relocate_preview(data.to_mut(), at as i64)?;
        }
        for (k, chunk) in data.chunks(c.max_size as usize).enumerate() {
            let (bytes, mut p) = data_page_out(chunk, c.encoding, crc_seed, &data_rs, &rnd);
            at += bytes.len() as u64;
            p.id = pages.len() as u64 + 3;
            p.offset = k as u64 * c.max_size;
            p.size = c.max_size;
            pages.push((p.id, bytes));
            section_pages.push((c.name.clone(), p));
        }
    }
    // the section map: sections this doesn't know, the known ones in the spec's order, an empty one
    let mut map_order: Vec<Content> = contents.iter().filter(|c| !known(&c.name)).cloned().collect();
    for k in KNOWN {
        map_order.extend(contents.iter().filter(|c| c.name == k.0).cloned());
    }
    map_order.push(Content {
        max_size: 0xF800,
        encoding: 4,
        ..Content::default()
    });
    let mut sm = vec![];
    for c in &map_order {
        let name: Vec<u8> = c.name.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let name_len = if name.is_empty() { 0 } else { name.len() + 2 };
        let ps: Vec<&SectionPage> = section_pages
            .iter()
            .filter(|p| p.0 == c.name)
            .map(|p| &p.1)
            .collect();
        let head = [
            c.data.len() as u64,
            c.max_size,
            c.encryption,
            c.hash,
            name_len as u64,
            c.unknown,
            c.encoding,
            ps.len() as u64,
        ];
        sm.extend(head.iter().flat_map(|v| v.to_le_bytes()));
        sm.extend_from_slice(&name);
        if name_len > 0 {
            sm.extend_from_slice(&[0, 0]);
        }
        for p in ps {
            let v = [
                p.offset,
                p.size,
                p.id,
                p.size_uncompressed,
                p.size_compressed,
                p.checksum,
                p.crc,
            ];
            sm.extend(v.iter().flat_map(|v| v.to_le_bytes()));
        }
    }
    let smap = system_page_out(&sm, crc_seed, &sys_rs, &rnd);
    let sm_id = pages.len() as u64 + 3;
    let pm_id = sm_id + 4;
    // the page map: itself and its copy, the data pages, the section map and its copy
    let mut entries: Vec<(u64, u64)> = vec![(pm_size, pm_id), (pm_size, pm_id + 1)];
    entries.extend(pages.iter().map(|p| (p.1.len() as u64, p.0)));
    entries.push((smap.bytes.len() as u64, sm_id));
    entries.push((smap.bytes.len() as u64, sm_id + 1));
    let pm: Vec<u8> = entries
        .iter()
        .flat_map(|e| [e.0.to_le_bytes(), e.1.to_le_bytes()].concat())
        .collect();
    let pmap = system_page_out(&pm, crc_seed, &sys_rs, &rnd);
    if pmap.bytes.len() as u64 != pm_size {
        return bad("page map size");
    }
    let mut body = vec![];
    body.extend_from_slice(&pmap.bytes);
    body.extend_from_slice(&pmap.bytes);
    let mut first_page_at: Vec<(String, u64)> = vec![];
    for (k, (_, bytes)) in pages.iter().enumerate() {
        let name = &section_pages[k].0;
        if !first_page_at.iter().any(|f| &f.0 == name) {
            first_page_at.push((name.clone(), PAGES_START + body.len() as u64));
        }
        body.extend_from_slice(bytes);
    }
    body.extend_from_slice(&smap.bytes);
    body.extend_from_slice(&smap.bytes);
    // the file header (5.2): random values in the order ODA draws them
    let sm_seed = rnd.encode(crc_seed as u16);
    let pm_seed = rnd.encode(crc_seed as u16);
    let (r1, r2) = (rnd.next_u64(), rnd.next_u64());
    let check_seed = rnd.encode(crc_seed as u16);
    let crc_seed_encoded = rnd.encode(crc_seed as u16);
    let (normal, mirrored) = check_crcs(r1, r2);
    let fields: [u64; 34] = [
        0x70,
        (PAGES_START as usize + body.len() + 0x400) as u64,
        pmap.crc_compressed,
        pmap.correction as u64,
        pm_seed,
        pm_size,
        pm_id + 1,
        0,
        pm_id,
        body.len() as u64,
        pmap.size_compressed as u64,
        pmap.size as u64,
        count as u64,
        pm_id + 1,
        0x20,
        0x40,
        pmap.crc,
        0xF800,
        4,
        1,
        map_order.len() as u64,
        smap.crc,
        smap.size_compressed as u64,
        sm_id + 1,
        sm_id,
        smap.size as u64,
        smap.crc_compressed,
        smap.correction as u64,
        sm_seed,
        0x60100,
        crc_seed,
        crc_seed_encoded,
        ODA_RANDOM_SEED,
        0,
    ];
    let mut raw: Vec<u8> = fields.iter().flat_map(|v| v.to_le_bytes()).collect();
    let crc = crc64(seed2(0, raw.len()), &raw);
    raw[0x108..].copy_from_slice(&crc.to_le_bytes());
    let stored = pack(&raw);
    let stored_len = if stored.len() < raw.len() {
        stored.len() as i64
    } else {
        -(raw.len() as i64)
    };
    let seq = rnd.next_u64();
    let seq_bytes = [seq.to_le_bytes(), rotate(seq, seq).to_le_bytes()].concat();
    let mut block = vec![];
    for v in [
        crc64(seed1(0, 16), &seq_bytes),
        seq,
        crc64(seed2(0, stored.len()), &stored),
    ] {
        block.extend_from_slice(&v.to_le_bytes());
    }
    block.extend_from_slice(&stored_len.to_le_bytes());
    block.extend_from_slice(&stored);
    let n = block.len();
    block.extend(rnd.pad(n.next_multiple_of(8) - n));
    let room = 3 * sys_rs.k();
    let mut pre = block.repeat(room / block.len());
    pre.extend(rnd.pad(room - pre.len()));
    let mut page = sys_rs.interleave(&pre, rnd.pad(0));
    page.extend(rnd.pad(0x3D8 - page.len()));
    for v in [normal, mirrored, r1, r2, check_seed] {
        page.extend_from_slice(&v.to_le_bytes());
    }
    // the meta data (5.2)
    let mut meta = file_header.to_vec();
    meta.resize(0x80, 0);
    meta[0..6].copy_from_slice(b"AC1021");
    meta[0x0B] = maint;
    meta[0x13..0x15].copy_from_slice(&codepage.to_le_bytes());
    let addr = |n: &str| first_page_at.iter().find(|p| p.0 == n).map_or(0, |p| p.1 as u32);
    meta[0x0D..0x11].copy_from_slice(&addr("AcDb:Preview").to_le_bytes());
    meta[0x20..0x24].copy_from_slice(&addr("AcDb:SummaryInfo").to_le_bytes());
    meta[0x24..0x28].copy_from_slice(&addr("AcDb:VBAProject").to_le_bytes());
    meta[0x28..0x2C].copy_from_slice(&0x80u32.to_le_bytes());
    meta[0x2C..0x30].copy_from_slice(&addr("AcDb:AppInfo").to_le_bytes());
    let mut file = meta;
    file.extend_from_slice(&page);
    file.extend_from_slice(&body);
    file.extend_from_slice(&page);
    Ok(file)
}

/// 5.10's compression: greedy matches found through a hash of the next three bytes.
pub fn compress(src: &[u8]) -> Vec<u8> {
    const MAX_CHAIN: usize = 64;
    const MAX_DIST: usize = 0xFFFF;
    const MAX_LEN: usize = 0xFFFF;
    let n = src.len();
    if n == 0 {
        return vec![];
    }
    let hash =
        |p: usize| ((src[p] as usize) << 10 ^ (src[p + 1] as usize) << 5 ^ src[p + 2] as usize) & 0x7FFF;
    let mut head = vec![usize::MAX; 1 << 15];
    let mut prev = vec![usize::MAX; n];
    let insert = |p: usize, head: &mut Vec<usize>, prev: &mut Vec<usize>| {
        if p + 3 <= n {
            let h = hash(p);
            prev[p] = head[h];
            head[h] = p;
        }
    };
    // (literals from, literal count, match length, match distance)
    let mut tokens: Vec<(usize, usize, usize, usize)> = vec![];
    let (mut p, mut lit_start) = (0, 0);
    while p < n {
        let mut best = (0, 0);
        if p + 3 <= n {
            let mut c = head[hash(p)];
            let mut k = 0;
            while c != usize::MAX && k < MAX_CHAIN && p - c <= MAX_DIST {
                let mut l = 0;
                while p + l < n && l < MAX_LEN && src[c + l] == src[p + l] {
                    l += 1;
                }
                if l >= 3 && l > best.0 {
                    best = (l, p - c);
                }
                c = prev[c];
                k += 1;
            }
        }
        if best.0 == 0 {
            insert(p, &mut head, &mut prev);
            p += 1;
            continue;
        }
        tokens.push((lit_start, p - lit_start, best.0, best.1));
        for q in p..p + best.0 {
            insert(q, &mut head, &mut prev);
        }
        p += best.0;
        lit_start = p;
    }
    let orders = literal_orders();
    let mut out = Vec::with_capacity(n / 2 + 16);
    let literals = |out: &mut Vec<u8>, lits: &[u8]| {
        for chunk in lits.chunks(32) {
            let mut placed = vec![0u8; chunk.len()];
            for (k, &s) in orders[chunk.len()].iter().enumerate() {
                placed[s] = chunk[k];
            }
            out.extend_from_slice(&placed);
        }
    };
    // a literal run of 8 or more: its count from an opcode with a clear high nibble
    let long_run = |out: &mut Vec<u8>, k: usize| {
        if k <= 0x16 {
            out.push((k - 8) as u8);
            return;
        }
        out.push(0x0F);
        let mut r = k - 0x17;
        if r < 0xFF {
            out.push(r as u8);
            return;
        }
        out.push(0xFF);
        r -= 0xFF;
        while r >= 0xFFFF {
            out.extend_from_slice(&[0xFF, 0xFF]);
            r -= 0xFFFF;
        }
        out.extend_from_slice(&(r as u16).to_le_bytes());
    };
    // the literals after a match (or at the start): up to 7 counted in the last opcode byte
    let run = |out: &mut Vec<u8>, next: Option<usize>, lits: &[u8]| {
        let k = lits.len();
        match next {
            Some(at) if k <= 7 => out[at] |= k as u8,
            Some(_) => long_run(out, k),
            None if k <= 7 => out.extend_from_slice(&[0x20, 0, 0, k as u8]),
            None => long_run(out, k),
        }
        literals(out, lits);
    };
    let mut next: Option<usize> = None;
    for &(from, k, len, dist) in &tokens {
        // a match straight after another has its case-0 opcode behind 0xF0
        let chained = next.is_some() && k == 0;
        if !chained {
            run(&mut out, next, &src[from..from + k]);
        }
        let off = dist - 1;
        if len <= 14 && off < 0x200 {
            out.push((len << 4 | off & 0xF) as u8);
            out.push(((off >> 4) << 3) as u8);
        } else if len <= 18 && off < 0x2000 {
            out.extend_from_slice(&[0x10 | (len - 3) as u8, off as u8, ((off >> 8) << 3) as u8]);
        } else if (19..=50).contains(&len) && off < 0x1000 {
            let l = len - 19;
            out.push(if chained { 0xF0 } else { 0 } | (l & 0xF) as u8);
            out.extend_from_slice(&[off as u8, ((l >> 4) << 7 | (off >> 8) << 3) as u8]);
        } else if len <= 0xFF {
            out.extend_from_slice(&[
                0x20 | (len & 7) as u8,
                dist as u8,
                (dist >> 8) as u8,
                (len & 0xF8) as u8,
            ]);
        } else {
            let v = len - 0x100;
            out.extend_from_slice(&[0x28 | (v & 7) as u8, off as u8, (off >> 8) as u8, (v >> 3) as u8]);
            out.push(((v >> 11) << 3) as u8);
        }
        next = Some(out.len() - 1);
    }
    if lit_start < n || next.is_none() {
        run(&mut out, next, &src[lit_start..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(src: &[u8]) {
        let c = compress(src);
        assert_eq!(decompress(&c, src.len()).unwrap(), src, "{} bytes", src.len());
    }

    #[test]
    fn rs_parity_of_real_blocks() {
        // knob.dwg (ODA): code word 1 of the file header page (255, 239) and code word 5 of the
        // first AcDbObjects page (255, 251), de-interleaved
        let header: [u8; 255] = [
            0x02, 0x7b, 0x40, 0x67, 0x00, 0x01, 0x56, 0x88, 0x04, 0x21, 0x8a, 0x98, 0x7f, 0x08, 0x4b, 0x77,
            0x08, 0x02, 0x04, 0xd0, 0x1c, 0x57, 0x25, 0x34, 0x44, 0xd7, 0x00, 0x89, 0x60, 0x09, 0x16, 0x77,
            0x08, 0x86, 0x01, 0x15, 0x78, 0x02, 0x72, 0xc0, 0x68, 0x01, 0x6b, 0x7f, 0x02, 0x01, 0x20, 0x68,
            0x01, 0x12, 0x7f, 0x00, 0x87, 0x19, 0x20, 0x7f, 0x01, 0x40, 0x77, 0x00, 0x02, 0xf8, 0x27, 0xdd,
            0x16, 0xd1, 0x1a, 0xde, 0x37, 0x00, 0xa2, 0x60, 0x08, 0x76, 0x31, 0x00, 0x7e, 0x1a, 0x0d, 0x00,
            0x77, 0x10, 0x02, 0x03, 0x03, 0x68, 0x58, 0xfa, 0x8f, 0x1a, 0x75, 0x17, 0xb1, 0x60, 0x08, 0x87,
            0x29, 0x11, 0x7f, 0x0a, 0x07, 0xb4, 0x68, 0x00, 0x01, 0x17, 0x12, 0xc1, 0x5e, 0x28, 0x4e, 0x52,
            0x05, 0xc1, 0x77, 0x08, 0x03, 0x06, 0x55, 0x86, 0x22, 0x78, 0x9c, 0xe1, 0x00, 0x01, 0x40, 0x91,
            0x71, 0x08, 0x66, 0x00, 0x0f, 0x01, 0x14, 0x73, 0x9d, 0x3b, 0x43, 0x6f, 0x28, 0x7d, 0x5c, 0xf3,
            0x41, 0x31, 0x16, 0x5a, 0xb5, 0x79, 0xda, 0x40, 0x7c, 0x55, 0x94, 0x74, 0xd2, 0xa0, 0xaf, 0x34,
            0x19, 0xba, 0x12, 0x04, 0x2b, 0xf6, 0xed, 0x26, 0xf4, 0xbc, 0x8f, 0xeb, 0xb3, 0x85, 0x64, 0x0e,
            0x22, 0x9c, 0x01, 0x15, 0xd1, 0x96, 0xd8, 0x5c, 0x28, 0xa5, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x20, 0x00, 0x00, 0x02, 0x00, 0x70, 0x60, 0x02, 0x7b, 0x40, 0x67, 0x00, 0x01, 0x56, 0x88,
            0x04, 0x21, 0x8a, 0x98, 0x7f, 0x08, 0x4b, 0x77, 0x08, 0x02, 0x04, 0xd0, 0x1c, 0x57, 0x25, 0x34,
            0x44, 0xd7, 0x00, 0x89, 0x60, 0x09, 0x16, 0x77, 0x08, 0x86, 0x01, 0x15, 0x78, 0x02, 0x72, 0xfe,
            0x3e, 0x7b, 0x9b, 0x51, 0x74, 0x48, 0xe5, 0x73, 0xa3, 0x43, 0x93, 0x00, 0x40, 0xbb, 0x35,
        ];
        let data: [u8; 255] = [
            0x03, 0x98, 0x02, 0x14, 0xa6, 0x00, 0xde, 0x38, 0x1b, 0xc8, 0x00, 0xd2, 0x37, 0x11, 0xd8, 0x36,
            0x20, 0x08, 0x50, 0x54, 0xc7, 0x79, 0x00, 0x4e, 0x67, 0x00, 0xdc, 0x00, 0xca, 0x00, 0xa8, 0x03,
            0x41, 0x05, 0x10, 0xbd, 0x2a, 0x2b, 0x49, 0x6d, 0x3c, 0x02, 0x48, 0x08, 0x20, 0x3c, 0x20, 0x4c,
            0x02, 0x04, 0x42, 0x80, 0x48, 0x77, 0x02, 0x50, 0x01, 0x58, 0x03, 0x7f, 0xf1, 0x01, 0xf8, 0x0a,
            0x54, 0x00, 0x1a, 0x66, 0x17, 0x80, 0x00, 0x0d, 0x20, 0x0c, 0xe9, 0x21, 0x31, 0x00, 0x01, 0x0d,
            0xc0, 0x04, 0x00, 0x0b, 0xe0, 0x0b, 0xe0, 0xa0, 0xc5, 0x00, 0x07, 0x01, 0x05, 0x00, 0xe7, 0x7a,
            0xb3, 0x00, 0x4e, 0x58, 0x34, 0x10, 0x55, 0x05, 0x80, 0x0c, 0x3d, 0x32, 0x6b, 0x49, 0x60, 0x29,
            0x06, 0x3c, 0x38, 0x4d, 0xab, 0x01, 0x00, 0x00, 0x05, 0x7c, 0x03, 0x81, 0x01, 0x00, 0x1e, 0x7c,
            0x00, 0x70, 0x08, 0x87, 0x58, 0x9f, 0x09, 0x15, 0x79, 0x02, 0x10, 0x18, 0xf2, 0x1c, 0x05, 0x48,
            0x32, 0x18, 0x10, 0xa8, 0x29, 0x70, 0x10, 0xa8, 0x29, 0x28, 0x10, 0xa4, 0x32, 0x86, 0x24, 0x3f,
            0x81, 0xdc, 0x34, 0xa9, 0xca, 0x34, 0xb1, 0x40, 0x10, 0x07, 0x29, 0xbe, 0x53, 0x00, 0x49, 0x00,
            0x9d, 0x02, 0x28, 0x01, 0x4f, 0x91, 0x50, 0x30, 0x07, 0x01, 0xf0, 0x00, 0x4e, 0x4d, 0xb9, 0xb7,
            0x43, 0x99, 0xab, 0x67, 0x2c, 0x06, 0xc8, 0x08, 0x20, 0x3e, 0x00, 0x3c, 0x00, 0x27, 0xb7, 0x00,
            0x58, 0x3a, 0x90, 0x41, 0x39, 0x02, 0x04, 0x98, 0x08, 0xf1, 0x1b, 0x09, 0x83, 0x1b, 0x79, 0x1a,
            0xaa, 0x22, 0x10, 0x60, 0x20, 0x02, 0xa0, 0x20, 0x0d, 0xc0, 0x0e, 0x80, 0x0d, 0xe0, 0x0d, 0x0c,
            0x66, 0xc8, 0x80, 0xd0, 0x4d, 0x00, 0xa1, 0x09, 0x1a, 0x66, 0xd1, 0x1c, 0x8a, 0xef, 0xd8,
        ];
        assert_eq!(Rs::system().parity(&header[..239]), &header[239..]);
        assert_eq!(Rs::data().parity(&data[..251]), &data[251..]);
        // interleaving puts them back where they were
        let rs = Rs::system();
        let two: Vec<u8> = [&header[..239], &header[..239]].concat();
        let coded = rs.interleave(&two, std::iter::empty());
        assert_eq!(deinterleave(&coded, 2, 239).unwrap(), two);
        assert_eq!(&coded[2 * 239..2 * 239 + 2], &[header[239], header[239]]);
    }

    #[test]
    fn random_values_as_oda_draws_them() {
        // knob.dwg's encoded section map seed, and its check data's two random values
        let mut r = Random::new(ODA_RANDOM_SEED);
        assert_eq!(r.encode(0), 0xe19c782286554091);
        r.encode(0);
        assert_eq!(r.next_u64(), 0x9e970fc87a29feac);
        assert_eq!(r.next_u64(), 0x40edaa6b0f92553a);
        assert_eq!(decode_random(r.encode(0x2A5)), 0x2A5);
        assert_eq!(&r.padding[..4], &[0xaf, 0x34, 0x19, 0xcb]);
    }

    #[test]
    fn compresses_and_reads_back() {
        round(b"a");
        round(b"abcdefg");
        round(b"abcdefgh");
        round(b"abcabcabcabcabcabc");
        round(&[0u8; 5000]);
        round(&[7u8; 0x20000]);
        let mut x: Vec<u8> = (0..70000u32)
            .map(|i| (i.wrapping_mul(2654435761) >> 13) as u8)
            .collect();
        round(&x);
        // runs of literals of every length between repeats of every length, near and far
        let mut y = vec![];
        for k in 0..400usize {
            y.extend((0..k).map(|i| (i * 31 + k * 17) as u8 ^ 0x5A));
            y.extend((0..k % 70 + 3).map(|i| (i * 7) as u8));
            if k % 5 == 0 {
                let from = y.len() / 3;
                let l = (k % 300 + 3).min(y.len() - from);
                y.extend_from_within(from..from + l);
            }
        }
        round(&y);
        x.extend_from_slice(&y);
        x.extend_from_within(100..40000);
        round(&x);
    }

    #[test]
    fn crc_tables_match_the_spec() {
        assert_eq!(NORMAL[1], 0x42f0e1eba9ea3693);
        assert_eq!(NORMAL[255], 0x9afce626ce85b507);
        assert_eq!(MIRRORED[1], 0x7ad870c830358979);
        assert_eq!(MIRRORED[128], 0x95ac9329ac4bc9b5);
        assert_eq!(MIRRORED[255], 0x29b7d047efec8728);
    }

    #[test]
    fn crc_byte_order() {
        // whole groups go 6 7 4 5 2 3 0 1, so swapping the order back gives a plain CRC
        let data: Vec<u8> = (1..=13).collect();
        let mut plain = 0x1234u64;
        for k in [6, 7, 4, 5, 2, 3, 0, 1, 10, 11, 8, 9, 12] {
            plain = NORMAL[((plain >> 56) as u8 ^ data[k]) as usize] ^ (plain << 8);
        }
        assert_eq!(crc64(0x1234, &data), !plain);
    }

    #[test]
    fn crc_of_the_check_sequence() {
        // knob.dwg (ODA): the checking sequence and its CRC, from its file header
        let v = 0x220e6485b3eb8fbcu64;
        let seq = [v.to_le_bytes(), rotate(v, v).to_le_bytes()].concat();
        assert_eq!(crc64(seed1(0, 16), &seq), 0xf426edf62b0412ba);
    }

    #[test]
    fn check_data_of_a_file() {
        // knob.dwg's check data at 0x3D8
        let c = [
            0x4bfb569a69983f20,
            0x5ee0f9e1bbbf3947,
            0x9e970fc87a29feac,
            0x40edaa6b0f92553a,
            0,
        ];
        assert!(check_data(&c).is_ok());
        assert!(check_data(&[c[0] ^ 1, c[1], c[2], c[3], 0]).is_err());
    }

    #[test]
    fn random_decoding() {
        // the encoder's bit spots for 0x3FF, with nothing else set
        assert_eq!(decode_random(0x0820_8208_2082_0820), 0x3FF);
        assert_eq!(decode_random(!0x0820_8208_2082_0820), 0);
    }

    #[test]
    fn deinterleaving() {
        // 3 blocks of 255: block b's byte j at 3j + b
        let mut coded = vec![0u8; 3 * 255];
        for b in 0..3 {
            for j in 0..255 {
                coded[3 * j + b] = (b * 100 + j) as u8;
            }
        }
        let d = deinterleave(&coded, 3, 239).unwrap();
        assert_eq!(d.len(), 3 * 239);
        assert_eq!(&d[..3], &[0, 1, 2]);
        assert_eq!(d[239], 100);
        assert_eq!(d[2 * 239 + 5], 205);
        assert!(deinterleave(&coded[..700], 3, 239).is_err());
    }

    #[test]
    fn literal_orders() {
        let order = |n| {
            let mut o = vec![];
            literal_order(n, 0, &mut o);
            o
        };
        assert_eq!(order(3), [2, 1, 0]);
        assert_eq!(order(4), [0, 1, 2, 3]);
        assert_eq!(order(7), [6, 5, 1, 2, 3, 4, 0]);
        assert_eq!(order(16), [8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7]);
        for n in 1..=32 {
            let mut o = order(n);
            o.sort();
            assert_eq!(o, (0..n).collect::<Vec<_>>(), "{n}");
        }
    }

    #[test]
    fn decompresses_a_hand_made_stream() {
        // literal "\x70\x00" (2, from a 0x2X start), 6 zeros back 1, literal "\x40\x7b",
        // 6 bytes back 8, then a 9-byte literal (opcode 1)
        let src = [
            0x20, 0, 0, 0x02, 0x00, 0x70, 0x60, 0x02, 0x7b, 0x40, 0x67, 0x00, 0x01, 1, 2, 3, 4, 5, 6, 7, 8, 9,
        ];
        let out = decompress(&src, 0x19).unwrap();
        let mut want = vec![0x70, 0, 0, 0, 0, 0, 0, 0, 0x40, 0x7b, 0, 0, 0, 0, 0, 0];
        want.extend([9, 1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(out, want);
        assert!(decompress(&src, 0x18).is_err());
        assert!(decompress(&src[..20], 0x19).is_err());
    }

    #[test]
    fn decompresses_a_long_match_and_literal() {
        // literal of 0x17 + 0x10 = 0x27 bytes (32 then 7), then opcode 0x12: 5 bytes back 1
        let mut src = vec![0x0F, 0x10];
        src.extend(0..0x27u8);
        src.extend([0x12, 0x00, 0x00]);
        let out = decompress(&src, 0x27 + 5).unwrap();
        let mut want: Vec<u8> = (24..32).chain(16..24).chain(8..16).chain(0..8).collect();
        want.extend([32 + 6, 32 + 5, 32 + 1, 32 + 2, 32 + 3, 32 + 4, 32]);
        want.extend([want[0x26]; 5]);
        assert_eq!(out, want);
    }
}

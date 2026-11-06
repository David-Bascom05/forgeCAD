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

/// The 10 bits a value made by the CRC random encoding (5.11) carries.
pub fn decode_random(v: u64) -> u16 {
    let (hi, lo) = ((v >> 32) as u32, v as u32);
    let mut r = 0;
    for (bit, m) in [
        (1, 0x800_0000),
        (2, 0x20_0000),
        (4, 0x8000),
        (8, 0x200),
        (0x10, 0x8),
    ] {
        if hi & m != 0 {
            r |= bit;
        }
    }
    for (bit, m) in [
        (0x20, 0x2000_0000),
        (0x40, 0x80_0000),
        (0x80, 0x2_0000),
        (0x100, 0x800),
        (0x200, 0x20),
    ] {
        if lo & m != 0 {
            r |= bit;
        }
    }
    r
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

/// Undoes 5.10's compression into exactly `size` bytes.
pub fn decompress(src: &[u8], size: usize) -> R<Vec<u8>> {
    let orders: Vec<Vec<usize>> = (0..33)
        .map(|n| {
            let mut o = vec![];
            if n > 0 {
                literal_order(n, 0, &mut o);
            }
            o
        })
        .collect();
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
    if crc64(!r2, &bytes(b)) != normal {
        return bad("file header check data CRC");
    }
    b[1] = rotate(normal, b[0]);
    b[2] = rotate(r2, b[1]);
    b[3] = rotate(normal, b[2]);
    b[4] = rotate(r1, b[3]);
    b[5] = rotate(normal, b[4]);
    b[6] = rotate(r2, b[5]);
    b[7] = rotate(b[6], b[6]);
    if crc64_mirrored(!r1, &bytes(b)) != mirrored {
        return bad("file header check data mirrored CRC");
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

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

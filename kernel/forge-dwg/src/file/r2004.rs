//! R2004 files (chapter 4), also the container of R2010, R2013 and R2018: a page map, a section
//! map, and each section's pages, compressed (4.7) behind masked headers (4.6).

use crate::bits::{R, bad};

/// The 0x6C bytes the file header data is XORed with (and gaps are filled from).
pub fn magic() -> [u8; 0x100] {
    let mut out = [0u8; 0x100];
    let mut seed: u32 = 1;
    for b in out.iter_mut() {
        seed = seed.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
        *b = (seed >> 16) as u8;
    }
    out
}

fn u32_at(b: &[u8], at: usize) -> R<u32> {
    match b.get(at..at + 4) {
        Some(x) => Ok(u32::from_le_bytes([x[0], x[1], x[2], x[3]])),
        None => bad("past the end"),
    }
}

fn u64_at(b: &[u8], at: usize) -> R<u64> {
    match b.get(at..at + 8) {
        Some(x) => Ok(u64::from_le_bytes(x.try_into().unwrap_or([0; 8]))),
        None => bad("past the end"),
    }
}

/// The decrypted file header data (4.1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeaderData {
    pub raw: Vec<u8>,
    pub page_map_id: u32,
    pub page_map_addr: u64,
    pub section_map_id: u32,
    pub page_array_size: u32,
    pub gap_array_size: u32,
    pub last_page_id: u32,
    pub last_page_end: u64,
    pub second_header_addr: u64,
    pub gap_amount: u32,
    pub page_amount: u32,
}

pub fn header_data(buf: &[u8]) -> R<HeaderData> {
    let Some(enc) = buf.get(0x80..0x80 + 0x6C) else {
        return bad("file header past the end");
    };
    let m = magic();
    let raw: Vec<u8> = enc.iter().zip(m.iter()).map(|(a, b)| a ^ b).collect();
    if &raw[0..11] != b"AcFssFcAJMB" {
        return bad("file header ID");
    }
    Ok(HeaderData {
        last_page_id: u32_at(&raw, 0x28)?,
        last_page_end: u64_at(&raw, 0x2C)?,
        second_header_addr: u64_at(&raw, 0x34)?,
        gap_amount: u32_at(&raw, 0x3C)?,
        page_amount: u32_at(&raw, 0x40)?,
        page_map_id: u32_at(&raw, 0x50)?,
        page_map_addr: u64_at(&raw, 0x54)?,
        section_map_id: u32_at(&raw, 0x5C)?,
        page_array_size: u32_at(&raw, 0x60)?,
        gap_array_size: u32_at(&raw, 0x64)?,
        raw,
    })
}

/// The page checksum (4.2).
pub fn checksum(seed: u32, data: &[u8]) -> u32 {
    let (mut s1, mut s2) = (seed & 0xFFFF, seed >> 16);
    for chunk in data.chunks(0x15B0) {
        for &b in chunk {
            s1 += b as u32;
            s2 += s1;
        }
        s1 %= 0xFFF1;
        s2 %= 0xFFF1;
    }
    (s2 << 16) | (s1 & 0xFFFF)
}

/// Undoes 4.7's LZ77 into at most `size` bytes.
pub fn decompress(src: &[u8], size: usize) -> R<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(size);
    let mut p = 0usize;
    let byte = |p: &mut usize| -> R<u8> {
        let Some(&b) = src.get(*p) else {
            return bad("compressed data ends early");
        };
        *p += 1;
        Ok(b)
    };
    let literal_len = |p: &mut usize| -> R<usize> {
        let Some(&b) = src.get(*p) else {
            return bad("compressed data ends early");
        };
        if b & 0xF0 != 0 {
            return Ok(0);
        }
        *p += 1;
        if b != 0 {
            return Ok(b as usize + 3);
        }
        let mut total = 0x0Fusize;
        loop {
            let Some(&b) = src.get(*p) else {
                return bad("compressed data ends early");
            };
            *p += 1;
            if b == 0 {
                total += 0xFF;
            } else {
                return Ok(total + b as usize + 3);
            }
        }
    };
    let long_len = |p: &mut usize| -> R<usize> {
        let b = byte(p)?;
        if b != 0 {
            return Ok(b as usize);
        }
        let mut total = 0xFFusize;
        loop {
            let b = byte(p)?;
            if b == 0 {
                total += 0xFF;
            } else {
                return Ok(total + b as usize);
            }
        }
    };
    let two_byte = |p: &mut usize| -> R<(usize, usize)> {
        let a = byte(p)? as usize;
        let b = byte(p)? as usize;
        Ok(((a >> 2) | (b << 6), a & 3))
    };
    let copy_lit = |p: &mut usize, n: usize, out: &mut Vec<u8>| -> R<()> {
        let Some(s) = src.get(*p..*p + n) else {
            return bad("literal past the end");
        };
        out.extend_from_slice(s);
        *p += n;
        Ok(())
    };
    let n = literal_len(&mut p)?;
    copy_lit(&mut p, n, &mut out)?;
    loop {
        let op = byte(&mut p)?;
        let (count, offset, lit) = match op {
            0x10 => {
                let c = long_len(&mut p)? + 9;
                let (o, l) = two_byte(&mut p)?;
                (c, o + 0x3FFF, l)
            }
            0x11 => break,
            0x12..=0x1F => {
                let (o, l) = two_byte(&mut p)?;
                ((op & 0x0F) as usize + 2, o + 0x3FFF, l)
            }
            0x20 => {
                let c = long_len(&mut p)? + 0x21;
                let (o, l) = two_byte(&mut p)?;
                (c, o, l)
            }
            0x21..=0x3F => {
                let (o, l) = two_byte(&mut p)?;
                (op as usize - 0x1E, o, l)
            }
            0x40..=0xFF => {
                let b = byte(&mut p)? as usize;
                (((op & 0xF0) >> 4) as usize - 1, (b << 2) | ((op & 0x0C) >> 2) as usize, (op & 3) as usize)
            }
            _ => return bad(format!("compression opcode {op:#x}")),
        };
        let Some(from) = out.len().checked_sub(offset + 1) else {
            return bad("compression offset before the start");
        };
        for k in 0..count {
            let b = out[from + k];
            out.push(b);
        }
        let lit = if lit == 0 { literal_len(&mut p)? } else { lit };
        copy_lit(&mut p, lit, &mut out)?;
        if out.len() > size + 0x10000 {
            return bad("decompressed past the size");
        }
    }
    Ok(out)
}

/// A system page (4.3): its type checked, its data decompressed.
fn system_page(buf: &[u8], at: usize, kind: u32) -> R<Vec<u8>> {
    if u32_at(buf, at)? != kind {
        return bad(format!("system page at {at:#x}: type {:#x}", u32_at(buf, at)?));
    }
    let size = u32_at(buf, at + 4)? as usize;
    let comp = u32_at(buf, at + 8)? as usize;
    let Some(data) = buf.get(at + 0x14..at + 0x14 + comp) else {
        return bad("system page past the end");
    };
    if u32_at(buf, at + 12)? == 2 {
        let mut d = decompress(data, size)?;
        d.truncate(size);
        Ok(d)
    } else {
        Ok(data.to_vec())
    }
}

/// A data section: its description and pages.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Section {
    pub name: String,
    pub size: u64,
    pub max_page: u32,
    pub unknown: u32,
    pub compressed: u32,
    pub id: u32,
    pub encrypted: u32,
    /// (page number, data size, start offset)
    pub pages: Vec<(u32, u32, u64)>,
}

/// The file's pages (number to address and size) and its sections.
pub struct Layout {
    pub header: HeaderData,
    pub pages: Vec<(i32, u64, u32)>,
    pub sections: Vec<Section>,
}

pub fn layout(buf: &[u8]) -> R<Layout> {
    let h = header_data(buf)?;
    let map = system_page(buf, h.page_map_addr as usize + 0x100, 0x4163_0E3B)?;
    let mut pages = vec![];
    let mut addr = 0x100u64;
    let mut k = 0;
    while k + 8 <= map.len() {
        let n = u32_at(&map, k)? as i32;
        let size = u32_at(&map, k + 4)?;
        pages.push((n, addr, size));
        addr += size as u64;
        k += 8;
        if n < 0 {
            k += 16;
        }
    }
    let Some(&(_, smap_at, _)) = pages.iter().find(|p| p.0 == h.section_map_id as i32) else {
        return bad("no section map page");
    };
    let smap = system_page(buf, smap_at as usize, 0x4163_003B)?;
    let n = u32_at(&smap, 0)? as usize;
    let mut sections = vec![];
    let mut at = 0x14;
    for _ in 0..n {
        let mut s = Section {
            size: u64_at(&smap, at)?,
            max_page: u32_at(&smap, at + 0x0C)?,
            unknown: u32_at(&smap, at + 0x10)?,
            compressed: u32_at(&smap, at + 0x14)?,
            id: u32_at(&smap, at + 0x18)?,
            encrypted: u32_at(&smap, at + 0x1C)?,
            ..Section::default()
        };
        let count = u32_at(&smap, at + 8)?;
        let name = smap.get(at + 0x20..at + 0x60).unwrap_or(&[]);
        s.name = name.iter().take_while(|&&b| b != 0).map(|&b| b as char).collect();
        at += 0x60;
        for _ in 0..count {
            s.pages.push((u32_at(&smap, at)?, u32_at(&smap, at + 4)?, u64_at(&smap, at + 8)?));
            at += 16;
        }
        sections.push(s);
    }
    Ok(Layout { header: h, pages, sections })
}

/// A data section's bytes: its pages unmasked, decompressed and laid end to end.
pub fn section_data(buf: &[u8], l: &Layout, name: &str) -> R<Option<Vec<u8>>> {
    let Some(s) = l.sections.iter().find(|s| s.name == name) else {
        return Ok(None);
    };
    let mut out = vec![0u8; s.size as usize];
    for &(page, size, start) in &s.pages {
        let Some(&(_, addr, _)) = l.pages.iter().find(|p| p.0 == page as i32) else {
            return bad(format!("{name}: no page {page}"));
        };
        let at = addr as usize;
        let mut hdr = [0u8; 32];
        let Some(raw) = buf.get(at..at + 32) else {
            return bad(format!("{name}: page past the end"));
        };
        let mask = 0x4164_536Bu32 ^ addr as u32;
        for k in 0..8 {
            let v = u32::from_le_bytes([raw[4 * k], raw[4 * k + 1], raw[4 * k + 2], raw[4 * k + 3]]) ^ mask;
            hdr[4 * k..4 * k + 4].copy_from_slice(&v.to_le_bytes());
        }
        if u32_at(&hdr, 0)? != 0x4163_043B {
            return bad(format!("{name}: page {page} type"));
        }
        let Some(data) = buf.get(at + 32..at + 32 + size as usize) else {
            return bad(format!("{name}: page data past the end"));
        };
        let page_bytes = if s.compressed == 2 { decompress(data, s.max_page as usize)? } else { data.to_vec() };
        let start = start as usize;
        let n = page_bytes.len().min(out.len().saturating_sub(start));
        out[start..start + n].copy_from_slice(&page_bytes[..n]);
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_matches_the_spec() {
        let m = magic();
        assert_eq!(&m[0..8], &[0x29, 0x23, 0xBE, 0x84, 0xE1, 0x6C, 0xD6, 0xAE]);
        assert_eq!(&m[0x64..0x68], &[0x44, 0x40, 0x66, 0xD0]);
    }
}

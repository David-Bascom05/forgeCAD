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
            0x11 => break,
            // far: bit 3 is the offset's 0x4000, the low three bits the count less two (0: a long
            // count less nine follows); the spec's 4.7 has these wrong
            0x10..=0x1F => {
                let c = match op & 7 {
                    0 => long_len(&mut p)? + 9,
                    k => k as usize + 2,
                };
                let hi = ((op & 8) as usize) << 11;
                let (o, l) = two_byte(&mut p)?;
                (c, (hi | o) + 0x3FFF, l)
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
                (
                    ((op & 0xF0) >> 4) as usize - 1,
                    (b << 2) | ((op & 0x0C) >> 2) as usize,
                    (op & 3) as usize,
                )
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
    let mut h0 = buf[at..at + 0x14].to_vec();
    h0[0x10..0x14].copy_from_slice(&[0; 4]);
    if checksum(checksum(0, &h0), data) != u32_at(buf, at + 0x10)? {
        return bad(format!("system page at {at:#x}: checksum"));
    }
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
            s.pages
                .push((u32_at(&smap, at)?, u32_at(&smap, at + 4)?, u64_at(&smap, at + 8)?));
            at += 16;
        }
        sections.push(s);
    }
    Ok(Layout {
        header: h,
        pages,
        sections,
    })
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
        // the data's checksum, then the header's from it (the header's own field zeroed)
        let dsum = checksum(0, data);
        let mut h0 = hdr;
        h0[0x18..0x1C].copy_from_slice(&[0; 4]);
        if u32_at(&hdr, 0x1C)? != dsum || u32_at(&hdr, 0x18)? != checksum(dsum, &h0) {
            return bad(format!("{name}: page {page} checksum"));
        }
        let page_bytes = if s.compressed == 2 {
            decompress(data, s.max_page as usize)?
        } else {
            data.to_vec()
        };
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
        // the spec's listing drops a row: its last line is at 0x50
        assert_eq!(&m[0x50..0x58], &[0xAD, 0x4F, 0x14, 0xF2, 0x44, 0x40, 0x66, 0xD0]);
    }
}

/// 4.7's LZ77: greedy matches found through a hash of the next three bytes, planned first and
/// then encoded, so runs of literals the format can't say (1-3 at the start, 18 anywhere) are
/// repaired before any byte is written.
pub fn compress(src: &[u8]) -> Vec<u8> {
    const MAX_CHAIN: usize = 32;
    const NONE: u32 = u32::MAX;
    let n = src.len();
    // a far match (offset past 0x3FFF) of three needs the offset's high bit: 0x11 ends the stream
    let far_ok = |len: usize, d: usize| d - 1 <= 0x3FFF || len >= 4 || (d - 1 - 0x3FFF) & 0x4000 != 0;
    // a literal run before a match (or the stream's end): `first` is the stream's first run
    let run_ok = |k: usize, first: bool| k != 18 && !(first && (1..=3).contains(&k));
    let mut head = vec![NONE; 1 << 15];
    let mut prev = vec![NONE; n];
    let hash =
        |p: usize| ((src[p] as usize) << 10 ^ (src[p + 1] as usize) << 5 ^ src[p + 2] as usize) & 0x7FFF;
    // (literal run start, match start, length, distance)
    let mut plan: Vec<(usize, usize, usize, usize)> = vec![];
    let (mut p, mut lit_start) = (0usize, 0usize);
    let insert = |q: usize, head: &mut Vec<u32>, prev: &mut Vec<u32>| {
        if q + 3 <= n {
            let h = hash(q);
            prev[q] = head[h];
            head[h] = q as u32;
        }
    };
    while p + 3 <= n {
        let (mut best_len, mut best_d) = (0, 0);
        let mut c = head[hash(p)];
        let mut k = 0;
        while c != NONE && k < MAX_CHAIN {
            let d = p - c as usize;
            if d > 0xBFFF {
                break;
            }
            let (a, b) = (&src[c as usize..], &src[p..]);
            let l = a.iter().zip(b).take_while(|(x, y)| x == y).count();
            if l > best_len && l >= 3 && far_ok(l, d) {
                best_len = l;
                best_d = d;
            }
            c = prev[c as usize];
            k += 1;
        }
        if best_len == 0 || !run_ok(p - lit_start, plan.is_empty()) {
            insert(p, &mut head, &mut prev);
            p += 1;
            continue;
        }
        plan.push((lit_start, p, best_len, best_d));
        for q in p..p + best_len {
            insert(q, &mut head, &mut prev);
        }
        p += best_len;
        lit_start = p;
    }
    // the last run: 18 can't be said; shorten the last match, or fold it into the run
    if n - lit_start == 18
        && let Some(&(ls, mp, len, d)) = plan.last()
    {
        if len > 3 && far_ok(len - 1, d) {
            plan.last_mut().unwrap().2 = len - 1;
            lit_start = mp + len - 1;
        } else {
            plan.pop();
            lit_start = ls;
        }
    }
    let mut out = Vec::with_capacity(n / 2 + 16);
    let mut pending: Option<usize> = None;
    let emit_run = |out: &mut Vec<u8>, lits: &[u8], pending: Option<usize>, first: bool| {
        let k = lits.len();
        match pending {
            Some(at) if (1..=3).contains(&k) => out[at] |= k as u8,
            _ if k == 0 => {}
            _ if k <= 17 => out.push((k - 3) as u8),
            _ => {
                let _ = first;
                out.push(0);
                let mut r = k - 3 - 0x0F;
                while r > 0xFF {
                    out.push(0);
                    r -= 0xFF;
                }
                out.push(r as u8);
            }
        }
        out.extend_from_slice(lits);
    };
    for (i, &(ls, mp, len, dist)) in plan.iter().enumerate() {
        emit_run(&mut out, &src[ls..mp], pending, i == 0);
        let off = dist - 1;
        let at;
        if off <= 0x3FF && len <= 14 {
            at = out.len();
            out.push((((len + 1) << 4) | ((off & 3) << 2)) as u8);
            out.push((off >> 2) as u8);
        } else {
            let far = off > 0x3FFF;
            let o = if far { (off - 0x3FFF) & 0x3FFF } else { off };
            if far {
                let hi = if (off - 0x3FFF) & 0x4000 != 0 { 8 } else { 0 };
                if len <= 9 {
                    out.push(0x10 | hi | (len - 2) as u8);
                } else {
                    out.push(0x10 | hi);
                    long_len(&mut out, len - 9);
                }
            } else if len <= 33 {
                out.push((len + 0x1E) as u8);
            } else {
                out.push(0x20);
                long_len(&mut out, len - 0x21);
            }
            at = out.len();
            out.push(((o & 0x3F) << 2) as u8);
            out.push((o >> 6) as u8);
        }
        pending = Some(at);
    }
    // the tail; with no match at all, a run of 1-3 or 18 is padded with zeros (readers cut pages
    // to the section's size)
    let mut tail = src[lit_start..].to_vec();
    if plan.is_empty() && !run_ok(tail.len(), true) {
        tail.resize(if tail.len() == 18 { 19 } else { 4 }, 0);
    }
    emit_run(&mut out, &tail, pending, plan.is_empty());
    out.push(0x11);
    out
}

fn long_len(out: &mut Vec<u8>, v: usize) {
    if v <= 0xFF {
        out.push(v as u8);
        return;
    }
    out.push(0);
    let mut r = v - 0xFF;
    while r > 0xFF {
        out.push(0);
        r -= 0xFF;
    }
    out.push(r as u8);
}

/// CRC-32 (2.14.2) from `seed`.
pub fn crc32(seed: u32, data: &[u8]) -> u32 {
    let mut c = !seed;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                (c >> 1) ^ 0xEDB8_8320
            } else {
                c >> 1
            };
        }
    }
    !c
}

#[cfg(test)]
mod compress_tests {
    use super::*;

    fn round(src: &[u8]) {
        let c = compress(src);
        let d = decompress(&c, src.len()).unwrap();
        assert_eq!(&d[..src.len()], src, "{} bytes", src.len());
        assert!(d.len() <= src.len().max(4));
    }

    #[test]
    fn compresses_and_reads_back() {
        round(b"");
        round(b"a");
        round(b"abc");
        round(b"abcabcabcabcabcabc");
        round(&[0u8; 5000]);
        let mut x: Vec<u8> = (0..70000u32).map(|i| (i * 7 % 251) as u8).collect();
        round(&x);
        // runs of literals of every length between repeats, and far repeats
        let mut y = vec![];
        for k in 0..600usize {
            y.extend((0..k).map(|i| (i * 31 + k * 17) as u8 ^ 0x5A));
            y.extend_from_slice(b"repeat-me-please");
        }
        round(&y);
        x.extend_from_slice(&y);
        x.extend(x.clone().iter().take(40000));
        round(&x);
    }

    #[test]
    fn runs_the_format_cannot_say() {
        // a match, then exactly 18 literals to the end (a real page ended so)
        let mut a = b"abcabcabc".to_vec();
        a.extend((0..18u8).map(|i| i.wrapping_mul(37) ^ 0xA5));
        round(&a);
        // every tail length after a match, and every first run before one
        for k in 0..40usize {
            let mut b = b"xyzxyzxyzxyz".to_vec();
            b.extend((0..k).map(|i| (i * 53 + 7) as u8));
            round(&b);
            let mut c: Vec<u8> = (0..k).map(|i| (i * 71 + 3) as u8 | 0x80).collect();
            c.extend_from_slice(b"qrsqrsqrsqrs");
            round(&c);
        }
    }
}

/// The section map's order: the empty section, sections this doesn't know (as read), then the
/// rest as AutoCAD orders them. Ids count down from the last.
const MAP_ORDER: [&str; 15] = [
    "AcDb:Security",
    "AcDb:FileDepList",
    "AcDb:VBAProject",
    "AcDb:AppInfo",
    "AcDb:Preview",
    "AcDb:SummaryInfo",
    "AcDb:RevHistory",
    "AcDb:AcDbObjects",
    "AcDb:ObjFreeSpace",
    "AcDb:Template",
    "AcDb:Handles",
    "AcDb:Classes",
    "AcDb:AuxHeader",
    "AcDb:Header",
    "",
];

/// The order pages are laid in the file.
const STREAM_ORDER: [&str; 14] = [
    "AcDb:SummaryInfo",
    "AcDb:Preview",
    "AcDb:VBAProject",
    "AcDb:AppInfo",
    "AcDb:FileDepList",
    "AcDb:RevHistory",
    "AcDb:Security",
    "",
    "AcDb:AcDbObjects",
    "AcDb:ObjFreeSpace",
    "AcDb:Template",
    "AcDb:Handles",
    "AcDb:Classes",
    "AcDb:AuxHeader",
];

/// A system page (4.3): header and compressed data, padded to a 0x20 boundary.
fn system_page_bytes(kind: u32, data: &[u8]) -> Vec<u8> {
    let c = compress(data);
    let mut h = vec![];
    for v in [kind, data.len() as u32, c.len() as u32, 2, 0] {
        h.extend_from_slice(&v.to_le_bytes());
    }
    let sum = checksum(checksum(0, &h), &c);
    h[0x10..0x14].copy_from_slice(&sum.to_le_bytes());
    h.extend_from_slice(&c);
    h
}

fn pad_to_0x20(page: &mut Vec<u8>) {
    let m = magic();
    let mut k = 0;
    while !page.len().is_multiple_of(0x20) {
        page.push(m[k % m.len()]);
        k += 1;
    }
}

/// An R2004-container file (R2004, R2010, R2013, R2018).
pub fn write(d: &super::Drawing) -> R<Vec<u8>> {
    use super::r2000::{aux_header, write_classes, write_map, write_objects};
    let ver = d.ver();
    // the sections' contents
    let (objs, locs) = write_objects(d, 4)?;
    let mut objects = 0x0DCAu32.to_le_bytes().to_vec();
    objects.extend_from_slice(&objs);
    let mut contents: Vec<Content> = vec![];
    let core = |name: &str, data: Vec<u8>| (name.to_string(), data, 0x7400u32, 2u32, 0u32, 1u32);
    contents.push(core(
        "AcDb:Header",
        crate::header::write_header(&d.header, ver, d.maint)?,
    ));
    contents.push(core("AcDb:Classes", write_classes(&d.classes, ver, d.maint)?));
    contents.push(core("AcDb:Handles", write_map(&locs)));
    contents.push(core("AcDb:AcDbObjects", objects));
    contents.push(core("AcDb:AuxHeader", aux_header(d)));
    for e in d.other.iter().filter(|e| e.name != "AcDb:AuxHeader") {
        let mut data = e.data.clone();
        if e.name == "AcDb:ObjFreeSpace" && data.len() >= 8 {
            data[4..8].copy_from_slice(&(locs.len() as u32).to_le_bytes());
        }
        let (mp, c, enc, u) = if e.max_page == 0 {
            (0x7400, 2, 0, 1)
        } else {
            (e.max_page, e.compressed, e.encrypted, e.unknown)
        };
        contents.push((e.name.clone(), data, mp, c, enc, u));
    }
    container(d.ver(), d.maint, d.codepage, &d.file_header, &contents)
}

/// (name, data, page size, compressed, encrypted, unknown) for each section.
pub type Content = (String, Vec<u8>, u32, u32, u32, u32);

/// Lays sections out as an R2004-container file.
pub fn container(
    ver: crate::version::Version,
    maint: u8,
    codepage: u16,
    file_header: &[u8],
    contents: &[Content],
) -> R<Vec<u8>> {
    let known = |n: &str| MAP_ORDER.contains(&n);
    // the section map's order and ids
    let mut map_names: Vec<String> = vec!["".into()];
    map_names.extend(contents.iter().filter(|c| !known(&c.0)).map(|c| c.0.clone()));
    for n in MAP_ORDER.iter().filter(|n| !n.is_empty()) {
        if contents.iter().any(|c| c.0 == *n) {
            map_names.push(n.to_string());
        }
    }
    let ids: Vec<(String, u32)> = map_names
        .iter()
        .enumerate()
        .map(|(k, n)| (n.clone(), if k == 0 { 0 } else { (map_names.len() - k) as u32 }))
        .collect();
    // the pages, in stream order
    let mut stream: Vec<String> = vec![];
    for n in STREAM_ORDER {
        if n.is_empty() {
            // sections this doesn't know go before the objects
            stream.extend(contents.iter().filter(|c| !known(&c.0)).map(|c| c.0.clone()));
        } else if contents.iter().any(|c| c.0 == n) {
            stream.push(n.to_string());
        }
    }
    stream.push("AcDb:Header".into());
    let mut file = vec![0u8; 0x100];
    let mut page_map: Vec<(i32, u32)> = vec![];
    // (name, page number, data size, start offset)
    let mut section_pages: Vec<(String, u32, u32, u64)> = vec![];
    let mut first_page_addr: Vec<(String, usize)> = vec![];
    for name in &stream {
        let Some(c) = contents.iter().find(|c| &c.0 == name) else {
            continue;
        };
        let (_, data, max_page, compressed, _, _) = c;
        let id = ids.iter().find(|i| &i.0 == name).map_or(0, |i| i.1);
        for (k, chunk) in data.chunks(*max_page as usize).enumerate() {
            let addr = file.len();
            if k == 0 {
                first_page_addr.push((name.clone(), addr));
            }
            let number = page_map.len() as u32 + 1;
            // every page holds a whole page's bytes, the last padded with zeros (readers expect it)
            let payload = if *compressed == 2 {
                let mut p = chunk.to_vec();
                p.resize(*max_page as usize, 0);
                compress(&p)
            } else {
                let mut p = chunk.to_vec();
                p.resize(*max_page as usize, 0);
                p
            };
            let start = (k * *max_page as usize) as u64;
            let mut hdr = vec![];
            for v in [0x4163_043Bu32, id, payload.len() as u32, 0] {
                hdr.extend_from_slice(&v.to_le_bytes());
            }
            hdr.extend_from_slice(&start.to_le_bytes());
            hdr.extend_from_slice(&[0; 8]);
            let size = (32 + payload.len()).div_ceil(0x20) * 0x20;
            hdr[0x0C..0x10].copy_from_slice(&(size as u32).to_le_bytes());
            let dsum = checksum(0, &payload);
            hdr[0x1C..0x20].copy_from_slice(&dsum.to_le_bytes());
            let hsum = checksum(dsum, &hdr);
            hdr[0x18..0x1C].copy_from_slice(&hsum.to_le_bytes());
            let mask = 0x4164_536Bu32 ^ addr as u32;
            for j in 0..8 {
                let v =
                    u32::from_le_bytes([hdr[4 * j], hdr[4 * j + 1], hdr[4 * j + 2], hdr[4 * j + 3]]) ^ mask;
                file.extend_from_slice(&v.to_le_bytes());
            }
            let mut page = payload.clone();
            page.splice(0..0, std::iter::empty());
            file.extend_from_slice(&page);
            while !(file.len() - addr).is_multiple_of(0x20) {
                file.push(magic()[(file.len() - addr) % 0x100]);
            }
            page_map.push((number as i32, (file.len() - addr) as u32));
            section_pages.push((name.clone(), number, payload.len() as u32, start));
        }
    }
    let data_pages = page_map.len() as u32;
    let smap_id = data_pages + 2;
    let pmap_id = data_pages + 4;
    // the section map
    let mut sm = vec![];
    let n = map_names.len() as u32;
    for v in [n, 2, 0x7400, 0, n] {
        sm.extend_from_slice(&v.to_le_bytes());
    }
    for name in &map_names {
        let (size, mp, comp, enc, unk) = match contents.iter().find(|c| &c.0 == name) {
            Some(c) => (c.1.len() as u64, c.2, c.3, c.4, c.5),
            None => (0, 0x7400, 2, 0, 1),
        };
        let pages: Vec<_> = section_pages.iter().filter(|p| &p.0 == name).collect();
        sm.extend_from_slice(&size.to_le_bytes());
        sm.extend_from_slice(&(pages.len() as u32).to_le_bytes());
        sm.extend_from_slice(&mp.to_le_bytes());
        sm.extend_from_slice(&unk.to_le_bytes());
        sm.extend_from_slice(&comp.to_le_bytes());
        sm.extend_from_slice(&ids.iter().find(|i| &i.0 == name).map_or(0, |i| i.1).to_le_bytes());
        sm.extend_from_slice(&enc.to_le_bytes());
        let mut nm = [0u8; 64];
        nm[..name.len().min(63)].copy_from_slice(&name.as_bytes()[..name.len().min(63)]);
        sm.extend_from_slice(&nm);
        for p in pages {
            sm.extend_from_slice(&p.1.to_le_bytes());
            sm.extend_from_slice(&p.2.to_le_bytes());
            sm.extend_from_slice(&p.3.to_le_bytes());
        }
    }
    let smap_at = file.len();
    let mut smap = system_page_bytes(0x4163_003B, &sm);
    pad_to_0x20(&mut smap);
    file.extend_from_slice(&smap);
    page_map.push((smap_id as i32, smap.len() as u32));
    let _ = smap_at;
    // the page map lists itself: settle its size first
    let pmap_at = file.len();
    let mut size = 0x20u32;
    let pmap = loop {
        let mut pm = vec![];
        for &(num, sz) in page_map.iter().chain(std::iter::once(&(pmap_id as i32, size))) {
            pm.extend_from_slice(&num.to_le_bytes());
            pm.extend_from_slice(&sz.to_le_bytes());
        }
        let mut page = system_page_bytes(0x4163_0E3B, &pm);
        pad_to_0x20(&mut page);
        if page.len() as u32 <= size {
            page.resize(size as usize, 0);
            break page;
        }
        size = page.len() as u32;
    };
    file.extend_from_slice(&pmap);
    let second_at = file.len();
    // the file header data
    let mut hd = vec![0u8; 0x6C];
    hd[0..12].copy_from_slice(b"AcFssFcAJMB\0");
    let put32 = |hd: &mut Vec<u8>, at: usize, v: u32| hd[at..at + 4].copy_from_slice(&v.to_le_bytes());
    let put64 = |hd: &mut Vec<u8>, at: usize, v: u64| hd[at..at + 8].copy_from_slice(&v.to_le_bytes());
    put32(&mut hd, 0x10, 0x6C);
    put32(&mut hd, 0x14, 0x04);
    put32(&mut hd, 0x24, 1);
    put32(&mut hd, 0x28, pmap_id);
    put64(&mut hd, 0x2C, (second_at - 0x100) as u64);
    put64(&mut hd, 0x34, second_at as u64);
    put32(&mut hd, 0x3C, 0);
    put32(&mut hd, 0x40, page_map.len() as u32 + 1);
    put32(&mut hd, 0x44, 0x20);
    put32(&mut hd, 0x48, 0x80);
    put32(&mut hd, 0x4C, 0x40);
    put32(&mut hd, 0x50, pmap_id);
    put64(&mut hd, 0x54, (pmap_at - 0x100) as u64);
    put32(&mut hd, 0x5C, smap_id);
    put32(&mut hd, 0x60, pmap_id);
    put32(&mut hd, 0x64, 0);
    let crc = crc32(0, &hd);
    put32(&mut hd, 0x68, crc);
    let m = magic();
    let enc: Vec<u8> = hd.iter().zip(m.iter()).map(|(a, b)| a ^ b).collect();
    // the file's first 0x100 bytes
    let mut fh = file_header.to_vec();
    fh.resize(0x80, 0);
    fh[0..6].copy_from_slice(ver.magic());
    fh[0x0B] = maint;
    fh[0x13..0x15].copy_from_slice(&codepage.to_le_bytes());
    let addr_of = |n: &str| {
        first_page_addr
            .iter()
            .find(|p| p.0 == n)
            .map_or(0, |p| p.1 as u32 + 0x20)
    };
    fh[0x0D..0x11].copy_from_slice(&addr_of("AcDb:Preview").to_le_bytes());
    fh[0x20..0x24].copy_from_slice(&addr_of("AcDb:SummaryInfo").to_le_bytes());
    fh[0x24..0x28].copy_from_slice(&addr_of("AcDb:VBAProject").to_le_bytes());
    fh[0x28..0x2C].copy_from_slice(&0x80u32.to_le_bytes());
    fh[0x2C..0x30].copy_from_slice(&addr_of("AcDb:AppInfo").to_le_bytes());
    file[0..0x80].copy_from_slice(&fh);
    file[0x80..0xEC].copy_from_slice(&enc);
    file[0xEC..0x100].copy_from_slice(&m[0xEC..0x100]);
    file.extend_from_slice(&enc);
    Ok(file)
}

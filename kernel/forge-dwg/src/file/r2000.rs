//! R13-R15 files (chapter 3): sections at addresses the file header lists.

use super::{Class, Drawing, Extra, Sections};
use crate::bits::Bad;
use crate::io::{Io, Reader};
use crate::bits::{BitReader, BitWriter, R, bad, crc16, read_mc};
use crate::object::{Classes, read_object};
use crate::str::Str;
use crate::version::Version;

const SN_CLASSES: [u8; 16] = [
    0x8D, 0xA1, 0xC4, 0xB8, 0xC4, 0xA9, 0xF8, 0xC5, 0xC0, 0xDC, 0xF4, 0x5F, 0xE7, 0xCF, 0xB6, 0x8A,
];
const SN_HEADER: [u8; 16] = [
    0xCF, 0x7B, 0x1F, 0x23, 0xFD, 0xDE, 0x38, 0xA9, 0x5F, 0x7C, 0x68, 0xB8, 0x4E, 0x6D, 0x33, 0x5F,
];

fn rl(buf: &[u8], at: usize) -> R<u32> {
    match buf.get(at..at + 4) {
        Some(b) => Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        None => bad("past the end"),
    }
}

/// A sentinel-wrapped section's data, its CRC checked.
fn sentinel_section<'a>(buf: &'a [u8], at: usize, sn: &[u8; 16], what: &str) -> R<&'a [u8]> {
    if buf.get(at..at + 16) != Some(&sn[..]) {
        return bad(format!("{what}: no sentinel at {at:#x}"));
    }
    let n = rl(buf, at + 16)? as usize;
    let Some(data) = buf.get(at + 20..at + 20 + n) else {
        return bad(format!("{what}: past the end"));
    };
    let crc = u16::from_le_bytes([buf[at + 20 + n], buf[at + 21 + n]]);
    if crc16(0xC0C1, &buf[at + 16..at + 20 + n]) != crc {
        return bad(format!("{what}: CRC"));
    }
    Ok(data)
}

/// The classes section (chapter 10): sentinel, size, the classes, CRC.
pub fn classes(sec: &[u8], ver: Version, maint: u8) -> R<Vec<Class>> {
    if sec.get(0..16) != Some(&SN_CLASSES[..]) {
        return bad("classes: no sentinel");
    }
    let size = rl(sec, 16)? as usize;
    let mut at = 20;
    // the size's high half: R2010 and R2013 with a maintenance version above 3, R2018 always
    if (ver >= Version::R2010 && maint > 3) || ver >= Version::R2018 {
        at += 4;
    }
    let Some(data) = sec.get(at..at + size) else {
        return bad("classes: past the end");
    };
    let crc = u16::from_le_bytes([sec[at + size], sec[at + size + 1]]);
    if crc16(0xC0C1, &sec[16..at + size]) != crc {
        return bad("classes: CRC");
    }
    let (mut r, end) = if ver >= Version::R2007 {
        // the size in bits (counted from itself), then the classes, their strings at the end
        let bits = rl(data, 0)? as usize;
        let (strs, end) = string_stream(data, bits)?;
        (Reader { ver, data: BitReader::new(data, 4), strs, hdl: BitReader::new(data, 0), base: 0, data_end: end }, end)
    } else {
        (Reader { ver, data: BitReader::new(data, 0), strs: None, hdl: BitReader::new(data, 0), base: 0, data_end: data.len() * 8 }, data.len() * 8)
    };
    if ver >= Version::R2004 {
        let (mut max, mut z, mut t) = (0i16, 0u8, false);
        r.bs(&mut max)?;
        r.rc(&mut z)?;
        r.rc(&mut z)?;
        r.b(&mut t)?;
    }
    let mut out = vec![];
    // to the end of the data (less a few bits of padding)
    while r.data.p + 18 < end {
        let mut c = Class::default();
        class_io(&mut c, &mut r)?;
        out.push(c);
    }
    Ok(out)
}

/// One class's fields.
pub fn class_io<S: Io>(c: &mut Class, s: &mut S) -> R<()> {
    s.bs(&mut c.number)?;
    s.bs(&mut c.flags)?;
    s.tv(&mut c.app)?;
    s.tv(&mut c.cpp)?;
    s.tv(&mut c.dxf)?;
    s.b(&mut c.zombie)?;
    s.bs(&mut c.item)?;
    if s.ver() >= Version::R2004 {
        s.bl(&mut c.count)?;
        s.bs(&mut c.dwg_version)?;
        s.bs(&mut c.maint_version)?;
        s.bl(&mut c.unknown[0])?;
        s.bl(&mut c.unknown[1])?;
    }
    Ok(())
}

/// R2007+: a stream of `bits` bits whose last is the string stream's presence: the strings'
/// reader, and where the data before them ends.
pub fn string_stream(d: &[u8], bits: usize) -> R<(Option<BitReader<'_>>, usize)> {
    if bits == 0 {
        return Ok((None, 0));
    }
    let mut b = BitReader { d, p: bits - 1 };
    if !b.b()? {
        return Ok((None, bits - 1));
    }
    let mut at = bits - 1 - 16;
    let mut n = BitReader { d, p: at }.rs()? as usize;
    if n & 0x8000 != 0 {
        at -= 16;
        let hi = BitReader { d, p: at }.rs()? as usize;
        n = (n & 0x7FFF) | (hi << 15);
    }
    let start = at.checked_sub(n).ok_or(crate::bits::Bad("string stream size".into()))?;
    Ok((Some(BitReader { d, p: start }), start))
}

/// The object map (chapter 23): (handle, file offset) pairs.
pub fn read_map(buf: &[u8], at: usize, len: usize) -> R<Vec<(u32, usize)>> {
    let mut out = vec![];
    let mut p = at;
    while p < at + len {
        let Some(s) = buf.get(p..p + 2) else {
            return bad("object map past the end");
        };
        let size = u16::from_be_bytes([s[0], s[1]]) as usize;
        if size == 2 {
            break;
        }
        let start = p;
        p += 2;
        let (mut h, mut loc) = (0i64, 0i64);
        while p - start < size {
            let (dh, q) = read_mc(buf, p, false)?;
            let (dl, q) = read_mc(buf, q, true)?;
            p = q;
            h += dh;
            loc += dl;
            out.push((h as u32, loc as usize));
        }
        let crc = u16::from_be_bytes([buf[p], buf[p + 1]]);
        if crc16(0xC0C1, &buf[start..p]) != crc {
            return bad("object map CRC");
        }
        p += 2;
    }
    Ok(out)
}

/// The sections at the addresses the file header gives.
pub fn sections(buf: &[u8]) -> R<Sections> {
    let n = rl(buf, 0x15)? as usize;
    let mut recs = vec![None; 6];
    for k in 0..n {
        let at = 0x19 + 9 * k;
        let Some(&nr) = buf.get(at) else {
            return bad("section locators past the end");
        };
        if (nr as usize) < 6 {
            recs[nr as usize] = Some((rl(buf, at + 1)? as usize, rl(buf, at + 5)? as usize));
        }
    }
    let (Some(h), Some(c), Some(m)) = (recs[0], recs[1], recs[2]) else {
        return bad("missing header, classes or object map");
    };
    let get = |(a, n): (usize, usize)| buf.get(a..a + n).map(|s| s.to_vec()).ok_or(Bad("section past the end".into()));
    let mut other = vec![];
    for (k, name) in [(3, "ObjFreeSpace"), (4, "Template"), (5, "AuxHeader")] {
        if let Some(r) = recs[k] {
            other.push(Extra { name: name.to_string(), data: get(r)?, ..Extra::default() });
        }
    }
    // the preview, at the image seeker: sentinel, size, its entries and data, sentinel
    let seeker = rl(buf, 0x0D)? as usize;
    if buf.get(seeker..seeker + 16) == Some(&SN_PREVIEW[..]) {
        let n = rl(buf, seeker + 16)? as usize;
        let mut p = get((seeker, 16 + 4 + n + 16))?;
        // its data addresses, relative to the sentinel from here on
        relocate_preview(&mut p, -(seeker as i64))?;
        other.push(Extra { name: "Preview".into(), data: p, ..Extra::default() });
    }
    Ok(Sections {
        ver: Version::R2000,
        maint: buf[11],
        codepage: u16::from_le_bytes([buf[0x13], buf[0x14]]),
        header: get(h)?,
        classes: get(c)?,
        handles: get(m)?,
        objects: buf.to_vec(),
        other,
        file_header: buf[..0x80.min(buf.len())].to_vec(),
    })
}

/// The object map and the classes, without reading the objects (for tools).
pub fn read_shell(buf: &[u8]) -> R<(Vec<(u32, usize)>, Vec<(String, bool)>)> {
    let n = rl(buf, 0x15)? as usize;
    let mut recs = vec![None; 6];
    for k in 0..n {
        let at = 0x19 + 9 * k;
        let nr = buf[at] as usize;
        if nr < 6 {
            recs[nr] = Some((rl(buf, at + 1)? as usize, rl(buf, at + 5)? as usize));
        }
    }
    let (Some(c), Some(m)) = (recs[1], recs[2]) else {
        return bad("missing classes or object map");
    };
    let classes = classes(&buf[c.0..c.0 + c.1], Version::R2000, buf[11])?;
    let list = classes.iter().map(|c| (c.dxf.text(), c.item == 0x1F2)).collect();
    Ok((read_map(buf, m.0, m.1)?, list))
}

/// An object's type number, read from its start.
pub fn peek_type(buf: &[u8], at: usize, ver: Version) -> R<u16> {
    let (_, mut start) = crate::bits::read_ms(buf, at)?;
    if ver >= Version::R2010 {
        start = crate::bits::read_mc(buf, start, false)?.1;
        let mut r = BitReader::new(buf, start);
        return Ok(match r.bb()? {
            0 => r.rc()? as u16,
            1 => r.rc()? as u16 + 0x1F0,
            _ => r.rs()?,
        });
    }
    Ok(BitReader::new(buf, start).bs()? as u16)
}

pub const SN_PREVIEW: [u8; 16] = [
    0x1F, 0x25, 0x6D, 0x07, 0xD4, 0x36, 0x28, 0x28, 0x9D, 0x57, 0xCA, 0x3F, 0x9D, 0x44, 0x10, 0x2B,
];
const SN_PREVIEW_END: [u8; 16] = [
    0xE0, 0xDA, 0x92, 0xF8, 0x2B, 0xC9, 0xD7, 0xD7, 0x62, 0xA8, 0x35, 0xC0, 0x62, 0xBB, 0xEF, 0xD4,
];
const SN_CLASSES_END: [u8; 16] = [
    0x72, 0x5E, 0x3B, 0x47, 0x3B, 0x56, 0x07, 0x3A, 0x3F, 0x23, 0x0B, 0xA0, 0x18, 0x30, 0x49, 0x75,
];
const SN_FILE_HEADER_END: [u8; 16] = [
    0x95, 0xA0, 0x4E, 0x28, 0x99, 0x82, 0x1A, 0xE5, 0x5E, 0x41, 0xE0, 0x5F, 0x9D, 0x3A, 0x4D, 0x00,
];
const SN_SECOND: [u8; 16] = [
    0xD4, 0x7B, 0x21, 0xCE, 0x28, 0x93, 0x9F, 0xBF, 0x53, 0x24, 0x40, 0x09, 0x12, 0x3C, 0xAA, 0x01,
];
const SN_SECOND_END: [u8; 16] = [
    0x2B, 0x84, 0xDE, 0x31, 0xD7, 0x6C, 0x60, 0x40, 0xAC, 0xDB, 0xBF, 0xF6, 0xED, 0xC3, 0x55, 0xFE,
];

/// Moves a preview's data addresses by `by` (its entries hold file addresses).
pub fn relocate_preview(p: &mut [u8], by: i64) -> R<()> {
    let count = *p.get(20).ok_or(Bad("preview: too short".into()))? as usize;
    for k in 0..count {
        let at = 21 + 9 * k + 1;
        let a = rl(p, at)? as i64 + by;
        if a < 0 {
            return bad("preview: address before the start");
        }
        p[at..at + 4].copy_from_slice(&(a as u32).to_le_bytes());
    }
    Ok(())
}

/// The classes section for a version.
pub fn write_classes(classes: &[Class], ver: Version, maint: u8) -> R<Vec<u8>> {
    use crate::io::Writer;
    let mut w = Writer::new(ver);
    let max = classes.iter().map(|c| c.number).max().unwrap_or(499);
    if ver >= Version::R2007 {
        w.data.rl(0); // the size in bits, below
        w.data.bl(max as i32);
        w.data.b(true);
    } else if ver >= Version::R2004 {
        w.data.bs(max);
        w.data.rc(0);
        w.data.rc(0);
        w.data.b(true);
    }
    for c in classes {
        class_io(&mut c.clone(), &mut w)?;
    }
    let mut d = w.data;
    if ver >= Version::R2007 {
        let has = w.strs.len > 0;
        if has {
            d.append(&w.strs);
            let n = w.strs.len;
            if n >= 0x8000 {
                d.rs((n >> 15) as u16);
                d.rs((n & 0x7FFF) as u16 | 0x8000);
            } else {
                d.rs(n as u16);
            }
        }
        d.b(has);
        let bits = d.len;
        d.patch_rl(0, bits as u32);
    }
    let mut body = (d.bytes.len() as u32).to_le_bytes().to_vec();
    if crate::header::has_hi_size(ver, maint) {
        body.extend_from_slice(&[0; 4]);
    }
    body.extend_from_slice(&d.bytes);
    let crc = crc16(0xC0C1, &body);
    let mut out = SN_CLASSES.to_vec();
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&SN_CLASSES_END);
    if ver >= Version::R2004 {
        out.extend_from_slice(&[0; 8]);
    }
    Ok(out)
}

/// The object map: (handle, offset) steps in sections of at most 2032 bytes, each starting again
/// from handle 0 and offset 0.
pub fn write_map(locs: &[(u32, u64)]) -> Vec<u8> {
    use crate::bits::{mc, umc};
    let mut out = vec![];
    let flush = |data: &[u8], out: &mut Vec<u8>| {
        let mut s = ((data.len() + 2) as u16).to_be_bytes().to_vec();
        s.extend_from_slice(data);
        let crc = crc16(0xC0C1, &s);
        out.extend_from_slice(&s);
        out.extend_from_slice(&crc.to_be_bytes());
    };
    let (mut data, mut last_h, mut last_loc) = (vec![], 0u32, 0u64);
    for &(h, loc) in locs {
        let mut step = vec![];
        umc(&mut step, h - last_h);
        mc(&mut step, loc as i64 - last_loc as i64);
        if data.len() + step.len() + 2 > 2032 {
            flush(&data, &mut out);
            data.clear();
            step.clear();
            umc(&mut step, h);
            mc(&mut step, loc as i64);
        }
        data.extend_from_slice(&step);
        last_h = h;
        last_loc = loc;
    }
    if !data.is_empty() {
        flush(&data, &mut out);
    }
    flush(&[], &mut out);
    out
}

/// The objects one after another from `base`, and the map of where each went.
pub fn write_objects(d: &Drawing, base: usize) -> R<(Vec<u8>, Vec<(u32, u64)>)> {
    let ver = d.ver();
    let mut out = vec![];
    let mut locs = vec![];
    for (h, o) in &d.objects {
        locs.push((*h, (base + out.len()) as u64));
        out.extend_from_slice(&crate::object::write_object(o, ver)?);
    }
    Ok((out, locs))
}

/// The auxiliary header (chapter 27): the one read with its handle seed brought up to date, or
/// a new one.
pub fn aux_header(d: &Drawing) -> Vec<u8> {
    let seed = d.handseed();
    if let Some(a) = d.extra("AuxHeader").filter(|a| a.data.len() >= 0x5F) {
        let mut a = a.data.clone();
        // the seed sits after 0x3F bytes of fixed fields and the two dates
        a[0x4F..0x53].copy_from_slice(&(if seed < 0x7FFF_FFFF { seed } else { 0xFFFF_FFFF }).to_le_bytes());
        return a;
    }
    let mut a = vec![0xFF, 0x77, 0x01];
    let ver = match d.ver() {
        Version::R2000 => 23u16,
        Version::R2004 => 25,
        Version::R2007 => 27,
        Version::R2010 => 29,
        Version::R2013 => 31,
        Version::R2018 => 33,
    };
    let rs = |a: &mut Vec<u8>, v: u16| a.extend_from_slice(&v.to_le_bytes());
    let rl = |a: &mut Vec<u8>, v: u32| a.extend_from_slice(&v.to_le_bytes());
    rs(&mut a, ver);
    rs(&mut a, d.maint as u16);
    rl(&mut a, 1);
    rl(&mut a, 0xFFFF_FFFF);
    rs(&mut a, 1);
    rs(&mut a, 0);
    rl(&mut a, 0);
    for _ in 0..2 {
        rs(&mut a, ver);
        rs(&mut a, d.maint as u16);
    }
    for v in [5, 0x893, 5, 0x893, 0, 1] {
        rs(&mut a, v);
    }
    for _ in 0..9 {
        rl(&mut a, 0);
    }
    rl(&mut a, if seed < 0x7FFF_FFFF { seed } else { 0xFFFF_FFFF });
    rl(&mut a, 0);
    rs(&mut a, 0);
    rs(&mut a, 1);
    for _ in 0..3 {
        rl(&mut a, 0);
    }
    rl(&mut a, 1);
    for _ in 0..4 {
        rl(&mut a, 0);
    }
    if d.ver() >= Version::R2018 {
        for _ in 0..3 {
            rs(&mut a, 0);
        }
    }
    a
}

/// The handles the second header lists, from the header variables.
fn second_handles(d: &Drawing) -> Vec<u32> {
    let h = |n: &str| d.header.handle(n).unwrap_or(0);
    vec![
        d.handseed(),
        h("BLOCK_CONTROL"),
        h("LAYER_CONTROL"),
        h("STYLE_CONTROL"),
        h("LTYPE_CONTROL"),
        h("VIEW_CONTROL"),
        h("UCS_CONTROL"),
        h("VPORT_CONTROL"),
        h("APPID_CONTROL"),
        h("DIMSTYLE_CONTROL"),
        h("VX_CONTROL"),
        h("DICT_NAMED_OBJECTS"),
        h("DICT_ACAD_MLINESTYLE"),
        h("DICT_ACAD_GROUP"),
    ]
}

/// The second file header (chapter 26).
fn second_header(d: &Drawing, at: u32, sections: &[(u8, u32, u32)]) -> Vec<u8> {
    let mut b = BitWriter::new();
    b.rl(0); // the size, patched below
    b.bl(at as i32);
    b.raw(b"AC1015\0\0\0\0\0");
    b.rc(d.maint);
    for _ in 0..4 {
        b.b(false);
    }
    b.rc(0x10);
    b.raw(&[0x5C, 0x18, 0xB4, 0x01]);
    b.rc(sections.len() as u8);
    for &(n, a, s) in sections {
        b.rc(n);
        b.bl(a as i32);
        b.bl(s as i32);
    }
    let hs = second_handles(d);
    b.bs(hs.len() as i16);
    for (k, h) in hs.iter().enumerate() {
        let bytes = h.to_be_bytes();
        let skip = bytes.iter().take_while(|&&x| x == 0).count().min(3);
        b.rc((4 - skip) as u8);
        b.rc(k as u8);
        b.raw(&bytes[skip..]);
    }
    while b.len % 8 != 0 {
        b.b(false);
    }
    let size = (b.bytes.len() + 2 + 8) as u32;
    b.patch_rl(0, size);
    let crc = crc16(0xC0C1, &b.bytes);
    let mut out = SN_SECOND.to_vec();
    out.extend_from_slice(&b.bytes);
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&[0; 8]);
    out.extend_from_slice(&SN_SECOND_END);
    out
}

/// An R2000 file, laid out as the Open Design Toolkit lays one out: file header, auxiliary
/// header, preview, header variables, classes, padding, objects, object map, free-space
/// section, second header, template.
pub fn write(d: &Drawing) -> R<Vec<u8>> {
    let ver = Version::R2000;
    const RECORDS: usize = 6;
    let file_header_len = 0x19 + 9 * RECORDS + 2 + 16;
    let aux = aux_header(d);
    let mut preview = match d.extra("Preview") {
        Some(p) => p.data.clone(),
        None => {
            let mut p = SN_PREVIEW.to_vec();
            p.extend_from_slice(&1u32.to_le_bytes());
            p.push(0);
            p.extend_from_slice(&SN_PREVIEW_END);
            p
        }
    };
    let header = crate::header::write_header(&d.header, ver, d.maint)?;
    let classes = write_classes(&d.classes, ver, d.maint)?;
    let aux_at = file_header_len;
    let preview_at = aux_at + aux.len();
    relocate_preview(&mut preview, preview_at as i64)?;
    let header_at = preview_at + preview.len();
    let classes_at = header_at + header.len();
    let objects_at = classes_at + classes.len() + 0x200;
    let (objects, locs) = write_objects(d, objects_at)?;
    let map_at = objects_at + objects.len();
    let map = write_map(&locs);
    let free_at = map_at + map.len();
    let mut free = match d.extra("ObjFreeSpace") {
        Some(f) if f.data.len() >= 21 => f.data.clone(),
        _ => {
            let mut f = vec![0u8; 21];
            f[20] = 4;
            for v in [0x32u32, 0, 0x64, 0, 0x200, 0, 0xFFFF_FFFF, 0] {
                f.extend_from_slice(&v.to_le_bytes());
            }
            f
        }
    };
    free[4..8].copy_from_slice(&(locs.len() as u32).to_le_bytes());
    free[16..20].copy_from_slice(&(objects_at as u32).to_le_bytes());
    let template = d.extra("Template").map_or(vec![0, 0, 1, 0], |t| t.data.clone());
    let second_at = free_at + free.len();
    let template_at_guess = 0; // placed after the second header, whose size doesn't depend on it
    let _ = template_at_guess;
    let sections = [
        (0u8, header_at as u32, header.len() as u32),
        (1, classes_at as u32, classes.len() as u32),
        (2, map_at as u32, map.len() as u32),
        (3, free_at as u32, free.len() as u32),
        (4, 0, 1),
        (5, aux_at as u32, aux.len() as u32),
    ];
    let second = second_header(d, second_at as u32, &sections);
    let template_at = second_at + second.len();

    let mut out = Vec::with_capacity(template_at + template.len());
    out.extend_from_slice(b"AC1015\0\0\0\0\0");
    out.push(d.maint);
    out.push(1);
    out.extend_from_slice(&(preview_at as u32).to_le_bytes());
    // the application version bytes as read
    out.extend_from_slice(d.file_header.get(0x11..0x13).unwrap_or(&[0, 0]));
    out.extend_from_slice(&d.codepage.to_le_bytes());
    out.extend_from_slice(&(RECORDS as u32).to_le_bytes());
    let real = [
        sections[0],
        sections[1],
        sections[2],
        sections[3],
        (4, template_at as u32, template.len() as u32),
        sections[5],
    ];
    for (n, a, s) in real {
        out.push(n);
        out.extend_from_slice(&a.to_le_bytes());
        out.extend_from_slice(&s.to_le_bytes());
    }
    let crc = crc16(0, &out) ^ 0x8461;
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&SN_FILE_HEADER_END);
    out.extend_from_slice(&aux);
    out.extend_from_slice(&preview);
    out.extend_from_slice(&header);
    out.extend_from_slice(&classes);
    out.extend_from_slice(&[0; 0x200]);
    out.extend_from_slice(&objects);
    out.extend_from_slice(&map);
    out.extend_from_slice(&free);
    out.extend_from_slice(&second);
    out.extend_from_slice(&template);
    Ok(out)
}

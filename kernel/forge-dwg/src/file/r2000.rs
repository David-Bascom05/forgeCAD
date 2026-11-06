//! R13-R15 files (chapter 3): sections at addresses the file header lists.

use super::{Class, Drawing};
use crate::bits::{BitReader, R, bad, crc16, read_mc};
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

pub fn read_classes(data: &[u8], ver: Version) -> R<Vec<Class>> {
    let mut r = BitReader::new(data, 0);
    let mut out = vec![];
    // R2004+: max class number, two zero bytes and a bit first
    if ver >= Version::R2004 {
        r.bs()?;
        r.rc()?;
        r.rc()?;
        r.b()?;
    }
    while r.p + 8 * 2 < data.len() * 8 {
        let mut c = Class {
            number: r.bs()?,
            flags: r.bs()?,
            ..Class::default()
        };
        if ver >= Version::R2007 {
            return bad("R2007 classes need the string stream");
        }
        c.app = Str::Cp(r.t()?);
        c.cpp = Str::Cp(r.t()?);
        c.dxf = Str::Cp(r.t()?);
        c.zombie = r.b()?;
        c.item = r.bs()?;
        if ver >= Version::R2004 {
            c.count = r.bl()?;
            c.dwg_version = r.bs()?;
            c.maint_version = r.bs()?;
            c.unknown = [r.bl()?, r.bl()?];
        }
        out.push(c);
    }
    Ok(out)
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

pub fn read(buf: &[u8]) -> R<Drawing> {
    let ver = Version::R2000;
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
    let header = sentinel_section(buf, h.0, &SN_HEADER, "header")?.to_vec();
    let classes = read_classes(sentinel_section(buf, c.0, &SN_CLASSES, "classes")?, ver)?;
    let mut d = Drawing {
        version: Some(ver),
        maint: buf[11],
        codepage: u16::from_le_bytes([buf[0x13], buf[0x14]]),
        header,
        classes,
        ..Drawing::default()
    };
    let list = d.class_list();
    let cl = Classes { list: &list };
    for (handle, at) in read_map(buf, m.0, m.1)? {
        let o = read_object(buf, at, ver, &cl, None).map_err(|e| crate::bits::Bad(format!("{handle:#x}: {}", e.0)))?;
        d.objects.insert(handle, o);
    }
    Ok(d)
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
    let classes = read_classes(sentinel_section(buf, c.0, &SN_CLASSES, "classes")?, Version::R2000)?;
    let list = classes.iter().map(|c| (c.dxf.text(), c.item == 0x1F2)).collect();
    Ok((read_map(buf, m.0, m.1)?, list))
}

/// An object's type number, read from its start.
pub fn peek_type(buf: &[u8], at: usize) -> R<u16> {
    let (_, start) = crate::bits::read_ms(buf, at)?;
    Ok(BitReader::new(buf, start).bs()? as u16)
}

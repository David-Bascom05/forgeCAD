//! R13-R15 files (chapter 3): sections at addresses the file header lists.

use super::{Class, Sections};
use crate::bits::Bad;
use crate::io::{Io, Reader};
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
            other.push((name.to_string(), get(r)?));
        }
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

//! A ZIP archive as a 3MF package needs (PKWARE APPNOTE): members deflated, or stored where that
//! is smaller, no ZIP64, every timestamp 1980-01-01 00:00 so the same members give the same bytes.
//! And a reader for what it writes and what other producers write (stored or deflated members),
//! for round trips and for reading a 3MF back.

use forge_git::zlib;

const LOCAL: u32 = 0x0403_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const END: u32 = 0x0605_4b50;
/// 1980-01-01, the earliest a DOS date holds.
const DOS_DATE: u16 = (1 << 5) | 1;

/// CRC-32 (IEEE 802.3, reflected), as ZIP checks each member with.
pub fn crc32(data: &[u8]) -> u32 {
    const fn table() -> [u32; 256] {
        let mut t = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            t[i] = c;
            i += 1;
        }
        t
    }
    static T: [u32; 256] = table();
    let mut c = !0u32;
    for &b in data {
        c = T[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8);
    }
    !c
}

#[derive(Debug, Clone, PartialEq)]
pub enum ZipError {
    /// Over 4 GiB or 65535 members: ZIP64, which this writer does not write.
    TooLarge,
    /// Not a ZIP archive, or a damaged one: what was wrong.
    Bad(String),
}

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZipError::TooLarge => write!(f, "archive over 4 GiB (ZIP64 is not written)"),
            ZipError::Bad(w) => write!(f, "bad zip: {w}"),
        }
    }
}

fn u16le(o: &mut Vec<u8>, v: u16) {
    o.extend_from_slice(&v.to_le_bytes());
}

fn u32le(o: &mut Vec<u8>, v: u32) {
    o.extend_from_slice(&v.to_le_bytes());
}

/// The archive of `members` (name, content), in the order given.
pub fn write(members: &[(&str, &[u8])]) -> Result<Vec<u8>, ZipError> {
    let big = |n: usize| u32::try_from(n).map_err(|_| ZipError::TooLarge);
    if members.len() > u16::MAX as usize {
        return Err(ZipError::TooLarge);
    }
    let mut out = Vec::new();
    let mut central = Vec::new();
    for &(name, data) in members {
        let packed = zlib::deflate(data);
        let (method, body): (u16, &[u8]) = if packed.len() < data.len() {
            (8, &packed)
        } else {
            (0, data)
        };
        let crc = crc32(data);
        let at = big(out.len())?;
        let header = |o: &mut Vec<u8>| -> Result<(), ZipError> {
            u16le(o, 20); // version needed: 2.0 (deflate)
            u16le(o, 0x0800); // flags: names are UTF-8
            u16le(o, method);
            u16le(o, 0); // time 00:00:00
            u16le(o, DOS_DATE);
            u32le(o, crc);
            u32le(o, big(body.len())?);
            u32le(o, big(data.len())?);
            u16le(o, u16::try_from(name.len()).map_err(|_| ZipError::TooLarge)?);
            u16le(o, 0); // extra
            Ok(())
        };
        u32le(&mut out, LOCAL);
        header(&mut out)?;
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(body);
        u32le(&mut central, CENTRAL);
        u16le(&mut central, 20); // made by: MS-DOS, 2.0
        header(&mut central)?;
        u16le(&mut central, 0); // comment
        u16le(&mut central, 0); // disk
        u16le(&mut central, 0); // internal attributes
        u32le(&mut central, 0); // external attributes
        u32le(&mut central, at);
        central.extend_from_slice(name.as_bytes());
    }
    let cd_at = big(out.len())?;
    let cd_len = big(central.len())?;
    out.extend_from_slice(&central);
    big(out.len() + 22)?;
    u32le(&mut out, END);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u16le(&mut out, members.len() as u16);
    u16le(&mut out, members.len() as u16);
    u32le(&mut out, cd_len);
    u32le(&mut out, cd_at);
    u16le(&mut out, 0);
    Ok(out)
}

/// An archive's members, name and content, in the central directory's order. Each member's
/// content is checked against its CRC; at most `cap` bytes are unpacked in all.
pub fn read(data: &[u8], cap: usize) -> Result<Vec<(String, Vec<u8>)>, ZipError> {
    let bad = |w: &str| ZipError::Bad(w.to_string());
    let r16 = |at: usize| -> Result<u16, ZipError> {
        data.get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .ok_or_else(|| bad("truncated"))
    };
    let r32 = |at: usize| -> Result<u32, ZipError> {
        data.get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| bad("truncated"))
    };
    // the end record: the last signature within a comment's reach of the end
    let lo = data.len().saturating_sub(22 + u16::MAX as usize);
    let end = (lo..=data.len().saturating_sub(22))
        .rev()
        .find(|&i| r32(i).ok() == Some(END))
        .ok_or_else(|| bad("no end of central directory"))?;
    let n = r16(end + 10)? as usize;
    let mut at = r32(end + 16)? as usize;
    let mut out = Vec::with_capacity(n);
    let mut left = cap;
    for _ in 0..n {
        if r32(at)? != CENTRAL {
            return Err(bad("central directory entry"));
        }
        let flags = r16(at + 8)?;
        let method = r16(at + 10)?;
        let crc = r32(at + 16)?;
        let csize = r32(at + 20)? as usize;
        let usize_ = r32(at + 24)? as usize;
        let (nl, xl, cl) = (
            r16(at + 28)? as usize,
            r16(at + 30)? as usize,
            r16(at + 32)? as usize,
        );
        let local = r32(at + 42)? as usize;
        let name = data
            .get(at + 46..at + 46 + nl)
            .ok_or_else(|| bad("truncated name"))?;
        let name = String::from_utf8_lossy(name).into_owned();
        at += 46 + nl + xl + cl;
        if flags & 1 != 0 {
            return Err(bad("encrypted member"));
        }
        if r32(local)? != LOCAL {
            return Err(bad("local header"));
        }
        let start = local + 30 + r16(local + 26)? as usize + r16(local + 28)? as usize;
        let body = data
            .get(start..start + csize)
            .ok_or_else(|| bad("truncated member"))?;
        if usize_ > left {
            return Err(bad("members larger than the cap"));
        }
        let content = match method {
            0 => body.to_vec(),
            8 => zlib::inflate(body, left).map_err(|e| ZipError::Bad(format!("{name}: {e:?}")))?,
            m => return Err(ZipError::Bad(format!("{name}: compression method {m}"))),
        };
        if content.len() != usize_ || crc32(&content) != crc {
            return Err(ZipError::Bad(format!("{name}: size or CRC mismatch")));
        }
        left -= content.len();
        out.push((name, content));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_of_the_check_string() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn round_trip_stored_and_deflated() {
        let long = "abc ".repeat(1000);
        let members: Vec<(&str, &[u8])> =
            vec![("a.txt", b"x"), ("dir/b.xml", long.as_bytes()), ("empty", b"")];
        let z = write(&members).unwrap();
        assert_eq!(z, write(&members).unwrap(), "deterministic");
        let back = read(&z, 1 << 20).unwrap();
        assert_eq!(back.len(), 3);
        for ((n, d), (m, e)) in members.iter().zip(&back) {
            assert_eq!(*n, m);
            assert_eq!(*d, &e[..]);
        }
        // the long member was deflated
        assert!(z.len() < long.len());
        assert!(read(&z[..z.len() - 1], 1 << 20).is_err());
        assert!(read(&z, 100).is_err());
    }
}

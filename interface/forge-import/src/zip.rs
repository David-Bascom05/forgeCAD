//! The members of a zip archive, stored or deflated, by the central directory, ZIP64's too: slicers
//! write even a small 3MF package with a ZIP64 end record, the classic one's counts and offset left
//! all ones. A file is untrusted: what its members would inflate to is capped before anything is
//! inflated, and a refusal names the cap it met.

use std::collections::HashMap;
use std::fmt;

/// The most one member may inflate to.
pub const MEMBER_MAX: usize = 256 << 20;
/// The most all members together may (the largest FreeCAD library file holds 393 MB).
pub const TOTAL_MAX: usize = 1 << 30;
/// Past 64 kB, a member may inflate at most this many times (the library's most is 59).
pub const RATIO_MAX: usize = 1000;

/// Why an archive is not read.
#[derive(Clone, Debug, PartialEq)]
pub enum ZipError {
    /// Not a zip archive, or a broken one: what is wrong.
    NotZip(&'static str),
    /// A member that would inflate past `MEMBER_MAX`.
    MemberTooLarge { name: String, size: u64 },
    /// Members that together would inflate past `TOTAL_MAX`.
    TotalTooLarge { size: u64 },
    /// A member that would inflate more than `RATIO_MAX` times its stored size.
    Ratio { name: String, size: u64, stored: u64 },
}

impl fmt::Display for ZipError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mb = |x: u64| x as f64 / (1u64 << 20) as f64;
        match self {
            ZipError::NotZip(why) => write!(f, "not a zip archive: {why}"),
            ZipError::MemberTooLarge { name, size } => write!(
                f,
                "member {name} would inflate to {:.0} MB, past the {} MB a member may",
                mb(*size),
                MEMBER_MAX >> 20
            ),
            ZipError::TotalTooLarge { size } => write!(
                f,
                "the members would inflate to {:.0} MB together, past the {} MB an archive may",
                mb(*size),
                TOTAL_MAX >> 20
            ),
            ZipError::Ratio { name, size, stored } => write!(
                f,
                "member {name} would inflate {} times (to {size} bytes from {stored}), past the {RATIO_MAX} times a member may",
                size / (*stored).max(1)
            ),
        }
    }
}

impl std::error::Error for ZipError {}

pub struct Zip<'a> {
    data: &'a [u8],
    /// name: (method, offset of the local header, compressed size, size)
    members: HashMap<String, (u16, usize, usize, usize)>,
}

fn u16_at(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(i..i.checked_add(2)?)?.try_into().ok()?))
}

fn u32_at(d: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(i..i.checked_add(4)?)?.try_into().ok()?))
}

fn u64_at(d: &[u8], i: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(i..i.checked_add(8)?)?.try_into().ok()?))
}

/// The ZIP64 end record's entry count and directory offset, where the locator before the classic
/// end record at `end` names one.
fn zip64_end(data: &[u8], end: usize) -> Option<(u64, u64)> {
    let loc = end.checked_sub(20)?;
    if u32_at(data, loc)? != 0x0706_4b50 {
        return None;
    }
    let at = usize::try_from(u64_at(data, loc + 8)?).ok()?;
    if u32_at(data, at)? != 0x0606_4b50 {
        return None;
    }
    Some((u64_at(data, at + 32)?, u64_at(data, at + 48)?))
}

impl<'a> Zip<'a> {
    pub fn new(data: &'a [u8]) -> Result<Zip<'a>, ZipError> {
        use ZipError::NotZip;
        // the end record, its comment up to 64k after it
        let last = data
            .len()
            .checked_sub(22)
            .ok_or(NotZip("shorter than an end record"))?;
        let lo = data.len().saturating_sub(22 + 0xffff);
        let end = (lo..=last)
            .rev()
            .find(|&i| data[i..i + 4] == [0x50, 0x4b, 0x05, 0x06])
            .ok_or(NotZip("no end record"))?;
        let short = NotZip("cut short");
        let n16 = u16_at(data, end + 10).ok_or(short.clone())?;
        let off32 = u32_at(data, end + 16).ok_or(short.clone())?;
        // all ones: the real values are in the ZIP64 end record (a writer may set them so either way)
        let (n, cd) = match zip64_end(data, end) {
            Some(z) if n16 == 0xffff || off32 == 0xffff_ffff => z,
            _ => (u64::from(n16), u64::from(off32)),
        };
        // each entry is at least 46 bytes: a count past that is a broken record, not a long loop
        if n > (data.len() / 46) as u64 {
            return Err(NotZip("more entries than the file holds"));
        }
        let mut at = usize::try_from(cd).map_err(|_| short.clone())?;
        let mut members = HashMap::new();
        let mut total = 0u64;
        for _ in 0..n {
            if u32_at(data, at) != Some(0x0201_4b50) {
                return Err(NotZip(
                    "a central directory entry is not where the end record says",
                ));
            }
            let f = |i: usize| u32_at(data, at + i).ok_or(short.clone());
            let g = |i: usize| u16_at(data, at + i).map(usize::from).ok_or(short.clone());
            let method = g(10)? as u16;
            let (mut csize, mut size, mut off) = (u64::from(f(20)?), u64::from(f(24)?), u64::from(f(42)?));
            let (nl, xl, cl) = (g(28)?, g(30)?, g(32)?);
            let name_at = at + 46;
            let name =
                String::from_utf8_lossy(data.get(name_at..name_at + nl).ok_or(short.clone())?).into_owned();
            // ZIP64's extra field: the 8-byte values of those left all ones, in this order
            let mut x = name_at + nl;
            let xend = x + xl;
            while x + 4 <= xend {
                let (id, len) = (
                    u16_at(data, x).ok_or(short.clone())?,
                    usize::from(u16_at(data, x + 2).ok_or(short.clone())?),
                );
                if id == 0x0001 {
                    let mut v = x + 4;
                    for slot in [&mut size, &mut csize, &mut off] {
                        if *slot == 0xffff_ffff && v + 8 <= x + 4 + len {
                            *slot = u64_at(data, v).ok_or(short.clone())?;
                            v += 8;
                        }
                    }
                }
                x += 4 + len;
            }
            total = total.saturating_add(size);
            if size > MEMBER_MAX as u64 {
                return Err(ZipError::MemberTooLarge { name, size });
            }
            if total > TOTAL_MAX as u64 {
                return Err(ZipError::TotalTooLarge { size: total });
            }
            if size > 1 << 16 && size / RATIO_MAX as u64 > csize {
                return Err(ZipError::Ratio {
                    name,
                    size,
                    stored: csize,
                });
            }
            if csize > data.len() as u64 || off > data.len() as u64 {
                return Err(NotZip("a member past the file's end"));
            }
            // a later member of the same name is the one read, as Python's zipfile does
            members.insert(name, (method, off as usize, csize as usize, size as usize));
            at = xend + cl;
        }
        Ok(Zip { data, members })
    }

    pub fn has(&self, name: &str) -> bool {
        self.members.contains_key(name)
    }

    pub fn size(&self, name: &str) -> Option<usize> {
        self.members.get(name).map(|m| m.3)
    }

    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        let &(method, off, csize, size) = self.members.get(name)?;
        let d = self.data;
        if u32_at(d, off)? != 0x0403_4b50 {
            return None;
        }
        let start = off + 30 + u16_at(d, off + 26)? as usize + u16_at(d, off + 28)? as usize;
        let raw = d.get(start..start.checked_add(csize)?)?;
        let out = match method {
            0 => raw.to_vec(),
            8 => forge_git::zlib::inflate(raw, size).ok()?,
            _ => return None,
        };
        (out.len() == size).then_some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le16(o: &mut Vec<u8>, v: u16) {
        o.extend_from_slice(&v.to_le_bytes());
    }
    fn le32(o: &mut Vec<u8>, v: u32) {
        o.extend_from_slice(&v.to_le_bytes());
    }
    fn le64(o: &mut Vec<u8>, v: u64) {
        o.extend_from_slice(&v.to_le_bytes());
    }

    /// One stored member, `zip64` as slicers write it: the ZIP64 end record and locator, the
    /// classic end record's fields all ones, and (`extra`) the entry's sizes and offset in its
    /// ZIP64 extra field.
    fn archive(name: &str, body: &[u8], zip64: bool, extra: bool) -> Vec<u8> {
        let mut o = vec![];
        le32(&mut o, 0x0403_4b50);
        le16(&mut o, 20);
        le16(&mut o, 0);
        le16(&mut o, 0);
        le32(&mut o, 0);
        le32(&mut o, 0);
        le32(&mut o, body.len() as u32);
        le32(&mut o, body.len() as u32);
        le16(&mut o, name.len() as u16);
        le16(&mut o, 0);
        o.extend_from_slice(name.as_bytes());
        o.extend_from_slice(body);
        let cd = o.len();
        le32(&mut o, 0x0201_4b50);
        le16(&mut o, 45);
        le16(&mut o, 45);
        le16(&mut o, 0);
        le16(&mut o, 0);
        le32(&mut o, 0);
        le32(&mut o, 0);
        let big = if extra { 0xffff_ffff } else { body.len() as u32 };
        le32(&mut o, big);
        le32(&mut o, big);
        le16(&mut o, name.len() as u16);
        le16(&mut o, if extra { 28 } else { 0 });
        le16(&mut o, 0);
        le16(&mut o, 0);
        le16(&mut o, 0);
        le32(&mut o, 0);
        le32(&mut o, if extra { 0xffff_ffff } else { 0 });
        o.extend_from_slice(name.as_bytes());
        if extra {
            le16(&mut o, 1);
            le16(&mut o, 24);
            le64(&mut o, body.len() as u64);
            le64(&mut o, body.len() as u64);
            le64(&mut o, 0);
        }
        let cd_size = o.len() - cd;
        if zip64 {
            let z = o.len();
            le32(&mut o, 0x0606_4b50);
            le64(&mut o, 44);
            le16(&mut o, 45);
            le16(&mut o, 45);
            le32(&mut o, 0);
            le32(&mut o, 0);
            le64(&mut o, 1);
            le64(&mut o, 1);
            le64(&mut o, cd_size as u64);
            le64(&mut o, cd as u64);
            le32(&mut o, 0x0706_4b50);
            le32(&mut o, 0);
            le64(&mut o, z as u64);
            le32(&mut o, 1);
        }
        le32(&mut o, 0x0605_4b50);
        le16(&mut o, if zip64 { 0xffff } else { 0 });
        le16(&mut o, if zip64 { 0xffff } else { 0 });
        le16(&mut o, if zip64 { 0xffff } else { 1 });
        le16(&mut o, if zip64 { 0xffff } else { 1 });
        le32(&mut o, if zip64 { 0xffff_ffff } else { cd_size as u32 });
        le32(&mut o, if zip64 { 0xffff_ffff } else { cd as u32 });
        le16(&mut o, 0);
        o
    }

    #[test]
    fn classic_zip64_and_zip64_extra_fields_read_alike() {
        for (z, x) in [(false, false), (true, false), (true, true)] {
            let a = archive("a.txt", b"hello", z, x);
            let zip = Zip::new(&a).unwrap_or_else(|e| panic!("{z} {x}: {e}"));
            assert_eq!(zip.read("a.txt").as_deref(), Some(&b"hello"[..]), "{z} {x}");
        }
    }

    #[test]
    fn refusals_name_their_cause() {
        assert_eq!(
            Zip::new(b"no zip here, just text, long enough").err(),
            Some(ZipError::NotZip("no end record"))
        );
        // a member said to inflate past the cap: named, with the cap
        let mut a = archive("big.model", b"x", true, true);
        // the ZIP64 extra field: id 1, 24 bytes, the size first
        let at = a.windows(4).position(|w| w == [1, 0, 24, 0]).unwrap();
        let size_at = at + 4;
        a[size_at..size_at + 8].copy_from_slice(&((MEMBER_MAX as u64) + 1).to_le_bytes());
        let e = Zip::new(&a).err().unwrap();
        assert!(
            matches!(e, ZipError::MemberTooLarge { ref name, .. } if name == "big.model"),
            "{e:?}"
        );
        assert!(e.to_string().contains("256 MB"), "{e}");
    }
}

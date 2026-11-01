//! The members of a zip archive, stored or deflated, by the central directory. No zip64: a
//! document past 4 GB is no part file.

use std::collections::HashMap;

pub struct Zip<'a> {
    data: &'a [u8],
    /// name: (method, offset of the local header, compressed size, size)
    members: HashMap<String, (u16, usize, usize, usize)>,
}

fn u16_at(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(i..i + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(i..i + 4)?.try_into().ok()?) as usize)
}

impl<'a> Zip<'a> {
    pub fn new(data: &'a [u8]) -> Option<Zip<'a>> {
        // the end record, its comment up to 64k after it
        let lo = data.len().saturating_sub(22 + 0xffff);
        let end = (lo..=data.len().checked_sub(22)?)
            .rev()
            .find(|&i| data[i..i + 4] == [0x50, 0x4b, 0x05, 0x06])?;
        let n = u16_at(data, end + 10)? as usize;
        let mut at = u32_at(data, end + 16)?;
        let mut members = HashMap::new();
        for _ in 0..n {
            if u32_at(data, at)? != 0x0201_4b50 {
                return None;
            }
            let method = u16_at(data, at + 10)?;
            let (csize, size) = (u32_at(data, at + 20)?, u32_at(data, at + 24)?);
            let (nl, xl, cl) = (
                u16_at(data, at + 28)? as usize,
                u16_at(data, at + 30)? as usize,
                u16_at(data, at + 32)? as usize,
            );
            let off = u32_at(data, at + 42)?;
            let name = String::from_utf8_lossy(data.get(at + 46..at + 46 + nl)?).into_owned();
            // a later member of the same name is the one read, as Python's zipfile does
            members.insert(name, (method, off, csize, size));
            at += 46 + nl + xl + cl;
        }
        Some(Zip { data, members })
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
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(raw, size).ok()?,
            _ => return None,
        };
        (out.len() == size).then_some(out)
    }
}

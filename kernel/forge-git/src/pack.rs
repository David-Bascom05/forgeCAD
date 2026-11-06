//! Pack files and their index, version 2 (gitformat-pack): objects found by id through the
//! index's fan-out and binary search, read at their offset, deltas (by offset or by id) applied
//! onto their bases. Read with `read_at`, never mapped.

use crate::Error;
use crate::hash::{HashKind, Oid};
use crate::object::Kind;
use crate::zlib;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The longest delta chain followed; git's own default depth is 50, and `--depth` caps at 4095.
pub const MAX_CHAIN: usize = 10_000;

pub struct Pack {
    pub path: PathBuf,
    file: File,
    len: u64,
    hash: HashKind,
    count: usize,
    fanout: Vec<u32>,
    names: Vec<u8>,
    offsets: Vec<u32>,
    large: Vec<u64>,
    sorted: OnceLock<Vec<u64>>,
}

pub enum Raw {
    Whole(Kind, Vec<u8>),
    Ofs(u64, Vec<u8>),
    Ref(Oid, Vec<u8>),
}

fn be32(d: &[u8], i: usize) -> Result<u32, Error> {
    d.get(i..i + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(Error::Corrupt("pack index"))
}

impl Pack {
    /// The pack beside an index `pack-X.idx`.
    pub fn open(idx_path: &Path, hash: HashKind) -> Result<Pack, Error> {
        let d = std::fs::read(idx_path)?;
        if d.len() < 8 + 1024 || d[..4] != [0xff, b't', b'O', b'c'] || be32(&d, 4)? != 2 {
            return Err(Error::Corrupt("pack index version"));
        }
        let fanout: Vec<u32> = (0..256).map(|i| be32(&d, 8 + 4 * i)).collect::<Result<_, _>>()?;
        let n = fanout[255] as usize;
        let h = hash.len();
        let names_at = 8 + 1024;
        let crc_at = names_at + n * h;
        let off_at = crc_at + n * 4;
        let large_at = off_at + n * 4;
        if d.len() < large_at + 2 * h {
            return Err(Error::Corrupt("pack index size"));
        }
        let names = d[names_at..crc_at].to_vec();
        let offsets: Vec<u32> = (0..n)
            .map(|i| be32(&d, off_at + 4 * i))
            .collect::<Result<_, _>>()?;
        let nlarge = offsets.iter().filter(|&&o| o & 0x8000_0000 != 0).count();
        let large = (0..nlarge)
            .map(|i| {
                d.get(large_at + 8 * i..large_at + 8 * i + 8)
                    .map(|b| u64::from_be_bytes(b.try_into().expect("8")))
                    .ok_or(Error::Corrupt("pack index large offsets"))
            })
            .collect::<Result<_, _>>()?;
        let path = idx_path.with_extension("pack");
        let file = File::open(&path)?;
        let len = file.metadata()?.len();
        let mut head = [0u8; 12];
        file.read_exact_at(&mut head, 0)?;
        if &head[..4] != b"PACK" || !matches!(be32(&head, 4)?, 2 | 3) || be32(&head, 8)? as usize != n {
            return Err(Error::Corrupt("pack header"));
        }
        Ok(Pack {
            path,
            file,
            len,
            hash,
            count: n,
            fanout,
            names,
            offsets,
            large,
            sorted: OnceLock::new(),
        })
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn id_at(&self, i: usize) -> Oid {
        let h = self.hash.len();
        Oid::from_bytes(self.hash, &self.names[i * h..i * h + h]).expect("len")
    }

    pub fn offset_at(&self, i: usize) -> u64 {
        let o = self.offsets[i];
        if o & 0x8000_0000 != 0 {
            self.large[(o & 0x7fff_ffff) as usize]
        } else {
            o as u64
        }
    }

    pub fn find(&self, id: &Oid) -> Option<u64> {
        let b = id.as_bytes();
        let first = b[0] as usize;
        let lo = if first == 0 {
            0
        } else {
            self.fanout[first - 1] as usize
        };
        let hi = self.fanout[first] as usize;
        let h = self.hash.len();
        let (mut lo, mut hi) = (lo, hi);
        while lo < hi {
            let mid = (lo + hi) / 2;
            match self.names[mid * h..mid * h + h].cmp(b) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => return Some(self.offset_at(mid)),
            }
        }
        None
    }

    /// Where the object after `off` begins: its compressed data ends there.
    fn end_of(&self, off: u64) -> u64 {
        let s = self.sorted.get_or_init(|| {
            let mut v: Vec<u64> = (0..self.count).map(|i| self.offset_at(i)).collect();
            v.sort_unstable();
            v
        });
        match s.binary_search(&off) {
            Ok(i) if i + 1 < s.len() => s[i + 1],
            _ => self.len - self.hash.len() as u64,
        }
    }

    /// The entry at `off` as stored: whole, or a delta and what it is on.
    pub fn raw(&self, off: u64) -> Result<Raw, Error> {
        let end = self.end_of(off);
        if off >= end || end > self.len {
            return Err(Error::Corrupt("pack offset"));
        }
        let span = (end - off) as usize;
        let mut d = vec![0u8; span];
        self.file.read_exact_at(&mut d, off)?;
        let mut i = 0;
        let c = d[0];
        let t = (c >> 4) & 7;
        let mut size = (c & 15) as u64;
        let mut shift = 4;
        let mut c = c;
        while c & 0x80 != 0 {
            i += 1;
            c = *d.get(i).ok_or(Error::Corrupt("pack entry header"))?;
            if shift > 60 {
                return Err(Error::Corrupt("pack entry size"));
            }
            size |= ((c & 0x7f) as u64) << shift;
            shift += 7;
        }
        i += 1;
        let size = usize::try_from(size).map_err(|_| Error::Corrupt("pack entry size"))?;
        let inflate = |from: usize| -> Result<Vec<u8>, Error> {
            let mut out = Vec::with_capacity(size);
            zlib::inflate_zlib_into(d.get(from..).ok_or(Error::Corrupt("pack entry"))?, size, &mut out)?;
            if out.len() != size {
                return Err(Error::Corrupt("pack entry length"));
            }
            Ok(out)
        };
        match t {
            6 => {
                let mut c = *d.get(i).ok_or(Error::Corrupt("ofs delta"))?;
                let mut back = (c & 0x7f) as u64;
                while c & 0x80 != 0 {
                    i += 1;
                    c = *d.get(i).ok_or(Error::Corrupt("ofs delta"))?;
                    back = back
                        .checked_add(1)
                        .and_then(|b| b.checked_mul(128))
                        .ok_or(Error::Corrupt("ofs delta"))?
                        | (c & 0x7f) as u64;
                }
                let base = off.checked_sub(back).ok_or(Error::Corrupt("ofs delta base"))?;
                if back == 0 {
                    return Err(Error::Corrupt("ofs delta base"));
                }
                Ok(Raw::Ofs(base, inflate(i + 1)?))
            }
            7 => {
                let h = self.hash.len();
                let id = Oid::from_bytes(self.hash, d.get(i..i + h).ok_or(Error::Corrupt("ref delta"))?)
                    .expect("len");
                Ok(Raw::Ref(id, inflate(i + h)?))
            }
            _ => {
                let kind = Kind::from_pack(t).ok_or(Error::Corrupt("pack entry type"))?;
                Ok(Raw::Whole(kind, inflate(i)?))
            }
        }
    }
}

fn varint(d: &[u8], i: &mut usize) -> Result<usize, Error> {
    let mut v = 0usize;
    let mut shift = 0;
    loop {
        let c = *d.get(*i).ok_or(Error::Corrupt("delta size"))?;
        *i += 1;
        if shift > 56 {
            return Err(Error::Corrupt("delta size"));
        }
        v |= ((c & 0x7f) as usize) << shift;
        shift += 7;
        if c & 0x80 == 0 {
            return Ok(v);
        }
    }
}

/// `delta` applied to `base`: copies from the base and inserted bytes.
pub fn apply_delta(base: &[u8], delta: &[u8]) -> Result<Vec<u8>, Error> {
    let mut i = 0;
    if varint(delta, &mut i)? != base.len() {
        return Err(Error::Corrupt("delta base size"));
    }
    let n = varint(delta, &mut i)?;
    let mut out = Vec::with_capacity(n);
    while i < delta.len() {
        let c = delta[i];
        i += 1;
        if c & 0x80 != 0 {
            let mut off = 0usize;
            let mut len = 0usize;
            for k in 0..4 {
                if c & (1 << k) != 0 {
                    off |= (*delta.get(i).ok_or(Error::Corrupt("delta copy"))? as usize) << (8 * k);
                    i += 1;
                }
            }
            for k in 0..3 {
                if c & (0x10 << k) != 0 {
                    len |= (*delta.get(i).ok_or(Error::Corrupt("delta copy"))? as usize) << (8 * k);
                    i += 1;
                }
            }
            if len == 0 {
                len = 0x10000;
            }
            let src = base
                .get(off..off.checked_add(len).ok_or(Error::Corrupt("delta copy"))?)
                .ok_or(Error::Corrupt("delta copy range"))?;
            out.extend_from_slice(src);
        } else if c != 0 {
            let src = delta
                .get(i..i + c as usize)
                .ok_or(Error::Corrupt("delta insert"))?;
            out.extend_from_slice(src);
            i += c as usize;
        } else {
            return Err(Error::Corrupt("delta opcode 0"));
        }
        if out.len() > n {
            return Err(Error::Corrupt("delta result size"));
        }
    }
    if out.len() != n {
        return Err(Error::Corrupt("delta result size"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_copies_and_inserts() {
        let base = b"hello, world";
        // src 12, dst 11: copy off 0 len 5, insert "!!", copy off 7 len 4
        let delta = [12, 11, 0x90, 5, 2, b'!', b'!', 0x91, 7, 4];
        assert_eq!(apply_delta(base, &delta).unwrap(), b"hello!!worl");
        assert!(apply_delta(base, &[12, 3, 0x90, 50]).is_err());
        assert!(apply_delta(base, &[11, 0]).is_err());
    }
}

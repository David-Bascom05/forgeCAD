//! The index (gitformat-index), versions 2 to 4: read whole, entries changed by path, written
//! back under `index.lock`. Optional extensions (an upper-case first letter) are dropped on
//! write, as stale ones would mislead git; an index with a required extension this does not know
//! (split index `link`, sparse `sdir`) or with conflict stages is left alone.

use crate::Error;
use crate::fsutil::Lock;
use crate::hash::{HashKind, Oid};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IEntry {
    pub ctime: (u32, u32),
    pub mtime: (u32, u32),
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub id: Oid,
    pub flags: u16,
    pub ext_flags: Option<u16>,
    pub path: Vec<u8>,
}

impl IEntry {
    pub fn stage(&self) -> u16 {
        (self.flags >> 12) & 3
    }

    /// An entry for a file just written, its stat taken now.
    pub fn from_file(path: &str, file: &Path, mode: u32, id: Oid) -> Result<IEntry, Error> {
        let m = std::fs::symlink_metadata(file)?;
        let s = stat(&m);
        Ok(IEntry {
            ctime: (s[0], s[1]),
            mtime: (s[2], s[3]),
            dev: s[4],
            ino: s[5],
            mode,
            uid: s[6],
            gid: s[7],
            size: m.len() as u32,
            id,
            flags: path.len().min(0xfff) as u16,
            ext_flags: None,
            path: path.as_bytes().to_vec(),
        })
    }
}

/// ctime, mtime (seconds and nanoseconds), dev, ino, uid, gid, as the index keeps them.
#[cfg(unix)]
fn stat(m: &std::fs::Metadata) -> [u32; 8] {
    use std::os::unix::fs::MetadataExt;
    [
        m.ctime() as u32,
        m.ctime_nsec() as u32,
        m.mtime() as u32,
        m.mtime_nsec() as u32,
        m.dev() as u32,
        m.ino() as u32,
        m.uid(),
        m.gid(),
    ]
}

/// Where the platform has no such stat: only the time (git then checks the content).
#[cfg(not(unix))]
fn stat(m: &std::fs::Metadata) -> [u32; 8] {
    let t = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .unwrap_or_default();
    [
        t.as_secs() as u32,
        t.subsec_nanos(),
        t.as_secs() as u32,
        t.subsec_nanos(),
        0,
        0,
        0,
        0,
    ]
}

#[derive(Clone, Debug)]
pub struct Index {
    pub version: u32,
    pub entries: Vec<IEntry>,
}

fn be32(d: &[u8], i: usize) -> Result<u32, Error> {
    d.get(i..i + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(Error::Corrupt("index"))
}

impl Index {
    pub fn empty() -> Index {
        Index {
            version: 2,
            entries: vec![],
        }
    }

    pub fn parse(d: &[u8], hash: HashKind) -> Result<Index, Error> {
        let h = hash.len();
        if d.len() < 12 + h || &d[..4] != b"DIRC" {
            return Err(Error::Corrupt("index header"));
        }
        let version = be32(d, 4)?;
        if !(2..=4).contains(&version) {
            return Err(Error::Unsupported(format!("index version {version}")));
        }
        let body = &d[..d.len() - h];
        let sum = &d[d.len() - h..];
        if sum.iter().any(|&b| b != 0) && hash.of(body).as_bytes() != sum {
            return Err(Error::Corrupt("index checksum"));
        }
        let n = be32(d, 8)? as usize;
        let mut i = 12;
        let mut entries = Vec::with_capacity(n);
        let mut prev: Vec<u8> = vec![];
        for _ in 0..n {
            let start = i;
            let w = |k: usize| be32(body, start + 4 * k);
            let id = Oid::from_bytes(
                hash,
                body.get(start + 40..start + 40 + h)
                    .ok_or(Error::Corrupt("index"))?,
            )
            .expect("len");
            let fl_at = start + 40 + h;
            let flags = u16::from_be_bytes(
                body.get(fl_at..fl_at + 2)
                    .ok_or(Error::Corrupt("index"))?
                    .try_into()
                    .expect("2"),
            );
            let mut j = fl_at + 2;
            let ext_flags = if flags & 0x4000 != 0 {
                if version < 3 {
                    return Err(Error::Corrupt("index extended flag in v2"));
                }
                let x = u16::from_be_bytes(
                    body.get(j..j + 2)
                        .ok_or(Error::Corrupt("index"))?
                        .try_into()
                        .expect("2"),
                );
                j += 2;
                Some(x)
            } else {
                None
            };
            let path = if version == 4 {
                let mut strip = 0usize;
                let mut shift = 0;
                // v4's offset varint: each continuation adds one before shifting
                let mut c = *body.get(j).ok_or(Error::Corrupt("index"))?;
                j += 1;
                strip |= (c & 0x7f) as usize;
                while c & 0x80 != 0 {
                    c = *body.get(j).ok_or(Error::Corrupt("index"))?;
                    j += 1;
                    strip = ((strip + 1) << 7) | (c & 0x7f) as usize;
                    shift += 1;
                    if shift > 8 {
                        return Err(Error::Corrupt("index path"));
                    }
                }
                let nul = body[j..]
                    .iter()
                    .position(|&c| c == 0)
                    .ok_or(Error::Corrupt("index path"))?
                    + j;
                let keep = prev
                    .len()
                    .checked_sub(strip)
                    .ok_or(Error::Corrupt("index path"))?;
                let mut p = prev[..keep].to_vec();
                p.extend_from_slice(&body[j..nul]);
                i = nul + 1;
                p
            } else {
                let nul = body[j..]
                    .iter()
                    .position(|&c| c == 0)
                    .ok_or(Error::Corrupt("index path"))?
                    + j;
                let p = body[j..nul].to_vec();
                let len = nul - start;
                i = start + (len + 8) / 8 * 8;
                p
            };
            prev = path.clone();
            entries.push(IEntry {
                ctime: (w(0)?, w(1)?),
                mtime: (w(2)?, w(3)?),
                dev: w(4)?,
                ino: w(5)?,
                mode: w(6)?,
                uid: w(7)?,
                gid: w(8)?,
                size: w(9)?,
                id,
                flags,
                ext_flags,
                path,
            });
        }
        while i + 8 <= body.len() {
            let sig = &body[i..i + 4];
            let len = be32(body, i + 4)? as usize;
            if !sig[0].is_ascii_uppercase() {
                return Err(Error::Unsupported(format!(
                    "index extension {}",
                    String::from_utf8_lossy(sig)
                )));
            }
            i += 8 + len;
        }
        if entries.iter().any(|e| e.stage() != 0) {
            return Err(Error::Busy("merge (the index has conflicts)"));
        }
        Ok(Index { version, entries })
    }

    pub fn write(&self, hash: HashKind) -> Vec<u8> {
        let version = if self.version == 2 && self.entries.iter().any(|e| e.ext_flags.is_some()) {
            3
        } else {
            self.version
        };
        let mut d = b"DIRC".to_vec();
        d.extend_from_slice(&version.to_be_bytes());
        d.extend_from_slice(&(self.entries.len() as u32).to_be_bytes());
        let mut prev: &[u8] = &[];
        for e in &self.entries {
            let start = d.len();
            for w in [
                e.ctime.0, e.ctime.1, e.mtime.0, e.mtime.1, e.dev, e.ino, e.mode, e.uid, e.gid, e.size,
            ] {
                d.extend_from_slice(&w.to_be_bytes());
            }
            d.extend_from_slice(e.id.as_bytes());
            let mut flags = (e.flags & 0xf000) | e.path.len().min(0xfff) as u16;
            flags &= !0x4000;
            if e.ext_flags.is_some() && version >= 3 {
                flags |= 0x4000;
            }
            d.extend_from_slice(&flags.to_be_bytes());
            if let Some(x) = e.ext_flags
                && version >= 3
            {
                d.extend_from_slice(&x.to_be_bytes());
            }
            if version == 4 {
                let common = prev.iter().zip(&e.path).take_while(|(a, b)| a == b).count();
                let mut strip = prev.len() - common;
                let mut v = vec![(strip & 0x7f) as u8];
                while strip >> 7 != 0 {
                    strip = (strip >> 7) - 1;
                    v.push(0x80 | (strip & 0x7f) as u8);
                }
                v.reverse();
                d.extend_from_slice(&v);
                d.extend_from_slice(&e.path[common..]);
                d.push(0);
            } else {
                d.extend_from_slice(&e.path);
                let len = d.len() - start;
                let pad = (len + 8) / 8 * 8 - len;
                d.extend(std::iter::repeat_n(0, pad));
            }
            prev = &e.path;
        }
        let sum = hash.of(&d);
        d.extend_from_slice(sum.as_bytes());
        d
    }

    /// Sets the entry for a path; entries it replaces (a file where its directory is, or files
    /// under a directory where it is) go.
    pub fn set(&mut self, e: IEntry) {
        let p = e.path.clone();
        self.entries.retain(|x| {
            let under = x.path.len() > p.len() && x.path.starts_with(&p) && x.path[p.len()] == b'/';
            let above = p.len() > x.path.len() && p.starts_with(&x.path) && p[x.path.len()] == b'/';
            x.path != p && !under && !above
        });
        let at = self.entries.partition_point(|x| x.path < p);
        self.entries.insert(at, e);
    }

    pub fn remove(&mut self, path: &str) {
        self.entries.retain(|x| x.path != path.as_bytes());
    }
}

/// Reads the index at `path`, changes it, and writes it back under its lock.
pub fn update(path: &Path, hash: HashKind, f: impl FnOnce(&mut Index)) -> Result<(), Error> {
    let mut lock = Lock::take(path)?;
    let mut idx = match std::fs::read(path) {
        Ok(d) => Index::parse(&d, hash)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Index::empty(),
        Err(e) => return Err(e.into()),
    };
    f(&mut idx);
    lock.write(&idx.write(hash))?;
    lock.commit()
}

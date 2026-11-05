//! Results kept on disk between runs, by content key: `$FORGECAD_CACHE`, else
//! `$XDG_CACHE_HOME/forgecad`, else `~/.cache/forgecad` (`FORGECAD_CACHE=off`: none kept).
//! Each entry is one file holding its key, the kernel it was made by and a checksum of its bytes;
//! one that does not match all three is removed, never read. The oldest entries go first once
//! the whole is over `FORGECAD_CACHE_MB` (2048 by default).

use forge_math::hash::{Hash128, hash128};
use std::path::{Path, PathBuf};
use std::sync::Once;

/// The sources' hash this binary was built from: entries of any other are not read.
pub const KERNEL: &str = env!("FORGE_KERNEL_HASH");

const MAGIC: &[u8; 4] = b"FGC1";
const HEAD: usize = 4 + 16 + 16 + 8 + 16;

/// A key over the kernel and whatever `f` puts in.
pub fn key(kind: &str, f: impl FnOnce(&mut Hash128)) -> u128 {
    let mut h = Hash128::new();
    h.str(KERNEL);
    h.str(kind);
    f(&mut h);
    h.finish()
}

pub fn root() -> Option<PathBuf> {
    let set = std::env::var_os("FORGECAD_CACHE");
    if set.as_deref().is_some_and(|s| s == "off" || s.is_empty()) {
        return None;
    }
    let base = set.map(PathBuf::from).or_else(|| {
        std::env::var_os("XDG_CACHE_HOME")
            .map(|d| PathBuf::from(d).join("forgecad"))
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache").join("forgecad")))
    })?;
    Some(base.join("results"))
}

fn path_in(root: &Path, kind: &str, key: u128) -> PathBuf {
    root.join(kind).join(format!("{key:032x}"))
}

fn kernel_id() -> u128 {
    hash128(KERNEL.as_bytes())
}

/// The bytes kept under `key`, if a whole, matching entry is there.
pub fn get(kind: &str, key: u128) -> Option<Vec<u8>> {
    get_in(&root()?, kind, key)
}

fn get_in(root: &Path, kind: &str, key: u128) -> Option<Vec<u8>> {
    let path = path_in(root, kind, key);
    let data = std::fs::read(&path).ok()?;
    match open(&data, key) {
        Some(body) => {
            // read lately: kept longest
            if let Ok(f) = std::fs::File::options().append(true).open(&path) {
                let _ = f.set_modified(std::time::SystemTime::now());
            }
            Some(body.to_vec())
        }
        None => {
            let _ = std::fs::remove_file(&path);
            None
        }
    }
}

fn open(data: &[u8], key: u128) -> Option<&[u8]> {
    let u128_at = |at: usize| Some(u128::from_le_bytes(data.get(at..at + 16)?.try_into().ok()?));
    if data.get(..4)? != MAGIC || u128_at(4)? != key || u128_at(20)? != kernel_id() {
        return None;
    }
    let len = u64::from_le_bytes(data.get(36..44)?.try_into().ok()?) as usize;
    let body = data.get(HEAD..)?;
    (body.len() == len && u128_at(44)? == hash128(body)).then_some(body)
}

/// Every whole, matching entry of `kind`, by key; the bad ones removed.
pub fn all(kind: &str) -> Vec<(u128, Vec<u8>)> {
    let Some(root) = root() else { return vec![] };
    let Ok(rd) = std::fs::read_dir(root.join(kind)) else {
        return vec![];
    };
    let mut out: Vec<(u128, Vec<u8>)> = rd
        .flatten()
        .filter_map(|e| u128::from_str_radix(e.file_name().to_str()?, 16).ok())
        .filter_map(|k| Some((k, get_in(&root, kind, k)?)))
        .collect();
    out.sort_by_key(|x| x.0);
    out
}

/// Drops the entry under `key`.
pub fn forget(kind: &str, key: u128) {
    if let Some(root) = root() {
        let _ = std::fs::remove_file(path_in(&root, kind, key));
    }
}

/// Keeps `bytes` under `key`. A cache that can't be written is no failure: the next run makes
/// them again.
pub fn put(kind: &str, key: u128, bytes: &[u8]) {
    if let Some(root) = root() {
        put_in(&root, kind, key, bytes);
        static TRIM: Once = Once::new();
        TRIM.call_once(|| trim(&root, limit()));
    }
}

fn put_in(root: &Path, kind: &str, key: u128, bytes: &[u8]) {
    let path = path_in(root, kind, key);
    let Some(dir) = path.parent() else { return };
    let _ = std::fs::create_dir_all(dir);
    let mut data = Vec::with_capacity(HEAD + bytes.len());
    data.extend_from_slice(MAGIC);
    data.extend_from_slice(&key.to_le_bytes());
    data.extend_from_slice(&kernel_id().to_le_bytes());
    data.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    data.extend_from_slice(&hash128(bytes).to_le_bytes());
    data.extend_from_slice(bytes);
    // written whole, then moved into place: a reader never sees half an entry
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    if std::fs::write(&tmp, &data).is_err() || std::fs::rename(&tmp, &path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

fn limit() -> u64 {
    std::env::var("FORGECAD_CACHE_MB")
        .ok()
        .and_then(|m| m.parse::<u64>().ok())
        .unwrap_or(2048)
        << 20
}

/// Removes the least lately used entries until the whole is under three quarters of `limit`,
/// once it is over it.
fn trim(root: &Path, limit: u64) {
    let mut all: Vec<(std::time::SystemTime, u64, PathBuf)> = Vec::new();
    let Ok(kinds) = std::fs::read_dir(root) else {
        return;
    };
    for k in kinds.flatten() {
        let Ok(files) = std::fs::read_dir(k.path()) else {
            continue;
        };
        for f in files.flatten() {
            if let Ok(m) = f.metadata() {
                all.push((m.modified().unwrap_or(std::time::UNIX_EPOCH), m.len(), f.path()));
            }
        }
    }
    let mut total: u64 = all.iter().map(|x| x.1).sum();
    if total <= limit {
        return;
    }
    all.sort();
    for (_, len, p) in all {
        if total <= limit / 4 * 3 {
            break;
        }
        if std::fs::remove_file(&p).is_ok() {
            total -= len;
        }
    }
}

/// Bytes out, for an entry.
#[derive(Default)]
pub struct Out(pub Vec<u8>);

impl Out {
    pub fn u64(&mut self, x: u64) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    pub fn u128(&mut self, x: u128) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    pub fn f64(&mut self, x: f64) {
        self.u64(x.to_bits());
    }
    pub fn bool(&mut self, x: bool) {
        self.0.push(x as u8);
    }
    pub fn bytes(&mut self, b: &[u8]) {
        self.u64(b.len() as u64);
        self.0.extend_from_slice(b);
    }
    pub fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
}

/// Bytes back, in the order `Out` put them; None past the end or on a malformed entry.
pub struct In<'a>(pub &'a [u8]);

impl<'a> In<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let (a, b) = (self.0.get(..n)?, self.0.get(n..)?);
        self.0 = b;
        Some(a)
    }
    pub fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    pub fn u128(&mut self) -> Option<u128> {
        Some(u128::from_le_bytes(self.take(16)?.try_into().ok()?))
    }
    pub fn f64(&mut self) -> Option<f64> {
        Some(f64::from_bits(self.u64()?))
    }
    pub fn bool(&mut self) -> Option<bool> {
        match self.take(1)?[0] {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
    pub fn bytes(&mut self) -> Option<&'a [u8]> {
        let n = usize::try_from(self.u64()?).ok()?;
        self.take(n)
    }
    pub fn str(&mut self) -> Option<&'a str> {
        std::str::from_utf8(self.bytes()?).ok()
    }
    /// A count of things to come, each at least `min` bytes: refused when there can't be that many.
    pub fn count(&mut self, min: usize) -> Option<usize> {
        let n = usize::try_from(self.u64()?).ok()?;
        (n.saturating_mul(min.max(1)) <= self.0.len()).then_some(n)
    }
    pub fn done(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("forge-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn an_entry_comes_back_under_its_key_only() {
        let d = scratch("key");
        put_in(&d, "t", 7, b"seven");
        assert_eq!(get_in(&d, "t", 7).as_deref(), Some(&b"seven"[..]));
        assert_eq!(get_in(&d, "t", 8), None);
        assert_eq!(get_in(&d, "u", 7), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_damaged_or_foreign_entry_is_dropped_not_read() {
        let d = scratch("bad");
        put_in(&d, "t", 1, b"some bytes");
        let p = path_in(&d, "t", 1);
        let mut data = std::fs::read(&p).unwrap();
        let last = data.len() - 1;
        data[last] ^= 1;
        std::fs::write(&p, &data).unwrap();
        assert_eq!(get_in(&d, "t", 1), None);
        assert!(!p.exists(), "a bad entry is removed");
        // cut short
        put_in(&d, "t", 2, b"some bytes");
        let p = path_in(&d, "t", 2);
        let data = std::fs::read(&p).unwrap();
        std::fs::write(&p, &data[..data.len() - 3]).unwrap();
        assert_eq!(get_in(&d, "t", 2), None);
        // another file moved under this key
        put_in(&d, "t", 3, b"three");
        std::fs::copy(path_in(&d, "t", 3), path_in(&d, "t", 4)).unwrap();
        assert_eq!(get_in(&d, "t", 4), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn trimming_drops_the_least_lately_used() {
        let d = scratch("trim");
        for k in 0..8u128 {
            put_in(&d, "t", k, &[0u8; 1000]);
            let t = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000 + k as u64);
            let f = std::fs::File::options()
                .append(true)
                .open(path_in(&d, "t", k))
                .unwrap();
            f.set_modified(t).unwrap();
        }
        trim(&d, 4 * 1100);
        let left: Vec<u128> = (0..8).filter(|k| get_in(&d, "t", *k).is_some()).collect();
        assert_eq!(left, vec![5, 6, 7]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn values_read_back_as_written() {
        let mut o = Out::default();
        o.u64(3);
        o.f64(-0.0);
        o.str("a");
        o.bool(true);
        let mut i = In(&o.0);
        assert_eq!(i.u64(), Some(3));
        assert_eq!(i.f64().map(f64::to_bits), Some((-0.0f64).to_bits()));
        assert_eq!(i.str(), Some("a"));
        assert_eq!(i.bool(), Some(true));
        assert!(i.done());
        assert_eq!(i.u64(), None);
    }
}

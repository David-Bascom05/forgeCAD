//! The object database: loose objects, every pack, and the stores `objects/info/alternates`
//! names. New objects are written loose: deflated to a temp file beside their place, synced,
//! then renamed there; an object already there is never written again.

use crate::Error;
use crate::fsutil;
use crate::hash::{HashKind, Oid};
use crate::object::{self, Kind};
use crate::pack::{self, Pack, Raw};
use crate::zlib;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Object {
    pub kind: Kind,
    pub data: Vec<u8>,
}

/// Resolved delta bases, kept while they fit the budget; the oldest go first.
struct Cache {
    map: HashMap<(usize, u64), Arc<Object>>,
    order: VecDeque<(usize, u64)>,
    bytes: usize,
}

const CACHE_BYTES: usize = 48 << 20;

impl Cache {
    fn get(&self, k: (usize, u64)) -> Option<Arc<Object>> {
        self.map.get(&k).cloned()
    }

    fn put(&mut self, k: (usize, u64), o: Arc<Object>) {
        if o.data.len() > CACHE_BYTES / 4 || self.map.contains_key(&k) {
            return;
        }
        self.bytes += o.data.len();
        self.map.insert(k, o);
        self.order.push_back(k);
        while self.bytes > CACHE_BYTES {
            let Some(old) = self.order.pop_front() else {
                break;
            };
            if let Some(o) = self.map.remove(&old) {
                self.bytes -= o.data.len();
            }
        }
    }
}

pub struct Odb {
    pub hash: HashKind,
    /// This repo's `objects` first, then its alternates.
    dirs: Vec<PathBuf>,
    packs: RwLock<Vec<Arc<Pack>>>,
    cache: Mutex<Cache>,
}

impl Odb {
    pub fn open(objects: &Path, hash: HashKind) -> Result<Odb, Error> {
        let mut dirs = vec![objects.to_path_buf()];
        alternates(objects, &mut dirs, 0);
        let odb = Odb {
            hash,
            dirs,
            packs: RwLock::new(vec![]),
            cache: Mutex::new(Cache {
                map: HashMap::new(),
                order: VecDeque::new(),
                bytes: 0,
            }),
        };
        odb.rescan()?;
        Ok(odb)
    }

    pub fn objects_dir(&self) -> &Path {
        &self.dirs[0]
    }

    /// Opens packs that appeared since the last look (git may have repacked meanwhile).
    pub fn rescan(&self) -> Result<bool, Error> {
        let mut packs = self.packs.write().expect("packs");
        let known: std::collections::HashSet<PathBuf> = packs.iter().map(|p| p.path.clone()).collect();
        let mut added = false;
        for d in &self.dirs {
            let Ok(rd) = std::fs::read_dir(d.join("pack")) else {
                continue;
            };
            let mut idx: Vec<PathBuf> = rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "idx"))
                .collect();
            idx.sort();
            for i in idx {
                if known.contains(&i.with_extension("pack")) || !i.with_extension("pack").exists() {
                    continue;
                }
                packs.push(Arc::new(Pack::open(&i, self.hash)?));
                added = true;
            }
        }
        Ok(added)
    }

    pub fn packs(&self) -> Vec<Arc<Pack>> {
        self.packs.read().expect("packs").clone()
    }

    fn loose_path(dir: &Path, id: &Oid) -> PathBuf {
        let h = id.hex();
        dir.join(&h[..2]).join(&h[2..])
    }

    pub fn read_loose(&self, id: &Oid) -> Result<Option<Object>, Error> {
        for d in &self.dirs {
            let p = Self::loose_path(d, id);
            let Ok(z) = std::fs::read(&p) else {
                continue;
            };
            let cap = z.len().saturating_mul(1100).max(1 << 16);
            let (raw, _) = zlib::inflate_zlib(&z, cap)?;
            let sp = raw
                .iter()
                .position(|&c| c == b' ')
                .ok_or(Error::Corrupt("loose header"))?;
            let nul = raw
                .iter()
                .position(|&c| c == 0)
                .ok_or(Error::Corrupt("loose header"))?;
            let kind = Kind::from_name(&raw[..sp]).ok_or(Error::Corrupt("loose type"))?;
            let len: usize = std::str::from_utf8(&raw[sp + 1..nul])
                .ok()
                .and_then(|s| s.parse().ok())
                .ok_or(Error::Corrupt("loose size"))?;
            if raw.len() - nul - 1 != len {
                return Err(Error::Corrupt("loose size"));
            }
            return Ok(Some(Object {
                kind,
                data: raw[nul + 1..].to_vec(),
            }));
        }
        Ok(None)
    }

    pub fn contains(&self, id: &Oid) -> bool {
        self.dirs.iter().any(|d| Self::loose_path(d, id).exists()) || self.find_packed(id).is_some()
    }

    fn find_packed(&self, id: &Oid) -> Option<(usize, Arc<Pack>, u64)> {
        let packs = self.packs.read().expect("packs");
        packs
            .iter()
            .enumerate()
            .find_map(|(i, p)| p.find(id).map(|o| (i, p.clone(), o)))
    }

    pub fn read(&self, id: &Oid) -> Result<Object, Error> {
        if let Some(o) = self.read_loose(id)? {
            return Ok(o);
        }
        for attempt in 0..2 {
            if let Some((i, p, off)) = self.find_packed(id) {
                return Ok(Arc::unwrap_or_clone(self.read_packed(i, &p, off)?));
            }
            if attempt == 0 && !self.rescan()? {
                // a loose object may have been packed and pruned between the two looks
                if let Some(o) = self.read_loose(id)? {
                    return Ok(o);
                }
                break;
            }
        }
        Err(Error::Missing(*id))
    }

    pub fn read_kind(&self, id: &Oid, kind: Kind) -> Result<Vec<u8>, Error> {
        let o = self.read(id)?;
        if o.kind != kind {
            return Err(Error::WrongKind(*id, kind));
        }
        Ok(o.data)
    }

    /// The object at `off` in pack number `pi`, its deltas applied.
    pub fn read_packed(&self, pi: usize, p: &Pack, off: u64) -> Result<Arc<Object>, Error> {
        let mut chain: Vec<(u64, Vec<u8>)> = vec![];
        let mut at = off;
        let base: Arc<Object> = loop {
            if let Some(o) = self.cache.lock().expect("cache").get((pi, at)) {
                break o;
            }
            if chain.len() > pack::MAX_CHAIN {
                return Err(Error::Corrupt("delta chain too long"));
            }
            match p.raw(at)? {
                Raw::Whole(kind, data) => break Arc::new(Object { kind, data }),
                Raw::Ofs(b, d) => {
                    chain.push((at, d));
                    at = b;
                }
                Raw::Ref(id, d) => {
                    chain.push((at, d));
                    match p.find(&id) {
                        Some(b) => at = b,
                        None => break Arc::new(self.read(&id)?),
                    }
                }
            }
        };
        let mut cur = base;
        if !chain.is_empty() {
            self.cache.lock().expect("cache").put((pi, at), cur.clone());
        }
        while let Some((o, d)) = chain.pop() {
            let data = pack::apply_delta(&cur.data, &d)?;
            cur = Arc::new(Object { kind: cur.kind, data });
            if !chain.is_empty() {
                self.cache.lock().expect("cache").put((pi, o), cur.clone());
            }
        }
        Ok(cur)
    }

    /// Writes an object loose (if no store has it); its id.
    pub fn write(&self, kind: Kind, data: &[u8]) -> Result<Oid, Error> {
        let id = object::id_of(self.hash, kind, data);
        if self.contains(&id) {
            return Ok(id);
        }
        let mut raw = object::header(kind, data.len());
        raw.extend_from_slice(data);
        let z = zlib::deflate_zlib(&raw);
        let path = Self::loose_path(&self.dirs[0], &id);
        let dir = path.parent().expect("fan-out dir");
        std::fs::create_dir_all(dir)?;
        fsutil::write_new(&path, &z)?;
        Ok(id)
    }

    /// Every loose object id in this repo's own store.
    pub fn loose_ids(&self) -> Vec<Oid> {
        let mut out = vec![];
        let Ok(rd) = std::fs::read_dir(&self.dirs[0]) else {
            return out;
        };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.len() != 2 || !n.bytes().all(|c| c.is_ascii_hexdigit()) {
                continue;
            }
            for f in std::fs::read_dir(e.path()).into_iter().flatten().flatten() {
                let rest = f.file_name().to_string_lossy().to_string();
                if let Some(id) = Oid::from_hex(self.hash, &format!("{n}{rest}")) {
                    out.push(id);
                }
            }
        }
        out.sort();
        out
    }
}

fn alternates(objects: &Path, dirs: &mut Vec<PathBuf>, depth: usize) {
    if depth > 5 {
        return;
    }
    let Ok(t) = std::fs::read_to_string(objects.join("info/alternates")) else {
        return;
    };
    for line in t.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let p = if Path::new(l).is_absolute() {
            PathBuf::from(l)
        } else {
            objects.join(l)
        };
        if p.is_dir() && !dirs.contains(&p) {
            dirs.push(p.clone());
            alternates(&p, dirs, depth + 1);
        }
    }
}

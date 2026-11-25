//! The durable store of phase 1: one append-only log file per document.
//!
//! Why a file log (DESIGN.md §5 keeps Postgres PROVISIONAL, FoundationDB and MongoDB unmeasured,
//! behind the `Store` trait): it is the fastest durable append measured (0.119 ms p50 with
//! fdatasync, MEASUREMENTS §d), needs no dependency (SQLite would bring rusqlite and C), and a
//! single server owns its documents in phase 1 (a folder lock makes sure of that: [`lock`]). Any
//! other store replaces it behind the same trait.
//!
//! Format, per document (`<dir>/<encoded name>.log`): records of
//! `[b"FGL1"][u32 LE payload length][u64 LE FNV-1a of the payload][payload]`. The first record's
//! payload is the document's first text (`{"first": TEXT}`), each next one an entry (forge-sync's
//! `entry_json`). Each append is written whole and `sync_data`ed before it returns; a new log is
//! made under a temporary name and renamed into place, so it is never seen empty.
//!
//! On open: a record **cut short by the end of the file** with nothing valid after it (a crash
//! mid-append: never acknowledged) is cut off. Anything else wrong (a bad checksum, a bad marker,
//! a length that runs past the end while whole records follow it) is corruption: the document
//! refuses to open, and nothing acknowledged is ever dropped silently.

use forge_agent::json::Json;
use forge_sync::wire::{entry_from, entry_json, read, write};
use forge_sync::{Entry, Store, text_hash};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// The most one record may hold (a document's first text is the largest).
pub const MAX_RECORD: u32 = 64 << 20;
const MAGIC: &[u8; 4] = b"FGL1";
const HEAD: usize = 16;

pub struct FileLogStore {
    dir: PathBuf,
    open: HashMap<String, File>,
    /// An open's parsed log, between `latest_snapshot` and `read_since`.
    loaded: HashMap<String, (u64, Vec<Entry>)>,
}

/// Holds the project folder for one server: a second server on the same folder refuses to
/// start (two sequencers of one document would fork its log). Released when dropped or when the
/// process ends.
pub fn lock(root: &Path) -> io::Result<File> {
    let d = root.join(".forgecad").join("server");
    std::fs::create_dir_all(&d)?;
    let f = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(d.join("lock"))?;
    match f.try_lock() {
        Ok(()) => Ok(f),
        Err(std::fs::TryLockError::WouldBlock) => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            format!("{}: another forge-server serves this folder", root.display()),
        )),
        Err(std::fs::TryLockError::Error(e)) => Err(e),
    }
}

impl FileLogStore {
    pub fn new(dir: PathBuf) -> io::Result<FileLogStore> {
        std::fs::create_dir_all(&dir)?;
        Ok(FileLogStore {
            dir,
            open: HashMap::new(),
            loaded: HashMap::new(),
        })
    }

    fn path(&self, doc: &str) -> PathBuf {
        self.dir.join(format!("{}.log", forge_history::names::enc(doc)))
    }

    fn bytes(payload: &[u8]) -> Vec<u8> {
        let mut b = Vec::with_capacity(payload.len() + HEAD);
        b.extend_from_slice(MAGIC);
        b.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        b.extend_from_slice(&fnv(payload).to_le_bytes());
        b.extend_from_slice(payload);
        b
    }

    fn file(&mut self, doc: &str) -> io::Result<&mut File> {
        if !self.open.contains_key(doc) {
            let f = OpenOptions::new().append(true).open(self.path(doc))?;
            self.open.insert(doc.to_string(), f);
        }
        Ok(self.open.get_mut(doc).expect("just opened"))
    }
}

fn fnv(b: &[u8]) -> u64 {
    // the same function as forge-sync's text hash, over bytes
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn corrupt(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why)
}

/// A whole, valid record at `at`: its payload's range.
fn record_at(bytes: &[u8], at: usize) -> Option<std::ops::Range<usize>> {
    let h = bytes.get(at..at + HEAD)?;
    if &h[..4] != MAGIC {
        return None;
    }
    let len = u32::from_le_bytes(h[4..8].try_into().ok()?) as usize;
    let sum = u64::from_le_bytes(h[8..16].try_into().ok()?);
    let p = bytes.get(at + HEAD..at + HEAD + len)?;
    (len <= MAX_RECORD as usize && fnv(p) == sum).then_some(at + HEAD..at + HEAD + len)
}

/// True if a valid record starts anywhere in `bytes[from..]` (then what precedes it is not a
/// torn tail but corruption).
fn any_record_after(bytes: &[u8], from: usize) -> bool {
    let mut k = from;
    while let Some(off) = bytes.get(k..).and_then(|b| b.windows(4).position(|w| w == MAGIC)) {
        if record_at(bytes, k + off).is_some() {
            return true;
        }
        k += off + 1;
    }
    false
}

impl FileLogStore {
    /// The whole log: (snapshot seq, its text, the entries after it).
    fn load(&mut self, doc: &str) -> io::Result<Option<(u64, String, Vec<Entry>)>> {
        let p = self.path(doc);
        let mut f = match OpenOptions::new().read(true).write(true).open(&p) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let mut bytes = vec![];
        f.read_to_end(&mut bytes)?;
        let (mut at, mut first, mut entries) = (0usize, None, vec![]);
        while at < bytes.len() {
            let Some(r) = record_at(&bytes, at) else {
                // not a whole valid record here: a torn tail only if nothing valid follows and
                // what is here is a prefix of a record (its marker, so far as it goes)
                // a tail of zeros: a crash after the file grew but before its data was written
                // (ext4 writeback); never acknowledged
                if bytes[at..].iter().all(|&b| b == 0) {
                    break;
                }
                let prefix_ok = bytes[at..].iter().zip(MAGIC).all(|(a, b)| a == b);
                let runs_past_end = bytes.len() - at < HEAD
                    || at + HEAD + u32::from_le_bytes(bytes[at + 4..at + 8].try_into().expect("4")) as usize
                        > bytes.len();
                if prefix_ok && runs_past_end && !any_record_after(&bytes, at + 1) {
                    break;
                }
                return Err(corrupt(format!("{}: a bad record at byte {at}", p.display())));
            };
            let payload = &bytes[r.clone()];
            let text = std::str::from_utf8(payload)
                .map_err(|_| corrupt(format!("{}: not UTF-8 at {at}", p.display())))?;
            let j = read(text).map_err(|e| corrupt(format!("{}: {e} at {at}", p.display())))?;
            if first.is_none() {
                let text = j
                    .get("first")
                    .and_then(Json::str)
                    .ok_or_else(|| corrupt(format!("{}: no first text", p.display())))?
                    .to_string();
                let seq = j.get("seq").and_then(Json::num).map_or(0, |x| x as u64);
                first = Some((seq, text));
            } else {
                entries.push(
                    entry_from(&j).ok_or_else(|| corrupt(format!("{}: a bad entry at {at}", p.display())))?,
                );
            }
            at = r.end;
        }
        if at < bytes.len() {
            // a crash mid-append: that record was never acknowledged
            eprintln!(
                "forge-server: {}: cut off {} bytes of an unfinished record",
                p.display(),
                bytes.len() - at
            );
            f.set_len(at as u64)?;
            f.sync_data()?;
        }
        f.seek(SeekFrom::End(0))?;
        match first {
            Some((seq, text)) => Ok(Some((seq, text, entries))),
            // never written whole (a log from before atomic creation): no document yet
            None => {
                drop(f);
                std::fs::remove_file(&p)?;
                Ok(None)
            }
        }
    }
}

impl Store for FileLogStore {
    fn latest_snapshot(&mut self, doc: &str) -> io::Result<Option<(u64, String)>> {
        let got = self.load(doc)?;
        let snap = got.as_ref().map(|(s, t, _)| (*s, t.clone()));
        // kept for the read_since that follows (an open reads both)
        if let Some((s, _, es)) = got {
            self.loaded.insert(doc.to_string(), (s, es));
        }
        Ok(snap)
    }

    fn read_since(&mut self, doc: &str, seq: u64) -> io::Result<Vec<Entry>> {
        let es = match self.loaded.remove(doc) {
            Some((_, es)) => es,
            None => self.load(doc)?.map(|x| x.2).unwrap_or_default(),
        };
        Ok(es.into_iter().filter(|e| e.seq > seq).collect())
    }

    fn compact(&mut self, doc: &str, seq: u64, text: &str) -> io::Result<()> {
        let j = Json::obj(vec![
            ("first", text.into()),
            ("seq", (seq as f64).into()),
            ("th", format!("{:016x}", text_hash(text)).into()),
        ]);
        // the new log replaces the old whole (renamed over it): after a crash, one or the other
        self.open.remove(doc);
        crate::fsx::put(&self.path(doc), &Self::bytes(write(&j).as_bytes()))?;
        if let Ok(d) = File::open(&self.dir) {
            let _ = d.sync_all();
        }
        Ok(())
    }

    fn create(&mut self, doc: &str, text: &str) -> io::Result<()> {
        let p = self.path(doc);
        if p.exists() {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, doc.to_string()));
        }
        let j = Json::obj(vec![
            ("first", text.into()),
            ("th", format!("{:016x}", text_hash(text)).into()),
        ]);
        // whole or not at all: never an empty log under the real name
        crate::fsx::put(&p, &Self::bytes(write(&j).as_bytes()))?;
        if let Ok(d) = File::open(&self.dir) {
            let _ = d.sync_all();
        }
        self.open.remove(doc);
        Ok(())
    }

    fn append(&mut self, doc: &str, e: &Entry) -> io::Result<()> {
        let b = Self::bytes(write(&entry_json(e)).as_bytes());
        let f = self.file(doc)?;
        let before = f.metadata()?.len();
        let r = f.write_all(&b).and_then(|_| f.sync_data());
        if r.is_err() {
            // a part written (a full disk): cut back, so the log stays whole and appendable
            let _ = f.set_len(before);
            let _ = f.sync_data();
            self.open.remove(doc);
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_lang::error::Op;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("forge-server-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn e(seq: u64) -> Entry {
        Entry {
            seq,
            author: "dev".into(),
            client: "c".into(),
            id: seq,
            base: seq - 1,
            op: Op::Rename {
                from: format!("a{seq}"),
                to: format!("a{}", seq + 1),
            },
            inv: None,
            th: seq * 7,
        }
    }

    fn log_of(d: &Path) -> PathBuf {
        d.join(format!("{}.log", forge_history::names::enc("p/a.fg")))
    }

    fn made(name: &str) -> (PathBuf, FileLogStore) {
        let d = tmp(name);
        let mut s = FileLogStore::new(d.clone()).unwrap();
        assert!(s.load("p/a.fg").unwrap().is_none());
        s.create("p/a.fg", "shared a1 = 1\n").unwrap();
        for k in 1..=3 {
            s.append("p/a.fg", &e(k)).unwrap();
        }
        (d, s)
    }

    #[test]
    fn round_trip_and_a_torn_tail_is_cut() {
        let (d, s) = made("torn");
        drop(s);
        // a crash in the middle of a fourth append: a record's head and part of its payload
        let whole = FileLogStore::bytes(b"{\"x\":1}");
        let mut f = OpenOptions::new().append(true).open(log_of(&d)).unwrap();
        f.write_all(&whole[..HEAD + 3]).unwrap();
        drop(f);
        let mut s = FileLogStore::new(d.clone()).unwrap();
        let (_, first, es) = s.load("p/a.fg").unwrap().unwrap();
        assert_eq!(first, "shared a1 = 1\n");
        assert_eq!(es, vec![e(1), e(2), e(3)]);
        s.append("p/a.fg", &e(4)).unwrap();
        let mut s = FileLogStore::new(d.clone()).unwrap();
        assert_eq!(s.load("p/a.fg").unwrap().unwrap().2.len(), 4);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_zero_tail_is_torn_and_compaction_keeps_the_head() {
        let (d, s) = made("zeros");
        drop(s);
        let mut f = OpenOptions::new().append(true).open(log_of(&d)).unwrap();
        f.write_all(&[0; 300]).unwrap();
        drop(f);
        let mut s = FileLogStore::new(d.clone()).unwrap();
        assert_eq!(s.load("p/a.fg").unwrap().unwrap().2.len(), 3);
        s.compact("p/a.fg", 3, "shared a4 = 1\n").unwrap();
        s.append("p/a.fg", &e(4)).unwrap();
        let mut s = FileLogStore::new(d.clone()).unwrap();
        assert_eq!(
            s.latest_snapshot("p/a.fg").unwrap(),
            Some((3, "shared a4 = 1\n".into()))
        );
        assert_eq!(s.read_since("p/a.fg", 3).unwrap(), vec![e(4)]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn corruption_is_never_cut_silently() {
        // a flipped payload byte: the checksum
        let (d, s) = made("flip");
        drop(s);
        let mut b = std::fs::read(log_of(&d)).unwrap();
        let k = b.len() - 5;
        b[k] ^= 1;
        std::fs::write(log_of(&d), &b).unwrap();
        assert!(FileLogStore::new(d.clone()).unwrap().load("p/a.fg").is_err());
        let _ = std::fs::remove_dir_all(&d);
        // the review's case: a mid-log length grown so the record runs past the end, whole
        // acknowledged records after it: corruption, not a torn tail
        let (d, s) = made("midlen");
        drop(s);
        let mut b = std::fs::read(log_of(&d)).unwrap();
        let first_len = u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
        let second = HEAD + first_len;
        let n = b.len() as u32;
        b[second + 4..second + 8].copy_from_slice(&n.to_le_bytes());
        std::fs::write(log_of(&d), &b).unwrap();
        let err = FileLogStore::new(d.clone()).unwrap().load("p/a.fg").unwrap_err();
        assert!(err.to_string().contains("bad record"), "{err}");
        assert_eq!(std::fs::read(log_of(&d)).unwrap(), b, "nothing was cut");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_empty_log_is_no_document_and_a_folder_has_one_server() {
        let d = tmp("empty");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(log_of(&d), b"").unwrap();
        let mut s = FileLogStore::new(d.clone()).unwrap();
        assert!(s.load("p/a.fg").unwrap().is_none());
        s.create("p/a.fg", "shared a = 1\n").unwrap();
        assert!(s.load("p/a.fg").unwrap().is_some());
        let held = lock(&d).unwrap();
        assert!(lock(&d).is_err(), "a second server on the folder");
        drop(held);
        assert!(lock(&d).is_ok());
        let _ = std::fs::remove_dir_all(&d);
    }
}

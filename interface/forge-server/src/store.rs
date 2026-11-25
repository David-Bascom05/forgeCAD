//! The durable store of phase 1: one append-only log file per document.
//!
//! Why a file log (DESIGN.md §5 keeps Postgres PROVISIONAL, FoundationDB and MongoDB unmeasured,
//! behind the `Store` trait): it is the fastest durable append measured (0.119 ms p50 with
//! fdatasync, MEASUREMENTS §d), needs no dependency (SQLite would bring rusqlite and C), and a
//! single server owns its documents in phase 1 (a folder lock makes sure of that: [`lock`]). Any
//! other store replaces it behind the same trait.
//!
//! Format, per document (`<dir>/<encoded name>.log`): records of
//! `[b"FGL1"][u32 LE payload length][u64 LE FNV-1a of the payload][payload]`. A record is a
//! snapshot (`{"first": TEXT, "seq": N, "seen": [..], "refused": [..]}`; the first record is the
//! document's first text) or an entry (forge-sync's `entry_json`). Each append is written whole
//! and `sync_data`ed before it returns; a new log is made under a temporary name and renamed into
//! place, so it is never seen empty.
//!
//! A snapshot is appended, never written over the log: the ops before it stay (the op log is the
//! document's live history, DESIGN.md §5). An open starts from the latest snapshot record and
//! replays the entries after it. Dropping old ops is a retention policy of its own, not this.
//!
//! On open: a record **cut short by the end of the file** with nothing valid after it (a crash
//! mid-append: never acknowledged) is cut off. Anything else wrong (a bad checksum, a bad marker,
//! a length that runs past the end while whole records follow it) is corruption: the document
//! refuses to open, and nothing acknowledged is ever dropped silently.

use forge_agent::json::Json;
use forge_sync::wire::{entry_from, entry_json, read, write};
use forge_sync::{Entry, Snapshot, Store, text_hash};
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
    fn load(&mut self, doc: &str) -> io::Result<Option<(Snapshot, Vec<Entry>)>> {
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
            if j.get("first").is_some() {
                // a snapshot: an open starts from the latest one
                let text = j
                    .get("first")
                    .and_then(Json::str)
                    .ok_or_else(|| corrupt(format!("{}: no first text", p.display())))?
                    .to_string();
                let seq = j.get("seq").and_then(Json::num).map_or(0, |x| x as u64);
                // the op ids committed before it: (client, id, seq) triples
                let seen = match j.get("seen") {
                    Some(Json::Arr(a)) => a
                        .iter()
                        .filter_map(|x| match x {
                            Json::Arr(t) if t.len() == 3 => {
                                Some((t[0].str()?.to_string(), t[1].num()? as u64, t[2].num()? as u64))
                            }
                            _ => None,
                        })
                        .collect(),
                    _ => vec![],
                };
                let refused = match j.get("refused") {
                    Some(Json::Arr(a)) => a
                        .iter()
                        .filter_map(|x| match x {
                            Json::Arr(t) if t.len() == 2 => {
                                Some((t[0].str()?.to_string(), t[1].num()? as u64))
                            }
                            _ => None,
                        })
                        .collect(),
                    _ => vec![],
                };
                first = Some(Snapshot {
                    seq,
                    text,
                    seen,
                    refused,
                });
                entries.clear();
            } else if first.is_none() {
                return Err(corrupt(format!("{}: no first text", p.display())));
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
            Some(snap) => Ok(Some((snap, entries))),
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
    fn latest_snapshot(&mut self, doc: &str) -> io::Result<Option<Snapshot>> {
        let got = self.load(doc)?;
        // the entries kept for the read_since that follows (an open reads both)
        Ok(got.map(|(snap, es)| {
            self.loaded.insert(doc.to_string(), (snap.seq, es));
            snap
        }))
    }

    fn read_since(&mut self, doc: &str, seq: u64) -> io::Result<Vec<Entry>> {
        let es = match self.loaded.remove(doc) {
            Some((_, es)) => es,
            None => self.load(doc)?.map(|x| x.1).unwrap_or_default(),
        };
        Ok(es.into_iter().filter(|e| e.seq > seq).collect())
    }

    fn compact(&mut self, doc: &str, snap: &Snapshot) -> io::Result<()> {
        let seen = snap
            .seen
            .iter()
            .map(|(c, i, s)| Json::Arr(vec![c.as_str().into(), (*i as f64).into(), (*s as f64).into()]))
            .collect();
        let refused = snap
            .refused
            .iter()
            .map(|(c, i)| Json::Arr(vec![c.as_str().into(), (*i as f64).into()]))
            .collect();
        let j = Json::obj(vec![
            ("first", snap.text.as_str().into()),
            ("seq", (snap.seq as f64).into()),
            ("th", format!("{:016x}", text_hash(&snap.text)).into()),
            ("seen", Json::Arr(seen)),
            ("refused", Json::Arr(refused)),
        ]);
        // appended like an entry (whole and synced, or cut back): the ops before it stay
        self.append_payload(doc, write(&j).as_bytes())
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
        self.append_payload(doc, write(&entry_json(e)).as_bytes())
    }
}

impl FileLogStore {
    fn append_payload(&mut self, doc: &str, payload: &[u8]) -> io::Result<()> {
        let b = Self::bytes(payload);
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

// sockets, threads, links and file locks: not in WASI (the wasm suite runs the workspace there)
#[cfg(all(test, not(target_os = "wasi")))]
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
        let (snap, es) = s.load("p/a.fg").unwrap().unwrap();
        assert_eq!(snap.text, "shared a1 = 1\n");
        assert_eq!(es, vec![e(1), e(2), e(3)]);
        s.append("p/a.fg", &e(4)).unwrap();
        let mut s = FileLogStore::new(d.clone()).unwrap();
        assert_eq!(s.load("p/a.fg").unwrap().unwrap().1.len(), 4);
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
        assert_eq!(s.load("p/a.fg").unwrap().unwrap().1.len(), 3);
        let snap = Snapshot {
            seq: 3,
            text: "shared a4 = 1\n".into(),
            seen: vec![("ada/c".into(), 7, 2)],
            refused: vec![("ada/c".into(), 5)],
        };
        s.compact("p/a.fg", &snap).unwrap();
        s.append("p/a.fg", &e(4)).unwrap();
        let mut s = FileLogStore::new(d.clone()).unwrap();
        assert_eq!(
            s.latest_snapshot("p/a.fg").unwrap(),
            Some(snap),
            "the dedup marks survive"
        );
        assert_eq!(s.read_since("p/a.fg", 3).unwrap(), vec![e(4)]);
        // the snapshot was appended: the ops before it are still in the log (the history)
        let b = std::fs::read(log_of(&d)).unwrap();
        let (mut at, mut n) = (0, 0);
        while let Some(r) = record_at(&b, at) {
            at = r.end;
            n += 1;
        }
        assert_eq!(
            (n, at),
            (6, b.len()),
            "first text, 3 entries, the snapshot, 1 entry"
        );
        // a second snapshot: an open starts from the latest
        let snap2 = Snapshot {
            seq: 4,
            text: "shared a5 = 1\n".into(),
            ..Default::default()
        };
        s.compact("p/a.fg", &snap2).unwrap();
        let mut s = FileLogStore::new(d.clone()).unwrap();
        assert_eq!(s.latest_snapshot("p/a.fg").unwrap(), Some(snap2));
        assert_eq!(s.read_since("p/a.fg", 4).unwrap(), vec![]);
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

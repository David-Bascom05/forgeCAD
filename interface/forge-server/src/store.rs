//! The durable store of phase 1: one append-only log file per document.
//!
//! Why a file log (DESIGN.md §5 keeps Postgres PROVISIONAL, FoundationDB and MongoDB unmeasured,
//! behind the `Store` trait): it is the fastest durable append measured (0.119 ms p50 with
//! fdatasync, MEASUREMENTS §d), needs no dependency (SQLite would bring rusqlite and C), and a
//! single server owns its documents in phase 1, so nothing needs a database's sharing yet. Any
//! other store replaces it behind the same trait.
//!
//! Format, per document (`<dir>/<encoded name>.log`): records of
//! `[u32 LE payload length][u64 LE FNV-1a of the payload][payload]`, the first record's payload
//! the document's first text (`{"first": TEXT}`), each next one an entry (forge-sync's
//! `entry_json`). Each append is written whole and `sync_data`ed before it returns. On open, a
//! record cut short at the end of the file (a crash mid-append: never acknowledged) is cut off;
//! a whole record whose checksum is wrong is corruption, and the document refuses to open.

use forge_agent::json::Json;
use forge_sync::wire::{entry_from, entry_json, read, write};
use forge_sync::{Entry, Store, text_hash};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

/// The most one record may hold (a document's first text is the largest).
pub const MAX_RECORD: u32 = 64 << 20;

pub struct FileLogStore {
    dir: PathBuf,
    open: HashMap<String, File>,
}

impl FileLogStore {
    pub fn new(dir: PathBuf) -> io::Result<FileLogStore> {
        std::fs::create_dir_all(&dir)?;
        Ok(FileLogStore {
            dir,
            open: HashMap::new(),
        })
    }

    fn path(&self, doc: &str) -> PathBuf {
        self.dir.join(format!("{}.log", forge_history::names::enc(doc)))
    }

    fn record(f: &mut File, payload: &[u8]) -> io::Result<()> {
        let mut b = Vec::with_capacity(payload.len() + 12);
        b.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        b.extend_from_slice(&fnv(payload).to_le_bytes());
        b.extend_from_slice(payload);
        f.write_all(&b)?;
        f.sync_data()
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

impl Store for FileLogStore {
    fn load(&mut self, doc: &str) -> io::Result<Option<(String, Vec<Entry>)>> {
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
            if bytes.len() - at < 12 {
                break; // a head cut short
            }
            let len = u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4")) as usize;
            let sum = u64::from_le_bytes(bytes[at + 4..at + 12].try_into().expect("8"));
            if len > MAX_RECORD as usize {
                return Err(corrupt(format!(
                    "{}: a record of {len} bytes at {at}",
                    p.display()
                )));
            }
            if bytes.len() - at - 12 < len {
                break; // a payload cut short
            }
            let payload = &bytes[at + 12..at + 12 + len];
            if fnv(payload) != sum {
                return Err(corrupt(format!(
                    "{}: the record at {at} fails its checksum",
                    p.display()
                )));
            }
            let text = std::str::from_utf8(payload)
                .map_err(|_| corrupt(format!("{}: not UTF-8 at {at}", p.display())))?;
            let j = read(text).map_err(|e| corrupt(format!("{}: {e} at {at}", p.display())))?;
            if first.is_none() {
                first = Some(
                    j.get("first")
                        .and_then(Json::str)
                        .ok_or_else(|| corrupt(format!("{}: no first text", p.display())))?
                        .to_string(),
                );
            } else {
                entries.push(
                    entry_from(&j).ok_or_else(|| corrupt(format!("{}: a bad entry at {at}", p.display())))?,
                );
            }
            at += 12 + len;
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
        let Some(first) = first else {
            return Ok(None);
        };
        Ok(Some((first, entries)))
    }

    fn create(&mut self, doc: &str, text: &str) -> io::Result<()> {
        let p = self.path(doc);
        let mut f = OpenOptions::new().write(true).create_new(true).open(&p)?;
        let j = Json::obj(vec![
            ("first", text.into()),
            ("th", format!("{:016x}", text_hash(text)).into()),
        ]);
        Self::record(&mut f, write(&j).as_bytes())?;
        // the new name durable too
        if let Ok(d) = File::open(&self.dir) {
            let _ = d.sync_all();
        }
        self.open.remove(doc);
        Ok(())
    }

    fn append(&mut self, doc: &str, e: &Entry) -> io::Result<()> {
        let payload = write(&entry_json(e));
        let f = self.file(doc)?;
        Self::record(f, payload.as_bytes())
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

    #[test]
    fn round_trip_and_a_torn_tail_is_cut() {
        let d = tmp("torn");
        let mut s = FileLogStore::new(d.clone()).unwrap();
        assert!(s.load("p/a.fg").unwrap().is_none());
        s.create("p/a.fg", "shared a1 = 1\n").unwrap();
        for k in 1..=3 {
            s.append("p/a.fg", &e(k)).unwrap();
        }
        drop(s);
        let p = d.join(format!("{}.log", forge_history::names::enc("p/a.fg")));
        // a crash in the middle of a fourth append
        let mut f = OpenOptions::new().append(true).open(&p).unwrap();
        f.write_all(&[200, 0, 0, 0, 1, 2, 3]).unwrap();
        drop(f);
        let mut s = FileLogStore::new(d.clone()).unwrap();
        let (first, es) = s.load("p/a.fg").unwrap().unwrap();
        assert_eq!(first, "shared a1 = 1\n");
        assert_eq!(es, vec![e(1), e(2), e(3)]);
        // appending goes on after the cut
        s.append("p/a.fg", &e(4)).unwrap();
        let mut s = FileLogStore::new(d.clone()).unwrap();
        assert_eq!(s.load("p/a.fg").unwrap().unwrap().1.len(), 4);
        // a flipped byte inside a whole record is corruption, never silently dropped
        let mut b = std::fs::read(&p).unwrap();
        let k = b.len() - 5;
        b[k] ^= 1;
        std::fs::write(&p, b).unwrap();
        let mut s = FileLogStore::new(d.clone()).unwrap();
        assert!(s.load("p/a.fg").unwrap_err().to_string().contains("checksum"));
        let _ = std::fs::remove_dir_all(&d);
    }
}

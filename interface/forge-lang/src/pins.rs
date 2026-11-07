//! Pinned parts: `part("bracket.fg", version: 3)` places the file as its named version 3 was
//! saved, read straight from the project's history. The pinned file stands at a path under the
//! project, `<project>/@forgecad-v3-<save>/bracket.fg`, that no disk holds: located and read
//! here, so its own `part(..)` and `use(..)` resolve in the same save, and the fence still
//! holds. A version's save never changes, so a pinned build is cached by its text like any other.

use forge_git::Oid;
use forge_history::History;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

const MARK: &str = "@forgecad-v";

/// A pinned file's place: the project, the version, its save, and the file's path in the project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    pub work: PathBuf,
    pub n: u32,
    pub save: String,
    pub rel: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PinError {
    /// The file's folder keeps no history.
    NoHistory,
    /// No such named version; the newest there is.
    NoVersion { newest: Option<u32> },
    /// The file is not inside its project.
    Outside,
}

fn histories() -> &'static Mutex<HashMap<PathBuf, Option<Arc<History>>>> {
    static H: OnceLock<Mutex<HashMap<PathBuf, Option<Arc<History>>>>> = OnceLock::new();
    H.get_or_init(Default::default)
}

/// The history of the project at or above `dir`, opened once per process.
pub fn history(dir: &Path) -> Option<Arc<History>> {
    let key = dir.to_path_buf();
    if let Some(h) = histories().lock().expect("histories").get(&key) {
        return h.clone();
    }
    let h = History::open(dir).map(Arc::new);
    histories().lock().expect("histories").insert(key, h.clone());
    h
}

/// Forgets opened histories (a test that makes a project where one was not).
pub fn forget() {
    histories().lock().expect("histories").clear();
}

/// `a/./b/../c` as `a/c`; None if it climbs above its start.
fn normal(parts: &[&str]) -> Option<String> {
    let mut out: Vec<&str> = vec![];
    for p in parts {
        match *p {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            x => out.push(x),
        }
    }
    Some(out.join("/"))
}

pub fn parse(p: &Path) -> Option<Pin> {
    let s = p.to_str()?;
    let at = s.find(&format!("/{MARK}"))?;
    let work = PathBuf::from(&s[..at]);
    let rest = &s[at + 1 + MARK.len()..];
    let (head, rel) = rest.split_once('/')?;
    let (n, save) = head.split_once('-')?;
    if save.len() < 40 || !save.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let parts: Vec<&str> = rel.split('/').collect();
    Some(Pin {
        work,
        n: n.parse().ok()?,
        save: save.to_string(),
        rel: normal(&parts)?,
    })
}

pub fn path_of(pin: &Pin) -> PathBuf {
    pin.work
        .join(format!("{MARK}{}-{}", pin.n, pin.save))
        .join(&pin.rel)
}

pub fn is_pinned(p: &str) -> bool {
    p.contains(&format!("/{MARK}"))
}

/// `file` (as the document beside `dir` names it) pinned to its version `n`.
pub fn pin(dir: &Path, file: &str, n: u32) -> Result<PathBuf, PinError> {
    let (h, rel) = place(dir, file)?;
    let vs = h.versions(Some(&rel)).map_err(|_| PinError::NoHistory)?;
    let Some(v) = vs.iter().find(|v| v.n == n) else {
        return Err(PinError::NoVersion {
            newest: vs.iter().map(|v| v.n).max(),
        });
    };
    Ok(path_of(&Pin {
        work: h.work().to_path_buf(),
        n,
        save: v.save.hex(),
        rel,
    }))
}

/// The project and the project path of `file` as a document in `dir` names it (a pinned
/// document's names are its project's too).
fn place(dir: &Path, file: &str) -> Result<(Arc<History>, String), PinError> {
    if let Some(p) = parse(&dir.join("x")) {
        let h = history(&p.work).ok_or(PinError::NoHistory)?;
        let mut parts: Vec<&str> = p.rel.split('/').collect();
        parts.pop();
        parts.extend(file.split('/'));
        return Ok((h, normal(&parts).ok_or(PinError::Outside)?));
    }
    let h = history(dir).ok_or(PinError::NoHistory)?;
    let abs = lexical(&dir.join(file)).ok_or(PinError::Outside)?;
    let rel = abs
        .strip_prefix(h.work())
        .map_err(|_| PinError::Outside)?
        .to_string_lossy()
        .to_string();
    Ok((h, rel))
}

fn lexical(p: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::CurDir => {}
            o => out.push(o),
        }
    }
    // the folders above the file resolved as the filesystem has them (links), the file not
    let parent = out.parent()?;
    let real = std::fs::canonicalize(parent).unwrap_or(parent.to_path_buf());
    Some(real.join(out.file_name()?))
}

/// The pinned file's text at its save, if the save holds it.
pub fn read(p: &Path) -> Option<String> {
    let pin = parse(p)?;
    let h = history(&pin.work)?;
    let save = Oid::from_hex(h.repo.hash, &pin.save)?;
    let data = h.repo.file_at(&save, &pin.rel).ok()??;
    String::from_utf8(data).ok()
}

/// Whether a pinned path names a file its save holds (located as `asm::locate` locates files).
pub fn locate(p: &Path) -> Option<PathBuf> {
    let pin = parse(p)?;
    read(p)?;
    Some(path_of(&pin))
}

/// The newest named version of the file a document beside `dir` names, for the badge.
pub fn newest(dir: &Path, file: &str) -> Option<u32> {
    let (h, rel) = place(dir, file).ok()?;
    h.newest(&rel).ok()?.map(|v| v.n)
}

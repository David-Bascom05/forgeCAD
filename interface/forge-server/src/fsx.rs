//! The project folder, carefully: which file a document name is, and writing one.
//!
//! A document's name must be **exactly** its path relative to the folder as the folder spells it:
//! no empty or `.` components, no `..`, no dot names, no link anywhere on the way (a link inside
//! the folder to `.git/config` or `.env` would serve it to everyone), and every component's case
//! as on disk (on a case-insensitive disk `Part.fg` and `part.fg` are one file: two rooms for it
//! would diverge, and their logs interleave). So one file has one name, and one room.

use std::io::Write;
use std::path::{Path, PathBuf};

/// The file a document name is, if the name is that file's one canonical spelling.
pub fn canonical_doc(root: &Path, doc: &str) -> Result<PathBuf, &'static str> {
    if doc.is_empty() || doc.len() > 512 || !doc.ends_with(".fg") {
        return Err("a document is a .fg path");
    }
    if !doc
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"_-./".contains(&b))
    {
        return Err("a document's path is letters, digits and _ - . /");
    }
    let mut at = root.to_path_buf();
    for part in doc.split('/') {
        if part.is_empty() || part == "." || part == ".." || part.starts_with('.') {
            return Err("a document's path stays inside the project");
        }
        // the folder must hold an entry spelt exactly so (case included)
        let exact = std::fs::read_dir(&at)
            .map_err(|_| "no such document")?
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_str() == Some(part));
        if !exact {
            return Err("no such document");
        }
        at.push(part);
        let meta = std::fs::symlink_metadata(&at).map_err(|_| "no such document")?;
        if meta.file_type().is_symlink() {
            return Err("a document's path stays inside the project (no links)");
        }
    }
    let meta = std::fs::symlink_metadata(&at).map_err(|_| "no such document")?;
    if !meta.is_file() {
        return Err("no such document");
    }
    // a hard link is a second name for a file that may be anywhere (a `.git/config`, say)
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.nlink() != 1 {
            return Err("a document's path stays inside the project (no links)");
        }
    }
    // and, belt and braces, the resolved file is inside the resolved folder
    let real = at.canonicalize().map_err(|_| "no such document")?;
    let real_root = root.canonicalize().map_err(|_| "the project folder is gone")?;
    if !real.starts_with(&real_root) {
        return Err("a document's path stays inside the project");
    }
    Ok(at)
}

/// A random name part (the OS's CSPRNG device where there is one).
pub fn random_hex() -> String {
    crate::random_token()
}

/// Written whole or not at all: a new file beside it with a name no one can guess (created new:
/// a link or file already there is refused, never written through), synced, renamed over it.
pub fn put(p: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = p.parent().ok_or(std::io::ErrorKind::InvalidInput)?;
    let name = p
        .file_name()
        .ok_or(std::io::ErrorKind::InvalidInput)?
        .to_string_lossy();
    for _ in 0..16 {
        let tmp = dir.join(format!(".{name}.forge-server-{}", random_hex()));
        let mut f = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let written = f.write_all(bytes).and_then(|_| f.sync_all());
        drop(f);
        return written.and_then(|_| std::fs::rename(&tmp, p)).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        });
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "no free temporary name",
    ))
}

/// The hash of the text the server last wrote to (or first read from) each document's file:
/// a file whose text differs was changed outside the server (an editor, a `git pull`), and the
/// server refuses to open it rather than silently overwrite the change. (Importing such a
/// change as ops is DESIGN.md §15.4, phase 4.)
pub struct DiskMarks {
    dir: PathBuf,
}

impl DiskMarks {
    pub fn new(dir: PathBuf) -> std::io::Result<DiskMarks> {
        std::fs::create_dir_all(&dir)?;
        Ok(DiskMarks { dir })
    }
    fn path(&self, doc: &str) -> PathBuf {
        self.dir.join(format!("{}.disk", forge_history::names::enc(doc)))
    }
    pub fn get(&self, doc: &str) -> Option<u64> {
        let t = std::fs::read_to_string(self.path(doc)).ok()?;
        u64::from_str_radix(t.trim(), 16).ok()
    }
    pub fn set(&self, doc: &str, text: &str) -> std::io::Result<()> {
        put(
            &self.path(doc),
            format!("{:016x}\n", forge_sync::text_hash(text)).as_bytes(),
        )
    }
}

// sockets, threads, links and file locks: not in WASI (the wasm suite runs the workspace there)
#[cfg(all(test, not(target_os = "wasi")))]
mod tests {
    use super::*;

    #[test]
    fn one_file_one_name() {
        let d = std::env::temp_dir().join(format!("forge-fsx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("a")).unwrap();
        std::fs::write(d.join("a/b.fg"), "x").unwrap();
        std::fs::write(d.join(".env.fg"), "secret").unwrap();
        assert!(canonical_doc(&d, "a/b.fg").is_ok());
        for bad in [
            "a//b.fg",
            "./a/b.fg",
            "a/./b.fg",
            "A/b.fg",
            "a/B.fg",
            "a/../a/b.fg",
            ".env.fg",
            "/a/b.fg",
            "a/b.fg/",
        ] {
            assert!(canonical_doc(&d, bad).is_err(), "{bad}");
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(d.join(".env.fg"), d.join("notes.fg")).unwrap();
            std::os::unix::fs::symlink(d.join("a"), d.join("l")).unwrap();
            assert!(canonical_doc(&d, "notes.fg").is_err(), "a link to a dot file");
            assert!(canonical_doc(&d, "l/b.fg").is_err(), "a linked folder");
            std::fs::hard_link(d.join(".env.fg"), d.join("hard.fg")).unwrap();
            assert!(canonical_doc(&d, "hard.fg").is_err(), "a hard link");
            // put never writes through a planted link
            put(&d.join("a/c.fg"), b"new").unwrap();
            assert_eq!(std::fs::read(d.join("a/c.fg")).unwrap(), b"new");
        }
        let _ = std::fs::remove_dir_all(&d);
    }
}

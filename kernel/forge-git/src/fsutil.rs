//! Writes that never leave a half-written file where git looks: a temp file beside the target,
//! synced, then renamed; and git's lock protocol (`<file>.lock`, made exclusively) for files that
//! are replaced (refs, the index).

use crate::Error;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_beside(path: &Path) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    path.with_file_name(format!("tmp_obj_{}_{n}_{t}", std::process::id()))
}

/// Writes a file that must not change once there (an object): no-op if it exists.
pub fn write_new(path: &Path, data: &[u8]) -> Result<(), Error> {
    if path.exists() {
        return Ok(());
    }
    let tmp = temp_beside(path);
    let r = (|| {
        let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        f.write_all(data)?;
        f.sync_data()?;
        drop(f);
        // the same content under the same name: a racing writer's copy is as good as ours
        std::fs::rename(&tmp, path)
    })();
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r.map_err(Error::from)
}

/// `<file>.lock`, held: committed by renaming over the file, or dropped (and removed).
pub struct Lock {
    target: PathBuf,
    lock: PathBuf,
    file: Option<File>,
}

impl Lock {
    pub fn take(target: &Path) -> Result<Lock, Error> {
        let mut name = target
            .file_name()
            .ok_or(Error::Corrupt("lock path"))?
            .to_os_string();
        name.push(".lock");
        let lock = target.with_file_name(name);
        if let Some(d) = lock.parent() {
            std::fs::create_dir_all(d)?;
        }
        match OpenOptions::new().write(true).create_new(true).open(&lock) {
            Ok(f) => Ok(Lock {
                target: target.to_path_buf(),
                lock,
                file: Some(f),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(Error::Locked(lock)),
            Err(e) => Err(e.into()),
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        self.file.as_mut().expect("open").write_all(data)?;
        Ok(())
    }

    pub fn commit(mut self) -> Result<(), Error> {
        let f = self.file.take().expect("open");
        f.sync_data()?;
        drop(f);
        std::fs::rename(&self.lock, &self.target)?;
        Ok(())
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        if self.file.take().is_some() {
            let _ = std::fs::remove_file(&self.lock);
        }
    }
}

/// Appends a line to a log, creating it and its directories.
pub fn append(path: &Path, line: &[u8]) -> Result<(), Error> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(line)?;
    Ok(())
}

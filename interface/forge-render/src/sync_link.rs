//! A SERVED part's link to the hosted server (research/hosted/DESIGN.md §14–§15).
//!
//! A project is SERVED iff a folder at or above the part holds `.forgecad/remote.toml`; [`open`]
//! looks **once, when the part opens**. A LOCAL part gets no link: its session's `sync` stays
//! None, and the edit path runs exactly as before (one `Option` test after a successful edit).
//!
//! The [`Link`] trait is what the part's session needs, the same on both hosts: natively a
//! forge-remote session (its own thread and socket), in the browser the page's WebSocket.

use forge_lang::error::Op;
use forge_sync::Event;
use std::sync::Arc;

pub trait Link {
    /// True once the server's snapshot arrived.
    fn ready(&self) -> bool;
    /// The text to show: the server's with this client's pending ops on it.
    fn text(&self) -> String;
    /// The last seq applied from the server.
    fn seq(&self) -> u64;
    /// Ops sent, not yet acked or refused.
    fn pending(&self) -> usize;
    /// The app applied and checked `op` and got `after`: share it.
    fn local(&self, op: Op, after: String);
    /// Hosted undo: the inverse of this client's last op, rebased (others' edits stay). None:
    /// nothing to undo; Err: it no longer applies.
    fn undo(&self) -> Option<Result<(), String>>;
    /// The exact hash of the body built from the server's text at `seq`.
    fn built(&self, seq: u64, hash: u64);
    /// A named version of the head (a forge-history commit and tag on the server).
    fn version(&self, name: &str);
    /// What happened since the last poll.
    fn poll(&self) -> Vec<Event>;
}

#[cfg(not(target_arch = "wasm32"))]
impl Link for forge_remote::Session {
    fn ready(&self) -> bool {
        forge_remote::Session::ready(self)
    }
    fn text(&self) -> String {
        forge_remote::Session::text(self)
    }
    fn seq(&self) -> u64 {
        forge_remote::Session::seq(self)
    }
    fn pending(&self) -> usize {
        forge_remote::Session::pending(self)
    }
    fn local(&self, op: Op, after: String) {
        forge_remote::Session::local(self, op, after);
    }
    fn undo(&self) -> Option<Result<(), String>> {
        forge_remote::Session::undo(self).map(|r| r.map(|_| ()).map_err(|e| format!("{}: {}", e.kind(), e.text())))
    }
    fn built(&self, seq: u64, hash: u64) {
        forge_remote::Session::built(self, seq, hash)
    }
    fn version(&self, name: &str) {
        forge_remote::Session::version(self, name)
    }
    fn poll(&self) -> Vec<Event> {
        forge_remote::Session::poll(self)
    }
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
/// The folder at or above `file`'s that holds `.forgecad/remote.toml`, and the file's path
/// relative to it (the document's name on the server, `/`-separated).
pub fn served_root(file: &std::path::Path) -> Option<(std::path::PathBuf, String)> {
    let file = forge_dirs::fs::canonicalize(file).ok()?;
    let mut dir = file.parent()?;
    loop {
        if dir.join(".forgecad").join("remote.toml").is_file() {
            let rel = file.strip_prefix(dir).ok()?;
            let parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
            return Some((dir.to_path_buf(), parts.join("/")));
        }
        dir = dir.parent()?;
    }
}

/// The part at `path`'s link, if its project is SERVED: None for LOCAL, Err for a remote.toml
/// that is there but wrong (said, never silently LOCAL).
#[cfg(not(target_arch = "wasm32"))]
pub fn open(path: &str, waker: Arc<dyn Fn() + Send + Sync>) -> Option<Result<Box<dyn Link>, String>> {
    let (root, doc) = served_root(std::path::Path::new(path))?;
    let remote = match forge_remote::remote(&root)? {
        Ok(r) => r,
        Err(e) => return Some(Err(e)),
    };
    Some(forge_remote::Session::start(&remote, &doc, Some(waker)).map(|s| Box::new(s) as Box<dyn Link>))
}

/// The browser build's link: the page's WebSocket, if the page said this tab is served.
#[cfg(target_arch = "wasm32")]
pub fn open(path: &str, waker: Arc<dyn Fn() + Send + Sync>) -> Option<Result<Box<dyn Link>, String>> {
    crate::websync::open(path, waker)
}

/// A SERVED part's file (native): the app writes the server's text to it (at most once a
/// second, only with nothing pending) and keeps the hash of what it wrote in
/// `<project>/.forgecad/sync/<doc>.hash`; a file whose text has another hash was edited by hand,
/// and the document stops syncing, the edit kept (DESIGN.md §15.4: detect and pause in phase 1).
#[cfg(not(target_arch = "wasm32"))]
pub struct SyncFile {
    path: std::path::PathBuf,
    mark: std::path::PathBuf,
    last: u64,
    checked: std::time::Instant,
    /// Set when opening found a hand edit.
    pub paused: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl SyncFile {
    /// The file of a part being opened with its text `src` (None: not under a served folder).
    pub fn open(file: &std::path::Path, src: &str) -> Option<SyncFile> {
        let (root, doc) = served_root(file)?;
        let mark = root
            .join(".forgecad")
            .join("sync")
            .join(format!("{}.hash", forge_history::names::enc(&doc)));
        let now = forge_sync::text_hash(src);
        let kept = std::fs::read_to_string(&mark)
            .ok()
            .and_then(|t| u64::from_str_radix(t.trim(), 16).ok());
        let mut f = SyncFile {
            path: file.to_path_buf(),
            mark,
            last: now,
            checked: std::time::Instant::now(),
            paused: None,
        };
        match kept {
            Some(h) if h != now => {
                f.paused = Some(format!(
                    "{doc} was edited outside the app since it last synced: not synced, your edit kept \
                     (bring it in by hand; merging such edits comes later)"
                ));
            }
            Some(_) => {}
            None => {
                let _ = f.write_mark(now);
            }
        }
        Some(f)
    }

    fn write_mark(&self, h: u64) -> std::io::Result<()> {
        if let Some(d) = self.mark.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(&self.mark, format!("{h:016x}\n"))
    }

    /// Once a second: a hand edit is Err (pause); else, when `settled`, the server's text is
    /// written if it changed.
    pub fn tick(&mut self, settled: bool, text: impl FnOnce() -> String) -> Result<(), String> {
        if self.checked.elapsed() < std::time::Duration::from_secs(1) {
            return Ok(());
        }
        self.checked = std::time::Instant::now();
        let on_disk = std::fs::read_to_string(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
        if forge_sync::text_hash(&on_disk) != self.last {
            return Err(format!(
                "{} was edited outside the app: syncing paused, your edit kept",
                self.path.display()
            ));
        }
        if settled {
            let t = text();
            let h = forge_sync::text_hash(&t);
            if h != self.last {
                // whole or not at all: a file beside it, renamed over it
                let tmp = self.path.with_extension("fg.forgecad-sync");
                std::fs::write(&tmp, &t)
                    .and_then(|_| std::fs::rename(&tmp, &self.path))
                    .map_err(|e| format!("{}: {e}", self.path.display()))?;
                self.last = h;
                self.write_mark(h).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}

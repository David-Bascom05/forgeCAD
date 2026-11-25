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

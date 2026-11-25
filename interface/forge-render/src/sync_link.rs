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
    /// The app applied and checked `op` and got `after`: share it. Err: not shared (the link
    /// has not joined yet, or the edit is outside the protocol's limits): why, in words.
    fn local(&self, op: Op, after: String) -> Result<(), String>;
    /// The app applied several ops as one edit (an apply-all) and checked the result, `after`:
    /// share them as one group, which the server commits whole or not at all. Err as `local`.
    fn local_group(&self, ops: Vec<Op>, after: String) -> Result<(), String>;
    /// Hosted undo: the inverse of this client's last op, rebased (others' edits stay). None:
    /// nothing to undo; Err: it no longer applies.
    fn undo(&self) -> Option<Result<(), String>>;
    /// The exact hash of the body built from the server's text at `seq`.
    fn built(&self, seq: u64, hash: u64);
    /// A named version of the head (a forge-history commit and tag on the server).
    fn version(&self, name: &str);
    /// What happened since the last poll.
    fn poll(&self) -> Vec<Event>;
    /// The client as it must outlive the app (its id, op ids, pending ops); empty where the
    /// host keeps none.
    fn saved(&self) -> String;
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
    fn local(&self, op: Op, after: String) -> Result<(), String> {
        forge_remote::Session::local(self, op, after).map(|_| ()).map_err(|r| r.text())
    }
    fn local_group(&self, ops: Vec<Op>, after: String) -> Result<(), String> {
        forge_remote::Session::local_group(self, ops, after)
            .map(|_| ())
            .map_err(|r| r.text())
    }
    fn undo(&self) -> Option<Result<(), String>> {
        forge_remote::Session::undo(self)
            .map(|r| r.map(|_| ()).map_err(|e| format!("{}: {}", e.kind(), e.text())))
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
    fn saved(&self) -> String {
        forge_remote::Session::saved(self)
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
            let parts: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            return Some((dir.to_path_buf(), parts.join("/")));
        }
        dir = dir.parent()?;
    }
}

/// The part at `path`'s link, if its project is SERVED: None for LOCAL, Err for a remote.toml
/// that is there but wrong (said, never silently LOCAL). The client the app kept when it last
/// closed (`.forgecad/sync/<doc>.pending`) is taken up: its unacked ops are sent again.
#[cfg(not(target_arch = "wasm32"))]
pub fn open(
    path: &str,
    waker: Arc<dyn Fn() + Send + Sync>,
    kept: Option<String>,
) -> Option<Result<Box<dyn Link>, String>> {
    let (root, doc) = served_root(std::path::Path::new(path))?;
    // the folder a forge-server serves is its own: the app editing it too would write the
    // server's files behind it (and the server's behind the app). Open a clone of the project.
    if root.join(".forgecad").join("server").is_dir() {
        return Some(Err(format!(
            "{} is the folder a forge-server serves: open a clone of the project instead (docs/hosted.md)",
            root.display()
        )));
    }
    let remote = match forge_remote::remote(&root)? {
        Ok(r) => r,
        Err(e) => return Some(Err(e)),
    };
    Some(
        forge_remote::Session::resume(&remote, &doc, kept.as_deref(), Some(waker))
            .map(|s| Box::new(s) as Box<dyn Link>),
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn sync_dir(root: &std::path::Path) -> std::path::PathBuf {
    root.join(".forgecad").join("sync")
}

/// `bytes` written whole or not at all (a file beside it, renamed over it).
#[cfg(not(target_arch = "wasm32"))]
fn put(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".forgecad-sync");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// The browser build's link: the page's WebSocket, if the page said this tab is served.
#[cfg(target_arch = "wasm32")]
pub fn open(
    path: &str,
    waker: Arc<dyn Fn() + Send + Sync>,
    _kept: Option<String>,
) -> Option<Result<Box<dyn Link>, String>> {
    // (the browser keeps no client across a closed tab yet)
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
    /// The client kept for the next open, and what was last written there.
    pending: std::path::PathBuf,
    kept: String,
    last: u64,
    checked: std::time::Instant,
    /// The file is known to hold the server's text (a mark it matches, or [`verify`]). Until
    /// then nothing is written to it, and the app does not attach (DESIGN.md §15.2).
    ///
    /// [`verify`]: SyncFile::verify
    verified: bool,
    /// Set when opening found a hand edit.
    pub paused: Option<String>,
    /// This app's hold on the document's kept client (`<doc>.pending.lock`): a second instance
    /// (a second window, a copied folder) neither takes it up nor writes it, or the two would
    /// share one client id and op ids (and one's edits be answered as the other's retries).
    owner: Option<std::fs::File>,
}

#[cfg(not(target_arch = "wasm32"))]
impl SyncFile {
    /// The file of a part being opened with its text `src` (None: not under a served folder).
    pub fn open(file: &std::path::Path, src: &str) -> Option<SyncFile> {
        let (root, doc) = served_root(file)?;
        let name = forge_history::names::enc(&doc);
        let mark = sync_dir(&root).join(format!("{name}.hash"));
        let pending = sync_dir(&root).join(format!("{name}.pending"));
        let now = forge_sync::text_hash(src);
        let kept = std::fs::read_to_string(&mark)
            .ok()
            .and_then(|t| u64::from_str_radix(t.trim(), 16).ok());
        let owner = std::fs::create_dir_all(sync_dir(&root)).ok().and_then(|_| {
            let f = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(sync_dir(&root).join(format!("{name}.pending.lock")))
                .ok()?;
            f.try_lock().ok().map(|_| f)
        });
        let mut f = SyncFile {
            owner,
            path: file.to_path_buf(),
            mark,
            pending,
            kept: String::new(),
            last: now,
            checked: std::time::Instant::now(),
            verified: kept == Some(now),
            paused: None,
        };
        if kept.is_some_and(|h| h != now) {
            f.paused = Some(format!(
                "{doc} was edited outside the app since it last synced: not synced, your edit kept \
                 (bring it in by hand; merging such edits comes later)"
            ));
        }
        // no mark (the first open since the project was served): attached only once the
        // server's text is seen to be the file's (`verify`)
        Some(f)
    }

    /// The file holds what the server last had (a mark it matches): the link attaches now.
    pub fn verified(&self) -> bool {
        self.verified
    }

    /// The first open of a served part: the server's text (with this client's pending ops),
    /// the file's and the editor's must be the same, or nothing is attached and the file is
    /// left as it is (an edit made before the project was served is never written over).
    pub fn verify(&mut self, server: &str, editor: &str) -> Result<(), String> {
        let on_disk =
            std::fs::read_to_string(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
        let h = forge_sync::text_hash(server);
        if forge_sync::text_hash(&on_disk) != h || forge_sync::text_hash(editor) != h {
            return Err(format!(
                "{} differs from the server's copy: not synced, your file kept (bring it in by hand, \
                 or make it the server's text, then open it again)",
                self.path.display()
            ));
        }
        self.write_mark(h).map_err(|e| e.to_string())?;
        self.last = h;
        self.verified = true;
        Ok(())
    }

    /// The app saved `text` to the file itself (Save, the agent's save before a turn): it is
    /// the app's own write, not a hand edit.
    pub fn wrote(&mut self, text: &str) {
        let h = forge_sync::text_hash(text);
        if self.verified && self.write_mark(h).is_ok() {
            self.last = h;
        }
    }

    /// The client kept when the app last closed, to take up: only by the instance that holds
    /// it, and only when the file is the one it was kept with (its mark matches: a file with
    /// no mark, or another text, is not where those ops were made, and they would be sent
    /// before the texts are compared).
    pub fn kept_client(&self) -> Option<String> {
        (self.owner.is_some() && self.verified && self.paused.is_none())
            .then(|| std::fs::read_to_string(&self.pending).ok())
            .flatten()
    }

    /// The client as it must outlive the app (`Link::saved`), written when it changed (by the
    /// instance that holds it only).
    pub fn keep_pending(&mut self, saved: String) {
        if self.owner.is_none() || saved.is_empty() || saved == self.kept {
            return;
        }
        if put(&self.pending, saved.as_bytes()).is_ok() {
            self.kept = saved;
        }
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
        if !self.verified || self.checked.elapsed() < std::time::Duration::from_secs(1) {
            return Ok(());
        }
        self.checked = std::time::Instant::now();
        let on_disk =
            std::fs::read_to_string(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
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
                // a save landing since the check above is not written over (read again, just
                // before the rename: the window left is the rename's own)
                let now = std::fs::read_to_string(&self.path)
                    .map_err(|e| format!("{}: {e}", self.path.display()))?;
                if forge_sync::text_hash(&now) != self.last {
                    return Err(format!(
                        "{} was edited outside the app: syncing paused, your edit kept",
                        self.path.display()
                    ));
                }
                put(&self.path, t.as_bytes()).map_err(|e| format!("{}: {e}", self.path.display()))?;
                self.last = h;
                self.write_mark(h).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::SyncFile;

    /// The clients re-review, #3 and #5: a second app instance on the same document neither
    /// takes up nor writes the kept client (one client id, one op-id counter, shared by two
    /// instances, made one's edits the other's retries); and a file with no mark (a fresh clone,
    /// a deleted mark) takes up nothing (those ops were not made on it).
    #[test]
    fn the_kept_client_is_one_instances_and_only_for_its_own_file() {
        let d = std::env::temp_dir().join(format!("forge-synclink-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join(".forgecad")).unwrap();
        std::fs::write(d.join(".forgecad/remote.toml"), "remote = \"lab\"\n").unwrap();
        let file = d.join("part.fg");
        let text = "shared w = 40\n";
        std::fs::write(&file, text).unwrap();
        // no mark yet: nothing taken up, though a kept client is there
        let mut first = SyncFile::open(&file, text).unwrap();
        first.keep_pending("{\"client\":\"one\"}".into());
        assert!(first.kept_client().is_none(), "no mark: not where the ops were made");
        first.verify(text, text).unwrap();
        assert_eq!(first.kept_client().as_deref(), Some("{\"client\":\"one\"}"));
        // a second instance, while the first holds the document
        let mut second = SyncFile::open(&file, text).unwrap();
        assert!(second.kept_client().is_none(), "the first instance's");
        second.keep_pending("{\"client\":\"two\"}".into());
        assert_eq!(first.kept_client().as_deref(), Some("{\"client\":\"one\"}"), "not written over");
        drop((first, second));
        // the first gone: the next open takes it up
        let again = SyncFile::open(&file, text).unwrap();
        assert_eq!(again.kept_client().as_deref(), Some("{\"client\":\"one\"}"));
        drop(again);
        let _ = std::fs::remove_dir_all(&d);
    }
}

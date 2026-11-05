//! The open documents: each its own view, camera, build and undo; one active. The active one's
//! state is in `Gui`'s and `Shell`'s fields; an idle one's waits in its `DocState`.

use super::*;
use crate::shell::docs::{DocKind, ShellDoc, per_doc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

per_doc!(
    #[derive(Default)]
    Gui,
    GuiDoc,
    [
        view: Option<View>,
        loading: Option<Receiver<Result<Loaded, String>>>,
        calls: Vec<Call>,
        view_file: Option<String>,
        tree_hover_was: Option<forge_doc::ItemId>,
        after_sketch: Option<&'static str>,
        sketch_for: Option<(String, SketchFor, Vec<String>)>,
        sketch_sources: std::collections::HashMap<String, Result<Vec<forge_doc::sketch::Refd>, String>>,
        fg: Option<crate::fg::Session>,
        fg_view: Option<Loaded>,
        fg_folded: Option<crate::mesh::PartMesh>,
        fg_flat: Option<crate::mesh::PartMesh>,
        fg_sketch: Option<crate::shell::fgsketch::Opened>,
        fg_sheets: Vec<crate::fg::SheetInfo>,
        fg_walls: Vec<crate::thermo_view::Gfx>,
        fg_anchors: Option<Arc<crate::fg::AnchorView>>,
        fg_asm: Option<AsmPlaced>,
        fg_asm_anchors: Option<Receiver<PartAnchors>>,
        fg_keep_camera: Option<Camera>,
        extras: gui_overlay::Extras,
        fg_asm_boxes: Vec<crate::explode::Aabb>,
        asm: gui_asm::AsmHost,
        explode: gui_explode::Explode,
        body_rows: Option<crate::scene::Rows>,
        inspector: gui_inspect::Inspector,
        fg_previewed: Option<String>,
    ],
    app [
        ctx, egui, shell, device, queue, adapter, waker, captured, over_view, pointer_px,
        pixels_per_point, warm_textures, target_px, mark_held, gpu_failed, settings_was, docs,
        active, watch, agent, apps,
    ]
);

/// An idle document's state, as the active one's is in `Gui` and `Shell`.
#[derive(Default)]
pub struct DocState {
    pub gui: GuiDoc,
    pub shell: ShellDoc,
}

impl Gui {
    /// The active document out to `d`, and `d`'s in.
    pub fn swap_state(&mut self, d: &mut DocState) {
        self.swap_doc(&mut d.gui);
        self.shell.swap_doc(&mut d.shell);
    }
}

/// A document open in a tab: a file, or one inside a file.
pub struct OpenDoc {
    /// Its file as opened; none for an untitled one.
    pub path: Option<String>,
    /// A document within the file (a drawing block in a .fg); none: the file itself.
    pub sub: Option<String>,
    pub title: String,
    pub dirty: bool,
    pub kind: crate::shell::docs::DocKind,
    /// None while it's the active one: its state is in the `Gui`'s and `Shell`'s fields.
    state: Option<Box<DocState>>,
}

impl OpenDoc {
    fn new(path: Option<String>, sub: Option<String>) -> OpenDoc {
        let title = path.as_deref().map(name_of).unwrap_or_else(|| "Untitled".into());
        OpenDoc {
            path,
            sub,
            title,
            dirty: false,
            kind: Default::default(),
            state: None,
        }
    }

    /// The same file (and sub-document) as this path names.
    fn is(&self, path: &str, sub: Option<&str>) -> bool {
        let canon = |p: &str| std::fs::canonicalize(p).unwrap_or_else(|_| p.into());
        self.sub.as_deref() == sub
            && self
                .path
                .as_deref()
                .is_some_and(|p| p == path || canon(p) == canon(path))
    }
}

impl Gui {
    /// The open documents, in their tabs' order.
    pub fn documents(&self) -> &[OpenDoc] {
        &self.docs
    }

    /// Which of them is in the window.
    pub fn active_document(&self) -> usize {
        self.active
    }

    /// Opens a file in a tab of its own (or finds the tab it's already in), and brings it to the
    /// window if `activate`; the tab's index. What the agent calls for a file it made.
    pub fn open_document(&mut self, path: &str, activate: bool) -> Option<usize> {
        if let Some(i) = self.docs.iter().position(|d| d.is(path, None)) {
            if activate {
                self.activate(i);
            }
            return Some(i);
        }
        let was = self.active;
        let fresh = self.docs.is_empty();
        if !fresh {
            self.sync_tab();
            let mut st = Box::<DocState>::default();
            self.swap_state(&mut st);
            self.docs[self.active].state = Some(st);
            self.after_switch();
        }
        self.docs.push(OpenDoc::new(Some(path.to_string()), None));
        self.active = self.docs.len() - 1;
        if !self.load(path) {
            // nothing opened: the tab goes, the one before comes back, saying why
            let why = std::mem::take(&mut self.shell.status);
            self.docs.pop();
            if fresh {
                let mut blank = DocState::default();
                self.swap_state(&mut blank);
                self.docs.clear();
                self.active = 0;
            } else {
                let mut st = self.docs[was].state.take().expect("an idle document's state");
                self.swap_state(&mut st);
                self.active = was;
                self.after_switch();
            }
            self.shell.status = why;
            self.sync_tab();
            return None;
        }
        self.sync_tab();
        let opened = self.active;
        if !activate {
            self.activate(was);
        }
        Some(opened)
    }

    /// Brings this tab's document to the window: its own view, camera, browser and undo, as left.
    pub fn activate(&mut self, i: usize) {
        if i == self.active || i >= self.docs.len() {
            return;
        }
        self.sync_tab();
        let mut st = self.docs[i].state.take().expect("an idle document's state");
        self.swap_state(&mut st);
        self.docs[self.active].state = Some(st);
        self.active = i;
        self.after_switch();
        self.sync_tab();
    }

    /// What was mid-gesture in the window is for the document that went.
    fn after_switch(&mut self) {
        let s = &mut self.shell;
        s.marking = None;
        s.marking_row = None;
        s.window_box = None;
        s.cube_menu = None;
        s.close_ask = None;
        self.captured = false;
        self.mark_held = None;
        if let Some(v) = &mut self.view {
            v.dirty = true;
        }
    }

    /// Closes a tab, asking first (in the window) if it has changes not saved.
    pub fn request_close(&mut self, i: usize) {
        if i >= self.docs.len() {
            return;
        }
        if i == self.active {
            self.sync_tab();
        }
        if self.docs[i].dirty {
            self.activate(i);
            self.shell.close_ask = Some(self.active);
        } else {
            self.close_document(i, false);
        }
    }

    /// Closes a tab, saving it first if `save`; its neighbour comes up if it was the active one.
    pub fn close_document(&mut self, i: usize, save: bool) {
        if i >= self.docs.len() {
            return;
        }
        self.shell.close_ask = None;
        if i != self.active {
            if save {
                self.activate(i);
                return self.close_document(self.active, true);
            }
            let gone = self.docs.remove(i);
            if i < self.active {
                self.active -= 1;
            }
            if let Some(st) = gone.state {
                self.drop_state(*st);
            }
            self.sync_tab();
            return;
        }
        if save {
            self.fg_save();
            if self.fg.as_ref().is_some_and(|s| s.dirty()) {
                return;
            }
        }
        let mut gone = DocState::default();
        if self.docs.len() == 1 {
            self.swap_state(&mut gone);
            self.docs.clear();
            self.active = 0;
            self.after_switch();
        } else {
            let next = if i + 1 < self.docs.len() { i + 1 } else { i - 1 };
            self.activate(next);
            gone = *self.docs.remove(i).state.expect("an idle document's state");
            if i < self.active {
                self.active -= 1;
            }
        }
        self.drop_state(gone);
        self.sync_tab();
    }

    /// A closed document's GPU picture goes with it.
    fn drop_state(&mut self, st: DocState) {
        if let Some(v) = &st.gui.view {
            self.egui.free_texture(&v.tex);
        }
    }

    /// A tab dragged to another place in the strip.
    pub fn move_document(&mut self, from: usize, to: usize) {
        if from >= self.docs.len() || to >= self.docs.len() || from == to {
            return;
        }
        let d = self.docs.remove(from);
        self.docs.insert(to, d);
        self.active = match self.active {
            a if a == from => to,
            a if from < a && a <= to => a - 1,
            a if to <= a && a < from => a + 1,
            a => a,
        };
        self.shell.close_ask = None;
        self.sync_tab();
    }

    /// The active tab's name and unsaved mark as its document has them; the strip's tabs.
    pub(super) fn sync_tab(&mut self) {
        if let Some(d) = self.docs.get_mut(self.active) {
            if !self.shell.name.is_empty() {
                d.title = self.shell.name.clone();
            }
            d.dirty = self.fg.as_ref().is_some_and(|s| s.dirty());
            d.kind = if self.shell.drawing.is_some() {
                DocKind::Drawing
            } else if self.shell.fg.as_ref().is_some_and(|p| p.assembly) {
                DocKind::Assembly
            } else {
                DocKind::Part
            };
            if let Some(p) = self.fg.as_ref().and_then(|s| s.path.clone()) {
                d.path = Some(p);
            }
        }
        self.shell.doc_tabs = self
            .docs
            .iter()
            .map(|d| crate::shell::docs::DocTab {
                title: d.title.clone(),
                dirty: d.dirty,
                kind: d.kind,
            })
            .collect();
        self.shell.active_doc = self.active;
    }

    /// The strip's and the keys' asks.
    pub(super) fn doc_cmd(&mut self, c: crate::shell::docs::DocCmd) {
        use crate::shell::docs::DocCmd;
        let n = self.docs.len();
        match c {
            DocCmd::Switch(i) => self.activate(i),
            DocCmd::Next if n > 1 => self.activate((self.active + 1) % n),
            DocCmd::Prev if n > 1 => self.activate((self.active + n - 1) % n),
            DocCmd::Next | DocCmd::Prev => {}
            DocCmd::Close(i) => self.request_close(i),
            DocCmd::CloseActive => self.request_close(self.active),
            DocCmd::CloseNow(i, save) => self.close_document(i, save),
            DocCmd::Move(a, b) => self.move_document(a, b),
        }
    }

    /// Files the chrome asked to open (a new drawing, a part from its assembly): each in its tab.
    pub(super) fn open_asked(&mut self) {
        for p in std::mem::take(&mut self.shell.open_paths) {
            self.open_document(&p, true);
        }
    }
}

/// O_NOFOLLOW and O_NONBLOCK (the values differ by platform): a name that is a link is not
/// opened, and a pipe swapped in after the check does not hold the open.
#[cfg(unix)]
const NOFOLLOW: i32 = if cfg!(any(target_os = "macos", target_os = "ios", target_os = "freebsd")) {
    0x100 | 0x4
} else if cfg!(any(target_arch = "aarch64", target_arch = "arm")) {
    0o100000 | 0o4000
} else {
    0o400000 | 0o4000
};

/// A regular file's text, never more than `max` bytes of it; nothing for a link, a device, a
/// folder, or a file swapped in between the look and the open.
fn read_file(p: &Path, max: u64) -> Option<Vec<u8>> {
    use std::io::Read;
    let checked = std::fs::symlink_metadata(p).ok()?;
    if !checked.is_file() {
        return None;
    }
    let mut o = std::fs::OpenOptions::new();
    o.read(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::custom_flags(&mut o, NOFOLLOW);
    let f = o.open(p).ok()?;
    let held = f.metadata().ok()?;
    if !held.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if (held.dev(), held.ino()) != (checked.dev(), checked.ino()) {
            return None;
        }
    }
    let mut b = vec![];
    f.take(max).read_to_end(&mut b).ok()?;
    Some(b)
}

/// A file as last seen: its length and when it was written.
type Stamp = Option<(u64, std::time::SystemTime)>;

fn stamp(p: &Path) -> Stamp {
    let m = std::fs::symlink_metadata(p).ok()?;
    if !m.is_file() {
        return None;
    }
    Some((m.len(), m.modified().ok()?))
}

/// How deep below an open document's folder its parts are looked for, and how many at most.
const DEP_DEPTH: usize = 3;
const DEP_MAX: usize = 2000;

/// The .fg files in a folder and below it.
fn fg_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    let mut todo = vec![(dir.to_path_buf(), 0)];
    while let Some((d, depth)) = todo.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() && depth < DEP_DEPTH {
                todo.push((p, depth + 1));
            } else if ft.is_file() && p.extension().is_some_and(|x| x == "fg") {
                out.push(p);
                if out.len() >= DEP_MAX {
                    return out;
                }
            }
        }
    }
    out.sort();
    out
}

fn folder_of(path: &str) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()?.parent().map(Path::to_path_buf)
}

/// What the open documents read on disk, as last looked at; a thread that wakes the window when
/// any of it changes.
#[derive(Default)]
pub struct Watch {
    seen: HashMap<PathBuf, Stamp>,
    /// The folders the last look was in.
    looked: Vec<PathBuf>,
    /// The folders looked in, shared with the thread.
    dirs: Arc<std::sync::Mutex<Vec<PathBuf>>>,
    /// The thread saw a change since the last look.
    changed: Arc<std::sync::atomic::AtomicBool>,
    started: bool,
    /// An agent's turn is going on: a new .fg in any open document's folder opens in a tab.
    pub agent: bool,
}

/// What the thread compares: every .fg file under the folders, and its stamp.
fn sweep(dirs: &[PathBuf]) -> Vec<(PathBuf, Stamp)> {
    let mut all: Vec<(PathBuf, Stamp)> = dirs
        .iter()
        .flat_map(|d| fg_files(d))
        .map(|p| {
            let s = stamp(&p);
            (p, s)
        })
        .collect();
    all.sort_by(|a, b| a.0.cmp(&b.0));
    all.dedup_by(|a, b| a.0 == b.0);
    all
}

impl Watch {
    /// The thread, once: it looks every half second and wakes the window on a change.
    fn start(&mut self, waker: Waker) {
        if self.started {
            return;
        }
        self.started = true;
        let (dirs, changed) = (self.dirs.clone(), self.changed.clone());
        let _ = std::thread::Builder::new()
            .name("forge-watch".into())
            .spawn(move || {
                let mut was: Option<Vec<(PathBuf, Stamp)>> = None;
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let now = {
                        let d = dirs.lock().unwrap_or_else(|e| e.into_inner()).clone();
                        sweep(&d)
                    };
                    if was.as_ref().is_some_and(|w| *w != now) {
                        changed.store(true, std::sync::atomic::Ordering::Relaxed);
                        waker();
                    }
                    was = Some(now);
                }
            });
    }
}

impl Gui {
    /// An agent's turn began or ended: while it goes on, a .fg it makes beside an open document
    /// opens in a tab of its own and comes up.
    pub fn agent_turn(&mut self, on: bool) {
        if on {
            // what's there already is not new
            self.check_files();
        }
        self.watch.agent = on;
    }

    /// What the open documents are in and read, on disk.
    fn watched_dirs(&self) -> Vec<PathBuf> {
        let mut d: Vec<PathBuf> = self
            .docs
            .iter()
            .filter_map(|d| folder_of(d.path.as_deref()?))
            .collect();
        d.sort();
        d.dedup();
        d
    }

    /// The frame's look at the disk: only when the thread saw something change.
    pub(super) fn watch_frame(&mut self) {
        let dirs = self.watched_dirs();
        // a folder not looked in yet: what's in it now is what's there, not new
        let moved = dirs != self.watch.looked;
        *self.watch.dirs.lock().unwrap_or_else(|e| e.into_inner()) = dirs;
        if !self.docs.is_empty() {
            self.watch.start(self.waker.clone());
        }
        if self
            .watch
            .changed
            .swap(false, std::sync::atomic::Ordering::Relaxed)
            || moved
        {
            self.check_files();
        }
    }

    /// Looks at what the open documents read: a document changed on disk (and not here) reads
    /// it again; an assembly or drawing whose parts changed builds again; a new .fg made in an
    /// agent's turn opens. The file watch calls this; so do saves, at once.
    pub fn check_files(&mut self) {
        let dirs = self.watched_dirs();
        let now: HashMap<PathBuf, Stamp> = sweep(&dirs).into_iter().collect();
        let looked = std::mem::replace(&mut self.watch.looked, dirs);
        let mut changed: Vec<PathBuf> = vec![];
        let mut made: Vec<PathBuf> = vec![];
        for (p, s) in &now {
            match self.watch.seen.get(p) {
                Some(was) if was == s => {}
                Some(_) => changed.push(p.clone()),
                // new in a folder looked in before: made since
                None if looked.iter().any(|d| p.starts_with(d)) => made.push(p.clone()),
                None => {}
            }
        }
        self.watch.seen = now;
        made.sort();
        for p in &changed {
            self.file_changed(p);
        }
        if self.watch.agent {
            for p in made {
                let p = p.to_string_lossy().into_owned();
                // the agent's tab says what it wrote; the watch opens it if the tab didn't
                if !self.docs.iter().any(|d| d.is(&p, None)) {
                    self.open_document(&p, true);
                }
            }
        }
    }

    /// One file changed on disk: the documents it is, or that read it, made again.
    fn file_changed(&mut self, file: &Path) {
        let text = read_file(file, forge_lang::asm::LIMIT).and_then(|b| String::from_utf8(b).ok());
        for i in 0..self.docs.len() {
            let Some(path) = self.docs[i].path.clone() else {
                continue;
            };
            let own = std::fs::canonicalize(&path).is_ok_and(|p| p == file);
            let under = folder_of(&path).is_some_and(|d| file.starts_with(d));
            let name = self.docs[i].title.clone();
            let (session, drawing, status) = self.doc_parts(i);
            if own {
                let Some(text) = &text else { continue };
                if let Some(s) = session {
                    if s.saved == *text {
                        continue;
                    }
                    if s.dirty() {
                        *status = format!(
                            "{name} changed on disk: your changes here are kept, Save writes over it"
                        );
                    } else {
                        s.reload(text.clone());
                        *status = format!("{name} changed on disk: read again");
                    }
                } else if let Some(d) = drawing
                    && d.src != *text
                {
                    d.reload(text.clone());
                }
            } else if under {
                // a part of an assembly, a drawing's model
                if let Some(s) = session.filter(|s| crate::fg::is_assembly(&s.src)) {
                    s.rebuild();
                } else if let Some(d) = drawing {
                    d.reload(d.src.clone());
                }
            }
        }
    }

    /// A document's build, drawing and status line, active or idle.
    fn doc_parts(
        &mut self,
        i: usize,
    ) -> (
        Option<&mut crate::fg::Session>,
        Option<&mut crate::shell::drawing::DrawingEnv>,
        &mut String,
    ) {
        if i == self.active {
            return (
                self.fg.as_mut(),
                self.shell.drawing.as_mut(),
                &mut self.shell.status,
            );
        }
        let st = self.docs[i].state.as_mut().expect("an idle document's state");
        (
            st.gui.fg.as_mut(),
            st.shell.drawing.as_mut(),
            &mut st.shell.status,
        )
    }
}

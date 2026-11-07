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
        back_image_was: Option<String>,
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
        fg_asm_located: Option<Receiver<Result<Vec<String>, String>>>,
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
        active, watch, doc_later, agent, apps,
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
    /// When it was last brought up, to let the least lately used's GPU picture go first.
    used: u64,
    /// Its GPU picture was let go: made again when it comes up.
    evicted: bool,
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
            used: 0,
            evicted: false,
            state: None,
        }
    }

    /// Its GPU picture is kept: it's up, or idle and not let go.
    pub fn resident(&self) -> bool {
        self.state.as_ref().is_none_or(|s| s.gui.view.is_some())
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

/// How many documents keep their GPU picture (renderer, meshes' buffers, target) at once; past
/// it the least lately used idle Forge document lets its go, made again from the build's warm
/// stages when it comes up.
pub const RESIDENT_VIEWS: usize = 6;

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
        let at = self.open_in(path, None, activate)?;
        // a .fg's drawing blocks: a tab each beside its model, Onshape's way
        if path.ends_with(".fg")
            && let Ok(src) = std::fs::read_to_string(path)
            && !forge_lang::drawing::is_drawing(&src)
        {
            for b in forge_lang::drawing::block::blocks(&src) {
                self.open_in(path, Some(&b.name), false);
            }
        }
        Some(at)
    }

    /// A drawing block `name` of the .fg at `path` in its tab (or the tab it's in), brought up.
    pub fn open_drawing_block(&mut self, path: &str, name: &str) -> Option<usize> {
        self.open_in(path, Some(name), true)
    }

    fn open_in(&mut self, path: &str, sub: Option<&str>, activate: bool) -> Option<usize> {
        if let Some(i) = self.docs.iter().position(|d| d.is(path, sub)) {
            if activate {
                self.activate(i);
            }
            // opened again after it changed on disk: what's on disk now
            self.check_files();
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
        self.docs
            .push(OpenDoc::new(Some(path.to_string()), sub.map(str::to_string)));
        self.active = self.docs.len() - 1;
        let loaded = match sub {
            Some(name) => match crate::shell::drawing::DrawingEnv::open_block(path, name) {
                Ok(env) => {
                    self.shell.open_drawing(env);
                    true
                }
                Err(e) => {
                    self.shell.status = e;
                    false
                }
            },
            None => self.load(path),
        };
        if !loaded {
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
        self.came_up();
        self.sync_tab();
        let opened = self.active;
        if !activate {
            self.activate(was);
        }
        Some(opened)
    }

    /// File > New: an untitled Forge part in a tab of its own, saved where asked on its first
    /// Save.
    pub fn new_document(&mut self) -> usize {
        if !self.docs.is_empty() {
            self.sync_tab();
            let mut st = Box::<DocState>::default();
            self.swap_state(&mut st);
            self.docs[self.active].state = Some(st);
            self.after_switch();
        }
        let n = 1 + self.docs.iter().filter(|d| d.path.is_none()).count();
        let name = format!("Part{n}.fg");
        let mut d = OpenDoc::new(None, None);
        d.title = name.clone();
        self.docs.push(d);
        self.active = self.docs.len() - 1;
        let gpu = (self.device.clone(), self.queue.clone(), self.adapter.clone());
        let waker = self.waker.clone();
        let waker: Arc<dyn Fn() + Send + Sync> = Arc::new(move || waker());
        let s = crate::fg::Session::new(None, String::new(), Some(gpu), waker);
        self.shell.fg = Some(crate::shell::fg::FgPanel {
            file: name.clone(),
            ..Default::default()
        });
        self.fg = Some(s);
        self.shell.name = name;
        self.sync_tab();
        self.active
    }

    /// Where the host's dialog said an untitled document goes: written there, its tab named so.
    pub fn save_as(&mut self, path: &str) {
        if let Some(d) = &mut self.shell.dwgdoc {
            d.save_as(path);
            self.shell.name = d.name.clone();
            if let Some(o) = self.docs.get_mut(self.active) {
                o.path = d.path.clone();
            }
            self.sync_tab();
            return;
        }
        let Some(s) = &mut self.fg else { return };
        s.path = Some(path.to_string());
        let name = name_of(path);
        self.shell.name = name.clone();
        if let Some(p) = &mut self.shell.fg {
            p.file = name;
        }
        if let Some(d) = self.docs.get_mut(self.active) {
            d.path = Some(path.to_string());
        }
        self.fg_save();
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
        self.came_up();
        self.sync_tab();
    }

    /// The active document marked as used now; its picture made again if it was let go; others'
    /// let go past the cap.
    fn came_up(&mut self) {
        let now = self.docs.iter().map(|d| d.used).max().unwrap_or(0) + 1;
        let gpu = (self.device.clone(), self.queue.clone(), self.adapter.clone());
        let d = &mut self.docs[self.active];
        d.used = now;
        if std::mem::take(&mut d.evicted)
            && let Some(s) = &mut self.fg
        {
            s.regpu(gpu);
        }
        loop {
            let idle: Vec<usize> = (0..self.docs.len())
                .filter(|&i| {
                    i != self.active
                        && self.docs[i]
                            .state
                            .as_ref()
                            .is_some_and(|s| s.gui.view.is_some() && s.gui.fg.is_some())
                })
                .collect();
            if idle.len() < RESIDENT_VIEWS {
                return;
            }
            let Some(&i) = idle.iter().min_by_key(|&&i| self.docs[i].used) else {
                return;
            };
            let d = &mut self.docs[i];
            d.evicted = true;
            let st = d.state.as_mut().expect("an idle document's state");
            st.gui.fg_view = None;
            st.shell.view_tex = None;
            if let Some(v) = st.gui.view.take() {
                st.gui.fg_keep_camera = Some(v.nav.camera);
                self.egui.free_texture(&v.tex);
            }
        }
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
            if d.path.is_none() && self.fg.as_ref().is_some_and(|s| s.path.is_none()) {
                // an untitled one keeps the name it was given
                self.shell.name.clone_from(&d.title);
            } else if !self.shell.name.is_empty() {
                d.title = self.shell.name.clone();
            }
            d.dirty = self.fg.as_ref().is_some_and(|s| s.dirty());
            d.kind = if self.shell.dwgdoc.is_some() {
                DocKind::Dwg
            } else if self.shell.drawing.is_some() {
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

    /// The tabs' and keys' asks of this frame.
    pub(super) fn docs_asked(&mut self) {
        for c in std::mem::take(&mut self.doc_later) {
            self.doc_cmd(c);
        }
    }

    /// The tab of a file that's gone (a made file the agent's Undo moved aside): closed, not
    /// saved.
    pub(super) fn close_file(&mut self, path: &str) {
        if let Some(i) = self.docs.iter().position(|d| d.is(path, None)) {
            self.close_document(i, false);
        }
    }

    /// Files the chrome asked to open (a new drawing, a part from its assembly): each in its tab.
    pub(super) fn open_asked(&mut self) {
        for p in std::mem::take(&mut self.shell.open_paths) {
            self.open_document(&p, true);
        }
        for (p, b) in std::mem::take(&mut self.shell.open_blocks) {
            self.open_drawing_block(&p, &b);
        }
    }
}

/// A file as last seen: its length, when it was written, and its content's hash (two writes in
/// one tick of the file clock, the same length, still differ); None: not there.
type Seen = Option<(u64, std::time::SystemTime, u128)>;

/// Past this a file is told apart by its length and time alone.
const HASH_MAX: u64 = 16 << 20;

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

fn look(p: &Path) -> Seen {
    let m = std::fs::symlink_metadata(p).ok()?;
    if !m.is_file() {
        return None;
    }
    let hash = read_file(p, HASH_MAX).map_or(0, |b| crate::cache::hash(&[&b]));
    Some((m.len(), m.modified().ok()?, hash))
}

/// The folder a document's reads are watched inside: the repository it's in (the nearest
/// folder up with a `.git`), else its own folder.
fn root_of(path: &str) -> Option<PathBuf> {
    let dir = folder_of(path)?;
    let repo = dir
        .ancestors()
        .find(|a| a.join(".git").exists())
        .map(Path::to_path_buf);
    Some(repo.unwrap_or(dir))
}

/// The .fg files in a folder (not below it): where an agent's new part shows up.
fn fg_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut out: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "fg") && p.is_file())
        .collect();
    out.sort();
    out
}

fn folder_of(path: &str) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()?.parent().map(Path::to_path_buf)
}

/// What's watched: each open document's file and what its last build read, and the folders
/// the documents are in.
#[derive(Clone, Default, PartialEq)]
struct Watched {
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
}

/// What it all looked like: each file, and each folder's .fg files.
#[derive(Clone, Default, PartialEq)]
struct Sight {
    files: HashMap<PathBuf, Seen>,
    dirs: HashMap<PathBuf, Vec<PathBuf>>,
}

fn sight(w: &Watched) -> Sight {
    Sight {
        files: w.files.iter().map(|p| (p.clone(), look(p))).collect(),
        dirs: w.dirs.iter().map(|d| (d.clone(), fg_in(d))).collect(),
    }
}

/// What the open documents read on disk, as last looked at; a thread that wakes the window when
/// any of it changes.
#[derive(Default)]
pub struct Watch {
    seen: Sight,
    /// What's watched, shared with the thread.
    watched: Arc<std::sync::Mutex<Watched>>,
    /// The thread saw a change since the last look.
    changed: Arc<std::sync::atomic::AtomicBool>,
    started: bool,
    /// An agent's turn is going on: a new .fg in an open document's folder opens in a tab.
    pub agent: bool,
}

impl Watch {
    /// The thread, once: it looks every half second and wakes the window on a change.
    fn start(&mut self, waker: Waker) {
        if self.started {
            return;
        }
        self.started = true;
        let (watched, changed) = (self.watched.clone(), self.changed.clone());
        let _ = std::thread::Builder::new()
            .name("forge-watch".into())
            .spawn(move || {
                let mut was: Option<(Watched, Sight)> = None;
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let w = watched.lock().unwrap_or_else(|e| e.into_inner()).clone();
                    let now = sight(&w);
                    if was.as_ref().is_some_and(|(ww, s)| *ww == w && *s != now) {
                        changed.store(true, std::sync::atomic::Ordering::Relaxed);
                        waker();
                    }
                    was = Some((w, now));
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

    /// A document's own file and what its last build read.
    fn reads_of(&self, i: usize) -> Vec<PathBuf> {
        let (fg, dwg) = if i == self.active {
            (self.fg.as_ref(), self.shell.drawing.as_ref())
        } else {
            let Some(st) = self.docs[i].state.as_ref() else {
                return vec![];
            };
            (st.gui.fg.as_ref(), st.shell.drawing.as_ref())
        };
        let mut out: Vec<PathBuf> = vec![];
        if let Some(s) = fg {
            out.extend(s.deps.lock().unwrap_or_else(|e| e.into_inner()).iter().cloned());
        }
        if let Some(d) = dwg {
            out.extend(d.deps.lock().unwrap_or_else(|e| e.into_inner()).iter().cloned());
        }
        out
    }

    /// The files the watch looks at: each open document's own and what it read inside its root.
    pub fn watched_files(&self) -> Vec<PathBuf> {
        self.watched().files
    }

    fn watched(&self) -> Watched {
        let mut files: Vec<PathBuf> = vec![];
        let mut dirs: Vec<PathBuf> = vec![];
        for (i, d) in self.docs.iter().enumerate() {
            let Some(p) = d.path.as_deref() else { continue };
            let Ok(own) = std::fs::canonicalize(p) else {
                continue;
            };
            files.push(own);
            dirs.extend(folder_of(p));
            // what it read, only inside its root
            if let Some(root) = root_of(p) {
                files.extend(self.reads_of(i).into_iter().filter(|r| r.starts_with(&root)));
            }
        }
        files.sort();
        files.dedup();
        dirs.sort();
        dirs.dedup();
        Watched { files, dirs }
    }

    /// The frame's look at the disk: when the thread saw something change, or what's watched did.
    pub(super) fn watch_frame(&mut self) {
        let w = self.watched();
        let moved = {
            let mut shared = self.watch.watched.lock().unwrap_or_else(|e| e.into_inner());
            let moved = *shared != w;
            *shared = w;
            moved
        };
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
    /// it again; one whose parts, models or imports changed builds again; a new .fg made in an
    /// agent's turn opens. The file watch calls this; so do saves, at once.
    pub fn check_files(&mut self) {
        let w = self.watched();
        let now = sight(&w);
        let was = std::mem::replace(&mut self.watch.seen, now.clone());
        // a file newly watched (a build read it first now) is what it is, not a change
        let changed: Vec<PathBuf> = now
            .files
            .iter()
            .filter(|(p, s)| was.files.get(*p).is_some_and(|old| old != *s))
            .map(|(p, _)| p.clone())
            .collect();
        // new in a folder looked in before: made since
        let mut made: Vec<PathBuf> = now
            .dirs
            .iter()
            .filter_map(|(d, list)| Some((list, was.dirs.get(d)?)))
            .flat_map(|(list, old)| {
                list.iter()
                    .filter(|p| !old.contains(p))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect();
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

    /// The drawing up wrote its file (a drawing block's edit writes its .fg): every other tab of
    /// that file reads it again now, not when the watch next looks.
    pub(super) fn drawing_wrote(&mut self) {
        let Some(d) = self.shell.drawing.as_mut().filter(|d| d.wrote) else {
            return;
        };
        d.wrote = false;
        if let Ok(p) = std::fs::canonicalize(&d.path) {
            self.file_changed(&p);
        }
    }

    /// One file changed on disk: the document it is made again from it, and those that read it
    /// built again.
    fn file_changed(&mut self, file: &Path) {
        let text = read_file(file, forge_lang::asm::LIMIT).and_then(|b| String::from_utf8(b).ok());
        for i in 0..self.docs.len() {
            let Some(path) = self.docs[i].path.clone() else {
                continue;
            };
            let own = std::fs::canonicalize(&path).is_ok_and(|p| p == file);
            // in an agent's turn its own writes come through its tab, as one labelled undo step
            if own && self.watch.agent {
                continue;
            }
            let inside = root_of(&path).is_some_and(|r| file.starts_with(r));
            let reads = !own && inside && self.reads_of(i).iter().any(|r| r == file);
            let name = self.docs[i].title.clone();
            let (session, drawing, status) = self.doc_parts(i);
            if own {
                let Some(text) = &text else { continue };
                if let Some(s) = session {
                    if s.saved == *text {
                        continue;
                    }
                    // a drawing block written from its tab: into the unsaved text, Save writes both
                    if s.dirty()
                        && let Some(m) = forge_lang::drawing::block::merge(&s.saved, &s.src, text)
                    {
                        s.merged(text.clone(), m);
                        *status = format!("{name}: the drawing's edit is in your unsaved changes");
                    } else if s.dirty() {
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
            } else if reads {
                if let Some(s) = session {
                    s.rebuild();
                } else if let Some(d) = drawing {
                    d.reload(d.src.clone());
                }
            }
        }
    }

    /// A document's build, drawing and status line, active or idle.
    pub(super) fn doc_parts(
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

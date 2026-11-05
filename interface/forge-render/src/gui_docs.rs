//! The open documents: each its own view, camera, build and undo; one active. The active one's
//! state is in `Gui`'s and `Shell`'s fields; an idle one's waits in its `DocState`.

use super::*;
use crate::shell::docs::{DocKind, ShellDoc, per_doc};

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
        active, agent, apps,
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

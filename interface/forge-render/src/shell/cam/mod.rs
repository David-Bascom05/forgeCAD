//! The Manufacture workspace: a machining document (docs/cam.md) opened in the app. Its text is
//! the source of truth, as a part's is: the browser, the dialogs and the ribbon only ever make
//! edits to it (doc.rs), and what they show is forge-cam-lang's run of it (run.rs), the same
//! reply `forgecad cam --json` gives. The part it machines is open beside it, in the view.
//!
//! What another view reads of it (the toolpath and simulation views are CAM-SIM's): `focus()`,
//! the setup and op selected and a check's witness asked to be located, and `run`'s paths.

pub mod doc;
pub mod nc;
pub mod run;
pub mod schema;
pub mod turnview;
mod ui;
pub use ui::{groups, status_line};

use doc::{Edit, Role, Stmt};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A check's witness asked to be shown: the op, the move along its path, and where (the setup's
/// WCS).
#[derive(Clone, Debug, PartialEq)]
pub struct Locate {
    pub op: String,
    pub mov: usize,
    pub at: [f64; 3],
    pub rule: String,
}

/// What the workspace has in focus, for the views that draw it (toolpath, stock, simulation):
/// the one interface between this workspace and those views.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Focus {
    pub setup: Option<String>,
    /// The op selected (None: the whole setup).
    pub op: Option<String>,
    /// A check's witness to show.
    pub locate: Option<Locate>,
    /// Show the in-process stock as it is after this op (`--at OP`).
    pub stock_at: Option<String>,
    /// The toolpaths shown: every op of the setup, or the selected one only.
    pub only_selected: bool,
    /// The toolpath coloured by a measured value per move (forge-cam-quality's), drawn by the
    /// simulation view; None: by op as before.
    pub colour_by: Option<ColourBy>,
}

/// A per-move value the toolpath can be coloured by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColourBy {
    /// The chip thickness after thinning against the op's target feed per tooth.
    ChipLoad,
    /// The engagement angle.
    Engagement,
}

/// What the ribbon asks for (an `Action` is Copy: the browser's asks carry names, `Cmd`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    NewSetup,
    NewLatheSetup,
    NewOp(&'static str),
    Generate,
    PostProcess,
    SetupSheet,
    ToolLibrary,
    Programs,
    Quality,
    Checks,
    Undo,
    Redo,
    Save,
    ToggleText,
    AutoGenerate,
    CheckNow,
    SectionView,
    /// NC Programs: every program's files with the host's Save As.
    ExportNc,
    /// The setup sheet and tool list with the host's Save As.
    ExportSheet,
    /// Re-post every program whose file is stale.
    RepostStale,
}

impl From<Ask> for Cmd {
    fn from(a: Ask) -> Cmd {
        match a {
            Ask::NewSetup => Cmd::NewSetup,
            Ask::NewLatheSetup => Cmd::NewLatheSetup,
            Ask::NewOp(f) => Cmd::NewOp(f),
            Ask::Generate => Cmd::Generate,
            Ask::PostProcess => Cmd::PostProcess,
            Ask::SetupSheet => Cmd::SetupSheet,
            Ask::ToolLibrary => Cmd::ToolLibrary,
            Ask::Programs => Cmd::Programs,
            Ask::Quality => Cmd::Quality,
            Ask::Checks => Cmd::Checks,
            Ask::Undo => Cmd::Undo,
            Ask::Redo => Cmd::Redo,
            Ask::Save => Cmd::Save,
            Ask::ToggleText => Cmd::ToggleText,
            Ask::AutoGenerate => Cmd::AutoGenerate,
            Ask::CheckNow => Cmd::CheckNow,
            Ask::SectionView => Cmd::SectionView,
            Ask::ExportNc => Cmd::ExportNc(None),
            Ask::ExportSheet => Cmd::ExportSheet,
            Ask::RepostStale => Cmd::RepostStale,
        }
    }
}

/// What the ribbon, the browser and the dialogs ask for.
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    NewSetup,
    /// A lathe setup (turning).
    NewLatheSetup,
    /// A new op of this built-in in the setup in focus.
    NewOp(&'static str),
    /// Statement `id`'s dialog.
    Edit(String),
    Select(String),
    Generate,
    Suppress(String),
    Delete(String),
    /// Earlier (true) or later in its setup.
    Move(String, bool),
    Rename(String, String),
    Locate(Locate),
    StockAt(Option<String>),
    PostProcess,
    SetupSheet,
    ToolLibrary,
    Programs,
    Quality,
    Checks,
    Undo,
    Redo,
    Save,
    /// The text, edited by hand (the Text window).
    Text(String),
    ToggleText,
    AutoGenerate,
    /// Run the certified checker now on what is generated (today: a generate, which checks).
    CheckNow,
    /// The turning view, shown or not.
    SectionView,
    /// Post program NAME (or all) into the document's folder.
    PostNc(Option<String>),
    /// The Post Process dialog at the K-th program.
    PostAt(usize),
    /// Program NAME's (or all) files with the host's Save As.
    ExportNc(Option<String>),
    ExportSheet,
    RepostStale,
}

/// The setup dialog's state: the args as edited, by keyword.
#[derive(Clone, Debug, Default)]
pub struct SetupDlg {
    /// The statement edited; None: a new one.
    pub id: Option<String>,
    pub name: String,
    pub tab: usize,
    /// keyword → text: machine, origin, tol, clearance, at, fixture, from, stock_kind, margin,
    /// min, max; a lathe's: machine, bar, front, chuck, at, offsets, tol, clearance
    pub args: Vec<(String, String)>,
    pub error: Option<String>,
    /// A lathe setup (`lathe_setup`, forge-turn-lang).
    pub lathe: bool,
}

/// An op dialog's state.
#[derive(Clone, Debug, Default)]
pub struct OpDlg {
    pub id: Option<String>,
    pub func: String,
    pub name: String,
    pub setup: String,
    pub tab: usize,
    /// argument (`tool`, `#1`, ...) → text; "" left out
    pub args: Vec<(String, String)>,
    pub was: Vec<(String, String)>,
    pub error: Option<String>,
}

impl OpDlg {
    pub fn get(&self, k: &str) -> &str {
        self.args.iter().find(|a| a.0 == k).map_or("", |a| a.1.as_str())
    }
    pub fn set(&mut self, k: &str, v: String) {
        match self.args.iter_mut().find(|a| a.0 == k) {
            Some(a) => a.1 = v,
            None => self.args.push((k.to_string(), v)),
        }
    }
}

/// The post dialog's state.
#[derive(Clone, Debug, Default)]
pub struct PostDlg {
    pub program: usize,
}

/// A part a setup machines, as read for picking: its holes and sketches.
#[derive(Clone, Debug, Default)]
pub struct PartInfo {
    /// The name the document binds it to (`f`).
    pub name: String,
    pub file: String,
    pub holes: Vec<forge_cam_lang::holes::HoleInfo>,
    pub sketches: Vec<String>,
}

pub struct CamWs {
    pub path: String,
    pub dir: PathBuf,
    pub text: String,
    /// The text as last saved.
    pub saved: String,
    pub stmts: Vec<Stmt>,
    /// The last run, and what of it is stale against the text now.
    pub run: Option<run::Run>,
    pub stale: Vec<String>,
    /// A run asked of the host, and the one it is making (its text).
    pub want_run: bool,
    /// A check of the last run asked of the host (Check Now).
    pub want_check: bool,
    pub running: Option<String>,
    /// Where the host's run lands.
    pub job: Arc<Mutex<Option<run::Run>>>,
    pub auto: bool,
    pub selected: Option<String>,
    pub focus: Focus,
    pub folded: Vec<String>,
    pub op_dlg: Option<OpDlg>,
    pub setup_dlg: Option<SetupDlg>,
    pub post_dlg: Option<PostDlg>,
    pub tools_open: bool,
    /// The NC programs window.
    pub programs_open: bool,
    /// The toolpath quality window.
    pub quality_open: bool,
    /// The tool library's import field: a Fusion tool library's file, beside the document.
    pub import_path: String,
    pub tools_filter: String,
    pub checks_open: bool,
    pub text_open: bool,
    pub text_edit: String,
    pub undo: Vec<String>,
    pub redo: Vec<String>,
    pub parts: Vec<PartInfo>,
    /// Said to the user (an edit refused, a file written).
    pub said: Option<String>,
    /// The setup sheet and program asked to be written (path, text).
    pub want_write: Vec<(String, String)>,
    /// A rename in the browser: the row and its text.
    pub renaming: Option<(String, String)>,
    pub lib: forge_tooling::Library,
    /// The turning view (a lathe setup in section).
    pub turnview: turnview::TurnView,
    /// Files for the host's Save As (taken by the host: a dialog natively, a download in the browser).
    pub export: Option<nc::ExportAsk>,
    /// Post and export programs not certified in full (their headers say so).
    pub allow_uncertified: bool,
    /// Files written so far (the posted states are read again after each).
    pub write_gen: u64,
    /// Exports handed to the host so far.
    pub exported: u64,
    /// A program setting being typed (its key, the text so far).
    pub prog_field: Option<(String, String)>,
    pub posted_cache: Option<(u64, Vec<nc::PostedRow>)>,
}

impl CamWs {
    /// The document at `path`, its text `text`.
    pub fn open(path: &str, text: &str) -> CamWs {
        let dir = std::path::Path::new(path)
            .parent()
            .map_or_else(|| PathBuf::from("."), std::path::Path::to_path_buf);
        let lib = library(&dir);
        let mut ws = CamWs {
            path: path.to_string(),
            dir,
            text: text.to_string(),
            saved: text.to_string(),
            stmts: doc::read(text).unwrap_or_default(),
            run: None,
            stale: vec![],
            want_run: true,
            want_check: false,
            running: None,
            job: Arc::new(Mutex::new(None)),
            auto: true,
            selected: None,
            focus: Focus::default(),
            folded: vec![],
            op_dlg: None,
            setup_dlg: None,
            post_dlg: None,
            tools_open: false,
            programs_open: false,
            quality_open: false,
            import_path: String::new(),
            tools_filter: String::new(),
            checks_open: false,
            text_open: false,
            text_edit: String::new(),
            undo: vec![],
            redo: vec![],
            parts: vec![],
            said: None,
            want_write: vec![],
            renaming: None,
            lib,
            turnview: turnview::TurnView {
                open: true,
                ..Default::default()
            },
            export: None,
            allow_uncertified: false,
            write_gen: 0,
            exported: 0,
            posted_cache: None,
            prog_field: None,
        };
        ws.read_parts();
        ws.restale();
        ws.focus.setup = ws.setups().first().map(|s| s.name.clone());
        ws
    }

    pub fn dirty(&self) -> bool {
        self.text != self.saved
    }

    /// The part file the view shows: the first `part(..)`'s, beside the document.
    pub fn part_path(&self) -> Option<PathBuf> {
        let p = self.stmts.iter().find(|s| s.role == Role::Part)?;
        Some(self.dir.join(doc::unquote(p.pos(0)?)))
    }

    fn read_parts(&mut self) {
        let mut out = vec![];
        for s in self.stmts.iter().filter(|s| s.role == Role::Part) {
            let Some(file) = s.pos(0).map(doc::unquote) else {
                continue;
            };
            let path = self.dir.join(&file);
            let mut info = PartInfo {
                name: s.name.clone(),
                file: file.clone(),
                ..Default::default()
            };
            if let Ok(src) = forge_dirs::fs::read_to_string(&path) {
                let opts = forge_lang::Options {
                    dir: Some(self.dir.clone()),
                    ..forge_lang::Options::default()
                };
                info.holes = forge_cam_lang::holes::hole_view(&src, opts);
                if let Ok(prog) = forge_lang::syntax::parse(&src) {
                    for st in &prog {
                        if let forge_lang::syntax::StmtKind::Block {
                            kind: forge_lang::syntax::BlockKind::Sketch,
                            name,
                            ..
                        } = &st.kind
                        {
                            info.sketches.push(name.clone());
                        }
                    }
                }
            }
            out.push(info);
        }
        self.parts = out;
    }

    /// The statements in the browser: setups, in order.
    pub fn setups(&self) -> Vec<&Stmt> {
        self.stmts.iter().filter(|s| s.role == Role::Setup).collect()
    }

    /// The ops (and programs) of setup `name`, in order.
    pub fn ops_of(&self, setup: &str) -> Vec<&Stmt> {
        self.stmts
            .iter()
            .filter(|s| s.is_op() && s.setup.as_deref() == Some(setup))
            .collect()
    }

    pub fn tools(&self) -> Vec<&Stmt> {
        self.stmts.iter().filter(|s| s.role == Role::Tool).collect()
    }

    pub fn stmt(&self, id: &str) -> Option<&Stmt> {
        self.stmts.iter().find(|s| s.name == id)
    }

    /// The op's state in the browser: suppressed, stale, generating, or the run's word.
    pub fn status(&self, id: &str) -> &str {
        if self.stmt(id).is_some_and(|s| s.suppressed) {
            return "suppressed";
        }
        if self.stale.iter().any(|s| s == id) {
            return if self.running.is_some() {
                "generating"
            } else {
                "stale"
            };
        }
        match self.run.as_ref().and_then(|r| r.op(id)) {
            Some(o) => o.status.as_str(),
            None if self.running.is_some() => "generating",
            None => "error",
        }
    }

    /// What the views read.
    pub fn focus(&self) -> &Focus {
        &self.focus
    }

    fn restale(&mut self) {
        self.stale = match &self.run {
            Some(r) => run::stale(&r.stmts, &self.stmts),
            None => self
                .stmts
                .iter()
                .filter(|s| s.is_op() && !s.suppressed)
                .map(|s| s.name.clone())
                .collect(),
        };
    }

    /// The text replaced by `text` (one undo step); generated again when auto.
    pub fn set_text(&mut self, text: String) {
        if text == self.text {
            return;
        }
        self.undo.push(std::mem::replace(&mut self.text, text));
        self.redo.clear();
        self.reread();
    }

    fn reread(&mut self) {
        if let Some(s) = doc::read(&self.text) {
            self.stmts = s;
        }
        if self.stmts.iter().any(|s| s.role == Role::Part)
            && self.parts.len() != self.stmts.iter().filter(|s| s.role == Role::Part).count()
        {
            self.read_parts();
        }
        self.restale();
        if self.auto {
            self.want_run = true;
        }
    }

    /// One edit; refused edits are said and change nothing.
    pub fn edit(&mut self, e: &Edit) -> bool {
        match doc::apply(&self.text, e) {
            Ok(t) => {
                self.set_text(t);
                true
            }
            Err(why) => {
                self.said = Some(why);
                false
            }
        }
    }

    /// Several edits as one undo step, all or none.
    pub fn edits(&mut self, es: &[Edit]) -> Result<(), String> {
        let mut t = self.text.clone();
        for e in es {
            t = doc::apply(&t, e)?;
        }
        self.set_text(t);
        Ok(())
    }

    /// A run finished (the host's).
    pub fn ran(&mut self, r: run::Run) {
        if self.running.as_deref() == Some(r.text.as_str()) {
            self.running = None;
        }
        self.run = Some(r);
        self.restale();
    }

    /// A pick in the view while an op dialog is up: the part's feature it is on, as the op's
    /// geometry (a hole feature, or a sketch) when the op takes one. Says what was taken.
    pub fn pick(&mut self, anchor: &str) -> String {
        let feat: String = anchor
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let Some(d) = &mut self.op_dlg else {
            return String::new();
        };
        let Some(spec) = schema::spec(&d.func) else {
            return String::new();
        };
        let Some(field) = spec
            .fields
            .iter()
            .find(|f| matches!(f.kind, schema::Kind::Hole | schema::Kind::Sketch))
        else {
            return format!("Picked {anchor}: {} takes no geometry from the model", spec.label);
        };
        let Some(part) = self.parts.first() else {
            return String::new();
        };
        let ok = match field.kind {
            schema::Kind::Hole => part.holes.iter().any(|h| h.name == feat),
            _ => part.sketches.contains(&feat),
        };
        if !ok {
            let what = if field.kind == schema::Kind::Hole {
                "a hole() feature"
            } else {
                "a sketch"
            };
            return format!("Picked {anchor}: not on {what} of {}", part.file);
        }
        let v = format!("{}.{feat}", part.name);
        d.set(field.arg, v.clone());
        // a hole's op takes its heights from the hole
        if field.kind == schema::Kind::Hole {
            for k in ["from", "to"] {
                d.set(k, String::new());
            }
        }
        format!("{}: {v}", field.label)
    }

    /// The setup in focus, else the last.
    pub fn setup_now(&self) -> Option<String> {
        self.focus
            .setup
            .clone()
            .filter(|s| self.stmt(s).is_some())
            .or_else(|| self.setups().last().map(|s| s.name.clone()))
    }

    pub fn command(&mut self, c: Cmd) {
        match c {
            Cmd::NewSetup => self.setup_dlg = Some(self.new_setup_dlg()),
            Cmd::NewLatheSetup => self.setup_dlg = Some(self.new_lathe_dlg()),
            Cmd::NewOp(func) => match self.setup_now() {
                Some(su) => self.op_dlg = Some(self.new_op_dlg(func, &su)),
                None => self.said = Some("make a setup first: Setup ▸ New Setup".into()),
            },
            Cmd::Edit(id) => self.open_dialog(&id),
            Cmd::Select(id) => self.select(&id),
            Cmd::Generate => self.want_run = true,
            Cmd::Suppress(id) => {
                let sup = self.stmt(&id).is_some_and(|s| s.suppressed);
                self.edit(&if sup {
                    Edit::Unsuppress { id }
                } else {
                    Edit::Suppress { id }
                });
            }
            Cmd::Delete(id) => {
                if self.edit(&Edit::Delete { id: id.clone() })
                    && self.selected.as_deref() == Some(id.as_str())
                {
                    self.selected = None;
                    self.focus.op = None;
                }
            }
            Cmd::Move(id, up) => self.move_op(&id, up),
            Cmd::Rename(from, to) => {
                if self.edit(&Edit::Rename {
                    from: from.clone(),
                    to: to.clone(),
                }) && self.selected.as_deref() == Some(from.as_str())
                {
                    self.select(&to);
                }
            }
            Cmd::Locate(l) => {
                self.select(&l.op.clone());
                self.focus.locate = Some(l);
            }
            Cmd::StockAt(op) => self.focus.stock_at = op,
            Cmd::PostProcess => {
                let open = self.post_dlg.is_none();
                self.close_windows();
                if open {
                    self.post_dlg = Some(PostDlg::default());
                }
            }
            Cmd::SetupSheet => match &self.run {
                Some(r) => {
                    let p = self.path.trim_end_matches(".fg").to_string() + "-sheet.html";
                    self.want_write.push((p, r.sheet.clone()));
                }
                None => self.said = Some("generate first".into()),
            },
            // the floating windows: one at a time (each opens where the last was, over the view)
            Cmd::ToolLibrary => {
                let open = !self.tools_open;
                self.close_windows();
                self.tools_open = open;
            }
            Cmd::Programs => {
                let open = !self.programs_open;
                self.close_windows();
                self.programs_open = open;
            }
            Cmd::Quality => {
                let open = !self.quality_open;
                self.close_windows();
                self.quality_open = open;
            }
            Cmd::Checks => {
                let open = !self.checks_open;
                self.close_windows();
                self.checks_open = open;
            }
            Cmd::Undo => {
                if let Some(t) = self.undo.pop() {
                    self.redo.push(std::mem::replace(&mut self.text, t));
                    self.reread();
                }
            }
            Cmd::Redo => {
                if let Some(t) = self.redo.pop() {
                    self.undo.push(std::mem::replace(&mut self.text, t));
                    self.reread();
                }
            }
            Cmd::Save => self.want_write.push((self.path.clone(), self.text.clone())),
            Cmd::Text(t) => self.set_text(t),
            Cmd::ToggleText => {
                let open = !self.text_open;
                self.close_windows();
                self.text_open = open;
                self.text_edit = self.text.clone();
            }
            Cmd::AutoGenerate => self.auto = !self.auto,
            Cmd::SectionView => self.turnview.open = !self.turnview.open,
            Cmd::PostNc(n) => self.post_programs(n.as_deref()),
            Cmd::PostAt(k) => {
                self.close_windows();
                self.post_dlg = Some(PostDlg { program: k });
            }
            Cmd::ExportNc(n) => self.export_programs(n.as_deref()),
            Cmd::ExportSheet => self.export_sheet(),
            Cmd::RepostStale => self.repost_stale(),
            // forge-cam-lang checks as it generates: until an on-demand checker of a run's
            // posted paths is exposed, checking now is generating again
            Cmd::CheckNow => {
                // a milling run is checked again as posted; anything else is generated (a
                // turning run checks as it generates)
                if self.run.as_ref().is_some_and(|r| r.cam.is_some()) && self.stale.is_empty() {
                    self.want_check = true;
                } else {
                    self.want_run = true;
                }
                self.checks_open = true;
            }
        }
    }

    fn select(&mut self, id: &str) {
        self.selected = Some(id.to_string());
        self.focus.locate = None;
        match self.stmt(id).map(|s| (s.role.clone(), s.setup.clone())) {
            Some((Role::Setup, _)) => {
                self.focus.setup = Some(id.to_string());
                self.focus.op = None;
            }
            Some((Role::Op(_), su)) | Some((Role::Program, su)) => {
                self.focus.setup = su;
                self.focus.op = Some(id.to_string());
            }
            _ => {}
        }
    }

    /// An op one place earlier or later among its setup's ops (the text's order is the order
    /// they run in).
    fn move_op(&mut self, id: &str, up: bool) {
        let Some(su) = self.stmt(id).and_then(|s| s.setup.clone()) else {
            return;
        };
        let ops: Vec<String> = self.ops_of(&su).iter().map(|s| s.name.clone()).collect();
        let Some(k) = ops.iter().position(|n| n == id) else {
            return;
        };
        let before = if up {
            if k == 0 {
                return;
            }
            Some(ops[k - 1].clone())
        } else {
            if k + 1 >= ops.len() {
                return;
            }
            // before the statement after the next op (or the end)
            let next = &ops[k + 1];
            let at = self.stmts.iter().position(|s| &s.name == next).unwrap_or(0);
            self.stmts.get(at + 1).map(|s| s.name.clone())
        };
        self.edit(&Edit::Move {
            id: id.to_string(),
            before,
        });
    }

    fn open_dialog(&mut self, id: &str) {
        let Some(s) = self.stmt(id).cloned() else { return };
        match &s.role {
            Role::Setup => {
                let mut d = SetupDlg {
                    id: Some(id.to_string()),
                    name: id.to_string(),
                    ..Default::default()
                };
                d.lathe = s.func == "lathe_setup";
                d.args = if d.lathe {
                    s.args
                        .iter()
                        .filter_map(|a| a.key.clone().map(|k| (k, a.text.clone())))
                        .collect()
                } else {
                    setup_args(&s)
                };
                self.setup_dlg = Some(d);
            }
            Role::Op(func) => {
                let mut d = OpDlg {
                    id: Some(id.to_string()),
                    func: func.clone(),
                    name: id.to_string(),
                    setup: s.setup.clone().unwrap_or_default(),
                    ..Default::default()
                };
                let mut pos = 0;
                for a in &s.args {
                    match &a.key {
                        Some(k) => d.args.push((k.clone(), a.text.clone())),
                        None => {
                            if pos > 0 {
                                d.args.push((format!("#{pos}"), a.text.clone()));
                            }
                            pos += 1;
                        }
                    }
                }
                d.was = d.args.clone();
                self.op_dlg = Some(d);
            }
            Role::Program => self.post_dlg = Some(PostDlg::default()),
            Role::Tool => self.tools_open = true,
            _ => self.text_open = true,
        }
    }

    fn new_setup_dlg(&self) -> SetupDlg {
        let name = doc::free_name(&self.stmts, &format!("op{}", self.setups().len() + 1));
        let machine = self
            .stmts
            .iter()
            .find(|s| s.role == Role::Machine)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "\"generic-vmc3\"".into());
        SetupDlg {
            id: None,
            name,
            tab: 0,
            args: vec![
                ("machine".into(), machine),
                ("stock_kind".into(), "margin".into()),
                ("margin".into(), "(3, 3, 2)".into()),
                ("origin".into(), "\"top-center\"".into()),
            ],
            error: None,
            lathe: false,
        }
    }

    /// Whether the document is a turning one (its setups are lathe setups).
    pub fn turning(&self) -> bool {
        self.stmts.iter().any(|s| s.func == "lathe_setup")
    }

    fn new_lathe_dlg(&self) -> SetupDlg {
        let name = doc::free_name(&self.stmts, &format!("op{}", self.setups().len() + 1));
        let chuck = self.stmts.iter().find(|s| s.func == "chuck").map_or_else(
            || "chuck(face: -70, grip: 30, jaw: 15)".to_string(),
            |s| s.name.clone(),
        );
        SetupDlg {
            id: None,
            name,
            tab: 0,
            args: vec![
                ("machine".into(), "\"generic-lathe\"".into()),
                ("bar".into(), "32".into()),
                ("front".into(), "2".into()),
                ("chuck".into(), chuck),
            ],
            error: None,
            lathe: true,
        }
    }

    fn new_op_dlg(&self, func: &str, setup: &str) -> OpDlg {
        let spec = schema::spec(func);
        let base = match func {
            "facing" | "turn_face" => "face",
            "edge_break" => "chamfer",
            "turn_rough" => "rough",
            "turn_finish" => "finish",
            "bore_rough" => "bore",
            "turn_groove" => "groove",
            "turn_thread" => "thread",
            "part_off" => "cut",
            "turn_drill" => "drilling",
            f => f,
        };
        let name = doc::free_name(&self.stmts, base);
        let mut args = vec![];
        if spec.is_some_and(|s| s.fields.iter().any(|f| f.arg == "tool")) {
            // the last tool used in the setup, else the first tool
            let tool = self
                .ops_of(setup)
                .iter()
                .rev()
                .find_map(|s| s.arg("tool").map(str::to_string))
                .or_else(|| {
                    let lathe = schema::is_turning_op(func);
                    self.tools()
                        .iter()
                        .find(|s| (s.func == "lathe_tool") == lathe)
                        .map(|s| s.name.clone())
                });
            if let Some(t) = tool {
                args.push(("tool".into(), t));
            }
        }
        OpDlg {
            id: None,
            func: func.to_string(),
            name,
            setup: setup.to_string(),
            tab: 0,
            was: vec![],
            args,
            error: None,
        }
    }

    /// The op dialog's text: `NAME = func(SETUP, #1, k: v, ...)`.
    pub fn op_text(d: &OpDlg) -> String {
        let mut parts = vec![d.setup.clone()];
        if let Some(g) = d.args.iter().find(|a| a.0 == "#1" && !a.1.trim().is_empty()) {
            parts.push(g.1.trim().to_string());
        }
        for (k, v) in &d.args {
            if k.starts_with('#') || v.trim().is_empty() {
                continue;
            }
            parts.push(format!("{k}: {}", v.trim()));
        }
        format!("{} = {}({})", d.name, d.func, parts.join(", "))
    }

    /// The op dialog's OK: a new op before its setup's program (or after its last op), or the
    /// edited one's arguments set, put in and taken out. True when made.
    pub fn op_ok(&mut self, d: &OpDlg) -> Result<(), String> {
        match &d.id {
            None => {
                // after the setup's last op, else after the setup
                let su = &d.setup;
                let last = self
                    .ops_of(su)
                    .last()
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| su.clone());
                let at = self.stmts.iter().position(|s| s.name == last).unwrap_or(0);
                let before = self.stmts.get(at + 1).map(|s| s.name.clone());
                self.edits(&[Edit::Add {
                    before,
                    text: Self::op_text(d),
                }])?;
                self.select(&d.name);
                Ok(())
            }
            Some(id) => {
                let get = |a: &[(String, String)], k: &str| {
                    a.iter()
                        .find(|x| x.0 == k)
                        .map(|x| x.1.trim().to_string())
                        .unwrap_or_default()
                };
                let mut es = vec![];
                let keys: Vec<String> = d.args.iter().chain(&d.was).map(|a| a.0.clone()).collect();
                let mut seen: Vec<&String> = vec![];
                // a positional geometry argument put in or taken out: the statement replaced
                let geo_now = get(&d.args, "#1");
                let geo_was = get(&d.was, "#1");
                if geo_now.is_empty() != geo_was.is_empty() {
                    // the statement's call written again in place (`ReplaceFeature`)
                    let full = Self::op_text(d);
                    let call = full.split_once(" = ").map_or(full.as_str(), |x| x.1).to_string();
                    let r = doc::replace_call(&self.text, id, &call)?;
                    self.set_text(r);
                } else {
                    for k in &keys {
                        if seen.contains(&k) {
                            continue;
                        }
                        seen.push(k);
                        let (now, was) = (get(&d.args, k), get(&d.was, k));
                        if now == was {
                            continue;
                        }
                        if now.is_empty() {
                            es.push(Edit::Unset {
                                id: id.clone(),
                                arg: k.clone(),
                            });
                        } else {
                            es.push(Edit::Set {
                                id: id.clone(),
                                arg: k.clone(),
                                text: now,
                            });
                        }
                    }
                    self.edits(&es)?;
                }
                if d.name != *id {
                    self.edits(&[Edit::Rename {
                        from: id.clone(),
                        to: d.name.clone(),
                    }])?;
                    self.select(&d.name);
                }
                Ok(())
            }
        }
    }

    /// The setup dialog's text.
    pub fn setup_text(&self, d: &SetupDlg) -> String {
        let part = self
            .stmts
            .iter()
            .find(|s| s.role == Role::Part)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "f".into());
        let mut parts = vec![part];
        for (k, v) in Self::setup_kw(d) {
            if !v.is_empty() {
                parts.push(format!("{k}: {v}"));
            }
        }
        let func = if d.lathe { "lathe_setup" } else { "setup" };
        format!("{} = {func}({})", d.name, parts.join(", "))
    }

    pub fn setup_ok(&mut self, d: &SetupDlg) -> Result<(), String> {
        let text = self.setup_text(d);
        match &d.id {
            None => {
                let mut text = text;
                let mut es = vec![];
                // a lathe setup's chuck written as a statement of its own (`chuck(..)` is the
                // language's value there, not a call a setup argument may hold)
                if d.lathe
                    && let Some(c) = Self::setup_kw(d)
                        .iter()
                        .find(|(k, v)| *k == "chuck" && v.starts_with("chuck("))
                        .map(|x| x.1.clone())
                {
                    let jaws = doc::free_name(&self.stmts, "jaws");
                    es.push(Edit::Add {
                        before: None,
                        text: format!("{jaws} = {c}"),
                    });
                    text = text.replacen(&c, &jaws, 1);
                }
                // at the end, so it reads what is above it
                es.push(Edit::Add { before: None, text });
                // and a program for it, so its ops are posted and certified
                let n = doc::free_name(&self.stmts, &format!("prog{}", self.setups().len() + 1));
                let num = if d.lathe {
                    1 + self.setups().len()
                } else {
                    1001 + self.setups().len()
                };
                let (func, post) = if d.lathe {
                    ("lathe_program", "fanuc-lathe")
                } else {
                    ("program", "fanuc")
                };
                es.push(Edit::Add {
                    before: None,
                    text: format!("{n} = {func}({}, post: \"{post}\", number: {num})", d.name),
                });
                self.edits(&es)?;
                self.select(&d.name);
                Ok(())
            }
            Some(id) => {
                // each argument set, put in or taken out where it is: the rest of the line kept
                let Some(s) = self.stmt(id).cloned() else {
                    return Err(format!("no setup `{id}`"));
                };
                let mut es = vec![];
                for (k, v) in Self::setup_kw(d) {
                    let was = s.arg(k).map(str::trim).unwrap_or("");
                    if v == was {
                        continue;
                    }
                    if v.is_empty() {
                        es.push(Edit::Unset {
                            id: id.clone(),
                            arg: k.into(),
                        });
                    } else {
                        es.push(Edit::Set {
                            id: id.clone(),
                            arg: k.into(),
                            text: v,
                        });
                    }
                }
                if d.name != *id {
                    es.push(Edit::Rename {
                        from: id.clone(),
                        to: d.name.clone(),
                    });
                }
                self.edits(&es)?;
                Ok(())
            }
        }
    }

    /// Every floating window closed (the dialogs of an op or a setup stay: they are edits).
    pub fn close_windows(&mut self) {
        self.post_dlg = None;
        self.tools_open = false;
        self.programs_open = false;
        self.quality_open = false;
        self.checks_open = false;
        self.text_open = false;
    }

    /// The setup dialog's keyword arguments, in the order they are written ("" left out).
    pub fn setup_kw(d: &SetupDlg) -> Vec<(&'static str, String)> {
        let get = |k: &str| {
            d.args
                .iter()
                .find(|a| a.0 == k)
                .map(|a| a.1.trim().to_string())
                .unwrap_or_default()
        };
        if d.lathe {
            return LATHE_KW.iter().map(|k| (*k, get(k))).collect();
        }
        // a stock call of its keyword arguments that are given
        let call = |f: &str, keys: &[&str]| {
            let a: Vec<String> = keys
                .iter()
                .filter(|k| !get(k).is_empty())
                .map(|k| format!("{k}: {}", get(k)))
                .collect();
            format!("{f}({})", a.join(", "))
        };
        let stock = match get("stock_kind").as_str() {
            "box" => format!("box(min: {}, max: {})", get("min"), get("max")),
            "from" => String::new(),
            "cylinder" => call("cylinder", &["d", "margin", "below"]),
            "tube" => call("tube", &["d", "id", "margin", "below"]),
            "solid" => format!("solid(\"{}\")", get("file").trim_matches('"')),
            _ => call("box", &["margin", "below"]),
        };
        let mut out = vec![
            ("from", get("from")),
            ("machine", get("machine")),
            ("stock", stock),
        ];
        for k in [
            "origin",
            "wcs",
            "tol",
            "clearance",
            "clear",
            "retract",
            "at",
            "fixture",
        ] {
            out.push((k, get(k)));
        }
        out
    }
}

/// The machining document of the part at `part_path` (the Environments tab's Enter, from a
/// part): `<part>_cam.fg` beside it if that is a machining document, else a new one linked to
/// the part, not yet written (it is dirty until saved). Refused if the file there is not one.
pub fn open_for(part_path: &str) -> Result<CamWs, String> {
    let part = std::path::Path::new(part_path);
    let stem = part
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("{part_path}: not a part file"))?;
    let file = part
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("{part_path}: not a part file"))?;
    let doc = part.with_file_name(format!("{stem}_cam.fg"));
    let path = doc.display().to_string();
    if let Ok(text) = forge_dirs::fs::read_to_string(&doc) {
        if !forge_cam_lang::is_machining(&text) {
            return Err(format!("{path} is there and is not a machining document"));
        }
        return Ok(CamWs::open(&path, &text));
    }
    let text = format!(
        "machining\n# {file} machined: add a setup (Setup > New Setup), then its operations.\n\
         f  = part(\"{file}\", group: \"N\")\nm  = machine(\"generic-vmc3\")\n\
         t1 = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")\n"
    );
    let mut ws = CamWs::open(&path, &text);
    ws.saved = String::new();
    Ok(ws)
}

/// The tool library a document sees: the built-in one and the project's `cam/*.toml` beside it.
pub fn library(dir: &std::path::Path) -> forge_tooling::Library {
    let mut lib = forge_tooling::Library::builtin();
    if let Ok(rd) = forge_dirs::fs::read_dir(dir.join("cam")) {
        let mut files: Vec<PathBuf> = rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        files.sort();
        for f in files {
            if let Ok(s) = forge_dirs::fs::read_to_string(&f) {
                let _ = lib.add(&f.display().to_string(), &s);
            }
        }
    }
    lib
}

impl CamWs {
    /// A Fusion tool library (its JSON, at `file` beside the document) converted into the
    /// project's `cam/<name>.toml` (forge-cam-import: read back before it is written), and the
    /// library reloaded. Says what happened.
    pub fn import_fusion(&mut self, file: &str) -> Result<String, String> {
        let src_path = self.dir.join(file.trim());
        let src =
            forge_dirs::fs::read_to_string(&src_path).map_err(|e| format!("{}: {e}", src_path.display()))?;
        let stem = src_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("fusion")
            .replace(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'), "_");
        let imported = forge_cam_import::fusion(&src, file.trim(), "imported in the app")?;
        let dir = self.dir.join("cam");
        let _ = forge_dirs::fs::create_dir_all(&dir);
        let out = dir.join(format!("{stem}.toml"));
        forge_dirs::fs::write(&out, imported.toml.as_bytes())
            .map_err(|e| format!("{}: {e}", out.display()))?;
        self.lib = library(&self.dir);
        let mut said = format!("{} tools imported into {}", imported.tools, out.display());
        if !imported.warnings.is_empty() {
            said.push_str(&format!(
                " ({} warnings: {})",
                imported.warnings.len(),
                imported.warnings.join("; ")
            ));
        }
        Ok(said)
    }

    /// Each setup's machining time from the run: (setup, its ops' time, s).
    pub fn setup_times(&self) -> Vec<(String, f64)> {
        let Some(r) = &self.run else { return vec![] };
        self.setups()
            .iter()
            .map(|s| {
                let t: f64 = r.ops.iter().filter(|o| o.setup == s.name).map(|o| o.time_s).sum();
                (s.name.clone(), t)
            })
            .collect()
    }
}

/// A lathe setup's keywords, in the order they are written (docs/cam-turning.md).
pub const LATHE_KW: [&str; 8] = [
    "bar",
    "front",
    "chuck",
    "machine",
    "at",
    "offsets",
    "tol",
    "clearance",
];

/// The lathes of the machine library (forge-machine's built-in definitions).
pub fn lathes() -> Vec<String> {
    use forge_machine::def::Kind;
    forge_machine::library::builtin()
        .into_iter()
        .filter(|m| m.kind == Kind::Lathe)
        .map(|m| m.name)
        .collect()
}

/// A call's arguments as written, split at top-level commas: (keyword, text).
fn call_args(inner: &str) -> Vec<(Option<String>, String)> {
    let mut out = vec![];
    let (mut depth, mut start, mut quote) = (0i32, 0usize, false);
    let mut push = |piece: &str| {
        let piece = piece.trim();
        if piece.is_empty() {
            return;
        }
        // `key: value` when the text before the first colon is a name
        match piece.split_once(':') {
            Some((k, v)) if k.trim().chars().all(|c| c.is_alphanumeric() || c == '_') && !k.trim().is_empty() => {
                out.push((Some(k.trim().to_string()), v.trim().to_string()))
            }
            _ => out.push((None, piece.to_string())),
        }
    };
    for (i, c) in inner.char_indices() {
        match c {
            '"' => quote = !quote,
            '(' | '[' if !quote => depth += 1,
            ')' | ']' if !quote => depth -= 1,
            ',' if depth == 0 && !quote => {
                push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    push(&inner[start..]);
    out
}

/// A setup statement's arguments as the dialog edits them.
pub fn setup_args(s: &Stmt) -> Vec<(String, String)> {
    let mut a: Vec<(String, String)> = vec![];
    for x in &s.args {
        let Some(k) = &x.key else { continue };
        if k == "stock" {
            let t = x.text.trim();
            let Some((head, inner)) = t.split_once('(') else { continue };
            let inner = inner.strip_suffix(')').unwrap_or(inner);
            let kws = call_args(inner);
            let kind = match head.trim() {
                "box" if kws.iter().any(|(k, _)| k.as_deref() == Some("min")) => "box",
                "box" => "margin",
                "cylinder" => "cylinder",
                "tube" => "tube",
                "solid" => "solid",
                _ => continue,
            };
            a.push(("stock_kind".into(), kind.into()));
            for (key, v) in kws {
                match key {
                    Some(key) => a.push((key, v)),
                    None if kind == "solid" => a.push(("file".into(), v.trim_matches('"').to_string())),
                    None => {}
                }
            }
            continue;
        }
        a.push((k.clone(), x.text.clone()));
    }
    if !a.iter().any(|x| x.0 == "stock_kind") {
        let from = a.iter().any(|x| x.0 == "from");
        a.push(("stock_kind".into(), if from { "from" } else { "margin" }.into()));
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each stock shape reads into the setup dialog and is written back as it was.
    #[test]
    fn stock_shapes_round_trip_through_the_setup_dialog() {
        for stock in [
            "box(margin: (3, 3, 2), below: 3)",
            "box(min: (-50, -50, 0), max: (50, 50, 25))",
            "cylinder(d: 60, margin: (2, 1), below: 4)",
            "cylinder(margin: (2, 1))",
            "tube(d: 60, id: 20, margin: (1, 1))",
            "solid(\"blank.fg\")",
        ] {
            let text = format!("machining\nf = part(\"p.fg\")\nop1 = setup(f, stock: {stock})\n");
            let st = doc::read(&text).unwrap();
            let s = st.iter().find(|s| s.name == "op1").unwrap();
            let d = SetupDlg {
                id: Some("op1".into()),
                name: "op1".into(),
                tab: 1,
                args: setup_args(s),
                error: None,
                lathe: false,
            };
            let kw = CamWs::setup_kw(&d);
            assert_eq!(kw.iter().find(|k| k.0 == "stock").unwrap().1, stock, "{:?}", d.args);
        }
    }

    const DOC: &str = "machining
shared tol = 0.01
f    = part(\"flange.fg\", group: \"N\")
m    = machine(\"generic-vmc3\")
t1   = tool(\"flat-10\", t: 1, stickout: 32, holder: \"bt40-er32\")
op1  = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\", tol: tol)
skim = facing(op1, tool: t1, to: 18, step: 1)
prof = contour(op1, tool: t1, side: \"outside\", from: 12, to: -0.5, step: 4)
prog = program(op1, post: \"fanuc\", number: 1001)
";

    fn ws() -> CamWs {
        let mut w = CamWs::open("/nowhere/job.fg", DOC);
        w.auto = false;
        w
    }

    #[test]
    fn a_new_op_goes_after_the_setups_last_op() {
        let mut w = ws();
        w.command(Cmd::NewOp("pocket"));
        let mut d = w.op_dlg.take().expect("dialog");
        assert_eq!(d.setup, "op1");
        assert_eq!(d.get("tool"), "t1");
        d.set("from", "12".into());
        d.set("to", "4".into());
        d.set("stepover", "2.5".into());
        w.op_ok(&d).expect("added");
        // named so it never hides the built-in
        assert!(
            w.text
                .contains("pocket2 = pocket(op1, tool: t1, from: 12, to: 4, stepover: 2.5)\nprog"),
            "{}",
            w.text
        );
        assert_eq!(w.selected.as_deref(), Some("pocket2"));
        assert!(w.stale.contains(&"pocket2".to_string()));
        // undone: the text as it was
        w.command(Cmd::Undo);
        assert_eq!(w.text, DOC);
    }

    #[test]
    fn an_op_edited_keeps_its_place_and_comments() {
        let mut w = ws();
        w.command(Cmd::Edit("prof".into()));
        let mut d = w.op_dlg.take().expect("dialog");
        assert_eq!(d.get("side"), "\"outside\"");
        d.set("leave", "0.2".into());
        d.set("step", "".into());
        w.op_ok(&d).expect("edited");
        assert!(
            w.text
                .contains("prof = contour(op1, tool: t1, side: \"outside\", from: 12, to: -0.5, leave: 0.2)"),
            "{}",
            w.text
        );
    }

    #[test]
    fn a_hole_put_in_rewrites_the_call_in_place() {
        let mut w = ws();
        w.edit(&Edit::Add {
            before: Some("prog".into()),
            text: "bore = pocket(op1, tool: t1, from: 12, to: 4)".into(),
        });
        w.command(Cmd::Edit("bore".into()));
        let mut d = w.op_dlg.take().unwrap();
        d.set("#1", "f.bore_h".into());
        d.set("from", "".into());
        d.set("to", "".into());
        w.op_ok(&d).expect("edited");
        assert!(
            w.text.contains("bore = pocket(op1, f.bore_h, tool: t1)\nprog"),
            "{}",
            w.text
        );
    }

    #[test]
    fn open_for_a_part_finds_or_makes_its_machining_document() {
        let ex = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined/flange.fg");
        let w = open_for(ex).expect("found");
        assert!(w.path.ends_with("flange_cam.fg"));
        assert!(!w.dirty());
        assert_eq!(w.setups().len(), 1);
        let ex = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/machined/nameplate.fg"
        );
        let w = open_for(ex).expect("made");
        assert!(w.path.ends_with("nameplate_cam.fg"));
        assert!(w.dirty(), "new: unsaved");
        assert!(forge_cam_lang::is_machining(&w.text));
        assert_eq!(w.stmt("f").and_then(|s| s.pos(0)), Some("\"nameplate.fg\""));
        assert!(w.part_path().is_some_and(|p| p.ends_with("nameplate.fg")));
    }

    #[test]
    fn a_lathe_setup_and_its_ops_are_written_as_turning_statements() {
        let text = "machining\nf  = part(\"shaft.fg\")\nt1 = lathe_tool(\"od-rough-cnmg\", t: 1)\n";
        let mut w = CamWs::open("/nowhere/shaft_turn.fg", text);
        w.auto = false;
        assert!(!w.turning());
        w.command(Cmd::NewLatheSetup);
        let mut d = w.setup_dlg.take().unwrap();
        assert!(d.lathe);
        d.args.push(("at".into(), "100".into()));
        w.setup_ok(&d).expect("added");
        assert!(w.turning());
        assert!(
            w.text.contains("jaws = chuck(face: -70, grip: 30, jaw: 15)\n"),
            "{}",
            w.text
        );
        assert!(
            w.text.contains(
                "op1 = lathe_setup(f, bar: 32, front: 2, chuck: jaws, machine: \"generic-lathe\", at: 100)"
            ),
            "{}",
            w.text
        );
        assert!(
            w.text
                .contains("prog1 = lathe_program(op1, post: \"fanuc-lathe\", number: 1)"),
            "{}",
            w.text
        );
        assert!(forge_turn_lang::is_turning(&w.text));
        w.command(Cmd::NewOp("turn_rough"));
        let mut o = w.op_dlg.take().unwrap();
        assert_eq!(o.get("tool"), "t1", "a lathe tool");
        o.set("leave", "0.3".into());
        o.set("step", "2".into());
        w.op_ok(&o).expect("added");
        assert!(
            w.text
                .contains("rough = turn_rough(op1, tool: t1, leave: 0.3, step: 2)\nprog1"),
            "{}",
            w.text
        );
        // the setup edited in place: the bar
        w.command(Cmd::Edit("op1".into()));
        let mut d = w.setup_dlg.take().unwrap();
        assert!(d.lathe);
        d.args.iter_mut().find(|a| a.0 == "bar").unwrap().1 = "36".into();
        w.setup_ok(&d).expect("edited");
        assert!(w.text.contains("lathe_setup(f, bar: 36, front: 2"), "{}", w.text);
        assert!(lathes().contains(&"generic-lathe".to_string()));
    }

    #[test]
    fn a_fusion_library_is_imported_into_the_project() {
        let dir = std::env::temp_dir().join(format!("fg-cam-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("shop.json"),
            r#"{"version": 36, "data": [
              {"guid": "a", "type": "flat end mill", "unit": "millimeters", "description": "6mm 3fl",
               "geometry": {"DC": 6, "LCF": 18, "OAL": 57, "SFDM": 6, "NOF": 3, "LB": 24},
               "post-process": {"number": 7}, "start-values": {}}]}"#,
        )
        .unwrap();
        let mut w = CamWs::open(dir.join("job.fg").to_str().unwrap(), DOC);
        let before = w.lib.tools.len();
        let said = w.import_fusion("shop.json").expect("imported");
        assert!(said.starts_with("1 tools imported"), "{said}");
        assert!(dir.join("cam/shop.toml").exists());
        assert_eq!(w.lib.tools.len(), before + 1);
        // and a document opened there sees it
        assert_eq!(library(&dir).tools.len(), before + 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_program_over_two_setups_generates() {
        let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/machined"));
        let text = std::fs::read_to_string(dir.join("flange_cam.fg")).unwrap();
        let mut w = CamWs::open(dir.join("flange_cam.fg").to_str().unwrap(), &text);
        w.auto = false;
        w.command(Cmd::NewSetup);
        let d = w.setup_dlg.take().unwrap();
        w.setup_ok(&d).expect("second setup");
        // prog posts both
        // a program reads its setups: refused above op2, made after moving below it
        assert!(!w.edit(&Edit::Set {
            id: "prog".into(),
            arg: "#0".into(),
            text: "[op1, op2]".into()
        }));
        w.edits(&[
            Edit::Move {
                id: "prog".into(),
                before: None,
            },
            Edit::Set {
                id: "prog".into(),
                arg: "#0".into(),
                text: "[op1, op2]".into(),
            },
        ])
        .expect("moved and set");
        assert!(w.text.contains("prog = program([op1, op2], post:"), "{}", w.text);
        let r = run::run(&w.text, &dir);
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert_eq!(
            r.programs
                .iter()
                .find(|p| p.name == "prog")
                .map(|p| p.text.is_some()),
            Some(true),
            "{:?}",
            r.programs
        );
        w.running = Some(r.text.clone());
        w.ran(r);
        let times = w.setup_times();
        assert_eq!(times.len(), 2);
        assert!(times[0].1 > 0.0);
    }

    #[test]
    fn reorder_and_suppress() {
        let mut w = ws();
        w.command(Cmd::Move("prof".into(), true));
        let names: Vec<&str> = w.ops_of("op1").iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["prof", "skim"]);
        w.command(Cmd::Move("prof".into(), false));
        assert_eq!(w.text, DOC);
        w.command(Cmd::Suppress("skim".into()));
        assert_eq!(w.status("skim"), "suppressed");
        assert!(w.text.contains("#~skim skim"));
        w.command(Cmd::Suppress("skim".into()));
        assert_eq!(w.text, DOC);
    }

    #[test]
    fn a_new_setup_comes_with_its_program() {
        let mut w = ws();
        w.command(Cmd::NewSetup);
        let d = w.setup_dlg.take().unwrap();
        assert_eq!(d.name, "op2");
        w.setup_ok(&d).expect("added");
        assert!(
            w.text.contains(
                "op2 = setup(f, machine: m, stock: box(margin: (3, 3, 2)), origin: \"top-center\")"
            ),
            "{}",
            w.text
        );
        assert!(
            w.text
                .contains("prog2 = program(op2, post: \"fanuc\", number: 1002)"),
            "{}",
            w.text
        );
    }

    #[test]
    fn a_setup_edited_round_trips() {
        let mut w = ws();
        w.command(Cmd::Edit("op1".into()));
        let mut d = w.setup_dlg.take().unwrap();
        assert!(d.args.contains(&("margin".into(), "(3, 3, 2)".into())));
        // nothing changed: the same text
        w.setup_ok(&d.clone()).expect("same");
        assert_eq!(w.text, DOC);
        d.args.retain(|a| a.0 != "margin");
        d.args.push(("margin".into(), "(5, 5, 2)".into()));
        w.setup_ok(&d).expect("edited");
        assert!(w.text.contains("stock: box(margin: (5, 5, 2))"), "{}", w.text);
    }
}

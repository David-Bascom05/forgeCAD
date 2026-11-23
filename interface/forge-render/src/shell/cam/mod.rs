//! The Manufacture workspace: a machining document (docs/cam.md) opened in the app. Its text is
//! the source of truth, as a part's is: the browser, the dialogs and the ribbon only ever make
//! edits to it (doc.rs), and what they show is forge-cam-lang's run of it (run.rs), the same
//! reply `forgecad cam --json` gives. The part it machines is open beside it, in the view.
//!
//! What another view reads of it (the toolpath and simulation views are CAM-SIM's): `focus()`,
//! the setup and op selected and a check's witness asked to be located, and `run`'s paths.

pub mod doc;
pub mod run;
pub mod schema;
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
}

/// What the ribbon asks for (an `Action` is Copy: the browser's asks carry names, `Cmd`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    NewSetup,
    NewOp(&'static str),
    Generate,
    PostProcess,
    SetupSheet,
    ToolLibrary,
    Checks,
    Undo,
    Redo,
    Save,
    ToggleText,
    AutoGenerate,
}

impl From<Ask> for Cmd {
    fn from(a: Ask) -> Cmd {
        match a {
            Ask::NewSetup => Cmd::NewSetup,
            Ask::NewOp(f) => Cmd::NewOp(f),
            Ask::Generate => Cmd::Generate,
            Ask::PostProcess => Cmd::PostProcess,
            Ask::SetupSheet => Cmd::SetupSheet,
            Ask::ToolLibrary => Cmd::ToolLibrary,
            Ask::Checks => Cmd::Checks,
            Ask::Undo => Cmd::Undo,
            Ask::Redo => Cmd::Redo,
            Ask::Save => Cmd::Save,
            Ask::ToggleText => Cmd::ToggleText,
            Ask::AutoGenerate => Cmd::AutoGenerate,
        }
    }
}

/// What the ribbon, the browser and the dialogs ask for.
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    NewSetup,
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
    Checks,
    Undo,
    Redo,
    Save,
    /// The text, edited by hand (the Text window).
    Text(String),
    ToggleText,
    AutoGenerate,
}

/// The setup dialog's state: the args as edited, by keyword.
#[derive(Clone, Debug, Default)]
pub struct SetupDlg {
    /// The statement edited; None: a new one.
    pub id: Option<String>,
    pub name: String,
    pub tab: usize,
    /// keyword → text: machine, origin, tol, clearance, at, fixture, from, stock_kind, margin,
    /// min, max
    pub args: Vec<(String, String)>,
    pub error: Option<String>,
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
}

impl CamWs {
    /// The document at `path`, its text `text`.
    pub fn open(path: &str, text: &str) -> CamWs {
        let dir = std::path::Path::new(path)
            .parent()
            .map_or_else(|| PathBuf::from("."), std::path::Path::to_path_buf);
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
        let mut ws = CamWs {
            path: path.to_string(),
            dir,
            text: text.to_string(),
            saved: text.to_string(),
            stmts: doc::read(text).unwrap_or_default(),
            run: None,
            stale: vec![],
            want_run: true,
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
                self.post_dlg = match self.post_dlg {
                    Some(_) => None,
                    None => Some(PostDlg::default()),
                }
            }
            Cmd::SetupSheet => match &self.run {
                Some(r) => {
                    let p = self.path.trim_end_matches(".fg").to_string() + "-sheet.html";
                    self.want_write.push((p, r.sheet.clone()));
                }
                None => self.said = Some("generate first".into()),
            },
            Cmd::ToolLibrary => self.tools_open = !self.tools_open,
            Cmd::Checks => self.checks_open = !self.checks_open,
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
                self.text_open = !self.text_open;
                self.text_edit = self.text.clone();
            }
            Cmd::AutoGenerate => self.auto = !self.auto,
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
                d.args = setup_args(&s);
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
        }
    }

    fn new_op_dlg(&self, func: &str, setup: &str) -> OpDlg {
        let spec = schema::spec(func);
        let base = match func {
            "facing" => "face",
            "edge_break" => "chamfer",
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
                .or_else(|| self.tools().first().map(|s| s.name.clone()));
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
                    let t = self.text.clone();
                    let r = doc::apply(
                        &t,
                        &Edit::Add {
                            before: Some(id.clone()),
                            text: Self::op_text(&OpDlg {
                                name: "cam_new_".into(),
                                ..d.clone()
                            }),
                        },
                    )
                    .and_then(|t| doc::apply(&t, &Edit::Delete { id: id.clone() }))
                    .and_then(|t| {
                        doc::apply(
                            &t,
                            &Edit::Rename {
                                from: "cam_new_".into(),
                                to: id.clone(),
                            },
                        )
                    })?;
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
        format!("{} = setup({})", d.name, parts.join(", "))
    }

    pub fn setup_ok(&mut self, d: &SetupDlg) -> Result<(), String> {
        let text = self.setup_text(d);
        match &d.id {
            None => {
                // at the end, so it reads what is above it
                self.edits(&[Edit::Add { before: None, text }])?;
                // and a program for it, so its ops are posted and certified
                let n = doc::free_name(&self.stmts, &format!("prog{}", self.setups().len()));
                let num = 1000 + self.setups().len();
                self.edits(&[Edit::Add {
                    before: None,
                    text: format!("{n} = program({}, post: \"fanuc\", number: {num})", d.name),
                }])?;
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

    /// The setup dialog's keyword arguments, in the order they are written ("" left out).
    pub fn setup_kw(d: &SetupDlg) -> Vec<(&'static str, String)> {
        let get = |k: &str| {
            d.args
                .iter()
                .find(|a| a.0 == k)
                .map(|a| a.1.trim().to_string())
                .unwrap_or_default()
        };
        let stock = match get("stock_kind").as_str() {
            "box" => format!("box(min: {}, max: {})", get("min"), get("max")),
            "from" => String::new(),
            _ => format!("box(margin: {})", get("margin")),
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

/// A setup statement's arguments as the dialog edits them.
pub fn setup_args(s: &Stmt) -> Vec<(String, String)> {
    let mut a: Vec<(String, String)> = vec![];
    for x in &s.args {
        let Some(k) = &x.key else { continue };
        if k == "stock" {
            let t = x.text.trim();
            if let Some(inner) = t.strip_prefix("box(").and_then(|r| r.strip_suffix(')')) {
                if let Some(m) = inner.trim().strip_prefix("margin:") {
                    a.push(("stock_kind".into(), "margin".into()));
                    a.push(("margin".into(), m.trim().into()));
                } else {
                    a.push(("stock_kind".into(), "box".into()));
                    // min: (..), max: (..)
                    if let (Some(i), Some(j)) = (inner.find("min:"), inner.find("max:")) {
                        let (lo, hi) = if i < j { (i, j) } else { (j, i) };
                        let first = inner[lo..hi].trim().trim_end_matches(',');
                        let second = inner[hi..].trim();
                        for part in [first, second] {
                            if let Some(v) = part.strip_prefix("min:") {
                                a.push(("min".into(), v.trim().into()));
                            } else if let Some(v) = part.strip_prefix("max:") {
                                a.push(("max".into(), v.trim().into()));
                            }
                        }
                    }
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

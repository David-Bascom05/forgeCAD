//! The Agent tab's host side: the backend made for the open document's folder and the
//! settings, its events into the transcript, what it writes taken in as one undo step a file a
//! turn, a file it makes opened. The chat is the app's: it follows the document open.

use super::Gui;
use crate::agent::claude::{self, ClaudeCode};
use crate::agent::openai::{self, OpenAi};
use crate::agent::{Backend, Doc, Out, Turns, keys, system_prompt, trash_path};
use crate::shell::agent::{AgentAct, Entry, Event, counts, prompt, undo_label};
use crate::shell::agent_settings::{AgentSettings, Kind, SettingsAct, missing};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

#[derive(Default)]
pub struct AgentHost {
    backend: Option<Box<dyn Backend>>,
    rx: Option<Receiver<Event>>,
    /// What the backend was made for: the folder, the settings.
    made_for: Option<(String, AgentSettings)>,
    /// CLI agent session, carried to a new process when only the model changed.
    session: Option<std::sync::Arc<std::sync::Mutex<Option<String>>>>,
    pub turns: Turns,
    /// The document the agent was last told is open.
    told: Option<String>,
    loaded: bool,
    keyring: Option<Receiver<Result<Option<bool>, String>>>,
    /// CLI program override, and where it runs, unless a test says otherwise.
    pub program: Option<String>,
    pub cwd: Option<PathBuf>,
    /// `agent.toml`, unless a test says otherwise.
    pub settings_file: Option<PathBuf>,
    /// The app tools' calls, to the frame's thread; the way to them, for the backends.
    pub app_rx: Option<Receiver<crate::agent::apptools::AppCall>>,
    app_tools: Option<crate::agent::apptools::AppTools>,
    /// CLI agents' MCP server: this app's socket and the `forge-app` that bridges to it.
    mcp: Option<String>,
    pub mcp_exe: Option<PathBuf>,
}

fn settings_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("forgecad").join("agent.toml"))
}

fn name_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_string(), |n| n.to_string_lossy().to_string())
}

fn dir_of(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|d| d.display().to_string())
        .unwrap_or_default()
}

impl Gui {
    /// A turn's running (not waiting to be let into a folder).
    pub fn agent_busy(&self) -> bool {
        self.shell.agent.running && self.shell.agent.trust_ask.is_none()
    }

    /// Before the frame: the settings once, the backend's events.
    pub(crate) fn take_agent(&mut self) {
        if !self.agent.loaded {
            self.agent.loaded = true;
            // an agent.toml from before settings.toml's [agent] table: moved in once (the file kept)
            if !AgentSettings::in_extra(&self.shell.settings.extra)
                && let Some(text) = self
                    .agent
                    .settings_file
                    .clone()
                    .or_else(settings_path)
                    .and_then(|p| std::fs::read_to_string(p).ok())
            {
                AgentSettings::from_toml(&text).write_extra(&mut self.shell.settings.extra);
            }
        }
        // settings.toml's [agent] table is the settings: the Settings window writes it
        let now = AgentSettings::from_extra(&self.shell.settings.extra);
        if now != self.shell.agent.settings {
            if now.base_url != self.shell.agent.settings.base_url {
                crate::shell::agent_settings::with_key(|k| k.stored = None);
            }
            self.shell.agent.settings = now;
        }
        for a in crate::shell::agent_settings::take_asked() {
            self.shell.agent.out.push(AgentAct::Settings(a));
        }
        if let Some(r) = &self.agent.keyring
            && let Ok(got) = r.try_recv()
        {
            self.agent.keyring = None;
            match got {
                Ok(s) => crate::shell::agent_settings::with_key(|k| {
                    k.stored = s;
                    k.error = None;
                }),
                Err(e) => crate::shell::agent_settings::with_key(|k| k.error = Some(e)),
            }
        }
        let a = &self.shell.agent;
        let on_agent = crate::shell::settings::SECTIONS
            .get(self.shell.prefs.tab)
            .is_some_and(|x| x.0 == "Agent");
        if self.shell.prefs.open
            && on_agent
            && a.settings.kind == Kind::OpenAi
            && crate::shell::agent_settings::with_key(|k| k.stored.is_none())
            && self.agent.keyring.is_none()
        {
            let url = a.settings.base_url.clone();
            self.agent_keyring(move || keys::get(&url).map(|k| Some(k.is_some())));
        }
        let got: Vec<Event> = self
            .agent
            .rx
            .as_ref()
            .map(|r| r.try_iter().collect())
            .unwrap_or_default();
        for e in got {
            match e {
                Event::FileChanged { path, before, text } => self.agent_file(path, before, text),
                Event::Done { .. } | Event::Problem { .. } => {
                    self.shell.agent.take(e);
                    self.agent_turn_over();
                    // what the agent left in the file is a save of its own, marked as its
                    if let Some(p) = self.fg.as_ref().and_then(|s| s.path.clone()) {
                        self.versions_saved(&p, Some("agent"));
                    }
                }
                e => self.shell.agent.take(e),
            }
        }
    }

    /// A file the agent wrote: the open document takes it as the turn's step; a file it made
    /// opens, the app switching to it.
    fn agent_file(&mut self, path: String, before: Option<String>, text: String) {
        let active = self.agent_path();
        if active.as_deref() == Some(path.as_str()) {
            let d = doc(&mut self.shell.drawing, &mut self.fg);
            // the document has it already (a file it made, opened by the watch first): still the
            // turn's change, kept for its diff and Undo
            let d = d.filter(|d| d.text() != text);
            let made = before.is_none();
            if self.agent.turns.changed(d, &path, before, text) && made {
                // opened by the watch before this came
                self.shell.agent.take(Event::Note {
                    text: format!("Opened {}, which the agent made", name_of(&path)),
                    error: false,
                });
            }
            self.fg_text();
            self.shell.status = "The agent changed the file".into();
            return;
        }
        // open in another tab: that document takes it, one undo step there
        if let Some(i) = self.agent_tab_of(&path) {
            let mut turns = std::mem::take(&mut self.agent.turns);
            let (session, drawing, _) = self.doc_parts(i);
            let d: Option<&mut dyn Doc> = match (session, drawing) {
                (_, Some(d)) => Some(d),
                (Some(s), None) => Some(s),
                _ => None,
            };
            let d = d.filter(|d| d.text() != text);
            turns.changed(d, &path, before, text);
            self.agent.turns = turns;
            return;
        }
        let made = before.is_none();
        let first = self.agent.turns.changed(None, &path, before, text);
        if made && first && path.ends_with(".fg") {
            if let Some(f) = self.agent.turns.file_mut(&path) {
                f.opened_from = active;
            }
            self.agent_open(&path);
            self.shell.agent.take(Event::Note {
                text: format!("Opened {}, which the agent made", name_of(&path)),
                error: false,
            });
        }
    }

    /// The tab a file is open in, if it's not the active one.
    fn agent_tab_of(&self, path: &str) -> Option<usize> {
        let want = std::fs::canonicalize(path).ok()?;
        let i = self.documents().iter().position(|d| {
            d.sub.is_none()
                && d.path
                    .as_deref()
                    .and_then(|p| std::fs::canonicalize(p).ok())
                    .as_ref()
                    == Some(&want)
        })?;
        (i != self.active_document()).then_some(i)
    }

    /// Opens a document in the app, in its tab, as the active one.
    fn agent_open(&mut self, path: &str) {
        self.open_document(path, true);
        if let Some(b) = &mut self.agent.backend {
            b.set_file(Path::new(path));
        }
    }

    /// The file the agent works on: the drawing open, else the part or assembly.
    pub fn agent_path(&self) -> Option<String> {
        let p = match &self.shell.drawing {
            Some(d) => d.path.clone(),
            None => self.fg.as_ref().and_then(|s| s.path.clone())?,
        };
        // absolute: the agent runs elsewhere
        Some(std::fs::canonicalize(&p).map_or(p, |a| a.display().to_string()))
    }

    fn agent_keyring(&mut self, f: impl FnOnce() -> Result<Option<bool>, String> + Send + 'static) {
        let (tx, rx) = channel();
        self.agent.keyring = Some(rx);
        let waker = self.waker.clone();
        std::thread::spawn(move || {
            let _ = tx.send(f());
            waker();
        });
    }

    fn agent_reset(&mut self) {
        self.agent.backend = None;
        self.agent.rx = None;
        self.agent.made_for = None;
        self.agent.session = None;
        self.agent.told = None;
        self.agent.turns = Turns::default();
    }

    /// The turn's changes, a diff for each file it changed or made.
    fn agent_turn_over(&mut self) {
        self.agent_turn(false);
        let Some((turn, files)) = self.agent.turns.finish() else {
            return;
        };
        for (index, lines) in files {
            let f = &self.agent.turns.done[turn].files[index];
            let (added, removed) = counts(&lines);
            self.shell.agent.entries.push(Entry::Diff {
                turn,
                index,
                created: f.before.is_none(),
                file: name_of(&f.path),
                lines,
                added,
                removed,
                undone: false,
            });
        }
    }

    /// Into settings.toml's [agent] table: the app saves it as it does every setting.
    fn agent_save_settings(&mut self) {
        let a = self.shell.agent.settings.clone();
        a.write_extra(&mut self.shell.settings.extra);
    }

    /// A file's change in a turn taken back or put back: through the document when it's open;
    /// a made file moved aside (never deleted) and its document closed, or moved back and opened.
    fn agent_flip(&mut self, turn: usize, index: usize, undo: bool) -> Result<(), String> {
        let Some(f) = self
            .agent
            .turns
            .done
            .get(turn)
            .and_then(|t| t.files.get(index))
            .cloned()
        else {
            return Err("no such change".into());
        };
        if f.undone == undo {
            return Ok(());
        }
        let active = self.agent_path();
        let set = |g: &mut Gui, trash: Option<PathBuf>| {
            let r = &mut g.agent.turns.done[turn].files[index];
            r.undone = undo;
            r.trash = trash;
        };
        if f.before.is_none() {
            if undo {
                let to = trash_path(Path::new(&f.path));
                if let Some(d) = to.parent() {
                    std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
                }
                std::fs::rename(&f.path, &to).map_err(|e| format!("{}: {e}", f.path))?;
                set(self, Some(to));
                if active.as_deref() == Some(f.path.as_str())
                    && let Some(back) = f.opened_from.clone()
                {
                    self.agent_open(&back);
                }
                self.close_file(&f.path);
                self.shell.status = format!("Moved {} aside to .forgecad-trash", name_of(&f.path));
            } else {
                let from = f.trash.clone().ok_or("the file isn't in the trash")?;
                if Path::new(&f.path).exists() {
                    return Err(format!("{} exists again: not put back", f.path));
                }
                std::fs::rename(&from, &f.path).map_err(|e| format!("{}: {e}", from.display()))?;
                set(self, None);
                self.agent_open(&f.path);
            }
            return Ok(());
        }
        // open in another tab: it comes up, and is taken back there
        let active = match self.agent_tab_of(&f.path) {
            Some(i) => {
                self.activate(i);
                self.agent_path()
            }
            None => active,
        };
        if active.as_deref() == Some(f.path.as_str())
            && let Some(d) = doc(&mut self.shell.drawing, &mut self.fg)
        {
            self.agent.turns.flip_doc(d, turn, index, undo);
            self.fg_text();
            return Ok(());
        }
        // not open: the file itself
        let text = if undo {
            f.before.clone().unwrap_or_default()
        } else {
            f.after.clone()
        };
        std::fs::write(&f.path, text).map_err(|e| format!("{}: {e}", f.path))?;
        set(self, None);
        Ok(())
    }

    /// After the frame: what the tab asked for.
    pub(crate) fn agent_acts(&mut self) {
        let acts = std::mem::take(&mut self.shell.agent.out);
        for a in acts {
            match a {
                AgentAct::Send {
                    text,
                    refs,
                    markup: true,
                } => self.markup_job = Some((text, refs)),
                AgentAct::Send { text, refs, .. } => self.agent_send(&text, &refs, None),
                AgentAct::ShowMarkup(k) => self.markup_show(k),
                AgentAct::Trust { dir, yes } => {
                    self.shell.agent.trust_ask = None;
                    let pending = self.shell.agent.pending.take();
                    if yes && !crate::shell::agent_settings::trustable(&dir) {
                        self.shell.agent.take(Event::Done {
                            error: Some(
                                "Not sent: that folder's path can't be trusted (it has control characters)"
                                    .into(),
                            ),
                            stopped: false,
                        });
                    } else if yes {
                        self.shell.agent.settings.trusted.push(dir);
                        self.agent_save_settings();
                        if let Some((text, refs, picture)) = pending {
                            self.agent_send(&text, &refs, picture);
                        }
                    } else {
                        self.shell.agent.take(Event::Done {
                            error: Some("Not sent: the folder isn't trusted".into()),
                            stopped: false,
                        });
                    }
                }
                AgentAct::Stop if self.shell.agent.trust_ask.is_some() => {
                    self.shell.agent.trust_ask = None;
                    self.shell.agent.pending = None;
                    self.shell.agent.take(Event::Done {
                        error: None,
                        stopped: true,
                    });
                }
                AgentAct::Stop => {
                    if let Some(b) = &mut self.agent.backend {
                        b.stop();
                    } else {
                        self.shell.agent.take(Event::Done {
                            error: None,
                            stopped: true,
                        });
                    }
                }
                AgentAct::NewChat => self.agent_reset(),
                AgentAct::Undo(k, i) | AgentAct::Redo(k, i) => {
                    let undo = matches!(a, AgentAct::Undo(..));
                    match self.agent_flip(k, i, undo) {
                        Ok(()) => {
                            for e in &mut self.shell.agent.entries {
                                if let Entry::Diff {
                                    turn, index, undone, ..
                                } = e
                                    && (*turn, *index) == (k, i)
                                {
                                    *undone = self.agent.turns.done[k].files[i].undone;
                                }
                            }
                            let label = &self.agent.turns.done[k].label;
                            if !self.shell.status.starts_with("Moved") {
                                self.shell.status = if undo {
                                    format!("Undid {label}")
                                } else {
                                    format!("Redid {label}")
                                };
                            }
                        }
                        Err(e) => self.shell.agent.take(Event::Note { text: e, error: true }),
                    }
                }
                AgentAct::Settings(SettingsAct::Changed) => self.agent_save_settings(),
                AgentAct::Settings(SettingsAct::SaveKey { url, key }) => {
                    let forget = key.is_empty();
                    self.agent_keyring(move || keys::set(&url, &key).map(|_| Some(!forget)));
                }
            }
        }
    }

    pub(crate) fn agent_send(&mut self, text: &str, refs: &[String], picture: Option<Vec<u8>>) {
        let fail = |g: &mut Gui, text: String, fix: Option<String>| {
            g.shell.agent.take(Event::Problem { text, fix });
        };
        let Some(path) = self.agent_path() else {
            let why = if self.fg.is_some() {
                "Save the part to a file first: the agent works on the file"
            } else {
                "Open a Forge part first"
            };
            return fail(self, why.into(), None);
        };
        // a folder outside the projects tree is trusted once, by asking
        let dir = Path::new(&path)
            .parent()
            .and_then(|d| d.canonicalize().ok())
            .map(|d| d.display().to_string())
            .unwrap_or_default();
        if !trusted(&dir, &self.shell.agent.settings.trusted) {
            self.shell.agent.trust_ask = Some(dir);
            self.shell.agent.pending = Some((text.to_string(), refs.to_vec(), picture));
            return;
        }
        // the agent reads the file: what's on screen is what it gets
        if self.shell.drawing.is_none()
            && let Some(s) = &mut self.fg
            && s.dirty()
        {
            if let Err(e) = s.save() {
                return fail(self, format!("Couldn't save the part for the agent: {e}"), None);
            }
            self.fg_text();
            self.shell.agent.take(Event::Note {
                text: format!(
                    "Saved your unsaved edits to {} first: the agent works on the file",
                    name_of(&path)
                ),
                error: false,
            });
        }
        let settings = self.shell.agent.settings.clone();
        if let Some(why) = missing(&settings) {
            self.shell.open_agent_settings();
            return fail(self, why.into(), None);
        }
        let want = (dir_of(&path), settings.clone());
        if self.agent.made_for.as_ref() != Some(&want) {
            let was = self.agent.made_for.take();
            let carry = match &was {
                Some((d, w))
                    if *d == want.0
                        && w.kind == settings.kind
                        && matches!(settings.kind, Kind::ClaudeCode | Kind::Codex) =>
                {
                    self.agent.session.as_ref().and_then(|s| s.lock().ok()?.clone())
                }
                _ => None,
            };
            if was.is_some() && carry.is_none() && !self.shell.agent.entries.is_empty() {
                let why = if was.as_ref().is_some_and(|w| w.0 != want.0) {
                    format!("Now in {}: a new conversation", want.0)
                } else {
                    format!("Now {}: a new conversation", settings.label())
                };
                self.shell.agent.take(Event::Note {
                    text: why,
                    error: false,
                });
            }
            self.agent_make(Path::new(&path), &settings, carry);
            self.agent.made_for = Some(want);
            self.agent.told = Some(path.clone());
        }
        self.agent.turns.start(undo_label(text));
        self.apps.view_set = false;
        let mut msg = prompt(text, refs);
        if self.agent.told.as_deref() != Some(path.as_str()) {
            msg = format!("(The open file is now {path}.)\n\n{msg}");
            self.agent.told = Some(path.clone());
            if let Some(b) = &mut self.agent.backend {
                b.set_file(Path::new(&path));
            }
        }
        if let (Some(t), Some(d)) = (&self.agent.app_tools, Path::new(&path).parent()) {
            t.set_dir(d);
        }
        // the turn's on: a .fg it makes beside an open document opens in a tab
        self.agent_turn(true);
        if let Some(b) = &mut self.agent.backend {
            b.send(msg, picture);
        }
    }

    fn agent_make(&mut self, file: &Path, s: &AgentSettings, session: Option<String>) {
        let (tx, rx) = channel();
        self.agent.rx = Some(rx);
        let out = Out {
            tx,
            wake: self.waker.clone(),
        };
        let system = system_prompt(file);
        if self.agent.app_tools.is_none() {
            let (tools, rx) = crate::agent::apptools::AppTools::new(self.waker.clone());
            self.agent.app_rx = Some(rx);
            // Claude Code reaches them through `forge-app --mcp-bridge`, so only from the app itself
            let exe = self.agent.mcp_exe.clone().or_else(|| {
                std::env::current_exe()
                    .ok()
                    .filter(|e| e.file_name().is_some_and(|n| n == "forge-app"))
            });
            if let Some(exe) = exe {
                let sock = crate::agent::mcp::socket_path();
                match crate::agent::mcp::serve(&sock, tools.clone()) {
                    Ok(()) => self.agent.mcp = Some(crate::agent::mcp::config(&exe, &sock)),
                    Err(e) => self.shell.agent.take(Event::Note {
                        text: format!(
                            "The app's own tools aren't there for the agent: {}: {e}",
                            sock.display()
                        ),
                        error: true,
                    }),
                }
            }
            self.agent.app_tools = Some(tools);
        }
        self.agent.backend = Some(match s.kind {
            Kind::ClaudeCode => {
                let c = ClaudeCode::new(
                    claude::Config {
                        program: self.agent.program.clone().unwrap_or_else(|| "claude".into()),
                        model: s.claude_model.clone(),
                        file: file.to_path_buf(),
                        system,
                        cwd: self.agent.cwd.clone().unwrap_or_else(claude::clean_cwd),
                        mcp: self.agent.mcp.clone(),
                    },
                    out,
                    session,
                );
                self.agent.session = Some(c.session.clone());
                Box::new(c)
            }
            Kind::Codex => {
                let c = crate::agent::codex::Codex::new(
                    claude::Config {
                        program: self.agent.program.clone().unwrap_or_else(|| "codex".into()),
                        model: s.codex_model.clone(),
                        file: file.to_path_buf(),
                        system,
                        cwd: self.agent.cwd.clone().unwrap_or_else(claude::clean_cwd),
                        mcp: self.agent.mcp.clone(),
                    },
                    out,
                    session,
                );
                self.agent.session = Some(c.session.clone());
                Box::new(c)
            }
            Kind::OpenAi => Box::new(OpenAi::new(
                openai::Config {
                    base_url: s.base_url.clone(),
                    model: s.model.clone(),
                    file: file.to_path_buf(),
                    system,
                    key: None,
                    apps: self.agent.app_tools.clone(),
                },
                out,
            )),
        });
    }
}

/// The document open: the drawing, else the part or assembly.
fn doc<'a>(
    drawing: &'a mut Option<crate::shell::drawing::DrawingEnv>,
    fg: &'a mut Option<crate::fg::Session>,
) -> Option<&'a mut dyn Doc> {
    match drawing {
        Some(d) => Some(d),
        None => fg.as_mut().map(|s| s as &mut dyn Doc),
    }
}

/// A folder the agent may work in without asking: under ~/projects, or trusted once.
pub fn trusted(dir: &str, trusted: &[String]) -> bool {
    if !crate::shell::agent_settings::trustable(dir) {
        return false;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let projects = format!("{home}/projects/");
    (!home.is_empty() && format!("{dir}/").starts_with(&projects)) || trusted.iter().any(|t| t == dir)
}

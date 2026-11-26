//! The Agent tab's host side: the backend made for the open document's folder and the
//! settings, its events into the transcript, what it writes taken in as one undo step a file a
//! turn, a file it makes opened. The chat is the app's: it follows the document open.

use super::Gui;
use crate::agent::claude;
#[cfg(not(target_arch = "wasm32"))]
use crate::agent::claude::ClaudeCode;
use crate::agent::openai::{self, OpenAi};
use crate::agent::{Backend, Doc, Out, Turns, keys, system_prompt, trash_path};
use crate::shell::agent::{AgentAct, Entry, Event, counts, prompt, undo_label};
use crate::shell::agent_settings::{AgentSettings, Kind, SettingsAct, missing};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

struct Followup {
    document: Option<String>,
    text: String,
    refs: Vec<String>,
    pictures: Vec<crate::agent::Picture>,
    /// Its message in the chat (sent, shown, waiting): taken out if it's dropped.
    entry: Option<usize>,
    /// Its card, not in the chat while it waits: added when it goes.
    card: Option<Entry>,
    project: Option<PathBuf>,
    model: String,
}

#[derive(Default)]
pub struct AgentHost {
    pub bridge: Option<forge_agent::chat_bridge::Bridge>,
    bridge_active: Option<(String, usize)>,
    backend: Option<Box<dyn Backend>>,
    turn_active: bool,
    followups: std::collections::VecDeque<Followup>,
    /// The card of the message being sent, when the chat hasn't it yet: added when it goes.
    card: Option<Entry>,
    rx: Option<Receiver<Event>>,
    /// What the backend was made for: the folder, the settings.
    made_for: Option<(String, AgentSettings)>,
    /// CLI agent session, carried to a new process when only the model changed.
    session: Option<std::sync::Arc<std::sync::Mutex<Option<String>>>>,
    pub turns: Turns,
    /// The document the agent was last told is open.
    told: Option<String>,
    loaded: bool,
    /// The project whose last conversation was looked for, when its Agent tab first showed.
    reopened_for: Option<PathBuf>,
    keyring: Option<Receiver<Result<Option<bool>, String>>>,
    /// CLI program override, and where it runs, unless a test says otherwise.
    pub program: Option<String>,
    pub cwd: Option<PathBuf>,
    /// Captured launch folder, used for project conversations before any document is open.
    pub project_dir: Option<PathBuf>,
    /// Local conversation storage override (tests and portable installations).
    pub conversations_dir: Option<PathBuf>,
    chat: Option<crate::agent::conversations::Conversation>,
    endpoint_history: Option<std::sync::Arc<std::sync::Mutex<Vec<forge_agent::json::Json>>>>,
    last_saved: Option<crate::plat::Instant>,
    save_error: Option<String>,
    resumed: bool,
    /// `agent.toml`, unless a test says otherwise.
    pub settings_file: Option<PathBuf>,
    /// The app tools' calls, to the frame's thread; the way to them, for the backends.
    pub app_rx: Option<Receiver<crate::agent::apptools::AppCall>>,
    app_tools: Option<crate::agent::apptools::AppTools>,
    /// CLI agents' MCP server: this app's socket and the `forge-app` that bridges to it.
    mcp: Option<String>,
    pub mcp_exe: Option<PathBuf>,
}

impl AgentHost {
    fn conversation_dir(&self) -> Option<PathBuf> {
        self.conversations_dir
            .clone()
            .or_else(|| {
                self.settings_file
                    .as_ref()
                    .and_then(|p| p.parent())
                    .map(|p| p.join(".conversations"))
            })
            .or_else(crate::agent::conversations::default_dir)
    }
    fn flush_conversation(&mut self) -> Result<(), String> {
        if self.chat.is_none() {
            return Ok(());
        }
        let base = self
            .conversation_dir()
            .ok_or("No local conversation storage directory is available")?;
        let chat = self.chat.as_mut().expect("chat");
        chat.session = self.session.as_ref().and_then(|s| s.lock().ok()?.clone());
        if let Some(history) = &self.endpoint_history {
            chat.history = history.lock().map_err(|e| e.to_string())?.clone();
        }
        chat.save(&base)?;
        self.last_saved = Some(crate::plat::Instant::now());
        Ok(())
    }
}
impl Drop for AgentHost {
    fn drop(&mut self) {
        let _ = self.flush_conversation();
    }
}

fn settings_path() -> Option<PathBuf> {
    let base = forge_dirs::config()?;
    Some(base.join("forgecad").join("agent.toml"))
}

fn name_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_string(), |n| n.to_string_lossy().to_string())
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
            if self.agent.project_dir.is_none() {
                self.agent.project_dir = std::env::current_dir().ok();
            }
            // an agent.toml from before settings.toml's [agent] table: moved in once (the file kept)
            if !AgentSettings::in_extra(&self.shell.settings.extra)
                && let Some(text) = self
                    .agent
                    .settings_file
                    .clone()
                    .or_else(settings_path)
                    .and_then(|p| forge_dirs::fs::read_to_string(p).ok())
            {
                AgentSettings::from_toml(&text).write_extra(&mut self.shell.settings.extra);
            }
        }
        self.agent_reopen_last();
        let project = self.agent_root().map(|p| p.display().to_string());
        if project != self.shell.agent.project {
            self.shell.agent.resume_open = false;
            self.shell.agent.saved_chats.clear();
        }
        self.shell.agent.project = project;
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
        let changed = !got.is_empty();
        let terminal = got.iter().any(crate::agent::conversations::terminal);
        for e in got {
            match e {
                Event::FileChanged { path, before, text } => self.agent_file(path, before, text),
                Event::Permission {
                    id,
                    tool,
                    tool_use,
                    summary,
                    input,
                    always,
                    raw,
                } => {
                    let input = self.agent_edit_preview(&tool, &raw).unwrap_or(input);
                    self.shell.agent.take(Event::Permission {
                        id,
                        tool,
                        tool_use,
                        summary,
                        input,
                        always,
                        raw,
                    })
                }
                Event::Done { .. } | Event::Problem { .. } => {
                    let error = match &e {
                        Event::Done { error, stopped } => error
                            .clone()
                            .or_else(|| stopped.then(|| "Stopped in the UI".into())),
                        Event::Problem { text, .. } => Some(text.clone()),
                        _ => None,
                    };
                    self.agent.turn_active = false;
                    self.shell.agent.take(e);
                    self.finish_bridge_request(error);
                    self.agent_turn_over();
                    self.versions_changed();
                }
                e => self.shell.agent.take(e),
            }
        }
        if changed {
            self.agent_capture_conversation(terminal);
        }
        self.take_followups();
        self.take_bridge_requests();
    }

    /// The tab's view of the queue: how many, and each one's words.
    fn sync_queue(&mut self) {
        self.shell.agent.queued = self.agent.followups.len();
        self.shell.agent.queue = self
            .agent
            .followups
            .iter()
            .map(|f| {
                match f
                    .card
                    .as_ref()
                    .or(f.entry.and_then(|k| self.shell.agent.entries.get(k)))
                {
                    Some(Entry::User { text, .. }) => text.clone(),
                    _ => f.text.clone(),
                }
            })
            .collect();
    }

    /// A queued message dropped, or taken back into the box to change and send again.
    fn unqueue(&mut self, index: usize, edit: bool) {
        let Some(f) = self.agent.followups.remove(index) else {
            return;
        };
        let mut text = f.text.clone();
        let mut refs = f.refs.clone();
        if let Some(Entry::User { text: t, refs: r, .. }) = &f.card {
            (text, refs) = (t.clone(), r.clone());
        } else if let Some(k) = f.entry
            && let Some(Entry::User { text: t, refs: r, .. }) = self.shell.agent.entries.get(k)
        {
            (text, refs) = (t.clone(), r.clone());
            self.shell.agent.entries.remove(k);
            for g in &mut self.agent.followups {
                if let Some(j) = &mut g.entry
                    && *j > k
                {
                    *j -= 1;
                }
            }
        }
        if edit {
            let a = &mut self.shell.agent;
            a.input = if a.input.trim().is_empty() {
                text
            } else {
                format!("{text}\n{}", a.input)
            };
            for r in refs {
                if !a.refs.contains(&r) {
                    a.refs.push(r);
                }
            }
            a.pictures.extend(f.pictures);
            a.focus_input = true;
        }
        self.sync_queue();
    }

    fn take_followups(&mut self) {
        self.sync_queue();
        if self.agent.turn_active
            || self.shell.agent.running
            || self.shell.agent.voice.busy()
            || self.markup_job.is_some()
            || self.markup_wait.is_some()
        {
            return;
        }
        let Some(next) = self.agent.followups.pop_front() else {
            return;
        };
        self.sync_queue();
        if next.project != self.agent_root()
            || next.model != self.shell.agent.settings.label()
            || (next.document.is_some() && next.document != self.agent_path())
        {
            self.shell.agent.take(Event::Note {
                text: "Queued message cancelled because the document, project or agent changed.".into(),
                error: true,
            });
            return;
        }
        self.shell.agent.running = true;
        self.agent.card = next.card;
        self.agent_send(&next.text, &next.refs, next.pictures);
    }

    fn cancel_followups(&mut self) {
        self.markup_job = None;
        self.markup_wait = None;
        self.shell.agent.markup.sending = false;
        let count = self.agent.followups.len();
        self.agent.followups.clear();
        self.sync_queue();
        if count > 0 {
            self.shell.agent.take(Event::Note {
                text: format!("Cancelled {count} queued message(s)."),
                error: false,
            });
        }
    }

    /// Publish a terminal connection for this window without starting a model session.
    pub fn start_agent_bridge(&mut self) {
        // a page has no terminal to connect (the agent runs in the desktop app): it failed there,
        // and "Agent connection: operation not supported on this platform" stood in the status bar
        if cfg!(target_arch = "wasm32") {
            return;
        }
        if self.agent.bridge.is_none() {
            match forge_agent::chat_bridge::Bridge::start(self.waker.clone()) {
                Ok(bridge) => self.agent.bridge = Some(bridge),
                Err(e) => self.shell.status = format!("Agent connection: {e}"),
            }
        }
    }

    fn finish_bridge_request(&mut self, error: Option<String>) {
        let Some((id, start)) = self.agent.bridge_active.take() else {
            return;
        };
        let mut reply = self
            .shell
            .agent
            .entries
            .get(start..)
            .unwrap_or_default()
            .iter()
            .filter_map(|e| match e {
                Entry::Agent { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        if let Some(e) = &error {
            if !reply.is_empty() {
                reply.push('\n');
            }
            reply.push_str(e);
        }
        if let Some(bridge) = &self.agent.bridge {
            bridge.finish(&id, reply, error.is_some());
        }
    }

    fn take_bridge_requests(&mut self) {
        let Some(bridge) = &self.agent.bridge else { return };
        let project = self
            .agent_root()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let model = self.shell.agent.settings.label();
        bridge.update(project.clone(), model.clone(), self.shell.agent.running);
        if self.shell.agent.running
            || self.shell.agent.voice.busy()
            || self.shell.agent.trust_ask.is_some()
            || self.agent.bridge_active.is_some()
        {
            return;
        }
        let Some(request) = bridge.next() else { return };
        if request.project != project || request.model != model {
            bridge.finish(&request.id, "The window's project or selected agent changed while this request was queued; send it again.".into(), true);
            return;
        }
        let text = format!("[Terminal agent request {}]\n{}", request.id, request.message);
        self.shell.agent.entries.push(Entry::User {
            text: text.clone(),
            refs: vec![],
            markup: None,
            pictures: 0,
        });
        self.shell.agent.problem = None;
        self.shell.agent.running = true;
        self.agent_send(&text, &[], vec![]);
        self.agent.bridge_active = Some((request.id, self.shell.agent.entries.len()));
        if !self.shell.agent.running {
            self.finish_bridge_request(Some(
                "The UI agent could not start; see the window for details.".into(),
            ));
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
        let want = forge_dirs::fs::canonicalize(path).ok()?;
        let i = self.documents().iter().position(|d| {
            d.sub.is_none()
                && d.path
                    .as_deref()
                    .and_then(|p| forge_dirs::fs::canonicalize(p).ok())
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

    /// A picture pasted or dropped on the window: the Agent tab's, for its next message, when the
    /// tab is the one showing. False when it isn't (or it's not a picture the models read).
    pub fn agent_picture(&mut self, bytes: Vec<u8>) -> bool {
        self.shell.agent.tab == crate::shell::agent::SideTab::Agent && self.shell.agent.add_picture(bytes)
    }

    /// An edit the agent asks to make, as its card shows it: the file as it is with the change
    /// made, -/+ lines with a line around them, so the card says what changes where (a param's
    /// name, not just "= 12" to "= 14"). None when it isn't an edit, or its file can't be read.
    fn agent_edit_preview(&self, tool: &str, raw: &str) -> Option<String> {
        use forge_agent::json::Json;
        let input = Json::parse(raw).ok()?;
        let path = input.get("file_path").and_then(Json::str)?;
        let file = self.agent_edit_file(path);
        let now = file
            .as_ref()
            .and_then(|f| forge_dirs::fs::read_to_string(f).ok())
            .unwrap_or_default();
        if file.is_none() && tool != "Write" {
            return None;
        }
        let new = crate::shell::agent::edited(tool, &input, &now)?;
        Some(crate::shell::agent::diff_text(&crate::shell::agent::diff(
            &now, &new, 1,
        )))
    }

    /// The file an edit names, here: as given if it's here, else from where the agent runs (its
    /// paths are relative to that), else (the page, whose project is mounted elsewhere than it
    /// is on the machine) the longest tail of it found under the project.
    fn agent_edit_file(&self, path: &str) -> Option<PathBuf> {
        let p = Path::new(path);
        // has_root, not is_absolute: in a page (wasm32-unknown) no path is absolute
        if p.has_root() && forge_dirs::fs::is_file(p) {
            return Some(p.to_path_buf());
        }
        let bases = [self.agent.cwd.clone(), self.agent_root()];
        let parts: Vec<_> = p.components().collect();
        let tails = if !p.has_root() { 0..1 } else { 1..parts.len() };
        bases.iter().flatten().find_map(|base| {
            tails
                .clone()
                .map(|i| base.join(parts[i..].iter().collect::<PathBuf>()))
                .find(|f| forge_dirs::fs::is_file(f))
        })
    }

    fn agent_root(&self) -> Option<PathBuf> {
        if let Some(file) = self.agent_path() {
            let file = Path::new(&file);
            if forge_history::History::open(file).is_none()
                && let Some((root, _)) = &self.agent.made_for
                && file.starts_with(root)
            {
                return Some(PathBuf::from(root));
            }
            return Some(crate::agent::project::root(file));
        }
        self.agent
            .made_for
            .as_ref()
            .map(|(p, _)| PathBuf::from(p))
            .or_else(|| {
                self.agent
                    .project_dir
                    .as_ref()
                    .map(|p| crate::agent::project::root(p))
            })
    }

    /// The file the agent works on: the drawing open, else the part or assembly.
    pub fn agent_path(&self) -> Option<String> {
        let p = match &self.shell.drawing {
            Some(d) => d.path.clone(),
            None => self.fg.as_ref().and_then(|s| s.path.clone())?,
        };
        // absolute: the agent runs elsewhere
        Some(forge_dirs::fs::canonicalize(&p).map_or(p, |a| a.display().to_string()))
    }

    fn agent_keyring(&mut self, f: impl FnOnce() -> Result<Option<bool>, String> + Send + 'static) {
        let (tx, rx) = channel();
        self.agent.keyring = Some(rx);
        let waker = self.waker.clone();
        crate::plat::spawn("forge-job", move || {
            let _ = tx.send(f());
            waker();
        });
    }

    fn agent_capture_conversation(&mut self, force: bool) {
        let Some(chat) = self.agent.chat.as_mut() else {
            return;
        };
        chat.entries = self.shell.agent.entries.clone();
        chat.turns = self.agent.turns.done.clone();
        if let Some((_, settings)) = &self.agent.made_for {
            chat.settings = settings.clone();
        }
        chat.settings.trusted.clear();
        chat.updated = forge_history::now() as u64;
        chat.interrupted = self.shell.agent.running;
        if force || self.agent.last_saved.is_none_or(|t| t.elapsed().as_secs() >= 1) {
            if let Err(error) = self.agent.flush_conversation() {
                if self.agent.save_error.as_ref() != Some(&error) {
                    self.shell.agent.take(Event::Note {
                        text: format!("Couldn't save this conversation: {error}"),
                        error: true,
                    });
                    self.agent.save_error = Some(error);
                }
            } else {
                self.agent.save_error = None;
            }
        }
    }

    /// The first time the Agent tab shows for a project, its last conversation comes back (the
    /// tab's + starts a new one), as `claude --continue` would.
    fn agent_reopen_last(&mut self) {
        if self.shell.agent.tab != crate::shell::agent::SideTab::Agent
            || !self.shell.agent.entries.is_empty()
            || self.agent.chat.is_some()
            || self.agent_busy()
        {
            return;
        }
        let Some(root) = self.agent_root() else { return };
        if self.agent.reopened_for.as_ref() == Some(&root) {
            return;
        }
        self.agent.reopened_for = Some(root.clone());
        let Some(base) = self.agent.conversation_dir() else {
            return;
        };
        let Ok(list) = crate::agent::conversations::list(&base, &root) else {
            return;
        };
        // the last saved; within one second (its stamp's grain), the last made (its id starts
        // with its time, in nanoseconds)
        let Some(last) = list.iter().max_by_key(|c| (c.updated, c.id.clone())) else {
            return;
        };
        let id = last.id.clone();
        self.agent_resume_conversation(&id);
        self.shell.agent.resume_open = false;
    }

    fn agent_list_conversations(&mut self) {
        if self.agent_busy() {
            return;
        }
        self.agent_capture_conversation(true);
        self.shell.agent.resume_open = true;
        self.shell.agent.resume_error = None;
        let got = (|| {
            let root = self.agent_root().ok_or("No project folder")?;
            let base = self
                .agent
                .conversation_dir()
                .ok_or("No conversation storage directory")?;
            crate::agent::conversations::import_codex(&base, &root)?;
            crate::agent::conversations::list(&base, &root)
        })();
        match got {
            Ok(list) => self.shell.agent.saved_chats = list,
            Err(e) => {
                self.shell.agent.saved_chats.clear();
                self.shell.agent.resume_error = Some(e);
            }
        }
    }

    fn agent_resume_conversation(&mut self, id: &str) {
        if self.agent_busy() {
            return;
        }
        self.shell.agent.resume_open = true;
        let got = (|| {
            let root = self.agent_root().ok_or("No project folder")?;
            let base = self
                .agent
                .conversation_dir()
                .ok_or("No conversation storage directory")?;
            crate::agent::conversations::load(&base, &root, id)
        })();
        let mut chat = match got {
            Ok(c) => c,
            Err(e) => {
                self.shell.agent.resume_error = Some(e);
                return;
            }
        };
        let same = self.agent.chat.as_ref().is_some_and(|c| c.id == chat.id);
        if same
            && self
                .agent
                .chat
                .as_ref()
                .is_some_and(|c| c.generation == chat.generation)
        {
            self.shell.agent.resume_open = false;
            return;
        }
        if !same && let Err(e) = self.agent.flush_conversation() {
            self.shell.agent.resume_error = Some(e);
            return;
        }
        let current = self.shell.agent.settings.clone();
        let interrupted = chat.interrupted;
        self.agent_reset();
        self.agent.turns.restore(chat.turns.clone());
        self.shell.agent.restore_entries(chat.entries.clone());
        chat.settings.trusted = current.trusted;
        chat.settings.speech_key = current.speech_key;
        chat.settings.speech_mode = current.speech_mode;
        chat.settings.speech_send = current.speech_send;
        self.shell.agent.settings = chat.settings.clone();
        self.agent_save_settings();
        self.agent.made_for = Some((chat.project.clone(), chat.settings.clone()));
        self.agent.project_dir = Some(PathBuf::from(&chat.project));
        self.agent.session = Some(std::sync::Arc::new(std::sync::Mutex::new(chat.session.clone())));
        self.agent.endpoint_history = Some(std::sync::Arc::new(std::sync::Mutex::new(chat.history.clone())));
        self.agent.resumed = true;
        // a divider, not a note: said once, wherever it was resumed, and not saved with the chat
        let when = crate::shell::agent::ago(chat.updated);
        self.shell.agent.resumed = Some((
            self.shell.agent.entries.len(),
            if interrupted {
                format!("Resumed · {when} · it stopped mid-turn")
            } else {
                format!("Resumed · {when}")
            },
        ));
        self.agent.chat = Some(chat);
        self.agent_capture_conversation(true);
    }

    fn agent_reset(&mut self) {
        self.cancel_followups();
        self.agent.turn_active = false;
        self.finish_bridge_request(Some("Conversation reset in the UI".into()));
        self.agent.backend = None;
        self.agent.rx = None;
        self.agent.made_for = None;
        self.agent.session = None;
        self.agent.told = None;
        self.agent.turns = Turns::default();
        self.agent.chat = None;
        self.agent.endpoint_history = None;
        self.agent.last_saved = None;
        self.agent.save_error = None;
        self.agent.resumed = false;
    }

    /// The turn's changes, a diff for each file it changed or made.
    fn agent_turn_over(&mut self) {
        self.agent_turn(false);
        let Some((turn, files)) = self.agent.turns.finish() else {
            return;
        };
        let record = &self.agent.turns.done[turn];
        let paths: Vec<PathBuf> = record.files.iter().map(|f| PathBuf::from(&f.path)).collect();
        if let Some(first) = paths.first()
            && let Some(history) = forge_history::History::open(first)
        {
            match history.save_group(&paths, &record.label, Some("agent")) {
                Ok(forge_history::Saved::Paused(why)) => self.shell.agent.take(Event::Note {
                    text: format!("Changes are on disk; history is paused: {why:?}"),
                    error: true,
                }),
                Err(e) => self.shell.agent.take(Event::Note {
                    text: format!("Changes are on disk; couldn't save history: {e}"),
                    error: true,
                }),
                _ => {}
            }
            forge_lang::pins::forget();
        }
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
    pub fn agent_flip(&mut self, turn: usize, index: usize, undo: bool) -> Result<(), String> {
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
        // a turn from a saved conversation: only over the file as it would leave it, never over
        // what's been done since
        if f.restored {
            let want = if undo { Some(&f.after) } else { f.before.as_ref() };
            // the file as the person has it: its document when it's open (in this tab or
            // another, which comes up), else what's on disk
            if self.agent_path().as_deref() != Some(f.path.as_str())
                && let Some(i) = self.agent_tab_of(&f.path)
            {
                self.activate(i);
            }
            let open = (self.agent_path().as_deref() == Some(f.path.as_str()))
                .then(|| doc(&mut self.shell.drawing, &mut self.fg).map(|d| d.text().to_string()))
                .flatten();
            let now = open.or_else(|| forge_dirs::fs::read_to_string(&f.path).ok());
            let fits = match (want, &now) {
                (Some(w), Some(n)) => w == n,
                // a file the turn made, undone: it's gone, as Redo wants
                (None, None) => true,
                _ => false,
            };
            if !fits {
                return Err(format!(
                    "{} has changed since this turn: not {}. Versions can take it back to any saved state.",
                    name_of(&f.path),
                    if undo { "undone" } else { "redone" }
                ));
            }
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
                    forge_dirs::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
                }
                forge_dirs::fs::rename(&f.path, &to).map_err(|e| format!("{}: {e}", f.path))?;
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
                if forge_dirs::fs::exists(Path::new(&f.path)) {
                    return Err(format!("{} exists again: not put back", f.path));
                }
                forge_dirs::fs::rename(&from, &f.path).map_err(|e| format!("{}: {e}", from.display()))?;
                set(self, None);
                if f.path.ends_with(".fg") {
                    self.agent_open(&f.path);
                }
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
        forge_dirs::fs::write(&f.path, text).map_err(|e| format!("{}: {e}", f.path))?;
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
                    pictures,
                    ..
                } => self.markup_job = Some((text, refs, pictures)),
                AgentAct::Send {
                    text,
                    refs,
                    pictures,
                    shown,
                    ..
                } => {
                    self.agent.card = (!shown).then(|| Entry::User {
                        text: text.clone(),
                        refs: refs.clone(),
                        markup: None,
                        pictures: pictures.len(),
                    });
                    self.agent_send(&text, &refs, pictures)
                }
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
                        if let Some((text, refs, pictures)) = pending {
                            self.agent_send(&text, &refs, pictures);
                        }
                    } else {
                        self.shell.agent.take(Event::Done {
                            error: Some("Not sent: the folder isn't trusted".into()),
                            stopped: false,
                        });
                    }
                }
                AgentAct::Stop if self.shell.agent.trust_ask.is_some() => {
                    self.cancel_followups();
                    self.shell.agent.trust_ask = None;
                    self.shell.agent.pending = None;
                    self.shell.agent.take(Event::Done {
                        error: None,
                        stopped: true,
                    });
                }
                AgentAct::Stop => {
                    self.cancel_followups();
                    if let Some(b) = &mut self.agent.backend {
                        b.stop();
                    } else {
                        self.agent.turn_active = false;
                        self.shell.agent.take(Event::Done {
                            error: None,
                            stopped: true,
                        });
                    }
                }
                AgentAct::Answer(id, answer) => {
                    if let Some(b) = &mut self.agent.backend {
                        b.answer(&id, answer);
                    }
                }
                AgentAct::Mode(m) => {
                    if let Some(b) = &mut self.agent.backend {
                        b.set_mode(m);
                    }
                }
                AgentAct::OwnMode(m) => {
                    if let Some(b) = &mut self.agent.backend {
                        b.set_own_mode(&m);
                    }
                }
                AgentAct::Unqueue { index, edit } => self.unqueue(index, edit),
                AgentAct::ListConversations => self.agent_list_conversations(),
                AgentAct::ResumeConversation(id) => self.agent_resume_conversation(&id),
                AgentAct::NewChat => {
                    if let Err(error) = self.agent.flush_conversation() {
                        self.shell.agent.take(Event::Note {
                            text: format!("Couldn't save the previous conversation: {error}"),
                            error: true,
                        });
                    }
                    self.agent_reset();
                }
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
                    self.agent_capture_conversation(true);
                }
                AgentAct::Settings(SettingsAct::Changed) => self.agent_save_settings(),
                AgentAct::Settings(SettingsAct::SaveKey { url, key }) => {
                    let forget = key.is_empty();
                    self.agent_keyring(move || keys::set(&url, &key).map(|_| Some(!forget)));
                }
            }
        }
    }

    pub(crate) fn agent_send(&mut self, text: &str, refs: &[String], pictures: Vec<crate::agent::Picture>) {
        if self.agent.turn_active {
            self.agent.followups.push_back(Followup {
                document: self.agent_path(),
                text: text.into(),
                refs: refs.to_vec(),
                pictures,
                // its card in the chat already (markup's), else kept to add when it goes
                entry: self
                    .agent
                    .card
                    .is_none()
                    .then(|| {
                        self.shell
                            .agent
                            .entries
                            .iter()
                            .rposition(|e| matches!(e, Entry::User { .. }))
                    })
                    .flatten(),
                card: self.agent.card.take(),
                project: self.agent_root(),
                model: self.shell.agent.settings.label(),
            });
            self.sync_queue();
            return;
        }
        // it goes now: its card, if the chat hasn't it yet
        if let Some(c) = self.agent.card.take() {
            self.shell.agent.entries.push(c);
        }
        let fail = |g: &mut Gui, text: String, fix: Option<String>| {
            g.shell.agent.take(Event::Problem { text, fix });
        };
        let document = self.agent_path();
        let Some(root) = self.agent_root() else {
            return fail(self, "Couldn't determine the project folder".into(), None);
        };
        let dir = root.display().to_string();
        // A directory is an explicit project-only context, never a fabricated part file.
        let path = document.clone().unwrap_or_else(|| dir.clone());
        if self.shell.agent.settings.kind == Kind::OpenAi
            && !trusted(&dir, &self.shell.agent.settings.trusted)
        {
            self.shell.agent.trust_ask = Some(dir);
            self.shell.agent.pending = Some((text.to_string(), refs.to_vec(), pictures));
            return;
        }
        // the agent reads the file: what's on screen is what it gets
        if document.is_some()
            && self.shell.drawing.is_none()
            && let Some(s) = &mut self.fg
            && s.dirty()
        {
            if let Err(e) = s.save() {
                return fail(self, format!("Couldn't save the part for the agent: {e}"), None);
            }
            self.fg_text();
            self.versions_saved(&path, None);
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
        let new_context = self.agent.chat.as_ref().is_some_and(|chat| {
            chat.project != dir
                || chat.settings.kind != settings.kind
                || (settings.kind == Kind::OpenAi && chat.settings.base_url != settings.base_url)
        });
        if new_context {
            if let Err(e) = self.agent.flush_conversation() {
                return fail(
                    self,
                    format!("Couldn't save the previous conversation: {e}"),
                    None,
                );
            }
            let user = self
                .shell
                .agent
                .entries
                .last()
                .filter(|e| matches!(e, Entry::User { .. }))
                .cloned();
            self.agent_reset();
            self.shell.agent.restore_entries(user.into_iter().collect());
            self.shell.agent.running = true;
        }
        if self.agent.chat.is_none() {
            match crate::agent::conversations::Conversation::new(&root, &settings, text) {
                Ok(chat) => self.agent.chat = Some(chat),
                Err(e) => return fail(self, e, None),
            }
        }
        let want = (dir.clone(), settings.clone());
        if self.agent.backend.is_none() || self.agent.made_for.as_ref() != Some(&want) {
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
        // a backend's slash command (/compact, a skill) goes as typed, nothing added: Claude Code
        // takes a message as a command only when it's the whole of it. What it'd be told waits for
        // the next message.
        let command = text.starts_with('/');
        let mut msg = if command {
            text.to_string()
        } else {
            prompt(text, refs)
        };
        if !command
            && document.is_some()
            && self.shell.drawing.is_none()
            && let Some(note) = self.shell.fg.as_ref().and_then(build_note)
        {
            msg.push_str(&format!("\n\n({note})"));
        }
        if !command && self.agent.told.as_deref() != Some(path.as_str()) {
            msg = if document.is_some() {
                format!("(The open file is now {path}.)\n\n{msg}")
            } else {
                format!("(No document is open. The project folder is {dir}.)\n\n{msg}")
            };
            self.agent.told = Some(path.clone());
            if let Some(b) = &mut self.agent.backend {
                b.set_file(Path::new(&path));
            }
        }
        if let Some(t) = &self.agent.app_tools {
            t.set_dir(Path::new(&dir));
        }
        if !command && self.agent.resumed {
            msg = format!(
                "(This conversation was resumed. Inspect the current project before editing; files may have changed. Current context: {path}.)\n\n{msg}"
            );
            self.agent.resumed = false;
        }
        // the turn's on: a .fg it makes beside an open document opens in a tab
        self.agent_turn(true);
        self.agent.turn_active = true;
        self.shell.agent.running = true;
        if let Some(b) = &mut self.agent.backend {
            b.send(msg, pictures);
        }
        self.agent_capture_conversation(true);
    }

    fn agent_make(&mut self, file: &Path, s: &AgentSettings, session: Option<String>) {
        let (tx, rx) = channel();
        self.agent.rx = Some(rx);
        let out = Out {
            tx,
            wake: self.waker.clone(),
        };
        let mut system = system_prompt(file);
        if let Some(bridge) = &self.agent.bridge {
            system.push_str(&format!(
                "\nThis UI window's terminal connection ID is {}. Terminal agents can use forgecad agent list/send/status to send requests and retrieve your replies. Reply normally to terminal requests. You may contact other windows with these commands, but never send a request to your own window and wait for it during your active turn: it would be queued behind you.", bridge.id
            ));
        }
        if self.agent.resumed
            && session.is_none()
            && s.kind != Kind::OpenAi
            && let Some(chat) = &self.agent.chat
        {
            let context: String = chat.context().chars().take(64000).collect();
            system.push_str(&format!(
                "\nSaved conversation context (the original CLI session id is unavailable):\n{context}"
            ));
        }
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
        let mut backend: Box<dyn crate::agent::Backend> = match s.kind {
            // in the page: the person's own Claude Code, through `forgecad web --agent`
            #[cfg(target_arch = "wasm32")]
            Kind::ClaudeCode if crate::agent::bridge::ready() => {
                Box::new(crate::agent::bridge::Bridge::new(out, file))
            }
            #[cfg(target_arch = "wasm32")]
            Kind::ClaudeCode => Box::new(crate::agent::bridge::Unbridged { out }),
            #[cfg(not(target_arch = "wasm32"))]
            Kind::ClaudeCode => {
                let c = ClaudeCode::new(
                    claude::Config {
                        program: self
                            .agent
                            .program
                            .clone()
                            .unwrap_or_else(|| crate::agent::program("claude")),
                        model: s.claude_model.clone(),
                        file: file.to_path_buf(),
                        system,
                        cwd: self
                            .agent
                            .cwd
                            .clone()
                            .unwrap_or_else(|| crate::agent::project::root(file)),
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
                        program: self
                            .agent
                            .program
                            .clone()
                            .unwrap_or_else(|| crate::agent::program("codex")),
                        model: s.codex_model.clone(),
                        file: file.to_path_buf(),
                        system,
                        cwd: self
                            .agent
                            .cwd
                            .clone()
                            .unwrap_or_else(|| crate::agent::project::root(file)),
                        mcp: self.agent.mcp.clone(),
                    },
                    out,
                    session,
                );
                self.agent.session = Some(c.session.clone());
                Box::new(c)
            }
            Kind::OpenAi => {
                let mut backend = OpenAi::new(
                    openai::Config {
                        base_url: s.base_url.clone(),
                        model: s.model.clone(),
                        file: file.to_path_buf(),
                        system,
                        key: None,
                        apps: self.agent.app_tools.clone(),
                    },
                    out,
                );
                if let Some(history) = &self.agent.endpoint_history {
                    backend.restore_history(history.lock().expect("history").clone());
                }
                self.agent.endpoint_history = Some(backend.history_handle());
                Box::new(backend)
            }
        };
        if self.shell.agent.mode_chosen {
            backend.set_mode(self.shell.agent.mode);
        }
        self.agent.backend = Some(backend);
    }
}

/// Issues told with a message, at most; the rest are counted.
const NOTE_ISSUES: usize = 3;

/// The open file as it builds now, for the agent with each message: its errors and warnings, the
/// first few by line. None while it hasn't built.
pub fn build_note(fg: &crate::shell::fg::FgPanel) -> Option<String> {
    if fg.file.is_empty() {
        return None;
    }
    let (errors, warnings) = crate::shell::fg::issue_counts(&fg.issues);
    let name = name_of(&fg.file);
    if errors + warnings == 0 {
        return Some(format!("{name} builds now with no errors or warnings"));
    }
    let shown: Vec<String> = fg
        .issues
        .iter()
        .filter(|i| !crate::shell::fg::is_note(i))
        .take(NOTE_ISSUES)
        .map(|i| format!("line {}: {}", i.line, i.text.lines().next().unwrap_or("")))
        .collect();
    let more = (errors + warnings).saturating_sub(shown.len());
    let plural = |n: usize, w: &str| format!("{n} {w}{}", if n == 1 { "" } else { "s" });
    Some(format!(
        "{name} now has {}, {}: {}{}",
        plural(errors, "error"),
        plural(warnings, "warning"),
        shown.join("; "),
        if more > 0 {
            format!("; and {more} more")
        } else {
            String::new()
        }
    ))
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

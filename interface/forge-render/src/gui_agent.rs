//! The Agent tab's host side: the backend made for the open file and the settings, its events
//! into the transcript, what it writes to the file taken in as one undo step a turn.

use super::Gui;
use crate::agent::claude::{self, ClaudeCode};
use crate::agent::openai::{self, OpenAi};
use crate::agent::{Backend, Out, Turns, fg_cli, keys, system_prompt};
use crate::shell::agent::{AgentAct, Entry, Event, counts, prompt, undo_label};
use crate::shell::agent_settings::{AgentSettings, Kind, SettingsAct, missing};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

#[derive(Default)]
pub struct AgentHost {
    backend: Option<Box<dyn Backend>>,
    rx: Option<Receiver<Event>>,
    /// What the backend was made for: the file, the settings.
    made_for: Option<(String, AgentSettings)>,
    /// Claude Code's session, carried to a new process when only the model changed.
    session: Option<std::sync::Arc<std::sync::Mutex<Option<String>>>>,
    pub turns: Turns,
    /// The file the chat's about.
    file: Option<String>,
    loaded: bool,
    keyring: Option<Receiver<Result<Option<bool>, String>>>,
    /// `claude`, unless a test says otherwise.
    pub program: Option<String>,
}

fn settings_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("forgecad").join("agent.toml"))
}

impl Gui {
    /// A turn's running.
    pub fn agent_busy(&self) -> bool {
        self.shell.agent.running
    }

    /// Before the frame: the settings once, a new file's fresh chat, the backend's events.
    pub(crate) fn take_agent(&mut self) {
        if !self.agent.loaded {
            self.agent.loaded = true;
            if let Some(text) = settings_path().and_then(|p| std::fs::read_to_string(p).ok()) {
                self.shell.agent.settings = AgentSettings::from_toml(&text);
            }
        }
        let file = self.fg.as_ref().and_then(|s| s.path.clone());
        if file != self.agent.file {
            if self.agent.file.is_some() && !self.shell.agent.entries.is_empty() {
                self.shell.agent.new_chat();
                self.shell.agent.out.clear();
            }
            self.agent_reset();
            self.agent.file = file;
        }
        if let Some(r) = &self.agent.keyring
            && let Ok(got) = r.try_recv()
        {
            self.agent.keyring = None;
            match got {
                Ok(s) => {
                    self.shell.agent.key.stored = s;
                    self.shell.agent.key.error = None;
                }
                Err(e) => self.shell.agent.key.error = Some(e),
            }
        }
        let a = &self.shell.agent;
        if a.settings_open
            && a.settings.kind == Kind::OpenAi
            && a.key.stored.is_none()
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
                Event::FileChanged(text) => {
                    if let Some(s) = &mut self.fg
                        && self.agent.turns.changed(s, text)
                    {
                        self.fg_text();
                        self.shell.status = "The agent changed the file".into();
                    }
                }
                Event::Done { .. } | Event::Problem { .. } => {
                    self.shell.agent.take(e);
                    self.agent_turn_over();
                }
                e => self.shell.agent.take(e),
            }
        }
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
        self.agent.turns = Turns::default();
    }

    /// The turn's change, if it made one, as its diff in the chat.
    fn agent_turn_over(&mut self) {
        let Some(s) = &self.fg else {
            self.agent.turns.open = None;
            return;
        };
        if let Some((turn, lines)) = self.agent.turns.finish(s) {
            let (added, removed) = counts(&lines);
            let file = s
                .path
                .as_deref()
                .and_then(|p| Path::new(p).file_name())
                .map_or_else(|| "file".into(), |n| n.to_string_lossy().to_string());
            self.shell.agent.entries.push(Entry::Diff {
                turn,
                file,
                lines,
                added,
                removed,
                undone: false,
            });
        }
    }

    /// After the frame: what the tab asked for.
    pub(crate) fn agent_acts(&mut self) {
        let acts = std::mem::take(&mut self.shell.agent.out);
        for a in acts {
            match a {
                AgentAct::Send { text, refs } => self.agent_send(&text, &refs),
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
                AgentAct::Undo(k) | AgentAct::Redo(k) => {
                    let undo = matches!(a, AgentAct::Undo(_));
                    if let Some(s) = &mut self.fg
                        && self.agent.turns.flip(s, k, undo)
                    {
                        self.fg_text();
                        for e in &mut self.shell.agent.entries {
                            if let Entry::Diff { turn, undone, .. } = e
                                && *turn == k
                            {
                                *undone = undo;
                            }
                        }
                        let label = &self.agent.turns.done[k].label;
                        self.shell.status = if undo {
                            format!("Undid {label}")
                        } else {
                            format!("Redid {label}")
                        };
                    }
                }
                AgentAct::Settings(SettingsAct::Changed) => {
                    if let Some(p) = settings_path() {
                        if let Some(d) = p.parent() {
                            let _ = std::fs::create_dir_all(d);
                        }
                        let _ = std::fs::write(p, self.shell.agent.settings.to_toml());
                    }
                }
                AgentAct::Settings(SettingsAct::SaveKey { url, key }) => {
                    let forget = key.is_empty();
                    self.agent_keyring(move || keys::set(&url, &key).map(|_| Some(!forget)));
                }
            }
        }
    }

    fn agent_send(&mut self, text: &str, refs: &[String]) {
        let fail = |g: &mut Gui, text: String, fix: Option<String>| {
            g.shell.agent.take(Event::Problem { text, fix });
        };
        let Some(s) = &mut self.fg else {
            return fail(self, "Open a Forge part first".into(), None);
        };
        let Some(path) = s.path.clone() else {
            return fail(
                self,
                "Save the part to a file first: the agent works on the file".into(),
                None,
            );
        };
        // the agent reads the file: what's on screen is what it gets
        if s.dirty() {
            if let Err(e) = s.save() {
                return fail(self, format!("Couldn't save the part for the agent: {e}"), None);
            }
            self.fg_text();
        }
        let settings = self.shell.agent.settings.clone();
        if let Some(why) = missing(&settings) {
            self.shell.agent.settings_open = true;
            return fail(self, why.into(), None);
        }
        let want = (path.clone(), settings.clone());
        if self.agent.made_for.as_ref() != Some(&want) {
            let was = self.agent.made_for.take();
            let carry = match &was {
                Some((f, w))
                    if *f == path && w.kind == Kind::ClaudeCode && settings.kind == Kind::ClaudeCode =>
                {
                    self.agent.session.as_ref().and_then(|s| s.lock().ok()?.clone())
                }
                _ => None,
            };
            if was.is_some() && carry.is_none() && !self.shell.agent.entries.is_empty() {
                self.shell.agent.take(Event::Note {
                    text: format!("Now {}: a new conversation", settings.label()),
                    error: false,
                });
            }
            self.agent_make(Path::new(&path), &settings, carry);
            self.agent.made_for = Some(want);
        }
        let src = self.fg.as_ref().map(|s| s.src.clone()).unwrap_or_default();
        self.agent.turns.start(undo_label(text), src);
        if let Some(b) = &mut self.agent.backend {
            b.send(prompt(text, refs));
        }
    }

    fn agent_make(&mut self, file: &Path, s: &AgentSettings, session: Option<String>) {
        let (tx, rx) = channel();
        self.agent.rx = Some(rx);
        let out = Out {
            tx,
            wake: self.waker.clone(),
        };
        let system = system_prompt(file, fg_cli().as_deref());
        self.agent.backend = Some(match s.kind {
            Kind::ClaudeCode => {
                let c = ClaudeCode::new(
                    claude::Config {
                        program: self.agent.program.clone().unwrap_or_else(|| "claude".into()),
                        model: s.claude_model.clone(),
                        file: file.to_path_buf(),
                        system,
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
                },
                out,
            )),
        });
    }
}

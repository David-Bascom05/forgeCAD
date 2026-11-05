//! The Agent tab's backends, each on its own threads, saying what happens as `Event`s: Claude
//! Code run headless (`claude.rs`), or any OpenAI-compatible endpoint driven by ForgeCAD's own
//! loop and tools (`openai.rs`, `tools.rs`). `Turns` makes what an agent writes to the open file
//! one undo step a turn.

pub mod claude;
pub mod keys;
pub mod openai;
pub mod tools;

pub use crate::shell::agent::Event;
use crate::shell::agent::{Line, diff};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;

pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// Where a backend's events go, and the frame they wake.
#[derive(Clone)]
pub struct Out {
    pub tx: Sender<Event>,
    pub wake: Waker,
}

impl Out {
    pub fn send(&self, e: Event) {
        let _ = self.tx.send(e);
        (self.wake)();
    }
}

pub trait Backend: Send {
    /// A message from the person: a new turn.
    fn send(&mut self, prompt: String);
    /// Stop the turn that's running; it ends with `Done { stopped: true }`.
    fn stop(&mut self);
}

/// The open file, read again after each tool: its text when it's changed since last seen.
pub struct Watch {
    pub path: PathBuf,
    last: Option<String>,
}

impl Watch {
    pub fn new(path: &Path) -> Watch {
        Watch {
            path: path.to_path_buf(),
            last: std::fs::read_to_string(path).ok(),
        }
    }

    pub fn check(&mut self) -> Option<String> {
        let now = std::fs::read_to_string(&self.path).ok()?;
        if self.last.as_deref() == Some(now.as_str()) {
            return None;
        }
        self.last = Some(now.clone());
        Some(now)
    }
}

/// The `fg` CLI the agent checks its work with: `$FORGECAD_FG`, beside this app, or on PATH.
pub fn fg_cli() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FORGECAD_FG")
        .map(PathBuf::from)
        .filter(|p| p.is_file())
    {
        return Some(p);
    }
    let beside = std::env::current_exe().ok()?.parent()?.join("fg");
    if beside.is_file() {
        return Some(beside);
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join("fg"))
        .find(|p| p.is_file())
}

/// What the agent is told it is, appended to its own system prompt.
pub fn system_prompt(file: &Path, fg: Option<&Path>) -> String {
    let f = file.display();
    let dir = file
        .parent()
        .map_or_else(|| ".".into(), |d| d.display().to_string());
    let check = match fg {
        Some(fg) => {
            let fg = fg.display();
            format!(
                "Check your work with the fg CLI: `{fg} run {f} --json` builds the part and lists its errors and warnings; \
                 `{fg} edit {f} 'OP'` applies a typed edit op (the errors' fixes are given as ops) and checks; \
                 `{fg} handles {f}` lists every parameter; `{fg} reference` is the Forge language reference."
            )
        }
        None => "The fg CLI isn't installed here: read the file closely and keep to its own forms.".into(),
    };
    format!(
        "You are ForgeCAD's in-app agent, in the Agent tab beside the part's browser. \
         The open file is {f}, a Forge (.fg) part or assembly. \
         Make the change asked by editing that file in place; the app reloads it as you write and shows your change as one undo step. \
         {check} \
         Ids like `cover.body.end` are Forge names in the open file: a feature, a face, an occurrence. \
         Never touch other projects or files outside {dir}, and never delete files. \
         Keep replies short: say what you changed and what the check said."
    )
}

/// A turn's change to the open file, kept to undo it and to show it.
#[derive(Clone, Debug, PartialEq)]
pub struct TurnRec {
    pub label: String,
    pub key: u64,
    pub before: String,
    pub after: String,
    pub undone: bool,
}

/// The agent's writes to the open file, each turn's as one undo step.
#[derive(Default)]
pub struct Turns {
    pub done: Vec<TurnRec>,
    /// The turn running: its label, key, and the text before it.
    pub open: Option<(String, u64, String)>,
    next: u64,
}

impl Turns {
    pub fn start(&mut self, label: String, before: String) {
        self.next += 1;
        self.open = Some((label, self.next, before));
    }

    /// The file says this now: the session takes it, joining the turn's step.
    pub fn changed(&mut self, s: &mut crate::fg::Session, text: String) -> bool {
        if text == s.src {
            s.external(text, "", 0, true);
            return false;
        }
        let (label, key) = match &self.open {
            Some((l, k, _)) => (l.clone(), *k),
            None => {
                // written after the turn: a step of its own
                self.next += 1;
                ("Agent".to_string(), self.next)
            }
        };
        s.external(text, &label, key, true);
        true
    }

    /// The turn's over: its change, if it made one, as `done[index]` and its diff.
    pub fn finish(&mut self, s: &crate::fg::Session) -> Option<(usize, Vec<Line>)> {
        let (label, key, before) = self.open.take()?;
        if before == s.src {
            return None;
        }
        let lines = diff(&before, &s.src, 2);
        self.done.push(TurnRec {
            label,
            key,
            before,
            after: s.src.clone(),
            undone: false,
        });
        Some((self.done.len() - 1, lines))
    }

    /// Takes a turn's change back (`undo`) or puts it back: the session's own undo when it's
    /// the last step, else a step of its own.
    pub fn flip(&mut self, s: &mut crate::fg::Session, turn: usize, undo: bool) -> bool {
        self.next += 1;
        let next = self.next;
        let Some(t) = self.done.get_mut(turn) else {
            return false;
        };
        if t.undone == undo {
            return false;
        }
        let top = s.undo_tag().map(|x| x.1);
        if undo && top == Some(t.key) && s.src == t.after {
            s.undo();
        } else {
            let (text, label) = if undo {
                (t.before.clone(), format!("Undo {}", t.label))
            } else {
                (t.after.clone(), t.label.clone())
            };
            s.external(text, &label, next, false);
        }
        t.undone = undo;
        true
    }
}

/// The environment a child `claude` starts with: this process's, less what marks a session
/// inside another Claude Code (it would run unsaved, as a child).
pub fn clean_env(c: &mut std::process::Command) {
    for (k, _) in std::env::vars_os() {
        let k = k.to_string_lossy().to_string();
        if (k.starts_with("CLAUDE") && k != "CLAUDE_CONFIG_DIR") || k == "AI_AGENT" {
            c.env_remove(k);
        }
    }
}
